//! Semantic hashing for duumbi graph modules.
//!
//! Computes a SHA-256 hash of a module's graph structure and operation values,
//! **excluding `@id` values**. Two modules with identical logic but different
//! node identifiers produce the same semantic hash.
//!
//! # Algorithm
//!
//! 1. Read all `.jsonld` files in the graph directory (sorted by name).
//! 2. Canonicalize each JSON value:
//!    - Remove `@id` keys from node objects (identity, not semantics).
//!    - Remove `@context` from the top level (namespace declaration).
//!    - Normalize reference objects `{"@id": "duumbi:X/Y/Z/N"}` to `{"_ref": N}`
//!      (positional index) or `{"_ref": "path/component"}` (named reference).
//!    - Sort all object keys alphabetically.
//! 3. Serialize each canonical value as compact JSON.
//! 4. Concatenate all canonical JSON strings (newline-separated, sorted by filename).
//! 5. Compute SHA-256 of the concatenated bytes.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during semantic hash computation.
#[derive(Debug, Error)]
pub enum HashError {
    /// Failed to read a file or directory.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// The path that caused the error.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// Failed to parse a `.jsonld` file as JSON.
    #[error("JSON parse error in {path}: {source}")]
    JsonParse {
        /// The path that caused the error.
        path: String,
        /// The underlying parse error.
        source: serde_json::Error,
    },
}

/// Computes the semantic hash of all `.jsonld` files in a graph directory.
///
/// The hash is a hex-encoded SHA-256 string. It is deterministic across
/// runs and machines for the same logical graph content, regardless of
/// `@id` assignment.
///
/// # Errors
///
/// Returns `HashError` if directory listing, file reading, or JSON parsing fails.
#[must_use = "the computed hash should be used"]
pub fn semantic_hash(graph_dir: &Path) -> Result<String, HashError> {
    let mut paths = collect_jsonld_files(graph_dir)?;
    paths.sort();

    let mut hasher = Sha256::new();

    for (i, path) in paths.iter().enumerate() {
        let content = fs::read_to_string(path).map_err(|e| HashError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let value: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| HashError::JsonParse {
                path: path.display().to_string(),
                source: e,
            })?;

        let canonical = canonicalize(&value, true);
        let json_bytes = canonical_json(&canonical);

        if i > 0 {
            hasher.update(b"\n");
        }
        hasher.update(json_bytes.as_bytes());
    }

    let result = hasher.finalize();
    Ok(hex_encode(result))
}

/// Computes the semantic hash of a single JSON-LD module value (in-memory).
///
/// The input should be a single module object (not an array of modules).
#[must_use]
pub fn semantic_hash_value(value: &serde_json::Value) -> String {
    let canonical = canonicalize(value, true);
    let json_bytes = canonical_json(&canonical);
    let result = Sha256::digest(json_bytes.as_bytes());
    hex_encode(result)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Collects all `.jsonld` file paths in a directory.
fn collect_jsonld_files(dir: &Path) -> Result<Vec<PathBuf>, HashError> {
    let entries = fs::read_dir(dir).map_err(|e| HashError::Io {
        path: dir.display().to_string(),
        source: e,
    })?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| HashError::Io {
            path: dir.display().to_string(),
            source: e,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonld") {
            paths.push(path);
        }
    }
    Ok(paths)
}

/// Canonicalizes a JSON-LD value for semantic hashing.
///
/// - Removes `@id` keys from objects (node identity).
/// - Removes `@context` from top-level objects.
/// - Normalizes reference objects `{"@id": "..."}` to `{"_ref": <index>}`.
/// - Inserts keys into `BTreeMap` for deterministic sorted output.
fn canonicalize(value: &serde_json::Value, is_top_level: bool) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            // Detect reference object: {"@id": "duumbi:..."} with only one key.
            // These are operand references, not node identities.
            if map.len() == 1
                && let Some(id_val) = map.get("@id")
                && let Some(id_str) = id_val.as_str()
            {
                return normalize_reference(id_str);
            }

            // Use BTreeMap directly for sorted keys — avoids a second pass.
            let mut canonical = BTreeMap::new();
            for (key, val) in map {
                // Skip @id (node identity — not semantically meaningful).
                if key == "@id" {
                    continue;
                }
                // Skip @context at top level (namespace declaration).
                if key == "@context" && is_top_level {
                    continue;
                }
                canonical.insert(key.clone(), canonicalize(val, false));
            }
            serde_json::to_value(canonical)
                .expect("invariant: BTreeMap<String, Value> must serialize")
        }
        serde_json::Value::Array(arr) => {
            let canonical: Vec<serde_json::Value> =
                arr.iter().map(|v| canonicalize(v, false)).collect();
            serde_json::Value::Array(canonical)
        }
        // Primitives pass through unchanged.
        other => other.clone(),
    }
}

/// Normalizes an `@id` reference for semantic hashing.
///
/// For numeric trailing components (op indices within a block), produces
/// `{"_ref": N}`. For named references (function/block names), preserves
/// the path after the `duumbi:` prefix to distinguish cross-module references.
///
/// Examples:
/// - `duumbi:main/main/entry/0` → `{"_ref": 0}` (positional op index)
/// - `duumbi:math/abs` → `{"_ref": "math/abs"}` (function reference)
fn normalize_reference(id: &str) -> serde_json::Value {
    // Strip the "duumbi:" prefix if present.
    let path = id.strip_prefix("duumbi:").unwrap_or(id);

    // Extract the last path component.
    let last = path.rsplit('/').next().unwrap_or(path);

    // Try to parse as integer (positional op index within a block).
    if let Ok(idx) = last.parse::<u64>() {
        serde_json::json!({"_ref": idx})
    } else {
        // Named reference — keep the full path to distinguish
        // cross-module references (e.g., "math/abs" vs "main/abs").
        serde_json::json!({"_ref": path})
    }
}

/// Serializes a JSON value as compact canonical JSON.
///
/// Keys are already sorted by `canonicalize()` which uses `BTreeMap`.
fn canonical_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).expect("invariant: canonical value must serialize")
}

/// Encodes bytes as a lowercase hexadecimal string.
fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").expect("invariant: writing to String cannot fail");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper: write a .jsonld file with given content.
    fn write_jsonld(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).expect("invariant: test file write must succeed");
    }

    #[test]
    fn same_structure_different_ids_same_hash() {
        let dir_a = TempDir::new().expect("invariant: temp dir creation must succeed");
        let dir_b = TempDir::new().expect("invariant: temp dir creation must succeed");

        // Module A: uses "duumbi:modA/..." identifiers.
        write_jsonld(
            dir_a.path(),
            "main.jsonld",
            r#"{
                "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
                "@type": "duumbi:Module",
                "@id": "duumbi:modA",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function",
                    "@id": "duumbi:modA/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "i64",
                    "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:modA/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:modA/main/entry/0",
                             "duumbi:value": 42, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Return", "@id": "duumbi:modA/main/entry/1",
                             "duumbi:operand": {"@id": "duumbi:modA/main/entry/0"}}
                        ]
                    }]
                }]
            }"#,
        );

        // Module B: identical logic but uses "duumbi:modB/..." identifiers.
        write_jsonld(
            dir_b.path(),
            "main.jsonld",
            r#"{
                "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
                "@type": "duumbi:Module",
                "@id": "duumbi:modB",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function",
                    "@id": "duumbi:modB/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "i64",
                    "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:modB/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:modB/main/entry/0",
                             "duumbi:value": 42, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Return", "@id": "duumbi:modB/main/entry/1",
                             "duumbi:operand": {"@id": "duumbi:modB/main/entry/0"}}
                        ]
                    }]
                }]
            }"#,
        );

        let hash_a = semantic_hash(dir_a.path()).expect("hash A should succeed");
        let hash_b = semantic_hash(dir_b.path()).expect("hash B should succeed");

        assert_eq!(
            hash_a, hash_b,
            "identical logic with different @ids must produce the same hash"
        );
    }

    #[test]
    fn different_op_values_different_hash() {
        let dir_a = TempDir::new().expect("invariant: temp dir creation must succeed");
        let dir_b = TempDir::new().expect("invariant: temp dir creation must succeed");

        // Module A: Const(42)
        write_jsonld(
            dir_a.path(),
            "main.jsonld",
            r#"{
                "@type": "duumbi:Module",
                "@id": "duumbi:main",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "i64",
                    "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:main/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
                             "duumbi:value": 42, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1",
                             "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}}
                        ]
                    }]
                }]
            }"#,
        );

        // Module B: Const(99) — different value.
        write_jsonld(
            dir_b.path(),
            "main.jsonld",
            r#"{
                "@type": "duumbi:Module",
                "@id": "duumbi:main",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "i64",
                    "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:main/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
                             "duumbi:value": 99, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1",
                             "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}}
                        ]
                    }]
                }]
            }"#,
        );

        let hash_a = semantic_hash(dir_a.path()).expect("hash A should succeed");
        let hash_b = semantic_hash(dir_b.path()).expect("hash B should succeed");

        assert_ne!(
            hash_a, hash_b,
            "different op values must produce different hashes"
        );
    }

    #[test]
    fn different_structure_different_hash() {
        let dir_a = TempDir::new().expect("invariant: temp dir creation must succeed");
        let dir_b = TempDir::new().expect("invariant: temp dir creation must succeed");

        // Module A: Const + Return
        write_jsonld(
            dir_a.path(),
            "main.jsonld",
            r#"{
                "@type": "duumbi:Module", "@id": "duumbi:m",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function", "@id": "duumbi:m/main",
                    "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block", "@id": "duumbi:m/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:m/main/entry/0",
                             "duumbi:value": 1, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Return", "@id": "duumbi:m/main/entry/1",
                             "duumbi:operand": {"@id": "duumbi:m/main/entry/0"}}
                        ]
                    }]
                }]
            }"#,
        );

        // Module B: Const + Print + Return (extra op — different structure).
        write_jsonld(
            dir_b.path(),
            "main.jsonld",
            r#"{
                "@type": "duumbi:Module", "@id": "duumbi:m",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function", "@id": "duumbi:m/main",
                    "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block", "@id": "duumbi:m/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:m/main/entry/0",
                             "duumbi:value": 1, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Print", "@id": "duumbi:m/main/entry/1",
                             "duumbi:operand": {"@id": "duumbi:m/main/entry/0"},
                             "duumbi:resultType": "void"},
                            {"@type": "duumbi:Return", "@id": "duumbi:m/main/entry/2",
                             "duumbi:operand": {"@id": "duumbi:m/main/entry/0"}}
                        ]
                    }]
                }]
            }"#,
        );

        let hash_a = semantic_hash(dir_a.path()).expect("hash A should succeed");
        let hash_b = semantic_hash(dir_b.path()).expect("hash B should succeed");

        assert_ne!(
            hash_a, hash_b,
            "different structure must produce different hashes"
        );
    }

    #[test]
    fn deterministic_across_runs() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_jsonld(
            dir.path(),
            "main.jsonld",
            r#"{
                "@type": "duumbi:Module", "@id": "duumbi:main",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function", "@id": "duumbi:main/main",
                    "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block", "@id": "duumbi:main/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
                             "duumbi:value": 7, "duumbi:resultType": "i64"},
                            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1",
                             "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}}
                        ]
                    }]
                }]
            }"#,
        );

        let hash1 = semantic_hash(dir.path()).expect("hash 1 should succeed");
        let hash2 = semantic_hash(dir.path()).expect("hash 2 should succeed");
        let hash3 = semantic_hash(dir.path()).expect("hash 3 should succeed");

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn empty_directory_produces_consistent_hash() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let hash = semantic_hash(dir.path()).expect("empty dir hash should succeed");
        // Empty input → SHA-256 of empty string.
        assert_eq!(hash.len(), 64, "SHA-256 hex digest is always 64 chars");
    }

    #[test]
    fn hash_value_matches_hash_file() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let json_str = r#"{
            "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
            "@type": "duumbi:Module", "@id": "duumbi:main",
            "duumbi:name": "test",
            "duumbi:functions": []
        }"#;
        write_jsonld(dir.path(), "main.jsonld", json_str);

        let hash_dir = semantic_hash(dir.path()).expect("dir hash should succeed");
        let value: serde_json::Value =
            serde_json::from_str(json_str).expect("invariant: test JSON must parse");
        let hash_val = semantic_hash_value(&value);

        assert_eq!(
            hash_dir, hash_val,
            "file-based and value-based hash must match"
        );
    }

    #[test]
    fn multi_file_order_independent_of_content_but_sorted_by_name() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");

        write_jsonld(
            dir.path(),
            "a_math.jsonld",
            r#"{"@type": "duumbi:Module", "@id": "duumbi:math", "duumbi:name": "math", "duumbi:functions": []}"#,
        );
        write_jsonld(
            dir.path(),
            "b_io.jsonld",
            r#"{"@type": "duumbi:Module", "@id": "duumbi:io", "duumbi:name": "io", "duumbi:functions": []}"#,
        );

        let hash1 = semantic_hash(dir.path()).expect("hash should succeed");
        let hash2 = semantic_hash(dir.path()).expect("hash should succeed");

        assert_eq!(hash1, hash2, "multi-file hash must be deterministic");
    }

    #[test]
    fn reference_normalization_numeric() {
        let result = normalize_reference("duumbi:main/main/entry/0");
        assert_eq!(result, serde_json::json!({"_ref": 0}));

        let result = normalize_reference("duumbi:math/abs/abs_neg/3");
        assert_eq!(result, serde_json::json!({"_ref": 3}));
    }

    #[test]
    fn reference_normalization_named_preserves_path() {
        // Function reference preserves full path (module/function).
        let result = normalize_reference("duumbi:math/abs");
        assert_eq!(result, serde_json::json!({"_ref": "math/abs"}));

        // Different module with same function name → different reference.
        let result = normalize_reference("duumbi:main/abs");
        assert_eq!(result, serde_json::json!({"_ref": "main/abs"}));
    }

    #[test]
    fn canonicalize_strips_id_and_context() {
        let input: serde_json::Value = serde_json::from_str(
            r#"{
                "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
                "@type": "duumbi:Module",
                "@id": "duumbi:main",
                "duumbi:name": "test"
            }"#,
        )
        .expect("invariant: test JSON must parse");

        let canonical = canonicalize(&input, true);
        let obj = canonical.as_object().expect("invariant: must be object");

        assert!(!obj.contains_key("@id"), "@id must be removed");
        assert!(
            !obj.contains_key("@context"),
            "@context must be removed at top level"
        );
        assert!(obj.contains_key("@type"), "@type must be preserved");
        assert!(
            obj.contains_key("duumbi:name"),
            "duumbi:name must be preserved"
        );
    }

    #[test]
    fn canonicalize_preserves_context_at_non_top_level() {
        let input: serde_json::Value = serde_json::from_str(
            r#"{
                "nested": {
                    "@context": {"x": "y"},
                    "@type": "test"
                }
            }"#,
        )
        .expect("invariant: test JSON must parse");

        let canonical = canonicalize(&input, true);
        let nested = canonical
            .get("nested")
            .expect("invariant: nested must exist");
        let obj = nested.as_object().expect("invariant: must be object");

        assert!(
            obj.contains_key("@context"),
            "@context at non-top-level must be preserved"
        );
    }

    #[test]
    fn stdlib_math_produces_valid_hash() {
        let stdlib_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib");
        assert!(
            stdlib_path.exists(),
            "stdlib directory must exist for this test"
        );

        let hash = semantic_hash(&stdlib_path).expect("stdlib hash should succeed");
        assert_eq!(hash.len(), 64, "SHA-256 hex digest is always 64 chars");

        let empty_hash = {
            let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
            semantic_hash(dir.path()).expect("empty hash should succeed")
        };
        assert_ne!(hash, empty_hash, "stdlib hash must differ from empty");
    }

    #[test]
    fn nonexistent_directory_returns_io_error() {
        let result = semantic_hash(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, HashError::Io { .. }),
            "expected HashError::Io, got: {err}"
        );
    }

    #[test]
    fn malformed_json_returns_parse_error() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_jsonld(dir.path(), "bad.jsonld", "{ this is not valid json }");

        let result = semantic_hash(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, HashError::JsonParse { .. }),
            "expected HashError::JsonParse, got: {err}"
        );
    }

    #[test]
    fn cross_module_references_distinguish_modules() {
        let dir_a = TempDir::new().expect("invariant: temp dir creation must succeed");
        let dir_b = TempDir::new().expect("invariant: temp dir creation must succeed");

        // Module A: calls math/abs
        write_jsonld(
            dir_a.path(),
            "main.jsonld",
            r#"{
                "@type": "duumbi:Module", "@id": "duumbi:main",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function", "@id": "duumbi:main/main",
                    "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block", "@id": "duumbi:main/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/0",
                             "duumbi:function": "abs", "duumbi:resultType": "i64",
                             "duumbi:operand": {"@id": "duumbi:math/abs"}}
                        ]
                    }]
                }]
            }"#,
        );

        // Module B: calls utils/abs (different module, same function name)
        write_jsonld(
            dir_b.path(),
            "main.jsonld",
            r#"{
                "@type": "duumbi:Module", "@id": "duumbi:main",
                "duumbi:name": "main",
                "duumbi:functions": [{
                    "@type": "duumbi:Function", "@id": "duumbi:main/main",
                    "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:params": [],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block", "@id": "duumbi:main/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/0",
                             "duumbi:function": "abs", "duumbi:resultType": "i64",
                             "duumbi:operand": {"@id": "duumbi:utils/abs"}}
                        ]
                    }]
                }]
            }"#,
        );

        let hash_a = semantic_hash(dir_a.path()).expect("hash A should succeed");
        let hash_b = semantic_hash(dir_b.path()).expect("hash B should succeed");

        assert_ne!(
            hash_a, hash_b,
            "references to different modules must produce different hashes"
        );
    }
}
