//! Knowledge graph foundation for DUUMBI's learning system.
//!
//! Stores successful mutations, design decisions, and recurring patterns
//! as typed nodes in `.duumbi/knowledge/`. The context assembly pipeline
//! queries these nodes to inject relevant few-shot examples into LLM prompts.
//!
//! # Storage layout
//!
//! ```text
//! .duumbi/
//!   knowledge/
//!     success/    # SuccessRecord JSON files
//!     decision/   # DecisionRecord JSON files
//!     pattern/    # PatternRecord JSON files
//!   learning/
//!     successes.jsonl  # Append-only log for few-shot scoring
//! ```

pub mod learning;
pub mod store;
pub mod types;

/// Errors from knowledge store operations.
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    /// File I/O error.
    #[error("knowledge I/O error: {0}")]
    Io(String),

    /// Serialization/deserialization error.
    #[error("knowledge serialization error: {0}")]
    Serialize(String),
}
