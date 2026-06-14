# DUUMBI-686: Reconcile Legacy Docs With Canonical Docs Site - Technical Specification

## Implementation Objective

Implement the approved product behavior in `specs/DUUMBI-686/PRODUCT.md` by
reconciling DUUMBI's legacy source-repo docs with the canonical public docs site
without losing useful content or publishing stale claims.

The implementation must coordinate two repositories:

- `hgahub/duumbi`: audit legacy docs, curate internal docs, remove migrated
  `sites/` content, and remove active mdbook references.
- `hgahub/duumbi-web`: migrate and refresh public docs under
  `docs/src/content/docs/`, update navigation, add provider/community surfaces,
  and validate the Starlight build.

Related to #686. This is a technical-specification artifact only. The execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Docs implementation agents working across `hgahub/duumbi` and
  `hgahub/duumbi-web`.
- Reviewer agents checking migration coverage, source-truth discipline, stale
  docs removal, and build/link evidence.
- Tester agents validating docs build, search checks, and reader smoke paths.
- Stage 9 technical reviewers checking implementability and resource policy.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/686
- Product spec: `specs/DUUMBI-686/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/706
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/686#issuecomment-4701701134
- Stage 7 AI gate decision:
  https://github.com/hgahub/duumbi/issues/686#issuecomment-4702119560
- Stage 7 product spec approval decision:
  https://github.com/hgahub/duumbi/issues/686#issuecomment-4702120134
- Stage 4 triage comment:
  https://github.com/hgahub/duumbi/issues/686#issuecomment-4696865520
- Related provider catalog issue: https://github.com/hgahub/duumbi/issues/675
- Related release/install issue: https://github.com/hgahub/duumbi/issues/687
- Related scaled eval issue: https://github.com/hgahub/duumbi/issues/689
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Active vault PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active vault glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Agentic Development Runbook:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Docs policy dots:
  - `Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Public Docs as Product Interface.md`
  - `Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Static Website and Docs Publishing.md`

Relevant source facts verified for Stage 8:

- Issue #686 is open with `accepted` and `needs-spec` labels.
- The Stage 5 decision says `Decision: Accept`, `Next state: Spec Needed`, and
  `Remaining open questions: none`.
- Product spec PR #706 was a one-file spec-only PR for
  `specs/DUUMBI-686/PRODUCT.md`.
- PR #706 passed PR checks, had Codex self-review with no blocking findings, and
  was merged by the Stage Approval workflow with merge SHA
  `90d4c9ed493dffeab1db397f6b674cc5e79c87bf`.
- The Stage 7 AI Gate Decision approved the product spec and recorded next state
  `Technical Spec Needed`.
- Issue #686 now has `product-spec-approved` and `needs-tech-spec` labels.
- The current GitHub token cannot read Project V2 fields because it lacks
  `read:project`; labels and comments are the verified workflow state available
  during drafting.
- `sites/docs/src/SUMMARY.md` lists legacy mdbook pages for introduction,
  getting started, CLI reference, JSON-LD format, module system, architecture,
  and contributing.
- `sites/docs/book.toml` exists and identifies the legacy mdbook surface.
- `docs/` contains mixed internal and user-facing material:
  - `docs/provider-catalog.md`
  - `docs/modes/README.md`
  - `docs/modes/query-mode-spec.md`
  - `docs/testing/phase12-walkthrough.md`
  - `docs/automation/*`
  - `docs/e2e/corpus/*`
  - `docs/e2e/results/*20260331*`
  - manual test protocols under `docs/testing/`
- `docs/provider-catalog.md` contains the accepted #675 provider table and
  catalog behavior, including MiniMax and accepted direct-provider keys.
- `docs/modes/query-mode-spec.md` says Query mode was delivered in v1 and is
  read-only by contract.
- `README.md` already uses `[[providers]]` with `role`, frames Query as
  read-only first, lists MiniMax, and documents `cargo install --path .`.
- `README.md` links to Discord and the public docs site.
- `.gitignore` already ignores `.DS_Store` and `docs/.obsidian/`.
- No `docs/.DS_Store` file or `docs/.obsidian/` directory was found in the
  inspected worktree.
- `.github/PULL_REQUEST_TEMPLATE.md` still tells reviewers to update docs under
  `sites/docs/src/`.
- `.github/CODEOWNERS` contains `/sites/docs/ @hgahub`.
- `hgahub/duumbi-web/docs/src/content/docs/` currently has getting-started,
  guides, registry, and reference pages, but no provider-catalog page.
- `hgahub/duumbi-web/docs/astro.config.mjs` defines Starlight navigation and
  GitHub/Discord social links.
- `hgahub/duumbi-web/docs/package.json` defines `npm run build` as
  `astro build`.
- `hgahub/duumbi-web/docs/src/content/docs/getting-started/installation.md`
  currently advertises `cargo install duumbi` and an old `[llm]` config table.
- `hgahub/duumbi-web/docs/src/content/docs/reference/config.md` currently
  includes `[llm]` as the primary example and older provider names.
- `hgahub/duumbi-web/docs/src/content/docs/getting-started/introduction.md`
  currently lists Anthropic, OpenAI, Grok, and OpenRouter for multi-provider
  support.
- `hgahub/duumbi-web/docs/public/model-catalog/v1/` contains static catalog
  artifacts from #675, but no docs content page links them as provider catalog
  documentation.

Assumptions for implementation:

- The implementation agent can obtain writable access to `hgahub/duumbi-web`.
  If not, Stage 10 must stop because the accepted scope requires canonical docs
  changes.
- A committed docs audit artifact should live in `hgahub/duumbi`, for example
  `docs/duumbi-686-docs-audit.md`, unless the implementation agent chooses a
  clearer existing docs/audit location during Stage 10.
- Final implementation may need two coordinated implementation PRs, one per
  repository, both linked from #686.
- The docs site build is the canonical automated proof for Starlight navigation,
  frontmatter, and internal links.
- No live LLM calls are required to verify a documentation reconciliation issue.
- Diagrams should be source-controlled and buildable through the docs site,
  preferably as Mermaid in Markdown/MDX or lightweight static assets already
  compatible with Astro/Starlight.

## Affected Areas

Expected `hgahub/duumbi` Stage 10 areas:

- Legacy docs tree:
  - `sites/docs/`
- Source-repo docs:
  - `docs/provider-catalog.md`
  - `docs/modes/README.md`
  - `docs/modes/query-mode-spec.md`
  - `docs/testing/phase12-walkthrough.md`
  - `docs/automation/*`
  - `docs/e2e/*`
  - `docs/testing/*`
  - `docs/architecture.md`
  - `docs/coding-conventions.md`
  - new or updated docs audit artifact
- Repo references:
  - `README.md`
  - `.github/PULL_REQUEST_TEMPLATE.md`
  - `.github/CODEOWNERS`
  - `.github/workflows/*` if any workflow references `sites/docs` or mdbook
  - `.gitignore` only if Stage 10 finds missing cruft ignore coverage

Expected `hgahub/duumbi-web` Stage 10 areas:

- Starlight docs content:
  - `docs/src/content/docs/getting-started/installation.md`
  - `docs/src/content/docs/getting-started/introduction.md`
  - `docs/src/content/docs/getting-started/quickstart.md`
  - `docs/src/content/docs/getting-started/first-ai-mutation.md`
  - `docs/src/content/docs/reference/config.md`
  - `docs/src/content/docs/reference/cli.md`
  - new or updated provider catalog page
  - new or updated Query-first page/section
  - new or updated community/support page/section
  - migrated or refreshed CLI, JSON-LD, module, architecture, workspace, and
    contributor pages as warranted by the audit
- Docs navigation/config:
  - `docs/astro.config.mjs`
- Static assets or diagrams:
  - `docs/src/assets/*` or Markdown/MDX Mermaid blocks when appropriate
- Existing model catalog static artifacts:
  - `docs/public/model-catalog/v1/model-catalog.v1.json`
  - `docs/public/model-catalog/v1/model-catalog.v1.sha256`
- Docs build config:
  - `docs/package.json`
  - `docs/package-lock.json` only if dependency changes are truly required

Areas expected not to change:

- DUUMBI compiler, parser, graph, runtime, registry, provider, Query, Intent, or
  Studio implementation code, unless Stage 10 proves a narrow docs-reference
  update is needed.
- Provider catalog generator behavior from #675.
- Release artifacts or install-channel implementation from #687.
- Scaled intent eval implementation from #689.
- Workflow stage approval semantics.
- Spec files for other issues.

## Technical Approach

### Repository Coordination

Use #686 as the umbrella execution record. The implementation agent should keep
both repos traceable to the same issue:

- `hgahub/duumbi` PR: audit artifact, legacy docs cleanup, internal docs
  curation, mdbook reference removal, and source repo evidence.
- `hgahub/duumbi-web` PR: canonical docs migration, content refresh,
  navigation/sidebar updates, diagrams, and docs build evidence.

If the implementation environment cannot edit `hgahub/duumbi-web`, stop and
record a blocker. A source-repo-only cleanup would not satisfy the accepted
issue.

### Audit Artifact

Create a durable audit artifact before deleting `sites/`. Recommended shape:

```markdown
# DUUMBI-686 Docs Audit

## Summary

## Routing Rules

## `sites/docs/src` Inventory

| Source path | Audience | Action | Destination | Rationale | Verification |
| --- | --- | --- | --- | --- | --- |

## `docs` Inventory

| Source path | Audience | Action | Destination | Rationale | Verification |
| --- | --- | --- | --- | --- | --- |

## Deletion Preconditions

## Follow-Up Items
```

Classify at least:

- every page listed by `sites/docs/src/SUMMARY.md`;
- every Markdown file under `docs/`;
- `docs/e2e/corpus/*`;
- `docs/e2e/intents/*`;
- `docs/e2e/results/*`;
- accidental or generated cruft if present.

The audit may group repetitive fixture files when the group has the same
audience/action/rationale, but grouped rows must still prove full coverage.

### Migration Rules

For `sites/docs/`:

- migrate useful public content before removing the source tree;
- rewrite stale content to current behavior;
- prefer canonical Starlight pages over mdbook structure;
- do not preserve obsolete phase/status claims;
- after migration, remove `sites/`, `sites/docs/book.toml`, and active source
  references to mdbook docs.

For source-repo `docs/`:

- move public reader content to `duumbi-web/docs`;
- keep internal architecture, automation, and QA content in `duumbi` only when
  useful and current;
- archive or mark historical material when it is useful but not current;
- remove accidental cruft and stale generated directories when present;
- do not publish internal agent workflow material as public docs without
  rewriting it for public readers.

### Canonical Docs Updates

Use the Starlight docs structure under `duumbi-web/docs/src/content/docs/`.
Expected page decisions:

- Add a provider catalog reference page using current #675 provider material.
- Update provider setup references to use `[[providers]]` with `role`.
- Link provider setup pages to the provider catalog page and static catalog
  artifacts when relevant.
- Remove unsupported `cargo install duumbi` as the recommended install path
  while #687 remains open. Use the README's verified source install path or a
  clear preview/install-pending note.
- Add Query-first guidance to getting-started or guide content. Keep the
  guarantee factual: Query is read-only by default; Agent and Intent are
  explicit write-capable handoffs.
- Refresh CLI reference against the current `README.md`, `AGENTS.md`, and
  `src/cli` command surface.
- Add community/support content reachable from navigation or the getting-started
  flow. Include Discord, GitHub Discussions, issue templates, source repo, and
  expected support/contribution behavior.
- Add diagrams where they are useful and maintainable:
  - compiler pipeline;
  - Query -> Agent/Intent flow;
  - provider catalog/setup flow;
  - registry/module flow.

Use conservative claims. Public docs should describe shipped or verified
preview behavior, not future architecture unless explicitly labeled.

### Source Repo Cleanup

After migration coverage is in place:

- remove `sites/`;
- remove or update `.github/CODEOWNERS` entries for `/sites/docs/`;
- update `.github/PULL_REQUEST_TEMPLATE.md` so docs impact points to the
  canonical docs site or source-repo internal docs as appropriate;
- search and update active README, workflow, docs, and source references to
  mdbook or `sites/docs/src/`;
- remove `docs/.DS_Store` and `docs/.obsidian/` if present;
- keep `.gitignore` coverage for `.DS_Store` and `docs/.obsidian/`.

### Cross-Repo PR Evidence

Implementation PR descriptions must include:

- link to #686;
- link to product and technical specs;
- audit artifact location;
- migration summary;
- build/check commands and results;
- follow-up items that do not block the accepted preview gate;
- clear note that #686 remains open until both repo scopes and Stage 12 closure
  evidence are complete.

Avoid spec-only closing language in spec PRs. Implementation PR language should
also be deliberate because Stage 12 owns final issue closure.

## Invariants

- `duumbi-web/docs` remains the canonical public docs source.
- `hgahub/duumbi/docs` remains internal/source-repo documentation unless a file
  is deliberately migrated or rewritten for public docs.
- Useful user-facing content is not deleted before migration coverage exists.
- Public docs do not claim unverified install, provider, config, or platform
  support.
- Query remains read-only in docs framing; Agent/Intent remain explicit write
  paths.
- Provider setup docs keep credentials under user control and do not ask users
  to manage default model IDs unless documenting compatibility.
- `sites/` is absent only after mdbook migration coverage and source reference
  cleanup are complete.
- No implementation code or behavior changes are introduced under this docs
  reconciliation issue unless a narrow docs-support change is explicitly
  justified and reviewed.
- Stage 10 Ralph cycles stay inside the approved product and technical specs.
- Greptile is not invoked for spec PRs.

## BDD-To-Test Mapping

| Product BDD scenario | Verification evidence |
| --- | --- |
| Every legacy mdbook page is classified before deletion | Audit artifact includes all `sites/docs/src/SUMMARY.md` entries; reviewer runs `find sites/docs/src -type f` before cleanup or reviews deletion diff plus audit rows. |
| User-facing mdbook content is migrated to canonical docs | `duumbi-web` PR diff includes refreshed pages/sections and sidebar/navigation wiring; audit rows map mdbook sources to destinations; `npm run build` succeeds. |
| Mixed source-repo docs are routed by audience | Audit artifact covers all `docs/**` files or grouped fixtures/results; `duumbi` PR diff keeps/updates/archives/deletes according to audit; reviewer checks stale e2e result treatment against #689. |
| Provider setup docs match current provider UX | Search checks in `duumbi-web/docs/src/content/docs` verify provider pages use `[[providers]]` with `role`, include MiniMax, link provider catalog, and avoid stale OpenRouter/Grok guidance except legacy notes. |
| Installation docs do not overclaim before release issue is done | Search checks verify `cargo install duumbi` is absent as a verified recommended path while #687 is open, or is explicitly marked pending/conditional; README and docs install guidance are consistent. |
| Query-first public framing is discoverable | Docs diff includes Query-first page/section; search checks verify read-only Query wording and explicit Agent/Intent handoff wording; docs build succeeds. |
| Community and support paths are visible to docs readers | `duumbi-web` diff includes community/support page or section, navigation link, and links to Discord, GitHub Discussions, issue templates, and source repo; docs build succeeds. |
| Legacy docs references are removed after migration | `rg -n "sites/docs|mdbook|mdBook|book.toml|docs/src" README.md docs .github .gitignore` returns only accepted historical/audit references; `sites/` is deleted in `duumbi` implementation PR. |
| Docs build validation proves the reader surface | `cd /Users/heizergabor/space/hgahub/duumbi-web/docs && npm run build` succeeds; optional local preview smoke confirms key pages render. |

## Live E2E Plan

This issue changes documentation, not LLM-backed runtime behavior. The live E2E
path is therefore a docs-site and CLI-claim validation path.

Canonical interface:

- Public docs site through local Astro/Starlight build and preview.
- CLI help/output only as evidence for documentation claims.

Credentials and external services:

- Required credentials: none.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Network dependency: none for local build when dependencies are already
  installed. If dependencies are missing, Stage 10 may need approval for package
  installation according to the execution environment.

Commands:

```bash
# In hgahub/duumbi-web/docs
npm run build

# Optional local preview smoke after a successful build
npm run preview -- --host 127.0.0.1 --port 4321
```

Manual/browser smoke checks after preview starts:

- `/getting-started/installation/`
- provider catalog page
- Query-first guide or getting-started section
- `/reference/config/`
- `/reference/cli/`
- community/support page or section

Source-repo claim checks:

```bash
# In hgahub/duumbi
rg -n "sites/docs|mdbook|mdBook|book.toml" README.md docs .github .gitignore
rg -n "cargo install duumbi|\\[llm\\]|OpenRouter|Grok|grok" /Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs
find docs -maxdepth 2 -type d -name ".obsidian" -print
find docs -maxdepth 2 -type f -name ".DS_Store" -print
```

Pass/fail criteria:

- Docs build succeeds.
- Preview pages render without obvious navigation/frontmatter failures.
- Search checks show no stale references except accepted historical/audit
  context.
- No live provider or LLM calls are needed.
- Audit evidence covers all legacy source paths.

## Ralph Cycle Protocol

Each Stage 10 cycle must:

1. summarize the current state and remaining unmet requirements;
2. propose one bounded implementation goal;
3. list intended repository, file areas, and commands;
4. estimate resource use and risk;
5. check whether the resource gate requires human approval;
6. implement only the approved or resource-permitted goal;
7. run the agreed checks;
8. report evidence, failures, and remaining gaps;
9. stop only if requirements are met, a blocker appears, the expected external
   LLM cost of the next cycle exceeds USD 1, scope changes, or a product or
   architecture decision is needed. Iteration count is not a stop condition.

## Cycle Budget

- Default cycle size: one bounded docs reconciliation goal per cycle.
- Max files or modules per cycle: one repository and one coherent docs area,
  such as audit artifact, provider docs, Query docs, mdbook migration group, or
  source-repo cleanup. A cross-repo cycle is allowed only when the same small
  migration requires source and destination edits together.
- Expected command budget per cycle:
  - lightweight search/list commands with `rg`, `find`, `git diff`;
  - targeted docs build or Markdown checks when the cycle affects docs site
    rendering;
  - no full Rust test suite unless implementation changes Rust behavior, which
    is not expected.
- Human approval required only when the cycle will use an external LLM with
  expected cost above USD 1, edits outside the approved repos or writable roots,
  exceeds approved scope, adds risky dependencies, performs irreversible
  operations, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is covered by the Codex
  App subscription and never triggers the gate.
- Expected external LLM use for this issue: none.
- No autonomous batch cap: cycles continue until completion, blocker, gate
  breach, or scope change.
- Stop and ask for human guidance when:
  - `duumbi-web` access is unavailable;
  - #687 status changes and creates a release/install documentation decision;
  - a legacy page cannot be classified safely;
  - migration requires new dependencies or a major docs-site restructuring;
  - build failures indicate broken docs framework configuration outside the
    approved docs reconciliation scope.

## Task Breakdown

1. Establish Stage 10 workspace access for both `hgahub/duumbi` and
   `hgahub/duumbi-web`.
2. Create the docs audit artifact in `hgahub/duumbi`.
3. Inventory `sites/docs/src/**` from `SUMMARY.md` and filesystem output.
4. Inventory `docs/**`, grouping repetitive fixture/corpus/result files only
   when coverage remains explicit.
5. Classify every item with audience, action, destination, rationale, and
   verification.
6. Add or update `duumbi-web` provider catalog docs using current #675 material.
7. Refresh `duumbi-web` installation/config/provider setup docs, coordinating
   install claims with #687.
8. Add or update Query-first public docs and link them from getting-started or
   guides.
9. Migrate useful mdbook CLI, JSON-LD, module, workspace, architecture, and
   contributing content into canonical docs pages or refreshed sections.
10. Add community/support docs and navigation.
11. Add diagrams or infographics where they improve reader understanding.
12. Build `duumbi-web/docs` and fix docs-site build/link errors.
13. Clean `hgahub/duumbi`: remove `sites/`, update CODEOWNERS/PR template/README
   references, remove cruft if present, and curate internal docs per audit.
14. Run source-repo search checks for legacy references and stale claims.
15. Open coordinated implementation PRs with evidence linked to #686.

## Verification Plan

Required automated or command evidence:

- `npm run build` from `hgahub/duumbi-web/docs`.
- `rg --files` or `find` evidence for removed `sites/` in `hgahub/duumbi`.
- Search checks for legacy mdbook references in active source-repo files.
- Search checks for stale install/provider/config claims in canonical docs.
- Search or review evidence that provider catalog, Query-first, and
  community/support pages are linked from navigation or relevant pages.
- `git diff --stat` and PR file list evidence proving docs-only scope unless a
  narrow justified non-docs metadata update is included.

Required review evidence:

- Review of the audit artifact against source filesystem coverage.
- Review of canonical docs pages for public-reader clarity and current behavior.
- Review that #687 and #689 dependencies are handled as coordination/follow-up,
  not silently completed by docs text.
- Stage 11 review should inspect both repo PRs, CI/build state, and all linked
  evidence before recommending closure.

Optional manual evidence:

- Local Astro preview screenshot or notes for installation, provider catalog,
  Query-first, CLI/config, and community/support pages.
- Browser click-through of navigation from the docs landing page to each new or
  migrated page.

## Completion Criteria

Before implementation PR review:

- Product spec BDD scenarios have concrete evidence in PR descriptions, build
  logs, search output, or screenshots/preview notes.
- `sites/` is removed from `hgahub/duumbi` only after migration coverage is
  present in `hgahub/duumbi-web`.
- The audit artifact covers all legacy source paths.
- Canonical docs build succeeds.
- Install docs do not overclaim while #687 remains open.
- Provider docs include MiniMax and current #675 provider catalog material.
- Query-first docs are present and accurate.
- Community/support docs are discoverable.
- Active repo references no longer point contributors to `sites/docs/src/`.
- No implementation code, provider/runtime behavior, release artifacts, or eval
  harness changes are included unless explicitly justified within scope.

Before Stage 12 closure:

- Both repo scopes are merged or otherwise verified complete.
- The issue contains links to the merged implementation PRs and build evidence.
- Any follow-up items are non-blocking and explicitly scoped.

## Failure And Escalation

- If `duumbi-web` cannot be edited, stop and mark the issue blocked; do not
  merge source-repo cleanup alone.
- If docs build fails because of existing unrelated docs-site breakage, record
  evidence and decide whether a narrow docs-site repair is inside scope.
- If #687 lands during implementation, update install docs to the newly verified
  install path only after evidence is available.
- If a page's correct audience/action is unclear and affects deletion or public
  migration, ask a targeted question rather than deleting or publishing it.
- If implementation touches runtime code, provider behavior, release workflows,
  or scaled eval behavior, stop unless the change is explicitly required by the
  accepted docs reconciliation scope.
- If the next cycle needs external LLM calls above USD 1, risky dependency
  changes, broad restructuring, or edits outside authorized repositories, request
  human approval before continuing.

## Open Questions

None blocking.

Non-blocking risks to watch during Stage 10:

- #687 may change the correct installation docs wording while implementation is
  in progress.
- Some historical e2e result material may be useful as archive context but must
  not be presented as current preview evidence before #689.
- The exact docs-page taxonomy in `duumbi-web` should follow the existing
  Starlight information architecture unless Stage 10 proves a navigation change
  is necessary.
