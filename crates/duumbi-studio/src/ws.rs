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
struct ChatRequest {
    /// Message type (always "chat").
    #[serde(rename = "type")]
    msg_type: String,
    /// User's natural language message.
    message: String,
    /// Currently selected module (e.g., "app/main").
    #[serde(default)]
    module: Option<String>,
    /// Current C4 drill-down level (context/container/component/code).
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

    // Cache config and provider chain per connection (not per message).
    let workspace = ctx.read().await.root.clone();
    let config = match load_config(&workspace) {
        Ok(c) => c,
        Err(e) => {
            let _ = send_error(&mut socket, &format!("Config error: {e}")).await;
            return;
        }
    };
    let providers = config.effective_providers();
    let client: Arc<dyn duumbi::agents::LlmProvider> = match create_provider_chain(&providers) {
        Ok(c) => Arc::from(c),
        Err(e) => {
            let _ = send_error(&mut socket, &format!("No LLM provider configured: {e}")).await;
            return;
        }
    };

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

        let module = req.module.as_deref().unwrap_or("app/main");

        // Enrich prompt with workspace context, filtered by C4 depth.
        let enriched_message =
            match context::assemble_context(&req.message, &workspace, &session_history) {
                Ok(bundle) => {
                    filter_context_by_c4_level(&bundle.enriched_message, req.c4_level.as_deref())
                }
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
        // Bounded channel with backpressure (256 chunks buffer).
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(256);

        let source_clone = source_value.clone();
        let enriched = enriched_message.clone();
        let client_clone = Arc::clone(&client);
        let mutation_handle = tokio::spawn(async move {
            orchestrator::mutate_streaming(
                client_clone.as_ref(),
                &source_clone,
                &enriched,
                3,
                library_mode,
                move |chunk| {
                    let _ = tx.try_send(chunk.to_string());
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
                let patched_json = match serde_json::to_string_pretty(&result.patched) {
                    Ok(json) => json,
                    Err(e) => {
                        let _ = send_error(
                            &mut socket,
                            &format!("Failed to serialize patched module {module}: {e}"),
                        )
                        .await;
                        continue;
                    }
                };
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

                // Detect changed nodes by diffing @id sets.
                let old_ids = collect_ids(&source_value);
                let new_ids = collect_ids(&result.patched);
                let changed_nodes: Vec<String> = new_ids
                    .difference(&old_ids)
                    .chain(old_ids.difference(&new_ids))
                    .cloned()
                    .collect();

                // Send result frame.
                let frame = ResultFrame {
                    msg_type: "result",
                    ops_count: result.ops_count,
                    changed_nodes,
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

/// Filters the enriched context prompt based on the C4 drill-down level.
///
/// Higher levels (Context, Container) receive less context to keep prompts
/// focused and token-efficient. Lower levels (Component, Code) include the
/// full enriched message for precise mutations.
#[cfg(feature = "ssr")]
fn filter_context_by_c4_level(enriched: &str, c4_level: Option<&str>) -> String {
    match c4_level {
        Some("context") => {
            // Context level: keep only the first paragraph (workspace overview).
            enriched
                .split("\n\n")
                .next()
                .unwrap_or(enriched)
                .to_string()
        }
        Some("container") => {
            // Container level: keep first 2 paragraphs (overview + module structure).
            let paragraphs: Vec<&str> = enriched.split("\n\n").collect();
            paragraphs[..paragraphs.len().min(2)].join("\n\n")
        }
        // Component and Code levels: full context for precise mutations.
        _ => enriched.to_string(),
    }
}

/// Collects all `@id` string values from a JSON-LD value tree.
#[cfg(feature = "ssr")]
fn collect_ids(value: &serde_json::Value) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    collect_ids_recursive(value, &mut ids);
    ids
}

#[cfg(feature = "ssr")]
fn collect_ids_recursive(value: &serde_json::Value, ids: &mut std::collections::HashSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(id)) = map.get("@id") {
                ids.insert(id.clone());
            }
            for v in map.values() {
                collect_ids_recursive(v, ids);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_ids_recursive(v, ids);
            }
        }
        _ => {}
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
