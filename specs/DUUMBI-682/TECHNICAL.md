# DUUMBI-682: Expose Model Usage Telemetry Analytics Via MCP - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-682/PRODUCT.md` by adding a
local, read-only MCP analytics surface over DUUMBI's existing model-access and
model-performance telemetry stores.

V1 must make these questions answerable through the MCP server without manual
JSON file inspection:

- which provider/model pairs have accessible, denied, auth-failed, unknown, or
  stale model-access evidence;
- which provider/model/task profiles have the highest success, failure, retry,
  parse-failure, validation-failure, latency, and cost evidence;
- whether the underlying telemetry stores are absent, empty, stale, partially
  unreadable, or malformed.

Related to #682. This is a technical-specification artifact only. The execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Reviewer agents checking MCP protocol behavior, privacy boundaries, telemetry
  parsing, no-mutation guarantees, and test coverage.
- Tester agents building fixture-backed MCP and store-level evidence.
- Docs agents updating developer-facing MCP analytics documentation.
- Stage 9 technical reviewers checking implementability and resource policy.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/682
- Product spec: `specs/DUUMBI-682/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/695
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/682#issuecomment-4695382423
- Stage 7 product spec approval decision:
  https://github.com/hgahub/duumbi/issues/682#issuecomment-4695598949
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Query mode specification: `docs/modes/query-mode-spec.md`
- Source intake note:
  `Duumbi/00 Inbox (ToProcess)/2026-06-06 - MCP Model Telemetry Analytics.md`
- PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Agentic development map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Workflow map:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Development Intake to Delivery Workflow.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- MCP resources and prompts note:
  `Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/MCP Resources and Prompts.md`

Relevant source facts verified for Stage 8:

- `src/mcp/server.rs`
  - Implements JSON-RPC 2.0 over stdio.
  - Advertises `capabilities.tools` from `initialize`.
  - Supports `tools/list` and `tools/call`.
  - Does not currently implement MCP `resources/list`, `resources/read`,
    `resources/templates/list`, or `prompts/list`.
  - `tools/call` wraps successful tool results as a single text content item
    containing pretty JSON.
  - Tool dispatch is synchronous and passes the active workspace path to tool
    handlers.
- `src/mcp/tools/mod.rs`
  - Declares existing tool namespaces: `build`, `deps`, `graph`, and `intent`.
  - MCP tool handlers currently return `Result<serde_json::Value, String>`.
- `src/mcp/tools/graph.rs`
  - Provides the current pattern for synchronous MCP filesystem tools.
  - `graph_query`, `graph_validate`, and `graph_describe` read workspace files;
    `graph_mutate` is the only write-capable graph tool.
- `src/agents/model_access.rs`
  - Persists user-level model-access metadata under
    `~/.duumbi/knowledge/model-access`.
  - Writes `current.json` and append-only `events.jsonl`.
  - Current records and events include `credential_fingerprint`, provider,
    model, access status, reason code, sanitized message, probe version, and
    timestamps.
  - `ModelAccessStore::load_db()` and `load_db_from_home()` return an empty
    database when the file is absent or malformed. That is acceptable for
    routing callers but insufficient for analytics health reporting.
  - `model_access_dir_for_home(home)` is public and can anchor read-only
    analytics path helpers.
- `src/agents/model_performance.rs`
  - Persists workspace-level model-performance telemetry under
    `.duumbi/knowledge/model-performance`.
  - Writes append-only `events.jsonl` and rolling `aggregates.json`.
  - `ModelCallEvent` includes provider, model, agent role, template version,
    task type, complexity, scope, risk, token estimates, latency, first-token
    latency, estimated cost, parse success, patch count, validation errors,
    retries, and final outcome.
  - `ModelAggregate` includes calls, successes, failures, EWMA latency, EWMA
    cost, parse failures, validation failures, retries, and last update.
  - Aggregate keys currently have exactly seven pipe-separated fields:
    `provider|model|agent_role|task_type|complexity|scope|risk`.
  - `ModelPerformanceStore::load_db(workspace)` returns an empty database when
    the aggregate file is absent or malformed. Analytics needs a fallible read
    path to distinguish those states.
- `src/cli/provider_startup.rs` and `src/cli/app.rs`
  - Existing provider setup/probe flows can create model-access telemetry.
  - A provider probe can call one request per catalog model for that provider,
    so live provider evidence must remain manual, bounded, and outside CI.
- `tests/integration_phase12.rs`
  - Exercises MCP graph tools without live LLM keys or network access.
  - Provides a suitable integration-test location for fixture-backed MCP
    discovery and `tools/call` coverage.
- `docs/modes/query-mode-spec.md`
  - Defines `query` as read-only: it may inspect, explain, and recommend, but
    must not mutate project state.
- Vault PRD and Agentic Development Map
  - Emphasize read-only questions over source, graph, sessions, knowledge, and
    evidence before mutation begins.
- `MCP Resources and Prompts.md`
  - Says MCP resources expose structured context and prompts expose reusable
    workflows. The current Rust MCP server has not implemented those protocol
    methods yet.

Active workflow state verified:

- Issue #682 is open.
- Issue #682 is labeled `accepted`, `product-spec-approved`, and
  `needs-tech-spec`.
- Product spec PR #695 is merged.
- Stage 7 approval was recorded manually because the maintainer stated that no
  Copilot review submission will be produced for this transition.
- GitHub Project items were not exposed by `gh issue view`; labels and issue
  comments are the available verified workflow state.

Assumptions for implementation:

- V1 should use MCP tools, not MCP resources, because the current MCP server only
  advertises and dispatches `tools`.
- MCP resources and prompts remain follow-up work unless Stage 9 explicitly
  approves expanding the MCP protocol surface before this issue is built.
- The analytics module may add read-only path and fallible-load helpers around
  model-access and model-performance stores, but must not change existing
  routing loader behavior.
- Default analytics should not promise aggregate token totals or
  template-version grouping. In current source, token counts and template
  version are raw event fields, not aggregate fields.
- Raw event access is part of V1 because the approved BDD scenarios cover it.
  It must be explicit, bounded, redacted, and disabled by default.

## Affected Areas

Expected source areas for Stage 10 implementation:

- MCP tool registration and dispatch:
  - `src/mcp/server.rs`
  - `src/mcp/tools/mod.rs`
- New analytics tool module:
  - `src/mcp/tools/model_telemetry.rs`
- Telemetry read helpers:
  - `src/agents/model_access.rs`
  - `src/agents/model_performance.rs`
- MCP and telemetry tests:
  - colocated unit tests in `src/mcp/tools/model_telemetry.rs`
  - existing MCP server tests in `src/mcp/server.rs`
  - `tests/integration_phase12.rs`
  - existing telemetry-store unit tests in `src/agents/model_access.rs` and
    `src/agents/model_performance.rs` when helper functions are added there
- Developer docs:
  - `docs/modes/query-mode-spec.md` or a new focused doc such as
    `docs/mcp-model-telemetry.md`
  - `README.md` only if the public command/interface summary needs a short MCP
    analytics mention

Areas that must not change during Stage 8:

- implementation source files
- tests
- generated outputs
- runtime assets
- product specs for #682 or other issues
- technical specs for other issues

Areas expected not to change during Stage 10:

- provider routing, model ranking, model selection, or fallback behavior
- provider credential setup semantics, except read-only helper exposure
- model catalog refresh behavior from #675
- workflow metrics behavior from #610
- graph parser, graph validator, compiler, runtime, registry, and Studio
  behavior unrelated to discovering or consuming the MCP analytics tool output

## Technical Approach

### MCP Surface

Add three read-only MCP tools:

| Tool | Purpose |
| --- | --- |
| `model_access_summary` | Summarize user-level provider/model accessibility from model-access `current.json`, with optional bounded redacted access events. |
| `model_performance_summary` | Summarize workspace model-performance `aggregates.json`, with optional bounded redacted call events. |
| `model_telemetry_health` | Report source health for model-access and model-performance stores without returning aggregate rows or raw events. |

Register the tools in `McpServer::list_tools()` and dispatch them from
`dispatch_tool_call()` to `tools::model_telemetry`.

Each tool description must include:

- read-only/no-mutation wording;
- source stores read by the tool;
- default and maximum limits;
- explicit privacy statement that credential fingerprints, secrets, provider
  messages, prompts, completions, and raw provider error bodies are not returned
  by default;
- for raw-capable tools, wording that raw mode is explicit, bounded, and
  redacted.

Do not add MCP resource or prompt methods in this issue unless Stage 9 changes
the approved scope. A resources-first implementation is rejected for V1 because
the server does not currently expose resource discovery or reads, while the
product outcome can be satisfied through existing MCP tool mechanics.

### Shared Response Contract

All three tools return a JSON object before MCP text wrapping. The common fields
are:

```json
{
  "status": "success",
  "data_status": "present",
  "generated_at": "2026-06-12T00:00:00Z",
  "scope": "workspace_model_performance",
  "filters": {},
  "summary": {},
  "rows": [],
  "raw_events": null,
  "privacy": {
    "credential_fingerprints_returned": false,
    "secrets_returned": false,
    "provider_messages_returned": false,
    "raw_prompts_or_completions_returned": false,
    "notes": "Default analytics are aggregate, bounded, and redacted."
  },
  "warnings": []
}
```

Allowed `status` values:

- `success`: requested data was read and summarized.
- `partial`: some data was unavailable, stale, unreadable, truncated, or had
  invalid rows, but the response is usable.
- `error`: request validation failed or all requested source data was malformed
  or unreadable.

Allowed `data_status` values:

- `present`
- `empty`
- `absent`
- `stale`
- `partial`
- `malformed`

Use deterministic row ordering for stable tests:

1. provider
2. model
3. agent role, task type, complexity, scope, risk when present
4. latest timestamp descending as a final tiebreaker only when needed

`generated_at` is the only field expected to vary for identical stored data and
filters.

### Input Schema And Validation

Common inputs:

- `provider`: optional string.
- `model`: optional string.
- `stale_after_hours`: optional positive integer, default `168`, maximum
  `8760`.
- `limit`: optional positive integer for aggregate rows, default `25`, maximum
  `100`.
- `include_raw_events`: optional boolean, default `false`.

Performance-only filters:

- `agent_role`: optional string.
- `task_type`: optional string.
- `complexity`: optional string.
- `scope`: optional string.
- `risk`: optional string.

Validation rules:

- Reject unknown input fields through JSON Schema `additionalProperties: false`
  and handler-side validation.
- Reject wrong JSON types with a clear validation error.
- Reject `stale_after_hours` less than `1` or greater than `8760`.
- Reject `limit` less than `1`.
- Clamp aggregate `limit` to `100` only when the schema cannot reject it; prefer
  explicit validation errors for hand-written handler checks.
- If `include_raw_events` is `true`, require an explicit `limit` and reject a
  limit greater than `50`. This makes raw mode visibly intentional and proves
  the unbounded raw scenario.
- Do not silently ignore unsupported filters.

### Store Read Helpers

Do not change the behavior of existing routing-oriented loaders:

- `ModelAccessStore::load_db()`
- `ModelAccessStore::load_db_from_home()`
- `ModelPerformanceStore::load_db(workspace)`

Add analytics-oriented read helpers that preserve missing and malformed state:

- model access:
  - expose read-only path helpers for `current.json` and `events.jsonl`, or a
    single helper returning the model-access directory plus file names;
  - add a fallible current-store reader returning absent, empty, present,
    malformed, or unreadable state;
  - add a bounded event reader that reads at most the requested number of recent
    JSONL rows and reports malformed lines as warnings without returning raw
    file contents.
- model performance:
  - expose read-only path helpers for `aggregates.json` and `events.jsonl`, or a
    single helper returning the model-performance directory plus file names;
  - add a fallible aggregate-store reader returning absent, empty, present,
    malformed, or unreadable state;
  - add a bounded event reader using the existing `ModelCallEvent` type.

Helpers may live in `src/mcp/tools/model_telemetry.rs` if they are purely
presentation-layer readers, but path constants should not drift from the store
modules. Prefer store-module path helpers over duplicating private file names in
the MCP module.

Malformed JSON handling must not use the existing defaulting loaders for
analytics health. If a file exists and cannot be parsed, report
`data_status: malformed` or `partial` with a non-secret diagnostic. Do not
overwrite, delete, truncate, or reformat source files.

### Model Access Summary

`model_access_summary` reads the user-level model-access current store from the
active process home path.

Aggregate current records by `provider` and `model`, not by
`credential_fingerprint`.

Each row should include:

- `provider`
- `model`
- `record_count`
- `status_counts` with keys `accessible`, `denied`, `auth_failed`, and
  `unknown`
- `latest_status`
- `last_checked`
- `last_success`
- `is_stale`
- `stale_after_hours`

Rules:

- `latest_status` is the status of the newest record by `last_checked` within
  the provider/model group.
- `is_stale` is true when the newest `last_checked` is older than the normalized
  `stale_after_hours`.
- Do not return `credential_fingerprint`, fingerprint-derived identifiers,
  provider `message`, or secret values.
- Do not return `reason_code` by default. It may appear in raw mode only if
  still provider-neutral and not derived from a raw provider body.
- When the current store is absent, return `status: success`,
  `data_status: absent`, empty rows, and no writes.
- When the current store exists but contains no records, return
  `data_status: empty`.
- When records are stale, return rows with `is_stale: true`, set top-level
  `data_status` to `stale` when all returned rows are stale, otherwise
  `partial`, and include a warning that stale access evidence is not current
  access proof.

### Model Performance Summary

`model_performance_summary` reads workspace-level model-performance aggregates
from the active MCP workspace path.

Parse aggregate keys as:

```text
provider|model|agent_role|task_type|complexity|scope|risk
```

Each row should include:

- `provider`
- `model`
- `agent_role`
- `task_type`
- `complexity`
- `scope`
- `risk`
- `calls`
- `successes`
- `failures`
- `success_rate`
- `failure_rate`
- `parse_failures`
- `validation_failures`
- `retries`
- `ewma_latency_ms`
- `ewma_cost_usd`
- `last_updated`
- `is_stale`

Rules:

- Treat `*` in aggregate-key profile fields as `null` or `"*"` consistently;
  document the chosen representation in the tool output docs.
- Compute rates as `count / calls` when `calls > 0`; use `null` when `calls` is
  zero.
- Apply provider/model/task-profile filters after parsing aggregate keys.
- If an aggregate key does not contain exactly seven segments, skip that row,
  return `status: partial`, and include a non-secret warning naming only the
  row index or a redacted key summary.
- Do not expose token totals or template-version grouping from aggregate
  responses because the current aggregate schema does not store them.
- When the aggregate store is absent or empty, return empty analytics without
  creating the store and without querying any cloud service.

### Model Telemetry Health

`model_telemetry_health` inspects both source stores and returns source health
without aggregate rows unless a future product spec asks for richer diagnostics.

Return at least:

- `model_access`:
  - `scope`: `user_home_model_access`
  - `current_status`: absent, empty, present, stale, malformed, partial, or
    unreadable
  - `record_count`
  - `latest_checked`
  - `stale_record_count`
  - `events_status`
  - `event_log_size_bytes` when available
- `model_performance`:
  - `scope`: `workspace_model_performance`
  - `aggregate_status`
  - `aggregate_count`
  - `latest_updated`
  - `stale_aggregate_count`
  - `events_status`
  - `event_log_size_bytes` when available

The active workspace path may be returned only as the MCP server workspace path
and only if no home directory secrets or credential material are embedded. Do
not return user home paths by default; use scope labels instead.

### Raw Event Mode

V1 implements explicit raw mode for `model_access_summary` and
`model_performance_summary`.

Raw mode rules:

- `include_raw_events` must be `true`.
- `limit` must be present and `1..=50`.
- Event rows must be newest-first and capped at `limit`.
- Return a warning stating raw mode is explicit, bounded, and redacted.
- Never return credential fingerprints.
- Never return provider messages or raw provider error bodies.
- Never return prompts or completions.
- Never read files outside the model-access and model-performance stores.

Access raw event rows may include:

- `provider`
- `model`
- `status`
- `reason_code`
- `checked_at`

Access raw event rows must omit:

- `credential_fingerprint`
- `message`

Performance raw event rows may include:

- `timestamp`
- `provider`
- `model`
- `agent_role`
- `task_type`
- `complexity`
- `scope`
- `risk`
- `latency_ms`
- `first_token_latency_ms`
- `cost_usd`
- `tool_parse_success`
- `patch_count`
- `validation_error_count`
- `retries`
- `outcome`
- optional token counts only when the implementation documents that they are
  numeric usage metadata and not prompt/completion content

Performance raw event rows must omit:

- prompt text
- completion text
- raw provider bodies
- full validation error messages when they could include user content; return
  counts or sanitized codes instead

If bounded recent-event reading would require loading an unreasonably large
file into memory, implement a reverse-tail or bounded ring-buffer reader. Do
not keep an unbounded `Vec` of all event rows for large logs.

### No-Mutation Contract

Analytics calls must not:

- write to `~/.duumbi/knowledge/model-access/current.json`;
- write to `~/.duumbi/knowledge/model-access/events.jsonl`;
- write to workspace `.duumbi/knowledge/model-performance/aggregates.json`;
- write to workspace `.duumbi/knowledge/model-performance/events.jsonl`;
- edit `.duumbi/config.toml`, user provider config, credentials, or model
  catalog state;
- mutate graph files;
- create, update, archive, or execute intents;
- change provider/model routing decisions;
- trigger provider setup, provider probes, model calls, registry calls, or cloud
  analytics uploads.

Tests must snapshot source file contents and relevant metadata before and after
analytics calls. Missing stores must remain missing.

### Documentation

Add developer-facing documentation for:

- tool names and input schemas;
- local-only behavior;
- aggregate defaults;
- raw-mode requirements and maximum limits;
- stale evidence interpretation;
- privacy and redaction guarantees;
- no-routing/no-mutation behavior;
- examples of MCP `tools/call` payloads and responses.

The docs must distinguish recorded local evidence from universal provider
truth:

- `accessible` means DUUMBI recorded a successful probe under a local credential
  context.
- `denied` means DUUMBI recorded provider/subscription denial under a local
  credential context.
- success and failure rates are local historical evidence, not global model
  quality guarantees.
- stale data is informational and must not be treated as current routing
  evidence.

## Invariants

- MCP analytics are read-only and local-only.
- Existing telemetry stores remain the source of truth; no new persistent
  telemetry store is introduced.
- Existing routing loaders keep their current absent/malformed default behavior
  for routing callers unless a separate approved change modifies that contract.
- Analytics health uses fallible readers and must distinguish absent, empty,
  stale, malformed, partial, and unreadable stores.
- Default responses never include credential fingerprints, fingerprint-derived
  stable IDs, provider secret values, provider messages, raw provider error
  bodies, prompts, or completions.
- Raw event output is disabled by default, requires an explicit flag and limit,
  and is redacted.
- Analytics calls never make external provider, registry, cloud analytics, or
  Slack calls.
- Analytics calls never mutate provider config, credentials, model catalog,
  graph files, intents, telemetry files, or routing decisions.
- Large event logs are bounded and cannot be streamed wholesale by default.
- All public Rust items added for this issue have doc comments, avoid `.unwrap()`
  in library code, and follow repository error-handling conventions.
- CI tests use fixtures, temp directories, and mocks only. Live provider keys
  are manual evidence and never required in CI.

## BDD-To-Test Mapping

| Product BDD scenario | Required verification evidence |
| --- | --- |
| MCP client discovers model telemetry analytics | Unit test in `src/mcp/server.rs` asserts `tools/list` includes `model_access_summary`, `model_performance_summary`, and `model_telemetry_health`, and each description includes read-only wording and source-store wording. |
| MCP analytics does not mutate local state | Integration test in `tests/integration_phase12.rs` creates temp model-access, model-performance, graph, config, catalog, and intent files; calls all analytics tools; asserts watched files are byte-identical afterward and no missing watched file is created. |
| Model access summary groups statuses without credential fingerprints | Unit test in `src/mcp/tools/model_telemetry.rs` writes fixture `current.json` records across accessible, denied, auth-failed, and unknown statuses with credential fingerprints; calls `model_access_summary`; asserts status counts and absence of fingerprints, messages, and secret-like fixture values in serialized output. |
| Stale model access probes are visible | Unit test writes an access record older than 168 hours; calls default summary; asserts `is_stale: true`, stale warning, and stored status still returned. |
| Empty model access store returns an empty status | Unit or integration test with a temp HOME lacking `.duumbi/knowledge/model-access`; calls access summary; asserts `data_status` absent or empty, empty rows, and no directory/file creation. |
| Performance summary returns success and failure rates | Unit test writes fixture `aggregates.json`; calls `model_performance_summary`; asserts calls, successes, failures, rates, parse failures, validation failures, retries, EWMA latency, and EWMA cost. |
| Performance summary can filter by task profile | Unit test writes multiple aggregate keys; calls with `task_type: "create"` and `risk: "high"`; asserts only matching rows are returned and normalized filters are echoed. |
| Missing workspace performance store returns empty analytics | Integration test with temp workspace lacking `.duumbi/knowledge/model-performance`; calls performance summary; asserts absent/empty data, no file creation, and no network/provider call hooks are invoked. |
| Default analytics omit raw event rows | Unit test writes current, aggregate, and event fixtures; calls summaries without `include_raw_events`; asserts no `raw_events` rows and no credential fingerprints or provider messages in serialized output. |
| Explicit raw mode is bounded and warns the client | Unit test writes access and performance event logs; calls summaries with `include_raw_events: true` and `limit: 10`; asserts at most 10 newest redacted rows and a raw-mode warning. |
| Unbounded raw mode is rejected or clamped | Unit test calls raw mode without `limit`; expected result is an invalid-params style tool error or tool-level validation error. The preferred V1 behavior is rejection. |
| Malformed telemetry file returns a non-secret diagnostic | Unit test writes malformed performance `aggregates.json`; calls performance summary; asserts no panic, `data_status: malformed` or `status: error`, no overwrite/delete, and diagnostic omits raw file contents. |
| Analytics do not alter provider or model selection | Integration test snapshots provider config and model catalog state before and after analytics calls; optional unit test asserts the new analytics functions do not call `ModelAccessStore::record_report`, provider startup, factory provider construction, or catalog adoption APIs. |

Additional coverage:

- Unit tests for invalid filter types, unknown fields, limit bounds, stale
  threshold bounds, and malformed aggregate keys.
- Unit tests for deterministic sorting and deterministic response shape.
- Unit tests for no token totals or template-version grouping in default
  aggregate responses.
- Documentation check or review evidence that the developer docs include
  privacy, staleness, raw-mode, and no-routing statements.

## Live E2E Plan

Canonical interface: `duumbi mcp` over stdio JSON-RPC.

Automated CI path:

- Uses fixture-backed local telemetry stores only.
- Makes zero external LLM calls.
- Runs through `tools/list` and `tools/call` so it verifies the same MCP surface
  that users and MCP clients consume.

Manual live provider-backed path:

- Purpose: prove that telemetry created by an existing DUUMBI provider probe can
  be queried by the new MCP analytics surface.
- Required credentials: one existing provider API key or subscription token
  already supported by DUUMBI, preferably the lowest-cost available provider
  for the maintainer environment.
- Recommended environment: disposable `HOME` and disposable workspace so live
  provider setup does not alter the maintainer's normal DUUMBI config.
- Expected external LLM calls: depends on provider catalog size because the
  existing probe path calls one request per provider/model entry. Stage 10 must
  inspect the chosen provider's current catalog entry count before running.
- Default budget: at most 2 live provider calls and USD $0.05 without explicit
  approval. If the existing provider probe would exceed that, skip the live
  seed path and record why it is over budget.

Manual command outline:

1. Build or reuse the debug binary:

   ```text
   cargo build
   ```

2. Create a disposable workspace and home directory.

3. Seed model-access telemetry through an existing provider setup/probe path
   only if the chosen provider is within the call and cost budget. The analytics
   implementation must not introduce a new provider-call path for this.

4. Start the MCP server in that workspace:

   ```text
   target/debug/duumbi mcp
   ```

5. Send JSON-RPC requests for:

   ```json
   {"jsonrpc":"2.0","id":1,"method":"tools/list"}
   ```

   ```json
   {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"model_access_summary","arguments":{"limit":10}}}
   ```

   ```json
   {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"model_telemetry_health","arguments":{}}}
   ```

6. Pass criteria:

   - analytics tool discovery succeeds;
   - model-access summary reports the probe evidence or a safe absent/empty
     status if the live seed was skipped;
   - no credential fingerprint, secret, provider message, raw provider body,
     prompt, or completion appears in MCP output;
   - model-performance analytics can be queried from fixture or existing local
     workspace evidence without making a live provider call;
   - source telemetry files are unchanged by analytics calls.

7. Capture evidence in the implementation PR:

   - provider selected, if any;
   - exact external LLM call count or reason live seed was skipped;
   - estimated live cost;
   - command transcript with secrets redacted;
   - MCP responses with privacy-sensitive fields absent.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 4 source modules plus directly associated
  tests/docs.
- Expected command budget per cycle:
  - `cargo fmt --check`
  - focused `cargo test` for edited modules or integration tests
  - `cargo clippy --all-targets -- -D warnings` when public APIs or shared
    modules change
- Full pre-review command budget:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --all`
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies or irreversible
  operations, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Issue-specific live-provider budget without approval: maximum 2 live provider
  calls and USD $0.05 total.
- CI external network budget: zero live provider calls and zero provider cost.
- Autonomous batch cap: 3 low-risk cycles.
- When to stop and ask for human guidance:
  - implementing MCP resources/prompts becomes necessary instead of tools;
  - analytics requires changing routing semantics;
  - fallible telemetry readers would break existing routing callers;
  - raw event redaction cannot be made safe without dropping required fields;
  - live provider evidence would exceed the issue-specific live-provider budget;
  - a new dependency is proposed for tailing, pagination, or JSON schema
    validation;
  - product behavior conflicts with the approved privacy/no-mutation contract.

## Task Breakdown

1. MCP contract and module skeleton
   - Add `src/mcp/tools/model_telemetry.rs`.
   - Add `pub mod model_telemetry` to `src/mcp/tools/mod.rs`.
   - Register and dispatch the three MCP tools in `src/mcp/server.rs`.
   - Add schema and discovery tests.

2. Shared validation and response model
   - Implement query input parsing with strict unknown-field and type checks.
   - Implement normalized filters, limits, stale threshold handling, warnings,
     privacy metadata, and deterministic sorting.
   - Add validation tests for bad params and bounds.

3. Fallible telemetry readers
   - Add read-only path/fallible-load helpers for model-access current store and
     events.
   - Add read-only path/fallible-load helpers for model-performance aggregates
     and events.
   - Preserve existing `load_db` behavior.
   - Add absent, empty, malformed, unreadable, and malformed-line tests.

4. Model access analytics
   - Aggregate records by provider/model.
   - Compute status counts, latest status, freshness, and stale warnings.
   - Apply provider/model filters and row limits.
   - Add privacy and staleness tests.

5. Model performance analytics
   - Parse aggregate keys into provider/model/task profile dimensions.
   - Compute rates and freshness.
   - Apply provider/model/task-profile filters and row limits.
   - Add malformed-key, no-token-total, no-template-version-grouping, and
     filter tests.

6. Raw event mode
   - Add bounded redacted access event output.
   - Add bounded redacted performance event output.
   - Reject raw mode without explicit `limit`.
   - Add no-message/no-fingerprint/no-prompt/no-completion tests.

7. Health tool
   - Implement combined source health summary.
   - Add source status, count, stale, malformed, and event-log metadata tests.

8. No-mutation integration tests
   - Add fixture-backed MCP `tools/call` tests in `tests/integration_phase12.rs`.
   - Snapshot model-access, model-performance, provider config, credentials,
     model catalog, graph, and intent paths before and after calls.
   - Assert analytics calls create no missing stores.

9. Documentation and final verification
   - Add or update MCP analytics docs.
   - Run focused tests, then full formatting, clippy, and test suite.
   - Capture optional manual MCP smoke and optional live provider seed evidence
     within the live-provider budget.

## Verification Plan

Required local verification before implementation PR review:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`
- Focused MCP server tests covering discovery and dispatch.
- Focused unit tests for `src/mcp/tools/model_telemetry.rs`.
- Focused unit tests for new fallible telemetry reader helpers.
- Integration tests in `tests/integration_phase12.rs` for MCP `tools/call` and
  no-mutation behavior.
- Documentation review confirming the new MCP interface docs describe local-only
  scope, aggregate defaults, raw-mode limits, privacy redaction, stale data
  interpretation, and no routing changes.

Manual verification before implementation PR review:

- Run `duumbi mcp` against a fixture-backed workspace.
- Call `tools/list`.
- Call `model_access_summary`, `model_performance_summary`, and
  `model_telemetry_health`.
- Verify responses are bounded, parseable JSON inside MCP text content, and do
  not include forbidden sensitive fields.
- Optional live provider seed path only when within the issue-specific
  live-provider budget.

## Completion Criteria

The implementation is complete only when:

- `tools/list` advertises all three model telemetry analytics tools.
- All three tools can be called through `tools/call`.
- Default analytics are aggregate, bounded, deterministic except
  `generated_at`, and credential-safe.
- Model-access analytics return provider/model status counts, latest status,
  freshness, stale warnings, and empty/absent status correctly.
- Model-performance analytics return calls, success/failure rates, parse
  failures, validation failures, retries, EWMA latency, EWMA cost, task-profile
  dimensions, filters, freshness, and empty/absent status correctly.
- Health analytics distinguish absent, empty, present, stale, partial,
  malformed, and unreadable states where those states can be fixture-tested.
- Raw event mode is explicit, bounded, newest-first, redacted, and disabled by
  default.
- Unbounded raw mode is rejected.
- Malformed telemetry files never panic and are not overwritten or deleted.
- No analytics call mutates telemetry, provider config, credentials, model
  catalog, graph files, intents, or routing decisions.
- No analytics call makes external provider, registry, cloud analytics, or Slack
  calls.
- Developer docs explain usage and privacy boundaries.
- Required tests and checks in the verification plan pass, or any skipped manual
  live provider evidence is explicitly justified by missing credentials or
  budget limits.

## Failure And Escalation

- If malformed-file detection conflicts with existing `load_db` callers, keep
  existing loaders unchanged and add separate analytics-only fallible readers.
- If a raw event field might expose user content, provider payloads, credentials,
  or unreviewed sensitive detail, omit or count it and add a warning rather than
  returning it.
- If event logs are too large for a simple reader, implement a bounded reader or
  stop for a design decision; do not load unbounded logs into memory.
- If MCP resources or prompts become necessary to satisfy discovery, stop for a
  Stage 9/product-scope decision because V1 intentionally uses tools.
- If tests require live credentials, replace them with fixtures and keep live
  provider evidence manual only.
- If a new dependency is proposed, stop for approval unless it is already in the
  dependency graph and clearly lower risk than a small local implementation.
- If GitHub Actions or local full-suite checks fail, implementation agents must
  triage whether the failure is caused by this change, patch within scope, and
  rerun focused checks before claiming completion.
- If product requirements conflict, route back to Stage 9 or the GitHub issue
  with a concrete blocking question instead of guessing.

## Open Questions

None blocking for Stage 10 implementation.

Non-blocking follow-up candidates:

- Add MCP resources or resource templates for static/default analytics summaries
  after the tool-based V1 is accepted.
- Add MCP prompts for repeatable narrative model telemetry reports after the
  structured analytics surface exists.
- Add an optional report/export command in a separate product spec.
- Let future routing/advisor work consume these analytics only after a separate
  approved product spec changes routing behavior.
