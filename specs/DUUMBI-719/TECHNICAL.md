# DUUMBI-719: Agent Substrate - MCP As First-Class Interface - Technical Specification

Related to #719. This is a technical-specification artifact only. The
execution issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 implementation review, and Stage 12 closure.

## Implementation Objective

Implement the approved behavior in `specs/DUUMBI-719/PRODUCT.md` by turning
DUUMBI MCP into a complete, documented, testable local interface for external
coding agents.

The finished implementation must:

- Publish an MCP workflow capability audit for init, query, intent, patch,
  build, run, evidence, and approval.
- Expose a coherent agent-facing MCP capability/status surface.
- Add conversational Query mode access through MCP while preserving read-only
  behavior.
- Replace prose-only MCP failures with a stable structured error envelope.
- Make core workflow steps currently represented by stubs either functional
  through MCP or explicitly structured as unavailable with a durable reason.
- Add an approval path for write-capable external-agent requests.
- Record MCP session/action/evidence enough for external agents and reviewers
  to inspect what happened.
- Add agent-facing docs plus a Claude Code skill/plugin source artifact or
  setup package.
- Add an MCP-only benchmark for the flagship HTTP + SQLite + JSON example and a
  comparable raw Rust baseline report.
- Keep all implementation inside the accepted #719 product scope.

Do not add implementation code during this Stage 8 spec PR.

## Agent Audience

- Codex App implementation agents running bounded Stage 10 Ralph cycles.
- Codex Cloud agents only when a human routes longer CI, docs, or benchmark
  work there.
- Codex CLI agents used for local MCP protocol investigation.
- Specialized reviewer agents checking MCP schema compatibility, approval
  safety, structured error coverage, benchmark honesty, and docs accuracy.
- Tester agents validating unit, integration, MCP protocol, TUI/Studio parity,
  and live E2E evidence.

## Source Context

- Product spec: `specs/DUUMBI-719/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/719
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/719#issuecomment-4727796012
- Processed inbox source:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-06-12 - Agent Substrate MCP First-Class.md`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active agentic runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Agentic development map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Query mode contract: `docs/modes/query-mode-spec.md`
- Query MCP task: `docs/modes/implementation-tasks.md`
- Rewrite engine contract: `docs/rewrite-engine.md`
- Workflow/review policy:
  - `docs/automation/agentic-development-orchestration.md`
  - `docs/automation/code-review-policy.md`

Relevant current source:

- MCP server and tools:
  - `src/mcp/mod.rs`
  - `src/mcp/server.rs`
  - `src/mcp/tools/mod.rs`
  - `src/mcp/tools/graph.rs`
  - `src/mcp/tools/build.rs`
  - `src/mcp/tools/deps.rs`
  - `src/mcp/tools/intent.rs`
  - `src/mcp/tools/rewrite.rs`
  - `src/mcp/tools/model_telemetry.rs`
  - `src/mcp/client/mod.rs`
  - `src/mcp/client/config.rs`
- Query and interaction:
  - `src/query/engine.rs`
  - `src/query/context.rs`
  - `src/query/sources.rs`
  - `src/query/prompt.rs`
  - `src/interaction/mod.rs`
  - `src/interaction/router.rs`
- CLI/backend behavior to reuse:
  - `src/main.rs`
  - `src/cli/mod.rs`
  - `src/cli/init.rs`
  - `src/cli/commands.rs`
  - `src/cli/rewrite.rs`
  - `src/cli/deps.rs`
  - `src/cli/repl.rs`
  - `src/workspace.rs`
  - `src/workflow.rs`
  - `src/intent/create.rs`
  - `src/intent/execute.rs`
  - `src/intent/review.rs`
  - `src/intent/status.rs`
- State, evidence, and errors:
  - `src/session/mod.rs`
  - `src/errors.rs`
  - `src/telemetry/mod.rs`
  - `src/rewrite/evidence.rs`
  - `src/properties/evidence.rs`
  - `src/agents/model_performance.rs`
  - `src/agents/model_access.rs`
- Studio/TUI surfaces:
  - `crates/duumbi-studio/src/ws.rs`
  - `crates/duumbi-studio/src/app.rs`
  - `crates/duumbi-studio/src/script/studio.js`
  - `src/cli/app.rs`
  - `src/cli/mode.rs`
- Benchmark and examples:
  - `src/bench/runner.rs`
  - `src/bench/report.rs`
  - `src/bench/showcases.rs`
  - `examples/flagship-http-sqlite-json/README.md`
  - `tests/integration_duumbi688_flagship_example.rs`
  - `specs/DUUMBI-689/TECHNICAL.md`
- Automation:
  - `.github/workflows/spec-ai-gate.yml`
  - `.github/workflows/stage-approval.yml`
  - `.github/workflows/ready-for-build-handoff.yml`

Relevant current tests:

- `tests/integration_phase12.rs`
- `tests/integration_duumbi684_rewrite.rs`
- `tests/integration_duumbi688_flagship_example.rs`
- `tests/integration_phase9c.rs`
- focused unit tests inside the modules listed above.

Verified source facts:

- `McpServer::run_stdio` currently runs a synchronous JSON-RPC stdio loop.
- `McpServer::list_tools` currently returns graph, build, deps, intent, model
  telemetry, and rewrite tool definitions.
- `McpServer::dispatch_tool_call` maps tool failures into JSON-RPC internal
  errors with a string `message` and no structured DUUMBI envelope.
- `tools/call` wraps successful tool values as pretty JSON text inside the MCP
  content array.
- `graph_query`, `graph_validate`, and `graph_describe` are read-only.
- `graph_mutate` applies `PatchOp` values to `.duumbi/graph/main.jsonld`,
  validates, and writes to disk.
- `rewrite_preview` is read-only; `rewrite_apply` reruns matching, validates,
  saves a snapshot, and writes the candidate graph.
- `build_compile`, `build_run`, `deps_search`, `deps_install`,
  `intent_create`, and `intent_execute` currently return descriptive errors
  directing callers to CLI commands.
- `QueryEngine` already calls a provider text API, returns `QueryAnswer`, and
  suggests handoff for mutation-shaped or intent-shaped prompts.
- `SessionManager` persists turns and usage stats, but it does not model
  pending approvals or MCP action evidence.
- CLI `duumbi run` can run a workspace binary and print stdout/stderr.
- `examples/flagship-http-sqlite-json` materializes a local workspace, vendors
  dependencies, builds offline, serves one loopback request, and returns the
  expected JSON body.
- `tests/integration_duumbi688_flagship_example.rs` already checks the flagship
  example through a local process-level smoke path.
- The current GitHub token lacks `read:project`; issue labels and comments are
  available but Project V2 status is not.

Assumptions and recommendations:

- Prefer adding shared MCP response/error/capability types before changing
  individual tools. This reduces schema drift.
- Prefer reusing existing CLI/backend helpers behind MCP instead of spawning
  shell commands from tool handlers.
- Refactor MCP dispatch for async-capable tools rather than creating nested
  Tokio runtimes in arbitrary synchronous handlers.
- Preserve existing CLI behavior and current tests while adding agent-safe MCP
  paths.
- Treat legacy immediate-write MCP tools as compatibility surfaces only if they
  cannot be approval-gated without unacceptable breakage. The documented
  external-agent path should use preview, approval request, and approved apply.
- Keep provider-backed MCP tests out of normal CI unless they use mocks. Live
  provider or external-agent runs belong in manual E2E evidence.

## Affected Areas

Expected Stage 10 source changes:

- `src/mcp/`
  - Add response/error/capability modules such as:
    - `response.rs`
    - `error.rs`
    - `capability.rs`
    - `approval.rs`
    - `evidence.rs`
  - Add or update tool modules such as:
    - `tools/status.rs`
    - `tools/workspace.rs`
    - `tools/query.rs`
    - `tools/approval.rs`
    - `tools/evidence.rs`
  - Update `tools/mod.rs` exports.
  - Update `server.rs` tool registry, dispatch, JSON-RPC error data, and
    async handling.
- Existing MCP tools:
  - `tools/graph.rs`: structured responses/errors, preview/request/apply path,
    exact-candidate stale checks where approval is used.
  - `tools/rewrite.rs`: structured responses/errors and optional approval
    integration for apply.
  - `tools/build.rs`: real build/run behavior or structured unavailable state.
  - `tools/deps.rs`: real vendor/install behavior where feasible or structured
    unavailable state for network-dependent operations.
  - `tools/intent.rs`: async provider-backed create/execute support through the
    shared dispatch design or structured unavailable state when credentials are
    missing.
  - `tools/model_telemetry.rs`: align errors with the shared envelope without
    weakening read-only safety.
- Query:
  - `src/query/*`: expose a command-facing API usable by MCP without changing
    CLI/Studio Query semantics.
  - Add tests for MCP query read-only guarantees.
- Session and approval state:
  - Extend `src/session/mod.rs` or add a new `src/approval/` module for pending
    approval records and MCP action evidence.
  - Store state under a stable local path such as `.duumbi/session/approvals/`
    or `.duumbi/evidence/mcp/`, chosen during implementation.
- CLI/TUI:
  - `src/cli/repl.rs`, `src/cli/app.rs`, and `src/cli/mode.rs` for a bounded
    approval panel or status path.
  - Preserve `Esc` close behavior and Query mode read-only defaults.
- Studio:
  - `crates/duumbi-studio/src/ws.rs`, `app.rs`, and/or `script/studio.js` for
    displaying pending approval records and sending approve/reject decisions.
  - Keep UI changes minimal and focused on approval control.
- Build/run and workspace helpers:
  - `src/workspace.rs`, `src/workflow.rs`, and `src/cli/commands.rs` may need
    shared non-CLI APIs for build/run capture.
  - Avoid parsing terminal output when a structured helper can return stdout,
    stderr, exit code, and artifact paths directly.
- Intent/deps helpers:
  - `src/intent/*` and `src/cli/deps.rs` may need command-facing APIs that
    return structured values without printing-only behavior.
- Benchmark:
  - `src/bench/*` or a focused new `src/agent_benchmark/` module if extending
    existing benchmark code would create unclear boundaries.
  - Prefer extending `src/bench` when the new benchmark can share report and
    provider telemetry patterns.
- Docs and source artifacts:
  - a focused guide such as `docs/agents/mcp-agent-guide.md`;
  - an MCP audit such as `docs/agents/mcp-workflow-audit.md`;
  - a benchmark/case-study artifact under `docs/e2e/` or `docs/agents/`;
  - a Claude Code skill/plugin package under a clear source-controlled path,
    such as `docs/agents/claude-code-duumbi-skill/` or `.agents/skills/` only
    if repo conventions allow it.
- Tests:
  - new `tests/integration_duumbi719_mcp_agent_surface.rs` or split focused
    files when one file becomes too broad;
  - existing `tests/integration_phase12.rs`;
  - existing `tests/integration_duumbi684_rewrite.rs`;
  - existing `tests/integration_duumbi688_flagship_example.rs`;
  - unit tests colocated with new response, approval, and capability modules.
- CI/docs:
  - `.github/workflows/ci.yml` only if new doc or MCP tests require workflow
    inclusion beyond normal test discovery.
  - Do not modify GitHub workflow semantics unless the implementation directly
    needs a new check for #719 evidence.

Areas that must not change during Stage 10 without explicit review:

- Product and technical spec files after Stage 9 approval.
- Provider setup UX, model catalog routing, or default model selection unless a
  narrow MCP intent/query path requires passing through existing provider APIs.
- Registry publishing/yanking semantics.
- Production network/TLS/authentication behavior.
- Mobile approval surfaces.
- Stage 11 review policy or Greptile routing.
- Broad Studio redesign unrelated to approval state.

## Technical Approach

### 1. Add Shared MCP Response And Error Types

Create DUUMBI-owned MCP response types instead of returning ad hoc `Value` or
string errors from every tool.

Recommended shape:

```rust
pub struct McpToolEnvelope<T> {
    pub ok: bool,
    pub tool: String,
    pub stage: String,
    pub data: Option<T>,
    pub error: Option<McpToolError>,
    pub evidence: Vec<McpEvidenceRef>,
}

pub struct McpToolError {
    pub code: String,
    pub category: McpErrorCategory,
    pub message: String,
    pub retryable: bool,
    pub node_ids: Vec<String>,
    pub files: Vec<String>,
    pub artifacts: Vec<String>,
    pub suggested_repairs: Vec<McpRepairCategory>,
    pub approval: Option<McpApprovalRef>,
    pub source: Option<McpErrorSource>,
    pub raw_exit: Option<McpProcessExit>,
}
```

Recommendations:

- Keep serialization field names stable and agent-friendly, likely camelCase in
  JSON if existing MCP payloads already use camelCase.
- Return structured tool failures through JSON-RPC `error.data` or a consistent
  MCP text JSON envelope. Do not mix styles by tool.
- Preserve JSON-RPC protocol errors for malformed JSON-RPC requests, but put
  DUUMBI diagnostic data in `data` when the method reached a DUUMBI tool.
- Map existing `errors.rs` codes into this shape before adding new
  MCP-specific codes.
- Add MCP-specific categories for:
  - `approval_required`;
  - `approval_rejected`;
  - `approval_stale`;
  - `unsupported`;
  - `provider_unavailable`;
  - `network_unavailable`;
  - `timeout`;
  - `workspace_uninitialized`.

### 2. Make Tool Metadata First-Class

Replace the hand-written `list_tools` vector with a registry that owns both
runtime dispatch metadata and public tool definitions.

Each tool definition should include internal metadata:

- name;
- description;
- JSON Schema;
- read/write safety;
- approval requirement;
- writes performed;
- provider/network requirement;
- evidence produced;
- backend function.

Expose an MCP capability/status tool that reports current workspace capability,
not only static tool schemas. Example data:

- workspace initialized: true/false;
- provider configured: true/false;
- dependencies vendored: true/false/unknown;
- build output present: true/false/unknown;
- approval records pending count;
- tools unavailable and why.

This is how an external agent decides whether the full loop is currently
possible.

### 3. Refactor Dispatch For Async-Capable Tools

Current `run_stdio` is synchronous, while intent and deps paths need async
provider or network calls. Stage 10 should choose one of these approaches after
inspecting constraints:

- Preferred: add an async stdio loop and async dispatch path, then run MCP from
  the existing Tokio runtime in `main.rs`.
- Acceptable if simpler and safe: keep synchronous stdio reading but hand work
  to async functions through a controlled runtime handle owned by `run_mcp`.

Avoid:

- creating a new Tokio runtime inside every tool call;
- blocking the async runtime with long build/run work;
- shelling out to `duumbi` when a shared Rust helper exists;
- losing structured stdout/stderr/exit evidence by relying only on terminal
  text.

Build/run can remain blocking internally if wrapped in `spawn_blocking` or an
equivalent boundary. Intent/deps network/provider paths should stay async.

### 4. Add MCP Query Tool

Add a read-only query tool, preferably named consistently with the local MCP
style. If the final name diverges from MODE-010 `query.ask`, document why.

Input:

- `question` required;
- optional `module`;
- optional `c4_level`;
- optional `include_sources`;
- optional `max_context` or other bounded knobs only if needed.

Output:

- answer text;
- model label;
- sources;
- confidence;
- suggested handoff;
- evidence reference if the implementation records a query turn.

Use `QueryEngine` and `QueryRequest`. For tests, use a mock `LlmProvider` so CI
does not make live provider calls.

Read-only guard:

- capture hashes or byte contents of `.duumbi/graph`, `.duumbi/intents`,
  `.duumbi/deps.lock`, `.duumbi/history`, and build outputs before and after
  query tests.
- Assert no mutation for normal questions and mutation-shaped prompts.

### 5. Add Workspace, Build, Run, Deps, Intent, And Evidence MCP Paths

Workspace:

- `workspace_status`: read-only summary of workspace initialization and key
  paths.
- `workspace_init`: create `.duumbi` workspace state by reusing
  `cli::init::run_init`.

Build/run:

- Replace stub behavior with shared backend calls.
- Return:
  - status;
  - output path;
  - stdout;
  - stderr;
  - exit code;
  - timeout state;
  - command/evidence metadata.
- Keep local loopback execution bounded. Do not introduce public network
  exposure.

Deps:

- Prefer supporting local/vendor operations needed by the flagship example
  first.
- For registry network operations, either implement the async path with existing
  registry client helpers or return structured `network_unavailable` /
  `unsupported` data when credentials/network are absent.

Intent:

- Reuse `intent::create`, `intent::review`, `intent::status`, and
  `intent::execute` APIs where possible.
- Add structured status and execution evidence so benchmark tooling can
  attribute first-pass, repair, and validation states if the external-agent path
  uses intents.
- Return provider credential issues as structured `provider_unavailable`
  errors.

Evidence:

- Add read-only evidence retrieval for recent MCP actions, approval records,
  intent reports, build/run results, rewrite evidence, benchmark reports, and
  telemetry/evidence files that already exist.
- Keep evidence bounded and redacted. Do not return secrets, raw provider
  credentials, or unbounded logs.

### 6. Add Approval State And Safe Write Flow

Introduce an approval model that can cover graph patch and rewrite apply first.

Recommended approval record:

```rust
pub struct McpApprovalRecord {
    pub id: String,
    pub status: McpApprovalStatus,
    pub requested_tool: String,
    pub requested_action: String,
    pub candidate_hash: String,
    pub workspace_hash: String,
    pub affected_files: Vec<String>,
    pub affected_node_ids: Vec<String>,
    pub risk: McpApprovalRisk,
    pub summary: String,
    pub evidence: Vec<McpEvidenceRef>,
    pub created_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_source: Option<String>,
    pub rejection_reason: Option<String>,
}
```

Flow:

1. Preview candidate without writes.
2. Request approval with candidate hash and affected areas.
3. TUI or Studio lists pending approvals.
4. Human approves or rejects.
5. Apply verifies:
   - approval exists;
   - approval is approved;
   - candidate hash matches;
   - workspace hash has not changed or is intentionally revalidated;
   - requested tool/action matches;
   - validation still passes.
6. Apply writes and records evidence, or rejects with a structured stale or
   invalid approval error.

Implementation guidance:

- Add new safe agent-facing tools instead of silently changing legacy write
  semantics if compatibility risk is high.
- If keeping `graph_mutate` immediate apply for compatibility, mark it clearly
  as trusted/immediate in capability metadata and guide external agents to the
  approval flow.
- Approval records should be local workspace state and should not require
  GitHub, Slack, or network access.
- TUI/Studio approval UI must be minimal, keyboard-complete, and avoid
  startup warnings during unrelated flows.

### 7. Add Agent Docs And Claude Code Skill/Plugin Artifact

Add documentation after tool names and schemas are stable enough.

Recommended source artifacts:

- `docs/agents/mcp-workflow-audit.md`
- `docs/agents/mcp-agent-guide.md`
- `docs/agents/mcp-error-contract.md`
- `docs/e2e/duumbi-719-mcp-agent-benchmark.md` or JSON plus Markdown summary
- `docs/agents/claude-code-duumbi-skill/` for the skill/plugin source package,
  unless an existing repo convention points elsewhere.

The guide must:

- show how to start `duumbi mcp`;
- show the full loop order;
- say which tools are read-only;
- explain approval;
- explain structured errors;
- define evidence expected before completion;
- include the flagship benchmark path;
- avoid secrets and provider keys;
- avoid Greptile for spec or implementation unless Stage 11 later recommends it
  for a final implementation PR.

### 8. Add MCP-Only Flagship Benchmark And Raw Rust Baseline

Use `examples/flagship-http-sqlite-json` as the canonical scenario.

Preferred implementation:

- Add a benchmark harness that can run a scripted external-agent MCP workflow
  deterministically for CI-safe portions.
- Add a manual live-agent mode for Claude Code, Codex, or another configured
  external agent.
- Reuse existing `src/bench/report.rs` patterns where practical.

Required MCP-only evidence:

- `initialize` and `tools/list`;
- capability/status;
- workspace init or materialization;
- dependency/vendor state;
- graph/intent inspection;
- build;
- run;
- HTTP response check;
- evidence retrieval;
- structured report with limitations.

Raw Rust baseline:

- Use the same flagship behavior and compare against an agent editing raw Rust
  or generating a Rust service directly.
- Record exact prompt, agent/provider if known, success/failure, elapsed time,
  turns, token/cost availability, output, and failure category.
- If token/cost telemetry is unavailable, report `unavailable` with a reason.

The case-study artifact must not claim production adoption, customer use, or
benchmark superiority beyond measured evidence.

## Invariants

- Query mode remains read-only in CLI, Studio, and MCP.
- Tool responses never include provider credentials, registry tokens, Slack
  secrets, approval capability URLs, or raw private environment values.
- Existing CLI commands continue to work unless the implementation explicitly
  updates and tests a shared backend contract.
- Graph writes remain all-or-nothing with validation before durable write.
- Approval applies only to the exact requested candidate or bounded operation.
- Build/run MCP tools must not expose public network listeners or unbounded
  processes.
- CI tests must not require live providers, live registries, external agents, or
  network access.
- Greptile is not invoked for spec PRs or this Stage 10 implementation unless
  Stage 11 later recommends it for the final implementation PR and a human
  triggers it.
- Stage 10 agents must not broaden this issue into mobile approval, cloud
  execution, marketplace publishing, or a general benchmark framework without a
  human product/architecture decision.

## BDD-To-Test Mapping

| Product BDD scenario | Verification evidence |
| --- | --- |
| Agent discovers the complete DUUMBI loop | MCP tool registry unit test plus integration test calling `initialize`, `tools/list`, and capability/status. Assert init, query, intent, preview, approval, apply, build, run, and evidence capabilities are represented with safety metadata. |
| Agent asks a read-only question through MCP | Mock-provider integration test for MCP query. Snapshot `.duumbi/graph`, `.duumbi/intents`, `.duumbi/history`, deps, and build outputs before/after and assert unchanged. |
| Mutation-shaped query returns a handoff | Mock-provider or classifier-driven MCP query test with "add a function"; assert suggested handoff is Agent or Intent and protected files are unchanged. |
| Write-capable request produces an approval record | Unit/integration test for graph patch or rewrite approval request. Assert pending record fields, affected file/node evidence when known, and unchanged graph. |
| Approved agent mutation applies exactly the reviewed candidate | Integration test that previews a small patch, approves it through local approval API, applies it, and asserts candidate hash, validation evidence, snapshot/rollback evidence, and graph change. |
| Stale approval is rejected | Integration test that changes the graph after approval request and before apply. Assert structured `approval_stale` error and no write from the stale candidate. |
| Build and run return captured evidence | Integration test materializing a small workspace through MCP, building, running, and asserting output path, stdout/stderr, exit code, timeout state, and evidence path. |
| Structured errors are actionable | Focused tests for malformed JSON params, missing workspace, invalid graph, validation/type/ownership errors where fixtures exist, missing provider, unsupported network, approval required, stale approval, build failure, runtime failure, and timeout. Assert code/category/retryable/suggested repair fields. |
| MCP-only flagship benchmark succeeds | Manual or gated live E2E report plus deterministic harness tests for report shape. The live report must include the flagship JSON response and MCP-only evidence, or a blocker report with structured failure. |
| Raw Rust baseline is reported honestly | Benchmark report test for baseline schema and unavailable telemetry handling; manual baseline evidence checked into docs/evidence if live run is performed. |
| Agent guide prevents unsafe assumptions | Docs/source test or script that checks required sections, current tool names, no auto-close issue references, no secrets, and no Greptile instructions for spec/implementation stages. |

## Live E2E Plan

Canonical interface: MCP stdio through `target/debug/duumbi mcp`.

Provider-free live smoke:

- Required credentials: none.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Command shape after implementation:

```sh
cargo build
target/debug/duumbi mcp
```

- Drive JSON-RPC over stdin/stdout from a test harness or script:
  - `initialize`;
  - `tools/list`;
  - capability/status;
  - workspace/materialization for the flagship example;
  - deps vendor/install path needed for offline build;
  - build;
  - run;
  - evidence retrieval.
- Pass criteria:
  - MCP responses are valid JSON-RPC/MCP payloads;
  - build succeeds;
  - run serves one loopback request;
  - JSON response body matches the flagship expected payload;
  - evidence includes command, status, stdout/stderr, exit, and artifact paths.

Live LLM-backed external-agent path:

- Required credentials:
  - an MCP-capable external agent configured by the human, or a scripted Codex
    App run using the local MCP server;
  - DUUMBI provider credentials only if the run uses `query` or `intent`
    provider-backed tools.
- Expected external LLM calls:
  - Codex App internal reasoning does not count against the Ralph gate.
  - External agent CLI/model calls and DUUMBI live provider calls do count.
  - A low-cost run should target a single flagship attempt and stay under USD
    1. If the next cycle estimates more than USD 1 in external LLM cost, the
    implementation agent must stop for human approval before running it.
- Command/evidence shape:
  - start `target/debug/duumbi mcp`;
  - run the configured external agent with the DUUMBI MCP guide/skill;
  - record prompt, model/agent when known, MCP transcript or redacted summary,
    benchmark report, response body, and limitations.
- Pass criteria:
  - external agent uses MCP for DUUMBI operations;
  - no raw graph/source editing is used for DUUMBI state unless the baseline run
    explicitly measures raw Rust behavior;
  - report includes success/failure, turns, elapsed time, token/cost
    availability, and failure categories.

TUI/Studio parity smoke:

- TUI: create a pending approval through MCP, open the REPL/TUI, verify the
  approval state can be viewed and approved/rejected with keyboard controls.
- Studio: create a pending approval through MCP, open Studio if the local dev
  server path is available, verify the approval state can be viewed and a
  decision can be sent.
- Full Studio visual E2E is required only for approval UI changes. Query/build
  backend behavior can be covered through MCP/CLI tests plus thin Studio parity.

## Ralph Cycle Protocol

Each Stage 10 cycle must:

1. summarize the current state and remaining unmet requirements;
2. propose one bounded implementation goal;
3. list intended file areas and commands;
4. estimate resource use and risk;
5. check whether the resource gate requires human approval;
6. implement only the approved or resource-permitted goal;
7. run the agreed checks;
8. report evidence, failures, and remaining gaps;
9. stop only if requirements are met, a blocker appears, the expected external
   LLM cost of the next cycle exceeds USD 1, scope changes, risky dependency or
   security-sensitive work appears, irreversible operations are needed, or a
   product/architecture decision is required.

Iteration count is not a stop condition.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle:
  - shared MCP response/error/capability primitives: up to 6 closely related
    files;
  - one tool family per cycle after primitives are in place;
  - approval state plus one UI surface per cycle, unless a smaller split is
    safer;
  - docs/benchmark evidence can be a separate cycle after APIs stabilize.
- Expected command budget per cycle:
  - always run targeted unit/integration tests for touched modules;
  - run `cargo fmt --check` or `cargo fmt` as appropriate before final review;
  - run `cargo clippy --all-targets -- -D warnings` before implementation PR
    readiness when Rust code changed;
  - run broader `cargo test --all` or CI-equivalent before final Stage 10 PR
    readiness when MCP shared behavior changed.
- Human approval is required only when:
  - the next cycle will use external LLM calls with expected cost above USD 1;
  - scope expands beyond #719 or these specs;
  - risky dependency, migration, security-sensitive behavior, irreversible
    operation, broad refactor, or product/architecture decision appears;
  - checks fail in a way the agent cannot resolve within approved scope;
  - the agent wants to run a live external-agent benchmark whose expected
    external model cost exceeds USD 1.
- External LLM usage counted:
  - DUUMBI live provider calls;
  - external model/agent CLI calls;
  - benchmark baseline model calls when not covered by Codex App subscription.
- Codex internal reasoning usage in Codex App is covered by the Codex App
  subscription and never triggers the gate.
- No autonomous batch cap: cycles continue until completion, blocker, gate
  breach, or scope change.
- Stop and ask for human guidance when:
  - approval UX requires a larger product choice than the spec makes;
  - MCP protocol compatibility requires breaking existing clients;
  - benchmark design cannot honestly compare MCP-only and raw Rust paths;
  - a security-sensitive approval or write path cannot be bounded locally.

## Task Breakdown

1. Add MCP workflow audit and finalize tool/capability names.
2. Add shared MCP response/error/capability data model and tests.
3. Refactor server tool registry/dispatch to use the shared metadata.
4. Add MCP capability/status tool.
5. Add MCP query tool using `QueryEngine` and mock-provider tests.
6. Convert existing graph/rewrite/model telemetry errors to the shared envelope.
7. Add workspace init/status MCP tools.
8. Add build/run MCP tools with captured evidence and bounded timeout behavior.
9. Add deps MCP behavior needed for local vendor/offline flagship workflows.
10. Add intent MCP behavior or structured provider-unavailable/unimplemented
    states, depending on async dispatch readiness.
11. Add approval record model, storage, preview/request/status/decision/apply
    flow for at least graph patch and rewrite apply.
12. Add TUI approval view/decision surface.
13. Add Studio approval view/decision surface if product checks require it for
    parity.
14. Add evidence retrieval MCP tool and bounded redaction rules.
15. Add agent guide, error-contract docs, audit docs, and Claude Code
    skill/plugin artifact.
16. Add benchmark/report support for MCP-only flagship and raw Rust baseline.
17. Run provider-free MCP flagship smoke.
18. Run low-cost live external-agent E2E only when the resource gate permits or
    human approval is obtained.
19. Consolidate docs, evidence, and final implementation PR review notes.

## Verification Plan

Automated checks:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`
- Focused MCP tests:
  - tool schema and capability metadata;
  - unknown field rejection;
  - structured error envelope;
  - query read-only behavior;
  - graph/rewrite approval flow;
  - stale approval rejection;
  - build/run capture;
  - evidence retrieval redaction;
  - benchmark report schema.
- Existing regression tests:
  - `tests/integration_phase12.rs`;
  - `tests/integration_duumbi684_rewrite.rs`;
  - `tests/integration_duumbi688_flagship_example.rs`;
  - `tests/integration_phase9c.rs`.

Manual/live checks:

- Provider-free MCP stdio smoke for flagship build/run.
- Low-cost live query or intent MCP call with configured provider when inside
  the USD 1 gate.
- External-agent MCP-only flagship benchmark or blocked-evidence report.
- Raw Rust baseline run or explicit unavailable/blocker evidence.
- TUI approval smoke.
- Studio approval smoke only if Studio approval UI changed.

Review evidence:

- Codex self-review of the implementation PR.
- Optional quick low-cost reviewer is advisory only.
- Greptile not used unless Stage 11 later recommends it for the final
  implementation PR and a human triggers it.

## Completion Criteria

Before the implementation PR is ready for Stage 11 review:

- Every product BDD scenario has passing automated evidence, live evidence, or a
  documented blocked state accepted by the issue owner.
- Tool discovery and capability/status let an external agent plan the full loop
  without hidden CLI assumptions.
- MCP Query is read-only and tested.
- Structured error envelope is used consistently by new and updated MCP paths.
- Approval flow prevents stale or unreviewed write candidates from being
  applied through the agent-safe path.
- Build/run MCP evidence includes stdout, stderr, exit code, timeout status,
  and artifact paths.
- Agent guide and Claude Code skill/plugin artifact exist and match current tool
  names.
- MCP-only flagship benchmark evidence exists, or the final report clearly
  identifies the blocker and no false success claim is made.
- Raw Rust baseline evidence exists or is explicitly unavailable with a reason.
- Normal CI-safe tests do not require live providers, live registries, external
  agents, or network.
- No implementation scope touches mobile approval, cloud execution, marketplace
  publishing, or final marketing claims beyond source evidence.

## Failure And Escalation

- If async MCP dispatch requires a broad server rewrite, stop after documenting
  the narrowest safe design and ask for an architecture decision.
- If approval semantics would break existing MCP clients, preserve compatibility
  with clear metadata and add a separate agent-safe approval flow.
- If TUI/Studio approval surfaces would become broad redesign work, implement
  the smallest reviewable control and defer polish.
- If live providers or external agents are unavailable, produce a provider-free
  smoke plus structured blocked evidence; do not fake live E2E success.
- If token/cost telemetry is unavailable, record `unavailable` with reason.
- If benchmark results are poor, commit honest failure evidence and failure
  categories rather than tuning prompts silently.
- If a dependency addition is proposed for MCP testing, approval, or benchmark
  harness work, justify it against standard-library or existing-crate options
  and treat risky additions as a resource gate.
- If security-sensitive behavior appears in approval, file access, command
  execution, or MCP transport, stop for review before implementation continues.

## Open Questions

- None blocking for Stage 10. Implementation may choose exact tool names,
  storage paths, and async dispatch shape within the constraints above.
