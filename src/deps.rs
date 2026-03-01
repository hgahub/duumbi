//! Dependency management — lockfile and multi-workspace program loading.
//!
//! Reads local path dependencies from `.duumbi/config.toml`, resolves them,
//! generates a deterministic `.duumbi/deps.lock` on each build, and
//! provides `load_program_with_deps()` to build a `Program` from all modules.

#![allow(dead_code)] // Many functions used by upcoming build command (#61)

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{self, DuumbiConfig};
use crate::graph::program::{Program, ProgramError};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by dependency resolution and lockfile operations.
#[allow(dead_code)] // Variants used by CLI and upcoming build integration (#61)
#[derive(Debug, Error)]
pub enum DepsError {
    /// A dependency path does not exist or has no `.duumbi/graph/` directory.
    #[error("Dependency '{name}' at path '{path}' is not a valid duumbi workspace: {reason}")]
    InvalidDepPath {
        /// Dependency name.
        name: String,
        /// Path that was tried.
        path: String,
        /// Reason it failed.
        reason: String,
    },

    /// I/O error while reading or writing.
    #[error("I/O error for '{path}': {source}")]
    Io {
        /// File or directory path.
        path: String,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// TOML parse error in the lockfile.
    #[error("Failed to parse deps.lock: {0}")]
    LockfileParse(#[from] toml::de::Error),

    /// Config error (from config.toml).
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),

    /// Program loading error.
    #[error("Program loading failed")]
    Program(Vec<ProgramError>),
}

// ---------------------------------------------------------------------------
// Lockfile types
// ---------------------------------------------------------------------------

/// A single resolved dependency entry in the lockfile.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LockEntry {
    /// Dependency name as declared in `config.toml`.
    pub name: String,
    /// Resolved absolute path to the dependency workspace.
    pub path: String,
    /// FNV-1a content fingerprint of all `.jsonld` files in the dep's graph dir.
    pub hash: String,
}

/// The `.duumbi/deps.lock` file contents.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DepsLock {
    /// Lockfile format version.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Resolved dependency entries.
    #[serde(default)]
    pub dependencies: Vec<LockEntry>,
}

fn default_version() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// Content hashing — FNV-1a (no external crates needed)
// ---------------------------------------------------------------------------

/// Computes an FNV-1a fingerprint of a byte slice.
fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Computes a fingerprint of all `.jsonld` files in a graph directory.
///
/// Files are sorted by name for determinism. Returns a hex string.
fn graph_dir_hash(graph_dir: &Path) -> Result<String, DepsError> {
    let mut combined: u64 = 0xcbf29ce484222325;

    let mut paths: Vec<PathBuf> = fs::read_dir(graph_dir)
        .map_err(|e| DepsError::Io {
            path: graph_dir.display().to_string(),
            source: e,
        })?
        .flatten()
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("jsonld"))
        .map(|e| e.path())
        .collect();

    paths.sort();

    for path in &paths {
        let content = fs::read(path).map_err(|e| DepsError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let file_hash = fnv1a(&content);
        // Mix the file hash into the combined hash
        combined ^= file_hash;
        combined = combined.wrapping_mul(0x100000001b3);
    }

    Ok(format!("{combined:016x}"))
}

// ---------------------------------------------------------------------------
// Lockfile I/O
// ---------------------------------------------------------------------------

/// Loads the lockfile from `<workspace>/.duumbi/deps.lock`.
///
/// Returns an empty [`DepsLock`] if the file does not exist.
#[must_use = "lock load errors should be handled"]
pub fn load_lockfile(workspace: &Path) -> Result<DepsLock, DepsError> {
    let lock_path = workspace.join(".duumbi").join("deps.lock");
    if !lock_path.exists() {
        return Ok(DepsLock::default());
    }
    let contents = fs::read_to_string(&lock_path).map_err(|e| DepsError::Io {
        path: lock_path.display().to_string(),
        source: e,
    })?;
    Ok(toml::from_str(&contents)?)
}

/// Generates and saves the lockfile to `<workspace>/.duumbi/deps.lock`.
///
/// Resolves all dependencies from `config.toml` and computes content hashes.
#[must_use = "lockfile generation errors should be handled"]
pub fn generate_lockfile(workspace: &Path, config: &DuumbiConfig) -> Result<DepsLock, DepsError> {
    let mut entries = Vec::new();

    for (name, dep) in &config.dependencies {
        let dep_path = resolve_dep_path(workspace, &dep.path)?;
        validate_dep_workspace(&dep_path, name)?;

        let graph_dir = dep_path.join(".duumbi").join("graph");
        let hash = graph_dir_hash(&graph_dir)?;

        entries.push(LockEntry {
            name: name.clone(),
            path: dep_path.display().to_string(),
            hash,
        });
    }

    // Sort for determinism
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    let lock = DepsLock {
        version: 1,
        dependencies: entries,
    };

    let lock_path = workspace.join(".duumbi").join("deps.lock");
    let contents = toml::to_string_pretty(&lock).map_err(|e| DepsError::Io {
        path: lock_path.display().to_string(),
        source: std::io::Error::other(e.to_string()),
    })?;
    fs::write(&lock_path, contents).map_err(|e| DepsError::Io {
        path: lock_path.display().to_string(),
        source: e,
    })?;

    Ok(lock)
}

// ---------------------------------------------------------------------------
// Path resolution and validation
// ---------------------------------------------------------------------------

/// Resolves a dependency path relative to the workspace root.
///
/// If the path is absolute it is used as-is; otherwise it is joined
/// to `workspace`.
fn resolve_dep_path(workspace: &Path, dep_path: &str) -> Result<PathBuf, DepsError> {
    let path = Path::new(dep_path);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };

    // Canonicalize to get an absolute path (also validates existence)
    resolved.canonicalize().map_err(|e| DepsError::Io {
        path: resolved.display().to_string(),
        source: e,
    })
}

/// Validates that a resolved path is a duumbi workspace with a graph directory.
fn validate_dep_workspace(dep_path: &Path, name: &str) -> Result<(), DepsError> {
    let graph_dir = dep_path.join(".duumbi").join("graph");
    if !graph_dir.exists() {
        return Err(DepsError::InvalidDepPath {
            name: name.to_string(),
            path: dep_path.display().to_string(),
            reason: "no .duumbi/graph/ directory found".to_string(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Program loading with dependencies
// ---------------------------------------------------------------------------

/// Loads a [`Program`] from a workspace, including all declared dependencies.
///
/// If the workspace has no `config.toml` or no `[dependencies]`, falls back
/// to loading from the workspace's `.duumbi/graph/` directory only (same as
/// `Program::load()`).
///
/// On success, also generates/updates the lockfile.
///
/// # Errors
///
/// Returns an error if any dependency path is invalid or if module loading fails.
#[must_use = "program load errors should be handled"]
pub fn load_program_with_deps(workspace: &Path) -> Result<Program, DepsError> {
    // Load config (optional)
    let config = config::load_config(workspace).unwrap_or_default();

    // Generate lockfile for all deps
    generate_lockfile(workspace, &config)?;

    // Collect all graph directories: workspace first, then deps
    let mut graph_dirs: Vec<PathBuf> = Vec::new();
    let workspace_graph = workspace.join(".duumbi").join("graph");
    if workspace_graph.exists() {
        graph_dirs.push(workspace_graph);
    }

    for (name, dep) in &config.dependencies {
        let dep_path = resolve_dep_path(workspace, &dep.path)?;
        validate_dep_workspace(&dep_path, name)?;
        graph_dirs.push(dep_path.join(".duumbi").join("graph"));
    }

    let graph_dir_refs: Vec<&Path> = graph_dirs.iter().map(|p| p.as_path()).collect();
    Program::load_from_dirs(&graph_dir_refs).map_err(DepsError::Program)
}

// ---------------------------------------------------------------------------
// Dependency CRUD (for deps add/remove commands)
// ---------------------------------------------------------------------------

/// Resolved dependency entry: `(name, declared_path, resolved_abs_path_or_error)`.
pub type DepEntry = (String, String, Result<PathBuf, String>);

/// Adds a dependency to `config.toml`.
///
/// Validates the dependency path before writing. Returns an error if the path
/// is not a valid duumbi workspace.
#[must_use = "deps add errors should be handled"]
pub fn add_dependency(workspace: &Path, name: &str, dep_path: &str) -> Result<(), DepsError> {
    // Validate the path first
    let resolved = resolve_dep_path(workspace, dep_path)?;
    validate_dep_workspace(&resolved, name)?;

    let mut config = config::load_config(workspace).unwrap_or_default();
    config.dependencies.insert(
        name.to_string(),
        config::DependencyConfig {
            path: dep_path.to_string(),
        },
    );

    config::save_config(workspace, &config).map_err(DepsError::Config)
}

/// Removes a dependency from `config.toml`.
///
/// Returns `true` if the dependency was found and removed, `false` if not found.
#[must_use = "deps remove errors should be handled"]
pub fn remove_dependency(workspace: &Path, name: &str) -> Result<bool, DepsError> {
    let mut config = config::load_config(workspace).unwrap_or_default();
    let removed = config.dependencies.remove(name).is_some();
    if removed {
        config::save_config(workspace, &config).map_err(DepsError::Config)?;
    }
    Ok(removed)
}

/// Lists all declared dependencies with their resolution status.
///
/// Returns `(name, dep_path, resolved_path_or_error)` for each dependency.
#[must_use = "deps list errors should be handled"]
pub fn list_dependencies(workspace: &Path) -> Result<Vec<DepEntry>, DepsError> {
    let config = config::load_config(workspace).unwrap_or_default();
    let mut out = Vec::new();

    for (name, dep) in &config.dependencies {
        let resolution = resolve_dep_path(workspace, &dep.path).map_err(|e| e.to_string());
        out.push((name.clone(), dep.path.clone(), resolution));
    }

    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_workspace(dir: &Path) {
        let graph = dir.join(".duumbi").join("graph");
        fs::create_dir_all(&graph).expect("invariant: must create graph dir");
        // Write a minimal valid module
        fs::write(
            graph.join("main.jsonld"),
            r#"{
    "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
    "@type": "duumbi:Module",
    "@id": "duumbi:main",
    "duumbi:name": "main",
    "duumbi:functions": [{
        "@type": "duumbi:Function",
        "@id": "duumbi:main/main",
        "duumbi:name": "main",
        "duumbi:returnType": "i64",
        "duumbi:blocks": [{
            "@type": "duumbi:Block",
            "@id": "duumbi:main/main/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
                  "duumbi:value": 0, "duumbi:resultType": "i64"},
                {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1",
                  "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}}
            ]
        }]
    }]
}"#,
        )
        .expect("invariant: must write module");
    }

    fn make_lib_workspace(dir: &Path, module_name: &str) {
        let graph = dir.join(".duumbi").join("graph");
        fs::create_dir_all(&graph).expect("invariant: must create graph dir");
        fs::write(
            graph.join(format!("{module_name}.jsonld")),
            format!(
                r#"{{
    "@context": {{"duumbi": "https://duumbi.dev/ns/core#"}},
    "@type": "duumbi:Module",
    "@id": "duumbi:{module_name}",
    "duumbi:name": "{module_name}",
    "duumbi:exports": ["helper"],
    "duumbi:functions": [{{
        "@type": "duumbi:Function",
        "@id": "duumbi:{module_name}/helper",
        "duumbi:name": "helper",
        "duumbi:returnType": "i64",
        "duumbi:blocks": [{{
            "@type": "duumbi:Block",
            "@id": "duumbi:{module_name}/helper/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {{"@type": "duumbi:Const", "@id": "duumbi:{module_name}/helper/entry/0",
                  "duumbi:value": 1, "duumbi:resultType": "i64"}},
                {{"@type": "duumbi:Return", "@id": "duumbi:{module_name}/helper/entry/1",
                  "duumbi:operand": {{"@id": "duumbi:{module_name}/helper/entry/0"}}}}
            ]
        }}]
    }}]
}}"#
            ),
        )
        .expect("invariant: must write lib module");
    }

    #[test]
    fn load_program_with_deps_no_config_uses_workspace_graph() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let program = load_program_with_deps(ws.path()).expect("must load");
        assert_eq!(program.modules.len(), 1);
    }

    #[test]
    fn add_dependency_writes_to_config() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add dep");

        let config = config::load_config(ws.path()).expect("config must exist");
        assert!(config.dependencies.contains_key("mylib"));
        assert_eq!(config.dependencies["mylib"].path, dep_path);
    }

    #[test]
    fn remove_dependency_removes_from_config() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add dep");
        let removed = remove_dependency(ws.path(), "mylib").expect("must remove");
        assert!(removed, "dep must be removed");

        let config = config::load_config(ws.path()).expect("config must exist");
        assert!(!config.dependencies.contains_key("mylib"));
    }

    #[test]
    fn remove_nonexistent_dependency_returns_false() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let removed = remove_dependency(ws.path(), "nonexistent").expect("must not error");
        assert!(!removed);
    }

    #[test]
    fn add_invalid_dep_path_returns_error() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let result = add_dependency(ws.path(), "bad", "/nonexistent/path/xyz");
        assert!(result.is_err(), "invalid path must error");
    }

    #[test]
    fn generate_lockfile_creates_file_with_hashes() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add");

        let config = config::load_config(ws.path()).expect("config");
        let lock = generate_lockfile(ws.path(), &config).expect("lockfile");
        assert_eq!(lock.dependencies.len(), 1);
        assert_eq!(lock.dependencies[0].name, "mylib");
        assert!(!lock.dependencies[0].hash.is_empty());

        // Lockfile file must exist on disk
        let lock_path = ws.path().join(".duumbi").join("deps.lock");
        assert!(lock_path.exists(), "deps.lock must be created");
    }

    #[test]
    fn load_program_with_deps_includes_dep_modules() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add");

        let program = load_program_with_deps(ws.path()).expect("must load");
        assert_eq!(program.modules.len(), 2, "must have main + mylib modules");
    }

    #[test]
    fn list_dependencies_shows_all_deps() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add");

        let deps = list_dependencies(ws.path()).expect("must list");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "mylib");
        assert!(deps[0].2.is_ok(), "dep must resolve");
    }
}
