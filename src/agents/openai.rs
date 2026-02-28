//! OpenAI provider using the function calling API.

use reqwest::Client;
use serde_json::json;

use crate::agents::AgentError;
use crate::patch::PatchOp;
use crate::tools::{OpenAiToolCall, openai_tools, patch_op_from_openai};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI LLM client using the function calling API.
pub struct OpenAiClient {
    model: String,
    api_key: String,
    http: Client,
}

impl OpenAiClient {
    /// Creates a new OpenAI client.
    #[must_use]
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
            http: Client::new(),
        }
    }

    /// Sends a message to OpenAI with graph-mutation tools attached.
    ///
    /// Returns the list of `PatchOp` values parsed from tool call responses.
    pub async fn call_with_tools(
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

        let resp = self
            .http
            .post(OPENAI_API_URL)
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

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
}
