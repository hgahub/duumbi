//! Semantic rewrite engine contracts.
//!
//! The rewrite module owns DUUMBI's deterministic graph-to-graph rewrite
//! substrate. V1 starts with provider-free rule metadata, bounded preview/apply
//! evidence, and typed errors. CLI, MCP, and apply adapters are layered on top
//! of these contracts in later Ralph cycles.

pub mod catalog;
pub mod engine;
pub mod error;
pub mod evidence;
pub mod rule;

pub use catalog::{BuiltInRuleKind, RewriteCatalog, RuleDefinition};
pub use engine::RewriteEngine;
pub use error::RewriteError;
pub use evidence::{
    ApplyMode, ApplyOptions, CostEvidence, RewriteApplyOutcome, RewriteApplyPlan, RewriteLimits,
    RewriteMatch, RewritePreview, ValidationEvidence,
};
pub use rule::{RuleCategory, RuleSummary, SafetyClass};
