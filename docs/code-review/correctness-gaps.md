# Correctness Gaps: Rust vs Reference Implementation

Comparison of this crate against [IBM/jsonsubschema](https://github.com/IBM/jsonsubschema),
the Python reference implementation of *"Checking Semantic Subtyping of JSON Schemas"*
(Habib, Shinnar, Hirzel; POPL 2021).

Differences due to the language (Rust vs Python) and spec version (2020-12 vs draft-4/7)
are expected and not covered here. This document focuses on places where the Rust
implementation is **less complete** in ways that affect correctness — i.e. cases where a
true subtype relationship would be missed.

---

## 1. `allOf` intersection is not computed

**What the paper does.** The paper defines meet (intersection) operations per type. Given
`allOf: [S1, S2]` where both branches share a type, the result is a single schema with
the tighter constraint from each side (max of minimums, min of maximums, LCM of
`multipleOf`, regex intersection, union of required keys, etc.).

**What Python does.** `JSONallOfFactory` folds all branches with `meet()`, producing a
single `TypedSchema` or `Bot`. By the time subtype checking runs, no `AllOf` node exists
in the IR.

**What Rust does.** `AllOf` survives into `ReducedSchema` as a residual node. The checker
at `src/check/mod.rs:71-78` applies a conservative rule:

```
allOf(A, B, ...) <: S  iff  some branch Ai <: S
```

This is sound but incomplete. It fails whenever the subtype relationship only holds for
the *intersection* of branches, not any individual branch.

**Example that Rust gets wrong:**

```json
sub: { "allOf": [
  { "type": "number", "minimum": 0 },
  { "type": "number", "maximum": 10 }
]}
sup: { "type": "number", "minimum": 0, "maximum": 10 }
```

Neither branch alone is `<: sup`, but their intersection `[0, 10]` is.

**Fix.** Implement per-type `meet()` operations and fold `allOf` during the simplification
phase. The type-specific intersections needed:

| Type | Meet operation |
|------|---------------|
| Number | `max(min1, min2)`, `min(max1, max2)`, same for exclusive bounds, `lcm(m1, m2)` for multipleOf |
| String | Regex intersection (already available via `regex_algebra::intersect`), `max(minLen)`, `min(maxLen)` |
| Boolean | BoolSet intersection |
| Array | `max(minItems)`, `min(maxItems)`, `unique1 \|\| unique2`, pairwise `meet` on prefixItems/items |
| Object | `max(minProps)`, `min(maxProps)`, union of required, merge patternProperties (intersect schemas for overlapping patterns) |
| Null | Identity (null meet null = null) |

When branches have **different** types, the meet is `Bottom`.

This should be implemented as a simplification rule that rewrites
`AllOf([Typed(A), Typed(B)])` into `Typed(meet(A, B))`. Heterogeneous-type detection
(`IntersectHeterogeneousTypes`) already exists in `simplify.rs` and handles the
different-type case — the missing piece is same-type folding.

---

## 2. Negation of typed schemas is not eliminated

**What the paper does.** Section 4.2 defines negation rewriting for each type. For
example, `not({type: string, minLength: 3})` becomes:

```
anyOf(
  {type: number}, {type: boolean}, {type: null}, {type: array}, {type: object},
  {type: string, maxLength: 2}
)
```

This converts `Not` nodes into unions of type-complement branches plus negated
constraints within the same type.

**What Python does.** Each type class has a `neg()` static method that performs this
conversion. After canonicalization, no `Not(typed-schema)` nodes remain.

**What Rust does.** De Morgan laws (`NotNot`, `NotAnyOf`, `NotAllOf`) are applied during
simplification, but `Not(Typed(...))` nodes survive into the IR. The checker at
`src/check/mod.rs:84` treats these as opaque:

```rust
_ => Ok(not_subtype("incompatible schema shapes")),
```

**Example that Rust gets wrong:**

```json
sub: { "type": "integer", "minimum": 0, "maximum": 5 }
sup: { "not": { "type": "string" } }
```

The sup accepts everything except strings. An integer schema is clearly a subtype, but
Rust cannot determine this because the `Not` node is never decomposed.

**Fix.** Add a simplification rule `NotTyped` that rewrites `not(Typed(T))` into an
`AnyOf` of:
1. One branch per type *other than* T's type (unconstrained).
2. Branches for the *negated constraints* within T's type.

Per-type negation rules:

| Type | Negation |
|------|----------|
| Null | `anyOf(string, number, boolean, array, object)` |
| Boolean(TrueOnly) | `anyOf(non-boolean-types..., boolean(FalseOnly))` |
| String(pat, minLen, maxLen) | `anyOf(non-string-types..., string(complement(pat)), string(maxLen: minLen-1), string(minLen: maxLen+1))` |
| Number(min, max, multipleOf) | `anyOf(non-number-types..., number(max: min), number(min: max))` — note: multipleOf negation is hard (Python also marks this as TODO) |
| Array (with keywords) | Complex — Python raises `UnsupportedNegatedArray`. Can start with the simple case (no keywords → complement types only). |
| Object (with keywords) | Complex — Python raises `UnsupportedNegatedObject`. Same approach. |

Even implementing the simple cases (negate null, boolean, and unconstrained
array/object) would cover the most common patterns from `oneOf` expansion, where `not`
is applied to each excluded branch.

---

## 3. `allOf` sub with `anyOf` sup interaction

**What the paper does.** The general rule is:

```
allOf(A1, ..., An) <: anyOf(B1, ..., Bm)
```

This requires checking whether the intersection of the Ai is contained in the union of
the Bj. With meet() available, this reduces to `meet(A1, ..., An) <: anyOf(B1, ..., Bm)`.

**What Rust does.** The `allOf` sub rule at `src/check/mod.rs:71-78` only checks whether
any *individual* branch is a subtype. This interacts badly with `anyOf` on the sup side,
since even after the `anyOf` rule at line 60 gets a chance to fire, the `allOf` sub case
is reached first due to match ordering.

**Fix.** This is a direct consequence of gap #1. Once `allOf` is folded via `meet()`,
the `AllOf` sub case would rarely (or never) be reached.

---

## 4. Number inhabitedness is approximate with exclusive bounds

**What Rust does.** `src/check/inhabited.rs:44-47`:

```rust
let effective_min = n.minimum.value.max(n.exclusive_minimum.value);
let effective_max = n.maximum.value.min(n.exclusive_maximum.value);
Ok(effective_min > effective_max)
```

This misses the case where `effective_min == effective_max` and at least one bound is
exclusive. For example: `{ minimum: 5, exclusiveMaximum: 5 }` has `effective_min = 5`,
`effective_max = 5`, but the range is empty because the maximum is exclusive.

**Fix.** Track inclusivity like the subtype checker already does in
`src/check/number.rs:7-26`, and check:

```rust
effective_min > effective_max
    || (effective_min == effective_max && (!min_inclusive || !max_inclusive))
```

---

## 5. Number subtype checking with exclusive bounds edge case

The `effective_lower` / `effective_upper` functions at `src/check/number.rs:7-26` handle
the case where exclusive and inclusive bounds are equal:

```rust
// Equal: exclusive is tighter
(excl_max, false)
```

This is correct. However the interaction with `multipleOf` is not checked: if
`sub` has `multipleOf: 1` (integer) and a non-integer exclusive bound, the effective
range could be tighter than what bound comparison alone determines. For example:

```json
sub: { "type": "integer", "exclusiveMinimum": 0.5, "maximum": 3 }
sup: { "type": "integer", "minimum": 1, "maximum": 3 }
```

Both accept `{1, 2, 3}`, so sub <: sup. But the bound check sees
`sub_lower = (0.5, exclusive)` and `sup_lower = (1, inclusive)`. The
`lower_contained` function returns true here (0.5 < 1 fails, so it falls through),
which is actually incorrect — `lower_contained` requires `sub.0 > sup.0` or equality,
but `0.5 < 1`. So this would return "not subtype" even though the effective integer
ranges are identical.

This is the same class of problem Python solves with its `IntegerBoundConversion`
canonicalization (converting exclusive bounds to inclusive for integer schemas). The Rust
canonicalization rule `IntegerBoundConversion` exists but only fires for literal
`multipleOf: 1` schemas — schemas that *acquire* `multipleOf: 1` through `allOf`
intersection would not benefit.

**Fix.** Either:
- Ensure `IntegerBoundConversion` runs after `allOf` folding, or
- Make the number subtype checker aware of `multipleOf` when comparing bounds (snap
  exclusive bounds to the nearest valid multiple).

---

## 6. Object subtype checking: uncovered sub patterns

**What Rust does.** `src/check/object.rs:34-61` iterates over sup's patterns and checks
that overlapping sub patterns have subtype schemas. If a sup pattern has no overlapping
sub pattern and the sup schema is not Top, the check fails.

**What's missing.** The reverse direction is not checked: if *sub* has a pattern that
doesn't overlap with any *sup* pattern, then sub constrains some property names that sup
leaves unconstrained (i.e. sup treats them as Top). This direction is fine — sub being
*more* constrained than sup's Top is always valid. So this is correct.

However, consider the case where sup has `maxProperties: 0` (no properties allowed).
If sub has pattern properties, sub still allows values with those properties (constrained
but present), which violates sup. The current check at `object.rs:10-15` catches this
via `max_properties` comparison, but only if sub's `minProperties` or `maxProperties`
reflects the actual minimum number of properties that could exist. If sub has required
properties, the effective minimum is `max(minProperties, len(required))`, but the check
only compares `minProperties` directly.

**Example:**

```json
sub: { "type": "object", "required": ["a", "b", "c"], "minProperties": 0 }
sup: { "type": "object", "maxProperties": 2 }
```

Sub requires 3 properties, so any valid sub value has at least 3. Sup allows at most 2.
This should be "not subtype", but the range check sees `minProperties(0) >= 0` and
`maxProperties(MAX) <= 2`... actually `sub.max_properties` would be MAX which is > 2, so
this particular example *is* caught. But inhabitedness could still play a role: sub is
*not* uninhabited (it has valid values with 3+ properties), so the uninhabited early-exit
doesn't fire.

The real gap: sub's effective minimum property count should be
`max(minProperties, len(required))` when comparing against sup's bounds.

**Fix.** In `check_object_subtype`, compute:

```rust
let sub_effective_min = sub.min_properties.value.max(sub.required.len() as u64);
```

and use that for the range containment check.

---

## 7. Priority and estimated impact

| Gap | Impact | Effort | Frequency |
|-----|--------|--------|-----------|
| 1. allOf meet | **High** — any `oneOf` with same-type branches produces `allOf` after expansion; none of these can be checked correctly today | Medium | Very common (oneOf is widely used) |
| 2. Not elimination | **High** — every `oneOf` branch generates `not(other_branches)`; if those branches are typed, the `not` is never resolved | Medium | Very common (same reason) |
| 3. allOf × anyOf | **Medium** — consequence of #1 | Free (fixed by #1) | Common |
| 4. Number inhabitedness | **Low** — edge case with exclusive bounds at exact boundary | Trivial | Rare |
| 5. Integer exclusive bounds | **Low** — only affects integers with non-integer exclusive bounds, which is unusual in practice | Small | Rare |
| 6. Object effective min | **Low** — only matters when required.len() > minProperties, which is uncommon | Trivial | Uncommon |

---

## Recommended implementation order

1. **allOf meet** (#1) — unblocks the largest class of currently-broken cases. Start
   with Number and String (most common types in `oneOf` patterns), then Boolean, then
   Array/Object.

2. **Not elimination** (#2) — completes the `oneOf` story. Start with simple types
   (null, boolean, string, number) and leave array/object negation as unsupported (same
   as Python).

3. **Number inhabitedness** (#4) and **object effective min** (#6) — trivial fixes,
   can be done anytime.

4. **Integer exclusive bounds** (#5) — small fix, can be folded into the `allOf meet`
   work if `IntegerBoundConversion` is re-run after folding.
