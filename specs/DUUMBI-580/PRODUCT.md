# DUUMBI-580: Define Phase 13 Self-Healing Scope And First Telemetry Slice

## Summary

Define the first executable Phase 13 product slice for runtime failure feedback.

The accepted scope is local developer/test telemetry that proves a controlled
runtime failure can be connected back to DUUMBI graph evidence. This is a
planning and product-specification parent for the Phase 13 child issues, not a
single implementation issue. It deliberately separates traced builds, trace
events, local crash artifacts, back-mapping evidence, repair-agent context, and
repair validation so self-healing does not begin as broad autonomous behavior.

For v1, the first measurable product promise is:

```text
controlled traced run failure -> local crash evidence -> graph function/block
context -> repair-ready evidence package
```

Autonomous repair loops, production telemetry ingestion, remote observability,
hot-swap, Studio dashboards, and automatic repair acceptance are not part of
this first slice.

## Problem

DUUMBI's roadmap and active product notes say runtime feedback should eventually
connect executable behavior back to semantic graph context. They also warn that
self-healing must not start as vague autonomous improvement.

Current source evidence supports parts of the future loop, but not the loop
itself:

- The compiler lowering path walks graph functions, blocks, and nodes and keeps
  graph `NodeId` values available during lowering.
- The runtime has `duumbi_panic` and runtime support functions, but panics only
  print a message and exit; there are no telemetry trace hooks or local crash
  artifacts.
- The graph patch and MCP graph tools already provide mutation and validation
  primitives, but they are not driven by runtime crash evidence.
- The seed Repair agent template exists, but its input is generic validation or
  test failure context, not a typed runtime failure evidence package.
- The intent execution repair loop is verifier-failure oriented, not runtime
  trace/back-mapping oriented.

Without this product boundary, Phase 13 could sprawl into remote telemetry,
dashboards, autonomous repair, anomaly detection, and production operations
before the core claim is proven. That would make the system harder to test,
harder to review, and less trustworthy.

## Outcome

When this Stage 6 product scope is eventually implemented through child issues:

- A maintainer can enable a traced local build or run path without changing
  default uninstrumented builds.
- A traced execution can emit graph-linked function/block telemetry events.
- A controlled runtime failure writes local, inspectable crash evidence.
- Local artifacts include enough correlation data to map runtime failure context
  back to DUUMBI graph function/block identifiers.
- A deterministic fixture proves the back-mapping contract without network
  access, external telemetry collectors, Studio, or agent repair.
- Repair-agent input is specified as an evidence-shaped context package after
  crash evidence exists.
- Repair patch validation is specified as graph validation, rebuild, tests, and
  human-reviewable evidence before any repair is accepted.
- The parent issue remains open for later Stage 7, Stage 8, implementation, and
  closure workflow; this spec PR is specification-only.

## Scope

### In Scope

- Define Phase 13's first product slice as local developer/test runtime failure
  feedback.
- Treat #583, #584, #585, and #586 as the first execution sequence:
  traced build/config, function/block trace events, local artifacts, and
  controlled runtime failure back-mapping evidence.
- Treat #588 and #587 as later repair-readiness slices gated behind accepted
  crash evidence.
- Require default builds to remain uninstrumented.
- Require traced behavior to work locally without network access or external
  collectors.
- Require function/block-level graph mapping for the first accepted milestone.
- Allow exact node-level mapping only when enough error context exists, and do
  not make exact node-level mapping a blocker for the first milestone.
- Define local artifact expectations for trace events, crash evidence, and
  trace-to-graph mapping data.
- Require deterministic tests or fixtures that prove artifact creation,
  artifact shape, and graph linkage.
- Require human-reviewable evidence before any repair result can be treated as
  accepted.
- Preserve existing graph validation, rebuild, test, and patch-review gates.

### Explicitly Out Of Scope

- Technical specification content or implementation instructions.
- Implementation code, source changes, or Ralph cycles during Stage 6.
- Remote OpenTelemetry export, external collectors, or production observability.
- Production customer crash ingestion, account-based telemetry, artifact upload,
  privacy/consent flows, release delivery, rollback, or silent updates.
- Studio telemetry dashboards, graph overlays, Ops monitoring, or alerting UI.
- Hot-swap or replacing a running binary.
- Per-operation tracing as the required v1 granularity.
- Autonomous multi-attempt repair loops.
- Automatic repair acceptance or application.
- Broad anomaly detection, adaptive baselines, or operations-agent behavior.
- Creating a new Phase 13 GitHub milestone or label as a precondition for the
  first product slice.

## Constraints And Assumptions

Facts:

- Issue #580 is open, accepted by Stage 5 on 2026-05-22, labeled `accepted` and
  `needs-spec`, and in GitHub Project status `Spec Needed` at Stage 6 intake.
- The Stage 5 decision records `Decision: Accept` and `Next state: Spec Needed`.
- Stage 5 explicitly deferred timing, mapping granularity, local artifact shape,
  and Phase 13 label/milestone questions to Stage 6.
- #583, #584, #585, #586, #588, and #587 exist as child execution candidates
  decomposed from #580.
- The active PRD defines the first runtime failure feedback promise as local
  developer/test feedback, not production customer telemetry or automatic
  production repair.
- The active service direction identifies runtime failure feedback as planned
  and frames the first credible story as traced local runs, crash evidence,
  graph back-mapping, repair-ready context, validation, and human review.
- The May 2026 roadmap says Phase 13 needs issue-backed scope before it starts.
- `src/compiler/lowering.rs` traverses graph functions, blocks, and nodes and
  uses graph `NodeId` values while lowering.
- `runtime/duumbi_runtime.c` has `duumbi_panic`, array, result, option, string,
  struct, and print support, but no trace or crash-dump artifact surface.
- `src/agents/template.rs` includes a generic Repair template.
- `src/intent/execute.rs` has verifier-failure repair precedent but does not
  consume runtime crash evidence.
- `src/patch.rs` and `src/mcp/tools/graph.rs` provide graph mutation/query and
  validation primitives relevant to later repair work.

Assumptions:

- The safest first Phase 13 proof is local and deterministic because it avoids
  privacy, account identity, upload, retention, remote collector, delivery, and
  rollback questions.
- Function/block-level mapping is sufficient for the first product milestone
  because runtime panic messages and graph block contents can narrow review
  context without tracing every operation.
- Exact node-level identification remains a long-term goal, but requiring it
  before any function/block proof would over-constrain the first slice.
- Child issue sequencing should optimize for evidence first: traced mode,
  trace events, local artifacts, controlled failure fixture, then repair input
  and validation.
- A dedicated `phase-13` label or milestone would be useful for tracking, but
  it is workflow metadata and not a product blocker.

Constraints:

- Default production-style builds must remain uninstrumented unless the user
  explicitly chooses traced behavior.
- Traced behavior must not require provider credentials, network access, Studio,
  external telemetry collectors, or remote services.
- Local telemetry artifacts must be inspectable and testable.
- Crash capture must preserve the original runtime failure signal instead of
  hiding it behind telemetry handling.
- Argument or value capture must be conservative because local crash artifacts
  can contain sensitive data.
- Repair behavior must remain behind validation, rebuild, tests, and human
  review.
- This spec PR must not mark the execution issue complete; it is a Stage 6
  review artifact only.

## Decisions

- **Decision:** Use a file-based product spec for #580.
  **Evidence:** The work is cross-module, architectural, user-visible, and
  durable enough to guide Stage 7, Stage 8, Stage 10, and Stage 11. It spans CLI
  or build UX, config, compiler lowering, runtime support, local artifacts,
  graph evidence, tests, agent context, and review workflow.

- **Decision:** Stage 6 should proceed now rather than defer until #422 and #423
  are done.
  **Evidence:** Stage 5 accepted #580 without deferral on 2026-05-22. #422 and
  #423 are related Phase 16 context, but the Stage 5 decision explicitly routed
  #580 to `Spec Needed`.

- **Decision:** The first accepted telemetry milestone is function/block
  back-mapping, not mandatory exact node-level mapping.
  **Evidence:** The Phase 13 archive recommends function/block instrumentation
  to avoid per-operation overhead, while the current source already has graph
  function/block/node structure available during lowering. Child #584 and #586
  both scope the first proof to function/block identifiers and leave exact
  node-level mapping as later work if needed.

- **Decision:** The minimum local artifact contract is trace events, crash
  evidence, and trace-to-graph mapping data.
  **Evidence:** #585 names traces, crash dumps, and mapping artifacts as the
  durable evidence needed for runtime failure review. The PRD also says the
  first promise is local crash evidence mapped to graph context.

- **Decision:** `.duumbi/telemetry/` is the default product location unless the
  technical spec identifies a safer equivalent.
  **Evidence:** The architecture reference already lists telemetry artifacts
  under `.duumbi/telemetry/`, and #585 asks for a conservative project-local
  path or equivalent.

- **Decision:** Remote export, Studio dashboards, anomaly detection, Ops
  monitoring, hot-swap, and autonomous repair are later product tracks.
  **Evidence:** The active PRD explicitly excludes cloud/customer crash
  ingestion, silent updates, hot-swap, and autonomous repair acceptance from the
  first slice. Child issues also mark these areas out of scope.

- **Decision:** Repair-agent input and repair validation should be specified
  after crash evidence is explicit.
  **Evidence:** The #580 decomposition says #583-#586 should prove telemetry and
  back-mapping before #588 and #587 drive repair execution work.

- **Decision:** A Phase 13 label or milestone is useful but not required before
  this spec can be reviewed.
  **Evidence:** Stage 5 deferred the question. No Phase 13 milestone or label
  existed during the decomposition review, but child issues were still created
  and added to the DUUMBI Project as Todo.

- **Decision:** This spec PR must be specification-only and must not mark #580
  complete.
  **Evidence:** Stage 6 creates a product spec for review. Product approval,
  technical specification, implementation, review, and closure evidence remain
  later workflow stages.

## Behavior

### Defaults

- Default builds remain uninstrumented.
- Traced behavior is opt-in through a product surface accepted by the later
  technical spec.
- Traced telemetry is local by default.
- The first mapping granularity is graph function/block.
- Repair automation is off by default and not part of the first telemetry proof.

### Inputs

- A DUUMBI workspace with graph modules.
- A traced build or traced run path selected by the user.
- Local telemetry configuration with conservative defaults.
- A controlled runtime failure fixture.
- Runtime panic or failure message.
- Compiler-emitted or locally persisted mapping between runtime identifiers and
  graph function/block `@id` values.
- Existing graph validation, build, test, and patch-review primitives for later
  repair validation.

### Outputs

- Local trace event artifacts for traced runs.
- Local crash evidence for controlled runtime failures.
- Local trace-to-graph mapping data.
- A deterministic test or fixture proving that a controlled runtime failure
  produces graph-linked function/block evidence.
- A repair-ready context contract after crash evidence exists.
- A human-reviewable repair validation evidence report in later child work.

### Visible States

- Uninstrumented default build: no trace hooks emitted, no telemetry artifacts
  expected, and no runtime behavior change from telemetry.
- Traced build/run enabled: local telemetry artifacts may be created under the
  accepted telemetry path.
- Controlled traced failure: runtime failure remains visible, and crash evidence
  records failure message plus trace correlation.
- Mapping available: a maintainer can identify the graph function/block
  associated with the failure.
- Mapping unavailable or malformed: the run reports missing evidence clearly and
  does not claim repair readiness.
- Repair context ready: crash evidence has enough graph-linked context to build
  a Repair-agent input package.
- Repair candidate validated: graph validation, rebuild, and relevant tests pass
  before human review.

### Error And Empty States

- If traced mode is not enabled, telemetry artifacts are not expected.
- If a telemetry directory or artifact cannot be written, the user sees a clear
  local artifact error while the original runtime failure remains visible.
- If mapping data is missing, malformed, or points to a graph identifier that no
  longer exists, back-mapping fails with explicit missing-evidence output.
- If a crash artifact exists without trace correlation, the system may present
  raw crash evidence but must not claim graph back-mapping success.
- If exact node-level context cannot be derived, the system reports
  function/block evidence and states that exact node identification is not
  available.
- If repair validation fails, the evidence report identifies the failing gate
  and does not treat the repair as accepted.

### Privacy And Locality

- v1 telemetry artifacts stay local.
- No external collector, account, upload, remote endpoint, or network service is
  part of the first slice.
- Argument/value capture is omitted or minimized by default unless later review
  accepts a safe, explicit field-level contract.
- Artifact examples and tests should avoid secret-like or user-private values.

### Accessibility And Reviewability

- CLI or log output for traced failure evidence should be concise and readable.
- Artifact formats should be line-oriented or otherwise easy to inspect in
  tests and PR review.
- Review evidence should link the original crash evidence, mapped graph
  context, validation output, test output, and proposed repair when repair work
  begins.

### Invariants

- Query mode remains read-only.
- Default build behavior does not change due to telemetry work.
- Traced local telemetry must not mutate graph source files.
- Runtime telemetry does not bypass graph validation.
- Repair candidates do not bypass rebuilds, tests, or human review.
- Production self-healing claims are not made from local developer/test telemetry
  evidence alone.

## BDD Scenarios

Feature: Phase 13 local runtime failure feedback

Rule: Default builds are not instrumented

Scenario: Building without traced telemetry
Given a DUUMBI workspace with a valid graph program
When the maintainer runs the default build path
Then the build does not emit telemetry trace calls
And no telemetry artifact is required for success
And the compiled program behavior matches the existing uninstrumented path

Scenario: Enabling traced local execution
Given a DUUMBI workspace with a valid graph program
And traced telemetry has been explicitly selected
When the maintainer builds or runs the program through the traced path
Then local trace event artifacts are written under the accepted telemetry path
And the trace events include stable runtime identifiers for graph functions and blocks
And no remote telemetry service is required

Rule: Controlled failures produce graph-linked evidence

Scenario: Controlled runtime failure writes crash evidence
Given a traced DUUMBI program that intentionally reaches a runtime panic path
When the program runs and fails
Then the original runtime failure remains visible
And a local crash evidence artifact is written
And the artifact includes the panic message or failure reason
And the artifact includes trace correlation data

Scenario: Crash evidence maps to graph function and block context
Given a traced failing run has produced crash evidence
And trace-to-graph mapping data exists for the compiled graph
When the maintainer inspects the back-mapping output
Then the failure is linked to a DUUMBI graph function identifier
And the failure is linked to a DUUMBI graph block identifier
And the output does not claim exact node-level context unless that evidence is present

Scenario: Missing mapping prevents repair readiness
Given a traced failing run has produced crash evidence
But the trace-to-graph mapping artifact is missing or malformed
When the maintainer asks for failure back-mapping
Then the system reports that graph mapping evidence is unavailable
And the system does not claim the crash is repair-ready
And the raw crash evidence remains available for inspection

Rule: Repair work remains gated behind evidence and review

Scenario: Repair-agent context is built from crash evidence
Given a controlled crash artifact with graph-linked function and block context
When the later repair-agent input contract is applied
Then the Repair-agent context includes the failure reason, trace correlation, graph identifiers, and relevant graph context
And it includes validation and test expectations
And it does not rely on raw logs alone

Scenario: Repair candidate requires validation and human review
Given a Repair agent proposes a graph patch from crash evidence
When the repair validation pipeline evaluates the proposal
Then graph validation must pass
And the project must rebuild
And relevant tests must pass
And the output is a human-reviewable evidence report
And the repair is not silently accepted or applied

Rule: Product boundaries remain explicit

Scenario: Production telemetry is requested before local proof exists
Given Phase 13 local telemetry back-mapping has not been accepted as complete
When a user asks for production crash ingestion or remote observability
Then the request is treated as out of scope for the first Phase 13 slice
And it is routed to later product specification instead of being added to this work

Scenario: Exact node-level mapping is unavailable in the first slice
Given a traced failure has function and block mapping evidence
But no exact graph operation evidence is available
When the maintainer reviews the crash evidence
Then the system reports function/block-level evidence as the accepted v1 result
And it identifies exact node-level mapping as unavailable or future work

## Tasks

- Product and workflow setup:
  - Keep #580 as the parent issue for Phase 13 scope and first-slice review.
  - Use #583-#586 as the first execution sequence.
  - Keep #588 and #587 behind accepted crash evidence.

- Traced build/config product surface:
  - Define opt-in traced behavior while preserving default builds.
  - Define local telemetry config defaults.
  - Keep remote export and external collector configuration out of v1 unless a
    later accepted spec adds it.

- Runtime trace events:
  - Define function/block trace event expectations.
  - Require stable joinability from runtime identifiers to graph `@id` values.
  - Exclude mandatory per-operation tracing from v1.

- Local artifacts:
  - Define trace event artifact expectations.
  - Define crash evidence artifact expectations.
  - Define trace-to-graph mapping artifact expectations.
  - Keep artifact formats deterministic and testable.

- Controlled failure proof:
  - Add a deterministic runtime failure fixture in later implementation work.
  - Assert artifact creation, artifact shape, and graph linkage.
  - Ensure the fixture is local and does not require providers, network, Studio,
    or external telemetry infrastructure.

- Repair readiness:
  - Define the crash-evidence-to-repair-context contract after back-mapping
    evidence is present.
  - Define the repair validation evidence report after the input contract is
    accepted.
  - Require graph validation, rebuild, relevant tests, and human review before
    any repair outcome is considered accepted.

## Checks

- Stage 7 product review confirms:
  - Stage 5 acceptance is correctly represented.
  - The first Phase 13 slice is local developer/test telemetry and
    back-mapping, not production self-healing.
  - The Stage 5 questions are answered by explicit decisions.
  - The child issue sequence is coherent.
  - No technical specification or implementation detail is over-prescribed.

- Stage 8 technical spec should later define:
  - The exact traced build/run surface.
  - The telemetry config shape and defaults.
  - The trace event ABI or equivalent boundary.
  - The artifact schemas.
  - The graph mapping representation.
  - The controlled failure fixture and tests.
  - The validation evidence expected for later repair work.

- Implementation PRs should later prove:
  - Default builds remain uninstrumented.
  - Traced local runs produce deterministic trace artifacts.
  - Controlled failures produce crash evidence.
  - Back-mapping yields graph function/block context.
  - Missing or malformed artifacts produce clear missing-evidence states.
  - Repair input and validation work, when implemented, remains gated behind
    crash evidence and human review.

- Suggested test coverage for later implementation:
  - Unit tests for telemetry config parsing and defaults.
  - Compiler/lowering tests or snapshots proving traced versus default output
    behavior.
  - Runtime tests or integration fixtures for trace event and crash artifact
    creation.
  - Back-mapping tests for valid, missing, and malformed mapping artifacts.
  - Repair context conversion tests after #588.
  - Repair validation pass/fail tests after #587.

- Manual review evidence for later implementation:
  - A small controlled failing graph program.
  - The traced run command used.
  - The local trace/crash/mapping artifacts generated.
  - The graph function/block evidence identified.
  - The validation/test output for any later repair candidate.

## Open Questions

No blocking open questions for Stage 7 product review.

Non-blocking questions for Stage 8 or later implementation:

- Should traced behavior be exposed through build only, run only, or both?
- Should `.duumbi/config.toml` own telemetry defaults, or should a dedicated
  telemetry file exist for artifact settings?
- Should argument snapshots be omitted in v1, opt-in only, or redacted by
  default?
- What stable runtime identifier scheme best survives compiler ordering changes?
- When function/block back-mapping is proven, what additional evidence is needed
  to claim exact node-level mapping?
- Should the GitHub project add a dedicated Phase 13 milestone or label after
  this product spec is reviewed?

## Sources

- Issue #580: https://github.com/hgahub/duumbi/issues/580
- Stage 5 human acceptance decision:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4522085626
- Phase 13 decomposition comment:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4505735849
- Child #583, traced build mode and telemetry configuration:
  https://github.com/hgahub/duumbi/issues/583
- Child #584, function/block trace events:
  https://github.com/hgahub/duumbi/issues/584
- Child #585, crash dumps and trace-to-graph mapping artifacts:
  https://github.com/hgahub/duumbi/issues/585
- Child #586, controlled runtime failure back-mapping evidence:
  https://github.com/hgahub/duumbi/issues/586
- Child #588, repair-agent input contract from crash evidence:
  https://github.com/hgahub/duumbi/issues/588
- Child #587, repair patch validation and human-reviewable evidence:
  https://github.com/hgahub/duumbi/issues/587
- DUUMBI PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- DUUMBI Service and Research Direction:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- DUUMBI Product Roadmap 2026-05:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Product Roadmap 2026-05.md`
- DUUMBI Phase 13 Self-Healing and Telemetry archive:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 13 - Self-Healing & Telemetry.md`
- Architecture reference: `docs/architecture.md`
- Compiler lowering source: `src/compiler/lowering.rs`
- Runtime source: `runtime/duumbi_runtime.c`
- Repair template source: `src/agents/template.rs`
- Intent execution source: `src/intent/execute.rs`
- Graph patch source: `src/patch.rs`
- MCP graph tools source: `src/mcp/tools/graph.rs`
