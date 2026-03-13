---
tags:
  - project/duumbi
  - milestone/phase-1
status: complete
github_milestone: "Phase 1: Usable CLI"
github_issues: "11/11 closed"
completed: 2026-02
---
# Phase 1 — Usable CLI ✅

> **Kill Criterion:** External developer installs and runs a non-trivial program (fibonacci) in < 10 minutes.
> **Eredmény:** ✅ Teljesítve (PR #25 merged)

← Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

A CLI termelési szintre emelése: minden alapvető parancs implementálva, a típusrendszer és op set bővítve, CI/CD beállítva. Külső fejlesztő < 10 perc alatt képes fibonacci programot futtatni.

## Főbb deliverable-ök

### CLI parancsok
- [x] `duumbi init` — workspace létrehozás
- [x] `duumbi build` — fordítás + linkelés
- [x] `duumbi run` — bináris futtatás
- [x] `duumbi check` — validáció (JSONL hibák)
- [x] `duumbi describe` — pszeudokód generálás

### Compiler bővítés
- [x] `f64` típus: `ConstF64`, `Add/Sub/Mul/Div` f64 variánsok
- [x] `bool` típus: i8 reprezentáció Cranelift-ben
- [x] `duumbi:Compare` → `icmp`/`fcmp`
- [x] `duumbi:Branch` → `brif` (feltételes branch)
- [x] `duumbi:Call` — függvényhívás argumentum átadással
- [x] `duumbi:Load` / `duumbi:Store` — változó kezelés
- [x] Multi-modul fordítás: több `.jsonld` → egyetlen binary
- [x] C runtime: `duumbi_print_f64`, `duumbi_print_bool`

### Infrastruktúra
- [x] GitHub Actions CI: build + test + lint
- [x] `README.md`: quickstart (< 5 perc)
- [x] `examples/fibonacci.jsonld`, `examples/hello.jsonld`

## GitHub

- Milestone: **Phase 1: Usable CLI** (#2)
- Issues: #14–#24 (11/11 lezárva)
- PR: #25 (merged)
