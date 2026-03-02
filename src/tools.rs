//! LLM tool schema definitions for AI-assisted graph mutation.
//!
//! Defines the JSON schema for 6 graph-mutation tools that are sent to the
//! LLM (both Anthropic and OpenAI). The LLM responds with tool calls whose
//! arguments are deserialized into [`PatchOp`] values.
//!
//! Each tool maps 1-to-1 to a [`crate::patch::PatchOp`] variant. The
//! `input_schema` / `parameters` field is the JSON Schema the LLM sees.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::patch::PatchOp;

// ---------------------------------------------------------------------------
// Tool descriptor types
// ---------------------------------------------------------------------------

/// A tool definition in Anthropic format (`tool_use` API).
#[allow(dead_code)] // Used in Issue #29 Anthropic provider
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicTool {
    /// Tool name (matches PatchOp kind).
    pub name: String,
    /// Human-readable description for the LLM.
    pub description: String,
    /// JSON Schema for the tool's input object.
    pub input_schema: serde_json::Value,
}

/// A tool definition in OpenAI format (function calling API).
#[allow(dead_code)] // Used in Issue #30 OpenAI provider
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiTool {
    /// Always `"function"` for OpenAI function-calling.
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition.
    pub function: OpenAiFunction,
}

/// The function sub-object in an OpenAI tool definition.
#[allow(dead_code)] // Used in Issue #30 OpenAI provider
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiFunction {
    /// Function name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the function parameters.
    pub parameters: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Tool call input types (deserialized from LLM responses)
// ---------------------------------------------------------------------------

/// A tool call as returned by the Anthropic API.
#[allow(dead_code)] // Used in Issue #29 Anthropic provider
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicToolCall {
    /// Tool name.
    pub name: String,
    /// Tool input arguments (JSON object).
    pub input: serde_json::Value,
}

/// A tool call as returned by the OpenAI API.
#[allow(dead_code)] // Used in Issue #30 OpenAI provider
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiToolCall {
    /// Always `"function"`.
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function call details.
    pub function: OpenAiFunctionCall,
}

/// The function call sub-object in an OpenAI tool call.
#[allow(dead_code)] // Used in Issue #30 OpenAI provider
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiFunctionCall {
    /// Function name.
    pub name: String,
    /// Arguments as a JSON string (must be parsed separately).
    pub arguments: String,
}

// ---------------------------------------------------------------------------
// Canonical tool list
// ---------------------------------------------------------------------------

/// Returns all 6 graph-mutation tools in Anthropic format.
///
/// The returned list is passed directly to the Anthropic tool_use API.
#[allow(dead_code)] // Used in Issue #29 Anthropic provider
#[must_use]
pub fn anthropic_tools() -> Vec<AnthropicTool> {
    TOOL_SPECS
        .iter()
        .map(|spec| AnthropicTool {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            input_schema: spec.schema.clone(),
        })
        .collect()
}

/// Returns all 6 graph-mutation tools in OpenAI format.
///
/// The returned list is passed directly to the OpenAI function-calling API.
#[allow(dead_code)] // Used in Issue #30 OpenAI provider
#[must_use]
pub fn openai_tools() -> Vec<OpenAiTool> {
    TOOL_SPECS
        .iter()
        .map(|spec| OpenAiTool {
            tool_type: "function".to_string(),
            function: OpenAiFunction {
                name: spec.name.to_string(),
                description: spec.description.to_string(),
                parameters: spec.schema.clone(),
            },
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tool call deserialization
// ---------------------------------------------------------------------------

/// Converts an Anthropic tool call into a [`PatchOp`].
///
/// Expects `call.name` to be one of the 6 canonical tool names and
/// `call.input` to match the corresponding JSON schema.
///
/// # Errors
///
/// Returns a descriptive string if the name is unknown or the input does
/// not deserialize to the expected shape.
#[allow(dead_code)] // Used in Issue #29 Anthropic provider
pub fn patch_op_from_anthropic(call: &AnthropicToolCall) -> Result<PatchOp, String> {
    parse_tool_call(&call.name, &call.input)
}

/// Converts an OpenAI tool call into a [`PatchOp`].
///
/// Parses `call.function.arguments` as JSON, then delegates to the same
/// inner parser used for Anthropic calls.
///
/// # Errors
///
/// Returns a descriptive string if parsing fails or the tool name is unknown.
#[allow(dead_code)] // Used in Issue #30 OpenAI provider
pub fn patch_op_from_openai(call: &OpenAiToolCall) -> Result<PatchOp, String> {
    let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
        .map_err(|e| format!("Failed to parse tool call arguments: {e}"))?;
    parse_tool_call(&call.function.name, &args)
}

/// Common inner parser: maps tool name + JSON args to a [`PatchOp`].
fn parse_tool_call(name: &str, args: &serde_json::Value) -> Result<PatchOp, String> {
    // Inject the "kind" tag so serde's internally-tagged enum can deserialize
    let mut tagged = args.clone();
    tagged["kind"] = serde_json::Value::String(name.to_string());
    serde_json::from_value(tagged)
        .map_err(|e| format!("Failed to deserialize tool call '{name}': {e}"))
}

// ---------------------------------------------------------------------------
// Internal tool spec table
// ---------------------------------------------------------------------------

struct ToolSpec {
    name: &'static str,
    description: &'static str,
    schema: serde_json::Value,
}

/// All tool specs, initialized once at first use.
static TOOL_SPECS: std::sync::LazyLock<Vec<ToolSpec>> = std::sync::LazyLock::new(build_tool_specs);

fn build_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "add_function",
            description: "Add a COMPLETE function to the duumbi module. \
                The function object MUST include at least one block in duumbi:blocks, \
                and each block MUST include its ops in duumbi:ops. \
                Do NOT pass empty duumbi:blocks — that is invalid. \
                Include the full function body (blocks + ops) in this single call.",
            schema: json!({
                "type": "object",
                "required": ["function"],
                "properties": {
                    "function": {
                        "type": "object",
                        "description": "Complete JSON-LD duumbi:Function node with blocks and ops included.",
                        "required": ["@type", "@id", "duumbi:name", "duumbi:returnType", "duumbi:blocks"],
                        "properties": {
                            "@type": { "type": "string", "const": "duumbi:Function" },
                            "@id": { "type": "string", "description": "Unique node ID, e.g. 'duumbi:main/multiply'" },
                            "duumbi:name": { "type": "string" },
                            "duumbi:returnType": { "type": "string", "enum": ["i64", "f64", "bool", "void"] },
                            "duumbi:params": {
                                "type": "array",
                                "description": "Function parameters. Each: {\"duumbi:name\": \"x\", \"duumbi:paramType\": \"i64\"}"
                            },
                            "duumbi:blocks": {
                                "type": "array",
                                "minItems": 1,
                                "description": "Non-empty array of duumbi:Block nodes, each with duumbi:ops populated."
                            }
                        }
                    }
                }
            }),
        },
        ToolSpec {
            name: "add_block",
            description: "Add a new basic block to an existing function. \
                Blocks contain an ordered list of operations.",
            schema: json!({
                "type": "object",
                "required": ["function_id", "block"],
                "properties": {
                    "function_id": {
                        "type": "string",
                        "description": "The @id of the function to add the block to."
                    },
                    "block": {
                        "type": "object",
                        "description": "Complete JSON-LD duumbi:Block node.",
                        "required": ["@type", "@id", "duumbi:label", "duumbi:ops"],
                        "properties": {
                            "@type": { "type": "string", "const": "duumbi:Block" },
                            "@id": { "type": "string" },
                            "duumbi:label": { "type": "string" },
                            "duumbi:ops": { "type": "array" }
                        }
                    }
                }
            }),
        },
        ToolSpec {
            name: "add_op",
            description: "Append a new operation (instruction) to the end of a block's op list. \
                The op must have a valid duumbi @type (e.g. duumbi:Const, duumbi:Add, duumbi:Return).",
            schema: json!({
                "type": "object",
                "required": ["block_id", "op"],
                "properties": {
                    "block_id": {
                        "type": "string",
                        "description": "The @id of the block to append the op to."
                    },
                    "op": {
                        "type": "object",
                        "description": "Complete JSON-LD op node.",
                        "required": ["@type", "@id"],
                        "properties": {
                            "@type": { "type": "string" },
                            "@id": { "type": "string" },
                            "duumbi:value": {},
                            "duumbi:resultType": { "type": "string" },
                            "duumbi:left": { "type": "object" },
                            "duumbi:right": { "type": "object" },
                            "duumbi:operand": { "type": "object" },
                            "duumbi:condition": { "type": "object" },
                            "duumbi:trueBlock": { "type": "string" },
                            "duumbi:falseBlock": { "type": "string" },
                            "duumbi:function": { "type": "string" },
                            "duumbi:args": { "type": "array" },
                            "duumbi:variable": { "type": "string" },
                            "duumbi:operator": { "type": "string" }
                        }
                    }
                }
            }),
        },
        ToolSpec {
            name: "modify_op",
            description: "Change a single field value on any existing node (op, block, or function) \
                identified by its @id. Use this to update constants, return types, labels, etc.",
            schema: json!({
                "type": "object",
                "required": ["node_id", "field", "value"],
                "properties": {
                    "node_id": {
                        "type": "string",
                        "description": "The @id of the node to modify."
                    },
                    "field": {
                        "type": "string",
                        "description": "JSON-LD field name to update, e.g. 'duumbi:value' or 'duumbi:resultType'."
                    },
                    "value": {
                        "description": "New value to set for the field."
                    }
                }
            }),
        },
        ToolSpec {
            name: "remove_node",
            description: "Remove a node (op, block, or function) from the graph by its @id. \
                This also removes all ops within a removed block, and all blocks within a removed function.",
            schema: json!({
                "type": "object",
                "required": ["node_id"],
                "properties": {
                    "node_id": {
                        "type": "string",
                        "description": "The @id of the node to remove."
                    }
                }
            }),
        },
        ToolSpec {
            name: "set_edge",
            description: "Update an operand reference (edge) on an existing op. \
                Sets the given field to {\"@id\": target_id}. \
                Use this to rewire data-flow edges between nodes.",
            schema: json!({
                "type": "object",
                "required": ["node_id", "field", "target_id"],
                "properties": {
                    "node_id": {
                        "type": "string",
                        "description": "The @id of the op whose operand to update."
                    },
                    "field": {
                        "type": "string",
                        "description": "Operand field name, e.g. 'duumbi:left', 'duumbi:operand', 'duumbi:condition'."
                    },
                    "target_id": {
                        "type": "string",
                        "description": "The @id of the node to reference."
                    }
                }
            }),
        },
        ToolSpec {
            name: "replace_block",
            description: "Replace ALL ops in an existing block with a completely new ops list. \
                Use this when rewriting a function body (e.g. changing Add to Call, inserting \
                a Call before Return). PREFERRED over multiple remove_node + add_op calls — \
                one atomic operation that cannot leave the block half-built. \
                The new ops array MUST end with a Return or Branch op.",
            schema: json!({
                "type": "object",
                "required": ["block_id", "ops"],
                "properties": {
                    "block_id": {
                        "type": "string",
                        "description": "The @id of the block to rewrite, e.g. 'duumbi:main/main/entry'."
                    },
                    "ops": {
                        "type": "array",
                        "minItems": 1,
                        "description": "Complete new ops list. Must end with Return or Branch. \
                            Each element is a full JSON-LD op object with @type and @id."
                    }
                }
            }),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::PatchOp;

    #[test]
    fn anthropic_tools_returns_seven_tools() {
        let tools = anthropic_tools();
        assert_eq!(tools.len(), 7);
    }

    #[test]
    fn openai_tools_returns_seven_tools() {
        let tools = openai_tools();
        assert_eq!(tools.len(), 7);
        for t in &tools {
            assert_eq!(t.tool_type, "function");
        }
    }

    #[test]
    fn tool_names_match_patch_op_kinds() {
        let expected = [
            "add_function",
            "add_block",
            "add_op",
            "modify_op",
            "remove_node",
            "set_edge",
            "replace_block",
        ];
        let tools = anthropic_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, expected);
    }

    #[test]
    fn anthropic_tools_serialize_to_json() {
        let tools = anthropic_tools();
        let json = serde_json::to_value(&tools).expect("must serialize");
        assert!(json.is_array());
        let arr = json.as_array().expect("must be array");
        assert_eq!(arr[0]["name"], "add_function");
        assert!(arr[0]["input_schema"].is_object());
    }

    #[test]
    fn openai_tools_serialize_to_json() {
        let tools = openai_tools();
        let json = serde_json::to_value(&tools).expect("must serialize");
        let arr = json.as_array().expect("must be array");
        assert_eq!(arr[0]["type"], "function");
        assert_eq!(arr[0]["function"]["name"], "add_function");
    }

    #[test]
    fn patch_op_from_anthropic_add_op() {
        let call = AnthropicToolCall {
            name: "add_op".to_string(),
            input: serde_json::json!({
                "block_id": "duumbi:main/main/entry",
                "op": {
                    "@type": "duumbi:Return",
                    "@id": "duumbi:main/main/entry/5",
                    "duumbi:operand": { "@id": "duumbi:main/main/entry/4" }
                }
            }),
        };
        let op = patch_op_from_anthropic(&call).expect("must parse");
        assert!(matches!(op, PatchOp::AddOp { .. }));
    }

    #[test]
    fn patch_op_from_anthropic_modify_op() {
        let call = AnthropicToolCall {
            name: "modify_op".to_string(),
            input: serde_json::json!({
                "node_id": "duumbi:main/main/entry/0",
                "field": "duumbi:value",
                "value": 42
            }),
        };
        let op = patch_op_from_anthropic(&call).expect("must parse");
        assert!(matches!(op, PatchOp::ModifyOp { .. }));
    }

    #[test]
    fn patch_op_from_anthropic_remove_node() {
        let call = AnthropicToolCall {
            name: "remove_node".to_string(),
            input: serde_json::json!({ "node_id": "duumbi:main/main/entry/0" }),
        };
        let op = patch_op_from_anthropic(&call).expect("must parse");
        assert!(matches!(op, PatchOp::RemoveNode { .. }));
    }

    #[test]
    fn patch_op_from_anthropic_set_edge() {
        let call = AnthropicToolCall {
            name: "set_edge".to_string(),
            input: serde_json::json!({
                "node_id": "duumbi:main/main/entry/2",
                "field": "duumbi:left",
                "target_id": "duumbi:main/main/entry/0"
            }),
        };
        let op = patch_op_from_anthropic(&call).expect("must parse");
        assert!(matches!(op, PatchOp::SetEdge { .. }));
    }

    #[test]
    fn patch_op_from_anthropic_replace_block() {
        let call = AnthropicToolCall {
            name: "replace_block".to_string(),
            input: serde_json::json!({
                "block_id": "duumbi:main/main/entry",
                "ops": [
                    {
                        "@type": "duumbi:Const",
                        "@id": "duumbi:main/main/entry/0",
                        "duumbi:value": 42,
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Return",
                        "@id": "duumbi:main/main/entry/1",
                        "duumbi:operand": { "@id": "duumbi:main/main/entry/0" }
                    }
                ]
            }),
        };
        let op = patch_op_from_anthropic(&call).expect("must parse replace_block");
        assert!(matches!(op, PatchOp::ReplaceBlock { .. }));
    }

    #[test]
    fn patch_op_from_anthropic_unknown_tool_returns_error() {
        let call = AnthropicToolCall {
            name: "unknown_tool".to_string(),
            input: serde_json::json!({}),
        };
        let err = patch_op_from_anthropic(&call).expect_err("must fail on unknown tool");
        assert!(err.contains("unknown_tool"));
    }

    #[test]
    fn patch_op_from_openai_modify_op() {
        let call = OpenAiToolCall {
            call_type: "function".to_string(),
            function: OpenAiFunctionCall {
                name: "modify_op".to_string(),
                arguments: r#"{"node_id":"duumbi:x","field":"duumbi:value","value":7}"#.to_string(),
            },
        };
        let op = patch_op_from_openai(&call).expect("must parse");
        assert!(matches!(op, PatchOp::ModifyOp { .. }));
    }

    #[test]
    fn patch_op_from_openai_invalid_json_args_returns_error() {
        let call = OpenAiToolCall {
            call_type: "function".to_string(),
            function: OpenAiFunctionCall {
                name: "modify_op".to_string(),
                arguments: "not json".to_string(),
            },
        };
        let err = patch_op_from_openai(&call).expect_err("must fail on invalid JSON");
        assert!(err.contains("Failed to parse tool call arguments"));
    }

    #[test]
    fn each_tool_has_required_fields_in_schema() {
        let tools = anthropic_tools();
        for tool in &tools {
            assert!(
                !tool.name.is_empty(),
                "tool '{}' must have a name",
                tool.name
            );
            assert!(
                !tool.description.is_empty(),
                "tool '{}' must have a description",
                tool.name
            );
            assert!(
                tool.input_schema.get("type").is_some(),
                "tool '{}' schema must have 'type'",
                tool.name
            );
        }
    }
}
