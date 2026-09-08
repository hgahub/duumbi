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
- update Project V2 Status without Stage 7/9 spec-PR merge validation
- keep combined two-file PRODUCT+TECHNICAL spec PRs unblocked
- leave Stage 7/9 single-file merge rules unchanged
- never reopen a closed issue from Slack

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
- `.github/workflows/ready-for-build-handoff.yml` — text-only `chat.postMessage`
- `.github/workflows/stage-approval.yml` — Stage 5/7/9 matrix;
  `validateAndMergeSpecPr` requires exactly one PRODUCT.md or TECHNICAL.md file
- `.github/workflows/stage-10-authorization.yml` — sibling-workflow precedent
- `.github/workflows/human-acceptance-request.yml` — Block Kit button shape
- `scripts/github-actions/stage-approval-workflow.test.mjs`
- No current `issues.reopened` Slack handoff exists

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
| Target Project Status | `Ready for Build` for both decisions |
| Closed issues | fail closed; do not reopen |

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
  "reviewer": "Slack (name)"
}
```

Rules:

- Do not include `slack_response_url`.
- Do not require `pr_number` or `stage`.
- `decision` is `ready-for-build` or `undo-done` only.
- `fallbackWorkflowName("project-status")` returns `project-status.yml`.
- `buildDispatchSuccessText` for this event must say Project Status update is
  running, not Stage 7/9 approval and not implementation.

Keep existing Stage 5/7/9/10 tests green. Add tests for the new route and for
the invariant that a Stage 9 approve payload still maps to `stage-approval`.

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
to send `blocks` plus fallback `text`.

Keep current candidate selection, `isReady` predicate, and v1 marker
`<!-- duumbi-ready-for-build-slack-notified:v1 issue=N -->`. Buttons are an
additive payload change. Do not require Project Status to already be Ready
for Build before showing **Set Ready for Build**; the button exists because
Status may still be Done or Spec Needed after a combined spec merge.

Query Project Status with the existing `getProjectStatus` helper. If it
returns `Done` or `null`, include **Undo Done**.

### Reopen / accidental-Done correction message

No reopen Slack path exists today. Add it in the same
`ready-for-build-handoff.yml` workflow (preferred: one notification job) or a
thin sibling workflow if that keeps the YAML readable.

Trigger: `issues` types `[reopened]` in addition to current labeled / schedule
/ `workflow_dispatch` triggers.

Post the correction message when **all** of these are true:

1. The issue is open after a reopen event (or a targeted `workflow_dispatch`).
2. The issue is not a pull request.
3. At least one Ready-for-Build track signal is present:
   - label `tech-spec-approved`, or
   - existing Ready-for-Build v1 marker comment, or
   - Project Status `Done`
4. The distinct correction marker is not already present:
   `<!-- duumbi-reopen-project-status-slack-notified:v1 issue=N -->`

The prior Ready-for-Build marker must **not** suppress this message.

After a successful Slack post, write the correction marker comment. Include
the same two buttons and fallback context.

Scheduled sweeps should not spam correction messages. Limit reopen handling to
the `issues.reopened` event plus optional `workflow_dispatch` with
`issue_number`. Do not scan every open issue for Done on the hourly cron.

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
```

Permissions: `contents: read`, `issues: write`. Do **not** grant
`pull-requests: write`. Do **not** call `github.rest.pulls.merge`.

Job steps:

1. Read `issue_number`, `decision`, `reviewer` from `client_payload` or
   `inputs`.
2. Reject unknown `action_type` values if present and not `project_status`.
3. Reject unknown `decision` values.
4. `GET` the issue. If `state !== "open"`, `core.setFailed` with a closed-issue
   message, notify Slack, and return without GraphQL mutation.
5. If `GH_PROJECT_PAT` is missing, fail the status update, comment that Status
   was not changed, and Slack the fallback.
6. Reuse the GraphQL Status update pattern from `stage-approval.yml` /
   `stage-10-authorization.yml`: find `projectItems`, find field `Status`, find
   option `Ready for Build`, `updateProjectV2ItemFieldValue`.
7. Both decisions write option **Ready for Build**. `undo-done` is an alias
   for the same mutation; keep the decision string in the issue comment for
   audit.
8. If Status is already Ready for Build, skip mutation or rewrite the same
   option, then report idempotent success.
9. Create an issue comment, for example:

   ```text
   ## Project Status Update
   **Decision:** Set Ready for Build
   **Reviewer source:** Slack (name)
   **Previous status:** Done
   **Project:** Ready for Build
   **Spec PR merged:** no
   ```

10. Post Slack success via `chat.postMessage` to `SLACK_REVIEW_CHANNEL_ID`.
    Do not rely on `response_url` from GitHub. The Function already posted the
    in-progress line through `response_url`.
11. Write metadata-only `duumbi-workflow-metrics.json` with
    `correlation.project_status: "Ready for Build"`, no Slack bodies, no
    secrets.

If GraphQL finds no project item or no Status option, fail visibly in Slack
with the Project UI fallback. Do not create Project fields.

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

Add deploy/rollback:

1. Merge implementation to default branch (GitHub workflows go live).
2. Deploy the Azure Function (`func azure functionapp publish
   func-duumbi-slack-bridge` or the duumbi-infra path). Until this deploy,
   new buttons dispatch as unknown `action_type` and fall through to
   `stage-approval`, which will fail closed for missing stage/decision.
3. Rollback: revert the Function first so stray clicks fail closed through
   existing Stage Approval, then revert workflows so new Slack posts lose the
   buttons. Do not leave buttons live against an undeployed Function.

Update `docs/automation/agentic-development-orchestration.md` Slack Bridge
Routing with the same row.

## Invariants

- Execution issue #789 stays open after this spec PR merges or closes.
- `stage-approval.yml` Stage 7/9 approve still requires a single spec file.
- Combined two-file spec PRs never need to pass that merge gate for Project
  Status correction.
- Project Status-only workflow never merges PRs and never reopens issues.
- Bridge still omits `slack_response_url` from GitHub payloads.
- No new GitHub labels or Project fields.
- No `src/` application/runtime changes.
- Metrics remain metadata-only.

## BDD-To-Test Mapping

| Product BDD scenario | Evidence type | Required implementation evidence |
|---|---|---|
| Owner sets Ready for Build from a Ready-for-Build handoff | Static workflow test + optional live Slack smoke | Assert `ready-for-build-handoff.yml` posts `blocks` with `action_type: "project_status"` and decision `ready-for-build`. Optional live click on a throwaway open issue proves Project Status becomes Ready for Build. |
| Combined two-file spec PR does not block the status button | Workflow contract test | Assert `project-status.yml` has no `pulls.merge` and no single-file PRODUCT/TECHNICAL requirement. Assert `stage-approval.yml` still has the single-file merge gate. |
| Undo Done moves Status on an open issue only | Bridge unit test + workflow script assertions | Button value `undo-done` routes to `project-status`. Workflow requires `issue.state === "open"` before GraphQL update. |
| Closed issue is not reopened from Slack | Workflow contract + unit/simulation | Assert no `issues.update` / `state: "open"` in `project-status.yml`. Simulate closed issue and expect failure Slack text. |
| Reopen after accidental Done gets a correction message | Workflow YAML test | Assert `issues.types` includes `reopened`, distinct marker string, and that the v1 Ready-for-Build marker is not used to skip correction posts. |
| Dispatch or Project update failure keeps a Slack fallback | Bridge unit test + workflow text assertion | `fallbackWorkflowName` is `project-status.yml`. Failure Slack mentions Project UI and does not tell the user to Approve Stage 7/9. |
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

Optional live smoke (human-approved throwaway issue only):

1. Open test issue on the Project board, Status Done, label
   `tech-spec-approved` or trigger Ready-for-Build handoff
   `workflow_dispatch`.
2. Confirm Slack message has buttons and fallback link.
3. Click **Set Ready for Build**.
4. Pass: Project Status is Ready for Build; issue stays open; no PR merge;
   Slack success reply; GitHub comment recorded.
5. Close the test issue, click a leftover button if present: Slack failure,
   issue stays closed.
6. Reopen the test issue: correction Slack posts despite Ready-for-Build
   marker; **Undo Done** / **Set Ready for Build** works.

Fail: Status unchanged without Slack fallback, closed issue reopened, or
`stage-approval.yml` merge path invoked.

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
  fields/labels, or restore previous Status instead of Ready for Build.

Suggested cycle order:

1. Bridge routing + unit tests + README dispatch table
2. `project-status.yml` + workflow contract tests (no merge, closed-issue fail)
3. Ready-for-Build handoff Block Kit buttons + tests
4. Reopen correction message + marker + tests
5. Orchestration docs + deploy/rollback notes + remaining acceptance sweep

## Task Breakdown

1. Add `project_status` routing in `slackApproval.js` and tests.
2. Add `.github/workflows/project-status.yml` with `repository_dispatch` and
   `workflow_dispatch`.
3. Implement open-issue Project V2 update, issue comment, Slack result,
   metrics.
4. Add Block Kit to `ready-for-build-handoff.yml`.
5. Add `issues.reopened` correction path and distinct marker.
6. Extend `stage-approval-workflow.test.mjs` (or sibling test) so Stage 7/9
   merge gates remain and the new workflow stays merge-free.
7. Update README and orchestration docs, including Azure Function deploy
   after workflow merge.
8. Optional live smoke on a throwaway issue.

## Verification Plan

- `node --test scripts/slack-approval-bridge/src/functions/slackApproval.test.js`
- `node --test scripts/github-actions/stage-approval-workflow.test.mjs`
- New tests for handoff blocks / reopen marker if extracted from inline YAML
- YAML load of `project-status.yml` and `ready-for-build-handoff.yml`
- Static grep: `project-status.yml` has no `pulls.merge`; `stage-approval.yml`
  still has `must change only`
- Codex self-review of the implementation PR
- Optional live Slack/Project smoke with human approval
- Azure Function deploy recorded in the implementation PR (or an explicit
  follow-up if infra deploy is separate)

## Completion Criteria

- All ten product-spec numbered acceptance criteria pass
- BDD-to-test mapping evidence exists for each scenario
- Stage 7/9 single-file merge tests still pass
- Bridge tests cover `project_status` and regression of Stage 5/7/9/10
- Docs list the new dispatch event and rollback
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
- If Azure Function deploy is blocked, ship GitHub workflows only with docs
  stating buttons will fail closed until the Function is published; do not
  silently route `project_status` through `stage-approval`.

## Open Questions

None blocking for implementation. Non-blocking:

- Infra may deploy the Function from `hgahub/duumbi-infra` rather than a
  manual `func azure functionapp publish`. Record the actual deploy path in
  the implementation PR.
- Extracting shared Project V2 GraphQL helpers out of inline `github-script`
  is optional and must not become a repo-wide refactor in this issue.
