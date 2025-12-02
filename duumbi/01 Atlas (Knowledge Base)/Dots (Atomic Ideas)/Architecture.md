---
tags:
  - project/duumbi
  - doc/architecture
  - product/stages/mvp
status: in-progress
note tags:
  - '[[duumbi-mvp]]'
created: 2025-10-29T14:46:00
updated: 2025-11-12T22:30:00
author: Heizer Gábor
---

# 🏗️ Duumbi Architecture

This document describes the high-level technical structure of the Duumbi platform. The system follows a **Modular Monolith** architecture and applies the **Modern Stack** (Hybrid) approach in the MVP phase for faster development.

## 🧩 System Overview

The platform runs on a hybrid infrastructure, leveraging the advantages of Vercel, Supabase, and Azure.

```mermaid
graph TD
    User[User / Browser] -->|HTTPS| Vercel[Vercel Edge Network]
    Vercel -->|Static| Web[apps/web - React (Vite)]

    subgraph Supabase["Supabase Platform"]
        Auth[GoTrue Auth]
        DB[(PostgreSQL)]
        Storage[Storage]
        Edge[Edge Functions]
    end

    subgraph Azure["Azure - ML & Background Services"]
        Backend[apps/backend - Python Modular Monolith]
    end

    Web -->|Supabase Client| Auth
    Web -->|Supabase Client| DB
    Web -->|Supabase Client| Storage
    Web -->|RPC| Edge
    Web -->|REST| Backend
    Backend -->|Read/Write| DB
```

## 📦 Components

### 1. Frontend (`apps/web`)

- **Type:** Single Page Application (SPA)
- **Tech:** React, TypeScript, Vite
- **Hosting:** **Vercel**
- **Role:** User interaction, display.

### 2. Backend & Data Layer (Supabase)

- **Database:** PostgreSQL (Supabase managed)
- **Auth:** Supabase Auth (Email, Social, Phone)
- **API:** Auto-generated REST/GraphQL API + Edge Functions
- **Storage:** Supabase Storage (S3 compatible)

### 3. Backend API (`apps/backend`)

- **Type:** Modular Monolith
- **Tech:** Python, FastAPI, Scikit-learn
- **Hosting:** **Azure Container Apps**
- **Role:** Centralized backend for AVM, Scraper, and Image Tagging.
  - **AVM Module:** Automatic property valuation.
  - **Scraper Module:** Data collection from external sources.
  - **Tagger Module:** Image analysis.

## ☁️ Infrastructure (IaC)

Infrastructure management is handled in a hybrid way:

- **Azure Resources:** **Pulumi** (TypeScript) (Container Apps, Azure AI Vision, Resource Groups).
- **Vercel & Supabase:** CLI or Dashboard based configuration for MVP speed (can be Pulumized later).

## 🔒 Security

- **Communication:** HTTPS everywhere.
- **Data Protection:** Supabase RLS (Row Level Security) to protect direct database access.
- **Secret Management:** **Doppler** is the single source of truth.
  - **Workflow:** Secrets are defined in Doppler and synced to:
    - **Vercel** (for Next.js frontend)
    - **Supabase** (for Database & Edge Functions)
    - **Azure Key Vault** (for Azure Container Apps & Functions)
  - **Local Dev:** Developers use `doppler run` to inject secrets.
  - **IaC:** Pulumi references secrets from Azure Key Vault (synced from Doppler).
