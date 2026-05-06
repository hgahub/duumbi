# DUUMBI Operating Modes Research

Status: proposed
Date: 2026-05-01

## Purpose

This document records the research pass behind the proposed `query` mode and the revised operating-mode model for DUUMBI.

The question under review: DUUMBI currently exposes `agent` and `intent` behavior. Should it also expose a first-class `query` mode where the user can ask questions and the system answers from its own state, properties, graph, session history, and knowledge?

## Reviewed Sources

Repository code:

- `src/cli/mode.rs`
- `src/cli/repl.rs`
- `src/cli/app.rs`
- `src/cli/completion.rs`
- `src/agents/mod.rs`
- `src/agents/orchestrator.rs`
- `src/context/mod.rs`
- `src/context/analyzer.rs`
- `src/knowledge/mod.rs`
- `src/knowledge/store.rs`
- `src/session/mod.rs`
- `src/mcp/tools/graph.rs`
- `crates/duumbi-studio/src/ws.rs`
- `crates/duumbi-studio/src/server_fns.rs`
- `docs/architecture.md`

Obsidian vault:

- `00 Inbox (ToProcess)/Original PRD.md`
- `01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Phase 4 - Interactive CLI & Module System.md`
- `01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Phase 5 - Intent-Driven Development.md`
- `01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Phase 10 - Intelligent Context & Knowledge Graph.md`
- `01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Phase 11 - CLI UX & Developer Experience.md`
- `01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Phase 12 - Dynamic Agent System & MCP.md`
- `01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Phase 15 - Studio Workflow Redesign.md`
- `01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - CLI Interactive Surface Map.md`
- `01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/AI Agent Development Workflow.md`
- `01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Intent-Driven Development.md`

## Current-State Facts

### REPL modes are currently CLI-local

`src/cli/mode.rs` defines `ReplMode` with only two variants:

- `Agent`
- `Intent`

The enum is documented as an Agent/Intent dual-mode system. It is not a shared library-level interaction model.

### Natural-language input currently means mutation or intent work

`src/cli/repl.rs` dispatches non-slash input like this:

- `ReplMode::Agent` -> `handle_ai_request(...)`
- `ReplMode::Intent` -> `handle_intent_input(...)`

There is no branch for read-only question answering.

### Agent mode is write-capable by default

`handle_ai_request(...)`:

- reads `.duumbi/graph/main.jsonld`
- calls `orchestrator::mutate_streaming(...)`
- saves a snapshot
- writes the patched graph
- auto-builds
- records the turn in session history

This is correct for direct mutation, but unsafe as the default behavior for questions.

### Intent mode is spec-driven

`handle_intent_input(...)` treats free-form input as:

- intent creation if no intent is focused
- intent execution for `execute` or `run`
- intent modification if an intent is focused

This is a useful write-capable mode, but it is not an interactive knowledge surface.

### Studio chat is mutation-first

`crates/duumbi-studio/src/ws.rs` receives chat messages and always routes them to `orchestrator::mutate_streaming(...)`. On success it writes patched JSON-LD and sends a refresh frame.

The `ChatRequest` protocol contains `type`, `message`, `module`, and `c4_level`, but no mode. Studio therefore lacks the same explicit interaction boundary the REPL has.

### Provider abstraction is tool-call oriented

`src/agents/mod.rs` defines `LlmProvider` with:

- `call_with_tools(...) -> Vec<PatchOp>`
- `call_with_tools_streaming(...) -> Vec<PatchOp>`

There is no provider-level method for plain explanatory text, no structured answer type, and no citation/source contract.

### Context and knowledge foundations already exist

`src/context/mod.rs` implements an assembly pipeline:

1. classify
2. traverse
3. collect
4. budget
5. few-shot

`src/context/analyzer.rs` can scan workspace modules and function signatures. `src/knowledge/store.rs` supports file-based knowledge node loading and querying by type or tag. `src/session/mod.rs` persists conversation turns and usage stats.

These are strong foundations for `query` mode, but the current context pipeline is mutation-oriented and not yet answer-oriented.

### MCP has graph query tools, but not conversational query mode

`src/mcp/tools/graph.rs` exposes graph tools such as query, mutate, validate, and describe. This gives external agents low-level graph access, but it does not create a user-facing interactive explanation mode.

### Documentation already points toward queryable understanding

The vault PRD states that DUUMBI's primary artifact is a semantic graph and that AI context should be the "full queryable graph." Phase 10 says the system should manage its own context rather than forcing users to maintain instruction files. Phase 15 makes the Graph panel plus chat the core Studio experience.

These documents make `query` mode a natural missing product capability, not an unrelated feature request.

## Interpretation

`query` mode is needed because DUUMBI's core promise is not only "AI mutates a graph." The stronger promise is "the semantic graph is understandable, traversable, validated, and usable as the developer's working model."

Without `query` mode, a user asking "What does this module do?", "Where should this function live?", "Why did this build fail?", or "What is the risk of changing this signature?" is forced into a write-capable path. That is a UX and safety mismatch.

## Proposed Mode Model

| Mode | Primary job | Mutates graph | Persistent artifact | Best for |
|---|---|---:|---|---|
| `query` | Understand, explain, inspect, diagnose, compare, advise | No | Session turn, optional knowledge note | Questions, architecture discussion, risk analysis, graph navigation |
| `agent` | Apply a bounded change now | Yes | Snapshot, graph patch, build result, session turn | Add/fix/refactor a known target |
| `intent` | Convert a goal into a reviewed spec and verified execution | Yes | Intent YAML, task plan, archive, verification report | Larger features, multi-module work, acceptance criteria |

## Key Gap

The main architectural gap is that "interaction mode" is currently a CLI rendering concern rather than a shared DUUMBI application concern.

Adding `Query` only to `src/cli/mode.rs` would fix the REPL surface, but Studio and MCP would remain divergent. The better approach is to introduce a shared interaction layer that CLI and Studio can both call.

## Recommended Direction

Build `query` as a first-class read-only service:

- Shared `InteractionMode` enum in library code.
- Query engine with explicit read-only data access.
- Provider text-answer API separate from graph-patch API.
- CLI and Studio mode selectors backed by the same mode model.
- Session recording for questions and answers.
- Optional handoff from query answer to agent or intent, with explicit user action.

This gives DUUMBI a smoother development experience while preserving the Semantic Fixed Point discipline: understand first, mutate only through a write-capable mode, validate every write.
