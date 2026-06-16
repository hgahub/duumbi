# DUUMBI-689: Multi-Function And Multi-Module Intent-Execute Eval At Scale - Technical Specification

Related to #689. This is a specification-only artifact. The execution issue
must remain open for Stage 9 Technical Spec Review, Stage 10 implementation,
Stage 11 implementation review, and Stage 12 closure.

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-689/PRODUCT.md` by
extending DUUMBI's existing provider-backed benchmark path so it can measure
scaled `intent execute` behavior beyond toy single-function examples.

The finished implementation must add:

- A committed scaled intent-execute corpus covering multi-function,
  multi-module, cross-module, branch/recursion, and HTTP + SQLite + JSON
  composition behavior.
- A constrained benchmark mode that runs the scaled corpus through isolated
  workspaces with provider filtering and low-budget smoke selection.
- Per-intent report fields for first-pass success, repair-assisted success,
  retry counts, provider usage availability, token/cost data when exposed,
  dominant error codes, broader failure categories, and supporting evidence.
- Aggregated report summaries for first-pass vs repair-assisted success,
  provider success, failure categories, dominant error codes, usage
  availability, and top failure patterns.
- A committed compact scaled-eval report and a preview known-limitations or
  release-evidence update based on the run.
- Focused automated tests for corpus shape, report aggregation, execution
  metric states, usage-unavailable behavior, and failure categorization.
- A bounded live E2E path that can be run with a configured low-cost provider
  without adding provider-backed calls to normal CI.

Do not add implementation code during this Stage 8 spec PR.

## Agent Audience

This spec is for the Stage 10 implementation agent and Stage 9 technical
reviewers. It assumes the agent can read Rust, DUUMBI intent specs, benchmark
reports, provider setup code, GitHub issue/PR history, and local vault context.

The implementation agent should optimize for honest, reproducible preview
evidence. A failing scaled run is acceptable if the report is accurate,
bounded, committed, and clear about failure patterns.

## Source Context

- Issue: https://github.com/hgahub/duumbi/issues/689
- Stage 4 triage refill comment:
  https://github.com/hgahub/duumbi/issues/689#issuecomment-4701786462
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/689#issuecomment-4711882257
- Product spec: `specs/DUUMBI-689/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/721
- Product spec merge: `578fb7c03847e5178c9e9c03e45a7288643e9600`
- Stage 6 draft comment:
  https://github.com/hgahub/duumbi/issues/689#issuecomment-4715883935
- Stage 7 AI gate decision:
  https://github.com/hgahub/duumbi/issues/689#issuecomment-4715894872
- Stage 7 product spec review decision:
  https://github.com/hgahub/duumbi/issues/689#issuecomment-4715897083
- Project architecture: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Existing intent-create eval script: `scripts/eval_intent.sh`
- Existing benchmark command and runner:
  - `src/cli/mod.rs`
  - `src/main.rs`
  - `src/bench/runner.rs`
  - `src/bench/report.rs`
  - `src/bench/showcases.rs`
- Intent execution source:
  - `src/intent/execute.rs`
  - `src/intent/coordinator.rs`
  - `src/intent/spec.rs`
  - `src/intent/verifier.rs`
- Runtime BDD artifact source:
  - `src/intent/bdd.rs`
  - `specs/DUUMBI-673/PRODUCT.md`
  - `specs/DUUMBI-673/TECHNICAL.md`
- Existing benchmark tests and protocols:
  - `tests/integration_phase9c.rs`
  - `docs/testing/phase9c-benchmark.md`
  - `docs/testing/phase15-walkthrough.md`
- HTTP + SQLite + JSON reference example:
  `examples/flagship-http-sqlite-json/README.md`
- Vault context:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Intent at Scale Multi-Module and BDD.md`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active agentic development runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`

Relevant source facts verified for Stage 8:

- `duumbi benchmark` already executes embedded showcases through
  provider-backed `intent execute` in temporary workspaces.
- `BenchmarkResult` currently records showcase, provider, attempt, success,
  error category, error message, tests passed, tests total, and duration.
- `BenchmarkReport` already aggregates showcase/provider success rates, error
  categories, kill-criterion state, JSON output, and baseline comparison.
- `src/bench/showcases.rs` currently embeds six small showcases, including one
  multi-module case, but the corpus is not sufficient for #689's scale and
  HTTP/SQLite/JSON requirements.
- `src/intent/execute.rs` performs initial mutation, verifier tests, and one
  repair pass, but benchmark output does not yet receive structured
  first-pass, repair, retry, or dominant-error metrics.
- `src/intent/execute.rs` already records mutation success metadata with retry
  count and extracted error codes for learning records; this is the preferred
  source for retry/error metrics when available.
- `src/intent/verifier.rs` verifies i64 function-call outcomes and has a
  special `main` execution path, but it does not verify arbitrary HTTP or JSON
  payload semantics.
- `examples/flagship-http-sqlite-json` demonstrates bounded loopback HTTP,
  in-memory SQLite, and JSON response behavior with accepted runtime/stdlib
  capability.
- `scripts/eval_intent.sh` already models honest unavailable usage reporting
  for provider token/cost fields.
- `IntentSpec` already supports BDD metadata from #673, so scaled eval entries
  can carry scenario context without inventing another BDD format.

Assumptions:

- Extending `src/bench` is lower risk than adding a new unrelated harness
  because it preserves current CLI behavior, provider filtering, isolated
  workspaces, JSON reports, and benchmark tests.
- Some providers may not expose token/cost usage through the current model
  access layer. The implementation should report explicit unavailable reasons
  rather than block the whole issue on telemetry completeness.
- The first implementation can use process-level evidence for HTTP +
  SQLite + JSON behavior while keeping i64 verifier tests for ordinary
  function and module tasks.
- A small committed report is sufficient product evidence when it identifies
  command, date, provider, task selection, result summary, limitations, and top
  failure patterns.

## Affected Areas

Expected Stage 10 source changes:

- `src/bench/showcases.rs`
  - Add scaled corpus entries or load them from a new adjacent data module.
  - Add stable task ids, feature tags, suite names, and evidence requirements.
- New optional `src/bench/scaled.rs` or `src/bench/corpus.rs`
  - Use only if the corpus shape becomes too large for `showcases.rs`.
  - Keep parsing deterministic and provider-free.
- `src/bench/runner.rs`
  - Run scaled cases through isolated workspaces.
  - Preserve existing benchmark behavior for current showcases.
  - Collect structured execution metrics from `intent execute`.
  - Record credential/provider blocks separately from graph failures.
- `src/bench/report.rs`
  - Extend result and aggregate report types with first-pass, repair, retry,
    provider usage, dominant error, evidence, and failure-pattern fields.
  - Preserve backward-compatible deserialization where practical by making new
    fields defaulted or optional.
- `src/intent/execute.rs`
  - Add a structured execution outcome path used by the benchmark runner.
  - Keep the existing public CLI behavior and log output stable.
- `src/intent/verifier.rs`
  - Do not broaden verifier semantics unless required by the minimal
    HTTP/SQLite/JSON evidence path.
  - Prefer a benchmark-level process evidence check for non-i64 behavior.
- `src/cli/mod.rs` and `src/main.rs`
  - Add only narrow benchmark flags if needed, such as `--suite scaled` or
    `--smoke`, while preserving current `--showcase`, `--provider`,
    `--attempts`, `--output`, `--ci`, and `--baseline` behavior.
- `tests/integration_phase9c.rs`
  - Extend existing benchmark coverage for new result fields and corpus
    selection.
- New focused tests only if they keep coverage clearer:
  - Corpus metadata tests.
  - Report aggregation tests.
  - Structured execution metric tests.
  - Process evidence tests using local deterministic fixtures.
- Documentation and evidence:
  - `docs/testing/phase9c-benchmark.md`
  - `docs/testing/phase15-walkthrough.md` or a focused scaled-eval doc.
  - A compact committed scaled-eval report under an agreed evidence path, such
    as `docs/e2e/duumbi-689-scaled-eval-report.json` or
    `docs/e2e/duumbi-689-scaled-eval-report.md`.
  - A preview known-limitations or release-evidence note.

Areas that must not change during Stage 8:

- implementation code
- tests
- benchmark reports
- product spec content
- issue workflow automation

Areas out of Stage 10 scope unless a later reviewer approves scope expansion:

- A new general-purpose eval framework unrelated to `duumbi benchmark`.
- Public network services, production HTTP listeners, production registry
  dependencies, or new credential types.
- Broad provider-routing redesign or user-facing default-model selection.
- Full Cucumber/Gherkin runtime support.
- General non-i64 verifier redesign.
- New DUUMBI runtime/stdlib operations solely to make the scaled eval pass.
- Query mode mutation behavior.

## Technical Approach

### 1. Keep `duumbi benchmark` As The Canonical Eval Surface

Extend the existing benchmark command rather than introducing a second command.
The preferred UX is:

```sh
target/debug/duumbi benchmark --suite scaled --smoke --provider minimax --attempts 1 --output /tmp/duumbi-689-scaled-smoke.json
```

If adding `--suite` is too broad for the current CLI shape, use an equivalent
explicit showcase filter such as:

```sh
target/debug/duumbi benchmark --showcase scaled_math_pipeline --showcase scaled_cross_module --provider minimax --attempts 1 --output /tmp/duumbi-689-scaled-smoke.json
```

The implementation must preserve existing benchmark invocations. Current
showcases should still run by default unless the final design documents a
compatible opt-in behavior for scaled tasks.

### 2. Define A Scaled Corpus With Metadata

Add a benchmark case model that wraps `IntentSpec` with eval metadata. The
exact type names are implementation choices, but the data should support this
shape:

```rust
pub struct BenchmarkCase {
    pub id: String,
    pub suite: BenchmarkSuite,
    pub tags: Vec<String>,
    pub intent: IntentSpec,
    pub verification: BenchmarkVerification,
    pub expected_budget: BenchmarkBudget,
}
```

Recommended verification variants:

- `I64Tests`: use the existing `IntentSpec.test_cases` and verifier path.
- `MainExit`: run the generated `main` path when the existing verifier can
  judge exit-code or last-stdout behavior.
- `ProcessEvidence`: run a bounded local command or check that records
  reviewable evidence for non-i64 behavior, such as loopback HTTP JSON output.

The first scaled corpus should include at least these cases:

- `scaled_math_pipeline`: one module, multiple functions, multiple verifier
  cases, no external dependencies.
- `scaled_cross_module_stats`: two or more modules where one module imports or
  calls another module's exported function.
- `scaled_branch_recursion`: branch-heavy or recursion-shaped behavior that can
  expose SSA dominance, control-flow, or verifier mismatch failures.
- `scaled_string_or_array_module`: multi-function behavior using current
  string or array support if already stable enough for benchmark evidence.
- `scaled_http_sqlite_json`: bounded loopback/in-memory composition based on
  existing flagship example capability and stdlib imports.

For `scaled_http_sqlite_json`, do not claim full generated-service success
unless the eval actually runs and checks generated behavior. If the existing
verifier cannot check the HTTP JSON response directly, record process evidence
with fields such as `evidence_kind`, `checked_command`, `expected_route`,
`expected_json_fields`, `status`, and `verification_gap`.

### 3. Expose Structured `intent execute` Metrics

Add a structured execution entry point in `src/intent/execute.rs`, for example:

```rust
pub struct IntentExecutionOutcome {
    pub success: bool,
    pub first_pass_success: bool,
    pub repair_attempted: bool,
    pub repair_success: Option<bool>,
    pub mutation_retry_count: u32,
    pub repair_retry_count: u32,
    pub dominant_error_code: Option<String>,
    pub error_codes: Vec<String>,
    pub tests_passed: usize,
    pub tests_total: usize,
}
```

The exact field names should align with `BenchmarkResult`, but the
implementation must keep existing CLI behavior available. A safe pattern is:

1. Refactor the current execute path into a structured internal function.
2. Keep `run_execute` and `run_execute_with_progress` as compatibility wrappers
   that render the same logs and boolean outcome.
3. Have the benchmark runner call the structured path.

Collect:

- first verifier/process result before repair
- whether repair was attempted
- final verifier/process result after repair
- mutation retry counts from task mutation outcomes
- repair retry counts from repair mutation outcomes when exposed
- dominant DUUMBI error code from structured errors or existing error-code
  extraction
- provider/credential/auth/rate-limit/timeout failures separately from graph
  failures

Do not change query mode side-effect behavior.

### 4. Extend Benchmark Result And Aggregation Types

Extend `BenchmarkResult` with defaulted or optional fields so old reports can
still be read where practical:

```rust
pub struct BenchmarkResult {
    // existing fields
    pub task_id: Option<String>,
    pub suite: Option<String>,
    pub tags: Vec<String>,
    pub first_pass_success: Option<bool>,
    pub repair_attempted: bool,
    pub repair_success: Option<bool>,
    pub mutation_retry_count: Option<u32>,
    pub repair_retry_count: Option<u32>,
    pub total_retry_count: Option<u32>,
    pub dominant_error_code: Option<String>,
    pub provider_usage: ProviderUsageSummary,
    pub evidence: Option<BenchmarkEvidence>,
}
```

Required usage shape:

```rust
pub struct ProviderUsageSummary {
    pub available: bool,
    pub request_count: Option<u32>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
    pub unavailable_reason: Option<String>,
}
```

When token/cost metadata is unavailable, write `available: false` and a reason
such as `provider_response_did_not_expose_usage`, `model_access_layer_missing`,
or `credential_blocked`. Do not omit usage fields from scaled reports.

Aggregates must include:

- per-task success rate
- per-provider success rate
- first-pass success rate
- repair-assisted success rate
- unrecovered failure count
- failure category counts
- dominant error-code counts
- total retry count
- usage-available and usage-unavailable counts
- total estimated cost when available
- top 2-3 failure patterns

### 5. Keep HTTP/SQLite/JSON Evidence Narrow And Honest

The HTTP + SQLite + JSON task should use only accepted local behavior:

- loopback host only
- bounded request count
- in-memory SQLite unless a workspace-confined file is required
- deterministic JSON fields
- no public service, no TLS/auth, no migrations, no concurrent server claim
- no committed raw provider payloads or secrets

The evidence path may reuse the flagship example as a reference fixture, but
the report must distinguish:

- generated intent-execute behavior
- reference process evidence
- manual review evidence
- unsupported verification gaps

If the implementation cannot make provider-generated graph output exercise
HTTP + SQLite + JSON in the first slice, it must still include the corpus entry
and report the gap explicitly instead of silently passing the scenario.

### 6. Produce Durable Preview Evidence

After the bounded run, commit a compact report. Prefer Markdown if the report
needs reviewer narrative, or JSON if it is primarily machine-readable. Either
format must include:

- exact command
- date
- DUUMBI version or commit
- provider and model when known
- selected tasks
- attempt count
- result table
- first-pass vs repair-assisted summary
- retry summary
- usage availability and total cost when available
- provider/credential blocks if any
- top 2-3 failure patterns
- known limitations
- path to any retained sampled evidence

Update the preview known-limitations or release-evidence note from the report.
Do not make success claims beyond measured evidence.

## Invariants

- The implementation must preserve current `duumbi benchmark` behavior for
  existing callers.
- Normal CI must not make live provider calls.
- CI-safe tests must use deterministic or mock provider behavior.
- Live eval commands must fail closed when credentials are absent and must not
  count missing credentials as graph-generation failure.
- Reports must explicitly represent unavailable usage fields.
- Reports must separate provider/auth/rate-limit/network failures from graph,
  compiler, verifier, and product-logic failures.
- First-pass success must mean verification passed before any repair cycle.
- Repair-assisted success must remain distinguishable from first-pass success.
- The HTTP + SQLite + JSON task must use local bounded behavior and must not
  imply production service readiness.
- Secrets, raw provider payloads, generated binaries, large logs, and temporary
  workspaces must not be committed.
- Spec-only and implementation PR text must use non-closing references such as
  `Related to #689` or `Implementation for #689`.

## BDD-To-Test Mapping

| Product scenario | Automated, E2E, manual, or review evidence |
| --- | --- |
| Run a constrained scaled eval corpus | Unit tests prove scaled cases load with required metadata; an integration test or deterministic runner test proves selected scaled cases run in isolated temp workspaces and emit one result per selected task/provider/attempt; live E2E evidence records the documented constrained command. |
| Evaluate cross-module behavior | Corpus test asserts at least one scaled case declares two or more modules and a cross-module expectation; deterministic or mock integration test asserts missing export/unresolved call failures map to a dominant error or failure category; live report records the generated graph build/verification outcome for the cross-module case. |
| Evaluate HTTP SQLite JSON composition | Corpus test asserts an HTTP/SQLite/JSON case exists with loopback/in-memory evidence metadata; process evidence test validates the evidence checker with deterministic local output; live or manual E2E report records the actual HTTP, SQLite, and JSON checks or an explicit unsupported verification gap. |
| First pass succeeds without repair | Report aggregation unit test builds a synthetic successful first-pass result and asserts `first_pass_success=true`, `repair_attempted=false`, retry fields present or unavailable, and aggregate first-pass rate updated. |
| Repair turns a failed first pass into a success | Structured execution or report unit test builds a synthetic repair-assisted result and asserts `first_pass_success=false`, `repair_attempted=true`, `repair_success=true`, final success true, and aggregate repair-assisted rate updated separately from first-pass rate. |
| Repair does not recover the task | Structured execution or report unit test builds an unrecovered failure and asserts final success false, dominant error/failure category present, sampled evidence path or message present, and failure-pattern aggregation updated. |
| Provider usage is available | Unit test builds a result with usage metadata and asserts request count, prompt tokens, completion tokens, total tokens, estimated cost, and aggregate cost are serialized and summarized. |
| Provider usage is unavailable | Unit test builds a result with `available=false` and a concrete reason; JSON snapshot or shape assertion proves usage fields are still present; docs/review evidence confirms no unsupported cost claim is made. |
| Commit a scaled eval report | Implementation PR includes a committed compact report under the chosen evidence path; review checks exact command, date, provider, selected tasks, result summary, limitations, and absence of secrets/raw provider payloads. |
| Publish known limitations from the run | Implementation PR updates preview known-limitations or release-evidence text from the committed report; review checks it states passed, failed, repair-assisted, and top 2-3 failure patterns without overstating reliability. |
| Missing credentials block live evidence without corrupting the score | Unit or integration test simulates absent credentials and asserts the result is provider-unavailable or blocked, not graph failure; live E2E instructions document the provider setup path for rerun. |

## Live E2E Plan

The live E2E plan is required for the implementation PR but must remain outside
normal CI. Prefer MiniMax for a low-cost local run when `MINIMAX_API_KEY` is
available; another configured direct provider may be used if MiniMax is not
available.

Canonical local command sequence:

```sh
cargo build
repo="$(pwd)"
tmpdir="$(mktemp -d)"
"$repo/target/debug/duumbi" init "$tmpdir"
cd "$tmpdir"
"$repo/target/debug/duumbi" benchmark --suite scaled --smoke --provider minimax --attempts 1 --output "$tmpdir/duumbi-689-scaled-smoke.json"
```

If `duumbi benchmark` must run from a repo checkout rather than an initialized
workspace, document the final correct command in the implementation PR and
report. The command must be runnable by a developer from a clean checkout with
normal provider setup.

Expected live run shape:

- Provider: MiniMax or another configured direct provider.
- Credentials: existing provider setup or provider-specific environment
  variable. Do not commit credentials.
- Suite: scaled smoke subset.
- Attempts: 1 per selected task/provider for the required low-budget evidence.
- Tasks: at least one multi-function task, one cross-module task, and the
  HTTP/SQLite/JSON task or its process-evidence equivalent.
- Expected external LLM call budget: implementation agent must estimate before
  running. If the expected external provider cost for a single live E2E action
  exceeds USD 1, stop and request human approval.
- Expected committed output: compact report under the chosen evidence path,
  not raw provider payloads.

Required live evidence:

1. Command used.
2. Provider and model if known.
3. Credential state or explicit credential block.
4. Selected task ids.
5. Per-task first-pass, repair, retry, usage, and failure evidence.
6. HTTP/SQLite/JSON evidence checked or explicit verification gap.
7. Top 2-3 failure patterns from the run.
8. Preview limitations text derived from the report.

If credentials are missing or the provider is unavailable, the implementation
PR may still proceed only if:

- deterministic tests pass;
- the report records the live block clearly;
- the PR explains the exact rerun command; and
- the issue still receives honest known-limitations evidence.

## Ralph Cycle Protocol

Use Ralph cycles only during Stage 10 implementation, not during this spec
stage. Each cycle should have one bounded goal and leave reviewable evidence in
the implementation PR.

Recommended cycles:

- Cycle 1: Add scaled corpus metadata and CI-safe corpus tests.
- Cycle 2: Add structured execution outcome plumbing from `intent execute` to
  the benchmark runner while preserving current CLI behavior.
- Cycle 3: Extend benchmark report fields, aggregation, JSON serialization, and
  report tests.
- Cycle 4: Add HTTP/SQLite/JSON process evidence support and deterministic
  tests for evidence gaps.
- Cycle 5: Add docs, live E2E command, committed report, preview limitations,
  and final verification evidence.

Stop early if a cycle reveals a blocker that would require general verifier
redesign, new runtime APIs, public network behavior, or product claims that
contradict `PRODUCT.md`.

## Cycle Budget

- External LLM approval threshold: request human approval before any single
  cycle, live E2E action, or reviewer action expected to exceed USD 1 in
  external LLM cost.
- Autonomous iteration cap: none. Continue Ralph cycles until the scoped
  implementation goal is complete, a gate fails, or a blocker appears.
- Default cycle scope: one coherent implementation slice touching roughly one
  subsystem plus its focused tests or docs evidence.
- Suggested per-cycle file budget: 2-8 files. Exceeding this is allowed for
  mechanical report/corpus/test propagation, but explain it in the PR.
- Suggested per-cycle command budget: targeted Rust tests for the changed
  module first; run broader verification before requesting review.
- Codex internal reasoning, local source inspection, and local deterministic
  tests do not count as external LLM cost.

Human approval is required before:

- Expected external provider cost above USD 1 for a single action.
- Adding a new external service, credential requirement, or public network
  dependency.
- Adding new DUUMBI runtime operations, stdlib exports, or verifier semantics.
- Replacing `duumbi benchmark` with a separate harness.
- Changing provider setup UX or requiring users to maintain a default model.
- Making a product decision that contradicts `PRODUCT.md`.
- Continuing after a cleanly reproduced architecture blocker.

## Task Breakdown

1. Confirm the issue has product-spec approval and is in the Stage 10-ready
   label/status path before starting implementation.
2. Add scaled corpus metadata and CI-safe corpus shape tests.
3. Add or refactor benchmark selection for scaled smoke/full suites while
   preserving existing benchmark behavior.
4. Add structured `intent execute` outcome plumbing for first-pass, repair,
   retry, error-code, and verifier metrics.
5. Extend benchmark result/report types with defaulted usage, evidence, and
   failure-pattern fields.
6. Add aggregation and serialization tests for first-pass success,
   repair-assisted success, unrecovered failure, usage available, usage
   unavailable, and provider blocked states.
7. Add narrow HTTP/SQLite/JSON process evidence support or explicit
   verification-gap reporting.
8. Add deterministic tests for cross-module failure categorization and
   HTTP/SQLite/JSON evidence shape.
9. Update benchmark docs and live E2E instructions.
10. Run the constrained live E2E path or record a credential/budget block.
11. Commit the compact scaled-eval report.
12. Update preview known-limitations or release evidence from the report.
13. Run required verification and record evidence in the implementation PR.

## Verification Plan

Minimum local verification during implementation:

```sh
cargo fmt --check
cargo test --test integration_phase9c
```

Expected additional targeted tests, with exact names chosen by Stage 10:

```sh
cargo test bench::report
cargo test bench::showcases
cargo test intent::execute
```

Recommended broader verification before implementation review:

```sh
cargo test --all
cargo clippy --all-targets -- -D warnings
```

Manual or gated live E2E verification:

```sh
cargo build
repo="$(pwd)"
tmpdir="$(mktemp -d)"
"$repo/target/debug/duumbi" init "$tmpdir"
cd "$tmpdir"
"$repo/target/debug/duumbi" benchmark --suite scaled --smoke --provider minimax --attempts 1 --output "$tmpdir/duumbi-689-scaled-smoke.json"
```

The implementation PR must include review evidence for:

- The exact live command or credential block.
- The committed report path.
- First-pass vs repair-assisted summary.
- Usage available/unavailable summary.
- HTTP/SQLite/JSON evidence or explicit verification gap.
- Top 2-3 failure patterns.
- Preview known-limitations or release-evidence update.
- Absence of secrets, raw provider payloads, and large generated logs.

## Completion Criteria

Stage 10 implementation is complete only when:

- The scaled corpus exists and includes all feature classes required by
  `PRODUCT.md`.
- `duumbi benchmark` or its compatible benchmark extension runs the constrained
  scaled smoke subset.
- Per-intent results expose first-pass, repair, retry, usage, dominant error,
  and evidence fields.
- Aggregates expose first-pass vs repair-assisted success, failure categories,
  dominant error codes, usage availability, and top failure patterns.
- CI-safe tests cover corpus parsing, report aggregation, unavailable usage,
  provider blocks, first-pass success, repair-assisted success, and unrecovered
  failure.
- HTTP + SQLite + JSON composition is either actually checked through bounded
  process evidence or explicitly reported as a verification gap.
- A compact scaled-eval report is committed.
- Preview known-limitations or release-evidence docs are updated from actual
  report findings.
- The implementation PR records live E2E evidence or an honest credential or
  budget block.
- Issue #689 remains open until the final implementation PR is merged and
  Stage 12 closure evidence is recorded.

## Failure And Escalation

Escalate and stop implementation if:

- `duumbi benchmark` cannot be extended without breaking existing benchmark
  behavior.
- Structured first-pass/repair metrics require broad intent-execute redesign
  instead of a narrow outcome wrapper.
- HTTP + SQLite + JSON evidence would require new runtime APIs, public network
  behavior, or production service semantics.
- Provider usage data cannot be exposed and no honest unavailable representation
  can be serialized.
- Live provider execution is expected to exceed USD 1 in external cost without
  human approval.
- Any gate reviewer raises a blocking product, architecture, security, or test
  finding.

Escalation output should include:

- The exact command, test, or source path that exposed the blocker.
- The smallest missing capability.
- Whether the blocker belongs to product scope, benchmark architecture,
  verifier limits, provider telemetry, or runtime/stdlib capability.
- A recommended next issue or narrower implementation slice.

## Open Questions

No blocking open questions remain.

Non-blocking Stage 10 choices:

- Whether scaled corpus entries remain embedded in `src/bench/showcases.rs` or
  move into adjacent source-controlled data files.
- Whether the initial HTTP + SQLite + JSON task reports generated behavior,
  reference process evidence, or an explicit verification gap.
- Whether provider usage can be sourced from current model telemetry in the
  first slice or must remain unavailable for some provider paths.
