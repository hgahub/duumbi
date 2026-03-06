---
tags:
  - project/duumbi
  - doc/architecture
status: final
created: 2026-02-15
updated: 2026-03-06
related_maps:
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - PRD]]"
  - "[[DUUMBI - Tools and Components]]"
  - "[[DUUMBI - Glossary]]"
  - "[[DUUMBI - Graph Repository Architecture]]"
---
# DUUMBI — Architecture Diagram

![[DUUMBI - Architecture Diagram.excalidraw|Architecture Diagram]]

## Diagram Overview

This diagram illustrates the data flow and component interaction for the DUUMBI pipeline across its MVP phases and post-MVP milestones. For component details, see [[DUUMBI - Tools and Components]]. For terminology, see [[DUUMBI - Glossary]].

1. **Phase 0 (Green)**: Core compilation pipeline. Developer writes JSON-LD → Parser → Semantic Graph → Cranelift Compiler → `.o` object → **Linker (`cc`)** → Native Binary.
2. **Phase 1 (Yellow)**: Validation and CLI usability. The Schema Validator ensures graph integrity before compilation. Error output follows [[DUUMBI - MVP Specification#Error Format Specification]].
3. **Phase 2 (Purple)**: AI Integration. The AI Agent Module mutates the graph based on natural language intent, calling external LLM APIs. Patches are validated before application.
4. **Phase 3 (Orange)**: Visualization. Telemetry injects traceIds into the binary. The Web Visualizer renders the graph state via local WebSocket.
5. **Phase 4 (Blue)**: Module System. Multi-module compilation, cross-module reference validation, FNV-1a lockfile, stdlib (math/io), and `duumbi deps` CLI.

## Component Roles

- **JSON-LD Parser**: Reads `.jsonld` files, validates against `core.schema.json`, outputs `serde_json::Value` tree.
- **Semantic Graph**: The single source of truth (`petgraph::StableGraph<N, E>`). All operations (compile, validate, mutate, visualize) read from or write to this graph. `StableGraph` ensures node/edge indices survive removals.
- **Schema Validator**: Enforces the `duumbi:` ontology rules — type checking, reference integrity, structural constraints. Outputs structured errors (JSONL).
- **Cranelift Compiler**: Transforms validated graph nodes into Cranelift IR, emits `.o` object file.
- **Linker**: Invokes system `cc` to combine `.o` files + runtime shim → native executable. Detects `$CC` env var with `cc` fallback.
- **Runtime Shim**: Small C library (`duumbi_runtime.c`) providing I/O functions (`duumbi_print_i64`, etc.) linked via `cc`.
- **AI Agent Module**: Orchestrates the "Intent → JSON-LD Patch" workflow using external LLMs (Anthropic/OpenAI). Schema-validates all patches before applying. Supports undo via snapshot history.
- **Telemetry Engine**: Maps runtime binary execution back to graph `nodeId`s for crash-to-graph debugging.
- **Web Visualizer**: Read-only browser view of the graph. Implemented with Cytoscape.js (vendored JS) + axum 0.8 HTTP server + WebSocket live sync. No WASM required.
- **Module System** (Phase 4): `Program::load()` discovers all `.jsonld` modules; `compile_program()` + `link_multi()` compile each to `.o` and link together.
- **Graph Repository** (M7): Three-layer storage (workspace/vendor/cache) with scope-based namespacing (`@scope/module`). Registry fetch with `deps.lock` integrity verification.

## Data Flow

```
.jsonld files  →  JSON-LD Parser  →  Semantic Graph  →  Schema Validator
                                                              │
                                                    Cranelift Compiler
                                                              │
                                                          output.o
                                                              │
                                                        Linker (cc)
                                                              │
                                                    output (executable)
```

**Module resolution order** (Phase 4+):
```
import request  →  Workspace (.duumbi/graph/)
                →  Vendor (.duumbi/vendor/)
                →  Cache (.duumbi/cache/)
                →  Registry fetch  →  E011 if not found
```

## Related Documents

- [[DUUMBI - MVP Specification]] — Authoritative build specification
- [[DUUMBI - PRD]] — Long-term product vision
- [[DUUMBI - Tools and Components]] — Detailed component descriptions
- [[DUUMBI - Task List]] — Implementation breakdown
- [[DUUMBI - Glossary]] — Term definitions
- [[DUUMBI - Graph Repository Architecture]] — Graph module storage and namespace design
