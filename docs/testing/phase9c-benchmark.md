# Phase 9C: Benchmark & Showcases — Manual Test Protocol

**Version:** 1.1
**Date:** 2026-03-18
**Branch:** `phase9c/benchmark-showcases`
**PR:** #329

---

## Prerequisites

- [ ] Rust toolchain installed (`rustup show` → stable)
- [ ] Project builds: `cargo build` (a repo gyökerében)
- [ ] Existing tests pass: `cargo test --all`
- [ ] At least one LLM provider configured
  - Anthropic: `ANTHROPIC_API_KEY` env var set
  - OpenAI: `OPENAI_API_KEY` env var set
  - (Optional) Grok: `XAI_API_KEY`, OpenRouter: `OPENROUTER_API_KEY`

### Binary: ne használj `cargo install`

**Ne futtasd `cargo install --path .`** — az a binárist globálisan telepíti
(`~/.cargo/bin/duumbi`), ami zavarhatja a fejlesztést (régi cached verzió
fut, PATH ütközés). Mindig a frissen fordított lokális binárist használd.

---

## Test Workspace Setup

**A tesztelést ne a repo gyökerében végezd.** A `duumbi init` egy `.duumbi/`
mappát hoz létre, amelynek egyes fájljai (`config.toml`, `graph/`, `schema/`)
**nincsenek gitignore-ban** (user fájlok), ezért a repo gyökerében való
futtatás szennyes `git status`-t okozna.

**Egy parancs a teljes tesztkörnyezet felállítására** (a repo gyökerében futtasd):

```bash
# 1. Exportáld a bináris elérési útját (egyszer, terminál session-önként)
export DUUMBI="$(pwd)/target/debug/duumbi"

# 2. Hozz létre test workspace-t a repón KÍVÜL
mkdir -p /tmp/duumbi-test
cd /tmp/duumbi-test

# 3. Init
$DUUMBI init .

# 4. Konfiguráld a provider(eke)t (lásd template lent)
code .duumbi/config.toml
```

**Takarítás a tesztelés után:**

```bash
cd ~
rm -rf /tmp/duumbi-test
```

### Config template

```toml
# /tmp/duumbi-test/.duumbi/config.toml
# Minimum 2 provider a kill criterion-hoz (T3, T9)
[workspace]
name = "benchmark-test"

[[providers]]
provider = "Anthropic"
role = "Primary"
api_key_env = "ANTHROPIC_API_KEY"

[[providers]]
provider = "openai"
role = "Fallback"
api_key_env = "OPENAI_API_KEY"
```

> **Megjegyzés:** T1 és T10.1 az egyetlen szekció, ahol a `DUUMBI` változót
> a **repo gyökeréből** futtatod (workspace nem szükséges). Az összes többi
> tesztet `/tmp/duumbi-test/`-ből futtasd.

---

## T1 — CLI Help & Argument Parsing

> Futtatás helye: **repo gyökere** (workspace nem szükséges)

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 1.1 | `$DUUMBI benchmark --help` | Help szöveg megjelenik az összes opcióval (--showcase, --provider, --attempts, --output, --ci, --baseline) | | |
| 1.2 | `$DUUMBI benchmark --attempts 0` | Hiba: no .duumbi/config.toml (nincs workspace) | | |
| 1.3 | (workspace-ből) `$DUUMBI benchmark --showcase nonexistent --attempts 1` | Hiba: "no showcases match the given filter" | | |
| 1.4 | (workspace-ből) `$DUUMBI benchmark --provider nonexistent --attempts 1` | Hiba: "no providers match the given filter" | | |
| 1.5 | (workspace-ből) `$DUUMBI benchmark --showcase calculator,fibonacci --attempts 1` | Csak a 2 kiválasztott showcase fut | | |

---

## T2 — Single Showcase, Single Provider

> Futtatás helye: **`/tmp/duumbi-test/`** (1 provider konfiggal)

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 2.1 | `cat .duumbi/config.toml` | Config létezik, 1 `[[providers]]` blokk | | |
| 2.2 | `$DUUMBI benchmark --showcase calculator --attempts 1` | Progress stderr-en, JSON report stdout-on | | |
| 2.3 | stderr tartalmaz `[1/1] calculator / ...` sort | Progress jelzés látható | | |
| 2.4 | `$DUUMBI benchmark --showcase calculator --attempts 1 \| jq .` | Valid JSON, nincs parse hiba | | |
| 2.5 | JSON: `showcases[0].name == "calculator"` | Showcase neve helyes | | |
| 2.6 | JSON: `results[0].duration_secs > 0` | Időmérés működik | | |
| 2.7 | Ha sikeres: `results[0].success == true` és `tests_passed == tests_total` | Teszt eredmények helyesek | | |
| 2.8 | Ha sikertelen: `error_category` kitöltve, `error_message` nem üres | Hibakategorizálás működik | | |

---

## T3 — Multiple Providers

> Futtatás helye: **`/tmp/duumbi-test/`** (2 provider konfiggal — lásd config template)

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 3.1 | `cat .duumbi/config.toml` | 2 `[[providers]]` blokk látható | | |
| 3.2 | `$DUUMBI benchmark --showcase calculator --attempts 1` | 2 run (1 showcase × 2 provider × 1 attempt) | | |
| 3.3 | stderr: `[1/2]` és `[2/2]` progress sorok | Mindkét provider fut | | |
| 3.4 | JSON: `showcases[0].providers` tömb 2 elemű | Két provider stat elkülönül | | |
| 3.5 | JSON: mindkét provider neve megjelenik | Provider nevek helyesek | | |

---

## T4 — Multiple Attempts

> Futtatás helye: **`/tmp/duumbi-test/`**

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 4.1 | `$DUUMBI benchmark --showcase fibonacci --attempts 3 --provider anthropic` | 3 run (1 showcase × 1 provider × 3 attempt) | | |
| 4.2 | JSON: `results` tömb 3 elemű | 3 eredmény rögzítve | | |
| 4.3 | JSON: `results[*].attempt` értékek 1, 2, 3 | Attempt számozás helyes | | |
| 4.4 | JSON: `attempts_per_run == 3` | Config érték tükröződik | | |

---

## T5 — Output File

> Futtatás helye: **`/tmp/duumbi-test/`**

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 5.1 | `$DUUMBI benchmark --showcase calculator --attempts 1 --output report.json` | Fájl létrejön: `/tmp/duumbi-test/report.json` | | |
| 5.2 | `cat report.json \| jq .kill_criterion_met` | Valid JSON, mező létezik | | |
| 5.3 | stderr: "Report written to report.json" üzenet | Visszajelzés megjelenik | | |
| 5.4 | stdout üres (nincs JSON a konzolra írva) | Csak fájlba írt | | |

---

## T6 — CI Mode

> Futtatás helye: **`/tmp/duumbi-test/`**

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 6.1 | `$DUUMBI benchmark --ci --showcase calculator --attempts 1; echo "exit: $?"` | Exit code: 0 (pass) vagy 1 (fail) | | |
| 6.2 | 1 provider konfiggal: `$DUUMBI benchmark --ci --showcase calculator --attempts 1; echo "exit: $?"` | Exit 1 (1 provider < 2, kill criterion nem teljesül) | | |
| 6.3 | `$DUUMBI benchmark --ci --showcase calculator --attempts 5 \| jq .attempts_per_run` | 20-at ad vissza (CI default override) | | |

---

## T7 — Summary Table & Error Breakdown

> Futtatás helye: **`/tmp/duumbi-test/`**

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 7.1 | Bármely sikeres futtatás után a stderr-en megjelenik a táblázat | Unicode keret (╔═╗║╚═╝), oszlopok: Showcase, Provider, Success, Rate | | |
| 7.2 | Kill criterion státusz megjelenik | "PASSED" vagy "NOT MET" szöveg | | |
| 7.3 | Ha voltak hibák: "Error breakdown:" szekció a stderr-en | Kategória → szám párok (pl. `logic_error: 2`) | | |

---

## T8 — Baseline Regression Detection

> Futtatás helye: **`/tmp/duumbi-test/`**

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 8.1 | `$DUUMBI benchmark --showcase calculator --attempts 3 --output baseline.json` | Baseline report mentve | | |
| 8.2 | `$DUUMBI benchmark --showcase calculator --attempts 3 --baseline baseline.json` | Összehasonlítás fut, nincs regresszió | | |
| 8.3 | Ha nincs regresszió: nincs "Regressions detected" üzenet a stderr-en | Csend = jó | | |
| 8.4 | `jq '.showcases[0].providers[0].success_rate = 1.0' baseline.json > b2.json && $DUUMBI benchmark --showcase calculator --attempts 3 --baseline b2.json` | "⚠ Regressions detected:" megjelenik (ha az aktuális rate < 0.95) | | |
| 8.5 | `$DUUMBI benchmark --showcase calculator --attempts 1 --baseline nonexistent.json` | Hiba: "failed to read baseline" | | |

---

## T9 — All 6 Showcases (Full Run)

> Futtatás helye: **`/tmp/duumbi-test/`** (2 provider konfiggal)
> **Figyelem:** Hosszú futás (6 × 2 × N attempt), API költséggel jár. Futtasd utoljára.

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 9.1 | `$DUUMBI benchmark --attempts 3 --output full-report.json` | Mind a 6 showcase fut mind a 2 provider-rel | | |
| 9.2 | Progress: `[1/36]` ... `[36/36]` (6×2×3) | Összes run megjelenik | | |
| 9.3 | JSON: `showcases` tömb 6 elemű | Minden showcase-nak van statisztikája | | |
| 9.4 | Summary table: 12 sor (6 showcase × 2 provider) | Minden kombináció listázva | | |
| 9.5 | `cat full-report.json \| jq .kill_criterion_met` | `true`, ha 5/6 showcase ≥ 95% × 2 provider | | |

### Showcase-specifikus ellenőrzés

| Showcase | Teszt esetek | Elvárt kimenet | ✓/✗ |
|----------|-------------|----------------|-----|
| calculator | add(3,5)=8, sub(10,4)=6, mul(7,6)=42, div(20,4)=5 | 4/4 pass | |
| fibonacci | fib(0)=0, fib(1)=1, fib(10)=55 | 3/3 pass | |
| sorting | sort_and_get(3,1,2,0)=1, sort_and_get(3,1,2,2)=3 | 2/2 pass | |
| state_machine | transition(0,1)=1, (1,2)=2, (2,3)=1, (1,4)=3 | 4/4 pass | |
| multi_module | double(7)=14, square(5)=25 | 2/2 pass | |
| string_ops | length_of_hello()=5, hello_contains_ell()=1 | 2/2 pass | |

---

## T10 — Edge Cases & Error Handling

| # | Lépés | Futtatás helye | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|----------------|-----------------|-----|------------|
| 10.1 | `$DUUMBI benchmark --showcase calculator --attempts 1` | Üres könyvtár (pl. `/tmp`) | "Cannot run benchmarks: no .duumbi/config.toml found" | | |
| 10.2 | Config: üres `[[providers]]` nélkül, majd `$DUUMBI benchmark --attempts 1` | `/tmp/duumbi-test/` | "No LLM providers configured" | | |
| 10.3 | `ANTHROPIC_API_KEY=invalid $DUUMBI benchmark --showcase calculator --attempts 1` | `/tmp/duumbi-test/` | Provider error kategória, nem crash | | |
| 10.4 | `ANTHROPIC_API_KEY="" $DUUMBI benchmark --showcase calculator --attempts 1` | `/tmp/duumbi-test/` | Értelmes hibaüzenet az API key hiányáról | | |

---

## Automated Test Verification (reference)

A következő tesztek API key nélkül futnak CI-ben (repo gyökerében):

```bash
cargo test bench::              # 14 unit test (showcases, report)
cargo test integration_phase9c  # 26 integration test
```

Összesen 40 automatizált teszt fedi le:
- ✅ Mind a 6 YAML parse-olható valid IntentSpec-be
- ✅ Showcase szűrés (filter_showcases)
- ✅ Report aggregáció (per-showcase, per-provider)
- ✅ Kill criterion: true (5/6 × 2 provider), false (4/6 vagy 1 provider)
- ✅ Error kategorizálás (6 kategória × több minta)
- ✅ JSON szerializáció/deszializáció roundtrip
- ✅ Regresszió detektálás (threshold felett/alatt)
- ✅ ErrorCategory serde snake_case

---

## Cleanup

```bash
cd ~
rm -rf /tmp/duumbi-test
# Ha $DUUMBI alias aktív:
unset DUUMBI
```

---

## Sign-off

| Ellenőrző | Dátum | Eredmény | Megjegyzés |
|-----------|-------|----------|------------|
| | | ☐ PASS / ☐ FAIL | |

**Kill criterion:** 5/6 showcase × 2+ LLM provider ≥ 95% success rate.
