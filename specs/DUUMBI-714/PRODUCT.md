# DUUMBI-714: Codegen Trap Discipline and Backend Hardening

## Summary

DUUMBI's graph-to-native backend must make runtime failures deterministic,
diagnosable, and aligned with later verification work. The accepted issue asks
for trap discipline around division and array access, a safe `ArrayTryGet`
path, Cranelift IR verification, differential interpreter-vs-native checks,
typed struct layout beyond the current fixed buffer, and a published integer
overflow policy with checked arithmetic variants.

Related to #714. This is a product-specification artifact only. The execution
issue must remain open for Stage 7 Product Spec Review, Stage 8 technical
specification, Stage 9 Technical Spec Review, Stage 10 implementation, Stage 11
review, and Stage 12 closure.

## Problem

DUUMBI's product promise depends on an inspectable semantic graph becoming
verified executable behavior. Current backend behavior is not yet strong enough
for that promise:

- Integer `Div` currently lowers directly to Cranelift `sdiv`, so division by
  zero can fail as an opaque host process failure rather than a DUUMBI runtime
  error with graph context.
- Runtime array get/set OOB checks exist in the current source, but they emit a
  generic `duumbi panic: array index out of bounds` message without op node
  attribution.
- `ArrayTryGet` exists in the parser and op model, but current codegen returns
  an explicit unimplemented `CompileError`.
- Runtime trace and crash evidence can map panics to function/block context, but
  exact operation node attribution is still missing for these trap cases.
- Struct allocation currently uses a fixed 64-byte default with sequential
  8-byte field offsets, which cannot prove layout correctness for larger
  mixed-type structs.
- CI runs format, clippy, audit, and nextest, but it does not require a
  dedicated Cranelift IR verifier path or differential interpreter-vs-native
  semantic checks.
- Integer overflow behavior is not documented as a stable semantic contract,
  and checked arithmetic operations are not available for verification-friendly
  graph programs.

This creates a reliability gap: graph programs can compile and run, but some
backend failures are not deterministic enough for users, tests, telemetry, or
future verification tools to explain.

## Outcome

When this work is done:

- Integer division by zero in `duumbi:Div` fails through a DUUMBI-controlled
  panic path with a deterministic stderr message and the exact operation node
  id.
- `ArrayGet` and `ArraySet` out-of-bounds accesses fail through the same
  deterministic node-attributed panic path. Negative indexes must be treated as
  out of bounds.
- No accepted trap case silently returns an incorrect value.
- `ArrayTryGet` works end-to-end for arrays of supported DUUMBI value types and
  returns `option<T>`: `Some(value)` when the index is valid and `None` when it
  is out of bounds.
- `ArrayTryGet` results can be consumed by `Match`, `OptionIsSome`, and
  `OptionUnwrap` according to existing Result/Option safety rules.
- Runtime crash evidence can include the operation node id for the new
  node-attributed trap cases, while preserving existing function/block
  telemetry behavior.
- Cranelift IR verification runs in local debug/test paths and in CI for the
  compiled fixture coverage touched by this issue.
- A small interpreter or semantics oracle exists for the deterministic subset
  needed by this issue, and CI compares interpreter output/failure semantics
  against native output/failure semantics.
- A struct with at least 12 mixed-type fields compiles, stores, reads, and lays
  out correctly without relying on the current 64-byte cap.
- Integer overflow behavior is documented. Existing unchecked integer
  `Add`, `Sub`, and `Mul` retain wrapping behavior for compatibility. New
  checked integer variants report overflow as typed `Result` values instead of
  panicking or wrapping.
- Cross-compilation groundwork is present as an implementation-facing target
  selection and linking strategy, but full cross-target release support remains
  outside this issue.

## Scope

### In Scope

- Deterministic node-attributed traps for integer division by zero and array OOB
  get/set.
- Runtime and telemetry changes needed to carry node attribution for these
  trap cases.
- `ArrayTryGet` codegen/runtime support returning `option<T>` and working with
  `Match`.
- A documented integer arithmetic policy:
  - unchecked integer add/sub/mul wrap by default for compatibility;
  - integer division by zero traps deterministically;
  - checked integer add/sub/mul/div variants return `Result`.
- Parser, type model, validator, codegen, and runtime support for checked
  arithmetic variants needed by the policy.
- Cranelift IR verifier integration for compiler tests and CI.
- Differential interpreter-vs-native tests for deterministic arithmetic,
  arrays, options/match, and struct layout cases covered by this issue.
- Type-aware struct layout for supported scalar, bool, string/pointer, array,
  result/option pointer, and struct-reference field shapes.
- A fixture proving 12 or more mixed-type struct fields.
- Focused docs updates under the source repo for backend semantics and test
  expectations.
- Cross-compilation groundwork limited to explicit target-triple planning or
  narrow target selection support, with documented linker limitations.

### Explicitly Out Of Scope

- Full SMT/VCGen formal verification.
- Production telemetry ingestion, cloud crash reporting, account identity, or
  autonomous repair acceptance.
- A complete cross-compilation release matrix, prebuilt binary release work, or
  WASM target support.
- Broad compiler refactors unrelated to trap discipline, checked arithmetic,
  struct layout, verifier integration, or differential tests.
- Changing float division semantics beyond documenting current behavior.
- Extending arrays to arbitrary by-value element layouts if the current runtime
  still represents supported heap and compound values as pointer-sized payloads.
- Studio, TUI, or Query UI changes unless needed to expose the same backend
  error text through existing command output.
- Implementation code, runtime changes, tests, docs edits, generated artifacts,
  or Ralph cycles during this specification stage.

## Constraints And Assumptions

Facts:

- Issue #714 is open and labeled `accepted` and `needs-spec`.
- The Stage 5 Human Acceptance Decision comment dated 2026-06-15 records
  `Decision: Accept`, `Next state: Spec Needed`, and no remaining open
  questions.
- `src/compiler/lowering.rs` currently lowers integer `Div` with Cranelift
  `sdiv`.
- `runtime/duumbi_runtime.c` currently panics for array get/set OOB, but the
  message is generic and does not include the graph op node id.
- `src/compiler/lowering.rs` currently returns a Cranelift compile error for
  `ArrayTryGet`.
- `src/parser/mod.rs` parses `duumbi:ArrayTryGet`, `duumbi:Option*`, and
  `duumbi:Match`.
- Existing Option and Match codegen paths are present.
- Runtime telemetry can write panic events and crash evidence with function and
  block ids.
- Struct codegen currently allocates a fixed 64-byte struct buffer and maps
  field names to sequential 8-byte offsets.
- CI currently uses a Linux/Windows matrix with `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo audit`, and
  `cargo nextest run --workspace` for Rust-relevant changes.
- The current GitHub token in this Codex session cannot read ProjectV2 fields
  because it lacks `read:project`; labels and issue comments are available.

Assumptions:

- Node-attributed traps should use graph op `@id` strings, not unstable
  petgraph indices.
- Exact node attribution is required for the trap cases in this issue even when
  normal tracing is not enabled. Trace files may add evidence, but stderr must
  be useful on its own.
- Existing wrapping arithmetic must remain backward-compatible unless a checked
  variant is used.
- Checked arithmetic variants should be explicit graph ops rather than a global
  compile flag, because future verification needs per-node intent.
- The first differential interpreter can cover a deterministic subset. It does
  not need to interpret networking, HTTP, DB, registry, or provider behavior.
- Cross-compilation groundwork should not block backend hardening; host builds
  remain the required live path for this issue.

## Decisions

- Stage 5 accepted the issue for specification with no remaining open questions
  on 2026-06-15.
- Product scope is defined from issue #714, the processed inbox source note,
  active PRD runtime-failure feedback goals, current source inspection, and the
  repo workflow policy.
- The default integer overflow policy is compatibility-preserving wrap for
  unchecked integer add/sub/mul, plus explicit checked variants for programs
  that need verifiable failure behavior.
- Integer division by zero is a deterministic trap for unchecked `Div`, not
  wrapping and not an OS-level failure.
- Floating-point division remains IEEE-like behavior for this issue and is
  documented rather than turned into a trap.
- The product requires exact op node ids for new trap cases. Function/block-only
  telemetry is not sufficient for #714.
- Cross-compilation work is limited to groundwork because full platform release
  work belongs to a release/platform issue.

## Behavior

### Deterministic Traps

- Integer `Div` with a zero divisor exits nonzero through a DUUMBI runtime panic.
- The stderr text must include:
  - a stable DUUMBI panic prefix;
  - the failure kind, such as `division by zero`;
  - the graph op node id of the failing `Div`.
- `ArrayGet` and `ArraySet` must apply bounds checks before reading or writing.
- Valid array indexes behave as they do today.
- An index less than 0 or greater than or equal to the array length is out of
  bounds.
- OOB `ArrayGet` and `ArraySet` exit nonzero through a DUUMBI runtime panic.
- OOB stderr text must include:
  - a stable DUUMBI panic prefix;
  - the failure kind, such as `array index out of bounds`;
  - the graph op node id of the failing array operation.
- When traced execution is enabled, crash evidence must preserve existing
  function/block evidence and add node attribution for these trap cases.
- When traced execution is not enabled, stderr still includes the node id.

### ArrayTryGet

- `ArrayTryGet` returns `option<T>`.
- A valid index returns `Some(value)`.
- An out-of-bounds index returns `None`.
- `ArrayTryGet` does not panic on out-of-bounds input.
- `Match` can branch on the resulting option:
  - the Some branch runs when the index is valid;
  - the None branch runs when the index is invalid.
- `OptionUnwrap` behavior remains unchanged: unwrapping `None` is a panic, but
  the new `ArrayTryGet` path should allow users to avoid that panic.

### Integer Arithmetic Policy

- Existing unchecked integer `Add`, `Sub`, and `Mul` keep wrapping behavior.
- Existing unchecked integer `Div` traps deterministically on divisor zero.
- Checked integer add/sub/mul/div variants return `Result<i64,string>` or the
  closest existing typed-result representation accepted by the type system.
- Checked variants return Ok with the arithmetic value on success.
- Checked add/sub/mul return Err when the operation would overflow `i64`.
- Checked div returns Err for divisor zero and for signed division overflow
  such as `i64::MIN / -1`.
- Checked variants do not panic for expected arithmetic failure cases.

### Struct Layout

- Struct allocation size must be derived from fields that the compiler can know
  for the struct type, not from a fixed 64-byte default.
- Field offsets must be stable for a given struct shape and must respect the
  storage size and alignment policy documented by implementation.
- A struct with at least 12 mixed-type fields must compile and run correctly.
- Field get/set must preserve current behavior for simple existing fixtures.

### Verifier And Differential Evidence

- Cranelift IR verifier failures fail compilation or tests with a useful error
  that names the DUUMBI function being compiled.
- CI must include verifier-backed coverage for the changed compiler paths.
- The interpreter-vs-native differential test path must compare:
  - stdout;
  - exit status;
  - deterministic DUUMBI panic kind and node id for expected trap cases.
- Differential tests must exclude nondeterministic or external-resource ops
  unless those ops have a deterministic local fixture.

### Empty, Error, Retry, Cancellation, Accessibility

- Empty arrays are valid. `ArrayTryGet` on an empty array returns `None`;
  `ArrayGet` and `ArraySet` on an empty array trap with node attribution.
- The feature does not introduce retry behavior.
- The feature does not introduce cancellation behavior.
- Accessibility/focus behavior is not applicable because this issue is backend
  compiler/runtime work. Existing CLI/TUI surfaces should continue to display
  stderr and command failures without clipping or replacing the panic text.

## BDD Scenarios

```gherkin
Feature: Deterministic backend trap discipline

  Scenario: Integer division by zero reports a DUUMBI node-attributed panic
    Given a valid graph contains an integer Div op with node id "duumbi:main/main/entry/div_zero"
    And the divisor evaluates to 0
    When the graph is compiled and the native binary is run
    Then the process exits nonzero
    And stderr contains "duumbi panic"
    And stderr contains "division by zero"
    And stderr contains "duumbi:main/main/entry/div_zero"

  Scenario: ArrayGet out of bounds reports the failing node id
    Given a valid graph creates an array with one element
    And the graph contains an ArrayGet op with node id "duumbi:main/main/entry/get_oob"
    When the program reads index 1
    Then the process exits nonzero
    And stderr contains "array index out of bounds"
    And stderr contains "duumbi:main/main/entry/get_oob"

  Scenario: ArraySet with a negative index reports the failing node id
    Given a valid graph creates an array with one element
    And the graph contains an ArraySet op with node id "duumbi:main/main/entry/set_negative"
    When the program writes index -1
    Then the process exits nonzero
    And stderr contains "array index out of bounds"
    And stderr contains "duumbi:main/main/entry/set_negative"

  Scenario: ArrayTryGet returns Some for an in-bounds index
    Given a valid graph creates an array containing 42
    And the graph calls ArrayTryGet at index 0
    When the result is matched
    Then the Some branch runs
    And the program prints 42
    And the process exits successfully

  Scenario: ArrayTryGet returns None for an out-of-bounds index
    Given a valid graph creates an array containing 42
    And the graph calls ArrayTryGet at index 9
    When the result is matched
    Then the None branch runs
    And the program prints a deterministic fallback value
    And the process exits successfully

  Scenario: Checked multiplication returns Err on overflow
    Given a valid graph uses checked integer multiplication
    And the operands would overflow i64
    When the checked result is matched
    Then the Err branch runs
    And the program does not panic

  Scenario: Unchecked multiplication preserves documented wrapping behavior
    Given a valid graph uses existing unchecked integer multiplication
    And the operands overflow i64
    When the graph is compiled and run
    Then the result matches the documented wrapping policy
    And the process does not panic because of overflow

  Scenario: A large mixed-type struct preserves field values
    Given a valid graph creates a struct with at least 12 mixed-type fields
    When the program writes and reads representative fields
    Then each read returns the value written to that field
    And no field aliases another field unexpectedly
    And the process exits successfully

  Scenario: Cranelift IR verification catches malformed generated IR
    Given the compiler emits Cranelift IR for a DUUMBI fixture
    When verifier mode is enabled in tests or CI
    Then invalid IR fails the check before object emission is accepted
    And the failure message names the DUUMBI function being compiled

  Scenario: Interpreter and native execution agree for deterministic fixtures
    Given a deterministic graph fixture in the supported interpreter subset
    When the interpreter and native binary both execute the fixture
    Then stdout, exit status, and expected DUUMBI panic metadata match
```

## Tasks

- Define the runtime failure message contract and node attribution shape.
- Add deterministic integer `Div` and array OOB trap behavior.
- Implement `ArrayTryGet` as `option<T>` and prove `Match` integration.
- Define and document arithmetic overflow semantics and checked variants.
- Add type-aware struct layout and a large mixed-type struct fixture.
- Enable Cranelift IR verifier checks in local tests and CI.
- Add a deterministic interpreter or semantics oracle for the subset needed by
  this issue.
- Add differential interpreter-vs-native fixtures and CI coverage.
- Document target-triple/linking groundwork without claiming full
  cross-compilation support.
- Record implementation evidence in the issue and implementation PR during
  later stages.

## Checks

- Unit and integration tests cover integer division by zero, array get OOB,
  array set OOB, negative indexes, valid array access, `ArrayTryGet` Some,
  `ArrayTryGet` None, checked arithmetic success, checked arithmetic overflow,
  unchecked wrap semantics, and mixed-type struct layout.
- Telemetry tests prove traced crash evidence includes node attribution for the
  new trap cases and still joins existing function/block trace context.
- Cranelift verifier tests fail on malformed generated IR and pass for the
  relevant fixtures.
- Differential tests compare interpreter and native behavior for deterministic
  fixtures.
- CI runs the relevant verifier and differential tests on Linux and Windows
  where supported by existing runtime dependencies.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test --all` or the CI-equivalent nextest command pass before the final
  implementation PR is reviewed.
- Manual live E2E evidence in Stage 10 compiles and runs at least:
  - div-by-zero trap fixture;
  - array OOB trap fixture;
  - `ArrayTryGet` Some/None with `Match`;
  - 12+ mixed-field struct fixture.

## Open Questions

None blocking for product specification.

Implementation should still verify exact Cranelift verifier APIs and the most
maintainable representation for checked arithmetic results before coding. If
the existing type/runtime representation cannot safely support the checked
variant contract, Stage 10 must stop for an architecture decision rather than
shipping a partial semantic claim.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/714
- Stage 5 Human Acceptance Decision:
  https://github.com/hgahub/duumbi/issues/714#issuecomment-4712216059
- Processed inbox source:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/2026-06-12 - Codegen Trap Discipline and Backend Hardening.md`
- Active DUUMBI PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active DUUMBI glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Active DUUMBI Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Repo architecture reference: `docs/architecture.md`
- Repo coding conventions: `docs/coding-conventions.md`
- Workflow policy: `docs/automation/agentic-development-orchestration.md`
- Review policy: `docs/automation/code-review-policy.md`
- Compiler lowering: `src/compiler/lowering.rs`
- Runtime C API and implementation:
  `runtime/duumbi_runtime.h`, `runtime/duumbi_runtime.c`
- Op and type model: `src/types.rs`
- Parser: `src/parser/mod.rs`
- Graph validator: `src/graph/validator.rs`
- Existing tests:
  `tests/integration_phase9a1.rs`,
  `tests/integration_phase9a3.rs`,
  `tests/integration_telemetry.rs`,
  `tests/fixtures/array_push_get.jsonld`,
  `tests/fixtures/struct_field.jsonld`,
  `tests/fixtures/error_handling/option_some_unwrap.jsonld`
