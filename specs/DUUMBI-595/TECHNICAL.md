# DUUMBI-595: Notify Stage 10 Authorization And Stage 11 Review Handoffs - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-595/PRODUCT.md` by adding
deterministic GitHub Actions and Slack bridge support for two implementation
handoffs:

```text
Stage 10 resource-gated Ralph Cycle request
  -> Slack notification
  -> explicit Stage 10 authorization decision
  -> durable GitHub issue comment and state update

Stage 10 implementation ready for review
  -> Slack notification
  -> ready-to-run Stage 11 Review Artifact prompt
  -> durable GitHub marker
```

This technical spec covers workflow, Slack bridge, skill guidance, parsing,
payload, idempotency, metrics, and verification design. It does not implement
source changes, approve a technical spec, run Ralph cycles, perform Stage 11
review, merge implementation PRs, or change resource thresholds.

Related to #595. This technical specification is a review artifact only; the
execution issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex implementation agents running bounded Stage 10 Ralph cycles.
- Oz or cloud workflow agents that trigger deterministic GitHub Actions from
  Slack or scheduled events.
- Workflow/CI agents editing GitHub Actions YAML and GitHub API scripts.
- Slack bridge agents editing the Azure Function under
  `scripts/slack-approval-bridge/`.
- Stage 9 technical spec reviewers checking implementability, boundaries, and
  resource policy.
- Stage 11 reviewer agents verifying that implementation evidence satisfies the
  product spec and this technical spec.

## Source Context

- Product spec: `specs/DUUMBI-595/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/616
- GitHub issue: https://github.com/hgahub/duumbi/issues/595
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/595#issuecomment-4526203205
- Stage 6 product spec draft comment:
  https://github.com/hgahub/duumbi/issues/595#issuecomment-4526179979
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/595#issuecomment-4526076913
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Existing automation guide: `docs/automation/human-acceptance-slack-gate.md`

Relevant code and workflow files verified for Stage 8:

- `.github/workflows/stage-approval.yml`
  - Handles only Stage 5, Stage 7, and Stage 9 decisions.
  - Supports `workflow_dispatch` and `repository_dispatch` event type
    `stage-approval`.
  - Posts decision comments, updates labels, attempts Project V2 status updates
    through `GH_PROJECT_PAT`, posts Slack summaries, writes workflow summaries,
    and emits metadata-only workflow metrics.
  - Its decision matrix is not suitable for Stage 10 resource authorization
    because Stage 10 decisions do not mean product/spec approval.
- `.github/workflows/human-acceptance-request.yml`
  - Uses `issues.labeled`, hourly schedule, and `workflow_dispatch`.
  - Posts Slack notifications, then writes a marker comment only after Slack
    delivery succeeds.
  - Uses a scheduled cap of 10 issues per run.
  - Emits metadata-only workflow metrics.
- `.github/workflows/spec-review-request.yml`
  - Uses `issues.labeled`, hourly schedule, and `workflow_dispatch`.
  - Finds Stage 6 product spec artifacts from issue comments.
  - Posts Slack review cards with buttons handled by the Slack approval bridge.
  - Emits metadata-only workflow metrics.
- `.github/workflows/technical-spec-review-request.yml`
  - Uses `issues.labeled`, hourly schedule, and `workflow_dispatch`.
  - Finds Stage 8 technical spec artifacts from issue comments.
  - Posts Slack review cards with buttons handled by the Slack approval bridge.
  - Emits metadata-only workflow metrics.
- `scripts/slack-approval-bridge/src/functions/slackApproval.js`
  - Verifies Slack signatures.
  - Handles Slack `block_actions`.
  - Parses the first action value as JSON.
  - Currently sends every interaction to GitHub `repository_dispatch` event type
    `stage-approval`.
  - Does not currently route by `action_type`.
- `scripts/slack-approval-bridge/package.json`
  - Has no meaningful tests. `npm test` currently prints `No tests configured`
    and exits successfully.
- `.agents/skills/duumbi-ralph-cycle/SKILL.md`
  - Defines the Ralph Cycle resource gate.
  - Defines the `## Ralph Cycle <N> Resource Approval Request` template.
  - Requires Project Status `Cycle Authorization` when approval is needed.
- `.agents/skills/duumbi-implementation/SKILL.md`
  - Coordinates Stage 10 state and routes to Ralph cycles, resource approval,
    blocker handling, PR evidence consolidation, or `In Review`.
- `.agents/skills/duumbi-review-artifact/SKILL.md`
  - Owns Stage 11 review evidence, not implementation or merge decisions.
- `docs/automation/human-acceptance-slack-gate.md`
  - Documents the existing label/manual/scheduled notification pattern and
    marker comment behavior.
- `specs/DUUMBI-610/TECHNICAL.md`
  - Defines the current metadata-only workflow metrics pattern for DUUMBI
    GitHub Actions.

Relevant Obsidian notes checked:

- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- `Duumbi/05 Archive/Processed Inbox/DUUMBI Pipeline Automation Spec.md`

Verified source facts:

- The active runbook says GitHub Issues, PRs, Project state, comments, labels,
  and CI are the execution source of truth.
- The active runbook defines `Cycle Authorization`, `In Progress`, `In Review`,
  `Blocked`, and `Deferred` as Stage 10 and Stage 11 statuses.
- The active runbook says future workflow metrics must stay metadata-only and
  must not store Slack payloads, issue bodies, comments, secrets, prompts, or
  completions.
- The product spec requires dispatch/manual operation even when proposed labels
  such as `needs-cycle-approval` or `needs-review` do not exist.
- The product spec forbids automatic approval, running implementation cycles
  from Slack button clicks, Stage 11 review execution from Slack buttons, label
  creation, Project field creation, and resource threshold changes.
- Stage 7 approved the product spec with no blocking findings and routed the
  issue to `Technical Spec Needed`.

Assumptions for implementation:

- New workflows may use `actions/checkout` and a local JavaScript helper because
  Stage 10 request parsing, idempotency, and Slack payload construction are more
  complex than the existing Stage 5/7/9 approval flows.
- Existing review workflows should not be refactored as part of this issue.
- Stage 10 authorization should be implemented as a dedicated workflow file,
  not as new Stage 10 rows inside `stage-approval.yml`.
- A Stage 10 `narrow-scope` Slack button can record a generic narrowing request
  and direct the reviewer to provide details in the issue or manual dispatch
  path. A richer Slack modal can be added later only if separately accepted or
  approved during implementation.
- A Stage 10 scheduled sweep is acceptable because the product BDD scenario
  references scheduled duplicate suppression and the repository already uses
  hourly sweeps for review-gate notifications. The schedule must gracefully skip
  label-based lookup when the optional trigger label is unavailable.

## Affected Areas

Expected implementation changes:

- New workflow:
  - `.github/workflows/ralph-cycle-approval-request.yml`
- New workflow:
  - `.github/workflows/stage-10-authorization.yml`
- New workflow:
  - `.github/workflows/implementation-review-request.yml`
- Slack bridge:
  - `scripts/slack-approval-bridge/src/functions/slackApproval.js`
  - `scripts/slack-approval-bridge/README.md`
  - `scripts/slack-approval-bridge/package.json` if meaningful tests are added
- Testable workflow helper:
  - `scripts/github-actions/duumbi-stage-handoff.mjs`
  - `scripts/github-actions/duumbi-stage-handoff.test.mjs`
- Stage skill guidance:
  - `.agents/skills/duumbi-ralph-cycle/SKILL.md`
  - `.agents/skills/duumbi-implementation/SKILL.md`
  - `.agents/skills/duumbi-review-artifact/SKILL.md`
- Optional docs:
  - `docs/automation/stage10-stage11-slack-handoffs.md`

Expected generated or runtime artifacts:

- GitHub issue comments:
  - Stage 10 notification marker comments.
  - Stage 10 authorization decision comments.
  - Stage 11 review handoff marker comments.
- GitHub Actions summaries and metadata-only metrics artifacts for each new
  workflow run.
- Slack messages posted through `SLACK_BOT_TOKEN` and
  `SLACK_REVIEW_CHANNEL_ID`.

Areas expected not to change:

- `specs/DUUMBI-595/PRODUCT.md`.
- Existing Stage 5, Stage 7, and Stage 9 decision semantics in
  `.github/workflows/stage-approval.yml`.
- Rust compiler, parser, graph, registry, runtime, MCP, TUI, Studio, and
  provider configuration code.
- Existing workflow labels or Project fields, unless they already exist and are
  being used by normal label/status mutation.
- Resource gate thresholds.
- Implementation source files for the feature itself during Stage 8.
- Generated workflow metrics artifacts or Slack messages checked into the repo.

## Technical Approach

### Shared helper design

Add a small, dependency-free Node.js helper under
`scripts/github-actions/duumbi-stage-handoff.mjs`. Keep it pure where possible
so it can be tested with Node's built-in `node:test` runner.

Recommended exports:

- `sanitizePromptField(value)`
- `sanitizeSlackField(value)`
- `extractGithubUrls(text)`
- `parseResourceApprovalRequest(comment)`
- `findLatestResourceApprovalRequest(comments, markerPrefix)`
- `hasStage10NotificationMarker(comments, requestCommentId, cycleNumber)`
- `buildStage10ApprovalSlackMessage(input)`
- `buildStage10DecisionComment(input)`
- `buildStage11ReviewSlackMessage(input)`
- `findStage6ProductSpecArtifact(comments)`
- `findStage8TechnicalSpecArtifact(comments)`
- `buildStage11Prompt(input)`
- `buildWorkflowMetrics(input)`

Implementation boundary:

- The helper must not read environment variables directly except in a thin CLI
  wrapper if the implementation chooses one.
- The helper must not perform network calls.
- Workflow scripts own GitHub and Slack API calls.
- Tests should use fixture comments and payload objects rather than real GitHub
  or Slack calls.

### Stage 10 resource notification workflow

Create `.github/workflows/ralph-cycle-approval-request.yml`.

Triggers:

```yaml
on:
  repository_dispatch:
    types: [ralph-cycle-approval-request]
  workflow_dispatch:
    inputs:
      issue_url:
        required: false
        type: string
      issue_number:
        required: true
        type: number
      request_comment_id:
        required: false
        type: string
  issues:
    types: [labeled]
  schedule:
    - cron: '0 * * * *'
```

Workflow behavior:

1. Ignore `issues.labeled` unless the label is `needs-cycle-approval`.
2. For scheduled runs, first check whether the repository has a
   `needs-cycle-approval` label. If it does not, report that scheduled
   label-based lookup is unavailable and exit successfully without posting
   Slack.
3. Cap scheduled processing at 10 issues per run.
4. For each issue, fetch comments and find the latest valid
   `## Ralph Cycle <N> Resource Approval Request`.
5. If `request_comment_id` is supplied, evaluate that comment first and fail if
   it is missing or malformed.
6. Validate required fields from the product spec:
   - `Issue`
   - `Product spec`
   - `Technical spec`
   - `Current State`
   - `Remaining Requirements`
   - `Proposed Cycle Goal`
   - `Planned Changes`
   - `Planned Checks`
   - `Resource Estimate`
   - `Approval Trigger`
   - `Stop Condition`
7. Extract cycle number from the heading with
   `/^## Ralph Cycle (\\d+) Resource Approval Request\\s*$/m`.
8. Treat missing required fields as a workflow failure for manual/dispatch runs
   and as a skipped issue with warning during scheduled sweeps.
9. Do not post Slack when parsing fails.
10. Build a Slack card with:
    - issue link
    - cycle number
    - request comment link
    - product spec link
    - technical spec link
    - proposed cycle goal
    - resource estimate
    - planned checks
    - approval trigger
    - stop condition
    - fallback link to `stage-10-authorization.yml`
11. Include three buttons:
    - `Approve Cycle`
    - `Narrow Scope`
    - `Reject / Defer`
12. Button values must include:

```json
{
  "action_type": "stage_10_authorization",
  "issue_number": 595,
  "cycle_number": 2,
  "request_comment_id": 123456789,
  "decision": "approve",
  "pr_number": 0
}
```

13. Use `decision: "narrow-scope"` and `decision: "reject-defer"` for the other
    buttons.
14. Post Slack through `chat.postMessage` using `SLACK_BOT_TOKEN` and
    `SLACK_REVIEW_CHANNEL_ID`.
15. Write a marker comment only after Slack delivery succeeds.
16. Marker format:

```html
<!-- duumbi-ralph-cycle-approval-slack-notified:v1 issue=595 cycle=2 request_comment_id=123456789 -->
```

17. Include workflow run URL and source event in the marker comment body.
18. Emit a metadata-only metrics artifact using the v1 pattern from issue #610.

The workflow may use a `notify`, `mark-notified`, and `metrics` job shape like
the existing review request workflows.

### Stage 10 authorization workflow

Create `.github/workflows/stage-10-authorization.yml`.

Triggers:

```yaml
on:
  workflow_dispatch:
    inputs:
      issue_number:
        required: true
        type: number
      cycle_number:
        required: true
        type: number
      request_comment_id:
        required: false
        type: string
      decision:
        required: true
        type: choice
        options: ['approve', 'narrow-scope', 'reject-defer']
      rationale:
        required: false
        type: string
      pr_number:
        required: false
        type: number
        default: 0
  repository_dispatch:
    types: [stage-10-authorization]
```

Decision matrix:

| Decision | Meaning | Labels | Project status | Next state |
| --- | --- | --- | --- | --- |
| `approve` | Authorize exactly the named bounded cycle. | Remove `needs-cycle-approval` if present. | `In Progress` when `GH_PROJECT_PAT` and option are available. | Stage 10 resource-permitted work may continue for that cycle. |
| `narrow-scope` | Original request is not authorized; agent must revise the cycle request. | Keep `needs-cycle-approval` if present. | `Cycle Authorization` when available. | Agent prepares a smaller request or asks for clarification. |
| `reject-defer` | Requested cycle is not authorized now. | Remove `needs-cycle-approval` if present. | `Deferred` when available, otherwise `Blocked` when available. | Stage 10 work stops until a new decision changes state. |

Workflow behavior:

1. Validate issue number and cycle number.
2. Fetch the issue and latest matching resource approval request.
3. If `request_comment_id` is supplied, verify the comment belongs to the issue
   and matches the cycle.
4. Fail if the requested cycle cannot be found. Do not infer approval for a
   missing request.
5. Post a structured issue comment:

```markdown
## Stage 10 Resource Authorization Decision

**Decision:** <Approve Cycle | Narrow Scope | Reject / Defer>
**Reviewer source:** <actor or Slack reviewer>
**Issue:** <issue URL>
**Cycle:** <N>
**Request comment:** <URL>
**Rationale:** <rationale or default>
**Authorized scope:** <only for approve>
**Next state:** <In Progress | Cycle Authorization | Deferred | Blocked>
```

6. For `approve`, include the proposed cycle goal from the request under
   `Authorized scope` and explicitly state that only that cycle is authorized.
7. For `narrow-scope`, include reviewer rationale when present. If no rationale
   is present from Slack, write `Narrow scope requested; reviewer details were
   not supplied in the button payload. Agent must request or use a narrower
   proposal before continuing.`
8. For `reject-defer`, record that no cycle is authorized.
9. Update existing labels only when they are present or can be added without
   creating labels.
10. Update Project V2 status using the same GraphQL pattern as
    `stage-approval.yml` when `GH_PROJECT_PAT` is configured.
11. Respond to Slack `response_url` when present, using the same best-effort
    pattern as `stage-approval.yml`.
12. Do not run implementation commands.
13. Do not dispatch a Ralph Cycle automatically.
14. Emit metadata-only workflow metrics.

This workflow must not be folded into `stage-approval.yml` unless Stage 9
explicitly approves that design and the Stage 5/7/9 matrix remains isolated from
Stage 10 authorization semantics.

### Slack approval bridge routing

Update `scripts/slack-approval-bridge/src/functions/slackApproval.js`.

Current behavior dispatches every Slack button to event type `stage-approval`.
Preserve that behavior for existing Stage 5, Stage 7, and Stage 9 buttons.

Add routing:

```javascript
const actionType = actionData.action_type || "stage_approval";
const eventType = actionType === "stage_10_authorization"
  ? "stage-10-authorization"
  : "stage-approval";
```

Required behavior:

- Include `action_type` in Stage 10 button values generated by the new
  notification workflow.
- Preserve existing payload fields:
  - `stage`
  - `issue_number`
  - `decision`
  - `rationale`
  - `pr_number`
  - `reviewer`
  - `slack_response_url`
- Add Stage 10 fields when present:
  - `cycle_number`
  - `request_comment_id`
- Preserve Slack signature verification and timestamp protection.
- Preserve immediate 200 response behavior so Slack does not retry.
- Update user-facing fallback text so Stage 10 failures point to
  `stage-10-authorization.yml`, while existing approvals continue pointing to
  `stage-approval.yml`.
- Add Node tests for action type routing and invalid payload handling.

Do not add a Slack modal in the first implementation unless the human explicitly
approves the added bridge complexity and any required Slack bot-token setting in
the Azure Function environment.

### Stage 11 implementation review request workflow

Create `.github/workflows/implementation-review-request.yml`.

Triggers:

```yaml
on:
  repository_dispatch:
    types: [implementation-review-request]
  workflow_dispatch:
    inputs:
      issue_url:
        required: false
        type: string
      issue_number:
        required: true
        type: number
      pr_number:
        required: false
        type: number
  issues:
    types: [labeled]
  pull_request:
    types: [labeled, ready_for_review]
  schedule:
    - cron: '0 * * * *'
```

Workflow behavior:

1. Ignore `issues.labeled` and `pull_request.labeled` unless the label is
   `needs-review`.
2. For scheduled runs, first check whether the repository has a `needs-review`
   label. If it does not, report that scheduled label-based lookup is
   unavailable and exit successfully without posting Slack.
3. Cap scheduled processing at 10 issues or PRs per run.
4. Resolve issue and PR:
   - workflow/dispatch inputs may provide both.
   - PR-triggered runs should parse issue references from the PR body using
     non-completing references such as `Related to #N`, `Supports #N`, or
     `Technical spec for #N`.
   - issue-triggered runs should find the PR from issue comments or linked PR
     references when possible. If ambiguous, fail with a clear diagnostic.
5. Fetch issue comments and find:
   - Stage 6 product spec artifact.
   - Stage 8 technical spec artifact.
   - latest Stage 10 evidence report or implementation coordination comment
     when available.
6. Query PR metadata and combined status/check information when available.
7. Build a Slack card with:
   - issue link
   - PR link
   - product spec artifact
   - technical spec artifact
   - check status summary
   - latest Stage 10 evidence link or `unavailable`
   - ready-to-run `duumbi-review-artifact` prompt
   - fallback links to the issue, PR, and manual workflow run
8. Do not include action buttons that run Stage 11 automatically.
9. Post Slack only when required issue/PR/spec links are known.
10. Write a marker only after Slack delivery succeeds.
11. Marker format:

```html
<!-- duumbi-implementation-review-slack-notified:v1 issue=595 pr=123 -->
```

12. Emit metadata-only workflow metrics.

Recommended Stage 11 prompt:

```text
Run DUUMBI Stage 11 Review Artifact with duumbi-review-artifact.

Target issue: <issue URL>
Implementation PR: <PR URL>
Product spec artifact: <product spec URL>
Technical spec artifact: <technical spec URL>

Goal: Verify CI, implementation evidence, BDD coverage, live E2E evidence, and
Ralph Cycle evidence against the approved specs; produce a structured review
artifact and recommendation for a human merge decision.

Do not merge PRs, close issues, move work to Done, perform Stage 12 closure, or
modify implementation code, specs, generated artifacts, or runtime assets.
```

### Skill guidance updates

Update `.agents/skills/duumbi-ralph-cycle/SKILL.md`:

- In the resource gate outcome, after writing the resource approval request,
  instruct the agent to use the exact heading and field names required by this
  technical spec.
- Tell the agent to add the existing `needs-cycle-approval` label only when
  available, or otherwise report that label-trigger integration is unavailable.
- Tell the agent to include the manual workflow fallback link to
  `.github/workflows/ralph-cycle-approval-request.yml`.
- Tell the agent not to create labels or Project fields.

Update `.agents/skills/duumbi-implementation/SKILL.md`:

- When routing `Request Resource Approval`, point to the Stage 10 notification
  workflow and the Stage 10 authorization decision workflow.
- When routing `Move To In Review`, tell the coordinator to add the existing
  `needs-review` label when available or trigger/report the manual
  `implementation-review-request.yml` path.
- Preserve the rule that implementation file edits happen only inside
  `duumbi-ralph-cycle`.

Update `.agents/skills/duumbi-review-artifact/SKILL.md`:

- Add the Stage 11 notification marker as context the reviewer may inspect.
- Keep the rule that Stage 11 review does not merge, close, move to Done, or
  edit implementation code/specs.

### Workflow metrics

Each new workflow must emit metadata-only metrics consistent with
`specs/DUUMBI-610/TECHNICAL.md`.

Required properties:

- `schema_version: "duumbi.workflow_metrics.v1"`
- workflow name, file, run ID, attempt, event, actor, ref, SHA, conclusion,
  timestamps, and duration.
- correlation:
  - issue number
  - PR number when applicable
  - stage (`"10"` or `"11"`)
  - decision when applicable
  - Project status target when applicable
- counts:
  - issues considered
  - issues queued
  - Slack notifications attempted
  - request parse failures
  - artifact links found/missing when applicable
- provider usage:
  - `available: false`
  - `reason: "no_provider_step"`
- privacy flags:
  - no raw Slack payloads
  - no issue/comment bodies
  - no prompts beyond the bounded ready-to-run Stage 11 prompt if reviewers
    accept it as workflow output; otherwise store only a prompt-present boolean
    in metrics.

Metrics generation and upload must be warning-only and must not mask the
primary workflow result.

### Rejected alternatives

- Add Stage 10 rows to `stage-approval.yml`: rejected because Stage 10 resource
  authorization is not product/spec approval and has different labels, statuses,
  and allowed decisions.
- Run implementation or Stage 11 review directly from Slack buttons: rejected
  by the product spec and active runbook.
- Create `needs-cycle-approval` and `needs-review` labels inside implementation:
  rejected because label creation is explicitly out of scope.
- Use Slack as durable state: rejected because GitHub is the source of truth.
- Store request comment bodies or Slack payload JSON in workflow metrics:
  rejected by the metadata-only metrics policy.
- Require Slack modals for `narrow-scope` in v1: rejected as a first slice
  because the current bridge has no modal path and no bridge-side Slack API
  token setting. Use manual rationale input or issue comments for detailed
  narrowing instructions.

## Invariants

- The execution issue remains open after this technical spec PR and the later
  implementation PR.
- Product spec content is not modified by Stage 10 implementation.
- Technical spec content is not modified by Stage 10 implementation unless a
  later Stage 9/Stage 8 revision path explicitly authorizes it.
- Stage 5, Stage 7, and Stage 9 approval behavior remains unchanged.
- Stage 10 authorization approves at most one named bounded cycle.
- Stage 10 authorization never runs implementation commands.
- Stage 11 handoff notification never runs review, merge, or closure commands.
- Slack notification markers are written only after successful Slack delivery.
- Idempotency markers are tied to a specific resource request or PR handoff and
  do not suppress later cycles or later review handoffs.
- Missing optional trigger labels do not block `workflow_dispatch` or
  `repository_dispatch`.
- Workflows do not create labels or Project fields.
- Slack and metrics payloads are sanitized and metadata-only.
- Workflow failures report missing required fields instead of guessing.
- Resource thresholds remain USD 2 or 10 external LLM calls unless a later
  approved spec changes them.

## BDD-To-Test Mapping

| Product BDD scenario | Evidence type | Required implementation evidence |
| --- | --- | --- |
| Repository dispatch notifies for a valid Stage 10 request | Unit tests, workflow fixture, optional live workflow smoke | Test `parseResourceApprovalRequest` with a valid `Ralph Cycle 2 Resource Approval Request` fixture. Simulate `repository_dispatch` payload for issue #595 and assert Slack message text/blocks include issue, cycle, resource estimate, proposed goal, planned checks, approval trigger, stop condition, and fallback link. Optional live smoke posts to Slack from a safe test issue only with human approval. |
| Duplicate Stage 10 notifications are suppressed | Unit tests and workflow fixture | Test marker matching with `request_comment_id` and cycle number. Assert the same request is skipped when marker exists and a later cycle/request comment is still eligible. Include scheduled, manual, and label-trigger fixture contexts if the implementation includes all three triggers. |
| Malformed Stage 10 request does not notify Slack | Unit tests and workflow negative fixture | Test a request missing `Resource Estimate`. Assert parse result lists the missing field, Slack payload is not built, no marker body is emitted, and the workflow exits with failure for manual/dispatch or warning skip for schedule. |
| Approve authorizes one bounded Ralph Cycle | Unit tests and workflow decision simulation | Simulate `stage-10-authorization` with `decision=approve`, cycle 2, and request comment ID. Assert decision comment states only cycle 2 is authorized, target state is `In Progress`, `needs-cycle-approval` removal is attempted only if present, and no implementation command is dispatched. |
| Narrow scope asks for a revised request | Unit tests and workflow decision simulation | Simulate `decision=narrow-scope` with and without rationale. Assert decision comment keeps the issue in `Cycle Authorization`, does not authorize the original cycle, and either includes supplied narrowing text or records that details must be supplied before continuing. |
| Reject-defer records no authorization | Unit tests and workflow decision simulation | Simulate `decision=reject-defer`. Assert decision comment records no authorization, target status is `Deferred` or fallback `Blocked`, and no implementation command is dispatched. |
| Review-ready implementation posts Stage 11 handoff | Unit tests, workflow fixture, optional live workflow smoke | Simulate issue/PR/spec artifacts and latest Stage 10 evidence. Assert Slack card includes issue, PR, product spec, technical spec, check/evidence summary, and ready-to-run `duumbi-review-artifact` prompt. Assert marker is tied to issue and PR. |
| Missing trigger labels do not block manual operation | Unit tests and workflow fixture | Simulate repository labels without `needs-cycle-approval` and `needs-review`. Assert `workflow_dispatch` still processes valid inputs and scheduled/label paths report label-trigger integration skipped without creating labels. |
| Slack is unavailable | Workflow simulation and optional live failure with dummy token | Simulate missing `SLACK_BOT_TOKEN`, missing `SLACK_REVIEW_CHANNEL_ID`, and Slack API failure. Assert no marker is written, workflow failure/warning is visible, and retry remains possible. |

Automation expectations:

- Run `node --test scripts/github-actions/duumbi-stage-handoff.test.mjs`.
- Run `node --check scripts/slack-approval-bridge/src/functions/slackApproval.js`
  or a stronger bridge test command if implementation adds one.
- Validate touched workflow YAML with:
  `ruby -e "require 'yaml'; ARGV.each { |f| YAML.load_file(f) }" <workflow...>`
  or an equivalent parser.
- Run `actionlint` on touched workflows when available.
- Use `rg` static checks to confirm new PR bodies/spec text do not introduce
  issue-specific auto-completion patterns for #595.
- Use `rg` static checks to confirm metrics code does not write
  `SLACK_BOT_TOKEN`, `GITHUB_TOKEN`, `GH_PROJECT_PAT`, Slack payload JSON, or
  broad comment bodies into artifacts.
- Live GitHub Actions/Slack smoke tests require a safe test issue or explicit
  human approval because they can post Slack messages, write issue comments,
  mutate labels, and attempt Project updates.

## Live E2E Plan

Canonical interface: GitHub Actions workflow dispatch and Slack Web API. This
issue is workflow/Slack-specific, not CLI, TUI, Studio, or provider behavior.

Real provider/LLM path:

- No DUUMBI live provider or external LLM call is required for this issue.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Codex internal reasoning usage during implementation is estimate-only and does
  not count as DUUMBI live provider usage.

Required credentials and configuration:

- GitHub Actions `GITHUB_TOKEN` for repository API calls.
- `GH_PROJECT_PAT` optional for Project V2 status mutation.
- `SLACK_BOT_TOKEN` with permission to post in the review channel.
- `SLACK_REVIEW_CHANNEL_ID` for notification target.
- Slack bridge Azure Function settings:
  - `SLACK_SIGNING_SECRET`
  - `GITHUB_TOKEN`
  - `GITHUB_REPO`

Stage 10 live smoke, when approved:

1. Create or select a safe open test issue.
2. Add a valid `## Ralph Cycle 1 Resource Approval Request` comment.
3. Run `ralph-cycle-approval-request.yml` with `workflow_dispatch`.
4. Pass criteria:
   - Slack receives one Stage 10 resource approval card.
   - Issue receives one marker comment after Slack success.
   - Re-running the same dispatch skips duplicate notification.
5. Run `stage-10-authorization.yml` manually for `approve`.
6. Pass criteria:
   - Issue receives a structured Stage 10 decision comment.
   - No implementation branch or file edits are created by the workflow.

Stage 11 live smoke, when approved:

1. Create or select a safe draft PR linked to a safe issue with product and
   technical spec comments.
2. Run `implementation-review-request.yml` with `workflow_dispatch`.
3. Pass criteria:
   - Slack receives one Stage 11 review handoff card.
   - Card includes issue, PR, product spec, technical spec, evidence/check
     summary, and ready-to-run Stage 11 prompt.
   - Issue receives one marker comment after Slack success.
   - Re-running the same dispatch skips duplicate notification.

Failure-path live smoke, when approved:

- Run a manual dispatch with a malformed resource request on a safe test issue.
- Pass criteria:
  - No Slack notification is posted.
  - No marker comment is written.
  - Workflow failure or warning names the missing required field.

Artifacts:

- GitHub Actions run URLs.
- Workflow summaries.
- Marker comments.
- Slack message timestamps from workflow logs.
- Metadata-only `duumbi-workflow-metrics.json` artifacts.

## Ralph Cycle Protocol

Each Stage 10 implementation cycle must:

1. Summarize the current state and remaining unmet requirements.
2. Propose one bounded implementation goal.
3. List intended file areas and commands.
4. Estimate resource use and risk.
5. Check whether the resource gate requires human approval.
6. Implement only the approved or resource-permitted goal.
7. Run the agreed checks.
8. Report evidence, failures, and remaining gaps.
9. Stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

For this issue, Stage 10 implementation must not combine all workflow and bridge
changes into one large unreviewable cycle. Separate workflow creation, helper
tests, bridge routing, and skill updates into bounded cycles.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 4 files or 2 closely related modules.
- Expected command budget per cycle:
  - `node --test scripts/github-actions/duumbi-stage-handoff.test.mjs`
  - `node --check scripts/slack-approval-bridge/src/functions/slackApproval.js`
  - YAML parser command for touched workflows
  - `actionlint` for touched workflows when available
  - targeted `rg` static checks
  - no full `cargo test --all` unless implementation unexpectedly touches Rust
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies or irreversible
  operations, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 3 low-budget Ralph cycles.
- When to stop and ask for human guidance:
  - a required label or Project status must be created rather than merely used
    when present
  - Slack modal support becomes necessary for `narrow-scope`
  - bridge deployment or Azure infrastructure changes are required
  - workflow smoke would affect a real production issue without explicit
    approval
  - existing Stage 5/7/9 approval behavior would need to change
  - test evidence shows the product BDD cannot be satisfied within the approved
    design

## Task Breakdown

1. Add helper and tests:
   - Create `scripts/github-actions/duumbi-stage-handoff.mjs`.
   - Create `scripts/github-actions/duumbi-stage-handoff.test.mjs`.
   - Cover request parsing, required-field errors, idempotency marker matching,
     Slack payload construction, Stage 10 decision comments, Stage 11 prompt
     construction, and metrics shape.
2. Add Stage 10 notification workflow:
   - Create `.github/workflows/ralph-cycle-approval-request.yml`.
   - Implement repository dispatch, manual dispatch, label trigger, and
     scheduled sweep.
   - Post Slack, write marker comments after success, and emit metrics.
3. Add Stage 10 authorization workflow:
   - Create `.github/workflows/stage-10-authorization.yml`.
   - Implement `approve`, `narrow-scope`, and `reject-defer`.
   - Reuse Project V2 update pattern from `stage-approval.yml`.
   - Emit metrics and workflow summary.
4. Extend Slack bridge routing:
   - Preserve existing `stage-approval` dispatch behavior.
   - Route `action_type: "stage_10_authorization"` to event type
     `stage-10-authorization`.
   - Add focused tests or testable seams for dispatch routing and invalid
     payloads.
   - Update bridge README.
5. Add Stage 11 review handoff workflow:
   - Create `.github/workflows/implementation-review-request.yml`.
   - Resolve issue/PR/spec/evidence links.
   - Post Slack card with ready-to-run `duumbi-review-artifact` prompt.
   - Write marker after success and emit metrics.
6. Update stage skills:
   - Update `duumbi-ralph-cycle`.
   - Update `duumbi-implementation`.
   - Update `duumbi-review-artifact`.
7. Add optional automation docs:
   - Document manual dispatch and live smoke procedures if workflow YAML and
     Slack behavior are not self-explanatory.
8. Run verification:
   - Node tests.
   - Bridge syntax/tests.
   - YAML parse.
   - `actionlint` when available.
   - Static secret/payload checks.
   - Optional live smoke only after human approval.
9. Prepare implementation PR evidence:
   - Link issue, product spec, technical spec, workflow runs or local evidence,
     and any live smoke artifacts.

## Verification Plan

Required local checks:

- `node --test scripts/github-actions/duumbi-stage-handoff.test.mjs`
- `node --check scripts/slack-approval-bridge/src/functions/slackApproval.js`
- YAML parser check for:
  - `.github/workflows/ralph-cycle-approval-request.yml`
  - `.github/workflows/stage-10-authorization.yml`
  - `.github/workflows/implementation-review-request.yml`
- `actionlint` for touched workflows when available.
- `rg -n "(?i)(close[sd]?|fix(e[sd])?|resolve[sd]?)\\s+#595"` on PR title/body
  text and spec files before publishing.
- `rg -n "SLACK_BOT_TOKEN|GH_PROJECT_PAT|GITHUB_TOKEN|client_payload|slack_messages"`
  on generated metrics fixtures, if any, to verify no secret or broad payload
  data is persisted.

Required review evidence:

- Diff review showing no implementation code is run from Slack buttons.
- Diff review showing no new labels or Project fields are created.
- Diff review showing Stage 5/7/9 approval matrix remains unchanged or only
  receives non-semantic refactoring explicitly justified by tests.
- Unit-test output for helper behavior.
- Workflow syntax evidence.
- Metrics artifact shape evidence from local fixture or safe workflow run.

Optional live checks:

- Stage 10 notification `workflow_dispatch` on a safe test issue.
- Stage 10 authorization `workflow_dispatch` on a safe test issue.
- Stage 11 handoff `workflow_dispatch` on a safe test issue/PR.

Do not run optional live checks against production issues without explicit human
approval during Stage 10.

## Completion Criteria

Implementation is complete when:

- `ralph-cycle-approval-request.yml` exists and supports repository dispatch,
  manual dispatch, existing-label trigger, scheduled label sweep, parsing,
  Slack notification, idempotency marker, and metrics.
- `stage-10-authorization.yml` exists and supports `approve`, `narrow-scope`,
  and `reject-defer` with structured issue comments, label/status updates when
  available, Slack response follow-up, and metrics.
- `implementation-review-request.yml` exists and supports dispatch/manual,
  existing-label and PR triggers, issue/PR/spec/evidence resolution, Slack
  notification, ready-to-run Stage 11 prompt, marker, and metrics.
- The Slack approval bridge routes Stage 10 authorization actions to the
  Stage 10 workflow while preserving existing Stage 5/7/9 routing.
- Stage skills document the new request, authorization, and review-handoff
  contracts without allowing label creation or unbounded implementation.
- BDD-to-test mapping evidence is attached to the implementation PR.
- Live E2E evidence is either attached from safe workflow runs or explicitly
  marked not run with the reason and local substitute evidence.
- No implementation changes exceed the approved product and technical scope.
- No source, metrics, or Slack payload stores secrets, raw comments, raw issue
  bodies, raw prompts, raw completions, or broad payload dumps.

## Failure And Escalation

When tests fail:

- Stop the current Ralph cycle after reporting the failing command and the
  smallest known cause.
- Do not broaden workflow architecture or bridge scope without approval.

When workflow parsing fails:

- Fix YAML syntax within the same bounded cycle if it is local and obvious.
- If parser/actionlint findings require changing trigger semantics, stop and
  request guidance.

When Slack delivery fails:

- Do not write marker comments.
- Preserve retryability.
- Report whether failure is missing secret, channel, Slack API error, or payload
  validation.

When labels are missing:

- Do not create them.
- Keep manual and repository dispatch paths working.
- Report label-trigger integration as unavailable.

When Project V2 mutation fails:

- Keep comments and labels as durable evidence.
- Report Project status as unavailable or not updated.
- Do not block notification behavior solely on Project mutation failure.

When `narrow-scope` needs detailed human input:

- Prefer manual `workflow_dispatch` with rationale or an issue comment.
- Do not add Slack modal support unless the human explicitly approves the extra
  bridge complexity and required environment settings.

When a live smoke test would mutate a real issue/PR/Slack channel:

- Ask for explicit human approval or use local fixture evidence instead.

## Open Questions

None block implementation under this technical spec.

Non-blocking Stage 9 review considerations:

- Stage 9 may choose to require Slack modal support for detailed
  `narrow-scope` input, but this technical spec recommends deferring that until
  a separate bridge enhancement is accepted.
- Stage 9 may choose whether optional docs belong in
  `docs/automation/stage10-stage11-slack-handoffs.md` or whether workflow
  summaries and README updates are enough.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/595
- Product spec PR: https://github.com/hgahub/duumbi/pull/616
- Product spec: `specs/DUUMBI-595/PRODUCT.md`
- Stage Approval workflow: `.github/workflows/stage-approval.yml`
- Human Acceptance Request workflow:
  `.github/workflows/human-acceptance-request.yml`
- Spec Review Request workflow: `.github/workflows/spec-review-request.yml`
- Technical Spec Review Request workflow:
  `.github/workflows/technical-spec-review-request.yml`
- Slack bridge: `scripts/slack-approval-bridge/src/functions/slackApproval.js`
- Slack bridge README: `scripts/slack-approval-bridge/README.md`
- Ralph Cycle skill: `.agents/skills/duumbi-ralph-cycle/SKILL.md`
- Implementation coordinator skill: `.agents/skills/duumbi-implementation/SKILL.md`
- Review artifact skill: `.agents/skills/duumbi-review-artifact/SKILL.md`
- Workflow metrics technical spec: `specs/DUUMBI-610/TECHNICAL.md`
- Automation guide: `docs/automation/human-acceptance-slack-gate.md`
- Active runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Pipeline automation source note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/DUUMBI Pipeline Automation Spec.md`
