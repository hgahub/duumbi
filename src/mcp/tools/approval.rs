//! MCP approval tools for agent-safe graph writes.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::mcp::approval;

/// Previews a graph patch without writing workspace graph files.
pub fn graph_patch_preview(workspace: &Path, params: &Value) -> Result<Value, String> {
    let params = OpsParams::parse(params)?;
    let (preview, _candidate) = approval::preview_graph_patch(workspace, &params.ops)?;
    Ok(serde_json::json!({
        "status": "success",
        "scope": "graph_patch_preview",
        "preview": preview,
        "read_only": true,
    }))
}

/// Creates a pending approval record for a graph patch candidate.
pub fn graph_patch_request_approval(workspace: &Path, params: &Value) -> Result<Value, String> {
    let params = RequestApprovalParams::parse(params)?;
    let (record, preview) =
        approval::request_graph_patch_approval(workspace, params.ops, params.summary)?;
    Ok(serde_json::json!({
        "status": "success",
        "scope": "graph_patch_request_approval",
        "approval": record,
        "preview": preview,
    }))
}

/// Reads one or all local approval records.
pub fn approval_status(workspace: &Path, params: &Value) -> Result<Value, String> {
    let params = ApprovalStatusParams::parse(params)?;
    if let Some(id) = params.id {
        let record = approval::load_record(workspace, &id)?;
        return Ok(serde_json::json!({
            "status": "success",
            "scope": "approval_status",
            "approvals": [record],
        }));
    }
    let records = approval::list_records(workspace)?;
    Ok(serde_json::json!({
        "status": "success",
        "scope": "approval_status",
        "approvals": records,
    }))
}

/// Approves or rejects a local approval record.
pub fn approval_decide(workspace: &Path, params: &Value) -> Result<Value, String> {
    let params = ApprovalDecisionParams::parse(params)?;
    let approve = matches!(params.decision.as_str(), "approve");
    let record = approval::decide_record(
        workspace,
        &params.id,
        approve,
        params.decision_source,
        params.rejection_reason,
    )?;
    Ok(serde_json::json!({
        "status": "success",
        "scope": "approval_decide",
        "approval": record,
    }))
}

/// Applies an approved graph patch candidate exactly.
pub fn graph_patch_apply_approval(workspace: &Path, params: &Value) -> Result<Value, String> {
    let params = ApprovalIdParams::parse(params)?;
    let record = approval::apply_approved_graph_patch(workspace, &params.id)?;
    Ok(serde_json::json!({
        "status": "success",
        "scope": "graph_patch_apply_approval",
        "approval": record,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpsParams {
    ops: Value,
}

impl OpsParams {
    fn parse(params: &Value) -> Result<Self, String> {
        let parsed: Self = serde_json::from_value(params.clone())
            .map_err(|error| format!("Invalid graph patch preview arguments: {error}"))?;
        ensure_ops_array(&parsed.ops)?;
        Ok(parsed)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestApprovalParams {
    ops: Value,
    summary: String,
}

impl RequestApprovalParams {
    fn parse(params: &Value) -> Result<Self, String> {
        let parsed: Self = serde_json::from_value(params.clone())
            .map_err(|error| format!("Invalid graph approval request arguments: {error}"))?;
        ensure_ops_array(&parsed.ops)?;
        if parsed.summary.trim().is_empty() {
            return Err("Invalid graph approval request: summary must not be empty".to_string());
        }
        Ok(Self {
            ops: parsed.ops,
            summary: parsed.summary.trim().to_string(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalStatusParams {
    #[serde(default)]
    id: Option<String>,
}

impl ApprovalStatusParams {
    fn parse(params: &Value) -> Result<Self, String> {
        serde_json::from_value(params.clone())
            .map_err(|error| format!("Invalid approval status arguments: {error}"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalDecisionParams {
    id: String,
    decision: String,
    #[serde(default = "default_decision_source")]
    decision_source: String,
    #[serde(default)]
    rejection_reason: Option<String>,
}

impl ApprovalDecisionParams {
    fn parse(params: &Value) -> Result<Self, String> {
        let parsed: Self = serde_json::from_value(params.clone())
            .map_err(|error| format!("Invalid approval decision arguments: {error}"))?;
        match parsed.decision.as_str() {
            "approve" => Ok(Self {
                rejection_reason: None,
                ..parsed
            }),
            "reject" => Ok(parsed),
            other => Err(format!(
                "Invalid approval decision '{other}': expected 'approve' or 'reject'"
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalIdParams {
    id: String,
}

impl ApprovalIdParams {
    fn parse(params: &Value) -> Result<Self, String> {
        let parsed: Self = serde_json::from_value(params.clone())
            .map_err(|error| format!("Invalid approval apply arguments: {error}"))?;
        if parsed.id.trim().is_empty() {
            return Err("Invalid approval apply arguments: id must not be empty".to_string());
        }
        Ok(Self {
            id: parsed.id.trim().to_string(),
        })
    }
}

fn default_decision_source() -> String {
    "mcp".to_string()
}

fn ensure_ops_array(value: &Value) -> Result<(), String> {
    match value.as_array() {
        Some(_) => Ok(()),
        None => Err("'ops' must be an array".to_string()),
    }
}
