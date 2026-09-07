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
- `src/agents/mod.rs` and provider implementations used by bench/replay
  (at least OpenAI and Anthropic): capture seam for optional raw response
  retention when `--capture-model-io` is on.
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
    pub retries_remaining: Option<u32>,
    pub tests_passed: usize,
    pub tests_total: usize,
    pub terminal_status: String,
    pub phase_events: Vec<ExecutionPhaseEvent>,
    pub diagnostics: Vec<CapturedDiagnostic>,
    pub dominant_error_code: Option<String>,
}

pub struct ExecutionPhaseEvent {
    pub phase: String,
    pub status: PhaseEventStatus, // terminal | recovered | informational
    pub error_code: Option<String>, // JSON name `error_code`, never `code`
}
```

`ExecutionPhaseEvent` records ordered phase names such as `preflight`,
`init`, `intent_save`, `mutation`, `verify`, `repair`, `reverify`,
`complete`, and `persist`. Repair attempted is true when a `repair` event
exists, not when a log line happens to survive. Recovered mutation/provider
errors keep `status: recovered` and are ignored by the classifier.

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
    pub root_cause: Option<RootCauseClass>,
    pub root_cause_attribution: Option<RootCauseAttribution>,
    pub error_category: Option<ErrorCategory>,
    pub evidence_persistence: EvidencePersistence,
    pub persistence_error: Option<String>,
    pub model_io_status: ModelIoStatus,
}

pub enum EvidencePersistence {
    Complete,
    Partial,
    Failed,
    JsonOnly,
}

pub enum RootCauseAttribution {
    MatchedRule,
    NoMatchingRule,
}

pub enum ModelIoStatus {
    NotRequested,
    Captured,
    Partial,
    Unavailable,
}
```

The outer executor:

1. Creates `TempDir` and initializes the workspace through the existing
   `init_workspace` callback.
2. Saves the intent.
3. Captures initial graph hashes when a graph exists.
4. Calls structured execute through a capturing provider decorator when
   `--capture-model-io` is on (see capture seam below).
5. Captures final hashes, intent status, validator/compiler/verifier
   summaries, and sanitized transcript.
6. Classifies `root_cause` from **terminal** `phase_evidence` only.
7. Sanitizes, then copies allowlisted artifacts into `artifact_dir` using
   `safe_artifact_key` and a unique `run-id`.
8. Optionally snapshots allowlisted `.duumbi` graph/intent files when
   `keep_workspace` is true. **Never** copy `model-io/`, `prompts/`, or
   `responses/` out of the workspace, including when `capture_model_io` is
   true.
9. Write redacted `model-io/` files only from the **current attempt**
   capture seam when `capture_model_io` is true and payloads were obtained.
   Do not populate `model-io/` by copying caches.
10. Runs a `finally` finalizer on **every** exit, including init/intent-save
    `Err` and infrastructure `Err` after repair began, then drops `TempDir`.

Copy failure is recorded as `evidence_persistence: failed|partial` and must
not rewrite an established execution `root_cause`. If no execution class
exists yet, classify from the infra/init facts; do not invent
`product_logic_mismatch`.

Determinism continues to wrap this helper with ledger events and agreement
metrics. Benchmark maps `AttemptEvidence` onto `BenchmarkResult`.

Rejected alternative: benchmark delegates to the full determinism runner.
That would couple regular eval JSON to replay ledgers, rewrite comparison,
and CI agreement thresholds, and it would invert the current
determinism-depends-on-bench direction.

### Capture Seam For `--capture-model-io`

Fact: `LlmProvider::call_with_tools*` returns `Vec<PatchOp>` and current
OpenAI/Anthropic implementations discard the raw HTTP body. Wrapping
`&dyn LlmProvider` cannot recover exact response payloads from parsed ops.

Required Stage 10 change, still inside this issue:

- Keep `call_with_tools` for existing callers.
- Add an object-safe capture method with a default that returns ops plus
  `raw_response: None` and `payload_status: Unavailable`.
- DUUMBI-constructed prompts are captured at the orchestrator (always
  available as request text).
- OpenAI and Anthropic (and any provider used by bench/replay) must retain
  the raw response body **only when capture is enabled**, then hand it to
  the redactor. Do not retain raw bodies by default.
- A capturing decorator records request text, optional raw response, and
  `model_io_status`.
- Tests must cover (a) a provider that exposes payloads and (b) one that
  does not.

Recommended record:

```rust
pub struct CapturedProviderCall {
    pub ops: Vec<PatchOp>,
    pub request_prompt: String,
    pub raw_response: Option<String>,
    pub payload_status: ModelIoStatus,
}
```

When payloads are unavailable, write no fake files and keep prompt hashes
`partial` if the final on-wire prompt is not exposed.

### 2. Stop Lossy `Ok(false)` Log Replacement And Finalize Every Err Path

Delete the `run_in_temp_workspace` pattern that turns `Ok(false)` into a
short `Err` string. The shared executor must return the structured outcome
on both success and unsuccessful-but-completed execute paths.

`Err` remains for infrastructure failures: tempdir creation, init, intent
save, or I/O. Those classify from terminal infra facts as
`provider_or_infrastructure` or `unknown` according to the rule table, not
as `logic_error`.

Every such `Err` still runs the evidence finalizer **before** `TempDir`
drop:

- init/intent-save failure: persist phase events `init`/`intent_save` as
  terminal, hashes if any, sanitized log if any
- execute `Err` after repair began: keep `repair_attempted=true`, persist
  partial log and events
- copy failure: keep execution `root_cause`; set
  `evidence_persistence=failed|partial`; write `--output` JSON
- JSON write failure: non-zero process exit, sanitized stderr, leave any
  already-copied attempt files on disk

`evidence_persistence` never overwrites `root_cause`. Default process exit
follows existing measurement behavior. `--ci` treats persistence `failed`
as infrastructure CI failure, not as a graph-failure kill.

Success attempts skip the failed-attempt allowlist (`execute.log` and
companions). They write files only for:

- `--keep-workspaces`: graph/intent snapshot
- `--capture-model-io` with current-attempt captured payloads: `model-io/`
  files (`evidence_persistence: complete`, `artifact_paths` lists those
  files)
- neither: `evidence_persistence: json_only`, empty `artifact_paths`

Success + capture + unavailable payloads + no keep-workspaces stays
`json_only` with `model_io_status: unavailable`.

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
  attempts copy the failed-attempt allowlist there before `TempDir` drop,
  or a `truncation.json` stub if the 32 MiB run budget is exhausted.
  Success attempts follow the success file exceptions in PRODUCT.md (JSON
  only; plus `model-io/` when capture obtained payloads; plus snapshot
  when `--keep-workspaces`).
- `--keep-workspaces`: false. When true, copy the graph/intent allowlist
  for success or failure. Exclude binaries, secrets, and **all**
  pre-existing prompt/response/`model-io` caches in every flag combination.
- `--capture-model-io`: false. When true, write redacted `model-io/` files
  from the current-attempt capture seam only, including on success, if
  payloads were obtained.

Flag matrix matches PRODUCT.md. Do not add a user-facing default-model
flag.

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

Locked serialization (do **not** add `ErrorCategory::Unknown`):

- Classified failures: map `root_cause` onto an existing snake_case
  `ErrorCategory` as in PRODUCT.md.
- Unknown failures: `"root_cause": "unknown"`,
  `"root_cause_attribution": "no_matching_rule"`, and
  `"error_category": null` with the key **present** (do not
  `skip_serializing_if` this null on failed attempts).
- Success: `root_cause` null/omitted, `error_category` omitted or null.

Bidirectional compatibility:

- New readers load historical baselines that lack `schema_version` and
  `root_cause`.
- Existing readers ignore additive fields and accept `error_category: null`
  because the field is already `Option<ErrorCategory>`.

Representative JSON lives in PRODUCT.md (`Success`, `Classified failure`,
`Unknown failure`). Tests must round-trip those three shapes.

### 5. Deterministic Root-Cause Rules

Store `phase_evidence` separately from `root_cause`. Classifier input is
**terminal** events only. Recovered events are stored and tested, then
ignored when selecting `root_cause`.

Do not emit `root_cause_confidence`. Use `root_cause_attribution`:
`matched_rule` or `no_matching_rule`.

Success: `root_cause = None`. Recovered errors on a successful attempt do
not create a failure class.

Recommended `phase_evidence` facts:

- preflight blocked
- mutation failed / retry exhausted / retries remaining
- diagnostic codes from validation or compilation
- SSA/dominance or backward-branch messages
- unresolved export/import/call (`E010`, missing exports)
- compiler/link/runtime (`E008`, cranelift, signal, write obj)
- verifier tests passed/failed vs expected, and whether the verifier ran
- process-evidence gap / unsupported
- provider/auth/rate-limit/timeout/missing credentials
- repair entered/applied/succeeded
- persist/copy errors (`evidence_persistence` only; not a graph class)

Ordered rules (first match on **terminal** evidence wins):

1. Provider/auth/rate-limit/timeout/missing credentials ->
   `provider_or_infrastructure` / `ErrorCategory::ProviderError`
2. Process-evidence gap without execution ->
   `verifier_mismatch_or_unsupported_evidence` /
   `ErrorCategory::EvidenceRequired`
3. Schema or graph validation (`E009` and schema messages, excluding SSA
   rules below) -> `schema_graph_validation` /
   `ErrorCategory::SchemaError`
4. Cross-module export/import/call (`E010`, missing export, unresolved
   call) -> `cross_module_resolution` /
   `ErrorCategory::MutationFailed` only. Do not remap to `logic_error`,
   `schema_error`, or any other coarse value.
5. SSA dominance, forward reference, or backward-branch loop construction
   messages -> `control_flow_or_ssa` / `ErrorCategory::SchemaError` only.
   A compiler/linker/runtime crash without those control-flow diagnostics
   is rule 6, not a remapping of this class to `Crash`.
6. Compiler/linker/runtime crash -> `compiler_or_runtime` /
   `ErrorCategory::Crash`
7. Missing function or failed task decomposition before a graph exists ->
   `task_decomposition_or_missing_function` /
   `ErrorCategory::MutationFailed`
8. Verifier or process checker cannot judge the claimed behavior
   (unsupported evidence, `broader_evidence_required`, verifier did not run
   applicable checks) -> `verifier_mismatch_or_unsupported_evidence` /
   `ErrorCategory::EvidenceRequired`
9. Verifier ran applicable checks with **positive applicability evidence**
   (compiled/runnable graph, `tests_total > 0` or process-evidence
   `passed`/`failed`, no unsupported marker), graph compiled, tests failed
   with no diagnostic code -> `product_logic_mismatch` /
   `ErrorCategory::LogicError`
10. Else -> `unknown` / `"error_category": null` /
    `root_cause_attribution: no_matching_rule`

Rule 8 is evaluated before rule 9. A compiled graph with unsupported
process evidence must not become `product_logic_mismatch`.

The PRODUCT JSON Compatibility table is the only coarse mapping. Tests
must not invent a second `error_category` for the same `root_cause`.
If terminal evidence includes both SSA diagnostics (rule 5) and a later
compile crash (rule 6), first-match keeps `control_flow_or_ssa` /
`schema_error`.

Overlapping-rule fixture: terminal E009 plus later verifier failure =>
`schema_graph_validation`. Recovered-error fixture: recovered provider
timeout plus terminal E009 => `schema_graph_validation`.

Missing credentials use rule 1 **and** are omitted from graph-failure
totals (see PRODUCT.md). Tests must assert the accounting exclusion, not
only the label.

Do not implement an LLM or offline inference classifier in this issue. If a
later issue adds one, it must write `root_cause_inference` and leave
`root_cause` untouched.

### 6. Privacy, Redaction, And Retention Bounds

Default evidence: hashes and sanitized summaries only. Sanitize **before**
persist.

Reuse or extract the secret-line sanitizer from
`src/knowledge/learning.rs` rather than inventing a second policy. Redact
at least:

- `authorization`, `bearer`, `x-api-key`, `api_key`, `api-key`
- values of configured credential env vars, never the env var names
- common token prefixes if present in transcripts

Allowlist (failed attempts, default flags):

- `execute.log`
- `phase_evidence.json`
- `intent-status.txt` (terminal intent status + validator/verifier summary)
- `summaries.json`
- `hashes.json` if not fully inlined

`--keep-workspaces` adds sanitized intent spec and graph JSON-LD. It must
not copy binaries, `target/`, credentials, or any pre-existing
`model-io/`, `prompts/`, or `responses/` caches, **including when**
`--capture-model-io` is on.

`--capture-model-io` writes redacted files under:

```text
<artifact-dir>/<run-id>/<task-key>/<provider-key>/<attempt>/model-io/
```

Those files are local-only. Implementation must:

- add them to ignored paths if they would otherwise be created inside a
  git worktree
- never print raw payloads to stdout JSON used by CI summaries
- never attach them to GitHub workflow summaries
- never commit them in the implementation PR
- skip writing files when `model_io_status` is `Unavailable`

Locked caps:

- sanitized `execute.log`: 256 KiB truncated
- model I/O: 256 KiB per file truncated after redaction
- per-attempt file count: 32 default, 64 with `--keep-workspaces`
- per-attempt total bytes: 1 MiB default, 8 MiB with `--keep-workspaces`
- per-run total: 32 MiB

Per-attempt overflow drops lowest-priority files inside that attempt and
writes truncation metadata.

When a later failed attempt would exceed the **per-run** 32 MiB budget:

- do not rewrite or delete earlier attempts' files
- still create the later attempt directory
- write at least `truncation.json` with `omission_reason:
  run_budget_exhausted`, omitted names, and bytes used vs cap
- inline hashes, intent status, validator/verifier summaries, and phase
  evidence in the parent report `run_retention` object
- set `evidence_persistence: partial` (not `failed`)
- `--ci` does not treat run-budget truncation as infrastructure failure

Truncation metadata is mandatory when dropping bytes or files:
`truncated`, `original_bytes`, `retained_bytes`, `omitted_files`.

Symlink/collision:

- Do not follow symlinks whose canonical target is outside the TempDir.
- Use `safe_artifact_key` for every path component.
- Unique `run-id` per invocation. Existing destination => persistence error
  or unique suffix; never overwrite or merge attempts.

Graph/intent content for autopsy requires `--keep-workspaces`. Hashes
remain the default and cannot reconstruct the graph.

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
- `root_cause` is rule-based, uses terminal evidence only, and may be
  `unknown` with `root_cause_attribution: no_matching_rule`.
- `phase_evidence` is observed fact, including recovered events. Inference
  belongs in a later labeled field, not here.
- `evidence_persistence` is independent of execution `root_cause`.
- Default evidence is sanitized/hash-based.
- `--capture-model-io` is local, redacted, off by default.
- Existing JSON required fields remain.
- Normal CI makes no live provider calls and uploads no workspaces or raw
  model I/O.
- Query mode stays read-only.
- Secrets never appear in reports, logs committed to git, or GitHub
  summaries.

## BDD-To-Test Mapping

Required execute fixture (not optional): a deterministic `LlmProvider` that
drives **actual** `run_execute` / structured execute through the verifier
repair cycle into **both** `duumbi benchmark` and
`duumbi determinism replay` reports. Injecting a pre-built
`IntentExecutionOutcome` is allowed as an extra unit test, but it does **not**
satisfy the product repair or retention scenarios.

| Product scenario | Automated, E2E, manual, or review evidence |
| --- | --- |
| Failed attempt retains evidence after TempDir lifecycle | Integration test uses the execute fixture, fails after execute, drops TempDir, asserts surviving `execute.log`, graph hashes, `intent-status.txt` (or equivalent) with terminal intent status, validator/verifier summaries, and report `artifact_paths`. Default flags, no `--keep-workspaces`. |
| Repair-entered failure / no patch | Execute fixture enters repair, applies no patch, fails. Both runner reports: `repair_attempted=true`, `repair_applied=false`, `first_pass_success=false`, `repair_success=false`. Does not scrape replaced short error strings. |
| Repair applies a patch and still fails | Execute fixture writes a repair patch then fails verification. Assert `repair_applied=true`, `repair_success=false`. |
| Repair converts the attempt to success | Execute fixture fails first pass, repair patch, verifier pass. Assert `success=true`, `repair_success=true`, `root_cause` null. |
| Failure after repair entry with retries remaining | Execute fixture enters repair with remaining retry budget and ends unsuccessful. Assert retries remaining in phase evidence and `repair_attempted=true`. |
| Taxonomy listed classes | Unit tests of the rule table **and** execute-driven fixtures for each required class plus unmatched `unknown`. Serialization test covers the three JSON shapes in PRODUCT.md. |
| Recovered provider error | Fixture recovers a timeout then fails E009. Assert recovered event present and `root_cause=schema_graph_validation`. |
| Overlapping terminal diagnostics | Fixture with terminal E009 plus later test failure. Assert schema class wins. |
| Product logic requires verifier applicability | Fixture compiles but process evidence is unsupported. Assert `verifier_mismatch_or_unsupported_evidence`, not `product_logic_mismatch`. |
| Success path still compatible | First-pass success serialize required fields; `root_cause` null; no bulky files without flags; `load_baseline` parses current `BenchmarkReport` JSON without `schema_version`. Replay `v1` still parses. |
| Opt-in model I/O | Payload with `Authorization: Bearer secret`. Flag on + exposing provider: redacted files from the current-attempt seam, JSON hashes only in the report body. Flag off: no model-io files. Non-exposing provider: `model_io_status=unavailable`, no fake bodies. |
| Success + capture, no keep-workspaces | First-pass success with capture payloads: `model-io/` files exist; no `execute.log`/snapshot; `artifact_paths` lists only those files; `evidence_persistence=complete`. |
| Capture × keep-workspaces matrix | All flag×outcome rows in PRODUCT. Snapshots **never** copy prior `model-io/`/`prompts/`/`responses/`, including when capture is on. |
| Default failed retention without flags | Assert allowlisted files including intent status and validator/verifier summaries; no graph snapshot; no model-io. |
| Persistence failure | After execution class is `schema_graph_validation`, inject copy failure. Assert `root_cause` unchanged and `evidence_persistence` failed/partial; JSON report still written. |
| Run-budget exhaustion | Multiple oversized failed attempts. Later attempts have `truncation.json`; parent report inlines hashes/intent/summaries/phase evidence; `partial`; not a `--ci` infra failure; earlier files kept. |
| Infra Err after repair began | Fixture enters repair then returns execute `Err`. Assert `repair_attempted=true` and partial evidence survives. |
| Missing credentials excluded | Provider route with absent credentials. Assert `provider_or_infrastructure` **and** graph-failure totals/histograms/denominators do not increment. Relabel-only implementations fail this test. |
| Symlink escape | Workspace symlink pointing outside TempDir is omitted; truncation metadata lists it; outside files unchanged. |

Additional technical tests:

- `--keep-workspaces` copies sanitized graph/intent and still sanitizes secrets.
- Path keys reject `..` and URL characters via `safe_artifact_key`.
- Run-id collision does not overwrite an existing attempt directory.
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
- Failed attempts have surviving allowlisted artifact files after process
  exit, including intent status and validator/verifier summaries.
- Successful attempts have hashes in JSON; bulky workspace files are
  absent unless `--keep-workspaces` was passed. `--capture-model-io` is
  not used in this live smoke.
- Report contains `root_cause` (null on success) and `repair_attempted`.
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
- Cycle 3: Wire bench runner; remove `Ok(false)` log loss; execute-through-
  repair fixture (not a pre-injected outcome). No live LLM.
- Cycle 4: Taxonomy rules and JSON schema versioning tests. No live LLM.
- Cycle 5: Determinism runner reuse, CLI flags, capture seam, redaction
  tests, docs. Optional local live smoke only if estimated cost is under
  USD 1.

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
8. Add `--capture-model-io` capture seam and redaction tests, including
   non-exposing providers and keep-workspaces exclusion of prior payloads.
9. Add the execute-through-repair fixture covering both runners, plus
   persistence-failure, credential-exclusion, symlink, and taxonomy edge
   tests.
10. Update integration tests and docs.
11. Optional local live smoke; never upload artifacts.

Independently executable slices: taxonomy unit tests, redaction unit
tests, and schema defaulting tests can start as soon as types exist. They
do not replace the execute-through-repair fixture required for both
runners.

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
- Failed attempts retain allowlisted evidence after `TempDir` drop,
  including intent status and validator/verifier summaries.
- A deterministic provider fixture drives actual execute through repair
  into both runner reports.
- Repair-entered failures report `repair_attempted=true`; no-patch, patched
  failure, repaired success, and infra-Err-after-repair are tested.
- Required `root_cause` classes plus `unknown` are tested, including
  recovered-error and overlapping-rule fixtures. Verifier-applicability
  gating is tested.
- Default evidence is sanitized/hash-based; opt-in I/O is local and
  redacted; keep-workspaces cannot smuggle uncaptured payloads.
- Existing JSON required fields still deserialize, including old baselines.
  Unknown failures serialize `"error_category": null`.
- Missing credentials are excluded from graph-failure accounting.
- Persistence failure keeps execution `root_cause`.
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

- Boolean `run_execute` remains. **Unreleased revert** (before a release
  ships this work): restore prior callers to the boolean path and drop the
  new helper if needed. That is a source revert, not a user-facing break.
- JSON: new fields defaulted; removing them later must keep serde
  defaults.
- CLI: new flags only. While they remain unreleased, they can be deleted
  without a compatibility promise. **After a release ships**
  `--keep-workspaces` (on benchmark) or `--capture-model-io`, withdrawing
  those flags is a breaking CLI change for scripts that pass them.
  Defaults-off means *not passing the flag* preserves current behavior; it
  does **not** mean a later flag removal is non-breaking.
- Taxonomy shift from catch-all `logic_error` to finer classes is
  intentional. Document it in benchmark docs so baseline category counts
  are not misread as product regressions.
- `--keep-workspaces` and `--capture-model-io` default off, so enabling
  them is opt-in on current default runs.

## Open Questions

None blocking.

Non-blocking Stage 10 choices:

- Exact file path of the shared executor module.

Locked (not Stage 10 choices):

- Success without capture or keep-workspaces is `json_only`. Success +
  `--capture-model-io` writes current-attempt `model-io/` when payloads
  exist. `--keep-workspaces` adds the graph/intent snapshot only.
- Workspace snapshots never copy pre-existing payload caches.
- Failed attempts persist the allowlist, or `truncation.json` plus inlined
  report fields when the 32 MiB run budget is exhausted (`partial`).
- `ErrorCategory` does **not** gain `Unknown`. Unknown failures serialize
  `"error_category": null` with `"root_cause": "unknown"`.
  `control_flow_or_ssa` → `schema_error`; `cross_module_resolution` →
  `mutation_failed` with no test-local remapping.
- Retention caps, allowlist, capture seam, terminal vs recovered taxonomy,
  and persistence-vs-execution split are specified above.

Owner input is not required for the remaining module-path choice. Request
Owner input only if implementation discovers that surviving evidence
requires unbounded workspace retention or default raw model I/O.
