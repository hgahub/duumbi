---
name: duumbi-inbox-enrichment
description: "Prepare captured DUUMBI Inbox notes for Stage 4, preserving the original input and ownership; record ready_for_triage or needs_clarification without creating execution work."
---

# DUUMBI Stage 3b — Inbox preparation

Read [the shared contract](../../../docs/automation/intake-contract.md). Select only
notes whose top-level frontmatter has `intake_status: captured`; never infer
readiness from headings or old processed tags. Sources may be Codex, Grok Bot,
or manual Obsidian. Missing/invalid status requires explicit migration or correction.

Inspect relevant active vault guidance and related context. Preserve original
input, source, owner, intake ID, and clarification answers. Write English
preparation in the delimited generated block, replacing that block on a later pass.
Classify, summarize, distinguish true repeats from new related requirements, and
recommend routing. No GitHub/Atlas writes, specs, implementation, or archiving.

First resolve questions from available context. Essential missing human intent or
evidence: record 1–3 concrete questions and why they block triage; set
`needs_clarification`. Minor uncertainty stays open without blocking preparation.
Otherwise set `ready_for_triage`; record duplicate/no-action recommendations in
`enrichment_result` for Stage 4 disposition, not as approval or automatic deletion.

The scheduled workflow handles at most one captured note per run, pushes its
changes, then sends a Slack completion/clarification handoff. Use the note owner,
falling back to the configured Inbox owner. Clarification notifications link the
note and explain continuation in Codex or Grok. Waiting notes must not repeat
model calls or notifications on subsequent scheduled runs. A failed Slack send
is reported in the workflow; the durable questions remain in the note.

For manual operation, use a clean isolated vault checkout, commit only the
changed notes, push without force, and verify the remote result. Preserve a draft
and report conflicts or failed synchronization. Do not mark a local save as a
completed cloud handoff. Report ready notes, waiting notes, owner, commit, and
notification outcome. Stage 4 owns disposition and archive moves.
