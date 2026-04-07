use crate::error::{SubtypeError, SubtypeRelation};
use crate::schema::*;
use super::{is_subtype, not_subtype};

pub fn check_object_subtype(
    sub: &ObjectSchema,
    sup: &ObjectSchema,
) -> Result<SubtypeRelation, SubtypeError> {
    // 1. minProperties/maxProperties range containment
    if sub.min_properties.value < sup.min_properties.value {
        return Ok(not_subtype("sub minProperties < sup minProperties"));
    }
    if sub.max_properties.value > sup.max_properties.value {
        return Ok(not_subtype("sub maxProperties > sup maxProperties"));
    }

    // 2. required: sub must require at least everything sup requires.
    // sub.required ⊇ sup.required
    for sup_req in &sup.required {
        let found = sub.required.iter().any(|sr| sr.value == sup_req.value);
        if !found {
            return Ok(not_subtype(&format!(
                "sup requires {:?} but sub does not",
                sup_req.value
            )));
        }
    }

    // 3. Pattern properties: for every sup pattern p2:s2, every property name
    //    matching p2 in an object accepted by sub must also satisfy s2.
    //    For each sub pattern p1:s1 that overlaps with p2, we need s1 <: s2.
    //    If no sub pattern overlaps with p2, then sub places no constraint on
    //    those properties (effectively Top), so we need Top <: s2.
    for (sup_pat, sup_schema) in &sup.pattern_properties {
        let sup_re = compile_property_pattern(&sup_pat.value)?;
        let mut covered = false;

        for (sub_pat, sub_schema) in &sub.pattern_properties {
            let sub_re = compile_property_pattern(&sub_pat.value)?;
            let inter = regex_algebra::intersect(&sub_re, &sup_re);

            if !regex_algebra::is_empty(&inter) {
                covered = true;
                if !is_subtype(sub_schema, sup_schema)?.is_subtype() {
                    return Ok(not_subtype(&format!(
                        "overlapping patterns {:?} and {:?}: sub schema not <: sup schema",
                        sub_pat.value, sup_pat.value
                    )));
                }
            }
        }

        // If no sub pattern covers this sup pattern, sub is unconstrained (Top)
        // for these properties. Top <: sup_schema only if sup_schema is also Top.
        if !covered && !matches!(sup_schema, ReducedSchema::Top(_)) {
            return Ok(not_subtype(&format!(
                "sup constrains pattern {:?} but sub has no matching constraint",
                sup_pat.value
            )));
        }
    }

    Ok(SubtypeRelation::Subtype)
}

/// Compile a property name pattern for regex algebra operations.
/// Property patterns from canonicalization are already anchored (^name$).
/// User-defined patternProperties use unanchored ECMA-262 patterns,
/// so we wrap with .*..*.
fn compile_property_pattern(
    pattern: &str,
) -> Result<regex_algebra::CompiledRegex, SubtypeError> {
    // If pattern starts with ^ and ends with $, it's already anchored.
    // Otherwise wrap for unanchored matching.
    let effective = if pattern.starts_with('^') && pattern.ends_with('$') {
        pattern.to_string()
    } else {
        format!(".*(?:{pattern}).*")
    };
    regex_algebra::compile(&effective).map_err(|e| SubtypeError::UnsupportedFeatures {
        details: format!("property pattern compilation failed: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::{Located, Provenance};

    fn make_object(
        min: u64,
        max: u64,
        required: &[&str],
        patterns: Vec<(&str, ReducedSchema)>,
    ) -> ObjectSchema {
        ObjectSchema {
            provenance: Provenance::synthetic(),
            min_properties: Located::synthetic(min),
            max_properties: Located::synthetic(max),
            required: required
                .iter()
                .map(|s| Located::synthetic(s.to_string()))
                .collect(),
            pattern_properties: patterns
                .into_iter()
                .map(|(p, s)| (Located::synthetic(p.to_string()), s))
                .collect(),
        }
    }

    fn string_schema() -> ReducedSchema {
        ReducedSchema::Typed(Box::new(TypedSchema::String(StringSchema {
            provenance: Provenance::synthetic(),
            pattern: Located::synthetic(String::new()),
            min_length: None,
            max_length: None,
        })))
    }

    fn number_schema() -> ReducedSchema {
        ReducedSchema::Typed(Box::new(TypedSchema::Number(NumberSchema {
            provenance: Provenance::synthetic(),
            minimum: Located::synthetic(f64::NEG_INFINITY),
            maximum: Located::synthetic(f64::INFINITY),
            exclusive_minimum: Located::synthetic(f64::NEG_INFINITY),
            exclusive_maximum: Located::synthetic(f64::INFINITY),
            multiple_of: None,
        })))
    }

    #[test]
    fn required_superset() {
        let sub = make_object(0, u64::MAX, &["a", "b"], vec![]);
        let sup = make_object(0, u64::MAX, &["a"], vec![]);
        assert!(check_object_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn required_not_superset() {
        let sub = make_object(0, u64::MAX, &["a"], vec![]);
        let sup = make_object(0, u64::MAX, &["a", "b"], vec![]);
        assert!(!check_object_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn min_max_properties() {
        let sub = make_object(2, 5, &[], vec![]);
        let sup = make_object(1, 10, &[], vec![]);
        assert!(check_object_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn pattern_properties_subtype() {
        let sub = make_object(0, u64::MAX, &[], vec![("^name$", string_schema())]);
        let sup = make_object(0, u64::MAX, &[], vec![("^name$", string_schema())]);
        assert!(check_object_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn pattern_properties_not_subtype() {
        let sub = make_object(0, u64::MAX, &[], vec![("^name$", string_schema())]);
        let sup = make_object(0, u64::MAX, &[], vec![("^name$", number_schema())]);
        assert!(!check_object_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn non_overlapping_patterns_not_subtype() {
        // Sub has ^name$: string, sup has ^age$: number — sub doesn't constrain "age"
        // so objects like {age: "hello"} pass sub but fail sup
        let sub = make_object(0, u64::MAX, &[], vec![("^name$", string_schema())]);
        let sup = make_object(0, u64::MAX, &[], vec![("^age$", number_schema())]);
        assert!(!check_object_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn non_overlapping_patterns_with_top_sup() {
        // If sup constrains a pattern to Top, sub doesn't need to match it
        let sub = make_object(0, u64::MAX, &[], vec![("^name$", string_schema())]);
        let sup = make_object(
            0,
            u64::MAX,
            &[],
            vec![("^age$", ReducedSchema::Top(Provenance::synthetic()))],
        );
        assert!(check_object_subtype(&sub, &sup).unwrap().is_subtype());
    }
}
