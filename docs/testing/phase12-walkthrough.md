# Phase 12: Dynamic Agent System & MCP — Felhasznaloi Utmutato

**Version:** 3.0
**Date:** 2026-03-24
**Branch:** `phase12/dynamic-agent-mcp`
**LLM provider:** MiniMax M2.7 (Token Plan — Starter)

---

## Bevezetes

A Phase 12 elott a DUUMBI minden AI-feladatot egyetlen agent-tel,
sorban hajtott vegre. A Phase 12 utan a rendszer **automatikusan
csapatot allit ossze** a feladat bonyolultsaga alapjan, es
**kulso eszkozokbol is elerheto** (Claude Desktop, Cursor).

Ez az utmutato vegigvezet 5 gyakorlati forgatokonyven — mindenhol
pontosan leírjuk, mit kell csinalnod.

---

## Elokeszites (egyszer kell megcsinalni)

### 1. lepes: Forditas

Nyiss terminalt a DUUMBI repo gyokereben:

```bash
cargo build
```

Sikeres kimenet:
```
   Compiling duumbi v0.1.1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.42s
```

### 2. lepes: Test workspace letrehozasa

**Fontos:** NE a repo mappajaban dolgozz — kulon test konyvtarat
hasznalunk.

```bash
export DUUMBI="$(pwd)/target/debug/duumbi"
mkdir -p /tmp/duumbi-p12-test
cd /tmp/duumbi-p12-test
$DUUMBI init .
```

Sikeres kimenet:
```
✓ Project initialized at /tmp/duumbi-p12-test
```

### 3. lepes: MiniMax M2.7 provider beallitasa

A DUUMBI teszteleshez a **MiniMax M2.7** modellt hasznaljuk a
**Token Plan — Starter** elofizetes keresztul. Ez az Anthropic
API-val kompatibilis vegpontot nyujt, igy a DUUMBI beepitett
Anthropic providere tortenetis valtoztatas nelkul mukodik.

#### 3a. Regisztracio a MiniMax platformon

1. Nyisd meg a bongeszoben: **https://platform.minimax.io**
2. Kattints a **"Sign Up"** gombra (jobb felso sarok)
3. Regisztralj e-mail cimmel **vagy** Google/GitHub fiokkal
4. Erosítsd meg az e-mail cimedet a kapott levellen

#### 3b. Token Plan feliratkozas (Starter)

1. A belépés utan menj ide:
   **https://platform.minimax.io/subscribe/token-plan**
2. Valassza a **Starter** csomagot:
   - **$10 / ho** (vagy $100 / ev — 2 honap ingyen)
   - 1 500 keres / 5 ora (M2.7 modell)
   - Hozzafers az osszes modalitashoz (szoveg, video, hang, kep)
3. Add meg a fizetesi adatokat es kattints a **"Subscribe"** gombra
4. Visszakerul a Dashboard-ra — a status **"Active"** legyen

> **Megjegyzes:** A Token Plan API kulcs **kulon kulcs** a pay-as-you-go
> API kulcstol. Ne keverd ossze a kettot!

#### 3c. API kulcs letrehozasa

1. A Dashboard bal oldalan valaszd: **API Keys** (vagy Settings → API Keys)
2. Kattints a **"Create new key"** gombra
3. Adj nevet a kulcsnak, pl. `duumbi-test`
4. Kattints a **"Create"** gombra
5. **Masold ki azonnal** — csak egyszer lathatod!
   A kulcs formatum: `eyJ...` (JWT forma)

#### 3d. Kornyezeti valtozok beallitasa

Nyiss uj terminalt (vagy add hozza a shell profiljaihoz), es futtasd:

```bash
export MINIMAX_API_KEY="eyJ..."          # a 3c. lepesben masolt kulcs
export ANTHROPIC_BASE_URL="https://api.minimax.io/anthropic"
```

> **Ellenorzes:**
> ```bash
> echo $MINIMAX_API_KEY | head -c 20
> echo $ANTHROPIC_BASE_URL
> ```
> Kimenet (pelda):
> ```
> eyJhbGciOiJSUzI1N
> https://api.minimax.io/anthropic
> ```

#### 3e. DUUMBI config beallitasa

```bash
nano /tmp/duumbi-p12-test/.duumbi/config.toml
```

Add hozza a fajl vegehez:

```toml
[[providers]]
provider = "Anthropic"
role = "Primary"
api_key_env = "MINIMAX_API_KEY"
```

Mentsd el (Ctrl+O, Enter, Ctrl+X).

> **Miert mukodik ez?** A MiniMax az Anthropic API formatumot valositja
> meg (`/anthropic` vegpont), ezert a DUUMBI Anthropic providere
> valtozatlan kodokkal mukodik — csak a vegpont URL es az API kulcs
> kulonbozik. Az `ANTHROPIC_BASE_URL` kornyezeti valtozot az Anthropic
> SDK automatikusan felhasznalia.

---

## 1. forgatokonyv: DUUMBI csatlakoztatasa a Claude Desktop-hoz

**Cel:** A Claude Desktop (vagy Cursor) eszkozkent latja a DUUMBI-t
es termeszetes nyelven kezelhetod a programod grafjat.

### 1.1 lepes: Keresd meg a duumbi binarist

```bash
which duumbi || echo "$(pwd)/target/debug/duumbi"
```

Jegyezd meg az eleresi utat, pl.: `/Users/te/duumbi/target/debug/duumbi`

### 1.2 lepes: Claude Desktop MCP konfiguracio

Nyisd meg a Claude Desktop beallitasait:
- **macOS:** Claude menu → Settings → Developer → Edit Config
- **Fajl helye:** `~/Library/Application Support/Claude/claude_desktop_config.json`

Add hozza a `"mcpServers"` szekciohoz (ha mar van mas szerver, vesszot
rakj ele):

```json
{
  "mcpServers": {
    "duumbi": {
      "command": "/Users/te/duumbi/target/debug/duumbi",
      "args": ["mcp"],
      "cwd": "/tmp/duumbi-p12-test"
    }
  }
}
```

> **Fontos:** A `command` legyen a teljes abszolut eleresi ut, ne
> `$DUUMBI`. A `cwd` az a mappa, ahol a `.duumbi/` konyvtar van.

### 1.3 lepes: Claude Desktop ujrainditasa

Zard be es nyisd ujra a Claude Desktop-ot.

### 1.4 lepes: Ellenorizd az integraciot

A Claude Desktop chat ablakaban ird be:

> _"Milyen fuggvenyek vannak a DUUMBI workspace-ben?"_

A Claude most a `graph_query` eszkozt fogja hasznalni. A valaszban
latni fogod a workspace grafjanak fuggvenyeit (a `duumbi init` altal
letrehozott `main` fuggvenyt).

### 1.5 lepes: Probalj ki tobb muveletet

Kerdd a Claude-ot termeszetes nyelven:

- _"Validald a DUUMBI grafot"_ → a `graph_validate` eszkozt hívja
- _"Ird le, mit csinal a program"_ → a `graph_describe` eszkozt hívja

**Igy nezel ki a Claude Desktop-ban:** A DUUMBI 10 eszkozt kinal
(graph_query, graph_mutate, graph_validate, stb.) — a Claude
automatikusan valasztja ki a megfelelot a kerdésed alapjan.

### Cursor integracio

Ha Cursor-t hasznalsz Claude Desktop helyett:

1. Hozz letre `.cursor/mcp.json` fajlt a projekt gyokereben:
   ```json
   {
     "mcpServers": {
       "duumbi": {
         "command": "/Users/te/duumbi/target/debug/duumbi",
         "args": ["mcp"],
         "cwd": "/tmp/duumbi-p12-test"
       }
     }
   }
   ```
2. Inditsd ujra a Cursor-t
3. A Composer-ben (Cmd+I) lathatod a DUUMBI eszkozokat

---

## 2. forgatokonyv: Koltsegvedelem beallitasa

**Cel:** Korlatozzuk, mennyi tokent hasznalhatnak az AI agentek,
hogy ne lepje tul a Token Plan keretet.

> **Token Plan Starter limit:** 1 500 keres / 5 ora. Az alabb
> beallitott token korlatok segítenek elosztani a keretet, hogy egy
> nagy intent ne hasznaljon el mindent egyszeriben.

### 2.1 lepes: Nyisd meg a config fajlt

```bash
cd /tmp/duumbi-p12-test
nano .duumbi/config.toml
```

### 2.2 lepes: Add hozza a [cost] szekciót

Illesszd be a fajl vegehez:

```toml
[cost]
budget-per-intent = 50000
budget-per-session = 200000
max-parallel-agents = 3
circuit-breaker-failures = 5
alert-threshold-pct = 80
```

Mentsd el.

### Mit jelentenek ezek?

| Beallitas | Mit csinal | Alapertek |
|-----------|------------|-----------|
| `budget-per-intent` | Egy `intent execute` parancs max ennyi tokent hasznalhat | 50 000 |
| `budget-per-session` | Egy teljes CLI session (REPL) max ennyi tokent hasznalhat | 200 000 |
| `max-parallel-agents` | Hany AI agent futhat egyszerre (parhuzamos feladatoknal) | 3 |
| `circuit-breaker-failures` | Ennyi egymast koveto hiba utan leall az agent-inditas | 5 |
| `alert-threshold-pct` | Figyelmeztes %-ban (pl. 80% = 40 000 token utan szol) | 80 |

### Mi tortenik, ha tullepjuk?

- **Budget tullepes:** `E040 BUDGET_EXCEEDED` hiba. A mar elkeszult
  munka megmarad, de uj agent nem indul.
- **Tul sok hiba egymas utan:** `E041 CIRCUIT_OPEN`. A rendszer
  blokkol minden uj agent-inditast, amig nem reseteled.
- **Nincs szabad slot:** `E044 AGENT_TIMEOUT`. Az agent 60
  masodpercig var, utana lemond.

### Ha nem kell koltsegvedelem

A `[cost]` szekció teljesen **opcionalis**. Ha nem adod meg, a
rendszer az alapertelmezett ertekekkel mukodik. A regi config.toml
fajlok valtozatas nelkul tovabb mukodnek.

---

## 3. forgatokonyv: Agent template-ek testreszabasa

**Cel:** Letrehozol egy sajat agent-tipust, ami mas promptot es
mas eszkozokat hasznal, mint a beepitett Coder.

### 3.1 lepes: Nezd meg a beepitett template-eket

```bash
ls .duumbi/knowledge/agent-templates/
```

Kimenet:
```
coder.json
planner.json
repair.json
reviewer.json
tester.json
```

Ez az 5 beepitett szerepkor, amit a `duumbi init` letrehozott.

### 3.2 lepes: Nezz bele egy template-be

```bash
cat .duumbi/knowledge/agent-templates/coder.json | python3 -m json.tool
```

Kimenet (roviden):
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

### 3.3 lepes: Hozz letre sajat template-et

Peldaul egy "Security Auditor" agentet, aki biztonsagi szempontbol
vizsgalja a kodot:

```bash
cp .duumbi/knowledge/agent-templates/reviewer.json \
   .duumbi/knowledge/agent-templates/security_auditor.json
```

### 3.4 lepes: Szerkeszd a template-et

```bash
nano .duumbi/knowledge/agent-templates/security_auditor.json
```

Valtoztasd meg:
- `"@id"` → `"duumbi:template/security_auditor"`
- `"duumbi:name"` → `"Security Auditor"`
- `"duumbi:systemPrompt"` → `"You are a security auditor agent. Review code for injection vulnerabilities, unsafe operations, and authentication issues."`
- `"duumbi:specialization"` → `["review", "security"]`

Mentsd el. A rendszer a kovetkezo futasnal automatikusan betolti
az uj template-et.

### 3.5 lepes: Ellenorizd

```bash
ls .duumbi/knowledge/agent-templates/
```

```
coder.json
planner.json
repair.json
reviewer.json
security_auditor.json   ← az uj template
tester.json
```

---

## 4. forgatokonyv: Intent vegrehajtasa dinamikus csapattal

**Cel:** Letrehozol egy intent-et (feladat-spec), amit a rendszer
automatikusan elemez es a megfelelo agent-csapatot allitja ossze.

> **Elofeltetel:** A 3. lepes elvegzese (MiniMax M2.7 beallitasa).

### 4.1 lepes: Intent letrehozasa

```bash
$DUUMBI intent create "Build a calculator module with add and multiply functions"
```

A rendszer egy YAML fajlt general:
```
✓ Intent spec created: .duumbi/intents/build-a-calculator-module.yaml
```

### 4.2 lepes: Nezd meg a spec-et

```bash
cat .duumbi/intents/build-a-calculator-module.yaml
```

Pelda kimenet:
```yaml
intent: "Build a calculator module with add and multiply functions"
version: 1
status: Pending
acceptance_criteria:
  - "add(a, b) returns a + b for i64 inputs"
  - "multiply(a, b) returns a * b for i64 inputs"
modules:
  create: ["calculator/ops"]
  modify: ["app/main"]
test_cases:
  - name: basic_add
    function: add
    args: [3, 5]
    expected_return: 8
  - name: basic_multiply
    function: multiply
    args: [4, 7]
    expected_return: 28
```

### 4.3 lepes: A rendszer igy elemzi a feladatot

A Phase 12 Task Analysis Engine ertekeli a spec-et:

```
Complexity:  Moderate   (2 test case → 2-5 tartomany)
TaskType:    Create     (van modules.create)
Scope:       MultiModule (1 create + 1 modify = 2 modul)
Risk:        Medium     (main modosul)

→ Csapat: Planner → Coder → Reviewer → Tester
→ Strategia: Pipeline
```

Ezt nem kell te beallitanod — a rendszer automatikusan donti el.

### 4.4 lepes: Futtasd a spec-et

```bash
$DUUMBI intent execute build-a-calculator-module
```

A kimenet mutatja, hogyan halad vegig a csapaton:
```
Executing intent: "Build a calculator module with add and multiply functions"

Plan (3 tasks):
  [1/3] Create module calculator/ops with functions: add, multiply
  [2/3] Add function: wire calculator functions into main
  [3/3] Verify all test cases

[1/3] Create module calculator/ops…
  ✓ Done (2 ops). Added 2 functions: add, multiply

[2/3] Wire into main…
  ✓ Done (3 ops). Modified main to call add and multiply

[3/3] Running 2 tests…
  ✓ basic_add: add(3, 5) = 8
  ✓ basic_multiply: multiply(4, 7) = 28

All 3 tasks completed.
Intent completed successfully.
```

### Mi tortent a hatterben?

1. **TaskAnalyzer** elemezte a spec-et (LLM-hivas nelkul!)
2. **TeamAssembler** a 9-soros lookup tablabol kivalasztotta a csapatot
3. A **Planner** szetbontotta a feladatot reszfeladatokra
   (→ MiniMax M2.7 API-hivas: `POST https://api.minimax.io/anthropic/v1/messages`)
4. A **Coder** legeneralta a kodot (MiniMax M2.7 API-hivas)
5. A **Reviewer** ellenorizte a patcht (ha a kockazat ≥ Medium)
6. A **Tester** futtatta a teszteket (forditas + vegrehajtas)

### Ha valami nem sikerul

A rendszer automatikusan kezeli:
- **Retry**: max 3 probalkozas, egyre reszletesebb hibauzenettel
- **Rollback**: ha a csapat megakad, az osszes fajl visszaall
- **Knowledge**: a sikerek es kudarcok felkerulnek a tudasbazisba

---

## 5. forgatokonyv: A tudasbazis megtekintese

**Cel:** Megnezed, mit tanult a rendszer a korabbi futatasokból.

### 5.1 lepes: Tudasbazis listazasa

```bash
$DUUMBI knowledge list
```

Pelda kimenet (ha mar futtattad az intent-et):
```
Success records: 3
  duumbi:success/1711234567890-1  CreateModule  calculator/ops  (2 ops)
  duumbi:success/1711234567891-2  ModifyMain    main           (3 ops)
  duumbi:success/1711234567892-3  AddFunction   main           (1 ops)

Decision records: 0
Pattern records: 0
```

### 5.2 lepes: Strategiak megtekintese

A strategiak a `.duumbi/knowledge/strategies/` mappaban vannak:

```bash
ls .duumbi/knowledge/strategies/ 2>/dev/null || echo "Meg nincs strategia"
```

A strategiak az ido elorehaladasaval gyulnek — minden sikeres es
sikertelen feladat utan a rendszer frissiti a szamlalokat.

### 5.3 lepes: Hogyan mukodik a tanulas?

Kepzeld el, hogy 10-szer futtattad az `intent execute`-ot:

```
Strategy: "Multi-function module: use add_function for each"
  successCount: 7    (7x sikeres volt ez a megkozelites)
  failCount: 3       (3x nem mukodott)
  deprecated: false  (30% fail < 70% → meg aktiv)
```

Ha a fail rate tullep 70%-ot (minimum 10 probalkozas utan):

```
Strategy: "Single block for everything"
  successCount: 2
  failCount: 8
  deprecated: true   (80% fail → a rendszer mar nem hasznalja)
```

A deprecated strategiak **soha nem torlodnek** — az audit trail
megmarad, de a rendszer nem ajánlja oket tobbe uj feladatokhoz.

---

## Kulso MCP szerverek csatlakoztatasa (halado)

Ha a DUUMBI agentjeit szeretned osszekotni kulso eszkozokkel
(pl. Figma, GitHub, bongeszo), add hozza a config.toml-hoz:

### Konfiguracio

```bash
nano .duumbi/config.toml
```

Add hozza:

```toml
[mcp-clients]
github = { url = "https://github.mcp.example.com/sse", description = "GitHub repository" }
figma = { url = "https://figma.mcp.example.com/sse", description = "Figma design data" }
```

### Biztonsag

Csak az itt felsorolt szerverek erhetok el. Ha le akarsz tiltani
egy szervert anelkul, hogy torolned:

```toml
[mcp-clients]
old-server = { url = "...", trusted = false }
```

---

## Hibauzenetek es megoldasuk

Ha valamelyik forgatokonyvben hibat kapsz, itt megtalálod a
magyarazatat:

| Hibauzenet | Mit jelent | Mit tegyel |
|------------|------------|------------|
| `E040 BUDGET_EXCEEDED` | Elfogyott a token keret | Noveld a `budget-per-intent` erteket a config-ban, vagy bontsd kisebb intent-ekre |
| `E041 CIRCUIT_OPEN` | Tul sok egymast koveto hiba | Ellenorizd a `MINIMAX_API_KEY`-t es az `ANTHROPIC_BASE_URL`-t, majd probalj ujra |
| `E044 AGENT_TIMEOUT` | Tul sokaig vart szabad slotra | Csokkentsd a parhuzamos feladatokat, vagy noveld `max-parallel-agents`-et |
| `E045 TEMPLATE_NOT_FOUND` | Ismeretlen agent template | Ellenorizd a `.duumbi/knowledge/agent-templates/` mappat |
| `E047 MCP_CLIENT_UNREACHABLE` | Kulso szerver nem erheto el | Ellenorizd a `[mcp-clients]` URL-eket a config-ban |
| `401 Unauthorized` | Ervenytelen MiniMax API kulcs | Ellenorizd az `MINIMAX_API_KEY` erteket; generalj uj kulcsot a platform.minimax.io-n |
| `429 Too Many Requests` | Tullepted a Token Plan limitet (1500 keres/5 ora) | Varj a limit visszaallasara, vagy bontsd kisebb feladatokra az intent-et |

---

## Takaritas

Amikor befejezted a tesztelest:

```bash
cd ~
rm -rf /tmp/duumbi-p12-test
unset DUUMBI MINIMAX_API_KEY ANTHROPIC_BASE_URL
```

---

## Osszefoglalo: Hol talalhatok a Phase 12 fajlok?

```
.duumbi/
  config.toml                         ← [[providers]], [cost] es [mcp-clients] szekciok
  knowledge/
    agent-templates/                  ← Agent template-ek (JSON)
      coder.json
      planner.json
      reviewer.json
      tester.json
      repair.json
      security_auditor.json           ← sajat template (ha letrehoztad)
    strategies/                       ← Tanult strategiak (JSON-LD)
    failure-patterns/                 ← Tanult hibamitak (JSON-LD)
```

**CLI parancsok:**
- `duumbi mcp` — MCP szerver inditasa (Claude Desktop / Cursor integracio)
- `duumbi intent create "..."` — Intent spec letrehozasa
- `duumbi intent execute <name>` — Intent vegrehajtasa (dinamikus csapattal)
- `duumbi knowledge list` — Tudasbazis megtekintese

---

## Hasznos linkek

- MiniMax platform (regisztracio, API kulcs): https://platform.minimax.io
- Token Plan feliratkozas: https://platform.minimax.io/subscribe/token-plan
- MiniMax M2.7 dokumentacio: https://platform.minimax.io/docs/guides/text-ai-coding-tools
- MiniMax M2.7 modell leiras: https://www.minimax.io/models/text/m27
