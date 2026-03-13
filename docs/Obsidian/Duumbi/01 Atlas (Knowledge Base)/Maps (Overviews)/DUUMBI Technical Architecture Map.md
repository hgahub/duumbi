---
tags:
  - project/duumbi
  - map/technical
status: active
created: 2026-03-12
updated: 2026-03-12
---
# DUUMBI Technical Architecture Map

> This map provides navigation for understanding DUUMBI's technical implementation. Use it to find the right document for any architectural question.

## System Overview

DUUMBI is a Rust CLI application that compiles JSON-LD semantic graphs to native binaries via Cranelift. It includes AI-powered graph mutation, validation, visualization, and an intent-driven development workflow.

## Architecture by Layer

### Compiler Core
- [[Compilation Pipeline]] — the 7-stage pipeline from parsing to linking
- [[JSON-LD Graph Representation]] — the input format and op node taxonomy
- [[DUUMBI - MVP Specification]] — canonical schema reference, type system, error codes

### AI Integration
- [[AI Agent Architecture]] — LLM client, orchestrator, retry logic, MCP vision
- [[Intent-Driven Development]] — spec → coordinator → tasks → graph patches → verification

### Module System
- [[Graph Repository Architecture]] — three-layer storage, namespace scoping, semantic hash
- [[DUUMBI - Graph Repository Architecture]] — full specification with CLI commands and migration plan

### Visualization
- Cytoscape.js + axum HTTP server (port 8420)
- WebSocket live sync via `tokio::sync::broadcast`
- See [[DUUMBI - MVP Specification]] Phase 3

## Technology Stack

| Component | Technology | Crate/Tool |
|---|---|---|
| Language | Rust | — |
| Compiler backend | Cranelift | `cranelift-codegen`, `cranelift-frontend`, `cranelift-module`, `cranelift-object` |
| Graph engine | petgraph | `petgraph` (StableGraph) |
| JSON parsing | serde | `serde`, `serde_json` |
| CLI framework | clap | `clap` |
| Web server | axum | `axum` 0.8 |
| Async runtime | tokio | `tokio` |
| Visualization | Cytoscape.js | vendored in `src/web/assets/` |

## For AI Agents

If you are implementing a feature:
1. Check [[DUUMBI - Tools and Components]] for the current tech stack details
2. Check [[DUUMBI - Task List]] for the implementation checklist
3. Validate all graph mutations against the schema in `.duumbi/schema/core.schema.json`
4. Run `cargo test` before committing — see CLAUDE.md for hooks configuration
