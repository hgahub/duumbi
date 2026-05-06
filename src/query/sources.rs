//! Query answer metadata and source references.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::interaction::InteractionMode;

/// Confidence level attached to a query answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnswerConfidence {
    /// Answer comes directly from local project state.
    High,
    /// Answer combines local facts with interpretation.
    Medium,
    /// Answer is partial or depends on missing context.
    Low,
}

/// Optional explicit handoff from Query mode to a write-capable mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeHandoff {
    /// Suggested mode for the follow-up action.
    pub mode: InteractionMode,
    /// Concrete request text to submit in that mode.
    pub suggested_request: String,
}

/// Source reference used to ground a query answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceRef {
    /// Workspace graph module file.
    GraphModule {
        /// Module name.
        module: String,
        /// Module JSON-LD path.
        path: PathBuf,
    },
    /// Intent YAML file.
    Intent {
        /// Intent slug.
        slug: String,
        /// Intent file path.
        path: PathBuf,
    },
    /// Knowledge node.
    KnowledgeNode {
        /// Knowledge node identifier.
        id: String,
        /// Knowledge node type.
        node_type: String,
    },
    /// Persistent session turn.
    SessionTurn {
        /// Turn index in current session.
        index: usize,
    },
    /// Workspace summary derived from graph analysis.
    WorkspaceSummary,
}

/// Answer returned from Query mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryAnswer {
    /// Assistant text.
    pub text: String,
    /// Non-secret provider/model label used for the answer.
    pub model: String,
    /// Local sources used to build the answer context.
    pub sources: Vec<SourceRef>,
    /// Confidence estimate.
    pub confidence: AnswerConfidence,
    /// Optional suggested next action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_handoff: Option<ModeHandoff>,
}
