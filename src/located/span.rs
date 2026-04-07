use smallvec::SmallVec;

/// A byte offset range in the original source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A JSON Pointer path (RFC 6901), e.g., "/properties/name/type".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPointer(pub Vec<String>);

impl JsonPointer {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn push(&self, segment: impl Into<String>) -> Self {
        let mut p = self.0.clone();
        p.push(segment.into());
        Self(p)
    }
}

impl std::fmt::Display for JsonPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in &self.0 {
            write!(f, "/{segment}")?;
        }
        Ok(())
    }
}

/// Tracks where a value came from in the original source.
/// A single value may be derived from multiple source locations
/// (e.g., when a rewrite rule merges multiple keywords).
#[derive(Debug, Clone)]
pub struct Provenance {
    pub spans: SmallVec<[Span; 1]>,
    pub pointers: SmallVec<[JsonPointer; 1]>,
}

impl Provenance {
    pub fn new(span: Span, pointer: JsonPointer) -> Self {
        Self {
            spans: SmallVec::from_elem(span, 1),
            pointers: SmallVec::from_elem(pointer, 1),
        }
    }

    /// A synthetic provenance with no source location.
    pub fn synthetic() -> Self {
        Self {
            spans: SmallVec::new(),
            pointers: SmallVec::new(),
        }
    }

    /// Merge two provenances (when a rewrite combines multiple sources).
    pub fn merge(&self, other: &Provenance) -> Self {
        let mut spans = self.spans.clone();
        spans.extend(other.spans.iter().copied());
        let mut pointers = self.pointers.clone();
        pointers.extend(other.pointers.iter().cloned());
        Self { spans, pointers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_pointer_display() {
        assert_eq!(JsonPointer::root().to_string(), "");
        assert_eq!(
            JsonPointer::root()
                .push("properties")
                .push("name")
                .to_string(),
            "/properties/name"
        );
    }

    #[test]
    fn provenance_merge() {
        let a = Provenance::new(Span::new(0, 5), JsonPointer::root().push("a"));
        let b = Provenance::new(Span::new(10, 15), JsonPointer::root().push("b"));
        let merged = a.merge(&b);
        assert_eq!(merged.spans.len(), 2);
        assert_eq!(merged.pointers.len(), 2);
    }
}
