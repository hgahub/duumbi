//! OpenRouter provider — delegates to [`OpenAiClient`] with OpenRouter headers.
//!
//! OpenRouter's API is OpenAI-compatible but requires `X-Title` and
//! `HTTP-Referer` headers for attribution.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::agents::openai::OpenAiClient;
use crate::agents::{AgentError, LlmProvider};
use crate::patch::PatchOp;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// OpenRouter LLM client (OpenAI-compatible with extra headers).
pub struct OpenRouterClient(OpenAiClient);

impl OpenRouterClient {
    /// Creates a new OpenRouter client with required attribution headers.
    #[must_use]
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        let mut headers = HashMap::new();
        headers.insert("X-Title".to_string(), "duumbi".to_string());
        headers.insert("HTTP-Referer".to_string(), "https://duumbi.dev".to_string());

        Self(
            OpenAiClient::with_base_url(model, api_key, OPENROUTER_API_URL)
                .with_extra_headers(headers)
                .with_provider_name("openrouter"),
        )
    }
}

impl LlmProvider for OpenRouterClient {
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
