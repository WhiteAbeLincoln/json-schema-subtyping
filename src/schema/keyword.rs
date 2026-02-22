/// Classification of a schema as top (accepts all), bottom (rejects all), or constrained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    Top,
    Bottom,
    Constrained,
}

/// A JSON Schema keyword. Defines the extracted value type and subtype comparison.
///
/// The GAT `Extracted<'a, Schema: 'a>` serves double duty:
/// - Validation keywords: ignore the Schema param (e.g., `Option<f64>`)
/// - Applicator keywords: reference Schema for recursive children (e.g., `Option<&'a Schema>`)
pub trait Keyword: 'static {
    type Extracted<'a, Schema: 'a>;

    /// Is sub's constraint at least as restrictive as sup's?
    /// `is_subtype` callback handles recursive sub-schema comparison.
    fn check_subtype<Sub, Sup>(
        sub: &Self::Extracted<'_, Sub>,
        sup: &Self::Extracted<'_, Sup>,
        is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool;
}

/// Base trait for a queryable schema. Provides the error type shared by all
/// keyword queries, and top/bottom/constrained classification.
pub trait QuerySchema: Sized {
    type Error;

    /// Classify this schema as top (accepts all), bottom (rejects all),
    /// or constrained (has keywords to compare).
    fn kind(&self) -> Result<SchemaKind, Self::Error>;
}

/// A schema that can answer queries about keyword K.
pub trait Get<K: Keyword>: QuerySchema {
    fn get(&self) -> Result<K::Extracted<'_, Self>, Self::Error>;
}
