# 🚀 Vercel Staging Environment - Setup Checklist

## Summary
- ✅ Git `staging` branch created and pushed
- ✅ DNS record `staging.duumbi.io` configured (via Pulumi)
- ✅ Documentation: `docs/.../VercelStagingSetup.md`

---

## Vercel Dashboard Setup (Do This Now)

### 1️⃣ Add Staging Domain
- [ ] Go to: https://vercel.com/dashboard → `duumbi-web` project
- [ ] Navigate to: **Settings → Domains**
- [ ] Click: **Add Domain**
- [ ] Enter: `staging.duumbi.io`
- [ ] Wait for DNS verification ✓

### 2️⃣ Link Domain to `staging` Branch
- [ ] Click on `staging.duumbi.io` domain
- [ ] Scroll to: **Git Branch**
- [ ] Select: `staging` branch from dropdown
- [ ] **Save** changes

### 3️⃣ Enable Password Protection (Optional)
- [ ] Go to: **Settings → Deployment Protection**
- [ ] Enable: **Password Protection**
- [ ] Set password (e.g., `staging-dev-2024`)
- [ ] This protects both `staging.duumbi.io` and preview URLs

### 4️⃣ Verify Deployment
- [ ] Go to: **Deployments**
- [ ] Look for first auto-deployment from `staging` branch
- [ ] Once green ✓, test: `https://staging.duumbi.io`

---

## Git Workflow (After Setup)

### Development
```bash
# Create feature branch from main
git checkout main
git pull origin main
git checkout -b feat/your-feature
# ... make changes ...
git push origin feat/your-feature
# → Creates PR with automatic Preview URL
```

### Testing in Staging
```bash
# Option 1: Push directly to staging
git checkout staging
git merge feat/your-feature
git push origin staging
# → Deploys to staging.duumbi.io automatically

# Option 2: Create PR to staging branch
# ... instead of main, open PR against staging
```

### Deploy to Production
```bash
# Merge to main
git checkout main
git pull origin main
git merge feat/your-feature
git push origin main
# → Deploys to duumbi.io automatically (with password)
```

---

## Environment Details

| Environment | Branch | Domain | Auto-Deploy | Status |
|-------------|--------|--------|-------------|--------|
| Production | `main` | `duumbi.io` | On push | Password protected |
| Staging | `staging` | `staging.duumbi.io` | On push | Optional password |
| Preview | Feature PRs | `duumbi-web-git-*.vercel.app` | On PR | Private |

---

## For More Details
See: `docs/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/VercelStagingSetup.md`

---

## Questions?
- DNS issues? Check: `docs/.../DNSConfiguration.md`
- Vercel config? Check: `vercel.json` in repo root
- Architecture? Check: `docs/.../Architecture.md`
