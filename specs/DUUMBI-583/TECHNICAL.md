# DUUMBI-583: Define Traced Build Mode And Telemetry Configuration - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-583/PRODUCT.md` by adding
the first Phase 13 build/config surface:

```text
default build remains uninstrumented
explicit traced build selection -> local-only telemetry settings -> testable traced build mode
```

This technical spec covers only #583. It must make `duumbi build --trace`
accepted and testable, define and validate conservative local telemetry
configuration, and thread an explicit trace build mode through the shared build
stack so later #584 and #585 work can add trace events and local artifacts
without redesigning the user-facing contract.

This issue does not emit function/block trace events, write crash dumps, add
remote export, add Studio telemetry UI, execute repair agents, or run Ralph
cycles.

## Agent Audience

- Codex implementation agents running bounded Stage 10 Ralph cycles.
- Oz implementation agents routing from GitHub or Slack.
- Rust CLI/config/compiler agents implementing the build mode and config
  plumbing.
- Stage 9 technical spec reviewers checking feasibility and scope boundaries.
- Stage 10 tester/reviewer agents verifying build/config behavior before later
  Phase 13 telemetry slices.

## Source Context

- Product spec: `specs/DUUMBI-583/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/583
- Product spec PR: https://github.com/hgahub/duumbi/pull/609
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/583#issuecomment-4524938355
- Stage 6 artifact link:
  https://github.com/hgahub/duumbi/issues/583#issuecomment-4524856551
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/583#issuecomment-4522522582
- Stage 4 triage:
  https://github.com/hgahub/duumbi/issues/583#issuecomment-4522470830
- Parent product spec: `specs/DUUMBI-580/PRODUCT.md`
- Parent technical sequencing context: `specs/DUUMBI-580/TECHNICAL.md`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant verified source facts:

- `src/cli/mod.rs`
  - `Commands::Build` currently accepts `input`, `--output`, and `--offline`.
  - No `--trace` build flag exists.
  - `Commands::Run` runs the already-built binary and accepts trailing binary
    args.
- `src/main.rs`
  - `run()` dispatches `Commands::Build` through
    `cli::commands::build_with_opts(&input_path, &output_path, offline)`.
  - `run()` dispatches `Commands::Run` through `workspace::run_workspace_binary`
    for workspaces and does not rebuild.
  - `resolve_input()` defaults to `.duumbi/graph/main.jsonld` when present.
  - `resolve_output()` defaults to `.duumbi/build/output` when `.duumbi/build`
    exists, otherwise `output`.
- `src/cli/commands.rs`
  - `build_with_opts(input, output, offline)` detects workspace graph inputs and
    delegates to `build_workspace_program`.
  - Single-file builds call `lowering::compile_to_object()`, compile the C
    runtime, and link the binary.
- `src/workspace.rs`
  - `build_workspace(workspace_root, output, offline)` loads modules through
    `deps::load_program_with_deps_opts`, compiles all module objects through
    `lowering::compile_program()`, compiles `runtime/duumbi_runtime.c`, and
    links.
  - `run_workspace_binary()` executes the existing workspace output binary.
- `src/workflow.rs`
  - `workflow::build_workspace(workspace)` wraps
    `workspace::build_workspace(workspace, &output, false)` for REPL and Studio.
- `src/cli/repl.rs`
  - `/build` calls `workflow::build_workspace(&app.workspace_root)`.
- `crates/duumbi-studio/src/server_fns.rs`
  - `build_workspace_for_api()` calls `duumbi::workflow::build_workspace(&root)`
    through `spawn_blocking`.
- `crates/duumbi-studio/src/lib.rs`
  - `POST /api/build` calls the same Studio server build helper.
- `src/config.rs`
  - Existing sections use serde defaults, kebab-case names, and explicit merge
    behavior in `merge_non_provider_fields`.
  - `load_config(workspace_root)` reads only `<workspace>/.duumbi/config.toml`.
  - `load_effective_config(workspace_root)` merges system, user, and workspace
    config.
  - There is no telemetry config section today.
- `src/compiler/mod.rs`
  - `CodegenBackend` is the boundary for compiler API.
  - Cranelift types must stay inside `src/compiler/`.
- `src/compiler/lowering.rs`
  - `compile_to_object(graph)` and `compile_program(program)` do not accept
    build options today.
  - `compile_to_object_impl()` is the shared single-module object emission path.
  - `compile_function()` iterates functions, blocks, and graph nodes and has the
    context later #584 needs for function/block instrumentation.
- `runtime/duumbi_runtime.c`
  - `duumbi_panic()` prints `duumbi panic: <message>` to stderr and exits 1.
  - There are no trace hooks or crash artifact writes today.
- `runtime/duumbi_runtime.h`
  - Exists as the runtime header surface, but current build helpers compile the
    C source directly.
- `src/cli/init.rs`
  - `duumbi init` already creates `.duumbi/telemetry/`.
  - The generated config does not include a telemetry section today.
- `docs/architecture.md`
  - Lists `.duumbi/telemetry/traces.jsonl` as runtime trace mapping data.
- `tests/integration_phase1.rs`
  - Exercises CLI `duumbi build`, workspace init/build/run, and default build
    behavior.
- `tests/integration_phase9a1.rs`
  - Exercises `duumbi build` on runtime-heavy fixtures.
- `tests/integration_phase9a3.rs`
  - Compiles error-handling fixtures through `lowering::compile_to_object()`.
- `src/config.rs` unit tests
  - Cover config parsing, defaults, merge behavior, save/load, and invalid TOML.

Relevant Obsidian notes:

- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 13 - Self-Healing & Telemetry.md`

## Affected Areas

Expected implementation changes:

- CLI command surface:
  - `src/cli/mod.rs`
  - `src/main.rs`
- Shared build helpers:
  - `src/cli/commands.rs`
  - `src/workspace.rs`
  - `src/workflow.rs`
- Config and telemetry domain:
  - `src/config.rs`
  - new `src/telemetry/mod.rs`
  - optional `src/telemetry/config.rs`
  - optional `src/telemetry/path.rs`
  - `src/lib.rs` and `src/main.rs` module declarations if a new telemetry
    module is added.
- Compiler API plumbing:
  - `src/compiler/mod.rs`
  - `src/compiler/lowering.rs`
- REPL and Studio parity:
  - `src/cli/repl.rs`
  - `crates/duumbi-studio/src/server_fns.rs`
  - `crates/duumbi-studio/src/lib.rs`
- Workspace initialization:
  - `src/cli/init.rs` only if implementation adds an explicit commented or
    serialized telemetry config example. Do not force telemetry keys into every
    workspace unless Stage 10 chooses that product option and tests it.
- Tests and fixtures:
  - `src/config.rs` unit tests.
  - new focused telemetry unit tests under `src/telemetry/` if that module is
    added.
  - `tests/integration_phase1.rs` or a new focused integration test file for
    CLI traced build, default build, workspace build, and invalid config.
  - existing fixtures such as `tests/fixtures/hello.jsonld` or the workspace
    skeleton generated by `duumbi init`.
- Docs/help:
  - CLI help text generated by clap.
  - Optional docs update only if the implementation adds user-facing command
    docs beyond clap help.

Areas expected not to change for #583:

- `specs/DUUMBI-583/PRODUCT.md`
- `specs/DUUMBI-580/PRODUCT.md`
- `specs/DUUMBI-580/TECHNICAL.md`
- Implementation behavior for #584 function/block trace events.
- Local crash dumps or trace-to-graph mapping artifacts for #585/#586.
- `runtime/duumbi_runtime.c` and `runtime/duumbi_runtime.h`, unless Stage 10
  discovers a compile/link requirement that cannot be avoided without no-op
  runtime declarations. Any runtime change must remain no-op for default builds
  and must not write artifacts in #583.
- Studio telemetry dashboards, graph overlays, or UI controls.
- Provider/model setup behavior.
- Query mode behavior.
- Registry/dependency resolution except preserving existing `--offline`.
- Generated telemetry output committed to the repo.

## Technical Approach

### 1. Add Explicit Telemetry Build Mode

Add a small build-mode type that is independent of Cranelift internals:

```rust
pub enum TelemetryBuildMode {
    Off,
    Trace,
}
```

Preferred location:

- `src/telemetry/mod.rs` if the implementation adds the telemetry domain module
  immediately.
- Re-export through `src/workspace.rs` or `src/compiler/mod.rs` only when needed
  by public APIs.

Required behavior:

- Existing `duumbi build` selects `TelemetryBuildMode::Off`.
- New `duumbi build --trace` selects `TelemetryBuildMode::Trace`.
- `[telemetry] enabled = true` never implies `TelemetryBuildMode::Trace`.
- `duumbi run` keeps running whatever binary already exists and must not
  silently rebuild.
- REPL `/build` and Studio `/api/build` keep default uninstrumented behavior
  unless later work explicitly adds trace toggles.

Rejected alternatives:

- Always-on instrumentation.
- `duumbi run --trace` in #583.
- Studio-only trace behavior.
- Treating `[telemetry] enabled = true` as a hidden build-mode selector.

### 2. Define Telemetry Configuration

Add a `[telemetry]` section to `src/config.rs` with serde defaults and
kebab-case TOML names.

Recommended v1 shape:

```toml
[telemetry]
enabled = false
sampling-mode = "deterministic"
sample-rate = 0.0
artifact-dir = ".duumbi/telemetry"
capture-values = false
```

Required types and validation:

- `TelemetrySection`
  - `enabled: bool`, default `false`.
  - `sampling_mode: TelemetrySamplingMode`, default `Deterministic`.
  - `sample_rate: f64`, default `0.0`.
  - `artifact_dir: PathBuf` or `String`, default `.duumbi/telemetry`.
  - `capture_values: bool`, default `false`.
- `TelemetrySamplingMode`
  - minimum accepted values: `deterministic` and `probabilistic`.
  - implementation may also accept `disabled` if it maps cleanly to
    `enabled = false`, but must avoid two conflicting disable mechanisms in user
    messages.
- `sample_rate`
  - valid range: `0.0..=1.0`.
  - reject NaN, infinity, negative values, and values above `1.0`.
  - `0.0` is the conservative default for #583 because this issue does not emit
    trace events yet.
- `artifact-dir`
  - default `.duumbi/telemetry`.
  - relative paths resolve against the workspace root for workspace builds and
    against the current working directory for explicit single-file builds.
  - absolute paths are allowed only when explicitly configured by the user or
    provided by `DUUMBI_TELEMETRY_DIR`.
  - reject empty paths and paths containing parent traversal components (`..`).
  - create no artifact files in #583.
- `DUUMBI_TELEMETRY_DIR`
  - overrides `artifact-dir` for traced builds, tests, and one-off runs.
  - follows the same empty/path traversal rejection rules.
- `capture-values`
  - parsed and saved for compatibility with the parent surface.
  - must remain `false` in #583.
  - if set to `true` for a traced build, fail with an actionable error saying
    value capture is out of scope for this issue.
  - default uninstrumented builds must not fail solely because
    `capture-values = true` is present.

Validation boundary:

- Default uninstrumented build paths do not validate telemetry-specific fields.
- Traced builds validate telemetry config before compilation.
- Invalid global TOML syntax can still fail wherever existing config loading
  already parses the file. The #583 product invariant is narrower: telemetry
  semantic validation must not make default uninstrumented builds fail.

Config layering:

- Add `telemetry: Option<TelemetrySection>` to `DuumbiConfig`.
- Extend `merge_non_provider_fields()` so workspace telemetry overrides user and
  system telemetry when present.
- Use effective config for traced workspace builds when possible. If single-file
  builds do not have a workspace root, use current directory as the config root
  and treat missing config as defaults.

### 3. Add Build Options And Thread Them Through Shared APIs

Avoid adding loose booleans to multiple layers. Prefer a small options struct:

```rust
pub struct BuildOptions {
    pub offline: bool,
    pub telemetry_mode: TelemetryBuildMode,
}
```

Implementation guidance:

- Keep existing `build_with_opts(input, output, offline)` as a compatibility
  wrapper if many tests or callers depend on it.
- Add `build_with_build_options(input, output, options)` or similar for the new
  path.
- Add `WorkspaceBuildOptions` or reuse `BuildOptions` for
  `workspace::build_workspace`.
- Preserve existing public `workspace::build_workspace(workspace, output,
  offline)` as a default-off wrapper if external callers or tests use it.
- Update `workflow::build_workspace(workspace)` to keep default-off behavior.
  Add a traced workflow helper only if needed by CLI implementation; do not add
  Studio UI trace controls in #583.
- Update `src/main.rs` build dispatch to pass `TelemetryBuildMode::Trace` when
  `--trace` is present.
- Preserve `--offline` dependency semantics exactly. It is not a telemetry
  network-control flag because telemetry is local-only in #583.

### 4. Keep Compiler API Ready But No-Op For #583 Instrumentation

The compiler needs an explicit trace-mode path so #584 can add
function/block event emission without another API redesign.

Preferred shape:

```rust
pub struct CodegenOptions {
    pub telemetry_mode: TelemetryBuildMode,
}
```

Implementation guidance:

- Keep existing `lowering::compile_to_object(graph)` and
  `lowering::compile_program(program)` as wrappers using `TelemetryBuildMode::Off`.
- Add option-aware functions such as:
  - `compile_to_object_with_options(graph, &CodegenOptions)`
  - `compile_program_with_options(program, &CodegenOptions)`
- Update `CodegenBackend` and `CraneliftBackend` only if implementation agents
  need the trait boundary for shared build paths. If changed, preserve default
  off behavior.
- `TelemetryBuildMode::Trace` in #583 must not emit function/block trace calls.
  It should validate and carry explicit mode metadata through the build stack.
- Do not add runtime trace C symbols unless Stage 10 proves link behavior
  requires no-op declarations. If no-op runtime declarations are added, they
  must not write files, alter default builds, or implement #584/#585 behavior.

### 5. CLI User Experience

Add to `Commands::Build`:

```rust
#[arg(long)]
trace: bool
```

Expected behavior:

- `duumbi build --help` shows `--trace` with local-only wording.
- `duumbi build --trace` accepts existing `input`, `--output`, and `--offline`
  combinations.
- `duumbi build --trace --offline` still means dependency resolution uses
  workspace/vendor layers only.
- `duumbi run --trace` remains unsupported by clap because `run` has no trace
  flag. If a user tries it, current trailing-arg behavior may pass `--trace` to
  the binary. Do not reinterpret this as a DUUMBI trace shortcut in #583.
- Any traced-build-specific config validation failure must be actionable and
  mention the field name.

### 6. Artifact Path Policy

#583 defines path resolution and validation but does not write telemetry
artifacts.

Rules:

- Default path: `.duumbi/telemetry`.
- Environment override: `DUUMBI_TELEMETRY_DIR`.
- Reject empty paths.
- Reject paths containing `..`.
- Relative paths are resolved from workspace root for workspace builds.
- Relative paths are resolved from current directory for single-file builds.
- Traced build may create the telemetry directory as a build-preparation step if
  implementation agents decide that makes manual verification clearer, but it
  must not create trace event, mapping, crash, baseline, or alert files in #583.

### 7. Rejected Scope For #583

Do not implement these in #583:

- `duumbi_trace_function_enter`, `duumbi_trace_function_exit`,
  `duumbi_trace_block_enter`, or `duumbi_trace_block_exit` runtime calls.
- `traces.jsonl`, `trace_map.json`, `crash_dump.jsonl`, `baseline.json`, or
  `alerts.jsonl` writes.
- Argument or value capture.
- Remote telemetry endpoint config.
- OpenTelemetry/OTLP export.
- Studio telemetry UI or graph overlays.
- `duumbi telemetry inspect`.
- `duumbi run --trace`.
- Repair-agent input or repair validation.

## Invariants

- `duumbi build` without `--trace` remains uninstrumented.
- Default builds do not require a telemetry config section.
- Default builds do not perform telemetry semantic validation.
- `[telemetry] enabled = true` does not imply instrumentation.
- `duumbi build --trace` is the only #583 instrumentation opt-in.
- `enabled = false` with `--trace` may build a trace-capable binary, but runtime
  telemetry emission and artifact writes stay disabled.
- Telemetry stays local-only.
- No external service, provider credential, OTLP collector, Studio server, Slack,
  or GitHub access is required for traced build behavior.
- No generated telemetry artifacts are committed.
- Cranelift types remain inside `src/compiler/`.
- New public Rust items have doc comments and meaningful `#[must_use]` where
  required by repo conventions.
- No `.unwrap()` is added in library code.

## BDD-To-Test Mapping

| Product BDD scenario | Verification evidence |
|---|---|
| Build without trace uses the existing default behavior | Integration test using an existing fixture such as `tests/fixtures/hello.jsonld`: run `duumbi build <fixture> -o <tmpbin>` without `--trace`, assert success and binary output/exit behavior matches existing expectations. Add a guard that no telemetry files are written under a temp telemetry dir when not traced. |
| Missing telemetry config does not break normal builds | Workspace integration test: create a temp workspace with `.duumbi/config.toml` omitted or without `[telemetry]`, run default `duumbi build`, assert success and no telemetry directory requirement beyond existing init-created directory. |
| Developer requests a traced build | CLI integration test: run `duumbi build --trace <fixture> -o <tmpbin>` with no telemetry config, assert command succeeds, does not contact network, and produces a binary. Evidence is command status plus stderr/stdout assertions. |
| Traced build preserves existing build options | CLI integration test: run `duumbi build --trace <fixture> --output <tmpbin>`, assert binary exists at the requested path and no fallback output path is used. |
| Offline traced build does not require telemetry services | Workspace integration test: initialize or construct a dependency-free workspace, run `duumbi build --trace --offline`, assert dependency behavior remains local and no external collector configuration is required. For dependency fixtures, reuse existing offline tests from `tests/integration_phase7_local.rs` only if setup cost is reasonable. |
| Traced build uses conservative defaults | Unit test for telemetry defaults plus integration test for `duumbi build --trace` with no `[telemetry]`: assert defaults are `enabled = false`, deterministic sampling mode, `sample_rate = 0.0`, `artifact-dir = .duumbi/telemetry`, `capture-values = false`, and no remote endpoint exists. |
| Telemetry config alone does not instrument default builds | Integration test: create `[telemetry] enabled = true` and run default `duumbi build` without `--trace`; assert success and no telemetry semantic validation or artifact writes. Include a compile output comparison or review evidence that default codegen path uses `TelemetryBuildMode::Off`. |
| Disabled telemetry config gates runtime emission in traced builds | Unit test for effective telemetry config plus traced build integration: with `[telemetry] enabled = false`, run `duumbi build --trace`, assert success and no trace event/crash/mapping files are created. |
| Invalid sample rate is rejected | Config/telemetry unit test and CLI integration test: `[telemetry] sample-rate = 1.5` with `duumbi build --trace` fails before compilation with a message naming `sample-rate` and valid range. A default build with the same semantic telemetry error must not fail solely because of telemetry validation. |
| Unsupported sampling mode is rejected | Config/telemetry unit test and CLI integration test: unsupported `sampling-mode` with `duumbi build --trace` fails before compilation with a message naming `sampling-mode`. |
| Unsafe artifact path is rejected | Config/telemetry unit test and CLI integration test: empty `artifact-dir` and `artifact-dir = "../outside"` are rejected for traced builds with actionable errors. Also test `DUUMBI_TELEMETRY_DIR=../outside` rejection. |
| Run trace shortcut is not available in v1 | Review evidence or CLI behavior test: confirm `Commands::Run` has no DUUMBI `--trace` flag and implementation documentation states `duumbi run` runs the existing binary. Do not add a run shortcut in #583. |
| Unsupported build context fails clearly | If implementation does not support both single-file and workspace traced builds, add integration tests for the unsupported path asserting a clear error. Preferred implementation supports both, in which case this scenario is proven by traced tests for both paths. |

## Live E2E Plan

Canonical interface: CLI.

LLM/provider path:

- None. #583 is CLI/config/compiler plumbing and does not touch LLM behavior.
- Required provider credentials: none.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.

Required local commands for Stage 10 evidence:

```text
cargo fmt --check
cargo test telemetry
cargo test --test integration_phase1
cargo test --test integration_phase7_local offline
cargo test --test integration_phase9a1
```

Stage 10 may narrow test command names to exact new test names after
implementation. If command filters do not match due Rust test naming, run the
smallest equivalent focused tests and report the exact commands.

Manual CLI smoke path:

```text
cargo build --quiet
tmp="$(mktemp -d)"
target/debug/duumbi build tests/fixtures/hello.jsonld -o "$tmp/default"
target/debug/duumbi build --trace tests/fixtures/hello.jsonld -o "$tmp/traced"
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" target/debug/duumbi build --trace tests/fixtures/hello.jsonld -o "$tmp/traced-env"
```

Workspace smoke path:

```text
cargo build --quiet
tmp="$(mktemp -d)"
target/debug/duumbi init "$tmp/ws"
(cd "$tmp/ws" && "$OLDPWD/target/debug/duumbi" build)
(cd "$tmp/ws" && "$OLDPWD/target/debug/duumbi" build --trace)
(cd "$tmp/ws" && "$OLDPWD/target/debug/duumbi" build --trace --offline)
```

Invalid config smoke path:

```text
cargo build --quiet
tmp="$(mktemp -d)"
target/debug/duumbi init "$tmp/ws"
cat >> "$tmp/ws/.duumbi/config.toml" <<'TOML'
[telemetry]
sample-rate = 2.0
TOML
(cd "$tmp/ws" && "$OLDPWD/target/debug/duumbi" build)
(cd "$tmp/ws" && "$OLDPWD/target/debug/duumbi" build --trace)
```

Pass/fail criteria:

- Default build succeeds in the invalid telemetry semantic-config case unless
  unrelated config parsing fails.
- Traced build rejects invalid telemetry config with an actionable field name.
- Traced build succeeds with defaults and with `DUUMBI_TELEMETRY_DIR`.
- No remote collector, network service, provider credential, Studio server, or
  generated telemetry event/crash artifact is required.

Studio/TUI parity:

- Studio full E2E is not required because #583 adds no Studio UI behavior.
- Require thin review evidence that Studio and REPL default build paths still
  call `workflow::build_workspace()` with default uninstrumented options.
- If implementation changes `workflow::build_workspace()` signature, add
  focused compile-time or unit evidence that Studio/REPL default behavior still
  selects `TelemetryBuildMode::Off`.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. summarize current state and remaining unmet #583 requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 6 source/test files, excluding this technical
  spec and generated lock/build artifacts.
- Expected command budget per cycle:
  - up to 4 focused read/search commands
  - up to 2 formatting/check commands
  - up to 4 focused Rust test commands
  - no full `cargo test --all` unless a cycle is otherwise complete and local
    time budget allows it
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 external calls, exceeds approved scope, adds risky dependencies, touches
  remote telemetry/export, changes runtime crash behavior, performs irreversible
  operations, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external model or
  agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 3 low-budget implementation cycles.
- Stop and ask for human guidance when:
  - default uninstrumented builds cannot remain unchanged
  - config parse semantics would make default builds fail on telemetry semantic
    errors
  - implementation requires runtime trace hooks or artifact writes
  - implementation requires a remote telemetry dependency
  - single-file and workspace build support cannot both be completed within the
    approved scope
  - tests reveal a product-spec ambiguity not resolved here

## Task Breakdown

1. Add telemetry domain types and validation.
   - Add `TelemetryBuildMode`, `TelemetrySection`, `TelemetrySamplingMode`, and
     effective validation helpers.
   - Add serde defaults and kebab-case config names.
   - Add unit tests for defaults, valid config, invalid sample rates, unsupported
     sampling modes, unsafe paths, `DUUMBI_TELEMETRY_DIR`, and
     `capture-values = true` traced-build rejection.

2. Add build options.
   - Add `BuildOptions` or equivalent with `offline` and `telemetry_mode`.
   - Preserve existing default-off wrappers where public callers/tests rely on
     them.
   - Add workspace-root/current-dir resolution for telemetry path validation.

3. Add CLI `--trace`.
   - Extend `Commands::Build`.
   - Update `src/main.rs` dispatch.
   - Preserve offline message behavior.
   - Ensure `duumbi build --help` exposes local-only traced build wording.

4. Thread options through single-file and workspace build paths.
   - Update `src/cli/commands.rs`.
   - Update `src/workspace.rs`.
   - Update `src/workflow.rs` without changing REPL/Studio default behavior.
   - Validate telemetry config only when `TelemetryBuildMode::Trace` is used.

5. Add compiler option plumbing.
   - Add option-aware wrappers in `src/compiler/mod.rs` and
     `src/compiler/lowering.rs`.
   - Keep existing `compile_to_object()` and `compile_program()` as
     default-off wrappers.
   - Do not emit trace runtime calls in #583.

6. Add integration tests.
   - Default build unchanged.
   - `duumbi build --trace` single-file.
   - `duumbi build --trace --output`.
   - workspace `duumbi build --trace`.
   - workspace `duumbi build --trace --offline`.
   - telemetry config enabled without `--trace` does not instrument or block
     default build.
   - invalid telemetry config fails only for traced builds.

7. Add focused docs/help checks.
   - Confirm `--trace` help text.
   - Add minimal docs only if needed beyond clap help.

## Verification Plan

Required automated checks:

- `cargo fmt --check`
- `cargo test telemetry` or exact focused telemetry unit-test filter
- `cargo test --test integration_phase1`
- Focused config tests in `src/config.rs`
- Focused compiler/lowering option tests if compiler API changes are non-trivial
- Focused workspace build tests for traced/default behavior

Recommended additional checks:

- `cargo test --test integration_phase9a1` to ensure runtime-heavy default build
  behavior still works.
- `cargo test --test integration_phase7_local offline` if offline build logic is
  touched or shared build options risk dependency behavior.
- `cargo clippy --all-targets -- -D warnings` before marking the implementation
  PR ready for review if local time budget permits.

Manual/review evidence:

- CLI help includes `--trace`.
- Default build path still reaches `TelemetryBuildMode::Off`.
- REPL `/build` and Studio `/api/build` still use default uninstrumented
  workflow behavior.
- No runtime trace/event/crash artifact files are written by #583.
- No product spec, technical spec approval comment, generated telemetry
  artifact, or implementation artifact is modified outside the implementation
  PR.

## Completion Criteria

Implementation is complete when:

- `duumbi build --trace` is accepted for both single-file and workspace builds,
  or any unsupported path has a reviewed explicit error and test.
- Default `duumbi build` remains uninstrumented and does not require telemetry
  config.
- Telemetry semantic validation runs for traced builds and not for default
  builds.
- `[telemetry] enabled = true` without `--trace` does not imply
  instrumentation.
- `[telemetry] enabled = false` with `--trace` produces a trace-capable build
  path while runtime telemetry emission and artifact writes stay disabled.
- `sample-rate`, `sampling-mode`, `artifact-dir`, `DUUMBI_TELEMETRY_DIR`, and
  `capture-values` behavior match this spec.
- `--offline` behavior remains dependency-resolution-only.
- No remote telemetry, Studio UI, runtime trace events, crash artifacts, repair
  behavior, or run trace shortcut is implemented.
- BDD-to-test mapping evidence is present in the implementation PR description
  or review notes.
- Required focused tests and formatting checks pass, or failures are documented
  with a blocker report.

## Failure And Escalation

- If adding telemetry config to `DuumbiConfig` makes default builds fail on
  telemetry semantic errors, stop and redesign validation so semantic telemetry
  checks are trace-mode-only.
- If clap `run` trailing args make `duumbi run --trace` behavior confusing, do
  not add a run flag. Document that `--trace` after `run` is passed to the
  binary under existing trailing-arg semantics unless product review decides
  otherwise.
- If workspace and single-file build paths require divergent implementations,
  prefer a shared `BuildOptions` path. If one path cannot be completed within
  scope, stop for human guidance before shipping partial traced support.
- If trace-capable compiler plumbing requires runtime trace symbols to link,
  stop and either keep #583 no-op at compiler lowering or ask for scope
  confirmation before changing runtime assets.
- If any implementation path requires OpenTelemetry, remote export, network
  access, provider credentials, Studio UI, or repair-agent behavior, stop as
  out of scope.
- If test runtime grows beyond the cycle budget, run focused tests first and
  report the remaining full-suite risk.

## Stage 9 Review Recommendations

Recommended Stage 9 decisions:

- Keep telemetry config omitted from `duumbi init` for #583. `duumbi build
  --trace` should work with internal conservative defaults, and users should add
  `[telemetry]` only when they want to tune local behavior. Prefer CLI help,
  docs, or a later explicit config-generation command over writing commented
  telemetry examples during workspace initialization.
- Keep traced behavior build-only for #583. Defer `duumbi run --trace` to a
  later UX issue after #584 and #585 prove real trace events and local artifact
  behavior. A run shortcut needs a separate decision on whether it rebuilds,
  validates trace-capable binaries, or only sets runtime environment.
- For #584, start real function/block event emission with only
  `deterministic` and `probabilistic` sampling modes. Use `enabled = false` as
  the only disable mechanism rather than adding a separate `disabled` sampling
  mode. Defer `always`, adaptive sampling, per-function overrides, and
  `always-trace-functions` until trace event shape and overhead are proven.

## Open Questions

No open question blocks #583 implementation if Stage 9 accepts the
recommendations above. Follow-up issues may still decide:

- whether `duumbi init` should eventually offer an explicit telemetry config
  example or config-generation command
- whether a later `duumbi run --trace` shortcut is worth the rebuild/staleness
  semantics it introduces
- whether #584 or later telemetry slices should add sampling modes beyond
  `deterministic` and `probabilistic`
