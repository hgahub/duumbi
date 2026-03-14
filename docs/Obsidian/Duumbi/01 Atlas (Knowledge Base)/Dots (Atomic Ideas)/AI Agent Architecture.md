---
tags:
  - project/duumbi
  - concept/development
status: final
created: 2026-03-12
updated: 2026-03-14
related_maps:
  - "[[DUUMBI Core Concepts Map]]"
---
# AI Agent Architecture

DUUMBI employs AI agents that operate on the semantic graph through structured mutations, not free-form code generation.

## Current (M5): Single Agent

One LLM client (Anthropic or OpenAI, configurable via `.duumbi/config.toml`) generates JSON-LD graph patches. The orchestrator applies a 3-step retry loop: generate → validate → correct.

## Planned (Phase 9): Multi-Provider

Multiple LLM providers (Anthropic, OpenAI, Grok, OpenRouter) with fallback chains. An autoresearch-inspired iterative benchmark system tests each provider against showcase applications.

## Planned (Phase 12): Dynamic Agent System with MCP

The DUUMBI CLI becomes an MCP (Model Context Protocol) server. Instead of hardcoded agent types, the system **dynamically assembles agent teams** based on task requirements:

- Agent templates stored as JSON-LD nodes — not hardcoded Rust types
- Task analysis determines which agents to spawn
- Each agent carries knowledge from previous successful operations
- Users see Markdown views of agent knowledge — editable, transparent
- New agent specializations = new graph nodes, not new code

This replaces the manual context management (CLAUDE.md, .cursorrules) that other tools require.

## MCP Tools (Phase 12)

`graph.query`, `graph.mutate`, `graph.validate`, `graph.describe`, `build.compile`, `build.run`, `deps.search`, `deps.install`, `intent.create`, `intent.execute`

## Self-Healing Loop (Phase 13)

1. Runtime telemetry with `traceId` → `nodeId` mapping (OpenTelemetry)
2. Anomaly detection (panic, threshold breach)
3. Back-mapping to exact JSON-LD node
4. Dynamic Repair Agent spawned with context + knowledge
5. Autonomous improvement cycle: try strategy → fail → try different strategy → repeat
6. Successful repairs feed back into agent knowledge graph

## Related

- [[Intent-Driven Development]] — agents execute intent-derived tasks
- [[Semantic Fixed Point]] — every agent mutation must maintain or reach the fixed point
- [[DUUMBI - Phase 9 - Build Excellence & Multi-LLM]] — multi-provider foundation
- [[DUUMBI - Phase 12 - Dynamic Agent System & MCP]] — full dynamic agent specification
- [[DUUMBI - Phase 13 - Self-Healing & Telemetry]] — self-healing specification
