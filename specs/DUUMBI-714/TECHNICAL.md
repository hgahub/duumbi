# DUUMBI-714: Codegen Trap Discipline and Backend Hardening - Technical Specification

## Implementation Objective

Implement the approved product behavior in `specs/DUUMBI-714/PRODUCT.md`:

- deterministic node-attributed runtime traps for integer division by zero and
  array get/set out-of-bounds;
- `ArrayTryGet` end-to-end as `option<T>` with `Match` support;
- published integer overflow semantics plus checked integer arithmetic variants;
- Cranelift IR verifier coverage in debug/test/CI paths;
- interpreter-vs-native differential checks for the deterministic subset in
  this issue;
- type-aware struct layout that handles at least 12 mixed-type fields;
- narrow cross-compilation groundwork without claiming full cross-target release
  support.

Related to #714. This is a technical-specification artifact only. The execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex App implementation agents running bounded local Ralph cycles.
- Codex Cloud implementation agents when routed from GitHub or Slack.
- Codex CLI agents used for local compiler/runtime investigation.
- Stage 9 technical reviewers checking implementability, source boundaries,
  BDD-to-test coverage, and resource policy.
- Stage 11 reviewers checking final implementation evidence against the product
  and technical specs.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/714
- Product spec: `specs/DUUMBI-714/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/726
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/714#issuecomment-4712216059
- Stage 6 product spec draft:
  https://github.com/hgahub/duumbi/issues/714#issuecomment-4722533143
- Stage 7 product spec approval:
  https://github.com/hgahub/duumbi/issues/714#issuecomment-4722576036
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Workflow policy: `docs/automation/agentic-development-orchestration.md`
- Review policy: `docs/automation/code-review-policy.md`

Verified source facts:

- `src/types.rs` defines `Op::Div`, `Op::ArrayGet`, `Op::ArraySet`,
  `Op::ArrayTryGet`, Result/Option ops, and `Op::Match`; no checked arithmetic
  variants currently exist.
- `src/parser/mod.rs` parses `duumbi:ArrayTryGet`, Result/Option ops, and
  `duumbi:Match`.
- `src/graph/validator.rs` gates Result/Option safety checks only when
  Result/Option ops are present.
- `src/compiler/lowering.rs` currently lowers integer `Div` directly to
  Cranelift `sdiv`.
- `src/compiler/lowering.rs` currently emits `ArrayGet` and `ArraySet` runtime
  calls without op-node-aware guard code.
- `src/compiler/lowering.rs` currently returns a compile error for
  `ArrayTryGet`.
- `runtime/duumbi_runtime.c` currently has `duumbi_panic`, trace panic
  recording, generic array OOB panics, `duumbi_array_len`, Result constructors,
  Option constructors, and Option unwrap behavior.
- Current telemetry crash evidence records function and block ids, not op node
  ids.
- Struct codegen currently allocates a fixed 64-byte struct object and maps
  field names to sequential 8-byte offsets.
- `.github/workflows/ci.yml` runs Linux and Windows checks, including format,
  clippy, audit, and `cargo nextest run --workspace` for Rust-relevant changes.

Assumptions for implementation:

- Node attribution should use DUUMBI graph op `@id` strings.
- Default unchecked integer add/sub/mul must keep wrapping behavior for
  compatibility.
- Checked integer variants should be graph ops with explicit JSON-LD names.
- Current runtime ABI stores scalars and heap references in pointer-width
  payload slots; struct layout can remain 8-byte-slot based in this issue if
  field types and offsets are derived from a real layout registry instead of the
  fixed 64-byte cap.
- Cross-compilation groundwork should be narrow and must not block the core
  backend-hardening acceptance criteria.

## Affected Areas

Expected Stage 10 source changes:

- Op and type surface:
  - `src/types.rs`
  - `src/parser/mod.rs`
  - `src/graph/builder.rs`
  - `src/graph/validator.rs`
  - `src/graph/result_safety.rs`
- Compiler backend:
  - `src/compiler/lowering.rs`
  - optional focused helper module under `src/compiler/` for trap guards,
    struct layout, verifier integration, or deterministic interpretation if it
    reduces complexity.
- Runtime:
  - `runtime/duumbi_runtime.h`
  - `runtime/duumbi_runtime.c`
- CLI/build path only as needed to expose verifier or target groundwork:
  - `src/compiler/mod.rs`
  - `src/compiler/linker.rs`
  - `src/cli/` or `src/main.rs` only if a user-facing target flag is required
    by the chosen cross-compilation groundwork slice.
- Tests and fixtures:
  - new `tests/integration_duumbi714_backend_hardening.rs` or similarly focused
    integration test file;
  - new fixtures under `tests/fixtures/backend_hardening/`;
  - existing `tests/integration_phase9a1.rs`;
  - existing `tests/integration_phase9a3.rs`;
  - existing `tests/integration_telemetry.rs`;
  - focused unit tests colocated with modified Rust modules.
- CI and docs:
  - `.github/workflows/ci.yml` if a dedicated verifier/differential command must
    be added beyond normal nextest discovery;
  - `docs/architecture.md`;
  - a focused backend semantics or testing document under `docs/` when the
    arithmetic policy needs more detail than the architecture table can hold.

Areas that must not change during Stage 10:

- `specs/DUUMBI-714/PRODUCT.md`
- `specs/DUUMBI-714/TECHNICAL.md` after Stage 9 approval, unless Stage 9 sends
  the issue back to technical-spec revision.
- Provider setup, model catalog, registry, MCP, Studio, TUI, and Slack workflow
  behavior unless a compile/test command already touches those paths.
- Generated release artifacts, production telemetry ingestion, release tags, or
  cross-target binaries.

## Technical Approach

### 1. Node-Attributed Runtime Panic Contract

Add a node-aware panic path while preserving the existing generic panic API:

- Keep `duumbi_panic(const char *msg)` for existing runtime callers.
- Add a new runtime function such as:

```c
void duumbi_panic_at(const char *msg, const char *node_id);
```

- Format stderr deterministically, for example:

```text
duumbi panic: division by zero at node duumbi:main/main/entry/div_zero
```

- Update `duumbi_trace_panic` or add a node-aware trace variant so traced crash
  evidence includes `node_id` when available while preserving existing
  function/block fields.
- Do not require `--trace` for stderr node attribution.
- In lowering, add a helper that embeds stable NUL-terminated strings for
  compile-time messages and node ids. Reuse the existing Cranelift data-section
  pattern where practical instead of ad hoc global handling.

### 2. Guard Integer Division

Implement integer `Div` as guarded codegen:

- Before `sdiv`, compare the right operand against zero.
- Branch to a local trap block when the divisor is zero.
- The trap block calls `duumbi_panic_at("division by zero", node_id)` and then
  emits an unreachable terminator or returns through a verifier-safe path.
- The nonzero path emits `sdiv` and continues normal codegen.
- Preserve f64 `Div` as `fdiv` and document that f64 divide-by-zero remains
  IEEE-like behavior for this issue.
- Handle Cranelift block sealing and SSA dominance through a focused helper
  rather than open-coded branching in the main match arm.

### 3. Guard ArrayGet And ArraySet

Implement node-attributed array bounds checks before runtime array access:

- Use `duumbi_array_len(arr)` to obtain the current length.
- Treat an index as out of bounds when `index < 0` or `index >= len`.
- Branch to `duumbi_panic_at("array index out of bounds", node_id)` for OOB.
- On the in-bounds path, call existing `duumbi_array_get` or
  `duumbi_array_set`.
- Keep generic runtime OOB checks as a defensive backstop; the product-visible
  node-attributed behavior must come from lowering guards for graph ops.

### 4. Implement ArrayTryGet

Implement `ArrayTryGet` as an option-returning codegen path:

- Validate that the op result type is `option<T>`.
- Reuse the same array bounds predicate as `ArrayGet`.
- In the in-bounds path, call `duumbi_array_get`, then
  `duumbi_option_new_some(value)`.
- In the OOB path, call `duumbi_option_new_none()`.
- Join both paths into a continuation block and store the resulting option
  pointer in `value_map` for the `ArrayTryGet` node.
- Use `FunctionBuilder` variables or block parameters to preserve SSA dominance;
  do not rely on mutable Rust local state across Cranelift control-flow joins.
- Add tests proving `Match` branches correctly on both Some and None outputs.

### 5. Add Checked Integer Arithmetic Variants

Add explicit i64 checked arithmetic graph ops:

- `duumbi:AddChecked`
- `duumbi:SubChecked`
- `duumbi:MulChecked`
- `duumbi:DivChecked`

Expected result type:

- `result<i64,string>` for all checked variants.

Behavior:

- Ok payload is the successful i64 result.
- Err payload is a deterministic string value describing the failure kind.
- Add/sub/mul Err on i64 overflow.
- Div Err on divisor zero and signed division overflow (`i64::MIN / -1`).
- Checked variants do not panic for expected arithmetic failure cases.

Implementation recommendation:

- Prefer small C runtime helpers returning DUUMBI Result pointers if that keeps
  overflow behavior consistent across platforms and avoids relying on
  Cranelift overflow opcode details.
- If using Cranelift-native overflow detection, verify the exact APIs in the
  locally pinned Cranelift version before coding and cover them with unit tests.
- Keep unchecked `Add`, `Sub`, and `Mul` lowering unchanged except for docs that
  define wrapping behavior.
- Update parser, op display/output type logic, graph builder paths, validation,
  and Result safety checks as needed.

### 6. Replace Fixed Struct Allocation With A Layout Registry

Add a struct layout registry before function lowering:

- Key layouts by struct name.
- Derive field names and field value types from `StructNew`, `FieldSet`,
  `FieldGet`, operand/result types, and graph edges that are available at
  compile time.
- Reject conflicting field type evidence with a compile error that names the
  struct and field.
- Compute stable offsets and total size from field order and alignment.
- For this issue, pointer-width storage for all currently supported scalar and
  heap-reference values is acceptable if documented and test-covered. The work
  must remove the fixed 64-byte cap and must not alias fields.
- Preserve simple existing struct fixtures.
- Add a 12+ mixed-field fixture and assert representative reads across early,
  middle, and late fields.

If implementation discovers that current JSON-LD lacks enough structure to
derive layout safely, stop for an architecture decision instead of shipping an
unsafe inferred layout.

### 7. Add Cranelift IR Verification

Add a verifier helper in the compiler backend:

- Run Cranelift verification after `compile_function` finalizes a function and
  before `ObjectModule::define_function` accepts it.
- Use the locally pinned Cranelift API; the currently available crate exposes
  `cranelift_codegen::verify_function`, but Stage 10 must verify exact
  signature and flags usage before coding.
- Enable verifier checks in debug/test builds and CI.
- Make verifier failures return `CompileError::Cranelift` with the DUUMBI
  function name and verifier message.
- Add a focused negative test for malformed generated IR if feasible without
  brittle test-only hooks; otherwise document why verifier coverage is proven by
  positive path execution plus a helper unit test.

### 8. Add Deterministic Interpreter-Vs-Native Differential Tests

Add a small semantics oracle for the deterministic subset needed by #714:

- constants, bools, add/sub/mul/div, checked arithmetic, compare, branch,
  return, print, array new/push/get/set/try-get/length, option constructors,
  `Match`, and struct field get/set.
- Exclude file, network, HTTP, DB, provider, registry, randomness, and live OS
  behavior from the first differential subset.
- Tests compare interpreter and native behavior for stdout, exit status, and
  deterministic DUUMBI panic kind/node id.
- Keep the interpreter either test-only or clearly marked as a semantics oracle;
  do not expose it as a product feature unless separately specified.

### 9. Cross-Compilation Groundwork

Keep this slice intentionally narrow:

- Refactor target triple selection so it can be tested independently from host
  detection.
- If a CLI flag is added, it must clearly be compile-target selection, not a
  promise that host linkers can produce every target binary.
- Document target-triple and linker responsibilities.
- Do not add release artifacts, platform installers, or full cross-target CI in
  this issue.

## Invariants

- Graph op node ids remain stable strings and are the attribution source for
  node-aware runtime failures.
- Existing valid array, struct, Result/Option, and telemetry fixtures continue
  to pass.
- Existing unchecked add/sub/mul behavior remains wrapping and compatibility
  preserving.
- `ArrayTryGet` never panics for an ordinary out-of-bounds index.
- `ArrayGet`, `ArraySet`, and integer `Div` never fail as an un-attributed host
  signal for the accepted trap cases.
- New checked arithmetic variants return `Result`; they must not panic for
  expected overflow/divide-by-zero cases.
- Cranelift types and builder details stay inside `src/compiler/`.
- No `.unwrap()` is added to library code.
- Runtime C changes remain portable across Linux, macOS, and Windows where the
  current runtime already builds.
- CI must not require live provider credentials or external network calls beyond
  normal dependency setup.

## BDD-To-Test Mapping

| Product BDD scenario | Primary verification evidence | Required command or artifact |
|---|---|---|
| Integer division by zero reports a DUUMBI node-attributed panic | Integration test compiles a fixture, runs the binary, asserts nonzero exit and stderr contains panic kind plus node id | `cargo test --test integration_duumbi714_backend_hardening div_zero_node_attribution` |
| ArrayGet out of bounds reports the failing node id | Integration test fixture with one-element array and OOB get; stderr assertion includes `array index out of bounds` and node id | `cargo test --test integration_duumbi714_backend_hardening array_get_oob_node_attribution` |
| ArraySet with a negative index reports the failing node id | Integration test fixture writes index `-1`; stderr assertion includes panic kind and `ArraySet` node id | `cargo test --test integration_duumbi714_backend_hardening array_set_negative_node_attribution` |
| ArrayTryGet returns Some for an in-bounds index | Integration test fixture uses `ArrayTryGet` then `Match`; stdout and exit status prove Some branch | `cargo test --test integration_duumbi714_backend_hardening array_try_get_some_match` |
| ArrayTryGet returns None for an out-of-bounds index | Integration test fixture uses OOB `ArrayTryGet` then `Match`; stdout and exit status prove None branch | `cargo test --test integration_duumbi714_backend_hardening array_try_get_none_match` |
| Checked multiplication returns Err on overflow | Parser/validator/codegen integration test for `MulChecked`; match result chooses Err branch without panic | `cargo test --test integration_duumbi714_backend_hardening checked_mul_overflow_returns_err` |
| Unchecked multiplication preserves documented wrapping behavior | Native fixture plus interpreter comparison confirms documented wrapping result | `cargo test --test integration_duumbi714_backend_hardening unchecked_mul_wrap_policy` |
| A large mixed-type struct preserves field values | Integration fixture with 12+ fields asserts representative field reads and no aliasing | `cargo test --test integration_duumbi714_backend_hardening large_mixed_struct_layout` |
| Cranelift IR verification catches malformed generated IR | Unit or integration test exercises verifier helper; implementation evidence explains any negative-test limitation | `cargo test compiler::` or the focused test name chosen during implementation |
| Interpreter and native execution agree for deterministic fixtures | Differential test suite compares interpreter and native stdout/exit/panic metadata | `cargo test --test integration_duumbi714_backend_hardening differential_interpreter_native_subset` |

Additional evidence:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`
- CI run URL on the final implementation PR
- Manual live E2E logs listed below

## Live E2E Plan

Canonical interface: DUUMBI CLI.

Provider/LLM path:

- This issue does not touch LLM/provider behavior.
- Required external LLM calls: 0.
- Required provider credentials: none.
- Estimated external LLM cost: USD 0.

Manual live E2E commands after implementation:

```bash
cargo build
cargo run -- build --trace tests/fixtures/backend_hardening/div_zero_node_attribution.jsonld -o /tmp/duumbi-714-div-zero
/tmp/duumbi-714-div-zero
cargo run -- build --trace tests/fixtures/backend_hardening/array_get_oob_node_attribution.jsonld -o /tmp/duumbi-714-array-get-oob
/tmp/duumbi-714-array-get-oob
cargo run -- build tests/fixtures/backend_hardening/array_try_get_match_some_none.jsonld -o /tmp/duumbi-714-array-try-get
/tmp/duumbi-714-array-try-get
cargo run -- build tests/fixtures/backend_hardening/large_mixed_struct_layout.jsonld -o /tmp/duumbi-714-struct-layout
/tmp/duumbi-714-struct-layout
```

Pass/fail criteria:

- Div-by-zero command exits nonzero and stderr includes `duumbi panic`,
  `division by zero`, and the failing `Div` node id.
- Array OOB command exits nonzero and stderr includes `duumbi panic`,
  `array index out of bounds`, and the failing array op node id.
- Traced trap runs produce telemetry/crash evidence that includes node
  attribution and still preserves function/block evidence.
- `ArrayTryGet` command exits successfully and prints evidence for both Some
  and None branches, or separate fixtures cover the two branches.
- Large struct command exits successfully and prints/asserts representative
  field values.

Artifacts:

- Terminal transcript or issue comment with command output summary.
- Final implementation PR evidence section with exact commands and results.
- CI URL for the implementation PR.
- Optional trace/crash artifacts paths when telemetry tests produce local
  evidence.

TUI/Studio parity:

- No full TUI or Studio E2E is required because the behavior is backend/CLI
  compiler/runtime behavior.
- If implementation touches CLI error rendering, run a thin smoke check that the
  CLI preserves stderr panic text.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements;
2. propose one bounded implementation goal;
3. list intended file areas and commands;
4. estimate resource use and risk;
5. check whether the resource gate requires human approval;
6. implement only the approved or resource-permitted goal;
7. run the agreed checks;
8. report evidence, failures, and remaining gaps;
9. stop only if requirements are met, a blocker appears, the expected external
   LLM cost of the next cycle exceeds USD 1, or scope changes; iteration count
   is not a stop condition.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Suggested cycle slices:
  1. node-aware panic runtime and telemetry contract;
  2. `Div`, `ArrayGet`, and `ArraySet` trap guards;
  3. `ArrayTryGet` with Option/Match tests;
  4. checked arithmetic ops and docs;
  5. struct layout registry and 12+ field fixture;
  6. Cranelift verifier integration;
  7. interpreter-vs-native differential tests;
  8. cross-compilation groundwork and final evidence cleanup.
- Max files or modules per cycle: prefer 1 to 3 tightly related modules plus
  matching tests/fixtures/docs. Exceeding 5 meaningful source files in one cycle
  requires an explicit rationale in the cycle plan.
- Expected command budget per cycle: focused unit/integration tests for the
  touched area plus format when Rust code changes. Run full clippy and
  all-tests before final PR readiness.
- Human approval is required only when the cycle will use an external LLM with
  expected cost above USD 1, exceeds approved scope, adds risky dependencies or
  irreversible operations, changes public product semantics beyond this spec, or
  needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external model or
  agent CLI calls. Codex internal reasoning usage is covered by the Codex App
  subscription and never triggers the gate.
- No autonomous batch cap: cycles continue until completion, blocker, gate
  breach, or scope change.
- Stop and ask for human guidance when:
  - struct layout cannot be derived safely from current graph data;
  - checked arithmetic cannot fit the existing Result/runtime representation;
  - Cranelift verifier integration requires a broad backend API refactor;
  - cross-compilation groundwork starts turning into full platform release work;
  - any required command fails repeatedly for reasons outside the approved
    implementation scope.

## Task Breakdown

1. Add backend-hardening fixtures for div zero, array OOB, ArrayTryGet
   Some/None, checked arithmetic, unchecked wrap, and large struct layout.
2. Add node-aware runtime panic API and telemetry crash field support.
3. Add Cranelift lowering helpers for node id/message data pointers and guarded
   trap blocks.
4. Guard integer `Div` and array get/set with node-attributed panic paths.
5. Implement `ArrayTryGet` with Option construction and Match-compatible SSA
   joining.
6. Add checked arithmetic op variants across type model, parser, graph builder,
   validator, lowering, runtime helpers, and docs.
7. Add struct layout registry and replace fixed 64-byte allocation.
8. Add Cranelift verifier helper and wire it into debug/test/CI compile paths.
9. Add deterministic interpreter/differential test support for the subset in
   this issue.
10. Add or update backend semantics documentation.
11. Run focused tests after each slice, then full final checks.
12. Consolidate implementation PR evidence against this technical spec.

## Verification Plan

Focused checks during implementation:

- `cargo test --test integration_duumbi714_backend_hardening`
- `cargo test --test integration_phase9a1`
- `cargo test --test integration_phase9a3`
- `cargo test --test integration_telemetry`
- Focused unit tests in modified compiler/parser/validator modules.

Final local checks before implementation PR review:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

CI evidence:

- The final implementation PR must show green required checks.
- If CI uses `cargo nextest run --workspace`, map local `cargo test --all`
  evidence to CI nextest evidence in the PR.

Review evidence:

- Codex self-review of the final implementation diff and evidence.
- Required `@chatgpt-codex-connector` review on the final implementation PR
  before Stage 11 merge readiness.
- Greptile is not invoked during specs or Stage 10; it may be recommended for
  the final implementation PR because this issue touches compiler lowering and
  runtime C, but a human must decide.

## Completion Criteria

Before Stage 10 can route the implementation PR to Stage 11:

- Every product BDD scenario has passing automated or live E2E evidence.
- Div zero, ArrayGet OOB, and ArraySet OOB are deterministic and include the
  failing node id in stderr.
- Traced crash evidence includes node attribution for the new trap cases.
- `ArrayTryGet` returns Some/None correctly and works with `Match`.
- Checked arithmetic variants exist and return Result values for success and
  expected failure cases.
- Existing unchecked add/sub/mul wrapping behavior is documented and tested.
- The 12+ mixed-field struct fixture passes and simple existing struct fixtures
  still pass.
- Cranelift verifier is active in debug/test/CI paths and failures are reported
  as DUUMBI compile errors with useful context.
- Interpreter-vs-native differential tests cover the approved deterministic
  subset.
- Cross-compilation groundwork is documented and does not overclaim full
  target support.
- Final checks and CI pass.
- Implementation evidence includes commands, results, remaining risks, and a
  statement that the execution issue remains open for Stage 11 and Stage 12.

## Failure And Escalation

- If an implementation slice cannot preserve existing passing fixtures, stop
  and report the regression before broadening scope.
- If Cranelift guard blocks introduce verifier failures, fix the lowering
  structure before adding more behavior.
- If runtime C changes behave differently on Linux and Windows, narrow the
  issue to portable behavior and document platform-specific evidence.
- If checked arithmetic needs a different Result payload representation than
  currently available, stop for an architecture decision.
- If differential tests become flaky, reduce the generated/random surface to
  deterministic fixtures first.
- If any cycle would add a new dependency, alter release workflows, or exceed
  the USD 1 external-LLM gate, request human approval before proceeding.

## Open Questions

None blocking for implementation as specified.

Implementation agents must still verify exact Cranelift verifier APIs and the
least invasive checked-arithmetic implementation. Those are technical
verification tasks, not product blockers, unless source inspection proves the
approved behavior cannot be implemented safely inside the current runtime/type
model.
