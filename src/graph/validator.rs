//! Semantic graph validation.
//!
//! Validates type consistency and checks for cycles before compilation.
//! Collects all errors rather than stopping at the first one.

use std::collections::HashMap;

use petgraph::algo::is_cyclic_directed;
use petgraph::visit::EdgeRef;

use crate::errors::Diagnostic;
use crate::errors::codes;
use crate::types::{DuumbiType, Op};

use super::{GraphEdge, SemanticGraph};

/// Validates the semantic graph against type rules and structural constraints.
///
/// Returns a list of all validation errors found. An empty vec means valid.
/// Does not short-circuit on first error — collects all errors.
#[must_use]
pub fn validate(graph: &SemanticGraph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    check_cycles(graph, &mut diagnostics);
    check_types(graph, &mut diagnostics);
    check_return_types(graph, &mut diagnostics);

    diagnostics
}

/// Checks for cycles in the data-flow graph.
fn check_cycles(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    if is_cyclic_directed(&graph.graph) {
        diagnostics.push(Diagnostic::error(
            codes::E007_CYCLE,
            "Cycle detected in the data-flow graph",
        ));
    }
}

/// Checks that binary operation operands have matching types.
fn check_types(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    for node_idx in graph.graph.node_indices() {
        let node = &graph.graph[node_idx];
        match node.op {
            Op::Add | Op::Sub | Op::Mul | Op::Div => {
                let mut left_type: Option<DuumbiType> = None;
                let mut right_type: Option<DuumbiType> = None;

                for edge_ref in graph
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                {
                    let source_node = &graph.graph[edge_ref.source()];
                    let source_type = resolve_output_type(source_node);

                    match edge_ref.weight() {
                        GraphEdge::Left => left_type = source_type,
                        GraphEdge::Right => right_type = source_type,
                        GraphEdge::Operand => {}
                    }
                }

                if let (Some(lt), Some(rt)) = (left_type, right_type)
                    && lt != rt
                {
                    let mut details = HashMap::new();
                    details.insert("expected".to_string(), lt.to_string());
                    details.insert("found".to_string(), rt.to_string());
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E001_TYPE_MISMATCH,
                            format!("Type mismatch: {} expects matching operand types", node.op),
                        )
                        .with_node(&node.id)
                        .with_details(details),
                    );
                }
            }
            _ => {}
        }
    }
}

/// Checks that Return operations match the declared function return type.
fn check_return_types(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    for func_info in &graph.functions {
        let expected_type = func_info.return_type;

        for block_info in &func_info.blocks {
            for &node_idx in &block_info.nodes {
                let node = &graph.graph[node_idx];
                if !matches!(node.op, Op::Return) {
                    continue;
                }

                // Find the operand of the Return node
                for edge_ref in graph
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                {
                    if !matches!(edge_ref.weight(), GraphEdge::Operand) {
                        continue;
                    }
                    let source_node = &graph.graph[edge_ref.source()];
                    if let Some(actual_type) = resolve_output_type(source_node)
                        && actual_type != expected_type
                    {
                        let mut details = HashMap::new();
                        details.insert("expected".to_string(), expected_type.to_string());
                        details.insert("found".to_string(), actual_type.to_string());
                        diagnostics.push(
                            Diagnostic::error(
                                codes::E001_TYPE_MISMATCH,
                                format!(
                                    "Return type mismatch: function '{}' expects {expected_type}",
                                    func_info.name
                                ),
                            )
                            .with_node(&node.id)
                            .with_details(details),
                        );
                    }
                }
            }
        }
    }
}

/// Resolves the output type of a graph node.
fn resolve_output_type(node: &super::GraphNode) -> Option<DuumbiType> {
    match node.op {
        Op::Const(_) | Op::Add | Op::Sub | Op::Mul | Op::Div => node.result_type,
        Op::Print => Some(DuumbiType::Void),
        Op::Return => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder::build_graph;
    use crate::parser::parse_jsonld;

    fn fixture_add() -> String {
        std::fs::read_to_string("tests/fixtures/add.jsonld")
            .expect("invariant: add.jsonld fixture must exist")
    }

    #[test]
    fn valid_add_graph_no_errors() {
        let module = parse_jsonld(&fixture_add()).expect("invariant: fixture must parse");
        let sg = build_graph(&module).expect("invariant: fixture must build");
        let diags = validate(&sg);
        assert!(diags.is_empty(), "Expected no errors, got: {diags:?}");
    }

    #[test]
    fn type_mismatch_binary_op() {
        use crate::graph::*;
        use petgraph::stable_graph::StableGraph;

        let mut graph = StableGraph::new();
        let a = graph.add_node(GraphNode {
            id: NodeId("a".to_string()),
            op: Op::Const(1),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        let b = graph.add_node(GraphNode {
            id: NodeId("b".to_string()),
            op: Op::Const(2),
            result_type: Some(DuumbiType::Void), // mismatch!
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        let add = graph.add_node(GraphNode {
            id: NodeId("add".to_string()),
            op: Op::Add,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        graph.add_edge(a, add, GraphEdge::Left);
        graph.add_edge(b, add, GraphEdge::Right);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![],
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E001_TYPE_MISMATCH),
            "Expected E001 type mismatch"
        );
    }

    #[test]
    fn cycle_detected() {
        use crate::graph::*;
        use petgraph::stable_graph::StableGraph;

        let mut graph = StableGraph::new();
        let a = graph.add_node(GraphNode {
            id: NodeId("a".to_string()),
            op: Op::Add,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        let b = graph.add_node(GraphNode {
            id: NodeId("b".to_string()),
            op: Op::Add,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        graph.add_edge(a, b, GraphEdge::Left);
        graph.add_edge(b, a, GraphEdge::Left); // cycle!

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![],
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E007_CYCLE),
            "Expected E007 cycle"
        );
    }

    #[test]
    fn return_type_mismatch() {
        use crate::graph::*;
        use petgraph::stable_graph::{NodeIndex, StableGraph};

        let mut graph = StableGraph::new();
        let void_node = graph.add_node(GraphNode {
            id: NodeId("print".to_string()),
            op: Op::Print,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        let ret = graph.add_node(GraphNode {
            id: NodeId("ret".to_string()),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
        });
        graph.add_edge(void_node, ret, GraphEdge::Operand);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![NodeIndex::new(0), NodeIndex::new(1)],
                }],
            }],
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E001_TYPE_MISMATCH),
            "Expected E001 return type mismatch"
        );
    }
}
