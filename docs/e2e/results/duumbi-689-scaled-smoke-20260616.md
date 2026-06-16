# DUUMBI-689 Scaled Intent-Execute Smoke Report

Related to #689.

## Run Metadata

- Date: 2026-06-16
- DUUMBI version: `0.4.0-preview`
- Source branch: `codex/duumbi-689-implementation`
- Provider: `minimax`
- Provider identifier: `minimax:auto:primary:MINIMAX_API_KEY`
- Attempts per selected task: 1
- Command:

```bash
repo="$(pwd)"
tmpdir="$(mktemp -d /tmp/duumbi-689-scaled-smoke.XXXXXX)"
"$repo/target/debug/duumbi" init "$tmpdir"
cd "$tmpdir"
"$repo/target/debug/duumbi" benchmark \
  --suite scaled \
  --smoke \
  --provider minimax:auto:primary:MINIMAX_API_KEY \
  --attempts 1 \
  --output "$tmpdir/duumbi-689-scaled-smoke.json"
```

## Result Summary

| Task | Tags | Result | First pass | Repair attempted | Failure category | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `scaled_math_pipeline` | scaled, multi_function, single_module, i64 | fail, 0/3 tests | false | false | `logic_error` | provider-backed intent execution ran; verifier tests failed |
| `scaled_cross_module_stats` | scaled, multi_module, cross_module, i64 | fail, 0/3 tests | false | false | `logic_error` | provider-backed intent execution ran; verifier tests failed |
| `scaled_http_sqlite_json` | scaled, http, sqlite, json, process_evidence | fail, 0/0 verifier tests | false | false | `evidence_required` | current verifier does not check HTTP JSON payload semantics |

Aggregate values:

- total results: 3
- successes: 0
- first-pass successes: 0
- repair attempts: 0
- repair successes: 0
- unrecovered failures: 3
- usage available: 0
- usage unavailable: 3
- usage unavailable reasons:
  - `provider_response_did_not_expose_usage`: 2
  - `process_evidence_not_executed`: 1
- dominant error codes/signals:
  - `broader_evidence_required`: 1
- top failure patterns:
  - `logic_error`: 2
  - `broader_evidence_required`: 1

## Interpretation

This run is intentionally not a success badge. It proves the scaled benchmark
surface can select and report multi-function, cross-module, and
HTTP/SQLite/JSON evidence cases under a constrained MiniMax run, and it shows
that the current write path did not pass the selected scaled i64 verifier cases
on the first attempt.

The HTTP + SQLite + JSON row is deliberately reported as
`evidence_required`. The current i64 verifier cannot prove loopback HTTP JSON
payload semantics, so the benchmark records the verification gap instead of
silently passing the task.

## Known Limitations From This Run

- The scaled smoke subset produced 0/3 successful task results.
- No selected task succeeded on first pass.
- No repair cycle was attempted in this run.
- Provider usage and token/cost data were unavailable from the benchmark report
  path used by this run.
- HTTP + SQLite + JSON composition is present in the scaled corpus, but it
  still needs automated process evidence before it can be counted as generated
  service behavior.

## Follow-Up Failure Patterns

1. `logic_error`: provider-backed scaled i64 tasks completed without passing
   verifier tests. The next implementation/research target should inspect the
   generated temp workspaces or add sampled evidence paths to identify whether
   failures are decomposition, cross-module exports/calls, verifier mismatch,
   or generated graph logic.
2. `broader_evidence_required`: the benchmark can label HTTP/SQLite/JSON
   composition honestly, but needs a bounded process evidence checker before
   this row can validate actual service output.
