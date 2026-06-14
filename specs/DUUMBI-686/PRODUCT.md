# DUUMBI-686: Reconcile Legacy Docs With Canonical Docs Site

## Summary

Define the product behavior for reconciling DUUMBI's legacy documentation in
`hgahub/duumbi` with the canonical public documentation in `hgahub/duumbi-web`.

The accepted issue is a cross-repo preview blocker. The canonical user-facing
documentation lives in `hgahub/duumbi-web/docs/src/content/docs/` and is served
through the Starlight docs site. The `hgahub/duumbi` repo still contains:

- `sites/docs/`, an mdbook-era docs tree that is stale but still contains useful
  source material.
- `docs/`, a mixed internal/user-facing directory with current user-facing
  material, internal process docs, manual test protocols, stale evaluation
  results, and historical planning material.

The expected product outcome is not a blind deletion pass. Useful user-facing
content must move to the canonical docs site first, be refreshed to match current
DUUMBI behavior, and then legacy surfaces can be deleted or curated with review
evidence.

Related to #686. This is a product-specification artifact only. The execution
issue must remain open for Stage 7 Product Spec Review, Stage 8 technical
specification, Stage 9 Technical Spec Review, Stage 10 implementation, Stage 11
review, and Stage 12 closure.

## Problem

DUUMBI's documentation source of truth is split across active and legacy
surfaces:

- Public readers are directed to `docs.duumbi.dev`, but useful user-facing
  content still lives in `sites/docs/` and `docs/` inside the source repo.
- `sites/docs/` is an obsolete mdbook tree with stale commands and release
  claims, yet it includes pages with no clear canonical equivalent.
- `docs/` mixes public reference material with internal workflows, manual test
  protocols, historical research, QA corpora, and generated or accidental
  artifacts.
- The canonical docs site still contains stale provider/config examples, an
  unsupported `cargo install duumbi` claim while #687 remains open, and no
  provider-catalog page despite #675 adding current catalog material.
- Query mode is now a headline DUUMBI surface, but public docs do not frame the
  product as query-first with the same clarity as the README and internal docs.
- Community/support paths exist in the website and README, but the docs site
  needs a deliberate reader-facing surface for Discord, GitHub Discussions,
  issue templates, and feedback.

Docs drift is product drift. When docs state stale install commands, provider
names, config schemas, or source-of-truth rules, users and agents make wrong
changes with confidence.

## Outcome

When this work is done:

- Every page under `sites/docs/src/` and every page under `docs/` has a reviewed
  classification and rationale.
- User-facing content from `sites/docs/` and `docs/` is migrated or represented
  in `hgahub/duumbi-web/docs/src/content/docs/` with current behavior, not stale
  copy-paste.
- `sites/` is removed from `hgahub/duumbi` only after the useful migration
  coverage is present in the canonical docs site.
- `docs/` in `hgahub/duumbi` is curated as an internal/source-repo docs area:
  current internal material remains, stale material is updated or archived, and
  accidental cruft is removed if present.
- The canonical docs site has a provider-catalog page sourced from the current
  #675 provider catalog material and linked from provider setup docs.
- Provider setup examples use the current `[[providers]]` shape with `role`, and
  the docs no longer present the old single `[llm]` table as the primary setup
  path.
- Public provider lists include MiniMax and avoid presenting OpenRouter or
  `grok` as current v1 public provider guidance unless clearly labeled as legacy
  compatibility.
- Installation docs do not claim `cargo install duumbi` works until #687 has
  delivered a verified install channel.
- Query-first framing is present in the public docs and aligns with the README:
  Query is read-only by default, Agent/Intent are explicit write-capable
  handoffs, and provider setup remains under `/provider`.
- The docs site exposes a clear community and support surface for Discord,
  GitHub Discussions, issue templates, and feedback/orientation paths.
- Diagrams or infographics exist where they make core flows easier to understand,
  especially the compiler pipeline, intent flow, provider/catalog model, or
  registry/module model.
- The docs site builds cleanly, internal links are valid under the Astro/Starlight
  build, and no legacy mdbook links remain in active repo docs, README, CI, PR
  templates, or code ownership.
- The execution issue remains open after spec PRs and implementation PRs until
  Stage 12 closure verifies merged implementation evidence.

## Scope

### In Scope

- Audit and classify every `sites/docs/src/**` page.
- Audit and classify every `docs/**` Markdown, corpus, result, and manual test
  artifact.
- Migrate or rewrite still-useful user-facing mdbook content into the canonical
  Starlight docs structure.
- Migrate or rewrite user-facing `docs/` content into the canonical docs site,
  including provider catalog and Query mode documentation.
- Curate internal `docs/` content in the source repo with a clear rationale:
  keep, update, archive, delete, or hand off to another issue.
- Remove `sites/` and any mdbook build, CODEOWNERS, PR-template, README, CI, or
  docs references only after migration coverage exists.
- Remove `docs/.DS_Store` and `docs/.obsidian/` if present, and ensure ignore
  rules prevent recurrence.
- Update canonical docs installation/provider/config pages to match the README,
  #675, and #687 constraints.
- Add or update public docs navigation so migrated pages are discoverable.
- Add reader-oriented how-to guides where a reference dump would not be enough.
- Add diagrams or infographics where they materially improve understanding.
- Run docs-site build validation and source-repo validation relevant to docs
  reconciliation.
- Preserve traceability to the accepted issue, source docs, related issues, and
  implementation evidence.

### Explicitly Out Of Scope

- Implementing the release/install channel tracked by #687.
- Implementing scaled intent evals tracked by #689.
- Changing DUUMBI runtime, compiler, provider, registry, or Query behavior
  beyond documentation reconciliation.
- Publishing speculative roadmap content as stable user docs.
- Treating Obsidian as the public docs source of truth.
- Keeping `sites/docs/` as a parallel public documentation surface after useful
  content has been migrated.
- Creating a separate companion execution issue for `duumbi-web`; #686 remains
  the umbrella tracker.
- Stage 10 implementation code, test edits, docs edits, generated assets, or
  Ralph cycles during this specification stage.
- Product spec approval, technical spec approval, implementation merge, or Stage
  12 closure.

## Constraints And Assumptions

Facts:

- Issue #686 is open.
- Issue #686 is labeled `accepted` and `needs-spec`.
- The Stage 5 Human Acceptance Decision comment dated 2026-06-14 records
  `Decision: Accept`, `Next state: Spec Needed`, and no remaining open
  questions.
- The issue body identifies this work as preview blocker 1/4 for the
  v0.4.0 Developer Preview gate.
- The accepted issue body says the canonical user-facing docs live in
  `hgahub/duumbi-web/docs/src/content/docs/`.
- The `hgahub/duumbi` worktree contains `sites/docs/src/` with mdbook-era pages
  for getting started, CLI, JSON-LD, module system, architecture, and
  contributing.
- The `hgahub/duumbi` worktree contains `docs/` with provider catalog, Query
  mode docs, automation docs, e2e corpus/results, manual test protocols, and
  architecture/coding-conventions docs.
- `docs/provider-catalog.md` in `hgahub/duumbi` contains the #675 provider
  catalog material and accepted v1 provider table.
- The `hgahub/duumbi-web` worktree exists locally and contains Starlight docs
  under `docs/src/content/docs/`.
- `hgahub/duumbi-web/docs/src/content/docs/getting-started/installation.md`
  currently advertises `cargo install duumbi` and an old `[llm]` config table.
- Issue #687 is open and tracks the preview release/install path.
- Issue #689 is open and tracks scaled multi-function/multi-module intent evals.
- `hgahub/duumbi-web/docs/astro.config.mjs` includes GitHub and Discord social
  links, but the docs content does not yet expose a dedicated community/support
  page.
- `hgahub/duumbi-web/docs/public/model-catalog/v1/` already contains static
  catalog artifacts from #675, but the docs content does not yet provide a
  provider-catalog page.
- `.github/PULL_REQUEST_TEMPLATE.md` in `hgahub/duumbi` still references docs
  updates under `sites/docs/src/`.
- `.github/CODEOWNERS` includes ownership for `/sites/docs/`.
- `.gitignore` already ignores `.DS_Store` and `docs/.obsidian/`.
- No `docs/.DS_Store` file or `docs/.obsidian/` directory was found in the
  inspected worktree, but implementation must still remove them if they appear
  on the target branch or are generated locally.
- The active DUUMBI runbook requires combined product and technical specs,
  Codex self-review, clean AI gates, and no Greptile invocation on spec PRs.

Assumptions:

- A durable audit artifact is needed to make "every page classified" reviewable
  after `sites/` is removed.
- Some mdbook pages should be represented by refreshed canonical docs pages
  rather than moved one-to-one.
- Some `docs/` content should remain internal because it is agent workflow,
  manual QA, architecture, or contributor context rather than public user docs.
- Some stale `docs/e2e/results/*20260331*` material may be best archived or
  explicitly marked historical until #689 produces current scaled eval evidence.
- The implementation agent may need writable access to both `hgahub/duumbi` and
  `hgahub/duumbi-web` to complete the accepted umbrella scope.
- A docs-only implementation should not require live LLM calls, but it should
  verify claims against current CLI output and repository docs.

Constraints:

- Do not claim a working install channel before #687 provides evidence.
- Do not publish internal process docs, agent workflow rules, or unstable manual
  test notes as stable user documentation unless the content is rewritten for a
  public reader and clearly scoped.
- Do not delete `sites/` before migration coverage is demonstrated.
- Do not leave canonical docs with broken internal links or stale sidebar
  entries.
- Do not leave active repo references that tell contributors to update
  `sites/docs/src/`.
- Do not use GitHub issue-closing references in spec-only PRs for this issue.

## Decisions

- **Decision:** Use file-based specs for #686.
  **Evidence:** The work is cross-repo, user-visible, durable, and large enough
  to need reviewable behavior, source context, BDD scenarios, implementation
  boundaries, and explicit verification.

- **Decision:** Treat `duumbi-web/docs` as the only canonical public docs
  surface.
  **Evidence:** The accepted issue body and the active vault note "Public Docs
  as Product Interface" identify `duumbi-web` as the public documentation
  surface.

- **Decision:** Keep #686 as the cross-repo umbrella.
  **Evidence:** The accepted issue body explicitly says there is no separate
  companion issue; audit/deletion/cruft cleanup happens in `hgahub/duumbi`, and
  migration/update/reference wiring lands in `hgahub/duumbi-web`.

- **Decision:** Coordinate installation docs with #687 rather than inventing a
  final install command.
  **Evidence:** #687 is still open and tracks the preview release/install path;
  current `cargo install duumbi` public docs are not supported.

- **Decision:** Treat #675 provider catalog content as current source material
  for a public provider-catalog page.
  **Evidence:** `docs/provider-catalog.md` exists in `hgahub/duumbi` and #675 is
  closed with provider catalog material, while the canonical docs content does
  not yet include a provider-catalog page.

## Behavior

### Audit And Classification

The implementation must produce reviewable classification evidence for every
legacy docs item. Each row or section must state:

- source path;
- audience classification: user-facing, contributor-facing, internal/process,
  QA/eval, historical/archive, generated/cruft, or delete;
- action: migrate, rewrite, keep, update, archive, delete, or hand off;
- destination or retained location when applicable;
- rationale;
- verification status.

User-facing items must not be deleted until the canonical docs site has an
equivalent or intentionally updated replacement. Internal items may remain in
`hgahub/duumbi/docs/` only when their audience and freshness are explicit.

### Migration Before Deletion

`sites/docs/` deletion is valid only after useful content is represented in
`duumbi-web/docs`. A migrated page may be:

- a direct Starlight page when the content remains current;
- a rewritten section inside an existing canonical page;
- a new task-oriented guide;
- a reference page;
- a deliberate non-migration with rationale when the content is obsolete or
  duplicated.

The canonical docs must be current at merge time. Stale mdbook text such as old
phase status, unsupported install commands, old provider lists, and outdated
config schemas must not be copied as-is.

### Mixed `docs/` Routing

The source repo `docs/` directory remains allowed for internal, contributor,
architecture, automation, QA, and implementation support content. It should not
remain a hidden public docs backlog.

User-facing docs candidates include:

- `docs/provider-catalog.md`;
- `docs/modes/README.md`;
- `docs/modes/query-mode-spec.md`, rewritten for public reader guarantees;
- `docs/testing/phase12-walkthrough.md`, rewritten or split if useful because it
  is currently a user guide in Hungarian;
- architecture overview content appropriate for public readers.

Internal candidates include automation runbooks, coding conventions, source repo
architecture reference, e2e corpus, manual test protocols, and planning/research
notes. These must be kept, updated, archived, or deleted based on current use and
reviewed rationale.

### Canonical Docs Refresh

The canonical docs site must align with current DUUMBI public behavior:

- installation guidance must not advertise an unverified release channel;
- provider setup must prefer the current `[[providers]]` shape with `role`;
- provider docs must include MiniMax and the accepted #675 provider table;
- public docs must explain provider catalog behavior and link to catalog
  artifacts where useful;
- Query mode must be presented as the default read-only understanding surface;
- Agent and Intent must be explicit write-capable handoffs;
- CLI reference should reflect current commands and README framing;
- supported platforms should align with README boundaries;
- registry, module, JSON-LD, and architecture pages should avoid stale mdbook
  claims.

### Reader Experience

The docs should be task-oriented enough for a preview user:

- a new reader can find install/setup limitations quickly;
- a reader can understand Query before mutation;
- a reader can find CLI/config/reference pages from navigation;
- provider setup links to the provider catalog;
- diagrams explain non-obvious flows without becoming speculative product claims;
- community/support routes are visible from the docs content, not only from tiny
  social icons.

### Failure And Blocked States

If #687 remains unresolved during implementation, installation docs must state
the currently verified temporary/source install path and link to the release
issue rather than pretending `cargo install duumbi` works.

If `duumbi-web` is not writable in the implementation environment, the agent must
stop with a blocker because the accepted issue requires canonical docs changes.

If a legacy page cannot be safely classified, the agent must stop with targeted
questions or route the specific item as a follow-up only when it does not block
the preview docs reconciliation outcome.

## BDD Scenarios

Feature: DUUMBI documentation reconciliation

Scenario: Every legacy mdbook page is classified before deletion
Given the `hgahub/duumbi` repo contains pages under `sites/docs/src/`
When the implementation prepares to remove `sites/`
Then reviewable audit evidence lists every `sites/docs/src/` page
And each page has an audience classification, action, destination or retained
location, rationale, and verification status
And no useful user-facing page is deleted without an updated canonical docs
replacement or explicit obsolete rationale

Scenario: User-facing mdbook content is migrated to canonical docs
Given a useful mdbook page describes CLI, JSON-LD, module, architecture, or
workspace behavior
When the docs reconciliation is complete
Then the relevant behavior is represented in `duumbi-web/docs/src/content/docs/`
And the migrated content reflects current DUUMBI behavior
And the Starlight sidebar or page links make the content discoverable
And `sites/` can be removed without losing useful user-facing content

Scenario: Mixed source-repo docs are routed by audience
Given `hgahub/duumbi/docs/` contains provider catalog, Query mode docs,
automation runbooks, e2e corpus, manual test protocols, and research notes
When the docs audit is complete
Then user-facing docs are migrated or rewritten for the canonical docs site
And internal/process/QA content is kept, updated, archived, deleted, or handed
off with rationale
And stale or historical evaluation results are not presented as current preview
evidence unless refreshed or clearly labeled

Scenario: Provider setup docs match current provider UX
Given the README uses `[[providers]]` with `role`
And #675 defines the current provider catalog material
When a reader opens the canonical installation, config, or provider-catalog docs
Then provider examples use the current provider setup shape
And MiniMax appears in the provider guidance
And OpenRouter and `grok` are not presented as current v1 public provider
recommendations unless explicitly marked as legacy compatibility
And the provider setup page links to the provider catalog page

Scenario: Installation docs do not overclaim before the release issue is done
Given #687 is still open
When a reader opens the canonical installation docs
Then the docs do not claim `cargo install duumbi` is the verified preview install
path
And the docs describe only the currently verified install path or clearly mark
the release/install channel as pending #687
And the README and canonical docs do not contradict each other about the
supported path

Scenario: Query-first public framing is discoverable
Given Query mode is the default read-only understanding surface in README and
implemented DUUMBI behavior
When a reader uses the public docs to learn DUUMBI's workflow
Then the docs explain Query before mutation
And Query is described as read-only by default
And Agent and Intent are described as explicit write-capable handoffs
And provider setup remains routed through `/provider` rather than model-choice
maintenance

Scenario: Community and support paths are visible to docs readers
Given DUUMBI has Discord, GitHub Discussions, issue templates, and repo links
When the canonical docs site is built
Then a reader can find a dedicated community/support page or section
And it links to Discord, GitHub Discussions, source repo, issue templates, and
contribution/support expectations
And it is reachable from docs navigation or a prominent getting-started route

Scenario: Legacy docs references are removed after migration
Given migration coverage and audit evidence are complete
When the source repo cleanup is merged
Then `sites/` is removed
And README, CI, CODEOWNERS, PR templates, and active repo docs no longer tell
contributors to update `sites/docs/src/`
And `docs/.DS_Store` or `docs/.obsidian/` are absent if they existed
And ignore rules prevent these artifacts from being reintroduced

Scenario: Docs build validation proves the reader surface
Given canonical docs have been migrated and refreshed
When the docs-site build runs
Then Astro/Starlight build succeeds
And internal docs links referenced by migrated pages are valid under the build
And no stale mdbook-only paths are required for the public docs site

## Tasks

- Create or update a docs audit artifact that inventories `sites/docs/src/**`
  and `docs/**`.
- Classify each item with audience, action, destination, rationale, and
  verification status.
- Prepare `duumbi-web/docs` migration pages and sidebar/navigation updates.
- Refresh installation, provider setup, config, Query-first, CLI/reference,
  community/support, and related docs pages.
- Add diagrams or infographics where they improve understanding of pipeline,
  intent, provider/catalog, or registry/module flows.
- Validate `duumbi-web/docs` with the repo's docs build.
- Clean `hgahub/duumbi` by removing migrated `sites/` content and stale
  references to mdbook docs.
- Curate or archive source-repo `docs/` material according to the audit.
- Verify no active docs, README, CI, PR template, or CODEOWNERS surface points
  contributors at `sites/docs/src/`.
- Link implementation evidence back to #686.

## Checks

- Stage 10 implementation PR evidence includes a complete docs audit table or
  equivalent durable artifact.
- `duumbi-web/docs` build succeeds with `npm run build`.
- Source-repo checks relevant to docs cleanup pass.
- Search evidence shows no active references to `sites/docs`, mdbook, stale
  install claims, stale provider examples, or accidental docs cruft remain
  outside accepted historical/archive context.
- Canonical docs pages for install, config/provider setup, provider catalog,
  Query-first workflow, CLI/reference, and community/support are reachable.
- BDD scenarios above are mapped to implementation tests, build commands, search
  checks, screenshots or preview review, and PR evidence in the technical spec.
- Stage 11 review verifies both repo scopes before recommending closure.

## Open Questions

None blocking.

Non-blocking implementation risk: #687 is still open, so installation docs must
avoid final install claims until that release work has evidence.

## Sources

- Issue #686: https://github.com/hgahub/duumbi/issues/686
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/686#issuecomment-4701701134
- Stage 4 triage comment:
  https://github.com/hgahub/duumbi/issues/686#issuecomment-4696865520
- Related issue #675: https://github.com/hgahub/duumbi/issues/675
- Related issue #687: https://github.com/hgahub/duumbi/issues/687
- Related issue #689: https://github.com/hgahub/duumbi/issues/689
- Source repo instructions: `AGENTS.md`
- Source repo architecture: `docs/architecture.md`
- Source repo coding conventions: `docs/coding-conventions.md`
- Current provider catalog source: `docs/provider-catalog.md`
- Query docs source: `docs/modes/README.md`
- Query mode spec source: `docs/modes/query-mode-spec.md`
- Legacy mdbook summary: `sites/docs/src/SUMMARY.md`
- Canonical docs config:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/astro.config.mjs`
- Canonical docs installation page:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/getting-started/installation.md`
- Canonical docs config reference:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/reference/config.md`
- Vault PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Vault glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Agentic Development Runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Public Docs as Product Interface:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Public Docs as Product Interface.md`
- Static Website and Docs Publishing:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Static Website and Docs Publishing.md`
