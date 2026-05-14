# DUUMBI-488: Phase 15 E2E Math Library Sample - Technical Specification

## Implementation Objective

Implement the approved DUUMBI-488 product spec by adding a `math-library`
Phase 15 E2E validation task alongside the existing `calculator` and
`string-utils` tasks. The implementation must prove that a fresh workspace can
create, execute, inspect, build, run, and present the Math Library sample
through both CLI and Studio without regressing the completed Phase 15 samples.

This technical spec implements these approved product-spec outcomes:

- `duumbi phase15-e2e math-library --provider <provider> --attempts 1 --output <path>` runs a stable Math Library E2E task.
- The CLI leg creates a fresh workspace, uses a real configured provider, creates the canonical intent, executes it, inspects graph evidence, builds, runs, and writes a structured report.
- Intent execution creates the canonical module `math/lib` and updates `app/main`.
- Graph/report evidence shows `factorial`, `fibonacci`, and `is_prime`.
- `app/main` calls the generated math functions and prints clear representative results:
  - `factorial(10) = 3628800`
  - `fibonacci(15) = 610`
  - `is_prime(97) = true` or `1`
- Studio validates the same CLI-generated workspace through the shared backend, including graph visibility, build, run, `Intents`/`Graph`/`Build` UX checks, Query read-only state, and Agent mode availability.
- Missing credentials, provider timeout/auth/rate-limit issues, compiler defects, graph validation defects, documentation mismatches, and evidence mismatches remain distinguishable.
- `docs/testing/phase15-walkthrough.md` gains a dedicated Math Library section without taking over #489 final all-samples protocol consolidation.

## Agent Audience

Primary implementation agents:

- Codex for local source edits, focused tests, documentation updates, deterministic checks, and GitHub evidence reporting.
- Oz or another cloud runner only when a human explicitly approves a Ralph cycle that needs orchestration-heavy or long-running live-provider validation.

Review and verification agents:

- Codex or Oz for Stage 9 technical-spec review.
- A specialized tester may run live-provider validation after deterministic checks pass and credentials are available.

## Source Context

Verified facts:

- Product spec: `specs/DUUMBI-488/PRODUCT.md`.
- GitHub issue: https://github.com/hgahub/duumbi/issues/488.
- Stage 5 acceptance: https://github.com/hgahub/duumbi/issues/488#issuecomment-4445517611.
- Stage 7 product-spec approval: https://github.com/hgahub/duumbi/issues/488#issuecomment-4449098629.
- Product spec draft PR: https://github.com/hgahub/duumbi/pull/548.
- Related implementation precedent: #487 String Utilities was completed by PR #546, and the current local source includes its descriptor-based Phase 15 harness changes.
- Existing harness: `src/cli/phase15_e2e.rs` defines `Phase15Task`, supports `calculator` and `string-utils`, rejects `math-library`, records `Phase15Report`, creates fresh workspaces, runs provider-backed CLI generation first, then validates Studio against the same CLI-generated workspace.
- Existing task descriptors: `calculator` uses `calculator/ops`; `string-utils` uses `string/utils` and expected functions `reverse`, `count_vowels`, `is_palindrome`.
- Existing CLI help: `src/cli/mod.rs` says hidden `phase15-e2e` accepts `calculator` or `string-utils`.
- Existing benchmark normalization: `src/intent/benchmarks.rs` recognizes `calculator` and `string-utils`, can build fallback specs, returns expected benchmark functions, and returns task-specific guidance for String Utilities.
- Existing intent creation prompt: `src/intent/create.rs` currently models verifier test cases as i64-only and instructs normal user-generated tests not to target `main`.
- Existing intent execution: `src/intent/execute.rs` derives expected exports from test cases and `expected_functions_for_benchmark`, preserves slash-named modules through `module_name_to_relative_path`, verifies intent test cases, and supports known-benchmark guidance.
- Existing algorithmic hints: `src/intent/coordinator.rs` contains hints for fibonacci, factorial, and prime checks. The fibonacci/factorial branch currently uses `if ... else if ...`, so a combined prompt containing both may not emit both hints unless implementation changes this or provides benchmark-specific guidance elsewhere.
- Existing graph/build/run backend: `src/workflow.rs` exposes `graph_evidence`, `build_workspace`, and `run_workspace`; graph evidence recursively scans `.duumbi/graph/**/*.jsonld`.
- Existing Studio API/backend: `crates/duumbi-studio/src/server_fns.rs` discovers nested modules such as `calculator/ops`, wraps shared build/run helpers, and exposes workspace status. `crates/duumbi-studio/src/lib.rs` exposes JSON endpoints including `/api/graph/context`, `/api/build`, and `/api/run`.
- Existing Studio UX evidence: `src/cli/phase15_e2e.rs` verifies footer items `Intents`, `Graph`, `Build`, Query default/read-only state, and Agent mode availability by inspecting SSR HTML.
- Existing Math source material: `docs/e2e/corpus/E08_factorial.gold.yaml`, `docs/e2e/corpus/M01_fibonacci.gold.yaml`, and `docs/e2e/corpus/M06_is_prime.gold.yaml` provide standalone i64 expectations for factorial, fibonacci, and prime checks.
- Existing math operation support: `src/types.rs`, `src/parser/mod.rs`, `src/compiler/lowering.rs`, and tests cover `duumbi:Modulo`, which is relevant for primality.
- Current Phase 15 walkthrough: `docs/testing/phase15-walkthrough.md` is scoped to #486 and #487 and explicitly says not to expand into #488 or #489.
- Repo instructions: `AGENTS.md`, `docs/architecture.md`, and `docs/coding-conventions.md` require graph-first transformations, Query mode read-only by default, provider setup through `/provider`, no `.unwrap()` in library code, doc comments for public items, zero-warning clippy, and focused state-machine/manual smoke checks for REPL/TUI UX changes.
- Relevant Obsidian notes inspected:
  - `DUUMBI - PRD`: DUUMBI should expose intent, structure, executable behavior, and evidence as connected artifacts.
  - `DUUMBI - Development Intake to Delivery Workflow`: Stage 8 creates a technical spec only after product approval and requires bounded Ralph cycles for implementation.
  - `DUUMBI Agentic Development Map`: agents should ask read-only questions first, create technical plans with affected files/risks/tests/evidence, and use CI/review/human verification before merge.
  - `DUUMBI - Product Roadmap 2026-05`: Phase 15 should close before broader surface expansion; Math Library is one remaining sample before #489.
  - `DUUMBI - Phase 15 - Studio Workflow Redesign`: the three sample tasks are the kill-criterion validation set for CLI REPL and Studio.
  - `DUUMBI - Service and Research Direction`: Studio E2E workflow remains partial until evidence is strong enough for public-facing claims.
  - `DUUMBI - Glossary`: agent skills are reusable operational playbooks.

Assumptions:

- The technical-spec PR may be stacked on the product-spec branch while PR #548 remains open; the artifact path and issue comment must make the dependency clear.
- Any configured supported provider remains acceptable for the live command. MiniMax may remain the documented example because the Phase 15 walkthrough currently uses `MINIMAX_API_KEY`.
- Studio should continue to validate the CLI-generated workspace rather than spending a second live provider-backed mutation.
- The first implementation should prefer deterministic Math Library benchmark normalization unless inspection proves the current generic prompt path already creates stable `math/lib`, functions, test cases, and report evidence.
- True static proof that `fibonacci` is implemented with a literal loop is not required for the initial pass. The required evidence is deterministic function behavior, `math/lib` function presence, cross-module calls from `app/main`, and representative output. If a reviewer requires literal iteration evidence, that should be handled as a Stage 9 or Ralph-cycle approval decision.

## Affected Areas

Expected source changes:

- `src/cli/phase15_e2e.rs`
  - Add a `math-library` task descriptor with canonical intent, module path `math/lib`, expected functions `factorial`, `fibonacci`, `is_prime`, task-specific output predicate, display name, and failure module.
  - Update task lookup tests so `math-library` is supported and unsupported-task guidance lists `calculator`, `string-utils`, and `math-library`.
  - Add Math Library output predicate tests for the representative result lines.
  - Add graph/module evidence tests for `math/lib`.
  - Ensure report evidence keys remain deterministic, for example `module_math_lib_exists=true`, `describe_contains_factorial=true`, `describe_contains_fibonacci=true`, `describe_contains_is_prime=true`, `graph_has_math_lib=true`, `stdout=...`.
  - Update Ralph Gate wording so it does not hardcode stale Calculator or #486 language for every task.

- `src/cli/mod.rs`
  - Update hidden `phase15-e2e` task help text to list `calculator`, `string-utils`, and `math-library`.

- `src/intent/benchmarks.rs`
  - Add Math Library benchmark recognition for prompts containing enough of `math library`, `factorial`, `fibonacci`, and `prime`/`is_prime`.
  - Add deterministic normalization to create `math/lib`, modify `app/main`, and set i64-compatible test cases for the expected functions.
  - Add `MATH_LIBRARY_EXPECTED_FUNCTIONS` to `expected_functions_for_benchmark`.
  - Add Math Library guidance only if needed to keep provider-generated graph mutations inside supported DUUMBI operations and canonical sample behavior.

- `src/intent/execute.rs`
  - No broad redesign expected.
  - Existing `expected_functions_for_benchmark` integration should pick up Math Library functions after `benchmarks.rs` is extended.
  - Modify only if implementation proves create-module retry, benchmark guidance, or `main` handling needs a small extension.

- `src/intent/coordinator.rs`
  - Optional but likely: make math algorithmic hints accumulate so combined prompts can include factorial, fibonacci, and prime guidance together, or rely on explicit Math Library benchmark guidance if that produces better-scoped behavior.

- `docs/testing/phase15-walkthrough.md`
  - Update top-level issue/scope text to include #488 while keeping #489 out of scope.
  - Add a dedicated Math Library protocol section with CLI, Studio, automated harness command, pass criteria, missing-credential behavior, provider guidance, and evidence expectations.
  - Do not consolidate the final all-samples protocol; #489 owns that.

Expected test changes:

- Unit tests in `src/cli/phase15_e2e.rs` for task lookup, unsupported task guidance, Math Library output predicate, function evidence, graph module evidence, and task-specific Ralph Gate text.
- Unit tests in `src/intent/benchmarks.rs` for Math Library matching, normalization, expected functions, deterministic fallback spec, and optional guidance.
- Existing or new unit tests in `src/intent/execute.rs` if a Math Library benchmark-specific create-module retry path needs direct coverage.
- Existing or new tests in `src/intent/coordinator.rs` if algorithmic hint accumulation is changed.
- Existing Studio nested module tests may be extended to include `math/lib` only if implementation changes module discovery behavior. Do not add redundant tests if current generic nested-module coverage already proves the contract.

Expected generated/local artifacts during validation:

- `/tmp/duumbi-phase15-math-library-report.json` for local live evidence.
- Temporary workspaces under the OS temp directory, created by the harness.
- No generated reports should be committed unless a later approved docs decision explicitly asks for a checked-in sample artifact.

CI/check paths:

- `cargo fmt --check`
- `cargo test phase15`
- `cargo test benchmarks`
- Any focused `cargo test` command for changed `intent::execute` or `intent::coordinator` tests.
- `cargo test -p duumbi-studio --features ssr` if Studio module discovery/API code changes.
- `cargo test --all`
- `cargo clippy --all-targets -- -D warnings`

## Technical Approach

### 1. Extend The Phase 15 Task Descriptor Model

Add a Math Library descriptor to the existing `PHASE15_TASKS` array rather than creating a second harness path.

Recommended descriptor shape:

```rust
const MATH_LIBRARY_INTENT: &str = "Build a math library with: factorial (recursive), fibonacci (iterative), and is_prime functions. The main function should compute factorial(10), fibonacci(15), and check if 97 is prime.";
const MATH_LIBRARY_FUNCTIONS: &[&str] = &["factorial", "fibonacci", "is_prime"];

Phase15Task {
    id: "math-library",
    display_name: "Math Library",
    intent: MATH_LIBRARY_INTENT,
    module_path: "math/lib",
    expected_functions: MATH_LIBRARY_FUNCTIONS,
    output_check: output_mentions_math_library_results,
    failure_module: "math/lib",
}
```

The output predicate should normalize whitespace, casing, and optional boolean representation while still requiring all three representative checks. It should accept labeled equivalents but reject output that omits any operation.

Minimum output evidence:

- factorial label and `3628800`
- fibonacci label and `610`
- `is_prime`/prime label, `97`, and `true` or `1`

Do not require `main` exit code `0` as the only success signal unless the normalized benchmark explicitly makes that stable. Preserve the existing harness behavior that treats stdout evidence as the product-facing pass signal, while recording `run_exit_code` for review.

### 2. Add Deterministic Math Library Benchmark Normalization

Add a known benchmark rule in `src/intent/benchmarks.rs` unless Stage 10 inspection proves it is unnecessary. The recommended path is to add it because #488 requires strict module/function naming, cross-module calls, and repeatable evidence.

Matching predicate:

- Match prompts that contain enough of:
  - `math`
  - `library`
  - `factorial`
  - `fibonacci`
  - `prime` or `is_prime`

Normalization:

- Set `modules.create = ["math/lib"]`.
- Ensure `modules.modify` contains `app/main`.
- Set acceptance criteria that require:
  - `factorial(n)` returns correct i64 factorial values for representative non-negative inputs.
  - `fibonacci(n)` returns correct i64 Fibonacci values for representative inputs.
  - `is_prime(n)` returns `1` for prime and `0` for non-prime values.
  - `main` calls `math/lib::factorial`, `math/lib::fibonacci`, and `math/lib::is_prime`, prints labeled lines for the three canonical representative results, and returns a stable value.
- Set i64 verifier test cases that prove functions, not just `app/main`, for example:
  - `factorial(0) = 1`
  - `factorial(1) = 1`
  - `factorial(5) = 120`
  - `factorial(10) = 3628800`
  - `fibonacci(0) = 0`
  - `fibonacci(1) = 1`
  - `fibonacci(2) = 1`
  - `fibonacci(10) = 55`
  - `fibonacci(15) = 610`
  - `is_prime(1) = 0`
  - `is_prime(2) = 1`
  - `is_prime(4) = 0`
  - `is_prime(97) = 1`

Reasoning:

- The current verifier is already i64-oriented, so Math Library can use normal test cases without extending the schema.
- Verifier tests protect against a trivial `app/main` that prints hardcoded results while leaving unusable functions.
- Harness stdout checks still prove the user-facing demonstration required by the product spec.

Guidance recommendation:

- Add task-specific benchmark guidance if live attempts otherwise drift into unsupported graph shapes or non-canonical modules.
- Guidance should prefer supported operations: `Const`, `Load`, `Compare`, `Branch`, `Call`, `Add`, `Sub`, `Mul`, `Div`, `Modulo`, `Print`, `PrintString`, `StringConcat`, `StringFromI64`, and `Return`.
- If current graph/control-flow support makes literal iterative Fibonacci unreliable, allow recursive Fibonacci or bounded repeated computation while recording that the representative validation remains behavioral, not static algorithm-shape proof.

Rejected alternatives:

- Do not extend `IntentSpec::TestCase` beyond i64 for #488.
- Do not introduce a new generic math standard library.
- Do not add a static algorithm-shape validator for recursion/iteration unless Stage 9 or an approved Ralph cycle explicitly requires it.
- Do not run Studio as a second provider-backed mutation path.

### 3. Validate CLI Evidence

The CLI leg should:

1. Create a fresh temp workspace.
2. Write provider config without logging secrets.
3. Seed Phase 15 learning cache using the task/provider-specific cache path.
4. Create and execute the canonical Math Library intent.
5. Inspect graph evidence and/or `describe` output.
6. Confirm `.duumbi/graph/math/lib.jsonld` exists.
7. Confirm graph/source evidence contains `factorial`, `fibonacci`, and `is_prime`.
8. Build through `crate::workflow::build_workspace`.
9. Run through `crate::workflow::run_workspace`.
10. Check stdout for all three representative results.
11. Record pass/fail, elapsed time, generated slug, workspace, build/run evidence, stdout, and failure category.

Recommended evidence keys:

- `module_math_lib_exists=true`
- `describe_contains_factorial=true`
- `describe_contains_fibonacci=true`
- `describe_contains_is_prime=true`
- `run_exit_code=<n>`
- `stdout=<truncated output>`
- `seeded_learning_records=<n>`
- `harvested_learning_records=<n>`

Failure categories should reuse existing categories where possible:

- `missing_provider_credentials`
- `provider_timeout`
- `provider_auth`
- `provider_rate_limit`
- `provider_server_error`
- `provider_or_intent_error`
- `mutation_failed`
- `describe_failed`
- `build_failed`
- `run_failed`
- `evidence_mismatch`
- `docs_mismatch`
- `studio_ux_failed`
- `studio_http_failed`
- `skipped_cli_failed`

Add a new category only if an observed failure cannot be represented without ambiguity.

### 4. Validate Studio Against The Same Workspace

Keep the established shared-backend pattern:

- Start Studio only after the CLI leg passes.
- Use the CLI-generated workspace as the Studio working directory.
- Do not spend a second live provider-backed mutation.
- Use the existing HTTP flow:
  - GET `/`
  - GET `/api/graph/context`
  - POST `/api/build`
  - POST `/api/run`
- Require graph modules to include `math/lib`.
- Require build JSON `ok == true`.
- Require run stdout to satisfy the Math Library output predicate.
- Keep the existing UX checks:
  - footer labels exactly `Intents`, `Graph`, `Build`
  - Query default active
  - Query mode exposes read-only title/state
  - Agent mode is available

If `GET /api/graph/context` continues to return module context through `load_initial_data`, do not redesign the Studio API. This issue validates the shared backend, not a new graph endpoint contract.

### 5. Update Documentation Without Performing #489

Update `docs/testing/phase15-walkthrough.md` with a dedicated #488 section:

- CLI manual path.
- Studio shared-backend path.
- Automated harness command:

```bash
$DUUMBI phase15-e2e math-library \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-math-library-report.json
```

- Pass criteria for `math/lib`, functions, build, run, stdout, Studio module visibility, and UX checks.
- Missing credential behavior with `missing_provider_credentials`.
- Provider failure classification guidance.
- Evidence report path for local validation.

Do not rewrite the document into the final cross-sample protocol. Leave final consolidation, cross-sample comparison, and public-facing completion language to #489.

## Invariants

- Preserve existing `calculator` and `string-utils` behavior, report shape, task ids, output predicates, and walkthrough sections.
- Keep Query mode read-only by default; mutation remains Agent mode or explicit Intent execution.
- Do not log raw provider secrets in console output, docs, reports, GitHub comments, or learning records.
- Missing credentials must produce structured blocked evidence, not a panic or ambiguous failure.
- Provider failures must stay distinguishable from deterministic code, graph, compiler, Studio, documentation, and evidence defects.
- Slash-named module paths must remain nested paths on disk: `math/lib` maps to `.duumbi/graph/math/lib.jsonld`.
- Graph discovery must keep recursively scanning `.duumbi/graph/**/*.jsonld`.
- `app/main` must call or otherwise depend on functions from `math/lib`; a direct print-only main must not satisfy the sample.
- Function names are strict for the default pass: `factorial`, `fibonacci`, and `is_prime`. If implementation accepts equivalents, the report must record the mapping and Stage 11 must judge it explicitly.
- Live provider validation requires explicit Ralph-cycle approval before any paid/API attempt.
- No implementation cycle may expand into #489 final protocol consolidation, Phase 16, Phase 13, provider model-selection UI, marketing/GTM work, or new general-purpose math stdlib design.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. ask for explicit approval before starting
6. implement only the approved goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop if requirements are met or request approval for the next cycle

No Ralph cycle may start from this technical spec alone. Stage 9 approval and an explicit cycle approval comment are required first.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle:
  - Deterministic harness/benchmark cycle: up to 4 source modules plus focused tests.
  - Documentation cycle: `docs/testing/phase15-walkthrough.md` only, unless evidence forces a small linked correction.
  - Live validation cycle: no planned source edits; one provider-backed run only.
- Expected command budget:
  - Focused implementation cycle: `cargo fmt --check`, `cargo test phase15`, `cargo test benchmarks`, and one or two exact focused tests for touched modules.
  - Broader readiness cycle: `cargo test --all`, `cargo test -p duumbi-studio --features ssr` if Studio code changed, and `cargo clippy --all-targets -- -D warnings`.
  - Live validation cycle: `cargo build`, `cargo build -p duumbi-studio --features ssr`, and exactly one approved `phase15-e2e math-library` command.
- Approval required before every cycle: yes.
- When to stop and ask for human guidance:
  - A cycle needs to change implementation code outside the planned file areas.
  - The work would require broad verifier schema changes, general loop semantics, a new math stdlib, or new Studio navigation.
  - More than one live provider attempt is needed.
  - Product requirements conflict, especially strict iterative Fibonacci versus currently practical graph behavior.
  - The implementation would weaken Query mode safety, provider secret handling, or existing #486/#487 evidence.
  - #489 final protocol consolidation becomes necessary to make #488 pass.

Recommended initial cycles:

1. Add deterministic Math Library descriptor, output predicate, benchmark normalization, expected functions, guidance as needed, and focused unit tests.
2. Add or refine coordinator/execute support only if focused tests or local dry inspection prove the benchmark path misses factorial/fibonacci/prime functions or guidance.
3. Update the Phase 15 walkthrough with the Math Library section and evidence instructions.
4. Run broader deterministic checks and fix only direct regressions.
5. Request a separate live-provider validation cycle for exactly one `math-library` attempt.

## Task Breakdown

1. Confirm the issue remains in the correct workflow state and product/technical specs are linked before implementation starts.
2. Inspect the current `Phase15Task` descriptor model and tests.
3. Add `math-library` descriptor and output predicate in `src/cli/phase15_e2e.rs`.
4. Update supported-task help text in `src/cli/mod.rs`.
5. Add focused harness tests for lookup, unsupported-task messaging, output matching, function evidence, module evidence, and task-specific Ralph Gate wording.
6. Add Math Library benchmark normalization in `src/intent/benchmarks.rs` with deterministic i64 test cases and expected functions.
7. Add focused benchmark tests for matching, normalization, deterministic fallback spec, expected functions, and optional guidance.
8. Inspect whether existing `src/intent/execute.rs` expected-export integration already covers Math Library after benchmark extension. If yes, leave it unchanged.
9. If implementation changes algorithmic hints, add a focused `src/intent/coordinator.rs` test proving combined factorial/fibonacci/prime prompts get enough guidance.
10. Update the Phase 15 walkthrough with Math Library CLI, Studio, automated harness, pass criteria, report path, and missing credential behavior.
11. Run focused deterministic checks.
12. Run broader readiness checks.
13. Request and run one live-provider validation cycle only after explicit approval and available credentials.
14. Open or update the implementation PR only after completion criteria and approved evidence are available.

## Verification Plan

Spec-only Stage 8 validation:

- `git diff --check`
- no implementation tests required for this technical-spec-only PR

Implementation verification:

- `cargo fmt --check`
- `cargo test phase15`
- `cargo test benchmarks`
- Focused tests for any changed `src/intent/execute.rs` or `src/intent/coordinator.rs` helpers.
- `cargo test -p duumbi-studio --features ssr` if Studio code or module discovery tests change.
- `cargo test --all`
- `cargo clippy --all-targets -- -D warnings`

Live/manual verification after explicit approval:

```bash
cargo build
cargo build -p duumbi-studio --features ssr
$DUUMBI phase15-e2e math-library \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-math-library-report.json
```

Provider may be changed to any configured supported provider if the approving comment names it. Do not switch provider or retry without a new approval.

Live report checks:

- Top-level `task` is `math-library`.
- CLI leg passes or reports a structured provider/credential class.
- CLI evidence includes workspace path, generated slug, `module_math_lib_exists=true`, all three `describe_contains_*` keys, build/run evidence, stdout, elapsed time, and failure category when applicable.
- Studio leg runs only after CLI pass.
- Studio evidence includes `shared_backend_workspace=true`, `graph_has_math_lib=true`, build output path, run stdout, `ux_footer_items=Intents,Graph,Build`, `ux_query_default_active=true`, `ux_query_read_only=true`, and `ux_agent_mode_available=true`.
- Output evidence includes `factorial(10)=3628800`, `fibonacci(15)=610`, and `is_prime(97)=true` or `1`.
- Report distinguishes provider timeout/auth/rate-limit/server errors from compiler, graph validation, verifier, Studio, docs, and evidence mismatches.
- Walkthrough commands and pass criteria match the implemented behavior.

## Completion Criteria

The implementation is ready for PR review only when all of these are true:

- `phase15-e2e` accepts `math-library` and unsupported-task errors list all supported tasks.
- Existing `calculator` and `string-utils` tests and behavior remain supported.
- The harness checks `math/lib`, `factorial`, `fibonacci`, `is_prime`, and all representative output results.
- Known benchmark handling creates or preserves `math/lib`, `app/main`, canonical expected functions, and i64 verifier test cases for Math Library.
- `app/main` in the generated workspace calls `math/lib` functions rather than satisfying the sample only with direct hardcoded output.
- Studio validates the same CLI-generated workspace through shared build/run APIs and graph module evidence.
- The Phase 15 walkthrough includes Math Library instructions and explicitly leaves #489 final protocol consolidation out of scope.
- Required deterministic checks pass or every failure is explained with an approved follow-up cycle request.
- One live-provider run has either passed with CLI and Studio evidence or is blocked by structured missing-credential/provider evidence accepted by the reviewer.
- No product spec, technical spec approval, generated report, runtime asset, implementation branch policy, or unrelated workflow file was changed outside approved scope.

## Failure And Escalation

- If focused tests fail, report the failing command, first relevant error, suspected cause, and the smallest next approved fix. Do not broaden scope automatically.
- If `cargo test --all` or clippy exposes unrelated failures, record them separately and ask whether to handle them in this issue or defer.
- If provider credentials are missing, record `missing_provider_credentials` and the missing environment variable name only. Do not ask the user to paste secrets into chat, docs, issue comments, or reports.
- If a live provider attempt times out or fails with auth/rate-limit/server errors, stop after the approved attempt and report provider evidence. Do not switch providers or retry without approval.
- If generated graph output uses semantically equivalent function names instead of strict names, mark the harness result as failed unless an approved cycle explicitly adds report-mapped equivalence handling.
- If generated output hardcodes only `app/main` output and lacks usable `math/lib` functions, classify as `evidence_mismatch` or verifier failure, not success.
- If strict iterative Fibonacci becomes a blocker, ask the human whether behavioral Fibonacci evidence is acceptable for #488 or whether to add static algorithm-shape validation at additional cost.
- If #489 consolidation appears necessary, stop. #489 is a separate issue and must not be folded into #488.
- If implementation touches provider setup UX, Query/Agent safety, broad Studio navigation, verifier schema, runtime assets, or compiler semantics beyond direct Math Library needs, stop and request a new scoped approval.

## Open Questions

No blocking questions.

Non-blocking items for Stage 9 or Ralph-cycle approval:

- Whether reviewers want strict static proof of iterative Fibonacci, or whether behavioral `fibonacci(15)=610` plus function/verifier evidence is enough for the Phase 15 sample.
- Whether a successful live run must use MiniMax for consistency with the walkthrough, or any configured supported provider is acceptable.
- Whether semantically equivalent generated function names should ever be accepted if the report records a mapping. The default technical recommendation is strict names only.
