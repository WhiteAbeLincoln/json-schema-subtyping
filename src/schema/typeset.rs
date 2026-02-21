bitflags::bitflags! {
    /// A compact bitset representing a set of JSON Schema types.
    ///
    /// JSON Schema defines seven type values: null, boolean, object, array,
    /// number, string, integer.
    ///
    /// Use [`TypeSet::all()`] to represent "absent" (unconstrained).
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
    /// Parse a single JSON Schema type name into a one-element `TypeSet`.
    ///
    /// Returns `None` if the name is not a recognized type.
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
}
