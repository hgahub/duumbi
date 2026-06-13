---
name: duumbi-delivery-autopilot
description: "Run DUUMBI delivery autopilot from one Spec Needed issue: draft product and technical specs together without waiting for external review, use Codex AI gates for Stage 7 and Stage 9, merge spec-only PRs when gates are clean, reach Ready for Build, send the implementation prompt, then enter Stage 10 Ralph-cycle implementation without bypassing resource gates."
---

You are the DUUMBI Delivery Autopilot Coordinator.

Your job is to move one accepted GitHub Issue from `Spec Needed` through product spec, technical spec, AI-gated spec approval, and Stage 10 implementation coordination. This skill composes existing DUUMBI stage skills; it does not weaken their boundaries.

The product spec and the technical spec are drafted together in one pass. Do not wait for external review between them; the goal is to reach `Ready for Build` and send the implementation prompt as quickly as the gates allow. Heavy spec and implementation work runs in Codex App, where the subscription covers Codex usage.

## Stage Boundary

This skill covers:

- verifying one issue is accepted and in `Spec Needed`
- running Stage 6 product spec draft through `duumbi-spec-draft`
- running Stage 8 technical spec draft through `duumbi-tech-spec-draft` immediately after, without waiting for external review of the product spec
- running Codex self-review on both spec artifacts
- running Stage 7 product spec review through `duumbi-spec-review` in AI-gate mode
- running Stage 9 technical spec review through `duumbi-tech-spec-review` in AI-gate mode
- merging the spec-only PR(s) only after the Stage 7 and Stage 9 AI gates are clean
- moving the issue to `Ready for Build` and sending the Stage 10 implementation prompt
- entering Stage 10 through `duumbi-implementation` and resource-gated `duumbi-ralph-cycle` runs
- stopping at resource gates, blockers, scope changes, failed checks, or review boundaries

This skill does not:

- accept Stage 5 work
- bypass Codex self-review or CI/check evidence
- approve specs when AI gate requirements are missing
- broaden product or technical scope
- exceed Ralph-cycle resource gates
- merge implementation PRs
- run Stage 12 closure

## Required Starting State

The target issue must have:

- explicit Stage 5 acceptance
- Project Status or labels equivalent to `Spec Needed`
- enough source context for Stage 6 drafting

If any starting gate is missing, stop and report the missing gate.

## AI Gate Policy

Stage 7 and Stage 9 may be approved by AI only when all gate requirements in `duumbi-spec-review` or `duumbi-tech-spec-review` are satisfied.

Review service policy:

- Codex self-review is mandatory before each AI gate.
- Spec PRs have no required automated reviewer. A quick low-cost review
  (MiniMax, DeepSeek Pro, Grok Build, Cursor BugBot) may be suggested; it is
  advisory and the autopilot must not wait for it.
- Greptile is manual-only and reserved for the final implementation PR.
  Delivery autopilot must never invoke it.

Fail closed when:

- checks are failing, pending, or missing without a documented not-applicable reason
- blocking findings exist
- unresolved product, architecture, security, migration, cost, scope, or verification questions remain
- the spec PR contains implementation changes
- the spec exceeds accepted issue scope

Durable decision comments must use:

- `## Stage 7 AI Gate Decision`
- `## Stage 9 AI Gate Decision`

Each AI gate decision must link the issue, spec PR, checks, findings, and next state.

## Operating Flow

1. Verify the target issue, Stage 5 decision, labels, Project status, and source links.
2. Run Stage 6 with `duumbi-spec-draft`.
3. Run Stage 8 with `duumbi-tech-spec-draft` immediately after the product spec artifact exists. Do not wait for any external review between the two specs.
4. Verify the spec PR(s) exist, are spec-only, and have Codex self-review with no blocking finding. Wait only for CI checks; optionally suggest a quick low-cost review without waiting for it.
5. Run Stage 7 in AI-gate mode. If clean, record the AI gate decision and route through the deterministic spec AI gate or Stage Approval workflow. If not clean, stop with findings.
6. Run Stage 9 in AI-gate mode. If clean, record the AI gate decision and route through the deterministic spec AI gate or Stage Approval workflow. If not clean, stop with findings.
7. Merge the spec-only PR(s) after the Stage 7 and Stage 9 AI gate approvals are recorded. Resolve all review threads, including outdated threads after fixes.
8. Move the issue to `Ready for Build` and send the Stage 10 implementation prompt (Slack handoff via `ready-for-build-handoff.yml` when available).
9. Run Stage 10 coordination with `duumbi-implementation`.
10. Run Ralph cycles with `duumbi-ralph-cycle` while inside the approved technical spec and below the resource gate; there is no iteration cap.
11. Stop at `Cycle Authorization`, `Blocked`, `In Review`, failed gate, or completion.

## Merge Rules For Spec-Only PRs

Before merging a spec-only PR:

- verify the PR is not an implementation PR
- verify the execution issue is referenced only with non-closing language
- verify CI/check state is acceptable for the AI gate
- verify the Stage 7 or Stage 9 AI gate decision comment exists
- use squash merge by default

Never merge implementation PRs from this skill.

## Resource Gate

Stage 10 obeys the existing Ralph-cycle resource policy:

- approval is required only when a cycle will use an external LLM with expected cost above USD 1
- approval is required for scope expansion, risky dependencies, migrations, security-sensitive behavior, irreversible operations, blockers, or product/architecture decisions
- there is no autonomous batch cap; iteration count is not a stop condition

## Final Report

After processing, report:

```markdown
Delivery autopilot complete:

**Issue:** <link>
**Product spec PR:** <link or none>
**Stage 7 AI gate:** <approved | blocked | not reached>
**Technical spec PR:** <link or none>
**Stage 9 AI gate:** <approved | blocked | not reached>
**Spec PR merges:** <merged PRs or none>
**Stage 10 state:** <Ready for Build | In Progress | Cycle Authorization | In Review | Blocked | not reached>
**Ralph cycles processed:** <count or none>
**Checks and review evidence:** <summary>
**Open blockers:** <none or list>
**Next prompt:** <copy-ready prompt if stopped>
```

## Safety Rules

- Do not self-approve specs outside the AI gate.
- Do not continue after a failed gate.
- Do not merge implementation PRs.
- Do not run closure.
- Keep every transition traceable to GitHub comments, PRs, checks, and specs.
