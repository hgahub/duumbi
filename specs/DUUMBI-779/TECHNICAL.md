# DUUMBI-779: Preserve Scaled Write-Path Evidence And Classify Failures - Technical Specification

Related to #779. This is a specification-only artifact. The execution issue
must remain open for Stage 9 Technical Spec Review, Stage 10 implementation,
Stage 11 implementation review, and Stage 12 closure.

Do not merge this specification PR as implementation completion, and do not
start Ralph cycles from this Stage 8 artifact until Stage 9 approval.

## Implementation Objective

Implement the product spec in `specs/DUUMBI-779/PRODUCT.md` so failed scaled
`intent execute` attempts remain diagnostically complete after the isolated
workspace is dropped.

The finished implementation must:

- Extract a shared lower-level per-attempt executor used by both
  `src/bench/runner.rs` and `src/determinism/runner.rs`.
- Replace boolean-only `run_execute` consumption plus log-string repair
  inference with a typed execution outcome.
- Copy bounded sanitized evidence out of `tempfile::TempDir` before drop.
- Classify failures with a deterministic root-cause taxonomy that includes
  `unknown`.
- Preserve existing benchmark and replay JSON fields; version schemas when
  new structure is added.
- Add `--capture-model-io` as local opt-in with secret redaction.
- Prove the product BDD scenarios with automated tests.

This issue does not improve pass rate, change prompts or models, run #781,
upload cloud artifacts, or teach #747 from unclassified failures.

## Agent Audience

- Stage 10 implementation agents running bounded Ralph cycles.
- Stage 9 technical reviewers and Stage 11 implementation reviewers.
- Maintainers consuming `duumbi benchmark --output` and
  `duumbi determinism replay --output` JSON.

Optimize for honest, bounded, local evidence. A failed attempt is valid
measurement if the retained evidence and classification are accurate.

## Source Context

- Product spec: `specs/DUUMBI-779/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/779
- Stage 5 acceptance:
  https://github.com/hgahub/duumbi/issues/779#issuecomment-5562078743
- Related specs: `specs/DUUMBI-689/PRODUCT.md`,
  `specs/DUUMBI-689/TECHNICAL.md`, `specs/DUUMBI-720/PRODUCT.md`,
  `specs/DUUMBI-720/TECHNICAL.md`
- Repo instructions: `AGENTS.md`
- Architecture: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant code verified for this spec:

- `src/bench/runner.rs`
  - `run_in_temp_workspace` creates `tempfile::TempDir`, calls
    `intent::execute::run_execute`, infers repair from log lines containing
    `[Repair] Attempting repair`, and on `Ok(false)` returns
    `Err(format!("intent execution failed: {tests_passed}/{tests_total} tests passed"))`,
    dropping the log that contained the repair marker.
  - On `Ok(outcome)` with incomplete tests, `error_category` is hard-coded
    `ErrorCategory::LogicError`.
- `src/bench/report.rs`
  - `BenchmarkResult` / `BenchmarkReport` have no `schema_version`.
  - New #689 fields are mostly `#[serde(default)]` optional.
  - `categorize_error` defaults unmatched messages to `LogicError`.
  - `load_baseline` deserializes previous JSON for `--baseline`.
- `src/determinism/runner.rs`
  - Owns `--artifact-dir` and `--keep-workspaces`.
  - Writes `execute.log`, optional workspace snapshot, graph hashes, and
    partial prompt hashes, then drops its own `TempDir`.
  - Still calls boolean `run_execute` and can classify failures as
    `LogicError` when execute returns `Ok(false)` without an error string.
- `src/determinism/evidence.rs`
  - `REPLAY_REPORT_SCHEMA_VERSION = "duumbi.determinism.replay_report.v1"`.
  - `ReplayAttempt` already has hashes, `artifact_paths`,
    `model_identity`, and `prompt_hashes`.
- `src/intent/execute.rs`
  - `run_execute` / `run_execute_with_progress` return `Result<bool>`.
  - Repair cycle emits `[Repair] Attempting repair…`, may apply patches,
    re-runs verifier tests, and returns `Ok(all_passed)`.
  - Callers: CLI, REPL, workflow, Phase 15 E2E, bench, determinism.
- `src/cli/mod.rs`
  - Benchmark flags: `--suite`, `--smoke`, `--showcase`, `--provider`,
    `--attempts`, `--output`, `--ci`, `--baseline`.
  - Determinism replay flags already include `--artifact-dir`,
    `--markdown-output`, `--ci` thresholds, and `--keep-workspaces`.
- `src/errors.rs` and `src/graph/validator.rs` provide deterministic
  diagnostic codes and SSA/dominance messages for taxonomy rules.
- `src/knowledge/learning.rs` sanitizes secret-bearing error summaries.
- Tests: `tests/integration_phase9c.rs`,
  `tests/integration_duumbi720_determinism.rs`, plus unit tests in
  `src/bench/report.rs` and `src/determinism/evidence.rs`.

Verified facts versus assumptions:

- Fact: bench depends on intent, not on determinism. Determinism already
  depends on bench helpers and report types.
- Fact: making benchmark call `run_replay` would pull ledger, agreement
  metrics, rewrite comparison, and CI thresholds into the regular bench
  path.
- Assumption: a shared isolated-attempt helper plus a structured execute
  outcome is the lowest-risk reuse path.
- Assumption: additive serde defaults remain the compatibility strategy
  unless a required field is introduced.

## Affected Areas

Expected Stage 10 source changes:

- `src/intent/execute.rs`
  - Add a structured execution outcome used by the shared attempt executor.
  - Keep `run_execute` and `run_execute_with_progress` as compatibility
    wrappers that still return `Result<bool>` and the same user-facing logs.
- New shared module, preferred path `src/intent/attempt.rs`, acceptable
  alternative `src/bench/attempt.rs` if that avoids unwanted intent-layer
  coupling to artifact layout. Both runners must call the same public
  contract.
- `src/bench/runner.rs`
  - Call the shared attempt executor instead of the private TempDir +
    `Ok(false)` log-replacement path.
  - Populate new result fields from typed outcome data.
- `src/bench/report.rs`
  - Additive result/report fields, taxonomy mapping, schema version.
- `src/determinism/runner.rs`
  - Call the same attempt executor; keep ledger, metrics, and rewrite
    comparison in the determinism runner.
  - Stop duplicating TempDir / `run_execute` / `retain_attempt_log` once
    the shared helper covers those concerns.
- `src/determinism/evidence.rs`
  - Additive attempt fields for phase evidence, repair telemetry, and
    root cause. Keep `v1` if additive; bump to `v2` only if a required
    field appears.
- `src/cli/mod.rs` and `src/main.rs`
  - Additive flags: benchmark `--artifact-dir`, `--keep-workspaces`,
    `--capture-model-io`; determinism `--capture-model-io`.
- Tests:
  - `src/intent/*` unit tests for outcome and taxonomy.
  - `src/bench/report.rs` and runner-focused tests.
  - `tests/integration_phase9c.rs`
  - `tests/integration_duumbi720_determinism.rs`
  - new focused integration test if that keeps coverage clearer, for
    example `tests/integration_duumbi779_attempt_evidence.rs`.
- Docs:
  - `docs/testing/phase9c-benchmark.md`
  - determinism replay docs if present
  - cleanup/retention notes; do not commit retained workspaces

Must not change during this spec PR:

- implementation code, tests, CI workflows, generated reports
- prompts, models, routing, retry policy, Op set
- Query mode, TUI, Studio
- #781 experiment assets

## Technical Approach

### 1. Shared Per-Attempt Execution And Evidence Contract

Do not make `duumbi benchmark` call `determinism::runner::run_replay`.

Split the work into two layers:

1. Inner structured execute outcome in `src/intent/execute.rs`.
2. Outer isolated-attempt executor used by bench and determinism.

Recommended inner type:

```rust
pub struct IntentExecutionOutcome {
    pub success: bool,
    pub first_pass_success: bool,
    pub repair_attempted: bool,
    pub repair_applied: bool,
    pub repair_success: Option<bool>,
    pub mutation_retry_count: Option<u32>,
    pub repair_retry_count: Option<u32>,
    pub tests_passed: usize,
    pub tests_total: usize,
    pub terminal_status: String,
    pub phase_events: Vec<ExecutionPhaseEvent>,
    pub diagnostics: Vec<CapturedDiagnostic>,
    pub dominant_error_code: Option<String>,
}
```

`ExecutionPhaseEvent` should record ordered phase names such as
`preflight`, `mutation`, `verify`, `repair`, `reverify`, and `complete`,
with status and optional error code. Repair attempted is true when a
`repair` event exists, not when a log line happens to survive.

Keep `run_execute` as:

```rust
pub async fn run_execute(...) -> Result<bool> {
    Ok(run_execute_structured(...).await?.success)
}
```

Existing CLI/REPL/workflow callers stay on the boolean wrapper.

Recommended outer request/result:

```rust
pub struct AttemptRequest<'a> {
    pub task_id: &'a str,
    pub spec: &'a IntentSpec,
    pub provider: &'a dyn LlmProvider,
    pub attempt: u32,
    pub artifact_dir: Option<&'a Path>,
    pub keep_workspace: bool,
    pub capture_model_io: bool,
}

pub struct AttemptEvidence {
    pub outcome: IntentExecutionOutcome,
    pub model_identity: ModelIdentity,
    pub hashes: AttemptHashes,
    pub sanitized_log: Vec<String>,
    pub artifact_paths: Vec<String>,
    pub phase_evidence: PhaseEvidence,
    pub root_cause: RootCauseClass,
    pub error_category: Option<ErrorCategory>,
}
```

The outer executor:

1. Creates `TempDir` and initializes the workspace through the existing
   `init_workspace` callback.
2. Saves the intent.
3. Captures initial graph hashes when a graph exists.
4. Calls structured execute.
5. Captures final hashes, intent status, validator/compiler/verifier
   summaries, and sanitized transcript.
6. Classifies `root_cause` from `phase_evidence`.
7. Copies bounded artifacts into `artifact_dir` using
   `safe_artifact_key`.
8. Optionally snapshots `.duumbi` when `keep_workspace` is true.
9. Optionally writes redacted model I/O when `capture_model_io` is true.
10. Drops `TempDir` only after copies succeed or a copy failure is recorded
    as `unknown` with phase evidence describing the copy error.

Determinism continues to wrap this helper with ledger events and agreement
metrics. Benchmark maps `AttemptEvidence` onto `BenchmarkResult`.

Rejected alternative: benchmark delegates to the full determinism runner.
That would couple regular eval JSON to replay ledgers, rewrite comparison,
and CI agreement thresholds, and it would invert the current
determinism-depends-on-bench direction.

### 2. Stop Lossy `Ok(false)` Log Replacement

Delete the `run_in_temp_workspace` pattern that turns `Ok(false)` into a
short `Err` string. The shared executor must return the structured outcome
on both success and unsuccessful-but-completed execute paths.

`Err` remains for infrastructure failures: tempdir creation, init, intent
save, or I/O. Those classify as `provider_or_infrastructure` or `unknown`
according to the rule table, not as `logic_error`.

### 3. CLI Flags

Additive flags only.

Benchmark:

```text
duumbi benchmark \
  --suite scaled --smoke --attempts 1 \
  --artifact-dir .duumbi/benchmark/attempts \
  --keep-workspaces \
  --capture-model-io \
  --output /tmp/duumbi-779-bench.json
```

Determinism replay already has `--artifact-dir` and `--keep-workspaces`.
Add `--capture-model-io` with the same semantics.

Defaults:

- `--artifact-dir` for benchmark: `.duumbi/benchmark/attempts`. Failed
  attempts always copy bounded sanitized evidence there (or to an override
  path) before `TempDir` drop. Success attempts may keep hashes in JSON and
  skip bulky file copies.
- `--keep-workspaces`: false
- `--capture-model-io`: false

Do not add a user-facing default-model flag.

### 4. Schema Versioning And Migration

Benchmark report:

- Add `schema_version` with default
  `duumbi.benchmark.report.v1` for current documents and
  `duumbi.benchmark.report.v2` once `root_cause`, `phase_evidence`, and
  artifact paths are present on results.
- Keep all current `BenchmarkResult` fields.
- New fields must be `#[serde(default)]` and skip-empty where that matches
  existing style.
- `load_baseline` must still parse reports that lack `schema_version`.

Replay report:

- Prefer additive fields on `ReplayAttempt` under existing
  `duumbi.determinism.replay_report.v1`.
- Bump to `v2` only if a currently required field is renamed or a new
  field is required without default.
- Read path must accept `v1`.

Compatibility consumers:

- `duumbi benchmark --baseline`
- `tests/integration_phase9c.rs`
- bench report unit tests
- determinism JSON/Markdown evidence
- any committed #689 JSON snapshots

Document that `error_category` counts may shift because unclassified
`Ok(false)` rows no longer become `logic_error`. That is a diagnostic
correction, not a kill-criterion change.

### 5. Deterministic Root-Cause Rules

Store `phase_evidence` separately from `root_cause`.

Recommended `phase_evidence` facts:

- preflight blocked
- mutation failed / retry exhausted
- diagnostic codes from validation or compilation
- SSA/dominance or backward-branch messages
- unresolved export/import/call (`E010`, missing exports)
- compiler/link/runtime (`E008`, cranelift, signal, write obj)
- verifier tests passed/failed vs expected
- process-evidence gap
- provider/auth/rate-limit/timeout
- repair entered/applied/succeeded

Ordered rules (first match wins):

1. Provider/auth/rate-limit/timeout/missing credentials ->
   `provider_or_infrastructure` / `ErrorCategory::ProviderError`
2. Process-evidence gap without execution ->
   `verifier_mismatch_or_unsupported_evidence` /
   `ErrorCategory::EvidenceRequired`
3. Schema or graph validation (`E009` and schema messages, excluding SSA
   rules below) -> `schema_graph_validation` /
   `ErrorCategory::SchemaError`
4. Cross-module export/import/call (`E010`, missing export, unresolved
   call) -> `cross_module_resolution` / `ErrorCategory::MutationFailed`
   unless a more specific existing category already applies; do not use
   `logic_error`
5. SSA dominance, forward reference, or backward-branch loop construction
   messages -> `control_flow_or_ssa` / `ErrorCategory::SchemaError` or
   `Crash` only if compilation actually crashed; prefer a dedicated mapping
   documented in tests
6. Compiler/linker/runtime crash -> `compiler_or_runtime` /
   `ErrorCategory::Crash`
7. Missing function or failed task decomposition before a graph exists ->
   `task_decomposition_or_missing_function` /
   `ErrorCategory::MutationFailed`
8. Verifier ran, graph compiled, tests failed with no diagnostic code ->
   `product_logic_mismatch` / `ErrorCategory::LogicError`
9. Verifier or process checker cannot judge the claimed behavior ->
   `verifier_mismatch_or_unsupported_evidence` /
   `ErrorCategory::EvidenceRequired`
10. Else -> `unknown` / `error_category` remains present as `None` or a
    new serialized value only if serde compatibility is preserved. Prefer
    keeping `ErrorCategory` unchanged and adding optional
    `error_category: None` plus `root_cause: unknown` rather than adding a
    new enum variant if that would break older readers. If a variant is
    required, add `Unknown` with `#[serde(other)]` or default so old
    writers still parse.

Do not implement an LLM or offline inference classifier in this issue. If a
later issue adds one, it must write `root_cause_inference` and leave
`root_cause` untouched.

### 6. Privacy, Redaction, And Retention Bounds

Default evidence: hashes and sanitized summaries only.

Reuse or extract the secret-line sanitizer from
`src/knowledge/learning.rs` rather than inventing a second policy. Redact
at least:

- `authorization`, `bearer`, `x-api-key`, `api_key`, `api-key`
- values of configured credential env vars, never the env var names
- common token prefixes if present in transcripts

`--capture-model-io` writes redacted files under:

```text
<artifact-dir>/<run-or-command>/<task-key>/<provider-key>/<attempt>/model-io/
```

Those files are local-only. Implementation must:

- add them to ignored paths if they would otherwise be created inside a
  git worktree
- never print raw payloads to stdout JSON used by CI summaries
- never attach them to GitHub workflow summaries
- never commit them in the implementation PR

Size caps (implementation may tune, but must test):

- sanitized `execute.log`: 256 KiB truncated
- copied workspace snapshot: skip `*.o`, binaries, and `target/`
- model I/O: 256 KiB per file truncated after redaction

### 7. Success-Path Compatibility

A first-pass success still serializes:

- `showcase` / `task_id`
- `provider`
- `attempt`
- `success: true`
- `tests_passed` / `tests_total`
- `duration_secs`
- `repair_attempted: false`
- existing usage and evidence optionals

No required field may be renamed. New fields on success may be omitted or
defaulted.

## Invariants

- Bench and determinism share one attempt executor; benchmark does not call
  the full replay runner.
- `TempDir` drop must not delete the only copy of attempt evidence.
- `repair_attempted` is typed from the execute outcome.
- `root_cause` is rule-based and may be `unknown`.
- `phase_evidence` is observed fact; inference belongs in a later labeled
  field, not here.
- Default evidence is sanitized/hash-based.
- `--capture-model-io` is local, redacted, off by default.
- Existing JSON required fields remain.
- Normal CI makes no live provider calls and uploads no workspaces or raw
  model I/O.
- Query mode stays read-only.
- Secrets never appear in reports, logs committed to git, or GitHub
  summaries.

## BDD-To-Test Mapping

| Product scenario | Automated, E2E, manual, or review evidence |
| --- | --- |
| Failed attempt retains evidence after TempDir lifecycle | Integration test builds a fixture provider that fails execute, runs the shared executor with an artifact dir, drops the TempDir (or lets it drop), then asserts `execute.log` or equivalent, graph hashes, and report `artifact_paths` still exist on disk. |
| Repair-entered failure reports `repair_attempted=true` | Unit or integration test feeds a structured outcome or mock execute path that emits a repair phase event and final failure; asserts `repair_attempted=true`, `first_pass_success=false`, `repair_success=false`, and that a replaced short error string is not required. Regression test covers the old `Ok(false)` log-loss case. |
| Taxonomy distinguishes listed failure classes | Unit tests for the rule table: one fixture or synthetic `phase_evidence` per required class, plus an unmatched fixture that yields `unknown`. Serialization test proves `root_cause` and `phase_evidence` both appear in JSON. |
| Opt-in model I/O stays local and redacted | Unit test with a payload containing `Authorization: Bearer secret` and an API key; with the flag, the written file is redacted and the JSON report has hashes only; without the flag, no model-io file is written. Review check: implementation PR contains no raw payloads. |
| Success path still compatible | Report unit test serializes a first-pass success and asserts required fields; `load_baseline` parses a fixture copied from current `BenchmarkReport` JSON without `schema_version` or `root_cause`. Determinism integration fixture still parses `replay_report.v1`. |

Additional technical tests:

- `--keep-workspaces` copies `.duumbi` and still sanitizes secrets.
- Path keys reject `..` and URL characters via `safe_artifact_key`.
- Provider/credential failures are not counted as `product_logic_mismatch`.
- Existing `tests/integration_phase9c.rs` baseline comparison still loads.

## Live E2E Plan

Canonical interface: CLI.

This issue is diagnostic plumbing. The required proof is local and
fixture-backed. Do not upload artifacts to GitHub or any cloud store.

Preferred verification, no external LLM:

```sh
cargo fmt --check
cargo test --test integration_duumbi779_attempt_evidence
cargo test --test integration_phase9c
cargo test --test integration_duumbi720_determinism
```

If the implementation PR also wants a live smoke and credentials exist,
keep it strictly local and one attempt:

```sh
repo="$(pwd)"
tmpdir="$(mktemp -d /tmp/duumbi-779-evidence.XXXXXX)"
"$repo/target/debug/duumbi" benchmark \
  --suite scaled \
  --smoke \
  --showcase scaled_math_pipeline \
  --provider minimax:auto:primary:MINIMAX_API_KEY \
  --attempts 1 \
  --artifact-dir "$tmpdir/attempts" \
  --output "$tmpdir/duumbi-779-bench.json"
```

Live constraints:

- Expected external LLM cost must stay under USD 1. If the estimate would
  exceed USD 1, skip live smoke and rely on fixtures.
- Inspect only local JSON and artifact paths.
- Do not pass `--capture-model-io` in any CI job.
- Do not copy `$tmpdir` into the repo or GitHub Actions summaries.
- TUI/Studio: no full E2E. Thin help-text smoke only if new flags appear
  in CLI help.

Pass/fail for optional live smoke:

- Command writes JSON.
- Failed or successful attempt has surviving artifact paths after process
  exit.
- Report contains `root_cause` and `repair_attempted`.
- No secrets in JSON.

## Ralph Cycle Protocol

Use Ralph cycles only during Stage 10, after Stage 9 approval. This spec PR
must not run them.

Each cycle must:

1. summarize current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop only if requirements are met, a blocker appears, the expected
   external LLM cost of the next cycle exceeds USD 1, or scope changes;
   iteration count is not a stop condition

Recommended conservative cycles:

- Cycle 1: Structured `IntentExecutionOutcome` in `execute.rs` plus unit
  tests. No live LLM.
- Cycle 2: Shared attempt executor, TempDir copy-out, artifact-path tests.
  No live LLM.
- Cycle 3: Wire bench runner; remove `Ok(false)` log loss; repair
  regression test. No live LLM.
- Cycle 4: Taxonomy rules and JSON schema versioning tests. No live LLM.
- Cycle 5: Determinism runner reuse, CLI flags, redaction tests, docs.
  Optional local live smoke only if estimated cost is under USD 1.

## Cycle Budget

- Default cycle size: one bounded implementation goal.
- Max files or modules per cycle: prefer 1-4 related Rust modules plus
  focused tests. Mechanical CLI/report plumbing may touch more if listed
  first.
- Expected command budget per cycle: targeted `cargo test` for changed
  modules; `cargo fmt --check`; clippy before implementation PR review.
- Human approval required only when:
  - expected external LLM cost of a cycle exceeds USD 1
  - scope expands into pass-rate work, #781, cloud upload, or prompt
    changes
  - raw model I/O would be stored by default
  - a product decision contradicts `PRODUCT.md`
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning does not trigger the
  gate.
- No autonomous batch cap.
- Conservative default: Stage 10 should complete with zero live provider
  calls. Live smoke is optional and local.

When to stop and ask for human guidance:

- Sharing an executor would require benchmark to depend on the full
  determinism runner.
- Evidence copy cannot survive `TempDir` drop without unbounded workspace
  retention.
- Redaction cannot be proven.
- Existing `--baseline` JSON cannot be read without a breaking change that
  this spec does not already authorize.

## Task Breakdown

1. Add structured execute outcome and keep boolean wrappers.
2. Add shared attempt executor with artifact copy-out and path safety.
3. Replace bench `run_in_temp_workspace` with the shared executor.
4. Add taxonomy rules and serialization tests.
5. Extend `BenchmarkResult` / `BenchmarkReport` with additive fields and
   schema version.
6. Reuse the executor from determinism replay without dropping ledger or
   metrics behavior.
7. Add CLI flags and help text.
8. Add `--capture-model-io` redaction tests.
9. Update integration tests and docs.
10. Optional local live smoke; never upload artifacts.

Independently executable slices: taxonomy unit tests, redaction unit
tests, and schema defaulting tests can start as soon as types exist.

## Verification Plan

Local automated checks:

```sh
cargo fmt --check
cargo test intent::attempt
cargo test intent::execute
cargo test bench::report
cargo test determinism
cargo test --test integration_phase9c
cargo test --test integration_duumbi720_determinism
```

Add the new integration test command once named. Before implementation PR
review:

```sh
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Manual/local checks:

- Help text shows new flags.
- Fixture failure leaves files under `--artifact-dir` after process exit.
- Old baseline JSON still loads.
- `--capture-model-io` redacts secrets and stays local.

Review artifacts for Stage 10:

- Commands and test output in the implementation PR.
- No retained workspaces or raw model I/O committed.
- Spec links use `Related to #779` only.

## Completion Criteria

Stage 10 is complete only when:

- Shared attempt executor is used by both runners.
- Failed attempts retain evidence after `TempDir` drop.
- Repair-entered failures report `repair_attempted=true`.
- Required `root_cause` classes plus `unknown` are tested.
- Default evidence is sanitized/hash-based; opt-in I/O is local and
  redacted.
- Existing JSON required fields still deserialize, including old baselines.
- Product BDD scenarios are mapped to passing tests or documented fixture
  evidence.
- Focused tests, fmt, and clippy pass.
- Issue #779 remains open until implementation merge and Stage 12 closure.

## Failure And Escalation

- If execute cannot expose repair as a typed field without a large
  refactor, still stop using log-string inference; escalate rather than
  ship another heuristic.
- If determinism reuse appears to require calling `run_replay` from bench,
  stop and request a spec amendment; that alternative is rejected here.
- If live smoke is the only remaining gap, skip it when credentials are
  missing or cost would exceed USD 1; fixtures are sufficient.
- If redaction misses a secret class discovered in tests, expand the
  sanitizer and re-test; do not ship `--capture-model-io` without the
  failing fixture turning green.
- If schema compatibility breaks `--baseline`, add defaults or a read
  migration; do not require users to regenerate all historical reports.

## Rollback / Compatibility Notes

- Boolean `run_execute` remains. Revert is possible by restoring callers
  without removing the structured path.
- JSON: new fields defaulted; removing them later must keep serde
  defaults.
- CLI: new flags only. Removing flags later is non-breaking if defaults
  restore current behavior.
- Taxonomy shift from catch-all `logic_error` to finer classes is
  intentional. Document it in benchmark docs so baseline category counts
  are not misread as product regressions.
- `--keep-workspaces` and `--capture-model-io` default off, so rollback of
  those flags does not change current default runs.

## Open Questions

None blocking.

Non-blocking Stage 10 choices:

- Exact file path of the shared executor module.
- Whether success attempts omit file copies and keep hashes in JSON only.
- Whether `ErrorCategory` gains `Unknown` or `unknown` is represented only
  on `root_cause`. Prefer representing `unknown` on `root_cause` and keep
  `error_category` optional to avoid breaking enum readers.

Owner input is not required for those choices. Request Owner input only if
implementation discovers that surviving evidence requires unbounded
workspace retention or default raw model I/O.
