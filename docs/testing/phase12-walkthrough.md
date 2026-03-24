# Phase 12: Dynamic Agent System & MCP — Feature Walkthrough

**Version:** 1.0
**Date:** 2026-03-24
**Branch:** `phase12/dynamic-agent-mcp`

Ez a dokumentum vegigvezet a Phase 12 uj funkcioin. Nem szukseges korabbi
DUUMBI ismeret — minden lepest a nullarol magyarazunk.

---

## Elofeltetelek

1. **Rust toolchain** telepitve (`rustup show` → stable)
2. Forditsd le a projektet:
   ```bash
   cd /path/to/duumbi
   git checkout phase12/dynamic-agent-mcp
   cargo build
   ```
3. Exportald a binaris eleresi utjat:
   ```bash
   export DUUMBI="$(pwd)/target/debug/duumbi"
   ```
4. Hozz letre egy test workspace-t (NE a repo mappajaban):
   ```bash
   mkdir -p /tmp/duumbi-p12-walkthrough
   cd /tmp/duumbi-p12-walkthrough
   $DUUMBI init .
   ```

---

## 1. MCP Server — A DUUMBI mint kulso eszkoz

A Phase 12 leglathatobb ujdonsaga: a DUUMBI CLI mostantol MCP
(Model Context Protocol) szerverkent is mukodik. Ez azt jelenti, hogy
barmely MCP-kompatibilis kliens (Claude Desktop, Cursor, sajat script)
tavvezerelheti a graf-muveletet.

### 1.1 Az MCP szerver inditasa

```bash
$DUUMBI mcp
```

A szerver elindul es JSON-RPC 2.0 keresekre var a stdin-en.
A valaszok a stdout-ra erkeznek. Minden uzenet egy sor (newline-terminated JSON).

### 1.2 Elso keres: inicializalas

Kuldj egy `initialize` kerest a stdin-re (egy sor!):

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

**Elvart valasz** (formatazva az olvashatosag kedveert):
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "serverInfo": {
      "name": "duumbi-mcp",
      "version": "0.1.1"
    },
    "capabilities": {
      "tools": {}
    }
  }
}
```

### 1.3 Eszkozok listazasa

```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

A valasz tartalmazza mind a 10 elerheto eszkozt:

| Eszkoz | Leiras |
|--------|--------|
| `graph_query` | Graf lekerdezes (node ID, @type, nev minta alapjan) |
| `graph_mutate` | Atomikus patch muveletek a grafon |
| `graph_validate` | Validacio: parse → build → validate pipeline |
| `graph_describe` | Pszeuodokoddá alakitas (ember altal olvashato) |
| `build_compile` | Forditas nativ binaris-sa (CLI kell hozza) |
| `build_run` | Forditas + futtatas (CLI kell hozza) |
| `deps_search` | Modul kereses a registry-ben |
| `deps_install` | Fuggosegek telepitese |
| `intent_create` | Intent spec generalas termeszetes nyelvi leirasbol |
| `intent_execute` | Intent vegrehajtasa (decompose → mutate → verify) |

### 1.4 Graf lekerdezes

Kerdezd le a workspace grafot @type alapjan:

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"graph_query","arguments":{"type_filter":"duumbi:Function"}}}
```

A valasz tartalmazza az osszes fuggvenyt a grafbol:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [{ "type": "text", "text": "{\"nodes\": [...]}" }]
  }
}
```

Egyedi node lekerdezes `node_id`-val:

```json
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"graph_query","arguments":{"node_id":"duumbi:test/main"}}}
```

### 1.5 Graf validacio

```json
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"graph_validate","arguments":{}}}
```

A valasz `valid: true/false` es egy `diagnostics` tombot tartalmaz.

### 1.6 Leallit

Nyomj Ctrl+D-t (EOF) a szerver leallitasahoz.

### 1.7 Claude Desktop / Cursor integracio

Adj hozza a `.mcp.json` (vagy a kliens konfiguracios fajljaba):

```json
{
  "mcpServers": {
    "duumbi": {
      "command": "/path/to/duumbi",
      "args": ["mcp"],
      "cwd": "/path/to/your/workspace"
    }
  }
}
```

Ezutan a Claude Desktop-ban (vagy Cursor-ben) lathatod a 10 DUUMBI eszkozt
es termeszetes nyelven kerhetsz graf-muveletet.

---

## 2. Task Analysis Engine — Hogyan dont a rendszer?

A Phase 12 bevezette a dinamikus csapatosszeallitast. Amikor egy intent
spec erkezik, a rendszer **LLM-hivas nelkul, determinisztikusan** elemzi
negy dimenzio menten:

### 2.1 A negy dimenzio

| Dimenzio | Ertekek | Szamitas |
|----------|---------|----------|
| **Complexity** | Simple / Moderate / Complex | Test case-ek szama: 0-1 → Simple, 2-5 → Moderate, 6+ → Complex |
| **TaskType** | Create / Modify / Test / Refactor / Fix | Intent szoveg kulcsszavai ("fix", "refactor", "test") + strukturalis jelek |
| **Scope** | SingleModule / MultiModule | modules.create + modules.modify szama: 0-1 → Single, 2+ → Multi |
| **Risk** | Low / Medium / High | Main modul erintett? Exports valtozik? Tobb modul modosul? |

### 2.2 Peldak

**Egyszeru feladat** — `"Add an add function"`, 1 test case, 1 modul:
```
Complexity: Simple, TaskType: Create, Scope: SingleModule, Risk: Low
→ Csapat: 1× Coder, szekvencialisan
```

**Kozepes multi-modul feladat** — `"Build a calculator with ops and display"`,
3 test case, 2 modules.create:
```
Complexity: Moderate, TaskType: Create, Scope: MultiModule, Risk: Low
→ Csapat: Planner → 2× Coder (parhuzamosan!) → Tester
```

**Komplex refaktoralas** — `"Refactor the authentication module"`, 8 test case:
```
Complexity: Complex, TaskType: Refactor, Scope: SingleModule, Risk: Medium
→ Csapat: Planner → Coder → Reviewer → Tester (pipeline)
```

### 2.3 A 9-soros lookup tabla

| # | Profil | Csapat | Strategia |
|---|--------|--------|-----------|
| 1 | Simple + Create + Single + Low | 1× Coder | Szekvenciális |
| 2 | Simple + Modify + Single + Low | 1× Coder | Szekvenciális |
| 3 | \* + Test + \* + \* | 1× Tester | Szekvenciális |
| 4 | Moderate + Create + Single + \* | Planner → Coder → Tester | Pipeline |
| 5 | Moderate + Create + Multi + \* | Planner → N× Coder → Tester | **Parhuzamos** |
| 6 | Moderate + Modify + \* + Medium/High | Planner → Coder → Reviewer → Tester | Pipeline |
| 7 | Complex + \* + Multi + \* | Planner → N× Coder → Reviewer → Tester | **Parhuzamos** |
| 8 | \* + Refactor + \* + \* | Planner → Coder → Reviewer → Tester | Pipeline |
| 9 | \* + Fix + \* + \* | 1× Coder (hiba kontextussal) | Szekvenciális |

Ha az elemzes barmi okbol kudarcot vall → visszaesik az egyszeru 1× Coder
modba (graceful degradation).

---

## 3. Agent Templates — 5 beepitett szerepkor

A Phase 12 ot beepitett agent template-tel erkezik. Ezek nem
hardkodolt tipusok, hanem **JSON-LD grafcsomópontok** — bovithetok.

### 3.1 A seed template-ek

| Szerep | Eszkozei | Specializacio |
|--------|----------|---------------|
| **Planner** | — | Feladat dekompozicio, tervezes |
| **Coder** | add_function, add_block, add_op, modify_op, remove_node, set_edge, replace_block | Kod generalas, modositas |
| **Reviewer** | — | Patch validacio, code review |
| **Tester** | — | Teszt vegrehajttas, verifikacio |
| **Repair** | (mint Coder) | Hibajavitas, error recovery |

### 3.2 Template-ek megtekintese

Az init utan a seed template-ek ide kerulnek:
```
.duumbi/knowledge/agent-templates/
  planner.json
  coder.json
  reviewer.json
  tester.json
  repair.json
```

Egy template tartalma (pelda):
```json
{
  "@type": "duumbi:AgentTemplate",
  "@id": "duumbi:template/coder",
  "duumbi:name": "Coder",
  "duumbi:role": "coder",
  "duumbi:systemPrompt": "You are a code generation agent...",
  "duumbi:tools": [
    "add_function", "add_block", "add_op",
    "modify_op", "remove_node", "set_edge", "replace_block"
  ],
  "duumbi:specialization": ["create", "modify"],
  "duumbi:tokenBudget": 4096,
  "duumbi:templateVersion": "1.0.0"
}
```

### 3.3 Sajat template keszitese

Masolj egy meglevo template-et es modositsd:
```bash
cp .duumbi/knowledge/agent-templates/coder.json \
   .duumbi/knowledge/agent-templates/security_auditor.json
```

Szerkeszd a fajlt — valtoztasd meg a nevet, szerepet, promptot.
A rendszer automatikusan betolti a kovetkezo futasnal.

---

## 4. Agent Knowledge — Onjavito tudasbazis

Az agentek minden futatasbol tanulnak. A sikereket es kudarcokat
JSON-LD csomópontkent tarolják.

### 4.1 Strategiak es hibamitak

A `.duumbi/knowledge/` mappastruktua:
```
.duumbi/knowledge/
  strategies/          # Sikeres megkozelitesek
    strategy-1711234567890-1.json
  failure-patterns/    # Ismetlodo hibak
    pattern-1711234567891-1.json
```

**Strategy pelda:**
```json
{
  "@type": "duumbi:Strategy",
  "@id": "duumbi:strategy/1711234567890-1",
  "templateId": "duumbi:template/coder",
  "description": "Multi-function modules: create all at once",
  "triggerPattern": "create task with 3+ functions",
  "approach": "Use add_function for each, then wire exports",
  "successCount": 5,
  "failCount": 1,
  "deprecated": false
}
```

### 4.2 Automatikus pruning

Ha egy strategia tobb mint 70%-ban kudarcot vall (es legalabb 10
kiserlet utan), a rendszer **deprecated** allapotba helyezi — de
**soha nem torli**. Igy az audit trail megmarad.

```
successCount: 2, failCount: 8, total: 10 → 80% fail → deprecated=true
successCount: 4, failCount: 3, total: 7  → <10 attempt → nem deprecated
successCount: 6, failCount: 4, total: 10 → 40% fail → aktiv marad
```

---

## 5. Cost Control — Koltsegvedelem

A Phase 12 kemeny koltsegkorlatokat vezet be, hogy az agent csapatok ne
futtatassak ki a tokenburgeted.

### 5.1 Konfiguracio

Add hozza a `.duumbi/config.toml`-hoz:

```toml
[cost]
budget-per-intent = 50000       # Max token egy intent vegrehajtas soran
budget-per-session = 200000     # Max token egy CLI session soran
max-parallel-agents = 3         # Egyszerre futo LLM hivasok szama
circuit-breaker-failures = 5    # Ennyszer egymst után kudarcra → leállás
alert-threshold-pct = 80        # Figyelmeztet ennyi %-nal
```

Mind a 5 mezo **opcionalis** — ha nem adod meg, az alapertelmezett
ertekek lepnek eletbe. A `[cost]` szekciót is elhagyhatod — a regi
config.toml-ok tovabbra is mukodnek.

### 5.2 Budget enforcement

Minden LLM hivas elott a rendszer ellenorzi: `check_budget()`.
Ha tullepne a korlátot → **E040 BUDGET_EXCEEDED** hiba, a feladat
leall, de a korabbi sikeresen vegzett munka megmarad.

### 5.3 Circuit Breaker

Ha 5 egymast koveto LLM hivas kudarcot vall (halozati hiba, timeout,
rate limit), a circuit breaker **Open** allapotba kerul es nem enged
tobb agent-et inditani. Ez vedi a koltsegkeretet a vegtelen ujraprobas
ellen.

```
Closed  ─── 5 failures ──→  Open (block)
                              │
                          reset()
                              │
                              ▼
                          HalfOpen ─── success ──→ Closed
                              │
                           failure
                              │
                              ▼
                            Open
```

### 5.4 Rate Limiter

A `max-parallel-agents` mezo korlatozza az egyideju LLM hivasokat.
Ha minden slot foglalt, az uj agent max 60 masodpercig var → **E044
AGENT_TIMEOUT** ha nem szabadul fel hely.

---

## 6. Concurrent Merge — Parhuzamos agentek eredmenyeinek osszefesulese

Amikor tobb Coder agent parhuzamosan dolgozik kulonbozo modulokon,
az eredmenyeiket ossze kell fusulni.

### 6.1 Az 5 merge szabaly

| # | Eset | Strategia |
|---|------|-----------|
| 1 | Kulonbozo modulok | Mindketto alkalmazva — nincs konfliktus |
| 2 | Kozos import | Import-ok halmaz-unioja |
| 3 | Mindketto modositja main.jsonld-t | Szekvencialis merge, ujravalidalas kozott |
| 4 | Azonos node @id | **Mindket patch elutasitva** → Planner ujratervez |
| 5 | Cross-module referencia | Topologiai sorrend (letrehozo elobb, hivo utana) |

### 6.2 Atomic rollback

A rendszer a csapat-vegrehajtas elott **snapshot-ot** ment az osszes
`.jsonld` fajlrol. Ha barmely agent veglegesen megakad (max retry utan):

1. Az osszes `.jsonld` fajl visszaall a snapshot-bol
2. A kudarc felkerul a knowledge graph-ba (tanulsag)
3. A felhasznalo ertesitest kap, mit probalt es mi nem sikerult

---

## 7. MCP Client — Kulso szerverek integracioja

A DUUMBI agentek nemcsak MCP szervert nyujtanak, hanem **kliens**kent
is kapcsolodhatnak kulso MCP szerverekhez (Figma, GitHub, bongeszo, DB).

### 7.1 Konfiguracio

```toml
[mcp-clients]
figma = { url = "https://figma.mcp.example.com/sse", description = "Figma design data" }
github = { url = "https://github.mcp.example.com/sse", description = "GitHub repos" }
browser = { url = "http://localhost:3001/sse", description = "Browser automation" }
```

### 7.2 Biztonsag

Csak a `[mcp-clients]` szekcioban explicit konfiguralt szerverek
erhetok el. Nincs implicit trust — a felhasznalo donti el, mely
kulso szerverek megbizhatoak.

Egy szerver `trusted = false` jelolessel letiltható:
```toml
[mcp-clients]
untrusted-server = { url = "...", trusted = false }
```

---

## 8. Uj Error Kodok

A Phase 12 a kovetkezo uj hibakodokkal boviti a rendszert:

| Kod | Nev | Mikor jelenik meg |
|-----|-----|-------------------|
| E040 | BUDGET_EXCEEDED | LLM token budget tullepes |
| E041 | CIRCUIT_OPEN | Circuit breaker nyitott — tul sok egymast koveto kudarc |
| E042 | MERGE_CONFLICT | Parhuzamos patchek osszeferhetetlen konfliktusa |
| E043 | NODE_ID_COLLISION | Ket patch azonos @id-vel hoz letre node-ot |
| E044 | AGENT_TIMEOUT | Agent inditas timeout (60s queue) |
| E045 | TEMPLATE_NOT_FOUND | Hivatkozott agent template nem talalhato |
| E046 | MCP_TOOL_ERROR | MCP eszkoz hivas sikertelen |
| E047 | MCP_CLIENT_UNREACHABLE | Kulso MCP szerver nem erheto el |
| E048 | MCP_CLIENT_TOOL_NOT_FOUND | Igenyelt eszkoz nem lezezik a kulso szerveren |

---

## 9. Gyorstalpaló — Amit kiprobalhatsz 5 percben

### 9a. MCP szerver teszt (2 perc)

```bash
cd /tmp/duumbi-p12-walkthrough
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | $DUUMBI mcp
```

Latni fogod az inicializacios valaszt a szervero nevevel es verziojával.

### 9b. Graf lekerdezes MCP-n at (1 perc)

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"graph_query","arguments":{"type_filter":"duumbi:Function"}}}' | $DUUMBI mcp
```

A valasz tartalmazza a workspace graph fuggvenyeit.

### 9c. Graf validacio (1 perc)

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"graph_validate","arguments":{}}}' | $DUUMBI mcp
```

`valid: true` ha a graf helyes, kulonben `diagnostics` tombben latod a hibakat.

### 9d. Eszkoz lista (1 perc)

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | $DUUMBI mcp
```

Mind a 10 eszkozt latod nevvel, leirassal es JSON Schema-val.

---

## Takaritas

```bash
cd ~
rm -rf /tmp/duumbi-p12-walkthrough
unset DUUMBI
```
