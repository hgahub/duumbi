# DUUMBI-595: Notify Stage 10 Authorization And Stage 11 Review Handoffs

## Summary

DUUMBI should notify the developer in Slack when implementation work reaches a
Stage 10 Ralph Cycle resource gate or a Stage 11 review handoff.

The notification path should be deterministic GitHub Actions glue around the
existing GitHub issue, PR, labels, Project status, and stage comments. It should
not run implementation work, approve resource-gated cycles, review PRs, merge
PRs, or change the resource threshold policy.

For v1, the accepted surface is:

```text
Stage comment or dispatch/label trigger -> GitHub Actions validation -> Slack
notification with fallback GitHub workflow link -> explicit human decision
recorded back on the issue
```

This is a specification PR only. Related to #595; the execution issue must stay
open for Stage 7 review and later workflow stages.

## Problem

DUUMBI already has deterministic Slack handoffs for human acceptance, product
spec review, and technical spec review. Implementation handoffs are weaker.

When a Ralph Cycle hits the resource gate, the agent writes a resource approval
request and stops. When implementation evidence is ready for Stage 11 review,
the developer still has to notice the stopped Codex run, inspect GitHub, and
manually launch the next stage. That makes the developer the polling loop during
the highest-friction part of the workflow.

This creates avoidable delays and safety risk:

- A valid resource request can sit unnoticed.
- A review-ready PR can wait without a Stage 11 review request.
- Manual prompts can omit the issue, PR, spec artifacts, resource estimate, or
  stage boundary warnings.
- Existing Stage 5, Stage 7, and Stage 9 approval semantics are not a clean fit
  for Stage 10 resource authorization decisions.

## Outcome

When this issue is implemented:

- A Stage 10 Ralph Cycle resource approval request can trigger a Slack
  notification without requiring the developer to watch Codex.
- The Slack card shows the issue, cycle number, resource estimate, proposed
  cycle goal, planned checks, approval trigger, stop condition, and fallback
  workflow link.
- Human decisions for resource requests are recorded as structured Stage 10
  authorization comments on the issue.
- Stage 10 decisions support at least `approve`, `narrow-scope`, and
  `reject-defer`.
- Stage 10 decision handling uses Stage 10 authorization semantics instead of
  reusing Stage 5, Stage 7, or Stage 9 approval transitions.
- A review-ready implementation handoff can trigger a Slack notification for
  Stage 11 Review Artifact work.
- The Stage 11 notification includes the issue, implementation PR, product spec,
  technical spec, current evidence, and a ready-to-run Stage 11 Codex prompt.
- GitHub remains the durable source of truth. Slack is a notification and action
  surface, not the only evidence path.
- Missing labels do not block manual or `repository_dispatch` operation. Label
  creation remains outside this issue unless label-management work separately
  approves it.

## Scope

### In Scope

- Add deterministic notification support for Stage 10 resource authorization.
- Add deterministic notification support for Stage 11 review handoff.
- Define the machine-parseable Ralph Cycle resource approval request contract.
- Validate and parse the latest relevant resource approval request comment from
  the issue.
- Post a Slack notification for a resource request through configured Slack
  secrets.
- Include a fallback GitHub Actions manual-dispatch link in Slack.
- Prevent duplicate Slack notifications for the same resource request through a
  durable issue-comment marker or equivalent GitHub-visible marker.
- Add an explicit Stage 10 authorization decision path for:
  - `approve`
  - `narrow-scope`
  - `reject-defer`
- Record Stage 10 authorization decisions as structured GitHub issue comments.
- Update existing labels and Project status when configured and available.
- Support existing trigger labels such as `needs-cycle-approval` and
  `needs-review` when those labels exist.
- Support `repository_dispatch` and `workflow_dispatch` so the workflow still
  works before new labels are created.
- Update `duumbi-ralph-cycle` guidance so resource-gated cycles write the
  machine-parseable request and trigger or label the authorization path when
  available.
- Update `duumbi-implementation` guidance so Stage 10 coordinator handoffs
  point to the deterministic authorization path.
- Update `duumbi-review-artifact` or adjacent review-handoff guidance so
  review-ready implementation work can trigger the Stage 11 notification.
- Add focused validation for comment parsing, Slack payload generation,
  idempotency, decision routing, and fallback behavior where practical.

### Explicitly Out Of Scope

- Automatically approving resource-gated Ralph Cycles.
- Running implementation cycles from Slack button clicks.
- Running Stage 11 Review Artifact work from Slack button clicks.
- Changing resource gate thresholds, including external LLM call or cost caps.
- Creating new GitHub labels, Project fields, or Project views.
- Creating additional product specs, technical specs, implementation code, or
  Ralph cycles as part of this Stage 6 handoff.
- Changing Stage 5, Stage 7, or Stage 9 approval semantics.
- Replacing the existing Slack approval bridge architecture.
- Merging PRs, closing execution issues, or moving completed work to Done.
- Capturing raw prompts, completions, secrets, large logs, or private payloads
  in Slack messages.

## Constraints And Assumptions

Facts:

- Issue #595 is open and accepted for specification.
- Issue #595 is labeled `accepted` and `needs-spec` at Stage 6 intake.
- The canonical Stage 5 decision comment on 2026-05-23 records
  `Decision: Accept`, `Next state: Spec Needed`, and no open questions.
- `.github/workflows/human-acceptance-request.yml`,
  `.github/workflows/spec-review-request.yml`, and
  `.github/workflows/technical-spec-review-request.yml` already provide Slack
  notifications for earlier human review gates.
- `.github/workflows/stage-approval.yml` currently handles Stage 5, Stage 7,
  and Stage 9 decisions.
- `scripts/slack-approval-bridge` currently bridges Slack button clicks to
  `stage-approval.yml` through `repository_dispatch`.
- `duumbi-ralph-cycle` already defines the resource gate and a structured
  `Ralph Cycle <N> Resource Approval Request` comment shape.
- The active runbook treats GitHub Issues and Project status as execution source
  of truth and Slack as a notification/action surface.

Assumptions:

- Stage 10 authorization decisions should be handled by a dedicated Stage 10
  path or a clearly separated Stage 10 mode, not by overloading Stage 5, Stage 7,
  or Stage 9 transitions.
- `repository_dispatch` and `workflow_dispatch` are required baseline triggers
  because first-class labels may not exist yet.
- Label triggers are useful when labels already exist, but this issue should not
  create labels as part of implementation.
- A ready-to-run Stage 11 Codex prompt is sufficient for v1 review handoff.
  Automatically launching Codex is future work.
- Slack delivery can fail or be unconfigured, so GitHub comments, summaries, and
  workflow logs must retain enough evidence to continue manually.

Constraints:

- Workflow failures must not silently mark a request as notified.
- Idempotency markers must be specific enough to avoid suppressing a later cycle
  request for the same issue.
- Slack payloads must sanitize GitHub text before posting.
- Missing required request fields must fail with actionable GitHub-visible
  diagnostics instead of guessing.
- Decision comments must avoid GitHub auto-completion language for the execution
  issue.
- Stage 10 approval must authorize only the requested bounded cycle, not all
  future resource-gated work.

## Decisions

- **Decision:** Use a file-based product spec for #595.
  **Evidence:** The work spans GitHub Actions, Slack notification payloads,
  issue comments, labels, Project status, Stage 10 and Stage 11 guidance, and
  the Slack approval bridge boundary. It needs source-controlled review history.

- **Decision:** Stage 10 authorization should use dedicated Stage 10 semantics.
  **Evidence:** `stage-approval.yml` currently models Stage 5 acceptance,
  Stage 7 product spec review, and Stage 9 technical spec review. Stage 10
  resource authorization is not a spec approval; it approves one bounded
  resource-gated implementation cycle.

- **Decision:** Trigger labels are optional integration points, not required
  infrastructure for v1.
  **Evidence:** Issue #595 explicitly asks whether `needs-cycle-approval` and
  `needs-review` should become first-class labels and keeps label creation out of
  implementation unless separately approved.

- **Decision:** Stage 11 handoff should notify and prepare review, not perform
  review.
  **Evidence:** The runbook separates implementation coordination from Stage 11
  Review Artifact work. The human still needs a visible handoff before review
  and merge decisions.

- **Decision:** GitHub must retain the durable decision record.
  **Evidence:** Slack messages are transient and can fail. DUUMBI execution
  state lives in GitHub Issues, PRs, comments, labels, and Project status.

## Behavior

### Stage 10 Resource Approval Request Contract

A resource-gated Ralph Cycle request is valid when the latest relevant issue
comment contains a stable heading and these fields:

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

The stable heading is:

```text
## Ralph Cycle <N> Resource Approval Request
```

The workflow may accept additional fields, but it must not require prose outside
the structured fields above.

### Stage 10 Notification

The Stage 10 notification path can be started by:

- `repository_dispatch` with a Stage 10 resource authorization event type.
- `workflow_dispatch` with issue number or issue URL.
- `issues.labeled` when an existing resource-approval label is configured and
  present.

The workflow:

1. Validates that the issue belongs to this repository and is open.
2. Finds the latest unnotified `Ralph Cycle <N> Resource Approval Request`
   comment.
3. Validates required fields.
4. Builds a sanitized Slack card with the issue, cycle, resource estimate,
   proposed cycle goal, planned checks, approval trigger, stop condition, and
   fallback workflow link.
5. Posts to Slack when Slack secrets are configured.
6. Writes a durable notification marker tied to the request comment ID or cycle
   number only after Slack delivery succeeds.
7. Writes a workflow summary with the parsed request and notification result.

If parsing fails, the workflow should not post Slack and should leave an
actionable GitHub-visible diagnostic.

### Stage 10 Decision Routing

Stage 10 decisions are explicit resource authorization decisions:

- `approve` authorizes the requested bounded cycle, records a structured
  decision comment, removes the resource-approval label when available, and
  routes the issue back to implementation work such as `In Progress` when
  Project status mutation is configured.
- `narrow-scope` records that the human wants a smaller cycle, keeps the issue
  in resource authorization state or equivalent visible waiting state, and asks
  the agent to revise the request before continuing.
- `reject-defer` records that the requested cycle is not authorized now and
  routes the issue to a deferred or blocked state when that Project status is
  available.

The decision path must not run implementation commands. It only records
authorization state and prepares the next human- or agent-run step.

### Stage 11 Review Handoff Notification

The Stage 11 review handoff path can be started by:

- `repository_dispatch` when implementation evidence is ready.
- `workflow_dispatch` with issue number or PR number.
- `issues.labeled` or `pull_request` triggers when an existing review-handoff
  label or state is configured and present.

The workflow:

1. Validates the issue and implementation PR.
2. Finds or accepts product spec and technical spec artifact links.
3. Includes recent implementation evidence when available, such as PR URL,
   latest checks, and the Stage 10 completion or handoff comment.
4. Builds a sanitized Slack card that asks for Stage 11 Review Artifact work.
5. Includes a ready-to-run Codex prompt using `duumbi-review-artifact`.
6. Includes a fallback GitHub workflow or issue link.
7. Writes a durable notification marker only after Slack delivery succeeds.

The notification must not claim the implementation is correct. It only says the
work is ready for Stage 11 review.

## BDD Scenarios

### Scenario: Repository dispatch notifies for a valid Stage 10 request

Given issue #595 has a latest unnotified `Ralph Cycle 2 Resource Approval
Request` comment with all required fields
And the workflow receives a Stage 10 resource authorization dispatch for issue
#595
When Slack secrets are configured
Then the workflow posts a Slack card for cycle 2
And the card includes the issue, resource estimate, proposed cycle goal, planned
checks, approval trigger, stop condition, and fallback workflow link
And the workflow writes a durable notification marker for that request

### Scenario: Duplicate Stage 10 notifications are suppressed

Given issue #595 already has a notification marker for the latest resource
approval request
When the scheduled, label, or manual path evaluates the same request again
Then the workflow does not post a second Slack message for that request
And the workflow summary reports that the request was already notified

### Scenario: Malformed Stage 10 request does not notify Slack

Given issue #595 has a `Ralph Cycle 3 Resource Approval Request` comment
And the comment omits `Resource Estimate`
When the Stage 10 notification workflow parses the request
Then the workflow does not post Slack
And the workflow reports the missing field in a GitHub-visible diagnostic
And no notification marker is written

### Scenario: Approve authorizes one bounded Ralph Cycle

Given Slack sends a Stage 10 `approve` decision for issue #595 cycle 2
When the Stage 10 authorization workflow handles the decision
Then it writes a structured Stage 10 authorization comment
And it records that only cycle 2 is authorized
And it removes the resource-approval label when available
And it routes the issue back to implementation work when Project mutation is
configured
And it does not run implementation commands

### Scenario: Narrow scope asks for a revised request

Given Slack sends a Stage 10 `narrow-scope` decision for issue #595 cycle 2
When the Stage 10 authorization workflow handles the decision
Then it writes a structured decision comment with the requested narrowing
And it leaves the issue in a visible waiting-for-authorization state
And it does not authorize the original cycle
And it does not run implementation commands

### Scenario: Reject defer records no authorization

Given Slack sends a Stage 10 `reject-defer` decision for issue #595 cycle 2
When the Stage 10 authorization workflow handles the decision
Then it writes a structured decision comment
And it records that the requested cycle is not authorized now
And it routes the issue to a deferred or blocked state when that status is
available
And it does not run implementation commands

### Scenario: Review-ready implementation posts Stage 11 handoff

Given issue #595 has a linked implementation PR
And product and technical spec artifact links are known
And implementation evidence is marked ready for review
When the Stage 11 handoff workflow runs
Then it posts a Slack card requesting Stage 11 Review Artifact work
And the card includes the issue, PR, product spec, technical spec, evidence
summary, and ready-to-run `duumbi-review-artifact` prompt
And the workflow writes a durable notification marker

### Scenario: Missing trigger labels do not block manual operation

Given `needs-cycle-approval` or `needs-review` is not present in the repository
When a valid `repository_dispatch` or `workflow_dispatch` request is made
Then the workflow still validates the issue and can notify Slack
And the workflow summary reports that label-trigger integration was skipped or
unavailable
And the implementation does not create the missing labels

### Scenario: Slack is unavailable

Given a valid Stage 10 resource request exists
And Slack secrets are missing or Slack delivery fails
When the notification workflow runs
Then it does not write a notification marker
And it writes a clear workflow failure or warning with the fallback manual path
And the request can be retried after Slack configuration is fixed

## Acceptance Criteria

- A file-based product spec exists at `specs/DUUMBI-595/PRODUCT.md`.
- A spec-only draft PR references issue #595 with non-completing language and
  states that the execution issue remains open.
- Stage 10 resource approval notification behavior is specified with triggers,
  parsing, Slack output, idempotency, fallback, and error handling.
- Stage 10 authorization decisions are specified separately from Stage 5, Stage
  7, and Stage 9 approval semantics.
- Stage 11 review handoff notification behavior is specified with issue, PR,
  spec, evidence, prompt, idempotency, and fallback expectations.
- Label creation remains out of scope; workflows can use existing labels but
  must support dispatch/manual operation.
- BDD scenarios cover valid notification, duplicate suppression, malformed
  request handling, each Stage 10 decision, Stage 11 handoff, missing labels,
  and Slack failure.

## Suggested Validation

- Validate workflow YAML syntax and run `actionlint` if available.
- Add parser tests or script-level fixtures for valid and malformed Ralph Cycle
  resource approval request comments.
- Add payload-generation tests or dry-run fixtures for Stage 10 and Stage 11
  Slack cards.
- Add idempotency tests showing that the same request comment is notified once
  and a later cycle request can still notify.
- Add decision-routing tests for `approve`, `narrow-scope`, and `reject-defer`.
- Run a controlled `workflow_dispatch` dry run or safe test issue path before
  enabling label-triggered production notifications.

## Open Questions For Stage 7

- Should `needs-cycle-approval` and `needs-review` become first-class repository
  labels in a separate label-management change?
- Should the Stage 10 decision path be a new workflow file or a Stage 10 mode in
  an existing workflow, as long as semantics remain separate from Stage 5, Stage
  7, and Stage 9?
- Should Stage 11 handoff notification be issue-driven, PR-driven, or support
  both as first-class triggers?
- What exact Slack button payload shape should the bridge use for
  `narrow-scope`, including where the human-entered narrowing instruction is
  captured?
