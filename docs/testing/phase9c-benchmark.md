# Phase 9C: Benchmark & Showcases — Manual Test Protocol

**Version:** 1.0
**Date:** 2026-03-18
**Branch:** `phase9c/benchmark-showcases`
**PR:** #329

---

## Prerequisites

- [ ] Rust toolchain installed (`rustup show` → stable)
- [ ] Project builds: `cargo build`
- [ ] Existing tests pass: `cargo test --all`
- [ ] At least one LLM provider configured in `.duumbi/config.toml`
  - Anthropic: `ANTHROPIC_API_KEY` env var set
  - OpenAI: `OPENAI_API_KEY` env var set
  - (Optional) Grok: `XAI_API_KEY`, OpenRouter: `OPENROUTER_API_KEY`

### Important: Binary Selection

**Do NOT use `cargo install`** — it places duumbi in `~/.cargo/bin/` globally, which can:
- Cache stale binaries from previous builds
- Interfere with development and later test runs
- Cause unexpected behavior from older versions

**Instead, use the locally built binary:**

```bash
# During testing, always use the local debug binary:
./target/debug/duumbi benchmark --help
./target/debug/duumbi benchmark --showcase calculator --attempts 1

# Optional: create a shell alias for convenience
alias duumbi-dev='./target/debug/duumbi'
duumbi-dev benchmark --help
```

**If you must install globally temporarily:**

```bash
# Before testing (not recommended, but if you must)
cargo install --path . --force

# ... run tests with: ./target/debug/duumbi benchmark ... ...

# After testing — IMPORTANT: clean up
cargo uninstall duumbi
```

### Config template

```toml
# .duumbi/config.toml — minimum 2 providers for kill criterion
[workspace]
name = "benchmark-test"

[[providers]]
provider = "Anthropic"
role = "Primary"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"

[[providers]]
provider = "openai"
role = "Fallback"
model = "gpt-4o"
api_key_env = "OPENAI_API_KEY"
```

---

## T1 — CLI Help & Argument Parsing

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 1.1 | `./target/debug/duumbi benchmark --help` | Megjelenik a help szöveg az összes opcióval (--showcase, --provider, --attempts, --output, --ci, --baseline) | | |
| 1.2 | `./target/debug/duumbi benchmark --attempts 0` | Elindul 0 attempt-tel, üres report JSON-t ad | | |
| 1.3 | `./target/debug/duumbi benchmark --showcase nonexistent --attempts 1` | Hiba: "no showcases match the given filter" | | |
| 1.4 | `./target/debug/duumbi benchmark --provider nonexistent --attempts 1` | Hiba: "no providers match the given filter" | | |
| 1.5 | `./target/debug/duumbi benchmark --showcase calculator,fibonacci --attempts 1` | Csak a 2 kiválasztott showcase fut | | |

---

## T2 — Single Showcase, Single Provider

> **Cél:** Ellenőrizni, hogy egy showcase végigmegy az intent pipeline-on.

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 2.1 | `../target/debug/duumbi init benchmark-test && cd benchmark-test` | Workspace létrejön | | |
| 2.2 | Config beállítása (1 provider) | `config.toml` tartalmazza a [[providers]] szekciót | | |
| 2.3 | `../target/debug/duumbi benchmark --showcase calculator --attempts 1` | Progress output stderr-en, JSON report stdout-on | | |
| 2.4 | Ellenőrizni: stderr tartalmaz `[1/1] calculator / ...` sort | Progress jelzés látható | | |
| 2.5 | Ellenőrizni: stdout valid JSON | `... \| jq .` sikeres parse | | |
| 2.6 | JSON report tartalmaz `showcases[0].name == "calculator"` | Showcase neve helyes | | |
| 2.7 | JSON report tartalmaz `results[0].duration_secs > 0` | Időmérés működik | | |
| 2.8 | Ha sikeres: `results[0].success == true`, `tests_passed == tests_total` | Teszt eredmények helyesek | | |
| 2.9 | Ha sikertelen: `error_category` kitöltve, `error_message` nem üres | Hibakategorizálás működik | | |

---

## T3 — Multiple Providers

> **Cél:** Ellenőrizni, hogy több provider-rel is fut, és a report külön tartja őket.

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 3.1 | Config: 2 provider (Anthropic + OpenAI) | Mindkét provider konfigurálva | | |
| 3.2 | `./target/debug/duumbi benchmark --showcase calculator --attempts 1` | 2 run (1 showcase × 2 provider × 1 attempt) | | |
| 3.3 | stderr: `[1/2]` és `[2/2]` progress sorok | Mindkét provider fut | | |
| 3.4 | JSON: `showcases[0].providers` tömb 2 elemű | Két provider stat elkülönül | | |
| 3.5 | JSON: mindkét provider neve megjelenik | Provider nevek helyesek | | |

---

## T4 — Multiple Attempts

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 4.1 | `./target/debug/duumbi benchmark --showcase fibonacci --attempts 3 --provider anthropic` | 3 run (1 × 1 × 3) | | |
| 4.2 | JSON: `results` tömb 3 elemű | 3 eredmény rögzítve | | |
| 4.3 | JSON: `results[*].attempt` értékek 1, 2, 3 | Attempt számozás helyes | | |
| 4.4 | JSON: `attempts_per_run == 3` | Config érték tükröződik | | |

---

## T5 — Output File

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 5.1 | `./target/debug/duumbi benchmark --showcase calculator --attempts 1 --output report.json` | Fájl létrejön | | |
| 5.2 | `cat report.json \| jq .kill_criterion_met` | Valid JSON, mező létezik | | |
| 5.3 | stderr: "Report written to report.json" üzenet | Visszajelzés megjelenik | | |
| 5.4 | stdout üres (nincs JSON a konzolra írva) | Csak fájlba írt | | |

---

## T6 — CI Mode

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 6.1 | `./target/debug/duumbi benchmark --ci --showcase calculator --attempts 1; echo "exit: $?"` | Exit code: 0 vagy 1 | | |
| 6.2 | Ha kill criterion nem teljesül (1 provider < 2): exit 1 | Nem-nulla exit code | | |
| 6.3 | `./target/debug/duumbi benchmark --ci --attempts 5` (nincs --attempts override) | Alapértelmezett 20-ra vált (ellenőrizni JSON `attempts_per_run`) | | |

---

## T7 — Summary Table & Error Breakdown

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 7.1 | Futtatás után a stderr-en megjelenik a táblázat | Unicode keret (╔═╗║╚═╝) és oszlopok: Showcase, Provider, Success, Rate | | |
| 7.2 | Kill criterion státusz megjelenik | "PASSED" vagy "NOT MET" szöveg | | |
| 7.3 | Ha voltak hibák: "Error breakdown:" szekció | Kategória → szám párok | | |

---

## T8 — Baseline Regression Detection

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 8.1 | Első futtatás: `duumbi benchmark --showcase calculator --attempts 3 --output baseline.json` | Baseline report mentve | | |
| 8.2 | Második futtatás: `duumbi benchmark --showcase calculator --attempts 3 --baseline baseline.json` | Összehasonlítás fut | | |
| 8.3 | Ha nincs regresszió: nincs "Regressions detected" üzenet | Csend = jó | | |
| 8.4 | Manuálisan módosítani baseline.json-ben egy success_rate-et 1.0-ra, majd újrafuttatni alacsonyabb eredménnyel | "⚠ Regressions detected:" megjelenik | | |
| 8.5 | `--baseline nonexistent.json` | Hiba: "failed to read baseline" | | |

---

## T9 — All 6 Showcases (Full Run)

> **Cél:** Teljes benchmark futtatás az összes showcase-zal. Ez a kill criterion validáció.
> **Figyelem:** Ez hosszú futás (6 × 2 × N attempt), API költséggel jár.

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 9.1 | `./target/debug/duumbi benchmark --attempts 3 --output full-report.json` | Mind a 6 showcase fut mind a 2 provider-rel | | |
| 9.2 | Progress: `[1/36]` ... `[36/36]` (6×2×3) | Összes run megjelenik | | |
| 9.3 | JSON: 6 showcase summary | Minden showcase-nak van statisztikája | | |
| 9.4 | Summary table: 12 sor (6 showcase × 2 provider) | Minden kombináció listázva | | |
| 9.5 | `kill_criterion_met` érték konzisztens a táblázattal | 5/6 × 2+ provider ≥ 95% → true | | |

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

| # | Lépés | Elvárt eredmény | ✓/✗ | Megjegyzés |
|---|-------|-----------------|-----|------------|
| 10.1 | Futtatás workspace nélkül (üres könyvtárból) | "Cannot run benchmarks: no .duumbi/config.toml found" | | |
| 10.2 | Futtatás üres [[providers]] konfiggal | "No LLM providers configured" | | |
| 10.3 | Érvénytelen API key-vel futtatás | Provider error kategória, nem crash | | |
| 10.4 | `ANTHROPIC_API_KEY="" ./target/debug/duumbi benchmark --showcase calculator --attempts 1` | Értelmes hibaüzenet | | |

---

## Automated Test Verification (reference)

A következő tesztek CI-ben futnak API key nélkül:

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

## Sign-off

| Ellenőrző | Dátum | Eredmény | Megjegyzés |
|-----------|-------|----------|------------|
| | | ☐ PASS / ☐ FAIL | |

**Kill criterion:** 5/6 showcase × 2+ LLM provider ≥ 95% success rate.
