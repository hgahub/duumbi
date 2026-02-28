//! Phase 3 web visualizer integration tests.
//!
//! Tests cover the Cytoscape.js serializer, the axum HTTP server endpoints,
//! and WebSocket connectivity. All tests are deterministic — no live file
//! watchers or browser interaction needed.
//!
//! # Test coverage
//!
//! 1. `test_graph_to_cytoscape_add_fixture` — add.jsonld → correct node/edge counts
//! 2. `test_graph_to_cytoscape_fibonacci_fixture` — fibonacci.jsonld → 2fn + 6+block
//! 3. `test_graph_to_cytoscape_error_on_invalid` — bad JSON → errors field populated
//! 4. `test_cytoscape_node_classes` — Op type → CSS class mapping
//! 5. `test_cytoscape_edge_labels` — edge type → label mapping
//! 6. `test_server_starts_and_serves_index` — GET / → 200 + HTML
//! 7. `test_api_graph_endpoint` — GET /api/graph → valid JSON
//! 8. `test_websocket_connection` — WS upgrade → graph_update message

use duumbi::graph::builder;
use duumbi::parser;
use duumbi::web::serialize::{error_json, graph_to_cytoscape};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn load_graph(fixture: &str) -> duumbi::graph::SemanticGraph {
    let path = format!("tests/fixtures/{fixture}");
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture not found: {path}"));
    let ast =
        parser::parse_jsonld(&source).unwrap_or_else(|e| panic!("parse failed for {fixture}: {e}"));
    builder::build_graph(&ast).unwrap_or_else(|e| panic!("build failed for {fixture}: {e:?}"))
}

// ---------------------------------------------------------------------------
// Serializer tests
// ---------------------------------------------------------------------------

#[test]
fn test_graph_to_cytoscape_add_fixture() {
    let graph = load_graph("add.jsonld");
    let cyto = graph_to_cytoscape(&graph);

    let nodes = cyto["nodes"].as_array().expect("nodes array");
    let edges = cyto["edges"].as_array().expect("edges array");
    let errors = cyto["errors"].as_array().expect("errors array");

    let fn_count = nodes
        .iter()
        .filter(|n| n["data"]["nodeType"] == "function")
        .count();
    let block_count = nodes
        .iter()
        .filter(|n| n["data"]["nodeType"] == "block")
        .count();
    let op_count = nodes
        .iter()
        .filter(|n| n["data"]["nodeType"] == "op")
        .count();

    assert_eq!(fn_count, 1, "expected 1 function");
    assert_eq!(block_count, 1, "expected 1 block");
    assert_eq!(op_count, 5, "expected 5 op nodes");
    assert_eq!(edges.len(), 4, "expected 4 edges");
    assert!(errors.is_empty(), "expected no errors");
}

#[test]
fn test_graph_to_cytoscape_fibonacci_fixture() {
    let graph = load_graph("fibonacci.jsonld");
    let cyto = graph_to_cytoscape(&graph);

    let nodes = cyto["nodes"].as_array().expect("nodes array");
    let edges = cyto["edges"].as_array().expect("edges array");

    let fn_count = nodes
        .iter()
        .filter(|n| n["data"]["nodeType"] == "function")
        .count();
    let block_count = nodes
        .iter()
        .filter(|n| n["data"]["nodeType"] == "block")
        .count();
    let op_count = nodes
        .iter()
        .filter(|n| n["data"]["nodeType"] == "op")
        .count();

    assert_eq!(fn_count, 2, "expected 2 functions (fib + main)");
    assert!(block_count >= 6, "expected 6+ blocks, got {block_count}");
    assert!(op_count >= 24, "expected 24+ ops, got {op_count}");
    assert!(edges.len() >= 20, "expected 20+ edges, got {}", edges.len());
}

#[test]
fn test_graph_to_cytoscape_error_on_invalid() {
    let result = error_json(vec!["Parse error: invalid JSON".to_string()]);

    assert!(
        result["nodes"].as_array().expect("nodes").is_empty(),
        "nodes should be empty on error"
    );
    assert!(
        result["edges"].as_array().expect("edges").is_empty(),
        "edges should be empty on error"
    );
    let errors = result["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].as_str().unwrap().contains("Parse error"));
}

#[test]
fn test_cytoscape_node_classes() {
    let graph = load_graph("add.jsonld");
    let cyto = graph_to_cytoscape(&graph);
    let nodes = cyto["nodes"].as_array().expect("nodes array");

    let op_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| n["data"]["nodeType"] == "op")
        .collect();

    for node in &op_nodes {
        let op_type = node["data"]["opType"].as_str().expect("opType");
        let classes = node["classes"].as_str().expect("classes");

        let expected_class = match op_type {
            "Const" | "ConstF64" | "ConstBool" => "op-const",
            "Add" | "Sub" | "Mul" | "Div" => "op-arithmetic",
            "Compare" => "op-compare",
            "Branch" => "op-control",
            "Call" => "op-call",
            "Load" | "Store" => "op-memory",
            "Print" | "Return" => "op-io",
            other => panic!("unexpected op type: {other}"),
        };
        assert_eq!(classes, expected_class, "wrong class for op {op_type}");
    }
}

#[test]
fn test_cytoscape_edge_labels() {
    let graph = load_graph("add.jsonld");
    let cyto = graph_to_cytoscape(&graph);
    let edges = cyto["edges"].as_array().expect("edges array");

    for edge in edges {
        let edge_type = edge["data"]["edgeType"].as_str().expect("edgeType");
        let label = edge["data"]["label"].as_str().expect("label");

        let expected_label = match edge_type {
            "Left" => "left",
            "Right" => "right",
            "Operand" => "operand",
            "Condition" => "condition",
            "TrueBlock" => "true",
            "FalseBlock" => "false",
            "Arg" => {
                assert!(
                    label.starts_with("arg["),
                    "Arg label should start with arg["
                );
                continue;
            }
            other => panic!("unexpected edge type: {other}"),
        };
        assert_eq!(label, expected_label, "wrong label for edge {edge_type}");
    }
}

// ---------------------------------------------------------------------------
// HTTP server tests (ephemeral port)
// ---------------------------------------------------------------------------

async fn start_test_server() -> (u16, duumbi::web::server::AppState) {
    use duumbi::web::serialize::graph_to_cytoscape;
    use duumbi::web::server::AppState;

    let graph = load_graph("add.jsonld");
    let initial = graph_to_cytoscape(&graph);
    let state = AppState::new(initial, false);
    let state_clone = state.clone();

    // Bind to port 0 to get an ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let port = listener.local_addr().expect("local_addr").port();

    let router = duumbi::web::server::build_router(state_clone.clone());

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("server error");
    });

    (port, state)
}

#[tokio::test]
async fn test_server_starts_and_serves_index() {
    let (port, _state) = start_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.expect("body");
    assert!(
        body.contains("duumbi"),
        "index.html should contain 'duumbi'"
    );
    assert!(body.contains("<html"), "response should be HTML");
}

#[tokio::test]
async fn test_api_graph_endpoint() {
    let (port, _state) = start_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{port}/api/graph"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.contains("application/json"), "should be JSON, got: {ct}");

    let body: serde_json::Value = resp.json().await.expect("JSON body");
    assert!(body.get("nodes").is_some(), "response should have nodes");
    assert!(body.get("edges").is_some(), "response should have edges");
}

#[tokio::test]
async fn test_websocket_connection() {
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    let (port, _state) = start_test_server().await;

    // Connect to WebSocket
    let url = format!("ws://127.0.0.1:{port}/ws");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("WebSocket connect failed");

    // Should receive graph_update immediately on connect
    let msg = ws_stream
        .next()
        .await
        .expect("no message received")
        .expect("WebSocket error");

    let text = match msg {
        WsMessage::Text(t) => t.to_string(),
        other => panic!("expected text message, got: {other:?}"),
    };

    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("message should be valid JSON");

    assert_eq!(
        parsed["type"], "graph_update",
        "message type should be graph_update"
    );
    assert!(parsed["data"].is_object(), "data should be an object");
    assert!(
        parsed["timestamp"].is_number(),
        "timestamp should be a number"
    );
}
