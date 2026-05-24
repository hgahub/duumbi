# DUUMBI Human Acceptance Slack Gate

The Human Acceptance Slack Gate connects the DUUMBI Stage 4 triage label
`needs-human-review` to a Slack review notification from GitHub Actions. The
current version uses GitHub Actions, Slack, Codex handoff prompts, and the
Slack approval bridge for deterministic button decisions.

The durable Stage 5 decision remains the GitHub issue comment and Project/label
updates performed by `duumbi-human-acceptance`.

## Flow

1. Stage 4 triage routes a GitHub issue to human acceptance.
2. Stage 4 adds the existing `needs-human-review` label to the issue.
3. `.github/workflows/human-acceptance-request.yml` runs on the GitHub
   `issues.labeled` event when the added label is `needs-human-review`.
4. The workflow also runs hourly as a scheduled sweep for open
   `needs-human-review` issues that were not already notified.
5. The read-only notification job posts one Slack message per issue with the
   Slack Web API `chat.postMessage`.
6. The Slack message includes interactive buttons when the bridge is configured.
   If the bridge is unavailable, the reviewer should record the decision from
   Codex App, Codex Cloud, Codex CLI, or a reviewed local agent run using the
   `duumbi-human-acceptance` skill.
7. After Slack posting succeeds, a separate marker job writes an operational
   marker comment to the issue so future scheduled sweeps do not send duplicate
   notifications.
8. Fallback only: the reviewer records the decision with the configured Codex
   handoff:

```text
Run DUUMBI Stage 5 Human Acceptance with duumbi-human-acceptance.

Target issue: <GitHub issue URL>
Human decision: Accept
Reviewer source: Slack
Rationale: <short human rationale>

Goal: Record the structured Stage 5 decision, set the issue to Spec Needed, add accepted and needs-spec labels, and remove needs-human-review when available.

Do not create product specs, technical specs, PRs, source-code changes, or implementation branches.
```

## Trigger Contract

GitHub Actions cannot directly trigger on GitHub Projects v2 Status changes
without an external webhook receiver. The no-server contract is therefore:

- Project v2 Status may still be the human workflow state of record.
- The GitHub Action trigger is the issue label `needs-human-review`.
- A scheduled sweep runs hourly and catches open `needs-human-review`
  issues that have not yet received the notification marker.
- A scheduled sweep processes at most 10 issues per run to keep generated
  handoff prompts and Slack notifications bounded.
- Stage 4 must keep adding `needs-human-review` whenever it routes an issue to
  `Needs Human Acceptance`.

This matches the current DUUMBI Stage 4 skill contract.

The notification marker is a GitHub issue comment containing:

```html
<!-- duumbi-human-acceptance-slack-notified -->
```

The marker is operational state only. It is not the Stage 5 decision record.

## Required Secrets

### GitHub Actions

- `SLACK_BOT_TOKEN`: Slack bot token with permission to post in the review
  channel.
- `SLACK_REVIEW_CHANNEL_ID`: Slack channel ID where review notifications should
  be posted, for example the ID of `#duumbi-ops`.

No `SLACK_WEBHOOK_URL` is required. The workflow uses the Slack Web API directly
from GitHub Actions. The Slack bot must be invited to the target review channel.

There is no secondary Slack-agent fallback path. If interactive buttons are
unavailable, the reviewer uses Codex App, Codex Cloud, Codex CLI, or a reviewed
local agent run with the `duumbi-human-acceptance` skill.

For interactive buttons, `scripts/slack-approval-bridge` routes Stage 5, Stage 7,
and Stage 9 to `stage-approval.yml`, Stage 10 to
`stage10-authorization-request.yml`, and Stage 11 to
`stage11-merge-decision.yml`.

The Slack notification job uses `issues: read`. A separate marker job uses
`issues: write` only after Slack posting succeeds.

## Manual Test

Run the workflow from GitHub Actions with:

- `issue_url`: a real `https://github.com/hgahub/duumbi/issues/<number>` URL.
- `issue_number`: the matching issue number.
- `status`: `Needs Human Acceptance`.

Expected result: the workflow posts a Slack notification to
`SLACK_REVIEW_CHANNEL_ID`, then writes the notification marker comment. If
`status` is any other value, the workflow exits without posting Slack.

## End-to-End Test

1. Add the `needs-human-review` label to a test issue.
2. Confirm that GitHub Actions runs `Human Acceptance Request`.
3. Confirm that the workflow posts a Slack notification.
4. If interactive buttons are unavailable, run the fallback Codex handoff:

```text
Run DUUMBI Stage 5 Human Acceptance with duumbi-human-acceptance.

Target issue: <GitHub issue URL>
Human decision: Accept
Reviewer source: Slack
Rationale: <short human rationale>

Goal: Record the structured Stage 5 decision, set the issue to Spec Needed, add accepted and needs-spec labels, and remove needs-human-review when available.

Do not create product specs, technical specs, PRs, source-code changes, or implementation branches.
```

6. Confirm that the Codex handoff records the acceptance decision.
7. Confirm that the issue receives the notification marker comment.
8. Confirm that the issue receives a Stage 5 decision comment, is moved to
   `Spec Needed`, gets `accepted` and `needs-spec`, and loses
   `needs-human-review` when that label is present.

If labels update but the Project status does not, verify the issue's Projects v2
item with:

```sh
gh issue view <number> --repo hgahub/duumbi --json projectItems
```

If `projectItems` contains `Duumbi project`, the item exists and the failure is a
Project v2 status-update permission/API issue, not missing project membership.

## Scheduled Sweep Test

1. Create or select an open test issue with `needs-human-review`.
2. Ensure it does not contain the notification marker comment.
3. Wait for the next hourly scheduled run.
4. Confirm that a Slack notification is posted and a notification marker comment
   is added.
5. Confirm that a later scheduled run skips the same issue because the marker is
   present.
6. If more than 10 unnotified issues exist, confirm that only 10 are processed in
   one hourly run and the rest remain eligible for later sweeps.
