# DUUMBI-588: Define Repair-Agent Input Contract From Crash Evidence - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-588/PRODUCT.md` by
hardening the Phase 13 repair-agent input contract into a reviewable,
serializable, provider-independent context package.

The implementation must prove this flow:

```text
traced crash artifact + trace map + graph source ->
selected crash entry -> mapped function/block evidence ->
bounded graph context -> RepairCrashContext JSON ->
validation/test expectations attached
```

This issue does not generate repair patches, call a provider, apply graph
mutations, start autonomous retries, accept repairs, modify runtime assets, or
start Ralph cycles. The work stops at a deterministic input package that a
later Repair-agent proposal path can consume.

## Agent Audience

- Codex implementation agents running bounded Stage 10 Ralph cycles.
- Codex CLI or Codex App agents verifying Stage 10 implementation evidence.
- Rust telemetry/context agents working in `src/telemetry/mod.rs`.
- CLI agents adding or validating a local `duumbi telemetry repair-context`
  command surface.
- Reviewer and tester agents validating artifact parsing, graph context bounds,
  BDD coverage, and no-provider behavior.

## Source Context

- Product spec: `specs/DUUMBI-588/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/651
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/588#issuecomment-4606500965
- GitHub issue: https://github.com/hgahub/duumbi/issues/588
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/588#issuecomment-4606221781
- Stage 4 triage context:
  https://github.com/hgahub/duumbi/issues/588#issuecomment-4599034980
- Parent issue: https://github.com/hgahub/duumbi/issues/580
- Parent product spec: `specs/DUUMBI-580/PRODUCT.md`
- Parent technical spec: `specs/DUUMBI-580/TECHNICAL.md`
- Controlled crash proof spec: `specs/DUUMBI-586/TECHNICAL.md`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant code verified for Stage 8:

- `src/telemetry/mod.rs`
  - Already defines `CrashArtifact`, `TraceMap`, `TraceMapEntry`,
    `TraceCorrelation`, `RepairCrashContext`,
    `repair_crash_context_from_artifacts()`, `inspect_crash_artifacts()`,
    `repair_graph_context()`, default validation/test expectations, and repair
    validation evidence helpers.
  - Current `repair_crash_context_from_artifacts()` reads mapped crash evidence,
    uses the latest non-empty crash entry, maps function/block trace IDs through
    `trace_map.json`, sets `exact_node_id` to `None`, and builds a bounded
    context from trace-map metadata only.
  - Current context does not yet include schema version, selected crash entry
    provenance, explicit crash-entry selection, graph-source-backed bounded
    context, or stale graph ID detection.
  - `read_latest_crash()` already establishes a useful default: latest
    non-empty JSONL line from `crash_dump.jsonl`.
- `src/cli/mod.rs`
  - `TelemetrySubcommand::Inspect` exists with `--telemetry-dir`, `--crash`,
    and `--map`.
  - No `telemetry repair-context` command exists yet.
- `src/main.rs`
  - Dispatches `Commands::Telemetry` through `run_telemetry()`.
  - `run_telemetry()` currently prints human-readable inspect output from
    `telemetry::inspect_crash_artifacts()`.
- `tests/integration_telemetry.rs`
  - Builds traced controlled crash fixtures, runs them with isolated
    `DUUMBI_TELEMETRY_DIR`, verifies crash/trace/map artifacts, runs
    `duumbi telemetry inspect`, and checks graph function/block output plus
    exact-node-unavailable text.
  - Contains `traced_option_none_unwrap_writes_crash_evidence_and_inspects()`
    and `traced_call_then_panic_preserves_caller_context()`.
- `tests/fixtures/telemetry/option_none_unwrap.jsonld`
  - Controlled `OptionNone` + `OptionUnwrap` crash fixture.
- `tests/fixtures/telemetry/call_then_panic.jsonld`
  - Controlled call-then-panic fixture that proves caller context is preserved.
- `src/patch.rs`
  - Defines `GraphPatch`, `PatchOp`, and atomic JSON-LD patch application.
- `src/mcp/tools/graph.rs`
  - `graph_mutate()` applies patches and validates parse/build/graph state
    before writing.
  - `graph_validate()` is read-only validation.
- `src/agents/template.rs`
  - Includes a generic Repair template oriented to validation/test failures,
    not runtime crash-evidence context.
- `src/intent/execute.rs`
  - Contains verifier-failure repair precedent with provider calls and patch
    writes. That path is related precedent, not part of #588.

Relevant tests verified for Stage 8:

- `src/telemetry/mod.rs` unit tests already cover serializable
  `RepairCrashContext`, missing map rejection, repair validation evidence gates,
  invalid repair patch parse rejection, trace-map generation, trace-map
  collision checks, telemetry config defaults, and artifact path resolution.
- `tests/integration_telemetry.rs` already covers traced local crash artifacts
  and `telemetry inspect`.

Relevant Obsidian notes:

- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
  - Local developer/test runtime failure feedback is the first promise.
  - Repair must pass graph validation, rebuild, tests, and human review before
    acceptance.
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
  - The local Phase 13 foundation exists as traced builds, local trace/crash
    artifacts, function/block back-mapping, telemetry inspection, repair crash
    context, and validation evidence contracts.
  - Production customer self-healing is later work.
- `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 13 - Self-Healing & Telemetry.md`
  - Phase 13’s broad self-healing vision includes repair loops, but the current
    local slice must stay bounded and avoid hot-swap, production ingestion, and
    autonomous repair acceptance.

Verified source facts:

- Default untraced builds are separate from traced telemetry behavior.
- Current controlled traced crash tests use local temporary telemetry
  directories and do not call providers.
- Existing repair context is already serializable but too narrow for the
  approved #588 product spec because it lacks provenance and graph-source-backed
  stale-context checks.
- Exact node evidence remains unavailable in v1.
- Existing repair validation evidence is related to #587 and should not be
  expanded by #588 except for expectation strings included in the input context.

Assumptions for implementation:

- The canonical user-visible interface for a reviewable context package should
  be local CLI JSON output: `duumbi telemetry repair-context`.
- The existing library API should remain usable for unit tests and internal
  callers only through graph-source-aware context assembly options. Stage 10 may
  add a richer options struct and keep a latest-entry convenience wrapper when it
  can receive or delegate to supplied graph sources. The current
  `repair_crash_context_from_artifacts()` helper must not keep emitting
  trace-map-only context; if its signature cannot carry graph sources, deprecate
  or fail that helper with the same missing-graph-source error and update
  callers/tests accordingly.
- Graph-source-backed context can be built from one or more JSON-LD file paths
  supplied by CLI or tests. Workspace discovery may be added only if it remains
  local, deterministic, and bounded.

## Affected Areas

Expected implementation changes:

- Telemetry domain:
  - `src/telemetry/mod.rs`
    - Add a repair-context schema version constant.
    - Extend `RepairCrashContext` with schema/provenance fields.
    - Add a crash selection type for latest entry and explicit 1-based JSONL
      line selection.
    - Add repair-context assembly options that include telemetry directory,
      optional crash path, optional map path, crash selection, and graph source
      paths or parsed graph source values.
    - Update every successful context assembly entry point, including any retained
      latest-entry helper, so it requires or delegates to supplied graph sources.
    - Read the selected crash entry without losing line/provenance metadata.
    - Build bounded graph-source-backed context and fail closed on stale graph
      IDs.
    - Keep exact-node evidence as explicit `None` in v1.
    - Keep argument/runtime value snapshots absent.
- CLI:
  - `src/cli/mod.rs`
    - Add `duumbi telemetry repair-context` parser support.
    - Reuse `--telemetry-dir`, `--crash`, and `--map`.
    - Add `--graph <path>` as repeatable graph source input.
    - Add optional `--crash-entry <N>` for explicit 1-based JSONL line
      selection; omit it to use latest non-empty crash entry.
  - `src/main.rs`
    - Dispatch `TelemetrySubcommand::RepairContext`.
    - Print the serialized `RepairCrashContext` JSON to stdout.
    - Send human-readable errors to stderr through existing `anyhow` handling.
- Tests:
  - `src/telemetry/mod.rs` unit tests for context fields, provenance, latest
    selection, explicit line selection, missing/malformed/unmapped/untraced
    evidence, stale graph IDs, exact-node-unavailable behavior, and privacy
    defaults.
  - `src/cli/mod.rs` parser tests for `telemetry repair-context`.
  - `tests/integration_telemetry.rs` CLI E2E coverage using the existing
    controlled crash fixture.
- Existing fixtures:
  - Reuse `tests/fixtures/telemetry/option_none_unwrap.jsonld`.
  - Reuse `tests/fixtures/telemetry/call_then_panic.jsonld` when caller-context
    evidence is useful.
  - Add a new fixture only if Stage 10 proves existing fixtures cannot cover a
    required scenario.

Areas expected not to change:

- `specs/DUUMBI-588/PRODUCT.md`
- implementation code outside the affected areas listed above unless Stage 10
  first reports the reason and stays within this technical spec
- `runtime/duumbi_runtime.c`
- `runtime/duumbi_runtime.h`
- compiler lowering or trace hook behavior
- generated telemetry artifacts
- provider/model configuration
- Repair-agent provider calls
- MCP graph mutation behavior
- product spec approval labels or comments

CI and local validation paths:

- `cargo fmt --check`
- `git diff --check`
- `cargo test repair_crash_context --lib`
- `cargo test telemetry --lib`
- `cargo test telemetry_repair_context_parses` or equivalent CLI parser test
- `cargo test --test integration_telemetry`
- A local CLI smoke path using `target/debug/duumbi telemetry repair-context`

## Technical Approach

### 1. Preserve The Existing Context Boundary

Keep the existing public concept: `RepairCrashContext` is the agent-facing
input package derived from mapped telemetry evidence. Do not introduce a repair
proposal type, provider client, patch application path, or acceptance state in
#588.

Recommended additions:

```rust
pub const REPAIR_CONTEXT_SCHEMA_VERSION: &str =
    "duumbi.telemetry.repair_context.v1";

pub struct RepairCrashContext {
    pub schema_version: String,
    pub crash_message: String,
    pub function_id: String,
    pub block_id: String,
    pub exact_node_id: Option<String>,
    pub trace_ids: TraceCorrelation,
    pub graph_context: serde_json::Value,
    pub evidence: RepairContextEvidence,
    pub validation_expectations: Vec<String>,
    pub test_expectations: Vec<String>,
    pub human_review_required: bool,
}
```

The exact Rust type names may change if Stage 10 finds a better local naming
fit, but the serialized JSON must keep these product fields available and
reviewable.

### 2. Define Crash Selection

Stage 8 defines the product-spec crash-selection question this way:

- Default selection is the latest non-empty JSONL entry in the selected crash
  artifact. This matches current `read_latest_crash()` behavior.
- Explicit selection is a 1-based JSONL line number in the selected crash
  artifact.
- Empty lines do not count as valid crash entries.
- If an explicit line is empty, out of range, or malformed, fail closed with a
  clear telemetry error.
- The resulting context must identify the selected crash path and selected line
  number in its evidence/provenance.

Suggested types:

```rust
pub enum CrashEntrySelection {
    Latest,
    LineNumber(usize),
}

pub struct RepairContextEvidence {
    pub source: String,
    pub crash_path: String,
    pub map_path: String,
    pub selected_crash_line: usize,
    pub selection: String,
    pub graph_sources: Vec<String>,
}
```

Canonical serialized `RepairContextEvidence.selection` values:

- `latest` for `CrashEntrySelection::Latest`.
- `line:<N>` for `CrashEntrySelection::LineNumber(N)`, where `<N>` is the same
  1-based JSONL line stored in `selected_crash_line`.

Canonical serialized `RepairContextEvidence.source` value:

- `local_telemetry_artifacts` for contexts assembled from local
  `crash_dump.jsonl`, `trace_map.json`, and supplied graph source files through
  `duumbi telemetry repair-context` or equivalent library options.

### 3. Build Bounded Graph Context From Graph Source

Replace or augment the current trace-map-only `repair_graph_context()` with a
bounded context built from supplied graph source.

Required behavior:

- Parse one or more supplied JSON-LD graph source files as `serde_json::Value`.
- Find the mapped function `@id`.
- Find the mapped block `@id`.
- Include a bounded context object with:
  - source marker, for example `mapped_graph_source`.
  - mapped function trace ID and graph ID.
  - mapped block trace ID and graph ID.
  - function shell metadata: `@id`, `@type`, `duumbi:name`,
    `duumbi:returnType`, and the selected block only.
  - selected block JSON-LD, including its operations.
  - `exact_node_evidence: null`.
  - `context_limit: "containing_function_and_selected_block"`.
- Do not include all modules, unrelated functions, unrelated blocks, runtime
  arguments, heap values, stack values, environment variables, provider
  prompts, or credentials.

Fail closed when graph source is unavailable or stale:

- If no graph source is supplied, context assembly is not repair-ready. Report
  missing graph context rather than falling back to raw logs or trace-map-only
  context.
- If the function ID is absent from all supplied graph files, report stale or
  missing function graph context.
- If the block ID is absent from the mapped function, report stale or missing
  block graph context.
- If the mapped function `@id` appears in more than one supplied graph file,
  report ambiguous graph source and fail closed.
- If the mapped function `@id` appears more than once within a single supplied
  graph file, report ambiguous graph source and fail closed.
- If the mapped block `@id` appears in more than one candidate mapped function
  context, report ambiguous graph source and fail closed.
- If the mapped block `@id` appears more than once within the candidate mapped
  function context, report ambiguous graph source and fail closed.
- If the graph source is malformed JSON, report parse failure with the source
  path.

### 4. Preserve Existing Trace Mapping Checks

Keep the existing mapped-evidence requirements:

- Crash artifact must parse as `CrashArtifact`.
- `trace_active` must be true.
- Function trace ID must join a `TraceMapKind::Function` entry.
- Block trace ID must join a `TraceMapKind::Block` entry.
- Unmapped IDs return `TelemetryError::Unmapped` or a clearer equivalent.

Do not accept raw stderr, raw logs, or crash message strings as repair-ready
context.

### 5. Add A CLI Context Surface

Add a local command:

```text
duumbi telemetry repair-context \
  [--telemetry-dir <dir>] \
  [--crash <path>] \
  [--map <path>] \
  --graph <path> [--graph <path> ...] \
  [--crash-entry <line>]
```

Behavior:

- Omitting `--crash-entry` uses the latest non-empty crash entry.
- `--crash-entry <line>` selects a 1-based JSONL line from the selected crash
  artifact.
- `--graph` is required for this command because #588 requires graph-source
  backed bounded context and stale ID detection.
- Output is pretty JSON on stdout.
- Errors remain human-readable through existing CLI error handling.
- The command must not call a provider, generate a patch, apply a patch, mutate
  graph source, or write telemetry artifacts.

Rejected alternatives:

- Do not make `telemetry inspect` output repair context. It is already a
  human-readable map inspection command.
- Do not create `duumbi heal`, `duumbi repair`, or a Repair-agent execution
  command in #588. Those cross into later repair proposal and validation work.
- Do not use MCP graph mutation to build context. Context assembly must be
  read-only.

### 6. Keep Repair Validation Separate

`RepairValidationEvidence` and `parse_repair_graph_patch()` already exist in
`src/telemetry/mod.rs`, but #588 should only attach validation/test
expectations to the input context. Do not broaden #588 into #587 by validating
candidate patches or producing repair success evidence.

## Invariants

- Product spec `specs/DUUMBI-588/PRODUCT.md` remains unchanged.
- Technical spec implementation does not modify runtime assets, compiler trace
  hooks, generated telemetry artifacts, provider setup, MCP mutation behavior,
  or product approval workflow.
- Default untraced runs do not produce repair context.
- Raw logs alone never produce repair-ready context.
- Context assembly is local, deterministic, and read-only.
- Context assembly requires mapped crash evidence and graph source context.
- Context assembly does not invoke OpenAI, Anthropic, OpenRouter, Minimax, Grok,
  Slack, GitHub, Studio, remote telemetry collectors, or any network service.
- Runtime values, arguments, heap snapshots, stack snapshots, environment
  variables, and credentials are absent from the context.
- Exact node evidence remains `None` or JSON `null` in v1.
- Existing graph validation and mutation boundaries are not bypassed.
- Any later repair candidate remains unaccepted until separate validation,
  rebuild, tests, and human review.

## BDD-To-Test Mapping

| Product BDD Scenario | Required Technical Evidence |
| --- | --- |
| Controlled crash becomes repair context | Unit test assembles `RepairCrashContext` from a crash artifact, trace map, and graph fixture; asserts schema version, crash message, trace IDs, graph function/block IDs, bounded graph context, `evidence.source` value `local_telemetry_artifacts`, provenance, validation expectations, and test expectations. Integration test runs `duumbi telemetry repair-context --telemetry-dir <dir> --graph tests/fixtures/telemetry/option_none_unwrap.jsonld` after a traced crash and asserts equivalent JSON fields. |
| Raw logs are not repair-ready | Unit test calls the context API without crash/map artifacts and asserts missing evidence. If `repair_crash_context_from_artifacts()` is retained, a unit test invokes it without graph source input and asserts the missing-graph-source/deprecated-helper error rather than a trace-map-only context. Review evidence confirms no API accepts raw stderr/log text as a repair context input and no provider call is made. |
| Unmapped trace IDs block repair readiness | Unit test writes a crash artifact whose function or block trace ID does not join the trace map; expects `TelemetryError::Unmapped` or equivalent and no context. |
| Exact node evidence is unavailable in v1 | Unit and CLI JSON tests assert `exact_node_id` is `None`/`null` and graph context contains `exact_node_evidence: null`. |
| Argument values are omitted by default | Serialization test asserts the context JSON has no argument, runtime value, heap, stack, or value snapshot fields. Review evidence confirms runtime crash artifact fields were not expanded for value capture. |
| Stale graph IDs prevent a misleading context | Unit test supplies a graph source missing the mapped function or block and asserts stale/missing graph context error. Add separate function-missing and block-missing tests if one combined test is not clear enough. |
| Duplicate graph IDs prevent ambiguous context | Unit test supplies two graph source files with the mapped function or block `@id` duplicated and asserts ambiguous graph source error instead of first-match or last-match context assembly. A separate unit test or fixture variant duplicates the mapped function or block `@id` within one supplied graph file and asserts the same fail-closed behavior. |
| Default crash selection remains traceable | Unit test writes two valid crash JSONL entries and no explicit `--crash-entry`; asserts the latest non-empty line is selected, `evidence.selected_crash_line` points to that line, and `evidence.selection` is `latest`. CLI E2E may cover this with a synthetic artifact if practical. |
| Explicit crash selection remains traceable | Unit test writes two valid crash JSONL entries and selects the first with `CrashEntrySelection::LineNumber(1)` or CLI `--crash-entry 1`; asserts the selected crash message, provenance line, and `evidence.selection` value `line:1`. |
| Repair agent receives validation expectations | Unit test asserts context includes expected validation/test strings: `GraphPatch` parse, atomic patch behavior, graph parse/build, graph validation, native rebuild, relevant tests, controlled crash reproducibility, targeted regression, and default untraced behavior unchanged. The same test asserts `human_review_required` is `true` so a later Repair agent cannot treat generated patch-shaped output as accepted without human review. |
| Patch-shaped output is not accepted automatically | Review evidence plus unit tests confirm #588 does not set `accepted_for_application`, apply `GraphPatch`, write graph files, or call `repair_validation_evidence_from_graph_patch()` as part of context assembly. |
| Graph mutation cannot bypass validation | Review evidence confirms context assembly is read-only and any later patch remains constrained to existing `GraphPatch`/MCP validation boundaries. No #588 test should mutate `.duumbi/graph`. |
| Production self-healing is requested from this contract | Review evidence confirms no `duumbi heal`, production ingestion, remote telemetry, hot-swap, Studio dashboard, or autonomous repair loop is added. |
| Provider execution is requested before context evidence exists | Review evidence confirms no provider call path is introduced. Tests run with zero provider credentials and zero external LLM calls. |

Scenarios that depend on repair proposal generation or human acceptance are
verified by review evidence in #588 and deferred to later approved issues.

## Live E2E Plan

This issue names a Repair-agent input contract but deliberately does not execute
an LLM. The live E2E plan is therefore local CLI/runtime evidence with zero
external LLM calls. A live provider-backed E2E is not required and would violate
the approved #588 boundary if it generated or applied a repair proposal.

Canonical interface:

- CLI: `duumbi telemetry repair-context`.

Required credentials and environment:

- No provider credentials.
- No network.
- `DUUMBI_TELEMETRY_DIR` set to a temporary directory for the traced fixture.

Expected external LLM calls:

- `0`.

Estimated external LLM cost:

- `USD 0`.

Canonical Unix-like smoke path:

```sh
cargo build
tmp="$(mktemp -d)"
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" \
  target/debug/duumbi build --trace \
  tests/fixtures/telemetry/option_none_unwrap.jsonld \
  -o "$tmp/panic-fixture"
set +e
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" "$tmp/panic-fixture"
status="$?"
set -e
test "$status" -ne 0
target/debug/duumbi telemetry repair-context \
  --telemetry-dir "$tmp/telemetry" \
  --graph tests/fixtures/telemetry/option_none_unwrap.jsonld
```

Pass criteria:

- traced fixture exits nonzero.
- stderr includes `duumbi panic: called Option::unwrap() on a None value`.
- `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` exist under the
  temp telemetry directory.
- `telemetry repair-context` exits successfully.
- stdout parses as JSON.
- JSON includes:
  - `schema_version`
  - `crash_message`
  - `function_id`
  - `block_id`
  - `exact_node_id: null`
  - `trace_ids.function_trace_id`
  - `trace_ids.block_trace_id`
  - `graph_context.context_limit`
  - `evidence.source: "local_telemetry_artifacts"`
  - `evidence.selected_crash_line`
  - `evidence.selection`
  - validation expectations
  - test expectations
- stdout does not include runtime argument/value/heap/stack snapshot fields.

Cross-platform canonical evidence:

- `cargo test --test integration_telemetry`, because the Rust test harness
  handles temp paths and executable suffixes.

Failure criteria:

- command accepts raw logs without artifacts.
- command succeeds without graph source.
- command picks first-match or last-match context when duplicate mapped graph
  `@id` values are supplied.
- command calls a provider.
- command writes or mutates graph source.
- command omits provenance or selected crash entry evidence.
- command claims exact node evidence when none exists.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. summarize current state and remaining unmet #588 requirements.
2. propose one bounded implementation goal.
3. list intended file areas and commands before editing.
4. estimate external LLM calls, external LLM cost, command budget, and risk.
5. check whether the resource gate requires human approval.
6. implement only the approved or resource-permitted goal.
7. run the agreed checks.
8. report evidence, failures, cost, and remaining gaps.
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

Allowed autonomous cycle goals:

- Cycle goal option A: repair context schema/provenance and crash selection
  unit tests in `src/telemetry/mod.rs`.
- Cycle goal option B: graph-source-backed bounded context and stale graph ID
  tests in `src/telemetry/mod.rs`.
- Cycle goal option C: `duumbi telemetry repair-context` CLI parser/dispatch and
  integration telemetry coverage.
- Cycle goal option D: final focused regression, local CLI smoke, and review
  evidence cleanup.

The implementation agent may combine small substeps only when the resource
policy stays below thresholds and the resulting diff remains reviewable.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: at most 3 source/test files from the approved
  implementation pool, excluding unchanged fixtures. The approved pool is
  `src/telemetry/mod.rs`, `src/cli/mod.rs`, `src/main.rs`, and
  `tests/integration_telemetry.rs`; touching 4 or more files in one cycle, or
  touching any file outside that pool, needs coordinator approval unless the
  extra file is a focused test fixture.
- Expected command budget per low-budget cycle: up to 6 focused commands.
- Expected external LLM calls per cycle: 0.
- Estimated external LLM cost per cycle: USD 0.
- Human approval required when planned external LLM usage exceeds USD 2,
  exceeds 10 calls, exceeds approved scope, modifies runtime assets, modifies
  compiler lowering, changes provider behavior, changes MCP mutation behavior,
  adds risky dependencies, performs migrations, writes generated artifacts,
  needs irreversible operations, hits a security/privacy decision, or needs a
  product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 3 low-budget cycles before pausing with evidence and
  remaining work.
- When to stop and ask for human guidance:
  - implementation requires provider calls.
  - implementation requires repair patch generation or application.
  - graph-source context would require storing runtime values.
  - CLI surface needs a different command name than `telemetry repair-context`.
  - stale graph detection cannot be implemented without a broader workspace
    indexing design.
  - exact node evidence becomes necessary to satisfy a test.
  - any check repeatedly fails after two focused fix attempts.

## Task Breakdown

1. Repair context schema and provenance:
   - Add `REPAIR_CONTEXT_SCHEMA_VERSION`.
   - Extend `RepairCrashContext` with schema and evidence/provenance fields.
   - Add crash selection types and selected-line metadata.
   - Keep a latest-entry helper only when it delegates to graph-source-aware
     options; otherwise deprecate or fail the old helper and update callers/tests.
   - Update existing serialization tests.

2. Crash selection and failure states:
   - Implement latest non-empty entry selection.
   - Implement explicit 1-based line selection.
   - Add tests for empty artifact, malformed selected line, out-of-range line,
     untraced crash, and unmapped IDs.

3. Graph-source-backed bounded context:
   - Add helpers that parse supplied graph source files.
   - Find mapped function and block IDs.
   - Return a bounded function/block context object.
   - Add stale function and stale block tests.
   - Assert exact node evidence remains unavailable in v1.

4. CLI repair context surface:
   - Add `TelemetrySubcommand::RepairContext`.
   - Add parser test.
   - Dispatch in `run_telemetry()`.
   - Serialize context JSON to stdout.
   - Require at least one `--graph`.

5. Integration and smoke evidence:
   - Extend `tests/integration_telemetry.rs` to run the controlled traced
     fixture and invoke `telemetry repair-context`.
   - Assert required JSON fields and privacy defaults.
   - Keep existing `telemetry inspect` tests intact.

6. Final regression and review:
   - Run focused tests and `cargo fmt --check`.
   - Run `git diff --check`.
   - Run the local CLI smoke when practical.
   - Confirm no product specs, runtime assets, generated artifacts, providers,
     repair generation, or graph mutation behavior changed.

Independently executable slices:

- Schema/provenance and selection unit tests can land before CLI work.
- Bounded graph context can land before integration E2E if unit tests cover it.
- CLI work can be implemented after the library contract is stable.

## Verification Plan

Required local checks for implementation PR readiness:

- `cargo fmt --check`
- `git diff --check`
- `cargo test repair_crash_context --lib`
- `cargo test telemetry --lib`
- CLI parser test for `telemetry repair-context`
- `cargo test --test integration_telemetry`

Required review evidence:

- Diff review shows only approved implementation areas changed.
- No product spec edits.
- No runtime asset edits.
- No generated telemetry artifact commits.
- No provider call path.
- No repair patch generation/application path.
- No graph mutation path in context assembly.
- Context JSON includes provenance and selected crash entry.
- Context JSON does not include runtime values or argument snapshots.

Manual/local smoke evidence:

- `target/debug/duumbi telemetry repair-context --telemetry-dir <tmp> --graph
  tests/fixtures/telemetry/option_none_unwrap.jsonld` after a traced crash.
- Paste or summarize stdout fields in the Stage 10 evidence report.

CI expectation:

- Existing repository CI should remain green because the diff is source/test
  only and uses local temp directories.

## Completion Criteria

Stage 10 implementation is complete for #588 only when:

- `RepairCrashContext` serializes with schema version and evidence provenance.
- Context assembly requires mapped crash evidence.
- Context assembly rejects missing, malformed, untraced, unmapped, and stale
  graph evidence.
- Context assembly rejects ambiguous graph source matches when duplicate mapped
  function or block `@id` values appear across supplied graph files or within a
  single supplied graph file.
- Default crash selection uses latest non-empty crash JSONL entry.
- Explicit crash selection supports a 1-based crash JSONL line.
- Context assembly requires graph source and builds bounded context from the
  mapped function/block.
- No successful context assembly path, including retained legacy helpers, emits a
  trace-map-only context without supplied graph sources.
- `RepairContextEvidence.source` serializes as `local_telemetry_artifacts` for
  locally assembled contexts.
- Exact node evidence remains explicitly unavailable in v1.
- Runtime values and argument snapshots remain absent.
- `duumbi telemetry repair-context` emits reviewable JSON.
- The controlled crash fixture proves the CLI path without provider calls.
- Validation/test expectations are present in the context.
- No repair patch is generated, applied, validated as successful, or accepted.
- No implementation scope from #587, production self-healing, hot-swap, Studio
  UI, remote telemetry, or autonomous repair loops is added.
- Required focused checks pass.
- Stage 11 implementation review has traceable evidence for every product BDD
  scenario listed in this spec.

## Failure And Escalation

- If graph source lookup cannot find a mapped function or block, fail closed
  with stale/missing graph context; do not emit a partial prompt.
- If crash selection is ambiguous, fail closed unless the default latest-entry
  rule or explicit line selection applies.
- If artifact parsing fails, report the exact artifact path and parse context.
- If tests require runtime values or exact node IDs, stop and request product
  guidance because that exceeds #588 v1.
- If implementation requires provider calls, stop; #588 is a local contract.
- If implementation requires changing runtime C files, compiler trace hooks, or
  telemetry artifact generation, stop and route to a separate accepted issue.
- If a CLI command name or UX trade-off becomes contentious, stop and ask for a
  human product decision.
- If a focused command fails because of a real code issue, fix within the same
  bounded cycle when possible. After two failed fix attempts on the same
  failure, stop and report the blocker.
- If resource thresholds are exceeded, stop before further work.

## Open Questions

No blocking open questions for Stage 10 implementation.

Resolved Stage 8 decisions:

- Default crash selection is latest non-empty JSONL entry.
- Explicit crash selection is a 1-based JSONL line in the selected crash file.
- Bounded graph context is the containing function shell plus selected block
  JSON-LD.
- Exact node evidence remains optional and unavailable in v1.
- Runtime value snapshots are out of scope for #588.
- Stale graph IDs are detected against supplied graph source paths.
