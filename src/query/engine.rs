//! Query mode engine.

use std::path::{Path, PathBuf};

use crate::agents::{AgentError, LlmProvider};
use crate::context::ContextError;
use crate::interaction::InteractionMode;
use crate::interaction::router::{RequestShape, classify_request};
use crate::query::context::{QueryContextOptions, assemble_query_context};
use crate::query::prompt::{QUERY_SYSTEM_PROMPT, build_query_message};
use crate::query::sources::{AnswerConfidence, ModeHandoff, QueryAnswer};
use crate::session::PersistentTurn;

/// Query-mode result alias.
pub type Result<T> = std::result::Result<T, QueryError>;

/// Query mode errors.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    /// Context assembly failed.
    #[error(transparent)]
    Context(#[from] ContextError),
    /// LLM provider failed.
    #[error(transparent)]
    Agent(#[from] AgentError),
}

/// Read-only query request.
#[derive(Debug, Clone)]
pub struct QueryRequest {
    /// User question.
    pub question: String,
    /// Workspace root.
    pub workspace_root: PathBuf,
    /// Visible module, if known.
    pub visible_module: Option<String>,
    /// Visible C4 level, if known.
    pub c4_level: Option<String>,
    /// Recent session turns.
    pub session_turns: Vec<PersistentTurn>,
}

impl QueryRequest {
    /// Creates a query request for a workspace and question.
    #[must_use]
    pub fn new(workspace_root: impl AsRef<Path>, question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            workspace_root: workspace_root.as_ref().to_path_buf(),
            visible_module: None,
            c4_level: None,
            session_turns: Vec::new(),
        }
    }
}

/// Read-only query engine.
#[derive(Debug, Default)]
pub struct QueryEngine;

impl QueryEngine {
    /// Creates a query engine.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Answers a query using a provider and streams text through `on_text`.
    pub async fn answer_streaming(
        &self,
        client: &dyn LlmProvider,
        request: QueryRequest,
        on_text: &(dyn Fn(&str) + Send + Sync),
    ) -> Result<QueryAnswer> {
        let options = QueryContextOptions {
            visible_module: request.visible_module.clone(),
            c4_level: request.c4_level.clone(),
            session_turns: request.session_turns.clone(),
        };
        let context = assemble_query_context(&request.workspace_root, &options)?;
        let message = build_query_message(&request.question, &context.text);
        let text = client
            .answer_streaming(QUERY_SYSTEM_PROMPT, &message, on_text)
            .await?;
        let model = client.model_label();

        Ok(QueryAnswer {
            text: text.trim().to_string(),
            model,
            confidence: if context.sources.is_empty() {
                AnswerConfidence::Low
            } else {
                AnswerConfidence::Medium
            },
            sources: context.sources,
            suggested_handoff: suggested_handoff(&request.question),
        })
    }
}

fn suggested_handoff(question: &str) -> Option<ModeHandoff> {
    match classify_request(question) {
        RequestShape::Mutation => Some(ModeHandoff {
            mode: InteractionMode::Agent,
            suggested_request: question.trim().to_string(),
        }),
        RequestShape::Intent => Some(ModeHandoff {
            mode: InteractionMode::Intent,
            suggested_request: question.trim().to_string(),
        }),
        RequestShape::Question | RequestShape::Unknown => None,
    }
}
