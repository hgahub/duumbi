---
tags:
  - project/duumbi
  - doc/architecture
  - doc/planning
status: draft
created: 2026-03-06
updated: 2026-03-06
related_maps:
  - "[[DUUMBI - Post-MVP Implementation Roadmap]]"
  - "[[DUUMBI - Post-MVP Roadmap]]"
  - "[[DUUMBI - PRD]]"
  - "[[DUUMBI - Architecture Diagram]]"
  - "[[DUUMBI - Glossary]]"
---

# DUUMBI — Graph Repository Architecture

> [!important] Ez a dokumentum a szemantikus gráf modulok **tárolási, feloldási és névtér-kezelési architektúráját** definiálja. A [[DUUMBI - Post-MVP Implementation Roadmap]] M4 (befejezett) és M7 (Registry) mérföldköveinek továbbfejlesztése.

---

## 1. Problémafelvetés

A jelenlegi (M4-es) állapot korlátai:

| Probléma | Jelenlegi állapot | Cél állapot |
|----------|------------------|-------------|
| Gráf fájlok elhelyezése | `.duumbi/graph/` + `.duumbi/stdlib/` (szétszórt) | Egységes, rétegzett könyvtárstruktúra |
| Távoli modulok | Nincs (csak lokális path) | Registry + privát registry támogatás |
| Vendored modulok | Nincs | `.duumbi/vendor/` — verziókezelt, VCS-be commitolható |
| Névtér ütközés | "first-seen wins" (implicit) | Scope-alapú névtér (`@scope/module`) |
| Modul eredet | Nem nyomon követhető | Lockfile v1: forrás URL + hash + integritás |
| Vállalati zárt logikák | Nincs támogatás | Privát registry, vendor mód, air-gapped deploy |

---

## 2. Infografika: Háromrétegű Gráf Tároló Architektúra

![[DUUMBI - Graph Repository Architecture.excalidraw|Háromrétegű Gráf Tároló Architektúra]]

```
╔══════════════════════════════════════════════════════════════════════════╗
║                    DUUMBI GRAPH REPOSITORY ARCHITECTURE                 ║
║              "Három réteg, egy gráf, nulla ütközés"                     ║
╚══════════════════════════════════════════════════════════════════════════╝

                         ┌─────────────────────┐
                         │   REGISTRY LAYER     │
                         │  (Távoli forrás)      │
                         └──────┬──────────┬────┘
                                │          │
          ┌─────────────────────┤          ├─────────────────────┐
          │                     │          │                     │
          ▼                     ▼          ▼                     ▼
  ┌───────────────┐   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
  │  registry.    │   │  registry.   │  │  git repo    │  │  lokális     │
  │  duumbi.dev   │   │  company.com │  │  (git+https) │  │  path        │
  │               │   │              │  │              │  │  (../libs/)  │
  │  🌐 Publikus  │   │  🔒 Privát   │  │  📦 Git dep  │  │  📁 Path dep │
  └───────┬───────┘   └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
          │                  │                 │                  │
          └──────────────────┴────────┬────────┴──────────────────┘
                                      │
                              `duumbi deps install`
                                      │
                                      ▼
  ╔═══════════════════════════════════════════════════════════════════╗
  ║                        LOCAL CACHE LAYER                         ║
  ║                 .duumbi/cache/  (NEM verziókezelt)                ║
  ║                                                                   ║
  ║  Automatikusan letöltött modulok — mint node_modules             ║
  ║  .gitignore-ban van — bármikor újra letölthető                   ║
  ║                                                                   ║
  ║  .duumbi/cache/                                                   ║
  ║  ├── @duumbi/                                                     ║
  ║  │   ├── stdlib-math@1.0.0/                                       ║
  ║  │   │   ├── graph/math.jsonld                                    ║
  ║  │   │   └── manifest.toml                                        ║
  ║  │   └── stdlib-io@1.0.0/                                         ║
  ║  │       ├── graph/io.jsonld                                      ║
  ║  │       └── manifest.toml                                        ║
  ║  ├── @community/                                                  ║
  ║  │   └── sorting@2.1.0/                                           ║
  ║  │       ├── graph/sort.jsonld                                    ║
  ║  │       └── manifest.toml                                        ║
  ║  └── @company/                                                    ║
  ║      └── auth-core@3.0.1/                                         ║
  ║          ├── graph/auth.jsonld                                    ║
  ║          └── manifest.toml                                        ║
  ╚═══════════════════════════════════════════════════════════════════╝
                                      │
                                      │  ha vendored:
                                      │  `duumbi deps vendor`
                                      │
                                      ▼
  ╔═══════════════════════════════════════════════════════════════════╗
  ║                        VENDOR LAYER                              ║
  ║              .duumbi/vendor/  (Verziókezelt, VCS-ben)            ║
  ║                                                                   ║
  ║  Kézzel bemásolt VAGY `duumbi deps vendor` által rögzített       ║
  ║  Garantált offline build — air-gapped deploy, audit              ║
  ║                                                                   ║
  ║  .duumbi/vendor/                                                  ║
  ║  ├── @company/                                                    ║
  ║  │   └── auth-core/                                               ║
  ║  │       ├── graph/auth.jsonld                                    ║
  ║  │       └── manifest.toml                                        ║
  ║  └── @internal/                                                   ║
  ║      └── billing-rules/                                           ║
  ║          ├── graph/billing.jsonld                                 ║
  ║          └── manifest.toml                                        ║
  ╚═══════════════════════════════════════════════════════════════════╝
                                      │
                                      │
                                      ▼
  ╔═══════════════════════════════════════════════════════════════════╗
  ║                      WORKSPACE LAYER                             ║
  ║              .duumbi/graph/  (Saját fejlesztés)                  ║
  ║                                                                   ║
  ║  Az alkalmazás saját gráf moduljai — itt dolgozik a fejlesztő    ║
  ║  és az AI. Ez a legmagasabb prioritású réteg.                    ║
  ║                                                                   ║
  ║  .duumbi/graph/                                                   ║
  ║  ├── main.jsonld              # Fő belépési pont                  ║
  ║  ├── api/                                                         ║
  ║  │   ├── handlers.jsonld      # API kezelők                       ║
  ║  │   └── routes.jsonld        # Útvonalak                        ║
  ║  └── domain/                                                      ║
  ║      ├── user.jsonld          # Felhasználó logika                ║
  ║      └── payment.jsonld       # Fizetés logika                   ║
  ╚═══════════════════════════════════════════════════════════════════╝

                        ┌─────────────────────┐
                        │   FELOLDÁSI SORREND  │
                        │   (Resolution Order) │
                        └─────────────────────┘

                    1. WORKSPACE   (.duumbi/graph/)
                         │ ← "Saját kód mindig nyer"
                         ▼
                    2. VENDOR      (.duumbi/vendor/)
                         │ ← "Rögzített, auditált verziók"
                         ▼
                    3. CACHE       (.duumbi/cache/)
                         │ ← "Registry-ből letöltött"
                         ▼
                    4. REGISTRY    (registry.duumbi.dev)
                         │ ← "Távoli forrás, ha nincs lokálisan"
                         ▼
                    ❌ NOT FOUND → Error E011
```

---

## 3. Névtér Rendszer (Namespace Scoping)

### 3.1 Scope Konvenció

```
@<scope>/<module-name>
```

| Scope | Példa | Leírás | Hozzáférés |
|-------|-------|--------|------------|
| `@duumbi` | `@duumbi/stdlib-math` | Hivatalos DUUMBI modulok | Publikus |
| `@<user>` | `@gabor/string-utils` | Egyéni fejlesztő publikált moduljai | Publikus / Privát |
| `@<org>` | `@acme/billing-core` | Szervezeti modulok | Privát (org registry) |
| (nincs scope) | `myapp/handlers` | Lokális workspace modulok | Csak helyi |

### 3.2 JSON-LD Névtér Mapping

```jsonld
{
  "@context": {
    "duumbi": "https://duumbi.dev/ns/core#",
    "stdlib": "https://duumbi.dev/ns/stdlib#",
    "acme": "https://registry.acme.com/ns#"
  },
  "@type": "duumbi:Module",
  "@id": "duumbi:myapp/main",
  "duumbi:name": "main",
  "duumbi:imports": [
    {
      "duumbi:module": "@duumbi/stdlib-math",
      "duumbi:version": "1.0",
      "duumbi:functions": ["abs", "max", "min"]
    },
    {
      "duumbi:module": "@acme/billing-core",
      "duumbi:version": "3.0",
      "duumbi:functions": ["calculate_tax", "apply_discount"]
    }
  ]
}
```

### 3.3 Ütközéskezelés

**Szabály:** Soha nem lehet két azonos nevű függvény ugyanabban a névtérben.

```
Feloldási prioritás (magasabbtól alacsonyabbig):

1. Workspace modul explicit import (duumbi:imports → functions lista)
2. Vendor réteg (scope + version rögzítve)
3. Cache réteg (lockfile hash validálva)

Ütközés esetén:
- Különböző scope → nincs ütközés (@duumbi/math::add ≠ @acme/math::add)
- Azonos scope + különböző verzió → lockfile dönt
- Workspace vs dependency → workspace nyer (felülírás)
- Ha kiértékelhetetlen → E012 ModuleConflict error
```

---

## 4. Teljes Workspace Struktúra

```
myapp/
├── .duumbi/
│   ├── config.toml                   # Workspace konfiguráció
│   ├── deps.lock                     # Lockfile v1 (hash + forrás + integritás)
│   │
│   ├── graph/                        # WORKSPACE LAYER — saját fejlesztés
│   │   ├── main.jsonld               #   Belépési pont
│   │   ├── api/                      #   Almodul könyvtár
│   │   │   └── handlers.jsonld       #     API kezelők
│   │   └── domain/                   #   Almodul könyvtár
│   │       └── user.jsonld           #     Domain logika
│   │
│   ├── vendor/                       # VENDOR LAYER — verziókezelt
│   │   └── @company/                 #   Szervezeti scope
│   │       └── auth-core/            #     Rögzített modul
│   │           ├── graph/
│   │           │   └── auth.jsonld
│   │           └── manifest.toml     #     Verzió + hash + forrás
│   │
│   ├── cache/                        # CACHE LAYER — .gitignore
│   │   ├── @duumbi/                  #   Hivatalos modulok
│   │   │   ├── stdlib-math@1.0.0/
│   │   │   └── stdlib-io@1.0.0/
│   │   └── @community/              #   Közösségi modulok
│   │       └── sorting@2.1.0/
│   │
│   ├── build/                        # Fordított fájlok (.o, bináris)
│   ├── history/                      # Undo snapshots
│   ├── intents/                      # Intent spec-ek (M5)
│   ├── learning/                     # Tanulási napló (M8)
│   └── schema/                       # Validációs sémák
│
├── .gitignore                        # Tartalmazza: .duumbi/cache/
└── ...
```

---

## 5. config.toml Bővítés

```toml
[workspace]
name = "myapp"
namespace = "myapp"          # Lokális modulok névtere
default-registry = "duumbi"  # Alapértelmezett registry

[registries]
duumbi = "https://registry.duumbi.dev"
company = "https://registry.acme.com"
# Air-gapped: nincs registry → vendor-only mód

[dependencies]
# Hivatalos stdlib (publikus registry)
"@duumbi/stdlib-math" = "1.0"
"@duumbi/stdlib-io" = "1.0"

# Közösségi modul (publikus registry)
"@community/sorting" = { version = "2.1", features = ["parallel"] }

# Vállalati zárt modul (privát registry)
"@company/auth-core" = { version = "3.0", registry = "company" }

# Lokális path dependency (fejlesztés közbeni hivatkozás)
"local-utils" = { path = "../shared/utils" }

# Git dependency (branch/tag/rev)
"experimental-ml" = { git = "https://github.com/user/ml-graphs.git", tag = "v0.3" }

[vendor]
# Explicit vendor szabályok
strategy = "selective"   # "all" | "selective" | "none"
include = ["@company/*"] # Csak ezeket vendoroljuk
```

---

## 6. deps.lock v1 Formátum

```toml
# Auto-generated by duumbi deps install — DO NOT EDIT
version = 1

[[dependencies]]
name = "@duumbi/stdlib-math"
version = "1.0.0"
source = "registry+https://registry.duumbi.dev"
semantic_hash = "a1b2c3d4e5f6..."
integrity = "sha256-ABCDEF..."
resolved_path = ".duumbi/cache/@duumbi/stdlib-math@1.0.0"

[[dependencies]]
name = "@company/auth-core"
version = "3.0.1"
source = "registry+https://registry.acme.com"
semantic_hash = "f6e5d4c3b2a1..."
integrity = "sha256-123456..."
resolved_path = ".duumbi/vendor/@company/auth-core"
vendored = true

[[dependencies]]
name = "local-utils"
version = "0.0.0"
source = "path+../shared/utils"
semantic_hash = "789abc..."
integrity = "sha256-FEDCBA..."
resolved_path = "../shared/utils/.duumbi/graph"
```

---

## 7. manifest.toml (Modul Metaadatok)

Minden publisholt modul tartalmaz egy `manifest.toml`-t:

```toml
[module]
name = "@duumbi/stdlib-math"
version = "1.0.0"
description = "Mathematical utility functions (abs, max, min)"
license = "MPL-2.0"
authors = ["DUUMBI Team <team@duumbi.dev>"]
repository = "https://github.com/hgahub/duumbi"
keywords = ["math", "stdlib", "numeric"]
categories = ["math", "stdlib"]

[module.duumbi]
min-compiler = "0.5.0"     # Minimális kompatibilis compiler verzió
namespace = "https://duumbi.dev/ns/stdlib#"

[exports]
functions = ["abs", "max", "min"]
# Jövő: types, interfaces, constants

[dependencies]
# Ez a modul saját függőségei (tranzitív)
# "@duumbi/stdlib-io" = "1.0"
```

---

## 8. Betöltési Pipeline (Resolution Pipeline)

![[DUUMBI - Resolution Pipeline.excalidraw|Modul Betöltési Pipeline]]

```
                         duumbi build / duumbi check
                                    │
                                    ▼
                        ┌───────────────────────┐
                        │  config.toml beolvasás │
                        │  [dependencies] szekció│
                        └───────────┬───────────┘
                                    │
                                    ▼
                        ┌───────────────────────┐
                        │  deps.lock ellenőrzés  │
                        │  Van lock? Érvényes?   │
                        └──┬────────────────┬───┘
                           │ Igen            │ Nem
                           ▼                 ▼
                ┌──────────────────┐  ┌──────────────────┐
                │ Lock alapján     │  │ Feloldás:         │
                │ betöltés         │  │ 1. Registry query │
                │ (gyors útvonal)  │  │ 2. Version solve  │
                └────────┬─────────┘  │ 3. Download       │
                         │            │ 4. Lock generálás  │
                         │            └────────┬───────────┘
                         │                     │
                         ▼                     ▼
                ┌───────────────────────────────────────┐
                │          MODUL FELOLDÁS                │
                │                                       │
                │  Minden dependency-re:                 │
                │  1. Workspace (.duumbi/graph/) ?       │
                │     → Ha van → használd (override)     │
                │  2. Vendor (.duumbi/vendor/) ?          │
                │     → Ha van → hash ellenőrzés → betölt│
                │  3. Cache (.duumbi/cache/) ?            │
                │     → Ha van + hash OK → betölt        │
                │  4. Registry letöltés → cache-be        │
                │     → Hash generálás → betölt          │
                │  5. Nincs sehol → E011 DependencyError  │
                └──────────────────┬────────────────────┘
                                   │
                                   ▼
                ┌───────────────────────────────────────┐
                │    PROGRAM ÖSSZEÁLLÍTÁS                │
                │                                       │
                │  Program::load_from_resolved_deps()   │
                │  1. Workspace modulok parse + graph    │
                │  2. Dependency modulok parse + graph   │
                │  3. Export tábla összeállítás           │
                │  4. Cross-module Call validáció         │
                │  5. Ütközés-detekció (E012)            │
                │  6. Ciklus-detekció (E007)             │
                └──────────────────┬────────────────────┘
                                   │
                                   ▼
                            SemanticFixedPoint ✓
```

---

## 9. CLI Parancsok (Bővítés)

### 9.1 Dependency Kezelés

```bash
# Dependency hozzáadása
duumbi deps add @duumbi/stdlib-math         # Legújabb verzió
duumbi deps add @community/sorting@2.1      # Adott verzió
duumbi deps add @company/auth --registry company  # Privát registry

# Dependency telepítés (lockfile alapján)
duumbi deps install                          # Minden dep letöltése
duumbi deps install --frozen                 # Lockfile változás tiltása

# Vendoring
duumbi deps vendor                           # config.toml vendor szabályok szerint
duumbi deps vendor --all                     # Mindent vendorol

# Frissítés
duumbi deps update                           # Kompatibilis verziók
duumbi deps update @community/sorting        # Egy modul frissítése

# Keresés
duumbi search "sorting algorithm"            # Szöveges keresés
duumbi search --type "i64 -> i64"            # Típus-alapú keresés
duumbi search --interface "Comparable"        # Interface-alapú keresés

# Publikálás
duumbi publish                               # Alapértelmezett registry-be
duumbi publish --registry company             # Privát registry-be

# Információ
duumbi deps tree                             # Dependency fa
duumbi deps audit                            # Biztonsági audit
```

### 9.2 Registry Kezelés

```bash
# Registry konfiguráció
duumbi registry add company https://registry.acme.com
duumbi registry login company                # Token-based auth
duumbi registry list                         # Konfigurált registry-k

# Offline mód
duumbi build --offline                       # Csak vendor + cache
```

---

## 10. Szemantikus Keresés Architektúra

![[DUUMBI - Semantic Search Levels.excalidraw|Szemantikus Keresési Rétegek]]

```
                    ┌─────────────────────────────────┐
                    │     KERESÉSI RÉTEGEK             │
                    └─────────────────────────────────┘

    ┌───────────────────────────────────────────────────────────┐
    │  Level 1: SZÖVEGES KERESÉS (M7)                          │
    │                                                           │
    │  "sorting" → modul név, leírás, kulcsszavak illesztése   │
    │  Eszköz: egyszerű string matching + manifest indexelés    │
    └───────────────────────────────────────────────────────────┘
                              │
                              ▼
    ┌───────────────────────────────────────────────────────────┐
    │  Level 2: TÍPUS-ALAPÚ KERESÉS (M7)                       │
    │                                                           │
    │  "fn(i64, i64) -> bool" → export szinatúra illesztés     │
    │  Eszköz: export tábla + típus-unifikáció                 │
    │                                                           │
    │  Példa: "Van-e modul ami két i64-ből bool-t ad?"         │
    │  → @duumbi/stdlib-math::compare, @community/sorting::lt  │
    └───────────────────────────────────────────────────────────┘
                              │
                              ▼
    ┌───────────────────────────────────────────────────────────┐
    │  Level 3: INTERFACE-ALAPÚ KERESÉS (M7+)                  │
    │                                                           │
    │  "Comparable" → interface implementáló modulok           │
    │  Eszköz: interface registry + contract matching          │
    └───────────────────────────────────────────────────────────┘
                              │
                              ▼
    ┌───────────────────────────────────────────────────────────┐
    │  Level 4: GRÁF-IZOMORFIZMUS KERESÉS (M8)                │
    │                                                           │
    │  "Logikailag hasonló modulok" → gráf struktúra hasonlóság│
    │  Eszköz: semantic hash variánsok + gráf kernel-ek        │
    │                                                           │
    │  Példa: "Van-e hasonló logikájú modul mint az enyém?"    │
    │  → Az upload-olt gráf szerkezetét hasonlítjuk            │
    └───────────────────────────────────────────────────────────┘
                              │
                              ▼
    ┌───────────────────────────────────────────────────────────┐
    │  Level 5: EMBEDDING-ALAPÚ KERESÉS (M8-EMBED)            │
    │                                                           │
    │  "Szemantikailag rokon modulok" → vektor hasonlóság      │
    │  Eszköz: HolE / ComplEx embedding + cosine similarity    │
    │                                                           │
    │  Példa: "Mi hasonlít a factorial függvényemre?"          │
    │  → Irányított gráf relációk kódolva a komplex térben     │
    └───────────────────────────────────────────────────────────┘
```

---

## 11. Forgatókönyvek (Use Cases)

### 11.1 Solo fejlesztő — publikus modulokkal

```bash
duumbi init myapp
cd myapp
duumbi deps add @duumbi/stdlib-math
duumbi deps add @community/sorting
duumbi deps install
# → .duumbi/cache/@duumbi/stdlib-math@1.0.0/
# → .duumbi/cache/@community/sorting@2.1.0/
duumbi build
```

### 11.2 Vállalati fejlesztő — privát registry + vendor

```bash
duumbi init internal-app
cd internal-app
duumbi registry add corp https://registry.corp.com
duumbi registry login corp
duumbi deps add @corp/auth-core --registry corp
duumbi deps add @corp/billing --registry corp
duumbi deps vendor --all   # Mindent VCS-be
duumbi build --offline     # CI/CD: nincs hálózati hozzáférés
```

### 11.3 Nyílt forráskódú könyvtár publikálása

```bash
cd my-sorting-lib
# manifest.toml szerkesztése: name, version, exports
duumbi publish             # → registry.duumbi.dev
# Más fejlesztő:
duumbi search "sorting"    # → @gabor/sorting 2.1.0
duumbi deps add @gabor/sorting
```

### 11.4 Kézzel bemásolt gráf könyvtár

```bash
# Vendor könyvtárba másolás
mkdir -p .duumbi/vendor/@internal/legacy-utils/graph/
cp ~/shared-graphs/utils.jsonld .duumbi/vendor/@internal/legacy-utils/graph/
# manifest.toml létrehozása
cat > .duumbi/vendor/@internal/legacy-utils/manifest.toml << EOF
[module]
name = "@internal/legacy-utils"
version = "0.0.1"
[exports]
functions = ["helper1", "helper2"]
EOF
# config.toml-ba hozzáadás
# "@internal/legacy-utils" = { vendor = true }
```

---

## 12. Migráció a Jelenlegi Állapotból

### 12.1 Visszafelé kompatibilitás

| Jelenlegi elem | Új elem | Migráció |
|----------------|---------|----------|
| `.duumbi/graph/` | `.duumbi/graph/` (változatlan) | Nincs szükséges |
| `.duumbi/stdlib/math/.duumbi/graph/math.jsonld` | `.duumbi/cache/@duumbi/stdlib-math@1.0.0/graph/math.jsonld` | Automatikus migráció `duumbi upgrade` |
| `config.toml [dependencies] math = { path = ".duumbi/stdlib/math" }` | `config.toml [dependencies] "@duumbi/stdlib-math" = "1.0"` | Config formátum v2 |
| `deps.lock` (hash + path) | `deps.lock` v1 (hash + source + integrity + vendored) | Automatikus újragenerálás |

### 12.2 Migrációs parancs

```bash
duumbi upgrade                # Felismeri a régi formátumot, migrálja
# 1. stdlib/ → cache/@duumbi/
# 2. config.toml path deps → scoped deps
# 3. deps.lock regenerálás
# 4. .gitignore frissítés (.duumbi/cache/)
```

---

## 13. Biztonsági Szempontok

| Szempont | Megoldás |
|----------|---------|
| Supply chain attack | Lockfile integrity hash (SHA-256) + semantic hash |
| Typosquatting | Scope-alapú névtér (nehezebb @duumbi/ scope-ot hamisítani) |
| Dependency confusion | Workspace mindig nyer → belső modul nem írható felül külsővel |
| Air-gapped deploy | `duumbi deps vendor --all` + `duumbi build --offline` |
| Audit trail | `duumbi deps audit` → ismert sérülékenységek ellenőrzése |
| Privát modulok szivárgása | Registry-szintű auth token + scope-level hozzáférés-kezelés |

---

## 14. Kapcsolat a Knowledge Graph Vízióval (PRD Phase A)

Ez a repository architektúra **alapozza meg** a PRD-ben leírt Knowledge Base víziót:

```
Jelenlegi terv (M7)              Hosszú táv (Phase A)
────────────────────             ──────────────────────
.jsonld modulok                  .jsonld + .md + .xml hibrid
manifest.toml metaadatok         Szemantikus metaadatok a gráfban
Scope-alapú névtér               Cross-format wikilink-ek
Registry keresés                 duumbi-daemon in-memory index
Semantic hash                    Knowledge graph traversal
deps.lock                        Gráf-szintű verziókezelés
```

A scope rendszer (`@scope/module`) és a manifest formátum úgy van tervezve, hogy természetesen bővíthető legyen a Knowledge Base irányba — a `manifest.toml` metaadatai átvihetők a gráf csomópontok property-jeibe.

---

## Kapcsolódó Dokumentumok

- [[DUUMBI - Post-MVP Implementation Roadmap]] — Végrehajtási terv (M7 Registry & Distribution)
- [[DUUMBI - Post-MVP Roadmap]] — Üzleti terv (Phase 5 Registry)
- [[DUUMBI - PRD]] — Hosszú távú vízió (Knowledge Base, Phase A)
- [[DUUMBI - Architecture Diagram]] — Technikai architektúra
- [[DUUMBI - Glossary]] — Fogalomtár
