//! Grok (xAI) provider — delegates to [`OpenAiClient`] with xAI's base URL.
//!
//! xAI's API is OpenAI-compatible, so this is a thin wrapper over the
//! existing OpenAI client with the xAI endpoint.

use std::future::Future;
use std::pin::Pin;

use crate::agents::openai::OpenAiClient;
use crate::agents::{AgentError, LlmProvider};
use crate::patch::PatchOp;

const GROK_API_URL: &str = "https://api.x.ai/v1/chat/completions";

/// Grok LLM client (xAI API — OpenAI-compatible).
pub struct GrokClient(OpenAiClient);

impl GrokClient {
    /// Creates a new Grok client targeting the xAI API.
    #[must_use]
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self(OpenAiClient::with_base_url(model, api_key, GROK_API_URL).with_provider_name("grok"))
    }
}

impl LlmProvider for GrokClient {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn call_with_tools<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        self.0.call_with_tools(system_prompt, user_message)
    }

    fn call_with_tools_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        self.0
            .call_with_tools_streaming(system_prompt, user_message, on_text)
    }
}
