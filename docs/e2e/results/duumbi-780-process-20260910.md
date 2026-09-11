# DUUMBI-780 local process E2E evidence — 2026-09-10

Historical platform evidence: native Windows support was subsequently retired
in [#800](https://github.com/hgahub/duumbi/issues/800). See the archival tag
`windows-support-final-2026-09-10` for the final source with Windows support.

Issue: <https://github.com/hgahub/duumbi/issues/780>

Historical evidence from the initial branch. See the [completion record](duumbi-780-completion-20260910.md)
for stable per-run ports, two-attempt replay and the live provider attempt.

The benchmark and replay runners exercised real native HTTP/SQLite/JSON
processes after deterministic provider mutations. The first-pass and repaired
benchmark runs both passed. This validates the execution and verification
machinery; live model authoring performance was not measured.

## Environment and method

- Host: macOS, Darwin arm64.
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`.
- Worktree: `codex/duumbi-780-process-evidence`, based on `0c05f4c`.
- Provider: injected offline fixture derived from the existing flagship graph,
  with runtime SQL input and authoring-validator Result guards.
- External LLM calls: 0; external LLM cost: USD 0.
- Native compiler, bundled SQLite runtime, and real loopback TCP/HTTP were used.

Executed command:

```sh
DUUMBI_780_EVIDENCE_DIR=docs/e2e/results/duumbi-780-process-20260910 \
  cargo test --all
```

The full workspace run passed **3050 tests**, failed **0**, and ignored **1**
existing test. The final small toolchain-error classification addition was
then covered by the focused process tests. `cargo clippy --all-targets -- -D
warnings` and formatting checks also passed.

## Observed outcomes

| Path | Result | Duration | Evidence |
| --- | --- | --- | --- |
| Benchmark, first pass | Success; no repair; 1/1 process check | 2.786 s | [Report and embedded process records](duumbi-780-process-20260910/first-pass.json) |
| Benchmark, deliberately wrong service value | Initial assertion failure; normal repair applied; fresh verification passed | 3.965 s | [Recovered failure and successful rerun](duumbi-780-process-20260910/repaired.json) |
| Benchmark, repair returns no patch | Failed; `product_logic_mismatch`, `logic_error`, `process_assertion` | Test assertion | `unrepaired_json_mismatch_remains_a_product_logic_failure` |
| Benchmark, authoring fails | Failed with original mutation category; process `not_run`; provider was invoked | Test assertion | `authoring_failure_is_preserved_and_process_is_not_run` |
| Determinism replay | Success; 1/1 process check; same process evidence contract | Test assertion | `determinism_replay_runs_the_same_process_contract` |

Both successful verification passes observed:

```json
{"service":"scaled-http-sqlite-json","route":"/facts","count":1,"first_fact":"Ada Lovelace","storage":"sqlite-memory"}
{"service":"scaled-http-sqlite-json","route":"/facts","count":2,"first_fact":"Grace Hopper","storage":"sqlite-memory"}
```

Every successful child exited with 0 and was reaped. Tests confirmed that its
listener was gone. Retained `process-evidence.json` files were read and checked
after the isolated application workspace had been dropped; copies accompany
the exported reports under their original relative artifact paths.

## Coverage and limits

Focused tests cover exact values/types/status, malformed and oversized HTTP,
request/response deadlines, missing compiler infrastructure, bounded/redacted
logs, noisy-child pipe draining, early exit, child termination and reaping,
Unix descendant cleanup after cancellation, and pre-launch binding checks
through forwarded function parameters. No public listener is launched by the
public/unknown-host rejection test; it tests graph analysis only.

An early native run exposed macOS-specific behavior in an alternate-loopback
probe. That probe was removed in favor of bounded pre-launch binding analysis;
the succeeding full run used the replacement.

Windows Job Object support and portable tests are included, but Windows and
Linux execution were not available in this local run. The existing CI matrix
covers those platforms when a PR is opened; this task intentionally creates no
PR. The process contract and conservative binding-analysis limits are documented
in [the verifier guide](../duumbi-780-process-evidence.md).
