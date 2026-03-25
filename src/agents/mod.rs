//! AI agent module for LLM-driven graph mutation.
//!
//! Provides the [`LlmProvider`] trait for pluggable LLM backends and
//! [`create_provider`] / [`create_provider_chain`] factory functions.
//! The orchestrator in [`orchestrator`] drives the full
//! prompt → LLM → patch → validate → retry loop.

pub mod agent_knowledge;
pub mod analyzer;
pub mod anthropic;
pub mod assembler;
pub mod cost;
pub mod factory;
pub mod fallback;
pub mod grok;
pub mod merger;
pub mod minimax;
pub mod openai;
pub mod openrouter;
pub mod orchestrator;
pub mod prompts;
pub mod rollback;
pub mod template;

use std::future::Future;
use std::pin::Pin;

use thiserror::Error;

use crate::patch::PatchOp;

/// Errors originating from LLM provider calls.
#[derive(Debug, Error)]
#[allow(dead_code)] // Timeout and RateLimited constructed by fallback/provider code paths
pub enum AgentError {
    /// HTTP request to the provider API failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The API returned an error response.
    #[error("Provider API error (status {status}): {body}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Response body.
        body: String,
    },

    /// The LLM response could not be parsed into patch operations.
    #[error("Failed to parse LLM response: {0}")]
    Parse(String),

    /// The LLM returned no tool calls in its response.
    #[error("LLM returned no tool calls — no mutations to apply")]
    NoToolCalls,

    /// Validation of the patched graph failed after max retries.
    #[error("Graph validation failed after retry: {0}")]
    ValidationFailed(String),

    /// Patch application failed.
    #[error("Patch application error: {0}")]
    PatchFailed(String),

    /// The LLM call timed out.
    #[error("Provider call timed out after {0}s")]
    Timeout(u64),

    /// The provider returned a rate-limit response (429).
    #[error("Rate limited by provider{}", .retry_after.map(|s| format!(", retry after {s}s")).unwrap_or_default())]
    RateLimited {
        /// Optional retry-after hint in seconds.
        retry_after: Option<u64>,
    },
}

impl AgentError {
    /// Returns `true` if this error is transient and a fallback provider
    /// should be attempted.
    ///
    /// Transient errors include network/timeout issues, server errors (5xx),
    /// and rate limiting (429). Auth errors (401/403), bad requests (400),
    /// and parse/logic errors are NOT transient.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        match self {
            AgentError::Http(_) | AgentError::Timeout(_) | AgentError::RateLimited { .. } => true,
            AgentError::ApiError { status, .. } => *status == 429 || *status >= 500,
            AgentError::Parse(_)
            | AgentError::NoToolCalls
            | AgentError::ValidationFailed(_)
            | AgentError::PatchFailed(_) => false,
        }
    }
}

/// Object-safe trait for LLM providers.
///
/// Each provider (Anthropic, OpenAI, Grok, OpenRouter) implements this trait
/// to enable dynamic dispatch and fallback chains via [`fallback::ProviderChain`].
///
/// The `on_text` parameter in [`call_with_tools_streaming`] uses `&dyn Fn(&str)`
/// instead of a generic `F: Fn(&str)` for object safety.
pub trait LlmProvider: Send + Sync {
    /// Returns the provider's display name (e.g. `"anthropic"`, `"grok"`).
    fn name(&self) -> &str;

    /// Sends a prompt with graph context to the LLM and returns parsed [`PatchOp`] values.
    fn call_with_tools<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>>;

    /// Sends a prompt to the LLM with streaming text output via `on_text`.
    ///
    /// For providers that support SSE streaming, text content is surfaced
    /// in real time. Otherwise, falls back to non-streaming.
    fn call_with_tools_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>>;
}

/// Type alias for a boxed LLM provider — the primary way callers hold providers.
pub type LlmClient = Box<dyn LlmProvider>;

/// Blanket impl so `Box<dyn LlmProvider>` and `&Box<dyn LlmProvider>` can be
/// used wherever `&dyn LlmProvider` is expected.
impl LlmProvider for Box<dyn LlmProvider> {
    fn name(&self) -> &str {
        (**self).name()
    }

    fn call_with_tools<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        (**self).call_with_tools(system_prompt, user_message)
    }

    fn call_with_tools_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        (**self).call_with_tools_streaming(system_prompt, user_message, on_text)
    }
}
