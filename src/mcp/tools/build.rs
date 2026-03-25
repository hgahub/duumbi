//! MCP build tools: compile and run the workspace.
//!
//! These tools invoke the full DUUMBI compilation pipeline. Because the
//! compiler and linker live in the binary-only `main.rs`/`cli` layer, these
//! functions return a descriptive error directing the caller to use the CLI
//! directly. The stubs are here so the tool definitions are always present
//! in the `tools/list` response; actual wiring happens when `duumbi mcp`
//! invokes the server from within the binary.

use std::path::Path;

use serde_json::Value;

/// Compile the workspace to a native binary.
///
/// This operation requires the full compiler pipeline which is not available
/// from the library crate. Use `duumbi build` from the CLI instead.
///
/// Returns `{ "error": "...", "hint": "..." }`.
pub fn build_compile(_workspace: &Path, _params: &Value) -> Result<Value, String> {
    Err("build_compile requires the full CLI pipeline. \
         Run `duumbi build` from the command line or invoke the MCP server \
         via `duumbi mcp` to enable this tool."
        .to_string())
}

/// Compile and run the workspace binary.
///
/// This operation requires the full compiler pipeline which is not available
/// from the library crate. Use `duumbi run` from the CLI instead.
///
/// Returns `{ "error": "...", "hint": "..." }`.
pub fn build_run(_workspace: &Path, _params: &Value) -> Result<Value, String> {
    Err("build_run requires the full CLI pipeline. \
         Run `duumbi run` from the command line or invoke the MCP server \
         via `duumbi mcp` to enable this tool."
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_compile_returns_informative_error() {
        let err = build_compile(&PathBuf::from("."), &serde_json::json!({}))
            .expect_err("should return error");
        assert!(
            err.contains("duumbi build"),
            "error should mention `duumbi build`"
        );
    }

    #[test]
    fn build_run_returns_informative_error() {
        let err = build_run(&PathBuf::from("."), &serde_json::json!({}))
            .expect_err("should return error");
        assert!(
            err.contains("duumbi run"),
            "error should mention `duumbi run`"
        );
    }
}
