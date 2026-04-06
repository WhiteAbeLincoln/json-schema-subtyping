use std::ops::Range;

use snafu::Snafu;

/// Represents a specific location in the JSON Schema where an error occurred.
pub struct Location {
    /// A json pointer to the location in the schema where the error occurred.
    pointer: String,
    /// The original source location in the input schema string for error reporting.
    source_location: Range<usize>,
    line: usize,
    column: usize,
}

pub struct DetailedOutput {
    // TODO: output including relevant schema locations and a human-readable error message
}

/// Represents the result of a subtype check.
pub enum SubtypeRelation {
    /// `sub` is a subtype of `sup`.
    Subtype,
    /// `sub` is not a subtype of `sup`, with details about the failure.
    NotSubtype(DetailedOutput),
}

#[derive(Debug, Snafu)]
pub enum SubtypeError {
    /// The schema is invalid and cannot be processed.
    #[snafu(display("{source}"))]
    InvalidSchema {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Unsupported schema features that prevent subtype checking.
    #[snafu(display("Unsupported schema features: {details}"))]
    UnsupportedFeatures { details: String },
}

#[derive(Debug, Clone, Copy)]
pub struct SubtypeChecker {
    options: SubtypeCheckerOptions,
}

/// Options for the subtype checker.
/// Contains the rewrite and inference rules.
#[derive(Debug, Clone, Copy)]
pub struct SubtypeCheckerOptions {}

/// Represents a parsed JSON Schema.
/// All rewritten forms point back to the original source locations for error reporting.
#[derive(Debug, Clone, Copy)]
pub struct ParsedSchema<'a> {
    source: &'a str,
    // need to hold on to options which defines
    // the canonicalization, simplification, and inference rules
}

impl SubtypeChecker {
    /// Checks if `sub` is a subtype of `sup` according to the JSON Schema specification.
    pub fn is_subtype(sup: &str, sub: &str) -> Result<SubtypeRelation, SubtypeError> {
        // step 1: parse the schemas
        // step 2: for each schema: convert to 2020 draft, canonicalize, and simplify
        // step 3: check if `sub` is a subtype of `sup`
        todo!()
    }

    /// Parses a JSON Schema string into an internal representation suitable for subtype checking.
    pub fn parse_schema<'a>(
        schema: &str,
        name: Option<String>,
    ) -> Result<ParsedSchema<'a>, SubtypeError> {
        todo!()
    }
}

impl<'a> ParsedSchema<'a> {
    /// Rewrites equivalent schemas into a canonical form to facilitate comparison
    pub fn canonicalize(&mut self) -> Result<(), SubtypeError> {
        todo!()
    }

    /// Converts a non-draft-2020 schema to draft-2020, preserving semantics
    pub fn to_draft_2020(&mut self) -> Result<(), SubtypeError> {
        todo!()
    }

    /// Further simplifies the schema by removing redundant constructs.
    pub fn simplify(&mut self) -> Result<(), SubtypeError> {
        todo!()
    }

    /// Checks if `self` is a subtype of `other` according to the JSON Schema specification.
    pub fn is_subtype_of<'b>(
        &self,
        other: &ParsedSchema<'b>,
    ) -> Result<SubtypeRelation, SubtypeError> {
        todo!()
    }
}
