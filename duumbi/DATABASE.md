# 🗄️ Duumbi Adatbázis Dokumentáció

A Duumbi platform **PostgreSQL** adatbázist használ, amely az **Azure Database for PostgreSQL - Flexible Server** szolgáltatáson fut. A térinformatikai funkciókhoz a **PostGIS** kiterjesztést alkalmazzuk.

## 📐 Adatbázis Séma

A rendszer fő entitásai és kapcsolataik.

### 1. `users` (Felhasználók)

A regisztrált felhasználók alapadatai. Az authentikációt külső szolgáltató (Azure AD B2C) végzi, itt csak a profiladatokat és beállításokat tároljuk.

- `id`: UUID (Primary Key)
- `auth_id`: String (Azure AD B2C User ID)
- `email`: String (Unique)
- `role`: Enum ('user', 'agent', 'admin')
- `created_at`: Timestamp

### 2. `listings` (Hirdetések)

Az ingatlanhirdetések központi táblája. A rugalmasság érdekében a változatos tulajdonságokat JSONB formátumban tároljuk.

- `id`: UUID (Primary Key)
- `user_id`: UUID (Foreign Key -> users.id)
- `title`: String
- `price`: Integer (HUF)
- `location`: Geography(Point, 4326) (PostGIS koordináta)
- `address`: JSONB (Irányítószám, város, utca, házszám)
- `features`: JSONB (Szobák, méret, emelet, fűtés, stb.)
- `status`: Enum ('active', 'sold', 'archived')
- `created_at`: Timestamp

### 3. `property_history` (Történet)

Az ingatlanok ár- és státuszváltozásainak naplózása az AVM (Automated Valuation Model) tanításához és piaci elemzésekhez.

- `id`: UUID
- `listing_id`: UUID
- `old_price`: Integer
- `new_price`: Integer
- `change_date`: Timestamp
- `event_type`: Enum ('price_change', 'status_change')

### 4. `saved_searches` (Mentett Keresések)

A felhasználók által mentett keresési feltételek és értesítési beállítások.

- `id`: UUID
- `user_id`: UUID
- `criteria`: JSONB (Keresési paraméterek)
- `notification_frequency`: Enum ('daily', 'weekly', 'instant')

## 🔄 Migrációk

Az adatbázis sémaváltozásait a **Flyway** eszközzel kezeljük.

### Struktúra

A migrációs fájlok a `apps/core-api/database/migrations` (vagy hasonló) könyvtárban találhatók.

- **Formátum:** `V{verzió}__{leírás}.sql`
- **Példa:** `V1__init_schema.sql`, `V2__add_user_roles.sql`

### Folyamat

1. Új SQL migrációs fájl létrehozása.
2. Helyi tesztelés (`flyway migrate`).
3. Commit és Push.
4. A CI/CD pipeline automatikusan futtatja a migrációkat a célkörnyezetben.

## 🌍 PostGIS

A térinformatikai keresésekhez (pl. "ingatlanok 5 km-es körzetben") a PostGIS funkcióit használjuk.

**Példa lekérdezés:**

```sql
SELECT * FROM listings
WHERE ST_DWithin(
  location,
  ST_SetSRID(ST_MakePoint(19.0402, 47.4979), 4326),
  5000 -- távolság méterben
);
```
