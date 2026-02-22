# Recursion-Scheme Based Schema Representation

Commit: `8e9a0bb` — *Implement JsonSchema trait with recursion-scheme based schema types*

> **Status**: Superseded by the query-based keyword system (`e7fa756`).
> See [query-design.md](./query-design.md) for the replacement design.

## Goal

Represent JSON Schema (draft 2020-12) as a recursive data type using recursion
schemes, enabling generic traversal and annotation of schema trees.

## Key References

- [Practical recursion schemes in Rust: traversing and extending trees (Tweag)](https://www.tweag.io/blog/2025-04-10-rust-recursion-schemes/)
- [Elegant and performant recursion in Rust](https://recursion.wtf/posts/rust_schemes/)
- [Fully generic recursion in Rust](https://recursion.wtf/posts/rust_schemes_2/)
- [ASTs with Fix and Free](https://chrispenner.ca/posts/asts-with-fix-and-free)

## Architecture

### Base functor: `SchemaF<S, A>`

The core type is a base functor parameterized over two type variables:

- **`S`** — string type (`String` for owned, `&str` for borrowed views)
- **`A`** — recursive child type (sub-schemas)

```
SchemaF<S, A>
├── True           — top type, accepts everything
├── False          — bottom type, rejects everything
└── Schema(Box<SchemaObject<S, A>>)
```

`SchemaObject<S, A>` holds all draft 2020-12 keywords as struct fields. It has
30+ fields covering validation (type, numeric, string, array, object, const/enum),
applicators (prefixItems, items, properties, allOf/anyOf/oneOf, if/then/else,
unevaluated), and dependencies.

### Fixed points: `Schema` and `Annotated<Ann>`

Two recursive types tie the knot:

- **`Schema`** = `Fix SchemaF` — plain schema tree. Wraps `Box<SchemaF<String, Schema>>`.
- **`Annotated<Ann>`** = `Cofree SchemaF Ann` — each node carries an annotation
  (e.g., `Location` for source position). Wraps `(Ann, Box<SchemaF<String, Annotated<Ann>>>)`.

### Coalgebra trait: `JsonSchema`

```rust
trait JsonSchema {
    type ViewError;
    fn try_view(&self) -> Result<SchemaF<&str, &Self>, Self::ViewError>;
}
```

The trait unfolds one layer of schema structure. The subtyping algorithm calls
`try_view()` recursively to traverse. `ViewError = Infallible` for statically
known types; a `JsonSchemaExt` blanket trait provides an infallible `.view()`
method for these.

### Functor operations

`SchemaF` and `SchemaObject` both provide:

- **`map<B>(self, f: FnMut(A) -> B)`** — transform children (the functor map)
- **`borrow(&self) -> SchemaF<&str, &A>`** — convert owned layer to a borrowed view
  (used by `Schema`/`Annotated` to implement `try_view()`)

## Design Decisions

### Sentinel values vs. `Option` for keyword absence

Fields use sentinels where "absent" equals the meta-schema default:

| Pattern | Example | Absent value |
|---|---|---|
| Default is identity/unconstrained | `type` | `TypeSet::all()` |
| Default is zero | `minLength`, `minItems`, `minProperties` | `0` |
| Default is false | `uniqueItems` | `false` |
| Absent is semantically distinct | `maximum`, `maxLength`, `items`, `not`, ... | `Option::None` |

This avoids wrapping every field in `Option` while preserving round-trip fidelity.

### `Vec<(S, A)>` instead of `BTreeMap<S, A>` for keyed collections

Properties, patternProperties, dependentSchemas, and dependentRequired use
`Vec<(S, A)>` instead of a tree map. Reason: `borrow()` needs to convert
`SchemaObject<String, A>` → `SchemaObject<&str, &A>`, and you can't borrow a
`BTreeMap<String, A>` as `BTreeMap<&str, &A>` without rebuilding the tree. A Vec
of tuples is cheaper to reborrow.

Tradeoff: O(n) key lookup instead of O(log n). For the subtyping algorithm, the
plan was to build a temporary HashMap at the comparison site, or add a
`property(name) -> Option<&Self>` method to `JsonSchema`.

### `JsonValue` — standalone JSON value type

A self-contained `JsonValue` enum (null, bool, number, string, array, object)
avoids a hard dependency on `serde_json` in the core types. Used for `const` and
`enum` keywords. Object equality is order-independent (unordered comparison via
nested iteration). Objects use `Vec<(String, JsonValue)>` rather than a map since
only equality checks are needed, never key lookup.

### `TypeSet` — bitflags for the type keyword

Seven JSON Schema types (null, boolean, object, array, number, string, integer)
packed into a `u8` bitset via `bitflags`. `TypeSet::all()` represents absent/
unconstrained. Subtype check is just `sub.is_subset(sup)`.

### Feature-gated `serde_json::Value` implementation

`impl JsonSchema for serde_json::Value` lives in `src/serde_impl.rs` behind
`feature = "serde"`. It parses keywords on-the-fly from the JSON map on each
`try_view()` call (zero-copy references into the original `Value` tree). Returns
`ViewError` for invalid schemas (non-bool/non-object values, wrong keyword types,
unknown type names).

### `Located` trait for error reporting

A separate `Located` trait provides optional position information (`Location`)
for error reporting during subtype checks. Default impl returns `None`.
`Annotated<Location>` and `Annotated<Option<Location>>` provide real locations.

## File Layout

```
src/schema/
├── mod.rs          — re-exports
├── node.rs         — SchemaF, SchemaObject, map(), borrow()
├── fixpoint.rs     — Schema (Fix), Annotated (Cofree)
├── traits.rs       — JsonSchema, JsonSchemaExt, Located
├── typeset.rs      — TypeSet bitflags
└── json_value.rs   — JsonValue enum
src/serde_impl.rs   — impl JsonSchema for serde_json::Value
```

## Why This Was Superseded

The coalgebra approach pushes the **full** `SchemaObject` (30+ fields) on every
`try_view()` call. This makes the subtype algorithm a monolithic field-by-field
comparison, hardcodes the keyword set to draft 2020-12, and requires touching 6+
sites to add a new keyword. The query-based design (see `query-design.md`) lets
the algorithm **pull** individual keywords on demand, making comparison modular
and extensible to new vocabularies.

Types kept from this design: `TypeSet`, `JsonValue`.
Types removed: `SchemaF`, `SchemaObject`, `Schema`, `Annotated`, `JsonSchema` trait.

---

## Original Implementation Plan

*From the `/plan` session that produced this commit.*

### Context

The project needs a `JsonSchema` trait that the `is_subtype` algorithm can operate on. Currently the trait is empty. We need:
- `SchemaF<S, A>` base functor (one layer of JSON Schema, `S` = string type, `A` = child type)
- `SchemaObject<S, A>` with all draft 2020-12 keywords, using sentinels where absent = default
- `TypeSet` bitset for the `type` keyword (via `bitflags`)
- `JsonValue` for `const`/`enum` keywords
- `Schema` (Fix point) and `Annotated<Ann>` (Cofree) recursive types
- `impl JsonSchema for serde_json::Value` (feature-gated)
- `Located` trait for optional position info

### Module structure

Replace `src/schema.rs` with a `src/schema/` module directory containing all schema-related types. `serde_impl.rs` stays at top level.

```
src/
  lib.rs              — re-exports, is_subtype
  output.rs           — Location, Output, OutputError (+ moved type aliases)
  serde_impl.rs       — impl JsonSchema for serde_json::Value (feature-gated)
  schema/
    mod.rs            — pub use re-exports for all submodules
    typeset.rs        — TypeSet (bitflags)
    json_value.rs     — JsonValue enum
    node.rs           — SchemaF, SchemaObject, map/borrow impls
    traits.rs         — JsonSchema trait, Located trait
    fixpoint.rs       — Schema (Fix), Annotated (Cofree)
```

### Files to create/modify

| File | Action |
|------|--------|
| `src/schema.rs` | **DELETE** (replaced by `src/schema/` directory) |
| `src/schema/mod.rs` | **CREATE** — re-exports from submodules |
| `src/schema/typeset.rs` | **CREATE** — `TypeSet` via `bitflags` |
| `src/schema/json_value.rs` | **CREATE** — `JsonValue` enum |
| `src/schema/node.rs` | **CREATE** — `SchemaF`, `SchemaObject`, `Default`, `map`, `borrow` |
| `src/schema/traits.rs` | **CREATE** — `JsonSchema` trait, `Located` trait |
| `src/schema/fixpoint.rs` | **CREATE** — `Schema`, `Annotated<Ann>` |
| `src/serde_impl.rs` | **CREATE** — `impl JsonSchema for serde_json::Value` (feature-gated) |
| `src/output.rs` | **MODIFY** — move `JsonPointer`/`FilePosition` type aliases here |
| `src/lib.rs` | **MODIFY** — update module declarations, re-exports |
| `Cargo.toml` | **MODIFY** — add `bitflags`, `default = ["serde"]`, test `required-features` |
| `tests/test_suite.rs` | **MODIFY** — remove `ValueSchema`, use `serde_json::Value` directly |

### Step-by-step implementation

#### Step 1: `src/output.rs` — move type aliases

Move `JsonPointer = String` and `FilePosition = Range<usize>` definitions from `schema.rs` into `output.rs`. Remove the `use crate::{...}` import.

#### Step 2: Delete `src/schema.rs`, create `src/schema/` directory

#### Step 3: `src/schema/typeset.rs`

```rust
bitflags::bitflags! {
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
```

Add `from_name(&str) -> Option<Self>` helper. `TypeSet::all()` = absent/unconstrained, `TypeSet::empty()` = no types.

#### Step 4: `src/schema/json_value.rs`

`JsonValue` enum: `Null`, `Bool(bool)`, `Number(f64)`, `String(String)`, `Array(Vec<JsonValue>)`, `Object(Vec<(String, JsonValue)>)`. Custom `PartialEq` via `json_eq` (unordered object keys). Feature-gated `From<&serde_json::Value>`.

#### Step 5: `src/schema/node.rs` — SchemaF and SchemaObject

- `SchemaF<S, A>` — `True | False | Schema(SchemaObject<S, A>)`
- `SchemaObject<S, A>` — all draft 2020-12 keywords:
  - Sentinels: `r#type: TypeSet` (all()), `min_length/min_items/min_properties: u64` (0), `unique_items: bool` (false), `required: Vec<S>` (empty), `properties/pattern_properties/dependent_schemas: Vec<(S, A)>` (empty), `dependent_required: Vec<(S, Vec<S>)>` (empty), `all_of/any_of/one_of/prefix_items: Vec<A>` (empty)
  - Options: `multiple_of/maximum/exclusive_maximum/minimum/exclusive_minimum: Option<f64>`, `max_length/max_items/max_properties/min_contains/max_contains: Option<u64>`, `pattern: Option<S>`, `items/contains/additional_properties/property_names/not/if/then/else/unevaluated_items/unevaluated_properties: Option<A>`, `r#const: Option<JsonValue>`, `r#enum: Option<Vec<JsonValue>>`
- `Default for SchemaObject<S, A>` — no bounds needed on S/A
- `SchemaF::map<B>(self, FnMut(A) -> B) -> SchemaF<S, B>` — Functor
- `SchemaObject::map<B>(self, FnMut(A) -> B) -> SchemaObject<S, B>`
- `SchemaF::borrow(&self) -> SchemaF<&str, &A> where S: AsRef<str>`
- `SchemaObject::borrow(&self) -> SchemaObject<&str, &A> where S: AsRef<str>`

#### Step 6: `src/schema/traits.rs`

```rust
pub trait JsonSchema {
    fn view(&self) -> SchemaF<&str, &Self>;
}

pub trait Located {
    fn location(&self) -> Option<&Location> { None }
}
```

#### Step 7: `src/schema/fixpoint.rs`

- `Schema(pub Box<SchemaF<String, Schema>>)` — implements `JsonSchema` via `self.0.borrow()`, `Located` (default `None`)
- `Annotated<Ann>(pub Ann, pub Box<SchemaF<String, Annotated<Ann>>>)` — implements `JsonSchema` via `self.1.borrow()`
- `Located` for `Annotated<Location>` returns `Some(&self.0)`
- `Located` for `Annotated<Option<Location>>` returns `self.0.as_ref()`
- Helper: `Annotated::annotation(&self) -> &Ann`

#### Step 8: `src/schema/mod.rs`

Re-export everything publicly:
```rust
mod typeset;
mod json_value;
mod node;
mod traits;
mod fixpoint;

pub use typeset::TypeSet;
pub use json_value::JsonValue;
pub use node::{SchemaF, SchemaObject};
pub use traits::{JsonSchema, Located};
pub use fixpoint::{Schema, Annotated};
```

#### Step 9: `src/serde_impl.rs` (feature-gated)

`impl JsonSchema for serde_json::Value`:
- `Bool(true)` → `SchemaF::True`
- `Bool(false)` → `SchemaF::False`
- `Object(map)` → `SchemaF::Schema(parse_schema_object(map))` extracting all keywords
- Other → `SchemaF::True` (permissive fallback)

Helper `parse_schema_object` builds `SchemaObject<&str, &Value>` borrowing strings from the JSON.
Helper `parse_type_set` handles `"string"` or `["string", "null"]` forms.

#### Step 10: `src/lib.rs` — wire everything

```rust
pub mod output;
pub mod schema;
#[cfg(feature = "serde")]
mod serde_impl;

pub use schema::{JsonSchema, Located, SchemaF, SchemaObject, TypeSet, JsonValue, Schema, Annotated};
pub use output::{Location, JsonPointer, FilePosition};
```

Keep existing `SubtypeRelation`, `SubtypeError`, `is_subtype` unchanged (still `todo!()`).

#### Step 11: `Cargo.toml` + `tests/test_suite.rs`

In `Cargo.toml`:
- Add `bitflags = "2"` to `[dependencies]`
- Add `default = ["serde"]` to `[features]`
- Add `required-features = ["serde"]` to the `[[test]]` section

In `tests/test_suite.rs`:
- Remove `ValueSchema` struct and its `impl JsonSchema`
- Update `run_subtype_test` to pass `&serde_json::Value` directly to `is_subtype`
- Update imports (remove `JsonSchema` import)

### Verification

1. `cargo check` — compiles with default features (serde)
2. `cargo check --no-default-features` — compiles without serde
3. `cargo clippy --all-targets -- --deny warnings` — no warnings
4. `cargo test --no-run` — test binary compiles (tests won't pass since `is_subtype` is still `todo!()`)
5. `cargo fmt --check` — properly formatted
