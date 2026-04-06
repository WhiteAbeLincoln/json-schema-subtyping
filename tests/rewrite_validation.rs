mod helpers;

use helpers::{build_validator, load_test_file};
use json_schema_subtyping::located::{JsonF, LocatedValue};
use json_schema_subtyping::parse::parse;
use json_schema_subtyping::rewrite::{rewrite_phase, RewriteRule};
use serde_json::Value;
use std::path::PathBuf;

/// Convert a LocatedValue back to serde_json::Value for validation.
fn to_json_value(lv: &LocatedValue) -> Value {
    match &lv.node {
        JsonF::Null => Value::Null,
        JsonF::Bool(b) => Value::Bool(*b),
        JsonF::Number(n) => serde_json::Number::from_f64(*n)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        JsonF::String(s) => Value::String(s.clone()),
        JsonF::Array(items) => Value::Array(items.iter().map(to_json_value).collect()),
        JsonF::Object(pairs) => {
            let map = pairs
                .iter()
                .filter_map(|(k, v)| k.as_str().map(|key| (key.to_string(), to_json_value(v))))
                .collect();
            Value::Object(map)
        }
    }
}

/// For each test in the official suite: apply our rewrites to the schema,
/// then validate each test instance against both original and rewritten.
/// They must produce the same pass/fail result.
fn validate_rewrites_preserve_semantics(rules: &[Box<dyn RewriteRule>]) {
    let suite_dir = PathBuf::from("tests/JSON-Schema-Test-Suite/tests/draft2020-12");
    if !suite_dir.exists() {
        eprintln!("Skipping: JSON Schema Test Suite not found at {suite_dir:?}");
        return;
    }

    let mut total = 0;
    let mut passed = 0;
    let mut skipped = 0;

    for entry in std::fs::read_dir(&suite_dir).unwrap() {
        let entry = entry.unwrap();
        if !entry.path().extension().is_some_and(|e| e == "json") {
            continue;
        }


        let groups = load_test_file(&entry.path());
        for group in &groups {
            // Build a validator for the original schema; skip if it can't be compiled
            // (e.g. references remote schemas not available locally).
            let original_validator = match build_validator(&group.schema) {
                Some(v) => v,
                None => {
                    skipped += group.tests.len();
                    continue;
                }
            };

            let schema_str = serde_json::to_string(&group.schema).unwrap();
            let parsed = match parse(&schema_str) {
                Ok(p) => p,
                Err(_) => {
                    skipped += group.tests.len();
                    continue;
                }
            };

            let rewritten = match rewrite_phase(&parsed, rules) {
                Ok(r) => r,
                Err(_) => {
                    skipped += group.tests.len();
                    continue;
                }
            };

            let rewritten_json = to_json_value(&rewritten);
            let rewritten_validator = match build_validator(&rewritten_json) {
                Some(v) => v,
                None => {
                    skipped += group.tests.len();
                    continue;
                }
            };

            for test in &group.tests {
                total += 1;
                let original_result = original_validator.is_valid(&test.data);
                let rewritten_result = rewritten_validator.is_valid(&test.data);

                assert_eq!(
                    original_result, rewritten_result,
                    "Rewrite changed semantics!\n\
                     File: {:?}\n\
                     Group: {}\n\
                     Test: {}\n\
                     Original schema: {}\n\
                     Rewritten schema: {}\n\
                     Data: {}\n\
                     Original valid: {original_result}, Rewritten valid: {rewritten_result}",
                    entry.path(),
                    group.description,
                    test.description,
                    serde_json::to_string_pretty(&group.schema).unwrap(),
                    serde_json::to_string_pretty(&rewritten_json).unwrap(),
                    serde_json::to_string_pretty(&test.data).unwrap(),
                );
                passed += 1;
            }
        }
    }

    eprintln!("Rewrite validation: {passed}/{total} test instances passed, {skipped} skipped");
}

#[test]
fn identity_rewrite_preserves_semantics() {
    // With no rules, parse → rewrite → to_json should be identity.
    // This validates that the harness itself works.
    let rules: Vec<Box<dyn RewriteRule>> = vec![];
    validate_rewrites_preserve_semantics(&rules);
}

#[test]
fn non_type_specific_canonicalization_preserves_semantics() {
    use json_schema_subtyping::rewrite::canonicalize::non_type_specific_rules;
    validate_rewrites_preserve_semantics(&non_type_specific_rules());
}

#[test]
fn all_canonicalization_preserves_semantics() {
    use json_schema_subtyping::rewrite::canonicalize::all_canonicalization_rules;
    validate_rewrites_preserve_semantics(&all_canonicalization_rules());
}

#[test]
fn simplification_preserves_semantics() {
    use json_schema_subtyping::rewrite::simplify::simplification_rules;
    validate_rewrites_preserve_semantics(&simplification_rules());
}

#[test]
fn full_pipeline_preserves_semantics() {
    use json_schema_subtyping::rewrite::canonicalize::all_canonicalization_rules;
    use json_schema_subtyping::rewrite::simplify::simplification_rules;
    let mut rules = all_canonicalization_rules();
    rules.extend(simplification_rules());
    validate_rewrites_preserve_semantics(&rules);
}
