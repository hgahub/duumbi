# DUUMBI-689: Multi-Function And Multi-Module Intent-Execute Eval At Scale

## Summary

Extend DUUMBI's intent execution evaluation so the project can measure whether
AI graph generation works beyond tiny single-function examples.

The current accepted issue asks for a scaled eval corpus and report covering
multi-function, multi-module programs, including at least one composition task
that exercises HTTP, SQLite, and JSON behavior. The outcome should be a
committed, reproducible evidence artifact that reports first-pass success,
repair-cycle success, retry count, token/cost visibility, and dominant failure
patterns by intent.

Related to #689. This PR is specification-only. The execution issue must remain
open for Stage 8 technical specification, Stage 9 approval, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Problem

DUUMBI has provider-backed intent-create and intent-execute evidence, but the
strongest passing examples are still small relative to the product claim. The
issue body identifies the gap directly: passing `add`, `is_even`, `fibonacci`,
`gcd`, and `is_prime` does not prove DUUMBI can build programs with interacting
functions, modules, stdlib composition, branch-heavy control flow, or repair
behavior under a realistic model budget.

That creates a preview credibility problem:

- Preview users may infer that the write path works for real application-scale
  work when the evidence only proves toy-scale behavior.
- DUUMBI cannot distinguish first-pass graph generation quality from success
  after repair.
- Provider or prompt changes can regress multi-module behavior without a
  stable corpus catching the regression.
- Known hard areas such as SSA dominance, branch-based loops, cross-module
  calls, stdlib imports, and ownership-like graph patterns are not separated in
  the published evidence.

The product problem is not that every scaled intent must pass before preview.
The product problem is that DUUMBI must publish an honest, repeatable view of
what passes, what fails, how much repair was needed, and which failure patterns
are blocking progress.

## Outcome

When this work is complete:

- DUUMBI has a committed scaled intent-execute corpus with multi-function and
  multi-module tasks.
- The corpus includes at least one bounded HTTP + SQLite + JSON composition
  task based on existing DUUMBI runtime and stdlib capability.
- A single documented eval command can run a constrained-budget scaled suite
  against configured providers.
- The report records per-intent first-pass success, repair-cycle success,
  retry count, provider/model identity, usage availability, token/cost fields
  when available, duration, test/build/runtime evidence, and dominant error
  codes or failure categories.
- The report is committed in the repo as a durable preview evidence artifact,
  not only left in `/tmp` or local terminal output.
- The preview docs or release notes receive a known-limitations summary based
  on the actual eval results.
- The top 2-3 failure patterns are identified in a form that can feed mutation
  prompts, algorithmic hints, agent knowledge, or follow-up issues.

## Scope

### In Scope

- Extend the existing intent-execute benchmark/eval surface rather than
  creating an unrelated one, unless implementation proves that reuse is not
  viable.
- Add a scaled corpus that includes:
  - multi-function tasks
  - multi-module tasks
  - branch/loop-shaped behavior where DUUMBI currently models loops through
    branches or recursion
  - cross-module call behavior
  - at least one HTTP + SQLite + JSON composition task using existing
    loopback/in-memory behavior
  - BDD scenario context where current runtime intent BDD artifacts can support
    the eval
- Track first-pass verification separately from repair-after-verification.
- Track retry counts from mutation and repair attempts where the current
  execution pipeline exposes them, and add visible "unavailable" markers where
  it does not yet expose them.
- Track token/cost usage when provider or model telemetry exposes it, and make
  unavailable usage explicit rather than omitting the field.
- Report dominant DUUMBI error codes and broader failure categories per intent.
- Produce a committed JSON or Markdown report under the repo's existing docs or
  eval evidence area.
- Update the preview known-limitations note, release-note draft, or internal
  preview evidence doc to summarize the results.
- Add focused automated tests for corpus parsing, report aggregation, error
  categorization, first-pass/repair metrics, and docs/report shape.
- Add a live E2E plan and at least one manual or gated live provider run path
  suitable for MiniMax or another configured direct provider.

### Explicitly Out Of Scope

- Claiming the scaled eval must achieve a specific high pass rate before this
  issue can close. The accepted issue asks for honest measurement and failure
  patterns; it does not require hiding poor results.
- Rebuilding the whole intent coordinator, verifier, provider routing, or graph
  compiler.
- Adding a general Cucumber/Gherkin runtime.
- Adding broad non-i64 verifier semantics unless the technical spec proves a
  narrow extension is needed for the composition eval.
- Adding public production HTTP services, public network listeners, persistent
  on-disk databases, TLS, authentication, migrations, or concurrent server
  behavior.
- Running provider-backed evals in normal CI.
- Requiring users to maintain a default model selection outside the provider
  setup flow.
- Starting implementation code, tests, Ralph cycles, or implementation PRs
  during this product specification stage.

## Constraints And Assumptions

Facts:

- Issue #689 is open and labeled `accepted` and `needs-spec`.
- The Stage 5 decision comment on June 15, 2026 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- The issue is a v0.4.0 Developer Preview blocker and is labeled
  `module:intent` and `testing`.
- The issue body requires multi-function and multi-module intents, including at
  least one HTTP/DB/JSON task.
- The existing `duumbi benchmark` command runs provider-backed `intent execute`
  against embedded showcases and emits JSON reports.
- Current benchmark result data records showcase, provider, attempt, success,
  error category, error message, tests passed, tests total, and duration.
- Existing benchmark reports do not yet expose first-pass success,
  repair-cycle success, retry count, token cost, or dominant error codes.
- `scripts/eval_intent.sh` evaluates `intent create`, not scaled
  `intent execute`, and marks provider usage as unavailable.
- `src/intent/execute.rs` performs task mutations, then verifier tests, then
  one repair pass when verifier tests fail.
- `src/intent/verifier.rs` currently verifies i64 function-call test cases and
  has special handling for `main`, but it returns a single i64 value rather
  than arbitrary structured output.
- `specs/DUUMBI-673` and the current source already add runtime intent BDD
  artifact support.
- `examples/flagship-http-sqlite-json` is a bounded loopback HTTP + SQLite +
  JSON reference example using `@duumbi/stdlib-db`, `@duumbi/stdlib-json`, and
  `@duumbi/stdlib-server`.
- The active PRD describes DUUMBI as intent-first, evidence-oriented, and
  human-verifiable.
- The active runbook sets the Ralph Cycle external-LLM approval threshold at
  expected cost above USD 1. The active glossary still contains older Ralph
  threshold wording, so the runbook and Stage 8 skill are treated as the
  current policy for this issue.

Assumptions:

- Reusing `duumbi benchmark` and `src/bench` is preferable because it already
  runs isolated provider-backed `intent execute` attempts and has report
  aggregation, provider filtering, CI mode, baseline comparison, and tests.
- The HTTP + SQLite + JSON eval should be bounded to loopback and in-memory
  behavior so it remains suitable for local/manual preview evidence.
- Provider usage telemetry may remain partially unavailable in the first
  implementation slice. If so, the product contract is explicit unavailable
  fields and a reason, not false precision.
- The first scaled corpus can be small enough to run on a constrained budget,
  provided it still covers multiple size and feature classes.
- A committed report can include failing results; failure is product evidence
  as long as the report is reproducible and clearly labeled.

Constraints:

- Specs and PR wording must use non-closing references such as `Related to
  #689`; spec-only PRs must not close the execution issue.
- Query mode must remain read-only.
- Provider setup remains `/provider`-centered; this issue must not introduce
  user-facing default-model maintenance.
- Live provider evals must never commit credentials, raw provider payloads, or
  secret-bearing logs.
- CI-safe tests must use deterministic or mock provider paths and must not make
  live LLM calls.
- Eval artifacts should be compact enough for review. Large raw logs belong in
  temporary or explicitly sampled evidence, not broad committed dumps.

## Decisions

- **Decision:** Use a source-repo file-based product spec.
  **Evidence:** The issue is preview-blocking, cross-module, touches provider
  eval behavior, reporting, docs, and release evidence, and needs durable Stage
  8/10 implementation context.

- **Decision:** The product surface should extend the existing benchmark/eval
  path rather than create a second command from scratch.
  **Evidence:** `duumbi benchmark` already runs provider-backed
  `intent execute` in temporary workspaces, supports provider/showcase filters,
  emits JSON, and has report aggregation tests.

- **Decision:** The eval should measure and publish both success and failure.
  **Evidence:** The issue's user outcome asks for an honest picture of where AI
  graph generation works and breaks, not only for a success badge.

- **Decision:** The HTTP + SQLite + JSON task should be bounded to existing
  loopback/in-memory capabilities.
  **Evidence:** The checked-in flagship example demonstrates this exact
  composition pattern and excludes production web-service features.

- **Decision:** BDD artifacts are an input/evidence aid, not the only proof.
  **Evidence:** DUUMBI-673 intentionally makes BDD scenarios guide agents and
  evidence mapping while preserving graph validation, build/run checks, and
  verifier tests as proof.

- **Decision:** Missing provider usage data must be represented explicitly.
  **Evidence:** Existing eval script reports usage as unavailable with a reason;
  hiding unavailable data would make cost claims misleading.

## Behavior

### Corpus Behavior

- The scaled corpus is stored in source-controlled files with stable names.
- Each corpus entry has:
  - a unique task id
  - intent text
  - expected created and modified modules
  - expected function or behavior checks
  - difficulty or feature tags
  - BDD scenario text or references when applicable
  - provider budget expectations or skip reason when live execution is too
    expensive
- The corpus contains small, medium, and hard tasks.
- At least one task requires multiple functions in one module.
- At least one task requires more than one new module.
- At least one task requires cross-module calls.
- At least one task stresses branch/recursion behavior that historically causes
  SSA dominance or branch-based loop risk.
- At least one task exercises existing HTTP + SQLite + JSON composition through
  bounded loopback/in-memory behavior.
- Tasks that cannot yet be fully verified by the i64 verifier must still
  produce reviewable evidence and must be labeled by evidence type.

### Eval Command Behavior

- A developer can run a constrained scaled suite with one documented command
  from a normal DUUMBI workspace that has provider setup.
- The command supports provider filtering and task/showcase filtering.
- The command supports a low-budget smoke subset and a larger full suite.
- Normal CI must not call live providers.
- CI-safe tests validate parsing, aggregation, report fields, and failure
  categorization without network calls.
- A live provider run fails closed when required credentials are missing and
  records the block clearly as missing credentials, not as product failure.
- If a provider call fails due to auth, rate limit, timeout, model access, or
  network problems, the report distinguishes provider failure from graph,
  compiler, verifier, or product-logic failure.

### Metrics Behavior

- Each per-intent result records:
  - task id
  - provider and model or model-unavailable reason
  - attempt number
  - duration
  - first-pass verification success
  - repair-cycle attempted
  - repair-cycle success
  - mutation retry count, repair retry count, and total retry count when
    available
  - token usage and estimated cost when available
  - explicit usage-unavailable reason when unavailable
  - tests passed and tests total when verifier tests apply
  - additional evidence status for non-i64 or process-level checks
  - dominant error code when DUUMBI emits one
  - broader failure category
  - path to sampled logs or artifacts when retained
- Aggregated report data includes:
  - per-task success rate
  - per-provider success rate
  - first-pass vs repair-assisted success rate
  - failure category counts
  - dominant error code counts
  - total estimated cost when usage is available
  - unavailable usage count and reasons
- Reports should be deterministic in field names and stable enough for baseline
  comparisons.

### Report And Preview Evidence Behavior

- A committed report summarizes at least one constrained scaled run.
- The report records the exact command, date, provider(s), attempted tasks,
  result summary, and known limitations.
- If the run is partial because credentials, budget, or provider availability
  blocked full execution, the report states the limitation explicitly.
- The preview known-limitations note or release-note draft summarizes:
  - which scaled tasks passed
  - which failed
  - whether success required repair
  - top 2-3 failure patterns
  - what users should not infer from the preview
- The report does not include secrets, raw provider payloads, or unnecessary
  full logs.

### BDD And Evidence Behavior

- Scaled eval entries that have BDD scenarios use English Gherkin-style
  scenario text.
- During execution, BDD context is used only through the existing runtime
  intent BDD behavior and bounded prompt/evidence summaries.
- BDD scenarios map to concrete evidence in the technical spec and
  implementation evidence. A scenario is not treated as passed merely because a
  `.feature` file exists.
- For non-i64 behaviors such as loopback HTTP output, the eval must record
  process or manual evidence until the verifier supports richer checks.

### Failure Pattern Behavior

- The final report identifies the top 2-3 failure patterns from the run.
- Failure pattern labels should be useful for follow-up work, such as:
  - wrong decomposition
  - missing export
  - cross-module call resolution
  - type mismatch
  - branch/SSA dominance
  - verifier mismatch
  - hallucinated op or stdlib function
  - provider failure
  - budget/credential block
- Each failure pattern includes enough evidence to route a follow-up prompt,
  algorithmic hint, or issue.

## BDD Scenarios

Feature: Scaled intent-execute eval evidence for Developer Preview

  Rule: The eval measures real multi-function and multi-module behavior

    Scenario: Run a constrained scaled eval corpus
      Given a DUUMBI workspace with at least one configured supported provider
      And the scaled corpus contains multi-function and multi-module tasks
      When the developer runs the documented constrained eval command
      Then DUUMBI runs each selected task in an isolated workspace
      And the report records one result per selected task, provider, and
      attempt
      And the report separates product/graph failures from provider or
      credential failures

    Scenario: Evaluate cross-module behavior
      Given the scaled corpus includes a task that creates at least two modules
      And the task requires one module to call an exported function from another
      module
      When the eval runs that task
      Then the report records whether the generated graph built and verified
      the cross-module call behavior
      And missing exports or unresolved cross-module calls are reported as
      dominant failure evidence when they occur

    Scenario: Evaluate HTTP SQLite JSON composition
      Given the scaled corpus includes a bounded loopback HTTP SQLite JSON task
      And the task uses existing DUUMBI stdlib/runtime capability rather than a
      public production server
      When the eval runs or verifies that task
      Then the report records the HTTP, SQLite, and JSON evidence that was
      actually checked
      And any unsupported verification gap is labeled as broader evidence
      required rather than silently passed

  Rule: The report distinguishes first-pass quality from repair-assisted success

    Scenario: First pass succeeds without repair
      Given a selected eval task completes all mutation tasks
      And verifier or process-level checks pass before repair
      When the report is written
      Then `first_pass_success` is true
      And `repair_attempted` is false
      And retry counts and usage fields are present or explicitly unavailable

    Scenario: Repair turns a failed first pass into a success
      Given a selected eval task fails initial verification
      And DUUMBI applies an allowed repair cycle
      And verification passes after repair
      When the report is written
      Then `first_pass_success` is false
      And `repair_attempted` is true
      And `repair_success` is true
      And the result remains distinguishable from a clean first-pass success

    Scenario: Repair does not recover the task
      Given a selected eval task fails initial verification
      And repair is attempted or no repair patch is available
      When final verification still fails
      Then the report records failure
      And the dominant error code or failure category is present
      And sampled evidence is sufficient to identify a follow-up failure
      pattern

  Rule: Cost and usage evidence is explicit

    Scenario: Provider usage is available
      Given the provider or model telemetry exposes token and cost data
      When a scaled eval task runs
      Then the per-task result records request count, prompt tokens,
      completion tokens, total tokens, and estimated cost
      And the aggregate report includes total estimated cost for available
      usage

    Scenario: Provider usage is unavailable
      Given the provider path does not expose token or cost data
      When a scaled eval task runs
      Then the per-task result still includes provider usage fields
      And `available` is false with a specific unavailable reason
      And the preview summary does not make unsupported cost claims

  Rule: Preview evidence is durable and honest

    Scenario: Commit a scaled eval report
      Given a constrained scaled eval run has completed
      When the implementation PR is prepared
      Then a compact report is committed under the agreed repo evidence path
      And the report includes command, date, provider, selected tasks, result
      summary, and limitations
      And raw secrets or provider payloads are absent

    Scenario: Publish known limitations from the run
      Given the report identifies passing and failing scaled tasks
      When the preview known-limitations note or release-note draft is updated
      Then it states what passed, what failed, and what required repair
      And it lists the top 2-3 failure patterns
      And it avoids claiming real-service reliability beyond measured evidence

    Scenario: Missing credentials block live evidence without corrupting the
    score
      Given the live provider credential required for the selected provider is
      absent
      When the developer runs the live eval command
      Then the eval reports a blocked or provider-unavailable result
      And the report does not count that result as a graph-generation failure
      And the user sees the credential/setup path needed to rerun the eval

## Tasks

1. Confirm the implementation surface:
   - Prefer extending `src/bench`, `src/main.rs` benchmark command handling,
     and `tests/integration_phase9c.rs`.
   - Document any reason a separate harness is required before creating one.
2. Define the scaled corpus:
   - Add multi-function, multi-module, branch/recursion, cross-module, and
     HTTP + SQLite + JSON entries.
   - Keep a constrained smoke subset for low-cost runs.
3. Extend result data:
   - Add first-pass, repair, retry, usage, dominant error code, and additional
     evidence fields.
   - Preserve backward-compatible report parsing where practical.
4. Extend aggregation and output:
   - Add first-pass vs repair-assisted success rates.
   - Add failure pattern and dominant error-code summaries.
   - Keep JSON field names stable and reviewable.
5. Add or update verification:
   - Unit tests for report aggregation and categorization.
   - Corpus parsing tests.
   - Mock or deterministic runner tests for first-pass/repair states.
   - Manual or gated live E2E evidence path for at least one provider.
6. Produce committed preview evidence:
   - Commit a compact scaled eval report.
   - Update preview known limitations or release-note evidence.
   - Identify top 2-3 failure patterns.

Independent work slices:

- Corpus definition can start independently from report aggregation once the
  file shape is agreed.
- Report aggregation tests can be implemented with synthetic results before
  live provider runs.
- Docs/report updates should wait for actual eval output.
- HTTP + SQLite + JSON verification should be implemented after the technical
  spec chooses whether it is a corpus task, a process-level check, or a
  reference-example-backed evidence check.

## Checks

Product proof requires:

- Stage 8 technical spec maps every BDD scenario above to unit, integration,
  E2E, manual, or review evidence.
- The implementation PR changes only approved areas from the technical spec.
- `cargo fmt --check` passes when implementation code changes.
- `cargo clippy --all-targets -- -D warnings` passes when implementation code
  changes.
- `cargo test --all` or the approved narrower Rust test set passes.
- New report aggregation tests cover first-pass success, repair-assisted
  success, unrecovered failure, provider failure, and unavailable usage.
- Corpus tests prove every scaled corpus entry parses and carries required
  metadata.
- A live provider E2E run is attempted under the technical spec's resource
  policy and records either pass/fail evidence or a credential/budget block.
- A committed report exists and is linked from the implementation PR.
- Preview limitations or release evidence are updated from the committed
  report.
- The implementation PR identifies the top 2-3 failure patterns from the run.

Expected artifacts:

- `specs/DUUMBI-689/PRODUCT.md`
- `specs/DUUMBI-689/TECHNICAL.md`
- scaled corpus files or embedded showcases under the technical spec's chosen
  path
- report data type and aggregation updates
- tests for corpus/report behavior
- committed scaled eval report
- preview known-limitations or release-note update

## Open Questions

None blocking for product specification.

Non-blocking implementation questions for Stage 8:

- Whether the HTTP + SQLite + JSON eval can be fully generated by
  provider-backed `intent execute` in the first implementation slice, or
  whether it should be represented as process-level evidence against the
  existing flagship example until richer verification is added.
- Whether token/cost usage can be sourced from existing model telemetry in this
  issue or must be reported as explicitly unavailable for some provider paths.
- Whether the scaled corpus should remain entirely in `src/bench/showcases` or
  move to data files under `docs/e2e` for easier report review.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/689
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/689#issuecomment-4711882257
- Stage 4 triage refill comment:
  https://github.com/hgahub/duumbi/issues/689#issuecomment-4701786462
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Active agentic development map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Active agentic development runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Intent-at-scale inbox note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Intent at Scale Multi-Module and BDD.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Existing intent-create eval script: `scripts/eval_intent.sh`
- Existing benchmark runner and report:
  - `src/bench/runner.rs`
  - `src/bench/report.rs`
  - `src/bench/showcases.rs`
- Benchmark CLI entry points:
  - `src/cli/mod.rs`
  - `src/main.rs`
- Intent execution source:
  - `src/intent/execute.rs`
  - `src/intent/coordinator.rs`
  - `src/intent/verifier.rs`
  - `src/intent/spec.rs`
- Existing benchmark tests: `tests/integration_phase9c.rs`
- Existing benchmark manual protocol: `docs/testing/phase9c-benchmark.md`
- Existing Phase 15 live E2E protocol: `docs/testing/phase15-walkthrough.md`
- Runtime BDD spec artifacts:
  - `specs/DUUMBI-673/PRODUCT.md`
  - `specs/DUUMBI-673/TECHNICAL.md`
- HTTP + SQLite + JSON reference example:
  `examples/flagship-http-sqlite-json/README.md`
