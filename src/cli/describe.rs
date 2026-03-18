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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder::build_graph_no_call_check;
    use crate::parser::parse_jsonld;

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
        assert_eq!(result, "Const(42)");
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
        assert_eq!(result, "ConstBool(true)");
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
        // Return should show its operand
        assert!(result.starts_with("Return("), "got: {result}");
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
        assert!(result.starts_with("Add("), "got: {result}");
        assert!(
            result.contains('%'),
            "must reference operand nodes: {result}"
        );
    }

    #[test]
    fn describe_runs_without_panic() {
        // Smoke test: describe() must not panic on a valid graph
        let graph = const_graph();
        // describe() calls println!, so just ensure it completes without panic.
        describe(&graph);
    }
}
