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
        Self::with_base_url_inner(model, api_key, OPENROUTER_API_URL)
    }

    /// Shared constructor used by both `new` and the test helper — single source of truth
    /// for header setup so production and test construction paths never drift.
    fn with_base_url_inner(
        model: impl Into<String>,
        api_key: impl Into<String>,
        url: &str,
    ) -> Self {
        let mut headers = HashMap::new();
        headers.insert("X-Title".to_string(), "duumbi".to_string());
        headers.insert("HTTP-Referer".to_string(), "https://duumbi.dev".to_string());
        Self(
            OpenAiClient::with_base_url(model, api_key, url)
                .with_extra_headers(headers)
                .with_provider_name("openrouter"),
        )
    }
}

/// Creates an OpenRouter client targeting a custom URL (for testing).
#[cfg(test)]
impl OpenRouterClient {
    fn with_base_url(model: impl Into<String>, api_key: impl Into<String>, url: &str) -> Self {
        Self::with_base_url_inner(model, api_key, url)
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn tool_call_response() -> serde_json::Value {
        serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "type": "function",
                        "function": {
                            "name": "modify_op",
                            "arguments": r#"{"node_id":"duumbi:main/main/entry/0","field":"duumbi:value","value":42}"#
                        }
                    }]
                }
            }]
        })
    }

    #[test]
    fn openrouter_name_returns_openrouter() {
        let client = OpenRouterClient::new("mistral-7b", "dummy-key");
        assert_eq!(client.name(), "openrouter");
    }

    #[tokio::test]
    async fn openrouter_sends_attribution_headers() {
        let server = MockServer::start().await;

        // Only respond 200 if both required attribution headers are present.
        // Missing headers → wiremock returns 404 → the test would fail on expect("must succeed").
        Mock::given(method("POST"))
            .and(path("/"))
            .and(header("X-Title", "duumbi"))
            .and(header("HTTP-Referer", "https://duumbi.dev"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response()))
            .mount(&server)
            .await;

        let client = OpenRouterClient::with_base_url("mistral-7b", "key", &server.uri());
        let ops = client
            .call_with_tools("sys", "user")
            .await
            .expect("must succeed — both attribution headers must be present");
        assert_eq!(ops.len(), 1);
    }

    #[tokio::test]
    async fn openrouter_call_with_tools_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response()))
            .mount(&server)
            .await;

        let client = OpenRouterClient::with_base_url("mistral-7b", "key", &server.uri());
        let ops = client
            .call_with_tools("sys", "user")
            .await
            .expect("must succeed");
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], PatchOp::ModifyOp { .. }));
    }

    #[tokio::test]
    async fn openrouter_call_with_tools_401_returns_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let client = OpenRouterClient::with_base_url("mistral-7b", "key", &server.uri());
        let err = client
            .call_with_tools("sys", "user")
            .await
            .expect_err("must fail");
        assert!(matches!(err, AgentError::ApiError { status: 401, .. }));
    }

    #[tokio::test]
    async fn openrouter_call_with_tools_no_tool_calls_returns_empty() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "No changes needed."
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;

        let client = OpenRouterClient::with_base_url("mistral-7b", "key", &server.uri());
        let ops = client
            .call_with_tools("sys", "user")
            .await
            .expect("must succeed");
        assert!(ops.is_empty());
    }
}
