---
tags:
  - project/duumbi
  - doc/architecture
  - topic/auth
status: draft
---

# 🔐 Authentication Architecture

This document describes the authentication flow using Supabase Auth (GoTrue).

## Overview

- **Provider**: Supabase Auth
- **Methods**: Email/Password, Google OAuth, Phone (planned)
- **Token Handling**: JWT stored in cookies/localStorage (handled by Supabase Client)

## Flows

1.  **Sign Up**: User registers -> Supabase sends confirmation email.
2.  **Sign In**: User logs in -> Receives Access Token & Refresh Token.
3.  **Protected Routes**: Middleware checks for valid session.

### Technical Implementation

#### 1. Supabase Client

- **Library**: `@supabase/supabase-js`
- **Initialization**: Singleton instance created in `apps/web/src/lib/supabase.ts`.
- **Configuration**: Uses `VITE_SUPABASE_URL` and `VITE_SUPABASE_ANON_KEY` from environment variables.

#### 2. Auth Context

- **Path**: `apps/web/src/context/AuthContext.tsx`
- **State**: Manages `session` (Session | null) and `user` (User | null).
- **Provider**: Wraps the application in `main.tsx`.
- **Listener**: Subscribes to `supabase.auth.onAuthStateChange` to handle session updates (sign-in, sign-out, token refresh).

#### 3. UI Components

- **TopMenu**:
  - Conditionally renders "Sign In" / "Sign Up" buttons or User Avatar based on auth state.
  - User Avatar opens a dropdown menu with "Sign Out" option.
- **Login Page**:
  - Route: `/login`
  - Features: Email/Password login, Google OAuth (future).

#### 4. Environment Variables

**Doppler Configuration:**

Environments and configs are set up in Doppler:
- **Development**: `duumbi` project → `dev_web` config
- **Preview**: `duumbi` project → `preview_web` config
- **Production**: `duumbi` project → `prd_web` config

All environments contain:
```bash
VITE_SUPABASE_URL=https://qtwsnoiotldufnemcnvl.supabase.co
VITE_SUPABASE_ANON_KEY=<supabase-anon-key>
```

**Local Development:**

1. Set up Doppler in the web app directory:
   ```bash
   cd apps/web
   doppler setup --project duumbi --config dev_web
   ```

2. Download secrets to `.env.local`:
   ```bash
   doppler secrets download --no-file --format env > .env.local
   ```

3. Run the dev server:
   ```bash
   nx serve web
   # or with Doppler:
   doppler run -- nx serve web
   ```
