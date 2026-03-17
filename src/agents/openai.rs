//! OpenAI-compatible provider using the function calling API.
//!
//! Also serves as the base for Grok and OpenRouter providers via
//! [`with_base_url`] and [`with_extra_headers`].

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use reqwest::Client;
use serde_json::json;

use crate::agents::{AgentError, LlmProvider};
use crate::patch::PatchOp;
use crate::tools::{OpenAiToolCall, openai_tools, patch_op_from_openai};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI LLM client using the function calling API.
///
/// Also used as the inner implementation for Grok and OpenRouter providers,
/// which are OpenAI-compatible APIs with different base URLs and headers.
pub struct OpenAiClient {
    model: String,
    api_key: String,
    base_url: String,
    extra_headers: HashMap<String, String>,
    provider_name: String,
    http: Client,
}

impl OpenAiClient {
    /// Creates a new OpenAI client with the default API URL.
    #[must_use]
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
            base_url: OPENAI_API_URL.to_string(),
            extra_headers: HashMap::new(),
            provider_name: "openai".to_string(),
            http: Client::new(),
        }
    }

    /// Creates a new client with a custom base URL (for OpenAI-compatible APIs).
    #[must_use]
    pub fn with_base_url(
        model: impl Into<String>,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
            base_url: base_url.into(),
            extra_headers: HashMap::new(),
            provider_name: "openai".to_string(),
            http: Client::new(),
        }
    }

    /// Adds extra HTTP headers to every request.
    #[must_use]
    pub fn with_extra_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.extra_headers = headers;
        self
    }

    /// Sets the provider name for display purposes.
    #[must_use]
    pub fn with_provider_name(mut self, name: impl Into<String>) -> Self {
        self.provider_name = name.into();
        self
    }

    /// Sends a message to the OpenAI-compatible API (internal).
    async fn do_call_with_tools(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<Vec<PatchOp>, AgentError> {
        let tools = openai_tools();
        let tools_json = serde_json::to_value(&tools)
            .map_err(|e| AgentError::Parse(format!("Failed to serialize tools: {e}")))?;

        let body = json!({
            "model": self.model,
            "tools": tools_json,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_message }
            ]
        });

        let mut req = self
            .http
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json");

        for (key, value) in &self.extra_headers {
            req = req.header(key.as_str(), value.as_str());
        }

        let resp = req.json(&body).send().await?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(AgentError::ApiError {
                status,
                body: body_text,
            });
        }

        let response: serde_json::Value = resp.json().await?;
        parse_openai_response(&response)
    }
}

impl LlmProvider for OpenAiClient {
    fn name(&self) -> &str {
        &self.provider_name
    }

    fn call_with_tools<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        Box::pin(self.do_call_with_tools(system_prompt, user_message))
    }

    fn call_with_tools_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        _on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        // OpenAI streaming for tool calls requires complex delta reconstruction;
        // fall back to non-streaming.
        Box::pin(self.do_call_with_tools(system_prompt, user_message))
    }
}

/// Parses the OpenAI API response into a list of `PatchOp` values.
fn parse_openai_response(response: &serde_json::Value) -> Result<Vec<PatchOp>, AgentError> {
    let message = response
        .pointer("/choices/0/message")
        .ok_or_else(|| AgentError::Parse("Response has no choices[0].message".to_string()))?;

    let tool_calls = match message.get("tool_calls").and_then(|v| v.as_array()) {
        Some(calls) if !calls.is_empty() => calls,
        _ => return Ok(Vec::new()), // No tool calls — no mutations
    };

    let mut ops = Vec::new();
    for item in tool_calls {
        let call: OpenAiToolCall = serde_json::from_value(item.clone())
            .map_err(|e| AgentError::Parse(format!("Failed to parse tool call: {e}")))?;
        let op = patch_op_from_openai(&call).map_err(AgentError::Parse)?;
        ops.push(op);
    }

    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_openai_response_extracts_tool_calls() {
        let response = json!({
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
        });
        let ops = parse_openai_response(&response).expect("must parse");
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], PatchOp::ModifyOp { .. }));
    }

    #[test]
    fn parse_openai_response_no_tool_calls_returns_empty() {
        let response = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "No changes needed."
                }
            }]
        });
        let ops = parse_openai_response(&response).expect("must succeed");
        assert!(ops.is_empty());
    }

    #[test]
    fn parse_openai_response_missing_choices_returns_error() {
        let response = json!({ "id": "chatcmpl-123" });
        let err = parse_openai_response(&response).expect_err("must error");
        assert!(matches!(err, AgentError::Parse(_)));
    }

    #[test]
    fn parse_openai_response_multiple_tool_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [
                        {
                            "type": "function",
                            "function": {
                                "name": "remove_node",
                                "arguments": r#"{"node_id":"duumbi:main/main/entry/1"}"#
                            }
                        },
                        {
                            "type": "function",
                            "function": {
                                "name": "add_op",
                                "arguments": r#"{"block_id":"duumbi:main/main/entry","op":{"@type":"duumbi:Return","@id":"duumbi:main/main/entry/1","duumbi:operand":{"@id":"duumbi:main/main/entry/0"}}}"#
                            }
                        }
                    ]
                }
            }]
        });
        let ops = parse_openai_response(&response).expect("must parse");
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn openai_client_with_base_url() {
        let client = OpenAiClient::with_base_url("gpt-4", "key", "https://custom.api/v1");
        assert_eq!(client.base_url, "https://custom.api/v1");
    }

    #[test]
    fn openai_client_with_extra_headers() {
        let mut headers = HashMap::new();
        headers.insert("X-Title".to_string(), "duumbi".to_string());
        let client = OpenAiClient::new("gpt-4", "key").with_extra_headers(headers);
        assert_eq!(client.extra_headers["X-Title"], "duumbi");
    }

    #[test]
    fn openai_client_with_provider_name() {
        let client = OpenAiClient::new("gpt-4", "key").with_provider_name("grok");
        assert_eq!(client.name(), "grok");
    }
}
