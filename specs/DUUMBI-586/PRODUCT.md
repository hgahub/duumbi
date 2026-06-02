# DUUMBI-586: Add Controlled Runtime Failure Back-Mapping Evidence

## Summary

Define the product contract for a deterministic Phase 13 verification fixture
that proves a controlled runtime failure can be mapped back to graph-level
evidence.

This issue is a focused verification slice under the approved #580 Phase 13
runtime feedback direction. It does not define a new telemetry product surface.
It proves that the existing traced-build, trace-event, crash-artifact, and
trace-map surfaces work together in an end-to-end failure path.

For v1, the accepted observable proof is:

```text
controlled traced fixture -> runtime panic -> local trace/crash/map artifacts
-> telemetry inspection -> graph function/block evidence
```

The proof must be local, deterministic, and reviewable in CI. It must not start
repair-agent prompting, patch generation, Studio UI work, remote telemetry, or
exact node-level repair claims.

## Problem

Phase 13 has a hard trust boundary: DUUMBI should not justify repair automation
until a runtime failure can be connected to graph-level evidence. Traced builds,
trace events, crash artifacts, and trace maps are necessary pieces, but they are
not enough by themselves. Without a controlled failing fixture, the telemetry
pipeline can look complete while still failing the core question:

```text
Can a real traced runtime failure be mapped back to human-reviewable graph
function/block context?
```

The gap is evidence quality. A generic unit test can prove serialization. A
happy-path trace test can prove event emission. This issue needs a failing
program path that exercises the compiled binary, runtime panic handling, local
artifact writing, trace-to-graph lookup, and human-readable inspection together.

The scope must stay narrow. If this issue expands into artifact format design,
remote export, broad runtime fuzzing, exact node-level tracing, repair-agent
prompting, or automatic patch validation, it will blur the first Phase 13 proof
instead of strengthening it.

## Outcome

When this issue is implemented:

- A deterministic DUUMBI graph fixture intentionally triggers a controlled
  runtime failure in traced mode.
- The traced binary exits nonzero and preserves the original runtime failure
  signal on stderr.
- The run writes local trace, crash, and trace-map evidence under the selected
  telemetry artifact directory.
- The crash evidence includes trace correlation for the active graph function
  and block at failure time.
- Trace event IDs and crash IDs can be joined to `trace_map.json` entries of
  the matching graph kind.
- `duumbi telemetry inspect` or the accepted equivalent maps the failure to
  graph function/block identifiers.
- Exact node-level evidence remains explicitly unavailable in v1 unless a later
  approved issue adds enough evidence to support it.
- Tests fail clearly when artifacts are absent, malformed, unjoined, or
  unmapped.
- The fixture is documented or named clearly enough to serve as the first
  Phase 13 back-mapping proof.
- The execution issue remains open for later DUUMBI workflow stages; this PR is
  specification-only and must leave #586 open.

## Scope

### In Scope

- Add or harden a deterministic telemetry fixture that produces a controlled
  runtime failure, such as `Option::unwrap()` on `None`.
- Build the fixture with the accepted traced-build surface.
- Run the traced binary with a test-isolated telemetry artifact directory.
- Assert the runtime exits nonzero and preserves the original panic message.
- Assert local telemetry artifacts exist for:
  - trace events.
  - crash evidence.
  - trace-to-graph mapping.
- Parse trace, crash, and map artifacts as structured data instead of relying
  only on substring checks.
- Assert function and block trace IDs from runtime evidence join to matching
  graph function/block entries in the trace map.
- Assert telemetry inspection reports the mapped graph function and block.
- Assert telemetry inspection does not claim exact node evidence in v1.
- Assert missing or malformed evidence fails explicitly instead of producing a
  false positive mapping.
- Keep the proof local, deterministic, CI-friendly, and independent of network
  access, provider credentials, Studio, or external telemetry collectors.
- Reuse or harden existing telemetry fixtures and tests when they already cover
  the accepted behavior, rather than adding redundant fixtures.
- Record review evidence that the fixture is the Phase 13 controlled failure
  back-mapping proof for #586.

### Explicitly Out Of Scope

- Technical specification content or implementation code during Stage 6.
- New telemetry artifact formats or broad artifact contract redesign.
- New traced-build configuration or CLI trace-selection behavior.
- New function/block trace-event product semantics.
- Per-operation tracing.
- Exact node-level runtime tracing or exact source-line UX.
- Runtime value capture, argument snapshots, stack capture, or heap snapshots.
- Repair-agent prompting, repair patch generation, repair patch application, or
  repair validation.
- Studio telemetry views, graph overlays, dashboards, or UI validation.
- Remote telemetry export, OpenTelemetry collector integration, account-based
  crash ingestion, upload, retention, privacy consent flows, or production
  observability commitments.
- Hot-swap, silent updates, autonomous repair loops, or automatic repair
  acceptance.
- Broad runtime fuzzing or many failure fixtures beyond the minimum deterministic
  proof.
- Committing generated telemetry artifacts, crash dumps, binaries, or trace
  output.

## Constraints And Assumptions

Facts:

- Issue #586 is open and has explicit Stage 5 human acceptance on 2026-06-02.
- The Stage 5 decision records `Decision: Accept`, `Next state: Spec Needed`,
  and no remaining open questions.
- Issue #586 is labeled `accepted` and `needs-spec` at Stage 6 intake.
- Issue #586 is a child of #580.
- #580 has approved product and technical specs for the first Phase 13 local
  runtime failure feedback slice.
- #583 has approved and merged product and technical specs for traced build mode
  and telemetry configuration.
- #584 has approved and merged product and technical specs for function/block
  trace events.
- At Stage 6 inspection, #585 exists for crash dumps and trace-to-graph mapping
  artifacts, but it is still in human-review intake and has no `specs/DUUMBI-585`
  product artifact in this worktree.
- Before Stage 10 implements #586, the asserted artifact contract must be stable
  in merged source or in an approved product/technical spec. If #585 is approved
  first and changes artifact names, fields, or join semantics, #586 must return
  to product-spec update or clarification before implementation proceeds.
- The current source tree already contains telemetry-focused fixtures under
  `tests/fixtures/telemetry/`.
- The current source tree already contains `tests/integration_telemetry.rs`,
  which exercises traced failure, artifact creation, trace-map joins, and
  telemetry inspection behavior.
- `src/telemetry/mod.rs` defines trace-map, crash-artifact, inspection, and
  repair-context domain types.
- `runtime/duumbi_runtime.c` contains traced runtime hooks and crash artifact
  writing behavior.
- `src/compiler/lowering.rs` emits traced function/block runtime calls only for
  traced builds.
- The active PRD defines the first runtime feedback promise as local
  developer/test evidence, not production self-repair.
- The active runtime feedback note treats function/block-level graph
  back-mapping as the first reliable mapping level and leaves exact node-level
  mapping for later work.

Assumptions:

- #586 can proceed as a verification/hardening spec because the product outcome
  is the end-to-end proof, not ownership of every underlying telemetry surface.
- A controlled `Option::unwrap(None)`-style panic is a valid first fixture
  because it is deterministic, local, easy to review, and already matches
  DUUMBI runtime panic semantics.
- Additional fixtures are useful only when they prove a distinct back-mapping
  risk, such as preserving caller context across a non-failing helper call.
- Function/block mapping is sufficient for this issue because exact node-level
  evidence is not part of the v1 accepted Phase 13 boundary.
- If implementation review finds the existing fixture already satisfies the
  product contract, Stage 10 should harden, document, or re-attribute that
  evidence instead of duplicating it.

Constraints:

- The fixture must not require network access, provider credentials, GitHub,
  Slack, Studio, or external telemetry services.
- Test runs must isolate telemetry output with a temporary artifact directory or
  an equivalent cleanup-safe mechanism.
- The proof must parse artifacts structurally where feasible.
- The proof must fail clearly when evidence is missing, malformed, or not
  graph-linked.
- Telemetry handling must not hide the original runtime failure signal.
- Default untraced behavior must not become dependent on telemetry artifacts.
- Generated artifacts must not be committed.
- Exact node evidence must not be implied unless the artifacts actually support
  it.
- This spec PR is a Stage 6 artifact and must leave #586 open for review,
  technical specification, implementation, review, and closure evidence.

## Decisions

- **Decision:** Use a file-based product spec for #586.
  **Evidence:** The issue is durable Phase 13 verification work that connects
  compiler lowering, runtime panic behavior, telemetry artifacts, trace-map
  inspection, fixtures, and CI evidence. It is narrow, but important enough to
  preserve as source-controlled implementation context.

- **Decision:** Treat #586 as an end-to-end verification slice, not a new
  telemetry format slice.
  **Evidence:** #586 acceptance criteria ask for a deterministic controlled
  failure and graph-linked evidence. #583 and #584 already define traced build
  and trace-event behavior, and #580 defines the parent evidence boundary.

- **Decision:** Require a controlled runtime panic fixture.
  **Evidence:** The issue asks for a small DUUMBI program or graph fixture that
  fails in a controlled runtime path. A panic path exercises the runtime failure
  behavior that Phase 13 must map before repair automation is justified.

- **Decision:** Require artifact join checks, not only artifact existence.
  **Evidence:** Artifact existence does not prove back-mapping. Trace IDs from
  events and crash evidence must map to graph function/block entries in
  `trace_map.json` or equivalent accepted mapping evidence.

- **Decision:** Use `duumbi telemetry inspect` as the human-readable mapping
  surface when available.
  **Evidence:** The parent #580 technical spec and current source expose
  telemetry inspection as the local way to verify that crash evidence maps to
  graph function/block context.

- **Decision:** Keep exact node-level evidence explicit but unavailable in v1.
  **Evidence:** The active PRD and runtime feedback note say function/block
  mapping is the first reliable level. Exact node-level mapping remains future
  work unless a later approved issue proves enough error context.

- **Decision:** Allow reuse or hardening of existing fixtures.
  **Evidence:** Stage 6 source inspection found existing telemetry fixtures and
  integration tests. Product value comes from verified durable evidence, not
  from creating redundant files.

- **Decision:** This spec PR must be specification-only and must leave #586
  open.
  **Evidence:** Stage 6 creates a product spec for review. Product approval,
  technical specification, implementation, review, and closure evidence remain
  later workflow stages.

## Behavior

### Defaults

- Normal untraced builds remain outside the accepted #586 proof.
- Default builds do not need telemetry artifacts to pass.
- Missing telemetry artifacts from an untraced run must not be treated as a
  valid back-mapping proof.
- Controlled failure evidence is accepted only from a traced run or the
  accepted equivalent telemetry-enabled local path.
- The fixture and tests use local temporary artifact paths by default.
- Exact node evidence is reported as unavailable unless a later approved
  artifact contract changes that.

### Inputs

- A deterministic DUUMBI graph fixture that intentionally triggers a runtime
  panic.
- The accepted traced-build surface, currently `duumbi build --trace`.
- A test-isolated telemetry artifact directory, such as one selected through
  `DUUMBI_TELEMETRY_DIR`.
- The compiled traced binary.
- Local trace event, crash, and mapping artifacts produced during the run.
- The accepted telemetry inspection command or equivalent local inspection API.

### Outputs

- A nonzero process exit for the controlled failing binary.
- The original runtime panic message on stderr.
- Local trace event evidence.
- Local crash evidence containing trace correlation.
- Local trace-to-graph mapping evidence.
- A human-readable inspection result naming the mapped graph function and block.
- A test or review artifact showing that trace IDs join to graph mapping
  entries.
- A clear statement that exact node evidence is unavailable in v1 unless a
  later accepted slice changes that.

### Error States

- If the traced build fails, the test fails before claiming runtime evidence.
- If the binary exits successfully, the fixture is invalid because the failure
  was not controlled.
- If the panic message is absent or hidden, the test fails because telemetry did
  not preserve the original runtime signal.
- If trace, crash, or map artifacts are missing, the test fails with missing
  evidence.
- If artifacts are malformed, the test fails with parse evidence.
- If crash trace IDs do not map to graph function/block entries, inspection
  fails and the issue remains unproven.
- If telemetry inspection succeeds without graph-linked context, the proof is
  invalid.
- If exact node evidence is claimed without supporting artifacts, the proof is
  invalid.

### Determinism And Isolation

- The fixture should not depend on wall-clock timing, random input, network
  services, provider credentials, external collectors, Studio, or host-specific
  persistent state.
- The test must isolate telemetry files so repeated runs do not read stale
  artifacts.
- The test should account for platform-specific executable suffixes and CI
  paths.
- Generated telemetry files and binaries must stay out of source control.

## BDD Scenarios

Feature: Controlled runtime failure back-mapping evidence

Rule: The proof is a local traced failure, not a default-build behavior claim

Scenario: Controlled fixture writes local crash evidence
Given a deterministic DUUMBI fixture that unwraps `None`
And a temporary telemetry artifact directory
When the developer builds the fixture with `duumbi build --trace`
And runs the traced binary with the same telemetry directory
Then the binary exits nonzero
And stderr preserves the original `duumbi panic:` message
And the telemetry directory contains trace, crash, and trace-map artifacts

Scenario: Crash evidence maps to graph function and block context
Given trace, crash, and trace-map artifacts from the controlled traced failure
When the developer runs `duumbi telemetry inspect` for that telemetry directory
Then the inspection succeeds
And it reports the mapped graph function identifier
And it reports the mapped graph block identifier
And it reports that exact node evidence is unavailable in v1

Scenario: Trace event IDs join to the trace map
Given trace event artifacts from the controlled traced failure
And the trace-map artifact from the same build
When the verification reads function and block trace events
Then every required trace event has a trace ID
And each function event trace ID maps to a function entry in the trace map
And each block event trace ID maps to a block entry in the trace map

Scenario: Crash IDs join to the trace map
Given crash evidence from the controlled traced failure
And the trace-map artifact from the same build
When the verification reads the latest crash record
Then the crash record is marked as trace-active
And its function trace ID maps to a graph function entry
And its block trace ID maps to a graph block entry

Scenario: Missing mapping evidence prevents a false positive
Given crash evidence from a traced runtime failure
And no usable trace-map artifact
When the developer runs telemetry inspection
Then inspection fails with missing or unmapped evidence
And DUUMBI does not report repair-ready graph context

Scenario: Malformed telemetry config is reported as configuration failure
Given a workspace with malformed telemetry configuration
When the developer runs telemetry inspection without explicit artifact paths
Then DUUMBI reports the telemetry configuration problem
And it does not hide the configuration failure behind a generic missing-evidence
message

Scenario: Default untraced failure is not accepted as back-mapping proof
Given the same controlled runtime failure fixture
When the developer builds and runs it without traced behavior
Then the original runtime failure still remains visible
But missing trace/crash/map evidence does not satisfy the #586 proof

Scenario: Existing equivalent fixture can be hardened instead of duplicated
Given the repository already contains a deterministic telemetry failure fixture
And that fixture produces graph-linked function/block back-mapping evidence
When Stage 10 implements this issue
Then the implementation may harden or document the existing fixture
And it does not need to create a second equivalent fixture solely for file
count

Rule: The proof stays inside the telemetry evidence boundary

Scenario: Back-mapping proof does not start repair automation
Given a controlled traced runtime failure with graph-linked evidence
When the verification completes
Then no repair-agent prompt is sent
And no graph patch is generated or applied
And no repair validation evidence is treated as accepted

Scenario: The proof works without external services
Given no provider credentials, no Studio session, and no external telemetry
collector
When the controlled traced failure verification runs in CI
Then the verification depends only on local DUUMBI build, runtime, telemetry,
and filesystem behavior

## Tasks

- Verify current telemetry fixtures, integration tests, and artifact inspection
  behavior against this product contract.
- Decide whether to reuse, rename, harden, or add the minimum controlled failure
  fixture.
- Ensure the fixture has stable graph function/block identifiers suitable for
  human review.
- Build the fixture with traced mode in a temporary output location.
- Run the traced binary with an isolated telemetry directory.
- Parse and assert trace event, crash, and trace-map artifacts.
- Assert telemetry inspection maps the crash to graph function/block context.
- Add negative coverage for missing, malformed, or unmapped evidence when not
  already covered.
- Record enough test names, fixture names, or review notes that future Stage 11
  reviewers can identify the #586 proof.
- Keep implementation changes scoped to fixtures, tests, telemetry inspection
  helpers, or minimal supporting documentation needed to prove the behavior.

Tasks that can run independently:

- Fixture review and fixture naming.
- Artifact parser/assertion hardening.
- Negative missing/malformed evidence coverage.
- CI and platform path validation.
- Documentation or review-evidence wording.

## Checks

The completed implementation should be proven by focused local checks and CI
evidence. Expected checks include:

- `cargo fmt --check`
- `git diff --check`
- `cargo test --test integration_telemetry`
- `cargo test telemetry --lib`
- `cargo test trace_hooks --lib` when compiler trace-event behavior is touched.
- `cargo test --all` when the implementation changes shared telemetry,
  compiler, runtime, or fixture behavior broadly enough to warrant full
  regression.

Expected live E2E shape:

```text
tmp=<temporary directory>
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" target/debug/duumbi build --trace tests/fixtures/telemetry/<controlled-failure>.jsonld -o "$tmp/panic-fixture"
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" "$tmp/panic-fixture"
target/debug/duumbi telemetry inspect --telemetry-dir "$tmp/telemetry"
```

Expected evidence:

- The traced build succeeds.
- The traced build and traced run use the same telemetry artifact directory.
- The traced binary exits nonzero.
- Stderr includes the original DUUMBI panic message.
- `$tmp/telemetry/traces.jsonl` or the accepted trace event artifact exists.
- `$tmp/telemetry/crash_dump.jsonl` or the accepted crash artifact exists.
- `$tmp/telemetry/trace_map.json` or the accepted trace-map artifact exists.
- The trace event artifact contains required function/block events with trace
  IDs.
- Crash evidence contains trace-active function/block trace IDs.
- Trace and crash IDs join to graph function/block entries in the trace map.
- Telemetry inspection reports mapped function/block graph IDs.
- Telemetry inspection does not claim exact node-level evidence in v1.
- No provider, network, Studio, external collector, or repair-agent execution is
  required.

## Open Questions

- Should #585 receive its own product and technical specs before any future work
  changes the artifact contract, or is the merged #580/#584 source behavior
  sufficient context for #586's verification-only scope?
- Should the #586 proof keep both the direct panic fixture and the
  call-then-panic fixture, or should Stage 10 narrow to the minimum fixture plus
  one caller-context regression only if needed?
- Should a later issue add exact node-level crash evidence after this
  function/block proof is stable?

None of these questions block this product spec because #586 can be implemented
as a bounded verification/hardening slice over the currently available local
telemetry behavior.

## Sources

- GitHub issue #586:
  https://github.com/hgahub/duumbi/issues/586
- Stage 4 triage refill for #586:
  https://github.com/hgahub/duumbi/issues/586#issuecomment-4597764103
- Stage 5 human acceptance decision for #586:
  https://github.com/hgahub/duumbi/issues/586#issuecomment-4599102268
- Parent issue #580:
  https://github.com/hgahub/duumbi/issues/580
- Parent product spec:
  `specs/DUUMBI-580/PRODUCT.md`
- Parent technical spec:
  `specs/DUUMBI-580/TECHNICAL.md`
- Traced build product spec:
  `specs/DUUMBI-583/PRODUCT.md`
- Function/block trace event product spec:
  `specs/DUUMBI-584/PRODUCT.md`
- Adjacent artifact issue #585:
  https://github.com/hgahub/duumbi/issues/585
- Architecture reference:
  `docs/architecture.md`
- Telemetry integration tests:
  `tests/integration_telemetry.rs`
- Controlled telemetry fixtures:
  `tests/fixtures/telemetry/option_none_unwrap.jsonld`
  `tests/fixtures/telemetry/call_then_panic.jsonld`
- Telemetry domain and inspection code:
  `src/telemetry/mod.rs`
- Runtime telemetry hooks:
  `runtime/duumbi_runtime.c`
  `runtime/duumbi_runtime.h`
- Compiler lowering telemetry instrumentation:
  `src/compiler/lowering.rs`
- Active PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Runtime feedback note:
  `Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Runtime Failure Feedback Loop.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic development map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Development intake to delivery workflow:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Development Intake to Delivery Workflow.md`
