//! MCP intent tools: create and execute intent-driven development specs.
//!
//! Intent creation and execution require async LLM calls, so these
//! synchronous stubs return informative errors directing the caller to
//! use the CLI. The full pipeline is available via `duumbi intent`.

use std::path::Path;

use serde_json::Value;

/// Create an intent spec from a natural language description.
///
/// Params:
/// - `description` — natural language description of the intent (required)
///
/// Intent creation requires an async LLM call. Use `duumbi intent create`
/// from the CLI instead.
pub fn intent_create(_workspace: &Path, params: &Value) -> Result<Value, String> {
    let description = params
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing required field 'description'".to_string())?;

    Err(format!(
        "intent_create (description='{description}') requires an async LLM call. \
         Use `duumbi intent create \"{description}\"` from the CLI instead."
    ))
}

/// Execute an intent: decompose → mutate graph → verify tests.
///
/// Params:
/// - `name` — intent slug/name to execute (required)
///
/// Intent execution requires async LLM calls and compilation. Use
/// `duumbi intent execute <name>` from the CLI instead.
pub fn intent_execute(_workspace: &Path, params: &Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing required field 'name'".to_string())?;

    Err(format!(
        "intent_execute (name='{name}') requires async LLM calls and compilation. \
         Use `duumbi intent execute {name}` from the CLI instead."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn intent_create_requires_description() {
        let err = intent_create(&PathBuf::from("."), &serde_json::json!({}))
            .expect_err("should require description");
        assert!(
            err.contains("description"),
            "error should mention missing 'description'"
        );
    }

    #[test]
    fn intent_create_suggests_cli() {
        let err = intent_create(
            &PathBuf::from("."),
            &serde_json::json!({ "description": "build a calculator" }),
        )
        .expect_err("should return error");
        assert!(
            err.contains("duumbi intent create"),
            "error should suggest `duumbi intent create`"
        );
    }

    #[test]
    fn intent_execute_requires_name() {
        let err = intent_execute(&PathBuf::from("."), &serde_json::json!({}))
            .expect_err("should require name");
        assert!(err.contains("name"), "error should mention missing 'name'");
    }

    #[test]
    fn intent_execute_suggests_cli() {
        let err = intent_execute(
            &PathBuf::from("."),
            &serde_json::json!({ "name": "calculator" }),
        )
        .expect_err("should return error");
        assert!(
            err.contains("duumbi intent execute"),
            "error should suggest `duumbi intent execute`"
        );
    }
}
