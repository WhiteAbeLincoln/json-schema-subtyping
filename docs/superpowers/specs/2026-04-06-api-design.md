# JSON Schema Subtyping — API and Architecture Design

## Overview

A Rust library for determining subtyping relationships between JSON Schemas
(draft 2020-12). Implements the canonicalization, simplification, and subtype
checking algorithm from
[Type Safety with JSON Subschema](https://arxiv.org/abs/1911.12651), with a
pluggable API for custom rewrite rules and vocabulary extensions.

## Design Decisions

These decisions were made during brainstorming and are **not negotiable** during
implementation.

1. **Novel semantics** — users can inject arbitrary rewrite passes and custom
   keyword comparison logic, not just toggle presets.
2. **Typed inference** — inference rules operate on a typed intermediate
   representation (the reduced schema IR), not raw JSON.
3. **Rewrites on located JSON** — rewrite rules operate on a library-owned,
   location-annotated JSON tree (`LocatedValue`). We do not use `serde_json`
   because we need to track source locations through every rewrite for error
   messages.
4. **Fail closed** — unknown keywords (not claimed by built-in or extension
   inference) produce an `UnsupportedFeatures` error. Users must register
   inference rules for any custom keywords present.
5. **Library-owned concrete types** — the `LocatedValue` tree and `ReducedSchema`
   IR are concrete types owned by this library. Trait-based abstractions over
   these can be added later.
6. **Fixed rewrite phases** — three named phases (DraftConversion,
   Canonicalization, Simplification) run in a fixed order. Users register rules
   into specific phases.
7. **Per-node rewrites** — rewrite rules see one schema node at a time. The
   library handles traversal. `$ref` resolution is a built-in whole-tree
   pre-pass, not user-configurable.
8. **Typed extension extraction (Option B)** — custom vocabularies register both
   an extractor (parse phase, raw JSON to typed data) and an inference rule
   (check phase, typed data). Type-erased `dyn Any` storage in the IR.
9. **Preset + overlay (Approach C)** — built-in rules come from a `Preset` that
   users can swap entirely or extend additively. Default preset is
   `Preset::draft_2020_12()`.
10. **Recursion schemes for the JSON tree** — `LocatedValue` is structured as
    Cofree over a `JsonF` base functor, using the `recursion` crate. Rewrite
    rules are algebras over `JsonF`.

## Architecture

### Pipeline

```
parse JSON string
  → LocatedValue tree (spans + JSON pointers)
  → $ref resolution (internal, whole-tree)
  → Phase::DraftConversion (per-node, bottom-up, fixed point)
  → Phase::Canonicalization (per-node, bottom-up, fixed point)
  → Phase::Simplification (per-node, bottom-up, fixed point)
  → extract into ReducedSchema + ExtensionData
  → subtype checking (built-in keywords + extension inference)
```

### Located JSON Tree

The base functor for one layer of JSON:

```rust
#[derive(Debug, Clone)]
pub enum JsonF<A> {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<A>),
    Object(Vec<(A, A)>),
}
```

Source provenance — where in the original document this came from. A node may be
derived from multiple locations (e.g., the *object with properties* rule merges
`properties`, `additionalProperties`, and `patternProperties`):

```rust
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct Provenance {
    pub spans: SmallVec<[Span; 1]>,
    pub pointers: SmallVec<[JsonPointer; 1]>,
}
```

The recursive located JSON tree (Cofree JsonF Provenance):

```rust
pub struct LocatedValue {
    pub provenance: Provenance,
    pub node: JsonF<LocatedValue>,
}
```

### Rewrite Phases and Rules

Three fixed phases:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Convert non-2020-12 schemas to 2020-12 semantics.
    DraftConversion,
    /// Normalize into canonical form — one type per schema, explicit defaults,
    /// eliminate oneOf, merge properties into patternProperties, etc.
    Canonicalization,
    /// Eliminate enum/not/allOf/anyOf where possible.
    Simplification,
}
```

Per-node rewrite rule trait:

```rust
pub trait RewriteRule: 'static {
    /// Attempt to rewrite a single node. Returns:
    /// - `Ok(Some(new_node))` if the rule fires and produces a rewritten node
    /// - `Ok(None)` if the rule doesn't apply to this node
    /// - `Err(...)` if the node is malformed in a way this rule can detect
    ///
    /// The `Ok(None)` vs `Ok(Some(...))` distinction drives fixed-point
    /// iteration — the phase re-runs all rules on a node until every rule
    /// returns `Ok(None)`.
    fn rewrite(
        &self,
        prov: &Provenance,
        node: &JsonF<LocatedValue>,
    ) -> Result<Option<JsonF<LocatedValue>>, SubtypeError>;
}
```

Rules within a phase are applied bottom-up (children fully rewritten before
parent). All rules in a phase are applied repeatedly to each node until every
rule returns `Ok(None)` (fixed point). Errors abort the pipeline immediately.
The node is passed by reference; rules that fire produce a new node.

Built-in `$ref` resolution runs before all phases as an internal whole-tree pass.

### Reduced Schema IR

After all rewrites, the `LocatedValue` tree is extracted into a typed IR matching
the paper's post-simplification schema form:

```rust
pub enum ReducedSchema {
    Top(Provenance),
    Bottom(Provenance),
    Typed(TypedSchema),
    AnyOf(Provenance, Vec<ReducedSchema>),
    AllOfNot(Provenance, Vec<ReducedSchema>),
}

pub enum TypedSchema {
    Null(Provenance),
    Boolean(BooleanSchema),
    String(StringSchema),
    Number(NumberSchema),
    Array(ArraySchema),
    Object(ObjectSchema),
}
```

A value with its own provenance:

```rust
pub struct Located<T> {
    pub provenance: Provenance,
    pub value: T,
}
```

Type-specific schemas after canonicalization and simplification:

- **StringSchema** — only `pattern: Located<Regex>` (minLength/maxLength compiled
  into regex)
- **BooleanSchema** — only `enum_values: Located<BoolSet>`
- **NumberSchema** — `minimum`, `maximum`, `exclusive_minimum`,
  `exclusive_maximum`, `multiple_of`
- **ArraySchema** — `min_items`, `max_items`, `items: Vec<ReducedSchema>` (always
  a list), `additional_items: Box<ReducedSchema>`, `unique_items`
- **ObjectSchema** — `min_properties`, `max_properties`,
  `required: Vec<Located<String>>`,
  `pattern_properties: Vec<(Located<Regex>, ReducedSchema)>` (properties and
  additionalProperties merged in)

### Extension Vocabulary System

Custom vocabularies register both extraction and inference:

```rust
pub trait VocabExtension: 'static {
    type Extracted: 'static;

    /// Keywords this extension claims. Used for fail-closed validation.
    fn keywords(&self) -> &[&str];

    /// Extract typed data from remaining keywords in the LocatedValue.
    /// Called once per schema node after rewrite phases complete.
    fn extract(
        &self,
        keywords: &HashMap<&str, &LocatedValue>,
    ) -> Result<Self::Extracted, SubtypeError>;

    /// Check subtype relationship for this extension's keywords.
    fn check_subtype(
        &self,
        sub: &Self::Extracted,
        sup: &Self::Extracted,
        ctx: &mut SubtypeContext,
    ) -> Result<SubtypeRelation, SubtypeError>;
}
```

Extensions are stored as type-erased slots in `ExtensionData`:

```rust
pub struct ExtensionData {
    slots: Vec<Box<dyn Any>>,
}
```

Each `VocabExtension` gets a slot index at registration time. The extraction runs
after all rewrite phases, before subtype checking. Any keywords not claimed by
built-in extraction or registered extensions trigger a fail-closed error.

### Subtype Inference

Built-in inference follows the paper's rules on `ReducedSchema`:

1. **Uninhabited** — uninhabited sub is subtype of anything
2. **Top/Bottom** — Top is supertype of all, Bottom is subtype of all
3. **AnyOf** — non-overlapping union: each `sub_i` must have a `sup_j` where
   `sub_i <: sup_j`
4. **Same-typed** — type-specific comparison (null trivial, boolean enum subset,
   string regex containment, number range + multipleOf, array item-wise, object
   pattern-wise)
5. **AllOfNot** — residual negation for numbers, arrays, objects (the `subNumber`
   relation from the paper)

Extension inference runs after built-in keyword comparison within each typed
schema. All checks must pass for subtype to hold.

Recursive sub-schema comparison is available to extensions via `SubtypeContext`:

```rust
pub struct SubtypeContext<'a> {
    checker: &'a SubtypeChecker,
}

impl<'a> SubtypeContext<'a> {
    pub fn is_subtype(
        &mut self,
        sub: &ReducedSchema,
        sup: &ReducedSchema,
    ) -> Result<SubtypeRelation, SubtypeError>;
}
```

### Preset and Builder API

```rust
pub struct Preset {
    rewrites: HashMap<Phase, Vec<Box<dyn RewriteRule>>>,
    extensions: Vec<ExtensionSlot>,
}

impl Preset {
    pub fn draft_2020_12() -> Self { /* all built-in rules */ }
    pub fn add_rewrite(&mut self, phase: Phase, rule: impl RewriteRule + 'static) -> &mut Self;
    pub fn add_extension<E: VocabExtension>(&mut self, ext: E) -> &mut Self;
}

pub struct SubtypeChecker {
    preset: Preset,
}

impl SubtypeChecker {
    pub fn new() -> Self { Self::from_preset(Preset::draft_2020_12()) }
    pub fn from_preset(preset: Preset) -> Self;
    pub fn builder() -> SubtypeCheckerBuilder;
    pub fn parse<'a>(&self, source: &'a str) -> Result<ParsedSchema<'a>, SubtypeError>;
    pub fn reduce(&self, parsed: &ParsedSchema) -> Result<ReducedSchemaSet, SubtypeError>;
    pub fn check(&self, sub: &ReducedSchemaSet, sup: &ReducedSchemaSet) -> Result<SubtypeRelation, SubtypeError>;
    pub fn is_subtype(&self, sub: &str, sup: &str) -> Result<SubtypeRelation, SubtypeError>;
}

pub struct SubtypeCheckerBuilder { preset: Preset }

impl SubtypeCheckerBuilder {
    pub fn preset(self, preset: Preset) -> Self;
    pub fn add_rewrite(self, phase: Phase, rule: impl RewriteRule + 'static) -> Self;
    pub fn add_extension<E: VocabExtension>(self, ext: E) -> Self;
    pub fn build(self) -> SubtypeChecker;
}
```

### Error Reporting

`SubtypeRelation::NotSubtype(DetailedOutput)` carries provenance from both
schemas pointing to the specific keywords that caused the failure. Because every
node in `ReducedSchema` carries `Provenance` (with original spans and JSON
pointers), error messages can show exact source locations even after extensive
rewriting.

```rust
pub enum SubtypeRelation {
    Subtype,
    NotSubtype(DetailedOutput),
}

pub struct DetailedOutput {
    pub sub_locations: Vec<Provenance>,
    pub sup_locations: Vec<Provenance>,
    pub message: String,
}
```

## Usage Examples

```rust
// Simple — default 2020-12, parse and check
let checker = SubtypeChecker::new();
let result = checker.is_subtype(sub_json, sup_json)?;

// With extensions
let checker = SubtypeChecker::builder()
    .add_extension(DiscriminatorExt)
    .build();
let result = checker.is_subtype(sub_json, sup_json)?;

// Reuse reduced schemas for multiple comparisons
let checker = SubtypeChecker::new();
let sup = checker.reduce(&checker.parse(sup_json)?)?;
for sub_json in schemas {
    let sub = checker.reduce(&checker.parse(sub_json)?)?;
    let result = checker.check(&sub, &sup)?;
}

// Fully custom preset
let mut preset = Preset::default();
preset.add_rewrite(Phase::DraftConversion, Draft07To2020Rule);
preset.add_rewrite(Phase::Canonicalization, MyCanonRule);
preset.add_extension(MyVocabExt);
let checker = SubtypeChecker::from_preset(preset);
```

## Dependencies

- `jsonc-parser` — parsing JSON with comments (already in Cargo.toml)
- `miette` — rich error reporting (already in Cargo.toml)
- `snafu` — error type derivation (already in Cargo.toml)
- `recursion` — recursion schemes for tree traversal and rewriting
- `smallvec` — inline storage for `Provenance` spans/pointers
- A regex algebra library (intersection, complement, containment) — needed for
  string schema canonicalization and subtype checking. Candidate: `regex-syntax`
  for parsing + custom algebra, or port of Python `greenery` library.

## Out of Scope

- Recursive `$ref` (mentioned in README as future work)
- `unevaluatedProperties` / `unevaluatedItems` (future work)
- Trait-based abstraction over JSON representations (future refactor)
- Incremental/salsa-based computation (future work)
- Non-draft-2020-12 presets (future — the extension API supports them but we
  won't ship any initially)
