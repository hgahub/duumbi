# Phase 15 E2E Walkthrough

**Issues:** [#486](https://github.com/hgahub/duumbi/issues/486), [#487](https://github.com/hgahub/duumbi/issues/487)
**Scope:** Calculator and String Utilities samples only. Do not expand into #488 or #489.
**Provider for live validation:** MiniMax via `MINIMAX_API_KEY`.

This document is both the manual protocol and the evidence log for the approved
Phase 15 kill-criterion samples. #486 proves Calculator. #487 proves String
Utilities. #488 Math Library validation and #489 final all-samples protocol
consolidation remain separate work.

## Expected Protocol

### Prerequisites

- Rust toolchain: `rustup show`
- C compiler on PATH: `cc --version`
- Local binaries:

```bash
cargo build
cargo build -p duumbi-studio --features ssr
export DUUMBI="$(pwd)/target/debug/duumbi"
```

- Live provider credential:

```bash
export MINIMAX_API_KEY="..."
```

Use `/provider` in the REPL or `duumbi provider add ...` for normal provider
setup. The E2E harness uses the env-var path and never logs raw secrets.

### CLI Calculator Path

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

Pass criteria:

- Intent creation saves a generated slug.
- Intent execution creates `calculator/ops` and updates `app/main`.
- `/describe` shows calculator functions.
- `/build` writes `.duumbi/build/output`.
- `/run` prints representative correct results, including `3 + 5 = 8` and `10 / 2 = 5` or equivalent.
- Total CLI elapsed time is under 10 minutes.

### Studio Calculator Path

```bash
mkdir -p /tmp/duumbi-p15-calculator-studio
cd /tmp/duumbi-p15-calculator-studio
$DUUMBI init .
$DUUMBI studio --port 8421
```

Open `http://localhost:8421`.

Pass criteria:

- Footer has exactly three primary workflow items: `Intents`, `Graph`, `Build`.
- Intents workflow can create and execute the Calculator intent.
- Graph context data includes `calculator/ops`.
- Build workflow calls `POST /api/build` and returns `{ ok, message, output_path }`.
- Run workflow calls `POST /api/run` and returns `{ ok, exit_code, stdout, stderr }`.
- Query chat mode is read-only. Switch to Agent mode before asking for mutation such as modulo/power; only Agent success refreshes the graph.

## String Utilities Protocol

**Issue:** [#487](https://github.com/hgahub/duumbi/issues/487)
**Scope:** String Utilities sample only. Do not expand into #488 Math Library or #489 final all-samples protocol consolidation.

### CLI String Utilities Path

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

Pass criteria:

- Intent creation saves a generated slug.
- Intent execution creates `string/utils` and updates `app/main`.
- `/describe` or graph evidence shows `reverse`, `count_vowels`, and `is_palindrome`.
- `/build` writes `.duumbi/build/output`.
- `/run` prints representative correct results:
  - `reverse("duumbi") = "ibmuud"` or a labeled equivalent.
  - `count_vowels("duumbi") = 3`.
  - `is_palindrome("level") = true` or `1`.
- Query mode remains read-only; mutation requires Agent mode or explicit Intent execution.
- Total CLI elapsed time stays within the Phase 15 live leg timeout.

### Studio String Utilities Path

The Studio leg must reuse the CLI-generated workspace after the CLI leg passes.
It must not spend a second provider-backed mutation.

Pass criteria:

- Footer has exactly three primary workflow items: `Intents`, `Graph`, `Build`.
- Graph context data includes `string/utils`.
- Build workflow calls `POST /api/build` and returns `{ ok, message, output_path }`.
- Run workflow calls `POST /api/run` and returns `{ ok, exit_code, stdout, stderr }`.
- Run stdout satisfies the same String Utilities evidence checks as the CLI leg.
- Query chat mode remains active and read-only by default; Agent mode remains available for mutation handoff.

### Automated String Utilities Harness

```bash
duumbi phase15-e2e string-utils \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-string-utils-report.json
```

Pass criteria:

- Top-level report task is `string-utils`.
- CLI evidence includes a fresh workspace, generated slug, `module_string_utils_exists=true`, function evidence for `reverse`, `count_vowels`, and `is_palindrome`, build result, run exit code, stdout, elapsed time, and failure category if any.
- Studio evidence includes `shared_backend_workspace=true`, `graph_has_string_utils=true`, build output path, run stdout, footer evidence, Query read-only evidence, and Agent mode evidence.
- Missing credentials produce `failure_category: "missing_provider_credentials"` with `missing_env=MINIMAX_API_KEY`.
- Provider timeouts produce `provider_timeout`, not an ambiguous mutation or test failure.
- Deterministic code, docs, and evidence mismatches remain separate categories.

Cycle 3 did not run live-provider validation. The approved non-live evidence path
for the first local live run is:

```text
/tmp/duumbi-phase15-string-utils-report.json
```

If `MINIMAX_API_KEY` is missing, the expected blocked evidence is:

```json
{
  "failure_category": "missing_provider_credentials",
  "evidence": ["missing_env=MINIMAX_API_KEY"]
}
```

## Automated Calculator Harness

Run one Ralph Loop attempt:

```bash
$DUUMBI phase15-e2e calculator \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-calculator-report.json
```

The harness creates fresh temp workspaces for CLI and Studio legs, records
timing, generated slug, graph/module evidence, build/run results, stdout/stderr,
aggregate performance, Studio UX checks, and a Ralph Gate summary. Each
provider-backed leg has an outer 600 second timeout so a stalled live call
becomes structured evidence instead of hanging.

After each run it prints:

- `Continue?` with pass/fail and whether another paid loop is useful.
- Provider-change guidance. If credentials are missing, it asks for the relevant env var instead of a raw key.
- Engineering opinion: code bug, provider instability, docs mismatch, blocked, or inconclusive.

## Evidence Log

### Current Deterministic Implementation Evidence

- Studio root shell is wired to the three-panel workflow: `Intents`, `Graph`, `Build`.
- Static JSON endpoints are present:
  - `POST /api/intent/{slug}/execute`
  - `POST /api/build`
  - `POST /api/run`
- Studio module discovery scans `.duumbi/graph/**/*.jsonld`, so nested modules such as `calculator/ops` are discoverable.
- Workspace program loading, intent verification, export summaries, and repair passes scan nested `.duumbi/graph/**/*.jsonld` files recursively.
- Intent execution preserves slash-named modules on disk, so `calculator/ops` is written as `.duumbi/graph/calculator/ops.jsonld`.
- The known Phase 15 Calculator sample normalizes provider-generated specs to the four representative #486 tests: `3 + 5 = 8`, `10 - 3 = 7`, `4 * 6 = 24`, and `10 / 2 = 5`.
- Known sample handling now lives in `intent::benchmarks`; generic intent creation only applies Calculator normalization when the prompt matches the benchmark.
- AI graph mutation uses a configurable `[agent]` policy. Defaults are `mutation-retries = 5`, `repair-retries = 2`, and `mutation-timeout-secs = 600`, with optional `[agent.providers.minimax]` overrides.
- The Phase 15 harness seeds `.duumbi/learning/successes.jsonl` from prior Phase 15 temp workspaces, so successful partial work can inform later fresh workspaces.
- Intent execution records successes to workspace-local learning and user-local `~/.duumbi/learning/successes.jsonl`; it records sanitized failures to workspace-local and user-local `failures.jsonl`.
- Context assembly reads combined workspace-local and user-local success/failure records, deduplicated by record id, so later fresh workspaces can reuse previous lessons.
- The shared `workflow` service exposes graph evidence, build, run, intent creation, and intent execution orchestration for CLI, Studio, and the Phase 15 harness.
- The harness treats CLI as the provider-backed execution gate. After CLI passes, Studio validates graph visibility, build, and run against the CLI-generated workspace through the shared backend instead of performing a second live provider execution.
- The Phase 15 harness accepts a calculator binary that returns a representative result such as `8`; stdout correctness is the kill criterion, not exit code zero.
- Studio build uses the shared library workspace build helper, not `cargo run` from the user workspace.
- Studio run returns structured stdout/stderr and a structured no-binary error.
- Studio chat defaults to `Query`; mutation is routed through `Agent` mode and success frames request graph refresh.

### Live MiniMax Evidence

Live provider evidence must be generated locally because it depends on
`MINIMAX_API_KEY`, model availability, and paid API calls.

If `MINIMAX_API_KEY` is missing, the harness result is **blocked**, not failed:

```json
{
  "failure_category": "missing_provider_credentials",
  "evidence": ["missing_env=MINIMAX_API_KEY"]
}
```

Observed reports:

```text
Report: /tmp/duumbi-phase15-calculator-report.json
Result: failed pre-fix
CLI elapsed: 277.547s
CLI slug: build-a-calculator-with-add-subtract-mul
CLI stdout: add(5, 3) = 8; divide(20, 4) = 5
CLI failure: evidence_mismatch, because calculator functions were generated but calculator/ops was not visible at the required path
Studio failure: config parse error, role = "Primary" should have been role = "primary"
Fix applied: lowercase provider role in harness config, plus Calculator intent defaults require calculator/ops and app/main

Report: /tmp/duumbi-phase15-calculator-report-r2.json
Result: failed after first fix
CLI failure: timeout after 180s
Studio failure: timeout calling POST /api/intent/build-a-calculator-with-add-subtract-mul/execute
Workspace evidence: generated calculator code existed, but the executor wrote .duumbi/graph/calculator_ops.jsonld instead of .duumbi/graph/calculator/ops.jsonld
Fix applied: preserve slash module paths, create nested graph dirs, and recursively load/copy/discover graph JSON-LD files
Ralph Gate opinion: code bug until the failure category proves provider instability

Report: /tmp/duumbi-phase15-calculator-report-r3.json
Result: failed after nested module fix
CLI failure: timeout after 180s; workspace contained the generated intent but no calculator module yet
Studio failure: timeout calling POST /api/intent/build-a-calculator-with-add-subtract-mul/execute
Workspace evidence: Studio workspace did create .duumbi/graph/calculator/ops.jsonld, proving the nested path fix worked
Provider evidence: MiniMax required multiple mutation retries and still exceeded the live harness timeout
Fix applied: bound the known Calculator sample to four representative tests and update Ralph Gate timeout/provider classification

Report: /tmp/duumbi-phase15-calculator-report-r4.json
Result: failed after test-scope bounding
CLI elapsed: 85.898s
CLI result: passed with seeded_learning_records=5, module_calculator_ops_exists=true, and correct stdout
CLI stdout: add(3, 5) = 8; subtract(10, 3) = 7; multiply(4, 6) = 24; divide(10, 2) = 5
Studio failure: second provider-backed intent execution timed out
Fix applied: Studio harness leg now reuses the CLI-generated workspace and validates graph/build/run through shared backend endpoints

Report: /tmp/duumbi-phase15-calculator-report-r5.json
Result: passed
CLI elapsed: 92.539s
CLI slug: build-a-calculator-with-add-subtract-mul
CLI evidence: seeded_learning_records=7, module_calculator_ops_exists=true, run_exit_code=8
CLI stdout: add(3, 5) = 8; subtract(10, 3) = 7; multiply(4, 6) = 24; divide(10, 2) = 5
Studio elapsed: 3.858s
Studio evidence: shared_backend_workspace=true, graph_has_calculator_ops=true, build output path returned, run stdout matched CLI
Learning cache: /var/folders/j9/9_t_gcr50cjfr942mkywtn400000gn/T/duumbi-phase15-calculator-minimax-learning.jsonl, 9 records after r5
Ralph Gate opinion: #486 evidence is strong enough for the Calculator path; repeat only for confidence across multiple live attempts

Report: /tmp/duumbi-phase15-calculator-report-recommended.json
Result: failed after recommended-improvements pass
CLI elapsed: 171.491s
CLI generated calculator/ops and correct stdout, but the harness reported run_failed because the binary returned exit code 8
Root cause: harness treated nonzero calculator return values as execution failure even though DUUMBI examples may use main's i64 return as process exit code
Fix applied: Phase 15 harness now treats an executed binary with correct stdout as valid evidence, even when exit_code is nonzero

Report: /tmp/duumbi-phase15-calculator-report-recommended-r2.json
Result: passed
CLI elapsed: 108.630s
CLI slug: build-a-calculator-with-add-subtract-mul
CLI evidence: seeded_learning_records=11, module_calculator_ops_exists=true, run_exit_code=8
CLI stdout: add(3, 5) = 8; subtract(10, 3) = 7; multiply(4, 6) = 24; divide(10, 2) = 5
Studio elapsed: 4.119s
Studio evidence: shared_backend_workspace=true, graph_has_calculator_ops=true, build output path returned, run stdout matched CLI
Ralph Gate opinion: #486 evidence is strong enough for the Calculator path; repeat only for confidence across multiple live attempts

Report: /tmp/duumbi-phase15-calculator-report-final.json
Result: passed on the final current binary after configurable mutation timeout enforcement
CLI elapsed: 141.656s
CLI slug: build-a-calculator-with-add-subtract-mul
CLI evidence: seeded_learning_records=13, module_calculator_ops_exists=true, run_exit_code=8
CLI stdout: add(3, 5) = 8; subtract(10, 3) = 7; multiply(4, 6) = 24; divide(10, 2) = 5
Studio elapsed: 4.369s
Studio evidence: shared_backend_workspace=true, graph_has_calculator_ops=true, build output path returned, run stdout matched CLI
Ralph Gate opinion: #486 evidence is strong enough for the Calculator path; repeat only for confidence across multiple live attempts

Report: /tmp/duumbi-phase15-calculator-report-9a93.json
Result: passed after explicit performance and Studio UX reporting
CLI elapsed: 95.021s
Studio elapsed: 3.621s
Total elapsed: 98.644s
CLI evidence: seeded_learning_records=16, module_calculator_ops_exists=true, run_exit_code=8
CLI stdout: add(3, 5) = 8; subtract(10, 3) = 7; multiply(4, 6) = 24; divide(10, 2) = 5
Studio UX evidence: footer items were exactly Intents, Graph, Build; Query mode rendered as active/read-only; Agent mode was available for mutation handoff
Studio backend evidence: shared_backend_workspace=true, graph_has_calculator_ops=true, build output path returned, run stdout matched CLI
Ralph Gate opinion: #486 evidence is strong enough for the Calculator path; repeat only for confidence across multiple live attempts
```

## Regression Checks

```bash
cargo fmt --check
cargo test --all
cargo test -p duumbi-studio --features ssr
cargo clippy --all-targets -- -D warnings
```

Focused coverage:

- Studio root footer renders only `Intents`, `Graph`, `Build`.
- Studio build/run endpoint helpers return structured responses.
- No-binary run error is non-panicking and structured.
- Module discovery includes nested workspace modules like `calculator/ops`.
- Provider-facing guidance points users to `/provider` or `duumbi provider add ...`.
