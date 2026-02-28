//! File watcher for live graph reloading.
//!
//! Uses `notify-debouncer-mini` with a 200 ms debounce to watch `.jsonld`
//! files and trigger a graph rebuild. On change, the new Cytoscape.js JSON is
//! stored in `AppState.current_graph` and broadcast to all WebSocket clients.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use tokio::sync::mpsc;

use crate::graph::{builder, validator};
use crate::parser;
use crate::web::serialize::{error_json, graph_to_cytoscape};
use crate::web::server::AppState;
use crate::web::ws::build_graph_update_message;

/// Spawns a background task that watches `graph_path` for changes.
///
/// When the file changes, reloads and validates the graph, updates
/// `AppState.current_graph`, and broadcasts the update to all WebSocket
/// clients via `AppState.tx`.
///
/// The returned `JoinHandle` can be aborted to stop the watcher.
pub fn spawn_watcher(graph_path: PathBuf, state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_watcher(graph_path, state).await {
            tracing::error!("File watcher error: {e}");
        }
    })
}

/// Runs the file watcher loop.
async fn run_watcher(graph_path: PathBuf, state: AppState) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<()>(8);

    // Spawn the synchronous notify watcher on a dedicated OS thread.
    //
    // NOTE: `Handle::current()` cannot be called here because `std::thread::spawn`
    // creates a plain OS thread with no tokio context. Instead we use
    // `Sender::blocking_send()`, which is designed for exactly this use case:
    // sending from sync code into an async channel.
    let watch_path = graph_path.clone();
    let tx_for_thread = tx.clone();
    let _watcher_handle = std::thread::spawn(move || {
        let mut debouncer = new_debouncer(
            Duration::from_millis(200),
            move |result: DebounceEventResult| {
                if result.is_ok() {
                    // blocking_send is safe from a non-tokio thread
                    let _ = tx_for_thread.blocking_send(());
                }
            },
        )
        .expect("invariant: failed to create file watcher");

        debouncer
            .watcher()
            .watch(&watch_path, notify::RecursiveMode::NonRecursive)
            .expect("invariant: failed to watch graph path");

        // Park keeps the debouncer (and watcher) alive for the process lifetime.
        std::thread::park();
    });

    // Process change events
    while let Some(()) = rx.recv().await {
        let graph_json = reload_graph(&graph_path);

        // Update shared state
        {
            let mut guard = state.current_graph.write().await;
            *guard = graph_json.clone();
        }

        // Broadcast to WebSocket clients (ignore if no clients connected)
        let message = build_graph_update_message(&graph_json);
        let _ = state.tx.send(message);

        tracing::debug!("Graph reloaded from '{}'", graph_path.display());
    }

    Ok(())
}

/// Loads, parses, builds, and validates the graph at `path`.
///
/// Returns a Cytoscape.js JSON on success, or an error JSON if any step fails.
fn reload_graph(path: &Path) -> serde_json::Value {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return error_json(vec![format!("Failed to read file: {e}")]);
        }
    };

    let ast = match parser::parse_jsonld(&source) {
        Ok(ast) => ast,
        Err(e) => {
            return error_json(vec![format!("Parse error: {e}")]);
        }
    };

    let graph = match builder::build_graph(&ast) {
        Ok(g) => g,
        Err(errors) => {
            let msgs = errors.iter().map(|e| format!("Graph error: {e}")).collect();
            return error_json(msgs);
        }
    };

    let diagnostics = validator::validate(&graph);
    if !diagnostics.is_empty() {
        let msgs = diagnostics
            .iter()
            .map(|d| format!("Validation error: {d}"))
            .collect();
        return error_json(msgs);
    }

    graph_to_cytoscape(&graph)
}

/// Loads the initial graph synchronously (called before spawning the async watcher).
///
/// Returns the Cytoscape.js JSON or an error JSON if parsing fails.
#[must_use]
pub fn load_initial_graph(graph_path: &Path) -> serde_json::Value {
    reload_graph(graph_path)
}
