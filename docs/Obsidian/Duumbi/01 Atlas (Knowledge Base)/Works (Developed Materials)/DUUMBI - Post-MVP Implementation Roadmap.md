---
tags:
  - project/duumbi
  - doc/strategy
  - doc/planning
status: active
created: 2026-02-28
updated: 2026-03-06
related_maps:
  - "[[DUUMBI - PRD]]"
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - Post-MVP Roadmap (Refined)]]"
  - "[[DUUMBI - Post-MVP Roadmap]]"
  - "[[DUUMBI - Task List]]"
  - "[[DUUMBI - Tools and Components]]"
  - "[[DUUMBI - Architecture Diagram]]"
  - "[[DUUMBI - Glossary]]"
  - "[[DUUMBI - Graph Repository Architecture]]"
---

# DUUMBI — Post-MVP Implementation Roadmap

> [!important] Ez a dokumentum az **autoritatív végrehajtási terv** a Phase 3 befejezése utáni fejlesztéshez. A [[DUUMBI - Post-MVP Roadmap (Refined)]] ajánlásait és a [[DUUMBI - PRD]] vízióját egyaránt figyelembe veszi, de **konkrét, megvalósítható feladatokra** bontja. Minden mérföldkőnek kill criterionja van.

---

## 0. Kiindulási állapot (2026-02-28)

### Amit a rendszer tud (Phase 0–3 befejezve)

| Képesség                                    | Állapot | Megjegyzés                        |
| ------------------------------------------- | ------- | --------------------------------- |
| JSON-LD → Cranelift → natív bináris         | ✅       | Phase 0 kill criterion teljesítve |
| CLI: init, build, run, check, describe      | ✅       | Phase 1 teljesítve                |
| AI mutáció: add, undo (Anthropic + OpenAI)  | ✅       | 20/20 benchmark (Phase 2)         |
| Web vizualizáció: viz (Cytoscape.js + axum) | ✅       | Phase 3 (PR #40)                  |
| Típusrendszer: i64, f64, bool, void         | ✅       |                                   |
| Op set: 14 op (Const → Return)              | ✅       |                                   |
|                                             |         |                                   |

### Amit NEM tud (és kell)

- Interaktív chat mód (CLI paraméterek nélkül)
- Modulrendszer (import/export, névtér, dependency)
- Intent-driven fejlesztés (spec → implementáció)
- Gazdag web felület (C4 navigáció, chat, ágenskezelés)
- Registry (megosztás, binary cache)
- Multi-ágens orchestráció + MCP szerver
- Self-healing (telemetria → auto-repair)

### Azonosított dokumentációs problémák

> [!warning] Ezek javítása a Milestone 4 része — lásd [[#M4-DOC: Dokumentáció szinkronizálás]].

| Dokumentum                                      | Probléma                                                           | Javítás                                                   |
| ----------------------------------------------- | ------------------------------------------------------------------ | --------------------------------------------------------- |
| [[DUUMBI - Post-MVP Roadmap]] (Phase 4.2 példa) | Namespace `duumbi.dev` ↔ `duumbi.dev` inkonzisztencia              | Egységesítés: `duumbi.dev` mindenhol                      |
| [[DUUMBI - Tools and Components]]               | axum `0.7.x` → valójában `0.8`, petgraph `0.6.x` → valójában `0.7` | Verziószámok frissítése                                   |
| [[DUUMBI - Tools and Components]]               | "WASM + Canvas" vizualizáció → valójában Cytoscape.js + axum SSR   | Leírás frissítése: "Cytoscape.js + axum HTTP szerver"     |
| [[DUUMBI - Tools and Components]]               | Hiányzó crate-ek: `notify-debouncer-mini`, `tower-http`, `toml`    | Kiegészítés                                               |
| [[DUUMBI - Architecture Diagram]]               | `petgraph::DiGraph` → valójában `StableGraph`                      | Javítás                                                   |
| [[DUUMBI - Glossary]]                           | Semantic Graph definíció: "DiGraph" → "StableGraph"                | Javítás                                                   |
| [[DUUMBI - Task List]]                          | Minden checkbox üres, holott Phase 0–2 kész, Phase 3 is kész       | Checkboxok frissítése, Phase 3 GitHub tracking hozzáadása |
| [[DUUMBI - MVP Specification]]                  | Key Technical Decisions: "WASM + Canvas (localhost)"               | Frissítés a tényleges implementációra                     |

---

## Stratégiai irány: "Érték előbb, varázslat később"

A [[DUUMBI - Post-MVP Roadmap (Refined)]] ajánlásával összhangban:

1. **Interaktivitás** — a CLI legyen chat-alapú, mint a Claude Code
2. **Újrafelhasználás** — modulok + import + lockfile
3. **Szándék kezelése** — intent spec + tesztek + traceability
4. **Vizualizáció** — gazdag web platform (Leptos/WASM), nem csak read-only gráf nézet
5. **Elosztás** — registry + binary cache
6. **Autonómia** — multi-ágens + self-healing

**Technológiai döntések (Post-MVP):**

| Döntés | Választás | Indoklás |
|--------|-----------|----------|
| Web frontend | **Leptos** (Rust → WASM) | Full-stack Rust, SSR + hydration, típusbiztos, egységes toolchain |
| Chat LLM kommunikáció | **Fázisos**: direkt API (M4) → MCP kliens (M8) | Gyors indulás, visszafelé kompatibilis |
| UI design irányvonal | **IcePanel-inspirált** | C4 drill-down, hierarchikus navigáció, fork-view-merge |
| Intent modell | **Augment Intent-inspirált**, DUUMBI-adaptált | Spec mint igazságforrás, koordinátor ágens, párhuzamos végrehajtás |
| Téma | Dark + Light (választható) | Induláshoz elég két téma |

---

## Milestone 4 — Interaktív CLI & Modulrendszer (8–10 hét)

> **Cél:** A DUUMBI CLI-t interaktív fejlesztői eszközzé alakítani (chat mód, `/` parancsok), és modulrendszert adni az újrafelhasználáshoz.
> **Kill Criterion:** Üres workspace-ből `duumbi init` + chat módban adott utasításokkal létrehozható egy 2-modulos alkalmazás, ami lefordul és fut. A lockfile determinisztikus.

### M4-CHAT: Interaktív chat mód

A `duumbi` paraméterek nélkül indítva **interaktív REPL** módba lép, a Claude Code mintájára.

**Felület elemei:**

```
┌─────────────────────────────────────────────────────────┐
│  duumbi v0.5.0 · claude-sonnet-4-6 · workspace: myapp  │
│  Context: 12.4k / 200k tokens                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  > Create a fibonacci function that works for i64       │
│                                                         │
│  ✓ Added function fibonacci(n: i64) -> i64              │
│  ✓ Schema validation passed                             │
│  ✓ Build successful                                     │
│                                                         │
│  > /describe                                            │
│  function fibonacci(n: i64) -> i64 {                    │
│    if n <= 1 { return n }                               │
│    return fibonacci(n-1) + fibonacci(n-2)               │
│  }                                                      │
│                                                         │
│  > /build                                               │
│  Built successfully: .duumbi/build/output               │
│                                                         │
│  >                                                      │
└─────────────────────────────────────────────────────────┘
```

**Deliverable-ök:**

- [x] **M4-CHAT-01**: Chat REPL loop — `rustyline` vagy `reedline` alapú input, history, autocomplete
- [x] **M4-CHAT-02**: Státusz sor — modell neve, context token használat, workspace név
- [x] **M4-CHAT-03**: `/` parancsrendszer — slash-prefixed parancsok dispatch:

| Parancs | Funkció | Megjegyzés |
|---------|---------|------------|
| `/build` | `duumbi build` futtatás | |
| `/run [args]` | `duumbi run` futtatás | |
| `/check` | `duumbi check` futtatás | |
| `/describe` | `duumbi describe` futtatás | |
| `/undo` | `duumbi undo` futtatás | |
| `/viz` | Vizualizáció indítása | |
| `/status` | Workspace állapot összefoglaló | |
| `/model [name]` | LLM modell váltás | Pl. `/model claude-sonnet-4-6` |
| `/config` | Config megjelenítés/szerkesztés | |
| `/help` | Parancs lista | |
| `/exit` | Kilépés | |
| `/intent` | Intent kezelés (Phase 5) | Előkészítve, M5-ben implementálva |
| `/deps` | Dependency kezelés | |

- [x] **M4-CHAT-04**: Szabad szöveges input → AI mutáció (a jelenlegi `duumbi add` logika)
- [x] **M4-CHAT-05**: Conversation history — kontextus megőrzés session-ön belül
- [x] **M4-CHAT-06**: Streaming válasz — token-by-token kiírás (Anthropic streaming API)
- [x] **M4-CHAT-07**: Auto build+check — minden AI mutáció után automatikus validáció és build

**Technikai megjegyzések:**
- A chat mód a meglévő `agents::orchestrator` logikára épít
- A direkt API hívás marad (config.toml-ból), MCP kliens majd M8-ban
- Context tracking: a teljes conversation JSON-t kell számolni (tiktoken vagy becsült token count)

### M4-MOD: Modulrendszer v1

**Deliverable-ök:**

- [x] **M4-MOD-01**: Namespace egységesítés — `https://duumbi.dev/ns/core#` mindenhol (kód + példák + séma + dokumentáció)
- [x] **M4-MOD-02**: `duumbi:Module` bővítés — `duumbi:imports` és `duumbi:exports` mezők
- [x] **M4-MOD-03**: Lokális modul import — path-alapú dependency feloldás
- [x] **M4-MOD-04**: Névtér kezelés — modulhatárokon egyedi névterek (`duumbi:math/add`)
- [x] **M4-MOD-05**: `duumbi deps install` CLI parancs — lokális forrásokból
- [x] **M4-MOD-06**: Lockfile v0 — `.duumbi/deps.lock` generálás és ellenőrzés
- [x] **M4-MOD-07**: Multi-modul fordítás — több `.jsonld` fájl → egyetlen bináris, cross-module function call resolution
- [x] **M4-MOD-08**: 2–3 stdlib modul — `stdlib/math` (abs, max, min), `stdlib/io` (print wrapperek)

**JSON-LD séma példa (modul import):**

```json
{
  "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
  "@type": "duumbi:Module",
  "@id": "duumbi:app/main",
  "duumbi:name": "main",
  "duumbi:imports": [
    {
      "duumbi:module": "stdlib/math",
      "duumbi:path": "../stdlib/math.jsonld",
      "duumbi:functions": ["abs", "max"]
    }
  ],
  "duumbi:functions": [ ... ]
}
```

### M4-DOC: Dokumentáció szinkronizálás

- [x] **M4-DOC-01**: Namespace javítás — minden Obsidian dokumentumban `duumbi.dev`
- [x] **M4-DOC-02**: Tools and Components frissítés — verziószámok, crate-ek, vizualizáció leírás
- [x] **M4-DOC-03**: Architecture Diagram — StableGraph, Phase 3 tényleges implementáció
- [x] **M4-DOC-04**: Glossary — StableGraph, új fogalmak (Intent, Chat Mode, Module)
- [x] **M4-DOC-05**: Task List — checkboxok frissítése (Phase 0–3 kész), Post-MVP task-ok hozzáadása
- [x] **M4-DOC-06**: MVP Specification — tényleges implementáció tükrözése a Technical Decisions táblában

### M4 összefoglaló

| Szál | Feladat szám | Becsült effort |
|------|-------------|----------------|
| Chat mód | 7 feladat | 3–4 hét |
| Modulrendszer | 8 feladat | 4–5 hét |
| Dokumentáció | 6 feladat | 1 hét |
| **Összesen** | **21 feladat** | **8–10 hét** |

---

## Milestone 5 — Intent-Driven Development (6–8 hét)

> **Cél:** A fejlesztő specifikáljon, ne prompt-tologasson. Az Intent rendszer strukturált spec-ből tesztelt, reprodukálható eredményt ad.
> **Kill Criterion:** Egy intent spec létrehoz egy többmodulos alkalmazást (min. 2 modul, min. 3 függvény), minden teszteset zöld, és az eredmény `duumbi build && duumbi run`-nal fut.

### Az Intent modell (Augment Code-inspirált, DUUMBI-adaptált)

Az [Augment Intent](https://docs.augmentcode.com/intent/overview) kulcs koncepciói adaptálva:

```
Felhasználói szándék (természetes nyelv vagy chat)
    │
    ▼
┌──────────────────────────────────────┐
│  Intent Spec (.duumbi/intents/*.yaml)│ ← Emberi review pont
│  - Cél leírás                        │
│  - Elfogadási kritériumok            │
│  - Érintett modulok                  │
│  - Tesztesetek (input → expected)    │
│  - Függőségek (más intent-ek)        │
└──────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────┐
│  Coordinator Agent                   │ ← Gráf analízis, feladatbontás
│  - Meglévő gráf elemzés             │
│  - Terv generálás (task lista)       │
│  - Modul határok meghatározás        │
└──────────────────────────────────────┘
    │
    ├────────────┬────────────┐
    ▼            ▼            ▼
┌─────────┐ ┌─────────┐ ┌──────────┐
│ Coder   │ │ Coder   │ │ Verifier │ ← Szekvenciális végrehajtás (M5)
│ Agent   │ │ Agent   │ │ Agent    │   Párhuzamos végrehajtás (M8)
└─────────┘ └─────────┘ └──────────┘
    │            │            │
    ▼            ▼            ▼
┌──────────────────────────────────────┐
│  Graph Merge + Schema Validation     │ ← Minden lépés után check
│  + Test Execution                    │
└──────────────────────────────────────┘
    │
    ▼
  Semantic Fixed Point ✓
```

**Fontos különbség az Augment Intent-hez képest:**
- Augment szöveges kódot generál → DUUMBI szemantikus gráfot
- Augment git branch-ekben dolgozik → DUUMBI snapshot history-val (undo stack)
- Augment-nél a spec szabad formájú → DUUMBI-ban YAML + tesztesetek (determinisztikus)

### Deliverable-ök

- [x] **M5-INT-01**: Intent spec formátum stabilizálás (YAML)

```yaml
# .duumbi/intents/calculator.yaml
intent: "Készíts egy egyszerű kalkulátort 4 alapművelettel"
version: 1
acceptance_criteria:
  - "4 függvény: add, sub, mul, div (mind i64 → i64)"
  - "Mindegyik két paramétert fogad"
  - "Div nullával való osztásnál 0-t ad vissza"
  - "A main függvény demonstrálja mind a 4 műveletet"
modules:
  create:
    - "calculator/ops"
  modify:
    - "app/main"
test_cases:
  - name: "addition"
    function: "add"
    args: [3, 5]
    expected_return: 8
  - name: "division_by_zero"
    function: "div"
    args: [10, 0]
    expected_return: 0
dependencies: []
```

- [x] **M5-INT-02**: `duumbi intent create "<szándék>"` — interaktív spec generálás (LLM segítségével)
- [x] **M5-INT-03**: `duumbi intent review` — spec megjelenítés + szerkesztési lehetőség
- [x] **M5-INT-04**: `duumbi intent execute` — végrehajtás:
  1. Coordinator elemzi a gráfot + spec-et
  2. Task listára bontja (determinisztikus sorrend)
  3. Minden task: AI mutáció → check → build
  4. Tesztesetek futtatása
  5. Diff preview + emberi jóváhagyás
- [x] **M5-INT-05**: `duumbi intent status` — végrehajtás állapota (ha több lépéses)
- [x] **M5-INT-06**: `/intent` parancs a chat módban (delegálás a CLI parancsokhoz)
- [x] **M5-INT-07**: Intent history — `.duumbi/intents/history/` archivált, végrehajtott spec-ek
- [x] **M5-INT-08**: Coordinator Agent v1 — task bontás, modul boundary felismerés
- [x] **M5-INT-09**: Verifier Agent v1 — teszteset generálás, elfogadási kritérium ellenőrzés
- [x] **M5-INT-10**: End-to-end teszt: üres workspace → `duumbi init` → intent execute → multi-modul app fut

**A "chat + intent" végcél demonstrálása:**

```
$ duumbi init myapp
$ cd myapp
$ duumbi

duumbi v0.6.0 · claude-sonnet-4-6 · workspace: myapp

> Készíts egy kalkulátor alkalmazást ami 4 alapműveletet támogat,
  és a main függvény demonstrálja mindegyiket

📋 Intent spec generálva: .duumbi/intents/calculator.yaml
   4 elfogadási kritérium, 2 modul, 4 teszteset

Jóváhagyod? [Y/n] y

🔄 Végrehajtás...
  ✓ [1/3] calculator/ops modul létrehozva (add, sub, mul, div)
  ✓ [2/3] app/main módosítva (demonstráció)
  ✓ [3/3] Tesztek: 4/4 zöld

✅ Intent teljesítve. Build sikeres.

> /run
12    (3+5+4)
...
```

### M5 összefoglaló

| Szál | Feladat szám | Becsült effort |
|------|-------------|----------------|
| Intent rendszer | 10 feladat | 5–7 hét |
| Coordinator + Verifier agent | 2 feladat | 2–3 hét (átfedés) |
| **Összesen** | **10 feladat** | **6–8 hét** |

---

## Milestone 6 — Web Platform: DUUMBI Studio (10–12 hét)

> **Cél:** IcePanel-inspirált, Leptos/WASM-alapú gazdag web felület, ami a CLI minden képességét grafikus formában biztosítja. Nem csupán vizualizáció — ez a fejlesztői cockpit.
> **Kill Criterion:** 3/3 fejlesztő megerősíti, hogy a web felületen gyorsabban navigálnak és értenek meg egy 3+ modulos alkalmazást, mint CLI + JSON-LD olvasással. A chat és intent funkciók a web-en és CLI-n azonos eredményt adnak.

### UI architektúra (IcePanel-inspirált)

```
┌──────────────────────────────────────────────────────────────────┐
│  DUUMBI Studio                                    🌙 Dark │ 🔍  │
├──────────┬───────────────────────────────────────────────────────┤
│          │                                                       │
│ 📁 Explorer  │  ┌─── C4 Navigator ────────────────────────┐     │
│          │  │                                              │     │
│ ▶ app/   │  │   ┌──────────┐    ┌──────────┐             │     │
│   main   │  │   │  app/    │───→│ stdlib/  │             │     │
│ ▶ stdlib/│  │   │  main    │    │  math    │             │     │
│   math   │  │   └──────────┘    └──────────┘             │     │
│   io     │  │                                              │     │
│          │  │   [Context] → [Container] → [Component] → [Code] │
│ 🤖 Agents│  │    ▲ kattintással drill-down                │     │
│          │  └──────────────────────────────────────────────┘     │
│ 📋 Intents│                                                      │
│          │  ┌─── Detail Panel ─────────────────────────────┐    │
│ ⚙️ Config │  │  @type: duumbi:Add                           │    │
│          │  │  @id: duumbi:app/main/entry/2                │    │
│          │  │  left: duumbi:app/main/entry/0 (Const 3)     │    │
│          │  │  right: duumbi:app/main/entry/1 (Const 5)    │    │
│          │  │  resultType: i64                              │    │
│          │  │  traceId: abc-123-...                         │    │
│          │  └──────────────────────────────────────────────┘    │
│          │                                                       │
├──────────┴───────────────────────────────────────────────────────┤
│  💬 Chat                                                         │
│  > Add a multiply function to the math module                    │
│  ✓ Added function mul(a: i64, b: i64) -> i64                    │
│  >                                                               │
└──────────────────────────────────────────────────────────────────┘
```

### Nézetek és navigáció

**C4-szintű drill-down** (IcePanel mintájára):

| Szint | Mit látunk | Kattintásra |
|-------|-----------|-------------|
| **Context** | Workspace áttekintés — modulok és külső függőségek | → Container |
| **Container** | Egy modul belső szerkezete — függvények és kapcsolataik | → Component |
| **Component** | Egy függvény blokkjai és adatfolyama | → Code |
| **Code** | Egyetlen blokk op-szintű részletei (a jelenlegi `viz` nézet) | Detail panel |

**Navigációs minták:**
- Breadcrumb: `workspace > app/main > fibonacci > entry`
- Keresés: `Ctrl+K` → gráf node keresés @id, @type, függvénynév alapján
- Minimap: kis áttekintő az aktuális nézet pozíciójáról

### Főbb felületek

#### 6.1 Graph Explorer (C4 Navigator)
- [ ] **M6-GFX-01**: Leptos projekt scaffolding — `crates/duumbi-studio` (WASM target)
- [ ] **M6-GFX-02**: Axum szerver integráció — SSR + WASM hydration, API endpoints
- [ ] **M6-GFX-03**: C4 Context nézet — modulok mint node-ok, dependency-k mint élek
- [ ] **M6-GFX-04**: C4 Container nézet — egy modul függvényei, belső adatfolyam
- [ ] **M6-GFX-05**: C4 Component nézet — egy függvény blokkjai, branch-ek
- [ ] **M6-GFX-06**: C4 Code nézet — op-szintű részletek (meglévő Cytoscape logika Leptos-ra portolva)
- [ ] **M6-GFX-07**: Drill-down animáció — smooth zoom a szintek között
- [ ] **M6-GFX-08**: Detail panel — node metaadatok kattintásra (type, id, connections, traceId)
- [ ] **M6-GFX-09**: WebSocket live sync — gráf változások real-time frissítés (meglévő WS logika)

#### 6.2 Chat felület
- [ ] **M6-CHAT-01**: Chat panel (alsó sáv) — azonos conversation, mint a CLI chat
- [ ] **M6-CHAT-02**: API bridge — a web chat és CLI chat ugyanazt a backend-et használja
- [ ] **M6-CHAT-03**: Streaming válaszok — token-by-token megjelenítés
- [ ] **M6-CHAT-04**: Gráf highlight — chat által érintett node-ok kiemelése a gráf nézetben
- [ ] **M6-CHAT-05**: Slash parancsok — `/build`, `/run`, `/describe` stb. a chat input-ból

#### 6.3 Sidebar nézetek
- [ ] **M6-SIDE-01**: File Explorer — workspace fájlstruktúra fa nézet
- [ ] **M6-SIDE-02**: Agent panel — aktív ágensek, állapotuk, utolsó akcióik
- [ ] **M6-SIDE-03**: Intent panel — aktív és archivált intent-ek listája, állapotjelző
- [ ] **M6-SIDE-04**: Config panel — workspace beállítások szerkesztése

#### 6.4 Keresés
- [ ] **M6-SRCH-01**: `Ctrl+K` gyors keresés — node-ok @id, @type, név alapján
- [ ] **M6-SRCH-02**: Szűrők — modul, függvény, op típus, típus

#### 6.5 Téma és UX
- [ ] **M6-UX-01**: Dark téma (alapértelmezett)
- [ ] **M6-UX-02**: Light téma
- [ ] **M6-UX-03**: Témaváltó (`Ctrl+Shift+T` vagy UI toggle)
- [ ] **M6-UX-04**: Responsive layout — sidebar összecsukható, chat panel átméretezhető
- [ ] **M6-UX-05**: Keyboard shortcuts — navigáció, keresés, build, run

#### 6.6 Infrastruktúra
- [ ] **M6-INFRA-01**: `duumbi studio` CLI parancs (a `viz` utódja, backward compatible alias)
- [ ] **M6-INFRA-02**: API réteg — REST endpoints a gráf CRUD-hoz, build/run triggerekhez
- [ ] **M6-INFRA-03**: Shared state — web és CLI chat ugyanazt a conversation state-et látja
- [ ] **M6-INFRA-04**: Asset pipeline — Leptos WASM build + trunk/cargo-leptos integráció

### Leptos technikai döntések

| Kérdés | Döntés | Indoklás |
|--------|--------|----------|
| Gráf renderelés | `<canvas>` + custom Rust rajzolás | Teljesítmény, nagy gráfok |
| Layout algoritmus | Dagre (portolva WASM-ba) vagy `d3-force` interop | Hierarchikus gráfokhoz jó |
| Szerver mód | axum + Leptos SSR | Meglévő axum infrastruktúra |
| State management | Leptos signals + WebSocket | Reaktív, real-time |
| Build tool | `cargo-leptos` | Hivatalos Leptos tooling |

### M6 összefoglaló

| Szál | Feladat szám | Becsült effort |
|------|-------------|----------------|
| Graph Explorer (C4) | 9 feladat | 4–5 hét |
| Chat felület | 5 feladat | 2 hét |
| Sidebar nézetek | 4 feladat | 2 hét |
| Keresés | 2 feladat | 1 hét |
| Téma + UX | 5 feladat | 1–2 hét |
| Infrastruktúra | 4 feladat | 2 hét |
| **Összesen** | **29 feladat** | **10–12 hét** |

---

## Milestone 7 — Registry & Distribution (10–12 hét)

> **Cél:** Háromrétegű gráf-tároló architektúra (workspace/vendor/cache), scope-alapú névtér rendszer, és "crates.io a szemantikus gráfoknak" — `registry.duumbi.dev`.
> **Kill Criterion:** Ugyanaz a modul két gépen (azonos target) ugyanazt a semantic hash-t adja. `duumbi publish` + `duumbi deps install` működik egy publikus modulra. `duumbi deps vendor --all` + `duumbi build --offline` működik. Lockfile v1 determinisztikus.

> [!info] Részletes architektúra: [[DUUMBI - Graph Repository Architecture]]

### M7-STORE: Háromrétegű Gráf Tároló

- [ ] **M7-STORE-01**: Workspace layer — `.duumbi/graph/` megtartása saját moduloknak (változatlan)
- [ ] **M7-STORE-02**: Vendor layer — `.duumbi/vendor/` implementálás (VCS-be commitolható, auditált)
- [ ] **M7-STORE-03**: Cache layer — `.duumbi/cache/` implementálás (registry letöltések, `.gitignore`-ban)
- [ ] **M7-STORE-04**: Resolution Pipeline — Workspace > Vendor > Cache > Registry feloldási sorrend
- [ ] **M7-STORE-05**: `manifest.toml` formátum — modul metaadatok (name, version, exports, license, keywords)
- [ ] **M7-STORE-06**: Migráció — `duumbi upgrade`: `.duumbi/stdlib/` → `.duumbi/cache/@duumbi/`

### M7-NS: Scope-alapú Névtér Rendszer

- [ ] **M7-NS-01**: `@<scope>/<module>` konvenció — `@duumbi/`, `@<user>/`, `@<org>/`
- [ ] **M7-NS-02**: JSON-LD `@context` mapping — scope → namespace URI
- [ ] **M7-NS-03**: Ütközéskezelés — scope-based prioritás + E012 ModuleConflict error
- [ ] **M7-NS-04**: config.toml v2 — `[dependencies]` scope szintaxis, `[registries]` + `[vendor]` szekciók

### M7-REG: Registry Backend és CLI

- [ ] **M7-REG-01**: Registry szerver (axum) — `registry.duumbi.dev` API
- [ ] **M7-REG-02**: `duumbi publish` — modul feltöltés (JSON-LD + manifest.toml + SemVer)
- [ ] **M7-REG-03**: `duumbi deps install` — registry letöltés cache-be + lockfile update
- [ ] **M7-REG-04**: `duumbi deps add <module>` — dependency hozzáadása config.toml-hoz
- [ ] **M7-REG-05**: `duumbi deps update` — lock file frissítése kompatibilis verziókra
- [ ] **M7-REG-06**: `duumbi search "<query>"` — szöveges + típus-alapú keresés (Level 1–2)
- [ ] **M7-REG-07**: `duumbi deps vendor` — szelektív vagy teljes vendoring
- [ ] **M7-REG-08**: `duumbi build --offline` — csak vendor + cache rétegből build
- [ ] **M7-REG-09**: `duumbi deps tree` — dependency fa megjelenítés
- [ ] **M7-REG-10**: `duumbi deps audit` — integrity hash ellenőrzés

### M7-HASH: Szemantikus Hash és Binary Cache

- [ ] **M7-HASH-01**: Semantic hash — gráf struktúra + op értékek ujjlenyomata (`@id` nélkül)
- [ ] **M7-HASH-02**: Binary cache — content-addressable `.o` fájlok CDN-ről
- [ ] **M7-HASH-03**: `duumbi yank <version>` — hibás verziók visszavonása

### M7-LOCK: Lockfile v1

- [ ] **M7-LOCK-01**: Lockfile v1 formátum — source URL + semantic hash + integrity (SHA-256) + vendored flag
- [ ] **M7-LOCK-02**: `duumbi deps install --frozen` — lockfile változás tiltása (CI-hoz)

### M7-PRIV: Privát Registry Támogatás

- [ ] **M7-PRIV-01**: `duumbi registry add <name> <url>` — többszörös registry konfiguráció
- [ ] **M7-PRIV-02**: `duumbi registry login <name>` — token-based autentikáció
- [ ] **M7-PRIV-03**: Scope-level hozzáférés-kezelés — `@org/` csak az org registry-ből
- [ ] **M7-PRIV-04**: `duumbi publish --registry <name>` — célzott publikálás

### M7-WEB: Registry Web Frontend

- [ ] **M7-WEB-01**: Registry web frontend (Leptos) — böngészés, keresés, modul docs
- [ ] **M7-WEB-02**: Studio integráció — registry keresés a sidebar-ból

### Hosting

Az [[DUUMBI - Post-MVP Roadmap#4.2 Hosting Platform Összehasonlítás]] ajánlásával összhangban: **Azure-központú** (Opció A) induláshoz, bootstrap fázisban **Cloudflare+Fly.io** (Opció B) is opció.

### M7 összefoglaló

| Szál | Feladat szám | Becsült effort |
|------|-------------|----------------|
| Háromrétegű tároló | 6 feladat | 2–3 hét |
| Névtér rendszer | 4 feladat | 1–2 hét |
| Registry backend + CLI | 10 feladat | 4–5 hét |
| Semantic hash + binary cache | 3 feladat | 2 hét |
| Lockfile v1 | 2 feladat | 1 hét |
| Privát registry | 4 feladat | 2 hét |
| Web frontend | 2 feladat | 2 hét |
| **Összesen** | **31 feladat** | **10–12 hét** |

---

## Milestone 8 — Multi-Agent Orchestráció & Self-Healing (8–10 hét)

> **Cél:** MCP szerver, specializált ágens swarm, telemetria visszacsatolás és automatikus javítási javaslat.
> **Kill Criterion:** A rendszer egy futásidejű hibánál (1) azonosítja a nodeId-t, (2) generál valid patch-javaslatot, (3) a patch után a tesztek zöldek, (4) a diff emberileg review-olható.

### M8-MCP: MCP Szerver

- [ ] **M8-MCP-01**: MCP szerver implementáció (`rmcp` crate) — a DUUMBI CLI-ből MCP szerver lesz
- [ ] **M8-MCP-02**: MCP toolok:

| Tool | Leírás |
|------|--------|
| `graph.query` | Gráf lekérdezés (node keresés, szomszédok, útvonalak) |
| `graph.mutate` | Gráf módosítás (patch alkalmazás, validációval) |
| `graph.validate` | Schema validáció futtatás |
| `graph.describe` | Gráf pszeudokód generálás |
| `build.compile` | Fordítás indítás |
| `build.run` | Futtatás indítás |
| `telemetry.query` | Telemetriai adatok lekérdezés |
| `deps.search` | Registry keresés |
| `deps.install` | Dependency telepítés |
| `intent.create` | Intent spec generálás |
| `intent.execute` | Intent végrehajtás |

- [ ] **M8-MCP-03**: Chat migráció — a direkt API hívás helyett MCP kliens + LLM (backward compatible config)

### M8-AGENT: Ágens Swarm

- [ ] **M8-AGENT-01**: Architect Agent — C4 struktúra tervezés, modul határok meghatározás intent spec-ből
- [ ] **M8-AGENT-02**: Coder Agent (a meglévő `duumbi add` logika ágensként) — funkció implementáció
- [ ] **M8-AGENT-03**: Reviewer Agent — biztonsági és teljesítmény ellenőrzés patch-eken
- [ ] **M8-AGENT-04**: Tester Agent — teszteset generálás, futtatás, regresszió detekció
- [ ] **M8-AGENT-05**: Ops Agent — futó alkalmazás megfigyelés, telemetria elemzés
- [ ] **M8-AGENT-06**: Repair Agent — hibás gráf fragment javítás, corrective patch generálás
- [ ] **M8-AGENT-07**: Párhuzamos végrehajtás — több Coder Agent egyszerre, Graph Merge

### M8-HEAL: Self-Healing

- [ ] **M8-HEAL-01**: Telemetria v2 — traceId → nodeId ipari szintű mapping (OpenTelemetry kompatibilis)
- [ ] **M8-HEAL-02**: Anomália detekció — panic, lassulás, hiba arány növekedés
- [ ] **M8-HEAL-03**: Back-mapping — futásidejű hiba → pontos JSON-LD node azonosítás
- [ ] **M8-HEAL-04**: Repair loop — Repair Agent kap: hibás fragment + hiba kontextus → patch → validate → test
- [ ] **M8-HEAL-05**: Studio integráció — self-healing panel: hibák, javaslatok, review, jóváhagyás

### M8 összefoglaló

| Szál | Feladat szám | Becsült effort |
|------|-------------|----------------|
| MCP szerver | 3 feladat | 3 hét |
| Ágens swarm | 7 feladat | 4–5 hét |
| Self-healing | 5 feladat | 3–4 hét |
| **Összesen** | **15 feladat** | **8–10 hét** |

---

## Összesítés és ütemterv

```
Phase 3 befejezés (2026 március)
    │
    ├── Milestone 4: Interaktív CLI & Modulrendszer    (8–10 hét)
    │   ├── Chat REPL + / parancsok
    │   ├── Modulrendszer v1 (import/export, lockfile)
    │   └── Dokumentáció szinkronizálás
    │   Kill: 2-modulos app chat-ből, determinisztikus lockfile
    │
    ├── Milestone 5: Intent-Driven Development          (6–8 hét)
    │   ├── Intent spec formátum (YAML)
    │   ├── Coordinator + Verifier agent
    │   └── End-to-end: üres workspace → multi-modul app
    │   Kill: intent → multi-modul app, tesztek zöldek
    │
    ├── Milestone 6: DUUMBI Studio (Web Platform)       (10–12 hét)
    │   ├── Leptos/WASM frontend
    │   ├── C4 drill-down navigáció (IcePanel-stílus)
    │   ├── Chat + Intent felület (azonos, mint CLI)
    │   ├── Ágens és workspace kezelés
    │   └── Dark/Light téma
    │   Kill: 3/3 dev gyorsabb navigáció + chat web = CLI
    │
    ├── Milestone 7: Registry & Distribution             (10–12 hét)
    │   ├── Háromrétegű tároló (workspace/vendor/cache)
    │   ├── Scope-alapú névtér (@scope/module)
    │   ├── publish/install/search/vendor/offline
    │   ├── Semantic hash + binary cache
    │   ├── Privát registry támogatás
    │   └── Registry web frontend
    │   Kill: determinisztikus hash + publish+install + offline build működik
    │
    └── Milestone 8: Multi-Agent & Self-Healing          (8–10 hét)
        ├── MCP szerver (rmcp)
        ├── 6 specializált ágens
        ├── Telemetria → auto-repair
        └── Studio self-healing panel
        Kill: hiba → nodeId → valid patch → tesztek zöldek

Összesen: ~42–54 hét (10–13 hónap)
```

### Függőségek

```
M4 ──→ M5 ──→ M6
 │             ↑
 └──→ M7 ─────┘
       │
       └──→ M8
```

- **M5 függ M4-től**: intent-nek kell a modulrendszer és a chat mód
- **M6 függ M5-től**: a Studio-nak kell az intent panel
- **M6 függ M7-től** (lazán): registry keresés a sidebar-ban
- **M7 függ M4-től**: publish-hoz kell a modulrendszer
- **M8 függ M7-től**: MCP toolok registry-t is kezelnek

**Javasolt végrehajtási sorrend:** M4 → M5 → (M6 ∥ M7) → M8

M6 és M7 párhuzamosítható, ha van elég kapacitás.

---

## Monetizációs pontok (referencia)

A [[DUUMBI - Post-MVP Roadmap]] üzleti tervével összhangban:

| Milestone | Monetizáció | Megjegyzés |
|-----------|-------------|------------|
| M4–M5 | ❌ Ingyenes | Közösségépítés, fejlesztői élmény |
| M6 | ❌ Ingyenes (core) | Studio alapfunkciók ingyenesek |
| M7 | 💰 **PRO indul** | Privát modulok, binary cache CDN |
| M8 | 💰 **TEAM indul** | Multi-ágens, self-healing, SLA |

---

## Kapcsolódó dokumentumok

- [[DUUMBI - PRD]] — Hosszú távú vízió
- [[DUUMBI - MVP Specification]] — MVP specifikáció (Phase 0–3)
- [[DUUMBI - Post-MVP Roadmap]] — Üzleti terv és monetizáció
- [[DUUMBI - Post-MVP Roadmap (Refined)]] — Audit és stratégiai ajánlás
- [[DUUMBI - Task List]] — MVP feladatlista
- [[DUUMBI - Tools and Components]] — Technikai stack
- [[DUUMBI - Architecture Diagram]] — Architektúra
- [[DUUMBI - Glossary]] — Fogalomtár
- [[DUUMBI - Graph Repository Architecture]] — Gráf tároló és névtér architektúra


---

## Webes jelenlét: duumbi.dev & docs.duumbi.dev

### Mikor hozzuk létre?

| Site | Mikor | Miért |
|------|-------|-------|
| `docs.duumbi.dev` | **M4 közben (most)** | A CLI stabilizálódik — dokumentálni kell menet közben |
| `duumbi.dev` | **M5–M6 között** | Akkor lesz mit megmutatni (Intent + Studio demo) |

### Repo elhelyezés: monorepo

A dokumentáció és a kód **ugyanabban a repóban** él (`hgahub/duumbi`). Egy PR egyszerre tartalmaz kódváltozást + doc frissítést. Kis csapatnál ez a praktikus megközelítés.

### Folder struktúra

```
duumbi/
├── src/              # Rust compiler/CLI
├── crates/           # Jövőbeli separate crate-ek (pl. duumbi-studio Leptos)
├── tests/
├── docs/             # Fejlesztői belső docs (CLAUDE.md, architecture.md) — meglévő
└── sites/
    ├── landing/      # duumbi.dev — marketing oldal (Astro, statikus)
    └── docs/         # docs.duumbi.dev — technikai dokumentáció (mdBook)
        ├── book.toml
        └── src/
```

> `sites/` azért jobb mint `web/`, mert a `src/web/` már foglalt (Phase 3 axum server).

### Tech stack

| Site | Tech | Miért |
|------|------|-------|
| `sites/docs/` | **mdBook** | Rust ökoszisztéma sztenderd, Markdown, egyszerű, CI-barát |
| `sites/landing/` | **Astro** (statikus) | Gyors, szép, nincs JS runtime, könnyű deploy |
| Studio (M6) | **Leptos** | Már eldöntött, `crates/duumbi-studio/` lesz |

### Deliverable-ök (M4-WEB)

**M4 — docs.duumbi.dev váz:**
- [x] **M4-WEB-01**: `sites/docs/` mdBook váz (book.toml, SUMMARY.md, kezdőoldal)
- [x] **M4-WEB-02**: CLI referencia (init, build, run, check, describe, add, undo, viz)
- [x] **M4-WEB-03**: JSON-LD formátum referencia (Op-ok, namespace, nodeId)
- [x] **M4-WEB-04**: Modulrendszer docs (M4 befejezése után)
- [x] **M4-WEB-05**: Deploy pipeline — GitHub Pages / Cloudflare Pages + CI

**M5–M6 — duumbi.dev landing:**
- [ ] **M5-WEB-01**: `sites/landing/` Astro scaffold
- [ ] **M5-WEB-02**: Hero szekció (tagline, CLI demo, install parancs)
- [ ] **M6-WEB-01**: Studio live demo embed
- [ ] **M6-WEB-02**: Saját domain + TLS deploy


---

## Alapelv: "Never Crash, Always Learn"

> [!important] Ez az alapelv a teljes roadmapra érvényes — minden AI-interakciós ponton (add, REPL, Studio, Agent swarm) alkalmazandó.

**Gyökérok-elemzés (2026-03-04):** A git commitok elemzése (13114faa..987ab18, 8 commit) feltárta, hogy a Sonnet 4.6 modell rendszeresen hibázik a JSON-LD szerkesztésnél:

| Hiba típus | Gyakoriság | Eddigi megoldás |
|---|---|---|
| Üres `duumbi:blocks: []` | Gyakori | `check_function_structure` validátor + SYSTEM_PROMPT bővítés |
| Hiányzó block terminátor (Return/Branch) | Gyakori | `check_terminator_position` validátor |
| Return/Branch utáni műveletek | Közepes | PATTERN A/B dokumentáció a promptban |
| Hibás insert sorrend (remove+add) | Gyakori | `replace_block` atomi eszköz bevezetése |

**Jelenlegi állapot:** Max 1 retry (hardcoded), nyers error string visszajelzés, program kilép hiba esetén.

**4 alapelv:**

1. **Hiba ≠ Leállás:** A program soha ne lépjen ki hiba esetén, ha van lehetőség továbblépésre
2. **Minden hiba tanulási lehetőség:** Naplózás + kontextus gazdagítás
3. **Eszkaláció:** retry → gazdagabb prompt → egyszerűsített utasítás → hibajelzés a felhasználónak
4. **Transzparencia:** A felhasználó mindig lássa mit csinál a rendszer, hányadik próbálkozásnál tart

---

## M5-RETRY: Intelligens Retry Pipeline (M5 kiegészítés)

> [!note] Ez a szekció az [[#Milestone 5 — Intent-Driven Development (6–8 hét)]] kiegészítése. A `duumbi add` ne egyszer próbálkozzon, hanem iteratív ciklusban, egyre gazdagabb kontextussal.

### M5-RETRY-01: Többlépcsős retry (max 3 próbálkozás, eszkalációval)

```
Retry 1: error message + hibás node @id + error code (E001-E009)
Retry 2: + releváns few-shot példa (hasonló sikeres művelet)
Retry 3: + egyszerűsített utasítás ("használd replace_block-ot")
Ha 3 retry sem sikerül → hibajelzés, session folytatódik
```

### M5-RETRY-02: Strukturált hibavisszajelzés az LLM-nek

Jelenlegi nyers string helyett:

```
"Previous attempt failed. Errors:
- E003 MISSING_FIELD at duumbi:main/factorial/recurse/3: field 'duumbi:right' required
- E001 TYPE_MISMATCH at duumbi:main/factorial/entry/1: Add expects i64, got f64
Fix hints:
- For E003: ensure all operand fields are present
- Consider using replace_block for atomic block rewrite"
```

### M5-RETRY-03: Dinamikus few-shot példák

- 10-15 gyakori sikeres patch példa (beágyazott `include_str!`)
- Kulcsszó-alapú matching az error kód + felhasználói kérés alapján
- Nem kell vektor DB, egyszerű rule-based kiválasztás

### M5-RETRY-04: REPL viselkedés hiba esetén

- Sikertelen mutáció → hibajelzés a felhasználónak (mit próbált, miért nem sikerült)
- A session folytatódik, a felhasználó dönt mit csinál
- **Nem kérdez vissza automatikusan** — csak kiírja a hibát és prompt-ot ad

**Deliverable lista:**

- [ ] **M5-RETRY-01**: 3-lépcsős retry ciklus eszkalációval az orchestrator-ban
- [ ] **M5-RETRY-02**: Strukturált hibavisszajelzés (error code + nodeId + fix hint)
- [ ] **M5-RETRY-03**: Dinamikus few-shot példák (`examples.rs`, 10-15 példa, rule-based matching)
- [ ] **M5-RETRY-04**: REPL graceful error handling (session folytatás hiba után)

**Érintett fájlok:** `orchestrator.rs`, `repl.rs`, `main.rs`, `tools.rs`, új `examples.rs`

**Prioritás:** ⭐ **Legnagyobb hatás, legkisebb munka** — a felhasználói élmény azonnal javul.

---

## M8 kiegészítések: Tanulás és Embedding

> [!note] Ez a szekció az [[#Milestone 8 — Multi-Agent Orchestráció & Self-Healing (8–10 hét)]] kiegészítése.

### M8-LEARN: Tanulási Adatbázis és GraphRAG Kontextus

#### Séma-alapú kontextus injektálás
- Teljes gráf JSON helyett **célzott kontextus** (releváns függvények/blokkok)
- Séma összefoglaló: elérhető Op típusok, típusszabályok, kötelező mezők
- Kevesebb token → jobb LLM fókusz, olcsóbb API hívások

#### Sikeres műveletek naplózása
- `.duumbi/learning/successes.jsonl` — minden sikeres `duumbi add` naplózva
- A retry prompt lekérdezheti: "Hasonló kérés korábban így sikerült: ..."
- Jövőbeli RAG/fine-tuning alapja

#### GraphRAG integráció
A JSON-LD gráf maga a tudásbázis (nem kell külön vektor DB!):
- **Szemantikus keresés:** petgraph traversal + séma metadata
- **Multi-hop reasoning:** "Melyik függvény hívja a factorial-t?" → gráf bejárás
- A prompt kontextusa a gráf struktúrából épül

**Deliverable-ök:**

- [ ] **M8-LEARN-01**: Séma-alapú kontextus injektálás (célzott gráf-részlet az LLM-nek)
- [ ] **M8-LEARN-02**: Sikeres műveletek naplózása (`.duumbi/learning/successes.jsonl`)
- [ ] **M8-LEARN-03**: Automatikus few-shot válogatás korábbi sikerekből
- [ ] **M8-LEARN-04**: GraphRAG integráció (petgraph traversal alapú kontextus-építés)

**Érintett fájlok:** `orchestrator.rs`, új `learning.rs`, új `context.rs`

### M8-EMBED: Holografikus/Komplex Embedding Kutatás

> [!info] Kutatási task — PoC szintű implementáció.

**Háttér:** A [Holographic Embeddings of Knowledge Graphs (AAAI 2016)](https://arxiv.org/abs/1510.04935) szerint a komplex számokra épülő embedding-ek előnyei tudásgráfoknál:

- **Circular correlation** megőrzi a relációk irányát (A→B ≠ B→A)
- **Lineáris memória-komplexitás** (nem kell nagy vektor DB)
- **Asszociatív memória** — a gráf struktúra implicit kódolva

**Relevancia:** A duumbi JSON-LD gráf irányított gráf, ahol node relációk fontosak. Holografikus embedding-ek pontosan ezt ragadják meg.

**Deliverable-ök:**

- [ ] **M8-EMBED-01**: PoC — duumbi JSON-LD gráf → HolE embedding → hasonlóság-keresés
- [ ] **M8-EMBED-02**: Integrálás a Repair Agent kontextus-építésébe

**Alternatíva:** [ComplEx embeddings](https://www.semanticscholar.org/paper/Complex-and-Holographic-Embeddings-of-Knowledge-A-Trouillon-Nickel/607e0bafc93089d25fed6d2ba1a41637ed7) (egyszerűbb implementáció)

**Források:**
- [Holographic Embeddings of Knowledge Graphs (AAAI 2016)](https://arxiv.org/abs/1510.04935)
- [ComplEx vs HolE összehasonlítás](https://www.semanticscholar.org/paper/Complex-and-Holographic-Embeddings-of-Knowledge-A-Trouillon-Nickel/607e0bafc93089d25fed6d2ba1a41637ed7)
- [GraphRAG Complete Guide 2026](https://www.meilisearch.com/blog/graph-rag)
- [Structured Output AI Reliability](https://www.cognitivetoday.com/2025/10/structured-output-ai-reliability/)
- [RAG vs Fine-Tuning 2026 Guide](https://umesh-malik.com/blog/rag-vs-fine-tuning-llms-2026)
- [GraphRAG & Knowledge Graphs (Fluree)](https://flur.ee/fluree-blog/graphrag-knowledge-graphs-making-your-data-ai-ready-for-2026/)

---

## Összefoglaló prioritás (AI megbízhatóság kiegészítések)

| Szint | Kód | Cél | Hatás |
|-------|-----|-----|-------|
| 1 | **M5-RETRY** | Intelligens retry pipeline | ⭐ Legnagyobb hatás, legkisebb munka |
| 2 | **M8-LEARN** | GraphRAG + tanulási napló | Közép-hosszú táv, rendszer szintű tanulás |
| 3 | **M8-EMBED** | Holografikus embedding kutatás | Hosszútávú innovációs irány |
