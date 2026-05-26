# DUUMBI-376: Discord Server Setup For Community Discussions And Support - Technical Specification

## Implementation Objective

Implement the approved product outcomes for issue #376 by creating and
launching the official public DUUMBI Discord community surface, adding official
invite discovery from public DUUMBI entry points, and recording enough review
evidence for implementation reviewers to verify the live configuration.

This is a community and documentation launch task. It must not change DUUMBI
compiler, CLI, Studio, registry, MCP, runtime, generated artifacts, product
specs, or execution workflow behavior.

Technical spec for #376. This specification is non-closing and the execution
issue must remain open for Stage 9 review and Stage 10 implementation.

## Agent Audience

Use this spec for:

- Codex App or Codex CLI agents coordinating Stage 10 Ralph-cycle implementation.
- Codex Cloud agents making bounded repository documentation edits.
- Human maintainers who must configure or verify Discord server settings through
  the Discord UI.
- Specialized reviewers checking evidence, permissions, public links, and
  no-secret/no-private-data constraints.

Do not use this spec to start implementation during Stage 8 or Stage 9.

## Source Context

- Product spec: `specs/DUUMBI-376/PRODUCT.md`.
- Product spec PR: `https://github.com/hgahub/duumbi/pull/634`.
- GitHub issue: `https://github.com/hgahub/duumbi/issues/376`.
- Stage 4 triage: issue comment
  `https://github.com/hgahub/duumbi/issues/376#issuecomment-4538613990`.
- Stage 5 acceptance: issue comment
  `https://github.com/hgahub/duumbi/issues/376#issuecomment-4548790938`.
- Stage 7 product spec approval: issue comment
  `https://github.com/hgahub/duumbi/issues/376#issuecomment-4549138795`.
- Relevant code: none. This issue does not require Rust, runtime, Studio, CLI,
  registry, MCP, parser, graph, compiler, or test harness changes.
- Relevant repository docs:
  - `README.md`: has a `Community` section with Code of Conduct guidance and no
    Discord invite at Stage 8 inspection time.
  - `CODE_OF_CONDUCT.md`: applies to all community spaces and names maintainers
    as enforcement owners.
  - `sites/docs/src/introduction.md`: docs landing content for
    `https://docs.duumbi.dev/`, including current GitHub/help pointers.
  - `sites/docs/src/contributing.md`: docs contributor page, currently minimal.
  - `sites/docs/book.toml`: verifies the docs site URL is
    `https://docs.duumbi.dev/`.
- Relevant tests and CI:
  - `.github/workflows/ci.yml`: docs/spec-only changes avoid Rust checks unless
    Rust-relevant paths change.
  - `.github/workflows/docs-review.yml`: triggers on Rust source and Cargo
    changes, not on README, `sites/docs`, or `specs` changes.
  - No existing automated test covers external Discord configuration.
- Relevant Obsidian notes from the approved product spec:
  - Active PRD.
  - Active Glossary.
  - Agentic Development Map.
  - Open Source Monetization Model B.
  - Archived Phase 14 Marketing & Go-to-Market roadmap.
- Repo instructions: `AGENTS.md` in this repository.

## Affected Areas

Expected Stage 10 implementation changes and evidence:

- External Discord configuration, outside this repository:
  - Official DUUMBI server under maintainer control.
  - Public channels: `#general`, `#help`, `#showcase`, `#development`, and
    `#announcements`.
  - Roles: `Maintainer`, `Contributor`, and `Community`.
  - Channel topics, pinned messages, server guide, onboarding prompts, or
    equivalent first-user guidance.
  - Welcome/onboarding automation.
  - Moderation automation and maintainer-visible report or moderation handling.
  - Stable official invite URL.
- Repository documentation:
  - `README.md`: add the official Discord invite to the public community or
    onboarding section.
  - `sites/docs/src/introduction.md` and/or `sites/docs/src/contributing.md`:
    add a docs-visible community/help pointer when useful, without claiming that
    docs are the `duumbi.dev` landing site.
- Website surface:
  - `duumbi.dev`: add the official Discord invite if the owning source or
    deployment path is available during implementation.
  - If unavailable, record evidence identifying the unavailable source or owning
    repository and create or link the follow-up path needed to update
    `duumbi.dev`.
- Review evidence:
  - Screenshots, exported non-sensitive settings, PR comments, or checklist
    notes showing live server structure, permissions, welcome flow, moderation,
    invite validity, README link, and website link or follow-up.

Areas that must not change:

- Rust source, Cargo files, generated outputs, runtime assets, product specs,
  compiler behavior, CLI behavior, Studio behavior, registry behavior, and
  persistent delivery workflow semantics.

## Technical Approach

Facts:

- The approved product spec requires `#help`, not `#support`; use the accepted
  issue and product spec as the source of truth.
- The repository contains README and docs source, but no verified source for the
  `duumbi.dev` landing page.
- Discord server configuration is live external state and cannot be fully
  represented by repository files.
- `CODE_OF_CONDUCT.md` already governs project community spaces.

Assumptions:

- A standard Discord welcome/onboarding feature or reputable moderation/welcome
  bot is sufficient.
- The implementation agent has access to a maintainer Discord account or can
  coordinate with a human maintainer who has that access.
- Public invite links can be stable Discord invites and must not assign elevated
  permissions by themselves.

Recommended implementation strategy:

1. Create or identify the official DUUMBI Discord server under maintainer
   control.
2. Configure channels, channel topics, role names, and permissions before
   publishing the invite.
3. Configure onboarding so required links are visible even when direct messages
   fail.
4. Configure moderation with standard Discord safety settings and/or a reputable
   moderation bot. Avoid custom bot development unless a concrete product gap is
   found and approved.
5. Generate the stable official invite and verify it from a non-admin context.
6. Add the invite to `README.md` and any appropriate docs source in this repo.
7. Identify the `duumbi.dev` publishing source. Update it when available; when
   unavailable, create or link a follow-up and include evidence in the
   implementation PR.
8. Collect non-sensitive screenshots or notes for review.

Rejected alternatives:

- Do not build a custom Discord bot by default; it increases scope, security
  review, hosting, token management, and maintenance burden without evidence
  that standard tools are insufficient.
- Do not use Discord as the durable source of truth for roadmap, issue
  acceptance, PR review, or project status.
- Do not defer README discovery until after server launch; public entry-point
  verification is part of the accepted product outcome.

## Invariants

- The Discord server remains governed by the project Code of Conduct.
- Community members do not receive administrative permissions by default.
- The official invite must not grant elevated roles or permissions.
- `#announcements` remains writable only by Maintainers or an explicitly
  maintainer-controlled integration.
- `#help` is the support channel name. Do not substitute `#support`.
- Discord-originated product or implementation decisions must be captured in
  GitHub or the DUUMBI intake workflow before being treated as accepted work.
- No Discord bot token, API token, private invite management token, private
  moderation log, private user data, or screenshot containing sensitive user
  information may be committed.
- Stage 10 implementation must stay within the approved product spec. Any
  governance program, support SLA, paid support, contributor ladder, or custom
  bot proposal requires separate approval.

## BDD-To-Test Mapping

| Product BDD scenario | Verification evidence |
| --- | --- |
| Reader joins from the GitHub README | Repository diff shows `README.md` contains the official invite. Manual browser review opens the link and shows the official DUUMBI server join flow. Reviewer evidence confirms the invite does not assign elevated roles. |
| Website visitor joins from `duumbi.dev` | If the source is available, website diff or deployment evidence shows the invite on `duumbi.dev`, and manual browser review opens the same official server or another approved invite for that server. If source is unavailable, the implementation PR links the owning source, follow-up issue, or update path. |
| Website source is unavailable during implementation | PR evidence names the unavailable source or owning repository and links the follow-up/update path. Approval requires this evidence if `duumbi.dev` cannot be changed directly. |
| New member receives onboarding links | Screenshot or exported non-sensitive configuration shows welcome/onboarding copy linking to docs, GitHub, getting-started material, and the Code of Conduct. A normal-member smoke check confirms the links are visible. |
| Direct-message welcome delivery fails | Evidence shows the same onboarding links are also available in a server-visible channel, server guide, pinned message, or equivalent public flow. |
| Member wants support | Channel list screenshot or settings evidence shows `#help`, with topic/pin/onboarding copy that makes support usage clear. |
| Member wants to share a demo | Channel list screenshot or settings evidence shows `#showcase`, with topic/pin/onboarding copy for examples, modules, screenshots, or demos. |
| Regular member joins | Permission review from a normal member or role-permission screenshot shows Community-level access, no administration rights, and no write access to `#announcements`. |
| Maintainer posts an announcement | Maintainer test post or permission evidence shows Maintainers can post in `#announcements`; normal-member evidence shows the announcement is visible but the channel is not writable by non-maintainers. |
| Recognized contributor helps in development discussion | Contributor role permissions show access to `#development` and `#help` without server administration permissions. |
| Spam message is posted | Moderation settings screenshot or bot configuration export shows spam blocking, flagging, quarantine, logging, rate limits, or maintainer surfacing. A live spam test is optional and must avoid abusive public content. |
| Code of Conduct issue is reported | Evidence shows maintainers can receive reports or moderation signals and can take standard Discord moderation actions consistent with `CODE_OF_CONDUCT.md`. |
| Product change proposed in Discord | README/docs/onboarding copy or reviewer evidence states that accepted work must move to GitHub Issues or the DUUMBI intake workflow before execution. |

## Live E2E Plan

Canonical interface: the public community onboarding path, because this issue is
external community infrastructure and documentation discovery, not a CLI or
LLM-backed DUUMBI behavior.

Provider/LLM path:

- No DUUMBI provider or LLM call is required.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Codex internal reasoning usage should be reported qualitatively only; it is
  not part of the DUUMBI external LLM budget.

Credentials and access:

- Maintainer Discord account with permission to create/manage the server,
  channels, roles, invite links, onboarding, and moderation.
- Optional moderation/welcome bot dashboard access if a bot is selected.
- GitHub write access for README/docs changes.
- Access to the `duumbi.dev` source or deployment owner if available.

Commands and checks:

- `rg -n "discord\\.gg|discord\\.com/invite|Discord" README.md sites/docs specs/DUUMBI-376`
  to verify public links and spec references.
- `git diff --name-only` to confirm the implementation PR does not include
  unrelated source changes.
- `git diff --check` to catch whitespace errors.
- `mdbook build sites/docs` if docs source is changed and `mdbook` is available.
  If unavailable, record that the docs build could not run and perform static
  Markdown review.
- Manual browser checks:
  - Open README-rendered invite and verify it reaches the official server.
  - Open `duumbi.dev` invite when updated, or verify the linked follow-up when
    source is unavailable.
  - Join or preview with a non-admin account and verify visible public channels,
    welcome links, and `#announcements` read-only behavior.

Artifacts:

- Implementation PR diff for README/docs changes.
- Non-sensitive screenshots or exported settings for channels, roles,
  onboarding, moderation, invite validity, normal-member permissions, and
  maintainer permissions.
- Link to `duumbi.dev` update, deployment evidence, or follow-up path.

Pass criteria:

- Every BDD scenario has matching automated, manual, or review evidence.
- Public links lead to the official server.
- Normal members cannot administer the server or post in `#announcements`.
- Required onboarding links are visible without relying only on direct messages.
- Moderation behavior is configured and reviewable by maintainers.

Fail criteria:

- Invite is expired, wrong, private when it should be public, or grants elevated
  access.
- Required channels or roles are missing or renamed outside product spec.
- Required onboarding links are visible only through direct messages.
- `duumbi.dev` is neither updated nor covered by an evidence-backed follow-up.
- Secrets, private logs, or personal data are committed.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. Summarize current state and remaining unmet product requirements.
2. Propose one bounded implementation goal.
3. List intended external Discord settings, file areas, and commands.
4. Estimate resource use, external LLM calls, cost, and risk.
5. Check whether the resource gate requires human approval.
6. Implement only the approved or resource-permitted goal.
7. Run the agreed checks and collect required evidence.
8. Report evidence, failures, and remaining gaps.
9. Stop when requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: two repository documentation files, plus
  external Discord settings for one coherent configuration area.
- Expected command budget per cycle: up to six local commands, excluding
  read-only `gh`/`git` inspection.
- Expected external LLM usage: 0 calls, USD 0.
- Human approval is required when planned external LLM usage exceeds USD 2,
  exceeds 10 calls, exceeds approved scope, adds risky dependencies, introduces
  a custom bot or hosted service, requires irreversible Discord ownership
  changes, involves secrets or tokens, needs a security/privacy decision,
  requires a product or architecture decision, or cannot verify `duumbi.dev`
  without external access.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: at most three Ralph cycles before asking for human
  review, even if resource usage remains low.
- Stop and ask for human guidance when Discord access is missing, a selected bot
  cannot satisfy required safeguards, `duumbi.dev` ownership cannot be
  identified, a public copy decision would imply a support SLA or governance
  commitment, or any required evidence would expose private user data.

## Task Breakdown

1. Confirm the implementation branch is based on the merged technical spec and
   that issue #376 is in Ready for Build.
2. Inspect current README/docs community copy and any available `duumbi.dev`
   source or owner information.
3. Create or identify the official DUUMBI Discord server under maintainer
   control.
4. Configure public channels and channel topics/pins for `#general`, `#help`,
   `#showcase`, `#development`, and `#announcements`.
5. Configure `Maintainer`, `Contributor`, and `Community` roles with least
   privilege.
6. Verify `#announcements` is read-only for non-maintainers and writable by
   Maintainers.
7. Configure onboarding/welcome behavior with docs, GitHub, getting-started, and
   Code of Conduct links.
8. Configure server-visible onboarding fallback for users who block direct
   messages.
9. Configure moderation automation and maintainer-visible reporting/action paths.
10. Generate the official invite and verify it from a non-admin or normal-member
    perspective.
11. Add the invite to `README.md` and appropriate docs source if needed.
12. Update `duumbi.dev` if the source is available; otherwise create or link the
    follow-up/update path and record evidence.
13. Run static checks and docs build when applicable.
14. Collect non-sensitive review evidence and attach or summarize it in the
    implementation PR.
15. Confirm no secrets, private logs, private user data, or unrelated source
    changes were included.

## Verification Plan

- Repository diff review:
  - Expected repository files are limited to README/docs changes and any allowed
    evidence links. Product specs and technical specs should not be edited during
    Stage 10 unless a later approved spec revision requires it.
  - `git diff --name-only` confirms no Rust source, Cargo, runtime, generated,
    or implementation-unrelated files changed.
- Static text checks:
  - `rg` confirms Discord invite references are present where intended.
  - `rg` or manual review confirms no closing issue keywords were added to
    spec-only references.
  - Secret scan by reviewer judgment and repository search confirms no Discord
    tokens, private logs, or personal data are committed.
- Docs checks:
  - If docs source changes and `mdbook` is available, run
    `mdbook build sites/docs`.
  - If unavailable, document the missing command and perform Markdown link/copy
    review.
- Discord configuration review:
  - Evidence shows required channels and roles.
  - Evidence shows least-privilege permission settings.
  - Evidence shows welcome/onboarding links and server-visible fallback.
  - Evidence shows moderation safeguards and maintainer action path.
- Live manual E2E:
  - Open README invite and verify the official DUUMBI join flow.
  - Open `duumbi.dev` invite if updated, or verify linked follow-up evidence.
  - Use a normal-member view to verify public channel visibility and restricted
    announcement posting.
  - Use a maintainer view to verify announcement posting and moderation controls.

## Completion Criteria

Implementation is ready for review when all of these are true:

- Official DUUMBI Discord server exists under maintainer control.
- Channels `#general`, `#help`, `#showcase`, `#development`, and
  `#announcements` exist and have understandable topics, pins, or onboarding
  guidance.
- Roles `Maintainer`, `Contributor`, and `Community` exist.
- Community users cannot administer the server and cannot post in
  `#announcements`.
- Maintainers can administer the server and post announcements.
- Contributors can help in `#development` and `#help` without full
  administration rights.
- Welcome/onboarding includes docs, GitHub, getting-started material, and Code
  of Conduct links.
- Required onboarding links are also visible through a server-visible fallback.
- Moderation automation is configured and review evidence identifies the enabled
  safeguards.
- README contains the official invite, and the invite works.
- `duumbi.dev` contains the official invite, or implementation evidence links an
  owner/source/follow-up path explaining why the source could not be updated.
- Discord-originated execution decisions are routed back to GitHub or DUUMBI
  intake before acceptance.
- No secrets, private moderation logs, private user data, or unrelated code
  changes are committed.
- Every product BDD scenario has corresponding test, manual, or review evidence.

## Failure And Escalation

- If Discord access is unavailable, stop the cycle and ask for maintainer access
  or maintainer-executed configuration.
- If required server configuration cannot be verified from a normal-member view,
  do not claim completion; capture the gap and continue only within the resource
  policy.
- If the selected moderation or welcome bot cannot satisfy required safeguards,
  either configure an equivalent standard capability or ask for product/technical
  guidance before adding custom bot scope.
- If `duumbi.dev` source cannot be found or changed, create or link the external
  follow-up/update path and include it as review evidence.
- If implementation requires credentials, tokens, private logs, personal data,
  paid services, custom bot hosting, or irreversible server ownership changes,
  stop and request human approval.
- If checks fail, fix only within the approved scope. If the fix requires code,
  generated artifacts, product changes, or broader documentation strategy,
  escalate before proceeding.
- If requirements conflict, prefer the approved product spec and issue #376 over
  older roadmap wording, then ask for clarification if the conflict still
  changes scope or risk.

## Open Questions

None blocking for implementation.

Accepted implementation risk:

- The `duumbi.dev` source or owning repository was not found in this repository
  during Stage 6 and Stage 8 context inspection. Stage 10 must either update the
  correct external source when available or provide evidence of the owner,
  follow-up, or update path before claiming this requirement complete.
