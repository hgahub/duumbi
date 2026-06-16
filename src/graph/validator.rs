//! Semantic graph validation.
//!
//! Validates type consistency and checks for cycles before compilation.
//! Collects all errors rather than stopping at the first one.

use std::collections::HashMap;

use petgraph::algo::is_cyclic_directed;
use petgraph::visit::EdgeRef;

use crate::errors::Diagnostic;
use crate::errors::codes;
use crate::types::{DuumbiType, NodeId, Op};

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
    check_ssa_dominance(graph, &mut diagnostics);
    check_branch_targets(graph, &mut diagnostics);

    // Ownership checks — only run if the graph contains ownership ops
    // INVARIANT: This gate preserves backward compatibility with Phase 0–8 graphs.
    // Phase 9a-2 graphs carry Alloc/Move/Borrow/Drop ops; older graphs do not.
    // AI-AGENT: Do NOT remove this guard — it would break all pre-ownership-phase
    // graphs with spurious E020–E029 errors.
    if super::ownership::has_ownership_ops(graph) {
        for func_info in &graph.functions {
            // Analyze once per function, pass result to all checks
            let analysis = super::ownership::analyze_function(graph, func_info);
            super::ownership::check_use_after_move(&analysis, &mut diagnostics);
            super::ownership::check_borrow_exclusivity(&analysis, &mut diagnostics);
            super::ownership::check_lifetimes(&analysis, &mut diagnostics);
            super::ownership::check_drop_safety(&analysis, &mut diagnostics);
            super::ownership::check_move_while_borrowed(&analysis, &mut diagnostics);
            super::ownership::check_lifetime_params(func_info, &mut diagnostics);
        }
    }

    // Result/Option safety checks (E030–E035) — only run if graph contains such ops
    // AI-AGENT: Same backward-compat gate as above. Do NOT remove.
    if super::result_safety::has_result_option_ops(graph) {
        super::result_safety::check_result_option_safety(graph, &mut diagnostics);
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

            // Check: last op must be Return, Branch, or Match (all are terminators)
            let last_idx = *block_info.nodes.last().expect("invariant: non-empty nodes");
            let last_node = &graph.graph[last_idx];
            if !matches!(&last_node.op, Op::Return | Op::Branch | Op::Match { .. }) {
                diagnostics.push(
                    Diagnostic::error(
                        codes::E009_SCHEMA_INVALID,
                        format!(
                            "Block '{}' in function '{}' does not end with Return, Branch, or Match \
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
            Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::AddChecked
            | Op::SubChecked
            | Op::MulChecked
            | Op::DivChecked
            | Op::Compare(_) => {
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
            Op::ReadLine => {
                check_exact_result_type(
                    node,
                    &result_string_string(),
                    "ReadLine must return result<string,string>",
                    diagnostics,
                );
            }
            Op::PrintLn => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::String,
                    "PrintLn operand must be string",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "PrintLn must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::ReadFile => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::String,
                    "ReadFile path must be string",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &DuumbiType::I64,
                    "ReadFile maxBytes must be i64",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_string_string(),
                    "ReadFile must return result<string,string>",
                    diagnostics,
                );
            }
            Op::WriteFile => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::String,
                    "WriteFile path must be string",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &DuumbiType::String,
                    "WriteFile contents must be string",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "WriteFile must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::FileExists => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::String,
                    "FileExists path must be string",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &DuumbiType::Result(Box::new(DuumbiType::Bool), Box::new(DuumbiType::String)),
                    "FileExists must return result<bool,string>",
                    diagnostics,
                );
            }
            Op::ListDir => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::String,
                    "ListDir path must be string",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &DuumbiType::Result(
                        Box::new(DuumbiType::Array(Box::new(DuumbiType::String))),
                        Box::new(DuumbiType::String),
                    ),
                    "ListDir must return result<array<string>,string>",
                    diagnostics,
                );
            }
            Op::PathJoin => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::String,
                    "PathJoin left must be string",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &DuumbiType::String,
                    "PathJoin right must be string",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_string_string(),
                    "PathJoin must return result<string,string>",
                    diagnostics,
                );
            }
            Op::ServerNew => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::String,
                    "ServerNew host must be string",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::I64,
                    "ServerNew port must be i64",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &DuumbiType::I64,
                    "ServerNew timeout_ms must be i64",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &DuumbiType::Result(
                        Box::new(DuumbiType::HttpServer),
                        Box::new(DuumbiType::String),
                    ),
                    "ServerNew must return result<http_server,string>",
                    diagnostics,
                );
            }
            Op::RouteAddStatic => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::HttpServer,
                    "RouteAddStatic server must be http_server",
                    diagnostics,
                );
                check_arg_types(
                    graph,
                    node_idx,
                    node,
                    &[
                        DuumbiType::String,
                        DuumbiType::String,
                        DuumbiType::I64,
                        DuumbiType::Json,
                        DuumbiType::String,
                    ],
                    "RouteAddStatic args must be method string, path string, status i64, headers json, body string",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "RouteAddStatic must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::ServerStart => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::HttpServer,
                    "ServerStart server must be http_server",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::I64,
                    "ServerStart max_requests must be i64",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &DuumbiType::I64,
                    "ServerStart timeout_ms must be i64",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "ServerStart must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::ServerClose => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::HttpServer,
                    "ServerClose server must be http_server",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "ServerClose must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::HttpGet | Op::HttpDelete => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::String,
                    &format!("{} url must be string", node.op),
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::Json,
                    &format!("{} headers must be json", node.op),
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &DuumbiType::I64,
                    &format!("{} timeout_ms must be i64", node.op),
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_http_response_string(),
                    &format!("{} must return result<http_response,string>", node.op),
                    diagnostics,
                );
            }
            Op::HttpPost | Op::HttpPut => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::String,
                    &format!("{} url must be string", node.op),
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::Json,
                    &format!("{} headers must be json", node.op),
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &DuumbiType::String,
                    &format!("{} body must be string", node.op),
                    diagnostics,
                );
                check_arg_type(
                    graph,
                    node_idx,
                    node,
                    0,
                    &DuumbiType::I64,
                    &format!("{} timeout_ms arg must be i64", node.op),
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_http_response_string(),
                    &format!("{} must return result<http_response,string>", node.op),
                    diagnostics,
                );
            }
            Op::HttpStatus => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::HttpResponse,
                    "HttpStatus response must be http_response",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "HttpStatus must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::HttpBody => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::HttpResponse,
                    "HttpBody response must be http_response",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_string_string(),
                    "HttpBody must return result<string,string>",
                    diagnostics,
                );
            }
            Op::HttpHeaders => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::HttpResponse,
                    "HttpHeaders response must be http_response",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_json_string(),
                    "HttpHeaders must return result<json,string>",
                    diagnostics,
                );
            }
            Op::HttpResponseFree => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::HttpResponse,
                    "HttpResponseFree response must be http_response",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "HttpResponseFree must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::DbOpen => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::String,
                    "DbOpen path must be string",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_db_connection_string(),
                    "DbOpen must return result<db_connection,string>",
                    diagnostics,
                );
            }
            Op::DbExecute => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::DbConnection,
                    "DbExecute connection must be db_connection",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::String,
                    "DbExecute sql must be string",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &array_string(),
                    "DbExecute params must be array<string>",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "DbExecute must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::DbQuery => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::DbConnection,
                    "DbQuery connection must be db_connection",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::String,
                    "DbQuery sql must be string",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &array_string(),
                    "DbQuery params must be array<string>",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_db_rows_string(),
                    "DbQuery must return result<db_rows,string>",
                    diagnostics,
                );
            }
            Op::DbRowsLen => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::DbRows,
                    "DbRowsLen rows must be db_rows",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "DbRowsLen must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::DbRowGet => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::DbRows,
                    "DbRowGet rows must be db_rows",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Left,
                    &DuumbiType::I64,
                    "DbRowGet row_index must be i64",
                    diagnostics,
                );
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Right,
                    &DuumbiType::String,
                    "DbRowGet column must be string",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_string_string(),
                    "DbRowGet must return result<string,string>",
                    diagnostics,
                );
            }
            Op::DbClose => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::DbConnection,
                    "DbClose connection must be db_connection",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "DbClose must return result<i64,string>",
                    diagnostics,
                );
            }
            Op::DbRowsFree => {
                check_operand_type(
                    graph,
                    node_idx,
                    node,
                    GraphEdge::Operand,
                    &DuumbiType::DbRows,
                    "DbRowsFree rows must be db_rows",
                    diagnostics,
                );
                check_exact_result_type(
                    node,
                    &result_i64_string(),
                    "DbRowsFree must return result<i64,string>",
                    diagnostics,
                );
            }
            _ => {}
        }
    }
}

fn array_string() -> DuumbiType {
    DuumbiType::Array(Box::new(DuumbiType::String))
}

fn result_string_string() -> DuumbiType {
    DuumbiType::Result(Box::new(DuumbiType::String), Box::new(DuumbiType::String))
}

fn result_i64_string() -> DuumbiType {
    DuumbiType::Result(Box::new(DuumbiType::I64), Box::new(DuumbiType::String))
}

fn result_json_string() -> DuumbiType {
    DuumbiType::Result(Box::new(DuumbiType::Json), Box::new(DuumbiType::String))
}

fn result_http_response_string() -> DuumbiType {
    DuumbiType::Result(
        Box::new(DuumbiType::HttpResponse),
        Box::new(DuumbiType::String),
    )
}

fn result_db_connection_string() -> DuumbiType {
    DuumbiType::Result(
        Box::new(DuumbiType::DbConnection),
        Box::new(DuumbiType::String),
    )
}

fn result_db_rows_string() -> DuumbiType {
    DuumbiType::Result(Box::new(DuumbiType::DbRows), Box::new(DuumbiType::String))
}

fn check_exact_result_type(
    node: &super::GraphNode,
    expected: &DuumbiType,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if node.result_type.as_ref() == Some(expected) {
        return;
    }

    let mut details = HashMap::new();
    details.insert("expected".to_string(), expected.to_string());
    details.insert(
        "found".to_string(),
        node.result_type
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "<missing>".to_string()),
    );
    diagnostics.push(
        Diagnostic::error(codes::E001_TYPE_MISMATCH, message)
            .with_node(&node.id)
            .with_details(details),
    );
}

fn check_operand_type(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    node: &super::GraphNode,
    edge: GraphEdge,
    expected: &DuumbiType,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut found_edge = false;
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if edge_ref.weight() != &edge {
            continue;
        }
        found_edge = true;
        let source_node = &graph.graph[edge_ref.source()];
        if let Some(actual_type) = resolve_output_type(source_node)
            && &actual_type != expected
        {
            let mut details = HashMap::new();
            details.insert("expected".to_string(), expected.to_string());
            details.insert("found".to_string(), actual_type.to_string());
            diagnostics.push(
                Diagnostic::error(codes::E001_TYPE_MISMATCH, message)
                    .with_node(&node.id)
                    .with_details(details),
            );
        }
    }

    if !found_edge {
        let mut details = HashMap::new();
        details.insert("expected".to_string(), expected.to_string());
        details.insert("found".to_string(), "<missing>".to_string());
        diagnostics.push(
            Diagnostic::error(codes::E009_SCHEMA_INVALID, message)
                .with_node(&node.id)
                .with_details(details),
        );
    }
}

fn check_arg_types(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    node: &super::GraphNode,
    expected: &[DuumbiType],
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut args: Vec<(usize, Option<DuumbiType>)> = Vec::new();
    for edge_ref in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        if let GraphEdge::Arg(idx) = edge_ref.weight() {
            let source_node = &graph.graph[edge_ref.source()];
            args.push((*idx, resolve_output_type(source_node)));
        }
    }
    args.sort_by_key(|(idx, _)| *idx);

    if args.len() != expected.len() {
        let mut details = HashMap::new();
        details.insert("expected".to_string(), expected.len().to_string());
        details.insert("found".to_string(), args.len().to_string());
        diagnostics.push(
            Diagnostic::error(codes::E009_SCHEMA_INVALID, message)
                .with_node(&node.id)
                .with_details(details),
        );
        return;
    }

    for (idx, actual) in args {
        let Some(expected_type) = expected.get(idx) else {
            let mut details = HashMap::new();
            details.insert("expected".to_string(), expected.len().to_string());
            details.insert("found".to_string(), idx.to_string());
            diagnostics.push(
                Diagnostic::error(codes::E009_SCHEMA_INVALID, message)
                    .with_node(&node.id)
                    .with_details(details),
            );
            continue;
        };
        if actual.as_ref() != Some(expected_type) {
            let mut details = HashMap::new();
            details.insert("expected".to_string(), expected_type.to_string());
            details.insert(
                "found".to_string(),
                actual
                    .map(|ty| ty.to_string())
                    .unwrap_or_else(|| "<missing>".to_string()),
            );
            diagnostics.push(
                Diagnostic::error(codes::E001_TYPE_MISMATCH, message)
                    .with_node(&node.id)
                    .with_details(details),
            );
        }
    }
}

fn check_arg_type(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    node: &super::GraphNode,
    index: usize,
    expected: &DuumbiType,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let edge = GraphEdge::Arg(index);
    check_operand_type(graph, node_idx, node, edge, expected, message, diagnostics);
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

/// Rule A2: SSA Dominance Check.
///
/// Within a block, an op at index N may only reference ops at index 0..N-1 in
/// the same block. A forward reference (referencing a higher index in the same
/// block) is a schema violation: operands must be defined before use.
///
/// Incoming edges checked: `Left`, `Right`, `Operand`, `Condition`, `Arg(_)`.
fn check_ssa_dominance(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    for func_info in &graph.functions {
        for block_info in &func_info.blocks {
            // Build a map from NodeId → position in this block for O(1) lookup.
            let position_in_block: HashMap<&NodeId, usize> = block_info
                .nodes
                .iter()
                .enumerate()
                .map(|(pos, &idx)| (&graph.graph[idx].id, pos))
                .collect();

            // Derive the block @id prefix: everything in the block shares an @id
            // of the form `duumbi:module/func/block/index`.  The prefix is the
            // common `duumbi:module/func/block/` portion, which we obtain by
            // stripping the last path segment from the first op's @id.
            let block_prefix: Option<String> = block_info.nodes.first().map(|&idx| {
                let id_str = &graph.graph[idx].id.0;
                // Find the last '/' and keep everything up to and including it.
                if let Some(slash_pos) = id_str.rfind('/') {
                    id_str[..=slash_pos].to_string()
                } else {
                    // No slash — id is not in the expected format; use the full
                    // id as the prefix so cross-block references are never
                    // mis-identified as same-block.
                    id_str.clone()
                }
            });

            let Some(prefix) = block_prefix else {
                continue; // Empty block; already reported by check_function_structure.
            };

            for (pos, &node_idx) in block_info.nodes.iter().enumerate() {
                let node = &graph.graph[node_idx];

                for edge_ref in graph
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                {
                    // Only data-flow operand edges carry SSA dependencies.
                    let is_operand_edge = matches!(
                        edge_ref.weight(),
                        GraphEdge::Left
                            | GraphEdge::Right
                            | GraphEdge::Operand
                            | GraphEdge::Condition
                            | GraphEdge::Arg(_)
                    );
                    if !is_operand_edge {
                        continue;
                    }

                    let src_node = &graph.graph[edge_ref.source()];
                    let src_id = &src_node.id;

                    // Only flag references within the same block (same prefix).
                    if !src_id.0.starts_with(&prefix) {
                        continue;
                    }

                    // The referenced node is in this block — check its position.
                    if let Some(&src_pos) = position_in_block.get(src_id)
                        && src_pos > pos
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                codes::E009_SCHEMA_INVALID,
                                format!(
                                    "SSA forward reference: op '{}' at index {} references op '{}' at index {} \
                                     — operands must be defined before use (lower index)",
                                    node.id, pos, src_id, src_pos
                                ),
                            )
                            .with_node(&node.id),
                        );
                    }
                }
            }
        }
    }
}

/// Rule A3: Branch Target Validation.
///
/// For every `Branch` op, checks that both `duumbi:trueBlock` and
/// `duumbi:falseBlock` labels resolve to an existing block label inside the
/// same function. The target labels are stored in `graph.branch_targets`.
fn check_branch_targets(graph: &SemanticGraph, diagnostics: &mut Vec<Diagnostic>) {
    for func_info in &graph.functions {
        // Collect all block labels defined in this function.
        let block_labels: std::collections::HashSet<&str> = func_info
            .blocks
            .iter()
            .map(|b| b.label.0.as_str())
            .collect();

        for block_info in &func_info.blocks {
            for &node_idx in &block_info.nodes {
                let node = &graph.graph[node_idx];
                if !matches!(&node.op, Op::Branch) {
                    continue;
                }

                let Some((true_label, false_label)) = graph.branch_targets.get(&node.id) else {
                    // Missing branch_targets entry — covered by check_terminator_position
                    // or the parser; no double-reporting here.
                    continue;
                };

                for label in [true_label.as_str(), false_label.as_str()] {
                    if !block_labels.contains(label) {
                        let available: Vec<&str> = {
                            let mut v: Vec<&str> = block_labels.iter().copied().collect();
                            v.sort_unstable();
                            v
                        };
                        diagnostics.push(
                            Diagnostic::error(
                                codes::E009_SCHEMA_INVALID,
                                format!(
                                    "Branch target '{}' in op '{}' does not match any block label \
                                     in function '{}'. Available labels: [{}]",
                                    label,
                                    node.id,
                                    func_info.name,
                                    available.join(", ")
                                ),
                            )
                            .with_node(&node.id),
                        );
                    }
                }
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

    fn test_node(id: &str, op: Op, result_type: Option<DuumbiType>) -> GraphNode {
        GraphNode {
            id: NodeId(id.to_string()),
            op,
            result_type,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        }
    }

    fn graph_for_contract(
        nodes: Vec<GraphNode>,
        edges: Vec<(usize, usize, GraphEdge)>,
    ) -> SemanticGraph {
        let mut graph = StableGraph::new();
        let mut node_map = std::collections::HashMap::new();
        let mut node_indices = Vec::new();
        for node in nodes {
            let id = node.id.clone();
            let idx = graph.add_node(node);
            node_map.insert(id, idx);
            node_indices.push(idx);
        }
        for (from, to, edge) in edges {
            graph.add_edge(node_indices[from], node_indices[to], edge);
        }
        SemanticGraph {
            graph,
            node_map,
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::Void,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: node_indices,
                }],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        }
    }

    #[test]
    fn valid_add_graph_no_errors() {
        let module = parse_jsonld(&fixture_add()).expect("invariant: fixture must parse");
        let sg = build_graph(&module).expect("invariant: fixture must build");
        let diags = validate(&sg);
        assert!(diags.is_empty(), "Expected no errors, got: {diags:?}");
    }

    #[test]
    fn server_stdlib_graph_validates() {
        let json = std::fs::read_to_string("stdlib/server.jsonld")
            .expect("invariant: server stdlib graph must exist");
        let module = parse_jsonld(&json).expect("server stdlib graph must parse");
        let sg = crate::graph::builder::build_graph_no_call_check(&module)
            .expect("server stdlib graph must build");
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
    fn http_post_timeout_arg_must_be_i64() {
        let sg = graph_for_contract(
            vec![
                test_node(
                    "url",
                    Op::ConstString("u".to_string()),
                    Some(DuumbiType::String),
                ),
                test_node("headers", Op::Const(0), Some(DuumbiType::Json)),
                test_node(
                    "body",
                    Op::ConstString("b".to_string()),
                    Some(DuumbiType::String),
                ),
                test_node(
                    "timeout",
                    Op::ConstString("bad".to_string()),
                    Some(DuumbiType::String),
                ),
                test_node("post", Op::HttpPost, Some(result_http_response_string())),
            ],
            vec![
                (0, 4, GraphEdge::Operand),
                (1, 4, GraphEdge::Left),
                (2, 4, GraphEdge::Right),
                (3, 4, GraphEdge::Arg(0)),
            ],
        );
        let mut diags = Vec::new();
        check_types(&sg, &mut diags);
        assert!(
            diags
                .iter()
                .any(|d| d.code == codes::E001_TYPE_MISMATCH && d.message.contains("timeout_ms")),
            "Expected timeout_ms type mismatch, got: {diags:?}"
        );
    }

    #[test]
    fn db_query_accepts_array_string_params() {
        let sg = graph_for_contract(
            vec![
                test_node("conn", Op::Const(0), Some(DuumbiType::DbConnection)),
                test_node(
                    "sql",
                    Op::ConstString("select 1".to_string()),
                    Some(DuumbiType::String),
                ),
                test_node(
                    "params",
                    Op::ArrayNew,
                    Some(DuumbiType::Array(Box::new(DuumbiType::String))),
                ),
                test_node("query", Op::DbQuery, Some(result_db_rows_string())),
            ],
            vec![
                (0, 3, GraphEdge::Operand),
                (1, 3, GraphEdge::Left),
                (2, 3, GraphEdge::Right),
            ],
        );
        let mut diags = Vec::new();
        check_types(&sg, &mut diags);
        assert!(
            diags.is_empty(),
            "Expected DB query contract to pass, got: {diags:?}"
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
                && d.message.contains("does not end with Return")),
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

    // -------------------------------------------------------------------------
    // Rule A2: SSA Dominance Check
    // -------------------------------------------------------------------------

    /// Helper: build a minimal SemanticGraph with two nodes in the same block
    /// where `user` (index 1) has an incoming edge from `src` (index 0).
    /// Returns `(graph, src_idx, user_idx)`.
    fn make_two_node_graph(
        src_id: &str,
        src_op: Op,
        user_id: &str,
        user_op: Op,
        edge: GraphEdge,
        nodes_order: &[usize], // 0 = src first, 1 = src second
    ) -> SemanticGraph {
        let mut graph = StableGraph::new();
        let src = graph.add_node(GraphNode {
            id: NodeId(src_id.to_string()),
            op: src_op,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let user = graph.add_node(GraphNode {
            id: NodeId(user_id.to_string()),
            op: user_op,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(src, user, edge);

        // nodes_order[0]=0 means src comes first in the block list
        let ordered: Vec<petgraph::stable_graph::NodeIndex> = nodes_order
            .iter()
            .map(|&i| if i == 0 { src } else { user })
            .collect();

        SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: ordered,
                }],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        }
    }

    #[test]
    fn ssa_forward_reference_produces_e009() {
        // Block order: user (index 0) → src (index 1)
        // user has a Left edge from src — src is defined AFTER user: forward reference.
        let sg = make_two_node_graph(
            "duumbi:m/main/entry/1", // src at position 1
            Op::Const(5),
            "duumbi:m/main/entry/0", // user at position 0
            Op::Add,
            GraphEdge::Left,
            &[1, 0], // user first (index 0 in block), src second (index 1)
        );

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E009_SCHEMA_INVALID
                && d.message.contains("SSA forward reference")),
            "Expected E009 SSA forward reference, got: {diags:?}"
        );
    }

    #[test]
    fn ssa_backward_reference_is_valid() {
        // Block order: src (index 0), user (index 1) — src defined before user.
        let sg = make_two_node_graph(
            "duumbi:m/main/entry/0", // src at position 0
            Op::Const(5),
            "duumbi:m/main/entry/1", // user at position 1
            Op::Add,
            GraphEdge::Left,
            &[0, 1], // src first, user second
        );

        let diags = validate(&sg);
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("SSA forward reference")),
            "Expected no SSA forward reference error for valid graph, got: {diags:?}"
        );
    }

    #[test]
    fn ssa_cross_block_reference_is_not_flagged() {
        // A node in block "then" referencing a node in block "entry" is fine —
        // the block prefix differs so the SSA check should not flag it.
        let mut graph = StableGraph::new();
        let src = graph.add_node(GraphNode {
            id: NodeId("duumbi:m/main/entry/0".to_string()),
            op: Op::Const(1),
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let user = graph.add_node(GraphNode {
            id: NodeId("duumbi:m/main/then/0".to_string()),
            op: Op::Add,
            result_type: Some(DuumbiType::I64),
            function: FunctionName("main".to_string()),
            block: BlockLabel("then".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(src, user, GraphEdge::Left);

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![
                    BlockInfo {
                        label: BlockLabel("entry".to_string()),
                        nodes: vec![src],
                    },
                    BlockInfo {
                        label: BlockLabel("then".to_string()),
                        nodes: vec![user],
                    },
                ],
            }],
            branch_targets: std::collections::HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("SSA forward reference")),
            "Cross-block reference should not produce SSA error, got: {diags:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Rule A3: Branch Target Validation
    // -------------------------------------------------------------------------

    #[test]
    fn branch_target_valid_labels_no_error() {
        let mut graph = StableGraph::new();
        let cond = graph.add_node(GraphNode {
            id: NodeId("duumbi:m/main/entry/0".to_string()),
            op: Op::ConstBool(true),
            result_type: Some(DuumbiType::Bool),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let branch = graph.add_node(GraphNode {
            id: NodeId("duumbi:m/main/entry/1".to_string()),
            op: Op::Branch,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(cond, branch, GraphEdge::Condition);

        let mut branch_targets = std::collections::HashMap::new();
        branch_targets.insert(
            NodeId("duumbi:m/main/entry/1".to_string()),
            ("then".to_string(), "else_".to_string()),
        );

        let sg = SemanticGraph {
            graph,
            node_map: std::collections::HashMap::new(),
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: vec![],
                lifetime_params: Vec::new(),
                blocks: vec![
                    BlockInfo {
                        label: BlockLabel("entry".to_string()),
                        nodes: vec![cond, branch],
                    },
                    BlockInfo {
                        label: BlockLabel("then".to_string()),
                        nodes: vec![],
                    },
                    BlockInfo {
                        label: BlockLabel("else_".to_string()),
                        nodes: vec![],
                    },
                ],
            }],
            branch_targets,
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            !diags.iter().any(|d| d.message.contains("Branch target")),
            "Expected no branch target error for valid labels, got: {diags:?}"
        );
    }

    #[test]
    fn branch_target_unknown_true_label_produces_e009() {
        let mut graph = StableGraph::new();
        let cond = graph.add_node(GraphNode {
            id: NodeId("duumbi:m/main/entry/0".to_string()),
            op: Op::ConstBool(true),
            result_type: Some(DuumbiType::Bool),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let branch = graph.add_node(GraphNode {
            id: NodeId("duumbi:m/main/entry/1".to_string()),
            op: Op::Branch,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(cond, branch, GraphEdge::Condition);

        let mut branch_targets = std::collections::HashMap::new();
        branch_targets.insert(
            NodeId("duumbi:m/main/entry/1".to_string()),
            ("nonexistent_block".to_string(), "entry".to_string()),
        );

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
                    nodes: vec![cond, branch],
                }],
            }],
            branch_targets,
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E009_SCHEMA_INVALID
                && d.message.contains("Branch target")
                && d.message.contains("nonexistent_block")),
            "Expected E009 for unknown branch target 'nonexistent_block', got: {diags:?}"
        );
    }

    #[test]
    fn branch_target_unknown_false_label_produces_e009() {
        let mut graph = StableGraph::new();
        let cond = graph.add_node(GraphNode {
            id: NodeId("duumbi:m/main/entry/0".to_string()),
            op: Op::ConstBool(false),
            result_type: Some(DuumbiType::Bool),
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        let branch = graph.add_node(GraphNode {
            id: NodeId("duumbi:m/main/entry/1".to_string()),
            op: Op::Branch,
            result_type: None,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        });
        graph.add_edge(cond, branch, GraphEdge::Condition);

        let mut branch_targets = std::collections::HashMap::new();
        branch_targets.insert(
            NodeId("duumbi:m/main/entry/1".to_string()),
            ("entry".to_string(), "missing_else".to_string()),
        );

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
                    nodes: vec![cond, branch],
                }],
            }],
            branch_targets,
            module_name: ModuleName("test".to_string()),
        };

        let diags = validate(&sg);
        assert!(
            diags.iter().any(|d| d.code == codes::E009_SCHEMA_INVALID
                && d.message.contains("Branch target")
                && d.message.contains("missing_else")),
            "Expected E009 for unknown false branch target 'missing_else', got: {diags:?}"
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
