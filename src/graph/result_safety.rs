//! Result/Option safety validation (E030–E035).
//!
//! Checks that Result and Option values produced by Call nodes are always
//! handled before use: matched exhaustively, checked with ResultIsOk/OptionIsSome,
//! or explicitly unwrapped (with a warning). Also validates that ResultOk/ResultErr
//! payloads match the declared Result<T,E> type parameters.

use std::collections::HashMap;

use petgraph::visit::EdgeRef;

use crate::errors::{Diagnostic, DiagnosticLevel, codes};
use crate::graph::{BlockInfo, FunctionInfo, GraphEdge, GraphNode, SemanticGraph};
use crate::types::{DuumbiType, NodeId, Op};

/// Returns `true` if the graph contains any Result/Option ops.
///
/// Used to gate the E030–E035 checks so Phase 0–9a-2 graphs are unaffected.
#[must_use]
pub fn has_result_option_ops(graph: &SemanticGraph) -> bool {
    graph.graph.node_indices().any(|idx| {
        matches!(
            &graph.graph[idx].op,
            Op::ResultOk
                | Op::ResultErr
                | Op::ResultIsOk
                | Op::ResultUnwrap
                | Op::ResultUnwrapErr
                | Op::OptionSome
                | Op::OptionNone
                | Op::OptionIsSome
                | Op::OptionUnwrap
                | Op::Match { .. }
        )
    })
}

/// Runs all Result/Option safety checks (E030–E035) for the given graph.
///
/// Collects all diagnostics without short-circuiting.
pub fn check_result_option_safety(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    for func_info in &graph.functions {
        for block_info in &func_info.blocks {
            check_unhandled_result_option(graph, func_info, block_info, diagnostics);
            check_unwrap_without_check(graph, block_info, diagnostics);
            check_non_exhaustive_match(graph, block_info, diagnostics);
            check_result_payload_types(graph, block_info, diagnostics);
        }
    }
}

/// E030/E031 — Checks that every Call returning Result/Option is handled in the same block.
///
/// A Call node's result is "handled" when a subsequent node in the same block consumes
/// it via `ResultIsOk`, `OptionIsSome`, `Match`, `ResultUnwrap`, or `OptionUnwrap`.
fn check_unhandled_result_option(
    graph: &SemanticGraph,
    _func_info: &FunctionInfo,
    block_info: &BlockInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Build a set of node IDs that are consumed by a handler in this block.
    let handled_ids = collect_handled_ids(graph, block_info);

    for &node_idx in &block_info.nodes {
        let node = &graph.graph[node_idx];
        if !matches!(&node.op, Op::Call { .. }) {
            continue;
        }
        let Some(ref result_ty) = node.result_type else {
            continue;
        };

        if result_ty.is_result() && !handled_ids.contains(&node.id) {
            diagnostics.push(
                Diagnostic::error(
                    codes::E030_UNHANDLED_RESULT,
                    format!(
                        "Unhandled Result: Call '{}' returns '{}' but no Match, \
                         ResultIsOk, or ResultUnwrap follows in the same block",
                        node.id, result_ty
                    ),
                )
                .with_node(&node.id),
            );
        } else if result_ty.is_option() && !handled_ids.contains(&node.id) {
            diagnostics.push(
                Diagnostic::error(
                    codes::E031_UNHANDLED_OPTION,
                    format!(
                        "Unhandled Option: Call '{}' returns '{}' but no Match, \
                         OptionIsSome, or OptionUnwrap follows in the same block",
                        node.id, result_ty
                    ),
                )
                .with_node(&node.id),
            );
        }
    }
}

/// E034 (WARNING) — Checks that ResultUnwrap/OptionUnwrap are preceded by a guard check.
///
/// An unwrap is "guarded" when a `ResultIsOk` or `OptionIsSome` node consuming the
/// same source appears earlier in the same block.
fn check_unwrap_without_check(
    graph: &SemanticGraph,
    block_info: &BlockInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Collect node IDs that were checked by ResultIsOk / OptionIsSome earlier in the block.
    let mut checked_ids: std::collections::HashSet<NodeId> = std::collections::HashSet::new();

    for &node_idx in &block_info.nodes {
        let node = &graph.graph[node_idx];

        match &node.op {
            Op::ResultIsOk | Op::OptionIsSome => {
                // Record the source of the check so subsequent unwraps are guarded.
                for src_id in operand_source_ids(graph, node_idx) {
                    checked_ids.insert(src_id);
                }
                // Also record this check node itself.
                checked_ids.insert(node.id.clone());
            }
            Op::ResultUnwrap | Op::ResultUnwrapErr | Op::OptionUnwrap => {
                // Determine what value is being unwrapped.
                let sources: Vec<NodeId> = operand_source_ids(graph, node_idx);
                let all_guarded =
                    !sources.is_empty() && sources.iter().all(|s| checked_ids.contains(s));

                if !all_guarded {
                    // E034 is a WARNING, not an error — unwrap is valid but risky.
                    diagnostics.push(Diagnostic {
                        level: DiagnosticLevel::Warning,
                        code: codes::E034_UNWRAP_WITHOUT_CHECK.to_string(),
                        message: format!(
                            "Unwrap without check: '{}' ({}) is used without a preceding \
                             ResultIsOk/OptionIsSome guard in the same block",
                            node.id, node.op
                        ),
                        node_id: Some(node.id.0.clone()),
                        file: None,
                        details: None,
                    });
                }
            }
            _ => {}
        }
    }
}

/// E032 — Checks that Match ops reference both ok_block and err_block that exist in the function.
fn check_non_exhaustive_match(
    graph: &SemanticGraph,
    block_info: &BlockInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for &node_idx in &block_info.nodes {
        let node = &graph.graph[node_idx];
        let Op::Match {
            ok_block,
            err_block,
        } = &node.op
        else {
            continue;
        };

        // Collect all block labels in the parent function for this node.
        let func_blocks: std::collections::HashSet<String> = graph
            .functions
            .iter()
            .find(|f| f.blocks.iter().any(|b| b.nodes.contains(&node_idx)))
            .map(|f| f.blocks.iter().map(|b| b.label.0.clone()).collect())
            .unwrap_or_default();

        if !func_blocks.contains(ok_block.as_str()) {
            let mut details = HashMap::new();
            details.insert("missing_branch".to_string(), ok_block.clone());
            diagnostics.push(
                Diagnostic::error(
                    codes::E032_NON_EXHAUSTIVE_MATCH,
                    format!(
                        "Non-exhaustive match: Match '{}' ok_block '{}' does not exist \
                         in the function",
                        node.id, ok_block
                    ),
                )
                .with_node(&node.id)
                .with_details(details),
            );
        }
        if !func_blocks.contains(err_block.as_str()) {
            let mut details = HashMap::new();
            details.insert("missing_branch".to_string(), err_block.clone());
            diagnostics.push(
                Diagnostic::error(
                    codes::E032_NON_EXHAUSTIVE_MATCH,
                    format!(
                        "Non-exhaustive match: Match '{}' err_block '{}' does not exist \
                         in the function",
                        node.id, err_block
                    ),
                )
                .with_node(&node.id)
                .with_details(details),
            );
        }
    }
}

/// E033/E035 — Checks that ResultOk/ResultErr/OptionSome payloads match the declared type params.
///
/// E033: `result_type` declared on the node is `Result<T,E>` but the wrapped payload's
///       type doesn't match T (for ResultOk) or E (for ResultErr).
/// E035: Same structural check, reported separately to distinguish construction errors
///       from type-param annotation errors.
fn check_result_payload_types(
    graph: &SemanticGraph,
    block_info: &BlockInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for &node_idx in &block_info.nodes {
        let node = &graph.graph[node_idx];

        match &node.op {
            Op::ResultOk => {
                let Some(ref declared) = node.result_type else {
                    continue;
                };
                let DuumbiType::Result(ok_ty, _) = declared else {
                    // result_type annotation is not Result<T,E> — E033
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E033_RESULT_TYPE_PARAM_MISMATCH,
                            format!(
                                "ResultOk '{}' has result_type '{}' which is not a Result type",
                                node.id, declared
                            ),
                        )
                        .with_node(&node.id),
                    );
                    continue;
                };
                // Check that the operand's type matches ok_ty.
                check_payload_matches(graph, node_idx, node, ok_ty, "ResultOk", "T", diagnostics);
            }
            Op::ResultErr => {
                let Some(ref declared) = node.result_type else {
                    continue;
                };
                let DuumbiType::Result(_, err_ty) = declared else {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E033_RESULT_TYPE_PARAM_MISMATCH,
                            format!(
                                "ResultErr '{}' has result_type '{}' which is not a Result type",
                                node.id, declared
                            ),
                        )
                        .with_node(&node.id),
                    );
                    continue;
                };
                check_payload_matches(graph, node_idx, node, err_ty, "ResultErr", "E", diagnostics);
            }
            Op::OptionSome => {
                let Some(ref declared) = node.result_type else {
                    continue;
                };
                let DuumbiType::Option(inner_ty) = declared else {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E033_RESULT_TYPE_PARAM_MISMATCH,
                            format!(
                                "OptionSome '{}' has result_type '{}' which is not an Option type",
                                node.id, declared
                            ),
                        )
                        .with_node(&node.id),
                    );
                    continue;
                };
                check_payload_matches(
                    graph,
                    node_idx,
                    node,
                    inner_ty,
                    "OptionSome",
                    "T",
                    diagnostics,
                );
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Checks that the Operand source of `node_idx` has a type matching `expected`.
///
/// Emits E035 if there is a mismatch.
fn check_payload_matches(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    node: &GraphNode,
    expected: &DuumbiType,
    op_name: &str,
    param_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if !matches!(edge_ref.weight(), GraphEdge::Operand) {
            continue;
        }
        let src = &graph.graph[edge_ref.source()];
        if let Some(actual_ty) = src.op.output_type(&src.result_type)
            && &actual_ty != expected
        {
            let mut details = HashMap::new();
            details.insert("expected".to_string(), expected.to_string());
            details.insert("found".to_string(), actual_ty.to_string());
            details.insert("param".to_string(), param_name.to_string());
            diagnostics.push(
                Diagnostic::error(
                    codes::E035_RESULT_PAYLOAD_TYPE_MISMATCH,
                    format!(
                        "Result construction type mismatch: {} '{}' expects {param_name}={expected} \
                         but payload has type {actual_ty}",
                        op_name, node.id
                    ),
                )
                .with_node(&node.id)
                .with_details(details),
            );
        }
    }
}

/// Collects node IDs that are referenced as operands by handler ops in the block.
///
/// Handler ops are: `ResultIsOk`, `OptionIsSome`, `Match`, `ResultUnwrap`,
/// `ResultUnwrapErr`, `OptionUnwrap`.
fn collect_handled_ids(
    graph: &SemanticGraph,
    block_info: &BlockInfo,
) -> std::collections::HashSet<NodeId> {
    let mut handled = std::collections::HashSet::new();

    for &node_idx in &block_info.nodes {
        let node = &graph.graph[node_idx];
        let is_handler = matches!(
            &node.op,
            Op::ResultIsOk
                | Op::OptionIsSome
                | Op::Match { .. }
                | Op::ResultUnwrap
                | Op::ResultUnwrapErr
                | Op::OptionUnwrap
        );
        if !is_handler {
            continue;
        }
        for src_id in operand_source_ids(graph, node_idx) {
            handled.insert(src_id);
        }
    }
    handled
}

/// Returns the `NodeId`s of all nodes that feed into `node_idx` via any incoming edge.
fn operand_source_ids(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
) -> Vec<NodeId> {
    graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
        .map(|e| graph.graph[e.source()].id.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{BlockInfo, FunctionInfo, GraphEdge, GraphNode, SemanticGraph};
    use crate::types::{BlockLabel, DuumbiType, FunctionName, ModuleName, NodeId, Op};
    use petgraph::stable_graph::StableGraph;

    /// Builds a minimal SemanticGraph containing a single function with a
    /// single block whose nodes are given in `ops_and_types`.
    fn make_graph(
        ops_and_types: Vec<(NodeId, Op, Option<DuumbiType>)>,
        edges: Vec<(usize, usize, GraphEdge)>,
    ) -> SemanticGraph {
        let mut graph = StableGraph::new();
        let mut node_map = std::collections::HashMap::new();
        let mut node_indices = Vec::new();

        for (id, op, result_type) in &ops_and_types {
            let idx = graph.add_node(GraphNode {
                id: id.clone(),
                op: op.clone(),
                result_type: result_type.clone(),
                function: FunctionName("main".to_string()),
                block: BlockLabel("entry".to_string()),
                owner: None,
                lifetime: None,
                lifetime_param: None,
            });
            node_map.insert(id.clone(), idx);
            node_indices.push(idx);
        }

        for (from, to, edge) in edges {
            graph.add_edge(node_indices[from], node_indices[to], edge);
        }

        let block_nodes = node_indices.clone();
        SemanticGraph {
            graph,
            node_map,
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::Void,
                params: vec![],
                lifetime_params: vec![],
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: block_nodes,
                }],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        }
    }

    fn nid(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    // --- E030: Unhandled Result ---

    #[test]
    fn e030_call_returning_result_without_handler() {
        let result_ty = DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        let sg = make_graph(
            vec![
                (
                    nid("call0"),
                    Op::Call {
                        function: "may_fail".to_string(),
                    },
                    Some(result_ty.clone()),
                ),
                (nid("ret0"), Op::Return, None),
            ],
            vec![],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == codes::E030_UNHANDLED_RESULT),
            "Expected E030, got: {diags:?}"
        );
    }

    #[test]
    fn e030_not_emitted_when_result_is_ok_follows() {
        let result_ty = DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        let sg = make_graph(
            vec![
                (
                    nid("call0"),
                    Op::Call {
                        function: "may_fail".to_string(),
                    },
                    Some(result_ty),
                ),
                (nid("check0"), Op::ResultIsOk, Some(DuumbiType::Bool)),
                (nid("ret0"), Op::Return, None),
            ],
            // check0 reads from call0
            vec![(0, 1, GraphEdge::Operand)],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            !diags.iter().any(|d| d.code == codes::E030_UNHANDLED_RESULT),
            "Expected no E030 when ResultIsOk present, got: {diags:?}"
        );
    }

    // --- E031: Unhandled Option ---

    #[test]
    fn e031_call_returning_option_without_handler() {
        let option_ty = DuumbiType::Option(Box::new(DuumbiType::I64));
        let sg = make_graph(
            vec![
                (
                    nid("call0"),
                    Op::Call {
                        function: "find".to_string(),
                    },
                    Some(option_ty),
                ),
                (nid("ret0"), Op::Return, None),
            ],
            vec![],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == codes::E031_UNHANDLED_OPTION),
            "Expected E031, got: {diags:?}"
        );
    }

    #[test]
    fn e031_not_emitted_when_option_is_some_follows() {
        let option_ty = DuumbiType::Option(Box::new(DuumbiType::I64));
        let sg = make_graph(
            vec![
                (
                    nid("call0"),
                    Op::Call {
                        function: "find".to_string(),
                    },
                    Some(option_ty),
                ),
                (nid("chk0"), Op::OptionIsSome, Some(DuumbiType::Bool)),
                (nid("ret0"), Op::Return, None),
            ],
            vec![(0, 1, GraphEdge::Operand)],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            !diags.iter().any(|d| d.code == codes::E031_UNHANDLED_OPTION),
            "Expected no E031 when OptionIsSome present, got: {diags:?}"
        );
    }

    // --- E032: Non-exhaustive match ---

    #[test]
    fn e032_match_missing_err_block() {
        // ok_block = "entry" (exists), err_block = "missing" (does not)
        let sg = make_graph(
            vec![(
                nid("m0"),
                Op::Match {
                    ok_block: "entry".to_string(),
                    err_block: "missing".to_string(),
                },
                None,
            )],
            vec![],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            diags.iter().any(|d| {
                d.code == codes::E032_NON_EXHAUSTIVE_MATCH && d.message.contains("missing")
            }),
            "Expected E032 for missing err_block, got: {diags:?}"
        );
    }

    #[test]
    fn e032_match_both_blocks_exist_no_error() {
        // Graph has two blocks: "entry" and "ok_branch"
        let result_ty = DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        let mut graph = StableGraph::new();
        let mut node_map = std::collections::HashMap::new();

        let call_idx = graph.add_node(GraphNode {
            id: nid("call0"),
            op: Op::Call {
                function: "f".to_string(),
            },
            result_type: Some(result_ty.clone()),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        node_map.insert(nid("call0"), call_idx);

        let match_idx = graph.add_node(GraphNode {
            id: nid("m0"),
            op: Op::Match {
                ok_block: "ok_branch".to_string(),
                err_block: "err_branch".to_string(),
            },
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        node_map.insert(nid("m0"), match_idx);
        graph.add_edge(call_idx, match_idx, GraphEdge::Operand);

        let ok_ret_idx = graph.add_node(GraphNode {
            id: nid("ret_ok"),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("ok_branch".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        node_map.insert(nid("ret_ok"), ok_ret_idx);

        let err_ret_idx = graph.add_node(GraphNode {
            id: nid("ret_err"),
            op: Op::Return,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("err_branch".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        node_map.insert(nid("ret_err"), err_ret_idx);

        let sg = SemanticGraph {
            graph,
            node_map,
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::Void,
                params: vec![],
                lifetime_params: vec![],
                blocks: vec![
                    BlockInfo {
                        label: BlockLabel("entry".to_string()),
                        nodes: vec![call_idx, match_idx],
                    },
                    BlockInfo {
                        label: BlockLabel("ok_branch".to_string()),
                        nodes: vec![ok_ret_idx],
                    },
                    BlockInfo {
                        label: BlockLabel("err_branch".to_string()),
                        nodes: vec![err_ret_idx],
                    },
                ],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            !diags
                .iter()
                .any(|d| d.code == codes::E032_NON_EXHAUSTIVE_MATCH),
            "Expected no E032 when both branches exist, got: {diags:?}"
        );
    }

    // --- E033: Result/Option type param mismatch ---

    #[test]
    fn e033_result_ok_has_non_result_type() {
        // ResultOk node declares result_type = i64 (not a Result type)
        let sg = make_graph(
            vec![
                (nid("c0"), Op::Const(1), Some(DuumbiType::I64)),
                (
                    nid("ok0"),
                    Op::ResultOk,
                    Some(DuumbiType::I64), // wrong: should be Result<T,E>
                ),
            ],
            vec![(0, 1, GraphEdge::Operand)],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            diags
                .iter()
                .any(|d| d.code == codes::E033_RESULT_TYPE_PARAM_MISMATCH),
            "Expected E033 for ResultOk with non-Result result_type, got: {diags:?}"
        );
    }

    #[test]
    fn e033_option_some_has_non_option_type() {
        let sg = make_graph(
            vec![
                (nid("c0"), Op::Const(1), Some(DuumbiType::I64)),
                (
                    nid("s0"),
                    Op::OptionSome,
                    Some(DuumbiType::I64), // wrong: should be Option<T>
                ),
            ],
            vec![(0, 1, GraphEdge::Operand)],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            diags
                .iter()
                .any(|d| d.code == codes::E033_RESULT_TYPE_PARAM_MISMATCH),
            "Expected E033 for OptionSome with non-Option result_type, got: {diags:?}"
        );
    }

    // --- E034: Unwrap without check (warning) ---

    #[test]
    fn e034_result_unwrap_without_is_ok_check() {
        let result_ty = DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        let sg = make_graph(
            vec![
                (
                    nid("call0"),
                    Op::Call {
                        function: "f".to_string(),
                    },
                    Some(result_ty),
                ),
                (nid("u0"), Op::ResultUnwrap, Some(DuumbiType::I64)),
                (nid("ret0"), Op::Return, None),
            ],
            // u0 reads from call0
            vec![(0, 1, GraphEdge::Operand), (1, 2, GraphEdge::Operand)],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            diags
                .iter()
                .any(|d| d.code == codes::E034_UNWRAP_WITHOUT_CHECK),
            "Expected E034 warning for unguarded ResultUnwrap, got: {diags:?}"
        );
        // Must be a warning, not an error.
        let e034 = diags
            .iter()
            .find(|d| d.code == codes::E034_UNWRAP_WITHOUT_CHECK)
            .unwrap();
        assert_eq!(
            e034.level,
            crate::errors::DiagnosticLevel::Warning,
            "E034 must be Warning level"
        );
    }

    #[test]
    fn e034_not_emitted_when_is_ok_precedes_unwrap() {
        let result_ty = DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        let sg = make_graph(
            vec![
                (
                    nid("call0"),
                    Op::Call {
                        function: "f".to_string(),
                    },
                    Some(result_ty),
                ),
                (nid("chk0"), Op::ResultIsOk, Some(DuumbiType::Bool)),
                (nid("u0"), Op::ResultUnwrap, Some(DuumbiType::I64)),
                (nid("ret0"), Op::Return, None),
            ],
            // chk0 reads call0; u0 reads call0
            vec![
                (0, 1, GraphEdge::Operand),
                (0, 2, GraphEdge::Operand),
                (2, 3, GraphEdge::Operand),
            ],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            !diags
                .iter()
                .any(|d| d.code == codes::E034_UNWRAP_WITHOUT_CHECK),
            "Expected no E034 when ResultIsOk precedes ResultUnwrap, got: {diags:?}"
        );
    }

    // --- E035: Result construction with wrong payload type ---

    #[test]
    fn e035_result_ok_wrong_payload_type() {
        // ResultOk wraps f64 but declares Result<i64, string>
        let result_ty = DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        let sg = make_graph(
            vec![
                (nid("c0"), Op::ConstF64(1.0), Some(DuumbiType::F64)), // f64, not i64
                (nid("ok0"), Op::ResultOk, Some(result_ty)),
            ],
            vec![(0, 1, GraphEdge::Operand)],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            diags
                .iter()
                .any(|d| d.code == codes::E035_RESULT_PAYLOAD_TYPE_MISMATCH),
            "Expected E035 for wrong Ok payload type, got: {diags:?}"
        );
    }

    #[test]
    fn e035_result_ok_correct_payload_no_error() {
        // ResultOk wraps i64, declares Result<i64, string> — correct
        let result_ty = DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String));
        let sg = make_graph(
            vec![
                (nid("c0"), Op::Const(42), Some(DuumbiType::I64)),
                (nid("ok0"), Op::ResultOk, Some(result_ty)),
            ],
            vec![(0, 1, GraphEdge::Operand)],
        );
        let mut diags = Vec::new();
        check_result_option_safety(&sg, &mut diags);
        assert!(
            !diags
                .iter()
                .any(|d| d.code == codes::E035_RESULT_PAYLOAD_TYPE_MISMATCH),
            "Expected no E035 for correct Ok payload type, got: {diags:?}"
        );
    }

    #[test]
    fn has_result_option_ops_detects_match() {
        let sg = make_graph(
            vec![(
                nid("m0"),
                Op::Match {
                    ok_block: "a".to_string(),
                    err_block: "b".to_string(),
                },
                None,
            )],
            vec![],
        );
        assert!(has_result_option_ops(&sg));
    }

    #[test]
    fn has_result_option_ops_false_for_plain_add() {
        let sg = make_graph(
            vec![
                (nid("c0"), Op::Const(1), Some(DuumbiType::I64)),
                (nid("c1"), Op::Const(2), Some(DuumbiType::I64)),
                (nid("add"), Op::Add, Some(DuumbiType::I64)),
                (nid("ret"), Op::Return, None),
            ],
            vec![
                (0, 2, GraphEdge::Left),
                (1, 2, GraphEdge::Right),
                (2, 3, GraphEdge::Operand),
            ],
        );
        assert!(!has_result_option_ops(&sg));
    }
}
