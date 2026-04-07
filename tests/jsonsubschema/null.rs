// Translated from: IBM/jsonsubschema test/test_null.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

// --- TestNull ---

#[test]
fn null1_enum_null_equiv_type_null_fwd() {
    assert!(is_sub(r#"{"enum": [null]}"#, r#"{"type": "null"}"#));
}

#[test]
fn null1_enum_null_equiv_type_null_rev() {
    assert!(is_sub(r#"{"type": "null"}"#, r#"{"enum": [null]}"#));
}

#[test]
fn null2_type_null_sub_top_fwd() {
    assert!(is_sub(r#"{"type": "null"}"#, r#"{}"#));
}

#[test]
fn null2_top_not_sub_type_null() {
    assert!(!is_sub(r#"{}"#, r#"{"type": "null"}"#));
}

#[test]
fn null3_enum_null_vs_enum_zero_fwd() {
    assert!(!is_sub(r#"{"enum": [null]}"#, r#"{"enum": [0]}"#));
}

#[test]
fn null3_enum_null_vs_enum_zero_rev() {
    assert!(!is_sub(r#"{"enum": [0]}"#, r#"{"enum": [null]}"#));
}
