# DUUMBI-580: Define Phase 13 Self-Healing Scope And First Telemetry Slice - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-580/PRODUCT.md` by adding
the first Phase 13 local runtime failure feedback slice.

The implementation must prove this flow:

```text
opt-in traced build -> traced local run -> controlled runtime failure ->
local trace/crash/mapping artifacts -> graph function/block back-mapping ->
repair-ready evidence boundary
```

This technical spec covers the parent architecture and sequencing for #580. It
does not collapse all child work into one implementation PR. Stage 10 agents
should prefer the existing child issue order:

1. #583 traced build mode and telemetry configuration.
2. #584 function/block trace events.
3. #585 local crash dumps and trace-to-graph mapping artifacts.
4. #586 controlled runtime failure back-mapping evidence.
5. #588 repair-agent input contract from crash evidence.
6. #587 repair patch validation and human-reviewable evidence.

The first four slices are required before any repair-agent behavior is trusted.
Remote telemetry export, Studio telemetry dashboards, production crash ingestion,
hot-swap, broad anomaly detection, autonomous repair loops, and automatic repair
acceptance remain out of scope.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Oz implementation agents when work is routed from Slack or GitHub.
- Specialized Rust/compiler/runtime agents working on Cranelift lowering and C
  runtime support.
- Stage 9 technical spec reviewers.
- Stage 10 tester/reviewer agents verifying telemetry artifacts and failure
  evidence.

## Source Context

- Product spec: `specs/DUUMBI-580/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/580
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4522341871
- Stage 6 product spec PR: https://github.com/hgahub/duumbi/pull/600
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4522085626
- Stage 4 / decomposition context:
  https://github.com/hgahub/duumbi/issues/580#issuecomment-4505735849
- Child issues: #583, #584, #585, #586, #588, #587
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant code verified for Stage 8:

- `src/cli/mod.rs`
  - `Commands::Build` currently accepts `input`, `--output`, and `--offline`.
  - `Commands::Run` runs the already built workspace binary and accepts trailing
    binary arguments.
- `src/main.rs`
  - `run()` dispatches `build`, `run`, `check`, and other CLI commands.
  - `resolve_input()` and `resolve_output()` define workspace-aware defaults.
- `src/cli/commands.rs`
  - `build_with_opts(input, output, offline)` chooses single-file or workspace
    build paths.
  - Single-file builds call `lowering::compile_to_object()` and compile/link
    `runtime/duumbi_runtime.c`.
- `src/workspace.rs`
  - `build_workspace(workspace_root, output, offline)` loads modules and
    dependencies, compiles all module objects, compiles the C runtime, and links.
  - `run_workspace_binary()` runs `.duumbi/build/output` from the workspace root.
- `src/compiler/mod.rs`
  - `CodegenBackend` is the boundary between graph/parser code and Cranelift.
  - Cranelift types must stay inside `src/compiler/`.
- `src/compiler/lowering.rs`
  - `declare_all_runtime_fns()` declares C runtime symbols.
  - `compile_to_object()` and `compile_program()` emit object bytes.
  - `compile_function()` iterates functions, blocks, and graph nodes and has
    access to `FunctionInfo`, block labels, graph `NodeId`, and Cranelift
    `FunctionBuilder`.
  - Runtime calls are already emitted for print, arrays, structs, result,
    option, math, and string operations.
- `runtime/duumbi_runtime.c`
  - `duumbi_panic()` prints `duumbi panic: <message>` to stderr and exits 1.
  - Array, Result, and Option paths call `duumbi_panic()` for runtime failures.
  - No trace hooks, current function/block thread-local state, or crash dump
    artifacts exist.
- `runtime/duumbi_runtime.h`
  - Declares the public C runtime functions used by generated code.
- `src/config.rs`
  - Existing config sections use serde defaults, kebab-case, and explicit
    effective defaults.
  - There is no telemetry config section today.
- `src/hash.rs`
  - SHA-256 hashing utilities are already available in the project, but
    semantic hashing intentionally ignores `@id`, so trace IDs must use a
    separate identity-preserving hash.
- `src/patch.rs`
  - Graph patch operations are atomic over JSON-LD values.
- `src/mcp/tools/graph.rs`
  - MCP graph mutation validates before writing.
- `src/agents/template.rs`
  - A generic Repair agent template exists.
- `src/intent/execute.rs`
  - Existing repair precedent is verifier-failure oriented, not runtime crash
    evidence oriented.
- `crates/duumbi-studio/src/server_fns.rs`
  - Studio build/run server functions call shared workspace build/run helpers.
- `crates/duumbi-studio/src/lib.rs`
  - HTTP `/api/build` and `/api/run` wrap the same backend helpers.

Relevant tests and fixtures verified for Stage 8:

- `tests/integration_phase1.rs`
  - CLI build/check/describe and workspace init/build/run coverage.
- `tests/integration_phase9a1.rs`
  - Heap/runtime fixture build and run patterns.
- `tests/integration_phase9a3.rs`
  - Result/Option parsing, validation, and runtime-adjacent coverage.
- `tests/fixtures/error_handling/option_some_unwrap.jsonld`
  - Successful `OptionSome` + `OptionUnwrap` fixture.
- `tests/fixtures/error_handling/result_ok_unwrap.jsonld`
  - Successful `ResultOk` + `ResultUnwrap` fixture.
- `runtime/duumbi_runtime.c`
  - Existing panic messages provide deterministic failure strings for a
    controlled `OptionNone` + `OptionUnwrap` fixture.

Relevant Obsidian notes:

- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Product Roadmap 2026-05.md`
- `Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 13 - Self-Healing & Telemetry.md`

Verified source facts:

- Default builds currently have no telemetry instrumentation.
- Current runtime panics are visible on stderr but are not persisted as local
  crash evidence.
- Current compiler lowering already has the function/block/node context needed
  to emit function/block trace calls.
- Existing workspace and CLI build paths are separate enough that both must be
  updated or explicitly delegated to shared options.
- Existing Studio build/run paths call shared workspace helpers and should not
  grow a separate telemetry implementation.

Assumptions for implementation:

- The first accepted trace granularity is function/block, not per-operation.
- Stable trace IDs should be derived from graph `@id` values with an
  identity-preserving SHA-256 based hash, not from traversal order.
- Local artifacts can be written relative to the process working directory under
  `.duumbi/telemetry/`, with `DUUMBI_TELEMETRY_DIR` as a test-friendly override.
- `duumbi build --trace` is the canonical opt-in traced build surface for v1.
  `duumbi run` then runs the instrumented binary. A `run --trace` shortcut can
  be deferred unless Stage 10 discovers the CLI UX is too awkward.

## Affected Areas

Expected implementation changes:

- CLI command surface:
  - `src/cli/mod.rs`
  - `src/main.rs`
  - `src/cli/commands.rs`
  - `src/cli/repl.rs` for `/build --trace` parity if the REPL parser supports
    build flags; otherwise document REPL trace support as future work.
- Workspace build/run helpers:
  - `src/workspace.rs`
- Compiler API and lowering:
  - `src/compiler/mod.rs`
  - `src/compiler/lowering.rs`
  - optional new `src/compiler/telemetry.rs` if helper separation is cleaner.
- Telemetry domain module:
  - new `src/telemetry/mod.rs`
  - optional `src/telemetry/artifacts.rs`
  - optional `src/telemetry/ids.rs`
  - optional `src/telemetry/inspect.rs`
- Runtime support:
  - `runtime/duumbi_runtime.c`
  - `runtime/duumbi_runtime.h`
- Config:
  - `src/config.rs`
  - `.duumbi/config.toml` examples produced by `src/cli/init.rs`, only if the
    config example must show telemetry defaults. Do not force new config keys
    into existing workspaces.
- CLI inspection surface:
  - add `duumbi telemetry inspect` or equivalent if artifact inspection is not
    otherwise exposed.
- Tests and fixtures:
  - new telemetry-focused integration tests under `tests/`.
  - new controlled panic fixture under `tests/fixtures/telemetry/`.
  - focused unit tests for config defaults, trace ID determinism, artifact
    parsing, and missing mapping behavior.
- Documentation:
  - minimal docs or command help for traced local telemetry and artifact paths.
  - do not modify the approved product spec.

Areas expected not to change:

- `specs/DUUMBI-580/PRODUCT.md`
- product spec approval labels or comments
- provider/model setup behavior
- Query mode semantics
- Studio telemetry dashboards or graph overlays
- remote telemetry/export code
- registry resolution
- generated artifacts, committed crash files, or committed trace output

CI and local validation paths:

- `cargo fmt --check`
- focused unit tests for telemetry modules and config defaults
- focused compiler/lowering tests for traced and default build behavior
- focused runtime integration tests for crash artifact creation
- `cargo test --test integration_phase1` or equivalent existing build/run
  regression coverage
- `cargo test --all` before implementation PR review when compiler/runtime or
  shared workspace helpers change

## Technical Approach

### 1. Add Explicit Build Options Without Changing Defaults

Introduce build options that are shared by CLI, REPL, workspace helpers, and
Studio:

```rust
pub struct BuildOptions {
    pub offline: bool,
    pub telemetry: TelemetryBuildMode,
}

pub enum TelemetryBuildMode {
    Off,
    Trace,
}
```

Recommended placement:

- Put user-facing telemetry config in `src/telemetry`.
- Re-export only small option types through `src/workspace.rs` or
  `src/compiler/mod.rs` when needed.
- Keep Cranelift-specific types inside `src/compiler/`.

Required behavior:

- Existing `duumbi build` remains equivalent to `TelemetryBuildMode::Off`.
- New `duumbi build --trace` selects `TelemetryBuildMode::Trace`.
- `--trace` must be opt-in and must not be implied by `[telemetry] enabled =
  true` unless Stage 10 explicitly adds a separate user confirmation rule.
- `duumbi run` runs whatever binary was built. It should not silently rebuild.
- Studio build should keep default uninstrumented behavior unless a later UI
  issue explicitly adds a trace toggle.

Rejected alternatives:

- Always-on instrumentation.
- Per-operation tracing for v1.
- A separate Studio-only telemetry path.
- Relying on compiler traversal order for trace IDs.

### 2. Add Conservative Telemetry Configuration

Add a `[telemetry]` config section with serde defaults. The first implementation
should read it only when trace mode is explicitly selected.

Recommended v1 shape:

```toml
[telemetry]
enabled = false
artifact-dir = ".duumbi/telemetry"
capture-values = false
```

Rules:

- `enabled = false` is the default.
- `artifact-dir` is interpreted relative to the workspace root or process
  current directory.
- `DUUMBI_TELEMETRY_DIR` overrides `artifact-dir` for tests and one-off runs.
- `capture-values` stays false in v1; if implemented, argument/value capture
  must be opt-in and redacted in tests.
- Do not add remote endpoint fields in this issue.

### 3. Generate Stable Trace IDs And Mapping Artifacts

Add an identity-preserving trace ID helper. Do not reuse `semantic_hash()` for
trace IDs because semantic hashes intentionally ignore `@id`.

Recommended algorithm:

```text
trace_id = first 63 bits of SHA-256("duumbi-trace-v1\0" + kind + "\0" + graph_id)
```

Where:

- `kind` is `function` or `block`.
- `graph_id` is the exact graph `@id`.
- The high bit is cleared so the value fits in signed `int64_t`.
- Collisions are checked within one compiled program. Any collision is a
  compiler error with both graph IDs in the diagnostic.

Mapping artifact:

```json
{
  "schema_version": "duumbi.telemetry.trace_map.v1",
  "program_hash": "<optional graph hash or build id>",
  "entries": [
    {
      "trace_id": 123,
      "kind": "function",
      "graph_id": "duumbi:main/main",
      "module": "main",
      "function": "main"
    },
    {
      "trace_id": 456,
      "kind": "block",
      "graph_id": "duumbi:main/main/entry",
      "module": "main",
      "function": "main",
      "block": "entry"
    }
  ]
}
```

Implementation guidance:

- For single-file builds, emit a sidecar map next to the output binary or into
  the telemetry artifact directory.
- For workspace builds, emit one combined map for all compiled modules.
- Sort mapping entries by `kind`, then `graph_id` for deterministic tests.
- Do not compile absolute workspace paths into the binary.

### 4. Add Runtime Trace Hook ABI

Extend `RuntimeFuncs` and `declare_all_runtime_fns()` with trace functions, but
emit calls only in traced mode.

Recommended C ABI:

```c
void duumbi_trace_init(void);
void duumbi_trace_function_enter(int64_t function_id);
void duumbi_trace_function_exit(int64_t function_id);
void duumbi_trace_block_enter(int64_t block_id);
void duumbi_trace_block_exit(int64_t block_id);
void duumbi_trace_panic(const char *msg);
```

Runtime state:

- Thread-local current function ID.
- Thread-local current block ID.
- Trace active flag initialized by `duumbi_trace_init()`.
- File paths resolved from `DUUMBI_TELEMETRY_DIR` or `.duumbi/telemetry`.

Trace event JSONL shape:

```json
{"schema_version":"duumbi.telemetry.trace.v1","event":"function_enter","trace_id":123,"timestamp_ns":0}
{"schema_version":"duumbi.telemetry.trace.v1","event":"block_enter","trace_id":456,"timestamp_ns":0}
{"schema_version":"duumbi.telemetry.trace.v1","event":"panic","function_id":123,"block_id":456,"message":"called Option::unwrap() on a None value"}
```

Testing rule:

- Runtime may include nondeterministic `timestamp_ns`, `pid`, or monotonic
  sequence fields.
- Tests must assert schema, event names, IDs, and graph linkage, not exact
  timestamps.

### 5. Instrument Function And Block Boundaries In Lowering

In traced mode:

- Call `duumbi_trace_init()` once at the beginning of `main`.
- Emit `duumbi_trace_function_enter(function_id)` at function entry.
- Emit `duumbi_trace_function_exit(function_id)` before each normal return.
- Emit `duumbi_trace_block_enter(block_id)` when switching into each block.
- Emit `duumbi_trace_block_exit(block_id)` before terminators when possible.
- If exact block exit instrumentation is difficult for branch-heavy control
  flow, function/block enter and crash-time current block are required for v1;
  block exit completeness can be a non-blocking enhancement.

Important boundary:

- Do not instrument every operation.
- Do not make trace calls part of default object emission.
- Do not let trace instrumentation change SSA dominance, terminator placement,
  return values, or existing runtime semantics.

### 6. Persist Crash Evidence From Runtime Panic Paths

Change `duumbi_panic()` so traced runs write crash evidence before printing the
existing stderr message and exiting.

Crash artifact JSONL shape:

```json
{
  "schema_version": "duumbi.telemetry.crash.v1",
  "event": "panic",
  "message": "called Option::unwrap() on a None value",
  "function_id": 123,
  "block_id": 456,
  "trace_active": true
}
```

Rules:

- Preserve the existing stderr prefix `duumbi panic:`.
- Preserve nonzero process exit.
- Do not suppress or replace the original failure.
- If artifact writing fails, print a secondary telemetry warning to stderr and
  still exit with the original panic behavior.
- Do not capture argument values by default.

### 7. Add Back-Mapping Inspection

Add a small inspection layer so users and tests can ask whether crash evidence
maps back to graph context.

Recommended CLI:

```text
duumbi telemetry inspect [--telemetry-dir <path>] [--crash <path>] [--map <path>]
```

Behavior:

- Reads crash artifact and trace map.
- Prints a concise human-readable summary.
- Returns success when crash evidence maps to a known function/block.
- Returns failure with explicit missing-evidence output when map or IDs are
  missing/malformed.
- Does not mutate graph files.
- Does not call an LLM.

Suggested output shape:

```text
Crash: called Option::unwrap() on a None value
Function: duumbi:main/main
Block: duumbi:main/main/entry
Node: unavailable in v1
Repair readiness: crash evidence mapped; repair context not generated
```

### 8. Add Controlled Failure Fixture

Add a deterministic fixture under `tests/fixtures/telemetry/`, such as:

- `option_none_unwrap.jsonld`

Fixture behavior:

- Validates successfully.
- Creates `OptionNone`.
- Calls `OptionIsSome` before `OptionUnwrap` so existing result-safety warning
  checks do not block compilation.
- Calls `OptionUnwrap` and triggers runtime panic
  `called Option::unwrap() on a None value`.

Integration test flow:

1. Build the fixture with `duumbi build --trace`.
2. Run the resulting binary with `DUUMBI_TELEMETRY_DIR` pointing at a temp dir.
3. Assert the process exits nonzero and stderr includes the original panic.
4. Assert `traces.jsonl`, `crash_dump.jsonl`, and `trace_map.json` exist.
5. Assert crash IDs map to `duumbi:<module>/main` and
   `duumbi:<module>/main/entry`.
6. Run `duumbi telemetry inspect` and assert function/block summary plus
   `Node: unavailable in v1`.

### 9. Define Repair Context After Crash Evidence Exists

After #583-#586 pass, #588 may add a repair context type. Recommended shape:

```rust
pub struct RepairCrashContext {
    pub crash_message: String,
    pub function_id: String,
    pub block_id: String,
    pub exact_node_id: Option<String>,
    pub trace_ids: TraceCorrelation,
    pub graph_context: serde_json::Value,
    pub validation_expectations: Vec<String>,
    pub test_expectations: Vec<String>,
}
```

Rules:

- The context must come from mapped crash evidence, not raw logs alone.
- It may include the full faulty block and containing function.
- It must not apply patches.
- It must not mark repairs accepted.
- It should be serializable for test fixtures and review evidence.

### 10. Define Repair Validation Evidence Separately

After #588 is accepted, #587 may add repair validation. It must reuse existing
graph validation/build/test primitives and produce evidence before human review.

Required gates:

- Proposed patch parses as a `GraphPatch`.
- Patch application is atomic.
- Graph parsing/building/validation passes.
- Native rebuild passes.
- Relevant tests pass.
- Evidence report links original crash artifact, mapped graph context, proposed
  graph changes, validation output, build output, and test output.
- No patch is silently accepted or applied as complete without human review.

## Invariants

- Existing `duumbi build` output and runtime behavior remain unchanged unless
  `--trace` is explicitly selected.
- Traced telemetry never mutates `.duumbi/graph`.
- Cranelift types stay inside `src/compiler/`.
- Graph IDs in artifacts are original JSON-LD `@id` values.
- Runtime C must remain buildable by the existing `cc` linker path.
- Telemetry artifact write failures must not hide the original runtime failure.
- Local telemetry must not require provider credentials, Studio, network access,
  external telemetry collectors, or remote services.
- Query mode remains read-only.
- Repair candidates remain gated behind validation, rebuild, tests, and human
  review.
- No implementation PR may claim production self-healing from local telemetry
  evidence alone.

## BDD-To-Test Mapping

| Product BDD Scenario | Verification Evidence |
|---|---|
| Building without traced telemetry | Add a focused compiler/CLI integration test that builds a known fixture with default `duumbi build`, runs it, asserts no telemetry artifacts are required, and optionally asserts object/runtime output does not reference trace hook calls. Keep existing `tests/integration_phase1.rs` build/run coverage green. |
| Enabling traced local execution | Add a telemetry integration test that builds a valid fixture with `duumbi build --trace`, runs the binary with `DUUMBI_TELEMETRY_DIR` set to a temp dir, and asserts `traces.jsonl` and `trace_map.json` contain function/block events with stable IDs. |
| Controlled runtime failure writes crash evidence | Add `tests/fixtures/telemetry/option_none_unwrap.jsonld`; build with `--trace`, run, assert nonzero exit, stderr keeps `duumbi panic: called Option::unwrap() on a None value`, and `crash_dump.jsonl` contains message plus trace correlation. |
| Crash evidence maps to graph function and block context | Add unit tests for trace-map lookup and an integration assertion through `duumbi telemetry inspect` showing `Function: duumbi:<module>/main` and `Block: duumbi:<module>/main/entry`. |
| Missing mapping prevents repair readiness | Add an integration test that removes or corrupts `trace_map.json` after a traced crash, runs `duumbi telemetry inspect`, expects a nonzero status and explicit missing-evidence output, and confirms raw crash evidence remains readable. |
| Repair-agent context is built from crash evidence | After #588, add unit tests converting a crash artifact plus trace map into `RepairCrashContext`; assert failure reason, graph IDs, trace IDs, graph context, and validation/test expectations are present. No provider call is required for contract tests. |
| Repair candidate requires validation and human review | After #587, add tests for valid and invalid repair candidates; assert graph validation, rebuild, relevant tests, and evidence report are required before reporting success. Assert no automatic acceptance state is written. |
| Production telemetry is requested before local proof exists | Add review evidence in implementation PRs and docs/help text showing remote export, production ingestion, Studio dashboard, and hot-swap are absent. If a CLI flag or config tries to add remote endpoint fields, Stage 10 must stop for scope review. |
| Exact node-level mapping is unavailable in the first slice | Add `duumbi telemetry inspect` assertion that function/block mapping is reported and exact node context is shown as unavailable unless a later accepted issue adds exact-node evidence. |

## Live E2E Plan

Canonical interface: CLI.

Telemetry and back-mapping E2E:

- Provider/LLM path: none for #583-#586.
- Required credentials: none.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Required environment:
  - working Rust toolchain
  - C compiler available through `$CC` or `cc`
  - writable temp directory
- Commands after implementation:

```bash
cargo build
tmp="$(mktemp -d)"
target/debug/duumbi build --trace tests/fixtures/telemetry/option_none_unwrap.jsonld -o "$tmp/panic-fixture"
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" "$tmp/panic-fixture"
target/debug/duumbi telemetry inspect --telemetry-dir "$tmp/telemetry"
```

Expected result:

- The traced binary exits nonzero.
- Stderr includes `duumbi panic: called Option::unwrap() on a None value`.
- `$tmp/telemetry/traces.jsonl` exists.
- `$tmp/telemetry/crash_dump.jsonl` exists.
- `$tmp/telemetry/trace_map.json` exists.
- `duumbi telemetry inspect` prints mapped function and block graph IDs.
- `duumbi telemetry inspect` does not claim exact node-level evidence in v1.

Workspace E2E:

```bash
tmp="$(mktemp -d)"
target/debug/duumbi init "$tmp/ws"
cp tests/fixtures/telemetry/option_none_unwrap.jsonld "$tmp/ws/.duumbi/graph/main.jsonld"
(cd "$tmp/ws" && target/debug/duumbi build --trace)
(cd "$tmp/ws" && DUUMBI_TELEMETRY_DIR=".duumbi/telemetry" target/debug/duumbi run)
(cd "$tmp/ws" && target/debug/duumbi telemetry inspect)
```

Pass/fail criteria:

- Default build path remains green in existing tests.
- Traced workspace path writes artifacts under `.duumbi/telemetry`.
- Runtime panic remains visible and nonzero.
- Back-mapping succeeds to function/block.
- No provider setup prompt or LLM call occurs.

Repair-context live LLM E2E for #588/#587 only:

- Required only if implementation adds a provider-backed repair proposal path.
- Canonical interface should be CLI, for example a future
  `duumbi repair propose --from-crash <path> --dry-run` command.
- Provider path: existing configured provider from `/provider` or
  `duumbi provider`.
- Credentials: one of the existing provider environment variables such as
  `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `OPENROUTER_API_KEY`,
  `XAI_API_KEY`, or `MINIMAX_API_KEY`.
- Expected external LLM calls: 1 for one repair proposal dry run.
- Estimated external LLM cost: under USD 0.50.
- The command must not apply the patch.
- The proposal must cite crash evidence, function/block graph IDs, validation
  expectations, and test expectations.
- This live provider path is not required for #583-#586.

Studio/TUI parity:

- No full Studio E2E is required for #583-#586 because no UI-specific behavior
  is accepted.
- If shared workspace build options change, add a thin Studio/API smoke check
  proving default Studio build remains uninstrumented.
- If REPL `/build --trace` is implemented, add a manual smoke or focused parser
  test proving it delegates to the same backend options.

## Ralph Cycle Protocol

Each Ralph cycle must:

1. Summarize current state and remaining unmet requirements.
2. Propose one bounded implementation goal.
3. List intended file areas and commands before editing.
4. Estimate resource use, external LLM calls, cost, and risk.
5. Check whether the resource gate requires human approval.
6. Implement only the approved or resource-permitted goal.
7. Run the agreed checks.
8. Report evidence, failures, and remaining gaps.
9. Stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 4 files or 1 cohesive module cluster.
- Expected command budget per cycle:
  - `cargo fmt --check`
  - 1-3 focused `cargo test` commands for touched areas
  - `cargo test --all` only when shared compiler/runtime/workspace behavior is
    ready for broad verification or before final implementation review
- Expected external LLM calls:
  - #583-#586: 0
  - #588/#587 repair proposal dry-run, if implemented: 1-2
- Expected external LLM cost:
  - #583-#586: USD 0
  - #588/#587 dry-run, if implemented: under USD 0.50
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies, changes production
  telemetry scope, adds remote export, changes repair acceptance semantics, or
  needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: three low-budget cycles before stopping for an evidence
  report and human review.
- Stop and ask for human guidance when:
  - trace IDs cannot be made deterministic without changing graph identifiers
  - runtime artifact writes require unsafe global paths
  - exact node-level mapping becomes necessary to satisfy a test
  - repair automation would apply patches automatically
  - remote telemetry or production ingestion is requested
  - tests require more than the approved command budget or LLM budget

## Task Breakdown

1. Telemetry model and config:
   - Add `src/telemetry` with config-independent domain types.
   - Add `[telemetry]` config defaults in `src/config.rs`.
   - Add unit tests for defaults and env override behavior.

2. Build option plumbing:
   - Add build options to CLI and workspace helpers.
   - Add `duumbi build --trace`.
   - Preserve existing `duumbi build`, workspace build, and Studio build
     defaults.

3. Trace ID and mapping generation:
   - Add identity-preserving trace ID hashing.
   - Generate function/block mapping entries from graph `@id` values.
   - Write deterministic `trace_map.json`.
   - Add collision tests and stable-output tests.

4. Runtime ABI:
   - Add trace function declarations to runtime header.
   - Add trace runtime implementation in C.
   - Preserve existing panic stderr and exit behavior.
   - Add runtime-level tests through compiled fixtures.

5. Compiler instrumentation:
   - Declare trace runtime functions.
   - Emit trace calls only in traced mode.
   - Instrument function/block enter and normal function returns.
   - Keep default compilation output uninstrumented.

6. Artifact inspection:
   - Add artifact parsers and mapping lookup.
   - Add `duumbi telemetry inspect` or equivalent.
   - Cover valid, missing, malformed, and unmapped artifact states.

7. Controlled failure proof:
   - Add `tests/fixtures/telemetry/option_none_unwrap.jsonld`.
   - Add CLI integration test for traced crash evidence.
   - Add workspace integration test if single-file coverage does not exercise
     `.duumbi/telemetry`.

8. Repair context contract:
   - Add serializable `RepairCrashContext`.
   - Convert mapped crash evidence into context.
   - Keep provider calls and patch application out unless a later accepted child
     issue authorizes them.

9. Repair validation evidence:
   - Define evidence report structures.
   - Reuse graph patch validation, rebuild, and tests.
   - Require human-reviewable output and no silent acceptance.

## Verification Plan

Required focused checks:

- `cargo fmt --check`
- Telemetry unit tests:
  - config defaults
  - env override path resolution
  - trace ID determinism
  - trace ID collision detection
  - trace map serialization order
  - crash/map parsing
  - missing/malformed mapping behavior
- Compiler/lowering tests:
  - default build does not emit trace calls
  - traced build emits function/block trace calls
  - branch/return paths remain valid Cranelift
- Runtime tests:
  - trace hooks create local artifacts in temp dir
  - `duumbi_panic()` writes crash evidence when tracing is active
  - panic stderr and exit code remain unchanged
- CLI integration tests:
  - `duumbi build` default path remains green
  - `duumbi build --trace` creates map sidecar/artifacts
  - traced controlled failure produces crash evidence
  - `duumbi telemetry inspect` maps function/block
  - corrupt or missing map produces missing-evidence output
- Regression checks:
  - `cargo test --test integration_phase1`
  - existing Phase 9a runtime tests relevant to arrays/result/option
  - `cargo test --all` before final implementation PR review

Manual/review evidence required:

- The exact traced E2E commands run.
- Paths to generated temp artifacts or sanitized excerpts.
- Crash message and mapped function/block IDs.
- Confirmation that default build output does not require telemetry artifacts.
- Confirmation that no remote telemetry endpoint or Studio dashboard was added.
- Confirmation that repair work, if present, stops at reviewable evidence.

## Completion Criteria

Stage 10 implementation work for #580 is complete only when:

- `duumbi build` remains uninstrumented by default.
- `duumbi build --trace` exists and is documented in CLI help or minimal docs.
- Traced builds emit function/block trace calls with stable graph-linked IDs.
- Traced runs write local trace artifacts without remote services.
- Runtime panics in traced mode write crash evidence and preserve original
  stderr/exit behavior.
- Trace-to-graph mapping artifacts are deterministic and inspectable.
- `duumbi telemetry inspect` or equivalent maps controlled crash evidence to
  graph function/block IDs.
- Missing/malformed mapping prevents repair readiness and reports explicit
  missing evidence.
- Exact node-level context is shown as unavailable unless real evidence exists.
- The controlled failure fixture proves the product BDD flow.
- Repair-context and repair-validation slices, when implemented, remain gated
  behind crash evidence, validation, rebuild, tests, and human review.
- Required tests and E2E evidence are included in the implementation PR.

## Failure And Escalation

If tests fail:

- Keep changes scoped to the current Ralph cycle.
- Report the failing command, failure summary, and likely affected boundary.
- Fix only within the approved file/module area unless the failure proves the
  cycle scope was wrong.

If trace IDs collide:

- Stop the cycle and report the two graph IDs and hash inputs.
- Do not fall back to traversal order without human approval.

If runtime artifact writes are unreliable:

- Preserve panic behavior first.
- Report artifact error handling evidence.
- Do not mask the original runtime failure.

If exact node-level mapping appears necessary:

- Stop and ask for product/architecture guidance.
- Do not expand v1 from function/block mapping to per-operation tracing without
  approval.

If repair scope expands:

- Stop before adding autonomous retry loops, auto-apply behavior, remote
  telemetry, production ingestion, hot-swap, or Studio dashboards.

If external LLM usage would exceed budget:

- Stop before making the call.
- Report estimated calls, provider, cost, and reason.

## Open Questions

No blocking technical questions for Stage 9 review.

Non-blocking implementation questions:

- Should REPL `/build` support `--trace` in the first implementation, or is CLI
  `duumbi build --trace` enough for v1?
- Should the trace map be named `trace_map.json` or `mapping.json` under
  `.duumbi/telemetry/`?
- Should workspace traced builds write one combined map or one map per module
  plus an index? This spec recommends one combined map.
- Should `duumbi telemetry inspect` be the final command name, or should Stage
  9 prefer `duumbi telemetry describe-crash` for clarity?
- Should argument/value capture remain absent in v1 or exist as an opt-in
  `capture-values = true` path with explicit redaction?
