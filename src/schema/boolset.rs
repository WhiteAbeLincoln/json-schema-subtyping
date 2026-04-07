/// The set of boolean values a schema permits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolSet {
    Neither,
    TrueOnly,
    FalseOnly,
    Both,
}

impl BoolSet {
    pub fn from_values(has_true: bool, has_false: bool) -> Self {
        match (has_true, has_false) {
            (false, false) => Self::Neither,
            (true, false) => Self::TrueOnly,
            (false, true) => Self::FalseOnly,
            (true, true) => Self::Both,
        }
    }

    pub fn is_subset_of(self, other: Self) -> bool {
        let s: u8 = self.into();
        let o: u8 = other.into();
        s & o == s
    }

    pub fn intersect(self, other: Self) -> Self {
        let s: u8 = self.into();
        let o: u8 = other.into();
        (s & o).into()
    }

    pub fn union(self, other: Self) -> Self {
        let s: u8 = self.into();
        let o: u8 = other.into();
        (s | o).into()
    }

    pub fn complement(self) -> Self {
        let s: u8 = self.into();
        ((!s) & 0b11).into()
    }

    pub fn is_empty(self) -> bool {
        self == Self::Neither
    }
}

impl From<BoolSet> for u8 {
    fn from(bs: BoolSet) -> u8 {
        match bs {
            BoolSet::Neither => 0b00,
            BoolSet::TrueOnly => 0b10,
            BoolSet::FalseOnly => 0b01,
            BoolSet::Both => 0b11,
        }
    }
}

impl From<u8> for BoolSet {
    fn from(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::Neither,
            0b10 => Self::TrueOnly,
            0b01 => Self::FalseOnly,
            0b11 => Self::Both,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subset() {
        assert!(BoolSet::TrueOnly.is_subset_of(BoolSet::Both));
        assert!(!BoolSet::Both.is_subset_of(BoolSet::TrueOnly));
    }

    #[test]
    fn complement() {
        assert_eq!(BoolSet::TrueOnly.complement(), BoolSet::FalseOnly);
        assert_eq!(BoolSet::Both.complement(), BoolSet::Neither);
    }

    #[test]
    fn intersect() {
        assert_eq!(BoolSet::Both.intersect(BoolSet::TrueOnly), BoolSet::TrueOnly);
    }
}
