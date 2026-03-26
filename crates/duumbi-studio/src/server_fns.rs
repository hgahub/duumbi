//! Server functions for the Studio.
//!
//! These `#[server]` functions run on the server and are callable from
//! the client via Leptos RPC. They bridge the Studio UI to the duumbi
//! workspace: graph loading, building, chat, and intent management.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// GraphNode/GraphEdge/InitialData are used inside #[server] fn bodies and load_initial_data (ssr feature only)
#[allow(unused_imports)]
use crate::state::{
    GraphData, GraphEdge, GraphNode, InitialData, InstalledDep, IntentSummary, RegistrySearchHit,
};

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

/// Returns the graph data for the C4 Context level.
///
/// Shows the application as a software system with the user (person)
/// and stdout (external system). Derives data from the main module graph.
#[server]
pub async fn get_graph_context() -> Result<GraphData, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;
    let graph = load_graph(&ws.root, "app/main")?;
    Ok(build_c4_context(&graph))
}

/// Builds C4 Context level graph data from a SemanticGraph.
#[cfg(feature = "ssr")]
fn build_c4_context(graph: &duumbi::graph::SemanticGraph) -> GraphData {
    let app_name = graph.module_name.0.as_str();
    let entry_return = graph
        .functions
        .iter()
        .find(|f| f.name.0 == "main")
        .map(|f| f.return_type.to_string())
        .unwrap_or_else(|| "void".to_string());

    // Uniform node size for C4 Context level
    let c4_w = 200.0;
    let c4_h = 80.0;

    let mut nodes = vec![
        GraphNode {
            id: "person:user".to_string(),
            label: "Felhasználó".to_string(),
            node_type: "person".to_string(),
            badge: Some("Futtatja a programot".to_string()),
            x: 0.0,
            y: 0.0,
            width: c4_w,
            height: c4_h,
        },
        GraphNode {
            id: "system:app".to_string(),
            label: app_name.to_string(),
            node_type: "system".to_string(),
            badge: Some(format!("[Software System] main() → {entry_return}")),
            x: 0.0,
            y: 0.0,
            width: c4_w,
            height: c4_h,
        },
    ];

    let mut edges = vec![GraphEdge {
        id: "e0".to_string(),
        source: "person:user".to_string(),
        target: "system:app".to_string(),
        label: "Futtatja".to_string(),
        edge_type: "uses".to_string(),
    }];

    if has_print_op(graph) {
        nodes.push(GraphNode {
            id: "external:stdout".to_string(),
            label: "stdout".to_string(),
            node_type: "external".to_string(),
            badge: Some("[Külső: Terminal I/O]".to_string()),
            x: 0.0,
            y: 0.0,
            width: c4_w,
            height: c4_h,
        });
        edges.push(GraphEdge {
            id: "e1".to_string(),
            source: "system:app".to_string(),
            target: "external:stdout".to_string(),
            label: "Kiírja az eredményt".to_string(),
            edge_type: "output".to_string(),
        });
    }

    GraphData { nodes, edges }
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

/// Returns graph data for the C4 Container level (binary + runtime shim).
///
/// Shows the native binary and runtime shim containers within the
/// software system boundary.
#[server]
pub async fn get_module_detail(module_name: String) -> Result<GraphData, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;
    let graph = load_graph(&ws.root, &module_name)?;
    Ok(build_c4_container(&graph))
}

/// Builds C4 Container level graph data.
#[cfg(feature = "ssr")]
fn build_c4_container(graph: &duumbi::graph::SemanticGraph) -> GraphData {
    let app_name = graph.module_name.0.as_str();
    let has_io = has_print_op(graph);

    let c4_w = 200.0;
    let c4_h = 80.0;

    let mut nodes = vec![
        GraphNode {
            id: "person:user".to_string(),
            label: "Felhasználó".to_string(),
            node_type: "person".to_string(),
            badge: None,
            x: 0.0,
            y: 0.0,
            width: c4_w,
            height: c4_h,
        },
        GraphNode {
            id: "boundary:app".to_string(),
            label: format!("{app_name} [Software System]"),
            node_type: "boundary".to_string(),
            badge: None,
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 200.0,
        },
        GraphNode {
            id: "container:binary".to_string(),
            label: "Natív Bináris".to_string(),
            node_type: "container".to_string(),
            badge: Some("[Cranelift compiled]".to_string()),
            x: 0.0,
            y: 0.0,
            width: c4_w,
            height: c4_h,
        },
    ];

    let mut edges = vec![GraphEdge {
        id: "e0".to_string(),
        source: "person:user".to_string(),
        target: "container:binary".to_string(),
        label: "Futtatja".to_string(),
        edge_type: "uses".to_string(),
    }];

    if has_io {
        nodes.push(GraphNode {
            id: "container:runtime".to_string(),
            label: "Runtime Shim".to_string(),
            node_type: "container".to_string(),
            badge: Some("[duumbi_runtime.c]".to_string()),
            x: 0.0,
            y: 0.0,
            width: c4_w,
            height: c4_h,
        });
        nodes.push(GraphNode {
            id: "external:stdout".to_string(),
            label: "stdout".to_string(),
            node_type: "external".to_string(),
            badge: Some("[Terminal I/O]".to_string()),
            x: 0.0,
            y: 0.0,
            width: c4_w,
            height: c4_h,
        });
        edges.push(GraphEdge {
            id: "e1".to_string(),
            source: "container:binary".to_string(),
            target: "container:runtime".to_string(),
            label: "hívja".to_string(),
            edge_type: "call".to_string(),
        });
        edges.push(GraphEdge {
            id: "e2".to_string(),
            source: "container:runtime".to_string(),
            target: "external:stdout".to_string(),
            label: "printf → stdout".to_string(),
            edge_type: "output".to_string(),
        });
    }

    GraphData { nodes, edges }
}

/// Returns graph data for the C4 Component level (active vs dead code).
///
/// Shows functions as components, separated into active (reachable from main)
/// and dead code groups. Also adds sub-component nodes for op categories.
#[server]
pub async fn get_function_detail(
    module_name: String,
    #[allow(unused_variables)] _function_name: String,
) -> Result<GraphData, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;
    let graph = load_graph(&ws.root, &module_name)?;
    Ok(build_c4_component(&graph))
}

/// Builds C4 Component level graph data.
#[cfg(feature = "ssr")]
fn build_c4_component(graph: &duumbi::graph::SemanticGraph) -> GraphData {
    let reachable = reachable_fns(graph);
    let has_io = has_print_op(graph);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut edge_counter: usize = 0;

    // Classify ops across active functions
    let mut has_arithmetic = false;
    let mut has_io_ops = false;
    let mut has_control_flow = false;

    for func in &graph.functions {
        let fn_name = &func.name.0;
        let is_active = reachable.contains(fn_name.as_str());

        if is_active {
            for block in &func.blocks {
                for &node_idx in &block.nodes {
                    match &graph.graph[node_idx].op {
                        duumbi::types::Op::Add
                        | duumbi::types::Op::Sub
                        | duumbi::types::Op::Mul
                        | duumbi::types::Op::Div => has_arithmetic = true,
                        duumbi::types::Op::Print => has_io_ops = true,
                        duumbi::types::Op::Compare(_) | duumbi::types::Op::Branch => {
                            has_control_flow = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect();
        let label = if params.is_empty() {
            format!("{}()", fn_name)
        } else {
            format!("{}({})", fn_name, params.join(", "))
        };
        let total_ops: usize = func.blocks.iter().map(|b| b.nodes.len()).sum();

        if is_active {
            let meta = if fn_name == "main" {
                format!("→ {} | entry point", func.return_type)
            } else {
                format!(
                    "→ {} | {} blocks, {} ops",
                    func.return_type,
                    func.blocks.len(),
                    total_ops
                )
            };
            nodes.push(GraphNode {
                id: format!("component:{fn_name}"),
                label,
                node_type: "component".to_string(),
                badge: Some(meta),
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 60.0,
            });
        } else {
            let meta = format!(
                "→ {} | {} blocks, {} ops",
                func.return_type,
                func.blocks.len(),
                total_ops
            );
            nodes.push(GraphNode {
                id: format!("component:{fn_name}"),
                label,
                node_type: "component-dead".to_string(),
                badge: Some(meta),
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 60.0,
            });
        }
    }

    // Sub-component nodes
    if has_arithmetic {
        nodes.push(GraphNode {
            id: "component:math".to_string(),
            label: "Aritmetika".to_string(),
            node_type: "component-sub".to_string(),
            badge: Some("Add/Sub/Mul/Div".to_string()),
            x: 0.0,
            y: 0.0,
            width: 140.0,
            height: 50.0,
        });
        edges.push(GraphEdge {
            id: format!("e{edge_counter}"),
            source: "component:main".to_string(),
            target: "component:math".to_string(),
            label: "használ".to_string(),
            edge_type: "uses".to_string(),
        });
        edge_counter += 1;
    }

    if has_io_ops {
        nodes.push(GraphNode {
            id: "component:io".to_string(),
            label: "I/O".to_string(),
            node_type: "component-sub".to_string(),
            badge: Some("Print".to_string()),
            x: 0.0,
            y: 0.0,
            width: 140.0,
            height: 50.0,
        });
        edges.push(GraphEdge {
            id: format!("e{edge_counter}"),
            source: "component:main".to_string(),
            target: "component:io".to_string(),
            label: "használ".to_string(),
            edge_type: "uses".to_string(),
        });
        edge_counter += 1;
    }

    if has_control_flow {
        nodes.push(GraphNode {
            id: "component:control".to_string(),
            label: "Control Flow".to_string(),
            node_type: "component-sub".to_string(),
            badge: Some("Compare/Branch".to_string()),
            x: 0.0,
            y: 0.0,
            width: 140.0,
            height: 50.0,
        });
        edges.push(GraphEdge {
            id: format!("e{edge_counter}"),
            source: "component:main".to_string(),
            target: "component:control".to_string(),
            label: "használ".to_string(),
            edge_type: "uses".to_string(),
        });
        edge_counter += 1;
    }

    // External dependencies — only add edges from component:io when the node exists
    if has_io {
        nodes.push(GraphNode {
            id: "external:runtime".to_string(),
            label: "Runtime Shim".to_string(),
            node_type: "external".to_string(),
            badge: Some("[duumbi_print_i64]".to_string()),
            x: 0.0,
            y: 0.0,
            width: 160.0,
            height: 60.0,
        });
        nodes.push(GraphNode {
            id: "external:stdout".to_string(),
            label: "stdout".to_string(),
            node_type: "external".to_string(),
            badge: Some("[Terminal]".to_string()),
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 50.0,
        });
        if has_io_ops {
            edges.push(GraphEdge {
                id: format!("e{edge_counter}"),
                source: "component:io".to_string(),
                target: "external:runtime".to_string(),
                label: "hívja".to_string(),
                edge_type: "call".to_string(),
            });
            edge_counter += 1;
        }
        edges.push(GraphEdge {
            id: format!("e{edge_counter}"),
            source: "external:runtime".to_string(),
            target: "external:stdout".to_string(),
            label: "→ stdout".to_string(),
            edge_type: "output".to_string(),
        });
    }

    GraphData { nodes, edges }
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
        let op_type = op_type_name_str(&node.op);
        let result_type = node
            .result_type
            .as_ref()
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
            let (label, edge_type) = edge_label_pair(edge_ref.weight());

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
    if stdlib_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&stdlib_dir)
    {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                modules.push(format!("stdlib/{}", entry.file_name().to_string_lossy()));
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
    if intents_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&intents_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "yaml")
                && let Ok(content) = std::fs::read_to_string(&path)
            {
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

    Ok(intents)
}

/// Returns the short type name for an Op.
#[cfg(feature = "ssr")]
pub fn op_type_name_str(op: &duumbi::types::Op) -> &'static str {
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
        Op::ConstString(_) => "ConstString",
        Op::PrintString => "PrintString",
        Op::StringConcat => "StringConcat",
        Op::StringEquals => "StringEquals",
        Op::StringCompare(_) => "StringCompare",
        Op::StringLength => "StringLength",
        Op::StringSlice => "StringSlice",
        Op::StringContains => "StringContains",
        Op::StringFind => "StringFind",
        Op::StringFromI64 => "StringFromI64",
        Op::ArrayNew => "ArrayNew",
        Op::ArrayPush => "ArrayPush",
        Op::ArrayGet => "ArrayGet",
        Op::ArraySet => "ArraySet",
        Op::ArrayLength => "ArrayLength",
        Op::StructNew { .. } => "StructNew",
        Op::FieldGet { .. } => "FieldGet",
        Op::FieldSet { .. } => "FieldSet",
        Op::Alloc { .. } => "Alloc",
        Op::Move { .. } => "Move",
        Op::Borrow { mutable: false, .. } => "Borrow",
        Op::Borrow { mutable: true, .. } => "BorrowMut",
        Op::Drop { .. } => "Drop",
        Op::ResultOk => "ResultOk",
        Op::ResultErr => "ResultErr",
        Op::ResultIsOk => "ResultIsOk",
        Op::ResultUnwrap => "ResultUnwrap",
        Op::ResultUnwrapErr => "ResultUnwrapErr",
        Op::OptionSome => "OptionSome",
        Op::OptionNone => "OptionNone",
        Op::OptionIsSome => "OptionIsSome",
        Op::OptionUnwrap => "OptionUnwrap",
        Op::Match { .. } => "Match",
        _ => "Unknown",
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

    let providers = config.effective_providers();
    let provider_cfg = providers.first().ok_or_else(|| {
        ServerFnError::new("No LLM provider configured in config.toml".to_string())
    })?;

    let client = duumbi::agents::factory::create_provider(provider_cfg)
        .map_err(|e| ServerFnError::new(format!("LLM provider error: {e}")))?;

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

/// Synchronously loads initial data for SSR rendering.
///
/// Called once at server startup. Returns C4 Context level graph, workspace
/// name, intents, and module list so the first SSR render has real data.
#[cfg(feature = "ssr")]
pub fn load_initial_data(workspace: &std::path::Path) -> InitialData {
    use std::fs;

    let name = workspace
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "workspace".to_string());

    // Build C4 Context graph from main module
    let graph = if workspace.join(".duumbi/graph/main.jsonld").exists() {
        match load_graph(workspace, "app/main") {
            Ok(sg) => build_c4_context(&sg),
            Err(_) => GraphData {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
        }
    } else {
        GraphData {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    };

    // Collect module list
    let mut modules = Vec::new();
    if workspace.join(".duumbi/graph/main.jsonld").exists() {
        modules.push("app/main".to_string());
    }
    let stdlib_dir = workspace.join(".duumbi/stdlib");
    if stdlib_dir.exists()
        && let Ok(entries) = fs::read_dir(&stdlib_dir)
    {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                modules.push(format!("stdlib/{}", entry.file_name().to_string_lossy()));
            }
        }
    }

    // Collect intents
    let mut intents = Vec::new();
    let intents_dir = workspace.join(".duumbi/intents");
    if intents_dir.exists()
        && let Ok(entries) = fs::read_dir(&intents_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "yaml")
                && let Ok(content) = fs::read_to_string(&path)
            {
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

    InitialData {
        graph,
        workspace_name: name,
        intents,
        modules,
    }
}

/// Computes the set of function names reachable from `main()` via Call ops (BFS).
#[cfg(feature = "ssr")]
fn reachable_fns(graph: &duumbi::graph::SemanticGraph) -> std::collections::HashSet<String> {
    use std::collections::{HashSet, VecDeque};

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back("main".to_string());
    visited.insert("main".to_string());

    while let Some(fn_name) = queue.pop_front() {
        if let Some(func) = graph.functions.iter().find(|f| f.name.0 == fn_name) {
            for block in &func.blocks {
                for &node_idx in &block.nodes {
                    if let duumbi::types::Op::Call { function, .. } = &graph.graph[node_idx].op {
                        let callee = function.to_string();
                        if visited.insert(callee.clone()) {
                            queue.push_back(callee);
                        }
                    }
                }
            }
        }
    }

    visited
}

/// Returns `true` if any function in the graph contains a Print op.
#[cfg(feature = "ssr")]
fn has_print_op(graph: &duumbi::graph::SemanticGraph) -> bool {
    graph.functions.iter().any(|func| {
        func.blocks.iter().any(|block| {
            block
                .nodes
                .iter()
                .any(|&idx| matches!(graph.graph[idx].op, duumbi::types::Op::Print))
        })
    })
}

/// Public wrapper for `build_c4_container` (used by API routes in lib.rs).
#[cfg(feature = "ssr")]
pub fn build_c4_container_pub(graph: &duumbi::graph::SemanticGraph) -> GraphData {
    build_c4_container(graph)
}

/// Public wrapper for `build_c4_component` (used by API routes in lib.rs).
#[cfg(feature = "ssr")]
pub fn build_c4_component_pub(graph: &duumbi::graph::SemanticGraph) -> GraphData {
    build_c4_component(graph)
}

/// Searches configured registries for modules matching a query.
#[server]
pub async fn search_registry(query: String) -> Result<Vec<RegistrySearchHit>, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;

    let config = duumbi::config::load_config(&ws.root)
        .map_err(|e| ServerFnError::new(format!("Config error: {e}")))?;

    let registries = config.registries.clone();

    if registries.is_empty() {
        return Ok(Vec::new());
    }

    let creds = duumbi::registry::credentials::load_credentials().unwrap_or_default();
    let client_creds = duumbi::registry::credentials::to_client_credentials(&creds);

    let client = duumbi::registry::RegistryClient::new(registries.clone(), client_creds, None)
        .map_err(|e| ServerFnError::new(format!("Registry client error: {e}")))?;

    let mut results = Vec::new();
    for registry_name in registries.keys() {
        match client.search(registry_name, &query).await {
            Ok(resp) => {
                for hit in resp.results {
                    results.push(RegistrySearchHit {
                        name: hit.name,
                        description: hit.description,
                        latest_version: hit.latest_version,
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Search failed for registry '{registry_name}': {e}");
            }
        }
    }

    Ok(results)
}

/// Installs a module by adding it as a dependency and downloading to cache.
///
/// First runs `duumbi deps add <module_name>` to register the dependency
/// in config.toml, then `duumbi deps install` to download and lock it.
#[server]
pub async fn install_module(module_name: String) -> Result<String, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;

    // Step 1: Add the module as a dependency in config.toml
    let add_output = tokio::process::Command::new("cargo")
        .args(["run", "--quiet", "--", "deps", "add", &module_name])
        .current_dir(&ws.root)
        .output()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to run deps add: {e}")))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(ServerFnError::new(format!(
            "Failed to add dependency: {stderr}"
        )));
    }

    // Step 2: Install (download + lock) the dependency
    let install_output = tokio::process::Command::new("cargo")
        .args(["run", "--quiet", "--", "deps", "install"])
        .current_dir(&ws.root)
        .output()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to run deps install: {e}")))?;

    if install_output.status.success() {
        Ok(format!("Installed {module_name} successfully"))
    } else {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        Err(ServerFnError::new(format!("Install failed: {stderr}")))
    }
}

/// Returns the list of installed dependencies from config.toml.
#[server]
pub async fn get_installed_deps() -> Result<Vec<InstalledDep>, ServerFnError> {
    let ws = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ws.read().await;

    let config = duumbi::config::load_config(&ws.root)
        .map_err(|e| ServerFnError::new(format!("Config error: {e}")))?;

    let deps_map = config.dependencies.clone();

    let mut deps = Vec::new();
    for (name, dep_config) in deps_map {
        let (version, source) = match dep_config {
            duumbi::config::DependencyConfig::Version(v) => (v, "registry".to_string()),
            duumbi::config::DependencyConfig::Path { path } => (path, "path".to_string()),
            duumbi::config::DependencyConfig::VersionWithRegistry { version, registry } => {
                (version, format!("registry ({registry})"))
            }
        };
        deps.push(InstalledDep {
            name,
            version,
            source,
        });
    }

    // Also check vendor dir — supports both `vendor/pkg` and `vendor/@scope/pkg`
    let vendor_dir = ws.root.join(".duumbi/vendor");
    if vendor_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&vendor_dir)
    {
        for entry in entries.flatten() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if dir_name.starts_with('@') {
                // Scoped package: walk children (e.g. `@scope/name`)
                let scope_dir = entry.path();
                if let Ok(children) = std::fs::read_dir(&scope_dir) {
                    for child in children.flatten() {
                        let child_name = child.file_name().to_string_lossy().to_string();
                        let full_name = format!("{dir_name}/{child_name}");
                        if !deps.iter().any(|d| d.name == full_name) {
                            deps.push(InstalledDep {
                                name: full_name,
                                version: "vendored".to_string(),
                                source: "vendor".to_string(),
                            });
                        }
                    }
                }
            } else if !deps.iter().any(|d| d.name == dir_name) {
                deps.push(InstalledDep {
                    name: dir_name,
                    version: "vendored".to_string(),
                    source: "vendor".to_string(),
                });
            }
        }
    }
    Ok(deps)
}

/// Returns (label, edge_type) for a graph edge.
#[cfg(feature = "ssr")]
pub fn edge_label_pair(edge: &duumbi::graph::GraphEdge) -> (&'static str, &'static str) {
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
        GraphEdge::Owns => ("owns", "Owns"),
        GraphEdge::MovesFrom => ("moves", "MovesFrom"),
        GraphEdge::BorrowsFrom => ("borrows", "BorrowsFrom"),
        GraphEdge::Drops => ("drops", "Drops"),
    }
}

// ── Phase 15 server functions ──────────────────────────────────────────────

/// Provider info for the command palette provider list.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderInfo {
    /// Provider kind (e.g., "Anthropic", "OpenAI").
    pub kind: String,
    /// Model name.
    pub model: String,
    /// Role: "Primary" or "Fallback".
    pub role: String,
}

/// Returns configured LLM providers from config.toml.
#[server]
pub async fn get_provider_list() -> Result<Vec<ProviderInfo>, ServerFnError> {
    let ctx = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ctx.read().await;

    let config = duumbi::config::load_config(&ws.root)
        .map_err(|e| ServerFnError::new(format!("Config: {e}")))?;

    let providers = config.effective_providers();
    Ok(providers
        .iter()
        .map(|p| ProviderInfo {
            kind: format!("{:?}", p.provider),
            model: p.model.clone(),
            role: format!("{:?}", p.role),
        })
        .collect())
}

/// Runs the compiled binary and returns stdout/stderr.
#[server]
pub async fn trigger_run() -> Result<String, ServerFnError> {
    let ctx = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ctx.read().await;

    let output_path = ws.root.join(".duumbi").join("build").join("output");
    if !output_path.exists() {
        return Err(ServerFnError::new(
            "No binary found. Build first.".to_string(),
        ));
    }

    let output = tokio::process::Command::new(&output_path)
        .current_dir(&ws.root)
        .output()
        .await
        .map_err(|e| ServerFnError::new(format!("Run failed: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit = output.status.code().unwrap_or(-1);

    Ok(format!(
        "Exit code: {exit}\n\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    ))
}

/// Executes an intent by slug (full pipeline: decompose → mutate → verify).
#[server]
pub async fn execute_intent(slug: String) -> Result<String, ServerFnError> {
    let ctx = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ctx.read().await;

    let config = duumbi::config::load_config(&ws.root)
        .map_err(|e| ServerFnError::new(format!("Config: {e}")))?;

    let providers = config.effective_providers();
    let client = duumbi::agents::factory::create_provider_chain(&providers)
        .map_err(|e| ServerFnError::new(format!("Provider: {e}")))?;

    let success = duumbi::intent::execute::run_execute(&*client, &ws.root, &slug)
        .await
        .map_err(|e| ServerFnError::new(format!("Execute: {e}")))?;

    if success {
        Ok(format!(
            "Intent '{slug}' executed successfully — all tests passed."
        ))
    } else {
        Ok(format!("Intent '{slug}' executed — some tests failed."))
    }
}

/// Creates a new intent from a natural language description.
///
/// Uses the LLM to generate a structured YAML spec, then saves it.
#[server]
pub async fn create_intent(description: String) -> Result<IntentSummary, ServerFnError> {
    let ctx = expect_context::<std::sync::Arc<tokio::sync::RwLock<WorkspaceContext>>>();
    let ws = ctx.read().await;

    let config = duumbi::config::load_config(&ws.root)
        .map_err(|e| ServerFnError::new(format!("Config: {e}")))?;

    let providers = config.effective_providers();
    let client = duumbi::agents::factory::create_provider_chain(&providers)
        .map_err(|e| ServerFnError::new(format!("Provider: {e}")))?;

    // Studio always auto-confirms (no interactive prompt).
    let slug = duumbi::intent::create::run_create(&*client, &ws.root, &description, true)
        .await
        .map_err(|e| ServerFnError::new(format!("Create: {e}")))?;

    // Reload the saved intent to get its status
    let spec = duumbi::intent::load_intent(&ws.root, &slug)
        .map_err(|e| ServerFnError::new(format!("Load: {e}")))?;

    Ok(IntentSummary {
        slug,
        description: spec.intent,
        status: format!("{:?}", spec.status),
    })
}
