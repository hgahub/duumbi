# DUUMBI-422: Windows Terminal Color Fallback Coverage

## Summary

Add focused Windows CI coverage for DUUMBI terminal color fallback behavior.
The work should prove that DUUMBI's user-visible terminal output remains
readable when Windows terminal color capability is limited, truecolor is not
advertised, output is not interactive, or `NO_COLOR` is set.

This spec treats the issue title's `anstream` wording as historical shorthand
from the Phase 16 roadmap. The current direct product surface is DUUMBI's
terminal styling behavior: non-TUI CLI output through `src/cli/theme.rs` and
full-screen TUI styles through the `ratatui` theme palette helpers. If Stage 8
finds a direct `anstream` assertion still useful through a transitive
dependency, it may include that as supporting evidence, but #422 should not
require a dependency migration or an `anstream`-specific public contract.

This is a specification artifact only. The linked execution issue must remain
open for Stage 7 review, Stage 8 technical specification, implementation,
review, and later closure evidence.

## Problem

Phase 16 is about making native Windows support credible. #420 has already
established the main `windows-latest` CI baseline, and #423 still owns public
README Windows requirements. The remaining risk for #422 is narrower: DUUMBI's
CLI and TUI can pass normal Rust checks while still emitting unreadable or
unsupported terminal styling on Windows or in constrained terminal contexts.

DUUMBI already centralizes terminal styling in `src/cli/theme.rs`, but current
coverage is mostly smoke-level:

- non-TUI color helpers return non-empty strings
- TUI style helpers compile and return styles
- the TUI fallback mapping from RGB palette values to named ANSI colors is
  deterministic
- one description path has a plain-text string assertion with no ANSI escapes

That is not enough to prove the Phase 16 behavior. The project needs reviewable
Windows CI evidence that color fallback remains deterministic and that explicit
no-color modes do not leak ANSI escape sequences into relevant non-TUI output.

## Outcome

When this is done:

- Windows CI runs targeted terminal color fallback coverage for Rust-relevant
  changes.
- `NO_COLOR=1` behavior is covered for representative non-TUI DUUMBI CLI
  output, and relevant output does not contain ANSI escape sequences.
- Non-interactive output behavior is covered where DUUMBI can assert it
  deterministically in CI.
- TUI palette fallback behavior is covered by deterministic tests or equivalent
  review evidence, including the non-truecolor path.
- The product contract is expressed in terms of observable DUUMBI terminal
  output and style fallback behavior, not a hard requirement to use `anstream`
  directly.
- Any remaining live Windows Terminal limitation that cannot be proven in
  headless GitHub Actions is called out as implementation or review evidence,
  not hidden inside passing unit tests.
- #420 remains the Windows CI baseline source, #423 remains the README Windows
  requirements source, and #422 remains scoped to color fallback evidence.

## Scope

### In Scope

- Add or adjust tests that verify terminal color fallback behavior relevant to
  Windows support.
- Verify `NO_COLOR=1` behavior for representative non-TUI CLI output without
  ANSI escape leakage.
- Verify non-truecolor TUI palette fallback behavior through deterministic test
  coverage or clearly equivalent CI evidence.
- Ensure the targeted checks run in the existing Windows CI path introduced by
  #420.
- Document, in implementation or review evidence, any behavior that cannot be
  reliably proven inside a headless `windows-latest` GitHub Actions runner.
- Keep the issue tied to the Phase 16 Windows support milestone.

### Explicitly Out Of Scope

- Adding the Windows CI baseline. That was #420 and is already completed.
- README or public Windows support requirement updates. That remains #423.
- Declaring Windows generally supported.
- Reworking the terminal color palette or brand styling beyond what is needed
  to make fallback behavior testable and deterministic.
- Migrating DUUMBI's direct styling implementation to `anstream`.
- Adding Cucumber, a Gherkin runner, or a new BDD execution framework.
- WSL2-specific testing, ARM64 Windows, MinGW/Cygwin, packaging, installers,
  release signing, or broad platform support work.
- Phase 13 self-healing, Phase 14 launch work, Studio UI theming, provider UX,
  registry behavior, or general CLI redesign.
- Technical specifications, implementation code, or Ralph-cycle instructions in
  this Stage 6 artifact.

## Constraints And Assumptions

Facts:

- Issue #422 is open, accepted by Stage 5 on 2026-05-19, labeled `accepted` and
  `needs-spec`, and in GitHub Project status `Spec Needed`.
- The Stage 5 decision explicitly accepted #422 as a well-scoped Phase 16
  reliability track and routed it to `Spec Needed`.
- #420 is completed, and the current `.github/workflows/ci.yml`
  includes `ubuntu-latest` and `windows-latest` in the main check matrix.
- #423 is open for README Windows requirements.
- `Cargo.toml` directly depends on `owo-colors` for non-TUI color helpers.
- `Cargo.toml` directly depends on `ratatui` and `crossterm` for TUI behavior.
- `Cargo.lock` contains `anstream`, but the direct DUUMBI styling surface found
  during Stage 6 source inspection is `owo-colors` plus `ratatui` theme helpers.
- `src/cli/theme.rs` says both non-TUI helpers and TUI styles should honor
  `NO_COLOR` and terminal color capability degradation.
- `src/cli/theme.rs` currently detects truecolor through `NO_COLOR`,
  `COLORTERM`, and `TERM_PROGRAM`, and maps DUUMBI RGB palette colors to named
  ANSI fallback colors when truecolor is not supported.
- The archived Phase 11 manual protocol already listed `NO_COLOR=1 duumbi
  build`, `NO_COLOR=1 duumbi check`, and piped output checks, but those were
  manual protocol items rather than Phase 16 Windows CI proof.

Assumptions:

- GitHub-hosted `windows-latest` is the authoritative CI environment for #422,
  because local macOS or Linux runs cannot prove native Windows runner behavior.
- Headless GitHub Actions cannot fully simulate every real Windows Terminal
  capability state, so the product contract should separate deterministic CI
  checks from any live manual terminal evidence that remains useful.
- The correct product target is readability and deterministic degradation for
  DUUMBI users and contributors, not byte-for-byte escape sequence snapshots.
- Tests should prefer semantic assertions such as absence of ANSI escapes,
  fallback color classes, and visible output readability over brittle full
  terminal transcript comparisons.
- The existing docs/spec-only CI skip behavior should remain intact for
  spec-only PRs.

Constraints:

- The implementation must not weaken the #420 Windows CI baseline.
- The implementation must not use #422 to update README Windows support claims.
- The implementation must not require a new product decision about Windows
  support level; #423 owns that user-facing documentation boundary.
- The implementation should keep tests deterministic across repeated CI runs.
- The implementation should avoid hidden global environment coupling in tests,
  especially around `NO_COLOR`, `COLORTERM`, `TERM_PROGRAM`, and cached palette
  decisions.
- The spec PR must use non-closing references such as `Spec for #422` or
  `Related to #422`; the execution issue stays open for later workflow stages.

## Decisions

- **Decision:** Use a file-based product spec for #422.
  **Evidence:** The work affects CI, platform reliability, source tests, and
  Phase 16 sequencing. It is durable implementation context, not a small
  issue-comment-only clarification.

- **Decision:** #422 can proceed now with #420 as completed baseline context.
  **Evidence:** #420 is completed, and the local workflow already has
  `windows-latest` in the main check matrix.

- **Decision:** Do not make direct `anstream` usage the product contract.
  **Evidence:** Current source inspection shows direct DUUMBI styling through
  `owo-colors` non-TUI helpers and `ratatui` TUI styles. The roadmap's
  `anstream` wording remains source context, but observable DUUMBI output is
  the product behavior that matters.

- **Decision:** Require deterministic CI checks first, with manual Windows
  Terminal evidence only when CI cannot prove an interactive behavior.
  **Evidence:** The Phase 16 kill criterion depends on GitHub-hosted Windows CI,
  while real terminal capability negotiation is partly environmental and can be
  hard to reproduce inside headless CI.

- **Decision:** Keep #423 separate.
  **Evidence:** README Windows requirements depend on CI and behavior evidence,
  but public support requirements are explicitly tracked in #423.

- **Decision:** This spec PR must keep #422 open.
  **Evidence:** Stage 6 creates a reviewable product spec only. Stage 7 product
  review, Stage 8 technical specification, Stage 9 technical review,
  implementation, Stage 11 review, and Stage 12 closure remain separate gates.

## Behavior

### Defaults

- Normal Rust-relevant PRs continue to run the existing CI check matrix on
  Ubuntu and Windows.
- The targeted #422 color fallback tests run on `windows-latest` when Rust
  checks are required.
- Default colorized output remains allowed where terminal capability supports
  it and no no-color policy is active.
- When `NO_COLOR=1` is active, relevant non-TUI CLI output should be readable
  plain text and should not contain ANSI escape sequences.
- When truecolor is not supported or not advertised for the TUI path, DUUMBI
  styles should use deterministic named-color fallbacks rather than RGB colors.
- Output readability must not depend on color alone.

### Inputs

- GitHub Actions runner OS, especially `windows-latest`.
- Environment variables that influence color behavior, including `NO_COLOR`,
  `CLICOLOR`, `COLORTERM`, `TERM_PROGRAM`, and `CARGO_TERM_COLOR` when relevant.
- Representative non-TUI DUUMBI commands that produce styled status,
  validation, build, check, diagnostic, or error output.
- Full-screen TUI style helpers used by the REPL.
- Non-interactive output conditions such as piping or captured process output,
  where deterministic in CI.

### Outputs

- CI logs showing targeted terminal color fallback checks ran on Windows.
- Test output proving `NO_COLOR=1` does not leak ANSI escapes for the selected
  non-TUI command path or paths.
- Test output proving non-truecolor TUI palette fallback is deterministic.
- Review evidence explaining any terminal behavior that remains manual or live
  environment dependent.
- No README support claim changes under #422.

### Visible States

- Normal color-capable terminal path: DUUMBI may emit styled output, and the
  content remains readable.
- `NO_COLOR=1` path: DUUMBI emits readable text without ANSI escape leakage for
  the tested non-TUI output.
- Non-truecolor TUI path: style helpers choose named-color fallbacks.
- Captured or piped non-TUI output path: DUUMBI avoids unreadable raw escape
  leakage where the behavior is deterministic and under DUUMBI's control.
- Windows CI path: the same targeted assertions are visible in the
  `windows-latest` check logs.

### Error States

- ANSI escape sequences appear in a selected `NO_COLOR=1` non-TUI output path.
- TUI fallback tests still return RGB colors when the simulated environment
  does not support truecolor.
- The Windows CI path does not run the targeted color fallback checks.
- A test depends on environment state that leaks across tests and makes the
  result order-dependent.
- The implementation changes README Windows requirements or broad platform
  support claims under #422.

### Cancellation And Retry

- Cancelled GitHub Actions runs do not count as passing Windows color fallback
  evidence.
- Retried CI runs may be used as evidence if the final linked run is clear and
  any retry reason is recorded when relevant.

### Race Conditions And Invariants

- Tests that modify process environment must isolate and restore environment
  state or otherwise avoid cross-test contamination.
- Cached palette or capability decisions must not make tests pass or fail based
  on execution order.
- Windows and Ubuntu should agree on the product contract even if their
  terminal capability defaults differ.
- The #420 `windows-latest` baseline must remain present and non-advisory.
- #422 must not introduce implementation dependence on README updates from
  #423.

### Accessibility And Focus

- Color must not be the only way DUUMBI communicates success, error, warning,
  or status in the tested terminal output.
- Plain-text output under `NO_COLOR=1` must still include the meaningful status
  or diagnostic text.
- No full-screen TUI keyboard-focus behavior changes are part of #422 unless
  Stage 8 identifies a narrow requirement directly tied to color fallback
  evidence.

## BDD Scenarios

Feature: Windows terminal color fallback coverage for DUUMBI

  Rule: Windows CI proves the targeted color fallback contract

    Scenario: Rust-relevant pull request runs terminal color fallback checks on Windows
      Given a pull request changes Rust-relevant DUUMBI files
      When the main CI workflow runs
      Then the workflow runs on `windows-latest`
      And the Windows check executes the targeted terminal color fallback tests
      And the Windows check fails if those tests fail

    Scenario: Spec-only pull request does not spend CI on Rust color checks
      Given a pull request changes only files under `specs/`
      When the main CI workflow runs
      Then the workflow reports that Rust checks are not required
      And no Rust terminal color fallback tests run for that spec-only PR

  Rule: Non-TUI no-color output remains readable and plain

    Scenario: NO_COLOR suppresses ANSI escapes for representative CLI output
      Given a representative non-TUI DUUMBI command emits styled status or diagnostic output
      And the environment contains `NO_COLOR=1`
      When the command output is captured in CI
      Then the captured output contains the meaningful status or diagnostic text
      And the captured output does not contain ANSI escape sequences

    Scenario: Non-interactive output avoids raw escape leakage where deterministic
      Given a representative non-TUI DUUMBI command emits styled output
      And the command output is captured or piped in a non-interactive context
      When the command completes in CI
      Then the captured output remains readable as plain text
      And raw ANSI escape leakage is absent for the tested path

  Rule: TUI palette fallback is deterministic

    Scenario: NO_COLOR disables truecolor for TUI style helpers
      Given DUUMBI's TUI theme is evaluated in an environment with `NO_COLOR=1`
      When the TUI style helpers choose colors
      Then truecolor support is treated as unavailable
      And DUUMBI palette colors map to named fallback colors

    Scenario: Missing truecolor signals use fallback colors
      Given `COLORTERM` does not advertise `truecolor` or `24bit`
      And no accepted terminal program signal enables truecolor
      When DUUMBI evaluates TUI palette colors
      Then the visible TUI styles use deterministic named-color fallbacks
      And the styles remain readable against the expected terminal background

  Rule: The `anstream` wording is interpreted as product outcome, not API lock-in

    Scenario: Implementation proves DUUMBI behavior without direct anstream assertions
      Given the source still uses `owo-colors` helpers and `ratatui` styles as the direct styling surface
      When #422 implementation adds terminal fallback coverage
      Then the checks assert observable DUUMBI output and fallback behavior
      And the implementation is not required to migrate DUUMBI styling to `anstream`

    Scenario: Implementation finds a direct anstream-specific mismatch
      Given Stage 8 or Stage 10 discovers a concrete `anstream` behavior mismatch that affects DUUMBI Windows output
      When the mismatch is within the accepted color fallback scope
      Then the implementation records the evidence and keeps the change scoped to terminal fallback behavior
      But broad dependency redesign or unrelated CLI styling work remains out of scope

  Rule: Phase 16 boundaries remain separate

    Scenario: README Windows requirements remain out of scope
      Given terminal color fallback checks pass on Windows CI
      When #422 implementation is reviewed
      Then review evidence links the Windows color fallback check result
      But README Windows requirement changes remain owned by #423

    Scenario: Windows CI baseline remains owned by #420
      Given #420 has established the Windows CI check matrix
      When #422 implementation adds color fallback coverage
      Then the existing Windows CI baseline remains present
      And #422 does not replace the general Rust build and test checks from #420

## Tasks

- Confirm the current Windows CI baseline from #420 remains present before
  implementation starts.
- Identify the representative non-TUI CLI output paths that best prove
  `NO_COLOR=1` and non-interactive readability without broad test setup.
- Identify or adapt the TUI palette capability seam needed for deterministic
  non-truecolor fallback tests.
- Add the targeted tests and ensure they run under the existing
  `windows-latest` CI path.
- Capture review evidence from Windows CI showing the targeted checks ran.
- Record any remaining live Windows Terminal evidence needs as review context
  or a follow-up, without broadening #422.

The non-TUI CLI output checks and the TUI palette fallback checks can be
implemented independently after Stage 8 defines the exact test shape. The CI
evidence collection depends on the implementation PR running in GitHub Actions.

## Checks

- Product spec review confirms all Stage 5 questions are answered:
  #422 can proceed after #420, `anstream` is historical shorthand rather than
  the direct product contract, and CI evidence is primary with manual evidence
  only where CI cannot prove interaction.
- Stage 8 technical spec maps every BDD scenario to unit, integration, CI,
  manual, or review evidence.
- Automated checks include a deterministic assertion that selected
  `NO_COLOR=1` non-TUI output has no ANSI escape sequences.
- Automated checks include a deterministic assertion for TUI non-truecolor
  fallback behavior.
- GitHub Actions evidence shows the targeted color fallback checks ran on
  `windows-latest`.
- Review evidence confirms no README Windows requirements changed under #422.
- Review evidence confirms the #420 Windows CI baseline remained intact.
- Existing project checks still pass or any failure is classified with evidence.

## Open Questions

- None blocking for Stage 7 product spec review.

Non-blocking questions for Stage 8:

- Which representative non-TUI command path gives the best coverage with the
  least workspace setup friction: `duumbi build`, `duumbi check`, a validation
  error path, or a narrower helper-level integration test?
- Does the existing TUI palette cache need a small testability adjustment to
  avoid environment-order coupling?
- Is a thin manual Windows Terminal smoke check still useful after deterministic
  CI coverage exists, or is Windows CI evidence sufficient for #422?

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/422
- Stage 5 human acceptance decision:
  https://github.com/hgahub/duumbi/issues/422#issuecomment-4491752231
- Related Windows CI baseline issue: https://github.com/hgahub/duumbi/issues/420
- Related README Windows requirements issue:
  https://github.com/hgahub/duumbi/issues/423
- Product spec baseline: `specs/DUUMBI-420/PRODUCT.md`
- Technical spec baseline: `specs/DUUMBI-420/TECHNICAL.md`
- Current CI workflow: `.github/workflows/ci.yml`
- Terminal theme source: `src/cli/theme.rs`
- Non-TUI describe plain-output test reference: `src/cli/describe.rs`
- CLI UX manual protocol: `docs/testing/phase11-cli-ux.md`
- Dependency manifest: `Cargo.toml`
- Active PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Agentic Development Runbook:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Phase 16 roadmap note:
  `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 16 - Windows & Cross-Platform Support.md`
