---
tags:
  - project/duumbi
  - doc/devops
  - topic/vercel-staging
status: active
---

# 🌐 Vercel Staging Environment Setup

This document provides step-by-step instructions for configuring the staging environment on Vercel.

## Prerequisites

- Vercel project (`duumbi-web`) already deployed to production (`duumbi.io`)
- `staging` branch created in GitHub repository
- `staging.duumbi.io` DNS record already pointing to Vercel (via Pulumi)

## Step-by-Step Setup

### Step 1: Add Staging Domain in Vercel Dashboard

1. **Go to Vercel Project Settings**
   - Navigate to: https://vercel.com/dashboard
   - Click on `duumbi-web` project
   - Go to **Settings** → **Domains**

2. **Add Staging Domain**
   - Click **Add Domain**
   - Enter: `staging.duumbi.io`
   - Verify the DNS records are correct (should show checkmark ✓)

3. **Link Staging Domain to `staging` Branch**
   - After domain is verified, click on `staging.duumbi.io`
   - Scroll down to **Git Branch**
   - Select: `staging` (from dropdown)
   - Save

### Step 2: Configure Environment Variables (if needed)

1. **Go to Settings → Environment Variables**
2. **Add variables for staging** (if different from production):
   - Mark as: **Staging** (if Vercel Pro tier)
   - Or add them globally and override per branch

### Step 3: Enable Deployment Protection for Staging (Optional but Recommended)

1. **Go to Settings → Deployment Protection**
2. **For Staging Environment:**
   - Enable **Password Protection** (Hobby tier available)
   - Set a staging password (e.g., `staging-dev-2024`)
   - This password will be required for any preview on the staging domain

### Step 4: Verify Configuration

1. **Monitor Deployment**
   - Go to **Deployments**
   - Wait for the first automatic deployment of the `staging` branch
   - Once successful, you should see: `staging.duumbi.io` → `staging` branch

2. **Test the Staging URL**
   ```bash
   curl -I https://staging.duumbi.io
   ```
   You should get a 200 OK response (or 401 if password-protected).

## Workflow

### For Development:
- Work on feature branches and create PRs against `main`
- Each PR gets an automatic **Preview URL** (e.g., `duumbi-web-git-feature.vercel.app`)
- Preview URLs are temporary and tied to the PR

### For Testing (Staging):
- Create PRs to `staging` branch for pre-release testing
- Or manually push to `staging` branch for continuous staging deployment
- Accessible via `staging.duumbi.io` with password protection

### For Production:
- Merge to `main` branch
- Automatic deployment to `duumbi.io` with password protection (until MVP release)
- Once ready for public launch, remove password protection from Vercel Settings

## Automation (GitOps)

Once configured, the following happens automatically:

| Branch | Domain | Deployment | Protection |
|--------|--------|------------|-----------|
| `main` | `duumbi.io` | Auto on push | Password (while MVP) |
| `staging` | `staging.duumbi.io` | Auto on push | Password (optional) |
| Feature PR branches | `duumbi-web-git-*.vercel.app` | Auto on PR | Private/accessible only to repo access |

## Troubleshooting

### Domain not verifying?
- Check DNS records in Azure Portal (via Pulumi)
- Verify DNS propagation: `dig staging.duumbi.io`
- Wait 5-15 minutes for DNS propagation

### Preview URL working but domain not?
- Ensure domain is **verified** in Vercel Settings
- Check that correct branch (`staging`) is selected for the domain
- Re-trigger deployment by pushing to `staging` branch

### Want to temporarily disable staging deployments?
- Go to **Settings → Git**
- Uncheck **Automatic Deployments** for the staging branch (not recommended for MVP)
- Or delete the domain from Vercel Settings

## Next Steps

1. Complete the steps above in Vercel Dashboard
2. Push a test change to `staging` branch: `git push origin staging`
3. Monitor the deployment in Vercel Dashboard
4. Access `staging.duumbi.io` and verify it loads correctly
5. Document any staging-specific environment variables in `.env.staging` (if applicable)
