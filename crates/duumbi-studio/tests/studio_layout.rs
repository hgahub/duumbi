//! Integration tests for DUUMBI Studio layout and edge routing.
//!
//! Tests the Sugiyama layout and orthogonal edge routing algorithms.

use duumbi_studio::layout;
use duumbi_studio::state::{GraphData, GraphEdge, GraphNode};

fn node(id: &str, ty: &str, w: f64, h: f64) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        label: id.to_string(),
        node_type: ty.to_string(),
        badge: None,
        x: 0.0,
        y: 0.0,
        width: w,
        height: h,
    }
}

fn edge(src: &str, tgt: &str, ty: &str) -> GraphEdge {
    GraphEdge {
        id: format!("{src}->{tgt}"),
        source: src.to_string(),
        target: tgt.to_string(),
        label: String::new(),
        edge_type: ty.to_string(),
    }
}

/// Context-level graph: two modules connected by a dependency edge.
#[test]
fn studio_layout_context_two_modules() {
    let data = GraphData {
        nodes: vec![
            node("app/main", "module", 180.0, 80.0),
            node("stdlib/math", "module", 180.0, 80.0),
        ],
        edges: vec![edge("app/main", "stdlib/math", "dependency")],
    };

    let (nodes, bbox) = layout::compute_layout(&data);

    assert_eq!(
        nodes.len(),
        2,
        "two modules should produce two layout nodes"
    );
    assert!(
        bbox.width() > 0.0,
        "bounding box should have positive width"
    );
    assert!(
        bbox.height() > 0.0,
        "bounding box should have positive height"
    );

    let main = nodes.iter().find(|n| n.id == "app/main").expect("app/main");
    let math = nodes
        .iter()
        .find(|n| n.id == "stdlib/math")
        .expect("stdlib/math");
    assert!(
        main.y < math.y,
        "dependency source should be in an earlier layer"
    );
}

/// Container-level graph: three functions with two call edges.
#[test]
fn studio_layout_container_functions() {
    let data = GraphData {
        nodes: vec![
            node("main", "function", 200.0, 60.0),
            node("fibonacci", "function", 200.0, 60.0),
            node("print_i64", "function", 200.0, 60.0),
        ],
        edges: vec![
            edge("main", "fibonacci", "call"),
            edge("main", "print_i64", "call"),
        ],
    };

    let (nodes, _bbox) = layout::compute_layout(&data);
    assert_eq!(nodes.len(), 3);

    let main_node = nodes.iter().find(|n| n.id == "main").expect("main");
    let fib = nodes
        .iter()
        .find(|n| n.id == "fibonacci")
        .expect("fibonacci");
    let print = nodes
        .iter()
        .find(|n| n.id == "print_i64")
        .expect("print_i64");

    assert!(
        main_node.layer < fib.layer || main_node.layer < print.layer,
        "caller should be in a lower layer than at least one callee"
    );
}

/// Edge routing: verify edges produce valid SVG path data.
#[test]
fn studio_edge_routing_produces_paths() {
    use duumbi_studio::layout::edge_routing::route_edges;

    let data = GraphData {
        nodes: vec![
            node("A", "module", 120.0, 50.0),
            node("B", "module", 120.0, 50.0),
        ],
        edges: vec![edge("A", "B", "dependency")],
    };

    let (layout_nodes, _bbox) = layout::compute_layout(&data);
    let layout_edges = route_edges(&data.edges, &layout_nodes);

    assert_eq!(layout_edges.len(), 1, "one edge should produce one path");
    assert!(
        !layout_edges[0].path_data.is_empty(),
        "path data must not be empty"
    );
    assert!(
        layout_edges[0].path_data.starts_with('M'),
        "SVG path must start with M"
    );
}

/// Disconnected graph: nodes without edges still get valid positions.
#[test]
fn studio_layout_disconnected_nodes() {
    let data = GraphData {
        nodes: vec![
            node("A", "block", 100.0, 40.0),
            node("B", "block", 100.0, 40.0),
            node("C", "block", 100.0, 40.0),
        ],
        edges: vec![],
    };

    let (nodes, bbox) = layout::compute_layout(&data);
    assert_eq!(nodes.len(), 3);
    assert!(bbox.min_x < bbox.max_x, "bbox should span positive width");

    for n in &nodes {
        assert!(n.x > 0.0, "x should be positive for node {}", n.id);
        assert!(n.y > 0.0, "y should be positive for node {}", n.id);
    }
}

/// Empty graph: compute_layout should return empty without panicking.
#[test]
fn studio_layout_empty_graph() {
    let data = GraphData {
        nodes: vec![],
        edges: vec![],
    };
    let (nodes, bbox) = layout::compute_layout(&data);
    assert!(nodes.is_empty());
    assert_eq!(bbox.width(), 0.0);
}

#[test]
fn studio_root_footer_source_has_phase15_three_panel_workflow() {
    let source = include_str!("../src/app.rs");
    assert!(source.contains("<span class=\"footer-label\">\"Intents\"</span>"));
    assert!(source.contains("<span class=\"footer-label\">\"Graph\"</span>"));
    assert!(source.contains("<span class=\"footer-label\">\"Build\"</span>"));
    assert!(!source.contains("<span class=\"footer-label\">\"Plans\"</span>"));
    assert!(!source.contains("<span class=\"footer-label\">\"Agents\"</span>"));
    assert!(!source.contains("<span class=\"footer-label\">\"Registry\"</span>"));
}

#[cfg(feature = "ssr")]
#[test]
fn studio_module_discovery_includes_nested_workspace_modules() {
    let root = unique_temp_dir("duumbi-studio-modules");
    let graph = root.join(".duumbi/graph/calculator");
    std::fs::create_dir_all(&graph).expect("create graph dir");
    std::fs::write(root.join(".duumbi/graph/main.jsonld"), "{}").expect("main");
    std::fs::write(graph.join("ops.jsonld"), "{}").expect("ops");

    let modules = duumbi_studio::server_fns::discover_workspace_modules(&root);
    assert!(modules.contains(&"app/main".to_string()));
    assert!(modules.contains(&"calculator/ops".to_string()));

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn studio_run_api_no_binary_error_is_structured() {
    let root = unique_temp_dir("duumbi-studio-run");
    std::fs::create_dir_all(root.join(".duumbi")).expect("duumbi dir");

    let response = duumbi_studio::server_fns::run_workspace_for_api(&root).await;
    assert!(!response.ok);
    assert_eq!(response.exit_code, -1);
    assert!(response.stderr.contains("Build first"));

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "ssr")]
fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
