use crate::error::{SubtypeError, SubtypeRelation};
use crate::schema::*;
use super::{not_subtype, inhabited};

pub fn check_null_subtype() -> SubtypeRelation {
    SubtypeRelation::Subtype
}

pub fn check_boolean_subtype(sub: &BooleanSchema, sup: &BooleanSchema) -> SubtypeRelation {
    if sub.enum_values.value.is_subset_of(sup.enum_values.value) {
        SubtypeRelation::Subtype
    } else {
        not_subtype("boolean enum values not a subset")
    }
}

pub fn check_string_subtype(
    sub: &StringSchema,
    sup: &StringSchema,
) -> Result<SubtypeRelation, SubtypeError> {
    let sub_re = inhabited::compile_string_pattern(&sub.pattern.value)?;
    let sub_effective = inhabited::apply_length_constraints(sub_re, &sub.min_length, &sub.max_length)?;

    let sup_re = inhabited::compile_string_pattern(&sup.pattern.value)?;
    let sup_effective = inhabited::apply_length_constraints(sup_re, &sup.min_length, &sup.max_length)?;

    if regex_algebra::is_subset(&sub_effective, &sup_effective) {
        Ok(SubtypeRelation::Subtype)
    } else {
        Ok(not_subtype("string pattern not contained in supertype"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::{Located, Provenance};
    use crate::schema::BoolSet;

    fn make_bool(bs: BoolSet) -> BooleanSchema {
        BooleanSchema {
            provenance: Provenance::synthetic(),
            enum_values: Located::synthetic(bs),
        }
    }

    fn make_string(pattern: &str) -> StringSchema {
        StringSchema {
            provenance: Provenance::synthetic(),
            pattern: Located::synthetic(pattern.to_string()),
            min_length: None,
            max_length: None,
        }
    }

    #[test]
    fn null_subtype_of_null() {
        assert!(check_null_subtype().is_subtype());
    }

    #[test]
    fn boolean_subset() {
        let sub = make_bool(BoolSet::TrueOnly);
        let sup = make_bool(BoolSet::Both);
        assert!(check_boolean_subtype(&sub, &sup).is_subtype());
        assert!(!check_boolean_subtype(&sup, &sub).is_subtype());
    }

    #[test]
    fn boolean_empty_is_subtype_of_anything() {
        let sub = make_bool(BoolSet::Neither);
        let sup = make_bool(BoolSet::TrueOnly);
        assert!(check_boolean_subtype(&sub, &sup).is_subtype());
    }

    #[test]
    fn string_pattern_containment() {
        let sub = make_string("^[a-z]{3}$");
        let sup = make_string("^[a-z]+$");
        assert!(check_string_subtype(&sub, &sup).unwrap().is_subtype());
        assert!(!check_string_subtype(&sup, &sub).unwrap().is_subtype());
    }

    #[test]
    fn string_any_is_supertype() {
        let sub = make_string("^[a-z]+$");
        let sup = make_string(""); // empty = match everything
        assert!(check_string_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn string_with_length_constraint() {
        let sub = StringSchema {
            provenance: Provenance::synthetic(),
            pattern: Located::synthetic("[a-z]+".to_string()),
            min_length: Some(Located::synthetic(3)),
            max_length: None,
        };
        let sup = make_string("[a-z]+");
        assert!(check_string_subtype(&sub, &sup).unwrap().is_subtype());
        assert!(!check_string_subtype(&sup, &sub).unwrap().is_subtype());
    }
}
