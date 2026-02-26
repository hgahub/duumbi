//! JSON-LD parsing module.
//!
//! Parses `.jsonld` files into a typed AST representation suitable for
//! graph construction. Uses `serde_json` for JSON parsing and dispatches
//! on `@type` fields to construct typed `OpAst` nodes.

pub mod ast;

use ast::{BlockAst, FunctionAst, ModuleAst, NodeRef, OpAst};
use thiserror::Error;

use crate::errors::codes;
use crate::types::{BlockLabel, DuumbiType, FunctionName, ModuleName, NodeId, Op};

/// Errors that can occur during JSON-LD parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Malformed JSON input.
    #[error("[{code}] Invalid JSON: {source}")]
    Json {
        /// Error code for diagnostics.
        code: &'static str,
        /// The underlying serde_json error.
        #[source]
        source: serde_json::Error,
    },

    /// A required field is missing from a JSON-LD node.
    #[error("[{code}] Missing field '{field}' on node {node_id}")]
    MissingField {
        /// Error code for diagnostics.
        code: &'static str,
        /// The missing field name.
        field: String,
        /// The `@id` of the node, if known.
        node_id: String,
    },

    /// An unknown `@type` was encountered.
    #[error("[{code}] Unknown op type '{op_type}' on node {node_id}")]
    UnknownOp {
        /// Error code for diagnostics.
        code: &'static str,
        /// The unrecognized type string.
        op_type: String,
        /// The `@id` of the node, if known.
        node_id: String,
    },

    /// The JSON-LD structure is invalid (wrong type, missing container, etc).
    #[error("[{code}] Schema invalid: {message}")]
    SchemaInvalid {
        /// Error code for diagnostics.
        code: &'static str,
        /// Description of the structural problem.
        message: String,
    },
}

/// Parses a JSON-LD string into a typed module AST.
///
/// Expects the top-level object to have `@type: "duumbi:Module"`.
#[must_use = "parsing errors should be handled"]
pub fn parse_jsonld(input: &str) -> Result<ModuleAst, ParseError> {
    let value: serde_json::Value = serde_json::from_str(input).map_err(|e| ParseError::Json {
        code: codes::E009_SCHEMA_INVALID,
        source: e,
    })?;

    parse_module(&value)
}

fn get_str<'a>(
    obj: &'a serde_json::Value,
    field: &str,
    node_id: &str,
) -> Result<&'a str, ParseError> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ParseError::MissingField {
            code: codes::E003_MISSING_FIELD,
            field: field.to_string(),
            node_id: node_id.to_string(),
        })
}

fn get_array<'a>(
    obj: &'a serde_json::Value,
    field: &str,
    node_id: &str,
) -> Result<&'a Vec<serde_json::Value>, ParseError> {
    obj.get(field)
        .and_then(|v| v.as_array())
        .ok_or_else(|| ParseError::MissingField {
            code: codes::E003_MISSING_FIELD,
            field: field.to_string(),
            node_id: node_id.to_string(),
        })
}

fn parse_type_str(s: &str) -> Result<DuumbiType, ParseError> {
    match s {
        "i64" => Ok(DuumbiType::I64),
        "void" => Ok(DuumbiType::Void),
        other => Err(ParseError::SchemaInvalid {
            code: codes::E009_SCHEMA_INVALID,
            message: format!("Unknown type '{other}'"),
        }),
    }
}

fn parse_node_ref(
    obj: &serde_json::Value,
    field: &str,
    node_id: &str,
) -> Result<NodeRef, ParseError> {
    let ref_obj = obj.get(field).ok_or_else(|| ParseError::MissingField {
        code: codes::E003_MISSING_FIELD,
        field: field.to_string(),
        node_id: node_id.to_string(),
    })?;
    let id =
        ref_obj
            .get("@id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ParseError::MissingField {
                code: codes::E003_MISSING_FIELD,
                field: format!("{field}.@id"),
                node_id: node_id.to_string(),
            })?;
    Ok(NodeRef {
        id: NodeId(id.to_string()),
    })
}

fn parse_module(value: &serde_json::Value) -> Result<ModuleAst, ParseError> {
    let node_id_str = get_str(value, "@id", "<root>")?;
    let at_type = get_str(value, "@type", node_id_str)?;
    if at_type != "duumbi:Module" {
        return Err(ParseError::SchemaInvalid {
            code: codes::E009_SCHEMA_INVALID,
            message: format!("Expected @type 'duumbi:Module', got '{at_type}'"),
        });
    }

    let name = get_str(value, "duumbi:name", node_id_str)?;
    let functions_arr = get_array(value, "duumbi:functions", node_id_str)?;

    let mut functions = Vec::with_capacity(functions_arr.len());
    for func_val in functions_arr {
        functions.push(parse_function(func_val)?);
    }

    Ok(ModuleAst {
        id: NodeId(node_id_str.to_string()),
        name: ModuleName(name.to_string()),
        functions,
    })
}

fn parse_function(value: &serde_json::Value) -> Result<FunctionAst, ParseError> {
    let node_id_str = get_str(value, "@id", "<unknown function>")?;
    let at_type = get_str(value, "@type", node_id_str)?;
    if at_type != "duumbi:Function" {
        return Err(ParseError::SchemaInvalid {
            code: codes::E009_SCHEMA_INVALID,
            message: format!("Expected @type 'duumbi:Function', got '{at_type}'"),
        });
    }

    let name = get_str(value, "duumbi:name", node_id_str)?;
    let return_type_str = get_str(value, "duumbi:returnType", node_id_str)?;
    let return_type = parse_type_str(return_type_str)?;
    let blocks_arr = get_array(value, "duumbi:blocks", node_id_str)?;

    let mut blocks = Vec::with_capacity(blocks_arr.len());
    for block_val in blocks_arr {
        blocks.push(parse_block(block_val)?);
    }

    Ok(FunctionAst {
        id: NodeId(node_id_str.to_string()),
        name: FunctionName(name.to_string()),
        return_type,
        blocks,
    })
}

fn parse_block(value: &serde_json::Value) -> Result<BlockAst, ParseError> {
    let node_id_str = get_str(value, "@id", "<unknown block>")?;
    let at_type = get_str(value, "@type", node_id_str)?;
    if at_type != "duumbi:Block" {
        return Err(ParseError::SchemaInvalid {
            code: codes::E009_SCHEMA_INVALID,
            message: format!("Expected @type 'duumbi:Block', got '{at_type}'"),
        });
    }

    let label = get_str(value, "duumbi:label", node_id_str)?;
    let ops_arr = get_array(value, "duumbi:ops", node_id_str)?;

    let mut ops = Vec::with_capacity(ops_arr.len());
    for op_val in ops_arr {
        ops.push(parse_op(op_val)?);
    }

    Ok(BlockAst {
        id: NodeId(node_id_str.to_string()),
        label: BlockLabel(label.to_string()),
        ops,
    })
}

fn parse_op(value: &serde_json::Value) -> Result<OpAst, ParseError> {
    let node_id_str = get_str(value, "@id", "<unknown op>")?;
    let at_type = get_str(value, "@type", node_id_str)?;

    let result_type = value
        .get("duumbi:resultType")
        .and_then(|v| v.as_str())
        .map(parse_type_str)
        .transpose()?;

    match at_type {
        "duumbi:Const" => {
            let val = value
                .get("duumbi:value")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| ParseError::MissingField {
                    code: codes::E003_MISSING_FIELD,
                    field: "duumbi:value".to_string(),
                    node_id: node_id_str.to_string(),
                })?;
            Ok(OpAst {
                id: NodeId(node_id_str.to_string()),
                op: Op::Const(val),
                result_type,
                left: None,
                right: None,
                operand: None,
            })
        }
        "duumbi:Add" | "duumbi:Sub" | "duumbi:Mul" | "duumbi:Div" => {
            let left = parse_node_ref(value, "duumbi:left", node_id_str)?;
            let right = parse_node_ref(value, "duumbi:right", node_id_str)?;
            let op = match at_type {
                "duumbi:Add" => Op::Add,
                "duumbi:Sub" => Op::Sub,
                "duumbi:Mul" => Op::Mul,
                "duumbi:Div" => Op::Div,
                _ => unreachable!(),
            };
            Ok(OpAst {
                id: NodeId(node_id_str.to_string()),
                op,
                result_type,
                left: Some(left),
                right: Some(right),
                operand: None,
            })
        }
        "duumbi:Print" | "duumbi:Return" => {
            let operand = parse_node_ref(value, "duumbi:operand", node_id_str)?;
            let op = match at_type {
                "duumbi:Print" => Op::Print,
                "duumbi:Return" => Op::Return,
                _ => unreachable!(),
            };
            Ok(OpAst {
                id: NodeId(node_id_str.to_string()),
                op,
                result_type,
                left: None,
                right: None,
                operand: Some(operand),
            })
        }
        other => Err(ParseError::UnknownOp {
            code: codes::E002_UNKNOWN_OP,
            op_type: other.to_string(),
            node_id: node_id_str.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_add() -> String {
        std::fs::read_to_string("tests/fixtures/add.jsonld")
            .expect("invariant: add.jsonld fixture must exist")
    }

    #[test]
    fn parse_add_jsonld_five_ops() {
        let module = parse_jsonld(&fixture_add()).expect("invariant: add.jsonld must parse");
        assert_eq!(module.name.0, "main");
        assert_eq!(module.functions.len(), 1);

        let func = &module.functions[0];
        assert_eq!(func.name.0, "main");
        assert_eq!(func.return_type, DuumbiType::I64);
        assert_eq!(func.blocks.len(), 1);

        let block = &func.blocks[0];
        assert_eq!(block.label.0, "entry");
        assert_eq!(block.ops.len(), 5);

        assert_eq!(block.ops[0].op, Op::Const(3));
        assert_eq!(block.ops[1].op, Op::Const(5));
        assert_eq!(block.ops[2].op, Op::Add);
        assert_eq!(block.ops[3].op, Op::Print);
        assert_eq!(block.ops[4].op, Op::Return);
    }

    #[test]
    fn missing_type_field() {
        let json = r#"{"@id": "duumbi:test"}"#;
        let err = parse_jsonld(json).unwrap_err();
        assert!(matches!(err, ParseError::MissingField { field, .. } if field == "@type"));
    }

    #[test]
    fn unknown_op_type() {
        let json = r#"{
            "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
            "@type": "duumbi:Module",
            "@id": "duumbi:test",
            "duumbi:name": "test",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:test/main",
                "duumbi:name": "main",
                "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:test/main/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [{
                        "@type": "duumbi:Modulo",
                        "@id": "duumbi:test/main/entry/0"
                    }]
                }]
            }]
        }"#;
        let err = parse_jsonld(json).unwrap_err();
        assert!(matches!(err, ParseError::UnknownOp { op_type, .. } if op_type == "duumbi:Modulo"));
    }

    #[test]
    fn missing_value_on_const() {
        let json = r#"{
            "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
            "@type": "duumbi:Module",
            "@id": "duumbi:test",
            "duumbi:name": "test",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:test/main",
                "duumbi:name": "main",
                "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:test/main/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [{
                        "@type": "duumbi:Const",
                        "@id": "duumbi:test/main/entry/0",
                        "duumbi:resultType": "i64"
                    }]
                }]
            }]
        }"#;
        let err = parse_jsonld(json).unwrap_err();
        assert!(matches!(err, ParseError::MissingField { field, .. } if field == "duumbi:value"));
    }

    #[test]
    fn invalid_json() {
        let err = parse_jsonld("not json at all").unwrap_err();
        assert!(matches!(err, ParseError::Json { .. }));
    }
}
