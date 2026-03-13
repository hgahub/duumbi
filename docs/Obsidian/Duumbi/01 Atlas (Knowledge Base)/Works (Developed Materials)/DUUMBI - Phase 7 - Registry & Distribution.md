---
tags:
  - project/duumbi
  - milestone/phase-7
status: in-progress
github_milestone: "Phase 7: Registry & Distribution"
github_issues: "35/37 closed"
updated: 2026-03-13
---
# Phase 7 — Registry & Distribution 🔄

> **Kill Criterion:** Determinisztikus hash + `duumbi publish` + `duumbi deps install` működik publikus modulra. `duumbi deps vendor --all` + offline build működik. Lockfile v1 determinisztikus.
> **Állapot:** 🔄 35/37 issue lezárva — 2 open infra issue (registry CI/CD + monitoring)

← Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

Client-side registry & distribution: háromrétegű gráf tároló (workspace/vendor/cache), scope-alapú névtér, modul csomagolás (.tar.gz), SemVer feloldás, lockfile v1 integrity hash-sel, privát registry támogatás.

> **Architektúra:** A registry szerver külön repo (`hgahub/duumbi-registry`), az infrastruktúra külön repo (`hgahub/duumbi-infra`). Ez a repo csak a **client oldalt** tartalmazza.

---

## Teljesített feladatok ✅

### Alap infrastruktúra
- [x] **M7-HASH**: Semantic hash (`src/hash.rs`) — SHA-256, @id-független
- [x] **M7-LOCK-01**: Lockfile v1 TOML formátum (name, version, source, semantic_hash, integrity, resolved_path, vendored)
- [x] **M7-LOCK-02**: `generate_lockfile_v1()` — determinisztikus
- [x] **M7-AUTH-01**: Credentials storage (`~/.duumbi/credentials.toml`, 0600 perms)

### Storage rétegek
- [x] **M7-STORE-01**: Workspace layer (`.duumbi/graph/`)
- [x] **M7-STORE-02**: Vendor layer (`.duumbi/vendor/`)
- [x] **M7-STORE-03**: Cache layer (`.duumbi/cache/`)
- [x] **M7-STORE-04**: `duumbi upgrade` migrációs parancs
- [x] **M7-STORE-05**: `duumbi init` M7 alapértelmezések

### Autentikáció és Registry
- [x] **M7-AUTH-02**: `duumbi registry add/list/remove` parancsok
- [x] **M7-AUTH-03**: `duumbi registry login` — token tárolás
- [x] **M7-AUTH-04**: Scope-level registry routing (`@scope/name` → registry named `scope`)
- [x] **M7-CLIENT-01**: `RegistryClient` HTTP modul (reqwest + retry)

### Publikálás
- [x] **M7-PUB-01**: Module packaging — `.tar.gz` + `manifest.toml` + `CHECKSUM`
- [x] **M7-PUB-02**: `duumbi publish` — validate → pack → hash → upload
- [x] **M7-PUB-03**: `duumbi yank @scope/name@version`

### Dependency management CLI
- [x] **M7-CLIENT-02**: `duumbi deps add` registry-aware bővítés
- [x] **M7-CLIENT-03**: `duumbi deps install --frozen`
- [x] **M7-CLIENT-04**: `duumbi search <query>`
- [x] **M7-CLIENT-05**: `duumbi deps tree`
- [x] **M7-CLIENT-06**: `duumbi deps audit`
- [x] **M7-CLIENT-07**: `duumbi deps update`

### Registry szerver (külön repo)
- [x] **M7-SERVER-01**: `duumbi-registry` repo scaffold
- [x] **M7-SERVER-02**: Adatbázis schema + modul tárolás (SQLite)
- [x] **M7-SERVER-03**: API endpoints (publish, download, search, yank)
- [x] **M7-SERVER-04**: Privát registry deployment (Docker + self-hosted docs)

### Web + Tesztek
- [x] **M7-WEB-01**: Registry web frontend (Leptos)
- [x] **M7-WEB-02**: Studio sidebar registry search integráció
- [x] **M7-TEST-01–06**: Unit + integration tesztek + kill criterion validáció + docs
- [x] **M7-REPL**: REPL slash commandok registry-hez

---

## Nyitott feladatok 🔄

| Issue | Cím | Megjegyzés |
|-------|-----|------------|
| #162 | M7-INFRA-02: CI/CD pipeline for duumbi-registry | `hgahub/duumbi-infra` repo |
| #163 | M7-INFRA-03: Monitoring and observability | Azure Monitor + Application Insights |

> Ezek a registry **szerver** infrastruktúráját érintik (külön repo), a client oldal kész.

---

## config.toml v2 formátum

```toml
[workspace]
name = "myapp"
namespace = "myapp"
default-registry = "duumbi"

[registries]
duumbi = "https://registry.duumbi.dev"

[dependencies]
"@duumbi/stdlib-math" = "^1.0"

[vendor]
strategy = "selective"
include = ["@company/*"]
```

## Lockfile v1

```toml
version = 1

[[dependencies]]
name = "@duumbi/stdlib-math"
version = "1.0.0"
source = "cache"
semantic_hash = "abc..."
integrity = "sha256-def..."
resolved_path = ".duumbi/cache/@duumbi/stdlib-math@1.0.0/graph"
vendored = false
```

---

## GitHub

- Milestone: **Phase 7: Registry & Distribution** (#8)
- Issues: #154–#190 (35/37 lezárva)
- Branch: `phase7/registry-distribution`
- Kapcsolódó repók: `hgahub/duumbi-registry`, `hgahub/duumbi-infra`

## Főbb fájlok

```
src/registry/
├── client.rs    — RegistryClient HTTP (reqwest + retry)
├── credentials.rs — ~/.duumbi/credentials.toml
├── packaging.rs — .tar.gz + CHECKSUM
└── search.rs    — szöveges keresés
src/hash.rs      — semantic hash (SHA-256)
src/config.rs    — Config v2 (workspace, registries, deps, vendor)
src/deps.rs      — lockfile v1, resolution pipeline
```
