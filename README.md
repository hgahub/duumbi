# 🏠 Duumbi Monorepo Structure

The architecture is based on a **Modern Stack (Hybrid)** approach, leveraging **Vercel**, **Supabase**, and **Azure Cloud Services**.

## ⚙️ Core Technology Stack

| Area               | Main Technology        | Service                                 |
| ------------------ | ---------------------- | --------------------------------------- |
| **Frontend**       | React, Typescript      | **Vercel** (Edge Network)               |
| **Backend**        | Supabase               | **Supabase** (Auth, DB, Edge Functions) |
| **Data/ML**        | Python (FastAPI/Flask) | **Azure** (Container Apps, AI Vision)   |
| **Database**       | PostgreSQL + PostGIS   | **Supabase** (Managed PostgreSQL)       |
| **Infrastructure** | Pulumi, TypeScript     | **Pulumi** (GitHub Actions)             |

## 📂 Monorepo Directory Structure

The repository follows the standard **Nx Workspace** conventions, separating code into deployable applications (`apps/`) and reusable libraries (`libs/`).

### 1. `apps/` (Deployable Projects)

These projects represent the individual, deployable microservices and applications.

| Directory               | Type                | Description                                                                                                                                                                           |
| ----------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`apps/web`**          | Frontend (React/TS) | The main user interface for listing creation and property search. Deployed to **Vercel**.                                                                                             |
| **`apps/scraper`**      | Python/Node.js App  | The **Data Aggregation Engine** prototype. Responsible for ethically collecting and aggregating public data (NAV, KSH, Land Registry) and advertising data.                           |
| **`apps/avm-api`**      | Python API (ML)     | Serves the **AVM v1.0** (Automated Valuation Model) as a managed endpoint. Called by `core-api` to provide estimated price ranges.                                                    |
| **`apps/image-tagger`** | Azure Function (TS) | Asynchronous service for **AI Image Processing**. Triggered by image uploads to Blob Storage. Calls Azure AI Vision/GPT-4o Vision to tag image features (e.g., "kitchen," "balcony"). |

### 2. `libs/` (Shared Code Libraries)

Reusable code modules to ensure consistency across services.

| Directory                 | Technology        | Purpose                                                                                                                            |
| ------------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **`libs/data/ts-models`** | Typescript        | Shared interfaces for key domain objects (Property, Listing, User). Used by the `web` and other projects.                          |
| **`libs/data/py-models`** | Python (Pydantic) | Shared data models for Python services (`scraper`, `avm-api`) to maintain data structure consistency with the PostgreSQL database. |
| **`libs/ui/components`**  | React/Typescript  | Centralized library for reusable UI components and the platform's design system.                                                   |

### 3. `infra/` (Infrastructure as Code - IaC)

Contains the **Pulumi** code required to provision and manage the Azure cloud environment.

| Directory            | Purpose                             | Tooling     |
| -------------------- | ----------------------------------- | ----------- |
| **`infra/`**         | Root for IaC code.                  | Pulumi      |
| **`infra/index.ts`** | Main entry point for Pulumi stacks. | Pulumi (TS) |

For detailed deployment strategy, see `docs/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/DeploymentStrategy.md`.
