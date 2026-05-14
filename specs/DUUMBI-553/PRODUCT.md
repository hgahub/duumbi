# DUUMBI-553: Deterministic IntentSpec Preflight Quality Gate

## Summary

Add a deterministic preflight quality gate for `IntentSpec` values before graph
mutation begins.

The gate should validate, score, and enrich an intent spec after generation or
manual review, then run again at the start of `intent execute`. Users should see
whether the spec is ready, what is weak or invalid, which fixes are suggested,
which existing workspace graph modules may be reusable, and what decomposition
hints will make execution more predictable.

For v1, severe structural or consistency problems block `intent execute`.
Warnings remain visible but do not block execution. The gate does not call an
LLM, does not rewrite the spec, and does not replace graph validation,
compilation, verifier tests, repair, or learning.

## Problem

`duumbi intent create` can produce a syntactically valid `IntentSpec` whose
content is too weak for reliable execution. Examples include missing
acceptance criteria, missing `app/main` intent wiring, duplicate or ambiguous
module targets, tests that do not match the requested behavior, unsupported test
types, missing edge cases, and specs that ignore reusable workspace graph
modules.

Today, `duumbi intent execute` accepts a loaded spec and immediately moves into
execution setup: it marks the intent in progress, snapshots the graph,
decomposes the spec, enriches prompts, asks the LLM to mutate graph files, runs
graph validation through later build paths, executes verifier tests, and may
attempt repair. Those downstream protections are valuable, but they happen after
the spec has already shaped task decomposition and mutation prompts.

The user-facing problem is that a weak upstream spec can cause avoidable
retries, bad patches, repair loops, unverifiable success, or confusing failure
evidence. Users need a deterministic readiness report before DUUMBI spends
provider calls and mutates the workspace.

## Outcome

When this is done:

- A deterministic preflight report can be produced for any loaded `IntentSpec`
  without provider credentials or network access.
- `duumbi intent create` displays the preflight report with the generated spec
  preview before interactive save confirmation.
- Non-interactive create paths, including `-y`, include the preflight report in
  the command or workflow log.
- `/intent create`, `/intent review`, Studio intent creation, and shared
  workflow responses expose the same report in a bounded human-readable form.
- `duumbi intent execute`, `/intent execute`, Studio execution, and the shared
  workflow service run preflight before marking the spec `in_progress`, taking a
  snapshot, decomposing tasks, calling a provider, writing graph files, or
  recording learning evidence.
- Blocking preflight failures stop execution with a clear report and leave the
  intent status, graph, snapshots, and learning logs unchanged.
- Warning-only reports allow execution to continue while preserving the warning
  details in logs.
- The report includes readiness status, score, highest severity, issues,
  suggested fixes, workspace reuse candidates, and decomposition hints.
- The report distinguishes spec-readiness checks from graph validation, compiler
  checks, verifier tests, and downstream repair.
- Existing valid intent workflows continue to execute without new provider or
  model configuration requirements.

## Scope

### In Scope

- Add a deterministic preflight report model for `IntentSpec` readiness.
- Add checks over the existing `IntentSpec` shape:
  - non-empty intent text
  - supported schema version
  - at least one target module
  - duplicate create/modify targets
  - ambiguous or invalid module names
  - `app/main` or an explicit entrypoint context
  - acceptance criteria presence
  - test-case presence for executable graph-changing intents
  - duplicate test names
  - `main` test-case exclusion
  - test-case function alignment with acceptance criteria when inferable
  - DUUMBI v1 verifier type constraints for i64 arguments and expected returns
  - boolean convention of `1` for true and `0` for false when the intent is
    boolean-like
  - edge-case coverage hints for division, modulo, branching, negative inputs,
    zero, strings, and empty-input wording
- Add a readiness score from 0 to 100 with deterministic thresholds.
- Add issue severities of `error`, `warning`, and `info`.
- Add human-readable suggested fixes for each actionable issue.
- Add workspace-only reuse candidate detection from `.duumbi/graph/**/*.jsonld`.
- Add deterministic decomposition hints based on modules, tests, and acceptance
  criteria.
- Integrate the report into CLI, REPL/TUI, Studio, and `src/workflow.rs`
  create/execute surfaces that already expose intent logs.
- Block `intent execute` on error-level findings or scores below the blocking
  threshold.
- Add focused tests for valid, warning, and blocked specs.
- Update user-facing docs or command help only where needed to explain the
  report and blocking behavior.

### Explicitly Out Of Scope

- Creating technical specifications or Ralph-cycle implementation instructions.
- Calling an LLM during preflight.
- Automatically rewriting or repairing the `IntentSpec`.
- Applying machine-generated fixes to YAML.
- Persisting preflight results into the `IntentSpec` YAML schema in v1.
- Treating vendor or cache modules as scoring or blocking reuse candidates in
  v1.
- Replacing graph validation, schema validation, compilation, verifier tests,
  repair, rollback, or learning logs.
- Broad redesign of `intent create`, `intent execute`, provider setup, model
  routing, Query mode, Agent mode, or Studio navigation.
- New public API guarantees for report serialization beyond what the product
  surfaces need for CLI, REPL/TUI, Studio, and workflow logs.

## Constraints And Assumptions

Facts:

- Issue #553 is the accepted canonical issue for the IntentSpec preflight
  quality gate.
- Stage 5 Human Acceptance exists for #553 and routes it to `Spec Needed`.
- The current `IntentSpec` contains intent text, version, status, acceptance
  criteria, create/modify module lists, i64 test cases, dependencies, optional
  context, creation time, and optional execution metadata.
- `intent create` currently uses an LLM or known benchmark fallback to generate
  an `IntentSpec`, previews it, and saves it after confirmation or `-y`.
- `intent execute` currently loads the intent, marks it in progress, saves it,
  snapshots the graph, decomposes tasks, assembles context, calls the provider
  for graph mutation, runs verifier tests, and may attempt repair.
- The coordinator decomposition is deterministic and depends heavily on
  `IntentSpec` modules, acceptance criteria, and test cases.
- The context analyzer already knows how to inspect workspace graph modules, and
  related code can inspect vendor/cache modules for prompt enrichment.
- The verifier currently models `TestCase` values as i64 arguments and i64
  expected returns.

Assumptions:

- Upstream `IntentSpec` quality is a major driver of downstream mutation
  reliability, retry cost, and verifier usefulness.
- A deterministic, inspectable gate is more useful for v1 than another
  provider-backed planning step.
- Users should be able to save a weak generated spec for manual editing, but
  execution should stop before graph mutation when the spec is clearly unsafe or
  unverifiable.
- Workspace-only reuse candidates are enough for v1 and reduce false positives
  compared with scanning vendor/cache modules for user-facing suggestions.
- Thresholds may be tuned after implementation evidence, but v1 needs explicit
  defaults so behavior is reviewable.

Constraints:

- Preflight must not require provider credentials.
- Preflight must not perform graph mutation or filesystem writes except normal
  command logs and existing create/save behavior.
- A blocked execution must leave the intent status unchanged and must not create
  a snapshot, mutate `.duumbi/graph`, append learning records, or archive the
  intent.
- A warning-only report must not silently disappear; users and Stage 11
  reviewers need the warning evidence in logs.
- Existing valid intent specs should not require schema migration.
- The gate must not create startup provider warnings during unrelated flows.
- Query mode must remain read-only.

## Decisions

- **Decision:** #553 should use a file-based product spec.
  **Evidence:** The work is cross-module, user-visible, architectural enough to
  affect execution safety, and useful as durable context for Stage 7, Stage 8,
  Stage 10, and Stage 11.

- **Decision:** v1 preflight is deterministic and provider-free.
  **Evidence:** The accepted issue asks for a deterministic gate, and the problem
  is upstream readiness, not another LLM planning layer.

- **Decision:** `intent execute` blocks on severe preflight findings in v1.
  **Evidence:** The stated user outcome is to avoid starting graph mutation from
  underspecified or internally inconsistent intent data. A warning-only gate
  would preserve visibility but would not stop the highest-cost failure mode.

- **Decision:** `intent create` shows the report but does not block saving in
  v1.
  **Evidence:** Saving an intent is not graph mutation, and users need a way to
  inspect and manually edit generated YAML. Blocking should happen before
  execution mutation, where the risk becomes material.

- **Decision:** v1 uses `pass`, `warn`, and `block` readiness states with score
  thresholds of 85 and 60.
  **Evidence:** Stage 5 carried threshold selection into Stage 6. The v1
  behavior is: `pass` when there are no errors and score is at least 85, `warn`
  when there are no errors and score is 60-84, and `block` when there is any
  error or score is below 60.

- **Decision:** Automatic spec repair is out of scope for v1.
  **Evidence:** The issue asks for suggested fixes, not automatic mutation. A
  deterministic report is inspectable and testable; automatic repair would add
  new product and safety behavior that should be accepted separately.

- **Decision:** Reuse candidates are workspace-only in v1.
  **Evidence:** The Stage 5 question explicitly asked whether vendor/cache reuse
  belongs in v1. Workspace-only keeps the report focused on modules the user is
  actively editing and avoids dependency-noise false positives.

- **Decision:** Preflight results are shown in command/workflow logs, not
  persisted into `IntentSpec` YAML in v1.
  **Evidence:** The Stage 5 question asked whether persistence belongs in v1.
  Avoiding YAML persistence prevents schema churn and stale report state while
  still giving users and reviewers visible evidence at create and execute time.

## Behavior

### Defaults

- Default readiness states:
  - `pass`: no error issues and score >= 85
  - `warn`: no error issues and 60 <= score < 85
  - `block`: at least one error issue or score < 60
- Default issue severities:
  - `error`: execution would be unsafe, ambiguous, or unverifiable
  - `warning`: execution may work but has quality, coverage, or clarity risk
  - `info`: useful context such as reuse candidates or decomposition hints
- Default reuse scope: workspace graph modules under `.duumbi/graph/**/*.jsonld`.
- Default execution behavior: warning-only reports continue; blocked reports stop
  before any graph mutation.

### Inputs

- Workspace root.
- Loaded or newly generated `IntentSpec`.
- Workspace graph module summaries, when available.
- Optional existing command or workflow log buffer.

No provider client, model selection, network access, or GitHub state is an input
to runtime preflight.

### Outputs

The report should be structured enough for tests and workflow consumers, and
bounded enough for human-facing logs. It should include:

- readiness state: `pass`, `warn`, or `block`
- numeric score from 0 to 100
- highest severity
- issue list with stable code, severity, field path, message, and suggested fix
- suggested fixes summarized by issue
- reuse candidates with module path, exported function names when available,
  reason, and confidence
- decomposition hints such as expected create-module tasks, expected main wiring,
  likely missing function targets, and test-function grouping

The user-facing rendering should avoid dumping raw JSON by default. A concise
CLI/REPL example shape is:

```text
Preflight: WARN (score 78)
Warnings:
  W_TEST_EDGE_COVERAGE test_cases - add a zero or division-by-zero case for div
Reuse candidates:
  math/ops - exports add, sub (possible duplicate module)
Decomposition hints:
  Create calculator/ops before modifying app/main
```

### Create And Review Behavior

- `duumbi intent create` runs preflight after generating the spec and before
  asking the interactive save question.
- `duumbi intent create -y` still saves the generated spec, but the log includes
  the report and clearly labels whether execution would pass, warn, or block.
- `/intent create` shows the same report in the TUI output panel without
  opening a new modal.
- `/intent review <slug>` includes the latest computed report for the loaded
  spec.
- Studio intent creation and review surfaces include the report in their
  existing workflow/log response area.
- A blocked report during create/review does not mean the YAML cannot be saved;
  it means `intent execute` should refuse until the spec is edited or
  regenerated.

### Execute Behavior

- `duumbi intent execute <slug>` runs preflight immediately after loading the
  spec and before:
  - changing `status`
  - saving the intent
  - taking a snapshot
  - calling `coordinator::decompose`
  - assembling mutation context
  - calling a provider
  - writing graph files
  - recording success or failure learning
  - archiving the intent
- If readiness is `block`, execution stops with a preflight-blocked result and a
  readable issue list. The intent remains in its previous status.
- If readiness is `warn`, execution continues and logs the warning report before
  the existing task profile and plan output.
- If readiness is `pass`, execution may show a one-line pass summary and
  continue.
- Studio and shared workflow execution use the same behavior as CLI execution.

### Check Categories

Structural checks:

- Empty intent text is an error.
- Unsupported schema version is an error.
- No create or modify modules is an error.
- Empty module names are errors.
- The same module appearing in both create and modify is an error.
- Duplicate module names within a list are warnings unless they create an
  execution ambiguity that Stage 8 classifies as an error.
- Missing `app/main` or explicit context entrypoint is an error for executable
  graph-changing intents.

Acceptance and test checks:

- No acceptance criteria is an error.
- No test cases is an error for executable graph-changing intents.
- Test cases for `main` are errors because the current verifier checks callable
  functions, not main-program behavior.
- Duplicate test names are warnings.
- Functions mentioned by test cases but absent from acceptance criteria are
  warnings when inferable.
- Acceptance criteria that imply functions with no test coverage are warnings
  when inferable.
- One test for a function is a warning; normal and edge coverage is preferred.

Type and capability checks:

- Non-i64 test arguments or expected returns are errors when they can be
  detected before YAML deserialization fails.
- Boolean-like specs should use `1` and `0` expected returns in test cases.
- String, floating-point, collection, option/result, ownership, or runtime-output
  behavior in acceptance criteria should produce a warning when current test
  cases cannot verify it directly.
- Division and modulo wording should produce an edge-case warning unless a zero
  or denominator edge case is present.
- Branching or comparison wording should produce an edge-case warning unless
  equal/boundary cases are present where relevant.

Reuse checks:

- Existing workspace modules with matching module names or exported function
  names become info-level reuse candidates.
- Reuse candidates must not reduce score below pass/warn/block thresholds by
  themselves.
- Vendor/cache module reuse remains out of v1 scoring and rendering.

Decomposition checks:

- The report should predict the high-level deterministic task shape from the
  spec: create-module tasks, non-main modify tasks, and final main modification.
- If tests imply functions that the create/modify module plan cannot plausibly
  host, the report should warn.
- Hints are advisory and should not replace coordinator behavior in v1.

### Empty And Error States

- If an intent YAML cannot be loaded, the existing load error remains the
  primary error; preflight does not need to recover malformed YAML.
- If workspace graph scanning fails, preflight should still run spec-only checks
  and include a warning that reuse candidates were unavailable.
- If a blocked execution happens in a non-interactive command, the command should
  fail or return an unsuccessful workflow result with a preflight-specific
  message rather than a provider, graph, compiler, or verifier failure.
- Missing provider credentials must not be reported by preflight because
  provider setup is not part of spec readiness.

### Cancellation, Retry, And Race Conditions

- Preflight is synchronous/deterministic enough that cancellation should leave no
  partial graph or intent mutation.
- Retry behavior belongs to downstream mutation and repair, not to preflight.
- If the intent YAML changes between review and execute, execute recomputes
  preflight from the current file and does not trust stale report text.
- Concurrent readers may compute reports independently; preflight must not rely
  on cached mutable global state.

### Accessibility And Focus Rules

- REPL/TUI output should remain bounded and keyboard-scrollable.
- A blocked preflight report must not open a modal that traps focus.
- Studio report rendering should expose text labels for readiness, score, issue
  severity, field, and suggested fix.
- The report must be readable as text; color alone must not convey pass, warn, or
  block.

### Invariants

- Preflight never mutates `.duumbi/graph`.
- Preflight never calls a provider.
- Preflight never changes `IntentSpec` status by itself.
- A blocked execution cannot create a graph snapshot or learning record.
- A passing preflight is not a proof that graph mutation, build, run, or verifier
  checks will pass.
- A warning-only preflight must remain visible in execution evidence.

## Tasks

- Define the preflight report model and readiness calculation.
- Implement deterministic checks over `IntentSpec` fields.
- Implement workspace graph reuse candidate collection for active workspace
  modules only.
- Implement deterministic decomposition hints.
- Add a concise text renderer for CLI/REPL/Studio workflow logs.
- Integrate preflight into `intent create` before save confirmation.
- Integrate preflight into `intent review`.
- Integrate preflight into `intent execute` before any status, snapshot, context,
  provider, graph, learning, or archive side effect.
- Integrate shared workflow responses so Studio receives the same evidence.
- Add tests for report scoring and severity aggregation.
- Add tests for create/review rendering.
- Add tests proving blocked execute leaves status, graph, snapshots, and learning
  evidence unchanged.
- Add tests proving warning-only specs still execute through the existing path.
- Add tests for workspace-only reuse candidates.
- Update docs or help text where users need to understand blocked execution.

Independent work:

- Report model, scoring, and pure `IntentSpec` checks.
- Text rendering.
- Workspace reuse candidate collection.
- Create/review integration.
- Unit tests for score/severity/check cases.

Sequential work:

- Execute integration should happen after the report model is stable because it
  must enforce block semantics before side effects.
- Studio/shared workflow evidence should follow CLI behavior so the surfaces do
  not diverge.
- Documentation should be finalized after user-facing wording is visible in
  command logs.

## Checks

- `cargo fmt --check`
- `cargo test --all`
- `cargo clippy --all-targets -- -D warnings`
- Focused unit tests for:
  - pass report for a strong spec
  - warning report for weak edge coverage
  - blocked report for empty acceptance criteria
  - blocked report for no test cases
  - blocked report for missing `app/main` or explicit entrypoint
  - duplicate create/modify target error
  - `main` test-case error
  - duplicate test name warning
  - boolean `1`/`0` convention
  - division/modulo zero-edge warning
  - workspace reuse candidate detection
  - vendor/cache modules excluded from v1 reuse rendering
  - decomposition hint grouping
- Focused integration tests for:
  - `intent create` logs preflight before save confirmation
  - `intent create -y` logs preflight and still saves
  - `intent review` displays the current computed report
  - blocked `intent execute` leaves intent status unchanged
  - blocked `intent execute` does not create a snapshot
  - blocked `intent execute` does not write graph files
  - blocked `intent execute` does not call the provider
  - warning-only `intent execute` continues into the existing pipeline
  - shared workflow responses expose the same report to Studio
- Manual checks:
  - Run a valid calculator-style intent and confirm preflight passes or warns
    without changing existing successful execution behavior.
  - Run a deliberately weak spec with no tests and confirm execution blocks
    before provider calls or graph writes.
  - Inspect REPL/TUI output for bounded, readable report rendering.
  - Inspect Studio create/review/execute output for text-readable pass/warn/block
    states.

## Open Questions

No blocking open questions for v1. The Stage 5 questions are answered by the
decisions above:

- severe findings block `intent execute`
- automatic repair is out of scope
- score thresholds are 85 for pass and 60 for block
- reuse candidates are workspace-only
- preflight results are not persisted into intent YAML

Future versions may revisit automatic repair, vendor/cache reuse, serialized
report artifacts, or stricter warning policies after v1 evidence exists.

## Sources

- GitHub Issue: https://github.com/hgahub/duumbi/issues/553
- Stage 5 acceptance comment:
  https://github.com/hgahub/duumbi/issues/553#issuecomment-4451667755
- Processed Inbox note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-05-14 - IntentSpec Preflight Quality Gate.md`
- PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Development Intake to Delivery Workflow:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Development Intake to Delivery Workflow.md`
- Intent-Driven Development dot:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Intent-Driven Development.md`
- Spec-First Agentic Development dot:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Spec-First Agentic Development.md`
- Architecture reference: `docs/architecture.md`
- Existing file-based product specs:
  - `specs/DUUMBI-487/PRODUCT.md`
  - `specs/DUUMBI-488/PRODUCT.md`
  - `specs/DUUMBI-489/PRODUCT.md`
- Relevant source context:
  - `src/intent/spec.rs`
  - `src/intent/create.rs`
  - `src/intent/execute.rs`
  - `src/intent/coordinator.rs`
  - `src/intent/review.rs`
  - `src/workflow.rs`
  - `src/context/mod.rs`
  - `src/context/analyzer.rs`
  - `src/graph/validator.rs`
