# Slack Approval Bridge

Azure Function that bridges Slack interactive button clicks to DUUMBI GitHub
Actions workflows via `repository_dispatch`.

Clicking **Approve**, **Request Changes**, or **Needs Clarification** in a
DUUMBI Slack notification triggers a deterministic GitHub Action instead of
launching an agent directly. Slack shortcuts can also dispatch Stage 1 intake.
Existing Stage 5, Stage 7, and Stage 9 buttons continue to route to
`stage-approval.yml`. Stage 10 resource authorization buttons that include
`action_type: "stage_10_authorization"` route to `stage-10-authorization.yml`.
For file-based Stage 7 and Stage 9 spec approvals, `stage-approval.yml`
revalidates the linked spec PR and squash-merges it before advancing the issue.

## Architecture

```text
Slack button click or shortcut
  → Slack sends interaction payload to Azure Function URL
      → Function verifies Slack signing secret
        → Function POSTs repository_dispatch to GitHub
        → the stage-specific GitHub Actions workflow runs deterministically
          → Posts decision comment, updates labels/status when available
            → Notifies Slack with result
```

## Dispatch Routing

The bridge chooses the repository dispatch event from the button payload:

| Payload stage | Dispatch event | Workflow |
|---|---|---|
| `5`, `7`, `9` | `stage-approval` | `stage-approval.yml` |
| `10` + `action_type: "stage_10_authorization"` | `stage-10-authorization` | `stage-10-authorization.yml` |
| `10` without `action_type` | `stage10-authorization` | `stage10-authorization-request.yml` |
| `11` | `stage11-merge-decision` | `stage11-merge-decision.yml` |
| Slack message/global shortcut | `slack-intake` | `slack-intake-dispatch.yml` |

Unknown stage values fall back to `stage-approval`, where unsupported stages fail
closed.

The bridge does not forward Slack `response_url` capability URLs or raw Slack
message text in `repository_dispatch` payloads. It posts immediate Slack
follow-up messages from the function process and passes channel/thread
identifiers to GitHub workflows for agent handoff.

## Infrastructure

The Azure Function App is provisioned via Pulumi in the
[duumbi-infra](https://github.com/hgahub/duumbi-infra) repository
(`stack-platform.ts`). The infra creates:

- Azure Function App (Consumption plan, Node.js 20, West Europe)
- App Settings: `SLACK_SIGNING_SECRET`, `GITHUB_TOKEN`, `GITHUB_REPO`
- DNS CNAME: `slack-bridge.duumbi.dev` → Function App hostname (optional)

Secrets are managed via Doppler → Azure Key Vault (existing pipeline).

## Local Development

```sh
cd scripts/slack-approval-bridge
npm install
npm test
func start
```

Use ngrok or Slack's socket mode to test locally.

## Deployment

Deployment is handled via `func azure functionapp publish`:

```sh
cd scripts/slack-approval-bridge
npm install --production
func azure functionapp publish func-duumbi-slack-bridge
```

Or via GitHub Actions CI (configure in `duumbi-infra`).

## Slack App Configuration

1. Go to [api.slack.com/apps](https://api.slack.com/apps) → DUUMBI app
2. **Interactivity & Shortcuts** → toggle **On**
3. Set **Request URL** to the Function URL (e.g. `https://func-duumbi-slack-bridge.azurewebsites.net/api/slack-approval`)
4. Optional: create a message shortcut for DUUMBI idea capture. It will route to
   `slack-intake-dispatch.yml`.
5. Save Changes

## App Settings

| Setting | Source | Purpose |
|---|---|---|
| `SLACK_SIGNING_SECRET` | Slack app → Basic Information → Signing Secret | Verify Slack requests |
| `GITHUB_TOKEN` | GitHub → Settings → PATs | Trigger `repository_dispatch` |
| `GITHUB_REPO` | Static: `hgahub/duumbi` | Target repository |

## Fallback

If the function is unavailable, Slack notifications include workflow fallback
links where decisions can be triggered directly from the GitHub Actions UI.
Stage 5, Stage 7, and Stage 9 approvals use `stage-approval.yml`; Stage 10
resource authorization uses `stage-10-authorization.yml` for action-typed
payloads and `stage10-authorization-request.yml` for legacy stage-only
payloads.
