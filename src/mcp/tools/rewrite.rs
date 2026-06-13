//! MCP rewrite tools: rule discovery, preview, and explicit apply.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::rewrite::{ApplyMode, ApplyOptions, RewriteEngine};
use crate::snapshot;

/// List available semantic rewrite rules.
pub fn rewrite_list_rules(_workspace: &Path, params: &Value) -> Result<Value, String> {
    reject_unknown_fields(params, &["include_experimental"])?;
    let include_experimental = optional_bool(params, "include_experimental")?.unwrap_or(true);
    let rules = RewriteEngine::default()
        .list_rules()
        .into_iter()
        .filter(|rule| include_experimental || rule.apply_capable)
        .collect::<Vec<_>>();
    Ok(serde_json::json!({ "rules": rules }))
}

/// Preview one semantic rewrite rule without mutating workspace state.
pub fn rewrite_preview(workspace: &Path, params: &Value) -> Result<Value, String> {
    reject_unknown_fields(params, &["rule_id", "module", "limit"])?;
    let rule_id = required_string(params, "rule_id")?;
    let module = optional_string(params, "module")?;
    let limit = optional_usize(params, "limit")?;
    let module_path = resolve_module_path(workspace, module.as_deref());
    let (_, source) = read_module_source(&module_path)?;
    let preview = RewriteEngine::default()
        .preview_source(&source, &rule_id, limit)
        .map_err(|err| err.to_string())?;
    serde_json::to_value(preview).map_err(|err| err.to_string())
}

/// Apply one semantic rewrite rule after validation and snapshot creation.
pub fn rewrite_apply(workspace: &Path, params: &Value) -> Result<Value, String> {
    reject_unknown_fields(
        params,
        &["rule_id", "module", "match_id", "all", "max_matches"],
    )?;
    let rule_id = required_string(params, "rule_id")?;
    let module = optional_string(params, "module")?;
    let match_id = optional_string(params, "match_id")?;
    let all = optional_bool(params, "all")?.unwrap_or(false);
    let max_matches = optional_usize(params, "max_matches")?;
    if all == match_id.is_some() {
        return Err("Specify exactly one of match_id or all=true".to_string());
    }

    let module_path = resolve_module_path(workspace, module.as_deref());
    let (source_text, source) = read_module_source(&module_path)?;
    let outcome = RewriteEngine::default()
        .apply_to_source(
            &source,
            &ApplyOptions {
                rule_id,
                module: None,
                mode: if all {
                    ApplyMode::All
                } else {
                    ApplyMode::Match
                },
                match_id,
                max_matches,
            },
        )
        .map_err(|err| err.to_string())?;

    let snapshot_path = snapshot::save_snapshot(workspace, &source_text)
        .map_err(|err| format!("Snapshot failed: {err:#}"))?;
    let patched = serde_json::to_string_pretty(&outcome.candidate_source)
        .map_err(|err| format!("Serialization failed: {err}"))?;
    fs::write(&module_path, patched)
        .map_err(|err| format!("Cannot write '{}': {err}", module_path.display()))?;

    let mut result = serde_json::to_value(outcome.plan).map_err(|err| err.to_string())?;
    result["snapshotPath"] = Value::String(snapshot_path.display().to_string());
    Ok(result)
}

fn resolve_module_path(workspace: &Path, module: Option<&str>) -> PathBuf {
    match module {
        None => workspace.join(".duumbi").join("graph").join("main.jsonld"),
        Some(name)
            if Path::new(name).extension().is_none()
                && !name.contains(std::path::MAIN_SEPARATOR) =>
        {
            workspace
                .join(".duumbi")
                .join("graph")
                .join(format!("{name}.jsonld"))
        }
        Some(path) => PathBuf::from(path),
    }
}

fn read_module_source(path: &Path) -> Result<(String, Value), String> {
    let source_text = fs::read_to_string(path)
        .map_err(|err| format!("Cannot read module '{}': {err}", path.display()))?;
    let source = serde_json::from_str(&source_text)
        .map_err(|err| format!("Invalid JSON in '{}': {err}", path.display()))?;
    Ok((source_text, source))
}

fn reject_unknown_fields(params: &Value, allowed: &[&str]) -> Result<(), String> {
    let object = params
        .as_object()
        .ok_or_else(|| "Tool arguments must be a JSON object".to_string())?;
    if let Some(key) = object.keys().find(|key| {
        !allowed
            .iter()
            .any(|allowed_key| allowed_key == &key.as_str())
    }) {
        return Err(format!("Unknown field '{key}'"));
    }
    Ok(())
}

fn required_string(params: &Value, field: &str) -> Result<String, String> {
    params
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Missing or invalid required field '{field}'"))
}

fn optional_string(params: &Value, field: &str) -> Result<Option<String>, String> {
    match params.get(field) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("Field '{field}' must be a string")),
    }
}

fn optional_bool(params: &Value, field: &str) -> Result<Option<bool>, String> {
    match params.get(field) {
        None => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("Field '{field}' must be a boolean")),
    }
}

fn optional_usize(params: &Value, field: &str) -> Result<Option<usize>, String> {
    match params.get(field) {
        None => Ok(None),
        Some(Value::Number(value)) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| format!("Field '{field}' must be a positive integer")),
        Some(_) => Err(format!("Field '{field}' must be an integer")),
    }
}
