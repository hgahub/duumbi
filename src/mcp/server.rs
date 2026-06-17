//! JSON-RPC 2.0 MCP server implementation.
//!
//! Implements the [Model Context Protocol](https://modelcontextprotocol.io)
//! over JSON-RPC 2.0 stdio transport. Each newline-delimited JSON object on
//! stdin is treated as a request; the corresponding response is written as a
//! newline-delimited JSON object to stdout.
//!
//! ## Supported MCP methods
//!
//! | Method                   | Description                          |
//! |--------------------------|--------------------------------------|
//! | `initialize`             | Handshake — returns server info      |
//! | `tools/list`             | List all available tool definitions  |
//! | `tools/call`             | Invoke a named tool                  |
//! | `notifications/initialized` | Client init ack (no response)     |

use std::io::{self, BufRead as _, Write as _};
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::capability::{self, ToolDefinition};
use super::error::McpToolError;
use super::tools;

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 types
// ---------------------------------------------------------------------------

/// JSON-RPC 2.0 request object.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    /// Must be `"2.0"`.
    #[allow(dead_code)] // Validated by JSON-RPC protocol, read by serde
    pub jsonrpc: String,
    /// Request identifier. `None` for notifications (no response expected).
    pub id: Option<Value>,
    /// Method name.
    pub method: String,
    /// Optional method parameters.
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response object.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Mirrors the request `id`.
    pub id: Option<Value>,
    /// Successful result payload, mutually exclusive with `error`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error payload, mutually exclusive with `result`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    /// Standard JSON-RPC error code.
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
    /// Optional additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Standard JSON-RPC error codes.
pub mod rpc_codes {
    /// Parse error — invalid JSON received by the server.
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid request — the JSON sent is not a valid Request object.
    #[allow(dead_code)] // Available for future error handling paths
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found — the method does not exist or is not available.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params — invalid method parameter(s).
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error — internal JSON-RPC error.
    pub const INTERNAL_ERROR: i32 = -32603;
}

// ---------------------------------------------------------------------------
// MCP server
// ---------------------------------------------------------------------------

/// DUUMBI MCP server.
///
/// Holds workspace state and handles JSON-RPC 2.0 requests.
pub struct McpServer {
    workspace: Arc<PathBuf>,
}

impl McpServer {
    /// Creates a new [`McpServer`] rooted at the given workspace path.
    #[must_use]
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace: Arc::new(workspace),
        }
    }

    /// Handle a single JSON-RPC request and return a response.
    ///
    /// Returns `None` for MCP notifications (which do not receive responses).
    #[must_use]
    pub fn handle_request(&self, request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        // Notifications have no id and no response should be sent.
        if request.id.is_none() && request.method.starts_with("notifications/") {
            return None;
        }

        let id = request.id.clone();
        let result = self.dispatch(&request.method, request.params.as_ref());

        Some(match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(value),
                error: None,
            },
            Err(err) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(err),
            },
        })
    }

    /// Return the list of available MCP tools with their JSON Schema definitions.
    #[must_use]
    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        capability::tool_definitions()
    }

    /// Run the server loop reading from stdin, writing to stdout.
    ///
    /// Each line is a JSON-RPC request; each non-notification response is
    /// written as a single line. Blank lines are ignored.
    ///
    /// This is a **synchronous** (blocking) function. Call it from an async
    /// context via `tokio::task::spawn_blocking` to avoid blocking the Tokio
    /// executor thread.
    pub fn run_stdio(&self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let response_json = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(req) => {
                    match self.handle_request(&req) {
                        None => continue, // notification — no response
                        Some(resp) => serde_json::to_string(&resp).unwrap_or_else(|e| {
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": null,
                                "error": {
                                    "code": rpc_codes::INTERNAL_ERROR,
                                    "message": format!("Response serialization failed: {e}")
                                }
                            })
                            .to_string()
                        }),
                    }
                }
                Err(e) => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": rpc_codes::PARSE_ERROR,
                        "message": format!("Parse error: {e}")
                    }
                })
                .to_string(),
            };

            writeln!(stdout, "{response_json}")?;
            stdout.flush()?;
        }

        Ok(())
    }

    /// Dispatch a `tools/call` request to the appropriate tool handler.
    fn dispatch_tool_call(&self, tool_name: &str, args: &Value) -> Result<Value, JsonRpcError> {
        let workspace = self.workspace.as_ref();

        let tool_result = match tool_name {
            "mcp_capability_status" => tools::status::mcp_capability_status(workspace, args),
            "graph_query" => tools::graph::graph_query(workspace, args),
            "graph_mutate" => tools::graph::graph_mutate(workspace, args),
            "graph_validate" => tools::graph::graph_validate(workspace, args),
            "graph_describe" => tools::graph::graph_describe(workspace, args),
            "build_compile" => tools::build::build_compile(workspace, args),
            "build_run" => tools::build::build_run(workspace, args),
            "deps_search" => tools::deps::deps_search(workspace, args),
            "deps_install" => tools::deps::deps_install(workspace, args),
            "intent_create" => tools::intent::intent_create(workspace, args),
            "intent_execute" => tools::intent::intent_execute(workspace, args),
            "model_access_summary" => tools::model_telemetry::model_access_summary(workspace, args),
            "model_performance_summary" => {
                tools::model_telemetry::model_performance_summary(workspace, args)
            }
            "model_telemetry_health" => {
                tools::model_telemetry::model_telemetry_health(workspace, args)
            }
            "rewrite_list_rules" => tools::rewrite::rewrite_list_rules(workspace, args),
            "rewrite_preview" => tools::rewrite::rewrite_preview(workspace, args),
            "rewrite_apply" => tools::rewrite::rewrite_apply(workspace, args),
            _ => {
                return Err(JsonRpcError {
                    code: rpc_codes::METHOD_NOT_FOUND,
                    message: format!("Unknown tool: '{tool_name}'"),
                    data: None,
                });
            }
        };

        tool_result.map_err(|msg| JsonRpcError {
            code: rpc_codes::INTERNAL_ERROR,
            message: msg.clone(),
            data: Some(McpToolError::from_tool_message(tool_name, msg).to_value()),
        })
    }

    /// Dispatch an incoming JSON-RPC method to the right handler.
    fn dispatch(&self, method: &str, params: Option<&Value>) -> Result<Value, JsonRpcError> {
        match method {
            "initialize" => Ok(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "duumbi-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {}
                }
            })),

            "tools/list" => {
                let tools = self.list_tools();
                let tool_values: Vec<Value> = tools
                    .into_iter()
                    .map(|t| serde_json::to_value(t).unwrap_or(Value::Null))
                    .collect();
                Ok(serde_json::json!({ "tools": tool_values }))
            }

            "tools/call" => {
                let params = params.ok_or_else(|| JsonRpcError {
                    code: rpc_codes::INVALID_PARAMS,
                    message: "tools/call requires params".to_string(),
                    data: None,
                })?;

                let tool_name =
                    params
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(|| JsonRpcError {
                            code: rpc_codes::INVALID_PARAMS,
                            message: "Missing required param 'name'".to_string(),
                            data: None,
                        })?;

                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));

                let result = self.dispatch_tool_call(tool_name, &args)?;

                // Wrap in MCP content format.
                Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result)
                            .unwrap_or_else(|_| result.to_string())
                    }]
                }))
            }

            "notifications/initialized" => {
                // Should not reach here (filtered out in handle_request),
                // but handle gracefully.
                Ok(Value::Null)
            }

            _ => Err(JsonRpcError {
                code: rpc_codes::METHOD_NOT_FOUND,
                message: format!("Method not found: '{method}'"),
                data: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn server_with_workspace(dir: &TempDir) -> McpServer {
        let graph_dir = dir.path().join(".duumbi").join("graph");
        std::fs::create_dir_all(&graph_dir).expect("create graph dir");
        McpServer::new(dir.path().to_path_buf())
    }

    fn make_request(method: &str, id: Value, params: Option<Value>) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: method.to_string(),
            params,
        }
    }

    // -----------------------------------------------------------------------
    // initialize
    // -----------------------------------------------------------------------

    #[test]
    fn handle_initialize_returns_server_info() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let req = make_request("initialize", serde_json::json!(1), None);

        let resp = server
            .handle_request(&req)
            .expect("should produce response");
        assert!(resp.error.is_none(), "should not have error");
        let result = resp.result.expect("should have result");
        assert_eq!(result["serverInfo"]["name"], "duumbi-mcp");
        assert!(result["capabilities"]["tools"].is_object());
    }

    // -----------------------------------------------------------------------
    // tools/list
    // -----------------------------------------------------------------------

    #[test]
    fn handle_tools_list_returns_all_tools() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let req = make_request("tools/list", serde_json::json!(2), None);

        let resp = server
            .handle_request(&req)
            .expect("should produce response");
        assert!(resp.error.is_none(), "should not have error");
        let result = resp.result.expect("should have result");
        let tools = result["tools"].as_array().expect("tools array");

        let expected_names = [
            "mcp_capability_status",
            "graph_query",
            "graph_mutate",
            "graph_validate",
            "graph_describe",
            "build_compile",
            "build_run",
            "deps_search",
            "deps_install",
            "intent_create",
            "intent_execute",
            "model_access_summary",
            "model_performance_summary",
            "model_telemetry_health",
            "rewrite_list_rules",
            "rewrite_preview",
            "rewrite_apply",
        ];

        for name in &expected_names {
            assert!(
                tools.iter().any(|t| t["name"] == *name),
                "tool '{name}' should be in the list"
            );
        }
    }

    #[test]
    fn tools_list_includes_duumbi_safety_metadata() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let tools = server.list_tools();

        let status = tools
            .iter()
            .find(|tool| tool.name == "mcp_capability_status")
            .expect("status tool exists");
        assert_eq!(status.metadata.safety, capability::ToolSafety::ReadOnly);
        assert!(!status.metadata.approval_required);
        assert_eq!(status.input_schema["additionalProperties"], false);

        let graph_mutate = tools
            .iter()
            .find(|tool| tool.name == "graph_mutate")
            .expect("graph_mutate exists");
        assert_eq!(
            graph_mutate.metadata.safety,
            capability::ToolSafety::TrustedImmediateWrite
        );
        assert!(
            graph_mutate
                .metadata
                .writes
                .iter()
                .any(|path| path == ".duumbi/graph/main.jsonld")
        );

        let build_compile = tools
            .iter()
            .find(|tool| tool.name == "build_compile")
            .expect("build_compile exists");
        assert!(
            build_compile.metadata.unavailable_reason.is_some(),
            "stubbed tool must expose structured unavailable reason"
        );
    }

    #[test]
    fn all_tool_definitions_have_valid_input_schema() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let tools = server.list_tools();

        for tool in &tools {
            assert!(!tool.name.is_empty(), "tool name should not be empty");
            assert!(
                !tool.description.is_empty(),
                "tool '{}' description should not be empty",
                tool.name
            );
            assert_eq!(
                tool.input_schema["type"], "object",
                "tool '{}' inputSchema should have type: object",
                tool.name
            );
        }
    }

    #[test]
    fn model_telemetry_tool_descriptions_are_read_only() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let tools = server.list_tools();

        for name in [
            "model_access_summary",
            "model_performance_summary",
            "model_telemetry_health",
        ] {
            let tool = tools
                .iter()
                .find(|tool| tool.name == name)
                .expect("model telemetry tool exists");
            let description = tool.description.to_lowercase();
            assert!(
                description.contains("read-only"),
                "{name} must identify itself as read-only"
            );
            assert!(
                description.contains("model-access")
                    || description.contains("model-performance")
                    || description.contains("telemetry stores"),
                "{name} must identify the telemetry source"
            );
            assert_eq!(tool.input_schema["additionalProperties"], false);
        }
    }

    // -----------------------------------------------------------------------
    // tools/call
    // -----------------------------------------------------------------------

    #[test]
    fn tools_call_missing_params_returns_error() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let req = make_request("tools/call", serde_json::json!(3), None);

        let resp = server.handle_request(&req).expect("response");
        assert!(
            resp.error.is_some(),
            "should have error when params missing"
        );
    }

    #[test]
    fn tools_call_missing_name_returns_error() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let req = make_request(
            "tools/call",
            serde_json::json!(4),
            Some(serde_json::json!({ "arguments": {} })),
        );

        let resp = server.handle_request(&req).expect("response");
        assert!(resp.error.is_some(), "should have error when name missing");
    }

    #[test]
    fn tools_call_unknown_tool_returns_method_not_found() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let req = make_request(
            "tools/call",
            serde_json::json!(5),
            Some(serde_json::json!({ "name": "nonexistent_tool", "arguments": {} })),
        );

        let resp = server.handle_request(&req).expect("response");
        let err = resp.error.expect("should have error");
        assert_eq!(err.code, rpc_codes::METHOD_NOT_FOUND);
        assert!(err.message.contains("nonexistent_tool"));
    }

    #[test]
    fn tools_call_build_compile_returns_error_content() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let req = make_request(
            "tools/call",
            serde_json::json!(6),
            Some(serde_json::json!({ "name": "build_compile", "arguments": {} })),
        );

        let resp = server.handle_request(&req).expect("response");
        // build_compile is a stub that returns an error via JsonRpcError
        assert!(
            resp.error.is_some(),
            "build_compile stub should return an RPC error"
        );
        let err = resp.error.expect("error");
        let data = err.data.expect("structured error data");
        assert_eq!(data["code"], "mcp.build");
        assert_eq!(data["category"], "build");
        assert_eq!(data["source"]["tool"], "build_compile");
        assert!(
            data["suggestedRepairs"]
                .as_array()
                .expect("repair categories")
                .iter()
                .any(|repair| repair == "build")
        );
    }

    #[test]
    fn tools_call_capability_status_is_read_only_and_agent_facing() {
        let (dir, server) = server_with_graph();
        let graph_path = dir.path().join(".duumbi/graph/main.jsonld");
        let before = std::fs::read(&graph_path).expect("read graph before");
        let req = make_request(
            "tools/call",
            serde_json::json!(10),
            Some(serde_json::json!({
                "name": "mcp_capability_status",
                "arguments": {}
            })),
        );

        let resp = server.handle_request(&req).expect("response");
        assert!(
            resp.error.is_none(),
            "mcp_capability_status should succeed: {:?}",
            resp.error
        );
        assert_eq!(
            std::fs::read(&graph_path).expect("read graph after"),
            before,
            "status tool must not mutate graph"
        );
        let result = resp.result.expect("result");
        let text = result["content"][0]["text"].as_str().expect("text content");
        let status: Value = serde_json::from_str(text).expect("status JSON");
        assert_eq!(status["status"], "success");
        assert_eq!(status["workspace"]["duumbiInitialized"], true);
        assert!(
            status["tools"]
                .as_array()
                .expect("tools")
                .iter()
                .any(|tool| tool["name"] == "mcp_capability_status"
                    && tool["duumbi"]["safety"] == "read_only")
        );
        assert!(
            status["capabilities"]["unavailableTools"]
                .as_array()
                .expect("unavailable tools")
                .iter()
                .any(|tool| tool["name"] == "build_compile")
        );
    }

    // -----------------------------------------------------------------------
    // Unknown method
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_method_returns_method_not_found() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let req = make_request("some/unknown/method", serde_json::json!(7), None);

        let resp = server.handle_request(&req).expect("response");
        let err = resp.error.expect("should have error");
        assert_eq!(err.code, rpc_codes::METHOD_NOT_FOUND);
    }

    // -----------------------------------------------------------------------
    // Notifications (no response)
    // -----------------------------------------------------------------------

    #[test]
    fn notification_returns_none() {
        let dir = TempDir::new().expect("tempdir");
        let server = server_with_workspace(&dir);
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        };

        let resp = server.handle_request(&notification);
        assert!(resp.is_none(), "notification should produce no response");
    }

    // -----------------------------------------------------------------------
    // Graph query via tools/call
    // -----------------------------------------------------------------------

    const SIMPLE_GRAPH: &str = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module",
        "@id": "duumbi:test",
        "duumbi:name": "test",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:test/main",
            "duumbi:name": "main",
            "duumbi:params": [],
            "duumbi:returnType": "void",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [{
                    "@type": "duumbi:Return",
                    "@id": "duumbi:test/main/entry/0",
                    "duumbi:operands": []
                }]
            }]
        }]
    }"#;

    fn server_with_graph() -> (TempDir, McpServer) {
        let dir = TempDir::new().expect("tempdir");
        let graph_dir = dir.path().join(".duumbi").join("graph");
        std::fs::create_dir_all(&graph_dir).expect("create graph dir");
        std::fs::write(graph_dir.join("main.jsonld"), SIMPLE_GRAPH).expect("write graph");
        let server = McpServer::new(dir.path().to_path_buf());
        (dir, server)
    }

    #[test]
    fn tools_call_graph_validate_on_valid_graph() {
        let (_dir, server) = server_with_graph();
        let req = make_request(
            "tools/call",
            serde_json::json!(8),
            Some(serde_json::json!({
                "name": "graph_validate",
                "arguments": {}
            })),
        );

        let resp = server.handle_request(&req).expect("response");
        assert!(
            resp.error.is_none(),
            "graph_validate should succeed: {:?}",
            resp.error
        );
        let result = resp.result.expect("result");
        let text = result["content"][0]["text"].as_str().expect("text content");
        assert!(
            text.contains("\"valid\""),
            "response should contain 'valid' field"
        );
    }

    #[test]
    fn tools_call_graph_query_by_type() {
        let (_dir, server) = server_with_graph();
        let req = make_request(
            "tools/call",
            serde_json::json!(9),
            Some(serde_json::json!({
                "name": "graph_query",
                "arguments": { "type_filter": "duumbi:Function" }
            })),
        );

        let resp = server.handle_request(&req).expect("response");
        assert!(resp.error.is_none(), "graph_query should succeed");
        let result = resp.result.expect("result");
        let text = result["content"][0]["text"].as_str().expect("text content");
        assert!(
            text.contains("duumbi:Function"),
            "response should contain matched function node"
        );
    }
}
