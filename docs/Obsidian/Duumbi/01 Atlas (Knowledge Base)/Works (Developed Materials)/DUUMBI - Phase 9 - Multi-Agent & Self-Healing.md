---
tags:
  - project/duumbi
  - milestone/phase-9
status: planned
github_milestone: ~
updated: 2026-03-13
---
# Phase 9 — Multi-Agent Orchestráció & Self-Healing ⏳

> **Kill Criterion:** A rendszer futásidejű hibánál (1) azonosítja a nodeId-t, (2) generál valid patch-javaslatot, (3) a patch után tesztek zöldek, (4) a diff emberileg review-olható.
> **Állapot:** ⏳ Tervezett — Phase 8 befejezése után indul

← Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

MCP szerver, specializált ágens swarm (Architect, Coder, Reviewer, Tester, Ops, Repair), OpenTelemetry telemetria, self-healing loop. A DUUMBI CLI-ből MCP szerver lesz.

## Tervezett feladatok

### MCP Szerver (M9-MCP)
- [ ] MCP szerver implementáció (`rmcp` crate)
- [ ] MCP toolok: `graph.query`, `graph.mutate`, `graph.validate`, `graph.describe`, `build.compile`, `build.run`, `telemetry.query`, `deps.search`, `deps.install`, `intent.create`, `intent.execute`
- [ ] Chat migráció — direkt API → MCP kliens (backward compatible)

### Ágens Swarm (M9-AGENT)
- [ ] Architect Agent — C4 struktúra tervezés intent spec-ből
- [ ] Coder Agent (meglévő `duumbi add` logika ágensként)
- [ ] Reviewer Agent — biztonsági és teljesítmény ellenőrzés
- [ ] Tester Agent — teszteset generálás + futtatás
- [ ] Ops Agent — futó alkalmazás megfigyelés
- [ ] Repair Agent — hibás gráf fragment javítás
- [ ] Párhuzamos végrehajtás — több Coder Agent + Graph Merge

### Self-Healing (M9-HEAL)
- [ ] Telemetria v2 — OpenTelemetry kompatibilis traceId → nodeId mapping
- [ ] Anomália detekció — panic, lassulás, hiba arány növekedés
- [ ] Back-mapping — futásidejű hiba → JSON-LD node azonosítás
- [ ] Repair loop — hibás fragment + hiba kontextus → patch → validate → test
- [ ] Studio self-healing panel — hibák, javaslatok, review

### Tanulás és Embedding (M9-LEARN)
- [ ] Séma-alapú kontextus injektálás (célzott gráf-részlet az LLM-nek)
- [ ] Sikeres műveletek naplózása (`.duumbi/learning/successes.jsonl`)
- [ ] Automatikus few-shot válogatás korábbi sikerekből
- [ ] GraphRAG integráció (petgraph traversal alapú kontextus-építés)

### Kutatás (M9-EMBED — PoC)
- [ ] Holografikus/komplex embedding PoC — JSON-LD gráf → HolE embedding
- [ ] Integrálás a Repair Agent kontextus-építésébe

## Monetizáció

Phase 9 unlock: **DUUMBI TEAM tier** ($49/hó/seat)
- Multi-ágens orchestráció
- Self-healing (telemetria → auto-patch javaslat)
- Team admin panel + audit log
- SLA: 99.9% registry uptime

## Függőségek

```
Phase 7 (Registry) ──→ Phase 9 (MCP, ágens swarm)
Phase 5 (Intent)   ──→ Phase 9 (intent.create/execute MCP toolok)
```

## Tervezett fájlok

```
src/mcp/         — MCP szerver implementáció (rmcp)
src/agents/      — specializált ágensek
src/telemetry/   — OpenTelemetry integráció (v2)
src/learning/    — successes.jsonl, few-shot matching
```
