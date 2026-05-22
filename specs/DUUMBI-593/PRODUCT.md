# DUUMBI-593: Add Ready-To-Run Codex Prompts After Approval Gates

## Summary

After a human approves Stage 5, Stage 7, or Stage 9, DUUMBI should generate a
copy-ready Codex launch prompt for the next generative stage.

The prompt should appear in the approval result surfaces the developer already
checks: Slack when Slack notification is configured, the GitHub decision comment,
and the GitHub Actions workflow summary. The prompt must include the target
issue, known spec artifacts, the correct skill name, the stage goal, and explicit
stage boundary warnings.

This work improves workflow handoff ergonomics only. It must not auto-start
Codex, bypass review gates, approve specs, create specs, generate technical
plans, start implementation, or close the execution issue.

## Problem

The current Stage Approval workflow records approval decisions, updates labels,
updates Project status when configured, and notifies Slack. The developer still
has to manually find the next runbook prompt, choose the right skill, copy the
issue URL, find product or technical spec artifact URLs, and restate the stage
boundary.

That makes the developer the orchestration layer at every approval handoff. The
friction is small once, but it repeats at Stage 5 to Stage 6, Stage 7 to Stage
8, and Stage 9 to Stage 10. It also creates avoidable risk: malformed manual
prompts can omit spec-only PR warnings, point to stale artifacts, or accidentally
ask an agent to cross into a later stage.

## Outcome

When this is done:

- A Stage 5 approval result gives the developer a ready-to-run Stage 6
  `duumbi-spec-draft` prompt.
- A Stage 7 approval result gives the developer a ready-to-run Stage 8
  `duumbi-tech-spec-draft` prompt with the product spec artifact when available.
- A Stage 9 approval result gives the developer a ready-to-run Stage 10
  `duumbi-implementation` prompt with the product and technical spec artifacts
  when available.
- The generated prompt is available in Slack when Slack notification succeeds.
- The generated prompt is also available from GitHub-only surfaces so Slack is
  not the only access path.
- Missing artifact data is called out explicitly in the generated prompt instead
  of being silently omitted or guessed.
- Prompt text includes the correct stage boundary warnings, including the
  spec-only PR rule for Stage 6 and Stage 8.
- Existing approval routing, labels, Project updates, Slack button handling, and
  human gates continue to work as before.

## Scope

### In Scope

- Extend `.github/workflows/stage-approval.yml` so successful `approve`
  decisions for Stage 5, Stage 7, and Stage 9 generate next-stage Codex prompts.
- Include generated prompts in the approval result Slack message when Slack is
  configured.
- Include generated prompts in the Stage Approval workflow summary.
- Include generated prompts in the GitHub decision comment, or otherwise provide
  an equally durable GitHub-linked fallback from the decision comment.
- Reuse or consolidate the artifact lookup logic already present in
  `stage-approval.yml` for Stage 7 and Stage 9.
- Detect product spec and technical spec artifact links from existing Stage 6
  and Stage 8 issue comments where those comments exist.
- Show explicit placeholders such as `<missing: product spec artifact>` when an
  artifact cannot be found.
- Generate prompts only for approval paths:
  - Stage 5 `approve` -> Stage 6 Product Spec Draft.
  - Stage 7 `approve` -> Stage 8 Technical Spec Draft.
  - Stage 9 `approve` -> Stage 10 Implementation Coordination.
- Preserve the existing Slack approval bridge contract that dispatches Stage
  Approval decisions into GitHub Actions.
- Add focused validation for prompt generation behavior where practical, either
  as unit-testable helper logic, workflow-script tests, or a controlled
  `workflow_dispatch` evidence path.

### Explicitly Out Of Scope

- Automatically launching Codex, Oz, or any other agent after an approval.
- Creating one-click local Codex deep links unless Codex already exposes a safe,
  supported launch URL that requires no additional product design.
- Creating or changing product specs, technical specs, implementation code, or
  Ralph-cycle work as part of the approval result.
- Changing Stage 5, Stage 7, or Stage 9 decision semantics.
- Adding new approval stages or changing Stage 10 resource-gate policy.
- Adding post-merge closure automation.
- Adding Ralph-cycle approval notification workflows.
- Replacing the Slack approval bridge architecture.
- Moving prompt text into a new reusable module unless the workflow script
  becomes materially hard to maintain.
- Creating new GitHub labels, Project fields, or Obsidian artifacts.

## Constraints And Assumptions

Facts:

- Issue #593 is open and accepted for specification.
- Issue #593 is labeled `accepted` and `needs-spec`.
- Issue #593 is in the `Spec Needed` Project status.
- The Stage 5 decision comment on 2026-05-22 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- `.github/workflows/stage-approval.yml` currently handles approval decisions
  for Stage 5, Stage 7, and Stage 9.
- `stage-approval.yml` already posts a structured decision comment, mutates
  labels, attempts Project V2 status updates through `GH_PROJECT_PAT`, posts a
  Slack summary when Slack is configured, and writes a workflow summary.
- `stage-approval.yml` already contains artifact lookup logic for Stage 7 and
  Stage 9 by reading Stage 6 and Stage 8 issue comments.
- `human-acceptance-request.yml`, `spec-review-request.yml`, and
  `technical-spec-review-request.yml` already notify Slack when issues are in
  the corresponding review states.
- `scripts/slack-approval-bridge/src/functions/slackApproval.js` currently
  converts Slack button actions into `repository_dispatch` events for
  `stage-approval.yml`.
- The active DUUMBI runbook says Stage 6 and Stage 8 spec PRs are spec-only
  review artifacts and must not close the execution issue.

Assumptions:

- A copy-ready prompt is sufficient for this issue. A deeper Codex launch URL is
  future work unless Codex already provides a safe supported URL during
  implementation.
- The Stage Approval workflow is the right first place for prompt generation
  because it already has the issue, decision, artifact lookup, Slack, and summary
  context.
- The prompt builder can start as a shared helper inside the GitHub Actions
  script. Extraction to a separate JavaScript module is justified only if tests
  or maintainability make the inline script too hard to reason about.
- Slack delivery may fail or be unconfigured, so GitHub must retain a durable
  prompt fallback.

Constraints:

- Generated prompts must not use GitHub auto-close keywords when referring to
  the execution issue in spec-only Stage 6 or Stage 8 contexts.
- Generated prompts must preserve stage boundaries and explicitly say what the
  next agent must not do.
- Prompt generation must not make approval workflows depend on optional Slack
  configuration.
- Missing artifact lookup must be visible and conservative.
- Existing label and Project state transitions remain the source of workflow
  state; generated prompts are launch assistance, not state.

## Decisions

- **Decision:** Use a file-based product spec for #593.
  **Evidence:** The work affects GitHub Actions, Slack-visible behavior,
  approval handoffs, artifact lookup, and workflow safety warnings. It is
  durable enough to require review history.

- **Decision:** Generate next-stage prompts only for approval decisions.
  **Evidence:** `request-changes`, `needs-clarification`, and `reject` do not
  authorize the next generative stage. Producing a launch prompt for those paths
  would conflict with the stage model.

- **Decision:** Stage 9 approval should point to `duumbi-implementation`, not
  directly to a Ralph cycle.
  **Evidence:** The active runbook separates Stage 10 implementation
  coordination from per-cycle Ralph execution. The coordinator verifies branch,
  PR, blocker, and evidence state before routing bounded cycle work.

- **Decision:** Prompt output should be present in GitHub as well as Slack.
  **Evidence:** Slack is a communication surface, while GitHub Issues, Project
  state, PRs, and workflow summaries are durable execution evidence. A Slack-only
  prompt would not be sufficient as an audit or fallback path.

- **Decision:** Missing artifact URLs should be explicit placeholders, not
  omitted fields.
  **Evidence:** Issue #593 identifies stale or malformed prompt generation as
  the main risk. Silent omission would make it easier to launch the wrong stage
  with incomplete context.

- **Decision:** This spec PR must not close the execution issue.
  **Evidence:** Stage 6 creates a reviewable product spec. Stage 7, Stage 8,
  Stage 9, Stage 10, Stage 11, and Stage 12 still need to happen.

## Behavior

### Defaults

- Approval result prompts are generated only when the Stage Approval decision is
  `approve`.
- No prompt is generated for rejected, clarification, or change-request paths.
- Prompt generation does not alter the existing decision comment fields, labels,
  Project status, or Slack approval bridge input contract except by adding the
  next-stage prompt output.
- GitHub output is required; Slack output is best-effort based on existing Slack
  configuration.

### Inputs

- Stage Approval input:
  - `stage`
  - `issue_number`
  - `decision`
  - `rationale`
  - optional `pr_number`
  - optional Slack response URL from the Slack approval bridge
- GitHub issue title, URL, labels, comments, and existing Stage 6 or Stage 8
  artifact comments.
- Existing runbook stage prompt text and stage boundary warnings.

### Outputs

For Stage 5 approval, the generated prompt includes:

- Stage name: Stage 6 Product Spec Draft.
- Skill name: `duumbi-spec-draft`.
- Target issue URL.
- Goal: verify Stage 5 acceptance, inspect context, draft the English product
  spec with BDD scenarios, open a draft spec PR if file-based, link the spec from
  the issue, and move the issue to Spec Review.
- Boundary warning: do not create technical specs, implementation code, or Ralph
  cycles.
- Spec-only PR rule: do not use auto-close keywords for the execution issue; use
  non-closing references such as `Related to #593` or `Spec for #593`; include a
  workflow note that the spec PR must not close the execution issue.

For Stage 7 approval, the generated prompt includes:

- Stage name: Stage 8 Technical Spec Draft.
- Skill name: `duumbi-tech-spec-draft`.
- Target issue URL.
- Product spec artifact URL or an explicit missing-artifact placeholder.
- Goal: verify product spec approval, inspect repo instructions and affected
  source areas, draft the technical spec with BDD-to-test mapping, live E2E plan,
  and Ralph Cycle resource policy, open a draft PR, link it from the issue, and
  move the issue to Technical Spec Review.
- Boundary warning: do not change implementation code, approve the technical
  spec, start implementation, or run Ralph cycles.
- Spec-only PR rule for the Stage 8 technical spec PR.

For Stage 9 approval, the generated prompt includes:

- Stage name: Stage 10 Implementation Coordination.
- Skill name: `duumbi-implementation`.
- Target issue URL.
- Product spec artifact URL or an explicit missing-artifact placeholder.
- Technical spec artifact URL or an explicit missing-artifact placeholder.
- Goal: verify Ready for Build context, manage branch and PR readiness,
  consolidate Ralph-cycle evidence when relevant, choose the next routed action,
  and delegate implementation edits only through bounded Ralph cycles.
- Boundary warning: do not exceed approved product and technical specs, do not
  skip resource gates, and do not close the issue without Stage 12 closure.

### Visible States

- Slack approval result with next-stage prompt when Slack is configured and the
  Slack post succeeds.
- GitHub decision comment containing or linking the next-stage prompt.
- GitHub Actions workflow summary containing the next-stage prompt.
- Existing Project status after the approval remains:
  - Stage 5 approval -> `Spec Needed`.
  - Stage 7 approval -> `Technical Spec Needed`.
  - Stage 9 approval -> `Ready for Build`.

### Empty And Missing States

- If no Stage 6 product spec comment is found for Stage 7 or Stage 9 prompt
  generation, the prompt includes `<missing: product spec artifact>`.
- If no Stage 8 technical spec comment is found for Stage 9 prompt generation,
  the prompt includes `<missing: technical spec artifact>`.
- If a PR number cannot be inferred, the prompt still uses the issue URL and the
  explicit missing-artifact field.
- Missing artifact fields do not fail the approval decision after the human has
  approved; they create visible follow-up work for the next operator.

### Error And Degraded States

- If Slack notification fails, the workflow still posts the GitHub decision
  comment and workflow summary.
- If Project V2 update fails, prompt generation still occurs and the existing
  Project warning behavior remains.
- If issue comment reads fail, the workflow should fail or warn consistently
  with the existing approval behavior. It must not produce artifact URLs that
  were not verified.
- If prompt generation fails after the approval decision is already recorded, the
  workflow summary and logs should make the failure visible.

### Invariants

- Generated prompts are assistance text, not workflow state.
- Human approval gates remain authoritative.
- Spec-only PRs remain open review artifacts and must not close execution
  issues.
- Stage 10 implementation still requires approved product and technical specs.
- Resource-gated Ralph cycle policy remains unchanged.

## BDD Scenarios

Feature: Approval results provide next-stage Codex prompts

  Rule: Prompts are generated only after approval decisions

    Scenario: Stage 5 approval produces a Stage 6 product spec prompt
      Given an open DUUMBI issue has been accepted at Stage 5
      When the Stage Approval workflow records Stage 5 with decision "approve"
      Then the approval result includes a ready-to-run Stage 6 prompt
      And the prompt names the `duumbi-spec-draft` skill
      And the prompt includes the target issue URL
      And the prompt says not to create technical specs, implementation code, or Ralph cycles
      And the prompt includes the spec-only PR rule for the execution issue

    Scenario: Stage 7 approval produces a Stage 8 technical spec prompt
      Given an open DUUMBI issue has a Stage 6 product spec artifact comment
      When the Stage Approval workflow records Stage 7 with decision "approve"
      Then the approval result includes a ready-to-run Stage 8 prompt
      And the prompt names the `duumbi-tech-spec-draft` skill
      And the prompt includes the target issue URL
      And the prompt includes the product spec artifact URL
      And the prompt says not to change implementation code or run Ralph cycles

    Scenario: Stage 9 approval produces a Stage 10 implementation coordination prompt
      Given an open DUUMBI issue has product and technical spec artifact comments
      When the Stage Approval workflow records Stage 9 with decision "approve"
      Then the approval result includes a ready-to-run Stage 10 prompt
      And the prompt names the `duumbi-implementation` skill
      And the prompt includes the target issue URL
      And the prompt includes the product spec artifact URL
      And the prompt includes the technical spec artifact URL
      And the prompt preserves the Ralph Cycle resource-gate boundary

  Rule: Non-approval decisions do not launch the next stage

    Scenario: Stage 7 request changes does not produce a Stage 8 launch prompt
      Given an open DUUMBI issue is in product spec review
      When the Stage Approval workflow records Stage 7 with decision "request-changes"
      Then the approval result does not include a Stage 8 Codex launch prompt
      And the issue remains routed back to product spec work

    Scenario: Stage 5 needs clarification does not produce a Stage 6 launch prompt
      Given an open DUUMBI issue needs human acceptance
      When the Stage Approval workflow records Stage 5 with decision "needs-clarification"
      Then the approval result does not include a Stage 6 Codex launch prompt
      And the issue is routed to clarification instead of specification drafting

  Rule: Missing artifact data is explicit

    Scenario: Product spec artifact is missing for Stage 7 approval
      Given an open DUUMBI issue has no Stage 6 Product Spec Draft comment
      When the Stage Approval workflow records Stage 7 with decision "approve"
      Then the Stage 8 prompt is still generated
      And the product spec artifact field says `<missing: product spec artifact>`
      And no unverified product spec URL is invented

    Scenario: Technical spec artifact is missing for Stage 9 approval
      Given an open DUUMBI issue has no Stage 8 Technical Spec Draft comment
      When the Stage Approval workflow records Stage 9 with decision "approve"
      Then the Stage 10 prompt is still generated
      And the technical spec artifact field says `<missing: technical spec artifact>`
      And no unverified technical spec URL is invented

  Rule: GitHub remains a durable fallback

    Scenario: Slack is unavailable
      Given Slack credentials are not configured for the Stage Approval workflow
      When an approval decision records successfully
      Then the next-stage prompt is still available from a GitHub surface
      And the workflow does not fail only because Slack could not receive the prompt

    Scenario: Workflow summary contains the prompt
      Given the Stage Approval workflow records an approval decision
      When the workflow summary is written
      Then the summary includes the same next-stage prompt content needed to run Codex
      And the prompt identifies missing artifacts explicitly when present

  Rule: Spec-only safety language is preserved

    Scenario: Generated Stage 6 prompt avoids issue-closing language
      Given Stage 5 approval is recorded for issue 593
      When the Stage 6 prompt is generated
      Then the prompt references the issue using non-closing language
      And the prompt includes a workflow note that the spec PR must not close the execution issue

    Scenario: Generated Stage 8 prompt avoids issue-closing language
      Given Stage 7 approval is recorded for issue 593
      When the Stage 8 prompt is generated
      Then the prompt references the issue using non-closing language
      And the prompt includes a workflow note that the technical spec PR must not close the execution issue

## Tasks

1. Verify the current Stage Approval paths for Stage 5, Stage 7, and Stage 9.
2. Define a small prompt-generation helper or data structure inside
   `stage-approval.yml` that maps approval stages to next-stage prompt content.
3. Reuse existing artifact comment lookup for Stage 7 and Stage 9, tightening it
   only as needed for explicit missing-artifact output.
4. Add the generated prompt to the Slack summary when Slack output is attempted.
5. Add the generated prompt to the GitHub decision comment or a linked durable
   GitHub fallback comment.
6. Add the generated prompt to the workflow summary.
7. Add focused validation for the prompt builder and artifact-missing behavior.
8. Run static workflow checks and, if practical, a controlled
   `workflow_dispatch` dry-run or safe test issue path.
9. Record review evidence showing that existing approval routing still works.

Tasks that can run independently:

- Prompt content mapping can be designed independently of Slack output wiring.
- Artifact placeholder handling can be tested independently of Slack delivery.
- GitHub summary/comment output can be validated without live Slack credentials.

## Checks

- Stage 5 approval path produces Stage 6 prompt content with:
  - `duumbi-spec-draft`
  - target issue URL
  - product spec goal
  - BDD requirement
  - spec-only PR warning
  - no technical spec or implementation authorization
- Stage 7 approval path produces Stage 8 prompt content with:
  - `duumbi-tech-spec-draft`
  - target issue URL
  - product spec artifact URL or explicit placeholder
  - technical spec goal
  - BDD-to-test and live E2E plan expectation
  - spec-only PR warning
- Stage 9 approval path produces Stage 10 prompt content with:
  - `duumbi-implementation`
  - target issue URL
  - product spec artifact URL or explicit placeholder
  - technical spec artifact URL or explicit placeholder
  - Ralph Cycle resource boundary
- Non-approval decisions do not include next-stage prompts.
- Existing label transitions remain unchanged.
- Existing Project V2 update behavior remains unchanged.
- Existing Slack button payload and `slack-approval-bridge` dispatch behavior
  remain compatible.
- Workflow summary contains the prompt for GitHub-only fallback.
- Decision comment or linked GitHub fallback contains the prompt.
- Static checks for workflow syntax and script logic pass.
- If a live workflow test is used, evidence links the run and confirms the issue
  was safe to mutate.
- Review evidence confirms no product spec approval, technical spec creation,
  implementation, or Ralph-cycle work happened during this Stage 6 spec PR.

## Open Questions

- Should the generated prompt be embedded directly in the existing decision
  comment, or posted as a separate "Next Codex Prompt" comment linked from the
  decision comment to keep decision records compact?
- Is there currently a supported Codex launch URL that can prefill a local thread
  safely, or should this remain copy-ready text only for now?
- Should Stage 9 point to Stage 10 implementation coordination only, as specified
  here, or should it offer both coordination and direct Ralph-cycle text when a
  future technical spec explicitly authorizes that shortcut?

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/593
- Stage 5 decision comment:
  https://github.com/hgahub/duumbi/issues/593#issuecomment-4518823242
- Source note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/DUUMBI Pipeline Automation Spec.md`
- Active runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Active workflow map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Stage Approval workflow:
  `.github/workflows/stage-approval.yml`
- Human Acceptance request workflow:
  `.github/workflows/human-acceptance-request.yml`
- Product Spec Review request workflow:
  `.github/workflows/spec-review-request.yml`
- Technical Spec Review request workflow:
  `.github/workflows/technical-spec-review-request.yml`
- Slack approval bridge:
  `scripts/slack-approval-bridge/src/functions/slackApproval.js`
