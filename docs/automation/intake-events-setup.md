# Deploy the Stage 3b event listener

The source repository owns the receiver, hourly fallback, model credentials, and
batch worker. The vault owns only the push listener. No new Azure service is used.

## Deployment files

Copy these reviewed source files to the vault repository (already prepared in the
companion vault PR):

| Source path | Vault path |
|---|---|
| `docs/automation/templates/vault-intake-events.yml` | `.github/workflows/intake-events.yml` |
| `scripts/intake/vault-listener.mjs` | `scripts/intake/vault-listener.mjs` |
| `scripts/intake/contract.mjs` | `scripts/intake/contract.mjs` |

The two JavaScript files are versioned copies; update both repositories together
when the listener or shared status parser changes. The listener runs without npm
installation. Source CI covers the listener with local Git fixtures and mocked HTTP.

## User steps

1. Merge the source PR first, so `duumbi-inbox-captured` has a receiver on main.
2. In GitHub Settings → Developer settings → Personal access tokens → Fine-grained
   tokens, create a token for resource owner `hgahub`, restricted to **only
   `duumbi`**, with repository **Contents: Read and write**. Set an expiration
   appropriate for your account and renew it before expiry. This permission is
   required by GitHub's repository-dispatch endpoint; the listener uses it only
   to send the fixed event.
3. In **hgahub/duumbi-vault → Settings → Secrets and variables → Actions → New
   repository secret**, save it as `DUUMBI_INTAKE_DISPATCH_TOKEN`. Enter the token
   directly in GitHub's secure field, not in chat or a committed file.
4. Merge the vault companion PR. A later captured-note push now sends one wake-up.
5. Submit a real idea or resolve a waiting note. Verify the vault listener reports
   `dispatched: true`, then check `Stage 3b - Inbox Enrichment` in `duumbi` for a
   `repository_dispatch` run and the note's new status/commit.

The listener has a manual dispatch for recovery: it scans tracked Inbox notes and
sends a wake-up if any is captured. HTTP 204 confirms event acceptance, not finished
processing. A missing/expired token fails the listener visibly; the central hourly
sweep still works independently. No test idea, external model call, or Slack message
is needed to validate deployment statically. The live end-to-end event test remains
after authentication and both merges.

GitHub reference (checked 2026-09-12):
[Repository dispatch and required token permission](https://docs.github.com/en/rest/repos/repos#create-a-repository-dispatch-event).
