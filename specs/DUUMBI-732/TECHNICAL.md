# DUUMBI-732: Docs Truth Reconciliation For v0.4.0-preview Remaining Items - Technical Specification

## Implementation Objective

Implement the approved product behavior in `specs/DUUMBI-732/PRODUCT.md` by
making public DUUMBI documentation match the shipped `v0.4.0-preview` release
truth after #686, #687, and #689.

The implementation must reconcile public docs, website, blog, metadata,
provider/model copy, registry/status wording, and roadmap visibility. It must
also add a concrete verification path that compares public install instructions
against the documented release tag metadata, not the moving latest-release
endpoint.

Related to #732. This is a technical-specification artifact only. The execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Docs implementation agents working primarily in `hgahub/duumbi-web`.
- Release/docs verification agents checking GitHub Release metadata against
  public install instructions.
- Reviewer agents checking public-claim truth, roadmap label honesty,
  accessibility of docs navigation, and evidence completeness.
- Tester agents validating Starlight/Astro builds, stale-claim searches, and
  the release-install verification guard.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/732
- Product spec: `specs/DUUMBI-732/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/741
- Product spec merge SHA:
  `46bd0aeb1cc933896bd38818b009b41c13bb6301`
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/732#issuecomment-4748932059
- Stage 6 product spec draft comment:
  https://github.com/hgahub/duumbi/issues/732#issuecomment-4749002670
- Stage 7 blocked finding:
  https://github.com/hgahub/duumbi/issues/732#issuecomment-4749010143
- Stage 7 product spec approval decision:
  https://github.com/hgahub/duumbi/issues/732#issuecomment-4749089921
- Relevant completed issue #686: https://github.com/hgahub/duumbi/issues/686
- Relevant completed issue #687: https://github.com/hgahub/duumbi/issues/687
- Relevant completed issue #689: https://github.com/hgahub/duumbi/issues/689
- `v0.4.0-preview` release:
  https://github.com/hgahub/duumbi/releases/tag/v0.4.0-preview
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Prior docs reconciliation spec:
  `specs/DUUMBI-686/TECHNICAL.md`
- Prior release/install spec:
  `specs/DUUMBI-687/TECHNICAL.md`
- Source repo README: `README.md`
- Public docs installation page:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/getting-started/installation.md`
- Public docs navigation:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/astro.config.mjs`
- Public docs provider catalog:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/reference/provider-catalog.md`
- Public docs config reference:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/reference/config.md`
- Public website quick-start component:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/components/GetStarted.astro`
- Public launch blog:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/content/blog/introducing-duumbi.md`
- Public compiler blog:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/content/blog/how-duumbi-compiles-jsonld.md`
- Public website index and structured data:
  `/Users/heizergabor/space/hgahub/duumbi-web/src/pages/index.astro`
- Public docs package script:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/package.json`
- Public website package script:
  `/Users/heizergabor/space/hgahub/duumbi-web/package.json`
- Active roadmap source:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Future Development Roadmap Map.md`
- Active processed inbox source:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-06-12 - Docs Truth Reconciliation.md`

Verified source facts:

- Issue #732 is open and currently labeled `accepted`,
  `product-spec-approved`, and `needs-tech-spec`.
- Stage 5 accepted #732 with no remaining open questions.
- Stage 7 approval was recorded after PR #741 addressed the review finding that
  release verification must pin to the documented tag instead of latest release
  metadata.
- PR #741 was a spec-only PR changing only
  `specs/DUUMBI-732/PRODUCT.md`; it was merged by Stage Approval.
- The published, non-draft `v0.4.0-preview` prerelease has these assets:
  - `checksums.txt`
  - `duumbi-v0.4.0-preview-aarch64-apple-darwin.tar.gz`
  - `duumbi-v0.4.0-preview-x86_64-unknown-linux-gnu.tar.gz`
  - `duumbi-v0.4.0-preview-aarch64-unknown-linux-gnu.tar.gz`
- The `v0.4.0-preview` release body says macOS Intel
  `x86_64-apple-darwin` is not included in the developer preview.
- The public docs installation page still lists macOS Intel
  `x86_64-apple-darwin` as a required preview target.
- The public website quick-start component still advertises
  `cargo install duumbi`.
- The launch blog still advertises `cargo install duumbi` and lists Grok and
  OpenRouter as normal multi-LLM support.
- The compiler blog still advertises `cargo install duumbi`.
- The public website structured data still reports `softwareVersion: '0.1.0'`.
- Public provider catalog and config docs already describe `xai` as canonical
  for xAI/Grok and exclude OpenRouter from the V1 direct-provider catalog.
- Public docs navigation currently has no Status and roadmap page.
- `duumbi-web` and `duumbi-web/docs` both define `npm run build` as
  `astro build`.
- No live DUUMBI LLM behavior changes are required by the approved product
  spec.

Assumptions for implementation:

- `hgahub/duumbi-web` remains the canonical public website and docs repository.
- Stage 10 can obtain writable access to `hgahub/duumbi-web`. If not, Stage 10
  must stop with a blocker or create a coordinated handoff PR in that repo.
- A small Node.js verification script in `duumbi-web` is lower-maintenance than
  adding source-repo Rust tooling for this docs-only public-site issue.
- Historical blog posts may either be updated in place or marked with a clear
  update note; the implementation agent should choose the option that best
  prevents reader confusion without rewriting unrelated historical content.
- The Status and roadmap page is manually curated from the active roadmap map
  plus public GitHub/release evidence; no live Obsidian synchronization is
  required.

## Affected Areas

Expected `hgahub/duumbi-web` Stage 10 areas:

- Public docs install and quickstart surfaces:
  - `docs/src/content/docs/getting-started/installation.md`
  - `docs/src/content/docs/getting-started/quickstart.md` if install or first
    run wording is stale after the audit
  - `docs/src/content/docs/index.mdx` if it contains stale install/status
    claims after the audit
- Public docs navigation:
  - `docs/astro.config.mjs`
  - new docs page, recommended:
    `docs/src/content/docs/project/status-roadmap.md`
- Public docs reference pages when stale claims are found:
  - `docs/src/content/docs/reference/cli.md`
  - `docs/src/content/docs/reference/config.md`
  - `docs/src/content/docs/reference/provider-catalog.md`
  - registry docs under `docs/src/content/docs/registry/**`
- Public website and content:
  - `src/components/GetStarted.astro`
  - `src/pages/index.astro`
  - `src/content/blog/introducing-duumbi.md`
  - `src/content/blog/how-duumbi-compiles-jsonld.md`
  - comparison pages under `src/pages/compare/**` when audit search finds
    stale claims
- Verification tooling:
  - recommended new script:
    `scripts/verify-preview-install-docs.mjs`
  - `package.json` or `docs/package.json` script entry if needed to expose the
    verification command consistently
  - optional fixture files under `scripts/fixtures/` only if needed to exercise
    negative cases without live GitHub mutation

Expected `hgahub/duumbi` Stage 10 areas:

- No compiler/runtime implementation changes.
- Optional source-repo docs/checklist update only when needed to keep release
  checklist evidence discoverable, for example:
  - `README.md`
  - `docs/duumbi-732-docs-truth-audit.md`
  - a release checklist doc under `docs/`
- GitHub issue comments on #732 for implementation evidence and blockers.

Areas expected not to change:

- `src/parser/`, `src/graph/`, `src/compiler/`, `src/agents/`, `src/intent/`,
  `src/registry/`, `src/mcp/`, and runtime behavior.
- DUUMBI provider routing, model catalog generation, registry server behavior,
  Query/Agent/Intent semantics, Studio behavior, or release artifact contents.
- `specs/DUUMBI-732/PRODUCT.md` after Stage 7 approval unless a later gate sends
  the work back to product specification.
- Obsidian vault content during Stage 10.

## Technical Approach

### Repository Coordination

Treat #732 as a cross-repo docs truth issue:

- `hgahub/duumbi-web` should carry the primary implementation PR because the
  stale public claims live there.
- `hgahub/duumbi` may carry only minimal issue evidence, optional release
  checklist docs, or a lightweight audit artifact.
- Every implementation PR must use non-closing issue references such as
  `Related to #732` and must leave the execution issue open for Stage 11 and
  Stage 12.

If Stage 10 cannot edit `hgahub/duumbi-web`, it must stop. A source-repo-only
change cannot satisfy the product spec.

### Public Claim Audit

Before editing, Stage 10 must run deterministic searches over `duumbi-web`,
excluding generated and dependency output:

```bash
rg -n "cargo install duumbi|OpenRouter|Grok|grok|x86_64-apple-darwin|softwareVersion|0\\.1\\.0|Status and roadmap|latest release" \
  /Users/heizergabor/space/hgahub/duumbi-web \
  -g '!node_modules' -g '!dist' -g '!build' -g '!*.lock'
```

Implementation evidence must include a table of audited public surfaces:

| Path | Claim category | Before | Action | Verification |
| --- | --- | --- | --- | --- |

Rows may group repetitive pages only when the same rationale and verification
applies. Install, platform, provider/model, registry, status/roadmap, and
version metadata claims must not be grouped together.

### Install And Platform Corrections

Update public install guidance to match the documented `v0.4.0-preview` tag:

- Keep GitHub Releases as the primary preview install channel.
- Remove macOS Intel from required preview archive rows; state that no macOS
  Intel archive exists for this preview and route Intel users to source build.
- Keep supported archive names synchronized with release assets:
  - `duumbi-v0.4.0-preview-aarch64-apple-darwin.tar.gz`
  - `duumbi-v0.4.0-preview-x86_64-unknown-linux-gnu.tar.gz`
  - `duumbi-v0.4.0-preview-aarch64-unknown-linux-gnu.tar.gz`
- Keep `checksums.txt` verification in the install path.
- Keep the instruction that the extracted release directory must remain
  together so `runtime/` remains beside the CLI.
- Do not present crates.io, Homebrew, apt, winget, signed installers,
  notarized packages, or auto-update as available preview distribution paths.
- Allow `cargo install --path .` only in a source-checkout or contributor path.

### Website And Blog Corrections

Update the landing quick-start component so the first install command points to
the GitHub Release archive path or links to the install docs. Do not keep
`cargo install duumbi` as the normal preview install command.

For blog posts:

- Update or annotate `introducing-duumbi.md` so the Get Started block no longer
  presents `cargo install duumbi` as available.
- Update or annotate `introducing-duumbi.md` provider copy so OpenRouter is not
  presented as a current V1 direct-provider target and `grok` is not promoted
  as the canonical config key.
- Update or annotate `how-duumbi-compiles-jsonld.md` so `cargo install duumbi`
  is not presented as available preview installation.
- If keeping historical sections, add an explicit update note near the stale
  content rather than relying on readers to discover newer docs elsewhere.

Update `src/pages/index.astro` structured data:

- `softwareVersion` should match the public preview version,
  `v0.4.0-preview`, unless Stage 10 finds a schema.org compatibility reason to
  use `0.4.0-preview`; record the rationale either way.
- `downloadUrl` should point to the `v0.4.0-preview` release or the install
  docs page rather than only the repository root, if the current schema shape
  supports that cleanly.

### Status And Roadmap Page

Add a public Status and roadmap page to Starlight docs. Recommended path:

```text
docs/src/content/docs/project/status-roadmap.md
```

Add it to `docs/astro.config.mjs` under a small public-facing group such as
`Project` or under `Community` if that better matches the existing sidebar.

The page must:

- explain that it is public product orientation, not live Project execution
  state;
- separate Delivered, Partial, Planned, and Research items;
- include M0 `v0.4.0-preview` truth, including delivered release/docs work and
  current limitations;
- include near-term M1 direction from the active roadmap map without wording it
  as shipped behavior;
- link to public evidence where practical, such as the release, issues, PRs, or
  docs;
- avoid raw vault-only language, private workflow assumptions, and unverified
  delivery claims;
- use accessible Markdown/Starlight headings, lists, links, and tables.

### Release-Install Verification Guard

Implement a concrete verification path. Recommended shape:

```text
scripts/verify-preview-install-docs.mjs
```

The script should:

- parse or accept the documented release tag, expected initially as
  `v0.4.0-preview`;
- fetch release metadata from
  `https://api.github.com/repos/hgahub/duumbi/releases/tags/<tag>` or use
  `gh api repos/hgahub/duumbi/releases/tags/<tag>`;
- fail if the tag cannot be fetched after one practical retry;
- compare documented required archive targets against release assets for that
  tag;
- fail if `checksums.txt` is absent;
- fail if docs present `cargo install duumbi` as a preview install path outside
  a clearly labeled local source checkout section;
- fail if docs list `x86_64-apple-darwin` as a required preview archive while
  the documented tag has no matching asset;
- fail if docs omit the unsupported-platform/source-build note for unsupported
  preview targets;
- produce concise stdout/stderr evidence suitable for PR review.

The script must not use the latest-release endpoint for this issue. Negative
coverage may be implemented with fixture JSON or an option such as
`--release-json <path>` so Stage 10 can prove stale matrices and missing
checksums fail without mutating GitHub releases.

Expose the command through package scripts if practical, for example:

```text
npm run verify:preview-install
```

The implementation PR must include the command output for the live
`v0.4.0-preview` tag and at least one negative check or documented dry run that
shows the guard fails stale input.

## Invariants

- GitHub Releases remains the documented preview install channel for
  `v0.4.0-preview`.
- Release verification is pinned to the documented tag/version, not latest
  release metadata.
- Public docs must not claim crates.io/package-manager/native-installer
  availability for this preview.
- Public docs must not claim macOS Intel or Windows preview archives exist
  unless new verified release assets are present before implementation.
- Provider setup remains provider-first and `[[providers]]` based; normal users
  are not asked to maintain default model IDs.
- `xai` remains the canonical xAI/Grok provider key; `grok` may appear only as
  compatibility or historical wording.
- OpenRouter is not presented as a current V1 direct-provider setup path.
- The Status and roadmap page is curated public orientation, not a live
  execution tracker or raw Obsidian mirror.
- Stage 10 must not change DUUMBI compiler/runtime behavior.
- Stage 10 must keep the execution issue open and avoid issue-closing
  references in PR titles, bodies, commits, and comments.

## BDD-To-Test Mapping

| Product BDD scenario | Required verification evidence |
| --- | --- |
| Supported user follows the archive install path | Live release metadata command for `v0.4.0-preview`; install page diff review; verification guard passing against the documented tag; docs build. |
| Unsupported platform is not hidden | Search result showing no required `x86_64-apple-darwin` preview row; install page diff review showing macOS Intel source-build fallback; verification guard passing. |
| Unavailable crates.io install is not advertised as primary | Repository-wide `rg` evidence for `cargo install duumbi` excluding generated/dependency files; reviewed exceptions limited to local source checkout or update notes; website build. |
| Provider setup stays provider-first | Search/diff evidence for provider setup pages and marketing/blog copy; provider docs still use `duumbi provider ...` or `[[providers]]`; MiniMax remains present; no default-model maintenance instruction is introduced. |
| Legacy provider names are not promoted | Search evidence for `Grok`, `grok`, and `OpenRouter`; launch copy updated or annotated; provider catalog/config pages remain canonical. |
| Reader opens the public Status and roadmap page | New Starlight page content review; public evidence links reviewed; docs build. |
| Roadmap page is discoverable | `docs/astro.config.mjs` diff review; docs build; optional local preview/manual navigation evidence if Stage 10 runs a preview server. |
| Release checklist detects a stale platform matrix | Verification guard negative fixture or documented dry run showing a required archive absent from tag metadata fails. |
| Release checklist detects missing checksums | Verification guard negative fixture or documented dry run showing missing `checksums.txt` fails. |
| Network failure does not create false confidence | Script review showing fetch errors fail closed after retry; optional dry run with invalid tag or mocked fixture path; implementation evidence states unavailable checks do not pass. |

If Stage 10 cannot automate a scenario fully, it must name the manual review
evidence and explain why automation is not practical for that scenario.

## Live E2E Plan

Canonical interface:

- Public docs and website build commands.
- GitHub Release metadata lookup for the documented tag.
- The verification guard command added by Stage 10.

Provider/LLM path:

- No DUUMBI LLM provider path is touched.
- Required credentials: none for LLM providers.
- External LLM calls: 0.
- Estimated external LLM cost: USD 0.
- If an implementation agent chooses to call an external review model, that is
  outside the product E2E path and must obey the Ralph Cycle resource gate.

Required commands and evidence:

```bash
gh api repos/hgahub/duumbi/releases/tags/v0.4.0-preview \
  --jq '{tag_name, assets: [.assets[].name]}'

cd /Users/heizergabor/space/hgahub/duumbi-web
npm run build

cd /Users/heizergabor/space/hgahub/duumbi-web/docs
npm run build
```

After Stage 10 adds the verification guard, also run the exposed command, for
example:

```bash
cd /Users/heizergabor/space/hgahub/duumbi-web
npm run verify:preview-install
```

Pass criteria:

- Release metadata resolves the documented tag and includes the expected
  archive assets plus `checksums.txt`.
- Public website build succeeds.
- Public docs build succeeds.
- Verification guard passes against live `v0.4.0-preview` metadata.
- Negative guard evidence shows stale platform or missing checksum input fails.
- Stale-claim search results are empty or explicitly justified as historical
  notes/local source checkout wording.

Fail criteria:

- Any public install page presents an unavailable preview install path as
  primary.
- The guard uses latest-release metadata instead of the documented tag.
- The guard cannot fetch metadata and silently passes.
- Public docs or website build fails.
- Status and roadmap page is missing from navigation.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. summarize current state and unmet requirements from the approved product and
   technical specs;
2. propose one bounded implementation goal;
3. list intended file areas and commands before editing;
4. estimate resource use and risk;
5. check whether the resource gate requires human approval;
6. implement only the approved or resource-permitted goal;
7. run the agreed checks;
8. report evidence, failures, and remaining gaps;
9. stop only if requirements are met, a blocker appears, expected external LLM
   cost of the next cycle exceeds USD 1, or scope changes; iteration count is
   not a stop condition.

## Cycle Budget

- Default cycle size: one bounded documentation or verification-tooling goal per
  cycle.
- Max files or modules per cycle: prefer 3-6 related public-docs files, or one
  verification script plus its package-script/fixture support.
- Expected command budget per cycle:
  - targeted `rg` checks;
  - relevant `npm run build` command when a docs/website surface changes;
  - verification guard command after it exists;
  - `git diff --check`.
- Human approval is required only when a cycle will use an external LLM with
  expected cost above USD 1, exceeds approved scope, adds risky dependencies,
  changes release artifacts, changes compiler/runtime behavior, performs an
  irreversible operation, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is covered by the Codex
  App subscription and never triggers the gate.
- No autonomous batch cap: cycles continue until completion, blocker, gate
  breach, or scope change.
- Stop and ask for human guidance when:
  - release metadata contradicts the approved product spec;
  - `duumbi-web` cannot be edited or built;
  - the roadmap source cannot be translated into public labels without making a
    product promise;
  - required verification would need new infrastructure or credentials outside
    the approved scope.

## Task Breakdown

1. Re-check #732, #686, #687, #689, the `v0.4.0-preview` release, README, and
   current `duumbi-web` stale-claim search results.
2. Create or prepare the implementation evidence table for audited public
   surfaces.
3. Update docs installation/platform guidance to remove the macOS Intel preview
   archive claim and preserve checksum/runtime instructions.
4. Replace or annotate `cargo install duumbi` in website and blog surfaces.
5. Align launch/blog provider wording with `xai` canonical naming and
   OpenRouter exclusion from V1 direct-provider docs.
6. Update public structured metadata version/download fields.
7. Add the Status and roadmap docs page and navigation entry.
8. Implement the pinned-tag install verification guard and expose its command.
9. Run stale-claim searches, public website build, public docs build, live
   release-metadata lookup, verification guard, and negative guard evidence.
10. Update issue/PR evidence with audited surfaces, commands, outputs, and any
    remaining intentionally historical claims.

## Verification Plan

Required local/static checks:

- `git diff --check`
- stale-claim `rg` search over `duumbi-web`, excluding generated/dependency
  output
- focused search for issue-closing references in PR text and commit messages
- public website build:
  `cd /Users/heizergabor/space/hgahub/duumbi-web && npm run build`
- public docs build:
  `cd /Users/heizergabor/space/hgahub/duumbi-web/docs && npm run build`
- release metadata lookup:
  `gh api repos/hgahub/duumbi/releases/tags/v0.4.0-preview`
- verification guard against live tag metadata
- verification guard negative evidence for stale platform matrix and missing
  checksums, implemented through fixtures or documented dry run

Required review evidence:

- implementation PR body lists affected public surfaces and audit table link or
  inline summary;
- implementation PR or issue comment includes command outputs or concise pass
  summaries;
- Status and roadmap page content review confirms label honesty and public
  evidence links;
- release verification guard review confirms it does not call latest-release
  metadata.

## Completion Criteria

Stage 10 implementation is complete only when:

- public install docs list only verified preview archives for
  `v0.4.0-preview`;
- macOS Intel and Windows limitations are visible and not hidden;
- no public primary install path advertises `cargo install duumbi` from
  crates.io;
- provider/model public copy aligns with `xai`, MiniMax, provider-first setup,
  and OpenRouter exclusion from V1 direct-provider setup;
- public structured version metadata no longer says `0.1.0`;
- Status and roadmap page exists, is in navigation, and separates Delivered,
  Partial, Planned, and Research claims;
- verification guard checks the documented tag metadata and fails closed on
  unavailable metadata, stale platform claims, or missing checksums;
- website and docs builds pass;
- implementation evidence links back to #732;
- no implementation PR uses an issue-closing reference for #732;
- no DUUMBI compiler/runtime/provider behavior was changed.

## Failure And Escalation

- If `duumbi-web` is unavailable or not writable, stop and record a blocker on
  #732; do not substitute source-repo-only docs edits.
- If the release metadata has changed since this spec, re-check the documented
  tag and update public docs to the verified tag state. If the change creates a
  product trade-off, stop and ask for guidance.
- If docs build fails because of unrelated existing breakage, capture the
  failure, isolate whether #732 changes caused it, and decide whether a narrow
  docs-build fix is in scope.
- If stale search results remain, classify each as fixed, intentionally
  historical with an update note, local source checkout wording, generated
  artifact, or blocker.
- If adding the verification guard requires new dependencies, prefer standard
  Node.js APIs first. Ask for approval before adding dependencies.
- If any cycle would exceed USD 1 in expected external LLM cost, request human
  approval before making that call.

## Open Questions

None blocking.

Accepted implementation discretion:

- The Status and roadmap page may live under `project/`, `community/`, or
  another docs section if the implementation agent keeps it discoverable and
  public-reader appropriate.
- Historical blog content may be updated in place or annotated with update
  notes, as long as readers are not left with stale actionable instructions.
- The install verification guard may be CI, a package script, or a concrete
  release-checklist command, but it must produce reviewable evidence and pin to
  the documented tag.
