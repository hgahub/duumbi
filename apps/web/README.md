# 🏠 Duumbi Web App

Ez a **Duumbi** platform fő felhasználói felülete (Frontend), amely **React** és **TypeScript** alapokon nyugszik. A projekt célja egy modern, AI-vezérelt ingatlankeresési élmény biztosítása.

## 🛠️ Tech Stack

- **Framework:** [React](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/)
- **Build Tool:** [Vite](https://vitejs.dev/)
- **Hosting:** Azure Static Web Apps (SWA)
- **Styling:** [Tailwind CSS](https://tailwindcss.com/) (Utility-first)
- **State Management:** React Context / Hooks (MVP fázisban)

## 🎨 Design System

A projekt egyedi Design System-et használ, amely a `libs/ui/components` könyvtárban található. A stílusok a Tailwind CSS konfigurációjára épülnek.

### Alapelvek

- **Egyszerűség:** Intuitív interfész, minimális tanulási görbe.
- **Megbízhatóság:** Professzionális, tiszta megjelenés.
- **AI-centrikus:** Az AI funkciók (pl. képfelismerés, árbecslés) természetes integrációja.

### Komponensek

A komponensek újrahasznosíthatók és a `libs/ui/components` alatt érhetők el.
További információk: [UI Library Documentation](../../libs/ui/README.md).

- **Gombok, Inputok:** Egységes stílusú űrlapelemek.
- **Kártyák:** Ingatlanhirdetések megjelenítése.
- **Layout:** Reszponzív elrendezések (Desktop, Tablet, Mobile).

### Adatmodellek

A közös típusdefiníciók a `libs/data/ts-models` könyvtárban találhatók: [TS Models Documentation](../../libs/data/ts-models/README.md).

## 🚀 Fejlesztés

A fejlesztői környezet indítása:

```bash
nx serve web
```

Ez elindítja a Vite fejlesztői szervert (általában a `http://localhost:4200` címen).

### Build

A produkciós build készítése:

```bash
nx build web
```

A kimenet a `dist/apps/web` könyvtárba kerül.

## 🧪 Tesztelés

Unit tesztek futtatása:

```bash
nx test web
```

E2E tesztek (ha vannak konfigurálva):

```bash
nx e2e web-e2e
```
