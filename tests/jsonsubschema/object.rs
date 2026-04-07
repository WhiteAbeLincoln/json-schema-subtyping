// Translated from: IBM/jsonsubschema test/test_object.py
// Copyright 2017-2018 IBM Corporation — Apache-2.0

use json_schema_subtyping::SubtypeChecker;

fn is_sub(s1: &str, s2: &str) -> bool {
    SubtypeChecker::new().is_subtype(s1, s2).unwrap().is_subtype()
}

// --- TestObjectSubtype ---

#[test]
fn obj_identity() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["M", "F"]},
                "email": {"type": "string"}
            }
        }"#
    ));
}

#[test]
fn obj_min_property_fwd() {
    assert!(is_sub(
        r#"{"type": "object", "minProperties": 1}"#,
        r#"{"type": "object"}"#
    ));
}

#[test]
fn obj_min_property_rev() {
    assert!(!is_sub(
        r#"{"type": "object"}"#,
        r#"{"type": "object", "minProperties": 1}"#
    ));
}

#[test]
fn obj_max_property_fwd() {
    assert!(is_sub(
        r#"{"type": "object", "maxProperties": 3}"#,
        r#"{"type": "object"}"#
    ));
}

#[test]
fn obj_max_property_rev() {
    assert!(!is_sub(
        r#"{"type": "object"}"#,
        r#"{"type": "object", "maxProperties": 3}"#
    ));
}

#[test]
fn obj_min_max_property1_fwd() {
    assert!(is_sub(
        r#"{"type": "object", "minProperties": 1, "maxProperties": 3}"#,
        r#"{"type": "object"}"#
    ));
}

#[test]
fn obj_min_max_property1_rev() {
    assert!(!is_sub(
        r#"{"type": "object"}"#,
        r#"{"type": "object", "minProperties": 1, "maxProperties": 3}"#
    ));
}

#[test]
fn obj_min_max_property2_fwd() {
    assert!(is_sub(
        r#"{"type": "object", "minProperties": 1, "maxProperties": 3}"#,
        r#"{"type": "object", "maxProperties": 5}"#
    ));
}

#[test]
fn obj_min_max_property2_rev() {
    assert!(!is_sub(
        r#"{"type": "object", "maxProperties": 5}"#,
        r#"{"type": "object", "minProperties": 1, "maxProperties": 3}"#
    ));
}

#[test]
fn obj_min_max_property3_fwd() {
    // s2 has minProperties > maxProperties → uninhabited → subtype of anything
    assert!(!is_sub(
        r#"{"type": "object", "minProperties": 1, "maxProperties": 3}"#,
        r#"{"type": "object", "minProperties": 5, "maxProperties": 2}"#
    ));
}

#[test]
fn obj_min_max_property3_rev() {
    assert!(is_sub(
        r#"{"type": "object", "minProperties": 5, "maxProperties": 2}"#,
        r#"{"type": "object", "minProperties": 1, "maxProperties": 3}"#
    ));
}

#[test]
fn obj_min_max_property4_fwd() {
    assert!(!is_sub(
        r#"{"type": "object", "minProperties": 1, "maxProperties": 10}"#,
        r#"{"type": "object", "minProperties": 2, "maxProperties": 5}"#
    ));
}

#[test]
fn obj_min_max_property4_rev() {
    assert!(is_sub(
        r#"{"type": "object", "minProperties": 2, "maxProperties": 5}"#,
        r#"{"type": "object", "minProperties": 1, "maxProperties": 10}"#
    ));
}

#[test]
fn obj_required1_fwd() {
    assert!(!is_sub(
        r#"{"type": "object", "minProperties": 1}"#,
        r#"{"type": "object", "required": ["p1"]}"#
    ));
}

#[test]
fn obj_required1_rev() {
    assert!(is_sub(
        r#"{"type": "object", "required": ["p1"]}"#,
        r#"{"type": "object", "minProperties": 1}"#
    ));
}

#[test]
fn obj_required2_fwd() {
    assert!(!is_sub(
        r#"{"type": "object", "minProperties": 1}"#,
        r#"{"type": "object", "required": ["p1", "p2"]}"#
    ));
}

#[test]
fn obj_required2_rev() {
    assert!(is_sub(
        r#"{"type": "object", "required": ["p1", "p2"]}"#,
        r#"{"type": "object", "minProperties": 1}"#
    ));
}

#[test]
fn obj_required3_fwd() {
    assert!(!is_sub(
        r#"{"type": "object", "maxProperties": 1}"#,
        r#"{"type": "object", "required": ["p1", "p2"]}"#
    ));
}

#[test]
fn obj_required3_rev() {
    assert!(!is_sub(
        r#"{"type": "object", "required": ["p1", "p2"]}"#,
        r#"{"type": "object", "maxProperties": 1}"#
    ));
}

#[test]
fn obj_required4_fwd() {
    assert!(is_sub(
        r#"{"type": "object", "required": ["p2", "p1"]}"#,
        r#"{"type": "object", "required": ["p1", "p2"]}"#
    ));
}

#[test]
fn obj_required4_rev() {
    assert!(is_sub(
        r#"{"type": "object", "required": ["p1", "p2"]}"#,
        r#"{"type": "object", "required": ["p2", "p1"]}"#
    ));
}

#[test]
fn obj_required5_fwd() {
    assert!(!is_sub(
        r#"{"type": "object", "required": ["p1"]}"#,
        r#"{"type": "object", "required": ["p2"]}"#
    ));
}

#[test]
fn obj_required5_rev() {
    assert!(!is_sub(
        r#"{"type": "object", "required": ["p2"]}"#,
        r#"{"type": "object", "required": ["p1"]}"#
    ));
}

#[test]
fn obj_required6_fwd() {
    assert!(is_sub(
        r#"{"type": "object", "required": ["p1", "p2"]}"#,
        r#"{"type": "object", "required": ["p2"]}"#
    ));
}

#[test]
fn obj_required6_rev() {
    assert!(!is_sub(
        r#"{"type": "object", "required": ["p2"]}"#,
        r#"{"type": "object", "required": ["p1", "p2"]}"#
    ));
}

#[test]
fn obj_required7_fwd() {
    assert!(!is_sub(
        r#"{"type": "object", "required": ["p1", "p2"]}"#,
        r#"{"type": "object", "required": ["p2"], "additionalProperties": {"type": "boolean"}}"#
    ));
}

#[test]
fn obj_required7_rev() {
    assert!(!is_sub(
        r#"{"type": "object", "required": ["p2"], "additionalProperties": {"type": "boolean"}}"#,
        r#"{"type": "object", "required": ["p1", "p2"]}"#
    ));
}

#[test]
fn obj_simple1_fwd() {
    // s1 has email, s2 does not → s1 constrains more props → s1 <: s2
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            }
        }"#
    ));
}

#[test]
fn obj_simple1_rev() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            }
        }"#
    ));
}

#[test]
fn obj_simple2_fwd() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {
                "^b.*b$": {"type": "boolean"}
            }
        }"#
    ));
}

#[test]
fn obj_simple2_rev() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {
                "^b.*b$": {"type": "boolean"}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            }
        }"#
    ));
}

#[test]
fn obj_simple3_fwd() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"b.*b": {"type": "boolean"}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"^ba+b$": {"type": "boolean"}}
        }"#
    ));
}

#[test]
fn obj_simple3_rev() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"^ba+b$": {"type": "boolean"}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"b.*b": {"type": "boolean"}}
        }"#
    ));
}

#[test]
fn obj_simple4_fwd() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"b.*b": {"type": "integer"}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"^ba+b$": {"type": "boolean"}}
        }"#
    ));
}

#[test]
fn obj_simple4_rev() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"^ba+b$": {"type": "boolean"}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"b.*b": {"type": "integer"}}
        }"#
    ));
}

#[test]
fn obj_simple5_fwd() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"b.*b": {"type": "integer"}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"^b(\\w)+b$": {"type": "integer", "minimum": 10}}
        }"#
    ));
}

#[test]
fn obj_simple5_rev() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"^b(\\w)+b$": {"type": "integer", "minimum": 10}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"}
            },
            "patternProperties": {"b.*b": {"type": "integer"}}
        }"#
    ));
}

#[test]
fn obj_tricky1_fwd() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"^emai(l|k)$": {"type": "string"}},
            "required": ["name"]
        }"#
    ));
}

#[test]
fn obj_tricky1_rev() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"^emai(l|k)$": {"type": "string"}},
            "required": ["name"]
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            }
        }"#
    ));
}

#[test]
fn obj_tricky2_fwd() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"^emai(l|k)$": {"type": "string"}}
        }"#
    ));
}

#[test]
fn obj_tricky2_rev() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"^emai(l|k)$": {"type": "string"}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            }
        }"#
    ));
}

#[test]
fn obj_tricky3_fwd() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string"}}
        }"#
    ));
}

#[test]
fn obj_tricky3_rev() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string"}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            }
        }"#
    ));
}

#[test]
fn obj_tricky4_fwd() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            }
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string", "minLength": 10}}
        }"#
    ));
}

#[test]
fn obj_tricky4_rev() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string", "minLength": 10}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            }
        }"#
    ));
}

#[test]
fn obj_tricky5_fwd() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            },
            "additionalProperties": {"type": "boolean"}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string", "minLength": 10}}
        }"#
    ));
}

#[test]
fn obj_tricky5_rev() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string", "minLength": 10}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            },
            "additionalProperties": {"type": "boolean"}
        }"#
    ));
}

#[test]
fn obj_tricky6_fwd() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            },
            "additionalProperties": {"type": "boolean"}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string", "minLength": 10}},
            "additionalProperties": {"type": "boolean"}
        }"#
    ));
}

#[test]
fn obj_tricky6_rev() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string", "minLength": 10}},
            "additionalProperties": {"type": "boolean"}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            },
            "additionalProperties": {"type": "boolean"}
        }"#
    ));
}

#[test]
fn obj_tricky7_fwd() {
    assert!(is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            },
            "additionalProperties": {"type": "string"}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string"}}
        }"#
    ));
}

#[test]
fn obj_tricky7_rev() {
    assert!(!is_sub(
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]}
            },
            "patternProperties": {"emai": {"type": "string"}}
        }"#,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "gender": {"type": "string", "maxLength": 1, "enum": ["F", "M"]},
                "email": {"type": "string"},
                "emaik": {"type": "string"}
            },
            "additionalProperties": {"type": "string"}
        }"#
    ));
}

// Real-world schemas with nested arrays inside object properties.
// Draft-4 `items` (dict form) inside nested arrays is the same in 2020-12.
#[test]
fn obj_required_with_real_schema_fwd() {
    assert!(is_sub(
        r#"{
            "additionalProperties": false,
            "properties": {
                "X": {
                    "type": "array", "minItems": 150, "maxItems": 150,
                    "items": {"type": "array", "minItems": 4, "maxItems": 4, "items": {"type": "number"}}
                },
                "y": {
                    "type": "array", "minItems": 150, "maxItems": 150,
                    "items": {"type": "integer"}
                }
            },
            "required": ["X", "y"],
            "type": "object"
        }"#,
        r#"{
            "additionalProperties": false,
            "properties": {
                "X": {"type": "array", "items": {"type": "array", "items": {"type": "number"}}},
                "y": {"type": "array", "items": {"type": "number"}}
            },
            "required": ["X", "y"],
            "type": "object"
        }"#
    ));
}

#[test]
fn obj_required_with_real_schema_rev() {
    assert!(!is_sub(
        r#"{
            "additionalProperties": false,
            "properties": {
                "X": {"type": "array", "items": {"type": "array", "items": {"type": "number"}}},
                "y": {"type": "array", "items": {"type": "number"}}
            },
            "required": ["X", "y"],
            "type": "object"
        }"#,
        r#"{
            "additionalProperties": false,
            "properties": {
                "X": {
                    "type": "array", "minItems": 150, "maxItems": 150,
                    "items": {"type": "array", "minItems": 4, "maxItems": 4, "items": {"type": "number"}}
                },
                "y": {
                    "type": "array", "minItems": 150, "maxItems": 150,
                    "items": {"type": "integer"}
                }
            },
            "required": ["X", "y"],
            "type": "object"
        }"#
    ));
}

// Real-world iris dataset schema with prefixItems (draft-4 list-form items → 2020-12 prefixItems)
#[test]
fn obj_real_schema_fwd() {
    assert!(is_sub(
        r#"{
            "additionalProperties": false,
            "properties": {
                "X": {
                    "type": "array", "minItems": 120, "maxItems": 120,
                    "items": {
                        "type": "array", "minItems": 4, "maxItems": 4,
                        "prefixItems": [
                            {"type": "number"},
                            {"type": "number"},
                            {"type": "number"},
                            {"type": "number"}
                        ]
                    }
                },
                "y": {
                    "type": "array", "minItems": 120, "maxItems": 120,
                    "items": {"type": "integer"}
                }
            },
            "required": ["X", "y"],
            "type": "object"
        }"#,
        r#"{
            "additionalProperties": false,
            "properties": {
                "X": {"type": "array", "items": {"type": "array", "items": {"type": "number"}}},
                "y": {"type": "array", "items": {"type": "number"}}
            },
            "required": ["X", "y"],
            "type": "object"
        }"#
    ));
}

#[test]
fn obj_real_schema_rev() {
    assert!(!is_sub(
        r#"{
            "additionalProperties": false,
            "properties": {
                "X": {"type": "array", "items": {"type": "array", "items": {"type": "number"}}},
                "y": {"type": "array", "items": {"type": "number"}}
            },
            "required": ["X", "y"],
            "type": "object"
        }"#,
        r#"{
            "additionalProperties": false,
            "properties": {
                "X": {
                    "type": "array", "minItems": 120, "maxItems": 120,
                    "items": {
                        "type": "array", "minItems": 4, "maxItems": 4,
                        "prefixItems": [
                            {"type": "number"},
                            {"type": "number"},
                            {"type": "number"},
                            {"type": "number"}
                        ]
                    }
                },
                "y": {
                    "type": "array", "minItems": 120, "maxItems": 120,
                    "items": {"type": "integer"}
                }
            },
            "required": ["X", "y"],
            "type": "object"
        }"#
    ));
}

#[test]
fn obj_property_top1_fwd() {
    assert!(is_sub(
        r#"{"type": "object", "properties": {"name": {}, "age": {"type": "integer"}}}"#,
        r#"{"type": "object", "properties": {"age": {"type": "integer"}}}"#
    ));
}

#[test]
fn obj_property_top1_rev() {
    assert!(is_sub(
        r#"{"type": "object", "properties": {"age": {"type": "integer"}}}"#,
        r#"{"type": "object", "properties": {"name": {}, "age": {"type": "integer"}}}"#
    ));
}

#[test]
fn obj_property_top2_fwd() {
    assert!(is_sub(
        r#"{"type": "object", "properties": {"name": {"type": ["number","integer","string","boolean","object","array","null"]}, "age": {"type": "integer"}}}"#,
        r#"{"type": "object", "properties": {"age": {"type": "integer"}, "name": {}}}"#
    ));
}

#[test]
fn obj_property_top2_rev() {
    assert!(is_sub(
        r#"{"type": "object", "properties": {"age": {"type": "integer"}, "name": {}}}"#,
        r#"{"type": "object", "properties": {"name": {"type": ["number","integer","string","boolean","object","array","null"]}, "age": {"type": "integer"}}}"#
    ));
}
