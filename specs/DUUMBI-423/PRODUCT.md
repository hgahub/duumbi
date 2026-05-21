# DUUMBI-423: Document Native Windows Requirements In README

## Summary

Update DUUMBI's user-facing Windows setup documentation so a native Windows
developer can understand the current support boundary, required Windows version,
required Rust/MSVC toolchain setup, and remaining limitations before installing
or building DUUMBI.

This spec covers README-level Windows support guidance and the public
installation page because both are onboarding surfaces. Keeping one stale while
updating the other would preserve the product problem: users would still receive
contradictory platform guidance.

This is a specification artifact only. The linked execution issue must remain
open for Stage 7 review, Stage 8 technical specification, implementation,
review, and later completion evidence.

## Problem

Phase 16 exists to remove a visible Windows credibility gap. #420 has already
established a native `windows-latest` CI baseline, and #422 has an approved
product spec for terminal color fallback coverage. The public documentation
still sends a stale negative signal:

- `README.md` lists Windows as not supported.
- `sites/docs/src/getting-started/installation.md` says Windows is not supported
  in the current release.
- `.github/workflows/ci.yml` now includes `windows-latest`, so the docs no
  longer match the repository's platform evidence.

The result is user confusion. A Windows developer can see native CI evidence in
the repo but still be told not to try Windows from the primary onboarding
surfaces. The documentation needs to state the actual boundary: native Windows
support is being completed under Phase 16, the supported target is Windows 10
version 1903+ with the MSVC toolchain, baseline build/test CI exists, and
remaining Phase 16 limitations must not be hidden.

## Outcome

When this is done:

- README no longer gives a blanket stale "Windows not supported" signal.
- The public installation page does not contradict README Windows guidance.
- Windows users can identify the native Windows target before installing:
  Windows 10 version 1903+ on x86_64 with the MSVC Rust toolchain.
- Windows users can identify required build prerequisites: Rust stable MSVC,
  Visual Studio Build Tools or equivalent MSVC C++ tools, Windows SDK, and a
  usable linker/C compiler environment.
- The docs explain that WSL2 is not required for the native Windows target.
- The docs clearly exclude unsupported or unproven targets such as ARM64
  Windows, MinGW/Cygwin, installers, release signing, and packaging.
- The docs distinguish baseline native CI evidence from full Phase 16
  completion, especially while #422 terminal color fallback implementation is
  not complete.
- The docs remain consistent with #420 Windows CI evidence and #422 terminal
  color fallback scope.
- The execution issue remains open after this product-spec PR because later
  workflow stages still need to review, specify, implement, and verify the
  actual documentation changes.

## Scope

### In Scope

- Update README supported-platform and installation/support guidance for native
  Windows.
- Update `sites/docs/src/getting-started/installation.md` only as needed to keep
  the public installation page aligned with README.
- Document Windows 10 version 1903+ as the accepted minimum native Windows
  target unless Stage 7 changes that decision.
- Document `x86_64-pc-windows-msvc` / stable MSVC Rust toolchain expectations in
  user-facing language.
- Document Visual Studio Build Tools, MSVC C++ build tools, Windows SDK, and
  linker/C compiler environment expectations where relevant.
- Explain the expected user path for `cargo install duumbi`, `duumbi init`, and
  local source builds on native Windows.
- State limitations without overclaiming mature Windows support before the rest
  of Phase 16 evidence exists.
- Keep wording consistent with #420 and #422.
- Add or run focused documentation checks if existing local patterns support
  them.

### Explicitly Out Of Scope

- Implementing or changing Windows CI behavior already covered by #420.
- Implementing terminal color fallback checks already covered by #422.
- Implementing runtime, compiler, credential, path, shell, or terminal behavior
  changes.
- Adding Windows packaging, installers, release signing, ARM64 Windows, WSL2
  testing, MinGW, Cygwin, or GNU Windows toolchain support.
- Updating marketing launch copy, Phase 14 content, registry behavior, provider
  setup, Studio behavior, semantic rewrite-engine architecture, telemetry, or
  self-healing work.
- Creating technical specs, implementation code, or Ralph cycles in this Stage 6
  artifact.

## Constraints And Assumptions

Facts:

- Issue #423 is open, accepted by Stage 5 on 2026-05-20, labeled `accepted` and
  `needs-spec`, and in GitHub Project status `Spec Needed` at Stage 6 intake.
- The Stage 5 decision explicitly accepted #423 and routed it to `Spec Needed`.
- The Stage 4 triage body identifies #423 as the canonical issue for README
  Windows requirements documentation.
- The current milestone is Phase 16: Windows & Cross-Platform Support.
- The Phase 16 roadmap note defines the kill criterion as `cargo test --all`
  passing on `windows-latest` with the MSVC toolchain, plus documented Windows
  install requirements for a Windows 10 version 1903+ user running `cargo
  install duumbi` and `duumbi init` without WSL2.
- #420 is completed; PR #571 was merged on 2026-05-19 and added the
  `ubuntu-latest` / `windows-latest` CI matrix plus narrow Windows CI bring-up
  changes.
- The current `.github/workflows/ci.yml` includes `windows-latest` in the main
  check matrix.
- #422 is open in `Technical Spec Needed`; its product spec was approved on
  2026-05-20, but terminal color fallback implementation is not complete yet.
- `README.md` currently lists Windows as not supported.
- `sites/docs/src/getting-started/installation.md` currently says Windows is not
  supported in the current release.
- DUUMBI public docs are treated as a product interface in the active Atlas
  context; stale public docs create product drift.

Assumptions:

- The intended native Windows target is x86_64 Windows with the MSVC toolchain,
  not MinGW/Cygwin/GNU, because Phase 16 and #420 are built around GitHub-hosted
  `windows-latest` and the stable MSVC Rust target.
- Windows 10 version 1903+ remains the product minimum because it is explicitly
  named in the Phase 16 kill criterion and no stronger replacement evidence was
  found during Stage 6.
- The docs should be conservative: they should acknowledge native CI progress
  and practical setup requirements without implying that every Windows
  distribution, terminal, or packaging path is complete.
- A README-only update would leave an obvious contradiction in the public
  installation page, so aligning that page is part of the product scope.
- The eventual implementation PR is expected to be documentation-only unless
  Stage 8 finds an existing docs validation hook that needs metadata-only
  support.

Constraints:

- The implementation must not weaken or alter the #420 CI baseline.
- The implementation must not add #422 terminal behavior tests under #423.
- The implementation must not claim Phase 16 as fully complete until the
  remaining Phase 16 evidence exists.
- Public docs should describe user-relevant support boundaries, not internal
  workflow details, except where issue/PR references appear in implementation or
  review evidence.
- Spec-only PRs must use non-finalizing references such as `Spec for #423` or
  `Related to #423`; the execution issue stays open for the later workflow
  stages.

## Decisions

- **Decision:** Use a file-based product spec for #423.
  **Evidence:** The work touches public onboarding, Phase 16 platform claims,
  README/docs consistency, and downstream implementation/review criteria. It is
  durable implementation context, not a small issue-comment-only clarification.

- **Decision:** Align both README and the public installation page.
  **Evidence:** Source inspection found both surfaces currently contain stale
  Windows-negative guidance. Updating only README would leave a public docs
  contradiction for the same installation journey.

- **Decision:** Use Windows 10 version 1903+ and x86_64 MSVC as the product
  baseline for this spec.
  **Evidence:** The Phase 16 roadmap note explicitly names Windows 10 version
  1903+, MSVC, `cargo install duumbi`, `duumbi init`, and no WSL2 as the
  documentation kill criterion.

- **Decision:** State Windows support conservatively until the remaining Phase
  16 evidence is complete.
  **Evidence:** #420 is complete, but #422 terminal color fallback work has only
  passed product spec review and still needs later workflow stages.

- **Decision:** Keep implementation issue references out of polished public
  docs unless Stage 8 deliberately chooses a contributor-facing note.
  **Evidence:** Public docs should describe dependable user behavior and support
  boundaries. GitHub issues and PRs remain the execution state and review
  evidence, not normal installation instructions.

- **Decision:** This spec PR must keep #423 open.
  **Evidence:** Stage 6 creates a reviewable product spec only. Product review,
  technical specification, technical review, implementation, review, and final
  completion evidence remain separate gates.

## Behavior

### Defaults

- README remains the primary repository-level onboarding surface.
- The public installation page remains the docs-site onboarding surface for
  install/build prerequisites.
- Both surfaces describe the same Windows support boundary.
- Windows is no longer represented by an unqualified "not supported" row or
  sentence after #423 implementation.
- Native Windows support is described as an x86_64 MSVC target with explicit
  prerequisites and limitations.
- Existing macOS and Linux support wording remains intact unless a small wording
  adjustment is required for table consistency.

### Inputs

- A Windows user reading README before installation.
- A Windows user reading `docs.duumbi.dev` installation guidance before
  installation.
- A contributor reading repository docs before building from source.
- Current Phase 16 evidence from #420 and #422.
- Local source files `README.md` and
  `sites/docs/src/getting-started/installation.md`.
- Existing CI docs/spec-only behavior.

### Outputs

- README platform/install guidance that names native Windows requirements and
  support limits.
- Public installation docs with matching Windows requirements and support
  limits.
- Review evidence showing no stale unsupported Windows statement remains in the
  affected onboarding surfaces.
- Review evidence showing docs changes did not alter Windows CI, terminal color
  fallback behavior, implementation code, generated assets, or technical specs.

### Visible States

- Existing macOS or Linux user: sees no regression in supported-platform
  guidance.
- Native Windows user with Windows 10 version 1903+ and MSVC tools: sees a
  documented path for `cargo install duumbi`, `duumbi init`, and source builds.
- Windows user without MSVC tools: sees that MSVC C++ build tools and Windows
  SDK/linker setup are prerequisites, not optional troubleshooting trivia.
- Windows user using MinGW/Cygwin/GNU, ARM64 Windows, or installer/package
  expectations: sees those targets are not covered by this support boundary.
- Contributor reviewing Phase 16: sees that docs match the #420 CI baseline but
  do not overstate #422 terminal color fallback completion.

### Empty States

- If the docs mention Windows only in platform tables, the implementation is not
  sufficient; the user also needs prerequisite and limitation guidance.
- If a docs validation tool is unavailable locally, review evidence should
  record the unavailable check and rely on diff review plus any available
  markdown/static checks.

### Error States

- README still says Windows is not supported after implementation.
- The public installation page still says Windows is not supported after
  implementation.
- README and the public installation page give different Windows minimum
  versions, toolchain requirements, or support boundaries.
- Docs imply ARM64, MinGW/Cygwin, GNU toolchain, installer, release signing, or
  WSL2-specific support that this issue does not cover.
- Docs claim terminal color fallback coverage is complete before #422
  implementation evidence exists.
- The implementation changes CI, Rust source, tests, generated assets, technical
  specs, or unrelated product docs under #423.

### Cancellation And Retry

- A cancelled docs-check run does not count as evidence that the documentation
  artifact is valid.
- Retried checks may be used as evidence when the final linked run is clear.
- If documentation checks are unavailable, Stage 10 and Stage 11 evidence should
  explicitly state that and use focused static review instead.

### Race Conditions And Invariants

- README and public installation docs must not drift during the same
  implementation PR.
- The Windows support boundary must be based on current GitHub/source evidence,
  not archived roadmap text alone.
- #423 must not depend on #422 implementation being complete to document
  prerequisite requirements, but it must not claim #422 behavior as complete.
- The docs must make the native Windows path understandable without requiring
  users to read GitHub issue history.
- The execution issue must remain open after spec-only PR handling.

### Accessibility And Focus

- The guidance should be scannable in both markdown source and rendered docs.
- Requirements should not be communicated only through table status text; users
  need short prose for prerequisites and limitations.
- Link text should be descriptive where links are added or changed.
- The docs should avoid ambiguous status words that force users to infer whether
  they can try native Windows today.

## BDD Scenarios

Feature: Native Windows requirements documentation for DUUMBI

  Rule: README gives accurate native Windows setup guidance

    Scenario: Windows user finds the native support target in README
      Given a Windows developer opens `README.md`
      When they read the supported platforms section
      Then Windows is not described by a blanket unsupported statement
      And the documented Windows target includes x86_64 MSVC
      And the guidance names Windows 10 version 1903+ as the minimum target

    Scenario: Windows user finds required build prerequisites in README
      Given a Windows developer wants to install or build DUUMBI natively
      When they read README installation or support guidance
      Then the docs identify the stable MSVC Rust toolchain requirement
      And the docs identify Visual Studio Build Tools or equivalent MSVC C++ tools
      And the docs identify Windows SDK/linker or C compiler setup as required

  Rule: Public installation docs match README

    Scenario: Public installation page no longer contradicts README
      Given README documents native Windows requirements
      When a user opens `sites/docs/src/getting-started/installation.md`
      Then the public installation page does not say Windows is unsupported
      And it describes the same Windows minimum version and MSVC toolchain target
      And it does not introduce a broader support claim than README

    Scenario: Existing macOS and Linux guidance remains intact
      Given the installation docs are updated for Windows
      When a macOS or Linux user reads the supported-platform guidance
      Then the existing macOS and Linux support status remains clear
      And the Windows update does not remove existing Rust or C compiler prerequisites

  Rule: Support boundaries stay conservative and user-visible

    Scenario: Unsupported Windows variants are not implied
      Given the docs describe native Windows support
      When a user checks limitations
      Then ARM64 Windows is not implied as supported
      And MinGW/Cygwin/GNU Windows toolchains are not implied as supported
      And installers, packaging, and release signing are not implied as available

    Scenario: WSL2 is not required for the native Windows target
      Given a Windows 10 version 1903+ user has the MSVC Rust toolchain available
      When they read the Windows requirements
      Then the docs make clear that the native Windows path does not require WSL2
      But the docs do not need to test or document a WSL2-specific path

    Scenario: Terminal color fallback work is not overstated
      Given #422 terminal color fallback implementation evidence is not complete
      When #423 documentation is implemented
      Then the docs do not claim final Phase 16 terminal behavior completion
      And review evidence confirms #422 remains the terminal fallback evidence source

  Rule: The work remains documentation-only

    Scenario: Documentation implementation does not change platform behavior
      Given #423 is a documentation requirements issue
      When the implementation PR is reviewed
      Then changed files are limited to README/public installation docs and any
      minimal docs metadata required by existing validation patterns
      And no Rust source, tests, CI behavior, generated assets, technical specs,
      or Ralph cycles are introduced under this issue

    Scenario: Stale Windows-negative text is removed from onboarding surfaces
      Given implementation is complete
      When reviewers search affected onboarding docs for Windows support text
      Then stale blanket "Windows is not supported" language is absent
      And the replacement text states requirements and limitations instead

## Tasks

- Confirm the live issue state and Stage 5 acceptance before Stage 8 work starts.
- Read the current README supported-platform section and installation guidance.
- Read `sites/docs/src/getting-started/installation.md` and align its Windows
  prerequisites and platform table with README.
- Draft concise user-facing Windows requirements language:
  - Windows 10 version 1903+.
  - x86_64 MSVC Rust target.
  - Rust stable MSVC toolchain.
  - Visual Studio Build Tools or equivalent MSVC C++ toolchain.
  - Windows SDK/linker/C compiler environment.
  - WSL2 not required for the native path.
  - ARM64, MinGW/Cygwin/GNU, packaging, installers, and release signing not
    covered.
- Keep #420 and #422 references in PR/issue evidence, not necessarily in public
  docs copy.
- Run focused docs/static checks available in the repo, or record why a check is
  unavailable.
- Review the diff to confirm no implementation code, technical specs, or
  unrelated docs changed.

The README and public installation page wording can be drafted together. Any
docs validation can run independently after the text exists. The final review
evidence depends on the implementation PR diff and checks.

## Checks

- Product spec review confirms all Stage 5 questions are answered:
  README and public installation docs are in scope, Windows 10 version 1903+ is
  the baseline, and the support statement remains conservative until the
  remaining Phase 16 evidence exists.
- Stage 8 technical spec maps every BDD scenario to static docs review, docs
  validation, CI evidence, or PR diff review.
- Static text review confirms README no longer contains the stale blanket
  Windows unsupported statement.
- Static text review confirms the public installation page no longer contains
  the stale blanket Windows unsupported statement.
- Static text review confirms README and the public installation page agree on
  Windows 10 version 1903+, x86_64 MSVC, Rust stable MSVC, Visual Studio Build
  Tools/MSVC C++ tools, Windows SDK/linker/C compiler setup, WSL2 boundary, and
  unsupported targets.
- PR diff review confirms #420 CI files are unchanged under #423.
- PR diff review confirms #422 terminal color behavior files/tests are unchanged
  under #423.
- PR diff review confirms no technical specs, implementation code, generated
  assets, or Ralph-cycle artifacts are added under #423.
- Existing docs checks, markdown checks, or CI checks run if available; if none
  are available locally, implementation evidence records that limitation.
- Spec-only PR wording uses non-finalizing references such as `Spec for #423` or
  `Related to #423` and includes a workflow note that the execution issue stays
  open.

## Open Questions

- None blocking for Stage 7 product spec review.

Non-blocking questions for Stage 8:

- Should the implementation use a compact README table footnote, a short
  Windows subsection, or both to keep the support boundary scannable?
- Should the public installation page duplicate the README requirements or link
  to README as the source, given docs.duumbi.dev should remain useful without
  repository context?
- If #422 completes before #423 implementation starts, should the final docs
  wording be updated from "terminal fallback evidence pending" to a completed
  Phase 16 terminal behavior statement?

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/423
- Stage 5 human acceptance decision:
  https://github.com/hgahub/duumbi/issues/423#issuecomment-4498707005
- Related Windows CI baseline issue: https://github.com/hgahub/duumbi/issues/420
- Merged Windows CI implementation PR:
  https://github.com/hgahub/duumbi/pull/571
- Related terminal color fallback issue:
  https://github.com/hgahub/duumbi/issues/422
- Product spec baseline for #422: `specs/DUUMBI-422/PRODUCT.md`
- Current CI workflow: `.github/workflows/ci.yml`
- README supported platforms table: `README.md`
- Public installation docs:
  `sites/docs/src/getting-started/installation.md`
- External repo (`hgahub/duumbi-vault`) - Active PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- External repo (`hgahub/duumbi-vault`) - Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- External repo (`hgahub/duumbi-vault`) - Agentic Development Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- External repo (`hgahub/duumbi-vault`) - Agentic Development Runbook:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- External repo (`hgahub/duumbi-vault`) - Public Docs as Product Interface:
  `Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Public Docs as Product Interface.md`
- External repo (`hgahub/duumbi-vault`) - Service and Research Direction:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- External repo (`hgahub/duumbi-vault`) - Phase 16 roadmap note:
  `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 16 - Windows & Cross-Platform Support.md`
- External repo (`hgahub/duumbi-vault`) - Product Roadmap 2026-05:
  `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Product Roadmap 2026-05.md`
