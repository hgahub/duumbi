# Duumbi Infrastructure

This directory contains the Infrastructure as Code (IaC) for the Duumbi MVP, built with **Pulumi** (TypeScript).

## Architecture Overview

We use a **Stack-Based** approach to manage resources for different environments, optimizing for cost and separation of concerns.

### 1. Persistent Stack (`persistent`)
Contains stateful and shared resources that should **always remain active**:
- **Resource Group**: `rg-duumbi-persistent`
- **Log Analytics Workspace**: `law-duumbi-mvp` (Centralized logging)
- **Cost**: ~$5-20/month (Estimated)

### 2. Platform Stack (`platform`)
Contains shared platform services required by workloads:
- **Resource Group**: `rg-duumbi-platform`
- **Azure Container Registry (ACR)**: Stores Docker images for all apps.
- **Cost**: Small fixed cost (Basic SKU).

### 3. Workload Stacks (`staging`, `production`)
Contains the application compute resources. These stacks depend on both `persistent` and `platform` stacks.

| Environment | Stack Name | Description | Cost Strategy |
|-------------|------------|-------------|---------------|
| **Staging** | `staging` | Mirror of production for testing. | **Scale-to-Zero** / Destroy when not in use. |
| **Production**| `production`| Live user-facing environment. | **Always On** / High Availability. |

---

## 🚀 Quick Start

### Prerequisites
- Azure CLI logged in (`az login`)
- Pulumi CLI installed
- Node.js & NPM installed
- Just command runner installed (optional, recommended)

### Using Just (Recommended)

We provide a `justfile` for convenient infrastructure management.

```bash
# See all available commands
just --list

# Check prerequisites
just check

# Setup everything (persistent + staging)
just setup

# Or setup individually
just setup-persistent
just setup-staging
just setup-production
```

### Using Pulumi Directly

If you prefer to use Pulumi commands directly:

**1. Setup Persistent Layer (One-time)**
```bash
pulumi stack select persistent --create
pulumi up
```

**2. Setup Platform Layer (One-time)**
```bash
pulumi stack select platform --create
pulumi up
```

**3. Deploy Staging Environment**
```bash
pulumi stack select staging --create
pulumi up
```

**3. Deploy Production Environment**
```bash
pulumi stack select production --create
pulumi up
```

---

## 💰 Cost Management (Staging)

To save costs on the Staging environment when it is not being used (e.g., weekends), you can destroy the compute resources while keeping the data/logs safe in the Persistent stack.

**Using Just:**
```bash
just destroy-staging  # Pause
just deploy-staging   # Resume
```

**Using Pulumi:**
```bash
# Pause
pulumi stack select staging
pulumi destroy

# Resume
pulumi stack select staging
pulumi up
```

---

## Project Structure

```
infra/
├── index.ts                 # Entry point (Stack Router)
├── lib/                     # Shared Utilities
│   └── tags.ts              # Tagging Logic
├── stack-persistent.ts      # Shared Resources (RG, Logs)
├── stack-platform.ts        # Platform Resources (ACR)
├── stack-workloads.ts       # Workload Resources (Container Apps) - used by Staging/Prod
├── Pulumi.yaml              # Project Configuration
└── Pulumi.*.yaml            # Stack-specific Configuration
```

## Troubleshooting

### "Stack not found"
If you see an error like `Stack 'staging' does not exist`, create it:
```bash
pulumi stack select staging --create
```

### "Cannot reference persistent stack outputs"
Ensure the `persistent` stack is deployed first, as other stacks read its outputs (Resource Group name, Workspace ID).
