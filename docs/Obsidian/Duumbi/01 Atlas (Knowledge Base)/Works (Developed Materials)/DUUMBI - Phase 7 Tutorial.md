---
tags:
  - project/duumbi
  - milestone/phase-7
  - tutorial
status: complete
updated: 2026-03-13
---
# Phase 7 Tutorial — Registry & Distribution

> Gyakorlati bemutató a Phase 7 teljes funkciókészletéhez.
> Végigvezetünk a modul készítéstől a publikáláson át a függőségkezelésig.

← Vissza: [[DUUMBI - Phase 7 - Registry & Distribution]]

---

## Előfeltételek

A tutorial előtt szükséges:
- `duumbi` CLI telepítve és elérhető a PATH-ban
- Működő `.duumbi/` workspace (`duumbi init` után)
- Internet kapcsolat a registry műveletekhez

---

## 1. Workspace beállítás

### 1.1 Új projekt létrehozása

```bash
mkdir myapp && cd myapp
duumbi init
```

Ez létrehozza a `.duumbi/` könyvtárat a következő tartalommal:
- `config.toml` — workspace konfiguráció
- `graph/main.jsonld` — fő modul
- `schema/core.schema.json` — validációs séma
- `cache/@duumbi/stdlib-math@1.0.0/` — beépített stdlib

### 1.2 Config v2 felépítése

A `config.toml` Phase 7-ben bővült. Nyisd meg a `.duumbi/config.toml` fájlt:

```toml
[workspace]
name = "myapp"
namespace = "myapp"
default-registry = "duumbi"

[registries]
duumbi = "https://registry.duumbi.dev"

[dependencies]
"@duumbi/stdlib-math" = "1.0"

[vendor]
strategy = "none"
include = []
```

**Új szekciók:**
- `[registries]` — nevesített registry endpoint-ok
- `[dependencies]` — SemVer verziókkal vagy helyi útvonalakkal
- `[vendor]` — offline build stratégia

---

## 2. Registry kezelés

### 2.1 Registry-k listázása

```bash
duumbi registry list
```

**Kimenet:**
```
NAME                 URL
────────────────────────────────────────────────────────
duumbi               https://registry.duumbi.dev (default)
```

### 2.2 Új registry hozzáadása

Tegyük fel, hogy a céged privát registry-t üzemeltet:

```bash
duumbi registry add company https://registry.acme.com
```

Ellenőrzés:
```bash
duumbi registry list
```

```
NAME                 URL
────────────────────────────────────────────────────────
duumbi               https://registry.duumbi.dev (default)
company              https://registry.acme.com
```

### 2.3 Alapértelmezett registry beállítása

```bash
duumbi registry default company
```

Ezután minden scope nélküli függőség a `company` registry-ből fog feloldódni.

Állítsuk vissza:
```bash
duumbi registry default duumbi
```

### 2.4 Bejelentkezés (autentikáció)

Publikáláshoz és yank-eléshez token szükséges:

```bash
# Interaktív mód — kézileg beírod a tokent
duumbi registry login duumbi

# CI/CD mód — token flag-gel
duumbi registry login duumbi --token "tok_abc123..."
```

A token tárolási helye: `~/.duumbi/credentials.toml` (0600 jogosultságok).

### 2.5 Kijelentkezés

```bash
# Egy registry-ből
duumbi registry logout duumbi

# Minden registry-ből
duumbi registry logout
```

### 2.6 Registry eltávolítása

```bash
duumbi registry remove company
```

> Figyelmeztet, ha még vannak függőségek amelyek erre a registry-re mutatnak.

---

## 3. Függőségkezelés

### 3.1 Függőség hozzáadása (helyi útvonal)

Helyi modul hozzáadása (például egy másik könyvtárból):

```bash
duumbi deps add local-utils ../shared/utils
```

A `config.toml`-ban megjelenik:
```toml
[dependencies]
"local-utils" = { path = "../shared/utils" }
```

### 3.2 Függőség hozzáadása (registry-ből)

```bash
# Legújabb verzió
duumbi deps add @duumbi/stdlib-math

# Adott verzió
duumbi deps add @duumbi/stdlib-math@^1.0

# Másik registry-ből
duumbi deps add @company/auth --registry company
```

**Scope-alapú routing szabályok:**
- `@scope/name` → automatikusan a `scope` nevű registry-ből
- `@duumbi/*` → mindig a `duumbi` registry-ből
- Scope nélküli nevek → `default-registry`-ből

### 3.3 Függőségek listázása

```bash
duumbi deps list
```

**Kimenet:**
```
@duumbi/stdlib-math: 1.0 → .duumbi/cache/@duumbi/stdlib-math@1.0.0/graph
local-utils: { path = "../shared/utils" } → ../shared/utils/.duumbi/graph
```

### 3.4 Függőségek telepítése

Ez a fő parancs: letölti a hiányzó függőségeket és generálja a lockfile-t.

```bash
duumbi deps install
```

**Kimenet:**
```
Resolving dependencies...
  ✓ @duumbi/stdlib-math@1.0.0 — downloaded
  ✓ local-utils — path dependency
Lockfile written: .duumbi/deps.lock
Downloaded: 1, Cached: 0, Path: 1
```

**CI/CD-ben** a `--frozen` flag biztosítja, hogy a lockfile ne változzon:
```bash
duumbi deps install --frozen
```

Ha a lockfile változna, hibát dob (determinisztikus build).

### 3.5 Függőségek frissítése

```bash
# Minden függőség frissítése a legújabb kompatibilis verzióra
duumbi deps update

# Csak egy adott függőség
duumbi deps update @duumbi/stdlib-math
```

**Kimenet:**
```
@duumbi/stdlib-math: ^1.0 → 1.2.0 (latest compatible)
Updated 1 dependency
```

### 3.6 Függőség eltávolítása

```bash
duumbi deps remove local-utils
```

### 3.7 Függőségfa megjelenítése

```bash
duumbi deps tree
```

**Kimenet:**
```
(workspace)
├── @duumbi/stdlib-math v1.0.0 (cached)
└── local-utils v? (path: ../shared/utils)
```

Mélység korlátozása:
```bash
duumbi deps tree --depth 2
```

---

## 4. Integritás-ellenőrzés (audit)

### 4.1 Lockfile ellenőrzés

A lockfile SHA-256 hash-eket tárol minden függőséghez. Az `audit` parancs újraszámolja és összeveti:

```bash
duumbi deps audit
```

**Sikeres kimenet:**
```
✓ @duumbi/stdlib-math v1.0.0 — integrity OK
All 1 dependencies passed integrity audit
```

**Ha valaki módosította a cache-elt fájlokat:**
```
✗ @duumbi/stdlib-math v1.0.0 — INTEGRITY MISMATCH (E015)
  Expected: sha256:abc123...
  Got:      sha256:def456...
```

> Ez biztonsági funkció: észleli ha a letöltött modulokat megváltoztatták.

---

## 5. Vendoring (offline build)

### 5.1 Összes függőség vendorolása

```bash
duumbi deps vendor --all
```

Ez átmásolja a cached függőségeket a `.duumbi/vendor/` könyvtárba:
```
Vendored @duumbi/stdlib-math → .duumbi/vendor/@duumbi/stdlib-math/
```

### 5.2 Szelektív vendoring

Csak bizonyos scope-ok vendorolása:
```bash
duumbi deps vendor --include "@company/*"
```

Vagy a `config.toml`-ban is beállítható:
```toml
[vendor]
strategy = "selective"
include = ["@company/*"]
```

Ezután a sima `duumbi deps vendor` követi a konfigot.

### 5.3 Offline build

Ha a vendor megtörtént, a build internet nélkül is működik:
```bash
duumbi build --offline
```

A feloldási sorrend ilyenkor:
1. **Workspace** — `.duumbi/graph/` (saját forrás)
2. **Vendor** — `.duumbi/vendor/@scope/name/graph/`
3. **Cache** — `.duumbi/cache/@scope/name@version/graph/`
4. Ha sehol → E011 hiba

---

## 6. Modul publikálás

### 6.1 Manifest elkészítése

A publikáláshoz szükséges egy `manifest.toml` a `.duumbi/` könyvtárban:

```toml
[module]
name = "@myorg/mylib"
version = "1.0.0"
description = "Saját duumbi modul"
license = "MPL-2.0"

[exports]
functions = ["calculate", "transform"]
```

### 6.2 Ellenőrzés (dry-run)

Először próbáld ki csomagolás nélkül:

```bash
duumbi publish --dry-run
```

**Kimenet:**
```
📦 Package: @myorg/mylib v1.0.0
   Registry: https://registry.duumbi.dev
   Size: 4.2 KB
   Integrity: sha256:abc123...
   Files:
     manifest.toml (245 B)
     graph/main.jsonld (3.8 KB)

Dry run — package not uploaded.
```

### 6.3 Éles publikálás

```bash
duumbi publish
```

Megerősítés után feltölti a modult. A `--yes` flag átugorja a megerősítést:

```bash
duumbi publish --yes
```

Adott registry-be publikálás:
```bash
duumbi publish --registry company
```

### 6.4 Modul keresése

A publikálás után mások megtalálják a modult:

```bash
duumbi search mylib
```

**Kimenet:**
```
NAME                 VERSION    DESCRIPTION
──────────────────────────────────────────────────────────────
@myorg/mylib         1.0.0      Saját duumbi modul

Found 1 result on duumbi
```

Adott registry-ben keresés:
```bash
duumbi search auth --registry company
```

---

## 7. Verzió visszavonás (yank)

Ha egy publikált verzióval probléma van:

```bash
duumbi yank @myorg/mylib@1.0.0
```

**Fontos tudnivalók:**
- A yank-elt verzió **nem törlődik** — létező lockfile-ok még elérik
- Új `deps install` már **nem választja** a yank-elt verziót
- Visszavonhatatlan művelet

CI/CD-ben:
```bash
duumbi yank @myorg/mylib@1.0.0 --yes
```

---

## 8. Migráció Phase 4-5-ből

Ha létező Phase 4-5 projekted van:

```bash
duumbi upgrade
```

Ez automatikusan:
- Átírja a `config.toml`-t v2 formátra
- Hozzáadja a `[registries]` és `[vendor]` szekciókat
- Átnevezi az stdlib útvonalakat az új cache elrendezés szerint
- Generálja a lockfile-t

---

## 9. Hibakódok gyorsreferencia

| Kód  | Jelentés                          | Tipikus ok                              |
|------|-----------------------------------|-----------------------------------------|
| E011 | Függőség nem található            | Nincs vendor/cache-ben, registry offline |
| E012 | Modul ütközés                     | Azonos név, különböző verzió            |
| E013 | Registry elérhetetlen             | Hálózati hiba, rossz URL                |
| E014 | Autentikáció sikertelen           | Hiányzó/érvénytelen token               |
| E015 | Integritás eltérés                | Módosított cache fájl                   |
| E016 | Verzió nem található              | Nem létező verzió a registry-ben        |

---

## 10. Registry infrastruktúra

A publikus registry a következő címen érhető el:

- **URL:** `https://registry.duumbi.dev`
- **Health:** `https://registry.duumbi.dev/health` → `ok`
- **Web:** `https://registry.duumbi.dev` (böngészőben)

**Technológia:**
- Rust (axum) szerver, SQLite adatbázis
- Azure Container Apps (scale-to-zero, 0.25 vCPU)
- Managed TLS certifikátum (DigiCert)
- CI/CD: push to main → GHCR → Azure deploy → smoke test

---

## Összefoglaló munkafolyamat

Egy tipikus fejlesztési ciklus Phase 7-ben:

```
1. duumbi init                         # Új projekt
2. duumbi registry login duumbi        # Autentikáció
3. duumbi deps add @duumbi/stdlib-math # Függőség hozzáadása
4. duumbi deps install                 # Letöltés + lockfile
5. duumbi build                        # Fordítás
6. duumbi deps audit                   # Integritás ellenőrzés
7. duumbi deps vendor --all            # Offline készítés
8. duumbi publish --dry-run            # Csomagolás tesztelése
9. duumbi publish                      # Éles publikálás
```

---

← Vissza: [[DUUMBI - Phase 7 - Registry & Distribution]]
Kapcsolódó: [[DUUMBI - Architecture Diagram]] · [[DUUMBI - Glossary]]
