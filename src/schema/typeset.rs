bitflags::bitflags! {
    /// The set of JSON types a schema permits.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct TypeSet: u8 {
        const NULL    = 1 << 0;
        const BOOLEAN = 1 << 1;
        const OBJECT  = 1 << 2;
        const ARRAY   = 1 << 3;
        const NUMBER  = 1 << 4;
        const STRING  = 1 << 5;
        const INTEGER = 1 << 6;
    }
}

impl TypeSet {
    /// Parse a JSON Schema type name string into a single-type TypeSet.
    pub fn from_type_name(name: &str) -> Option<Self> {
        match name {
            "null" => Some(Self::NULL),
            "boolean" => Some(Self::BOOLEAN),
            "object" => Some(Self::OBJECT),
            "array" => Some(Self::ARRAY),
            "number" => Some(Self::NUMBER),
            "string" => Some(Self::STRING),
            "integer" => Some(Self::INTEGER),
            _ => None,
        }
    }

    /// All JSON types including integer.
    pub fn all_types() -> Self {
        Self::all()
    }

    /// Is this a subset of the other type set?
    pub fn is_subset_of(self, other: Self) -> bool {
        self & other == self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name() {
        assert_eq!(TypeSet::from_type_name("string"), Some(TypeSet::STRING));
        assert_eq!(TypeSet::from_type_name("unknown"), None);
    }

    #[test]
    fn subset() {
        let nums = TypeSet::NUMBER | TypeSet::INTEGER;
        assert!(TypeSet::NUMBER.is_subset_of(nums));
        assert!(!TypeSet::STRING.is_subset_of(nums));
    }
}
