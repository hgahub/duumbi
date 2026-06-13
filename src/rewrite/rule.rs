//! Rewrite rule metadata and safety classification.

use serde::{Deserialize, Serialize};

/// Stable category for a rewrite rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleCategory {
    /// Canonicalizes equivalent graph shapes into a preferred form.
    Canonicalization,
    /// Performs a local behavior-preserving simplification.
    LocalOptimization,
    /// Adjusts non-executable graph structure or metadata.
    StructuralCleanup,
}

/// Declared safety level for a rewrite rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SafetyClass {
    /// Changes graph structure without changing executable behavior.
    StructuralOnly,
    /// Preserves executable behavior under declared local preconditions.
    LocalSemanticsPreserving,
    /// Preview-only in V1 unless a later spec changes the policy.
    Experimental,
}

impl SafetyClass {
    /// Returns whether this safety class may be applied in V1.
    #[must_use]
    pub fn is_apply_capable(self) -> bool {
        !matches!(self, Self::Experimental)
    }
}

/// Public metadata for one rewrite rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSummary {
    /// Stable rule identifier, such as `i64-add-zero-right`.
    pub id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// High-level rule category.
    pub category: RuleCategory,
    /// Declared safety class.
    pub safety_class: SafetyClass,
    /// Concise description of what the rule detects.
    pub description: String,
    /// Human-readable precondition summary.
    pub preconditions: String,
    /// Human-readable effect summary.
    pub effect_summary: String,
    /// Default bounded cost estimate for one match.
    pub default_cost: u32,
    /// Human-readable explanation template.
    pub explanation_template: String,
    /// Whether this rule can be applied in V1.
    pub apply_capable: bool,
}

impl RuleSummary {
    /// Creates a rule summary and derives apply capability from safety class.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        category: RuleCategory,
        safety_class: SafetyClass,
        description: impl Into<String>,
        preconditions: impl Into<String>,
        effect_summary: impl Into<String>,
        default_cost: u32,
        explanation_template: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            category,
            safety_class,
            description: description.into(),
            preconditions: preconditions.into(),
            effect_summary: effect_summary.into(),
            default_cost,
            explanation_template: explanation_template.into(),
            apply_capable: safety_class.is_apply_capable(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_class_serializes_to_kebab_case() {
        let value = serde_json::to_value(SafetyClass::LocalSemanticsPreserving)
            .expect("invariant: safety class serializes");
        assert_eq!(value, serde_json::json!("local-semantics-preserving"));
    }

    #[test]
    fn experimental_rules_are_not_apply_capable() {
        assert!(!SafetyClass::Experimental.is_apply_capable());
        assert!(SafetyClass::LocalSemanticsPreserving.is_apply_capable());
        assert!(SafetyClass::StructuralOnly.is_apply_capable());
    }

    #[test]
    fn rule_summary_derives_apply_capable_from_safety_class() {
        let summary = RuleSummary::new(
            "experimental-fold-i64-const-add",
            "Experimental fold i64 const add",
            RuleCategory::Canonicalization,
            SafetyClass::Experimental,
            "Preview constant folding for i64 Add ops",
            "Both operands are i64 constants",
            "Would replace Add with one i64 Const",
            1,
            "Fold constant Add when both operands are known",
        );

        assert!(!summary.apply_capable);
        let value = serde_json::to_value(&summary).expect("invariant: summary serializes");
        assert_eq!(value["id"], "experimental-fold-i64-const-add");
        assert_eq!(value["safetyClass"], "experimental");
    }
}
