# DUUMBI-732: Docs Truth Reconciliation For v0.4.0-preview Remaining Items

## Summary

Define the remaining product behavior for public documentation truth after the
#686 legacy-docs reconciliation, #687 preview release, and #689 scaled eval work
have landed.

Issue #732 is not a second pass over the removed `sites/docs/` tree. It covers
the public claims that still need to be reconciled for the shipped
`v0.4.0-preview` reality: install channel, platform matrix, provider/model
wording, registry claims, status and roadmap visibility, and a release-time
check or checklist item that prevents stale install instructions from being
published again.

Related to #732. This is a product-specification artifact only. The execution
issue must remain open for Stage 7 Product Spec Review, Stage 8 technical
specification, Stage 9 Technical Spec Review, Stage 10 implementation, Stage 11
review, and Stage 12 closure.

## Problem

DUUMBI has a verified preview release, but several public surfaces still carry
pre-release assumptions or incomplete launch framing:

- The canonical docs site installation page still lists macOS Intel
  `x86_64-apple-darwin` as a required preview archive, while the actual
  `v0.4.0-preview` release and the source repo README say macOS Intel is not
  included in this developer preview.
- The public website still shows `cargo install duumbi` in the "Running in 60
  seconds" component, and older blog posts use the same unavailable crates.io
  install command.
- The launch blog still lists `Grok` and `OpenRouter` as normal multi-LLM
  support, while current provider guidance says `xai` is the canonical key for
  xAI/Grok and OpenRouter is excluded from the V1 direct-provider catalog.
- The public website structured data still reports `softwareVersion: 0.1.0`,
  which does not match the shipped `v0.4.0-preview` release.
- The vault roadmap calls for a public "Status and roadmap" page with honest
  Delivered, Partial, and Research labels, but the current docs sidebar has no
  status or roadmap page.
- There is no verified guard that checks public install instructions against
  the latest release assets before publishing or release completion.

The product risk is direct: a user may follow a public claim, choose an
unsupported platform or install command, or misread roadmap research as shipped
product. That damages trust more than a missing feature would.

## Outcome

When this work is done:

- Public install guidance consistently names GitHub Releases as the
  `v0.4.0-preview` install channel.
- Public install guidance does not present crates.io, Homebrew, apt, winget,
  signed installers, notarized packages, or auto-update as available preview
  distribution channels.
- Public platform matrices match the actual release assets:
  - macOS Apple Silicon: `aarch64-apple-darwin`
  - Linux x86_64: `x86_64-unknown-linux-gnu`
  - Linux ARM64: `aarch64-unknown-linux-gnu`
  - macOS Intel: no preview archive; source build fallback
  - Windows: no preview archive for this release path unless later evidence is
    added before implementation
- Public install instructions include checksum verification and keep the
  extracted release directory together so `runtime/` remains beside the CLI.
- Public provider/model docs and marketing copy align with the provider catalog:
  normal setup uses `duumbi provider ...` and `[[providers]]`, model selection
  is internal, MiniMax is included, `xai` is the canonical xAI key, `grok` is
  compatibility-only legacy wording, and OpenRouter is not presented as a V1
  direct-provider target.
- Public registry claims are accurate for current shipped behavior and do not
  imply unavailable package contents, private registry operations, or future
  graph-aware registry capabilities unless clearly labeled as roadmap material.
- A Status and roadmap page exists in the public docs, is discoverable from
  navigation, and uses explicit Delivered, Partial, Research, or Planned labels
  sourced from the active vault roadmap and verified GitHub evidence.
- The public website landing, blog, comparison pages, docs home, installation,
  quickstart, provider, registry, CLI, and config pages have been audited for
  stale launch claims.
- Reviewable audit evidence records which public surfaces were checked, which
  claims changed, and which claims are intentionally left as historical blog
  context.
- A release-time verification path exists. It may be automated CI or a required
  release-checklist script, but it must compare install instructions with the
  latest release metadata well enough to catch an unavailable primary install
  command, a stale platform matrix, missing checksums, or missing required
  release assets.
- Public docs and website builds pass after the changes.
- The execution issue remains open after spec PRs and implementation PRs until
  Stage 12 verifies merged evidence.

## Scope

### In Scope

- Audit public-facing `hgahub/duumbi-web` surfaces, including:
  - docs under `docs/src/content/docs/**`
  - docs navigation in `docs/astro.config.mjs`
  - landing-page components under `src/components/**`
  - website pages under `src/pages/**`
  - blog posts under `src/content/blog/**`
  - comparison pages under `src/pages/compare/**`
- Correct stale install commands, platform matrices, checksum guidance, provider
  lists, model examples, registry status language, release version metadata, and
  preview support boundaries.
- Add or update a public Status and roadmap docs page using the active roadmap
  map as source material and GitHub issue/PR/release evidence for delivered
  labels.
- Add navigation from the docs sidebar and at least one reader-facing entry
  point to the Status and roadmap page.
- Add reviewable audit evidence for public-claim reconciliation.
- Add a CI check, script, or release checklist item that verifies install
  instructions against release metadata before publish/release completion.
- Update source-repo documentation only when needed to keep internal audit,
  release checklist, or README guidance aligned.
- Preserve historical blog posts only if they are clearly marked as historical
  where stale commands or provider claims would otherwise mislead readers.

### Explicitly Out Of Scope

- Publishing DUUMBI to crates.io.
- Adding new release artifacts, package managers, signed installers,
  notarization, or auto-update.
- Changing DUUMBI compiler, runtime, registry, provider, Query, Agent, Intent,
  or Studio behavior.
- Reopening #686 legacy `sites/docs/` migration work.
- Rewriting the whole marketing site beyond truth reconciliation.
- Publishing speculative roadmap promises as shipped product behavior.
- Creating or maintaining live roadmap synchronization from Obsidian.
- Stage 10 implementation code, docs edits, scripts, generated artifacts, or
  Ralph cycles during this specification stage.
- Product spec approval, technical spec approval, implementation merge, or
  Stage 12 closure.

## Constraints And Assumptions

Facts:

- Issue #732 is open and currently labeled `accepted` and `needs-spec`.
- The Stage 5 Human Acceptance Decision comment dated 2026-06-19 records
  `Decision: Accept`, `Next state: Spec Needed`, and no remaining open
  questions.
- Issue #686 is closed as completed.
- Issue #687 is closed as completed.
- Issue #689 is closed as completed.
- The `v0.4.0-preview` tag exists and points to commit
  `506837abb2d49d33116e8d5f53af8ad508235248`.
- GitHub Releases has a published, non-draft prerelease named
  `v0.4.0-preview`.
- The `v0.4.0-preview` release currently has these assets:
  - `checksums.txt`
  - `duumbi-v0.4.0-preview-aarch64-apple-darwin.tar.gz`
  - `duumbi-v0.4.0-preview-x86_64-unknown-linux-gnu.tar.gz`
  - `duumbi-v0.4.0-preview-aarch64-unknown-linux-gnu.tar.gz`
- The release body says macOS Intel is not included in this developer preview.
- The source repo README agrees that macOS Intel is not included in this
  developer preview and documents GitHub Release archives plus checksums.
- The canonical docs installation page currently lists macOS Intel
  `x86_64-apple-darwin` as a required preview target.
- The public website `GetStarted` component currently shows
  `cargo install duumbi`.
- The public blog posts inspected currently show `cargo install duumbi` in
  getting-started sections.
- The launch blog currently lists `Grok` and `OpenRouter` as normal multi-LLM
  support.
- Current provider docs list direct providers through the V1 provider catalog,
  include MiniMax, identify `xai` as canonical for xAI/Grok, and exclude
  OpenRouter from the V1 direct-provider catalog.
- The public website structured data currently reports `softwareVersion:
  0.1.0`.
- The active roadmap map says M0 `v0.4.0-preview` should have honest docs and
  names docs truth reconciliation as an M0 release-gate item.
- The active runbook requires combined product and technical specs, clean
  Stage 7 and Stage 9 AI gates, no Greptile on spec PRs, and resource-gated
  Ralph cycles after Ready for Build.

Assumptions:

- `hgahub/duumbi-web` remains the canonical public website and docs repository.
- The Stage 10 implementation environment can obtain writable access to
  `hgahub/duumbi-web`; if it cannot, implementation must stop with a blocker or
  create the smallest coordinated handoff needed for that repository.
- A public Status and roadmap page should be manually curated from the active
  roadmap plus GitHub evidence, not generated directly from Obsidian at runtime.
- Historical blog posts may remain published, but if they contain stale commands
  or provider claims, the reader must see an update note or the stale section
  must be refreshed.
- A release checklist item is acceptable only if it is concrete, versioned, and
  produces reviewable evidence. A vague human reminder is not enough.

Constraints:

- Do not claim crates.io availability until a separate crates.io release is
  implemented and verified.
- Do not claim macOS Intel, Windows, package-manager, signing, notarization, or
  auto-update support for `v0.4.0-preview` unless new release evidence exists
  before implementation.
- Do not expose private vault-only notes as public roadmap copy without
  rewriting them for a public reader.
- Do not let the public roadmap page become a live execution tracker; GitHub
  Issues and Project state remain the execution source of truth.
- Do not use GitHub issue-closing references in spec-only PRs for this issue.

## Decisions

- **Decision:** Use GitHub Releases as the documented preview install channel.
  **Evidence:** #687 is complete, the `v0.4.0-preview` prerelease exists, and
  the release assets include platform archives and checksums.

- **Decision:** Treat crates.io and package-manager distribution as unavailable
  for this issue.
  **Evidence:** The verified `v0.4.0-preview` release is GitHub Releases based,
  and #687 explicitly kept crates.io publishing out of preview scope.

- **Decision:** Treat macOS Intel as unsupported by the current preview archive
  path.
  **Evidence:** The release body and current README say macOS Intel is not
  included, and the release assets do not include an
  `x86_64-apple-darwin` archive.

- **Decision:** Require a public Status and roadmap page, but make it
  reader-facing and label-based rather than a raw vault mirror.
  **Evidence:** The docs truth reconciliation source note calls for
  Delivered, Partial, and Research labels sourced from the roadmap map; the PRD
  says GitHub Project remains the current execution tracker and Obsidian stores
  durable knowledge.

- **Decision:** Keep the product work cross-repo-aware.
  **Evidence:** Public stale claims live in `hgahub/duumbi-web`, while release
  evidence, specs, and internal workflow docs live in `hgahub/duumbi`.

## Behavior

### Public Claim Audit

The implementation must inspect all public DUUMBI surfaces that a preview user
can reasonably reach:

- docs pages and docs navigation;
- landing page sections and structured metadata;
- blog posts;
- comparison pages;
- public README and release-linked install material when alignment is needed.

For each inspected surface, implementation evidence must record:

- path or URL;
- claim category: install, platform, provider/model, registry, roadmap/status,
  version, support boundary, or historical context;
- whether the current claim is correct, stale, intentionally historical, or
  out of scope;
- change made or rationale for no change;
- verification evidence.

### Install And Platform Claims

Default state:

- The primary preview install path is the GitHub Release archive for the user's
  supported target.
- Source build is the fallback for contributors and unsupported targets.

Success state:

- A reader can identify the right archive for a supported target, download it,
  verify checksums, put it on `PATH`, and run `duumbi --version`.
- The reader is not told that `cargo install duumbi` from crates.io is available.

Unsupported state:

- macOS Intel, Windows, package managers, signed installers, notarized packages,
  and auto-update are explicitly absent from `v0.4.0-preview` unless new release
  evidence exists.
- Unsupported users are routed to source build or future roadmap wording, not a
  missing binary.

Error state:

- If release metadata cannot be checked during implementation, the agent must
  not invent asset availability. It must keep the last verified evidence in the
  spec implementation report and mark live verification unavailable.

### Provider And Model Claims

Provider setup must remain `/provider`-first and `[[providers]]`-based. Public
copy must not ask normal users to maintain default model IDs. Public provider
lists must align with the current provider catalog and distinguish direct
providers from legacy compatibility or aggregator surfaces.

### Registry Claims

Public registry docs may describe the current registry server, CLI commands,
authentication, and self-hosting only when the behavior exists in the current
docs/source context. Future graph-aware registry, session sync, or roadmap
registry evolution belongs on the roadmap page with non-shipped labels.

### Status And Roadmap Page

The Status and roadmap page must:

- be discoverable from docs navigation;
- explain that it is public product orientation, not live GitHub Project state;
- separate shipped, partial, planned, and research items;
- cite or link stable public evidence where practical, such as release, issue,
  PR, docs, or known limitations;
- avoid raw internal vault language that assumes private workflow context;
- include M0 preview status and at least near-term M1 direction from the active
  roadmap map.

Empty state:

- If a roadmap item lacks public evidence, it must not be labeled Delivered.
  Use Planned or Research and keep the copy conservative.

### Release-Time Install Verification

The guard may be a CI job, script, or release checklist item, but it must be
specific enough to run during Stage 10 and later release work. It must verify at
least:

- the documented release tag exists;
- documented required archive names match release assets;
- `checksums.txt` exists;
- docs do not present unavailable primary install commands;
- unsupported platform wording agrees with the release assets;
- evidence is captured in the implementation PR or issue.

Cancellation and retry behavior:

- If the verification script or check cannot reach GitHub because of network or
  rate limits, the implementation must retry once when practical and then
  record the check as unavailable rather than passing silently.

Accessibility and focus behavior:

- Navigation additions for Status and roadmap must be reachable through the
  existing docs sidebar and keyboard navigation.
- Any new tables or callouts must use normal Markdown/Starlight semantics so
  screen readers receive headings, table headers, links, and labels.

Race conditions:

- If release assets change while the implementation is in progress, Stage 10
  must re-check release metadata before final evidence and update docs to the
  latest verified release state.

## BDD Scenarios

Feature: Public docs match the shipped v0.4.0-preview release

  Rule: Install instructions must reflect verified release assets

    Scenario: Supported user follows the archive install path
      Given the `v0.4.0-preview` GitHub Release exists
      And the release has checksums and supported target archives
      When a reader opens public installation guidance
      Then the guidance names GitHub Releases as the preview install channel
      And the guidance lists only targets with verified release assets as preview archives
      And the guidance includes checksum verification
      And the guidance tells the reader to keep `runtime/` beside the CLI

    Scenario: Unsupported platform is not hidden
      Given the `v0.4.0-preview` release does not include a macOS Intel archive
      When a reader checks the platform support table
      Then macOS Intel is not listed as a required preview archive
      And the reader sees a source-build fallback or unsupported-platform note

    Scenario: Unavailable crates.io install is not advertised as primary
      Given DUUMBI is not published to crates.io for this preview
      When a reader opens docs, landing, blog, or comparison pages with install commands
      Then the reader is not told to use `cargo install duumbi` as the preview install path
      And any source checkout install command is clearly labeled as local source installation

  Rule: Provider and model claims must match the provider catalog

    Scenario: Provider setup stays provider-first
      Given public provider docs list direct provider configuration
      When a reader follows provider setup guidance
      Then examples use `duumbi provider ...` or `[[providers]]`
      And MiniMax appears in supported direct-provider lists
      And normal setup does not require a user-maintained default model ID

    Scenario: Legacy provider names are not promoted
      Given `xai` is the canonical config key for xAI/Grok models
      And OpenRouter is excluded from the V1 direct-provider catalog
      When a reader opens public provider or launch copy
      Then `grok` appears only as compatibility or historical wording
      And OpenRouter is not presented as a current V1 direct-provider setup path

  Rule: Roadmap content must be honest public product orientation

    Scenario: Reader opens the public Status and roadmap page
      Given the active roadmap map contains delivered, partial, planned, and research work
      When a reader opens the public Status and roadmap page
      Then the page separates shipped preview behavior from partial, planned, and research items
      And delivered items link to public release, issue, PR, docs, or evidence when practical
      And research items are not worded as shipped product behavior

    Scenario: Roadmap page is discoverable
      Given the public docs site has a sidebar
      When a reader navigates the docs with keyboard or normal links
      Then the Status and roadmap page is reachable from navigation
      And the page uses accessible headings, links, and table headers

  Rule: Release verification must catch stale install claims

    Scenario: Release checklist detects a stale platform matrix
      Given public install docs list a required target archive
      And the latest release metadata lacks that archive
      When the release install verification check runs
      Then the check fails or records a blocking checklist failure
      And the implementation evidence identifies the stale target claim

    Scenario: Release checklist detects missing checksums
      Given public install docs instruct users to verify checksums
      And the latest release metadata lacks `checksums.txt`
      When the release install verification check runs
      Then the check fails or records a blocking checklist failure
      And the release must not be described as install-verified

    Scenario: Network failure does not create false confidence
      Given release metadata cannot be fetched because of network failure
      When the install verification check runs
      Then the implementation records the verification as unavailable or failed
      And the check does not silently pass as if release metadata matched

## Tasks

- Re-check current #732, #686, #687, #689, release, README, docs-site, website,
  blog, compare, provider catalog, and roadmap context at Stage 10 start.
- Create a public-claim audit artifact or implementation evidence table.
- Update `duumbi-web` installation guidance to match the actual release asset
  matrix and unsupported-platform boundaries.
- Replace or annotate unavailable `cargo install duumbi` claims in landing and
  blog surfaces.
- Align public provider/model copy with the provider catalog.
- Align public registry/status language with shipped behavior and roadmap
  boundaries.
- Add the Status and roadmap page plus navigation.
- Add a concrete install verification script, CI path, or release checklist
  item and run it.
- Run public website and docs-site build checks.
- Link implementation evidence back to #732.

## Checks

- Product spec review against this document and issue #732.
- Technical spec review maps every BDD scenario to concrete checks.
- Search checks over `hgahub/duumbi-web` for stale install, provider, model,
  version, and platform claims, excluding `node_modules` and generated build
  output.
- GitHub API or `gh` evidence for `v0.4.0-preview` release assets and
  `checksums.txt`.
- Public docs build from `hgahub/duumbi-web/docs`.
- Public website build from `hgahub/duumbi-web`.
- Install verification guard evidence showing pass/fail behavior against the
  current release metadata.
- Static review of the Status and roadmap page for label honesty, public
  suitability, and source traceability.
- Implementation PR evidence includes affected public surfaces, commands run,
  and remaining limitations.

BDD scenario coverage:

- Archive install path: release metadata check, docs diff review, install
  verification guard, docs build.
- Unsupported platform: release metadata check, docs diff review, search for
  `x86_64-apple-darwin`.
- Unavailable crates.io install: search checks for `cargo install duumbi`,
  reviewed exceptions only for local source install or historical notes.
- Provider setup and legacy provider names: search checks plus provider catalog
  diff review.
- Status and roadmap: docs navigation diff, content review, docs build.
- Release checklist failures: unit or script-level negative checks when
  practical, otherwise documented manual checklist dry run with intentionally
  stale sample input.

## Open Questions

None blocking.

Non-blocking implementation risk: the exact verification mechanism may be a
script, CI job, or release checklist item. The technical spec should choose the
lowest-maintenance option that still produces concrete evidence and catches the
stale-claim cases above.

## Sources

- Issue #732: https://github.com/hgahub/duumbi/issues/732
- Stage 5 acceptance comment on #732:
  https://github.com/hgahub/duumbi/issues/732#issuecomment-4748932059
- Issue #686: https://github.com/hgahub/duumbi/issues/686
- Issue #687: https://github.com/hgahub/duumbi/issues/687
- Issue #689: https://github.com/hgahub/duumbi/issues/689
- `v0.4.0-preview` release:
  https://github.com/hgahub/duumbi/releases/tag/v0.4.0-preview
- Source repo README: `README.md`
- Source repo docs audit: `docs/duumbi-686-docs-audit.md`
- Product spec for #686: `specs/DUUMBI-686/PRODUCT.md`
- Product spec for #687: `specs/DUUMBI-687/PRODUCT.md`
- Public docs installation page:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/getting-started/installation.md`
- Public docs provider catalog:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/reference/provider-catalog.md`
- Public docs config reference:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/reference/config.md`
- Public docs navigation:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/astro.config.mjs`
- Public website get-started component:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/components/GetStarted.astro`
- Public launch blog:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/content/blog/introducing-duumbi.md`
- Public compiler blog:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/content/blog/how-duumbi-compiles-jsonld.md`
- Public comparison page:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/pages/compare/duumbi-vs-claude-code.astro`
- Public website index:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/pages/index.astro`
- Active vault source note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-06-12 - Docs Truth Reconciliation.md`
- Active roadmap map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Future Development Roadmap Map.md`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active agentic development runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
