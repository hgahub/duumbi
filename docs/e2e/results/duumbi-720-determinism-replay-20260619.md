# DUUMBI-720 Determinism Replay E2E Evidence

Related to #720.

Date: 2026-06-19

## Command

Run from a temporary workspace with a workspace-local `.duumbi/config.toml`
containing only:

```toml
[[providers]]
provider = "minimax"
role = "primary"
api_key_env = "MINIMAX_API_KEY"
timeout_secs = 120
```

Replay command:

```bash
/Users/heizergabor/.codex/worktrees/bf92/duumbi/target/debug/duumbi determinism replay \
  --suite core \
  --showcase calculator \
  --provider minimax:auto:primary:MINIMAX_API_KEY \
  --attempts 2 \
  --artifact-dir /private/tmp/duumbi-720-live-output/replays \
  --output /private/tmp/duumbi-720-live-output/duumbi-720-replay.json \
  --markdown-output /private/tmp/duumbi-720-live-output/duumbi-720-replay.md
```

Exit status: 0.

## Report Evidence

- Schema: `duumbi.determinism.replay_report.v1`
- Run id: `duumbi-720-2026-06-19T07-03-49Z-core-full`
- Provider source: `workspace`
- Provider route: `minimax:auto:primary:MINIMAX_API_KEY`
- Model identity: `minimax/MiniMax-M2.7-highspeed`
- Attempts: 2
- Attempt 1: success, 4/4 tests passed
- Attempt 2: success, 4/4 tests passed
- Prompt hashes: `partial`, with `intent_spec`, `bdd_context`, and `context_pack`
- Provider usage: unavailable, reason `provider_response_did_not_expose_usage`
- Rewrite comparison: `not_yet_comparable`

## Metrics

| Metric | Status | Rate | Comparable Attempts |
| --- | --- | ---: | ---: |
| Exact graph agreement | available | 0.500 | 2 |
| Semantic graph agreement | available | 0.500 | 2 |
| Behavioral agreement | available | 1.000 | 2 |
| Failure category agreement | unavailable | n/a | 0 |

## Ledger Evidence

The final run ledger had 7 JSONL events:

- `run_started`
- `task_selected`
- `attempt_started`
- `attempt_completed`
- `attempt_started`
- `attempt_completed`
- `run_completed`

## Safety Check

Secret scan command:

```bash
grep -R "sk-\|Bearer \|MINIMAX_API_KEY=.*\|api_key =" /private/tmp/duumbi-720-live-output || true
```

Result: no matches.

Raw replay bundles and provider logs were not committed.
