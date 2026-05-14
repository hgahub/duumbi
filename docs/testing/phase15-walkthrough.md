# Phase 15 Final E2E Walkthrough Protocol

**Canonical issue:** [#489](https://github.com/hgahub/duumbi/issues/489)
**Sample issues:** [#486 Calculator](https://github.com/hgahub/duumbi/issues/486), [#487 String Utilities](https://github.com/hgahub/duumbi/issues/487), [#488 Math Library](https://github.com/hgahub/duumbi/issues/488)
**Purpose:** provide one operational protocol for validating DUUMBI's Phase 15 CLI REPL and Studio workflow evidence.

This document is the reviewer-facing Phase 15 walkthrough. It replaces the older two-sample scope and is organized around the final all-samples protocol required by #489.

Current evidence status:

| Sample | Issue | Task id | Module evidence | Representative output | Evidence status |
|---|---:|---|---|---|---|
| Calculator | [#486](https://github.com/hgahub/duumbi/issues/486) | `calculator` | `calculator/ops` with `add`, `subtract`, `multiply`, `divide` | `3 + 5 = 8`, `10 / 2 = 5` | Accepted |
| String Utilities | [#487](https://github.com/hgahub/duumbi/issues/487) | `string-utils` | `string/utils` with `reverse`, `count_vowels`, `is_palindrome` | `duumbi -> ibmuud`, vowel count `3`, `level` palindrome | Accepted |
| Math Library | [#488](https://github.com/hgahub/duumbi/issues/488) | `math-library` | `math/lib` with `factorial`, `fibonacci`, `is_prime` | `factorial(10) = 3628800`, `fibonacci(15) = 610`, `is_prime(97) = 1` | Accepted in #488 via [PR #552](https://github.com/hgahub/duumbi/pull/552) |

Do not use this document to claim Phase 15 completion until all three sample rows have accepted evidence and the final #489 implementation review confirms that the walkthrough matches real results.

## Prerequisites

Run from a fresh DUUMBI source checkout unless a sample section explicitly says to reuse a generated workspace.

Required local tools:

```bash
rustup show
cc --version
cargo build
cargo build -p duumbi-studio --features ssr
export DUUMBI="$(pwd)/target/debug/duumbi"
```

Provider setup:

- Use `/provider` in the REPL or `duumbi provider add ...` for normal user-facing setup.
- The Phase 15 harness also supports provider env vars. For the documented MiniMax path, set `MINIMAX_API_KEY` in the shell environment.
- Never paste raw provider secrets into docs, issue comments, reports, screenshots, or logs.
- Missing credentials are a blocked evidence condition, not a deterministic product failure.

Example provider check shape:

```bash
test -n "$MINIMAX_API_KEY" || echo "MINIMAX_API_KEY is not set"
```

Recommended report paths:

```text
/tmp/duumbi-phase15-calculator-report.json
/tmp/duumbi-phase15-string-utils-report.json
/tmp/duumbi-phase15-math-library-report.json
```

## Shared Review Flow

For each sample:

1. Verify prerequisites.
2. Run the automated harness when the sample has accepted implementation support.
3. Inspect the JSON report for task id, provider, workspace, generated slug, CLI evidence, Studio evidence, elapsed time, UX checks, failure category, and Ralph Gate guidance.
4. Confirm graph/module evidence before trusting build or run output.
5. Confirm Studio validates the same generated workspace through the shared backend after CLI generation passes.
6. Confirm Query mode remains read-only and mutation remains Agent mode or explicit intent execution.
7. Classify any failure using the troubleshooting table in this document.
8. Record issue, PR, report, and evidence-comment links for Stage 11/12 review.

The Studio leg should reuse the CLI-generated workspace after the CLI leg passes. It should not spend a second provider-backed mutation unless a later approved cycle explicitly changes that policy.

## Calculator

**Issue:** [#486](https://github.com/hgahub/duumbi/issues/486)
**Task id:** `calculator`
**Timing target:** under 10 minutes for a normal single-sample run.

Canonical intent:

```text
Build a calculator with add, subtract, multiply, divide functions that work on i64 numbers
```

Automated harness:

```bash
$DUUMBI phase15-e2e calculator \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-calculator-report.json
```

Manual CLI path:

```bash
mkdir -p /tmp/duumbi-p15-calculator-cli
cd /tmp/duumbi-p15-calculator-cli
$DUUMBI init .
$DUUMBI
```

In the REPL:

```text
/intent create "Build a calculator with add, subtract, multiply, divide functions that work on i64 numbers"
y
/intent execute <generated-slug>
/describe
/build
/run
```

Required evidence:

- Top-level report task is `calculator`.
- CLI evidence includes a fresh workspace, generated slug, elapsed time, build/run result, stdout, and failure category if any.
- Graph evidence includes `calculator/ops`.
- Function evidence includes `add`, `subtract`, `multiply`, and `divide`.
- Run output includes representative correct arithmetic results such as `3 + 5 = 8` and `10 / 2 = 5`.
- Studio evidence includes `shared_backend_workspace=true`, `graph_has_calculator_ops=true`, build output path, run stdout, footer evidence, Query read-only evidence, and Agent mode availability.
- The process exit code may be nonzero for Calculator because DUUMBI examples can return an i64 from `main`; stdout correctness is the sample evidence gate.

## String Utilities

**Issue:** [#487](https://github.com/hgahub/duumbi/issues/487)
**Implementation PR:** [#546](https://github.com/hgahub/duumbi/pull/546)
**Task id:** `string-utils`
**Timing target:** under 15 minutes for a normal single-sample run.

Canonical intent:

```text
Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main.
```

Automated harness:

```bash
$DUUMBI phase15-e2e string-utils \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-string-utils-report.json
```

Manual CLI path:

```bash
mkdir -p /tmp/duumbi-p15-string-utils-cli
cd /tmp/duumbi-p15-string-utils-cli
$DUUMBI init .
$DUUMBI
```

In the REPL:

```text
/intent create "Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main."
y
/intent execute <generated-slug>
/describe
/build
/run
```

Required evidence:

- Top-level report task is `string-utils`.
- CLI evidence includes a fresh workspace, generated slug, elapsed time, build/run result, stdout, and failure category if any.
- Graph evidence includes `string/utils`.
- Function evidence includes `reverse`, `count_vowels`, and `is_palindrome`.
- Run output demonstrates:
  - `reverse("duumbi") = "ibmuud"` or a labeled equivalent.
  - `count_vowels("duumbi") = 3`.
  - `is_palindrome("level") = true` or `1`.
- Studio evidence includes `shared_backend_workspace=true`, `graph_has_string_utils=true`, build output path, run stdout, footer evidence, Query read-only evidence, and Agent mode availability.

Latest accepted evidence reference from #487 used Anthropic:

```text
/tmp/duumbi-phase15-string-utils-anthropic-cycle19-report.json
```

## Math Library

**Issue:** [#488](https://github.com/hgahub/duumbi/issues/488)
**Implementation PR:** [#552](https://github.com/hgahub/duumbi/pull/552)
**Task id:** `math-library`
**Accepted provider:** MiniMax
**Accepted report path:** `/tmp/duumbi-phase15-math-library-report.json`
**Timing target:** under 15 minutes for a normal single-sample run.

Canonical intent:

```text
Build a math library with: factorial (recursive), fibonacci (iterative), and is_prime functions. The main function should compute factorial(10), fibonacci(15), and check if 97 is prime.
```

Automated harness:

```bash
$DUUMBI phase15-e2e math-library \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-math-library-report.json
```

Manual CLI path:

```bash
mkdir -p /tmp/duumbi-p15-math-library-cli
cd /tmp/duumbi-p15-math-library-cli
$DUUMBI init .
$DUUMBI
```

In the REPL:

```text
/intent create "Build a math library with: factorial (recursive), fibonacci (iterative), and is_prime functions. The main function should compute factorial(10), fibonacci(15), and check if 97 is prime."
y
/intent execute <generated-slug>
/describe
/build
/run
```

Required evidence:

- Top-level report task is `math-library`.
- CLI evidence includes a fresh workspace, generated slug, elapsed time, build/run result, stdout, and failure category if any.
- Graph evidence includes `math/lib`.
- Function evidence includes `factorial`, `fibonacci`, and `is_prime`.
- Run output demonstrates:
  - `factorial(10) = 3628800` or a labeled equivalent.
  - `fibonacci(15) = 610`.
  - `is_prime(97) = true` or `1`.
- `app/main` calls or otherwise depends on functions from `math/lib`; direct print-only output from `app/main` is not sufficient evidence.
- Studio evidence includes `shared_backend_workspace=true`, `graph_has_math_lib=true`, build output path, run stdout, footer evidence, Query read-only evidence, and Agent mode availability.

Accepted #488 evidence:

- Live evidence report: [Ralph Cycle 4 evidence](https://github.com/hgahub/duumbi/issues/488#issuecomment-4450913577).
- Closure evidence: [Stage 12 closure comment](https://github.com/hgahub/duumbi/issues/488#issuecomment-4451303014).
- Provider: `minimax`.
- Report path: `/tmp/duumbi-phase15-math-library-report.json`.
- Top-level report values: task `math-library`, provider `minimax`, attempts `1`, aggregate attempt passed.
- CLI elapsed: `513.023s`; total elapsed: `517.212s`; Studio elapsed: `4.188s`.
- CLI evidence includes `module_math_lib_exists=true`, `describe_contains_factorial=true`, `describe_contains_fibonacci=true`, and `describe_contains_is_prime=true`.
- Run exit code was `0`.
- Accepted stdout includes:
  - `factorial(10) = 3628800`.
  - `fibonacci(15) = 610`.
  - `is_prime(97) = 1`.
- Studio evidence includes `shared_backend_workspace=true`, `graph_has_math_lib=true`, `ux_footer_items=Intents,Graph,Build`, `ux_query_default_active=true`, `ux_query_read_only=true`, and `ux_agent_mode_available=true`.
- The live report's stale Calculator/#486 Ralph Gate wording was fixed before merge by #488 Cycle 5 and included in [PR #552](https://github.com/hgahub/duumbi/pull/552).

## Studio Validation

For each sample, Studio validation should confirm:

- Footer has exactly three primary workflow items: `Intents`, `Graph`, `Build`.
- `Intents` can create and execute the sample intent when a live mutation is intended.
- `Graph` can show the expected module evidence for the generated workspace.
- `Build` calls `POST /api/build` and returns `{ ok, message, output_path }`.
- Run calls `POST /api/run` and returns `{ ok, exit_code, stdout, stderr }`.
- Run stdout satisfies the same sample evidence checks as the CLI leg.
- Query chat mode is active/read-only by default.
- Graph mutation requires Agent mode or explicit Intent execution.

When capturing screenshots later, capture deterministic states: Intents with the selected sample, Graph with the expected module visible, Build output after build, Run output after run, and Query mode showing read-only state.

## Failure Classification

| Failure class | Evidence signal | Interpretation | Next action |
|---|---|---|---|
| Missing credentials | `failure_category: "missing_provider_credentials"` and `missing_env=<PROVIDER_KEY>` | Blocked setup, not product failure | Configure provider through `/provider`, `duumbi provider add`, or the documented env var. Do not paste secrets into docs or comments. |
| Provider authentication | auth-related provider error | Provider setup failed | Verify provider account/key outside committed artifacts. |
| Provider rate limit/server/network | rate-limit, 5xx, network, or timeout evidence | Provider instability or quota issue | Record provider, retry only when approved, or switch provider with approval. |
| Provider timeout | `provider_timeout` or timeout elapsed evidence | Live mutation exceeded budget | Inspect generated workspace and learning records before another paid run. |
| Graph evidence mismatch | expected module/function evidence missing | Generated graph does not satisfy sample contract | Treat as deterministic implementation or generation-quality failure until provider evidence proves otherwise. |
| Compiler/build failure | build result `ok=false` or compiler diagnostics | Graph could not compile | File or fix compiler/graph issue under a separate approved cycle if outside current scope. |
| Run-output mismatch | build succeeds but stdout misses required values | Behavior mismatch | Compare graph functions and `app/main` calls with sample pass criteria. |
| Report serialization failure | report missing or invalid JSON | Harness/report defect | Fix report path or serialization under an approved implementation cycle. |
| Studio shared-backend failure | CLI passes but Studio graph/build/run fails on same workspace | Studio/server/shared workflow issue | Inspect `POST /api/build`, `POST /api/run`, and graph context evidence. |
| Query/Agent mode regression | Query allows mutation or Agent unavailable | UX/safety regression | Stop and fix under approved Studio/REPL scope. |
| Docs mismatch | walkthrough command or expected output contradicts accepted evidence | Documentation defect | Update the walkthrough to match source-backed evidence. |

## Evidence And Traceability

Minimum evidence to keep with the implementation PR or issue comments:

- GitHub issue links: #486, #487, #488, #489.
- Product spec: `specs/DUUMBI-489/PRODUCT.md`.
- Technical spec: `specs/DUUMBI-489/TECHNICAL.md`.
- Implementation PR links for each completed sample.
- Report paths and provider used for each accepted run.
- Commands/checks run and results.
- Any deviations from timing targets, with accepted evidence explaining them.
- Remaining risks or follow-up issues.

Accepted implementation references:

- String Utilities implementation: [PR #546](https://github.com/hgahub/duumbi/pull/546).
- Math Library implementation and evidence fixes: [PR #552](https://github.com/hgahub/duumbi/pull/552).
- Math Library live evidence: [#488 Cycle 4 evidence](https://github.com/hgahub/duumbi/issues/488#issuecomment-4450913577).
- Math Library closure evidence: [#488 Stage 12 closure](https://github.com/hgahub/duumbi/issues/488#issuecomment-4451303014).

## Regression Checks

Docs-only changes should at minimum run:

```bash
git diff --check
```

If a later approved cycle changes source code, harness behavior, Studio behavior, provider behavior, or tests, run the focused tests named in that cycle. Shared source changes should normally also run:

```bash
cargo fmt --check
cargo test --all
cargo test -p duumbi-studio --features ssr
cargo clippy --all-targets -- -D warnings
```

Do not run live provider validation without explicit approval for that cycle.
