# DUUMBI-719 MCP Agent Benchmark Plan

Related to #719.

This file defines the benchmark evidence expected before Stage 11. It is a plan
and evidence placeholder until the full MCP-only flagship transcript is
available.

## Scenario

Canonical scenario:

- `examples/flagship-http-sqlite-json`
- expected local behavior: build the example, run the local service, send one
  loopback HTTP request, and verify the JSON response body.

## MCP-Only Path

Required transcript steps:

1. `initialize`
2. `tools/list`
3. `mcp_capability_status`
4. workspace status or initialization tool once available
5. dependency/vendor status or structured unavailable report
6. graph or intent inspection
7. build through `build_compile`
8. run through `build_run`
9. evidence retrieval through `mcp_evidence_status`

## Raw Rust Baseline

The baseline should use the same user-visible target behavior with an agent
editing or generating raw Rust directly. Record:

- prompt;
- agent/provider;
- elapsed time;
- turns;
- token and cost availability;
- pass/fail;
- failure category when any;
- exact output or reason unavailable.

## Reporting Rules

- Use `unavailable` when tokens, cost, or model telemetry cannot be measured.
- Do not compare superiority unless both paths have measured evidence.
- Do not include secrets or raw provider credentials.
- If the external-agent run would exceed USD 1 expected external LLM cost, stop
  and request Stage 10 resource approval before running it.

## Current Evidence

Current status: partial, source-recorded on 2026-06-17.

Implemented and verified:

- `mcp_capability_status` reports build/run and evidence retrieval available.
- `mcp_evidence_status` returns bounded read-only local evidence metadata.
- `build_compile` and `build_run` execute through the shared workspace backend
  and report output path, stdout, stderr, exit code, and timeout state.
- Provider-free stdio smoke through `target/debug/duumbi mcp` passed:
  `initialize`, `tools/list`, `mcp_capability_status`, and
  `mcp_evidence_status`.
- Automated checks passed:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --all`
  - focused MCP tests for server dispatch, query, approval, build/run,
    evidence status, and DUUMBI-719 integration coverage.

Structured blockers and limitations:

- Full MCP-only flagship transcript remains partial because dependency/vendor
  MCP tools still report structured unavailable state. The current accepted
  path can report that blocker through `mcp_capability_status`, but cannot yet
  materialize the flagship dependency state entirely through MCP.
- Low-cost live external-agent E2E was not run in this Stage 10 pass. No
  external agent/provider call was invoked, external cost was USD 0, and the
  run should be treated as blocked pending a separately routed live-agent
  exercise or an explicit decision that the structured MCP smoke is sufficient
  for Stage 11 review.
- Raw Rust baseline is unavailable in this Stage 10 pass because no comparable
  external agent was run against the raw Rust target. Token, cost, elapsed time,
  turns, and provider telemetry are therefore unavailable rather than measured.

Automated source evidence already present:

- `tests/integration_duumbi688_flagship_example.rs`
- `examples/flagship-http-sqlite-json/README.md`

The final Stage 10 implementation PR must carry this partial evidence honestly.
Do not claim MCP-only benchmark success or raw Rust superiority until a live
transcript and comparable baseline are recorded.
