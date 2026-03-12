//! Module packaging for registry distribution.
//!
//! Creates reproducible `.tar.gz` archives from a module's graph files and
//! manifest. The archive layout:
//!
//! ```text
//! manifest.toml         # Module metadata (name, version, exports, license)
//! graph/*.jsonld         # All graph files
//! CHECKSUM              # SHA-256 of each included file
//! ```
//!
//! Reproducibility is achieved by sorting entries alphabetically and using
//! a fixed mtime of 0 (Unix epoch) for all entries.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::manifest::{self, ModuleManifest};

/// Errors produced during module packaging.
#[derive(Debug, Error)]
pub enum PackageError {
    /// The manifest file is missing or invalid.
    #[error("Manifest error: {0}")]
    Manifest(#[from] manifest::ManifestError),

    /// A required manifest field is empty.
    #[error("Manifest field '{field}' is required but empty")]
    MissingField {
        /// The field that is missing.
        field: String,
    },

    /// The version string is not valid SemVer.
    #[error("Invalid version '{version}': {reason}")]
    InvalidVersion {
        /// The version string that failed to parse.
        version: String,
        /// Parse error details.
        reason: String,
    },

    /// I/O error reading workspace files.
    #[error("I/O error at '{path}': {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Tar/gzip archive creation failed.
    #[error("Failed to create archive: {0}")]
    Archive(String),

    /// No graph files found in the workspace.
    #[error("No .jsonld files found in {0}")]
    EmptyGraph(String),
}

/// Packs a duumbi module workspace into a `.tar.gz` archive in memory.
///
/// Reads `manifest.toml` and all `.jsonld` files from the workspace's
/// `.duumbi/graph/` directory. Validates required manifest fields (name, version).
/// Produces a reproducible archive: sorted entries, fixed timestamps.
///
/// # Errors
///
/// Returns `PackageError` if the manifest is missing/invalid, no graph files
/// exist, or an I/O error occurs.
#[must_use = "packaging errors should be handled"]
pub fn pack_module(workspace_path: &Path) -> Result<Vec<u8>, PackageError> {
    let duumbi_dir = workspace_path.join(".duumbi");
    let graph_dir = duumbi_dir.join("graph");
    let manifest_path = duumbi_dir.join("manifest.toml");

    // 1. Load and validate manifest
    let manifest = if manifest_path.exists() {
        manifest::parse_manifest(&manifest_path)?
    } else {
        // Try to build manifest from config.toml workspace section
        return Err(PackageError::Manifest(manifest::ManifestError::Io {
            path: manifest_path.display().to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "manifest.toml not found — run `duumbi init` or create manifest.toml",
            ),
        }));
    };

    validate_manifest(&manifest)?;

    // 2. Collect graph files (sorted for reproducibility)
    let graph_files = collect_graph_files(&graph_dir)?;
    if graph_files.is_empty() {
        return Err(PackageError::EmptyGraph(graph_dir.display().to_string()));
    }

    // 3. Read all file contents into a sorted map
    let manifest_bytes = manifest.to_toml().into_bytes();
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    files.insert("manifest.toml".to_string(), manifest_bytes);

    for (name, path) in &graph_files {
        let content = fs::read(path).map_err(|source| PackageError::Io {
            path: path.display().to_string(),
            source,
        })?;
        files.insert(format!("graph/{name}"), content);
    }

    // 4. Generate CHECKSUM file
    let checksum = generate_checksum(&files);
    files.insert("CHECKSUM".to_string(), checksum.into_bytes());

    // 5. Build tar.gz archive
    build_tarball(&files)
}

/// Validates that the manifest has all required fields for publishing.
fn validate_manifest(manifest: &ModuleManifest) -> Result<(), PackageError> {
    if manifest.module.name.is_empty() {
        return Err(PackageError::MissingField {
            field: "module.name".to_string(),
        });
    }
    if manifest.module.version.is_empty() {
        return Err(PackageError::MissingField {
            field: "module.version".to_string(),
        });
    }
    // Validate version is valid SemVer
    semver::Version::parse(&manifest.module.version).map_err(|e| PackageError::InvalidVersion {
        version: manifest.module.version.clone(),
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Collects all `.jsonld` files from a graph directory, sorted by name.
fn collect_graph_files(
    graph_dir: &Path,
) -> Result<Vec<(String, std::path::PathBuf)>, PackageError> {
    if !graph_dir.exists() {
        return Err(PackageError::Io {
            path: graph_dir.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "graph directory not found"),
        });
    }

    let mut files: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in fs::read_dir(graph_dir).map_err(|source| PackageError::Io {
        path: graph_dir.display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| PackageError::Io {
            path: graph_dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonld") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| PackageError::Io {
                    path: path.display().to_string(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "non-UTF-8 filename",
                    ),
                })?
                .to_string();
            files.push((name, path));
        }
    }

    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// Generates a CHECKSUM file with SHA-256 hashes for all included files.
///
/// Format: `sha256:<hex>  <filename>` (one line per file, sorted).
fn generate_checksum(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut checksum = String::new();
    for (name, content) in files {
        if name == "CHECKSUM" {
            continue;
        }
        let hash = Sha256::digest(content);
        let mut hex = String::with_capacity(64);
        for b in hash.as_slice() {
            write!(hex, "{b:02x}").expect("invariant: writing to String cannot fail");
        }
        writeln!(checksum, "sha256:{hex}  {name}")
            .expect("invariant: writing to String cannot fail");
    }
    checksum
}

/// Builds a `.tar.gz` archive from a sorted map of filename → content.
///
/// All entries use mtime=0 and mode=0o644 for reproducibility.
fn build_tarball(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, PackageError> {
    let mut buf = Vec::new();
    {
        // Pin compression level for cross-version reproducibility.
        let encoder = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::new(6));
        let mut builder = tar::Builder::new(encoder);

        for (name, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(0);
            header.set_cksum();
            builder
                .append_data(&mut header, name, content.as_slice())
                .map_err(|e| PackageError::Archive(e.to_string()))?;
        }

        builder
            .finish()
            .map_err(|e| PackageError::Archive(e.to_string()))?;
    }
    Ok(buf)
}

/// Unpacks a `.tar.gz` archive and returns the contained files as a map.
///
/// Useful for verifying package contents in tests.
#[cfg(test)]
fn unpack_to_map(data: &[u8]) -> BTreeMap<String, Vec<u8>> {
    use std::io::Read as _;

    let decoder = flate2::read::GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    let mut files = BTreeMap::new();

    for entry in archive.entries().expect("invariant: valid tar") {
        let mut entry = entry.expect("invariant: valid entry");
        let path = entry
            .path()
            .expect("invariant: valid path")
            .to_string_lossy()
            .to_string();
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .expect("invariant: readable entry");
        files.insert(path, content);
    }
    files
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Creates a minimal publishable workspace.
    fn make_publishable_workspace(dir: &Path, name: &str, version: &str) {
        let duumbi = dir.join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("invariant: must create dirs");

        // Write manifest
        let manifest = ModuleManifest::new(name, version, "Test module", vec!["test_fn".into()]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml()).expect("invariant: write");

        // Write graph file
        fs::write(
            graph.join("main.jsonld"),
            r#"{
                "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
                "@type": "duumbi:Module",
                "@id": "duumbi:main",
                "duumbi:name": "main",
                "duumbi:functions": []
            }"#,
        )
        .expect("invariant: write graph");
    }

    #[test]
    fn pack_module_produces_valid_tarball() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/example", "1.0.0");

        let tarball = pack_module(tmp.path()).expect("pack must succeed");
        assert!(!tarball.is_empty(), "tarball must not be empty");

        // Unpack and verify contents
        let files = unpack_to_map(&tarball);
        assert!(files.contains_key("manifest.toml"), "must contain manifest");
        assert!(
            files.contains_key("graph/main.jsonld"),
            "must contain graph file"
        );
        assert!(files.contains_key("CHECKSUM"), "must contain CHECKSUM");
    }

    #[test]
    fn pack_module_manifest_roundtrips() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/example", "2.3.4");

        let tarball = pack_module(tmp.path()).expect("pack must succeed");
        let files = unpack_to_map(&tarball);

        let manifest_bytes = &files["manifest.toml"];
        let manifest: ModuleManifest =
            toml::from_str(&String::from_utf8_lossy(manifest_bytes)).expect("must parse manifest");
        assert_eq!(manifest.module.name, "@test/example");
        assert_eq!(manifest.module.version, "2.3.4");
        assert_eq!(manifest.exports.functions, vec!["test_fn"]);
    }

    #[test]
    fn pack_module_checksum_contains_all_files() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/example", "1.0.0");

        let tarball = pack_module(tmp.path()).expect("pack must succeed");
        let files = unpack_to_map(&tarball);

        let checksum = String::from_utf8_lossy(&files["CHECKSUM"]);
        assert!(
            checksum.contains("manifest.toml"),
            "CHECKSUM must reference manifest"
        );
        assert!(
            checksum.contains("graph/main.jsonld"),
            "CHECKSUM must reference graph file"
        );
        // Verify format: sha256:<hex>  <filename>
        for line in checksum.lines() {
            assert!(
                line.starts_with("sha256:"),
                "each line must start with sha256:"
            );
            let parts: Vec<&str> = line.splitn(2, "  ").collect();
            assert_eq!(parts.len(), 2, "format: sha256:<hex>  <filename>");
            assert_eq!(parts[0].len(), 71, "sha256: (7) + 64 hex chars = 71");
        }
    }

    #[test]
    fn pack_module_is_reproducible() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/example", "1.0.0");

        let tarball1 = pack_module(tmp.path()).expect("pack 1 must succeed");
        let tarball2 = pack_module(tmp.path()).expect("pack 2 must succeed");

        assert_eq!(
            tarball1, tarball2,
            "same input must produce identical tarballs"
        );
    }

    #[test]
    fn pack_module_missing_manifest_returns_error() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let graph = tmp.path().join(".duumbi/graph");
        fs::create_dir_all(&graph).expect("create dirs");
        fs::write(graph.join("main.jsonld"), "{}").expect("write");

        let err = pack_module(tmp.path()).expect_err("must fail without manifest");
        assert!(
            matches!(err, PackageError::Manifest(_)),
            "expected Manifest error, got: {err}"
        );
    }

    #[test]
    fn pack_module_empty_name_returns_error() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("create dirs");
        fs::write(graph.join("main.jsonld"), "{}").expect("write");

        let manifest = ModuleManifest::new("", "1.0.0", "desc", vec![]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml()).expect("write");

        let err = pack_module(tmp.path()).expect_err("must fail with empty name");
        assert!(
            matches!(err, PackageError::MissingField { .. }),
            "expected MissingField, got: {err}"
        );
    }

    #[test]
    fn pack_module_invalid_version_returns_error() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("create dirs");
        fs::write(graph.join("main.jsonld"), "{}").expect("write");

        let manifest = ModuleManifest::new("@test/mod", "not-semver", "desc", vec![]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml()).expect("write");

        let err = pack_module(tmp.path()).expect_err("must fail with invalid version");
        assert!(
            matches!(err, PackageError::InvalidVersion { .. }),
            "expected InvalidVersion, got: {err}"
        );
    }

    #[test]
    fn pack_module_no_graph_files_returns_error() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("create dirs");

        let manifest = ModuleManifest::new("@test/empty", "1.0.0", "desc", vec![]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml()).expect("write");

        let err = pack_module(tmp.path()).expect_err("must fail with no graph files");
        assert!(
            matches!(err, PackageError::EmptyGraph(_)),
            "expected EmptyGraph, got: {err}"
        );
    }

    #[test]
    fn pack_module_multiple_graph_files() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("create dirs");

        let manifest =
            ModuleManifest::new("@test/multi", "1.0.0", "desc", vec!["a".into(), "b".into()]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml()).expect("write");
        fs::write(
            graph.join("module_a.jsonld"),
            r#"{"@type": "duumbi:Module"}"#,
        )
        .expect("write");
        fs::write(
            graph.join("module_b.jsonld"),
            r#"{"@type": "duumbi:Module"}"#,
        )
        .expect("write");
        fs::write(graph.join("README.md"), "ignored").expect("write");

        let tarball = pack_module(tmp.path()).expect("pack must succeed");
        let files = unpack_to_map(&tarball);

        assert!(files.contains_key("graph/module_a.jsonld"));
        assert!(files.contains_key("graph/module_b.jsonld"));
        assert!(
            !files.contains_key("graph/README.md"),
            "non-jsonld files must be excluded"
        );
        assert_eq!(files.len(), 4, "manifest + 2 graph + CHECKSUM = 4");
    }

    #[test]
    fn checksum_integrity_verifiable() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/verify", "1.0.0");

        let tarball = pack_module(tmp.path()).expect("pack must succeed");
        let files = unpack_to_map(&tarball);
        let checksum_str = String::from_utf8_lossy(&files["CHECKSUM"]);

        // Verify each checksum line against actual file content
        for line in checksum_str.lines() {
            let parts: Vec<&str> = line.splitn(2, "  ").collect();
            let expected_hash = parts[0];
            let filename = parts[1];

            let content = &files[filename];
            let actual = Sha256::digest(content);
            let mut hex = String::with_capacity(71);
            hex.push_str("sha256:");
            for b in actual.as_slice() {
                write!(hex, "{b:02x}").expect("invariant: writing to String cannot fail");
            }

            assert_eq!(expected_hash, hex, "CHECKSUM hash mismatch for {filename}");
        }
    }

    #[test]
    fn pack_module_empty_version_returns_error() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("create dirs");
        fs::write(graph.join("main.jsonld"), "{}").expect("write");

        let manifest = ModuleManifest::new("@test/mod", "", "desc", vec![]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml()).expect("write");

        let err = pack_module(tmp.path()).expect_err("must fail with empty version");
        assert!(
            matches!(err, PackageError::MissingField { .. }),
            "expected MissingField, got: {err}"
        );
    }

    #[test]
    fn pack_module_graph_content_preserved() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("create dirs");

        let manifest = ModuleManifest::new("@test/content", "1.0.0", "desc", vec![]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml()).expect("write");

        let original_content =
            r#"{"@type": "duumbi:Module", "duumbi:name": "test", "duumbi:value": 42}"#;
        fs::write(graph.join("main.jsonld"), original_content).expect("write");

        let tarball = pack_module(tmp.path()).expect("pack must succeed");
        let files = unpack_to_map(&tarball);

        let packed_content = String::from_utf8_lossy(&files["graph/main.jsonld"]);
        assert_eq!(
            packed_content, original_content,
            "graph file content must be preserved byte-for-byte"
        );
    }

    #[test]
    fn pack_module_no_graph_dir_returns_error() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("create dirs");

        let manifest = ModuleManifest::new("@test/no-graph", "1.0.0", "desc", vec![]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml()).expect("write");

        let err = pack_module(tmp.path()).expect_err("must fail without graph dir");
        assert!(
            matches!(err, PackageError::Io { .. }),
            "expected Io error, got: {err}"
        );
    }
}
