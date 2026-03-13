---
tags:
  - project/duumbi
  - doc/technical
status: draft
created: 2026-02-15
updated: 2026-03-06
related_maps:
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - Task List]]"
  - "[[DUUMBI - Architecture Diagram]]"
  - "[[DUUMBI - Glossary]]"
---
# DUUMBI — Tools and Components

> Technical stack reference for the DUUMBI MVP. All component descriptions align with [[DUUMBI - MVP Specification]]. For terminology, see [[DUUMBI - Glossary]].

## Supported Platforms (MVP)

| Platform | Architecture | Status |
|---|---|---|
| macOS | aarch64 (Apple Silicon) | Primary development target |
| macOS | x86_64 | CI-tested |
| Linux | x86_64 | CI-tested |

Windows support is not planned for MVP.

## Core Components

### 1. CLI Application (`duumbi`)
- **Language**: Rust
- **Framework**: `clap` 4.x (argument parsing)
- **Role**: Primary user interface. All operations go through CLI commands.
- **Key Crates**: `clap`, `anyhow`, `thiserror`
- **Commands**: `init`, `build`, `run`, `check`, `describe` (Phase 1); `add`, `undo` (Phase 2); `viz` (Phase 3)
- **Error Output**: Structured JSONL to stdout, human-readable summary to stderr. See [[DUUMBI - MVP Specification#Error Format Specification]].

### 2. JSON-LD Engine
- **Role**: Parse, validate, and manipulate semantic graphs stored as `.jsonld` files.
- **Schema**: Custom ontology at `https://duumbi.dev/ns/core#`
- **Crates**: `serde`, `serde_json`, `jsonschema`
- **Storage**: `.jsonld` files in `.duumbi/graph/` within the workspace
- **Validation**: Enforces the core schema defined in [[DUUMBI - MVP Specification#JSON-LD Core Schema]]

### 3. Semantic Graph (In-Memory)
- **Role**: In-memory directed graph for validation, querying, and traversal.
- **Crate**: `petgraph` — **`StableGraph<N, E>`** (not `DiGraph`). `StableGraph` preserves `NodeIndex` and `EdgeIndex` values across node/edge removals, which is required so that graph indices remain valid throughout the mutation and compilation pipeline.
- **Operations**: Add/remove nodes, traverse dependencies, detect cycles, detect orphan references, compute content hashes
- **Invariant**: Every node must have a unique `@id`. Every `{"@id": "..."}` reference must resolve to an existing node.

### 4. Schema Validator
- **Role**: Enforce type correctness, reference integrity, and structural rules before compilation.
- **Input**: Semantic Graph (in-memory `petgraph`)
- **Output**: List of structured validation errors in JSONL format (see [[DUUMBI - MVP Specification#Error Format Specification]])
- **Rules enforced**:
  - All required fields present per Op type
  - Type consistency (e.g., `Add` operands and result must have matching numeric types)
  - No orphan references (`E004`)
  - No duplicate `@id` (`E005`)
  - No cycles in data flow (`E007`)
  - Entry function `main` exists (`E006`)

### 5. Cranelift Compiler
- **Role**: Transform validated semantic graph nodes → native machine code.
- **Crates**: `cranelift-codegen`, `cranelift-frontend`, `cranelift-module`, `cranelift-object`
- **Target Architecture (MVP)**: Host architecture only (x86_64 or aarch64, auto-detected)
- **Output**: `.o` object file written to `.duumbi/build/output.o`
- **Lowering map** (Phase 0):
  - `duumbi:Const` → `iconst` (i64)
  - `duumbi:Add` → `iadd`
  - `duumbi:Sub` → `isub`
  - `duumbi:Mul` → `imul`
  - `duumbi:Div` → `sdiv`
  - `duumbi:Print` → `call duumbi_print_i64`
  - `duumbi:Return` → `return`

### 6. Linker
- **Role**: Combine Cranelift-emitted `.o` files + runtime shim into a native executable.
- **Method**: Invoke system C compiler (`cc`) as linker.
- **Command**: `cc output.o duumbi_runtime.o -o output -lc`
- **Detection**: `$CC` env var → `cc` on PATH → error with install instructions.
- **Runtime Shim**: Small C file (`duumbi_runtime.c`) compiled once per build, providing:
  - `duumbi_print_i64(int64_t)` — print i64 + newline
  - `duumbi_print_f64(double)` — (Phase 1)
  - `duumbi_print_bool(int8_t)` — (Phase 1)

### 7. Telemetry Engine
- **Role**: Inject `traceId` metadata into compiled binaries for crash-to-graph mapping.
- **Crates**: `tracing`, `tracing-subscriber`
- **Output**: Structured JSON logs with `traceId` → `nodeId` mapping written to `.duumbi/telemetry/traces.jsonl`

### 8. AI Agent Module (Phase 2)
- **Role**: Translate natural language intent → JSON-LD graph patches.
- **Crates**: `reqwest` (HTTP client), `tokio` (async runtime)
- **Supported Providers**: OpenAI API, Anthropic API (configurable via `.duumbi/config.toml`)
- **Workflow**: User intent → system prompt + graph context → LLM API → JSON-LD patch → schema validation → diff preview → user confirmation → apply.

### 9. Web Visualizer (Phase 3)
- **Role**: Read-only, browser-based graph visualization.
- **Tech**: Cytoscape.js (vendored JS) — served by `axum` 0.8 HTTP + WebSocket server. No WASM required.
- **Server**: `axum` 0.8 + `tower-http` for static file serving
- **File Watcher**: `notify-debouncer-mini` → mpsc channel → tokio async reload → WebSocket push to browser
- **Default Port**: 8420

## Rust Crate Dependencies

| Crate | Purpose | Phase |
|---|---|---|
| `clap` 4.x | CLI argument parsing | 0 |
| `serde` 1.x | Serialization framework | 0 |
| `serde_json` 1.x | JSON parsing | 0 |
| `anyhow` 1.x | Error handling | 0 |
| `thiserror` 1.x | Custom error types | 0 |
| `petgraph` 0.7.x | Graph data structure (`StableGraph`) | 0 |
| `cranelift-codegen` 0.129.x | Code generation backend | 0 |
| `cranelift-frontend` 0.129.x | IR builder API | 0 |
| `cranelift-module` 0.129.x | Module management | 0 |
| `cranelift-object` 0.129.x | Object file emission | 0 |
| `jsonschema` | JSON Schema validation | 1 |
| `tracing` 0.1.x | Structured logging | 1 |
| `tracing-subscriber` 0.3.x | Log output formatting | 1 |
| `toml` 0.8.x | Config file parsing | 1 |
| `reqwest` 0.12.x | HTTP client for LLM API | 2 |
| `tokio` 1.x | Async runtime | 2 |
| `axum` 0.8.x | Web server framework | 3 |
| `tower-http` 0.6.x | Static file serving for axum | 3 |
| `notify-debouncer-mini` 0.5.x | File system watcher for live reload | 3 |

## Development Tools

| Tool | Purpose |
|---|---|
| `cargo` | Build system and package manager |
| `rustfmt` | Code formatting |
| `clippy` | Linting |
| `cargo test` | Unit and integration testing |
| `cargo bench` | Performance benchmarking (compilation speed) |
| `pre-commit` | Git hooks for quality gates |
| `cc` (system) | Linking object files to executables |
| GitHub Actions | CI/CD pipeline |
## Workspace File Structure

```
project/
├── .duumbi/
│   ├── config.toml            # Project configuration (LLM provider, registries, deps)
│   ├── deps.lock              # Lockfile — semantic hash + SHA-256 per dependency
│   ├── schema/
│   │   └── core.schema.json   # JSON-LD schema definition
│   ├── graph/                 # Workspace Layer — local source modules
│   │   ├── main.jsonld        # Main module graph
│   │   └── lib/               # Library modules
│   ├── vendor/                # Vendor Layer — VCS-committed dependency snapshots
│   │   └── @duumbi/
│   │       └── stdlib-math@1.0.0/
│   ├── cache/                 # Cache Layer — registry-resolved downloads (not in VCS)
│   │   └── @duumbi/
│   │       └── stdlib-math@1.0.0/
│   ├── build/
│   │   ├── output.o           # Cranelift object file
│   │   ├── duumbi_runtime.o   # Compiled runtime shim
│   │   └── output             # Final linked executable
│   ├── telemetry/
│   │   └── traces.jsonl       # Runtime trace logs
│   └── history/               # Undo stack — {N:06}.jsonld snapshots (Phase 2)
│       └── 000001.jsonld
├── sites/                     # Web presence (M4+)
│   ├── docs/                  # docs.duumbi.dev (mdBook)
│   └── landing/               # duumbi.dev (Astro, M5+)
├── examples/
│   ├── add.jsonld             # add(3, 5) → prints 8
│   ├── fibonacci.jsonld       # Fibonacci with branching and recursion (Phase 1)
│   └── hello.jsonld           # Print multiple values (Phase 1)
└── README.md
```

## Post-MVP Tech Stack

| Component | Tech | Milestone |
|-----------|------|-----------|
| Chat REPL line editor | `reedline` 0.37 | M4 |
| Token counting | `tiktoken-rs` | M4 |
| Docs site | `mdBook` | M4 |
| Landing page | Astro (static) | M5 |
| Web Studio | Leptos (Rust → WASM) | M6 |
| MCP server | `rmcp` crate | M8 |
| Graph registry client | custom HTTP client | M7 |

## Related Documents

- [[DUUMBI - MVP Specification]] — Authoritative build specification
- [[DUUMBI - Task List]] — Implementation breakdown
- [[DUUMBI - Architecture Diagram]] — Visual component overview
- [[DUUMBI - Glossary]] — Canonical term definitions
- [[DUUMBI - Graph Repository Architecture]] — Graph module storage and namespace design
