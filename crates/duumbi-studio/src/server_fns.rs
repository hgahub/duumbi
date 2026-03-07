//! Server functions for the Studio.
//!
//! These `#[server]` functions run on the server and are callable from
//! the client via Leptos RPC. They bridge the Studio UI to the duumbi
//! workspace: graph loading, building, chat, and intent management.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// GraphNode/GraphEdge are used inside #[server] fn bodies (ssr feature only)
#[allow(unused_imports)]
use crate::state::{GraphData, GraphEdge, GraphNode, IntentSummary};

/// Server-side workspace context, shared via Leptos context.
#[derive(Clone)]
pub struct WorkspaceContext {
    /// Root path of the duumbi workspace.
    pub root: std::path::PathBuf,
}

/// Workspace status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    /// Workspace name (directory name).
    pub name: String,
    /// Number of modules in the workspace.
    pub module_count: usize,
    /// List of module names.
    pub modules: Vec<String>,
}

/// Returns the graph data for the Context level (modules overview).
#[server]
pub async fn get_graph_context() -> Result<GraphData, ServerFnError> {
    use std::fs;

    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;
    let graph_dir = ws.root.join(".duumbi/graph");

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    if graph_dir.join("main.jsonld").exists() {
        nodes.push(GraphNode {
            id: "app/main".to_string(),
            label: "app/main".to_string(),
            node_type: "module".to_string(),
            badge: None,
            x: 0.0,
            y: 0.0,
            width: 180.0,
            height: 80.0,
        });
    }

    let config_path = ws.root.join(".duumbi/config.toml");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = content.parse::<toml::Table>() {
                if let Some(deps) = config.get("dependencies").and_then(|v| v.as_table()) {
                    for (name, _) in deps {
                        nodes.push(GraphNode {
                            id: name.clone(),
                            label: name.clone(),
                            node_type: "module".to_string(),
                            badge: None,
                            x: 0.0,
                            y: 0.0,
                            width: 180.0,
                            height: 80.0,
                        });
                        edges.push(GraphEdge {
                            id: format!("dep:{name}"),
                            source: "app/main".to_string(),
                            target: name.clone(),
                            label: "depends on".to_string(),
                            edge_type: "dependency".to_string(),
                        });
                    }
                }
            }
        }
    }

    let stdlib_dir = ws.root.join(".duumbi/stdlib");
    if stdlib_dir.exists() {
        if let Ok(entries) = fs::read_dir(&stdlib_dir) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|t| t.is_dir()) {
                    let name = format!("stdlib/{}", entry.file_name().to_string_lossy());
                    nodes.push(GraphNode {
                        id: name.clone(),
                        label: name.clone(),
                        node_type: "module".to_string(),
                        badge: None,
                        x: 0.0,
                        y: 0.0,
                        width: 180.0,
                        height: 80.0,
                    });
                    edges.push(GraphEdge {
                        id: format!("dep:{name}"),
                        source: "app/main".to_string(),
                        target: name,
                        label: "uses".to_string(),
                        edge_type: "dependency".to_string(),
                    });
                }
            }
        }
    }

    Ok(GraphData { nodes, edges })
}

/// Resolves the graph file path for a module name.
#[cfg(feature = "ssr")]
fn resolve_graph_path(root: &std::path::Path, module_name: &str) -> std::path::PathBuf {
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

/// Loads and builds a semantic graph from a module path.
#[cfg(feature = "ssr")]
fn load_graph(
    root: &std::path::Path,
    module_name: &str,
) -> Result<duumbi::graph::SemanticGraph, ServerFnError> {
    let graph_path = resolve_graph_path(root, module_name);
    let source = std::fs::read_to_string(&graph_path)
        .map_err(|e| ServerFnError::new(format!("Failed to read {}: {e}", graph_path.display())))?;
    let ast = duumbi::parser::parse_jsonld(&source)
        .map_err(|e| ServerFnError::new(format!("Parse error: {e}")))?;
    duumbi::graph::builder::build_graph(&ast)
        .map_err(|errors| ServerFnError::new(format!("Graph errors: {errors:?}")))
}

/// Returns graph data for the Container level (functions within a module).
#[server]
pub async fn get_module_detail(module_name: String) -> Result<GraphData, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;
    let graph = load_graph(&ws.root, &module_name)?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for func in &graph.functions {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect();

        nodes.push(GraphNode {
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
                    edges.push(GraphEdge {
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

    Ok(GraphData { nodes, edges })
}

/// Returns graph data for the Component level (blocks within a function).
#[server]
pub async fn get_function_detail(
    module_name: String,
    function_name: String,
) -> Result<GraphData, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;
    let graph = load_graph(&ws.root, &module_name)?;

    let func = graph
        .functions
        .iter()
        .find(|f| f.name.0 == function_name)
        .ok_or_else(|| ServerFnError::new(format!("Function '{function_name}' not found")))?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for block in &func.blocks {
        nodes.push(GraphNode {
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

    // Add branch edges between blocks
    for block in &func.blocks {
        for &node_idx in &block.nodes {
            let node = &graph.graph[node_idx];
            if let Some((true_block, false_block)) = graph.branch_targets.get(&node.id) {
                edges.push(GraphEdge {
                    id: format!("branch:{}:true", block.label),
                    source: block.label.to_string(),
                    target: true_block.clone(),
                    label: "true".to_string(),
                    edge_type: "branch_true".to_string(),
                });
                edges.push(GraphEdge {
                    id: format!("branch:{}:false", block.label),
                    source: block.label.to_string(),
                    target: false_block.clone(),
                    label: "false".to_string(),
                    edge_type: "branch_false".to_string(),
                });
            }
        }
    }

    Ok(GraphData { nodes, edges })
}

/// Returns graph data for the Code level (ops within a block).
#[server]
pub async fn get_block_ops(
    module_name: String,
    function_name: String,
    block_label: String,
) -> Result<GraphData, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;
    let graph = load_graph(&ws.root, &module_name)?;

    let func = graph
        .functions
        .iter()
        .find(|f| f.name.0 == function_name)
        .ok_or_else(|| ServerFnError::new(format!("Function '{function_name}' not found")))?;

    let block = func
        .blocks
        .iter()
        .find(|b| b.label.0 == block_label)
        .ok_or_else(|| ServerFnError::new(format!("Block '{block_label}' not found")))?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for &node_idx in &block.nodes {
        let node = &graph.graph[node_idx];
        let op_type = op_type_name(&node.op);
        let result_type = node
            .result_type
            .map_or("void".to_string(), |t| t.to_string());

        nodes.push(GraphNode {
            id: node.id.to_string(),
            label: node.op.to_string(),
            node_type: op_type.to_string(),
            badge: Some(result_type),
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 40.0,
        });

        use petgraph::visit::EdgeRef;
        for edge_ref in graph
            .graph
            .edges_directed(node_idx, petgraph::Direction::Incoming)
        {
            let source_node = &graph.graph[edge_ref.source()];
            let (label, edge_type) = edge_label_str(edge_ref.weight());

            edges.push(GraphEdge {
                id: format!("e:{}:{}", source_node.id, node.id),
                source: source_node.id.to_string(),
                target: node.id.to_string(),
                label: label.to_string(),
                edge_type: edge_type.to_string(),
            });
        }
    }

    Ok(GraphData { nodes, edges })
}

/// Returns workspace status information.
#[server]
pub async fn get_workspace_status() -> Result<WorkspaceStatus, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;

    let name = ws
        .root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "workspace".to_string());

    let mut modules = Vec::new();

    if ws.root.join(".duumbi/graph/main.jsonld").exists() {
        modules.push("app/main".to_string());
    }

    let stdlib_dir = ws.root.join(".duumbi/stdlib");
    if stdlib_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&stdlib_dir) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|t| t.is_dir()) {
                    modules.push(format!("stdlib/{}", entry.file_name().to_string_lossy()));
                }
            }
        }
    }

    let module_count = modules.len();
    Ok(WorkspaceStatus {
        name,
        module_count,
        modules,
    })
}

/// Triggers a workspace build.
#[server]
pub async fn trigger_build() -> Result<String, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;

    let output = tokio::process::Command::new("cargo")
        .args(["run", "--", "build"])
        .current_dir(&ws.root)
        .output()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to run build: {e}")))?;

    if output.status.success() {
        Ok("Build successful".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ServerFnError::new(format!("Build failed: {stderr}")))
    }
}

/// Returns the list of intents in the workspace.
#[server]
pub async fn get_intents() -> Result<Vec<IntentSummary>, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;

    let mut intents = Vec::new();

    let intents_dir = ws.root.join(".duumbi/intents");
    if intents_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&intents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "yaml") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let slug = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();

                        if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                            let description = value["intent"].as_str().unwrap_or(&slug).to_string();
                            let status = value["status"].as_str().unwrap_or("Unknown").to_string();

                            intents.push(IntentSummary {
                                slug,
                                description,
                                status,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(intents)
}

/// Returns the short type name for an Op.
#[cfg(feature = "ssr")]
fn op_type_name(op: &duumbi::types::Op) -> &'static str {
    use duumbi::types::Op;
    match op {
        Op::Const(_) => "Const",
        Op::ConstF64(_) => "ConstF64",
        Op::ConstBool(_) => "ConstBool",
        Op::Add => "Add",
        Op::Sub => "Sub",
        Op::Mul => "Mul",
        Op::Div => "Div",
        Op::Compare(_) => "Compare",
        Op::Branch => "Branch",
        Op::Call { .. } => "Call",
        Op::Load { .. } => "Load",
        Op::Store { .. } => "Store",
        Op::Print => "Print",
        Op::Return => "Return",
    }
}

/// Sends a chat message to the LLM and applies any graph mutation.
///
/// Loads the workspace config, calls `orchestrator::mutate`, applies the patch
/// if accepted, and returns the AI's response text along with changed node ids.
#[server]
pub async fn send_chat_message(
    message: String,
) -> Result<crate::state::ChatResponse, ServerFnError> {
    use std::fs;

    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;

    // Load config
    let config = duumbi::config::load_config(&ws.root)
        .map_err(|e| ServerFnError::new(format!("Config error: {e}")))?;

    let llm_cfg = config
        .llm
        .ok_or_else(|| ServerFnError::new("No [llm] section in config.toml".to_string()))?;

    let api_key = llm_cfg
        .resolve_api_key()
        .map_err(|e| ServerFnError::new(format!("API key error: {e}")))?;

    let client = match llm_cfg.provider {
        duumbi::config::LlmProvider::Anthropic => {
            duumbi::agents::LlmClient::anthropic(&llm_cfg.model, api_key)
        }
        duumbi::config::LlmProvider::OpenAI => {
            duumbi::agents::LlmClient::openai(&llm_cfg.model, api_key)
        }
    };

    let graph_path = ws.root.join(".duumbi/graph/main.jsonld");
    let source_str = fs::read_to_string(&graph_path)
        .map_err(|e| ServerFnError::new(format!("Failed to read graph: {e}")))?;
    let source: serde_json::Value = serde_json::from_str(&source_str)
        .map_err(|e| ServerFnError::new(format!("Failed to parse graph: {e}")))?;

    let result = duumbi::agents::orchestrator::mutate(&client, &source, &message, 3)
        .await
        .map_err(|e| ServerFnError::new(format!("LLM error: {e}")))?;

    let diff = duumbi::agents::orchestrator::describe_changes(&source, &result.patched);

    // Save snapshot and write patched graph
    duumbi::snapshot::save_snapshot(&ws.root, &source_str)
        .map_err(|e| ServerFnError::new(format!("Snapshot error: {e}")))?;

    let patched_str = serde_json::to_string_pretty(&result.patched)
        .map_err(|e| ServerFnError::new(format!("Serialize error: {e}")))?;

    fs::write(&graph_path, patched_str)
        .map_err(|e| ServerFnError::new(format!("Write error: {e}")))?;

    Ok(crate::state::ChatResponse {
        text: format!("Applied {} change(s):\n{}", result.ops_count, diff),
        changed_node_ids: Vec::new(), // TODO: extract from patched nodes
    })
}

/// Returns (label, edge_type) for a graph edge.
#[cfg(feature = "ssr")]
fn edge_label_str(edge: &duumbi::graph::GraphEdge) -> (&'static str, &'static str) {
    use duumbi::graph::GraphEdge;
    match edge {
        GraphEdge::Left => ("left", "Left"),
        GraphEdge::Right => ("right", "Right"),
        GraphEdge::Operand => ("operand", "Operand"),
        GraphEdge::Condition => ("condition", "Condition"),
        GraphEdge::TrueBlock => ("true", "TrueBlock"),
        GraphEdge::FalseBlock => ("false", "FalseBlock"),
        GraphEdge::Arg(n) => match n {
            0 => ("arg[0]", "Arg"),
            1 => ("arg[1]", "Arg"),
            2 => ("arg[2]", "Arg"),
            _ => ("arg[N]", "Arg"),
        },
    }
}
