# DUUMBI Human Acceptance Slack Gate

The Human Acceptance Slack Gate connects the DUUMBI Stage 4 triage label
`needs-human-review` to the configured Warp/Oz Slack integration. This version
intentionally uses only GitHub Actions, Slack, and Warp/Oz. It does not require
a separate server, webhook bridge, Slack Incoming Webhook, GitHub App receiver,
or Project v2 event listener.

The durable Stage 5 decision remains the GitHub issue comment and Project/label
updates performed by `duumbi-human-acceptance`.

## Flow

1. Stage 4 triage routes a GitHub issue to human acceptance.
2. Stage 4 adds the existing `needs-human-review` label to the issue.
3. `.github/workflows/human-acceptance-request.yml` runs on the GitHub
   `issues.labeled` event when the added label is `needs-human-review`.
4. The workflow also runs hourly as a scheduled sweep for open
   `needs-human-review` issues that were not already notified.
5. The read-only notification job starts `warpdotdev/oz-agent-action` with
   `WARP_API_KEY`.
6. Oz runs with the `duumbi-vault-knowledge-env` profile and uses the configured
   Warp/Oz Slack integration to notify the DUUMBI reviewer.
7. After Oz succeeds, a separate marker job writes an operational marker comment
   to the issue so future scheduled sweeps do not send duplicate notifications.
8. The reviewer replies in Slack with:

```text
@Oz accepted: <short rationale>
```

9. Warp/Oz Slack integration runs in `duumbi-vault-knowledge-env` and executes:

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
- A scheduled sweep processes at most 10 issues per run to keep the Oz prompt
  and Slack notifications bounded.
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

- `WARP_API_KEY`: Warp/Oz API key used by `warpdotdev/oz-agent-action`.

No `SLACK_WEBHOOK_URL` is required. Slack delivery is handled by the configured
Warp/Oz Slack integration, visible in Slack under Apps -> Warp.

The Oz notification job uses `issues: read`. A separate marker job uses
`issues: write` only after the Oz notification job succeeds, so Oz does not run
with an issue-write-capable `GITHUB_TOKEN`.

## Manual Test

Run the workflow from GitHub Actions with:

- `issue_url`: a real `https://github.com/hgahub/duumbi/issues/<number>` URL.
- `issue_number`: the matching issue number.
- `status`: `Needs Human Acceptance`.

Expected result: the workflow starts an Oz run, and Oz notifies the configured
DUUMBI reviewer through Slack, then writes the notification marker comment. If
`status` is any other value, the workflow exits without starting Oz.

## End-to-End Test

1. Add the `needs-human-review` label to a test issue.
2. Confirm that GitHub Actions runs `Human Acceptance Request`.
3. Confirm that the workflow starts an Oz run.
4. Confirm that the Warp app posts or delivers the Slack notification.
5. Reply in Slack with:

```text
@Oz accepted: <short rationale>
```

6. Confirm that Oz runs in `duumbi-vault-knowledge-env`.
7. Confirm that the issue receives the notification marker comment.
8. Confirm that the issue receives a Stage 5 decision comment, is moved to
   `Spec Needed`, gets `accepted` and `needs-spec`, and loses
   `needs-human-review` when that label is present.

## Scheduled Sweep Test

1. Create or select an open test issue with `needs-human-review`.
2. Ensure it does not contain the notification marker comment.
3. Wait for the next hourly scheduled run.
4. Confirm that Oz starts and a notification marker comment is added.
5. Confirm that a later scheduled run skips the same issue because the marker is
   present.
6. If more than 10 unnotified issues exist, confirm that only 10 are processed in
   one hourly run and the rest remain eligible for later sweeps.
