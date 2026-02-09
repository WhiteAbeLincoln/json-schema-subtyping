use snafu::Snafu;
pub mod output;
pub mod schema;
pub use schema::{FilePosition, JsonPointer, JsonSchema};

use crate::output::{DetailedOutput, Location};

pub enum SubtypeRelation {
    Subtype,
    // TODO: allow choosing between basic and detailed output
    NotSubtype(DetailedOutput),
}

#[derive(Debug, Snafu)]
pub enum SubtypeError {
    // TODO: include location information
    UnsupportedKeyword { keyword: String, location: Location },
}

/// Checks if `a` is a subtype of `b` according to the JSON Schema specification.
pub fn is_subtype<A: JsonSchema, B: JsonSchema>(
    sup: &A,
    sub: &B,
) -> Result<SubtypeRelation, SubtypeError> {
    todo!()
}
