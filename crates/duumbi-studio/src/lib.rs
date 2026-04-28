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
pub mod ws;

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
    let chat_ws = workspace_ctx.clone();
    let app = Router::new()
        // WebSocket chat endpoint (streaming LLM responses)
        .route(
            "/ws/chat",
            get({
                let ws = chat_ws;
                move |upgrade: axum::extract::ws::WebSocketUpgrade| {
                    let ws = ws.clone();
                    async move { upgrade.on_upgrade(move |socket| ws::handle_chat_ws(socket, ws)) }
                }
            }),
        )
        // Serve the Studio CSS inline (embedded)
        .route("/studio.css", get(serve_studio_css))
        .route("/studio.js", get(serve_studio_js))
        // Settings API: providers + env check
        .route(
            "/api/settings/providers",
            get({
                let ws = api_ws.clone();
                move || {
                    let ws = ws.clone();
                    async move { api_get_providers(ws).await }
                }
            })
            .post({
                let ws = api_ws.clone();
                move |body: axum::extract::Json<Vec<serde_json::Value>>| {
                    let ws = ws.clone();
                    async move { api_save_providers(ws, body.0).await }
                }
            }),
        )
        .route(
            "/api/settings/check-env",
            get({
                move |query: axum::extract::Query<std::collections::HashMap<String, String>>| {
                    async move { api_check_env(query.0) }
                }
            }),
        )
        // Agent templates API (delegates to shared helper in server_fns).
        .route(
            "/api/agent_templates",
            get(|| async {
                let infos = crate::server_fns::build_agent_template_infos();
                axum::Json(infos)
            }),
        )
        // Intent API: create + detail
        .route(
            "/api/intent/create",
            axum::routing::post({
                let ws = api_ws.clone();
                move |body: axum::extract::Json<std::collections::HashMap<String, String>>| {
                    let ws = ws.clone();
                    async move { api_create_intent(ws, body.0).await }
                }
            }),
        )
        .route(
            "/api/intent/{slug}",
            get({
                let ws = api_ws.clone();
                move |path: axum::extract::Path<String>| {
                    let ws = ws.clone();
                    async move { api_get_intent(ws, path.0).await }
                }
            }),
        )
        // JSON API for raw JSON-LD source (used by code view toggle)
        .route(
            "/api/source",
            get({
                let ws = api_ws.clone();
                move |query: axum::extract::Query<std::collections::HashMap<String, String>>| {
                    let ws = ws.clone();
                    async move { api_source(ws, query.0).await }
                }
            }),
        )
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
/// `GET /api/graph/context` → C4 Context (person + software system + stdout)
/// `GET /api/graph/container?module=app/main` → C4 Container (binary + runtime shim)
/// `GET /api/graph/component?module=app/main&function=main` → C4 Component (active vs dead code)
/// `GET /api/graph/code?module=app/main&function=main&block=entry` → Code level (ops)
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
            let module = params
                .get("module")
                .cloned()
                .unwrap_or("app/main".to_string());
            match parse_module(&ws.root, &module) {
                Ok(graph) => {
                    let gd = server_fns::build_c4_container_pub(&graph);
                    Ok(layout_to_json_with(&gd, layout_type))
                }
                Err(e) => Err(e),
            }
        }
        "component" => {
            let module = params
                .get("module")
                .cloned()
                .unwrap_or("app/main".to_string());
            match parse_module(&ws.root, &module) {
                Ok(graph) => {
                    let gd = server_fns::build_c4_component_pub(&graph);
                    Ok(layout_to_json_with(&gd, layout_type))
                }
                Err(e) => Err(e),
            }
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

/// JSON API handler for raw JSON-LD source of a module.
///
/// `GET /api/source?module=app/main` → `{"source": "<raw json-ld>", "path": "app/main"}`
#[cfg(feature = "ssr")]
async fn api_source(
    ws: std::sync::Arc<tokio::sync::RwLock<server_fns::WorkspaceContext>>,
    params: std::collections::HashMap<String, String>,
) -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    let ws = ws.read().await;
    let module = params
        .get("module")
        .cloned()
        .unwrap_or("app/main".to_string());

    let path = resolve_module_path(&ws.root, &module);
    match std::fs::read_to_string(&path) {
        Ok(source) => (
            [(http::header::CONTENT_TYPE, "application/json")],
            serde_json::json!({"source": source, "path": module}).to_string(),
        )
            .into_response(),
        Err(e) => (
            http::StatusCode::BAD_REQUEST,
            [(http::header::CONTENT_TYPE, "application/json")],
            serde_json::json!({"error": format!("Read {}: {e}", path.display())}).to_string(),
        )
            .into_response(),
    }
}

/// Resolves graph file path for a module name.
///
/// Validates the module name to prevent path traversal attacks — only
/// alphanumeric characters, `/`, `-`, and `_` are allowed.
#[cfg(feature = "ssr")]
fn resolve_module_path(root: &std::path::Path, module_name: &str) -> std::path::PathBuf {
    // Reject path traversal attempts and invalid characters
    if module_name.contains("..")
        || module_name.contains('\\')
        || !module_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '/' || c == '-' || c == '_')
    {
        // Return a path that won't exist, causing a clean "file not found" error
        return root.join(".duumbi/graph/__invalid__");
    }

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
///
/// Boundary nodes are excluded from the layout algorithm (they are purely
/// visual containers whose position is computed client-side from their
/// children). They are re-added to the output with zero position.
#[cfg(feature = "ssr")]
fn layout_to_json_with(gd: &state::GraphData, layout_type: &str) -> serde_json::Value {
    // Separate boundary nodes from layout-eligible nodes
    let boundary_nodes: Vec<_> = gd
        .nodes
        .iter()
        .filter(|n| n.node_type == "boundary")
        .cloned()
        .collect();
    let filtered = state::GraphData {
        nodes: gd
            .nodes
            .iter()
            .filter(|n| n.node_type != "boundary")
            .cloned()
            .collect(),
        edges: gd.edges.clone(),
    };

    let (mut layout_nodes, bbox) = match layout_type {
        "horizontal" => layout::compute_layout_horizontal(&filtered),
        "radial" => layout::compute_layout_radial(&filtered),
        _ => layout::compute_layout(&filtered),
    };

    // Add boundary nodes back with zero position (JS recomputes from children)
    for bn in &boundary_nodes {
        layout_nodes.push(layout::LayoutNode {
            id: bn.id.clone(),
            label: bn.label.clone(),
            node_type: bn.node_type.clone(),
            badge: bn.badge.clone(),
            x: 0.0,
            y: 0.0,
            width: bn.width,
            height: bn.height,
            layer: 0,
            order: 0,
        });
    }

    // Snap nodes to 12px grid (matching client-side GRID_BASE)
    for node in &mut layout_nodes {
        node.x = (node.x / 12.0).round() * 12.0;
        node.y = (node.y / 12.0).round() * 12.0;
    }

    let layout_edges = layout::edge_routing::route_edges(&gd.edges, &layout_nodes);
    serde_json::json!({
        "nodes": layout_nodes,
        "edges": layout_edges,
        "bbox": { "min_x": bbox.min_x, "min_y": bbox.min_y, "max_x": bbox.max_x, "max_y": bbox.max_y }
    })
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

    for &node_idx in &block.nodes {
        let node = &graph.graph[node_idx];
        let result_type = node
            .result_type
            .as_ref()
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

        // Branch nodes get TrueBlock/FalseBlock edges to show execution paths.
        // All other data-dependency edges (Left, Right, Operand, Arg) are
        // intentionally omitted — this view shows execution flow, not data flow.
        use duumbi::graph::GraphEdge as GE;
        use petgraph::visit::EdgeRef;
        for edge_ref in graph
            .graph
            .edges_directed(node_idx, petgraph::Direction::Outgoing)
        {
            let target_node = &graph.graph[edge_ref.target()];
            let target_id = target_node.id.to_string();
            match edge_ref.weight() {
                GE::TrueBlock | GE::FalseBlock => {
                    let (label, edge_type) = server_fns::edge_label_pair(edge_ref.weight());
                    if block_node_ids.contains(&target_id) {
                        edges.push(state::GraphEdge {
                            id: format!("e:{}:{}", node.id, target_id),
                            source: node.id.to_string(),
                            target: target_id,
                            label: label.to_string(),
                            edge_type: edge_type.to_string(),
                        });
                    }
                }
                _ => {} // data-dependency edges hidden in execution-flow view
            }
        }
    }

    // Sequential execution edges between consecutive ops in the block.
    // This represents the normal "fall-through" execution order.
    for i in 0..block_node_ids.len().saturating_sub(1) {
        let src = &block_node_ids[i];
        let tgt = &block_node_ids[i + 1];
        edges.push(state::GraphEdge {
            id: format!("seq:{src}:{tgt}"),
            source: src.clone(),
            target: tgt.clone(),
            label: String::new(),
            edge_type: "sequence".to_string(),
        });
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

/// Creates a new intent from a description via LLM.
///
/// `POST /api/intent/create` with `{"description": "Build a calculator..."}`
/// Returns `{"slug": "calculator", "description": "...", "status": "Pending"}`
#[cfg(feature = "ssr")]
async fn api_create_intent(
    ws: std::sync::Arc<tokio::sync::RwLock<server_fns::WorkspaceContext>>,
    body: std::collections::HashMap<String, String>,
) -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    let description = match body.get("description") {
        Some(d) if !d.trim().is_empty() => d.clone(),
        _ => {
            return (
                http::StatusCode::BAD_REQUEST,
                [(http::header::CONTENT_TYPE, "application/json")],
                r#"{"error":"description is required"}"#.to_string(),
            )
                .into_response();
        }
    };

    let ws = ws.read().await;
    let config = match duumbi::config::load_config(&ws.root) {
        Ok(c) => c,
        Err(e) => {
            return (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                [(http::header::CONTENT_TYPE, "application/json")],
                serde_json::json!({"error": format!("Config: {e}")}).to_string(),
            )
                .into_response();
        }
    };

    let providers = config.effective_providers();
    let client = match duumbi::agents::factory::create_provider_chain_for_global_access(&providers)
    {
        Ok(c) => c,
        Err(e) => {
            return (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                [(http::header::CONTENT_TYPE, "application/json")],
                serde_json::json!({"error": format!("Provider: {e}")}).to_string(),
            )
                .into_response();
        }
    };

    let mut log = Vec::new();
    match duumbi::intent::create::run_create(&*client, &ws.root, &description, true, &mut log).await
    {
        Ok(slug) => {
            let spec_desc = duumbi::intent::load_intent(&ws.root, &slug)
                .map(|s| s.intent)
                .unwrap_or_else(|_| description.clone());
            (
                [(http::header::CONTENT_TYPE, "application/json")],
                serde_json::json!({
                    "slug": slug,
                    "description": spec_desc,
                    "status": "Pending"
                })
                .to_string(),
            )
                .into_response()
        }
        Err(e) => (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            [(http::header::CONTENT_TYPE, "application/json")],
            serde_json::json!({"error": format!("Create failed: {e}")}).to_string(),
        )
            .into_response(),
    }
}

/// Returns intent details as markdown-formatted HTML.
///
/// `GET /api/intent/{slug}` → `{"slug": "...", "intent": "...", "status": "...",
///   "acceptance_criteria": [...], "test_cases": [...], "html": "<h1>..."}`
#[cfg(feature = "ssr")]
async fn api_get_intent(
    ws: std::sync::Arc<tokio::sync::RwLock<server_fns::WorkspaceContext>>,
    slug: String,
) -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    let ws = ws.read().await;
    match duumbi::intent::load_intent(&ws.root, &slug) {
        Ok(spec) => {
            // Build simple markdown-ish HTML for the md-panel
            let mut html = format!("<h1>{}</h1>\n", spec.intent);
            html.push_str(&format!(
                "<p style=\"color:#908c82\">Status: <code>{:?}</code></p>\n",
                spec.status
            ));

            if !spec.acceptance_criteria.is_empty() {
                html.push_str("<h2>Acceptance Criteria</h2>\n<ul>\n");
                for c in &spec.acceptance_criteria {
                    html.push_str(&format!("<li>{c}</li>\n"));
                }
                html.push_str("</ul>\n");
            }

            if !spec.test_cases.is_empty() {
                html.push_str("<h2>Test Cases</h2>\n<ul>\n");
                for tc in &spec.test_cases {
                    html.push_str(&format!(
                        "<li><code>{}</code>({}) → expected: {}</li>\n",
                        tc.function,
                        tc.args
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        tc.expected_return
                    ));
                }
                html.push_str("</ul>\n");
            }

            if !spec.modules.create.is_empty() {
                html.push_str("<h2>Modules to Create</h2>\n<ul>\n");
                for m in &spec.modules.create {
                    html.push_str(&format!("<li><code>{m}</code></li>\n"));
                }
                html.push_str("</ul>\n");
            }
            if !spec.modules.modify.is_empty() {
                html.push_str("<h2>Modules to Modify</h2>\n<ul>\n");
                for m in &spec.modules.modify {
                    html.push_str(&format!("<li><code>{m}</code></li>\n"));
                }
                html.push_str("</ul>\n");
            }

            (
                [(http::header::CONTENT_TYPE, "application/json")],
                serde_json::json!({
                    "slug": slug,
                    "intent": spec.intent,
                    "status": format!("{:?}", spec.status),
                    "html": html
                })
                .to_string(),
            )
                .into_response()
        }
        Err(e) => (
            http::StatusCode::NOT_FOUND,
            [(http::header::CONTENT_TYPE, "application/json")],
            serde_json::json!({"error": format!("Intent not found: {e}")}).to_string(),
        )
            .into_response(),
    }
}

/// Returns configured providers as JSON.
#[cfg(feature = "ssr")]
async fn api_get_providers(
    ws: std::sync::Arc<tokio::sync::RwLock<server_fns::WorkspaceContext>>,
) -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    let ws = ws.read().await;
    let config = match duumbi::config::load_config(&ws.root) {
        Ok(c) => c,
        Err(e) => {
            return (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                [(http::header::CONTENT_TYPE, "application/json")],
                serde_json::json!({"error": format!("{e}")}).to_string(),
            )
                .into_response();
        }
    };

    let providers: Vec<serde_json::Value> = config
        .effective_providers()
        .iter()
        .map(|p| {
            serde_json::json!({
                "provider": format!("{:?}", p.provider).to_lowercase(),
                "role": format!("{:?}", p.role).to_lowercase(),
                "api_key_env": p.api_key_env,
                "auth_token_env": p.auth_token_env,
                "base_url": p.base_url,
            })
        })
        .collect();

    (
        [(http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&providers).unwrap_or_else(|_| "[]".to_string()),
    )
        .into_response()
}

/// Saves providers to config.toml, preserving non-provider sections.
#[cfg(feature = "ssr")]
async fn api_save_providers(
    ws: std::sync::Arc<tokio::sync::RwLock<server_fns::WorkspaceContext>>,
    providers: Vec<serde_json::Value>,
) -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    let ws = ws.read().await;
    let config_path = ws.root.join(".duumbi").join("config.toml");

    let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
    let mut doc = existing
        .parse::<toml::Table>()
        .unwrap_or_else(|_| toml::Table::new());

    // Remove old provider sections
    doc.remove("providers");
    doc.remove("llm");

    // Build new [[providers]] array
    let mut toml_providers = Vec::new();
    for p in &providers {
        let mut entry = toml::Table::new();
        if let Some(s) = p.get("provider").and_then(|v| v.as_str()) {
            entry.insert("provider".into(), toml::Value::String(s.to_string()));
        }
        if let Some(s) = p.get("role").and_then(|v| v.as_str()) {
            entry.insert("role".into(), toml::Value::String(s.to_string()));
        }
        if let Some(s) = p.get("api_key_env").and_then(|v| v.as_str()) {
            entry.insert("api_key_env".into(), toml::Value::String(s.to_string()));
        }
        if let Some(s) = p
            .get("auth_token_env")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            entry.insert("auth_token_env".into(), toml::Value::String(s.to_string()));
        }
        if let Some(s) = p
            .get("base_url")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            entry.insert("base_url".into(), toml::Value::String(s.to_string()));
        }
        toml_providers.push(toml::Value::Table(entry));
    }

    doc.insert("providers".into(), toml::Value::Array(toml_providers));

    let toml_str = toml::to_string_pretty(&doc).unwrap_or_default();
    if let Err(e) = std::fs::write(&config_path, &toml_str) {
        return (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            [(http::header::CONTENT_TYPE, "application/json")],
            serde_json::json!({"error": format!("Write failed: {e}")}).to_string(),
        )
            .into_response();
    }

    (
        [(http::header::CONTENT_TYPE, "application/json")],
        r#"{"ok":true}"#.to_string(),
    )
        .into_response()
}

/// Checks if an environment variable is set (security-restricted to *_API_KEY / *_AUTH_TOKEN).
#[cfg(feature = "ssr")]
fn api_check_env(params: std::collections::HashMap<String, String>) -> axum::response::Response {
    use axum::http;
    use axum::response::IntoResponse;

    let var_name = params.get("var").cloned().unwrap_or_default();
    let allowed = var_name.ends_with("_API_KEY") || var_name.ends_with("_AUTH_TOKEN");
    let is_set = allowed && std::env::var(&var_name).is_ok();

    (
        [(http::header::CONTENT_TYPE, "application/json")],
        serde_json::json!({"var": var_name, "set": is_set}).to_string(),
    )
        .into_response()
}
