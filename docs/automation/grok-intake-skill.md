# Skill recipe to save in Grok Bot

**name:** duumbi-grok-intake

**description:** Capture an explicitly submitted idea into the configured planning
vault Inbox, or clarify an existing needs_clarification note. Interpret the request,
check related context, preserve source and ownership, synchronize one note, and
stop before triage or implementation. Merely discussing this skill does not capture anything.

**body:**

Use the configured intake workspace and shared contract. Configuration belongs
in this Bot's profile, memory, or routine: vault repository, vault checkout,
intake tooling checkout, default owner, and any optional owner Slack ID. Do not
bake account-specific repository names, user IDs, channels, or credentials into
this reusable recipe. If configuration or authenticated access is missing,
report it rather than writing to a substitute location.

1. Read the tooling checkout's `docs/automation/intake-contract.md`. Treat it as
   the shared schema and state-machine contract. Use the user's language in chat
   and English in the durable note.
2. For an explicit new capture, interpret the problem, affected user, desired
   outcome, and exclusions. Inspect only relevant current vault guidance. Check
   Inbox, Processed Inbox, Atlas, and relevant GitHub context read-only for
   duplicates. A related topic with new evidence or requirements is not a duplicate.
   Exact repetition returns the existing item.
3. Ask up to three focused questions when missing information materially changes
   capture. Do not produce specifications or implementation work. Preserve minor
   uncertainty as open questions.
4. Create exactly one English Markdown note under the configured Inbox with a
   date/title filename, stable UUID `intake_id`, `source: grok`, explicit
   `intake_owner`, `intake_status: captured`, and timestamp. Use the shared template.
   Explicit capture authorizes this bounded save and sync, subject to the Bot's
   configured approval policy. A question about the skill alone does not.
5. Synchronize using `scripts/intake/sync-note.mjs` from the tooling checkout.
   For a new note use expected blob `absent`. The helper must report verified
   success before you say the note is available to Stage 3b. A local-only save
   is partial completion; retain the draft and report the error.
6. For an existing clarification, fetch the latest remote note and record its
   blob SHA before editing. Keep the same ID, filename, source, and owner. Answer
   from context where possible; ask the owner to decide what remains unclear.
   Append dated answers outside the generated enrichment block. Return to
   `captured` only when blocking questions are resolved; partial answers keep
   `needs_clarification`. Sync using the recorded expected blob, and stop on a
   concurrent change until it is reconciled.
7. Reply with the verified note URL, owner, interpretation, open questions,
   intake status, and remote commit, or clearly say saved locally / sync failed.
   Do not create Issues/PRs, write Atlas artifacts, merge, deploy, start triage,
   or implement the idea. Stage 3b prepares, Stage 4 disposes/routes the note.

Installing this recipe alone must not run it, create a note, or create a routine.
