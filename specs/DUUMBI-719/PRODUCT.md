# DUUMBI-719: Agent Substrate - MCP As First-Class Interface

## Summary

Make DUUMBI's MCP server a first-class external coding-agent interface. An
off-the-shelf agent should be able to discover DUUMBI capabilities, initialize
or inspect a workspace, ask read-only questions, create and execute intents,
preview and request graph changes, build, run, collect evidence, and recover
from structured errors through MCP alone.

Related to #719. This is a product-specification artifact only. The execution
issue must remain open for Stage 8 technical specification, Stage 9 Technical
Spec Review, Stage 10 implementation, Stage 11 implementation review, and Stage
12 closure.

## Problem

DUUMBI's product thesis is strongest when an agent edits a typed,
ownership-checked, schema-validated semantic graph instead of raw source text.
The source issue identifies a product gap: DUUMBI has an MCP server, but
external coding agents are not yet treated as first-class users of the full
DUUMBI loop.

Current verified source context shows the gap is real:

- `src/mcp/server.rs` registers graph, build, deps, intent, model telemetry, and
  rewrite tools, but several workflow tools are currently descriptive stubs
  that tell the caller to use the CLI.
- `src/mcp/tools/build.rs`, `src/mcp/tools/deps.rs`, and
  `src/mcp/tools/intent.rs` return prose errors for core workflow steps.
- `graph_mutate` writes directly after validation and returns string-based
  failures rather than a stable machine-readable repair contract.
- Query mode exists in CLI/Studio with answer text, sources, confidence, model,
  and suggested handoff, but MCP does not yet expose the conversational query
  surface described by MODE-010.
- The semantic rewrite MCP tools are safer and more evidence-rich than raw
  mutation, but they do not cover the complete loop.
- The flagship HTTP + SQLite + JSON example exists, but there is no benchmark
  proving that an external agent can drive that path through MCP only.
- Agent-facing onboarding docs and a Claude Code skill/plugin package are not
  yet the documented product path.

The issue is not only "add more tools." A first-class agent substrate also
needs structured capability discovery, stable error objects, approval gates for
write-capable requests, documentation, and measured evidence that the MCP path
reduces raw-text failure modes.

## Outcome

When this work is done:

- DUUMBI has an MCP workflow audit artifact that maps the full agent loop to
  implemented MCP capabilities:
  - workspace initialization and status;
  - read-only query;
  - intent creation, review/status, and execution;
  - graph query, validation, description, mutation preview, mutation apply, and
    semantic rewrite preview/apply;
  - dependency/vendor operations needed by local examples;
  - build, run, and command output capture;
  - evidence retrieval for session, intent, build/run, benchmark, rewrite, and
    validation artifacts.
- `tools/list` exposes a coherent, documented agent-facing tool surface. Tool
  names, descriptions, JSON Schemas, read/write safety, and approval behavior
  are stable enough for external agents to plan against.
- External agents can ask conversational read-only questions through MCP and
  receive answer text, source references, confidence, model metadata, and
  suggested handoff without mutating graph, intent, session, or build state.
- Workflow tools that previously redirected to the CLI have either real MCP
  behavior or a documented non-goal with a structured unavailable response.
- All MCP tool failures use a machine-readable error envelope with an error
  code, category, message, affected node ids when available, file or artifact
  references when available, suggested repair categories, retryability, and
  source command/tool context.
- Agent-initiated write-capable requests have an explicit approval path. A
  caller can preview, submit an approval request, inspect approval state, and
  apply only after approval where required.
- TUI and Studio surfaces expose enough approval state for a human to approve
  or reject agent-initiated mutations without requiring a separate terminal
  command.
- Session and evidence artifacts record agent-facing MCP actions, approval
  decisions, applied changes, build/run results, and benchmark outcomes.
- An AGENTS.md-grade external-agent guide explains how Claude Code, Codex, or a
  comparable MCP-capable agent should drive DUUMBI through MCP.
- A Claude Code skill/plugin package or equivalent source artifact is present
  and documented. It should configure or instruct the agent to use DUUMBI MCP
  safely; it does not need to ship through an external marketplace in this
  issue unless that publishing path already exists.
- The flagship HTTP + SQLite + JSON example can be driven by an external agent
  through MCP only, with measured success/failure evidence.
- The benchmark compares the MCP-only path against a raw Rust text-editing
  baseline for the same scenario, recording success, turns, tokens when
  available, cost when available, and failure categories.
- A source-repo case-study or evidence document is ready for publication and
  clearly separates measured facts from product interpretation.

## Scope

### In Scope

- Audit the current MCP tool surface and publish a gap map in the source repo.
- Add or harden MCP tools required for the full local DUUMBI loop:
  - workspace init/status;
  - query ask;
  - intent create/status/review/execute;
  - dependency install/vendor as needed for local examples;
  - build and run with captured stdout, stderr, exit code, timeout, and artifact
    paths;
  - evidence/status retrieval;
  - approval request/status/decision integration for write-capable agent calls.
- Preserve and clearly label read-only versus write-capable tools.
- Define and implement a stable MCP error envelope across graph, parser,
  validator, compiler, runtime, dependency, registry, intent, query, rewrite,
  build, run, and approval paths.
- Map existing DUUMBI error codes into the envelope and add MCP-specific codes
  where needed.
- Provide repair categories such as `schema`, `type`, `ownership`,
  `missing_dependency`, `provider`, `approval_required`, `build`, `runtime`,
  `timeout`, `network`, `workspace`, and `unsupported`.
- Add approval ledger/state sufficient for an external agent to request and
  observe human decisions while the user remains in control of writes.
- Add TUI and Studio approval visibility only to the extent needed for the
  agent approval gate.
- Add agent-facing documentation and a Claude Code skill/plugin source package
  or setup recipe.
- Add tests for tool schemas, read-only guarantees, write approval behavior,
  structured errors, build/run capture, query MCP behavior, and evidence
  retrieval.
- Add an MCP-only flagship benchmark using
  `examples/flagship-http-sqlite-json`.
- Add a raw Rust baseline benchmark or recorded comparable baseline for the
  same flagship task, with honest limitations when token/cost telemetry is
  unavailable.
- Add a compact case-study/evidence document suitable for later public docs.

### Explicitly Out Of Scope

- Mobile approval surfaces.
- Cloud-hosted DUUMBI agent execution.
- Public hosted docs deployment if the existing docs pipeline does not already
  support it. The required deliverable is source-repo documentation ready to be
  published.
- Marketplace distribution of the Claude Code skill/plugin beyond a source
  artifact or documented package.
- A new general-purpose benchmark framework unrelated to the flagship MCP path.
- Autonomous acceptance of agent writes without human approval where approval is
  required.
- Production network exposure, authentication, TLS, or multi-tenant MCP hosting.
- Replacing the CLI or Studio. The MCP path should share backend behavior with
  existing surfaces where practical.
- Greptile review or any final implementation PR review work during this spec
  stage.
- Implementation code, tests, generated evidence, runtime assets, or Ralph
  cycles during specification.

## Constraints And Assumptions

Facts:

- Issue #719 is open and labeled `accepted` and `needs-spec`.
- The Stage 5 Human Acceptance Decision comment dated 2026-06-17 records
  `Decision: Accept`, `Next state: Spec Needed`, and no remaining open
  questions.
- The current GitHub token in this Codex session can read issues and labels but
  cannot read Project V2 fields because it lacks `read:project`.
- The processed inbox source asks for a full loop through MCP alone: init,
  query, intent, patch, build, run, and evidence.
- `src/mcp/server.rs` implements JSON-RPC 2.0 stdio transport and registers
  graph, build, deps, intent, model telemetry, and rewrite MCP tools.
- `src/mcp/tools/graph.rs` implements graph query, mutate, validate, and
  describe against `.duumbi/graph/main.jsonld`.
- `src/mcp/tools/build.rs`, `src/mcp/tools/deps.rs`, and
  `src/mcp/tools/intent.rs` currently return informative errors that redirect
  to CLI commands for core workflow behavior.
- `src/mcp/tools/rewrite.rs` already exposes read-only rule discovery,
  read-only preview, and write-capable apply with snapshot-backed validation.
- `docs/modes/query-mode-spec.md` says Query mode is delivered for CLI/Studio
  and identifies MODE-010 as the MCP conversational query tool.
- `src/query/engine.rs` and `src/query/sources.rs` define Query answers with
  text, model, sources, confidence, and suggested handoff.
- `src/session/mod.rs` persists conversation turns and usage stats but is not
  currently an approval ledger.
- `examples/flagship-http-sqlite-json` and
  `tests/integration_duumbi688_flagship_example.rs` prove a bounded local HTTP
  + SQLite + JSON reference program exists.
- The active PRD describes DUUMBI as intent-first, queryable-first,
  graph-centered, evidence-oriented, human-verifiable, and tool-agnostic.
- The active agentic runbook defines the Ralph Cycle external-LLM resource gate
  as USD 1 for Stage 10 implementation cycles.

Assumptions:

- The initial external-agent target is a local trusted developer environment
  using stdio MCP, not a remote multi-tenant server.
- Existing CLI behavior should remain available while MCP gains equivalent
  backend paths.
- Existing write-capable tools may keep compatibility only when their safety
  semantics are explicitly documented. New agent onboarding should prefer
  preview/request/apply flows with human approval for mutations.
- Token and cost measurement may be unavailable for some external agents or
  providers. The benchmark should report unavailable telemetry explicitly
  instead of fabricating estimates.
- The first Claude Code skill/plugin can be a source package or setup guide if
  automated packaging would require a separate distribution pipeline.
- The case study should be evidence-backed and source-controlled first; public
  marketing publication can follow once the evidence is reviewed.

## Decisions

- **Decision:** Use source-repo file-based specs for #719.
  **Evidence:** The issue is architectural, cross-module, agent-facing, and
  durable. It affects MCP, query, intent, build/run, approval UX, docs, and
  benchmark evidence.

- **Decision:** Treat the current "10 tools" language as historical input, not
  as a hard tool-count contract.
  **Evidence:** The current server registers graph, build, deps, intent, model
  telemetry, and rewrite tools. The product requirement is complete workflow
  coverage, not preserving a fixed number.

- **Decision:** The first benchmark target is the existing flagship HTTP +
  SQLite + JSON example.
  **Evidence:** The processed inbox source names that scenario, and #688
  already delivered a checked-in example plus an integration smoke test.

- **Decision:** MCP query must reuse Query mode semantics rather than calling
  graph mutation tools with a read-only prompt.
  **Evidence:** The Query Mode specification explicitly says Query is a
  different contract from Agent mode and must enforce read-only behavior in
  code.

- **Decision:** Structured errors are part of the product contract, not only an
  implementation detail.
  **Evidence:** External agents need stable fields to self-correct; prose-only
  errors are not reliable planning inputs.

- **Decision:** Write-capable external-agent calls need an approval model.
  **Evidence:** The source issue explicitly calls out approval gates for
  agent-initiated mutations and DUUMBI's PRD requires human-verifiable behavior.

- **Decision:** Stage 10 should not add mobile approval or cloud agent
  execution.
  **Evidence:** The accepted issue marks mobile approval as a separate effort
  and excludes cloud-based agent execution from M1.

## Behavior

### Capability Discovery

- `tools/list` must clearly distinguish:
  - read-only tools;
  - preview-only tools;
  - write-capable tools;
  - tools requiring human approval;
  - tools requiring provider credentials or network access;
  - tools unavailable in the current workspace.
- Tool descriptions must identify whether the tool may write graph files,
  intent files, config, dependency state, build artifacts, session state,
  evidence artifacts, or approval records.
- Tool schemas must reject unknown fields unless a tool has an explicit
  extensibility reason.
- A machine-readable capability report must allow an agent to answer: "Can I
  complete init -> query -> intent -> patch -> build -> run -> evidence through
  MCP in this workspace?"

### Read-Only Query Through MCP

- An external agent can submit a question with optional module, C4 level, and
  source-inclusion preferences.
- DUUMBI assembles the same kind of read-only context used by CLI/Studio Query
  mode.
- The response includes:
  - answer text;
  - source references;
  - confidence;
  - non-secret model metadata;
  - optional suggested handoff to an agent or intent workflow.
- Query MCP calls must not write graph files, intent files, dependency files,
  build outputs, snapshots, approval records, or telemetry beyond explicitly
  approved session/evidence metadata.
- Mutation-shaped prompts must return a suggested handoff instead of silently
  applying changes.

### Full Local Workflow Through MCP

- An agent can initialize a workspace or verify that a workspace is initialized.
- An agent can inspect workspace status, graph modules, dependencies, build
  status, current intents, and recent evidence.
- An agent can create an intent from a natural-language request when provider
  credentials are configured.
- An agent can inspect, review, and execute an intent through MCP where the same
  backend behavior exists for CLI.
- An agent can preview graph or rewrite changes without writing state.
- An agent can request approval for write-capable mutations.
- After approval, an agent can apply the approved mutation and receive
  validation evidence.
- An agent can build and run the workspace through MCP and receive stdout,
  stderr, exit code, artifact paths, timeout status, and diagnostic evidence.
- An agent can retrieve evidence records for the last relevant query, intent,
  mutation, rewrite, build, run, benchmark, or approval event.

### Structured Error Contract

- Every MCP tool failure returns a structured envelope.
- The envelope includes at minimum:
  - `ok: false`;
  - `code`;
  - `category`;
  - `message`;
  - `retryable`;
  - `tool`;
  - `stage`;
  - optional `nodeIds`;
  - optional `files`;
  - optional `artifacts`;
  - optional `suggestedRepairs`;
  - optional `approval`;
  - optional `source`;
  - optional `rawExit`.
- JSON-RPC protocol errors may still use JSON-RPC error codes, but tool
  failures must put actionable DUUMBI data in `error.data` or the MCP content
  payload consistently.
- Existing DUUMBI error codes such as parser, validator, ownership, registry,
  and MCP integration codes should map into the envelope rather than being
  replaced with unrelated strings.
- Missing provider credentials, unavailable registry/network, unsupported async
  capability, approval required, stale approval, timeout, build failure, and
  runtime failure must be distinguishable categories.

### Approval Gate

- A write-capable agent request can be previewed without approval.
- A write-capable request that requires approval returns an approval-required
  response with a stable approval id and a summary of the requested write.
- The approval record includes:
  - requesting tool;
  - requested operation;
  - affected files/modules/node ids when known;
  - risk class;
  - evidence or preview summary;
  - requester/session metadata;
  - creation time;
  - decision state;
  - decision source;
  - expiration or invalidation condition when applicable.
- TUI and Studio expose pending approval records and allow approve/reject
  decisions.
- Rejection leaves workspace state unchanged and records the reason when
  available.
- Approval applies only to the exact candidate or bounded operation it
  references. Stale previews must not be applied silently.
- Approval records are evidence; they do not replace graph validation, build,
  tests, or human implementation review.

### Agent Documentation And Skill Package

- The agent guide explains:
  - how to start or configure DUUMBI MCP;
  - which tools are read-only, preview, write-capable, or approval-gated;
  - the preferred workflow order;
  - how to interpret structured errors;
  - how to recover from common categories;
  - how to request human approval;
  - how to gather evidence before reporting success;
  - how to avoid raw source edits when MCP graph operations exist.
- The Claude Code skill/plugin artifact includes safe operating instructions
  and the MCP setup path. It must not embed secrets.
- Documentation must distinguish measured facts from product positioning.

### Benchmark And Case Study

- The MCP-only benchmark uses the flagship HTTP + SQLite + JSON example.
- The external-agent run must be constrained to MCP for DUUMBI operations. If
  the agent reads docs or issue context directly, the benchmark report must say
  so.
- Required MCP-only evidence includes:
  - capability discovery;
  - workspace initialization/status;
  - dependency/vendor step when needed;
  - graph or intent operations;
  - build;
  - run;
  - HTTP response body check;
  - evidence retrieval.
- The raw Rust baseline uses the same desired behavior and records comparable
  success/failure, turns, elapsed time, token availability, and failure modes.
- The case-study artifact may interpret why the MCP path helps, but only after
  presenting measured evidence and limitations.

### Empty, Error, Retry, Cancellation, And Focus States

- Empty or uninitialized workspaces return structured workspace guidance, not
  panics.
- Unknown tools, malformed params, unknown fields, and unsupported operations
  return structured errors.
- Provider, registry, network, and build timeouts are retryable only when the
  envelope marks them retryable.
- Cancellation or timeout must leave partial writes either unapplied or recorded
  as failed with enough evidence to inspect state.
- TUI and Studio approval panels must be keyboard-reachable and must preserve
  the current rule that Query mode is read-only by default.

## BDD Scenarios

Feature: MCP as a first-class external agent interface

  Scenario: Agent discovers the complete DUUMBI loop
    Given an initialized DUUMBI workspace
    When an external agent calls MCP tool discovery and capability status
    Then the response lists the tools or capability records for init/status, query, intent, preview, approval, apply, build, run, and evidence
    And each capability identifies whether it is read-only, preview-only, write-capable, or approval-gated

  Scenario: Agent asks a read-only question through MCP
    Given a DUUMBI workspace with at least one graph module
    When an external agent calls the MCP query tool with "What modules exist?"
    Then DUUMBI returns answer text, sources, confidence, model metadata, and no graph refresh instruction
    And the graph, intent files, dependency state, snapshots, and build outputs are unchanged

  Scenario: Mutation-shaped query returns a handoff
    Given Query mode is available through MCP
    When an external agent asks the MCP query tool to add a function
    Then DUUMBI returns a suggested Agent or Intent handoff
    And no graph mutation occurs

  Scenario: Write-capable request produces an approval record
    Given an external agent has prepared a graph mutation candidate through MCP
    When the agent requests write approval
    Then DUUMBI records a pending approval with affected files or node ids when available
    And the response includes a stable approval id
    And the workspace graph remains unchanged

  Scenario: Approved agent mutation applies exactly the reviewed candidate
    Given a pending approval exists for a specific graph mutation candidate
    And a human approves the candidate through TUI or Studio
    When the agent applies the approved request through MCP
    Then DUUMBI applies only that candidate
    And it returns validation evidence, snapshot or rollback evidence, and changed node ids when available

  Scenario: Stale approval is rejected
    Given a pending approval exists for a previewed candidate
    And the underlying graph changes before apply
    When the agent attempts to apply the approval
    Then DUUMBI rejects the apply request with a structured stale-approval error
    And it suggests previewing the current graph again

  Scenario: Build and run return captured evidence
    Given a valid initialized DUUMBI workspace
    When an external agent builds and runs the workspace through MCP
    Then DUUMBI returns build artifacts, stdout, stderr, exit code, timeout status, and evidence paths
    And failures include structured diagnostics rather than prose-only messages

  Scenario: Structured errors are actionable
    Given a graph mutation candidate with a type mismatch
    When an external agent validates or applies it through MCP
    Then DUUMBI returns an error envelope with a stable error code, category `type`, affected node ids when available, and suggested repair categories

  Scenario: MCP-only flagship benchmark succeeds
    Given the flagship HTTP + SQLite + JSON example is available in the repo
    And a configured external agent can use DUUMBI MCP
    When the benchmark instructs the agent to complete the flagship workflow through MCP only
    Then the agent produces a local loopback HTTP response containing the expected JSON payload
    And the benchmark report records success, turns, elapsed time, token/cost availability, and evidence artifacts

  Scenario: Raw Rust baseline is reported honestly
    Given the same flagship behavior is used for a raw Rust baseline
    When the baseline run completes or fails
    Then the report records comparable success/failure and limitations
    And unavailable token or cost telemetry is labeled unavailable rather than guessed

  Scenario: Agent guide prevents unsafe assumptions
    Given an external agent reads the DUUMBI MCP guide or skill package
    When it plans a write-capable operation
    Then the instructions direct it to preview, request approval when required, apply only approved changes, run checks, and gather evidence before reporting completion

## Tasks

- Stage 10 can start with an MCP gap audit and error-envelope design because
  those choices constrain the rest of the work.
- Query MCP, build/run MCP, and evidence retrieval can be implemented as
  independent slices if they share the same error envelope.
- Approval ledger and TUI/Studio approval UI should be one coherent slice
  because behavior must line up across caller and human surfaces.
- Intent/deps async MCP behavior should follow the server-runtime decision made
  in the technical spec.
- Agent docs and the Claude Code skill/plugin can be drafted once the tool
  names and safety semantics are stable.
- Benchmark and case-study artifacts should be final slices after the MCP path
  is implemented enough to run the flagship example honestly.

## Checks

- Product spec BDD scenarios map to tests in `TECHNICAL.md`.
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all` or the repo's current CI-equivalent test command.
- MCP tool schema tests verify names, safety labels, JSON Schemas, and unknown
  field rejection.
- Read-only tests verify query, capability, status, preview, and evidence
  retrieval do not mutate protected files.
- Approval tests verify pending, approved, rejected, stale, and exact-candidate
  apply behavior.
- Structured-error tests cover parser, graph validation, type, ownership,
  dependency, provider, approval, build, run, timeout, and unsupported paths
  where practical.
- Build/run MCP tests capture stdout, stderr, exit code, timeout, and artifact
  paths.
- Live E2E benchmark evidence shows an external agent completing the flagship
  example through MCP only, or records the blocking failure with evidence.
- Documentation checks verify the guide references current tool names and does
  not embed secrets or invoke Greptile.
- Stage 10 evidence must say whether Project V2 status writes were available or
  label-only workflow state was used.

## Open Questions

- None blocking for specification. Non-blocking implementation choices remain:
  exact MCP tool names, whether to evolve the existing synchronous stdio server
  or add an async dispatch layer, and the exact source location for the Claude
  Code skill/plugin package.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/719
- Stage 5 acceptance:
  https://github.com/hgahub/duumbi/issues/719#issuecomment-4727796012
- Processed inbox source:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-06-12 - Agent Substrate MCP First-Class.md`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Agentic development map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Agentic runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Query mode spec: `docs/modes/query-mode-spec.md`
- Query MCP task: `docs/modes/implementation-tasks.md`
- Rewrite engine docs: `docs/rewrite-engine.md`
- Automation policy: `docs/automation/agentic-development-orchestration.md`
- Review policy: `docs/automation/code-review-policy.md`
- MCP server: `src/mcp/server.rs`
- MCP graph tools: `src/mcp/tools/graph.rs`
- MCP build tools: `src/mcp/tools/build.rs`
- MCP deps tools: `src/mcp/tools/deps.rs`
- MCP intent tools: `src/mcp/tools/intent.rs`
- MCP rewrite tools: `src/mcp/tools/rewrite.rs`
- Query engine and answer metadata: `src/query/engine.rs`,
  `src/query/sources.rs`
- Session state: `src/session/mod.rs`
- Flagship example: `examples/flagship-http-sqlite-json/README.md`
- Flagship integration test: `tests/integration_duumbi688_flagship_example.rs`
