---
tags:
  - project/duumbi
  - milestone/phase-0
status: complete
github_milestone: "Phase 0: Proof of Concept"
github_issues: "9/9 closed"
completed: 2026-02
---
# Phase 0 — Proof of Concept ✅

> **Kill Criterion:** `add(3, 5)` JSON-LD graph → native binary prints `8`, exits with code 8.
> **Eredmény:** ✅ Teljesítve

← Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

Az első fázis célja: bebizonyítani, hogy a JSON-LD gráf → Cranelift → natív bináris pipeline működőképes. A teljes kompilációs lánc end-to-end implementálva lett, beleértve a C runtime shimmel való linkelést.

## Főbb deliverable-ök

- [x] `core.schema.json` — Phase 0 op-ok sémája
- [x] JSON-LD parser: `.jsonld` → `serde_json::Value` → validáció
- [x] Graph builder: `petgraph::StableGraph` felépítése az AST-ből
- [x] Cranelift codegen: `Const`, `Add`, `Sub`, `Mul`, `Div`, `Return`, `Print`
- [x] C runtime shim: `duumbi_print_i64(int64_t)`
- [x] Linker: `$CC` / `cc` fallback, `.o` → binary
- [x] Integration test: `add.jsonld` → binary → `8\n` + exit code 8
- [x] Gate Review: ✅ Go

## GitHub

- Milestone: **Phase 0: Proof of Concept** (#1)
- Issues: #5–#13 (9/9 lezárva)
- Branch: `main` (merged)

## Architektúra döntések

- `petgraph::StableGraph` (nem `DiGraph`) — node indices életben maradnak törléskor
- C runtime `include_str!` makróval beágyazva — hordozhatóság
- Error JSONL stdout-ra, human-readable stderr-re
- Newtypes: `NodeId`, `EdgeId` (nem raw String/u32)
