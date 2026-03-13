---
tags:
  - project/duumbi
  - map/overview
status: active
created: 2026-03-12
updated: 2026-03-12
---
# DUUMBI Core Concepts Map

> This map connects the foundational ideas that define DUUMBI. Start here to understand what DUUMBI is and how its pieces fit together.

## The Central Thesis

DUUMBI is an AI-first semantic graph compiler. Software is represented as a JSON-LD graph, not text files. AI generates structured data, not character sequences. What can be validated by schema cannot be hallucination.

## Core Concept Notes (Dots)

### Representation
- [[JSON-LD Graph Representation]] — why JSON-LD, how ops map to IR instructions
- [[C4 Model in DUUMBI]] — how C4 architecture layers map to graph layers

### Compilation
- [[Semantic Fixed Point]] — the central invariant: valid schema + passing tests + fulfilled intent
- [[Compilation Pipeline]] — JSON-LD → Cranelift IR → native binary, no intermediate language

### Development Workflow
- [[Intent-Driven Development]] — developer defines intent, AI generates graph patches
- [[AI Agent Architecture]] — single agent (current) → multi-agent MCP orchestration (vision)

### Infrastructure
- [[Graph Repository Architecture]] — three-layer module storage (Workspace → Vendor → Cache)
- [[Open Source Monetization Model B]] — fully open source + managed service revenue

## Reference Documents (Works)

- [[DUUMBI - PRD]] — long-term product vision and C4 architecture
- [[DUUMBI - MVP Specification]] — authoritative build specification (Phase 0-3)
- [[DUUMBI - Glossary]] — canonical term definitions
- [[DUUMBI - Post-MVP Roadmap]] — business plan and monetization strategy
- [[DUUMBI - Post-MVP Implementation Roadmap]] — executable milestone plan (M4-M8+)
- [[DUUMBI - Graph Repository Architecture]] — detailed registry and namespace design

## For AI Agents

If you are an AI agent working on DUUMBI:
1. Read [[DUUMBI - Glossary]] first for canonical term definitions
2. Read [[DUUMBI - MVP Specification]] for current build scope and schema reference
3. Check [[DUUMBI - Post-MVP Implementation Roadmap]] for the active milestone
4. Every graph mutation must reach the [[Semantic Fixed Point]] before compilation
5. Follow the JSON-LD schema at `.duumbi/schema/core.schema.json`
