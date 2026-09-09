# DUUMBI-789: Slack Buttons To Set Project Ready for Build / Undo Done - Technical Specification

Related to #789. This technical specification is a review artifact only. The
execution issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Implementation Objective

Implement `specs/DUUMBI-789/PRODUCT.md` so Owner can set GitHub Project V2
Status to **Ready for Build** from Slack Block Kit buttons on Ready-for-Build
and reopen / accidental-Done correction messages.

The implementation must:

- reuse `scripts/slack-approval-bridge` → `repository_dispatch`
- reject invalid/stale Slack signatures with no GitHub dispatch and no
  request-directed Slack callbacks
- carry Slack `channel_id` and parent `thread_ts` on `project_status` dispatch
  (never `response_url`) and reply in-thread when that metadata is present
- post a correction/entry card after combined-spec merge or reopen when Status
  is Done or Spec Needed, even without `tech-spec-approved`
- dedupe that card per occurrence, not once per issue lifetime
- update Status only on the configured DUUMBI Project
- update Project V2 Status without Stage 7/9 spec-PR merge validation
- keep combined two-file PRODUCT+TECHNICAL spec PRs unblocked
- leave Stage 7/9 single-file merge rules unchanged
- never reopen a closed issue from Slack
- not post live Slack buttons until the Function routes `project_status`

This spec does not implement the change, approve itself, or start Ralph cycles.

## Agent Audience

- Codex or cloud implementation agents running bounded Stage 10 Ralph cycles
- Workflow/CI agents editing GitHub Actions YAML and the Azure Function
- Stage 9 reviewers checking implementability and merge-gate isolation
- Testers running Node workflow/bridge tests and an optional live Slack smoke

## Source Context

- Product spec: `specs/DUUMBI-789/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/789
- Stage 5 Accept: https://github.com/hgahub/duumbi/issues/789#issuecomment-5583019589
- Trigger: https://github.com/hgahub/duumbi/issues/779 and https://github.com/hgahub/duumbi/pull/787
- Repo instructions: `AGENTS.md`
- Orchestration: `docs/automation/agentic-development-orchestration.md`
- Slack gate: `docs/automation/human-acceptance-slack-gate.md`

Verified source at Stage 6 inspection:

- `scripts/slack-approval-bridge/src/functions/slackApproval.js`
  - `actionTypeForAction` defaults to `stage_approval`
  - `eventTypeForAction` special-cases only `stage_10_authorization`
  - everything else uses `eventTypeForStage` → `stage-approval` except stage `10`
  - `buildClientPayload` forwards `action_type` only for Stage 10
  - `fallbackWorkflowName` returns `stage-10-authorization.yml` or
    `stage-approval.yml`
- `scripts/slack-approval-bridge/src/functions/slackApproval.test.js`
  - invalid/missing timestamps fail `verifySlackSignature`; handler returns 401
    before dispatch today, but PRODUCT must not require a Slack follow-up on
    that path
- `.github/workflows/ready-for-build-handoff.yml` — text-only `chat.postMessage`;
  `isReady` is `tech-spec-approved` or Status `Ready for Build`; Project scan
  uses `DUUMBI_PROJECT_NUMBER` / `DUUMBI_PROJECT_OWNER` /
  `DUUMBI_PROJECT_OWNER_TYPE`
- `.github/workflows/stage-approval.yml` — Stage 5/7/9 matrix;
  `validateAndMergeSpecPr` requires exactly one PRODUCT.md or TECHNICAL.md file;
  Project write iterates all `issue.projectItems`
- `.github/workflows/stage-10-authorization.yml` — sibling-workflow precedent;
  also iterates all `issue.projectItems`
- `.github/workflows/human-acceptance-request.yml` — Block Kit button shape
- `scripts/github-actions/triage-queue-refill.mjs` — fail-closed if
  `DUUMBI_PROJECT_NUMBER` is missing
- `scripts/github-actions/stage-approval-workflow.test.mjs`
- No current `issues.reopened` Slack handoff exists
- Slack shortcut intake already copies `channel_id` / `thread_ts` /
  `message_ts` into `client_payload` without `response_url`; `block_actions`
  `project_status` must do the same for in-thread GitHub replies

## Affected Areas

Expected implementation files (Stage 10 only; not this spec PR):

- `scripts/slack-approval-bridge/src/functions/slackApproval.js`
- `scripts/slack-approval-bridge/src/functions/slackApproval.test.js`
- `scripts/slack-approval-bridge/README.md`
- `.github/workflows/project-status.yml` (new sibling workflow)
- `.github/workflows/ready-for-build-handoff.yml`
- `scripts/github-actions/stage-approval-workflow.test.mjs` (assert merge gates
  unchanged; add Project Status contract tests in this file or a sibling test)
- `docs/automation/agentic-development-orchestration.md`
- `docs/automation/human-acceptance-slack-gate.md` (bridge routing table)

Do not modify:

- Stage 7/9 merge validation inside `validateAndMergeSpecPr`
- `spec-review-request.yml` / `technical-spec-review-request.yml` review-clean
  gates
- Stage 10 authorization semantics
- application/runtime/CLI source under `src/`

## Technical Approach

### Locked contract

| Field | Value |
|---|---|
| Slack `action_type` | `project_status` |
| `repository_dispatch` event | `project-status` |
| Workflow file | `.github/workflows/project-status.yml` |
| Fallback workflow name | `project-status.yml` |
| Decisions | `ready-for-build`, `undo-done` |
| Target Project | repository vars `DUUMBI_PROJECT_NUMBER`, `DUUMBI_PROJECT_OWNER`, `DUUMBI_PROJECT_OWNER_TYPE` (same as Ready-for-Build handoff / triage refill) |
| Target Project Status | `Ready for Build` for both decisions |
| Slack thread metadata | `channel_id` + parent `thread_ts` in `client_payload`; never `response_url` |
| Closed issues | fail closed; do not reopen |
| Signature failure | HTTP 401; no dispatch; no Slack callbacks |
| Button enablement | Function with `project_status` routing must be live, or workflows must omit `blocks` until an enablement var is true |

Use a **sibling workflow**, not an extra Stage in `stage-approval.yml`. Stage 10
already split away from `stage-approval.yml` so resource authorization would not
inherit spec-PR merge. Project Status-only updates have the same isolation
requirement: they must never call `validateAndMergeSpecPr` or `pulls.merge`.

Rejected alternatives:

- Reuse `stage-approval` event with `stage: "9"` / `decision: "approve"` —
  would enter merge validation and fail combined two-file PRs.
- Add a silent bypass inside `validateAndMergeSpecPr` — forbidden. Do not
  weaken Stage 7/9 real file-approval merge rules.
- Reopen closed issues from Slack — out of product scope.
- Restore previous Status — out of v1 scope; both buttons target Ready for
  Build.

### Bridge routing

Extend `eventTypeForAction`:

```javascript
if (actionType === "stage_10_authorization") return "stage-10-authorization";
if (actionType === "project_status") return "project-status";
return eventTypeForStage(actionData?.stage);
```

`buildClientPayload` for `project_status` must send:

```json
{
  "action_type": "project_status",
  "issue_number": 789,
  "decision": "ready-for-build",
  "rationale": "Ready for build by Slack (name)",
  "reviewer": "Slack (name)",
  "channel_id": "C123",
  "thread_ts": "1234567890.123456"
}
```

`channel_id` comes from the verified Slack payload (`payload.channel.id` or
`payload.channel_id`). Parent `thread_ts` is `payload.message.thread_ts` when
the card is already in a thread, otherwise `payload.message.ts` (the card
itself is the thread root). Do not send empty strings as substitutes for
missing IDs; omit the fields when absent.

Rules:

- Do not include `slack_response_url` or `response_url`.
- Do not require `pr_number` or `stage`.
- `decision` is `ready-for-build` or `undo-done` only.
- `fallbackWorkflowName("project-status")` returns `project-status.yml`.
- `buildDispatchSuccessText` for this event must say Project Status update is
  running, not Stage 7/9 approval and not implementation.
- Immediate Function follow-up after a **valid** signature may use Slack
  `response_url` inside the Function process only (the existing
  `dispatchAsync` pattern). That is not a GitHub payload field.

Keep existing Stage 5/7/9/10 tests green. Add tests for the new route, for
thread metadata, and for the invariant that a Stage 9 approve payload still
maps to `stage-approval`.

### Invalid or stale signature (fail-closed)

Keep `verifySlackSignature` as the first gate. On failure:

1. Return HTTP 401 with a generic body (`Invalid signature`).
2. Do not `JSON.parse` the Slack payload for side effects that call out.
3. Do not call `fetch` against GitHub `repository_dispatch`.
4. Do not call `fetch` against `response_url`, `chat.postMessage`, or any
   other Slack URL taken from the request.

Negative tests (required):

- missing signing secret / missing timestamp / non-numeric timestamp / stale
  timestamp (>300s) / wrong HMAC → `verifySlackSignature` is false
- handler (or a testable wrapper around dispatch) makes **zero** outbound
  Slack or GitHub HTTP calls when verification fails
- no `repository_dispatch` body is constructed

Do not add a same-thread fallback on this path. Slack's own ephemeral or
original-card error to the clicker is sufficient.

### Slack button value

Follow Stage 5 JSON-in-`value` shape from
`human-acceptance-request.yml`:

```javascript
const buttonValue = (decision) => JSON.stringify({
  action_type: "project_status",
  issue_number: issue.number,
  decision,
});
```

Suggested `action_id`s: `project_status_ready_for_build`,
`project_status_undo_done`. Keep `value` well under Slack's 2000-character
limit.

Button copy:

- Set Ready for Build — `style: "primary"`, confirm dialog explaining Project
  Status will become Ready for Build and no spec PR will merge
- Undo Done — shown when current Status is `Done` or Status query failed;
  confirm dialog explaining this moves an **open** issue off Done to Ready
  for Build and will not reopen a closed issue

Context block fallback (adapt owner/repo/issue):

```text
Buttons are handled by the Slack approval bridge. Fallback: run
<.../actions/workflows/project-status.yml|Project Status> workflow manually
(decision=ready-for-build, issue=N), or set Status in the GitHub Project UI.
```

Do not point this fallback at `stage-approval.yml` Approve.

### Ready-for-Build handoff

In `.github/workflows/ready-for-build-handoff.yml`, change `chat.postMessage`
to send `blocks` plus fallback `text` **only when Project Status buttons are
enabled** (see Rollout). Until then, keep text-only posts so live cards cannot
click an undeployed Function.

Keep current candidate selection, `isReady` predicate, and v1 marker
`<!-- duumbi-ready-for-build-slack-notified:v1 issue=N -->`. Combined-spec
issues that stay `Spec Needed` without `tech-spec-approved` will **not** match
`isReady`; they are covered by the correction/entry producer below, not by
widening `isReady` in a way that spams Stage 10 handoff prompts.

Query Status with the **DUUMBI target Project only** (do not reuse a
`projectItems` scan that returns the first Status field from any board). If
that Status is `Done` or the query fails after buttons are enabled, include
**Undo Done**.

### Project Status correction/entry producer

Ready-for-Build handoff does not cover combined-spec completion: after a
two-file `PRODUCT.md` + `TECHNICAL.md` merge, the issue may remain open at
**Spec Needed** without `tech-spec-approved`, so `isReady` never posts a
button. Reopen alone also misses that case when GitHub never closed the issue.

**Producer (required):** post the same Project Status button card as the
reopen correction when **all** of these are true:

1. The issue is open and is not a pull request.
2. DUUMBI Project Status is `Spec Needed` or `Done` (or Status cannot be read
   after a qualifying producer event).
3. A qualifying event fired:
   - `issues.reopened`, or
   - `pull_request` closed and merged whose changed files are exactly
     `specs/DUUMBI-<N>/PRODUCT.md` and `specs/DUUMBI-<N>/TECHNICAL.md` for
     that issue `<N>`, or
   - `workflow_dispatch` naming `issue_number` for an issue that already has
     those two spec files on the default branch and/or combined Stage 7+9
     accept comments.
4. Buttons are enabled (same rollout gate as Ready-for-Build handoff).
5. No correction marker already exists for **this occurrence**.

Put this in `ready-for-build-handoff.yml` (preferred: extra triggers + a
separate candidate path) or a thin sibling workflow. Do not call
`pulls.merge`. Do not require a single-file Stage 7/9 spec PR. Do not require
the Owner to open the GitHub Project UI.

Hourly Ready-for-Build cron must **not** scan every `Spec Needed` issue for
this card. Combined-spec entry is event-driven (merged two-file spec PR,
reopen, or targeted dispatch).

#### Per-occurrence dedupe

Do **not** use an issue-wide marker such as
`<!-- duumbi-reopen-project-status-slack-notified:v1 issue=N -->`.

Use a stable occurrence id that is identical across retries of the same
delivery and different for a later close/reopen:

- Reopen: `issue=<N>;kind=reopened;occurrence=<stable>` where `<stable>` is
  the GitHub webhook delivery id (`github.event.delivery` / `GITHUB_DELIVERY`)
  when present, otherwise `issue.updated_at` from the `issues.reopened`
  payload (ISO timestamp of that reopen).
- Combined-spec merge: `issue=<N>;kind=combined-spec;occurrence=<merge SHA>`.
- Manual dispatch: `issue=<N>;kind=dispatch;occurrence=<run_id>` is allowed
  because humans can re-run deliberately; document that retries of the same
  run_attempt should still no-op if the marker was written.

Marker example:

```html
<!-- duumbi-project-status-correction-slack-notified:v1 issue=779;kind=reopened;occurrence=111222333 -->
```

Rules:

- Duplicate delivery of the same reopen: marker already present → do not post
  again.
- A later close + reopen: new occurrence id → post again.
- Prior Ready-for-Build v1 marker must **not** suppress correction/entry.
- Write the occurrence marker only after Slack post succeeds.

Tests: same occurrence twice → one Slack post; two sequential reopens → two
posts.

### Project Status-only workflow

Create `.github/workflows/project-status.yml`.

Triggers:

```yaml
on:
  repository_dispatch:
    types: [project-status]
  workflow_dispatch:
    inputs:
      issue_number:
        required: true
        type: number
      decision:
        required: true
        type: choice
        options: ['ready-for-build', 'undo-done']
      rationale:
        required: false
        type: string
      reviewer:
        required: false
        type: string
```

Permissions: `contents: read`, `issues: write`. Do **not** grant
`pull-requests: write`. Do **not** call `github.rest.pulls.merge`.

Env (same Project identity as Ready-for-Build handoff):

- `GH_PROJECT_PAT`
- `SLACK_BOT_TOKEN`
- `SLACK_REVIEW_CHANNEL_ID`
- `DUUMBI_PROJECT_NUMBER`
- `DUUMBI_PROJECT_OWNER` (default `github.repository_owner`)
- `DUUMBI_PROJECT_OWNER_TYPE`

Job steps:

1. Read `issue_number`, `decision`, `reviewer`, `channel_id`, `thread_ts` from
   `client_payload` or `inputs`. Reviewer is `raw.reviewer` if non-empty,
   otherwise `context.actor` (covers manual dispatch with no reviewer input).
2. Reject unknown `action_type` values if present and not `project_status`.
3. Reject unknown `decision` values.
4. `GET` the issue. If `state !== "open"`, `core.setFailed` with a closed-issue
   message, notify Slack using the thread rules below, and return without
   GraphQL mutation.
5. If `GH_PROJECT_PAT` or `DUUMBI_PROJECT_NUMBER` is missing, fail the status
   update, comment that Status was not changed, and Slack the fallback. Do not
   scan arbitrary `issue.projectItems`.
6. Resolve **one** target Project: `DUUMBI_PROJECT_OWNER` +
   `DUUMBI_PROJECT_NUMBER` (infer owner type as Ready-for-Build handoff does).
   Load that `projectV2`, find the issue's item **on that Project only**, find
   Status option `Ready for Build`. If the Project, item, or option is missing,
   fail visibly. Do not update any other Project.
7. Both decisions write option **Ready for Build**. `undo-done` is an alias
   for the same mutation; keep the decision string in the issue comment for
   audit.
8. If Status on the DUUMBI Project is already Ready for Build, skip mutation
   or rewrite the same option, then report idempotent success.
9. Create an issue comment, for example:

   ```text
   ## Project Status Update
   **Decision:** Set Ready for Build
   **Reviewer source:** Slack (name)
   **Previous status:** Done
   **Project:** Ready for Build
   **Target project:** DUUMBI_PROJECT_NUMBER=<n>
   **Spec PR merged:** no
   ```

10. Slack result:
    - If `channel_id` and parent `thread_ts` are present, `chat.postMessage`
      to that channel with `thread_ts` set (in-thread reply). Do not use
      `response_url`.
    - If either is missing (manual `workflow_dispatch`), post channel-only to
      `SLACK_REVIEW_CHANNEL_ID` with no `thread_ts`. The GitHub comment must
      say the Slack reply was not in-thread.
    - Never post to a channel ID taken from an unverified Slack body.
11. Write metadata-only `duumbi-workflow-metrics.json` with
    `correlation.project_status: "Ready for Build"`, no Slack bodies, no
    secrets.

Do not create Project fields. Do not iterate every `projectItems` node and
write Status on each board.

### Stage 7/9 isolation

`scripts/github-actions/stage-approval-workflow.test.mjs` must keep asserting:

- `files[0].filename.match(/^specs\/DUUMBI-(\d+)\/PRODUCT\.md$/)`
- `files[0].filename.match(/^specs\/DUUMBI-(\d+)\/TECHNICAL\.md$/)`
- `must change only ${policy.expectedPath}`
- `pulls.merge`

Add assertions that `project-status.yml` exists, lists `repository_dispatch`
type `project-status`, does not contain `pulls.merge`, and does not mention
`validateAndMergeSpecPr`.

Do not edit the `validateAndMergeSpecPr` function body except if a comment is
needed to say Project Status-only traffic must not enter it. Prefer zero edits
to that function.

### Docs

Update the dispatch table in `scripts/slack-approval-bridge/README.md`:

| Payload | Dispatch event | Workflow |
|---|---|---|
| `action_type: "project_status"` | `project-status` | `project-status.yml` |

Keep Stage 5/7/9 and Stage 10 rows unchanged.

### Rollout (Function first)

Do **not** ship live Slack buttons against an undeployed Function. Unknown
`action_type` currently falls through to `stage-approval`, which is the wrong
workflow.

Required order:

1. Implement and test `project_status` routing in the Function.
2. Deploy the Azure Function (`func azure functionapp publish
   func-duumbi-slack-bridge` or the duumbi-infra path) so `project_status`
   dispatches `project-status`.
3. Only then enable Block Kit `blocks` on Ready-for-Build and correction/entry
   posts. Enablement is either (a) deploy the workflow change after the
   Function is verified live, or (b) a repository variable such as
   `DUUMBI_PROJECT_STATUS_SLACK_BUTTONS=true` that workflows check before
   attaching buttons. Default is buttons off.

Rollback:

1. Turn the enablement var off or revert the workflow files that attach
   `blocks`, so new Slack posts are text-only.
2. Then revert or disable the Function routing if needed.

Do not leave buttons live against an undeployed Function. Do not document
"merge workflows first, Function later" as an acceptable ship state.

Update `docs/automation/agentic-development-orchestration.md` Slack Bridge
Routing with the `project_status` row and the Function-first note.

## Invariants

- Execution issue #789 stays open after this spec PR merges or closes.
- `stage-approval.yml` Stage 7/9 approve still requires a single spec file.
- Combined two-file spec PRs never need to pass that merge gate for Project
  Status correction. Combined-spec merge or reopen at Done/Spec Needed still
  produces a button card.
- Project Status-only workflow never merges PRs and never reopens issues.
- Status mutation targets only `DUUMBI_PROJECT_NUMBER` / owner vars.
- Bridge still omits `slack_response_url` from GitHub payloads.
- Invalid Slack signatures produce no outbound Slack or GitHub calls.
- `project_status` dispatch includes `channel_id` and parent `thread_ts` when
  Slack provided them; in-thread replies use those fields.
- Correction/entry markers are per occurrence, not once per issue.
- Live Slack buttons are not posted until Function `project_status` routing is
  live or an enablement gate is on.
- No new GitHub labels or Project fields.
- No `src/` application/runtime changes.
- Metrics remain metadata-only.

## BDD-To-Test Mapping

| Product BDD scenario | Evidence type | Required implementation evidence |
|---|---|---|
| Owner sets Ready for Build from a Ready-for-Build handoff | Static workflow test + optional live Slack smoke | Assert `ready-for-build-handoff.yml` posts `blocks` with `action_type: "project_status"` only when the enablement gate is on. Dispatch payload includes `channel_id` and parent `thread_ts`, not `response_url`. Optional live click on a throwaway open issue proves DUUMBI Project Status becomes Ready for Build. |
| Combined-spec history produces a button without Stage 7/9 merge | Workflow fixture + contract test | Simulate merged two-file `PRODUCT.md`+`TECHNICAL.md` PR with issue open and DUUMBI Status `Spec Needed`; assert a correction/entry Slack payload with buttons is produced; `project-status.yml` has no `pulls.merge`. |
| Combined two-file spec PR does not block the status button | Workflow contract test | Assert `project-status.yml` has no `pulls.merge` and no single-file PRODUCT/TECHNICAL requirement. Assert `stage-approval.yml` still has the single-file merge gate. |
| Undo Done moves Status on an open issue only | Bridge unit test + workflow script assertions | Button value `undo-done` routes to `project-status`. Workflow requires `issue.state === "open"` before GraphQL update. |
| Closed issue is not reopened from Slack | Workflow contract + unit/simulation | Assert no `issues.update` / `state: "open"` in `project-status.yml`. Simulate closed issue and expect in-thread failure Slack when thread metadata is present. |
| Reopen after accidental Done gets a correction message | Workflow YAML/helper test | Assert `issues.types` includes `reopened`, occurrence marker includes reopen identity, and the v1 Ready-for-Build marker is not used to skip correction posts. |
| Duplicate reopen delivery posts once; a later reopen posts again | Helper unit test | Same occurrence id → skip second Slack post. New occurrence id after a second reopen → post again. |
| Invalid Slack signature is fail-closed | Bridge negative test | Stale/wrong/missing signature: HTTP 401; assert zero GitHub and Slack outbound fetches; no `repository_dispatch` body. |
| Manual dispatch without thread metadata | Workflow simulation | Missing `channel_id`/`thread_ts` → channel-only `SLACK_REVIEW_CHANNEL_ID` post, no `thread_ts`. Reviewer is `github.actor` when input omitted. Present thread metadata → `chat.postMessage` uses that channel + `thread_ts`. |
| Dispatch or Project update failure keeps a Slack fallback | Bridge unit test + workflow text assertion | `fallbackWorkflowName` is `project-status.yml`. Valid-signature failure Slack mentions Project UI and does not tell the user to Approve Stage 7/9. |
| Unrelated Project boards are not updated | GraphQL fixture test | Issue has two project items; mutation is called only for the `DUUMBI_PROJECT_NUMBER` item; the other item's Status is unchanged. Missing target Project/item/option fails without mutating others. |
| Existing Stage 7/9 buttons still merge only single-file spec PRs | Existing `stage-approval-workflow.test.mjs` plus one extra assertion | Keep current merge-gate tests. Add that a Stage 9 payload still maps to `stage-approval` in `slackApproval.test.js`. |
| Already Ready for Build is idempotent success | Workflow simulation or comment/Slack copy test | Status already Ready for Build does not fail the job. |

Commands:

```sh
node --test scripts/slack-approval-bridge/src/functions/slackApproval.test.js
node --test scripts/github-actions/stage-approval-workflow.test.mjs
# plus any new project-status / handoff test file added beside those tests
```

Use `ruby -e "require 'yaml'; YAML.load_file(...)"` or equivalent to parse
touched workflows when `actionlint` is unavailable.

## Live E2E Plan

This issue does not change LLM behavior. There is no DUUMBI provider/CLI live
path and no expected external LLM cost.

Canonical interface: Slack Block Kit in the review channel plus GitHub Project
V2 Status.

Optional live smoke (human-approved throwaway issue only), after Function
`project_status` routing is live and buttons are enabled:

1. Open test issue on the DUUMBI Project board, Status Done, label
   `tech-spec-approved` or trigger Ready-for-Build handoff
   `workflow_dispatch`.
2. Confirm Slack message has buttons and fallback link.
3. Click **Set Ready for Build**.
4. Pass: DUUMBI Project Status is Ready for Build; other boards unchanged;
   issue stays open; no PR merge; **same-thread** Slack success reply; GitHub
   comment recorded.
5. Close the test issue, click a leftover button if present: Slack in-thread
   failure, issue stays closed.
6. Reopen the test issue: correction Slack posts despite Ready-for-Build
   marker; **Undo Done** / **Set Ready for Build** works.
7. Reopen a second time: a new correction card posts (new occurrence).
8. Combined-spec entry: open issue at Spec Needed with a merged two-file
   PRODUCT+TECHNICAL spec (or fixture equivalent); correction/entry card posts
   without `tech-spec-approved`; click updates DUUMBI Status only.

Fail: Status unchanged without Slack fallback (except invalid-signature 401),
closed issue reopened, `stage-approval.yml` merge path invoked, or buttons
posted before Function routing is live.

TUI/Studio: not applicable. No parity checks.

Expected external LLM calls: 0. Estimated cost: USD 0.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop only if requirements are met, a blocker appears, the expected
   external LLM cost of the next cycle exceeds USD 1, or scope changes;
   iteration count is not a stop condition

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 4 (bridge + tests, one workflow file, one
  docs file, or handoff YAML + tests).
- Expected command budget: `node --test` for bridge and workflow tests; YAML
  parse; optional `actionlint`. No `cargo` unless an unrelated repo check is
  already running.
- Human approval required only when the cycle will use an external LLM with
  expected cost above USD 1, exceeds approved scope, adds risky dependencies
  or irreversible operations, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning never triggers the gate.
- Expected external LLM cost for implementation: USD 0.
- No autonomous batch cap.
- When to stop and ask for human guidance: any proposal to edit
  `validateAndMergeSpecPr` merge rules, reopen closed issues, create Project
  fields/labels, restore previous Status instead of Ready for Build, or post
  live buttons before Function `project_status` routing is deployed.

Suggested cycle order:

1. Bridge routing + signature-fail-closed tests + thread metadata in payload
   (no live buttons yet)
2. Deploy Function; enablement gate remains off
3. `project-status.yml` + single-Project GraphQL + in-thread vs channel-only
   replies + `github.actor` audit
4. Correction/entry producer (combined-spec merge + per-occurrence reopen
   dedupe) with buttons still gated
5. Enable buttons (var or post-deploy workflow) + Ready-for-Build `blocks` +
   orchestration docs

## Task Breakdown

1. Add `project_status` routing in `slackApproval.js` and tests (including
   zero outbound calls on bad signature; `channel_id`/`thread_ts` present;
   `response_url` absent).
2. Deploy the Azure Function; keep Slack `blocks` disabled until verified.
3. Add `.github/workflows/project-status.yml` with `repository_dispatch` and
   `workflow_dispatch`.
4. Implement open-issue DUUMBI-Project-only V2 update, issue comment, in-thread
   or channel-only Slack result, `github.actor` fallback, metrics.
5. Add gated Block Kit to `ready-for-build-handoff.yml`.
6. Add combined-spec merge + `issues.reopened` correction/entry path with
   per-occurrence markers.
7. Extend `stage-approval-workflow.test.mjs` (or sibling test) so Stage 7/9
   merge gates remain, the new workflow stays merge-free, and multi-Project
   fixtures leave unrelated boards unchanged.
8. Update README and orchestration docs with Function-first rollout.
9. Optional live smoke on a throwaway issue after buttons are enabled.

## Verification Plan

- `node --test scripts/slack-approval-bridge/src/functions/slackApproval.test.js`
- `node --test scripts/github-actions/stage-approval-workflow.test.mjs`
- New tests for handoff blocks / correction occurrence markers / combined-spec
  producer if extracted from inline YAML
- YAML load of `project-status.yml` and `ready-for-build-handoff.yml`
- Static grep: `project-status.yml` has no `pulls.merge`; `stage-approval.yml`
  still has `must change only`; `project-status.yml` references
  `DUUMBI_PROJECT_NUMBER`
- Codex self-review of the implementation PR
- Optional live Slack/Project smoke with human approval **after** Function
  deploy and button enablement
- Azure Function deploy recorded before buttons are enabled

## Completion Criteria

- All ten product-spec numbered acceptance criteria pass
- BDD-to-test mapping evidence exists for each scenario, including signature
  fail-closed, in-thread vs manual-dispatch, per-occurrence dedupe,
  combined-spec entry, and single-Project targeting
- Stage 7/9 single-file merge tests still pass
- Bridge tests cover `project_status` and regression of Stage 5/7/9/10
- Docs list Function-first rollout / enablement gate
- Implementation PR uses non-closing `Related to #789` wording
- Execution issue #789 remains open

## Failure And Escalation

- If tests fail, fix the current cycle slice; do not expand scope.
- If Project V2 update fails in live smoke, keep Slack fallback and check
  `GH_PROJECT_PAT` / Status option names before inventing new fields.
- If a reviewer asks to change Stage 7/9 merge rules so combined spec PRs
  auto-advance, stop and treat that as a separate issue. This spec forbids
  that change.
- If expected external LLM cost would exceed USD 1, stop and ask. This work
  should not need any.
- If Azure Function deploy is blocked, do **not** enable Slack `blocks`. Keep
  text-only handoffs. Do not silently route `project_status` through
  `stage-approval`.

## Open Questions

None blocking for implementation. Non-blocking:

- Infra may deploy the Function from `hgahub/duumbi-infra` rather than a
  manual `func azure functionapp publish`. Record the actual deploy path in
  the implementation PR.
- Extracting shared Project V2 GraphQL helpers out of inline `github-script`
  is optional and must not become a repo-wide refactor in this issue.
