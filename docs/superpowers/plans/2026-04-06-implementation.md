# JSON Schema Subtyping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a Rust library that checks subtyping between JSON Schemas (draft 2020-12) using canonicalization, simplification, and inference — with a pluggable API for custom rewrite rules and vocabulary extensions.

**Architecture:** Parse JSON into a location-annotated tree (`LocatedValue` as Cofree over `JsonF`), apply per-node rewrite rules in three fixed phases (draft conversion, canonicalization, simplification) bottom-up to fixed point, extract into a typed reduced schema IR, then check subtype relationships using type-directed inference rules. Extension vocabularies register typed extractors and inference rules via `dyn Any` slots.

**Tech Stack:** Rust (edition 2024), `jsonc-parser` (JSON parsing with spans), `recursion` (recursion schemes), `smallvec` (inline provenance storage), `bitflags` (TypeSet), `regex-automata` + `regex-syntax` (regex algebra for string schemas), `miette` + `snafu` (error reporting), `libtest-mimic` (runtime test discovery).

**Reference:** The algorithm is from [Type Safety with JSON Subschema](https://arxiv.org/abs/1911.12651), adapted from draft-04 to draft 2020-12. The paper's TeX source is in `references/type-safety-with-json-subschema-tex/sections/algorithm.tex`.

---

## Key Differences: Draft-04 (Paper) vs Draft 2020-12 (Ours)

The paper targets draft-04. Our implementation targets 2020-12. Key adaptations:

| Draft-04 (paper) | Draft 2020-12 (ours) | Impact on rules |
|---|---|---|
| `exclusiveMinimum: true` (boolean) | `exclusiveMinimum: 5` (number) | No boolean-to-value conversion needed |
| `items` as array or schema | `prefixItems` (array), `items` (schema) | Array canonicalization simplified |
| `additionalItems` | `items` (when `prefixItems` present) | Keyword rename |
| `dependencies` (mixed) | `dependentSchemas` + `dependentRequired` | Already split, simpler rules |
| No `if/then/else` | `if/then/else` supported | New canonicalization rule needed |
| No `const` | `const` keyword | Canonicalize `const: v` → `enum: [v]` |
| No `contains` | `contains`, `minContains`, `maxContains` | New array keywords in IR |
| No `propertyNames` | `propertyNames` keyword | New object keyword in IR |

## File Structure

```
src/
  lib.rs                  — Public API: SubtypeChecker, Builder, Preset, re-exports
  error.rs                — SubtypeError, SubtypeRelation, DetailedOutput

  located/
    mod.rs                — Re-exports
    span.rs               — Span, JsonPointer, Provenance
    value.rs              — JsonF<A>, LocatedValue, map/traverse helpers
    located.rs            — Located<T> newtype wrapper

  parse.rs                — JSON string → LocatedValue via jsonc-parser

  rewrite/
    mod.rs                — Phase, RewriteRule trait, bottom-up fixed-point engine
    ref_resolution.rs     — $ref resolution (internal whole-tree pre-pass)
    canonicalize.rs       — All canonicalization rules (§4.1 of paper)
    simplify.rs           — All simplification rules (§4.2 of paper)

  regex_algebra.rs        — Regex intersection, complement, containment, union via DFA

  schema/
    mod.rs                — Re-exports
    typeset.rs            — TypeSet bitflags (7 JSON types)
    boolset.rs            — BoolSet for boolean enum values
    reduced.rs            — ReducedSchema, TypedSchema, and per-type schema structs
    extract.rs            — LocatedValue → ReducedSchema extraction

  check/
    mod.rs                — is_subtype entry point, SubtypeContext
    inhabited.rs          — inhabited(schema) predicate
    primitive.rs          — null, boolean, string subtype checks
    number.rs             — number subtype (subRange from paper)
    array.rs              — array subtype check
    object.rs             — object subtype check
    connective.rs         — anyOf, allOfNot subtype checks

  extension.rs            — VocabExtension trait, ExtensionSlot, ExtensionData
  preset.rs               — Preset struct, Preset::draft_2020_12()

tests/
  helpers/
    mod.rs                — Test utilities: parse, structural comparison, macros
  rewrite_validation.rs   — Validate rewrites against official JSON Schema test suite
  subtype_suite.rs        — Subtype test suite (recovered from git + new cases)
```

## Draft 2020-12 Keyword Reference

For implementers — the complete keyword sets by type, with defaults. This drives the `missing_keyword` and `irrelevant_keywords` rules.

```
kw(string)  = {minLength: 0, maxLength: ∞, pattern: ""}
kw(number)  = {minimum: -∞, maximum: ∞, exclusiveMinimum: -∞, exclusiveMaximum: ∞, multipleOf: (absent)}
kw(integer) = same as number (integer is rewritten to number + multipleOf)
kw(array)   = {prefixItems: [], items: {}, minItems: 0, maxItems: ∞, uniqueItems: false,
               contains: (absent), minContains: 1, maxContains: ∞}
kw(object)  = {properties: {}, patternProperties: {}, additionalProperties: {},
               minProperties: 0, maxProperties: ∞, required: [],
               dependentRequired: {}, dependentSchemas: {}, propertyNames: {}}
kw(boolean) = {} (boolean has no type-specific validation keywords)
kw(null)    = {} (null has no type-specific validation keywords)
```

---

### Task 1: Project Setup and Dependencies

**Files:**
- Modify: `Cargo.toml`
- Create: `src/error.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add dependencies to Cargo.toml**

```toml
[dependencies]
bitflags = "2"
jsonc-parser = { version = "0.29.0" }
miette = { version = "7.6.0", features = ["fancy"] }
recursion = "0.6"
smallvec = "1"
snafu = "0.8.9"
regex-syntax = "0.8"
regex-automata = { version = "0.4", features = ["dfa-build", "dfa-search", "nfa-thompson"] }

[dev-dependencies]
libtest-mimic = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Create `src/error.rs` with error types**

```rust
use std::ops::Range;

use snafu::Snafu;

/// Represents the result of a subtype check.
#[derive(Debug)]
pub enum SubtypeRelation {
    /// `sub` is a subtype of `sup`.
    Subtype,
    /// `sub` is not a subtype of `sup`, with details about the failure.
    NotSubtype(DetailedOutput),
}

impl SubtypeRelation {
    pub fn is_subtype(&self) -> bool {
        matches!(self, SubtypeRelation::Subtype)
    }
}

/// Details about why a subtype check failed.
#[derive(Debug)]
pub struct DetailedOutput {
    pub message: String,
    // TODO: sub_locations and sup_locations with Provenance
    // will be filled in when Provenance type is available
}

#[derive(Debug, Snafu)]
pub enum SubtypeError {
    #[snafu(display("{source}"))]
    InvalidSchema {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[snafu(display("Unsupported schema features: {details}"))]
    UnsupportedFeatures { details: String },
    #[snafu(display("Rewrite failed: {message}"))]
    RewriteFailed { message: String },
}
```

- [ ] **Step 3: Update `src/lib.rs` to declare modules**

```rust
pub mod error;

pub use error::{DetailedOutput, SubtypeError, SubtypeRelation};
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/error.rs src/lib.rs
git commit -m "Add dependencies and error types"
```

---

### Task 2: Core Located Types

**Files:**
- Create: `src/located/mod.rs`
- Create: `src/located/span.rs`
- Create: `src/located/located.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write tests for Provenance**

Create `src/located/span.rs`:

```rust
use smallvec::SmallVec;

/// A byte offset range in the original source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A JSON Pointer path (RFC 6901), e.g., "/properties/name/type".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPointer(pub Vec<String>);

impl JsonPointer {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn push(&self, segment: impl Into<String>) -> Self {
        let mut p = self.0.clone();
        p.push(segment.into());
        Self(p)
    }
}

impl std::fmt::Display for JsonPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "")
        } else {
            for segment in &self.0 {
                write!(f, "/{segment}")?;
            }
            Ok(())
        }
    }
}

/// Tracks where a value came from in the original source.
/// A single value may be derived from multiple source locations
/// (e.g., when a rewrite rule merges multiple keywords).
#[derive(Debug, Clone)]
pub struct Provenance {
    pub spans: SmallVec<[Span; 1]>,
    pub pointers: SmallVec<[JsonPointer; 1]>,
}

impl Provenance {
    pub fn new(span: Span, pointer: JsonPointer) -> Self {
        Self {
            spans: SmallVec::from_elem(span, 1),
            pointers: SmallVec::from_elem(pointer, 1),
        }
    }

    /// A synthetic provenance with no source location.
    pub fn synthetic() -> Self {
        Self {
            spans: SmallVec::new(),
            pointers: SmallVec::new(),
        }
    }

    /// Merge two provenances (when a rewrite combines multiple sources).
    pub fn merge(&self, other: &Provenance) -> Self {
        let mut spans = self.spans.clone();
        spans.extend_from_slice(&other.spans);
        let mut pointers = self.pointers.clone();
        pointers.extend_from_slice(&other.pointers);
        Self { spans, pointers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_pointer_display() {
        assert_eq!(JsonPointer::root().to_string(), "");
        assert_eq!(
            JsonPointer::root().push("properties").push("name").to_string(),
            "/properties/name"
        );
    }

    #[test]
    fn provenance_merge() {
        let a = Provenance::new(Span::new(0, 5), JsonPointer::root().push("a"));
        let b = Provenance::new(Span::new(10, 15), JsonPointer::root().push("b"));
        let merged = a.merge(&b);
        assert_eq!(merged.spans.len(), 2);
        assert_eq!(merged.pointers.len(), 2);
    }
}
```

- [ ] **Step 2: Create `src/located/located.rs`**

```rust
use super::span::Provenance;

/// A value annotated with source provenance.
#[derive(Debug, Clone)]
pub struct Located<T> {
    pub provenance: Provenance,
    pub value: T,
}

impl<T> Located<T> {
    pub fn new(provenance: Provenance, value: T) -> Self {
        Self { provenance, value }
    }

    pub fn synthetic(value: T) -> Self {
        Self {
            provenance: Provenance::synthetic(),
            value,
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Located<U> {
        Located {
            provenance: self.provenance,
            value: f(self.value),
        }
    }
}
```

- [ ] **Step 3: Create `src/located/mod.rs`**

```rust
mod located;
mod span;

pub use located::Located;
pub use span::{JsonPointer, Provenance, Span};
```

- [ ] **Step 4: Wire into `src/lib.rs` and run tests**

Add `pub mod located;` to `src/lib.rs`.

Run: `cargo test`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/located/
git commit -m "Add core located types: Span, Provenance, JsonPointer, Located<T>"
```

---

### Task 3: JsonF Base Functor and LocatedValue

**Files:**
- Create: `src/located/value.rs`
- Modify: `src/located/mod.rs`

- [ ] **Step 1: Create `src/located/value.rs` with JsonF and LocatedValue**

```rust
use super::span::Provenance;

/// One layer of JSON, parameterized over the recursive child type.
/// This is the base functor for our recursion scheme.
#[derive(Debug, Clone)]
pub enum JsonF<A> {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<A>),
    /// Key-value pairs. Keys are always JsonF::String nodes but carry
    /// their own provenance as LocatedValue when A = LocatedValue.
    Object(Vec<(A, A)>),
}

impl<A> JsonF<A> {
    /// Functor map — transform all children.
    pub fn map<B>(self, mut f: impl FnMut(A) -> B) -> JsonF<B> {
        match self {
            JsonF::Null => JsonF::Null,
            JsonF::Bool(b) => JsonF::Bool(b),
            JsonF::Number(n) => JsonF::Number(n),
            JsonF::String(s) => JsonF::String(s),
            JsonF::Array(items) => JsonF::Array(items.into_iter().map(&mut f).collect()),
            JsonF::Object(pairs) => JsonF::Object(
                pairs.into_iter().map(|(k, v)| (f(k), f(v))).collect(),
            ),
        }
    }

    /// Fallible functor map — transform all children, short-circuiting on error.
    pub fn try_map<B, E>(self, mut f: impl FnMut(A) -> Result<B, E>) -> Result<JsonF<B>, E> {
        Ok(match self {
            JsonF::Null => JsonF::Null,
            JsonF::Bool(b) => JsonF::Bool(b),
            JsonF::Number(n) => JsonF::Number(n),
            JsonF::String(s) => JsonF::String(s),
            JsonF::Array(items) => {
                JsonF::Array(items.into_iter().map(&mut f).collect::<Result<_, _>>()?)
            }
            JsonF::Object(pairs) => JsonF::Object(
                pairs
                    .into_iter()
                    .map(|(k, v)| Ok((f(k)?, f(v)?)))
                    .collect::<Result<_, _>>()?,
            ),
        })
    }
}

/// The recursive located JSON tree.
/// Equivalent to Cofree JsonF Provenance — each node carries provenance
/// (source spans + JSON pointers) plus one layer of JSON structure.
#[derive(Debug, Clone)]
pub struct LocatedValue {
    pub provenance: Provenance,
    pub node: JsonF<LocatedValue>,
}

impl LocatedValue {
    pub fn new(provenance: Provenance, node: JsonF<LocatedValue>) -> Self {
        Self { provenance, node }
    }

    /// Create a synthetic (no source location) LocatedValue.
    pub fn synthetic(node: JsonF<LocatedValue>) -> Self {
        Self {
            provenance: Provenance::synthetic(),
            node,
        }
    }

    /// Helper: is this node a JSON object?
    pub fn as_object(&self) -> Option<&[(LocatedValue, LocatedValue)]> {
        match &self.node {
            JsonF::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    /// Helper: is this node a JSON string?
    pub fn as_str(&self) -> Option<&str> {
        match &self.node {
            JsonF::String(s) => Some(s),
            _ => None,
        }
    }

    /// Helper: is this node a JSON number?
    pub fn as_number(&self) -> Option<f64> {
        match &self.node {
            JsonF::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Helper: is this node a JSON bool?
    pub fn as_bool(&self) -> Option<bool> {
        match &self.node {
            JsonF::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Helper: is this node a JSON array?
    pub fn as_array(&self) -> Option<&[LocatedValue]> {
        match &self.node {
            JsonF::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Helper: is this node JSON null?
    pub fn is_null(&self) -> bool {
        matches!(&self.node, JsonF::Null)
    }

    /// Look up a key in an object node. Returns None if not an object or key not found.
    pub fn get_key(&self, key: &str) -> Option<&LocatedValue> {
        self.as_object()?.iter().find_map(|(k, v)| {
            if k.as_str() == Some(key) {
                Some(v)
            } else {
                None
            }
        })
    }

    /// Structural equality ignoring provenance. Useful for tests.
    pub fn structural_eq(&self, other: &LocatedValue) -> bool {
        match (&self.node, &other.node) {
            (JsonF::Null, JsonF::Null) => true,
            (JsonF::Bool(a), JsonF::Bool(b)) => a == b,
            (JsonF::Number(a), JsonF::Number(b)) => a == b,
            (JsonF::String(a), JsonF::String(b)) => a == b,
            (JsonF::Array(a), JsonF::Array(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|(x, y)| x.structural_eq(y))
            }
            (JsonF::Object(a), JsonF::Object(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|((k1, v1), (k2, v2))| {
                        k1.structural_eq(k2) && v1.structural_eq(v2)
                    })
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonf_map() {
        let node: JsonF<i32> = JsonF::Array(vec![1, 2, 3]);
        let doubled = node.map(|x| x * 2);
        match doubled {
            JsonF::Array(v) => assert_eq!(v, vec![2, 4, 6]),
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn located_value_get_key() {
        let key = LocatedValue::synthetic(JsonF::String("name".into()));
        let val = LocatedValue::synthetic(JsonF::String("test".into()));
        let obj = LocatedValue::synthetic(JsonF::Object(vec![(key, val)]));
        assert_eq!(obj.get_key("name").unwrap().as_str(), Some("test"));
        assert!(obj.get_key("missing").is_none());
    }

    #[test]
    fn structural_eq_ignores_provenance() {
        let a = LocatedValue::new(
            Provenance::new(
                crate::located::Span::new(0, 5),
                crate::located::JsonPointer::root(),
            ),
            JsonF::Number(42.0),
        );
        let b = LocatedValue::synthetic(JsonF::Number(42.0));
        assert!(a.structural_eq(&b));
    }
}
```

- [ ] **Step 2: Update `src/located/mod.rs`**

```rust
mod located;
mod span;
mod value;

pub use located::Located;
pub use span::{JsonPointer, Provenance, Span};
pub use value::{JsonF, LocatedValue};
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass (prior + 3 new).

- [ ] **Step 4: Commit**

```bash
git add src/located/
git commit -m "Add JsonF base functor and LocatedValue (Cofree)"
```

---

### Task 4: JSON Parser

**Files:**
- Create: `src/parse.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write parser tests**

The parser converts a JSON string into a `LocatedValue` tree with accurate spans and JSON pointers. Add tests to `src/parse.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::{JsonF, JsonPointer};

    #[test]
    fn parse_number() {
        let v = parse("42").unwrap();
        assert_eq!(v.as_number(), Some(42.0));
        assert_eq!(v.provenance.spans[0], Span::new(0, 2));
    }

    #[test]
    fn parse_string() {
        let v = parse(r#""hello""#).unwrap();
        assert_eq!(v.as_str(), Some("hello"));
    }

    #[test]
    fn parse_object_with_pointers() {
        let v = parse(r#"{"a": 1, "b": "two"}"#).unwrap();
        let a_val = v.get_key("a").unwrap();
        assert_eq!(a_val.as_number(), Some(1.0));
        assert_eq!(a_val.provenance.pointers[0], JsonPointer::root().push("a"));
    }

    #[test]
    fn parse_nested_object() {
        let v = parse(r#"{"outer": {"inner": true}}"#).unwrap();
        let inner = v.get_key("outer").unwrap().get_key("inner").unwrap();
        assert_eq!(inner.as_bool(), Some(true));
        assert_eq!(
            inner.provenance.pointers[0],
            JsonPointer::root().push("outer").push("inner")
        );
    }

    #[test]
    fn parse_array() {
        let v = parse("[1, 2, 3]").unwrap();
        let items = v.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(
            items[1].provenance.pointers[0],
            JsonPointer::root().push("1")
        );
    }

    #[test]
    fn parse_true_false_null() {
        assert!(parse("true").unwrap().as_bool() == Some(true));
        assert!(parse("false").unwrap().as_bool() == Some(false));
        assert!(parse("null").unwrap().is_null());
    }

    #[test]
    fn parse_invalid_json() {
        assert!(parse("{invalid}").is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test parse`
Expected: FAIL — `parse` function doesn't exist yet.

- [ ] **Step 3: Implement parser using jsonc-parser**

```rust
use crate::error::SubtypeError;
use crate::located::{JsonF, JsonPointer, LocatedValue, Provenance, Span};

/// Parse a JSON string into a LocatedValue tree.
/// Every node carries its source span and JSON pointer path.
pub fn parse(source: &str) -> Result<LocatedValue, SubtypeError> {
    let parsed = jsonc_parser::parse_to_ast(
        source,
        &Default::default(),
        &Default::default(),
    )
    .map_err(|e| SubtypeError::InvalidSchema {
        source: Box::new(e),
    })?;

    match parsed.value {
        Some(value) => convert_value(&value, JsonPointer::root()),
        None => Err(SubtypeError::InvalidSchema {
            source: "Empty input".into(),
        }),
    }
}

fn convert_value(
    value: &jsonc_parser::ast::Value,
    pointer: JsonPointer,
) -> Result<LocatedValue, SubtypeError> {
    let span = Span::new(value.range().start, value.range().end);
    let prov = Provenance::new(span, pointer);

    let node = match value {
        jsonc_parser::ast::Value::NullKeyword(_) => JsonF::Null,
        jsonc_parser::ast::Value::BooleanLit(b) => JsonF::Bool(b.value),
        jsonc_parser::ast::Value::NumberLit(n) => {
            let num: f64 = n.value.parse().map_err(|e: std::num::ParseFloatError| {
                SubtypeError::InvalidSchema { source: Box::new(e) }
            })?;
            JsonF::Number(num)
        }
        jsonc_parser::ast::Value::StringLit(s) => JsonF::String(s.value.to_string()),
        jsonc_parser::ast::Value::Array(arr) => {
            let items = arr
                .elements
                .iter()
                .enumerate()
                .map(|(i, elem)| convert_value(elem, prov.pointers[0].push(i.to_string())))
                .collect::<Result<Vec<_>, _>>()?;
            JsonF::Array(items)
        }
        jsonc_parser::ast::Value::Object(obj) => {
            let pairs = obj
                .properties
                .iter()
                .map(|prop| {
                    let key_span = Span::new(prop.name.range().start, prop.name.range().end);
                    let child_pointer = prov.pointers[0].push(prop.name.value.as_ref());
                    let key = LocatedValue::new(
                        Provenance::new(key_span, child_pointer.clone()),
                        JsonF::String(prop.name.value.to_string()),
                    );
                    let val = convert_value(&prop.value, child_pointer)?;
                    Ok((key, val))
                })
                .collect::<Result<Vec<_>, SubtypeError>>()?;
            JsonF::Object(pairs)
        }
    };

    Ok(LocatedValue::new(prov, node))
}
```

Note: The exact `jsonc-parser` AST API may differ slightly — check the crate docs and adapt field names as needed. The structure above captures the intent: walk the AST, build `LocatedValue` nodes with spans from the AST ranges and JSON pointers computed during traversal.

- [ ] **Step 4: Run tests**

Run: `cargo test parse`
Expected: All 7 tests pass.

- [ ] **Step 5: Add to `src/lib.rs` and commit**

Add `pub mod parse;` to `src/lib.rs`.

```bash
git add src/parse.rs src/lib.rs
git commit -m "Add JSON parser: string → LocatedValue with spans and JSON pointers"
```

---

### Task 5: Rewrite Engine Infrastructure

**Files:**
- Create: `src/rewrite/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write rewrite engine tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::located::{JsonF, LocatedValue};

    /// A test rule that doubles all numbers.
    struct DoubleNumbers;
    impl RewriteRule for DoubleNumbers {
        fn rewrite(
            &self,
            _prov: &Provenance,
            node: &JsonF<LocatedValue>,
        ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
            if let JsonF::Number(n) = node {
                Ok(Some(JsonF::Number(n * 2.0)))
            } else {
                Ok(None)
            }
        }
    }

    #[test]
    fn rewrite_bottom_up_transforms_leaves() {
        let tree = crate::parse::parse(r#"{"a": 5, "b": [1, 2]}"#).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(DoubleNumbers)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert_eq!(result.get_key("a").unwrap().as_number(), Some(10.0));
        let arr = result.get_key("b").unwrap().as_array().unwrap();
        assert_eq!(arr[0].as_number(), Some(2.0));
        assert_eq!(arr[1].as_number(), Some(4.0));
    }

    /// A test rule that fires twice: wraps number in array, then the
    /// identity. Tests that fixed-point stops after the first change.
    struct WrapOnce;
    impl RewriteRule for WrapOnce {
        fn rewrite(
            &self,
            prov: &Provenance,
            node: &JsonF<LocatedValue>,
        ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
            if let JsonF::Number(_) = node {
                // Wrap number in a single-element array
                let inner = LocatedValue::new(prov.clone(), node.clone());
                Ok(Some(JsonF::Array(vec![inner])))
            } else {
                Ok(None)
            }
        }
    }

    #[test]
    fn rewrite_fixed_point_stops_when_no_rule_fires() {
        let tree = crate::parse::parse("42").unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(WrapOnce)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        // Number wrapped in array, then WrapOnce doesn't fire on the array
        assert!(result.as_array().is_some());
        let inner = &result.as_array().unwrap()[0];
        assert_eq!(inner.as_number(), Some(42.0));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test rewrite`
Expected: FAIL.

- [ ] **Step 3: Implement rewrite engine**

```rust
use crate::error::SubtypeError;
use crate::located::{JsonF, LocatedValue, Provenance};

/// The three fixed rewrite phases, run in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    DraftConversion,
    Canonicalization,
    Simplification,
}

/// A per-node rewrite rule. See design spec for full docs.
pub trait RewriteRule: 'static {
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError>;
}

/// Apply all rules in a phase to a tree, bottom-up, to fixed point per node.
pub fn rewrite_phase(
    tree: &LocatedValue,
    rules: &[Box<dyn RewriteRule>],
) -> Result<LocatedValue, SubtypeError> {
    // Bottom-up: first rewrite all children
    let rewritten_children = match &tree.node {
        JsonF::Null | JsonF::Bool(_) | JsonF::Number(_) | JsonF::String(_) => tree.node.clone(),
        JsonF::Array(items) => {
            let new_items = items
                .iter()
                .map(|item| rewrite_phase(item, rules))
                .collect::<Result<Vec<_>, _>>()?;
            JsonF::Array(new_items)
        }
        JsonF::Object(pairs) => {
            let new_pairs = pairs
                .iter()
                .map(|(k, v)| {
                    // Keys are not rewritten (they're just strings)
                    Ok((k.clone(), rewrite_phase(v, rules)?))
                })
                .collect::<Result<Vec<_>, _>>()?;
            JsonF::Object(new_pairs)
        }
    };

    // Fixed-point: apply rules to this node until none fire
    let mut current = rewritten_children;
    loop {
        let mut changed = false;
        for rule in rules {
            if let Some(new_node) = rule.rewrite(&tree.provenance, &current)? {
                current = new_node;
                changed = true;
                break; // restart from first rule after a change
            }
        }
        if !changed {
            break;
        }
    }

    Ok(LocatedValue::new(tree.provenance.clone(), current))
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test rewrite`
Expected: All tests pass.

- [ ] **Step 5: Wire into `src/lib.rs` and commit**

Add `pub mod rewrite;` to `src/lib.rs`.

```bash
git add src/rewrite/
git commit -m "Add rewrite engine: Phase, RewriteRule trait, bottom-up fixed-point"
```

---

### Task 6: TypeSet and BoolSet

**Files:**
- Create: `src/schema/mod.rs`
- Create: `src/schema/typeset.rs`
- Create: `src/schema/boolset.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/schema/typeset.rs`**

```rust
bitflags::bitflags! {
    /// The set of JSON types a schema permits.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct TypeSet: u8 {
        const NULL    = 1 << 0;
        const BOOLEAN = 1 << 1;
        const OBJECT  = 1 << 2;
        const ARRAY   = 1 << 3;
        const NUMBER  = 1 << 4;
        const STRING  = 1 << 5;
        const INTEGER = 1 << 6;
    }
}

impl TypeSet {
    /// Parse a type name string into a single-type TypeSet.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "null" => Some(Self::NULL),
            "boolean" => Some(Self::BOOLEAN),
            "object" => Some(Self::OBJECT),
            "array" => Some(Self::ARRAY),
            "number" => Some(Self::NUMBER),
            "string" => Some(Self::STRING),
            "integer" => Some(Self::INTEGER),
            _ => None,
        }
    }

    /// All JSON types including integer.
    pub fn all_types() -> Self {
        Self::all()
    }

    /// Is this a subset of the other type set?
    pub fn is_subset_of(self, other: Self) -> bool {
        self & other == self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name() {
        assert_eq!(TypeSet::from_name("string"), Some(TypeSet::STRING));
        assert_eq!(TypeSet::from_name("unknown"), None);
    }

    #[test]
    fn subset() {
        let nums = TypeSet::NUMBER | TypeSet::INTEGER;
        assert!(TypeSet::NUMBER.is_subset_of(nums));
        assert!(!TypeSet::STRING.is_subset_of(nums));
    }
}
```

- [ ] **Step 2: Create `src/schema/boolset.rs`**

```rust
/// The set of boolean values a schema permits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolSet {
    Neither,      // empty — uninhabited boolean
    TrueOnly,
    FalseOnly,
    Both,
}

impl BoolSet {
    pub fn from_values(has_true: bool, has_false: bool) -> Self {
        match (has_true, has_false) {
            (false, false) => Self::Neither,
            (true, false) => Self::TrueOnly,
            (false, true) => Self::FalseOnly,
            (true, true) => Self::Both,
        }
    }

    pub fn is_subset_of(self, other: Self) -> bool {
        let s: u8 = self.into();
        let o: u8 = other.into();
        s & o == s
    }

    pub fn intersect(self, other: Self) -> Self {
        let s: u8 = self.into();
        let o: u8 = other.into();
        (s & o).into()
    }

    pub fn union(self, other: Self) -> Self {
        let s: u8 = self.into();
        let o: u8 = other.into();
        (s | o).into()
    }

    pub fn complement(self) -> Self {
        let s: u8 = self.into();
        ((!s) & 0b11).into()
    }

    pub fn is_empty(self) -> bool {
        self == Self::Neither
    }
}

impl From<BoolSet> for u8 {
    fn from(bs: BoolSet) -> u8 {
        match bs {
            BoolSet::Neither => 0b00,
            BoolSet::TrueOnly => 0b10,
            BoolSet::FalseOnly => 0b01,
            BoolSet::Both => 0b11,
        }
    }
}

impl From<u8> for BoolSet {
    fn from(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::Neither,
            0b10 => Self::TrueOnly,
            0b01 => Self::FalseOnly,
            0b11 => Self::Both,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subset() {
        assert!(BoolSet::TrueOnly.is_subset_of(BoolSet::Both));
        assert!(!BoolSet::Both.is_subset_of(BoolSet::TrueOnly));
    }

    #[test]
    fn complement() {
        assert_eq!(BoolSet::TrueOnly.complement(), BoolSet::FalseOnly);
        assert_eq!(BoolSet::Both.complement(), BoolSet::Neither);
    }

    #[test]
    fn intersect() {
        assert_eq!(BoolSet::Both.intersect(BoolSet::TrueOnly), BoolSet::TrueOnly);
    }
}
```

- [ ] **Step 3: Create `src/schema/mod.rs`, wire into lib, run tests**

```rust
mod boolset;
mod typeset;

pub use boolset::BoolSet;
pub use typeset::TypeSet;
```

Add `pub mod schema;` to `src/lib.rs`.

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/schema/
git commit -m "Add TypeSet (bitflags) and BoolSet for type/boolean keywords"
```

---

### Task 7: ReducedSchema IR Types

**Files:**
- Create: `src/schema/reduced.rs`
- Modify: `src/schema/mod.rs`

- [ ] **Step 1: Create `src/schema/reduced.rs`**

These are the typed IR types that the subtype checker operates on. They match the paper's post-simplification schema form, adapted for 2020-12.

```rust
use crate::located::{Located, Provenance};
use super::boolset::BoolSet;

/// A fully reduced, typed schema ready for subtype checking.
#[derive(Debug)]
pub enum ReducedSchema {
    /// Top type — accepts everything ({} or true).
    Top(Provenance),
    /// Bottom type — accepts nothing ({not: {}} or false).
    Bottom(Provenance),
    /// A type-homogeneous schema.
    Typed(TypedSchema),
    /// Union of schemas. After simplification, elements are non-overlapping
    /// for primitives (may still overlap for arrays/objects).
    AnyOf(Provenance, Vec<ReducedSchema>),
    /// Residual conjunction with negation — couldn't be simplified away.
    /// Used for number/array/object negation patterns.
    AllOf(Provenance, Vec<ReducedSchema>),
    /// Negation that couldn't be eliminated (number, array, object).
    Not(Provenance, Box<ReducedSchema>),
}

/// Type-homogeneous schema — exactly one JSON type.
#[derive(Debug)]
pub enum TypedSchema {
    Null(Provenance),
    Boolean(BooleanSchema),
    String(StringSchema),
    Number(NumberSchema),
    Array(ArraySchema),
    Object(ObjectSchema),
}

/// After canonicalization, boolean schemas only have enum.
#[derive(Debug)]
pub struct BooleanSchema {
    pub provenance: Provenance,
    pub enum_values: Located<BoolSet>,
}

/// After canonicalization, string schemas only have pattern
/// (minLength/maxLength compiled into the regex).
#[derive(Debug)]
pub struct StringSchema {
    pub provenance: Provenance,
    /// The regex pattern. After canonicalization this encodes all string constraints.
    pub pattern: Located<String>, // stored as pattern string; compiled to DFA for checking
}

/// Number schemas retain all numeric keywords.
#[derive(Debug)]
pub struct NumberSchema {
    pub provenance: Provenance,
    pub minimum: Located<f64>,
    pub maximum: Located<f64>,
    pub exclusive_minimum: Located<f64>,
    pub exclusive_maximum: Located<f64>,
    pub multiple_of: Option<Located<f64>>,
}

/// Array schemas — prefixItems/items model from 2020-12.
#[derive(Debug)]
pub struct ArraySchema {
    pub provenance: Provenance,
    pub min_items: Located<u64>,
    pub max_items: Located<u64>,             // u64::MAX = unbounded
    pub prefix_items: Vec<ReducedSchema>,     // per-position schemas
    pub items: Box<ReducedSchema>,            // schema for items beyond prefix
    pub unique_items: Located<bool>,
    // contains/minContains/maxContains deferred for now
}

/// Object schemas — after canonicalization, properties+additionalProperties
/// are merged into patternProperties with non-overlapping regexes.
#[derive(Debug)]
pub struct ObjectSchema {
    pub provenance: Provenance,
    pub min_properties: Located<u64>,
    pub max_properties: Located<u64>,         // u64::MAX = unbounded
    pub required: Vec<Located<String>>,
    pub pattern_properties: Vec<(Located<String>, ReducedSchema)>, // (regex, schema)
}

impl ReducedSchema {
    pub fn provenance(&self) -> &Provenance {
        match self {
            ReducedSchema::Top(p)
            | ReducedSchema::Bottom(p)
            | ReducedSchema::AnyOf(p, _)
            | ReducedSchema::AllOf(p, _)
            | ReducedSchema::Not(p, _) => p,
            ReducedSchema::Typed(t) => t.provenance(),
        }
    }
}

impl TypedSchema {
    pub fn provenance(&self) -> &Provenance {
        match self {
            TypedSchema::Null(p) => p,
            TypedSchema::Boolean(s) => &s.provenance,
            TypedSchema::String(s) => &s.provenance,
            TypedSchema::Number(s) => &s.provenance,
            TypedSchema::Array(s) => &s.provenance,
            TypedSchema::Object(s) => &s.provenance,
        }
    }
}
```

- [ ] **Step 2: Update `src/schema/mod.rs`**

Add `mod reduced; pub use reduced::*;`.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add src/schema/
git commit -m "Add ReducedSchema IR types for post-simplification schemas"
```

---

### Task 8: Rewrite Validation Test Harness

Before implementing canonicalization rules, set up the test infrastructure that validates rewrites preserve semantics by running the official JSON Schema Test Suite against both original and rewritten schemas.

**Files:**
- Create: `tests/helpers/mod.rs`
- Create: `tests/rewrite_validation.rs`

- [ ] **Step 1: Add JSON Schema Test Suite as a git submodule**

```bash
git submodule add https://github.com/json-schema-org/JSON-Schema-Test-Suite.git tests/JSON-Schema-Test-Suite
```

- [ ] **Step 2: Create test helpers**

Create `tests/helpers/mod.rs`:

```rust
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

/// Validate a JSON value against a schema using a simple reference validator.
/// We use the `jsonschema` crate for this — add it as a dev-dependency.
/// Returns true if the value is valid against the schema.
pub fn validate_instance(schema: &Value, instance: &Value) -> bool {
    // This will use the jsonschema crate — see step 3
    todo!()
}
```

- [ ] **Step 3: Add `jsonschema` dev-dependency**

Add to `Cargo.toml`:

```toml
[dev-dependencies]
jsonschema = "0.28"
```

Implement `validate_instance`:

```rust
pub fn validate_instance(schema: &Value, instance: &Value) -> bool {
    // jsonschema crate API — check exact API and adapt
    jsonschema::is_valid(schema, instance)
}
```

- [ ] **Step 4: Create rewrite validation test**

Create `tests/rewrite_validation.rs`:

```rust
mod helpers;

use helpers::{load_test_file, validate_instance};
use json_schema_subtyping::parse::parse;
use json_schema_subtyping::rewrite::{rewrite_phase, Phase};
use serde_json::Value;
use std::path::PathBuf;

/// Convert a LocatedValue back to serde_json::Value for validation.
fn to_json_value(lv: &json_schema_subtyping::located::LocatedValue) -> Value {
    use json_schema_subtyping::located::JsonF;
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
                .filter_map(|(k, v)| {
                    k.as_str().map(|key| (key.to_string(), to_json_value(v)))
                })
                .collect();
            Value::Object(map)
        }
    }
}

/// For each test in the official suite: apply our rewrites to the schema,
/// then validate each test instance against both original and rewritten.
/// They must produce the same pass/fail result.
fn validate_rewrites_preserve_semantics(test_dir: &str, rules_phase: Phase) {
    let suite_dir = PathBuf::from("tests/JSON-Schema-Test-Suite/tests/draft2020-12");
    let test_path = suite_dir.join(test_dir);

    if !test_path.exists() {
        eprintln!("Skipping {test_dir}: path not found");
        return;
    }

    for entry in std::fs::read_dir(&test_path).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|e| e == "json") {
            let groups = load_test_file(&entry.path());
            for group in &groups {
                // Parse the schema
                let schema_str = serde_json::to_string(&group.schema).unwrap();
                let parsed = match parse(&schema_str) {
                    Ok(p) => p,
                    Err(_) => continue, // skip schemas we can't parse
                };

                // Apply rewrites
                // Get rules for the phase from the default preset
                let preset = json_schema_subtyping::preset::Preset::draft_2020_12();
                let rules = preset.rules_for(rules_phase);
                let rewritten = match rewrite_phase(&parsed, rules) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                let rewritten_json = to_json_value(&rewritten);

                // Validate each test instance against both schemas
                for test in &group.tests {
                    let original_result = validate_instance(&group.schema, &test.data);
                    let rewritten_result = validate_instance(&rewritten_json, &test.data);

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
                }
            }
        }
    }
}

#[test]
fn canonicalization_preserves_semantics() {
    validate_rewrites_preserve_semantics(".", Phase::Canonicalization);
}

#[test]
fn simplification_preserves_semantics() {
    validate_rewrites_preserve_semantics(".", Phase::Simplification);
}
```

Note: The `Preset::rules_for` method and exact API will be finalized in Task 19. This test won't fully pass until canonicalization rules are implemented — that's expected. The test harness validates that as we add rules, they preserve semantics.

- [ ] **Step 5: Commit**

```bash
git add tests/ .gitmodules
git commit -m "Add rewrite validation harness using official JSON Schema Test Suite"
```

---

### Task 9: Non-Type-Specific Canonicalization Rules

These correspond to Figure 5 in the paper, adapted for 2020-12.

**Files:**
- Create: `src/rewrite/canonicalize.rs`
- Modify: `src/rewrite/mod.rs`

Rules in this task:
1. `const_to_enum` — `{const: v}` → `{enum: [v]}`
2. `if_then_else` — rewrite to allOf/anyOf/not
3. `multiple_types` — `{type: [t1,...,tn], ...}` → `{anyOf: [{...type:t1}, ..., {...type:tn}]}`
4. `multiple_connectives` — separate connectives from type keywords using allOf
5. `missing_type` — add `type: [all types]` when no type/connective present

- [ ] **Step 1: Write tests for each rule**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::rewrite::rewrite_phase;

    fn apply_rule(input: &str, rule: &dyn RewriteRule) -> LocatedValue {
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![];
        // We test individual rules by wrapping in a Vec
        // For now, use rewrite_phase with a single rule
        let boxed: Vec<Box<dyn RewriteRule>> = vec![];
        // Actually, let's test via a helper
        todo!("implement after defining the rules")
    }

    #[test]
    fn const_to_enum() {
        let input = r#"{"const": 42}"#;
        let expected = r#"{"enum": [42]}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(ConstToEnum)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let expected_tree = parse(expected).unwrap();
        assert!(result.structural_eq(&expected_tree));
    }

    #[test]
    fn multiple_types_splits_into_anyof() {
        let input = r#"{"type": ["string", "null"], "minLength": 1}"#;
        // → {"anyOf": [{"type": "string", "minLength": 1}, {"type": "null", "minLength": 1}]}
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(MultipleTypes)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let anyof = result.get_key("anyOf").unwrap().as_array().unwrap();
        assert_eq!(anyof.len(), 2);
    }

    #[test]
    fn if_then_else_rewrite() {
        let input = r#"{"if": {"type": "string"}, "then": {"minLength": 1}, "else": {"type": "number"}}"#;
        // → {"anyOf": [{"allOf": [if, then]}, {"allOf": [{not: if}, else]}]}
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(IfThenElse)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        assert!(result.get_key("anyOf").is_some());
    }

    #[test]
    fn missing_type_adds_all_types() {
        let input = r#"{"minLength": 5}"#;
        let tree = parse(input).unwrap();
        let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(MissingType)];
        let result = rewrite_phase(&tree, &rules).unwrap();
        let type_val = result.get_key("type").unwrap();
        let types = type_val.as_array().unwrap();
        assert_eq!(types.len(), 7); // all JSON types
    }
}
```

- [ ] **Step 2: Implement each rule**

Each rule is a struct implementing `RewriteRule`. They examine the object keys of a schema node and return `Some(new_node)` when they fire.

Key implementation notes:
- Rules only fire on `JsonF::Object` nodes (schemas are JSON objects).
- Each rule checks preconditions (which keys are present/absent) before firing.
- When building the new node, merge provenance from all consumed keywords.
- Helper functions to build `LocatedValue` objects for synthetic nodes (e.g., `make_string`, `make_object`, `make_array`).

The rules are implemented in `src/rewrite/canonicalize.rs`. Each rule struct is exported, and a `canonicalization_rules()` function returns them all as `Vec<Box<dyn RewriteRule>>`.

```rust
use crate::error::SubtypeError;
use crate::located::{JsonF, JsonPointer, LocatedValue, Provenance, Span};
use crate::rewrite::RewriteRule;

/// Helper: build a synthetic LocatedValue with merged provenance.
fn synthetic(prov: &Provenance, node: JsonF<LocatedValue>) -> LocatedValue {
    LocatedValue::new(prov.clone(), node)
}

fn make_string(prov: &Provenance, s: &str) -> LocatedValue {
    synthetic(prov, JsonF::String(s.to_string()))
}

fn make_array(prov: &Provenance, items: Vec<LocatedValue>) -> LocatedValue {
    synthetic(prov, JsonF::Array(items))
}

fn make_object(prov: &Provenance, pairs: Vec<(&str, LocatedValue)>) -> LocatedValue {
    let obj_pairs = pairs
        .into_iter()
        .map(|(k, v)| (make_string(prov, k), v))
        .collect();
    synthetic(prov, JsonF::Object(obj_pairs))
}

/// Helper: get a key-value pair from an object node.
fn get_entry<'a>(pairs: &'a [(LocatedValue, LocatedValue)], key: &str) -> Option<&'a LocatedValue> {
    pairs.iter().find_map(|(k, v)| {
        if k.as_str() == Some(key) { Some(v) } else { None }
    })
}

/// Helper: does this object have a given key?
fn has_key(pairs: &[(LocatedValue, LocatedValue)], key: &str) -> bool {
    get_entry(pairs, key).is_some()
}

/// Helper: remove a key from an object, returning remaining pairs.
fn without_key(pairs: &[(LocatedValue, LocatedValue)], key: &str) -> Vec<(LocatedValue, LocatedValue)> {
    pairs.iter()
        .filter(|(k, _)| k.as_str() != Some(key))
        .cloned()
        .collect()
}

/// Helper: remove multiple keys from an object.
fn without_keys(pairs: &[(LocatedValue, LocatedValue)], keys: &[&str]) -> Vec<(LocatedValue, LocatedValue)> {
    pairs.iter()
        .filter(|(k, _)| {
            k.as_str().map_or(true, |s| !keys.contains(&s))
        })
        .cloned()
        .collect()
}

// --- Rule: const → enum ---

pub struct ConstToEnum;

impl RewriteRule for ConstToEnum {
    fn rewrite(
        &self, prov: &Provenance, node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else { return Ok(None) };
        let Some(const_val) = get_entry(pairs, "const") else { return Ok(None) };

        let mut new_pairs = without_key(pairs, "const");
        let enum_array = make_array(prov, vec![const_val.clone()]);
        new_pairs.push((make_string(prov, "enum"), enum_array));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

// --- Rule: if/then/else → anyOf/allOf/not ---

pub struct IfThenElse;

impl RewriteRule for IfThenElse {
    fn rewrite(
        &self, prov: &Provenance, node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else { return Ok(None) };
        let Some(if_schema) = get_entry(pairs, "if") else { return Ok(None) };

        let then_schema = get_entry(pairs, "then");
        let else_schema = get_entry(pairs, "else");

        if then_schema.is_none() && else_schema.is_none() {
            return Ok(None); // if without then/else is meaningless
        }

        let not_if = make_object(prov, vec![("not", if_schema.clone())]);

        let branches = match (then_schema, else_schema) {
            (Some(then_s), Some(else_s)) => {
                // {anyOf: [{allOf: [if, then]}, {allOf: [{not: if}, else]}]}
                let branch1 = make_object(prov, vec![
                    ("allOf", make_array(prov, vec![if_schema.clone(), then_s.clone()]))
                ]);
                let branch2 = make_object(prov, vec![
                    ("allOf", make_array(prov, vec![not_if, else_s.clone()]))
                ]);
                vec![branch1, branch2]
            }
            (Some(then_s), None) => {
                // {anyOf: [{not: if}, then]}
                vec![not_if, then_s.clone()]
            }
            (None, Some(else_s)) => {
                // {anyOf: [if, else]}
                vec![if_schema.clone(), else_s.clone()]
            }
            (None, None) => unreachable!(),
        };

        let remaining = without_keys(pairs, &["if", "then", "else"]);
        if remaining.is_empty() {
            Ok(Some(JsonF::Object(vec![
                (make_string(prov, "anyOf"), make_array(prov, branches))
            ])))
        } else {
            // Remaining keywords → wrap in allOf with the anyOf
            let anyof_part = make_object(prov, vec![
                ("anyOf", make_array(prov, branches))
            ]);
            let rest_part = synthetic(prov, JsonF::Object(remaining));
            Ok(Some(JsonF::Object(vec![
                (make_string(prov, "allOf"), make_array(prov, vec![rest_part, anyof_part]))
            ])))
        }
    }
}

// --- Rule: multiple types → anyOf ---

pub struct MultipleTypes;

impl RewriteRule for MultipleTypes {
    fn rewrite(
        &self, prov: &Provenance, node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else { return Ok(None) };
        let Some(type_val) = get_entry(pairs, "type") else { return Ok(None) };
        let Some(types) = type_val.as_array() else { return Ok(None) };

        if types.len() <= 1 { return Ok(None) }

        let other_pairs = without_key(pairs, "type");
        let branches: Vec<LocatedValue> = types.iter().map(|t| {
            let mut branch_pairs = other_pairs.clone();
            branch_pairs.push((make_string(prov, "type"), t.clone()));
            synthetic(prov, JsonF::Object(branch_pairs))
        }).collect();

        Ok(Some(JsonF::Object(vec![
            (make_string(prov, "anyOf"), make_array(prov, branches))
        ])))
    }
}

// --- Rule: multiple connectives → allOf ---

pub struct MultipleConnectives;

impl RewriteRule for MultipleConnectives {
    fn rewrite(
        &self, prov: &Provenance, node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else { return Ok(None) };

        let connectives = ["enum", "anyOf", "allOf", "oneOf", "not"];
        let present: Vec<&str> = connectives.iter()
            .filter(|c| has_key(pairs, c))
            .copied()
            .collect();

        if present.is_empty() { return Ok(None) }

        let non_connective: Vec<_> = pairs.iter()
            .filter(|(k, _)| k.as_str().map_or(true, |s| !connectives.contains(&s)))
            .cloned()
            .collect();

        if non_connective.is_empty() { return Ok(None) }

        // Split first connective from the rest
        let connective_key = present[0];
        let connective_val = get_entry(pairs, connective_key).unwrap().clone();
        let connective_part = make_object(prov, vec![(connective_key, connective_val)]);

        let mut rest_pairs = non_connective;
        for &c in &present[1..] {
            let val = get_entry(pairs, c).unwrap().clone();
            rest_pairs.push((make_string(prov, c), val));
        }
        let rest_part = synthetic(prov, JsonF::Object(rest_pairs));

        Ok(Some(JsonF::Object(vec![
            (make_string(prov, "allOf"), make_array(prov, vec![connective_part, rest_part]))
        ])))
    }
}

// --- Rule: missing type → add type: [all types] ---

pub struct MissingType;

impl RewriteRule for MissingType {
    fn rewrite(
        &self, prov: &Provenance, node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError> {
        let JsonF::Object(pairs) = node else { return Ok(None) };

        let has_type_or_connective = ["type", "enum", "anyOf", "allOf", "oneOf", "not"]
            .iter()
            .any(|k| has_key(pairs, k));

        if has_type_or_connective { return Ok(None) }
        if pairs.is_empty() { return Ok(None) } // empty object = top, not missing type

        let all_types = ["null", "boolean", "object", "array", "number", "string", "integer"]
            .iter()
            .map(|t| make_string(prov, t))
            .collect();
        let mut new_pairs = pairs.to_vec();
        new_pairs.push((make_string(prov, "type"), make_array(prov, all_types)));
        Ok(Some(JsonF::Object(new_pairs)))
    }
}

/// Returns all non-type-specific canonicalization rules in application order.
pub fn non_type_specific_rules() -> Vec<Box<dyn RewriteRule>> {
    vec![
        Box::new(ConstToEnum),
        Box::new(IfThenElse),
        Box::new(MultipleTypes),
        Box::new(MultipleConnectives),
        Box::new(MissingType),
    ]
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test canonicalize`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/rewrite/
git commit -m "Add non-type-specific canonicalization rules (const, if/then/else, types, connectives)"
```

---

### Task 10: Type-Specific Canonicalization Rules

Remaining canonicalization rules from Figure 6 in the paper, adapted for 2020-12.

**Files:**
- Modify: `src/rewrite/canonicalize.rs`

Rules:
1. `missing_keyword` — add defaults for missing type-specific keywords
2. `irrelevant_keywords` — strip keywords not relevant to the schema's type
3. `integer_to_number` — `type: integer` → `type: number, multipleOf: lcm(1, existing)`
4. `heterogeneous_enum` — split enum by type into anyOf
5. `oneOf_to_anyOf` — oneOf → anyOf with allOf/not exclusion
6. `string_canonicalize` — compile minLength/maxLength into pattern (requires regex intersection from Task 12)
7. `object_additional_properties_false` — `additionalProperties: false` → `{not: {}}`
8. `object_with_properties` — merge properties+additionalProperties into patternProperties
9. `dependent_required` — convert to dependentSchemas
10. `dependent_schemas` — convert to allOf/anyOf
11. `overlapping_pattern_properties` — make pattern regexes disjoint

These are substantial. Each follows the same pattern as Task 9: struct implementing `RewriteRule`, checking preconditions on the object keys, building the rewritten node.

- [ ] **Step 1: Write tests for integer, enum, oneOf rules**

```rust
#[test]
fn integer_to_number() {
    let input = r#"{"type": "integer", "minimum": 0}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(IntegerToNumber)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    assert_eq!(result.get_key("type").unwrap().as_str(), Some("number"));
    assert!(result.get_key("multipleOf").is_some());
}

#[test]
fn heterogeneous_enum_splits() {
    let input = r#"{"enum": [1, "hello", true]}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(HeterogeneousEnum)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    assert!(result.get_key("anyOf").is_some());
}

#[test]
fn oneof_to_anyof() {
    let input = r#"{"oneOf": [{"type": "string"}, {"type": "number"}]}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(OneOfToAnyOf)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    assert!(result.get_key("anyOf").is_some());
    assert!(result.get_key("oneOf").is_none());
}
```

- [ ] **Step 2: Implement rules, run tests**

Each rule follows the same structural pattern as Task 9. Key implementation details:

- `IntegerToNumber`: Check `type == "integer"`, replace with `type: "number"`, set `multipleOf: lcm(1, existing_multipleOf)`. `lcm(1, x) = x` if defined, `1` if not.
- `HeterogeneousEnum`: Check `enum` contains values of different types (use a `typeof_json_value` helper). Split into `anyOf` branches grouped by type.
- `OneOfToAnyOf`: `{oneOf: [s1, ..., sn]}` → `{anyOf: [{allOf: [s1, {not: s2}, ..., {not: sn}]}, ..., {allOf: [{not: s1}, ..., {not: s_{n-1}}, sn]}]}`

- [ ] **Step 3: Write tests for object rules**

```rust
#[test]
fn object_additional_properties_false_to_not() {
    let input = r#"{"type": "object", "additionalProperties": false}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(AdditionalPropertiesFalse)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    let ap = result.get_key("additionalProperties").unwrap();
    assert!(ap.get_key("not").is_some());
}

#[test]
fn object_properties_merged_into_pattern_properties() {
    let input = r#"{
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "additionalProperties": {"type": "number"}
    }"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(ObjectWithProperties)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    assert!(result.get_key("properties").is_none());
    assert!(result.get_key("additionalProperties").is_none());
    assert!(result.get_key("patternProperties").is_some());
}
```

- [ ] **Step 4: Implement object rules, run tests**

- `AdditionalPropertiesFalse`: `additionalProperties: false` → `additionalProperties: {not: {}}`
- `ObjectWithProperties`: Convert `properties` keys to `^key$` patterns in `patternProperties`, subtract from existing pattern properties, add catch-all pattern for `additionalProperties`. Remove `properties` and `additionalProperties` keys.
- `DependentRequired`: `dependentRequired: {k: [k1,...]}` → `dependentSchemas: {k: {type: object, required: [k1,...]}}`
- `DependentSchemas`: `dependentSchemas: {k: s}` → `allOf: [original_without_dep, {anyOf: [s, {type: object, properties: {k: {not: {}}}}]}]`
- `OverlappingPatternProperties`: For each pair of patterns, split into intersection/difference (requires regex algebra from Task 12).

Note: String canonicalization and overlapping pattern properties require regex algebra. These rules will be skipped initially and implemented after Task 12.

- [ ] **Step 5: Run all tests, commit**

Run: `cargo test`
Expected: All tests pass.

```bash
git add src/rewrite/
git commit -m "Add type-specific canonicalization rules (integer, enum, oneOf, object)"
```

---

### Task 11: Simplification Rules

Figures 7-10 in the paper. These eliminate enum, not, allOf, anyOf where possible.

**Files:**
- Create: `src/rewrite/simplify.rs`
- Modify: `src/rewrite/mod.rs`

This is a large set of rules. Group by category:

**Enum elimination (Figure 7):**
- `multi_valued_enum` — non-boolean multi-enum → anyOf of singletons
- `null_enum` — `{type: null, enum: [null]}` → `{type: null}`
- `string_enum` — `{type: string, enum: [v]}` → `{type: string, pattern: ^v$}` (requires regex escaping)
- `number_enum` — `{type: number, enum: [v]}` → `{type: number, minimum: v, maximum: v}`
- `array_enum` — push enum into per-item schemas
- `object_enum` — push enum into per-property schemas

**Negation elimination (Figure 8):**
- `not_type` — `{not: {type: t, ...}}` → `{anyOf: [complement(s), {type: (all \ t)}]}`
- `complement_null` — → bottom
- `complement_boolean` — → complement of BoolSet
- `complement_string` — → complement regex (requires regex algebra)
- `not_anyOf` — De Morgan → allOf of nots
- `not_allOf` — De Morgan → anyOf of nots
- `not_not` — double negation elimination

**AllOf elimination (Figure 9):**
- `singleton_allOf` — unwrap
- `fold_allOf` — n-ary → binary
- `intersect_heterogeneous` — → bottom
- `intersect_null`, `intersect_boolean`, `intersect_string`, `intersect_number` — merge same-typed
- `intersect_array` — merge per-item (adapted for prefixItems/items)
- `intersect_object` — union patternProperties
- `intersect_anyOf` — distribute allOf over anyOf

**AnyOf simplification (Figure 10):**
- `singleton_anyOf` — unwrap
- `fold_anyOf` — n-ary → binary
- `union_null`, `union_boolean`, `union_string` — merge same-typed
- `union_number` — make disjoint ranges

- [ ] **Step 1: Write tests for enum and negation rules**

Tests follow the same pattern: parse input, apply rule(s), check output structure.

```rust
#[test]
fn null_enum_simplified() {
    let input = r#"{"type": "null", "enum": [null]}"#;
    let expected = r#"{"type": "null"}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(NullEnum)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    let expected_tree = parse(expected).unwrap();
    assert!(result.structural_eq(&expected_tree));
}

#[test]
fn not_not_eliminated() {
    let input = r#"{"not": {"not": {"type": "string"}}}"#;
    let expected = r#"{"type": "string"}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(NotNot)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    let expected_tree = parse(expected).unwrap();
    assert!(result.structural_eq(&expected_tree));
}

#[test]
fn singleton_allof_unwrapped() {
    let input = r#"{"allOf": [{"type": "string"}]}"#;
    let expected = r#"{"type": "string"}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(SingletonAllOf)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    let expected_tree = parse(expected).unwrap();
    assert!(result.structural_eq(&expected_tree));
}

#[test]
fn intersect_heterogeneous_types_is_bottom() {
    let input = r#"{"allOf": [{"type": "string"}, {"type": "number"}]}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(IntersectHeterogeneousTypes)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    // Should be {not: {}} (bottom)
    assert!(result.get_key("not").is_some());
}
```

- [ ] **Step 2: Implement rules**

Each rule follows the same pattern. Rules that need regex algebra (complement_string, intersect_string, union_string, overlapping patterns) will be implemented after Task 12.

- [ ] **Step 3: Run tests, commit**

```bash
git add src/rewrite/
git commit -m "Add simplification rules (enum, negation, allOf, anyOf elimination)"
```

---

### Task 12: Regex Algebra

String schema canonicalization and simplification require regex operations: intersection, complement, union, and containment checking.

**Files:**
- Create: `src/regex_algebra.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection_of_overlapping_patterns() {
        // .{3,} ∩ [a-z]+ = [a-z]{3,}
        // This is hard to test exactly, so test via containment
        let a = compile("^.{3,}$").unwrap();
        let b = compile("^[a-z]+$").unwrap();
        let inter = intersect(&a, &b).unwrap();
        assert!(matches_str(&inter, "abc"));
        assert!(!matches_str(&inter, "ab"));    // too short
        assert!(!matches_str(&inter, "AB"));    // wrong chars
    }

    #[test]
    fn complement() {
        let a = compile("^abc$").unwrap();
        let comp = complement(&a).unwrap();
        assert!(!matches_str(&comp, "abc"));
        assert!(matches_str(&comp, "def"));
        assert!(matches_str(&comp, "ab"));
    }

    #[test]
    fn containment() {
        let sub = compile("^[a-z]{3}$").unwrap();
        let sup = compile("^[a-z]+$").unwrap();
        assert!(is_subset(&sub, &sup).unwrap());
        assert!(!is_subset(&sup, &sub).unwrap());
    }

    #[test]
    fn union() {
        let a = compile("^abc$").unwrap();
        let b = compile("^def$").unwrap();
        let u = union(&a, &b).unwrap();
        assert!(matches_str(&u, "abc"));
        assert!(matches_str(&u, "def"));
        assert!(!matches_str(&u, "ghi"));
    }

    #[test]
    fn length_pattern() {
        let p = length_pattern(2, Some(5));
        assert!(matches_str(&compile(&p).unwrap(), "ab"));
        assert!(matches_str(&compile(&p).unwrap(), "abcde"));
        assert!(!matches_str(&compile(&p).unwrap(), "a"));
        assert!(!matches_str(&compile(&p).unwrap(), "abcdef"));
    }
}
```

- [ ] **Step 2: Implement regex algebra using `regex-automata`**

The approach:
1. Parse regex pattern strings into `regex-automata` DFAs.
2. Implement set operations on DFAs (intersection via product construction, complement via state inversion, union via product construction).
3. For containment: `L(a) ⊆ L(b)` iff `L(a) ∩ L(¬b) = ∅`.
4. Helper `length_pattern(min, max)` → `"^.{min,max}$"` for compiling length constraints.

```rust
use regex_automata::dfa::{dense, Automaton};
use regex_automata::util::syntax;

/// A compiled regex for set operations.
pub struct CompiledRegex {
    // Internal DFA representation — exact type depends on regex-automata API
    // May need to use dense::DFA or other representation
}

/// Compile a regex pattern string to a DFA.
pub fn compile(pattern: &str) -> Result<CompiledRegex, SubtypeError> { todo!() }

/// Intersection of two regexes.
pub fn intersect(a: &CompiledRegex, b: &CompiledRegex) -> Result<CompiledRegex, SubtypeError> { todo!() }

/// Complement of a regex (matches everything the original doesn't).
pub fn complement(a: &CompiledRegex) -> Result<CompiledRegex, SubtypeError> { todo!() }

/// Union of two regexes.
pub fn union(a: &CompiledRegex, b: &CompiledRegex) -> Result<CompiledRegex, SubtypeError> { todo!() }

/// Check if L(a) ⊆ L(b).
pub fn is_subset(a: &CompiledRegex, b: &CompiledRegex) -> Result<bool, SubtypeError> { todo!() }

/// Check if a regex matches any string at all.
pub fn is_empty(a: &CompiledRegex) -> Result<bool, SubtypeError> { todo!() }

/// Check if a regex matches a specific string.
pub fn matches_str(a: &CompiledRegex, s: &str) -> bool { todo!() }

/// Build a pattern string for length constraints: "^.{min,max}$"
pub fn length_pattern(min: u64, max: Option<u64>) -> String {
    match max {
        Some(m) => format!("^.{{{min},{m}}}$"),
        None => format!("^.{{{min},}}$"),
    }
}
```

Implementation notes:
- `regex-automata`'s `dense::DFA` supports `complement()` on individual states.
- For intersection: build product DFA where a state is `(state_a, state_b)` and it's accepting iff both component states are accepting.
- For union: same product DFA but accepting iff either component state is accepting.
- For emptiness: BFS/DFS from start state checking if any accepting state is reachable.
- The exact API of `regex-automata` v0.4 should be checked during implementation. The DFA module provides `dense::Builder` for construction and methods for traversal.

- [ ] **Step 3: Run tests, commit**

```bash
git add src/regex_algebra.rs
git commit -m "Add regex algebra: intersection, complement, union, containment via DFA"
```

---

### Task 13: String Canonicalization (requires regex algebra)

Now that regex algebra is available, implement the string canonicalization rules that compile minLength/maxLength into patterns.

**Files:**
- Modify: `src/rewrite/canonicalize.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn string_without_maxlength() {
    // {type: string, minLength: 3, pattern: "[a-z]+"} → {type: string, pattern: "[a-z]+" ∩ "^.{3,}$"}
    let input = r#"{"type": "string", "minLength": 3, "pattern": "[a-z]+"}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(StringCanonicalize)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    assert!(result.get_key("minLength").is_none());
    assert!(result.get_key("pattern").is_some());
}

#[test]
fn string_with_maxlength() {
    let input = r#"{"type": "string", "minLength": 1, "maxLength": 10}"#;
    let tree = parse(input).unwrap();
    let rules: Vec<Box<dyn RewriteRule>> = vec![Box::new(StringCanonicalize)];
    let result = rewrite_phase(&tree, &rules).unwrap();
    assert!(result.get_key("minLength").is_none());
    assert!(result.get_key("maxLength").is_none());
    assert!(result.get_key("pattern").is_some());
}
```

- [ ] **Step 2: Implement `StringCanonicalize` rule**

The rule:
1. Check `type == "string"` and either `minLength` or `maxLength` present.
2. Get existing `pattern` (default: `""` meaning match everything).
3. Build length constraint pattern: `^.{min,max}$` or `^.{min,}$`.
4. Intersect existing pattern with length pattern using `regex_algebra::intersect`.
5. Return schema with only `type` and `pattern`, removing `minLength`/`maxLength`.

- [ ] **Step 3: Also implement overlapping pattern properties rule**

This rule also needs regex algebra for computing pattern intersections and differences.

- [ ] **Step 4: Run tests, commit**

```bash
git add src/rewrite/
git commit -m "Add string canonicalization and overlapping pattern properties rules"
```

---

### Task 14: Extraction (LocatedValue → ReducedSchema)

After all rewrite phases, convert the `LocatedValue` tree into the typed `ReducedSchema` IR.

**Files:**
- Create: `src/schema/extract.rs`
- Modify: `src/schema/mod.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn extract_top() {
        let tree = parse("{}").unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::Top(_)));
    }

    #[test]
    fn extract_bottom_true() {
        let tree = parse("true").unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::Top(_)));
    }

    #[test]
    fn extract_bottom_false() {
        let tree = parse("false").unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::Bottom(_)));
    }

    #[test]
    fn extract_string_schema() {
        // After canonicalization, string schema has only type + pattern
        let tree = parse(r#"{"type": "string", "pattern": "^.+$"}"#).unwrap();
        let schema = extract(&tree).unwrap();
        match schema {
            ReducedSchema::Typed(TypedSchema::String(s)) => {
                assert_eq!(s.pattern.value, "^.+$");
            }
            other => panic!("expected string schema, got {other:?}"),
        }
    }

    #[test]
    fn extract_number_schema() {
        let tree = parse(r#"{"type": "number", "minimum": 0, "maximum": 100}"#).unwrap();
        let schema = extract(&tree).unwrap();
        match schema {
            ReducedSchema::Typed(TypedSchema::Number(n)) => {
                assert_eq!(n.minimum.value, 0.0);
                assert_eq!(n.maximum.value, 100.0);
            }
            other => panic!("expected number schema, got {other:?}"),
        }
    }

    #[test]
    fn extract_anyof() {
        let tree = parse(r#"{"anyOf": [{"type": "string"}, {"type": "null"}]}"#).unwrap();
        let schema = extract(&tree).unwrap();
        assert!(matches!(schema, ReducedSchema::AnyOf(_, _)));
    }

    #[test]
    fn unknown_keyword_fails_closed() {
        let tree = parse(r#"{"type": "string", "x-custom": true}"#).unwrap();
        let result = extract(&tree);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Implement extraction**

```rust
use crate::error::SubtypeError;
use crate::located::{JsonF, LocatedValue, Located, Provenance};
use super::reduced::*;

/// Known keywords for the built-in draft 2020-12 vocabulary.
const KNOWN_KEYWORDS: &[&str] = &[
    "type", "enum", "const",
    "allOf", "anyOf", "oneOf", "not",
    "if", "then", "else",
    // string
    "minLength", "maxLength", "pattern",
    // number
    "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf",
    // array
    "prefixItems", "items", "minItems", "maxItems", "uniqueItems",
    "contains", "minContains", "maxContains",
    // object
    "properties", "patternProperties", "additionalProperties",
    "minProperties", "maxProperties", "required",
    "dependentRequired", "dependentSchemas", "propertyNames",
    // meta
    "$schema", "$id", "$ref", "$defs", "$anchor",
    "title", "description", "default", "examples",
    "deprecated", "readOnly", "writeOnly",
    "$comment",
];

/// Extract a LocatedValue (post-rewrite) into a ReducedSchema.
pub fn extract(tree: &LocatedValue) -> Result<ReducedSchema, SubtypeError> {
    match &tree.node {
        JsonF::Bool(true) => Ok(ReducedSchema::Top(tree.provenance.clone())),
        JsonF::Bool(false) => Ok(ReducedSchema::Bottom(tree.provenance.clone())),
        JsonF::Object(pairs) if pairs.is_empty() => {
            Ok(ReducedSchema::Top(tree.provenance.clone()))
        }
        JsonF::Object(pairs) => extract_object(&tree.provenance, pairs),
        _ => Err(SubtypeError::InvalidSchema {
            source: "Schema must be a boolean or object".into(),
        }),
    }
}

fn extract_object(
    prov: &Provenance,
    pairs: &[(LocatedValue, LocatedValue)],
) -> Result<ReducedSchema, SubtypeError> {
    // Check for unknown keywords (fail-closed)
    for (k, _) in pairs {
        if let Some(key) = k.as_str() {
            if !KNOWN_KEYWORDS.contains(&key) {
                return Err(SubtypeError::UnsupportedFeatures {
                    details: format!("Unknown keyword: {key}"),
                });
            }
        }
    }

    // Check for connectives first
    if let Some(anyof) = get_key(pairs, "anyOf") {
        let items = anyof.as_array().ok_or_else(|| SubtypeError::InvalidSchema {
            source: "anyOf must be an array".into(),
        })?;
        let schemas = items.iter().map(extract).collect::<Result<Vec<_>, _>>()?;
        return Ok(ReducedSchema::AnyOf(prov.clone(), schemas));
    }

    if let Some(allof) = get_key(pairs, "allOf") {
        let items = allof.as_array().ok_or_else(|| SubtypeError::InvalidSchema {
            source: "allOf must be an array".into(),
        })?;
        let schemas = items.iter().map(extract).collect::<Result<Vec<_>, _>>()?;
        return Ok(ReducedSchema::AllOf(prov.clone(), schemas));
    }

    if let Some(not_schema) = get_key(pairs, "not") {
        let inner = extract(not_schema)?;
        // Check if this is {not: {}} = bottom
        if matches!(inner, ReducedSchema::Top(_)) {
            return Ok(ReducedSchema::Bottom(prov.clone()));
        }
        return Ok(ReducedSchema::Not(prov.clone(), Box::new(inner)));
    }

    // Type-directed extraction
    let type_val = get_key(pairs, "type");
    let type_str = type_val.and_then(|v| v.as_str());

    match type_str {
        Some("null") => Ok(ReducedSchema::Typed(TypedSchema::Null(prov.clone()))),
        Some("boolean") => extract_boolean(prov, pairs),
        Some("string") => extract_string(prov, pairs),
        Some("number") => extract_number(prov, pairs),
        Some("array") => extract_array(prov, pairs),
        Some("object") => extract_object_schema(prov, pairs),
        Some(other) => Err(SubtypeError::InvalidSchema {
            source: format!("Unknown type: {other}").into(),
        }),
        None => {
            // No type, no connective — treat as top if only meta keywords
            Ok(ReducedSchema::Top(prov.clone()))
        }
    }
}

// Type-specific extractors: extract_boolean, extract_string, extract_number,
// extract_array, extract_object_schema — each reads the relevant keywords
// from the object pairs and builds the corresponding typed schema struct.
// These follow directly from the ReducedSchema field definitions.

fn get_key<'a>(pairs: &'a [(LocatedValue, LocatedValue)], key: &str) -> Option<&'a LocatedValue> {
    pairs.iter().find_map(|(k, v)| {
        if k.as_str() == Some(key) { Some(v) } else { None }
    })
}

fn extract_boolean(prov: &Provenance, pairs: &[(LocatedValue, LocatedValue)]) -> Result<ReducedSchema, SubtypeError> {
    let enum_values = if let Some(enum_val) = get_key(pairs, "enum") {
        let items = enum_val.as_array().ok_or_else(|| SubtypeError::InvalidSchema {
            source: "enum must be an array".into(),
        })?;
        let has_true = items.iter().any(|v| v.as_bool() == Some(true));
        let has_false = items.iter().any(|v| v.as_bool() == Some(false));
        Located::new(enum_val.provenance.clone(), BoolSet::from_values(has_true, has_false))
    } else {
        Located::new(prov.clone(), BoolSet::Both)
    };
    Ok(ReducedSchema::Typed(TypedSchema::Boolean(BooleanSchema {
        provenance: prov.clone(),
        enum_values,
    })))
}

fn extract_string(prov: &Provenance, pairs: &[(LocatedValue, LocatedValue)]) -> Result<ReducedSchema, SubtypeError> {
    let pattern = if let Some(p) = get_key(pairs, "pattern") {
        Located::new(p.provenance.clone(), p.as_str().unwrap_or("").to_string())
    } else {
        Located::new(prov.clone(), String::new()) // empty = match everything
    };
    Ok(ReducedSchema::Typed(TypedSchema::String(StringSchema {
        provenance: prov.clone(),
        pattern,
    })))
}

fn extract_number(prov: &Provenance, pairs: &[(LocatedValue, LocatedValue)]) -> Result<ReducedSchema, SubtypeError> {
    let get_f64 = |key: &str, default: f64| -> Located<f64> {
        get_key(pairs, key)
            .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n)))
            .unwrap_or_else(|| Located::new(prov.clone(), default))
    };

    Ok(ReducedSchema::Typed(TypedSchema::Number(NumberSchema {
        provenance: prov.clone(),
        minimum: get_f64("minimum", f64::NEG_INFINITY),
        maximum: get_f64("maximum", f64::INFINITY),
        exclusive_minimum: get_f64("exclusiveMinimum", f64::NEG_INFINITY),
        exclusive_maximum: get_f64("exclusiveMaximum", f64::INFINITY),
        multiple_of: get_key(pairs, "multipleOf")
            .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n))),
    })))
}

fn extract_array(prov: &Provenance, pairs: &[(LocatedValue, LocatedValue)]) -> Result<ReducedSchema, SubtypeError> {
    let prefix_items = match get_key(pairs, "prefixItems") {
        Some(v) => v.as_array()
            .ok_or_else(|| SubtypeError::InvalidSchema { source: "prefixItems must be array".into() })?
            .iter().map(extract).collect::<Result<Vec<_>, _>>()?,
        None => vec![],
    };

    let items = match get_key(pairs, "items") {
        Some(v) => Box::new(extract(v)?),
        None => Box::new(ReducedSchema::Top(prov.clone())),
    };

    let get_u64 = |key: &str, default: u64| -> Located<u64> {
        get_key(pairs, key)
            .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n as u64)))
            .unwrap_or_else(|| Located::new(prov.clone(), default))
    };

    let unique_items = get_key(pairs, "uniqueItems")
        .and_then(|v| v.as_bool().map(|b| Located::new(v.provenance.clone(), b)))
        .unwrap_or_else(|| Located::new(prov.clone(), false));

    Ok(ReducedSchema::Typed(TypedSchema::Array(ArraySchema {
        provenance: prov.clone(),
        min_items: get_u64("minItems", 0),
        max_items: get_u64("maxItems", u64::MAX),
        prefix_items,
        items,
        unique_items,
    })))
}

fn extract_object_schema(prov: &Provenance, pairs: &[(LocatedValue, LocatedValue)]) -> Result<ReducedSchema, SubtypeError> {
    let get_u64 = |key: &str, default: u64| -> Located<u64> {
        get_key(pairs, key)
            .and_then(|v| v.as_number().map(|n| Located::new(v.provenance.clone(), n as u64)))
            .unwrap_or_else(|| Located::new(prov.clone(), default))
    };

    let required = match get_key(pairs, "required") {
        Some(v) => v.as_array()
            .ok_or_else(|| SubtypeError::InvalidSchema { source: "required must be array".into() })?
            .iter()
            .map(|item| {
                let s = item.as_str().ok_or_else(|| SubtypeError::InvalidSchema {
                    source: "required items must be strings".into(),
                })?;
                Ok(Located::new(item.provenance.clone(), s.to_string()))
            })
            .collect::<Result<Vec<_>, SubtypeError>>()?,
        None => vec![],
    };

    let pattern_properties = match get_key(pairs, "patternProperties") {
        Some(v) => {
            let obj = v.as_object().ok_or_else(|| SubtypeError::InvalidSchema {
                source: "patternProperties must be object".into(),
            })?;
            obj.iter()
                .map(|(k, v)| {
                    let pattern = k.as_str().ok_or_else(|| SubtypeError::InvalidSchema {
                        source: "pattern property key must be string".into(),
                    })?;
                    Ok((
                        Located::new(k.provenance.clone(), pattern.to_string()),
                        extract(v)?,
                    ))
                })
                .collect::<Result<Vec<_>, SubtypeError>>()?
        }
        None => vec![],
    };

    Ok(ReducedSchema::Typed(TypedSchema::Object(ObjectSchema {
        provenance: prov.clone(),
        min_properties: get_u64("minProperties", 0),
        max_properties: get_u64("maxProperties", u64::MAX),
        required,
        pattern_properties,
    })))
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git add src/schema/
git commit -m "Add ReducedSchema extraction from LocatedValue"
```

---

### Task 15: Subtype Checking — Structural Rules

**Files:**
- Create: `src/check/mod.rs`
- Create: `src/check/inhabited.rs`
- Create: `src/check/connective.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_is_supertype_of_everything() {
        let top = ReducedSchema::Top(Provenance::synthetic());
        let bottom = ReducedSchema::Bottom(Provenance::synthetic());
        let s = make_string_schema("^.*$");
        assert!(is_subtype(&bottom, &top).unwrap().is_subtype());
        assert!(is_subtype(&s, &top).unwrap().is_subtype());
    }

    #[test]
    fn bottom_is_subtype_of_everything() {
        let top = ReducedSchema::Top(Provenance::synthetic());
        let bottom = ReducedSchema::Bottom(Provenance::synthetic());
        let s = make_string_schema("^.*$");
        assert!(is_subtype(&bottom, &top).unwrap().is_subtype());
        assert!(is_subtype(&bottom, &s).unwrap().is_subtype());
    }

    #[test]
    fn nothing_is_subtype_of_bottom() {
        let bottom = ReducedSchema::Bottom(Provenance::synthetic());
        let s = make_string_schema("^.*$");
        assert!(!is_subtype(&s, &bottom).unwrap().is_subtype());
    }

    #[test]
    fn anyof_subtype_when_each_branch_has_supertype() {
        // {anyOf: [null, string]} <: {anyOf: [null, string, number]}
        let sub = ReducedSchema::AnyOf(Provenance::synthetic(), vec![
            ReducedSchema::Typed(TypedSchema::Null(Provenance::synthetic())),
            make_string_schema("^.*$"),
        ]);
        let sup = ReducedSchema::AnyOf(Provenance::synthetic(), vec![
            ReducedSchema::Typed(TypedSchema::Null(Provenance::synthetic())),
            make_string_schema("^.*$"),
            make_number_schema(f64::NEG_INFINITY, f64::INFINITY),
        ]);
        assert!(is_subtype(&sub, &sup).unwrap().is_subtype());
    }
}
```

- [ ] **Step 2: Implement structural subtype checking**

```rust
// src/check/mod.rs
mod connective;
mod inhabited;

use crate::error::{SubtypeError, SubtypeRelation, DetailedOutput};
use crate::schema::reduced::*;

pub fn is_subtype(
    sub: &ReducedSchema,
    sup: &ReducedSchema,
) -> Result<SubtypeRelation, SubtypeError> {
    // Rule: top is supertype of everything
    if matches!(sup, ReducedSchema::Top(_)) {
        return Ok(SubtypeRelation::Subtype);
    }
    // Rule: bottom is subtype of everything
    if matches!(sub, ReducedSchema::Bottom(_)) {
        return Ok(SubtypeRelation::Subtype);
    }
    // Rule: nothing is subtype of bottom (except bottom, handled above)
    if matches!(sup, ReducedSchema::Bottom(_)) {
        return Ok(not_subtype("sup is bottom but sub is not"));
    }
    // Rule: top is not subtype of anything (except top, handled above)
    if matches!(sub, ReducedSchema::Top(_)) {
        return Ok(not_subtype("sub is top but sup is not top"));
    }
    // Rule: uninhabited sub is subtype of anything
    if inhabited::is_uninhabited(sub)? {
        return Ok(SubtypeRelation::Subtype);
    }

    // Dispatch to structural/typed checking
    match (sub, sup) {
        (ReducedSchema::AnyOf(_, sub_branches), ReducedSchema::AnyOf(_, sup_branches)) => {
            connective::check_anyof_subtype(sub_branches, sup_branches)
        }
        (ReducedSchema::AnyOf(_, sub_branches), _) => {
            // Each branch of sub must be subtype of sup
            for branch in sub_branches {
                if !is_subtype(branch, sup)?.is_subtype() {
                    return Ok(not_subtype("not all branches of sub's anyOf are subtypes of sup"));
                }
            }
            Ok(SubtypeRelation::Subtype)
        }
        (_, ReducedSchema::AnyOf(_, sup_branches)) => {
            // Sub must be subtype of at least one branch
            for branch in sup_branches {
                if is_subtype(sub, branch)?.is_subtype() {
                    return Ok(SubtypeRelation::Subtype);
                }
            }
            Ok(not_subtype("sub is not a subtype of any branch of sup's anyOf"))
        }
        (ReducedSchema::Typed(sub_t), ReducedSchema::Typed(sup_t)) => {
            check_typed_subtype(sub_t, sup_t)
        }
        _ => Ok(not_subtype("incompatible schema shapes")),
    }
}

fn not_subtype(message: &str) -> SubtypeRelation {
    SubtypeRelation::NotSubtype(DetailedOutput {
        message: message.to_string(),
    })
}

fn check_typed_subtype(
    sub: &TypedSchema,
    sup: &TypedSchema,
) -> Result<SubtypeRelation, SubtypeError> {
    // Dispatch will be filled in as type-specific checkers are added
    todo!()
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git add src/check/
git commit -m "Add subtype checking: top/bottom, uninhabited, anyOf structural rules"
```

---

### Task 16: Subtype Checking — Primitive Types

**Files:**
- Create: `src/check/primitive.rs`
- Modify: `src/check/mod.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn null_subtype_of_null() {
    let a = TypedSchema::Null(Provenance::synthetic());
    let b = TypedSchema::Null(Provenance::synthetic());
    assert!(check_typed_subtype(&a, &b).unwrap().is_subtype());
}

#[test]
fn boolean_subset() {
    let sub = make_bool_schema(BoolSet::TrueOnly);
    let sup = make_bool_schema(BoolSet::Both);
    assert!(check_typed_subtype(&sub, &sup).unwrap().is_subtype());
    assert!(!check_typed_subtype(&sup, &sub).unwrap().is_subtype());
}

#[test]
fn string_pattern_containment() {
    let sub = make_string_schema("^[a-z]{3}$");
    let sup = make_string_schema("^[a-z]+$");
    assert!(check_typed_subtype(&sub, &sup).unwrap().is_subtype());
    assert!(!check_typed_subtype(&sup, &sub).unwrap().is_subtype());
}

#[test]
fn different_types_not_subtype() {
    let s = make_string_schema("^.*$");
    let n = TypedSchema::Null(Provenance::synthetic());
    assert!(!check_typed_subtype(&s, &n).unwrap().is_subtype());
}
```

- [ ] **Step 2: Implement primitive type subtype checks**

```rust
// src/check/primitive.rs
pub fn check_null_subtype() -> SubtypeRelation {
    SubtypeRelation::Subtype // null <: null always
}

pub fn check_boolean_subtype(sub: &BooleanSchema, sup: &BooleanSchema) -> SubtypeRelation {
    if sub.enum_values.value.is_subset_of(sup.enum_values.value) {
        SubtypeRelation::Subtype
    } else {
        not_subtype("boolean enum not a subset")
    }
}

pub fn check_string_subtype(
    sub: &StringSchema,
    sup: &StringSchema,
) -> Result<SubtypeRelation, SubtypeError> {
    let sub_re = regex_algebra::compile(&sub.pattern.value)?;
    let sup_re = regex_algebra::compile(&sup.pattern.value)?;
    if regex_algebra::is_subset(&sub_re, &sup_re)? {
        Ok(SubtypeRelation::Subtype)
    } else {
        Ok(not_subtype("string pattern not contained"))
    }
}
```

- [ ] **Step 3: Wire into `check_typed_subtype`, run tests, commit**

```bash
git add src/check/
git commit -m "Add primitive subtype checks: null, boolean, string"
```

---

### Task 17: Subtype Checking — Number

The most complex primitive check due to `multipleOf` interactions.

**Files:**
- Create: `src/check/number.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn number_range_subtype() {
    // [0, 10] <: [-5, 100]
    let sub = make_number_schema(0.0, 10.0);
    let sup = make_number_schema(-5.0, 100.0);
    assert!(check_number_subtype(&sub, &sup).unwrap().is_subtype());
}

#[test]
fn number_range_not_subtype() {
    // [-5, 100] NOT <: [0, 10]
    let sub = make_number_schema(-5.0, 100.0);
    let sup = make_number_schema(0.0, 10.0);
    assert!(!check_number_subtype(&sub, &sup).unwrap().is_subtype());
}

#[test]
fn number_multiple_of_subtype() {
    // multipleOf 6 <: multipleOf 3 (6 is divisible by 3)
    let sub = make_number_schema_with_multiple(f64::NEG_INFINITY, f64::INFINITY, Some(6.0));
    let sup = make_number_schema_with_multiple(f64::NEG_INFINITY, f64::INFINITY, Some(3.0));
    assert!(check_number_subtype(&sub, &sup).unwrap().is_subtype());
}

#[test]
fn number_multiple_of_not_subtype() {
    // multipleOf 3 NOT <: multipleOf 6
    let sub = make_number_schema_with_multiple(f64::NEG_INFINITY, f64::INFINITY, Some(3.0));
    let sup = make_number_schema_with_multiple(f64::NEG_INFINITY, f64::INFINITY, Some(6.0));
    assert!(!check_number_subtype(&sub, &sup).unwrap().is_subtype());
}
```

- [ ] **Step 2: Implement number subtype check**

For basic number subtype (without the full `subRange` relation from the paper):
1. Check `sub.minimum >= sup.minimum` (accounting for exclusive bounds)
2. Check `sub.maximum <= sup.maximum` (accounting for exclusive bounds)
3. Check `sub.multipleOf` divides `sup.multipleOf` (if sup has one)
4. If sup has no multipleOf, any sub multipleOf is fine

The full `subRange` relation (handling AllOf with negated number schemas) is more complex and matches the paper's Section 4.3. Implement the basic version first, extend to handle AllOfNot later.

- [ ] **Step 3: Run tests, commit**

```bash
git add src/check/
git commit -m "Add number subtype checking with range and multipleOf comparison"
```

---

### Task 18: Subtype Checking — Array

**Files:**
- Create: `src/check/array.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn array_min_max_items() {
    // [minItems: 2, maxItems: 5] <: [minItems: 1, maxItems: 10]
    let sub = make_array_schema(2, 5, vec![], top(), false);
    let sup = make_array_schema(1, 10, vec![], top(), false);
    assert!(check_array_subtype(&sub, &sup).unwrap().is_subtype());
}

#[test]
fn array_per_item_subtype() {
    // prefixItems: [int, string], items: number
    // <: prefixItems: [number, string], items: top
    // because int <: number and string <: string and number <: top
    let sub = make_array_schema(0, u64::MAX,
        vec![make_number_schema(f64::NEG_INFINITY, f64::INFINITY), make_string_schema("^.*$")],
        make_number_schema(f64::NEG_INFINITY, f64::INFINITY),
        false);
    let sup = make_array_schema(0, u64::MAX,
        vec![make_number_schema(f64::NEG_INFINITY, f64::INFINITY), make_string_schema("^.*$")],
        ReducedSchema::Top(Provenance::synthetic()),
        false);
    assert!(check_array_subtype(&sub, &sup).unwrap().is_subtype());
}

#[test]
fn array_unique_items() {
    // uniqueItems: true → uniqueItems: true OK
    // uniqueItems: false → uniqueItems: true FAIL
    let sub = make_array_schema(0, u64::MAX, vec![], top(), false);
    let sup = make_array_schema(0, u64::MAX, vec![], top(), true);
    assert!(!check_array_subtype(&sub, &sup).unwrap().is_subtype());
}
```

- [ ] **Step 2: Implement array subtype check**

Following the paper's `subschema array` rule, adapted for 2020-12:
1. `sub.min_items >= sup.min_items`
2. `sub.max_items <= sup.max_items`
3. For each position `i` up to `max(len(sub.prefix_items), len(sup.prefix_items)) + 1`:
   - effective sub schema = `sub.prefix_items[i]` if exists, else `sub.items`
   - effective sup schema = `sup.prefix_items[i]` if exists, else `sup.items`
   - check `effective_sub <: effective_sup`
4. `sup.unique_items` implies (`sub.unique_items` or `all_disjoint_items(sub)`)

- [ ] **Step 3: Run tests, commit**

```bash
git add src/check/
git commit -m "Add array subtype checking"
```

---

### Task 19: Subtype Checking — Object

**Files:**
- Create: `src/check/object.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn object_required_superset() {
    // {required: [a, b]} <: {required: [a]}
    let sub = make_object_schema(0, u64::MAX, &["a", "b"], vec![]);
    let sup = make_object_schema(0, u64::MAX, &["a"], vec![]);
    assert!(check_object_subtype(&sub, &sup).unwrap().is_subtype());
}

#[test]
fn object_required_not_superset() {
    // {required: [a]} NOT <: {required: [a, b]}
    let sub = make_object_schema(0, u64::MAX, &["a"], vec![]);
    let sup = make_object_schema(0, u64::MAX, &["a", "b"], vec![]);
    assert!(!check_object_subtype(&sub, &sup).unwrap().is_subtype());
}

#[test]
fn object_pattern_properties_subtype() {
    // {patternProperties: {"^name$": string}} <: {patternProperties: {"^name$": string}}
    // Also: overlapping patterns checked
}
```

- [ ] **Step 2: Implement object subtype check**

Following the paper's `subschema object` rule:
1. `sub.min_properties >= sup.min_properties`
2. `sub.max_properties <= sup.max_properties`
3. `sub.required ⊇ sup.required`
4. For every pattern `p1:s1` in sub and `p2:s2` in sup:
   if `p1 ∩ p2 ≠ ∅` then `s1 <: s2` (uses regex intersection non-emptiness check)

- [ ] **Step 3: Run tests, commit**

```bash
git add src/check/
git commit -m "Add object subtype checking"
```

---

### Task 20: Inhabited Predicate

**Files:**
- Modify: `src/check/inhabited.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn bottom_is_uninhabited() {
    assert!(is_uninhabited(&ReducedSchema::Bottom(Provenance::synthetic())).unwrap());
}

#[test]
fn top_is_inhabited() {
    assert!(!is_uninhabited(&ReducedSchema::Top(Provenance::synthetic())).unwrap());
}

#[test]
fn contradictory_number_is_uninhabited() {
    // minimum: 10, maximum: 5 → uninhabited
    let s = make_number_schema(10.0, 5.0);
    assert!(is_uninhabited(&ReducedSchema::Typed(s)).unwrap());
}

#[test]
fn empty_boolset_is_uninhabited() {
    let s = make_bool_schema(BoolSet::Neither);
    assert!(is_uninhabited(&ReducedSchema::Typed(s)).unwrap());
}
```

- [ ] **Step 2: Implement inhabited predicate**

```rust
pub fn is_uninhabited(schema: &ReducedSchema) -> Result<bool, SubtypeError> {
    match schema {
        ReducedSchema::Bottom(_) => Ok(true),
        ReducedSchema::Top(_) => Ok(false),
        ReducedSchema::Typed(t) => is_typed_uninhabited(t),
        ReducedSchema::AnyOf(_, branches) => {
            // Uninhabited if all branches are uninhabited
            for branch in branches {
                if !is_uninhabited(branch)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ReducedSchema::AllOf(_, branches) => {
            // Uninhabited if any branch is uninhabited
            for branch in branches {
                if is_uninhabited(branch)? {
                    return Ok(true);
                }
            }
            Ok(false) // conservative: could still be uninhabited due to interaction
        }
        ReducedSchema::Not(_, inner) => {
            // not(top) = bottom = uninhabited. Otherwise, conservative false.
            Ok(matches!(inner.as_ref(), ReducedSchema::Top(_)))
        }
    }
}

fn is_typed_uninhabited(schema: &TypedSchema) -> Result<bool, SubtypeError> {
    match schema {
        TypedSchema::Null(_) => Ok(false),
        TypedSchema::Boolean(b) => Ok(b.enum_values.value.is_empty()),
        TypedSchema::String(s) => {
            let re = regex_algebra::compile(&s.pattern.value)?;
            regex_algebra::is_empty(&re)
        }
        TypedSchema::Number(n) => {
            // Uninhabited if min > max (accounting for exclusive bounds)
            let effective_min = n.minimum.value.max(n.exclusive_minimum.value);
            let effective_max = n.maximum.value.min(n.exclusive_maximum.value);
            Ok(effective_min > effective_max)
        }
        TypedSchema::Array(a) => {
            // Uninhabited if minItems > maxItems
            Ok(a.min_items.value > a.max_items.value)
        }
        TypedSchema::Object(o) => {
            // Uninhabited if minProperties > maxProperties
            // or if required count > maxProperties
            Ok(o.min_properties.value > o.max_properties.value
                || o.required.len() as u64 > o.max_properties.value)
        }
    }
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git add src/check/
git commit -m "Add inhabited predicate for uninhabited schema detection"
```

---

### Task 21: Extension Vocabulary System

**Files:**
- Create: `src/extension.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write tests with a mock extension**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct TestExt;
    struct TestData(Option<String>);

    impl VocabExtension for TestExt {
        type Extracted = TestData;

        fn keywords(&self) -> &[&str] { &["x-test"] }

        fn extract(
            &self,
            keywords: &std::collections::HashMap<&str, &LocatedValue>,
        ) -> Result<TestData, SubtypeError> {
            let val = keywords.get("x-test")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Ok(TestData(val))
        }

        fn check_subtype(
            &self,
            sub: &TestData,
            sup: &TestData,
            _ctx: &mut SubtypeContext,
        ) -> Result<SubtypeRelation, SubtypeError> {
            match (&sub.0, &sup.0) {
                (_, None) => Ok(SubtypeRelation::Subtype),
                (None, Some(_)) => Ok(SubtypeRelation::NotSubtype(DetailedOutput {
                    message: "sub missing x-test".into(),
                })),
                (Some(s), Some(p)) => {
                    if s == p {
                        Ok(SubtypeRelation::Subtype)
                    } else {
                        Ok(SubtypeRelation::NotSubtype(DetailedOutput {
                            message: format!("x-test mismatch: {s} vs {p}"),
                        }))
                    }
                }
            }
        }
    }

    #[test]
    fn extension_slot_roundtrip() {
        let ext = TestExt;
        let slot = ExtensionSlot::new(ext);
        // Test that extraction and downcasting work
    }
}
```

- [ ] **Step 2: Implement extension types**

```rust
use std::any::Any;
use std::collections::HashMap;
use crate::error::{SubtypeError, SubtypeRelation};
use crate::located::LocatedValue;

pub struct SubtypeContext<'a> {
    // Will hold reference to checker for recursive checks
    _phantom: std::marker::PhantomData<&'a ()>,
}

pub trait VocabExtension: 'static {
    type Extracted: 'static;
    fn keywords(&self) -> &[&str];
    fn extract(
        &self,
        keywords: &HashMap<&str, &LocatedValue>,
    ) -> Result<Self::Extracted, SubtypeError>;
    fn check_subtype(
        &self,
        sub: &Self::Extracted,
        sup: &Self::Extracted,
        ctx: &mut SubtypeContext,
    ) -> Result<SubtypeRelation, SubtypeError>;
}

/// Type-erased wrapper around a VocabExtension.
pub struct ExtensionSlot {
    pub keywords: Vec<String>,
    extract_fn: Box<dyn Fn(&HashMap<&str, &LocatedValue>) -> Result<Box<dyn Any>, SubtypeError>>,
    check_fn: Box<dyn Fn(&dyn Any, &dyn Any, &mut SubtypeContext) -> Result<SubtypeRelation, SubtypeError>>,
}

impl ExtensionSlot {
    pub fn new<E: VocabExtension>(ext: E) -> Self {
        let keywords = ext.keywords().iter().map(|s| s.to_string()).collect();
        let ext_extract = std::sync::Arc::new(ext);
        let ext_check = ext_extract.clone();

        Self {
            keywords,
            extract_fn: Box::new(move |kws| {
                let data = ext_extract.extract(kws)?;
                Ok(Box::new(data))
            }),
            check_fn: Box::new(move |sub, sup, ctx| {
                let sub = sub.downcast_ref::<E::Extracted>().unwrap();
                let sup = sup.downcast_ref::<E::Extracted>().unwrap();
                ext_check.check_subtype(sub, sup, ctx)
            }),
        }
    }

    pub fn extract(&self, kws: &HashMap<&str, &LocatedValue>) -> Result<Box<dyn Any>, SubtypeError> {
        (self.extract_fn)(kws)
    }

    pub fn check(&self, sub: &dyn Any, sup: &dyn Any, ctx: &mut SubtypeContext) -> Result<SubtypeRelation, SubtypeError> {
        (self.check_fn)(sub, sup, ctx)
    }
}

/// Storage for extracted extension data on a schema.
pub struct ExtensionData {
    pub slots: Vec<Box<dyn Any>>,
}
```

Note: `VocabExtension` needs to be `Sync + Send` for the `Arc`. Add those bounds or use a different sharing strategy depending on the threading model.

- [ ] **Step 3: Run tests, commit**

```bash
git add src/extension.rs
git commit -m "Add extension vocabulary system: VocabExtension trait, ExtensionSlot, ExtensionData"
```

---

### Task 22: Preset, Builder, and Public API

**Files:**
- Create: `src/preset.rs`
- Modify: `src/lib.rs` (major rewrite to wire everything together)

- [ ] **Step 1: Write API-level tests**

```rust
#[test]
fn simple_subtype_check() {
    let checker = SubtypeChecker::new();
    let result = checker.is_subtype(
        r#"{"type": "integer", "minimum": 0, "maximum": 10}"#,
        r#"{"type": "number"}"#,
    ).unwrap();
    assert!(result.is_subtype());
}

#[test]
fn not_subtype_check() {
    let checker = SubtypeChecker::new();
    let result = checker.is_subtype(
        r#"{"type": "number"}"#,
        r#"{"type": "integer"}"#,
    ).unwrap();
    assert!(!result.is_subtype());
}

#[test]
fn builder_with_extension() {
    let checker = SubtypeChecker::builder()
        .add_extension(TestExt)
        .build();
    // Test that extension keywords are recognized
}
```

- [ ] **Step 2: Implement Preset**

```rust
use std::collections::HashMap;
use crate::rewrite::{Phase, RewriteRule};
use crate::extension::ExtensionSlot;

pub struct Preset {
    pub(crate) rewrites: HashMap<Phase, Vec<Box<dyn RewriteRule>>>,
    pub(crate) extensions: Vec<ExtensionSlot>,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            rewrites: HashMap::new(),
            extensions: Vec::new(),
        }
    }
}

impl Preset {
    pub fn draft_2020_12() -> Self {
        let mut p = Self::default();
        p.rewrites.insert(Phase::Canonicalization,
            crate::rewrite::canonicalize::all_canonicalization_rules());
        p.rewrites.insert(Phase::Simplification,
            crate::rewrite::simplify::all_simplification_rules());
        p
    }

    pub fn add_rewrite(&mut self, phase: Phase, rule: impl RewriteRule + 'static) -> &mut Self {
        self.rewrites.entry(phase).or_default().push(Box::new(rule));
        self
    }

    pub fn add_extension<E: crate::extension::VocabExtension>(&mut self, ext: E) -> &mut Self {
        self.extensions.push(ExtensionSlot::new(ext));
        self
    }

    pub fn rules_for(&self, phase: Phase) -> &[Box<dyn RewriteRule>] {
        self.rewrites.get(&phase).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
```

- [ ] **Step 3: Implement SubtypeChecker and Builder in `src/lib.rs`**

```rust
pub struct SubtypeChecker {
    preset: Preset,
}

impl SubtypeChecker {
    pub fn new() -> Self {
        Self::from_preset(Preset::draft_2020_12())
    }

    pub fn from_preset(preset: Preset) -> Self {
        Self { preset }
    }

    pub fn builder() -> SubtypeCheckerBuilder {
        SubtypeCheckerBuilder {
            preset: Preset::draft_2020_12(),
        }
    }

    pub fn parse(&self, source: &str) -> Result<ParsedSchema, SubtypeError> {
        let tree = parse::parse(source)?;
        Ok(ParsedSchema { source: source.to_string(), tree })
    }

    pub fn reduce(&self, parsed: &ParsedSchema) -> Result<ReducedSchemaSet, SubtypeError> {
        let mut tree = parsed.tree.clone();

        // Run rewrite phases in order
        for phase in [Phase::DraftConversion, Phase::Canonicalization, Phase::Simplification] {
            let rules = self.preset.rules_for(phase);
            if !rules.is_empty() {
                tree = rewrite::rewrite_phase(&tree, rules)?;
            }
        }

        // Extract into ReducedSchema
        let schema = schema::extract::extract(&tree)?;

        Ok(ReducedSchemaSet { schema, extensions: ExtensionData { slots: vec![] } })
    }

    pub fn check(
        &self,
        sub: &ReducedSchemaSet,
        sup: &ReducedSchemaSet,
    ) -> Result<SubtypeRelation, SubtypeError> {
        check::is_subtype(&sub.schema, &sup.schema)
    }

    pub fn is_subtype(&self, sub: &str, sup: &str) -> Result<SubtypeRelation, SubtypeError> {
        let sub_parsed = self.parse(sub)?;
        let sup_parsed = self.parse(sup)?;
        let sub_reduced = self.reduce(&sub_parsed)?;
        let sup_reduced = self.reduce(&sup_parsed)?;
        self.check(&sub_reduced, &sup_reduced)
    }
}

pub struct SubtypeCheckerBuilder {
    preset: Preset,
}

impl SubtypeCheckerBuilder {
    pub fn preset(mut self, preset: Preset) -> Self {
        self.preset = preset;
        self
    }

    pub fn add_rewrite(mut self, phase: Phase, rule: impl RewriteRule + 'static) -> Self {
        self.preset.add_rewrite(phase, rule);
        self
    }

    pub fn add_extension<E: VocabExtension>(mut self, ext: E) -> Self {
        self.preset.add_extension(ext);
        self
    }

    pub fn build(self) -> SubtypeChecker {
        SubtypeChecker::from_preset(self.preset)
    }
}

pub struct ParsedSchema {
    source: String,
    tree: LocatedValue,
}

pub struct ReducedSchemaSet {
    pub schema: ReducedSchema,
    pub extensions: ExtensionData,
}
```

- [ ] **Step 4: Run all tests, commit**

```bash
git add src/
git commit -m "Add Preset, SubtypeChecker builder, and public API"
```

---

### Task 23: $ref Resolution

**Files:**
- Create: `src/rewrite/ref_resolution.rs`
- Modify: `src/rewrite/mod.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn resolve_local_ref() {
    let input = r#"{
        "$defs": {"pos_int": {"type": "integer", "minimum": 0}},
        "type": "object",
        "properties": {"age": {"$ref": "#/$defs/pos_int"}}
    }"#;
    let tree = parse(input).unwrap();
    let resolved = resolve_refs(&tree).unwrap();
    let age_schema = resolved.get_key("properties").unwrap()
        .get_key("age").unwrap();
    // $ref should be replaced with the referenced schema
    assert_eq!(age_schema.get_key("type").unwrap().as_str(), Some("integer"));
}
```

- [ ] **Step 2: Implement $ref resolution**

Walk the tree top-down. When encountering `{"$ref": "#/path/to/def"}`:
1. Parse the JSON pointer from the `$ref` value.
2. Resolve it against the root document.
3. Replace the `$ref` node with a copy of the referenced schema.
4. Merge provenance (the `$ref` location + the definition location).

Note: This only handles local references (`#/...`). External references are out of scope.

- [ ] **Step 3: Wire into the pipeline (before rewrite phases), run tests, commit**

```bash
git add src/rewrite/
git commit -m "Add local $ref resolution as pre-pass before rewrite phases"
```

---

### Task 24: Recover and Extend Subtype Test Suite

Recover the test suite from git history and add new cases for 2020-12 features.

**Files:**
- Recover: `test-suite/` directory from commit `5d4168f`
- Recover: `tests/test_suite.rs` (adapted for new API)

- [ ] **Step 1: Recover test suite files**

```bash
git checkout 5d4168f -- test-suite/
```

- [ ] **Step 2: Adapt test runner for new API**

Update `tests/test_suite.rs` to use `SubtypeChecker::new().is_subtype(sub, sup)` instead of the old API. The test file format (`.nix` files evaluated to JSON) stays the same.

- [ ] **Step 3: Add new test cases for 2020-12 features**

Create new `.nix` test files for:
- `prefixItems` / `items` (2020-12 array model)
- `dependentRequired` / `dependentSchemas`
- `if` / `then` / `else`
- `const`
- `contains` / `minContains` / `maxContains`

- [ ] **Step 4: Run tests, commit**

```bash
git add test-suite/ tests/
git commit -m "Recover and extend subtype test suite for draft 2020-12"
```

---

### Task 25: Integration Tests and Cleanup

Final pass: run all tests, fix any issues, clean up.

**Files:**
- Various fixes across the codebase

- [ ] **Step 1: Run the full test suite**

```bash
cargo test
```

- [ ] **Step 2: Run the rewrite validation against official test suite**

```bash
cargo test rewrite_validation
```

- [ ] **Step 3: Run clippy**

```bash
cargo clippy --all-targets -- --deny warnings
```

- [ ] **Step 4: Fix any issues found**

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "Fix integration issues and pass full test suite"
```

---

## Task Dependency Graph

```
Task 1 (setup)
  → Task 2 (located types)
    → Task 3 (JsonF + LocatedValue)
      → Task 4 (parser)
        → Task 5 (rewrite engine)
          → Task 8 (rewrite validation harness)
          → Task 9 (non-type-specific canonicalization)
            → Task 10 (type-specific canonicalization)
              → Task 13 (string canonicalization) ← Task 12 (regex algebra)
                → Task 11 (simplification rules)
      → Task 14 (extraction) ← Task 7 (ReducedSchema types) ← Task 6 (TypeSet/BoolSet)
        → Task 15-20 (subtype checking)
          → Task 22 (Preset + API) ← Task 21 (extension system)
            → Task 23 ($ref resolution)
              → Task 24 (test suite)
                → Task 25 (integration)
```

Independent tracks that can be parallelized:
- **Track A:** Tasks 1-5, 8-11, 13 (parsing → rewrites)
- **Track B:** Tasks 6-7, 14-20 (schema IR → checking)
- **Track C:** Task 12 (regex algebra — standalone)
- **Track D:** Task 21 (extension system — standalone)

Tracks converge at Task 22 (Preset + API).
