# DUUMBI-682: Expose Model Usage Telemetry Analytics Via MCP

## Summary

DUUMBI should expose its existing local model-access and model-performance
knowledge through a read-only MCP analytics surface. The surface lets developers
and analysis agents answer practical provider/model questions without manually
opening JSON files:

- which provider/model pairs are currently accessible, denied, auth-failed,
  unknown, or stale;
- which provider/model/task profiles succeed or fail most often;
- which provider/model/task profiles are slow, costly, retry-heavy, or prone to
  parse or validation failures;
- whether the available evidence is fresh enough to trust.

The product slice is local-only and aggregate-first. It reuses the existing
telemetry stores, does not create a new telemetry store, does not change routing,
does not expose secrets or credential fingerprints, and does not connect to a
cloud analytics service.

Spec-only workflow note: any PR carrying this product spec is specification-only
and must leave execution issue #682 open for later workflow stages.

## Problem

DUUMBI already records useful local model evidence, but that evidence is hard to
ask questions about:

- `~/.duumbi/knowledge/model-access/current.json` and `events.jsonl` record
  provider/model accessibility by non-reversible credential fingerprint.
- workspace `.duumbi/knowledge/model-performance/aggregates.json` and
  `events.jsonl` record model-call outcomes, token estimates, latency, cost,
  retry counts, parse failures, validation failures, and task profile
  dimensions.

Today that knowledge mostly helps internal routing and provider setup code. A
developer or external MCP client cannot ask DUUMBI for a safe, structured answer
about model health, cost, latency, or stale probes. This creates manual log
inspection, weak evidence for provider/model decisions, and duplicated analysis
outside DUUMBI.

The risk is not lack of raw data. The risk is exposing the wrong slice of data:
credential fingerprints, provider error details, exact event history, or future
prompt/completion fields could leak sensitive operational context if surfaced
without a privacy contract.

## Outcome

When this work is done:

- MCP clients can discover and call/read a local model telemetry analytics
  surface from DUUMBI.
- The default response is aggregate, bounded, and credential-safe.
- Model-access analytics show provider/model accessibility counts, latest
  status, and freshness/staleness without returning credential fingerprints.
- Model-performance analytics show call counts, success/failure rates, failure
  classes, retry counts, latency EWMA, cost EWMA, and available token totals by
  provider/model/task profile.
- Empty, absent, stale, or partially unreadable stores return explicit status
  metadata instead of panicking or pretending the data is fresh.
- Raw event inspection, if implemented in V1, is explicit, bounded, redacted, and
  disabled by default.
- No MCP analytics call mutates telemetry, provider config, model catalog,
  routing behavior, workspace graph, or credentials.

## Scope

### In Scope

- A read-only MCP analytics surface for existing local model telemetry.
- Model-access aggregate analytics from the user-level model-access store.
- Model-performance aggregate analytics from the active workspace-level
  model-performance store.
- Freshness metadata for both stores, including stale probe detection.
- Query filters for provider, model, task profile dimensions, freshness window,
  and result limits where useful.
- Privacy-aware redaction for credential fingerprints, provider messages, raw
  error bodies, prompts, completions, and fine-grained event history.
- Bounded raw event inspection only behind an explicit request flag if the V1
  implementation chooses to expose it.
- MCP discovery metadata that clearly marks the analytics surface read-only.
- Developer documentation for the new MCP analytics interface.
- Unit and integration tests for successful analytics, empty stores, redaction,
  staleness, bounds, malformed data, and no-mutation guarantees.

### Explicitly Out Of Scope

- Automatic routing, model ranking, provider ordering, or catalog updates based
  on analytics.
- Provider credential setup, credential mutation, credential export, or API key
  inspection.
- Cloud telemetry ingestion, centralized reporting, dashboards, or scheduled
  uploads.
- A new persistent telemetry store.
- Reading raw prompts, completions, or hidden provider payloads.
- Exposing raw credential fingerprints, even though the stored fingerprints are
  non-reversible.
- Full provider error bodies or unsanitized provider messages.
- Automatic alerts, Slack notifications, or periodic report generation.
- Product spec approval, technical spec drafting, implementation code, or Ralph
  cycles.

## Constraints And Assumptions

Facts:

- Issue #682 is open, labeled `accepted` and `needs-spec`, and has a Stage 5
  Human Acceptance Decision comment dated 2026-06-12 with `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- The issue has no visible GitHub Project item through the available issue
  metadata at Stage 6 intake.
- The source issue originated from
  `Duumbi/00 Inbox (ToProcess)/2026-06-06 - MCP Model Telemetry Analytics.md`.
- That source note says the feature should be separate from provider catalog
  refresh issue #675 and workflow metrics issue #610.
- GitHub search at Stage 6 intake found issue #682 as the active matching issue
  and did not find an existing #682 spec PR.
- DUUMBI currently exposes an MCP server with `tools/list` and `tools/call`
  support for graph, build, dependency, and intent tools.
- The current MCP server does not expose model telemetry analytics.
- `src/agents/model_access.rs` stores model-access current records and
  append-only events under `~/.duumbi/knowledge/model-access`.
- Model-access records are keyed by non-reversible credential fingerprint and
  include provider, model, status, reason code, sanitized message, probe
  version, `last_checked`, and `last_success`.
- `src/agents/model_performance.rs` stores model-performance events and rolling
  aggregates under workspace `.duumbi/knowledge/model-performance`.
- Model-performance events can include provider, model, agent role, template
  version, task type, complexity, scope, risk, prompt tokens, completion tokens,
  reasoning tokens, latency, first-token latency, estimated cost, parse success,
  patch count, validation errors, retries, and final outcome.
- Model-performance aggregates are keyed by provider/model/agent/task profile
  dimensions and include calls, successes, failures, EWMA latency, EWMA cost,
  parse failures, validation failures, retries, and `last_updated`.
- DUUMBI's PRD emphasizes read-only question answering over source, graph,
  sessions, knowledge, and evidence before mutation begins.
- The active AI review policy requires Codex self-review and actual Copilot
  review evidence for file-based Stage 6 product spec PRs. Greptile is
  manual-only and was not requested for this product spec.

Assumptions:

- V1 should prefer the smallest MCP protocol shape that gives reliable,
  discoverable, parameterized analytics. If Stage 8 determines that DUUMBI's MCP
  resource support should be added first, resources are acceptable as long as
  the behavior contract in this product spec is preserved.
- Aggregate analytics are safe to expose by default when credential
  fingerprints, raw provider messages, raw event rows, and sensitive future
  fields are omitted.
- A default stale threshold of 168 hours is reasonable for V1 if no existing
  provider freshness setting is available. The threshold should be overrideable
  per query.
- The active workspace root defines the model-performance scope. The active
  user's DUUMBI home defines the model-access scope.
- Exact implementation names may be finalized in the technical spec, but the
  MCP surface must be discoverable and clearly read-only.

## Decisions

- **Decision:** Use a file-based product spec.
  **Evidence:** The issue is cross-module, user-visible, MCP-facing, and
  privacy-sensitive. It needs durable review history and implementation context.

- **Decision:** Treat issue #682 as separate from provider catalog refresh.
  **Evidence:** Issue #675 and `specs/DUUMBI-675/PRODUCT.md` explicitly classify
  MCP model telemetry analytics as a separate follow-up, while #675 concerns
  catalog publication, provider set updates, and opt-in catalog adoption.

- **Decision:** Treat issue #682 as separate from workflow metrics issue #610.
  **Evidence:** Issue #610 concerns GitHub Actions workflow metrics artifacts.
  Issue #682 concerns local DUUMBI model-access and model-performance knowledge
  queried through MCP.

- **Decision:** V1 analytics are read-only and local-only.
  **Evidence:** The accepted issue, source inbox note, PRD query-first principle,
  and Stage 5 acceptance all preserve user control and exclude routing changes,
  cloud telemetry ingestion, and external data sharing.

- **Decision:** Default analytics are aggregate-first.
  **Evidence:** The source note calls out privacy leakage risks and recommends
  aggregate questions as the starting point.

- **Decision:** Raw event inspection is optional, explicit, bounded, and
  redacted.
  **Evidence:** The source note permits raw events only behind explicit flags.
  The product risk is privacy leakage from exact event history, credential
  fingerprints, and provider error details.

- **Decision:** No MCP prompt surface is required in V1.
  **Evidence:** The user outcome is structured analytics. Repeatable narrative
  report prompts may be useful later, but they are not needed to answer the
  accepted first questions.

## Behavior

### MCP Discovery

The DUUMBI MCP server exposes a discoverable model telemetry analytics surface.
The surface may be implemented as MCP tools, MCP resources/resource templates,
or a combination, but it must be visible through the appropriate MCP discovery
method and must identify itself as read-only.

The discovery metadata includes:

- a short description of the analytics scope;
- accepted input filters or resource URI parameters;
- explicit read-only/no-mutation wording;
- default limits and maximum limits;
- response fields or schema enough for MCP clients to consume results safely.

If DUUMBI exposes both static resources and parameterized tools, static resources
should represent default summaries, while tools or resource templates should
serve filtered queries.

### Analytics Categories

V1 supports at least these categories:

- `model_access_summary`
  - summarizes provider/model accessibility from the user-level access store;
  - returns counts by status: accessible, denied, auth-failed, unknown;
  - returns latest status and freshness per provider/model where available;
  - flags records older than the stale threshold;
  - never returns credential fingerprints.

- `model_performance_summary`
  - summarizes workspace model-performance aggregates;
  - returns calls, successes, failures, success rate, failure rate, parse
    failures, validation failures, retry totals, EWMA latency, EWMA cost, and
    last update metadata;
  - groups by provider/model and optional task profile dimensions: agent role,
    template version, task type, complexity, scope, and risk;
  - supports filters and result limits.

- `model_telemetry_health`
  - reports whether each source store is absent, present, empty, stale,
    partially unreadable, or malformed;
  - reports the active workspace path only when safe and useful;
  - reports source freshness and row/aggregate counts without secrets.

The exact names can change in the technical spec if the MCP naming convention
requires it, but these categories are required product capabilities.

### Inputs

Common query inputs:

- `provider`: optional provider name filter.
- `model`: optional model name filter.
- `task_type`: optional task type filter for performance analytics.
- `agent_role`: optional agent role filter for performance analytics.
- `complexity`, `scope`, `risk`: optional task profile filters for performance
  analytics.
- `stale_after_hours`: optional freshness threshold; defaults to 168 hours.
- `limit`: optional row limit; default and maximum must be documented.
- `include_raw_events`: optional boolean; defaults to false.

Inputs must reject or clamp unbounded requests. Unsupported filters should return
clear validation errors, not broad raw dumps.

### Outputs

Default outputs include:

- `status`: success, partial, or error.
- `data_status`: present, empty, absent, stale, partial, or malformed.
- `generated_at`: the time the analytics response was generated.
- `scope`: source scope such as `user_home_model_access` or
  `workspace_model_performance`.
- `filters`: normalized filters applied.
- `rows`: bounded aggregate rows.
- `summary`: top-level counts and freshness metadata.
- `privacy`: a short statement confirming that credential fingerprints, secrets,
  raw prompts, completions, and provider error bodies were not returned.
- `warnings`: non-secret warnings about missing, stale, unreadable, or truncated
  data.

Responses should be deterministic for the same stored data and filters except
for `generated_at`.

### Privacy And Redaction

The analytics surface must not return:

- provider secret values;
- credential fingerprints or fingerprint-derived stable identifiers;
- raw provider error bodies;
- provider messages by default, even when stored messages are sanitized;
- raw prompts or completions;
- file contents outside the existing model-access and model-performance stores;
- exact raw event streams unless explicitly requested and bounded.

If raw event inspection is implemented:

- `include_raw_events` must be true;
- `limit` must be required or clamped to a documented maximum;
- event rows must redact credential fingerprints;
- event rows must omit provider messages by default;
- exact timestamps may be returned only when they are necessary for freshness or
  recent-trend analysis;
- the response must include a visible raw-mode warning.

### Empty, Missing, Stale, And Malformed Stores

When no model-access data exists, the access summary returns an empty result with
`data_status: absent` or `empty`.

When no model-performance data exists for the active workspace, the performance
summary returns an empty result with `data_status: absent` or `empty`.

When records are older than the stale threshold, analytics still return the
available data but include stale flags and warnings.

When a store cannot be parsed or read safely, analytics return a partial/error
response with a non-secret diagnostic. The MCP server must not panic, write a
replacement file, delete the source file, or silently report the store as fresh.

### No Mutation

Calling or reading the analytics surface must not:

- write to `~/.duumbi/knowledge/model-access`;
- write to workspace `.duumbi/knowledge/model-performance`;
- edit `.duumbi/config.toml`, user config, provider credentials, or model
  catalog state;
- mutate the semantic graph;
- create, update, or execute intents;
- change provider/model routing decisions;
- trigger live provider probes or external API calls.

Analytics may read the relevant local files and may compute in-memory summaries.

### Performance And Bounds

The default path should read aggregate files before append-only event logs when
aggregate files can answer the request. Event-log inspection is allowed only
when explicitly requested or needed to compute a requested bounded recent trend.

Large logs must be bounded through limits, pagination, or a documented maximum.
The response should indicate truncation when results are limited.

### User-Facing Interpretation

Analytics must distinguish facts from inferences in response fields or docs:

- `accessible` means DUUMBI has a recorded successful probe for the
  provider/model under some current stored credential context, not that every
  user credential can access it.
- `denied` means DUUMBI recorded provider/subscription denial for a probed
  provider/model, not that the model is universally unavailable.
- performance success/failure rates are historical local evidence, not a global
  quality guarantee.
- stale data should be shown as stale, not treated as current routing evidence.

## BDD Scenarios

Feature: Read-only MCP model telemetry analytics

Rule: The analytics surface is discoverable and read-only

Scenario: MCP client discovers model telemetry analytics
Given the DUUMBI MCP server is running for a workspace
When an MCP client asks for available DUUMBI MCP capabilities
Then the response includes a model telemetry analytics surface
And the surface description says it is read-only
And the surface description identifies the model-access and model-performance
sources it can summarize

Scenario: MCP analytics does not mutate local state
Given model-access and model-performance files exist
And their file contents are captured before the request
When an MCP client requests model telemetry analytics
Then the response contains analytics data or a non-secret diagnostic
And the model-access files are unchanged
And the model-performance files are unchanged
And provider config, credentials, model catalog state, graph files, and intent
files are unchanged

Rule: Model access analytics are aggregate and credential-safe

Scenario: Model access summary groups statuses without credential fingerprints
Given the model-access current store contains records for provider "minimax"
And the records include accessible, denied, auth-failed, and unknown statuses
When an MCP client requests the model access summary
Then the response groups counts by provider, model, and status
And the response includes latest freshness metadata
And the response does not include credential fingerprints
And the response does not include provider secret values

Scenario: Stale model access probes are visible
Given the model-access current store contains a model last checked more than 168
hours ago
When an MCP client requests the model access summary with the default freshness
threshold
Then the response marks that provider/model evidence as stale
And the response still returns the stored status
And the response warns that stale evidence should not be treated as current
access proof

Scenario: Empty model access store returns an empty status
Given the user-level model-access store does not exist
When an MCP client requests the model access summary
Then the response status is success or partial
And the data status is absent or empty
And the rows are empty
And the response does not create the missing store

Rule: Model performance analytics summarize workspace evidence

Scenario: Performance summary returns success and failure rates
Given the workspace model-performance aggregate store contains calls for
provider "anthropic" and model "claude-sonnet"
When an MCP client requests the model performance summary
Then the response includes call count, success count, failure count, success
rate, and failure rate
And the response includes parse failure and validation failure counts
And the response includes retry totals when available
And the response includes EWMA latency and EWMA cost when available

Scenario: Performance summary can filter by task profile
Given the workspace model-performance aggregate store contains multiple task
types and risk levels
When an MCP client requests model performance summary filtered to task type
"create" and risk "high"
Then the response includes only aggregate rows matching those filters
And the normalized filters are echoed in the response
And the response does not expose unrelated raw events

Scenario: Missing workspace performance store returns empty analytics
Given the active workspace has no `.duumbi/knowledge/model-performance` store
When an MCP client requests the model performance summary
Then the response reports absent or empty performance data
And the response does not create the missing store
And the response does not query any cloud service

Rule: Raw event access is explicit, bounded, and redacted

Scenario: Default analytics omit raw event rows
Given model-access and model-performance event logs exist
When an MCP client requests analytics without `include_raw_events`
Then the response contains aggregate rows only
And raw event rows are absent
And credential fingerprints and provider messages are absent

Scenario: Explicit raw mode is bounded and warns the client
Given model-access and model-performance event logs exist
When an MCP client requests raw events with `include_raw_events` set to true and
limit 10
Then the response returns at most 10 redacted event rows
And credential fingerprints are omitted
And provider messages are omitted unless a later approved contract permits a
sanitized field
And the response includes a warning that raw mode is explicit and bounded

Scenario: Unbounded raw mode is rejected or clamped
Given model-access or model-performance event logs contain many rows
When an MCP client requests raw events without a limit
Then the request is rejected with a validation error or clamped to a documented
maximum
And the response does not stream the entire event log by default

Rule: Bad telemetry data fails safely

Scenario: Malformed telemetry file returns a non-secret diagnostic
Given the model-performance aggregate file contains malformed JSON
When an MCP client requests the model performance summary
Then the MCP server does not panic
And the response reports malformed or partial data
And the response does not overwrite or delete the malformed file
And the diagnostic does not include secrets or raw file contents

Rule: Analytics do not change routing

Scenario: Analytics do not alter provider or model selection
Given DUUMBI has existing provider configuration and model catalog state
When an MCP client requests model telemetry analytics
Then provider order is unchanged
And explicit model overrides are unchanged
And the model catalog state is unchanged
And the response does not claim that routing has been updated

## Tasks

- Define the MCP analytics surface contract and naming.
- Add read-only model-access aggregation behavior over the existing access
  store.
- Add read-only model-performance aggregation behavior over the existing
  performance aggregate store.
- Add optional, bounded, redacted event inspection if accepted for V1.
- Add freshness/staleness handling and source health metadata.
- Add input validation, result limits, and truncation reporting.
- Add privacy redaction tests that prove credential fingerprints and sensitive
  fields are not returned.
- Add no-mutation tests that compare relevant source files before and after MCP
  analytics calls.
- Add malformed/missing store tests.
- Add MCP discovery and call/read integration tests.
- Add developer documentation for using the analytics surface and interpreting
  results.
- Keep routing, provider setup, catalog refresh, cloud reporting, and automatic
  scheduling out of this issue.

Independent work streams after approval:

- MCP surface/discovery contract.
- Model-access summary implementation and tests.
- Model-performance summary implementation and tests.
- Privacy/redaction/no-mutation test suite.
- Documentation.

## Checks

Stage 8 and implementation should define exact commands, but the work is not
complete unless these checks pass or are explicitly replaced with equivalent
evidence:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`
- Focused tests for the new MCP analytics modules and server dispatch.
- Focused tests for malformed model-access and model-performance files.
- Focused tests proving missing stores return empty/absent status without
  writes.
- Focused tests proving analytics calls do not mutate:
  - `~/.duumbi/knowledge/model-access/current.json`
  - `~/.duumbi/knowledge/model-access/events.jsonl`
  - workspace `.duumbi/knowledge/model-performance/aggregates.json`
  - workspace `.duumbi/knowledge/model-performance/events.jsonl`
  - provider config and credential files
  - graph and intent files
- Focused tests proving credential fingerprints never appear in default or raw
  analytics output.
- Focused tests proving raw event access is disabled by default and bounded when
  enabled.
- Manual MCP smoke path:
  - start `duumbi mcp` in a workspace;
  - discover the analytics surface;
  - request model-access summary with absent or fixture-backed access data;
  - request model-performance summary with absent or fixture-backed performance
    data;
  - verify JSON responses are bounded and read-only.
- Documentation review verifying the docs explain:
  - local-only scope;
  - aggregate default behavior;
  - raw-mode limitations;
  - privacy redaction;
  - stale data interpretation;
  - no routing changes.
- File-based spec gate evidence before Stage 7:
  - Codex self-review has no blocking finding;
  - actual non-dismissed Copilot review evidence exists;
  - checks are green, neutral, skipped, or explicitly not applicable;
  - review threads are resolved;
  - Greptile was not invoked unless a developer explicitly requested manual deep
    review.

## Open Questions

None blocking for Stage 6.

Follow-up candidates that should not block V1:

- Whether a later report/export command should generate periodic model analytics
  artifacts.
- Whether future model capability advisor work should consume these analytics
  after a separate routing/advisor product spec accepts that behavior.
- Whether MCP prompts should provide reusable natural-language report templates
  after the structured analytics surface exists.
- Whether opt-in cloud sync or community aggregate model evidence belongs in a
  later registry or account-backed product track.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/682
- Stage 5 acceptance comment:
  https://github.com/hgahub/duumbi/issues/682#issuecomment-4695382423
- Source intake note:
  `Duumbi/00 Inbox (ToProcess)/2026-06-06 - MCP Model Telemetry Analytics.md`
- Active PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Agentic development map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Development intake to delivery workflow:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Development Intake to Delivery Workflow.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- MCP resources and prompts note:
  `Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/MCP Resources and Prompts.md`
- Future development roadmap:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Future Development Roadmap Map.md`
- Adjacent provider catalog issue:
  https://github.com/hgahub/duumbi/issues/675
- Adjacent provider catalog product spec:
  `specs/DUUMBI-675/PRODUCT.md`
- Related workflow metrics issue:
  https://github.com/hgahub/duumbi/issues/610
- Architecture reference: `docs/architecture.md`
- Query mode specification: `docs/modes/query-mode-spec.md`
- AI code review policy: `docs/automation/code-review-policy.md`
- MCP server implementation: `src/mcp/server.rs`
- MCP tool modules: `src/mcp/tools/mod.rs`
- Model access store: `src/agents/model_access.rs`
- Model performance store: `src/agents/model_performance.rs`
- Provider setup and model-access tests: `src/cli/app.rs`
- Session usage stats: `src/session/mod.rs`
- Current Stage 6 intake duplicate/context checks:
  - GitHub issue search for model telemetry analytics found #682 as the active
    matching issue.
  - GitHub PR search did not find an existing #682 product spec PR.
