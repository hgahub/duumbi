---
tags:
  - project/duumbi
  - milestone/phase-5
status: complete
github_milestone: "Phase 5: Intent-Driven Development"
github_issues: "22/22 closed"
completed: 2026-03-06
---
# Phase 5 — Intent-Driven Development ✅

> **Kill Criterion:** Intent spec → multi-modul app (min 2 modul, min 3 függvény), tesztek zöldek, `duumbi build && duumbi run` fut.
> **Eredmény:** ✅ `double(21)=42` és `double(0)=0` — 209 teszt zöld (13 új Phase 5 integration test)

← Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

Spec-vezérelt fejlesztési mód: a fejlesztő strukturált YAML intent spec-et ír (vagy LLM generálja), a Coordinator Task-okra bontja, a Verifier ellenőrzi az eredményt. 3-lépéses retry pipeline.

## Teljesített feladatok

### Intent rendszer (M5-INT)
- [x] Intent spec formátum stabilizálás (YAML) — `.duumbi/intents/<slug>.yaml`
- [x] `duumbi intent create "<szándék>"` — LLM generálja a spec-et
- [x] `duumbi intent review [name]` — spec megjelenítés, `--edit` flag
- [x] `duumbi intent execute <name>` — Coordinator → mutáció → Verifier → archive
- [x] `duumbi intent status [name]` — végrehajtás állapota
- [x] `/intent` slash parancs a REPL-ben
- [x] Intent history archív — `.duumbi/intents/history/`
- [x] Coordinator Agent v1 — CreateModule → AddFunction → ModifyMain sorrend
- [x] Verifier Agent v1 — temp `main.jsonld` generálás, fordítás, exit code check
- [x] End-to-end teszt: üres workspace → intent execute → multi-modul app fut

### Intelligens Retry (M5-RETRY)
- [x] 3-lépéses retry eszkalációval (error + nodeId → few-shot → egyszerűsített utasítás)
- [x] Strukturált hibavisszajelzés (error code + nodeId + fix hint)
- [x] Dinamikus few-shot példák (`src/examples.rs`, rule-based matching)
- [x] REPL graceful error handling (session folytatás hiba után)

## Intent YAML formátum

```yaml
intent: "Build a calculator"
version: 1
status: Pending
acceptance_criteria:
  - "add(a, b) returns a+b for i64"
modules:
  create: ["calculator/ops"]
  modify: ["app/main"]
test_cases:
  - name: basic_add
    function: add
    args: [3, 5]
    expected_return: 8
```

## GitHub

- Milestone: **Phase 5: Intent-Driven Development** (#6)
- Issues: #69–#90 (22/22 lezárva)

## Főbb fájlok

```
src/intent/
├── mod.rs        — module entry
├── spec.rs       — IntentSpec YAML struktúra
├── coordinator.rs — task bontás
├── verifier.rs   — teszteset futtatás
├── create.rs     — LLM → spec
├── review.rs     — spec megjelenítés
├── status.rs     — állapot lekérdezés
└── execute.rs    — teljes pipeline
src/examples.rs   — few-shot példák
```
