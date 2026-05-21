# DUUMBI-423: Document Native Windows Requirements In README - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-423/PRODUCT.md` by updating
DUUMBI's user-facing Windows setup documentation in the repository README and
the public installation page.

This technical spec implements these approved product outcomes:

- README no longer gives a stale blanket Windows unsupported signal.
- The public installation page no longer contradicts README Windows guidance.
- Windows users can identify the native Windows target before installation:
  Windows 10 version 1903+ on x86_64 with the MSVC Rust toolchain.
- Windows users can identify required build prerequisites: stable Rust MSVC,
  Visual Studio Build Tools or equivalent MSVC C++ tools, Windows SDK, and a
  usable linker/C compiler environment.
- The docs explain that WSL2 is not required for the native Windows path.
- The docs exclude unsupported or unproven targets such as ARM64 Windows,
  MinGW/Cygwin/GNU Windows toolchains, installers, release signing, and
  packaging.
- The docs stay consistent with #420 Windows CI evidence and #422 terminal
  color fallback scope without overstating Phase 16 completion.
- The implementation remains documentation-only.

This technical spec does not authorize implementation during Stage 8. Stage 10
implementation agents must follow the Ralph Cycle resource policy below before
changing documentation.

## Agent Audience

- Codex implementation agents performing local docs edits, static checks, PR
  preparation, and evidence collection.
- Oz agents only if a human routes GitHub Actions evidence gathering or review
  follow-up work outside the local Codex session.
- Stage 9 technical spec reviewers validating affected areas, BDD-to-test
  mapping, live E2E plan, and resource policy.
- Stage 10 reviewers checking the implementation PR against the approved
  product and technical specs.

## Source Context

- Product spec: `specs/DUUMBI-423/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/423
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/423#issuecomment-4498707005
- Product spec PR: https://github.com/hgahub/duumbi/pull/578
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/423#issuecomment-4505534207
- Related Windows CI baseline issue: https://github.com/hgahub/duumbi/issues/420
- Related terminal color fallback issue:
  https://github.com/hgahub/duumbi/issues/422
- Repo instructions: `AGENTS.md`

Relevant source files verified for Stage 8:

- `README.md`
- `sites/docs/src/getting-started/installation.md`
- `sites/docs/book.toml`
- `sites/docs/src/SUMMARY.md`
- `.github/workflows/ci.yml`
- `.github/workflows/coverage.yml`
- `.github/workflows/docs-review.yml`
- `specs/DUUMBI-420/TECHNICAL.md`
- `specs/DUUMBI-422/TECHNICAL.md`

Relevant Obsidian notes verified for Stage 8:

- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- `Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Public Docs as Product Interface.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 16 - Windows & Cross-Platform Support.md`
- `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Product Roadmap 2026-05.md`

Verified source facts:

- Issue #423 is open, labeled `accepted`, `product-spec-approved`, and
  `needs-tech-spec`, and was in Project status `Technical Spec Needed` when
  this Stage 8 draft started.
- The Stage 7 decision approved `specs/DUUMBI-423/PRODUCT.md` and reported no
  blocking findings.
- PR #578 merged `specs/DUUMBI-423/PRODUCT.md` into `main` on 2026-05-21.
- `README.md` currently has one compact requirements sentence that names Rust
  stable 1.80+ and a generic C compiler on `$PATH`.
- `README.md` currently lists Windows as `Not supported` in the supported
  platforms table.
- `sites/docs/src/getting-started/installation.md` currently names Rust and a
  generic C compiler as prerequisites, with macOS and Debian/Ubuntu examples.
- `sites/docs/src/getting-started/installation.md` currently says Windows is
  not supported in the current release.
- `sites/docs/book.toml` defines the public docs as an mdBook site under
  `sites/docs/src`.
- `sites/docs/src/SUMMARY.md` already includes the installation page; no summary
  edit is needed for this issue.
- `.github/workflows/ci.yml` runs a non-Rust-relevant skip path (controlled by
  the `rust_relevant_pattern` shell variable in `.github/workflows/ci.yml`) for
  pull requests whose changed files do not match Rust-relevant paths.
- README and `sites/docs/**` changes are not Rust-relevant under the current
  `.github/workflows/ci.yml` pattern, so the implementation PR should exercise
  the documentation-only CI path rather than Rust checks.
- `.github/workflows/coverage.yml` is path-limited to Rust/workflow inputs and
  should not run for README/docs-only changes.
- `.github/workflows/docs-review.yml` is path-limited to code changes and
  should not run for README/docs-only changes.
- `mdbook` was not available in the local Stage 8 environment inspected for this
  draft.
- #420 is completed and established the `windows-latest` CI baseline.
- #422 is in `Ready for Build` at Stage 8 inspection time. Its technical spec is
  approved, but implementation evidence is not yet completed.

Assumptions and recommendations:

- The lowest-risk implementation is a documentation-only change to `README.md`
  and `sites/docs/src/getting-started/installation.md`.
- README should use both a concise supported-platforms table row and a short
  Windows setup paragraph or subsection. A table row alone is not enough for the
  required prerequisites and limitation boundary.
- The public installation page should duplicate the essential Windows
  requirements instead of only linking back to README, because docs.duumbi.dev
  should remain understandable without repository context.
- The docs should avoid normal user-facing references to issue numbers and
  workflow stages. Put issue/PR references in implementation evidence, not in
  polished setup instructions.
- Because #422 implementation evidence is not complete yet, the final wording
  should not claim completed terminal color fallback coverage. If #422 reaches
  Stage 12 before #423 implementation starts, the implementation agent may update
  wording only after verifying that closure evidence.

## Affected Areas

Expected implementation changes:

- `README.md`
  - Replace or expand the generic requirements sentence so Windows prerequisites
    are explicit.
  - Replace the stale Windows unsupported table row with a native Windows MSVC
    target row.
  - Add a short Windows native setup paragraph or subsection that names Windows
    10 version 1903+, `x86_64-pc-windows-msvc`, stable Rust MSVC, Visual Studio
    Build Tools or equivalent MSVC C++ tools, Windows SDK/linker setup, WSL2
    not required, and unsupported Windows variants.
- `sites/docs/src/getting-started/installation.md`
  - Update prerequisites to include native Windows MSVC setup.
  - Update the supported-platforms section so it matches README.
  - Remove the stale Windows unsupported sentence.
  - Include enough Windows detail that the public docs page works standalone.

Expected review evidence and no-code areas:

- PR body evidence listing the exact changed docs files.
- Static text scans proving stale unsupported statements are gone from the
  affected onboarding surfaces.
- Static text scans proving the required Windows target, prerequisites, WSL2
  boundary, and unsupported targets are documented.
- Optional mdBook build evidence if `mdbook` is available in the implementation
  environment.
- GitHub Actions evidence showing the docs/spec-only CI path ran as expected.

Areas expected not to change:

- Rust source files under `src/`.
- Rust tests under `tests/` or module test blocks.
- CI workflow files.
- Coverage workflow files.
- Docs review workflow files.
- `sites/docs/src/SUMMARY.md`, unless Stage 10 discovers the installation page
  has moved or summary metadata is actually required.
- Product specs, including `specs/DUUMBI-423/PRODUCT.md`.
- Technical specs for unrelated issues.
- Runtime assets under `runtime/`.
- Generated output, rendered book output, screenshots, coverage output,
  `target/`, release artifacts, or vendored artifacts.

## Technical Approach

### 1. Keep The Change Documentation-Only

Do not edit Rust code, Rust tests, CI, runtime assets, generated docs output, or
product specs. This issue is about user-facing Windows requirements language.

The implementation PR should normally contain exactly these changed files:

- `README.md`
- `sites/docs/src/getting-started/installation.md`

If an implementation agent believes any other file must change, it must stop and
record why that file is required before continuing. A change outside those two
docs files needs human approval unless it is purely unavoidable docs metadata
within the existing source repo pattern.

### 2. Update README Requirements And Platform Status

Replace the current one-line requirements text with a short list or compact
paragraph that keeps existing macOS/Linux expectations and adds Windows-native
requirements.

Recommended content elements:

- Rust stable 1.80+ through `rustup`.
- macOS: Xcode Command Line Tools or equivalent C compiler/linker.
- Linux: `build-essential` or equivalent C compiler/linker.
- Windows native: Windows 10 version 1903+, stable MSVC Rust toolchain,
  `x86_64-pc-windows-msvc`, Visual Studio Build Tools or equivalent MSVC C++
  tools, Windows SDK, and usable linker environment.

Update the supported-platforms table so Windows is no longer represented by a
dash placeholder and `Not supported`.

Recommended row shape:

```markdown
| Windows | x86_64-pc-windows-msvc | Native target; MSVC tools required |
```

Then add short prose below the table or requirements section:

- WSL2 is not required for the native Windows path.
- ARM64 Windows is not covered by this support boundary.
- MinGW, Cygwin, and GNU Windows toolchains are not covered by this support
  boundary.
- Installers, packaged Windows releases, and release signing are not covered by
  this support boundary.
- Avoid claiming terminal color fallback completion unless #422 has verified
  implementation and closure evidence before Stage 10 starts.

Do not add GitHub issue links to the public README copy unless a human
explicitly asks for contributor-facing traceability there.

### 3. Update Public Installation Docs To Match README

Apply the same support boundary to
`sites/docs/src/getting-started/installation.md`.

Recommended structure:

- Keep the existing `## Prerequisites` section.
- Convert prerequisite bullets into OS-specific bullets if needed for clarity.
- Keep the existing `cargo install duumbi`, source build, and verification
  commands unless implementation evidence shows they are inaccurate.
- Update the supported-platforms table to include the same Windows row as
  README.
- Replace the current unsupported Windows sentence with the same native Windows
  requirements and unsupported-target boundary.

The public docs page should not merely say "see README" for Windows setup. It
can link to README as supporting context, but the required setup information
must be present in the page itself.

### 4. Preserve Existing Docs Site Structure

Do not edit `sites/docs/src/SUMMARY.md`; the installation page is already in the
Getting Started section.

Do not commit generated mdBook output. If `mdbook build sites/docs` is run, any
generated `book/` output must remain untracked and out of the implementation
diff.

### 5. Keep #420 And #422 Boundaries Explicit In Evidence

The implementation PR evidence should state:

- #420 remains the Windows CI baseline; no CI files changed under #423.
- #422 remains the terminal color fallback implementation/evidence source; no
  terminal behavior files or tests changed under #423.
- #423 only changes Windows requirements documentation.

This boundary belongs in PR/issue evidence. User-facing docs should focus on
setup requirements and limitations.

## Invariants

- The implementation must change only approved documentation surfaces unless
  human approval expands scope.
- README and the public installation page must agree on Windows minimum version,
  architecture/toolchain target, prerequisites, WSL2 boundary, and unsupported
  targets.
- Existing macOS and Linux guidance must remain clear.
- The docs must not imply ARM64 Windows, MinGW/Cygwin/GNU Windows toolchains,
  installers, packaging, or release signing support.
- The docs must not claim final terminal color fallback completion unless #422
  completion evidence is verified before implementation.
- The implementation must not change CI behavior, Rust behavior, generated
  artifacts, product specs, runtime assets, or tests.
- The execution issue must remain open for later workflow stages.

## BDD-To-Test Mapping

| Product-spec scenario | Evidence type | Required proof |
|---|---|---|
| Windows user finds the native support target in README | Static text review | `README.md` no longer has the stale unsupported Windows row; it names Windows 10 version 1903+ and `x86_64-pc-windows-msvc`. |
| Windows user finds required build prerequisites in README | Static text review | `README.md` names stable MSVC Rust, Visual Studio Build Tools or equivalent MSVC C++ tools, Windows SDK, and linker/C compiler setup. |
| Public installation page no longer contradicts README | Static text review | `sites/docs/src/getting-started/installation.md` no longer says Windows is unsupported and matches README on version, target, and support boundary. |
| Existing macOS and Linux guidance remains intact | Static text review | README and installation docs still describe macOS and Linux prerequisites/status clearly after the Windows edit. |
| Unsupported Windows variants are not implied | Static text review | Both docs surfaces state ARM64 Windows, MinGW/Cygwin/GNU toolchains, installers, packaging, and release signing are outside the current support boundary. |
| WSL2 is not required for the native Windows target | Static text review | Both docs surfaces state WSL2 is not required for the native Windows path and do not add WSL2-specific setup as a requirement. |
| Terminal color fallback work is not overstated | PR diff and issue-state review | User-facing docs do not claim completed terminal fallback evidence unless #422 completion evidence is verified; PR evidence records #422 boundary. |
| Documentation implementation does not change platform behavior | PR diff review | Changed files are limited to README/public installation docs unless separately approved; no Rust, tests, CI, generated assets, runtime assets, or specs are changed. |
| Stale Windows-negative text is removed from onboarding surfaces | Static text scan | A targeted scan over `README.md` and `sites/docs/src/getting-started/installation.md` finds no stale blanket unsupported Windows wording. |

Recommended static commands for Stage 10 evidence:

```sh
git diff --check
git diff --name-only
rg -n -e "\| Windows \|.*\| Not supported \|" -e "Windows is not supported in the current release" README.md sites/docs/src/getting-started/installation.md
rg -n "Windows 10 version 1903\\+|x86_64-pc-windows-msvc|MSVC|Visual Studio Build Tools|Windows SDK|WSL2|ARM64|MinGW|Cygwin" README.md sites/docs/src/getting-started/installation.md
```

The first `rg` command should return no matches. The second should show the
required Windows requirements and boundaries in the affected docs.

Optional docs-render command when available:

```sh
mdbook build sites/docs
```

If `mdbook` is unavailable, record that in implementation evidence and rely on
static markdown review plus PR CI.

## Live E2E Plan

Canonical interface: user-facing documentation source and PR-rendered markdown.
This issue does not touch LLM behavior, CLI runtime behavior, TUI behavior,
Studio behavior, or provider behavior.

Real provider/LLM path:

- None required.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Required credentials: none.

Commands and evidence:

- Run `git diff --check`.
- Run `git diff --name-only` and confirm the implementation diff is limited to
  approved docs surfaces.
- Run the targeted stale-text `rg` scan and confirm no stale unsupported Windows
  statement remains.
- Run the targeted requirements `rg` scan and confirm the Windows version,
  MSVC target, prerequisite, WSL2 boundary, and unsupported-target terms appear
  in the expected docs.
- Run `mdbook build sites/docs` only when `mdbook` is installed in the
  implementation environment.
- After opening the implementation PR, verify GitHub Actions shows the main
  `check` workflow taking the documentation-only path. Coverage and docs-review
  workflows are not expected for docs-only changes under current path filters.

Pass criteria:

- README and public installation docs agree on all required Windows setup
  terms.
- No stale unsupported Windows language remains in the affected onboarding
  surfaces.
- No out-of-scope files are changed.
- PR evidence clearly states that #420 and #422 boundaries remain separate.
- The issue remains open for later workflow stages.

Fail criteria:

- Any required Windows setup term is missing from README or public installation
  docs.
- README and public installation docs disagree on the support boundary.
- The diff includes Rust, tests, CI, generated assets, runtime assets, product
  specs, or unrelated docs without explicit human approval.
- The docs imply unsupported Windows targets or completed terminal fallback
  evidence without source-backed proof.
- The documentation-only CI path does not run or reports failure.

## Ralph Cycle Protocol

Each cycle must:

1. Summarize current docs state and remaining unmet requirements.
2. Propose one bounded implementation goal.
3. List intended file areas and commands before editing.
4. Estimate resource use and risk.
5. Check whether the resource gate requires human approval.
6. Implement only the approved or resource-permitted goal.
7. Run the agreed checks.
8. Report evidence, failures, and remaining gaps.
9. Stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded documentation implementation goal per cycle.
- Max files or modules per cycle: 2 docs files by default (`README.md` and
  `sites/docs/src/getting-started/installation.md`).
- Expected command budget:
  - `git diff --check`
  - targeted `rg` scans
  - optional `mdbook build sites/docs` when available
  - PR CI inspection after opening the implementation PR
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies or irreversible
  operations, changes CI/Rust/tests/generated assets/runtime assets/specs, or
  needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Expected external LLM usage for this issue: 0 calls, USD 0.
- Autonomous batch cap: 2 low-budget documentation cycles.
- When to stop and ask for human guidance:
  - Any implementation appears to require code, tests, CI, generated output, or
    product spec edits.
  - README and public docs cannot be made consistent without changing the
    approved product boundary.
  - #422 state changes create a product wording decision about whether terminal
    fallback evidence can be described as complete.
  - Docs validation requires installing new tools or changing repo metadata.

## Task Breakdown

1. Confirm #423 still has product spec approval and is in a Stage 10-ready state
   before implementation begins.
2. Review `README.md` requirements and supported-platforms sections.
3. Review `sites/docs/src/getting-started/installation.md` prerequisites and
   supported-platforms sections.
4. Draft README wording:
   - preserve Rust stable 1.80+ requirement
   - preserve macOS/Linux C compiler guidance
   - add Windows 10 version 1903+, `x86_64-pc-windows-msvc`, stable MSVC Rust,
     Visual Studio Build Tools or equivalent MSVC C++ tools, Windows SDK, and
     linker environment
   - state WSL2 is not required
   - state ARM64 Windows, MinGW/Cygwin/GNU, installers, packaging, and release
     signing are outside current support
5. Mirror the same essential guidance in
   `sites/docs/src/getting-started/installation.md`.
6. Run static checks and optional mdBook build when available.
7. Review the diff for scope boundaries and public-doc consistency.
8. Open the implementation PR and record evidence, including docs-only CI
   behavior.

The README edit and public installation docs edit should happen in the same
cycle to avoid intentional drift. Static checks can run after both files are
updated.

## Verification Plan

Required local/static checks:

- `git diff --check`
- `git diff --name-only`
- Targeted stale-text scan:
  `rg -n -e "\| Windows \|.*\| Not supported \|" -e "Windows is not supported in the current release" README.md sites/docs/src/getting-started/installation.md`
- Targeted requirements scan:
  `rg -n "Windows 10 version 1903\\+|x86_64-pc-windows-msvc|MSVC|Visual Studio Build Tools|Windows SDK|WSL2|ARM64|MinGW|Cygwin" README.md sites/docs/src/getting-started/installation.md`

Required PR review evidence:

- Diff limited to approved docs files or explicitly approved docs metadata.
- README and public installation docs agree on Windows support boundary.
- Existing macOS/Linux guidance remains clear.
- PR evidence states #420 CI files did not change.
- PR evidence states #422 terminal fallback files/tests did not change.
- PR evidence states no implementation code, tests, generated assets, runtime
  assets, product specs, or unrelated docs changed.

Optional validation:

- `mdbook build sites/docs` when `mdbook` is available.
- Rendered Markdown preview in GitHub PR UI for table formatting and prose
  readability.

Expected GitHub Actions evidence:

- Main `check` workflow runs for the implementation PR.
- The workflow reports the documentation-only path and does not run Rust
  setup/cargo commands for a docs-only diff.
- `coverage` is not expected to run for this docs-only diff under current path
  filters.
- `Docs Review (Copilot)` is not expected to run for this docs-only diff under
  current path filters.

## Completion Criteria

Before Stage 10 implementation is ready for review:

- README no longer contains the stale Windows unsupported row.
- Public installation docs no longer contain the stale Windows unsupported
  sentence.
- README and public installation docs both document:
  - Windows 10 version 1903+
  - `x86_64-pc-windows-msvc`
  - stable MSVC Rust toolchain
  - Visual Studio Build Tools or equivalent MSVC C++ tools
  - Windows SDK/linker/C compiler setup
  - WSL2 not required for native Windows
  - ARM64 Windows outside current support
  - MinGW/Cygwin/GNU Windows toolchains outside current support
  - installers, packaging, and release signing outside current support
- Existing macOS/Linux prerequisite guidance remains clear.
- No CI, code, test, generated artifact, runtime asset, product spec, or
  unrelated docs changes are included.
- Required static checks and available docs-render checks are reported.
- PR CI evidence is linked or summarized.
- The implementation PR body uses non-finalizing issue references and states the
  execution issue must remain open for later workflow stages.

## Failure And Escalation

- If the implementation cannot keep README and public installation docs
  consistent, stop and ask for a product wording decision.
- If a reviewer asks to change Windows support beyond Windows 10 version 1903+
  and x86_64 MSVC, stop and route that as a product-scope decision.
- If implementation needs Rust code, tests, CI, generated artifacts, runtime
  assets, or product spec edits, stop and ask for human approval because that
  exceeds this technical spec.
- If `mdbook` is unavailable, do not install tooling as part of the Ralph cycle
  without approval; record the unavailable check and continue with static review
  evidence.
- If GitHub Actions behaves differently from the expected docs-only path,
  record the observed behavior and ask for guidance before changing CI.
- If #422 reaches completed implementation evidence before #423 implementation
  starts, verify the issue and PR evidence before deciding whether docs wording
  may mention terminal fallback completion.

## Open Questions

- None blocking for Stage 9 technical spec review.

Non-blocking implementation note:

- Final public wording should be concise. Prefer a compact requirements list and
  one short Windows support-boundary paragraph over long roadmap prose.
