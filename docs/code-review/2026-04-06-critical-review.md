# Code Review: `json-schema-subtyping`

**Date:** 2026-04-06
**Reviewer:** Claude Opus 4.6
**Scope:** Full crate review — correctness, performance, idioms, testing

---

## Summary

This is a well-architected, research-backed implementation of JSON Schema subtype checking. The cofree comonad pattern for provenance tracking, the phased rewrite pipeline, and the DFA-based regex algebra are all well-chosen designs. Below are findings organized by severity.

---

## 1. Correctness Bugs

### 1.1 `AllTypesIsTop` treats "integer" as covering "number" (HIGH)

**File:** `src/rewrite/canonicalize.rs:220-226`

```rust
let all_covered = BASIC_TYPES.iter().all(|basic| {
    if *basic == "number" {
        type_strs.contains(&"number") || type_strs.contains(&"integer")
    } else {
        type_strs.contains(basic)
    }
});
```

If a schema has `type: ["null", "boolean", "object", "array", "integer", "string"]` (no `"number"`), this rule fires and collapses it to `{}` (top). But this schema does NOT accept non-integer numbers like `1.5`. The rule should require `"number"` specifically:

```rust
type_strs.contains(&"number")
```

### 1.2 Circular `$ref` causes stack overflow (HIGH)

**File:** `src/rewrite/ref_resolution.rs:35`

```rust
resolve_refs(root, &result)
```

Recursive references (e.g., `$defs/a` referencing `$defs/b` which references `$defs/a`) will cause unbounded recursion. There's no cycle detection. A visited-set or depth limit is needed.

### 1.3 `is_uninhabited` for numbers ignores exclusive semantics (MEDIUM)

**File:** `src/check/inhabited.rs:45-46`

```rust
let effective_min = n.minimum.value.max(n.exclusive_minimum.value);
let effective_max = n.maximum.value.min(n.exclusive_maximum.value);
Ok(effective_min > effective_max)
```

This computes the effective bounds by taking the max/min of inclusive and exclusive bounds, then checks `min > max`. But it conflates inclusive and exclusive: `exclusiveMinimum: 5, exclusiveMaximum: 6` gives `effective_min=5, effective_max=6`, `5 > 6` is false → inhabited. But the open interval `(5, 6)` with `multipleOf: 1` contains no integers. The conservative approach is documented, but the check could be tightened to also catch `effective_min == effective_max` when both are exclusive at the boundary.

### 1.4 Floating-point epsilon in `multipleOf` check (MEDIUM)

**File:** `src/check/number.rs:64-65`

```rust
let ratio = sub_m.value / sup_m.value;
if (ratio - ratio.round()).abs() > 1e-10 {
```

For large values (e.g., `sub_m = 6e15, sup_m = 3`), floating-point precision loss can produce false positives/negatives. Consider checking `sub_m.value % sup_m.value` with an appropriate tolerance, or converting to integers when both values are integral.

### 1.5 `AllOf` sub handling is incomplete (MEDIUM)

**File:** `src/check/mod.rs:71-77`

```rust
(ReducedSchema::AllOf(_, sub_branches), _) => {
    for branch in sub_branches {
        if is_subtype(branch, sup)?.is_subtype() {
            return Ok(SubtypeRelation::Subtype);
        }
    }
    Ok(not_subtype("no allOf branch individually <: sup"))
}
```

This is documented as conservative, but it means `allOf[{minimum: 0}, {maximum: 10}] <: {minimum: 0, maximum: 10}` returns `NotSubtype`. After simplification, same-typed allOf branches should ideally be merged (intersection of constraints). This is a semantic incompleteness rather than a bug, but it will affect real-world schemas.

---

## 2. Performance Issues

### 2.1 DFA transition table memory (MEDIUM)

**File:** `crates/regex-algebra/src/dfa.rs:4`, `lib.rs:103`

Each DFA state consumes `257 * 4 = 1,028 bytes`. Product construction of two DFAs with N and M states produces up to N*M states. Two 100-state DFAs → ~10 MB. Complex ECMA-262 patterns or repeated intersections (e.g., many overlapping `patternProperties`) could blow up memory. Consider:
- Adding a state limit to `product()` that returns an error if exceeded
- Using alphabet compression (byte classes) from `regex-automata` instead of the full 256-byte alphabet

### 2.2 Excessive cloning in rewrite rules (LOW-MEDIUM)

**Files:** `src/rewrite/canonicalize.rs`, `src/rewrite/simplify.rs`

`without_key()` and `without_keys()` clone the entire pairs vector for each rule invocation. Combined with the fixed-point loop, a schema with 20 keywords could clone its pairs vector hundreds of times. Consider using `Cow` or an arena-based approach.

### 2.3 Linear key lookup in object pairs (LOW)

**File:** `src/located/value.rs:114-122`, `src/rewrite/canonicalize.rs:27-34`

`get_entry()` / `get_key()` do linear scans. JSON Schema objects rarely have more than ~20 keys so this is acceptable, but if you ever plan to support schemas with many properties, an indexed lookup would help.

### 2.4 Rewrite fixed-point restarts from first rule (LOW)

**File:** `src/rewrite/mod.rs:117-129`

After any rule fires, the loop restarts from rule[0]. Rules near the end of the list may force many no-op evaluations of earlier rules. For the current rule count (~15 per phase) this is fine, but as rules grow, a worklist approach would be more efficient.

---

## 3. Correctness Concerns (Not Bugs, But Worth Noting)

### 3.1 ECMA-262 vs Rust regex semantics

JSON Schema specifies ECMA-262 regular expressions, which support lookahead (`(?=...)`, `(?!...)`), lookbehind, backreferences, and `\d` matching only ASCII. The `regex-automata` crate doesn't support these features. Schemas using them will fail at compile time (which is the right behavior — fail-closed), but this is a significant limitation worth documenting clearly.

### 3.2 `structural_eq` is order-sensitive for objects

**File:** `src/located/value.rs:135-139`

Object comparison uses positional zip, so `{"a": 1, "b": 2}` ≠ `{"b": 2, "a": 1}`. This is only used in tests, but could cause false failures if rewrite rules produce keys in different orders.

### 3.3 Extension system is wired but not integrated

**File:** `src/lib.rs:64`

```rust
extensions: ExtensionData { slots: vec![] },
```

`reduce()` always creates empty extension data and `check()` ignores extensions entirely. The extension infrastructure exists but is unused in the pipeline. Either integrate it or mark it clearly as experimental/planned.

### 3.4 `Not` schemas fall through as incompatible

**File:** `src/check/mod.rs:84`

```rust
_ => Ok(not_subtype("incompatible schema shapes")),
```

Any remaining shape (including `Not`) is treated as not-subtype. This means `not(string) <: not(string)` returns `NotSubtype`. The conservative approach is sound (never claims false subtypes), but could confuse users.

---

## 4. Idiomatic Rust

### 4.1 Unused `miette` dependency

**File:** `Cargo.toml:18`

`miette` with the `fancy` feature is declared but never `use`d anywhere in the source. This adds ~40 transitive dependencies and significant compile time. Remove it until actually needed.

### 4.2 Dead code: `TypeSet`

**File:** `src/schema/typeset.rs`

`TypeSet` is defined, exported, and tested but never used outside its own module. It's presumably planned for future use. Either remove it or add `#[allow(dead_code)]` with a comment explaining the plan.

### 4.3 Mixed error libraries: `snafu` vs `thiserror`

The main crate uses `snafu` while `regex-algebra` uses `thiserror`. Both serve the same purpose. Consolidating to one (probably `thiserror` — it's lighter and more widely used) would reduce dependencies and cognitive load.

### 4.4 Missing `#[must_use]` annotations

**File:** `src/error.rs:13`

`SubtypeRelation::is_subtype()` returns a `bool` that's easy to accidentally discard. Add `#[must_use]` to it. Same for `BoolSet::is_empty()`, `BoolSet::is_subset_of()`, etc.

### 4.5 `RewriteRule` trait lacks `Send + Sync`

**File:** `src/rewrite/mod.rs:21`

```rust
pub trait RewriteRule: 'static {
```

All concrete implementations are unit structs (trivially `Send + Sync`), but the trait doesn't require these bounds. This means `SubtypeChecker` is not `Send + Sync`, which would prevent it from being shared across threads (e.g., in a web server).

### 4.6 Helper functions duplicated between modules

`get_entry()`, `without_key()`, `make_string()`, `make_object()`, `make_array()`, and `synthetic()` are defined identically in both `canonicalize.rs` and `simplify.rs`. Extract them to a shared module (e.g., `rewrite::helpers`).

### 4.7 `Phase` as `HashMap` key is wasteful

**File:** `src/preset.rs:9`

```rust
pub(crate) rewrites: HashMap<Phase, Vec<Box<dyn RewriteRule>>>,
```

`Phase` has exactly 3 variants. A `[Vec<Box<dyn RewriteRule>>; 3]` or three named fields would be simpler and avoid the hash overhead.

---

## 5. Testing Gaps

| Gap | Risk |
|-----|------|
| No test for circular `$ref` | Stack overflow in production |
| No test for `AllTypesIsTop` with `["integer"]` but no `["number"]` | Confirms the bug in 1.1 |
| No fuzz tests for `regex-algebra` | DFA state explosion on adversarial patterns |
| Extension system not integration-tested through the full pipeline | Could be silently broken |
| No test for deeply nested schemas (rewrite termination) | Non-termination or stack overflow |
| `structural_eq` order sensitivity not tested | False test failures possible |

---

## 6. Minor Nits

- `src/located/mod.rs` — re-export pattern is clean, no issues.
- `BoolSet` hand-rolled bitflag is actually fine — it's tiny and doesn't need the `bitflags` macro.
- The `Provenance::merge()` accumulates without deduplication — after many rewrites, provenance lists could grow large. Consider deduplicating or capping.
- `src/schema/extract.rs:141` — `p.as_str().unwrap_or("")` silently treats non-string patterns as empty. Should this be an error?

---

## Priority Recommendations

1. **Fix `AllTypesIsTop` integer/number bug** — straightforward, prevents incorrect subtype claims
2. **Add cycle detection to `resolve_refs`** — prevents stack overflow on malicious/malformed input
3. **Remove unused `miette` dependency** — free build time win
4. **Add `Send + Sync` to `RewriteRule`** — enables concurrent use
5. **Add DFA state limit** — prevents DoS via crafted regex patterns
6. **Extract duplicated rewrite helpers** — reduces maintenance burden

---
---

# Recommended Tasks

The following are concrete, prioritized work items derived from the review above. Each is scoped to be a single PR.

---

## Task 1: Fix `AllTypesIsTop` integer/number conflation

**Priority:** P0 — Correctness bug, silent wrong answers
**Effort:** Small (< 1 hour)
**Files:** `src/rewrite/canonicalize.rs`

### Description

The `AllTypesIsTop` rewrite rule incorrectly treats `"integer"` as covering the `"number"` type when checking whether all JSON types are present. A schema like `type: ["null", "boolean", "object", "array", "integer", "string"]` is collapsed to `{}` (top), but it should not accept non-integer numbers like `1.5`.

### Acceptance criteria

- Change the "number" coverage check to require `"number"` explicitly (not `"integer"`).
- Add a test: `type: ["null", "boolean", "object", "array", "integer", "string"]` must NOT be collapsed to top.
- Add a test: `type: ["null", "boolean", "object", "array", "number", "string"]` (without "integer") SHOULD be collapsed to top (number subsumes integer).
- Existing tests continue to pass.

---

## Task 2: Add cycle detection to `$ref` resolution

**Priority:** P0 — Stack overflow on malformed input
**Effort:** Small (~1-2 hours)
**Files:** `src/rewrite/ref_resolution.rs`

### Description

`resolve_refs()` recursively resolves `$ref` pointers without tracking which refs have been visited. Circular references (e.g., `$defs/a` → `$defs/b` → `$defs/a`) cause unbounded recursion and stack overflow.

### Acceptance criteria

- Add a `HashSet<String>` (or equivalent) of visited ref paths passed through recursive calls.
- When a ref is encountered that's already in the visited set, return `SubtypeError::UnsupportedFeatures` with a message about circular references.
- Add test: direct self-reference (`$ref: "#/$defs/a"` where `$defs/a` contains `$ref: "#/$defs/a"`) returns an error.
- Add test: indirect cycle (`a → b → a`) returns an error.
- Existing ref resolution tests continue to pass.

---

## Task 3: Remove unused `miette` dependency

**Priority:** P1 — Build hygiene
**Effort:** Trivial (< 15 min)
**Files:** `Cargo.toml`

### Description

The `miette` crate (with `fancy` feature) is declared as a dependency but never imported or used anywhere in the source tree. It pulls in ~40 transitive dependencies and slows compilation.

### Acceptance criteria

- Remove `miette` from `[dependencies]` in `Cargo.toml`.
- `cargo build` and `cargo test` pass cleanly.

---

## Task 4: Add `Send + Sync` bounds to `RewriteRule` trait

**Priority:** P1 — Enables multi-threaded usage
**Effort:** Small (< 30 min)
**Files:** `src/rewrite/mod.rs`, potentially `src/preset.rs`

### Description

The `RewriteRule` trait only requires `'static` but not `Send + Sync`. All existing implementations are unit structs (trivially `Send + Sync`), but the trait bound doesn't guarantee it. This means `Preset` and `SubtypeChecker` are not `Send + Sync`, preventing use in async web frameworks (e.g., `axum`, `actix`).

### Acceptance criteria

- Change trait definition to `pub trait RewriteRule: Send + Sync + 'static`.
- Add a compile-time assertion that `SubtypeChecker` is `Send + Sync` (e.g., `const _: () = { fn assert_send_sync<T: Send + Sync>() {} fn check() { assert_send_sync::<SubtypeChecker>(); } };`).
- All existing code compiles without changes (all implementations already satisfy these bounds).

---

## Task 5: Add DFA state limit to `regex-algebra` product construction

**Priority:** P1 — Prevents resource exhaustion
**Effort:** Medium (~2-3 hours)
**Files:** `crates/regex-algebra/src/lib.rs`

### Description

The `product()` function performs BFS product construction with no upper bound on the number of states. Two DFAs with N and M states can produce up to N*M states, each consuming ~1 KB. An adversarial pair of patterns could exhaust memory.

### Acceptance criteria

- Add a configurable state limit (default: e.g., 100,000 states) to `product()`.
- When the limit is exceeded, return `Error::Unsupported("DFA product exceeded state limit")`.
- Propagate the error through `intersect()`, `union()`, `is_subset()`, `complement()` (change signatures to return `Result`).
- Update all callers in the main crate to handle the new `Result` return.
- Add a test that triggers the limit with a pathological pattern pair.

---

## Task 6: Extract duplicated rewrite helper functions

**Priority:** P2 — Code hygiene
**Effort:** Small (~1 hour)
**Files:** `src/rewrite/canonicalize.rs`, `src/rewrite/simplify.rs`, new `src/rewrite/helpers.rs`

### Description

`get_entry()`, `without_key()`, `without_keys()`, `make_string()`, `make_object()`, `make_array()`, `synthetic()`, and `bottom()` are duplicated between `canonicalize.rs` and `simplify.rs`. Extract them to a shared `rewrite::helpers` module.

### Acceptance criteria

- Create `src/rewrite/helpers.rs` with the shared functions.
- Update `canonicalize.rs` and `simplify.rs` to import from `helpers`.
- Remove the duplicated definitions.
- All tests pass unchanged.

---

## Task 7: Improve number uninhabitedness check

**Priority:** P2 — Correctness improvement
**Effort:** Small (~1-2 hours)
**Files:** `src/check/inhabited.rs`

### Description

The `is_typed_uninhabited` check for numbers does not properly account for exclusive bounds. It should detect cases like `exclusiveMinimum: 5, exclusiveMaximum: 6` with `multipleOf: 1` as uninhabited (open interval `(5, 6)` contains no integers).

### Acceptance criteria

- When both bounds are exclusive and equal (`exclusive_min == exclusive_max`), return uninhabited.
- When both bounds are exclusive, `multipleOf` is set, and no multiple exists in the open interval, return uninhabited.
- Add tests for: `exclusiveMin: 5, exclusiveMax: 6, multipleOf: 1` → uninhabited.
- Add tests for: `exclusiveMin: 5, exclusiveMax: 7, multipleOf: 1` → inhabited (6 is in range).
- Conservative behavior preserved: when unsure, return `false` (inhabited).

---

## Task 8: Improve `multipleOf` floating-point robustness

**Priority:** P2 — Correctness for edge cases
**Effort:** Small (~1-2 hours)
**Files:** `src/check/number.rs`

### Description

The `multipleOf` divisibility check uses `(ratio - ratio.round()).abs() > 1e-10`, which can fail for large values due to floating-point precision loss. For example, with `sub_m = 6e15` and `sup_m = 3`, the ratio computation may lose precision.

### Acceptance criteria

- When both `sub_m` and `sup_m` are integers (`.fract() == 0.0`), use integer arithmetic for the check.
- For non-integer values, use `f64::rem_euclid` or a more robust tolerance approach.
- Add test with large integer multiples (e.g., `multipleOf: 6_000_000_000_000_000` <: `multipleOf: 3`).
- Existing tests pass.

---

## Task 9: Remove dead code (`TypeSet`) and clean up dependencies

**Priority:** P3 — Code hygiene
**Effort:** Trivial (< 30 min)
**Files:** `src/schema/typeset.rs`, `src/schema/mod.rs`, `Cargo.toml`

### Description

`TypeSet` is defined, exported, and tested, but never used outside its own module. Additionally, `snafu` and `thiserror` serve the same purpose across the main crate and `regex-algebra` sub-crate.

### Acceptance criteria

- Either remove `TypeSet` entirely, or gate it behind `#[allow(dead_code)]` with a `// TODO: planned for ...` comment.
- Consider consolidating `snafu` → `thiserror` (or vice versa) across both crates for consistency. (This may be deferred to a separate task if scope is large.)
- Add `#[must_use]` to `SubtypeRelation::is_subtype()`, `BoolSet::is_empty()`, `BoolSet::is_subset_of()`.
- `cargo build` and `cargo test` pass cleanly.

---

## Task 10: Improve `allOf` sub handling in subtype checker

**Priority:** P3 — Semantic completeness
**Effort:** Large (~4-8 hours)
**Files:** `src/check/mod.rs`, potentially `src/schema/reduced.rs`

### Description

The current `AllOf` sub handling only checks if any individual branch is a subtype of `sup`. This misses cases where the intersection of branches would be a subtype but no single branch is. For example, `allOf[{minimum: 0}, {maximum: 10}] <: {minimum: 0, maximum: 10}` incorrectly returns `NotSubtype`.

### Acceptance criteria

- When all `allOf` branches are `Typed` with the same JSON type, merge their constraints and check the merged schema against `sup`.
- At minimum, handle the common case of same-typed number/string/array/object constraint intersection.
- Add tests for: `allOf[{min: 0}, {max: 10}] <: {min: 0, max: 10}` → Subtype.
- Add tests for: `allOf[{type: string}, {type: number}]` → still Bottom (heterogeneous).
- Conservative fallback preserved for cases that can't be merged.

---

## Task 11: Add `structural_eq` order-insensitive object comparison

**Priority:** P3 — Test reliability
**Effort:** Small (~1 hour)
**Files:** `src/located/value.rs`

### Description

`structural_eq` compares object key-value pairs positionally. Two objects with identical content in different key order are considered unequal. This could cause flaky tests if rewrite rules produce keys in different orders.

### Acceptance criteria

- Change object comparison in `structural_eq` to be order-insensitive (e.g., for each pair in `a`, find a matching pair in `b`).
- Handle duplicate keys correctly (count-based matching).
- Add test: `{"a": 1, "b": 2}.structural_eq({"b": 2, "a": 1})` → `true`.

---

## Task 12: Document ECMA-262 regex limitations

**Priority:** P3 — User documentation
**Effort:** Trivial (< 30 min)
**Files:** `README.md` or `src/lib.rs` (module-level doc comment)

### Description

JSON Schema specifies ECMA-262 regular expressions, but the `regex-automata` crate only supports a subset. Features like lookahead, lookbehind, and backreferences will cause a compile-time error. This limitation should be clearly documented.

### Acceptance criteria

- Add a "Limitations" section to the crate-level documentation listing unsupported ECMA-262 features.
- Note that the fail-closed behavior means unsupported patterns produce an error (not silent incorrect results).
