---
tags:
  - project/duumbi
  - milestone/phase-3
status: complete
github_milestone: "Phase 3: Web Visualizer"
github_issues: "7/7 closed"
completed: 2026-03-01
---
# Phase 3 — Web Visualizer ✅

> **Kill Criterion:** 3/3 devs confirm faster comprehension than raw JSON-LD (timed comparison).
> **Eredmény:** ✅ Teljesítve (PR #40 merged)

← Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

Browser-alapú szemantikus gráf vizualizátor: Cytoscape.js + axum HTTP szerver + WebSocket live sync. A `duumbi viz` parancs elindítja a szervert (port 8420).

## Főbb deliverable-ök

- [x] `duumbi viz` CLI parancs — axum 0.8 szerver port 8420-on
- [x] Graph serializer — Cytoscape.js JSON formátum (`src/web/serialize.rs`)
- [x] File watcher — `notify-debouncer-mini` → mpsc → tokio async reload loop
- [x] WebSocket handler — `tokio::sync::broadcast` → ws stream per client
- [x] Frontend assets — Cytoscape.js (vendored) + HTML skeleton
- [x] Node részletek hover/kattintásra: `@type`, `@id`, connections, `traceId`

## GitHub

- Milestone: **Phase 3: Web Visualizer** (#4)
- Issues: #41–#47 (7/7 lezárva)
- PR: #40 (merged)

## Architektúra döntések

- Cytoscape.js + axum SSR (nem WASM + Canvas) — egyszerűbb, nincs WASM toolchain
- `AppState`: `Arc<RwLock<Value>>` graph + `broadcast::Sender` WS push-hoz
- File watcher: notify thread → mpsc channel → tokio async reload loop (blocking elkerülése)
- Port 8420 (nem 8080 — konfliktus elkerülése)

## Főbb fájlok

```
src/web/
├── mod.rs       — modul entry
├── serialize.rs — gráf → Cytoscape.js JSON
├── server.rs    — axum routes, AppState
├── watcher.rs   — file watch + debounce
├── ws.rs        — WebSocket handler
└── assets/      — vendored Cytoscape.js + HTML
```
