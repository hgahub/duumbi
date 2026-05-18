# DUUMBI Human Acceptance Slack Gate

The Human Acceptance Slack Gate connects the DUUMBI Project v2 status
`Needs Human Acceptance` to a Slack review notification. Slack is only the human
interaction surface; the durable Stage 5 decision remains the GitHub issue
comment and Project/label updates performed by `duumbi-human-acceptance`.

## Flow

1. A GitHub Projects v2 item changes Status to `Needs Human Acceptance`.
2. The Project v2 webhook bridge receives the `projects_v2_item` webhook.
3. The bridge filters for Status exactly equal to `Needs Human Acceptance`.
4. The bridge sends a `repository_dispatch` event named
   `duumbi-human-acceptance-needed` to `hgahub/duumbi`.
5. `.github/workflows/human-acceptance-request.yml` fetches the issue and posts
   one Slack notification through `SLACK_WEBHOOK_URL`.
6. The reviewer replies in the Slack thread with:

```text
@Oz accepted: <short rationale>
```

7. Warp/Oz Slack integration runs in `duumbi-vault-knowledge-env` and executes:

```text
Run DUUMBI Stage 5 Human Acceptance with duumbi-human-acceptance.

Target issue: <GitHub issue URL>
Human decision: Accept
Reviewer source: Slack
Rationale: <short human rationale>

Goal: Record the structured Stage 5 decision, set the issue to Spec Needed, add accepted and needs-spec labels, and remove needs-human-review when available.

Do not create product specs, technical specs, PRs, source-code changes, or implementation branches.
```

## Required Secrets

### GitHub Actions

- `WARP_API_KEY`: existing Warp API key.
- `SLACK_WEBHOOK_URL`: Slack Incoming Webhook URL for the target review channel.

`SLACK_WEBHOOK_URL` is not the workspace URL `https://hgabor.slack.com`. It must
be the generated webhook URL that starts with:

```text
https://hooks.slack.com/services/
```

Create it in Slack by creating or selecting a Slack app in `hgabor.slack.com`,
enabling Incoming Webhooks, adding a new webhook to the target channel, and
storing the generated URL as a GitHub Actions repository secret.

### Webhook Bridge

- `GITHUB_WEBHOOK_SECRET`: secret configured on the GitHub webhook.
- `GITHUB_DISPATCH_TOKEN`: GitHub App installation token or fine-grained PAT
  that can call `POST /repos/hgahub/duumbi/dispatches`.

The dispatch token needs repository `contents: write` permission because GitHub
requires that permission for `repository_dispatch`.

## Deployment Notes

Deploy the bridge in `tools/project-v2-human-acceptance-bridge/server.mjs` on any
small HTTPS-capable service with Node.js 18 or newer. Configure GitHub to send
organization Project v2 item events to:

```text
https://<bridge-host>/webhook
```

The bridge is intentionally dependency-free and exposes:

- `GET /healthz` for health checks.
- `POST /webhook` for GitHub `projects_v2_item` webhooks.

## Manual Test

Run the workflow from GitHub Actions with:

- `issue_url`: a real `https://github.com/hgahub/duumbi/issues/<number>` URL.
- `issue_number`: the matching issue number.
- `status`: `Needs Human Acceptance`.

Expected result: exactly one Slack notification is posted. If `status` is any
other value, the workflow exits without posting.

Run the bridge parser self-test locally:

```sh
node tools/project-v2-human-acceptance-bridge/server.mjs --self-test
```
