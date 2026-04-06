use crate::located::{Located, Provenance};
use super::boolset::BoolSet;

/// A fully reduced, typed schema ready for subtype checking.
#[derive(Debug)]
pub enum ReducedSchema {
    /// Top type — accepts everything ({} or true).
    Top(Provenance),
    /// Bottom type — accepts nothing ({not: {}} or false).
    Bottom(Provenance),
    /// A type-homogeneous schema.
    Typed(TypedSchema),
    /// Union of schemas. After simplification, elements are non-overlapping
    /// for primitives (may still overlap for arrays/objects).
    AnyOf(Provenance, Vec<ReducedSchema>),
    /// Residual conjunction with negation — couldn't be simplified away.
    /// Used for number/array/object negation patterns.
    AllOf(Provenance, Vec<ReducedSchema>),
    /// Negation that couldn't be eliminated (number, array, object).
    Not(Provenance, Box<ReducedSchema>),
}

/// Type-homogeneous schema — exactly one JSON type.
#[derive(Debug)]
pub enum TypedSchema {
    Null(Provenance),
    Boolean(BooleanSchema),
    String(StringSchema),
    Number(NumberSchema),
    Array(ArraySchema),
    Object(ObjectSchema),
}

/// After canonicalization, boolean schemas only have enum.
#[derive(Debug)]
pub struct BooleanSchema {
    pub provenance: Provenance,
    pub enum_values: Located<BoolSet>,
}

/// After canonicalization, string schemas only have pattern
/// (minLength/maxLength compiled into the regex).
#[derive(Debug)]
pub struct StringSchema {
    pub provenance: Provenance,
    /// The regex pattern. After canonicalization this encodes all string constraints.
    pub pattern: Located<String>,
}

/// Number schemas retain all numeric keywords.
#[derive(Debug)]
pub struct NumberSchema {
    pub provenance: Provenance,
    pub minimum: Located<f64>,
    pub maximum: Located<f64>,
    pub exclusive_minimum: Located<f64>,
    pub exclusive_maximum: Located<f64>,
    pub multiple_of: Option<Located<f64>>,
}

/// Array schemas — prefixItems/items model from 2020-12.
#[derive(Debug)]
pub struct ArraySchema {
    pub provenance: Provenance,
    pub min_items: Located<u64>,
    pub max_items: Located<u64>,
    pub prefix_items: Vec<ReducedSchema>,
    pub items: Box<ReducedSchema>,
    pub unique_items: Located<bool>,
}

/// Object schemas — after canonicalization, properties+additionalProperties
/// are merged into patternProperties with non-overlapping regexes.
#[derive(Debug)]
pub struct ObjectSchema {
    pub provenance: Provenance,
    pub min_properties: Located<u64>,
    pub max_properties: Located<u64>,
    pub required: Vec<Located<String>>,
    pub pattern_properties: Vec<(Located<String>, ReducedSchema)>,
}

impl ReducedSchema {
    pub fn provenance(&self) -> &Provenance {
        match self {
            ReducedSchema::Top(p)
            | ReducedSchema::Bottom(p)
            | ReducedSchema::AnyOf(p, _)
            | ReducedSchema::AllOf(p, _)
            | ReducedSchema::Not(p, _) => p,
            ReducedSchema::Typed(t) => t.provenance(),
        }
    }
}

impl TypedSchema {
    pub fn provenance(&self) -> &Provenance {
        match self {
            TypedSchema::Null(p) => p,
            TypedSchema::Boolean(s) => &s.provenance,
            TypedSchema::String(s) => &s.provenance,
            TypedSchema::Number(s) => &s.provenance,
            TypedSchema::Array(s) => &s.provenance,
            TypedSchema::Object(s) => &s.provenance,
        }
    }
}
