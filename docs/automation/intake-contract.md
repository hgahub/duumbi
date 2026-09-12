# DUUMBI intake contract

Codex, Grok Bot, and manual Obsidian capture converge on the planning vault Inbox.
GitHub Issues and Discussions are execution/duplicate context, not entry points.
Slack provides notifications and clarification handoffs, not idea submission.

## State machine

| Frontmatter `intake_status` | Owner | Next action |
|---|---|---|
| `captured` | Stage 3b | Prepare the note once |
| `needs_clarification` | Submitter / Inbox owner, assisted by Codex or Grok | Answer blocking questions in the same note |
| `ready_for_triage` | Stage 4 | Decide execution, knowledge, duplicate, defer, or no action |
| `triaged` | Stage 4 | Archive with disposition evidence |

Clarification answers return the same note to `captured`. A `ready_for_triage`
result is not acceptance, approval, or permission to implement. Duplicate/no-action
recommendations remain ready for Stage 4 disposition. The scheduled queue refill
handles execution candidates; run `duumbi-triage` for knowledge-only or other
non-execution dispositions. It must not fabricate an issue to empty the Inbox.

Only top-level frontmatter is authoritative. Runtime ignores missing, malformed,
or unknown statuses; text in a note body and historical processed markers cannot
select a note. The supported metadata is a flat YAML string mapping; quote values
containing punctuation. Other properties and original body content are preserved.

## Capture note

Location: `duumbi-vault/Duumbi/00 Inbox (ToProcess)/YYYY-MM-DD - Short title.md`.
Use the current local date, a short English title, and exclusive file creation.

```markdown
---
intake_id: "<UUID, stable across clarification rounds>"
intake_status: captured
source: codex
intake_owner: "<submitter or configured Inbox owner>"
intake_updated_at: "<ISO timestamp>"
---
# Short title

## Source
- Conversation: <reference if available>

## Raw input
<source excerpt or summary, preserving meaningful terms>

## Problem
<problem and evidence>

## Affected user
<who or which workflow>

## Desired outcome
<what should become possible or improve>

## Out of scope
<exclusions or unknown>

## Interpreted intent
<working interpretation>

## Classification
<idea, bug, feature, research, architecture, execution, knowledge, skill, unclear>

## Related context
<inspected notes / GitHub links; say Not inspected when appropriate>

## Open questions
<remaining uncertainty>

## Requested follow-up
<requested later work>

## Notes
- Facts:
- Assumptions:
- Recommendations:
```

Allowed sources are `codex`, `grok`, and `obsidian`. A manual note may start with
only `intake_status: captured` plus real content. Stage 3b supplies an ID, source
`obsidian`, and the configured default owner. It does not guess an author from
prose. Set `intake_owner` to override the owner. Optional `intake_owner_slack_id`
can target a Slack mention; it does not change the notification channel.

A similar topic is not enough to declare a duplicate. Preserve new requirements
or evidence and link the related item. An exact repeat returns the canonical item.

## Save and synchronize

A local file is not visible to GitHub Actions until it reaches vault `main`.
Node.js 22+ and authenticated Git access are required. Keep a current checkout of
the DUUMBI source repository for the helper; it has no npm dependencies.

Before editing an existing note, fetch vault `origin/main`, read that version,
and record its blob SHA. Do not overwrite conflicting local changes:

```sh
git -C "$VAULT" fetch origin main
git -C "$VAULT" rev-parse "origin/main:$NOTE"
```

For a new note, expected blob is `absent`. Write the file, then run from the
source-repository root (replace the variables with your configured paths):

```sh
node scripts/intake/sync-note.mjs --vault "$VAULT" --note "$NOTE" --expected-blob "$BASE_BLOB"
```

`NOTE` is relative to the vault root and starts with `Duumbi/00 Inbox (ToProcess)/`.
The helper uses an isolated clone, commits only that note, pushes `HEAD:main`
without force, and verifies the remote blob. It never stages or publishes other
local changes. Same-note conflicts fail; one unrelated remote-branch advance may
be retried after refreshing. The same intake ID under another Inbox/archive
filename is rejected. An already identical remote note is a successful no-op.
The source checkout's branch, index, and local draft remain untouched; the source
checkout can therefore remain behind remote main after a successful publication.
Reconcile or refresh it before subsequent manual Git work.

Reply with `saved locally` on failure and `synced` only on verified success, plus
the remote commit and note link. A skill invocation about the skill itself is
not a capture request. Explicitly requesting capture/refinement authorizes this
bounded note operation; do not ask for a second generic confirmation.

## Clarification and ownership

Stage 3b first uses available evidence. Only missing human intent or evidence
that materially blocks triage triggers `needs_clarification`, with a reason and
1–3 concrete questions. Nonblocking uncertainty stays open in a ready note.

After pushing the questions, it sends one Slack notification with the owner,
note link, and a continuation prompt. No user questions or raw idea text are put
in workflow metrics. The actual questions stay in the linked note. The owner
continues via Codex or the installed Grok intake skill. Keep the same ID/source,
append dated answers under `## User clarifications` outside the generated block,
and return to `captured` only when blockers are resolved. Partial answers remain
`needs_clarification`. Both can be synchronized with the original remote blob.

Waiting notes are not selected by scheduled Stage 3b or Stage 4, so they do not
repeat model calls or Slack notifications. Notifications are attempted after a
successful vault push only. A failed Slack send remains a visible workflow
warning; do not automatically rerun enrichment to resend it. The durable note
and the workflow summary identify the waiting item and its owner.

Repo variables: `DUUMBI_INBOX_OWNER` (defaults to GitHub repo owner), optional
`DUUMBI_INBOX_OWNER_SLACK_ID`. Existing Slack token/channel settings are reused.
No new secret is needed for this lifecycle.

Stage 3b and the scheduled Stage 4 workflow share a concurrency group, so their
vault changes do not overlap. A concurrent human push is rejected safely by Git;
reconcile before retrying. If GitHub writes succeeded but an archive push failed,
inspect existing issues before retrying to avoid duplicate execution work.

## Existing notes and rollout

Legacy notes need one explicit metadata migration; they are not silently treated
as ready based on headings. Preview using:

```sh
node scripts/intake/migrate-notes.mjs --vault "$VAULT" --owner "$OWNER"
```

On a clean isolated vault checkout, add `--apply`, review the changed Inbox
files, commit only those files, push, and verify. Existing statuses are left
unchanged. Legacy enrichment with a recognized result becomes ready or waiting;
raw notes become captured. Ambiguous legacy metadata is reported as
`review_required` and not changed. Migrate first, then enable the new source
workflow revision. Legacy processed markers remain compatible with the old
workflow during rollout.

Validation is offline: state/selection tests, clarification roundtrip tests,
local bare-Git synchronization tests, and mocked enrichment/triage calls. A live
capture is a real write and may trigger later paid enrichment on its schedule.
