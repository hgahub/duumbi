//! DUUMBI Studio — browser-based developer cockpit.
//!
//! Provides a Leptos-based web application with C4 drill-down graph
//! visualization, AI chat, intent management, and search. Built on
//! `leptos_axum` for SSR + hydration.

pub mod app;
pub mod components;
pub mod layout;
pub mod server_fns;
pub mod state;
pub mod theme;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}

/// Starts the DUUMBI Studio SSR server.
///
/// Sets up `leptos_axum` routing with workspace context, serves the Leptos app
/// with SSR, and provides static asset endpoints.
#[cfg(feature = "ssr")]
pub async fn start_server(port: u16, workspace: std::path::PathBuf) -> anyhow::Result<()> {
    use std::net::SocketAddr;
    use std::sync::Arc;

    use axum::{Router, routing::get};
    use leptos::config::LeptosOptions;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use tokio::sync::RwLock;
    use tower_http::cors::CorsLayer;

    let workspace_ctx = Arc::new(RwLock::new(server_fns::WorkspaceContext {
        root: workspace.clone(),
    }));

    // Pre-load initial data for SSR rendering.
    let initial_data = server_fns::load_initial_data(&workspace);

    // Leptos config: minimal setup for SSR
    let leptos_opts = LeptosOptions::builder()
        .output_name("duumbi-studio")
        .site_addr(SocketAddr::from(([127, 0, 0, 1], port)))
        .build();

    // Generate route list from the App component's routes
    let routes = generate_route_list(app::App);

    let leptos_opts_clone = leptos_opts.clone();
    let ws_ctx = workspace_ctx.clone();

    // JSON API routes for interactive navigation
    let api_ws = workspace_ctx.clone();
    let app = Router::new()
        // Serve the Studio CSS inline (embedded)
        .route("/studio.css", get(serve_studio_css))
        .route("/studio.js", get(serve_studio_js))
        // JSON API for graph data (used by client JS)
        .route(
            "/api/graph/{level}",
            get({
                let ws = api_ws.clone();
                move |path: axum::extract::Path<String>,
                      query: axum::extract::Query<std::collections::HashMap<String, String>>| {
                    let ws = ws.clone();
                    async move { api_graph(ws, path.0, query.0).await }
                }
            }),
        )
        // Register all Leptos routes (SSR), injecting workspace context
        .leptos_routes_with_context(
            &leptos_opts,
            routes,
            {
                let initial = initial_data.clone();
                move || {
                    let ws = ws_ctx.clone();
                    leptos::prelude::provide_context(ws);
                    leptos::prelude::provide_context(initial.clone());
                }
            },
            app::App,
        )
        .layer(CorsLayer::permissive())
        .with_state(leptos_opts_clone);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind port {port}: {e}"))?;

    eprintln!("DUUMBI Studio running at http://localhost:{port}");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("invariant: failed to install CTRL+C handler");
        })
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {e}"))?;

    eprintln!("Studio stopped.");
    Ok(())
}

/// JSON API handler for graph data at various C4 levels.
///
/// `GET /api/graph/context` → modules overview
/// `GET /api/graph/container?module=app/main` → functions in a module
/// `GET /api/graph/component?module=app/main&function=main` → blocks in a function
/// `GET /api/graph/code?module=app/main&function=main&block=entry` → ops in a block
#[cfg(feature = "ssr")]
async fn api_graph(
    ws: std::sync::Arc<tokio::sync::RwLock<server_fns::WorkspaceContext>>,
    level: String,
    params: std::collections::HashMap<String, String>,
) -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    let ws = ws.read().await;
    let layout_type = params
        .get("layout")
        .map(|s| s.as_str())
        .unwrap_or("hierarchical");

    let result = match level.as_str() {
        "context" => {
            let data = server_fns::load_initial_data(&ws.root);
            let json = layout_to_json_with(&data.graph, layout_type);
            let mut obj = json;
            obj["modules"] = serde_json::json!(data.modules);
            Ok(obj)
        }
        "container" => {
            let module = params.get("module").cloned().unwrap_or_default();
            load_module_graph_with(&ws.root, &module, layout_type)
        }
        "component" => {
            let module = params.get("module").cloned().unwrap_or_default();
            let function = params.get("function").cloned().unwrap_or_default();
            load_function_blocks_with(&ws.root, &module, &function, layout_type)
        }
        "code" => {
            let module = params.get("module").cloned().unwrap_or_default();
            let function = params.get("function").cloned().unwrap_or_default();
            let block = params.get("block").cloned().unwrap_or_default();
            load_block_ops_with(&ws.root, &module, &function, &block, layout_type)
        }
        _ => Err(format!("Unknown level: {level}")),
    };

    match result {
        Ok(json) => (
            [(http::header::CONTENT_TYPE, "application/json")],
            json.to_string(),
        )
            .into_response(),
        Err(e) => (
            http::StatusCode::BAD_REQUEST,
            [(http::header::CONTENT_TYPE, "application/json")],
            serde_json::json!({"error": e}).to_string(),
        )
            .into_response(),
    }
}

/// Resolves graph file path for a module name.
#[cfg(feature = "ssr")]
fn resolve_module_path(root: &std::path::Path, module_name: &str) -> std::path::PathBuf {
    if module_name == "app/main" {
        root.join(".duumbi/graph/main.jsonld")
    } else {
        let parts: Vec<&str> = module_name.split('/').collect();
        if parts.first() == Some(&"stdlib") && parts.len() == 2 {
            root.join(format!(".duumbi/stdlib/{}", parts[1]))
                .join(".duumbi/graph/main.jsonld")
        } else {
            root.join(format!(".duumbi/graph/{module_name}.jsonld"))
        }
    }
}

/// Parses and builds a semantic graph from a module.
#[cfg(feature = "ssr")]
fn parse_module(
    root: &std::path::Path,
    module_name: &str,
) -> Result<duumbi::graph::SemanticGraph, String> {
    let path = resolve_module_path(root, module_name);
    let source =
        std::fs::read_to_string(&path).map_err(|e| format!("Read {}: {e}", path.display()))?;
    let ast = duumbi::parser::parse_jsonld(&source).map_err(|e| format!("Parse: {e}"))?;
    duumbi::graph::builder::build_graph(&ast).map_err(|e| format!("Graph: {e:?}"))
}

/// Runs layout on graph data and returns JSON with nodes, edges, bbox.
/// `layout_type`: "hierarchical" (default), "horizontal", "radial"
#[cfg(feature = "ssr")]
fn layout_to_json_with(gd: &state::GraphData, layout_type: &str) -> serde_json::Value {
    let (mut layout_nodes, bbox) = match layout_type {
        "horizontal" => layout::compute_layout_horizontal(gd),
        "radial" => layout::compute_layout_radial(gd),
        _ => layout::compute_layout(gd),
    };

    // Snap nodes to 24px grid
    for node in &mut layout_nodes {
        node.x = (node.x / 24.0).round() * 24.0;
        node.y = (node.y / 24.0).round() * 24.0;
    }

    let layout_edges = layout::edge_routing::route_edges(&gd.edges, &layout_nodes);
    serde_json::json!({
        "nodes": layout_nodes,
        "edges": layout_edges,
        "bbox": { "min_x": bbox.min_x, "min_y": bbox.min_y, "max_x": bbox.max_x, "max_y": bbox.max_y }
    })
}

/// Loads a module's function graph with a specific layout type.
#[cfg(feature = "ssr")]
fn load_module_graph_with(
    root: &std::path::Path,
    module_name: &str,
    layout_type: &str,
) -> Result<serde_json::Value, String> {
    let graph = parse_module(root, module_name)?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for func in &graph.functions {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect();
        nodes.push(state::GraphNode {
            id: func.name.to_string(),
            label: format!(
                "{}({}) -> {}",
                func.name,
                params.join(", "),
                func.return_type
            ),
            node_type: "function".to_string(),
            badge: Some(format!("{} blocks", func.blocks.len())),
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 60.0,
        });
    }

    for func in &graph.functions {
        for block in &func.blocks {
            for &node_idx in &block.nodes {
                let node = &graph.graph[node_idx];
                if let duumbi::types::Op::Call { function, .. } = &node.op {
                    edges.push(state::GraphEdge {
                        id: format!("call:{}:{}", func.name, node.id),
                        source: func.name.to_string(),
                        target: function.to_string(),
                        label: "calls".to_string(),
                        edge_type: "call".to_string(),
                    });
                }
            }
        }
    }

    let gd = state::GraphData { nodes, edges };
    Ok(layout_to_json_with(&gd, layout_type))
}

/// Loads blocks within a function with a specific layout type.
#[cfg(feature = "ssr")]
fn load_function_blocks_with(
    root: &std::path::Path,
    module_name: &str,
    function_name: &str,
    layout_type: &str,
) -> Result<serde_json::Value, String> {
    let graph = parse_module(root, module_name)?;
    let func = graph
        .functions
        .iter()
        .find(|f| f.name.0 == function_name)
        .ok_or_else(|| format!("Function '{function_name}' not found"))?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for block in &func.blocks {
        nodes.push(state::GraphNode {
            id: block.label.to_string(),
            label: block.label.to_string(),
            node_type: "block".to_string(),
            badge: Some(format!("{} ops", block.nodes.len())),
            x: 0.0,
            y: 0.0,
            width: 160.0,
            height: 50.0,
        });
    }

    for block in &func.blocks {
        for &node_idx in &block.nodes {
            let node = &graph.graph[node_idx];
            if let Some((true_block, false_block)) = graph.branch_targets.get(&node.id) {
                edges.push(state::GraphEdge {
                    id: format!("branch:{}:true", block.label),
                    source: block.label.to_string(),
                    target: true_block.clone(),
                    label: "true".to_string(),
                    edge_type: "branch_true".to_string(),
                });
                edges.push(state::GraphEdge {
                    id: format!("branch:{}:false", block.label),
                    source: block.label.to_string(),
                    target: false_block.clone(),
                    label: "false".to_string(),
                    edge_type: "branch_false".to_string(),
                });
            }
        }
    }

    let gd = state::GraphData { nodes, edges };
    Ok(layout_to_json_with(&gd, layout_type))
}

/// Loads ops within a block with a specific layout type.
#[cfg(feature = "ssr")]
fn load_block_ops_with(
    root: &std::path::Path,
    module_name: &str,
    function_name: &str,
    block_label: &str,
    layout_type: &str,
) -> Result<serde_json::Value, String> {
    let graph = parse_module(root, module_name)?;
    let func = graph
        .functions
        .iter()
        .find(|f| f.name.0 == function_name)
        .ok_or_else(|| format!("Function '{function_name}' not found"))?;
    let block = func
        .blocks
        .iter()
        .find(|b| b.label.0 == block_label)
        .ok_or_else(|| format!("Block '{block_label}' not found"))?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let block_node_ids: Vec<String> = block
        .nodes
        .iter()
        .map(|&idx| graph.graph[idx].id.to_string())
        .collect();

    let mut has_incoming: std::collections::HashSet<String> = std::collections::HashSet::new();

    for &node_idx in &block.nodes {
        let node = &graph.graph[node_idx];
        let result_type = node
            .result_type
            .map_or("void".to_string(), |t| t.to_string());

        let op_type = server_fns::op_type_name_str(&node.op);
        let is_first = block.nodes.first() == Some(&node_idx);
        let is_exit = matches!(op_type, "Return" | "Branch");
        let node_type = if is_first {
            format!("{op_type} entry")
        } else if is_exit {
            format!("{op_type} exit")
        } else {
            op_type.to_string()
        };

        nodes.push(state::GraphNode {
            id: node.id.to_string(),
            label: node.op.to_string(),
            node_type,
            badge: Some(result_type),
            x: 0.0,
            y: 0.0,
            width: 140.0,
            height: 45.0,
        });

        use petgraph::visit::EdgeRef;
        for edge_ref in graph
            .graph
            .edges_directed(node_idx, petgraph::Direction::Incoming)
        {
            let source_node = &graph.graph[edge_ref.source()];
            if block_node_ids.contains(&source_node.id.to_string()) {
                let (label, edge_type) = server_fns::edge_label_pair(edge_ref.weight());
                edges.push(state::GraphEdge {
                    id: format!("e:{}:{}", source_node.id, node.id),
                    source: source_node.id.to_string(),
                    target: node.id.to_string(),
                    label: label.to_string(),
                    edge_type: edge_type.to_string(),
                });
                has_incoming.insert(node.id.to_string());
            }
        }
    }

    for i in 1..block_node_ids.len() {
        let cur_id = &block_node_ids[i];
        if !has_incoming.contains(cur_id) {
            let prev_id = &block_node_ids[i - 1];
            edges.push(state::GraphEdge {
                id: format!("seq:{prev_id}:{cur_id}"),
                source: prev_id.clone(),
                target: cur_id.clone(),
                label: "seq".to_string(),
                edge_type: "sequence".to_string(),
            });
        }
    }

    let gd = state::GraphData { nodes, edges };
    Ok(layout_to_json_with(&gd, layout_type))
}

/// Serves the embedded Studio CSS.
#[cfg(feature = "ssr")]
async fn serve_studio_css() -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    static STUDIO_CSS: &str = include_str!("style/studio.css");
    (
        [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("text/css"),
        )],
        STUDIO_CSS,
    )
        .into_response()
}

/// Serves the inline Studio JavaScript for interactivity.
#[cfg(feature = "ssr")]
async fn serve_studio_js() -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    static STUDIO_JS: &str = include_str!("script/studio.js");
    (
        [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/javascript"),
        )],
        STUDIO_JS,
    )
        .into_response()
}
