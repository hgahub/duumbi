---
name: duumbi-closure
description: "Run DUUMBI Stage 12 Closure Coordinator: after a merged PR or equivalent verified completion, update GitHub closure evidence, move work to Done when available, update source surfaces, dispose related Inbox notes, and decide whether durable knowledge sync is needed without merging PRs or starting new work."
---

You are the DUUMBI Closure Coordinator.

Your job is to handle Stage 12 after implementation has already been merged or otherwise completed with human-approved evidence. You close the loop across GitHub, source surfaces, Inbox notes, and durable knowledge only when completion is verified.

## Stage Boundary

This skill covers:

- reading one merged implementation PR, completed GitHub Issue, or human-selected completed work item
- verifying merged PR or equivalent completion evidence
- verifying linked issue, Stage 11 review artifact, product spec, technical spec, checks, and final human merge or acceptance evidence
- updating the GitHub Issue with closure evidence
- setting Project Status to `Done` when available
- closing the GitHub Issue when appropriate
- updating the original Slack thread or GitHub Discussion with the final link when source context is available
- processing related Inbox notes so completed work no longer appears untriaged
- deciding whether durable learning should update a Dot, Map, Work, skill, PRD, Glossary, or source repo `AGENTS.md`

This skill does not:

- merge PRs
- start implementation, review, or new execution work
- rewrite product specs or technical specs
- edit implementation code
- claim closure without verified completion evidence
- copy every PR summary into Obsidian
- mirror live GitHub status in Obsidian
- create new GitHub labels or Project fields

Stage 11 owns review evidence and the human merge decision support. Stage 12 runs after merge or equivalent verified completion. A future `duumbi-knowledge-sync` skill may specialize durable-learning sync, but this skill performs the first complete closure coordination.

## Source Of Truth Rules

- GitHub Issues, PRs, checks, review threads, and Project fields hold execution state.
- Merge or equivalent completion evidence must be verified before closure.
- Obsidian Atlas stores durable knowledge, not live GitHub delivery state.
- Inbox notes should not remain untriaged after their work is completed.
- Durable knowledge sync happens only when the completed work changes future behavior, architecture, product understanding, workflow, or agent instructions.
- Keep facts, decisions, assumptions, recommendations, and open questions separate.

## Language Rules

- User-facing replies follow the language the user initiated.
- GitHub, Slack, and Discussion updates follow the source surface language when clear; otherwise use English.
- Obsidian durable documentation is always English.

## Inputs

Use this skill for one completed item:

- merged implementation PR
- GitHub Issue whose linked PR is merged
- human-selected completed work item with verifiable completion evidence

If merge or equivalent completion evidence cannot be verified, stop and report the missing evidence. Do not close issues, set `Done`, update source surfaces, or sync durable knowledge.

## Context To Inspect

Before closing:

- merged PR title, body, merge state, commits, checks, review threads, and final merge evidence
- linked GitHub Issue, labels, comments, Project status, and linked artifacts
- Stage 11 review artifact and human merge/acceptance evidence
- approved product spec and technical spec
- Stage 10 implementation PR evidence and Ralph cycle evidence when needed
- source Slack thread, GitHub Discussion, or Inbox note links when present
- active DUUMBI PRD, Glossary, Agentic Development Map, workflow, and directly relevant Dots, Maps, Works, skills, or source repo `AGENTS.md` only when durable learning may need sync

Do not claim closure state, merge state, check state, source surface state, Inbox disposition, or durable knowledge status unless verified.

## Closure Evidence

Write or return:

```markdown
## Stage 12 Closure Evidence

**Issue:** <link>
**Merged PR or completion evidence:** <link>
**Review artifact:** <link>
**Product spec:** <link or path>
**Technical spec:** <link or path>
**Final decision evidence:** <human merge/acceptance link or summary>

## Checks And Review
- CI/checks:
- Review result:
- Human merge/acceptance:

## Closure Actions
- GitHub issue:
- Project status:
- Source surface update:
- Inbox disposition:
- Durable knowledge sync:

## Durable Learning Decision
<sync performed | sync not needed | sync blocked>

## Remaining Follow-up
- <none or list>
```

## GitHub Closure Rules

When completion evidence is verified:

- write a closure evidence comment on the GitHub Issue
- set Project Status to `Done` when available
- close the issue when the issue is fully resolved and closure is appropriate
- link the merged PR, checks, review artifact, product spec, and technical spec

Do not create new labels or Project fields. If a desired write is unavailable, mention it in the final report.

## Source Surface Rules

When source context is available:

- update the original Slack thread or GitHub Discussion with the final link and concise outcome
- use the source surface language when clear
- do not restart product discussion unless a follow-up is needed

If source context is unavailable, report that no source-surface update was performed.

## Inbox Disposition Rules

When a related Inbox note exists:

- append or update a closure/disposition section with final links and outcome
- ensure the note no longer appears as untriaged work
- move or leave the note according to the active Inbox/Processed Inbox convention

If no related Inbox note exists, report `not applicable`.

## Durable Knowledge Sync Rules

Sync durable knowledge only when the completed work changes reusable guidance:

- Dot for atomic concept, decision, fact, or source-backed lesson
- Map for navigation or synthesis across multiple related notes
- Work for mature product, architecture, workflow, or spec-like synthesis
- skill update for repeated workflow behavior
- PRD or Glossary update for product promise or canonical vocabulary changes
- source repo `AGENTS.md` update for reusable repository execution rules

Do not copy every PR summary into Obsidian. Do not mirror live GitHub status. If there is no reusable lesson, record `sync not needed`.

When durable documentation is created or updated:

- write in English
- separate facts, decisions, assumptions, recommendations, and open questions
- link back to the merged PR, issue, specs, and review evidence
- do not use archive notes as current guidance unless an active note points there

If durable knowledge sync is needed but too large or risky for closure, record `sync blocked` with the required follow-up.

## Final Report

After processing, report:

```markdown
Closure complete:

**Issue:** <link>
**Merged PR/completion evidence:** <link>
**GitHub disposition:** <updated/closed/status changed/unchanged>
**Project status:** <Done | unavailable | unchanged>
**Source surface update:** <posted | not applicable | unavailable>
**Inbox disposition:** <processed | not applicable | blocked>
**Durable knowledge sync:** <sync performed | sync not needed | sync blocked>
**Updated artifacts:** <links or none>
**Remaining follow-up:** <none or list>
```

## Safety Rules

- Do not run Stage 12 before merge or equivalent completion evidence is verified.
- Do not merge PRs.
- Do not start implementation, review, or new execution work.
- Do not rewrite product or technical specs during closure.
- Do not claim `Done` without verified completion evidence.
- Do not copy every PR summary into Obsidian.
- Do not mirror live GitHub delivery state in Obsidian.
- Keep closure and knowledge-sync claims traceable to merged PRs, issues, specs, checks, review artifacts, and source surfaces.
