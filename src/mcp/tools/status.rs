//! Read-only MCP capability and workspace status tool.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::config;
use crate::mcp::capability;

/// Returns DUUMBI MCP capability metadata and local workspace readiness.
pub fn mcp_capability_status(workspace: &Path, _params: &Value) -> Result<Value, String> {
    let tools = capability::tool_definitions();
    let tool_count = tools.len();
    let read_only_count = tools
        .iter()
        .filter(|tool| matches!(tool.metadata.safety, capability::ToolSafety::ReadOnly))
        .count();
    let write_capable_count = tools
        .iter()
        .filter(|tool| !matches!(tool.metadata.safety, capability::ToolSafety::ReadOnly))
        .count();
    let unavailable_tools: Vec<Value> = tools
        .iter()
        .filter_map(|tool| {
            tool.metadata.unavailable_reason.as_ref().map(|reason| {
                serde_json::json!({
                    "name": tool.name,
                    "reason": reason,
                    "providerRequired": tool.metadata.provider_required,
                    "networkRequired": tool.metadata.network_required,
                })
            })
        })
        .collect();

    let duumbi_dir = workspace.join(".duumbi");
    let graph_dir = duumbi_dir.join("graph");
    let main_graph = graph_dir.join("main.jsonld");
    let intents_dir = duumbi_dir.join("intents");
    let deps_lock = duumbi_dir.join("deps.lock");
    let build_output = crate::workspace::workspace_output_path(workspace);
    let approvals_dir = duumbi_dir.join("session").join("approvals");
    let providers_configured = config::load_effective_config(workspace)
        .map(|config| !config.config.effective_providers().is_empty())
        .unwrap_or(false);

    Ok(serde_json::json!({
        "status": "success",
        "scope": "mcp_capability_status",
        "workspace": {
            "root": workspace.display().to_string(),
            "duumbiInitialized": duumbi_dir.is_dir(),
            "graphDirPresent": graph_dir.is_dir(),
            "mainGraphPresent": main_graph.is_file(),
            "intentsDirPresent": intents_dir.is_dir(),
            "depsLockPresent": deps_lock.is_file(),
            "buildOutputPresent": build_output.is_file(),
            "providerConfigured": providers_configured,
            "pendingApprovalCount": pending_approval_count(&approvals_dir),
        },
        "capabilities": {
            "toolCount": tool_count,
            "readOnlyToolCount": read_only_count,
            "writeCapableToolCount": write_capable_count,
            "approvalFlowAvailable": true,
            "queryToolAvailable": true,
            "buildRunAvailable": true,
            "evidenceRetrievalAvailable": true,
            "unavailableTools": unavailable_tools,
        },
        "tools": tools,
        "privacy": {
            "secretsIncluded": false,
            "rawCredentialsIncluded": false,
            "rawProviderResponsesIncluded": false,
        },
    }))
}

fn pending_approval_count(approvals_dir: &Path) -> usize {
    if !approvals_dir.is_dir() {
        return 0;
    }
    match fs::read_dir(approvals_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "json")
            })
            .count(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn capability_status_reports_workspace_without_mutating() {
        let dir = TempDir::new().expect("tempdir");
        let graph_dir = dir.path().join(".duumbi/graph");
        fs::create_dir_all(&graph_dir).expect("create graph");
        fs::write(graph_dir.join("main.jsonld"), "{}").expect("write graph");

        let before = fs::read(graph_dir.join("main.jsonld")).expect("read before");
        let result = mcp_capability_status(dir.path(), &serde_json::json!({})).expect("status ok");
        let after = fs::read(graph_dir.join("main.jsonld")).expect("read after");

        assert_eq!(before, after);
        assert_eq!(result["status"], "success");
        assert_eq!(result["workspace"]["duumbiInitialized"], true);
        assert_eq!(result["workspace"]["mainGraphPresent"], true);
        assert!(
            result["capabilities"]["toolCount"]
                .as_u64()
                .expect("tool count")
                > 0
        );
        assert!(
            result["tools"]
                .as_array()
                .expect("tools")
                .iter()
                .any(|tool| tool["name"] == "mcp_capability_status")
        );
    }
}
