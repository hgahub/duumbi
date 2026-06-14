//! Rewrite engine error types.

use thiserror::Error;

/// Errors produced by rewrite rule discovery, preview, and apply planning.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RewriteError {
    /// The requested rewrite rule is not registered.
    #[error("Unknown rewrite rule: '{0}'")]
    UnknownRule(String),

    /// The requested module could not be loaded.
    #[error("Module not found: '{0}'")]
    ModuleNotFound(String),

    /// The current graph is invalid and cannot be safely rewritten.
    #[error("Current graph is invalid: {0}")]
    InvalidCurrentGraph(String),

    /// The selected match does not exist in the current graph state.
    #[error("Rewrite match is stale or missing: '{0}'")]
    StaleMatch(String),

    /// The rule is preview-only in V1.
    #[error("Rewrite rule is preview-only in V1: '{0}'")]
    UnsupportedSafetyClass(String),

    /// The requested rewrite would exceed configured bounds.
    #[error("Rewrite cost bound exceeded: {0}")]
    CostBoundExceeded(String),

    /// The candidate graph failed parse, build, or validation.
    #[error("Rewrite candidate validation failed: {0}")]
    ValidationFailed(String),

    /// The requested apply shape is invalid.
    #[error("Invalid rewrite request: {0}")]
    InvalidRequest(String),
}
