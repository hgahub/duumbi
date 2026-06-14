//! Built-in rewrite rule catalog.

use super::rule::{RuleCategory, RuleSummary, SafetyClass};

/// Internal matcher kind for built-in V1 rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInRuleKind {
    /// Match `Add(x, 0)` where all values are i64.
    I64AddZeroRight,
    /// Match `Add(0, x)` where all values are i64.
    I64AddZeroLeft,
    /// Match `Mul(x, 1)` where all values are i64.
    I64MulOneRight,
    /// Preview-only constant folding for `Add(Const(a), Const(b))`.
    ExperimentalFoldI64ConstAdd,
}

/// One registered rewrite rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleDefinition {
    /// Public rule metadata.
    pub summary: RuleSummary,
    /// Built-in matcher implementation.
    pub kind: BuiltInRuleKind,
}

impl RuleDefinition {
    fn new(summary: RuleSummary, kind: BuiltInRuleKind) -> Self {
        Self { summary, kind }
    }
}

/// Deterministic catalog of available rewrite rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteCatalog {
    rules: Vec<RuleDefinition>,
}

impl RewriteCatalog {
    /// Returns the V1 built-in rule catalog.
    #[must_use]
    pub fn built_in() -> Self {
        Self {
            rules: vec![
                RuleDefinition::new(
                    RuleSummary::new(
                        "i64-add-zero-right",
                        "I64 add zero right",
                        RuleCategory::LocalOptimization,
                        SafetyClass::LocalSemanticsPreserving,
                        "Detects i64 Add nodes whose right operand is zero.",
                        "The operation and both operands are i64, and the right operand is Const(0).",
                        "Replace the Add result with the left operand.",
                        1,
                        "Adding zero on the right preserves the left operand value.",
                    ),
                    BuiltInRuleKind::I64AddZeroRight,
                ),
                RuleDefinition::new(
                    RuleSummary::new(
                        "i64-add-zero-left",
                        "I64 add zero left",
                        RuleCategory::LocalOptimization,
                        SafetyClass::LocalSemanticsPreserving,
                        "Detects i64 Add nodes whose left operand is zero.",
                        "The operation and both operands are i64, and the left operand is Const(0).",
                        "Replace the Add result with the right operand.",
                        1,
                        "Adding zero on the left preserves the right operand value.",
                    ),
                    BuiltInRuleKind::I64AddZeroLeft,
                ),
                RuleDefinition::new(
                    RuleSummary::new(
                        "i64-mul-one-right",
                        "I64 multiply one right",
                        RuleCategory::LocalOptimization,
                        SafetyClass::LocalSemanticsPreserving,
                        "Detects i64 Mul nodes whose right operand is one.",
                        "The operation and both operands are i64, and the right operand is Const(1).",
                        "Replace the Mul result with the left operand.",
                        1,
                        "Multiplying by one on the right preserves the left operand value.",
                    ),
                    BuiltInRuleKind::I64MulOneRight,
                ),
                RuleDefinition::new(
                    RuleSummary::new(
                        "experimental-fold-i64-const-add",
                        "Experimental fold i64 const add",
                        RuleCategory::Canonicalization,
                        SafetyClass::Experimental,
                        "Detects i64 Add nodes with two constant operands.",
                        "Both operands are i64 constants.",
                        "Would replace the Add result with one folded i64 Const.",
                        1,
                        "Constant i64 Add can be folded when both operands are known.",
                    ),
                    BuiltInRuleKind::ExperimentalFoldI64ConstAdd,
                ),
            ],
        }
    }

    /// Returns summaries for all registered rules in catalog order.
    #[must_use]
    pub fn summaries(&self) -> Vec<RuleSummary> {
        self.rules.iter().map(|rule| rule.summary.clone()).collect()
    }

    /// Returns one registered rule by stable rule ID.
    #[must_use]
    pub fn find(&self, rule_id: &str) -> Option<&RuleDefinition> {
        self.rules.iter().find(|rule| rule.summary.id == rule_id)
    }
}

impl Default for RewriteCatalog {
    fn default() -> Self {
        Self::built_in()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_catalog_exposes_expected_rule_ids() {
        let ids: Vec<_> = RewriteCatalog::built_in()
            .summaries()
            .into_iter()
            .map(|summary| summary.id)
            .collect();

        assert_eq!(
            ids,
            vec![
                "i64-add-zero-right",
                "i64-add-zero-left",
                "i64-mul-one-right",
                "experimental-fold-i64-const-add",
            ]
        );
    }

    #[test]
    fn experimental_const_fold_is_preview_only() {
        let catalog = RewriteCatalog::built_in();
        let rule = catalog
            .find("experimental-fold-i64-const-add")
            .expect("invariant: built-in rule exists");

        assert!(!rule.summary.apply_capable);
        assert_eq!(rule.kind, BuiltInRuleKind::ExperimentalFoldI64ConstAdd);
    }
}
