---
tags:
  - project/duumbi
  - doc/infrastructure
  - topic/dns
status: active
---

# 🌐 DNS Configuration

This document describes the DNS setup for the `duumbi.io` domain, managed via **Pulumi** (Azure DNS) and pointing to various services.

## DNS Provider
- **Provider:** Azure DNS
- **Zone:** `duumbi.io`
- **IaC:** Managed by Pulumi in `infra/stack-platform.ts`

## Deployment Process

To apply DNS changes:

```bash
cd infra
pulumi up -s platform
```

Changes typically propagate within 5-15 minutes, but can take up to 48 hours globally.
