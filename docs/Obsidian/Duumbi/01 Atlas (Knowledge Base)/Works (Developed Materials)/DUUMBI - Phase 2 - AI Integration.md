---
tags:
  - project/duumbi
  - milestone/phase-2
status: complete
github_milestone: "Phase 2: AI Integration"
github_issues: "13/13 closed"
completed: 2026-02
---
# Phase 2 — AI Integration ✅

> **Kill Criterion:** > 70% correct mutations on 20-command benchmark set.
> **Eredmény:** ✅ 20/20 (100%) — PR #39 merged

← Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

AI-alapú gráfmutáció implementálva tool use / function calling API-val. Mindkét provider (Anthropic + OpenAI) támogatott. Git-like undo history.

## Főbb deliverable-ök

### AI pipeline
- [x] `LLmProvider` trait — Anthropic + OpenAI implementációk
- [x] 6 tool/function: `add_function`, `add_block`, `add_op`, `modify_op`, `remove_node`, `set_edge`
- [x] `GraphPatch` — all-or-nothing alkalmazás JSON-LD Value szinten
- [x] `duumbi add "<kérés>"` — LLM → patch → validáció → alkalmazás
- [x] Max 1 retry hibaüzenettel a promptba fűzve
- [x] Diff preview + felhasználói jóváhagyás (--yes flag)

### Undo rendszer
- [x] `duumbi undo` — LIFO snapshot stack (`.duumbi/history/`)
- [x] Snapshot minden mutáció előtt

### Benchmark
- [x] 20 parancs + gold-standard graph diff fixtures
- [x] Mock LLM responses — determinisztikus CI tesztek
- [x] Eredmény: **20/20** ✅

## GitHub

- Milestone: **Phase 2: AI Integration** (#3)
- Issues: #26–#38 (13/13 lezárva)
- PR: #39 (merged)

## Architektúra döntések

- `serde` tag a PatchOp-on: `"kind"` (nem `"op"` — mezőnév-konfliktus)
- Tool use (structured output), nem free-form JSON
- `tokio` + `reqwest` alapértelmezett dep (nem feature gate)
