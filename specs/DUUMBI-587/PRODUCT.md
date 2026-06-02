# DUUMBI-587: Validate Repair Patches And Produce Human-Reviewable Evidence

## Summary

Define the product guardrail for validating a runtime-failure repair candidate
before DUUMBI treats it as reviewable.

This issue is a focused Phase 13 repair-readiness child under #580. It follows
the local telemetry/back-mapping foundation and the repair-agent input contract
from mapped crash evidence. The accepted product behavior is not autonomous
self-healing. It is a validation and evidence boundary:

```text
mapped crash context + proposed graph patch -> validation gates ->
human-reviewable evidence report -> pending human review
```

A repair candidate is not successful merely because an agent proposed a patch.
It must parse through the canonical GraphPatch contract, apply atomically, pass
graph parsing, pass graph building, pass graph validation, rebuild natively,
pass relevant tests, and produce an evidence report that a human can inspect.
Even when all local gates pass, the repair remains pending human review and is
not accepted or applied silently.

This specification PR is related to #587 and must leave the execution issue
open for Stage 7 review, Stage 8 technical specification, implementation,
review, and Stage 12 closure evidence.

## Problem

Self-healing becomes unsafe if the system accepts a generated patch because it
looks plausible or because an agent claims success. Runtime crash evidence and
repair context only establish where a repair might be needed. They do not prove
that a candidate patch is constrained, valid, buildable, testable, or safe to
apply.

The current DUUMBI source already has useful pieces:

- mapped crash context can be derived from local trace/crash artifacts.
- GraphPatch operations apply atomically over JSON-LD values.
- MCP graph mutation validates patched graph state before writing.
- telemetry repair validation evidence types can record local gate results
  while keeping human review required.
- intent execution has a verifier-failure repair precedent, but that loop is
  oriented around failed intent tests and can write repaired graph files after a
  provider mutation.

The missing product contract is the guardrail between a proposed runtime-crash
repair and any claim that the repair is complete. DUUMBI needs an observable
validation pipeline and evidence report that links the original crash, the
proposed change, validation output, rebuild output, relevant tests, and human
review state. Without that contract, repair behavior could become an opaque
agent loop that mutates graph source, hides failed validation, or bypasses the
human acceptance boundary required by Phase 13.

## Outcome

When this issue is implemented:

- A repair candidate can be evaluated only from mapped repair crash context and
  a proposed GraphPatch-shaped change.
- Invalid or malformed patch payloads fail before any graph source is changed.
- Patch application is atomic: a failed patch leaves the original source
  unchanged.
- Patched JSON-LD is parsed, converted to DUUMBI graph IR, and graph-validated
  before the candidate can be reported as locally valid.
- The candidate must rebuild successfully after validation.
- Relevant tests must pass before the candidate can be reported as locally
  valid.
- The output is a human-reviewable evidence report that links:
  - the original mapped crash context.
  - the proposed patch or diff summary.
  - the graph/source artifacts considered.
  - each validation gate and result.
  - rebuild and test output summaries.
  - the final human-review requirement.
- A candidate with all local gates passing is marked as pending human review,
  not accepted for application.
- Failed gates produce explicit not-ready states and preserve the original
  crash evidence for inspection.
- Default behavior stops at reviewable evidence and does not silently apply,
  accept, merge, deploy, hot-swap, or close the repair.
- Existing Query mode, graph validation, rebuild, test, and GitHub review
  boundaries are preserved.
- The execution issue remains open for later workflow stages. This product spec
  PR is specification-only.

## Scope

### In Scope

- Define the repair patch validation product contract for #587.
- Require mapped repair crash context as the starting point for validation.
- Require a proposed patch to parse through the canonical GraphPatch contract.
- Require atomic patch behavior so failed patch application does not modify the
  source graph.
- Require patched graph parsing, graph building, and graph validation.
- Require native rebuild before local validation can pass.
- Require relevant tests before local validation can pass.
- Require human-reviewable evidence that links crash context, patch candidate,
  changed graph/source artifacts, validation results, rebuild output, and test
  output.
- Require the evidence to separate local validation success from human
  acceptance.
- Require failed gates to produce clear, inspectable not-ready states.
- Reuse existing graph mutation, validation, patch, telemetry, and test
  primitives where applicable.
- Keep test selection conservative until stronger impacted-test analysis exists.
- Keep the first contract local and developer/test oriented.

### Explicitly Out Of Scope

- Technical specification content or implementation instructions during Stage 6.
- Implementation code, source changes outside this product spec, or Ralph cycles
  during Stage 6.
- Generating repair patches from an LLM.
- Designing the full repair-agent prompt or input contract already covered by
  #588.
- Applying a repair automatically after validation.
- Treating a repair as accepted without explicit human review.
- Merging, deploying, releasing, hot-swapping, or rolling back repaired
  binaries.
- Autonomous retry loops, multi-agent repair orchestration, or production
  self-healing policy.
- Studio dashboards, graph overlays, alerting, or operations monitoring.
- Remote telemetry ingestion, external collectors, account-based artifact
  upload, retention, privacy consent, release delivery, or customer-production
  crash handling.
- Creating or changing GitHub Project fields, labels, or approval semantics as
  part of the product behavior.
- Changing Query mode from read-only behavior.

## Constraints And Assumptions

Facts:

- Issue #587 is open and accepted for specification.
- Issue #587 is labeled `accepted` and `needs-spec`.
- The Stage 5 decision comment on 2026-06-02 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- Stage 4 routed #587 to `Needs Human Acceptance` as the Phase 13 repair
  validation guardrail after telemetry, repair context, and validation evidence
  became the next self-healing concern.
- #587 is a child issue of #580.
- #580 was approved, technically specified, implemented, and closed with local
  Phase 13 telemetry/back-mapping foundation evidence.
- #588 defines the related repair-agent input contract from mapped crash
  evidence and has a review-ready product spec PR at the time this spec was
  drafted.
- The active PRD says any repair must pass graph validation, rebuild, tests, and
  human review before it is accepted.
- The active Runtime Failure Feedback Loop note says repair validation evidence
  may report local gate results, but must not silently accept or apply a repair.
- `src/telemetry/mod.rs` contains `RepairCrashContext`,
  `RepairValidationGate`, `RepairValidationGateEvidence`,
  `RepairValidationEvidence`, `required_repair_validation_gates()`,
  `parse_repair_graph_patch()`, and
  `repair_validation_evidence_from_graph_patch()`.
- `src/telemetry/mod.rs` currently identifies these required gates:
  GraphPatch parse, atomic patch application, graph parse, graph validation,
  native rebuild, and relevant tests.
- In current source, the graph validation gate description includes both graph
  building and graph validation. This product spec separates graph build failure
  and graph validation failure as product-visible evidence states so Stage 8 can
  decide whether to expose graph build as a distinct enum gate or as an explicit
  sub-result of the existing graph validation gate.
- `RepairValidationEvidence` keeps `requires_human_review = true` and
  `accepted_for_application = false`.
- `src/patch.rs` defines GraphPatch operations and an all-or-nothing
  applicator over JSON-LD values.
- `src/mcp/tools/graph.rs` validates patched graph state before writing in MCP
  graph mutation behavior.
- `src/intent/execute.rs` has verifier-failure repair precedent, but that path
  is not the same as runtime crash evidence validation and may write repaired
  graph files after provider mutation.

Assumptions:

- The safest v1 repair validation behavior is local and deterministic, because
  production repair raises privacy, consent, deployment, rollback, and
  operational safety questions.
- The proposed patch should be represented through GraphPatch for v1 because it
  matches existing atomic graph mutation behavior and keeps the candidate
  reviewable.
- Relevant tests should include the controlled crash regression, targeted tests
  for the changed behavior, and a default untraced behavior guard when
  applicable.
- Full `cargo test --all` may be required for broad or shared changes, but the
  exact test set belongs to Stage 8 and Stage 10 because it depends on affected
  areas.
- A human reviewer needs concise command/result summaries and links or paths to
  artifacts, not raw unbounded logs.
- The evidence report should be serializable or otherwise stable enough for CI,
  GitHub review, and later workflow audit.

Constraints:

- Validation must not start without mapped crash context or an explicit
  proposed patch.
- Raw logs alone must not be enough to validate a repair candidate.
- Patch-shaped provider output must not be trusted until it parses through the
  accepted patch contract.
- Failed parse, patch, graph, rebuild, or test gates must block local validation
  success.
- Local validation success must not imply human acceptance.
- Evidence must preserve the original crash signal and failed gate output
  instead of replacing them with a generic success/failure summary.
- Validation must not require provider credentials, network access, Studio,
  Slack, GitHub, or external telemetry collectors.
- This spec PR must not mark #587 complete; it is a Stage 6 review artifact
  only.

## Decisions

- **Decision:** Use a file-based product spec for #587.
  **Evidence:** The work is architectural, cross-module, and durable. It spans
  runtime crash context, GraphPatch behavior, graph validation, rebuild/test
  evidence, telemetry evidence structures, agent boundaries, and human-review
  workflow.

- **Decision:** Require mapped repair crash context before validating a repair
  candidate.
  **Evidence:** Phase 13 is runtime-evidence-backed. #588 and #580 both frame
  repair work around mapped crash evidence, not raw logs or generic prompts.

- **Decision:** Use GraphPatch parse as the first repair-candidate gate.
  **Evidence:** `src/patch.rs` defines the canonical patch contract and
  `src/telemetry/mod.rs` already exposes repair patch parsing through that
  contract.

- **Decision:** Treat atomic patch behavior as product-visible, not only an
  implementation detail.
  **Evidence:** A failed repair patch must not leave a partially mutated graph.
  The existing GraphPatch applicator is all-or-nothing, and #587 should preserve
  that user-visible safety property.

- **Decision:** Require graph parse, graph build, graph validation, native
  rebuild, and relevant tests before local validation can pass.
  **Evidence:** #587 acceptance criteria require graph validation, rebuild, and
  relevant tests. The active PRD says any repair must pass graph validation,
  rebuild, tests, and human review before acceptance.

- **Decision:** Separate local validation success from repair acceptance.
  **Evidence:** `RepairValidationEvidence` already keeps human review required
  and acceptance false. Phase 13 explicitly excludes automatic repair acceptance
  from the first local developer/test slice.

- **Decision:** The output is an evidence report, not a silent source write.
  **Evidence:** #587 asks for human-reviewable evidence linking the crash,
  changed artifacts, validation, and test output. Human review cannot happen if
  the result is only an agent claim or an already-applied mutation.

- **Decision:** The v1 contract remains local and developer/test oriented.
  **Evidence:** The PRD and Runtime Failure Feedback Loop note both exclude
  production crash ingestion, hot-swap, silent updates, and autonomous repair
  acceptance from the first Phase 13 slice.

## Behavior

### Defaults

- Repair validation is not run for default untraced runtime failures.
- Repair validation is not run from raw stderr, raw logs, or crash text alone.
- Repair validation is local and deterministic for the controlled v1 path.
- Repair validation does not invoke a provider.
- Repair validation does not silently write graph source files.
- Repair validation does not accept, merge, deploy, hot-swap, or close a repair.
- A locally valid repair candidate remains pending human review.

### Inputs

- A mapped `RepairCrashContext` or equivalent context assembled from local
  trace/crash artifacts and graph back-mapping evidence.
- A proposed GraphPatch-shaped repair candidate.
- The original JSON-LD graph source or workspace graph artifact being repaired.
- The workspace or build target needed to rebuild after the candidate patch.
- A relevant test plan or conservative default test selection.
- Optional paths or identifiers for crash artifacts, trace maps, candidate
  patch artifacts, build output, and test output.

### Outputs

- A repair validation evidence report.
- The original crash context or a link/path to it.
- The proposed patch serialized for review or a patch/diff summary.
- The source graph or workspace artifact considered.
- One result per required validation gate.
- Rebuild result summary.
- Relevant test result summary.
- A local validation status.
- A human-review requirement.
- A clear acceptance state that remains not accepted until human review happens
  outside the local validation gate.

### Visible States

- Missing repair context: validation cannot start because mapped crash evidence
  is absent.
- Missing patch candidate: validation cannot start because no proposed patch was
  provided.
- Patch malformed: the proposed patch does not parse as GraphPatch.
- Patch application failed: the patch cannot be applied atomically to the
  source graph.
- Graph parse failed: patched JSON-LD cannot be parsed into DUUMBI AST.
- Graph IR build failed: parsed DUUMBI AST cannot be converted to graph IR.
- Graph validation failed: semantic validation produced diagnostics.
- Rebuild failed: native rebuild does not complete successfully.
- Tests failed: relevant tests do not pass.
- Local validation passed: every required local gate passed.
- Pending human review: local validation passed, but human acceptance is still
  required.
- Human rejected or revision needed: a reviewer does not accept the candidate,
  and the repair remains unaccepted.

### Gate Semantics

- GraphPatch parse gate:
  - passes only when the candidate deserializes through the canonical GraphPatch
    contract.
  - fails before any patch application when the payload is malformed.

- Atomic patch application gate:
  - passes only when the candidate patch can be applied to a clone of the
    source and the original source is structurally preserved on any failure.
  - fails when any operation targets a missing node, block, function, or invalid
    JSON-LD structure.

- Graph parse gate:
  - passes only when the patched JSON-LD parses into the DUUMBI AST.
  - fails with diagnostics that identify the parse problem clearly enough for
    review.

- Graph build gate:
  - passes only when the parsed DUUMBI AST builds into graph IR.
  - fails with diagnostics that identify the graph construction problem clearly
    enough for review.

- Graph validation gate:
  - passes only when the patched graph has no blocking validation diagnostics.
  - fails with the diagnostic messages needed to revise the candidate.

- Native rebuild gate:
  - passes only when the patched graph can be rebuilt through the selected
    local build path.
  - fails with command and output summary evidence.

- Relevant tests gate:
  - passes only when the selected targeted and regression tests pass.
  - must include enough evidence to explain why the selected tests are relevant.
  - must stay conservative when impacted-test analysis is unavailable.

- Human review gate:
  - is not satisfied by local validation.
  - requires a human reviewer or later accepted workflow to approve the
    candidate before it can be treated as accepted.

### Evidence Report Requirements

- The report identifies the schema or format version when practical.
- The report identifies the crash context used for validation.
- The report identifies the patch candidate used for validation.
- The report identifies the source graph/workspace artifact validated.
- The report records each gate as passed or failed.
- The report includes concise command/result summaries for rebuild and tests.
- The report preserves failed gate output in a reviewable form.
- The report says whether all local validation gates passed.
- The report says human review is required.
- The report does not mark the candidate accepted for application by default.
- The report is bounded enough for GitHub review or CI artifacts and does not
  include unbounded raw logs by default.

### Error And Empty States

- If mapped crash context is missing, report that repair validation requires
  mapped crash evidence and do not validate a patch.
- If the candidate patch is missing, report that no repair candidate was
  provided and do not produce local validation success.
- If the patch is malformed, report the GraphPatch parse error and do not apply
  the patch.
- If patch application fails, report the failed operation and keep original
  source unchanged.
- If graph validation fails, report diagnostics and do not rebuild as a local
  success path.
- If rebuild fails, report the rebuild command summary and failure output.
- If tests fail, report the failed tests and preserve local validation as
  failed.
- If all local gates pass but no human review exists, report pending human
  review rather than success.

### Invariants

- Original crash evidence remains traceable from every validation report.
- Patch-shaped output is not enough to claim repair success.
- Graph mutation cannot bypass parse, build, or validate behavior.
- Failed gates cannot be hidden behind an agent success message.
- Local validation success cannot bypass human review.
- Default untraced behavior remains unchanged unless later approved scope says
  otherwise.
- Production self-healing claims are not made from this local guardrail.

## BDD Scenarios

Feature: Repair patch validation and human-reviewable evidence

Rule: Repair validation starts only from mapped evidence and a candidate patch

Scenario: Controlled repair candidate enters validation
Given mapped repair crash context exists for a traced runtime failure
And a proposed repair candidate is provided as GraphPatch-shaped data
When repair validation is requested
Then the candidate is evaluated through the required local validation gates
And the original crash context remains linked to the validation result
And no repair is accepted before the gates and human review are complete

Scenario: Raw crash logs are not enough to validate a repair
Given a runtime failure message exists in stderr or a log file
But no mapped repair crash context exists
When repair validation is requested with a proposed patch
Then the system reports that mapped crash evidence is required
And no local validation success is reported
And no source graph is silently changed

Scenario: Missing patch candidate blocks validation
Given mapped repair crash context exists
But no proposed patch candidate is provided
When repair validation is requested
Then the system reports that a patch candidate is required
And no validation gates are reported as passed

Rule: Patch validation is canonical and atomic

Scenario: Malformed patch fails before application
Given mapped repair crash context exists
And the proposed patch cannot parse as GraphPatch
When repair validation is requested
Then the GraphPatch parse gate fails
And the patch is not applied
And the evidence report records the parse failure

Scenario: Patch application failure preserves the original graph
Given mapped repair crash context exists
And a proposed GraphPatch references a missing graph node
When repair validation applies the patch candidate
Then the atomic patch application gate fails
And the original graph source remains unchanged
And the evidence report identifies the failed patch operation

Scenario: Patch-shaped output is not automatically accepted
Given a Repair agent has produced GraphPatch-shaped output
But graph validation, rebuild, tests, and human review have not all completed
When the candidate is inspected
Then the candidate is not accepted for application
And the evidence report records the gates that have not passed

Rule: Graph, rebuild, and test gates must pass

Scenario: Graph parse failure blocks local success
Given a proposed patch parses and applies atomically
But the patched JSON-LD cannot be parsed into the DUUMBI AST
When repair validation runs
Then the graph parse gate fails
And local validation is not marked as passed
And the evidence report includes the parse diagnostics

Scenario: Graph build failure blocks local success
Given a proposed patch parses and applies atomically
And the patched JSON-LD parses into the DUUMBI AST
But the parsed DUUMBI AST cannot be converted to graph IR
When repair validation runs
Then the graph build gate fails
And local validation is not marked as passed
And the evidence report includes the graph construction diagnostics

Scenario: Graph validation failure blocks local success
Given a proposed patch parses and applies atomically
But the patched graph produces validation diagnostics
When repair validation runs
Then the graph validation gate fails
And local validation is not marked as passed
And the evidence report includes the validation diagnostics

Scenario: Native rebuild failure blocks local success
Given a proposed patch passes graph validation
But the native rebuild fails
When repair validation runs
Then the native rebuild gate fails
And local validation is not marked as passed
And the evidence report includes rebuild command and output summary

Scenario: Relevant test failure blocks local success
Given a proposed patch passes graph validation
And the native rebuild succeeds
But a relevant targeted or regression test fails
When repair validation runs
Then the relevant tests gate fails
And local validation is not marked as passed
And the evidence report identifies the failed tests

Scenario: All local gates pass but human review is still required
Given mapped repair crash context exists
And a proposed patch parses as GraphPatch
And patch application is atomic
And the patched graph parses, builds, and validates
And the native rebuild succeeds
And relevant tests pass
When repair validation produces evidence
Then local validation is marked as passed
And the evidence report requires human review
And the repair is not marked accepted for application

Rule: Evidence is traceable and bounded

Scenario: Evidence report links crash, patch, validation, rebuild, and tests
Given a repair candidate has been evaluated
When the evidence report is produced
Then the report links the mapped crash context
And the report links or summarizes the proposed patch
And the report records each validation gate result
And the report summarizes rebuild output
And the report summarizes relevant test output
And the report states the human-review requirement

Scenario: Failed validation remains reviewable
Given a repair candidate fails one required gate
When the evidence report is produced
Then the failed gate output remains visible for review
And earlier passed gate evidence remains visible
And the original crash evidence remains linked
And the result is not reported as locally validated

Rule: Product boundaries remain explicit

Scenario: Production self-healing is requested from the validation guardrail
Given the repair validation guardrail is available for local developer/test evidence
When a user asks it to ingest production crashes or automatically deploy a repair
Then the request is out of scope for #587
And it must be routed to later product specification
And no automatic acceptance is reported

Scenario: Query mode asks about repair validation evidence
Given repair validation evidence exists
When Query mode is used to inspect the evidence
Then Query mode can describe the evidence read-only
And Query mode does not mutate graph source
And Query mode does not accept the repair

## Tasks

- Product and workflow setup:
  - Keep #587 as the repair validation and human-reviewable evidence child
    issue under #580.
  - Use non-closing workflow references such as `Related to #587`.
  - Keep the execution issue open for later workflow stages.

- Validation contract:
  - Define the required starting inputs: mapped repair crash context and a
    proposed GraphPatch candidate.
  - Define the required local gates: GraphPatch parse, atomic patch application,
    graph parse, graph build, graph validation, native rebuild, and relevant
    tests.
  - Define not-ready behavior for missing context, missing patch, malformed
    patch, failed patch application, graph diagnostics, rebuild failure, and
    test failure.

- Evidence report:
  - Define the minimum evidence fields for crash context, patch candidate,
    source artifact, gate results, rebuild summary, test summary, local
    validation status, and human-review requirement.
  - Require failed gate output to remain inspectable.
  - Require local validation and human acceptance to remain separate states.

- Human-review boundary:
  - Define that all local gates passing means pending human review, not repair
    acceptance.
  - Keep automatic apply, merge, deploy, hot-swap, and production self-healing
    outside this issue.

- Test and review planning:
  - Require later implementation tests for each pass/fail gate.
  - Require evidence that failed patch application preserves the original graph.
  - Require evidence that default untraced behavior remains unchanged.
  - Require the controlled crash path and relevant regression to be represented
    in later test evidence.

## Checks

- Stage 7 product review confirms:
  - Stage 5 acceptance is correctly represented.
  - The product spec stays within #587 and does not create a technical spec.
  - Repair validation starts from mapped crash context and a proposed patch.
  - GraphPatch parse and atomic patch behavior are product-visible gates.
  - Graph parse, graph build, graph validation, native rebuild, and relevant
    tests are required before local validation success.
  - Local validation success remains distinct from human acceptance.
  - Evidence report requirements are reviewable and bounded.
  - Production self-healing, hot-swap, deployment, and autonomous acceptance are
    out of scope.
  - No auto-closing issue references are used in the spec-only PR.

- Stage 8 technical spec should later define:
  - The exact evidence report schema.
  - How the original source graph is preserved while validating a candidate.
  - How candidate patch artifacts are represented and diffed.
  - How graph parse, graph build, and graph validation gates map to existing
    parser, builder, and validator APIs.
  - How rebuild commands are selected for single-file and workspace cases.
  - How relevant tests are selected conservatively.
  - How large command output is summarized and linked.
  - How human-review state is represented without accepting the repair locally.

- Implementation PRs should later prove:
  - Missing mapped crash context blocks validation.
  - Missing patch candidate blocks validation.
  - Malformed GraphPatch data fails before application.
  - Failed patch application preserves the original graph.
  - Graph parse, graph build, and graph validation failures block local success.
  - Native rebuild failure blocks local success.
  - Relevant test failure blocks local success.
  - All local gates passing still keeps human review required and acceptance
    false.
  - The evidence report links crash context, candidate patch, validation gates,
    rebuild output, and test output.
  - Default untraced behavior remains unchanged.

- Suggested test coverage for later implementation:
  - Unit tests for evidence serialization and required fields.
  - Unit tests for GraphPatch parse success and failure.
  - Unit tests for atomic patch failure preserving source.
  - Unit tests or integration tests for graph validation failure evidence.
  - Integration coverage using the controlled runtime failure fixture.
  - Rebuild and targeted-test evidence for a candidate patch path.
  - Regression coverage that a locally validated candidate remains pending human
    review.

- Manual review evidence for later implementation:
  - The controlled failing DUUMBI graph program.
  - The mapped crash context used.
  - The proposed patch candidate.
  - The graph/source diff or patch summary.
  - The validation diagnostics or pass result.
  - The rebuild command and result.
  - The relevant test command(s) and result(s).
  - The final evidence report showing local validation state and human-review
    requirement.

## Open Questions

No blocking open questions for Stage 7 product review.

Non-blocking questions for Stage 8 or later implementation:

- Should the evidence report be a JSON artifact, a Markdown report, or both?
- Where should local validation evidence be written by default so it is easy to
  review but not accidentally committed?
- What exact command set is required for native rebuild in single-file versus
  workspace repair cases?
- How should relevant tests be selected before DUUMBI has stronger impacted-test
  analysis?
- Should the validation guardrail support source-code patch evidence in addition
  to GraphPatch evidence after later work expands repair scope?
- What later human-review workflow should turn pending-review evidence into an
  accepted repair state, if any?

## Sources

- Issue #587: https://github.com/hgahub/duumbi/issues/587
- Stage 4 triage refill for #587:
  https://github.com/hgahub/duumbi/issues/587#issuecomment-4601062097
- Stage 5 human acceptance decision for #587:
  https://github.com/hgahub/duumbi/issues/587#issuecomment-4606336397
- Parent issue #580:
  https://github.com/hgahub/duumbi/issues/580
- Phase 13 decomposition comment:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4505735849
- Stage 12 closure evidence for #580:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4525647674
- Parent product spec: `specs/DUUMBI-580/PRODUCT.md`
- Parent technical spec: `specs/DUUMBI-580/TECHNICAL.md`
- Related child #588, repair-agent input contract:
  https://github.com/hgahub/duumbi/issues/588
- #588 product spec PR:
  https://github.com/hgahub/duumbi/pull/651
- Related child #585, local crash dumps and trace-to-graph mapping artifacts:
  https://github.com/hgahub/duumbi/issues/585
- DUUMBI PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Runtime Failure Feedback Loop:
  `Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Runtime Failure Feedback Loop.md`
- DUUMBI Service and Research Direction:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- DUUMBI Product Roadmap 2026-05:
  `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Product Roadmap 2026-05.md`
- DUUMBI Phase 13 Self-Healing and Telemetry archive:
  `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 13 - Self-Healing & Telemetry.md`
- Architecture reference: `docs/architecture.md`
- Telemetry and repair validation source: `src/telemetry/mod.rs`
- GraphPatch source: `src/patch.rs`
- MCP graph tools source: `src/mcp/tools/graph.rs`
- Intent verifier-repair precedent: `src/intent/execute.rs`
