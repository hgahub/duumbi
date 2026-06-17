# DUUMBI MCP Workflow Audit

Related to #719.

This audit maps the external-agent DUUMBI loop to the current MCP surface. It
separates implemented paths from structured gaps so agents do not infer hidden
CLI behavior.

## Current Tool Surface

| Loop step | MCP tool | Safety | Status |
| --- | --- | --- | --- |
| Discover capabilities | `initialize`, `tools/list`, `mcp_capability_status` | read-only | Implemented |
| Ask read-only questions | `query_ask` | read-only, provider-backed | Implemented with provider-unavailable errors when no provider is configured |
| Inspect graph | `graph_query`, `graph_validate`, `graph_describe` | read-only | Implemented |
| Preview graph patch | `graph_patch_preview` | read-only | Implemented |
| Request graph approval | `graph_patch_request_approval` | writes approval ledger only | Implemented |
| Inspect approvals | `approval_status` | read-only | Implemented |
| Decide approval | `approval_decide` | writes approval ledger only | Implemented |
| Apply approved graph patch | `graph_patch_apply_approval` | approval-gated write | Implemented for `.duumbi/graph/main.jsonld` |
| Legacy graph mutation | `graph_mutate` | trusted immediate write | Compatibility path; not the default external-agent path |
| Rewrite rule discovery | `rewrite_list_rules` | read-only | Implemented |
| Rewrite preview | `rewrite_preview` | read-only | Implemented |
| Rewrite apply | `rewrite_apply` | trusted immediate write with snapshot | Existing compatibility path; approval integration still pending |
| Build | `build_compile` | write-capable | Structured unavailable state; shared backend wiring pending |
| Run | `build_run` | write-capable | Structured unavailable state; shared backend wiring pending |
| Dependency search/install | `deps_search`, `deps_install` | write-capable/network | Structured unavailable state; MCP backend wiring pending |
| Intent create/execute | `intent_create`, `intent_execute` | provider-backed write-capable | Structured unavailable state; async dispatch/backend wiring pending |
| Model telemetry | `model_access_summary`, `model_performance_summary`, `model_telemetry_health` | read-only | Implemented |
| Evidence retrieval | none yet | read-only | Pending |

## Agent-Safe Write Path

For graph patches, external agents must use:

1. `graph_patch_preview`
2. `graph_patch_request_approval`
3. `approval_status`
4. human decision through `approval_decide`, TUI, or Studio when available
5. `graph_patch_apply_approval`

`graph_patch_apply_approval` verifies the approval status, workspace hash,
candidate hash, and validation result before writing the graph.

## Known Gaps

- Build/run are listed but still require a shared backend implementation.
- Dependency and intent tools still need async-capable MCP dispatch or explicit
  backend helpers.
- Evidence retrieval is not yet exposed as an MCP tool.
- TUI and Studio approval visibility are still pending.
- Rewrite apply still uses its existing trusted immediate-write path.
- The flagship MCP-only benchmark and raw Rust baseline are not yet implemented.

## Verification

Current automated coverage:

- `cargo test mcp::server::tests:: --lib`
- `cargo test mcp::approval::tests:: --lib`
- `cargo test mcp::tools::query::tests:: --lib`
- `cargo test duumbi719_mcp --test integration_phase12`

Live evidence still required before Stage 11:

- provider-free MCP stdio smoke through `target/debug/duumbi mcp`;
- build/run/evidence path once those tools are implemented or explicitly
  reported blocked;
- low-cost external-agent E2E when inside the USD 1 resource gate, or a
  structured blocked report;
- raw Rust baseline evidence or explicit unavailable reason.
