// Translated from: IBM/jsonsubschema test/test_numeric.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0
//
// Draft-4 boolean exclusiveMinimum/exclusiveMaximum have been converted to
// 2020-12 numeric values.

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

// ============================================================
// TestIntegerSubtype
// ============================================================

#[test]
fn int_identity() {
    assert!(is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn int_min_min_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "minimum": 5}"#,
        r#"{"type": "integer", "minimum": 1}"#
    ));
}

#[test]
fn int_min_min_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 1}"#,
        r#"{"type": "integer", "minimum": 5}"#
    ));
}

#[test]
fn int_max_max_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "maximum": 10}"#,
        r#"{"type": "integer", "maximum": 5}"#
    ));
}

#[test]
fn int_max_max_rev() {
    assert!(is_sub(
        r#"{"type": "integer", "maximum": 5}"#,
        r#"{"type": "integer", "maximum": 10}"#
    ));
}

#[test]
fn int_max_min_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "maximum": 10}"#,
        r#"{"type": "integer", "minimum": 5}"#
    ));
}

#[test]
fn int_max_min_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5}"#,
        r#"{"type": "integer", "maximum": 10}"#
    ));
}

#[test]
fn int_min_max_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 10}"#,
        r#"{"type": "integer", "maximum": 20}"#
    ));
}

#[test]
fn int_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "maximum": 20}"#,
        r#"{"type": "integer", "minimum": 10}"#
    ));
}

#[test]
fn int_min_max_min_max1_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "minimum": 5, "maximum": 10}"#,
        r#"{"type": "integer", "minimum": 1, "maximum": 20}"#
    ));
}

#[test]
fn int_min_max_min_max1_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 1, "maximum": 20}"#,
        r#"{"type": "integer", "minimum": 5, "maximum": 10}"#
    ));
}

#[test]
fn int_min_max_min_max2_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5, "maximum": 20}"#,
        r#"{"type": "integer", "minimum": 10, "maximum": 20}"#
    ));
}

#[test]
fn int_min_max_min_max2_rev() {
    assert!(is_sub(
        r#"{"type": "integer", "minimum": 10, "maximum": 20}"#,
        r#"{"type": "integer", "minimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn int_min_max_min_max3_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5, "maximum": 20}"#,
        r#"{"type": "integer", "minimum": 40, "maximum": 100}"#
    ));
}

#[test]
fn int_min_max_min_max3_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 40, "maximum": 100}"#,
        r#"{"type": "integer", "minimum": 5, "maximum": 20}"#
    ));
}

// Draft-4: exclusiveMinimum: true on minimum: 5 → 2020-12: exclusiveMinimum: 5
#[test]
fn int_xmin_max_min_max_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "exclusiveMinimum": 5, "maximum": 20}"#,
        r#"{"type": "integer", "minimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn int_xmin_max_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5, "maximum": 20}"#,
        r#"{"type": "integer", "exclusiveMinimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn int_xmin_max_min_xmax_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "exclusiveMinimum": 5, "maximum": 20}"#,
        r#"{"type": "integer", "minimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn int_xmin_max_min_xmax_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "integer", "exclusiveMinimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn int_xmin_xmax_min_max_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "integer", "minimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn int_xmin_xmax_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5, "maximum": 20}"#,
        r#"{"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#
    ));
}

// Draft-4: excl(5,20) equiv [6,19]. 2020-12: excl(5,20) equiv [6,19] for integers.
#[test]
fn int_min_max_xmin_xmax1_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "integer", "minimum": 6, "maximum": 19}"#
    ));
}

#[test]
fn int_min_max_xmin_xmax1_rev() {
    assert!(is_sub(
        r#"{"type": "integer", "minimum": 6, "maximum": 19}"#,
        r#"{"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn int_min_max_xmin_xmax2_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "integer", "minimum": 6, "maximum": 20}"#
    ));
}

#[test]
fn int_min_max_xmin_xmax2_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 6, "maximum": 20}"#,
        r#"{"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#
    ));
}

// Draft-4: exclusiveMinimum: false → just minimum. exclusiveMaximum: true → exclusive.
// 2020-12: minimum: 5 (inclusive), exclusiveMaximum: 20.
#[test]
fn int_xmin_xmax_xmin_xmax_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn int_xmin_xmax_xmin_xmax_rev() {
    assert!(is_sub(
        r#"{"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "integer", "minimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn int_mulof1_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "multipleOf": 10}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn int_mulof1_rev() {
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "integer", "multipleOf": 10}"#
    ));
}

#[test]
fn int_mulof2_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "multipleOf": 10}"#,
        r#"{"type": "integer", "multipleOf": 5}"#
    ));
}

#[test]
fn int_mulof2_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 5}"#,
        r#"{"type": "integer", "multipleOf": 10}"#
    ));
}

#[test]
fn int_mulof3_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 10}"#,
        r#"{"type": "integer", "multipleOf": 98}"#
    ));
}

#[test]
fn int_mulof3_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 98}"#,
        r#"{"type": "integer", "multipleOf": 10}"#
    ));
}

#[test]
fn int_mulof_min_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 10}"#,
        r#"{"type": "integer", "minimum": 5}"#
    ));
}

#[test]
fn int_mulof_min_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5}"#,
        r#"{"type": "integer", "multipleOf": 10}"#
    ));
}

#[test]
fn int_mulof_min_min_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "multipleOf": 10, "minimum": 10}"#,
        r#"{"type": "integer", "minimum": 5}"#
    ));
}

#[test]
fn int_mulof_min_min_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5}"#,
        r#"{"type": "integer", "multipleOf": 10, "minimum": 10}"#
    ));
}

#[test]
fn int_mulof_min_min_max_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 10, "minimum": 10}"#,
        r#"{"type": "integer", "minimum": 5, "maximum": 500}"#
    ));
}

#[test]
fn int_mulof_min_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5, "maximum": 500}"#,
        r#"{"type": "integer", "multipleOf": 10, "minimum": 10}"#
    ));
}

#[test]
fn int_min_max_mul_fwd() {
    // [5,10] ∩ multipleOf:15 is empty (no multiple of 15 in [5,10])
    assert!(is_sub(
        r#"{"type": "integer", "minimum": 5, "maximum": 10, "multipleOf": 15}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn int_min_max_mul_rev() {
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "integer", "minimum": 5, "maximum": 10, "multipleOf": 15}"#
    ));
}

// --- anyOf (join) tests ---

#[test]
fn int_join1_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 10}, {"type": "integer"}]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn int_join1_rev() {
    assert!(is_sub(
        r#"{"type": "integer"}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 10}, {"type": "integer"}]}"#
    ));
}

#[test]
fn int_join2_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 10}, {"type": "integer", "minimum": 0}]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn int_join2_rev() {
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 10}, {"type": "integer", "minimum": 0}]}"#
    ));
}

#[test]
fn int_join3_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 10}, {"type": "integer", "minimum": 0, "maximum": 3}]}"#,
        r#"{"type": "integer", "minimum": -1}"#
    ));
}

#[test]
fn int_join3_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": -1}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 10}, {"type": "integer", "minimum": 0, "maximum": 3}]}"#
    ));
}

#[test]
fn int_join4_fwd() {
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 10}, {"type": "integer", "minimum": 0, "maximum": 4}]}"#,
        r#"{"type": "integer", "minimum": 1, "maximum": 8}"#
    ));
}

#[test]
fn int_join4_rev() {
    assert!(is_sub(
        r#"{"type": "integer", "minimum": 1, "maximum": 8}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 10}, {"type": "integer", "minimum": 0, "maximum": 4}]}"#
    ));
}

#[test]
fn int_join5_fwd() {
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer", "exclusiveMinimum": 5, "maximum": 10}, {"type": "integer", "minimum": 0, "maximum": 4}]}"#,
        r#"{"type": "integer", "minimum": 1, "maximum": 8}"#
    ));
}

#[test]
fn int_join5_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 1, "maximum": 8}"#,
        r#"{"anyOf": [{"type": "integer", "exclusiveMinimum": 5, "maximum": 10}, {"type": "integer", "minimum": 0, "maximum": 4}]}"#
    ));
}

// join6: anyOf([0,10], [11,∞]) equiv [0,∞)
// requires non-trivial anyOf subtype (enumerative or join)
#[test]
fn int_join6_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 0, "maximum": 10}, {"type": "integer", "minimum": 11}]}"#,
        r#"{"type": "integer", "minimum": 0}"#
    ));
}

#[test]
fn int_join6_rev() {
    // requires non-trivial anyOf subtype
    assert!(is_sub(
        r#"{"type": "integer", "minimum": 0}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 0, "maximum": 10}, {"type": "integer", "minimum": 11}]}"#
    ));
}

#[test]
fn int_join_mulof1_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "multipleOf": 5}, {"type": "integer"}]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn int_join_mulof1_rev() {
    assert!(is_sub(
        r#"{"type": "integer"}"#,
        r#"{"anyOf": [{"type": "integer", "multipleOf": 5}, {"type": "integer"}]}"#
    ));
}

#[test]
fn int_join_mulof2_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "multipleOf": 5}, {"type": "integer", "multipleOf": 7}]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn int_join_mulof2_rev() {
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"anyOf": [{"type": "integer", "multipleOf": 5}, {"type": "integer", "multipleOf": 7}]}"#
    ));
}

#[test]
fn int_join_mulof3_fwd() {
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer", "multipleOf": 5}, {"type": "integer", "multipleOf": 7}]}"#,
        r#"{"type": "integer", "multipleOf": 35}"#
    ));
}

#[test]
fn int_join_mulof3_rev() {
    assert!(is_sub(
        r#"{"type": "integer", "multipleOf": 35}"#,
        r#"{"anyOf": [{"type": "integer", "multipleOf": 5}, {"type": "integer", "multipleOf": 7}]}"#
    ));
}

#[test]
fn int_join_mulof4_fwd() {
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer", "multipleOf": 5}, {"type": "integer", "multipleOf": 7}]}"#,
        r#"{"type": "integer", "multipleOf": 5}"#
    ));
}

#[test]
fn int_join_mulof4_rev() {
    assert!(is_sub(
        r#"{"type": "integer", "multipleOf": 5}"#,
        r#"{"anyOf": [{"type": "integer", "multipleOf": 5}, {"type": "integer", "multipleOf": 7}]}"#
    ));
}

#[test]
fn int_join_mulof5_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "multipleOf": 3}, {"type": "integer", "multipleOf": 6}]}"#,
        r#"{"type": "integer", "multipleOf": 3}"#
    ));
}

#[test]
fn int_join_mulof5_rev() {
    assert!(is_sub(
        r#"{"type": "integer", "multipleOf": 3}"#,
        r#"{"anyOf": [{"type": "integer", "multipleOf": 3}, {"type": "integer", "multipleOf": 6}]}"#
    ));
}

#[test]
fn int_join_mulof6_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "multipleOf": 12}, {"type": "integer", "multipleOf": 9}]}"#,
        r#"{"type": "integer", "multipleOf": 3}"#
    ));
}

#[test]
fn int_join_mulof6_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 3}"#,
        r#"{"anyOf": [{"type": "integer", "multipleOf": 12}, {"type": "integer", "multipleOf": 9}]}"#
    ));
}

#[test]
fn int_join_mulof7_fwd() {
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer", "multipleOf": 3, "maximum": 10}, {"type": "integer", "multipleOf": 5}]}"#,
        r#"{"type": "integer", "multipleOf": 3}"#
    ));
}

#[test]
fn int_join_mulof7_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 3}"#,
        r#"{"anyOf": [{"type": "integer", "multipleOf": 3, "maximum": 10}, {"type": "integer", "multipleOf": 5}]}"#
    ));
}

// join_mulof8: requires non-trivial anyOf subtype (enumerative)
#[test]
fn int_join_mulof8_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 15, "multipleOf": 5}, {"type": "integer", "minimum": 5, "maximum": 15, "multipleOf": 3}]}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 0, "maximum": 12, "multipleOf": 3}, {"type": "integer", "minimum": 1, "maximum": 20, "multipleOf": 5}]}"#
    ));
}

#[test]
fn int_join_mulof8_rev() {
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 0, "maximum": 12, "multipleOf": 3}, {"type": "integer", "minimum": 1, "maximum": 20, "multipleOf": 5}]}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 5, "maximum": 15, "multipleOf": 5}, {"type": "integer", "minimum": 5, "maximum": 15, "multipleOf": 3}]}"#
    ));
}

// join_mulof9: requires non-trivial anyOf subtype (enumerative)
#[test]
fn int_join_mulof9_fwd() {
    assert!(is_sub(
        r#"{"type": "integer", "minimum": -4, "maximum": 10, "multipleOf": 5}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 0, "maximum": 20, "multipleOf": 10}, {"type": "integer", "minimum": 1, "maximum": 10, "multipleOf": 5}]}"#
    ));
}

#[test]
fn int_join_mulof9_rev() {
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 0, "maximum": 20, "multipleOf": 10}, {"type": "integer", "minimum": 1, "maximum": 10, "multipleOf": 5}]}"#,
        r#"{"type": "integer", "minimum": -4, "maximum": 10, "multipleOf": 5}"#
    ));
}

// join_mulof10: enum + anyOf
#[test]
fn int_join_mulof10_fwd() {
    assert!(is_sub(
        r#"{"enum": [1, 3, 5, 7, 9, 10]}"#,
        r#"{"anyOf": [{"type": "integer", "minimum": 0, "maximum": 20, "multipleOf": 10}, {"type": "integer", "minimum": 1, "maximum": 10, "multipleOf": 5}, {"enum": [1, 3, 7, 9]}]}"#
    ));
}

#[test]
fn int_join_mulof10_rev() {
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer", "minimum": 0, "maximum": 20, "multipleOf": 10}, {"type": "integer", "minimum": 1, "maximum": 10, "multipleOf": 5}, {"enum": [1, 3, 7, 9]}]}"#,
        r#"{"enum": [1, 3, 5, 7, 9, 10]}"#
    ));
}

// ============================================================
// TestNumberSubtype
// ============================================================

#[test]
fn num_identity() {
    assert!(is_sub(
        r#"{"type": "number"}"#,
        r#"{"type": "number"}"#
    ));
}

#[test]
fn num_min_min_fwd() {
    assert!(is_sub(
        r#"{"type": "number", "minimum": 5}"#,
        r#"{"type": "number", "minimum": 1}"#
    ));
}

#[test]
fn num_min_min_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 1}"#,
        r#"{"type": "number", "minimum": 5}"#
    ));
}

#[test]
fn num_max_max_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "maximum": 10}"#,
        r#"{"type": "number", "maximum": 5}"#
    ));
}

#[test]
fn num_max_max_rev() {
    assert!(is_sub(
        r#"{"type": "number", "maximum": 5}"#,
        r#"{"type": "number", "maximum": 10}"#
    ));
}

#[test]
fn num_max_min_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "maximum": 10}"#,
        r#"{"type": "number", "minimum": 5}"#
    ));
}

#[test]
fn num_max_min_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5}"#,
        r#"{"type": "number", "maximum": 10}"#
    ));
}

#[test]
fn num_min_max_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 10}"#,
        r#"{"type": "number", "maximum": 20}"#
    ));
}

#[test]
fn num_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "maximum": 20}"#,
        r#"{"type": "number", "minimum": 10}"#
    ));
}

#[test]
fn num_min_max_min_max1_fwd() {
    assert!(is_sub(
        r#"{"type": "number", "minimum": 5, "maximum": 10}"#,
        r#"{"type": "number", "minimum": 1, "maximum": 20}"#
    ));
}

#[test]
fn num_min_max_min_max1_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 1, "maximum": 20}"#,
        r#"{"type": "number", "minimum": 5, "maximum": 10}"#
    ));
}

#[test]
fn num_min_max_min_max2_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5, "maximum": 20}"#,
        r#"{"type": "number", "minimum": 10, "maximum": 20}"#
    ));
}

#[test]
fn num_min_max_min_max2_rev() {
    assert!(is_sub(
        r#"{"type": "number", "minimum": 10, "maximum": 20}"#,
        r#"{"type": "number", "minimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn num_min_max_min_max3_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5, "maximum": 20}"#,
        r#"{"type": "number", "minimum": 40, "maximum": 100}"#
    ));
}

#[test]
fn num_min_max_min_max3_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 40, "maximum": 100}"#,
        r#"{"type": "number", "minimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn num_xmin_max_min_max_fwd() {
    assert!(is_sub(
        r#"{"type": "number", "exclusiveMinimum": 5, "maximum": 20}"#,
        r#"{"type": "number", "minimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn num_xmin_max_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5, "maximum": 20}"#,
        r#"{"type": "number", "exclusiveMinimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn num_xmin_max_min_xmax_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "exclusiveMinimum": 5, "maximum": 20}"#,
        r#"{"type": "number", "minimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn num_xmin_max_min_xmax_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "number", "exclusiveMinimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn num_xmin_xmax_min_max_fwd() {
    assert!(is_sub(
        r#"{"type": "number", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "number", "minimum": 5, "maximum": 20}"#
    ));
}

#[test]
fn num_xmin_xmax_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5, "maximum": 20}"#,
        r#"{"type": "number", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#
    ));
}

// For numbers (non-integer), excl(5,20) does NOT equal [6,19]
#[test]
fn num_min_max_xmin_xmax1_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "number", "minimum": 6, "maximum": 19}"#
    ));
}

#[test]
fn num_min_max_xmin_xmax1_rev() {
    assert!(is_sub(
        r#"{"type": "number", "minimum": 6, "maximum": 19}"#,
        r#"{"type": "number", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn num_min_max_xmin_xmax2_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "number", "minimum": 6, "maximum": 20}"#
    ));
}

#[test]
fn num_min_max_xmin_xmax2_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 6, "maximum": 20}"#,
        r#"{"type": "number", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn num_xmin_xmax_xmin_xmax_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "number", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn num_xmin_xmax_xmin_xmax_rev() {
    assert!(is_sub(
        r#"{"type": "number", "exclusiveMinimum": 5, "exclusiveMaximum": 20}"#,
        r#"{"type": "number", "minimum": 5, "exclusiveMaximum": 20}"#
    ));
}

#[test]
fn num_mulof1_fwd() {
    assert!(is_sub(
        r#"{"type": "number", "multipleOf": 10.5}"#,
        r#"{"type": "number"}"#
    ));
}

#[test]
fn num_mulof1_rev() {
    assert!(!is_sub(
        r#"{"type": "number"}"#,
        r#"{"type": "number", "multipleOf": 10.5}"#
    ));
}

#[test]
fn num_mulof2_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "multipleOf": 1.5}"#,
        r#"{"type": "number", "multipleOf": 6}"#
    ));
}

#[test]
fn num_mulof2_rev() {
    assert!(is_sub(
        r#"{"type": "number", "multipleOf": 6}"#,
        r#"{"type": "number", "multipleOf": 1.5}"#
    ));
}

#[test]
fn num_mulof4_fwd() {
    assert!(is_sub(
        r#"{"type": "number", "multipleOf": 1}"#,
        r#"{"type": "number"}"#
    ));
}

#[test]
fn num_mulof4_rev() {
    assert!(!is_sub(
        r#"{"type": "number"}"#,
        r#"{"type": "number", "multipleOf": 1}"#
    ));
}

#[test]
fn num_mulof_min_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "multipleOf": 10}"#,
        r#"{"type": "number", "minimum": 5}"#
    ));
}

#[test]
fn num_mulof_min_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5}"#,
        r#"{"type": "number", "multipleOf": 10}"#
    ));
}

#[test]
fn num_mulof_min_min_fwd() {
    assert!(is_sub(
        r#"{"type": "number", "multipleOf": 10, "minimum": 10}"#,
        r#"{"type": "number", "minimum": 5}"#
    ));
}

#[test]
fn num_mulof_min_min_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5}"#,
        r#"{"type": "number", "multipleOf": 10, "minimum": 10}"#
    ));
}

#[test]
fn num_mulof_min_min_max_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "multipleOf": 10, "minimum": 10}"#,
        r#"{"type": "number", "minimum": 5, "maximum": 500}"#
    ));
}

#[test]
fn num_mulof_min_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 5, "maximum": 500}"#,
        r#"{"type": "number", "multipleOf": 10, "minimum": 10}"#
    ));
}

// ============================================================
// TestNumericSubtype (cross-type integer/number)
// ============================================================

#[test]
fn int_sub_num_fwd() {
    assert!(is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "number"}"#
    ));
}

#[test]
fn int_sub_num_rev() {
    assert!(!is_sub(
        r#"{"type": "number"}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn num_min_int_min_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "minimum": 1.5}"#,
        r#"{"type": "integer", "minimum": 1}"#
    ));
}

#[test]
fn num_min_int_min_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 1}"#,
        r#"{"type": "number", "minimum": 1.5}"#
    ));
}

#[test]
fn mulof_num_min_int_fwd() {
    assert!(!is_sub(
        r#"{"type": "number", "multipleOf": 10}"#,
        r#"{"type": "integer", "minimum": 5}"#
    ));
}

#[test]
fn mulof_num_min_int_rev() {
    assert!(!is_sub(
        r#"{"type": "integer", "minimum": 5}"#,
        r#"{"type": "number", "multipleOf": 10}"#
    ));
}

#[test]
fn mulof_num_int_fwd() {
    // number with multipleOf:10 → always integer → subtype of integer
    assert!(is_sub(
        r#"{"type": "number", "multipleOf": 10}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn mulof_num_int_rev() {
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "number", "multipleOf": 10}"#
    ));
}

#[test]
fn mulof_num_int2_fwd() {
    // number with multipleOf:1 equiv integer
    assert!(is_sub(
        r#"{"type": "number", "multipleOf": 1}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn mulof_num_int2_rev() {
    assert!(is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "number", "multipleOf": 1}"#
    ));
}

#[test]
fn decimal1_fwd() {
    // {maximum: 10.0} equiv {maximum: 10}
    assert!(is_sub(
        r#"{"maximum": 10.0}"#,
        r#"{"maximum": 10}"#
    ));
}

#[test]
fn decimal1_rev() {
    assert!(is_sub(
        r#"{"maximum": 10}"#,
        r#"{"maximum": 10.0}"#
    ));
}

// ============================================================
// TestCompositeNumericSubtype
// ============================================================

#[test]
fn int_int_num1_fwd() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "number", "allOf": [{"type": "integer"}, {"type": "number", "minimum": 10}]}"#
    ));
}

#[test]
fn int_int_num1_rev() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"type": "number", "allOf": [{"type": "integer"}, {"type": "number", "minimum": 10}]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn int_int_num2_fwd() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 5}"#,
        r#"{"type": "number", "allOf": [{"type": "integer"}, {"type": "number", "minimum": 10}]}"#
    ));
}

#[test]
fn int_int_num2_rev() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "number", "allOf": [{"type": "integer"}, {"type": "number", "minimum": 10}]}"#,
        r#"{"type": "integer", "multipleOf": 5}"#
    ));
}

#[test]
fn int_mul_mul1_fwd() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "integer", "multipleOf": 5}"#,
        r#"{"type": "number", "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 3}]}"#
    ));
}

#[test]
fn int_mul_mul1_rev() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "number", "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 3}]}"#,
        r#"{"type": "integer", "multipleOf": 5}"#
    ));
}

#[test]
fn int_mul_mul2_fwd() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"type": "integer", "multipleOf": 15}"#,
        r#"{"type": "number", "multipleOf": 3, "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 5}]}"#
    ));
}

#[test]
fn int_mul_mul2_rev() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"type": "number", "multipleOf": 3, "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 5}]}"#,
        r#"{"type": "integer", "multipleOf": 15}"#
    ));
}

#[test]
fn all_all_1_fwd() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "integer", "allOf": [{"multipleOf": 3}, {"minimum": 5}]}"#,
        r#"{"type": "number", "multipleOf": 3, "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 5}]}"#
    ));
}

#[test]
fn all_all_1_rev() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "number", "multipleOf": 3, "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 5}]}"#,
        r#"{"type": "integer", "allOf": [{"multipleOf": 3}, {"minimum": 5}]}"#
    ));
}

#[test]
fn all_all_2_fwd() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"type": "integer", "allOf": [{"multipleOf": 3}]}"#,
        r#"{"type": "number", "multipleOf": 3, "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 3}]}"#
    ));
}

#[test]
fn all_all_2_rev() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"type": "number", "multipleOf": 3, "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 3}]}"#,
        r#"{"type": "integer", "allOf": [{"multipleOf": 3}]}"#
    ));
}

#[test]
fn all_all_3_fwd() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "number", "allOf": [{"multipleOf": 0.3}]}"#,
        r#"{"type": "number", "multipleOf": 3, "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 3}]}"#
    ));
}

#[test]
fn all_all_3_rev() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "number", "multipleOf": 3, "allOf": [{"type": "integer"}, {"type": "number", "multipleOf": 3}]}"#,
        r#"{"type": "number", "allOf": [{"multipleOf": 0.3}]}"#
    ));
}

#[test]
fn num_enum1_fwd() {
    assert!(is_sub(
        r#"{"enum": [1, 2, 3]}"#,
        r#"{"type": "number"}"#
    ));
}

#[test]
fn num_enum1_rev() {
    assert!(!is_sub(
        r#"{"type": "number"}"#,
        r#"{"enum": [1, 2, 3]}"#
    ));
}

#[test]
fn num_enum2_fwd() {
    assert!(!is_sub(
        r#"{"enum": [1.0, 2, 3]}"#,
        r#"{"enum": [1, 2.0]}"#
    ));
}

#[test]
fn num_enum2_rev() {
    assert!(is_sub(
        r#"{"enum": [1, 2.0]}"#,
        r#"{"enum": [1.0, 2, 3]}"#
    ));
}

#[test]
fn num_enum3_fwd() {
    assert!(is_sub(
        r#"{"enum": [1, 2, 3]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn num_enum3_rev() {
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"enum": [1, 2, 3]}"#
    ));
}

#[test]
fn num_enum4_fwd() {
    // 2.0 is integer-valued
    assert!(is_sub(
        r#"{"enum": [1, 2.0, 3]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn num_enum4_rev() {
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"enum": [1, 2.0, 3]}"#
    ));
}
