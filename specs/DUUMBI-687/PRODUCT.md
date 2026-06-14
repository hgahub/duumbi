# DUUMBI-687: Tag v0.4.0-preview With Prebuilt Binaries And A Working Install Path

## Summary

Define the product behavior for the first DUUMBI developer-preview release.
DUUMBI currently has a working source build path, but it has no git tags, no
GitHub Releases, no prebuilt release artifacts, and no published crates.io
package. The accepted issue asks for a real `v0.4.0-preview` release that a
developer can install without cloning the repository and building the whole
workspace by hand.

The preview install channel for this issue is GitHub Releases with prebuilt
archives. Each release archive must include the DUUMBI CLI, the Studio binary,
and the runtime files needed by compiled DUUMBI programs. The README and the
canonical public docs site must describe the verified install path and must
clearly label the release as a developer/research preview, not a stable 1.0
release.

Related to #687. This is a product-specification artifact only. The execution
issue must remain open for Stage 7 Product Spec Review, Stage 8 technical
specification, Stage 9 Technical Spec Review, Stage 10 implementation, Stage 11
review, and Stage 12 closure.

## Problem

The current public onboarding path is still too expensive for a preview user:

- `Cargo.toml` and `crates/duumbi-studio/Cargo.toml` currently declare version
  `0.3.3`.
- The repository has no local or remote tags and the GitHub Releases API returns
  no releases.
- `README.md` still documents `git clone` plus `cargo install --path .`, which
  requires a full Rust toolchain and a local source checkout.
- The canonical public docs site currently says the packaged Cargo install
  channel is still being verified and points users back to a source build.
- `.github/workflows/release.yml` exists and is active, but the shipped release
  state has not been proven by a tag-triggered run, attached artifacts,
  checksums, install docs, or clean-machine verification evidence.

That mismatch creates a credibility problem: DUUMBI can be built by contributors,
but a new developer cannot yet try a named preview release through a normal
installable artifact.

## Outcome

When this work is done:

- `v0.4.0-preview` exists as a git tag in `hgahub/duumbi`.
- GitHub Releases has a `v0.4.0-preview` release that is clearly marked and
  worded as a developer/research preview.
- The release includes prebuilt archives for at least:
  - macOS aarch64 (`aarch64-apple-darwin`)
  - macOS x86_64 (`x86_64-apple-darwin`)
  - Linux x86_64 (`x86_64-unknown-linux-gnu`)
- Linux ARM64 may remain included because the current release workflow already
  names `aarch64-unknown-linux-gnu`; if retained, it must have the same archive
  and checksum standards.
- Windows x86_64 is optional for this issue. If included, it must be clearly
  labeled experimental and must not imply installer, signing, ARM64 Windows,
  MinGW, Cygwin, or GNU Windows support.
- Every attached archive includes:
  - `duumbi`
  - `studio`
  - `runtime/duumbi_runtime.c`
  - `runtime/third_party/`
  - license and README material when available
- Every attached archive has a published checksum.
- A user can follow the README or docs-site install instructions and run
  `duumbi --version` without building DUUMBI from source.
- `duumbi --version` and `studio` release metadata identify the preview as
  `0.4.0-preview` or an equivalent semver pre-release form accepted by Cargo.
- README and docs.duumbi.dev agree on the preview install path, supported
  platform matrix, source-build fallback, and preview limitations.
- Release notes describe what is included, how to install, what is experimental,
  and what remains outside the preview.
- Clean-machine verification evidence exists for each required platform.
- The execution issue remains open until Stage 12 verifies release artifacts,
  docs, and implementation evidence.

## Scope

### In Scope

- Choose and document GitHub Releases with prebuilt archives as the preview
  install channel for #687.
- Update workspace package versions for the preview release.
- Update or harden `.github/workflows/release.yml` so a `v0.4.0-preview` tag
  builds release archives and uploads them to GitHub Releases.
- Ensure release artifacts include CLI, Studio, runtime source, vendored runtime
  dependencies, license/README material, and checksums.
- Add or update release notes text so the release is clearly a
  developer/research preview.
- Update `README.md` install guidance to prefer the verified preview artifact
  once it exists, while retaining source-build fallback guidance.
- Update the canonical public docs site under `hgahub/duumbi-web/docs` so the
  installation page and quickstart match the release artifact.
- Verify required-platform artifacts and install instructions on clean or
  clean-enough machines/containers/runners.
- Record release evidence in the issue and implementation PR.
- Preserve the existing source-build and contributor workflows.

### Explicitly Out Of Scope

- Publishing the `duumbi` crate to crates.io for this preview.
- Designing a long-term package manager strategy beyond the preview release.
- Homebrew, apt, dnf, pacman, winget, installer packages, notarization, signing,
  auto-update, or package-manager repository maintenance.
- Stable 1.0 positioning, compatibility promises, or production support claims.
- Windows release support unless the implementation agent explicitly includes an
  experimental x86_64 Windows artifact with evidence.
- Changes to compiler semantics, runtime behavior, registry behavior, provider
  behavior, Query, Agent, Intent, Studio UX, or graph validation except where
  required to expose correct release version metadata.
- Rewriting broader docs outside the install/release surfaces unless stale
  install claims are discovered during verification.
- Implementation code, tests, docs edits, tag creation, release publishing, or
  Ralph cycles during this specification stage.

## Constraints And Assumptions

Facts:

- Issue #687 is open.
- Issue #687 is labeled `accepted` and `needs-spec`.
- The Stage 5 Human Acceptance Decision comment dated 2026-06-14 records
  `Decision: Accept`, `Next state: Spec Needed`, and no remaining open
  questions.
- The issue body identifies #687 as preview blocker 2/4 for the
  `v0.4.0 Developer Preview` gate.
- The issue body asks for a tagged preview release, prebuilt binaries, a working
  install path, release workflow support, docs updates, and explicit preview
  labeling.
- Local `git tag --list` returned no tags.
- GitHub API checks for `hgahub/duumbi` returned no releases and no tag refs.
- `.github/workflows/release.yml` exists, is active, and triggers on `v*` tag
  pushes.
- The current release workflow builds `duumbi` and `studio`, packages runtime
  files, and uploads release archives for macOS aarch64, macOS x86_64,
  Linux x86_64, and Linux ARM64.
- The current release workflow does not publish checksums.
- `README.md` currently documents source checkout plus `cargo install --path .`
  under Install.
- `docs.duumbi.dev` is sourced from `hgahub/duumbi-web/docs`. Its installation
  page currently says packaged install is still being verified and documents a
  source build path.
- #686 reconciled legacy docs and recorded that public install claims should not
  overstate `cargo install duumbi` while #687 remains open.
- The active vault release note says the desired preview path is GitHub Releases
  binaries plus an install command or script, with clean-machine evidence.
- The active DUUMBI runbook requires combined product and technical specs, Codex
  self-review, clean Stage 7 and Stage 9 AI gates, no Greptile on spec PRs, and
  resource-gated Ralph cycles after Ready for Build.

Assumptions:

- GitHub Releases is the safest preview channel because the release workflow
  already exists and the crate is not currently published to crates.io.
- The implementation agent can edit `hgahub/duumbi` and can also obtain writable
  access to `hgahub/duumbi-web` for public docs updates. If docs-site write
  access is unavailable, Stage 10 must record a blocker or create a coordinated
  docs PR in the docs repository.
- Clean-machine verification can use GitHub-hosted runners, fresh containers,
  temporary user accounts, or disposable local environments when they prove the
  same user-visible install commands.
- A short copy-paste install sequence is acceptable for the preview if it avoids
  building the workspace and is verified. A single shell script is useful but
  not required unless Stage 10 chooses it as the lowest-risk path.
- Windows remains optional because the accepted issue marks it optional even
  though the broader vault release note wants eventual Windows coverage.

Constraints:

- Do not publish claims that the preview is stable, production-ready, signed,
  notarized, auto-updating, or package-manager distributed unless separately
  proven.
- Do not leave docs that recommend `cargo install duumbi` from crates.io unless
  a separate crates.io release decision has been implemented and verified.
- Do not make source-build contributors worse off while adding prebuilt preview
  artifacts.
- Do not create mutable or ambiguous release artifacts after users may have
  downloaded them. If a release mistake is found, document replacement policy
  explicitly rather than silently changing evidence.
- Do not use issue-closing references in spec-only PRs for this issue.

## Decisions

- **Decision:** Use GitHub Releases with prebuilt archives as the
  `v0.4.0-preview` install channel.
  **Evidence:** The issue asks for tagged release binaries or another working
  install path; no crates.io package exists; the release workflow already exists
  and is active; the vault release note names GitHub Releases binaries as the
  primary preview path.

- **Decision:** Keep crates.io publishing out of scope for #687.
  **Evidence:** The issue requires a working preview install path, not a
  long-term package-channel strategy. Publishing to crates.io would introduce
  naming, metadata, ownership, irreversible versioning, and support concerns
  that are not needed for the preview.

- **Decision:** Require checksums for release archives.
  **Evidence:** Prebuilt binary distribution needs a minimal integrity check.
  The current workflow uploads archives but does not publish checksums.

- **Decision:** Treat docs-site updates as required release behavior.
  **Evidence:** The accepted issue requires README plus docs-site install
  updates, and #686 established `hgahub/duumbi-web/docs` as the canonical public
  docs surface.

- **Decision:** Keep Windows optional and experimental if included.
  **Evidence:** The accepted issue says Windows is optional and should be marked
  experimental. README currently documents native Windows support boundaries but
  not installer or release packaging support.

## Behavior

The release behavior must be observable through GitHub, the release artifacts,
the installed binaries, and public docs.

Default channel:

- The default preview install path is a GitHub Release archive for the user's
  platform.
- Source build remains available as a fallback for contributors and unsupported
  platforms.
- Crates.io is not presented as the preview install channel.

Inputs:

- A merged implementation state on the release branch.
- A release tag named `v0.4.0-preview`.
- GitHub Actions release workflow configuration.
- Platform target matrix.
- README and docs-site install pages.

Outputs:

- A Git tag.
- A GitHub Release.
- Platform archives.
- Checksum files or a checksum manifest.
- Release notes.
- README/docs install instructions.
- Verification evidence in the issue and PR.

Success states:

- A user downloads the archive for a required platform, installs or places the
  binaries on PATH, and runs `duumbi --version`.
- `duumbi init`, `duumbi build`, and `duumbi run` work from the installed
  binary on at least one smoke fixture per required platform.
- README and docs-site instructions match the artifact names and commands.

Empty states:

- If a platform is not supported by the preview, docs must say so and direct the
  user to source build or future support rather than hiding the absence.
- If there are no releases before implementation, docs may continue to describe
  source build as the current fallback until release evidence exists.

Error states:

- If a tag-triggered release workflow fails, no successful preview release may
  be claimed.
- If an archive is missing required contents, that platform is not release-ready.
- If checksums are missing, the release is not release-ready.
- If install docs point to an artifact name or target that does not exist, the
  release is not release-ready.
- If a required platform smoke check fails, that platform is not release-ready.

Retry and cancellation:

- Failed release workflow runs may be retried before public release claims are
  made.
- Tag creation must happen only after release-prep implementation changes are
  merged and reviewed.
- If a release candidate cannot be verified, Stage 10 must stop with findings
  rather than publishing partial success.

Race and invariants:

- The tag, release notes, artifact names, version metadata, README, and docs
  must agree.
- Release artifacts must be generated from the tagged source, not from an
  unreviewed local build.
- The execution issue must remain open after spec PR merges and implementation
  PR merge until release evidence is verified by Stage 12.

Accessibility and focus:

- Installation docs must be copy-pasteable and scannable.
- Platform names, architecture names, and unsupported cases must be explicit.
- The docs must not require users to infer whether they need Rust or a C
  compiler for the prebuilt path.

## BDD Scenarios

Feature: DUUMBI developer-preview release

  Rule: The preview has a real release artifact

    Scenario: Required platform archives are attached to the preview release
      Given the release operator has pushed the `v0.4.0-preview` tag
      When the release workflow finishes successfully
      Then GitHub Releases shows a `v0.4.0-preview` release
      And the release includes archives for macOS aarch64, macOS x86_64, and Linux x86_64
      And each required archive has a matching checksum
      And the release notes identify the release as a developer/research preview

    Scenario: Release archives include the runtime files needed by DUUMBI programs
      Given a required-platform release archive has been downloaded
      When the archive is extracted
      Then it contains the `duumbi` executable
      And it contains the `studio` executable
      And it contains `runtime/duumbi_runtime.c`
      And it contains the vendored runtime third-party files needed by the C runtime

    Scenario: Version metadata identifies the preview
      Given a user has installed DUUMBI from a required-platform preview archive
      When the user runs `duumbi --version`
      Then the output identifies DUUMBI as the `0.4.0-preview` release or an equivalent semver pre-release build

  Rule: A new user can install without building the workspace

    Scenario: macOS user installs from the prebuilt preview path
      Given a macOS user is on a supported required architecture
      And the user has not cloned the DUUMBI repository
      When the user follows the README or docs-site preview install instructions
      Then `duumbi --version` runs successfully
      And the install path does not require building DUUMBI from source

    Scenario: Linux x86_64 user installs from the prebuilt preview path
      Given a Linux x86_64 user has a clean shell environment
      And the user has not cloned the DUUMBI repository
      When the user follows the README or docs-site preview install instructions
      Then `duumbi --version` runs successfully
      And the install path does not require a Rust toolchain to build DUUMBI

    Scenario: Unsupported platform guidance is explicit
      Given a user is on a platform without a preview archive
      When the user reads the install docs
      Then the docs identify that the preview archive is unavailable for that platform
      And the docs provide a source-build fallback or a clear future-support note
      And the docs do not imply signed installers or package-manager support

  Rule: Public docs match the shipped release

    Scenario: README and docs-site install instructions agree
      Given the `v0.4.0-preview` release exists with required artifacts
      When a reviewer compares `README.md` with the docs-site installation page
      Then both surfaces name the same preview install channel
      And both surfaces use artifact names that exist on the release
      And both surfaces identify source build as fallback rather than the primary preview install path

    Scenario: Docs no longer advertise an unavailable crates.io channel
      Given DUUMBI is not published to crates.io for this preview
      When a user reads the install docs
      Then the docs do not present `cargo install duumbi` from crates.io as the preview install path
      And any `cargo install --path .` guidance is clearly labeled as a source-checkout contributor flow

  Rule: Release readiness is evidence-backed

    Scenario: Required-platform smoke checks prove the installed binary works
      Given a required-platform preview archive has been installed in a clean or clean-enough environment
      When the verifier runs the documented smoke path
      Then `duumbi --version` succeeds
      And `duumbi init` creates a workspace
      And `duumbi build` succeeds for the default workspace
      And `duumbi run` runs the compiled output successfully

    Scenario: Failed release workflow blocks release claims
      Given the tag-triggered release workflow fails for a required platform
      When the implementation agent reports release status
      Then the agent reports the failure as blocking
      And the agent does not claim that `v0.4.0-preview` is release-ready

## Tasks

- Confirm current release state: tags, releases, workflow activity, package
  versions, README install docs, and docs-site install docs.
- Prepare release metadata and versioning changes for `0.4.0-preview`.
- Harden the release workflow for required platform archives, checksums, and
  release notes.
- Decide whether to keep Linux ARM64 in the workflow and document it if kept.
- Decide whether to add Windows x86_64 as an experimental archive; do not block
  #687 on Windows unless Stage 10 deliberately takes it on.
- Update README install guidance after the artifact naming and commands are
  final.
- Update docs.duumbi.dev install and quickstart references in `hgahub/duumbi-web`.
- Validate release workflow behavior before tag publication as far as safely
  possible.
- After review and merge, create the `v0.4.0-preview` tag and verify the
  GitHub Release artifacts.
- Run clean-machine or runner-based install smoke checks for required platforms.
- Record evidence links in the issue and implementation PR.

Independent slices:

- Version and release-workflow preparation can be reviewed before tag creation.
- README and docs-site install copy can be drafted with placeholders, then
  finalized after artifact naming is confirmed.
- Required platform smoke evidence can be collected independently per platform.
- Windows experimental work, if attempted, should be isolated so it cannot block
  required platform release readiness.

## Checks

- Product spec review against this document and #687.
- Technical spec maps every BDD scenario to concrete test, workflow, manual, or
  release evidence.
- PR checks for release-prep code/docs changes:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo nextest run --workspace`
  - `cargo audit`
  - docs-site `npm run build` when `hgahub/duumbi-web/docs` changes
- Release workflow dry-run or equivalent review evidence for matrix, archive
  contents, artifact names, checksums, and release notes.
- GitHub API or `gh` evidence that `v0.4.0-preview` tag and release exist after
  release publication.
- Archive inspection for each required platform.
- Checksum verification for each required platform.
- Install smoke evidence for macOS aarch64, macOS x86_64, and Linux x86_64.
- README/docs-site text review showing no unavailable crates.io install claim.
- Stage 11 review evidence before implementation merge.
- Stage 12 closure evidence before the issue is completed.

## Open Questions

None blocking.

Non-blocking decisions for Stage 10:

- Whether Linux ARM64 remains in the preview matrix as a supported artifact or
  is explicitly labeled extra/experimental.
- Whether Windows x86_64 is attempted for this preview or deferred.
- Whether the preview install path is a short manual archive-install sequence
  or a small install script plus manual fallback.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/687
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/687#issuecomment-4702626396
- Stage 4 triage comment:
  https://github.com/hgahub/duumbi/issues/687#issuecomment-4697632616
- GitHub release workflow: `.github/workflows/release.yml`
- Ready-for-build handoff workflow: `.github/workflows/ready-for-build-handoff.yml`
- Spec AI gate workflow: `.github/workflows/spec-ai-gate.yml`
- Stage approval workflow: `.github/workflows/stage-approval.yml`
- Source package metadata: `Cargo.toml`, `crates/duumbi-studio/Cargo.toml`
- README install docs: `README.md`
- Canonical docs installation page:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/getting-started/installation.md`
- Canonical docs quickstart:
  `/Users/heizergabor/space/hgahub/duumbi-web/docs/src/content/docs/getting-started/quickstart.md`
- DUUMBI docs audit from #686: `docs/duumbi-686-docs-audit.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
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
- Spec-first agentic development dot:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Spec-First Agentic Development.md`
- AI review service policy dot:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/AI Code Review Service Policy.md`
