use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Deserialize)]
pub struct TestGroup {
    pub description: String,
    pub schema: Value,
    pub tests: Vec<TestCase>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct TestCase {
    pub description: String,
    pub data: Value,
    pub valid: bool,
}

/// Load all test groups from a JSON Schema Test Suite test file.
pub fn load_test_file(path: &Path) -> Vec<TestGroup> {
    let content = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&content).unwrap()
}

/// Build a validator for a schema, returning None if the schema can't be compiled
/// (e.g. uses remote $ref that isn't available).
pub fn build_validator(schema: &Value) -> Option<jsonschema::Validator> {
    jsonschema::validator_for(schema).ok()
}
