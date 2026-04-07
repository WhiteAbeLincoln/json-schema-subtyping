use crate::error::{SubtypeError, SubtypeRelation};
use crate::schema::*;
use super::{is_subtype, not_subtype};

pub fn check_array_subtype(
    sub: &ArraySchema,
    sup: &ArraySchema,
) -> Result<SubtypeRelation, SubtypeError> {
    // 1. minItems/maxItems range containment
    if sub.min_items.value < sup.min_items.value {
        return Ok(not_subtype("sub minItems < sup minItems"));
    }
    if sub.max_items.value > sup.max_items.value {
        return Ok(not_subtype("sub maxItems > sup maxItems"));
    }

    // 2. uniqueItems: if sup requires uniqueItems, sub must also
    if sup.unique_items.value && !sub.unique_items.value {
        return Ok(not_subtype("sup requires uniqueItems but sub does not"));
    }

    // 3. Per-position item subtype check
    // Check positions up to max(len(sub.prefix), len(sup.prefix)),
    // then check the items (tail) schema.
    let max_prefix = sub.prefix_items.len().max(sup.prefix_items.len());
    for i in 0..max_prefix {
        let sub_schema = sub.prefix_items.get(i).unwrap_or(&sub.items);
        let sup_schema = sup.prefix_items.get(i).unwrap_or(&sup.items);
        if !is_subtype(sub_schema, sup_schema)?.is_subtype() {
            return Ok(not_subtype(&format!(
                "item at position {i} is not a subtype"
            )));
        }
    }

    // 4. Tail items: sub.items <: sup.items
    if !is_subtype(&sub.items, &sup.items)?.is_subtype() {
        return Ok(not_subtype("sub items schema is not a subtype of sup items"));
    }

    Ok(SubtypeRelation::Subtype)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::{Located, Provenance};

    fn make_array(
        min: u64,
        max: u64,
        prefix: Vec<ReducedSchema>,
        items: ReducedSchema,
        unique: bool,
    ) -> ArraySchema {
        ArraySchema {
            provenance: Provenance::synthetic(),
            min_items: Located::synthetic(min),
            max_items: Located::synthetic(max),
            prefix_items: prefix,
            items: Box::new(items),
            unique_items: Located::synthetic(unique),
        }
    }

    fn top() -> ReducedSchema {
        ReducedSchema::Top(Provenance::synthetic())
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
    fn min_max_items_subtype() {
        let sub = make_array(2, 5, vec![], top(), false);
        let sup = make_array(1, 10, vec![], top(), false);
        assert!(check_array_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn min_max_items_not_subtype() {
        let sub = make_array(1, 10, vec![], top(), false);
        let sup = make_array(2, 5, vec![], top(), false);
        assert!(!check_array_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn unique_items_subtype() {
        let sub = make_array(0, u64::MAX, vec![], top(), true);
        let sup = make_array(0, u64::MAX, vec![], top(), true);
        assert!(check_array_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn unique_items_not_subtype() {
        let sub = make_array(0, u64::MAX, vec![], top(), false);
        let sup = make_array(0, u64::MAX, vec![], top(), true);
        assert!(!check_array_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn prefix_items_subtype() {
        let sub = make_array(0, u64::MAX, vec![string_schema()], top(), false);
        let sup = make_array(0, u64::MAX, vec![top()], top(), false);
        assert!(check_array_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn items_subtype() {
        let sub = make_array(0, u64::MAX, vec![], number_schema(), false);
        let sup = make_array(0, u64::MAX, vec![], top(), false);
        assert!(check_array_subtype(&sub, &sup).unwrap().is_subtype());
    }

    #[test]
    fn items_not_subtype() {
        let sub = make_array(0, u64::MAX, vec![], top(), false);
        let sup = make_array(0, u64::MAX, vec![], number_schema(), false);
        assert!(!check_array_subtype(&sub, &sup).unwrap().is_subtype());
    }
}
