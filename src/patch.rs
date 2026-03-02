//! GraphPatch format and all-or-nothing applicator.
//!
//! Defines the atomic mutation operations that the AI agent proposes.
//! `apply_patch` works on the raw JSON-LD `serde_json::Value`, clones it,
//! applies every operation, and returns the modified value on success.
//! If any operation fails the original value is unchanged.
//!
//! After a successful `apply_patch`, the caller is responsible for
//! running the full parse → build → validate pipeline before committing
//! (writing) the result to disk.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when applying a [`GraphPatch`].
#[derive(Debug, Error)]
pub enum PatchError {
    /// The requested function `@id` does not exist in the module.
    #[error("Function not found: '{0}'")]
    FunctionNotFound(String),

    /// The requested block `@id` does not exist in any function.
    #[error("Block not found: '{0}'")]
    BlockNotFound(String),

    /// The requested node `@id` (op, block, or function) was not found.
    #[error("Node not found: '{0}'")]
    NodeNotFound(String),

    /// The JSON-LD structure is missing an expected field or array.
    #[error("Invalid JSON-LD structure: {0}")]
    InvalidStructure(String),
}

/// A single atomic mutation on the JSON-LD source.
///
/// The `"kind"` tag is used for serde serialization / LLM tool call
/// deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PatchOp {
    /// Append a new function to `duumbi:functions`.
    AddFunction {
        /// Complete JSON-LD object for the new function.
        function: serde_json::Value,
    },

    /// Append a new block to a function's `duumbi:blocks`.
    AddBlock {
        /// The `@id` of the target function.
        function_id: String,
        /// Complete JSON-LD object for the new block.
        block: serde_json::Value,
    },

    /// Append a new op to a block's `duumbi:ops`.
    AddOp {
        /// The `@id` of the target block.
        block_id: String,
        /// Complete JSON-LD object for the new op.
        op: serde_json::Value,
    },

    /// Set a field value on any node (op, block, or function) by `@id`.
    ModifyOp {
        /// The `@id` of the node to modify.
        node_id: String,
        /// JSON-LD field name to set (e.g. `"duumbi:value"`).
        field: String,
        /// New value.
        value: serde_json::Value,
    },

    /// Remove a node (op, block, or function) from the graph by `@id`.
    RemoveNode {
        /// The `@id` to remove.
        node_id: String,
    },

    /// Set an operand reference on a node (shorthand for `ModifyOp` with
    /// `{"@id": target_id}` as value).
    SetEdge {
        /// The `@id` of the node whose operand to update.
        node_id: String,
        /// The operand field (e.g. `"duumbi:left"`, `"duumbi:operand"`).
        field: String,
        /// The `@id` of the target node.
        target_id: String,
    },

    /// Replace the entire ops list of a block with a new list.
    ///
    /// Preferred over multiple `RemoveNode` + `AddOp` calls when rewriting a
    /// block body (e.g. changing `Add` to `Call`). Single atomic operation
    /// that cannot leave the block in a partial state.
    ReplaceBlock {
        /// The `@id` of the target block.
        block_id: String,
        /// Complete new ops list (array of JSON-LD op objects).
        /// Must end with a Return or Branch op.
        ops: Vec<serde_json::Value>,
    },
}

/// A sequence of patch operations to apply in order.
///
/// Produced by the AI agent and applied atomically via [`apply_patch`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPatch {
    /// Ordered list of atomic operations.
    pub ops: Vec<PatchOp>,
}

/// Applies a [`GraphPatch`] to a JSON-LD module value.
///
/// The source is cloned; operations are applied to the clone. If every
/// operation succeeds the modified clone is returned. If any operation
/// fails the source is left unchanged and an error is returned.
///
/// # Errors
///
/// Returns [`PatchError`] if a referenced `@id` is not found or the
/// JSON-LD structure is missing expected fields.
pub fn apply_patch(
    source: &serde_json::Value,
    patch: &GraphPatch,
) -> Result<serde_json::Value, PatchError> {
    let mut doc = source.clone();
    for op in &patch.ops {
        apply_op(&mut doc, op)?;
    }
    Ok(doc)
}

// ---------------------------------------------------------------------------
// Internal applicators
// ---------------------------------------------------------------------------

fn apply_op(doc: &mut serde_json::Value, op: &PatchOp) -> Result<(), PatchError> {
    match op {
        PatchOp::AddFunction { function } => {
            functions_array_mut(doc)?.push(function.clone());
        }
        PatchOp::AddBlock { function_id, block } => {
            let func = find_function_mut(doc, function_id)?;
            blocks_array_of_mut(func, function_id)?.push(block.clone());
        }
        PatchOp::AddOp { block_id, op } => {
            let block = find_block_mut(doc, block_id)?;
            ops_array_of_mut(block, block_id)?.push(op.clone());
        }
        PatchOp::ModifyOp {
            node_id,
            field,
            value,
        } => {
            if !modify_node(doc, node_id, field, value) {
                return Err(PatchError::NodeNotFound(node_id.clone()));
            }
        }
        PatchOp::RemoveNode { node_id } => {
            if !remove_node(doc, node_id) {
                return Err(PatchError::NodeNotFound(node_id.clone()));
            }
        }
        PatchOp::SetEdge {
            node_id,
            field,
            target_id,
        } => {
            let edge_value = serde_json::json!({ "@id": target_id });
            if !modify_node(doc, node_id, field, &edge_value) {
                return Err(PatchError::NodeNotFound(node_id.clone()));
            }
        }
        PatchOp::ReplaceBlock { block_id, ops } => {
            let block = find_block_mut(doc, block_id)?;
            block["duumbi:ops"] = serde_json::Value::Array(ops.clone());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Array accessors (return mutable references)
// ---------------------------------------------------------------------------

fn functions_array_mut(
    doc: &mut serde_json::Value,
) -> Result<&mut Vec<serde_json::Value>, PatchError> {
    doc.get_mut("duumbi:functions")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| {
            PatchError::InvalidStructure("Module is missing 'duumbi:functions' array".to_string())
        })
}

fn blocks_array_of_mut<'a>(
    func: &'a mut serde_json::Value,
    func_id: &str,
) -> Result<&'a mut Vec<serde_json::Value>, PatchError> {
    func.get_mut("duumbi:blocks")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| {
            PatchError::InvalidStructure(format!(
                "Function '{func_id}' is missing 'duumbi:blocks' array"
            ))
        })
}

fn ops_array_of_mut<'a>(
    block: &'a mut serde_json::Value,
    block_id: &str,
) -> Result<&'a mut Vec<serde_json::Value>, PatchError> {
    block
        .get_mut("duumbi:ops")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| {
            PatchError::InvalidStructure(format!(
                "Block '{block_id}' is missing 'duumbi:ops' array"
            ))
        })
}

// ---------------------------------------------------------------------------
// Node finders (return mutable references to nodes by @id)
// ---------------------------------------------------------------------------

fn find_function_mut<'a>(
    doc: &'a mut serde_json::Value,
    id: &str,
) -> Result<&'a mut serde_json::Value, PatchError> {
    functions_array_mut(doc)?
        .iter_mut()
        .find(|f| node_id_eq(f, id))
        .ok_or_else(|| PatchError::FunctionNotFound(id.to_string()))
}

fn find_block_mut<'a>(
    doc: &'a mut serde_json::Value,
    id: &str,
) -> Result<&'a mut serde_json::Value, PatchError> {
    let functions = functions_array_mut(doc)?;
    for func in functions.iter_mut() {
        if let Some(blocks) = func.get_mut("duumbi:blocks").and_then(|v| v.as_array_mut())
            && let Some(block) = blocks.iter_mut().find(|b| node_id_eq(b, id))
        {
            return Ok(block);
        }
    }
    Err(PatchError::BlockNotFound(id.to_string()))
}

// ---------------------------------------------------------------------------
// In-place mutation helpers
// ---------------------------------------------------------------------------

/// Sets `doc[field] = value` if `doc["@id"] == id`, or recurses into
/// functions → blocks → ops. Returns `true` if the node was found.
fn modify_node(
    doc: &mut serde_json::Value,
    id: &str,
    field: &str,
    value: &serde_json::Value,
) -> bool {
    if node_id_eq(doc, id) {
        doc[field] = value.clone();
        return true;
    }

    // Search in functions
    if let Some(functions) = doc
        .get_mut("duumbi:functions")
        .and_then(|v| v.as_array_mut())
    {
        for func in functions.iter_mut() {
            if modify_node(func, id, field, value) {
                return true;
            }
        }
    }

    // Search in blocks
    if let Some(blocks) = doc.get_mut("duumbi:blocks").and_then(|v| v.as_array_mut()) {
        for block in blocks.iter_mut() {
            if modify_node(block, id, field, value) {
                return true;
            }
        }
    }

    // Search in ops
    if let Some(ops) = doc.get_mut("duumbi:ops").and_then(|v| v.as_array_mut()) {
        for op in ops.iter_mut() {
            if modify_node(op, id, field, value) {
                return true;
            }
        }
    }

    false
}

/// Removes the node with `@id == id` from the module. Returns `true` if found.
fn remove_node(doc: &mut serde_json::Value, id: &str) -> bool {
    // Try removing from functions array
    if let Some(functions) = doc
        .get_mut("duumbi:functions")
        .and_then(|v| v.as_array_mut())
    {
        let before = functions.len();
        functions.retain(|f| !node_id_eq(f, id));
        if functions.len() < before {
            return true;
        }

        // Try removing from blocks or ops inside each function
        for func in functions.iter_mut() {
            if let Some(blocks) = func.get_mut("duumbi:blocks").and_then(|v| v.as_array_mut()) {
                let before = blocks.len();
                blocks.retain(|b| !node_id_eq(b, id));
                if blocks.len() < before {
                    return true;
                }

                for block in blocks.iter_mut() {
                    if let Some(ops) = block.get_mut("duumbi:ops").and_then(|v| v.as_array_mut()) {
                        let before = ops.len();
                        ops.retain(|o| !node_id_eq(o, id));
                        if ops.len() < before {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Returns `true` if `node["@id"]` equals `id`.
fn node_id_eq(node: &serde_json::Value, id: &str) -> bool {
    node.get("@id").and_then(|v| v.as_str()) == Some(id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Minimal module fixture: one function, one block, two ops.
    fn minimal_module() -> serde_json::Value {
        json!({
            "@type": "duumbi:Module",
            "@id": "duumbi:main",
            "duumbi:name": "main",
            "duumbi:functions": [
                {
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "i64",
                    "duumbi:blocks": [
                        {
                            "@type": "duumbi:Block",
                            "@id": "duumbi:main/main/entry",
                            "duumbi:label": "entry",
                            "duumbi:ops": [
                                {
                                    "@type": "duumbi:Const",
                                    "@id": "duumbi:main/main/entry/0",
                                    "duumbi:value": 3,
                                    "duumbi:resultType": "i64"
                                },
                                {
                                    "@type": "duumbi:Return",
                                    "@id": "duumbi:main/main/entry/1",
                                    "duumbi:operand": { "@id": "duumbi:main/main/entry/0" }
                                }
                            ]
                        }
                    ]
                }
            ]
        })
    }

    #[test]
    fn empty_patch_returns_unchanged_doc() {
        let source = minimal_module();
        let patch = GraphPatch { ops: vec![] };
        let result = apply_patch(&source, &patch).expect("empty patch must succeed");
        assert_eq!(result, source);
    }

    #[test]
    fn add_function_appends_to_functions_array() {
        let source = minimal_module();
        let new_func = json!({
            "@type": "duumbi:Function",
            "@id": "duumbi:main/helper",
            "duumbi:name": "helper",
            "duumbi:returnType": "i64",
            "duumbi:blocks": []
        });
        let patch = GraphPatch {
            ops: vec![PatchOp::AddFunction {
                function: new_func.clone(),
            }],
        };
        let result = apply_patch(&source, &patch).expect("add_function must succeed");
        let functions = result["duumbi:functions"]
            .as_array()
            .expect("functions must be array");
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[1]["@id"], "duumbi:main/helper");
    }

    #[test]
    fn add_block_appends_to_function() {
        let source = minimal_module();
        let new_block = json!({
            "@type": "duumbi:Block",
            "@id": "duumbi:main/main/exit",
            "duumbi:label": "exit",
            "duumbi:ops": []
        });
        let patch = GraphPatch {
            ops: vec![PatchOp::AddBlock {
                function_id: "duumbi:main/main".to_string(),
                block: new_block,
            }],
        };
        let result = apply_patch(&source, &patch).expect("add_block must succeed");
        let blocks = result["duumbi:functions"][0]["duumbi:blocks"]
            .as_array()
            .expect("blocks must be array");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1]["@id"], "duumbi:main/main/exit");
    }

    #[test]
    fn add_op_appends_to_block() {
        let source = minimal_module();
        let new_op = json!({
            "@type": "duumbi:Const",
            "@id": "duumbi:main/main/entry/2",
            "duumbi:value": 99,
            "duumbi:resultType": "i64"
        });
        let patch = GraphPatch {
            ops: vec![PatchOp::AddOp {
                block_id: "duumbi:main/main/entry".to_string(),
                op: new_op,
            }],
        };
        let result = apply_patch(&source, &patch).expect("add_op must succeed");
        let ops = result["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"]
            .as_array()
            .expect("ops must be array");
        assert_eq!(ops.len(), 3);
        assert_eq!(ops[2]["duumbi:value"], 99);
    }

    #[test]
    fn modify_op_changes_field_on_op() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/0".to_string(),
                field: "duumbi:value".to_string(),
                value: json!(42),
            }],
        };
        let result = apply_patch(&source, &patch).expect("modify_op must succeed");
        let val =
            result["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][0]["duumbi:value"]
                .clone();
        assert_eq!(val, json!(42));
    }

    #[test]
    fn set_edge_updates_operand_reference() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::SetEdge {
                node_id: "duumbi:main/main/entry/1".to_string(),
                field: "duumbi:operand".to_string(),
                target_id: "duumbi:main/main/entry/99".to_string(),
            }],
        };
        let result = apply_patch(&source, &patch).expect("set_edge must succeed");
        let operand =
            &result["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][1]["duumbi:operand"];
        assert_eq!(operand["@id"], "duumbi:main/main/entry/99");
    }

    #[test]
    fn remove_node_removes_op() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::RemoveNode {
                node_id: "duumbi:main/main/entry/0".to_string(),
            }],
        };
        let result = apply_patch(&source, &patch).expect("remove_node must succeed");
        let ops = result["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"]
            .as_array()
            .expect("ops must be array");
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0]["@id"], "duumbi:main/main/entry/1");
    }

    #[test]
    fn remove_node_removes_block() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::RemoveNode {
                node_id: "duumbi:main/main/entry".to_string(),
            }],
        };
        let result = apply_patch(&source, &patch).expect("remove block must succeed");
        let blocks = result["duumbi:functions"][0]["duumbi:blocks"]
            .as_array()
            .expect("blocks must be array");
        assert!(blocks.is_empty());
    }

    #[test]
    fn remove_node_removes_function() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::RemoveNode {
                node_id: "duumbi:main/main".to_string(),
            }],
        };
        let result = apply_patch(&source, &patch).expect("remove function must succeed");
        let functions = result["duumbi:functions"]
            .as_array()
            .expect("functions must be array");
        assert!(functions.is_empty());
    }

    #[test]
    fn modify_op_on_missing_id_returns_node_not_found() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/999".to_string(),
                field: "duumbi:value".to_string(),
                value: json!(1),
            }],
        };
        let err = apply_patch(&source, &patch).expect_err("must error on missing id");
        assert!(matches!(err, PatchError::NodeNotFound(_)));
    }

    #[test]
    fn add_block_to_missing_function_returns_function_not_found() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::AddBlock {
                function_id: "duumbi:main/nonexistent".to_string(),
                block: json!({}),
            }],
        };
        let err = apply_patch(&source, &patch).expect_err("must error on missing function");
        assert!(matches!(err, PatchError::FunctionNotFound(_)));
    }

    #[test]
    fn add_op_to_missing_block_returns_block_not_found() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::AddOp {
                block_id: "duumbi:main/main/nonexistent".to_string(),
                op: json!({}),
            }],
        };
        let err = apply_patch(&source, &patch).expect_err("must error on missing block");
        assert!(matches!(err, PatchError::BlockNotFound(_)));
    }

    #[test]
    fn patch_is_all_or_nothing_on_error() {
        let source = minimal_module();
        // First op is valid, second op references a missing id
        let patch = GraphPatch {
            ops: vec![
                PatchOp::ModifyOp {
                    node_id: "duumbi:main/main/entry/0".to_string(),
                    field: "duumbi:value".to_string(),
                    value: json!(999),
                },
                PatchOp::ModifyOp {
                    node_id: "duumbi:main/main/entry/MISSING".to_string(),
                    field: "duumbi:value".to_string(),
                    value: json!(1),
                },
            ],
        };
        let err = apply_patch(&source, &patch).expect_err("second op must fail");
        assert!(matches!(err, PatchError::NodeNotFound(_)));
        // Source is unchanged (we cloned at the start)
        let original_val =
            source["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][0]["duumbi:value"]
                .clone();
        assert_eq!(original_val, json!(3));
    }

    #[test]
    fn patch_op_serializes_with_op_tag() {
        let op = PatchOp::ModifyOp {
            node_id: "id".to_string(),
            field: "f".to_string(),
            value: json!(1),
        };
        let s = serde_json::to_string(&op).expect("must serialize");
        let parsed: serde_json::Value = serde_json::from_str(&s).expect("must parse");
        assert_eq!(parsed["kind"], "modify_op");
    }

    #[test]
    fn replace_block_replaces_ops_array() {
        let source = minimal_module();
        let new_ops = vec![
            json!({
                "@type": "duumbi:Const",
                "@id": "duumbi:main/main/entry/0",
                "duumbi:value": 99,
                "duumbi:resultType": "i64"
            }),
            json!({
                "@type": "duumbi:Return",
                "@id": "duumbi:main/main/entry/1",
                "duumbi:operand": { "@id": "duumbi:main/main/entry/0" }
            }),
        ];
        let patch = GraphPatch {
            ops: vec![PatchOp::ReplaceBlock {
                block_id: "duumbi:main/main/entry".to_string(),
                ops: new_ops,
            }],
        };
        let result = apply_patch(&source, &patch).expect("replace_block must succeed");
        let ops = result["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"]
            .as_array()
            .expect("ops must be array");
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0]["duumbi:value"], 99);
    }

    #[test]
    fn replace_block_missing_id_returns_block_not_found() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![PatchOp::ReplaceBlock {
                block_id: "duumbi:main/main/nonexistent".to_string(),
                ops: vec![],
            }],
        };
        let err = apply_patch(&source, &patch).expect_err("must fail on missing block");
        assert!(matches!(err, PatchError::BlockNotFound(_)));
    }

    #[test]
    fn graph_patch_round_trips_json() {
        let patch = GraphPatch {
            ops: vec![
                PatchOp::AddOp {
                    block_id: "b".to_string(),
                    op: json!({"@type": "duumbi:Return"}),
                },
                PatchOp::RemoveNode {
                    node_id: "n".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&patch).expect("must serialize");
        let back: GraphPatch = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(back.ops.len(), 2);
    }
}
