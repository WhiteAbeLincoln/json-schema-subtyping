use snafu::Snafu;

pub mod output;
pub mod schema;

#[cfg(feature = "serde")]
mod serde_impl;

pub use output::{FilePosition, JsonPointer, Location};
pub use schema::{
    Draft2020_12, Get, JsonValue, Keyword, QuerySchema, SchemaKind, TypeSet, check_keywords,
};
#[cfg(feature = "serde")]
pub use serde_impl::ViewError;

use crate::output::{DetailedOutput, OutputError};

pub enum SubtypeRelation {
    Subtype,
    NotSubtype(DetailedOutput),
}

#[derive(Debug, Snafu)]
pub enum SubtypeError {
    #[snafu(display("{source}"))]
    InvalidSchema {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

fn not_subtype() -> SubtypeRelation {
    SubtypeRelation::NotSubtype(DetailedOutput {
        supertype_location: Location {
            path: String::new(),
            file_position: None,
        },
        subtype_location: Location {
            path: String::new(),
            file_position: None,
        },
        error: OutputError::Basic("not a subtype".to_string()),
    })
}

/// Checks if `sub` is a subtype of `sup` according to the JSON Schema specification.
pub fn is_subtype<A: Draft2020_12, B: Draft2020_12>(
    sup: &A,
    sub: &B,
) -> Result<SubtypeRelation, SubtypeError>
where
    A::Error: Into<SubtypeError>,
    B::Error: Into<SubtypeError>,
{
    use SchemaKind::*;

    let sup_kind = sup.kind().map_err(Into::into)?;
    let sub_kind = sub.kind().map_err(Into::into)?;

    match (sup_kind, sub_kind) {
        // Everything is a subtype of top
        (Top, _) => Ok(SubtypeRelation::Subtype),
        // Bottom is a subtype of everything
        (_, Bottom) => Ok(SubtypeRelation::Subtype),
        // Nothing else is a subtype of bottom
        (Bottom, _) => Ok(not_subtype()),
        // Constrained vs Top or Constrained: compare keywords
        (Constrained, Top) | (Constrained, Constrained) => {
            let mut error: Option<SubtypeError> = None;
            let mut callback = |sub_child: &B, sup_child: &A| -> bool {
                if error.is_some() {
                    return false;
                }
                match is_subtype(sup_child, sub_child) {
                    Ok(SubtypeRelation::Subtype) => true,
                    Ok(SubtypeRelation::NotSubtype(_)) => false,
                    Err(e) => {
                        error = Some(e);
                        false
                    }
                }
            };

            let result: Result<bool, SubtypeError> =
                check_keywords(sub, sup, &mut callback);

            if let Some(e) = error {
                return Err(e);
            }

            if result? {
                Ok(SubtypeRelation::Subtype)
            } else {
                Ok(not_subtype())
            }
        }
    }
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
