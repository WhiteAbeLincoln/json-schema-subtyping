pub mod check;
pub mod error;
pub mod extension;
pub mod located;
pub mod parse;
pub mod preset;
pub mod rewrite;
pub mod schema;

pub use error::{DetailedOutput, SubtypeError, SubtypeRelation};
pub use extension::VocabExtension;
pub use preset::Preset;
pub use rewrite::Phase;

use extension::ExtensionData;
use located::LocatedValue;
use schema::ReducedSchema;

/// The main entry point: parses, rewrites, extracts, and checks subtype relationships.
pub struct SubtypeChecker {
    preset: Preset,
}

impl SubtypeChecker {
    /// Create a checker with the default draft 2020-12 preset.
    pub fn new() -> Self {
        Self::from_preset(Preset::draft_2020_12())
    }

    /// Create a checker from a custom preset.
    pub fn from_preset(preset: Preset) -> Self {
        Self { preset }
    }

    /// Create a builder for customizing the checker.
    pub fn builder() -> SubtypeCheckerBuilder {
        SubtypeCheckerBuilder {
            preset: Preset::draft_2020_12(),
        }
    }

    /// Parse a JSON Schema source string.
    pub fn parse(&self, source: &str) -> Result<ParsedSchema, SubtypeError> {
        let tree = parse::parse(source)?;
        Ok(ParsedSchema { tree })
    }

    /// Reduce a parsed schema through rewrite phases and extraction.
    pub fn reduce(&self, parsed: &ParsedSchema) -> Result<ReducedSchemaSet, SubtypeError> {
        // Resolve local $refs before rewrite phases.
        let mut tree = rewrite::ref_resolution::resolve_refs(&parsed.tree, &parsed.tree)?;

        for phase in [Phase::DraftConversion, Phase::Canonicalization, Phase::Simplification] {
            let rules = self.preset.rules_for(phase);
            if !rules.is_empty() {
                tree = rewrite::rewrite_phase(&tree, rules)?;
            }
        }

        let schema = schema::extract::extract(&tree)?;

        Ok(ReducedSchemaSet {
            schema,
            extensions: ExtensionData { slots: vec![] },
        })
    }

    /// Check if `sub` is a subtype of `sup` given their reduced schemas.
    pub fn check(
        &self,
        sub: &ReducedSchemaSet,
        sup: &ReducedSchemaSet,
    ) -> Result<SubtypeRelation, SubtypeError> {
        check::is_subtype(&sub.schema, &sup.schema)
    }

    /// All-in-one: parse both schemas, reduce, and check subtype.
    pub fn is_subtype(&self, sub: &str, sup: &str) -> Result<SubtypeRelation, SubtypeError> {
        let sub_parsed = self.parse(sub)?;
        let sup_parsed = self.parse(sup)?;
        let sub_reduced = self.reduce(&sub_parsed)?;
        let sup_reduced = self.reduce(&sup_parsed)?;
        self.check(&sub_reduced, &sup_reduced)
    }
}

impl Default for SubtypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for SubtypeChecker with customization options.
pub struct SubtypeCheckerBuilder {
    preset: Preset,
}

impl SubtypeCheckerBuilder {
    /// Replace the entire preset.
    pub fn preset(mut self, preset: Preset) -> Self {
        self.preset = preset;
        self
    }

    /// Add a rewrite rule to a specific phase.
    pub fn add_rewrite(mut self, phase: Phase, rule: impl rewrite::RewriteRule + 'static) -> Self {
        self.preset.add_rewrite(phase, rule);
        self
    }

    /// Add a vocabulary extension.
    pub fn add_extension<E: VocabExtension>(mut self, ext: E) -> Self {
        self.preset.add_extension(ext);
        self
    }

    /// Build the SubtypeChecker.
    pub fn build(self) -> SubtypeChecker {
        SubtypeChecker::from_preset(self.preset)
    }
}

/// A parsed but not yet reduced schema.
pub struct ParsedSchema {
    tree: LocatedValue,
}

/// A fully reduced schema ready for subtype checking.
pub struct ReducedSchemaSet {
    pub schema: ReducedSchema,
    pub extensions: ExtensionData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_subtype_check() {
        let checker = SubtypeChecker::new();
        let result = checker
            .is_subtype(
                r#"{"type": "integer", "minimum": 0, "maximum": 10}"#,
                r#"{"type": "number"}"#,
            )
            .unwrap();
        assert!(result.is_subtype());
    }

    #[test]
    fn not_subtype_check() {
        let checker = SubtypeChecker::new();
        let result = checker
            .is_subtype(r#"{"type": "number"}"#, r#"{"type": "integer"}"#)
            .unwrap();
        // number is NOT a subtype of integer (integer = number + multipleOf:1)
        assert!(!result.is_subtype());
    }

    #[test]
    fn string_pattern_subtype() {
        let checker = SubtypeChecker::new();
        let result = checker
            .is_subtype(
                r#"{"type": "string", "pattern": "^[a-z]{3}$"}"#,
                r#"{"type": "string", "pattern": "^[a-z]+$"}"#,
            )
            .unwrap();
        assert!(result.is_subtype());
    }

    #[test]
    fn null_subtype_of_nullable() {
        let checker = SubtypeChecker::new();
        let result = checker
            .is_subtype(
                r#"{"type": "null"}"#,
                r#"{"anyOf": [{"type": "null"}, {"type": "string"}]}"#,
            )
            .unwrap();
        assert!(result.is_subtype());
    }

    #[test]
    fn boolean_true_and_false() {
        let checker = SubtypeChecker::new();
        // true (Top) is NOT subtype of string
        let result = checker
            .is_subtype("true", r#"{"type": "string"}"#)
            .unwrap();
        assert!(!result.is_subtype());
        // string IS subtype of true (Top)
        let result = checker
            .is_subtype(r#"{"type": "string"}"#, "true")
            .unwrap();
        assert!(result.is_subtype());
        // false (Bottom) is subtype of everything
        let result = checker.is_subtype("false", "true").unwrap();
        assert!(result.is_subtype());
        let result = checker
            .is_subtype("false", r#"{"type": "string"}"#)
            .unwrap();
        assert!(result.is_subtype());
    }

    #[test]
    fn empty_schema_is_top() {
        let checker = SubtypeChecker::new();
        // {} accepts everything, string doesn't
        let result = checker
            .is_subtype("{}", r#"{"type": "string"}"#)
            .unwrap();
        assert!(!result.is_subtype());
        // string is subtype of {}
        let result = checker
            .is_subtype(r#"{"type": "string"}"#, "{}")
            .unwrap();
        assert!(result.is_subtype());
    }

    #[test]
    fn array_subtype() {
        let checker = SubtypeChecker::new();
        let result = checker
            .is_subtype(
                r#"{"type": "array", "items": {"type": "string"}, "minItems": 1}"#,
                r#"{"type": "array", "items": {"type": "string"}}"#,
            )
            .unwrap();
        assert!(result.is_subtype());
    }

    #[test]
    fn object_subtype() {
        let checker = SubtypeChecker::new();
        let result = checker
            .is_subtype(
                r#"{"type": "object", "required": ["name", "age"]}"#,
                r#"{"type": "object", "required": ["name"]}"#,
            )
            .unwrap();
        assert!(result.is_subtype());
    }

    #[test]
    fn ref_resolution_in_pipeline() {
        let checker = SubtypeChecker::new();
        // Schema using $ref to define a property type
        let sub = r##"{
            "$defs": {"pos_int": {"type": "integer", "minimum": 0}},
            "type": "object",
            "properties": {"age": {"$ref": "#/$defs/pos_int"}}
        }"##;
        let sup = r#"{"type": "object"}"#;
        let result = checker.is_subtype(sub, sup).unwrap();
        assert!(result.is_subtype());
    }
}
