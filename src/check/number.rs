use crate::error::{SubtypeError, SubtypeRelation};
use crate::schema::NumberSchema;
use super::not_subtype;

/// Effective upper bound: the tighter of inclusive maximum and exclusive maximum.
/// Returns (value, is_inclusive).
fn effective_upper(max: f64, excl_max: f64) -> (f64, bool) {
    if max < excl_max {
        (max, true)
    } else if excl_max < max {
        (excl_max, false)
    } else {
        // Equal: exclusive is tighter
        (excl_max, false)
    }
}

/// Effective lower bound: the tighter of inclusive minimum and exclusive minimum.
/// Returns (value, is_inclusive).
fn effective_lower(min: f64, excl_min: f64) -> (f64, bool) {
    if min > excl_min {
        (min, true)
    } else {
        (excl_min, false)
    }
}

/// Check sub's upper bound is within sup's upper bound.
fn upper_contained(sub: (f64, bool), sup: (f64, bool)) -> bool {
    sub.0 < sup.0 || (sub.0 == sup.0 && (!sub.1 || sup.1))
}

/// Check sub's lower bound is within sup's lower bound.
fn lower_contained(sub: (f64, bool), sup: (f64, bool)) -> bool {
    sub.0 > sup.0 || (sub.0 == sup.0 && (!sub.1 || sup.1))
}

pub fn check_number_subtype(
    sub: &NumberSchema,
    sup: &NumberSchema,
) -> Result<SubtypeRelation, SubtypeError> {
    // Compute effective bounds: the tighter of inclusive/exclusive for each side.
    // An effective bound is (value, is_inclusive).
    let sub_upper = effective_upper(sub.maximum.value, sub.exclusive_maximum.value);
    let sup_upper = effective_upper(sup.maximum.value, sup.exclusive_maximum.value);
    let sub_lower = effective_lower(sub.minimum.value, sub.exclusive_minimum.value);
    let sup_lower = effective_lower(sup.minimum.value, sup.exclusive_minimum.value);

    // sub's range must be contained within sup's range.
    if !lower_contained(sub_lower, sup_lower) {
        return Ok(not_subtype("sub lower bound is below sup lower bound"));
    }
    if !upper_contained(sub_upper, sup_upper) {
        return Ok(not_subtype("sub upper bound is above sup upper bound"));
    }

    // multipleOf containment:
    // If sup has multipleOf M, then sub must also have multipleOf N where M | N.
    if let Some(ref sup_m) = sup.multiple_of {
        match &sub.multiple_of {
            Some(sub_m) => {
                // sub.multipleOf must be a multiple of sup.multipleOf.
                // i.e., sub_m / sup_m must be an integer.
                let ratio = sub_m.value / sup_m.value;
                if (ratio - ratio.round()).abs() > 1e-10 {
                    return Ok(not_subtype("sub multipleOf is not a multiple of sup multipleOf"));
                }
            }
            None => {
                return Ok(not_subtype("sup requires multipleOf but sub has none"));
            }
        }
    }

    Ok(SubtypeRelation::Subtype)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::{Located, Provenance};

    fn make_number(min: f64, max: f64) -> NumberSchema {
        NumberSchema {
            provenance: Provenance::synthetic(),
            minimum: Located::synthetic(min),
            maximum: Located::synthetic(max),
            exclusive_minimum: Located::synthetic(f64::NEG_INFINITY),
            exclusive_maximum: Located::synthetic(f64::INFINITY),
            multiple_of: None,
        }
    }

    fn make_number_with_multiple(min: f64, max: f64, m: Option<f64>) -> NumberSchema {
        NumberSchema {
            provenance: Provenance::synthetic(),
            minimum: Located::synthetic(min),
            maximum: Located::synthetic(max),
            exclusive_minimum: Located::synthetic(f64::NEG_INFINITY),
            exclusive_maximum: Located::synthetic(f64::INFINITY),
            multiple_of: m.map(Located::synthetic),
        }
    }

    #[test]
    fn range_subtype() {
        let sub = make_number(0.0, 10.0);
        let sup = make_number(-5.0, 100.0);
        assert!(check_number_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn range_not_subtype() {
        let sub = make_number(-5.0, 100.0);
        let sup = make_number(0.0, 10.0);
        assert!(!check_number_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn equal_ranges() {
        let a = make_number(0.0, 10.0);
        let b = make_number(0.0, 10.0);
        assert!(check_number_subtype(&a, &b).unwrap().is_subtype());
    }

    #[test]
    fn multiple_of_subtype() {
        // multipleOf 6 <: multipleOf 3 (6 is divisible by 3)
        let sub = make_number_with_multiple(f64::NEG_INFINITY, f64::INFINITY, Some(6.0));
        let sup = make_number_with_multiple(f64::NEG_INFINITY, f64::INFINITY, Some(3.0));
        assert!(check_number_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn multiple_of_not_subtype() {
        // multipleOf 3 NOT <: multipleOf 6
        let sub = make_number_with_multiple(f64::NEG_INFINITY, f64::INFINITY, Some(3.0));
        let sup = make_number_with_multiple(f64::NEG_INFINITY, f64::INFINITY, Some(6.0));
        assert!(!check_number_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn sup_no_multiple_of_accepts_any() {
        let sub = make_number_with_multiple(0.0, 100.0, Some(5.0));
        let sup = make_number(0.0, 100.0);
        assert!(check_number_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn sub_no_multiple_of_fails_when_sup_requires() {
        let sub = make_number(0.0, 100.0);
        let sup = make_number_with_multiple(0.0, 100.0, Some(5.0));
        assert!(!check_number_subtype(&sub, &sup).unwrap().is_subtype());
    }
}
