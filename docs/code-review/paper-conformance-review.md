# Paper Conformance Review: Implementation vs. Habib et al. 2020

**Date:** 2026-04-06
**Reference:** "Type Safety with JSON Subschema" (Habib, Shinnar, Hirzel, Pradel, 2020)
**Scope:** Core rewrite rules (canonicalization, simplification) and subtype inference rules

---

## Evaluation: Implementation vs. Paper (Habib et al. 2020)

### Correctly Implemented Core Rules

The overall architecture faithfully follows the paper's three-phase pipeline (canonicalize → simplify → check). The following rules are correctly implemented:

**Canonicalization (Figures 4-5):**
- `multiple types` → `MultipleTypes` (also handles integer→number inline)
- `multiple connectives` → `MultipleConnectives`  
- `missing type` → `MissingType`
- `integer` → `IntegerToNumber` + `IntegerBoundConversion`
- `heterogeneous enum` → `HeterogeneousEnum`
- `oneOf` → `OneOfToAnyOf`
- `additionalProperties false` → `AdditionalPropertiesFalse`
- `object with properties` → `ObjectWithProperties` (partial — see issue below)
- `dependentRequired`/`dependentSchemas` → correctly adapted for 2020-12
- `irrelevant keywords` → `IrrelevantKeywords`
- `missing keyword` → `MissingKeyword`
- `string` length→pattern (when no existing pattern)
- Extra rules for 2020-12: `ConstToEnum`, `IfThenElse`, `AllTypesIsTop`

**Simplification (Figures 6-9):**
- `not not` (double negation) ✓
- `not anyOf` / `not allOf` (De Morgan) ✓
- `singleton`/`empty`/`flatten` for allOf and anyOf ✓
- `intersect heterogeneous types → bottom` ✓
- `allOf with bottom → bottom` ✓
- `anyOf with top → top` ✓
- `null enum` and `number enum` elimination ✓
- `multi-valued enum` splitting ✓

**Subtype Checking (Figure 10):**
- `subschema uninhabited` (conservative) ✓
- `subschema null` ✓
- `subschema boolean` (BoolSet subset) ✓
- `subschema string` (regex `is_subset` via DFA) ✓
- `subschema number` (range containment + multipleOf divisibility) ✓
- `subschema array` (bounds, per-item, tail, uniqueItems) ✓
- `subschema object` (bounds, required, pattern overlap) ✓
- anyOf/allOf structural dispatch ✓

**Regex Algebra:**
- Exact DFA-based operations: compile, intersect, union, complement, is_subset, is_empty — all correct ✓

---

### Issues Found

#### 1. `additionalProperties` Lost During Extraction (Correctness Bug)

The paper's `object with properties` rule (Figure 5) creates a **complement catch-all pattern** in `patternProperties` for `additionalProperties`:

```
patternProperties += { neg('^(k1|...|kn)$|p1|...|pm'): s.additionalProperties }
```

The implementation (`ObjectWithProperties`, line 746) converts `properties` to `patternProperties` entries but **does not** convert `additionalProperties` into a complement catch-all. The comment at line 782 says: "For now, keep additionalProperties if present."

Meanwhile, `extract_object_schema` (line 217) **ignores** `additionalProperties` entirely — it only reads `patternProperties`. This means constraints like `additionalProperties: false` are **silently dropped** after properties→patternProperties conversion.

**Impact:** A schema `{"properties": {"name": {"type":"string"}}, "additionalProperties": false}` would incorrectly appear to accept any additional properties, producing wrong subtype results.

**Minimal reproduction:**

The following should report `sub <: sup` (a closed object is a subtype of an open one), but the reverse should NOT hold. With `additionalProperties` lost, the checker cannot distinguish the two and may report both directions as subtypes.

```json
// sub: closed object — only "name" allowed
{
  "type": "object",
  "properties": { "name": { "type": "string" } },
  "additionalProperties": false
}

// sup: open object — "name" required, anything else allowed
{
  "type": "object",
  "properties": { "name": { "type": "string" } }
}
```

Expected: `sub <: sup` = true, `sup <: sub` = false.
Actual (predicted): both report true, because the `additionalProperties: false` constraint is dropped.

---

#### 2. No Type-Specific Intersection Rules (Figure 8)

The paper's Figure 8 defines intersection rules for each JSON type:
- `intersect string` → pattern intersection
- `intersect number` → range intersection + lcm(multipleOf)
- `intersect array` → item-wise intersection, min/max, uniqueItems
- `intersect object` → merge patternProperties, union required
- `intersect anyOf` → distributivity

**None of these are implemented.** `allOf` of same-type schemas stays as `AllOf(...)` in the IR, and the subtype checker uses a conservative fallback at `check/mod.rs:71`: "check if any individual branch is <: sup." This is sound but very incomplete — it cannot reason about the combined constraint of an intersection.

**Impact:** Schemas involving `allOf` of same-type constraints (common in real-world schemas, and also produced by `oneOf` elimination) will frequently return false negatives.

**Minimal reproduction:**

```json
// sub: allOf of two string constraints (should simplify to a single string)
{
  "allOf": [
    { "type": "string", "pattern": "^[a-z]+$" },
    { "type": "string", "maxLength": 5 }
  ]
}

// sup: a string schema that covers the intersection
{
  "type": "string",
  "pattern": "^[a-z]{1,5}$"
}
```

Expected: `sub <: sup` = true (the intersection of `[a-z]+` with maxLength 5 is exactly `[a-z]{1,5}`).
Actual (predicted): false, because the `allOf` is not reduced — the checker tries each branch independently and neither alone is `<: sup`.

---

#### 3. No Type-Level Complement Rules (Figure 7)

The paper defines complement rules for null, boolean, and string:
- `complement null` → bottom
- `complement boolean` → BoolSet complement
- `complement string` → regex complement
- `not type` → anyOf[complement-within-type, other-types]

None implemented. Negation stays as `Not(...)` and the subtype checker conservatively returns not-subtype for all `Not` nodes (line 84).

**Impact:** Any schema using `oneOf` (which canonicalizes to `anyOf` of `allOf` with `not`) will typically fail, since the `not` branches can't be reduced. The paper specifically states that null, boolean, and string **are** closed under complement and should be eliminated.

**Minimal reproduction:**

```json
// sub: exactly one of two string patterns (uses oneOf)
{
  "oneOf": [
    { "type": "string", "pattern": "^[a-z]+$" },
    { "type": "string", "pattern": "^[0-9]+$" }
  ]
}

// sup: any string
{
  "type": "string"
}
```

Expected: `sub <: sup` = true (both branches of oneOf are strings, so any value matching oneOf is a string).
Actual (predicted): false. `oneOf` canonicalizes to:
```json
{"anyOf": [
  {"allOf": [{"type":"string","pattern":"^[a-z]+$"}, {"not": {"type":"string","pattern":"^[0-9]+$"}}]},
  {"allOf": [{"not": {"type":"string","pattern":"^[a-z]+$"}}, {"type":"string","pattern":"^[0-9]+$"}]}
]}
```
The `not` nodes cannot be simplified, the `allOf` branches cannot be intersected, and the checker conservatively rejects.

---

#### 4. Missing Enum Elimination Rules for String/Array/Object (Figure 6)

Only `null enum` and `number enum` are implemented. Missing:
- `string enum` → exact-match pattern `^v$`
- `array enum` → push down to per-item enums
- `object enum` → push down to per-property enums

The comment at `simplify.rs:772` confirms this is deferred.

**Minimal reproduction:**

```json
// sub: a specific string value via enum
{
  "type": "string",
  "enum": ["hello"]
}

// sup: a pattern that matches "hello"
{
  "type": "string",
  "pattern": "^[a-z]+$"
}
```

Expected: `sub <: sup` = true (`"hello"` matches `^[a-z]+$`).
Actual (predicted): error or false, because the string enum `["hello"]` is not converted to the pattern `^hello$`, so the checker cannot compare it against the sup pattern.

---

#### 5. No Union Simplification Rules (Figure 9)

The paper defines union rules for same-type `anyOf`:
- `union null` → null
- `union boolean` → BoolSet union
- `union string` → regex union
- `union number` → disjoint range splitting with gcd(multipleOf)

None implemented. `anyOf` branches of the same type remain as separate `AnyOf` entries.

**Impact:** Less severe than the intersection gap since the subtype checker can still match individual branches, but it prevents consolidation that would enable more successful checks.

**Minimal reproduction:**

```json
// sub: union of two string patterns
{
  "anyOf": [
    { "type": "string", "pattern": "^[a-z]+$" },
    { "type": "string", "pattern": "^[0-9]+$" }
  ]
}

// sup: a pattern that covers both
{
  "type": "string",
  "pattern": "^[a-z0-9]+$"
}
```

Expected: `sub <: sup` = true (both `[a-z]+` and `[0-9]+` are subsets of `[a-z0-9]+`).
Actual (predicted): true in this case (the checker decomposes `anyOf` sub and checks each branch individually, both of which succeed). **However**, the reverse direction fails:

```json
// sub: the combined pattern
{
  "type": "string",
  "pattern": "^[a-z0-9]+$"
}

// sup: union of two string patterns
{
  "anyOf": [
    { "type": "string", "pattern": "^[a-z]+$" },
    { "type": "string", "pattern": "^[0-9]+$" }
  ]
}
```

Expected: `sub <: sup` = false (`"a1"` matches sub but neither sup branch).
Actual: false (correct in this case). But if the sup union were consolidated via regex union into `^([a-z]+|[0-9]+)$`, the checker could reason about it as a single schema, enabling correct results in cases where the unconsolidated form causes false negatives due to the "sub must match at least one branch" rule.

---

#### 6. Array `uniqueItems` Missing `allDisjointItems` Check

The paper's array subtype rule says:
```
s2.uniqueItems ⟹ (s1.uniqueItems OR allDisjointItems(s1))
```

The implementation (`check/array.rs:18`) only checks `s1.uniqueItems`, missing the `allDisjointItems` alternative. This means a schema that implicitly guarantees uniqueness (via pairwise-disjoint item schemas) won't be recognized as a subtype of a schema requiring uniqueItems.

**Minimal reproduction:**

```json
// sub: tuple with disjoint types — uniqueness is guaranteed structurally
{
  "type": "array",
  "prefixItems": [
    { "type": "string" },
    { "type": "number" }
  ],
  "items": false,
  "minItems": 2,
  "maxItems": 2
}

// sup: requires unique items
{
  "type": "array",
  "prefixItems": [
    { "type": "string" },
    { "type": "number" }
  ],
  "items": false,
  "minItems": 2,
  "maxItems": 2,
  "uniqueItems": true
}
```

Expected: `sub <: sup` = true (a string and a number can never be equal, so uniqueness is structurally guaranteed by `allDisjointItems`).
Actual (predicted): false, because sub does not explicitly set `uniqueItems: true`.

---

#### 7. Number Inhabited Check Edge Case

`inhabited.rs:44-48` computes:
```rust
let effective_min = n.minimum.value.max(n.exclusive_minimum.value);
let effective_max = n.maximum.value.min(n.exclusive_maximum.value);
Ok(effective_min > effective_max)
```

This doesn't track inclusivity. A schema with `exclusiveMinimum: 5, maximum: 5` (i.e., x > 5 AND x ≤ 5) is uninhabited but reports as inhabited since 5 == 5 is not > 5. This is acceptable given the conservative design, but the `effective_lower`/`effective_upper` logic in `number.rs` (which does track inclusivity) could be reused here for better precision.

**Minimal reproduction:**

```json
// sub: contradictory number range (uninhabited)
{
  "type": "number",
  "exclusiveMinimum": 5,
  "maximum": 5
}

// sup: any type (e.g., string)
{
  "type": "string"
}
```

Expected: `sub <: sup` = true (sub is uninhabited — no number is both > 5 and ≤ 5 — so it is vacuously a subtype of anything).
Actual (predicted): false, because the uninhabited check doesn't detect this case, and then the checker sees different types (number vs string) and returns not-subtype.

---

### Summary

| Category | Paper Rules | Implemented | Missing |
|----------|-----------|-------------|---------|
| Non-type-specific canonicalization | 3 | 3 + extras | — |
| Type-specific canonicalization | 13 | 10 | catch-all additionalProperties, overlapping patterns (deferred) |
| Enum elimination | 7 | 3 | string/array/object enum |
| Negation elimination | 7 | 3 | complement null/bool/string, not-type |
| allOf intersection | 9 | 4 (structural only) | all type-specific intersections, distributivity |
| anyOf union | 7 | 5 (structural only) | null/bool/string/number union |
| Subtype inference | 8 | 8 | allDisjointItems, subNumber negation |

**Bottom line:** The structural scaffolding is solid and the primitive-type checks (null, boolean, string, number) work correctly for direct comparisons. The regex algebra layer is well-implemented. The main gaps are in the **algebraic simplification** layer — type-specific intersection, complement, and union rules — which means schemas involving `allOf`, `oneOf`, or `not` across same-type branches will often produce false negatives. The `additionalProperties` extraction bug is the most critical correctness issue since it silently drops constraints.
