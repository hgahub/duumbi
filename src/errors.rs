//! Error types, diagnostic codes, and structured JSONL reporting.
//!
//! All duumbi errors use error codes E001–E035 and E040–E048. The `Diagnostic`
//! struct serializes to JSONL for machine-readable output.
//!
//! E001–E016: core compiler and registry errors.
//! E020–E029: ownership and lifetime errors.
//! E030–E035: error-handling (Result/Option) errors.
//! E040–E048: agent execution, graph-merge, MCP integration errors.

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
    /// Registry server is unreachable (network error, DNS failure, timeout).
    #[allow(dead_code)] // Used by registry client (Phase 7)
    pub const E013_REGISTRY_UNREACHABLE: &str = "E013";
    /// Authentication failed (invalid, expired, or missing token).
    #[allow(dead_code)] // Used by registry client (Phase 7)
    pub const E014_AUTH_FAILED: &str = "E014";
    /// Integrity mismatch: downloaded content SHA-256 does not match lockfile.
    #[allow(dead_code)] // Used by registry client (Phase 7)
    pub const E015_INTEGRITY_MISMATCH: &str = "E015";
    /// Requested version not found in the registry.
    #[allow(dead_code)] // Used by registry client (Phase 7)
    pub const E016_VERSION_NOT_FOUND: &str = "E016";
    /// Ownership violation: a value can only have one owner at a time.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E020_SINGLE_OWNER: &str = "E020";
    /// Use after move: value was moved and cannot be accessed.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E021_USE_AFTER_MOVE: &str = "E021";
    /// Borrow exclusivity: cannot have shared and mutable borrows simultaneously.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E022_BORROW_EXCLUSIVITY: &str = "E022";
    /// Lifetime exceeded: borrow outlives the owner's scope.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E023_LIFETIME_EXCEEDED: &str = "E023";
    /// Drop incomplete: not all code paths drop the value.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E024_DROP_INCOMPLETE: &str = "E024";
    /// Double free: value dropped more than once.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E025_DOUBLE_FREE: &str = "E025";
    /// Dangling reference: borrow used after the value was dropped.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E026_DANGLING_REFERENCE: &str = "E026";
    /// Move while borrowed: cannot move a value that has active borrows.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E027_MOVE_WHILE_BORROWED: &str = "E027";
    /// Missing lifetime parameter on function that borrows.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E028_LIFETIME_PARAM_MISSING: &str = "E028";
    /// Return lifetime mismatch: returned borrow doesn't tie to input lifetime.
    #[allow(dead_code)] // Used by ownership validator (Phase 9a-2)
    pub const E029_RETURN_LIFETIME_MISMATCH: &str = "E029";
    /// Unhandled Result: Call returns Result but no Match/ResultIsOk follows in the block.
    #[allow(dead_code)] // Used by error handling validator (Phase 9a-3)
    pub const E030_UNHANDLED_RESULT: &str = "E030";
    /// Unhandled Option: Call returns Option but no Match/OptionIsSome follows in the block.
    #[allow(dead_code)] // Used by error handling validator (Phase 9a-3)
    pub const E031_UNHANDLED_OPTION: &str = "E031";
    /// Non-exhaustive match: Match op is missing ok_block or err_block.
    #[allow(dead_code)] // Used by error handling validator (Phase 9a-3)
    pub const E032_NON_EXHAUSTIVE_MATCH: &str = "E032";
    /// Result/Option type param mismatch: ResultOk wraps wrong type for declared Result<T,E>.
    #[allow(dead_code)] // Used by error handling validator (Phase 9a-3)
    pub const E033_RESULT_TYPE_PARAM_MISMATCH: &str = "E033";
    /// Unwrap without check: ResultUnwrap/OptionUnwrap used without preceding check in the block.
    #[allow(dead_code)] // Used by error handling validator (Phase 9a-3) — warning level
    pub const E034_UNWRAP_WITHOUT_CHECK: &str = "E034";
    /// Result construction with wrong payload: ResultOk/ResultErr payload doesn't match T or E.
    #[allow(dead_code)] // Used by error handling validator (Phase 9a-3)
    pub const E035_RESULT_PAYLOAD_TYPE_MISMATCH: &str = "E035";
    /// Agent token budget exceeded.
    #[allow(dead_code)] // Used by agent execution pipeline (Phase 12)
    pub const E040_BUDGET_EXCEEDED: &str = "E040";
    /// Circuit breaker open — too many consecutive agent failures.
    #[allow(dead_code)] // Used by agent execution pipeline (Phase 12)
    pub const E041_CIRCUIT_OPEN: &str = "E041";
    /// Concurrent graph patches have irreconcilable conflicts.
    #[allow(dead_code)] // Used by patch merge pipeline (Phase 12)
    pub const E042_MERGE_CONFLICT: &str = "E042";
    /// Two patches create nodes with the same `@id`.
    #[allow(dead_code)] // Used by patch merge pipeline (Phase 12)
    pub const E043_NODE_ID_COLLISION: &str = "E043";
    /// Agent execution or spawn queue wait timed out.
    #[allow(dead_code)] // Used by agent execution pipeline (Phase 12)
    pub const E044_AGENT_TIMEOUT: &str = "E044";
    /// Referenced agent template not found in store.
    #[allow(dead_code)] // Used by agent template resolver (Phase 12)
    pub const E045_TEMPLATE_NOT_FOUND: &str = "E045";
    /// MCP tool invocation failed.
    #[allow(dead_code)] // Used by MCP integration layer (Phase 12)
    pub const E046_MCP_TOOL_ERROR: &str = "E046";
    /// External MCP server connection failed.
    #[allow(dead_code)] // Used by MCP integration layer (Phase 12)
    pub const E047_MCP_CLIENT_UNREACHABLE: &str = "E047";
    /// Requested tool not available on external MCP server.
    #[allow(dead_code)] // Used by MCP integration layer (Phase 12)
    pub const E048_MCP_CLIENT_TOOL_NOT_FOUND: &str = "E048";
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
            codes::E013_REGISTRY_UNREACHABLE,
            codes::E014_AUTH_FAILED,
            codes::E015_INTEGRITY_MISMATCH,
            codes::E016_VERSION_NOT_FOUND,
            codes::E020_SINGLE_OWNER,
            codes::E021_USE_AFTER_MOVE,
            codes::E022_BORROW_EXCLUSIVITY,
            codes::E023_LIFETIME_EXCEEDED,
            codes::E024_DROP_INCOMPLETE,
            codes::E025_DOUBLE_FREE,
            codes::E026_DANGLING_REFERENCE,
            codes::E027_MOVE_WHILE_BORROWED,
            codes::E028_LIFETIME_PARAM_MISSING,
            codes::E029_RETURN_LIFETIME_MISMATCH,
            codes::E030_UNHANDLED_RESULT,
            codes::E031_UNHANDLED_OPTION,
            codes::E032_NON_EXHAUSTIVE_MATCH,
            codes::E033_RESULT_TYPE_PARAM_MISMATCH,
            codes::E034_UNWRAP_WITHOUT_CHECK,
            codes::E035_RESULT_PAYLOAD_TYPE_MISMATCH,
            codes::E040_BUDGET_EXCEEDED,
            codes::E041_CIRCUIT_OPEN,
            codes::E042_MERGE_CONFLICT,
            codes::E043_NODE_ID_COLLISION,
            codes::E044_AGENT_TIMEOUT,
            codes::E045_TEMPLATE_NOT_FOUND,
            codes::E046_MCP_TOOL_ERROR,
            codes::E047_MCP_CLIENT_UNREACHABLE,
            codes::E048_MCP_CLIENT_TOOL_NOT_FOUND,
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
    fn e013_registry_unreachable_with_url() {
        let mut details = HashMap::new();
        details.insert(
            "registry_url".to_string(),
            "https://registry.duumbi.dev".to_string(),
        );
        let diag = Diagnostic::error(
            codes::E013_REGISTRY_UNREACHABLE,
            "Cannot connect to registry: connection timed out",
        )
        .with_details(details);
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E013");
        assert_eq!(
            parsed["details"]["registry_url"],
            "https://registry.duumbi.dev"
        );
    }

    #[test]
    fn e014_auth_failed_with_url() {
        let mut details = HashMap::new();
        details.insert(
            "registry_url".to_string(),
            "https://registry.duumbi.dev".to_string(),
        );
        let diag = Diagnostic::error(
            codes::E014_AUTH_FAILED,
            "Authentication failed: token expired",
        )
        .with_details(details);
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E014");
        assert_eq!(
            parsed["details"]["registry_url"],
            "https://registry.duumbi.dev"
        );
    }

    #[test]
    fn e015_integrity_mismatch_with_details() {
        let mut details = HashMap::new();
        details.insert("module".to_string(), "@duumbi/stdlib-math".to_string());
        details.insert("expected".to_string(), "sha256:abc123".to_string());
        details.insert("found".to_string(), "sha256:def456".to_string());
        let diag = Diagnostic::error(
            codes::E015_INTEGRITY_MISMATCH,
            "Integrity check failed: SHA-256 hash does not match lockfile",
        )
        .with_details(details);
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E015");
        assert_eq!(parsed["details"]["module"], "@duumbi/stdlib-math");
        assert_eq!(parsed["details"]["expected"], "sha256:abc123");
    }

    #[test]
    fn e016_version_not_found_with_details() {
        let mut details = HashMap::new();
        details.insert("module".to_string(), "@community/sorting".to_string());
        details.insert("version".to_string(), "^2.0.0".to_string());
        details.insert(
            "registry_url".to_string(),
            "https://registry.duumbi.dev".to_string(),
        );
        let diag = Diagnostic::error(
            codes::E016_VERSION_NOT_FOUND,
            "Version ^2.0.0 of @community/sorting not found in registry",
        )
        .with_details(details);
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E016");
        assert_eq!(parsed["details"]["module"], "@community/sorting");
        assert_eq!(parsed["details"]["version"], "^2.0.0");
    }

    #[test]
    fn diagnostic_display_format() {
        let diag = Diagnostic::error(codes::E013_REGISTRY_UNREACHABLE, "Cannot connect")
            .with_node(&NodeId("duumbi:main/main/entry/0".to_string()));

        let display = diag.to_string();
        assert!(display.contains("[E013]"));
        assert!(display.contains("error"));
        assert!(display.contains("Cannot connect"));
        assert!(display.contains("duumbi:main/main/entry/0"));
    }

    #[test]
    fn diagnostic_display_without_node() {
        let diag = Diagnostic::error(codes::E016_VERSION_NOT_FOUND, "Version not found");
        let display = diag.to_string();
        assert!(display.contains("[E016]"));
        assert!(!display.contains("(at"));
    }

    #[test]
    fn all_e013_e016_codes_serialize_with_details() {
        let test_cases = [
            (codes::E013_REGISTRY_UNREACHABLE, "E013"),
            (codes::E014_AUTH_FAILED, "E014"),
            (codes::E015_INTEGRITY_MISMATCH, "E015"),
            (codes::E016_VERSION_NOT_FOUND, "E016"),
        ];

        for (code, expected) in test_cases {
            let mut details = HashMap::new();
            details.insert("key".to_string(), "value".to_string());
            let diag = Diagnostic::error(code, format!("Test message for {expected}"))
                .with_node(&NodeId("duumbi:test/node".to_string()))
                .with_file("test.jsonld")
                .with_details(details);

            let json = diag.to_jsonl();
            let parsed: serde_json::Value = serde_json::from_str(&json)
                .expect("invariant: diagnostic must serialize to valid JSON");

            assert_eq!(parsed["code"], expected);
            assert_eq!(parsed["level"], "error");
            assert!(parsed["message"].as_str().is_some());
            assert_eq!(parsed["nodeId"], "duumbi:test/node");
            assert_eq!(parsed["file"], "test.jsonld");
            assert_eq!(parsed["details"]["key"], "value");
        }
    }

    #[test]
    fn diagnostic_level_display() {
        assert_eq!(DiagnosticLevel::Error.to_string(), "error");
        assert_eq!(DiagnosticLevel::Warning.to_string(), "warning");
    }

    #[test]
    fn diagnostic_warning_level() {
        let diag = Diagnostic {
            level: DiagnosticLevel::Warning,
            code: codes::E015_INTEGRITY_MISMATCH.to_string(),
            message: "Hash mismatch (non-fatal)".to_string(),
            node_id: None,
            file: None,
            details: None,
        };
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["level"], "warning");
        assert!(diag.to_string().contains("warning"));
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

    /// Verify that every E040-E048 code serializes with the correct `"code"` value
    /// and that the numeric portion is properly formatted (four-digit zero-padded).
    #[test]
    fn e040_e048_codes_serialize_correctly() {
        let test_cases = [
            (codes::E040_BUDGET_EXCEEDED, "E040"),
            (codes::E041_CIRCUIT_OPEN, "E041"),
            (codes::E042_MERGE_CONFLICT, "E042"),
            (codes::E043_NODE_ID_COLLISION, "E043"),
            (codes::E044_AGENT_TIMEOUT, "E044"),
            (codes::E045_TEMPLATE_NOT_FOUND, "E045"),
            (codes::E046_MCP_TOOL_ERROR, "E046"),
            (codes::E047_MCP_CLIENT_UNREACHABLE, "E047"),
            (codes::E048_MCP_CLIENT_TOOL_NOT_FOUND, "E048"),
        ];

        for (code, expected) in test_cases {
            // Constant value matches expected string.
            assert_eq!(
                code, expected,
                "Code constant value mismatch for {expected}"
            );

            // Properly formatted: one uppercase letter followed by exactly three digits.
            let mut chars = code.chars();
            let letter = chars.next().expect("invariant: code must be non-empty");
            assert!(
                letter.is_ascii_uppercase(),
                "Code {expected} must start with an uppercase letter"
            );
            let digits: String = chars.collect();
            assert_eq!(
                digits.len(),
                3,
                "Code {expected} must have exactly three digits"
            );
            assert!(
                digits.chars().all(|c| c.is_ascii_digit()),
                "Code {expected} suffix must be all digits"
            );

            // Serialization round-trip.
            let diag = Diagnostic::error(code, format!("Test for {expected}"));
            let json = diag.to_jsonl();
            let parsed: serde_json::Value = serde_json::from_str(&json)
                .expect("invariant: diagnostic must serialize to valid JSON");
            assert_eq!(parsed["code"], expected);
            assert_eq!(parsed["level"], "error");
        }
    }

    #[test]
    fn e040_budget_exceeded_with_details() {
        let mut details = HashMap::new();
        details.insert("budget_tokens".to_string(), "8192".to_string());
        details.insert("used_tokens".to_string(), "9100".to_string());
        let diag = Diagnostic::error(codes::E040_BUDGET_EXCEEDED, "Agent token budget exceeded")
            .with_details(details);
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E040");
        assert_eq!(parsed["details"]["budget_tokens"], "8192");
    }

    #[test]
    fn e042_merge_conflict_with_node() {
        let diag = Diagnostic::error(
            codes::E042_MERGE_CONFLICT,
            "Concurrent patches have irreconcilable conflicts",
        )
        .with_node(&NodeId("duumbi:main/main/entry/5".to_string()));
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E042");
        assert_eq!(parsed["nodeId"], "duumbi:main/main/entry/5");
    }

    #[test]
    fn e047_mcp_client_unreachable_with_url() {
        let mut details = HashMap::new();
        details.insert(
            "server_url".to_string(),
            "http://localhost:9000".to_string(),
        );
        let diag = Diagnostic::error(
            codes::E047_MCP_CLIENT_UNREACHABLE,
            "MCP server connection failed",
        )
        .with_details(details);
        let json = diag.to_jsonl();
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("invariant: diagnostic must serialize to valid JSON");
        assert_eq!(parsed["code"], "E047");
        assert_eq!(parsed["details"]["server_url"], "http://localhost:9000");
    }
}
