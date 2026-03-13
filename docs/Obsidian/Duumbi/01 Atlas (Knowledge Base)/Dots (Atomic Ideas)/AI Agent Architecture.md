---
tags:
  - project/duumbi
  - concept/development
status: final
created: 2026-03-12
updated: 2026-03-12
related_maps:
  - "[[DUUMBI Core Concepts Map]]"
---
# AI Agent Architecture

DUUMBI employs specialized AI agents that operate on the semantic graph through structured mutations, not free-form code generation.

## Current (M5): Single Agent

One LLM client (Anthropic or OpenAI, configurable via `.duumbi/config.toml`) generates JSON-LD graph patches. The orchestrator applies a 3-step retry loop: generate → validate → correct.

## Planned (Vision): Multi-Agent with MCP

The DUUMBI CLI becomes an MCP (Model Context Protocol) server, exposing tools to specialized agents:

| Agent | Responsibility | Trigger |
|---|---|---|
| **Architect** | C4 structure, module boundaries | Intent spec received |
| **Coder** | Function implementation (Op generation) | Task assignment |
| **Reviewer** | Security and performance validation | Graph patch submitted |
| **Tester** | Test case generation and execution | Implementation complete |
| **Ops** | Runtime monitoring | Telemetry alert |
| **Repair** | Faulty graph fragment correction | Ops alert |

## MCP Tools (Planned)

`graph.query`, `graph.mutate`, `graph.validate`, `build.compile`, `build.run`, `telemetry.query`, `deps.search`, `deps.install`

## Self-Healing Loop (Vision)

1. Runtime telemetry with `traceId` → `nodeId` mapping
2. Anomaly detection (panic, threshold breach)
3. Back-mapping to exact JSON-LD node
4. Repair Agent generates corrective graph patch
5. Validate → rebuild → test → deploy or PR

## Related

- [[Intent-Driven Development]] — agents execute intent-derived tasks
- [[Semantic Fixed Point]] — every agent mutation must maintain or reach the fixed point
- [[DUUMBI - PRD]] — Vision Phase B (Agent Swarm) and Phase C (Self-Healing)
