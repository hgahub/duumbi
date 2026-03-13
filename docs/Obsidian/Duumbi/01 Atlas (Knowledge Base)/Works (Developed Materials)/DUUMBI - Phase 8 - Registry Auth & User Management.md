---
tags:
  - project/duumbi
  - milestone/phase-8
status: in-progress
github_milestone: 9
updated: 2026-03-13
---
# Phase 8 — Registry Auth & User Management

> **Kill Criterion:** Felhasználó GitHub OAuth-tal bejelentkezik a registry.duumbi.dev-en, generál API tokent, majd `duumbi registry login duumbi` device code flow-val hitelesít a CLI-ből.
> **Milestone:** [Phase 8: Registry Auth & User Management](https://github.com/hgahub/duumbi/milestone/9)
> **Fejlesztői ág:** `phase8/registry-auth`

<- Vissza: [[DUUMBI Roadmap Map]]

---

## Összefoglaló

Felhasználó regisztráció, bejelentkezés és API token kezelés a duumbi registry-hez. Két auth mód: **GitHub OAuth2** a globális registry-hez (`registry.duumbi.dev`), **username+password** a privát (Docker self-hosted) registry-khez.

## Architektúra

```
AUTH_MODE=github_oauth (globális)     AUTH_MODE=local_password (privát)
┌──────────────────────┐              ┌──────────────────────┐
│  "Sign in with       │              │  Username + Password │
│   GitHub" gomb       │              │  regisztrációs form  │
│        │             │              │        │             │
│        ▼             │              │        ▼             │
│  GitHub OAuth2       │              │  Argon2id hash       │
│  Authorization Code  │              │  SQLite users tábla  │
│        │             │              │        │             │
│        ▼             │              │        ▼             │
│  JWT cookie session  │              │  JWT cookie session  │
└──────────────────────┘              └──────────────────────┘
         │                                     │
         └──────────── Közös ─────────────────┘
                        │
              ┌─────────┴──────────┐
              │  Token Management  │
              │  /settings/tokens  │
              │  SHA-256 hash DB   │
              └────────────────────┘
```

### CLI Device Code Flow

```
CLI                              Server                          Browser
 │ POST /api/v1/auth/device/code  │                               │
 │──────────────────────────────>│                               │
 │  { user_code: "ABCD-1234" }   │                               │
 │<──────────────────────────────│                               │
 │ open browser ─────────────────────────────────────────────── >│
 │                                │    GET /device                │
 │                                │<──────────────────────────────│
 │                                │    (login first if needed)    │
 │                                │    POST /device {user_code}   │
 │                                │<──────────────────────────────│
 │ poll /api/v1/auth/device/token │                               │
 │──────────────────────────────>│                               │
 │  { token: "duu_...",           │                               │
 │    username: "heizergabor" }   │                               │
 │<──────────────────────────────│                               │
 │ → ~/.duumbi/credentials.toml  │                               │
```

## Web UI

### Nav bar

```
Bejelentkezés elott:
┌────────────────────────────────────────────────────┐
│  duumbi registry    [search...]    Publish [Sign in]│
└────────────────────────────────────────────────────┘

Bejelentkezés után:
┌────────────────────────────────────────────────────┐
│  duumbi registry    [search...]    Publish  [avatar]│
└────────────────────────────────────────────────────┘
                                              │
                                    ┌─────────┴─────────┐
                                    │ @heizergabor      │
                                    │ ────────────────  │
                                    │ API Tokens        │
                                    │ Sign out          │
                                    └───────────────────┘
```

### Token kezelés (/settings/tokens)

- Token lista: név, prefix (`duu_a1b2...`), created_at, last_used_at
- Új token: név megadás → egyszer mutatja a teljes tokent (copy gomb)
- Revoke gomb minden tokenhez

## Adatbázis séma (v2 migráció)

```sql
CREATE TABLE users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT NOT NULL UNIQUE,
    display_name  TEXT,
    avatar_url    TEXT,
    email         TEXT,
    password_hash TEXT,    -- NULL OAuth usereknél
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE oauth_accounts (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id       INTEGER NOT NULL REFERENCES users(id),
    provider      TEXT NOT NULL,
    provider_id   TEXT NOT NULL,
    access_token  TEXT,
    created_at    TEXT NOT NULL,
    UNIQUE(provider, provider_id)
);

CREATE TABLE device_codes (
    device_code   TEXT PRIMARY KEY,
    user_code     TEXT NOT NULL UNIQUE,
    status        TEXT NOT NULL DEFAULT 'pending',
    user_id       INTEGER REFERENCES users(id),
    token         TEXT,
    expires_at    TEXT NOT NULL,
    created_at    TEXT NOT NULL
);

-- tokens tábla bővítés
ALTER TABLE tokens ADD COLUMN user_id INTEGER REFERENCES users(id);
ALTER TABLE tokens ADD COLUMN token_name TEXT NOT NULL DEFAULT 'default';
ALTER TABLE tokens ADD COLUMN last_used_at TEXT;
```

## Security

| Téma | Megoldás |
|------|---------|
| CSRF | Double-submit cookie pattern + OAuth state paraméter |
| Token tárolás | SHA-256 hash a DB-ben, soha nyers token |
| Jelszó hash | Argon2id (19 MiB, 2 iter) |
| Cookie | `HttpOnly; Secure; SameSite=Lax; Max-Age=7d` |
| Rate limit | In-memory: login 5/min, register 3/10min |
| Device code | 15 perc lejárat, XXXX-XXXX format, háttér cleanup |

## Új endpointok

### API
- `GET /api/v1/auth/mode` — auth mód discovery
- `GET /api/v1/auth/verify` — token validáció
- `POST /api/v1/auth/device/code` — device code generálás
- `POST /api/v1/auth/device/token` — CLI poll

### Web
- `GET /auth/github` + `GET /auth/github/callback` — OAuth flow
- `GET/POST /login` — bejelentkezés
- `GET/POST /register` — regisztráció (local_password)
- `POST /logout` — kijelentkezés
- `GET/POST /device` — device code bevitel
- `GET /settings/tokens` + `POST /settings/tokens` + `POST /settings/tokens/{id}/revoke`

## Érintett fájlok

### duumbi-registry (szerver)
- `src/auth/{mod,jwt,oauth,device_code,password,session}.rs`
- `src/web/{mod,auth_routes,settings}.rs`
- `src/api/mod.rs`, `src/db/mod.rs`, `src/lib.rs`, `src/main.rs`
- `src/error.rs`, `src/types.rs`
- `templates/{base,login,register,device,device_success,settings_tokens,token_created}.html`
- `templates/style.css`
- `Cargo.toml`, `docker-compose.yml`

### duumbi (kliens)
- `src/cli/registry.rs` — device code flow, auth mode discovery

### duumbi-infra
- Pulumi secrets (GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET, JWT_SECRET)

## Fejlesztési trackek

| Track | Issues | Leírás |
|-------|--------|--------|
| A: Core Auth | #194-#198 | Migrációk, users, JWT, auth mode, verify |
| B: GitHub OAuth | #199-#202 | OAuth flow, oauth_accounts, device code, /device page |
| C: Local Password | #203 | Argon2 registration + login |
| D: Token Mgmt | #204-#206 | Hashed storage, naming, web UI |
| E: Web UI | #207-#209 | Nav bar, login/register pages, CSS |
| F: Security | #210-#212 | CSRF, rate limiting, cleanup task |
| G: CLI | #213 | Device code flow in duumbi CLI |
| H: Infra | #214-#215 | GitHub OAuth App, docker-compose |

### Végrehajtási sorrend

```
Track A (#194→#195→#196→#197,#198)
    │
    ├── Track B (#199→#200→#201→#202) ──┐
    ├── Track C (#203)                   ├── Track E (#207→#208→#209)
    └── Track D (#204→#205→#206) ───────┘
                                              │
                                    Track F (#210,#211,#212)
                                              │
                                    Track G (#213) + Track H (#214,#215)
```

## Függőségek

```
Phase 7 (Registry szerver) ──→ Phase 8 (Auth ráépül)
Phase 8 (Auth)             ──→ Phase 9 (Multi-Agent, Self-Healing)
```

## Új crate-ek (duumbi-registry)

- `argon2 = "0.5"` — jelszó hashing
- `rand = "0.8"` — token/code generálás
- `uuid = { version = "1", features = ["v4"] }` — session ID-k
- `cookie = "0.18"` — cookie parsing
- reqwest → dependencies-be mozgatás (GitHub API hívások)
