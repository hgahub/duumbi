//! Error types, diagnostic codes, and structured JSONL reporting.
//!
//! All duumbi errors use error codes E001–E012. The `Diagnostic` struct
//! serializes to JSONL for machine-readable output.

use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

use crate::types::NodeId;

/// Error code constants for structured diagnostics.
pub mod codes {
    /// Type mismatch (e.g. binary op operand types differ).
    pub const E001_TYPE_MISMATCH: &str = "E001";
    /// Unknown Op `@type`.
    pub const E002_UNKNOWN_OP: &str = "E002";
    /// Required field missing in JSON-LD node.
    pub const E003_MISSING_FIELD: &str = "E003";
    /// Reference to a non-existent `@id`.
    pub const E004_ORPHAN_REF: &str = "E004";
    /// Duplicate `@id` in the graph.
    pub const E005_DUPLICATE_ID: &str = "E005";
    /// No entry function (`main`) found.
    pub const E006_NO_ENTRY: &str = "E006";
    /// Cycle detected in the data-flow graph.
    pub const E007_CYCLE: &str = "E007";
    /// Linker invocation failed.
    pub const E008_LINK_FAILED: &str = "E008";
    /// Schema validation failed (malformed JSON-LD structure).
    pub const E009_SCHEMA_INVALID: &str = "E009";
    /// Unresolved cross-module function reference.
    pub const E010_UNRESOLVED_CROSS_MODULE: &str = "E010";
    /// Dependency module not found in any resolution layer (workspace, vendor, cache, registry).
    #[allow(dead_code)] // Used by deps resolution pipeline (Phase 5)
    pub const E011_DEPENDENCY_NOT_FOUND: &str = "E011";
    /// Module name conflict: same-scope modules export the same function and resolution is ambiguous.
    #[allow(dead_code)] // Used by deps resolution pipeline (Phase 5)
    pub const E012_MODULE_CONFLICT: &str = "E012";
}

/// Severity level for a diagnostic message.
#[allow(dead_code)] // Warning variant planned for future phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    /// Unrecoverable error — compilation cannot proceed.
    Error,
    /// Warning — compilation continues but output may be unexpected.
    Warning,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Error => f.write_str("error"),
            DiagnosticLevel::Warning => f.write_str("warning"),
        }
    }
}

/// Structured diagnostic message serializable to JSONL.
///
/// Emitted to stdout for machine consumption. Human-readable summaries
/// go to stderr.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Severity level.
    pub level: DiagnosticLevel,
    /// Error code (e.g. `"E001"`).
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// The `@id` of the node where the error occurred, if known.
    #[serde(rename = "nodeId", skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// Source file path, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Additional structured details about the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, String>>,
}

impl Diagnostic {
    /// Creates an error diagnostic with the given code and message.
    #[must_use]
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            code: code.to_string(),
            message: message.into(),
            node_id: None,
            file: None,
            details: None,
        }
    }

    /// Attaches a node ID to this diagnostic.
    #[must_use]
    pub fn with_node(mut self, node_id: &NodeId) -> Self {
        self.node_id = Some(node_id.0.clone());
        self
    }

    /// Attaches a source file path to this diagnostic.
    #[allow(dead_code)] // Used in future phases for file-level diagnostics
    #[must_use]
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Attaches additional details to this diagnostic.
    #[must_use]
    pub fn with_details(mut self, details: HashMap<String, String>) -> Self {
        self.details = Some(details);
        self
    }

    /// Serializes this diagnostic as a JSON line.
    ///
    /// Returns the JSON string without trailing newline.
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            format!(
                r#"{{"level":"error","code":"INTERNAL","message":"Failed to serialize diagnostic: {e}"}}"#
            )
        })
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.code, self.level, self.message)?;
        if let Some(ref nid) = self.node_id {
            write!(f, " (at {nid})")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_jsonl_serialization() {
        let diag = Diagnostic::error(
            codes::E001_TYPE_MISMATCH,
            "Type mismatch: Add expects matching operand types",
        )
        .with_node(&NodeId("duumbi:main/main/entry/2".to_string()))
        .with_file("graph/main.jsonld");

        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");

        assert_eq!(parsed["level"], "error");
        assert_eq!(parsed["code"], "E001");
        assert_eq!(parsed["nodeId"], "duumbi:main/main/entry/2");
        assert_eq!(parsed["file"], "graph/main.jsonld");
    }

    #[test]
    fn diagnostic_without_optional_fields_omits_them() {
        let diag = Diagnostic::error(codes::E002_UNKNOWN_OP, "Unknown op");
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");

        assert!(parsed.get("nodeId").is_none());
        assert!(parsed.get("file").is_none());
        assert!(parsed.get("details").is_none());
    }

    #[test]
    fn error_codes_are_unique() {
        let codes = [
            codes::E001_TYPE_MISMATCH,
            codes::E002_UNKNOWN_OP,
            codes::E003_MISSING_FIELD,
            codes::E004_ORPHAN_REF,
            codes::E005_DUPLICATE_ID,
            codes::E006_NO_ENTRY,
            codes::E007_CYCLE,
            codes::E008_LINK_FAILED,
            codes::E009_SCHEMA_INVALID,
            codes::E010_UNRESOLVED_CROSS_MODULE,
            codes::E011_DEPENDENCY_NOT_FOUND,
            codes::E012_MODULE_CONFLICT,
        ];
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(codes.len(), unique.len(), "Error codes must be unique");
    }

    #[test]
    fn e011_dependency_not_found_serializes_correctly() {
        let diag = Diagnostic::error(
            codes::E011_DEPENDENCY_NOT_FOUND,
            "@community/sorting not found",
        );
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E011");
    }

    #[test]
    fn e012_module_conflict_serializes_correctly() {
        let diag = Diagnostic::error(
            codes::E012_MODULE_CONFLICT,
            "Module conflict: 'sort' exported by both @duumbi/sorting and @community/sorting",
        );
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E012");
    }

    #[test]
    fn diagnostic_with_details() {
        let mut details = HashMap::new();
        details.insert("expected".to_string(), "i64".to_string());
        details.insert("found".to_string(), "f64".to_string());

        let diag =
            Diagnostic::error(codes::E001_TYPE_MISMATCH, "Type mismatch").with_details(details);
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");

        assert_eq!(parsed["details"]["expected"], "i64");
        assert_eq!(parsed["details"]["found"], "f64");
    }
}
