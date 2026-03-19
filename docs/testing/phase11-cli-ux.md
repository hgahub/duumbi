# Phase 11: CLI UX & Developer Experience — Manual Test Protocol

**Version:** 1.0
**Date:** 2026-03-19
**Branch:** `phase11/cli-ux-developer-experience`
**PR:** #425

---

## Prerequisites

- [ ] Rust toolchain installed (`rustup show` → stable)
- [ ] Project builds: `cargo build` (a repo gyokereben)
- [ ] Existing tests pass: `cargo test --all` (1259 teszt zold)
- [ ] At least one LLM provider configured (T5, T7 szekciohoz)
  - Anthropic: `ANTHROPIC_API_KEY` env var set

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
mkdir -p /tmp/duumbi-p11-test
cd /tmp/duumbi-p11-test

# 3. Init
$DUUMBI init .

# 4. Konfiguralod a provider(eke)t
nano .duumbi/config.toml
```

**Config template:**

```toml
# /tmp/duumbi-p11-test/.duumbi/config.toml
[workspace]
name = "p11-test"

[[providers]]
provider = "Anthropic"
role = "Primary"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
```

**Takaritas a teszteles utan:**

```bash
cd ~
rm -rf /tmp/duumbi-p11-test
unset DUUMBI
```

---

## T1 — Post-Init Guidance (Track E)

> Futtatasi hely: **ures temp konyvtar** (NEM a mar init-elt workspace)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 1.1 | `mkdir /tmp/p11-init-test && cd /tmp/p11-init-test && $DUUMBI init .` | "✓ Project initialized at ..." megjelenik | | |
| 1.2 | Az init kimenet tartalmaz "Next steps:" szekciót | Lepesek listaja: API key, config.toml, REPL | | |
| 1.3 | Ha `ANTHROPIC_API_KEY` be van allitva: "Uncomment a [[providers]] section" az elso lepes | Felismeri a meglévo API key-t | | |
| 1.4 | Ha `ANTHROPIC_API_KEY` NINCS beallitva: "Set an API key: export ANTHROPIC_API_KEY=sk-..." az elso lepes | Utmutatast ad az API key beallitasahoz | | |
| 1.5 | Takaritas: `cd ~ && rm -rf /tmp/p11-init-test` | | | |

---

## T2 — Szines REPL Header & /status (Track A)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 2.1 | `$DUUMBI` (REPL indul) | Header sor megjelenik: "**duumbi** v0.1.1 · *model* · workspace: **p11-test**" | | |
| 2.2 | A "duumbi" szo **bold**, a verzio **dim**, a model **cyan** | Szinek lathatok terminálban | | |
| 2.3 | Header masodik sora: "/help" cyan+bold-dal kiemelve | Parancs kiemelés mukodik | | |
| 2.4 | REPL-ben: `/status` | Workspace sor **bold**, graph/binary utak **dim** | | |
| 2.5 | Graph ✓ **zold**, Binary "(not built)" **dim** | Szines statusz jelolok | | |
| 2.6 | Model nev **cyan**, provider **dim** zárójelben | | | |
| 2.7 | Ha nincs LLM konfigurálva: Model "not configured" **sarga** | Warning szin | | |
| 2.8 | REPL-ben: `/exit` | | | |

---

## T3 — Szines /help (Track A)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 3.1 | `$DUUMBI` → `/help` | Szekcio fejlecek **bold** ("Slash commands:", "Intent commands:", stb.) | | |
| 3.2 | Parancsnevek (`/build`, `/intent create`, stb.) **cyan+bold** | | | |
| 3.3 | Argumentumok (`[args]`, `<name>`, stb.) es leirasok **dim** | | | |
| 3.4 | `/clear` parancs megjelenik a listaban: `/clear [chat\|session\|all]` | Uj parancs lathato | | |
| 3.5 | Az utolso sor ("Any other input...") **dim** | | | |

---

## T4 — NO_COLOR Support (Track A)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 4.1 | `NO_COLOR=1 $DUUMBI build` | Kimenet nem tartalmaz ANSI escape kodokat (\\x1b[) | | |
| 4.2 | `NO_COLOR=1 $DUUMBI check` | Kimenet nem tartalmaz ANSI escape kodokat | | |
| 4.3 | `$DUUMBI build 2>&1 \| cat` | Ha pipe-on megy: ANSI kodok auto-strip (nem TTY) | | |

---

## T5 — Tab Completion: Slash Commands (Track B)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`** (REPL-ben)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 5.1 | `$DUUMBI` (REPL indul) | | | |
| 5.2 | Gepelj `/bui` es nyomj `TAB` | `/build` kiegeszul | | |
| 5.3 | Gepelj `/` es nyomj `TAB` | Osszes parancs listaja megjelenik (dropdown/lista) | | |
| 5.4 | Gepelj `/intent ` (szokozt is) es nyomj `TAB` | Alparancsok: create, review, execute, status | | |
| 5.5 | Gepelj `/deps ` es nyomj `TAB` | Alparancsok: list, audit, tree, update, vendor, install | | |
| 5.6 | `/exit` | | | |

---

## T6 — Tab Completion: Dynamic Arguments (Track B)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`** (REPL-ben)
> **Elofeltetel:** Letezik legalabb egy intent YAML a `.duumbi/intents/` alatt.

**Setup** (ha meg nincs intent):

```bash
# Kezzel letrehozunk egy dummy intent-et a teszthez
mkdir -p .duumbi/intents
cat > .duumbi/intents/test-calculator.yaml << 'EOF'
intent: "Test calculator"
version: 1
status: Pending
acceptance_criteria:
  - "add(a,b) returns a+b"
modules:
  create: []
  modify: ["app/main"]
test_cases: []
EOF
```

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 6.1 | `$DUUMBI` → `/intent execute ` + `TAB` | "test-calculator" megjelenik javaslatként | | |
| 6.2 | `/intent review ` + `TAB` | "test-calculator" megjelenik javaslatként | | |
| 6.3 | `/intent status ` + `TAB` | "test-calculator" megjelenik javaslatként | | |
| 6.4 | `/exit` | | | |

---

## T7 — Ghost-Text Hinter (Track B)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`** (REPL-ben)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 7.1 | `$DUUMBI` → gepelj `/bui` (NE nyomj TAB-ot) | Halvany (dim/szurke) "ld" ghost-text megjelenik a cursor utan | | |
| 7.2 | Gepelj `/he` | Halvany "lp" ghost-text jelenik meg | | |
| 7.3 | Gepelj `/intent` (teljes parancs) | Nincs ghost-text (exact match) | | |
| 7.4 | Gepelj `hello` (nem slash parancs) | Nincs ghost-text | | |
| 7.5 | `/exit` | | | |

---

## T8 — Shell Completions CLI (Track B)

> Futtatasi hely: **repo gyokere** (workspace nem szukseges)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 8.1 | `$DUUMBI completions bash` | Bash completion script stdout-ra (tartalmaz `_duumbi` fuggvenyt) | | |
| 8.2 | `$DUUMBI completions zsh` | Zsh completion script stdout-ra (tartalmaz `#compdef`) | | |
| 8.3 | `$DUUMBI completions fish` | Fish completion script stdout-ra (tartalmaz `complete -c duumbi`) | | |
| 8.4 | `$DUUMBI completions powershell` | PowerShell completion script stdout-ra | | |
| 8.5 | `$DUUMBI completions --help` | Shell opcio dokumentalva: bash, zsh, fish, powershell | | |

---

## T9 — Spinner: AI Mutation (Track C)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`** (LLM provider konfiggal)
> **Megjegyzes:** Igazi LLM API hivast vegez.

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 9.1 | `$DUUMBI` (REPL) → "add a hello function that returns 42" | Animalt spinner jelenik meg "Thinking... (~X.Xk context)" szoveggel | | |
| 9.2 | A spinner eltünik MIELOTT a streaming szoveg megjelenik | Nincs interleave (spinner es text nem keveredik) | | |
| 9.3 | CLI: `$DUUMBI add "add a greet function that returns 1" -y` | Spinner jelenik meg "Calling <provider>..." szoveggel | | |
| 9.4 | A spinner eltünik a streaming szoveg elott | Nincs interleave | | |

---

## T10 — Szines Build/Check/Diagnostic (Track A)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 10.1 | `$DUUMBI build` | "✓ Build successful: ..." — a ✓ **zold+bold** | | |
| 10.2 | `$DUUMBI check` | "✓ Validation passed." — a ✓ **zold+bold** | | |
| 10.3 | Hozz letre szandekosan hibas graph-ot es futtasd `$DUUMBI check` | Error kod (pl. E001) **piros+bold**, node ID (duumbi:...) **kek** | | |

---

## T11 — Tablazatos Output (Track D)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`**

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 11.1 | `$DUUMBI deps list` | Tablazat: Name, Version/Path, Source, Status oszlopok, UTF-8 kerettel | | |
| 11.2 | `$DUUMBI` → `/deps list` | Ugyanaz a tablazat a REPL-ben | | |
| 11.3 | `$DUUMBI` → `/knowledge list` | Tablazat: Type, ID oszlopok (vagy "No knowledge nodes found.") | | |
| 11.4 | `$DUUMBI knowledge list` | Ugyanaz CLI-bol | | |

---

## T12 — "Did You Mean?" Javaslatok (Track E)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`** (REPL-ben)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 12.1 | `/buidl` (elgepeles) | "Unknown command: /buidl. Did you mean /build?" | | |
| 12.2 | `/stauts` | "Unknown command: /stauts. Did you mean /status?" | | |
| 12.3 | `/chekc` | "Unknown command: /chekc. Did you mean /check?" | | |
| 12.4 | `/xyzabc` (teljesen ismeretlen) | "Unknown command: /xyzabc" (nincs "Did you mean?" — tul tavol) | | |
| 12.5 | A "Did you mean?" javaslatban a parancs **cyan+bold** | Szines kiemelés | | |
| 12.6 | "Try /help" uzenet megjelenik | | | |

---

## T13 — Ures Workspace Detekció (Track E)

> Futtatasi hely: **friss workspace** (a skeleton main.jsonld-vel)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 13.1 | `mkdir /tmp/p11-empty && cd /tmp/p11-empty && $DUUMBI init . && $DUUMBI` | REPL indul | | |
| 13.2 | A header utan megjelenik: "Tip: This is an empty workspace. Try one of these:" | Guided javaslatokat ad | | |
| 13.3 | Javaslatok kozott: `/intent create` **cyan+bold** | Parancs kiemelés | | |
| 13.4 | Es: "or type a request directly" **dim** | | | |
| 13.5 | Ha a workspace mar tartalmaz Add/Call op-ot: a tip NEM jelenik meg | Csak ures workspace-nel aktiv | | |
| 13.6 | Takaritas: `cd ~ && rm -rf /tmp/p11-empty` | | | |

---

## T14 — "No LLM Configured" Javitott Uzenet (Track E)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`** (LLM provider NELKUL)

**Setup:** Kommenteld ki az osszes `[[providers]]` szekciót a config.toml-ban.

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 14.1 | `$DUUMBI` → "add something" | "Warning: AI mutations are not available." **sarga** | | |
| 14.2 | Utana inline config pelda jelenik meg: `[[providers]]`, `provider = "anthropic"`, stb. | **dim** szinnel | | |
| 14.3 | "Then set ANTHROPIC_API_KEY and restart the REPL." uzenet | **cyan** kiemelés az env var neven | | |

---

## T15 — /clear Parancs (Track E)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`** (REPL-ben)

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 15.1 | `$DUUMBI` → `/history` | "No session history yet." (vagy korábbi history) | | |
| 15.2 | `/clear` (argumentum nelkul) | "✓ Chat history cleared." | | |
| 15.3 | `/history` | "No session history yet." (torolt) | | |
| 15.4 | `/clear chat` | "✓ Chat history cleared." | | |
| 15.5 | `/clear session` | "✓ Session archived and cleared." | | |
| 15.6 | `/clear all` | "✓ History, session cleared." | | |
| 15.7 | `/clear invalid` | "Unknown clear target: invalid. Use: /clear chat, /clear session, or /clear all" | | |
| 15.8 | `/exit` | | | |

---

## T16 — Intent Task Progress Szinek (Track A)

> Futtatasi hely: **`/tmp/duumbi-p11-test/`** (LLM provider konfiggal)
> **Megjegyzes:** Igazi LLM API hivast vegez.

| # | Lepes | Elvart eredmeny | ✓/✗ | Megjegyzes |
|---|-------|-----------------|-----|------------|
| 16.1 | `$DUUMBI intent create "Create a double function that takes i64 n and returns n*2, main calls double(21)" -y` | Intent letrejott | | |
| 16.2 | `$DUUMBI intent execute <slug>` | Task progress: "✓" **zold+bold** sikeres task-oknal | | |
| 16.3 | Ha egy task sikertelen: "✗" **piros+bold** | Hiba jeloles szines | | |

---

## Automated Test Verification (reference)

A kovetkezo tesztek API key nelkul futnak CI-ben (repo gyokereben):

```bash
cargo test cli::theme::         # 2 unit test (color functions, unicode chars)
cargo test cli::completion::    # 7 unit test (prefix match, dynamic, hinter)
cargo test cli::progress::      # 3 unit test (spinner, progress_bar, multi)
```

Osszesen **12 uj automatizalt teszt** fedi le:
- theme.rs: szin fuggvenyek nem-ures string-et adnak, check/cross unicode karakter
- completion.rs: prefix matching, no-slash skip, intent subcommand, dynamic slug, hinter suffix/exact/space
- progress.rs: spinner create+finish, progress_bar tracking, multi_progress add

Teljes teszt szam: **1259 teszt zold** (korabbi 1247 + 12 uj).

---

## Kill Criterion (Phase 11)

**3/3 uj user sikeresen futtat intent-et a `/` menuvel 10 percen belul, dokumentacio olvasasa nelkul.**

Ellenorizendo:
1. [ ] REPL indul → ures workspace tip megjelenik
2. [ ] `/` + TAB → parancs lista lathato, valasztható
3. [ ] `/intent create "..."` → intent letrejön
4. [ ] `/intent execute <TAB>` → slug kiegeszul
5. [ ] Intent execute → szines progress (✓/✗)
6. [ ] Teljes folyamat < 10 perc (elso hasznalat, dokumentacio nelkul)

---

## Cleanup

```bash
cd ~
rm -rf /tmp/duumbi-p11-test
rm -rf /tmp/p11-init-test
rm -rf /tmp/p11-empty
unset DUUMBI
```

---

## Sign-off

| Ellenorzo | Datum | Eredmeny | Megjegyzes |
|-----------|-------|----------|------------|
| | | ☐ PASS / ☐ FAIL | |
