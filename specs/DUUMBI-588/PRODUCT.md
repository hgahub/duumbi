# DUUMBI-588: Define Repair-Agent Input Contract From Crash Evidence

## Summary

Define the product contract for turning Phase 13 runtime crash evidence into a
reviewable Repair-agent input package.

This issue is a focused Phase 13 child slice under #580. It depends on local
trace/crash evidence and graph back-mapping being available before a Repair
agent is allowed to reason about a runtime failure. The contract is deliberately
evidence-shaped rather than prompt-shaped: a repair prompt or context package
must be derived from mapped crash artifacts, graph function/block identifiers,
bounded graph context, and explicit validation/test expectations.

For v1, the accepted product surface is:

```text
mapped crash evidence -> bounded graph-linked repair context ->
repair proposal expectations, with validation and human review still required
```

This spec does not approve automatic patch generation, patch application,
autonomous retry loops, hot-swap repair, production self-healing, or automatic
repair acceptance.

## Problem

DUUMBI's Phase 13 direction requires runtime failures to connect back to the
semantic graph before repair automation can be trusted. A generic Repair agent
template already exists, and the parent Phase 13 implementation has a local
telemetry foundation, but an agent should not receive a free-form crash log and
invent its own repair scope.

The risk is that repair work becomes prompt-shaped rather than evidence-shaped.
If a Repair agent is started from raw logs alone, it can miss the graph element
that failed, overreach into unrelated graph changes, bypass validation, or
claim success without a reproducible failing artifact.

The product need is a narrow input contract that says:

- what crash evidence is required before repair context exists.
- which graph identifiers and trace correlations must be present.
- how much graph context is safe to include.
- what privacy-sensitive values are omitted or explicitly gated.
- what validation and test expectations must accompany any proposed repair.
- how existing graph query/mutation and validation boundaries remain in force.

## Outcome

When this issue is implemented:

- A mapped runtime crash can be converted into a structured Repair-agent context
  package.
- The context is derived from local telemetry artifacts and graph mapping
  evidence, not from raw logs alone.
- The context includes the failure reason, trace correlation, graph function
  identifier, graph block identifier, optional exact node identifier when
  available, bounded graph context, and evidence provenance.
- The context includes validation expectations and test expectations that a
  later repair candidate must satisfy before it is reviewable.
- Missing, malformed, stale, or unmapped evidence prevents repair readiness
  instead of producing a vague prompt.
- Argument or runtime value snapshots are absent by default unless a later
  accepted contract explicitly allows safe, redacted, opt-in capture.
- Repair candidates remain proposals. They must not be treated as applied,
  accepted, or successful by this contract.
- Existing graph query, graph mutation, graph validation, rebuild, test, and
  human-review boundaries are preserved.
- The execution issue remains open for later Stage 7, Stage 8, implementation,
  review, and closure evidence. This product spec PR is specification-only.

## Scope

### In Scope

- Define the crash-evidence-to-repair-context product contract for #588.
- Require mapped crash evidence as the source of repair context.
- Include crash message or failure reason, trace IDs, graph function ID, graph
  block ID, optional exact node ID, bounded graph context, and evidence source
  metadata in the context.
- Define repair-readiness states for mapped evidence, missing evidence, malformed
  evidence, unmapped trace IDs, stale graph IDs, and unavailable exact node
  evidence.
- Require validation expectations for proposed repairs, including graph patch
  parsing, atomic patch behavior, graph parsing/building, graph validation,
  native rebuild, relevant tests, and human review.
- Require test expectations for the controlled crash path, targeted regression,
  and unchanged default untraced behavior.
- Specify that existing graph query/mutation tools remain bounded by their
  read/write mode and validation behavior.
- Require examples or tests that prove a controlled crash artifact can become a
  repair context without provider calls.
- Keep the first contract focused on one controlled failure and
  function/block-level graph context.
- Document how later exact-node evidence can fit the contract without making it
  mandatory in v1.

### Explicitly Out Of Scope

- Technical specification content or implementation instructions during Stage 6.
- Implementation code, source changes outside this product spec, or Ralph
  cycles during Stage 6.
- Generating repair patches from an LLM.
- Applying repair patches.
- Marking a repair as accepted or successful.
- Autonomous retry loops, multi-agent repair orchestration, or self-healing
  policy.
- Hot-swap or replacing a running binary.
- Production crash ingestion, account-based telemetry, artifact upload,
  retention, privacy consent, release delivery, or rollback behavior.
- Studio repair UX, graph overlays, dashboards, alerting, or Ops monitoring.
- Broad anomaly detection or baseline operations work.
- Making exact node-level evidence mandatory before the function/block v1
  contract can be reviewed.
- Changing Query mode from read-only behavior.

## Constraints And Assumptions

Facts:

- Issue #588 is open and accepted for specification.
- Issue #588 is labeled `accepted` and `needs-spec`.
- The Stage 5 decision comment on 2026-06-02 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- Stage 4 routed #588 to `Needs Human Acceptance` as a Phase 13 repair-input
  contract after telemetry and back-mapping evidence.
- #588 is a child issue of #580.
- #580 was approved, technically specified, implemented, and closed with local
  Phase 13 telemetry/back-mapping foundation evidence.
- The active PRD says the first runtime failure feedback promise is local
  developer/test feedback, not production customer telemetry or automatic
  production repair.
- The active service direction says the local Phase 13 foundation exists in
  source: opt-in traced builds, local trace/crash artifacts, graph
  function/block back-mapping, telemetry inspection, repair crash context, and
  repair validation evidence contracts.
- Current source contains telemetry types and helpers for mapped crash context,
  trace correlation, bounded graph context, validation expectations, and repair
  validation evidence.
- `src/agents/template.rs` includes a generic Repair template whose current
  prompt is validation/test-failure oriented rather than a runtime
  crash-evidence contract.
- `src/intent/execute.rs` contains verifier-failure repair precedent, but that
  loop is not the same as runtime crash evidence back-mapping.
- `src/mcp/tools/graph.rs` exposes graph query, graph mutate, graph validate,
  and graph describe tools. Graph mutation validates patched graph state before
  writing.
- Exact node-level crash evidence remains unavailable in the accepted v1 local
  foundation unless a later approved issue adds stronger evidence.

Assumptions:

- Function/block-level graph context is the right v1 minimum because it matches
  the accepted Phase 13 local telemetry foundation and avoids overpromising
  exact node evidence.
- The repair context should be serializable and reviewable so tests and
  reviewers can inspect it without invoking a provider.
- Provider calls are not needed to prove the input contract. They belong to
  later repair-proposal behavior, if accepted.
- Repair context should include enough surrounding graph information to focus a
  proposal, but not so much that the agent is encouraged to rewrite unrelated
  parts of the program.
- Value and argument snapshots are privacy-sensitive. The safe v1 default is to
  omit them unless later work accepts explicit redaction and opt-in behavior.
- The existing graph mutation and validation pipeline should remain the product
  boundary for any later candidate patch.

Constraints:

- Repair context must not exist unless crash evidence can be mapped to graph
  function/block context.
- Raw logs alone must not be accepted as repair-ready context.
- The context must preserve the original crash signal instead of hiding it
  behind repair automation.
- Context assembly must be local and deterministic for the controlled v1
  failure path.
- Context assembly must not mutate JSON-LD graph source files.
- Context assembly must not require network access, provider credentials,
  Studio, Slack, GitHub, or external telemetry collectors.
- Any repair candidate produced later must remain behind graph validation,
  rebuild, relevant tests, and human review.
- This spec PR must not mark #588 complete; it is a Stage 6 review artifact
  only.

## Decisions

- **Decision:** Use a file-based product spec for #588.
  **Evidence:** The work is architectural, cross-module, and durable. It spans
  telemetry artifacts, graph identity, agent input shape, MCP tool boundaries,
  validation expectations, test evidence, and Phase 13 sequencing.

- **Decision:** Make mapped crash evidence mandatory before repair context
  exists.
  **Evidence:** #588 acceptance criteria require graph-linked failure evidence
  rather than raw logs alone. The parent #580 scope says repair-agent context
  must be gated behind crash evidence.

- **Decision:** Use function/block graph context as the v1 required mapping
  level, with exact node evidence optional.
  **Evidence:** The accepted #580 product direction and current source evidence
  identify function/block-level back-mapping as the first local foundation.
  Exact node-level mapping remains future work unless later evidence supports
  it.

- **Decision:** Keep repair context local and provider-independent.
  **Evidence:** The product contract should be testable from controlled crash
  artifacts without an LLM call. Provider behavior belongs to later repair
  proposal work.

- **Decision:** Include validation and test expectations in the context.
  **Evidence:** Phase 13 requires repair proposals to pass graph validation,
  rebuild, relevant tests, and human review. Putting those expectations in the
  input package keeps the Repair agent scoped to reviewable outcomes.

- **Decision:** Do not include raw argument or runtime value snapshots by
  default.
  **Evidence:** Stage 4 and #588 both flag privacy/sensitivity concerns.
  Function/block evidence is enough for the v1 controlled failure contract.

- **Decision:** Preserve existing graph tool validation boundaries.
  **Evidence:** MCP graph mutation already validates before writing. A repair
  context may point the agent at relevant graph query/mutation concepts, but it
  must not create an alternate write path that bypasses validation.

- **Decision:** This spec PR must be specification-only and must leave #588
  open.
  **Evidence:** Stage 6 creates a reviewable product spec. Product approval,
  technical specification, implementation, implementation review, and closure
  evidence remain later workflow stages.

## Behavior

### Defaults

- No repair context is produced for default untraced runs.
- No repair context is produced from raw stderr, logs, or crash text alone.
- Context assembly is local, deterministic, and read-only.
- Context assembly does not invoke a provider.
- Context assembly does not apply a patch.
- Exact node evidence is optional and may be explicitly unavailable in v1.
- Argument and value snapshots are omitted by default.

### Inputs

- A local crash artifact from a traced runtime failure.
- Trace correlation from the crash artifact.
- A trace-to-graph mapping artifact for graph function/block identifiers.
- The current or referenced graph source needed to assemble bounded context.
- The crash message or failure reason.
- Optional explicit artifact paths when a caller wants a specific crash/map
  pair instead of the default local artifact location.

### Outputs

- A structured Repair-agent context package.
- A crash message or failure reason.
- Runtime function and block trace IDs used for correlation.
- Mapped graph function and block identifiers.
- Optional exact graph node identifier when evidence exists.
- Bounded graph context for the mapped function and block.
- Evidence provenance that identifies the context as mapped telemetry evidence.
- Validation expectations for any later candidate patch.
- Test expectations for the controlled failure, targeted regression, and
  default untraced behavior.
- A clear not-ready result when required evidence is missing or invalid.

### Visible States

- Evidence unavailable: no crash artifact or mapping artifact exists.
- Evidence malformed: an artifact exists but cannot be parsed or fails the
  accepted schema.
- Evidence unmapped: crash trace IDs do not map to graph function/block entries.
- Evidence stale: mapped graph IDs no longer correspond to available graph
  context.
- Function/block mapped: the crash is linked to graph function and block IDs.
- Exact node unavailable: function/block mapping exists, but exact node evidence
  is absent and explicitly reported as unavailable.
- Repair context ready: mapped crash evidence and bounded graph context exist,
  and validation/test expectations are attached.
- Repair proposal pending validation: a later agent has proposed a patch, but
  this contract still treats it as unaccepted until validation and human review
  happen outside #588.

### Error And Empty States

- If traced mode was not active when the crash artifact was written, repair
  readiness is rejected.
- If the crash artifact is empty, the system reports missing crash evidence.
- If the trace map is missing, malformed, or points to unrelated graph IDs, the
  system reports missing or unmapped graph evidence.
- If multiple crash entries exist, the context must identify which entry was
  used or accept an explicit artifact selection. Silent ambiguity is not
  acceptable for review evidence.
- If exact node evidence is absent, the context still may be ready at
  function/block level but must not imply exact node mapping.
- If graph context cannot be bounded safely, the context is not repair-ready
  until the missing boundary is explicit.

### Graph Tool Boundaries

- Graph query and graph describe behavior may be used to inspect relevant graph
  context.
- Query mode remains read-only.
- A Repair agent may propose graph patch operations only as a reviewable
  candidate.
- Any graph mutation must use the existing graph patch and validation path.
- A proposed repair must not be reported as successful merely because an agent
  produced a patch-shaped response.
- Repair validation, rebuild, relevant tests, and human review are required
  before any repair can be accepted by later workflow stages.

### Privacy And Locality

- v1 repair context is assembled from local artifacts.
- No external collector, account, upload, remote endpoint, or network service is
  part of this contract.
- Runtime values, arguments, heap content, stack snapshots, and secrets are not
  included by default.
- If future work allows value snapshots, the context must mark them as explicit,
  redacted when needed, and opt-in.
- Test fixtures should avoid secret-like or user-private values.

### Invariants

- The original runtime failure remains visible and reproducible.
- Repair context is not accepted from raw logs alone.
- Context assembly does not mutate graph source files.
- Repair context does not bypass graph validation.
- Repair candidates do not bypass rebuilds, tests, or human review.
- Production self-healing claims are not made from this local input contract.

## BDD Scenarios

Feature: Repair-agent input contract from crash evidence

Rule: Repair context requires mapped crash evidence

Scenario: Controlled crash becomes repair context
Given a traced DUUMBI run has produced a crash artifact
And the crash artifact includes a failure reason and trace correlation
And a trace-to-graph map links the trace IDs to graph function and block IDs
When repair context is assembled from the artifacts
Then the context includes the failure reason
And the context includes the function and block trace IDs
And the context includes the mapped graph function and block IDs
And the context includes bounded graph context
And the context includes validation and test expectations

Scenario: Raw logs are not repair-ready
Given a runtime failure message exists in stderr or a log file
But no mapped crash artifact and trace-to-graph map are available
When repair context is requested
Then the system reports that mapped crash evidence is required
And no Repair-agent context is produced
And no repair proposal is requested from a provider

Scenario: Unmapped trace IDs block repair readiness
Given a traced crash artifact contains function and block trace IDs
But the trace-to-graph map does not contain matching graph entries
When repair context is assembled
Then the system reports unmapped trace evidence
And the crash is not marked repair-ready
And the raw crash evidence remains available for inspection

Scenario: Exact node evidence is unavailable in v1
Given a crash has mapped graph function and block evidence
But no exact graph node evidence is present
When repair context is assembled
Then the context includes the function and block graph identifiers
And the exact node field is empty or explicitly unavailable
And the context does not claim exact node-level mapping

Rule: The context is bounded and privacy-aware

Scenario: Argument values are omitted by default
Given a traced crash occurred while runtime values may have existed in memory
When repair context is assembled for v1
Then the context omits argument and runtime value snapshots by default
And the context still includes graph-linked failure evidence
And any future value capture requirement is treated as separate accepted scope

Scenario: Stale graph IDs prevent a misleading context
Given crash artifacts map to graph function and block identifiers
But the current graph source no longer contains the mapped context
When repair context is assembled
Then the system reports stale or missing graph context
And it does not create a vague prompt from the crash message alone

Scenario: Multiple crash entries require traceable selection
Given a crash dump contains more than one crash entry
When repair context is assembled
Then the resulting context identifies which crash entry was used
Or the caller can provide an explicit crash artifact selection
And the review evidence can trace the context back to that selected crash

Rule: Repair proposals remain gated

Scenario: Repair agent receives validation expectations
Given a repair context has been assembled from mapped crash evidence
When the context is prepared for a later Repair-agent proposal
Then the context tells the agent that any patch must parse as a graph patch
And graph validation must pass
And the project must rebuild
And relevant tests must pass
And human review is still required

Scenario: Patch-shaped output is not accepted automatically
Given a Repair agent later proposes a graph patch from the repair context
When the proposal exists but validation has not run
Then the proposal is not accepted
And the graph source is not silently changed
And the result remains pending validation and human review

Scenario: Graph mutation cannot bypass validation
Given a Repair agent later proposes operations for a mapped graph context
When those operations are evaluated through graph mutation behavior
Then the existing parse, build, and graph validation path is required
And a validation failure prevents the mutation from being treated as successful
And the repair remains unaccepted

Rule: Product boundaries remain explicit

Scenario: Production self-healing is requested from this contract
Given the repair context contract is available for local developer/test evidence
When a user asks for production crash ingestion or automatic self-repair
Then the request is out of scope for #588
And it must be routed to later product specification

Scenario: Provider execution is requested before context evidence exists
Given mapped crash evidence is missing
When a user asks to start a Repair agent anyway
Then the system refuses repair readiness
And it asks for crash evidence or routes to telemetry/back-mapping work
And no provider call is required to prove the refusal

## Tasks

- Product and workflow setup:
  - Keep #588 as the repair-agent input contract child issue under #580.
  - Use non-closing workflow references such as `Related to #588`.
  - Keep the execution issue open for later workflow stages.

- Evidence contract:
  - Define the required crash evidence, trace correlation, and graph mapping
    inputs.
  - Define not-ready behavior for missing, malformed, unmapped, stale, or
    untraced evidence.
  - Keep function/block graph context as the required v1 mapping level.

- Context package:
  - Define the required fields for a Repair-agent context package.
  - Include failure reason, trace IDs, graph identifiers, bounded graph context,
    optional exact node evidence, and evidence provenance.
  - Include validation and test expectations as first-class fields.
  - Omit argument/value snapshots by default.

- Agent boundary:
  - Make the context suitable for a later Repair-agent proposal without making
    provider execution part of #588.
  - Require any later patch proposal to stay compatible with graph patch and
    graph validation boundaries.
  - Keep repair acceptance, validation evidence, and human-review state outside
    this issue unless a later approved workflow routes them here.

- Tests and examples:
  - Prove that a controlled crash artifact plus trace map can become a context
    package.
  - Prove missing or unmapped evidence prevents repair readiness.
  - Prove exact node evidence can be absent without being misrepresented.
  - Prove the context serializes or can otherwise be reviewed deterministically.

- Review evidence:
  - Link the source issue, Stage 5 acceptance comment, parent #580 evidence,
    relevant child issues, source surfaces, and test expectations.
  - Confirm the PR is specification-only and leaves #588 open.

## Checks

- Stage 7 product review confirms:
  - Stage 5 acceptance is correctly represented.
  - The product spec stays within #588 and does not create a technical spec.
  - Repair context is derived from mapped crash evidence, not raw logs alone.
  - Function/block graph mapping is the required v1 contract, and exact node
    evidence remains optional.
  - Privacy-sensitive values are omitted by default.
  - Repair candidates remain behind validation, rebuild, tests, and human
    review.
  - No auto-closing issue references are used in the spec-only PR.

- Stage 8 technical spec should later define:
  - The exact artifact inputs and context schema shape.
  - How graph context is bounded.
  - How explicit crash selection works when more than one crash entry exists.
  - How stale graph identifiers are detected.
  - The exact tests and fixtures for the controlled crash conversion path.
  - How the context is handed to an agent prompt without allowing mutation
    bypasses.

- Implementation PRs should later prove:
  - Context assembly succeeds from a controlled crash artifact and trace map.
  - Context assembly fails clearly for missing crash artifacts.
  - Context assembly fails clearly for missing or malformed trace maps.
  - Context assembly fails clearly for unmapped trace IDs.
  - Exact node evidence remains explicit when unavailable.
  - Default untraced behavior does not create repair context.
  - The context includes validation and test expectations.
  - No provider call is required for contract tests.

- Suggested test coverage for later implementation:
  - Unit tests for context serialization and required fields.
  - Unit tests for missing, malformed, unmapped, and stale evidence.
  - Unit tests for privacy defaults around value/argument capture.
  - Integration coverage using the controlled runtime failure fixture.
  - Regression coverage that default untraced builds do not become repair-ready.

- Manual review evidence for later implementation:
  - The controlled failing DUUMBI graph program.
  - The traced build/run command used.
  - The crash artifact and trace map used.
  - The mapped graph function/block context.
  - The generated repair context package.
  - The validation/test expectations included in the package.

## Open Questions

No blocking open questions for Stage 7 product review.

Non-blocking questions for Stage 8 or later implementation:

- Should context assembly default to the latest crash entry, require explicit
  selection, or support both with clear provenance?
- What is the smallest useful bounded graph context around a mapped function and
  block?
- Should exact node evidence become required after a later mapping issue proves
  it, or remain optional for compatibility?
- Should value snapshots ever be allowed in repair context, and if so, what
  redaction and opt-in rules are required?
- How should stale graph IDs be detected when the graph source has changed
  after the crash artifact was written?

## Sources

- Issue #588: https://github.com/hgahub/duumbi/issues/588
- Stage 4 triage refill for #588:
  https://github.com/hgahub/duumbi/issues/588#issuecomment-4599034980
- Stage 5 human acceptance decision for #588:
  https://github.com/hgahub/duumbi/issues/588#issuecomment-4606221781
- Parent issue #580:
  https://github.com/hgahub/duumbi/issues/580
- Phase 13 decomposition comment:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4505735849
- Stage 12 closure evidence for #580:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4525647674
- Parent product spec: `specs/DUUMBI-580/PRODUCT.md`
- Parent technical spec: `specs/DUUMBI-580/TECHNICAL.md`
- Child #583, traced build mode and telemetry configuration:
  https://github.com/hgahub/duumbi/issues/583
- Child #584, function/block trace events:
  https://github.com/hgahub/duumbi/issues/584
- Child #585, crash dumps and trace-to-graph mapping artifacts:
  https://github.com/hgahub/duumbi/issues/585
- Child #586, controlled runtime failure back-mapping evidence:
  https://github.com/hgahub/duumbi/issues/586
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
- Telemetry and repair-context source: `src/telemetry/mod.rs`
- Repair template source: `src/agents/template.rs`
- Intent verifier-repair precedent: `src/intent/execute.rs`
- MCP graph tools source: `src/mcp/tools/graph.rs`
