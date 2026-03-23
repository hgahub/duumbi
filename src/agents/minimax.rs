//! MiniMax provider — delegates to [`OpenAiClient`] with MiniMax's base URL.
//!
//! MiniMax's API is OpenAI-compatible, so this is a thin wrapper over the
//! existing OpenAI client with the MiniMax endpoint.

use std::future::Future;
use std::pin::Pin;

use crate::agents::openai::OpenAiClient;
use crate::agents::{AgentError, LlmProvider};
use crate::patch::PatchOp;

const MINIMAX_API_URL: &str = "https://api.minimax.io/v1/chat/completions";

/// MiniMax LLM client (OpenAI-compatible).
pub struct MiniMaxClient(OpenAiClient);

impl MiniMaxClient {
    /// Creates a new MiniMax client targeting the MiniMax API.
    #[must_use]
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self(
            OpenAiClient::with_base_url(model, api_key, MINIMAX_API_URL)
                .with_provider_name("minimax"),
        )
    }
}

/// Creates a MiniMax client targeting a custom URL (for testing).
#[cfg(test)]
impl MiniMaxClient {
    fn with_base_url(model: impl Into<String>, api_key: impl Into<String>, url: &str) -> Self {
        Self(OpenAiClient::with_base_url(model, api_key, url).with_provider_name("minimax"))
    }
}

impl LlmProvider for MiniMaxClient {
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
    fn minimax_name_returns_minimax() {
        let client = MiniMaxClient::new("MiniMax-M2.7", "dummy-key");
        assert_eq!(client.name(), "minimax");
    }

    #[tokio::test]
    async fn minimax_call_with_tools_parses_tool_call() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response()))
            .mount(&server)
            .await;

        let client = MiniMaxClient::with_base_url("MiniMax-M2.7", "key", &server.uri());
        let ops = client
            .call_with_tools("sys", "user")
            .await
            .expect("must succeed");
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], PatchOp::ModifyOp { .. }));
    }

    #[tokio::test]
    async fn minimax_call_with_tools_401_returns_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let client = MiniMaxClient::with_base_url("MiniMax-M2.7", "key", &server.uri());
        let err = client
            .call_with_tools("sys", "user")
            .await
            .expect_err("must fail");
        assert!(matches!(err, AgentError::ApiError { status: 401, .. }));
    }
}
