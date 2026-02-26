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

use super::{BlockInfo, FunctionInfo, GraphEdge, GraphError, GraphNode, SemanticGraph};

/// Builds a semantic graph from a parsed module AST.
///
/// Collects all errors rather than stopping at the first one.
/// Returns the graph on success, or all accumulated errors.
#[must_use = "graph build errors should be handled"]
pub fn build_graph(module: &ModuleAst) -> Result<SemanticGraph, Vec<GraphError>> {
    let mut graph = StableGraph::new();
    let mut node_map: HashMap<NodeId, petgraph::stable_graph::NodeIndex> = HashMap::new();
    let mut errors = Vec::new();
    let mut functions = Vec::new();
    let mut has_main = false;

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

                let node = GraphNode {
                    id: op_ast.id.clone(),
                    op: op_ast.op.clone(),
                    result_type: op_ast.result_type,
                    function: func_ast.name.clone(),
                    block: block_ast.label.clone(),
                };

                let idx = graph.add_node(node);
                node_map.insert(op_ast.id.clone(), idx);
                block_nodes.push(idx);
            }

            block_infos.push(BlockInfo {
                label: block_ast.label.clone(),
                nodes: block_nodes,
            });
        }

        functions.push(FunctionInfo {
            name: func_ast.name.clone(),
            return_type: func_ast.return_type,
            blocks: block_infos,
        });
    }

    if !has_main {
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
            }
        }
    }

    if errors.is_empty() {
        Ok(SemanticGraph {
            graph,
            node_map,
            functions,
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
        // Manually construct an AST with duplicate IDs
        use crate::parser::ast::*;
        use crate::types::*;

        let module = ModuleAst {
            id: NodeId("duumbi:test".to_string()),
            name: ModuleName("test".to_string()),
            functions: vec![FunctionAst {
                id: NodeId("duumbi:test/main".to_string()),
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
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
                        },
                        OpAst {
                            id: NodeId("duumbi:test/main/entry/0".to_string()), // duplicate!
                            op: Op::Const(2),
                            result_type: Some(DuumbiType::I64),
                            left: None,
                            right: None,
                            operand: None,
                        },
                    ],
                }],
            }],
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
                    }],
                }],
            }],
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
                    }],
                }],
            }],
        };

        let errs = build_graph(&module).unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, GraphError::NoEntry { .. })));
    }
}
