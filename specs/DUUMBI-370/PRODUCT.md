# DUUMBI-370: LinkedIn Series For Weekly Development Progress Posts

## Summary

Create a reviewable DUUMBI LinkedIn content series package for twelve weekly
development progress posts, including a twelve-week content calendar, the first
four complete post drafts, reusable visual-branding templates, a hashtag
strategy, and Twitter/X cross-post variants.

The series should turn DUUMBI's existing development evidence into public,
evidence-backed progress communication. It is Phase 14 marketing work. It does
not change DUUMBI compiler, CLI, Studio, registry, MCP, agent workflow, or
runtime behavior.

This is a specification artifact only. The linked execution issue must remain
open for Stage 7 review, Stage 8 technical specification, implementation,
review, and later completion evidence.

## Problem

DUUMBI has a substantial technical story: a graph-native compiler, JSON-LD as
program structure, Cranelift native output, intent-driven development, Studio,
multi-LLM support, dynamic agents, MCP, and an active evidence-oriented delivery
workflow. That story is currently scattered across README, docs, blog posts,
roadmap notes, issues, specs, tests, and release evidence.

Without a planned public content series, Phase 14 marketing risks becoming
inconsistent and opportunistic:

- posts may overclaim future capabilities before evidence exists;
- technical decisions may be explained without links to credible source
  material;
- each post may require fresh format decisions, slowing publication;
- LinkedIn and Twitter/X variants may drift from each other;
- the first month of publication may stall because no launch-ready drafts exist.

The accepted issue asks for a LinkedIn series covering weekly progress,
technical milestones, architecture decisions, and lessons learned. The product
requirement is not just to write social copy. It is to create a repeatable,
evidence-backed content package that can support steady public communication
without weakening DUUMBI's credibility.

## Outcome

When this is done:

- A twelve-week content calendar exists and is reviewable in source control or
  another durable repository artifact chosen by the technical spec.
- Each calendar entry has a week number, theme, primary audience, core message,
  evidence/source links, intended asset/template, LinkedIn call to action,
  Twitter/X cross-post approach, and draft status.
- The first four LinkedIn posts are fully drafted and ready for human editorial
  review.
- The first four posts each have a Twitter/X cross-post variant or thread plan
  that preserves the core message within the platform's shorter format.
- A consistent visual-branding template set exists for the series, with enough
  guidance for later posts to reuse the style.
- A hashtag strategy exists and includes baseline DUUMBI/project hashtags plus
  per-topic hashtags, including the accepted issue's seed set:
  `#rust`, `#compiler`, `#ai`, and `#semanticweb`.
- Every post claim is tied to available DUUMBI evidence or clearly marked as a
  future-looking roadmap point.
- The content package avoids claims that Phase 14 guidance says are premature,
  especially unproven self-healing, enterprise control-plane, or launch-maturity
  claims.
- Later implementation review can verify the package without needing access to
  private social-media accounts.
- The execution issue remains open after this product-spec PR because later
  workflow stages still need to review, specify, implement, and verify the
  actual content artifacts.

## Scope

### In Scope

- Create a twelve-week LinkedIn content calendar for DUUMBI development progress.
- Draft the first four LinkedIn posts completely.
- Include post topics that cover technical milestones, architecture decisions,
  demos, development workflow lessons, and public adoption context.
- Define a reusable visual-branding template set for the series.
- Define where visual templates, post drafts, and calendar artifacts should live
  so later stages can review them durably.
- Define a hashtag strategy, including baseline series tags and per-topic tags.
- Create Twitter/X cross-post variants or thread plans for each of the first
  four posts.
- Define editorial rules for evidence-backed claims, future-roadmap language,
  and links to DUUMBI public surfaces.
- Use available source context from `hgahub/duumbi`, `hgahub/duumbi-web`, and
  the DUUMBI vault to ground the series.
- Include review evidence expectations for calendar completeness, draft quality,
  branding consistency, hashtag strategy, cross-post variants, and source
  traceability.

### Explicitly Out Of Scope

- Creating technical specifications during Stage 6.
- Implementing content artifacts, visual assets, website changes, automation,
  social publishing, or Ralph cycles during Stage 6.
- Publishing posts to LinkedIn, Twitter/X, Medium, blog, Discord, Reddit, Hacker
  News, or any other public channel.
- Creating or requiring social-media account credentials, API keys, scheduler
  integrations, analytics integrations, or automated posting bots.
- Making implementation changes to DUUMBI compiler, CLI, Studio, registry, MCP,
  runtime, provider behavior, workflow automation, or tests.
- Claiming Phase 13 self-healing, enterprise control-plane, production
  telemetry, launch readiness, or public adoption metrics before evidence exists.
- Replacing the existing blog strategy, YouTube/demo video tasks, Discord
  community work, or Hacker News launch task.
- Creating a broad brand system, style guide, media kit, paid campaign, or
  growth analytics dashboard.

## Constraints And Assumptions

Facts:

- Issue #370 is open, labeled `phase-14`, `module:marketing`, `accepted`, and
  `needs-spec`, and is routed to Stage 6 from `Spec Needed`.
- The Stage 4 triage comment routed #370 to `Needs Human Acceptance` as a P1
  marketing task connected to weekly LinkedIn development progress posts.
- The Stage 5 human acceptance decision accepted #370 on 2026-06-01, recorded
  no remaining open questions, no canonical duplicate, and next state
  `Spec Needed`.
- The accepted issue requires a twelve-week minimum content calendar, the first
  four posts drafted, consistent visual branding templates, a hashtag strategy,
  and Twitter/X cross-posting.
- The archived Phase 14 Marketing & Go-to-Market roadmap includes a LinkedIn
  series under content marketing: weekly posts on development progress,
  technical decisions, and demos.
- Phase 14 guidance says marketing should start after stable showcase results
  and must not market features that do not reliably work.
- The active Product Roadmap 2026-05 says Phase 14 may continue in parallel, but
  launch claims should follow product evidence and should not over-market
  self-healing, remote sync, or enterprise control-plane capabilities before
  those areas are specified.
- The active PRD describes DUUMBI as an intent-driven development system that
  connects product intent, semantic graph structure, executable behavior,
  runtime feedback, and agentic development workflows.
- The `hgahub/duumbi-web` repository owns `duumbi.dev`, `docs.duumbi.dev`,
  public messaging, blog content, and developer-facing documentation.
- `duumbi-web` already has blog posts, OpenGraph/Twitter card defaults, and blog
  share links for Twitter/X, LinkedIn, and Hacker News.
- The related Phase 14 community issue #376 has completed through Stage 12 and
  can be referenced as evidence for public community surface maturity, but it is
  not a duplicate of #370.

Assumptions:

- The content artifacts should be durable review artifacts, not private drafts
  stored only inside LinkedIn, Twitter/X, or a personal note.
- The technical spec may choose the exact repository/path for the content
  package. The expected destination is likely `hgahub/duumbi-web` if the
  artifacts become public website/blog collateral, or `hgahub/duumbi` under a
  content/spec-support path if the artifacts are internal launch collateral.
- Cross-posting means creating platform-appropriate Twitter/X copy or thread
  plans, not automatically publishing to Twitter/X.
- Visual-branding templates can initially be lightweight source-controlled
  artifacts such as Markdown guidance, reusable image layouts, CSS/HTML
  snippets, Figma-exportable descriptions, or static template files, as long as
  implementation review can verify consistency.
- The first four posts should be editorially complete but do not need to be
  published during the implementation issue.
- The maintainer will make final editorial and publication decisions after the
  content package is reviewed.

Constraints:

- Every claim about shipped behavior, demos, tests, support, platform coverage,
  or product maturity must cite or reference available evidence.
- Future-looking content must be labeled as roadmap, planned, or in progress,
  not as shipped capability.
- Public copy must avoid confidential details from Slack, private Codex runs,
  private cost data, credentials, unpublished customer details, or non-public
  personal information.
- The content package must not require committing social-media credentials,
  analytics tokens, private screenshots, private Slack material, or unpublished
  capability URLs.
- The package must be useful for human editorial review even if LinkedIn or
  Twitter/X accounts are unavailable during implementation.
- Spec-only PRs must use non-closing references such as `Spec for #370` or
  `Related to #370`; the execution issue stays open for later workflow stages.

## Decisions

- **Decision:** Use a file-based product spec for #370.
  **Evidence:** The work is public-facing, durable, cross-surface marketing
  content with calendar, copy, visual-template, hashtag, and cross-posting
  requirements. It needs review iterations and should remain traceable for later
  implementation.

- **Decision:** Treat the accepted issue as a content-package deliverable, not
  live publishing.
  **Evidence:** The issue acceptance criteria require calendar, drafts,
  templates, hashtag strategy, and cross-posting. It does not require account
  access, public posting, analytics, or scheduling automation.

- **Decision:** Require evidence-backed claims by default.
  **Evidence:** Phase 14 guidance says never market a feature that does not
  reliably work, and the Product Roadmap warns against over-marketing
  self-healing, remote sync, and enterprise control-plane claims before
  specification and evidence exist.

- **Decision:** Use `hgahub/duumbi-web` as the primary source context for public
  messaging and social-sharing behavior, while leaving final artifact placement
  to Stage 8.
  **Evidence:** The DUUMBI Repository Map says `duumbi-web` owns public
  messaging, blog content, and developer-facing documentation. The local
  `duumbi-web` source already includes blog posts and platform share links.

- **Decision:** Include Twitter/X variants as drafts or thread plans, not
  automated cross-posting.
  **Evidence:** The accepted issue asks for cross-posting to Twitter/X, but no
  source context establishes social account access, scheduler requirements, API
  credentials, or automation policy. A draft/thread-plan interpretation keeps
  the requirement reviewable without adding security or operations scope.

- **Decision:** Visual templates should be reusable and reviewable, but they do
  not need to become a comprehensive brand system in this issue.
  **Evidence:** The issue asks for consistent visual branding templates. A broad
  brand system, design language, or media kit would expand beyond the accepted
  scope.

- **Decision:** This spec PR must not close the execution issue.
  **Evidence:** Stage 6 creates a product spec candidate. Stage 7 review, Stage
  8 technical specification, implementation, review, merge, and Stage 12 closure
  still need to happen.

## Behavior

### Defaults

- The series is weekly and contains at least twelve planned LinkedIn entries.
- The first four entries have complete LinkedIn drafts.
- The first four entries have Twitter/X variants or thread plans.
- Each entry references one primary DUUMBI story rather than trying to cover the
  whole product.
- Each entry identifies whether it is about shipped evidence, current work,
  roadmap context, or a lesson learned.
- Each entry identifies at least one source of evidence or source context.
- Calls to action point to public DUUMBI surfaces where appropriate, such as
  `duumbi.dev`, `docs.duumbi.dev`, the GitHub repository, blog posts, or
  Discord.
- Hashtags use a consistent baseline plus topic-specific additions.
- Visual branding uses a repeatable template family rather than one-off
  graphics per post.

### Inputs

- Issue #370 title, body, acceptance criteria, labels, Stage 4 triage comment,
  and Stage 5 human acceptance decision.
- Active DUUMBI PRD, Glossary, Agentic Development Runbook, Repository Map, and
  Phase 14/Product Roadmap context.
- Existing DUUMBI website/blog/docs source context from `hgahub/duumbi-web`.
- Existing public collateral such as README, docs, blog posts, screenshots,
  diagrams, specs, issue evidence, and CI/test evidence selected by the later
  implementation stage.
- Human editorial preference during later implementation review.

### Outputs

- A twelve-week content calendar artifact.
- Four complete LinkedIn post drafts.
- Four Twitter/X cross-post variants or thread plans.
- A hashtag strategy artifact or section.
- A visual-template artifact or section with reusable layout guidance and any
  source-controlled template assets selected by Stage 8.
- Review evidence that maps posts to DUUMBI source/evidence links.

### Visible States

- Calendar entry is planned, drafted, needs evidence, needs editorial review, or
  ready for publication.
- Post claim is evidence-backed, future-roadmap, or rejected as unsupported.
- Visual template is defined, used by a draft, or missing.
- Twitter/X variant is provided, needs trimming, or intentionally omitted with a
  reviewable reason.
- Hashtag set is baseline, topic-specific, or not approved.

### Empty States

- If a future week does not yet have a final draft, the calendar still contains
  enough topic, audience, source, CTA, hashtag, and template guidance for a
  later writer to draft it.
- If a visual asset cannot be generated in implementation, the package must
  still include template guidance and a clear follow-up path rather than leaving
  the series visually undefined.
- If a source link for a proposed claim cannot be found, the draft must either
  remove the claim or mark it as future-looking roadmap language.

### Error States And Recovery

- If a post claims an unshipped capability as shipped, implementation review
  fails until the claim is corrected or backed by evidence.
- If a draft uses private Slack/Codex/source material that should not be public,
  the material must be removed or summarized safely before review.
- If the content calendar has fewer than twelve weeks, review fails.
- If fewer than four LinkedIn drafts are complete, review fails.
- If Twitter/X variants are absent for the first four posts, review fails unless
  a human reviewer explicitly accepts a narrower cross-posting definition.
- If visual-branding templates are missing or too vague to reproduce, review
  fails.
- If hashtag strategy omits the accepted seed tags without rationale, review
  fails.
- If the selected artifact path is outside available repositories or cannot be
  reviewed through PR evidence, the implementation stage must stop and ask for a
  narrower placement decision.

### Accessibility And Usability

- Visual templates should work as static images or source layouts that remain
  legible on mobile LinkedIn feeds.
- Image text should be short enough to read on mobile and should not be the only
  place where essential information appears.
- Drafts should avoid unexplained internal jargon when speaking to a public
  audience.
- Long technical concepts should link to docs/blog/source material instead of
  relying on dense social copy.
- Calls to action should be direct and actionable.

### Invariants

- GitHub issues, PRs, specs, tests, docs, and public site/source artifacts remain
  the source of evidence for product claims.
- Social platforms are distribution channels, not durable sources of product
  truth.
- The content package must not require storing credentials or private account
  tokens in any repository.
- The execution issue remains open through Stage 6 and later workflow gates.
- Publication decisions remain human-controlled.

## BDD Scenarios

Feature: DUUMBI weekly development progress content series

  Rule: The calendar must support twelve weeks of evidence-backed planning

    Scenario: Reviewer inspects the content calendar
      Given the implementation adds a DUUMBI LinkedIn series content calendar
      When a reviewer checks the calendar
      Then it contains at least twelve weekly entries
      And each entry has a week number, theme, audience, core message,
      evidence/source links, intended visual template, LinkedIn call to action,
      Twitter/X cross-post approach, and draft status

    Scenario: A planned post lacks evidence for a shipped claim
      Given a calendar entry or draft claims a DUUMBI capability is shipped
      When no public or repository evidence is linked for that claim
      Then the claim is either removed, rewritten as future-roadmap language, or
      linked to valid evidence before review passes

    Scenario: A planned post discusses future work
      Given a post topic covers roadmap, planned, or in-progress work
      When the draft describes that work
      Then the copy clearly marks it as roadmap, planned, or in progress
      And it does not present the work as already shipped

  Rule: The first four LinkedIn posts must be editorially reviewable

    Scenario: Reviewer inspects the first four posts
      Given the content package includes the first four LinkedIn drafts
      When a reviewer opens each draft
      Then each draft has a headline or opening hook, body copy, core technical
      point, evidence/source link, call to action, hashtags, and visual-template
      reference

    Scenario: A draft is too dependent on internal context
      Given a LinkedIn draft uses internal DUUMBI vocabulary
      When the vocabulary is not explained in the post or linked source
      Then the draft is revised to explain the concept plainly or link to
      suitable public context

    Scenario: A draft includes private source material
      Given a LinkedIn draft includes private Slack, Codex, credential, or
      unpublished capability-url material
      When the content package is reviewed
      Then the private material is removed or replaced with a safe public
      summary before review passes

  Rule: Visual branding must be reusable

    Scenario: Reviewer inspects visual templates
      Given the content package includes visual-branding templates
      When a reviewer checks the template guidance or source assets
      Then the templates define a repeatable DUUMBI series style
      And the first four posts reference or use those templates

    Scenario: A template contains dense text
      Given a visual template includes text intended for a social image
      When the image is reviewed for mobile-feed readability
      Then the essential message is short, legible, and repeated or supported in
      the post copy

  Rule: Hashtags must be consistent and topic-aware

    Scenario: Reviewer inspects hashtag strategy
      Given the content package includes a hashtag strategy
      When a reviewer checks the baseline hashtags
      Then the strategy includes `#rust`, `#compiler`, `#ai`, and `#semanticweb`
      And it explains when topic-specific hashtags should be added or omitted

    Scenario: A post uses unrelated hashtags
      Given a draft includes a topic-specific hashtag
      When the hashtag does not match the post theme or audience
      Then the hashtag is removed or replaced before review passes

  Rule: Twitter/X variants must preserve the LinkedIn message

    Scenario: Reviewer inspects cross-post variants
      Given the first four LinkedIn drafts are present
      When a reviewer checks the Twitter/X variants or thread plans
      Then each variant preserves the core message and call to action
      And each variant is short enough for the selected Twitter/X format or is
      structured as a thread plan

    Scenario: LinkedIn copy is too long for a single Twitter/X post
      Given a LinkedIn draft exceeds a practical single-post Twitter/X length
      When the implementation creates the cross-post variant
      Then it provides a shorter summary or thread plan instead of copying the
      LinkedIn draft verbatim

  Rule: The series must align with DUUMBI public surfaces

    Scenario: A post points readers to DUUMBI
      Given a draft includes a public call to action
      When a reviewer checks the link target
      Then the target is a public DUUMBI surface such as `duumbi.dev`,
      `docs.duumbi.dev`, GitHub, Discord, or a DUUMBI blog post

    Scenario: A post references a completed public community surface
      Given a draft references community discussion or support
      When a reviewer checks the supporting evidence
      Then the draft may point to the official Discord invite or related
      completed #376 evidence
      And it does not imply Discord replaces GitHub workflow state

  Rule: Stage 6 must remain specification-only

    Scenario: Stage 6 PR is reviewed
      Given this Stage 6 product spec PR is open
      When a reviewer inspects the changed files
      Then only `specs/DUUMBI-370/PRODUCT.md` is changed
      And no technical spec, implementation content artifact, source code,
      generated asset, workflow, or Ralph-cycle evidence is added

## Tasks

- Draft and review this product spec.
- Decide the implementation artifact placement in Stage 8.
- Create the twelve-week content calendar.
- Select evidence-backed weekly themes from DUUMBI roadmap, specs, docs, public
  website/blog, tests, and completed issue evidence.
- Draft the first four LinkedIn posts.
- Create Twitter/X variants or thread plans for the first four posts.
- Define the baseline and topic-specific hashtag strategy.
- Define or create reusable visual-branding templates for the series.
- Map each draft's claims to public or repository source evidence.
- Review drafts for unsupported claims, private material, jargon, and premature
  roadmap claims.
- Verify the content package can be reviewed without social account access.
- Keep later publication and scheduling decisions human-controlled.

Independent work:

- Calendar construction can proceed independently from visual template creation
  after the evidence rules are known.
- Hashtag strategy can proceed independently from the first four drafts.
- Twitter/X variants can be drafted after each LinkedIn draft reaches editorial
  completeness.
- Visual templates can be created before all twelve calendar entries are fully
  detailed, but the first four posts must reference or use them before
  implementation completion.

## Checks

- Product spec review:
  - Stage 7 review confirms the spec covers the twelve-week calendar, first four
    drafts, visual templates, hashtag strategy, Twitter/X variants, evidence
    rules, out-of-scope boundaries, and BDD scenarios.
  - The spec PR changes only `specs/DUUMBI-370/PRODUCT.md`.
  - The spec PR uses non-closing references to issue #370 and states that the
    execution issue must remain open.
  - Required automated reviews and checks are clean before the issue is moved to
    Spec Review.

- Implementation review expectations for later stages:
  - Verify the content calendar contains at least twelve weekly entries.
  - Verify each calendar entry includes theme, audience, core message,
    evidence/source links, intended visual template, LinkedIn CTA, Twitter/X
    cross-post approach, and status.
  - Verify the first four LinkedIn posts are complete drafts.
  - Verify the first four posts each include evidence/source links, CTA,
    hashtags, and visual-template reference.
  - Verify the first four posts each have Twitter/X variants or thread plans.
  - Verify visual-branding templates exist and are specific enough to reuse.
  - Verify hashtag strategy includes the accepted seed tags and explains
    topic-specific usage.
  - Verify shipped-capability claims have evidence links.
  - Verify roadmap claims are labeled as roadmap, planned, or in progress.
  - Verify drafts avoid private Slack/Codex/credential/capability-url material.
  - Verify social account credentials, scheduler tokens, analytics tokens,
    private screenshots, and generated secrets are not committed.
  - Verify no DUUMBI compiler, CLI, Studio, registry, MCP, runtime, workflow, or
    provider behavior changes are made for this marketing content package.
  - Verify publication did not occur as part of implementation unless a later
    explicit human decision authorizes it outside this issue's default scope.

- BDD scenario coverage:
  - Twelve-week calendar completeness.
  - Evidence-backed shipped claims.
  - Roadmap/future-work labeling.
  - First four LinkedIn drafts.
  - Plain-language explanation or public context for jargon.
  - Private material exclusion.
  - Reusable visual templates.
  - Mobile-readable visual copy.
  - Baseline and topic-specific hashtag strategy.
  - Twitter/X variants or thread plans.
  - Public DUUMBI call-to-action links.
  - Discord/community references staying subordinate to durable GitHub workflow.
  - Stage 6 spec-only boundary.

- Local checks for this Stage 6 artifact:
  - `git diff --check -- specs/DUUMBI-370/PRODUCT.md`
  - Auto-closing keyword scan confirms the spec, commit, and PR copy use
    non-closing references for #370.
  - Markdown/path inspection confirms only `specs/DUUMBI-370/PRODUCT.md` is
    added by the spec PR.

## Open Questions

None blocking for product-spec review.

Non-blocking Stage 8 placement question:

- Should the implementation artifacts live in `hgahub/duumbi-web` as public
  marketing/blog collateral, in `hgahub/duumbi` as internal launch collateral,
  or split between a public content subset and internal editorial planning? The
  technical spec should decide this before implementation.

Non-blocking editorial questions:

- Which four themes should be used for the first month if the maintainer wants
  final editorial control before publication?
- Should Twitter/X variants be single posts by default, or thread plans when the
  LinkedIn post is too technical for one short post?

## Sources

- GitHub issue #370:
  `https://github.com/hgahub/duumbi/issues/370`
- Stage 4 triage comment:
  `https://github.com/hgahub/duumbi/issues/370#issuecomment-4544282722`
- Stage 5 human acceptance decision:
  `https://github.com/hgahub/duumbi/issues/370#issuecomment-4596583800`
- Related completed Phase 14 issue #376:
  `https://github.com/hgahub/duumbi/issues/376`
- Related product spec:
  `specs/DUUMBI-376/PRODUCT.md`
- Active PRD:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active Glossary:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Active Agentic Development Runbook:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- DUUMBI Repository Map:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Repository Map.md`
- Archived Phase 14 Marketing & Go-to-Market roadmap:
  `hgahub/duumbi-vault: Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 14 - Marketing & Go-to-Market.md`
- Product Roadmap 2026-05:
  `hgahub/duumbi-vault: Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Product Roadmap 2026-05.md`
- Static Website and Docs Publishing:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Static Website and Docs Publishing.md`
- Public Docs as Product Interface:
  `hgahub/duumbi-vault: Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Public Docs as Product Interface.md`
- DUUMBI architecture reference:
  `docs/architecture.md`
- DUUMBI coding conventions:
  `docs/coding-conventions.md`
- DUUMBI web README:
  `hgahub/duumbi-web: README.md`
- DUUMBI web blog source:
  `hgahub/duumbi-web: src/content/blog/introducing-duumbi.md`
- DUUMBI web metadata and social-sharing source:
  `hgahub/duumbi-web: src/layouts/Layout.astro`
- Stage 7 readiness workflow:
  `.github/workflows/spec-review-request.yml`
