use std::fmt;

use serde_json::Value;

use crate::schema::{JsonSchema, JsonValue, SchemaF, SchemaObject, TypeSet};

/// Error returned when a `serde_json::Value` is not a valid JSON Schema.
#[derive(Debug)]
pub enum ViewError {
    /// The value is not a boolean or object (e.g. a number, string, array, or null).
    InvalidSchema { found: &'static str },
    /// A keyword has an invalid type (e.g. `"type": 42`).
    InvalidKeywordType {
        keyword: &'static str,
        expected: &'static str,
    },
    /// An unrecognized type name in the `type` keyword (e.g. `"type": "foo"`).
    UnknownTypeName { name: String },
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewError::InvalidSchema { found } => {
                write!(f, "expected a boolean or object schema, found {found}")
            }
            ViewError::InvalidKeywordType { keyword, expected } => {
                write!(f, "keyword \"{keyword}\" must be {expected}")
            }
            ViewError::UnknownTypeName { name } => {
                write!(f, "unknown type name \"{name}\" in \"type\" keyword")
            }
        }
    }
}

impl std::error::Error for ViewError {}

impl JsonSchema for Value {
    type ViewError = ViewError;

    fn try_view(&self) -> Result<SchemaF<&str, &Self>, ViewError> {
        match self {
            Value::Bool(true) => Ok(SchemaF::True),
            Value::Bool(false) => Ok(SchemaF::False),
            Value::Object(map) => Ok(SchemaF::Schema(Box::new(parse_schema_object(map)?))),
            Value::Null => Err(ViewError::InvalidSchema { found: "null" }),
            Value::Number(_) => Err(ViewError::InvalidSchema { found: "number" }),
            Value::String(_) => Err(ViewError::InvalidSchema { found: "string" }),
            Value::Array(_) => Err(ViewError::InvalidSchema { found: "array" }),
        }
    }
}

fn parse_schema_object(
    map: &serde_json::Map<String, Value>,
) -> Result<SchemaObject<&str, &Value>, ViewError> {
    let mut obj = SchemaObject::default();

    // ---- type ----
    if let Some(ty) = map.get("type") {
        obj.r#type = parse_type_set(ty)?;
    }

    // ---- numeric ----
    if let Some(v) = map.get("multipleOf") {
        obj.multiple_of = Some(v.as_f64().ok_or(ViewError::InvalidKeywordType {
            keyword: "multipleOf",
            expected: "a number",
        })?);
    }
    if let Some(v) = map.get("maximum") {
        obj.maximum = Some(v.as_f64().ok_or(ViewError::InvalidKeywordType {
            keyword: "maximum",
            expected: "a number",
        })?);
    }
    if let Some(v) = map.get("exclusiveMaximum") {
        obj.exclusive_maximum = Some(v.as_f64().ok_or(ViewError::InvalidKeywordType {
            keyword: "exclusiveMaximum",
            expected: "a number",
        })?);
    }
    if let Some(v) = map.get("minimum") {
        obj.minimum = Some(v.as_f64().ok_or(ViewError::InvalidKeywordType {
            keyword: "minimum",
            expected: "a number",
        })?);
    }
    if let Some(v) = map.get("exclusiveMinimum") {
        obj.exclusive_minimum = Some(v.as_f64().ok_or(ViewError::InvalidKeywordType {
            keyword: "exclusiveMinimum",
            expected: "a number",
        })?);
    }

    // ---- string ----
    if let Some(v) = map.get("minLength") {
        obj.min_length = v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword: "minLength",
            expected: "a non-negative integer",
        })?;
    }
    if let Some(v) = map.get("maxLength") {
        obj.max_length = Some(v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword: "maxLength",
            expected: "a non-negative integer",
        })?);
    }
    if let Some(v) = map.get("pattern") {
        obj.pattern = Some(v.as_str().ok_or(ViewError::InvalidKeywordType {
            keyword: "pattern",
            expected: "a string",
        })?);
    }

    // ---- array ----
    if let Some(v) = map.get("minItems") {
        obj.min_items = v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword: "minItems",
            expected: "a non-negative integer",
        })?;
    }
    if let Some(v) = map.get("maxItems") {
        obj.max_items = Some(v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword: "maxItems",
            expected: "a non-negative integer",
        })?);
    }
    if let Some(v) = map.get("uniqueItems") {
        obj.unique_items = v.as_bool().ok_or(ViewError::InvalidKeywordType {
            keyword: "uniqueItems",
            expected: "a boolean",
        })?;
    }
    if let Some(v) = map.get("minContains") {
        obj.min_contains = Some(v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword: "minContains",
            expected: "a non-negative integer",
        })?);
    }
    if let Some(v) = map.get("maxContains") {
        obj.max_contains = Some(v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword: "maxContains",
            expected: "a non-negative integer",
        })?);
    }

    // ---- object ----
    if let Some(v) = map.get("minProperties") {
        obj.min_properties = v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword: "minProperties",
            expected: "a non-negative integer",
        })?;
    }
    if let Some(v) = map.get("maxProperties") {
        obj.max_properties = Some(v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword: "maxProperties",
            expected: "a non-negative integer",
        })?);
    }
    if let Some(v) = map.get("required") {
        let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
            keyword: "required",
            expected: "an array of strings",
        })?;
        obj.required = arr.iter().filter_map(|v| v.as_str()).collect();
    }

    // ---- const / enum ----
    if let Some(v) = map.get("const") {
        obj.r#const = Some(JsonValue::from(v));
    }
    if let Some(v) = map.get("enum") {
        let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
            keyword: "enum",
            expected: "an array",
        })?;
        obj.r#enum = Some(arr.iter().map(JsonValue::from).collect());
    }

    // ---- applicator: array ----
    if let Some(v) = map.get("prefixItems") {
        let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
            keyword: "prefixItems",
            expected: "an array of schemas",
        })?;
        obj.prefix_items = arr.iter().collect();
    }
    if let Some(v) = map.get("items") {
        obj.items = Some(v);
    }
    if let Some(v) = map.get("contains") {
        obj.contains = Some(v);
    }

    // ---- applicator: object ----
    if let Some(v) = map.get("properties") {
        let props = v.as_object().ok_or(ViewError::InvalidKeywordType {
            keyword: "properties",
            expected: "an object",
        })?;
        obj.properties = props.iter().map(|(k, v)| (k.as_str(), v)).collect();
    }
    if let Some(v) = map.get("patternProperties") {
        let props = v.as_object().ok_or(ViewError::InvalidKeywordType {
            keyword: "patternProperties",
            expected: "an object",
        })?;
        obj.pattern_properties = props.iter().map(|(k, v)| (k.as_str(), v)).collect();
    }
    if let Some(v) = map.get("additionalProperties") {
        obj.additional_properties = Some(v);
    }
    if let Some(v) = map.get("propertyNames") {
        obj.property_names = Some(v);
    }

    // ---- applicator: conditional ----
    if let Some(v) = map.get("if") {
        obj.r#if = Some(v);
    }
    if let Some(v) = map.get("then") {
        obj.then = Some(v);
    }
    if let Some(v) = map.get("else") {
        obj.r#else = Some(v);
    }

    // ---- applicator: composition ----
    if let Some(v) = map.get("allOf") {
        let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
            keyword: "allOf",
            expected: "an array of schemas",
        })?;
        obj.all_of = arr.iter().collect();
    }
    if let Some(v) = map.get("anyOf") {
        let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
            keyword: "anyOf",
            expected: "an array of schemas",
        })?;
        obj.any_of = arr.iter().collect();
    }
    if let Some(v) = map.get("oneOf") {
        let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
            keyword: "oneOf",
            expected: "an array of schemas",
        })?;
        obj.one_of = arr.iter().collect();
    }
    if let Some(v) = map.get("not") {
        obj.not = Some(v);
    }

    // ---- applicator: unevaluated ----
    if let Some(v) = map.get("unevaluatedItems") {
        obj.unevaluated_items = Some(v);
    }
    if let Some(v) = map.get("unevaluatedProperties") {
        obj.unevaluated_properties = Some(v);
    }

    // ---- applicator: dependencies ----
    if let Some(v) = map.get("dependentSchemas") {
        let deps = v.as_object().ok_or(ViewError::InvalidKeywordType {
            keyword: "dependentSchemas",
            expected: "an object",
        })?;
        obj.dependent_schemas = deps.iter().map(|(k, v)| (k.as_str(), v)).collect();
    }
    if let Some(v) = map.get("dependentRequired") {
        let deps = v.as_object().ok_or(ViewError::InvalidKeywordType {
            keyword: "dependentRequired",
            expected: "an object",
        })?;
        obj.dependent_required = deps
            .iter()
            .filter_map(|(k, v)| {
                if let Value::Array(arr) = v {
                    Some((k.as_str(), arr.iter().filter_map(|s| s.as_str()).collect()))
                } else {
                    None
                }
            })
            .collect();
    }

    Ok(obj)
}

fn parse_type_name(name: &str) -> Result<TypeSet, ViewError> {
    TypeSet::from_type_name(name).ok_or_else(|| ViewError::UnknownTypeName {
        name: name.to_owned(),
    })
}

fn parse_type_set(value: &Value) -> Result<TypeSet, ViewError> {
    match value {
        Value::String(s) => parse_type_name(s),
        Value::Array(arr) => {
            let mut set = TypeSet::empty();
            for item in arr {
                let s = item.as_str().ok_or(ViewError::InvalidKeywordType {
                    keyword: "type",
                    expected: "a string or array of strings",
                })?;
                set |= parse_type_name(s)?;
            }
            Ok(set)
        }
        _ => Err(ViewError::InvalidKeywordType {
            keyword: "type",
            expected: "a string or array of strings",
        }),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Helper: call try_view and unwrap the Schema variant's object.
    fn view_obj(v: &Value) -> SchemaObject<&str, &Value> {
        match v.try_view().unwrap() {
            SchemaF::Schema(obj) => *obj,
            other => panic!("expected SchemaF::Schema, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Boolean schemas
    // -----------------------------------------------------------------------

    #[test]
    fn bool_true_is_top() {
        assert!(matches!(json!(true).try_view().unwrap(), SchemaF::True));
    }

    #[test]
    fn bool_false_is_bottom() {
        assert!(matches!(json!(false).try_view().unwrap(), SchemaF::False));
    }

    // -----------------------------------------------------------------------
    // Invalid schemas
    // -----------------------------------------------------------------------

    #[test]
    fn null_is_invalid() {
        let err = json!(null).try_view().unwrap_err();
        assert!(matches!(err, ViewError::InvalidSchema { found: "null" }));
    }

    #[test]
    fn number_is_invalid() {
        let err = json!(42).try_view().unwrap_err();
        assert!(matches!(err, ViewError::InvalidSchema { found: "number" }));
    }

    #[test]
    fn string_is_invalid() {
        let err = json!("hello").try_view().unwrap_err();
        assert!(matches!(err, ViewError::InvalidSchema { found: "string" }));
    }

    #[test]
    fn array_is_invalid() {
        let err = json!([1, 2]).try_view().unwrap_err();
        assert!(matches!(err, ViewError::InvalidSchema { found: "array" }));
    }

    // -----------------------------------------------------------------------
    // Empty object = unconstrained
    // -----------------------------------------------------------------------

    #[test]
    fn empty_object_has_defaults() {
        let v = json!({});
        let obj = view_obj(&v);
        assert_eq!(obj.r#type, TypeSet::all());
        assert_eq!(obj.min_length, 0);
        assert_eq!(obj.min_items, 0);
        assert_eq!(obj.min_properties, 0);
        assert!(!obj.unique_items);
        assert!(obj.properties.is_empty());
        assert!(obj.required.is_empty());
        assert!(obj.all_of.is_empty());
        assert!(obj.maximum.is_none());
        assert!(obj.items.is_none());
        assert!(obj.not.is_none());
    }

    // -----------------------------------------------------------------------
    // type keyword
    // -----------------------------------------------------------------------

    #[test]
    fn type_single_string() {
        let v = json!({"type": "string"});
        let obj = view_obj(&v);
        assert_eq!(obj.r#type, TypeSet::STRING);
    }

    #[test]
    fn type_array_of_types() {
        let v = json!({"type": ["string", "null"]});
        let obj = view_obj(&v);
        assert_eq!(obj.r#type, TypeSet::STRING | TypeSet::NULL);
    }

    #[test]
    fn type_unknown_name_errors() {
        let err = json!({"type": "foo"}).try_view().unwrap_err();
        assert!(matches!(err, ViewError::UnknownTypeName { name } if name == "foo"));
    }

    #[test]
    fn type_invalid_value_errors() {
        let err = json!({"type": 42}).try_view().unwrap_err();
        assert!(matches!(
            err,
            ViewError::InvalidKeywordType {
                keyword: "type",
                ..
            }
        ));
    }

    #[test]
    fn type_array_with_non_string_errors() {
        let err = json!({"type": ["string", 42]}).try_view().unwrap_err();
        assert!(matches!(
            err,
            ViewError::InvalidKeywordType {
                keyword: "type",
                ..
            }
        ));
    }

    #[test]
    fn type_array_with_unknown_name_errors() {
        let err = json!({"type": ["string", "bogus"]}).try_view().unwrap_err();
        assert!(matches!(err, ViewError::UnknownTypeName { name } if name == "bogus"));
    }

    // -----------------------------------------------------------------------
    // Numeric keywords
    // -----------------------------------------------------------------------

    #[test]
    fn numeric_constraints() {
        let v = json!({
            "multipleOf": 5,
            "maximum": 100,
            "exclusiveMaximum": 100.5,
            "minimum": 0,
            "exclusiveMinimum": -0.5
        });
        let obj = view_obj(&v);
        assert_eq!(obj.multiple_of, Some(5.0));
        assert_eq!(obj.maximum, Some(100.0));
        assert_eq!(obj.exclusive_maximum, Some(100.5));
        assert_eq!(obj.minimum, Some(0.0));
        assert_eq!(obj.exclusive_minimum, Some(-0.5));
    }

    #[test]
    fn numeric_invalid_type_errors() {
        let err = json!({"maximum": "ten"}).try_view().unwrap_err();
        assert!(matches!(
            err,
            ViewError::InvalidKeywordType {
                keyword: "maximum",
                ..
            }
        ));
    }

    // -----------------------------------------------------------------------
    // String keywords
    // -----------------------------------------------------------------------

    #[test]
    fn string_constraints() {
        let v = json!({
            "minLength": 1,
            "maxLength": 255,
            "pattern": "^[a-z]+$"
        });
        let obj = view_obj(&v);
        assert_eq!(obj.min_length, 1);
        assert_eq!(obj.max_length, Some(255));
        assert_eq!(obj.pattern, Some("^[a-z]+$"));
    }

    // -----------------------------------------------------------------------
    // Array keywords
    // -----------------------------------------------------------------------

    #[test]
    fn array_constraints() {
        let v = json!({
            "minItems": 1,
            "maxItems": 10,
            "uniqueItems": true,
            "minContains": 2,
            "maxContains": 5
        });
        let obj = view_obj(&v);
        assert_eq!(obj.min_items, 1);
        assert_eq!(obj.max_items, Some(10));
        assert!(obj.unique_items);
        assert_eq!(obj.min_contains, Some(2));
        assert_eq!(obj.max_contains, Some(5));
    }

    #[test]
    fn prefix_items_and_items() {
        let v = json!({
            "prefixItems": [{"type": "string"}, {"type": "number"}],
            "items": {"type": "integer"}
        });
        let obj = view_obj(&v);
        assert_eq!(obj.prefix_items.len(), 2);
        assert!(obj.items.is_some());
    }

    // -----------------------------------------------------------------------
    // Object keywords
    // -----------------------------------------------------------------------

    #[test]
    fn object_constraints() {
        let v = json!({
            "minProperties": 1,
            "maxProperties": 10,
            "required": ["foo", "bar"]
        });
        let obj = view_obj(&v);
        assert_eq!(obj.min_properties, 1);
        assert_eq!(obj.max_properties, Some(10));
        assert_eq!(obj.required, vec!["foo", "bar"]);
    }

    #[test]
    fn properties_parsed() {
        let v = json!({
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });
        let obj = view_obj(&v);
        assert_eq!(obj.properties.len(), 2);
        let keys: Vec<&str> = obj.properties.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"name"));
        assert!(keys.contains(&"age"));
    }

    #[test]
    fn properties_invalid_type_errors() {
        let err = json!({"properties": [1, 2]}).try_view().unwrap_err();
        assert!(matches!(
            err,
            ViewError::InvalidKeywordType {
                keyword: "properties",
                ..
            }
        ));
    }

    // -----------------------------------------------------------------------
    // Applicator keywords
    // -----------------------------------------------------------------------

    #[test]
    fn composition_applicators() {
        let v = json!({
            "allOf": [{"type": "object"}],
            "anyOf": [{"type": "string"}, {"type": "number"}],
            "oneOf": [{"const": 1}, {"const": 2}],
            "not": {"type": "null"}
        });
        let obj = view_obj(&v);
        assert_eq!(obj.all_of.len(), 1);
        assert_eq!(obj.any_of.len(), 2);
        assert_eq!(obj.one_of.len(), 2);
        assert!(obj.not.is_some());
    }

    #[test]
    fn conditional_applicators() {
        let v = json!({
            "if": {"type": "string"},
            "then": {"minLength": 1},
            "else": {"type": "number"}
        });
        let obj = view_obj(&v);
        assert!(obj.r#if.is_some());
        assert!(obj.then.is_some());
        assert!(obj.r#else.is_some());
    }

    #[test]
    fn allof_invalid_type_errors() {
        let err = json!({"allOf": "not-an-array"}).try_view().unwrap_err();
        assert!(matches!(
            err,
            ViewError::InvalidKeywordType {
                keyword: "allOf",
                ..
            }
        ));
    }

    // -----------------------------------------------------------------------
    // const / enum
    // -----------------------------------------------------------------------

    #[test]
    fn const_keyword() {
        let v = json!({"const": "hello"});
        let obj = view_obj(&v);
        assert_eq!(obj.r#const, Some(JsonValue::String("hello".into())));
    }

    #[test]
    fn enum_keyword() {
        let v = json!({"enum": [1, "two", null]});
        let obj = view_obj(&v);
        let values = obj.r#enum.unwrap();
        assert_eq!(values.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Recursive children are references into the original Value
    // -----------------------------------------------------------------------

    #[test]
    fn children_borrow_from_original() {
        let v = json!({"properties": {"x": {"type": "string"}}});
        let obj = view_obj(&v);
        let (key, child) = &obj.properties[0];
        assert_eq!(*key, "x");
        // The child is a reference into the original serde_json::Value tree.
        let child_obj = match child.try_view().unwrap() {
            SchemaF::Schema(obj) => *obj,
            other => panic!("expected Schema, got {other:?}"),
        };
        assert_eq!(child_obj.r#type, TypeSet::STRING);
    }
}
