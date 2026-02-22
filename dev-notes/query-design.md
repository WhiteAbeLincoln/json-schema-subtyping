# Query-Based Schema Subtyping Design

## Context

The current `JsonSchema` trait is a coalgebra that pushes the full `SchemaObject` struct
(30+ fields) out via `try_view()`. This makes `is_subtype` a monolithic field-by-field
comparison, hardcodes draft 2020-12 keywords, and requires touching 6+ sites to add a keyword.

We want a **query-based** design where the subtype algorithm *pulls* individual keyword
constraints from an opaque schema. This makes keyword comparison modular, DRY, and
extensible to new vocabularies — while avoiding dynamic dispatch and supporting
cross-type comparison (e.g., `serde_json::Value` vs a JSONC schema type).

## Core Traits

### 1. `Keyword` — a keyword that knows its own subtyping semantics

```rust
/// A JSON Schema keyword. Defines the extracted value type and subtype comparison.
///
/// The GAT `Extracted<'a, Schema: 'a>` serves double duty:
/// - Validation keywords: ignore the Schema param (e.g., `Option<f64>`)
/// - Applicator keywords: reference Schema for recursive children (e.g., `Option<&'a Schema>`)
trait Keyword: 'static {
    type Extracted<'a, Schema: 'a>;

    /// Is sub's constraint at least as restrictive as sup's?
    /// `is_subtype` callback handles recursive sub-schema comparison.
    fn check_subtype<Sub, Sup>(
        sub: &Self::Extracted<'_, Sub>,
        sup: &Self::Extracted<'_, Sup>,
        is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool;
}
```

No dynamic dispatch, no downcasting. The keyword type parameter statically determines
the value type. Different schema types extract the same `Extracted` type for the same keyword.

### 2. `QuerySchema` — base trait with error type and top/bottom classification

```rust
/// Base trait for a queryable schema. Provides the error type shared by all
/// keyword queries, and top/bottom/constrained classification.
trait QuerySchema: Sized {
    type Error;

    /// Classify this schema as top (accepts all), bottom (rejects all),
    /// or constrained (has keywords to compare).
    fn kind(&self) -> Result<SchemaKind, Self::Error>;
}

enum SchemaKind {
    Top,        // true / {}
    Bottom,     // false
    Constrained,
}
```

For `serde_json::Value`, `Error = ViewError` and `kind()` returns `Err` for non-bool/non-object
values. For infallible types, `Error = Infallible`.

### 3. `Get<K>` — query a schema for a keyword's value (fallible)

```rust
/// A schema that can answer queries about keyword K.
trait Get<K: Keyword>: QuerySchema {
    fn get(&self) -> Result<K::Extracted<'_, Self>, Self::Error>;
}
```

Both `serde_json::Value` and a JSONC type implement `Get<MaximumKw>` returning
`Result<Option<f64>, _>`. Invalid keyword types (e.g., `"maximum": "ten"`) produce errors.
The keyword type defines the common value type; concrete schemas just extract it.

Errors propagate from `get()` calls through `check_keywords` to `is_subtype`'s return type.
The keyword's `check_subtype` itself stays `-> bool` — it only runs on successfully
extracted values.

## Keyword Implementations

Keywords group by comparison pattern. Each is a zero-sized struct implementing `Keyword`.

### Upper bound pattern: sub ≤ sup

`maximum`, `exclusiveMaximum`, `maxLength`, `maxItems`, `maxProperties`, `maxContains`

```rust
struct MaximumKw;
impl Keyword for MaximumKw {
    type Extracted<'a, S: 'a> = Option<f64>;
    fn check_subtype<Sub, Sup>(
        sub: &Option<f64>, sup: &Option<f64>,
        _is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        match (sub, sup) {
            (_, None) => true,              // sup unconstrained
            (None, Some(_)) => false,       // sub unconstrained, sup constrained
            (Some(s), Some(p)) => *s <= *p,
        }
    }
}
```

A macro can generate all 6 upper-bound keywords from just the name + value type.

### Lower bound pattern: sub ≥ sup

`minimum`, `exclusiveMinimum`, `minLength`, `minItems`, `minProperties`, `minContains`

Same shape, reversed comparison (`*s >= *p`).

### Subset: `type`

```rust
struct TypeKw;
impl Keyword for TypeKw {
    type Extracted<'a, S: 'a> = Option<TypeSet>;
    fn check_subtype<Sub, Sup>(sub: &Option<TypeSet>, sup: &Option<TypeSet>, ..) -> bool {
        match (sub, sup) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(s), Some(p)) => s.is_subset(*p),
        }
    }
}
```

### Superset: `required`

```rust
struct RequiredKw;
impl Keyword for RequiredKw {
    type Extracted<'a, S: 'a> = Vec<&'a str>;
    fn check_subtype<Sub, Sup>(sub: &[&str], sup: &[&str], ..) -> bool {
        sup.iter().all(|s| sub.contains(s))  // sub ⊇ sup
    }
}
```

### Applicator — covariant recursive: `properties`

```rust
struct PropertiesKw;
impl Keyword for PropertiesKw {
    type Extracted<'a, S: 'a> = Vec<(&'a str, &'a S)>;  // key-value pairs
    fn check_subtype<Sub, Sup>(
        sub: &[(&str, &Sub)], sup: &[(&str, &Sup)],
        is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        sup.iter().all(|(key, sup_child)| {
            match sub.iter().find(|(k, _)| k == key) {
                Some((_, sub_child)) => is_subtype(sub_child, sup_child),
                None => false,  // absent in sub = unconstrained = NOT subtype of constrained sup
            }
        })
    }
}
```

### Applicator — optional recursive: `items`, `contains`, `additionalProperties`, etc.

```rust
struct ItemsKw;
impl Keyword for ItemsKw {
    type Extracted<'a, S: 'a> = Option<&'a S>;
    fn check_subtype<Sub, Sup>(
        sub: &Option<&Sub>, sup: &Option<&Sup>,
        is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        match (sub, sup) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(s), Some(p)) => is_subtype(s, p),
        }
    }
}
```

### Applicator — list recursive: `prefixItems`

```rust
struct PrefixItemsKw;
impl Keyword for PrefixItemsKw {
    type Extracted<'a, S: 'a> = Vec<&'a S>;
    fn check_subtype<Sub, Sup>(
        sub: &[&Sub], sup: &[&Sup],
        is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
    ) -> bool {
        // Each position: sub's schema ≤ sup's schema
        // sub may have more prefix items (OK), fewer (those positions unconstrained)
        sup.iter().enumerate().all(|(i, sup_child)| {
            match sub.get(i) {
                Some(sub_child) => is_subtype(sub_child, sup_child),
                None => false,
            }
        })
    }
}
```

## Vocabulary Bundling

A vocabulary is a super-trait that bundles `ClassifySchema + Get<K>` for all its keywords.

```rust
trait Draft2020_12:
    ClassifySchema
    + Get<TypeKw>
    + Get<MaximumKw> + Get<ExclusiveMaximumKw>
    + Get<MinimumKw> + Get<ExclusiveMinimumKw>
    + Get<MultipleOfKw>
    + Get<MaxLengthKw> + Get<MinLengthKw>
    + Get<MaxItemsKw> + Get<MinItemsKw> + Get<UniqueItemsKw>
    + Get<MaxContainsKw> + Get<MinContainsKw>
    + Get<MaxPropertiesKw> + Get<MinPropertiesKw>
    + Get<RequiredKw>
    + Get<PropertiesKw> + Get<PatternPropertiesKw>
    + Get<AdditionalPropertiesKw> + Get<PropertyNamesKw>
    + Get<ItemsKw> + Get<PrefixItemsKw> + Get<ContainsKw>
    // composition keywords (allOf, anyOf, oneOf, not, if/then/else)
    // handled separately — see below
    + Sized
{}

// Blanket impl: any type satisfying all bounds is Draft2020_12
impl<T> Draft2020_12 for T where T: ClassifySchema + Get<TypeKw> + Get<MaximumKw> + ... {}
```

A `define_vocabulary!` macro generates the super-trait, blanket impl, and the
keyword-by-keyword comparison function:

```rust
define_vocabulary! {
    Draft2020_12,
    keywords: [
        TypeKw, MaximumKw, ExclusiveMaximumKw, MinimumKw, ExclusiveMinimumKw,
        MultipleOfKw, MaxLengthKw, MinLengthKw, MaxItemsKw, MinItemsKw,
        UniqueItemsKw, MaxContainsKw, MinContainsKw, MaxPropertiesKw,
        MinPropertiesKw, RequiredKw, PropertiesKw, PatternPropertiesKw,
        AdditionalPropertiesKw, PropertyNamesKw, ItemsKw, PrefixItemsKw,
        ContainsKw,
    ]
}
```

This generates a `check_keywords<Sup: Draft2020_12, Sub: Draft2020_12>` function that
calls `K::check_subtype` for each keyword in the list, passing
`is_subtype` as the recursive callback.

## `is_subtype` Algorithm

```rust
pub fn is_subtype<Sup, Sub>(sup: &Sup, sub: &Sub) -> Result<SubtypeRelation, SubtypeError>
where
    Sup: Draft2020_12,
    Sub: Draft2020_12,
    Sup::Error: Into<SubtypeError>,
    Sub::Error: Into<SubtypeError>,
{
    match (sup.kind()?, sub.kind()?) {
        (SchemaKind::Top, _) => Ok(Subtype),
        (_, SchemaKind::Bottom) => Ok(Subtype),
        (SchemaKind::Bottom, _) => Ok(NotSubtype(..)),
        (_, SchemaKind::Top) => Ok(NotSubtype(..)),
        (SchemaKind::Constrained, SchemaKind::Constrained) => {
            // 1. Check all vocabulary keywords (generated by define_vocabulary!)
            //    Each keyword: sup.get()? and sub.get()? then K::check_subtype
            let keywords_ok = check_keywords(sup, sub)?;
            // 2. Handle composition (allOf, anyOf, oneOf, not) — see below
            // 3. Detect unsatisfiable sub (effective bottom) — see below
            ...
        }
    }
}
```

The `?` on `kind()` and `get()` propagates schema parsing errors. Keyword comparison
(`check_subtype`) itself is infallible — it only runs on successfully extracted values.

## Composition Keywords — Separate Mechanism

`allOf`, `anyOf`, `oneOf`, `not`, `if/then/else` can't be checked keyword-by-keyword
because they affect the **whole schema's** semantics:

- `{allOf: [A, B]}` — sub must be subtype of both A and B (whole-schema comparison)
- `{anyOf: [A, B]}` — sub must be subtype of A or B
- `{not: A}` — the complement; deeply complex for subtyping

These are handled at the `is_subtype` level, not via `Keyword`. They'll use `Get<K>`
to extract the sub-schema lists, but their comparison logic accesses the whole schema:

```rust
trait CompositionQuery {
    fn all_of(&self) -> Vec<&Self>;
    fn any_of(&self) -> Vec<&Self>;
    fn one_of(&self) -> Vec<&Self>;
    fn not(&self) -> Option<&Self>;
}
```

Or they can still use `Get<AllOfKw>` etc., but the comparison function lives in
`is_subtype` rather than in `Keyword::check_subtype`.

## Cross-Keyword Interactions

Handled via pre-processing/normalization, not in the keyword trait:

1. **Unsatisfiable detection** (`min > max` → effective bottom):
   Checked as a pre-pass before keyword comparison. Could be a method on the vocabulary
   or a standalone function.

2. **exclusive vs. inclusive bounds + integer type**:
   `{exclusiveMaximum: 11, type: "integer"}` ≡ `{maximum: 10, type: "integer"}`.
   Handled either by normalizing bounds before comparison, or by a combined
   `NumericBoundsKw` that extracts all numeric constraints together and compares them
   as a unit.

## Relationship to Existing Code

### Remove coalgebra types (clean slate)

Delete `SchemaObject`, `SchemaF`, `Schema`, `Annotated`, and the `JsonSchema` trait.
The query-based traits are the only interface. `serde_json::Value` is the primary
concrete type. An owned schema representation can be reintroduced later if needed —
it would just implement `QuerySchema + Get<K>` like any other type.

Files to remove: `src/schema/node.rs`, `src/schema/fixpoint.rs`, `src/schema/traits.rs`.
Keep: `src/schema/typeset.rs`, `src/schema/json_value.rs` (still needed by keyword impls).

### `serde_json::Value` — primary concrete type

Each keyword's `Get` impl parses from the JSON map on-the-fly:

```rust
impl Get<MaximumKw> for Value {
    fn get(&self) -> Result<Option<f64>, ViewError> {
        let Some(obj) = self.as_object() else { return Ok(None) };
        match obj.get("maximum") {
            None => Ok(None),
            Some(v) => Ok(Some(v.as_f64().ok_or(ViewError::InvalidKeywordType {
                keyword: "maximum", expected: "a number",
            })?)),
        }
    }
}
```

## Files to Create/Modify

| File | Action | Purpose |
|---|---|---|
| `src/schema/keyword.rs` | **Create** | `Keyword` trait, `Get` trait, `QuerySchema`, `SchemaKind` |
| `src/schema/keywords/` | **Create dir** | Keyword ZSTs + impls grouped by pattern (numeric, string, array, object, type, applicators) |
| `src/schema/vocabulary.rs` | **Create** | `define_vocabulary!` macro, `Draft2020_12` trait |
| `src/schema/mod.rs` | **Rewrite** | Export new modules, re-export `TypeSet`, `JsonValue` |
| `src/lib.rs` | **Rewrite** | Implement `is_subtype` using the new traits |
| `src/serde_impl.rs` | **Rewrite** | `QuerySchema` + `Get<K>` impls for `serde_json::Value` |
| `src/schema/node.rs` | **Delete** | Replaced by keyword system |
| `src/schema/fixpoint.rs` | **Delete** | Replaced by keyword system |
| `src/schema/traits.rs` | **Delete** | Replaced by `QuerySchema` + `Get<K>` |
| `src/output.rs` | **Keep** | Error reporting structures still needed |
| `src/schema/typeset.rs` | **Keep** | `TypeSet` used by `TypeKw` |
| `src/schema/json_value.rs` | **Keep** | `JsonValue` used by `ConstKw`/`EnumKw` |

## Verification

1. `cargo check` — new trait system compiles, GATs resolve correctly
2. `cargo test` — existing test suite passes against `serde_json::Value` impls
3. Key test patterns to verify:
   - Top/bottom handling (top-and-bottom.nix)
   - Upper/lower bound keywords (numeric-instances.nix, string-instances.nix, basic-array.nix)
   - Subset/superset keywords (type-keyword.nix, basic-object.nix)
   - Recursive applicators (object-applicators/properties.nix)
   - Invalid schema error propagation (ViewError from `Get<K>`)

## Open Questions

1. **Error reporting in keywords**: Should `check_subtype` return `bool` or a richer
   error type (with keyword name, expected/actual values) for `NotSubtype` diagnostics?

2. **Vocabulary composition**: Should vocabularies compose? E.g.,
   `trait OpenAPI: Draft2020_12 + Get<DiscriminatorKw> {}`.

3. **Macro vs. manual**: The `define_vocabulary!` macro is convenient but opaque.
   Alternative: hand-written vocabulary traits + a `check_keywords!` macro that just
   generates the comparison chain.
