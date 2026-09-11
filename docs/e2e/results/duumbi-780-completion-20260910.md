# DUUMBI-780 completion evidence — 2026-09-10

Historical platform evidence: native Windows support was subsequently retired
in [#800](https://github.com/hgahub/duumbi/issues/800). The results below describe
the source preserved at `windows-support-final-2026-09-10`, not current support.

Related to [#780](https://github.com/hgahub/duumbi/issues/780), implementing the
[accepted branch decision](https://github.com/hgahub/duumbi/issues/780#issuecomment-5620459440).
This supersedes the per-attempt-port limitation in the earlier local evidence.

## Local application execution before PR creation

Host: macOS arm64, Rust 1.95.0. Branch:
`codex/duumbi-780-process-evidence`, rebased onto current `main@0c05f4c`
(already up to date). No manual Cargo.lock edits were required.

```sh
DUUMBI_780_EVIDENCE_DIR=/tmp/duumbi780-native-evidence \
  cargo test --test integration_duumbi780_process_evidence
```

**5/5 passed**, with actual initialized workspaces, DUUMBI native builds,
in-memory SQLite, and loopback HTTP. Each successful verification launches the
compiled service twice, requests `/facts` on the readiness socket, validates
exact JSON values and types, and verifies clean exit/reaping and closed listener.

| Path | Result | Retained evidence |
| --- | --- | --- |
| First-pass fixture | Passed, 1/1 behavioral check, no repair | [First pass](duumbi-780-completion-20260910/first-pass.json) |
| Incorrect service value, then repair | Initial assertion failed; normal graph repair succeeded; both datasets passed | [Repair passes in order](duumbi-780-completion-20260910/repaired.json) |
| Two-attempt replay with Pass fixture | Both passed; intent, final exact and final semantic hashes equal; each signature ends in `;process=passed` | [Replay report](duumbi-780-completion-20260910/replay.json) |
| Unrepaired mismatch | `process_assertion`, `product_logic_mismatch`; execution log verifies cache dependency additions and named preflight waiver | Integration assertion |
| Failed authoring | Original failure preserved, process `not_run` | Integration assertion |

Reports preserve original run-relative artifact metadata; process artifacts
referenced by the first-pass and repair reports are included alongside them.
The replay report contains full embedded process evidence; its temporary run
workspace and ledger are not retained here. Offline fixtures make zero external
LLM calls and prove the harness, not model authoring performance.

The real native test caught a macOS TIME_WAIT issue in a plain bind probe.
The corrected availability check uses SO_REUSEADDR on Unix, matching the
runtime. Following automated review, Windows uses a bounded, read-only TCP-table
check before permitting a reuse bind for stale TIME_WAIT, rejecting live IPv4
and IPv6 endpoints. Suspended child creation now ensures Job Object assignment
precedes execution, with explicit suspension/membership/cancellation tests.
A dedicated portable test keeps a real port occupied and verifies Start
infrastructure attribution without launching a child or requesting graph repair.

## Live provider attempt

One benchmark attempt ran in `/tmp/duumbi780-live`, with workspace provider
configuration explicitly selecting `openai`, `gpt-5.6-luna`, and
`OPENAI_API_KEY`, with a 60-second provider timeout. No alternative model or
provider was selected. Command:

```sh
duumbi benchmark --showcase scaled_http_sqlite_json --attempts 1 \
  --provider openai:gpt-5.6-luna --output /tmp/duumbi780-live/result.json \
  --artifact-dir /tmp/duumbi780-live/artifacts --keep-workspaces
```

Result: **failed before graph generation**, process status **not_run**, 1.874 s.
The first LLM request returned HTTP 400 `invalid_request_error`: function tools
with reasoning are unsupported for this model on `/v1/chat/completions`; the
provider response asks for `/v1/responses` or `reasoning_effort: none`.

- [Sanitized report](duumbi-780-completion-20260910/live-result.json)
- [Allowlisted provider error excerpt](duumbi-780-completion-20260910/live-provider-error.json)

The existing text taxonomy labels this error `schema_error` /
`control_flow_or_ssa`; **that classification is not evidence of a generated
graph defect**. The HTTP 400 and allowlisted error excerpt are the controlling observation.
The full local execution log is deliberately not published.
No generated service was built or launched in this live attempt. Provider
request compatibility and general authoring-error taxonomy are outside #780's
approved process-verifier changes. A successful live authoring claim remains
unsupported. Actual usage/cost is unavailable in the report; there was one
rejected request and no successful generation (planning estimate below USD 1).

## Verification and review

Before PR creation: `cargo test --all` passed **3060 tests**, failed **0**,
and ignored **1** existing test. The focused library suite passed **137** tests;
format, clippy (`--all-targets -- -D warnings`), diff checks, and all pre-commit
hooks passed. Ubuntu/Windows CI evidence is recorded in the PR. Local source
review covered stable per-run inputs, verifier layering, preflight scope,
cache version selection, failure attribution, lifecycle cleanup and the
accepted stdin-SQL burden. Additional stage timings and per-field assertion
records (optional decision item 9) remain deferred; the existing versioned
contract, failure kind/stage and ordered process records are retained.

### Automated-review follow-up

The initial head `cb14c80` passed both CI platforms and coverage. Codex identified
three actionable findings, addressed in the follow-up: Windows TCP TIME_WAIT
inspection before port reuse, suspended creation before Job Object assignment,
and cache materialization on `spawn_blocking`. The Windows-specific helper and
tests also passed an isolated cross-target clippy check using Rust's Windows GNU
target; actual Windows execution is still validated by PR CI.

Concurrent local native tests exposed a transient socket-release handoff: a
failed bind became available within 250 ms, with no active listener observed.
The pre-launch probe now retries AddrInUse asynchronously for that bounded
window; a persistent occupied port still fails without launching the service.
The temporary socket-ownership diagnostic code was removed before commit.

The full local suite after these lifecycle fixes passed **3062 tests**, with
**0 failures** and **1 existing ignored test**; the focused suite passed **138**.
A second review reproduced a Linux-container-only test failure: `kill(pid, 0)`
counts an unreaped zombie as present. The test's Linux liveness probe now reads
the process state asynchronously, treating dead/zombie states as terminated.
A Linux regression deliberately retains a killed child until after the probe,
so it verifies this behavior without depending on PID 1's reaping policy.
Production cleanup is unchanged by this final test correction. Local process
tests (**13 passed**), all-target clippy, and pre-commit checks pass; final-head
Ubuntu and Windows execution evidence is recorded in the PR.
