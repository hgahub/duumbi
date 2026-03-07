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
    use leptos::prelude::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}

/// Starts the Studio server on the given port.
///
/// This is the main entry point called from `duumbi studio` CLI command.
/// It sets up the axum server with Leptos SSR, WebSocket live sync,
/// and all API endpoints.
#[cfg(feature = "ssr")]
pub async fn start_server(
    port: u16,
    workspace: std::path::PathBuf,
    dev: bool,
) -> anyhow::Result<()> {
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::LeptosRoutes;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let workspace_state = Arc::new(RwLock::new(server_fns::WorkspaceContext {
        root: workspace.clone(),
    }));

    // Build Leptos options
    let conf = leptos::config::LeptosOptions {
        site_addr: SocketAddr::from(([127, 0, 0, 1], port)),
        ..Default::default()
    };

    let routes = leptos_axum::generate_route_list(app::App);

    let app = Router::new()
        .leptos_routes(&conf, routes, {
            let ws = workspace_state.clone();
            move || {
                let ws = ws.clone();
                leptos::prelude::provide_context(ws);
                app::App()
            }
        })
        .with_state(conf);

    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to port {port}: {e}"))?;

    eprintln!("DUUMBI Studio running at http://localhost:{port}");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(async {
            let () = tokio::signal::ctrl_c()
                .await
                .expect("invariant: failed to install CTRL+C handler");
        })
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {e}"))?;

    eprintln!("Studio stopped.");
    Ok(())
}
