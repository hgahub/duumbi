---
tags:
  - project/duumbi
  - doc/architecture
  - topic/database
status: draft
---

# 🗄️ Database Schema

This document outlines the PostgreSQL schema design for Duumbi.

## Core Tables

- `users`: Extends Supabase `auth.users`.
- `listings`: Main property listings table.
- `properties`: Physical property attributes (normalized).
- `images`: References to files in Storage.

## Relationships

- User -> Listings (1:N)
- Listing -> Property (1:1)
- Listing -> Images (1:N)

_(This is a placeholder. Please expand with ERD and specific column details.)_
