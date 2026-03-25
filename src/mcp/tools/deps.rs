//! MCP dependency tools: search registries and install dependencies.
//!
//! These tools interface with the DUUMBI dependency resolution system.
//! Registry search requires async HTTP, so it returns an informative error
//! when called from the synchronous MCP dispatch path. The `duumbi mcp`
//! binary command can extend these with async capability.

use std::path::Path;

use serde_json::Value;

/// Search registries for available modules.
///
/// Params:
/// - `query`    — search terms (required)
/// - `registry` — limit to a named registry (optional)
///
/// Registry search requires async HTTP. This synchronous stub returns an
/// informative message; the full search is available via `duumbi search`.
pub fn deps_search(_workspace: &Path, params: &Value) -> Result<Value, String> {
    let query = params
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing required field 'query'".to_string())?;

    Err(format!(
        "deps_search (query='{query}') requires async HTTP to registry. \
         Use `duumbi search {query}` from the CLI instead."
    ))
}

/// Install all declared dependencies into the local cache.
///
/// Params:
/// - `frozen` — if true, fail if the lockfile would change (bool, optional)
///
/// Dependency installation requires async HTTP. This synchronous stub returns
/// an informative message; the full install is available via `duumbi deps install`.
pub fn deps_install(_workspace: &Path, params: &Value) -> Result<Value, String> {
    let frozen = params
        .get("frozen")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Err(format!(
        "deps_install (frozen={frozen}) requires async HTTP to registry. \
         Use `duumbi deps install{}` from the CLI instead.",
        if frozen { " --frozen" } else { "" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn deps_search_requires_query() {
        let err = deps_search(&PathBuf::from("."), &serde_json::json!({}))
            .expect_err("should require query");
        assert!(
            err.contains("query"),
            "error should mention missing 'query'"
        );
    }

    #[test]
    fn deps_search_suggests_cli() {
        let err = deps_search(&PathBuf::from("."), &serde_json::json!({ "query": "math" }))
            .expect_err("should return error");
        assert!(
            err.contains("duumbi search"),
            "error should suggest `duumbi search`"
        );
    }

    #[test]
    fn deps_install_suggests_cli() {
        let err = deps_install(&PathBuf::from("."), &serde_json::json!({}))
            .expect_err("should return error");
        assert!(
            err.contains("duumbi deps install"),
            "error should suggest `duumbi deps install`"
        );
    }

    #[test]
    fn deps_install_frozen_flag_in_message() {
        let err = deps_install(&PathBuf::from("."), &serde_json::json!({ "frozen": true }))
            .expect_err("should return error");
        assert!(
            err.contains("--frozen"),
            "error should mention --frozen flag"
        );
    }
}
