---
tags:
  - project/duumbi
  - doc/reference
status: draft
created: 2026-02-17
updated: 2026-03-06
related_maps:
  - "[[DUUMBI - PRD]]"
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - Tools and Components]]"
  - "[[DUUMBI - Post-MVP Implementation Roadmap]]"
  - "[[DUUMBI - Graph Repository Architecture]]"
---
# DUUMBI — Glossary

Canonical term definitions for human developers and AI agents. Every term used across DUUMBI documentation must resolve to exactly one definition in this glossary.

## Core Concepts

**Semantic Fixed Point**
The state where the semantic graph is valid (passes all schema and type checks), all tests pass, and human intent is fulfilled. The compilation pipeline only produces a binary when the graph is at its fixed point. A graph that fails validation is not at its fixed point and cannot be compiled.
*Used in:* [[DUUMBI - PRD]], [[DUUMBI - MVP Specification]]

**Semantic Graph**
The in-memory directed graph (`petgraph::StableGraph<N, E>`) representation of all JSON-LD nodes and their relationships. It is the single source of truth for all operations: validation, compilation, mutation, and visualization. Each node has a unique `nodeId`. `StableGraph` is used (not `DiGraph`) because it preserves `NodeIndex` and `EdgeIndex` values across node/edge removals, which is required so that graph indices remain valid throughout the mutation and compilation pipeline.
*Used in:* [[DUUMBI - MVP Specification]], [[DUUMBI - Architecture Diagram]], [[DUUMBI - Tools and Components]]

**Op (Operation)**
A single instruction node in the semantic graph. Every Op has a `@type` field prefixed with `duumbi:` (e.g., `duumbi:Const`, `duumbi:Add`). Ops are the atomic units of program logic. They map 1:1 to Cranelift IR instructions during compilation.
*Used in:* [[DUUMBI - MVP Specification]], [[DUUMBI - Task List]]

**JSON-LD**
JavaScript Object Notation for Linked Data. The storage format for all executable logic in DUUMBI. Files use the `.jsonld` extension. JSON-LD provides semantic typing via `@type` and identity via `@id`, enabling schema validation and graph construction.
*Used in:* All DUUMBI documents

**Graph Patch**
A JSON-LD fragment produced by the AI Agent Module that describes a mutation to the semantic graph (add, modify, or remove nodes/edges). Patches are validated against the schema before application. A patch that fails validation is rejected.
*Used in:* [[DUUMBI - MVP Specification]] (Phase 2)

**Kill Criterion**
A binary pass/fail condition evaluated at the end of each MVP phase. If the criterion fails, the project stops and reassesses rather than proceeding to the next phase. Every phase has exactly one kill criterion.
*Used in:* [[DUUMBI - MVP Specification]], [[DUUMBI - Task List]]

**Gate Review**
The evaluation event at the end of a phase where the kill criterion is tested. Results are documented. A gate review produces a Go/No-Go decision.
*Used in:* [[DUUMBI - Task List]]

## Graph Structure

**Node**
A vertex in the semantic graph representing a single Op, function definition, or module. Every node has a unique `@id` (the `nodeId`) and a `@type`.

**Edge**
A directed connection between two nodes in the semantic graph. Edges represent data flow (e.g., the output of an `Add` op feeds into a `Print` op) or structural containment (e.g., a function contains a sequence of ops).

**nodeId**
The unique `@id` value assigned to every node in the semantic graph. Format: `duumbi:<module>/<function>/<block>/<index>` (e.g., `duumbi:main/entry/b0/3`). Used for telemetry tracing and crash-to-graph mapping.
*Used in:* [[DUUMBI - MVP Specification]], [[DUUMBI - Tools and Components]]

**traceId**
A UUID assigned to each nodeId at compile time and injected into the compiled binary as structured log metadata. Enables mapping runtime errors back to the exact graph node that caused them.
*Used in:* [[DUUMBI - MVP Specification]] (CLI-06), [[DUUMBI - PRD]]

## Compilation Pipeline

**Cranelift**
A code generation backend written in Rust. Used by DUUMBI (MVP) to transform the validated semantic graph into native machine code. Chosen over LLVM for MVP due to lighter weight, embeddability, and faster compile times. LLVM may be added in future phases.
*Used in:* [[DUUMBI - MVP Specification]], [[DUUMBI - Tools and Components]]

**LLVM IR**
Low-Level Virtual Machine Intermediate Representation. The future (post-MVP) compilation target for advanced optimizations. Not used in MVP; documented in [[DUUMBI - PRD]] as part of the long-term vision.
*Used in:* [[DUUMBI - PRD]]

**Lowering**
The process of transforming validated JSON-LD Op nodes into Cranelift IR instructions. Each `duumbi:Add` becomes a Cranelift `iadd`, each `duumbi:Branch` becomes a conditional branch, etc.
*Used in:* [[DUUMBI - MVP Specification]], [[DUUMBI - PRD]]

**Linking**
The process of combining Cranelift-emitted `.o` object files into a native executable using the system C compiler (`cc`). Linking also resolves libc symbols needed for I/O operations (e.g., `Op:Print`).
*Used in:* [[DUUMBI - MVP Specification]]

## AI Integration

**AI Agent Module**
The component (Phase 2) that translates natural language intent into JSON-LD graph patches via an external LLM API. Configured via `.duumbi/config.toml`.
*Used in:* [[DUUMBI - MVP Specification]], [[DUUMBI - Architecture Diagram]]

**Correct Mutation**
An AI-generated graph patch that: (1) passes schema validation, (2) passes `duumbi check`, and (3) produces the expected graph diff when compared to a gold-standard reference. Used to measure AI accuracy.
*Used in:* [[DUUMBI - MVP Specification]] (Phase 2 kill criterion)

## Project Structure

**Workspace**
A directory initialized with `duumbi init`, containing a `.duumbi/` subdirectory. The workspace uses a three-layer storage model: `graph/` for local source files, `vendor/` for VCS-committed dependency snapshots, and `cache/` for registry-resolved modules. Also contains `config.toml`, `schema/`, `build/`, `telemetry/`, and `history/` directories.
*Used in:* [[DUUMBI - Tools and Components]], [[DUUMBI - Task List]], [[DUUMBI - Graph Repository Architecture]]

**Vault (Vision)**
The long-term concept of a hybrid file system combining `.md`, `.xml`, and `.jsonld` files into a single knowledge graph. Not part of MVP scope. Described in [[DUUMBI - PRD]].
*Used in:* [[DUUMBI - PRD]]

## Graph Repository

**Scope**
A namespace prefix used to group related modules and prevent naming collisions. Format: `@<scope>/<module-name>`. Built-in scopes: `@duumbi/` (official stdlib), `@<user>/` (personal modules), `@<org>/` (organization modules). Unscoped names (e.g., `local-utils`) refer to workspace-local modules only.
*Used in:* [[DUUMBI - Graph Repository Architecture]]

**Workspace Layer**
The first and highest-priority storage layer in the three-layer model. Contains locally developed `.jsonld` source files under `.duumbi/graph/`. Always takes precedence over Vendor and Cache layers during module resolution.
*Used in:* [[DUUMBI - Graph Repository Architecture]]

**Vendor Layer**
The second storage layer under `.duumbi/vendor/`. Contains committed snapshots of dependencies for reproducible offline builds. Managed via `duumbi deps vendor`. Checked into VCS to ensure air-gapped deployments work without network access.
*Used in:* [[DUUMBI - Graph Repository Architecture]]

**Cache Layer**
The third storage layer under `.duumbi/cache/`. Contains registry-resolved modules downloaded on demand. Not committed to VCS. Populated automatically during `duumbi build` or `duumbi deps sync` when a module is not found in Workspace or Vendor layers.
*Used in:* [[DUUMBI - Graph Repository Architecture]]

**Manifest**
A `manifest.toml` file present in every publishable graph module. Declares the module's name, version, exported symbols, license, and minimum compiler version required. Used by the registry for discovery and by the resolver for compatibility checks.
*Used in:* [[DUUMBI - Graph Repository Architecture]]

**Semantic Hash**
A deterministic fingerprint of a graph module's structure and op values, excluding `@id` values. Used in `deps.lock` for binary cache keying and integrity verification. Two modules with the same semantic hash are structurally and semantically identical regardless of their node identity assignments.
*Used in:* [[DUUMBI - Graph Repository Architecture]]

**Resolution Pipeline**
The ordered lookup sequence used to locate a module during `duumbi build`: (1) Workspace Layer → (2) Vendor Layer → (3) Cache Layer → (4) Registry fetch. The first layer to satisfy the request wins. If all layers fail, compilation aborts with error E011.
*Used in:* [[DUUMBI - Graph Repository Architecture]]

---

## Related Documents

- [[DUUMBI - PRD]] — Product requirements and long-term vision
- [[DUUMBI - MVP Specification]] — Phase-by-phase build specification
- [[DUUMBI - Tools and Components]] — Technical stack reference
- [[DUUMBI - Task List]] — Implementation checklist
- [[DUUMBI - Post-MVP Implementation Roadmap]] — Post-MVP milestone plan
- [[DUUMBI - Graph Repository Architecture]] — Graph module storage and namespace design
