---
tags:
  - project/duumbi
  - doc/specification
status: draft
created: 2026-02-15
updated: 2026-02-17
related_maps:
  - "[[DUUMBI - PRD]]"
  - "[[DUUMBI - Tools and Components]]"
  - "[[DUUMBI - Task List]]"
  - "[[DUUMBI - Architecture Diagram]]"
  - "[[DUUMBI - Glossary]]"
---

# DUUMBI — MVP Specification

> [!important] This is the **authoritative build specification**. All implementation decisions are made here. For the long-term vision, see [[DUUMBI - PRD]]. For terminology, see [[DUUMBI - Glossary]].

## Target User

**General developer** with no prior JSON-LD knowledge. The CLI must be approachable: clear error messages in plain English, a quickstart guide that gets from install to running binary in under 5 minutes, and a `describe` command that renders the graph as human-readable pseudo-code.

## Installation Method

**MVP:** Install from source via Cargo.
```bash
git clone https://github.com/hgahub/duumbi.git
cd duumbi
cargo install --path .
```

**Prerequisite:** Rust toolchain (rustup), system C compiler (`cc`) for linking.

**Post-MVP:** Pre-built binaries via GitHub Releases for macOS aarch64, macOS x86_64, Linux x86_64.

## Type System Specification

DUUMBI uses a simple, explicit type system that expands across phases.

**Phase 0 (Proof of Concept):**
- `i64` — 64-bit signed integer. The only type.

**Phase 1 (Usable CLI):**
- `i64` — 64-bit signed integer
- `f64` — 64-bit IEEE 754 floating point
- `bool` — Boolean (`true`/`false`, represented as `i8` in Cranelift: 0 = false, 1 = true)
- `void` — No return value (for functions with side effects only, e.g., `Print`)

**Phase 2+ (Future):**
- `string` — UTF-8 byte sequence (heap-allocated, length-prefixed)
- Composite types (structs, arrays) — definition TBD based on Phase 1 learnings

Type names are used in the `duumbi:resultType` field of every Op node. Type checking is enforced by the schema validator: an `Add` op requires both operands and its result to have the same numeric type.

## JSON-LD Core Schema

The schema below defines all valid Op types for the DUUMBI semantic graph. This is the canonical reference. The machine-readable version lives at `.duumbi/schema/core.schema.json` in every workspace.

**Namespace:** `https://duumbi.dev/ns/core#` (prefix: `duumbi:`)

### Context Definition

Every `.jsonld` file must include this context:
```json
{
  "@context": {
    "duumbi": "https://duumbi.dev/ns/core#"
  }
}
```

### Common Node Properties

Every Op node has these required fields:

| Field | Type | Description |
|---|---|---|
| `@type` | string | The Op type, prefixed with `duumbi:` (e.g., `duumbi:Add`) |
| `@id` | string | Unique node identifier. Format: `duumbi:<module>/<function>/<block>/<index>` |

### Phase 0 Operations

**`duumbi:Const`** — Produce a constant value.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:value` | number | Yes | The constant value |
| `duumbi:resultType` | string | Yes | `"i64"` (Phase 0) |

**`duumbi:Add`** — Integer addition.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:left` | `{"@id": "..."}` | Yes | Reference to left operand node |
| `duumbi:right` | `{"@id": "..."}` | Yes | Reference to right operand node |
| `duumbi:resultType` | string | Yes | Must match operand types |

**`duumbi:Sub`** — Integer subtraction.
Same fields as `duumbi:Add`.

**`duumbi:Mul`** — Integer multiplication.
Same fields as `duumbi:Add`.

**`duumbi:Div`** — Integer division (truncating).
Same fields as `duumbi:Add`.

**`duumbi:Print`** — Print a value to stdout, followed by a newline.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:operand` | `{"@id": "..."}` | Yes | Reference to value node to print |

Implementation note: `Print` emits a call to a DUUMBI runtime shim that calls libc `printf`. The shim is linked via `cc`.

**`duumbi:Return`** — Return a value from the current function (or set process exit code for `main`).
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:operand` | `{"@id": "..."}` | Yes | Reference to return value node |

### Phase 1 Operations (added in Phase 1)

**`duumbi:Branch`** — Conditional branch.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:condition` | `{"@id": "..."}` | Yes | Reference to a bool-typed node |
| `duumbi:trueBlock` | string | Yes | Block ID to jump to if true |
| `duumbi:falseBlock` | string | Yes | Block ID to jump to if false |

**`duumbi:Compare`** — Compare two values.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:left` | `{"@id": "..."}` | Yes | Reference to left operand |
| `duumbi:right` | `{"@id": "..."}` | Yes | Reference to right operand |
| `duumbi:operator` | string | Yes | One of: `eq`, `ne`, `lt`, `le`, `gt`, `ge` |
| `duumbi:resultType` | string | Yes | Always `"bool"` |

**`duumbi:Call`** — Call a function.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:function` | string | Yes | Name of the target function |
| `duumbi:args` | array of `{"@id": "..."}` | Yes | References to argument value nodes |
| `duumbi:resultType` | string | No | Return type (omit for void functions) |

**`duumbi:Load`** — Load a value from a named variable.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:variable` | string | Yes | Variable name |
| `duumbi:resultType` | string | Yes | Type of the loaded value |

**`duumbi:Store`** — Store a value into a named variable.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:variable` | string | Yes | Variable name |
| `duumbi:operand` | `{"@id": "..."}` | Yes | Reference to value node to store |

### Structural Nodes

**`duumbi:Function`** — A function definition.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:name` | string | Yes | Function name (must be unique within module) |
| `duumbi:params` | array of `{name, type}` | Yes | Parameter list (may be empty) |
| `duumbi:returnType` | string | Yes | Return type (`"void"` if no return) |
| `duumbi:blocks` | array of `duumbi:Block` | Yes | Ordered list of basic blocks |

**`duumbi:Block`** — A basic block (sequence of Ops with a single entry point).
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:label` | string | Yes | Block label (unique within function) |
| `duumbi:ops` | array of Op nodes | Yes | Ordered list of operations |

**`duumbi:Module`** — Top-level container.
| Field | Type | Required | Description |
|---|---|---|---|
| `duumbi:name` | string | Yes | Module name |
| `duumbi:functions` | array of `duumbi:Function` | Yes | Functions in this module |

### Complete Phase 0 Example

A program that computes `add(3, 5)` and prints `8`:

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
            {
              "@type": "duumbi:Const",
              "@id": "duumbi:main/main/entry/0",
              "duumbi:value": 3,
              "duumbi:resultType": "i64"
            },
            {
              "@type": "duumbi:Const",
              "@id": "duumbi:main/main/entry/1",
              "duumbi:value": 5,
              "duumbi:resultType": "i64"
            },
            {
              "@type": "duumbi:Add",
              "@id": "duumbi:main/main/entry/2",
              "duumbi:left": { "@id": "duumbi:main/main/entry/0" },
              "duumbi:right": { "@id": "duumbi:main/main/entry/1" },
              "duumbi:resultType": "i64"
            },
            {
              "@type": "duumbi:Print",
              "@id": "duumbi:main/main/entry/3",
              "duumbi:operand": { "@id": "duumbi:main/main/entry/2" }
            },
            {
              "@type": "duumbi:Return",
              "@id": "duumbi:main/main/entry/4",
              "duumbi:operand": { "@id": "duumbi:main/main/entry/2" }
            }
          ]
        }
      ]
    }
  ]
}
```

Expected behavior: Compile to native binary. Running the binary prints `8` to stdout and exits with code 8.

## Linking Strategy

**Problem:** Cranelift emits `.o` object files, not executables. A linker is needed to produce the final binary.

**MVP Solution:** Use the system C compiler (`cc`) as the linker.

**Pipeline:**
1. Cranelift emits `output.o` to `.duumbi/build/`
2. DUUMBI invokes: `cc output.o -o output -lc`
3. The `-lc` flag links libc (required for `Print` op's `printf` calls)
4. The resulting `output` binary is placed in `.duumbi/build/`

**Linker Detection:**
1. Check `$CC` environment variable
2. Fall back to `cc` on PATH
3. If neither found, emit error: `"No C compiler found. Install gcc or clang and ensure 'cc' is on your PATH."`

**Runtime Shim:**
DUUMBI compiles a small C runtime shim (`duumbi_runtime.c`) that provides:
- `duumbi_print_i64(int64_t val)` — prints the value followed by newline
- `duumbi_print_f64(double val)` — (Phase 1)
- `duumbi_print_bool(int8_t val)` — (Phase 1)

This shim is compiled once during `duumbi build` and linked alongside user code.

## Error Format Specification

All errors emitted by `duumbi check` and `duumbi build` use this JSON format (one error per line, JSONL):

```json
{
  "level": "error",
  "code": "E001",
  "message": "Type mismatch: Add operation expects matching operand types",
  "nodeId": "duumbi:main/main/entry/2",
  "file": "graph/main.jsonld",
  "details": {
    "expected": "i64",
    "found": "f64",
    "field": "duumbi:left"
  }
}
```

**Required fields:**
| Field | Type | Description |
|---|---|---|
| `level` | string | `"error"`, `"warning"`, or `"info"` |
| `code` | string | Error code (see Error Codes below) |
| `message` | string | Human-readable description |

**Optional fields:**
| Field | Type | Description |
|---|---|---|
| `nodeId` | string | The `@id` of the node that caused the error |
| `file` | string | Relative path to the `.jsonld` file |
| `details` | object | Error-specific structured data |

**Error Codes:**
- `E001` — Type mismatch
- `E002` — Unknown Op type
- `E003` — Missing required field
- `E004` — Orphan reference (node references a nonexistent `@id`)
- `E005` — Duplicate `@id`
- `E006` — Invalid module structure (missing entry function)
- `E007` — Cycle detected in data flow graph
- `E008` — Linking failed
- `E009` — Schema validation failed
- `W001` — Unused node (defined but never referenced)
- `W002` — Unreachable block

The CLI also prints a **human-readable summary** to stderr:
```
error[E001]: Type mismatch in duumbi:main/main/entry/2
  --> graph/main.jsonld
  = Add operation expects matching operand types (expected i64, found f64)
```

## MVP Phases

### Phase 0 — Proof of Concept (The Spike)

> **Goal**: Prove JSON-LD → Cranelift → native binary works.
> **Duration**: 1-2 weeks
> **Kill Criterion**: Compile a hand-written JSON-LD graph (`add(3, 5)`) to a running native binary that prints `8`.

**Deliverables:**
- JSON-LD core schema definition (`core.schema.json`) for Phase 0 ops
- Rust program that reads a `.jsonld` file, validates it, and emits a native binary via Cranelift + `cc`
- One working example: the `add(3, 5)` program from the schema section above

> [!warning] This is the most critical phase. If this fails, the entire DUUMBI thesis is invalid.

### Phase 1 — Usable CLI

> **Goal**: A general developer can install the tool, create a project, write a logic graph, and run it in under 10 minutes.
> **Duration**: 3-4 weeks
> **Kill Criterion**: An external developer (not the author) installs and runs a non-trivial program (with branching and function calls) in < 10 minutes, measured.

| ID | Requirement | Acceptance Criteria |
|---|---|---|
| CLI-01 | Project Init | `duumbi init` creates `.duumbi/` directory with config.toml, schema/, skeleton graph, and README |
| CLI-02 | Build | `duumbi build` discovers all `.jsonld` files in `graph/`, validates, compiles to single native binary |
| CLI-03 | Run | `duumbi run` executes the compiled binary, streams stdout/stderr |
| CLI-04 | Validate | `duumbi check` validates graph against schema, outputs errors in JSONL format (see Error Format) |
| CLI-05 | Describe | `duumbi describe` outputs human-readable pseudo-code summary of the graph |
| CLI-06 | Telemetry | Compiled binary emits structured JSON logs with `traceId` → `nodeId` mapping |

### Phase 2 — AI Integration

> **Goal**: Natural language → JSON-LD graph mutations via external LLM.
> **Duration**: 3-4 weeks
> **Kill Criterion**: >70% of a predefined benchmark set of 20 natural language commands produce a **correct mutation** (defined below).

**Definition of Correct Mutation:**
A mutation is correct if and only if:
1. The generated graph patch passes schema validation (`duumbi check` returns zero errors)
2. The patched graph compiles successfully (`duumbi build` succeeds)
3. The resulting graph diff matches the expected diff from the gold-standard benchmark (structural match, ignoring `@id` values)

| ID | Requirement | Acceptance Criteria |
|---|---|---|
| AI-01 | Add Intent | `duumbi add "description"` sends prompt to configured LLM, receives JSON-LD patch |
| AI-02 | Schema Enforcement | AI output is validated against schema before applying; invalid patches are rejected with error |
| AI-03 | Diff Preview | User sees proposed graph changes in a readable diff before confirming |
| AI-04 | Model Config | `.duumbi/config.toml` stores LLM provider, model name, and API key env var reference |
| AI-05 | Undo | `duumbi undo` reverts the last AI mutation using graph snapshots |

### Phase 3 — Visualization

> **Goal**: Read-only web-based graph view that helps developers understand the graph faster than reading raw JSON-LD.
> **Duration**: 2-3 weeks
> **Kill Criterion**: 3 developers confirm the visualizer accelerates graph understanding vs. raw JSON-LD in a timed comparison test.

| ID | Requirement | Acceptance Criteria |
|---|---|---|
| VIZ-01 | Server | `duumbi viz` starts a localhost web server (default port 8420) |
| VIZ-02 | Graph View | Displays node-based flowchart of functions (nodes = ops, edges = data flow) |
| VIZ-03 | Live Sync | Graph file changes reflect in browser within 1 second via WebSocket |

## Explicitly Out of Scope

- Autonomous self-healing (report errors, do not auto-fix)
- Cloud sync / remote repository (local-only)
- Multi-agent orchestration (single AI agent only)
- Projectional editing
- XML-based ticket system
- Obsidian-like knowledge graph integration
- Complex UI / theming
- Binary cache / package registry
- LLVM backend (Cranelift only for MVP)

## Success Metrics

| Metric | Target | Phase |
|---|---|---|
| Compilation Correctness | 100% of valid graphs produce correct binaries | 0-1 |
| Time-to-Hello-World | Install → running binary < 5 minutes | 1 |
| Roundtrip Integrity | Edit graph → rebuild → run without broken references | 1 |
| AI Accuracy | >70% correct mutations on 20-command benchmark | 2 |
| Visualizer Speedup | 3/3 testers confirm faster understanding vs raw JSON-LD | 3 |

## Key Technical Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Language | Rust | Memory safety, Cranelift integration, performance |
| Compiler Backend | Cranelift (not LLVM) | Lighter, embeddable, faster compile times. LLVM can be added later. |
| Linker | System `cc` | Universal availability, avoids bundling complexity |
| Data Format | JSON-LD (`serde_json`) | Semantic web compatible, schema-enforceable, machine-native |
| CLI Framework | `clap` | Industry standard for Rust CLIs |
| Graph Library | `petgraph` | Proven in-memory graph operations in Rust |
| LLM Provider | Configurable (OpenAI / Anthropic) | Avoid vendor lock-in via config |
| Visualization | WASM + Canvas (localhost) | Zero external dependencies for end user |
| Runtime I/O | C shim linked via `cc` | Simplest path to libc printf for `Print` op |

## Related Documents

- [[DUUMBI - PRD]] — Long-term product vision
- [[DUUMBI - Tools and Components]] — Detailed technical stack
- [[DUUMBI - Task List]] — Implementation breakdown with requirement ID references
- [[DUUMBI - Architecture Diagram]] — Visual component overview
- [[DUUMBI - Glossary]] — Canonical term definitions

---

## Corrections (as of 2026-03-01)

> [!note] These corrections supersede outdated spec entries above.

**Key Technical Decisions — Visualization:** The Phase 3 implementation uses **Cytoscape.js + axum** (not "WASM + Canvas" as written above). The final architecture:
- `axum` 0.8 HTTP server (`duumbi viz`, port 8420)
- Vendored Cytoscape.js rendered in the browser (`src/web/assets/`)
- WebSocket live sync via `tokio::sync::broadcast` + `notify-debouncer-mini`
- No WASM toolchain required

The rationale for the change: Cytoscape.js achieves the same graph visualization UX with zero build complexity (no `wasm-pack`, no WASM runtime). The kill criterion (3/3 devs confirm faster than raw JSON-LD) is unchanged.
