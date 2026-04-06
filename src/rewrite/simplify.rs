use crate::error::SubtypeError;
use crate::located::{JsonF, LocatedValue, Provenance};
use crate::rewrite::RewriteRule;

fn synthetic(prov: &Provenance, node: JsonF<LocatedValue>) -> LocatedValue {
    LocatedValue::new(prov.clone(), node)
}

fn make_string(prov: &Provenance, s: &str) -> LocatedValue {
    synthetic(prov, JsonF::String(s.to_string()))
}

fn make_array(prov: &Provenance, items: Vec<LocatedValue>) -> LocatedValue {
    synthetic(prov, JsonF::Array(items))
}

fn make_object(prov: &Provenance, pairs: Vec<(&str, LocatedValue)>) -> LocatedValue {
    let obj_pairs = pairs
        .into_iter()
        .map(|(k, v)| (make_string(prov, k), v))
        .collect();
    synthetic(prov, JsonF::Object(obj_pairs))
}

fn get_entry<'a>(pairs: &'a [(LocatedValue, LocatedValue)], key: &str) -> Option<&'a LocatedValue> {
    pairs.iter().find_map(|(k, v)| {
        if k.as_str() == Some(key) {
            Some(v)
        } else {
            None
        }
    })
}

fn without_key(
    pairs: &[(LocatedValue, LocatedValue)],
    key: &str,
) -> Vec<(LocatedValue, LocatedValue)> {
    pairs
        .iter()
        .filter(|(k, _)| k.as_str() != Some(key))
        .cloned()
        .collect()
}

/// The bottom schema: {not: {}} — accepts nothing.
fn bottom(prov: &Provenance) -> JsonF<LocatedValue> {
    JsonF::Object(vec![(
        make_string(prov, "not"),
        make_object(prov, vec![]),
    )])
}

// ============================================================
// Enum elimination (Figure 7)
// ============================================================

// --- null enum → type: null ---

pub struct NullEnum;

impl RewriteRule for NullEnum {
    fn rewrite(
        &self,
        _prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(type_val) = get_entry(pairs, "type") else {
            return Ok(None);
        };
        if type_val.as_str() != Some("null") {
            return Ok(None);
        }
        let Some(enum_val) = get_entry(pairs, "enum") else {
            return Ok(None);
        };
        let Some(values) = enum_val.as_array() else {
            return Ok(None);
        };

        // Check all values are null.
        if values.iter().all(|v| v.is_null()) {
            Ok(Some(JsonF::Object(without_key(pairs, "enum"))))
        } else {
            Ok(None)
        }
    }
}

// --- number enum singleton → min/max range ---

pub struct NumberEnum;

impl RewriteRule for NumberEnum {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(type_val) = get_entry(pairs, "type") else {
            return Ok(None);
        };
        if type_val.as_str() != Some("number") {
            return Ok(None);
        }
        let Some(enum_val) = get_entry(pairs, "enum") else {
            return Ok(None);
        };
        let Some(values) = enum_val.as_array() else {
            return Ok(None);
        };
        if values.len() != 1 {
            return Ok(None);
        }
        let Some(n) = values[0].as_number() else {
            return Ok(None);
        };

        let mut new_pairs = without_key(pairs, "enum");
        new_pairs.push((
            make_string(prov, "minimum"),
            synthetic(prov, JsonF::Number(n)),
        ));
        new_pairs.push((
            make_string(prov, "maximum"),
            synthetic(prov, JsonF::Number(n)),
        ));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// --- multi-valued enum → anyOf of singletons ---

pub struct MultiValuedEnum;

impl RewriteRule for MultiValuedEnum {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(enum_val) = get_entry(pairs, "enum") else {
            return Ok(None);
        };
        let Some(values) = enum_val.as_array() else {
            return Ok(None);
        };
        // Boolean enums are handled separately by BoolSet.
        // Only split non-boolean multi-valued enums.
        if values.len() <= 1 {
            return Ok(None);
        }
        // Check this has a type (so we know it's been through canonicalization).
        let Some(type_val) = get_entry(pairs, "type") else {
            return Ok(None);
        };
        if type_val.as_str() == Some("boolean") {
            return Ok(None);
        }

        let other_pairs = without_key(pairs, "enum");
        let branches: Vec<LocatedValue> = values
            .iter()
            .map(|v| {
                let mut branch = other_pairs.clone();
                branch.push((
                    make_string(prov, "enum"),
                    make_array(prov, vec![v.clone()]),
                ));
                synthetic(prov, JsonF::Object(branch))
            })
            .collect();

        Ok(Some(JsonF::Object(vec![(
            make_string(prov, "anyOf"),
            make_array(prov, branches),
        )])))
    }
}

// ============================================================
// Negation elimination (Figure 8)
// ============================================================

// --- not(not(s)) → s ---

pub struct NotNot;

impl RewriteRule for NotNot {
    fn rewrite(
        &self,
        _prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(inner) = get_entry(pairs, "not") else {
            return Ok(None);
        };
        let Some(inner_pairs) = inner.as_object() else {
            return Ok(None);
        };
        if inner_pairs.len() != 1 {
            return Ok(None);
        }
        let Some(inner_inner) = get_entry(inner_pairs, "not") else {
            return Ok(None);
        };
        Ok(Some(inner_inner.node.clone()))
    }
}

// --- not(anyOf: [s1,...]) → allOf: [not(s1),...] (De Morgan) ---

pub struct NotAnyOf;

impl RewriteRule for NotAnyOf {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(inner) = get_entry(pairs, "not") else {
            return Ok(None);
        };
        let Some(inner_pairs) = inner.as_object() else {
            return Ok(None);
        };
        let Some(anyof) = get_entry(inner_pairs, "anyOf") else {
            return Ok(None);
        };
        let Some(schemas) = anyof.as_array() else {
            return Ok(None);
        };

        let nots: Vec<LocatedValue> = schemas
            .iter()
            .map(|s| make_object(prov, vec![("not", s.clone())]))
            .collect();

        Ok(Some(JsonF::Object(vec![(
            make_string(prov, "allOf"),
            make_array(prov, nots),
        )])))
    }
}

// --- not(allOf: [s1,...]) → anyOf: [not(s1),...] (De Morgan) ---

pub struct NotAllOf;

impl RewriteRule for NotAllOf {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(inner) = get_entry(pairs, "not") else {
            return Ok(None);
        };
        let Some(inner_pairs) = inner.as_object() else {
            return Ok(None);
        };
        let Some(allof) = get_entry(inner_pairs, "allOf") else {
            return Ok(None);
        };
        let Some(schemas) = allof.as_array() else {
            return Ok(None);
        };

        let nots: Vec<LocatedValue> = schemas
            .iter()
            .map(|s| make_object(prov, vec![("not", s.clone())]))
            .collect();

        Ok(Some(JsonF::Object(vec![(
            make_string(prov, "anyOf"),
            make_array(prov, nots),
        )])))
    }
}

// ============================================================
// AllOf elimination (Figure 9)
// ============================================================

// --- singleton allOf → unwrap ---

pub struct SingletonAllOf;

impl RewriteRule for SingletonAllOf {
    fn rewrite(
        &self,
        _prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(allof) = get_entry(pairs, "allOf") else {
            return Ok(None);
        };
        let Some(schemas) = allof.as_array() else {
            return Ok(None);
        };
        if schemas.len() == 1 {
            Ok(Some(schemas[0].node.clone()))
        } else {
            Ok(None)
        }
    }
}

// --- empty allOf → top (accepts everything) ---

pub struct EmptyAllOf;

impl RewriteRule for EmptyAllOf {
    fn rewrite(
        &self,
        _prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(allof) = get_entry(pairs, "allOf") else {
            return Ok(None);
        };
        let Some(schemas) = allof.as_array() else {
            return Ok(None);
        };
        if schemas.is_empty() {
            // Top = {} (empty object)
            Ok(Some(JsonF::Object(vec![])))
        } else {
            Ok(None)
        }
    }
}

// --- allOf with heterogeneous types → bottom ---

pub struct IntersectHeterogeneousTypes;

impl RewriteRule for IntersectHeterogeneousTypes {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(allof) = get_entry(pairs, "allOf") else {
            return Ok(None);
        };
        let Some(schemas) = allof.as_array() else {
            return Ok(None);
        };
        if schemas.len() < 2 {
            return Ok(None);
        }

        // Collect types from each schema in the allOf.
        let types: Vec<Option<&str>> = schemas
            .iter()
            .filter_map(|s| {
                s.as_object().map(|pairs| {
                    get_entry(pairs, "type").and_then(|t| t.as_str())
                })
            })
            .collect();

        // If we found types for at least 2 schemas and they disagree → bottom.
        let typed: Vec<&str> = types.iter().filter_map(|t| *t).collect();
        if typed.len() >= 2 && !typed.windows(2).all(|w| w[0] == w[1]) {
            return Ok(Some(bottom(prov)));
        }

        Ok(None)
    }
}

// --- allOf containing bottom → bottom ---

pub struct AllOfWithBottom;

impl RewriteRule for AllOfWithBottom {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(allof) = get_entry(pairs, "allOf") else {
            return Ok(None);
        };
        let Some(schemas) = allof.as_array() else {
            return Ok(None);
        };

        // Check if any element is bottom: {not: {}}
        let has_bottom = schemas.iter().any(|s| {
            if let Some(inner_pairs) = s.as_object() {
                if inner_pairs.len() == 1 {
                    if let Some(not_val) = get_entry(inner_pairs, "not") {
                        if let Some(not_pairs) = not_val.as_object() {
                            return not_pairs.is_empty();
                        }
                    }
                }
            }
            false
        });

        if has_bottom {
            Ok(Some(bottom(prov)))
        } else {
            Ok(None)
        }
    }
}

// --- flatten nested allOf ---

pub struct FlattenAllOf;

impl RewriteRule for FlattenAllOf {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(allof) = get_entry(pairs, "allOf") else {
            return Ok(None);
        };
        let Some(schemas) = allof.as_array() else {
            return Ok(None);
        };

        // Check if any element is itself an allOf.
        let has_nested = schemas.iter().any(|s| {
            s.as_object()
                .is_some_and(|p| p.len() == 1 && get_entry(p, "allOf").is_some())
        });

        if !has_nested {
            return Ok(None);
        }

        let mut flattened = Vec::new();
        for s in schemas {
            if let Some(inner_pairs) = s.as_object() {
                if inner_pairs.len() == 1 {
                    if let Some(inner_allof) = get_entry(inner_pairs, "allOf") {
                        if let Some(inner_schemas) = inner_allof.as_array() {
                            flattened.extend(inner_schemas.iter().cloned());
                            continue;
                        }
                    }
                }
            }
            flattened.push(s.clone());
        }

        Ok(Some(JsonF::Object(vec![(
            make_string(prov, "allOf"),
            make_array(prov, flattened),
        )])))
    }
}

// ============================================================
// AnyOf simplification (Figure 10)
// ============================================================

// --- singleton anyOf → unwrap ---

pub struct SingletonAnyOf;

impl RewriteRule for SingletonAnyOf {
    fn rewrite(
        &self,
        _prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(anyof) = get_entry(pairs, "anyOf") else {
            return Ok(None);
        };
        let Some(schemas) = anyof.as_array() else {
            return Ok(None);
        };
        if schemas.len() == 1 {
            Ok(Some(schemas[0].node.clone()))
        } else {
            Ok(None)
        }
    }
}

// --- empty anyOf → bottom ---

pub struct EmptyAnyOf;

impl RewriteRule for EmptyAnyOf {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(anyof) = get_entry(pairs, "anyOf") else {
            return Ok(None);
        };
        let Some(schemas) = anyof.as_array() else {
            return Ok(None);
        };
        if schemas.is_empty() {
            Ok(Some(bottom(prov)))
        } else {
            Ok(None)
        }
    }
}

// --- anyOf containing top → top ---

pub struct AnyOfWithTop;

impl RewriteRule for AnyOfWithTop {
    fn rewrite(
        &self,
        _prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(anyof) = get_entry(pairs, "anyOf") else {
            return Ok(None);
        };
        let Some(schemas) = anyof.as_array() else {
            return Ok(None);
        };

        // Top = {} (empty object)
        let has_top = schemas.iter().any(|s| {
            s.as_object().is_some_and(|p| p.is_empty())
        });

        if has_top {
            Ok(Some(JsonF::Object(vec![])))
        } else {
            Ok(None)
        }
    }
}

// --- flatten nested anyOf ---

pub struct FlattenAnyOf;

impl RewriteRule for FlattenAnyOf {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(anyof) = get_entry(pairs, "anyOf") else {
            return Ok(None);
        };
        let Some(schemas) = anyof.as_array() else {
            return Ok(None);
        };

        let has_nested = schemas.iter().any(|s| {
            s.as_object()
                .is_some_and(|p| p.len() == 1 && get_entry(p, "anyOf").is_some())
        });

        if !has_nested {
            return Ok(None);
        }

        let mut flattened = Vec::new();
        for s in schemas {
            if let Some(inner_pairs) = s.as_object() {
                if inner_pairs.len() == 1 {
                    if let Some(inner_anyof) = get_entry(inner_pairs, "anyOf") {
                        if let Some(inner_schemas) = inner_anyof.as_array() {
                            flattened.extend(inner_schemas.iter().cloned());
                            continue;
                        }
                    }
                }
            }
            flattened.push(s.clone());
        }

        Ok(Some(JsonF::Object(vec![(
            make_string(prov, "anyOf"),
            make_array(prov, flattened),
        )])))
    }
}

// --- anyOf: remove bottom branches ---

pub struct AnyOfRemoveBottom;

impl RewriteRule for AnyOfRemoveBottom {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(anyof) = get_entry(pairs, "anyOf") else {
            return Ok(None);
        };
        let Some(schemas) = anyof.as_array() else {
            return Ok(None);
        };

        let is_bottom = |s: &LocatedValue| -> bool {
            if let Some(inner_pairs) = s.as_object() {
                if inner_pairs.len() == 1 {
                    if let Some(not_val) = get_entry(inner_pairs, "not") {
                        if let Some(not_pairs) = not_val.as_object() {
                            return not_pairs.is_empty();
                        }
                    }
                }
            }
            false
        };

        let filtered: Vec<_> = schemas
            .iter()
            .filter(|s| !is_bottom(s))
            .cloned()
            .collect();

        if filtered.len() == schemas.len() {
            Ok(None)
        } else {
            Ok(Some(JsonF::Object(vec![(
                make_string(prov, "anyOf"),
                make_array(prov, filtered),
            )])))
        }
    }
}

// --- allOf: remove top branches ---

pub struct AllOfRemoveTop;

impl RewriteRule for AllOfRemoveTop {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if pairs.len() != 1 {
            return Ok(None);
        }
        let Some(allof) = get_entry(pairs, "allOf") else {
            return Ok(None);
        };
        let Some(schemas) = allof.as_array() else {
            return Ok(None);
        };

        let filtered: Vec<_> = schemas
            .iter()
            .filter(|s| !s.as_object().is_some_and(|p| p.is_empty()))
            .cloned()
            .collect();

        if filtered.len() == schemas.len() {
            Ok(None)
        } else {
            Ok(Some(JsonF::Object(vec![(
                make_string(prov, "allOf"),
                make_array(prov, filtered),
            )])))
        }
    }
}

/// Returns all simplification rules in application order.
pub fn simplification_rules() -> Vec<Box<dyn RewriteRule>> {
    vec![
        // Negation elimination
        Box::new(NotNot),
        Box::new(NotAnyOf),
        Box::new(NotAllOf),
        // AllOf simplification
        Box::new(EmptyAllOf),
        Box::new(AllOfWithBottom),
        Box::new(AllOfRemoveTop),
        Box::new(FlattenAllOf),
        Box::new(SingletonAllOf),
        Box::new(IntersectHeterogeneousTypes),
        // AnyOf simplification
        Box::new(EmptyAnyOf),
        Box::new(AnyOfWithTop),
        Box::new(AnyOfRemoveBottom),
        Box::new(FlattenAnyOf),
        Box::new(SingletonAnyOf),
        // Enum elimination
        Box::new(NullEnum),
        Box::new(NumberEnum),
        Box::new(MultiValuedEnum),
        // String/array/object enum elimination deferred to after regex algebra.
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::rewrite::rewrite_phase;

    #[test]
    fn null_enum_simplified() {
        let input = r#"{"type": "null", "enum": [null]}"#;
        let expected = r#"{"type": "null"}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(NullEnum)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let expected_tree = parse(expected).unwrap();
        assert!(result.structural_eq(&expected_tree));
    }

    #[test]
    fn number_enum_to_range() {
        let input = r#"{"type": "number", "enum": [42]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(NumberEnum)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert_eq!(result.get_key("minimum").unwrap().as_number(), Some(42.0));
        assert_eq!(result.get_key("maximum").unwrap().as_number(), Some(42.0));
        assert!(result.get_key("enum").is_none());
    }

    #[test]
    fn not_not_eliminated() {
        let input = r#"{"not": {"not": {"type": "string"}}}"#;
        let expected = r#"{"type": "string"}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(NotNot)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let expected_tree = parse(expected).unwrap();
        assert!(result.structural_eq(&expected_tree));
    }

    #[test]
    fn not_anyof_de_morgan() {
        let input = r#"{"not": {"anyOf": [{"type": "string"}, {"type": "number"}]}}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(NotAnyOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let allof = result.get_key("allOf").unwrap().as_array().unwrap();
        assert_eq!(allof.len(), 2);
        assert!(allof[0].get_key("not").is_some());
    }

    #[test]
    fn not_allof_de_morgan() {
        let input = r#"{"not": {"allOf": [{"type": "string"}, {"type": "number"}]}}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(NotAllOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let anyof = result.get_key("anyOf").unwrap().as_array().unwrap();
        assert_eq!(anyof.len(), 2);
    }

    #[test]
    fn singleton_allof_unwrapped() {
        let input = r#"{"allOf": [{"type": "string"}]}"#;
        let expected = r#"{"type": "string"}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(SingletonAllOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let expected_tree = parse(expected).unwrap();
        assert!(result.structural_eq(&expected_tree));
    }

    #[test]
    fn singleton_anyof_unwrapped() {
        let input = r#"{"anyOf": [{"type": "string"}]}"#;
        let expected = r#"{"type": "string"}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(SingletonAnyOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let expected_tree = parse(expected).unwrap();
        assert!(result.structural_eq(&expected_tree));
    }

    #[test]
    fn empty_allof_is_top() {
        let input = r#"{"allOf": []}"#;
        let expected = r#"{}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(EmptyAllOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let expected_tree = parse(expected).unwrap();
        assert!(result.structural_eq(&expected_tree));
    }

    #[test]
    fn empty_anyof_is_bottom() {
        let input = r#"{"anyOf": []}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(EmptyAnyOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("not").is_some());
    }

    #[test]
    fn intersect_heterogeneous_types_is_bottom() {
        let input = r#"{"allOf": [{"type": "string"}, {"type": "number"}]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(IntersectHeterogeneousTypes)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("not").is_some());
    }

    #[test]
    fn flatten_nested_allof() {
        let input = r#"{"allOf": [{"allOf": [{"type": "string"}, {"minLength": 1}]}, {"maxLength": 10}]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(FlattenAllOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let allof = result.get_key("allOf").unwrap().as_array().unwrap();
        assert_eq!(allof.len(), 3);
    }

    #[test]
    fn flatten_nested_anyof() {
        let input = r#"{"anyOf": [{"anyOf": [{"type": "string"}, {"type": "number"}]}, {"type": "null"}]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(FlattenAnyOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let anyof = result.get_key("anyOf").unwrap().as_array().unwrap();
        assert_eq!(anyof.len(), 3);
    }

    #[test]
    fn allof_with_bottom_collapses() {
        let input = r#"{"allOf": [{"type": "string"}, {"not": {}}]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(AllOfWithBottom)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("not").is_some());
        assert!(result.get_key("allOf").is_none());
    }

    #[test]
    fn anyof_with_top_collapses() {
        let input = r#"{"anyOf": [{"type": "string"}, {}]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(AnyOfWithTop)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.as_object().unwrap().is_empty());
    }
}
