use std::convert::Infallible;

use crate::output::Location;

use super::node::SchemaF;

/// The coalgebra for JSON Schema: unfold one layer of structure.
///
/// Implementors provide a borrowed view of a single layer of the schema tree.
/// String data is borrowed as `&str`, recursive children as `&Self`.
///
/// The subtyping algorithm calls `try_view()` recursively to traverse the schema.
///
/// # Error handling
///
/// The associated `ViewError` type allows fallible implementations (e.g.
/// `serde_json::Value` where the JSON may not be a valid schema) alongside
/// infallible ones (e.g. statically-derived schemas). Set `ViewError = Infallible`
/// for types that can never fail, which gives a free `.view()` method via
/// [`JsonSchemaExt`].
pub trait JsonSchema {
    type ViewError;

    /// Try to unfold one layer of this schema's structure.
    fn try_view(&self) -> Result<SchemaF<&str, &Self>, Self::ViewError>;
}

/// Convenience extension for schemas whose `try_view` can never fail.
///
/// Automatically implemented for any `JsonSchema<ViewError = Infallible>`.
pub trait JsonSchemaExt: JsonSchema<ViewError = Infallible> {
    /// Unfold one layer of this schema's structure.
    ///
    /// This is a convenience wrapper around `try_view()` for infallible schemas.
    fn view(&self) -> SchemaF<&str, &Self> {
        match self.try_view() {
            Ok(v) => v,
            Err(infallible) => match infallible {},
        }
    }
}

impl<T: JsonSchema<ViewError = Infallible>> JsonSchemaExt for T {}

/// Optional position information for schema nodes.
///
/// Implementors can provide location info (JSON Pointer path, file byte range)
/// for error reporting during subtype checks.
pub trait Located {
    fn location(&self) -> Option<&Location> {
        None
    }
}
