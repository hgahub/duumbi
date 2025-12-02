---
tags:
  - project/duumbi
  - doc/devops
  - topic/deployment
status: draft
---

# 🚀 Deployment Strategy

This document outlines the deployment strategy for the Duumbi MVP, following a **Hybrid Architecture** (Vercel + Supabase + Azure) and **GitOps** principles.

## 🌍 Environments

We define three distinct environments to ensure stability and quality.

### 1. Development (Local)
The local development environment for day-to-day coding.
- **Frontend**: Localhost (`nx serve web`).
- **Backend**: Supabase Local CLI (`supabase start`) or linked Dev project.
- **Azure**: Local Python environment or on-demand Dev Stack.
- **Secrets**: Injected via `doppler run`.

### 2. Staging (`staging.duumbi.io`)
A mirror of production for testing and validation.
- **Frontend**: Vercel Preview Deployments (automatically built from PRs/branches).
- **Backend**: Supabase **Staging** Project.
- **Azure**: Pulumi `staging` stack (Azure Container Apps).
    - *Optimization*: Configured to **scale-to-zero** to minimize costs when not in use.
- **Trigger**: Git push to `staging` branch or tagged releases.

### 3. Production (`duumbi.io`)
The live, user-facing environment.
- **Frontend**: Vercel Production Deployment.
- **Backend**: Supabase **Production** Project.
- **Azure**: Pulumi `production` stack.
    - *Configuration*: High availability, production-tier resources.
- **Trigger**: Git push to `main`.

## 🏗️ Infrastructure (Hybrid Stack)

The infrastructure is managed using a hybrid approach, leveraging the best tools for each component.

### Frontend (Vercel)
- **React (Vite)** application hosting.
- **Deployment**: Managed via Vercel-GitHub integration.
- **Features**: Automatic preview URLs, DDoS protection, Edge Network.

### Backend (Supabase)
- **Database**: Managed PostgreSQL.
- **Auth**: Supabase Auth.
- **Storage**: Object storage for images/documents.
- **Management**: Migrations applied via GitHub Actions (GitOps).

### Compute & ML (Azure via Pulumi)
- **Tooling**: **Pulumi** (TypeScript) for Infrastructure as Code (IaC).
- **Resources**:
    - **Persistent Stack**: Shared resources (Resource Group, Log Analytics).
    - **Environment Stacks** (`staging`, `production`): Container Apps Environments, Container Apps (Python APIs), Azure Functions.
- **Strategy**: Separation of stateful (Persistent) and stateless (Environment) resources to allow flexible cost management.

## 🔄 CI/CD & GitOps

We prioritize automation using **GitHub Actions**.

- **Pipeline**:
    1.  **Lint & Test**: `nx affected --target=lint,test` runs on every PR.
    2.  **Build**: Docker images built and pushed to Azure Container Registry (ACR).
    3.  **Infra Update**: `pulumi up` runs automatically to apply infrastructure changes.
    4.  **Notify**: Deployment status sent to **Slack**.

## 🔑 Integrations & Secrets

- **Secret Management**: **Doppler** is the single source of truth.
    - Secrets are synced to Vercel, GitHub Actions, and Azure Key Vault/Container Apps.
- **Email**: **Resend.com** for transactional emails (SMTP/API).
- **Notifications**: **Slack** for infrastructure alerts and deployment notifications.

## 🚧 Gap Analysis (Roadmap to MVP)

The following steps are required to fully realize this strategy from the current state:

1.  **Containerization**:
    - [ ] Add `Dockerfile` for `apps/avm-api`.
    - [ ] Add `Dockerfile` for `apps/core-api`.
2.  **Infrastructure Refactor**:
    - [ ] Update `infra` to support `staging` and `production` stack names (migrating from the `temporary` concept).
    - [x] Configure DNS for `duumbi.io` and `staging.duumbi.io`.
3.  **CI/CD Setup**:
    - [ ] Create GitHub Actions workflows for `pulumi up`.
    - [x] Configure Vercel project and GitHub integration.
4.  **Integrations**:
    - [ ] Set up Resend.com account and verify domain.
    - [ ] Configure Slack incoming webhooks for notifications.
