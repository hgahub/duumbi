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
///
/// Ownership checks (E020–E029) are gated on the presence of ownership ops
/// in the graph — Phase 0–8 graphs skip them for backward compatibility.
#[must_use]
pub fn validate(graph: &SemanticGraph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    check_function_structure(graph, &mut diagnostics);
    check_terminator_position(graph, &mut diagnostics);
    check_cycles(graph, &mut diagnostics);
    check_types(graph, &mut diagnostics);
    check_return_types(graph, &mut diagnostics);
    check_branch_conditions(graph, &mut diagnostics);

    // Ownership checks — only run if the graph contains ownership ops
    if super::ownership::has_ownership_ops(graph) {
        for func_info in &graph.functions {
            super::ownership::check_use_after_move(graph, func_info, &mut diagnostics);
            super::ownership::check_borrow_exclusivity(graph, func_info, &mut diagnostics);
            super::ownership::check_lifetimes(graph, func_info, &mut diagnostics);
            super::ownership::check_drop_safety(graph, func_info, &mut diagnostics);
            super::ownership::check_move_while_borrowed(graph, func_info, &mut diagnostics);
            super::ownership::check_lifetime_params(graph, func_info, &mut diagnostics);
        }
    }

    diagnostics
}

/// Checks that every function has at least one block, and every block has at least one op.
///
/// A function with no blocks, or a block with no ops, will cause Cranelift to fail
/// with an opaque "No blocks in function" error. Catching this here produces a
/// user-readable E009 diagnostic before compilation is attempted.
fn check_function_structure(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    for func_info in &graph.functions {
        if func_info.blocks.is_empty() {
            diagnostics.push(Diagnostic::error(
                codes::E009_SCHEMA_INVALID,
                format!(
                    "Function '{}' has no blocks — every function must have at least one block",
                    func_info.name
                ),
            ));
            continue;
        }
        for block_info in &func_info.blocks {
            if block_info.nodes.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        codes::E009_SCHEMA_INVALID,
                        format!(
                            "Block '{}' in function '{}' has no ops — every block must have at least one op",
                            block_info.label, func_info.name
                        ),
                    ),
                );
            }
        }
    }
}

/// Checks that Return and Branch ops appear only as the last op in a block.
///
/// Cranelift treats these as block terminators — any instruction emitted after
/// a terminator causes a panic ("you cannot add an instruction to a block already filled").
fn check_terminator_position(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    for func_info in &graph.functions {
        for block_info in &func_info.blocks {
            if block_info.nodes.is_empty() {
                continue; // Already reported by check_function_structure
            }

            // Check: last op must be Return or Branch
            let last_idx = *block_info.nodes.last().expect("invariant: non-empty nodes");
            let last_node = &graph.graph[last_idx];
            if !matches!(&last_node.op, Op::Return | Op::Branch) {
                diagnostics.push(
                    Diagnostic::error(
                        codes::E009_SCHEMA_INVALID,
                        format!(
                            "Block '{}' in function '{}' does not end with Return or Branch \
                             — last op is {} '{}'",
                            block_info.label, func_info.name, last_node.op, last_node.id
                        ),
                    )
                    .with_node(&last_node.id),
                );
            }

            // Check: no terminator before the last position
            for (i, &node_idx) in block_info.nodes.iter().enumerate() {
                let node = &graph.graph[node_idx];
                let is_terminator = matches!(&node.op, Op::Return | Op::Branch);
                if is_terminator && i < block_info.nodes.len() - 1 {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E009_SCHEMA_INVALID,
                            format!(
                                "{} op '{}' is not the last op in block '{}' of function '{}' \
                                 — no ops may follow a terminator",
                                node.op, node.id, block_info.label, func_info.name
                            ),
                        )
                        .with_node(&node.id),
                    );
                }
            }
        }
    }
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
        match &node.op {
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Compare(_) => {
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
                        _ => {}
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
        let expected_type = &func_info.return_type;

        for block_info in &func_info.blocks {
            for &node_idx in &block_info.nodes {
                let node = &graph.graph[node_idx];
                if !matches!(&node.op, Op::Return) {
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
                        && actual_type != *expected_type
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

/// Checks that Branch condition operands are boolean.
fn check_branch_conditions(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    for node_idx in graph.graph.node_indices() {
        let node = &graph.graph[node_idx];
        if !matches!(&node.op, Op::Branch) {
            continue;
        }

        for edge_ref in graph
            .graph
            .edges_directed(node_idx, petgraph::Direction::Incoming)
        {
            if !matches!(edge_ref.weight(), GraphEdge::Condition) {
                continue;
            }
            let source_node = &graph.graph[edge_ref.source()];
            if let Some(actual_type) = resolve_output_type(source_node)
                && actual_type != DuumbiType::Bool
            {
                let mut details = HashMap::new();
                details.insert("expected".to_string(), "bool".to_string());
                details.insert("found".to_string(), actual_type.to_string());
                diagnostics.push(
                    Diagnostic::error(codes::E001_TYPE_MISMATCH, "Branch condition must be bool")
                        .with_node(&node.id)
                        .with_details(details),
                );
            }
        }
    }
}

/// Resolves the output type of a graph node.
fn resolve_output_type(node: &super::GraphNode) -> Option<DuumbiType> {
    node.op.output_type(&node.result_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder::build_graph;
    use crate::graph::*;
    use crate::parser::parse_jsonld;
    use crate::types::*;
    use petgraph::stable_graph::StableGraph;

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
        let mut graph = StableGraph::new();
        let a = graph.add_node(GraphNode {
            id: NodeId("a".to_string()),
            op: Op::Const(1),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let b = graph.add_node(GraphNode {
            id: NodeId("b".to_string()),
            op: Op::Const(2),
            result_type: Some(DuumbiType::Void), // mismatch!
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let add = graph.add_node(GraphNode {
            id: NodeId("add".to_string()),
            op: Op::Add,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(a, add, GraphEdge::Left);
        graph.add_edge(b, add, GraphEdge::Right);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E001_TYPE_MISMATCH),
            "Expected E001 type mismatch"
        );
    }

    #[test]
    fn type_mismatch_f64_mixed_operands() {
        let mut graph = StableGraph::new();
        let a = graph.add_node(GraphNode {
            id: NodeId("a".to_string()),
            op: Op::Const(1),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let b = graph.add_node(GraphNode {
            id: NodeId("b".to_string()),
            op: Op::ConstF64(2.0),
            result_type: Some(DuumbiType::F64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let add = graph.add_node(GraphNode {
            id: NodeId("add".to_string()),
            op: Op::Add,
            result_type: Some(DuumbiType::F64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(a, add, GraphEdge::Left);
        graph.add_edge(b, add, GraphEdge::Right);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E001_TYPE_MISMATCH),
            "Expected E001 for mixed i64/f64 operands"
        );
    }

    #[test]
    fn cycle_detected() {
        let mut graph = StableGraph::new();
        let a = graph.add_node(GraphNode {
            id: NodeId("a".to_string()),
            op: Op::Add,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let b = graph.add_node(GraphNode {
            id: NodeId("b".to_string()),
            op: Op::Add,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(a, b, GraphEdge::Left);
        graph.add_edge(b, a, GraphEdge::Left); // cycle!

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E007_CYCLE),
            "Expected E007 cycle"
        );
    }

    #[test]
    fn return_type_mismatch() {
        use petgraph::stable_graph::NodeIndex;

        let mut graph = StableGraph::new();
        let void_node = graph.add_node(GraphNode {
            id: NodeId("print".to_string()),
            op: Op::Print,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let ret = graph.add_node(GraphNode {
            id: NodeId("ret".to_string()),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(void_node, ret, GraphEdge::Operand);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![NodeIndex::new(0), NodeIndex::new(1)],
                }],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E001_TYPE_MISMATCH),
            "Expected E001 return type mismatch"
        );
    }

    #[test]
    fn branch_condition_not_bool() {
        let mut graph = StableGraph::new();
        let cond = graph.add_node(GraphNode {
            id: NodeId("cond".to_string()),
            op: Op::Const(1),
            result_type: Some(DuumbiType::I64), // not bool!
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let branch = graph.add_node(GraphNode {
            id: NodeId("branch".to_string()),
            op: Op::Branch,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(cond, branch, GraphEdge::Condition);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E001_TYPE_MISMATCH),
            "Expected E001 for non-bool Branch condition"
        );
    }

    #[test]
    fn branch_condition_bool_is_valid() {
        let mut graph = StableGraph::new();
        let cond = graph.add_node(GraphNode {
            id: NodeId("cond".to_string()),
            op: Op::ConstBool(true),
            result_type: Some(DuumbiType::Bool),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let branch = graph.add_node(GraphNode {
            id: NodeId("branch".to_string()),
            op: Op::Branch,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(cond, branch, GraphEdge::Condition);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            !diags.iter().any(|d| d.code == codes::E001_TYPE_MISMATCH),
            "Expected no E001 for bool Branch condition"
        );
    }

    #[test]
    fn function_with_no_blocks_produces_e009() {
        let graph = StableGraph::new();
        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("empty_fn".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![], // no blocks!
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags
                .iter()
                .any(|d| d.code == codes::E009_SCHEMA_INVALID && d.message.contains("no blocks")),
            "Expected E009 for function with no blocks, got: {diags:?}"
        );
    }

    #[test]
    fn block_with_no_ops_produces_e009() {
        let graph = StableGraph::new();
        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("fn_empty_block".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![], // no ops!
                }],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags
                .iter()
                .any(|d| d.code == codes::E009_SCHEMA_INVALID && d.message.contains("no ops")),
            "Expected E009 for block with no ops, got: {diags:?}"
        );
    }

    #[test]
    fn block_missing_terminator_produces_e009() {
        let mut graph = StableGraph::new();
        let c = graph.add_node(GraphNode {
            id: NodeId("c".to_string()),
            op: Op::Const(1),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![c], // Const only, no Return!
                }],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E009_SCHEMA_INVALID
                && d.message.contains("does not end with Return or Branch")),
            "Expected E009 for missing terminator, got: {diags:?}"
        );
    }

    #[test]
    fn return_not_last_op_produces_e009() {
        let mut graph = StableGraph::new();
        let ret = graph.add_node(GraphNode {
            id: NodeId("ret".to_string()),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let extra = graph.add_node(GraphNode {
            id: NodeId("extra".to_string()),
            op: Op::Const(1),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: vec![ret, extra], // Return before Const — invalid!
                }],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags
                .iter()
                .any(|d| d.code == codes::E009_SCHEMA_INVALID
                    && d.message.contains("not the last op")),
            "Expected E009 for Return not at end, got: {diags:?}"
        );
    }

    #[test]
    fn compare_operands_type_mismatch() {
        let mut graph = StableGraph::new();
        let a = graph.add_node(GraphNode {
            id: NodeId("a".to_string()),
            op: Op::Const(1),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let b = graph.add_node(GraphNode {
            id: NodeId("b".to_string()),
            op: Op::ConstF64(2.0),
            result_type: Some(DuumbiType::F64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let cmp = graph.add_node(GraphNode {
            id: NodeId("cmp".to_string()),
            op: Op::Compare(CompareOp::Eq),
            result_type: Some(DuumbiType::Bool),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(a, cmp, GraphEdge::Left);
        graph.add_edge(b, cmp, GraphEdge::Right);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E001_TYPE_MISMATCH),
            "Expected E001 for Compare with mismatched operand types"
        );
    }

    #[test]
    fn plain_add_graph_skips_ownership_checks() {
        // Phase 0-8 graphs have no ownership ops — validate() should skip ownership checks
        let module = parse_jsonld(&fixture_add()).expect("invariant: fixture must parse");
        let sg = build_graph(&module).expect("invariant: fixture must build");
        let diags = validate(&sg);
        assert!(
            diags.is_empty(),
            "Plain add(3,5) should produce zero diagnostics, got: {diags:?}"
        );
    }

    #[test]
    fn ownership_checks_run_when_ops_present() {
        // A graph with a use-after-move should produce E021 via validate()
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Move", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/2",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("parse");
        let sg = build_graph(&module).expect("build");
        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E021_USE_AFTER_MOVE),
            "Expected E021 from validate(), got: {diags:?}"
        );
    }
}
