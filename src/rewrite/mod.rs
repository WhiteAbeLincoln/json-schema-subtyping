pub mod canonicalize;
pub mod ref_resolution;
pub mod simplify;

use crate::error::SubtypeError;
use crate::located::{JsonF, LocatedValue, Provenance};

/// The three fixed rewrite phases, run in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    DraftConversion,
    Canonicalization,
    Simplification,
}

/// A per-node rewrite rule.
///
/// Returns `Ok(Some(new_node))` if the rule fires, `Ok(None)` if it doesn't
/// apply, or `Err` if the schema is malformed. The library traverses bottom-up
/// and runs all rules to fixed point per node.
pub trait RewriteRule: 'static {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError>;
}

/// JSON Schema keywords whose values are data (not sub-schemas).
/// The rewrite traversal must NOT recurse into these values, since rewrite
/// rules are schema-level transforms and would corrupt plain JSON data
/// (e.g., an object inside `const` or `enum`).
const DATA_VALUE_KEYS: &[&str] = &[
    "const",
    "enum",
    "default",
    "examples",
    "required",
    "dependentRequired",
    // Scalars below are harmless (no rules fire on them), but listed for clarity.
    "$schema",
    "$comment",
    "$id",
    "$anchor",
    "title",
    "description",
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
    "minProperties",
    "maxProperties",
    "minContains",
    "maxContains",
    "readOnly",
    "writeOnly",
    "deprecated",
    "contentMediaType",
    "contentEncoding",
    "$ref",
    "$dynamicRef",
    "$dynamicAnchor",
];

/// Apply all rules to a tree, bottom-up, to fixed point per node.
///
/// The traversal is schema-aware: it only recurses into object values
/// whose keys indicate sub-schemas (e.g., `properties`, `items`, `allOf`).
/// Values of data-only keys (e.g., `const`, `enum`) are left untouched.
pub fn rewrite_phase(
    tree: &LocatedValue,
    rules: &[Box<dyn RewriteRule>],
) -> Result<LocatedValue, SubtypeError> {
    if rules.is_empty() {
        return Ok(tree.clone());
    }

    // Bottom-up: first rewrite all children (schema-aware for objects)
    let rewritten_children = match &tree.node {
        JsonF::Null | JsonF::Bool(_) | JsonF::Number(_) | JsonF::String(_) => tree.node.clone(),
        JsonF::Array(items) => {
            let new_items = items
                .iter()
                .map(|item| rewrite_phase(item, rules))
                .collect::<Result<Vec<_>, _>>()?;
            JsonF::Array(new_items)
        }
        JsonF::Object(pairs) => {
            let new_pairs = pairs
                .iter()
                .map(|(k, v)| {
                    let is_data = k
                        .as_str()
                        .is_some_and(|s| DATA_VALUE_KEYS.contains(&s));
                    let new_v = if is_data {
                        v.clone()
                    } else {
                        rewrite_phase(v, rules)?
                    };
                    Ok((k.clone(), new_v))
                })
                .collect::<Result<Vec<_>, _>>()?;
            JsonF::Object(new_pairs)
        }
    };

    // Fixed-point: apply rules to this node until none fire
    let mut current = rewritten_children;
    loop {
        let mut changed = false;
        for rule in rules {
            if let Some(new_node) = rule.rewrite(&tree.provenance, &current)? {
                current = new_node;
                changed = true;
                break; // restart from first rule after a change
            }
        }
        if !changed {
            break;
        }
    }

    Ok(LocatedValue::new(tree.provenance.clone(), current))
}


#[cfg(test)]
mod tests {
    use super::*;

    /// A test rule that converts numbers to their string representation.
    /// Fires once per number node and then doesn't match (it's a string now).
    struct NumberToString;
    impl RewriteRule for NumberToString {
        fn rewrite(
            &self,
            _prov: &Provenance,
            node: &JsonF<LocatedValue>,
        ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
            if let JsonF::Number(n) = node {
                Ok(Some(JsonF::String(n.to_string())))
            } else {
                Ok(None)
            }
        }
    }

    #[test]
    fn rewrite_bottom_up_transforms_leaves() {
        let tree = crate::parse::parse(r#"{"a": 5, "b": [1, 2]}"#).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(NumberToString)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert_eq!(result.get_key("a").unwrap().as_str(), Some("5"));
        let arr = result.get_key("b").unwrap().as_array().unwrap();
        assert_eq!(arr[0].as_str(), Some("1"));
        assert_eq!(arr[1].as_str(), Some("2"));
    }

    /// A test rule that wraps a number in an array (fires once, then no longer matches).
    struct WrapNumberInArray;
    impl RewriteRule for WrapNumberInArray {
        fn rewrite(
            &self,
            prov: &Provenance,
            node: &JsonF<LocatedValue>,
        ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
            if let JsonF::Number(_) = node {
                let inner = LocatedValue::new(prov.clone(), node.clone());
                Ok(Some(JsonF::Array(vec![inner])))
            } else {
                Ok(None)
            }
        }
    }

    #[test]
    fn rewrite_fixed_point_stops_when_no_rule_fires() {
        let tree = crate::parse::parse("42").unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(WrapNumberInArray)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        // Number wrapped in array, then WrapNumberInArray doesn't fire on the array
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].as_number(), Some(42.0));
    }

    /// A rule that returns an error for negative numbers.
    struct RejectNegative;
    impl RewriteRule for RejectNegative {
        fn rewrite(
            &self,
            _prov: &Provenance,
            node: &JsonF<LocatedValue>,
        ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
            if let JsonF::Number(n) = node {
                if *n < 0.0 {
                    return Err(SubtypeError::RewriteFailed {
                        message: "negative number".into(),
                    });
                }
            }
            Ok(None)
        }
    }

    #[test]
    fn rewrite_error_propagates() {
        let tree = crate::parse::parse("-5").unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(RejectNegative)];
        let result = rewrite_phase(&tree, &rules);
        assert!(result.is_err());
    }
}
