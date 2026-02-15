# Tagger Module - Azure Deployment Guide

## Overview

This guide covers deploying the Tagger module's Azure infrastructure using Pulumi and configuring secrets with Doppler.

## Prerequisites

- Pulumi CLI installed
- Azure CLI installed and logged in
- Doppler CLI installed (optional, for local development)
- Access to Doppler project
- Azure subscription with appropriate permissions

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Doppler (Secrets)                       │
│  - TAGGER_AZURE_VISION_ENDPOINT                            │
│  - TAGGER_AZURE_VISION_KEY                                 │
│  - TAGGER_* (all configuration)                            │
└────────────────────┬────────────────────────────────────────┘
                     │ Sync
                     ↓
┌─────────────────────────────────────────────────────────────┐
│              Azure Key Vault (Optional)                     │
│  - Stores synced secrets from Doppler                      │
└────────────────────┬────────────────────────────────────────┘
                     │ Reference
                     ↓
┌─────────────────────────────────────────────────────────────┐
│           Azure Container Apps (Backend)                    │
│  - Reads secrets via Managed Identity                      │
│  - Runs FastAPI backend with Tagger module                 │
└─────────────────────────────────────────────────────────────┘
                     ↑
                     │ Uses
┌─────────────────────────────────────────────────────────────┐
│         Azure AI Vision (Computer Vision)                   │
│  - Created by Pulumi                                        │
│  - Provides image analysis API                             │
└─────────────────────────────────────────────────────────────┘
```

## Deployment Steps

### Step 1: Deploy Azure Infrastructure with Pulumi

```bash
cd infra

# Select production stack
pulumi stack select production

# Preview changes
pulumi preview

# Deploy infrastructure
pulumi up
```

This will create:
- Resource Group (`rg-duumbi-production`)
- Azure AI Vision account (`cv-duumbi-production`)
- Container Apps Environment (`cae-duumbi-production`)
- Container App (`ca-backend-production`)

### Step 2: Get Azure AI Vision Credentials

```bash
# Get endpoint
pulumi stack output visionEndpoint

# Get API key (encrypted in Pulumi state)
pulumi stack output visionKey --show-secrets
```

**Example output:**
```
visionEndpoint: https://cv-duumbi-production.cognitiveservices.azure.com/
visionKey: 1234567890abcdef1234567890abcdef
```

### Step 3: Configure Secrets in Doppler

#### Option A: Doppler Web UI

1. Go to [Doppler Dashboard](https://dashboard.doppler.com/)
2. Select project: `duumbi`
3. Select environment: `production`
4. Add/Update secrets:

```
TAGGER_AZURE_VISION_ENDPOINT=https://cv-duumbi-production.cognitiveservices.azure.com/
TAGGER_AZURE_VISION_KEY=<from-pulumi-output>
```

#### Option B: Doppler CLI

```bash
# Set secrets
doppler secrets set TAGGER_AZURE_VISION_ENDPOINT="https://cv-duumbi-production.cognitiveservices.azure.com/" --project duumbi --config production

doppler secrets set TAGGER_AZURE_VISION_KEY="<from-pulumi-output>" --project duumbi --config production
```

### Step 4: Sync Doppler to Azure Key Vault (Optional)

If using Azure Key Vault for Container Apps:

1. **Configure Doppler Integration:**
   - See [DOPPLER_INTEGRATION.md](./DOPPLER_INTEGRATION.md)

2. **Enable Auto-Sync:**
   - Doppler → Integrations → Azure Key Vault
   - Configure sync rules

3. **Verify Sync:**
   ```bash
   az keyvault secret list --vault-name kv-duumbi-persistent
   ```

### Step 5: Update Container App with Secrets

#### Option A: Environment Variables (Simple)

```bash
# Update Container App with environment variables
az containerapp update \
  --name ca-backend-production \
  --resource-group rg-duumbi-production \
  --set-env-vars \
    TAGGER_AZURE_VISION_ENDPOINT=secretref:tagger-vision-endpoint \
    TAGGER_AZURE_VISION_KEY=secretref:tagger-vision-key \
  --secrets \
    tagger-vision-endpoint="<endpoint>" \
    tagger-vision-key="<key>"
```

#### Option B: Key Vault Reference (Recommended)

Update `stack-workloads.ts`:

```typescript
const backendApp = new app.ContainerApp(`ca-backend-${nameSuffix}`, {
  // ... other config
  configuration: {
    secrets: [
      {
        name: 'tagger-vision-key',
        keyVaultUrl: 'https://kv-duumbi-persistent.vault.azure.net/secrets/TAGGER-AZURE-VISION-KEY',
        identity: managedIdentity.id,
      },
    ],
  },
  template: {
    containers: [
      {
        env: [
          {
            name: 'TAGGER_AZURE_VISION_ENDPOINT',
            value: cognitiveServices.visionEndpoint,
          },
          {
            name: 'TAGGER_AZURE_VISION_KEY',
            secretRef: 'tagger-vision-key',
          },
        ],
      },
    ],
  },
});
```

Then redeploy:
```bash
pulumi up
```

### Step 6: Verify Deployment

```bash
# Get backend URL
pulumi stack output backendUrl

# Test health endpoint
curl https://<backend-url>/api/tagger/health

# Expected response:
# {"status":"healthy","service":"tagger","azure_configured":true}
```

## Configuration Reference

### Required Secrets (Doppler)

```bash
# Azure AI Vision (from Pulumi)
TAGGER_AZURE_VISION_ENDPOINT=https://cv-duumbi-<env>.cognitiveservices.azure.com/
TAGGER_AZURE_VISION_KEY=<api-key>

# Image Constraints (optional, defaults provided)
TAGGER_MAX_IMAGE_SIZE_MB=10
TAGGER_MAX_IMAGE_WIDTH=4096
TAGGER_MAX_IMAGE_HEIGHT=4096
TAGGER_MIN_IMAGE_WIDTH=640
TAGGER_MIN_IMAGE_HEIGHT=480

# Quality Thresholds (optional)
TAGGER_MIN_QUALITY_SCORE=5.0
TAGGER_MIN_BRIGHTNESS_SCORE=4.0
TAGGER_MIN_SHARPNESS_SCORE=5.0

# Processing (optional)
TAGGER_TIMEOUT_SECONDS=30
TAGGER_MAX_RETRIES=3
```

## Pricing

### Azure AI Vision

**Free Tier (F0):**
- 5,000 transactions/month
- 20 calls/minute
- **Cost:** Free
- **Use for:** Development/Testing

**Standard Tier (S1):**
- Pay per 1,000 transactions
- 10 calls/second
- **Cost:** ~$1.00 per 1,000 transactions
- **Use for:** Production

**Estimated Monthly Cost (Production):**
- 10,000 images/month: ~$10/month
- 50,000 images/month: ~$50/month

## Troubleshooting

### Issue: "Access denied due to invalid subscription key"

**Check:**
```bash
# Verify secret in Doppler
doppler secrets get TAGGER_AZURE_VISION_KEY --project duumbi --config production

# Verify in Container App
az containerapp show \
  --name ca-backend-production \
  --resource-group rg-duumbi-production \
  --query "properties.template.containers[0].env"
```

### Issue: Container App can't access Key Vault

**Solution:**
```bash
# Grant Managed Identity access to Key Vault
az keyvault set-policy \
  --name kv-duumbi-persistent \
  --object-id <managed-identity-principal-id> \
  --secret-permissions get list
```

### Issue: Pulumi deployment fails

**Check:**
```bash
# Verify Azure login
az account show

# Check Pulumi state
pulumi stack --show-urns

# View detailed logs
pulumi up --logtostderr -v=9
```

## Rollback

If deployment fails:

```bash
# Rollback to previous Pulumi state
pulumi stack export > backup.json
pulumi cancel
pulumi refresh

# Or destroy and recreate
pulumi destroy
pulumi up
```

## Monitoring

### Check Azure AI Vision Usage

```bash
# View metrics
az monitor metrics list \
  --resource /subscriptions/<sub-id>/resourceGroups/rg-duumbi-production/providers/Microsoft.CognitiveServices/accounts/cv-duumbi-production \
  --metric "TotalCalls"
```

### Check Container App Logs

```bash
# Stream logs
az containerapp logs show \
  --name ca-backend-production \
  --resource-group rg-duumbi-production \
  --follow
```

## Next Steps

1. ✅ Deploy infrastructure with Pulumi
2. ✅ Configure secrets in Doppler
3. ✅ Verify deployment
4. 🔄 Set up CI/CD pipeline
5. 🔄 Configure monitoring and alerts
6. 🔄 Set up secret rotation

## References

- [Pulumi Azure Native](https://www.pulumi.com/registry/packages/azure-native/)
- [Doppler Documentation](https://docs.doppler.com/)
- [Azure AI Vision Pricing](https://azure.microsoft.com/en-us/pricing/details/cognitive-services/computer-vision/)
- [Container Apps Secrets](https://learn.microsoft.com/en-us/azure/container-apps/manage-secrets)

