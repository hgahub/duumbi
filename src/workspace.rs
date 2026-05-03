//! Workspace build and run helpers shared by CLI and Studio.
//!
//! These helpers keep the native compilation path in the library crate so
//! browser-facing surfaces do not need to shell out through `cargo run`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use thiserror::Error;

use crate::compiler::{linker, lowering};
use crate::deps;

const RUNTIME_C_SOURCE: &str = include_str!("../runtime/duumbi_runtime.c");

/// Broad build-failure class for user-facing recovery suggestions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceBuildErrorKind {
    /// Program loading, graph construction, or validation failed.
    Graph,
    /// Native object generation failed.
    Compilation,
    /// Runtime compilation or native linking failed.
    Link,
}

/// Error produced while building a workspace binary.
#[derive(Debug, Error)]
pub enum WorkspaceBuildError {
    /// Program loading, graph construction, or validation failed.
    #[error("Graph construction failed: {0}")]
    Graph(#[source] deps::DepsError),
    /// Native object generation failed.
    #[error("Cranelift compilation failed: {0}")]
    Compilation(#[source] crate::compiler::CompileError),
    /// Internal inconsistency while collecting generated object files.
    #[error("Cranelift compilation failed: {message}")]
    CompilationInternal {
        /// Human-readable failure message.
        message: String,
    },
    /// Build filesystem setup failed.
    #[error("{context}: {source}")]
    BuildIo {
        /// Human-readable build step context.
        context: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Runtime compilation or native linking failed.
    #[error("Failed to link binary: {0}")]
    Link(#[source] anyhow::Error),
}

impl WorkspaceBuildError {
    /// Returns the broad error kind for CLI suggestion selection.
    #[must_use]
    pub fn kind(&self) -> WorkspaceBuildErrorKind {
        match self {
            Self::Graph(_) => WorkspaceBuildErrorKind::Graph,
            Self::Compilation(_) | Self::CompilationInternal { .. } | Self::BuildIo { .. } => {
                WorkspaceBuildErrorKind::Compilation
            }
            Self::Link(_) => WorkspaceBuildErrorKind::Link,
        }
    }
}

/// Result of executing a compiled workspace binary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BinaryRunOutput {
    /// Process exit code, or -1 if the process was terminated by signal.
    pub exit_code: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

/// Compiles all modules in a workspace and links them into `output`.
///
/// When `offline` is true, dependency resolution skips the cache and registry
/// layers and only uses workspace/vendor sources.
pub fn build_workspace(
    workspace_root: &Path,
    output: &Path,
    offline: bool,
) -> std::result::Result<PathBuf, WorkspaceBuildError> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|source| WorkspaceBuildError::BuildIo {
            context: format!("Failed to create build directory '{}'", parent.display()),
            source,
        })?;
    }

    let program = deps::load_program_with_deps_opts(workspace_root, offline)
        .map_err(WorkspaceBuildError::Graph)?;

    let objects = lowering::compile_program(&program).map_err(WorkspaceBuildError::Compilation)?;

    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp_dir =
        std::env::temp_dir().join(format!("duumbi_build_{}_{}", std::process::id(), unique_id));
    fs::create_dir_all(&tmp_dir).map_err(|source| WorkspaceBuildError::BuildIo {
        context: "Failed to create temp build directory".to_string(),
        source,
    })?;

    let mut module_names: Vec<&String> = objects.keys().collect();
    module_names.sort();

    let mut object_paths = Vec::with_capacity(module_names.len());
    for module_name in module_names {
        let obj_bytes =
            objects
                .get(module_name)
                .ok_or_else(|| WorkspaceBuildError::CompilationInternal {
                    message: format!("Missing object bytes for module '{module_name}'"),
                })?;
        let obj_path = tmp_dir.join(format!("{module_name}.o"));
        if let Some(parent) = obj_path.parent() {
            fs::create_dir_all(parent).map_err(|source| WorkspaceBuildError::BuildIo {
                context: format!("Failed to create dir for '{}'", obj_path.display()),
                source,
            })?;
        }
        fs::write(&obj_path, obj_bytes).map_err(|source| WorkspaceBuildError::BuildIo {
            context: format!("Failed to write object file '{}'", obj_path.display()),
            source,
        })?;
        object_paths.push(obj_path);
    }

    let runtime_c = find_runtime_c().map_err(WorkspaceBuildError::Link)?;
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o)
        .context("Failed to compile C runtime")
        .map_err(WorkspaceBuildError::Link)?;

    let object_path_refs: Vec<&Path> = object_paths.iter().map(|p| p.as_path()).collect();
    linker::link_multi(&object_path_refs, &runtime_o, output)
        .context("Failed to link binary")
        .map_err(WorkspaceBuildError::Link)?;

    let _ = fs::remove_dir_all(&tmp_dir);
    Ok(output.to_path_buf())
}

/// Runs a compiled workspace binary, capturing stdout and stderr.
pub fn run_workspace_binary(workspace_root: &Path, args: &[String]) -> Result<BinaryRunOutput> {
    let output_path = workspace_root.join(".duumbi/build/output");
    if !output_path.exists() {
        anyhow::bail!(
            "No binary found at '{}'. Build first.",
            output_path.display()
        );
    }

    let output = std::process::Command::new(&output_path)
        .args(args)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("Failed to execute '{}'", output_path.display()))?;

    Ok(BinaryRunOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn find_runtime_c() -> Result<PathBuf> {
    let candidates = [
        PathBuf::from("runtime/duumbi_runtime.c"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("runtime/duumbi_runtime.c")))
            .unwrap_or_default(),
    ];

    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    let tmp_dir = std::env::temp_dir().join("duumbi_build");
    fs::create_dir_all(&tmp_dir).context("Failed to create temp build directory")?;
    let runtime_path = tmp_dir.join("duumbi_runtime.c");
    fs::write(&runtime_path, RUNTIME_C_SOURCE).context("Failed to write embedded runtime")?;
    Ok(runtime_path)
}
