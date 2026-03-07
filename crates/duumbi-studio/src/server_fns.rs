//! Server functions for the Studio.
//!
//! These `#[server]` functions run on the server and are callable from
//! the client via Leptos RPC. They bridge the Studio UI to the duumbi
//! workspace: graph loading, building, chat, and intent management.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::state::{GraphData, IntentSummary};

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

    // Scan for modules: main.jsonld + subdirectories with graph/
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

    // Check for dependency modules
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

                        // Add dependency edge from main to this module
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

    // Check stdlib
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

/// Returns graph data for the Container level (functions within a module).
#[server]
pub async fn get_module_detail(module_name: String) -> Result<GraphData, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;

    let graph_path = if module_name == "app/main" {
        ws.root.join(".duumbi/graph/main.jsonld")
    } else {
        // Try stdlib or dependency paths
        let parts: Vec<&str> = module_name.split('/').collect();
        if parts.first() == Some(&"stdlib") && parts.len() == 2 {
            ws.root
                .join(format!(".duumbi/stdlib/{}", parts[1]))
                .join(".duumbi/graph/main.jsonld")
        } else {
            ws.root.join(format!(".duumbi/graph/{module_name}.jsonld"))
        }
    };

    let source = std::fs::read_to_string(&graph_path)
        .map_err(|e| ServerFnError::new(format!("Failed to read {}: {e}", graph_path.display())))?;

    let ast = duumbi::parser::parse_jsonld(&source)
        .map_err(|e| ServerFnError::new(format!("Parse error: {e}")))?;

    let graph = duumbi::graph::builder::build_graph(&ast)
        .map_err(|errors| ServerFnError::new(format!("Graph errors: {errors:?}")))?;

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

    // Add call edges between functions
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

    let graph_path = if module_name == "app/main" {
        ws.root.join(".duumbi/graph/main.jsonld")
    } else {
        let parts: Vec<&str> = module_name.split('/').collect();
        if parts.first() == Some(&"stdlib") && parts.len() == 2 {
            ws.root
                .join(format!(".duumbi/stdlib/{}", parts[1]))
                .join(".duumbi/graph/main.jsonld")
        } else {
            ws.root.join(format!(".duumbi/graph/{module_name}.jsonld"))
        }
    };

    let source = std::fs::read_to_string(&graph_path)
        .map_err(|e| ServerFnError::new(format!("Failed to read: {e}")))?;
    let ast = duumbi::parser::parse_jsonld(&source)
        .map_err(|e| ServerFnError::new(format!("Parse error: {e}")))?;
    let graph = duumbi::graph::builder::build_graph(&ast)
        .map_err(|errors| ServerFnError::new(format!("Graph errors: {errors:?}")))?;

    let func = graph
        .functions
        .iter()
        .find(|f| f.name.as_ref() == function_name)
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
    let branch_targets = &graph.branch_targets;
    for block in &func.blocks {
        for &node_idx in &block.nodes {
            if let Some(targets) = branch_targets.get(&node_idx) {
                if let Some(ref true_block) = targets.true_block {
                    edges.push(GraphEdge {
                        id: format!("branch:{}:true", block.label),
                        source: block.label.to_string(),
                        target: true_block.clone(),
                        label: "true".to_string(),
                        edge_type: "branch_true".to_string(),
                    });
                }
                if let Some(ref false_block) = targets.false_block {
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

    let graph_path = if module_name == "app/main" {
        ws.root.join(".duumbi/graph/main.jsonld")
    } else {
        let parts: Vec<&str> = module_name.split('/').collect();
        if parts.first() == Some(&"stdlib") && parts.len() == 2 {
            ws.root
                .join(format!(".duumbi/stdlib/{}", parts[1]))
                .join(".duumbi/graph/main.jsonld")
        } else {
            ws.root.join(format!(".duumbi/graph/{module_name}.jsonld"))
        }
    };

    let source = std::fs::read_to_string(&graph_path)
        .map_err(|e| ServerFnError::new(format!("Failed to read: {e}")))?;
    let ast = duumbi::parser::parse_jsonld(&source)
        .map_err(|e| ServerFnError::new(format!("Parse error: {e}")))?;
    let graph = duumbi::graph::builder::build_graph(&ast)
        .map_err(|errors| ServerFnError::new(format!("Graph errors: {errors:?}")))?;

    let func = graph
        .functions
        .iter()
        .find(|f| f.name.as_ref() == function_name)
        .ok_or_else(|| ServerFnError::new(format!("Function '{function_name}' not found")))?;

    let block = func
        .blocks
        .iter()
        .find(|b| b.label.as_ref() == block_label)
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

        // Add data flow edges
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

                        // Parse just the intent and status fields
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

/// Returns (label, edge_type) for a graph edge.
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
