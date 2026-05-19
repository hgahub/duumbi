# DUUMBI-420: Add Windows CI Coverage To The Main Rust Checks - Technical Specification

## Implementation Objective

Implement the approved `specs/DUUMBI-420/PRODUCT.md` behavior by updating
DUUMBI's main GitHub Actions CI workflow so Rust-relevant changes run the core
Rust quality gates on both `ubuntu-latest` and `windows-latest`.

This technical spec implements these approved product outcomes:

- Native Windows CI coverage exists for DUUMBI Rust changes.
- Non-doc/spec pull requests run Rust checks on Ubuntu and Windows.
- The Windows CI path runs `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo build`, and
  `cargo test --all`.
- Dependency audit remains required for non-doc/spec PRs, without requiring a
  duplicate audit run on every OS.
- Docs/spec-only pull requests still avoid Rust toolchain installation and Rust
  checks.
- Pushes to `main` always run Rust checks.
- Windows failures fail CI and are visible in PR checks.
- Adjacent Phase 16 work remains separate: #422 owns Windows terminal color
  fallback testing, and #423 owns README Windows requirements.

This technical spec does not authorize implementation during Stage 8. Stage 10
implementation agents must follow the Ralph Cycle resource policy below before
changing `.github/workflows/ci.yml`.

## Agent Audience

- Codex implementation agents for local workflow inspection, YAML edits, PR
  preparation, and evidence collection.
- Oz agents only if a human explicitly routes CI investigation or GitHub-hosted
  workflow follow-up work outside the local Codex session.
- Stage 9 technical spec reviewers validating the implementation boundary,
  BDD-to-test mapping, live E2E plan, and resource policy.
- Stage 10 reviewers/testers verifying GitHub Actions evidence against the
  approved product and technical specs.

## Source Context

- Product spec: `specs/DUUMBI-420/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/420
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/420#issuecomment-4477465951
- Stage 6 product spec draft:
  https://github.com/hgahub/duumbi/issues/420#issuecomment-4477533187
- Product spec PR: https://github.com/hgahub/duumbi/pull/567
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/420#issuecomment-4477655074
- Repo instructions: `AGENTS.md`

Relevant source files verified for Stage 8:

- `.github/workflows/ci.yml`
- `.github/workflows/coverage.yml`
- `.github/workflows/release.yml`
- `.github/PULL_REQUEST_TEMPLATE.md`
- `.github/copilot-review-instructions.md`
- `README.md`
- `docs/coding-conventions.md`

Relevant GitHub/roadmap context verified for Stage 8:

- #420 is open, has product-spec approval, and is in Project status
  `Technical Spec Needed`.
- #421 is the completed path separator audit.
- #422 owns Windows terminal color fallback testing.
- #423 owns README Windows requirements documentation.
- Phase 16 kill criterion requires `cargo test --all` to pass on
  `windows-latest` with the MSVC toolchain.

Relevant source facts:

- The current `.github/workflows/ci.yml` has one `check` job running on
  `ubuntu-latest`.
- The current `check` job runs for pushes to `main` and pull requests.
- The current workflow uses a Bash `Detect check scope` step. For pull requests,
  it fetches the base branch, computes changed files, and sets
  `run_rust_checks=false` only when all changed files are under `docs/` or
  `specs/`.
- The current workflow installs stable Rust with `rustfmt` and `clippy`, uses
  `Swatinem/rust-cache@v2`, runs format, clippy, installs `cargo-audit`, runs
  `cargo audit`, builds, and runs `cargo test --all`.
- `coverage.yml` already skips docs/spec-only PRs through `paths-ignore`, but it
  is separate from the main CI workflow and remains Ubuntu-only.
- `release.yml` has platform matrix patterns, but it is a tag-only release
  workflow and is not the right place for #420 CI checks.
- The pull request template already asks authors to validate `cargo fmt`,
  clippy, `cargo test --all`, and `cargo audit`.
- README still lists Windows as not supported, which must remain unchanged for
  #420.

Relevant Obsidian notes:

- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
  says DUUMBI should be evidence-oriented and human-verifiable.
- `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
  requires CI, PR review, and human verification before merge.
- `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 16 - Windows & Cross-Platform Support.md`
  defines #420 as Track A CI infrastructure and separates #422, #421, and #423.
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
  defines Stage 8 as a technical-spec-only step and forbids implementation
  changes.

Assumptions and recommendations:

- GitHub-hosted `windows-latest` includes Git for Windows and Git Bash. If an
  implementation keeps the existing Bash scope-detection step inside a Windows
  matrix job, that is acceptable as long as the actual workflow run proves it.
- The lowest-risk implementation is to preserve the existing `check` workflow
  semantics and add an OS matrix, while making `cargo audit` Ubuntu-only inside
  that matrix.
- No local test can prove hosted Windows behavior. The required live evidence is
  a GitHub Actions run from the implementation PR.

## Affected Areas

Expected implementation change:

- `.github/workflows/ci.yml`
  - Add `windows-latest` to the main Rust-check path.
  - Preserve push and pull request triggers.
  - Preserve docs/spec-only skip behavior.
  - Keep `cargo audit` required for Rust-relevant PRs, but run it once on
    Ubuntu.

Expected review evidence and no-code areas:

- GitHub Actions run logs from the implementation PR.
- PR body/checklist evidence documenting Ubuntu, Windows, audit, and
  docs/spec-only behavior.
- Optional workflow syntax evidence from `actionlint` or equivalent if available
  in the implementation environment.

Areas expected not to change:

- Rust source files under `src/`.
- Integration/unit tests under `tests/` or Rust module test blocks.
- Product spec files, including `specs/DUUMBI-420/PRODUCT.md`.
- Runtime assets under `runtime/`.
- Generated artifacts, screenshots, coverage output, target directories, or
  release artifacts.
- README and public Windows support documentation, unless #423 is explicitly
  routed into the same later implementation scope by a human.
- Terminal color fallback checks, unless #422 is explicitly routed into the same
  later implementation scope by a human.
- `coverage.yml`, `release.yml`, `docs-review.yml`, and
  `copilot-review.yml`, unless implementation discovers a direct conflict with
  the main CI workflow.

## Technical Approach

### 1. Preserve The Main CI Workflow Boundary

Change `.github/workflows/ci.yml`; do not introduce a separate workflow for
Windows CI in #420. Keeping the main workflow as the source of truth makes the
required Rust quality signal visible in one place and matches the issue's
accepted scope.

Recommended implementation shape:

- Keep `name: CI`.
- Keep the existing `push` and `pull_request` triggers.
- Keep `env: CARGO_TERM_COLOR: always`.
- Keep a `check` job and add a strategy matrix over:
  - `ubuntu-latest`
  - `windows-latest`
- Use `runs-on: ${{ matrix.os }}`.
- Prefer `fail-fast: false` so Ubuntu and Windows evidence can both be observed
  when one OS fails early.

Do not use `continue-on-error` for Windows.

### 2. Preserve Docs/Spec-Only Detection

The existing docs/spec-only logic is part of the accepted product contract.
Implementation may keep it in the matrix job or factor it into a small
preflight/scope job, but final behavior must satisfy:

- Non-PR events set `run_rust_checks=true`.
- Pull requests with no changed-file list fall back to `run_rust_checks=true`.
- Pull requests with any changed file outside `docs/` and `specs/` set
  `run_rust_checks=true`.
- Pull requests changing only `docs/` and `specs/` set
  `run_rust_checks=false`.
- Docs/spec-only runs emit an explicit skip message.
- Docs/spec-only runs do not install Rust or run cargo commands.

If the implementation uses a matrix job, the skip decision must be identical for
Ubuntu and Windows. If the implementation uses a separate scope job, both OS
jobs and the audit path must consume the same output.

Recommended low-risk option:

- Keep the current `Detect check scope` shell step in the matrix job with
  `shell: bash`.
- Keep the `Documentation-only check` step guarded by
  `steps.scope.outputs.run_rust_checks == 'false'`.
- Guard all Rust setup/cache/cargo steps with
  `steps.scope.outputs.run_rust_checks == 'true'`.

Alternative acceptable option:

- Add a lightweight Ubuntu `scope` job with a job output.
- Make `check` matrix and `audit` jobs depend on `scope`.
- Add a docs/spec-only reporting job for the skip path.

Reject any implementation that removes docs/spec-only skipping or silently skips
Rust checks when changed-file detection is unavailable.

### 3. Add Windows Rust Checks

For both `ubuntu-latest` and `windows-latest`, when Rust checks are required:

- Check out the repo.
- Install stable Rust through `dtolnay/rust-toolchain@stable`.
- Include `rustfmt` and `clippy` components.
- Use `Swatinem/rust-cache@v2`.
- Run `cargo fmt --check`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo build`.
- Run `cargo test --all`.

Use default runner shell for cargo steps unless implementation has a concrete
reason to use Bash everywhere. The product contract is observable CI behavior,
not the shell implementation.

Do not add platform-specific test filtering in #420 unless a Windows failure is
proven unrelated to the accepted CI bring-up scope and a human approves a
follow-up or scope decision.

### 4. Keep Dependency Audit Required Once

Run `cargo audit` for Rust-relevant changes, but only once on Ubuntu.

Recommended implementation:

- Keep `Install cargo-audit` and `Dependency audit` inside the matrix job.
- Guard them with both:
  - `steps.scope.outputs.run_rust_checks == 'true'`
  - `matrix.os == 'ubuntu-latest'`

Alternative acceptable implementation:

- Split dependency audit into a dedicated Ubuntu job that depends on the same
  scope decision.

Do not run `cargo audit` on Windows unless Stage 10 discovers a concrete reason
and the extra CI time/noise is accepted in the implementation evidence.

### 5. Keep Adjacent Phase 16 Scope Separate

#420 implementation must not:

- Change README Windows support status.
- Add Windows installation documentation.
- Add terminal color fallback checks.
- Add `anstream`-specific assertions.
- Reopen the path separator audit.
- Add WSL2, ARM64 Windows, MinGW/Cygwin, packaging, or release-matrix work.

If Windows CI exposes a source or test failure, classify it before changing
scope:

- Narrow CI bring-up issue: fix in #420 only if it is directly required for
  baseline `cargo fmt`, clippy, build, or `cargo test --all` to pass and does
  not expand into a broader feature.
- Broader platform behavior: stop, record evidence, and ask whether to create or
  route a linked Phase 16 follow-up.

### 6. PR Evidence

The implementation PR must include:

- A concise summary of the workflow structure used.
- A statement that Windows is not allowed to fail.
- A statement that docs/spec-only PR behavior is preserved.
- Links to relevant GitHub Actions runs when available.
- Notes on any Windows-only failure encountered and whether it was fixed within
  scope or split out.

## Invariants

- The execution issue remains open until Stage 12 closure verifies merged
  implementation evidence.
- The technical spec PR and later implementation PR must use non-closing
  references such as `Technical spec for #420`, `Related to #420`, or
  `Supports #420`.
- Windows CI must be a required failing/pass signal for Rust-relevant changes;
  it must not use `continue-on-error`.
- Ubuntu Rust coverage must remain present.
- `cargo test --all` must run on `windows-latest` when Rust checks are required.
- `cargo audit` remains required at least once for non-doc/spec PRs.
- Docs/spec-only PRs must still avoid Rust setup and cargo commands.
- Pushes to `main` must run Rust checks regardless of changed file scope.
- README Windows support status remains conservative until #423.
- #422 and #423 remain separate unless a human explicitly changes scope.
- No implementation code, tests, generated artifacts, runtime assets, or product
  specs may be changed under this Stage 8 technical-spec PR.

## BDD-To-Test Mapping

| Product-spec scenario | Required evidence | Automation level |
|---|---|---|
| Pull request changes Rust source code | Implementation PR GitHub Actions run shows `check` on `ubuntu-latest` and `windows-latest`; Windows log shows `cargo fmt --check`, clippy, `cargo build`, and `cargo test --all`. | Live GitHub Actions E2E |
| Windows-only Rust failure blocks the pull request | Static workflow review confirms no `continue-on-error`; if a Windows failure occurs during implementation, PR evidence links the failing check. | Static review plus live CI evidence when failure occurs |
| Pull request changes only docs and specs | Evidence that docs/spec-only path still emits the skip message and does not install Rust or run cargo. Preferred evidence is a linked docs/spec-only PR/workflow run; acceptable fallback is static workflow review if producing a separate evidence PR is not reasonable. | Live CI evidence preferred; static review acceptable |
| Changed-file detection is unavailable | Static workflow review confirms empty or unavailable changed-file output falls back to `run_rust_checks=true`, matching current behavior. | Static review |
| Commit is pushed to main | Static workflow review confirms non-PR events set `run_rust_checks=true`; final post-merge CI run on `main` can serve as Stage 12 closure evidence. | Static review now; live post-merge evidence later |
| Dependency audit remains required without duplicating OS work | Static workflow review confirms `cargo audit` runs for Rust-relevant changes on Ubuntu only or in a dedicated Ubuntu audit job; implementation PR run shows audit passed. | Static review plus live GitHub Actions E2E |
| Adjacent Phase 16 work remains separate | Diff review confirms README, terminal color fallback tests, release packaging, and non-CI source files were not changed. | PR diff review |

## Live E2E Plan

Canonical interface: GitHub Actions for the `CI` workflow on the implementation
PR.

This issue does not touch LLM behavior, provider routing, Query mode, Agent
mode, Intent mode, Studio, or external model APIs. A live LLM-backed E2E path is
therefore not required for #420. The external LLM call count is `0`, required
provider credentials are `none`, and estimated external LLM cost is `USD 0`.

Required live CI evidence:

- Open the implementation PR with a Rust-relevant workflow change to trigger the
  CI workflow.
- Confirm the Ubuntu check passes.
- Confirm the Windows check passes.
- Confirm the Windows run includes:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo build`
  - `cargo test --all`
- Confirm dependency audit passes in the required Ubuntu path.
- Confirm no Windows check is marked allowed-to-fail.

Optional but useful evidence:

- A docs/spec-only PR or temporary evidence branch showing the skip message and
  no Rust toolchain install. Do not create unnecessary throwaway PRs if static
  workflow review is sufficient for Stage 11.
- A post-merge `main` workflow run showing the same behavior for Stage 12
  closure.

Pass criteria:

- All required live CI checks for the implementation PR are green.
- The PR body or review evidence links the relevant workflow run(s).
- Any retry is explained when it changes the evidence story.

Fail criteria:

- Windows job is missing, skipped for Rust-relevant changes, allowed to fail, or
  does not run `cargo test --all`.
- Ubuntu coverage is removed or weakened.
- `cargo audit` no longer runs for Rust-relevant PRs.
- Docs/spec-only PRs install Rust or run cargo checks unnecessarily.

## Ralph Cycle Protocol

Each cycle must:

1. Summarize current state and remaining unmet requirements.
2. Propose one bounded implementation goal.
3. List intended file areas and commands.
4. Estimate resource use and risk.
5. Check whether the resource gate requires human approval.
6. Implement only the approved or resource-permitted goal.
7. Run the agreed checks.
8. Report evidence, failures, and remaining gaps.
9. Stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 1 source file for implementation
  (`.github/workflows/ci.yml`) plus PR-body/evidence updates.
- Expected command budget:
  - Local/static cycle: `git diff --check`, targeted inspection of
    `.github/workflows/ci.yml`, and `actionlint .github/workflows/ci.yml` if
    `actionlint` is available.
  - Live CI cycle: `gh pr checks --watch` or equivalent GitHub Actions check
    inspection after pushing implementation.
  - No local `cargo test --all` is required for a workflow-only change unless
    implementation expands into Rust source or test files.
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies or irreversible
  operations, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 2 low-budget cycles. The first cycle should implement
  the workflow change and static checks; the second may respond to CI feedback
  if it remains within #420 scope.
- When to stop and ask for human guidance:
  - Windows CI failure requires source or test changes beyond narrow CI
    bring-up.
  - Dependency audit placement conflicts with branch protection or required
    check naming.
  - Docs/spec-only skip behavior cannot be preserved without substantial
    workflow restructuring.
  - Implementation would touch README, terminal color tests, release packaging,
    Rust source, or product specs.
  - GitHub-hosted Windows runners are unavailable or fail in a way that cannot
    be diagnosed from logs.

## Task Breakdown

1. Inspect current workflow and branch protection/check expectations if
   available.
2. Update `.github/workflows/ci.yml` to add Windows Rust checks while preserving
   current triggers, scope detection, and docs/spec-only skip behavior.
3. Make dependency audit run once for Rust-relevant changes, preferably on
   Ubuntu.
4. Run static local checks:
   - `git diff --check`
   - manual YAML review
   - `actionlint .github/workflows/ci.yml` if available
5. Push implementation PR and inspect GitHub Actions runs.
6. If CI fails, classify failure as:
   - workflow syntax/configuration bug within #420
   - narrow Windows CI bring-up fix within #420
   - broader Phase 16 follow-up requiring human guidance
7. Update implementation PR evidence with workflow run links, docs/spec-only
   skip evidence or static rationale, and any follow-up notes.

The workflow YAML edit and local static checks are independent from GitHub-hosted
runtime proof. The runtime proof depends on the implementation PR triggering
GitHub Actions.

## Verification Plan

Required local/static verification:

- `git diff --check`
- Static review of `.github/workflows/ci.yml` confirms:
  - `ubuntu-latest` and `windows-latest` Rust-check paths exist.
  - Push and pull request triggers remain.
  - Non-PR events force Rust checks on.
  - Docs/spec-only detection remains present.
  - Empty/unavailable changed-file output falls back to Rust checks.
  - Rust setup/cache/cargo steps are guarded by the scope decision.
  - Windows is not allowed to fail.
  - `cargo audit` still runs once for Rust-relevant changes.
- `actionlint .github/workflows/ci.yml` when available. If unavailable, record
  that the check could not be run locally and rely on GitHub Actions syntax
  validation from the PR run.

Required live GitHub Actions verification:

- CI workflow run on the implementation PR.
- Ubuntu Rust check passes.
- Windows Rust check passes.
- Windows logs show `cargo test --all`.
- Dependency audit passes in the required Ubuntu path.
- PR evidence links the run(s).

Manual/review verification:

- Review diff confirms no implementation code, tests, runtime assets, generated
  artifacts, README support claims, or product spec changes were included.
- Review confirms #422 and #423 scope remains untouched.
- Review confirms BDD scenarios are either proven by live CI, static workflow
  review, or explicitly deferred to Stage 12 post-merge evidence.

## Completion Criteria

The implementation is ready for PR review when:

- `.github/workflows/ci.yml` is the only required implementation file changed.
- Rust-relevant PRs run the CI Rust checks on Ubuntu and Windows.
- Windows CI runs `cargo fmt --check`, clippy, build, and `cargo test --all`.
- Windows CI failures fail the PR.
- Docs/spec-only PR skip behavior is preserved.
- Pushes to `main` still run Rust checks.
- `cargo audit` remains required once for Rust-relevant PRs.
- GitHub Actions evidence is linked in the PR or review notes.
- Any Windows-specific failure outside narrow CI bring-up is captured as a
  linked follow-up or brought back for human guidance.
- No README Windows support claims or terminal color checks are introduced under
  #420.

## Failure And Escalation

- If workflow syntax fails, fix the YAML in the same cycle if the correction
  stays within `.github/workflows/ci.yml`.
- If Windows `cargo fmt --check`, clippy, build, or `cargo test --all` fails
  because of workflow setup, fix the setup in #420.
- If Windows CI reveals Rust source, test, runtime, credential, file-system, or
  terminal behavior problems, stop after recording evidence and ask whether the
  fix belongs in #420 or a linked Phase 16 follow-up.
- If `cargo audit` install time or network behavior causes instability, keep the
  audit on Ubuntu only and record the evidence. Do not drop the audit signal
  without explicit human approval.
- If branch protection requires old check names that conflict with matrix job
  names, stop and ask for guidance before restructuring required checks.
- If implementation would require changes outside `.github/workflows/ci.yml`,
  stop unless the change is explicitly approved as narrow CI bring-up scope.

## Open Questions

None blocking for Stage 9 review.

Non-blocking implementation detail: the implementation may keep the existing
Bash-based scope detection inside the matrix job or factor scope detection into
a separate Ubuntu job. The required behavior is identical scope decisions for
Ubuntu, Windows, and audit paths, not a specific YAML layout.
