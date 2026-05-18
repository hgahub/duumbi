# DUUMBI-420: Add Windows CI Coverage To The Main Rust Checks

## Summary

Add native Windows coverage to DUUMBI's main GitHub Actions CI workflow so the
core Rust quality gates run on `windows-latest` as well as Ubuntu when a change
requires Rust validation.

This is the first Phase 16 CI infrastructure step. It should prove that DUUMBI's
baseline Rust build and test suite can run on a GitHub-hosted Windows runner
with the stable MSVC Rust toolchain, while keeping documentation/spec-only PRs
lightweight and leaving Windows terminal behavior and README support claims to
their own Phase 16 issues.

## Problem

DUUMBI currently describes Windows as unsupported in the README, while the Phase
16 kill criterion requires `cargo test --all` to pass on `windows-latest` with
the MSVC toolchain before Windows support can become credible. The active CI
workflow only runs its Rust checks on `ubuntu-latest`, so regressions in
Windows-specific path handling, process behavior, linking, dependency setup, or
test assumptions are invisible until someone performs manual testing.

The immediate product risk is false confidence. Without a native Windows CI
signal, the project cannot tell contributors whether a change preserves the
minimum Phase 16 compatibility contract.

## Outcome

When this is done:

- The main CI workflow has native Windows coverage for Rust changes.
- Non-doc/spec PRs run Rust checks on both `ubuntu-latest` and
  `windows-latest`.
- The Windows CI path runs `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo build`, and
  `cargo test --all`.
- Dependency audit remains a required CI signal for non-doc/spec PRs, but it
  does not have to run separately on every operating system.
- Documentation/spec-only PRs still avoid unnecessary Rust checks.
- Pushes to `main` run Rust checks regardless of changed file scope.
- Windows failures in the required Rust checks fail CI; the Windows path is not
  advisory or allowed to fail silently.
- Any Windows-specific behavior discovered during CI bring-up is either handled
  within the narrow CI bring-up scope or recorded as a linked Phase 16 follow-up
  before the execution issue is considered ready for later closure stages.
- The execution issue remains open after this product-spec PR because Stage 7,
  Stage 8, implementation, review, and closure still need to happen.

## Scope

### In Scope

- Update `.github/workflows/ci.yml` so Rust-checking CI runs on
  `ubuntu-latest` and `windows-latest` when Rust checks are required.
- Preserve the existing trigger behavior for pushes to `main` and pull
  requests.
- Preserve the existing docs/spec-only skip behavior for pull requests that
  modify only `docs/` and `specs/`.
- Ensure the Windows CI path installs a stable Rust toolchain with `rustfmt` and
  `clippy` support.
- Ensure the Windows CI path runs formatting, clippy, build, and the full test
  suite.
- Keep dependency audit as part of required CI for non-doc/spec PRs, while
  allowing it to run once instead of once per OS.
- Record any Windows-specific CI limitation or discovered platform behavior in
  review evidence or a linked follow-up issue.
- Keep Phase 16 milestone traceability for the Windows CI kill criterion.

### Explicitly Out Of Scope

- Terminal color fallback testing and `anstream` Windows terminal capability
  verification; that remains #422.
- README or public documentation changes that declare Windows requirements or
  support status; that remains #423.
- Reopening the completed module path separator audit; #421 is already closed
  with audit evidence.
- WSL2-specific testing.
- ARM64 Windows runner support.
- MinGW, Cygwin, or GNU Windows toolchain support.
- Windows packaging, installer, release signing, or distribution work.
- Broad CLI UX changes, Studio work, registry behavior, provider behavior,
  terminal color policy, or self-healing/telemetry work.
- Technical specifications, implementation code, or Ralph-cycle instructions in
  this Stage 6 artifact.

## Constraints And Assumptions

Facts:

- Issue #420 is open, labeled `accepted` and `needs-spec`, and in Project status
  `Spec Needed`.
- Stage 5 accepted #420 on 2026-05-18 and routed it to `Spec Needed`.
- The Phase 16 kill criterion requires `cargo test --all` to pass on
  `windows-latest` with the MSVC toolchain.
- `.github/workflows/ci.yml` currently has one `check` job running on
  `ubuntu-latest`.
- The current CI workflow skips Rust checks for pull requests that change only
  files under `docs/` or `specs/`.
- The current CI workflow runs `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo audit`, `cargo build`,
  and `cargo test --all` when Rust checks are required.
- README currently lists Windows as not supported.
- #421 is closed with path separator audit evidence.
- #422 is open for Windows terminal color fallback testing.
- #423 is open for Windows requirements documentation.

Assumptions:

- GitHub-hosted `windows-latest` provides the MSVC environment expected by the
  stable Rust toolchain action.
- `cargo fmt --check` and clippy are useful cross-platform hygiene checks even
  though their main value is not Windows-specific.
- `cargo audit` is a dependency vulnerability check, not a native Windows
  compatibility proof, so running it once per non-doc/spec PR is enough for the
  product contract.
- Some Windows failures may expose real code or test assumptions. Narrow
  adjustments required to make the baseline CI path pass may remain part of the
  eventual implementation, but broader support work should be split into linked
  Phase 16 follow-ups.
- The implementation PR can rely on GitHub Actions as the authoritative Windows
  runtime evidence because local macOS/Linux workstations cannot reproduce the
  hosted Windows runner.

Constraints:

- The Windows CI result must be a required quality signal for Rust-relevant
  changes, not a best-effort advisory signal.
- The implementation must not weaken existing Ubuntu CI coverage.
- The implementation must not remove docs/spec-only skip behavior.
- The implementation must not use the product-spec PR to close the execution
  issue. Use non-closing references such as `Spec for #420` or
  `Related to #420`.
- The implementation should keep CI time and network work reasonable by avoiding
  duplicated dependency-audit installs unless Stage 8 deliberately requires
  otherwise.
- The README support table must stay conservative until #423 explicitly updates
  Windows user-facing support requirements.

## Decisions

- **Decision:** Use a file-based product spec for #420.
  **Evidence:** The work changes CI infrastructure, affects contributor trust,
  supports a Phase 16 kill criterion, and should be durable context for Stage 7,
  Stage 8, implementation, and review.

- **Decision:** Windows CI must run the platform-sensitive Rust checks:
  formatting, clippy, build, and `cargo test --all`.
  **Evidence:** The issue asks for native Windows CI coverage, and the Phase 16
  kill criterion specifically depends on the test suite passing on
  `windows-latest`.

- **Decision:** `cargo audit` must remain required for non-doc/spec PRs, but it
  does not need to run on both Ubuntu and Windows.
  **Evidence:** Dependency vulnerability auditing is not a Windows compatibility
  proof. Running it once keeps the existing quality signal while avoiding
  duplicated install/network work and reducing Windows bring-up noise.

- **Decision:** The docs/spec-only skip behavior remains part of the CI product
  contract.
  **Evidence:** The current workflow already avoids Rust checks for PRs that
  change only `docs/` and `specs/`, and spec-only PRs are common in the DUUMBI
  Stage 6 and Stage 8 workflow.

- **Decision:** #420 does not declare Windows supported in user-facing docs.
  **Evidence:** README Windows support requirements are owned by #423 and should
  wait for native CI evidence and any remaining Phase 16 requirements.

- **Decision:** #420 does not verify terminal color fallback.
  **Evidence:** Windows terminal capability testing is separately tracked in
  #422 and would expand this CI matrix issue beyond its accepted scope.

- **Decision:** This spec PR must not close #420.
  **Evidence:** Stage 6 creates a reviewable product spec only. Stage 7 product
  review, Stage 8 technical specification, Stage 9 technical review,
  implementation, Stage 11 review, and Stage 12 closure remain separate gates.

## Behavior

### Defaults

- CI continues to run on pushes to `main` and pull requests.
- Pull requests with Rust-relevant changes run required CI on Ubuntu and
  Windows.
- Pull requests with only `docs/` and `specs/` changes skip Rust checks and
  report that Rust checks are not required.
- Pushes to `main` run Rust checks regardless of changed file scope.
- Windows uses the stable Rust toolchain with `rustfmt` and `clippy`
  components.
- Windows CI failures in required checks fail the workflow.
- Dependency audit remains required once per non-doc/spec PR.

### Inputs

- GitHub event type: push to `main` or pull request.
- Changed file list for pull requests.
- Existing Rust workspace and lockfile.
- GitHub-hosted runner OS: `ubuntu-latest` and `windows-latest`.
- Stable Rust toolchain installation behavior on each runner.

### Outputs

- GitHub Actions summary and check results showing Ubuntu Rust checks.
- GitHub Actions summary and check results showing Windows Rust checks.
- A dependency-audit result for non-doc/spec PRs.
- A documentation/spec-only PR path that avoids Rust checks and exits
  successfully with an explicit skip message.
- Review evidence linking to the relevant workflow runs.

### Visible States

- Rust-relevant PR with all checks passing: Ubuntu and Windows Rust checks pass;
  dependency audit passes; CI is green.
- Rust-relevant PR with Windows-only failure: Windows check fails; CI is red; the
  failure is visible in the PR checks.
- Docs/spec-only PR: CI reports the docs/spec-only skip path; Rust check steps do
  not run.
- Push to `main`: Rust checks run even if the pushed commit happens to touch
  docs/spec files only.

### Error States

- Toolchain installation failure on Windows fails the Windows CI path.
- Build or test failure on Windows fails the Windows CI path.
- If dependency audit fails, the required audit signal fails the workflow even
  when OS-specific Rust checks pass.
- If docs/spec-only detection cannot determine changed files, CI should prefer
  running Rust checks rather than skipping them silently.

### Cancellation And Retry

- Cancelled GitHub Actions runs behave as normal cancelled checks and do not
  count as passing evidence.
- Retried runs may be used as review evidence if the final run is linked and the
  reason for retry is recorded when relevant.

### Race Conditions And Invariants

- Changed-file detection must compare the pull request head against the correct
  base branch.
- The docs/spec-only skip decision must be identical across OS-specific Rust
  jobs if the implementation uses a matrix.
- Windows CI must not be marked allowed-to-fail in the final implementation.
- Ubuntu coverage must remain present after adding Windows coverage.

### Accessibility And Focus

- No user-facing app accessibility or keyboard-focus behavior changes are part
  of this issue.
- GitHub Actions step names should remain clear enough for reviewers to identify
  Ubuntu, Windows, dependency audit, docs/spec-only skip, build, and test
  outcomes without reading raw logs.

## BDD Scenarios

Feature: Native Windows CI coverage for DUUMBI Rust checks

  Rule: Rust-relevant pull requests run checks on Ubuntu and Windows

    Scenario: A pull request changes Rust source code
      Given a pull request changes a file outside `docs/` and `specs/`
      When the CI workflow runs for the pull request
      Then the workflow runs Rust checks on `ubuntu-latest`
      And the workflow runs Rust checks on `windows-latest`
      And the Windows path runs `cargo fmt --check`
      And the Windows path runs `cargo clippy --all-targets -- -D warnings`
      And the Windows path runs `cargo build`
      And the Windows path runs `cargo test --all`

    Scenario: Windows-only Rust failure blocks the pull request
      Given a pull request requires Rust checks
      And the Ubuntu checks pass
      And `cargo test --all` fails on `windows-latest`
      When GitHub reports the workflow result
      Then the CI result is failing
      And the Windows failure is visible in the pull request checks

  Rule: Documentation and specification changes stay lightweight

    Scenario: A pull request changes only docs and specs
      Given a pull request changes only files under `docs/` and `specs/`
      When the CI workflow evaluates the changed files
      Then Rust check steps do not run
      And CI reports that Rust checks are not required for the PR
      And the workflow can pass without installing the Rust toolchain

    Scenario: Changed-file detection is unavailable
      Given a pull request CI run cannot determine the changed file list
      When the workflow decides whether Rust checks are required
      Then the workflow runs Rust checks
      And it does not silently skip required validation

  Rule: Pushes to main always prove baseline health

    Scenario: A commit is pushed to main
      Given the CI workflow is triggered by a push to `main`
      When the workflow runs
      Then Rust checks run on `ubuntu-latest`
      And Rust checks run on `windows-latest`
      And the docs/spec-only skip path is not used

  Rule: Dependency audit remains required without duplicating OS work

    Scenario: A pull request requires Rust checks
      Given a pull request changes a file outside `docs/` and `specs/`
      When CI runs
      Then a dependency-audit check runs and reports a required result
      And the product contract does not require a separate audit run on every OS

  Rule: Adjacent Phase 16 work remains separate

    Scenario: Windows CI coverage is added before documentation support claims
      Given the Windows Rust checks pass in CI
      When #420 reaches implementation review
      Then #420 review evidence links the Windows CI run
      But README Windows support requirements remain out of scope for #420
      And terminal color fallback verification remains out of scope for #420

## Tasks

- Update the CI workflow structure so Rust checks can run on both Ubuntu and
  Windows without removing the docs/spec-only skip path.
- Keep or introduce a single required dependency-audit path for non-doc/spec
  PRs.
- Verify Windows Rust toolchain setup, cache behavior, build, clippy, and
  `cargo test --all` on GitHub Actions.
- Review any Windows-only failures and decide whether each one is narrow CI
  bring-up scope or a linked Phase 16 follow-up.
- Capture implementation review evidence with workflow run links and any
  discovered limitations.

The workflow-structure work and dependency-audit placement can be reasoned about
independently during Stage 8. The Windows runtime proof depends on a GitHub
Actions run from the implementation PR.

## Checks

- Static review of `.github/workflows/ci.yml` confirms:
  - `ubuntu-latest` and `windows-latest` Rust-check paths exist.
  - Pull request docs/spec-only detection is preserved.
  - Pushes to `main` do not use docs/spec-only skipping.
  - Windows is not allowed to fail.
  - Dependency audit still runs for non-doc/spec PRs at least once.
- If available, run `actionlint` or an equivalent workflow syntax check.
- Implementation PR evidence includes a GitHub Actions run for a Rust-relevant
  change showing:
  - Ubuntu Rust checks passed.
  - Windows Rust checks passed.
  - `cargo test --all` passed on `windows-latest`.
  - Dependency audit produced a required result.
- Implementation review records how docs/spec-only skip behavior was preserved.
  Evidence may be a linked docs/spec-only workflow run, workflow-logic review, or
  another reviewer-verifiable artifact.
- No README Windows support claims are changed under #420 unless #423 is
  explicitly routed into the same later implementation scope by a human.
- No terminal color fallback checks are added under #420 unless #422 is
  explicitly routed into the same later implementation scope by a human.
- Stage 11 review should verify BDD scenario coverage against the final PR
  checks and implementation notes.

## Open Questions

None blocking for Stage 7 review.

Non-blocking implementation detail: Stage 8 can choose whether the Windows job
uses Git Bash, PowerShell, or split shell steps. The product contract is the
observable CI behavior, not a specific shell implementation.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/420
- Stage 5 decision comment:
  https://github.com/hgahub/duumbi/issues/420#issuecomment-4477465951
- Related issue #421: https://github.com/hgahub/duumbi/issues/421
- Related issue #422: https://github.com/hgahub/duumbi/issues/422
- Related issue #423: https://github.com/hgahub/duumbi/issues/423
- CI workflow: `.github/workflows/ci.yml`
- README supported platforms table: `README.md`
- Active PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Agentic Development Runbook:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Development workflow:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Development Intake to Delivery Workflow.md`
- Phase 16 roadmap:
  `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 16 - Windows & Cross-Platform Support.md`
- Existing product spec format examples:
  `specs/DUUMBI-553/PRODUCT.md`, `specs/DUUMBI-556/PRODUCT.md`
