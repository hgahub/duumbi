# DUUMBI-584: Emit Function And Block Trace Events From Compiled Graphs - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-584/PRODUCT.md` by making
function/block trace events an explicit, tested Stage 10 implementation slice.

The accepted product behavior is:

```text
explicit traced build -> graph-linked function/block enter and exit events ->
local trace evidence that can be correlated with graph function/block metadata
```

This technical spec is intentionally narrower than the Phase 13 parent #580. It
must cover traced function/block event emission, stable graph-linked trace IDs,
runtime hook ABI behavior, local trace event output, and default-build
non-instrumentation. It must not add per-operation tracing, remote telemetry,
Studio UI, repair-agent behavior, production crash ingestion, or autonomous
repair.

Verified source state at Stage 8: `main` already contains Phase 13 telemetry
foundation from #580 and #583, including `TelemetryBuildMode`, trace-map
helpers, runtime trace hooks, compiler trace-call emission, and telemetry
integration tests. Stage 10 agents must first audit this baseline against the
#584 product contract, then make only the smallest remaining changes required
to satisfy this technical spec.

## Agent Audience

- Codex implementation agents running bounded Stage 10 Ralph cycles.
- Oz or GitHub-routed implementation agents using this spec from the issue.
- Rust compiler/runtime agents working in Cranelift lowering and the C runtime.
- Stage 9 technical spec reviewers checking feasibility, scope, and evidence.
- Stage 10 tester/reviewer agents validating trace event artifacts and default
  non-instrumentation.

## Source Context

- Product spec: `specs/DUUMBI-584/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/628
- GitHub issue: https://github.com/hgahub/duumbi/issues/584
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/584#issuecomment-4547784393
- Stage 6 product spec link:
  https://github.com/hgahub/duumbi/issues/584#issuecomment-4541710946
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/584#issuecomment-4540367441
- Stage 4 triage:
  https://github.com/hgahub/duumbi/issues/584#issuecomment-4535968301
- Parent product spec: `specs/DUUMBI-580/PRODUCT.md`
- Parent technical sequencing context: `specs/DUUMBI-580/TECHNICAL.md`
- Traced build/config technical spec: `specs/DUUMBI-583/TECHNICAL.md`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant verified source facts:

- `src/telemetry/mod.rs`
  - Defines `TelemetryBuildMode::{Off, Trace}` and `BuildOptions`.
  - Defines `TraceMapKind::{Function, Block}`, `TraceMap`, `TraceMapEntry`,
    `trace_id()`, `function_trace_graph_id()`, and
    `block_trace_graph_id()`.
  - Derives trace IDs from graph identity with a `duumbi-trace-v1` SHA-256
    domain and truncates them to signed 63-bit-compatible values.
  - Writes `trace_map.json` and validates trace ID collision behavior.
  - Computes local telemetry artifact directories through config and
    `DUUMBI_TELEMETRY_DIR`.
- `src/cli/mod.rs`, `src/main.rs`, and `src/cli/commands.rs`
  - Expose `duumbi build --trace`.
  - Select `TelemetryBuildMode::Trace` only when `--trace` is provided.
  - Keep default `duumbi build` on `TelemetryBuildMode::Off`.
  - Single-file traced builds compile with telemetry and write a trace map.
- `src/workspace.rs`
  - Provides `build_workspace_with_options()`.
  - Compiles workspace modules with the selected telemetry build mode.
  - Writes a workspace trace map only when telemetry mode is trace.
- `src/compiler/lowering.rs`
  - Declares trace runtime functions only when `TelemetryBuildMode::Trace` is
    selected.
  - Emits `duumbi_trace_init()` for `main`.
  - Emits function enter/exit and block enter/exit calls in traced mode.
  - Keeps Cranelift-specific types inside `src/compiler/`.
  - Has focused tests proving default objects do not reference trace hooks and
    traced objects reference trace hooks.
- `runtime/duumbi_runtime.c`
  - Defines `duumbi_trace_init()`, `duumbi_trace_function_enter()`,
    `duumbi_trace_function_exit()`, `duumbi_trace_block_enter()`,
    `duumbi_trace_block_exit()`, and `duumbi_trace_panic()`.
  - Uses thread-local current function/block trace state.
  - Writes local `traces.jsonl` events with schema version
    `duumbi.telemetry.trace.v1`.
  - Preserves the existing `duumbi panic: <message>` stderr behavior.
  - Prints telemetry warnings on artifact path/open failures instead of
    replacing original program behavior.
- `runtime/duumbi_runtime.h`
  - Declares the trace hook ABI used by generated code.
- `tests/integration_telemetry.rs`
  - Builds traced controlled-panic fixtures.
  - Runs compiled binaries with `DUUMBI_TELEMETRY_DIR`.
  - Asserts `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` exist.
  - Verifies mapped function/block context through `duumbi telemetry inspect`.
- `tests/integration_phase1.rs`
  - Covers `duumbi build --trace` CLI behavior.
  - Verifies workspace traced build supports `--offline`.
  - Verifies traced builds do not write runtime `traces.jsonl` or
    `crash_dump.jsonl` during build.

Relevant Obsidian notes:

- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Runtime Failure Feedback Loop.md`
- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 13 - Self-Healing & Telemetry.md`

Assumptions for implementation:

- The current telemetry foundation may already satisfy much of #584, but Stage
  10 must still produce issue-specific evidence against this spec.
- The implementation may refine existing trace event behavior, tests, or docs
  only when a gap is verified against the approved product spec.
- Local trace event files are test artifacts and must not be committed.
- Exact node-level mapping remains unavailable in v1 unless a later accepted
  issue changes the product contract.

## Affected Areas

Expected implementation areas if gaps remain:

- Compiler lowering:
  - `src/compiler/lowering.rs`
- Telemetry domain:
  - `src/telemetry/mod.rs`
- Runtime trace ABI and local writer:
  - `runtime/duumbi_runtime.c`
  - `runtime/duumbi_runtime.h`
- Build path plumbing only if existing behavior is incomplete:
  - `src/cli/commands.rs`
  - `src/workspace.rs`
  - `src/main.rs`
- Tests and fixtures:
  - `src/compiler/lowering.rs` unit tests.
  - `src/telemetry/mod.rs` unit tests.
  - `tests/integration_telemetry.rs`.
  - `tests/integration_phase1.rs` or a narrow new integration file.
  - existing telemetry fixtures under `tests/fixtures/telemetry/`.
- Documentation/help only if the event shape or behavior is not otherwise
  documented:
  - `docs/architecture.md` or a telemetry-focused doc.

Areas expected not to change for #584:

- `specs/DUUMBI-584/PRODUCT.md`
- `specs/DUUMBI-580/PRODUCT.md`
- `specs/DUUMBI-580/TECHNICAL.md`
- `specs/DUUMBI-583/PRODUCT.md`
- `specs/DUUMBI-583/TECHNICAL.md`
- Provider/model setup, Query mode, registry behavior, Studio dashboards,
  remote telemetry/export paths, repair-agent behavior, repair validation, and
  production crash ingestion.
- Generated binaries, object files, `traces.jsonl`, `trace_map.json`,
  `crash_dump.jsonl`, or other generated telemetry artifacts.

CI and local validation paths:

- `cargo fmt --check`
- `git diff --check`
- focused compiler/lowering tests for default and traced object behavior
- focused telemetry unit tests for trace IDs and trace maps
- `cargo test --test integration_telemetry`
- targeted CLI/workspace traced-build regression from `tests/integration_phase1.rs`
- `cargo test --all` before implementation PR review if compiler, runtime,
  workspace, or telemetry shared behavior changes

## Technical Approach

### 1. Start With A Baseline Audit

Before changing code, the Stage 10 agent must map current behavior to the
approved #584 BDD scenarios:

- inspect `src/compiler/lowering.rs` trace emission points.
- inspect `src/telemetry/mod.rs` trace ID and trace map behavior.
- inspect `runtime/duumbi_runtime.c` trace hook ABI and event output.
- inspect `tests/integration_telemetry.rs` and `tests/integration_phase1.rs`.
- run focused checks where practical before editing.

If the baseline already satisfies a requirement, record evidence instead of
rewriting it. If a gap exists, make the smallest scoped change needed.

### 2. Preserve Build-Mode Boundaries

Required behavior:

- `TelemetryBuildMode::Off` must remain the default for all existing build
  paths.
- `duumbi build --trace` must remain the only accepted v1 user-facing traced
  build selector.
- `[telemetry] enabled = true` must not instrument default builds by itself.
- `duumbi run` must not silently rebuild or change trace mode.
- Studio and REPL build paths must keep existing default uninstrumented
  behavior unless a later accepted issue adds trace UI or command parity.

Implementation guidance:

- Keep the selected telemetry build mode in `BuildOptions`.
- Keep Cranelift-specific implementation details inside `src/compiler/`.
- Keep telemetry config/path/trace-map concepts in `src/telemetry/`.

### 3. Runtime Trace ABI

The v1 trace ABI should remain:

```c
void duumbi_trace_init(void);
void duumbi_trace_function_enter(int64_t function_id);
void duumbi_trace_function_exit(int64_t function_id);
void duumbi_trace_block_enter(int64_t block_id);
void duumbi_trace_block_exit(int64_t block_id);
```

`duumbi_trace_panic(const char *msg)` may remain an internal runtime panic
integration point, but #584 implementation evidence must focus on function and
block enter/exit events.

Required runtime behavior:

- Maintain thread-local current function and block trace IDs.
- Emit local trace events only after `duumbi_trace_init()` activates tracing.
- Write events to `traces.jsonl` under the resolved local telemetry directory.
- Use event names `function_enter`, `function_exit`, `block_enter`, and
  `block_exit`.
- Include `schema_version`, `event`, `trace_id`, and `timestamp_ns` in v1
  event lines.
- Keep `timestamp_ns = 0` acceptable for v1 unless Stage 10 needs real ordering
  semantics for a test.
- Report telemetry write/path failures to stderr as warnings and preserve
  original program output or failure behavior.
- Do not capture runtime values, function arguments, heap snapshots, stack
  snapshots, or secrets.

### 4. Compiler Instrumentation

Required compiler behavior:

- Declare trace runtime symbols only in traced mode.
- In default mode, object bytes must not reference `duumbi_trace_init`,
  `duumbi_trace_function_enter`, `duumbi_trace_function_exit`,
  `duumbi_trace_block_enter`, or `duumbi_trace_block_exit`.
- In traced mode, emit `duumbi_trace_init()` once at the start of `main`.
- Emit function enter once at function entry.
- Emit function exit before each normal return.
- Emit block enter when execution enters a compiled graph block.
- Emit block exit before normal branch/return transitions where possible.
- Preserve SSA correctness, dominance, terminator placement, and return value
  behavior.
- Preserve auto-drop behavior before returns.

Block-exit decision:

- For v1, branch and return paths must emit block-exit events before the
  terminator or return.
- If a future unsupported terminator or control-flow form makes perfect
  block-exit coverage unsafe, the implementation must document the limitation,
  keep block-enter and crash-time current block context correct, and add a test
  showing the accepted limitation.

### 5. Stable Graph-Linked Trace IDs

Required trace ID behavior:

- Derive function trace IDs from function graph identity.
- Derive block trace IDs from block graph identity.
- Keep trace ID generation deterministic for the same graph identity.
- Do not derive trace IDs from compiler traversal order, block index alone, or
  non-stable memory addresses.
- Keep IDs compatible with the signed `int64_t` runtime ABI.
- Detect trace ID collisions when building the trace map.

Recommended current strategy:

- Continue using the existing `duumbi-trace-v1` SHA-256 domain in
  `src/telemetry/mod.rs`.
- Continue deriving graph identity from graph `@id` parent segments when node
  IDs are available, with deterministic module/function/block fallback IDs.
- Keep `trace_map.json` as the join artifact for later #585/#586 work.

### 6. Event Shape And Local Artifacts

The accepted v1 function/block trace event line is:

```json
{"schema_version":"duumbi.telemetry.trace.v1","event":"function_enter","trace_id":123,"timestamp_ns":0}
```

Required artifacts for #584 evidence:

- traced runtime execution writes `traces.jsonl`.
- traced build writes or preserves a matching `trace_map.json` when the build
  path already owns trace-map generation.
- default build/run does not write `traces.jsonl` or require trace hooks.

Do not expand #584 into crash dump or inspection scope. Existing crash/inspect
tests may be used as integration evidence only because they exercise the trace
event stream and active function/block context.

### 7. Documentation

If implementation changes or clarifies behavior, update only minimal docs:

- event shape.
- local artifact path.
- default-off behavior.
- function/block granularity.

Do not update product specs or create generated artifacts.

## Invariants

- Default builds remain uninstrumented.
- Traced behavior is explicit and local-only.
- Trace events are function/block-level only.
- No per-operation, value, argument, heap, stack, remote export, Studio UI, or
  repair behavior is added for #584.
- Trace IDs preserve graph identity and are deterministic for unchanged graph
  identities.
- Telemetry write failures do not hide original program behavior.
- Compiler instrumentation does not change program semantics except local trace
  side effects in traced mode.
- The execution issue remains open after the technical spec PR; Stage 9 and
  Stage 10 are still required.

## BDD-To-Test Mapping

| Product BDD scenario | Required verification evidence |
| --- | --- |
| Default build does not emit trace events | Integration evidence from `tests/integration_phase1.rs` or a new focused test: run default `duumbi build`, run the binary, assert no `traces.jsonl` is produced under the telemetry directory. |
| Default compiled output does not reference trace hooks | Existing or expanded `src/compiler/lowering.rs` unit test: compile with `compile_to_object()` and assert object bytes do not contain `duumbi_trace_init`, `duumbi_trace_function_enter`, `duumbi_trace_function_exit`, `duumbi_trace_block_enter`, or `duumbi_trace_block_exit`. |
| Traced run emits function enter and exit events | Integration test: build a simple fixture with `duumbi build --trace`, run it with `DUUMBI_TELEMETRY_DIR`, parse `traces.jsonl`, and assert matching `function_enter` and `function_exit` events whose trace IDs exist as function entries in `trace_map.json`. |
| Traced run emits block enter and exit events | Integration test: parse `traces.jsonl` from the traced run and assert matching `block_enter` and `block_exit` events whose trace IDs exist as block entries in `trace_map.json`. |
| Trace IDs are stable across equivalent traced builds | Unit test in `src/telemetry/mod.rs`: assert repeated `trace_id(TraceMapKind::Function, graph_id)` and `trace_id(TraceMapKind::Block, graph_id)` calls return the same values; optionally add an integration comparison across two traced builds of the same fixture if not already covered. |
| Trace IDs do not depend on compiler traversal order | Unit/review evidence: trace ID helper must use graph identity strings, not traversal indices. Add a unit test with two manually ordered `TraceMapEntry` collections or graph fixtures if a concrete ordering regression is feasible. |
| Trace event emission does not require external services | Integration evidence: traced build/run tests execute with local `DUUMBI_TELEMETRY_DIR` and no provider credentials, Studio, network service, or collector. Record this in the Ralph evidence report. |
| Trace artifact write failure does not hide original behavior | Runtime-focused test or manual smoke: run a traced binary with `DUUMBI_TELEMETRY_DIR` set to an unwritable or invalid path, assert stderr includes a telemetry warning and the original program output or panic behavior remains visible. If this cannot be made portable across macOS/Linux/Windows CI, document manual evidence and keep unit coverage for runtime warning paths if practical. |
| Traced runtime records active context for a later crash | Existing `tests/integration_telemetry.rs` crash fixture evidence: traced panic writes trace/crash artifacts and `duumbi telemetry inspect` maps function/block context. Ensure the test asserts caller/current context expected by #584. |
| Missing runtime hooks fail before false trace success | Compile/link evidence: traced object references trace hook symbols, and normal link with `runtime/duumbi_runtime.c` succeeds. Negative missing-hook testing may be review evidence only unless a portable linker failure test is cheap. |
| Missing graph identity prevents traced instrumentation | Unit/review evidence: current fallback graph IDs are deterministic when node-derived IDs are unavailable. If product reviewers require hard failure instead of deterministic fallback, stop for a product decision before changing behavior. |

## Live E2E Plan

Canonical interface: CLI.

This issue does not touch LLM provider behavior. No live LLM-backed E2E is
required because the product behavior is compiler/runtime telemetry, not agent
generation or provider routing.

Required local live E2E path:

1. Build the CLI:
   ```text
   cargo build
   ```
2. Build a simple fixture without tracing:
   ```text
   target/debug/duumbi build tests/fixtures/add.jsonld -o /tmp/duumbi-add-default
   ```
3. Run the default binary with `DUUMBI_TELEMETRY_DIR` pointing at a temp
   directory and verify no `traces.jsonl` is required or produced.
4. Build a simple fixture with tracing:
   ```text
   DUUMBI_TELEMETRY_DIR=/tmp/duumbi-telemetry \
     target/debug/duumbi build --trace tests/fixtures/add.jsonld -o /tmp/duumbi-add-traced
   ```
5. Run the traced binary:
   ```text
   DUUMBI_TELEMETRY_DIR=/tmp/duumbi-telemetry /tmp/duumbi-add-traced
   ```
6. Inspect `/tmp/duumbi-telemetry/traces.jsonl` and
   `/tmp/duumbi-telemetry/trace_map.json`.

Pass criteria:

- default build/run does not emit trace events.
- traced run emits at least one `function_enter`, `function_exit`,
  `block_enter`, and `block_exit` event.
- every event `trace_id` joins to a matching function or block entry in
  `trace_map.json`.
- no provider credentials, network service, Studio server, external telemetry
  collector, or remote export is required.

External LLM calls: 0.

Estimated external LLM cost: USD 0.

Artifacts to report, not commit:

- command output.
- temp directory path.
- selected `traces.jsonl` event kinds and trace IDs.
- selected `trace_map.json` matching entries.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. summarize the current state and remaining unmet #584 requirements.
2. propose one bounded implementation goal.
3. list intended file areas and commands before editing.
4. estimate resource use, external LLM calls, command count, and risk.
5. check whether the resource gate requires human approval.
6. implement only the approved or resource-permitted goal.
7. run the agreed checks.
8. report evidence, failures, and remaining gaps in the GitHub issue or PR.
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 3 source modules plus directly related tests,
  unless the cycle is documentation-only.
- Expected command budget per cycle: up to 6 focused commands.
- Human approval required when planned external LLM usage exceeds USD 2,
  exceeds 10 external LLM calls, exceeds approved scope, adds dependencies,
  changes public telemetry semantics, changes product behavior, performs
  irreversible operations, or requires a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 3 low-budget cycles before stopping for human review,
  even if all resource thresholds remain below limits.
- When to stop and ask for human guidance:
  - product behavior conflicts with this technical spec.
  - exact node-level tracing appears necessary.
  - remote telemetry, Studio UI, repair behavior, or value capture becomes
    tempting to satisfy a test.
  - trace ID fallback versus hard failure needs a product decision.
  - portable write-failure coverage cannot be designed without brittle CI
    behavior.

## Task Breakdown

1. Baseline audit:
   - compare current compiler/runtime/telemetry behavior against the BDD mapping.
   - run focused tests that already cover trace IDs, trace hook references, and
     telemetry integration.
   - record which requirements are already satisfied.
2. Trace event verification hardening:
   - if missing, add parsing assertions for `traces.jsonl` event kinds and
     trace IDs in `tests/integration_telemetry.rs` or a narrow new integration
     test.
   - verify event IDs join with `trace_map.json`.
3. Default-build non-instrumentation hardening:
   - ensure default object tests cover all trace hook symbols, not only a subset.
   - ensure default build/run integration does not create runtime trace events.
4. Runtime warning behavior:
   - add portable test or manual evidence for artifact write failure preserving
     program behavior.
5. Documentation:
   - update minimal documentation only if current docs do not describe event
     shape, artifact path, or default-off behavior.
6. Regression:
   - run focused checks, then broader regression if shared compiler/runtime
     behavior changed.

## Verification Plan

Required before Stage 10 implementation PR review:

- `cargo fmt --check`
- `git diff --check`
- `cargo test trace_hooks --lib`
- `cargo test telemetry --lib`
- `cargo test --test integration_telemetry`
- targeted `tests/integration_phase1.rs` traced/default build tests, either by
  exact test name or a focused filter
- live CLI E2E from this spec, unless the integration test already exercises
  the same path and the evidence report explains why it is equivalent

Required if compiler/runtime/shared build files change:

- `cargo test --all`

Required review evidence:

- PR summary lists changed files and confirms no product specs, generated
  telemetry artifacts, remote export, Studio UI, repair logic, or runtime assets
  outside the accepted runtime source files were added.
- PR evidence maps every #584 BDD scenario to tests, manual smoke, or review
  evidence.
- Automated review and CI are clean before the issue is moved to Technical Spec
  Review or later implementation review states.

## Completion Criteria

- Function and block enter/exit events are emitted for traced runs.
- Default builds remain uninstrumented and do not reference trace hooks.
- Event trace IDs are deterministic and graph-linked.
- Event trace IDs join with trace-map function/block metadata.
- Trace event emission works locally without network services or provider
  credentials.
- Telemetry write failure behavior is either tested portably or documented with
  manual evidence and runtime warning review.
- No out-of-scope telemetry, Studio, repair, per-operation, value-capture, or
  production ingestion work is added.
- All required checks in the Verification Plan pass or have an explicitly
  accepted reason for deferral.

## Failure And Escalation

- If focused tests fail, stop and classify whether the failure is a pre-existing
  baseline issue or caused by the current cycle.
- If the current implementation already satisfies #584, do not make cosmetic
  changes. Report evidence and route to review.
- If trace ID behavior conflicts with graph identity requirements, stop for
  human guidance before changing ID semantics.
- If block-exit completeness conflicts with Cranelift terminator constraints,
  preserve program correctness and ask for product/technical review.
- If write-failure testing is not portable, propose manual evidence rather than
  adding brittle CI behavior.
- If a cycle would require remote telemetry, Studio UI, repair behavior,
  dependency changes, generated artifacts, or product spec edits, stop because
  that exceeds #584 scope.
- If command budget or external LLM thresholds would be exceeded, request human
  approval before continuing.

## Open Questions

- Should #584 implementation harden the current deterministic graph-ID fallback
  for functions/blocks without node-derived IDs, or should a later product
  decision require hard failure for missing graph identity?
- Should portable CI cover telemetry write failure, or is manual smoke evidence
  acceptable for platform-specific filesystem permission behavior?

These questions do not block Stage 9 review. The required implementation path is
clear: keep default builds uninstrumented, emit local graph-linked
function/block events only in traced mode, and prove the event stream joins to
function/block graph metadata.
