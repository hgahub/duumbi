> **RETIRED — 2026-09-16.** Do not install or run the Grok Spec bot, skill or routines below.
> These instructions are historical recovery documentation only. Disable the three saved
> routines, preserve existing evidence, and use [manual Codex Desktop specification](manual-spec-handoff.md).
> Stage 5 no longer emits worker events. The Owner starts the Desktop task manually.

# DUUMBI specification worker

**name:** DUUMBI specification worker

**description:** Process a trusted human-acceptance event with the configured persistent
Codex CLI specification worker, or finalize its spec PR after a human merge. Covers Stage
6–9 only and stops at Ready for Build.

**body:**

1. Read repository/checkout/state-directory/project and trusted event-sender settings from
   this bot's configuration. Never embed credentials in this recipe.
2. Require a versioned acceptance event with positive integer issue and decision comment IDs,
   or a merged spec-PR notification mapping to those IDs. Do not infer acceptance yourself.
3. Use the configured checkout's `scripts/spec-automation/run.mjs`:
   - acceptance: `enqueue ISSUE DECISION run`
   - merged PR: `enqueue ISSUE DECISION finalize`
   Pass numbers as separate arguments, never shell-interpolate arbitrary event text.
4. `enqueue` persists and deduplicates events; the worker serializes jobs. The configured
   lightweight five-minute routine runs `drain` to service remaining pending events.
   A busy worker leaves the event queued; do not run another host. A runtime failure means report
   to the owner and preserve the checkpoint, not automatic repeated model calls.
5. Report the returned job state or spec PR link. Await human merge when requested. On an
   operational blocker give the error and the recovery command from grok-spec-setup.md.
   For interactive/needs_clarification/review_blocked, retrieve `handoff ISSUE DECISION`
   and send the durable GitHub handoff link plus copyable prompt once per attempt.
   Only on explicit owner request run `continue ISSUE DECISION COMMENT_ID`; the referenced
   human-writer comment must confirm unchanged scope with the exact attempt header.
   Changed scope needs renewed Stage 5 acceptance and reconciliation. Never erase checkpoints.
   Use the notification receipt policy in grok-spec-routines.md.
6. Never choose a paid API fallback, change model IDs, clear locks, force-push, merge PRs,
   bypass a failed gate, launch delivery-autopilot, or start Stage 10.

The worker, not the bot's conversational model, checks current GitHub acceptance, performs
bounded Codex generation/review, writes specs and finalizes status. A received Slack event
is not evidence that the job completed.
