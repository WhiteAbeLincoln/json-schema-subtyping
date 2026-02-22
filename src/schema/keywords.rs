use super::keyword::Keyword;
use super::typeset::TypeSet;

// ---------------------------------------------------------------------------
// Bound type for combined inclusive/exclusive bounds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bound {
    Unbounded,
    Inclusive(f64),
    Exclusive(f64),
}

// ---------------------------------------------------------------------------
// UpperBound: combines maximum and exclusiveMaximum
// ---------------------------------------------------------------------------

pub struct UpperBoundKw;
impl Keyword for UpperBoundKw {
    type Extracted<'a, S: 'a> = Bound;
    fn check_subtype<Sub, Sup>(
        sub: &Bound,
        sup: &Bound,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        match (sub, sup) {
            (_, Bound::Unbounded) => true,
            (Bound::Unbounded, _) => false,
            (Bound::Inclusive(s), Bound::Inclusive(p)) => s <= p,
            (Bound::Exclusive(s), Bound::Exclusive(p)) => s <= p,
            (Bound::Exclusive(s), Bound::Inclusive(p)) => s <= p,
            (Bound::Inclusive(s), Bound::Exclusive(p)) => s < p,
        }
    }
}

// ---------------------------------------------------------------------------
// LowerBound: combines minimum and exclusiveMinimum
// ---------------------------------------------------------------------------

pub struct LowerBoundKw;
impl Keyword for LowerBoundKw {
    type Extracted<'a, S: 'a> = Bound;
    fn check_subtype<Sub, Sup>(
        sub: &Bound,
        sup: &Bound,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        match (sub, sup) {
            (_, Bound::Unbounded) => true,
            (Bound::Unbounded, _) => false,
            (Bound::Inclusive(s), Bound::Inclusive(p)) => s >= p,
            (Bound::Exclusive(s), Bound::Exclusive(p)) => s >= p,
            (Bound::Exclusive(s), Bound::Inclusive(p)) => s >= p,
            (Bound::Inclusive(s), Bound::Exclusive(p)) => s > p,
        }
    }
}

// ---------------------------------------------------------------------------
// Upper-bound keywords (non-numeric): sub ≤ sup
// ---------------------------------------------------------------------------

macro_rules! upper_bound_keyword {
    ($name:ident, $ty:ty) => {
        pub struct $name;
        impl Keyword for $name {
            type Extracted<'a, S: 'a> = Option<$ty>;
            fn check_subtype<Sub, Sup>(
                sub: &Option<$ty>,
                sup: &Option<$ty>,
                _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
            ) -> bool {
                match (sub, sup) {
                    (_, None) => true,
                    (None, Some(_)) => false,
                    (Some(s), Some(p)) => *s <= *p,
                }
            }
        }
    };
}

upper_bound_keyword!(MaxLengthKw, u64);
upper_bound_keyword!(MaxItemsKw, u64);
upper_bound_keyword!(MaxPropertiesKw, u64);
upper_bound_keyword!(MaxContainsKw, u64);

// ---------------------------------------------------------------------------
// Lower-bound keywords (non-numeric): sub ≥ sup
// ---------------------------------------------------------------------------

macro_rules! lower_bound_keyword {
    ($name:ident, $ty:ty) => {
        pub struct $name;
        impl Keyword for $name {
            type Extracted<'a, S: 'a> = Option<$ty>;
            fn check_subtype<Sub, Sup>(
                sub: &Option<$ty>,
                sup: &Option<$ty>,
                _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
            ) -> bool {
                match (sub, sup) {
                    (_, None) => true,
                    (None, Some(_)) => false,
                    (Some(s), Some(p)) => *s >= *p,
                }
            }
        }
    };
}

lower_bound_keyword!(MinLengthKw, u64);
lower_bound_keyword!(MinItemsKw, u64);
lower_bound_keyword!(MinPropertiesKw, u64);
lower_bound_keyword!(MinContainsKw, u64);

// ---------------------------------------------------------------------------
// MultipleOf: sub.multipleOf must be a multiple of sup.multipleOf
// ---------------------------------------------------------------------------

pub struct MultipleOfKw;
impl Keyword for MultipleOfKw {
    type Extracted<'a, S: 'a> = Option<f64>;
    fn check_subtype<Sub, Sup>(
        sub: &Option<f64>,
        sup: &Option<f64>,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        match (sub, sup) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(s), Some(p)) => {
                let ratio = s / p;
                (ratio - ratio.round()).abs() < 1e-9
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Type: subset check
// ---------------------------------------------------------------------------

pub struct TypeKw;
impl Keyword for TypeKw {
    type Extracted<'a, S: 'a> = Option<TypeSet>;
    fn check_subtype<Sub, Sup>(
        sub: &Option<TypeSet>,
        sup: &Option<TypeSet>,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        let sub_set = sub.unwrap_or(TypeSet::all());
        let mut sup_set = sup.unwrap_or(TypeSet::all());
        // integer is a subtype of number
        if sup_set.contains(TypeSet::NUMBER) {
            sup_set |= TypeSet::INTEGER;
        }
        sup_set.contains(sub_set)
    }
}

// ---------------------------------------------------------------------------
// UniqueItems: true is more restrictive than false
// ---------------------------------------------------------------------------

pub struct UniqueItemsKw;
impl Keyword for UniqueItemsKw {
    type Extracted<'a, S: 'a> = bool;
    fn check_subtype<Sub, Sup>(
        sub: &bool,
        sup: &bool,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        !sup || *sub
    }
}

// ---------------------------------------------------------------------------
// Required: sub ⊇ sup (sub requires at least everything sup requires)
// ---------------------------------------------------------------------------

pub struct RequiredKw;
impl Keyword for RequiredKw {
    type Extracted<'a, S: 'a> = Vec<&'a str>;
    fn check_subtype<Sub, Sup>(
        sub: &Vec<&str>,
        sup: &Vec<&str>,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        sup.iter().all(|s| sub.contains(s))
    }
}

// ---------------------------------------------------------------------------
// Const / Enum keywords
// ---------------------------------------------------------------------------

use super::json_value::JsonValue;

pub struct ConstKw;
impl Keyword for ConstKw {
    type Extracted<'a, S: 'a> = Option<JsonValue>;
    fn check_subtype<Sub, Sup>(
        sub: &Option<JsonValue>,
        sup: &Option<JsonValue>,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        match (sub, sup) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(s), Some(p)) => s == p,
        }
    }
}

pub struct EnumKw;
impl Keyword for EnumKw {
    type Extracted<'a, S: 'a> = Option<Vec<JsonValue>>;
    fn check_subtype<Sub, Sup>(
        sub: &Option<Vec<JsonValue>>,
        sup: &Option<Vec<JsonValue>>,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        match (sub, sup) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(s), Some(p)) => s.iter().all(|sv| p.iter().any(|pv| sv == pv)),
        }
    }
}

// ---------------------------------------------------------------------------
// Applicator: optional recursive (Option<&S>)
// ---------------------------------------------------------------------------

macro_rules! optional_applicator {
    ($name:ident) => {
        pub struct $name;
        impl Keyword for $name {
            type Extracted<'a, S: 'a> = Option<&'a S>;
            fn check_subtype<Sub, Sup>(
                sub: &Option<&Sub>,
                sup: &Option<&Sup>,
                is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
            ) -> bool {
                match (sub, sup) {
                    (_, None) => true,
                    (None, Some(_)) => false,
                    (Some(s), Some(p)) => is_subtype(s, p),
                }
            }
        }
    };
}

optional_applicator!(ItemsKw);
optional_applicator!(ContainsKw);
optional_applicator!(AdditionalPropertiesKw);
optional_applicator!(PropertyNamesKw);

// ---------------------------------------------------------------------------
// Applicator: Properties (covariant recursive key-value pairs)
// Uses Option<&S> where None = top schema (unconstrained)
// ---------------------------------------------------------------------------

pub struct PropertiesKw;
impl Keyword for PropertiesKw {
    type Extracted<'a, S: 'a> = Vec<(&'a str, Option<&'a S>)>;
    fn check_subtype<Sub, Sup>(
        sub: &Vec<(&str, Option<&Sub>)>,
        sup: &Vec<(&str, Option<&Sup>)>,
        is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        sup.iter().all(|(key, sup_child)| {
            let sub_child = sub
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v);
            match (sub_child, sup_child) {
                // sup's property is top → any sub is fine
                (_, None) => true,
                // sub absent or explicitly top, sup constrained → fail
                (None, Some(_)) => false,
                (Some(None), Some(_)) => false,
                // both constrained → recursive check
                (Some(Some(sub_c)), Some(sup_c)) => is_subtype(sub_c, sup_c),
            }
        })
    }
}

pub struct PatternPropertiesKw;
impl Keyword for PatternPropertiesKw {
    type Extracted<'a, S: 'a> = Vec<(&'a str, Option<&'a S>)>;
    fn check_subtype<Sub, Sup>(
        sub: &Vec<(&str, Option<&Sub>)>,
        sup: &Vec<(&str, Option<&Sup>)>,
        is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        sup.iter().all(|(key, sup_child)| {
            let sub_child = sub
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v);
            match (sub_child, sup_child) {
                (_, None) => true,
                (None, Some(_)) => false,
                (Some(None), Some(_)) => false,
                (Some(Some(sub_c)), Some(sup_c)) => is_subtype(sub_c, sup_c),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Applicator: PrefixItems (list recursive)
// ---------------------------------------------------------------------------

pub struct PrefixItemsKw;
impl Keyword for PrefixItemsKw {
    type Extracted<'a, S: 'a> = Vec<&'a S>;
    fn check_subtype<Sub, Sup>(
        sub: &Vec<&Sub>,
        sup: &Vec<&Sup>,
        is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        sup.iter().enumerate().all(|(i, sup_child)| {
            match sub.get(i) {
                Some(sub_child) => is_subtype(sub_child, sup_child),
                None => false,
            }
        })
    }
}
