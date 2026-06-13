# DUUMBI-673: BDD Specification Artifacts For Runtime Intents - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-673/PRODUCT.md` by adding
plain-text BDD/Gherkin companion artifacts to DUUMBI runtime intents and by
feeding those scenarios into intent review, preflight, execution prompts,
repair prompts, and execution evidence.

Required product outcomes implemented by this technical spec:

- New executable `intent create` flows produce or help author at least one
  linked `.feature` companion artifact by default.
- `IntentSpec` references those artifacts through a backward-compatible YAML
  field.
- Users can inspect and edit `.feature` files outside DUUMBI.
- CLI, REPL/TUI, Studio, and shared workflow surfaces expose BDD artifact paths,
  scenario count, readiness, and coverage summaries in bounded text.
- `intent review` and `intent execute` recompute BDD readiness from current
  files.
- Missing or structurally unusable explicitly linked BDD files block
  `intent execute` before mutation side effects.
- Legacy intents without BDD references remain loadable and executable with a
  warning.
- BDD scenarios guide builder and tester agents but do not become a Cucumber
  runtime or replace existing i64 verifier tests.

Related to #673. This is a technical-specification artifact only. The execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex App implementation agents running bounded local Ralph cycles.
- Codex Cloud or Oz implementation agents only when a human routes longer
  validation or provider-backed live E2E work there.
- Reviewer agents checking schema compatibility, prompt boundaries, side-effect
  ordering, BDD-to-test evidence, and UI/log output.
- Tester agents validating CLI, REPL/TUI, Studio, shared workflow, and live
  provider-backed intent flows.
- Stage 9 technical reviewers checking implementability, scope boundaries, and
  resource policy.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/673
- Product spec: `specs/DUUMBI-673/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/694
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/673#issuecomment-4695365838
- Stage 6 product spec draft/link comment:
  https://github.com/hgahub/duumbi/issues/673#issuecomment-4695482575
- Stage 7 human approval decision:
  https://github.com/hgahub/duumbi/issues/673#issuecomment-4695571580
- Product spec merge commit: `d9085594d5f45d0b2cf8b0b5e2f3d0e0e2156906`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant source facts verified for Stage 8:

- `src/intent/spec.rs`
  - `IntentSpec` currently has no BDD field.
  - Existing fields are `intent`, `version`, `status`, `acceptance_criteria`,
    `modules`, `test_cases`, `dependencies`, optional `context`, optional
    `created_at`, and optional `execution`.
  - Public structs require doc comments.
  - Legacy YAML deserialization is tolerant for missing optional fields only
    when new fields use `#[serde(default)]`.
- `src/intent/create.rs`
  - `INTENT_SYSTEM_PROMPT` currently asks for JSON containing acceptance
    criteria, module targets, i64 test cases, and dependencies only.
  - `parse_llm_response` extracts the first JSON object and drops `main` test
    cases.
  - `run_create_with_context` previews the generated spec with
    `format_spec_detail_with_workspace`, asks confirmation when interactive, and
    then saves only the YAML intent.
  - Known benchmark fallback can return deterministic specs when LLM parsing
    fails.
- `src/intent/review.rs`
  - CLI and REPL review render intent fields and the current deterministic
    preflight report.
  - Review has separate stderr and log-buffer renderers.
- `src/intent/preflight.rs`
  - DUUMBI-553 already implemented deterministic provider-free readiness checks.
  - `run_preflight` returns `IntentPreflightReport`; blocked reports are used by
    execute before provider construction.
  - `render_preflight_report` is already used by CLI, REPL/TUI, Studio, and
    shared workflow evidence.
- `src/intent/execute.rs`
  - `run_execute_blocking_preflight` loads the spec and can block before a
    provider is constructed.
  - `run_execute_with_progress` currently loads the spec, runs preflight, then
    marks `InProgress`, saves the intent, snapshots graph state, decomposes
    tasks, assembles context, calls the provider, writes graph files, runs
    verifier tests, and may run an LLM repair.
  - `build_task_prompt` includes intent, clarified context, acceptance
    criteria, benchmark guidance, and the current task.
  - `build_repair_prompt` includes failed verifier details but no BDD scenario
    context.
- `src/intent/coordinator.rs`
  - Decomposition is deterministic and uses the existing `IntentSpec` fields.
  - BDD should influence decomposition, prompts, and evidence through bounded
    summaries, but should not replace the coordinator in v1.
- `src/intent/verifier.rs`
  - Verifier tests are i64 function calls and expected i64 returns.
  - BDD scenarios must map to verifier tests only when their observable outcome
    is actually covered by that model.
- `src/workflow.rs`
  - Shared create/execute return log vectors used by CLI and Studio-like
    callers.
- `src/main.rs`
  - `duumbi intent create` and `duumbi intent execute` already go through
    shared intent create/execute functions and the provider-free execute
    preflight check.
- `src/cli/repl.rs`
  - TUI create calls `run_create_with_context(..., yes=true, ...)`.
  - TUI execute calls `run_execute_blocking_preflight` before selecting a
    provider and then `run_execute`.
  - Output is appended to the existing chat/output buffer; avoid new modals.
- `crates/duumbi-studio/src/server_fns.rs`
  - Studio create delegates to `duumbi::workflow::create_intent`.
  - Studio detail rendering includes preflight HTML from
    `duumbi::intent::preflight`.
  - Studio execute calls `run_execute_blocking_preflight` before provider setup
    and exposes `preflight` lines in `IntentExecuteApiResponse`.
- `crates/duumbi-studio/tests/studio_layout.rs`
  - Existing tests create `IntentSpec` literals; adding a required non-default
    field would break these tests, so BDD references must be defaulted.
- `docs/testing/phase15-walkthrough.md`
  - Existing live sample protocol exercises CLI-generated workspaces and Studio
    shared-backend validation.

Relevant durable context:

- PRD: DUUMBI is intent-first, evidence-oriented, and human-verifiable.
- Glossary: an intent should become explicit defaults, outputs, edge cases,
  invariants, and verification steps before implementation.
- Original processed inbox note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-05-18 - BDD In Intent Workflow.md`
  clarifies this is product behavior for DUUMBI users, not DUUMBI's internal
  development workflow.

Assumptions and recommendations:

- Use an additive YAML field named `bdd` with `feature_files` for v1:

```yaml
bdd:
  feature_files:
    - features/<slug>.feature
```

- Store default generated BDD files beside the YAML intent at
  `.duumbi/intents/<slug>/features/<slug>.feature`.
- Treat `features/<slug>.feature` as intent-relative when resolving from
  `.duumbi/intents/<slug>.yaml`; also support safe workspace-relative paths
  under `.duumbi/intents/` for future multi-file edits.
- Put BDD parsing, readiness, coverage, rendering, and prompt summaries in a
  new `src/intent/bdd.rs` module instead of overloading the existing
  `preflight.rs`.
- Integrate BDD findings into the existing preflight report as additional
  issues so every surface already consuming preflight gets BDD readiness.
- Keep v1 parsing deterministic and conservative. Do not add a Gherkin crate or
  Cucumber runtime unless Stage 10 discovers an accepted, low-risk dependency
  already present in the workspace.

## Affected Areas

Expected Stage 10 source changes:

- `src/intent/spec.rs`
  - Add `IntentBdd` and a defaulted `bdd` field to `IntentSpec`.
  - Update existing struct literals and tests.
- `src/intent/mod.rs`
  - Export the new `bdd` module.
- New `src/intent/bdd.rs`
  - Feature file path resolution and safety checks.
  - Lightweight Gherkin parser.
  - BDD readiness report and issue model.
  - Scenario-to-verifier coverage classifier.
  - Bounded renderers for review/preflight logs.
  - Bounded prompt-context formatter for execute and repair prompts.
  - Default feature file generation helpers.
- `src/intent/create.rs`
  - Extend LLM prompt/response handling to include BDD feature text.
  - Add deterministic fallback feature generation from the generated
    `IntentSpec`.
  - Save YAML and feature artifacts together after confirmation.
  - Include BDD path/count/coverage in preview and save logs.
- `src/intent/review.rs`
  - Add BDD readiness and coverage rendering after the existing preflight block
    or as part of preflight lines.
- `src/intent/preflight.rs`
  - Append BDD readiness issues into `IntentPreflightReport` for explicit BDD
    references.
  - Emit warning-only legacy behavior for specs with no BDD references.
- `src/intent/execute.rs`
  - Load BDD report once before side effects.
  - Include BDD scenario context in task and repair prompts.
  - Emit execution evidence for loaded feature files, scenario count, coverage,
    and scenarios requiring broader evidence.
- `src/workflow.rs`
  - Preserve BDD create/execute evidence in existing logs.
  - Add structured fields only if Studio tests prove log extraction is
    insufficient.
- `src/main.rs`
  - No new command is required; confirm existing command paths show BDD
    evidence.
- `src/cli/repl.rs`
  - Confirm TUI output displays BDD evidence in the existing output buffer.
  - Add focused tests if output routing changes.
- `crates/duumbi-studio/src/server_fns.rs`
  - Add BDD detail HTML and API evidence for create, detail/review, and execute.
  - Extend log extraction if BDD lines are separate from preflight lines.
- `crates/duumbi-studio/tests/studio_layout.rs`
  - Update `IntentSpec` literals and add BDD detail rendering tests.
- Documentation/help:
  - Add minimal user-facing docs or help text for BDD artifact layout and edit
    expectations. Candidate files include `docs/architecture.md`,
    `docs/testing/phase15-walkthrough.md`, or a focused docs file under
    `docs/`.

Areas that must not change during Stage 8:

- implementation code
- tests
- generated `.feature` runtime artifacts
- product specs
- issue workflow automation

Areas out of Stage 10 scope unless later approved:

- A full Cucumber/Gherkin runtime.
- Non-i64 verifier payloads.
- Broad redesign of coordinator decomposition. A narrow BDD-summary input to
  task decomposition is in scope.
- Query mode behavior.
- DUUMBI internal Stage 6/8 spec workflow.

## Technical Approach

### 1. Add Backward-Compatible IntentSpec BDD References

Add the following model to `src/intent/spec.rs`:

```rust
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct IntentBdd {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_files: Vec<String>,
}

pub struct IntentSpec {
    // existing fields
    #[serde(default, skip_serializing_if = "IntentBdd::is_empty")]
    pub bdd: IntentBdd,
}
```

Implementation may add `IntentBdd::is_empty()` as a small helper. The field
must default during deserialization so legacy YAML remains loadable. Keep the
schema version at `1` for v1 because this is an additive compatible extension;
only bump the version if Stage 10 discovers a breaking YAML requirement.

Use intent-relative feature paths by default:

```text
.duumbi/intents/<slug>.yaml
.duumbi/intents/<slug>/features/<slug>.feature
```

Persist in YAML as:

```yaml
bdd:
  feature_files:
    - features/<slug>.feature
```

Rejected alternatives:

- Embedding scenario text directly in YAML: rejected because the product spec
  requires external Gherkin-aware tooling compatibility.
- Absolute paths: rejected because intent artifacts should remain portable
  inside the workspace.
- A Cucumber runtime dependency: rejected for v1 because scenarios are
  specification/evidence artifacts, not executable tests.

### 2. Add `intent::bdd`

Create `src/intent/bdd.rs` with explicit types:

- `BddReadinessReport`
  - `readiness: BddReadiness`
  - `issues: Vec<BddIssue>`
  - `feature_files: Vec<BddFeatureFile>`
  - `scenario_count: usize`
  - `coverage: Vec<BddScenarioCoverage>`
- `BddReadiness`
  - `Ready`
  - `Warning`
  - `Blocked`
- `BddIssue`
  - stable `code`
  - severity using `IntentPreflightSeverity` or a local severity enum converted
    into preflight issues
  - `path`
  - `message`
  - `suggested_fix`
- `BddFeatureFile`
  - resolved path
  - display path
  - feature title
  - scenarios
- `BddScenario`
  - optional rule
  - name
  - ordered steps
  - split `given`, `when`, `then` availability flags
- `BddScenarioCoverage`
  - scenario name
  - classification
  - matched test case names
  - required broader evidence text
- `BddCoverageClassification`
  - `VerifierCovered`
  - `PartiallyCovered`
  - `BroaderEvidenceRequired`
  - `Blocked`

Public functions:

```rust
pub fn default_feature_path(slug: &str) -> PathBuf
pub fn render_default_feature(spec: &IntentSpec, slug: &str) -> String
pub fn load_bdd_report(workspace: &Path, slug: &str, spec: &IntentSpec) -> BddReadinessReport
pub fn render_bdd_report(report: &BddReadinessReport) -> Vec<String>
pub fn render_bdd_prompt_context(report: &BddReadinessReport, limit: usize) -> String
pub fn preflight_issues_for_bdd(report: &BddReadinessReport) -> Vec<IntentPreflightIssue>
```

If a helper does not need the slug, omit it, but avoid deriving default paths
without the slug at create time.

### 3. Path Resolution And Safety

Rules:

- Reject absolute paths.
- Reject paths containing `..`.
- Reject paths with non-UTF-8 components.
- Treat files that cannot be read as UTF-8 text as blocking BDD errors.
- Allow intent-relative `features/*.feature` paths.
- Allow workspace-relative `.duumbi/intents/<slug>/features/*.feature` paths.
- Reject paths outside `.duumbi/intents/` in v1.
- Require `.feature` extension for referenced scenario files.

Return BDD issues rather than panicking. Do not use `.unwrap()` in library code.

### 4. Lightweight Gherkin Parser

Implement a deterministic line parser that supports:

- `Feature:`
- `Rule:`
- `Scenario:`
- `Given`
- `When`
- `Then`
- `And`
- `But`
- comments beginning with `#`
- tags beginning with `@`, preserved only as text or ignored for v1 logic

Parser behavior:

- A file with no `Feature:` is blocked.
- A file with no `Scenario:` is blocked.
- A scenario with no effective `Given`, no effective `When`, or no effective
  `Then` is blocked.
- `And` and `But` inherit the most recent `Given`/`When`/`Then` category for
  structure validation.
- Unknown non-empty lines become warning issues, not parser panics.
- Scenario outlines and examples are not interpreted in v1. If present, render
  a warning that they are preserved for external tools but not used for DUUMBI
  coverage mapping.

### 5. Create-Flow Generation

Extend `INTENT_SYSTEM_PROMPT` so the LLM returns one JSON object containing the
existing fields plus:

```json
{
  "bdd_feature": "Feature: ...\n\n  Scenario: ..."
}
```

Do not ask the model to choose file paths. DUUMBI owns path selection.

Update `parse_llm_response` or add a sibling parser that returns:

```rust
struct GeneratedIntentArtifacts {
    spec: IntentSpec,
    bdd_feature: Option<String>,
}
```

Then:

1. Generate the `IntentSpec`.
2. Compute slug with existing `slugify` / `unique_slug`.
3. Set `spec.bdd.feature_files = vec!["features/<slug>.feature"]`.
4. Validate the LLM-provided feature text with `intent::bdd`.
5. If the model omitted or produced unusable BDD, derive a deterministic
   default feature from `spec.intent`, `acceptance_criteria`, and `test_cases`.
6. Preview intent detail plus BDD readiness before save confirmation.
7. On confirmation, create `.duumbi/intents/<slug>/features/`, write the
   feature file, then save the YAML.

Atomicity guidance:

- Use a small helper such as `save_intent_with_bdd(workspace, slug, spec,
  feature_texts)` so YAML and feature files are saved by one path.
- If writing the feature file succeeds but YAML save fails, report the partial
  state explicitly and return an error.
- Prefer writing feature files before YAML so a successful YAML reference does
  not point at a missing file.
- Do not claim the intent is saved until all intended artifacts are written.

Known benchmark fallback:

- Add deterministic feature generators for the existing benchmark specs where
  practical.
- A generic fallback feature is acceptable when there is no known benchmark:
  one `Feature` with one scenario per acceptance criterion or grouped function,
  using test cases as observable examples.

### 6. Review And Preflight Integration

`intent review` should render:

- BDD artifact paths.
- Feature title(s).
- Scenario names.
- Scenario count.
- readiness status.
- coverage summary.
- blocking BDD issues.

Integrate BDD with preflight by:

- Adding warning issue `W_BDD_MISSING` when `spec.bdd.feature_files` is empty.
- Adding error issues for explicit broken references:
  - `E_BDD_PATH_UNSAFE`
  - `E_BDD_FILE_MISSING`
  - `E_BDD_FILE_UNREADABLE`
  - `E_BDD_FILE_EMPTY`
  - `E_BDD_NO_FEATURE`
  - `E_BDD_NO_SCENARIOS`
  - `E_BDD_SCENARIO_INCOMPLETE`
- Adding warning issue `W_BDD_UNKNOWN_LINE` or
  `W_BDD_SCENARIO_OUTLINE_UNMAPPED` for preserved but unmapped Gherkin syntax.

Use the existing preflight block behavior: any BDD error blocks execute before
side effects. Missing BDD on a legacy intent warns but does not block.

### 7. Execute Prompt And Evidence Integration

Load the BDD report once immediately after loading the intent and before
preflight side effects. Implementation can either:

- have `run_preflight` include BDD issues and separately call
  `load_bdd_report` for prompt context, or
- return an augmented preflight object that carries BDD report data.

Keep the implementation simple: a separate `bdd_report` variable in
`run_execute_with_progress` is acceptable.

Before `coordinator::decompose` builds the execution task plan:

- Provide the coordinator a bounded BDD decomposition context derived from
  `render_bdd_prompt_context(&report, limit)` or an equivalent structured
  summary.
- The summary must include feature names, scenario names, and the first bounded
  Given/When/Then steps for ready scenarios.
- Decomposition must be able to create or adjust tasks for scenario-only
  behavior that is not obvious from `acceptance_criteria` or verifier
  `test_cases`.
- The displayed execution plan should reflect materially distinct BDD-driven
  tasks when scenarios add behavior beyond the YAML intent fields.
- If BDD context is summarized because of size, the execution log should say
  the decomposition context was summarized.
- Do not make the coordinator parse `.feature` text directly; pass pre-parsed,
  bounded BDD data from the BDD helper layer.

For task prompts:

- Append a bounded section after acceptance criteria:

```text
BDD scenario contract:
- Feature: ...
- Scenario: ...
  Given ...
  When ...
  Then ...
Coverage: verifier tests [...]
```

- Use `render_bdd_prompt_context(&report, limit)` to avoid unbounded prompt
  growth.
- If the feature file is oversized, include feature/scenario names plus the
  first bounded steps and log that the context was summarized.

For repair prompts:

- Include the failing verifier details as today.
- Add scenario context for scenarios mapped to failed test names/functions.
- If no scenario maps directly, include the highest-level BDD summary and say
  no direct scenario-to-test match was inferred.

Execution evidence:

- Log loaded feature files.
- Log scenario count.
- Log coverage classification counts.
- Log scenarios requiring broader evidence.
- Keep logs text-readable and bounded.

### 8. Studio And REPL/TUI

REPL/TUI:

- Use existing output buffer.
- Do not add a modal.
- Ensure blocked BDD preflight uses the same `Esc`/focus behavior as current
  preflight output.
- Add focused state/output tests only if existing tests cover the relevant path.

Studio:

- Add BDD detail rendering to `render_intent_detail_html`.
- Add BDD/preflight lines to create and execute API responses through existing
  `log` and `preflight` fields when possible.
- If structured UI needs a separate field, use additive fields only.
- Long feature bodies should be summarized in `<pre>` or a compact list; do not
  dump huge files into the page by default.

### 9. Documentation

Add a concise docs/help update that explains:

- where BDD files are stored
- how to edit them
- what missing legacy BDD means
- what broken explicit BDD references mean
- that DUUMBI does not require Cucumber in v1
- that verifier tests and scenario evidence are different proof layers

## Invariants

- Legacy `IntentSpec` YAML without `bdd` remains deserializable.
- `IntentSpec.version` remains `1` unless implementation discovers a breaking
  schema requirement and stops for approval.
- BDD readiness never mutates `.duumbi/graph`.
- BDD readiness never calls a provider during execute.
- A BDD-blocked execute cannot change intent status, save YAML, create a graph
  snapshot, call a provider, write graph files, record learning, or archive the
  intent.
- Missing BDD on a legacy intent is warning-only.
- Explicit broken BDD references block execute.
- Unreadable or non-UTF-8 linked BDD files block execute with
  `E_BDD_FILE_UNREADABLE`.
- BDD scenario files are plain UTF-8 `.feature` text.
- BDD scenarios are not accepted as proof without mapped verifier, graph, build,
  run, manual, or review evidence.
- No Cucumber or Gherkin execution runtime is required in v1.
- Query mode remains read-only.
- Provider setup UX remains `/provider`-centered; BDD work must not add
  unrelated provider warnings.
- Stage 10 must leave the #673 execution issue open until the formal closure
  gate.

## BDD-To-Test Mapping

| Product BDD scenario | Required technical evidence |
|---|---|
| Create an executable intent with a linked feature file | Unit/integration test for create flow saves `.duumbi/intents/<slug>.yaml`, writes `.duumbi/intents/<slug>/features/<slug>.feature`, sets `bdd.feature_files`, and logs path/scenario count/coverage. |
| Save cancellation leaves no misleading BDD completion evidence | CLI create test or focused helper test proving cancellation returns an error/log without reporting a saved slug or completed BDD artifact set. |
| Review an edited feature file | Review test writes an intent and feature file, edits the feature file, runs detail formatting, and asserts current scenario names/readiness/coverage are rendered. |
| Review a legacy intent without BDD | Legacy YAML deserialization and review-format test asserting load succeeds and `W_BDD_MISSING` or equivalent warning appears without blocking review. |
| Execute with a missing linked feature file | Execute preflight side-effect test asserting blocked result, unchanged status, no snapshot, no provider call, no graph write, and no learning/archive evidence. |
| Execute with an unreadable or non-UTF-8 linked feature file | BDD read/preflight test asserting `E_BDD_FILE_UNREADABLE`, blocked execute, and no mutation side effects. |
| Execute a legacy intent without BDD | Execute preflight test with otherwise valid legacy spec asserting warning output and continuation through provider-backed path when a mock provider is supplied. |
| Execute with scenario-only behavior | Coordinator/decomposition test asserting bounded BDD summary can create or adjust task plan entries before mutation starts and the displayed plan reflects the scenario-driven behavior. |
| Builder prompts include relevant scenario context | Unit test for prompt construction or execute prompt helper asserting Given/When/Then context appears alongside acceptance criteria and verifier tests. |
| Repair prompts include failing scenario context | Unit test for repair prompt helper mapping failed test/function names to relevant scenario context. |
| Scenario is fully covered by verifier tests | Coverage classifier test using a scenario with callable i64 behavior and matching `TestCase` names/functions; expects `VerifierCovered` and named test cases. |
| Scenario requires broader evidence | Coverage classifier test with visible output or workflow wording; expects `BroaderEvidenceRequired` and no false claim of verifier coverage. |
| Inspect the scenario artifact with external tools | File-content test or review evidence confirming saved file uses standard `Feature`, `Scenario`, `Given`, `When`, `Then` syntax and `.feature` extension. |
| No Cucumber runtime is required | Dependency review plus tests proving create/review/execute behavior works through DUUMBI parser and existing verifier without adding or invoking Cucumber. |

## Live E2E Plan

Canonical interface: CLI.

Why live provider-backed E2E is required:

- The feature changes LLM-backed `intent create` output and graph mutation prompt
  context during `intent execute`.
- Unit tests with mock providers prove deterministic behavior, but at least one
  live create/execute path should prove the product flow with a real supported
  provider.

Default live path:

```text
duumbi intent create "Build a calculator with add, subtract, multiply, and divide functions that work on i64 numbers" -y
duumbi intent review <generated-slug>
duumbi intent execute <generated-slug>
duumbi build
duumbi run
```

Required credentials:

- Any configured supported direct provider available through the normal
  `/provider` or provider config path.
- Prefer the same provider used by existing Phase 15 smoke paths.
- Do not commit credentials, provider payloads, or raw provider responses.

Expected external LLM calls:

- 1 call for `intent create`.
- 1-6 calls for `intent execute`, depending on task count and repair needs.
- Total expected: 2-7 calls.

Estimated external LLM cost:

- Expected under USD 2 for one calculator-style sample.
- If planned live validation exceeds USD 2 or 10 calls, stop for human approval.

Pass criteria:

- Created intent YAML includes `bdd.feature_files`.
- Feature file exists and is readable plain text.
- `intent review` shows BDD readiness and coverage.
- `intent execute` logs BDD scenario context/evidence and completes or fails
  with diagnostically useful evidence.
- Existing verifier tests still run.
- Build/run behavior is no worse than the existing accepted sample behavior.

Studio/TUI:

- Full provider-backed Studio E2E is not required because the feature is not a
  new Studio-only behavior.
- Require thin parity checks:
  - REPL/TUI create/review displays BDD evidence in the output buffer.
  - Studio intent detail renders BDD/preflight evidence for the same saved
    intent.
  - Studio execute still calls the shared backend and blocks on broken BDD
    before provider construction.

## Ralph Cycle Protocol

Each Stage 10 implementation cycle must:

1. summarize current state and remaining unmet #673 requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate external LLM calls, expected cost, and operational risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 5 source/test/docs files unless the cycle is a
  mechanical struct-literal update required by the `IntentSpec` field addition.
- Expected command budget per cycle:
  - `cargo fmt --check`
  - focused `cargo test` targets for changed modules
  - broader `cargo test --all` before implementation PR review readiness
  - `cargo clippy --all-targets -- -D warnings` before final implementation
    readiness when source code changed
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies or irreversible
  operations, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external model or
  agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 3 low-budget implementation cycles.
- When to stop and ask for human guidance:
  - a Cucumber/Gherkin runtime appears necessary
  - non-i64 verifier payload support appears necessary
  - schema version bump appears necessary
  - BDD paths outside `.duumbi/intents/` appear necessary
  - create/execute behavior would silently degrade from the product spec
  - live provider cost exceeds thresholds
  - tests show blocked BDD can still mutate graph/status/snapshots

## Task Breakdown

1. Add schema and serialization support.
   - Add `IntentBdd`.
   - Add defaulted `bdd` field to `IntentSpec`.
   - Update struct literals and legacy YAML tests.

2. Add `intent::bdd` report foundation.
   - Implement path resolution and safety.
   - Implement lightweight parser.
   - Implement readiness report and rendering.
   - Implement prompt context summary.

3. Add coverage classification.
   - Map scenarios to `TestCase` names/functions/step text.
   - Identify broader-evidence scenarios conservatively.
   - Add unit tests for full, partial, broader, and blocked classifications.

4. Integrate create flow.
   - Extend prompt/parse result.
   - Add deterministic fallback feature generation.
   - Save feature file plus YAML.
   - Render preview/save logs.

5. Integrate review and preflight.
   - Add BDD warning/error issues into preflight.
   - Render BDD readiness in CLI/REPL/Studio detail.
   - Keep legacy warning-only behavior.

6. Integrate execute prompts and evidence.
   - Load BDD report before side effects.
   - Block on explicit broken references.
   - Pass bounded BDD summary into task decomposition before mutation starts.
   - Add BDD prompt context to task and repair prompts.
   - Log BDD evidence after execute setup.

7. Add Studio and TUI parity.
   - Update Studio detail and API response handling.
   - Add focused tests for bounded evidence rendering.
   - Confirm TUI output buffer behavior.

8. Add docs/help updates.
   - Document artifact layout, editing, warnings, blocking states, and no
     Cucumber requirement.

9. Run final validation.
   - Focused tests.
   - `cargo test --all`.
   - `cargo clippy --all-targets -- -D warnings`.
   - One approved live provider-backed CLI sample.

## Verification Plan

Focused unit tests:

- `IntentSpec` legacy YAML without `bdd` parses.
- `IntentSpec` with `bdd.feature_files` round-trips.
- unsafe BDD paths are rejected.
- missing feature file creates blocking BDD issue.
- unreadable or non-UTF-8 feature file creates blocking BDD issue.
- empty feature file creates blocking BDD issue.
- no `Feature:` creates blocking BDD issue.
- no `Scenario:` creates blocking BDD issue.
- scenario without Given/When/Then creates blocking BDD issue.
- `And`/`But` inherit the previous step kind.
- unknown Gherkin lines warn but do not panic.
- scenario outline/examples warn as unmapped in v1.
- default feature generation creates standard Feature/Scenario/Given/When/Then
  text.
- coverage classifier reports verifier-covered scenario when matching tests
  exist.
- coverage classifier reports broader evidence for visible output/workflow
  scenarios.
- prompt context renderer is bounded and includes scenario names and steps.

Focused integration tests:

- create flow saves YAML and `.feature` file together.
- create cancellation does not report saved artifacts.
- `intent create -y` logs feature path and scenario count.
- known benchmark fallback produces deterministic BDD where practical.
- review of edited feature file uses current file contents.
- review of legacy intent warns but loads.
- execute with missing linked feature blocks before provider construction.
- blocked BDD execute leaves status unchanged.
- blocked BDD execute creates no snapshot.
- blocked BDD execute writes no graph files.
- blocked BDD execute appends no learning/archive evidence.
- legacy no-BDD execute warning path remains non-blocking.
- task decomposition receives bounded BDD context before mutation starts.
- displayed execution plan reflects materially distinct BDD-driven tasks.
- task prompt helper includes BDD context.
- repair prompt helper includes relevant scenario context.
- Studio detail HTML includes BDD evidence.
- Studio execute API reports blocked BDD before provider setup.

Command checks:

- `cargo fmt --check`
- focused `cargo test` commands for `intent::bdd`, `intent::spec`,
  `intent::create`, `intent::review`, `intent::execute`, `workflow`, and Studio
  rendering/tests as affected
- `cargo test --all`
- `cargo clippy --all-targets -- -D warnings`

Manual checks:

- CLI create/review/execute with one calculator-style live provider path.
- Delete the linked `.feature` file and verify execute blocks before provider
  setup.
- Review a legacy intent without BDD and verify warning-only behavior.
- Inspect the saved `.feature` in a text editor or Gherkin-aware editor.
- REPL/TUI create/review smoke for bounded BDD output.
- Studio detail/execute smoke for shared-backend BDD evidence.

## Completion Criteria

- `IntentSpec` has a backward-compatible BDD reference model.
- New executable create flows save linked `.feature` files by default.
- Legacy intents without BDD remain loadable.
- Explicit broken BDD references block execute before mutation side effects.
- Review and execute recompute BDD readiness from current files.
- Scenario coverage is visible and conservative.
- Task and repair prompts include bounded scenario context.
- CLI, REPL/TUI, Studio, and shared workflow surfaces expose BDD evidence.
- Existing verifier tests still run and remain authoritative only for i64
  callable behavior.
- No Cucumber runtime is required.
- Documentation explains artifact layout and user expectations.
- All required tests/checks and the approved live E2E evidence are attached to
  the implementation PR.

## Failure And Escalation

- If schema changes cannot remain backward-compatible, stop for human approval.
- If BDD parsing requires a full Gherkin runtime or new dependency, stop for
  human approval.
- If external paths outside `.duumbi/intents/` appear necessary, stop for a
  product/security decision.
- If live provider calls exceed USD 2 or 10 calls, stop for approval.
- If blocked BDD can still mutate status, graph files, snapshots, learning, or
  archives, treat as blocking P0/P1 implementation failure.
- If reviewer feedback requests non-i64 verifier payloads, route that as a
  follow-up unless the human explicitly expands #673 scope.
- If Studio or TUI output cannot stay bounded, stop and narrow the renderer
  before adding more UI behavior.

## Open Questions

No blocking open questions for implementation.

Accepted Stage 8 decisions:

- YAML shape: `bdd.feature_files`.
- Default artifact path: `.duumbi/intents/<slug>/features/<slug>.feature`.
- Multiple files: supported by schema and parser, but generated create output
  defaults to one feature file.
- User-removed BDD references: warning-only for v1 when other preflight checks
  pass.
- Prompt-budget behavior: summarize after a bounded number of scenarios/steps;
  Stage 10 should choose a concrete default such as 5 scenarios and 40 rendered
  lines, then test it.
