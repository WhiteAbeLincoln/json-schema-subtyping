use std::ops::Range;

pub type JsonPointer = String;
pub type FilePosition = Range<usize>;

/// A trait for items that have a known
/// position in a file.
pub trait FilePositioned {
    /// Returns the byte range of this item in the source file.
    fn range(&self) -> Range<usize>;
}

/// A trait for items that have a position in a
/// JSON document, represented as a JSON Pointer.
pub trait JsonPointerPositioned {
    /// Returns the JSON Pointer to this item in the document.
    fn json_pointer(&self) -> JsonPointer;
}

/// A trait for types that can be used as JSON Schemas.
/// This allows different concrete representations of JSON Schemas to be used with the same subtyping logic.
///
/// i.e. a serde-json based representation,
/// a jsonc based representation, a custom AST, etc.
pub trait JsonSchema {
    // TODO: fill with getters for various keywords
}
