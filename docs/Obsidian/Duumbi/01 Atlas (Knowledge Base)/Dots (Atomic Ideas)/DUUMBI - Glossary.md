---
tags:
  - project/duumbi
  - doc/reference
status: draft
created: 2026-02-17
updated: 2026-02-17
related_maps:
  - "[[DUUMBI - PRD]]"
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - Tools and Components]]"
---
# DUUMBI — Glossary

Canonical term definitions for human developers and AI agents. Every term used across DUUMBI documentation must resolve to exactly one definition in this glossary.

## Core Concepts

**Semantic Fixed Point**
The state where the semantic graph is valid (passes all schema and type checks), all tests pass, and human intent is fulfilled. The compilation pipeline only produces a binary when the graph is at its fixed point. A graph that fails validation is not at its fixed point and cannot be compiled.
*Used in:* [[DUUMBI - PRD]], [[DUUMBI - MVP Specification]]

**Semantic Graph**
The in-memory directed graph (`petgraph`) representation of all JSON-LD nodes and their relationships. It is the single source of truth for all operations: validation, compilation, mutation, and visualization. Each node has a unique `nodeId`.
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
A directory initialized with `duumbi init`, containing a `.duumbi/` subdirectory with `config.toml`, `schema/`, `graph/`, `build/`, and `telemetry/` directories.
*Used in:* [[DUUMBI - Tools and Components]], [[DUUMBI - Task List]]

**Vault (Vision)**
The long-term concept of a hybrid file system combining `.md`, `.xml`, and `.jsonld` files into a single knowledge graph. Not part of MVP scope. Described in [[DUUMBI - PRD]].
*Used in:* [[DUUMBI - PRD]]

---

## Corrections (as of 2026-03-01)

> [!note] These corrections supersede outdated definitions above.

**Semantic Graph** — Implementation note: The actual type is `petgraph::StableGraph<N, E>` (not a bare `petgraph::DiGraph`). `StableGraph` preserves `NodeIndex` and `EdgeIndex` values across node/edge removals, which is required so that graph indices remain valid throughout the mutation and compilation pipeline. The `DiGraph` mention above reflects the original design; `StableGraph` was chosen during Phase 0 implementation and is now the canonical type.
