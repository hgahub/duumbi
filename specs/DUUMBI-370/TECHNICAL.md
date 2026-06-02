# DUUMBI-370: LinkedIn Series For Weekly Development Progress Posts - Technical Specification

## Implementation Objective

Implement the approved product outcomes for issue #370 by creating a durable,
reviewable DUUMBI social-content package for a twelve-week LinkedIn development
progress series.

The implementation must produce:

- a twelve-week content calendar;
- four complete LinkedIn post drafts;
- four Twitter/X variants or thread plans;
- reusable visual-branding templates;
- a hashtag strategy including `#rust`, `#compiler`, `#ai`, and
  `#semanticweb`;
- evidence mapping for shipped, roadmap, and unsupported claims.

This is Phase 14 marketing content work. It must not change DUUMBI compiler,
CLI, Studio, registry, MCP, runtime, provider behavior, delivery workflow
automation, generated runtime artifacts, or product specs.

Technical spec for #370. This specification is non-closing and the execution
issue must remain open for Stage 9 review and Stage 10 implementation.

## Agent Audience

Use this spec for:

- Codex App or Codex CLI agents coordinating Stage 10 Ralph-cycle
  implementation.
- Codex Cloud agents making bounded content-package edits.
- Human maintainers reviewing editorial claims, content strategy, visual
  direction, and publication readiness.
- Specialized reviewers checking public-claim evidence, private-material
  exclusion, hashtag strategy, cross-post variants, and no-credential/no-posting
  boundaries.

Do not use this spec to start implementation during Stage 8 or Stage 9.

## Source Context

- Product spec: `specs/DUUMBI-370/PRODUCT.md`.
- Product spec PR: `https://github.com/hgahub/duumbi/pull/644`.
- GitHub issue: `https://github.com/hgahub/duumbi/issues/370`.
- Stage 4 triage: issue comment
  `https://github.com/hgahub/duumbi/issues/370#issuecomment-4544282722`.
- Stage 5 acceptance: issue comment
  `https://github.com/hgahub/duumbi/issues/370#issuecomment-4596583800`.
- Stage 7 product spec approval: issue comment on #370 records approval of
  PR #644 and merge SHA
  `1facb4ab41805bb7e90ca0a8630c2af3b1c3e15e`.
- Repo instructions: `AGENTS.md`.
- Core repository references:
  - `README.md`: public positioning and current project claims.
  - `docs/architecture.md`: graph, JSON-LD, validation, Cranelift, agent,
    Query, and Intent source context.
  - `docs/coding-conventions.md`: source evidence for engineering discipline.
- Public website repository references from local
  `/Users/heizergabor/space/hgahub/duumbi-web` inspection:
  - `README.md`: confirms `duumbi-web` owns `https://duumbi.dev`, public
    messaging, landing-page content, documentation links, Discord, and GitHub
    Discussions.
  - `package.json`: Astro 6 site with `npm run build`.
  - `src/content.config.ts`: existing Astro content collection is limited to
    blog posts in `src/content/blog`.
  - `src/content/blog/introducing-duumbi.md`: public thesis and AI-first
    semantic graph compiler messaging.
  - `src/content/blog/how-duumbi-compiles-jsonld.md`: public technical source
    for JSON-LD, petgraph, validation, Cranelift, runtime, and AI mutation
    claims.
  - `src/layouts/Layout.astro`: OpenGraph and Twitter card defaults.
  - `src/pages/blog/[...slug].astro`: platform share links for Twitter/X,
    LinkedIn, and Hacker News.
  - `docs/src/content/docs/getting-started/introduction.md`: public docs source
    for the graph-native thesis and current feature list.
- Related Phase 14 references:
  - Product spec for #369: Medium article thesis and evidence-boundary context.
  - Product and technical specs for #376: completed Discord community surface
    context.

Verified source facts:

- `duumbi-web` is the correct repository for public website messaging and
  public content collateral.
- `duumbi-web` does not currently have a social-content package convention.
- The Astro site has an existing blog collection under `src/content/blog`.
- Adding social drafts to `src/content/blog` would imply blog publication
  semantics and schema requirements that are not part of #370.
- The product spec requires durable review artifacts, not live social
  publishing, account access, scheduler setup, analytics, or automation.
- Public DUUMBI copy already includes claims that must be rechecked against
  current evidence before reuse in social drafts.

Assumptions for implementation:

- A source-controlled review-only content package is sufficient for this issue;
  live publishing remains a later human-controlled decision.
- The preferred implementation repository is `hgahub/duumbi-web`, because it
  owns DUUMBI public messaging and social-sharing source context.
- The social-content package should live outside Astro routed content by
  default so implementation can remain content-only and publication-neutral.
- Markdown is the default artifact format for calendar, post drafts, evidence
  maps, hashtag rules, and visual-template guidance.
- Visual templates can start as precise Markdown template specifications.
  Lightweight source SVG or HTML/CSS template previews may be added during
  Stage 10 only when they are reviewable source files and stay inside the
  approved content package.

## Affected Areas

Expected Stage 10 implementation changes:

- External repository: `hgahub/duumbi-web`.
- New review-only content package:
  - `content/social/linkedin-progress-series/README.md`
  - `content/social/linkedin-progress-series/calendar.md`
  - `content/social/linkedin-progress-series/evidence-map.md`
  - `content/social/linkedin-progress-series/hashtag-strategy.md`
  - `content/social/linkedin-progress-series/visual-templates.md`
  - `content/social/linkedin-progress-series/posts/week-01.md`
  - `content/social/linkedin-progress-series/posts/week-02.md`
  - `content/social/linkedin-progress-series/posts/week-03.md`
  - `content/social/linkedin-progress-series/posts/week-04.md`
  - `content/social/linkedin-progress-series/x/week-01.md`
  - `content/social/linkedin-progress-series/x/week-02.md`
  - `content/social/linkedin-progress-series/x/week-03.md`
  - `content/social/linkedin-progress-series/x/week-04.md`
- Optional Stage 10 files inside the same package only:
  - `content/social/linkedin-progress-series/templates/*.md`
  - `content/social/linkedin-progress-series/templates/*.svg`
  - `content/social/linkedin-progress-series/templates/*.html`
  - `content/social/linkedin-progress-series/templates/*.css`

Source context to inspect but not edit by default:

- `duumbi-web/README.md`
- `duumbi-web/src/content/blog/*.md`
- `duumbi-web/docs/src/content/docs/**/*.md`
- `duumbi-web/src/layouts/Layout.astro`
- `duumbi-web/src/pages/blog/[...slug].astro`
- `duumbi/README.md`
- `duumbi/docs/architecture.md`
- issue, PR, spec, and test evidence linked from the calendar or evidence map

Areas that must not change for #370 implementation without separate approval:

- `hgahub/duumbi` Rust source, Cargo files, runtime files, tests, generated
  artifacts, workflow automation, product specs, and existing technical specs.
- `duumbi-web` application routes, Astro content schema, build configuration,
  package dependencies, public blog posts, docs pages, generated `dist/`
  output, analytics, scheduler code, or social posting automation.
- LinkedIn, Twitter/X, Medium, dev.to, Hashnode, Discord, Reddit, Hacker News,
  or any other live public channel.
- Social-media credentials, API keys, scheduler tokens, analytics tokens,
  private screenshots, private Slack material, unpublished capability URLs, or
  personal data.

If Stage 10 cannot access or propose a PR against `hgahub/duumbi-web`, it must
stop and ask for a placement decision instead of silently placing the package in
`hgahub/duumbi`.

## Technical Approach

Facts:

- The approved product spec leaves artifact placement to Stage 8.
- `duumbi-web` owns public DUUMBI messaging and already includes public blog,
  documentation, OpenGraph, and sharing context.
- #370 asks for durable content artifacts, not web publication.
- Public claims must be evidence-backed or clearly labeled as roadmap, planned,
  or in progress.

Placement decision:

- Place implementation artifacts in `hgahub/duumbi-web` under
  `content/social/linkedin-progress-series/`.
- Keep the package outside `src/content/blog` and outside `docs/src/content`
  unless a later human decision explicitly changes publication scope.
- Treat the package as review-ready editorial source, not as published site
  content.

Required package structure:

1. `README.md`
   - identifies the series, intended audience, editorial boundaries, evidence
     rules, and publication-control rule;
   - links the product and technical specs;
   - states that drafts are not published posts.
2. `calendar.md`
   - contains at least twelve weekly entries;
   - each entry has week number, theme, audience, core message, source/evidence
     links, visual template, LinkedIn CTA, Twitter/X approach, and draft status.
3. `evidence-map.md`
   - lists claim candidates and labels each as `shipped evidence`,
     `roadmap/planned`, `in progress`, or `unsupported/rejected`;
   - links each supported claim to public or repository evidence.
4. `hashtag-strategy.md`
   - defines baseline series tags including `#rust`, `#compiler`, `#ai`, and
     `#semanticweb`;
   - defines topic-specific tag selection and omission rules.
5. `visual-templates.md`
   - defines reusable template family, visual tone, layout rules, copy limits,
     mobile-readability requirements, and first-four-post template assignments.
6. `posts/week-01.md` through `posts/week-04.md`
   - contain complete LinkedIn drafts with hook, body, core technical point,
     evidence links, CTA, hashtags, and template reference.
7. `x/week-01.md` through `x/week-04.md`
   - contain Twitter/X single-post variants or thread plans preserving the
     LinkedIn draft's core message and CTA.

Recommended first-month topic set:

1. DUUMBI's core thesis: programs as typed JSON-LD semantic graphs.
2. Compilation pipeline: JSON-LD to petgraph validation to Cranelift native
   output.
3. AI-first mutation: graph patches, validation, and evidence-oriented agent
   workflow.
4. Public community and launch context: docs, website, GitHub, and Discord as
   public feedback surfaces.

This topic set is not mandatory if Stage 10 finds stronger evidence-backed
themes, but every replacement must preserve the product spec's requirements and
evidence rules.

Rejected alternatives:

- Do not use LinkedIn, Twitter/X, or a scheduler as the durable source of truth;
  those are distribution channels.
- Do not place the package in private notes only; implementation review needs PR
  evidence.
- Do not put drafts in `src/content/blog` by default; the issue is a social
  series, not blog publication.
- Do not generate or commit raster social images by default. If final social
  images are needed, request human design approval or create source templates
  first.
- Do not add automation, tracking, or live publishing to satisfy
  "cross-posting"; cross-posting means draft variants or thread plans.

## Invariants

- The execution issue remains open through Stage 10 and later workflow gates.
- Public claims about shipped behavior must link to public or repository
  evidence.
- Future work must be labeled as roadmap, planned, or in progress.
- Unsupported claims must be removed or marked rejected in the evidence map.
- Social platforms are distribution channels, not durable sources of product
  truth.
- Publication and scheduling decisions remain human-controlled.
- No credentials, API keys, scheduler tokens, analytics tokens, private Slack
  content, private Codex transcripts, unpublished capability URLs, private
  screenshots, customer data, or personal data may be committed.
- Discord or community references must not imply that Discord replaces GitHub
  Issues, PRs, specs, or the DUUMBI intake workflow for accepted work.
- No DUUMBI compiler, CLI, Studio, registry, MCP, runtime, provider, workflow,
  or test behavior changes are required or allowed for this issue.

## BDD-To-Test Mapping

| Product BDD scenario | Verification evidence |
| --- | --- |
| Reviewer inspects the content calendar | `content/social/linkedin-progress-series/calendar.md` exists with at least twelve weekly entries. Each entry includes week number, theme, audience, core message, evidence/source links, visual template, LinkedIn CTA, Twitter/X approach, and draft status. |
| A planned post lacks evidence for a shipped claim | `evidence-map.md`, calendar entries, and draft files show each shipped claim linked to public or repository evidence. Unsupported claims are removed, rewritten as future-looking, or listed as rejected. |
| A planned post discusses future work | Draft and calendar copy labels roadmap, planned, or in-progress work clearly and does not present it as shipped. Reviewer checks terms such as `roadmap`, `planned`, and `in progress` against the evidence map. |
| Reviewer inspects the first four posts | `posts/week-01.md` through `posts/week-04.md` exist and each contains hook, body copy, core technical point, evidence/source link, CTA, hashtags, and visual-template reference. |
| A draft is too dependent on internal context | Draft review confirms internal terms such as graph IR, intent workflow, Ralph cycle, Query mode, MCP, or Cranelift are explained plainly or linked to public context. |
| A draft includes private source material | Static search and manual review find no private Slack, credential, private Codex, unpublished capability URL, private screenshot, customer, or personal-data material in the package. |
| Reviewer inspects visual templates | `visual-templates.md` and any optional files under `templates/` define reusable layout, tone, copy limits, palette/typography guidance, and first-four-post assignments. |
| A template contains dense text | Visual-template guidance caps image text, requires mobile-feed readability, and requires essential meaning to be repeated or supported in post copy. |
| Reviewer inspects hashtag strategy | `hashtag-strategy.md` includes `#rust`, `#compiler`, `#ai`, and `#semanticweb`, plus baseline and topic-specific selection rules. |
| A post uses unrelated hashtags | Draft review verifies topic-specific hashtags match each post's theme and audience. Unrelated tags are removed or replaced. |
| Reviewer inspects cross-post variants | `x/week-01.md` through `x/week-04.md` exist and each preserves the matching LinkedIn draft's core message and CTA as a single-post variant or thread plan. |
| LinkedIn copy is too long for a single Twitter/X post | The matching `x/week-0N.md` file uses a shorter summary or thread structure instead of copying long LinkedIn text verbatim. |
| A post points readers to DUUMBI | CTA links point to public DUUMBI surfaces such as `https://duumbi.dev`, `https://docs.duumbi.dev`, GitHub, Discord, or DUUMBI blog posts. |
| A post references a completed public community surface | Discord/community references link to public community evidence or completed #376 evidence and do not treat Discord as workflow state. |
| Stage 8 PR is reviewed | Stage 8 PR changes only `specs/DUUMBI-370/TECHNICAL.md`, uses non-closing references, and does not add implementation code, tests, generated artifacts, runtime assets, product specs, or Ralph-cycle evidence. |

Suggested static checks for Stage 10 implementation:

```bash
find content/social/linkedin-progress-series -type f | sort
rg -n "^## Week|^### Week" content/social/linkedin-progress-series/calendar.md
rg -n "#rust|#compiler|#ai|#semanticweb" content/social/linkedin-progress-series/hashtag-strategy.md
rg -n "evidence|source|CTA|Twitter/X|template|status" content/social/linkedin-progress-series/calendar.md
rg -n "Slack|credential|token|secret|private|capability-url|unpublished|customer" content/social/linkedin-progress-series
git diff --name-only
git diff --check
```

These checks do not replace manual editorial review. They are guards for
structure, required seeds, private-material risk, and PR scope.

## Live E2E Plan

Canonical interface: source-control review of the content package in
`hgahub/duumbi-web`, because #370 is a marketing-content deliverable and not a
DUUMBI CLI, Studio, MCP, runtime, provider, or web-publication behavior.

Provider/LLM path:

- No DUUMBI provider call is required.
- No external social-platform API call is required.
- Expected DUUMBI live LLM calls: 0.
- Expected external publishing/scheduler calls: 0.
- Estimated external cost: USD 0.
- Codex internal reasoning usage should be reported qualitatively only; it is
  not part of the DUUMBI external LLM budget.

Credentials and access:

- GitHub write access to propose a `hgahub/duumbi-web` PR.
- No LinkedIn, Twitter/X, scheduler, analytics, or social API credentials are
  required.
- No DUUMBI provider credentials are required.

Repository checks:

- In `hgahub/duumbi-web`, run `git diff --name-only` and confirm changes are
  limited to `content/social/linkedin-progress-series/**` unless a later human
  approval explicitly expands scope.
- Run `git diff --check`.
- Run the static checks listed in the BDD mapping.
- Run `npm run build` in `duumbi-web` when dependencies are available. The
  content package is outside Astro routed content, so this build is a regression
  guard, not the primary acceptance mechanism.
- If Stage 10 edits `src/**`, `docs/**`, package files, or public routes despite
  the default scope, it must first request human approval and then run the
  relevant Astro build for the touched surface.

Manual review:

- Open the PR-rendered Markdown files and verify every BDD scenario has
  matching evidence.
- Click or inspect public links for `duumbi.dev`, `docs.duumbi.dev`, GitHub,
  blog posts, and Discord when referenced.
- Review the first four LinkedIn drafts for audience fit, public clarity,
  evidence-backed claims, and roadmap labeling.
- Review Twitter/X variants for length, thread structure, message preservation,
  and CTA consistency.
- Review visual-template guidance for reuse and mobile readability.

Pass criteria:

- Every product BDD scenario has automated, manual, or review evidence.
- The content package contains all required files.
- Calendar contains at least twelve weekly entries with required fields.
- The first four LinkedIn drafts and Twitter/X variants are complete.
- Baseline and topic-specific hashtag strategy is present.
- Visual templates are specific enough for later posts to reuse.
- Claims are evidence-backed, roadmap-labeled, or rejected.
- PR scope stays within the approved content package.
- No credentials, private material, generated `dist/`, social publishing, or
  unrelated code changes are included.

Fail criteria:

- Calendar has fewer than twelve weeks.
- Fewer than four LinkedIn drafts or Twitter/X variants are complete.
- Shipped claims lack evidence.
- Future claims read as already shipped.
- Hashtag strategy omits required seed tags without an explicit reviewer-
  accepted rationale.
- Visual templates are missing or too vague to reuse.
- Private material, credentials, tokens, or unpublished capability URLs appear.
- Implementation publishes, schedules, or automates social posts without a
  separate human decision.
- Implementation edits DUUMBI compiler/runtime/workflow behavior or unrelated
  `duumbi-web` site behavior.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. Summarize current state and remaining unmet product requirements.
2. Propose one bounded content-package goal.
3. List intended files, source references, commands, and manual checks.
4. Estimate external LLM calls, external service use, cost, and risk.
5. Check whether the resource gate requires human approval.
6. Implement only the approved or resource-permitted goal.
7. Run the agreed checks and collect required evidence.
8. Report changed files, verification results, failures, and remaining gaps.
9. Stop when requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded content-package goal per cycle.
- Max files per cycle:
  - Calendar/evidence cycle: up to four Markdown files.
  - Draft cycle: up to eight Markdown files when producing paired LinkedIn and
    Twitter/X drafts for weeks 1-4.
  - Template/hashtag/review cycle: up to six source-controlled content or
    template files.
- Expected command budget per cycle: up to eight local commands, excluding
  read-only `git` or `gh` inspection.
- Expected DUUMBI live provider usage: 0 calls, USD 0.
- Expected external social API usage: 0 calls, USD 0.
- Human approval is required when planned external LLM usage exceeds USD 2,
  exceeds 10 external calls, adds generated raster images, adds dependencies,
  changes Astro routes or published site content, requires social account
  access, publishes or schedules posts, introduces analytics or tracking,
  commits binary/generated assets, moves artifacts outside the approved package,
  changes implementation code, changes product specs, changes workflow behavior,
  or requires a product/editorial decision that materially changes scope.
- External LLM usage counted: DUUMBI live provider calls and external model or
  agent CLI/API calls outside Codex internal reasoning. Codex internal reasoning
  usage is reported only as an estimate.
- Autonomous batch cap: at most three Ralph cycles before asking for human
  review, even if resource use remains low.
- Stop and ask for human guidance when evidence for a desired claim cannot be
  found, the artifact placement is unavailable, public-source links are broken
  in a way that changes CTAs, a proposed visual direction requires final brand
  approval, or any required verification would expose private material.

## Task Breakdown

1. Confirm the implementation branch is based on the merged technical spec and
   that issue #370 is in Ready for Build.
2. Inspect `hgahub/duumbi-web` README, blog posts, docs intro, layout metadata,
   and sharing source for current public messaging.
3. Inspect `hgahub/duumbi` README, `docs/architecture.md`, related specs, and
   completed issue evidence for source-backed claim candidates.
4. Create `content/social/linkedin-progress-series/README.md` with scope,
   evidence, publication-control, and review instructions.
5. Create `evidence-map.md` with shipped, roadmap, in-progress, and rejected
   claim categories.
6. Create `calendar.md` with at least twelve weekly entries and all required
   fields.
7. Draft `posts/week-01.md` through `posts/week-04.md`.
8. Draft `x/week-01.md` through `x/week-04.md`.
9. Create `hashtag-strategy.md` with baseline and topic-specific rules.
10. Create `visual-templates.md` and optional source template files inside
    `templates/` if needed.
11. Run static checks, diff checks, and `npm run build` when available.
12. Review for unsupported claims, private material, jargon, premature roadmap
    language, CTA quality, hashtag relevance, visual-template reuse, and
    cross-post consistency.
13. Open a `hgahub/duumbi-web` implementation PR with review evidence and link
    it from issue #370.

## Verification Plan

- Repository diff review:
  - Stage 10 default implementation files are limited to
    `content/social/linkedin-progress-series/**` in `hgahub/duumbi-web`.
  - No DUUMBI compiler, CLI, Studio, registry, MCP, runtime, provider,
    workflow, product spec, generated `dist/`, or unrelated website files are
    changed.
- Static text checks:
  - `find` confirms the required package files exist.
  - `rg` confirms required hashtag seeds are present.
  - `rg` confirms required calendar field names or equivalents are present.
  - `rg` plus manual review confirms no credentials, secrets, private Slack
    material, private Codex material, unpublished capability URLs, private
    screenshots, customer data, or personal data are included.
  - Auto-closing keyword scan confirms spec-only references use non-closing
    wording.
- Build checks:
  - `npm run build` in `duumbi-web` when dependencies are available.
  - If dependencies are missing and installing them requires approval, record
    the blocker and complete static/manual checks.
- Editorial checks:
  - Every shipped claim has evidence.
  - Every future claim is labeled.
  - Unsupported claims are removed or rejected.
  - Internal vocabulary is explained or linked to public context.
  - CTAs point to public DUUMBI surfaces.
  - Discord references stay subordinate to GitHub workflow state.
- Content completeness checks:
  - Twelve calendar entries exist.
  - First four LinkedIn drafts are complete.
  - First four Twitter/X variants or thread plans are complete.
  - Visual templates are reusable and assigned to first-four drafts.
  - Hashtag strategy covers baseline and topic-specific usage.

## Completion Criteria

Implementation is ready for review when all of these are true:

- `hgahub/duumbi-web` contains
  `content/social/linkedin-progress-series/`.
- Required package files are present.
- Calendar contains at least twelve weekly entries with required fields.
- Weeks 1-4 have complete LinkedIn drafts.
- Weeks 1-4 have Twitter/X variants or thread plans.
- Hashtag strategy includes `#rust`, `#compiler`, `#ai`, and `#semanticweb`.
- Visual templates define repeatable DUUMBI series style and assign templates to
  the first four posts.
- Evidence map links shipped claims to public or repository evidence.
- Future-looking claims are labeled as roadmap, planned, or in progress.
- Unsupported claims are removed from drafts or listed as rejected.
- Drafts avoid private material and unexplained internal vocabulary.
- CTAs point to public DUUMBI surfaces.
- No social publishing, scheduling, analytics, credentials, or automation are
  added.
- No DUUMBI implementation code, tests, runtime assets, generated artifacts,
  workflow automation, product specs, or unrelated web surfaces are changed.
- Verification evidence is summarized in the implementation PR and linked from
  issue #370.

## Failure And Escalation

- If `hgahub/duumbi-web` cannot be accessed or used for a PR, stop and ask for a
  placement decision.
- If a desired claim cannot be backed by public or repository evidence, remove
  it, rewrite it as roadmap/in-progress language, or ask for editorial guidance.
- If a post would require private Slack, private Codex, credentials, private
  screenshots, unpublished capability URLs, customer data, or personal data,
  remove the material and use public evidence instead.
- If the work needs generated raster images, social posting, scheduling,
  analytics, account credentials, or publication, stop and request human
  approval.
- If implementation requires changes outside
  `content/social/linkedin-progress-series/**`, stop unless the change is a
  read-only inspection or an explicitly approved scope expansion.
- If checks fail, fix only within the approved content-package scope. If the fix
  requires code, dependencies, workflow changes, product-spec changes, or public
  site publication, escalate before proceeding.
- If requirements conflict, prefer the approved product spec and issue #370 over
  older roadmap wording, then ask for clarification if the conflict still
  changes scope or risk.

## Open Questions

None blocking for implementation.

Accepted implementation risk:

- `content/social/linkedin-progress-series/` is a new review-only convention in
  `hgahub/duumbi-web`. It is intentionally outside Astro routed content so the
  implementation can deliver durable editorial artifacts without accidentally
  publishing the series. If maintainers later want the package exposed on
  `duumbi.dev`, that should be handled as a separate publication decision.
