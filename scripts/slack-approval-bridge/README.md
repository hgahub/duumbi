# Slack Approval Bridge

Azure Function that bridges Slack interactive button clicks to DUUMBI GitHub
Actions workflows via `repository_dispatch`.

Clicking **Approve**, **Request Changes**, or **Needs Clarification** in a
DUUMBI Slack notification triggers a deterministic GitHub Action instead of
launching an agent directly. Slack shortcuts can also dispatch Stage 1 intake.
Existing Stage 5, Stage 7, and Stage 9 buttons continue to route to
`stage-approval.yml`. Stage 10 resource authorization buttons that include
`action_type: "stage_10_authorization"` route to `stage-10-authorization.yml`;
legacy Stage 10 payloads are normalized into that same workflow. Stage 11 merge
or status decisions are handled directly by the human reviewer in GitHub.
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

| Payload | Dispatch event | Workflow |
|---|---|---|
| stage `5`, `7`, `9` | `stage-approval` | `stage-approval.yml` |
| `10` + `action_type: "stage_10_authorization"` | `stage-10-authorization` | `stage-10-authorization.yml` |
| `10` without `action_type` | `stage-10-authorization` | `stage-10-authorization.yml` |
| `action_type: "project_status"` | `project-status` | `project-status.yml` |
| Slack message/global shortcut | `slack-intake` | `slack-intake-dispatch.yml` |

Unknown stage values fall back to `stage-approval`, where unsupported stages fail
closed.

The bridge does not forward Slack `response_url` capability URLs or raw Slack
message text in `repository_dispatch` payloads. It posts immediate Slack
follow-up messages from the function process and passes channel/thread
identifiers to GitHub workflows for agent handoff.

## Infrastructure

Ownership is split, and the split matters:

| Layer | Owner | How it changes |
|---|---|---|
| Function App, plan, storage, app settings, DNS | [duumbi-infra](https://github.com/hgahub/duumbi-infra) (`stack-platform.ts`) | `pulumi up` |
| Function code (this directory) | this repository | `.github/workflows/deploy-slack-bridge.yml`, or `func azure functionapp publish` |

Pulumi never deploys the code. The infra creates:

- Azure Function App (Consumption plan, Node.js 22, West Europe)
- App Settings: `SLACK_SIGNING_SECRET`, `GITHUB_TOKEN`, `GITHUB_REPO`,
  `WEBSITE_RUN_FROM_PACKAGE`
- DNS CNAME: `slack-bridge.duumbi.dev` → Function App hostname (optional)

Secrets are managed via Doppler → Azure Key Vault (existing pipeline).

### `WEBSITE_RUN_FROM_PACKAGE` is load-bearing

The published code is uploaded as a zip to `/home/data/SitePackages` and is
only mounted while `WEBSITE_RUN_FROM_PACKAGE=1` is set. `stack-platform.ts`
declares the app settings as a complete list, so any `pulumi up` on the
Function App replaces them: if that setting is not declared there, the
deployed code silently disappears and every Slack button gets HTTP 404.
This happened between 2026-09-07 (token rotation) and 2026-09-09.

## Local Development

```sh
cd scripts/slack-approval-bridge
npm install
npm test
func start
```

Use ngrok or Slack's socket mode to test locally.

## Deployment

`.github/workflows/deploy-slack-bridge.yml` runs the tests and publishes this
directory to `func-duumbi-slack-bridge` on every push to `main` that touches
`scripts/slack-approval-bridge/**`, and on `workflow_dispatch`. It requires the
repository secret `AZURE_FUNCTIONAPP_PUBLISH_PROFILE`; without it the deploy job
warns and skips instead of failing. Refresh that secret with:

```sh
az functionapp deployment list-publishing-profiles \
  -g rg-duumbi-platform -n func-duumbi-slack-bridge --xml
```

and store the XML with `gh secret set AZURE_FUNCTIONAPP_PUBLISH_PROFILE`.

Manual deployment (same result, needs Azure Functions Core Tools):

```sh
cd scripts/slack-approval-bridge
npm ci --omit=dev
func azure functionapp publish func-duumbi-slack-bridge
```

### Health check

```sh
curl -s -o /dev/null -w '%{http_code}\n' -X POST \
  https://func-duumbi-slack-bridge.azurewebsites.net/api/slack-approval
```

`401` is healthy — the request is unsigned, so the function itself rejects it.
`404` means the host loaded no functions at all; see
`WEBSITE_RUN_FROM_PACKAGE` above. The deploy workflow runs this check after
every publish.

### Function-first Project Status buttons (DUUMBI-789)

Do **not** post live Slack `blocks` for Project Status until this Function
revision is deployed to `func-duumbi-slack-bridge` and
`action_type: "project_status"` dispatches `project-status` (not
`stage-approval`).

Rollout:

1. Merge the Function + `project-status.yml` code with repository variable
   `DUUMBI_PROJECT_STATUS_SLACK_BUTTONS` unset or `false` (default: buttons
   off; Ready-for-Build and correction/entry posts stay text-only).
2. Deploy the Function with the command above, or the duumbi-infra publish
   path that targets `func-duumbi-slack-bridge`.
3. Verify a signed `project_status` click (or a local `node --test` plus a
   Function smoke) routes to `project-status`.
4. Only then set `DUUMBI_PROJECT_STATUS_SLACK_BUTTONS=true` so new Slack
   cards include **Set Ready for Build** / **Undo Done**.

Rollback:

1. Set `DUUMBI_PROJECT_STATUS_SLACK_BUTTONS` to `false` (or unset it) so new
   posts are text-only.
2. Revert or disable Function routing if needed.

Do not document "merge workflows first, Function later" as an acceptable
state for **live buttons**. Workflows may merge first only while the
enablement variable stays off.

Manual Project Status fallback (no Slack thread): run
`.github/workflows/project-status.yml` with `decision=ready-for-build` and
the issue number. Reviewer identity is `github.actor` when the reviewer
input is omitted. That path posts channel-only to `SLACK_REVIEW_CHANNEL_ID`.

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
| `WEBSITE_RUN_FROM_PACKAGE` | Static: `1` | Mounts the published zip from `/home/data/SitePackages`; without it the host serves no functions |

## Fallback

If the function is unavailable, Slack notifications include workflow fallback
links where decisions can be triggered directly from the GitHub Actions UI.
Stage 5, Stage 7, and Stage 9 approvals use `stage-approval.yml`; Stage 10
resource authorization uses `stage-10-authorization.yml`; Project Status
uses `project-status.yml`. Stage 11 merge, request-changes, clarification,
and abandon decisions are made directly in GitHub by the human reviewer.
