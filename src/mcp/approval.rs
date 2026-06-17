//! Local MCP approval records for agent-initiated write candidates.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hash::{semantic_hash, semantic_hash_value};
use crate::patch::{GraphPatch, PatchOp, apply_patch};

/// Local approval state for one MCP write candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpApprovalStatus {
    /// Candidate is waiting for a human decision.
    Pending,
    /// Candidate was approved and may be applied if still current.
    Approved,
    /// Candidate was rejected and must not be applied.
    Rejected,
    /// Candidate was applied.
    Applied,
}

/// Local MCP approval record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpApprovalRecord {
    /// Stable local approval id.
    pub id: String,
    /// Current approval status.
    pub status: McpApprovalStatus,
    /// Tool family that requested approval.
    pub requested_tool: String,
    /// Specific action that approval covers.
    pub requested_action: String,
    /// Candidate semantic hash.
    pub candidate_hash: String,
    /// Workspace semantic hash when approval was requested.
    pub workspace_hash: String,
    /// Files expected to be affected.
    pub affected_files: Vec<String>,
    /// Node ids expected to be affected.
    pub affected_node_ids: Vec<String>,
    /// Human-readable risk label.
    pub risk: String,
    /// Human-readable candidate summary.
    pub summary: String,
    /// Patch operations for graph patch approvals.
    pub ops: Value,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Decision timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<DateTime<Utc>>,
    /// Decision source, such as MCP or TUI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_source: Option<String>,
    /// Rejection reason when rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

/// Result of previewing a graph patch candidate.
#[derive(Debug, Clone, Serialize)]
pub struct GraphPatchPreview {
    /// Number of patch operations.
    pub ops_count: usize,
    /// Graph file targeted by this first approval slice.
    pub graph_path: String,
    /// Current workspace semantic hash.
    pub workspace_hash: String,
    /// Candidate semantic hash.
    pub candidate_hash: String,
    /// Whether the candidate validates.
    pub valid: bool,
    /// Validation diagnostics.
    pub diagnostics: Vec<Value>,
    /// Affected node ids extracted from patch operations.
    pub affected_node_ids: Vec<String>,
}

/// Previews a graph patch candidate without writing the graph.
pub fn preview_graph_patch(
    workspace: &Path,
    ops_value: &Value,
) -> Result<(GraphPatchPreview, Value), String> {
    let ops: Vec<PatchOp> = serde_json::from_value(ops_value.clone())
        .map_err(|error| format!("Invalid patch ops: {error}"))?;
    let graph_dir = workspace.join(".duumbi").join("graph");
    let graph_path = graph_dir.join("main.jsonld");
    let source_text = fs::read_to_string(&graph_path)
        .map_err(|error| format!("Cannot read main.jsonld: {error}"))?;
    let source: Value = serde_json::from_str(&source_text)
        .map_err(|error| format!("Invalid JSON in main.jsonld: {error}"))?;
    let patch = GraphPatch { ops: ops.clone() };
    let candidate =
        apply_patch(&source, &patch).map_err(|error| format!("Patch failed: {error}"))?;
    let diagnostics = validate_candidate(&candidate)?;
    let workspace_hash =
        semantic_hash(&graph_dir).map_err(|error| format!("Workspace hash failed: {error}"))?;
    let candidate_hash = semantic_hash_value(&candidate);

    Ok((
        GraphPatchPreview {
            ops_count: ops.len(),
            graph_path: graph_path.display().to_string(),
            workspace_hash,
            candidate_hash,
            valid: diagnostics.is_empty(),
            diagnostics,
            affected_node_ids: affected_node_ids(&ops),
        },
        candidate,
    ))
}

/// Creates a pending graph patch approval record.
pub fn request_graph_patch_approval(
    workspace: &Path,
    ops_value: Value,
    summary: String,
) -> Result<(McpApprovalRecord, GraphPatchPreview), String> {
    let (preview, _candidate) = preview_graph_patch(workspace, &ops_value)?;
    if !preview.valid {
        return Err(format!(
            "Approval candidate is invalid: {}",
            serde_json::to_string(&preview.diagnostics)
                .unwrap_or_else(|_| "diagnostics unavailable".to_string())
        ));
    }
    let id = approval_id(&preview.candidate_hash);
    let record = McpApprovalRecord {
        id,
        status: McpApprovalStatus::Pending,
        requested_tool: "graph_patch_request_approval".to_string(),
        requested_action: "graph_patch_apply_approval".to_string(),
        candidate_hash: preview.candidate_hash.clone(),
        workspace_hash: preview.workspace_hash.clone(),
        affected_files: vec![".duumbi/graph/main.jsonld".to_string()],
        affected_node_ids: preview.affected_node_ids.clone(),
        risk: "local_graph_patch".to_string(),
        summary,
        ops: ops_value,
        created_at: Utc::now(),
        decided_at: None,
        decision_source: None,
        rejection_reason: None,
    };
    save_record(workspace, &record)?;
    Ok((record, preview))
}

/// Loads all local approval records.
pub fn list_records(workspace: &Path) -> Result<Vec<McpApprovalRecord>, String> {
    let dir = approvals_dir(workspace);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Cannot read approvals dir '{}': {error}", dir.display()))?
    {
        let entry = entry.map_err(|error| format!("Cannot read approval entry: {error}"))?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            records.push(read_record_path(&path)?);
        }
    }
    records.sort_by_key(|record| record.created_at);
    Ok(records)
}

/// Loads one local approval record.
pub fn load_record(workspace: &Path, id: &str) -> Result<McpApprovalRecord, String> {
    read_record_path(&record_path(workspace, id))
}

/// Records an approve/reject decision.
pub fn decide_record(
    workspace: &Path,
    id: &str,
    approve: bool,
    source: String,
    rejection_reason: Option<String>,
) -> Result<McpApprovalRecord, String> {
    let mut record = load_record(workspace, id)?;
    if record.status != McpApprovalStatus::Pending {
        return Err(format!(
            "Approval '{id}' is not pending; current status is {:?}",
            record.status
        ));
    }
    record.status = if approve {
        McpApprovalStatus::Approved
    } else {
        McpApprovalStatus::Rejected
    };
    record.decided_at = Some(Utc::now());
    record.decision_source = Some(source);
    record.rejection_reason = rejection_reason;
    save_record(workspace, &record)?;
    Ok(record)
}

/// Applies an approved graph patch after candidate and workspace checks.
pub fn apply_approved_graph_patch(workspace: &Path, id: &str) -> Result<McpApprovalRecord, String> {
    let mut record = load_record(workspace, id)?;
    match record.status {
        McpApprovalStatus::Approved => {}
        McpApprovalStatus::Pending => {
            return Err(format!("Approval required before applying '{id}'"));
        }
        McpApprovalStatus::Rejected => {
            return Err(format!("Approval '{id}' was rejected"));
        }
        McpApprovalStatus::Applied => {
            return Err(format!("Approval '{id}' was already applied"));
        }
    }

    let (preview, candidate) = preview_graph_patch(workspace, &record.ops)?;
    if preview.workspace_hash != record.workspace_hash {
        return Err(format!(
            "Approval stale for '{id}': workspace hash changed from {} to {}",
            record.workspace_hash, preview.workspace_hash
        ));
    }
    if preview.candidate_hash != record.candidate_hash {
        return Err(format!(
            "Approval stale for '{id}': candidate hash changed from {} to {}",
            record.candidate_hash, preview.candidate_hash
        ));
    }
    if !preview.valid {
        return Err(format!("Approved candidate no longer validates for '{id}'"));
    }

    let graph_path = workspace.join(".duumbi").join("graph").join("main.jsonld");
    let pretty = serde_json::to_string_pretty(&candidate)
        .map_err(|error| format!("Serialization error: {error}"))?;
    fs::write(&graph_path, pretty).map_err(|error| format!("Cannot write main.jsonld: {error}"))?;
    record.status = McpApprovalStatus::Applied;
    save_record(workspace, &record)?;
    Ok(record)
}

fn approvals_dir(workspace: &Path) -> PathBuf {
    workspace.join(".duumbi").join("session").join("approvals")
}

fn record_path(workspace: &Path, id: &str) -> PathBuf {
    approvals_dir(workspace).join(format!("{id}.json"))
}

fn save_record(workspace: &Path, record: &McpApprovalRecord) -> Result<(), String> {
    let dir = approvals_dir(workspace);
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Cannot create approvals dir '{}': {error}", dir.display()))?;
    let path = record_path(workspace, &record.id);
    let text = serde_json::to_string_pretty(record)
        .map_err(|error| format!("Approval serialization failed: {error}"))?;
    fs::write(&path, text)
        .map_err(|error| format!("Cannot write approval '{}': {error}", path.display()))
}

fn read_record_path(path: &Path) -> Result<McpApprovalRecord, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Cannot read approval '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("Invalid approval '{}': {error}", path.display()))
}

fn validate_candidate(candidate: &Value) -> Result<Vec<Value>, String> {
    use crate::graph::{builder, validator};
    use crate::parser;

    let candidate_text = serde_json::to_string(candidate)
        .map_err(|error| format!("Serialization error: {error}"))?;
    let module_ast = match parser::parse_jsonld(&candidate_text) {
        Ok(ast) => ast,
        Err(error) => {
            return Ok(vec![serde_json::json!({
                "level": "error",
                "message": error.to_string(),
            })]);
        }
    };
    let semantic_graph = match builder::build_graph(&module_ast) {
        Ok(graph) => graph,
        Err(errors) => {
            return Ok(errors
                .iter()
                .map(|error| {
                    serde_json::json!({
                        "level": "error",
                        "message": error.to_string(),
                    })
                })
                .collect());
        }
    };
    Ok(validator::validate(&semantic_graph)
        .iter()
        .map(|diagnostic| {
            serde_json::json!({
                "level": diagnostic.level,
                "code": diagnostic.code,
                "message": diagnostic.message,
            })
        })
        .collect())
}

fn approval_id(candidate_hash: &str) -> String {
    let prefix: String = candidate_hash.chars().take(12).collect();
    let timestamp = Utc::now()
        .timestamp_nanos_opt()
        .expect("invariant: current timestamp must fit in nanoseconds");
    format!("mcp-{timestamp}-{prefix}")
}

fn affected_node_ids(ops: &[PatchOp]) -> Vec<String> {
    let mut ids = Vec::new();
    for op in ops {
        match op {
            PatchOp::AddFunction { function } => push_value_id(function, &mut ids),
            PatchOp::AddBlock { function_id, block } => {
                ids.push(function_id.clone());
                push_value_id(block, &mut ids);
            }
            PatchOp::AddOp { block_id, op } => {
                ids.push(block_id.clone());
                push_value_id(op, &mut ids);
            }
            PatchOp::ModifyOp { node_id, .. }
            | PatchOp::RemoveNode { node_id }
            | PatchOp::SetEdge { node_id, .. } => ids.push(node_id.clone()),
            PatchOp::ReplaceBlock { block_id, .. } => ids.push(block_id.clone()),
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

fn push_value_id(value: &Value, ids: &mut Vec<String>) {
    if let Some(id) = value.get("@id").and_then(Value::as_str) {
        ids.push(id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const VALID_GRAPH: &str = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:main/main",
            "duumbi:name": "main",
            "duumbi:params": [],
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:main/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [
                    {
                        "@type": "duumbi:Const",
                        "@id": "duumbi:main/main/entry/0",
                        "duumbi:value": 0,
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Return",
                        "@id": "duumbi:main/main/entry/1",
                        "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}
                    }
                ]
            }]
        }]
    }"#;

    fn workspace() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let graph_dir = dir.path().join(".duumbi/graph");
        fs::create_dir_all(&graph_dir).expect("create graph");
        fs::write(graph_dir.join("main.jsonld"), VALID_GRAPH).expect("write graph");
        dir
    }

    fn modify_const_ops(value: i64) -> Value {
        serde_json::json!([{
            "kind": "modify_op",
            "node_id": "duumbi:main/main/entry/0",
            "field": "duumbi:value",
            "value": value,
        }])
    }

    #[test]
    fn graph_patch_approval_applies_exact_approved_candidate() {
        let dir = workspace();
        let graph_path = dir.path().join(".duumbi/graph/main.jsonld");
        let before = fs::read_to_string(&graph_path).expect("read before");
        let ops = modify_const_ops(7);

        let (record, preview) =
            request_graph_patch_approval(dir.path(), ops, "Change main return value".to_string())
                .expect("request approval");
        assert_eq!(record.status, McpApprovalStatus::Pending);
        assert!(preview.valid);
        assert_eq!(
            fs::read_to_string(&graph_path).expect("read after request"),
            before,
            "approval request must not write graph"
        );

        let err = apply_approved_graph_patch(dir.path(), &record.id)
            .expect_err("pending approval must not apply");
        assert!(err.contains("Approval required"));

        let approved =
            decide_record(dir.path(), &record.id, true, "test".to_string(), None).expect("approve");
        assert_eq!(approved.status, McpApprovalStatus::Approved);
        let applied = apply_approved_graph_patch(dir.path(), &record.id).expect("apply");
        assert_eq!(applied.status, McpApprovalStatus::Applied);

        let after = fs::read_to_string(&graph_path).expect("read after apply");
        assert!(after.contains("\"duumbi:value\": 7"));
    }

    #[test]
    fn stale_graph_patch_approval_is_rejected_without_writing_candidate() {
        let dir = workspace();
        let graph_path = dir.path().join(".duumbi/graph/main.jsonld");
        let (record, _preview) = request_graph_patch_approval(
            dir.path(),
            modify_const_ops(7),
            "Change value".to_string(),
        )
        .expect("request approval");

        let stale = fs::read_to_string(&graph_path)
            .expect("read graph")
            .replace("\"duumbi:value\": 0", "\"duumbi:value\": 3");
        fs::write(&graph_path, stale).expect("write stale graph");
        decide_record(dir.path(), &record.id, true, "test".to_string(), None).expect("approve");

        let err = apply_approved_graph_patch(dir.path(), &record.id)
            .expect_err("stale approval must fail");
        assert!(err.contains("Approval stale"));
        let after = fs::read_to_string(&graph_path).expect("read graph after stale");
        assert!(after.contains("\"duumbi:value\": 3"));
        assert!(!after.contains("\"duumbi:value\": 7"));
    }
}
