//! Rewrite preview and apply evidence contracts.

use serde::{Deserialize, Serialize};

use super::rule::RuleSummary;

/// Safe default limits for preview and apply operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteLimits {
    /// Maximum matches returned by one preview.
    pub max_matches_per_preview: usize,
    /// Maximum matches applied without explicit apply-all.
    pub max_matches_per_apply: usize,
    /// Maximum matches applied by an explicit apply-all request.
    pub max_apply_all_matches: usize,
    /// Maximum graph nodes touched by one match.
    pub max_touched_nodes_per_match: usize,
    /// Maximum patch operations produced by one match.
    pub max_patch_ops_per_match: usize,
    /// Maximum node IDs rendered in explanations before truncation.
    pub max_explanation_nodes: usize,
}

impl Default for RewriteLimits {
    fn default() -> Self {
        Self {
            max_matches_per_preview: 100,
            max_matches_per_apply: 1,
            max_apply_all_matches: 10,
            max_touched_nodes_per_match: 25,
            max_patch_ops_per_match: 25,
            max_explanation_nodes: 20,
        }
    }
}

/// Bounded cost evidence for a rewrite operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostEvidence {
    /// Number of matches inspected.
    pub matches_considered: usize,
    /// Number of matches returned to the caller.
    pub matches_returned: usize,
    /// Number of matches omitted because of limits.
    pub matches_truncated: usize,
    /// Number of graph nodes touched.
    pub touched_node_count: usize,
    /// Number of patch operations produced.
    pub patch_op_count: usize,
    /// Deterministic abstract cost units.
    pub estimated_cost_units: u32,
    /// Limits used for the operation.
    pub limits: RewriteLimits,
}

impl CostEvidence {
    /// Creates zero-cost evidence using the provided limits.
    #[must_use]
    pub fn empty(limits: RewriteLimits) -> Self {
        Self {
            matches_considered: 0,
            matches_returned: 0,
            matches_truncated: 0,
            touched_node_count: 0,
            patch_op_count: 0,
            estimated_cost_units: 0,
            limits,
        }
    }
}

/// Validation status reported by preview or apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationEvidence {
    /// Whether validation was run.
    pub ran: bool,
    /// Whether the validated graph was valid.
    pub valid: bool,
    /// Sanitized validation diagnostic messages.
    pub diagnostics: Vec<String>,
}

impl ValidationEvidence {
    /// Returns evidence for a valid candidate.
    #[must_use]
    pub fn valid() -> Self {
        Self {
            ran: true,
            valid: true,
            diagnostics: Vec::new(),
        }
    }

    /// Returns evidence for a step where validation was not run.
    #[must_use]
    pub fn not_run() -> Self {
        Self {
            ran: false,
            valid: false,
            diagnostics: Vec::new(),
        }
    }
}

/// One deterministic rewrite match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteMatch {
    /// Stable match identifier for unchanged graph input.
    pub match_id: String,
    /// Rule ID that produced the match.
    pub rule_id: String,
    /// Module name or path inspected.
    pub module: String,
    /// Primary node for the match.
    pub primary_node_id: String,
    /// Nodes touched by the proposed rewrite.
    pub touched_node_ids: Vec<String>,
    /// Human-readable effect summary for this match.
    pub operation_summary: String,
    /// Human-readable explanation for this match.
    pub explanation: String,
    /// Cost evidence for this match.
    pub cost: CostEvidence,
    /// Validation evidence for the preview candidate, when available.
    pub validation: ValidationEvidence,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
}

/// Preview response shared by CLI and MCP adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewritePreview {
    /// Response status, such as `success`.
    pub status: String,
    /// Rule metadata.
    pub rule: RuleSummary,
    /// Matches returned by the preview.
    pub matches: Vec<RewriteMatch>,
    /// Aggregate preview cost evidence.
    pub cost: CostEvidence,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
}

/// Apply selection mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplyMode {
    /// Apply one selected match ID.
    Match,
    /// Apply all matches within configured bounds.
    All,
}

/// Options for a backend apply operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOptions {
    /// Rule ID to apply.
    pub rule_id: String,
    /// Optional module name or path.
    pub module: Option<String>,
    /// Apply mode.
    pub mode: ApplyMode,
    /// Selected match ID when mode is `match`.
    pub match_id: Option<String>,
    /// Maximum matches for apply-all mode.
    pub max_matches: Option<usize>,
}

/// Validated apply plan before adapter-level snapshot/write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteApplyPlan {
    /// Response status, such as `success`.
    pub status: String,
    /// Rule metadata.
    pub rule: RuleSummary,
    /// Match IDs selected for apply.
    pub match_ids: Vec<String>,
    /// Nodes touched by all selected matches.
    pub touched_node_ids: Vec<String>,
    /// Human-readable operation summary.
    pub operation_summary: String,
    /// Validation evidence for the candidate graph.
    pub validation: ValidationEvidence,
    /// Cost evidence for the apply plan.
    pub cost: CostEvidence,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
}

/// In-memory candidate source plus apply evidence.
///
/// CLI and MCP adapters own snapshot and write behavior after receiving this
/// outcome. The backend never writes the candidate source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteApplyOutcome {
    /// Validated candidate JSON-LD source.
    pub candidate_source: serde_json::Value,
    /// Evidence describing the selected rewrite operation.
    pub plan: RewriteApplyPlan,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rewrite::rule::{RuleCategory, SafetyClass};

    fn test_rule() -> RuleSummary {
        RuleSummary::new(
            "i64-add-zero-right",
            "I64 add zero right",
            RuleCategory::LocalOptimization,
            SafetyClass::LocalSemanticsPreserving,
            "Simplifies Add by zero",
            "Right operand is i64 zero",
            "Uses the left operand value",
            1,
            "Add by zero preserves the left operand",
        )
    }

    #[test]
    fn rewrite_limits_default_matches_spec() {
        let limits = RewriteLimits::default();
        assert_eq!(limits.max_matches_per_preview, 100);
        assert_eq!(limits.max_matches_per_apply, 1);
        assert_eq!(limits.max_apply_all_matches, 10);
        assert_eq!(limits.max_touched_nodes_per_match, 25);
        assert_eq!(limits.max_patch_ops_per_match, 25);
        assert_eq!(limits.max_explanation_nodes, 20);
    }

    #[test]
    fn preview_serializes_with_camel_case_fields() {
        let limits = RewriteLimits::default();
        let preview = RewritePreview {
            status: "success".to_string(),
            rule: test_rule(),
            matches: vec![RewriteMatch {
                match_id: "i64-add-zero-right:main:duumbi:main/main/entry/2:0".to_string(),
                rule_id: "i64-add-zero-right".to_string(),
                module: "main".to_string(),
                primary_node_id: "duumbi:main/main/entry/2".to_string(),
                touched_node_ids: vec!["duumbi:main/main/entry/2".to_string()],
                operation_summary: "Replace Add by zero with left operand".to_string(),
                explanation: "The right operand is i64 zero".to_string(),
                cost: CostEvidence::empty(limits),
                validation: ValidationEvidence::not_run(),
                warnings: Vec::new(),
            }],
            cost: CostEvidence::empty(limits),
            warnings: Vec::new(),
        };

        let value = serde_json::to_value(preview).expect("invariant: preview serializes");
        assert_eq!(value["status"], "success");
        assert!(value["matches"][0].get("matchId").is_some());
        assert!(value["matches"][0].get("touchedNodeIds").is_some());
        assert!(value["cost"].get("estimatedCostUnits").is_some());
    }

    #[test]
    fn apply_mode_serializes_to_kebab_case() {
        let value = serde_json::to_value(ApplyMode::All).expect("invariant: mode serializes");
        assert_eq!(value, serde_json::json!("all"));
    }
}
