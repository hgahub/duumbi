# DUUMBI-553: Deterministic IntentSpec Preflight Quality Gate - Technical Specification

## Implementation Objective

Implement the approved DUUMBI-553 product spec by adding a deterministic, provider-free preflight quality gate for loaded or newly generated `IntentSpec` values before graph mutation begins.

Verified product-spec outcomes this technical spec implements:

- A structured preflight report can be computed for any loaded `IntentSpec` without provider credentials, network access, graph mutation, or YAML schema migration.
- `intent create` surfaces the report before interactive save confirmation, and non-interactive create paths still save while logging whether the generated spec would pass, warn, or block execution.
- `intent review` surfaces the latest computed report for the current YAML contents.
- `intent execute` runs preflight immediately after loading the spec and before status changes, snapshots, decomposition, context assembly, provider calls, graph writes, learning records, or archiving.
- `block` readiness stops execution and leaves the intent status, graph files, snapshots, and learning records unchanged.
- `warn` readiness remains visible in logs and allows the existing execution pipeline to continue.
- The report includes readiness, score, highest severity, stable issue records, suggested fixes, workspace-only reuse candidates, and deterministic decomposition hints.
- CLI, REPL/TUI, Studio, and shared workflow surfaces expose the same bounded human-readable report evidence.

This technical spec does not authorize implementation during Stage 8. Stage 10 implementation agents must request explicit Ralph-cycle approval before changing source code, tests, docs, generated artifacts, runtime assets, or product specs.

## Agent Audience

Primary implementation agents:

- Codex for local source inspection, Rust implementation, tests, documentation/help updates, and PR evidence.
- Oz or another cloud runner only if a human explicitly approves a Ralph cycle that needs longer-running validation or broader CI investigation.

Review agents:

- Codex or Oz for Stage 9 technical spec review.
- A specialized tester may run manual CLI, REPL/TUI, and Studio checks after deterministic tests pass.

## Source Context

Verified facts:

- Product spec: `specs/DUUMBI-553/PRODUCT.md`.
- GitHub issue: https://github.com/hgahub/duumbi/issues/553.
- Stage 5 acceptance: https://github.com/hgahub/duumbi/issues/553#issuecomment-4451667755 accepted the work for specification and carried forward block/warn, repair, threshold, reuse, and persistence questions.
- Stage 7 approval: https://github.com/hgahub/duumbi/issues/553#issuecomment-4451987043 approved the product spec, recorded no blocking findings, and routed to `Technical Spec Needed`.
- Workflow gate verified on 2026-05-14: #553 is open, Project status is `Technical Spec Needed`, and labels include `product-spec-approved` and `needs-tech-spec`.
- Current `IntentSpec` shape lives in `src/intent/spec.rs` and includes intent text, version, status, acceptance criteria, create/modify modules, i64 test cases, dependencies, optional context, creation time, and optional execution metadata.
- `src/intent/create.rs` generates specs through an LLM or known-benchmark fallback, has `run_create` / `run_create_with_context` log buffers, and currently saves after confirmation or `yes`.
- `src/intent/review.rs` prints and formats intent details for CLI and REPL-safe buffers.
- `src/intent/execute.rs` currently loads the intent, emits the intent line, marks `InProgress`, saves the YAML, snapshots `main.jsonld`, analyzes/decomposes tasks, assembles context, calls the provider, writes graph files, records learning, verifies, repairs, and archives/fails.
- `src/intent/coordinator.rs` deterministically decomposes create modules first, non-main modifications next, and final main modification last; it derives exports and main prompts from acceptance criteria and test cases.
- `src/intent/verifier.rs` executes i64 test cases, currently has explicit handling for `function == "main"`, and still models arguments and expected returns as `i64`.
- `src/context/analyzer.rs` scans workspace graph modules plus vendor/cache dependency modules; DUUMBI-553 v1 reuse candidates must be workspace-only, so implementation must not reuse `analyze_workspace` unchanged for scoring/rendering unless dependency modules are filtered out.
- `src/workflow.rs` wraps shared create/execute workflows and returns log vectors used by CLI/Studio-style callers.
- `crates/duumbi-studio/src/lib.rs` exposes JSON endpoints for create, get/review, and execute; create currently omits the log from the JSON success body, get/review builds HTML from loaded intent fields, and execute delegates to `server_fns::execute_intent_for_api`.
- `crates/duumbi-studio/src/server_fns.rs` delegates Studio create through `duumbi::workflow::create_intent` and execution through `duumbi::workflow::execute_intent`.
- `src/cli/repl.rs` routes `/intent review` through `handle_intent_review`, TUI create through `run_create_with_context`, and TUI execute through `run_execute`.
- `AGENTS.md` requires provider setup to remain `/provider`-centered, Query mode to remain read-only, bounded keyboard-complete TUI behavior, focused state-machine tests for REPL/TUI changes, and at least one manual smoke path for changed REPL/TUI interactions.
- `docs/architecture.md` identifies `IntentSpec` YAML, coordinator decomposition, verifier tests, graph snapshots, repair, and learning as part of the intent execution pipeline.
- `docs/coding-conventions.md` requires module-specific errors in library code, no `.unwrap()` in library code, doc comments for public items, `#[must_use]` on meaningful returned values, and focused tests for error behavior.
- Relevant Obsidian notes inspected:
  - `DUUMBI - PRD`: DUUMBI is intent-first, queryable before mutation, evidence-oriented, and human-verifiable.
  - `DUUMBI - Glossary`: a technical specification is agent-facing and Ralph cycles are permission-gated.
  - `DUUMBI Agentic Development Map`: read-only context gathering precedes write-capable mutation.
  - `DUUMBI - Development Intake to Delivery Workflow`: Stage 8 produces reviewed technical specs; Stage 10 implementation runs through approval-gated Ralph cycles.
  - `Intent-Driven Development`: intent should become explicit behavior, graph changes, tests, and evidence.
  - `Spec-First Agentic Development`: agents should not implement from vague intent; specs must define behavior, invariants, and verification.

Assumptions and recommendations:

- A new `src/intent/preflight.rs` module is the lowest-risk home for the report model, checks, scoring, workspace-only reuse collection, decomposition hints, and text rendering.
- Preflight should return a report, not an error, for spec-readiness findings; I/O failures from workspace graph scanning should become warning-level report issues where possible so spec-only checks still run.
- Malformed YAML remains owned by `load_intent`; preflight does not need to recover data that cannot deserialize into `IntentSpec`.
- Studio JSON response shapes may need additive fields for preflight logs/report evidence. Keep additive compatibility where possible.
- Existing verifier support for `function == "main"` conflicts with the product-spec rule that main test cases should be preflight errors. Treat the product spec as authoritative for DUUMBI-553 v1 because main tests are not reliable for the intended callable-function readiness gate.

## Affected Areas

Expected source changes:

- `src/intent/preflight.rs`
  - New report model, readiness calculation, deterministic checks, workspace-only reuse candidate collection, decomposition hints, and text renderer.
- `src/intent/mod.rs`
  - Export the preflight module.
- `src/intent/create.rs`
  - Run preflight after spec generation/context application and before save confirmation or auto-save.
  - Append bounded rendered report lines to the existing `log`.
- `src/intent/review.rs`
  - Include the latest computed report in CLI stderr rendering and REPL-safe `Vec<String>` formatting.
- `src/intent/execute.rs`
  - Run preflight immediately after `load_intent` and before any status, snapshot, decomposition, context, provider, graph, learning, repair, or archive side effect.
  - Return a failed execution result without mutating state when readiness is `block`.
- `src/workflow.rs`
  - Preserve report evidence in shared create/execute result logs.
  - Consider additive structured fields only if Studio/API code needs them; do not create a new public serialization contract broader than product surfaces require.
- `src/cli/commands.rs` or the existing CLI command dispatch file
  - Ensure `duumbi intent review` and `duumbi intent execute` use the updated review/execute paths and display preflight output consistently.
- `src/cli/repl.rs`
  - Ensure TUI create, review, and execute display the same bounded report without a new modal or focus trap.
- `crates/duumbi-studio/src/lib.rs`
  - Add report evidence to create/get/review/execute API responses or generated HTML/logs.
- `crates/duumbi-studio/src/server_fns.rs`
  - Preserve shared workflow preflight logs in Studio create/execute surfaces.
- `crates/duumbi-studio/src/app.rs`, `crates/duumbi-studio/src/script/studio.js`, and/or `crates/duumbi-studio/src/components/panels/intents_panel.rs`
  - Only if needed to render create/review report text in the existing workflow/log area.
- `src/intent/status.rs`
  - No expected change; mention only as an invariant boundary because blocked preflight must not archive or mark status.

Expected tests:

- Unit tests in `src/intent/preflight.rs` for report model, scoring, issue checks, reuse candidates, decomposition hints, and rendering.
- Focused create/review tests in existing intent or CLI/TUI test modules.
- Execute side-effect tests proving blocked preflight leaves status, graph files, snapshots, provider calls, learning records, and archives unchanged.
- Studio/shared workflow tests where current coverage makes this practical.

Potential documentation/help changes:

- CLI help or user docs only where needed to explain preflight readiness, blocking behavior, and warning behavior.

Generated/local artifacts expected only during validation:

- Temporary workspaces, `.duumbi/intents`, `.duumbi/graph`, `.duumbi/history`, and learning/log files inside test temp directories.
- No generated reports, screenshots, binaries, provider secrets, runtime assets, or product specs should be committed for this issue unless a later approved cycle explicitly expands scope.

## Technical Approach

### 1. Add A Pure Preflight Module

Create `src/intent/preflight.rs` with small, explicit data types:

- `IntentPreflightReport`
  - `readiness: IntentPreflightReadiness`
  - `score: u8`
  - `highest_severity: Option<IntentPreflightSeverity>`
  - `issues: Vec<IntentPreflightIssue>`
  - `suggested_fixes: Vec<String>` or derive from issues during rendering
  - `reuse_candidates: Vec<IntentReuseCandidate>`
  - `decomposition_hints: Vec<String>`
- `IntentPreflightReadiness`
  - `Pass`, `Warn`, `Block`
- `IntentPreflightSeverity`
  - `Error`, `Warning`, `Info`
- `IntentPreflightIssue`
  - stable `code`, `severity`, `field_path`, `message`, `suggested_fix`
- `IntentReuseCandidate`
  - module name/path, exported or defined function names, reason, confidence

Provide a public entry point such as:

```rust
pub fn run_preflight(spec: &IntentSpec, workspace: &Path) -> IntentPreflightReport
```

and a spec-only helper for tests:

```rust
pub fn run_spec_checks(spec: &IntentSpec) -> IntentPreflightReport
```

Recommendation: keep report findings as values rather than returning `Result` for readiness problems. Use `Result` only for lower-level helpers if needed, then degrade graph-scanning failures into warning issues at the top-level `run_preflight`.

### 2. Score Deterministically

Use the approved thresholds:

- `pass`: no `error` issues and score >= 85
- `warn`: no `error` issues and 60 <= score < 85
- `block`: any `error` issue or score < 60

Keep scoring simple and reviewable. Recommended v1 shape:

- Start from 100.
- Deduct fixed weights by stable issue code.
- Clamp to 0..100.
- Info issues and reuse candidates do not reduce score.
- Error-level findings normally deduct more than warnings, but readiness still blocks on any error even if score remains >= 60.

The implementation must not make score depend on filesystem traversal order. Sort modules, function names, issues, reuse candidates, and hints before rendering where order is not semantically fixed.

### 3. Implement Product-Specified Checks

Structural checks:

- Empty or whitespace-only `intent`: error.
- Unsupported `version`: error. v1 supports version `1`.
- No create or modify modules: error.
- Empty module names: error.
- Same module in both create and modify: error.
- Duplicate module names within `create` or `modify`: warning unless implementation documents a concrete ambiguity that justifies error.
- Missing `app/main`, `main`, or explicit `spec.context.entrypoint`: error for executable graph-changing intents.

Acceptance and test checks:

- Empty `acceptance_criteria`: error.
- Empty `test_cases`: error for executable graph-changing intents.
- `function == "main"` test case: error for DUUMBI-553 v1 readiness, even though the current verifier has a main fallback.
- Duplicate test names: warning.
- Test functions not inferably mentioned by acceptance criteria: warning.
- Acceptance-criteria function names without test coverage: warning when function names can be inferred conservatively.
- One test case for a function: warning.

Type and capability checks:

- `TestCase` is already typed as `i64` after deserialization; document that malformed non-i64 YAML remains a load/parse failure unless raw YAML preflight is later accepted.
- Boolean-like specs should have expected returns covering `1` and `0`; missing one side is a warning.
- Acceptance criteria mentioning strings, floats, collections, option/result, ownership, runtime output, or similar not-directly-verifiable behavior should warn that current v1 tests cannot fully prove the behavior.
- Division/modulo wording should warn unless a zero/denominator edge case is present.
- Branching/comparison wording should warn unless equal or boundary cases are present where inferable.

Implementation detail: use conservative lowercase token matching and simple snake_case/function-call extraction. Do not build a broad natural-language parser in v1.

### 4. Collect Workspace-Only Reuse Candidates

Implement a dedicated workspace scanner for `.duumbi/graph/**/*.jsonld` or add an analyzer mode that explicitly excludes `.duumbi/vendor` and `.duumbi/cache`.

For each parseable `duumbi:Module`, collect:

- `duumbi:name`
- function names from `duumbi:functions`
- exports from `duumbi:exports` when present
- source path relative to `.duumbi/graph`

Report info-level reuse candidates when:

- a planned create/modify module name already exists
- a test-case function name already exists in a workspace module
- an acceptance-criteria function name already exists in a workspace module

If workspace graph scanning fails, preflight should still run spec-only checks and include a warning such as `W_REUSE_SCAN_UNAVAILABLE`.

Do not scan or render vendor/cache reuse candidates in v1.

### 5. Generate Decomposition Hints Without Replacing Coordinator

Use the same high-level task ordering as `coordinator::decompose`:

- create-module tasks for `modules.create`
- non-main modify tasks for `modules.modify`
- final main modification

Hints should be advisory text in the report. They must not be consumed by coordinator behavior in v1 unless a later approved cycle explicitly changes coordinator contracts.

Warn when test-case functions cannot plausibly be hosted by any planned non-main module.

### 6. Render Bounded Human-Readable Output

Provide a renderer such as:

```rust
pub fn render_preflight_report(report: &IntentPreflightReport) -> Vec<String>
```

Recommended default output:

- one summary line: `Preflight: WARN (score 78, 2 warnings)`
- grouped `Errors`, `Warnings`, and `Info` lines, capped by count with an overflow line
- reuse candidates, capped
- decomposition hints, capped
- no raw JSON by default

Keep output readable in stderr, REPL/TUI output buffers, Studio HTML/log panels, and workflow JSON logs. Color may be added at CLI-only boundaries, but text labels must carry the meaning.

### 7. Integrate Create And Review

Create:

- After generated spec is finalized, including optional TUI clarified context and known benchmark fallback normalization, call preflight before confirmation/save.
- Append rendered report lines to `log`.
- Interactive create still lets the user decline or save; blocked preflight during create does not prevent saving.
- `yes` mode saves after logging the report.

Review:

- Load the spec, compute preflight from current YAML and workspace state, and render it with the existing detailed intent output.
- Update both stderr and REPL-safe formatting paths.
- Do not persist report output to YAML.

### 8. Integrate Execute Before Any Side Effect

In `run_execute_with_progress`, the side-effect boundary must be:

1. load spec
2. compute/render preflight
3. if `Block`, return `Ok(false)` or a preflight-specific unsuccessful workflow result without changing anything
4. only then emit existing execution setup and mutate state

The implementation must prove blocked preflight happens before:

- `spec.status = IntentStatus::InProgress`
- `save_intent`
- `snapshot::save_snapshot`
- `agent_analyzer::analyze`
- `coordinator::decompose`
- `context::assemble_context`
- provider mutation calls
- graph file writes
- learning records
- archive/failure status writes

Recommendation: emit the preflight report before `Executing intent: ...` or immediately after it, but keep the blocking decision before state mutation. Tests should assert behavior, not exact cosmetic order, unless the output contract is intentionally fixed.

### 9. Integrate Workflow And Studio Surfaces

Shared workflow:

- Preserve preflight render lines in `IntentCreateWorkflowResult.log` and `IntentExecuteWorkflowResult.log`.
- If structured report fields are added, make them additive and narrowly scoped to product surfaces.

Studio:

- API create success should expose log/report evidence or enough response data for the existing intent panel to show it.
- API get/review should include computed preflight report in the generated HTML or additive JSON fields.
- API execute should preserve blocked/warn/pass report lines in `log`.
- Avoid new modal behavior; render in existing workflow/log/detail areas.

### 10. Avoid Rejected Alternatives

Rejected for v1:

- LLM-based preflight.
- Automatic YAML repair or machine-applicable fixes.
- Persisting report output into `IntentSpec` YAML.
- Vendor/cache reuse scoring or rendering.
- Replacing graph validation, compilation, verifier tests, repair, rollback, or learning.
- Broad provider/model behavior changes.
- Query mode changes or startup provider warnings in unrelated flows.

## Invariants

- Preflight never mutates `.duumbi/graph`.
- Preflight never calls a provider or requires provider credentials.
- Preflight never changes `IntentSpec` status by itself.
- Preflight report output is recomputed from the current YAML and workspace state; stale text is not trusted.
- A blocked execution must not create a graph snapshot, graph write, learning record, archive file, or status change.
- A passing preflight is not proof that graph mutation, build, run, verifier, repair, or CI checks will pass.
- Warning-only reports remain visible in execution evidence.
- Query mode remains read-only.
- Provider setup remains `/provider`-centered.
- No provider secret is logged or rendered by preflight.
- Product spec approval and technical spec approval remain separate; this document does not approve itself.
- Stage 10 implementation must run only after explicit Ralph-cycle approval.

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
- Max files or modules per cycle: normally 1-3 tightly related Rust modules plus their focused tests; UI/API cycles may include one Studio/server surface plus tests. Do not mix pure report-model work with Studio rendering unless the prior cycle has passed.
- Expected command budget per model/check cycle: `cargo fmt --check`, targeted `cargo test` for touched modules, and `cargo test --all` when shared intent behavior changes.
- Expected command budget for execute side-effect cycles: focused tests proving no status/snapshot/graph/provider/learning/archive side effects on block, plus relevant `cargo test` targets.
- Expected command budget for REPL/TUI or Studio cycles: focused state/API tests and at least one manual smoke path for the changed interaction when practical.
- Approval required before every cycle: yes.
- When to stop and ask for human guidance: product-spec behavior conflicts with source reality; implementation requires YAML schema persistence; preflight would need provider/model behavior; Studio response compatibility requires a breaking API change; scoring thresholds or severity mapping would materially differ from the approved product spec; test cost grows beyond the approved cycle.

## Task Breakdown

1. Report model and scoring
   - Add `src/intent/preflight.rs`.
   - Define report, readiness, severity, issue, reuse candidate, and hint data types.
   - Implement deterministic scoring, threshold mapping, sorting, and summary rendering.
   - Add unit tests for pass, warn, block, score clamping, highest severity, and stable rendering.

2. Pure `IntentSpec` checks
   - Implement structural, acceptance/test, type/capability, boolean, edge-case, and module-target checks.
   - Add focused unit tests for every product-specified check category.

3. Workspace-only reuse candidates
   - Scan `.duumbi/graph/**/*.jsonld` only.
   - Extract module names, functions, exports, and relative paths.
   - Add tests proving workspace modules are included and vendor/cache modules are excluded.
   - Add scan-failure warning behavior if applicable.

4. Decomposition hints
   - Generate advisory hints matching current coordinator task ordering.
   - Warn on implausible function/module hosting.
   - Add tests for create-module, non-main modify, final main, and missing-host cases.

5. Create and review integration
   - Insert report rendering in `run_create_with_context`.
   - Insert report rendering in `print_spec_detail` and `format_spec_detail`.
   - Add focused tests for create `yes` logging and review formatting.

6. Execute integration and side-effect protection
   - Run preflight before any mutation side effect.
   - Return blocked execution as an unsuccessful result with report evidence.
   - Add tests proving blocked execution leaves status, graph, snapshots, provider calls, learning records, and archives unchanged.
   - Add tests proving warning-only execution reaches the existing path.

7. Shared workflow and Studio integration
   - Preserve preflight evidence in workflow logs.
   - Add additive Studio response/rendering support where needed.
   - Add API/server function tests or targeted Studio tests for create/review/execute visibility.

8. User-facing docs/help polish
   - Update only the minimal help/docs needed to explain pass/warn/block behavior.
   - Verify no provider setup, Query mode, or model selection behavior is broadened.

9. Final verification and review evidence
   - Run formatting, targeted tests, broader tests, and clippy as appropriate for touched areas.
   - Perform manual CLI/REPL/TUI/Studio smoke checks when UI surfaces change.
   - Produce a Stage 10 evidence report mapping behavior back to product and technical spec requirements.

## Verification Plan

Required automated checks for the full implementation:

- `cargo fmt --check`
- `cargo test --all`
- `cargo clippy --all-targets -- -D warnings`

Focused unit tests:

- pass report for a strong calculator-style spec
- warning report for weak edge coverage
- blocked report for empty intent text
- blocked report for unsupported version
- blocked report for no create/modify modules
- blocked report for empty module names
- blocked report for duplicate create/modify target
- warning report for duplicate module names inside one list
- blocked report for missing `app/main`, `main`, or explicit context entrypoint
- blocked report for empty acceptance criteria
- blocked report for no test cases on executable graph-changing specs
- blocked report for `main` test cases
- warning report for duplicate test names
- warning report for test functions not inferably covered by criteria
- warning report for criteria functions without test coverage
- warning report for one test per function
- boolean-like spec warning when expected returns do not cover both `1` and `0`
- division/modulo zero-edge warning
- branching/comparison boundary warning
- warning for not-directly-verifiable string/float/collection/result/option/ownership/runtime-output criteria
- workspace reuse candidate detection
- vendor/cache modules excluded from v1 reuse
- deterministic decomposition hint ordering
- bounded text rendering and overflow behavior

Focused integration or behavior tests:

- `intent create` logs preflight before save confirmation.
- `intent create -y` logs preflight and still saves.
- TUI create logs preflight in the output buffer.
- `intent review` displays the current computed report.
- TUI `/intent review` displays the same bounded report.
- blocked `intent execute` leaves intent status unchanged.
- blocked `intent execute` does not create a snapshot.
- blocked `intent execute` does not write graph files.
- blocked `intent execute` does not call the provider.
- blocked `intent execute` does not write learning records.
- blocked `intent execute` does not archive or move the intent.
- warning-only `intent execute` continues into the existing pipeline.
- shared workflow responses expose preflight evidence in logs.
- Studio create/review/execute surfaces expose text-readable preflight states.

Manual checks when relevant surfaces change:

- Create a valid calculator-style intent and confirm preflight pass/warn appears without requiring new provider/model configuration beyond existing create behavior.
- Review a saved weak intent and confirm the report is recomputed from current YAML.
- Execute a deliberately weak spec with no tests and confirm execution blocks before provider calls or graph writes.
- Inspect REPL/TUI output for bounded readable report text and no modal/focus trap.
- Inspect Studio create/review/execute output for text-readable pass/warn/block states.

## Completion Criteria

Implementation is complete only when:

- `src/intent/preflight.rs` exposes a deterministic provider-free report over `IntentSpec`.
- Approved thresholds and severities map exactly to `pass`, `warn`, and `block`.
- All in-scope product-spec checks have tests or a documented reason they are covered by existing deserialization/load behavior.
- Reuse candidates are workspace-only and vendor/cache modules are excluded.
- Create, review, execute, shared workflow, REPL/TUI, and Studio surfaces show bounded preflight evidence.
- Blocked execute returns before status/snapshot/decomposition/context/provider/graph/learning/archive side effects.
- Warning-only execute preserves warning evidence and continues through the existing pipeline.
- No product spec, runtime asset, generated artifact, unrelated provider/model behavior, or Query mode behavior is changed.
- Required automated checks pass, or failures are documented as unrelated with concrete evidence.
- Manual smoke evidence exists for changed CLI/REPL/TUI/Studio interactions when those surfaces are touched.

## Failure And Escalation

- If product-spec behavior conflicts with current source behavior, follow the approved product spec and document the source conflict in the cycle report. If the conflict changes public behavior materially, stop and ask for human guidance.
- If score thresholds or severity classifications need different values from 85/60 or error/warning/info, stop and request Stage 6/7 revision guidance.
- If preflight cannot inspect workspace graph modules because of I/O or parse issues, degrade reuse detection to a warning and continue spec-only checks unless the failure prevents deterministic behavior.
- If blocked execute still mutates any status, graph, snapshot, learning, or archive state in tests, treat it as a blocker and do not proceed to UI integration.
- If Studio needs breaking API changes to show report evidence, stop and propose an explicit compatibility decision.
- If test runtime becomes excessive, narrow to targeted tests for the current cycle and defer broader `cargo test --all` / clippy to the final implementation cycle with explicit approval.
- If implementation pressure expands into auto-repair, YAML persistence, vendor/cache reuse, provider/model setup, graph validation replacement, or Query mode changes, stop; those are out of scope for DUUMBI-553 v1.

## Open Questions

None blocking for v1.

Non-blocking implementation choices to settle during Stage 10 cycle proposals:

- Exact fixed score deductions per issue code.
- Exact cap counts for rendered issues, reuse candidates, and decomposition hints.
- Whether Studio should expose a structured `preflight` JSON field in addition to human-readable log/html text.
- Whether `run_execute` should return `Ok(false)` for preflight block or introduce a more specific workflow-level status while preserving existing caller compatibility.

## Stage 8 Notes

- Draft PR: not created, per explicit user instruction for this Stage 8 run.
- GitHub status/labels: not changed in this run because the normal Stage 8 outcome expects a draft PR link before routing to `Technical Spec Review`.
