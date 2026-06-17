# DUUMBI-720: Determinism Program For AI Development - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-720/PRODUCT.md` by adding a
CLI-first determinism replay harness for provider-backed DUUMBI intent
execution.

The implementation must prove this flow:

```text
selected benchmark or intent -> locked replay context -> isolated attempts ->
attempt graph/evidence capture -> equivalence metrics -> append-only ledger +
summary report
```

This issue implements the first measurable determinism slice. It does not
replace provider mutation with rewrite-rule mutation, add a Studio dashboard,
add formal verification, or change default `duumbi add`, `duumbi intent
execute`, `duumbi benchmark`, TUI, or Studio behavior.

## Agent Audience

- Codex App implementation agents running bounded Ralph cycles.
- Codex CLI or Codex Cloud agents used for focused Rust implementation,
  testing, and review evidence.
- Stage 10 implementation coordinators.
- Stage 11 review agents validating evidence against this spec.
- Human maintainers reviewing replay metrics and live E2E evidence.

## Source Context

- Product spec: `specs/DUUMBI-720/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/735
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/720#issuecomment-4728066606
- Stage 7 AI gate decision:
  https://github.com/hgahub/duumbi/issues/720#issuecomment-4728038262
- GitHub issue: https://github.com/hgahub/duumbi/issues/720
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/720#issuecomment-4727795690
- Stage 6 product spec draft comment:
  https://github.com/hgahub/duumbi/issues/720#issuecomment-4728024889
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant code verified for Stage 8:

- `src/main.rs`
  - Owns CLI dispatch and already handles `Commands::Benchmark`.
  - `run_benchmark()` loads effective providers, applies suite/showcase/provider
    filters, runs the benchmark runner, writes JSON reports, and handles CI
    exit behavior.
- `src/cli/mod.rs`
  - Defines `Commands`, `BenchmarkSuiteArg`, and existing benchmark flags:
    `--suite`, `--smoke`, `--showcase`, `--provider`, `--attempts`, `--output`,
    `--ci`, and `--baseline`.
- `src/bench/showcases.rs`
  - Defines `ShowcaseSuite`, `ShowcaseVerification`, core showcases, scaled
    showcases, smoke flags, tags, process-evidence gaps, and filtering helpers.
  - `scaled_smoke_filter_includes_process_evidence_case` proves the scaled
    smoke subset includes `scaled_http_sqlite_json`.
- `src/bench/runner.rs`
  - Runs provider-backed benchmark attempts in isolated `TempDir` workspaces.
  - It saves an intent as `benchmark-showcase`, calls
    `intent::execute::run_execute()`, loads the final or archived intent, and
    returns `BenchmarkResult`.
  - `run_in_temp_workspace()` and `provider_name()` are currently private.
- `src/bench/report.rs`
  - Defines `BenchmarkResult`, `BenchmarkReport`, `BenchmarkSummary`,
    `ProviderUsageSummary`, `BenchmarkEvidence`, and `ErrorCategory`.
  - It already records first-pass success, repair attempts, failure category,
    dominant error code, usage availability, and cost when available.
- `src/intent/execute.rs`
  - Runs preflight with BDD, renders BDD prompt context, loads effective agent
    policy, saves a rollback snapshot, decomposes intent tasks, assembles
    context, builds task prompts, calls the provider mutation orchestrator, and
    verifies test cases.
  - Existing prompt/context details are not emitted as structured replay
    evidence.
- `src/intent/bdd.rs`
  - Supports BDD readiness, parsed feature files, scenario coverage
    classification, rendered BDD reports, and bounded BDD prompt context.
- `src/intent/spec.rs`
  - Defines `IntentSpec`, `IntentBdd`, `ExecutionMeta`, and task/test data.
- `src/context/mod.rs`
  - Assembles deterministic context bundles and has tests proving stable output
    for identical inputs.
- `src/hash.rs`
  - Provides `semantic_hash()` for graph directories and
    `semantic_hash_value()` for in-memory JSON-LD values.
  - Semantic hashes intentionally ignore node `@id` values.
- `src/rewrite/*`
  - Provides deterministic rewrite rule summaries, preview/apply evidence,
    stable match IDs, safety classes, validation evidence, and cost evidence.
  - It is provider-free and is not yet an end-to-end constrained LLM mutation
    strategy.
- `src/properties/evidence.rs`
  - Provides a schema-versioned JSON evidence precedent.
- `src/telemetry/mod.rs`
  - Provides schema-versioned local artifact precedent, deterministic IDs, path
    helpers, and explicit evidence types.
- `src/lib.rs`
  - Exports library modules for integration tests and external tooling.

Relevant tests and evidence verified for Stage 8:

- `tests/integration_phase10_context.rs`
  - `assemble_context_deterministic()` proves context assembly stability.
- `tests/integration_phase9c.rs`
  - Covers benchmark reports, regression detection, and runner infrastructure.
- `tests/integration_duumbi684_rewrite.rs`
  - Covers rewrite list, preview, apply, safety, validation, and MCP exposure.
- `docs/e2e/results/duumbi-689-scaled-smoke-20260616.md`
  - Records scaled smoke risk evidence: 0/3 successes and provider usage
    unavailable for all selected rows.
- `docs/e2e/duumbi-689-known-limitations.md`
  - Documents scaled smoke limitations and failure patterns.

Relevant Obsidian notes:

- `Duumbi/05 Archive/Processed Inbox/2026-06-12 - Determinism Program for AI Development.md`
- `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Future Development Roadmap Map.md`
- `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`

Verified source facts:

- Existing benchmark attempts already run in isolated temp workspaces, but
  final graph artifacts are dropped before cross-attempt comparison.
- Existing benchmark reports do not record graph exact hashes, semantic hashes,
  prompt/context hashes, registry state hashes, or replay equivalence metrics.
- Existing BDD evidence is available to the intent execution path but not
  retained in benchmark reports as a replay equivalence tier.
- Existing provider identifiers include provider kind, role, credential env name,
  and optional model/base URL, but they intentionally do not include credential
  values.
- Existing rewrite evidence can be listed and compared as metadata, but it
  should not be represented as a full LLM mutation strategy.

Assumptions for implementation:

- The first implementation should reuse benchmark and intent code rather than
  adding a separate benchmark execution stack.
- The replay harness may initially target benchmark showcases only if persisted
  arbitrary intents would require unsafe or unclear path semantics. Persisted
  intent replay can be added in the same issue only if it stays inside the
  approved scope and tests remain focused.
- Prompt/context hashing should be captured at the narrowest stable seam that
  reflects the actual prompts used by mutation. If the exact final provider
  prompt cannot be captured safely in the first cycle, record a deterministic
  `context_pack_hash` from intent spec, BDD prompt context, coordinator tasks,
  benchmark guidance, and assembled context metadata, and label it precisely.

## Affected Areas

Expected implementation changes:

- CLI command definitions:
  - `src/cli/mod.rs`
- CLI dispatch:
  - `src/main.rs`
- New determinism/replay domain module:
  - `src/determinism/mod.rs`
  - optional `src/determinism/runner.rs`
  - optional `src/determinism/evidence.rs`
  - optional `src/determinism/metrics.rs`
  - optional `src/determinism/digest.rs`
  - optional `src/determinism/report.rs`
  - optional `src/determinism/markdown.rs`
- Library exports:
  - `src/lib.rs`
  - `src/main.rs` module declaration
- Benchmark support:
  - `src/bench/runner.rs`
  - `src/bench/report.rs`
  - `src/bench/showcases.rs`
- Intent execution evidence seam:
  - `src/intent/execute.rs`
  - `src/intent/coordinator.rs` only if a stable context-pack helper needs
    access to decomposition details
  - `src/intent/bdd.rs` only if report rendering needs a small public helper
- Hashing and artifact helpers:
  - `src/hash.rs` only if a reusable exact graph digest belongs there; otherwise
    keep exact replay digests in `src/determinism/digest.rs`
- Tests:
  - new focused unit tests in `src/determinism/*`
  - new integration test such as `tests/integration_duumbi720_determinism.rs`
  - focused additions to `tests/integration_phase9c.rs` only if benchmark
    runner behavior is refactored
- Evidence docs:
  - new live E2E evidence under `docs/e2e/results/` only when captured during
    implementation
  - do not commit transient replay workspaces or raw provider output

Areas expected not to change:

- `specs/DUUMBI-720/PRODUCT.md`
- provider credential setup flows
- default `duumbi add`
- default `duumbi intent execute`
- default `duumbi benchmark`
- Query mode mutation rules
- TUI startup provider warning behavior
- Studio UI
- Cranelift lowering or runtime code
- registry network client behavior
- rewrite rule semantics

CI and local validation paths:

- `cargo fmt --check`
- Focused unit tests for new determinism modules.
- Focused integration test for replay CLI and artifacts.
- Existing benchmark/report tests touched by refactors.
- `cargo clippy --all-targets -- -D warnings` before implementation PR review.
- `cargo test --all` before implementation PR review when shared benchmark or
  intent execution behavior changes substantially.

## Technical Approach

### 1. Add A CLI-First Determinism Surface

Add a top-level command:

```text
duumbi determinism replay [options]
```

Recommended CLI shape:

```text
duumbi determinism replay \
  --suite scaled \
  --smoke \
  --showcase scaled_math_pipeline \
  --provider minimax:auto:primary:MINIMAX_API_KEY \
  --attempts 2 \
  --output docs/e2e/results/duumbi-720-replay.json \
  --artifact-dir .duumbi/determinism/replays \
  --markdown-output docs/e2e/results/duumbi-720-replay.md
```

Recommended options:

- `--suite <core|scaled>`: same semantics as benchmark suite filtering.
- `--smoke`: same low-budget subset semantics as benchmark.
- `--showcase <name>[,<name>]`: same showcase filtering semantics as benchmark.
- `--provider <name>[,<name>]`: reuse benchmark provider filtering.
- `--attempts <N>`: default `2`, minimum `2`. The default should be low-cost
  because replay multiplies provider calls.
- `--output <path>`: write summary JSON. If omitted, print JSON to stdout.
- `--artifact-dir <path>`: root for run bundles. Default
  `.duumbi/determinism/replays`.
- `--markdown-output <path>`: optional human-readable report.
- `--ci`: return non-zero when configured thresholds fail.
- `--min-exact-agreement <0.0..1.0>`: optional CI threshold.
- `--min-semantic-agreement <0.0..1.0>`: optional CI threshold.
- `--min-behavioral-agreement <0.0..1.0>`: optional CI threshold.
- `--keep-workspaces`: optional debug flag retaining isolated attempt
  workspaces under the run bundle. Default is false.

Avoid a hidden or experimental flag. This is a user-visible measurement surface.
If the final command name differs, update this technical spec in the
implementation PR or explain the deviation in PR evidence.

### 2. Add Schema-Versioned Replay Evidence

Add a new module with evidence types and schema versions.

Recommended constants:

```rust
pub const REPLAY_REPORT_SCHEMA_VERSION: &str = "duumbi.determinism.replay_report.v1";
pub const REPLAY_LEDGER_SCHEMA_VERSION: &str = "duumbi.determinism.ledger_event.v1";
```

Recommended report shape:

```json
{
  "schema_version": "duumbi.determinism.replay_report.v1",
  "run_id": "duumbi-720-20260617T091500Z-scaled-smoke",
  "started_at": "...",
  "finished_at": "...",
  "duumbi_version": "0.4.0-preview",
  "source_commit": "...",
  "inputs": {
    "suite": "scaled",
    "smoke": true,
    "showcases": ["scaled_math_pipeline"],
    "providers": ["minimax:auto:primary:MINIMAX_API_KEY"],
    "attempts": 2
  },
  "environment": {
    "provider_source": "user",
    "registry_state_hash": "...",
    "lockfile_hash": "..."
  },
  "tasks": [],
  "attempts": [],
  "metrics": {},
  "rewrite_comparison": {
    "status": "not_yet_comparable",
    "reason": "no constrained LLM rewrite mutation strategy for this task"
  },
  "warnings": []
}
```

Recommended attempt fields:

- `task_id`
- `suite`
- `tags`
- `provider`
- `attempt`
- `workspace_strategy`
- `initial_graph_exact_hash`
- `initial_graph_semantic_hash`
- `final_graph_exact_hash`
- `final_graph_semantic_hash`
- `intent_spec_hash`
- `bdd_context_hash`
- `context_pack_hash`
- `prompt_hashes`
- `success`
- `tests_passed`
- `tests_total`
- `bdd_readiness`
- `bdd_coverage`
- `behavior_signature`
- `error_category`
- `dominant_error_code`
- `provider_usage`
- `artifact_paths`
- `duration_secs`

Keep raw prompts out of the report by default. If raw prompt retention is ever
added for local debugging, it must be opt-in, clearly labeled, bounded, and
excluded from committed evidence.

### 3. Add An Append-Only Ledger

For every run, create:

```text
.duumbi/determinism/replays/<run-id>/
  ledger.jsonl
  report.json
  report.md                 # optional
  attempts/<task>/<provider>/<attempt>/...
```

Ledger events should be append-only JSON lines:

- `run_started`
- `task_selected`
- `attempt_started`
- `context_locked`
- `attempt_completed`
- `attempt_failed`
- `run_completed`
- `run_interrupted`

Each ledger event must include:

- `schema_version`
- `run_id`
- `event`
- `sequence`
- `timestamp`
- `task_id` when applicable
- `provider` when applicable
- `attempt` when applicable
- `payload`

Use `OpenOptions::append(true).create(true)` and flush after event writes that
would be useful after interruption. Keep tests deterministic by separating
sequence and schema assertions from timestamp values.

### 4. Reuse Benchmark Selection And Provider Routing

The replay command should reuse:

- `ShowcaseSuite`
- `filter_showcases_with_options`
- `parse_showcase`
- provider loading from `config::load_effective_config()`
- provider filtering semantics from benchmark runner
- benchmark failure categories and usage summary types where practical

Do not duplicate provider selection logic. If `bench::runner::provider_name()`
needs to be shared, expose it as `pub(crate)` or move it to a small shared helper
with tests preserving existing output.

### 5. Own Attempt Workspaces Long Enough To Capture Evidence

Do not call existing `bench::runner::run_benchmark()` and try to infer
determinism from its returned `BenchmarkResult` alone; that loses the attempt
workspace before graph digests can be captured.

Preferred implementation:

1. Add `src/determinism/runner.rs`.
2. Follow the benchmark runner pattern:
   - create one isolated `TempDir` or artifact workspace per attempt
   - call `init_workspace`
   - save the selected `IntentSpec`
   - run `intent::execute::run_execute*`
   - load active or archived final intent
   - compute graph digests before dropping the workspace
   - optionally copy bounded artifacts into the replay bundle
3. Keep provider attempts sequential per provider by default to reduce rate
   limit and cost variance. Parallel provider support can be added later.

If code duplication with `bench::runner` becomes meaningful, extract a shared
internal attempt executor, but do not redesign the benchmark runner broadly.

### 6. Compute Exact, Semantic, And Behavior Digests Separately

Add replay-specific exact graph digesting:

- collect all `.jsonld` files under `.duumbi/graph`
- sort paths by workspace-relative path
- hash each relative path plus exact file bytes
- include file boundaries in the hash input to avoid ambiguity
- produce lowercase SHA-256 hex

Use `hash::semantic_hash(graph_dir)` for semantic graph identity. Do not reuse
semantic hash as an exact digest because it intentionally ignores `@id`.

Recommended behavior signature input:

- `success`
- `tests_passed`
- `tests_total`
- `IntentStatus`
- BDD readiness label when available
- BDD scenario coverage classifications when available
- process-evidence status when applicable
- error category and dominant error code when failing

Hash a canonical JSON representation of this behavior signature for grouping,
but retain the readable fields in the report.

### 7. Record Context And Prompt Hashes Safely

The product spec requires prompt/context hash evidence. Implement this without
leaking secrets.

Preferred approach:

- Add a small structured evidence sink or callback to `intent::execute` that can
  receive:
  - BDD prompt context hash
  - coordinator task list hash
  - context bundle hash per task
  - final mutation prompt hash per task
  - provider name
- Hash prompt/context text with SHA-256 and immediately discard the raw text.
- Keep the existing `run_execute()` public behavior unchanged by making the
  sink optional and defaulting to no-op.

If the first implementation cannot safely capture final mutation prompt hashes
without invasive refactoring, record:

- `prompt_hashes.status = "partial"`
- `prompt_hashes.reason = "final provider prompt hash not exposed by execution seam"`
- a `context_pack_hash` derived from intent spec, BDD prompt context,
  coordinator tasks, benchmark guidance availability, and context bundle
  metadata

Do not silently label partial context hashing as full prompt hashing.

### 8. Record Registry And Lockfile State

Recommended state hash inputs:

- `.duumbi/deps.lock` if present
- `.duumbi/config.toml` dependency and registry sections if present
- `vendor/` manifest files or module manifests if cheap and deterministic
- absence markers for missing files

Keep the first implementation bounded:

- `lockfile_hash`: hash `.duumbi/deps.lock` or `absent`.
- `workspace_dependency_config_hash`: hash dependency and registry config or
  `absent`.
- `registry_state_hash`: combine the two fields above and report limitations.

Do not query remote registries just to compute replay state.

### 9. Report Equivalence Metrics

For each task/provider group, compute:

- `attempts_total`
- `attempts_completed`
- `exact_graph_agreement_rate`
- `semantic_graph_agreement_rate`
- `behavioral_agreement_rate`
- `failure_category_agreement_rate`
- dominant exact hash group
- dominant semantic hash group
- dominant behavior signature group
- divergence examples with attempt numbers

Use largest-group agreement:

```text
agreement_rate = largest_equivalence_group_count / comparable_attempt_count
```

This is easier to interpret than pairwise agreement for the first report. If a
future implementation needs pairwise agreement, add it as a separate metric.

Comparable attempt rules:

- Completed attempts with final graph evidence are comparable for exact and
  semantic graph agreement.
- Attempts with verifier, BDD, process-evidence, or failure-category evidence
  are comparable for behavioral agreement.
- Provider setup failures before mutation are not graph-comparable but are
  included in failure metrics.

### 10. Integrate Rewrite Evidence Conservatively

Add a `rewrite_comparison` section with one of:

- `not_applicable`
- `not_yet_comparable`
- `metadata_only`
- `comparable`

For this issue, `not_yet_comparable` is expected for provider-backed
intent-generation tasks unless Stage 10 adds a real constrained rewrite mutation
strategy. The implementation may include current rewrite catalog metadata, such
as apply-capable rule counts and safety-class counts, but it must not claim a
determinism improvement without same-task comparative evidence.

### 11. Add Markdown Summary Rendering

Implement a compact Markdown summary for release evidence:

- title, run metadata, command, provider route, attempt count
- aggregate agreement metrics
- per-task/provider table
- divergence examples
- provider usage availability
- BDD/process-evidence gaps
- rewrite comparison status
- limitations and follow-ups

Keep Markdown generation deterministic except timestamps and run id.

## Invariants

- Replay attempts must not mutate the caller's active `.duumbi/graph`.
- Default benchmark and intent command behavior must remain unchanged.
- Exact graph digest and semantic hash are separate concepts.
- Semantic hash cannot be used for trace IDs, exact graph identity, or raw
  artifact identity.
- Missing provider usage must be recorded as unavailable with a stable reason.
- Raw credential values must never be serialized.
- Report rows and ledger-derived summaries must use stable ordering.
- A successful replay measurement can contain failed or divergent attempts.
- CI mode is the only mode where agreement thresholds determine the process
  exit code.
- Rewrite comparison must fail closed to `not_yet_comparable` rather than
  overstating determinism improvements.

## BDD-To-Test Mapping

| Product BDD Scenario | Verification Evidence |
| --- | --- |
| Run a low-cost replay for one benchmark showcase | Integration test runs `duumbi determinism replay --suite core --showcase calculator --provider <mock-provider> --attempts 2 --output <tmp>/report.json --artifact-dir <tmp>/replays`; asserts two attempt records, locked metadata fields, summary metrics, and schema version. If mock-provider plumbing is unavailable, use a deterministic in-process provider fixture at the replay runner layer and one CLI test for argument/output validation. |
| Preserve the user's active graph during replay | Integration test initializes a workspace, hashes `.duumbi/graph` before replay, runs replay with artifact/temp workspaces, hashes active graph after replay, and asserts equality. |
| Final graphs differ but semantic hashes match | Unit test builds synthetic attempt records with different exact graph hashes and identical semantic hashes; asserts exact agreement below 1.0, semantic agreement 1.0, and structural divergence classification. |
| Semantic hashes differ but verifier or BDD evidence matches | Unit test builds synthetic attempt records with different semantic hashes and identical behavior signatures; asserts semantic mismatch remains visible and behavioral agreement is 1.0. |
| Provider usage is unavailable | Unit test and integration report fixture assert `ProviderUsageSummary::unavailable(<reason>)` is preserved in attempt records, aggregate counts, and Markdown summary without estimated cost. |
| Scaled smoke replay includes broader-evidence tasks | Unit test or integration test targets `scaled_http_sqlite_json` selection with process-evidence verification and asserts behavior status is broader-evidence-required unless process evidence is actually produced. |
| Rewrite comparison is not yet comparable | Unit test asserts default provider-backed replay report emits `rewrite_comparison.status = "not_yet_comparable"` with no improvement claim. |
| CI gating is explicitly requested | Integration test runs replay with `--ci --min-semantic-agreement 1.0` against a deterministic fixture that produces divergent semantic hashes; asserts non-zero exit and retained report bundle. Add a positive CI threshold test where the threshold passes. |

## Live E2E Plan

Canonical interface: CLI.

Provider path:

- Use an already configured low-cost provider route. Preferred smoke route when
  available: `minimax:auto:primary:MINIMAX_API_KEY`.
- Required credential: the provider's configured API key env var, such as
  `MINIMAX_API_KEY`.
- Do not add a new provider setup flow for this issue.

Expected external LLM usage:

- One selected smoke showcase.
- Two replay attempts.
- Current intent execution may call the provider once per decomposed task plus
  retries and repair attempts. Expected range for the smoke E2E is 2-12 external
  provider calls depending on selected showcase and failures.
- Expected external provider cost should be below USD 1. If the planned live E2E
  provider, showcase, or attempt count would exceed USD 1, stop and request
  human approval before running it.

Recommended command after implementation:

```bash
repo="$(pwd)"
tmpdir="$(mktemp -d /tmp/duumbi-720-replay.XXXXXX)"
"$repo/target/debug/duumbi" determinism replay \
  --suite core \
  --showcase calculator \
  --provider minimax:auto:primary:MINIMAX_API_KEY \
  --attempts 2 \
  --artifact-dir "$tmpdir/replays" \
  --output "$tmpdir/duumbi-720-replay.json" \
  --markdown-output "$tmpdir/duumbi-720-replay.md"
```

Artifacts:

- JSON report path
- Markdown report path
- replay bundle directory with `ledger.jsonl`
- command output and exit status

Pass/fail criteria:

- Command exits 0 when not in CI mode, unless there is command-level failure.
- JSON report validates schema version and contains two attempt records.
- Ledger contains `run_started`, two `attempt_started`, two terminal attempt
  events, and `run_completed`.
- Report includes exact, semantic, and behavioral agreement metrics.
- Report includes provider usage availability or unavailable reason.
- The live run does not need every generated graph to pass verifier tests; the
  E2E proves measurement correctness, not model quality.

TUI and Studio parity:

- No full TUI or Studio E2E is required because this issue adds a CLI-first
  measurement surface and does not change shared mutation defaults.
- If Stage 10 exposes the command through REPL help, run a thin REPL help smoke
  only. Do not add TUI mutation behavior for this issue.

## Ralph Cycle Protocol

Each Stage 10 implementation cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop only if requirements are met, a blocker appears, the expected external
   LLM cost of the next cycle exceeds USD 1, or scope changes; iteration count
   is not a stop condition

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: prefer 1-4 closely related Rust modules plus
  directly related tests. Larger cycles are allowed only for mechanical
  plumbing across CLI/module declarations when the intended file list is stated
  before work.
- Expected command budget per cycle:
  - `cargo fmt --check` or `cargo fmt`
  - focused `cargo test` for changed modules
  - focused integration test when CLI behavior changes
  - `cargo clippy --all-targets -- -D warnings` before final implementation PR
    readiness
- Human approval required only when:
  - a cycle will use an external LLM with expected cost above USD 1
  - scope exceeds this technical spec
  - the agent proposes a risky dependency, migration, networked service, or
    irreversible operation
  - a product or architecture decision is needed
  - security-sensitive evidence retention behavior changes
  - implementation requires storing raw prompts or provider output by default
- External LLM usage counted:
  - DUUMBI live provider calls made by replay E2E
  - external model/agent CLI calls
  - Codex internal reasoning usage is covered by the Codex App subscription and
    never triggers the gate
- No autonomous batch cap: cycles continue until completion, blocker, gate
  breach, or scope change.
- When to stop and ask for human guidance:
  - inability to satisfy no-secret evidence requirements
  - inability to capture prompt/context hashes without invasive architecture
    changes
  - branch protection or workflow failures that cannot be resolved by updating
    the spec-only or implementation branch
  - live E2E projected external provider cost over USD 1
  - disagreement between product spec and technical feasibility

## Task Breakdown

1. Add `determinism` module skeleton and evidence/report types.
2. Add exact graph digest and state hash helpers with unit tests.
3. Add equivalence grouping and metric calculations with unit tests.
4. Add append-only ledger writer and path-safety checks.
5. Add replay runner for benchmark showcases using isolated workspaces and
   existing provider/config/showcase semantics.
6. Add optional execution evidence sink or context-pack hashing seam in
   `intent::execute`.
7. Add CLI command definitions and dispatch.
8. Add JSON report writing and optional Markdown report rendering.
9. Add rewrite comparison status and metadata-only reporting.
10. Add integration tests for CLI replay, artifact shape, active graph
    preservation, and CI threshold behavior.
11. Run focused local checks and update evidence docs if a live E2E is captured.

Independently executable slices:

- Evidence schema and metrics can be implemented before provider-backed runner.
- Digest/path safety helpers can be implemented and tested independently.
- CLI parsing can land with a stub runner only inside the same cycle if the
  stub is immediately replaced before PR readiness.
- Markdown rendering can follow JSON reporting.
- Live E2E evidence should be last.

## Verification Plan

Local automated checks:

- `cargo fmt --check`
- `cargo test determinism`
- `cargo test --test integration_duumbi720_determinism`
- `cargo test --test integration_phase9c` if benchmark runner/report code is
  changed
- `cargo test --test integration_phase10_context` if context assembly seams are
  changed
- `cargo clippy --all-targets -- -D warnings`

Manual/live checks:

- Build debug binary with `cargo build`.
- Run the live E2E command from this spec with a configured low-cost provider if
  expected provider cost stays below USD 1.
- Inspect JSON and Markdown reports for:
  - schema version
  - run id
  - input filters
  - two attempt records
  - exact/semantic/behavioral agreement metrics
  - provider usage availability/unavailable reason
  - rewrite comparison status
  - no raw credentials
- Verify active workspace graph is unchanged after replay.

Review artifacts:

- Implementation PR summary links this technical spec and product spec.
- Implementation PR includes local test commands and live E2E evidence or a
  clear reason live E2E could not run.
- Implementation PR does not include generated replay workspaces unless a small
  curated Markdown/JSON evidence artifact is intentionally committed under
  `docs/e2e/results/`.

## Completion Criteria

Before implementation PR review:

- `duumbi determinism replay` or the final accepted equivalent CLI command
  exists and is documented in help text.
- Replay output includes schema-versioned JSON report and append-only ledger.
- Exact, semantic, behavioral, and failure-category agreement metrics are
  calculated and tested.
- Attempt isolation is tested.
- Active workspace graph preservation is tested.
- Provider usage unavailable handling is tested.
- Rewrite comparison defaults to `not_yet_comparable` or equivalent conservative
  state unless real comparison evidence exists.
- BDD/process-evidence gaps are represented honestly.
- CI threshold behavior is tested.
- No raw credentials are serialized.
- Product BDD scenarios are covered by tests or documented live/manual evidence.
- Focused tests and clippy pass, or any failures are explained with a blocking
  reason and the issue is not advanced to review.

## Failure And Escalation

- If exact prompt hashing requires invasive changes to the provider or
  orchestrator path, implement partial context-pack hashing with explicit
  `partial` status and file a follow-up only after the core replay metric works.
- If provider-backed live E2E fails because the model produces bad code, keep
  the replay report as valid measurement evidence and do not treat model quality
  as a command failure.
- If provider credentials are missing, skip live E2E and report the missing env
  var; do not add credentials or change provider setup.
- If branch protection requires a fresh `check`, rebase/update the branch and
  rerun checks rather than bypassing the gate.
- If artifact retention risks secrets or unbounded output, default to hashes and
  summaries only.
- If scope expands into Studio dashboard, formal verification, constrained LLM
  rewrite mutation, registry sync, or public marketing copy, stop and request a
  new product decision.

## Open Questions

None blocking.

Accepted implementation risks:

- Exact final provider prompt hashes may require a small execution evidence seam
  in `intent::execute`.
- Persisted arbitrary intent replay may be deferred if benchmark replay covers
  the first measurement slice and arbitrary workspace paths would increase
  safety risk.
- Provider usage may remain unavailable for some adapters until provider clients
  expose usage metadata consistently.
