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
# Semantic Fixed Point

The **Semantic Fixed Point** is the central invariant of the DUUMBI system.

## Definition

The state where the semantic graph satisfies all three conditions simultaneously:

1. **Schema validity** — the JSON-LD graph passes all schema and type checks
2. **Test compliance** — all defined test cases pass
3. **Intent fulfillment** — the human-specified intent is satisfied

## Significance

The DUUMBI compiler **only produces a binary** when the graph is at its fixed point. A graph that fails any validation cannot be compiled. This is a hard constraint in the compiler, not a guideline.

## Contrast with Traditional Development

In traditional development, code can be syntactically valid but semantically wrong. The compiler does not enforce intent. In DUUMBI, the fixed point concept closes this gap by making validation a prerequisite for compilation, not an afterthought.

## AI Agent Relevance

When an AI agent generates a graph patch, the patch must bring the graph to (or keep it at) its fixed point. The 3-step retry loop in `intent execute` exists to iteratively approach this state: generate → validate → correct.

## Related

- [[JSON-LD Graph Representation]] — the format that enables schema validation
- [[Compilation Pipeline]] — the pipeline that enforces the fixed point
- [[DUUMBI - Glossary]] — canonical definition
