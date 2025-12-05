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
- **Methods**: Email/Password, Google OAuth ✅, Phone (planned)
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
  - Features: Email/Password login, Google OAuth ✅

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

## Google OAuth Configuration

### Prerequisites

1. **Google Cloud Console Project**: Create a project at [Google Cloud Console](https://console.cloud.google.com/)
2. **OAuth 2.0 Credentials**: Client ID and Client Secret
3. **Supabase Project**: Access to Duumbi's Supabase dashboard

### Setup Steps

#### 1. Google Cloud Console Configuration

**Create OAuth Consent Screen:**
- Navigate to: APIs & Services → OAuth consent screen
- User Type: **External** (for public access)
- App Information:
  - App name: `Duumbi`
  - User support email: Your email
  - Developer contact: Your email
- Authorized domains: `duumbi.io`, `supabase.co`
- Scopes: Default (email, profile) - automatically included
- Publishing status: **Testing** (for development/staging)

**Create OAuth 2.0 Credentials:**
- Navigate to: APIs & Services → Credentials
- Create Credentials → OAuth 2.0 Client ID
- Application type: **Web application**
- Name: `Duumbi Web App`
- Authorized redirect URIs:
  ```
  https://qtwsnoiotldufnemcnvl.supabase.co/auth/v1/callback
  ```
- Save and note down **Client ID** and **Client Secret**

#### 2. Supabase Configuration

**Enable Google Provider:**
1. Go to [Supabase Dashboard](https://supabase.com/dashboard)
2. Select project: `qtwsnoiotldufnemcnvl`
3. Navigate to: Authentication → Providers
4. Find "Google" and click to enable
5. Enter:
   - **Client ID**: From Google Cloud Console
   - **Client Secret**: From Google Cloud Console
6. Save configuration

**Redirect URLs:**
Supabase automatically handles redirect URLs. The callback URL format is:
```
https://<project-ref>.supabase.co/auth/v1/callback
```

For Duumbi: `https://qtwsnoiotldufnemcnvl.supabase.co/auth/v1/callback`

### Frontend Implementation

**Component:** `apps/web/src/components/GoogleOAuthButton.tsx`
- Reusable OAuth button component
- Official Google branding (4-color logo)
- Dark mode support
- Loading and error states

**Integration:** `apps/web/src/pages/Login.tsx`
- Google OAuth button below email/password form
- Visual "OR" divider
- Shared error message display

**Authentication Flow:**
1. User clicks "Continue with Google"
2. Redirects to Google consent screen
3. User grants permissions
4. Google redirects back to app with auth code
5. Supabase exchanges code for session
6. AuthContext updates with new session
7. User is logged in

### Testing

**Development:**
```bash
nx serve web
```

Navigate to http://localhost:4200/login and click "Continue with Google"

**Test Users (while in Testing mode):**
- Add test users in Google Cloud Console → OAuth consent screen → Test users
- Maximum 100 test users allowed in Testing mode

### Production Deployment

**Before going live:**
1. Submit OAuth app for verification in Google Cloud Console
2. Move from "Testing" to "Production" status
3. Verify all redirect URLs are correct for production domain
4. Test OAuth flow on production URL

### Troubleshooting

**Issue: "Redirect URI mismatch"**
- **Cause**: Redirect URL in Google Console doesn't match Supabase callback URL
- **Solution**: Verify exact match: `https://qtwsnoiotldufnemcnvl.supabase.co/auth/v1/callback`

**Issue: "Access blocked: This app's request is invalid"**
- **Cause**: OAuth consent screen not properly configured
- **Solution**: Complete all required fields in OAuth consent screen

**Issue: "OAuth popup blocked"**
- **Cause**: Browser blocking popup windows
- **Solution**: Allow popups for the domain or use redirect flow (default)

**Issue: "User already exists"**
- **Cause**: Email already registered with email/password
- **Solution**: This is expected behavior - OAuth will link to existing account

**Issue: "App not verified"**
- **Cause**: App still in Testing mode
- **Solution**: Add user as test user, or submit for verification for production

### Security Considerations

- ✅ **CSRF Protection**: Handled automatically by Supabase via state parameter
- ✅ **Token Storage**: JWT stored in httpOnly cookies (secure by default)
- ✅ **Redirect Validation**: Only whitelisted URLs in Google Console work
- ✅ **Minimal Scopes**: Only email and basic profile requested
- ✅ **Rate Limiting**: Supabase has built-in rate limiting for auth endpoints

### Multi-Language Support

OAuth button text is translated in all supported languages:
- English: "Continue with Google"
- German: "Mit Google fortfahren"
- Hungarian: "Folytatás Google-lal"
- Spanish: "Continuar con Google"
- Polish: "Kontynuuj z Google"
- Italian: "Continua con Google"
