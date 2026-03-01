//! AI agent module for LLM-driven graph mutation.
//!
//! Provides [`LlmClient`] (an enum over providers) and [`call_mutation`]
//! which orchestrates the full prompt → LLM → patch → validate → retry loop.

pub mod anthropic;
pub mod openai;
pub mod orchestrator;

use thiserror::Error;

use crate::patch::PatchOp;

/// Errors originating from LLM provider calls.
#[allow(dead_code)] // Some variants used in future provider error paths
#[derive(Debug, Error)]
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
}

/// Abstraction over LLM providers.
///
/// Call [`LlmClient::call_with_tools`] to send a prompt and receive a list
/// of [`PatchOp`] values derived from the LLM's tool call responses.
pub enum LlmClient {
    /// Anthropic Claude (tool_use API).
    Anthropic(anthropic::AnthropicClient),
    /// OpenAI (function calling API).
    OpenAi(openai::OpenAiClient),
}

impl LlmClient {
    /// Creates an Anthropic client from the given model and API key.
    #[must_use]
    pub fn anthropic(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        LlmClient::Anthropic(anthropic::AnthropicClient::new(model, api_key))
    }

    /// Creates an OpenAI client from the given model and API key.
    #[must_use]
    pub fn openai(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        LlmClient::OpenAi(openai::OpenAiClient::new(model, api_key))
    }

    /// Sends a prompt with graph context to the LLM and returns parsed [`PatchOp`] values.
    ///
    /// Returns the list of operations proposed by the LLM. An empty list means the
    /// LLM made no tool calls.
    pub async fn call_with_tools(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<Vec<PatchOp>, AgentError> {
        match self {
            LlmClient::Anthropic(c) => c.call_with_tools(system_prompt, user_message).await,
            LlmClient::OpenAi(c) => c.call_with_tools(system_prompt, user_message).await,
        }
    }

    /// Sends a prompt to the LLM with streaming text output via `on_text`.
    ///
    /// For Anthropic, text content blocks are streamed in real time via the
    /// server-sent events API. For OpenAI, this falls back to non-streaming
    /// (tool call arguments are not surfaced as streaming text).
    ///
    /// Returns the parsed [`PatchOp`] values once the full response is received.
    pub async fn call_with_tools_streaming<F>(
        &self,
        system_prompt: &str,
        user_message: &str,
        on_text: &F,
    ) -> Result<Vec<PatchOp>, AgentError>
    where
        F: Fn(&str),
    {
        match self {
            LlmClient::Anthropic(c) => {
                c.call_with_tools_streaming(system_prompt, user_message, on_text)
                    .await
            }
            LlmClient::OpenAi(c) => {
                c.call_with_tools_streaming(system_prompt, user_message, on_text)
                    .await
            }
        }
    }
}
