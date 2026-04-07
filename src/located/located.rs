use super::span::Provenance;

/// A value annotated with source provenance.
#[derive(Debug, Clone)]
pub struct Located<T> {
    pub provenance: Provenance,
    pub value: T,
}

impl<T> Located<T> {
    pub fn new(provenance: Provenance, value: T) -> Self {
        Self { provenance, value }
    }

    pub fn synthetic(value: T) -> Self {
        Self {
            provenance: Provenance::synthetic(),
            value,
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Located<U> {
        Located {
            provenance: self.provenance,
            value: f(self.value),
        }
    }
}
