# DUUMBI-689 Preview Known Limitations

Related to #689.

The scaled intent-execute smoke evidence from 2026-06-16 should be treated as
preview risk evidence, not as a passing benchmark claim.

Evidence source:

- `docs/e2e/results/duumbi-689-scaled-smoke-20260616.md`
- Command: `duumbi benchmark --suite scaled --smoke --provider minimax:auto:primary:MINIMAX_API_KEY --attempts 1`

## What Passed

- The benchmark surface now has an opt-in scaled suite.
- The scaled smoke subset can select multi-function, cross-module, and
  HTTP/SQLite/JSON evidence tasks.
- The report distinguishes first-pass success, repair attempts, provider usage
  availability, failure categories, dominant evidence signals, and process
  evidence gaps.

## What Failed

- `scaled_math_pipeline` failed verifier evidence: 0/3 tests passed.
- `scaled_cross_module_stats` failed verifier evidence: 0/3 tests passed.
- `scaled_http_sqlite_json` was not counted as a pass because the current
  verifier does not check HTTP JSON payload semantics.

## What Preview Users Should Not Infer

- Do not infer that the intent write path reliably handles application-scale
  or service-like tasks.
- Do not infer that cross-module generated programs pass reliably under the
  current provider/prompt path.
- Do not infer HTTP + SQLite + JSON generated-service reliability from the
  scaled corpus entry alone. The repository has a flagship reference example,
  but this smoke run did not prove generated service behavior.
- Do not make cost claims from this run. Provider usage fields were present but
  token/cost values were unavailable.

## Top Failure Patterns To Feed Back

1. Scaled i64 task verifier failures need sampled generated-workspace evidence
   so follow-up work can distinguish wrong decomposition, missing exports,
   cross-module call resolution, and logic mistakes.
2. HTTP + SQLite + JSON needs a bounded process evidence checker before the
   scaled benchmark can validate service output directly.
3. Provider usage telemetry needs integration into benchmark results before
   preview cost claims are supported.
