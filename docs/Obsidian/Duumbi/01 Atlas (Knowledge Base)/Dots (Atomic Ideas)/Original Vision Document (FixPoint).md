---
tags:
  - project/duumbi
  - concept/history
status: archived
created: 2026-03-12
updated: 2026-03-12
related_maps:
  - "[[DUUMBI Core Concepts Map]]"
---
# Original Vision Document (FixPoint)

The initial DUUMBI concept was called **"FixPoint"** and was written as an extensive Hungarian-language PRD. This document established the foundational ideas that evolved into the current DUUMBI platform.

## Key Ideas Carried Forward

- **Semantic Fixed Point** concept — software isn't ready until the graph is valid, tests pass, and intent is fulfilled
- **JSON-LD as program representation** — instead of textual source code
- **C4 model mapping** — architecture layers correspond to graph layers
- **AI agent swarm** — specialized agents (Architect, Coder, Reviewer, Ops, Repair)
- **Self-healing loop** — traceId → nodeId back-mapping for autonomous error correction
- **Obsidian-like knowledge management** — hybrid .md/.xml/.jsonld file system

## Key Changes Since Original

| Original Vision | Current Implementation |
|---|---|
| "FixPoint" name | "DUUMBI" |
| LLVM via `inkwell` | Cranelift (MVP), LLVM as future option |
| XML for business objects | Deferred to Vision Phase A |
| Full IDE with projectional editing | CLI-first, viz as separate web UI |
| `fp:` namespace prefix | `duumbi:` namespace prefix |

## Location

The original Hungarian text is preserved at `00 Inbox (ToProcess)/Original PRD.md` for historical reference. The current English PRD at [[DUUMBI - PRD]] supersedes it entirely.

## Related

- [[DUUMBI - PRD]] — the current, authoritative vision document
- [[Semantic Fixed Point]] — the concept that survived unchanged from the original
