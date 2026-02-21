use std::ops::Range;

pub type JsonPointer = String;
pub type FilePosition = Range<usize>;

#[derive(Debug)]
pub struct Location {
    pub path: JsonPointer,
    pub file_position: Option<FilePosition>,
}

/// Records the location of a mismatch in a subtyping check,
/// along with the error message itself.
pub struct Output<E> {
    /// The json pointer to the location in
    /// the first schema where the error occurred.
    pub supertype_location: Location,

    /// The json pointer to the location in the
    /// second schema where the error occurred.
    pub subtype_location: Location,

    // TODO: include absolute variants of locations?
    // TODO: include file-position information?
    pub error: E,
}

pub enum OutputError {
    Basic(String),
    Detailed(Vec<Output<OutputError>>),
}

pub type DetailedOutput = Output<OutputError>;
pub type BasicOutput = Output<String>;
