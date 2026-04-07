// Translated from: IBM/jsonsubschema test/test_boolean.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

// --- TestSingletonBooleans ---

#[test]
fn singleton_oneof_fwd() {
    assert!(is_sub(
        r#"{"oneOf": [{"type": "string"}]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn singleton_oneof_rev() {
    assert!(is_sub(
        r#"{"type": "string"}"#,
        r#"{"oneOf": [{"type": "string"}]}"#
    ));
}

#[test]
fn singleton_anyof_fwd() {
    assert!(is_sub(
        r#"{"anyOf": [{"type": "string"}]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn singleton_anyof_rev() {
    assert!(is_sub(
        r#"{"type": "string"}"#,
        r#"{"anyOf": [{"type": "string"}]}"#
    ));
}

#[test]
fn singleton_allof_fwd() {
    assert!(is_sub(
        r#"{"allOf": [{"type": "string"}]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn singleton_allof_rev() {
    assert!(is_sub(
        r#"{"type": "string"}"#,
        r#"{"allOf": [{"type": "string"}]}"#
    ));
}

#[test]
fn singleton_allof_oneof_fwd() {
    assert!(is_sub(
        r#"{"allOf": [{"type": "string"}]}"#,
        r#"{"oneOf": [{"type": "string"}]}"#
    ));
}

#[test]
fn singleton_allof_oneof_rev() {
    assert!(is_sub(
        r#"{"oneOf": [{"type": "string"}]}"#,
        r#"{"allOf": [{"type": "string"}]}"#
    ));
}

// --- TestOneOf ---

#[test]
fn oneof1_fwd() {
    // oneOf[string, {}] equiv not(string)
    assert!(!is_sub(
        r#"{"oneOf": [{"type": "string"}, {}]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn oneof1_rev() {
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"oneOf": [{"type": "string"}, {}]}"#
    ));
}

#[test]
fn oneof2_fwd() {
    // oneOf[string, {}] equiv not(string)
    // requires negation elimination
    assert!(is_sub(
        r#"{"oneOf": [{"type": "string"}, {}]}"#,
        r#"{"not": {"type": "string"}}"#
    ));
}

#[test]
fn oneof2_rev() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"not": {"type": "string"}}"#,
        r#"{"oneOf": [{"type": "string"}, {}]}"#
    ));
}

#[test]
fn oneof4_fwd() {
    // oneOf[boolean, enum:[true]] = enum:[false] (exactly one must match)
    assert!(is_sub(
        r#"{"oneOf": [{"type": "boolean"}, {"enum": [true]}]}"#,
        r#"{"enum": [false]}"#
    ));
}

#[test]
fn oneof4_rev() {
    assert!(is_sub(
        r#"{"enum": [false]}"#,
        r#"{"oneOf": [{"type": "boolean"}, {"enum": [true]}]}"#
    ));
}

#[test]
fn oneof5_fwd() {
    // oneOf[[1,2,3], [1,2]] = enum:[3] (only 3 satisfies exactly one)
    assert!(is_sub(
        r#"{"oneOf": [{"enum": [1, 2, 3]}, {"enum": [1, 2]}]}"#,
        r#"{"enum": [3]}"#
    ));
}

#[test]
fn oneof5_rev() {
    assert!(is_sub(
        r#"{"enum": [3]}"#,
        r#"{"oneOf": [{"enum": [1, 2, 3]}, {"enum": [1, 2]}]}"#
    ));
}

#[test]
fn oneof6_fwd() {
    // oneOf[[1,2,3], [1,2]] = enum:[3], not [1,2]
    assert!(!is_sub(
        r#"{"oneOf": [{"enum": [1, 2, 3]}, {"enum": [1, 2]}]}"#,
        r#"{"enum": [1, 2]}"#
    ));
}

#[test]
fn oneof6_rev() {
    assert!(!is_sub(
        r#"{"enum": [1, 2]}"#,
        r#"{"oneOf": [{"enum": [1, 2, 3]}, {"enum": [1, 2]}]}"#
    ));
}

// --- TestAllOf ---

#[test]
fn allof1_fwd() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"allOf": [{"type": "string"}, {"type": "string", "pattern": "a"}]}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn allof1_rev() {
    // requires allOf meet
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"allOf": [{"type": "string"}, {"type": "string", "pattern": "a"}]}"#
    ));
}

#[test]
fn allof2_fwd() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"allOf": [{"minimum": 10}, {"maximum": 20}]}"#,
        r#"{"minimum": 10, "maximum": 20}"#
    ));
}

#[test]
fn allof2_rev() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"minimum": 10, "maximum": 20}"#,
        r#"{"allOf": [{"minimum": 10}, {"maximum": 20}]}"#
    ));
}

// --- TestNotBoolean ---

#[test]
fn not_allof1_fwd() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"not": {"allOf": [{"type": "string"}, {"type": "string", "pattern": "a"}]}}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn not_allof1_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"not": {"allOf": [{"type": "string"}, {"type": "string", "pattern": "a"}]}}"#
    ));
}

#[test]
fn not_allof2_fwd() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"not": {"allOf": [{"type": "string"}, {"type": "string", "pattern": "a"}]}}"#,
        r#"{"anyOf": [{"type": "integer"}, {"type": "number"}, {"type": "boolean"}, {"type": "array"}, {"type": "object"}, {"type": "string"}, {"type": "null"}]}"#
    ));
}

#[test]
fn not_allof2_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer"}, {"type": "number"}, {"type": "boolean"}, {"type": "array"}, {"type": "object"}, {"type": "string"}, {"type": "null"}]}"#,
        r#"{"not": {"allOf": [{"type": "string"}, {"type": "string", "pattern": "a"}]}}"#
    ));
}

#[test]
fn not_allof3_fwd() {
    // requires negation elimination + allOf meet
    assert!(is_sub(
        r#"{"not": {"allOf": [{"type": "string"}, {"type": "string", "pattern": "a"}]}}"#,
        r#"{"anyOf": [{"type": "integer"}, {"type": "number"}, {"type": "boolean"}, {"type": "array"}, {"type": "object"}, {"type": "string", "pattern": "^[^a]*$"}, {"type": "null"}]}"#
    ));
}

#[test]
fn not_allof3_rev() {
    // requires negation elimination + allOf meet
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer"}, {"type": "number"}, {"type": "boolean"}, {"type": "array"}, {"type": "object"}, {"type": "string", "pattern": "^[^a]*$"}, {"type": "null"}]}"#,
        r#"{"not": {"allOf": [{"type": "string"}, {"type": "string", "pattern": "a"}]}}"#
    ));
}

#[test]
fn not_allof4_fwd() {
    // not(allOf[string, boolean]) = not(bottom) = top
    assert!(is_sub(
        r#"{"not": {"allOf": [{"type": "string"}, {"type": "boolean"}]}}"#,
        r#"{}"#
    ));
}

#[test]
fn not_allof4_rev() {
    assert!(is_sub(
        r#"{}"#,
        r#"{"not": {"allOf": [{"type": "string"}, {"type": "boolean"}]}}"#
    ));
}

#[test]
fn not_anyof1_fwd() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"not": {"anyOf": [{"type": "string"}, {"type": "null"}]}}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn not_anyof1_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"not": {"anyOf": [{"type": "string"}, {"type": "null"}]}}"#
    ));
}

#[test]
fn not_anyof2_fwd() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"not": {"anyOf": [{"type": "string"}, {"type": "null"}]}}"#,
        r#"{"anyOf": [{"type": "integer"}, {"type": "number"}, {"type": "boolean"}, {"type": "array"}, {"type": "object"}, {"type": "string"}, {"type": "null"}]}"#
    ));
}

#[test]
fn not_anyof2_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"anyOf": [{"type": "integer"}, {"type": "number"}, {"type": "boolean"}, {"type": "array"}, {"type": "object"}, {"type": "string"}, {"type": "null"}]}"#,
        r#"{"not": {"anyOf": [{"type": "string"}, {"type": "null"}]}}"#
    ));
}

#[test]
fn not_oneof1_fwd() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"not": {"oneOf": [{"type": "string"}, {"type": "null"}]}}"#,
        r#"{"type": "string"}"#
    ));
}

#[test]
fn not_oneof1_rev() {
    // requires negation elimination
    assert!(!is_sub(
        r#"{"type": "string"}"#,
        r#"{"not": {"oneOf": [{"type": "string"}, {"type": "null"}]}}"#
    ));
}

#[test]
fn not_oneof2_fwd() {
    // not(oneOf[[1,2,3],[1,2]]) equiv not(enum:[3])
    // requires negation elimination
    assert!(is_sub(
        r#"{"not": {"oneOf": [{"enum": [1, 2, 3]}, {"enum": [1, 2]}]}}"#,
        r#"{"not": {"enum": [3]}}"#
    ));
}

#[test]
fn not_oneof2_rev() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"not": {"enum": [3]}}"#,
        r#"{"not": {"oneOf": [{"enum": [1, 2, 3]}, {"enum": [1, 2]}]}}"#
    ));
}

// --- TestNotBooleans ---

#[test]
fn not_and_allof1_fwd() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"not": {"type": "string"}, "allOf": [{"type": "integer"}, {"enum": [5]}]}"#,
        r#"{"enum": [5]}"#
    ));
}

#[test]
fn not_and_allof1_rev() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"enum": [5]}"#,
        r#"{"not": {"type": "string"}, "allOf": [{"type": "integer"}, {"enum": [5]}]}"#
    ));
}

#[test]
fn not_and_anyof1_fwd() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"not": {"type": "string"}, "anyOf": [{"type": "integer"}, {"type": "boolean"}]}"#,
        r#"{"type": ["integer", "boolean"]}"#
    ));
}

#[test]
fn not_and_anyof1_rev() {
    // requires negation elimination
    assert!(is_sub(
        r#"{"type": ["integer", "boolean"]}"#,
        r#"{"not": {"type": "string"}, "anyOf": [{"type": "integer"}, {"type": "boolean"}]}"#
    ));
}

#[test]
fn not_and_two_booleans_fwd() {
    // requires allOf meet + negation elimination
    assert!(is_sub(
        r#"{"not": {"type": "string"}, "anyOf": [{"type": "integer"}, {"type": "boolean"}], "allOf": [{"minimum": 10}]}"#,
        r#"{"type": ["integer", "boolean"]}"#
    ));
}

#[test]
fn not_and_two_booleans_rev() {
    // requires allOf meet + negation elimination
    assert!(!is_sub(
        r#"{"type": ["integer", "boolean"]}"#,
        r#"{"not": {"type": "string"}, "anyOf": [{"type": "integer"}, {"type": "boolean"}], "allOf": [{"minimum": 10}]}"#
    ));
}

#[test]
fn two_booleans_fwd() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"anyOf": [{"type": "integer"}, {"type": "boolean"}], "allOf": [{"minimum": 10}, {"maximum": 20}]}"#,
        r#"{"type": ["integer", "boolean"], "minimum": 10, "maximum": 20}"#
    ));
}

#[test]
fn two_booleans_rev() {
    // requires allOf meet
    assert!(is_sub(
        r#"{"type": ["integer", "boolean"], "minimum": 10, "maximum": 20}"#,
        r#"{"anyOf": [{"type": "integer"}, {"type": "boolean"}], "allOf": [{"minimum": 10}, {"maximum": 20}]}"#
    ));
}
