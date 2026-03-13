---
tags:
  - project/duumbi
  - concept/core
status: final
created: 2026-03-12
updated: 2026-03-12
related_maps:
  - "[[DUUMBI Core Concepts Map]]"
  - "[[DUUMBI - Glossary]]"
---
# JSON-LD Graph Representation

DUUMBI represents program logic as a **semantic graph** stored in JSON-LD format, not as textual source code.

## Core Idea

JSON-LD (JavaScript Object Notation for Linked Data) provides semantic typing via `@type` and identity via `@id`, enabling schema validation and graph construction. Every program element is a node in a directed graph with typed edges representing data flow.

## Namespace

All DUUMBI operations live under `https://duumbi.dev/ns/core#` (prefix: `duumbi:`).

## Why JSON-LD over Text

- **Schema-enforceable** — every node must conform to a JSON Schema before acceptance
- **AI-native** — structured output is easier for LLMs than free-form syntax
- **Language-independent** — the graph is abstract; it can target Cranelift, LLVM, or WASM
- **Queryable** — the full program is a graph that can be searched, filtered, and analyzed

## Op Nodes

Elementary operations (Op nodes) map 1:1 to compiler IR instructions: `Const`, `Add`, `Sub`, `Mul`, `Div`, `Print`, `Return`, `Branch`, `Compare`, `Call`, `Load`, `Store`.

## Node Identity

Every node has a unique `@id` with format: `duumbi:<module>/<function>/<block>/<index>`. This enables deterministic traceability from runtime errors back to exact graph nodes.

## Related

- [[Semantic Fixed Point]] — the graph must reach this state before compilation
- [[Compilation Pipeline]] — how the graph becomes a binary
- [[DUUMBI - Glossary]] — canonical term definitions
