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
- GitHub Actions generally avoid direct model calls. The Stage 4
  `triage-queue-refill.yml` workflow is the explicit exception: it may call a
  bounded Z.ai/Zhipu-backed triage step when the Project V2 `Needs Human Acceptance`
  queue drops below the configured minimum. Other scheduled workflows create
  deterministic dispatch records and Slack handoffs for Codex Cloud, Codex App,
  Codex CLI, or reviewed local agent runs.

## Skills Added Or Updated

- `duumbi-inbox-enrichment` normalizes manually edited Inbox notes and detects
  duplicates before Stage 4 triage.
- `duumbi-delivery-autopilot` coordinates a single `Spec Needed` issue through
  Stage 6, Stage 7 AI gate, Stage 8, Stage 9 AI gate, and Stage 10 entry.
- `duumbi-obsidian-capture` and `duumbi-codex-intake` now search active Inbox,
  Processed Inbox, Atlas, and GitHub before creating duplicate notes.
- `duumbi-spec-review` and `duumbi-tech-spec-review` now support bounded AI
  gates while still failing closed on missing checks, scope, unresolved
  findings, or unmerged spec PR readiness. Spec gates have no required
  automated reviewer by default; Greptile is manual-only and reserved for the
  final implementation PR.
- `duumbi-closure` runs after a verified merge or equivalent completion
  evidence to close the loop across GitHub, source surfaces, Inbox notes, and
  durable knowledge sync decisions.

## Workflows Added

| Workflow | Trigger | Purpose |
|---|---|---|
| `slack-intake-dispatch.yml` | Slack shortcut repository dispatch, manual | Dispatches Stage 1 Slack intake without requiring the developer to name the skill. |
| `inbox-enrichment-dispatch.yml` | 06:00 UTC and 18:00 UTC, manual | Uses DeepSeek to enrich one unprocessed Inbox note in `duumbi-vault/main`, then posts Slack only when a vault commit is created. |
| `triage-queue-refill.yml` | every 4 hours, manual | Reads Project V2 `Needs Human Acceptance` count and uses a bounded Z.ai/Zhipu-backed Stage 4 triage refill when fewer than three issues are waiting. |
| `clarification-routing.yml` | issue comment created, manual | Filters for explicit `@Clarification` comments on `needs-human-review` issues, uses DeepSeek for synthesis, posts a GitHub comment, and sends Slack. |
| `spec-ai-gate.yml` | manual, repository dispatch | Records Stage 7/9 AI gate decisions and dispatches `stage-approval.yml` for clean approvals. |
| `ready-for-build-handoff.yml` | `tech-spec-approved` label, hourly, manual | Sends the Stage 10 Slack handoff when an issue becomes Ready for Build, with an idempotent issue marker so the notification can be retried independently from `stage-approval.yml`. |
| `ralph-cycle-approval-request.yml` | `needs-cycle-approval` label, twice daily, manual, repository dispatch | Sends Stage 10 bounded-cycle resource authorization Slack notifications; decisions are recorded through `stage-10-authorization.yml`. |
| `implementation-review-request.yml` | `needs-review` label, PR ready/labeled, twice daily, manual, repository dispatch | Sends implementation review handoff notifications with linked spec and PR evidence. |
| `stage12-closure-dispatch.yml` | merged PR, manual | Dispatches `duumbi-closure` after a developer merges the implementation PR. It does not merge, close issues, or claim `Done` itself. |

## Slack Bridge Routing

`scripts/slack-approval-bridge` now dispatches by stage:

- Stage 5, 7, and 9 buttons use `stage-approval`.
- Stage 10 resource buttons use `stage-10-authorization` when the payload has
  `action_type: "stage_10_authorization"`; legacy stage-only buttons are
  normalized into the same workflow.
- Stage 11 merge, request-changes, clarification, and abandon decisions are made
  directly by the human reviewer in GitHub.
- Slack shortcuts use `slack-intake` with Slack channel/thread identifiers only.

Unknown stages fall back to `stage-approval`, where unsupported stages fail
closed.

## Required Configuration

- `SLACK_BOT_TOKEN`: Slack bot token for notification posts.
- `SLACK_REVIEW_CHANNEL_ID`: human review channel.
- `DUUMBI_AGENT_DISPATCH_CHANNEL_ID`: optional agent dispatch channel; falls
  back to `SLACK_REVIEW_CHANNEL_ID`.
- `GH_PROJECT_PAT`: PAT that can read and update GitHub Project V2 and write
  enrichment commits to `duumbi-vault/main`.
- `DEEPSEEK_API_KEY`: DeepSeek API key used by
  `inbox-enrichment-dispatch.yml` for one-note Inbox preparation and by
  `clarification-routing.yml` for explicit `@Clarification` synthesis.
- `DEEPSEEK_MODEL`: optional repository variable for DeepSeek-backed
  automation; defaults to `deepseek-v4-pro`.
- `ZHIPUAI_API_KEY`: Z.ai/Zhipu API key used by
  `triage-queue-refill.yml` when the `Needs Human Acceptance` queue is below
  target.
- `ZHIPU_MODEL`: optional repository variable for the triage refill model;
  defaults to `glm-5.2`.
- `DUUMBI_PROJECT_NUMBER`: repository variable for the Project V2 number used by
  `triage-queue-refill.yml`.
- `DUUMBI_PROJECT_OWNER`: optional repository variable; defaults to repository
  owner.
- `DUUMBI_PROJECT_OWNER_TYPE`: optional repository variable; use `user` for a
  personal-account project and `organization` for an org-owned project. When
  omitted, the workflow infers the repository owner type from the GitHub event.

## Clarification Routing Policy

`clarification-routing.yml` is registered on all created issue comments and
filters in code. It ignores general `@Codex` issue comments. It
only processes comments whose visible text starts with `@Clarification`, and
only when the target issue still carries the `needs-human-review` Stage 5 label.
The workflow calls DeepSeek for a bounded JSON clarification synthesis, writes
the synthesis as an issue comment, and sends a Slack notification when Slack
secrets are configured. It does not update labels, Project V2 status, specs,
PRs, or source code.

## Inbox Enrichment Policy

`inbox-enrichment-dispatch.yml` scans `Duumbi/00 Inbox (ToProcess)/` in
`duumbi-vault`, selects at most one unprocessed Markdown note per run, and asks
DeepSeek for a bounded English enrichment using active vault docs plus selected
source-code context. It rewrites only that Inbox note, commits directly to
`duumbi-vault/main` with `GH_PROJECT_PAT`, and never creates GitHub issues,
specs, PRs, Atlas notes, or implementation changes.

The enriched note must include the original raw input, interpreted intent,
developer summary, Mermaid UML-style overview, classification, business value,
importance, complexity, scope, risks, open questions, and instructions for a
later AI agent that will create a GitHub issue. The workflow marks completion
with `duumbi/status/processed` and the `duumbi-inbox-enrichment:v1` marker, so
later scheduled runs ignore the note.

Slack is commit-gated. If no candidate note exists, or if the selected note
does not produce a vault diff, the workflow records metadata-only metrics and
does not post Slack. If a note is committed and Slack secrets are configured,
the workflow posts a short metadata-only completion notification.

## Stage 4 Refill LLM Policy

`triage-queue-refill.yml` runs at most six times per day. It first reads Project
V2 with `GH_PROJECT_PAT`; if at least three open issues are already in
`Needs Human Acceptance`, it exits without calling a model or posting Slack.

When refill is needed, the workflow checks out `duumbi-vault`, builds bounded
context from active Inbox notes, Ideas Discussions, Project V2 issue state, and
active Atlas/runbook docs, and asks Z.ai/Zhipu for one strict JSON decision:
`route_existing_issue`, `create_issue`, `needs_clarification`, or `no_action`.
Only `route_existing_issue` and `create_issue` perform GitHub writes, and at
most one issue is queued per run.

All GitHub writes use `GH_PROJECT_PAT` rather than `GITHUB_TOKEN`, so adding the
existing `needs-human-review` label can trigger the separate Human Acceptance
Slack gate. The refill workflow itself does not post Slack notifications; this
avoids duplicate messages. Clarification routing uses `GITHUB_TOKEN` because it
only comments on the already-routed issue.

Default model: `glm-5.2` through the Z.ai/Zhipu chat completions endpoint:
`https://api.z.ai/api/paas/v4/chat/completions`. The workflow records token
counts when the provider returns usage metadata, but cost estimation is
intentionally left null until stable pricing for this routed model is documented
in the repository.

## Gate Policy

Review service selection is governed by
`docs/automation/code-review-policy.md`:

- Codex self-review is mandatory before agents mark work ready, approve an AI
  gate, or recommend implementation merge readiness.
- Codex review via `@chatgpt-codex-connector` is the required automated
  reviewer on the final implementation PR.
- Quick low-cost reviewers (MiniMax, DeepSeek Pro, Grok Build, Cursor BugBot)
  are optional and advisory on non-final PRs; they are never a DUUMBI gate.
- Greptile is manual-only, quota-limited, and reserved for the final
  implementation PR when an explicitly requested deep review is justified. Do
  not include Greptile in `DUUMBI_REQUIRED_SPEC_REVIEWERS`.

Stage 7 and Stage 9 AI gates may approve only when:

- the PR is spec-only
- the PR is open, non-draft, and ready for approval merge
- actual non-dismissed review submissions exist for every configured required
  reviewer. By default `DUUMBI_REQUIRED_SPEC_REVIEWERS` is empty and no
  automated reviewer is required; repositories can opt in with a
  comma-separated low-cost reviewer list
- reviewer-request workflow success does not count as review evidence
- automated reviews and human reviews have no blocking `CHANGES_REQUESTED`
  decision
- every review thread is resolved, including threads that became outdated after
  a fix
- relevant checks are passing or explicitly not applicable
- no product, architecture, security, migration, cost, scope, or verification
  question remains
- the proposed spec stays inside the accepted issue scope

Stage 7 and Stage 9 human Slack approvals are merge finalizers for file-based
specs. The review request workflows send Slack approval cards only after the
linked PRODUCT.md or TECHNICAL.md PR is review-clean. Review-clean means the
PR has green checks, no blocking review decisions, no unresolved review
threads, and submissions from any configured required reviewers. Approval then
revalidates the exact PR, squash-merges the spec artifact with non-closing issue
references, records the stage decision, and advances the issue to the next
workflow state. If the PR is draft, dirty, not spec-only, missing required
reviewer submissions, or has unresolved review threads, the workflow fails
closed or defers notification.

The Stage 5 approval prompt is intentionally a combined spec handoff: it
instructs Codex to draft the product spec and the technical spec together,
without waiting for external review between them, then run the Stage 7 and
Stage 9 gates, merge the spec-only PR(s), move the issue to `Ready for Build`,
and send the Stage 10 implementation prompt. The Stage 7 approval prompt
remains a Stage 8-to-Ready handoff for issues that took the human review path.

`ready-for-build-handoff.yml` is the fallback and retry path for the Stage 10
Slack handoff. It posts when `tech-spec-approved` is added and also scans for
open issues that are already labeled `tech-spec-approved` or whose Project V2
Status is `Ready for Build`. It records
`<!-- duumbi-ready-for-build-slack-notified:v1 issue=N -->` on the issue after a
successful Slack post, so reruns and scheduled scans do not duplicate the same
handoff. This keeps Slack delivery independent from `stage-approval.yml` merge
or validation failures while preserving GitHub Issues and Project V2 as the
source of truth.

Stage 11 merge remains human-authorized. The merge workflow requires explicit
human decision, Stage 11 review artifact, green checks, a clean or handled
Codex (`@chatgpt-codex-connector`) review, and an open non-draft implementation
PR. It uses squash merge by default and emits the Stage 12 closure prompt
after merge.

## Metrics And Privacy

All new workflows write metadata-only metrics artifacts. They must not store raw
Slack payloads, issue bodies, comments, prompts from users, model completions,
provider payloads, credentials, or broad logs. The Stage 4 refill workflow may
send bounded triage context to Z.ai/Zhipu, but its metrics artifact records only
metadata, counts, provider name/model, token usage, estimated cost, and
warnings.

Slack capability URLs, including `response_url`, stay inside the Slack bridge
function and are not forwarded through GitHub `repository_dispatch` payloads.
Slack shortcut dispatches pass source identifiers instead of raw Slack message
text; GitHub workflow summaries intentionally omit generated agent prompts that
could contain user-provided Slack content.
