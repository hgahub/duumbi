//! WebSocket handler for live graph updates.
//!
//! When a client connects, it immediately receives the current graph state.
//! Subsequently, all graph updates broadcast from the file watcher are
//! forwarded to the client. Disconnected clients are silently cleaned up.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;

use crate::web::server::AppState;

/// Axum route handler that upgrades an HTTP request to a WebSocket connection.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Manages a single WebSocket connection.
///
/// Sends the current graph immediately on connect, then subscribes to the
/// broadcast channel and forwards all subsequent updates. Exits when the
/// client disconnects or the server channel is closed.
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Send current graph on connect
    let initial = {
        let graph = state.current_graph.read().await;
        build_graph_update_message(&graph)
    };

    if socket.send(Message::Text(initial.into())).await.is_err() {
        return;
    }

    // Subscribe to future updates
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(update) => {
                        if socket.send(Message::Text(update.into())).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // Server shutting down
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Client lagging — send current state to resync
                        let graph = state.current_graph.read().await;
                        let update = build_graph_update_message(&graph);
                        drop(graph);
                        if socket.send(Message::Text(update.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
            // Handle pings / close from client side
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Wraps a Cytoscape.js graph JSON into the WebSocket envelope format.
///
/// ```json
/// { "type": "graph_update", "data": {...}, "timestamp": 1234567890 }
/// ```
pub fn build_graph_update_message(graph: &serde_json::Value) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());

    let msg = serde_json::json!({
        "type": "graph_update",
        "data": graph,
        "timestamp": timestamp
    });

    serde_json::to_string(&msg)
        .unwrap_or_else(|_| r#"{"type":"graph_update","data":{}}"#.to_string())
}
