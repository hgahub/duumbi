---
name: duumbi-mcp-agent
description: Use DUUMBI through its local MCP server for read-only query, graph inspection, approval-gated graph patches, and evidence-oriented local workflows.
---

# DUUMBI MCP Agent

Use this skill when working in a DUUMBI workspace with `duumbi mcp`.

## Required Flow

1. Start or connect to `target/debug/duumbi mcp`.
2. Call `initialize`.
3. Call `tools/list`.
4. Call `mcp_capability_status`.
5. Prefer read-only tools first:
   - `query_ask`
   - `graph_query`
   - `graph_validate`
   - `graph_describe`
   - `rewrite_list_rules`
   - `rewrite_preview`
6. For graph patches, use:
   - `graph_patch_preview`
   - `graph_patch_request_approval`
   - `approval_status`
   - wait for approval
   - `graph_patch_apply_approval`

## Do Not

- Do not use `graph_mutate` by default; it is a trusted immediate-write
  compatibility tool.
- Do not put credentials in prompts or tool arguments.
- Do not claim build/run benchmark success without MCP transcript evidence.
- Do not mark DUUMBI issues done during Stage 10.

## Error Handling

Inspect `error.data.category` and `error.data.suggestedRepairs` before retrying.
For `approval_required` or `approval_stale`, use the approval tools rather than
retrying an apply request.
