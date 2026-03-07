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

    // Leptos config: minimal setup for SSR
    let leptos_opts = LeptosOptions::builder()
        .output_name("duumbi-studio")
        .site_addr(SocketAddr::from(([127, 0, 0, 1], port)))
        .build();

    // Generate route list from the App component's routes
    let routes = generate_route_list(app::App);

    let leptos_opts_clone = leptos_opts.clone();
    let ws_ctx = workspace_ctx.clone();

    let app = Router::new()
        // Serve the Studio CSS inline (embedded)
        .route("/studio.css", get(serve_studio_css))
        // Register all Leptos routes (SSR), injecting workspace context
        .leptos_routes_with_context(
            &leptos_opts,
            routes,
            move || {
                let ws = ws_ctx.clone();
                leptos::prelude::provide_context(ws);
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
