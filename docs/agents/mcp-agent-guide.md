# DUUMBI MCP Agent Guide

Related to #719.

This guide is for local, trusted external coding agents using DUUMBI through
stdio MCP.

## Start MCP

```sh
cargo build
target/debug/duumbi mcp
```

The MCP server reads newline-delimited JSON-RPC requests from stdin and writes
newline-delimited JSON-RPC responses to stdout.

## Discovery First

Start with:

1. `initialize`
2. `tools/list`
3. `tools/call` with `name: "mcp_capability_status"`

Use the DUUMBI metadata in `tools/list`:

- `duumbi.safety`
- `duumbi.approvalRequired`
- `duumbi.writes`
- `duumbi.providerRequired`
- `duumbi.networkRequired`
- `duumbi.unavailableReason`

## Read-Only Query

Use `query_ask` for questions. It is read-only. Mutation-shaped questions
return a suggested handoff instead of mutating the graph.

Example arguments:

```json
{
  "question": "What does the main module return?",
  "module": "main",
  "include_sources": true
}
```

If no provider is configured, DUUMBI returns `provider_unavailable` in
`error.data`. Configure providers through DUUMBI provider setup, not by putting
secrets in prompts.

## Agent-Safe Graph Writes

Use the approval path:

1. `graph_patch_preview`
2. `graph_patch_request_approval`
3. `approval_status`
4. wait for a human approval decision
5. `graph_patch_apply_approval`

Do not use `graph_mutate` as the default external-agent path. It remains a
trusted compatibility tool and is labeled `trusted_immediate_write`.

## Structured Errors

When a tool fails, inspect `error.data` first. Use `category`,
`suggestedRepairs`, and `source.tool` to decide the next action.

## Current Limits

- Build/run MCP tools still report structured unavailable state.
- Dependency and intent MCP tools still need backend wiring.
- Evidence retrieval is not yet exposed as a dedicated MCP tool.
- TUI and Studio approval views are pending.
- The flagship MCP-only benchmark is pending.

## Benchmark Evidence

Use `examples/flagship-http-sqlite-json` as the Stage 10 flagship scenario.
Record `initialize`, `tools/list`, `mcp_capability_status`, graph or intent
inspection, build/run evidence when available, and the loopback JSON response.
Until build/run/evidence tools are implemented, report this path as blocked
instead of claiming live MCP-only success.

## Safety Rules

- Keep Query mode read-only.
- Do not write graph files without approval unless a human explicitly chooses a
  trusted compatibility path.
- Do not include credentials or private environment values in prompts,
  diagnostics, docs, or evidence.
- Do not claim live E2E success without a transcript or structured evidence.
- Do not mark issue #719 done from Stage 10.
