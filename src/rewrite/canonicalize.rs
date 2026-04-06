use crate::error::SubtypeError;
use crate::located::{JsonF, LocatedValue, Provenance};
use crate::rewrite::RewriteRule;

/// Helper: build a synthetic LocatedValue with given provenance.
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

/// Get the value for a key in an object's pairs.
fn get_entry<'a>(pairs: &'a [(LocatedValue, LocatedValue)], key: &str) -> Option<&'a LocatedValue> {
    pairs.iter().find_map(|(k, v)| {
        if k.as_str() == Some(key) {
            Some(v)
        } else {
            None
        }
    })
}

/// Does this object have a given key?
fn has_key(pairs: &[(LocatedValue, LocatedValue)], key: &str) -> bool {
    get_entry(pairs, key).is_some()
}

/// Remove a key from an object, returning remaining pairs.
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

/// Remove multiple keys from an object.
fn without_keys(
    pairs: &[(LocatedValue, LocatedValue)],
    keys: &[&str],
) -> Vec<(LocatedValue, LocatedValue)> {
    pairs
        .iter()
        .filter(|(k, _)| k.as_str().map_or(true, |s| !keys.contains(&s)))
        .cloned()
        .collect()
}

// --- Rule: reject unsupported keywords (fail-closed) ---

/// Keywords we don't yet support. Schemas containing these produce an error.
const UNSUPPORTED_KEYWORDS: &[&str] = &[
    "$ref",
    "$dynamicRef",
    "$dynamicAnchor",
    "unevaluatedItems",
    "unevaluatedProperties",
];

pub struct UnsupportedKeywords;

impl RewriteRule for UnsupportedKeywords {
    fn rewrite(
        &self,
        _prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        for (k, _) in pairs {
            if let Some(key) = k.as_str() {
                if UNSUPPORTED_KEYWORDS.contains(&key) {
                    return Err(SubtypeError::UnsupportedFeatures {
                        details: format!("unsupported keyword: {key}"),
                    });
                }
            }
        }
        Ok(None)
    }
}

// --- Rule: const → enum ---

pub struct ConstToEnum;

impl RewriteRule for ConstToEnum {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(const_val) = get_entry(pairs, "const") else {
            return Ok(None);
        };

        let mut new_pairs = without_key(pairs, "const");
        let enum_array = make_array(prov, vec![const_val.clone()]);
        new_pairs.push((make_string(prov, "enum"), enum_array));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// --- Rule: if/then/else → anyOf/allOf/not ---

pub struct IfThenElse;

impl RewriteRule for IfThenElse {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(if_schema) = get_entry(pairs, "if") else {
            return Ok(None);
        };

        let then_schema = get_entry(pairs, "then");
        let else_schema = get_entry(pairs, "else");

        if then_schema.is_none() && else_schema.is_none() {
            return Ok(None);
        }

        let not_if = make_object(prov, vec![("not", if_schema.clone())]);

        let branches = match (then_schema, else_schema) {
            (Some(then_s), Some(else_s)) => {
                let branch1 = make_object(
                    prov,
                    vec![(
                        "allOf",
                        make_array(prov, vec![if_schema.clone(), then_s.clone()]),
                    )],
                );
                let branch2 = make_object(
                    prov,
                    vec![(
                        "allOf",
                        make_array(prov, vec![not_if, else_s.clone()]),
                    )],
                );
                vec![branch1, branch2]
            }
            (Some(then_s), None) => {
                vec![not_if, then_s.clone()]
            }
            (None, Some(else_s)) => {
                vec![if_schema.clone(), else_s.clone()]
            }
            (None, None) => unreachable!(),
        };

        let remaining = without_keys(pairs, &["if", "then", "else"]);
        if remaining.is_empty() {
            Ok(Some(JsonF::Object(vec![(
                make_string(prov, "anyOf"),
                make_array(prov, branches),
            )])))
        } else {
            let anyof_part = make_object(prov, vec![("anyOf", make_array(prov, branches))]);
            let rest_part = synthetic(prov, JsonF::Object(remaining));
            Ok(Some(JsonF::Object(vec![(
                make_string(prov, "allOf"),
                make_array(prov, vec![rest_part, anyof_part]),
            )])))
        }
    }
}

// --- Rule: multiple types → anyOf ---

pub struct MultipleTypes;

impl RewriteRule for MultipleTypes {
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
        let Some(types) = type_val.as_array() else {
            return Ok(None);
        };

        if types.len() <= 1 {
            return Ok(None);
        }

        let other_pairs = without_key(pairs, "type");
        let branches: Vec<LocatedValue> = types
            .iter()
            .map(|t| {
                let mut branch_pairs = other_pairs.clone();
                branch_pairs.push((make_string(prov, "type"), t.clone()));
                synthetic(prov, JsonF::Object(branch_pairs))
            })
            .collect();

        Ok(Some(JsonF::Object(vec![(
            make_string(prov, "anyOf"),
            make_array(prov, branches),
        )])))
    }
}

// --- Rule: multiple connectives → allOf ---

pub struct MultipleConnectives;

/// Annotation/metadata keywords that don't affect validation semantics.
/// These are kept alongside whatever they're with — they don't trigger separation.
const ANNOTATION_KEYS: &[&str] = &[
    "$schema",
    "$comment",
    "$id",
    "$anchor",
    "$defs",
    "title",
    "description",
    "default",
    "examples",
    "readOnly",
    "writeOnly",
    "deprecated",
    "contentMediaType",
    "contentEncoding",
    "contentSchema",
];

impl RewriteRule for MultipleConnectives {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };

        let connectives = ["enum", "anyOf", "allOf", "oneOf", "not"];
        let present: Vec<&str> = connectives
            .iter()
            .filter(|c| has_key(pairs, c))
            .copied()
            .collect();

        if present.is_empty() {
            return Ok(None);
        }

        // Separate non-connective, non-annotation keys (validation keywords).
        let validation_keys: Vec<_> = pairs
            .iter()
            .filter(|(k, _)| {
                k.as_str().map_or(true, |s| {
                    !connectives.contains(&s) && !ANNOTATION_KEYS.contains(&s)
                })
            })
            .cloned()
            .collect();

        // Fire if there are validation keywords to separate from connectives,
        // OR if there are multiple connectives that need wrapping in allOf.
        let needs_separation = !validation_keys.is_empty() || present.len() > 1;
        if !needs_separation {
            return Ok(None);
        }

        // Each connective becomes its own allOf member.
        let mut allof_members: Vec<LocatedValue> = present
            .iter()
            .map(|&c| {
                let val = get_entry(pairs, c).unwrap().clone();
                make_object(prov, vec![(c, val)])
            })
            .collect();

        // Validation keywords go in their own member (if any).
        if !validation_keys.is_empty() {
            allof_members.push(synthetic(prov, JsonF::Object(validation_keys)));
        }

        // Annotations stay at the top level alongside allOf.
        let annotations: Vec<_> = pairs
            .iter()
            .filter(|(k, _)| {
                k.as_str()
                    .is_some_and(|s| ANNOTATION_KEYS.contains(&s))
            })
            .cloned()
            .collect();

        let mut result_pairs = annotations;
        result_pairs.push((
            make_string(prov, "allOf"),
            make_array(prov, allof_members),
        ));

        Ok(Some(JsonF::Object(result_pairs)))
    }
}

// --- Rule: missing type → add type: [all types] ---

/// Known JSON Schema validation/applicator keywords. An object must contain
/// at least one of these to be treated as a schema (not a data value).
const SCHEMA_KEYWORDS: &[&str] = &[
    "properties",
    "patternProperties",
    "additionalProperties",
    "items",
    "prefixItems",
    "required",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "minLength",
    "maxLength",
    "pattern",
    "format",
    "minItems",
    "maxItems",
    "uniqueItems",
    "contains",
    "minContains",
    "maxContains",
    "minProperties",
    "maxProperties",
    "propertyNames",
    "dependentRequired",
    "dependentSchemas",
    "if",
    "then",
    "else",
    "$ref",
    "$defs",
];

pub struct MissingType;

impl RewriteRule for MissingType {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };

        let has_type_or_connective = ["type", "enum", "anyOf", "allOf", "oneOf", "not"]
            .iter()
            .any(|k| has_key(pairs, k));

        if has_type_or_connective {
            return Ok(None);
        }
        if pairs.is_empty() {
            return Ok(None);
        }

        // Only fire on objects that look like schemas (contain a known schema keyword).
        // This prevents mangling data values inside const/enum/default.
        let has_schema_keyword = SCHEMA_KEYWORDS.iter().any(|k| has_key(pairs, k));
        if !has_schema_keyword {
            return Ok(None);
        }

        let all_types = [
            "null", "boolean", "object", "array", "number", "string", "integer",
        ]
        .iter()
        .map(|t| make_string(prov, t))
        .collect();
        let mut new_pairs = pairs.to_vec();
        new_pairs.push((make_string(prov, "type"), make_array(prov, all_types)));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// ============================================================
// Type-specific canonicalization rules (Figure 6 in the paper)
// ============================================================

// --- Rule: integer → number with multipleOf ---

pub struct IntegerToNumber;

impl RewriteRule for IntegerToNumber {
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
        if type_val.as_str() != Some("integer") {
            return Ok(None);
        }

        let mut new_pairs = without_key(pairs, "type");
        new_pairs.push((make_string(prov, "type"), make_string(prov, "number")));

        if !has_key(pairs, "multipleOf") {
            new_pairs.push((
                make_string(prov, "multipleOf"),
                synthetic(prov, JsonF::Number(1.0)),
            ));
        }
        // If multipleOf already set, lcm(1, x) = x, so keep existing.

        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// --- Rule: heterogeneous enum → anyOf grouped by type ---

/// Classify a LocatedValue into its JSON type name.
fn json_type_of(val: &LocatedValue) -> &'static str {
    match &val.node {
        JsonF::Null => "null",
        JsonF::Bool(_) => "boolean",
        JsonF::Number(_) => "number",
        JsonF::String(_) => "string",
        JsonF::Array(_) => "array",
        JsonF::Object(_) => "object",
    }
}

pub struct HeterogeneousEnum;

impl RewriteRule for HeterogeneousEnum {
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
        if values.is_empty() {
            return Ok(None);
        }

        // Check if all values are the same type.
        let first_type = json_type_of(&values[0]);
        let all_same = values.iter().all(|v| json_type_of(v) == first_type);
        if all_same {
            return Ok(None);
        }

        // Group by type, build anyOf branches.
        let type_order = ["null", "boolean", "number", "string", "array", "object"];
        let other_pairs = without_key(pairs, "enum");
        let mut branches = Vec::new();

        for ty in &type_order {
            let type_values: Vec<_> = values
                .iter()
                .filter(|v| json_type_of(v) == *ty)
                .cloned()
                .collect();
            if type_values.is_empty() {
                continue;
            }
            let mut branch_pairs = other_pairs.clone();
            branch_pairs.push((
                make_string(prov, "enum"),
                make_array(prov, type_values),
            ));
            branches.push(synthetic(prov, JsonF::Object(branch_pairs)));
        }

        Ok(Some(JsonF::Object(vec![(
            make_string(prov, "anyOf"),
            make_array(prov, branches),
        )])))
    }
}

// --- Rule: oneOf → anyOf with mutual exclusion ---

pub struct OneOfToAnyOf;

impl RewriteRule for OneOfToAnyOf {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(oneof_val) = get_entry(pairs, "oneOf") else {
            return Ok(None);
        };
        let Some(schemas) = oneof_val.as_array() else {
            return Ok(None);
        };

        // oneOf: [s1, ..., sn] → anyOf: [
        //   {allOf: [s1, {not: s2}, ..., {not: sn}]},
        //   {allOf: [{not: s1}, s2, {not: s3}, ..., {not: sn}]},
        //   ...
        // ]
        let branches: Vec<LocatedValue> = schemas
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let allof_members: Vec<LocatedValue> = schemas
                    .iter()
                    .enumerate()
                    .map(|(j, s)| {
                        if i == j {
                            s.clone()
                        } else {
                            make_object(prov, vec![("not", s.clone())])
                        }
                    })
                    .collect();
                make_object(
                    prov,
                    vec![("allOf", make_array(prov, allof_members))],
                )
            })
            .collect();

        let mut new_pairs = without_key(pairs, "oneOf");
        new_pairs.push((
            make_string(prov, "anyOf"),
            make_array(prov, branches),
        ));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// --- Rule: additionalProperties: false → additionalProperties: {not: {}} ---

pub struct AdditionalPropertiesFalse;

impl RewriteRule for AdditionalPropertiesFalse {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(ap) = get_entry(pairs, "additionalProperties") else {
            return Ok(None);
        };
        if ap.as_bool() != Some(false) {
            return Ok(None);
        }

        let bottom = make_object(prov, vec![("not", make_object(prov, vec![]))]);
        let mut new_pairs = without_key(pairs, "additionalProperties");
        new_pairs.push((make_string(prov, "additionalProperties"), bottom));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// --- Rule: properties + additionalProperties → patternProperties ---

/// Escape a literal string for use in a regex pattern.
fn regex_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    result.push('^');
    for c in s.chars() {
        if "\\^$.|?*+()[]{}".contains(c) {
            result.push('\\');
        }
        result.push(c);
    }
    result.push('$');
    result
}

pub struct ObjectWithProperties;

impl RewriteRule for ObjectWithProperties {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        if !has_key(pairs, "properties") {
            return Ok(None);
        }

        let props = get_entry(pairs, "properties").unwrap();
        let prop_pairs = match props.as_object() {
            Some(p) => p,
            None => return Ok(None),
        };

        // Collect existing patternProperties if any.
        let existing_pp = get_entry(pairs, "patternProperties")
            .and_then(|pp| pp.as_object())
            .unwrap_or(&[]);

        let mut pp_entries: Vec<(LocatedValue, LocatedValue)> = existing_pp.to_vec();

        // Convert each property to a pattern: ^propertyName$ → schema
        for (key_lv, val_lv) in prop_pairs {
            if let Some(key_str) = key_lv.as_str() {
                let pattern = regex_escape(key_str);
                pp_entries.push((make_string(prov, &pattern), val_lv.clone()));
            }
        }

        // Handle additionalProperties as catch-all pattern.
        // If additionalProperties is present and is a schema (not just removed),
        // we'd need to compute a "not any of the property patterns" regex.
        // For now, keep additionalProperties if present (will be handled by
        // overlapping pattern properties + regex algebra later).

        let mut new_pairs =
            without_keys(pairs, &["properties", "patternProperties"]);
        new_pairs.push((
            make_string(prov, "patternProperties"),
            synthetic(prov, JsonF::Object(pp_entries)),
        ));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// --- Rule: dependentRequired → dependentSchemas ---

pub struct DependentRequired;

impl RewriteRule for DependentRequired {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(dep_req) = get_entry(pairs, "dependentRequired") else {
            return Ok(None);
        };
        let Some(dep_pairs) = dep_req.as_object() else {
            return Ok(None);
        };

        // dependentRequired: {k: [k1,...]} →
        // dependentSchemas: {k: {required: [k1,...]}}
        let schema_pairs: Vec<(LocatedValue, LocatedValue)> = dep_pairs
            .iter()
            .map(|(key, req_array)| {
                let schema = make_object(prov, vec![("required", req_array.clone())]);
                (key.clone(), schema)
            })
            .collect();

        let mut new_pairs = without_key(pairs, "dependentRequired");
        // Merge with existing dependentSchemas if present.
        if let Some(existing) = get_entry(pairs, "dependentSchemas") {
            if let Some(existing_pairs) = existing.as_object() {
                let mut merged = existing_pairs.to_vec();
                merged.extend(schema_pairs);
                new_pairs = without_keys(&new_pairs, &["dependentSchemas"]);
                new_pairs.push((
                    make_string(prov, "dependentSchemas"),
                    synthetic(prov, JsonF::Object(merged)),
                ));
                return Ok(Some(JsonF::Object(new_pairs)));
            }
        }
        new_pairs.push((
            make_string(prov, "dependentSchemas"),
            synthetic(prov, JsonF::Object(schema_pairs)),
        ));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// --- Rule: dependentSchemas → allOf/anyOf ---

pub struct DependentSchemas;

impl RewriteRule for DependentSchemas {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else {
            return Ok(None);
        };
        let Some(dep_schemas) = get_entry(pairs, "dependentSchemas") else {
            return Ok(None);
        };
        let Some(dep_pairs) = dep_schemas.as_object() else {
            return Ok(None);
        };
        if dep_pairs.is_empty() {
            return Ok(None);
        }

        // dependentSchemas: {k: s} →
        // allOf: [original_without_dep, {anyOf: [s, {not: {required: [k]}}]}]
        // for each k, chained in allOf
        let remaining = without_key(pairs, "dependentSchemas");
        let mut allof_members = vec![synthetic(prov, JsonF::Object(remaining))];

        for (key, schema) in dep_pairs {
            // {anyOf: [s, {not: {required: [k]}}]}
            let not_has_key = make_object(
                prov,
                vec![(
                    "not",
                    make_object(
                        prov,
                        vec![("required", make_array(prov, vec![key.clone()]))],
                    ),
                )],
            );
            let branch = make_object(
                prov,
                vec![(
                    "anyOf",
                    make_array(prov, vec![schema.clone(), not_has_key]),
                )],
            );
            allof_members.push(branch);
        }

        Ok(Some(JsonF::Object(vec![(
            make_string(prov, "allOf"),
            make_array(prov, allof_members),
        )])))
    }
}

// --- Rule: add default keywords for typed schemas ---

pub struct MissingKeyword;

/// Default values for type-specific keywords when absent.
impl RewriteRule for MissingKeyword {
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
        let Some(type_str) = type_val.as_str() else {
            return Ok(None);
        };

        let mut new_pairs = pairs.to_vec();
        let mut changed = false;

        match type_str {
            "number" => {
                if !has_key(pairs, "minimum") {
                    new_pairs.push((
                        make_string(prov, "minimum"),
                        synthetic(prov, JsonF::Number(f64::NEG_INFINITY)),
                    ));
                    changed = true;
                }
                if !has_key(pairs, "maximum") {
                    new_pairs.push((
                        make_string(prov, "maximum"),
                        synthetic(prov, JsonF::Number(f64::INFINITY)),
                    ));
                    changed = true;
                }
                if !has_key(pairs, "exclusiveMinimum") {
                    new_pairs.push((
                        make_string(prov, "exclusiveMinimum"),
                        synthetic(prov, JsonF::Number(f64::NEG_INFINITY)),
                    ));
                    changed = true;
                }
                if !has_key(pairs, "exclusiveMaximum") {
                    new_pairs.push((
                        make_string(prov, "exclusiveMaximum"),
                        synthetic(prov, JsonF::Number(f64::INFINITY)),
                    ));
                    changed = true;
                }
            }
            "array" => {
                if !has_key(pairs, "minItems") {
                    new_pairs.push((
                        make_string(prov, "minItems"),
                        synthetic(prov, JsonF::Number(0.0)),
                    ));
                    changed = true;
                }
                if !has_key(pairs, "uniqueItems") {
                    new_pairs.push((
                        make_string(prov, "uniqueItems"),
                        synthetic(prov, JsonF::Bool(false)),
                    ));
                    changed = true;
                }
            }
            "object" => {
                if !has_key(pairs, "minProperties") {
                    new_pairs.push((
                        make_string(prov, "minProperties"),
                        synthetic(prov, JsonF::Number(0.0)),
                    ));
                    changed = true;
                }
                if !has_key(pairs, "required") {
                    new_pairs.push((
                        make_string(prov, "required"),
                        make_array(prov, vec![]),
                    ));
                    changed = true;
                }
            }
            _ => {}
        }

        if changed {
            Ok(Some(JsonF::Object(new_pairs)))
        } else {
            Ok(None)
        }
    }
}

// --- Rule: strip keywords irrelevant to the schema's type ---

pub struct IrrelevantKeywords;

impl RewriteRule for IrrelevantKeywords {
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
        let Some(type_str) = type_val.as_str() else {
            return Ok(None);
        };

        let relevant: &[&str] = match type_str {
            "null" => &["type"],
            "boolean" => &["type", "enum"],
            "string" => &["type", "minLength", "maxLength", "pattern", "format", "enum"],
            "number" => &[
                "type",
                "minimum",
                "maximum",
                "exclusiveMinimum",
                "exclusiveMaximum",
                "multipleOf",
                "enum",
            ],
            "array" => &[
                "type",
                "minItems",
                "maxItems",
                "prefixItems",
                "items",
                "uniqueItems",
                "contains",
                "minContains",
                "maxContains",
                "enum",
            ],
            "object" => &[
                "type",
                "minProperties",
                "maxProperties",
                "properties",
                "patternProperties",
                "additionalProperties",
                "required",
                "propertyNames",
                "dependentRequired",
                "dependentSchemas",
                "enum",
            ],
            _ => return Ok(None),
        };

        let filtered: Vec<_> = pairs
            .iter()
            .filter(|(k, _)| {
                k.as_str()
                    .map_or(true, |s| relevant.contains(&s) || ANNOTATION_KEYS.contains(&s))
            })
            .cloned()
            .collect();

        if filtered.len() == pairs.len() {
            Ok(None)
        } else {
            Ok(Some(JsonF::Object(filtered)))
        }
    }
}

/// Returns all non-type-specific canonicalization rules in application order.
pub fn non_type_specific_rules() -> Vec<Box<dyn RewriteRule>> {
    vec![
        Box::new(UnsupportedKeywords), // must be first: fail-closed
        Box::new(ConstToEnum),
        Box::new(IfThenElse),
        Box::new(MultipleTypes),
        Box::new(MultipleConnectives),
        Box::new(MissingType),
    ]
}

/// Returns all type-specific canonicalization rules in application order.
pub fn type_specific_rules() -> Vec<Box<dyn RewriteRule>> {
    vec![
        Box::new(IntegerToNumber),
        Box::new(HeterogeneousEnum),
        Box::new(OneOfToAnyOf),
        Box::new(AdditionalPropertiesFalse),
        Box::new(ObjectWithProperties),
        Box::new(DependentRequired),
        Box::new(DependentSchemas),
        Box::new(IrrelevantKeywords),
        Box::new(MissingKeyword),
        // String canonicalization and overlapping pattern properties
        // deferred to after Task 12 (regex algebra).
    ]
}

/// Returns all canonicalization rules (non-type-specific + type-specific).
pub fn all_canonicalization_rules() -> Vec<Box<dyn RewriteRule>> {
    let mut rules = non_type_specific_rules();
    rules.extend(type_specific_rules());
    rules
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::rewrite::rewrite_phase;

    #[test]
    fn const_to_enum() {
        let input = r#"{"const": 42}"#;
        let expected = r#"{"enum": [42]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(ConstToEnum)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let expected_tree = parse(expected).unwrap();
        assert!(result.structural_eq(&expected_tree));
    }

    #[test]
    fn multiple_types_splits_into_anyof() {
        let input = r#"{"type": ["string", "null"], "minLength": 1}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(MultipleTypes)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let anyof = result.get_key("anyOf").unwrap().as_array().unwrap();
        assert_eq!(anyof.len(), 2);
    }

    #[test]
    fn if_then_else_rewrite() {
        let input = r#"{"if": {"type": "string"}, "then": {"minLength": 1}, "else": {"type": "number"}}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(IfThenElse)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("anyOf").is_some());
    }

    #[test]
    fn missing_type_adds_all_types() {
        let input = r#"{"minLength": 5}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(MissingType)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let type_val = result.get_key("type").unwrap();
        let types = type_val.as_array().unwrap();
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn missing_type_skips_empty_object() {
        let input = r#"{}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(MissingType)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("type").is_none());
    }

    #[test]
    fn multiple_connectives_separates() {
        let input = r#"{"enum": [1, 2], "minLength": 5}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(MultipleConnectives)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let allof = result.get_key("allOf").unwrap().as_array().unwrap();
        assert_eq!(allof.len(), 2);
    }

    // --- Type-specific rule tests ---

    #[test]
    fn integer_to_number() {
        let input = r#"{"type": "integer", "minimum": 0}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(IntegerToNumber)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert_eq!(result.get_key("type").unwrap().as_str(), Some("number"));
        assert_eq!(result.get_key("multipleOf").unwrap().as_number(), Some(1.0));
    }

    #[test]
    fn integer_to_number_preserves_existing_multiple_of() {
        let input = r#"{"type": "integer", "multipleOf": 3}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(IntegerToNumber)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert_eq!(result.get_key("type").unwrap().as_str(), Some("number"));
        assert_eq!(result.get_key("multipleOf").unwrap().as_number(), Some(3.0));
    }

    #[test]
    fn heterogeneous_enum_splits() {
        let input = r#"{"enum": [1, "hello", true]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(HeterogeneousEnum)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let anyof = result.get_key("anyOf").unwrap().as_array().unwrap();
        assert_eq!(anyof.len(), 3); // boolean, number, string groups
    }

    #[test]
    fn homogeneous_enum_unchanged() {
        let input = r#"{"enum": [1, 2, 3]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(HeterogeneousEnum)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("enum").is_some());
        assert!(result.get_key("anyOf").is_none());
    }

    #[test]
    fn oneof_to_anyof() {
        let input = r#"{"oneOf": [{"type": "string"}, {"type": "number"}]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(OneOfToAnyOf)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("anyOf").is_some());
        assert!(result.get_key("oneOf").is_none());
        let branches = result.get_key("anyOf").unwrap().as_array().unwrap();
        assert_eq!(branches.len(), 2);
    }

    #[test]
    fn additional_properties_false_to_not() {
        let input = r#"{"type": "object", "additionalProperties": false}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(AdditionalPropertiesFalse)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let ap = result.get_key("additionalProperties").unwrap();
        assert!(ap.get_key("not").is_some());
    }

    #[test]
    fn properties_to_pattern_properties() {
        let input = r#"{
            "type": "object",
            "properties": {"name": {"type": "string"}}
        }"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(ObjectWithProperties)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("properties").is_none());
        let pp = result.get_key("patternProperties").unwrap();
        assert!(pp.as_object().is_some());
    }

    #[test]
    fn dependent_required_to_schemas() {
        let input = r#"{"dependentRequired": {"foo": ["bar"]}}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(DependentRequired)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("dependentRequired").is_none());
        let ds = result.get_key("dependentSchemas").unwrap();
        let foo_schema = ds.get_key("foo").unwrap();
        assert!(foo_schema.get_key("required").is_some());
    }

    #[test]
    fn dependent_schemas_to_allof() {
        let input = r#"{"type": "object", "dependentSchemas": {"foo": {"required": ["bar"]}}}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(DependentSchemas)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("dependentSchemas").is_none());
        assert!(result.get_key("allOf").is_some());
    }

    #[test]
    fn missing_keyword_adds_number_defaults() {
        let input = r#"{"type": "number"}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(MissingKeyword)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("minimum").is_some());
        assert!(result.get_key("maximum").is_some());
        assert!(result.get_key("exclusiveMinimum").is_some());
        assert!(result.get_key("exclusiveMaximum").is_some());
    }

    #[test]
    fn irrelevant_keywords_stripped() {
        let input = r#"{"type": "null", "minLength": 5, "minimum": 0}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(IrrelevantKeywords)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert_eq!(result.get_key("type").unwrap().as_str(), Some("null"));
        assert!(result.get_key("minLength").is_none());
        assert!(result.get_key("minimum").is_none());
    }

    #[test]
    fn unsupported_keyword_errors() {
        let input = r##"{"$ref": "#/defs/foo"}"##;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(UnsupportedKeywords)];
        let result = rewrite_phase(&tree, &rules);
        assert!(result.is_err());
    }
}
