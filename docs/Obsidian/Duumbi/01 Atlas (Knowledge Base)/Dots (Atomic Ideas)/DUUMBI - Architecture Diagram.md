---
tags:
  - project/duumbi
  - doc/architecture
status: final
created: 2026-02-15
updated: 2026-02-17
related_maps:
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - PRD]]"
  - "[[DUUMBI - Tools and Components]]"
  - "[[DUUMBI - Glossary]]"
---
# DUUMBI — Architecture Diagram

![[DUUMBI - Architecture Diagram.excalidraw|Architecture Diagram]]

## Diagram Overview

This diagram illustrates the data flow and component interaction for the DUUMBI MVP across its 4 phases. For component details, see [[DUUMBI - Tools and Components]]. For terminology, see [[DUUMBI - Glossary]].

1. **Phase 0 (Green)**: Core compilation pipeline. Developer writes JSON-LD → Parser → Semantic Graph → Cranelift Compiler → `.o` object → **Linker (`cc`)** → Native Binary.
2. **Phase 1 (Yellow)**: Validation and CLI usability. The Schema Validator ensures graph integrity before compilation. Error output follows [[DUUMBI - MVP Specification#Error Format Specification]].
3. **Phase 2 (Purple)**: AI Integration. The AI Agent Module mutates the graph based on natural language intent, calling external LLM APIs. Patches are validated before application.
4. **Phase 3 (Orange)**: Visualization. Telemetry injects traceIds into the binary. The Web Visualizer renders the graph state via local WebSocket.

## Component Roles

- **JSON-LD Parser**: Reads `.jsonld` files, validates against `core.schema.json`, outputs `serde_json::Value` tree.
- **Semantic Graph**: The single source of truth (`petgraph::DiGraph`). All operations (compile, validate, mutate, visualize) read from or write to this graph.
- **Schema Validator**: Enforces the `duumbi:` ontology rules — type checking, reference integrity, structural constraints. Outputs structured errors (JSONL).
- **Cranelift Compiler**: Transforms validated graph nodes into Cranelift IR, emits `.o` object file.
- **Linker**: Invokes system `cc` to combine `.o` files + runtime shim → native executable. Detects `$CC` env var with `cc` fallback.
- **Runtime Shim**: Small C library (`duumbi_runtime.c`) providing I/O functions (`duumbi_print_i64`, etc.) linked via `cc`.
- **AI Agent Module**: Orchestrates the "Intent → JSON-LD Patch" workflow using external LLMs. Schema-validates all patches before applying.
- **Telemetry Engine**: Maps runtime binary execution back to graph `nodeId`s for crash-to-graph debugging.
- **Web Visualizer**: Read-only browser view of the graph with WebSocket live sync.

## Data Flow (Phase 0)

```
.jsonld files → JSON-LD Parser → Semantic Graph → Schema Validator → Cranelift Compiler → output.o → Linker (cc) → output (executable)
```

## Related Documents

- [[DUUMBI - MVP Specification]] — Authoritative build specification
- [[DUUMBI - PRD]] — Long-term product vision
- [[DUUMBI - Tools and Components]] — Detailed component descriptions
- [[DUUMBI - Task List]] — Implementation breakdown
- [[DUUMBI - Glossary]] — Term definitions

---

## Corrections (as of 2026-03-01)

> [!note] These corrections supersede the outdated descriptions above.

- **Semantic Graph** uses `petgraph::StableGraph` (not `DiGraph` as referenced above). `StableGraph` ensures node/edge indices remain stable across mutations — required for the graph IR to survive mutation operations. See `src/graph/` for the canonical implementation.
- **Web Visualizer** uses **Cytoscape.js + axum** (not WASM + Canvas). No WASM toolchain needed. Cytoscape.js is vendored in `src/web/assets/`. Phase 3 PR #40.
