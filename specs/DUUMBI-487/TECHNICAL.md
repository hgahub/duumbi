# DUUMBI-487: Phase 15 E2E String Utilities Sample - Technical Specification

## Implementation Objective

Implement the approved DUUMBI-487 product spec by adding a `string-utils` task to the existing Phase 15 E2E validation path. The implementation must prove that a fresh workspace can create, execute, inspect, build, run, and expose the String Utilities sample through both CLI and Studio without regressing the already validated `calculator` task.

This technical spec implements these approved outcomes:

- `duumbi phase15-e2e string-utils --provider <provider> --attempts 1 --output <path>` runs a stable String Utilities E2E task.
- Intent execution creates `.duumbi/graph/string/utils.jsonld` and updates `.duumbi/graph/main.jsonld`.
- Graph and report evidence show `reverse`, `count_vowels`, and `is_palindrome`.
- Build and run evidence demonstrates representative correct String Utilities behavior.
- Studio validates the same CLI-generated workspace through the shared backend, including graph visibility, build, run, and Phase 15 UX checks.
- Missing credentials, provider failures, deterministic code defects, docs mismatches, and evidence mismatches remain distinct report categories.
- `docs/testing/phase15-walkthrough.md` gains a dedicated String Utilities section without taking over #489 final all-samples protocol work.

## Agent Audience

Primary implementation agents:

- Codex for local source edits, focused tests, docs, and evidence collection.
- Oz or another cloud runner only if a human explicitly approves a Ralph cycle that needs long-running live-provider validation.

Review agents:

- Codex or Oz for Stage 9 technical spec review.
- A specialized tester may run the live MiniMax validation after deterministic tests pass and credentials are available.

## Source Context

Verified facts:

- Product spec: `specs/DUUMBI-487/PRODUCT.md`.
- GitHub issue: https://github.com/hgahub/duumbi/issues/487.
- Stage 7 approval: https://github.com/hgahub/duumbi/issues/487#issuecomment-4416487430.
- Existing harness: `src/cli/phase15_e2e.rs` is Calculator-specific and rejects every task except `calculator`.
- CLI command: `src/cli/mod.rs` defines hidden `phase15-e2e` with a free-form `task` argument, provider, attempts, output, and Studio port.
- Current benchmark normalization: `src/intent/benchmarks.rs` recognizes only Calculator and rewrites its modules and i64 test cases.
- Current intent verifier: `src/intent/verifier.rs` supports i64 arguments and i64 expected returns. `function: "main"` test cases run the workspace main and fall back to process exit code when stdout is not an integer.
- Current intent execution: `src/intent/execute.rs` skips verification when `test_cases` is empty, otherwise runs verifier tests and can repair failures.
- Current create-module missing-function retry: `src/intent/execute.rs` derives expected functions from non-main test cases.
- Shared graph/build/run backend: `src/workflow.rs` exposes `graph_evidence`, `build_workspace`, and `run_workspace`; graph evidence recursively scans `.duumbi/graph/**/*.jsonld`.
- Studio shared backend wrappers: `crates/duumbi-studio/src/server_fns.rs` exposes build/run API helpers and `discover_workspace_modules`, which maps nested graph paths such as `string/utils`.
- Studio UX evidence: `src/cli/phase15_e2e.rs` validates footer labels `Intents`, `Graph`, `Build`, Query default/read-only state, and Agent mode availability by inspecting SSR HTML.
- Existing string support: `src/types.rs`, `src/parser/mod.rs`, `src/compiler/lowering.rs`, `runtime/duumbi_runtime.c`, `tests/fixtures/string_length.jsonld`, `tests/fixtures/string_concat.jsonld`, and `tests/integration_phase9a_stdlib.rs` cover current string operations.
- Existing string showcase: `src/bench/showcases/string_ops.yaml` uses i64-returning helper functions for string operation validation.
- Repo instructions: `AGENTS.md` requires no implementation without spec approval, no `.unwrap()` in library code, public docs on public items, zero-warning clippy, and Query mode remaining read-only.

Relevant Obsidian notes checked during product spec preparation:

- `DUUMBI - PRD`
- `DUUMBI - Development Intake to Delivery Workflow`
- `DUUMBI - Glossary`
- `DUUMBI Agentic Development Map`
- `DUUMBI - Product Roadmap 2026-05`
- `DUUMBI - Phase 15 - Studio Workflow Redesign`

Assumptions:

- MiniMax remains the documented live provider for local evidence because #486 uses MiniMax, but implementation must keep the existing provider parameter.
- `string-utils` should follow the existing shared-backend pattern: one live provider-backed CLI generation, then Studio graph/build/run/UX validation against that same workspace.
- The implementation should not add string-valued generic verifier test cases in this issue; that would expand the intent verifier contract beyond the approved #487 scope.

## Affected Areas

Expected source changes:

- `src/cli/phase15_e2e.rs`
  - Generalize the harness from Calculator-only constants and predicates to task descriptors.
  - Add `string-utils` descriptor, CLI evidence checks, Studio graph checks, stdout checks, learning/failure metadata, and task-specific messages.
  - Keep existing report schema compatible unless a field is strictly necessary and covered by tests.

- `src/cli/mod.rs`
  - Update `phase15-e2e` task help text to list `calculator` and `string-utils`.

- `src/intent/benchmarks.rs`
  - Add String Utilities benchmark recognition and deterministic normalization.
  - Preserve Calculator normalization behavior.
  - Expose task-specific expected functions if needed by intent execution.

- `src/intent/execute.rs`
  - If implementation needs deterministic create-module retry for String Utilities, extend expected-function derivation for known benchmark functions instead of parsing prose heuristically.
  - Keep generic intent behavior unchanged unless directly necessary for the benchmark.

- `docs/testing/phase15-walkthrough.md`
  - Add String Utilities protocol, commands, pass criteria, report path, and evidence log section.
  - Preserve Calculator history and avoid final all-samples consolidation for #489.

Expected test changes:

- Unit tests in `src/cli/phase15_e2e.rs` for task lookup, unsupported-task error text, String Utilities stdout predicate, Studio UX evidence still passing, and task-specific graph evidence.
- Unit tests in `src/intent/benchmarks.rs` for String Utilities normalization.
- Focused tests in `src/workflow.rs` or Studio layout tests if nested module discovery needs a String Utilities-specific assertion. Existing nested module tests may be extended from `calculator/ops` to include `string/utils` without broad refactor.

Expected generated/local artifacts during validation:

- `/tmp/duumbi-phase15-string-utils-report.json` for live evidence.
- Temporary workspaces under the OS temp directory, created by the harness.
- No committed generated reports unless a later approved docs decision requires them.

CI/check paths:

- `cargo fmt --check`
- `cargo test --all`
- `cargo test -p duumbi-studio --features ssr`
- `cargo clippy --all-targets -- -D warnings`

## Technical Approach

### 1. Generalize The Phase 15 Harness With Task Descriptors

Replace Calculator-specific constants and predicates in `src/cli/phase15_e2e.rs` with a small static descriptor model, for example:

```rust
struct Phase15Task {
    id: &'static str,
    display_name: &'static str,
    intent: &'static str,
    module_path: &'static str,
    expected_functions: &'static [&'static str],
    output_check: fn(&str) -> bool,
    failure_module: &'static str,
}
```

Required descriptors:

- `calculator`
  - intent: existing Calculator prompt
  - module path: `calculator/ops`
  - expected functions: `add`, `subtract`, `multiply`, `divide`
  - output predicate: preserve existing Calculator behavior

- `string-utils`
  - intent: `Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main.`
  - module path: `string/utils`
  - expected functions: `reverse`, `count_vowels`, `is_palindrome`
  - output predicate:
    - recognizes `reverse("duumbi") = "ibmuud"` or a close labeled equivalent
    - recognizes `count_vowels("duumbi") = 3`
    - recognizes `is_palindrome("level") = true` or `1`
    - optionally recognizes `is_palindrome("duumbi") = false` or `0`

Recommendations:

- Implement task lookup with a clear unsupported-task error: `Unsupported Phase 15 E2E task '<task>'. Supported tasks: calculator, string-utils.`
- Pass `&Phase15Task` through `run_cli_leg`, `run_studio_leg`, `run_studio_http_flow`, failure recording, output predicates, and evidence construction.
- Keep the JSON report's top-level `task` field as the user-provided stable id.

### 2. Normalize The String Utilities Benchmark Deterministically

Add a second known benchmark to `src/intent/benchmarks.rs`.

Matching predicate:

- Match prompts that contain enough of:
  - `string`
  - `reverse`
  - `vowel` or `count vowels`
  - `palindrome`

Normalization:

- Set `modules.create = ["string/utils"]`.
- Ensure `modules.modify` contains `app/main`.
- Set acceptance criteria that explicitly require:
  - `reverse` demonstrates reversing `duumbi` to `ibmuud`.
  - `count_vowels` demonstrates `duumbi` has 3 vowels.
  - `is_palindrome` demonstrates `level` is true and/or `duumbi` is false.
  - `main` prints human-readable labeled output for all three functions and returns 0.
- Set a `main_returns_zero` verifier test case:

```yaml
test_cases:
  - name: "main_returns_zero"
    function: "main"
    args: []
    expected_return: 0
```

Reasoning:

- The current generic `TestCase` schema supports only i64 arguments and expected returns.
- The current verifier has explicit support for `function: "main"` and process exit code fallback.
- Harness-level stdout predicates should validate the string-returning and boolean user-visible semantics.
- This avoids a broad generic verifier redesign while still requiring the generated workspace to compile and run during intent execution.

If implementation finds that create-module missing-function retry needs direct expected function names, add a small benchmark metadata helper rather than parsing acceptance-criteria prose:

```rust
pub fn expected_functions_for_benchmark(description: &str) -> Option<&'static [&'static str]>;
```

Then make `expected_exports_for_module` include those functions for `CreateModule` tasks when the current intent matches `string-utils`.

Rejected alternative:

- Do not extend `IntentSpec::TestCase` to support string args/returns in this issue. That is a broader verifier schema, wrapper-main, parser, docs, and compatibility change. #487 only needs the Phase 15 String Utilities E2E evidence path.

### 3. Validate CLI Evidence For String Utilities

In the CLI leg:

- Create and execute the normalized `string-utils` intent.
- Describe or otherwise inspect graph evidence after execution.
- Check `.duumbi/graph/string/utils.jsonld` exists.
- Check graph/source evidence contains all expected function names.
- Build via `crate::workflow::build_workspace`.
- Run via `crate::workflow::run_workspace`.
- Treat a process that prints correct stdout as valid even if process exit semantics need interpretation, consistent with the Calculator precedent. For String Utilities, the normalized main should return 0, so a nonzero exit with otherwise correct stdout should be recorded as evidence and evaluated deliberately.
- Emit evidence keys such as:
  - `module_string_utils_exists=true`
  - `describe_contains_reverse=true`
  - `describe_contains_count_vowels=true`
  - `describe_contains_is_palindrome=true`
  - `run_exit_code=0`
  - `stdout=<truncated output>`

Failure categories:

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
- `studio_ux_failed`
- `studio_http_failed`
- `skipped_cli_failed`

### 4. Validate Studio Against The Same Workspace

Keep the #486 shared-backend architecture:

- Start Studio in the CLI-generated workspace only after the CLI leg passes.
- Do not run a second live provider-backed intent execution in Studio.
- Reuse the existing HTTP flow:
  - GET `/`
  - GET `/api/graph/context`
  - POST `/api/build`
  - POST `/api/run`
- For `string-utils`, require graph modules to include `string/utils`.
- Require build JSON `ok == true`.
- Require run stdout to satisfy the task-specific String Utilities predicate.
- Keep the existing UX checks:
  - footer items exactly `Intents`, `Graph`, `Build`
  - Query default active
  - Query read-only title/state
  - Agent mode available

### 5. Documentation Update

Add a dedicated String Utilities section to `docs/testing/phase15-walkthrough.md`:

- Scope: #487 only.
- Prerequisites: same provider setup pattern as Calculator.
- CLI command:

```bash
$DUUMBI phase15-e2e string-utils \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-string-utils-report.json
```

- Pass criteria matching the product spec and harness evidence.
- Missing credential behavior: `missing_provider_credentials` with `missing_env=MINIMAX_API_KEY`.
- Evidence log placeholder or observed local report path after live validation.
- Explicit note that #488 Math Library and #489 final all-samples protocol remain out of scope.

## Invariants

- `calculator` behavior and report semantics must remain compatible.
- Query mode remains read-only by default; graph mutation remains Agent mode or explicit Intent execution.
- The Phase 15 harness must not log raw provider secrets.
- Missing provider credentials must be structured evidence, not a panic.
- Provider timeout must be bounded by the existing live leg timeout model.
- Slash-named modules must remain nested paths such as `.duumbi/graph/string/utils.jsonld`.
- Studio validation must reuse the CLI-generated workspace after CLI success.
- No second provider-backed Studio mutation should be introduced.
- The report must preserve CLI and Studio evidence separately.
- Generic user intent creation must not be rewritten by the String Utilities benchmark unless the prompt clearly matches the benchmark.
- The technical implementation must not broaden #487 into #488 Math Library or #489 final protocol consolidation.

## Ralph Cycle Protocol

Each cycle must:

1. Summarize the current state and remaining unmet requirements.
2. Propose one bounded implementation goal.
3. List intended file areas and commands.
4. Estimate resource use and risk.
5. Ask for explicit approval before starting.
6. Implement only the approved goal.
7. Run the agreed checks.
8. Report evidence, failures, and remaining gaps.
9. Stop if requirements are met or request approval for the next cycle.

No Ralph cycle may run live-provider validation without explicit human approval because it can spend paid API calls.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 4 source files plus directly related focused tests/docs.
- Expected command budget per deterministic cycle:
  - `cargo fmt --check`
  - focused `cargo test` commands for touched modules
  - broader `cargo test --all` only after focused checks pass or before final review
- Expected command budget for final deterministic cycle:
  - `cargo fmt --check`
  - `cargo test --all`
  - `cargo test -p duumbi-studio --features ssr`
  - `cargo clippy --all-targets -- -D warnings`
- Live-provider budget:
  - one `phase15-e2e string-utils` attempt after deterministic checks pass and credentials are available
  - additional attempts require explicit approval
- Approval required before every cycle: yes.
- Stop and ask for human guidance when:
  - implementation would require generic string-valued `IntentSpec::TestCase` schema changes
  - the harness needs a new provider UX or model selection behavior
  - live provider failures repeat after deterministic evidence is clean
  - #487 scope starts depending on #488 or #489 work
  - more than two bounded cycles are needed after deterministic tests pass

## Task Breakdown

1. Harness descriptor refactor
   - Introduce task descriptors in `src/cli/phase15_e2e.rs`.
   - Preserve Calculator behavior through the descriptor.
   - Add unsupported-task test coverage.

2. String Utilities benchmark normalization
   - Add benchmark matching and normalization in `src/intent/benchmarks.rs`.
   - Add tests proving `string-utils` maps to `string/utils`, `app/main`, deterministic acceptance criteria, and `main_returns_zero`.
   - Add expected-function metadata if needed by create-module retry.

3. String Utilities CLI evidence
   - Add module/function/stdout predicates.
   - Record task-specific evidence keys.
   - Add focused tests for the stdout predicate and graph evidence behavior.

4. String Utilities Studio evidence
   - Pass task descriptor into the Studio HTTP flow.
   - Check `string/utils` instead of hardcoded `calculator/ops`.
   - Reuse build/run and UX checks.

5. Documentation
   - Add the #487 String Utilities section to `docs/testing/phase15-walkthrough.md`.
   - Keep #489 final all-samples protocol out of scope.

6. Final deterministic verification
   - Run format, focused tests, full tests, Studio SSR tests, and clippy.

7. Live validation
   - Build binaries.
   - Run one approved provider-backed `string-utils` attempt.
   - Add the observed report path and result summary to the walkthrough if a live run is completed.

Independent slices:

- Descriptor refactor and tests.
- Benchmark normalization and tests.
- Documentation section draft.

Sequential slices:

- CLI String Utilities evidence after descriptor refactor.
- Studio String Utilities evidence after CLI evidence.
- Live validation after deterministic checks.

## Verification Plan

Focused deterministic tests:

- `cargo test phase15`
- `cargo test benchmarks`
- `cargo test graph_evidence`
- `cargo test -p duumbi-studio --features ssr studio_module_discovery_includes_nested_workspace_modules`
- Any new exact test names introduced by implementation.

Required final checks:

```bash
cargo fmt --check
cargo test --all
cargo test -p duumbi-studio --features ssr
cargo clippy --all-targets -- -D warnings
```

Live/manual check when credentials are available:

```bash
cargo build
cargo build -p duumbi-studio --features ssr
export MINIMAX_API_KEY="..."
./target/debug/duumbi phase15-e2e string-utils \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-string-utils-report.json
```

Report review requirements:

- Top-level `task` is `string-utils`.
- CLI evidence includes fresh workspace, slug, `module_string_utils_exists=true`, expected function evidence, build result, run exit code, stdout, elapsed time, and failure category if any.
- Studio evidence includes `shared_backend_workspace=true`, `graph_has_string_utils=true`, build output path, run stdout, UX footer evidence, Query read-only evidence, and Agent mode evidence.
- Missing credentials produce `failure_category: "missing_provider_credentials"` and `missing_env=MINIMAX_API_KEY`.
- Provider timeouts produce `provider_timeout`, not an ambiguous mutation or test failure.

Manual review:

- Inspect generated `.duumbi/graph/string/utils.jsonld` in the live workspace if the report fails with `evidence_mismatch`.
- Confirm `docs/testing/phase15-walkthrough.md` remains scoped and does not claim #489 completion.

## Completion Criteria

Implementation is complete when:

- `duumbi phase15-e2e calculator` remains supported.
- `duumbi phase15-e2e string-utils` is supported.
- Unsupported tasks produce clear supported-task guidance.
- String Utilities benchmark normalization is deterministic and tested.
- String Utilities CLI evidence can distinguish module/function/stdout mismatch from provider and build/run failures.
- Studio validation checks `string/utils` through the same shared-backend path used for Calculator.
- Query/Agent/Intent mode safety checks are unchanged and still reported.
- Documentation includes the #487 protocol and evidence report path or blocked credential evidence.
- Required deterministic checks pass, or any unrun check is explicitly reported with the reason.
- A live report exists when credentials are available, or missing credentials are reported as blocked rather than failed.

## Failure And Escalation

- If deterministic tests fail, stop the cycle after reporting the failing command and the smallest likely affected area.
- If `string-utils` generation succeeds but expected functions are missing, prefer benchmark expected-function metadata over broad verifier schema changes.
- If run stdout is semantically correct but exit code is nonzero, record both facts and decide based on the task descriptor's evidence policy; for String Utilities the normalized main should return 0, so nonzero exit is a deterministic defect unless justified by implementation evidence.
- If provider credentials are missing, do not ask for raw secrets in comments or docs. Report the missing environment variable.
- If provider timeouts repeat, preserve workspace evidence, harvest sanitized learning, and ask whether to retry, switch providers, or adjust provider timeout policy.
- If implementation appears to need generic string-valued verifier support, stop and ask for a scope decision because that exceeds #487.
- If docs work starts becoming an all-samples protocol, stop and route that work to #489.

## Open Questions

None blocking.

Technical decisions made here for Stage 10:

- Use `duumbi`, `ibmuud`, and `level` as canonical representative strings for String Utilities evidence.
- Add String Utilities benchmark normalization because the generic intent prompt is i64-oriented and provider variability is material for a live E2E kill criterion.
- Keep semantic equivalence in harness stdout checks permissive enough for labels and true/false representation, but require canonical graph function names `reverse`, `count_vowels`, and `is_palindrome` unless implementation evidence shows DUUMBI cannot reliably enforce them without broader changes.
