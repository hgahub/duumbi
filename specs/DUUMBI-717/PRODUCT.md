# DUUMBI-717: Contract-Based Property Test Generation

## Summary

Add a first property-testing assurance tier for DUUMBI functions that declare
contracts. DUUMBI should parse a shared JSON-LD contract vocabulary, derive
deterministic type-driven inputs, run seeded randomized cases against compiled
functions, check postconditions, shrink failures to minimal reproducible
counterexamples, and record reviewable evidence.

Related to #717. This is a product-specification artifact only. The execution
issue must remain open for Stage 8 technical specification, Stage 9 Technical
Spec Review, Stage 10 implementation, Stage 11 implementation review, and Stage
12 closure.

## Problem

DUUMBI's semantic graph already carries typed functions, stable node ids,
validation, ownership checks, runtime evidence, and benchmark evidence. That is
enough to make graph programs more inspectable than text-only code, but it does
not yet prove that a function with an intended behavior keeps that behavior
across generated inputs.

The accepted issue identifies the gap: full SMT-based formal verification is
planned later, but the contract vocabulary needed for that work can provide
value earlier through property-based testing. Without a property-test tier:

- contracts are not represented in a shared machine-readable JSON-LD shape;
- typed DUUMBI functions do not automatically receive randomized input
  coverage;
- failures do not shrink to small counterexamples linked to graph node ids;
- deterministic seed evidence is unavailable for replay, query, describe,
  intent, or future formal verification workflows;
- Stage 10 agents may overfit to a few example tests while missing behavior
  classes that the type system could explore cheaply.

## Outcome

When this work is done:

- DUUMBI has a shared v1 JSON-LD contract vocabulary for preconditions,
  postconditions, invariants, and effect declarations that can be reused by the
  future VCGen work.
- Function-level contracts are parsed and preserved through the typed AST and
  semantic graph.
- `duumbi check --properties` runs deterministic property checks for functions
  with supported contracts.
- Property runs derive input generators from DUUMBI parameter and return types.
- Supported v1 generator coverage includes pure deterministic values for
  `i64`, `f64`, `bool`, `string`, `json`, `array<T>`, `struct<Name>`,
  `option<T>`, and `result<T,E>` when their nested types are also supported and
  bounded.
- Runtime-owned resource and effectful types such as `tcp_socket`,
  `tcp_listener`, `http_server`, `http_response`, `db_connection`, and `db_rows`
  are recognized and reported as unsupported for v1 property execution unless
  a later approved effect model defines safe generation.
- Preconditions constrain generated inputs. Rejected candidates are counted and
  bounded so runs cannot loop forever on overly narrow preconditions.
- Postconditions are checked against each successful function execution.
- A failing run reports the seed, function id, relevant contract id or label,
  failing input values, actual output or failure, and a shrunk minimal
  counterexample when shrink succeeds.
- Property evidence is written as a compact JSON artifact under the local
  `.duumbi` evidence area and is also summarized in command output.
- `duumbi describe` and query-context sources can surface whether a function
  has contracts and recent property evidence without treating missing evidence
  as proof.
- CI can run a deterministic, provider-free stdlib Tier 1 property subset.
- The future `duumbi verify` / VCGen path can consume the same contract fields
  without a vocabulary rewrite.

## Scope

### In Scope

- Define the v1 DUUMBI contract vocabulary on function nodes:
  - preconditions;
  - postconditions;
  - loop/block invariants as accepted metadata, even if property execution only
    reports them as not directly exercised in v1;
  - effect declarations used to decide whether property execution is safe.
- Extend parser, AST, semantic graph metadata, graph description, and validation
  enough to preserve and validate contract metadata.
- Add deterministic type-driven generators for supported pure value types.
- Add bounded generation controls:
  - seed;
  - case count;
  - max generated collection length;
  - max precondition rejections per case;
  - deterministic ordering for function selection and evidence.
- Add a property runner that builds/runs compiled DUUMBI functions and evaluates
  postconditions for supported contracts.
- Add shrinking for failing inputs with a deterministic, type-aware shrink
  order.
- Add local evidence records with schema version, command, seed, function id,
  contract id, generator stats, pass/fail counts, rejected case counts,
  unsupported reason, counterexample, and artifact provenance.
- Add `duumbi check --properties` as the canonical CLI entry.
- Add CI-safe tests and fixtures that prove passing properties, failing
  properties, shrink output, unsupported effectful contracts, and deterministic
  seed replay.
- Annotate a small stdlib Tier 1 subset with contracts. The subset must be
  enough to prove the full path without turning this issue into the whole stdlib
  verification program.
- Document the contract vocabulary and property evidence expectations in the
  source repo.

### Explicitly Out Of Scope

- Full SMT / VCGen formal verification.
- `duumbi verify`, SMT-LIB emission, Z3/CVC5 integration, weakest-precondition
  traversal, or proof status claims.
- Dynamic symbolic execution.
- Production telemetry ingestion or remote evidence upload.
- Random testing of effectful IO/state/resource functions without an approved
  effect model.
- Treating property-test success as formal proof.
- Requiring live LLM calls or provider credentials in normal CI.
- Broad stdlib contract coverage beyond the first deterministic Tier 1 slice.
- A public Studio/TUI property-dashboard redesign. Thin parity is enough unless
  implementation discovers an existing shared backend surface that requires a
  small display update.
- Implementation code, tests, generated evidence, or Ralph cycles during this
  specification stage.

## Constraints And Assumptions

Facts:

- Issue #717 is open and labeled `accepted` and `needs-spec`.
- The Stage 5 Human Acceptance Decision comment dated 2026-06-16 records
  `Decision: Accept`, `Next state: Spec Needed`, and no remaining open
  questions.
- The processed inbox source says property testing is the cheap assurance rung
  between types and future SMT verification.
- The active PRD defines DUUMBI as intent-first, graph-centered,
  evidence-oriented, and human-verifiable.
- `docs/architecture.md` defines the semantic graph as the central IR and the
  semantic fixed point as schema validation, type checking, and tests passing.
- `src/parser/ast.rs` currently parses function params, return type, blocks,
  ops, and lifetime params, but it does not define contract metadata.
- `src/graph/mod.rs` currently stores `FunctionInfo`, `GraphNode`, and graph
  edges, but it does not store function contracts.
- `src/types.rs` currently defines pure values and opaque runtime resource
  types.
- `src/cli/mod.rs` currently has `duumbi check` for parse/validate and
  `duumbi benchmark` for provider-backed evidence; `check --properties` does
  not exist.
- The current checkout already includes scaled benchmark evidence and report
  conventions from #689.
- The default GitHub token in this Codex session cannot read ProjectV2 fields
  without `read:project`; `GH_PROJECT_PAT` is present for possible ProjectV2
  reads/writes.

Assumptions:

- The v1 property runner should be provider-free and deterministic.
- A function is property-testable only when it can be executed with generated
  inputs in a bounded local build/run environment.
- Supported contracts should be evaluated through a small expression/predicate
  vocabulary, not arbitrary provider-generated code.
- Resource and effectful functions need explicit effect models before property
  execution can be meaningful. Reporting them as unsupported is safer than
  producing fake confidence.
- First implementation should reuse existing workspace build/run helpers where
  feasible and add narrow function-call execution helpers only where current
  APIs cannot invoke parameterized functions.
- Property evidence should follow the compact, honest-evidence style used by
  current benchmark and telemetry artifacts.

## Decisions

- **Decision:** Use a source-repo file-based product spec.
  **Evidence:** The issue defines a durable graph contract, CLI behavior,
  evidence schema, stdlib annotations, and future formal-verification boundary.

- **Decision:** The v1 contract vocabulary is shared with future VCGen but does
  not implement VCGen.
  **Evidence:** The source note explicitly says contract fields should be
  defined once and consumed by property tests now and formal verification later.

- **Decision:** Property execution is pure/deterministic in v1.
  **Evidence:** The issue's open question about effects materially affects
  safety. Current DUUMBI resource types are runtime-owned handles, not values
  that can be generated reproducibly without environmental models.

- **Decision:** Unsupported types/effects are first-class evidence, not silent
  skips.
  **Evidence:** DUUMBI's product thesis values honest, reviewable evidence over
  false assurance.

- **Decision:** `duumbi check --properties` is the canonical v1 user surface.
  **Evidence:** The issue suggests `duumbi check --properties` and the existing
  `check` command already owns parse/validate behavior.

- **Decision:** Property-test success is evidence, not proof.
  **Evidence:** The future VCGen note owns proof status. Randomized testing can
  find counterexamples and improve confidence but cannot prove universal
  correctness.

## Behavior

### Contract Vocabulary

- A function may declare zero or more contract clauses.
- Contract clauses have stable ids or labels so evidence can point to the exact
  failed or unsupported clause.
- Preconditions filter generated inputs before the function executes.
- Postconditions evaluate after successful function execution and may reference:
  - named parameters;
  - the returned value;
  - basic comparisons;
  - boolean combinations;
  - simple arithmetic expressions supported by the property evaluator;
  - length/contains-style predicates only when supported by deterministic local
    evaluation.
- Invariants are parsed and described, but property execution only reports them
  as unsupported unless a later accepted implementation proves an invariant
  execution model.
- Effect declarations classify a function as pure, read-only deterministic, or
  effectful/unsupported for v1 property execution.

### Generator Behavior

- The runner derives generators from `duumbi:params`.
- Generation is deterministic for a given seed, graph, function order, and
  runner settings.
- Primitive generators cover boundary and random values:
  - `i64`: zero, small positives/negatives, min/max-adjacent values, and seeded
    random values;
  - `f64`: finite values, zero, signed values, small fractions, and explicitly
    documented exclusions for NaN/infinity unless a contract opts in;
  - `bool`: both true and false;
  - `string`: empty, ASCII, whitespace, numeric text, and bounded random text.
- Compound generators are bounded:
  - arrays have bounded length;
  - structs generate each known field when field metadata is available;
  - options include `None` and generated `Some`;
  - results include generated `Ok` and `Err` when both payload types are
    supported.
- Unsupported generator cases are reported with a reason and do not count as
  passing cases.

### Property Run Behavior

- `duumbi check --properties` first performs ordinary graph parse/validation.
- If ordinary validation fails, property execution does not run.
- The command discovers property-testable functions in deterministic order.
- The user can provide a seed and case count. Defaults are deterministic and
  documented.
- A passing function reports:
  - function id/name;
  - cases generated;
  - cases executed;
  - precondition rejections;
  - postconditions checked;
  - seed;
  - evidence artifact path.
- An unsupported function reports:
  - function id/name;
  - unsupported type, contract feature, invariant, or effect reason;
  - whether any other contract on the same function was checked.
- A failing function reports:
  - original seed and case index;
  - failing input values;
  - actual return value, trap, or execution failure;
  - failed postcondition id;
  - shrink status;
  - minimal counterexample when found;
  - graph node ids needed for review.
- The command exits nonzero when any supported property fails or when an
  explicit `--require-properties` style behavior is added and no properties are
  testable. If no such stricter flag is added, unsupported-only functions should
  produce warnings and evidence rather than a failure exit.

### Evidence Behavior

- Evidence is written locally and deterministically enough for review diffs.
- Evidence includes a schema version and command settings.
- Evidence does not include credentials, raw provider payloads, or unbounded
  logs.
- Evidence can be referenced from query and describe surfaces, but stale
  evidence must be labeled by timestamp/seed rather than treated as current
  proof.

### CI Behavior

- Normal CI uses provider-free deterministic fixtures.
- CI should run a small stdlib/property subset only when Rust-relevant files or
  property fixtures change.
- Live provider or LLM paths are not required for CI.

## BDD Scenarios

Feature: Contract-based property test generation

  Rule: Contracts are parsed as reusable graph metadata

    Scenario: A pure function declares a supported precondition and postcondition
      Given a DUUMBI graph function with typed parameters
      And the function declares a v1 precondition and postcondition
      When the graph is parsed and described
      Then the contract metadata is preserved on the function
      And the description shows that the function has property-testable contracts

    Scenario: A malformed contract is present
      Given a DUUMBI graph function with a malformed v1 contract clause
      When the user runs `duumbi check`
      Then validation fails with a structured diagnostic
      And property execution does not start

  Rule: Property inputs are deterministic and type-driven

    Scenario: A supported pure function passes generated property cases
      Given a function `abs` with an `i64` parameter and postcondition `result >= 0`
      And the user selects seed `717` and a fixed case count
      When the user runs `duumbi check --properties --seed 717`
      Then DUUMBI executes the same generated cases on every run
      And the evidence records passed case counts and the seed

    Scenario: Preconditions filter generated cases without hiding the count
      Given a function contract with a precondition `n > 0`
      When the property runner generates candidate inputs
      Then inputs that do not satisfy the precondition are rejected
      And the evidence records the rejected candidate count
      And the runner stops with an unsupported or failed-precondition-status if
      the rejection budget is exhausted

  Rule: Failures shrink to reproducible counterexamples

    Scenario: A postcondition fails for a generated case
      Given a function whose contract says `result >= input`
      And one generated input violates that postcondition
      When `duumbi check --properties` runs with a fixed seed
      Then the command exits nonzero
      And the evidence records the seed, failing input, actual output, and
      failed postcondition id
      And the evidence records the smallest counterexample found by the
      deterministic shrinker

    Scenario: A shrink attempt cannot reduce the failing case
      Given a failing generated input that is already minimal
      When the shrinker runs
      Then the evidence records `shrink_status: minimal`
      And the original failing input is preserved as the counterexample

  Rule: Unsupported effects are honest evidence

    Scenario: A contract is attached to an effectful resource function
      Given a function accepts `db_connection` or `http_response`
      And no approved effect model exists for that resource
      When `duumbi check --properties` runs
      Then DUUMBI does not generate arbitrary resource handles
      And the evidence records the function as unsupported for v1 property
      execution with a concrete reason

  Rule: Property evidence is visible but not proof

    Scenario: Recent property evidence exists for a function
      Given a function has a recent property evidence artifact
      When the user runs `duumbi describe`
      Then the function summary references the contract and evidence status
      But it does not claim the function is formally proved

    Scenario: Future formal verification consumes the same vocabulary
      Given a function has v1 preconditions and postconditions
      When the later VCGen workflow is implemented
      Then it can consume the same contract fields without requiring a contract
      vocabulary migration

## Tasks

- Contract vocabulary and metadata:
  - define v1 contract structs and JSON-LD field names;
  - extend parser/AST/graph metadata;
  - validate contract shape and unsupported predicate features.
- Property generation:
  - implement deterministic generator settings and seed policy;
  - add generators for supported pure value types;
  - add unsupported reasons for resource/effectful types.
- Property execution:
  - add a runner that invokes compiled functions under bounded settings;
  - evaluate preconditions and postconditions;
  - add deterministic shrinking.
- Evidence and surfaces:
  - write compact evidence JSON;
  - summarize evidence in CLI output;
  - add describe/query source exposure for contract/evidence status.
- Stdlib and fixtures:
  - add a small Tier 1 property subset;
  - add passing, failing, unsupported, and replay fixtures.
- Documentation:
  - document contract vocabulary, CLI use, evidence schema, limitations, and
    relationship to future formal verification.

## Checks

- Product/contract checks:
  - a fixture with v1 contracts parses and preserves metadata;
  - malformed contract fixtures fail with structured diagnostics;
  - contract descriptions show property-testability and unsupported reasons.
- Generator checks:
  - same seed and settings produce identical cases;
  - different seeds can produce different random cases while preserving
    deterministic boundary cases;
  - pure supported types have generator coverage;
  - resource/effectful types report unsupported reasons.
- Runner checks:
  - passing function evidence records pass counts;
  - failing function evidence records seed, case index, failed contract, and
    counterexample;
  - precondition rejection budgets are enforced;
  - unsupported-only contracts are not counted as passes.
- CLI checks:
  - `duumbi check` still behaves as before without `--properties`;
  - `duumbi check --properties` runs ordinary validation first;
  - `duumbi check --properties --seed 717` is reproducible;
  - evidence path and summary are stable and bounded.
- CI/manual checks:
  - `cargo fmt --check`;
  - `cargo clippy --all-targets -- -D warnings`;
  - focused Rust tests for property modules and fixtures;
  - `cargo test --all` before final implementation review;
  - a local CLI smoke run on a passing and failing fixture.
- BDD coverage:
  - every BDD scenario above maps to a unit, integration, CLI smoke, or review
    evidence item in the technical spec.

## Open Questions

None block Stage 8.

Accepted v1 limitations:

- Effectful/resource-backed functions are recognized and reported as
  unsupported until an effect model is approved.
- Invariants are parsed and preserved, but v1 property execution does not claim
  loop invariant checking.
- Property success is evidence, not formal proof.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/717
- Stage 5 decision:
  https://github.com/hgahub/duumbi/issues/717#issuecomment-4722615299
- Source inbox note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-06-12 - Contract Property Test Generation.md`
- Future VCGen note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Formal Verification VCGen MVP.md`
- Determinism roadmap note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Determinism Program for AI Development.md`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Current parser and graph metadata:
  - `src/parser/ast.rs`
  - `src/parser/mod.rs`
  - `src/graph/mod.rs`
  - `src/graph/builder.rs`
  - `src/graph/validator.rs`
- Current CLI/evidence surfaces:
  - `src/cli/mod.rs`
  - `src/main.rs`
  - `src/cli/commands.rs`
  - `src/graph/describe.rs`
  - `src/bench/report.rs`
  - `src/bench/runner.rs`
  - `docs/testing/phase9c-benchmark.md`
