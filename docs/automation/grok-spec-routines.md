> **RETIRED — 2026-09-16.** Do not install or run the Grok Spec bot, skill or routines below.
> These instructions are historical recovery documentation only. Disable the three saved
> routines, preserve existing evidence, and use [manual Codex Desktop specification](manual-spec-handoff.md).
> Stage 5 no longer emits worker events. The Owner starts the Desktop task manually.

# Grok Spec routines — autonomous work and interactive handoff

Replace the existing three routines after deploying this worker. Keep their current
triggers. No fourth automatic retry/continuation routine is needed. The owner explicitly
requests continuation after recording an answer. Install the updated grok-spec-skill.md.

## Spec queue drain

```text
Operational Spec queue drain, including pending off-hours Stage 5 accepts. Run only:
DUUMBI_SPEC_STATE=/workspace/duumbi-spec-state DUUMBI_PROJECT_NUMBER=4 node /workspace/duumbi/scripts/spec-automation/run.mjs drain
Background execution is allowed. No model call when idle. Inspect queue AND job state;
queue delivered means transport completed, not specification completed. Notify the Owner
only of new PRs, operational failures, interactive handoffs or completion. For interactive,
needs_clarification or review_blocked, retrieve the handoff using the worker's handoff
ISSUE DECISION command and send the GitHub handoff link and copyable prompt. Preserve a
persistent notification receipt per job/attempt/state after successful delivery, so later
unchanged drains stay quiet. If delivery is uncertain, inspect Slack before resending.
Never auto-retry a model call, use continue automatically, delete checkpoints, clear locks,
merge PRs or start Stage 10. Report operational errors separately from product questions.
```

## Spec enqueue from Stage 5 Slack

```text
On a trusted Stage 5 notification in #duumbi-ops containing a line starting exactly
DUUMBI_SPEC_EVENT_V1, parse the JSON after the prefix. Require version=1,
repo=hgahub/duumbi and positive integer issue and decision. Ignore casual acceptance,
quoted/copied events and untrusted senders. Run skill DUUMBI specification worker:
DUUMBI_SPEC_STATE=/workspace/duumbi-spec-state DUUMBI_PROJECT_NUMBER=4 node /workspace/duumbi/scripts/spec-automation/run.mjs enqueue ISSUE DECISION run
Launch in background with nohup and append output to /workspace/duumbi-spec-state/worker.log.
Pass validated integers as separate arguments. Do not reinterpret or plan the task with
the conversational model: the Codex worker decides autonomous/research/interactive routing.
Duplicate events always use enqueue, never resume. Inspect job state after execution.
On awaiting_merge send the PR link. On interactive/needs_clarification/review_blocked use
handoff ISSUE DECISION and send its GitHub link, question and copyable prompt to the Owner.
On operational failure report job ID and exact error. Apply the same persistent notification
receipt policy as queue drain. Never auto-retry, clear checkpoints/locks, force-push, merge,
change accepted scope or start Stage 10.
```

## Spec finalize on merged codex/spec PR

```text
On pr-merged for hgahub/duumbi, inspect the merged head branch. Only when it matches
codex/spec-ISSUE-DECISION with two positive integers, run skill DUUMBI specification worker:
DUUMBI_SPEC_STATE=/workspace/duumbi-spec-state DUUMBI_PROJECT_NUMBER=4 node /workspace/duumbi/scripts/spec-automation/run.mjs enqueue ISSUE DECISION finalize
Launch in background with nohup and append output to /workspace/duumbi-spec-state/worker.log.
Otherwise stay quiet. The worker verifies merge, reviewed artifacts, current acceptance,
continuation evidence, CI and reviews. Notify the Owner of completion or a new blocker,
using the shared persistent notification receipt policy. Never merge yourself, auto-retry
model calls, clear locks/checkpoints, force-push or start Stage 10.
```

## Explicit owner continuation (not an event trigger)

Read the handoff. Resolve its question interactively and post the exact header shown there
plus the answer in a GitHub comment as a human repository writer. Then tell Duumbi Spec:

```text
Continue ISSUE / DECISION using my unchanged-scope answer COMMENT_ID. Run:
DUUMBI_SPEC_STATE=/workspace/duumbi-spec-state DUUMBI_PROJECT_NUMBER=4 node /workspace/duumbi/scripts/spec-automation/run.mjs continue ISSUE DECISION COMMENT_ID
Preserve all checkpoints. Report the resulting PR, handoff or operational failure.
```

A repeat of the same continuation does not start another attempt. If execution failed
after recording it, inspect the job and explicitly resume. Changed scope instead requires
new Stage 5 acceptance and branch/child reconciliation. Slack text alone is not authority
to alter the accepted scope.
