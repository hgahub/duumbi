//! Plain-text descriptions for semantic graphs.

use petgraph::visit::EdgeRef;

use crate::graph::{GraphEdge, SemanticGraph};
use crate::types::Op;

/// Returns a plain-text human-readable pseudo-code description of the semantic graph.
#[must_use]
pub fn describe_to_string(graph: &SemanticGraph) -> String {
    let mut lines = Vec::new();
    for func in &graph.functions {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect();
        lines.push(format!(
            "function {}({}) -> {} {{",
            func.name,
            params.join(", "),
            func.return_type
        ));

        for block in &func.blocks {
            lines.push(format!("  {}:", block.label));
            for &node_idx in &block.nodes {
                let node = &graph.graph[node_idx];
                let desc = describe_op_plain(graph, node_idx, &node.op);
                lines.push(format!("    %{} = {}", node.id, desc));
            }
        }
        lines.push("}".to_string());
    }
    lines.join("\n")
}

fn describe_op_plain(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
    op: &Op,
) -> String {
    use petgraph::Direction;

    let incoming: Vec<_> = graph
        .graph
        .edges_directed(node_idx, Direction::Incoming)
        .collect();

    fn ref_plain(id: &crate::types::NodeId) -> String {
        format!("%{}", id.0)
    }

    match op {
        Op::Const(v) => format!("Const({v})"),
        Op::ConstF64(v) => format!("ConstF64({v})"),
        Op::ConstBool(v) => format!("ConstBool({v})"),
        Op::ConstString(s) => format!("ConstString(\"{s}\")"),
        Op::Add | Op::Sub | Op::Mul | Op::Div => {
            let mut left = String::from("?");
            let mut right = String::from("?");
            for e in &incoming {
                let src = &graph.graph[e.source()];
                match e.weight() {
                    GraphEdge::Left => left = ref_plain(&src.id),
                    GraphEdge::Right => right = ref_plain(&src.id),
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
                    GraphEdge::Left => left = ref_plain(&src.id),
                    GraphEdge::Right => right = ref_plain(&src.id),
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
                        cond = ref_plain(&graph.graph[e.source()].id);
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
                    args.push((*i, ref_plain(&graph.graph[e.source()].id)));
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
                    val = ref_plain(&graph.graph[e.source()].id);
                }
            }
            format!("Store({variable}, {val})")
        }
        Op::Print => {
            let mut val = String::from("?");
            for e in &incoming {
                if matches!(e.weight(), GraphEdge::Operand) {
                    val = ref_plain(&graph.graph[e.source()].id);
                }
            }
            format!("Print({val})")
        }
        Op::Return => {
            let mut val = String::from("?");
            for e in &incoming {
                if matches!(e.weight(), GraphEdge::Operand) {
                    val = ref_plain(&graph.graph[e.source()].id);
                }
            }
            format!("Return({val})")
        }
        other => other.to_string(),
    }
}
