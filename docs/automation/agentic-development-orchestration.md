# DUUMBI Agentic Development Orchestration

This document records the repository-side implementation of the redesigned
DUUMBI intake-to-delivery workflow. The canonical operating model remains the
DUUMBI Agentic Development Runbook in the vault; these files are the executable
or source-repo contracts that support it.

## Source Of Truth

- GitHub Issues, PRs, CI, review threads, and Project V2 status hold execution
  state.
- Obsidian stores raw intake and durable knowledge.
- Slack is a capture, notification, clarification, and approval surface.
- GitHub Actions do not call model APIs directly. Scheduled workflows create
  deterministic dispatch records and Slack handoffs for Codex Cloud, Codex App,
  Codex CLI, or reviewed local agent runs.

## Skills Added Or Updated

- `duumbi-inbox-enrichment` normalizes manually edited Inbox notes and detects
  duplicates before Stage 4 triage.
- `duumbi-delivery-autopilot` coordinates a single `Spec Needed` issue through
  Stage 6, Stage 7 AI gate, Stage 8, Stage 9 AI gate, and Stage 10 entry.
- `duumbi-merge-decision` processes explicit human Stage 11 merge decisions.
- `duumbi-obsidian-capture` and `duumbi-codex-intake` now search active Inbox,
  Processed Inbox, Atlas, and GitHub before creating duplicate notes.
- `duumbi-spec-review` and `duumbi-tech-spec-review` now support bounded AI
  gates while still failing closed on missing Copilot, checks, scope, or
  unresolved findings.

## Workflows Added

| Workflow | Trigger | Purpose |
|---|---|---|
| `slack-intake-dispatch.yml` | Slack shortcut repository dispatch, manual | Dispatches Stage 1 Slack intake without requiring the developer to name the skill. |
| `inbox-enrichment-dispatch.yml` | 06:00 UTC and 18:00 UTC, manual | Checks `duumbi-vault` for unnormalized Inbox notes and dispatches `duumbi-inbox-enrichment` only when candidate notes exist. |
| `triage-queue-refill.yml` | every 3 hours, manual | Reads Project V2 Todo count and dispatches bounded Stage 4 triage when Todo is below the configured minimum. |
| `clarification-routing.yml` | issue comment mention, manual | Routes `@Codex` or `@Copilot` clarification replies to the right stage-specific agent. |
| `spec-ai-gate.yml` | manual, repository dispatch | Records Stage 7/9 AI gate decisions and dispatches `stage-approval.yml` for clean approvals. |
| `stage10-authorization-request.yml` | label, hourly, manual, repository dispatch | Sends Stage 10 resource authorization Slack notifications and records resource decisions. |
| `stage11-review-request.yml` | label, hourly, manual | Sends implementation review handoff notifications and records the Stage 11 notification marker. |
| `stage11-merge-decision.yml` | manual, repository dispatch | Processes explicit human merge authorization, fails closed on missing evidence, and squash-merges only when Stage 11 evidence, CI, and Copilot review are clean. |

## Slack Bridge Routing

`scripts/slack-approval-bridge` now dispatches by stage:

- Stage 5, 7, and 9 buttons use `stage-approval`.
- Stage 10 resource buttons use `stage-10-authorization` when the payload has
  `action_type: "stage_10_authorization"`; legacy stage-only buttons continue
  to use `stage10-authorization`.
- Stage 11 buttons use `stage11-merge-decision`.
- Slack shortcuts use `slack-intake` with Slack channel/thread identifiers only.

Unknown stages fall back to `stage-approval`, where unsupported stages fail
closed.

## Required Configuration

- `SLACK_BOT_TOKEN`: Slack bot token for notification posts.
- `SLACK_REVIEW_CHANNEL_ID`: human review channel.
- `DUUMBI_AGENT_DISPATCH_CHANNEL_ID`: optional agent dispatch channel; falls
  back to `SLACK_REVIEW_CHANNEL_ID`.
- `GH_PROJECT_PAT`: PAT that can read and update GitHub Project V2.
- `DUUMBI_PROJECT_NUMBER`: repository variable for the Project V2 number used by
  `triage-queue-refill.yml`.
- `DUUMBI_PROJECT_OWNER`: optional repository variable; defaults to repository
  owner.

## Gate Policy

Stage 7 and Stage 9 AI gates may approve only when:

- the PR is spec-only
- Copilot review exists and has no unresolved blocking feedback
- relevant checks are passing or explicitly not applicable
- no product, architecture, security, migration, cost, scope, or verification
  question remains
- the proposed spec stays inside the accepted issue scope

Stage 11 merge remains human-authorized. The merge workflow requires explicit
human decision, Stage 11 review artifact, green checks, clean Copilot review,
and an open non-draft implementation PR. It uses squash merge by default and
emits the Stage 12 closure prompt after merge.

## Metrics And Privacy

All new workflows write metadata-only metrics artifacts. They must not store raw
Slack payloads, issue bodies, comments, prompts from users, model completions,
provider payloads, credentials, or broad logs.

Slack capability URLs, including `response_url`, stay inside the Slack bridge
function and are not forwarded through GitHub `repository_dispatch` payloads.
Slack shortcut dispatches pass source identifiers instead of raw Slack message
text; GitHub workflow summaries intentionally omit generated agent prompts that
could contain user-provided Slack content.
