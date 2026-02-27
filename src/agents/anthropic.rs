//! Anthropic Claude provider using the tool_use API.

use reqwest::Client;
use serde_json::json;

use crate::agents::AgentError;
use crate::patch::PatchOp;
use crate::tools::{AnthropicToolCall, anthropic_tools, patch_op_from_anthropic};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;

/// Anthropic Claude LLM client using the tool_use API.
pub struct AnthropicClient {
    model: String,
    api_key: String,
    http: Client,
}

impl AnthropicClient {
    /// Creates a new Anthropic client.
    #[must_use]
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
            http: Client::new(),
        }
    }

    /// Sends a message to Claude with graph-mutation tools attached.
    ///
    /// Returns the list of `PatchOp` values parsed from tool call responses.
    pub async fn call_with_tools(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<Vec<PatchOp>, AgentError> {
        let tools = anthropic_tools();
        let tools_json = serde_json::to_value(&tools)
            .map_err(|e| AgentError::Parse(format!("Failed to serialize tools: {e}")))?;

        let body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": system_prompt,
            "tools": tools_json,
            "messages": [
                { "role": "user", "content": user_message }
            ]
        });

        let resp = self
            .http
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
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
        parse_anthropic_response(&response)
    }
}

/// Parses the Anthropic API response into a list of `PatchOp` values.
fn parse_anthropic_response(response: &serde_json::Value) -> Result<Vec<PatchOp>, AgentError> {
    let content = response["content"]
        .as_array()
        .ok_or_else(|| AgentError::Parse("Response has no 'content' array".to_string()))?;

    let mut ops = Vec::new();

    for item in content {
        if item.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
            let call: AnthropicToolCall = serde_json::from_value(item.clone())
                .map_err(|e| AgentError::Parse(format!("Failed to parse tool_use block: {e}")))?;
            let op = patch_op_from_anthropic(&call).map_err(AgentError::Parse)?;
            ops.push(op);
        }
    }

    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_anthropic_response_extracts_tool_calls() {
        let response = json!({
            "content": [
                {
                    "type": "text",
                    "text": "I'll add a constant op."
                },
                {
                    "type": "tool_use",
                    "id": "toolu_01",
                    "name": "modify_op",
                    "input": {
                        "node_id": "duumbi:main/main/entry/0",
                        "field": "duumbi:value",
                        "value": 42
                    }
                }
            ]
        });
        let ops = parse_anthropic_response(&response).expect("must parse");
        assert_eq!(ops.len(), 1);
        assert!(
            matches!(&ops[0], PatchOp::ModifyOp { node_id, .. } if node_id == "duumbi:main/main/entry/0")
        );
    }

    #[test]
    fn parse_anthropic_response_with_no_tool_calls_returns_empty() {
        let response = json!({
            "content": [
                { "type": "text", "text": "I have no changes to make." }
            ]
        });
        let ops = parse_anthropic_response(&response).expect("must succeed with empty ops");
        assert!(ops.is_empty());
    }

    #[test]
    fn parse_anthropic_response_missing_content_returns_error() {
        let response = json!({ "stop_reason": "end_turn" });
        let err = parse_anthropic_response(&response).expect_err("must error on missing content");
        assert!(matches!(err, AgentError::Parse(_)));
    }

    #[test]
    fn parse_anthropic_response_multiple_tool_calls() {
        let response = json!({
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_01",
                    "name": "remove_node",
                    "input": { "node_id": "duumbi:main/main/entry/0" }
                },
                {
                    "type": "tool_use",
                    "id": "toolu_02",
                    "name": "add_op",
                    "input": {
                        "block_id": "duumbi:main/main/entry",
                        "op": {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:main/main/entry/0",
                            "duumbi:value": 99,
                            "duumbi:resultType": "i64"
                        }
                    }
                }
            ]
        });
        let ops = parse_anthropic_response(&response).expect("must parse");
        assert_eq!(ops.len(), 2);
    }
}
