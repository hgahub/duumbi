---
tags:
  - project/duumbi
  - concept/architecture
status: final
created: 2026-03-12
updated: 2026-03-12
related_maps:
  - "[[DUUMBI Core Concepts Map]]"
---
# C4 Model in DUUMBI

DUUMBI maps the C4 architectural model (Context → Container → Component → Code) directly onto JSON-LD graph layers. Each C4 level corresponds to a specific set of DUUMBI entity types.

## Layer Mapping

| C4 Level | DUUMBI Entity Types | Storage | AI Agent (Vision) |
|---|---|---|---|
| **Context** | `duumbi:System`, `duumbi:Actor` | XML (requirements) | Business Analyst |
| **Container** | `duumbi:Container` | JSON-LD (infra config) | Architect / DevOps |
| **Component** | `duumbi:Component` | JSON-LD (interface) | Lead Developer |
| **Code** | `duumbi:Function`, `duumbi:Block`, Op nodes | JSON-LD (implementation) | Coder / Repair |

## Current Status

- **Code level**: fully operational (MVP Phase 0-3)
- **Component level**: representable in graph, manual creation
- **Container/Context levels**: vision — planned for multi-agent orchestration

## Why C4

C4 provides a hierarchical zoom: from business stakeholders (Context) down to individual instructions (Code). In DUUMBI, this hierarchy lives in a single queryable graph, enabling traceability from a business requirement to the binary instruction that implements it.

## Related

- [[JSON-LD Graph Representation]] — the format for all C4 levels
- [[Intent-Driven Development]] — how intents map to C4 levels
- [[DUUMBI - PRD]] — full C4 architecture description
