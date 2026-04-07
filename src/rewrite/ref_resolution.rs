use crate::error::SubtypeError;
use crate::located::{JsonF, LocatedValue};

/// Resolve all local `$ref`s (like `#/$defs/foo`) in the tree.
///
/// Walks the tree top-down. When encountering `{"$ref": "#/path/to/def"}`,
/// resolves the JSON pointer against the root document and replaces the
/// `$ref` node with a copy of the referenced schema, merging provenance.
///
/// Only handles local references (`#/...`). External references produce an error.
pub fn resolve_refs(root: &LocatedValue, tree: &LocatedValue) -> Result<LocatedValue, SubtypeError> {
    match &tree.node {
        JsonF::Object(pairs) => {
            if let Some(ref_val) = get_ref(pairs) {
                let ref_str = ref_val.as_str().ok_or_else(|| SubtypeError::InvalidSchema {
                    source: "$ref must be a string".into(),
                })?;

                if !ref_str.starts_with('#') {
                    return Err(SubtypeError::UnsupportedFeatures {
                        details: format!("external $ref not supported: {ref_str}"),
                    });
                }

                // Parse JSON pointer: "#/$defs/foo" → ["$defs", "foo"]
                let pointer = &ref_str[1..]; // strip '#'
                let resolved = resolve_pointer(root, pointer)?;

                // Merge provenance: the $ref location + the definition location
                let prov = tree.provenance.merge(&resolved.provenance);
                let mut result = resolved.clone();
                result.provenance = prov;

                // Recursively resolve refs in the resolved schema
                resolve_refs(root, &result)
            } else {
                // No $ref — recurse into children
                let new_pairs = pairs
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), resolve_refs(root, v)?)))
                    .collect::<Result<Vec<_>, SubtypeError>>()?;
                Ok(LocatedValue::new(tree.provenance.clone(), JsonF::Object(new_pairs)))
            }
        }
        JsonF::Array(items) => {
            let new_items = items
                .iter()
                .map(|item| resolve_refs(root, item))
                .collect::<Result<Vec<_>, SubtypeError>>()?;
            Ok(LocatedValue::new(tree.provenance.clone(), JsonF::Array(new_items)))
        }
        // Leaf nodes: no resolution needed
        _ => Ok(tree.clone()),
    }
}

fn get_ref(pairs: &[(LocatedValue, LocatedValue)]) -> Option<&LocatedValue> {
    pairs.iter().find_map(|(k, v)| {
        if k.as_str() == Some("$ref") { Some(v) } else { None }
    })
}

/// Resolve a JSON Pointer path against a LocatedValue tree.
fn resolve_pointer(root: &LocatedValue, pointer: &str) -> Result<LocatedValue, SubtypeError> {
    if pointer.is_empty() || pointer == "/" {
        return Ok(root.clone());
    }

    // Split on '/' and skip the leading empty segment
    let segments: Vec<&str> = pointer.split('/').skip(1).collect();
    let mut current = root;

    for segment in &segments {
        // Unescape JSON Pointer encoding: ~1 → /, ~0 → ~
        let key = segment.replace("~1", "/").replace("~0", "~");

        match &current.node {
            JsonF::Object(pairs) => {
                current = pairs
                    .iter()
                    .find_map(|(k, v)| {
                        if k.as_str() == Some(&key) { Some(v) } else { None }
                    })
                    .ok_or_else(|| SubtypeError::InvalidSchema {
                        source: format!("$ref pointer segment not found: {key} in {pointer}").into(),
                    })?;
            }
            JsonF::Array(items) => {
                let idx: usize = key.parse().map_err(|_| SubtypeError::InvalidSchema {
                    source: format!("$ref pointer: non-numeric array index: {key}").into(),
                })?;
                current = items.get(idx).ok_or_else(|| SubtypeError::InvalidSchema {
                    source: format!("$ref pointer: array index out of bounds: {idx}").into(),
                })?;
            }
            _ => {
                return Err(SubtypeError::InvalidSchema {
                    source: format!("$ref pointer: cannot traverse into non-container at {key}").into(),
                });
            }
        }
    }

    Ok(current.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn resolve_local_ref() {
        let input = r##"{
            "$defs": {"pos_int": {"type": "integer", "minimum": 0}},
            "type": "object",
            "properties": {"age": {"$ref": "#/$defs/pos_int"}}
        }"##;
        let tree = parse(input).unwrap();
        let resolved = resolve_refs(&tree, &tree).unwrap();
        let props = resolved.get_key("properties").unwrap();
        let age = props.get_key("age").unwrap();
        assert_eq!(age.get_key("type").unwrap().as_str(), Some("integer"));
        assert_eq!(age.get_key("minimum").unwrap().as_number(), Some(0.0));
    }

    #[test]
    fn no_ref_unchanged() {
        let input = r#"{"type": "string"}"#;
        let tree = parse(input).unwrap();
        let resolved = resolve_refs(&tree, &tree).unwrap();
        assert!(resolved.structural_eq(&tree));
    }

    #[test]
    fn external_ref_errors() {
        let input = r#"{"$ref": "https://example.com/schema.json"}"#;
        let tree = parse(input).unwrap();
        let result = resolve_refs(&tree, &tree);
        assert!(result.is_err());
    }

    #[test]
    fn nested_ref() {
        let input = r##"{
            "$defs": {
                "name": {"type": "string"},
                "person": {
                    "type": "object",
                    "properties": {"name": {"$ref": "#/$defs/name"}}
                }
            },
            "$ref": "#/$defs/person"
        }"##;
        let tree = parse(input).unwrap();
        let resolved = resolve_refs(&tree, &tree).unwrap();
        assert_eq!(resolved.get_key("type").unwrap().as_str(), Some("object"));
        let props = resolved.get_key("properties").unwrap();
        let name = props.get_key("name").unwrap();
        assert_eq!(name.get_key("type").unwrap().as_str(), Some("string"));
    }

    #[test]
    fn ref_to_def() {
        let input = r##"{
            "$defs": {"all": {}},
            "items": {"$ref": "#/$defs/all"}
        }"##;
        let tree = parse(input).unwrap();
        let resolved = resolve_refs(&tree, &tree).unwrap();
        let items = resolved.get_key("items").unwrap();
        assert!(items.as_object().unwrap().is_empty());
    }
}
