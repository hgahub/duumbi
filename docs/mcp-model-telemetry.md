# MCP Model Telemetry Analytics

DUUMBI exposes local model telemetry analytics through read-only MCP tools.
These tools inspect existing local telemetry stores and return bounded JSON
summaries. They do not change provider configuration, credentials, routing,
model catalog state, graph files, intents, or telemetry files.

## Tools

`model_access_summary`

- Source: `~/.duumbi/knowledge/model-access/current.json`.
- Optional source when raw mode is explicit:
  `~/.duumbi/knowledge/model-access/events.jsonl`.
- Summarizes provider/model accessibility states: `accessible`, `denied`,
  `auth_failed`, and `unknown`.
- Flags stale access evidence. The default stale threshold is 168 hours.

`model_performance_summary`

- Source: workspace `.duumbi/knowledge/model-performance/aggregates.json`.
- Optional source when raw mode is explicit:
  `.duumbi/knowledge/model-performance/events.jsonl`.
- Summarizes calls, successes, failures, success/failure rates, parse failures,
  validation failures, retries, EWMA latency, EWMA cost, and task profile
  dimensions.

`model_telemetry_health`

- Sources: both model-access and model-performance stores.
- Reports whether each source is absent, empty, present, stale, partial, or
  malformed.
- Returns source-specific health metadata without raw event rows:
  `current_status`, `record_count`, `latest_checked`, and
  `stale_record_count` for model access; `aggregate_status`,
  `aggregate_count`, `latest_updated`, and `stale_aggregate_count` for model
  performance.

## Inputs

Common inputs:

- `provider`: optional provider filter.
- `model`: optional model filter.
- `stale_after_hours`: optional freshness threshold, default `168`, maximum
  `8760`.
- `limit`: optional aggregate row limit, default `25`, maximum `100`.
- `include_raw_events`: optional boolean, default `false`. This is rejected by
  `model_telemetry_health` because health reports never include raw rows.

Performance-only filters:

- `agent_role`
- `task_type`
- `complexity`
- `scope`
- `risk`

Raw event mode is explicit and bounded:

- `include_raw_events` must be `true`.
- `limit` must be supplied.
- Raw event `limit` must be at most `50`.
- Raw event rows are newest-first and redacted.
- Raw event rows honor the same provider/model and task-profile filters as
  aggregate rows.

Unsupported fields and wrong JSON types return validation errors instead of
falling back to broader data dumps.

## Output Contract

Each tool returns JSON in the MCP text content wrapper:

```json
{
  "status": "success",
  "data_status": "present",
  "generated_at": "2026-06-13T10:00:00Z",
  "scope": "workspace_model_performance",
  "filters": {},
  "summary": {},
  "rows": [],
  "raw_events": null,
  "privacy": {},
  "warnings": []
}
```

`status` can be `success`, `partial`, or `error`.

`data_status` can be `present`, `empty`, `absent`, `stale`, `partial`, or
`malformed`.

Rows are deterministic for the same stored data and filters, except
`generated_at`.

## Privacy

Default outputs never include:

- provider secret values;
- credential fingerprints or fingerprint-derived stable identifiers;
- provider messages;
- raw provider error bodies;
- prompts or completions;
- raw event rows.

Raw access event rows omit credential fingerprints and provider messages.

Raw performance event rows include numeric usage metadata and task profile
fields, but omit prompt text, completion text, raw provider bodies, and full
validation messages.

## Interpretation

`accessible` means DUUMBI has recorded a successful local probe for a
provider/model under some local credential context. It does not prove universal
provider availability.

`denied` means DUUMBI recorded provider or subscription denial under a local
credential context. It does not prove that the model is unavailable to every
user.

Performance success/failure rates are local historical evidence. They are not a
global model quality guarantee and they do not update routing automatically.

Stale data remains visible, but stale evidence should not be treated as current
access or routing proof.

## Examples

List tools:

```json
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
```

Query model access:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "model_access_summary",
    "arguments": {
      "provider": "minimax",
      "limit": 10
    }
  }
}
```

Query model performance for one task profile:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "model_performance_summary",
    "arguments": {
      "task_type": "create",
      "risk": "high",
      "limit": 10
    }
  }
}
```

Check source health:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "model_telemetry_health",
    "arguments": {}
  }
}
```
