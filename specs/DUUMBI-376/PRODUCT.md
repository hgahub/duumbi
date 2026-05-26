# DUUMBI-376: Discord Server Setup For Community Discussions And Support

## Summary

Create and launch the public DUUMBI Discord server as the community discussion,
support, showcase, development coordination, and announcement surface for the
project.

The server should give new and returning community members a clear first path:
join through official DUUMBI links, understand the community rules, find the
right channel, receive useful onboarding links, and interact under basic
moderation safeguards. This is Phase 14 community-building work. It supports
public adoption without changing DUUMBI compiler, registry, Studio, CLI, or
agent behavior.

## Problem

DUUMBI has public project surfaces in GitHub, `duumbi.dev`, `docs.duumbi.dev`,
and the registry, but it does not yet have an official real-time community
space for discussion, support, showcase sharing, contributor coordination, or
announcements.

That creates two product risks:

- Interested users have no obvious place to ask support questions or discuss
  examples after reading the README or docs.
- Community growth work can fragment across informal channels that are not
  linked from official DUUMBI surfaces or governed by the existing Code of
  Conduct.

The accepted issue asks for a Discord server with specific channels, roles, a
welcome bot, official invite links, and moderation bot configuration. The
product requirement is not merely "create a server"; it is to create an
official, discoverable, minimally moderated community entry point that matches
DUUMBI's current public positioning.

## Outcome

When this is done:

- DUUMBI has an official public Discord server owned by the project maintainer
  account or an explicitly delegated maintainer-controlled account.
- The server exposes these public channels: `#general`, `#help`, `#showcase`,
  `#development`, and `#announcements`.
- The server has the roles `Maintainer`, `Contributor`, and `Community`, with
  permissions that match their names and avoid unnecessary elevated access for
  regular members.
- New members receive an automated welcome message that links to DUUMBI docs,
  GitHub, getting-started material, and the Code of Conduct.
- Moderation automation is configured enough to reduce spam and enforce basic
  community safety without blocking legitimate first-time participation.
- An official invite link is added to the GitHub README.
- An official invite link is added to `duumbi.dev` if the website source is
  available to the implementation stage; otherwise implementation evidence must
  record the unavailable source and create or link the follow-up needed to update
  the website.
- The implementation records review evidence showing the server structure,
  roles, welcome flow, moderation configuration, and linked public entry points.

## Scope

### In Scope

- Create the official DUUMBI Discord server.
- Configure the public channels:
  - `#general` for broad project discussion.
  - `#help` for support questions and troubleshooting.
  - `#showcase` for demos, examples, screenshots, and community-built modules.
  - `#development` for contributor and implementation discussion.
  - `#announcements` for maintainer-controlled project updates.
- Configure the roles:
  - `Maintainer` for trusted project maintainers who can administer the server
    and publish announcements.
  - `Contributor` for recognized project contributors who may help with support
    and development discussion without receiving full administration rights.
  - `Community` for ordinary members.
- Set channel permissions so `#announcements` is writable only by Maintainers,
  while community discussion channels remain usable by non-maintainers subject to
  moderation limits.
- Configure automated welcome behavior for new members.
- Include official links in the welcome message:
  - DUUMBI docs: `https://docs.duumbi.dev/`
  - DUUMBI GitHub repository: `https://github.com/hgahub/duumbi`
  - Getting-started documentation or README quickstart.
  - Code of Conduct: repository `CODE_OF_CONDUCT.md`.
- Configure moderation automation for spam reduction and Code of Conduct
  enforcement support.
- Add the Discord invite to README community/onboarding content.
- Add the Discord invite to `duumbi.dev` if its source is available to the
  implementation stage.
- Record evidence that the invite link is active and points to the official
  server.
- Keep the execution issue open through the specification and later
  implementation stages.

### Explicitly Out Of Scope

- Technical specification creation during Stage 6.
- Implementation code, source documentation edits, Discord server creation, bot
  installation, or Ralph cycles during Stage 6.
- Building custom Discord bots from scratch unless a later technical spec
  decides a standard bot cannot satisfy the required behavior.
- Creating a broad community program, contributor ladder, governance charter,
  ambassador program, or support SLA.
- Replacing GitHub Issues, Pull Requests, Project state, or Obsidian as durable
  sources of truth.
- Moving product decisions, issue state, or acceptance approvals into Discord as
  durable records.
- Adding paid support, enterprise support, or account-linked support workflows.
- Changing DUUMBI compiler, CLI, Studio, registry, MCP, or agent behavior.

## Constraints And Assumptions

Facts:

- Issue #376 is open and titled `community: Discord server setup - channels and
  welcome bot`.
- Issue #376 was accepted in Stage 5 on 2026-05-26 and routed to `Spec Needed`.
- Issue #376 has labels `phase-14`, `module:marketing`, `accepted`, and
  `needs-spec`.
- Stage 4 triage routed the issue to `Needs Human Acceptance` as an independent
  P1 community setup task with no dependencies.
- The issue acceptance criteria name the public channels `#general`, `#help`,
  `#showcase`, `#development`, and `#announcements`.
- The archived Phase 14 roadmap includes Discord under community building, but
  uses `#support` instead of the accepted issue's `#help`.
- README currently has a `Community` section with Code of Conduct guidance, but
  no Discord invite.
- `CODE_OF_CONDUCT.md` applies to all community spaces and gives maintainers
  responsibility for enforcing acceptable behavior.
- `sites/docs` exists in this repository as docs source for
  `https://docs.duumbi.dev/`.
- No `duumbi.dev` landing-site source was found in this repository during Stage
  6 context inspection.

Assumptions:

- The accepted issue is the source of truth for the channel name `#help`;
  `#support` in the older roadmap describes the purpose, not a required channel
  name.
- A standard, reputable Discord moderation/welcome bot is sufficient unless the
  later technical spec identifies a concrete gap.
- The invite link can be a stable public invite, but it should not grant elevated
  role access by itself.
- The implementation stage can verify the live Discord configuration manually
  and with screenshots or exported settings as review evidence.

Constraints:

- Discord is a public community surface, not the execution source of truth.
  GitHub Issues, Project state, PRs, and CI remain authoritative for delivery
  workflow.
- Discord decisions that affect product scope, roadmap, implementation, or
  acceptance must be copied into GitHub or the active DUUMBI knowledge workflow
  before they are treated as durable.
- Role permissions must follow least privilege.
- Bot configuration must not require secrets, API tokens, or private moderation
  logs to be committed to this repository.
- Public copy must not promise support response times, production maturity,
  Windows maturity, or support tiers that the current product does not provide.
- The spec-only PR must use non-closing issue references such as `Spec for #376`
  or `Related to #376`; it must leave the execution issue open.

## Decisions

- **Decision:** Use a file-based product spec for #376.
  **Evidence:** The accepted work is public-facing, durable, crosses Discord,
  README, website/docs discovery, moderation, and launch-readiness evidence, and
  should be reviewed before implementation.

- **Decision:** The required support channel is `#help`, not `#support`.
  **Evidence:** The Stage 5-accepted issue explicitly names `#help`; the older
  Phase 14 roadmap says `#support`. Stage 6 follows the accepted issue and uses
  support as the channel purpose.

- **Decision:** Discord must not become a durable workflow source of truth.
  **Evidence:** The active PRD and Agentic Development Map state that GitHub
  Project, issues, PRs, and CI hold execution state. Slack and other chat
  surfaces are capture or coordination surfaces only.

- **Decision:** Code of Conduct applicability must be explicit in onboarding and
  moderation behavior.
  **Evidence:** `CODE_OF_CONDUCT.md` says it applies within all community spaces
  and when representing the community publicly.

- **Decision:** A missing `duumbi.dev` landing source should not block the
  product requirement, but implementation must produce evidence.
  **Evidence:** The issue requires adding the invite to `duumbi.dev`; Stage 6
  found README and docs source locally, but not landing-site source. The product
  outcome remains required, with implementation evidence documenting whether the
  source was available or a follow-up/update path was needed.

- **Decision:** This spec PR must not close the execution issue.
  **Evidence:** Stage 6 creates a product spec candidate. Stage 7 review, Stage
  8 technical specification, implementation, review, merge, and Stage 12 closure
  still need to happen.

## Behavior

### Defaults

- The Discord server is public and discoverable from official DUUMBI surfaces.
- A new member receives the `Community` role by default or through a low-friction
  onboarding flow.
- Maintainers can administer the server, manage channels, configure bots,
  moderate members, and post announcements.
- Contributors can participate in support and development discussions without
  full administrative permissions.
- `#announcements` is read-only for non-maintainers.
- Public discussion channels remain open enough for new users to ask questions
  without manual preapproval.

### Inputs

- A user follows an official Discord invite from README or `duumbi.dev`.
- A new member joins the Discord server.
- A community member asks a question, shares a project, discusses development,
  or reads announcements.
- A maintainer posts an announcement.
- A spammy or Code-of-Conduct-violating message appears.
- A maintainer or contributor verifies server configuration during review.

### Outputs

- New members can identify where to post general discussion, help requests,
  showcases, development discussion, and announcement follow-up.
- New members see or receive links to docs, GitHub, getting-started material,
  and the Code of Conduct.
- Maintainers can publish announcements without regular members posting into
  `#announcements`.
- Moderation automation either blocks, flags, quarantines, logs, or rate-limits
  obvious abuse according to the selected bot's supported behavior.
- README contains the official Discord invite.
- `duumbi.dev` contains the official Discord invite or implementation evidence
  clearly documents why the website source was unavailable and what update path
  was taken.

### Visible States

- Invite works and lands on the official DUUMBI server.
- Public channels are visible to a normal community member.
- `#announcements` is visible to community members but not writable by them.
- Maintainer-only administration/moderation controls are not visible or usable
  by regular community members.
- Welcome/onboarding copy is visible either as an automated direct message, a
  welcome channel message, Discord onboarding prompt, or equivalent bot-supported
  flow.
- Moderation bot status/configuration is visible to Maintainers during review.

### Empty States

- If no discussions exist yet, each public channel still has enough description,
  topic text, pinned message, or onboarding copy for a first user to understand
  the channel purpose.
- If no contributors have been assigned yet, the `Contributor` role still exists
  and has conservative permissions.

### Error States And Recovery

- If the invite is expired, invalid, or points to the wrong server, the linked
  public surfaces fail review until the invite is replaced.
- If the welcome bot fails to send direct messages because a user blocks DMs, the
  server must still expose the same onboarding links through a visible server
  channel, server guide, or pinned message.
- If a selected moderation bot cannot support a required safeguard, the
  implementation must either configure an equivalent bot capability or document
  the gap for technical/product review before claiming completion.
- If `duumbi.dev` cannot be updated from this repository, implementation
  evidence must identify the owning repository or deployment path, or create/link
  a follow-up issue for that external surface.

### Accessibility And Usability

- Channel names use plain lowercase names that are easy to read, type, and link.
- Welcome copy is concise and avoids burying the important links.
- Pinned or channel-topic guidance should be readable without requiring prior
  project context.
- Important public links should not exist only in a bot DM because some users
  disable DMs from servers.

### Invariants

- The server remains governed by the project Code of Conduct.
- Community members do not receive administrative permissions by default.
- The official invite must not grant elevated access.
- Discord does not replace GitHub for issues, pull requests, project status, or
  acceptance decisions.
- The implementation must not commit Discord bot tokens, secrets, private invite
  management tokens, or moderation log exports containing private user data.

## BDD Scenarios

Feature: Official DUUMBI Discord community launch

  Rule: Public entry points must lead to the official server

    Scenario: A reader joins from the GitHub README
      Given the README contains an official DUUMBI Discord invite
      When a user opens the invite
      Then Discord opens the official DUUMBI server join flow
      And the invite does not grant elevated roles

    Scenario: A website visitor joins from duumbi.dev
      Given `duumbi.dev` has been updated with the official Discord invite
      When a visitor opens the invite from the website
      Then Discord opens the official DUUMBI server join flow
      And the invite matches the README invite or another approved official
      invite for the same server

    Scenario: Website source is unavailable during implementation
      Given the implementation stage cannot access the source that publishes
      `duumbi.dev`
      When the Discord work is reviewed
      Then the review evidence identifies the unavailable source or owning
      repository
      And it links the follow-up or update path needed to add the invite to
      `duumbi.dev`

  Rule: New members must know where to go

    Scenario: A new member receives onboarding links
      Given a user joins the DUUMBI Discord server
      When the welcome flow runs
      Then the user sees links to DUUMBI docs, GitHub, getting-started material,
      and the Code of Conduct
      And the user can identify where to ask for help

    Scenario: Direct-message welcome delivery fails
      Given a new member has disabled direct messages from servers
      When the welcome bot cannot send a direct message
      Then equivalent onboarding links are still visible in the server through a
      welcome channel, server guide, pinned message, or equivalent public flow

    Scenario: A member wants support
      Given a community member has a question about installing or using DUUMBI
      When they inspect the available channels
      Then `#help` is the obvious place for the question

    Scenario: A member wants to share a demo
      Given a community member has a DUUMBI example, module, screenshot, or demo
      When they inspect the available channels
      Then `#showcase` is the obvious place to share it

  Rule: Roles must follow least privilege

    Scenario: A regular member joins
      Given a new member joins the server
      When their default permissions are applied
      Then they have Community-level access
      And they cannot administer the server
      And they cannot post in `#announcements`

    Scenario: A maintainer posts an announcement
      Given a user has the Maintainer role
      When they post in `#announcements`
      Then the announcement is visible to community members
      And non-maintainers cannot post competing announcements in that channel

    Scenario: A recognized contributor helps in development discussion
      Given a user has the Contributor role
      When they participate in `#development` or `#help`
      Then they can discuss and assist without receiving full server
      administration rights

  Rule: Moderation must support community safety

    Scenario: A spam message is posted
      Given a new or existing member posts obvious spam
      When the configured moderation automation evaluates the message
      Then the message is blocked, flagged, quarantined, logged, rate-limited, or
      otherwise surfaced to Maintainers according to the selected bot capability

    Scenario: A Code of Conduct issue is reported
      Given a community member reports abusive behavior
      When a Maintainer reviews the report
      Then the Maintainer can take an appropriate moderation action
      And the action follows the Code of Conduct enforcement expectations

  Rule: Discord must not replace durable project workflow

    Scenario: A member proposes a product change in Discord
      Given a member proposes a product or implementation change in Discord
      When the idea is considered for execution
      Then the durable work item is captured in GitHub or the DUUMBI intake
      workflow before it is treated as accepted work

## Tasks

- Draft and review this product spec.
- Create the Discord server under maintainer control.
- Configure public channels and channel topics/pins.
- Configure roles and least-privilege channel permissions.
- Configure welcome/onboarding behavior with required links.
- Configure moderation automation and maintainer-visible moderation handling.
- Verify normal-member and maintainer-visible behavior.
- Add the official invite to README.
- Add the official invite to `duumbi.dev` or document the external update path
  if the source is unavailable.
- Collect review evidence for channels, roles, welcome flow, moderation, invite
  validity, README link, and website link or follow-up.
- Keep Discord-originated product decisions routed back to GitHub or the DUUMBI
  intake workflow.

Independent work:

- Server/channel/role setup can proceed independently from README and website
  link updates after the official invite exists.
- README and website updates can proceed independently from moderation tuning
  once the invite is stable.
- Moderation and welcome bot configuration can be verified independently, but
  both must be complete before the issue is implementation-complete.

## Checks

- Product spec review:
  - Stage 7 review confirms the spec covers channels, roles, welcome behavior,
    moderation, public links, out-of-scope boundaries, and BDD scenarios.
  - The spec PR changes only `specs/DUUMBI-376/PRODUCT.md`.
  - The spec PR uses non-closing references to issue #376 and states that the
    execution issue must remain open.
  - Required automated reviews and checks are clean before the issue is moved to
    Spec Review.

- Implementation review expectations for later stages:
  - Verify the Discord server exists and is owned or controlled by the project
    maintainer.
  - Verify public channels `#general`, `#help`, `#showcase`, `#development`,
    and `#announcements` exist.
  - Verify roles `Maintainer`, `Contributor`, and `Community` exist.
  - Verify Community users cannot post in `#announcements` or administer the
    server.
  - Verify Maintainers can administer the server and post announcements.
  - Verify Contributors do not have full administrative permissions by default.
  - Verify welcome/onboarding behavior includes docs, GitHub, getting-started
    material, and Code of Conduct links.
  - Verify onboarding links remain visible even when bot direct messages are
    unavailable.
  - Verify moderation automation is configured and review evidence identifies
    what safeguards are enabled.
  - Verify the README invite link works and points to the official server.
  - Verify the `duumbi.dev` invite link works, or review evidence links the
    follow-up/update path if the website source was unavailable.
  - Verify no Discord secrets, tokens, private logs, or personal data are
    committed.
  - Verify Discord does not claim to replace GitHub Issues, PRs, or Project
    workflow.

- BDD scenario coverage:
  - README invite.
  - Website invite or documented external update path.
  - New-member onboarding.
  - Direct-message welcome fallback.
  - Help and showcase channel discovery.
  - Community, Contributor, and Maintainer permissions.
  - Maintainer-only announcements.
  - Moderation handling.
  - Durable routing of Discord-originated product changes.

- Local checks for this Stage 6 artifact:
  - Markdown/path inspection confirms only `specs/DUUMBI-376/PRODUCT.md` is
    added by the spec PR.
  - No compiler, CLI, Studio, registry, website, or implementation files are
    changed during Stage 6.

## Open Questions

None blocking for product-spec review.

Non-blocking implementation detail:

- The implementation stage should identify the current owner/source path for
  `duumbi.dev` before claiming the website invite update is complete, because no
  landing-site source was found in this repository during Stage 6 inspection.

## Sources

- GitHub issue #376:
  `https://github.com/hgahub/duumbi/issues/376`
- Stage 4 triage comment:
  `https://github.com/hgahub/duumbi/issues/376#issuecomment-4538613990`
- Stage 5 human acceptance decision:
  `https://github.com/hgahub/duumbi/issues/376#issuecomment-4548790938`
- README community section:
  `README.md`
- Code of Conduct:
  `CODE_OF_CONDUCT.md`
- Active PRD:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active Glossary:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Open Source Monetization Model B:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Open Source Monetization Model B.md`
- Archived Phase 14 Marketing & Go-to-Market roadmap:
  `hgahub/duumbi-vault: Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 14 - Marketing & Go-to-Market.md`
- Stage 7 readiness workflow:
  `.github/workflows/spec-review-request.yml`
