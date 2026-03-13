---
tags:
  - project/duumbi
  - milestone/phase-4
status: complete
github_milestone: "Phase 4: Interactive CLI & Module System"
github_issues: "16/16 closed"
completed: 2026-03-04
---
# Phase 4 — Interactive CLI & Module System ✅

> **Kill Criterion:** `duumbi init` → chat módban 2-modulos alkalmazás → `duumbi build && duumbi run` → `abs(-7) = 7`
> **Eredmény:** ✅ Teljesítve — 202 teszt zöld

← Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

A DUUMBI CLI interaktív fejlesztői eszközzé alakítva (REPL, `/` parancsok). Modulrendszer v1 implementálva lokális path-alapú dependency feloldással, FNV-1a lockfile-lal, stdlib modulokkal.

## Teljesített feladatok

### Interaktív REPL (M4-CHAT)
- [x] Chat REPL loop — `reedline 0.37` alapú input, history
- [x] Státusz sor — modell neve, workspace név
- [x] `/` parancsrendszer: `/build`, `/run`, `/check`, `/describe`, `/undo`, `/viz`, `/deps`, `/help`, `/exit`
- [x] Szabad szöveges input → AI mutáció (meglévő `orchestrator` logika)
- [x] Conversation history — kontextus megőrzés session-ön belül
- [x] Auto build+check minden AI mutáció után

### Modulrendszer (M4-MOD)
- [x] Namespace egységesítés: `https://duumbi.dev/ns/core#` mindenhol
- [x] `duumbi:imports` és `duumbi:exports` mezők `duumbi:Module`-on
- [x] Lokális modul import — path-alapú dependency feloldás
- [x] `duumbi deps list/add/remove` CLI subcommandok
- [x] Lockfile v0 — FNV-1a hash alapú, determinisztikus
- [x] Multi-modul fordítás: `Program::load()`, `compile_program()`, `link_multi()`
- [x] Stdlib: `stdlib/math.jsonld` (abs, max, min), `stdlib/io.jsonld` (print wrapperek)
- [x] Stdlib beágyazva a `duumbi init`-be

### Dokumentáció (M4-DOC)
- [x] `sites/docs/` mdBook scaffold (29 fájl) — docs.duumbi.dev
- [x] Namespace javítás, Tools and Components frissítés, Architecture Diagram fix

## GitHub

- Milestone: **Phase 4: Interactive CLI & Module System** (#5)
- Issues: #58–#62 + #51 + #50 (16/16 lezárva)
- Branch: `phase4/interactive-cli-module-system`

## Főbb fájlok

```
src/cli/
├── repl.rs      — Session struct, slash commands, auto-build
├── commands.rs  — shared build/check/describe/parse_and_validate
src/deps.rs      — FNV-1a lockfile, add/remove/list deps
src/manifest.rs  — ModuleManifest (TOML)
stdlib/
├── math.jsonld  — abs, max, min
└── io.jsonld    — print wrapperek
```
