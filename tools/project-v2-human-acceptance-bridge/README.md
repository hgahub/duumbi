# Project v2 Human Acceptance Bridge

This is a small dependency-free webhook receiver for GitHub Projects v2 item
events. GitHub Actions cannot directly trigger on Projects v2 item status
changes, so this bridge receives `projects_v2_item` webhooks, filters for Status
becoming `Needs Human Acceptance`, and dispatches the repository workflow
`duumbi-human-acceptance-needed`.

## Required Environment

- `GITHUB_WEBHOOK_SECRET`: shared secret configured on the GitHub webhook.
- `GITHUB_DISPATCH_TOKEN`: GitHub App installation token or fine-grained PAT that
  can create `repository_dispatch` events for `hgahub/duumbi`.
- `GITHUB_OWNER`: defaults to `hgahub`.
- `GITHUB_REPO`: defaults to `duumbi`.
- `PORT`: defaults to `3000`.

The dispatch token needs repository `contents: write` permission for the target
repository because GitHub gates `repository_dispatch` behind that permission.

## GitHub Webhook

Configure a GitHub App or organization webhook with:

- Event: `projects_v2_item`
- Payload URL: `https://<bridge-host>/webhook`
- Content type: `application/json`
- Secret: value stored in `GITHUB_WEBHOOK_SECRET`

The bridge ignores all events except `projects_v2_item` payloads whose changed
field is Status and whose new value is exactly `Needs Human Acceptance`.

## Run

Use Node.js 18 or newer.

```sh
GITHUB_WEBHOOK_SECRET=... \
GITHUB_DISPATCH_TOKEN=... \
GITHUB_OWNER=hgahub \
GITHUB_REPO=duumbi \
node tools/project-v2-human-acceptance-bridge/server.mjs
```

Health check:

```sh
curl http://localhost:3000/healthz
```

Local parser self-test:

```sh
node tools/project-v2-human-acceptance-bridge/server.mjs --self-test
```
