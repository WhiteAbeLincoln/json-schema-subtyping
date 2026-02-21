use std::path::{Path, PathBuf};
use std::process::Command;

use libtest_mimic::{Arguments, Failed, Trial};
use serde::Deserialize;

use json_schema_subtyping::{SubtypeRelation, is_subtype};

// ---------------------------------------------------------------------------
// Data types — mirrors the structure defined in .test-config.json
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TestConfig {
    tests: Vec<TestSuite>,
}

#[derive(Deserialize)]
struct TestSuite {
    name: String,
    cases: Vec<TestCase>,
}

#[derive(Deserialize)]
struct TestCase {
    name: Option<String>,
    sup: SchemaOrSchemas,
    sub: SchemaOrSchemas,
    result: TestResult,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum SchemaOrSchemas {
    /// Must come before Single so serde tries array-of-values first.
    Multiple(Vec<serde_json::Value>),
    Single(serde_json::Value),
}

impl SchemaOrSchemas {
    fn as_slice(&self) -> &[serde_json::Value] {
        match self {
            SchemaOrSchemas::Single(v) => std::slice::from_ref(v),
            SchemaOrSchemas::Multiple(v) => v,
        }
    }

    fn is_array(&self) -> bool {
        matches!(self, SchemaOrSchemas::Multiple(_))
    }
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum TestResult {
    Pass(bool),
    #[allow(dead_code)]
    FailMessage(String),
}

impl TestResult {
    fn expected_subtype(&self) -> bool {
        match self {
            TestResult::Pass(b) => *b,
            TestResult::FailMessage(_) => false,
        }
    }
}

// ---------------------------------------------------------------------------
// JSON Schema validation
// ---------------------------------------------------------------------------

fn load_schema_validator(test_suite_dir: &Path) -> jsonschema::Validator {
    let schema_path = test_suite_dir.join(".test-config.json");
    let schema_str = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("Failed to read schema {}: {e}", schema_path.display()));
    let schema: serde_json::Value = serde_json::from_str(&schema_str)
        .unwrap_or_else(|e| panic!("Failed to parse schema {}: {e}", schema_path.display()));
    jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|e| panic!("Failed to compile schema {}: {e}", schema_path.display()))
}

fn validate_against_schema(
    validator: &jsonschema::Validator,
    instance: &serde_json::Value,
    file_name: &str,
) {
    let result = validator.validate(instance);
    if let Err(error) = result {
        let mut msg = format!("{file_name}: schema validation failed against .test-config.json:\n");
        // validate() returns the first error; also collect the rest via iter_errors
        msg.push_str(&format!("  - {error}\n"));
        for error in validator.iter_errors(instance).skip(1) {
            msg.push_str(&format!("  - {error}\n"));
        }
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// Caching
// ---------------------------------------------------------------------------

/// Returns the cache path for a given nix file.
/// e.g. `test-suite/object-applicators/properties.nix` → `test-suite/.cache/object-applicators/properties.json`
fn cache_path(test_suite_dir: &Path, nix_file: &Path) -> PathBuf {
    let rel = nix_file
        .strip_prefix(test_suite_dir)
        .expect("nix file is under test-suite dir");
    test_suite_dir
        .join(".cache")
        .join(rel)
        .with_extension("json")
}

/// Returns true if the cached JSON is up-to-date (exists and newer than the nix source).
fn cache_is_fresh(nix_file: &Path, cached: &Path) -> bool {
    let Ok(cache_meta) = std::fs::metadata(cached) else {
        return false;
    };
    let Ok(nix_meta) = std::fs::metadata(nix_file) else {
        return false;
    };
    let (Ok(cache_mtime), Ok(nix_mtime)) = (cache_meta.modified(), nix_meta.modified()) else {
        return false;
    };
    cache_mtime >= nix_mtime
}

fn write_cache(path: &Path, json_str: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            panic!("Failed to create cache directory {}: {e}", parent.display())
        });
    }
    std::fs::write(path, json_str)
        .unwrap_or_else(|e| panic!("Failed to write cache file {}: {e}", path.display()));
}

fn read_cache(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read cache file {}: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// Nix evaluation
// ---------------------------------------------------------------------------

fn nix_eval_to_json(nix_path: &Path) -> String {
    let abs_path = nix_path
        .canonicalize()
        .unwrap_or_else(|e| panic!("Failed to canonicalize {}: {e}", nix_path.display()));

    let expr = format!("builtins.toJSON (import {})", abs_path.display());

    let output = Command::new("nix")
        .args(["eval", "--raw", "--impure", "--expr", &expr])
        .output()
        .unwrap_or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                panic!(
                    "nix not found in PATH. Install Nix (https://nixos.org) to run these tests."
                );
            }
            panic!("Failed to run nix: {e}");
        });

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("nix eval failed for {}:\n{stderr}", nix_path.display());
    }

    String::from_utf8(output.stdout)
        .unwrap_or_else(|e| panic!("nix eval produced invalid UTF-8: {e}"))
}

// ---------------------------------------------------------------------------
// Test discovery
// ---------------------------------------------------------------------------

fn discover_nix_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    discover_nix_files_recursive(dir, &mut files);
    files.sort();
    files
}

fn discover_nix_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Failed to read directory {}: {e}", dir.display()));

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            discover_nix_files_recursive(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "nix") {
            files.push(path);
        }
    }
}

/// Compute the relative stem from the test-suite directory.
/// e.g. `test-suite/object-applicators/properties.nix` → `object-applicators/properties`
fn relative_stem(base: &Path, file: &Path) -> String {
    let rel = file
        .strip_prefix(base)
        .unwrap_or_else(|_| panic!("{} is not under {}", file.display(), base.display()));
    rel.with_extension("").to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// Test expansion
// ---------------------------------------------------------------------------

fn expand_trials(test_suite_dir: &Path) -> Vec<Trial> {
    let nix_files = discover_nix_files(test_suite_dir);
    // Only compile the schema validator if we actually need to run nix eval
    // (i.e. at least one file has a stale or missing cache).
    let mut validator: Option<jsonschema::Validator> = None;
    let mut trials = Vec::new();

    for nix_file in &nix_files {
        let file_stem = relative_stem(test_suite_dir, nix_file);
        let cached = cache_path(test_suite_dir, nix_file);

        let json_str = if cache_is_fresh(nix_file, &cached) {
            read_cache(&cached)
        } else {
            let json_str = nix_eval_to_json(nix_file);

            let json_value: serde_json::Value =
                serde_json::from_str(&json_str).unwrap_or_else(|e| {
                    panic!(
                        "Failed to parse JSON from {}: {e}\nJSON:\n{json_str}",
                        nix_file.display()
                    );
                });

            let v = validator.get_or_insert_with(|| load_schema_validator(test_suite_dir));
            validate_against_schema(v, &json_value, &file_stem);

            write_cache(&cached, &json_str);
            json_str
        };

        let config: TestConfig = serde_json::from_str(&json_str).unwrap_or_else(|e| {
            panic!("Failed to deserialize {}: {e}", nix_file.display());
        });

        for suite in &config.tests {
            let suite_name = &suite.name;

            for (case_idx, case) in suite.cases.iter().enumerate() {
                let case_name = case
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("case_{case_idx}"));

                let sups = case.sup.as_slice();
                let subs = case.sub.as_slice();
                let sup_is_array = case.sup.is_array();
                let sub_is_array = case.sub.is_array();
                let needs_index = sup_is_array || sub_is_array;

                for (si, sup_val) in sups.iter().enumerate() {
                    for (sj, sub_val) in subs.iter().enumerate() {
                        let test_name = if needs_index {
                            format!("{file_stem}::{suite_name}::{case_name}[sup={si},sub={sj}]")
                        } else {
                            format!("{file_stem}::{suite_name}::{case_name}")
                        };

                        let expected = case.result.expected_subtype();
                        let sup_json = sup_val.clone();
                        let sub_json = sub_val.clone();

                        trials.push(Trial::test(test_name, move || {
                            run_subtype_test(&sup_json, &sub_json, expected)
                        }));
                    }
                }
            }
        }
    }

    trials
}

// ---------------------------------------------------------------------------
// Test body
// ---------------------------------------------------------------------------

fn run_subtype_test(
    sup: &serde_json::Value,
    sub: &serde_json::Value,
    expected_subtype: bool,
) -> Result<(), Failed> {
    match is_subtype(sup, sub) {
        Ok(SubtypeRelation::Subtype) => {
            if expected_subtype {
                Ok(())
            } else {
                Err(
                    format!("Expected NOT subtype, but got Subtype\n  sup: {sup}\n  sub: {sub}")
                        .into(),
                )
            }
        }
        Ok(SubtypeRelation::NotSubtype(_)) => {
            if expected_subtype {
                Err(
                    format!("Expected Subtype, but got NotSubtype\n  sup: {sup}\n  sub: {sub}")
                        .into(),
                )
            } else {
                Ok(())
            }
        }
        Err(e) => Err(format!("is_subtype returned error: {e}\n  sup: {sup}\n  sub: {sub}").into()),
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let args = Arguments::from_args();

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let test_suite_dir = Path::new(manifest_dir).join("test-suite");

    let trials = expand_trials(&test_suite_dir);

    libtest_mimic::run(&args, trials).exit();
}
