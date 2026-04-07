use std::collections::HashMap;

use crate::extension::{ExtensionSlot, VocabExtension};
use crate::rewrite::{Phase, RewriteRule};

/// A preset bundles rewrite rules and extensions for a particular JSON Schema dialect.
#[derive(Default)]
pub struct Preset {
    pub(crate) rewrites: HashMap<Phase, Vec<Box<dyn RewriteRule>>>,
    pub(crate) extensions: Vec<ExtensionSlot>,
}


impl Preset {
    /// Create a preset for JSON Schema draft 2020-12.
    pub fn draft_2020_12() -> Self {
        use crate::rewrite::canonicalize::{non_type_specific_rules, type_specific_rules};
        use crate::rewrite::simplify::simplification_rules;

        let mut p = Self::default();
        // Non-type-specific rules run in DraftConversion (first pass).
        // They create structure (e.g. anyOf branches from MultipleTypes)
        // that type-specific rules need to process in a second bottom-up pass.
        p.rewrites
            .insert(Phase::DraftConversion, non_type_specific_rules());
        p.rewrites
            .insert(Phase::Canonicalization, type_specific_rules());
        p.rewrites
            .insert(Phase::Simplification, simplification_rules());
        p
    }

    /// Add a rewrite rule to a specific phase.
    pub fn add_rewrite(&mut self, phase: Phase, rule: impl RewriteRule + 'static) -> &mut Self {
        self.rewrites.entry(phase).or_default().push(Box::new(rule));
        self
    }

    /// Add a vocabulary extension.
    pub fn add_extension<E: VocabExtension>(&mut self, ext: E) -> &mut Self {
        self.extensions.push(ExtensionSlot::new(ext));
        self
    }

    /// Get the rules for a given phase.
    pub fn rules_for(&self, phase: Phase) -> &[Box<dyn RewriteRule>] {
        self.rewrites
            .get(&phase)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
