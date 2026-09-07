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
- Classification uses only **terminal** phase evidence. Recovered
  mutation/provider errors that were retried or repaired must stay in
  `phase_evidence` and must not drive final `root_cause`.
- Success attempts do not receive a failure `root_cause`. `product_logic_mismatch`
  requires positive evidence that the verifier could judge the claimed
  behavior.
- Default retained evidence is sanitized and hash-based: final graph hashes,
  intent-state hashes, validator/compiler/verifier summaries, sanitized
  execution transcript, exact provider/model identity when available, and
  prompt/context hashes. Hashes cannot reconstruct the graph; graph/intent
  content for autopsy requires `--keep-workspaces`.
- Exact model I/O is retained only behind an explicit local opt-in such as
  `--capture-model-io`, with secret redaction. It is never committed by
  default and never emitted to GitHub workflow summaries.
- Every execute and infrastructure error path finalizes partial evidence.
  Execution outcome and evidence-persistence status are separate fields.
  Persistence failure does not override an established execution cause.
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
- A deterministic provider fixture that drives **actual** execute through
  repair is required to prove evidence retention, repair telemetry,
  taxonomy, and privacy. Injecting a pre-built structured outcome is not
  sufficient. Live provider calls are not required to close the diagnostic
  contract.
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
- Retention must follow the allowlist, byte/file-count caps, sanitize-
  before-persist, symlink, and flag-matrix rules in Evidence Retention.
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

- **Decision:** Classification uses terminal phase evidence only. Recovered
  errors remain recorded but never select `root_cause`. Success paths omit
  failure `root_cause`. `product_logic_mismatch` requires positive verifier
  applicability evidence. Attribution language is `matched_rule` or
  `no_matching_rule`, never `root_cause_confidence: observed`.
  **Evidence:** Codex Stage 6/8 review of PR #787, blocking finding 1.

- **Decision:** Every error exit, including init/intent-save failure and
  infrastructure `Err` after repair began, must finalize a partial attempt
  record. `evidence_persistence` is separate from execution `root_cause`.
  Persistence failure does not rewrite an established execution cause.
  **Evidence:** Codex Stage 6/8 review of PR #787, blocking finding 2.

- **Decision:** Retention is an allowlisted, byte- and file-count-bounded
  contract. Sanitize before persist. Flag matrix for `--keep-workspaces` ×
  `--capture-model-io` is specified below, including providers that do not
  expose payloads. Default autopsy is hash/summary based; reconstructing
  graph or intent content requires `--keep-workspaces`.
  **Evidence:** Codex Stage 6/8 review of PR #787, blocking finding 3;
  existing `safe_artifact_key` and `#720` workspace snapshot.

- **Decision:** Serialized unknown compatibility is locked now: do **not**
  add `ErrorCategory::Unknown`. Failed attempts always include the
  `error_category` field. Classified failures use an existing snake_case
  enum value. Unmatched failures serialize `"error_category": null` and
  `"root_cause": "unknown"`. Historical baselines omit `root_cause` and
  remain readable. Old readers ignore additive fields and accept null
  `error_category` because the field is already `Option`.
  **Evidence:** Codex Stage 6/8 review of PR #787, blocking finding 4;
  `src/bench/report.rs` `error_category: Option<ErrorCategory>` without
  `deny_unknown_fields`.

## Behavior

### Shared Attempt Contract

- Benchmark and determinism replay execute each attempt through one shared
  isolated-attempt executor.
- The executor initializes an isolated workspace, saves the intent, runs the
  structured execute path, captures hashes and phase evidence, copies bounded
  evidence out of the temp workspace, then drops the `TempDir`.
- A `finally`-style finalizer runs on every exit, including `Err` after
  repair began, init failure, and intent-save failure. Partial outcomes are
  preserved before `TempDir` drop.
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

JSON fields retained for every attempt, success or failure:

- provider route and resolved model identity, or explicit unavailable reason
- prompt/context hashes, including honest `partial` status when final
  provider prompts are not exposed
- initial and final graph exact and semantic hashes when a graph exists
- intent spec hash and terminal intent status
- validator, compiler, and verifier summaries or hashes
- sanitized execution transcript hash plus, on failed attempts, the truncated
  transcript file
- structured phase events (terminal and recovered) and terminal status
- `root_cause` plus `root_cause_attribution` on failures; both omitted/null
  on success
- `evidence_persistence` status
- relative artifact paths that remain after `TempDir` teardown

File persistence differs by outcome:

- **Failed attempts always persist** the allowlisted files under a local
  artifact directory so inspection does not depend on remembering a flag.
- **Success attempts persist hashes and summaries in JSON only** by default.
  They do not copy bulky workspace files unless `--keep-workspaces` is set.
  Success `artifact_paths` may be empty.

Recommended default root for benchmark is `.duumbi/benchmark/attempts`.
Determinism keeps its existing `--artifact-dir` default
(`.duumbi/determinism/replays`). `--artifact-dir` overrides the root.
Layout is `<artifact-dir>/<run-id>/<task-key>/<provider-key>/<attempt>/`
with `safe_artifact_key` components and a unique `run-id` per invocation.
Path traversal is rejected. If the destination already exists, the executor
must not overwrite it; it records a persistence error or a unique suffix
and never merges two attempts into one directory.

#### Allowlist

Default failed-attempt files (sanitize, then persist, never persist then
redact):

- `execute.log` (sanitized, truncated)
- `phase_evidence.json`
- `intent-status.txt` (terminal intent status plus validator/verifier
  summary text)
- `summaries.json` (validator, compiler, verifier hashes/summaries)
- `hashes.json` when not already fully inlined in the parent report

`--keep-workspaces` additionally allowlists:

- sanitized intent spec (YAML/JSON)
- sanitized graph JSON-LD under `.duumbi/graph` or equivalent
- existing non-secret `.duumbi` config needed to reopen the attempt

Never allowlisted:

- generated binaries, `*.o`, `target/`, object files
- credential values, API keys, secret-bearing headers
- model prompt/response files unless `--capture-model-io` is on
- provider caches, previous `model-io/`, `prompts/`, or `responses/`
  directories that would bypass the capture flag
- symlinks that escape the TempDir workspace

#### Size And Count Limits

Locked bounds (Stage 10 may implement with these exact caps, not larger
undocumented limits):

- sanitized `execute.log`: 256 KiB
- model I/O: 256 KiB per file after redaction
- per-attempt file count: 32 default, 64 with `--keep-workspaces`
- per-attempt total bytes: 1 MiB default, 8 MiB with `--keep-workspaces`
- per-run total across attempts: 32 MiB

Overflow must write truncation metadata (`truncated: true`,
`original_bytes`, `retained_bytes`, `omitted_files`) and drop lowest-priority
files rather than exceeding the cap. Priority: hashes/summaries, then
`execute.log`, then graph/intent snapshot, then model I/O.

#### Flag Matrix

| `--keep-workspaces` | `--capture-model-io` | Files |
| --- | --- | --- |
| false | false | Failed: allowlist without workspace snapshot and without model-io files. Success: JSON hashes only. |
| true | false | Allowlist plus bounded sanitized graph/intent snapshot. No model-io files. Previously stored prompts/responses in the workspace are excluded. |
| false | true | Failed allowlist plus redacted model-io files **only if** the capture seam obtained payloads. No graph snapshot. |
| true | true | Snapshot plus redacted model-io when payloads exist. Snapshot copy still excludes uncaptured prompt/response caches. |

When a provider or orchestrator does not expose raw payloads:

- `model_io_status` is `unavailable` or `partial` with reason
  `provider_did_not_expose_payload`
- no placeholder/fake payload files are written
- prompt hashes stay honest `partial` when the final provider prompt is not
  exposed
- `--capture-model-io` does not fail the attempt

#### Autopsy Versus Hashes

Hashes, summaries, and truncated transcripts are enough to **classify** a
failed attempt. They cannot reconstruct the graph or intent. Graph-content
or intent-content autopsy for #779 requires `--keep-workspaces`. Default
evidence remains hash-based.

#### Symlink And Collision Protection

Copy must not follow a symlink whose resolved target is outside the
attempt TempDir. Escaping links are skipped and listed in truncation
metadata as omitted. `safe_artifact_key` remains the only path-component
encoder. Concurrent runs must not share `run-id`.

Cleanup guidance documents how to delete `--keep-workspaces` snapshots.
Model I/O files are excluded from Git, docs commits, and GitHub workflow
summaries.

### Root-Cause Taxonomy

Each attempt stores:

- `phase_evidence`: observed stage facts. Every event has
  `status: terminal | recovered | informational`. Recovered events include
  mutation or provider errors that were retried successfully and repair
  attempts that were later superseded by a later phase. Informational events
  include hashes captured and repair-entered markers that did not fail.
- `root_cause`: a rule-derived class computed from **terminal** events only
- `root_cause_attribution`: `matched_rule` when a rule fired, or
  `no_matching_rule` when the class is `unknown`

Do not serialize `root_cause_confidence: observed`. Observed facts live in
`phase_evidence`. Attribution lives in `root_cause` +
`root_cause_attribution`. `unknown` is rule-derived (`no_matching_rule`),
not an observed cause.

#### Success-path classification

When `success` is true:

- `root_cause` is JSON `null` and may be omitted
- `root_cause_attribution` is omitted
- `error_category` is omitted or JSON `null` (current success behavior)
- recovered mutation/provider errors that occurred before eventual success
  remain in `phase_evidence` with `status: recovered` and do not produce a
  failure class

#### Verifier applicability before product logic

Assign `product_logic_mismatch` only when **all** of the following terminal
facts are present:

1. A graph compiled or otherwise produced a runnable artifact.
2. The verifier actually executed applicable checks (`tests_total > 0`, or
   process-evidence status is `passed`/`failed`).
3. Terminal evidence has no `unsupported`, `broader_evidence_required`, or
   verification-gap marker.

If the verifier or process checker cannot judge the claimed behavior, the
class is `verifier_mismatch_or_unsupported_evidence`, even if tests also
look empty or failed. That rule is evaluated **before**
`product_logic_mismatch`.

#### Required classes

- `schema_graph_validation`
- `task_decomposition_or_missing_function`
- `cross_module_resolution`
- `control_flow_or_ssa`
- `compiler_or_runtime`
- `product_logic_mismatch`
- `verifier_mismatch_or_unsupported_evidence`
- `provider_or_infrastructure`
- `unknown`

Rules are deterministic and ordered. First matching rule on **terminal**
evidence wins. Overlapping recovered+terminal evidence must not let the
recovered error win. If no rule matches, the class is `unknown`, not
`logic_error`.

`error_category` remains the coarse compatibility field using the existing
snake_case enum. Mapping is locked in JSON Compatibility below.
Unclassified rows no longer collapse into `logic_error`.

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

Locked `root_cause` → `error_category` mapping (existing enum only; **do
not** add `ErrorCategory::Unknown`):

| `root_cause` | serialized `error_category` |
| --- | --- |
| `schema_graph_validation` | `"schema_error"` |
| `control_flow_or_ssa` | `"schema_error"` |
| `task_decomposition_or_missing_function` | `"mutation_failed"` |
| `cross_module_resolution` | `"mutation_failed"` |
| `compiler_or_runtime` | `"crash"` |
| `product_logic_mismatch` | `"logic_error"` |
| `verifier_mismatch_or_unsupported_evidence` | `"evidence_required"` |
| `provider_or_infrastructure` | `"provider_error"` |
| `unknown` | JSON `null` (field present on failed attempts) |
| success | omitted or JSON `null` (current behavior) |

Failed attempts always include the `error_category` key. For `unknown`, the
value is JSON `null` rather than omitted, so consumers can distinguish
"success" from "unmatched failure". Implementation must serialize that null
even if other optionals use `skip_serializing_if`.

Bidirectional compatibility:

- **New readers + historical baselines:** missing `root_cause` /
  `phase_evidence` / `schema_version` default to absent. Historical
  `error_category` strings still parse. New code must not require the new
  fields when loading `--baseline`.
- **Existing readers + new reports:** extra fields are ignored (no
  `deny_unknown_fields` today). Classified failures use enum strings those
  readers already accept. Unknown failures use `error_category: null`, which
  already deserializes as `Option::None`. Old binaries do not see a new enum
  variant.

Representative JSON (fields abbreviated):

Success:

```json
{
  "showcase": "scaled_math_pipeline",
  "provider": "mock",
  "attempt": 1,
  "success": true,
  "tests_passed": 4,
  "tests_total": 4,
  "duration_secs": 1.2,
  "repair_attempted": false,
  "first_pass_success": true,
  "root_cause": null,
  "evidence_persistence": "json_only"
}
```

Classified failure:

```json
{
  "success": false,
  "tests_passed": 0,
  "tests_total": 4,
  "repair_attempted": true,
  "repair_applied": false,
  "repair_success": false,
  "first_pass_success": false,
  "root_cause": "schema_graph_validation",
  "root_cause_attribution": "matched_rule",
  "error_category": "schema_error",
  "evidence_persistence": "complete",
  "phase_evidence": {
    "events": [
      {"phase": "provider", "status": "recovered", "code": "timeout"},
      {"phase": "validate", "status": "terminal", "code": "E009"}
    ]
  }
}
```

Unknown failure:

```json
{
  "success": false,
  "repair_attempted": false,
  "root_cause": "unknown",
  "root_cause_attribution": "no_matching_rule",
  "error_category": null,
  "phase_evidence": {"events": [{"phase": "complete", "status": "terminal"}]},
  "evidence_persistence": "complete"
}
```

### Failure And Privacy Behavior

- A failed attempt is a successful measurement. The command should still
  write the report and retain evidence.
- Execution outcome (`success`, `root_cause`, `error_category`) is separate
  from `evidence_persistence` (`complete`, `partial`, `failed`,
  `json_only`).
- Provider or credential failures are classified as
  `provider_or_infrastructure`, not as graph or logic failure.
- Missing credentials fail closed and are **excluded from graph-failure
  accounting**. Relabeling them `provider_or_infrastructure` is not enough:
  graph-failure totals, taxonomy histograms of graph classes, and scaled
  graph-generation denominators must not increment. They may appear in a
  separate infra/credential bucket.
- Interrupted runs preserve already-copied attempt evidence when possible.
- The command never writes raw credential values.
- GitHub workflow summaries, if they mention a run, may include hashes,
  taxonomy, and paths, never raw prompts, responses, or secret-bearing logs.
- Default `duumbi intent execute` user-facing logs remain available; this
  issue does not require changing interactive execute UX beyond exposing
  structured outcome data to the shared executor.

#### Evidence finalization on every error exit

The attempt finalizer always runs, including:

- `TempDir` create failure (record persistence/infra failure with no
  workspace)
- workspace init failure
- intent-save failure
- structured execute `Err` after repair has already begun
- copy/retention I/O failure
- process interruption after some files were copied

Partial outcomes (phase events so far, hashes so far, sanitized log so far,
repair telemetry already known) must be copied before `TempDir` drop.

When retention/copy fails:

- `root_cause` stays the execution classification if one was already
  established. Persistence failure does **not** override it.
- `evidence_persistence` becomes `failed` or `partial`, with
  `persistence_error` text (sanitized).
- The `--output` JSON report is the durable record of that persistence
  status. Already-copied files under the attempt directory remain.
- Default process exit follows existing measurement behavior (failed
  attempts do not by themselves fail the process).
- If even the JSON report cannot be written, the process exits non-zero,
  prints a sanitized stderr diagnostic, and leaves any already-copied
  attempt files on disk.
- `--ci`: persistence `failed` is an infrastructure CI failure, distinct
  from graph-failure kill criteria.

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
      And `repair_applied` is false when no repair patch was written
      And the classification does not depend on scanning a replaced short
      error string for `[Repair] Attempting repair`

    Scenario: Repair applies a patch and still fails
      Given intent execute enters repair and writes at least one patch
      And verification still fails
      When the report is written
      Then `repair_attempted` is true
      And `repair_applied` is true
      And `repair_success` is false

    Scenario: Repair converts the attempt to success
      Given intent execute fails first-pass verification
      And repair applies a patch
      And final verification passes
      When the report is written
      Then `success` is true
      And `repair_attempted` is true
      And `repair_success` is true
      And `root_cause` is null
      And recovered first-pass failure events do not select a failure class

    Scenario: Failure after repair entry with retries remaining
      Given repair is entered
      And retry budget still has remaining attempts
      And the attempt ends unsuccessful
      When the report is written
      Then remaining retry availability is recorded in phase evidence
      And `repair_attempted` is true
      And `success` is false

  Rule: Taxonomy uses terminal evidence and verifier applicability

    Scenario: Taxonomy distinguishes listed failure classes
      Given failed attempts whose **terminal** phase evidence matches schema
      validation, missing function or decomposition, cross-module
      resolution, control-flow or SSA, compiler or runtime, product-logic
      mismatch, verifier mismatch or unsupported evidence, and provider
      failure
      When the reports are written
      Then each attempt receives the corresponding `root_cause` class
      And unmatched failures receive `unknown` with
      `root_cause_attribution` `no_matching_rule`
      And `phase_evidence` remains present even when `root_cause` is
      `unknown`
      And classified failures populate `error_category` with the mapped
      existing enum value
      And unknown failures serialize `"error_category": null`

    Scenario: Recovered provider error does not become root_cause
      Given a provider timeout is retried successfully
      And the attempt later fails graph validation with E009
      When the report is written
      Then phase evidence includes the timeout with status recovered
      And `root_cause` is `schema_graph_validation`
      And `root_cause` is not `provider_or_infrastructure`

    Scenario: Overlapping terminal diagnostics pick the first matching rule
      Given terminal evidence includes both schema validation E009 and a
      later verifier test failure
      When the report is written
      Then `root_cause` is `schema_graph_validation`
      And the later test failure is recorded but does not win

    Scenario: Product logic requires verifier applicability
      Given a compiled graph whose verifier did not run applicable checks
      And process evidence is unsupported
      When the report is written
      Then `root_cause` is `verifier_mismatch_or_unsupported_evidence`
      And `root_cause` is not `product_logic_mismatch`

    Scenario: Success path has no failure root_cause
      Given first-pass verification passes
      When the report is written
      Then `success` is true
      And `root_cause` is null
      And `error_category` is omitted or null

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

    Scenario: Capture flag combinations including unavailable payloads
      Given each combination of `--keep-workspaces` and `--capture-model-io`
      And a provider that does not expose raw response payloads
      When attempts run
      Then model-io files exist only when the flag is on and payloads were
      actually captured
      And keep-workspaces snapshots never include previously stored
      prompts or responses unless `--capture-model-io` is on
      And unavailable payloads record `model_io_status` without inventing
      bodies

    Scenario: Default failed retention without extra flags
      Given a failed attempt with default flags
      When TempDir is dropped
      Then allowlisted sanitized files exist under the artifact dir
      And they include terminal intent status and validator or verifier
      summaries
      And no workspace graph snapshot is copied
      And no model-io files are written

    Scenario: Persistence failure does not rewrite execution cause
      Given execute already classified `schema_graph_validation`
      And copying artifacts fails
      When the report is written
      Then `root_cause` remains `schema_graph_validation`
      And `evidence_persistence` is `failed` or `partial`
      And the JSON report still exists unless report write also failed

    Scenario: Infra error after repair began still keeps partial evidence
      Given repair has been entered
      And execute then returns an infrastructure Err
      When the TempDir is dropped
      Then `repair_attempted` is true
      And partial phase evidence and sanitized log survive under the
      artifact dir or in the JSON report
      And `evidence_persistence` is not omitted

    Scenario: Missing credentials are excluded from graph-failure accounting
      Given a provider route whose credentials are absent
      When the run finishes
      Then those attempts are `provider_or_infrastructure`
      And graph-failure totals and graph-class histograms do not increment
      And the exclusion is not implemented merely by relabeling a row that
      still counts as a graph failure

    Scenario: Symlink escape is not copied
      Given the temp workspace contains a symlink pointing outside itself
      When evidence is copied
      Then the escaping link is omitted
      And truncation metadata lists it
      And files outside the TempDir are unchanged

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
      And bulky workspace files are not copied unless `--keep-workspaces`
      is set

## Tasks

1. Specify and implement the shared isolated-attempt executor and structured
   execute outcome, including a finally-style evidence finalizer.
2. Stop replacing `Ok(false)` logs with a short error that drops repair
   evidence.
3. Copy bounded allowlisted evidence out of `TempDir` before drop; add
   compatible `--artifact-dir` / `--keep-workspaces` to benchmark.
4. Add `--capture-model-io` as local opt-in with a provider capture seam and
   redaction. Exclude prior prompt/response caches from snapshots unless
   the flag is on.
5. Add deterministic `root_cause` plus terminal/recovered `phase_evidence`,
   keeping `error_category` as specified in JSON Compatibility.
6. Version report schemas additively and update baseline docs.
7. Add a deterministic provider fixture that drives **actual** execute
   through repair into both runner reports, plus tests for retention,
   persistence failure, taxonomy/privacy edges, and credential exclusion.
8. Document flags, layout, retention bounds, and cleanup.

Independent slices:

- Structured execute outcome can land before artifact-layout flags.
- Taxonomy rules can be unit-tested with synthetic phase evidence **and**
  must also be proven through the live execute fixture.
- Privacy/redaction tests can use fixtures without live providers.
- Live provider evidence is optional and must stay local.

## Checks

Independently testable acceptance criteria:

1. After a failed attempt, artifact files exist at the reported paths after
   the process has dropped the `TempDir`. Default retention (no extra flags)
   includes terminal intent status and validator/verifier summaries.
2. A deterministic provider fixture that drives actual `run_execute` through
   repair, not a pre-built structured outcome, reports
   `repair_attempted=true` on failure and the matching repair fields on
   no-patch, patched-failure, and repaired-success cases. The same fixture
   feeds both benchmark and determinism reports.
3. Synthetic **and** execute-driven fixtures covering each required
   `root_cause` class serialize that class; a non-matching failure
   serializes `unknown` with `"error_category": null`. Overlapping-rule and
   recovered-error fixtures pass.
4. `--capture-model-io` writes redacted local files only when payloads were
   captured and does not put raw payloads into JSON reports used for GitHub
   summaries; without the flag, reports contain hashes only. Keep-workspaces
   copies cannot smuggle prior prompts/responses.
5. A successful first-pass result still deserializes as today's
   `BenchmarkResult` / `ReplayAttempt` required fields, and a pre-change
   baseline JSON still loads. Success attempts do not copy bulky files by
   default.
6. Missing-credential attempts are excluded from graph-failure accounting.
7. Persistence-failure fixture keeps the execution `root_cause` and records
   `evidence_persistence`.
8. `cargo fmt --check`, focused tests, and `cargo clippy --all-targets -- -D
   warnings` pass on the implementation PR.
9. No committed raw provider payloads, secrets, or retained workspaces.

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
  opt-in with redaction. Graph/intent content autopsy needs
  `--keep-workspaces`; hashes remain the default.
- Taxonomy is deterministic rules plus `unknown`; later inference is a
  separate labeled field and out of scope. Terminal vs recovered, success
  classification, and verifier-applicability gating are locked.
- Existing JSON fields stay; schema version is added or bumped when needed.
- `error_category` for unknown failures is JSON `null` with `root_cause:
  unknown`; no new `ErrorCategory` variant.
- Success attempts persist JSON hashes/summaries only; failed attempts
  always persist allowlisted files. `--keep-workspaces` snapshots success
  or failure when requested.
- Retention caps, allowlist, flag matrix, symlink/collision, and capture
  seam are locked above.
- Persistence failure does not override execution `root_cause`.

Non-blocking Stage 10 choices:

- Exact module path of the shared executor (`src/intent/attempt.rs` versus
  `src/bench/attempt.rs`), provided both runners call the same contract.

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
