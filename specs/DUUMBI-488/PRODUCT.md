# DUUMBI-488: Phase 15 E2E Math Library Sample

## Summary

Add product-level support for proving the third Phase 15 representative
end-to-end workflow: a fresh user can create, execute, inspect, build, and run
the Math Library sample through the CLI REPL and Duumbi Studio with a real
configured LLM provider.

The stable task id should be `math-library`. The canonical user intent is:

```text
Build a math library with: factorial (recursive), fibonacci (iterative), and is_prime functions. The main function should compute factorial(10), fibonacci(15), and check if 97 is prime.
```

This is the final engineering-heavy Phase 15 sample before #489 can consolidate
the all-samples walkthrough. It should preserve the already validated
Calculator and String Utilities behavior while adding a more complex math sample
that exercises branching, recursion or repeated computation, cross-module calls,
build/run evidence, and Studio graph/build/run presentation.

## Problem

Phase 15 claims that DUUMBI can support representative intent-driven
development workflows across CLI REPL and Studio. Calculator has evidence, and
String Utilities was completed by #487, but Phase 15 still lacks evidence for a
moderate cross-module math workflow.

Without this sample, DUUMBI can claim simple arithmetic and scoped string demo
coverage, but not a stronger workflow that combines multiple functions, numeric
control flow, cross-module calls, and deterministic run-output checks. That
would leave the Phase 15 kill criterion partially unproven and would make #489
premature.

## Outcome

When this is done:

- `duumbi phase15-e2e math-library --provider <provider> --attempts 1 --output <path>` or an equivalent stable task naming path runs the Math Library E2E validation.
- A fresh CLI workspace can create, execute, describe, build, and run the canonical Math Library intent using a configured provider.
- Intent execution creates the canonical module `math/lib` and updates `app/main`.
- Graph evidence shows `factorial`, `fibonacci`, and `is_prime`, or report-recorded semantically equivalent generated names if the implementation accepts equivalents.
- The built program demonstrates all three functions with deterministic representative results:
  - `factorial(10) = 3628800`
  - `fibonacci(15) = 610`
  - `is_prime(97) = true` or `1`
- Studio validates the same generated workspace through the shared backend, including `Intents`, `Graph`, and `Build`, nested graph discovery, build, run, and read-only Query mode behavior.
- The Phase 15 report records CLI evidence, Studio evidence, elapsed time, UX checks, failure category, provider guidance, and Ralph Gate guidance.
- Missing provider credentials, provider timeout, provider error, compiler defects, graph validation defects, documentation mismatches, and evidence mismatches are distinguishable in the report.
- `docs/testing/phase15-walkthrough.md` includes a dedicated Math Library section without turning #488 into the final all-samples protocol for #489.

## Scope

### In Scope

- Add Math Library as a stable Phase 15 E2E task alongside Calculator and String Utilities.
- Preserve existing Calculator and String Utilities task behavior and report shape.
- Use `math-library` as the stable task id.
- Use `math/lib` as the canonical generated module path.
- Generate or validate functions named `factorial`, `fibonacci`, and `is_prime`.
- Update `app/main` to call all three functions and print clear, human-readable result lines.
- Validate CLI create, execute, describe or equivalent graph evidence, build, and run.
- Validate Studio graph visibility, build endpoint, run endpoint, shared-backend workspace behavior, and the simplified three-panel workflow.
- Validate Query mode remains read-only and mutation still requires Agent mode or explicit Intent execution.
- Add or extend known benchmark normalization only if needed to make this Phase 15 sample deterministic enough for repeatable evidence.
- Update `docs/testing/phase15-walkthrough.md` with Math Library setup, commands, pass criteria, provider behavior, and evidence expectations.
- Add focused automated tests for harness/report behavior that do not require live provider credentials.

### Explicitly Out Of Scope

- Final all-samples E2E protocol consolidation, which belongs to #489 except for the Math Library section required here.
- Phase 16 Windows and cross-platform work.
- Phase 13 telemetry or self-healing work.
- Marketing or GTM content updates.
- New general-purpose math standard library design beyond what is necessary to validate this generated user module.
- Broad Studio navigation, chat, provider setup, or model-selection product behavior unrelated to proving this sample.
- Product spec approval, technical spec creation, Ralph-cycle authorization, or implementation work as part of this Stage 6 artifact.

## Constraints And Assumptions

Facts:

- #488 is the accepted canonical execution issue for the Phase 15 Math Library sample.
- Stage 5 acceptance exists and routes #488 to `Spec Needed`.
- #486 Calculator is complete.
- #487 String Utilities is complete.
- #489 remains open and depends on #486, #487, and #488.
- The current Phase 15 harness supports `calculator` and `string-utils`; it explicitly rejects `math-library` as unsupported.
- `docs/testing/phase15-walkthrough.md` is currently scoped to Calculator and String Utilities and says not to expand into #488 or #489.
- Source context already contains relevant algorithm hints for factorial, fibonacci, and prime checks in `src/intent/coordinator.rs`.
- Existing E2E corpus material covers standalone factorial, fibonacci, and is_prime tasks.

Assumptions:

- The product goal is representative validation, not a fully general math package.
- The canonical module path should be strict enough for the harness to check: `math/lib`.
- Function names should be strict by default: `factorial`, `fibonacci`, and `is_prime`.
- If implementation accepts semantically equivalent names, the report must record the mapping and Stage 7/Stage 11 review should be able to judge it.
- Any configured supported provider is acceptable for the command, but the walkthrough may keep MiniMax as the documented example for consistency with prior Phase 15 evidence.
- Provider-backed CLI execution remains the live mutation gate; Studio should validate graph/build/run and UX against the CLI-generated workspace through the shared backend instead of spending a second live provider mutation.

Constraints:

- Do not weaken Query/Agent/Intent mode safety: Query mode must remain read-only.
- Do not log raw provider secrets in docs, reports, console output, or evidence files.
- Missing credentials must produce structured `missing_provider_credentials` evidence, not a panic or ambiguous failure.
- Provider timeout or provider instability must be classified separately from deterministic code, graph, compiler, docs, and evidence defects.
- Nested module path handling must preserve slash-named modules on disk and in graph discovery.
- The report must preserve the distinction between graph existence, function evidence, build success, run success, and output correctness.
- The sample must not be marked passing if it only hardcodes final output in `app/main` without usable `math/lib` function evidence.

## Decisions

- **Decision:** #488 is the canonical Math Library execution issue.
  **Evidence:** Stage 4 triage normalized #488 and found it to be the remaining Phase 15 Math Library issue.

- **Decision:** The issue is accepted for specification.
  **Evidence:** Stage 5 Human Acceptance Decision comment on #488 records `Decision: Accept` and `Next state: Spec Needed`.

- **Decision:** This spec is file-based.
  **Evidence:** The work is user-visible, cross-surface, non-trivial, likely to need review iteration, and useful as durable implementation context for Stage 8 and Stage 11.

- **Decision:** The stable task id is `math-library`.
  **Evidence:** #488 acceptance criteria use `duumbi phase15-e2e math-library`.

- **Decision:** The canonical module path is `math/lib`.
  **Evidence:** #488 names `math/lib` as the expected module, and strict module evidence is needed for a reliable Phase 15 sample.

- **Decision:** #488 should not consolidate the final all-samples protocol.
  **Evidence:** #489 is the separate final protocol issue and depends on #488.

## Behavior

### Defaults

- The stable task id is `math-library`.
- The canonical module path is `math/lib`.
- The canonical functions are `factorial`, `fibonacci`, and `is_prime`.
- The provider argument works consistently with the existing Phase 15 harness.
- Attempt count, output path, Studio port, report structure, timeout behavior, and Ralph Gate guidance remain consistent with the existing harness unless Stage 8 identifies a necessary adjustment.

### Inputs

- Required task id: `math-library`.
- Required provider selection: the existing provider argument path.
- Optional attempts count.
- Optional report output path.
- Optional Studio port if already supported by the harness.
- Provider credential through the existing provider-specific environment variable or configured provider path.
- Canonical intent text:

```text
Build a math library with: factorial (recursive), fibonacci (iterative), and is_prime functions. The main function should compute factorial(10), fibonacci(15), and check if 97 is prime.
```

### Outputs

- JSON report with task, provider, attempts, per-attempt CLI and Studio evidence, performance, UX checks, failure category, and Ralph Gate guidance.
- CLI evidence including workspace path, intent slug, module checks, function checks, build output path, run result, stdout/stderr, elapsed time, seeded/harvested learning counts when available, and whether `math/lib` exists.
- Studio evidence including shared backend workspace use, graph contains `math/lib`, build endpoint success, run endpoint success, footer workflow items, Query mode read-only state, and Agent mode availability.
- Documentation updates in `docs/testing/phase15-walkthrough.md` that explain how to run and judge the Math Library sample.

### Success States

- The CLI leg creates a fresh workspace, creates the canonical intent, executes it, finds `math/lib`, finds `factorial`, `fibonacci`, and `is_prime`, builds successfully, runs successfully, and demonstrates the required representative results.
- The Studio leg uses the CLI-generated workspace, confirms the three-panel workflow remains `Intents`, `Graph`, `Build`, sees `math/lib`, builds successfully, runs successfully, and confirms Query mode remains read-only.
- The report is written to the requested output path and contains enough evidence for a human reviewer to decide whether the sample passed.

### Representative Correct Results

The sample must prove all three operations:

- `factorial(10)` returns `3628800`.
- `fibonacci(15)` returns `610`.
- `is_prime(97)` returns true or `1`.

Additional edge checks may be included in implementation or tests, such as
`factorial(0) = 1`, `fibonacci(0) = 0`, `fibonacci(1) = 1`,
`is_prime(1) = false/0`, and `is_prime(4) = false/0`, but they are not required
for the product-level pass unless Stage 8 chooses to add them.

### Empty And Error States

- Unknown task ids should return a clear unsupported-task error that lists supported task ids, including `math-library` after this work is implemented.
- Missing provider credentials should report `missing_provider_credentials` with the missing environment variable name and no secret value.
- Provider authentication, rate-limit, network, timeout, and malformed-output failures should be classified separately where detectable.
- Build failures should include structured build evidence and should not be misclassified as provider failures once graph generation has completed.
- Run failures should distinguish missing binary, process launch failure, nonzero process exit with otherwise correct stdout, and output mismatch.
- Missing `math/lib`, missing expected functions, missing cross-module calls, or incorrect representative outputs should be deterministic evidence failures.

### Retry, Timeout, And Cancellation

- Provider-backed CLI mutation should keep the existing bounded timeout model.
- A timeout must write structured evidence and harvest any available sanitized learning records.
- Attempts should be independent fresh workspaces while allowing the existing learning cache behavior to seed later attempts.
- User interruption should leave any partial report or console evidence in a state that does not imply a pass.

### Race Conditions And Invariants

- Studio validation must not start until the CLI leg has passed and reported a reusable workspace.
- Graph discovery must recursively include nested modules under `.duumbi/graph/**/*.jsonld`.
- Slash-named module paths must remain slash paths on disk, not flattened names.
- The report should preserve the distinction between generated graph existence, function evidence, build success, run success, and output correctness.
- `app/main` must call or otherwise depend on functions from `math/lib`; a direct print-only demonstration in `app/main` must not satisfy the sample by itself.
- Query mode must not mutate the workspace; mutation remains Agent mode or explicit Intent execution.

### Accessibility And Focus Rules

- The three primary Studio workflow items must remain visible and keyboard-reachable as `Intents`, `Graph`, and `Build`.
- Query and Agent mode controls must remain distinguishable to keyboard and screen-reader users through existing labels/states.
- Build and run results must remain exposed as readable text, not only transient visual state.

## Tasks

- Add a Math Library task descriptor to the Phase 15 harness with the canonical prompt, task id, module path, expected function names, output predicate, and failure-module label.
- Preserve existing Calculator and String Utilities task descriptors and tests.
- Add known benchmark normalization for the Math Library prompt only if provider variability prevents reliable evidence.
- Define Math Library acceptance criteria and test cases for intent normalization if benchmark normalization is added.
- Extend CLI leg evidence checks for `math/lib`, `factorial`, `fibonacci`, `is_prime`, build output, run stdout, and failure categories.
- Extend Studio leg validation to check `math/lib` against the shared backend workspace, without triggering a second live provider mutation.
- Ensure nested graph path checks work for `.duumbi/graph/math/lib.jsonld`.
- Update report aggregation, performance, UX checks, and Ralph Gate guidance to account for the selected task.
- Add focused tests for supported/unsupported task ids, missing provider credentials, report serialization, Math Library evidence classification, and nested module path checks.
- Update `docs/testing/phase15-walkthrough.md` with Math Library commands, pass criteria, report examples, provider credential behavior, and evidence expectations.
- Run regression checks and the live provider command when credentials are available.

Independent work:

- Harness task descriptor addition and unsupported-task tests.
- Math Library output predicate and report tests.
- Documentation update.
- Optional benchmark-normalization tests.

Sequential work:

- Live provider CLI validation before Studio shared-backend validation.
- Documentation evidence log after a local report path exists.
- #489 final protocol consolidation only after this issue has real validation evidence.

## Checks

- `cargo fmt --check`
- `cargo test --all`
- `cargo test -p duumbi-studio --features ssr`
- `cargo clippy --all-targets -- -D warnings`
- Focused harness tests for:
  - `calculator` remains supported.
  - `string-utils` remains supported.
  - `math-library` is supported.
  - unsupported task ids return a clear error listing all supported tasks.
  - missing provider credentials produce `missing_provider_credentials`.
  - report JSON includes task-specific CLI evidence, Studio evidence, elapsed time, UX checks, failure category, and Ralph Gate guidance.
  - nested module path checks preserve `math/lib`.
  - Math Library output evidence recognizes `factorial(10)=3628800`, `fibonacci(15)=610`, and `is_prime(97)=true` or `1`.
- Live/manual validation when credentials are available:

```bash
cargo build
cargo build -p duumbi-studio --features ssr
export MINIMAX_API_KEY="..."
./target/debug/duumbi phase15-e2e math-library \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-math-library-report.json
```

Pass criteria:

- The report result passes or, if blocked, clearly identifies a provider/credential class rather than a deterministic code/docs mismatch.
- CLI evidence includes a fresh workspace, generated slug, `math/lib`, the three math functions, build path, run evidence, elapsed time, and failure category if any.
- Studio evidence includes shared backend workspace, `math/lib` graph visibility, build success, run success, `Intents`/`Graph`/`Build` UX checks, Query read-only state, and Agent mode availability.
- The walkthrough documents the protocol and at least one local evidence report path for this sample.

## Open Questions

None blocking for product specification.

Non-blocking items for Stage 8:

- Whether to require strict function names only, or allow semantically equivalent names with explicit report mapping.
- Whether Math Library needs deterministic benchmark normalization, and how much normalization is acceptable before it weakens live-provider evidence.
- Whether the implementation should prove iterative fibonacci literally, or may use recursive/repeated-computation behavior if current graph support makes true iteration inappropriate.
- Whether `is_prime(97)` should be represented as true/false or `1`/`0` in output checks.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/488
- Stage 5 Human Acceptance Decision: https://github.com/hgahub/duumbi/issues/488#issuecomment-4445517611
- Calculator precedent: https://github.com/hgahub/duumbi/issues/486
- String Utilities precedent: https://github.com/hgahub/duumbi/issues/487
- String Utilities implementation PR: https://github.com/hgahub/duumbi/pull/546
- Qualified cross-module call prerequisite PR: https://github.com/hgahub/duumbi/pull/542
- Final protocol documentation separate scope: https://github.com/hgahub/duumbi/issues/489
- Existing String Utilities product spec: `specs/DUUMBI-487/PRODUCT.md`
- Phase 15 walkthrough: `docs/testing/phase15-walkthrough.md`
- Phase 15 harness: `src/cli/phase15_e2e.rs`
- Known benchmark normalization: `src/intent/benchmarks.rs`
- Algorithmic hints: `src/intent/coordinator.rs`
- Core operation types: `src/types.rs`
- Factorial E2E corpus: `docs/e2e/corpus/E08_factorial.gold.yaml`
- Fibonacci E2E corpus: `docs/e2e/corpus/M01_fibonacci.gold.yaml`
- Prime E2E corpus: `docs/e2e/corpus/M06_is_prime.gold.yaml`
- DUUMBI PRD: `DUUMBI - PRD`
- DUUMBI Glossary: `DUUMBI - Glossary`
- DUUMBI Agentic Development Map: `DUUMBI Agentic Development Map`
- DUUMBI Development Intake to Delivery Workflow: `DUUMBI - Development Intake to Delivery Workflow`
- DUUMBI Product Roadmap 2026-05: `DUUMBI - Product Roadmap 2026-05`
