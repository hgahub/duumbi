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

/// Creates a Grok client targeting a custom URL (for testing).
#[cfg(test)]
impl GrokClient {
    fn with_base_url(model: impl Into<String>, api_key: impl Into<String>, url: &str) -> Self {
        Self(OpenAiClient::with_base_url(model, api_key, url).with_provider_name("grok"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
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
                            "arguments": r#"{"node_id":"duumbi:main/main/entry/0","field":"duumbi:value","value":99}"#
                        }
                    }]
                }
            }]
        })
    }

    #[test]
    fn grok_name_returns_grok() {
        let client = GrokClient::new("grok-beta", "dummy-key");
        assert_eq!(client.name(), "grok");
    }

    #[tokio::test]
    async fn grok_call_with_tools_parses_tool_call() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response()))
            .mount(&server)
            .await;

        let client = GrokClient::with_base_url("grok-beta", "key", &server.uri());
        let ops = client
            .call_with_tools("sys", "user")
            .await
            .expect("must succeed");
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], PatchOp::ModifyOp { .. }));
    }

    #[tokio::test]
    async fn grok_call_with_tools_401_returns_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let client = GrokClient::with_base_url("grok-beta", "key", &server.uri());
        let err = client
            .call_with_tools("sys", "user")
            .await
            .expect_err("must fail");
        assert!(matches!(err, AgentError::ApiError { status: 401, .. }));
    }

    #[tokio::test]
    async fn grok_call_with_tools_no_tool_calls_returns_empty() {
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

        let client = GrokClient::with_base_url("grok-beta", "key", &server.uri());
        let ops = client
            .call_with_tools("sys", "user")
            .await
            .expect("must succeed");
        assert!(ops.is_empty());
    }
}
