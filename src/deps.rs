//! Dependency management — lockfile, module resolution, and multi-workspace program loading.
//!
//! Reads dependencies from `.duumbi/config.toml`, resolves them through the
//! three-layer pipeline (workspace → vendor → cache), generates a deterministic
//! `.duumbi/deps.lock`, and provides `load_program_with_deps()` to build a
//! `Program` from all modules.
//!
//! # Resolution order
//!
//! 1. **Workspace** (`.duumbi/graph/`) — own source, highest priority
//! 2. **Vendor** (`.duumbi/vendor/`) — pinned, audited copies
//! 3. **Cache** (`.duumbi/cache/`) — stdlib and downloaded modules
//! 4. ❌ **Not found** → `E011 DependencyNotFound`

#![allow(dead_code)] // Many functions used by upcoming build command (#61)

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{self, DependencyConfig, DuumbiConfig};
use crate::graph::program::{Program, ProgramError};
use crate::manifest::{self, ModuleManifest};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by dependency resolution and lockfile operations.
#[allow(dead_code)] // Variants used by CLI and upcoming build integration (#61)
#[derive(Debug, Error)]
pub enum DepsError {
    /// A dependency path does not exist or has no graph directory.
    #[error("Dependency '{name}' at path '{path}' is not a valid duumbi workspace: {reason}")]
    InvalidDepPath {
        /// Dependency name.
        name: String,
        /// Path that was tried.
        path: String,
        /// Reason it failed.
        reason: String,
    },

    /// A version-based dependency could not be found in any resolution layer.
    #[error("Dependency '{name}@{version}' not found in vendor or cache layers")]
    NotFound {
        /// Dependency name (may include scope, e.g. `@duumbi/stdlib-math`).
        name: String,
        /// Version that was requested.
        version: String,
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
    /// Resolved absolute path to the dependency's graph directory.
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
// Module resolution types (#77)
// ---------------------------------------------------------------------------

/// Which resolution layer a module was found in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleSource {
    /// Found in the workspace's own `.duumbi/graph/` directory.
    Workspace,
    /// Found in `.duumbi/vendor/@scope/name/graph/`.
    Vendor,
    /// Found in `.duumbi/cache/@scope/name@version/graph/`.
    Cache,
}

/// A successfully resolved module with its graph directory and optional manifest.
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// Module key as declared in `config.toml`.
    pub name: String,
    /// Which layer this module was resolved from.
    pub source: ModuleSource,
    /// Absolute path to the module's graph directory.
    pub graph_dir: PathBuf,
    /// Parsed `manifest.toml`, if present.
    pub manifest: Option<ModuleManifest>,
}

// ---------------------------------------------------------------------------
// Scope / name parsing helpers
// ---------------------------------------------------------------------------

/// Splits a scoped module key `"@scope/name"` into `("@scope", "name")`.
///
/// Returns `None` if the key is not scoped (i.e. does not start with `@`).
fn parse_scoped_name(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix('@')?;
    let slash = rest.find('/')?;
    // scope includes the leading '@'
    let scope = &key[..slash + 1]; // e.g. "@duumbi"
    let name = &rest[slash + 1..]; // e.g. "stdlib-math"
    Some((scope, name))
}

/// Builds the cache graph directory for a scoped module at a given version.
///
/// Layout: `.duumbi/cache/@scope/name@version/graph/`
fn cache_entry_graph_dir(workspace: &Path, scope: &str, name: &str, version: &str) -> PathBuf {
    workspace
        .join(".duumbi")
        .join("cache")
        .join(scope)
        .join(format!("{name}@{version}"))
        .join("graph")
}

/// Builds the vendor graph directory for a scoped module.
///
/// Layout: `.duumbi/vendor/@scope/name/graph/`
fn vendor_entry_graph_dir(workspace: &Path, scope: &str, name: &str) -> PathBuf {
    workspace
        .join(".duumbi")
        .join("vendor")
        .join(scope)
        .join(name)
        .join("graph")
}

/// Reads `manifest.toml` from the parent of the given graph directory, if it exists.
fn try_read_manifest(graph_dir: &Path) -> Option<ModuleManifest> {
    let manifest_path = graph_dir.parent()?.join("manifest.toml");
    manifest::parse_manifest(&manifest_path).ok()
}

// ---------------------------------------------------------------------------
// Module resolution pipeline (#77)
// ---------------------------------------------------------------------------

/// Resolves a version-pinned dependency through the three-layer pipeline.
///
/// Resolution order: vendor → cache → `E011 NotFound`.
/// (Workspace layer is handled separately by `load_program_with_deps`.)
///
/// # Errors
///
/// Returns [`DepsError::NotFound`] if the module is not present in any layer.
pub fn resolve_module(
    workspace: &Path,
    module_key: &str,
    version: &str,
) -> Result<ResolvedModule, DepsError> {
    if let Some((scope, name)) = parse_scoped_name(module_key) {
        // 1. Vendor layer
        let vendor_dir = vendor_entry_graph_dir(workspace, scope, name);
        if vendor_dir.exists() {
            return Ok(ResolvedModule {
                name: module_key.to_string(),
                source: ModuleSource::Vendor,
                manifest: try_read_manifest(&vendor_dir),
                graph_dir: vendor_dir,
            });
        }

        // 2. Cache layer
        let cache_dir = cache_entry_graph_dir(workspace, scope, name, version);
        if cache_dir.exists() {
            return Ok(ResolvedModule {
                name: module_key.to_string(),
                source: ModuleSource::Cache,
                manifest: try_read_manifest(&cache_dir),
                graph_dir: cache_dir,
            });
        }
    }

    Err(DepsError::NotFound {
        name: module_key.to_string(),
        version: version.to_string(),
    })
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
/// Handles both path-based and version-based (cache) dependencies.
#[must_use = "lockfile generation errors should be handled"]
pub fn generate_lockfile(workspace: &Path, config: &DuumbiConfig) -> Result<DepsLock, DepsError> {
    let mut entries = Vec::new();

    for (name, dep) in &config.dependencies {
        let graph_dir = match dep {
            DependencyConfig::Path { path } => {
                let dep_path = resolve_dep_path(workspace, path)?;
                validate_dep_workspace(&dep_path, name)?;
                dep_path.join(".duumbi").join("graph")
            }
            DependencyConfig::Version(version) => {
                let resolved = resolve_module(workspace, name, version)?;
                resolved.graph_dir
            }
        };

        let hash = graph_dir_hash(&graph_dir)?;
        entries.push(LockEntry {
            name: name.clone(),
            path: graph_dir.display().to_string(),
            hash,
        });
    }

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
// Path resolution and validation (for path-based deps)
// ---------------------------------------------------------------------------

/// Resolves a path dependency relative to the workspace root.
///
/// If the path is absolute it is used as-is; otherwise it is joined to `workspace`.
fn resolve_dep_path(workspace: &Path, dep_path: &str) -> Result<PathBuf, DepsError> {
    let path = Path::new(dep_path);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };
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
/// Resolves through the three-layer pipeline (workspace → vendor → cache) for
/// version-based deps. Path-based deps are resolved relative to the workspace root.
///
/// On success, also generates/updates the lockfile.
#[must_use = "program load errors should be handled"]
pub fn load_program_with_deps(workspace: &Path) -> Result<Program, DepsError> {
    let config = config::load_config(workspace).unwrap_or_default();

    generate_lockfile(workspace, &config)?;

    let mut graph_dirs: Vec<PathBuf> = Vec::new();
    let workspace_graph = workspace.join(".duumbi").join("graph");
    if workspace_graph.exists() {
        graph_dirs.push(workspace_graph);
    }

    for (name, dep) in &config.dependencies {
        let graph_dir = match dep {
            DependencyConfig::Path { path } => {
                let dep_path = resolve_dep_path(workspace, path)?;
                validate_dep_workspace(&dep_path, name)?;
                dep_path.join(".duumbi").join("graph")
            }
            DependencyConfig::Version(version) => {
                let resolved = resolve_module(workspace, name, version)?;
                resolved.graph_dir
            }
        };
        graph_dirs.push(graph_dir);
    }

    let graph_dir_refs: Vec<&Path> = graph_dirs.iter().map(|p| p.as_path()).collect();
    Program::load_from_dirs(&graph_dir_refs).map_err(DepsError::Program)
}

// ---------------------------------------------------------------------------
// Dependency CRUD (for deps add/remove commands)
// ---------------------------------------------------------------------------

/// Resolved dependency entry: `(name, declared_spec, resolved_abs_path_or_error)`.
pub type DepEntry = (String, String, Result<PathBuf, String>);

/// Adds a path-based dependency to `config.toml`.
///
/// Validates the path before writing. Returns an error if the path is not a
/// valid duumbi workspace.
#[must_use = "deps add errors should be handled"]
pub fn add_dependency(workspace: &Path, name: &str, dep_path: &str) -> Result<(), DepsError> {
    let resolved = resolve_dep_path(workspace, dep_path)?;
    validate_dep_workspace(&resolved, name)?;

    let mut config = config::load_config(workspace).unwrap_or_default();
    config.dependencies.insert(
        name.to_string(),
        DependencyConfig::Path {
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
/// Returns `(name, declared_spec, resolved_graph_dir_or_error)` for each dependency.
#[must_use = "deps list errors should be handled"]
pub fn list_dependencies(workspace: &Path) -> Result<Vec<DepEntry>, DepsError> {
    let config = config::load_config(workspace).unwrap_or_default();
    let mut out = Vec::new();

    for (name, dep) in &config.dependencies {
        let (spec, resolution) = match dep {
            DependencyConfig::Path { path } => {
                let res = resolve_dep_path(workspace, path)
                    .map(|p| p.join(".duumbi").join("graph"))
                    .map_err(|e| e.to_string());
                (path.clone(), res)
            }
            DependencyConfig::Version(version) => {
                let res = resolve_module(workspace, name, version)
                    .map(|r| r.graph_dir)
                    .map_err(|e| e.to_string());
                (version.clone(), res)
            }
        };
        out.push((name.clone(), spec, resolution));
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

    fn make_cache_module(workspace: &Path, scope: &str, name: &str, version: &str) {
        let graph_dir = cache_entry_graph_dir(workspace, scope, name, version);
        fs::create_dir_all(&graph_dir).expect("create cache dir");
        make_lib_workspace_in_graph(&graph_dir, name);
    }

    /// Writes a minimal lib jsonld directly into an arbitrary graph dir.
    fn make_lib_workspace_in_graph(graph_dir: &Path, module_name: &str) {
        fs::write(
            graph_dir.join(format!("{module_name}.jsonld")),
            format!(
                r#"{{
    "@context": {{"duumbi": "https://duumbi.dev/ns/core#"}},
    "@type": "duumbi:Module",
    "@id": "duumbi:{module_name}",
    "duumbi:name": "{module_name}",
    "duumbi:functions": []
}}"#
            ),
        )
        .expect("write cache module");
    }

    // -------------------------------------------------------------------------
    // Existing tests (path-based deps)
    // -------------------------------------------------------------------------

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
        assert_eq!(config.dependencies["mylib"].path(), Some(dep_path.as_str()));
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

        assert!(
            ws.path().join(".duumbi").join("deps.lock").exists(),
            "deps.lock must be created"
        );
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

    // -------------------------------------------------------------------------
    // New tests: module resolution pipeline (#77)
    // -------------------------------------------------------------------------

    #[test]
    fn parse_scoped_name_splits_correctly() {
        let (scope, name) = parse_scoped_name("@duumbi/stdlib-math").expect("must parse");
        assert_eq!(scope, "@duumbi");
        assert_eq!(name, "stdlib-math");
    }

    #[test]
    fn parse_scoped_name_returns_none_for_unscoped() {
        assert!(parse_scoped_name("mylib").is_none());
        assert!(parse_scoped_name("no-scope").is_none());
    }

    #[test]
    fn resolve_module_finds_cache_entry() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

        let resolved =
            resolve_module(ws.path(), "@duumbi/stdlib-math", "1.0.0").expect("must resolve");
        assert_eq!(resolved.source, ModuleSource::Cache);
        assert!(resolved.graph_dir.exists());
    }

    #[test]
    fn resolve_module_vendor_takes_priority_over_cache() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        // Both vendor and cache exist
        make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");
        let vendor_dir = vendor_entry_graph_dir(ws.path(), "@duumbi", "stdlib-math");
        fs::create_dir_all(&vendor_dir).expect("create vendor dir");
        make_lib_workspace_in_graph(&vendor_dir, "stdlib-math");

        let resolved =
            resolve_module(ws.path(), "@duumbi/stdlib-math", "1.0.0").expect("must resolve");
        assert_eq!(resolved.source, ModuleSource::Vendor, "vendor must win");
    }

    #[test]
    fn resolve_module_not_found_returns_error() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());

        let result = resolve_module(ws.path(), "@duumbi/nonexistent", "1.0.0");
        assert!(matches!(result, Err(DepsError::NotFound { .. })));
    }

    #[test]
    fn resolve_module_reads_manifest_when_present() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

        // Write a manifest alongside the graph
        let cache_root = ws.path().join(".duumbi/cache/@duumbi/stdlib-math@1.0.0");
        let m = crate::manifest::ModuleManifest::new(
            "@duumbi/stdlib-math",
            "1.0.0",
            "Math stdlib",
            vec!["abs".into()],
        );
        crate::manifest::write_manifest(&cache_root, &m).expect("write manifest");

        let resolved =
            resolve_module(ws.path(), "@duumbi/stdlib-math", "1.0.0").expect("must resolve");
        assert!(resolved.manifest.is_some(), "manifest must be loaded");
        assert_eq!(
            resolved.manifest.unwrap().module.name,
            "@duumbi/stdlib-math"
        );
    }

    #[test]
    fn version_dep_in_config_resolves_from_cache() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

        // Write config with a version-based dep
        let mut cfg = DuumbiConfig::default();
        cfg.dependencies.insert(
            "@duumbi/stdlib-math".to_string(),
            DependencyConfig::Version("1.0.0".to_string()),
        );
        config::save_config(ws.path(), &cfg).expect("save config");

        let deps = list_dependencies(ws.path()).expect("must list");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "@duumbi/stdlib-math");
        assert!(deps[0].2.is_ok(), "cache dep must resolve");
    }
}
