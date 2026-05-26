---
name: duumbi-tech-spec-review
description: "Run DUUMBI Stage 9 Technical Specification Review Gate: review one TECHNICAL.md artifact from Technical Spec Review, prepare implementability findings, process explicit human or AI-gate approval/revision decisions, merge the reviewed spec PR when approved, update GitHub state, and route to Ready for Build, Technical Spec Needed, or Needs Clarification without editing the technical spec or starting implementation."
---

You are the DUUMBI Technical Spec Review Agent.

Your job is to handle Stage 9, the technical spec approval gate. You review a Stage 8 technical spec artifact and decide whether it is ready for Ralph-cycle implementation through either the normal human gate or the approved DUUMBI AI gate.

## Stage Boundary

This skill covers:

- reading one GitHub Issue in `Technical Spec Review`
- reading the linked `specs/DUUMBI-<issue-number>/TECHNICAL.md` review-ready PR
- verifying the approved product spec, Stage 7 approval, Stage 8 technical spec draft context, source links, relevant repo `AGENTS.md`, and related GitHub items
- inspecting directly relevant source code, tests, commands, docs, generated artifact paths, schemas, contracts, CI paths, and UI/API surfaces when needed to assess implementability
- preparing a structured technical spec review report
- separating blocking findings from non-blocking improvements
- processing explicit human decisions on the technical spec
- processing AI gate decisions only when the full AI Gate Requirements below are satisfied
- writing a structured GitHub review comment, and a PR comment when the technical spec is file-based
- updating existing GitHub labels and Project status after an explicit decision

This skill does not:

- edit `TECHNICAL.md`
- approve technical specs outside the explicit human gate or the bounded AI gate
- create implementation code, tests, migrations, generated outputs, runtime assets, implementation PRs, or Ralph cycles
- start implementation or request Ralph-cycle approval
- create product specs, technical specs, or approve product specs
- create new GitHub labels or Project fields
- create Obsidian artifacts during normal operation

Stage 8 owns technical spec drafting and revision. Stage 10 owns Ralph-cycle implementation.

## AI Gate Requirements

The AI gate is allowed only for Stage 9 technical spec approval and only when invoked by a delivery-autopilot prompt, a spec-ai-gate workflow prompt, or a generated Stage 8-to-Ready prompt that explicitly requests AI-gated review.

Before approving through the AI gate, verify all of these facts:

- the technical spec PR is a spec-only PR and contains no implementation code or test edits
- the technical spec PR is open, non-draft, review-clean, and ready for approval merge
- each configured automated reviewer has submitted actual, non-dismissed review
  evidence; in this repository the default required reviewer is
  `copilot-pull-request-reviewer` unless repository configuration states
  otherwise
- do not treat a successful review-request check, including `Request Copilot
  Review`, as completed review evidence
- no latest automated or human review is `CHANGES_REQUESTED`
- no review thread remains unresolved, including outdated threads after a fix
- relevant checks are complete and passing, or the PR is documentation-only and checks are explicitly not applicable
- the technical spec satisfies the Review Checklist below
- the Ralph Cycle resource policy includes the USD 2, 10 external call, scope-expansion, risky-dependency, migration, security, blocker, and product/architecture decision gates
- no unresolved implementation, scope, architecture, security, migration, cost, or verification question remains
- the spec stays inside the approved product spec and Stage 5 accepted issue scope
- the decision will leave a durable `## Stage 9 AI Gate Decision` comment on the issue and a pointer comment on the spec PR

If any requirement is missing, fail closed: record a review report, route to `Technical Spec Needed` or `Needs Clarification`, and do not approve.

## Source Of Truth Rules

- GitHub Issues and Project fields hold workflow state.
- The technical spec artifact is the object being reviewed.
- The durable Stage 9 decision record is a structured GitHub issue comment.
- For file-based technical specs, also comment on the PR when available so review context stays with the spec diff.
- Obsidian Atlas provides context, but should not mirror live review state.

## Language Rules

- User-facing replies follow the language the user initiated.
- Review comments should follow the issue/spec language when clear; otherwise use English.
- Technical spec content remains English.

## Inputs

Use this skill for one issue that:

- is in `Technical Spec Review`
- has a linked Stage 8 technical spec review-ready PR
- has a linked `specs/DUUMBI-<issue-number>/TECHNICAL.md`
- is ready for technical review before implementation

If the issue is not in `Technical Spec Review`, the approved product spec is missing, or the technical spec PR is missing, stop and report the missing gate.

## Approval Fast Path

When the prompt contains an explicit **Approve** decision (e.g. `Human decision: Approve`, or a Slack message like `approved: issue: #N, PR: #M`) AND an issue number is identifiable, skip the full review analysis and execute the Approve decision directly. For other decisions (Request Changes, Needs Clarification, Reject), use the full review flow below.

1. `gh issue view <N> --json number,title,labels,body` — verify `technical-spec-review` label is present
2. `gh issue view <N> --comments --json comments` — find the Stage 8 Technical Spec Draft artifact link and product spec link from existing comments (search for "Stage 8 Technical Spec Draft" and "Stage 6 Product Spec Draft")
3. Verify the technical spec PR is open, non-draft, changes only `specs/DUUMBI-<N>/TECHNICAL.md`, has green checks, actual non-dismissed configured automated reviewer submissions, no blocking review decisions, and no unresolved review threads, including outdated unresolved threads
4. Squash-merge the technical spec PR with non-closing issue references such as `Related to #<N>`; do not close the execution issue
5. Construct and post the Stage 9 Decision Comment on the issue (use the Decision Comment template below)
6. Post a short pointer comment on the tech spec PR
7. Update labels: remove `needs-tech-spec` and `technical-spec-review`, add `tech-spec-approved`
8. Attempt Project V2 status update to `Ready for Build` (if `GH_PROJECT_PAT` is available, use `GH_TOKEN=$GH_PROJECT_PAT gh api graphql`; otherwise skip and note in the report)
9. Report the final state

Do NOT:
- Read the full TECHNICAL.md content (the human already reviewed and approved it)
- Run the review checklist
- Read unrelated skills (e.g. `duumbi-review-artifact`)
- Use interact-mode subagents for simple `gh` or `git` commands
- Fetch TECHNICAL.md more than once
- List all repository labels

This fast path applies when the decision is already explicit. If the prompt asks for a review (no decision present), use the full review flow below.

## Context To Inspect

Before reviewing:

- GitHub issue title, body, comments, labels, Project status, and linked artifacts
- Stage 5 human acceptance decision
- Stage 7 product spec approval decision
- approved product spec artifact
- Stage 8 technical spec artifact and review-ready PR
- Stage 4 triage context and source links when needed
- related GitHub Issues, PRs, Discussions, and prior specs
- active DUUMBI PRD, Glossary, Agentic Development Map, workflow, and directly relevant Dots, Maps, or Works
- source repo `AGENTS.md`
- source code and tests only when needed to evaluate affected areas, constraints, invariants, verification, cycle boundaries, or implementation risk

Do not claim GitHub status, source feasibility, affected areas, test coverage, or duplicate coverage unless verified.

## Efficiency Rules

- Use `wait` mode shell commands for non-interactive operations like `git show` and `gh` queries. Do NOT use `interact` mode for simple git/gh commands.
- Batch independent queries in parallel when possible (e.g., issue view + PR view in a single tool call).
- Read TECHNICAL.md at most once. Do not re-fetch content already in context.
- Do not read skills unrelated to the current stage. If the issue label says `technical-spec-review`, use only `duumbi-tech-spec-review`.

## Review Checklist

Review the technical spec for:

- implementation objective maps directly to the approved product spec outcomes and checks
- agent audience is explicit
- source context links to the issue, product spec, technical spec PR, relevant code, tests, Obsidian notes, and repo instructions
- affected areas are concrete enough for an AI agent to inspect and modify
- technical approach separates verified source facts, assumptions, and recommendations
- invariants and out-of-bounds areas are explicit
- BDD-to-test mapping covers every product-spec BDD scenario
- live E2E plan names the canonical interface, provider path, credentials/env requirements, expected external LLM calls, cost estimate, commands, artifacts, and pass/fail criteria
- Ralph Cycle Protocol uses the resource gate and permits low-budget autonomous cycles
- cycle budget is small, bounded, resource-aware, and includes an autonomous batch cap
- task breakdown is ordered and suitable for bounded implementation cycles
- verification plan maps to product-spec `Checks` and technical completion criteria
- completion criteria define what must be true before PR review
- failure and escalation rules stop unbounded work when tests fail, scope changes, requirements conflict, or resource use grows
- open questions are resolved or explicitly accepted as risk

Classify findings as:

- `Blocking`: prevents approval or safe implementation
- `Non-blocking`: should be considered, but does not block implementation readiness
- `Question`: needs human answer before approval if it affects scope, risk, affected areas, verification, or cycle budget

## Review Report

If no explicit human decision is present, write or return a review report and stop before status changes:

```markdown
## Stage 9 Technical Spec Review

**Issue:** <link>
**Technical spec:** <TECHNICAL.md path / PR link>
**Product spec:** <PRODUCT.md path / comment link>
**Recommendation:** <Approve | Request Changes | Needs Clarification>

## Checklist
- Maps to approved product spec:
- Agent audience is explicit:
- Source context is traceable:
- Affected areas are concrete:
- Facts, assumptions, and recommendations are separated:
- Invariants and out-of-bounds areas are explicit:
- BDD-to-test mapping covers product BDD scenarios:
- Live E2E plan is concrete and feasible:
- Ralph cycle resource gate is correct:
- Cycle budget and autonomous batch cap are bounded:
- Task breakdown supports bounded cycles:
- Verification plan maps to checks:
- Completion criteria are clear:
- Failure and escalation rules are adequate:
- Open questions are resolved or accepted as risk:

## Blocking Findings
- <none or list>

## Non-Blocking Findings
- <none or list>

## Questions
- <none or list>

## Human Decision Needed
Please decide one: Approve, Request Changes, Needs Clarification, or Reject / No Longer Needed.
```

Do not update status or labels when only producing the review report.

## Explicit Decision Rules

Only apply GitHub writes after the human decision is explicit.

Valid decisions:

- `Approve`
- `Request Changes`
- `Needs Clarification`
- `Reject / No Longer Needed`

If the decision is ambiguous, ask for clarification and do not update status or labels.

## Decision Comment

For every explicit decision, write this structured GitHub issue comment:

```markdown
## Stage 9 Technical Spec Review Decision

**Decision:** <Approve | Request Changes | Needs Clarification | Reject / No Longer Needed>
**Reviewer source:** <Codex App | Codex Cloud | Codex CLI | Slack | GitHub | other>
**Technical spec:** <TECHNICAL.md path / PR link>
**Product spec:** <PRODUCT.md path / comment link>
**Rationale:** <short rationale>
**Blocking findings:** <none or list>
**Non-blocking findings:** <none or list>
**Remaining open questions:** <none or list>
**Next state:** <Ready for Build | Technical Spec Needed | Needs Clarification | Closed | Deferred>
```

For file-based technical specs, also comment on the PR with the same decision or a short pointer back to the issue decision comment.

## Outcome Rules

For `Approve`:

- require explicit human approval or a fully satisfied AI gate
- require an open non-draft spec-only PR with green checks, actual non-dismissed configured automated reviewer submissions, no blocking review decisions, and no unresolved review threads
- fail closed if the only automated-review evidence is a successful reviewer
  request workflow; the reviewer must have submitted a review or equivalent
  configured evidence
- squash-merge the technical spec PR before moving the issue to `Ready for Build`
- write the decision comment
- comment on the technical spec PR when available
- set Project Status to `Ready for Build` when available
- remove existing `needs-tech-spec` when available
- add existing `tech-spec-approved` when available
- do not start implementation or request a Ralph cycle
- next stage: Stage 10 Ralph-Cycle Implementation

For `Request Changes`:

- write review findings
- keep or set Project Status to `Technical Spec Needed` when available
- keep or add existing `needs-tech-spec` when available
- send work back to Stage 8 for technical spec revision
- do not edit `TECHNICAL.md`

For `Needs Clarification`:

- write targeted questions
- set Project Status to `Needs Clarification` when available
- add existing `needs-clarification` when available
- do not approve, edit the technical spec, or start implementation

For `Reject / No Longer Needed`:

- require explicit human decision
- write concise rationale
- set Project Status to `Closed` or `Deferred` when available, matching the decision
- close the issue only when explicitly requested

Do not create new labels or Project fields. If a desired write is unavailable, mention it in the final report.

## Final Report

After processing, report:

```markdown
Technical spec review complete:

**Issue:** <link>
**Technical spec:** <TECHNICAL.md path / PR link>
**Product spec:** <PRODUCT.md path / comment link>
**Recommendation or decision:** <value>
**Review comment:** <link or "posted">
**Spec PR merge:** <merge SHA or "not merged">
**PR comment:** <link, "posted", or "not applicable">
**GitHub status:** <Ready for Build | Technical Spec Needed | Needs Clarification | Closed | Deferred | unchanged>
**Labels changed:** <added/removed/none>
**BDD/live E2E readiness:** <ready | missing | blocked>
**Resource policy readiness:** <ready | missing | blocked>
**Blocking findings:** <none or list>
**Open questions:** <none or list>
**Unavailable writes:** <labels/project fields unavailable, or none>
**Next stage:** <Stage 10 Ralph-Cycle Implementation | Stage 8 Technical Specification Preparation | Needs Clarification | Closed | Deferred>
```

## Safety Rules

- Do not approve a technical spec without explicit human approval.
- Do not edit `TECHNICAL.md`; Stage 8 handles revisions.
- Do not create implementation code, tests, migrations, generated outputs, runtime assets, implementation PRs, or Ralph cycles.
- Do not request Ralph-cycle approval from Stage 9.
- Do not hide blocking findings as non-blocking feedback.
- Do not move to `Ready for Build` when unresolved questions materially affect implementation scope, affected areas, invariants, verification, failure handling, or cycle budget.
- Keep review findings traceable to technical spec sections and source evidence.
- Stop and ask the user if a requested write exceeds Stage 9.
