//! WebSocket chat handler for Studio.
//!
//! Provides `/ws/chat` endpoint that connects the Studio chat panel to
//! the LLM mutation pipeline with streaming responses.

#[cfg(feature = "ssr")]
use std::sync::Arc;

#[cfg(feature = "ssr")]
use axum::extract::ws::{Message, WebSocket};
#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use tokio::sync::RwLock;

#[cfg(feature = "ssr")]
use crate::server_fns::WorkspaceContext;

/// Incoming chat message from the client.
#[cfg(feature = "ssr")]
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // c4_level will be used for context-aware prompts in #478
struct ChatRequest {
    /// Message type (always "chat").
    #[serde(rename = "type")]
    msg_type: String,
    /// User's natural language message.
    message: String,
    /// Currently selected module (e.g., "app/main").
    #[serde(default)]
    module: Option<String>,
    /// Current C4 drill-down level.
    #[serde(default)]
    c4_level: Option<String>,
}

/// Streaming text chunk sent to the client.
#[cfg(feature = "ssr")]
#[derive(Serialize)]
struct ChunkFrame {
    #[serde(rename = "type")]
    msg_type: &'static str,
    text: String,
}

/// Mutation result sent to the client on success.
#[cfg(feature = "ssr")]
#[derive(Serialize)]
struct ResultFrame {
    #[serde(rename = "type")]
    msg_type: &'static str,
    ops_count: usize,
    changed_nodes: Vec<String>,
    refresh: bool,
}

/// Error frame sent to the client.
#[cfg(feature = "ssr")]
#[derive(Serialize)]
struct ErrorFrame {
    #[serde(rename = "type")]
    msg_type: &'static str,
    message: String,
}

/// Clarification request from the LLM.
#[cfg(feature = "ssr")]
#[derive(Serialize)]
struct ClarifyFrame {
    #[serde(rename = "type")]
    msg_type: &'static str,
    question: String,
}

/// Main chat loop: reads messages, runs mutation pipeline, streams responses.
///
/// Called from `lib.rs` after WebSocket upgrade.
#[cfg(feature = "ssr")]
pub async fn handle_chat_ws(mut socket: WebSocket, ctx: Arc<RwLock<WorkspaceContext>>) {
    use duumbi::agents::factory::create_provider_chain;
    use duumbi::agents::orchestrator::{self, MutationOutcome};
    use duumbi::config::load_config;
    use duumbi::context;
    use duumbi::session::PersistentTurn;

    let mut session_history: Vec<PersistentTurn> = Vec::new();

    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        let req: ChatRequest = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                let _ = send_error(&mut socket, &format!("Invalid message: {e}")).await;
                continue;
            }
        };

        if req.msg_type != "chat" {
            continue;
        }

        let workspace = ctx.read().await.root.clone();
        let module = req.module.as_deref().unwrap_or("app/main");

        // Load config and create provider chain.
        let config = match load_config(&workspace) {
            Ok(c) => c,
            Err(e) => {
                let _ = send_error(&mut socket, &format!("Config error: {e}")).await;
                continue;
            }
        };

        let providers = config.effective_providers();
        let client = match create_provider_chain(&providers) {
            Ok(c) => c,
            Err(e) => {
                let _ = send_error(&mut socket, &format!("No LLM provider configured: {e}")).await;
                continue;
            }
        };

        // Enrich prompt with workspace context.
        let enriched_message =
            match context::assemble_context(&req.message, &workspace, &session_history) {
                Ok(bundle) => bundle.enriched_message,
                Err(_) => req.message.clone(),
            };

        // Load the module's JSON-LD source.
        let module_path = resolve_module_path(&workspace, module);
        let source = match std::fs::read_to_string(&module_path) {
            Ok(s) => s,
            Err(e) => {
                let _ = send_error(&mut socket, &format!("Cannot read {module}: {e}")).await;
                continue;
            }
        };

        let source_value: serde_json::Value = match serde_json::from_str(&source) {
            Ok(v) => v,
            Err(e) => {
                let _ = send_error(&mut socket, &format!("Invalid JSON-LD: {e}")).await;
                continue;
            }
        };

        // Detect library mode: modules other than app/main skip Call validation.
        let library_mode = module != "app/main";

        // Stream mutation with text callback.
        // We use a channel to bridge the sync callback with async WS send.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let source_clone = source_value.clone();
        let enriched = enriched_message.clone();
        let mutation_handle = tokio::spawn(async move {
            orchestrator::mutate_streaming(
                client.as_ref(),
                &source_clone,
                &enriched,
                3,
                library_mode,
                move |chunk| {
                    let _ = tx.send(chunk.to_string());
                },
            )
            .await
        });

        // Forward streaming chunks to client.
        loop {
            tokio::select! {
                chunk = rx.recv() => {
                    match chunk {
                        Some(text) => {
                            let frame = ChunkFrame {
                                msg_type: "chunk",
                                text,
                            };
                            if let Ok(json) = serde_json::to_string(&frame)
                                && socket.send(Message::Text(json.into())).await.is_err()
                            {
                                return; // Client disconnected.
                            }
                        }
                        None => break, // Channel closed — mutation done.
                    }
                }
            }
        }

        // Get mutation result.
        let outcome = match mutation_handle.await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(e)) => {
                let _ = send_error(&mut socket, &format!("Mutation failed: {e}")).await;
                continue;
            }
            Err(e) => {
                let _ = send_error(&mut socket, &format!("Task error: {e}")).await;
                continue;
            }
        };

        match outcome {
            MutationOutcome::Success(result) => {
                // Write patched JSON-LD to disk.
                let patched_json =
                    serde_json::to_string_pretty(&result.patched).unwrap_or_default();
                if let Err(e) = std::fs::write(&module_path, &patched_json) {
                    let _ =
                        send_error(&mut socket, &format!("Failed to write {module}: {e}")).await;
                    continue;
                }

                // Record in session history.
                session_history.push(PersistentTurn {
                    request: req.message.clone(),
                    summary: format!("{} ops applied", result.ops_count),
                    timestamp: chrono::Utc::now(),
                    task_type: "studio_chat".to_string(),
                });

                // Send result frame.
                let frame = ResultFrame {
                    msg_type: "result",
                    ops_count: result.ops_count,
                    changed_nodes: Vec::new(),
                    refresh: true,
                };
                if let Ok(json) = serde_json::to_string(&frame) {
                    let _ = socket.send(Message::Text(json.into())).await;
                }
            }
            MutationOutcome::NeedsClarification(question) => {
                let frame = ClarifyFrame {
                    msg_type: "clarify",
                    question,
                };
                if let Ok(json) = serde_json::to_string(&frame) {
                    let _ = socket.send(Message::Text(json.into())).await;
                }
            }
        }
    }
}

/// Sends an error frame to the client.
#[cfg(feature = "ssr")]
async fn send_error(socket: &mut WebSocket, message: &str) -> Result<(), axum::Error> {
    let frame = ErrorFrame {
        msg_type: "error",
        message: message.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&frame) {
        socket.send(Message::Text(json.into())).await
    } else {
        Ok(())
    }
}

/// Resolves module name to JSON-LD file path.
///
/// Reuses the same path resolution logic as `lib.rs` to prevent divergence.
#[cfg(feature = "ssr")]
fn resolve_module_path(root: &std::path::Path, module_name: &str) -> std::path::PathBuf {
    if module_name.contains("..")
        || module_name.contains('\\')
        || !module_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '/' || c == '-' || c == '_')
    {
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
