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

## Frontend (Vercel)

### Production
- **Domain:** `duumbi.io`
- **Record Type:** A
- **Value:** `76.76.21.21` (Vercel Anycast IP)

- **Domain:** `www.duumbi.io`
- **Record Type:** CNAME
- **Value:** `cname.vercel-dns.com`

### Staging
- **Domain:** `staging.duumbi.io`
- **Record Type:** CNAME
- **Value:** `cname.vercel-dns.com`

### Alternative Subdomains
- **Domain:** `app.duumbi.io`
- **Record Type:** CNAME
- **Value:** `cname.vercel-dns.com`
- **Purpose:** Alternative access point (can be removed if not needed)

## Email (PrivateEmail)

### MX Records
- `mx1.privateemail.com` (Priority: 10)
- `mx2.privateemail.com` (Priority: 10)

### SPF Record
- **Record Type:** TXT
- **Value:** `v=spf1 include:spf.privateemail.com ~all`

### DKIM Record
- **Record Type:** TXT
- **Subdomain:** `default._domainkey.duumbi.io`
- **Value:** (RSA public key for email authentication)

## Monitoring

### Status Page
- **Domain:** `status.duumbi.io`
- **Record Type:** CNAME
- **Value:** `statuspage.betteruptime.com`

## Verification Records

### Domain Verification (Azure/Vercel)
- **Record Type:** TXT
- **Value:** `4cv262m6mvm7718jlj7bq5tdr1mrb9lk`

### GitHub Verification
- **Subdomain:** `_github-challenge-duumbi-org.duumbi.io`
- **Record Type:** TXT
- **Value:** `afff4f34e4`

## Local Development Records

### Local Testing (Optional)
- `maildev.local.duumbi.io` → `127.0.0.1`
- `whoami.local.duumbi.io` → `127.0.0.1`

## Deployment Process

To apply DNS changes:

```bash
cd infra
pulumi up -s platform
```

Changes typically propagate within 5-15 minutes, but can take up to 48 hours globally.
