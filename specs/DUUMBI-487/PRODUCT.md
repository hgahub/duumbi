# DUUMBI-487: Phase 15 E2E String Utilities Sample

## Summary

Add product-level support for proving the second Phase 15 representative end-to-end workflow: a fresh user can create, execute, inspect, build, and run the String Utilities sample through the CLI REPL and Duumbi Studio with a real configured LLM provider.

The canonical task id should be `string-utils`, matching the issue language. The canonical user intent is:

```text
Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main.
```

The work should generalize the existing Phase 15 E2E harness beyond the Calculator-only path while preserving the Calculator behavior already validated by #486.

## Problem

Phase 15 claims that DUUMBI can support representative intent-driven development workflows across CLI REPL and Studio. The Calculator path has evidence, but the remaining Phase 15 milestone still needs String Utilities, Math Library, and final protocol documentation before the milestone can credibly close.

Today, the Phase 15 walkthrough and harness are calculator-focused. That leaves a product evidence gap: DUUMBI has not yet proven that a moderate string-focused workflow can produce the expected nested module, expose graph evidence, build, run, and stay visible through the simplified Studio workflow.

## Outcome

When this is done:

- `duumbi phase15-e2e string-utils --provider <provider> --attempts 1 --output <path>` or an equivalent stable task id runs the String Utilities E2E path.
- A fresh CLI workspace can create, execute, describe, build, and run the canonical String Utilities intent using a configured provider.
- Intent execution creates `string/utils` and updates `app/main`.
- Graph evidence shows `reverse`, `count_vowels`, and `is_palindrome`, or semantically equivalent generated names that are recorded in the report.
- The built program demonstrates all three functions with representative correct results.
- Studio validates the same generated workspace through the shared backend, including the `Intents`, `Graph`, and `Build` workflow, nested graph discovery, build, run, and read-only Query mode behavior.
- The Phase 15 report records CLI evidence, Studio evidence, elapsed time, UX checks, failure category, provider guidance, and Ralph Gate guidance.
- Missing provider credentials, provider timeout, provider error, code defects, documentation mismatches, and evidence mismatches are distinguishable in the report.
- `docs/testing/phase15-walkthrough.md` includes a dedicated String Utilities section and at least one evidence report path without turning #487 into the final all-samples protocol for #489.

## Scope

### In Scope

- Add String Utilities as a stable Phase 15 E2E task alongside the existing Calculator task.
- Preserve the Calculator task behavior and report shape.
- Generate or normalize the canonical String Utilities intent only to the extent needed to reduce provider variability for this sample.
- Validate creation of nested module path `.duumbi/graph/string/utils.jsonld`.
- Validate that `app/main` demonstrates all three generated functions.
- Validate CLI create, execute, describe or equivalent graph evidence, build, and run.
- Validate Studio graph visibility, build endpoint, run endpoint, and the simplified three-panel workflow.
- Validate Query mode remains read-only and mutation still requires Agent mode or explicit Intent execution.
- Update the Phase 15 walkthrough with String Utilities setup, commands, pass criteria, troubleshooting notes where sample-specific, and an evidence log entry.
- Add focused automated tests for harness/report behavior that do not require live provider credentials.

### Explicitly Out Of Scope

- Math Library sample validation, which belongs to #488.
- Final all-samples E2E protocol consolidation, which belongs to #489 except for the String Utilities section required here.
- New Studio navigation, chat, provider setup, or model-selection product behavior unrelated to proving this sample.
- Broad string stdlib design beyond what is required to validate the generated sample.
- Implementation of a technical architecture different from the existing shared backend approach unless Stage 8 explicitly approves it.
- Product spec approval, technical spec creation, Ralph-cycle authorization, or implementation work as part of this Stage 6 artifact.

## Constraints And Assumptions

Facts:

- #487 is the accepted canonical execution issue for the String Utilities Phase 15 E2E sample.
- Stage 5 acceptance exists and routes the issue to `Spec Needed`.
- The active Phase 15 spec defines Sample 2 as String Utilities with module `string/utils`, `app/main` modification, and three string-function test cases.
- The current walkthrough is scoped to the Calculator path and explicitly says not to expand into #487 or #488.
- The current harness is Calculator-only and rejects unsupported Phase 15 tasks.
- DUUMBI already has string-related primitives and fixtures, including `stdlib/string.jsonld`, `tests/fixtures/string_length.jsonld`, `tests/fixtures/string_concat.jsonld`, and `src/bench/showcases/string_ops.yaml`.

Assumptions:

- The canonical function names should be `reverse`, `count_vowels`, and `is_palindrome` unless provider output requires semantically equivalent names that are explicitly recorded.
- MiniMax remains the documented live-provider example because the Calculator walkthrough and issue validation use it, but the harness should remain provider-parameterized.
- The expected CLI elapsed budget should remain under 10 minutes unless implementation evidence proves the String Utilities sample needs a different Phase 15 budget.
- A provider-backed CLI pass is the live mutation gate; Studio should validate graph/build/run and UX against the same generated workspace through the shared backend rather than spending a second live provider mutation.

Constraints:

- Do not weaken Query/Agent/Intent mode safety: Query mode must remain read-only.
- Do not rely on raw secrets in docs, reports, logs, or evidence files.
- Missing credentials must produce structured `missing_provider_credentials` evidence, not a panic or ambiguous test failure.
- Provider timeout or provider instability must be classified separately from deterministic code, graph, compiler, docs, and evidence defects.
- Nested module path handling must preserve slash-named modules on disk and in graph discovery.
- The product evidence should be human-readable enough for Stage 7 review and later Stage 11 review mapping.

## Decisions

- **Decision:** #487 is the canonical String Utilities execution issue.
  **Evidence:** Stage 4 triage comment on #487 says to keep it as the canonical execution issue and found no duplicate.

- **Decision:** The issue is accepted for specification.
  **Evidence:** Stage 5 Human Acceptance Decision comment on #487 records `Decision: Accept` and `Next state: Spec Needed`.

- **Decision:** This spec should be file-based, not an issue-only comment.
  **Evidence:** The work is user-visible, cross-surface, harness-and-docs affecting, and useful as durable implementation context for Stage 8 and later review.

- **Decision:** The String Utilities intent must not expand into Math Library or final protocol consolidation.
  **Evidence:** #487 scope excludes #488 and #489 except for the minimal String Utilities walkthrough update; #488 and #489 remain separate open issues.

- **Decision:** CLI remains the provider-backed live execution gate, while Studio validates the shared backend against the already-generated workspace.
  **Evidence:** The Calculator walkthrough documents this as the shared backend validation approach after #486, and #487 implementation notes request reusing that precedent.

## Behavior

### Defaults

- The stable task id is `string-utils`.
- The provider argument works the same way as the Calculator harness.
- The default attempt count behavior remains consistent with the existing harness.
- The default report schema should keep existing Calculator fields and add String Utilities evidence without breaking consumers of the Phase 15 report.

### Inputs

- Required task id: `string-utils`.
- Required provider selection: existing provider argument or configured default path already supported by the command.
- Optional attempts count.
- Optional report output path.
- Optional Studio port, if already supported by the harness.
- Provider credential through the existing provider-specific environment variable or configured provider path.

### Outputs

- JSON report with task, provider, attempts, per-attempt CLI and Studio evidence, performance, UX checks, failure category, and Ralph Gate guidance.
- CLI evidence including workspace path, intent slug, graph/module checks, build path, run result, stdout/stderr, elapsed time, seeded/harvested learning counts when available, and whether `string/utils` exists.
- Studio evidence including shared backend workspace use, graph contains `string/utils`, build endpoint success, run endpoint success, footer workflow items, Query mode read-only state, and Agent mode availability.
- Documentation updates in `docs/testing/phase15-walkthrough.md` that explain how to run and judge the String Utilities sample.

### Success States

- The CLI leg creates a fresh workspace, creates the canonical intent, executes it, finds `string/utils`, finds all three expected string operations in graph evidence or report-recorded equivalents, builds successfully, runs successfully, and demonstrates correct representative behavior.
- The Studio leg uses the CLI-generated workspace, confirms the three-panel workflow remains `Intents`, `Graph`, `Build`, sees `string/utils`, builds successfully, runs successfully, and confirms Query mode remains read-only.
- The report is written to the requested output path and has enough evidence for a human reviewer to decide whether the sample passed.

### Representative Correct Results

The implementation should choose deterministic representative inputs. The exact strings can be set in Stage 8, but the evidence must prove all three operations. Acceptable examples include:

- `reverse("duumbi") = "ibmuud"` or an equivalent non-palindrome string reversal.
- `count_vowels("duumbi") = 3` if `u`, `u`, and `i` are counted as vowels.
- `is_palindrome("level") = true` and/or `is_palindrome("duumbi") = false`.

If provider output uses different sample strings, the report must record the observed input/output pairs and why they satisfy the operation semantics.

### Empty And Error States

- Unknown task ids should return a clear unsupported-task error that lists supported task ids.
- Missing provider credentials should report `missing_provider_credentials` with the missing environment variable name, without exposing secret values.
- Provider authentication, rate-limit, network, timeout, and malformed-output failures should be classified separately where detectable.
- Build failures should include structured build evidence and should not be misclassified as provider failures once graph generation has completed.
- Run failures should distinguish missing binary, process launch failure, nonzero process exit with otherwise correct stdout, and output mismatch.
- Missing `string/utils`, missing expected functions, or incorrect representative outputs should be `evidence_mismatch` or an equivalent deterministic evidence category.

### Retry, Timeout, And Cancellation

- Provider-backed CLI mutation should keep the existing bounded timeout model.
- A timeout must write structured evidence and harvest any available sanitized learning records.
- Attempts should be independent fresh workspaces while allowing the existing learning cache behavior to seed later attempts.
- User interruption should leave any partial report or console evidence in a state that does not imply a pass.

### Race Conditions And Invariants

- Studio validation must not start until the CLI leg has passed and reported a reusable workspace.
- Graph discovery must recursively include nested modules under `.duumbi/graph/**/*.jsonld`.
- Slash-named module paths must remain slash paths on disk, not flattened names.
- The report should preserve the distinction between generated graph existence, build success, and run correctness.
- Query mode must not mutate the workspace; mutation remains Agent mode or explicit Intent execution.

### Accessibility And Focus Rules

- The three primary Studio workflow items must remain visible and keyboard-reachable as `Intents`, `Graph`, and `Build`.
- Query and Agent mode controls must remain distinguishable to keyboard and screen-reader users through existing labels/states.
- Build and run results must remain exposed as readable text, not only transient visual state.

## Tasks

- Generalize the Phase 15 harness task model so `calculator` and `string-utils` are both supported stable task ids.
- Define a String Utilities task descriptor containing the canonical prompt, expected module path, expected operation names or semantic checks, representative run-output expectations, and report labels.
- Preserve Calculator-specific normalization and add String Utilities-specific normalization only if provider variability makes live validation unreliable.
- Extend CLI leg evidence checks for `string/utils`, `reverse`, `count_vowels`, `is_palindrome`, build output, run stdout, and failure categories.
- Extend Studio leg validation to check `string/utils` against the shared backend workspace, without triggering a second live provider mutation.
- Ensure nested graph path checks work for `.duumbi/graph/string/utils.jsonld`.
- Update report aggregation, performance, UX checks, and Ralph Gate guidance to account for the selected task.
- Add focused tests for supported/unsupported task ids, missing provider credentials, report serialization, String Utilities evidence classification, and nested module path checks.
- Update `docs/testing/phase15-walkthrough.md` with String Utilities commands, pass criteria, report examples, provider credential behavior, and evidence-log placeholder or local report path.
- Run regression checks and the live provider command when credentials are available.

Independent work:

- Harness task descriptor refactor and unsupported-task tests.
- String Utilities evidence predicates and report tests.
- Documentation update.

Sequential work:

- Live provider CLI validation before Studio shared-backend validation.
- Documentation evidence log after a local report path exists.

## Checks

- `cargo fmt --check`
- `cargo test --all`
- `cargo test -p duumbi-studio --features ssr`
- `cargo clippy --all-targets -- -D warnings`
- Focused harness tests for:
  - `calculator` remains supported.
  - `string-utils` is supported.
  - unsupported task ids return a clear error.
  - missing provider credentials produce `missing_provider_credentials`.
  - report JSON includes task-specific CLI evidence, Studio evidence, elapsed time, UX checks, failure category, and Ralph Gate guidance.
  - nested module path checks preserve `string/utils`.
- Live/manual validation when credentials are available:

```bash
cargo build
cargo build -p duumbi-studio --features ssr
export MINIMAX_API_KEY="..."
./target/debug/duumbi phase15-e2e string-utils \
  --provider minimax \
  --attempts 1 \
  --output /tmp/duumbi-phase15-string-utils-report.json
```

Pass criteria:

- The report result passes or, if blocked, clearly identifies a provider/credential class rather than a deterministic code/docs mismatch.
- CLI evidence includes a fresh workspace, generated slug, `string/utils`, the three string utility behaviors, build path, run evidence, elapsed time, and failure category if any.
- Studio evidence includes shared backend workspace, `string/utils` graph visibility, build success, run success, `Intents`/`Graph`/`Build` UX checks, Query read-only state, and Agent mode availability.
- The walkthrough documents the protocol and at least one local evidence report path for this sample.

## Open Questions

None blocking for product specification.

Non-blocking items for Stage 8:

- Choose the exact representative strings used for run-output checks.
- Decide whether String Utilities needs deterministic benchmark normalization or whether raw provider output is stable enough.
- Decide whether semantically equivalent function names are accepted automatically or require strict canonical names for this sample.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/487
- Stage 4 triage result: https://github.com/hgahub/duumbi/issues/487#issuecomment-4416382721
- Stage 5 Human Acceptance Decision: https://github.com/hgahub/duumbi/issues/487#issuecomment-4416434645
- Calculator precedent: https://github.com/hgahub/duumbi/issues/486
- Math Library separate scope: https://github.com/hgahub/duumbi/issues/488
- Final protocol documentation separate scope: https://github.com/hgahub/duumbi/issues/489
- Phase 15 walkthrough: `docs/testing/phase15-walkthrough.md`
- Phase 15 harness: `src/cli/phase15_e2e.rs`
- String showcase: `src/bench/showcases/string_ops.yaml`
- String fixtures: `tests/fixtures/string_length.jsonld`, `tests/fixtures/string_concat.jsonld`
- DUUMBI PRD: `DUUMBI - PRD`
- DUUMBI Development Intake to Delivery Workflow: `DUUMBI - Development Intake to Delivery Workflow`
- DUUMBI Glossary: `DUUMBI - Glossary`
- DUUMBI Agentic Development Map: `DUUMBI Agentic Development Map`
- DUUMBI Product Roadmap 2026-05: `DUUMBI - Product Roadmap 2026-05`
- DUUMBI Phase 15 Studio Workflow Redesign: `DUUMBI - Phase 15 - Studio Workflow Redesign`
