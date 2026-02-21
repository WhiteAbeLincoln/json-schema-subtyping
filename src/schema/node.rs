// TODO: Keyed collections (properties, pattern_properties, dependent_schemas,
// dependent_required) use Vec<(S, A)> instead of BTreeMap<S, A>. This is because
// borrow() needs to convert SchemaObject<String, A> to SchemaObject<&str, &A>,
// and you can't borrow a BTreeMap<String, A> as BTreeMap<&str, &A> without
// rebuilding the tree — a Vec of tuples is cheaper to reconstruct. The tradeoff
// is O(n) key lookup instead of O(log n). For the subtyping algorithm, consider
// building a temporary HashMap at the comparison site, or adding a
// property(name) -> Option<&Self> method to JsonSchema for direct lookup.

use super::json_value::JsonValue;
use super::typeset::TypeSet;

/// The base functor for JSON Schema.
///
/// Represents one layer of a JSON Schema tree, parameterized over:
/// - `S`: the string type (`&str` for borrowed views, `String` for owned storage)
/// - `A`: the recursive child type (sub-schemas)
#[derive(Clone, Debug)]
pub enum SchemaF<S, A> {
    /// `true` -- the top type, accepts everything.
    True,
    /// `false` -- the bottom type, rejects everything.
    False,
    /// A schema object with keyword constraints.
    Schema(Box<SchemaObject<S, A>>),
}

/// All recognized JSON Schema (draft 2020-12) keywords.
///
/// Fields use sentinel values where "absent" equals "default" per the meta-schema,
/// and `Option` where absence is semantically distinct from any valid value.
#[derive(Clone, Debug)]
pub struct SchemaObject<S, A> {
    // ---- validation: type ----
    /// The `type` keyword. `TypeSet::all()` means absent (unconstrained).
    pub r#type: TypeSet,

    // ---- validation: numeric ----
    /// Must be > 0 when present.
    pub multiple_of: Option<f64>,
    pub maximum: Option<f64>,
    pub exclusive_maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub exclusive_minimum: Option<f64>,

    // ---- validation: string ----
    /// 0 = absent (meta-schema default is 0).
    pub min_length: u64,
    /// 0 is a valid constraint (only empty string matches).
    pub max_length: Option<u64>,
    pub pattern: Option<S>,

    // ---- validation: array ----
    /// 0 = absent (meta-schema default is 0).
    pub min_items: u64,
    /// 0 is a valid constraint (only empty array matches).
    pub max_items: Option<u64>,
    /// `false` = absent (meta-schema default is false).
    pub unique_items: bool,
    /// Default is 1 when absent, and 0 is a valid distinct setting.
    pub min_contains: Option<u64>,
    /// 0 is a valid constraint.
    pub max_contains: Option<u64>,

    // ---- validation: object ----
    /// 0 = absent (meta-schema default is 0).
    pub min_properties: u64,
    /// 0 is a valid constraint (only empty object matches).
    pub max_properties: Option<u64>,
    /// Empty = absent (meta-schema default is `[]`).
    pub required: Vec<S>,

    // ---- validation: any ----
    pub r#const: Option<JsonValue>,
    /// `None` = absent (unconstrained). `Some(vec![])` = nothing matches (bottom).
    pub r#enum: Option<Vec<JsonValue>>,

    // ---- applicator: array ----
    /// Empty = absent (schemaArray requires minItems: 1).
    pub prefix_items: Vec<A>,
    pub items: Option<A>,
    pub contains: Option<A>,

    // ---- applicator: object ----
    /// Empty = absent (meta-schema default is `{}`).
    pub properties: Vec<(S, A)>,
    /// Empty = absent (meta-schema default is `{}`).
    pub pattern_properties: Vec<(S, A)>,
    pub additional_properties: Option<A>,
    pub property_names: Option<A>,

    // ---- applicator: conditional ----
    pub r#if: Option<A>,
    pub then: Option<A>,
    pub r#else: Option<A>,

    // ---- applicator: composition ----
    /// Empty = absent (schemaArray requires minItems: 1).
    pub all_of: Vec<A>,
    /// Empty = absent (schemaArray requires minItems: 1).
    pub any_of: Vec<A>,
    /// Empty = absent (schemaArray requires minItems: 1).
    pub one_of: Vec<A>,
    pub not: Option<A>,

    // ---- applicator: unevaluated ----
    pub unevaluated_items: Option<A>,
    pub unevaluated_properties: Option<A>,

    // ---- applicator: object dependencies ----
    /// Empty = absent.
    pub dependent_schemas: Vec<(S, A)>,
    /// Empty = absent.
    pub dependent_required: Vec<(S, Vec<S>)>,
}

// ---------------------------------------------------------------------------
// Default
// ---------------------------------------------------------------------------

impl<S, A> Default for SchemaObject<S, A> {
    fn default() -> Self {
        Self {
            r#type: TypeSet::all(),
            multiple_of: None,
            maximum: None,
            exclusive_maximum: None,
            minimum: None,
            exclusive_minimum: None,
            min_length: 0,
            max_length: None,
            pattern: None,
            min_items: 0,
            max_items: None,
            unique_items: false,
            min_contains: None,
            max_contains: None,
            min_properties: 0,
            max_properties: None,
            required: Vec::new(),
            r#const: None,
            r#enum: None,
            prefix_items: Vec::new(),
            items: None,
            contains: None,
            properties: Vec::new(),
            pattern_properties: Vec::new(),
            additional_properties: None,
            property_names: None,
            r#if: None,
            then: None,
            r#else: None,
            all_of: Vec::new(),
            any_of: Vec::new(),
            one_of: Vec::new(),
            not: None,
            unevaluated_items: None,
            unevaluated_properties: None,
            dependent_schemas: Vec::new(),
            dependent_required: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Functor: map over recursive children
// ---------------------------------------------------------------------------

impl<S, A> SchemaF<S, A> {
    /// Map a function over all recursive children, preserving string data.
    pub fn map<B>(self, f: impl FnMut(A) -> B) -> SchemaF<S, B> {
        match self {
            SchemaF::True => SchemaF::True,
            SchemaF::False => SchemaF::False,
            SchemaF::Schema(obj) => SchemaF::Schema(Box::new(obj.map(f))),
        }
    }
}

impl<S, A> SchemaObject<S, A> {
    /// Map a function over all recursive children, preserving string and scalar data.
    pub fn map<B>(self, mut f: impl FnMut(A) -> B) -> SchemaObject<S, B> {
        SchemaObject {
            // Non-recursive fields pass through unchanged.
            r#type: self.r#type,
            multiple_of: self.multiple_of,
            maximum: self.maximum,
            exclusive_maximum: self.exclusive_maximum,
            minimum: self.minimum,
            exclusive_minimum: self.exclusive_minimum,
            min_length: self.min_length,
            max_length: self.max_length,
            pattern: self.pattern,
            min_items: self.min_items,
            max_items: self.max_items,
            unique_items: self.unique_items,
            min_contains: self.min_contains,
            max_contains: self.max_contains,
            min_properties: self.min_properties,
            max_properties: self.max_properties,
            required: self.required,
            r#const: self.r#const,
            r#enum: self.r#enum,
            dependent_required: self.dependent_required,

            // Recursive fields: apply f.
            prefix_items: self.prefix_items.into_iter().map(&mut f).collect(),
            items: self.items.map(&mut f),
            contains: self.contains.map(&mut f),
            properties: self
                .properties
                .into_iter()
                .map(|(k, v)| (k, f(v)))
                .collect(),
            pattern_properties: self
                .pattern_properties
                .into_iter()
                .map(|(k, v)| (k, f(v)))
                .collect(),
            additional_properties: self.additional_properties.map(&mut f),
            property_names: self.property_names.map(&mut f),
            r#if: self.r#if.map(&mut f),
            then: self.then.map(&mut f),
            r#else: self.r#else.map(&mut f),
            all_of: self.all_of.into_iter().map(&mut f).collect(),
            any_of: self.any_of.into_iter().map(&mut f).collect(),
            one_of: self.one_of.into_iter().map(&mut f).collect(),
            not: self.not.map(&mut f),
            unevaluated_items: self.unevaluated_items.map(&mut f),
            unevaluated_properties: self.unevaluated_properties.map(&mut f),
            dependent_schemas: self
                .dependent_schemas
                .into_iter()
                .map(|(k, v)| (k, f(v)))
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Borrow: convert owned storage to a borrowed view
// ---------------------------------------------------------------------------

impl<S: AsRef<str>, A> SchemaF<S, A> {
    /// Borrow one layer: convert owned strings to `&str` and children to `&A`.
    ///
    /// Used by [`Schema`](super::Schema) and [`Annotated`](super::Annotated)
    /// to implement [`JsonSchema::view`](super::JsonSchema::view).
    pub fn borrow(&self) -> SchemaF<&str, &A> {
        match self {
            SchemaF::True => SchemaF::True,
            SchemaF::False => SchemaF::False,
            SchemaF::Schema(obj) => SchemaF::Schema(Box::new(obj.borrow())),
        }
    }
}

impl<S: AsRef<str>, A> SchemaObject<S, A> {
    /// Borrow one layer: convert owned strings to `&str` and children to `&A`.
    pub fn borrow(&self) -> SchemaObject<&str, &A> {
        SchemaObject {
            r#type: self.r#type,
            multiple_of: self.multiple_of,
            maximum: self.maximum,
            exclusive_maximum: self.exclusive_maximum,
            minimum: self.minimum,
            exclusive_minimum: self.exclusive_minimum,
            min_length: self.min_length,
            max_length: self.max_length,
            pattern: self.pattern.as_ref().map(|s| s.as_ref()),
            min_items: self.min_items,
            max_items: self.max_items,
            unique_items: self.unique_items,
            min_contains: self.min_contains,
            max_contains: self.max_contains,
            min_properties: self.min_properties,
            max_properties: self.max_properties,
            required: self.required.iter().map(|s| s.as_ref()).collect(),
            r#const: self.r#const.clone(),
            r#enum: self.r#enum.clone(),

            prefix_items: self.prefix_items.iter().collect(),
            items: self.items.as_ref(),
            contains: self.contains.as_ref(),
            properties: self
                .properties
                .iter()
                .map(|(k, v)| (k.as_ref(), v))
                .collect(),
            pattern_properties: self
                .pattern_properties
                .iter()
                .map(|(k, v)| (k.as_ref(), v))
                .collect(),
            additional_properties: self.additional_properties.as_ref(),
            property_names: self.property_names.as_ref(),
            r#if: self.r#if.as_ref(),
            then: self.then.as_ref(),
            r#else: self.r#else.as_ref(),
            all_of: self.all_of.iter().collect(),
            any_of: self.any_of.iter().collect(),
            one_of: self.one_of.iter().collect(),
            not: self.not.as_ref(),
            unevaluated_items: self.unevaluated_items.as_ref(),
            unevaluated_properties: self.unevaluated_properties.as_ref(),
            dependent_schemas: self
                .dependent_schemas
                .iter()
                .map(|(k, v)| (k.as_ref(), v))
                .collect(),
            dependent_required: self
                .dependent_required
                .iter()
                .map(|(k, vs)| (k.as_ref(), vs.iter().map(|s| s.as_ref()).collect()))
                .collect(),
        }
    }
}
