// Translated from: IBM/jsonsubschema test/test_enum.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

#[test]
fn enum_simple1_fwd() {
    assert!(is_sub(r#"{"enum": [1]}"#, r#"{"enum": [1, 2]}"#));
}

#[test]
fn enum_simple1_rev() {
    assert!(!is_sub(r#"{"enum": [1, 2]}"#, r#"{"enum": [1]}"#));
}

#[test]
fn enum_simple2_fwd() {
    assert!(!is_sub(r#"{"enum": [true]}"#, r#"{"enum": [1, 2]}"#));
}

#[test]
fn enum_simple2_rev() {
    assert!(!is_sub(r#"{"enum": [1, 2]}"#, r#"{"enum": [true]}"#));
}

#[test]
fn enum_simple3_fwd() {
    assert!(!is_sub(
        r#"{"type": "integer", "enum": [1, 2]}"#,
        r#"{"type": "boolean", "enum": [true]}"#
    ));
}

#[test]
fn enum_simple3_rev() {
    assert!(!is_sub(
        r#"{"type": "boolean", "enum": [true]}"#,
        r#"{"type": "integer", "enum": [1, 2]}"#
    ));
}

#[test]
fn enum_simple4_fwd() {
    assert!(!is_sub(
        r#"{"enum": ["1", 2]}"#,
        r#"{"enum": [1, "2"]}"#
    ));
}

#[test]
fn enum_simple4_rev() {
    assert!(!is_sub(
        r#"{"enum": [1, "2"]}"#,
        r#"{"enum": ["1", 2]}"#
    ));
}

#[test]
fn enum_uninhabited1_fwd() {
    // type:string with enum:[1,2] — no string values in enum → uninhabited
    assert!(is_sub(
        r#"{"type": "string", "enum": [1, 2]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn enum_uninhabited1_rev() {
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"type": "string", "enum": [1, 2]}"#
    ));
}

#[test]
fn enum_uninhabited2_fwd() {
    // type:string, enum:[0,1] → uninhabited. type:boolean, enum:[0] → uninhabited (0 is not boolean).
    // Both uninhabited → mutual subtypes.
    assert!(is_sub(
        r#"{"type": "string", "enum": [0, 1]}"#,
        r#"{"type": "boolean", "enum": [0]}"#
    ));
}

#[test]
fn enum_uninhabited2_rev() {
    assert!(is_sub(
        r#"{"type": "boolean", "enum": [0]}"#,
        r#"{"type": "string", "enum": [0, 1]}"#
    ));
}

#[test]
fn enum_regex_string_fwd() {
    assert!(!is_sub(
        r#"{"enum": ["^*"]}"#,
        r#"{"enum": ["^^"]}"#
    ));
}

#[test]
fn enum_regex_string_rev() {
    assert!(!is_sub(
        r#"{"enum": ["^^"]}"#,
        r#"{"enum": ["^*"]}"#
    ));
}
