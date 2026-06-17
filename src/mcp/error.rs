//! Structured DUUMBI MCP diagnostic envelope.
//!
//! JSON-RPC protocol failures still use JSON-RPC error codes. When a request
//! reaches a DUUMBI tool, this module provides stable `error.data` content so
//! external agents can classify failures and choose a repair path.

use serde::Serialize;
use serde_json::Value;

/// Structured MCP tool diagnostic attached to JSON-RPC `error.data`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct McpToolError {
    /// Stable DUUMBI-oriented error code.
    pub code: String,
    /// Broad machine-readable category for repair planning.
    pub category: McpErrorCategory,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Whether retrying the same request can plausibly succeed without edits.
    pub retryable: bool,
    /// Affected graph node ids when known.
    #[serde(rename = "nodeIds")]
    pub node_ids: Vec<String>,
    /// Affected local files when known.
    pub files: Vec<String>,
    /// Affected evidence or build artifacts when known.
    pub artifacts: Vec<String>,
    /// Suggested repair categories.
    #[serde(rename = "suggestedRepairs")]
    pub suggested_repairs: Vec<McpRepairCategory>,
    /// Source tool or subsystem that emitted the diagnostic.
    pub source: McpErrorSource,
}

/// Broad DUUMBI MCP error category.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[allow(dead_code)] // Stage 10 approval/build cycles wire these categories incrementally.
#[serde(rename_all = "snake_case")]
pub enum McpErrorCategory {
    /// Request parameters or JSON shape are invalid.
    Schema,
    /// Workspace is missing required `.duumbi` state.
    Workspace,
    /// Graph parsing failed.
    Parse,
    /// Graph build, type, or ownership validation failed.
    Validation,
    /// A dependency operation is unavailable or failed.
    MissingDependency,
    /// Provider credentials or model access are unavailable.
    ProviderUnavailable,
    /// Network access is unavailable or unsupported.
    NetworkUnavailable,
    /// Human approval is required before the write can proceed.
    ApprovalRequired,
    /// Human approval was rejected.
    ApprovalRejected,
    /// Approval or candidate state is stale.
    ApprovalStale,
    /// Build failed.
    Build,
    /// Runtime execution failed.
    Runtime,
    /// Operation timed out.
    Timeout,
    /// Tool is listed but not fully supported yet.
    Unsupported,
    /// Catch-all for unexpected tool failures.
    Internal,
}

/// Suggested repair category for an agent or human.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpRepairCategory {
    /// Fix request shape or required fields.
    Schema,
    /// Initialize or select a valid workspace.
    Workspace,
    /// Repair JSON-LD parse errors.
    Parse,
    /// Repair graph validation, type, or ownership errors.
    Validation,
    /// Install, vendor, or declare dependencies.
    MissingDependency,
    /// Configure provider credentials or choose a non-provider path.
    Provider,
    /// Retry with network available or use an offline path.
    Network,
    /// Request or inspect human approval.
    Approval,
    /// Inspect build diagnostics.
    Build,
    /// Inspect runtime diagnostics.
    Runtime,
    /// Use an alternate supported tool or CLI path.
    Unsupported,
}

/// Source of an MCP diagnostic.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct McpErrorSource {
    /// MCP tool name or JSON-RPC method.
    pub tool: String,
}

impl McpToolError {
    /// Builds a diagnostic for an MCP tool error string.
    #[must_use]
    pub fn from_tool_message(tool: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        let category = classify(tool, &message);
        let suggested_repairs = suggested_repairs(&category);
        let retryable = matches!(
            category,
            McpErrorCategory::NetworkUnavailable | McpErrorCategory::Timeout
        );
        Self {
            code: code_for(&category).to_string(),
            category,
            message,
            retryable,
            node_ids: Vec::new(),
            files: files_for(tool),
            artifacts: Vec::new(),
            suggested_repairs,
            source: McpErrorSource {
                tool: tool.to_string(),
            },
        }
    }

    /// Serializes the diagnostic for JSON-RPC `error.data`.
    #[must_use]
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("invariant: MCP tool error serializes")
    }
}

fn classify(tool: &str, message: &str) -> McpErrorCategory {
    let lower = message.to_lowercase();
    if lower.contains("missing required")
        || lower.contains("invalid patch ops")
        || lower.contains(" must be ")
        || lower.contains(" entries must be ")
    {
        McpErrorCategory::Schema
    } else if lower.contains("approval required") {
        McpErrorCategory::ApprovalRequired
    } else if lower.contains("approval stale") {
        McpErrorCategory::ApprovalStale
    } else if lower.contains("was rejected") {
        McpErrorCategory::ApprovalRejected
    } else if lower.contains("cannot read graph dir")
        || lower.contains("cannot read main.jsonld")
        || lower.contains("workspace")
    {
        McpErrorCategory::Workspace
    } else if lower.contains("parse error") || lower.contains("invalid json") {
        McpErrorCategory::Parse
    } else if lower.contains("validation")
        || lower.contains("graph errors")
        || lower.contains("patch failed")
    {
        McpErrorCategory::Validation
    } else if lower.contains("provider") || lower.contains("llm") || tool.starts_with("intent_") {
        McpErrorCategory::ProviderUnavailable
    } else if lower.contains("network") || lower.contains("registry") || tool.starts_with("deps_") {
        McpErrorCategory::NetworkUnavailable
    } else if lower.contains("timeout") {
        McpErrorCategory::Timeout
    } else if lower.contains("duumbi build") || tool == "build_compile" {
        McpErrorCategory::Build
    } else if lower.contains("duumbi run") || tool == "build_run" {
        McpErrorCategory::Runtime
    } else if lower.contains("requires")
        || lower.contains("not yet")
        || lower.contains("unsupported")
    {
        McpErrorCategory::Unsupported
    } else {
        McpErrorCategory::Internal
    }
}

fn code_for(category: &McpErrorCategory) -> &'static str {
    match category {
        McpErrorCategory::Schema => "mcp.schema",
        McpErrorCategory::Workspace => "mcp.workspace_uninitialized",
        McpErrorCategory::Parse => "mcp.parse",
        McpErrorCategory::Validation => "mcp.validation",
        McpErrorCategory::MissingDependency => "mcp.missing_dependency",
        McpErrorCategory::ProviderUnavailable => "mcp.provider_unavailable",
        McpErrorCategory::NetworkUnavailable => "mcp.network_unavailable",
        McpErrorCategory::ApprovalRequired => "mcp.approval_required",
        McpErrorCategory::ApprovalRejected => "mcp.approval_rejected",
        McpErrorCategory::ApprovalStale => "mcp.approval_stale",
        McpErrorCategory::Build => "mcp.build",
        McpErrorCategory::Runtime => "mcp.runtime",
        McpErrorCategory::Timeout => "mcp.timeout",
        McpErrorCategory::Unsupported => "mcp.unsupported",
        McpErrorCategory::Internal => "mcp.internal",
    }
}

fn suggested_repairs(category: &McpErrorCategory) -> Vec<McpRepairCategory> {
    match category {
        McpErrorCategory::Schema => vec![McpRepairCategory::Schema],
        McpErrorCategory::Workspace => vec![McpRepairCategory::Workspace],
        McpErrorCategory::Parse => vec![McpRepairCategory::Parse],
        McpErrorCategory::Validation => vec![McpRepairCategory::Validation],
        McpErrorCategory::MissingDependency => vec![McpRepairCategory::MissingDependency],
        McpErrorCategory::ProviderUnavailable => vec![McpRepairCategory::Provider],
        McpErrorCategory::NetworkUnavailable => vec![McpRepairCategory::Network],
        McpErrorCategory::ApprovalRequired
        | McpErrorCategory::ApprovalRejected
        | McpErrorCategory::ApprovalStale => vec![McpRepairCategory::Approval],
        McpErrorCategory::Build => vec![McpRepairCategory::Build],
        McpErrorCategory::Runtime => vec![McpRepairCategory::Runtime],
        McpErrorCategory::Timeout => vec![McpRepairCategory::Unsupported],
        McpErrorCategory::Unsupported => vec![McpRepairCategory::Unsupported],
        McpErrorCategory::Internal => vec![McpRepairCategory::Unsupported],
    }
}

fn files_for(tool: &str) -> Vec<String> {
    match tool {
        "graph_query" | "graph_mutate" | "graph_validate" | "graph_describe" => {
            vec![".duumbi/graph/main.jsonld".to_string()]
        }
        "rewrite_preview" | "rewrite_apply" => vec![".duumbi/graph".to_string()],
        "model_performance_summary" | "model_telemetry_health" => {
            vec![".duumbi/knowledge/model-performance".to_string()]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_run_argument_shape_errors_are_schema_errors() {
        let err = McpToolError::from_tool_message("build_run", "args entries must be strings");

        assert_eq!(err.code, "mcp.schema");
        assert_eq!(err.category, McpErrorCategory::Schema);
        assert_eq!(err.suggested_repairs, vec![McpRepairCategory::Schema]);
    }
}
