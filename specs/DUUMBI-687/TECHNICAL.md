# DUUMBI-687: Tag v0.4.0-preview With Prebuilt Binaries And A Working Install Path - Technical Specification

## Implementation Objective

Implement the approved product behavior in `specs/DUUMBI-687/PRODUCT.md` by
preparing and executing the first DUUMBI developer-preview release path:

- update release version metadata for `0.4.0-preview`;
- harden the tag-triggered GitHub Release workflow so it publishes required
  prebuilt archives and checksums;
- update source-repo README and canonical public docs install guidance;
- create and verify the `v0.4.0-preview` tag and GitHub Release only after
  release-prep changes are merged;
- record platform smoke evidence proving users can install and run DUUMBI
  without building the workspace from source.

Related to #687. This is a technical-specification artifact only. The execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Release/tooling agents updating GitHub Actions, version metadata, install
  docs, and release evidence.
- Docs agents coordinating source-repo README changes with the canonical
  `hgahub/duumbi-web/docs` install page.
- Reviewer agents checking release safety, artifact integrity, docs truth, and
  non-closing workflow references.
- Tester agents validating CI, release workflow behavior, archive contents,
  checksums, and installed-binary smoke paths.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/687
- Product spec: `specs/DUUMBI-687/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/709
- Product spec merge SHA:
  `fcd6496578ba15984c4284c34d9ccb7697ea8ede`
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/687#issuecomment-4702626396
- Stage 6 product spec draft comment:
  https://github.com/hgahub/duumbi/issues/687#issuecomment-4702654242
- Stage 7 AI gate decision:
  https://github.com/hgahub/duumbi/issues/687#issuecomment-4702656171
- Stage 7 product spec approval decision:
  https://github.com/hgahub/duumbi/issues/687#issuecomment-4702656757
- Stage 4 triage comment:
  https://github.com/hgahub/duumbi/issues/687#issuecomment-4697632616
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Release workflow: `.github/workflows/release.yml`
- CI workflow: `.github/workflows/ci.yml`
- Stage approval workflows:
  - `.github/workflows/spec-ai-gate.yml`
  - `.github/workflows/stage-approval.yml`
  - `.github/workflows/ready-for-build-handoff.yml`
- Source package metadata:
  - `Cargo.toml`
  - `crates/duumbi-studio/Cargo.toml`
- CLI version source: `src/cli/mod.rs` uses Clap's package version metadata.
- Studio binary entry point: `crates/duumbi-studio/src/bin/studio.rs`
- Source README install guidance: `README.md`
- Canonical docs installation page:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/getting-started/installation.md`
- Canonical docs quickstart:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/getting-started/quickstart.md`
- Canonical docs package scripts:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/package.json`
- Active vault PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active vault glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Agentic Development Runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Release preview vault note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Release v0.4.0-preview TUI-first.md`
- Docs truth reconciliation vault note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Docs Truth Reconciliation.md`

Verified source facts:

- Issue #687 is open and currently has `accepted`, `product-spec-approved`, and
  `needs-tech-spec` labels.
- Stage 5 accepted the issue with no remaining open questions.
- Product spec PR #709 was a one-file spec-only PR for
  `specs/DUUMBI-687/PRODUCT.md`.
- PR #709 passed CI checks, had Codex self-review with no blocking findings,
  had no review threads, and was merged by Stage Approval.
- Stage 7 AI gate and Stage 7 product-spec approval comments are recorded on
  the issue.
- The current GitHub token cannot read Project V2 fields because it lacks
  `read:project`; labels and comments are the verified workflow state available
  during drafting.
- `Cargo.toml` and `crates/duumbi-studio/Cargo.toml` currently declare version
  `0.3.3`.
- `git tag --list` returned no local tags.
- GitHub API checks returned no remote tag refs and no GitHub Releases.
- `.github/workflows/release.yml` is active and triggers on `v*` tag pushes.
- The current release workflow builds `duumbi` and `studio`, packages runtime
  files, and uploads archives for macOS aarch64, macOS x86_64, Linux x86_64,
  and Linux ARM64.
- The current release workflow does not publish checksums.
- `.github/workflows/ci.yml` treats `specs/**` as documentation-only for Rust
  check scope; release workflow, Cargo metadata, source, runtime, and docs-site
  changes are not documentation-only.
- `README.md` currently documents source checkout plus `cargo install --path .`
  as the install path.
- The canonical docs installation page currently documents source build while
  packaged install is still being verified.
- The canonical docs package defines `npm run build` as `astro build`.
- `src/cli/mod.rs` uses Clap's package version metadata for CLI version output.
- `crates/duumbi-studio/src/bin/studio.rs` has no standalone `--help` smoke
  path; it starts a server only when pointed at a DUUMBI workspace.

Assumptions for implementation:

- `0.4.0-preview` is acceptable Cargo semver pre-release syntax. Stage 10 must
  verify this with Cargo commands before relying on it.
- The release-prep implementation should land through a normal implementation
  PR before any tag is pushed.
- Tag and release creation are release operations. They may be performed by a
  Stage 10 release agent only after the release-prep PR is merged and only when
  the operator has appropriate GitHub authorization.
- Public docs changes require writable access to `hgahub/duumbi-web`. If that
  access is unavailable, Stage 10 must record a blocker or create a coordinated
  docs-site PR rather than leaving docs stale.
- A prebuilt install path may be a concise archive-install command sequence or a
  small install script plus manual fallback. The implementation agent should
  choose the simpler path that can be verified on required platforms.

## Affected Areas

Expected `hgahub/duumbi` Stage 10 areas:

- Version metadata:
  - `Cargo.toml`
  - `crates/duumbi-studio/Cargo.toml`
  - `Cargo.lock` if package version changes update lock metadata
- Release automation:
  - `.github/workflows/release.yml`
  - optional helper script under `scripts/` only if it reduces workflow
    duplication or makes checksum/archive validation safer
- Install and release docs:
  - `README.md`
  - optional internal release checklist under `docs/` if Stage 10 needs a
    durable operator checklist
- Release evidence:
  - GitHub issue comments on #687
  - implementation PR body/comments
  - GitHub Release notes and assets

Expected `hgahub/duumbi-web` Stage 10 areas:

- `docs/src/content/docs/getting-started/installation.md`
- `docs/src/content/docs/getting-started/quickstart.md`
- possibly `docs/src/content/docs/reference/cli.md` if install/version wording
  needs a reference update
- `docs/astro.config.mjs` only if navigation needs no hidden install page
- `docs/package.json` and lockfile only if docs tooling changes are truly
  required, which is not expected

Areas expected not to change:

- `src/parser/`, `src/graph/`, `src/compiler/`, `src/agents/`,
  `src/intent/`, `src/registry/`, `src/mcp/`, and runtime semantics, except
  for narrow version-surface changes if Stage 10 proves Clap/package metadata is
  insufficient.
- Tests unrelated to release workflow, version metadata, docs install claims, or
  archive validation.
- Provider/model catalog behavior.
- Registry publish/yank behavior.
- Studio UI behavior beyond release packaging and smoke evidence.
- Existing specs for other issues.

## Technical Approach

### Stage 10 Shape

Use at least two implementation phases, even if one Codex session handles both:

1. **Release-prep PR phase.** Update version metadata, release workflow,
   checksums/archive validation, README, and docs-site install guidance. Run
   CI and docs-site build checks. Open an implementation PR and request the
   required implementation review.
2. **Post-merge release operation phase.** After the release-prep PR is merged,
   create and push `v0.4.0-preview`, wait for the release workflow, verify
   assets/checksums/install smoke evidence, and record evidence on #687.

Do not push the release tag from an unreviewed implementation branch.

### Version Metadata

- Update the workspace root package version from `0.3.3` to the chosen preview
  version.
- Update `crates/duumbi-studio/Cargo.toml` to the same preview version.
- Run `cargo metadata --format-version 1 --locked` or an equivalent Cargo
  command to verify the version string and lockfile state.
- Verify `duumbi --version` prints the preview version after a local build.
- If Cargo rejects `0.4.0-preview`, stop and choose a valid semver pre-release
  form such as `0.4.0-preview.0`; update README/docs/release tag naming only
  after making the version/tag relationship explicit on #687.

### Release Workflow

Keep the existing tag-triggered workflow, but harden it:

- Use locked release builds unless Stage 10 proves that breaks current release
  behavior:
  - `cargo build --release --locked --target <target>`
  - `cargo build --release --locked --target <target> -p duumbi-studio --features ssr`
- Preserve required targets:
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
  - `x86_64-unknown-linux-gnu`
- Linux ARM64 may remain in the matrix if it continues to build reliably; if
  retained, document whether it is supported or extra/experimental.
- Add checksum generation. Prefer a release-job manifest after all artifacts
  are downloaded on Ubuntu:
  - `sha256sum *.tar.gz > checksums.txt`
  - attach `checksums.txt` to the GitHub Release
- Add archive-content validation before upload:
  - archive contains `duumbi`
  - archive contains `studio`
  - archive contains `runtime/duumbi_runtime.c`
  - archive contains `runtime/third_party/`
  - archive contains license/README material when available
- Keep the release notes body explicit about:
  - developer/research preview status
  - install command or install sequence
  - included binaries/files
  - required platform matrix
  - optional/experimental platforms, if any
  - source-build fallback
  - unsupported package managers/signing/installers

Do not add crates.io publish steps for this issue.

### Install Docs

Update README and docs-site content together:

- Primary path: GitHub Release prebuilt archive for the user's platform.
- Fallback path: source checkout and local build/install.
- Do not present `cargo install duumbi` from crates.io as available unless a
  separate release decision implements and verifies it.
- Keep `cargo install --path .` only as source-checkout/contributor guidance.
- Name exact artifact patterns and checksum verification commands.
- State that Rust and a C compiler are not required to build DUUMBI when using
  the prebuilt archive, but the user may still need platform runtime/toolchain
  pieces for compiling DUUMBI programs if applicable.
- Keep Windows wording conservative. If no Windows archive ships, say no Windows
  preview archive is available. If an experimental Windows archive ships, label
  it experimental and preserve existing support boundaries.

### Docs-Site Coordination

The source repo cannot alone satisfy #687 because the product spec requires
docs.duumbi.dev alignment.

- If the Stage 10 environment has writable `hgahub/duumbi-web`, update docs
  there and create a coordinated docs PR if the repo requires separate review.
- If writable docs-site access is missing, stop with a blocker or record a
  linked docs-site PR request on #687. Do not mark #687 release-ready with stale
  public docs.
- Run `npm run build` from `hgahub/duumbi-web/docs` after docs-site changes.

### Release Operation

After the release-prep implementation PR is merged:

- Ensure local and remote `main` are at the merged commit intended for release.
- Create an annotated or signed tag only if the operator environment supports
  the chosen tag policy.
- Push `v0.4.0-preview`.
- Wait for `.github/workflows/release.yml`.
- Verify the GitHub Release is published, not a draft, and explicitly marked or
  worded as a preview/pre-release.
- Verify assets and checksums through GitHub API or `gh release view`.
- Run required-platform install smoke checks.
- Post evidence on #687 and in the implementation PR.

If any required platform fails, keep the issue out of Stage 12 completion and
record the blocking release evidence.

## Invariants

- Spec-only PRs for #687 must use non-closing issue references.
- The release-prep implementation PR must not claim release success before the
  tag-triggered workflow succeeds.
- Release artifacts must come from the tagged source.
- Artifact names, release notes, README, docs-site install guidance, and
  checksum manifest must agree.
- `duumbi` and `studio` package versions must agree for the preview.
- Source build remains available for contributors.
- Crates.io is not presented as the preview install path.
- Windows is not implied unless an experimental Windows artifact is deliberately
  produced and verified.
- Public docs must not require users to infer whether they are using a prebuilt
  install path or a source-build path.
- Stage 10 must not broaden #687 into package-manager distribution, signing,
  notarization, auto-update, or stable-release policy.

## BDD-To-Test Mapping

| Product BDD scenario | Verification evidence |
| --- | --- |
| Required platform archives are attached to the preview release | Tag-triggered `Release` workflow run passes; `gh release view v0.4.0-preview --json tagName,isDraft,isPrerelease,assets,url` shows required target archives and checksum manifest; issue evidence links the run and release. |
| Release archives include the runtime files needed by DUUMBI programs | Workflow archive validation step runs `tar tzf` or equivalent for each archive and asserts `duumbi`, `studio`, `runtime/duumbi_runtime.c`, and `runtime/third_party/` are present. Attach logs or CI step links. |
| Version metadata identifies the preview | Local or CI command after version update runs `cargo run -- --version` or built archive `duumbi --version` and asserts `0.4.0-preview` or approved equivalent. Also inspect `Cargo.toml`, `crates/duumbi-studio/Cargo.toml`, and `Cargo.lock` metadata. |
| macOS user installs from the prebuilt preview path | Required smoke evidence from macOS aarch64 and macOS x86_64 environments downloads the release archive, verifies checksum, installs or PATHs `duumbi`, runs `duumbi --version`, `duumbi init`, `duumbi build`, and `duumbi run`. |
| Linux x86_64 user installs from the prebuilt preview path | Required smoke evidence from a clean Linux x86_64 runner/container downloads the release archive, verifies checksum, installs or PATHs `duumbi`, runs `duumbi --version`, `duumbi init`, `duumbi build`, and `duumbi run`. |
| Unsupported platform guidance is explicit | Static docs review over `README.md` and docs-site install page verifies unsupported/optional platform wording and no claims of signing, installers, package-manager support, or Windows support beyond evidence. |
| README and docs-site install instructions agree | Static text review plus targeted `rg` checks verify both surfaces use the same artifact naming, tag, install channel, checksum guidance, and source-build fallback. Docs-site build passes. |
| Docs no longer advertise an unavailable crates.io channel | Targeted `rg -n "cargo install duumbi|crates\\.io|cargo install --path" README.md <docs-site-install-page>` verifies crates.io is not presented as available and `cargo install --path .` is source-checkout-only. |
| Required-platform smoke checks prove the installed binary works | Per-platform smoke logs include `duumbi --version`, `duumbi init`, `duumbi build`, and `duumbi run` output from the installed archive path. |
| Failed release workflow blocks release claims | Stage 10 evidence policy: failed required workflow/check exits nonzero, issue comment records the failure, no Stage 11 merge-readiness recommendation or Stage 12 closure claim is made until fixed. |

## Live E2E Plan

Canonical interface: CLI from the GitHub Release archive.

Provider/LLM path:

- This issue does not change DUUMBI LLM behavior.
- No live DUUMBI provider calls are required for the release/install E2E path.
- Required environment variables for LLM providers: none.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.

Credentials and permissions:

- GitHub token or release-operator credentials capable of pushing the release
  tag and allowing the release workflow to publish assets.
- No registry credentials.
- No crates.io credentials.
- No Slack credentials required for release validation itself, though workflow
  handoff notifications may use existing repository secrets.

Required platform E2E matrix:

- macOS aarch64.
- macOS x86_64.
- Linux x86_64.

Optional/extra matrix:

- Linux ARM64 if retained in the release workflow.
- Windows x86_64 only if Stage 10 deliberately includes an experimental
  archive.

Representative commands for each required platform, adjusted to the documented
install path and artifact name:

```bash
tmp="$(mktemp -d)"
cd "$tmp"
curl -L -o duumbi.tar.gz "https://github.com/hgahub/duumbi/releases/download/v0.4.0-preview/<archive-name>.tar.gz"
curl -L -o checksums.txt "https://github.com/hgahub/duumbi/releases/download/v0.4.0-preview/checksums.txt"
shasum -a 256 -c checksums.txt --ignore-missing || sha256sum -c checksums.txt --ignore-missing
tar xzf duumbi.tar.gz
export PATH="$tmp/<extracted-archive>:$PATH"
duumbi --version
duumbi init smoke
cd smoke
duumbi build
duumbi run
```

Studio archive smoke:

- Because `studio` starts a server and requires a DUUMBI workspace, validate
  that the binary exists in the archive and starts against the smoke workspace
  only when the platform runner can safely allocate and tear down a local port.
- If full Studio startup is not run on every platform, record archive-content
  validation plus at least one local startup smoke on a supported platform:

```bash
studio --workspace "$tmp/smoke" --port 8421
```

The startup smoke must terminate the server after verifying the process starts
or an HTTP response is available.

Pass/fail criteria:

- Required archive downloads successfully.
- Checksum verification passes.
- Archive content validation passes.
- `duumbi --version` reports the approved preview version.
- `duumbi init`, `duumbi build`, and `duumbi run` succeed from the installed
  archive path.
- README and docs-site commands match the verified commands.
- Any required platform failure blocks release readiness.

Artifacts:

- Release workflow run URL.
- GitHub Release URL.
- Asset list and checksum manifest.
- Per-platform smoke logs.
- README and docs-site PR links.
- Implementation PR and issue evidence comments.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop only if requirements are met, a blocker appears, the expected external
   LLM cost of the next cycle exceeds USD 1, or scope changes; iteration count
   is not a stop condition

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle:
  - Release metadata cycle: `Cargo.toml`, `crates/duumbi-studio/Cargo.toml`,
    and `Cargo.lock` if needed.
  - Release workflow cycle: `.github/workflows/release.yml` plus one helper
    script only if justified.
  - Source docs cycle: `README.md` plus optional internal release checklist.
  - Docs-site cycle: bounded docs-site install/quickstart/reference files in
    `hgahub/duumbi-web/docs`.
  - Release operation cycle: tag/release verification and issue evidence only;
    no source edits unless a failure requires returning to implementation.
- Expected command budget per code/workflow cycle:
  - `cargo fmt --check`
  - `cargo metadata --format-version 1 --locked`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo nextest run --workspace`
  - `cargo audit`
  - targeted `rg` checks for install/crates.io wording
  - docs-site `npm run build` when docs-site files change
  - GitHub API/CLI release verification after tag publication
- Human approval required only when:
  - a cycle will use an external LLM with expected cost above USD 1
  - the work exceeds the approved product or technical spec
  - a cycle adds risky dependencies, package-manager distribution, signing,
    notarization, auto-update, or irreversible release operations not already
    covered here
  - release/tag operations target a commit that is not the reviewed and merged
    release-prep state
  - a required platform cannot be verified and a product decision is needed
  - credentials or repository permissions are missing
  - a product/architecture decision appears
- External LLM usage counted: DUUMBI live provider calls and external model or
  agent CLI calls. Codex internal reasoning usage is covered by the Codex App
  subscription and never triggers the gate.
- No autonomous batch cap: cycles continue until completion, blocker, gate
  breach, or scope change.
- When to stop and ask for human guidance:
  - version/tag naming conflict
  - crates.io or package-manager strategy pressure
  - Windows support decision beyond optional experimental artifact
  - release artifact replacement after publication
  - missing docs-site write access
  - required platform smoke cannot be run or fails for environmental reasons

## Task Breakdown

1. Confirm Stage 9 starting context: issue labels, product spec approval, PR
   merge, source state, docs-site state, no existing tag/release.
2. Create release-prep implementation branch after Stage 9 approval.
3. Update version metadata and verify Cargo accepts the preview version.
4. Harden `.github/workflows/release.yml` for locked builds, archive content
   validation, checksums, and release notes.
5. Update README install guidance.
6. Update canonical docs-site install/quickstart guidance in `hgahub/duumbi-web`
   or stop with a blocker if write access is unavailable.
7. Run local checks and open implementation PR(s).
8. Request final implementation PR review from `@chatgpt-codex-connector`;
   recommend Greptile only if the implementation becomes high-risk codebase
   work beyond docs/workflow/versioning.
9. After human merge, create and push `v0.4.0-preview` from the reviewed state.
10. Wait for release workflow completion and verify assets/checksums.
11. Run required-platform install smoke checks.
12. Record evidence on #687 and route to Stage 11/Stage 12 according to the
    DUUMBI workflow.

Independently executable slices:

- Version metadata and Cargo verification.
- Release workflow hardening.
- Source README install wording.
- Docs-site install/quickstart wording.
- Post-merge release verification.
- Per-platform smoke checks.

## Verification Plan

Pre-release-prep PR checks:

- `cargo fmt --check`
- `cargo metadata --format-version 1 --locked`
- `cargo clippy --all-targets -- -D warnings`
- `cargo nextest run --workspace`
- `cargo audit`
- targeted `rg` checks:
  - no unavailable crates.io install claim
  - README/docs artifact names match release workflow names
  - no stale "packaged install is still being verified" text after release docs
    are finalized
- docs-site `npm run build` from `hgahub/duumbi-web/docs` when docs-site files
  change
- manual review of `.github/workflows/release.yml` for required targets,
  checksum generation, archive validation, release notes, and no crates.io
  publish step

Post-merge release checks:

- `git rev-parse v0.4.0-preview^{commit}` matches the intended release commit.
- Release workflow completed successfully.
- `gh release view v0.4.0-preview --json tagName,isDraft,isPrerelease,assets,url`
  shows required assets and checksums.
- Download and verify each required archive.
- Run required-platform CLI E2E smoke.
- Optional Studio startup smoke on at least one supported platform.
- Issue evidence links the release, workflow run, checksums, and smoke logs.

Review evidence:

- Codex self-review on implementation PR.
- `@chatgpt-codex-connector` review on final implementation PR.
- Greptile only if explicitly requested for a high-risk final implementation
  PR; not required for specs and not expected for routine release/docs/workflow
  prep.

## Completion Criteria

- Product spec and technical spec are approved and merged.
- Release-prep implementation PR(s) are merged.
- Package versions identify the preview release consistently.
- Release workflow produces required archives and checksum manifest.
- `v0.4.0-preview` tag exists on the intended release commit.
- GitHub Release for `v0.4.0-preview` exists with required assets.
- README and docs.duumbi.dev agree with the shipped release.
- Required-platform smoke evidence is recorded.
- No blocking release, docs, artifact integrity, or platform-verification
  question remains.
- The issue remains open until Stage 12 closure verifies the final evidence.

## Failure And Escalation

- If Cargo rejects the preview version string, stop and ask for a release naming
  decision.
- If the release workflow cannot build a required platform, keep the issue in
  implementation/blocker state and record the failing job.
- If checksums cannot be generated or verified, treat release readiness as
  blocked.
- If docs-site write access is missing, stop with a blocker or create a linked
  docs-site PR request; do not ship stale public install docs.
- If the tag is pushed to the wrong commit, stop and request human release
  guidance before changing or replacing public release artifacts.
- If an already-published artifact is wrong, do not silently replace it. Record
  the problem, decide whether to publish a replacement release/tag or mark the
  asset withdrawn, and preserve audit evidence.
- If a platform smoke fails because of a product/runtime problem, return to
  implementation.
- If a platform smoke fails because of environment access, record the limitation
  and request human guidance if the platform is required.
- If scope expands to crates.io, package managers, signing, notarization,
  auto-update, or stable-release policy, stop for a product decision.

## Open Questions

None blocking.

Non-blocking Stage 10 choices:

- Whether Linux ARM64 is a supported preview artifact or an extra/experimental
  artifact.
- Whether Windows x86_64 is attempted as experimental or deferred.
- Whether the install path is a short archive-install command sequence or a
  small install script plus manual fallback.
