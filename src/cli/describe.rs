//! Describe command — prints human-readable pseudo-code from a semantic graph.
//!
//! Walks the graph's function/block/node structure and outputs a readable
//! summary of the program.

use petgraph::visit::EdgeRef;

use crate::graph::{GraphEdge, SemanticGraph};
use crate::types::Op;

/// Prints a human-readable pseudo-code description of the semantic graph.
pub fn describe(graph: &SemanticGraph) {
    for func in &graph.functions {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect();
        println!(
            "function {}({}) -> {} {{",
            func.name,
            params.join(", "),
            func.return_type
        );

        for block in &func.blocks {
            println!("  {}:", block.label);
            for &node_idx in &block.nodes {
                let node = &graph.graph[node_idx];
                let desc = describe_op(graph, node_idx, &node.op);
                println!("    %{} = {}", node.id, desc);
            }
        }
        println!("}}");
    }
}

/// Produces a string description of a single operation.
fn describe_op(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    op: &Op,
) -> String {
    use petgraph::Direction;

    let incoming: Vec<_> = graph
        .graph
        .edges_directed(node_idx, Direction::Incoming)
        .collect();

    match op {
        Op::Const(v) => format!("Const({v})"),
        Op::ConstF64(v) => format!("ConstF64({v})"),
        Op::ConstBool(v) => format!("ConstBool({v})"),
        Op::Add | Op::Sub | Op::Mul | Op::Div => {
            let mut left = String::from("?");
            let mut right = String::from("?");
            for e in &incoming {
                let src = &graph.graph[e.source()];
                match e.weight() {
                    GraphEdge::Left => left = format!("%{}", src.id),
                    GraphEdge::Right => right = format!("%{}", src.id),
                    _ => {}
                }
            }
            format!("{op}({left}, {right})")
        }
        Op::Compare(cmp_op) => {
            let mut left = String::from("?");
            let mut right = String::from("?");
            for e in &incoming {
                let src = &graph.graph[e.source()];
                match e.weight() {
                    GraphEdge::Left => left = format!("%{}", src.id),
                    GraphEdge::Right => right = format!("%{}", src.id),
                    _ => {}
                }
            }
            format!("Compare({left}, {right}, {cmp_op})")
        }
        Op::Branch => {
            let mut cond = String::from("?");
            if let Some((true_lbl, false_lbl)) = graph.branch_targets.get(&graph.graph[node_idx].id)
            {
                for e in &incoming {
                    if matches!(e.weight(), GraphEdge::Condition) {
                        cond = format!("%{}", graph.graph[e.source()].id);
                    }
                }
                format!("Branch({cond}, {true_lbl}, {false_lbl})")
            } else {
                format!("Branch({cond}, ?, ?)")
            }
        }
        Op::Call { function } => {
            let mut args: Vec<(usize, String)> = Vec::new();
            for e in &incoming {
                if let GraphEdge::Arg(i) = e.weight() {
                    args.push((*i, format!("%{}", graph.graph[e.source()].id)));
                }
            }
            args.sort_by_key(|(i, _)| *i);
            let arg_strs: Vec<String> = args.into_iter().map(|(_, s)| s).collect();
            format!("Call({function}, [{}])", arg_strs.join(", "))
        }
        Op::Load { variable } => format!("Load({variable})"),
        Op::Store { variable } => {
            let mut val = String::from("?");
            for e in &incoming {
                if matches!(e.weight(), GraphEdge::Operand) {
                    val = format!("%{}", graph.graph[e.source()].id);
                }
            }
            format!("Store({variable}, {val})")
        }
        Op::Print => {
            let mut val = String::from("?");
            for e in &incoming {
                if matches!(e.weight(), GraphEdge::Operand) {
                    val = format!("%{}", graph.graph[e.source()].id);
                }
            }
            format!("Print({val})")
        }
        Op::Return => {
            let mut val = String::from("?");
            for e in &incoming {
                if matches!(e.weight(), GraphEdge::Operand) {
                    val = format!("%{}", graph.graph[e.source()].id);
                }
            }
            format!("Return({val})")
        }
        // Phase 9a-1 ops — use Display impl for describe output
        other => format!("{other}"),
    }
}
