use super::span::Provenance;

/// One layer of JSON, parameterized over the recursive child type.
/// This is the base functor for our recursion scheme.
#[derive(Debug, Clone)]
pub enum JsonF<A> {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<A>),
    /// Key-value pairs. Keys are always string-valued LocatedValues
    /// when A = LocatedValue, carrying their own provenance.
    Object(Vec<(A, A)>),
}

impl<A> JsonF<A> {
    /// Functor map — transform all children.
    pub fn map<B>(self, mut f: impl FnMut(A) -> B) -> JsonF<B> {
        match self {
            JsonF::Null => JsonF::Null,
            JsonF::Bool(b) => JsonF::Bool(b),
            JsonF::Number(n) => JsonF::Number(n),
            JsonF::String(s) => JsonF::String(s),
            JsonF::Array(items) => JsonF::Array(items.into_iter().map(&mut f).collect()),
            JsonF::Object(pairs) => {
                JsonF::Object(pairs.into_iter().map(|(k, v)| (f(k), f(v))).collect())
            }
        }
    }

    /// Fallible functor map — transform all children, short-circuiting on error.
    pub fn try_map<B, E>(self, mut f: impl FnMut(A) -> Result<B, E>) -> Result<JsonF<B>, E> {
        Ok(match self {
            JsonF::Null => JsonF::Null,
            JsonF::Bool(b) => JsonF::Bool(b),
            JsonF::Number(n) => JsonF::Number(n),
            JsonF::String(s) => JsonF::String(s),
            JsonF::Array(items) => {
                JsonF::Array(items.into_iter().map(&mut f).collect::<Result<_, _>>()?)
            }
            JsonF::Object(pairs) => JsonF::Object(
                pairs
                    .into_iter()
                    .map(|(k, v)| Ok((f(k)?, f(v)?)))
                    .collect::<Result<_, _>>()?,
            ),
        })
    }
}

/// The recursive located JSON tree.
/// Equivalent to Cofree JsonF Provenance — each node carries provenance
/// (source spans + JSON pointers) plus one layer of JSON structure.
#[derive(Debug, Clone)]
pub struct LocatedValue {
    pub provenance: Provenance,
    pub node: JsonF<LocatedValue>,
}

impl LocatedValue {
    pub fn new(provenance: Provenance, node: JsonF<LocatedValue>) -> Self {
        Self { provenance, node }
    }

    /// Create a synthetic (no source location) LocatedValue.
    pub fn synthetic(node: JsonF<LocatedValue>) -> Self {
        Self {
            provenance: Provenance::synthetic(),
            node,
        }
    }

    pub fn as_object(&self) -> Option<&[(LocatedValue, LocatedValue)]> {
        match &self.node {
            JsonF::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.node {
            JsonF::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match &self.node {
            JsonF::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.node {
            JsonF::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[LocatedValue]> {
        match &self.node {
            JsonF::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(&self.node, JsonF::Null)
    }

    /// Look up a key in an object node.
    pub fn get_key(&self, key: &str) -> Option<&LocatedValue> {
        self.as_object()?.iter().find_map(|(k, v)| {
            if k.as_str() == Some(key) {
                Some(v)
            } else {
                None
            }
        })
    }

    /// Structural equality ignoring provenance. Useful for tests.
    pub fn structural_eq(&self, other: &LocatedValue) -> bool {
        match (&self.node, &other.node) {
            (JsonF::Null, JsonF::Null) => true,
            (JsonF::Bool(a), JsonF::Bool(b)) => a == b,
            (JsonF::Number(a), JsonF::Number(b)) => a == b,
            (JsonF::String(a), JsonF::String(b)) => a == b,
            (JsonF::Array(a), JsonF::Array(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|(x, y)| x.structural_eq(y))
            }
            (JsonF::Object(a), JsonF::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|((k1, v1), (k2, v2))| k1.structural_eq(k2) && v1.structural_eq(v2))
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::Span;

    #[test]
    fn jsonf_map() {
        let node: JsonF<i32> = JsonF::Array(vec![1, 2, 3]);
        let doubled = node.map(|x| x * 2);
        match doubled {
            JsonF::Array(v) => assert_eq!(v, vec![2, 4, 6]),
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn located_value_get_key() {
        let key = LocatedValue::synthetic(JsonF::String("name".into()));
        let val = LocatedValue::synthetic(JsonF::String("test".into()));
        let obj = LocatedValue::synthetic(JsonF::Object(vec![(key, val)]));
        assert_eq!(obj.get_key("name").unwrap().as_str(), Some("test"));
        assert!(obj.get_key("missing").is_none());
    }

    #[test]
    fn structural_eq_ignores_provenance() {
        use crate::located::JsonPointer;
        let a = LocatedValue::new(
            Provenance::new(Span::new(0, 5), JsonPointer::root()),
            JsonF::Number(42.0),
        );
        let b = LocatedValue::synthetic(JsonF::Number(42.0));
        assert!(a.structural_eq(&b));
    }
}
