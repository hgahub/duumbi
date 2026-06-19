//! Deterministic digest and artifact-key helpers for replay evidence.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Marker used when an optional replay state input is missing.
pub const ABSENT_STATE_HASH: &str = "absent";

/// Errors produced while computing replay digests.
#[derive(Debug, Error)]
pub enum DigestError {
    /// A filesystem operation failed.
    #[error("{context} at {path}: {source}")]
    Io {
        /// Human-readable operation.
        context: &'static str,
        /// Path involved in the failure.
        path: String,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// A path could not be represented safely in replay evidence.
    #[error("path is not valid UTF-8 for replay evidence: {0}")]
    NonUtf8Path(String),
    /// A graph file was outside the expected graph directory.
    #[error("graph file is outside graph root: {0}")]
    OutsideGraphRoot(String),
}

/// Hashes recorded for local dependency and registry state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceStateHashes {
    /// Hash of `.duumbi/deps.lock`, or `absent`.
    pub lockfile_hash: String,
    /// Hash of `.duumbi/config.toml`, or `absent`.
    pub workspace_dependency_config_hash: String,
    /// Combined hash over the bounded local registry/dependency state fields.
    pub registry_state_hash: String,
}

/// Computes an exact SHA-256 digest for all `.jsonld` graph files.
///
/// This is intentionally different from semantic hashing: it includes sorted
/// workspace-relative paths and exact file bytes, so `@id` differences and
/// serialization differences remain visible.
///
/// # Errors
///
/// Returns [`DigestError`] when graph files cannot be listed, read, or
/// represented as safe UTF-8 relative paths.
#[must_use = "exact graph digest evidence should be used"]
pub fn exact_graph_digest(graph_dir: &Path) -> Result<String, DigestError> {
    let mut files = BTreeSet::new();
    collect_jsonld_files(graph_dir, graph_dir, &mut files)?;

    let mut hasher = Sha256::new();
    for relative_path in files {
        let absolute_path = graph_dir.join(&relative_path);
        let bytes = fs::read(&absolute_path).map_err(|source| DigestError::Io {
            context: "failed to read graph file",
            path: absolute_path.display().to_string(),
            source,
        })?;
        let normalized_path = normalize_relative_path(&relative_path)?;
        hasher.update(b"path\0");
        hasher.update(normalized_path.as_bytes());
        hasher.update(b"\0len\0");
        hasher.update(bytes.len().to_string().as_bytes());
        hasher.update(b"\0bytes\0");
        hasher.update(&bytes);
        hasher.update(b"\0end\0");
    }

    Ok(hex_encode(hasher.finalize()))
}

/// Produces a filesystem-safe artifact key for a raw task or provider value.
///
/// The returned key keeps a readable slug prefix and appends a short stable
/// hash of the raw value. Raw provider routes may contain URLs or `..`; those
/// bytes are never used directly as path components.
#[must_use]
pub fn safe_artifact_key(raw: &str, fallback_prefix: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() || slug == "." || slug == ".." {
        slug = fallback_prefix
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
            .collect();
    }
    if slug.is_empty() {
        slug = "value".to_string();
    }
    if slug.len() > 48 {
        slug.truncate(48);
        while slug.ends_with('-') {
            slug.pop();
        }
    }

    format!("{slug}-{}", short_hash(raw))
}

/// Hashes a file when present, otherwise returns [`ABSENT_STATE_HASH`].
///
/// # Errors
///
/// Returns [`DigestError`] if the file exists but cannot be read.
#[must_use = "state hash evidence should be used"]
pub fn file_hash_or_absent(path: &Path) -> Result<String, DigestError> {
    if !path.exists() {
        return Ok(ABSENT_STATE_HASH.to_string());
    }
    let bytes = fs::read(path).map_err(|source| DigestError::Io {
        context: "failed to read state file",
        path: path.display().to_string(),
        source,
    })?;
    Ok(hex_encode(Sha256::digest(&bytes)))
}

/// Computes bounded local workspace state hashes for replay reports.
///
/// The first implementation hashes local lockfile/config evidence only and does
/// not query remote registries.
///
/// # Errors
///
/// Returns [`DigestError`] if a present state file cannot be read.
#[must_use = "workspace state hashes should be recorded"]
pub fn workspace_state_hashes(workspace_root: &Path) -> Result<WorkspaceStateHashes, DigestError> {
    let lockfile_hash = file_hash_or_absent(&workspace_root.join(".duumbi/deps.lock"))?;
    let workspace_dependency_config_hash =
        file_hash_or_absent(&workspace_root.join(".duumbi/config.toml"))?;
    let registry_state_hash =
        combined_registry_state_hash(&lockfile_hash, &workspace_dependency_config_hash);
    Ok(WorkspaceStateHashes {
        lockfile_hash,
        workspace_dependency_config_hash,
        registry_state_hash,
    })
}

/// Combines bounded local registry/dependency state fields into one hash.
#[must_use]
pub fn combined_registry_state_hash(lockfile_hash: &str, dependency_config_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"duumbi-determinism-registry-state-v1\0");
    hasher.update(b"lockfile\0");
    hasher.update(lockfile_hash.as_bytes());
    hasher.update(b"\0dependency-config\0");
    hasher.update(dependency_config_hash.as_bytes());
    hex_encode(hasher.finalize())
}

fn collect_jsonld_files(
    graph_root: &Path,
    dir: &Path,
    files: &mut BTreeSet<PathBuf>,
) -> Result<(), DigestError> {
    let entries = fs::read_dir(dir).map_err(|source| DigestError::Io {
        context: "failed to read graph directory",
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| DigestError::Io {
            context: "failed to read graph directory entry",
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonld_files(graph_root, &path, files)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("jsonld") {
            let relative = path
                .strip_prefix(graph_root)
                .map_err(|_| DigestError::OutsideGraphRoot(path.display().to_string()))?;
            files.insert(relative.to_path_buf());
        }
    }
    Ok(())
}

fn normalize_relative_path(path: &Path) -> Result<String, DigestError> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part
                    .to_str()
                    .ok_or_else(|| DigestError::NonUtf8Path(path.display().to_string()))?;
                parts.push(part.to_string());
            }
            _ => return Err(DigestError::OutsideGraphRoot(path.display().to_string())),
        }
    }
    Ok(parts.join("/"))
}

fn short_hash(raw: &str) -> String {
    let hash = hex_encode(Sha256::digest(raw.as_bytes()));
    hash.chars().take(12).collect()
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("invariant: writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn exact_graph_digest_changes_when_exact_bytes_change() {
        let dir = TempDir::new().expect("temp dir");
        let graph_dir = dir.path().join(".duumbi/graph");
        fs::create_dir_all(&graph_dir).expect("graph dir");
        fs::write(graph_dir.join("main.jsonld"), r#"{"@id":"a","value":1}"#).expect("write");
        let first = exact_graph_digest(&graph_dir).expect("digest");

        fs::write(graph_dir.join("main.jsonld"), r#"{"@id":"b","value":1}"#).expect("write");
        let second = exact_graph_digest(&graph_dir).expect("digest");

        assert_ne!(first, second);
    }

    #[test]
    fn exact_graph_digest_sorts_nested_paths() {
        let dir = TempDir::new().expect("temp dir");
        let graph_dir = dir.path().join(".duumbi/graph");
        fs::create_dir_all(graph_dir.join("nested")).expect("graph dir");
        fs::write(graph_dir.join("z.jsonld"), "{}").expect("write");
        fs::write(graph_dir.join("nested/a.jsonld"), "{}").expect("write");

        let first = exact_graph_digest(&graph_dir).expect("digest");
        let second = exact_graph_digest(&graph_dir).expect("digest");

        assert_eq!(first, second);
    }

    #[test]
    fn artifact_key_sanitizes_urls_and_path_traversal() {
        let key = safe_artifact_key("minimax:http://localhost:8080/v1/../../key", "provider");

        assert!(!key.contains('/'));
        assert!(!key.contains(".."));
        assert!(key.starts_with("minimax-http-localhost-8080-v1-key-"));
    }

    #[test]
    fn missing_state_file_is_absent() {
        let dir = TempDir::new().expect("temp dir");
        let hash = file_hash_or_absent(&dir.path().join(".duumbi/deps.lock")).expect("state hash");

        assert_eq!(hash, ABSENT_STATE_HASH);
    }

    #[test]
    fn workspace_state_hash_combines_absent_markers() {
        let dir = TempDir::new().expect("temp dir");
        let state = workspace_state_hashes(dir.path()).expect("state hashes");

        assert_eq!(state.lockfile_hash, ABSENT_STATE_HASH);
        assert_eq!(state.workspace_dependency_config_hash, ABSENT_STATE_HASH);
        assert_ne!(state.registry_state_hash, ABSENT_STATE_HASH);
    }
}
