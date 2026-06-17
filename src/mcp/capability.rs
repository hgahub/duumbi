//! Agent-facing MCP tool capability metadata.
//!
//! The definitions in this module are the source of truth for `tools/list` and
//! for the DUUMBI capability/status tool. They intentionally describe safety
//! and availability in machine-readable fields so external coding agents do
//! not have to infer write behavior from prose.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP tool definition returned in `tools/list` responses.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    /// Unique tool name used as `name` in `tools/call`.
    pub name: String,
    /// Human-readable description shown to the LLM.
    pub description: String,
    /// JSON Schema object describing the tool's input parameters.
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    /// DUUMBI-specific agent metadata for safety and planning.
    #[serde(rename = "duumbi")]
    pub metadata: ToolMetadata,
}

/// DUUMBI-specific planning metadata for one MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolMetadata {
    /// DUUMBI delivery stage that owns this tool behavior.
    pub stage: String,
    /// Whether the tool reads only, may write, or is a legacy trusted write.
    pub safety: ToolSafety,
    /// Whether the documented agent-safe path requires human approval.
    #[serde(rename = "approvalRequired")]
    pub approval_required: bool,
    /// Durable workspace areas this tool may write.
    pub writes: Vec<String>,
    /// Whether normal execution needs a configured LLM provider.
    #[serde(rename = "providerRequired")]
    pub provider_required: bool,
    /// Whether normal execution needs network access.
    #[serde(rename = "networkRequired")]
    pub network_required: bool,
    /// Evidence or report classes the tool can produce.
    #[serde(rename = "evidenceProduced")]
    pub evidence_produced: Vec<String>,
    /// Structured reason when a listed tool is not fully implemented yet.
    #[serde(rename = "unavailableReason", skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

/// Safety classification for DUUMBI MCP tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolSafety {
    /// The tool must not mutate graph, intent, dependency, build, or evidence state.
    ReadOnly,
    /// The tool can write local workspace state through an agent-safe path.
    WriteCapable,
    /// Legacy compatibility tool that writes immediately and should not be the
    /// default external-agent path.
    TrustedImmediateWrite,
}

impl ToolMetadata {
    /// Metadata for a read-only tool.
    #[must_use]
    pub fn read_only(stage: &str, evidence: &[&str]) -> Self {
        Self {
            stage: stage.to_string(),
            safety: ToolSafety::ReadOnly,
            approval_required: false,
            writes: Vec::new(),
            provider_required: false,
            network_required: false,
            evidence_produced: evidence.iter().map(|item| (*item).to_string()).collect(),
            unavailable_reason: None,
        }
    }

    /// Metadata for a legacy immediate-write tool retained for compatibility.
    #[must_use]
    pub fn trusted_write(stage: &str, writes: &[&str], evidence: &[&str]) -> Self {
        Self {
            stage: stage.to_string(),
            safety: ToolSafety::TrustedImmediateWrite,
            approval_required: false,
            writes: writes.iter().map(|item| (*item).to_string()).collect(),
            provider_required: false,
            network_required: false,
            evidence_produced: evidence.iter().map(|item| (*item).to_string()).collect(),
            unavailable_reason: None,
        }
    }

    /// Metadata for a listed tool whose full implementation is not available yet.
    #[must_use]
    pub fn unavailable(
        stage: &str,
        provider_required: bool,
        network_required: bool,
        reason: &str,
    ) -> Self {
        Self {
            stage: stage.to_string(),
            safety: ToolSafety::WriteCapable,
            approval_required: false,
            writes: Vec::new(),
            provider_required,
            network_required,
            evidence_produced: Vec::new(),
            unavailable_reason: Some(reason.to_string()),
        }
    }
}

/// Returns the public MCP tool definitions plus DUUMBI metadata.
#[must_use]
pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        tool(
            "mcp_capability_status",
            "Read-only DUUMBI MCP capability and workspace status report. Helps external agents plan the local init, query, approval, build, run, and evidence loop without mutating workspace state.",
            empty_schema(),
            ToolMetadata::read_only("stage10", &["capability_status"]),
        ),
        tool(
            "query_ask",
            "Read-only conversational Query mode over the local DUUMBI workspace. Returns answer text, model metadata, source references, confidence, and suggested handoff without graph, intent, dependency, build, or evidence writes.",
            serde_json::json!({
                "type": "object",
                "required": ["question"],
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Read-only question to answer from local DUUMBI context"
                    },
                    "module": {
                        "type": "string",
                        "description": "Optional visible module name, such as 'main'"
                    },
                    "c4_level": {
                        "type": "string",
                        "description": "Optional visible C4 level"
                    },
                    "include_sources": {
                        "type": "boolean",
                        "description": "Whether source references should be returned; defaults to true"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata {
                provider_required: true,
                ..ToolMetadata::read_only("stage10", &["query_answer"])
            },
        ),
        tool(
            "graph_patch_preview",
            "Read-only graph patch preview. Applies patch operations in memory, validates the candidate, and returns workspace/candidate hashes without writing graph files.",
            serde_json::json!({
                "type": "object",
                "required": ["ops"],
                "properties": {
                    "ops": {
                        "type": "array",
                        "description": "Array of GraphPatch operations with 'kind' tag"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::read_only("stage10", &["graph_patch_preview", "graph_validation"]),
        ),
        tool(
            "graph_patch_request_approval",
            "Create a pending local approval record for a graph patch candidate after read-only preview and validation. Writes only approval ledger state, not graph files.",
            serde_json::json!({
                "type": "object",
                "required": ["ops", "summary"],
                "properties": {
                    "ops": {
                        "type": "array",
                        "description": "Array of GraphPatch operations with 'kind' tag"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Human-readable summary of the requested graph change"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata {
                stage: "stage10".to_string(),
                safety: ToolSafety::WriteCapable,
                approval_required: false,
                writes: vec![".duumbi/session/approvals".to_string()],
                provider_required: false,
                network_required: false,
                evidence_produced: vec![
                    "approval_record".to_string(),
                    "graph_patch_preview".to_string(),
                ],
                unavailable_reason: None,
            },
        ),
        tool(
            "approval_status",
            "Read-only local approval ledger lookup. Returns one approval record by id or all approval records.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Optional local approval id"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::read_only("stage10", &["approval_record"]),
        ),
        tool(
            "approval_decide",
            "Record a local human approval decision for a pending MCP approval record. This writes only approval ledger state.",
            serde_json::json!({
                "type": "object",
                "required": ["id", "decision"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Local approval id"
                    },
                    "decision": {
                        "type": "string",
                        "enum": ["approve", "reject"]
                    },
                    "decision_source": {
                        "type": "string",
                        "description": "Decision source such as mcp, tui, or studio"
                    },
                    "rejection_reason": {
                        "type": "string",
                        "description": "Optional rejection reason"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata {
                stage: "stage10".to_string(),
                safety: ToolSafety::WriteCapable,
                approval_required: false,
                writes: vec![".duumbi/session/approvals".to_string()],
                provider_required: false,
                network_required: false,
                evidence_produced: vec!["approval_decision".to_string()],
                unavailable_reason: None,
            },
        ),
        tool(
            "graph_patch_apply_approval",
            "Apply an approved graph patch exactly after checking approval status, workspace hash, candidate hash, and validation. Rejects stale or unapproved candidates.",
            serde_json::json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Approved local graph patch approval id"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata {
                stage: "stage10".to_string(),
                safety: ToolSafety::WriteCapable,
                approval_required: true,
                writes: vec![
                    ".duumbi/graph/main.jsonld".to_string(),
                    ".duumbi/session/approvals".to_string(),
                ],
                provider_required: false,
                network_required: false,
                evidence_produced: vec![
                    "approval_record".to_string(),
                    "graph_validation".to_string(),
                ],
                unavailable_reason: None,
            },
        ),
        tool(
            "graph_query",
            "Query the DUUMBI semantic graph by node ID, @type, or name pattern. Returns matching nodes from all .jsonld files in the workspace.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": {
                        "type": "string",
                        "description": "Exact @id to look up (e.g. 'duumbi:main/main/entry/0')"
                    },
                    "type_filter": {
                        "type": "string",
                        "description": "Match nodes by @type (e.g. 'duumbi:Add', 'duumbi:Function')"
                    },
                    "name_pattern": {
                        "type": "string",
                        "description": "Substring match against duumbi:name field"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::read_only("stage10", &["graph_query_result"]),
        ),
        tool(
            "graph_mutate",
            "Legacy trusted graph patch operation. Applies atomic patch operations to the workspace graph and validates the result before writing. External agents should prefer approval-gated preview/request/apply tools when available.",
            serde_json::json!({
                "type": "object",
                "required": ["ops"],
                "properties": {
                    "ops": {
                        "type": "array",
                        "description": "Array of GraphPatch operations with 'kind' tag",
                        "items": {
                            "type": "object",
                            "required": ["kind"],
                            "properties": {
                                "kind": {
                                    "type": "string",
                                    "enum": [
                                        "add_function", "add_block", "add_op",
                                        "modify_op", "replace_block", "remove_node", "set_edge"
                                    ]
                                }
                            }
                        }
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::trusted_write(
                "stage10",
                &[".duumbi/graph/main.jsonld"],
                &["graph_validation"],
            ),
        ),
        tool(
            "graph_validate",
            "Validate the workspace graph without modifying it. Runs the full parse, build, and validate pipeline and returns diagnostics.",
            empty_schema(),
            ToolMetadata::read_only("stage10", &["graph_validation"]),
        ),
        tool(
            "graph_describe",
            "Describe the workspace graph as human-readable pseudo-code. Useful for understanding the current state of the program.",
            empty_schema(),
            ToolMetadata::read_only("stage10", &["graph_description"]),
        ),
        tool(
            "build_compile",
            "Compile the workspace graph to a native binary. Currently reports structured unavailable state until the shared backend path is wired.",
            empty_schema(),
            ToolMetadata::unavailable(
                "stage10",
                false,
                false,
                "Full MCP build backend is not wired yet; use CLI build until this Stage 10 slice implements it.",
            ),
        ),
        tool(
            "build_run",
            "Compile and run the workspace binary. Currently reports structured unavailable state until the shared backend path is wired.",
            empty_schema(),
            ToolMetadata::unavailable(
                "stage10",
                false,
                false,
                "Full MCP run backend is not wired yet; use CLI run until this Stage 10 slice implements it.",
            ),
        ),
        tool(
            "deps_search",
            "Search registries for available DUUMBI modules. Currently reports structured unavailable state for network-dependent registry search.",
            serde_json::json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search terms to look for in the registry"
                    },
                    "registry": {
                        "type": "string",
                        "description": "Limit search to this named registry (optional)"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::unavailable(
                "stage10",
                false,
                true,
                "Registry search can require network access and is not yet exposed through the synchronous MCP backend.",
            ),
        ),
        tool(
            "deps_install",
            "Install all declared dependencies into the local cache. Currently reports structured unavailable state until dependency helpers are exposed through MCP.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "frozen": {
                        "type": "boolean",
                        "description": "Fail if the lockfile would change (CI reproducibility)"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::unavailable(
                "stage10",
                false,
                true,
                "Dependency install can require network access and is not yet exposed through the MCP backend.",
            ),
        ),
        tool(
            "intent_create",
            "Create an intent spec from a natural-language description. Currently reports structured unavailable state until async provider dispatch is available.",
            serde_json::json!({
                "type": "object",
                "required": ["description"],
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Natural language description of what to build"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::unavailable(
                "stage10",
                true,
                false,
                "Intent creation requires provider-backed async dispatch that is not yet exposed through MCP.",
            ),
        ),
        tool(
            "intent_execute",
            "Execute an intent: decompose, mutate graph, and verify tests. Currently reports structured unavailable state until async provider dispatch is available.",
            serde_json::json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Intent slug/name to execute"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::unavailable(
                "stage10",
                true,
                false,
                "Intent execution requires provider-backed async dispatch that is not yet exposed through MCP.",
            ),
        ),
        tool(
            "model_access_summary",
            "Read-only local analytics over the user-level model-access store. Summarizes ~/.duumbi/knowledge/model-access/current.json and optional bounded redacted events without mutating telemetry, credentials, provider config, model catalog, graph files, or intents.",
            model_telemetry_schema(false),
            ToolMetadata::read_only("stage10", &["model_access_summary"]),
        ),
        tool(
            "model_performance_summary",
            "Read-only local analytics over workspace model-performance telemetry. Summarizes .duumbi/knowledge/model-performance/aggregates.json and optional bounded redacted events without provider calls, routing changes, or telemetry writes.",
            model_telemetry_schema(true),
            ToolMetadata::read_only("stage10", &["model_performance_summary"]),
        ),
        tool(
            "model_telemetry_health",
            "Read-only local health report for model-access and model-performance telemetry stores. Reports absent, empty, stale, partial, malformed, or present source state without returning secrets or raw event rows.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "provider": {
                        "type": "string",
                        "description": "Optional provider filter echoed in normalized filters"
                    },
                    "model": {
                        "type": "string",
                        "description": "Optional model filter echoed in normalized filters"
                    },
                    "stale_after_hours": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 8760,
                        "description": "Freshness threshold in hours; defaults to 168"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Normalized row limit echoed in filters; defaults to 25"
                    },
                    "include_raw_events": {
                        "type": "boolean",
                        "description": "Must remain false for health; raw event rows are not returned"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::read_only("stage10", &["model_telemetry_health"]),
        ),
        tool(
            "rewrite_list_rules",
            "Read-only semantic rewrite rule discovery. Does not read or write graph files, snapshots, config, credentials, registry cache, intents, or telemetry.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "include_experimental": {
                        "type": "boolean",
                        "description": "Include preview-only experimental rules; defaults to true"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::read_only("stage10", &["rewrite_rule_catalog"]),
        ),
        tool(
            "rewrite_preview",
            "Read-only semantic rewrite preview for one rule and module. Parses, builds, validates, and matches without writing graph files or snapshots.",
            serde_json::json!({
                "type": "object",
                "required": ["rule_id"],
                "properties": {
                    "rule_id": {
                        "type": "string",
                        "description": "Stable rewrite rule ID"
                    },
                    "module": {
                        "type": "string",
                        "description": "Module name such as 'main', or a path to a .jsonld file; defaults to main"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Maximum matches to return, bounded by engine limits"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::read_only("stage10", &["rewrite_preview", "graph_validation"]),
        ),
        tool(
            "rewrite_apply",
            "Write-capable semantic rewrite apply. Reruns matching and validation, saves an undo snapshot, then writes only after the candidate graph validates.",
            serde_json::json!({
                "type": "object",
                "required": ["rule_id"],
                "properties": {
                    "rule_id": {
                        "type": "string",
                        "description": "Stable rewrite rule ID"
                    },
                    "module": {
                        "type": "string",
                        "description": "Module name such as 'main', or a path to a .jsonld file; defaults to main"
                    },
                    "match_id": {
                        "type": "string",
                        "description": "Selected match ID from rewrite_preview"
                    },
                    "all": {
                        "type": "boolean",
                        "description": "Apply all matches within the bounded max_matches setting"
                    },
                    "max_matches": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 10,
                        "description": "Maximum matches for all=true"
                    }
                },
                "additionalProperties": false
            }),
            ToolMetadata::trusted_write(
                "stage10",
                &[".duumbi/graph", ".duumbi/snapshots"],
                &["rewrite_apply", "graph_validation", "snapshot"],
            ),
        ),
    ]
}

fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    metadata: ToolMetadata,
) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema,
        metadata,
    }
}

fn empty_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

fn model_telemetry_schema(include_profile_filters: bool) -> Value {
    let mut properties = serde_json::json!({
        "provider": {
            "type": "string",
            "description": "Optional provider name filter"
        },
        "model": {
            "type": "string",
            "description": "Optional model name filter"
        },
        "stale_after_hours": {
            "type": "integer",
            "minimum": 1,
            "maximum": 8760,
            "description": "Freshness threshold in hours; defaults to 168"
        },
        "limit": {
            "type": "integer",
            "minimum": 1,
            "maximum": 100,
            "description": "Aggregate row limit; defaults to 25. Required and capped at 50 when include_raw_events is true."
        },
        "include_raw_events": {
            "type": "boolean",
            "description": "Explicit bounded raw mode; defaults to false and requires a limit when true"
        }
    });
    if include_profile_filters {
        let object = properties
            .as_object_mut()
            .expect("invariant: schema properties object");
        for field in ["agent_role", "task_type", "complexity", "scope", "risk"] {
            object.insert(
                field.to_string(),
                serde_json::json!({
                    "type": "string",
                    "description": format!("Optional {field} task-profile filter")
                }),
            );
        }
    }
    serde_json::json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false
    })
}
