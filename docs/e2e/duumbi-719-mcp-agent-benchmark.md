# DUUMBI-719 MCP Agent Benchmark Plan

Related to #719.

This file defines the benchmark evidence expected before Stage 11. It is a plan
and evidence placeholder until the build/run/evidence MCP tools are wired.

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
7. build through MCP or structured blocked evidence
8. run through MCP or structured blocked evidence
9. evidence retrieval once available

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

Current status: blocked on remaining MCP build/run/evidence tooling.

Automated source evidence already present:

- `tests/integration_duumbi688_flagship_example.rs`
- `examples/flagship-http-sqlite-json/README.md`

The final Stage 10 implementation PR must replace this current-status section
with either live benchmark evidence or a structured blocked report.
