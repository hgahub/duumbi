//! Workspace build and run helpers shared by CLI and Studio.
//!
//! These helpers keep the native compilation path in the library crate so
//! browser-facing surfaces do not need to shell out through `cargo run`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use thiserror::Error;

use crate::compiler::{linker, lowering};
use crate::deps;
use crate::telemetry::{BuildOptions, TELEMETRY_DIR_ENV};

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
    /// Runtime config could not be loaded for traced build validation.
    #[error("Failed to load telemetry config: {0}")]
    Config(#[source] crate::config::ConfigError),
    /// Telemetry config is invalid for a traced build.
    #[error("Invalid telemetry config: {0}")]
    TelemetryConfig(#[source] crate::telemetry::TelemetryValidationError),
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
    /// Local telemetry artifact generation failed.
    #[error("Telemetry artifact generation failed: {0}")]
    Telemetry(#[source] crate::telemetry::TelemetryError),
}

impl WorkspaceBuildError {
    /// Returns the broad error kind for CLI suggestion selection.
    #[must_use]
    pub fn kind(&self) -> WorkspaceBuildErrorKind {
        match self {
            Self::Config(_) | Self::TelemetryConfig(_) => WorkspaceBuildErrorKind::Compilation,
            Self::Graph(_) => WorkspaceBuildErrorKind::Graph,
            Self::Compilation(_) | Self::CompilationInternal { .. } | Self::BuildIo { .. } => {
                WorkspaceBuildErrorKind::Compilation
            }
            Self::Link(_) => WorkspaceBuildErrorKind::Link,
            Self::Telemetry(_) => WorkspaceBuildErrorKind::Compilation,
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

/// Returns the default native binary path for a workspace build.
#[must_use]
pub fn workspace_output_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join(".duumbi/build")
        .join(format!("output{}", std::env::consts::EXE_SUFFIX))
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
    build_workspace_with_options(workspace_root, output, BuildOptions::offline(offline))
}

/// Compiles all modules in a workspace with explicit build options.
pub fn build_workspace_with_options(
    workspace_root: &Path,
    output: &Path,
    options: BuildOptions,
) -> std::result::Result<PathBuf, WorkspaceBuildError> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|source| WorkspaceBuildError::BuildIo {
            context: format!("Failed to create build directory '{}'", parent.display()),
            source,
        })?;
    }

    let telemetry_config = validate_trace_config(workspace_root, options.telemetry)?;

    let program = deps::load_program_with_deps_opts(workspace_root, options.offline)
        .map_err(WorkspaceBuildError::Graph)?;

    let objects = lowering::compile_program_with_telemetry(&program, options.telemetry)
        .map_err(WorkspaceBuildError::Compilation)?;

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

    if options.telemetry.is_trace() {
        let trace_map = crate::telemetry::TraceMap::from_program(&program)
            .map_err(WorkspaceBuildError::Telemetry)?;
        let telemetry_dir = telemetry_config
            .as_ref()
            .expect("invariant: trace config is present when telemetry mode is trace")
            .artifact_dir
            .clone();
        crate::telemetry::write_trace_map(&trace_map, &telemetry_dir)
            .map_err(WorkspaceBuildError::Telemetry)?;
    }

    let _ = fs::remove_dir_all(&tmp_dir);
    Ok(output.to_path_buf())
}

fn validate_trace_config(
    workspace_root: &Path,
    telemetry: crate::telemetry::TelemetryBuildMode,
) -> std::result::Result<Option<crate::telemetry::ResolvedTelemetryConfig>, WorkspaceBuildError> {
    if !telemetry.is_trace() {
        return Ok(None);
    }

    let section = crate::config::load_effective_config(workspace_root)
        .map_err(WorkspaceBuildError::Config)?
        .config
        .telemetry
        .unwrap_or_default();
    section
        .resolve_for_trace(workspace_root)
        .map(Some)
        .map_err(WorkspaceBuildError::TelemetryConfig)
}

/// Runs a compiled workspace binary, capturing stdout and stderr.
#[must_use = "the captured process output should be inspected"]
pub fn run_workspace_binary(workspace_root: &Path, args: &[String]) -> Result<BinaryRunOutput> {
    run_workspace_binary_inner(workspace_root, args, BinaryStdin::Inherit)
}

/// Runs a compiled workspace binary with supplied stdin, capturing stdout and stderr.
#[allow(dead_code)] // Public lib API used by integration tests; binary target uses inherited stdin.
#[must_use = "the captured process output should be inspected"]
pub fn run_workspace_binary_with_stdin(
    workspace_root: &Path,
    args: &[String],
    stdin: &str,
) -> Result<BinaryRunOutput> {
    run_workspace_binary_inner(workspace_root, args, BinaryStdin::Bytes(stdin))
}

#[allow(dead_code)] // The binary target only constructs inherited stdin.
enum BinaryStdin<'a> {
    Inherit,
    Bytes(&'a str),
}

fn run_workspace_binary_inner(
    workspace_root: &Path,
    args: &[String],
    stdin: BinaryStdin<'_>,
) -> Result<BinaryRunOutput> {
    let output_path = workspace_output_path(workspace_root);
    if !output_path.exists() {
        anyhow::bail!(
            "No binary found at '{}'. Build first.",
            output_path.display()
        );
    }

    let mut command = std::process::Command::new(&output_path);
    command
        .args(args)
        .current_dir(workspace_root)
        .env("DUUMBI_WORKSPACE_ROOT", workspace_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if std::env::var_os(TELEMETRY_DIR_ENV).is_none_or(|value| value.is_empty())
        && let Some(telemetry_dir) = workspace_runtime_telemetry_dir(workspace_root)
    {
        command.env(TELEMETRY_DIR_ENV, telemetry_dir);
    }

    match stdin {
        BinaryStdin::Inherit => {
            command.stdin(std::process::Stdio::inherit());
        }
        BinaryStdin::Bytes(_) => {
            command.stdin(std::process::Stdio::piped());
        }
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("Failed to execute '{}'", output_path.display()))?;

    if let BinaryStdin::Bytes(input) = stdin {
        if !input.is_empty() {
            let child_stdin = child.stdin.as_mut().context("Failed to open child stdin")?;
            child_stdin
                .write_all(input.as_bytes())
                .context("Failed to write child stdin")?;
        }
        drop(child.stdin.take());
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed to wait for '{}'", output_path.display()))?;

    Ok(BinaryRunOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn workspace_runtime_telemetry_dir(workspace_root: &Path) -> Option<PathBuf> {
    let section = crate::config::load_effective_config(workspace_root)
        .ok()?
        .config
        .telemetry
        .unwrap_or_default();
    Some(section.resolve_for_trace(workspace_root).ok()?.artifact_dir)
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
    copy_runtime_sqlite_deps(&runtime_path)?;
    Ok(runtime_path)
}

fn copy_runtime_sqlite_deps(runtime_c: &Path) -> Result<()> {
    let runtime_dir = runtime_c
        .parent()
        .with_context(|| format!("Runtime path '{}' has no parent", runtime_c.display()))?;
    let sqlite_dir = runtime_dir.join("third_party").join("sqlite");
    fs::create_dir_all(&sqlite_dir).context("Failed to create temp SQLite runtime directory")?;

    for file_name in ["sqlite3.c", "sqlite3.h"] {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("runtime")
            .join("third_party")
            .join("sqlite")
            .join(file_name);
        let destination = sqlite_dir.join(file_name);
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "Failed to copy SQLite runtime dependency '{}' to '{}'",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn run_workspace_binary_sets_workspace_root_and_writes_stdin() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().expect("tempdir");
        let output_path = workspace_output_path(tmp.path());
        fs::create_dir_all(output_path.parent().expect("output parent")).expect("build dir");
        fs::write(
            &output_path,
            b"#!/bin/sh\nread line\nprintf '%s|%s' \"$DUUMBI_WORKSPACE_ROOT\" \"$line\"\n",
        )
        .expect("write script");
        let mut permissions = fs::metadata(&output_path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&output_path, permissions).expect("chmod");

        let output =
            run_workspace_binary_with_stdin(tmp.path(), &[], "hello\n").expect("run binary");

        assert_eq!(output.exit_code, 0);
        assert_eq!(
            output.stdout,
            format!("{}|hello", tmp.path().display()),
            "runner must set DUUMBI_WORKSPACE_ROOT and pass stdin"
        );
    }
}
