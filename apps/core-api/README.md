# 🧠 Duumbi Core API

A **Core API** a Duumbi platform központi backend szolgáltatása, amely az üzleti logikát, a felhasználókezelést és az adatbázis-műveleteket végzi.

## 🛠️ Tech Stack

- **Runtime:** [Node.js](https://nodejs.org/)
- **Framework:** [Fastify](https://www.fastify.io/) vagy [Express](https://expressjs.com/) (TBD)
- **Language:** TypeScript
- **Database:** PostgreSQL + PostGIS
- **Shared Models:** [TS Models Documentation](../../libs/data/ts-models/README.md)
- **ORM/Query Builder:** (TBD - Jelenleg natív SQL vagy query builder használata javasolt a teljesítmény miatt)
- **Migration Tool:** Flyway (SQL alapú migrációk)

## 🗄️ Adatbázis

Az alkalmazás **PostgreSQL** adatbázist használ **PostGIS** kiterjesztéssel a térinformatikai adatok kezelésére.

### Főbb Entitások

- **`users`**: Felhasználói fiókok (Azure AD B2C integrációval).
- **`listings`**: Ingatlanhirdetések (JSONB mezőkkel a rugalmasságért).
- **`property_history`**: Ár- és státuszváltozások követése.
- **`saved_searches`**: Mentett keresések és értesítések.

A részletes adatbázis séma a `docs/DATABASE.md` fájlban található.

## 🔌 API Végpontok (Tervezett)

### Auth

- A hitelesítés az Azure AD B2C-n keresztül történik, az API JWT tokeneket validál.

### Listings

- `GET /api/listings`: Hirdetések keresése és listázása (szűrés, lapozás).
- `GET /api/listings/:id`: Egy hirdetés részletei.
- `POST /api/listings`: Új hirdetés feladása.
- `PUT /api/listings/:id`: Hirdetés módosítása.

### Search

- `POST /api/search`: Összetett keresés (geo-spatial query-k).

## 🚀 Fejlesztés

A fejlesztői szerver indítása:

```bash
nx serve core-api
```

Ez elindítja a Node.js szervert watch módban.

### Build

```bash
nx build core-api
```

A kimenet a `dist/apps/core-api` könyvtárba kerül.
