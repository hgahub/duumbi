---
tags:
  - project/duumbi
  - doc/planning
status: active
created: 2026-02-15
updated: 2026-03-01
related_maps:
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - Tools and Components]]"
  - "[[DUUMBI - Glossary]]"
---
# DUUMBI — Task List

> Atomic implementation checklist. Each task references a requirement ID from [[DUUMBI - MVP Specification]]. Tasks are ordered by dependency — complete top to bottom within each phase.

## Phase 0: Proof of Concept — Week 1-2

> [!warning] Kill Criterion: Compile `add(3, 5)` JSON-LD graph → native binary that prints `8`. Go/No-Go.

### Schema & Project Setup
- [ ] Write `core.schema.json` for Phase 0 ops: `duumbi:Module`, `duumbi:Function`, `duumbi:Block`, `duumbi:Const`, `duumbi:Add`, `duumbi:Sub`, `duumbi:Mul`, `duumbi:Div`, `duumbi:Print`, `duumbi:Return`. Schema must enforce all required fields per [[DUUMBI - MVP Specification#JSON-LD Core Schema]]
- [ ] Write `examples/add.jsonld` — the `add(3, 5)` program from [[DUUMBI - MVP Specification#Complete Phase 0 Example]]
- [ ] `cargo init duumbi` — create Rust project with `Cargo.toml`
- [ ] Add Phase 0 crate dependencies: `clap`, `serde`, `serde_json`, `anyhow`, `thiserror`, `petgraph`, `cranelift-codegen`, `cranelift-frontend`, `cranelift-module`, `cranelift-object`

### Parser & Graph
- [ ] Implement JSON-LD parser: read `.jsonld` file → `serde_json::Value` → validate against `core.schema.json`
- [ ] Implement graph builder: transform validated JSON → `petgraph::DiGraph` with nodes (Ops) and edges (data flow references)
- [ ] Implement reference resolver: verify every `{"@id": "..."}` points to an existing node, emit `E004` if not

### Compiler & Linker
- [ ] Implement Cranelift codegen for `duumbi:Const` → `iconst` (i64 only)
- [ ] Implement Cranelift codegen for `duumbi:Add` → `iadd`
- [ ] Implement Cranelift codegen for `duumbi:Sub` → `isub`
- [ ] Implement Cranelift codegen for `duumbi:Mul` → `imul`
- [ ] Implement Cranelift codegen for `duumbi:Div` → `sdiv`
- [ ] Implement Cranelift codegen for `duumbi:Return` → `return`
- [ ] Implement Cranelift codegen for `duumbi:Print` → `call duumbi_print_i64` (external function call)
- [ ] Write `duumbi_runtime.c` with `duumbi_print_i64(int64_t val)` function
- [ ] Implement object emission: graph → Cranelift IR → `.o` file at `.duumbi/build/output.o`
- [ ] Implement linker invocation: detect `$CC` or `cc`, run `cc output.o duumbi_runtime.o -o output -lc`, handle linker errors as `E008`

### Validation
- [ ] Write integration test: `add.jsonld` → parse → graph → compile → link → run → assert stdout == `8\n` and exit code == 8
- [ ] **GATE REVIEW**: Demo working end-to-end. Decide Go/No-Go.

## Phase 1: Usable CLI — Week 3-6

> [!warning] Kill Criterion: External developer installs and runs a non-trivial program in < 10 minutes.

### CLI Commands
- [ ] [CLI-01] `duumbi init` — create `.duumbi/` with `config.toml`, `schema/core.schema.json`, `graph/main.jsonld` (skeleton), `README.md`
- [ ] [CLI-02] `duumbi build` — discover all `.jsonld` in `.duumbi/graph/`, validate, compile, link to `.duumbi/build/output`
- [ ] [CLI-03] `duumbi run` — execute `.duumbi/build/output`, stream stdout/stderr to terminal
- [ ] [CLI-04] `duumbi check` — validate graph, output errors as JSONL to stdout + human-readable to stderr (see [[DUUMBI - MVP Specification#Error Format Specification]])
- [ ] [CLI-05] `duumbi describe` — walk graph, output pseudo-code summary (e.g., `function main(): i64 { return 3 + 5 }`)

### Compiler Expansion
- [ ] Add `f64` support: `duumbi:Const` with `resultType: "f64"` → Cranelift `f64const`; `Add/Sub/Mul/Div` → `fadd/fsub/fmul/fdiv`
- [ ] Add `bool` support: represent as `i8` (0=false, 1=true) in Cranelift
- [ ] Implement `duumbi:Compare` → Cranelift `icmp`/`fcmp` based on operand type
- [ ] Implement `duumbi:Branch` → Cranelift `brif` (conditional branch to true/false blocks)
- [ ] Implement `duumbi:Call` → Cranelift function call with argument passing
- [ ] Implement `duumbi:Load` → Cranelift `stack_load` from named variable slot
- [ ] Implement `duumbi:Store` → Cranelift `stack_store` to named variable slot
- [ ] Implement `duumbi:Function` with parameters and return type
- [ ] Implement multi-module compilation: multiple `.jsonld` files → single binary (resolve cross-module function calls)
- [ ] Update `duumbi_runtime.c`: add `duumbi_print_f64(double)` and `duumbi_print_bool(int8_t)`

### Telemetry
- [ ] [CLI-06] Inject `traceId` into compiled code as metadata
- [ ] Emit structured JSON logs from compiled binaries to `.duumbi/telemetry/traces.jsonl`
- [ ] Implement crash-to-graph mapping: panic handler → lookup nodeId from traceId

### Examples
- [ ] Write `examples/fibonacci.jsonld` — recursive Fibonacci using `Branch`, `Compare`, `Call`
- [ ] Write `examples/hello.jsonld` — print multiple values sequentially

### Quality
- [ ] Unit tests: JSON-LD parser (valid input, each error code)
- [ ] Unit tests: graph builder (valid graph, orphan refs, duplicate IDs, cycles)
- [ ] Unit tests: each Cranelift codegen Op (correct output for known inputs)
- [ ] Unit tests: each CLI command (success and error cases)
- [ ] Integration tests: `fibonacci.jsonld` → correct output for inputs 1-10
- [ ] Set up `pre-commit` hooks: `rustfmt`, `clippy`
- [ ] Set up GitHub Actions CI: build + test + lint on every push
- [ ] Write `README.md`: installation, prerequisites (`cc` required), quickstart (init → build → run in < 5 min)
- [ ] **GATE REVIEW**: External person installs and runs. Measure time. Must be < 10 min.

## Phase 2: AI Integration — Week 7-10

> [!warning] Kill Criterion: >70% correct mutations on 20-command benchmark set.

### Core AI Workflow
- [ ] [AI-04] Implement `.duumbi/config.toml` parser — fields: `llm.provider` ("openai"/"anthropic"), `llm.model`, `llm.api_key_env` (env var name, NOT the key itself)
- [ ] Design LLM system prompt template: include graph context (current module structure) + schema summary + user intent → expected output: JSON-LD patch
- [ ] [AI-01] Implement `duumbi add "text"` — send prompt to configured LLM, receive JSON-LD patch
- [ ] [AI-02] Validate AI-generated patch against schema before applying; reject with error message if invalid
- [ ] [AI-03] Implement diff preview: show proposed graph changes as readable diff, prompt for confirmation (y/n)
- [ ] [AI-05] Implement `duumbi undo` — snapshot graph state before each mutation, restore previous snapshot on undo

### Benchmark & Measurement
- [ ] Create benchmark set: 20 natural language commands with gold-standard expected graph diffs (store in `tests/ai_benchmark/`)
- [ ] Define "correct mutation": (1) passes `duumbi check`, (2) `duumbi build` succeeds, (3) graph diff structurally matches gold standard (ignoring `@id` values)
- [ ] Test with OpenAI GPT-4o — record accuracy
- [ ] Test with Anthropic Claude — record accuracy
- [ ] Write integration tests for AI workflow using mock LLM responses (deterministic, no API calls)
- [ ] **GATE REVIEW**: Accuracy report. If <70% on both providers, iterate on prompt design before proceeding.

## Phase 3: Visualization — Week 11-13

> [!warning] Kill Criterion: 3 developers confirm visualizer helps vs raw JSON-LD.

- [ ] [VIZ-01] Set up `axum` web server in `duumbi viz` command (default port 8420)
- [ ] [VIZ-02] Implement WASM-based Canvas renderer: display function-level flowchart (nodes = Ops, edges = data flow)
- [ ] [VIZ-03] Implement WebSocket live sync: file watcher → browser push on `.jsonld` file change (< 1 sec latency)
- [ ] Display node metadata on click/hover: `@type`, `@id`, connected nodes, `traceId`
- [ ] Test cross-browser: Chrome, Firefox, Safari
- [ ] **GATE REVIEW**: Timed comparison test with 3 developers (read raw JSON-LD vs visualizer).

## Infrastructure & Ongoing

- [ ] Configure GitHub Actions release workflow: build binaries for macOS aarch64, macOS x86_64, Linux x86_64
- [ ] Set up Slack notifications for CI/CD failures
- [ ] Write `CONTRIBUTING.md`
- [ ] Publish to crates.io (or private registry) after Phase 1 gate passes

## Related Documents

- [[DUUMBI - MVP Specification]] — Requirement IDs referenced in tasks
- [[DUUMBI - Tools and Components]] — Technical stack details
- [[DUUMBI - Glossary]] — Term definitions


---

## Phase 2 GitHub Tracking

> [!info] Issues #26-#38 · Milestone "Phase 2: AI Integration" (#3) · Branch `phase2/implementation`

| # | Issue | Title |
|---|-------|-------|
| #26 | Config module | Parse `.duumbi/config.toml` |
| #27 | GraphPatch | Format & applicator (all-or-nothing) |
| #28 | LLM tool schema | 6 tools: AddFunction, AddOp, ModifyOp, RemoveNode, AddBlock, SetEdge |
| #29 | Anthropic provider | Tool use, max 1 retry |
| #30 | OpenAI provider | Function calling, max 1 retry |
| #31 | Orchestrator | Prompt → LLM → tools → patch → validate → apply |
| #32 | `duumbi add` | CLI command (async, diff preview, confirm) |
| #33 | `duumbi undo` | Git-like snapshot history |
| #34 | Benchmark fixtures | 20-command + mock LLM responses |
| #35 | Benchmark harness | Score pass/fail, report accuracy |
| #36 | Integration tests | Mock responses (deterministic) |
| #37 | Docs update | README, architecture, config reference |
| #38 | Gate Review | ≥14/20 (70%) benchmark pass |

---

## ✅ Completion Summary

> [!success] All MVP phases 0–3 complete as of 2026-03-01. Phase 4 (Module System) also complete.
> Task checkboxes above reflect the original planning document — all `[ ]` items in Phases 0–3 were implemented and tested. See GitHub tracking sections below for the full breakdown.

## Phase 3 GitHub Tracking

> [!info] PR #40 · Milestone "Phase 3: Visualization" · Branch `phase3/web-visualizer`

| Component | Implementation |
|-----------|----------------|
| `duumbi viz` CLI command | axum 0.8 server on port 8420 |
| Graph serializer | Cytoscape.js JSON format (`src/web/serialize.rs`) |
| File watcher | `notify-debouncer-mini` → mpsc → tokio async reload loop |
| WebSocket handler | `tokio::sync::broadcast` → ws stream per client |
| Frontend assets | Vendored Cytoscape.js + HTML skeleton (`src/web/assets/`) |
| AppState | `Arc<RwLock<Value>>` graph + `broadcast::Sender` for WS push |

**Note on VIZ-02:** Final implementation used Cytoscape.js + axum (not WASM + Canvas) — simpler, no WASM toolchain needed, same user-facing result.

## Phase 4: Module System & Standard Library

> [!warning] Kill Criterion: `duumbi init` → write 2-module program → `duumbi build && duumbi run` → binary prints correct output. M4: `abs(-7) = 7`. ✓

### Multi-Module Compilation
- [x] Implement `Program::load()` — discover and load all `.jsonld` modules from `.duumbi/graph/`
- [x] Implement cross-module reference validation — E010 error for unresolved cross-module refs
- [x] Implement `compile_program()` — compile all modules to `.o` files (one per module)
- [x] Implement `link_multi()` — link multiple `.o` files + runtime into a single binary

### Dependency Management
- [x] Implement `DuumbiConfig` — TOML parsing with `[dependencies]` section
- [x] Implement `generate_lockfile()` — FNV-1a hash-based deterministic lockfile
- [x] Implement `add_dependency()` / `remove_dependency()` — CRUD for workspace deps
- [x] Implement `duumbi deps list/add/remove` CLI subcommands

### Standard Library
- [x] Write `stdlib/math.jsonld` — `abs`, `max`, `min` functions (i64, using Branch+Compare+Load)
- [x] Write `stdlib/io.jsonld` — `print_i64`, `print_f64`, `print_bool` wrappers
- [x] Embed stdlib in `duumbi init` — write to `.duumbi/stdlib/math/` and `.duumbi/stdlib/io/`
- [x] Configure stdlib deps in default `config.toml` generated by `duumbi init`

### Quality
- [x] Integration tests: 2-module compilation, stdlib abs/max/min correctness
- [x] E010 error test: unresolved cross-module reference
- [x] Lockfile determinism test
- [x] M4 kill criterion test: `init → write program → compile → run → verify output`
- [x] **GATE REVIEW**: M4 kill criterion passed — `abs(-7) = 7`, 202 tests green.

## Phase 4 GitHub Tracking

> [!info] Issues #58–#62 · Branch `phase4/interactive-cli-module-system`

| # | Issue | Title |
|---|-------|-------|
| #58 | Multi-module loading | `Program::load()` — discover + validate all modules in workspace |
| #59 | Multi-module compiler | `compile_program()` + `link_multi()` — multi-object linking |
| #60 | deps command | `duumbi deps list/add/remove` + FNV-1a lockfile generation |
| #61 | stdlib math/io | `stdlib/math.jsonld` + `stdlib/io.jsonld` + embedded in `duumbi init` |
| #62 | Integration tests | Phase 4 test suite + M4 kill criterion (`abs(-7) = 7`) |
| #51 | Task List update | Mark phases 0–4 complete, add GitHub tracking tables |
| #50 | Doc fixes | Architecture Diagram, Glossary, MVP Spec accuracy fixes |
