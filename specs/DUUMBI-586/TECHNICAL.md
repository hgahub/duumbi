# DUUMBI-586: Controlled Runtime Failure Back-Mapping Evidence - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-586/PRODUCT.md` by
making the controlled runtime failure back-mapping proof deterministic,
repeatable, and test-covered.

The accepted product behavior is:

```text
controlled traced fixture -> runtime panic -> local trace/crash/map artifacts ->
telemetry inspection -> graph function/block evidence
```

This technical spec is intentionally an evidence-hardening slice. The current
Phase 13 source already contains traced builds, function/block trace events,
trace maps, crash dumps, telemetry inspection, and controlled panic fixtures.
Stage 10 agents must first audit that baseline against the BDD scenarios, then
make only the smallest remaining changes required to prove the #586 product
contract.

The implementation must not introduce new telemetry schemas, new artifact
names, per-operation or exact-node tracing, value capture, remote telemetry,
Studio UI, repair-agent behavior, patch generation, repair validation, or
autonomous repair.

## Agent Audience

- Codex implementation agents running bounded Stage 10 Ralph cycles.
- Stage 10 tester/reviewer agents validating the telemetry proof.
- Rust test agents working in integration and telemetry unit coverage.
- Compiler/runtime agents only if the baseline audit exposes a product-blocking
  trace evidence bug.
- Stage 9 technical spec reviewers checking feasibility, scope, and evidence.

## Source Context

- Product spec: `specs/DUUMBI-586/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/649
- GitHub issue: https://github.com/hgahub/duumbi/issues/586
- Stage 7 product spec approval:
  https://github.com/hgahub/duumbi/issues/586#issuecomment-4606382591
- Stage 6 product spec draft:
  https://github.com/hgahub/duumbi/issues/586#issuecomment-4606317476
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/586#issuecomment-4599102268
- Stage 4 triage:
  https://github.com/hgahub/duumbi/issues/586#issuecomment-4597764103
- Parent Phase 13 product spec: `specs/DUUMBI-580/PRODUCT.md`
- Parent Phase 13 technical sequencing context:
  `specs/DUUMBI-580/TECHNICAL.md`
- Traced build/config technical spec: `specs/DUUMBI-583/TECHNICAL.md`
- Function/block trace event technical spec: `specs/DUUMBI-584/TECHNICAL.md`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant verified source facts:

- `tests/integration_telemetry.rs`
  - Builds `tests/fixtures/telemetry/option_none_unwrap.jsonld` with
    `duumbi build --trace`.
  - Runs the compiled fixture with an isolated `DUUMBI_TELEMETRY_DIR`.
  - Asserts the process exits nonzero and preserves the original
    `duumbi panic: called Option::unwrap() on a None value` stderr behavior.
  - Asserts `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` exist.
  - Runs `duumbi telemetry inspect --telemetry-dir <dir>` and asserts mapped
    function/block output plus `Exact node evidence: unavailable in v1`.
  - Parses trace events and asserts required function/block trace event IDs
    join entries in the trace map.
  - Contains a second `call_then_panic` fixture proving the panic reports the
    caller context after a prior helper call.
  - Contains a malformed telemetry config integration test for
    `duumbi telemetry inspect`.
- `tests/fixtures/telemetry/option_none_unwrap.jsonld`
  - Provides the deterministic `OptionNone` + `OptionUnwrap` controlled panic
    fixture.
  - Uses graph IDs `duumbi:telemetry/main` and
    `duumbi:telemetry/main/entry`.
- `tests/fixtures/telemetry/call_then_panic.jsonld`
  - Provides caller/helper context coverage and ensures the reported crash
    context remains the active caller block, not a previously exited helper.
- `src/telemetry/mod.rs`
  - Defines `TELEMETRY_DIR_ENV`, `TRACE_MAP_FILE`, `CRASH_DUMP_FILE`, trace map
    schema types, crash artifact parsing, and `InspectReport::to_cli_output()`.
  - Maps a crash artifact to graph function/block entries by joining crash
    `function_id` and `block_id` to `trace_map.json`.
  - Rejects inactive traced crashes and unmapped function/block trace IDs.
  - Already contains unit coverage for trace map generation, inspect mapping,
    repair context serialization, and missing map rejection.
  - Contains repair-context helpers that must remain dormant for #586.
- `src/cli/mod.rs`, `src/main.rs`, and `src/cli/commands.rs`
  - Expose `duumbi build --trace`.
  - Expose `duumbi telemetry inspect [--telemetry-dir] [--crash] [--map]`.
  - Resolve telemetry artifact directories through config and
    `DUUMBI_TELEMETRY_DIR`.
- `src/compiler/lowering.rs`
  - Emits trace runtime calls only when `TelemetryBuildMode::Trace` is selected.
  - Keeps default builds uninstrumented.
  - Has focused unit tests for traced and default object behavior.
- `runtime/duumbi_runtime.c`
  - Writes trace events to `traces.jsonl`.
  - Writes panic crash records to `crash_dump.jsonl`.
  - Preserves the original runtime panic stderr message and exits nonzero.
- `runtime/duumbi_runtime.h`
  - Declares the trace hook ABI used by generated code.

Relevant Stage 8 interpretation:

- The main #586 gap is likely test/evidence hardening, not a new runtime or
  compiler feature.
- Existing trace event assertions prove that trace events join the trace map,
  but Stage 10 must explicitly verify whether crash dump `function_id` and
  `block_id` are also asserted against the trace map in the live fixture path.
- Existing inspect tests cover missing map behavior, but Stage 10 must verify
  whether unmapped crash IDs and default untraced fixture failures are covered
  well enough to satisfy the approved BDD scenarios.
- #585 is not an approved prerequisite changing the current artifact contract
  at Stage 8. If #585 is approved before or during Stage 10 and changes
  artifact names, fields, or join semantics, the #586 implementation must stop
  and route back to product/spec clarification before editing.

## Affected Areas

Expected implementation areas:

- Primary integration evidence:
  - `tests/integration_telemetry.rs`
- Existing telemetry fixtures, only if the baseline audit proves the current
  fixtures cannot express the accepted BDD evidence:
  - `tests/fixtures/telemetry/option_none_unwrap.jsonld`
  - `tests/fixtures/telemetry/call_then_panic.jsonld`
- Telemetry unit tests, only if negative mapping behavior is not already
  sufficiently covered:
  - `src/telemetry/mod.rs`

Areas that should not change unless the baseline audit finds a product-blocking
bug and the agent escalates before editing:

- Compiler lowering:
  - `src/compiler/lowering.rs`
- Runtime trace writer:
  - `runtime/duumbi_runtime.c`
  - `runtime/duumbi_runtime.h`
- CLI command implementation:
  - `src/cli/mod.rs`
  - `src/main.rs`
  - `src/cli/commands.rs`
- Telemetry artifact schema or artifact names:
  - `trace_map.json`
  - `traces.jsonl`
  - `crash_dump.jsonl`

Areas out of scope for #586:

- `specs/DUUMBI-586/PRODUCT.md`
- Other approved product or technical specs.
- Provider/model setup, external LLM behavior, Query mode, registry behavior,
  Studio dashboards, remote telemetry/export, repair-agent input generation,
  repair patch validation, autonomous repair, and generated telemetry artifacts.

## Technical Approach

### 1. Run A Baseline Contract Audit

Before editing, the Stage 10 agent must map current source behavior to each
approved BDD scenario:

- inspect `tests/integration_telemetry.rs` and fixture helper behavior.
- inspect crash artifact parsing and mapping in `src/telemetry/mod.rs`.
- inspect trace hook output in `runtime/duumbi_runtime.c` only if fixture
  behavior is unclear.
- inspect traced/default build behavior in `src/compiler/lowering.rs` only if
  default untraced coverage is unclear.
- record which BDD scenarios are already satisfied by existing tests.

If a scenario is already covered by equivalent tests, preserve that coverage and
reference it in the implementation PR evidence instead of duplicating it.

### 2. Harden The Controlled Fixture Evidence

The implementation should keep using the existing controlled panic fixture
unless the audit proves it is insufficient.

Required fixture-path proof:

- traced build uses `duumbi build --trace`.
- build and run both use an isolated `DUUMBI_TELEMETRY_DIR`.
- compiled fixture exits nonzero.
- stderr still includes the original Option unwrap panic text.
- `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` exist in the
  isolated telemetry directory.
- `duumbi telemetry inspect --telemetry-dir <dir>` succeeds.
- inspect output reports the expected graph function and block.
- inspect output does not claim exact node evidence.
- function/block trace event IDs in `traces.jsonl` join `trace_map.json`.
- panic crash record `function_id` and `block_id` in `crash_dump.jsonl` join
  the function/block entries in `trace_map.json`.

Implementation guidance:

- Prefer adding a test-local crash record reader in `tests/integration_telemetry.rs`
  over exposing new production APIs.
- If a public telemetry type can be reused without weakening module boundaries,
  that is acceptable, but do not make private production internals public only
  for this test.
- Keep the fixture deterministic and do not assert timestamps, absolute paths,
  generated binary names, or ordering that is not part of the artifact contract.
- Keep exact-node evidence explicitly unavailable in v1.

### 3. Prove Negative Evidence Boundaries

The implementation must show that the back-mapping proof fails closed when
evidence is absent, malformed, or not produced by a traced run.

Required negative checks:

- Missing map evidence must not be accepted as a valid back-mapping proof.
- A malformed telemetry config must be reported as a configuration failure, not
  as generic missing evidence.
- A default untraced fixture failure must not be accepted as traced
  back-mapping proof.
- If crash IDs are present but do not join the trace map, inspect must fail with
  an unmapped-evidence error.

Implementation guidance:

- Reuse existing unit tests where they already prove the behavior.
- Add the narrowest missing unit or integration test for any uncovered negative
  scenario.
- For the default untraced scenario, the most direct proof is to build the
  controlled fixture without `--trace`, run it under an isolated telemetry dir,
  assert the original panic still occurs, and assert telemetry inspection does
  not produce mapped graph evidence for that run.
- For the unmapped-ID scenario, prefer a small telemetry unit test that writes a
  valid trace map and a crash artifact with nonmatching IDs, then asserts
  `TelemetryError::Unmapped`.

### 4. Preserve Repair And External-Service Boundaries

#586 is not allowed to start repair automation.

The implementation must not:

- call provider/model setup flows.
- call OpenAI, Anthropic, or other external LLM APIs.
- generate repair prompts, repair patches, or repair validation evidence.
- accept a proposed repair.
- add Studio or remote telemetry flows.

Existing repair-context code in `src/telemetry/mod.rs` may remain untouched.
The #586 proof ends at mapped function/block crash evidence.

## Invariants

- Default builds remain untraced.
- `duumbi build --trace` remains the v1 opt-in traced build surface.
- Artifact names remain `trace_map.json`, `traces.jsonl`, and
  `crash_dump.jsonl`.
- Trace IDs remain graph-linked function/block IDs, not traversal-order IDs.
- Crash dump IDs must join the trace map before inspect reports graph context.
- Missing, inactive, malformed, or unmapped evidence fails closed.
- Runtime panic stderr remains the original user-visible failure signal.
- Exact node evidence remains unavailable in v1.
- No generated telemetry artifacts are committed.
- No product specs, implementation scope, or telemetry schema contracts are
  changed in #586 without returning to the appropriate DUUMBI spec stage.

## BDD-To-Test Mapping

| Product BDD Scenario | Required Technical Evidence |
| --- | --- |
| Controlled fixture writes local crash evidence | `tests/integration_telemetry.rs` traced fixture path builds with `--trace`, runs with isolated `DUUMBI_TELEMETRY_DIR`, exits nonzero, preserves original panic stderr, and asserts `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` exist. |
| Crash evidence maps to graph function/block context | Integration test runs `duumbi telemetry inspect --telemetry-dir <dir>` and asserts expected function/block graph IDs plus exact-node-unavailable output. |
| Trace event IDs join trace map | Existing or hardened integration helper parses `traces.jsonl`, requires function/block trace events, and joins every asserted trace ID to a same-kind `TraceMapEntry`. |
| Crash trace IDs join trace map | Add or verify integration evidence that parses latest panic record from `crash_dump.jsonl` and joins `function_id` and `block_id` to same-kind `TraceMapEntry` values. |
| Missing mapping evidence prevents false positive | Existing missing-map unit test may satisfy missing evidence; add a focused unmapped-ID unit test if no current test proves `TelemetryError::Unmapped` for nonjoining crash IDs. |
| Malformed telemetry config reported as config failure | Existing `telemetry_inspect_without_dir_reports_malformed_config` integration test should remain and assert config failure text instead of generic missing evidence. |
| Default untraced failure is not accepted as back-mapping proof | Add or verify a default-build fixture test that omits `--trace`, observes the original panic, and asserts telemetry inspect cannot report mapped graph evidence for that run. |
| Existing equivalent fixture can be hardened instead of duplicated | Implementation PR evidence should state whether the existing `option_none_unwrap` and `call_then_panic` fixtures were reused; no duplicate fixture is required when existing fixtures cover the behavior. |
| Back-mapping proof does not start repair automation | Diff review plus tests must show no provider calls, repair prompt generation, repair patch generation, validation evidence generation, or repair acceptance path was added. |
| Proof works without external services | Focused tests and live E2E run locally with filesystem telemetry only; external LLM calls and network calls remain zero. |

## Live E2E Plan

This issue does not touch LLM behavior, so no live LLM-backed E2E path is
required. The live E2E path is a local CLI/runtime smoke test with zero external
LLM calls and zero expected provider cost.

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
test -f "$tmp/telemetry/trace_map.json"
test -f "$tmp/telemetry/traces.jsonl"
test -f "$tmp/telemetry/crash_dump.jsonl"
target/debug/duumbi telemetry inspect --telemetry-dir "$tmp/telemetry"
```

Expected live E2E observations:

- fixture run exits nonzero.
- stderr includes `duumbi panic: called Option::unwrap() on a None value`.
- telemetry artifacts are created under the temporary telemetry directory.
- inspect output contains the expected graph function and block.
- inspect output contains `Exact node evidence: unavailable in v1`.

Cross-platform canonical evidence is `cargo test --test integration_telemetry`,
because the Rust test harness avoids shell-specific tempdir and executable
suffix behavior.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must have one bounded goal and must stop at the first
gate that requires human or coordinator approval.

Allowed autonomous cycle goals:

- Audit existing #586 coverage and report exact remaining gaps.
- Add missing integration assertions for crash dump IDs joining the trace map.
- Add one focused negative test for default untraced behavior.
- Add one focused unit test for unmapped crash IDs if the audit finds no
  equivalent existing coverage.
- Update implementation PR evidence after tests pass.

Default cycle constraints:

- Touch at most two files per cycle unless the implementation coordinator
  explicitly approves a broader cycle.
- Prefer test-only changes in `tests/integration_telemetry.rs` and, if needed,
  `src/telemetry/mod.rs` unit tests.
- Do not edit compiler, runtime, CLI, config, or artifact schema code without
  stopping for approval after documenting the baseline bug.
- Do not create, edit, or commit generated telemetry artifacts.
- Do not edit product specs or this technical spec during Stage 10.

Human approval is required before:

- changing artifact names, schemas, fields, or join semantics.
- changing compiler trace emission behavior.
- changing runtime panic behavior or trace hook ABI.
- changing CLI command behavior beyond test invocation.
- adding dependencies, accepting a risky dependency, or using external
  services.
- touching provider/model behavior or external LLM calls.
- expanding from function/block evidence to exact node or per-operation
  evidence.
- adding repair-agent, repair-patch, repair-validation, or Studio behavior.
- making a product, architecture, security, migration, or irreversible workflow
  decision.
- exceeding the external LLM cost or call thresholds below.

## Cycle Budget

- Default autonomous batch cap: 3 low-budget Ralph cycles.
- Expected external LLM calls for implementation: 0.
- Expected external LLM cost for implementation: USD 0.
- Hard approval threshold: stop before external LLM usage exceeds USD 2 or
  before external LLM calls exceed 10.
- Stop immediately on repeated test instability, unclear artifact contract,
  #585 contract conflict, schema mismatch, blocker discovery, or scope
  expansion.

## Task Breakdown

1. Verify the issue still has `product-spec-approved` and is still ready for
   technical implementation planning.
2. Check whether #585 has gained an approved spec or merged source change that
   changes telemetry artifact names, fields, or join semantics. If yes, stop
   and route #586 back for product/spec clarification.
3. Audit existing #586 coverage in `tests/integration_telemetry.rs`,
   `src/telemetry/mod.rs`, and telemetry fixtures.
4. Add missing crash dump join assertions to the controlled traced fixture path
   if they are not already present.
5. Add or verify negative coverage for missing map evidence, unmapped crash
   IDs, malformed telemetry config, and default untraced failure.
6. Preserve existing caller-context coverage with `call_then_panic`.
7. Run focused verification.
8. Record implementation PR evidence showing which BDD scenarios are covered
   by which tests and live E2E results.

## Verification Plan

Required local checks for the implementation PR:

```sh
cargo fmt --check
git diff --check
cargo test --test integration_telemetry
cargo test telemetry --lib
```

Conditional checks:

```sh
cargo test trace_hooks --lib
```

Run `cargo test trace_hooks --lib` if any compiler trace-hook behavior is
touched or if the audit depends on compiler trace-hook guarantees.

```sh
cargo test --all
```

Run `cargo test --all` before implementation PR review if the diff touches
shared compiler, runtime, CLI, config, workspace, telemetry schema, or other
broad behavior.

Manual live E2E:

- Run the Live E2E Plan above after the focused test suite passes.
- Record the temporary artifact directory only in PR evidence; do not commit
  generated files.

## Completion Criteria

The implementation is complete when:

- All approved #586 BDD scenarios have mapped test or live E2E evidence.
- The controlled traced fixture produces local trace, crash, and map artifacts.
- Trace event IDs and crash record IDs join the trace map.
- Inspect reports graph function/block context and exact-node-unavailable v1
  output.
- Missing, malformed, untraced, or unmapped evidence fails closed.
- No external services are required.
- No repair automation starts.
- Required focused checks pass.
- Any broader checks required by touched files pass.
- The implementation PR references #586 with non-closing language and leaves the
  execution issue open.

## Failure And Escalation

Stop and request coordinator or human approval if:

- #585 or another approved spec changes the artifact contract before #586 is
  implemented.
- The current telemetry artifact shape cannot satisfy the approved product spec
  without schema or runtime changes.
- The default untraced negative proof conflicts with existing CLI/runtime
  behavior in a way that requires product interpretation.
- A test-only implementation cannot prove the crash IDs join the trace map.
- Compiler, runtime, CLI, config, or telemetry schema changes appear necessary.
- Any Stage 10 agent would need external LLM calls, new dependencies, generated
  committed artifacts, or repair-agent behavior.

## Open Questions

None blocking. Treat the following as fixed Stage 10 constraints:

- Use the merged source artifact contract unless #585 is approved first and
  changes it; in that case stop and route back for clarification.
- Reuse and harden existing telemetry fixtures instead of duplicating them when
  they provide equivalent evidence.
- Keep exact node evidence out of scope for #586.
