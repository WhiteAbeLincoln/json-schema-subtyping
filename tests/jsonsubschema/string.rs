// Translated from: IBM/jsonsubschema test/test_string.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

// --- TestStringSubtype ---

#[test]
fn str_min_vs_int_max_fwd() {
    assert!(!is_sub(
        r#"{"type": "string", "minLength": 5}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn str_min_vs_int_max_rev() {
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "string", "minLength": 5}"#
    ));
}

#[test]
fn empty_pattern_equiv_string_fwd() {
    assert!(is_sub(
        r#"{"type": "string", "pattern": ""}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn empty_pattern_equiv_string_rev() {
    assert!(is_sub(
        r#"{"type": "string"}"#,
        r#"{"type": "string", "pattern": ""}"#
    ));
}

#[test]
fn regx_range1_fwd() {
    assert!(!is_sub(
        r#"{"type": "string", "maxLength": 5, "pattern": "(ab)*"}"#,
        r#"{"type": "string", "pattern": "(ab){3}"}"#
    ));
}

#[test]
fn regx_range1_rev() {
    assert!(!is_sub(
        r#"{"type": "string", "pattern": "(ab){3}"}"#,
        r#"{"type": "string", "maxLength": 5, "pattern": "(ab)*"}"#
    ));
}

#[test]
fn regx_range2_fwd() {
    assert!(is_sub(
        r#"{"type": "string", "maxLength": 5, "pattern": "^(ab)*$"}"#,
        r#"{"type": "string", "pattern": "^(ab){0,3}$"}"#
    ));
}

#[test]
fn regx_range2_rev() {
    assert!(!is_sub(
        r#"{"type": "string", "pattern": "^(ab){0,3}$"}"#,
        r#"{"type": "string", "maxLength": 5, "pattern": "^(ab)*$"}"#
    ));
}

// --- TestNotStringSubtype ---
// These tests require negation elimination (not(typed-schema) → anyOf)
// and/or allOf meet. They are expected to fail until those features land.

#[test]
fn str_not_str_fwd() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"not": {"type": "string"}}"#
    ));
}

#[test]
fn str_not_str_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"not": {"type": "string"}}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn str_not_str_with_range_fwd() {
    // requires allOf meet + negation elimination
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#
    ));
}

#[test]
fn str_not_str_with_range_rev() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn str_not_str_with_range2_fwd() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"type": "string", "maxLength": 1}"#,
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#
    ));
}

#[test]
fn str_not_str_with_range2_rev() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#,
        r#"{"type": "string", "maxLength": 1}"#
    ));
}

#[test]
fn str_not_str_with_range3_fwd() {
    // requires allOf meet + negation elimination
    assert!(!is_sub(
        r#"{"type": "string", "minLength": 1, "maxLength": 5}"#,
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#
    ));
}

#[test]
fn str_not_str_with_range3_rev() {
    // requires allOf meet + negation elimination
    assert!(!is_sub(
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#,
        r#"{"type": "string", "minLength": 1, "maxLength": 5}"#
    ));
}

#[test]
fn not_str_not_str1_fwd() {
    // not(string) equiv not(not(not(string)))
    // requires negation elimination
    assert!(is_sub(
        r#"{"not": {"type": "string"}}"#,
        r#"{"not": {"not": {"not": {"type": "string"}}}}"#
    ));
}

#[test]
fn not_str_not_str1_rev() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"not": {"not": {"not": {"type": "string"}}}}"#,
        r#"{"not": {"type": "string"}}"#
    ));
}

#[test]
fn not_str_not_str2_fwd() {
    // not(string) NOT equiv not(not(string)) = string
    // requires negation elimination
    assert!(!is_sub(
        r#"{"not": {"type": "string"}}"#,
        r#"{"not": {"not": {"type": "string"}}}"#
    ));
}

#[test]
fn not_str_not_str2_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"not": {"not": {"type": "string"}}}"#,
        r#"{"not": {"type": "string"}}"#
    ));
}

#[test]
fn all_str_not_str1_fwd() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn all_str_not_str1_rev() {
    // requires allOf meet + negation elimination
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#
    ));
}

#[test]
fn all_str_not_str2_fwd() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#,
        r#"{"type": "string", "maxLength": 1}"#
    ));
}

#[test]
fn all_str_not_str2_rev() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"type": "string", "maxLength": 1}"#,
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2}}]}"#
    ));
}

#[test]
fn all_str_not_str3_fwd() {
    // requires allOf meet + negation elimination
    assert!(!is_sub(
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2, "pattern": "ab"}}]}"#,
        r#"{"type": "string", "maxLength": 1}"#
    ));
}

#[test]
fn all_str_not_str3_rev() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"type": "string", "maxLength": 1}"#,
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 2, "pattern": "ab"}}]}"#
    ));
}

// test_not_str_and_join_string
#[test]
fn not_str_and_join_string_fwd() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 5, "pattern": "a"}}]}"#,
        r#"{"anyOf": [{"type": "string", "maxLength": 4}, {"type": "string", "pattern": "[^a]"}]}"#
    ));
}

#[test]
fn not_str_and_join_string_rev() {
    // requires allOf meet + negation elimination
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "string", "maxLength": 4}, {"type": "string", "pattern": "[^a]"}]}"#,
        r#"{"allOf": [{"type": "string"}, {"not": {"type": "string", "minLength": 5, "pattern": "a"}}]}"#
    ));
}

// test_equiv_multiple_case: multiple equivalent representations of type: ["string", "null"], minLength: 1
#[test]
fn equiv_multiple_case_s1_s2() {
    let s1 = r#"{"type": ["string", "null"], "minLength": 1}"#;
    let s2 = r#"{"anyOf": [{"type": "string", "minLength": 1}, {"type": "null"}]}"#;
    assert!(is_sub(s1, s2));
    assert!(is_sub(s2, s1));
}

#[test]
fn equiv_multiple_case_s1_s3() {
    let s1 = r#"{"type": ["string", "null"], "minLength": 1}"#;
    let s3 = r#"{"anyOf": [{"type": "string", "pattern": ".+"}, {"enum": [null]}]}"#;
    assert!(is_sub(s1, s3));
    assert!(is_sub(s3, s1));
}

#[test]
fn equiv_multiple_case_s1_s4() {
    let s1 = r#"{"type": ["string", "null"], "minLength": 1}"#;
    let s4 = r#"{"type": ["string", "null"], "pattern": ".{1,}"}"#;
    assert!(is_sub(s1, s4));
    assert!(is_sub(s4, s1));
}

#[test]
fn equiv_multiple_case_s1_s5() {
    // requires negation elimination
    let s1 = r#"{"type": ["string", "null"], "minLength": 1}"#;
    let s5 = r#"{"type": ["string", "null"], "not": {"enum": [""]}}"#;
    assert!(is_sub(s1, s5));
    assert!(is_sub(s5, s1));
}

#[test]
fn equiv_multiple_case_s6_s7() {
    let s6 = r#"{"type": ["string", "null"], "pattern": ".{2,}"}"#;
    let s7 = r#"{"type": ["string", "null"], "minLength": 2}"#;
    assert!(is_sub(s6, s7));
    assert!(is_sub(s7, s6));
}

#[test]
fn equiv_multiple_case_s6_sub_s1() {
    let s6 = r#"{"type": ["string", "null"], "pattern": ".{2,}"}"#;
    let s1 = r#"{"type": ["string", "null"], "minLength": 1}"#;
    assert!(is_sub(s6, s1));
}

#[test]
fn equiv_multiple_case_s1_not_sub_s7() {
    let s1 = r#"{"type": ["string", "null"], "minLength": 1}"#;
    let s7 = r#"{"type": ["string", "null"], "minLength": 2}"#;
    assert!(!is_sub(s1, s7));
}

// --- TestStringEnumSubtype ---

#[test]
fn enum_str_a_equiv_enum_a_fwd() {
    assert!(is_sub(
        r#"{"type": "string", "enum": ["a"]}"#,
        r#"{"enum": ["a"]}"#
    ));
}

#[test]
fn enum_str_a_equiv_enum_a_rev() {
    assert!(is_sub(
        r#"{"enum": ["a"]}"#,
        r#"{"type": "string", "enum": ["a"]}"#
    ));
}

#[test]
fn enum_str_a_sub_enum_ab_fwd() {
    assert!(is_sub(
        r#"{"type": "string", "enum": ["a"]}"#,
        r#"{"enum": ["a", "b"]}"#
    ));
}

#[test]
fn enum_str_a_sub_enum_ab_rev() {
    assert!(!is_sub(
        r#"{"enum": ["a", "b"]}"#,
        r#"{"type": "string", "enum": ["a"]}"#
    ));
}

#[test]
fn enum_str_a_empty_vs_enum_ab_fwd() {
    assert!(!is_sub(
        r#"{"type": "string", "enum": ["a", ""]}"#,
        r#"{"enum": ["a", "b"]}"#
    ));
}

#[test]
fn enum_str_a_empty_vs_enum_ab_rev() {
    assert!(!is_sub(
        r#"{"enum": ["a", "b"]}"#,
        r#"{"type": "string", "enum": ["a", ""]}"#
    ));
}

#[test]
fn enum_abc_or_string_equiv_string_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"enum": ["a", "b", "c"]}, {"type": "string"}]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn enum_abc_or_string_equiv_string_rev() {
    assert!(is_sub(
        r#"{"type": "string"}"#,
        r#"{"anyOf": [{"enum": ["a", "b", "c"]}, {"type": "string"}]}"#
    ));
}

#[test]
fn not_enum_a_sub_string_fwd() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"type": "string", "not": {"enum": ["a"]}}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn not_enum_a_sub_string_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"type": "string", "not": {"enum": ["a"]}}"#
    ));
}

#[test]
fn not_enum_ab_vs_enum_ab_fwd() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"type": "string", "not": {"enum": ["a", "b"]}}"#,
        r#"{"type": "string", "enum": ["a", "b"]}"#
    ));
}

#[test]
fn not_enum_ab_vs_enum_ab_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"type": "string", "enum": ["a", "b"]}"#,
        r#"{"type": "string", "not": {"enum": ["a", "b"]}}"#
    ));
}
