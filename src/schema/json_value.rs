/// A self-contained JSON value type for `const` and `enum` keywords.
///
/// This avoids a hard dependency on `serde_json` in the core types.
#[derive(Clone, Debug)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    /// Key-value pairs rather than a map because this type only needs equality
    /// checks (for `const`/`enum` subtyping), never key lookup. JSON object
    /// equality is order-independent, so `PartialEq` does an unordered comparison
    /// regardless of container type. A `Vec` is simpler and cache-friendly for the
    /// small objects that `const` values typically are.
    Object(Vec<(String, JsonValue)>),
}

impl PartialEq for JsonValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(k, v)| b.iter().any(|(k2, v2)| k == k2 && v == v2))
                    && b.iter().all(|(k, _)| a.iter().any(|(k2, _)| k == k2))
            }
            _ => false,
        }
    }
}

#[cfg(feature = "serde")]
impl From<&serde_json::Value> for JsonValue {
    fn from(v: &serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => JsonValue::Null,
            serde_json::Value::Bool(b) => JsonValue::Bool(*b),
            serde_json::Value::Number(n) => JsonValue::Number(n.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::String(s) => JsonValue::String(s.clone()),
            serde_json::Value::Array(arr) => {
                JsonValue::Array(arr.iter().map(JsonValue::from).collect())
            }
            serde_json::Value::Object(obj) => JsonValue::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), JsonValue::from(v)))
                    .collect(),
            ),
        }
    }
}
