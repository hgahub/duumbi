# DUUMBI MCP Error Contract

Related to #719.

DUUMBI MCP keeps JSON-RPC protocol failures separate from DUUMBI tool
diagnostics. If a request reaches a DUUMBI tool and the tool fails, the
JSON-RPC response uses the normal `error` object and attaches a structured
DUUMBI diagnostic in `error.data`.

## Shape

```json
{
  "code": "mcp.provider_unavailable",
  "category": "provider_unavailable",
  "message": "Provider unavailable for query_ask: No LLM providers configured.",
  "retryable": false,
  "nodeIds": [],
  "files": [],
  "artifacts": [],
  "suggestedRepairs": ["provider"],
  "source": {
    "tool": "query_ask"
  }
}
```

## Categories

| Category | Meaning |
| --- | --- |
| `schema` | Request arguments are missing or malformed. |
| `workspace` | Workspace state such as `.duumbi/graph/main.jsonld` is missing or unreadable. |
| `parse` | JSON or JSON-LD parsing failed. |
| `validation` | Graph patch, build, type, or ownership validation failed. |
| `missing_dependency` | Dependency state is absent or invalid. |
| `provider_unavailable` | Provider configuration or credential access is unavailable. |
| `network_unavailable` | Network-backed registry/provider path is unavailable. |
| `approval_required` | The requested apply operation has no approved matching record. |
| `approval_rejected` | The approval record was rejected. |
| `approval_stale` | Workspace or candidate hash no longer matches the approved record. |
| `build` | Build failed or build backend is unavailable. |
| `runtime` | Runtime execution failed or run backend is unavailable. |
| `timeout` | Bounded execution timed out. |
| `unsupported` | Tool is listed but the full backend is not implemented yet. |
| `internal` | Unexpected tool failure. |

## Agent Rules

- Do not retry non-retryable errors blindly.
- For `schema`, repair arguments and call the same tool again.
- For `provider_unavailable`, either configure a provider through `/provider`
  or use provider-free MCP paths.
- For `approval_required`, request or inspect approval before apply.
- For `approval_stale`, regenerate preview and request a fresh approval.
- For `unsupported`, use the listed fallback only as a human-facing suggestion;
  do not assume hidden CLI access is allowed.

## Privacy

MCP diagnostics must not include provider keys, registry tokens, Slack tokens,
raw private environment values, or unbounded provider responses.
