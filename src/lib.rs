use snafu::Snafu;

pub mod output;
pub mod schema;

#[cfg(feature = "serde")]
mod serde_impl;

pub use output::{FilePosition, JsonPointer, Location};
pub use schema::{
    Annotated, JsonSchema, JsonSchemaExt, JsonValue, Located, Schema, SchemaF, SchemaObject,
    TypeSet,
};
#[cfg(feature = "serde")]
pub use serde_impl::ViewError;

use crate::output::DetailedOutput;

pub enum SubtypeRelation {
    Subtype,
    // TODO: allow choosing between basic and detailed output
    NotSubtype(DetailedOutput),
}

#[derive(Debug, Snafu)]
pub enum SubtypeError {
    // TODO: include location information
    UnsupportedKeyword {
        keyword: String,
        location: Location,
    },
    #[snafu(display("{source}"))]
    InvalidSchema {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Checks if `sub` is a subtype of `sup` according to the JSON Schema specification.
pub fn is_subtype<A: JsonSchema, B: JsonSchema>(
    _sup: &A,
    _sub: &B,
) -> Result<SubtypeRelation, SubtypeError>
where
    A::ViewError: Into<SubtypeError>,
    B::ViewError: Into<SubtypeError>,
{
    todo!()
}

// Infallible schemas satisfy the Into<SubtypeError> bound automatically
// via the blanket `From<Infallible>` impl in std.

#[cfg(feature = "serde")]
impl From<ViewError> for SubtypeError {
    fn from(e: ViewError) -> Self {
        SubtypeError::InvalidSchema {
            source: Box::new(e),
        }
    }
}
