# DUUMBI — Architecture Reference

> Companion to CLAUDE.md. Claude Code reads this when working on structural,
> cross-component, or pipeline-level tasks.

## Core thesis

Program logic is stored as a **typed semantic graph** (JSON-LD), not as text
files. AI generates and mutates the graph. The toolchain validates every
mutation before compilation. Syntax errors are structurally impossible.

**Semantic Fixed Point:** a graph is compilable only when it passes schema
validation, type checking, and all tests pass.

---

## Component map

```
.jsonld files
    │
    ▼
JSON-LD Parser          (serde_json → typed AST)
    │
    ▼
Semantic Graph          (petgraph::StableGraph — single source of truth)
    │         │
    ▼         ▼
Schema        Cranelift Compiler    (graph nodes → Cranelift IR → .o)
Validator         │
    │             ▼
    │         Linker (cc)           ($CC env or `cc` fallback)
    │             │
    ▼             ▼
Error JSONL   Native Binary

Phase 2 additions:
  AI Agent Module  →  Graph Patch  →  Schema Validator  →  Semantic Graph

Phase 3 additions:
  Telemetry Engine  →  traceId injection  →  Web Visualizer (WASM + axum)
```

---

## Data formats

| Format | Role | Location |
|--------|------|----------|
| `.jsonld` | Source of truth for program logic | `.duumbi/graph/` |
| `core.schema.json` | JSON Schema for Op node validation | `.duumbi/schema/` |
| `.o` | Cranelift object file output | `.duumbi/build/` |
| `traces.jsonl` | Runtime traceId → nodeId mapping | `.duumbi/telemetry/` |
| `config.toml` | LLM provider, model, API key env ref | `.duumbi/` |

**JSON-LD namespace:** `https://duumbi.dev/ns/core#` (prefix: `duumbi:`)

**nodeId format:** `duumbi:<module>/<function>/<block>/<index>`
Example: `duumbi:main/main/entry/2`

---

## Phase 0 pipeline (current build target)

```
add.jsonld  →  parse  →  DiGraph  →  validate  →  Cranelift IR  →  output.o
                                                                        │
                                         cc output.o duumbi_runtime.o -o output
```

Kill criterion: `add(3, 5)` → binary prints `8`, exits with code 8.

**Phase 0 Op set:** `Const`, `Add`, `Sub`, `Mul`, `Div`, `Print`, `Return`

**Cranelift lowering map:**

| duumbi: Op | Cranelift IR |
|------------|--------------|
| `Const` (i64) | `iconst` |
| `Add` | `iadd` |
| `Sub` | `isub` |
| `Mul` | `imul` |
| `Div` | `sdiv` |
| `Print` | `call duumbi_print_i64` |
| `Return` | `return` |

---

## Linker strategy

1. Check `$CC` env var
2. Fall back to `cc` on PATH
3. Command: `cc output.o duumbi_runtime.o -o output -lc`

`duumbi_runtime.c` provides: `duumbi_print_i64(int64_t)`,
`duumbi_print_f64(double)` (Phase 1), `duumbi_print_bool(int8_t)` (Phase 1).

---

## Error format (JSONL to stdout)

```json
{
  "level": "error",
  "code": "E001",
  "message": "Type mismatch: Add expects matching operand types",
  "nodeId": "duumbi:main/main/entry/2",
  "file": "graph/main.jsonld",
  "details": { "expected": "i64", "found": "f64", "field": "duumbi:left" }
}
```

Error codes: E001 type mismatch · E002 unknown Op · E003 missing field ·
E004 orphan reference · E005 duplicate @id · E006 no entry function ·
E007 cycle · E008 link failed · E009 schema invalid

---

## Phase roadmap

| Phase | Goal | Kill criterion |
|-------|------|----------------|
| 0 | JSON-LD → native binary | `add(3,5)` prints `8` |
| 1 | Usable CLI | External dev installs + runs in < 10 min |
| 2 | AI graph mutation | > 70% correct on 20-cmd benchmark |
| 3 | Web visualizer | 3/3 devs confirm faster than raw JSON-LD |

Phases beyond MVP (A–D): Knowledge base, Agent swarm, Self-healing, IDE.
