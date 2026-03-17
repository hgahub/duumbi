//! Anthropic Claude provider using the tool_use API.

use std::collections::HashMap;

use futures_util::StreamExt as _;
use reqwest::Client;
use serde_json::json;

use std::future::Future;
use std::pin::Pin;

use crate::agents::{AgentError, LlmProvider};
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

    /// Sends a message to Claude with graph-mutation tools attached (internal).
    async fn do_call_with_tools(
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

    /// Sends a streaming message to Claude (internal).
    async fn do_call_with_tools_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Result<Vec<PatchOp>, AgentError> {
        let tools = anthropic_tools();
        let tools_json = serde_json::to_value(&tools)
            .map_err(|e| AgentError::Parse(format!("Failed to serialize tools: {e}")))?;

        let body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": system_prompt,
            "tools": tools_json,
            "stream": true,
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

        parse_sse_stream(resp, on_text).await
    }
}

impl LlmProvider for AnthropicClient {
    fn name(&self) -> &str {
        "anthropic"
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
        on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        Box::pin(self.do_call_with_tools_streaming(system_prompt, user_message, on_text))
    }
}

// ---------------------------------------------------------------------------
// SSE stream parser
// ---------------------------------------------------------------------------

/// State for one accumulating tool_use content block.
struct ToolBlock {
    name: String,
    json_buf: String,
}

/// Consumes an Anthropic SSE response stream, calling `on_text` for each
/// streamed text chunk and accumulating tool call inputs to return as `PatchOp`s.
async fn parse_sse_stream(
    resp: reqwest::Response,
    on_text: &(dyn Fn(&str) + Send + Sync),
) -> Result<Vec<PatchOp>, AgentError> {
    let mut byte_stream = resp.bytes_stream();
    let mut line_buf = String::new();

    // content_block index → ToolBlock
    let mut tool_blocks: HashMap<usize, ToolBlock> = HashMap::new();
    // Ordered list of tool_use block indices (preserves call order)
    let mut tool_indices: Vec<usize> = Vec::new();

    while let Some(chunk) = byte_stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);

        for ch in text.chars() {
            if ch == '\n' {
                let line = std::mem::take(&mut line_buf);
                process_sse_line(&line, &mut tool_blocks, &mut tool_indices, on_text);
            } else if ch != '\r' {
                line_buf.push(ch);
            }
        }
    }

    // Process any remaining buffered line (stream ended without trailing newline)
    if !line_buf.is_empty() {
        process_sse_line(&line_buf, &mut tool_blocks, &mut tool_indices, on_text);
    }

    // Build PatchOps from completed tool blocks, in call order
    let mut ops = Vec::new();
    for idx in tool_indices {
        if let Some(block) = tool_blocks.remove(&idx) {
            let input: serde_json::Value = serde_json::from_str(&block.json_buf).map_err(|e| {
                AgentError::Parse(format!(
                    "Failed to parse tool '{}' input JSON: {e}",
                    block.name
                ))
            })?;
            let call = AnthropicToolCall {
                name: block.name,
                input,
            };
            let op = patch_op_from_anthropic(&call).map_err(AgentError::Parse)?;
            ops.push(op);
        }
    }

    Ok(ops)
}

/// Processes one SSE `data:` line, updating tool state and invoking `on_text`.
fn process_sse_line(
    line: &str,
    tool_blocks: &mut HashMap<usize, ToolBlock>,
    tool_indices: &mut Vec<usize>,
    on_text: &dyn Fn(&str),
) {
    let Some(data) = line.strip_prefix("data: ") else {
        return;
    };

    let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
        return;
    };

    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "content_block_start" => {
            let index = event.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let block_type = event
                .pointer("/content_block/type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if block_type == "tool_use" {
                let name = event
                    .pointer("/content_block/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                tool_blocks.insert(
                    index,
                    ToolBlock {
                        name,
                        json_buf: String::new(),
                    },
                );
                tool_indices.push(index);
            }
        }
        "content_block_delta" => {
            let index = event.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let delta_type = event
                .pointer("/delta/type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match delta_type {
                "text_delta" => {
                    if let Some(text) = event.pointer("/delta/text").and_then(|v| v.as_str()) {
                        on_text(text);
                    }
                }
                "input_json_delta" => {
                    if let Some(partial) = event
                        .pointer("/delta/partial_json")
                        .and_then(|v| v.as_str())
                        && let Some(block) = tool_blocks.get_mut(&index)
                    {
                        block.json_buf.push_str(partial);
                    }
                }
                _ => {}
            }
        }
        _ => {}
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

    // -------------------------------------------------------------------------
    // SSE line processor tests
    // -------------------------------------------------------------------------

    fn make_sse_line(event_json: serde_json::Value) -> String {
        format!("data: {event_json}")
    }

    #[test]
    fn process_sse_line_text_delta_calls_on_text() {
        let line = make_sse_line(json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": { "type": "text_delta", "text": "Hello " }
        }));

        let mut tool_blocks = std::collections::HashMap::new();
        let mut tool_indices = Vec::new();
        let captured = std::cell::RefCell::new(String::new());

        process_sse_line(&line, &mut tool_blocks, &mut tool_indices, &|t| {
            captured.borrow_mut().push_str(t);
        });

        assert_eq!(*captured.borrow(), "Hello ");
        assert!(tool_indices.is_empty());
    }

    #[test]
    fn process_sse_line_content_block_start_tool_use_registers_block() {
        let line = make_sse_line(json!({
            "type": "content_block_start",
            "index": 1,
            "content_block": { "type": "tool_use", "id": "t01", "name": "modify_op", "input": {} }
        }));

        let mut tool_blocks = std::collections::HashMap::new();
        let mut tool_indices = Vec::new();

        process_sse_line(&line, &mut tool_blocks, &mut tool_indices, &|_| {});

        assert_eq!(tool_indices, vec![1]);
        assert!(tool_blocks.contains_key(&1));
        assert_eq!(tool_blocks[&1].name, "modify_op");
    }

    #[test]
    fn process_sse_line_input_json_delta_accumulates() {
        let mut tool_blocks = std::collections::HashMap::new();
        let mut tool_indices = Vec::new();

        // First register the tool_use block
        let start_line = make_sse_line(json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": { "type": "tool_use", "id": "t01", "name": "remove_node", "input": {} }
        }));
        process_sse_line(&start_line, &mut tool_blocks, &mut tool_indices, &|_| {});

        // Then send partial JSON deltas
        for fragment in ["{\"node_id\":", "\"duumbi:x\"}"] {
            let delta_line = make_sse_line(json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": { "type": "input_json_delta", "partial_json": fragment }
            }));
            process_sse_line(&delta_line, &mut tool_blocks, &mut tool_indices, &|_| {});
        }

        assert_eq!(tool_blocks[&0].json_buf, r#"{"node_id":"duumbi:x"}"#);
    }

    #[test]
    fn process_sse_line_ignores_non_data_lines() {
        let mut tool_blocks = std::collections::HashMap::new();
        let mut tool_indices = Vec::new();
        let called = std::cell::Cell::new(false);

        process_sse_line("event: ping", &mut tool_blocks, &mut tool_indices, &|_| {
            called.set(true);
        });
        process_sse_line("", &mut tool_blocks, &mut tool_indices, &|_| {
            called.set(true);
        });

        assert!(!called.get());
        assert!(tool_indices.is_empty());
    }

    #[test]
    fn process_sse_line_non_tool_use_content_block_is_ignored() {
        let line = make_sse_line(json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": { "type": "text", "text": "" }
        }));

        let mut tool_blocks = std::collections::HashMap::new();
        let mut tool_indices = Vec::new();

        process_sse_line(&line, &mut tool_blocks, &mut tool_indices, &|_| {});

        assert!(tool_indices.is_empty());
        assert!(tool_blocks.is_empty());
    }
}
