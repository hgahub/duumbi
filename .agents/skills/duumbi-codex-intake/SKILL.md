---
name: duumbi-codex-intake
description: "Capture a DUUMBI idea into the planning Inbox and synchronize it to GitHub, or clarify an existing needs_clarification note. Use for explicit capture or refinement requests in Codex; discussing this skill alone does not create a note."
---

# DUUMBI Stage 2 — Codex intake

Capture one idea or continue one existing Inbox note. Use the user's language in
conversation and English in the note. Read [the shared intake contract](../../../docs/automation/intake-contract.md)
for metadata, the note template, synchronization commands, and state transitions.

## Destination and boundary

Write only to the configured `duumbi-vault` repository under
`Duumbi/00 Inbox (ToProcess)/`. The local default is
`/Users/heizergabor/space/hgahub/duumbi-vault`. Verify the Git root and destination;
never create `Duumbi/` inside the source repository. If the vault or remote access
is unavailable, preserve the draft and report the missing dependency.

No issue/PR creation, implementation, Atlas updates, triage, or product acceptance.
GitHub may be read to verify duplicates or existing work, but an uninspected state
must be recorded as `Not inspected`.

## Capture

1. Distinguish an explicit capture/refinement request from discussion. An explicit
   request to save/process an idea authorizes this bounded capture and sync;
   do not ask for redundant confirmation. Ask only when intended scope, a major
   interpretation, or sensitive content needs the user's decision.
2. Inspect the current vault guidance (`How to use.md`, runbook, relevant PRD,
   Glossary, Agentic Development Map sections) and only the context needed for
   this idea. Search Inbox, Processed Inbox, Atlas, and relevant GitHub context.
   Exact repeat: return the canonical item. Related topic with a new requirement
   or new evidence: capture the difference and link the related item.
3. Clarify the problem, affected user, desired outcome, and scope exclusions.
   Ask up to three questions only if missing answers materially affect capture.
   Preserve other uncertainty as open questions. Do not draft a full spec here.
4. Use `YYYY-MM-DD - Short English title.md`, the current local date, and a new
   UUID `intake_id`. Create the file exclusively; never overwrite a title collision.
   Use `source: codex`, `intake_status: captured`, and an explicit owner (the
   verified submitting GitHub login, or the configured Inbox owner). Record a
   conversation reference when available. Keep source facts and assumptions separate.
5. Save the note and use the shared sync helper with `--expected-blob absent`.
   Report `synced` only after remote verification. A failed push means **saved
   locally, not synchronized**; do not create a second note or claim Stage 3b can see it.

## Clarify an existing note

1. Fetch the vault's `origin/main` and read the named note at that ref. Preserve
   conflicting local edits instead of overwriting them. Record its blob SHA
   before editing (`git rev-parse "origin/main:<relative-note-path>"`).
2. Verify `needs_clarification`. Read the generated questions, blocking reason, and any `Stage 4 clarification`,
   inspect available context, and ask the owner only what remains unresolved.
3. Keep the same filename, `intake_id`, source, owner, original input, and previous
   answers. Append dated answers under `## User clarifications` outside the
   generated enrichment block. Do not invent a decision for the user.
4. When all blocking questions have answers, change `intake_status` back to
   `captured` and update `intake_updated_at`. Keep the prior generated block as
   evidence; Stage 3b replaces it on the next pass. If answers remain missing,
   keep `needs_clarification` and do not claim resubmission.
5. Sync with the previously recorded `--expected-blob`. The helper rejects
   concurrent same-note changes. Reconcile them explicitly before retrying.

## Reply

Return the note link/path, interpreted intent, classification, open questions,
owner, intake status, and sync result (remote commit if verified). Explain that
Stage 3b prepares the captured note; Stage 4 only takes `ready_for_triage` notes.
Do not describe capture or enrichment as accepted development work.
