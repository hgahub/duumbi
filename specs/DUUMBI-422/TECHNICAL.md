# DUUMBI-422: Windows Terminal Color Fallback Coverage - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-422/PRODUCT.md` by adding
focused, deterministic coverage for DUUMBI terminal color fallback behavior on
the existing Windows CI path.

This technical spec implements these approved product outcomes:

- Rust-relevant CI runs terminal color fallback coverage on `windows-latest`.
- `NO_COLOR=1` suppresses ANSI escape sequences for representative non-TUI CLI
  output while preserving meaningful status or diagnostic text.
- Captured or piped representative non-TUI output avoids raw ANSI escape leakage
  where DUUMBI can assert that behavior deterministically.
- TUI palette fallback behavior is testable without order-dependent global
  environment state.
- Missing truecolor signals and `NO_COLOR=1` lead to deterministic named-color
  TUI fallbacks.
- The implementation verifies observable DUUMBI behavior and does not require a
  direct migration to `anstream`.
- #420 remains the Windows CI baseline source, #423 remains the README Windows
  requirements source, and #422 remains scoped to terminal color fallback
  evidence.

This technical spec does not authorize implementation during Stage 8. Stage 10
implementation agents must follow the Ralph Cycle resource policy below before
changing code, tests, CI, or documentation.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles in the DUUMBI source
  repo.
- Oz agents only if a human routes GitHub Actions evidence gathering or cloud
  follow-up work outside the local Codex session.
- Stage 9 technical spec reviewers validating source facts, implementation
  boundaries, BDD-to-test mapping, live E2E plan, and resource policy.
- Stage 10 testers and reviewers checking implementation evidence against the
  approved product and technical specs.

## Source Context

- Product spec: `specs/DUUMBI-422/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/422
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/422#issuecomment-4491752231
- Product spec PR: https://github.com/hgahub/duumbi/pull/575
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/422#issuecomment-4498513941
- Related Windows CI baseline issue: https://github.com/hgahub/duumbi/issues/420
- Related README Windows requirements issue:
  https://github.com/hgahub/duumbi/issues/423
- Repo instructions: `AGENTS.md`

Relevant source files verified for Stage 8:

- `.github/workflows/ci.yml`
- `Cargo.toml`
- `Cargo.lock`
- `src/cli/theme.rs`
- `src/cli/commands.rs`
- `src/cli/describe.rs`
- `src/cli/mod.rs`
- `src/main.rs`
- `tests/integration_phase1.rs`
- `docs/testing/phase11-cli-ux.md`
- `specs/DUUMBI-420/TECHNICAL.md`

Relevant Obsidian notes verified for Stage 8:

- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 16 - Windows & Cross-Platform Support.md`

Verified source facts:

- Issue #422 is open, labeled `accepted`, `product-spec-approved`, and
  `needs-tech-spec`, and was in Project status `Technical Spec Needed` when
  this Stage 8 draft started.
- The Stage 7 decision approved `specs/DUUMBI-422/PRODUCT.md` and reported no
  blocking findings.
- `.github/workflows/ci.yml` already runs the main `check` job on both
  `ubuntu-latest` and `windows-latest`.
- `.github/workflows/ci.yml` skips Rust setup and cargo commands for pull
  requests that change only docs/spec files.
- `Cargo.toml` directly depends on `owo-colors`, `ratatui`, and `crossterm`.
- `Cargo.lock` contains `anstream`, but the direct DUUMBI color styling surface
  inspected for Stage 8 is `owo-colors` for non-TUI strings and `ratatui` style
  helpers for the TUI.
- `src/cli/theme.rs` centralizes non-TUI semantic color helpers such as
  `error`, `success`, `dim`, `bold`, `check_mark`, and `cross_mark`.
- `src/cli/theme.rs` currently formats non-TUI helpers with direct
  `owo-colors::OwoColorize` calls such as `text.green().bold()`.
- `src/cli/theme.rs` has a TUI `OnceLock<Palette>` that caches whether
  truecolor is available.
- `src/cli/theme.rs` detects TUI truecolor using `NO_COLOR`, `COLORTERM`, and
  `TERM_PROGRAM`.
- `src/cli/theme.rs` already has a deterministic `fallback(Color) -> Color`
  mapping from DUUMBI RGB palette values to named ANSI colors.
- Existing `src/cli/theme.rs` tests cover helper non-emptiness, check/cross
  Unicode markers, public TUI style helper smoke coverage, fallback mapping,
  and a non-mocked `col` smoke path.
- `src/cli/commands.rs` prints `Build successful` and `Validation passed` to
  stderr using `theme::check_mark()`.
- `src/cli/commands.rs` prints parse, graph, validation, compilation, and link
  diagnostics with styled error codes, node IDs, and suggestions.
- `tests/integration_phase1.rs` already runs CLI commands through
  `std::process::Command` and captures process output.
- `docs/testing/phase11-cli-ux.md` contains historical manual checks for
  `NO_COLOR=1 duumbi build`, `NO_COLOR=1 duumbi check`, and piped output.
- The Phase 16 roadmap note lists #422 as the terminal color fallback track and
  keeps #420 and #423 separate.
- The active Agentic Development Runbook requires Stage 8 technical specs to
  include BDD-to-test mapping, a live E2E plan, and a Ralph Cycle resource
  policy.

Assumptions and implementation recommendations:

- The lowest-friction representative non-TUI command path is `duumbi check
  tests/fixtures/fibonacci.jsonld`, because it validates a known-good fixture,
  avoids C compiler/linker variability, and emits styled status text to stderr.
- `duumbi build` can be used as secondary evidence when an implementation agent
  wants build-path coverage, but it should not be the first required path
  because it adds native linker and C compiler variability to a color fallback
  issue.
- The TUI palette cache should not be tested by mutating process environment
  after `PALETTE` is initialized. Prefer a small pure decision helper or
  injectable palette decision that tests can exercise without resetting global
  state.
- The current `anstream` wording should remain roadmap context only. Stage 10
  should add `anstream`-specific assertions only if it finds a concrete
  DUUMBI-visible Windows output mismatch inside the accepted scope.
- GitHub-hosted `windows-latest` is the required live CI evidence for native
  Windows behavior. Local macOS/Linux runs are useful preflight checks but do
  not prove the Phase 16 Windows path.

## Affected Areas

Expected implementation changes:

- `src/cli/theme.rs`
  - Add deterministic no-color and non-interactive behavior for the non-TUI
    semantic color helpers or add a narrowly scoped helper layer they call.
  - Add a testable TUI palette decision seam that avoids order-dependent
    environment mutation around the existing `OnceLock`.
  - Add focused tests for `NO_COLOR`, missing truecolor signals, and named-color
    fallback behavior.
- `src/cli/commands.rs`
  - May need only indirect coverage through existing command output. Change this
    file only if the non-TUI color helper API needs a stream-aware call site or
    output path adjustment.
- `tests/integration_phase16_color.rs` or another focused integration test file
  under `tests/`
  - Add process-level checks for representative `duumbi check` output under
    `NO_COLOR=1`.
  - Add process-level checks for captured non-interactive `duumbi check` output
    where deterministic.
- `.github/workflows/ci.yml`
  - Usually no change is required if the targeted tests run under the existing
    `cargo test --all` Windows matrix from #420.
  - A small workflow change is acceptable only if needed to make targeted
    Windows color fallback evidence visible and non-advisory.

Expected review evidence and no-code areas:

- GitHub Actions logs from the implementation PR showing the targeted tests ran
  on `windows-latest`.
- PR evidence naming the exact test commands and targeted test names.
- PR evidence confirming no README Windows requirements changed under #422.
- PR evidence confirming the #420 Windows CI baseline remains present.
- Optional manual Windows Terminal smoke evidence if implementation discovers an
  interactive behavior that headless CI cannot prove.

Areas expected not to change:

- Product spec files, including `specs/DUUMBI-422/PRODUCT.md`.
- Technical specs for unrelated issues.
- README Windows requirements or public Windows support claims. #423 owns those.
- Broad CLI/TUI visual redesign, color palette rebranding, or Studio theming.
- Registry, provider, agent, graph, parser, compiler, runtime, MCP, or intent
  behavior unless a test harness requires only a narrow import or fixture use.
- Generated artifacts, screenshots, coverage output, `target/`, vendored
  artifacts, runtime assets, or release artifacts.
- The #420 Windows CI baseline semantics.

## Technical Approach

### 1. Preserve The Existing CI Baseline

Treat #420 as completed baseline infrastructure. Keep the main workflow's
Ubuntu and Windows matrix intact. Do not make the Windows job advisory.

The preferred implementation is to add tests that run under existing
`cargo test --all`, because that already runs on `windows-latest` for
Rust-relevant pull requests.

Only edit `.github/workflows/ci.yml` when one of these is true:

- the targeted tests cannot be made visible enough through normal test names
  and logs
- the Windows job no longer runs `cargo test --all`
- a Stage 10 agent finds a direct CI scoping issue that prevents #422 evidence
  from running on Rust-relevant changes

Do not weaken docs/spec-only skipping. This Stage 8 PR itself is expected to be
spec-only and should not require Rust checks.

### 2. Add A Focused Non-TUI Color Policy

The non-TUI helpers in `src/cli/theme.rs` are the right central boundary for
status, error, warning, info, dim, bold, check mark, cross mark, error-code, and
node-ID styling.

Recommended implementation shape:

- Add a small internal decision function that answers whether color should be
  emitted for a given non-TUI stream.
- The decision must disable color when `NO_COLOR` is present.
- The decision should disable color when `CLICOLOR=0`.
- The decision should disable color for captured or piped output unless an
  explicit local color-force policy already exists and is intentionally
  documented.
- The decision may account for `CLICOLOR_FORCE` if the implementation chooses to
  support forced local color, but tests for #422 must still prove `NO_COLOR=1`
  wins.
- Keep visible text meaningful without color. For example, a success line still
  needs `Validation passed.` and a check marker or other readable status cue.

Low-risk option:

- Keep the public helper names stable and route them through an internal
  `style_if_color_enabled` helper.
- Default the existing stderr-oriented helpers to stderr policy, because
  `build`, `check`, diagnostics, and suggestions currently use stderr for
  human-readable command output.
- If stdout color policy is needed for `describe`, add stream-specific helpers
  rather than making every caller guess.

Rejected alternatives:

- Do not scatter environment checks at individual call sites.
- Do not add a broad terminal abstraction across the full CLI/TUI surface for
  this issue.
- Do not require a direct `anstream` migration just because the issue title
  contains historical `anstream` wording.
- Do not snapshot full escape sequences as the main assertion; prefer semantic
  output plus absence of ANSI escapes.

### 3. Add Process-Level CLI Tests

Add a focused integration test file such as `tests/integration_phase16_color.rs`
that follows the existing `std::process::Command` pattern from
`tests/integration_phase1.rs`.

Recommended tests:

- `no_color_check_output_has_no_ansi_escapes`
  - Run the local DUUMBI binary or `cargo run --quiet -- check
    tests/fixtures/fibonacci.jsonld`.
  - Set `NO_COLOR=1`.
  - Capture stderr.
  - Assert success.
  - Assert stderr contains `Validation passed`.
  - Assert stderr does not contain `\x1b[`.
- `captured_check_output_has_no_raw_ansi_escapes`
  - Run the same command without `NO_COLOR`.
  - Capture stderr through `Command::output`.
  - Assert success and meaningful text.
  - Assert stderr does not contain `\x1b[` when the output is non-interactive.

If the test invokes `cargo run`, keep it focused and avoid broad workspace setup.
If it invokes the compiled test binary through `CARGO_BIN_EXE_duumbi`, prefer
that once verified locally because it avoids nested cargo overhead.

Use an ANSI detection helper that checks for control sequence introducer
patterns such as `\x1b[`. Do not rely on stripping ANSI and comparing full
transcripts.

### 4. Make TUI Palette Decisions Testable

Current TUI style helpers cache `Palette { truecolor }` through a `OnceLock`.
That is acceptable for runtime startup behavior, but direct environment-mutation
tests around the cached global can become order-dependent.

Recommended implementation shape:

- Extract a pure helper, for example:
  - `detect_truecolor_from_env(no_color, colorterm, term_program) -> bool`
  - or `Palette::from_env_snapshot(&EnvSnapshot) -> Palette`
- Keep the existing public `truecolor_supported()` and `col(Color)` behavior.
- Add a pure helper for test-time mapping, for example
  `col_with_palette(color, Palette { truecolor: false })`, or make the existing
  `fallback` plus pure decision helper cover the required assertions.
- Test `NO_COLOR=1`, no truecolor signals, `COLORTERM=truecolor`,
  `COLORTERM=24bit`, and `TERM_PROGRAM` behavior without relying on global
  process environment order.

Required assertions:

- `NO_COLOR` makes truecolor unavailable.
- Missing `COLORTERM` and missing `TERM_PROGRAM` make truecolor unavailable.
- `COLORTERM=truecolor` and `COLORTERM=24bit` make truecolor available when
  `NO_COLOR` is absent.
- `TERM_PROGRAM` makes truecolor available only when `NO_COLOR` is absent.
- When truecolor is unavailable, DUUMBI palette colors map to named fallback
  colors using the existing fallback table.

### 5. Keep The `anstream` Boundary Observable

The product spec explicitly interprets `anstream` as historical shorthand. Stage
10 should not add a direct dependency migration or public API contract around
`anstream`.

Implementation agents may inspect transitive `anstream` behavior if a real
Windows output mismatch appears, but any resulting change must stay inside the
approved terminal fallback scope and must be backed by DUUMBI-visible evidence.

### 6. Preserve Phase 16 Boundaries

Do not use #422 to update public Windows support claims. If implementation
discovers wording that belongs in README or public docs, record it as review
context for #423.

Do not use #422 to rework the Windows CI matrix. If the #420 baseline is absent
or broken, stop and report the blocker instead of replacing #420 inside this
issue.

## Invariants

- The execution issue remains open after this technical spec PR.
- Stage 10 implementation must stay within the approved product spec and this
  technical spec.
- `windows-latest` remains part of the main Rust-check CI matrix.
- Windows CI failures for Rust-relevant changes remain blocking.
- Docs/spec-only pull requests continue to avoid Rust toolchain installation
  and cargo commands.
- `NO_COLOR=1` takes precedence over any color-capability or force-color signal
  covered by the implementation.
- Meaningful status or diagnostic text remains present when ANSI styling is
  disabled.
- Output readability must not depend on color alone.
- TUI palette tests must not depend on process-wide environment mutation after
  a global cache is initialized.
- #422 does not change README Windows requirement claims.
- #422 does not require a direct `anstream` migration.
- No generated artifacts, runtime assets, or product specs are changed by
  implementation.

## BDD-To-Test Mapping

| Product BDD scenario | Required evidence | Recommended implementation evidence |
|---|---|---|
| Rust-relevant pull request runs terminal color fallback checks on Windows | CI/review evidence | Implementation PR changes Rust-relevant files; GitHub Actions shows `check (windows-latest)` ran `cargo test --all`; logs include the targeted test names such as `no_color_check_output_has_no_ansi_escapes`, `captured_check_output_has_no_raw_ansi_escapes`, and TUI palette tests. |
| Spec-only pull request does not spend CI on Rust color checks | CI/review evidence | Stage 8 spec-only PR and any later spec-only PR show the documentation-only check path and no Rust toolchain setup. If GitHub Actions skips as expected, link that run in review evidence. No new implementation test is required because #420 owns skip behavior; #422 must not regress it. |
| `NO_COLOR` suppresses ANSI escapes for representative CLI output | Integration test | `tests/integration_phase16_color.rs` runs `duumbi check tests/fixtures/fibonacci.jsonld` with `NO_COLOR=1`, captures stderr, asserts success, asserts `Validation passed`, and asserts no `\x1b[` sequence. |
| Non-interactive output avoids raw escape leakage where deterministic | Integration test | `tests/integration_phase16_color.rs` runs the same representative command with `Command::output`, captures stderr without `NO_COLOR`, asserts success and meaningful text, and asserts no `\x1b[` sequence for the captured non-interactive path. If implementation finds an intentional force-color setting, document it and keep the test on an unforced path. |
| `NO_COLOR` disables truecolor for TUI style helpers | Unit test | `src/cli/theme.rs` tests a pure palette decision helper with `NO_COLOR` present and asserts truecolor is unavailable; asserts selected DUUMBI palette colors use named fallbacks when truecolor is unavailable. |
| Missing truecolor signals use fallback colors | Unit test | `src/cli/theme.rs` tests a pure palette decision helper with no `COLORTERM` and no `TERM_PROGRAM`, then asserts `RUST -> Red`, `BLUE_INK -> Cyan`, `PARCHMENT -> White`, `HAIRLINE -> Gray`, and representative style helpers remain readable through named colors. |
| Implementation proves DUUMBI behavior without direct `anstream` assertions | Review evidence plus tests above | PR diff and evidence show tests assert DUUMBI CLI output and TUI fallback behavior. No direct `anstream` assertion is required unless a concrete mismatch is discovered. |
| Implementation finds a direct `anstream`-specific mismatch | Conditional review evidence and focused test | If discovered, implementation records the observed DUUMBI-visible mismatch, adds the narrowest test that reproduces the issue through DUUMBI output, and avoids broad dependency redesign. If no mismatch is discovered, the PR states that no direct `anstream` mismatch was found. |
| README Windows requirements remain out of scope | Review evidence | PR changed-files list excludes README/public Windows support docs. If a note is useful for #423, leave it as review context rather than editing README. |
| Windows CI baseline remains owned by #420 | CI/review evidence | PR diff keeps `.github/workflows/ci.yml` matrix with `ubuntu-latest` and `windows-latest`; if workflow is unchanged, review evidence points to existing matrix. GitHub Actions shows Windows cargo tests are non-advisory. |

## Live E2E Plan

Canonical interface: CLI.

Reasoning:

- #422 affects terminal output behavior, not LLM behavior.
- Full-screen TUI behavior should be covered by deterministic style-helper unit
  tests because headless CI cannot faithfully represent every Windows Terminal
  interactive state.
- A thin manual Windows Terminal smoke check is optional review evidence only
  if implementation discovers an interactive behavior that the CI tests cannot
  prove.

Required credentials and environment variables:

- No provider credentials are required.
- No external LLM API keys are required.
- Test-specific environment:
  - `NO_COLOR=1` for no-color CLI checks.
  - Absence or controlled values of `COLORTERM` and `TERM_PROGRAM` for pure TUI
    palette decision tests.

Expected external LLM usage:

- DUUMBI live provider calls: 0
- External model/agent CLI calls: 0
- Estimated external LLM cost: USD 0

Local preflight commands:

```bash
cargo fmt --check
cargo test cli::theme::
cargo test --test integration_phase16_color
```

Full local check when implementation touches shared CLI behavior:

```bash
cargo test --all
```

Live CI evidence:

- Open the Stage 10 implementation PR with Rust-relevant changes.
- Let the main CI workflow run on GitHub Actions.
- Verify the `check (windows-latest)` matrix job runs `cargo test --all`.
- Verify the targeted color fallback tests appear in the Windows job logs or in
  the Rust test output.
- Verify the Windows job is required and fails when the targeted tests fail.

Optional manual Windows Terminal smoke path:

```powershell
$env:NO_COLOR = "1"
target\debug\duumbi.exe check tests\fixtures\fibonacci.jsonld
Remove-Item Env:NO_COLOR
```

Manual pass criteria:

- Output contains meaningful validation status text.
- Output is readable as plain text.
- No visible raw ANSI escape sequences appear.

Manual evidence is not required when deterministic Windows CI evidence fully
proves the accepted product behavior.

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
9. stop if requirements are met, a blocker appears, resource thresholds are exceeded, scope changes, or the autonomous batch cap is reached

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 3 source/test/workflow files, excluding this
  technical spec and PR metadata.
- Expected command budget: up to 4 focused local commands per cycle, plus one
  `cargo test --all` before implementation PR review when shared CLI behavior
  changes.
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies or irreversible
  operations, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 3 low-budget Ralph cycles.
- When to stop and ask for human guidance: if #420 Windows CI baseline is
  absent, if tests require broad CLI/TUI redesign, if README Windows
  requirements need editing, if a new dependency is required beyond a narrow
  feature or test utility, if Windows CI behavior contradicts local evidence, or
  if a direct `anstream` mismatch implies a dependency architecture decision.

## Task Breakdown

1. Confirm workflow baseline and branch state.
   - Verify `.github/workflows/ci.yml` still includes `windows-latest`.
   - Verify docs/spec-only skip behavior still exists.
   - Verify #423 remains separate for README Windows requirements.

2. Implement non-TUI color policy.
   - Update `src/cli/theme.rs` so semantic helper output can be plain under
     `NO_COLOR=1` and captured non-interactive output.
   - Keep public helper behavior stable for callers.
   - Add focused unit tests for no-color policy decisions where practical.

3. Implement TUI palette testability seam.
   - Extract a pure truecolor decision helper or palette snapshot helper.
   - Add unit tests for `NO_COLOR`, missing truecolor signals,
     `COLORTERM=truecolor`, `COLORTERM=24bit`, and `TERM_PROGRAM`.
   - Keep the runtime `OnceLock` cache behavior unless there is direct evidence
     it violates the accepted product behavior.

4. Add CLI integration tests.
   - Add focused process-level tests for `duumbi check
     tests/fixtures/fibonacci.jsonld`.
   - Assert meaningful output text.
   - Assert no raw ANSI escape leakage for `NO_COLOR=1`.
   - Assert no raw ANSI escape leakage for captured non-interactive output where
     deterministic.

5. Run focused local checks.
   - `cargo fmt --check`
   - `cargo test cli::theme::`
   - `cargo test --test integration_phase16_color`

6. Run broad local checks if shared CLI behavior changed.
   - `cargo test --all`
   - `cargo clippy --all-targets -- -D warnings` when implementation touches
     shared source or public helper contracts.

7. Collect GitHub Actions evidence.
   - Open the implementation PR.
   - Confirm `check (windows-latest)` runs and includes targeted tests.
   - Record CI run links and relevant log evidence in the PR.

8. Preserve boundaries in review evidence.
   - Confirm no README Windows requirements changed.
   - Confirm #420 baseline remains intact.
   - Confirm #422 did not require direct `anstream` API lock-in.

## Verification Plan

Required automated checks:

- `cargo fmt --check`
- `cargo test cli::theme::`
- `cargo test --test integration_phase16_color`
- `cargo test --all` before review if shared CLI behavior changes
- `cargo clippy --all-targets -- -D warnings` before review if source changes
  alter shared helper behavior or public APIs

Required CI evidence:

- GitHub Actions `check (windows-latest)` passes for the implementation PR.
- Windows job logs show `cargo test --all` ran.
- Windows job logs or test output identify the targeted color fallback tests.
- GitHub Actions `check (ubuntu-latest)` still passes.
- Dependency audit remains required on Ubuntu for Rust-relevant changes.

Required review evidence:

- PR changed-files list includes only approved #422 areas.
- PR evidence maps each product BDD scenario to test, CI, manual, or review
  evidence.
- PR evidence confirms no README Windows support requirement changes.
- PR evidence confirms #420 Windows CI baseline remains present.
- PR evidence states whether any direct `anstream` mismatch was found. If none
  was found, no `anstream`-specific test is required.

Optional manual evidence:

- Windows Terminal smoke output for `NO_COLOR=1 duumbi check` if CI cannot
  prove an interactive behavior discovered during implementation.

## Completion Criteria

Implementation is ready for Stage 11 review when all of these are true:

- Every product BDD scenario has mapped evidence in the implementation PR.
- Representative `NO_COLOR=1` non-TUI CLI output contains meaningful text and no
  ANSI escape leakage.
- Representative captured non-interactive CLI output contains meaningful text
  and no raw ANSI escape leakage where deterministic.
- TUI truecolor detection is covered through pure deterministic tests.
- TUI named-color fallback mapping remains covered.
- Targeted tests run on `windows-latest` through the existing main CI path.
- The Windows CI job is non-advisory.
- #420 Windows CI baseline remains present.
- README Windows requirements remain unchanged under #422.
- No implementation code, generated artifacts, runtime assets, or product specs
  are outside the approved scope.
- PR evidence includes commands run, CI links, resource usage, remaining risks,
  and any optional manual evidence.

## Failure And Escalation

When a focused test fails locally:

- Keep the cycle bounded to the failing behavior.
- Report the failing command, expected behavior, actual output, and affected
  files.
- Continue only if the cause is inside the approved #422 scope and below the
  resource thresholds.

When Windows CI contradicts local evidence:

- Treat GitHub-hosted `windows-latest` as the authoritative Phase 16 signal.
- Inspect logs and reproduce with the narrowest command possible.
- Stop for human guidance if the path requires broad CI architecture changes or
  a product decision about supported Windows terminal behavior.

When implementation discovers README or public docs requirements:

- Do not edit README under #422.
- Record the finding as review context for #423.

When implementation discovers a direct `anstream` mismatch:

- Verify the mismatch through DUUMBI-visible output first.
- Add only the narrowest test and code change needed inside #422 scope.
- Stop for human guidance before dependency redesign, migration, or broad
  terminal abstraction work.

When resource thresholds are exceeded:

- Stop the Ralph cycle.
- Report current evidence, planned next step, external LLM call count, estimated
  cost, and why approval is needed.

## Open Questions

None blocking.

Stage 10 implementation agents may still make these bounded choices without
returning to Stage 8 if they preserve the mapping above:

- Whether the process-level CLI tests use `CARGO_BIN_EXE_duumbi` or nested
  `cargo run --quiet --`.
- Whether the non-TUI color policy is stderr-only for current callers or exposes
  a small stream-specific helper for future stdout callers.
- Whether optional manual Windows Terminal smoke evidence adds value after
  deterministic Windows CI evidence is available.
