//! MCP build tools: compile and run the workspace.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

const DEFAULT_RUN_TIMEOUT_SECS: u64 = 30;
const MAX_RUN_TIMEOUT_SECS: u64 = 300;
const MAX_STDIN_BYTES: usize = 64 * 1024;

/// Compile the workspace to a native binary.
pub fn build_compile(workspace: &Path, params: &Value) -> Result<Value, String> {
    let offline = optional_bool(params, "offline")?.unwrap_or(false);
    let output_path = crate::workspace::workspace_output_path(workspace);
    let built_path = crate::workspace::build_workspace(workspace, &output_path, offline)
        .map_err(|error| format!("build_compile failed: {error:#}"))?;

    Ok(serde_json::json!({
        "status": "success",
        "scope": "build_compile",
        "ok": true,
        "outputPath": built_path.display().to_string(),
        "offline": offline,
        "outputCapture": {
            "stdout": "unavailable",
            "stderr": "unavailable",
            "reason": "shared library build backend does not expose compiler process streams"
        },
        "exitCode": 0,
        "timedOut": false,
        "evidence": [{
            "kind": "build_output",
            "path": built_path.display().to_string()
        }]
    }))
}

/// Compile and run the workspace binary.
pub fn build_run(workspace: &Path, params: &Value) -> Result<Value, String> {
    let offline = optional_bool(params, "offline")?.unwrap_or(false);
    let build_first = optional_bool(params, "build")?.unwrap_or(true);
    let args = optional_string_array(params, "args")?.unwrap_or_default();
    let stdin = optional_string(params, "stdin")?.unwrap_or_default();
    if stdin.len() > MAX_STDIN_BYTES {
        return Err(format!("stdin must be at most {MAX_STDIN_BYTES} bytes"));
    }
    let timeout_secs = optional_u64(params, "timeout_secs")?
        .unwrap_or(DEFAULT_RUN_TIMEOUT_SECS)
        .clamp(1, MAX_RUN_TIMEOUT_SECS);
    let output_path = crate::workspace::workspace_output_path(workspace);

    let build = if build_first {
        let built_path = crate::workspace::build_workspace(workspace, &output_path, offline)
            .map_err(|error| format!("build_run build step failed: {error:#}"))?;
        Some(serde_json::json!({
            "ok": true,
            "outputPath": built_path.display().to_string(),
            "offline": offline
        }))
    } else {
        None
    };

    let output = crate::workspace::run_workspace_binary_with_stdin_and_timeout(
        workspace,
        &args,
        &stdin,
        Duration::from_secs(timeout_secs),
    )
    .map_err(|error| format!("build_run failed: {error:#}"))?;

    Ok(serde_json::json!({
        "status": if output.exit_code == 0 && !output.timed_out { "success" } else { "failed" },
        "scope": "build_run",
        "ok": output.exit_code == 0 && !output.timed_out,
        "build": build,
        "outputPath": output_path.display().to_string(),
        "args": args,
        "stdout": output.stdout,
        "stderr": output.stderr,
        "exitCode": output.exit_code,
        "timedOut": output.timed_out,
        "timeoutSecs": timeout_secs,
        "evidence": [{
            "kind": "build_output",
            "path": output_path.display().to_string()
        }]
    }))
}

fn optional_bool(params: &Value, field: &str) -> Result<Option<bool>, String> {
    match params.get(field) {
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| format!("{field} must be a boolean")),
        None => Ok(None),
    }
}

fn optional_u64(params: &Value, field: &str) -> Result<Option<u64>, String> {
    match params.get(field) {
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{field} must be an unsigned integer")),
        None => Ok(None),
    }
}

fn optional_string_array(params: &Value, field: &str) -> Result<Option<Vec<String>>, String> {
    match params.get(field) {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(ToString::to_string)
                    .ok_or_else(|| format!("{field} entries must be strings"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(format!("{field} must be an array of strings")),
        None => Ok(None),
    }
}

fn optional_string(params: &Value, field: &str) -> Result<Option<String>, String> {
    match params.get(field) {
        Some(value) => value
            .as_str()
            .map(ToString::to_string)
            .map(Some)
            .ok_or_else(|| format!("{field} must be a string")),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    static BUILD_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    fn setup_workspace() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let graph_dir = dir.path().join(".duumbi/graph");
        fs::create_dir_all(&graph_dir).expect("create graph dir");
        fs::write(graph_dir.join("main.jsonld"), VALID_GRAPH).expect("write graph");
        fs::write(
            dir.path().join(".duumbi/config.toml"),
            "[workspace]\nname = \"test\"\n",
        )
        .expect("write config");
        dir
    }

    #[test]
    fn build_compile_builds_workspace_binary() {
        let _guard = BUILD_TEST_LOCK.lock().expect("build test lock");
        let dir = setup_workspace();
        let result = build_compile(dir.path(), &serde_json::json!({ "offline": true }))
            .expect("build succeeds");

        assert_eq!(result["status"], "success");
        assert_eq!(result["ok"], true);
        assert_eq!(result["outputCapture"]["stdout"], "unavailable");
        assert!(
            PathBuf::from(result["outputPath"].as_str().expect("output path")).is_file(),
            "build output must exist"
        );
    }

    #[test]
    fn build_run_builds_and_captures_output() {
        let _guard = BUILD_TEST_LOCK.lock().expect("build test lock");
        let dir = setup_workspace();
        let result = build_run(
            dir.path(),
            &serde_json::json!({ "offline": true, "timeout_secs": 5 }),
        )
        .expect("run succeeds");

        assert_eq!(result["scope"], "build_run");
        assert_eq!(result["ok"], true);
        assert_eq!(result["exitCode"], 0);
        assert_eq!(result["timedOut"], false);
    }

    #[test]
    fn build_run_rejects_invalid_args() {
        let err = build_run(&PathBuf::from("."), &serde_json::json!({ "args": [1] }))
            .expect_err("args should reject non-string entries");
        assert!(err.contains("args entries must be strings"));
    }

    #[test]
    fn build_run_rejects_oversized_stdin() {
        let err = build_run(
            &PathBuf::from("."),
            &serde_json::json!({ "stdin": "x".repeat(MAX_STDIN_BYTES + 1) }),
        )
        .expect_err("oversized stdin should be rejected");

        assert!(err.contains("stdin must be at most"));
    }
}
