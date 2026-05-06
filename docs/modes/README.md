# DUUMBI Operating Modes

This directory contains the proposed operating-mode architecture for DUUMBI.

The short version:

- `query`: read-only understanding, explanation, navigation, diagnostics, and architectural conversation.
- `agent`: direct graph mutation for bounded changes.
- `intent`: spec-driven development for larger goals with acceptance criteria, task decomposition, execution, and verification.

## Documents

- [Operating Modes Research](operating-modes-research.md) - current-state findings from repository code and the Obsidian vault.
- [Query Mode Specification](query-mode-spec.md) - product behavior, system contract, UX, and service architecture.
- [Implementation Tasks](implementation-tasks.md) - concrete development backlog with file scope, tests, and acceptance criteria.

## Design Position

`query` should be a first-class mode, not a weaker variant of `agent`.

The existing DUUMBI vision says the developer acts as architect and validator, while the system owns a queryable semantic graph. That workflow needs a safe conversational surface where the developer can ask what exists, why something behaves a certain way, where a change belongs, and what risks are visible before allowing graph mutations.

The user experience should make mode boundaries obvious:

- Asking questions should never mutate the graph.
- Mutation requests should create snapshots and remain undoable.
- Larger feature work should route through intent specs, acceptance criteria, and verification.
- Switching from understanding to action should preserve context, but require an explicit write-capable handoff.
