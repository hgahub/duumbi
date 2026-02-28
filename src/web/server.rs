//! Axum HTTP server for the web visualizer.
//!
//! Serves embedded frontend assets, provides a JSON API for the current graph
//! state, and handles WebSocket upgrades for live sync. In `--dev` mode, assets
//! are served from the filesystem for rapid iteration.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Router, http};
use tokio::sync::{RwLock, broadcast};
use tower_http::cors::CorsLayer;

/// Shared application state passed to all route handlers.
#[derive(Clone)]
pub struct AppState {
    /// Current Cytoscape.js graph JSON (RwLock: watcher writes, handlers read).
    pub current_graph: Arc<RwLock<serde_json::Value>>,
    /// Broadcast channel for sending graph updates to WebSocket clients.
    pub tx: broadcast::Sender<String>,
    /// When true, assets are served from the filesystem under `src/web/assets/`.
    pub dev_mode: bool,
}

impl AppState {
    /// Creates a new `AppState` with a broadcast channel of capacity 16.
    #[must_use]
    pub fn new(initial_graph: serde_json::Value, dev_mode: bool) -> Self {
        let (tx, _) = broadcast::channel(16);
        Self {
            current_graph: Arc::new(RwLock::new(initial_graph)),
            tx,
            dev_mode,
        }
    }
}

// --- Embedded assets (compiled into the binary) ---
static INDEX_HTML: &str = include_str!("assets/index.html");
static STYLE_CSS: &str = include_str!("assets/style.css");
static APP_JS: &str = include_str!("assets/app.js");
static CYTOSCAPE_JS: &str = include_str!("assets/cytoscape.min.js");
static DAGRE_JS: &str = include_str!("assets/dagre.min.js");
static CYTOSCAPE_DAGRE_JS: &str = include_str!("assets/cytoscape-dagre.min.js");

/// Builds the axum router with all routes wired up.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_style))
        .route("/app.js", get(serve_app_js))
        .route("/cytoscape.min.js", get(serve_cytoscape))
        .route("/dagre.min.js", get(serve_dagre))
        .route("/cytoscape-dagre.min.js", get(serve_cytoscape_dagre))
        .route("/api/graph", get(api_graph))
        .route("/ws", get(crate::web::ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Starts the HTTP server on the given port and returns when the server exits.
///
/// Prints the URL to stderr once the server is ready. Shuts down gracefully
/// on CTRL+C / SIGTERM.
pub async fn run_server(port: u16, state: AppState) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let router = build_router(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to port {port}: {e}"))?;

    eprintln!("Visualizer running at http://localhost:{port}");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {e}"))?;

    eprintln!("Visualizer stopped.");
    Ok(())
}

/// Returns the `PathBuf` for a named asset in dev mode.
fn dev_asset_path(name: &str) -> PathBuf {
    PathBuf::from("src/web/assets").join(name)
}

/// Reads an asset file from disk in dev mode.
async fn read_dev_asset(name: &str) -> anyhow::Result<String> {
    let path = dev_asset_path(name);
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read dev asset '{}': {e}", path.display()))
}

// --- Route handlers ---

/// Serves `index.html`.
async fn serve_index(State(state): State<AppState>) -> Response {
    let body = if state.dev_mode {
        match read_dev_asset("index.html").await {
            Ok(s) => s,
            Err(e) => return server_error(e.to_string()),
        }
    } else {
        INDEX_HTML.to_string()
    };
    Html(body).into_response()
}

/// Serves `style.css`.
async fn serve_style(State(state): State<AppState>) -> Response {
    let body = if state.dev_mode {
        match read_dev_asset("style.css").await {
            Ok(s) => s,
            Err(e) => return server_error(e.to_string()),
        }
    } else {
        STYLE_CSS.to_string()
    };
    css_response(body)
}

/// Serves `app.js`.
async fn serve_app_js(State(state): State<AppState>) -> Response {
    let body = if state.dev_mode {
        match read_dev_asset("app.js").await {
            Ok(s) => s,
            Err(e) => return server_error(e.to_string()),
        }
    } else {
        APP_JS.to_string()
    };
    js_response(body)
}

/// Serves `cytoscape.min.js`.
async fn serve_cytoscape(State(_state): State<AppState>) -> Response {
    js_response(CYTOSCAPE_JS.to_string())
}

/// Serves `dagre.min.js`.
async fn serve_dagre(State(_state): State<AppState>) -> Response {
    js_response(DAGRE_JS.to_string())
}

/// Serves `cytoscape-dagre.min.js`.
async fn serve_cytoscape_dagre(State(_state): State<AppState>) -> Response {
    js_response(CYTOSCAPE_DAGRE_JS.to_string())
}

/// Returns the current graph as Cytoscape.js JSON.
async fn api_graph(State(state): State<AppState>) -> Response {
    let graph = state.current_graph.read().await;
    let body = serde_json::to_string(&*graph).unwrap_or_else(|_| "{}".to_string());
    (
        [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/json"),
        )],
        body,
    )
        .into_response()
}

// --- Helper response builders ---

fn css_response(body: String) -> Response {
    (
        [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("text/css"),
        )],
        body,
    )
        .into_response()
}

fn js_response(body: String) -> Response {
    (
        [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/javascript"),
        )],
        body,
    )
        .into_response()
}

fn server_error(msg: String) -> Response {
    (http::StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
}

// --- Graceful shutdown ---

async fn shutdown_signal() {
    let () = tokio::signal::ctrl_c()
        .await
        .expect("invariant: failed to install CTRL+C handler");
}
