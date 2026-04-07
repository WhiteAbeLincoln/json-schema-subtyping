// Translated from: IBM/jsonsubschema test/test_const.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

#[test]
fn const_equal_num_fwd() {
    assert!(is_sub(r#"{"const": 1}"#, r#"{"const": 1}"#));
}

#[test]
fn const_equal_num_rev() {
    assert!(is_sub(r#"{"const": 1}"#, r#"{"const": 1}"#));
}

#[test]
fn const_equal_str_fwd() {
    assert!(is_sub(r#"{"const": "a"}"#, r#"{"const": "a"}"#));
}

#[test]
fn const_equal_str_rev() {
    assert!(is_sub(r#"{"const": "a"}"#, r#"{"const": "a"}"#));
}

#[test]
fn const_vs_enum_fwd() {
    assert!(is_sub(r#"{"const": 1}"#, r#"{"enum": [1, 2]}"#));
}

#[test]
fn const_vs_enum_rev() {
    assert!(!is_sub(r#"{"enum": [1, 2]}"#, r#"{"const": 1}"#));
}

#[test]
fn const_vs_wrong_enum_fwd() {
    assert!(!is_sub(r#"{"const": 1}"#, r#"{"enum": [2, 3]}"#));
}

#[test]
fn const_vs_wrong_enum_rev() {
    assert!(!is_sub(r#"{"enum": [2, 3]}"#, r#"{"const": 1}"#));
}

#[test]
fn const_vs_type_fwd() {
    assert!(is_sub(r#"{"const": 1}"#, r#"{"type": "integer"}"#));
}

#[test]
fn const_vs_type_rev() {
    assert!(!is_sub(r#"{"type": "integer"}"#, r#"{"const": 1}"#));
}

#[test]
fn const_vs_wrong_type_fwd() {
    assert!(!is_sub(r#"{"const": 1}"#, r#"{"type": "string"}"#));
}

#[test]
fn const_vs_wrong_type_rev() {
    assert!(!is_sub(r#"{"type": "string"}"#, r#"{"const": 1}"#));
}

#[test]
fn const_type_mix_fwd() {
    assert!(!is_sub(r#"{"const": "1"}"#, r#"{"const": 1}"#));
}

#[test]
fn const_type_mix_rev() {
    assert!(!is_sub(r#"{"const": 1}"#, r#"{"const": "1"}"#));
}

#[test]
fn const_uninhabited1_fwd() {
    // type:string, const:1 → uninhabited
    assert!(is_sub(
        r#"{"type": "string", "const": 1}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn const_uninhabited1_rev() {
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"type": "string", "const": 1}"#
    ));
}

#[test]
fn const_uninhabited2_fwd() {
    // Both uninhabited: type:string+const:1 and type:boolean+const:1
    assert!(is_sub(
        r#"{"type": "string", "const": 1}"#,
        r#"{"type": "boolean", "const": 1}"#
    ));
}

#[test]
fn const_uninhabited2_rev() {
    assert!(is_sub(
        r#"{"type": "boolean", "const": 1}"#,
        r#"{"type": "string", "const": 1}"#
    ));
}
