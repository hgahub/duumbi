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
  bounded DeepSeek API triage step when the Project V2 `Needs Human Acceptance`
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
  gates while still failing closed on missing required automated review
  submissions, checks, scope, unresolved findings, or unmerged spec PR
  readiness. Copilot is the default required reviewer; Greptile is manual-only.
- `duumbi-closure` runs after a verified merge or equivalent completion
  evidence to close the loop across GitHub, source surfaces, Inbox notes, and
  durable knowledge sync decisions.

## Workflows Added

| Workflow | Trigger | Purpose |
|---|---|---|
| `slack-intake-dispatch.yml` | Slack shortcut repository dispatch, manual | Dispatches Stage 1 Slack intake without requiring the developer to name the skill. |
| `inbox-enrichment-dispatch.yml` | 06:00 UTC and 18:00 UTC, manual | Uses DeepSeek to enrich one unprocessed Inbox note in `duumbi-vault/main`, then posts Slack only when a vault commit is created. |
| `triage-queue-refill.yml` | every 4 hours, manual | Reads Project V2 `Needs Human Acceptance` count and uses a bounded DeepSeek Stage 4 triage refill when fewer than three issues are waiting. |
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
  `inbox-enrichment-dispatch.yml` for one-note Inbox preparation, by
  `triage-queue-refill.yml` when the `Needs Human Acceptance` queue is below
  target, and by `clarification-routing.yml` for explicit `@Clarification`
  synthesis.
- `DEEPSEEK_MODEL`: optional repository variable for DeepSeek-backed
  automation; defaults to `deepseek-v4-pro`.
- `DUUMBI_PROJECT_NUMBER`: repository variable for the Project V2 number used by
  `triage-queue-refill.yml`.
- `DUUMBI_PROJECT_OWNER`: optional repository variable; defaults to repository
  owner.
- `DUUMBI_PROJECT_OWNER_TYPE`: optional repository variable; use `user` for a
  personal-account project and `organization` for an org-owned project. When
  omitted, the workflow infers the repository owner type from the GitHub event.

## Clarification Routing Policy

`clarification-routing.yml` is registered on all created issue comments and
filters in code. It ignores general `@Codex` and `@Copilot` issue comments. It
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
active Atlas/runbook docs, and asks DeepSeek for one strict JSON decision:
`route_existing_issue`, `create_issue`, `needs_clarification`, or `no_action`.
Only `route_existing_issue` and `create_issue` perform GitHub writes, and at
most one issue is queued per run.

All GitHub writes use `GH_PROJECT_PAT` rather than `GITHUB_TOKEN`, so adding the
existing `needs-human-review` label can trigger the separate Human Acceptance
Slack gate. The refill workflow itself does not post Slack notifications; this
avoids duplicate messages. Clarification routing uses `GITHUB_TOKEN` because it
only comments on the already-routed issue.

Default model: `deepseek-v4-pro`. DeepSeek's public docs list V4 Pro and Flash
with 1M context, JSON output, tool calls, and low per-token prices. As of
2026-05-25, approximate monthly costs for six runs per day are:

| Scenario | DeepSeek Flash | DeepSeek Pro |
|---|---:|---:|
| 20k input + 2k output per run | USD 0.61/month | USD 1.88/month |
| 80k input + 6k output per run | USD 2.32/month | USD 7.20/month |

Reference pricing pages:

- DeepSeek models and pricing: https://api-docs.deepseek.com/quick_start/pricing
- DeepSeek JSON output: https://api-docs.deepseek.com/guides/json_mode
- OpenAI pricing: https://developers.openai.com/api/docs/pricing
- Anthropic pricing: https://platform.claude.com/docs/en/about-claude/pricing

## Gate Policy

Review service selection is governed by
`docs/automation/code-review-policy.md`:

- Codex self-review is mandatory before agents mark work ready, approve an AI
  gate, or recommend implementation merge readiness.
- Copilot is the default automated PR reviewer and the default required
  evidence source for file-based Stage 7 and Stage 9 gates.
- CodeRabbit is advisory when present; it is not a DUUMBI gate unless branch
  protection explicitly requires it.
- Greptile is manual-only, quota-limited, and reserved for stable high-risk
  implementation PRs or explicitly requested deep review. Do not include
  Greptile in `DUUMBI_REQUIRED_SPEC_REVIEWERS`.

Stage 7 and Stage 9 AI gates may approve only when:

- the PR is spec-only
- the PR is open, non-draft, and ready for approval merge
- actual non-dismissed required automated review submissions exist. By
  default this means `copilot-pull-request-reviewer`; repositories can override
  the comma-separated low-cost reviewer list with
  `DUUMBI_REQUIRED_SPEC_REVIEWERS`
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
PR has actual non-dismissed required automated reviewer submissions, green
checks, no blocking review decisions, and no unresolved review threads. Approval then
revalidates the exact PR, squash-merges the spec artifact with non-closing issue
references, records the stage decision, and advances the issue to the next
workflow state. If the PR is draft, dirty, not spec-only, missing required
reviewer submissions, or has unresolved review threads, the workflow fails
closed or defers notification.

The Stage 7 approval prompt is intentionally a Stage 8-to-Ready handoff. It
instructs Codex to draft the technical spec, wait for required automated
reviewers, fix and resolve review feedback, route to `Technical Spec Review`,
then run Stage 9 through the configured AI or human gate. Only a satisfied
Stage 9 gate may merge the technical spec PR, move the issue to `Ready for
Build`, and send the Stage 10 prompt.

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
human decision, Stage 11 review artifact, green checks, clean or handled
Copilot review,
and an open non-draft implementation PR. It uses squash merge by default and
emits the Stage 12 closure prompt after merge.

## Metrics And Privacy

All new workflows write metadata-only metrics artifacts. They must not store raw
Slack payloads, issue bodies, comments, prompts from users, model completions,
provider payloads, credentials, or broad logs. The Stage 4 refill workflow may
send bounded triage context to DeepSeek, but its metrics artifact records only
metadata, counts, provider name/model, token usage, estimated cost, and
warnings.

Slack capability URLs, including `response_url`, stay inside the Slack bridge
function and are not forwarded through GitHub `repository_dispatch` payloads.
Slack shortcut dispatches pass source identifiers instead of raw Slack message
text; GitHub workflow summaries intentionally omit generated agent prompts that
could contain user-provided Slack content.
