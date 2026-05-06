//! Shared workspace workflow orchestration.
//!
//! CLI commands, Studio endpoints, and local validation harnesses use this
//! layer for intent create/execute, graph evidence, build, and run operations.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::agents::LlmProvider;

/// Result for intent creation workflows.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntentCreateWorkflowResult {
    /// Saved intent slug.
    pub slug: String,
    /// Human-readable status message.
    pub message: String,
    /// Workflow log lines.
    pub log: Vec<String>,
}

/// Result for intent execution workflows.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntentExecuteWorkflowResult {
    /// Whether the intent completed successfully.
    pub ok: bool,
    /// Human-readable status message.
    pub message: String,
    /// Workflow log lines.
    pub log: Vec<String>,
}

/// Result for graph evidence collection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphEvidence {
    /// Module names discovered under `.duumbi/graph/**/*.jsonld`.
    pub modules: Vec<String>,
}

/// Result for workspace build workflows.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildWorkflowResult {
    /// Whether build succeeded.
    pub ok: bool,
    /// Human-readable status message.
    pub message: String,
    /// Output binary path when build succeeded.
    pub output_path: Option<String>,
}

/// Result for workspace run workflows.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunWorkflowResult {
    /// Whether the binary executed with exit code 0.
    pub ok: bool,
    /// Process exit code, or -1 when unavailable.
    pub exit_code: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

/// Creates and saves an intent through the shared workflow service.
pub async fn create_intent(
    client: &dyn LlmProvider,
    workspace: &Path,
    description: &str,
    yes: bool,
) -> Result<IntentCreateWorkflowResult> {
    let mut log = Vec::new();
    let slug = crate::intent::create::run_create(client, workspace, description, yes, &mut log)
        .await
        .context("intent create failed")?;
    Ok(IntentCreateWorkflowResult {
        message: format!("Intent saved as {slug}"),
        slug,
        log,
    })
}

/// Executes an intent through the shared workflow service.
pub async fn execute_intent(
    client: &dyn LlmProvider,
    workspace: &Path,
    slug: &str,
) -> Result<IntentExecuteWorkflowResult> {
    let mut log = Vec::new();
    let ok = crate::intent::execute::run_execute(client, workspace, slug, &mut log)
        .await
        .context("intent execute failed")?;
    Ok(IntentExecuteWorkflowResult {
        ok,
        message: if ok {
            format!("Intent '{slug}' completed")
        } else {
            format!("Intent '{slug}' failed")
        },
        log,
    })
}

/// Collects graph module evidence from `.duumbi/graph/**/*.jsonld`.
pub fn graph_evidence(workspace: &Path) -> Result<GraphEvidence> {
    let graph_dir = workspace.join(".duumbi/graph");
    let mut modules = Vec::new();
    collect_jsonld_modules(&graph_dir, &graph_dir, &mut modules)?;
    modules.sort();
    modules.dedup();
    Ok(GraphEvidence { modules })
}

/// Builds the current workspace.
#[must_use]
pub fn build_workspace(workspace: &Path) -> BuildWorkflowResult {
    let output = workspace.join(".duumbi/build/output");
    match crate::workspace::build_workspace(workspace, &output, false) {
        Ok(path) => BuildWorkflowResult {
            ok: true,
            message: format!("Build successful: {}", path.display()),
            output_path: Some(path.display().to_string()),
        },
        Err(e) => BuildWorkflowResult {
            ok: false,
            message: format!("Build failed: {e:#}"),
            output_path: None,
        },
    }
}

/// Runs the current workspace binary.
#[must_use]
pub fn run_workspace(workspace: &Path) -> RunWorkflowResult {
    match crate::workspace::run_workspace_binary(workspace, &[]) {
        Ok(output) => RunWorkflowResult {
            ok: output.exit_code == 0,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
        },
        Err(e) => RunWorkflowResult {
            ok: false,
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("{e:#}"),
        },
    }
}

fn collect_jsonld_modules(root: &Path, dir: &Path, modules: &mut Vec<String>) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if dir == root && e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(e).with_context(|| format!("read graph dir '{}'", dir.display()));
        }
    };

    for entry in entries {
        let entry = entry.with_context(|| format!("read graph dir '{}'", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonld_modules(root, &path, modules)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonld")
            && let Some(module) = module_name_from_path(root, &path)
        {
            modules.push(module);
        }
    }

    Ok(())
}

fn module_name_from_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut without_ext = PathBuf::from(relative);
    without_ext.set_extension("");
    Some(
        without_ext
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn graph_evidence_includes_nested_modules() {
        let tmp = TempDir::new().expect("temp dir");
        let graph_dir = tmp.path().join(".duumbi/graph/calculator");
        std::fs::create_dir_all(&graph_dir).expect("mkdir");
        std::fs::write(graph_dir.join("ops.jsonld"), "{}").expect("write");
        std::fs::write(tmp.path().join(".duumbi/graph/main.jsonld"), "{}").expect("write");

        let evidence = graph_evidence(tmp.path()).expect("evidence");

        assert!(evidence.modules.contains(&"calculator/ops".to_string()));
        assert!(evidence.modules.contains(&"main".to_string()));
    }

    #[test]
    fn run_workspace_returns_structured_no_binary_error() {
        let tmp = TempDir::new().expect("temp dir");
        let result = run_workspace(tmp.path());

        assert!(!result.ok);
        assert_eq!(result.exit_code, -1);
        assert!(result.stderr.contains("No binary found"));
    }
}
