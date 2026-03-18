# Phase 10: Intelligent Context & Knowledge Graph — Manual Test Protocol

**Version:** 1.0
**Date:** 2026-03-18
**Branch:** `phase10/knowledge-context`
**PR:** #383

---

## Prerequisites

- [ ] Rust toolchain installed (`rustup show` → stable)
- [ ] Project builds: `cargo build` (a repo gyokereben)
- [ ] Existing tests pass: `cargo test --all` (1244 teszt zold)
- [ ] At least one LLM provider configured
  - Anthropic: `ANTHROPIC_API_KEY` env var set
  - OpenAI: `OPENAI_API_KEY` env var set

### Binary: ne hasznalj `cargo install`

**Ne futtasd `cargo install --path .`** — az a binarist globalisan telepiti
(`~/.cargo/bin/duumbi`), ami zavarhatja a fejlesztest. Mindig a frissen
forditott lokalis binarist hasznald.

---

## Test Workspace Setup

**A tesztelest ne a repo gyokereben vegezd.** A `duumbi init` egy `.duumbi/`
mappat hoz letre, amelynek egyes fajljai **nincsenek gitignore-ban**.

**Egy parancs a teljes tesztkornyezet felallitasara** (a repo gyokereben futtasd):

```bash
# 1. Exportald a binaris eleresi utjat (egyszer, terminal session-onkent)
export DUUMBI="$(pwd)/target/debug/duumbi"

# 2. Hozz letre test workspace-t a repon KIVUL
mkdir -p /tmp/duumbi-p10-test
cd /tmp/duumbi-p10-test

# 3. Init
$DUUMBI init .

# 4. Konfiguralod a provider(eke)t
nano .duumbi/config.toml
```

**Config template:**

```toml
# /tmp/duumbi-p10-test/.duumbi/config.toml
[workspace]
name = "p10-test"

[[providers]]
provider = "Anthropic"
role = "Primary"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
```

**Takaritas a teszteles utan:**

```bash
cd ~
rm -rf /tmp/duumbi-p10-test
unset DUUMBI
```

---

## T1 — Knowledge CLI: Help & Argument Parsing

> Futtatasi hely: **repo gyokere** (workspace nem szukseges a help-hez)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 1.1 | `$DUUMBI knowledge --help` | Help szoveg: list, show, prune, stats subcommandok | ✓ | |
| 1.2 | `$DUUMBI knowledge list --help` | `--type` opcio lathato (success, decision, pattern) | ✓ | |
| 1.3 | `$DUUMBI knowledge prune --help` | `--older-than` opcio lathato (default: 90) | ✓ | |
| 1.4 | `$DUUMBI knowledge show --help` | `id` pozicionalis argumentum lathato | ✓ | |

---

## T2 — Knowledge CLI: Stats & List (Empty Store)

> Futtatasi hely: **`/tmp/duumbi-p10-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 2.1 | `$DUUMBI knowledge stats` | "Knowledge store:" + "Success records: 0" + "Learning log: 0 entries" | ✓ | |
| 2.2 | `$DUUMBI knowledge list` | "No knowledge nodes found." | ✓ | |
| 2.3 | `$DUUMBI knowledge list --type success` | "No knowledge nodes found." | ✓ | |
| 2.4 | `$DUUMBI knowledge show "duumbi:nonexistent"` | "Node not found: duumbi:nonexistent" | ✓ | |
| 2.5 | `$DUUMBI knowledge prune` | "Pruned 0 node(s) older than 90 days." | ✓ | |

---

## T3 — Knowledge Learning: AI Mutation Logs Success

> Futtatasi hely: **`/tmp/duumbi-p10-test/`** (LLM provider konfiggal)
> **Megjegyzes:** Ez a teszt igazi LLM API hivast vegez.

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 3.1 | `$DUUMBI add "add a multiply function that takes two i64 parameters a and b and returns a*b" -y` | Mutacio sikeres (function hozzaadva) | ✓ | |
| 3.2 | `$DUUMBI knowledge stats` | "Learning log: 0 entries" (a CLI `add` nem integralja meg a learning log-ot — csak intent) |✓ | |
| 3.3 | Intent letrehozasa:<br>`$DUUMBI intent create "Create a double function that takes an i64 parameter n and returns n*2. The main function should call double(21) and exit with the result." -y` | Intent YAML letrejott `.duumbi/intents/` alatt | ✓ | |
| 3.4 | `$DUUMBI intent execute <slug>` (a 3.3-ban kapott slug) | Task(ok) futnak, verifier: double(21)=42 PASS | ✓ | |
| 3.5 | `$DUUMBI knowledge stats` | "Learning log: N entries" ahol N >= 1 (intent task success logged) | ✓ | |
| 3.6 | `cat .duumbi/learning/successes.jsonl` | JSONL sorok, minden sor valid JSON; tartalmazza `request`, `taskType`, `opsCount` mezoket | ✓ | |

> **Megjegyzes:** Ha a Coordinator "ModifyMain" egyetlen task-ot general (a fuggvenyt a
> main modulba teszi), a verifier E010-et adhat multi-module modban. Ilyenkor a task
> attol meg "Completed" (a mutation sikeres volt), de a verifier teszt bukik.
> Ez a Coordinator ismert korlatozasa — a learning log ilyenkor is ira task siker rekordot.

---

## T4 — Knowledge Store: CRUD via CLI

> Futtatasi hely: **`/tmp/duumbi-p10-test/`** (T3 utan, ahol mar van learning log)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 4.1 | `$DUUMBI knowledge list` | Ha van intent success: legalabb 0 node (a store meg ures, mert a learning log kulon) | ✓ | |
| 4.2 | `ls .duumbi/knowledge/` | 3 almappa: `success/`, `decision/`, `pattern/` | ✓ | |
| 4.3 | `$DUUMBI knowledge stats` | Osszes szamok osszegezve | ✓ | |

---

## T5 — REPL: /knowledge Slash Command

> Futtatasi hely: **`/tmp/duumbi-p10-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 5.1 | `$DUUMBI` (REPL indul) | `>` prompt megjelenik | ✓ | |
| 5.2 | REPL-ben: `/help` | "Knowledge commands:" szekcio lathato, `/knowledge`, `/knowledge list`, `/knowledge stats` | ✓ | |
| 5.3 | REPL-ben: `/knowledge` | Statisztikak megjelennek (success/decision/pattern/total + learning log count) | ✓ | |
| 5.4 | REPL-ben: `/knowledge stats` | Ugyanaz mint `/knowledge` | ✓ | |
| 5.5 | REPL-ben: `/knowledge list` | Node lista (vagy "No knowledge nodes found.") | ✓ | |
| 5.6 | REPL-ben: `/knowledge invalid` | "Usage: /knowledge [list\|stats]" help uzenet | ✓ | |
| 5.7 | REPL-ben: `/exit` | REPL kilep | ✓ | |

---

## T6 — Context Assembly: Multi-Module Awareness

> Futtatasi hely: **`/tmp/duumbi-p10-test/`** (LLM provider konfiggal)
> **Cel:** Ellenorizni, hogy a `duumbi add` felismeri a meglevo modulokat.

**Setup:** Eloszor hozzuk letre a 3-modulos projektet manualis intent-tel.

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 6.1 | `$DUUMBI intent create "Build a calculator with add and multiply in a separate ops module, then call both from main" -y` | Intent YAML letrejott | | |
| 6.2 | `$DUUMBI intent execute calculator` | Taskok futnak: CreateModule (ops) → AddFunction(add) → AddFunction(multiply) → ModifyMain | | |
| 6.3 | `ls .duumbi/graph/` | `main.jsonld` + `ops.jsonld` (ket modul) | | |
| 6.4 | `$DUUMBI build` | Build sikerul (0 exit code) | | |
| 6.5 | `.duumbi/build/output` | Binary fut, helyes kimenet | | |

**Ezutan teszteljuk a context assembly-t:**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 6.6 | `$DUUMBI add "add a subtract function to the ops module that takes two i64 and returns a-b" -y` | Az LLM az ops modulba teszi (nem main-be), mert a context tartalmazza a modul informaciot | | |
| 6.7 | `$DUUMBI check` | Check sikerul (nem duplikalt fuggveny, helyes modul) | | |
| 6.8 | `$DUUMBI describe` | A subtract fuggveny az ops modulban lathato | | |

---

## T7 — Workspace Module Auto-Discovery

> Futtatasi hely: **`/tmp/duumbi-p10-test/`** (T6 utan)
>
> **Megjegyzes:** A workspace `.duumbi/graph/` konyvtarban levo modulokat a
> `Program::load()` automatikusan betolti — nem kell dependency-kent
> regisztralni a config.toml-ban. Ez a teszt azt ellenorzi.

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 7.1 | `cat .duumbi/config.toml` | A config.toml NEM tartalmaz "ops" dependency-t (nincs [dependencies] szekcio vagy ures) | | |
| 7.2 | `$DUUMBI build` | Build sikerul — a Program::load() megtalalta az ops.jsonld-t automatikusan | | |
| 7.3 | `$DUUMBI check` | Check sikerul — cross-module Call-ok feloldodnak | | |

---

## T8 — Session Persistence

> Futtatasi hely: **`/tmp/duumbi-p10-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 8.1 | `$DUUMBI` (REPL indul) | REPL megnyilik | | |
| 8.2 | REPL-ben: barmely AI mutacio (pl. "add a negate function") | Siker | | |
| 8.3 | REPL-ben: `/status` | Session info megjelenik (workspace, model, turns) | | |
| 8.4 | REPL-ben: `/exit` | REPL bezarul | | |
| 8.5 | `ls .duumbi/session/` | `current.json` letezik | | |
| 8.6 | `cat .duumbi/session/current.json \| python3 -m json.tool` | Valid JSON, tartalmaz `session_id`, `turns`, `usage_stats` mezoket | | |

---

## T9 — MutationOutcome: Clarification Detection

> Ez a teszt nehezen reprodukalhato manualisban, mert az LLM kell hogy
> az `ask_clarification` tool-t hasznalja. Az automatizalt tesztek fedik.

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 9.1 | REPL-ben: homályos keres, pl. "change something" | Az LLM vagy vegrehajtja, vagy `? <kerdes>` jelenik meg (clarification) | | |
| 9.2 | Ha clarification volt: a kovetkezo input a valasz kontextusakent erkezik | A conversation history tartalmazza a clarification-t | | |

---

## T10 — Knowledge Prune

> Futtatasi hely: **`/tmp/duumbi-p10-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 10.1 | `$DUUMBI knowledge prune --older-than 0` | "Pruned N node(s) older than 0 days." — minden node torolve | | |
| 10.2 | `$DUUMBI knowledge stats` | Total: 0 (ha nem volt uj node) | | |
| 10.3 | `$DUUMBI knowledge prune --older-than 365` | "Pruned 0 node(s)" (semmi nem oreg annyira) | | |

---

## T11 — Edge Cases & Error Handling

| # | Lepes | Futtatasi hely | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|----------------|-----------------|-----|------------|
| 11.1 | `$DUUMBI knowledge stats` | Ures konyvtar (nincs `.duumbi/`) | A knowledge alkonyvtar letrehozodik, "Total: 0" | | |
| 11.2 | Torold manualisban `.duumbi/session/current.json`, majd `$DUUMBI` | REPL indul, uj session | Corrupted/missing file recovery mukodik | | |
| 11.3 | Irj hibas JSON-t `.duumbi/session/current.json`-ba, majd `$DUUMBI` | REPL indul, uj session | Graceful recovery | | |
| 11.4 | `echo "not json" >> .duumbi/learning/successes.jsonl` | Ezutan `$DUUMBI knowledge stats` | Learning log szamlalo a valid sorokat szamolja (nem crash-el) | | |

---

## Automated Test Verification (reference)

A kovetkezo tesztek API key nelkul futnak CI-ben (repo gyokereben):

```bash
cargo test knowledge::           # 23 unit test (types, store, learning)
cargo test context::              # ~35 unit test (classifier, traversal, collector, budget, fewshot, modularizer)
cargo test session::              # 7 unit test (session manager)
cargo test integration_phase10    # 69 integration test (5 test fajl)
```

Osszesen **~134 automatizalt teszt** fedi le:
- Knowledge store CRUD (save, load, query by type/tag, remove, overwrite)
- JSONL append + query roundtrip, limit, count
- Scoring (task_type match, error_code overlap, module match)
- KnowledgeNode JSON-LD roundtrip (all 3 types)
- Task classifier (7 type × multiple inputs), ambiguity detection
- Traversal plans (per-TaskType, target module guessing)
- Node collector (dedup, priority sort, empty plan)
- Token budget (CharEstimator, fit_to_budget, priority drop)
- Few-shot selection (empty history, matching, threshold, max 3)
- ProjectMap analyzer (single/multi module, exports, params, malformed skip)
- Module summary formatting
- Modularizer (suggest_module, duplicate detect, module size)
- Session save/load roundtrip, resume detection, archive, multiple archives
- Session ID uniqueness, corrupted recovery, truncated file recovery
- UsageStats accumulation + persistence
- assemble_context determinism, session history inclusion, classification

---

## Kill Criterion (Phase 10)

Egy 3+ modulbol allo projektben a `duumbi add "..."`:
1. **Felismeri a meglevo modulokat** (T6.6 — nem dumpol mindent main-be)
2. **Kerdez, ha az intent nem egyertelmu** (T9.1 — MutationOutcome::NeedsClarification)
3. **Uj modult hoz letre ha kell** (T6.2 — CreateModule task)
4. **Workspace modulok automatikusan betoltodnek** (T7.2 — Program::load auto-discovery)
5. **Context assembly zero manualis config fajlt igenyel** (T6.6 — assemble_context)
6. **Session state megmarad CLI ujrainditaskor** (T8.5-T8.6 — persistent session)

**Mind a 6 pont teljesitese szukseges a Phase 10 PASS-hoz.**

---

## Cleanup

```bash
cd ~
rm -rf /tmp/duumbi-p10-test
unset DUUMBI
```

---

## Sign-off

| Ellenorzo | Datum | Eredmeny | Megjegyzes |
|-----------|-------|----------|------------|
| | | ☐ PASS / ☐ FAIL | |
