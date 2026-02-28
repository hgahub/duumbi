//! Cytoscape.js JSON serializer for semantic graphs.
//!
//! Converts a `SemanticGraph` into Cytoscape.js-compatible JSON with compound
//! nodes: functions are outer compounds, blocks are inner compounds, and op
//! nodes are leaves. Edges represent data flow between operations.

use petgraph::visit::EdgeRef;
use serde_json::{Value, json};

use crate::graph::{GraphEdge, SemanticGraph};
use crate::types::Op;

/// Converts a `SemanticGraph` into Cytoscape.js-compatible JSON.
///
/// The output has `nodes`, `edges`, and `errors` arrays. Functions and blocks
/// become compound (parent) nodes; op nodes are leaf nodes with CSS classes
/// for visual styling. Edges carry labels and type metadata.
#[must_use]
pub fn graph_to_cytoscape(graph: &SemanticGraph) -> Value {
    let mut nodes: Vec<Value> = Vec::new();
    let mut edges: Vec<Value> = Vec::new();
    let mut edge_counter: usize = 0;

    for func in &graph.functions {
        let fn_id = format!("fn:{}", func.name);
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect();
        let label = format!(
            "{}({}) -> {}",
            func.name,
            params.join(", "),
            func.return_type
        );

        // Function compound node
        nodes.push(json!({
            "data": {
                "id": &fn_id,
                "label": label,
                "nodeType": "function"
            }
        }));

        for block in &func.blocks {
            let block_id = format!("block:{}/{}", func.name, block.label);

            // Block compound node
            nodes.push(json!({
                "data": {
                    "id": &block_id,
                    "label": block.label.to_string(),
                    "nodeType": "block",
                    "parent": &fn_id
                }
            }));

            for &node_idx in &block.nodes {
                let node = &graph.graph[node_idx];
                let op_label = node.op.to_string();
                let op_type = op_type_name(&node.op);
                let classes = op_class(&node.op);
                let result_type = node
                    .result_type
                    .map_or("void".to_string(), |t| t.to_string());

                nodes.push(json!({
                    "data": {
                        "id": node.id.to_string(),
                        "label": op_label,
                        "nodeType": "op",
                        "opType": op_type,
                        "resultType": result_type,
                        "function": func.name.to_string(),
                        "block": block.label.to_string(),
                        "parent": &block_id
                    },
                    "classes": classes
                }));

                // Emit edges from this node's incoming connections
                for edge_ref in graph
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                {
                    let source_node = &graph.graph[edge_ref.source()];
                    let (label, edge_type) = edge_label(edge_ref.weight());

                    edges.push(json!({
                        "data": {
                            "id": format!("e{edge_counter}"),
                            "source": source_node.id.to_string(),
                            "target": node.id.to_string(),
                            "label": label,
                            "edgeType": edge_type
                        }
                    }));
                    edge_counter += 1;
                }
            }
        }
    }

    json!({
        "nodes": nodes,
        "edges": edges,
        "errors": []
    })
}

/// Returns a Cytoscape.js-compatible JSON for an error state.
///
/// The `nodes` and `edges` arrays are empty; the `errors` array contains
/// the provided error messages.
#[must_use]
pub fn error_json(errors: Vec<String>) -> Value {
    json!({
        "nodes": [],
        "edges": [],
        "errors": errors
    })
}

/// Returns the short type name for an Op (e.g., "Const", "Add").
fn op_type_name(op: &Op) -> &'static str {
    match op {
        Op::Const(_) => "Const",
        Op::ConstF64(_) => "ConstF64",
        Op::ConstBool(_) => "ConstBool",
        Op::Add => "Add",
        Op::Sub => "Sub",
        Op::Mul => "Mul",
        Op::Div => "Div",
        Op::Compare(_) => "Compare",
        Op::Branch => "Branch",
        Op::Call { .. } => "Call",
        Op::Load { .. } => "Load",
        Op::Store { .. } => "Store",
        Op::Print => "Print",
        Op::Return => "Return",
    }
}

/// Returns the CSS class name for Cytoscape.js node styling.
fn op_class(op: &Op) -> &'static str {
    match op {
        Op::Const(_) | Op::ConstF64(_) | Op::ConstBool(_) => "op-const",
        Op::Add | Op::Sub | Op::Mul | Op::Div => "op-arithmetic",
        Op::Compare(_) => "op-compare",
        Op::Branch => "op-control",
        Op::Call { .. } => "op-call",
        Op::Load { .. } | Op::Store { .. } => "op-memory",
        Op::Print | Op::Return => "op-io",
    }
}

/// Returns (label, edgeType) for a graph edge.
fn edge_label(edge: &GraphEdge) -> (&'static str, &'static str) {
    match edge {
        GraphEdge::Left => ("left", "Left"),
        GraphEdge::Right => ("right", "Right"),
        GraphEdge::Operand => ("operand", "Operand"),
        GraphEdge::Condition => ("condition", "Condition"),
        GraphEdge::TrueBlock => ("true", "TrueBlock"),
        GraphEdge::FalseBlock => ("false", "FalseBlock"),
        GraphEdge::Arg(n) => {
            // We use a static set of labels for the first few args
            match n {
                0 => ("arg[0]", "Arg"),
                1 => ("arg[1]", "Arg"),
                2 => ("arg[2]", "Arg"),
                3 => ("arg[3]", "Arg"),
                _ => ("arg[N]", "Arg"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder;
    use crate::parser;

    fn load_fixture(name: &str) -> SemanticGraph {
        let path = format!("tests/fixtures/{name}");
        let source =
            std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture not found: {path}"));
        let ast = parser::parse_jsonld(&source)
            .unwrap_or_else(|e| panic!("parse failed for {name}: {e}"));
        builder::build_graph(&ast).unwrap_or_else(|e| panic!("build failed for {name}: {e:?}"))
    }

    #[test]
    fn test_add_fixture_structure() {
        let graph = load_fixture("add.jsonld");
        let cyto = graph_to_cytoscape(&graph);

        let nodes = cyto["nodes"].as_array().expect("nodes array");
        let edges = cyto["edges"].as_array().expect("edges array");
        let errors = cyto["errors"].as_array().expect("errors array");

        // 1 function compound + 1 block compound + 5 ops = 7 nodes
        let fn_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n["data"]["nodeType"] == "function")
            .collect();
        let block_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n["data"]["nodeType"] == "block")
            .collect();
        let op_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n["data"]["nodeType"] == "op")
            .collect();

        assert_eq!(fn_nodes.len(), 1, "expected 1 function compound");
        assert_eq!(block_nodes.len(), 1, "expected 1 block compound");
        assert_eq!(op_nodes.len(), 5, "expected 5 op nodes");
        assert_eq!(edges.len(), 4, "expected 4 edges");
        assert!(errors.is_empty(), "expected no errors");
    }

    #[test]
    fn test_fibonacci_fixture_structure() {
        let graph = load_fixture("fibonacci.jsonld");
        let cyto = graph_to_cytoscape(&graph);

        let nodes = cyto["nodes"].as_array().expect("nodes array");
        let edges = cyto["edges"].as_array().expect("edges array");

        let fn_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n["data"]["nodeType"] == "function")
            .collect();
        let block_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n["data"]["nodeType"] == "block")
            .collect();
        let op_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n["data"]["nodeType"] == "op")
            .collect();

        assert_eq!(
            fn_nodes.len(),
            2,
            "expected 2 function compounds, got {}",
            fn_nodes.len()
        );
        assert!(
            block_nodes.len() >= 6,
            "expected 6+ block compounds, got {}",
            block_nodes.len()
        );
        assert!(
            op_nodes.len() >= 24,
            "expected 24+ op nodes, got {}",
            op_nodes.len()
        );
        assert!(edges.len() >= 20, "expected 20+ edges, got {}", edges.len());
    }

    #[test]
    fn test_node_classes() {
        let graph = load_fixture("add.jsonld");
        let cyto = graph_to_cytoscape(&graph);
        let nodes = cyto["nodes"].as_array().expect("nodes array");

        let op_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n["data"]["nodeType"] == "op")
            .collect();

        // Const nodes should have op-const class
        let const_nodes: Vec<_> = op_nodes
            .iter()
            .filter(|n| {
                n["data"]["opType"]
                    .as_str()
                    .map_or(false, |t| t.starts_with("Const"))
            })
            .collect();
        for node in &const_nodes {
            assert_eq!(node["classes"], "op-const");
        }

        // Add node should have op-arithmetic class
        let add_nodes: Vec<_> = op_nodes
            .iter()
            .filter(|n| n["data"]["opType"] == "Add")
            .collect();
        for node in &add_nodes {
            assert_eq!(node["classes"], "op-arithmetic");
        }
    }

    #[test]
    fn test_edge_labels() {
        let graph = load_fixture("add.jsonld");
        let cyto = graph_to_cytoscape(&graph);
        let edges = cyto["edges"].as_array().expect("edges array");

        let left_edges: Vec<_> = edges
            .iter()
            .filter(|e| e["data"]["edgeType"] == "Left")
            .collect();
        let right_edges: Vec<_> = edges
            .iter()
            .filter(|e| e["data"]["edgeType"] == "Right")
            .collect();

        assert!(!left_edges.is_empty(), "expected Left edges");
        assert!(!right_edges.is_empty(), "expected Right edges");

        for e in &left_edges {
            assert_eq!(e["data"]["label"], "left");
        }
        for e in &right_edges {
            assert_eq!(e["data"]["label"], "right");
        }
    }

    #[test]
    fn test_compound_parent_hierarchy() {
        let graph = load_fixture("add.jsonld");
        let cyto = graph_to_cytoscape(&graph);
        let nodes = cyto["nodes"].as_array().expect("nodes array");

        // Block parent should be the function
        let block = nodes
            .iter()
            .find(|n| n["data"]["nodeType"] == "block")
            .expect("block node");
        assert!(
            block["data"]["parent"]
                .as_str()
                .expect("parent")
                .starts_with("fn:"),
            "block parent should be a function"
        );

        // Op parent should be the block
        let op = nodes
            .iter()
            .find(|n| n["data"]["nodeType"] == "op")
            .expect("op node");
        assert!(
            op["data"]["parent"]
                .as_str()
                .expect("parent")
                .starts_with("block:"),
            "op parent should be a block"
        );
    }

    #[test]
    fn test_error_json() {
        let result = error_json(vec!["parse error".to_string()]);
        assert!(result["nodes"].as_array().expect("nodes").is_empty());
        assert!(result["edges"].as_array().expect("edges").is_empty());
        assert_eq!(result["errors"].as_array().expect("errors").len(), 1);
    }
}
