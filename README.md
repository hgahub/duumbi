# DUUMBI

> AI-first semantic graph compiler. Programs are stored as JSON-LD graphs — not text files. The toolchain validates, compiles, and links them to native binaries via Cranelift.

**Status:** Phase 0 — active development. No release yet.

---

## What is DUUMBI?

Traditional compilers transform text → AST → machine code. DUUMBI skips the text entirely. Program logic lives as a typed semantic graph in JSON-LD format. An AI agent generates and mutates the graph; the toolchain validates every mutation before compilation. Syntax errors are structurally impossible.

The core invariant is the **Semantic Fixed Point**: a graph is compilable only when it passes schema validation, type checking, and all tests pass.

```
.jsonld files  →  Parser  →  Semantic Graph  →  Schema Validator  →  Cranelift  →  .o  →  cc  →  binary
```

---

## Prerequisites

- Rust stable (`rustup update stable`)
- A C compiler on PATH — `cc` or `$CC` env var (Xcode CLT on macOS, `gcc`/`clang` on Linux)
- Windows is not supported in MVP

Verify:
```bash
rustc --version   # 1.80+
cc --version
```

---

## Build

```bash
git clone git@github.com:hgahub/duumbi.git
cd duumbi
cargo build
cargo test --all
```


## Project Structure

```
duumbi/
├── src/
│   ├── main.rs          # CLI entry point (clap)
│   ├── parser/          # JSON-LD → serde_json → typed structs
│   ├── graph/           # petgraph semantic graph IR
│   ├── compiler/        # Graph → Cranelift IR → .o
│   ├── cli/             # Command implementations
│   ├── agents/          # AI agent module (Phase 2)
│   └── web/             # WASM visualizer + axum server (Phase 3)
├── tests/               # Integration tests with .jsonld fixtures
├── examples/
│   ├── add.jsonld        # add(3, 5) → prints 8  [Phase 0 kill criterion]
│   ├── fibonacci.jsonld  # Recursive Fibonacci    [Phase 1]
│   └── hello.jsonld      # Multiple prints        [Phase 1]
├── docs/
│   ├── architecture.md   # Component map, data formats, Cranelift lowering
│   └── coding-conventions.md  # Error handling, newtypes, graph/Cranelift rules
├── Cargo.toml
└── duumbi_runtime.c      # C shim: duumbi_print_i64, duumbi_print_f64, etc.
```

A user's DUUMBI workspace (created by `duumbi init`) lives separately:
```
my-project/
└── .duumbi/
    ├── config.toml       # LLM provider, model, api_key_env
    ├── schema/
    │   └── core.schema.json
    ├── graph/
    │   └── main.jsonld
    ├── build/
    │   └── output        # compiled binary
    └── telemetry/
        └── traces.jsonl
```

---

## JSON-LD Graph Format

Programs are stored as typed graphs. Every node has a unique `@id` in the format `duumbi:<module>/<function>/<block>/<index>`.

Namespace: `https://duumbi.dev/ns/core#` (prefix: `duumbi:`)

Minimal working program — `add(3, 5)`:

```json
{
  "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [
    {
      "@type": "duumbi:Function",
      "@id": "duumbi:main/main",
      "duumbi:name": "main",
      "duumbi:params": [],
      "duumbi:returnType": "i64",
      "duumbi:blocks": [
        {
          "@type": "duumbi:Block",
          "@id": "duumbi:main/main/entry",
          "duumbi:label": "entry",
          "duumbi:ops": [
            { "@type": "duumbi:Const",  "@id": "duumbi:main/main/entry/0", "duumbi:value": 3, "duumbi:resultType": "i64" },
            { "@type": "duumbi:Const",  "@id": "duumbi:main/main/entry/1", "duumbi:value": 5, "duumbi:resultType": "i64" },
            { "@type": "duumbi:Add",    "@id": "duumbi:main/main/entry/2",
              "duumbi:left": {"@id": "duumbi:main/main/entry/0"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/1"},
              "duumbi:resultType": "i64" },
            { "@type": "duumbi:Print",  "@id": "duumbi:main/main/entry/3", "duumbi:operand": {"@id": "duumbi:main/main/entry/2"} },
            { "@type": "duumbi:Return", "@id": "duumbi:main/main/entry/4", "duumbi:operand": {"@id": "duumbi:main/main/entry/2"} }
          ]
        }
      ]
    }
  ]
}
```

Expected output: prints `8`, exits with code 8.


---

## Phase 0 Op Set

| Op | Cranelift | Description |
|----|-----------|-------------|
| `duumbi:Const` | `iconst.i64` | Constant integer value |
| `duumbi:Add` | `iadd` | Integer addition |
| `duumbi:Sub` | `isub` | Integer subtraction |
| `duumbi:Mul` | `imul` | Integer multiplication |
| `duumbi:Div` | `sdiv` | Integer division (truncating) |
| `duumbi:Print` | `call duumbi_print_i64` | Print value to stdout |
| `duumbi:Return` | `return` | Return from function |

Phase 1 adds: `f64`, `bool`, `Compare`, `Branch`, `Call`, `Load`, `Store`, multi-module.
Phase 2 adds: AI mutation (`duumbi add "..."`, `duumbi undo`).
Phase 3 adds: WASM web visualizer (`duumbi viz`).

---

## Error Format

All validation and compile errors go to **stdout as JSONL** (one error per line). Human-readable summary goes to **stderr**. Never mix the two.

```json
{"level":"error","code":"E001","message":"Type mismatch: Add expects matching operand types","nodeId":"duumbi:main/main/entry/2","file":"graph/main.jsonld","details":{"expected":"i64","found":"f64"}}
```

| Code | Condition |
|------|-----------|
| E001 | Type mismatch |
| E002 | Unknown Op type |
| E003 | Missing required field |
| E004 | Orphan `@id` reference |
| E005 | Duplicate `@id` |
| E006 | No `main` function |
| E007 | Cycle in data flow |
| E008 | Linker failure |
| E009 | Schema validation failed |

---

## Code Standards

Read `docs/coding-conventions.md` before writing any code. Key rules:

**Error handling:** `thiserror` in library code (`src/parser/`, `src/graph/`, `src/compiler/`), `anyhow` only at CLI boundary (`src/cli/`, `main.rs`). `.unwrap()` is forbidden — CI clippy rejects it.

**Type safety:** Newtypes are mandatory for all graph identifiers. Never pass raw strings or integers where a `NodeId`, `FunctionName`, `BlockLabel`, or `ModuleName` is expected.

**Graph:** Use `StableGraph<N, E>` when node indices must survive removals. Use `petgraph::visit::Topo` for traversal order. Run `petgraph::algo::is_cyclic_directed` before compilation.

**Cranelift:** Always use `FunctionBuilder` via `FunctionBuilderContext`. Call `builder.seal_block()` before `builder.finalize()`. Call `builder.finalize()` before `module.define_function()`.

**Output:** Structured JSONL → stdout. Human text → stderr. `println!` for debug output is forbidden — use `tracing::debug!`.

---

## Development Workflow

```bash
# Check compilation
cargo check

# Run tests
cargo test --all

# Lint (zero warnings enforced)
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt

# Run a specific integration test
cargo test --test <name>
```

The `pre-commit` hooks run `rustfmt` and `clippy` automatically on every commit.

---

## Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS | aarch64 (Apple Silicon) | Primary |
| macOS | x86_64 | CI-tested |
| Linux | x86_64 | CI-tested |
| Windows | — | Not planned for MVP |

---

## MVP Phases and Kill Criteria

| Phase | Goal | Kill Criterion | Timeline |
|-------|------|----------------|----------|
| 0 | Proof of concept | `add(3, 5)` compiles and prints `8` | Week 1-2 |
| 1 | Usable CLI | External dev installs + runs in < 10 min | Week 3-6 |
| 2 | AI mutation | > 70% correct on 20-command benchmark | Week 7-10 |
| 3 | Web visualizer | 3/3 devs confirm faster than raw JSON-LD | Week 11-13 |

Each phase ends with a Gate Review — Go/No-Go decision before proceeding.

