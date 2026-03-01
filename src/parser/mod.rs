//! JSON-LD parsing module.
//!
//! Parses `.jsonld` files into a typed AST representation suitable for
//! graph construction. Uses `serde_json` for JSON parsing and dispatches
//! on `@type` fields to construct typed `OpAst` nodes.

pub mod ast;

use ast::{BlockAst, FunctionAst, ImportAst, ModuleAst, NodeRef, OpAst, ParamAst};
use thiserror::Error;

use crate::errors::codes;
use crate::types::{BlockLabel, CompareOp, DuumbiType, FunctionName, ModuleName, NodeId, Op};

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
        "f64" => Ok(DuumbiType::F64),
        "bool" => Ok(DuumbiType::Bool),
        "void" => Ok(DuumbiType::Void),
        other => Err(ParseError::SchemaInvalid {
            code: codes::E009_SCHEMA_INVALID,
            message: format!("Unknown type '{other}'"),
        }),
    }
}

fn parse_compare_op(s: &str) -> Result<CompareOp, ParseError> {
    match s {
        "eq" => Ok(CompareOp::Eq),
        "ne" => Ok(CompareOp::Ne),
        "lt" => Ok(CompareOp::Lt),
        "le" => Ok(CompareOp::Le),
        "gt" => Ok(CompareOp::Gt),
        "ge" => Ok(CompareOp::Ge),
        other => Err(ParseError::SchemaInvalid {
            code: codes::E009_SCHEMA_INVALID,
            message: format!("Unknown compare operator '{other}'"),
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

    // Parse imports (optional — missing field defaults to empty)
    let imports = match value.get("duumbi:imports").and_then(|v| v.as_array()) {
        Some(arr) => {
            let mut imports = Vec::with_capacity(arr.len());
            for import_val in arr {
                imports.push(parse_import(import_val, node_id_str)?);
            }
            imports
        }
        None => Vec::new(),
    };

    // Parse exports (optional — missing field defaults to empty)
    let exports = match value.get("duumbi:exports").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        None => Vec::new(),
    };

    Ok(ModuleAst {
        id: NodeId(node_id_str.to_string()),
        name: ModuleName(name.to_string()),
        functions,
        imports,
        exports,
    })
}

/// Parses one entry from the `duumbi:imports` array.
fn parse_import(value: &serde_json::Value, parent_id: &str) -> Result<ImportAst, ParseError> {
    let module_name = get_str(value, "duumbi:module", parent_id)?;
    let path = get_str(value, "duumbi:path", parent_id)?;

    let functions = match value.get("duumbi:functions").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        None => Vec::new(),
    };

    Ok(ImportAst {
        module_name: module_name.to_string(),
        path: path.to_string(),
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

    // Parse params (optional — Phase 0 functions may not have params)
    let params = if let Some(params_val) = value.get("duumbi:params") {
        if let Some(params_arr) = params_val.as_array() {
            let mut params = Vec::with_capacity(params_arr.len());
            for param_val in params_arr {
                let param_name = get_str(param_val, "duumbi:name", node_id_str)?;
                let param_type_str = get_str(param_val, "duumbi:paramType", node_id_str)?;
                let param_type = parse_type_str(param_type_str)?;
                params.push(ParamAst {
                    name: param_name.to_string(),
                    param_type,
                });
            }
            params
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let mut blocks = Vec::with_capacity(blocks_arr.len());
    for block_val in blocks_arr {
        blocks.push(parse_block(block_val)?);
    }

    Ok(FunctionAst {
        id: NodeId(node_id_str.to_string()),
        name: FunctionName(name.to_string()),
        return_type,
        params,
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

/// Creates a default OpAst with only id, op, and result_type set.
fn make_op_ast(id: NodeId, op: Op, result_type: Option<DuumbiType>) -> OpAst {
    OpAst {
        id,
        op,
        result_type,
        left: None,
        right: None,
        operand: None,
        condition: None,
        true_block: None,
        false_block: None,
        args: Vec::new(),
    }
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
            // Determine const type from resultType
            let rt = result_type.unwrap_or(DuumbiType::I64);
            let op = match rt {
                DuumbiType::F64 => {
                    let val = value
                        .get("duumbi:value")
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| ParseError::MissingField {
                            code: codes::E003_MISSING_FIELD,
                            field: "duumbi:value".to_string(),
                            node_id: node_id_str.to_string(),
                        })?;
                    Op::ConstF64(val)
                }
                DuumbiType::Bool => {
                    let val = value
                        .get("duumbi:value")
                        .and_then(|v| v.as_bool())
                        .ok_or_else(|| ParseError::MissingField {
                            code: codes::E003_MISSING_FIELD,
                            field: "duumbi:value".to_string(),
                            node_id: node_id_str.to_string(),
                        })?;
                    Op::ConstBool(val)
                }
                _ => {
                    let val = value
                        .get("duumbi:value")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| ParseError::MissingField {
                            code: codes::E003_MISSING_FIELD,
                            field: "duumbi:value".to_string(),
                            node_id: node_id_str.to_string(),
                        })?;
                    Op::Const(val)
                }
            };
            Ok(make_op_ast(
                NodeId(node_id_str.to_string()),
                op,
                result_type,
            ))
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
            let mut ast = make_op_ast(NodeId(node_id_str.to_string()), op, result_type);
            ast.left = Some(left);
            ast.right = Some(right);
            Ok(ast)
        }
        "duumbi:Compare" => {
            let operator_str = get_str(value, "duumbi:operator", node_id_str)?;
            let compare_op = parse_compare_op(operator_str)?;
            let left = parse_node_ref(value, "duumbi:left", node_id_str)?;
            let right = parse_node_ref(value, "duumbi:right", node_id_str)?;
            let mut ast = make_op_ast(
                NodeId(node_id_str.to_string()),
                Op::Compare(compare_op),
                result_type,
            );
            ast.left = Some(left);
            ast.right = Some(right);
            Ok(ast)
        }
        "duumbi:Branch" => {
            let condition = parse_node_ref(value, "duumbi:condition", node_id_str)?;
            let true_block_str = get_str(value, "duumbi:trueBlock", node_id_str)?;
            let false_block_str = get_str(value, "duumbi:falseBlock", node_id_str)?;
            let mut ast = make_op_ast(NodeId(node_id_str.to_string()), Op::Branch, result_type);
            ast.condition = Some(condition);
            ast.true_block = Some(BlockLabel(true_block_str.to_string()));
            ast.false_block = Some(BlockLabel(false_block_str.to_string()));
            Ok(ast)
        }
        "duumbi:Call" => {
            let function_name = get_str(value, "duumbi:function", node_id_str)?;
            let args = if let Some(args_val) = value.get("duumbi:args") {
                if let Some(args_arr) = args_val.as_array() {
                    let mut refs = Vec::with_capacity(args_arr.len());
                    for arg_val in args_arr {
                        let id = arg_val.get("@id").and_then(|v| v.as_str()).ok_or_else(|| {
                            ParseError::MissingField {
                                code: codes::E003_MISSING_FIELD,
                                field: "duumbi:args.@id".to_string(),
                                node_id: node_id_str.to_string(),
                            }
                        })?;
                        refs.push(NodeRef {
                            id: NodeId(id.to_string()),
                        });
                    }
                    refs
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            let mut ast = make_op_ast(
                NodeId(node_id_str.to_string()),
                Op::Call {
                    function: function_name.to_string(),
                },
                result_type,
            );
            ast.args = args;
            Ok(ast)
        }
        "duumbi:Load" => {
            let variable = get_str(value, "duumbi:variable", node_id_str)?;
            Ok(make_op_ast(
                NodeId(node_id_str.to_string()),
                Op::Load {
                    variable: variable.to_string(),
                },
                result_type,
            ))
        }
        "duumbi:Store" => {
            let variable = get_str(value, "duumbi:variable", node_id_str)?;
            let operand = parse_node_ref(value, "duumbi:operand", node_id_str)?;
            let mut ast = make_op_ast(
                NodeId(node_id_str.to_string()),
                Op::Store {
                    variable: variable.to_string(),
                },
                result_type,
            );
            ast.operand = Some(operand);
            Ok(ast)
        }
        "duumbi:Print" | "duumbi:Return" => {
            let operand = parse_node_ref(value, "duumbi:operand", node_id_str)?;
            let op = match at_type {
                "duumbi:Print" => Op::Print,
                "duumbi:Return" => Op::Return,
                _ => unreachable!(),
            };
            let mut ast = make_op_ast(NodeId(node_id_str.to_string()), op, result_type);
            ast.operand = Some(operand);
            Ok(ast)
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
    use crate::types::CompareOp;

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

    #[test]
    fn parse_compare_all_operators() {
        for (op_str, expected) in [
            ("eq", CompareOp::Eq),
            ("ne", CompareOp::Ne),
            ("lt", CompareOp::Lt),
            ("le", CompareOp::Le),
            ("gt", CompareOp::Gt),
            ("ge", CompareOp::Ge),
        ] {
            let json = format!(
                r#"{{
                "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
                "duumbi:functions": [{{
                    "@type": "duumbi:Function", "@id": "duumbi:t/main",
                    "duumbi:name": "main", "duumbi:returnType": "i64",
                    "duumbi:blocks": [{{
                        "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {{"@type": "duumbi:Const", "@id": "duumbi:t/main/e/0", "duumbi:value": 1, "duumbi:resultType": "i64"}},
                            {{"@type": "duumbi:Const", "@id": "duumbi:t/main/e/1", "duumbi:value": 2, "duumbi:resultType": "i64"}},
                            {{
                                "@type": "duumbi:Compare",
                                "@id": "duumbi:t/main/e/2",
                                "duumbi:operator": "{op_str}",
                                "duumbi:left": {{"@id": "duumbi:t/main/e/0"}},
                                "duumbi:right": {{"@id": "duumbi:t/main/e/1"}},
                                "duumbi:resultType": "bool"
                            }}
                        ]
                    }}]
                }}]
            }}"#
            );
            let module = parse_jsonld(&json)
                .unwrap_or_else(|e| panic!("parse failed for operator '{op_str}': {e}"));
            let op = &module.functions[0].blocks[0].ops[2].op;
            assert_eq!(*op, Op::Compare(expected), "failed for operator '{op_str}'");
        }
    }

    #[test]
    fn parse_branch() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/0", "duumbi:value": true, "duumbi:resultType": "bool"},
                        {
                            "@type": "duumbi:Branch",
                            "@id": "duumbi:t/main/e/1",
                            "duumbi:condition": {"@id": "duumbi:t/main/e/0"},
                            "duumbi:trueBlock": "then",
                            "duumbi:falseBlock": "else"
                        }
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        let branch = &module.functions[0].blocks[0].ops[1];
        assert_eq!(branch.op, Op::Branch);
        assert!(branch.condition.is_some());
        assert_eq!(
            branch.true_block.as_ref().map(|b| &b.0),
            Some(&"then".to_string())
        );
        assert_eq!(
            branch.false_block.as_ref().map(|b| &b.0),
            Some(&"else".to_string())
        );
    }

    #[test]
    fn parse_branch_missing_true_block() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [{
                        "@type": "duumbi:Branch",
                        "@id": "duumbi:t/main/e/0",
                        "duumbi:condition": {"@id": "duumbi:t/main/e/x"},
                        "duumbi:falseBlock": "else"
                    }]
                }]
            }]
        }"#;
        let err = parse_jsonld(json).unwrap_err();
        assert!(
            matches!(err, ParseError::MissingField { field, .. } if field == "duumbi:trueBlock")
        );
    }

    #[test]
    fn parse_call() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/0", "duumbi:value": 10, "duumbi:resultType": "i64"},
                        {
                            "@type": "duumbi:Call",
                            "@id": "duumbi:t/main/e/1",
                            "duumbi:function": "fib",
                            "duumbi:args": [{"@id": "duumbi:t/main/e/0"}],
                            "duumbi:resultType": "i64"
                        }
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        let call = &module.functions[0].blocks[0].ops[1];
        assert_eq!(
            call.op,
            Op::Call {
                function: "fib".to_string()
            }
        );
        assert_eq!(call.args.len(), 1);
        assert_eq!(call.args[0].id.0, "duumbi:t/main/e/0");
    }

    #[test]
    fn parse_load_store() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/0", "duumbi:value": 42, "duumbi:resultType": "i64"},
                        {
                            "@type": "duumbi:Store",
                            "@id": "duumbi:t/main/e/1",
                            "duumbi:variable": "x",
                            "duumbi:operand": {"@id": "duumbi:t/main/e/0"}
                        },
                        {
                            "@type": "duumbi:Load",
                            "@id": "duumbi:t/main/e/2",
                            "duumbi:variable": "x",
                            "duumbi:resultType": "i64"
                        }
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        let store = &module.functions[0].blocks[0].ops[1];
        assert_eq!(
            store.op,
            Op::Store {
                variable: "x".to_string()
            }
        );
        assert!(store.operand.is_some());

        let load = &module.functions[0].blocks[0].ops[2];
        assert_eq!(
            load.op,
            Op::Load {
                variable: "x".to_string()
            }
        );
    }

    #[test]
    fn parse_const_f64() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [{
                        "@type": "duumbi:Const",
                        "@id": "duumbi:t/main/e/0",
                        "duumbi:value": 2.5,
                        "duumbi:resultType": "f64"
                    }]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        assert_eq!(module.functions[0].blocks[0].ops[0].op, Op::ConstF64(2.5));
    }

    #[test]
    fn parse_const_bool() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [{
                        "@type": "duumbi:Const",
                        "@id": "duumbi:t/main/e/0",
                        "duumbi:value": true,
                        "duumbi:resultType": "bool"
                    }]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        assert_eq!(module.functions[0].blocks[0].ops[0].op, Op::ConstBool(true));
    }

    #[test]
    fn parse_function_params() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/fib",
                "duumbi:name": "fib", "duumbi:returnType": "i64",
                "duumbi:params": [
                    {"duumbi:name": "n", "duumbi:paramType": "i64"}
                ],
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/fib/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:t/fib/e/0", "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/fib/e/1", "duumbi:operand": {"@id": "duumbi:t/fib/e/0"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        let func = &module.functions[0];
        assert_eq!(func.params.len(), 1);
        assert_eq!(func.params[0].name, "n");
        assert_eq!(func.params[0].param_type, DuumbiType::I64);
    }

    // -------------------------------------------------------------------------
    // Import / export parsing tests (#49)
    // -------------------------------------------------------------------------

    #[test]
    fn module_without_imports_exports_defaults_to_empty() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/0", "duumbi:value": 1, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/1", "duumbi:operand": {"@id": "duumbi:t/main/e/0"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        assert!(module.imports.is_empty(), "imports should default to empty");
        assert!(module.exports.is_empty(), "exports should default to empty");
    }

    #[test]
    fn module_with_imports_parsed_correctly() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:app", "duumbi:name": "app",
            "duumbi:imports": [
                {
                    "duumbi:module": "stdlib/math",
                    "duumbi:path": "../stdlib/math.jsonld",
                    "duumbi:functions": ["abs", "max"]
                },
                {
                    "duumbi:module": "utils",
                    "duumbi:path": "./utils.jsonld"
                }
            ],
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:app/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:app/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:app/main/e/0", "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:app/main/e/1", "duumbi:operand": {"@id": "duumbi:app/main/e/0"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");

        assert_eq!(module.imports.len(), 2);

        let first = &module.imports[0];
        assert_eq!(first.module_name, "stdlib/math");
        assert_eq!(first.path, "../stdlib/math.jsonld");
        assert_eq!(first.functions, vec!["abs", "max"]);

        let second = &module.imports[1];
        assert_eq!(second.module_name, "utils");
        assert_eq!(second.path, "./utils.jsonld");
        assert!(
            second.functions.is_empty(),
            "omitted duumbi:functions defaults to empty"
        );
    }

    #[test]
    fn module_with_exports_parsed_correctly() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:lib", "duumbi:name": "lib",
            "duumbi:exports": ["add", "sub"],
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:lib/add",
                "duumbi:name": "add", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:lib/add/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:lib/add/e/0", "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:lib/add/e/1", "duumbi:operand": {"@id": "duumbi:lib/add/e/0"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        assert_eq!(module.exports, vec!["add", "sub"]);
    }

    #[test]
    fn import_missing_module_field_returns_error() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:app", "duumbi:name": "app",
            "duumbi:imports": [
                { "duumbi:path": "./other.jsonld" }
            ],
            "duumbi:functions": []
        }"#;
        let err = parse_jsonld(json).expect_err("must fail on missing duumbi:module");
        assert!(
            matches!(err, ParseError::MissingField { ref field, .. } if field == "duumbi:module"),
            "expected MissingField for duumbi:module, got: {err:?}"
        );
    }

    #[test]
    fn import_missing_path_field_returns_error() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:app", "duumbi:name": "app",
            "duumbi:imports": [
                { "duumbi:module": "stdlib/math" }
            ],
            "duumbi:functions": []
        }"#;
        let err = parse_jsonld(json).expect_err("must fail on missing duumbi:path");
        assert!(
            matches!(err, ParseError::MissingField { ref field, .. } if field == "duumbi:path"),
            "expected MissingField for duumbi:path, got: {err:?}"
        );
    }

    #[test]
    fn missing_compare_operator() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [{
                        "@type": "duumbi:Compare",
                        "@id": "duumbi:t/main/e/0",
                        "duumbi:left": {"@id": "duumbi:t/main/e/x"},
                        "duumbi:right": {"@id": "duumbi:t/main/e/y"}
                    }]
                }]
            }]
        }"#;
        let err = parse_jsonld(json).unwrap_err();
        assert!(
            matches!(err, ParseError::MissingField { field, .. } if field == "duumbi:operator")
        );
    }
}
