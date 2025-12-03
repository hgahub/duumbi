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

_(This is a placeholder. Please expand with specific implementation details.)_
