// Translated from: IBM/jsonsubschema test/test_mix.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

// --- TestMixedTypes ---

#[test]
fn mix_num_vs_arr_fwd() {
    assert!(!is_sub(r#"{"type": "number"}"#, r#"{"type": "array"}"#));
}

#[test]
fn mix_num_vs_arr_rev() {
    assert!(!is_sub(r#"{"type": "array"}"#, r#"{"type": "number"}"#));
}

#[test]
fn mix_num_vs_list_num_fwd() {
    assert!(is_sub(r#"{"type": "number"}"#, r#"{"type": ["number"]}"#));
}

#[test]
fn mix_num_vs_list_num_rev() {
    assert!(is_sub(r#"{"type": ["number"]}"#, r#"{"type": "number"}"#));
}

#[test]
fn mix_int_vs_list_num_fwd() {
    assert!(is_sub(r#"{"type": "integer"}"#, r#"{"type": ["number"]}"#));
}

#[test]
fn mix_int_vs_list_num_rev() {
    assert!(!is_sub(r#"{"type": ["number"]}"#, r#"{"type": "integer"}"#));
}

#[test]
fn mix_int_vs_list_num_str_fwd() {
    assert!(is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": ["number", "string"]}"#
    ));
}

#[test]
fn mix_int_vs_list_num_str_rev() {
    assert!(!is_sub(
        r#"{"type": ["number", "string"]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn mix_list_str_arr_vs_list_num_str_fwd() {
    assert!(!is_sub(
        r#"{"type": ["string", "array"]}"#,
        r#"{"type": ["number", "string"]}"#
    ));
}

#[test]
fn mix_list_str_arr_vs_list_num_str_rev() {
    assert!(!is_sub(
        r#"{"type": ["number", "string"]}"#,
        r#"{"type": ["string", "array"]}"#
    ));
}

#[test]
fn mix_str_int_fwd() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "string", "pattern": "a+", "allOf": [{"type": "string", "pattern": "b+"}, {"allOf": [{"type": "string", "maxLength": 10}]}]}"#,
        r#"{"type": "integer"}"#
    ));
}

#[test]
fn mix_str_int_rev() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "integer"}"#,
        r#"{"type": "string", "pattern": "a+", "allOf": [{"type": "string", "pattern": "b+"}, {"allOf": [{"type": "string", "maxLength": 10}]}]}"#
    ));
}

#[test]
fn mix_str_bool_equiv_anyof_fwd() {
    assert!(is_sub(
        r#"{"type": ["string", "boolean"]}"#,
        r#"{"anyOf": [{"type": "string"}, {"type": "boolean"}]}"#
    ));
}

#[test]
fn mix_str_bool_equiv_anyof_rev() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "string"}, {"type": "boolean"}]}"#,
        r#"{"type": ["string", "boolean"]}"#
    ));
}

#[test]
fn mix_allany_any_fwd() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"allOf": [{"type": ["string", "boolean"]}], "type": ["string", "boolean"]}"#,
        r#"{"anyOf": [{"type": "string"}, {"type": "boolean"}]}"#
    ));
}

#[test]
fn mix_allany_any_rev() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"anyOf": [{"type": "string"}, {"type": "boolean"}]}"#,
        r#"{"allOf": [{"type": ["string", "boolean"]}], "type": ["string", "boolean"]}"#
    ));
}

#[test]
fn mix_enum1_fwd() {
    assert!(!is_sub(
        r#"{"enum": [1, 2, "test", false]}"#,
        r#"{"type": ["integer", "string"], "minimum": 10, "enum": [1, 2]}"#
    ));
}

#[test]
fn mix_enum1_rev() {
    assert!(is_sub(
        r#"{"type": ["integer", "string"], "minimum": 10, "enum": [1, 2]}"#,
        r#"{"enum": [1, 2, "test", false]}"#
    ));
}

#[test]
fn mix_enum2_fwd() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"allOf": [{"enum": [1, 2, 3]}, {"type": "integer"}], "enum": [3, 4, 5]}"#,
        r#"{"type": "integer", "enum": [1, 2, 3]}"#
    ));
}

#[test]
fn mix_enum2_rev() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "integer", "enum": [1, 2, 3]}"#,
        r#"{"allOf": [{"enum": [1, 2, 3]}, {"type": "integer"}], "enum": [3, 4, 5]}"#
    ));
}

#[test]
fn mix_enum3_fwd() {
    assert!(!is_sub(
        r#"{"enum": [3, 4, 5]}"#,
        r#"{"enum": [1, 2, 3]}"#
    ));
}

#[test]
fn mix_enum3_rev() {
    assert!(!is_sub(
        r#"{"enum": [1, 2, 3]}"#,
        r#"{"enum": [3, 4, 5]}"#
    ));
}

#[test]
fn mix_enum4_fwd() {
    assert!(is_sub(
        r#"{"enum": [3, 4, 5]}"#,
        r#"{"enum": [4, 5, 3]}"#
    ));
}

#[test]
fn mix_enum4_rev() {
    assert!(is_sub(
        r#"{"enum": [4, 5, 3]}"#,
        r#"{"enum": [3, 4, 5]}"#
    ));
}

#[test]
fn mix_enum5_fwd() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"enum": [3, 4, 5], "allOf": [{"enum": [1, 2]}]}"#,
        r#"{"enum": [4, 5, 3]}"#
    ));
}

#[test]
fn mix_enum5_rev() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"enum": [4, 5, 3]}"#,
        r#"{"enum": [3, 4, 5], "allOf": [{"enum": [1, 2]}]}"#
    ));
}

#[test]
fn mix_enum6_fwd() {
    // type:string + enum:[3,4,5] → uninhabited
    assert!(is_sub(
        r#"{"enum": [3, 4, 5], "type": "string"}"#,
        r#"{"enum": [4, 5, 3]}"#
    ));
}

#[test]
fn mix_enum6_rev() {
    assert!(!is_sub(
        r#"{"enum": [4, 5, 3]}"#,
        r#"{"enum": [3, 4, 5], "type": "string"}"#
    ));
}

#[test]
fn mix_top_nottop_fwd() {
    assert!(!is_sub(r#"{}"#, r#"{"type": "string"}"#));
}

#[test]
fn mix_top_nottop_rev() {
    assert!(is_sub(r#"{"type": "string"}"#, r#"{}"#));
}

#[test]
fn mix_top_bot_fwd() {
    // type:string + enum:[1,2,3] → uninhabited → subtype of everything
    assert!(!is_sub(
        r#"{}"#,
        r#"{"type": "string", "enum": [1, 2, 3]}"#
    ));
}

#[test]
fn mix_top_bot_rev() {
    assert!(is_sub(
        r#"{"type": "string", "enum": [1, 2, 3]}"#,
        r#"{}"#
    ));
}

#[test]
fn mix_uninhabited1_fwd() {
    // type:string + enum:[2] → uninhabited
    assert!(is_sub(
        r#"{"type": "string", "enum": [2]}"#,
        r#"{"type": "boolean"}"#
    ));
}

#[test]
fn mix_uninhabited1_rev() {
    assert!(!is_sub(
        r#"{"type": "boolean"}"#,
        r#"{"type": "string", "enum": [2]}"#
    ));
}

#[test]
fn mix_not_number_fwd() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"enum": ["<0", "0<=X<200", ">=200", "no checking"]}"#,
        r#"{"not": {"type": "number"}}"#
    ));
}

#[test]
fn mix_not_number_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"not": {"type": "number"}}"#,
        r#"{"enum": ["<0", "0<=X<200", ">=200", "no checking"]}"#
    ));
}

// --- TestBottomAndTop ---

#[test]
fn bot1_fwd() {
    assert!(is_sub(r#"{"not": {}}"#, r#"{"type": "string"}"#));
}

#[test]
fn bot1_rev() {
    assert!(!is_sub(r#"{"type": "string"}"#, r#"{"not": {}}"#));
}

#[test]
fn bot2_fwd() {
    // description is annotation-only → still bottom
    assert!(is_sub(
        r#"{"description": "bottom", "not": {}}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn bot2_rev() {
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"description": "bottom", "not": {}}"#
    ));
}

#[test]
fn top1_fwd() {
    assert!(!is_sub(r#"{}"#, r#"{"type": "string"}"#));
}

#[test]
fn top1_rev() {
    assert!(is_sub(r#"{"type": "string"}"#, r#"{}"#));
}

#[test]
fn top2_fwd() {
    // description is annotation-only → still top
    assert!(is_sub(r#"{"description": "top"}"#, r#"{}"#));
}

#[test]
fn top2_rev() {
    assert!(is_sub(r#"{}"#, r#"{"description": "top"}"#));
}
