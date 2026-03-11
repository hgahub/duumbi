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
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::config::{self, DependencyConfig, DuumbiConfig};
use crate::graph::program::{Program, ProgramError};
use crate::hash;
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
///
/// Supports both v0 (name/path/hash) and v1 (full provenance) formats.
/// When generating, all v1 fields are populated. When reading an old v0
/// lockfile, only `name`, `path`, and `hash` are present.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LockEntry {
    /// Dependency name as declared in `config.toml`.
    pub name: String,

    // --- v1 fields ---
    /// SemVer version string (e.g. `"1.0.0"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Provenance source: `"registry+<url>"`, `"path+<relative>"`, or `"vendor"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Semantic hash (SHA-256, `@id`-independent) from `hash::semantic_hash`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_hash: Option<String>,
    /// Integrity hash: SHA-256 of raw `.jsonld` file bytes for tamper detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
    /// Resolved path to the dependency's graph directory (relative or absolute).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
    /// Whether this dependency is vendored (copied into `.duumbi/vendor/`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub vendored: bool,

    // --- v0 compat fields (not written in v1 output) ---
    /// Resolved absolute path (v0 format). Superseded by `resolved_path` in v1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// FNV-1a content fingerprint (v0 format). Superseded by `semantic_hash`/`integrity` in v1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl LockEntry {
    /// Returns the resolved path, checking v1 `resolved_path` first, then v0 `path`.
    #[must_use]
    pub fn effective_path(&self) -> Option<&str> {
        self.resolved_path.as_deref().or(self.path.as_deref())
    }

    /// Returns `true` if this entry has v1 provenance fields populated.
    #[must_use]
    pub fn is_v1(&self) -> bool {
        self.semantic_hash.is_some() && self.integrity.is_some()
    }
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

/// Computes an FNV-1a fingerprint of a byte slice (v0 lockfile compat).
fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Computes an FNV-1a fingerprint of all `.jsonld` files in a graph directory (v0 compat).
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

/// Computes a SHA-256 integrity hash of all `.jsonld` file bytes in a graph directory.
///
/// Unlike [`hash::semantic_hash`], this hashes raw file bytes — any byte change
/// (whitespace, comments, `@id` values) produces a different hash. Used for
/// tamper detection in lockfile verification.
fn integrity_hash(graph_dir: &Path) -> Result<String, DepsError> {
    let mut paths: Vec<PathBuf> = fs::read_dir(graph_dir)
        .map_err(|e| DepsError::Io {
            path: graph_dir.display().to_string(),
            source: e,
        })?
        .map(|entry| {
            entry.map_err(|e| DepsError::Io {
                path: graph_dir.display().to_string(),
                source: e,
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("jsonld"))
        .map(|e| e.path())
        .collect();

    paths.sort();

    let mut hasher = Sha256::new();
    for path in &paths {
        // Include filename in hash so swapping file contents is detected
        if let Some(name) = path.file_name() {
            hasher.update(name.as_encoded_bytes());
        }
        let content = fs::read(path).map_err(|e| DepsError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        hasher.update(&content);
    }

    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        std::fmt::Write::write_fmt(&mut hex, format_args!("{byte:02x}"))
            .expect("invariant: writing to String never fails");
    }
    Ok(format!("sha256-{hex}"))
}

/// Determines the source provenance string for a dependency.
///
/// Format: `"path+<relative>"` for path deps, `"cache"` for locally cached,
/// `"vendor"` for vendored. Registry sources will use `"registry+<url>"` when
/// registry client is implemented.
fn dep_source_string(dep: &DependencyConfig, source: &ModuleSource) -> String {
    match dep {
        DependencyConfig::Path { path } => format!("path+{path}"),
        DependencyConfig::Version(_) | DependencyConfig::VersionWithRegistry { .. } => match source
        {
            ModuleSource::Vendor => "vendor".to_string(),
            ModuleSource::Cache => "cache".to_string(),
            ModuleSource::Workspace => "workspace".to_string(),
        },
    }
}

/// Extracts the version string from a dependency config.
fn dep_version_string(dep: &DependencyConfig) -> Option<String> {
    dep.version().map(|v| v.to_string())
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
/// Produces v1 format with full provenance tracking: version, source,
/// semantic hash, integrity hash, resolved path, and vendored flag.
/// Output is deterministic — entries are sorted by name, hashes are stable.
#[must_use = "lockfile generation errors should be handled"]
pub fn generate_lockfile(workspace: &Path, config: &DuumbiConfig) -> Result<DepsLock, DepsError> {
    let mut entries = Vec::new();

    for (name, dep) in &config.dependencies {
        let (graph_dir, mod_source) = match dep {
            DependencyConfig::Path { path } => {
                let dep_path = resolve_dep_path(workspace, path)?;
                validate_dep_workspace(&dep_path, name)?;
                (
                    dep_path.join(".duumbi").join("graph"),
                    ModuleSource::Workspace,
                )
            }
            DependencyConfig::Version(version)
            | DependencyConfig::VersionWithRegistry { version, .. } => {
                let resolved = resolve_module(workspace, name, version)?;
                let source = resolved.source;
                (resolved.graph_dir, source)
            }
        };

        let sem_hash =
            hash::semantic_hash(&graph_dir).map_err(|e: hash::HashError| DepsError::Io {
                path: graph_dir.display().to_string(),
                source: std::io::Error::other(e.to_string()),
            })?;
        let int_hash = integrity_hash(&graph_dir)?;
        let is_vendored = mod_source == ModuleSource::Vendor;

        entries.push(LockEntry {
            name: name.clone(),
            version: dep_version_string(dep),
            source: Some(dep_source_string(dep, &mod_source)),
            semantic_hash: Some(sem_hash),
            integrity: Some(int_hash),
            resolved_path: Some(graph_dir.display().to_string()),
            vendored: is_vendored,
            // v0 fields not written in v1 output
            path: None,
            hash: None,
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

/// Verifies that all lockfile entries match the current state on disk.
///
/// For v1 entries, checks the integrity hash (SHA-256 of raw file bytes).
/// Returns a list of entries that failed verification. An empty vec means all OK.
///
/// # Errors
///
/// Returns [`DepsError::Io`] if the graph directory cannot be read.
#[must_use = "verification errors should be handled"]
pub fn verify_lockfile(lock: &DepsLock) -> Result<Vec<LockIntegrityError>, DepsError> {
    let mut failures = Vec::new();

    for entry in &lock.dependencies {
        let Some(ref recorded_integrity) = entry.integrity else {
            // v0 entry — no integrity hash to verify
            continue;
        };
        let Some(ref resolved) = entry.effective_path() else {
            continue;
        };

        let graph_dir = Path::new(resolved);
        if !graph_dir.exists() {
            failures.push(LockIntegrityError {
                name: entry.name.clone(),
                expected: recorded_integrity.clone(),
                actual: "<missing>".to_string(),
            });
            continue;
        }

        match integrity_hash(graph_dir) {
            Ok(current_integrity) => {
                if current_integrity != *recorded_integrity {
                    failures.push(LockIntegrityError {
                        name: entry.name.clone(),
                        expected: recorded_integrity.clone(),
                        actual: current_integrity,
                    });
                }
            }
            Err(_) => {
                failures.push(LockIntegrityError {
                    name: entry.name.clone(),
                    expected: recorded_integrity.clone(),
                    actual: "<unreadable>".to_string(),
                });
            }
        }
    }

    Ok(failures)
}

/// A lockfile entry whose integrity hash does not match the files on disk.
#[derive(Debug, Clone)]
pub struct LockIntegrityError {
    /// Module name that failed verification.
    pub name: String,
    /// Integrity hash recorded in the lockfile.
    pub expected: String,
    /// Integrity hash computed from current files on disk.
    pub actual: String,
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
            DependencyConfig::Version(version)
            | DependencyConfig::VersionWithRegistry { version, .. } => {
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
            DependencyConfig::Version(version)
            | DependencyConfig::VersionWithRegistry { version, .. } => {
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
        assert!(
            lock.dependencies[0].integrity.is_some(),
            "v1 lockfile must have integrity hash"
        );
        assert!(
            lock.dependencies[0].semantic_hash.is_some(),
            "v1 lockfile must have semantic hash"
        );

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

    // -------------------------------------------------------------------------
    // Lockfile v1 tests (#155)
    // -------------------------------------------------------------------------

    #[test]
    fn lockfile_v1_has_all_provenance_fields() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add");

        let config = config::load_config(ws.path()).expect("config");
        let lock = generate_lockfile(ws.path(), &config).expect("lockfile");
        let entry = &lock.dependencies[0];

        assert_eq!(entry.name, "mylib");
        assert!(entry.source.as_deref() == Some(&format!("path+{dep_path}")));
        assert!(entry.semantic_hash.is_some());
        assert!(entry.integrity.is_some());
        assert!(entry.resolved_path.is_some());
        assert!(!entry.vendored);
        // v0 fields should be None
        assert!(entry.path.is_none());
        assert!(entry.hash.is_none());
        assert!(entry.is_v1());
    }

    #[test]
    fn lockfile_v1_deterministic_output() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add");

        let config = config::load_config(ws.path()).expect("config");

        // Generate twice — must be identical
        generate_lockfile(ws.path(), &config).expect("lockfile 1");
        let contents1 =
            fs::read_to_string(ws.path().join(".duumbi/deps.lock")).expect("read lockfile 1");
        generate_lockfile(ws.path(), &config).expect("lockfile 2");
        let contents2 =
            fs::read_to_string(ws.path().join(".duumbi/deps.lock")).expect("read lockfile 2");

        assert_eq!(
            contents1, contents2,
            "lockfile output must be deterministic"
        );
    }

    #[test]
    fn lockfile_v1_verify_passes_for_untampered() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add");

        let config = config::load_config(ws.path()).expect("config");
        let lock = generate_lockfile(ws.path(), &config).expect("lockfile");

        let failures = verify_lockfile(&lock).expect("verify must not error");
        assert!(
            failures.is_empty(),
            "untampered lockfile must pass verification"
        );
    }

    #[test]
    fn lockfile_v1_verify_detects_tampered_file() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        let dep_ws = tempfile::TempDir::new().expect("tempdir");
        make_lib_workspace(dep_ws.path(), "mylib");

        let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
        add_dependency(ws.path(), "mylib", &dep_path).expect("must add");

        let config = config::load_config(ws.path()).expect("config");
        let lock = generate_lockfile(ws.path(), &config).expect("lockfile");

        // Tamper with the dependency file
        let graph_dir = dep_ws.path().join(".duumbi/graph");
        fs::write(graph_dir.join("mylib.jsonld"), r#"{"tampered": true}"#).expect("tamper");

        let failures = verify_lockfile(&lock).expect("verify must not error");
        assert_eq!(failures.len(), 1, "tampered file must be detected");
        assert_eq!(failures[0].name, "mylib");
        assert_ne!(failures[0].expected, failures[0].actual);
    }

    #[test]
    fn lockfile_v1_backward_compat_reads_v0() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        let duumbi = ws.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("create .duumbi");

        // Write a v0-style lockfile
        let v0_content = r#"version = 1

[[dependencies]]
name = "mylib"
path = "/some/old/path"
hash = "0123456789abcdef"
"#;
        fs::write(duumbi.join("deps.lock"), v0_content).expect("write v0 lockfile");

        let lock = load_lockfile(ws.path()).expect("must load v0 lockfile");
        assert_eq!(lock.dependencies.len(), 1);
        assert_eq!(lock.dependencies[0].name, "mylib");
        assert_eq!(
            lock.dependencies[0].effective_path(),
            Some("/some/old/path")
        );
        assert!(!lock.dependencies[0].is_v1(), "v0 entry must not be v1");
    }

    #[test]
    fn lockfile_v1_version_dep_includes_version_field() {
        let ws = tempfile::TempDir::new().expect("tempdir");
        make_workspace(ws.path());
        make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

        let mut cfg = DuumbiConfig::default();
        cfg.dependencies.insert(
            "@duumbi/stdlib-math".to_string(),
            DependencyConfig::Version("1.0.0".to_string()),
        );
        config::save_config(ws.path(), &cfg).expect("save config");

        let config = config::load_config(ws.path()).expect("config");
        let lock = generate_lockfile(ws.path(), &config).expect("lockfile");
        let entry = &lock.dependencies[0];

        assert_eq!(entry.version.as_deref(), Some("1.0.0"));
        assert_eq!(entry.source.as_deref(), Some("cache"));
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
