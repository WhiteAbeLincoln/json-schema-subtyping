// Translated from: IBM/jsonsubschema test/test_array.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0
//
// Draft-4 `items` (list form) + `additionalItems` has been converted to
// 2020-12 `prefixItems` + `items`.

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

// --- TestArraySubtype ---

#[test]
fn arr_identity() {
    assert!(is_sub(
        r#"{"type": "array", "minItems": 5, "maxItems": 10}"#,
        r#"{"type": "array", "minItems": 5, "maxItems": 10}"#
    ));
}

#[test]
fn arr_min_max_fwd() {
    assert!(is_sub(
        r#"{"type": "array", "minItems": 5, "maxItems": 10}"#,
        r#"{"type": "array", "minItems": 1, "maxItems": 20}"#
    ));
}

#[test]
fn arr_min_max_rev() {
    assert!(!is_sub(
        r#"{"type": "array", "minItems": 1, "maxItems": 20}"#,
        r#"{"type": "array", "minItems": 5, "maxItems": 10}"#
    ));
}

#[test]
fn arr_unique_fwd() {
    assert!(is_sub(
        r#"{"type": "array", "uniqueItems": true}"#,
        r#"{"type": "array", "uniqueItems": false}"#
    ));
}

#[test]
fn arr_unique_rev() {
    assert!(!is_sub(
        r#"{"type": "array", "uniqueItems": false}"#,
        r#"{"type": "array", "uniqueItems": true}"#
    ));
}

#[test]
fn arr_empty_items1_fwd() {
    // {type: array} equiv {type: array, items: {}}
    assert!(is_sub(
        r#"{"type": "array"}"#,
        r#"{"type": "array", "items": {}}"#
    ));
}

#[test]
fn arr_empty_items1_rev() {
    assert!(is_sub(
        r#"{"type": "array", "items": {}}"#,
        r#"{"type": "array"}"#
    ));
}

// In 2020-12, `additionalItems` without a list-form `items` has no effect (same as draft-4).
// So {type: array, items: false} in 2020-12 is different from draft-4's {additionalItems: false}.
// The draft-4 test_empty_items2 tested that {additionalItems: false} ≡ {items: {}} when no
// list-form items is present. In 2020-12, the equivalent is just {type: array} ≡ {type: array, items: {}}.
// That's already covered by arr_empty_items1. Skipping test_empty_items2 as not applicable to 2020-12.

// Draft-4: {items: [{},{}], additionalItems: false} → 2020-12: {prefixItems: [{},{}], items: false}
#[test]
fn arr_empty_items3_fwd() {
    // prefixItems with items:false limits length; items:{} does not
    assert!(is_sub(
        r#"{"type": "array", "prefixItems": [{}, {}], "items": false}"#,
        r#"{"type": "array", "items": {}}"#
    ));
}

#[test]
fn arr_empty_items3_rev() {
    assert!(!is_sub(
        r#"{"type": "array", "items": {}}"#,
        r#"{"type": "array", "prefixItems": [{}, {}], "items": false}"#
    ));
}

// Draft-4: {items: [{},{}], additionalItems: true} ≡ {items: {}}
// 2020-12: {prefixItems: [{},{}], items: {}} ≡ {items: {}}
#[test]
fn arr_empty_items4_fwd() {
    assert!(is_sub(
        r#"{"type": "array", "prefixItems": [{}, {}], "items": {}}"#,
        r#"{"type": "array", "items": {}}"#
    ));
}

#[test]
fn arr_empty_items4_rev() {
    assert!(is_sub(
        r#"{"type": "array", "items": {}}"#,
        r#"{"type": "array", "prefixItems": [{}, {}], "items": {}}"#
    ));
}

// Draft-4: {items: [{},{}], additionalItems: false} vs {items: [{}], additionalItems: false}
// 2020-12: maxLength 2 vs maxLength 1
#[test]
fn arr_empty_items5_fwd() {
    assert!(!is_sub(
        r#"{"type": "array", "prefixItems": [{}, {}], "items": false}"#,
        r#"{"type": "array", "prefixItems": [{}], "items": false}"#
    ));
}

#[test]
fn arr_empty_items5_rev() {
    assert!(is_sub(
        r#"{"type": "array", "prefixItems": [{}], "items": false}"#,
        r#"{"type": "array", "prefixItems": [{}, {}], "items": false}"#
    ));
}

// Draft-4: items: {type:string} vs items: [{type:string}]
// 2020-12: items: {type:string} vs prefixItems: [{type:string}], items: {}
#[test]
fn arr_dict_list_items1_fwd() {
    assert!(is_sub(
        r#"{"type": "array", "items": {"type": "string"}}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}]}"#
    ));
}

#[test]
fn arr_dict_list_items1_rev() {
    assert!(!is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}]}"#,
        r#"{"type": "array", "items": {"type": "string"}}"#
    ));
}

#[test]
fn arr_dict_list_items2_fwd() {
    assert!(is_sub(
        r#"{"type": "array", "items": {"type": "string"}}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "string"}]}"#
    ));
}

#[test]
fn arr_dict_list_items2_rev() {
    assert!(!is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "string"}]}"#,
        r#"{"type": "array", "items": {"type": "string"}}"#
    ));
}

// Draft-4: items:[{string}] vs items:[{string},{number}]
// 2020-12: prefixItems:[{string}],items:{} vs prefixItems:[{string},{number}],items:{}
#[test]
fn arr_dict_list_items3_fwd() {
    // s1 allows [str, anything, ...] — position 1 could be non-number
    assert!(!is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}]}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "number"}]}"#
    ));
}

#[test]
fn arr_dict_list_items3_rev() {
    assert!(is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "number"}]}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}]}"#
    ));
}

// Draft-4: items:[{string}], additionalItems:false vs items:[{string},{number}]
// 2020-12: prefixItems:[{string}], items:false vs prefixItems:[{string},{number}]
#[test]
fn arr_dict_list_items4_fwd() {
    assert!(is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}], "items": false}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "number"}]}"#
    ));
}

#[test]
fn arr_dict_list_items4_rev() {
    assert!(!is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "number"}]}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}], "items": false}"#
    ));
}

// Draft-4: items:[{string}], additionalItems:true vs items:[{string},{number}]
// 2020-12: prefixItems:[{string}], items:{} vs prefixItems:[{string},{number}]
#[test]
fn arr_dict_list_items5_fwd() {
    assert!(!is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}], "items": {}}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "number"}]}"#
    ));
}

#[test]
fn arr_dict_list_items5_rev() {
    assert!(is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "number"}]}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}], "items": {}}"#
    ));
}

// Draft-4: items:[{string}], additionalItems:{} vs items:[{string},{number}]
// 2020-12: prefixItems:[{string}], items:{} vs prefixItems:[{string},{number}]
// (additionalItems: {} = true in draft-4 → items: {} in 2020-12)
#[test]
fn arr_dict_list_items6_fwd() {
    assert!(!is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}], "items": {}}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "number"}]}"#
    ));
}

#[test]
fn arr_dict_list_items6_rev() {
    assert!(is_sub(
        r#"{"type": "array", "prefixItems": [{"type": "string"}, {"type": "number"}]}"#,
        r#"{"type": "array", "prefixItems": [{"type": "string"}], "items": {}}"#
    ));
}

// --- TestNestedArray ---

#[test]
fn nested_array1() {
    assert!(!is_sub(
        r#"{
            "type": "array",
            "minItems": 150, "maxItems": 150,
            "items": {
                "type": "array",
                "minItems": 4, "maxItems": 4,
                "items": {"type": "number"}
            }
        }"#,
        r#"{
            "anyOf": [
                {"type": "array", "items": {"type": "string"}},
                {"type": "array", "items": {
                    "type": "array",
                    "minItems": 1, "maxItems": 1,
                    "items": {"type": "string"}
                }}
            ]
        }"#
    ));
}
