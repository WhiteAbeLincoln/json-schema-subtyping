use snafu::Snafu;

/// Represents the result of a subtype check.
#[derive(Debug)]
pub enum SubtypeRelation {
    /// `sub` is a subtype of `sup`.
    Subtype,
    /// `sub` is not a subtype of `sup`, with details about the failure.
    NotSubtype(DetailedOutput),
}

impl SubtypeRelation {
    pub fn is_subtype(&self) -> bool {
        matches!(self, SubtypeRelation::Subtype)
    }
}

/// Details about why a subtype check failed.
#[derive(Debug)]
pub struct DetailedOutput {
    pub message: String,
}

#[derive(Debug, Snafu)]
pub enum SubtypeError {
    #[snafu(display("{source}"))]
    InvalidSchema {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[snafu(display("Unsupported schema features: {details}"))]
    UnsupportedFeatures { details: String },
    #[snafu(display("Rewrite failed: {message}"))]
    RewriteFailed { message: String },
}
