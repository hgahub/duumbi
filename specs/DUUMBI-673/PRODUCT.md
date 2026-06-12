# DUUMBI-673: BDD Specification Artifacts For Runtime Intents

## Summary

Add first-class BDD/Gherkin scenario artifacts to DUUMBI's runtime intent
workflow.

When a user creates an executable intent, DUUMBI should help produce a linked
`.feature` companion artifact that describes the intended behavior in
Gherkin-style scenarios. The intent specification should reference those
scenario files, review surfaces should show their readiness and coverage, and
`intent execute` should use them as scenario context for builder and tester
agents before graph mutation starts.

This is product behavior for DUUMBI users building applications with DUUMBI. It
does not change the internal Stage 6/8 BDD workflow used to develop DUUMBI
itself.

The execution issue must remain open after this spec PR. This PR is
specification-only and is related to #673; it is not completion evidence for the
execution work.

## Problem

DUUMBI already turns a natural-language request into an `IntentSpec` containing
acceptance criteria, target modules, i64 verifier test cases, dependencies, and
optional context. The completed preflight gate improves quality by detecting
weak or unsafe specs before mutation. That still leaves a gap between product
behavior and executable test cases.

The current verifier test model is intentionally narrow: each test case calls a
function with i64 arguments and compares an i64 return value. This is useful for
compiler and graph correctness, but it cannot express many behavior contracts a
user naturally cares about:

- the user context and visible outcome for a behavior
- state transitions and invalid inputs
- command, TUI, Studio, or generated application behavior
- output formatting and demonstration expectations
- scenarios that require graph validation, build evidence, manual inspection,
  or future E2E evidence instead of a single callable i64 test

Without a scenario artifact, builder agents mostly see a list of criteria and
test cases. Tester agents can prove the callable checks, but they have no
durable, user-readable scenario contract that maps higher-level behavior to
evidence. This increases ambiguity, makes review harder, and allows the intent,
mutation prompt, verifier tests, and final behavior to drift.

## Outcome

When this is implemented:

- New executable `intent create` flows produce or help author at least one
  linked Gherkin `.feature` artifact by default.
- Scenario artifacts are stored beside the runtime intent artifacts and are
  referenced from `IntentSpec` in a backward-compatible way.
- Users can inspect and edit the scenario files with ordinary text editors and
  external Gherkin-aware tools.
- CLI, REPL/TUI, Studio, and shared workflow surfaces show the linked scenario
  artifact paths, scenario count, readiness state, and coverage summary.
- `intent review` recomputes scenario readiness and coverage from current
  files, not stale generated text.
- `intent execute` loads linked scenarios before graph mutation side effects
  and passes relevant scenario context into task decomposition, mutation
  prompts, repair prompts, and tester evidence.
- Broken linked scenario references block execution before status changes,
  snapshots, provider calls, graph writes, learning records, or archive writes.
- Existing intents without BDD artifacts remain loadable and executable; they
  receive a visible warning rather than a schema-migration failure.
- BDD scenarios supplement acceptance criteria and verifier tests. They do not
  replace graph validation, compiler checks, runtime execution, preflight, or
  existing verifier test cases.
- Stage 8 and later review evidence can map each scenario to i64 verifier
  tests, graph validation, build/run evidence, manual checks, or E2E evidence.

## Scope

### In Scope

- Define BDD/Gherkin companion artifacts for runtime DUUMBI intents.
- Store linked `.feature` files under the intent artifact area, with paths that
  are stable and inspectable by users.
- Add a backward-compatible `IntentSpec` reference to one or more scenario
  files.
- Update `intent create` so generated executable intents include scenario
  artifacts by default, with preview and confirmation behavior that keeps the
  YAML and feature files together.
- Update `intent create -y` and shared workflow create paths to log the saved
  scenario artifact paths.
- Update `/intent create`, `/intent review`, Studio intent creation, and Studio
  review surfaces to expose scenario readiness and coverage in bounded text.
- Validate linked scenario files during preflight and review.
- Add scenario context to `intent execute` before coordinator decomposition and
  LLM graph mutation.
- Include scenario context in repair prompts when verifier failures happen.
- Produce execution evidence that summarizes which scenarios were covered by
  verifier tests and which require other evidence.
- Preserve old `IntentSpec` YAML compatibility when no BDD reference exists.
- Document the runtime BDD intent workflow where users need to understand the
  artifact layout and review expectations.

### Explicitly Out Of Scope

- Technical specification content or implementation instructions during Stage
  6.
- Implementation code, tests, docs changes, generated `.feature` files, or
  migration scripts during Stage 6.
- Ralph cycles or implementation coordination.
- Changing DUUMBI's internal GitHub issue-to-spec development workflow.
- Requiring Cucumber, a Gherkin runner, or a new BDD execution framework in v1.
- Treating `.feature` files as executable tests by themselves.
- Removing or weakening existing i64 verifier test cases.
- Replacing deterministic preflight, graph validation, compilation, runtime
  execution, repair, rollback, or learning records.
- Making BDD artifacts mandatory for every legacy intent before execution.
- Adding non-i64 verifier test payloads as part of this issue.
- Automatically accepting behavior because a scenario exists.

## Constraints And Assumptions

Facts:

- Issue #673 is open.
- Issue #673 is labeled `accepted` and `needs-spec`.
- The Stage 5 decision comment on 2026-06-12 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- The issue body asks DUUMBI to add first-class BDD/Gherkin artifacts to
  runtime intent creation and execution.
- The original intake note clarifies that the feature is about DUUMBI users
  building applications with DUUMBI, not about DUUMBI's own internal
  development workflow.
- `IntentSpec` currently stores intent text, version, status, acceptance
  criteria, create/modify module lists, i64 test cases, dependencies, optional
  context, creation time, and optional execution metadata.
- `IntentSpec` currently has no BDD scenario field and no reference to external
  BDD/Gherkin artifacts.
- `intent create` currently asks a provider to generate acceptance criteria,
  modules, i64 test cases, and dependencies, then saves a YAML intent.
- `intent create` already has a known benchmark fallback for some common
  requests.
- `intent execute` currently runs deterministic preflight before side effects,
  then marks the intent in progress, snapshots graph state, decomposes tasks,
  enriches mutation context, calls the provider, writes graph files, runs
  verifier tests, and may attempt repair.
- The current verifier test case model uses callable function names, i64
  arguments, and i64 expected return values.
- DUUMBI-553 added deterministic preflight readiness checks over the existing
  intent shape without persisting preflight results into YAML.
- The active PRD describes DUUMBI as intent-first, evidence-oriented, and
  human-verifiable.

Assumptions:

- Scenario artifacts will improve execution quality only if users can inspect
  them and execution evidence maps scenarios to proof. Hidden generated text is
  not enough.
- External `.feature` files are more useful for user inspection and tooling than
  embedding loose scenario text only inside YAML.
- Backward compatibility matters because users may already have active
  `.duumbi/intents/*.yaml` files without BDD references.
- It is acceptable for v1 to use Gherkin as a specification language without
  adding a Cucumber runtime.
- Broken explicit scenario references are stronger evidence of workspace
  inconsistency than an absent legacy BDD reference, so broken references should
  block before mutation while absent legacy BDD should warn.
- The exact richness of scenario-to-test mapping can improve over time, but v1
  needs a visible mapping contract so Stage 8 and Stage 11 can verify coverage.

Constraints:

- Scenario validation must run before `intent execute` side effects when
  scenarios are referenced.
- A blocked scenario preflight must leave intent status, graph files,
  snapshots, learning records, and archives unchanged.
- Runtime BDD support must not introduce startup provider warnings during
  unrelated flows.
- Query mode remains read-only.
- Scenario artifacts must be plain text and usable outside DUUMBI.
- User-facing output must stay bounded in CLI, REPL/TUI, Studio, and shared
  workflow logs.
- Color cannot be the only indicator of scenario readiness or coverage.

## Decisions

- **Decision:** #673 should use a file-based product spec.
  **Evidence:** The feature is user-visible, cross-module, changes runtime
  intent storage and execution behavior, and will need durable review context
  for Stage 7, Stage 8, Stage 10, and Stage 11.

- **Decision:** v1 uses linked `.feature` companion artifacts rather than only
  embedded scenario text.
  **Evidence:** The issue and intake note ask for Gherkin-aware external tool
  compatibility and storage beside the intent spec by reference.

- **Decision:** New executable intents should get BDD artifacts by default, but
  legacy intents without BDD remain executable with warnings.
  **Evidence:** The product goal is to improve new workflow quality without
  turning a schema addition into a migration blocker for existing workspaces.

- **Decision:** A linked scenario file that is missing, unreadable, or structurally
  unusable blocks `intent execute` before mutation side effects.
  **Evidence:** Once the intent references BDD artifacts, agents and reviewers
  should not proceed from stale or absent behavior contracts.

- **Decision:** BDD scenarios guide agents and evidence mapping; they are not a
  new executable test framework in v1.
  **Evidence:** The current verifier has a narrow i64 function model. Adding a
  full Gherkin runtime would be a larger accepted behavior change than the issue
  requires.

- **Decision:** Scenario coverage supplements existing verifier tests.
  **Evidence:** The current verifier remains valuable for graph correctness, and
  BDD scenarios express broader behavior that may map to multiple evidence
  types.

- **Decision:** Runtime BDD artifacts are separate from DUUMBI's internal
  development BDD specs.
  **Evidence:** The original intake explicitly separated product use of DUUMBI
  from the process used to develop DUUMBI itself.

## Behavior

### Artifact Model

- Existing intent YAML files remain under `.duumbi/intents/<slug>.yaml`.
- The default v1 companion artifact area is under the same intent namespace,
  using a stable per-intent feature directory:

```text
.duumbi/intents/<slug>.yaml
.duumbi/intents/<slug>/features/<slug>.feature
```

- `IntentSpec` references scenario files with workspace-stable relative paths or
  intent-relative paths. The reference format must be backward-compatible so
  older YAML files without BDD references still deserialize.
- DUUMBI may support more than one `.feature` file per intent when the user
  splits behavior by feature, rule, or surface.
- Scenario files are plain UTF-8 text and use `.feature` extension.
- Generated scenario files must use standard English Gherkin keywords:
  `Feature`, optional `Rule`, `Scenario`, `Given`, `When`, `Then`, `And`, and
  `But`.
- Tags and comments may be preserved for external tooling, but v1 DUUMBI
  behavior must not depend on a full Cucumber tag-expression engine.

### Intent Create

- For executable graph-changing intents, `intent create` asks the provider for
  both the structured `IntentSpec` content and BDD scenarios, or derives
  scenarios from generated criteria and tests when provider output is partial.
- Known benchmark fallback paths should produce deterministic companion
  scenarios for the fallback spec when practical.
- The interactive preview shows:
  - intent YAML summary
  - feature file path
  - scenario count
  - scenario readiness state
  - scenario-to-test coverage summary
- On confirmation, DUUMBI saves the intent YAML and scenario files together.
- If a save fails after any file is written, DUUMBI reports the partial state
  clearly and must not imply that the intent artifact set is complete.
- `intent create -y` saves without interactive confirmation but still logs the
  feature file path and coverage summary.
- Users can edit the `.feature` file after create; review and execute recompute
  readiness from the current file.

### Review Behavior

- `intent review <slug>` shows linked BDD artifacts when present.
- Review output includes bounded scenario details:
  - feature title
  - scenario names
  - missing Given/When/Then structure when detected
  - scenario coverage status
  - linked verifier test names when inferable
- Review output does not dump entire large feature files by default.
- Missing BDD on a legacy intent is a warning, not a load failure.
- Broken explicit BDD references are displayed as blocking readiness findings.

### Execute Behavior

- `intent execute <slug>` loads and validates linked BDD artifacts immediately
  after loading the intent and before:
  - changing intent status
  - saving the intent
  - taking a graph snapshot
  - calling `coordinator::decompose`
  - assembling mutation context
  - calling a provider
  - writing graph files
  - recording learning evidence
  - archiving the intent
- If a referenced feature file is missing, unreadable, empty, or has no usable
  `Feature` and `Scenario` structure, execution blocks with a BDD-specific
  report.
- If no BDD reference exists on a legacy intent, execution may continue with a
  warning that scenario guidance is unavailable.
- The coordinator and mutation prompts receive a concise scenario contract:
  feature title, scenario names, Given/When/Then steps, and mapped verifier test
  names when available.
- Create-module, modify-module, modify-main, and repair prompts all preserve
  the relevant scenario context instead of relying only on acceptance criteria.
- Scenario context must stay bounded; DUUMBI should summarize or select relevant
  scenarios when a feature file is too large for the prompt budget.

### Coverage And Evidence

- Each scenario receives a coverage classification during review and execute:
  - covered by one or more existing i64 verifier test cases
  - partially covered by verifier tests and needing broader evidence
  - not covered by verifier tests and requiring graph/build/run/manual/E2E
    evidence
  - blocked because the scenario is structurally unusable
- Coverage mapping may use scenario names, function names, acceptance criteria,
  test names, and explicit references where available.
- A passing verifier test does not automatically prove a whole scenario unless
  the mapping is explicit and the scenario's observable outcome is within i64
  verifier capability.
- Execution evidence should summarize:
  - loaded feature files
  - scenario count
  - scenario coverage classifications
  - verifier tests run
  - scenarios still requiring non-verifier evidence
- Stage 8 technical specs must map approved BDD scenarios to tests, CI, manual
  checks, or review evidence.
- Stage 11 review must verify that implementation evidence accounts for every
  approved scenario.

### Empty And Error States

- No linked BDD reference on an old intent: warn and continue.
- No linked BDD reference after a user deliberately removes the reference from a
  BDD-aware intent: warn during review and execute as a legacy-compatible path
  when other preflight checks pass.
- A BDD-aware create flow must not silently save an executable intent while
  claiming BDD artifacts exist. It either saves linked scenarios, reports that
  BDD generation was unavailable, or fails clearly.
- Linked path missing: block execute.
- Linked path points outside the workspace or intent artifact area: block
  execute unless the path is an explicitly supported safe relative path.
- Feature file exists but has no scenarios: block execute.
- Scenario exists without a `Given`, `When`, or `Then` step: block execute for
  that referenced BDD artifact because agents cannot safely consume it as a
  behavior contract.
- Provider cannot generate scenarios during create: show a clear create failure
  or fallback path; do not silently save an intent while claiming BDD exists.
- User deletes or edits a feature file between review and execute: execute uses
  the current file state and recomputes readiness.

### Accessibility And UI Rules

- CLI output is readable without color.
- REPL/TUI output stays bounded and scrollable; BDD readiness should not open a
  modal that traps focus.
- `Esc` behavior in the TUI remains consistent with existing active-panel
  rules.
- Studio surfaces expose text labels for feature path, readiness, scenario
  count, coverage status, and blocking findings.
- Long scenario bodies are collapsed or summarized by default in bounded
  surfaces.

### Invariants

- BDD preflight never mutates `.duumbi/graph`.
- BDD preflight never calls a provider during execute.
- BDD preflight never changes intent status by itself.
- A blocked BDD readiness result cannot create a snapshot, graph write, learning
  record, or archive write.
- Scenario artifacts are not accepted as proof without matching execution or
  review evidence.
- Existing valid intent YAML without BDD references remains deserializable.
- Query mode remains read-only.

## BDD Scenarios

```gherkin
Feature: Runtime intent BDD companion artifacts

  Rule: New executable intents receive inspectable scenario artifacts

    Scenario: Create an executable intent with a linked feature file
      Given a user has a DUUMBI workspace
      And the user requests an executable graph-changing intent
      When the user runs intent create and accepts the generated artifact preview
      Then DUUMBI saves the IntentSpec YAML
      And DUUMBI saves a linked .feature file under the intent artifact area
      And the IntentSpec references the feature file
      And the create log shows the feature path, scenario count, and coverage summary

    Scenario: Save cancellation leaves no misleading BDD completion evidence
      Given a generated intent preview includes BDD scenarios
      When the user cancels before saving
      Then DUUMBI does not report an intent slug as saved
      And DUUMBI does not claim that linked BDD artifacts are complete

  Rule: Review surfaces recompute BDD readiness from current files

    Scenario: Review an edited feature file
      Given an existing intent references a feature file
      And the user edits the feature file outside DUUMBI
      When the user runs intent review for that intent
      Then DUUMBI reads the current feature file
      And DUUMBI shows scenario names, readiness, and coverage based on current content

    Scenario: Review a legacy intent without BDD
      Given an existing intent YAML has no BDD reference
      When the user runs intent review
      Then DUUMBI loads the intent successfully
      And DUUMBI shows a warning that BDD scenario guidance is unavailable
      But DUUMBI does not require a schema migration before review

  Rule: Execute blocks on broken linked BDD artifacts before mutation side effects

    Scenario: Execute with a missing linked feature file
      Given an intent references a feature file
      And the referenced file is missing
      When the user runs intent execute
      Then DUUMBI blocks execution with a BDD-specific readiness report
      And the intent status remains unchanged
      And no graph snapshot is created
      And no provider call is made
      And no graph file is written
      And no learning or archive record is written

    Scenario: Execute a legacy intent without BDD
      Given an existing intent has no BDD reference
      And the intent otherwise passes deterministic preflight
      When the user runs intent execute
      Then DUUMBI logs that scenario guidance is unavailable
      And DUUMBI continues through the existing execution path

  Rule: Agents use scenarios as behavior contracts, not as standalone tests

    Scenario: Builder prompts include relevant scenario context
      Given an intent has linked BDD scenarios
      And the scenarios are structurally usable
      When DUUMBI decomposes and executes graph mutation tasks
      Then each relevant builder prompt includes a bounded scenario contract
      And the prompt preserves Given, When, and Then behavior expectations
      And the prompt still includes existing acceptance criteria and verifier tests

    Scenario: Repair prompts include failing scenario context
      Given an intent has linked BDD scenarios
      And verifier tests fail after graph mutation
      When DUUMBI prepares a repair prompt
      Then DUUMBI includes the failed verifier details
      And DUUMBI includes the relevant BDD scenario context for the behavior under repair

  Rule: Scenario coverage maps to evidence

    Scenario: Scenario is fully covered by verifier tests
      Given a BDD scenario describes a callable i64 behavior
      And the IntentSpec includes verifier tests that match the scenario outcome
      When DUUMBI reviews or executes the intent
      Then DUUMBI marks the scenario as covered by verifier tests
      And the evidence names the matching test cases

    Scenario: Scenario requires broader evidence
      Given a BDD scenario describes visible output or application workflow behavior
      And no i64 verifier test can prove the whole observable outcome
      When DUUMBI reviews or executes the intent
      Then DUUMBI marks the scenario as requiring broader evidence
      And DUUMBI does not claim the scenario is fully proven by verifier tests alone

  Rule: Gherkin remains an external specification format in v1

    Scenario: Inspect the scenario artifact with external tools
      Given an intent has a linked .feature file
      When the user opens the file in a Gherkin-aware editor
      Then the file uses standard Feature, Scenario, Given, When, and Then syntax
      And no DUUMBI-only binary format is required to inspect the scenarios

    Scenario: No Cucumber runtime is required
      Given an intent has linked BDD scenarios
      When DUUMBI executes the intent
      Then DUUMBI does not require a Cucumber runtime to run
      And DUUMBI still runs graph validation, compilation, and existing verifier tests
```

## Tasks

- Define the runtime BDD artifact contract and default layout.
- Add backward-compatible `IntentSpec` references to linked feature files.
- Add BDD scenario generation or derivation to executable `intent create`
  flows.
- Add artifact preview and save behavior for interactive and `-y` create paths.
- Add scenario readiness parsing and bounded rendering.
- Add scenario-to-verifier coverage classification.
- Integrate BDD readiness into preflight, review, CLI, REPL/TUI, Studio, and
  shared workflow responses.
- Integrate scenario context into `intent execute` before coordinator
  decomposition, mutation prompts, and repair prompts.
- Add execution evidence summary for loaded scenarios and coverage status.
- Update docs or help text for users creating, reviewing, editing, and
  executing intents with BDD artifacts.

Independent work:

- Artifact layout and serialization contract.
- Scenario parser/readiness model.
- Coverage classifier.
- Text renderers for CLI/REPL/Studio logs.
- Documentation of artifact layout and user workflow.

Sequential work:

- Create-flow integration should follow the serialization contract.
- Execute integration should follow readiness parsing because it must enforce
  blocking before side effects.
- Prompt integration should follow bounded scenario summarization.
- Studio/shared workflow behavior should follow CLI behavior to avoid divergent
  evidence.

## Checks

- `cargo fmt --check`
- `cargo test --all`
- `cargo clippy --all-targets -- -D warnings`
- Focused tests for:
  - deserializing legacy `IntentSpec` YAML without BDD references
  - serializing a new `IntentSpec` with linked feature files
  - generated executable intent saves YAML and feature artifacts together
  - `intent create -y` logs feature file path and scenario count
  - known benchmark fallback produces deterministic scenario artifacts when
    practical
  - review warning for legacy intent without BDD
  - review block for missing linked feature file
  - review block for empty feature file
  - review block for scenario missing Given/When/Then structure
  - scenario coverage classified as verifier-covered when test mapping is clear
  - scenario coverage classified as broader evidence when verifier cannot prove
    the observable outcome
  - execute with missing linked feature file leaves status unchanged
  - execute with missing linked feature file does not create a snapshot
  - execute with missing linked feature file does not call a provider
  - execute with missing linked feature file does not write graph files
  - execute with missing linked feature file does not append learning records
  - execute with no BDD on a legacy intent warns and continues when other
    preflight checks pass
  - mutation prompts include bounded relevant scenario context
  - repair prompts include relevant scenario context after verifier failure
  - oversized feature files are summarized or bounded before prompt assembly
  - unsafe or workspace-escaping feature paths are rejected
- Manual checks:
  - Create an executable calculator-style intent and inspect the saved `.feature`
    file in a text editor.
  - Review the created intent and confirm scenario readiness and coverage are
    visible without dumping an oversized file.
  - Execute the created intent and confirm scenario context appears in evidence
    while normal verifier tests still run.
  - Delete the linked feature file and confirm execute blocks before side
    effects.
  - Run a legacy intent without BDD and confirm warning-only behavior.
  - Inspect REPL/TUI output for bounded readable scenario evidence and stable
    `Esc` behavior.
  - Inspect Studio create/review/execute responses for text-readable readiness
    and coverage labels.

BDD coverage expectations:

- Create artifact scenarios are covered by create-flow tests and manual file
  inspection.
- Review scenarios are covered by parser/readiness and review-rendering tests.
- Execute blocking scenarios are covered by preflight side-effect tests.
- Agent-context scenarios are covered by prompt assembly tests.
- Evidence-mapping scenarios are covered by coverage classifier tests and Stage
  11 review evidence.
- External tooling and no-Cucumber scenarios are covered by documentation and
  review of saved plain-text `.feature` artifacts.

## Open Questions

No blocking open questions for v1.

Non-blocking Stage 8 decisions:

- Exact Rust field name and YAML shape for BDD references.
- Whether v1 supports multiple feature files in create output by default or only
  after user edits.
- Whether user-removed BDD references should remain warning-only or become
  stricter after migration and implementation evidence exists.
- The exact prompt-budget threshold for summarizing large scenario files.

Future versions may revisit:

- Scenario outlines and example tables.
- A native Gherkin runner or Cucumber integration.
- Non-i64 verifier payloads.
- Automatic scenario repair.
- Stricter BDD requirements for all executable intents after migration tooling
  exists.

## Sources

- GitHub Issue: https://github.com/hgahub/duumbi/issues/673
- Stage 5 acceptance comment:
  https://github.com/hgahub/duumbi/issues/673#issuecomment-4695365838
- Original processed inbox note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-05-18 - BDD In Intent Workflow.md`
- PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Architecture reference: `docs/architecture.md`
- Prior related spec:
  `specs/DUUMBI-553/PRODUCT.md`
- Relevant source context:
  - `src/intent/spec.rs`
  - `src/intent/create.rs`
  - `src/intent/execute.rs`
  - `src/intent/coordinator.rs`
  - `src/intent/verifier.rs`
  - `src/intent/preflight.rs`
  - `src/workflow.rs`
  - `src/context/mod.rs`
