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
# Intent-Driven Development

In DUUMBI, the developer defines **intent** in natural language. The system transforms this into a structured spec, decomposes it into tasks, and generates JSON-LD graph patches through AI agents.

## The Intent Flow

1. Developer writes natural language intent (e.g., "Build a calculator with add, sub, mul, div")
2. `duumbi intent create` — AI generates a structured YAML spec with acceptance criteria, modules, and test cases
3. **Coordinator Agent** decomposes the spec into ordered tasks (CreateModule, AddFunction, ModifyMain)
4. `duumbi intent execute` — each task is executed via AI graph mutation with 3-step retry
5. **Verifier Agent** runs test cases from the spec
6. If all pass → intent is marked Completed; graph reaches [[Semantic Fixed Point]]

## Intent Spec Format

Stored as `.duumbi/intents/<slug>.yaml`. Contains: intent description, acceptance criteria, module targets (create/modify), test cases with expected return values, and execution metadata.

## Current Status (M5)

The Coordinator uses **rule-based decomposition** (no LLM call): deterministic task generation from the spec. LLM-based decomposition planned for a future milestone.

## Inspiration

Inspired by Augment Intent's spec-driven development model, adapted for semantic graph compilation. The key difference: DUUMBI's spec output is a graph patch, not text code — enabling deterministic validation.

## Related

- [[Semantic Fixed Point]] — the goal of every intent execution
- [[C4 Model in DUUMBI]] — intents map to C4 component/code levels
- [[DUUMBI - MVP Specification]] — Phase 2 (AI Integration) and Phase 6 (Intent Engine)
