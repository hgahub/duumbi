//! Describe command — prints human-readable pseudo-code from a semantic graph.
//!
//! Walks the graph's function/block/node structure and outputs a readable
//! summary of the program.

use owo_colors::OwoColorize;
use petgraph::visit::EdgeRef;

use crate::graph::{GraphEdge, SemanticGraph};
use crate::types::Op;

/// Prints a colorized human-readable pseudo-code description of the semantic graph.
///
/// Colors: cyan bold = keywords/structure, magenta = op names,
/// green = literals, dimmed = node ID references.
pub fn describe(graph: &SemanticGraph) {
    for func in &graph.functions {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect();
        println!(
            "{} {}({}) {} {} {{",
            "function".cyan().bold(),
            func.name.cyan().bold(),
            params.join(", "),
            "->".cyan().bold(),
            func.return_type
        );

        for block in &func.blocks {
            println!("  {}{}", block.label.cyan().bold(), ":".cyan().bold(),);
            for &node_idx in &block.nodes {
                let node = &graph.graph[node_idx];
                let desc = describe_op(graph, node_idx, &node.op);
                println!("    {} = {}", format!("%{}", node.id).dimmed(), desc,);
            }
        }
        println!("{}", "}".cyan().bold());
    }
}

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

    /// Formats a node reference as a dimmed `%id` string.
    fn ref_dim(id: &crate::types::NodeId) -> String {
        format!("{}", format!("%{}", id.0).dimmed())
    }

    match op {
        Op::Const(v) => format!("{}({})", "Const".magenta(), v.to_string().green()),
        Op::ConstF64(v) => format!("{}({})", "ConstF64".magenta(), v.to_string().green()),
        Op::ConstBool(v) => format!("{}({})", "ConstBool".magenta(), v.to_string().green()),
        Op::ConstString(s) => {
            format!(
                "{}({})",
                "ConstString".magenta(),
                format!("\"{s}\"").green()
            )
        }
        Op::Add | Op::Sub | Op::Mul | Op::Div => {
            let mut left = String::from("?");
            let mut right = String::from("?");
            for e in &incoming {
                let src = &graph.graph[e.source()];
                match e.weight() {
                    GraphEdge::Left => left = ref_dim(&src.id),
                    GraphEdge::Right => right = ref_dim(&src.id),
                    _ => {}
                }
            }
            format!("{}({left}, {right})", op.to_string().magenta())
        }
        Op::Compare(cmp_op) => {
            let mut left = String::from("?");
            let mut right = String::from("?");
            for e in &incoming {
                let src = &graph.graph[e.source()];
                match e.weight() {
                    GraphEdge::Left => left = ref_dim(&src.id),
                    GraphEdge::Right => right = ref_dim(&src.id),
                    _ => {}
                }
            }
            format!("{}({left}, {right}, {cmp_op})", "Compare".magenta())
        }
        Op::Branch => {
            let mut cond = String::from("?");
            if let Some((true_lbl, false_lbl)) = graph.branch_targets.get(&graph.graph[node_idx].id)
            {
                for e in &incoming {
                    if matches!(e.weight(), GraphEdge::Condition) {
                        cond = ref_dim(&graph.graph[e.source()].id);
                    }
                }
                format!(
                    "{}({cond}, {}, {})",
                    "Branch".magenta(),
                    true_lbl.cyan(),
                    false_lbl.cyan()
                )
            } else {
                format!("{}({cond}, ?, ?)", "Branch".magenta())
            }
        }
        Op::Call { function } => {
            let mut args: Vec<(usize, String)> = Vec::new();
            for e in &incoming {
                if let GraphEdge::Arg(i) = e.weight() {
                    args.push((*i, ref_dim(&graph.graph[e.source()].id)));
                }
            }
            args.sort_by_key(|(i, _)| *i);
            let arg_strs: Vec<String> = args.into_iter().map(|(_, s)| s).collect();
            format!(
                "{}({}, [{}])",
                "Call".magenta(),
                function.cyan(),
                arg_strs.join(", ")
            )
        }
        Op::Load { variable } => format!("{}({variable})", "Load".magenta()),
        Op::Store { variable } => {
            let mut val = String::from("?");
            for e in &incoming {
                if matches!(e.weight(), GraphEdge::Operand) {
                    val = ref_dim(&graph.graph[e.source()].id);
                }
            }
            format!("{}({variable}, {val})", "Store".magenta())
        }
        Op::Print => {
            let mut val = String::from("?");
            for e in &incoming {
                if matches!(e.weight(), GraphEdge::Operand) {
                    val = ref_dim(&graph.graph[e.source()].id);
                }
            }
            format!("{}({val})", "Print".magenta())
        }
        Op::Return => {
            let mut val = String::from("?");
            for e in &incoming {
                if matches!(e.weight(), GraphEdge::Operand) {
                    val = ref_dim(&graph.graph[e.source()].id);
                }
            }
            format!("{}({val})", "Return".magenta())
        }
        // Phase 9a+ ops — use Display impl with magenta op name
        other => format!("{}", other.to_string().magenta()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder::build_graph_no_call_check;
    use crate::parser::parse_jsonld;

    /// Strips ANSI escape sequences from a string for comparison.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_escape = false;
        for c in s.chars() {
            if c == '\x1b' {
                in_escape = true;
            } else if in_escape {
                if c == 'm' {
                    in_escape = false;
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn make_graph(jsonld: &str) -> SemanticGraph {
        let module = parse_jsonld(jsonld).expect("fixture must parse");
        build_graph_no_call_check(&module).expect("fixture must build")
    }

    fn const_graph() -> SemanticGraph {
        make_graph(
            r#"{
                "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
                "@type": "duumbi:Module",
                "@id": "duumbi:test",
                "duumbi:name": "test",
                "duumbi:functions": [{
                    "@type": "duumbi:Function",
                    "@id": "duumbi:test/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "i64",
                    "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:test/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {
                                "@type": "duumbi:Const",
                                "@id": "duumbi:test/main/entry/0",
                                "duumbi:value": 42
                            },
                            {
                                "@type": "duumbi:Return",
                                "@id": "duumbi:test/main/entry/1",
                                "duumbi:operand": {"@id": "duumbi:test/main/entry/0"}
                            }
                        ]
                    }]
                }]
            }"#,
        )
    }

    #[test]
    fn describe_op_const() {
        let graph = const_graph();
        let node_idx = graph
            .graph
            .node_indices()
            .find(|&i| matches!(graph.graph[i].op, Op::Const(42)))
            .expect("Const(42) node must exist");
        let result = describe_op(&graph, node_idx, &Op::Const(42));
        assert_eq!(strip_ansi(&result), "Const(42)");
    }

    #[test]
    fn describe_op_const_bool() {
        let graph = make_graph(
            r#"{
            "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
            "@type": "duumbi:Module",
            "@id": "duumbi:test",
            "duumbi:name": "test",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:test/main",
                "duumbi:name": "main",
                "duumbi:returnType": "bool",
                "duumbi:params": [],
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:test/main/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:test/main/entry/0",
                            "duumbi:value": true,
                            "duumbi:resultType": "bool"
                        },
                        {
                            "@type": "duumbi:Return",
                            "@id": "duumbi:test/main/entry/1",
                            "duumbi:operand": {"@id": "duumbi:test/main/entry/0"}
                        }
                    ]
                }]
            }]
        }"#,
        );
        let node_idx = graph
            .graph
            .node_indices()
            .find(|&i| matches!(graph.graph[i].op, Op::ConstBool(true)))
            .expect("ConstBool(true) node must exist");
        let result = describe_op(&graph, node_idx, &Op::ConstBool(true));
        assert_eq!(strip_ansi(&result), "ConstBool(true)");
    }

    #[test]
    fn describe_op_return() {
        let graph = const_graph();
        let node_idx = graph
            .graph
            .node_indices()
            .find(|&i| matches!(graph.graph[i].op, Op::Return))
            .expect("Return node must exist");
        let result = describe_op(&graph, node_idx, &Op::Return);
        // Return should show its operand (strip ANSI for comparison)
        let plain = strip_ansi(&result);
        assert!(plain.starts_with("Return("), "got: {plain}");
    }

    #[test]
    fn describe_op_add() {
        let graph = make_graph(
            r#"{
            "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
            "@type": "duumbi:Module",
            "@id": "duumbi:test",
            "duumbi:name": "test",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:test/main",
                "duumbi:name": "main",
                "duumbi:returnType": "i64",
                "duumbi:params": [],
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:test/main/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:test/main/entry/0",
                            "duumbi:value": 3
                        },
                        {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:test/main/entry/1",
                            "duumbi:value": 5
                        },
                        {
                            "@type": "duumbi:Add",
                            "@id": "duumbi:test/main/entry/2",
                            "duumbi:left": {"@id": "duumbi:test/main/entry/0"},
                            "duumbi:right": {"@id": "duumbi:test/main/entry/1"}
                        },
                        {
                            "@type": "duumbi:Return",
                            "@id": "duumbi:test/main/entry/3",
                            "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}
                        }
                    ]
                }]
            }]
        }"#,
        );
        let node_idx = graph
            .graph
            .node_indices()
            .find(|&i| matches!(graph.graph[i].op, Op::Add))
            .expect("Add node must exist");
        let result = describe_op(&graph, node_idx, &Op::Add);
        let plain = strip_ansi(&result);
        assert!(plain.starts_with("Add("), "got: {plain}");
        assert!(plain.contains('%'), "must reference operand nodes: {plain}");
    }

    #[test]
    fn describe_runs_without_panic() {
        // Smoke test: describe() must not panic on a valid graph
        let graph = const_graph();
        // describe() calls println!, so just ensure it completes without panic.
        describe(&graph);
    }

    #[test]
    fn describe_to_string_returns_plain_text() {
        let graph = const_graph();
        let out = describe_to_string(&graph);
        assert!(out.contains("function main() -> i64 {"));
        assert!(out.contains("Const(42)"));
        assert!(out.contains("Return("));
        assert!(
            !out.contains('\u{1b}'),
            "plain output must not contain ANSI escapes"
        );
    }
}
