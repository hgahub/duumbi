# DUUMBI-489: docs(phase15): End-to-end test protocol document - Technical Specification

## Implementation Objective

Implement the approved DUUMBI-489 product spec by turning `docs/testing/phase15-walkthrough.md` into the canonical Phase 15 all-samples operational walkthrough for Calculator, String Utilities, and Math Library.

Verified product-spec outcomes this technical spec implements:

- `docs/testing/phase15-walkthrough.md` becomes the canonical Phase 15 all-samples walkthrough.
- A developer can follow the document from a fresh checkout to validate CLI REPL and Studio behavior for Calculator, String Utilities, and Math Library.
- The walkthrough states prerequisites for Rust, C compiler, local DUUMBI binary, Studio, provider configuration, and evidence report paths.
- Each sample section has copy-pasteable commands, expected graph/build/run evidence, timing targets, and pass/fail criteria.
- Studio validation covers the `Intents`, `Graph`, and `Build` workflow for each sample.
- Query mode remains documented and verified as read-only; mutation remains Agent mode or explicit intent execution.
- Provider, graph, compiler, run-output, report, and Studio shared-backend failures are distinguishable.
- The document links canonical issues and evidence for #486, #487, #488, and #489.
- Phase 15 closure language remains evidence-based and must not claim completion until all three sample paths match real accepted evidence.

This technical spec does not authorize implementation during Stage 8. Stage 10 implementation agents must request explicit Ralph-cycle approval before changing docs, code, tests, generated artifacts, or runtime assets.

## Agent Audience

Primary implementation agents:

- Codex for local documentation edits, source inspection, focused deterministic checks, and PR evidence.
- Oz or another cloud runner only if a human explicitly approves a Ralph cycle that needs long-running live-provider validation.

Review agents:

- Codex or Oz for Stage 9 technical spec review.
- A specialized tester may run live-provider Phase 15 evidence after #488 is implemented and credentials are available.

## Source Context

Verified facts:

- Product spec: `specs/DUUMBI-489/PRODUCT.md`.
- GitHub issue: https://github.com/hgahub/duumbi/issues/489.
- Stage 7 approval: https://github.com/hgahub/duumbi/issues/489#issuecomment-4449106306 approved the product spec and recorded no blocking findings.
- Workflow gate: #489 is open, labeled `product-spec-approved` and `needs-tech-spec`, and its Project status is `Technical Spec Needed`.
- Dependency issue: #488 is open and approved for technical specification. Its issue body defines the Math Library target intent and expected evidence, but its implementation evidence is not present in this worktree.
- Current walkthrough: `docs/testing/phase15-walkthrough.md` is scoped to Calculator and String Utilities and explicitly says not to expand into #488 or #489.
- Current harness: `src/cli/phase15_e2e.rs` supports `calculator` and `string-utils`; `math-library` is rejected as unsupported by an existing test.
- Current CLI command: `src/cli/mod.rs` describes hidden `phase15-e2e` task choices as `calculator` or `string-utils`.
- Current benchmark normalization: `src/intent/benchmarks.rs` recognizes Calculator and String Utilities only.
- Shared backend: `src/workflow.rs` exposes graph evidence, build, and run helpers. Graph evidence recursively scans `.duumbi/graph/**/*.jsonld`.
- Studio API surface: `crates/duumbi-studio/src/lib.rs` exposes `POST /api/build` and `POST /api/run` returning structured JSON.
- Studio workflow surface: `crates/duumbi-studio/src/components/footer.rs` renders exactly `Intents`, `Graph`, and `Build`.
- Studio chat behavior: `crates/duumbi-studio/src/ws.rs` routes Query mode through read-only query handling; Intent mode tells users to use the Intents panel; mutation occurs through Agent mode.
- Studio panel behavior: `crates/duumbi-studio/src/components/panels/intents_panel.rs` creates and executes intents, and `crates/duumbi-studio/src/components/panels/build_panel.rs` builds and runs the workspace.
- Existing #487 technical spec: `specs/DUUMBI-487/TECHNICAL.md` defines and implemented the current String Utilities harness/documentation pattern and explicitly deferred #489 final all-samples consolidation.
- Repo instructions: `AGENTS.md` requires source-aware work, Query mode read-only behavior, provider UX through `/provider`, focused REPL/TUI tests for interaction changes, and the standard Rust checks when code changes.
- Relevant Obsidian notes inspected:
  - `DUUMBI - PRD`: DUUMBI is evidence-oriented and human-verifiable.
  - `DUUMBI - Development Intake to Delivery Workflow`: GitHub is execution source of truth; Stage 8 produces technical specs; Stage 10 owns implementation through approval-gated Ralph cycles.
  - `DUUMBI Agentic Development Map`: read-only context gathering precedes write-capable mutation.
  - `DUUMBI - Service and Research Direction`: Studio E2E workflow remains partial until evidence is strong enough for public-facing claims.

Assumptions and recommendations:

- #488 accepted evidence is a hard implementation prerequisite for final Math Library command, timing, Studio, and pass/fail sections. Implementation may prepare document structure and prerequisite/failure-class sections before #488 completes, but must not invent Math Library evidence.
- Provider-per-sample evidence is acceptable if each provider is recorded, because Stage 7 explicitly treated single-provider consistency as non-blocking.
- Deterministic screenshot-ready Studio steps are acceptable unless Stage 9 or a human reviewer requires committed screenshots.

## Affected Areas

Expected documentation changes:

- `docs/testing/phase15-walkthrough.md`
  - Replace two-sample scope language with final all-samples protocol scope.
  - Add or revise prerequisites for Rust, C compiler, `cargo build`, Studio build/run, provider setup through `/provider` or supported env vars, and output directories.
  - Add a compact evidence matrix near the top unless Stage 10 proposes and gets approval for an equally scannable structure.
  - Normalize Calculator, String Utilities, and Math Library sections around consistent fields: issue, task id, provider, command, report path, expected graph evidence, expected build/run evidence, Studio checks, timing target, failure classification, and pass criteria.
  - Link canonical issues #486, #487, #488, #489 and relevant PR/evidence artifacts.
  - Remove stale statements that #488 or #489 are out of scope after the final protocol is implemented.

Implementation-dependent source areas to inspect during Stage 10, but not necessarily change for #489:

- `src/cli/phase15_e2e.rs` for supported task ids, report shape, timing, failure categories, Studio shared-backend evidence, and Ralph Gate output.
- `src/cli/mod.rs` for `phase15-e2e` task help text.
- `src/intent/benchmarks.rs` for known benchmark normalization and expected functions.
- `src/workflow.rs` for graph/build/run evidence behavior.
- `crates/duumbi-studio/src/lib.rs`, `crates/duumbi-studio/src/ws.rs`, `crates/duumbi-studio/src/components/footer.rs`, `crates/duumbi-studio/src/components/panels/intents_panel.rs`, and `crates/duumbi-studio/src/components/panels/build_panel.rs` for Studio API and UX facts.

Potential test/check areas if implementation touches code:

- `src/cli/phase15_e2e.rs` unit tests for task lookup, output predicates, graph evidence, UX checks, failure categories, and Math Library support if #488 adds it.
- `src/intent/benchmarks.rs` tests if known benchmark behavior changes.
- `src/workflow.rs` tests if graph evidence or build/run shared backend behavior changes.
- `crates/duumbi-studio/tests/` if Studio UI behavior changes.

Generated/local artifacts expected only during validation:

- `/tmp/duumbi-phase15-calculator-report.json`
- `/tmp/duumbi-phase15-string-utils-report.json`
- `/tmp/duumbi-phase15-math-library-report.json` or the accepted #488 report path
- Temporary workspaces under the OS temp directory

No generated reports, screenshots, binaries, runtime assets, or provider secrets should be committed unless a later approved spec revision explicitly requires them.

## Technical Approach

### 1. Convert The Walkthrough Into An Operational Protocol

Replace historical or issue-scoped framing with a reviewer-oriented protocol:

- Purpose and scope: prove Phase 15 CLI REPL and Studio workflow across all three accepted sample issues.
- Evidence status: distinguish accepted evidence, implementation prerequisite, local rerun command, and optional repeatability run.
- Prerequisites: Rust, C compiler, local `duumbi` binary, Studio server, provider setup, safe report paths, and secret-handling rules.
- Common workflow: build once, set `DUUMBI`, configure provider, run one sample, inspect report, validate Studio, classify failures, decide whether another Ralph loop is useful.
- Failure taxonomy: missing credentials, provider authentication/rate-limit/network/timeout, graph evidence mismatch, compiler/build failure, run-output mismatch, report serialization failure, Studio shared-backend failure, and UX/read-only-mode regression.

Recommendation: include a compact evidence matrix near the top with rows for `calculator`, `string-utils`, and `math-library`; columns should include issue, task id, module, expected functions/results, report path, CLI target, Studio target, timing target, and evidence status.

### 2. Preserve Existing Calculator And String Utilities Evidence

For Calculator:

- Preserve the canonical intent from #486.
- Keep module evidence for `calculator/ops`.
- Keep representative arithmetic output checks including `3 + 5 = 8` and `10 / 2 = 5` or the accepted equivalent evidence.
- Keep the under-10-minute target from the product spec.
- Link accepted issue/PR/evidence rather than embedding long historical logs as the primary path.

For String Utilities:

- Preserve the canonical intent from #487.
- Keep module evidence for `string/utils`.
- Keep function evidence for `reverse`, `count_vowels`, and `is_palindrome`.
- Keep representative output checks for `duumbi`, `ibmuud`, vowel count `3`, and `level` palindrome behavior.
- Use the under-15-minute target from the #489 product spec.
- Link accepted #487 implementation/evidence artifacts, including PR #546 where appropriate.

### 3. Gate Math Library Content On #488 Evidence

Math Library final content must be based on accepted #488 evidence.

Required Math Library facts from #488 issue context:

- Canonical target intent: `Build a math library with: factorial (recursive), fibonacci (iterative), and is_prime functions. The main function should compute factorial(10), fibonacci(15), and check if 97 is prime.`
- Stable task id should be `math-library` or an explicitly accepted equivalent.
- Expected module is `math/lib` unless #488 implementation accepts and records an equivalent mapping.
- Expected functions are `factorial`, `fibonacci`, and `is_prime`.
- Expected deterministic outputs are `factorial(10)=3628800`, `fibonacci(15)=610`, and `is_prime(97)=true` or `1`.
- Expected Studio validation uses the same shared backend style as Calculator and String Utilities.

Implementation agents may draft a clearly marked placeholder subsection before #488 completes, but the final walkthrough must not present speculative command output, timing, report paths, or Studio screenshots as accepted evidence. If #488 is still incomplete when a Stage 10 cycle reaches Math Library finalization, stop and request human guidance.

### 4. Keep Query And Provider UX Boundaries Intact

The walkthrough must state:

- Query mode is read-only by default.
- Graph mutation requires Agent mode or explicit intent execution.
- Provider setup should route through `/provider` or documented existing configuration paths.
- Documentation must not ask users to paste raw provider secrets into issues, docs, report examples, screenshots, or logs.
- Missing credentials are a blocked evidence condition, not a deterministic product failure.

### 5. Avoid Broad Source Changes In #489 Unless Needed By Reality

The expected #489 implementation is documentation-only if #488 has already added `math-library` harness support and accepted evidence. If Stage 10 discovers the final walkthrough cannot be truthful without source changes, the agent must stop the current docs-only cycle and propose a new bounded cycle explaining:

- the source gap
- the exact affected files/modules
- why #488 did not already own that change
- what tests and live evidence would be required
- whether the work exceeds #489's approved scope

Rejected alternatives:

- Do not make #489 implement the Math Library harness if #488 remains incomplete. That would collapse the prerequisite boundary and make the final protocol speculative.
- Do not broaden the walkthrough into Phase 16, Phase 13, Phase 14, public marketing, or stdlib ecosystem claims.
- Do not commit raw logs as the primary walkthrough path. Link historical logs and keep the main path operational.

## Invariants

- Product spec approval and technical spec approval remain separate; this document does not approve itself.
- Stage 10 implementation must run only after explicit Ralph-cycle approval.
- Query mode must remain read-only in CLI/Studio documentation and evidence.
- Mutation must remain Agent mode or explicit intent execution.
- Provider secrets must never appear in committed docs, reports, screenshots, logs, or GitHub comments.
- #488 is a hard prerequisite for final Math Library evidence and final all-samples pass/fail language.
- The final walkthrough must not claim Phase 15 completion unless all three sample paths are backed by accepted evidence.
- Existing Calculator and String Utilities behavior and traceability must not regress.
- Implementation must not modify generated artifacts, runtime assets, product specs, or unrelated source files unless a later approved cycle explicitly authorizes it.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. ask for explicit approval before starting
6. implement only the approved goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop if requirements are met or request approval for the next cycle

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: default 1 documentation file; up to 3 source/test modules only if source reality blocks truthful documentation and the user approves a code-touching cycle.
- Expected command budget for docs-only cycles: `git diff --check`, targeted Markdown inspection, and link verification by opening or otherwise validating referenced issue/PR URLs.
- Expected command budget for code-touching cycles, if approved later: focused unit tests for touched modules, `cargo fmt --check`, relevant `cargo test ...`, and broader `cargo test --all` / `cargo clippy --all-targets -- -D warnings` when shared behavior changes.
- Live-provider command budget: at most one approved `phase15-e2e` attempt per sample per cycle, unless the evidence report shows provider instability and the human approves another paid run.
- Approval required before every cycle: yes.
- When to stop and ask for human guidance: #488 evidence missing when Math Library finalization is needed; source changes exceed docs-only #489 scope; provider cost or timeout repeats; accepted evidence conflicts with the product spec; reviewer asks for committed screenshots or single-provider policy not currently approved.

## Task Breakdown

1. Preflight and evidence inventory
   - Confirm #489 remains in the correct workflow state.
   - Confirm #488 implementation/evidence status before finalizing Math Library content.
   - Inventory accepted evidence links for #486, #487, #488, and #489.

2. Walkthrough structure update
   - Rewrite top-level scope from two-sample historical log to all-samples operational protocol.
   - Add prerequisite and common execution sections.
   - Add compact evidence matrix.

3. Calculator section normalization
   - Preserve accepted command paths and pass criteria.
   - Move long historical logs behind links or an appendix if the document remains too noisy.
   - Keep timing and failure interpretation explicit.

4. String Utilities section normalization
   - Preserve accepted command paths and pass criteria.
   - Record provider/evidence path from accepted #487 artifacts.
   - Keep Studio shared-backend validation explicit.

5. Math Library section finalization
   - Use accepted #488 evidence only.
   - Add task id, command, report path, module/function evidence, expected outputs, timing, Studio checks, and pass/fail criteria.
   - If accepted #488 evidence is absent, stop with a blocking report instead of inventing content.

6. Failure taxonomy and reviewer decision flow
   - Add structured troubleshooting for provider, graph, compiler/build, run-output, report, Studio shared-backend, and UX/read-only failures.
   - Add guidance for when to rerun, switch provider, file a follow-up issue, or stop.

7. Final documentation review
   - Check headings, code fences, links, copy-pasteability, and secret safety.
   - Verify stale #488/#489 exclusion language is gone when final protocol is complete.
   - Report remaining gaps and evidence status.

## Verification Plan

Required for docs-only implementation:

- Inspect `docs/testing/phase15-walkthrough.md` rendered or as Markdown for readable heading hierarchy, code fences, tables, and links.
- Run `git diff --check`.
- Verify every command block is copy-pasteable or clearly marked as an example.
- Verify every referenced issue, PR, spec, report, or evidence comment link resolves.
- Verify no raw provider secret appears in the document.
- Verify stale statements excluding #488 or #489 are removed after final implementation.
- Verify the document explicitly gates Math Library finalization on accepted #488 evidence when that evidence is not yet merged or available.

Required once #488 is complete and accepted:

- Verify Calculator evidence from the final walkthrough by rerun or accepted evidence link.
- Verify String Utilities evidence from the final walkthrough by rerun or accepted evidence link.
- Verify Math Library evidence from the final walkthrough by rerun or accepted evidence link.
- Verify Studio `Intents`, `Graph`, and `Build` steps for all three samples against generated or accepted workspaces.
- Verify Query mode read-only behavior remains documented and, where harness evidence exists, reported.
- Verify timing targets are met or deviations are explicitly explained by accepted evidence.

Required if implementation touches code:

- Run focused tests for every touched Rust module.
- Run `cargo fmt --check`.
- Run `cargo test --all` and `cargo clippy --all-targets -- -D warnings` when shared CLI, Studio, workflow, intent, or harness behavior changes.
- Run at least one manual smoke path for changed REPL/TUI/Studio interactions when relevant.

## Completion Criteria

- `docs/testing/phase15-walkthrough.md` is the canonical Phase 15 all-samples protocol.
- The walkthrough covers Calculator, String Utilities, and Math Library with consistent task id, provider command, output report path, expected graph evidence, build/run evidence, Studio checks, timing target, and pass criteria.
- The walkthrough is faithful to accepted evidence and does not invent Math Library evidence before #488 is complete.
- The document links #486, #487, #488, #489, specs, PRs, and evidence artifacts clearly enough for Stage 9, Stage 11, and Stage 12 traceability.
- Provider, graph, compiler/build, run-output, report, and Studio shared-backend failures are distinguishable.
- Query mode read-only expectations are preserved.
- No raw provider secrets, generated reports, screenshots, binaries, or runtime assets are committed.
- Required docs-only checks pass, or code-touching checks pass if a later approved cycle expands scope.

## Failure And Escalation

- If #489 loses `Technical Spec Needed` status or product spec approval, stop and report the missing gate.
- If #488 evidence is missing when Math Library finalization is needed, stop and report that implementation is blocked on #488.
- If accepted #488 evidence conflicts with the #489 product spec, stop and request human guidance before choosing a documented behavior.
- If docs-only implementation exposes a source bug, stop after documenting the evidence and request approval for a separate code-touching cycle or follow-up issue.
- If provider credentials are missing, classify validation as blocked and explain the relevant env var/setup path without asking for raw secrets.
- If provider errors repeat, report provider-specific evidence and ask before spending another live attempt.
- If checks fail, report the failing command, relevant output summary, suspected cause, and the smallest next cycle needed.
- If scope expands into implementation code, generated artifacts, runtime assets, public marketing, Phase 16, Phase 13, or Phase 14, stop and ask for human approval or a separate issue.

## Open Questions

None blocking for implementation planning.

Non-blocking items for Stage 9 or Stage 10:

- Should the final protocol require a single provider across all three sample evidence runs, or is provider-per-sample evidence acceptable when each provider is recorded?
- Should committed screenshots be required, or are deterministic screenshot-ready Studio steps enough?
- Should the Math Library elapsed-time target remain under 15 minutes, or should accepted #488 evidence define a different target?
- Should the evidence matrix remain near the top, or should evidence live only within sample sections if reviewers find the matrix redundant?
