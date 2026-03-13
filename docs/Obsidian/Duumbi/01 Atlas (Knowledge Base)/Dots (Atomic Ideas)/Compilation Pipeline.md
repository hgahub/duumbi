---
tags:
  - project/duumbi
  - concept/core
status: final
created: 2026-03-12
updated: 2026-03-12
related_maps:
  - "[[DUUMBI Core Concepts Map]]"
---
# Compilation Pipeline

The DUUMBI compilation pipeline transforms a validated JSON-LD semantic graph into a native binary. There is no intermediate human-readable programming language.

## Pipeline Stages

1. **Parsing** — `serde_json` reads `.jsonld` files into memory
2. **Graph Construction** — parsed nodes assembled into a `petgraph::StableGraph` directed graph
3. **Validation** — schema validation, type checking, reference integrity (no orphan nodes)
4. **Lowering** — validated graph nodes transformed to Cranelift IR instructions (e.g., `duumbi:Add` → `iadd`)
5. **Optimization** — Cranelift's built-in optimization passes (dead code elimination, constant folding)
6. **Object Emission** — Cranelift emits a `.o` object file to `.duumbi/build/`
7. **Linking** — system `cc` links the object file + libc into a native executable

## Key Properties

- **Deterministic**: identical graph → identical binary (guaranteed)
- **No intermediate language**: JSON-LD → Cranelift IR → machine code
- **Incremental** (future): only recompile modules whose semantic hash changed

## Backend Choice

**MVP**: Cranelift — lighter, embeddable, faster compile times.
**Future**: LLVM via `inkwell` crate — advanced optimizations, wider target support (WASM, RISC-V).

## Related

- [[Semantic Fixed Point]] — compilation only proceeds when the graph is at its fixed point
- [[JSON-LD Graph Representation]] — the input format
- [[DUUMBI - Glossary]] — canonical definitions
