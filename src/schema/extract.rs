use crate::error::SubtypeError;
use crate::located::{JsonF, Located, LocatedValue, Provenance};
use super::boolset::BoolSet;
use super::reduced::*;

/// Annotation/metadata keywords that don't affect validation semantics.
const ANNOTATION_KEYWORDS: &[&str] = &[
    "$schema", "$id", "$anchor", "$defs", "$comment",
    "title", "description", "default", "examples",
    "deprecated", "readOnly", "writeOnly",
    "contentMediaType", "contentEncoding", "contentSchema",
    "format",
];

/// Keywords we know about and can handle after canonicalization.
/// Anything NOT in this list triggers a fail-closed error.
const KNOWN_KEYWORDS: &[&str] = &[
    // type & connectives (should be resolved by canonicalization)
    "type", "enum",
    "allOf", "anyOf", "not",
    // string
    "minLength", "maxLength", "pattern",
    // number
    "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf",
    // array
    "prefixItems", "items", "minItems", "maxItems", "uniqueItems",
    // object
    "patternProperties", "additionalProperties",
    "minProperties", "maxProperties", "required",
];

/// Extract a LocatedValue (post-rewrite) into a ReducedSchema.
pub fn extract(tree: &LocatedValue) -> Result<ReducedSchema, SubtypeError> {
    match &tree.node {
        JsonF::Bool(true) => Ok(ReducedSchema::Top(tree.provenance.clone())),
        JsonF::Bool(false) => Ok(ReducedSchema::Bottom(tree.provenance.clone())),
        JsonF::Object(pairs) if pairs.is_empty() => {
            Ok(ReducedSchema::Top(tree.provenance.clone()))
        }
        JsonF::Object(pairs) => extract_object(&tree.provenance, pairs),
        _ => Err(SubtypeError::InvalidSchema {
            source: "schema must be a boolean or object".into(),
        }),
    }
}

fn get_key<'a>(pairs: &'a [(LocatedValue, LocatedValue)], key: &str) -> Option<&'a LocatedValue> {
    pairs.iter().find_map(|(k, v)| {
        if k.as_str() == Some(key) { Some(v) } else { None }
    })
}

fn extract_object(
    prov: &Provenance,
    pairs: &[(LocatedValue, LocatedValue)],
) -> Result<ReducedSchema, SubtypeError> {
    // Check for unknown keywords (fail-closed).
    for (k, _) in pairs {
        if let Some(key) = k.as_str()
            && !KNOWN_KEYWORDS.contains(&key) && !ANNOTATION_KEYWORDS.contains(&key) {
                return Err(SubtypeError::UnsupportedFeatures {
                    details: format!("unknown keyword at extraction: {key}"),
                });
            }
    }

    // Connectives first — after canonicalization, a schema should have
    // at most one connective (MultipleConnectives splits them).
    if let Some(anyof) = get_key(pairs, "anyOf") {
        let items = anyof.as_array().ok_or_else(|| SubtypeError::InvalidSchema {
            source: "anyOf must be an array".into(),
        })?;
        let schemas = items.iter().map(extract).collect::<Result<Vec<_>, _>>()?;
        return Ok(ReducedSchema::AnyOf(prov.clone(), schemas));
    }

    if let Some(allof) = get_key(pairs, "allOf") {
        let items = allof.as_array().ok_or_else(|| SubtypeError::InvalidSchema {
            source: "allOf must be an array".into(),
        })?;
        let schemas = items.iter().map(extract).collect::<Result<Vec<_>, _>>()?;
        return Ok(ReducedSchema::AllOf(prov.clone(), schemas));
    }

    if let Some(not_schema) = get_key(pairs, "not") {
        let inner = extract(not_schema)?;
        if matches!(inner, ReducedSchema::Top(_)) {
            return Ok(ReducedSchema::Bottom(prov.clone()));
        }
        if matches!(inner, ReducedSchema::Bottom(_)) {
            return Ok(ReducedSchema::Top(prov.clone()));
        }
        return Ok(ReducedSchema::Not(prov.clone(), Box::new(inner)));
    }

    // Type-directed extraction.
    let type_str = get_key(pairs, "type").and_then(|v| v.as_str());

    match type_str {
        Some("null") => Ok(ReducedSchema::Typed(Box::new(TypedSchema::Null(prov.clone())))),
        Some("boolean") => extract_boolean(prov, pairs),
        Some("string") => extract_string(prov, pairs),
        Some("number") => extract_number(prov, pairs),
        Some("array") => extract_array(prov, pairs),
        Some("object") => extract_object_schema(prov, pairs),
        Some(other) => Err(SubtypeError::InvalidSchema {
            source: format!("unknown type: {other}").into(),
        }),
        None => {
            // No type, no connective — treat as top (only annotation keywords remain).
            Ok(ReducedSchema::Top(prov.clone()))
        }
    }
}

fn extract_boolean(
    prov: &Provenance,
    pairs: &[(LocatedValue, LocatedValue)],
) -> Result<ReducedSchema, SubtypeError> {
    let enum_values = if let Some(enum_val) = get_key(pairs, "enum") {
        let items = enum_val.as_array().ok_or_else(|| SubtypeError::InvalidSchema {
            source: "enum must be an array".into(),
        })?;
        let has_true = items.iter().any(|v| v.as_bool() == Some(true));
        let has_false = items.iter().any(|v| v.as_bool() == Some(false));
        Located::new(enum_val.provenance.clone(), BoolSet::from_values(has_true, has_false))
    } else {
        Located::new(prov.clone(), BoolSet::Both)
    };
    Ok(ReducedSchema::Typed(Box::new(TypedSchema::Boolean(BooleanSchema {
        provenance: prov.clone(),
        enum_values,
    }))))
}

fn extract_string(
    prov: &Provenance,
    pairs: &[(LocatedValue, LocatedValue)],
) -> Result<ReducedSchema, SubtypeError> {
    let pattern = if let Some(p) = get_key(pairs, "pattern") {
        Located::new(p.provenance.clone(), p.as_str().unwrap_or("").to_string())
    } else {
        Located::new(prov.clone(), String::new())
    };

    // Residual length constraints (when pattern + length coexist).
    let min_length = get_key(pairs, "minLength")
        .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n as u64)));
    let max_length = get_key(pairs, "maxLength")
        .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n as u64)));

    Ok(ReducedSchema::Typed(Box::new(TypedSchema::String(StringSchema {
        provenance: prov.clone(),
        pattern,
        min_length,
        max_length,
    }))))
}

fn extract_number(
    prov: &Provenance,
    pairs: &[(LocatedValue, LocatedValue)],
) -> Result<ReducedSchema, SubtypeError> {
    let get_f64 = |key: &str, default: f64| -> Located<f64> {
        get_key(pairs, key)
            .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n)))
            .unwrap_or_else(|| Located::new(prov.clone(), default))
    };

    Ok(ReducedSchema::Typed(Box::new(TypedSchema::Number(NumberSchema {
        provenance: prov.clone(),
        minimum: get_f64("minimum", f64::NEG_INFINITY),
        maximum: get_f64("maximum", f64::INFINITY),
        exclusive_minimum: get_f64("exclusiveMinimum", f64::NEG_INFINITY),
        exclusive_maximum: get_f64("exclusiveMaximum", f64::INFINITY),
        multiple_of: get_key(pairs, "multipleOf")
            .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n))),
    }))))
}

fn extract_array(
    prov: &Provenance,
    pairs: &[(LocatedValue, LocatedValue)],
) -> Result<ReducedSchema, SubtypeError> {
    let prefix_items = match get_key(pairs, "prefixItems") {
        Some(v) => v.as_array()
            .ok_or_else(|| SubtypeError::InvalidSchema { source: "prefixItems must be array".into() })?
            .iter().map(extract).collect::<Result<Vec<_>, _>>()?,
        None => vec![],
    };

    let items = match get_key(pairs, "items") {
        Some(v) => Box::new(extract(v)?),
        None => Box::new(ReducedSchema::Top(prov.clone())),
    };

    let get_u64 = |key: &str, default: u64| -> Located<u64> {
        get_key(pairs, key)
            .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n as u64)))
            .unwrap_or_else(|| Located::new(prov.clone(), default))
    };

    let unique_items = get_key(pairs, "uniqueItems")
        .and_then(|v| v.as_bool().map(|b| Located::new(v.provenance.clone(), b)))
        .unwrap_or_else(|| Located::new(prov.clone(), false));

    Ok(ReducedSchema::Typed(Box::new(TypedSchema::Array(ArraySchema {
        provenance: prov.clone(),
        min_items: get_u64("minItems", 0),
        max_items: get_u64("maxItems", u64::MAX),
        prefix_items,
        items,
        unique_items,
    }))))
}

fn extract_object_schema(
    prov: &Provenance,
    pairs: &[(LocatedValue, LocatedValue)],
) -> Result<ReducedSchema, SubtypeError> {
    let get_u64 = |key: &str, default: u64| -> Located<u64> {
        get_key(pairs, key)
            .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n as u64)))
            .unwrap_or_else(|| Located::new(prov.clone(), default))
    };

    let required = match get_key(pairs, "required") {
        Some(v) => v.as_array()
            .ok_or_else(|| SubtypeError::InvalidSchema { source: "required must be array".into() })?
            .iter()
            .map(|item| {
                let s = item.as_str().ok_or_else(|| SubtypeError::InvalidSchema {
                    source: "required items must be strings".into(),
                })?;
                Ok(Located::new(item.provenance.clone(), s.to_string()))
            })
            .collect::<Result<Vec<_>, SubtypeError>>()?,
        None => vec![],
    };

    let pattern_properties = match get_key(pairs, "patternProperties") {
        Some(v) => {
            let obj = v.as_object().ok_or_else(|| SubtypeError::InvalidSchema {
                source: "patternProperties must be object".into(),
            })?;
            obj.iter()
                .map(|(k, v)| {
                    let pattern = k.as_str().ok_or_else(|| SubtypeError::InvalidSchema {
                        source: "pattern property key must be string".into(),
                    })?;
                    Ok((
                        Located::new(k.provenance.clone(), pattern.to_string()),
                        extract(v)?,
                    ))
                })
                .collect::<Result<Vec<_>, SubtypeError>>()?
        }
        None => vec![],
    };

    Ok(ReducedSchema::Typed(Box::new(TypedSchema::Object(ObjectSchema {
        provenance: prov.clone(),
        min_properties: get_u64("minProperties", 0),
        max_properties: get_u64("maxProperties", u64::MAX),
        required,
        pattern_properties,
    }))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn extract_top_empty_object() {
        let tree = parse("{}").unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::Top(_)));
    }

    #[test]
    fn extract_top_true() {
        let tree = parse("true").unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::Top(_)));
    }

    #[test]
    fn extract_bottom_false() {
        let tree = parse("false").unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::Bottom(_)));
    }

    #[test]
    fn extract_bottom_not_empty() {
        let tree = parse(r#"{"not": {}}"#).unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::Bottom(_)));
    }

    #[test]
    fn extract_null() {
        let tree = parse(r#"{"type": "null"}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        assert!(matches!(t.as_ref(), TypedSchema::Null(_)));
    }

    #[test]
    fn extract_boolean_no_enum() {
        let tree = parse(r#"{"type": "boolean"}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        let TypedSchema::Boolean(b) = t.as_ref() else { panic!("expected boolean") };
        assert_eq!(b.enum_values.value, BoolSet::Both);
    }

    #[test]
    fn extract_boolean_with_enum() {
        let tree = parse(r#"{"type": "boolean", "enum": [true]}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        let TypedSchema::Boolean(b) = t.as_ref() else { panic!("expected boolean") };
        assert_eq!(b.enum_values.value, BoolSet::TrueOnly);
    }

    #[test]
    fn extract_string_with_pattern() {
        let tree = parse(r#"{"type": "string", "pattern": "^.+$"}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        let TypedSchema::String(s) = t.as_ref() else { panic!("expected string") };
        assert_eq!(s.pattern.value, "^.+$");
        assert!(s.min_length.is_none());
        assert!(s.max_length.is_none());
    }

    #[test]
    fn extract_string_with_residual_length() {
        let tree = parse(r#"{"type": "string", "pattern": "[a-z]+", "minLength": 3}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        let TypedSchema::String(s) = t.as_ref() else { panic!("expected string") };
        assert_eq!(s.pattern.value, "[a-z]+");
        assert_eq!(s.min_length.as_ref().unwrap().value, 3);
    }

    #[test]
    fn extract_number() {
        let tree = parse(r#"{"type": "number", "minimum": 0, "maximum": 100}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        let TypedSchema::Number(n) = t.as_ref() else { panic!("expected number") };
        assert_eq!(n.minimum.value, 0.0);
        assert_eq!(n.maximum.value, 100.0);
    }

    #[test]
    fn extract_number_defaults() {
        let tree = parse(r#"{"type": "number"}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        let TypedSchema::Number(n) = t.as_ref() else { panic!("expected number") };
        assert_eq!(n.minimum.value, f64::NEG_INFINITY);
        assert_eq!(n.maximum.value, f64::INFINITY);
        assert!(n.multiple_of.is_none());
    }

    #[test]
    fn extract_array() {
        let tree = parse(r#"{"type": "array", "minItems": 1, "items": {"type": "string"}}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        let TypedSchema::Array(a) = t.as_ref() else { panic!("expected array") };
        assert_eq!(a.min_items.value, 1);
        assert_eq!(a.max_items.value, u64::MAX);
        let ReducedSchema::Typed(items_t) = a.items.as_ref() else { panic!("expected typed items") };
        assert!(matches!(items_t.as_ref(), TypedSchema::String(_)));
    }

    #[test]
    fn extract_object() {
        let tree = parse(r#"{"type": "object", "required": ["name"], "minProperties": 1}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        let TypedSchema::Object(o) = t.as_ref() else { panic!("expected object") };
        assert_eq!(o.min_properties.value, 1);
        assert_eq!(o.required.len(), 1);
        assert_eq!(o.required[0].value, "name");
    }

    #[test]
    fn extract_anyof() {
        let tree = parse(r#"{"anyOf": [{"type": "string"}, {"type": "null"}]}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::AnyOf(_, schemas) = &schema else { panic!("expected anyOf") };
        assert_eq!(schemas.len(), 2);
    }

    #[test]
    fn extract_allof() {
        let tree = parse(r#"{"allOf": [{"type": "string"}, {"type": "string"}]}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::AllOf(_, schemas) = &schema else { panic!("expected allOf") };
        assert_eq!(schemas.len(), 2);
    }

    #[test]
    fn extract_not() {
        let tree = parse(r#"{"not": {"type": "string"}}"#).unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::Not(_, _)));
    }

    #[test]
    fn unknown_keyword_fails_closed() {
        let tree = parse(r#"{"type": "string", "x-custom": true}"#).unwrap();
        let result = extract(&tree);
        assert!(result.is_err());
    }

    #[test]
    fn annotation_keywords_ignored() {
        let tree = parse(r#"{"type": "string", "title": "Name", "description": "A name"}"#).unwrap();
        let schema = extract(&tree).unwrap();
        let ReducedSchema::Typed(t) = &schema else { panic!("expected typed") };
        assert!(matches!(t.as_ref(), TypedSchema::String(_)));
    }
}
