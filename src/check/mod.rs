mod array;
pub mod inhabited;
mod number;
mod object;
mod primitive;

use crate::error::{DetailedOutput, SubtypeError, SubtypeRelation};
use crate::schema::*;

/// Check if `sub` is a subtype of `sup` (every value accepted by sub is also accepted by sup).
pub fn is_subtype(
    sub: &ReducedSchema,
    sup: &ReducedSchema,
) -> Result<SubtypeRelation, SubtypeError> {
    // Top is supertype of everything.
    if matches!(sup, ReducedSchema::Top(_)) {
        return Ok(SubtypeRelation::Subtype);
    }
    // Bottom is subtype of everything.
    if matches!(sub, ReducedSchema::Bottom(_)) {
        return Ok(SubtypeRelation::Subtype);
    }
    // Nothing else is subtype of bottom.
    if matches!(sup, ReducedSchema::Bottom(_)) {
        return Ok(not_subtype("sup is bottom but sub is not"));
    }
    // Top is not subtype of anything except top (handled above).
    if matches!(sub, ReducedSchema::Top(_)) {
        return Ok(not_subtype("sub is top but sup is not top"));
    }
    // Uninhabited sub is subtype of anything.
    if inhabited::is_uninhabited(sub)? {
        return Ok(SubtypeRelation::Subtype);
    }

    match (sub, sup) {
        // AnyOf sub: each branch must be <: sup.
        (ReducedSchema::AnyOf(_, sub_branches), _) => {
            for branch in sub_branches {
                if !is_subtype(branch, sup)?.is_subtype() {
                    return Ok(not_subtype(
                        "not all branches of sub's anyOf are subtypes of sup",
                    ));
                }
            }
            Ok(SubtypeRelation::Subtype)
        }
        // AllOf sup: sub must be <: each branch.
        (_, ReducedSchema::AllOf(_, sup_branches)) => {
            for branch in sup_branches {
                if !is_subtype(sub, branch)?.is_subtype() {
                    return Ok(not_subtype(
                        "sub is not a subtype of all branches of sup's allOf",
                    ));
                }
            }
            Ok(SubtypeRelation::Subtype)
        }
        // AnyOf sup: sub must be <: at least one branch.
        (_, ReducedSchema::AnyOf(_, sup_branches)) => {
            for branch in sup_branches {
                if is_subtype(sub, branch)?.is_subtype() {
                    return Ok(SubtypeRelation::Subtype);
                }
            }
            Ok(not_subtype(
                "sub is not a subtype of any branch of sup's anyOf",
            ))
        }
        // AllOf sub: conservative — check if any branch is <: sup.
        (ReducedSchema::AllOf(_, sub_branches), _) => {
            for branch in sub_branches {
                if is_subtype(branch, sup)?.is_subtype() {
                    return Ok(SubtypeRelation::Subtype);
                }
            }
            Ok(not_subtype("no allOf branch individually <: sup"))
        }
        // Typed × Typed: dispatch to type-specific check.
        (ReducedSchema::Typed(sub_t), ReducedSchema::Typed(sup_t)) => {
            check_typed_subtype(sub_t, sup_t)
        }
        // Not: conservative handling.
        _ => Ok(not_subtype("incompatible schema shapes")),
    }
}

pub(crate) fn not_subtype(message: &str) -> SubtypeRelation {
    SubtypeRelation::NotSubtype(DetailedOutput {
        message: message.to_string(),
    })
}

fn check_typed_subtype(
    sub: &TypedSchema,
    sup: &TypedSchema,
) -> Result<SubtypeRelation, SubtypeError> {
    match (sub, sup) {
        (TypedSchema::Null(_), TypedSchema::Null(_)) => Ok(primitive::check_null_subtype()),
        (TypedSchema::Boolean(s), TypedSchema::Boolean(p)) => {
            Ok(primitive::check_boolean_subtype(s, p))
        }
        (TypedSchema::String(s), TypedSchema::String(p)) => primitive::check_string_subtype(s, p),
        (TypedSchema::Number(s), TypedSchema::Number(p)) => number::check_number_subtype(s, p),
        (TypedSchema::Array(s), TypedSchema::Array(p)) => array::check_array_subtype(s, p),
        (TypedSchema::Object(s), TypedSchema::Object(p)) => object::check_object_subtype(s, p),
        // Different types: not subtypes of each other.
        _ => Ok(not_subtype("different JSON types")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::{Located, Provenance};
    use crate::schema::BoolSet;

    fn top() -> ReducedSchema {
        ReducedSchema::Top(Provenance::synthetic())
    }

    fn bottom() -> ReducedSchema {
        ReducedSchema::Bottom(Provenance::synthetic())
    }

    fn null() -> ReducedSchema {
        ReducedSchema::Typed(Box::new(TypedSchema::Null(Provenance::synthetic())))
    }

    fn make_string_schema(pattern: &str) -> ReducedSchema {
        ReducedSchema::Typed(Box::new(TypedSchema::String(StringSchema {
            provenance: Provenance::synthetic(),
            pattern: Located::synthetic(pattern.to_string()),
            min_length: None,
            max_length: None,
        })))
    }

    fn make_number_schema(min: f64, max: f64) -> ReducedSchema {
        ReducedSchema::Typed(Box::new(TypedSchema::Number(NumberSchema {
            provenance: Provenance::synthetic(),
            minimum: Located::synthetic(min),
            maximum: Located::synthetic(max),
            exclusive_minimum: Located::synthetic(f64::NEG_INFINITY),
            exclusive_maximum: Located::synthetic(f64::INFINITY),
            multiple_of: None,
        })))
    }

    // --- Structural rules ---

    #[test]
    fn top_is_supertype_of_everything() {
        assert!(is_subtype(&bottom(), &top()).unwrap().is_subtype());
        assert!(is_subtype(&make_string_schema(""), &top()).unwrap().is_subtype());
        assert!(is_subtype(&null(), &top()).unwrap().is_subtype());
    }

    #[test]
    fn bottom_is_subtype_of_everything() {
        assert!(is_subtype(&bottom(), &top()).unwrap().is_subtype());
        assert!(is_subtype(&bottom(), &make_string_schema("")).unwrap().is_subtype());
        assert!(is_subtype(&bottom(), &null()).unwrap().is_subtype());
    }

    #[test]
    fn nothing_is_subtype_of_bottom() {
        assert!(!is_subtype(&make_string_schema(""), &bottom()).unwrap().is_subtype());
        assert!(!is_subtype(&top(), &bottom()).unwrap().is_subtype());
    }

    #[test]
    fn top_not_subtype_of_typed() {
        assert!(!is_subtype(&top(), &null()).unwrap().is_subtype());
    }

    #[test]
    fn different_types_not_subtype() {
        assert!(!is_subtype(&null(), &make_string_schema("")).unwrap().is_subtype());
        assert!(!is_subtype(&make_string_schema(""), &make_number_schema(0.0, 10.0)).unwrap().is_subtype());
    }

    // --- AnyOf ---

    #[test]
    fn anyof_subtype_when_each_branch_has_supertype() {
        let sub = ReducedSchema::AnyOf(Provenance::synthetic(), vec![null(), make_string_schema("")]);
        let sup = ReducedSchema::AnyOf(
            Provenance::synthetic(),
            vec![null(), make_string_schema(""), make_number_schema(f64::NEG_INFINITY, f64::INFINITY)],
        );
        assert!(is_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn anyof_sub_branch_not_covered() {
        let sub = ReducedSchema::AnyOf(
            Provenance::synthetic(),
            vec![null(), make_number_schema(0.0, 10.0)],
        );
        let sup = make_string_schema("");
        assert!(!is_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn anyof_sup_sub_matches_one_branch() {
        let sub = null();
        let sup = ReducedSchema::AnyOf(
            Provenance::synthetic(),
            vec![null(), make_string_schema("")],
        );
        assert!(is_subtype(&sub, &sup).unwrap().is_subtype());
    }

    // --- AllOf ---

    #[test]
    fn allof_sup_sub_must_match_all() {
        let sub = make_number_schema(0.0, 10.0);
        let sup = ReducedSchema::AllOf(
            Provenance::synthetic(),
            vec![
                make_number_schema(-5.0, 100.0),
                make_number_schema(-10.0, 50.0),
            ],
        );
        assert!(is_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn allof_sup_fails_if_one_branch_fails() {
        let sub = make_number_schema(0.0, 100.0);
        let sup = ReducedSchema::AllOf(
            Provenance::synthetic(),
            vec![
                make_number_schema(0.0, 50.0), // sub max (100) > branch max (50)
                make_number_schema(0.0, 200.0),
            ],
        );
        assert!(!is_subtype(&sub, &sup).unwrap().is_subtype());
    }

    // --- Uninhabited ---

    #[test]
    fn uninhabited_is_subtype_of_anything() {
        // empty boolean set is uninhabited
        let empty_bool = ReducedSchema::Typed(Box::new(TypedSchema::Boolean(BooleanSchema {
            provenance: Provenance::synthetic(),
            enum_values: Located::synthetic(BoolSet::Neither),
        })));
        assert!(is_subtype(&empty_bool, &null()).unwrap().is_subtype());
        assert!(is_subtype(&empty_bool, &make_string_schema("")).unwrap().is_subtype());
    }

    // --- Cross-type with anyOf ---

    #[test]
    fn typed_subtype_of_anyof_containing_type() {
        let sub = make_string_schema("^[a-z]+$");
        let sup = ReducedSchema::AnyOf(
            Provenance::synthetic(),
            vec![make_string_schema(""), make_number_schema(0.0, 10.0)],
        );
        assert!(is_subtype(&sub, &sup).unwrap().is_subtype());
    }
}
