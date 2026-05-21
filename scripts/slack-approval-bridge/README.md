# Slack Approval Bridge

Azure Function that bridges Slack interactive button clicks to the
`stage-approval.yml` GitHub Actions workflow via `repository_dispatch`.

Clicking **Approve**, **Request Changes**, or **Needs Clarification** in a
DUUMBI Slack notification triggers a deterministic GitHub Action instead of
an LLM-based Oz agent — reducing approval execution from ~3 hours / ~108
credits to ~30 seconds / 0 credits.

## Architecture

```
Slack button click
  → Slack sends interaction payload to Azure Function URL
    → Function verifies Slack signing secret
      → Function POSTs repository_dispatch to GitHub
        → stage-approval.yml runs deterministically
          → Posts decision comment, updates labels, updates Project V2
            → Notifies Slack with result
```

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
4. Save Changes

## App Settings

| Setting | Source | Purpose |
|---|---|---|
| `SLACK_SIGNING_SECRET` | Slack app → Basic Information → Signing Secret | Verify Slack requests |
| `GITHUB_TOKEN` | GitHub → Settings → PATs | Trigger `repository_dispatch` |
| `GITHUB_REPO` | Static: `hgahub/duumbi` | Target repository |

## Fallback

If the function is unavailable, the Slack notification includes a link to
the `stage-approval.yml` manual dispatch page where approvals can be
triggered directly from the GitHub Actions UI.
