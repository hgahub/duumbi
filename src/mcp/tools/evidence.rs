//! Read-only MCP evidence discovery.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde_json::Value;

const DEFAULT_LIMIT: usize = 25;
const MAX_LIMIT: usize = 100;

/// Returns bounded local evidence metadata without reading raw logs or secrets.
pub fn mcp_evidence_status(workspace: &Path, params: &Value) -> Result<Value, String> {
    let limit = optional_limit(params)?
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);

    let approval_records = crate::mcp::approval::list_records(workspace)?
        .into_iter()
        .take(limit)
        .map(|record| {
            serde_json::json!({
                "id": record.id,
                "status": record.status,
                "requestedTool": record.requested_tool,
                "requestedAction": record.requested_action,
                "candidateHash": record.candidate_hash,
                "workspaceHash": record.workspace_hash,
                "affectedFiles": record.affected_files,
                "affectedNodeIds": record.affected_node_ids,
                "risk": record.risk,
                "summary": record.summary,
                "createdAt": record.created_at,
                "decidedAt": record.decided_at,
                "decisionSource": record.decision_source,
                "rejectionReason": record.rejection_reason,
            })
        })
        .collect::<Vec<_>>();

    let session_summary = session_summary(&workspace.join(".duumbi/session/current.json"));
    let evidence_roots = evidence_roots(workspace, limit);

    Ok(serde_json::json!({
        "status": "success",
        "scope": "mcp_evidence_status",
        "readOnly": true,
        "limit": limit,
        "workspace": workspace.display().to_string(),
        "session": session_summary,
        "approvals": approval_records,
        "roots": evidence_roots,
        "privacy": {
            "secretsIncluded": false,
            "rawCredentialsIncluded": false,
            "rawProviderResponsesIncluded": false,
            "rawLogsIncluded": false,
        }
    }))
}

fn session_summary(path: &Path) -> Value {
    let Ok(text) = fs::read_to_string(path) else {
        return serde_json::json!({
            "present": false,
            "path": display_path(path),
        });
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return serde_json::json!({
            "present": true,
            "path": display_path(path),
            "validJson": false,
        });
    };
    serde_json::json!({
        "present": true,
        "path": display_path(path),
        "validJson": true,
        "sessionId": value.get("session_id").cloned().unwrap_or(Value::Null),
        "startedAt": value.get("started_at").cloned().unwrap_or(Value::Null),
        "turnCount": value
            .get("turns")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        "usageStats": value.get("usage_stats").cloned().unwrap_or(Value::Null),
    })
}

fn evidence_roots(workspace: &Path, limit: usize) -> Vec<Value> {
    [
        ("session_history", workspace.join(".duumbi/session/history")),
        ("approvals", workspace.join(".duumbi/session/approvals")),
        (
            "model_performance",
            workspace.join(".duumbi/knowledge/model-performance"),
        ),
        ("snapshots", workspace.join(".duumbi/snapshots")),
        ("build_output", workspace.join(".duumbi/build")),
        ("local_evidence", workspace.join(".duumbi/evidence")),
        ("docs_e2e", workspace.join("docs/e2e")),
    ]
    .into_iter()
    .map(|(name, path)| root_summary(name, workspace, &path, limit))
    .collect()
}

fn root_summary(name: &str, workspace: &Path, path: &Path, limit: usize) -> Value {
    let (files, truncated) = if path.is_dir() {
        list_files(workspace, path, limit).unwrap_or_default()
    } else if path.is_file() {
        (vec![file_summary(workspace, path)], false)
    } else {
        (Vec::new(), false)
    };
    serde_json::json!({
        "name": name,
        "path": relative_or_display(workspace, path),
        "present": path.exists(),
        "fileCount": files.len(),
        "truncated": truncated,
        "files": files,
    })
}

fn list_files(workspace: &Path, root: &Path, limit: usize) -> Result<(Vec<Value>, bool), String> {
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir)
            .map_err(|error| format!("Cannot read evidence dir '{}': {error}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("Cannot read evidence entry: {error}"))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                files.push(file_summary(workspace, &path));
                if files.len() > limit {
                    files.sort_by_key(|file| file["path"].as_str().unwrap_or("").to_string());
                    files.truncate(limit);
                    return Ok((files, true));
                }
            }
        }
    }
    files.sort_by_key(|file| file["path"].as_str().unwrap_or("").to_string());
    Ok((files, false))
}

fn file_summary(workspace: &Path, path: &Path) -> Value {
    let metadata = fs::metadata(path).ok();
    serde_json::json!({
        "path": relative_or_display(workspace, path),
        "bytes": metadata.as_ref().map_or(0, fs::Metadata::len),
        "modifiedUnixSecs": metadata
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
    })
}

fn relative_or_display(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .map(display_path)
        .unwrap_or_else(|_| display_path(path))
}

fn display_path(path: &Path) -> String {
    normalize_path(PathBuf::from(path))
}

fn normalize_path(path: PathBuf) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn optional_limit(params: &Value) -> Result<Option<usize>, String> {
    match params.get("limit") {
        Some(value) => value
            .as_u64()
            .map(|limit| limit as usize)
            .map(Some)
            .ok_or_else(|| "limit must be an unsigned integer".to_string()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn evidence_status_lists_bounded_metadata_without_raw_turns() {
        let dir = TempDir::new().expect("tempdir");
        let session_dir = dir.path().join(".duumbi/session");
        fs::create_dir_all(&session_dir).expect("session dir");
        fs::write(
            session_dir.join("current.json"),
            serde_json::json!({
                "session_id": "session-test",
                "started_at": "2026-06-17T00:00:00Z",
                "turns": [{"request": "secret-shaped request", "summary": "done"}],
                "usage_stats": {"llm_calls": 0}
            })
            .to_string(),
        )
        .expect("write session");
        let build_dir = dir.path().join(".duumbi/build");
        fs::create_dir_all(&build_dir).expect("build dir");
        fs::write(build_dir.join("output"), "binary").expect("build output");

        let result = mcp_evidence_status(dir.path(), &serde_json::json!({ "limit": 10 }))
            .expect("evidence status");

        assert_eq!(result["status"], "success");
        assert_eq!(result["readOnly"], true);
        assert_eq!(result["session"]["sessionId"], "session-test");
        assert_eq!(result["session"]["turnCount"], 1);
        assert!(
            !result.to_string().contains("secret-shaped request"),
            "raw session turn text must not be returned"
        );
        assert!(
            result["roots"]
                .as_array()
                .expect("roots")
                .iter()
                .any(|root| root["name"] == "build_output" && root["present"] == true)
        );
    }

    #[test]
    fn evidence_status_rejects_invalid_limit() {
        let err = mcp_evidence_status(Path::new("."), &serde_json::json!({ "limit": "many" }))
            .expect_err("invalid limit should fail");

        assert!(err.contains("limit must be an unsigned integer"));
    }
}
