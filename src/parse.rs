use crate::error::SubtypeError;
use crate::located::{JsonF, JsonPointer, LocatedValue, Provenance, Span};
use jsonc_parser::ast::Value;
use jsonc_parser::common::Ranged;

/// Parse a JSON string into a LocatedValue tree.
/// Every node carries its source span and JSON pointer path.
pub fn parse(source: &str) -> Result<LocatedValue, SubtypeError> {
    let parsed = jsonc_parser::parse_to_ast(
        source,
        &jsonc_parser::CollectOptions::default(),
        &jsonc_parser::ParseOptions::default(),
    )
    .map_err(|e| SubtypeError::InvalidSchema {
        source: Box::new(e),
    })?;

    match &parsed.value {
        Some(value) => convert_value(value, JsonPointer::root()),
        None => Err(SubtypeError::InvalidSchema {
            source: "Empty input".into(),
        }),
    }
}

fn convert_value(value: &Value<'_>, pointer: JsonPointer) -> Result<LocatedValue, SubtypeError> {
    let range = value.range();
    let span = Span::new(range.start, range.end);
    let prov = Provenance::new(span, pointer.clone());

    let node = match value {
        Value::NullKeyword(_) => JsonF::Null,
        Value::BooleanLit(b) => JsonF::Bool(b.value),
        Value::NumberLit(n) => {
            let num: f64 = n.value.parse().map_err(|e: std::num::ParseFloatError| {
                SubtypeError::InvalidSchema {
                    source: Box::new(e),
                }
            })?;
            JsonF::Number(num)
        }
        Value::StringLit(s) => JsonF::String(s.value.to_string()),
        Value::Array(arr) => {
            let items = arr
                .elements
                .iter()
                .enumerate()
                .map(|(i, elem)| convert_value(elem, pointer.push(i.to_string())))
                .collect::<Result<Vec<_>, _>>()?;
            JsonF::Array(items)
        }
        Value::Object(obj) => {
            let pairs = obj
                .properties
                .iter()
                .map(|prop| {
                    let key_name = prop.name.as_str();
                    let key_range = prop.name.range();
                    let key_span = Span::new(key_range.start, key_range.end);
                    let child_pointer = pointer.push(key_name);
                    let key = LocatedValue::new(
                        Provenance::new(key_span, child_pointer.clone()),
                        JsonF::String(key_name.to_string()),
                    );
                    let val = convert_value(&prop.value, child_pointer)?;
                    Ok((key, val))
                })
                .collect::<Result<Vec<_>, SubtypeError>>()?;
            JsonF::Object(pairs)
        }
    };

    Ok(LocatedValue::new(prov, node))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_number() {
        let v = parse("42").unwrap();
        assert_eq!(v.as_number(), Some(42.0));
        assert_eq!(v.provenance.spans[0], Span::new(0, 2));
    }

    #[test]
    fn parse_string() {
        let v = parse(r#""hello""#).unwrap();
        assert_eq!(v.as_str(), Some("hello"));
    }

    #[test]
    fn parse_object_with_pointers() {
        let v = parse(r#"{"a": 1, "b": "two"}"#).unwrap();
        let a_val = v.get_key("a").unwrap();
        assert_eq!(a_val.as_number(), Some(1.0));
        assert_eq!(a_val.provenance.pointers[0], JsonPointer::root().push("a"));
    }

    #[test]
    fn parse_nested_object() {
        let v = parse(r#"{"outer": {"inner": true}}"#).unwrap();
        let inner = v.get_key("outer").unwrap().get_key("inner").unwrap();
        assert_eq!(inner.as_bool(), Some(true));
        assert_eq!(
            inner.provenance.pointers[0],
            JsonPointer::root().push("outer").push("inner")
        );
    }

    #[test]
    fn parse_array() {
        let v = parse("[1, 2, 3]").unwrap();
        let items = v.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(
            items[1].provenance.pointers[0],
            JsonPointer::root().push("1")
        );
    }

    #[test]
    fn parse_true_false_null() {
        assert!(parse("true").unwrap().as_bool() == Some(true));
        assert!(parse("false").unwrap().as_bool() == Some(false));
        assert!(parse("null").unwrap().is_null());
    }

    #[test]
    fn parse_invalid_json() {
        assert!(parse("{invalid}").is_err());
    }
}
