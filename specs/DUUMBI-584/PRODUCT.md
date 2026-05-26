# DUUMBI-584: Emit Function And Block Trace Events From Compiled Graphs

## Summary

Define the product contract for graph-linked runtime trace events emitted by
compiled DUUMBI graphs.

This issue is a focused Phase 13 child slice under #580. It depends on the
local traced-build boundary from #583 and defines the observable runtime event
behavior needed before crash dump, trace-map inspection, and repair-context
work can rely on execution evidence.

For v1, the accepted product surface is:

```text
explicit traced build -> graph-linked function/block enter and exit events ->
local trace evidence that can be correlated with graph function/block metadata
```

Default builds must remain uninstrumented. The first trace granularity is
function/block-level, not per-operation or exact node-level tracing.

## Problem

Phase 13 needs runtime evidence that connects generated execution back to the
semantic graph. Without function and block events, later crash evidence can say
that a runtime failure happened, but cannot reliably identify which graph-level
function and block were active around the failure.

The parent #580 scope and #583 traced-build slice establish a local,
developer/test telemetry boundary. This issue specifies the next behavior:
when a developer intentionally builds traced output, the compiled program emits
trace events for graph function and block entry/exit without changing default
build behavior.

The risk is scope creep. If this issue tries to include per-operation tracing,
remote telemetry export, Studio dashboards, repair-agent execution, or
production crash collection, it will skip the narrow evidence step that Phase 13
needs first.

## Outcome

When this issue is implemented:

- A traced DUUMBI build emits function enter, function exit, block enter, and
  block exit events during local execution.
- Each emitted event contains a stable trace identifier that can be joined with
  graph function/block metadata.
- Trace IDs preserve graph identity and do not depend on incidental compiler
  traversal order.
- Default untraced builds do not initialize tracing, call trace hooks, or emit
  trace event artifacts.
- Trace event emission works without provider credentials, network access,
  external telemetry collectors, Studio, or remote services.
- Runtime trace hooks are narrow enough to support future language targets and
  later crash evidence.
- Tests or equivalent review evidence prove both traced and default build
  behavior.
- The execution issue remains open for later workflow stages; this product spec
  PR is specification-only.

## Scope

### In Scope

- Define traced runtime events for function enter, function exit, block enter,
  and block exit.
- Require events only for explicitly traced builds.
- Preserve default uninstrumented build and runtime behavior.
- Require stable function and block trace identifiers that can be mapped back to
  graph `@id` values or equivalent graph identity metadata.
- Require local event output that later #585 and #586 work can inspect.
- Define expected behavior when trace emission cannot write local artifacts.
- Require function/block-level granularity as the accepted v1 surface.
- Require test coverage or equivalent evidence for traced event emission and
  default-build non-emission.
- Allow implementation to use runtime no-op or local trace writer hooks when
  needed by generated code.
- Document the initial trace event shape and how it supports crash
  back-mapping.

### Explicitly Out Of Scope

- Technical specification content or implementation code during Stage 6.
- Per-operation tracing.
- Exact node-level runtime event tracing.
- Runtime value, argument, heap, or stack capture.
- Crash dump persistence beyond the trace context needed by later slices.
- Trace-to-graph inspection commands; those belong to later artifact and
  back-mapping work.
- Repair-agent execution, repair prompt contracts, repair patch generation, or
  repair validation.
- Studio dashboards, graph overlays, Ops monitoring, alerting, or telemetry UI.
- Remote telemetry export, OTLP integration, external collectors, cloud
  ingestion, account-based telemetry, upload, retention, privacy consent flows,
  or production crash collection.
- Hot-swap, autonomous repair loops, silent updates, or automatic repair
  acceptance.
- Changing the user-facing traced-build selection accepted by #583 unless a
  later approved issue changes that surface.

## Constraints And Assumptions

Facts:

- Issue #584 is open and was accepted by Stage 5 on 2026-05-26.
- The Stage 5 decision records `Decision: Accept`, `Next state: Spec Needed`,
  and no remaining open questions.
- Stage 4 routed #584 to `Needs Human Acceptance` as the next concrete Phase 13
  telemetry implementation slice.
- #584 is a child issue of #580.
- The approved #580 product direction defines the first runtime feedback slice
  as local developer/test telemetry with function/block graph back-mapping, not
  production self-healing.
- #583 defines the traced-build and telemetry configuration boundary that this
  issue builds on.
- Active DUUMBI knowledge notes treat function/block-level graph back-mapping as
  the first reliable mapping level, with exact node-level mapping left for
  future work.
- The source repository already contains telemetry-related code from #580 and
  #583, including local trace config, trace-map concepts, runtime trace hooks,
  and compiler lowering surfaces that can inform later implementation.
- `src/compiler/lowering.rs` is the source area where compiled graph functions
  and blocks can be instrumented.
- `runtime/duumbi_runtime.c` and `runtime/duumbi_runtime.h` are the runtime
  source areas for local trace hook behavior.

Assumptions:

- The v1 product need is execution correlation, not full observability.
- Function/block enter and exit events are enough for the first crash
  back-mapping contract because crash handling can record the active
  function/block context.
- Trace identifiers should be derived from graph identity rather than build-time
  traversal order.
- A narrow trace hook ABI is preferable because it keeps the runtime surface
  portable and reviewable.
- Local trace artifact writing may fail on some filesystems, and such failure
  should not hide the original program behavior.

Constraints:

- Default builds must not emit trace events, require telemetry config, or depend
  on trace runtime initialization.
- Traced builds must remain local-only and testable without networked services.
- Trace event emission must not change program semantics except for the expected
  local telemetry side effect in traced mode.
- Instrumentation must not break compiler control-flow correctness, return
  behavior, or branch behavior.
- Trace event shape must be stable enough for later crash dump and inspection
  work.
- Trace IDs must be compatible with the runtime ABI selected by the technical
  spec.
- This spec PR must not mark #584 complete; it is a Stage 6 review artifact
  only.

## Decisions

- **Decision:** Use a file-based product spec for #584.
  **Evidence:** The work is cross-module and durable. It affects compiler
  lowering, runtime hooks, local artifacts, graph identity, tests, and Phase 13
  sequencing.

- **Decision:** Emit function/block enter and exit events only for traced
  builds.
  **Evidence:** #584 acceptance criteria require default builds to remain
  uninstrumented, and #583 makes traced behavior explicit through the traced
  build boundary.

- **Decision:** Use function/block-level events as the v1 granularity.
  **Evidence:** #584 explicitly scopes trace calls to function/block enter and
  exit. The parent #580 product spec and active runtime-feedback note identify
  function/block mapping as the first reliable level and defer exact node-level
  mapping.

- **Decision:** Preserve graph identity through stable trace identifiers.
  **Evidence:** The issue calls out stable identifiers that map back to graph
  `@id` values and warns against depending on incidental compiler ordering.

- **Decision:** Keep the event ABI small.
  **Evidence:** #584 identifies runtime hook ABI size as a risk, and the Phase
  13 direction is local evidence first rather than full observability.

- **Decision:** Treat trace artifact write failure as telemetry degradation, not
  as a replacement for original program behavior.
  **Evidence:** The parent runtime feedback direction requires telemetry to
  preserve the original runtime signal instead of hiding it behind telemetry
  handling.

- **Decision:** This spec PR must be specification-only and must leave #584
  open.
  **Evidence:** Stage 6 creates a reviewable product spec. Product approval,
  technical specification, implementation, review, and final workflow handling
  remain later stages.

## Behavior

### Defaults

- Default builds are uninstrumented.
- Default builds do not initialize trace state.
- Default builds do not call function or block trace hooks.
- Default builds do not write trace event artifacts.
- Missing or invalid trace-only telemetry settings do not affect the default
  build path unless another accepted issue changes that contract.

### Traced Execution

- A traced build produces compiled output that can emit trace events during
  execution.
- Trace initialization happens only for traced output.
- Function entry emits a function-enter event with the active graph function
  trace ID.
- Function exit emits a function-exit event with the active graph function
  trace ID before a normal return.
- Block entry emits a block-enter event with the active graph block trace ID.
- Block exit emits a block-exit event with the active graph block trace ID when
  execution leaves that block normally.
- Branch-heavy control flow should emit enough block context to identify the
  active block during a later failure. If a technical limitation prevents
  perfect block-exit coverage in v1, the limitation must be explicit and tested,
  and block-enter plus crash-time current block context remains required.
- Nested calls preserve the current function and block context so later failure
  evidence can identify the active graph location.

### Inputs

- A DUUMBI graph or workspace compiled with the accepted traced-build surface.
- Graph function and block identity metadata.
- Local telemetry configuration accepted by the traced-build boundary.
- Runtime hooks or equivalent local trace writer functions available to traced
  generated code.

### Outputs

- Local trace event records for traced executions.
- Event records include:
  - schema or event format version.
  - event kind: function enter, function exit, block enter, or block exit.
  - stable trace ID for the graph function or block.
  - timestamp or placeholder timestamp when a real clock is not yet accepted.
- The event stream is joinable with graph function/block mapping data.
- Default executions produce no trace event records.

### Error States

- If trace artifact writing fails during traced execution, DUUMBI reports a
  telemetry warning or equivalent local diagnostic and preserves the program's
  original behavior.
- If a traced build cannot derive a stable graph identifier for a function or
  block, it fails before claiming traced instrumentation is available.
- If runtime hooks are missing for traced output, the build or link step fails
  clearly instead of producing a binary that appears trace-capable but cannot
  emit events.
- If telemetry is disabled by local config in a traced build, trace event
  emission follows the accepted #583 config semantics.

### Invariants

- Trace instrumentation is opt-in.
- Trace IDs are stable for a given graph identity.
- Trace event emission is local-only.
- Trace events do not contain runtime values, arguments, secrets, heap snapshots,
  or stack snapshots in this issue.
- The first trace event surface supports later crash back-mapping but does not
  itself claim repair readiness.

## BDD Scenarios

Feature: Function and block trace events from compiled graphs

Rule: Default builds remain uninstrumented

Scenario: Default build does not emit trace events

Given a valid DUUMBI graph
When the developer builds without traced mode
And runs the compiled program
Then the program follows the existing runtime behavior
And no function trace events are emitted
And no block trace events are emitted
And no trace runtime initialization is required

Scenario: Default compiled output does not reference trace hooks

Given a valid DUUMBI graph
When DUUMBI compiles the graph without traced mode
Then the compiled output does not require function trace hook symbols
And the compiled output does not require block trace hook symbols
And linking does not depend on trace runtime entry points

Rule: Traced builds emit graph-linked events

Scenario: Traced run emits function enter and exit events

Given a valid DUUMBI graph with a main function
When the developer builds with traced mode
And runs the compiled program
Then the local trace event stream contains a function-enter event for the main
function
And the local trace event stream contains a function-exit event for the main
function when it returns normally
And both events include the stable trace ID for that graph function

Scenario: Traced run emits block enter and exit events

Given a valid DUUMBI graph with an entry block
When the developer builds with traced mode
And runs the compiled program
Then the local trace event stream contains a block-enter event for the entry
block
And the local trace event stream contains a block-exit event for the entry block
when execution leaves it normally
And both events include the stable trace ID for that graph block

Scenario: Trace IDs are stable across equivalent traced builds

Given a valid DUUMBI graph whose function and block graph identities do not
change
When the developer builds the graph with traced mode twice
And runs both compiled outputs
Then corresponding function events use the same function trace ID
And corresponding block events use the same block trace ID
And the IDs can be joined with the same graph function/block mapping metadata

Scenario: Trace IDs do not depend on compiler traversal order

Given two DUUMBI graphs with the same graph function and block identities
But with incidental ordering differences that do not change those identities
When both graphs are built with traced mode
Then corresponding function and block trace IDs remain stable
And later crash evidence can use those IDs for graph back-mapping

Rule: Trace behavior is local and conservative

Scenario: Trace event emission does not require external services

Given a valid DUUMBI graph
When the developer builds with traced mode
And runs the compiled program on a machine without provider credentials,
Studio, network access, or an external telemetry collector
Then function and block trace events are emitted locally according to local
configuration
And no remote telemetry export is attempted

Scenario: Trace artifact write failure does not hide original behavior

Given a traced DUUMBI binary
And the configured local trace artifact path cannot be written
When the developer runs the binary
Then DUUMBI reports a telemetry write warning or equivalent local diagnostic
And the original program output or runtime failure remains visible
And DUUMBI does not report remote telemetry fallback

Scenario: Traced runtime records active context for a later crash

Given a traced DUUMBI binary that enters a graph function and block
When the program later fails through an accepted runtime panic path
Then the trace state has enough active function and block context for later
crash evidence
And exact node-level evidence is not required by this issue

Rule: Unsupported or incomplete tracing fails clearly

Scenario: Missing runtime hooks fail before false trace success

Given a traced build that requires runtime trace hooks
And the trace runtime hooks are unavailable
When DUUMBI builds or links the output
Then the command fails clearly
And it does not produce an output that claims traced event emission is available

Scenario: Missing graph identity prevents traced instrumentation

Given a graph function or block without a stable graph identity
When DUUMBI builds with traced mode
Then the traced build fails with an actionable diagnostic
And it does not emit trace events with order-derived placeholder IDs

## Tasks

- Define the product event kinds and minimum event fields for function/block
  tracing.
- Define the graph identity and trace ID stability expectations.
- Define default-build non-instrumentation behavior.
- Define traced-build runtime behavior for function enter/exit and block
  enter/exit.
- Define local artifact write failure behavior.
- Ensure the runtime trace hook surface remains narrow and local.
- Add or update user-facing documentation for the first trace event shape and
  crash back-mapping purpose.
- Prepare later #585 and #586 work to consume the function/block event stream
  without changing this product contract.

## Checks

- Product review verifies this spec against #584, Stage 5 acceptance, Stage 4
  triage, #580, #583, active DUUMBI runtime-feedback notes, and relevant source
  context.
- Focused compiler or integration tests prove default builds do not initialize
  tracing or reference trace hook symbols.
- Focused compiler or integration tests prove traced builds emit function-enter
  and function-exit events.
- Focused compiler or integration tests prove traced builds emit block-enter and
  block-exit events.
- Tests prove trace IDs are deterministic for stable graph function/block
  identities.
- Tests or review evidence prove trace IDs are joinable with graph
  function/block metadata.
- Tests or manual smoke evidence prove traced execution works locally without
  network access, provider credentials, Studio, or external collectors.
- Tests or manual smoke evidence cover a telemetry write failure path when
  practical.
- Review evidence confirms no per-operation tracing, remote export, Studio UI,
  repair execution, or production crash ingestion was added for this issue.
- CI includes normal formatting and Rust test checks expected for touched
  modules at implementation time.

## Open Questions

- Should block-exit completeness be mandatory for every branch-heavy control
  flow path in v1, or is block-enter plus crash-time current block context
  enough when a technical limitation is explicit and tested?
- Should trace events use a real timestamp in the first implementation, or is a
  placeholder timestamp acceptable until artifact inspection work needs ordering
  semantics?
- Should trace event records include both function and block IDs on every block
  event, or keep the runtime ABI minimal and rely on trace state plus map
  artifacts?

None of these questions blocks Stage 7 review because the product contract is
clear: traced builds emit local graph-linked function/block events, default
builds stay uninstrumented, and broader telemetry/repair behavior stays out of
scope.

## Sources

- Related to #584:
  https://github.com/hgahub/duumbi/issues/584
- Parent issue:
  https://github.com/hgahub/duumbi/issues/580
- Parent product spec:
  `specs/DUUMBI-580/PRODUCT.md`
- Parent technical sequencing context:
  `specs/DUUMBI-580/TECHNICAL.md`
- Traced build product spec:
  `specs/DUUMBI-583/PRODUCT.md`
- Stage 4 triage comment:
  https://github.com/hgahub/duumbi/issues/584#issuecomment-4535968301
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/584#issuecomment-4540367441
- Runtime failure feedback note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Runtime Failure Feedback Loop.md`
- Active service and research direction:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic development map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Phase 13 roadmap note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 13 - Self-Healing & Telemetry.md`
- Architecture reference:
  `docs/architecture.md`
- Compiler lowering source:
  `src/compiler/lowering.rs`
- Telemetry source:
  `src/telemetry/mod.rs`
- Runtime source:
  `runtime/duumbi_runtime.c`
  `runtime/duumbi_runtime.h`
- Telemetry integration tests:
  `tests/integration_telemetry.rs`
