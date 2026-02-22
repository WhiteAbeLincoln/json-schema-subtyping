use std::fmt;

use serde_json::Value;

use crate::schema::keyword::{Get, QuerySchema, SchemaKind};
use crate::schema::keywords::*;
use crate::schema::{JsonValue, TypeSet};

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

// ---------------------------------------------------------------------------
// QuerySchema for serde_json::Value
// ---------------------------------------------------------------------------

impl QuerySchema for Value {
    type Error = ViewError;

    fn kind(&self) -> Result<SchemaKind, ViewError> {
        match self {
            Value::Bool(true) => Ok(SchemaKind::Top),
            Value::Bool(false) => Ok(SchemaKind::Bottom),
            Value::Object(map) => {
                // Empty object = top (no constraints)
                if map.is_empty() {
                    return Ok(SchemaKind::Top);
                }

                // Check if effectively top (only unconstrained keywords)
                if is_effectively_top(map) {
                    return Ok(SchemaKind::Top);
                }

                // {not: X} where X is top → bottom, X is bottom → top
                if let Some(not_val) = map.get("not")
                    && map.len() == 1
                {
                    match not_val.kind()? {
                        SchemaKind::Top => return Ok(SchemaKind::Bottom),
                        SchemaKind::Bottom => return Ok(SchemaKind::Top),
                        SchemaKind::Constrained => {}
                    }
                }

                // Unsatisfiable min/max pairs → bottom
                if is_unsatisfiable(map) {
                    return Ok(SchemaKind::Bottom);
                }

                Ok(SchemaKind::Constrained)
            }
            Value::Null => Err(ViewError::InvalidSchema { found: "null" }),
            Value::Number(_) => Err(ViewError::InvalidSchema { found: "number" }),
            Value::String(_) => Err(ViewError::InvalidSchema { found: "string" }),
            Value::Array(_) => Err(ViewError::InvalidSchema { found: "array" }),
        }
    }
}

/// Check if a schema object is effectively top (all keywords at their
/// unconstrained defaults). Currently checks for {type: [all types]}.
fn is_effectively_top(map: &serde_json::Map<String, Value>) -> bool {
    map.iter().all(|(key, val)| match key.as_str() {
        "type" => parse_type_set(val).is_ok_and(|ts| ts == TypeSet::all()),
        _ => false,
    })
}

fn is_unsatisfiable(map: &serde_json::Map<String, Value>) -> bool {
    exceeds_u64(map, "minLength", "maxLength")
        || exceeds_u64(map, "minItems", "maxItems")
        || exceeds_u64(map, "minProperties", "maxProperties")
        || exceeds_u64(map, "minContains", "maxContains")
        || exceeds_f64(map, "minimum", "maximum")
}

fn exceeds_u64(map: &serde_json::Map<String, Value>, min_key: &str, max_key: &str) -> bool {
    let Some(min_val) = map.get(min_key).and_then(|v| v.as_u64()) else {
        return false;
    };
    let Some(max_val) = map.get(max_key).and_then(|v| v.as_u64()) else {
        return false;
    };
    min_val > max_val
}

fn exceeds_f64(map: &serde_json::Map<String, Value>, min_key: &str, max_key: &str) -> bool {
    let Some(min_val) = map.get(min_key).and_then(|v| v.as_f64()) else {
        return false;
    };
    let Some(max_val) = map.get(max_key).and_then(|v| v.as_f64()) else {
        return false;
    };
    min_val > max_val
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn get_opt_f64(
    map: &serde_json::Map<String, Value>,
    keyword: &'static str,
) -> Result<Option<f64>, ViewError> {
    match map.get(keyword) {
        None => Ok(None),
        Some(v) => Ok(Some(v.as_f64().ok_or(ViewError::InvalidKeywordType {
            keyword,
            expected: "a number",
        })?)),
    }
}

fn get_opt_u64(
    map: &serde_json::Map<String, Value>,
    keyword: &'static str,
) -> Result<Option<u64>, ViewError> {
    match map.get(keyword) {
        None => Ok(None),
        Some(v) => Ok(Some(v.as_u64().ok_or(ViewError::InvalidKeywordType {
            keyword,
            expected: "a non-negative integer",
        })?)),
    }
}

/// Check if the schema restricts to integer values (type: "integer" or multipleOf: 1).
fn is_integer_domain(map: &serde_json::Map<String, Value>) -> bool {
    if let Some(ty) = map.get("type")
        && ty.as_str() == Some("integer")
    {
        return true;
    }
    if let Some(m) = map.get("multipleOf")
        && m.as_f64() == Some(1.0)
    {
        return true;
    }
    false
}

/// Compute effective upper bound from maximum and exclusiveMaximum,
/// with integer normalization when applicable.
fn effective_upper_bound(
    maximum: Option<f64>,
    exclusive_maximum: Option<f64>,
    integer: bool,
) -> Bound {
    let max_bound = maximum.map(Bound::Inclusive);
    let excl_bound = exclusive_maximum.map(|e| {
        if integer {
            // For integers, x < N ≡ x ≤ N-1
            Bound::Inclusive(e - 1.0)
        } else {
            Bound::Exclusive(e)
        }
    });

    match (max_bound, excl_bound) {
        (None, None) => Bound::Unbounded,
        (Some(b), None) | (None, Some(b)) => b,
        (Some(Bound::Inclusive(m)), Some(Bound::Inclusive(e))) => Bound::Inclusive(m.min(e)),
        (Some(Bound::Inclusive(m)), Some(Bound::Exclusive(e))) => {
            if m < e {
                Bound::Inclusive(m)
            } else {
                Bound::Exclusive(e)
            }
        }
        _ => unreachable!(),
    }
}

/// Compute effective lower bound from minimum and exclusiveMinimum,
/// with integer normalization when applicable.
fn effective_lower_bound(
    minimum: Option<f64>,
    exclusive_minimum: Option<f64>,
    integer: bool,
) -> Bound {
    let min_bound = minimum.map(Bound::Inclusive);
    let excl_bound = exclusive_minimum.map(|e| {
        if integer {
            // For integers, x > N ≡ x ≥ N+1
            Bound::Inclusive(e + 1.0)
        } else {
            Bound::Exclusive(e)
        }
    });

    match (min_bound, excl_bound) {
        (None, None) => Bound::Unbounded,
        (Some(b), None) | (None, Some(b)) => b,
        (Some(Bound::Inclusive(m)), Some(Bound::Inclusive(e))) => Bound::Inclusive(m.max(e)),
        (Some(Bound::Inclusive(m)), Some(Bound::Exclusive(e))) => {
            if m > e {
                Bound::Inclusive(m)
            } else {
                Bound::Exclusive(e)
            }
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Get<K> implementations for serde_json::Value
// ---------------------------------------------------------------------------

impl Get<TypeKw> for Value {
    fn get(&self) -> Result<Option<TypeSet>, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(None);
        };
        let mut ts = match obj.get("type") {
            None => {
                // multipleOf: 1 implies integer domain for subtyping purposes
                if is_integer_domain(obj) {
                    return Ok(Some(TypeSet::INTEGER));
                }
                return Ok(None);
            }
            Some(v) => parse_type_set(v)?,
        };
        // multipleOf: 1 with type: "number" → effectively integer
        if ts.contains(TypeSet::NUMBER) && is_integer_domain(obj) {
            ts = (ts - TypeSet::NUMBER) | TypeSet::INTEGER;
        }
        if ts == TypeSet::all() {
            Ok(None)
        } else {
            Ok(Some(ts))
        }
    }
}

// --- Combined numeric bounds ---

impl Get<UpperBoundKw> for Value {
    fn get(&self) -> Result<Bound, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(Bound::Unbounded);
        };
        let maximum = get_opt_f64(obj, "maximum")?;
        let exclusive_maximum = get_opt_f64(obj, "exclusiveMaximum")?;
        let integer = is_integer_domain(obj);
        Ok(effective_upper_bound(maximum, exclusive_maximum, integer))
    }
}

impl Get<LowerBoundKw> for Value {
    fn get(&self) -> Result<Bound, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(Bound::Unbounded);
        };
        let minimum = get_opt_f64(obj, "minimum")?;
        let exclusive_minimum = get_opt_f64(obj, "exclusiveMinimum")?;
        let integer = is_integer_domain(obj);
        Ok(effective_lower_bound(minimum, exclusive_minimum, integer))
    }
}

// --- MultipleOf ---

impl Get<MultipleOfKw> for Value {
    fn get(&self) -> Result<Option<f64>, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(None);
        };
        let m = get_opt_f64(obj, "multipleOf")?;
        if m.is_some() {
            return Ok(m);
        }
        // type: "integer" implies multipleOf: 1
        if let Some(ty) = obj.get("type")
            && ty.as_str() == Some("integer")
        {
            return Ok(Some(1.0));
        }
        Ok(None)
    }
}

// --- Integer upper/lower bounds ---

macro_rules! impl_get_opt_u64 {
    ($kw:ty, $json_key:expr) => {
        impl Get<$kw> for Value {
            fn get(&self) -> Result<Option<u64>, ViewError> {
                let Some(obj) = self.as_object() else {
                    return Ok(None);
                };
                get_opt_u64(obj, $json_key)
            }
        }
    };
}

impl_get_opt_u64!(MaxLengthKw, "maxLength");
impl_get_opt_u64!(MinLengthKw, "minLength");
impl_get_opt_u64!(MaxItemsKw, "maxItems");
impl_get_opt_u64!(MinItemsKw, "minItems");
impl_get_opt_u64!(MaxContainsKw, "maxContains");
impl_get_opt_u64!(MinContainsKw, "minContains");
impl_get_opt_u64!(MaxPropertiesKw, "maxProperties");
impl_get_opt_u64!(MinPropertiesKw, "minProperties");

// --- UniqueItems ---

impl Get<UniqueItemsKw> for Value {
    fn get(&self) -> Result<bool, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(false);
        };
        match obj.get("uniqueItems") {
            None => Ok(false),
            Some(v) => Ok(v.as_bool().ok_or(ViewError::InvalidKeywordType {
                keyword: "uniqueItems",
                expected: "a boolean",
            })?),
        }
    }
}

// --- Required ---

impl Get<RequiredKw> for Value {
    fn get(&self) -> Result<Vec<&str>, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(Vec::new());
        };
        match obj.get("required") {
            None => Ok(Vec::new()),
            Some(v) => {
                let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
                    keyword: "required",
                    expected: "an array of strings",
                })?;
                Ok(arr.iter().filter_map(|v| v.as_str()).collect())
            }
        }
    }
}

// --- Const ---

impl Get<ConstKw> for Value {
    fn get(&self) -> Result<Option<JsonValue>, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(None);
        };
        Ok(obj.get("const").map(JsonValue::from))
    }
}

// --- Enum ---

impl Get<EnumKw> for Value {
    fn get(&self) -> Result<Option<Vec<JsonValue>>, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(None);
        };
        match obj.get("enum") {
            None => Ok(None),
            Some(v) => {
                let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
                    keyword: "enum",
                    expected: "an array",
                })?;
                Ok(Some(arr.iter().map(JsonValue::from).collect()))
            }
        }
    }
}

// --- Optional applicators (Option<&Self>) ---

macro_rules! impl_get_optional_applicator {
    ($kw:ty, $json_key:expr) => {
        impl Get<$kw> for Value {
            fn get(&self) -> Result<Option<&Self>, ViewError> {
                let Some(obj) = self.as_object() else {
                    return Ok(None);
                };
                Ok(obj.get($json_key))
            }
        }
    };
}

impl_get_optional_applicator!(ItemsKw, "items");
impl_get_optional_applicator!(ContainsKw, "contains");
impl_get_optional_applicator!(AdditionalPropertiesKw, "additionalProperties");
impl_get_optional_applicator!(PropertyNamesKw, "propertyNames");

// --- Properties (key-value applicators with top-schema detection) ---

macro_rules! impl_get_kv_applicator {
    ($kw:ty, $json_key:expr) => {
        impl Get<$kw> for Value {
            fn get(&self) -> Result<Vec<(&str, Option<&Self>)>, ViewError> {
                let Some(obj) = self.as_object() else {
                    return Ok(Vec::new());
                };
                match obj.get($json_key) {
                    None => Ok(Vec::new()),
                    Some(v) => {
                        let props = v.as_object().ok_or(ViewError::InvalidKeywordType {
                            keyword: $json_key,
                            expected: "an object",
                        })?;
                        Ok(props
                            .iter()
                            .map(|(k, v)| {
                                // Mark top schemas as None so keyword comparison
                                // can distinguish "present but unconstrained" from "constrained"
                                let child = match v.kind() {
                                    Ok(SchemaKind::Top) => None,
                                    _ => Some(v),
                                };
                                (k.as_str(), child)
                            })
                            .collect())
                    }
                }
            }
        }
    };
}

impl_get_kv_applicator!(PropertiesKw, "properties");
impl_get_kv_applicator!(PatternPropertiesKw, "patternProperties");

// --- PrefixItems ---

impl Get<PrefixItemsKw> for Value {
    fn get(&self) -> Result<Vec<&Self>, ViewError> {
        let Some(obj) = self.as_object() else {
            return Ok(Vec::new());
        };
        match obj.get("prefixItems") {
            None => Ok(Vec::new()),
            Some(v) => {
                let arr = v.as_array().ok_or(ViewError::InvalidKeywordType {
                    keyword: "prefixItems",
                    expected: "an array of schemas",
                })?;
                Ok(arr.iter().collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn bool_true_is_top() {
        assert_eq!(json!(true).kind().unwrap(), SchemaKind::Top);
    }

    #[test]
    fn bool_false_is_bottom() {
        assert_eq!(json!(false).kind().unwrap(), SchemaKind::Bottom);
    }

    #[test]
    fn empty_object_is_top() {
        assert_eq!(json!({}).kind().unwrap(), SchemaKind::Top);
    }

    #[test]
    fn all_types_is_top() {
        let v = json!({"type": ["null", "boolean", "object", "array", "number", "string", "integer"]});
        assert_eq!(v.kind().unwrap(), SchemaKind::Top);
    }

    #[test]
    fn not_false_is_top() {
        assert_eq!(json!({"not": false}).kind().unwrap(), SchemaKind::Top);
    }

    #[test]
    fn not_true_is_bottom() {
        assert_eq!(json!({"not": true}).kind().unwrap(), SchemaKind::Bottom);
    }

    #[test]
    fn not_empty_object_is_bottom() {
        assert_eq!(json!({"not": {}}).kind().unwrap(), SchemaKind::Bottom);
    }

    #[test]
    fn not_all_types_is_bottom() {
        let v = json!({"not": {"type": ["null", "boolean", "object", "array", "number", "string", "integer"]}});
        assert_eq!(v.kind().unwrap(), SchemaKind::Bottom);
    }

    #[test]
    fn not_not_true_is_top() {
        assert_eq!(
            json!({"not": {"not": true}}).kind().unwrap(),
            SchemaKind::Top
        );
    }

    #[test]
    fn null_is_invalid() {
        let err = json!(null).kind().unwrap_err();
        assert!(matches!(err, ViewError::InvalidSchema { found: "null" }));
    }

    #[test]
    fn min_gt_max_is_bottom() {
        assert_eq!(
            json!({"minimum": 10, "maximum": 5}).kind().unwrap(),
            SchemaKind::Bottom
        );
    }

    #[test]
    fn upper_bound_combined() {
        let v = json!({"maximum": 10, "exclusiveMaximum": 8});
        let b: Bound = <Value as Get<UpperBoundKw>>::get(&v).unwrap();
        assert_eq!(b, Bound::Exclusive(8.0));
    }

    #[test]
    fn upper_bound_integer_normalization() {
        let v = json!({"exclusiveMaximum": 11, "type": "integer"});
        let b: Bound = <Value as Get<UpperBoundKw>>::get(&v).unwrap();
        assert_eq!(b, Bound::Inclusive(10.0));
    }

    #[test]
    fn lower_bound_integer_normalization() {
        let v = json!({"exclusiveMinimum": 9, "type": "integer"});
        let b: Bound = <Value as Get<LowerBoundKw>>::get(&v).unwrap();
        assert_eq!(b, Bound::Inclusive(10.0));
    }

    #[test]
    fn properties_top_detection() {
        let v = json!({"properties": {"a": {"type": "string"}, "b": {}}});
        let props: Vec<(&str, Option<&Value>)> = <Value as Get<PropertiesKw>>::get(&v).unwrap();
        let a = props.iter().find(|(k, _)| *k == "a").unwrap();
        let b = props.iter().find(|(k, _)| *k == "b").unwrap();
        assert!(a.1.is_some()); // "a" is constrained
        assert!(b.1.is_none()); // "b" is top (unconstrained)
    }
}
