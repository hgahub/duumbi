//! Graph builder — converts parsed AST into a `SemanticGraph`.
//!
//! Two-pass algorithm:
//! 1. Create graph nodes from all ops, building `NodeId → NodeIndex` map.
//! 2. Resolve references and create edges.

use std::collections::HashMap;

use petgraph::stable_graph::StableGraph;

use crate::errors::codes;
use crate::parser::ast::ModuleAst;
use crate::types::NodeId;

use super::{BlockInfo, FunctionInfo, GraphEdge, GraphError, GraphNode, ParamInfo, SemanticGraph};

/// Builds a semantic graph from a parsed module AST.
///
/// Collects all errors rather than stopping at the first one.
/// Returns the graph on success, or all accumulated errors.
///
/// For multi-module programs use [`build_graph_no_call_check`] to skip
/// intra-module `Call` validation; cross-module call resolution is handled
/// by the [`crate::graph::program`] layer.
#[must_use = "graph build errors should be handled"]
pub fn build_graph(module: &ModuleAst) -> Result<SemanticGraph, Vec<GraphError>> {
    build_graph_impl(module, true, true)
}

/// Builds a semantic graph for a library module in a multi-module program.
///
/// Skips both intra-module `Call` target validation and the `main` entry-point
/// requirement: library modules are allowed to export functions without
/// defining `main`. Cross-module call resolution and entry-point validation
/// are handled by the [`crate::graph::program`] and compiler layers.
#[allow(dead_code)] // Called by graph::program, which is used in upcoming phase (#59)
#[must_use = "graph build errors should be handled"]
pub fn build_graph_no_call_check(module: &ModuleAst) -> Result<SemanticGraph, Vec<GraphError>> {
    build_graph_impl(module, false, false)
}

fn build_graph_impl(
    module: &ModuleAst,
    validate_calls: bool,
    require_main: bool,
) -> Result<SemanticGraph, Vec<GraphError>> {
    let mut graph = StableGraph::new();
    let mut node_map: HashMap<NodeId, petgraph::stable_graph::NodeIndex> = HashMap::new();
    let mut errors = Vec::new();
    let mut functions = Vec::new();
    let mut has_main = false;
    let mut branch_targets: HashMap<NodeId, (String, String)> = HashMap::new();

    // Collect all function names for Call target validation
    let function_names: std::collections::HashSet<&str> =
        module.functions.iter().map(|f| f.name.0.as_str()).collect();

    // Pass 1: Create all nodes
    for func_ast in &module.functions {
        if func_ast.name.0 == "main" {
            has_main = true;
        }

        let mut block_infos = Vec::new();

        for block_ast in &func_ast.blocks {
            let mut block_nodes = Vec::new();

            for op_ast in &block_ast.ops {
                if node_map.contains_key(&op_ast.id) {
                    errors.push(GraphError::DuplicateId {
                        code: codes::E005_DUPLICATE_ID,
                        node_id: op_ast.id.0.clone(),
                    });
                    continue;
                }

                // Validate Call targets against locally-defined functions.
                // In multi-module mode (validate_calls = false) this check is
                // skipped; the program layer performs cross-module validation.
                if validate_calls
                    && let crate::types::Op::Call { ref function } = op_ast.op
                    && !function_names.contains(function.as_str())
                {
                    errors.push(GraphError::OrphanRef {
                        code: codes::E004_ORPHAN_REF,
                        from_node: op_ast.id.0.clone(),
                        target: function.clone(),
                    });
                }

                let node = GraphNode {
                    id: op_ast.id.clone(),
                    op: op_ast.op.clone(),
                    result_type: op_ast.result_type,
                    function: func_ast.name.clone(),
                    block: block_ast.label.clone(),
                };

                // Store branch target labels for later lowering
                if matches!(op_ast.op, crate::types::Op::Branch)
                    && let (Some(tb), Some(fb)) = (&op_ast.true_block, &op_ast.false_block)
                {
                    branch_targets.insert(op_ast.id.clone(), (tb.0.clone(), fb.0.clone()));
                }

                let idx = graph.add_node(node);
                node_map.insert(op_ast.id.clone(), idx);
                block_nodes.push(idx);
            }

            block_infos.push(BlockInfo {
                label: block_ast.label.clone(),
                nodes: block_nodes,
            });
        }

        let params = func_ast
            .params
            .iter()
            .map(|p| ParamInfo {
                name: p.name.clone(),
                param_type: p.param_type,
            })
            .collect();

        functions.push(FunctionInfo {
            name: func_ast.name.clone(),
            return_type: func_ast.return_type,
            params,
            blocks: block_infos,
        });
    }

    if require_main && !has_main {
        errors.push(GraphError::NoEntry {
            code: codes::E006_NO_ENTRY,
        });
    }

    // Pass 2: Create edges from references
    for func_ast in &module.functions {
        for block_ast in &func_ast.blocks {
            for op_ast in &block_ast.ops {
                let Some(&src_idx) = node_map.get(&op_ast.id) else {
                    continue; // Skip duplicates already reported
                };

                // Left operand edge
                if let Some(ref left) = op_ast.left {
                    match node_map.get(&left.id) {
                        Some(&target_idx) => {
                            graph.add_edge(target_idx, src_idx, GraphEdge::Left);
                        }
                        None => errors.push(GraphError::OrphanRef {
                            code: codes::E004_ORPHAN_REF,
                            from_node: op_ast.id.0.clone(),
                            target: left.id.0.clone(),
                        }),
                    }
                }

                // Right operand edge
                if let Some(ref right) = op_ast.right {
                    match node_map.get(&right.id) {
                        Some(&target_idx) => {
                            graph.add_edge(target_idx, src_idx, GraphEdge::Right);
                        }
                        None => errors.push(GraphError::OrphanRef {
                            code: codes::E004_ORPHAN_REF,
                            from_node: op_ast.id.0.clone(),
                            target: right.id.0.clone(),
                        }),
                    }
                }

                // Single operand edge (Print, Return, Store)
                if let Some(ref operand) = op_ast.operand {
                    match node_map.get(&operand.id) {
                        Some(&target_idx) => {
                            graph.add_edge(target_idx, src_idx, GraphEdge::Operand);
                        }
                        None => errors.push(GraphError::OrphanRef {
                            code: codes::E004_ORPHAN_REF,
                            from_node: op_ast.id.0.clone(),
                            target: operand.id.0.clone(),
                        }),
                    }
                }

                // Condition edge (Branch)
                if let Some(ref condition) = op_ast.condition {
                    match node_map.get(&condition.id) {
                        Some(&target_idx) => {
                            graph.add_edge(target_idx, src_idx, GraphEdge::Condition);
                        }
                        None => errors.push(GraphError::OrphanRef {
                            code: codes::E004_ORPHAN_REF,
                            from_node: op_ast.id.0.clone(),
                            target: condition.id.0.clone(),
                        }),
                    }
                }

                // Call argument edges
                for (i, arg_ref) in op_ast.args.iter().enumerate() {
                    match node_map.get(&arg_ref.id) {
                        Some(&target_idx) => {
                            graph.add_edge(target_idx, src_idx, GraphEdge::Arg(i));
                        }
                        None => errors.push(GraphError::OrphanRef {
                            code: codes::E004_ORPHAN_REF,
                            from_node: op_ast.id.0.clone(),
                            target: arg_ref.id.0.clone(),
                        }),
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(SemanticGraph {
            graph,
            node_map,
            functions,
            branch_targets,
        })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_jsonld;

    fn fixture_add() -> String {
        std::fs::read_to_string("tests/fixtures/add.jsonld")
            .expect("invariant: add.jsonld fixture must exist")
    }

    #[test]
    fn build_add_graph_five_nodes_four_edges() {
        let module = parse_jsonld(&fixture_add()).expect("invariant: fixture must parse");
        let sg = build_graph(&module).expect("invariant: valid fixture must build");

        assert_eq!(sg.graph.node_count(), 5);
        assert_eq!(sg.graph.edge_count(), 4); // left+right for Add, operand for Print and Return
    }

    #[test]
    fn duplicate_id_detected() {
        use crate::parser::ast::*;
        use crate::types::*;

        let module = ModuleAst {
            id: NodeId("duumbi:test".to_string()),
            name: ModuleName("test".to_string()),
            functions: vec![FunctionAst {
                id: NodeId("duumbi:test/main".to_string()),
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                blocks: vec![BlockAst {
                    id: NodeId("duumbi:test/main/entry".to_string()),
                    label: BlockLabel("entry".to_string()),
                    ops: vec![
                        OpAst {
                            id: NodeId("duumbi:test/main/entry/0".to_string()),
                            op: Op::Const(1),
                            result_type: Some(DuumbiType::I64),
                            left: None,
                            right: None,
                            operand: None,
                            condition: None,
                            true_block: None,
                            false_block: None,
                            args: Vec::new(),
                        },
                        OpAst {
                            id: NodeId("duumbi:test/main/entry/0".to_string()), // duplicate!
                            op: Op::Const(2),
                            result_type: Some(DuumbiType::I64),
                            left: None,
                            right: None,
                            operand: None,
                            condition: None,
                            true_block: None,
                            false_block: None,
                            args: Vec::new(),
                        },
                    ],
                }],
            }],
            imports: vec![],
            exports: vec![],
        };

        let errs = build_graph(&module).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GraphError::DuplicateId { .. }))
        );
    }

    #[test]
    fn orphan_reference_detected() {
        use crate::parser::ast::*;
        use crate::types::*;

        let module = ModuleAst {
            id: NodeId("duumbi:test".to_string()),
            name: ModuleName("test".to_string()),
            functions: vec![FunctionAst {
                id: NodeId("duumbi:test/main".to_string()),
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                blocks: vec![BlockAst {
                    id: NodeId("duumbi:test/main/entry".to_string()),
                    label: BlockLabel("entry".to_string()),
                    ops: vec![OpAst {
                        id: NodeId("duumbi:test/main/entry/0".to_string()),
                        op: Op::Return,
                        result_type: None,
                        left: None,
                        right: None,
                        operand: Some(NodeRef {
                            id: NodeId("duumbi:nonexistent".to_string()),
                        }),
                        condition: None,
                        true_block: None,
                        false_block: None,
                        args: Vec::new(),
                    }],
                }],
            }],
            imports: vec![],
            exports: vec![],
        };

        let errs = build_graph(&module).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GraphError::OrphanRef { .. }))
        );
    }

    #[test]
    fn no_main_function_detected() {
        use crate::parser::ast::*;
        use crate::types::*;

        let module = ModuleAst {
            id: NodeId("duumbi:test".to_string()),
            name: ModuleName("test".to_string()),
            functions: vec![FunctionAst {
                id: NodeId("duumbi:test/helper".to_string()),
                name: FunctionName("helper".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                blocks: vec![BlockAst {
                    id: NodeId("duumbi:test/helper/entry".to_string()),
                    label: BlockLabel("entry".to_string()),
                    ops: vec![OpAst {
                        id: NodeId("duumbi:test/helper/entry/0".to_string()),
                        op: Op::Const(1),
                        result_type: Some(DuumbiType::I64),
                        left: None,
                        right: None,
                        operand: None,
                        condition: None,
                        true_block: None,
                        false_block: None,
                        args: Vec::new(),
                    }],
                }],
            }],
            imports: vec![],
            exports: vec![],
        };

        let errs = build_graph(&module).unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, GraphError::NoEntry { .. })));
    }

    #[test]
    fn call_unknown_function_detected() {
        use crate::parser::ast::*;
        use crate::types::*;

        let module = ModuleAst {
            id: NodeId("duumbi:test".to_string()),
            name: ModuleName("test".to_string()),
            functions: vec![FunctionAst {
                id: NodeId("duumbi:test/main".to_string()),
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                blocks: vec![BlockAst {
                    id: NodeId("duumbi:test/main/entry".to_string()),
                    label: BlockLabel("entry".to_string()),
                    ops: vec![OpAst {
                        id: NodeId("duumbi:test/main/entry/0".to_string()),
                        op: Op::Call {
                            function: "nonexistent".to_string(),
                        },
                        result_type: Some(DuumbiType::I64),
                        left: None,
                        right: None,
                        operand: None,
                        condition: None,
                        true_block: None,
                        false_block: None,
                        args: Vec::new(),
                    }],
                }],
            }],
            imports: vec![],
            exports: vec![],
        };

        let errs = build_graph(&module).unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, GraphError::OrphanRef {
                target, ..
            } if target == "nonexistent")));
    }

    #[test]
    fn function_params_preserved() {
        use crate::types::DuumbiType;
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:params": [{"duumbi:name": "n", "duumbi:paramType": "i64"}],
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/0", "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/1", "duumbi:operand": {"@id": "duumbi:t/main/e/0"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        let sg = build_graph(&module).expect("build should succeed");
        assert_eq!(sg.functions[0].params.len(), 1);
        assert_eq!(sg.functions[0].params[0].name, "n");
        assert_eq!(sg.functions[0].params[0].param_type, DuumbiType::I64);
    }

    #[test]
    fn multi_block_graph() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [
                    {
                        "@type": "duumbi:Block", "@id": "duumbi:t/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:t/main/entry/0", "duumbi:value": 1, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Return", "@id": "duumbi:t/main/entry/1", "duumbi:operand": {"@id": "duumbi:t/main/entry/0"}}
                        ]
                    },
                    {
                        "@type": "duumbi:Block", "@id": "duumbi:t/main/alt",
                        "duumbi:label": "alt",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:t/main/alt/0", "duumbi:value": 2, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Return", "@id": "duumbi:t/main/alt/1", "duumbi:operand": {"@id": "duumbi:t/main/alt/0"}}
                        ]
                    }
                ]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse should succeed");
        let sg = build_graph(&module).expect("build should succeed");
        assert_eq!(sg.functions[0].blocks.len(), 2);
        assert_eq!(sg.functions[0].blocks[0].label.0, "entry");
        assert_eq!(sg.functions[0].blocks[1].label.0, "alt");
        assert_eq!(sg.graph.node_count(), 4);
    }
}
