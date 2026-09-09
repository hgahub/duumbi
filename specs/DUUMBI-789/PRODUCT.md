# DUUMBI-789: Slack Buttons To Set Project Ready for Build / Undo Done

Related to #789. This is a specification-only artifact. The execution issue
must remain open for Stage 7 Spec Review, Stage 8 technical specification,
Stage 9 approval, Stage 10 implementation, Stage 11 review, and Stage 12
closure.

## Summary

Give the Owner Slack Block Kit buttons on Ready-for-Build and
reopen / accidental-Done correction handoff messages so Project V2 Status can
be set to **Ready for Build** (and **Undo Done** when Status is Done) without
opening the GitHub Project UI.

The action path reuses the existing Stage 5 pattern:

```text
Slack Block Kit button
  -> Azure Function scripts/slack-approval-bridge
  -> repository_dispatch
  -> Project Status-only GitHub Actions workflow
  -> Project V2 Status update + Slack result
```

Project Status-only updates must not enter Stage 7 or Stage 9 spec-PR merge
validation. Combined two-file `PRODUCT.md` + `TECHNICAL.md` spec PRs must not
block the buttons. Stage 7 and Stage 9 single-file file-approval merge rules
stay unchanged.

## Goal

From Slack, the Owner can move an **open** GitHub issue's Project V2 Status to
**Ready for Build** after a Ready-for-Build handoff, after an accidental Done
/ reopen, or after a combined two-file spec merge that leaves Status at
**Done** or **Spec Needed**, without using the GitHub Project board.

## Problem

Stage 5, Stage 7, and Stage 9 already post interactive Slack buttons. Those
clicks go through `scripts/slack-approval-bridge` to `stage-approval.yml`.
Ready-for-Build handoff does not.

Verified on 2026-09-08:

- `.github/workflows/ready-for-build-handoff.yml` posts a text-only Slack
  message (`chat.postMessage` with `text`, no `blocks`, no buttons).
- After that post it writes
  `<!-- duumbi-ready-for-build-slack-notified:v1 issue=N -->` and later runs
  skip the same issue.
- `stage-approval.yml` Stage 7/9 **approve** always runs
  `validateAndMergeSpecPr`, which requires a **single** `PRODUCT.md` or
  `TECHNICAL.md` file. Combined two-file spec PRs are ignored on
  `pull_request` closed and cannot be merge-gated through that path.
- Issue #779 showed the live gap: squash-merge of combined spec PR #787
  auto-closed the issue through GitHub Development linkage, Project Status
  moved to **Done**, Owner reopened the issue, and then had to set
  **Ready for Build** in the Project UI. The Ready-for-Build Slack handoff had
  already posted (marker present), so a later handoff would not offer a new
  control. If GitHub had not closed the issue, Status could also have stayed
  **Spec Needed** without `tech-spec-approved`, and `isReady` would still not
  post a button.

The Owner is the polling loop for a status correction that Stage 5-style
buttons already solve for acceptance.

## Outcome

When this work is done:

1. Ready-for-Build Slack handoff messages include Block Kit buttons so the
   Owner can set Project Status to **Ready for Build** from Slack.
2. A Project Status correction/entry Slack message includes the same buttons
   after reopen / accidental Done **or** after a combined two-file spec merge
   / combined Stage 7+9 accept that leaves Status at **Done** or **Spec Needed**.
3. Button clicks update GitHub Project V2 Status only. They do not merge spec
   PRs, do not require a single-file PRODUCT/TECHNICAL PR, and do not change
   Stage 7 or Stage 9 file-approval merge rules.
4. Combined two-file spec history produces an eligible Slack card with buttons
   without the Owner opening the GitHub Project UI and without a single-file
   Stage 7/9 merge.
5. **Undo Done** moves Project Status on **open** issues. Slack does not reopen
   closed issues.
6. Authenticated button-click results reply in the same Slack thread. Invalid
   or stale Slack signatures are rejected with no dispatch and no
   request-directed Slack callback.
7. Status reads and writes target one deterministic DUUMBI Project. Other
   boards on the same issue are left unchanged.
8. GitHub remains the source of truth. Slack is an action surface, not a second
   workflow state store.

## Scope

### In Scope

1. Add Slack Block Kit buttons to `.github/workflows/ready-for-build-handoff.yml`
   posts, only after the Azure Function that understands `project_status` is
   deployed (or behind an equivalent enablement gate).
2. Add a Project Status correction/entry Slack message with the same buttons
   for (a) reopen / accidental Done and (b) combined two-file spec completion
   that leaves Status at **Done** or **Spec Needed**.
3. Route those buttons through `scripts/slack-approval-bridge` with a new
   Project Status-only `action_type` and `repository_dispatch` event that
   carries Slack channel and parent thread identifiers, not `response_url`.
4. Execute the click in a Project Status-only GitHub Actions path that updates
   only the configured DUUMBI Project V2 Status and does not run spec-PR merge
   validation.
5. Record a durable GitHub issue comment for the Project Status decision.
6. Reply in the originating Slack thread on success and on failure when the
   click was authentically signed. Invalid/stale signatures get no callback.
7. Document the bridge dispatch table, fallback workflow, Function-first
   rollout, and rollback notes.
8. Add focused tests for signature fail-closed, thread vs manual-dispatch
   replies, per-occurrence correction dedupe, combined-spec entry, and
   single-Project targeting.

### Explicitly Out Of Scope

1. Changing Stage 7 or Stage 9 single-file PRODUCT/TECHNICAL merge gates,
   review-clean checks, or squash-merge commit wording.
2. Reopening a **closed** GitHub issue from Slack.
3. Fixing GitHub Development linkage / auto-close of execution issues when a
   spec PR merges. That remains a separate problem. This issue only recovers
   Project Status after the issue is open again.
4. Restoring a previous non-Done Project Status (for example Spec Needed).
   v1 **Undo Done** and **Set Ready for Build** both target **Ready for Build**.
5. Stage 10 resource authorization UX (`stage-10-authorization.yml`).
6. Stage 11 merge buttons or Stage 12 Done/close automation.
7. Creating new GitHub labels, Project fields, or Project views.
8. Restricting button clicks to a named Slack user allow-list. v1 uses the same
   review-channel trust model as Stage 5.
9. Application, compiler, runtime, CLI, TUI, or Studio product code.
10. Asking the Owner to choose or maintain a default LLM model.

## Constraints And Assumptions

Facts:

- Stage 5 acceptance is recorded on #789 (`accepted` + `needs-spec`) in
  https://github.com/hgahub/duumbi/issues/789#issuecomment-5583019589.
- `scripts/slack-approval-bridge` already routes by `action_type` /
  `stage`. Stage 5/7/9 use `stage-approval`. Stage 10 uses
  `stage_10_authorization` → `stage-10-authorization.yml`. Unknown stages fall
  back to `stage-approval`, which then fails closed for unsupported stages.
- Invalid or stale Slack signatures already return HTTP 401 from
  `verifySlackSignature` before payload parse or dispatch. That path must stay
  fail-closed: no `repository_dispatch` and no request-directed Slack calls.
- The bridge does not forward Slack `response_url` in `repository_dispatch`
  payloads. Immediate Slack follow-up after a **valid** signature stays in the
  Function process via Slack's `response_url`. Result replies from GitHub
  Actions cannot use `response_url` and therefore need channel + thread
  identifiers.
- `ready-for-build-handoff.yml` is text-only today. Its `isReady` predicate is
  `tech-spec-approved` **or** Project Status `Ready for Build`. After a combined
  two-file spec merge the issue may remain `Spec Needed` without
  `tech-spec-approved`, so that handoff would not post.
- `stage-approval.yml` `pull_request` closed handling ignores merged PRs that
  are not a single spec file. Combined two-file spec PRs therefore cannot ride
  the Stage 7/9 approve-and-merge path.
- Ready-for-Build handoff and triage refill already identify the DUUMBI
  Project with repository variables `DUUMBI_PROJECT_NUMBER`,
  `DUUMBI_PROJECT_OWNER`, and `DUUMBI_PROJECT_OWNER_TYPE`.
- Project V2 updates in `stage-approval.yml` / `stage-10-authorization.yml`
  currently iterate `issue.projectItems` without selecting that configured
  Project. This issue must not copy that ambiguity.
- No current workflow posts a Slack message on `issues.reopened`.

Assumptions:

- The review channel (`SLACK_REVIEW_CHANNEL_ID`) is the correct default surface
  for new cards and for manual-dispatch replies that have no thread metadata.
- Existing secrets (`SLACK_BOT_TOKEN`, `GH_PROJECT_PAT`, Function
  `GITHUB_TOKEN` / `SLACK_SIGNING_SECRET`) and existing Project variables are
  sufficient. No new secret names or Project fields are required.
- Project Status option names **Ready for Build**, **Done**, and **Spec Needed**
  already exist. This issue does not create them.
- Owner means the human in the DUUMBI review Slack channel who is allowed to
  click Stage 5 buttons today.

## Decisions

| Decision | Choice | Evidence |
|---|---|---|
| Accept this workflow gap for spec | Accept | Stage 5 comment on #789 |
| Reuse Slack bridge → `repository_dispatch` | Required | Issue body; Stage 5/10 pattern |
| Do not change Stage 7/9 merge gates | Required | Stage 5 remaining constraint; `stage-approval.yml` `validateAndMergeSpecPr` |
| Combined 2-file spec PRs must not block status buttons | Required | #779 / #787; `files.length !== 1` ignore path |
| Combined-spec entry to a button | Correction/entry producer after combined-spec merge or combined Stage 7+9 accept, or after reopen, when Status is Done or Spec Needed | Lead review: `isReady` would not post without `tech-spec-approved` / Ready for Build |
| `action_type` / event | `project_status` / `project-status` | Stage 10 sibling-workflow pattern; keeps merge gates untouched |
| Messages that get buttons | Ready-for-Build handoff **and** Project Status correction/entry | Stage 5 remaining question; combined-spec and reopen gaps |
| Correction dedupe | Per reopen or producer occurrence, not once-per-issue | Duplicate delivery posts once; a later reopen posts again |
| Same-thread result | Channel + parent thread identifiers in dispatch; never `response_url` | Slack `response_url` is not forwarded to GitHub |
| Invalid Slack signature | HTTP 401; no dispatch; no request-directed Slack callback | Existing `verifySlackSignature` fail-closed path |
| Target Project | Configured DUUMBI Project only (`DUUMBI_PROJECT_NUMBER` + owner vars) | Same vars as Ready-for-Build handoff / triage refill |
| Undo Done | Project Status move on **open** issues only; target **Ready for Build** | Lead-pilot preference; #779 after reopen |
| Closed-issue reopen from Slack | Out of scope | Lead-pilot preference; GitHub issue reopen stays in GitHub |
| Channel trust | Same as Stage 5 review-channel buttons | Existing `human-acceptance-request.yml` does not encode an Owner allow-list |
| Rollout | Do not post live buttons until the Function routes `project_status` | Unknown `action_type` would otherwise fall through to `stage-approval` |

## Behavior

### Messages that get buttons

1. **Ready-for-Build handoff** (`.github/workflows/ready-for-build-handoff.yml`)
   must post Block Kit `blocks` in addition to fallback `text` when its existing
   `isReady` predicate is true **and** Project Status buttons are enabled.
   Buttons appear on every new handoff that the workflow posts.
2. **Project Status correction/entry** must post a **new** Slack message with
   the same buttons when an **open** execution issue needs a Status move to
   Ready for Build and the Ready-for-Build handoff would not post. That includes:
   - **Reopen / accidental Done:** the issue is reopened and Status on the
     DUUMBI Project is **Done** or **Spec Needed**, or a prior Ready-for-Build
     marker / `tech-spec-approved` label is present.
   - **Combined-spec stuck:** a combined `PRODUCT.md` + `TECHNICAL.md` spec PR
     was merged, and/or combined Stage 7+9 accept comments exist, and Status on
     the DUUMBI Project is **Done** or **Spec Needed**, even if the issue was
     never closed and `tech-spec-approved` was never added.
3. The Ready-for-Build v1 marker must not suppress correction/entry messages.
   Correction/entry dedupe is **per occurrence** (each reopen delivery or each
   combined-spec completion), not once for the lifetime of the issue.
4. Other existing Slack cards (Stage 5/7/9/10/11/12) keep their current
   buttons and copy. This issue does not add Project Status buttons to Stage 7
   or Stage 9 approval cards.

### Buttons

Primary button, always present on Ready-for-Build handoff and correction/entry
messages:

- **Set Ready for Build**

Optional second button, present when Project Status is **Done** or cannot be
read:

- **Undo Done**

v1 backend target for both buttons is Project Status **Ready for Build**.
The second button exists so the accidental-Done case is obvious in Slack. It
is not a restore-previous-status control.

Each button uses a Slack confirm dialog before dispatch.

Each message includes a context fallback line with a GitHub Actions
`workflow_dispatch` link to the Project Status-only workflow (not the Stage 7/9
merge path) and a reminder that Project Status can be set in the GitHub
Project UI. That context line is part of the original card. It is the only
Slack-visible fallback when a later click has an invalid or stale signature.

### Click contract

On a **validly signed** click:

1. Slack sends `block_actions` to the Azure Function.
2. The Function verifies the Slack signature, acknowledges within 3 seconds,
   and dispatches GitHub without putting `response_url` in the GitHub payload.
3. The dispatch payload includes the validated Slack `channel_id` and the
   parent `thread_ts` (Slack `thread_ts` when already in a thread, otherwise
   the clicked message `ts`).
4. GitHub runs the Project Status-only workflow.
5. The workflow loads the issue. If the issue is **closed**, it fails closed:
   no Project update, no reopen. Slack receives a failure reply in the same
   thread telling the Owner to reopen the GitHub issue first, then use the
   correction/entry message or Project UI.
6. If the issue is **open**, the workflow sets Status to **Ready for Build**
   on the configured DUUMBI Project only.
7. The workflow writes an issue comment recording reviewer, decision, previous
   status when known, new status, target Project identity, and that no spec PR
   was merged. For `workflow_dispatch`, reviewer is the supplied input or
   `github.actor` when that input is absent.
8. Slack receives a success or failure reply in the same thread (same
   `channel_id` + parent `thread_ts`).

Idempotency: if Status on the DUUMBI Project is already **Ready for Build**,
treat the click as success and say so in Slack. Do not fail.

Manual `workflow_dispatch` has no Slack thread. In that case post a
channel-only result to `SLACK_REVIEW_CHANNEL_ID` without `thread_ts`, and
record on the GitHub comment that the reply was not in-thread.

### Invalid or stale Slack signature

If signature verification fails (missing, malformed, or timestamp older than
the existing five-minute window):

- Return HTTP 401.
- Do not parse the payload as trusted input.
- Do not call `repository_dispatch`.
- Do not POST to `response_url`, `chat.postMessage`, or any other
  request-directed Slack callback, including untrusted channel IDs from the
  body.
- Do not claim a same-thread follow-up. Slack may still show the clicker its
  own ephemeral or original-card error; that is the only UX this issue relies
  on for this case.

### Failure Slack fallback

This guarantee applies only after a **valid signature**.

If GitHub dispatch fails, the issue is closed, `GH_PROJECT_PAT` is missing, the
configured DUUMBI Project / item / **Ready for Build** option is missing, or
GraphQL update fails, Slack must receive a non-replacing **in-thread** follow-up
(when thread metadata is present) that includes:

- that Project Status was **not** changed
- a link to run the Project Status-only workflow manually
- a reminder to set Status in the GitHub Project UI

When thread metadata is absent (manual dispatch), use the documented
channel-only post instead of pretending to be in-thread.

The fallback must not tell the Owner to run Stage 7/9 **Approve** on a
combined two-file spec PR as the way to fix Status.

### Target Project

Status reads and writes use the single configured DUUMBI Project identified by
`DUUMBI_PROJECT_NUMBER`, `DUUMBI_PROJECT_OWNER`, and
`DUUMBI_PROJECT_OWNER_TYPE` (same variables as Ready-for-Build handoff and
triage refill). If the issue is also on other boards, those boards are not
updated. If the target Project, the issue's item on that Project, or the
**Ready for Build** option is missing, fail visibly with the Slack/GitHub
fallback. Do not create Project fields or guess another board.

### Invariants

- Stage 7/9 `validateAndMergeSpecPr` single-file rules remain exactly as they
  are.
- Project Status-only payloads must not call `pulls.merge`.
- Status mutation targets only the configured DUUMBI Project.
- Execution issues stay open. These buttons never close an issue and never
  use GitHub auto-close keywords.
- Invalid Slack signatures produce no GitHub dispatch and no request-directed
  Slack callback.
- `response_url` is never forwarded to GitHub.
- No new labels or Project fields.
- Live Slack buttons are not posted until the Function understands
  `project_status` (Function-first deploy or an explicit enablement gate).
- Metrics remain metadata-only: no raw Slack payloads, secrets, or issue
  bodies.

## BDD Scenarios

```gherkin
Feature: Slack Project Status buttons for Ready for Build

  Scenario: Owner sets Ready for Build from a Ready-for-Build handoff
    Given an open GitHub issue whose DUUMBI Project Status is Done or Spec Needed
    And Ready-for-Build handoff posts a Slack message with Block Kit buttons
    When the Owner confirms Set Ready for Build
    Then the Slack approval bridge dispatches action_type project_status
    And the dispatch payload includes channel_id and parent thread_ts
    And the dispatch payload does not include response_url
    And GitHub Project V2 Status on the DUUMBI Project becomes Ready for Build
    And other Projects on the issue are unchanged
    And the issue remains open
    And no spec PR is merged
    And Slack receives a success reply in the same thread

  Scenario: Combined-spec history produces a button without Stage 7/9 merge
    Given a combined PRODUCT.md and TECHNICAL.md spec PR was merged
    And Stage Approval merge automation did not add tech-spec-approved
    And the issue is open with DUUMBI Project Status Spec Needed or Done
    When the correction/entry producer runs
    Then Slack receives a message with Set Ready for Build
    And the Owner can click that button
    Then Project Status on the DUUMBI Project becomes Ready for Build
    And the project-status path does not call pulls.merge
    And Stage 7 and Stage 9 file-approval merge rules are unchanged

  Scenario: Combined two-file spec PR does not block the status button
    Given the issue reached combined-spec completion without a single-file
      PRODUCT or TECHNICAL merge
    When the Owner clicks Set Ready for Build on the posted card
    Then Project Status updates without requiring a single-file PRODUCT or
      TECHNICAL PR
    And Stage 7 and Stage 9 file-approval merge rules are unchanged

  Scenario: Undo Done moves Status on an open issue only
    Given an open GitHub issue whose DUUMBI Project Status is Done
    And a Ready-for-Build or correction/entry Slack message shows Undo Done
    When the Owner confirms Undo Done
    Then Project Status on the DUUMBI Project becomes Ready for Build
    And the GitHub issue is not closed or reopened by the workflow

  Scenario: Closed issue is not reopened from Slack
    Given a GitHub issue that is still closed
    When a validly signed Project Status button is clicked for that issue
    Then Project Status is not changed
    And the issue remains closed
    And Slack receives a same-thread failure reply telling the Owner to
      reopen the issue in GitHub first, then use Project UI or the
      correction/entry message

  Scenario: Reopen after accidental Done gets a correction message
    Given an execution issue was closed and DUUMBI Project Status moved to Done
    And a prior Ready-for-Build Slack marker already exists on the issue
    When the Owner reopens the GitHub issue
    Then Slack receives a new correction message with Set Ready for Build
    And the prior Ready-for-Build marker does not suppress that message

  Scenario: Duplicate reopen delivery posts once; a later reopen posts again
    Given a correction/entry message was already posted for this reopen
      occurrence
    When the same reopen delivery is retried
    Then Slack does not receive a duplicate correction message
    When the issue is closed and reopened again as a new occurrence
    Then Slack receives another correction message with the buttons

  Scenario: Invalid Slack signature is fail-closed
    Given a Project Status button click with a missing, wrong, or stale
      Slack signature
    When the Azure Function receives the request
    Then it returns HTTP 401
    And it does not dispatch repository_dispatch
    And it does not POST to response_url or any Slack API
    And no same-thread GitHub-driven follow-up is required

  Scenario: Manual dispatch without thread metadata
    Given a human runs project-status.yml workflow_dispatch without Slack
      thread identifiers
    When the workflow finishes
    Then it posts a channel-only result to the review channel
    And it does not claim an in-thread reply
    And reviewer identity is github.actor when no reviewer input is supplied

  Scenario: Dispatch or Project update failure keeps a Slack fallback
    Given the Owner clicked Set Ready for Build with a valid signature
    When GitHub dispatch or DUUMBI Project V2 update fails
    Then Slack receives a non-replacing in-thread warning
    And the warning links the Project Status-only workflow_dispatch fallback
    And the warning tells the Owner that Project Status can be set in GitHub
    And the warning does not instruct Stage 7/9 Approve-and-merge as the fix

  Scenario: Unrelated Project boards are not updated
    Given the issue is on the DUUMBI Project and on a second Project
    When Set Ready for Build succeeds
    Then only the DUUMBI Project Status is Ready for Build
    And the second Project is unchanged

  Scenario: Existing Stage 7 and Stage 9 buttons still merge only single-file spec PRs
    Given a Stage 9 Approve Slack button for a technical spec review
    When that button is clicked
    Then the bridge still dispatches stage-approval
    And stage-approval.yml still requires a single TECHNICAL.md file to merge
    And this issue's Project Status path is not used

  Scenario: Already Ready for Build is idempotent success
    Given an open issue whose DUUMBI Project Status is already Ready for Build
    When the Owner clicks Set Ready for Build
    Then Project Status remains Ready for Build
    And Slack reports that Status was already Ready for Build
```

## Tasks

1. Specify and implement bridge routing for `action_type: project_status`.
2. Deploy the Azure Function with that routing **before** posting live buttons,
   or gate buttons behind an enablement flag until the Function is live.
3. Add the Project Status-only GitHub Actions workflow and `workflow_dispatch`
   fallback, including thread metadata and `github.actor` audit identity.
4. Add Block Kit buttons to Ready-for-Build handoff Slack posts.
5. Add correction/entry Slack posts for reopen occurrences and combined-spec
   stuck Status (Done or Spec Needed).
6. Record GitHub issue comments and Slack success/failure replies.
7. Update bridge README and orchestration docs (Function-first rollout).
8. Add bridge unit tests and workflow contract tests for blockers 1–5.

Tasks 1–3 can proceed independently of 4–5 once the payload contract is
stable. Task 8 should land with each slice, not only at the end.

## Checks

Numbered acceptance criteria:

1. A Ready-for-Build Slack handoff for an open issue includes **Set Ready for
   Build** and, when Status is Done or unknown, **Undo Done**, and only when
   Project Status buttons are enabled.
2. Clicking **Set Ready for Build** on an open issue sets DUUMBI Project V2
   Status to **Ready for Build** without merging any PR.
3. Clicking **Undo Done** on an open issue whose Status is Done sets Status to
   **Ready for Build** and does not reopen or close the issue.
4. After a combined two-file spec merge or combined Stage 7+9 accept, if the
   issue is open and DUUMBI Status is **Done** or **Spec Needed**, a
   correction/entry Slack message with the buttons is posted without a
   single-file Stage 7/9 merge and without the Owner opening Project UI.
   Clicking the button then updates Status. The backend still must not call
   `pulls.merge`.
5. A closed issue click does not reopen the issue and replies in the same
   Slack thread with the GitHub reopen + Project fallback.
6. After a valid signature, dispatch or Project update failure replies in the
   same Slack thread with the Project Status-only workflow link and a Project
   UI reminder. Invalid/stale signatures return 401 with no dispatch and no
   request-directed Slack callback.
7. Stage 7/9 approve still requires a single-file PRODUCT/TECHNICAL PR merge;
   contract tests prove `stage-approval.yml` still contains that gate.
8. Reopening a stuck issue posts a correction Slack message even when the v1
   Ready-for-Build marker already exists. Dedupe is per reopen occurrence: a
   retried delivery of the same reopen posts once; a later close/reopen posts
   again.
9. Bridge tests prove `project_status` routes to `project-status` with
   `channel_id` and parent `thread_ts` and without `response_url`, and that
   Stage 5/7/9/10 routing is unchanged.
10. Status reads/writes use only the configured DUUMBI Project. Missing
    Project, item, or Ready for Build option fails visibly. Unrelated boards
    stay unchanged. Azure Function deploy-first (or enablement-gate) notes
    exist in the bridge README.

Evidence types: Node unit tests for the bridge, static workflow tests, optional
live Slack smoke on a throwaway open issue with human approval. No live LLM
path is required.

## Open Questions

None blocking. Non-blocking follow-ups:

- Should a later issue restore the previous Project Status instead of always
  targeting Ready for Build?
- Should GitHub Development auto-close of spec-only PRs be fixed so accidental
  Done happens less often? That is outside this issue.
- Should button clicks be limited to a named Slack user? v1 keeps Stage 5
  channel trust.
- Combined Stage 7+9 accept without a merged two-file PR is a secondary
  producer signal; the required producer is combined-spec merge and/or reopen
  with Status Done or Spec Needed.

## Sources

- Issue: https://github.com/hgahub/duumbi/issues/789
- Stage 5 Accept: https://github.com/hgahub/duumbi/issues/789#issuecomment-5583019589
- Trigger issue #779: https://github.com/hgahub/duumbi/issues/779
- Combined spec PR #787: https://github.com/hgahub/duumbi/pull/787
- `scripts/slack-approval-bridge/README.md`
- `scripts/slack-approval-bridge/src/functions/slackApproval.js`
- `scripts/slack-approval-bridge/src/functions/slackApproval.test.js`
- `.github/workflows/ready-for-build-handoff.yml`
- `.github/workflows/stage-approval.yml`
- `.github/workflows/stage-10-authorization.yml`
- `.github/workflows/human-acceptance-request.yml`
- `scripts/github-actions/stage-approval-workflow.test.mjs`
- `docs/automation/agentic-development-orchestration.md`
- `docs/automation/human-acceptance-slack-gate.md`
- `specs/DUUMBI-595/PRODUCT.md` (Stage 10 sibling-workflow precedent)
