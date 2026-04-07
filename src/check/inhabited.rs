use crate::error::SubtypeError;
use crate::schema::*;

/// Check if a schema is uninhabited (matches no values).
/// Conservative: returns false when unsure (may miss some uninhabited schemas).
pub fn is_uninhabited(schema: &ReducedSchema) -> Result<bool, SubtypeError> {
    match schema {
        ReducedSchema::Bottom(_) => Ok(true),
        ReducedSchema::Top(_) => Ok(false),
        ReducedSchema::Typed(t) => is_typed_uninhabited(t),
        ReducedSchema::AnyOf(_, branches) => {
            for branch in branches {
                if !is_uninhabited(branch)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ReducedSchema::AllOf(_, branches) => {
            for branch in branches {
                if is_uninhabited(branch)? {
                    return Ok(true);
                }
            }
            // Conservative: could still be uninhabited due to interaction
            Ok(false)
        }
        ReducedSchema::Not(_, inner) => {
            Ok(matches!(inner.as_ref(), ReducedSchema::Top(_)))
        }
    }
}

fn is_typed_uninhabited(schema: &TypedSchema) -> Result<bool, SubtypeError> {
    match schema {
        TypedSchema::Null(_) => Ok(false),
        TypedSchema::Boolean(b) => Ok(b.enum_values.value.is_empty()),
        TypedSchema::String(s) => {
            let re = compile_string_pattern(&s.pattern.value)?;
            // If there are residual length constraints, intersect
            let effective = apply_length_constraints(re, &s.min_length, &s.max_length)?;
            Ok(regex_algebra::is_empty(&effective))
        }
        TypedSchema::Number(n) => {
            let effective_min = n.minimum.value.max(n.exclusive_minimum.value);
            let effective_max = n.maximum.value.min(n.exclusive_maximum.value);
            Ok(effective_min > effective_max)
        }
        TypedSchema::Array(a) => Ok(a.min_items.value > a.max_items.value),
        TypedSchema::Object(o) => {
            Ok(o.min_properties.value > o.max_properties.value
                || o.required.len() as u64 > o.max_properties.value)
        }
    }
}

use crate::located::Located;

/// Compile a string pattern for set-algebraic operations.
/// Empty pattern means "match everything". Non-empty patterns are wrapped
/// with `.*...*` to simulate JSON Schema's unanchored matching semantics.
pub(crate) fn compile_string_pattern(
    pattern: &str,
) -> Result<regex_algebra::CompiledRegex, SubtypeError> {
    let effective = if pattern.is_empty() {
        ".*".to_string()
    } else {
        format!(".*(?:{pattern}).*")
    };
    regex_algebra::compile(&effective).map_err(|e| SubtypeError::UnsupportedFeatures {
        details: format!("regex compilation failed: {e}"),
    })
}

/// Apply residual min/max length constraints by intersecting with the length DFA.
pub(crate) fn apply_length_constraints(
    re: regex_algebra::CompiledRegex,
    min_length: &Option<Located<u64>>,
    max_length: &Option<Located<u64>>,
) -> Result<regex_algebra::CompiledRegex, SubtypeError> {
    match (min_length, max_length) {
        (None, None) => Ok(re),
        _ => {
            let min = min_length.as_ref().map(|l| l.value).unwrap_or(0);
            let max = max_length.as_ref().map(|l| l.value);
            let length_pat = regex_algebra::length_pattern(min, max);
            let length_re = regex_algebra::compile(&length_pat).map_err(|e| {
                SubtypeError::UnsupportedFeatures {
                    details: format!("length pattern compilation failed: {e}"),
                }
            })?;
            Ok(regex_algebra::intersect(&re, &length_re))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::Provenance;
    use crate::schema::BoolSet;

    #[test]
    fn bottom_is_uninhabited() {
        assert!(is_uninhabited(&ReducedSchema::Bottom(Provenance::synthetic())).unwrap());
    }

    #[test]
    fn top_is_inhabited() {
        assert!(!is_uninhabited(&ReducedSchema::Top(Provenance::synthetic())).unwrap());
    }

    #[test]
    fn contradictory_number_is_uninhabited() {
        let s = ReducedSchema::Typed(Box::new(TypedSchema::Number(NumberSchema {
            provenance: Provenance::synthetic(),
            minimum: Located::synthetic(10.0),
            maximum: Located::synthetic(5.0),
            exclusive_minimum: Located::synthetic(f64::NEG_INFINITY),
            exclusive_maximum: Located::synthetic(f64::INFINITY),
            multiple_of: None,
        })));
        assert!(is_uninhabited(&s).unwrap());
    }

    #[test]
    fn empty_boolset_is_uninhabited() {
        let s = ReducedSchema::Typed(Box::new(TypedSchema::Boolean(BooleanSchema {
            provenance: Provenance::synthetic(),
            enum_values: Located::synthetic(BoolSet::Neither),
        })));
        assert!(is_uninhabited(&s).unwrap());
    }

    #[test]
    fn empty_string_pattern_is_uninhabited() {
        // Pattern that matches nothing: intersection of disjoint patterns
        let s = ReducedSchema::Typed(Box::new(TypedSchema::String(StringSchema {
            provenance: Provenance::synthetic(),
            pattern: Located::synthetic("^a$".to_string()),
            min_length: Some(Located::synthetic(5)),
            max_length: None,
        })));
        // "^a$" matches only "a" (length 1), but minLength is 5 → empty intersection
        assert!(is_uninhabited(&s).unwrap());
    }

    #[test]
    fn anyof_all_uninhabited() {
        let s = ReducedSchema::AnyOf(Provenance::synthetic(), vec![
            ReducedSchema::Bottom(Provenance::synthetic()),
            ReducedSchema::Bottom(Provenance::synthetic()),
        ]);
        assert!(is_uninhabited(&s).unwrap());
    }

    #[test]
    fn anyof_some_inhabited() {
        let s = ReducedSchema::AnyOf(Provenance::synthetic(), vec![
            ReducedSchema::Bottom(Provenance::synthetic()),
            ReducedSchema::Top(Provenance::synthetic()),
        ]);
        assert!(!is_uninhabited(&s).unwrap());
    }

    #[test]
    fn impossible_array_is_uninhabited() {
        let s = ReducedSchema::Typed(Box::new(TypedSchema::Array(ArraySchema {
            provenance: Provenance::synthetic(),
            min_items: Located::synthetic(10),
            max_items: Located::synthetic(5),
            prefix_items: vec![],
            items: Box::new(ReducedSchema::Top(Provenance::synthetic())),
            unique_items: Located::synthetic(false),
        })));
        assert!(is_uninhabited(&s).unwrap());
    }
}
