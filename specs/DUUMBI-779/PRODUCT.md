# DUUMBI-779: Preserve Scaled Write-Path Evidence And Classify Failures

Related to #779. This is a specification-only artifact. The execution issue
must remain open for Stage 7 Spec Review, Stage 8 technical specification,
Stage 9 approval, Stage 10 implementation, Stage 11 review, and Stage 12
closure.

## Summary

Make scaled `intent execute` failures diagnostically complete and
reproducible. Converge the regular benchmark path and the determinism replay
path on one structured per-attempt evidence contract so a developer can inspect
every failed scaled attempt after the run and determine which pipeline stage
failed, whether mutation or repair ran, what graph was produced, and which
validator, compiler, or verifier evidence supports the classification.

This issue is the first implementation prerequisite for the scaled write-path
autopsy. It does not improve pass rate, change prompts or models, or run the
authoring-ceiling experiment.

## Goal

Preserve scaled write-path evidence and classify failures so a developer can
inspect every failed attempt after the run and tell orchestration failure
apart from graph-representation or verifier failure.

User outcome: after `duumbi benchmark` or `duumbi determinism replay`, failed
attempts still have local, sanitized, stage-aware evidence, including whether
repair ran.

Non-goals: pass-rate work, prompt/model/routing/Op-set changes, the #781
authoring-ceiling experiment, cloud artifact upload, and committing raw
model I/O.

## Problem

The retained #689 scaled smoke report records two `logic_error` results and
zero repair attempts, but the current benchmark path cannot support that causal
conclusion.

Observed facts from source inspection:

- `src/bench/runner.rs` runs each attempt in `tempfile::TempDir`. The isolated
  workspace is dropped when `run_in_temp_workspace` returns, so generated
  graphs, intent state, validator output, and execution logs disappear with
  the attempt.
- When `intent::execute::run_execute` returns `Ok(false)`,
  `run_in_temp_workspace` discards the in-memory execution log and returns a
  short `intent execution failed: X/Y tests passed` error. The outer
  `run_single` error path then infers `repair_attempted` from
  `msg.contains("[Repair] Attempting repair")`. Because that marker lived only
  in the discarded log, unsuccessful runs can report `repair_attempted: false`
  even when the execute path emitted `[Repair] Attempting repair`.
- `src/intent/execute.rs` does contain a verifier-driven repair cycle. It
  emits `[Repair] Attempting repair…`, may apply repair patches, and returns
  `Ok(all_passed)` rather than a structured outcome. The #689 smoke report's
  `repair_attempts: 0` is therefore an observation from a lossy reporting
  path, not proof that repair never ran.
- On the `Ok(outcome)` path, any `tests_passed < tests_total` result is
  classified as `ErrorCategory::LogicError`. `categorize_error` also defaults
  unmatched messages to `logic_error`. That catch-all hides decomposition,
  export/call resolution, control-flow/SSA, graph validation, compiler/runtime,
  true product-logic, and verifier-mismatch failures.
- `src/determinism/runner.rs` already retains `execute.log`, supports
  `--keep-workspaces`, records graph/context hashes, and copies a bounded
  `.duumbi` snapshot into the replay bundle before the temp workspace is
  dropped. `src/determinism/evidence.rs` already versions replay reports as
  `duumbi.determinism.replay_report.v1`. Duplicating that machinery inside the
  benchmark runner would increase drift.
- `BenchmarkReport` currently has no `schema_version`. New fields on
  `BenchmarkResult` are mostly `#[serde(default)]` optional, which is the
  existing compatibility pattern used by #689. Existing JSON consumers include
  `duumbi benchmark --baseline`, `tests/integration_phase9c.rs`, bench report
  unit tests, and determinism reports that reuse `ErrorCategory`,
  `ProviderUsageSummary`, and `BenchmarkEvidence`.

The product problem is not that every scaled attempt must pass. The product
problem is that a failed attempt currently leaves too little durable evidence
to distinguish orchestration failure from graph-representation or verifier
failure, and that `logic_error` plus string-inferred repair telemetry makes
follow-up issues such as #781 scientifically unsafe.

## Outcome

When this work is complete:

- A developer can run `duumbi benchmark` or `duumbi determinism replay` and,
  after a failed attempt, inspect retained local evidence for that attempt.
- Each attempt records structured phase evidence: mutation retries, verifier
  results, repair attempted/applied/succeeded, and terminal status. Repair
  telemetry is typed, not inferred from leftover log text.
- Failed attempts retain enough evidence to identify the failing pipeline
  stage and a deterministic root-cause class, including explicit `unknown`.
- Default retained evidence is sanitized and hash-based: final graph hashes,
  intent-state hashes, validator/compiler/verifier summaries, sanitized
  execution transcript, exact provider/model identity when available, and
  prompt/context hashes.
- Exact model I/O is retained only behind an explicit local opt-in such as
  `--capture-model-io`, with secret redaction. It is never committed by
  default and never emitted to GitHub workflow summaries.
- Existing benchmark and replay JSON consumers keep working: current fields
  remain, new fields are additive or defaulted, and schema versioning is
  used when the report shape changes.
- Regression tests prove that a failed attempt which entered repair is
  reported as `repair_attempted=true` and that retained evidence survives the
  temporary workspace lifecycle.
- Success-path reports remain compatible with current `duumbi benchmark` and
  `duumbi determinism replay` callers.

## Scope

### In Scope

- A shared structured per-attempt execution and evidence contract used by
  both `duumbi benchmark` and `duumbi determinism replay`.
- Accurate mutation, retry, and repair telemetry on success and failure.
- Opt-in local retention of failed workspaces and bounded, sanitized
  evidence, reusing existing `--artifact-dir` and `--keep-workspaces`
  semantics where they already exist and adding compatible flags to
  benchmark if the regular path retains artifacts.
- Fine-grained deterministic root-cause taxonomy with explicit `unknown`.
- Schema versioning and compatibility handling for existing benchmark and
  replay JSON.
- Focused automated tests for evidence retention across `TempDir` teardown,
  repair-entered failure reporting, taxonomy classification, opt-in model I/O
  privacy, and success-path compatibility.
- Documentation of the new flags, evidence layout, retention bounds, and
  cleanup guidance.

### Explicitly Out Of Scope

- Changing prompts, models, routing, retry policy, or the Op set to improve
  pass rate.
- Running the authoring-ceiling experiment itself (#781).
- Cloud artifact upload, hosted telemetry, or GitHub Actions artifact
  publication of retained workspaces or model I/O.
- Committing raw provider payloads, secrets, large logs, or retained
  workspaces.
- Teaching #747 active learning from unclassified or incorrectly classified
  failures.
- Changing default `duumbi add`, Query mode mutation rules, TUI startup, or
  Studio behavior.
- Requiring users to choose or maintain a default model.
- Implementation code, Ralph cycles, or merging this spec PR during this
  specification stage.

## Constraints And Assumptions

Facts:

- Issue #779 is open, labeled `accepted` and `needs-spec`, and has a Stage 5
  Human Acceptance Decision dated 2026-09-06 with `Decision: Accept` and
  `Next state: Spec Needed`. Remaining open questions were recorded as
  non-blocking for acceptance and were assigned to Stage 6/8 specs.
- The Stage 5 rationale confirms the diagnostic gap in `src/bench/runner.rs`
  against the #689 smoke evidence and the #720 determinism evidence model.
- #689 delivered the scaled corpus and the lossy smoke evidence in
  `docs/e2e/results/duumbi-689-scaled-smoke-20260616.md` and
  `docs/e2e/duumbi-689-known-limitations.md`.
- #720 delivered determinism replay, artifact bundles, optional workspace
  retention, and schema-versioned replay reports.
- `src/intent/execute.rs` returns `Result<bool>`. Repair is a log-emitting
  side path inside `run_execute_with_progress`. REPL, CLI, workflow, Phase 15
  E2E, benchmark, and determinism all call this boolean API today.
- Determinism already depends on bench helpers (`extract_error_codes`,
  `filter_providers`, `load_archived_intent`, `provider_name`) and bench
  report types. Bench does not depend on the determinism module.
- `ErrorCategory` currently has `schema_error`, `type_error`, `logic_error`,
  `crash`, `provider_error`, `mutation_failed`, and `evidence_required`.
- `src/errors.rs` already defines structured codes used by validation and
  compilation, including `E009` schema, `E010` unresolved cross-module,
  `E008` link failure, and SSA/dominance diagnostics from
  `src/graph/validator.rs`.
- `src/knowledge/learning.rs` already sanitizes secret-bearing error
  summaries.

Assumptions:

- Sharing a lower-level attempt executor is safer than making benchmark call
  the full determinism replay runner, because replay also owns ledger events,
  equivalence metrics, rewrite comparison, and CI agreement thresholds that
  benchmark must not inherit.
- Existing JSON consumers can tolerate additive optional fields, as #689
  already did, but cannot tolerate removed or renamed required fields.
- Changing unclassified `Ok(false)` rows from `logic_error` to a finer class
  or `unknown` is an intended diagnostic correction. Baseline comparison
  docs must call out that category counts may shift.
- A mock or deterministic provider fixture is sufficient to prove evidence
  retention, repair telemetry, taxonomy, and privacy. Live provider calls
  are not required to close the diagnostic contract.
- Bounded local evidence is enough. Cloud upload would expand privacy risk
  without improving the autopsy prerequisite.

Constraints:

- Specs and PR wording must use non-closing references such as `Related to
  #779`. This spec PR must not close the execution issue.
- Default evidence must remain hash-based and sanitized.
- Exact model I/O, if captured, must stay local, redacted, opt-in, and
  excluded from committed docs and GitHub workflow summaries.
- Normal CI must not make live provider calls and must not upload retained
  workspaces or raw model I/O.
- Query mode must remain read-only.
- Retention must be bounded: no binaries, no credential values, truncated
  transcripts, and explicit cleanup guidance for `--keep-workspaces`.
- Cranelift internals must not leak outside `src/compiler/`.

## Decisions

- **Decision:** Use a source-repo file-based product spec plus a technical
  spec on the same branch.
  **Evidence:** The issue is cross-module, implementation-facing, and the
  Stage 5 decision plus this Stage 6 prompt requested combined product and
  technical artifacts.

- **Decision:** Prefer a shared lower-level attempt executor used by both
  benchmark and determinism replay. Do not make `duumbi benchmark` call the
  full determinism replay runner.
  **Evidence:** Code inspection shows both paths already call
  `intent::execute::run_execute` independently. Determinism adds ledger,
  agreement metrics, rewrite comparison, and CI thresholds that benchmark
  must not inherit. DUUMBI-720 already recommended extracting a shared
  internal attempt executor if duplication became meaningful. Bench does not
  currently depend on `src/determinism`.

- **Decision:** Default evidence is sanitized and hash-based. Exact model I/O
  is local opt-in only, with secret redaction, and is never default-committed
  or emitted to GitHub summaries.
  **Evidence:** Issue #779 privacy risk, existing replay prompt-hash
  `Partial` status, and `src/knowledge/learning.rs` secret sanitization.

- **Decision:** Root-cause taxonomy is deterministic rules first, with
  explicit `unknown`. Direct phase evidence is stored separately from the
  inferred class. Any later offline inference must be labeled as inference,
  not phase evidence, and is out of this issue's implementation.
  **Evidence:** Issue risk that a taxonomy can imply false certainty, and
  #747 must not learn from unclassified or incorrectly classified failures.

- **Decision:** Preserve backward-compatible fields for existing benchmark
  and replay JSON consumers. Version the schema when new required structure
  is added. Keep `ErrorCategory` values. Add finer `root_cause` as an
  additive field rather than replacing `error_category`.
  **Evidence:** `BenchmarkResult` already uses `serde(default)` optional
  fields; replay reports already have `schema_version`;
  `duumbi benchmark --baseline` deserializes previous JSON.

- **Decision:** Repair telemetry must come from typed execute outcome fields,
  not from scanning log text after `Ok(false)` log replacement.
  **Evidence:** The `Ok(false)` path in `run_in_temp_workspace` is the
  concrete cause of false `repair_attempted: false`.

- **Decision:** #781 authoring-ceiling experiment and pass-rate work are
  non-dependencies. This issue only makes later autopsy evidence trustworthy.
  **Evidence:** Issue #781 lists #779 as a prerequisite and is still in
  human-acceptance. Issue #779 scope explicitly excludes pass-rate work.

## Behavior

### Shared Attempt Contract

- Benchmark and determinism replay execute each attempt through one shared
  isolated-attempt executor.
- The executor initializes an isolated workspace, saves the intent, runs the
  structured execute path, captures hashes and phase evidence, copies bounded
  evidence out of the temp workspace, then drops the `TempDir`.
- The full determinism replay runner remains responsible for ledger events,
  agreement metrics, rewrite comparison, and CI thresholds. Benchmark does
  not grow those concerns.
- Existing public CLI entry points stay: `duumbi benchmark` and
  `duumbi determinism replay`. New flags are additive.

### Repair And Mutation Telemetry

- `repair_attempted` is true if and only if the execute path entered the
  verifier-driven repair cycle, including when repair applied no patch and
  when the attempt later failed.
- `repair_applied` records whether at least one repair patch was written.
- `repair_success` is true only when repair was attempted and final
  verification passed.
- `first_pass_success` is true only when verification passed before any
  repair cycle.
- Mutation retry counts and repair retry counts are copied from structured
  execute metadata when available, otherwise explicitly unavailable.
- String matching on `[Repair] Attempting repair` is not the source of
  truth after this issue.

### Evidence Retention

Default retained evidence for every attempt, success or failure:

- provider route and resolved model identity, or explicit unavailable reason
- prompt/context hashes, including honest `partial` status when final
  provider prompts are not exposed
- initial and final graph exact and semantic hashes when a graph exists
- intent spec hash and terminal intent status
- validator, compiler, and verifier summaries or hashes
- sanitized execution transcript
- structured phase events and terminal status
- root-cause class plus the phase evidence that produced it
- relative artifact paths that remain after `TempDir` teardown

Failed attempts always persist bounded sanitized evidence under a local
artifact directory so inspection does not depend on remembering a flag.
Recommended default root for benchmark is `.duumbi/benchmark/attempts`.
Determinism keeps its existing `--artifact-dir` default
(`.duumbi/determinism/replays`). `--artifact-dir` overrides the root.
Layout must be filesystem-safe and must reject path traversal.

`--keep-workspaces` retains a bounded `.duumbi` snapshot under the artifact
bundle, matching current determinism semantics. Default remains false.

`--capture-model-io` retains redacted prompt and response payloads locally
under the artifact bundle. Default is off. When off, only hashes are stored.

Retention bounds:

- no credential values, API keys, or secret-bearing headers
- no generated binaries or object files
- transcripts truncated to a documented size cap
- model I/O files excluded from Git, docs commits, and GitHub workflow
  summaries
- cleanup guidance documents how to delete `--keep-workspaces` snapshots

Failed attempts must still retain evidence after the temp workspace is
dropped. Success attempts must not break current JSON consumers.

### Root-Cause Taxonomy

Each attempt stores:

- `phase_evidence`: observed stage facts such as validation diagnostics,
  compiler/linker errors, verifier test counts, repair entered/applied, and
  provider errors
- `root_cause`: a deterministic class derived from those facts
- `root_cause_confidence`: `observed` for rule-based classification

Required classes:

- `schema_graph_validation`
- `task_decomposition_or_missing_function`
- `cross_module_resolution`
- `control_flow_or_ssa`
- `compiler_or_runtime`
- `product_logic_mismatch`
- `verifier_mismatch_or_unsupported_evidence`
- `provider_or_infrastructure`
- `unknown`

Rules must be deterministic and ordered. If no rule matches, the class is
`unknown`, not `logic_error`. `error_category` remains the coarse
compatibility field and continues to serialize with current snake_case
values. Mapping from `root_cause` onto `error_category` is documented in the
technical spec; unclassified rows no longer collapse into `logic_error`.

Later offline inference, if ever added by another issue, must use a separate
field such as `root_cause_inference` and must not overwrite `root_cause` or
`phase_evidence`. This issue does not implement that layer.

### JSON Compatibility

- Existing `BenchmarkResult` fields stay readable and writable.
- New fields are optional or defaulted for old documents.
- `BenchmarkReport` gains an additive `schema_version` when the new evidence
  fields are present. Old reports without the field still deserialize.
- Determinism replay keeps `duumbi.determinism.replay_report.v1` if new
  attempt fields are additive. If a required field is introduced, bump to
  `v2` and accept `v1` on read.
- `duumbi benchmark --baseline` continues to load previous reports. A
  documented category-count shift from finer classification is not a
  regression by itself.

### Failure And Privacy Behavior

- A failed attempt is a successful measurement. The command should still
  write the report and retain evidence.
- Provider or credential failures are classified as
  `provider_or_infrastructure`, not as graph or logic failure.
- Missing credentials fail closed and do not count as graph-generation
  failure.
- Interrupted runs preserve already-copied attempt evidence when possible.
- The command never writes raw credential values.
- GitHub workflow summaries, if they mention a run, may include hashes,
  taxonomy, and paths, never raw prompts, responses, or secret-bearing logs.
- Default `duumbi intent execute` user-facing logs remain available; this
  issue does not require changing interactive execute UX beyond exposing
  structured outcome data to the shared executor.

## BDD Scenarios

Feature: Scaled write-path attempt evidence and failure classification

  Rule: Failed attempts retain evidence after the temporary workspace is gone

    Scenario: Failed attempt retains evidence after TempDir lifecycle
      Given a benchmark or determinism attempt runs in an isolated TempDir
      And intent execute finishes with failure
      When the isolated TempDir is dropped
      Then the attempt evidence still exists under the configured artifact dir
      And the evidence includes graph hashes, sanitized execution transcript,
      intent status, and validator or verifier summaries
      And the report points at those surviving artifact paths

  Rule: Repair telemetry is typed and accurate on failure

    Scenario: Repair-entered failure reports repair_attempted true
      Given intent execute enters the verifier-driven repair cycle
      And the attempt still fails after repair
      When the benchmark or replay report is written
      Then `repair_attempted` is true
      And `first_pass_success` is false
      And `repair_success` is false
      And the classification does not depend on scanning a replaced short
      error string for `[Repair] Attempting repair`

  Rule: Taxonomy distinguishes listed failure classes

    Scenario: Taxonomy distinguishes listed failure classes
      Given failed attempts whose phase evidence matches schema validation,
      missing function or decomposition, cross-module resolution,
      control-flow or SSA, compiler or runtime, product-logic mismatch,
      verifier mismatch or unsupported evidence, and provider failure
      When the reports are written
      Then each attempt receives the corresponding `root_cause` class
      And unmatched failures receive `unknown`
      And `phase_evidence` remains present even when `root_cause` is
      `unknown`
      And `error_category` remains populated for existing consumers

  Rule: Opt-in model I/O stays local and redacted

    Scenario: Opt-in model I/O stays local and redacted
      Given the user enables `--capture-model-io`
      When an attempt runs
      Then redacted model input and output files are written only under the
      local artifact dir
      And secret-bearing headers and credential values are absent
      And GitHub workflow summaries and default committed reports contain
      hashes rather than raw payloads
      And without the flag only sanitized hashes are retained

  Rule: Success path stays compatible

    Scenario: Success path still compatible
      Given an attempt passes verifier or accepted evidence without repair
      When the report is written
      Then existing fields such as `success`, `tests_passed`, `tests_total`,
      `duration_secs`, `provider`, and `attempt` remain present
      And `repair_attempted` is false
      And `first_pass_success` is true
      And old baseline JSON without the new fields still deserializes
      And current `duumbi benchmark --output` and `duumbi determinism replay
      --output` callers can read the report without a breaking rename

## Tasks

1. Specify and implement the shared isolated-attempt executor and structured
   execute outcome.
2. Stop replacing `Ok(false)` logs with a short error that drops repair
   evidence.
3. Copy bounded evidence out of `TempDir` before drop; add compatible
   `--artifact-dir` / `--keep-workspaces` to benchmark.
4. Add `--capture-model-io` as local opt-in with redaction.
5. Add deterministic `root_cause` plus `phase_evidence`, keeping
   `error_category`.
6. Version report schemas additively and update baseline docs.
7. Add regression tests for TempDir survival, repair-entered failure,
   taxonomy classes, opt-in I/O privacy, and success compatibility.
8. Document flags, layout, retention bounds, and cleanup.

Independent slices:

- Structured execute outcome can land before artifact-layout flags.
- Taxonomy rules can be unit-tested with synthetic phase evidence.
- Privacy/redaction tests can use fixtures without providers.
- Live provider evidence is optional and must stay local.

## Checks

Independently testable acceptance criteria:

1. After a failed attempt, artifact files exist at the reported paths after
   the process has dropped the `TempDir`.
2. A fixture that emits the repair-cycle entry and then fails reports
   `repair_attempted=true`.
3. Synthetic or fixture failures covering each required `root_cause` class
   serialize that class; a non-matching failure serializes `unknown`.
4. `--capture-model-io` writes redacted local files and does not put raw
   payloads into JSON reports used for GitHub summaries; without the flag,
   reports contain hashes only.
5. A successful first-pass result still deserializes as today's
   `BenchmarkResult` / `ReplayAttempt` required fields, and a pre-change
   baseline JSON still loads.
6. `cargo fmt --check`, focused tests, and `cargo clippy --all-targets -- -D
   warnings` pass on the implementation PR.
7. No committed raw provider payloads, secrets, or retained workspaces.

Expected artifacts for later Stage 10, not this spec PR:

- shared attempt executor and structured execute outcome
- report schema versioning
- tests named in the technical spec
- docs for flags and retention
- optional local live smoke evidence that is not uploaded to GitHub

## Open Questions

None blocking.

Resolved in this spec:

- Shared lower-level attempt executor, not benchmark-calls-determinism.
- Default evidence is sanitized/hash-based; `--capture-model-io` is local
  opt-in with redaction.
- Taxonomy is deterministic rules plus `unknown`; later inference is a
  separate labeled field and out of scope.
- Existing JSON fields stay; schema version is added or bumped when needed.

Non-blocking Stage 10 choices:

- Exact module path of the shared executor (`src/intent/attempt.rs` versus
  `src/bench/attempt.rs`), provided both runners call the same contract.
- Transcript size cap, as long as it is documented, tested, and bounded.
- Whether success attempts omit file copies and keep hashes in JSON only,
  provided failed attempts still persist files by default.

Owner input is not required to start Stage 10 after spec approval. The only
later Owner call is the existing Ralph external-LLM gate if a live provider
smoke would exceed USD 1, which this issue should avoid by using fixtures.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/779
- Stage 5 acceptance:
  https://github.com/hgahub/duumbi/issues/779#issuecomment-5562078743
- Related corpus and lossy smoke evidence: https://github.com/hgahub/duumbi/issues/689
- Related determinism evidence: https://github.com/hgahub/duumbi/issues/720
- Non-dependency experiment: https://github.com/hgahub/duumbi/issues/781
- Downstream consumers, not blockers: https://github.com/hgahub/duumbi/issues/739,
  https://github.com/hgahub/duumbi/issues/747
- Product specs: `specs/DUUMBI-689/PRODUCT.md`, `specs/DUUMBI-720/PRODUCT.md`
- Technical specs: `specs/DUUMBI-689/TECHNICAL.md`, `specs/DUUMBI-720/TECHNICAL.md`
- Source:
  - `src/bench/runner.rs`
  - `src/bench/report.rs`
  - `src/determinism/runner.rs`
  - `src/determinism/evidence.rs`
  - `src/intent/execute.rs`
  - `src/cli/mod.rs`
  - `src/errors.rs`
  - `src/graph/validator.rs`
  - `src/knowledge/learning.rs`
- Evidence docs:
  - `docs/e2e/results/duumbi-689-scaled-smoke-20260616.md`
  - `docs/e2e/duumbi-689-known-limitations.md`
  - `docs/e2e/results/duumbi-720-determinism-replay-20260619.md`
- Tests: `tests/integration_phase9c.rs`, `tests/integration_duumbi720_determinism.rs`
- Repo instructions: `AGENTS.md`
