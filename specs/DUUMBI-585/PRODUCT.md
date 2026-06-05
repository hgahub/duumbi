# DUUMBI-585: Persist Crash Dumps And Trace-To-Graph Mapping Artifacts

## Summary

Define the product contract and reconciliation path for DUUMBI's local telemetry
artifacts that make traced runtime failures explainable from durable evidence.

This issue sits inside the approved #580 Phase 13 runtime failure feedback
slice. It owns the observable artifact contract across:

- `trace_map.json`: deterministic trace ID to graph function/block `@id`
  mapping.
- `traces.jsonl`: line-oriented traced runtime events.
- `crash_dump.jsonl`: line-oriented crash records that join to the trace map.

The product goal is not generic observability. The v1 contract is:

```text
explicit traced build/run -> local trace + crash + map artifacts ->
function/block graph evidence -> inspectable failure explanation
```

At Stage 6 inspection, current `main` already appears to contain substantial
telemetry artifact behavior through #580, #583, #584, #586, #588, and #587. This
spec therefore treats #585 as an artifact-contract and reconciliation issue:
implementation must first audit current source and evidence, then either record
that the accepted contract is already satisfied or identify the exact remaining
gap before making any narrow hardening change.

This specification PR is related to #585 and must leave the execution issue
open for Stage 7 review, Stage 8 technical specification, implementation audit
or hardening, review, and Stage 12 evidence.

## Problem

DUUMBI's Phase 13 repair story is only trustworthy if a runtime failure can be
explained from local evidence. A panic on stderr is not enough for a developer,
maintainer, or later repair agent to answer:

- what failed.
- which graph function and block were active at failure time.
- which artifact proves that runtime trace IDs map back to graph `@id` values.
- whether the evidence came from an explicitly traced run.
- whether missing, malformed, or unmapped evidence was rejected instead of
  overinterpreted.

The related Phase 13 work has advanced quickly. Parent #580 and children #583,
#584, #586, #588, and #587 now provide source and test evidence for opt-in
traced builds, trace events, crash artifacts, inspection, repair context, and
repair validation. The remaining product risk for #585 is drift and ownership:
if no durable product contract reconciles these artifacts, later agents may
duplicate already-merged behavior, fork schema ownership away from source
types, assume default builds produce telemetry, or treat partial artifacts as
repair-ready evidence.

The product need is a reviewable contract that defines artifact names, local
path behavior, join semantics, default-off behavior, privacy boundaries, and
the evidence required before #585 can be considered satisfied.

## Outcome

When this work is done:

- A reviewer can determine whether current `main` already satisfies #585, needs
  only focused hardening, or has a specific remaining product gap.
- `trace_map.json` is the durable map from stable runtime trace IDs to graph
  function/block `@id` values.
- `traces.jsonl` is the local JSON Lines event stream for traced runtime
  function/block activity and panic correlation.
- `crash_dump.jsonl` is the local JSON Lines crash artifact that records panic
  message, active function trace ID, active block trace ID, and traced-run
  evidence.
- Crash inspection joins `crash_dump.jsonl` to `trace_map.json` before
  reporting graph function/block context.
- Default untraced builds and runs do not produce trace, crash, or trace-map
  evidence and are not accepted as back-mapping proof.
- Missing, malformed, untraced, or unmapped evidence fails closed.
- Local artifact paths are deterministic and remain local developer/test
  evidence, not production telemetry ingestion.
- Runtime argument and value snapshots remain absent in v1.
- The final evidence for #585 links the current source, tests, and merged
  issues/PRs that satisfy the contract, or names the exact remaining gap.
- The execution issue remains open until the DUUMBI workflow reaches its normal
  final evidence stage.

## Scope

### In Scope

- Define the product artifact contract for `trace_map.json`, `traces.jsonl`,
  and `crash_dump.jsonl`.
- Require deterministic trace-map entries for graph functions and blocks.
- Require trace/crash records to be line-oriented JSON artifacts suitable for
  local inspection and tests.
- Require crash evidence to include function and block trace IDs that join to
  trace-map entries.
- Require traced-run evidence such as `trace_active` before crash artifacts can
  be treated as mapped runtime failure evidence.
- Require `duumbi telemetry inspect` or the accepted equivalent to report
  mapped graph function/block context from local artifacts.
- Require default untraced builds and runs to avoid telemetry artifact
  emission.
- Require local artifact path behavior through `.duumbi/telemetry/`,
  configured artifact directories, or `DUUMBI_TELEMETRY_DIR` overrides.
- Require missing, malformed, untraced, and unmapped evidence to fail closed.
- Require structural parsing and join checks in tests or completion evidence.
- Require source reconciliation before any new implementation work starts.
- Allow focused hardening tests, docs, or code only when the audit finds a
  specific unmet criterion.
- Preserve original runtime panic/stderr behavior when telemetry is active.
- Preserve the v1 privacy boundary that runtime argument/value snapshots are
  absent.

### Explicitly Out Of Scope

- Technical specification content or implementation instructions during Stage 6.
- Implementation code, source changes outside this product spec, or Ralph
  cycles during Stage 6.
- Creating a parallel schema outside the source telemetry types and constants.
- New traced-build selection UX beyond the #583 contract.
- New function/block trace-event semantics beyond the #584 contract.
- Controlled failure fixture behavior beyond the #586 proof.
- Repair-agent input behavior beyond the #588 contract.
- Repair patch validation behavior beyond the #587 contract.
- Remote telemetry export, OpenTelemetry, OTLP, collectors, dashboards, alerts,
  production observability, or account-based crash ingestion.
- Studio telemetry UI, graph overlays, or operations monitoring.
- Hot-swap, silent updates, autonomous repair acceptance, or production
  self-healing.
- Exact node-level crash evidence as a v1 requirement.
- Runtime value, argument, heap, stack, or snapshot capture.
- Run IDs, rotation, cleanup, compression, retention, upload, or privacy policy
  for telemetry artifacts.
- Committing generated telemetry artifacts, crash dumps, binaries, or trace
  output.

## Constraints And Assumptions

Facts:

- Issue #585 is open and has explicit Stage 5 human acceptance on 2026-06-05.
- The Stage 5 decision records `Decision: Accept`, `Next state: Spec Needed`,
  and no remaining open questions.
- Issue #585 is labeled `accepted` and `needs-spec` at Stage 6 intake.
- Stage 4 routed #585 to `Needs Human Acceptance` as a Phase 13 telemetry
  artifact contract and reconciliation issue.
- #585 is a child of #580.
- The approved #580 product and technical specs define the first Phase 13
  runtime feedback slice as local developer/test telemetry, graph
  function/block back-mapping, and repair-ready evidence boundaries.
- #583 defines traced build mode and local telemetry configuration.
- #584 defines function/block trace events for explicitly traced builds.
- #586 defines the controlled runtime failure proof that joins trace/crash/map
  artifacts and uses telemetry inspection.
- #588 defines repair-agent input from mapped crash evidence.
- #587 defines repair patch validation evidence and keeps human review required.
- The active PRD states that runtime failure feedback is local developer/test
  feedback first, not automatic production self-repair.
- The active Runtime Failure Feedback Loop note says the first implemented
  foundation is local and opt-in, with trace/crash artifacts and graph
  function/block back-mapping.
- The active AI review policy says Copilot is the default automated reviewer
  for file-based spec gates and Greptile is manual-only.
- `src/telemetry/mod.rs` defines `TRACE_MAP_FILE`, `CRASH_DUMP_FILE`,
  `TRACE_MAP_SCHEMA_VERSION`, `CRASH_SCHEMA_VERSION`, `TraceMap`,
  `TraceMapEntry`, `CrashArtifact`, `inspect_crash_artifacts`, telemetry path
  resolution, and repair-context/repair-validation evidence types.
- `runtime/duumbi_runtime.c` defines traced runtime hooks, writes
  `traces.jsonl`, writes `crash_dump.jsonl`, honors `DUUMBI_TELEMETRY_DIR`,
  falls back to `.duumbi/telemetry`, and preserves `duumbi panic:` stderr.
- `src/workspace.rs` writes `trace_map.json` only for traced builds.
- `tests/integration_telemetry.rs` exercises traced panic evidence, trace-map
  joins, telemetry inspection, default untraced non-emission, and repair
  context/validation evidence.
- `src/telemetry/mod.rs` tests path resolution, environment override behavior,
  conservative defaults, unsupported value capture, and unmapped evidence
  rejection.

Assumptions:

- The v1 product need is durable local failure explanation, not full
  observability.
- Current `main` may already satisfy most or all of #585 through merged related
  work, but Stage 10 must still verify that against this product contract and
  current CI before final disposition.
- Source telemetry types and constants should remain the canonical schema owner
  because tests and CLI behavior depend on them.
- Function/block-level mapping is sufficient for v1 because exact node evidence
  has not yet been specified or proven.
- Append-only JSONL is acceptable for v1 as long as inspection and tests select
  the relevant crash record deterministically.
- Local telemetry artifacts may contain sensitive failure context, so runtime
  value capture should stay absent until separately specified with privacy and
  redaction controls.

Constraints:

- Default builds must not emit trace calls, initialize trace state, write
  telemetry artifacts, or require trace-only telemetry config.
- Traced artifact behavior must remain local and testable without provider
  credentials, network access, GitHub, Slack, Studio, or external collectors.
- Artifact writing or inspection must not mutate graph source files.
- Crash artifact writing must not hide or replace the original runtime failure
  signal.
- The system must not report graph-linked crash context unless crash IDs join
  to trace-map entries of the expected kind.
- Missing, malformed, untraced, or unmapped evidence must produce explicit
  not-ready or error behavior.
- This spec PR must not mark #585 complete; it is a Stage 6 review artifact
  only.

## Decisions

- **Decision:** Use a file-based product spec for #585.
  **Evidence:** The issue is architectural, cross-module, user-visible, and
  durable. It spans compiler/workspace build behavior, runtime hooks, telemetry
  domain types, local artifacts, CLI inspection, tests, and Phase 13 workflow
  evidence.

- **Decision:** Treat #585 as artifact-contract reconciliation before new
  implementation.
  **Evidence:** Stage 6 source inspection found current telemetry code and
  tests already covering trace maps, crash dumps, trace events, inspection,
  default untraced behavior, and repair evidence. New work should be limited to
  verified gaps.

- **Decision:** Source telemetry types own the canonical v1 schemas.
  **Evidence:** `src/telemetry/mod.rs` defines the schema constants and Rust
  structures consumed by build, inspect, repair-context, repair-validation, and
  tests. A second product-only schema would create drift.

- **Decision:** Keep the three artifact names explicit.
  **Evidence:** #585 names `trace_map.json`, `traces.jsonl`, and
  `crash_dump.jsonl`, and current source/runtime/tests use those names.

- **Decision:** Require joinability, not only artifact existence.
  **Evidence:** A crash file without a matching trace map can prove that a
  panic happened but cannot prove graph back-mapping. Crash function/block IDs
  must join to trace-map entries of the matching kind.

- **Decision:** Keep default untraced behavior outside the evidence contract.
  **Evidence:** #583 and #584 require traced behavior to be explicit. The
  controlled untraced failure test confirms that untraced panic output is not
  accepted as mapped telemetry evidence.

- **Decision:** Preserve original runtime failure behavior.
  **Evidence:** Runtime telemetry calls `duumbi_trace_panic` before printing the
  original `duumbi panic:` message and exiting; tests assert the stderr signal
  remains visible.

- **Decision:** Runtime values and arguments remain absent in v1.
  **Evidence:** Active product notes flag privacy risk. Current telemetry config
  rejects `capture-values = true`, and repair-context tests assert no heap,
  stack, runtime value, or value snapshot content appears.

- **Decision:** Leave exact node evidence unavailable unless later accepted work
  proves it.
  **Evidence:** The accepted Phase 13 foundation and active runtime feedback
  note define function/block-level graph back-mapping as the first reliable
  level.

- **Decision:** This spec PR must be specification-only and must leave #585
  open.
  **Evidence:** Stage 6 drafts a review artifact. Product approval, technical
  specification, implementation audit or hardening, review, and final evidence
  remain later workflow stages.

## Behavior

### Defaults

- A normal untraced build does not write `trace_map.json`.
- A normal untraced run does not write `traces.jsonl`.
- A normal untraced runtime failure does not write `crash_dump.jsonl`.
- A normal untraced runtime failure still reports the original panic/stderr
  behavior.
- `duumbi telemetry inspect` must not report function/block graph context when
  required traced evidence is absent.
- Trace-only telemetry config errors must not block normal untraced builds
  unless another accepted issue changes that contract.

### Traced Artifact Creation

- An explicitly traced build produces or prepares a local telemetry artifact
  directory.
- A traced build writes `trace_map.json` with the trace-map schema version,
  optional program/build hash, and deterministic function/block entries.
- Each trace-map entry contains:
  - stable runtime trace ID.
  - graph element kind: function or block.
  - graph `@id`.
  - module name.
  - function name.
  - optional block label for block entries.
- Trace IDs are stable for graph identity and must reject collisions.
- A traced runtime emits JSON Lines trace events to `traces.jsonl` for the
  accepted function/block event surface and panic correlation.
- A traced runtime failure appends a JSON Lines crash record to
  `crash_dump.jsonl`.
- A crash record contains the crash schema version, event kind, panic message,
  active function trace ID, active block trace ID, and traced-run evidence.
- JSONL files stay append-only in v1.

### Artifact Paths

- The default artifact directory is `.duumbi/telemetry`.
- A configured telemetry artifact directory may override the default for traced
  builds when valid.
- `DUUMBI_TELEMETRY_DIR` may override the artifact directory for local tests or
  one-off runs.
- Relative artifact paths resolve relative to the workspace root.
- A configured `artifact-dir` must not silently split `trace_map.json` from
  `traces.jsonl` or `crash_dump.jsonl`. If the traced runtime cannot honor the
  configured directory without an environment override, the implementation must
  expose that limitation as a Stage 8/Stage 10 gap before claiming #585 evidence
  is complete.
- Invalid path traversal, unsupported sampling, invalid sample rates, and
  unsupported value capture fail before traced build behavior proceeds.
- Generated telemetry artifacts must not be committed to source control.

### Inspection And Join Semantics

- Inspection reads `crash_dump.jsonl` and `trace_map.json` from the selected
  telemetry directory unless explicit paths are supplied.
- Inspection selects the latest non-empty crash record by default or an
  explicit JSONL line when supported by the accepted surface.
- Inspection rejects crash records not marked as traced evidence.
- Inspection maps the crash function trace ID only to a function trace-map
  entry.
- Inspection maps the crash block trace ID only to a block trace-map entry.
- Inspection reports the crash message, mapped function graph ID, mapped block
  graph ID, and explicit v1 exact-node unavailability.
- Inspection must not infer graph context from raw stderr, generic logs, or
  incomplete artifacts.

### Error And Empty States

- Missing crash evidence fails with an explicit missing-evidence or read error.
- Missing trace map evidence fails with an explicit missing-evidence or read
  error.
- Malformed JSON in either artifact fails with parse context that identifies
  the artifact.
- A crash record with `trace_active = false` is not accepted as graph-linked
  evidence.
- Function trace IDs absent from the trace map fail as unmapped evidence.
- Block trace IDs absent from the trace map fail as unmapped evidence.
- Artifact write failures may warn, but must not hide the original runtime
  panic path.

### Reconciliation Path

- Stage 10 must audit current `main` against this product spec before adding
  code.
- If current source and tests satisfy all accepted behavior, the implementation
  artifact should be evidence-focused rather than duplicating telemetry code.
- If a gap remains, implementation should make the smallest hardening change
  that satisfies the specific unmet criterion.
- The final evidence must identify:
  - merged related issues/PRs used as evidence.
  - source files that own the contract.
  - tests or checks that prove traced artifact creation.
  - tests or checks that prove default untraced non-emission.
  - tests or checks that prove crash-to-trace-map join behavior.
  - tests or checks that prove fail-closed behavior.
  - confirmation that runtime value/argument capture remains absent.

## BDD Scenarios

Feature: Durable local telemetry artifact contract

Scenario: Traced runtime failure produces the three local evidence artifacts
Given a DUUMBI graph fixture that fails deterministically at runtime
And the fixture is built with traced telemetry enabled
When the traced binary runs with a local telemetry artifact directory
Then `trace_map.json` exists in the selected telemetry directory
And `traces.jsonl` exists in the selected telemetry directory
And `crash_dump.jsonl` exists in the selected telemetry directory
And the original runtime panic message is still visible on stderr

Scenario: Trace map entries preserve graph function and block identity
Given a traced DUUMBI build has completed
When a reviewer parses `trace_map.json`
Then the artifact uses the accepted trace-map schema version
And each function entry maps a stable trace ID to a graph function `@id`
And each block entry maps a stable trace ID to a graph block `@id`
And entries are deterministic enough for repeatable tests and review
And trace ID collisions fail instead of producing ambiguous mappings

Scenario: Crash evidence joins to graph context
Given a traced runtime failure has written `crash_dump.jsonl`
And the same telemetry directory contains `trace_map.json`
When `duumbi telemetry inspect` reads the crash and map artifacts
Then the crash function trace ID joins to a function trace-map entry
And the crash block trace ID joins to a block trace-map entry
And the inspection output reports the mapped graph function `@id`
And the inspection output reports the mapped graph block `@id`
And the output says exact node evidence is unavailable in v1

Scenario: Default untraced failure is not accepted as back-mapping evidence
Given the same failing fixture is built without traced telemetry
When the untraced binary fails at runtime
Then the original runtime panic message is still visible on stderr
But `trace_map.json` is not produced as traced evidence
And `traces.jsonl` is not produced as traced evidence
And `crash_dump.jsonl` is not produced as traced evidence
And telemetry inspection does not report graph function or block context

Scenario: Missing evidence fails closed
Given a telemetry directory does not contain `crash_dump.jsonl`
When a reviewer runs telemetry inspection for that directory
Then inspection fails with an explicit missing or read error
And inspection does not report mapped graph context

Scenario: Malformed evidence fails closed
Given a telemetry directory contains malformed crash or trace-map JSON
When a reviewer runs telemetry inspection for that directory
Then inspection fails with parse context for the malformed artifact
And inspection does not report mapped graph context

Scenario: Unmapped crash evidence fails closed
Given `crash_dump.jsonl` contains a traced crash record
And `trace_map.json` does not contain the crash function or block trace ID
When a reviewer runs telemetry inspection
Then inspection fails with unmapped evidence
And the system does not claim a graph-linked runtime failure

Scenario: Local artifact path override is honored for traced runs
Given a developer sets `DUUMBI_TELEMETRY_DIR` to a local test directory
And builds and runs a traced DUUMBI binary
When telemetry artifacts are written
Then the trace map, trace events, and crash dump are written under the override
And the default source-controlled graph files are not mutated

Scenario: Configured artifact directory keeps evidence together
Given a workspace config sets a valid telemetry `artifact-dir`
And no `DUUMBI_TELEMETRY_DIR` override is set
When a traced build and runtime failure are executed from that workspace
Then `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` are all found in
the same accepted telemetry directory
And telemetry inspection can join crash evidence from that directory

Scenario: Config-only runtime artifact routing gap is reported
Given a traced build can write `trace_map.json` to a configured `artifact-dir`
But the traced runtime cannot write `traces.jsonl` or `crash_dump.jsonl` to that
configured directory without `DUUMBI_TELEMETRY_DIR`
When Stage 10 audits the #585 artifact contract
Then the implementation evidence reports a specific config-only artifact routing
gap
And the artifact contract is not claimed complete from split default/configured
locations

Scenario: Value capture remains absent in v1
Given a developer configures telemetry value capture for a traced build
When telemetry config is validated
Then the configuration is rejected as unsupported
And trace/crash/repair evidence does not contain runtime argument snapshots
And trace/crash/repair evidence does not contain heap or stack snapshots

Scenario: Source reconciliation avoids duplicate telemetry work
Given current source already satisfies the accepted artifact contract
When Stage 10 audits #585 before implementation
Then the implementation artifact records source and test evidence
And no duplicate telemetry schema or redundant runtime artifact writer is added
And any remaining work is limited to the exact uncovered criterion

## Tasks

- Verify the Stage 7-approved product spec against current `main` before Stage
  8 technical planning.
- During Stage 8, decide whether #585 needs:
  - evidence-only technical guidance for source reconciliation.
  - focused hardening tests.
  - narrow documentation of the artifact contract.
  - a small code patch for a specific gap.
- Audit current telemetry source and tests against each BDD scenario.
- Verify related merged evidence for #580, #583, #584, #586, #588, and #587.
- Preserve source schema ownership in `src/telemetry/mod.rs`.
- Preserve runtime artifact writing ownership in `runtime/duumbi_runtime.c`.
- Preserve traced-build trace-map writing ownership in `src/workspace.rs`.
- Preserve user-facing inspection behavior in the telemetry CLI.
- Add or harden only the tests needed for any uncovered behavior.
- Prepare final evidence that links accepted behavior to source, tests, and
  related merged PRs.

Independent work:

- Source/test audit can run independently from PR/issue evidence collection.
- Documentation or evidence-matrix updates can run after the audit identifies
  whether any contract gap exists.
- Hardening tests can run independently when they do not change public product
  behavior.

Sequential work:

- Do not make code changes until the current-source audit identifies a specific
  gap.
- Do not start repair-agent or patch-validation work from #585; those belong to
  #588 and #587.
- Do not request final workflow approval for #585 until source evidence,
  checks, and review feedback are clean.

## Checks

- Product spec review:
  - Stage 7 review confirms this spec stays within #585 and #580.
  - Review confirms the spec is artifact-contract and reconciliation work, not
    new implementation scope.
  - Review confirms no technical spec or implementation code was created during
    Stage 6.

- Automated review:
  - Codex self-review finds no blocking product, scope, or BDD gaps.
  - Copilot submits actual non-dismissed review evidence for the file-based
    spec PR.
  - Greptile is not used unless a human explicitly requests manual deep review.
  - All review threads are resolved after verified fixes.

- Local checks for this Stage 6 PR:
  - Markdown/spec-only inspection confirms only
    `specs/DUUMBI-585/PRODUCT.md` changed.
  - `cargo fmt --check` is not required for a Markdown-only change unless the
    repository workflow runs it anyway.
  - `cargo test --all` is optional for Stage 6 because this PR changes only a
    product spec; if run, it should be reported as broad confidence rather than
    spec syntax proof.

- Required later implementation or evidence checks:
  - Test or evidence proves traced runs produce `trace_map.json`,
    `traces.jsonl`, and `crash_dump.jsonl`.
  - Test or evidence proves `trace_map.json` contains deterministic
    function/block graph mappings and collision handling.
  - Test or evidence proves crash function/block IDs join to trace-map entries.
  - Test or evidence proves `duumbi telemetry inspect` reports function/block
    context only after a valid join.
  - Test or evidence proves default untraced builds/runs do not emit telemetry
    artifacts.
  - Test or evidence proves original panic/stderr behavior is preserved.
  - Test or evidence proves missing, malformed, untraced, and unmapped evidence
    fails closed.
  - Test or evidence proves local artifact path defaults and
    `DUUMBI_TELEMETRY_DIR` override behavior.
  - Test or evidence proves configured `artifact-dir` behavior keeps
    `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` in the same
    inspectable directory, or records a specific Stage 8/Stage 10 gap before the
    artifact contract is claimed complete.
  - Test or evidence proves runtime argument/value capture remains absent.
  - Final evidence links the related merged issues/PRs and identifies whether
    no new code was needed or names the precise remaining gap.

## Open Questions

- None blocking for Stage 6.
- Stage 8 should decide whether #585 needs a technical spec that is primarily
  an evidence/audit plan or a narrow hardening plan after Stage 7 review.
- Stage 8 or Stage 10 should decide whether a concise docs page is useful, or
  whether source types, CLI help, tests, and completion evidence are sufficient
  artifact-contract documentation.
- A later issue should decide whether exact node evidence is needed and what
  runtime/compiler evidence can support it.
- A later issue should decide whether runtime value capture is ever allowed,
  including privacy, consent, redaction, retention, and storage controls.

## Sources

- Issue #585: https://github.com/hgahub/duumbi/issues/585
- Stage 5 decision for #585:
  https://github.com/hgahub/duumbi/issues/585#issuecomment-4629889364
- Stage 4 triage for #585:
  https://github.com/hgahub/duumbi/issues/585#issuecomment-4550311052
- Parent issue #580: https://github.com/hgahub/duumbi/issues/580
- `specs/DUUMBI-580/PRODUCT.md`
- `specs/DUUMBI-580/TECHNICAL.md`
- `specs/DUUMBI-583/PRODUCT.md`
- `specs/DUUMBI-583/TECHNICAL.md`
- `specs/DUUMBI-584/PRODUCT.md`
- `specs/DUUMBI-584/TECHNICAL.md`
- `specs/DUUMBI-586/PRODUCT.md`
- `specs/DUUMBI-586/TECHNICAL.md`
- `specs/DUUMBI-588/PRODUCT.md`
- `specs/DUUMBI-588/TECHNICAL.md`
- `specs/DUUMBI-587/PRODUCT.md`
- `specs/DUUMBI-587/TECHNICAL.md`
- `src/telemetry/mod.rs`
- `src/workspace.rs`
- `src/main.rs`
- `runtime/duumbi_runtime.c`
- `runtime/duumbi_runtime.h`
- `tests/integration_telemetry.rs`
- `tests/integration_phase1.rs`
- `tests/fixtures/telemetry/option_none_unwrap.jsonld`
- `tests/fixtures/telemetry/call_then_panic.jsonld`
- `docs/architecture.md`
- `docs/coding-conventions.md`
- `docs/automation/code-review-policy.md`
- `.github/workflows/spec-review-request.yml`
- `.github/workflows/copilot-review.yml`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active Glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Runtime Failure Feedback Loop:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Runtime Failure Feedback Loop.md`
- Service and Research Direction:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- DUUMBI Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- DUUMBI Agentic Development Runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- AI Code Review Service Policy:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/AI Code Review Service Policy.md`
