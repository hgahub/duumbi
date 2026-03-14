---
tags:
  - project/duumbi
  - doc/strategy
status: draft
created: 2026-02-28
updated: 2026-02-28
related_maps:
  - "[[DUUMBI - PRD]]"
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - Task List]]"
  - "[[DUUMBI - Tools and Components]]"
  - "[[DUUMBI - Architecture Diagram]]"
  - "[[DUUMBI - Glossary]]"
---

# DUUMBI — Post-MVP Roadmap, Üzleti Terv és Monetizációs Stratégia

> [!important] Ez a dokumentum az MVP (Phase 0–3) **befejezése utáni** fejlesztési tervet, üzleti stratégiát és monetizációs modellt tartalmazza. Az MVP specifikációért lásd: [[DUUMBI - MVP Specification]]. A hosszú távú vízióért: [[DUUMBI - PRD]].

---

## 1. Kiindulási Állapot — Amit az MVP Szállít

Az MVP befejezésére (Phase 0–3) a DUUMBI az alábbi képességekkel rendelkezik:

| Képesség | Állapot |
|----------|---------|
| JSON-LD → Cranelift → natív bináris | ✅ Működik |
| CLI: `init`, `build`, `run`, `check`, `describe` | ✅ Production-ready |
| AI mutáció: `add`, `undo` (Anthropic + OpenAI) | ✅ >70% benchmark pontosság |
| Web vizualizáció: `viz` (localhost, read-only) | ✅ Működik |
| Típusrendszer: `i64`, `f64`, `bool`, `void` | ✅ Komplett |
| Op set: Const, Add/Sub/Mul/Div, Compare, Branch, Call, Load/Store, Print, Return | ✅ Komplett |

**Ami NINCS az MVP-ben:** repository/registry, modulrendszer, objektumok/interfészek, multi-ágens, self-healing, projektmenedzsment integráció, cloud.

---

## 2. Post-MVP Fázisok — Végrehajtási Terv

### Phase 4: Modulrendszer és Újrafelhasználhatóság (6-8 hét)

> **Cél:** Lehetővé tenni gráf-részletek újrafelhasználását projektek között.
> **Kill Criterion:** Egy projektben importált külső modul sikeresen fordul és fut.

#### 4.1 Típusrendszer Bővítés

| Elem | Leírás | Prioritás |
|------|--------|-----------|
| `duumbi:Struct` | Összetett adattípus definiálása (névvel ellátott mezők, típusokkal) | P0 |
| `duumbi:Array` | Fix méretű tömb típus | P0 |
| `duumbi:String` | UTF-8 heap-allokált string (length-prefixed) | P1 |
| `duumbi:Pointer` | Referencia típus (borrow semantics nélkül, GC-vel) | P2 |

#### 4.2 Interface és Contract Rendszer

```jsonld
{
  "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
  "@type": "duumbi:Interface",
  "@id": "duumbi:stdlib/Printable",
  "duumbi:name": "Printable",
  "duumbi:methods": [
    {
      "duumbi:name": "to_string",
      "duumbi:params": [{"name": "self", "type": "Self"}],
      "duumbi:returnType": "string"
    }
  ]
}
```

- Az interfészek JSON-LD sémaként definiáltak
- Modulok **implementálhatnak** interfészeket: `duumbi:implements` mező
- Az AI ágens az interfész specifikációból automatikusan generálhat implementációt
- Schema validator kikényszeríti, hogy minden deklarált metódus létezzen

#### 4.3 Modul Rendszer

- **Import/Export mechanizmus:** `duumbi:imports` és `duumbi:exports` mező az `duumbi:Module` típuson
- **Névtér kezelés:** Modulhatárokon egyedi névterek (`duumbi:mylib/math/add`)
- **Külső modul hivatkozás:** `duumbi:dependency` node, ami verziószámmal és forrással (lokális path vagy registry URL) hivatkozik
- **CLI parancs:** `duumbi deps install` — feloldja és letölti a függőségeket

---

### Phase 5: DUUMBI Registry — Szemantikus Gráf Repository (8-10 hét)

> **Cél:** Maven/npm/crates.io-szerű központi regisztri, ahol JSON-LD modulok megoszthatók.
> **Kill Criterion:** `duumbi publish` + `duumbi deps install` működik egy publikus modulra.

#### 5.1 Registry Architektúra

```
registry.duumbi.dev
├── API (REST + GraphQL)
│   ├── POST /publish          — modul feltöltés (JSON-LD + metadata)
│   ├── GET  /search?q=...     — szemantikus keresés
│   ├── GET  /module/{id}      — modul letöltés (versioned)
│   └── GET  /graph/{id}       — vizuális gráf nézet (embed)
├── Storage
│   ├── Object Storage         — gráf fájlok + pre-compiled binárisok
│   └── PostgreSQL             — metadata, felhasználók, verziók
├── CDN
│   └── Binary Cache           — platformfüggő pre-compiled .o fájlok
└── Web Frontend
    ├── Keresés + böngészés
    ├── Modul dokumentáció (auto-generált a gráfból)
    └── Felhasználói profilok
```

#### 5.2 Szemantikus Keresés

A hagyományos szöveges keresés helyett a registry **gráf-alapú keresést** kínál:

- **Típus-alapú keresés:** "Function that takes two i64 and returns bool" → megtalálja a `Compare`-szerű modulokat
- **Interface-alapú keresés:** "Module implementing Printable" → listázza az összes implementáló modult
- **Hasonlósági keresés:** Gráf-izomorfizmus alapján hasonló logikájú modulok keresése
- **Kategória címkék:** `math`, `io`, `data-structures`, `algorithms` stb.

#### 5.3 Verziókezelés

| Aspektus | Megoldás |
|----------|----------|
| Verzió formátum | SemVer 2.0 (`MAJOR.MINOR.PATCH`) |
| Kompatibilitás | Interface-kompatibilitás vizsgálat (gráf diffing) |
| Lock file | `.duumbi/deps.lock` — pontos verziók rögzítése |
| Yanked verziók | `duumbi yank 1.2.3` — hibás verziók visszavonása |

#### 5.4 Binary Cache (Content-Addressable)

- Minden modul **szemantikus hash-t** kap (a gráf struktúra + op értékek ujjlenyomata, `@id` értékek nélkül)
- Ha a hash egyezik → a pre-compiled `.o` letöltődik a CDN-ről
- A fordítási idő **proporcionálisan csökken** a változatlan modulok számával
- Platform-specifikus binárisok: `{hash}-{target}.o` (pl. `abc123-aarch64-apple-darwin.o`)

#### 5.5 CLI Parancsok

```bash
duumbi publish                  # aktuális modul publikálása a registry-be
duumbi deps add math/vectors    # függőség hozzáadása
duumbi deps install             # összes függőség letöltése
duumbi deps update              # lock file frissítése a legújabb kompatibilis verziókra
duumbi search "sort algorithm"  # szemantikus keresés
```

---

### Phase 6: Intent-Driven Development — Spec-Alapú Fejlesztés (6-8 hét)

> **Cél:** Az Augment Intent által inspirált, de DUUMBI-specifikus spec-vezérelt fejlesztési mód bevezetése.
> **Kill Criterion:** Spec-ből generált, többfüggvényes program korrekt outputot ad.

#### 6.1 A DUUMBI Intent Modell

Az Augment Intent megközelítését adaptáljuk a szemantikus gráf világára. Az Intent by Augment kulcs koncepciói:
- **Spec mint igazságforrás** — a változtatás terve strukturált dokumentumban
- **Koordinátor ágens** — elemzi a kódbázist és tervezetet készít
- **Specializált ágensek** — párhuzamos végrehajtás

A DUUMBI-ban ez **gráf-natívan** működik:

```
Felhasználói szándék (természetes nyelv)
    │
    ▼
┌─────────────────────────┐
│  Intent Spec (YAML/MD)  │  ← Emberi felülvizsgálat pont
│  - Cél leírás           │
│  - Elfogadási kritérium │
│  - Érintett modulok     │
│  - Tesztesetek           │
└─────────────────────────┘
    │
    ▼
┌─────────────────────────┐
│  Coordinator Agent      │  ← Elemzi a gráfot, feladatokra bont
│  - Gráf analízis        │
│  - Terv generálás       │
│  - Task felbontás       │
└─────────────────────────┘
    │
    ├──────────┬────────────┐
    ▼          ▼            ▼
┌────────┐ ┌────────┐ ┌─────────┐
│ Coder  │ │ Coder  │ │ Tester  │  ← Párhuzamos végrehajtás
│ Agent  │ │ Agent  │ │ Agent   │
└────────┘ └────────┘ └─────────┘
    │          │            │
    ▼          ▼            ▼
    ┌──────────────────────┐
    │   Graph Merge +      │  ← Automatikus merge
    │   Schema Validation  │
    └──────────────────────┘
    │
    ▼
  Semantic Fixed Point ✓
```

#### 6.2 Intent Spec Formátum

```yaml
# .duumbi/intents/add-sorting-module.yaml
intent: "Valósíts meg egy mergesort algoritmust i64 tömbre"
acceptance_criteria:
  - "A sort funkció rendezett tömböt ad vissza"
  - "A bemenet nem módosul (immutable)"
  - "Rekurzív implementáció"
affected_modules:
  - "algorithms/sorting"
test_cases:
  - input: [5, 3, 8, 1, 2]
    expected: [1, 2, 3, 5, 8]
  - input: [1]
    expected: [1]
  - input: []
    expected: []
```

#### 6.3 CLI

```bash
duumbi intent create "Valósíts meg mergesort-ot"  # interaktív spec generálás
duumbi intent review                                # spec felülvizsgálat
duumbi intent execute                               # ágensek végrehajtják a spec-et
duumbi intent status                                # végrehajtás állapota
```

---

### Phase 8: Registry Auth & User Management (4-6 hét)

> **Cél:** Felhasználó regisztráció, bejelentkezés és API token kezelés a duumbi registry-hez.
> **Kill Criterion:** GitHub OAuth login → token generálás → CLI device code flow login működik.

- **Global registry (registry.duumbi.dev):** GitHub OAuth2
- **Private registry (Docker):** Username + password
- **CLI:** Device code flow (`duumbi registry login duumbi`)
- **Web UI:** Login/avatar nav bar, token management oldal
- **Security:** JWT cookie, CSRF, hashed tokens (SHA-256), argon2 passwords, rate limiting

Részletek: [[DUUMBI - Phase 8 - Registry Auth & User Management]]

---

### Phase 9: Multi-Ágens Orchestráció és Self-Healing (8-10 hét)

> **Cél:** Az Original PRD víziójának megvalósítása — MCP szerver, specializált ágensek, telemetria visszacsatolás.
> **Kill Criterion:** A rendszer detektál egy futásidejű hibát, azonosítja a gráf csomópontot, és javítási javaslatot ad.

#### 7.1 MCP Szerver

A DUUMBI CLI-ből MCP szerver lesz, amely eszközöket (Tools) biztosít az ágenseknek:

| Tool | Leírás |
|------|--------|
| `graph.query` | Gráf lekérdezés (node keresés, szomszédok, útvonalak) |
| `graph.mutate` | Gráf módosítás (patch alkalmazás) |
| `graph.validate` | Schema validáció futtatás |
| `build.compile` | Fordítás indítás |
| `build.run` | Futtatás indítás |
| `telemetry.query` | Telemetriai adatok lekérdezése |
| `deps.search` | Registry keresés |
| `deps.install` | Függőség telepítés |

#### 7.2 Ágens Típusok

| Ágens | Felelősség | Trigger |
|-------|------------|---------|
| **Architect** | C4 struktúra tervezés, modul határok | Intent spec beérkezés |
| **Coder** | Funkció implementáció (JSON-LD Op generálás) | Task assignment |
| **Reviewer** | Biztonsági és teljesítmény ellenőrzés | PR / gráf patch |
| **Tester** | Teszteset generálás és futtatás | Implementáció kész |
| **Ops** | Futó alkalmazás megfigyelés | Telemetria alert |
| **Repair** | Hibás gráf fragment javítás | Ops alert |

#### 7.3 Self-Healing Loop

1. Compiled binary futás közben → OpenTelemetry trace + `traceId` → `nodeId` mapping
2. Anomália detekció (panic, lassulás, hiba arány növekedés)
3. `traceId` → pontos JSON-LD csomópont azonosítás
4. Repair Agent kap: hibás fragment + változó értékek + hibaüzenet
5. Generál javító patch → schema validáció → rebuild → tesztek → PR vagy hot-swap

---

## 3. Üzleti Terv és Monetizációs Stratégia

### 3.1 Licenc és Nyílt Forráskód Döntés

> [!important] **Döntés: Open-Core Modell** (nyílt mag + fizetős kiegészítések)

| Komponens | Licenc | Indoklás |
|-----------|--------|----------|
| **DUUMBI CLI** (core compiler, validator, all ops) | **MPL-2.0** (Mozilla Public License) | Nyílt, de a módosított fájlokat vissza kell adni a közösségnek. Megengedőbb, mint GPL — vállalatok is beágyazhatják. |
| **JSON-LD Core Schema** | **CC0** (Public Domain) | A séma egy szabvány — bárki szabadon használhassa |
| **Registry szerver** | **Zárt forráskód** | Üzleti érték, SaaS futtatás |
| **Intent Engine** (spec-driven AI orchestráció) | **Zárt forráskód** | Prémium funkcionalitás |
| **DUUMBI Cloud** (hosted registry + binary cache + telemetria dashboard) | **Zárt forráskód** | Fő bevételi forrás |

**Miért MPL-2.0 és nem MIT/Apache?**
- MIT/Apache túl megengedő: versenytárs könnyen fork-olhatna és zárt termékké alakítva
- GPL túl szigorú: vállalatok nem ágyaznák be
- MPL-2.0 = "fájl szintű copyleft" — ha módosítod a DUUMBI fájlokat, a módosításokat nyilvánossá kell tenni, de saját zárt kóddal szabadon kombinálható

### 3.2 Monetizációs Pontok — Mikor Legyen Fizetős?

```
┌──────────────────────────────────────────────────────────────┐
│                    INGYENES (MPL-2.0)                        │
│                                                              │
│  ✅ duumbi CLI (init, build, run, check, describe)           │
│  ✅ Minden Op típus (Const, Add, Sub, stb.)                  │
│  ✅ Cranelift compiler                                       │
│  ✅ Lokális gráf vizualizáció (duumbi viz)                   │
│  ✅ 1 AI mutáció / perc (rate limit, saját API kulcs)        │
│  ✅ Lokális binary cache                                     │
│  ✅ Lokális modul import/export                               │
│  ✅ JSON-LD schema + validátor                                │
│  ✅ Community registry (publikus modulok böngészése)          │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│               DUUMBI PRO — $19/hó (fejlesztőnként)          │
│                                                              │
│  ⭐ Registry: privát modulok (max 50)                        │
│  ⭐ Registry: binary cache CDN (gyors letöltés)              │
│  ⭐ AI: korlátlan AI mutáció (DUUMBI hosted proxy)           │
│  ⭐ AI: prémium promptok (magasabb pontosság)                │
│  ⭐ Intent Engine: spec-driven fejlesztés                    │
│  ⭐ Telemetria dashboard (web)                               │
│  ⭐ Prioritásos support (48h válaszidő)                      │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│           DUUMBI TEAM — $49/hó (fejlesztőnként, min 5)      │
│                                                              │
│  ⭐⭐ Minden PRO funkció                                     │
│  ⭐⭐ Registry: korlátlan privát modulok                     │
│  ⭐⭐ Multi-ágens orchestráció                               │
│  ⭐⭐ Self-healing (telemetria → auto-patch javaslat)        │
│  ⭐⭐ Team admin panel + audit log                           │
│  ⭐⭐ SSO (SAML/OIDC)                                       │
│  ⭐⭐ SLA: 99.9% registry uptime                             │
│  ⭐⭐ 24h support válaszidő                                  │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│           DUUMBI ENTERPRISE — Egyedi árazás                  │
│                                                              │
│  ⭐⭐⭐ Minden TEAM funkció                                  │
│  ⭐⭐⭐ On-premise registry telepítés                        │
│  ⭐⭐⭐ Saját LLM integráció (lokális Llama, stb.)           │
│  ⭐⭐⭐ Dedikált support mérnök                              │
│  ⭐⭐⭐ Custom ágens fejlesztés                              │
│  ⭐⭐⭐ Air-gapped deployment                                │
│  ⭐⭐⭐ Compliance tanúsítványok (SOC2, ISO 27001)           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### 3.3 Bevételi Fázisok — Mikor Érdemes Fizetőssé Tenni?

| Időszak | Mérföldkő | Monetizáció | Indoklás |
|---------|-----------|-------------|----------|
| **Phase 0–3** (MVP) | Működő compiler + AI | ❌ Nincs bevétel | A közösség épül. Ingyenes használat terjedést biztosít. |
| **Phase 4–5** (Modul + Registry) | Registry él, >100 publikus modul | 💰 **PRO indulás** | A registry az első természetes fizetős pont: privát modulok + binary cache = egyértelmű érték. |
| **Phase 6** (Intent) | Spec-driven fejlesztés működik | 💰 PRO bővítés Intent-tel | Az Intent Engine jelentős AI költséget generál → indokolt a fizetős szint. |
| **Phase 7** (Multi-ágens) | Self-healing, MCP szerver | 💰 **TEAM indulás** | Csapat funkcionalitás + SLA = vállalati érték. |
| **12+ hónap** | Nagyvállalati érdeklődés | 💰 **ENTERPRISE** | On-premise, compliance, dedikált support. |

### 3.4 Költség Becslés és Break-Even

> [!note] A költségek platformfüggők — részletes összehasonlítást lásd: [[#4.2 Hosting Platform Összehasonlítás]].

**Break-even point:** ~30 PRO fizető fejlesztő ($19 × 30 = $570/hó) fedezi az alapköltségeket.

---

## 4. Fenntartandó Weboldalak és Szolgáltatások

> [!note] **Domain:** `duumbi.dev` — Namecheap regisztráció, Azure DNS zónakezelés.

| Oldal / Szolgáltatás     | URL                        | Funkció                          | Fázis          |
| ------------------------ | -------------------------- | -------------------------------- | -------------- |
| **Marketing oldal**      | `duumbi.dev`               | Landing page, dokumentáció, blog | Phase 4-től    |
| **Registry**             | `registry.duumbi.dev`      | Modul keresés, böngészés, profil | Phase 5-től    |
| **Registry API**         | `api.registry.duumbi.dev`  | REST/GraphQL API a CLI számára   | Phase 5-től    |
| **Dokumentáció**         | `docs.duumbi.dev`          | Spec, tutorial, API ref          | Phase 4-től    |
| **Telemetria Dashboard** | `telemetry.duumbi.dev`     | Trace vizualizáció (PRO+)        | Phase 7-től    |
| **Státusz oldal**        | `status.duumbi.dev`        | Service uptime monitoring        | Phase 5-től    |
| **GitHub Repository**    | `github.com/hgahub/duumbi` | Forráskód, issues, CI/CD         | Jelenleg is él |
| **Binary cache CDN**     | `cdn.duumbi.dev`           | Pre-compiled .o fájlok           | Phase 5-től    |

### 4.1 DNS Architektúra

```
Namecheap (domain registrar)
    │
    └── NS records → Azure DNS Zone (duumbi.dev)
                        ├── A/CNAME  duumbi.dev         → Marketing (static site)
                        ├── CNAME    docs.duumbi.dev     → Dokumentáció (static site)
                        ├── CNAME    registry.duumbi.dev → Registry Web Frontend
                        ├── CNAME    api.registry.duumbi.dev → Registry API
                        ├── CNAME    cdn.duumbi.dev      → Binary Cache CDN
                        ├── CNAME    telemetry.duumbi.dev → Dashboard
                        ├── CNAME    status.duumbi.dev   → Uptime monitor
                        └── TXT/MX   (email, SPF, DKIM) → Email protection
```

### 4.2 Hosting Platform Összehasonlítás

Az alábbi három hosting stratégiát hasonlítjuk össze. Mivel az Azure DNS már kezelésben van, az Azure integráció természetes előnyt élvez.

#### Opció A: Azure-központú (ajánlott)

> [!tip] **Előny:** Egyetlen cloud provider, Azure DNS már működik, egységes billing és monitoring, vállalati compliance (SOC2, ISO 27001) készen áll.

| Szolgáltatás | Azure erőforrás | Havi költség (becsült) |
|-------------|----------------|----------------------|
| Marketing + Docs | **Azure Static Web Apps** (ingyenes tier) | $0 |
| Registry API (Rust) | **Azure Container Apps** (consumption plan) | $15–50 |
| Database | **Azure Database for PostgreSQL — Flexible Server** (Burstable B1ms) | $15–25 |
| Object Storage | **Azure Blob Storage** (Hot tier + CDN) | $5–20 |
| Binary Cache CDN | **Azure CDN** (Standard Edgio/Microsoft) | $10–30 |
| Monitoring | **Azure Monitor + Application Insights** (basic) | $0–15 |
| DNS | **Azure DNS** (már fut) | ~$1 |
| Secrets | **Azure Key Vault** | $0–5 |
| **Összesen** | | **$46–146/hó** |

**Azure-specifikus előnyök:**
- Azure Static Web Apps: beépített GitHub Actions CI/CD, global CDN, Custom domain + SSL automatikus
- Azure Container Apps: serverless Rust containers, scale-to-zero (nincs forgalom = nincs költség)
- Azure Blob Storage: egress is olcsó a CDN profillal
- Egységes Azure RBAC és managed identity — nincs API key kezelés a szolgáltatások között
- Ha ENTERPRISE tier jön: Azure AD integráció SSO-hoz azonnal kész

#### Opció B: Cloudflare + Fly.io (legtakarékosabb)

| Szolgáltatás | Platform | Havi költség (becsült) |
|-------------|----------|----------------------|
| Marketing + Docs | **Cloudflare Pages** (ingyenes) | $0 |
| Registry API (Rust) | **Fly.io** (shared-1x-cpu, 256MB) | $5–15 |
| Database | **Neon** (PostgreSQL serverless, free tier → pro) | $0–19 |
| Object Storage | **Cloudflare R2** ($0 egress!) | $5–15 |
| Binary Cache CDN | **Cloudflare R2 + CDN** | $0 (R2 egress-free) |
| Monitoring | **Grafana Cloud** (free tier) | $0 |
| DNS | Azure DNS (megtartjuk) | ~$1 |
| **Összesen** | | **$11–50/hó** |

**Előnyök:** Legolcsóbb, Cloudflare R2 egress-free, Fly.io-n Rust natívan fut.
**Hátrányok:** Több provider = több fiók, több billing, nincs egységes IAM. Enterprise compliance nehezebb.

#### Opció C: Vercel + Azure háttér (hibrid)

| Szolgáltatás | Platform | Havi költség (becsült) |
|-------------|----------|----------------------|
| Marketing + Docs | **Vercel** (hobby → pro) | $0–20 |
| Registry Web Frontend | **Vercel** (Next.js SSR) | $0–20 |
| Registry API (Rust) | **Azure Container Apps** | $15–50 |
| Database | **Azure PostgreSQL Flexible** | $15–25 |
| Object Storage | **Azure Blob Storage** | $5–20 |
| Binary Cache CDN | **Azure CDN** | $10–30 |
| Monitoring | **Azure Monitor** | $0–15 |
| DNS | Azure DNS | ~$1 |
| **Összesen** | | **$46–181/hó** |

**Előnyök:** Vercel kiváló DX (developer experience) a marketing oldalhoz és a frontendhez, instant preview deploys, beépített analytics.
**Hátrányok:** Vercel Pro ($20/hó/tag) drága lehet. A Registry API Rustban van, Vercel Rust-ot csak serverless function-ként támogatja (cold start). A backend és a frontend két külön platformon — komplexebb.

#### Összefoglaló Mátrix

| Szempont | Azure (A) | CF+Fly (B) | Vercel+Azure (C) |
|----------|-----------|-----------|------------------|
| **Havi költség (indulás)** | $46–146 | **$11–50** ✅ | $46–181 |
| **Havi költség (scale)** | Jó | Jó | Drágulhat |
| **Egységes billing** | **Igen** ✅ | Nem (3–4 fiók) | Nem (2 fiók) |
| **Azure DNS integráció** | **Natív** ✅ | Manuális CNAME | Manuális CNAME |
| **Rust backend támogatás** | **Container Apps** ✅ | **Fly.io** ✅ | Container Apps ✅ |
| **Static site DX** | Jó | Jó | **Kiváló** ✅ |
| **Enterprise compliance** | **Kész** ✅ | Nehéz | Részben |
| **CDN egress költség** | Mérsékelt | **$0 (R2)** ✅ | Mérsékelt |
| **Komplexitás** | Alacsony | Közepes | Közepes |
| **Scale-to-zero** | **Igen** ✅ | Nem (min 1 VM) | Igen (Vercel) |

> [!important] **Ajánlás:** Induláshoz **Opció A (Azure-központú)** az optimális, mert:
> 1. Az Azure DNS már fut — nincs extra CNAME konfiguráció
> 2. Egységes billing, monitoring, RBAC
> 3. Azure Container Apps scale-to-zero: nincs forgalom = nincs költség  
> 4. Ha ENTERPRISE szint jön, az Azure compliance (SOC2, ISO) és Azure AD SSO azonnal elérhető
> 5. Az indulási költség ($46–146) a 30 PRO felhasználó break-even pontnál bőven fedezett
>
> Ha a költségoptimalizálás kritikus (bootstrapping fázis), az **Opció B** a legolcsóbb, és később migrálható Azure-ra.

---

## 5. Versenytárs Elemzés és Pozícionálás

| Termék | Hasonlóság | DUUMBI Differenciáció |
|--------|------------|----------------------|
| **GitHub Copilot** | AI kódgenerálás | DUUMBI: gráf, nem szöveg → szintaxis hiba lehetetlen |
| **Cursor** | AI-first IDE | DUUMBI: nincs emberi nyelv a pipeline-ban, séma validáció |
| **Augment Intent** | Spec-driven fejlesztés | DUUMBI: a spec outputja gráf, nem szöveg → determinisztikus |
| **Replit Agent** | Full-stack AI fejlesztés | DUUMBI: natív bináris (nem interpretált), binary cache |
| **Unison Lang** | Content-addressed code | DUUMBI: JSON-LD (szabvány), nem custom AST |
| **MPS (JetBrains)** | Projectional editing | DUUMBI: gráf-natív, AI-first (MPS emberi szerkesztés-központú) |

**Egyedi pozíció:** DUUMBI az egyetlen rendszer, ahol:
1. A program reprezentáció egy W3C szabvány (JSON-LD)
2. AI generál, de séma validáció garantálja a helyességet
3. Nincs köztes emberi nyelv (gráf → gépi kód)
4. Binary cache tartalom-címzett (mint Nix, de szemantikus szinten)

---

## 6. Közösségépítés és Go-to-Market Stratégia

### 6.1 Phase 4–5 Időszak: Közösség Alapozás

| Csatorna                     | Tevékenység | KPI |
| ---------------------------- | -------------------------------------------------- | ---------------------- |
| **GitHub**                   | Open source fejlesztés, Issues, Discussions | ⭐ 500 stars (12 hónap) |
| **Discord**                  | Közösségi szerver, support, showcase | 200 aktív tag |
| **Blog** (`duumbi.dev/blog`) | Heti tech post (JSON-LD, szemantikus fordítás, AI) | 2 post/hó |
| **Hacker News / Reddit**     | Launch poszt Phase 5-nél (registry launch) | 1 front page hit |
| **YouTube**                  | Demo videók, tutorial sorozat | 10 videó |
| **Conference Talk**          | RustConf, Strange Loop, AI Engineering Summit | 1 talk/év |

### 6.2 Adoption Funnel

```
Kíváncsi fejlesztő → duumbi.dev (landing)
    │
    ▼
  Quickstart (5 perc) → "Wow, ez tényleg gráfból fordít binárist"
    │
    ▼
  Tutorial sorozat → saját kis projekt
    │
    ▼
  Registry → más moduljait használja → saját modult publikál
    │
    ▼
  PRO → privát modulok, Intent Engine
    │
    ▼
  TEAM → csapat bevonás, self-healing
```

---

## 7. Kockázatok és Mitigáció

| Kockázat | Valószínűség | Hatás | Mitigáció |
|----------|-------------|-------|-----------|
| JSON-LD túl bőbeszédű → fejlesztők elutasítják | Közepes | Magas | Projectional editing (Phase 8), `duumbi describe` jó legyen |
| AI pontosság nem elég (< 70%) | Alacsony | Kritikus | Prompt finomhangolás, fine-tuned modell, tool-use javítás |
| Versenytárs (Cursor, Copilot) hasonlót épít | Közepes | Közepes | Speed + JSON-LD szabvány differenciátor |
| Registry nem vonz elég modult | Közepes | Magas | Seed-eld: stdlib modulok (math, string, io, data structures) |
| Rust Cranelift függőség — breaking change | Alacsony | Közepes | Pin-eld a verziót, LLVM backend mint fallback (Phase 8) |

---

## 8. Hosszú Távú Vízió (18+ hónap)

> Ezek ötlet szintű irányok a Phase 7 sikere után.

| # | Irány | Leírás |
|---|-------|--------|
| 8a | **LLVM Backend** | Inkwell crate-tel LLVM IR generálás → fejlettebb optimalizáció, több target (WASM, RISC-V) |
| 8b | **Projectional Editing** | IDE plugin (VS Code / saját): gráf ↔ pszeudokód ↔ diagram nézet váltás |
| 8c | **Obsidian-szerű Vault** | .md + .xml + .jsonld hibrid fájlrendszer, cross-format wikilink-ek, daemon indexer |
| 8d | **DUUMBI Playground** | Böngészőben futó WASM-based compiler + vizualizátor (onboarding-hoz) |
| 8e | **IDE Plugin** | VS Code extension: syntax highlighting, schema autocomplete, live graph preview |
| 8f | **Importálás meglévő kódból** | Python/JS/Rust forráskód → JSON-LD gráf konverzió (AST parsing + AI segítség) |

---

## 9. Végrehajtási Ütemterv Összefoglaló

```
MVP befejezés
    │
    ├── Phase 4: Modulrendszer (6-8 hét)     — 100% ingyenes
    │   ├── Struct, Array, String típusok
    │   ├── Interface/Contract rendszer
    │   └── Import/Export mechanizmus
    │
    ├── Phase 5: Registry (8-10 hét)          — 💰 PRO indul
    │   ├── registry.duumbi.dev
    │   ├── Szemantikus keresés
    │   ├── Binary cache CDN
    │   └── duumbi publish/deps
    │
    ├── Phase 6: Intent Engine (6-8 hét)      — 💰 PRO bővül
    │   ├── Spec-driven fejlesztés
    │   ├── Coordinator Agent
    │   └── Párhuzamos ágens végrehajtás
    │
    ├── Phase 8: Registry Auth (4-6 hét)
    │   ├── GitHub OAuth2 + local password
    │   ├── Device code flow (CLI)
    │   └── Token management web UI
    │
    ├── Phase 9: Multi-ágens + Self-Healing (8-10 hét)  — 💰 TEAM indul
    │   ├── MCP szerver
    │   ├── 6 specializált ágens
    │   └── Telemetria → auto-repair
    │
    └── Phase 10+: Hosszú táv (18+ hónap)    — 💰 ENTERPRISE
        ├── LLVM backend
        ├── Projectional editing
        ├── Obsidian vault
        └── IDE plugin
```

---

## Kapcsolódó Dokumentumok

- [[DUUMBI - PRD]] — Hosszú távú vízió
- [[DUUMBI - MVP Specification]] — MVP specifikáció (Phase 0-3)
- [[DUUMBI - Task List]] — MVP feladat lista
- [[DUUMBI - Tools and Components]] — Technikai stack
- [[DUUMBI - Architecture Diagram]] — Architektúra
- [[DUUMBI - Glossary]] — Fogalomtár


---

> [!warning] **Updated 2026-03-14:** The phase numbers in sections 2 and 9 of this document reflect the original roadmap. The original Phase 9 (Multi-Agent & Self-Healing) has since been decomposed into Phases 9–14. See [[DUUMBI Roadmap Map]] for the current, authoritative roadmap. Phase numbering: Phase 4→4, Phase 5→7 (Registry), Phase 6→5 (Intent), Phase 8→8 (Auth), and the new Phases 9–14 replace the old Phase 9.
