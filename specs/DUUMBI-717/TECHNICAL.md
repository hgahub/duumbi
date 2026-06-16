# DUUMBI-717: Contract-Based Property Test Generation - Technical Specification

Related to #717. This is a specification-only artifact. The execution issue
must remain open for Stage 9 Technical Spec Review, Stage 10 implementation,
Stage 11 implementation review, and Stage 12 closure.

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-717/PRODUCT.md` by adding a
provider-free, deterministic property-test path for DUUMBI functions with v1
contracts.

The finished implementation must:

- Parse and preserve a shared JSON-LD contract vocabulary for function
  preconditions, postconditions, invariants, and effect declarations.
- Validate contract shape before property execution starts.
- Derive deterministic type-driven generators from `duumbi:params` and
  `DuumbiType`.
- Execute supported pure functions against seeded generated inputs.
- Evaluate preconditions and postconditions through a bounded local predicate
  evaluator.
- Shrink failing inputs to minimal reproducible counterexamples when possible.
- Write compact local evidence artifacts.
- Expose the path through `duumbi check --properties`.
- Keep property success clearly framed as evidence, not formal proof.

Do not add implementation code during this Stage 8 spec PR.

## Agent Audience

- Codex App implementation agents running bounded Stage 10 Ralph cycles.
- Codex Cloud implementation agents only if a human routes longer validation or
  CI evidence work there.
- Reviewer agents checking parser/schema compatibility, deterministic execution,
  unsupported-effect handling, evidence shape, and CLI behavior.
- Tester agents validating unit, integration, CLI smoke, and CI-safe evidence.

## Source Context

- Product spec: `specs/DUUMBI-717/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/717
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/717#issuecomment-4722615299
- Source inbox note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-06-12 - Contract Property Test Generation.md`
- Future VCGen context:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Formal Verification VCGen MVP.md`
- Determinism context:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Determinism Program for AI Development.md`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Relevant current source:
  - `src/parser/ast.rs`
  - `src/parser/mod.rs`
  - `src/graph/mod.rs`
  - `src/graph/builder.rs`
  - `src/graph/validator.rs`
  - `src/graph/describe.rs`
  - `src/types.rs`
  - `src/cli/mod.rs`
  - `src/cli/commands.rs`
  - `src/main.rs`
  - `src/workspace.rs`
  - `src/compiler/mod.rs`
  - `src/bench/report.rs`
  - `src/bench/runner.rs`
  - `src/bench/showcases.rs`
- Relevant existing tests/fixtures:
  - `tests/fixtures/add.jsonld`
  - `tests/fixtures/fibonacci.jsonld`
  - `tests/fixtures/string_length.jsonld`
  - `tests/fixtures/array_push_get.jsonld`
  - `tests/fixtures/struct_field.jsonld`
  - `tests/fixtures/error_handling/*.jsonld`
  - `tests/integration_phase9c.rs`
  - `tests/integration_phase9a_stdlib.rs`
  - `tests/integration_phase4.rs`
- CI workflow: `.github/workflows/ci.yml`

Verified source facts:

- `src/parser/ast.rs::FunctionAst` currently stores id, name, return type,
  params, blocks, and lifetime params only.
- `src/parser/mod.rs::parse_function` currently parses function params and
  lifetime params but no contract metadata.
- `src/graph/mod.rs::FunctionInfo` currently stores name, return type, params,
  blocks, and lifetime params only.
- `src/graph/mod.rs::GraphNode` stores op-level metadata but not function-level
  contracts.
- `src/types.rs::DuumbiType` includes pure primitive/compound values and opaque
  runtime resources such as TCP, HTTP, and DB handles.
- `src/cli/mod.rs::Commands::Check` currently has only an optional input path.
- `src/main.rs` dispatches `Commands::Check` directly to
  `cli::commands::check`.
- `src/cli/commands.rs::check` currently parses/builds/validates a graph and
  does not run property checks.
- `src/workspace.rs` provides build/run helpers for workspace binaries, but no
  general parameterized function invocation API.
- `src/bench/report.rs` already defines compact provider/evidence report
  patterns that should inform, but not dictate, property evidence shape.
- CI runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo audit`, and `cargo nextest run --workspace` when Rust-relevant files
  change.

Assumptions and recommendations:

- Prefer a new `src/contracts/` or `src/properties/` module pair over burying
  the property runner inside `graph::validator`. Validation should decide
  whether contracts are well-formed; property execution should remain a
  separate command path.
- Use a small DUUMBI-owned predicate AST instead of accepting arbitrary strings
  as executable expressions.
- Add a deterministic in-repo generator/shrinker rather than pulling in
  `proptest` immediately. If Stage 10 proves a dependency is significantly
  safer, it must still preserve DUUMBI's evidence schema and deterministic
  seed policy.
- Keep the first stdlib annotation slice small and pure, such as math/string
  functions that do not require external resources.

## Affected Areas

Expected Stage 10 source changes:

- New `src/contracts/` or `src/contract.rs`
  - Contract data model:
    - `ContractSet`
    - `ContractClause`
    - `ContractKind`
    - `EffectClass`
    - predicate/expression AST
  - JSON-LD field constants and serde helpers.
  - Validation helpers for malformed or unsupported clauses.
- `src/parser/ast.rs`
  - Add defaulted contract metadata to `FunctionAst`.
  - Add typed structs for function contracts or use the shared contract model.
- `src/parser/mod.rs`
  - Parse optional `duumbi:contracts` or equivalent v1 field on
    `duumbi:Function`.
  - Fail malformed contract shape with E009-style schema diagnostics.
  - Preserve backward compatibility when contracts are absent.
- `src/graph/mod.rs`
  - Add contract metadata to `FunctionInfo`.
- `src/graph/builder.rs`
  - Copy parsed function contracts into graph metadata.
- `src/graph/validator.rs`
  - Validate contract references, supported predicate operators, and type
    compatibility between predicate operands and function parameter/return
    types.
  - Keep ordinary validation usable for graphs without contracts.
- New `src/properties/`
  - `generator.rs`: deterministic generators for supported `DuumbiType` values.
  - `shrink.rs`: deterministic shrink order for generated values.
  - `predicate.rs`: evaluator for preconditions and postconditions.
  - `runner.rs`: property discovery and execution.
  - `evidence.rs`: serializable evidence schema.
  - `mod.rs`: public command-facing API.
- `src/cli/mod.rs`
  - Add flags to `Commands::Check`:
    - `--properties`;
    - `--seed <u64>`;
    - `--cases <u32>`;
    - optional `--property-output <path>` if the default artifact path is not
      sufficient.
  - Preserve current `duumbi check` behavior when `--properties` is absent.
- `src/main.rs`
  - Dispatch `check --properties` through the property runner after ordinary
    validation.
- `src/cli/commands.rs`
  - Either extend `check` with options or add a sibling helper that returns the
    parsed graph/program evidence needed by the property runner.
- `src/workspace.rs` or a new execution helper
  - Add a bounded way to execute generated function cases.
  - Preferred: build a temporary wrapper `main` per generated case or per batch
    that calls the target function and serializes a simple result.
  - Alternative: add a reusable function-invocation harness if it can be
    implemented without broad compiler/runtime redesign.
- `src/graph/describe.rs` and `src/cli/describe.rs`
  - Surface contract presence and last known evidence status in bounded text.
  - Do not claim formal proof.
- `src/query/context.rs`
  - Include contract/evidence metadata only as source context if the
    implementation can do so without changing Query mode's read-only behavior.
- `stdlib/*.jsonld`
  - Add a small pure Tier 1 contract subset.
  - Avoid annotating effectful resources in the first implementation unless
    they are intentionally used to prove unsupported evidence.
- New fixtures under `tests/fixtures/properties/`
  - passing pure property fixture;
  - failing property fixture;
  - malformed contract fixture;
  - unsupported effectful/resource fixture;
  - shrink fixture.
- New or extended tests:
  - parser/contract unit tests;
  - graph validator unit tests;
  - generator/shrinker unit tests;
  - property runner unit tests;
  - CLI integration tests, possibly in a new `tests/integration_duumbi717_properties.rs`.
- Documentation:
  - a focused contract/property doc such as `docs/testing/property-checks.md` or
    `docs/contracts.md`;
  - update `docs/architecture.md` only if implementation adds stable contract
    vocabulary to the architecture reference.

Areas that must not change during Stage 10 without explicit review:

- live provider setup or model routing;
- Query mode mutation behavior;
- production telemetry ingestion;
- SMT/VCGen proof claims;
- broad stdlib/resource semantics unrelated to property execution.

## Technical Approach

### 1. Define A Typed Contract Vocabulary

Use a single typed model for property tests and future verification. The exact
JSON-LD field name is an implementation detail, but the preferred shape is:

```json
{
  "@type": "duumbi:Function",
  "@id": "duumbi:math/abs",
  "duumbi:name": "abs",
  "duumbi:returnType": "i64",
  "duumbi:params": [
    { "duumbi:name": "n", "duumbi:paramType": "i64" }
  ],
  "duumbi:contracts": {
    "duumbi:effect": "pure",
    "duumbi:preconditions": [],
    "duumbi:postconditions": [
      {
        "duumbi:id": "result-nonnegative",
        "duumbi:expr": {
          "duumbi:op": ">=",
          "duumbi:left": { "duumbi:var": "result" },
          "duumbi:right": { "duumbi:const": 0 }
        }
      }
    ],
    "duumbi:invariants": []
  }
}
```

Recommended v1 expression nodes:

- variable reference: parameter name or `result`;
- constants: bool, i64, f64, string, json literal where safe;
- comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`;
- boolean combinators: `and`, `or`, `not`;
- arithmetic over numeric supported values: `+`, `-`, `*`, `/`, `%` only where
  the evaluator can report division/modulo by zero as contract-evaluation
  failure rather than a panic;
- length for string/array/json-array if implementation can evaluate it
  deterministically.

Reject malformed or unknown predicate operators during validation. Do not
evaluate arbitrary string expressions, shell commands, Rust code, or provider
outputs.

### 2. Keep Validation Separate From Property Execution

Ordinary `duumbi check` should still parse, build, and validate graph structure.
When contracts are present, validation should also check:

- the contract object shape;
- duplicate contract ids within a function;
- pre/postcondition expression type compatibility;
- `result` references only inside postconditions;
- referenced parameter names exist;
- unsupported expression features produce clear diagnostics;
- effect declarations use known values.

Only after ordinary validation succeeds should the property runner execute.

### 3. Implement Deterministic Value Generation

Represent generated values with a local enum such as:

```rust
pub enum PropertyValue {
    I64(i64),
    F64(f64),
    Bool(bool),
    String(String),
    Json(serde_json::Value),
    Array(Vec<PropertyValue>),
    Struct { name: String, fields: BTreeMap<String, PropertyValue> },
    Option(Option<Box<PropertyValue>>),
    ResultOk(Box<PropertyValue>),
    ResultErr(Box<PropertyValue>),
}
```

Use deterministic seed handling:

- global run seed from CLI or default;
- per-function derived seed using a stable hash of module/function id;
- per-case derived seed using case index;
- deterministic boundary cases before random cases.

Do not use ambient entropy for the default once the run begins. If an RNG crate
is needed, choose one with stable reproducible behavior or wrap it so evidence
does not promise cross-version byte-for-byte sequences unless verified.

Unsupported generator outcomes:

- `void` as a parameter is invalid or unsupported;
- `tcp_socket`, `tcp_listener`, `http_server`, `http_response`,
  `db_connection`, and `db_rows` are unsupported for v1 generation;
- references require a generated owned base value and must not create dangling
  or aliased mutable references;
- structs require field metadata. If current source lacks a struct registry
  that can name fields, report `unsupported_struct_field_metadata_missing`.

### 4. Execute Generated Cases Through A Bounded Harness

The implementation must choose a concrete execution strategy and document it in
code comments/tests. Preferred strategy:

1. Build or reuse a temporary workspace copy for the graph under test.
2. For each property case or small batch, synthesize a temporary wrapper
   `main.jsonld` that calls the target function with generated constant inputs.
3. Build and run through existing workspace build/run helpers.
4. Parse stdout/exit/output into `PropertyValue`.
5. Evaluate postconditions locally against input values and result.

This is slower than in-process interpretation but safer for v1 because it uses
the actual compiler/runtime path. If Stage 10 discovers this is too slow, a
small interpreter can be introduced only when it is clearly scoped and compared
against native behavior for the covered subset.

Implementation must avoid:

- provider calls;
- credential reads beyond ordinary workspace config;
- unbounded process execution;
- running arbitrary user commands from contract metadata;
- committing generated evidence unless the implementation spec or reviewer
  explicitly requests a durable evidence artifact.

### 5. Shrink Failures Deterministically

Shrink only values whose generators are supported. Use simple monotonic rules:

- `i64`: move toward 0, then small boundary values;
- `f64`: move toward 0.0 and simple finite values; avoid NaN/infinity unless
  explicitly generated;
- `bool`: try `false` then `true` or whichever is simpler by defined ordering;
- `string`: shorten, then simplify characters;
- arrays: reduce length, then shrink elements;
- option/result: try simpler variant only when type/contract semantics allow;
- structs: shrink fields in deterministic field-name order.

Each shrink candidate must re-run preconditions. Candidates that violate
preconditions are rejected and counted separately from function failures.

### 6. Evidence Schema

Add a schema-versioned JSON artifact. Recommended top-level shape:

```json
{
  "schema_version": "duumbi.property_evidence.v1",
  "command": "duumbi check --properties --seed 717 --cases 64",
  "graph_input": "tests/fixtures/properties/passing_abs.jsonld",
  "started_at": "2026-06-16T00:00:00Z",
  "finished_at": "2026-06-16T00:00:01Z",
  "settings": {
    "seed": 717,
    "cases": 64,
    "max_array_len": 8,
    "max_precondition_rejections": 256
  },
  "summary": {
    "functions_discovered": 1,
    "functions_checked": 1,
    "functions_unsupported": 0,
    "properties_failed": 0
  },
  "functions": []
}
```

Function records should include:

- module/function id and name;
- effect class;
- contract ids;
- generator support status;
- cases generated/executed/rejected;
- postconditions checked;
- failure/counterexample when present;
- unsupported reason when present.

The CLI summary should be concise and point to the artifact path.

### 7. CLI Contract

Target UX:

```sh
duumbi check --properties --seed 717 --cases 64
duumbi check path/to/main.jsonld --properties --seed 717 --cases 64
```

Behavior:

- without `--properties`, current `duumbi check` behavior is unchanged;
- with `--properties`, ordinary validation runs first;
- if validation fails, property execution does not start;
- if a supported property fails, exit nonzero;
- if no supported properties are found, print a clear warning and evidence
  summary. Add a stricter flag only if implementation needs CI to require at
  least one property-bearing function;
- write evidence to a deterministic default under `.duumbi/evidence/properties/`
  for workspace runs and to a temp or adjacent configured path for direct file
  runs.

### 8. Describe And Query Exposure

Keep exposure bounded:

- `duumbi describe` can show `contracts: N postconditions, effect=pure` and
  `property evidence: latest pass/fail/unsupported at <path>` when discoverable.
- Query mode can use the same metadata as read-only source context.
- Neither surface should claim "verified" or "proved" from property tests.

## Invariants

- Graph/parser modules must not import Cranelift types.
- Existing graphs without contracts remain parseable and checkable.
- `duumbi check` without `--properties` remains backward-compatible.
- Property runs are provider-free and do not require LLM credentials.
- Property evidence never contains secrets, raw provider payloads, or unbounded
  logs.
- Unsupported effectful/resource functions are reported honestly, not counted
  as passes.
- Property-test success is not formal verification.
- Generated cases and shrink order are deterministic for the same seed/settings
  within the supported implementation version.
- Query mode remains read-only.

## BDD-To-Test Mapping

| Product BDD scenario | Required verification evidence |
| --- | --- |
| A pure function declares a supported precondition and postcondition | Parser unit test for contract metadata; graph builder test preserving `FunctionInfo` contracts; `duumbi describe` fixture showing bounded contract summary. |
| A malformed contract is present | Parser/validator test with malformed contract JSON-LD; CLI integration test proving `duumbi check --properties` does not run after validation failure. |
| A supported pure function passes generated property cases | Property runner unit test and CLI fixture for `abs` or equivalent pure function; assert deterministic pass evidence with seed `717`. |
| Preconditions filter generated cases without hiding the count | Generator/runner test where `n > 0` rejects candidates; assert rejected count and rejection-budget behavior in evidence. |
| A postcondition fails for a generated case | Failing fixture where implementation violates a postcondition; assert nonzero CLI exit and evidence fields for seed, case, actual result, failed condition, and counterexample. |
| A shrink attempt cannot reduce the failing case | Shrinker unit test with already minimal failing input; assert `shrink_status: minimal` and preserved counterexample. |
| A contract is attached to an effectful resource function | Fixture using `db_connection` or `http_response`; assert unsupported reason and no arbitrary resource generation. |
| Recent property evidence exists for a function | Describe/query-context test showing evidence status without proof wording. |
| Future formal verification consumes the same vocabulary | Review evidence: contract vocabulary is in shared model/docs, not property-runner-private fields; no VCGen implementation required. |

## Live E2E Plan

Canonical interface: CLI.

Provider path: none. This issue is provider-free; live E2E must not call an LLM.

Credentials or environment variables: none beyond the local Rust/DUUMBI build
environment and the C compiler already required by DUUMBI.

Expected external LLM calls: 0.

Estimated external LLM cost: USD 0.

Commands:

```sh
cargo build
cargo test --test integration_duumbi717_properties
target/debug/duumbi check tests/fixtures/properties/passing_abs.jsonld --properties --seed 717 --cases 32 --property-output /tmp/duumbi-717-passing.json
target/debug/duumbi check tests/fixtures/properties/failing_abs.jsonld --properties --seed 717 --cases 32 --property-output /tmp/duumbi-717-failing.json
target/debug/duumbi check tests/fixtures/properties/unsupported_effect.jsonld --properties --seed 717 --cases 32 --property-output /tmp/duumbi-717-unsupported.json
```

Pass/fail criteria:

- passing fixture exits 0 and evidence has at least one checked function and no
  failed properties;
- failing fixture exits nonzero and evidence includes a failed postcondition and
  counterexample;
- unsupported fixture exits according to the finalized unsupported policy and
  always records a concrete unsupported reason without generated resource
  handles;
- all artifacts are bounded JSON and contain no credentials or raw provider
  payloads.

TUI/Studio parity:

- No full UI E2E is required unless Stage 10 changes shared check/describe APIs
  used by TUI or Studio.
- If describe/query metadata changes are shared with TUI/Studio surfaces, run a
  thin smoke proving those surfaces still render bounded text and remain
  read-only where applicable.

## Ralph Cycle Protocol

Each Stage 10 implementation cycle must:

1. summarize the current state and remaining unmet requirements;
2. propose one bounded implementation goal;
3. list intended file areas and commands;
4. estimate resource use and risk;
5. check whether the resource gate requires human approval;
6. implement only the approved or resource-permitted goal;
7. run the agreed checks;
8. report evidence, failures, and remaining gaps;
9. stop only if requirements are met, a blocker appears, the expected external
   LLM cost of the next cycle exceeds USD 1, scope changes, a risky dependency
   or irreversible operation is needed, or a product/architecture decision is
   required. Iteration count is not a stop condition.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: prefer 3-6 tightly related files. Exceed this
  only when a mechanical rename or shared type propagation requires it and the
  cycle states the reason.
- Expected command budget per cycle:
  - focused `cargo test <module-or-integration-target>`;
  - `cargo fmt --check` after Rust edits;
  - `cargo clippy --all-targets -- -D warnings` before PR readiness;
  - `cargo test --all` before final implementation review.
- Human approval required only when the cycle will use an external LLM with
  expected cost above USD 1, exceeds approved scope, adds risky dependencies or
  irreversible operations, changes public contract semantics beyond this spec,
  or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is covered by the Codex
  App subscription and never triggers the gate.
- No autonomous batch cap: cycles continue until completion, blocker, gate
  breach, or scope change.
- When to stop and ask for human guidance:
  - the implementation needs effectful resource property execution instead of
    unsupported evidence;
  - contract vocabulary changes would conflict with the future VCGen source
    note;
  - a new dependency is necessary and meaningfully increases supply-chain or CI
    risk;
  - generated cases require a broader runtime invocation model than wrapper
    `main` or a small accepted harness;
  - CI/runtime behavior is nondeterministic across repeated local runs.

## Task Breakdown

1. Contract model and parser propagation
   - Add contract structs and JSON-LD parsing.
   - Add defaulted contract fields to AST and graph metadata.
   - Add parser tests for absent, valid, and malformed contracts.

2. Contract validation
   - Validate predicate syntax, type compatibility, parameter references,
     duplicate ids, and effect declarations.
   - Add diagnostics and fixture tests.

3. Generator and value model
   - Add `PropertyValue`, generator settings, deterministic seed derivation,
     supported value generators, and unsupported reason handling.
   - Add unit tests for reproducibility and supported/unsupported types.

4. Predicate evaluator
   - Evaluate preconditions and postconditions over generated inputs and
     returned values.
   - Add tests for comparisons, boolean combinators, arithmetic, rejection
     counts, and unsupported expressions.

5. Native execution harness
   - Implement the smallest wrapper or invocation path that calls generated
     functions through compiled DUUMBI code.
   - Add passing/failing fixture tests.

6. Shrinker
   - Add deterministic shrink rules and evidence statuses.
   - Add failing/minimal tests.

7. Evidence and CLI
   - Add evidence schema and artifact writing.
   - Extend `duumbi check` flags and dispatch.
   - Add CLI integration tests for pass/fail/unsupported paths.

8. Describe/query/docs/stdlib slice
   - Add bounded contract/evidence summaries.
   - Annotate a small pure stdlib subset.
   - Document vocabulary, CLI, evidence, and limitations.

9. Final verification
   - Run formatting, focused tests, clippy, full test suite, and the live CLI
     E2E commands.

## Verification Plan

Required local checks before implementation PR review:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test properties::
cargo test contracts::
cargo test --test integration_duumbi717_properties
cargo test --all
```

Required CLI smoke checks:

```sh
cargo build
target/debug/duumbi check tests/fixtures/properties/passing_abs.jsonld --properties --seed 717 --cases 32 --property-output /tmp/duumbi-717-passing.json
target/debug/duumbi check tests/fixtures/properties/failing_abs.jsonld --properties --seed 717 --cases 32 --property-output /tmp/duumbi-717-failing.json
target/debug/duumbi check tests/fixtures/properties/unsupported_effect.jsonld --properties --seed 717 --cases 32 --property-output /tmp/duumbi-717-unsupported.json
```

Required review evidence:

- diff changes are limited to approved affected areas;
- generated evidence files are not committed unless intentionally added as a
  compact docs artifact;
- every unsupported generator case has a reason;
- evidence schema is documented and tested;
- no provider calls or credentials are required;
- describe/query wording avoids proof claims.

## Completion Criteria

The implementation is complete only when:

- `PRODUCT.md` BDD scenarios are covered by tests or explicit review evidence;
- contracts parse into typed metadata and remain backward-compatible for graphs
  without contracts;
- malformed contracts fail validation before property execution;
- supported pure functions can pass deterministic property cases;
- failing properties produce nonzero CLI result and counterexample evidence;
- unsupported effectful/resource contracts are reported honestly;
- `duumbi check --properties` has stable flags, help text, and tests;
- property evidence is schema-versioned, bounded, and free of secrets;
- a small pure stdlib Tier 1 contract subset exists;
- docs explain property tests versus formal proof;
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, focused
  tests, CLI smoke, and `cargo test --all` pass or any environmental failure is
  recorded with exact evidence.

## Failure And Escalation

- If ordinary graph validation fails, do not run property execution; report the
  validation diagnostics.
- If wrapper-based function execution cannot represent generated input values
  safely, stop and report the exact type/harness blocker.
- If generated values expose nondeterministic runtime behavior, reduce scope to
  deterministic supported types or stop for architecture review.
- If a postcondition evaluator feature would require arbitrary code execution,
  reject it and add a structured unsupported predicate diagnostic.
- If resource/effectful generation is requested during implementation, stop for
  product/architecture approval.
- If a new dependency is required, explain why local deterministic code is not
  sufficient and request approval when supply-chain or CI risk is meaningful.
- If repeated tests are flaky with the same seed/settings, block readiness until
  determinism is restored or the flaky behavior is isolated and explicitly
  excluded.

## Open Questions

None block Stage 10.

Accepted implementation limits:

- effectful/resource types are unsupported for v1 property execution;
- invariants are parsed and preserved but not executed as loop proofs;
- property-test success is evidence only, not formal verification;
- a small pure stdlib subset is enough for first delivery, provided the
  extension path is documented.
