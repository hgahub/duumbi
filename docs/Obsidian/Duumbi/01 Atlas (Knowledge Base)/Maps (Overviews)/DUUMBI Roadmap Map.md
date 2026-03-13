---
tags:
  - project/duumbi
  - map/roadmap
status: active
created: 2026-03-12
updated: 2026-03-13
---
# DUUMBI — Fejlesztési Roadmap (Hub)

> Ez a fő összefoglaló. Minden milestone önálló jegyzethez vezet. A státusz a GitHub Issues alapján van naprakészen tartva.

---

## Állapot összefoglaló

| Phase | Cím | GitHub | Státusz |
|-------|-----|--------|---------|
| 0 | Proof of Concept | 9/9 issue ✓ | ✅ Kész |
| 1 | Usable CLI | 11/11 issue ✓ | ✅ Kész |
| 2 | AI Integration | 13/13 issue ✓ | ✅ Kész |
| 3 | Web Visualizer | 7/7 issue ✓ | ✅ Kész |
| 4 | Interactive CLI & Module System | 16/16 issue ✓ | ✅ Kész |
| 5 | Intent-Driven Development | 22/22 issue ✓ | ✅ Kész |
| 6 | DUUMBI Studio | 47/47 issue ✓ | ✅ Kész |
| 7 | Registry & Distribution | 35/37 issue ✓ | 🔄 Folyamatban (2 infra open) |
| 8 | Multi-Agent & Self-Healing | – | ⏳ Tervezett |

**Aktuális branch:** `phase7/registry-distribution`

---

## Milestone jegyzetek

### MVP (Phase 0–3)

- [[DUUMBI - Phase 0 - Proof of Concept]] — JSON-LD → Cranelift → natív bináris ✅
- [[DUUMBI - Phase 1 - Usable CLI]] — CLI parancsok, fibonacci, f64/bool ✅
- [[DUUMBI - Phase 2 - AI Integration]] — `duumbi add`, undo, 20/20 benchmark ✅
- [[DUUMBI - Phase 3 - Web Visualizer]] — Cytoscape.js + axum + WebSocket ✅

### Post-MVP (Phase 4–8)

- [[DUUMBI - Phase 4 - Interactive CLI & Module System]] — REPL, modulrendszer, stdlib ✅
- [[DUUMBI - Phase 5 - Intent-Driven Development]] — Intent spec, Coordinator, Verifier ✅
- [[DUUMBI - Phase 6 - DUUMBI Studio]] — Leptos SSR, C4 drill-down, chat UI ✅
- [[DUUMBI - Phase 7 - Registry & Distribution]] — publish, install, lockfile v1 🔄
- [[DUUMBI - Phase 8 - Multi-Agent & Self-Healing]] — MCP, ágens swarm, self-healing ⏳

---

## Kill Criterion összefoglaló

| Phase | Kill Criterion | Eredmény |
|-------|----------------|----------|
| 0 | `add(3,5)` → binary prints `8` | ✅ |
| 1 | External dev installs + runs in < 10 min | ✅ |
| 2 | > 70% correct on 20-command benchmark | ✅ 20/20 |
| 3 | 3/3 devs confirm faster than raw JSON-LD | ✅ |
| 4 | `abs(-7) = 7` via init → 2-module → binary | ✅ |
| 5 | `double(21)=42` via intent pipeline | ✅ |
| 6 | 3/3 devs gyorsabb navigáció web-en vs CLI | ✅ |
| 7 | Determinisztikus hash + publish+install + offline build | 🔄 |
| 8 | Hiba → nodeId → valid patch → tesztek zöldek | ⏳ |

---

## Üzleti mérföldkövek

| Időszak | Esemény | Modell |
|---------|---------|--------|
| Phase 0–6 | Közösségépítés | Donation (GitHub Sponsors) |
| Phase 7 | Privát registry | PRO tier ($19/hó) |
| Phase 7–8 | Csapat funkciók | TEAM tier ($49/hó/seat) |
| 18+ hónap | Enterprise igény | Enterprise (egyedi) |

→ Részletek: [[DUUMBI - Post-MVP Roadmap]]

---

## Kapcsolódó dokumentumok

- [[DUUMBI - PRD]] — Hosszú távú vízió
- [[DUUMBI - MVP Specification]] — Phase 0–3 specifikáció
- [[DUUMBI - Post-MVP Roadmap]] — Üzleti terv, monetizáció
- [[DUUMBI - Post-MVP Implementation Roadmap]] — Részletes impl. terv (M4–M8)
- [[DUUMBI - Graph Repository Architecture]] — M7 architektúra
- [[DUUMBI - Architecture Diagram]] — Technikai architektúra
- [[DUUMBI - Tools and Components]] — Tech stack
- [[DUUMBI - Glossary]] — Fogalomtár
