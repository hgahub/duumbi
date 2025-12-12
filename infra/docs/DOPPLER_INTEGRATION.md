# Doppler Integration Guide

## Overview

Doppler is used as the central secrets management system for the Duumbi platform. It can sync secrets to Azure Key Vault for use by Azure Container Apps.

## Architecture

```
Doppler (Source of Truth)
    ↓ (Sync)
Azure Key Vault
    ↓ (Reference)
Azure Container Apps
```

## Setup

### 1. Doppler Configuration

#### Create Doppler Project

```bash
# Install Doppler CLI
brew install dopplerhq/cli/doppler  # macOS
# or
curl -Ls https://cli.doppler.com/install.sh | sh  # Linux

# Login
doppler login

# Setup project
doppler setup
```

#### Configure Secrets in Doppler

Create the following secrets in Doppler for each environment (staging, production):

**Tagger Module Secrets:**
```bash
# Azure AI Vision
TAGGER_AZURE_VISION_ENDPOINT=https://cv-duumbi-staging.cognitiveservices.azure.com/
TAGGER_AZURE_VISION_KEY=<from-pulumi-output>

# Image Constraints
TAGGER_MAX_IMAGE_SIZE_MB=10
TAGGER_MAX_IMAGE_WIDTH=4096
TAGGER_MAX_IMAGE_HEIGHT=4096
TAGGER_MIN_IMAGE_WIDTH=640
TAGGER_MIN_IMAGE_HEIGHT=480

# Quality Thresholds
TAGGER_MIN_QUALITY_SCORE=5.0
TAGGER_MIN_BRIGHTNESS_SCORE=4.0
TAGGER_MIN_SHARPNESS_SCORE=5.0

# Processing
TAGGER_TIMEOUT_SECONDS=30
TAGGER_MAX_RETRIES=3
```

### 2. Azure Key Vault Sync

#### Enable Doppler → Azure Key Vault Integration

1. **In Doppler Dashboard:**
   - Go to Integrations
   - Select "Azure Key Vault"
   - Click "Add Integration"

2. **Configure Azure Service Principal:**
   ```bash
   # Create service principal for Doppler
   az ad sp create-for-rbac \
     --name "doppler-keyvault-sync" \
     --role "Key Vault Secrets Officer" \
     --scopes /subscriptions/<subscription-id>/resourceGroups/rg-duumbi-persistent/providers/Microsoft.KeyVault/vaults/kv-duumbi-persistent
   ```

3. **Add Credentials to Doppler:**
   - Tenant ID: `<from-sp-output>`
   - Client ID: `<from-sp-output>`
   - Client Secret: `<from-sp-output>`
   - Key Vault Name: `kv-duumbi-persistent`

4. **Configure Sync:**
   - Select secrets to sync
   - Map Doppler secret names to Key Vault secret names
   - Enable auto-sync

#### Manual Sync (Alternative)

If automatic sync is not available, use Doppler CLI:

```bash
# Export secrets from Doppler
doppler secrets download --no-file --format env > .env.doppler

# Upload to Azure Key Vault
while IFS='=' read -r key value; do
  az keyvault secret set \
    --vault-name kv-duumbi-persistent \
    --name "$key" \
    --value "$value"
done < .env.doppler
```

### 3. Pulumi Integration

#### Update Pulumi Stack Configuration

The Pulumi stack will:
1. Create Azure AI Vision resource
2. Store credentials in Pulumi secrets (encrypted)
3. Reference Doppler/Key Vault for runtime secrets

**Example: `stack-workloads.ts`**
```typescript
import { createCognitiveServices } from './modules/cognitive-services';

// Create Azure AI Vision
const cognitiveServices = createCognitiveServices({
  resourceGroupName: resourceGroup.name,
  location,
  nameSuffix,
  tags,
  sku: isProduction ? 'S1' : 'F0', // Free tier for staging
});

// Export for Doppler configuration
export const visionEndpoint = cognitiveServices.visionEndpoint;
export const visionKey = pulumi.secret(cognitiveServices.visionKey);
```

#### Get Pulumi Outputs for Doppler

```bash
# Get Azure AI Vision credentials
pulumi stack output visionEndpoint
pulumi stack output visionKey --show-secrets

# Add to Doppler
doppler secrets set TAGGER_AZURE_VISION_ENDPOINT="<endpoint>"
doppler secrets set TAGGER_AZURE_VISION_KEY="<key>"
```

### 4. Container Apps Configuration

#### Reference Key Vault Secrets

**Option A: Direct Key Vault Reference (Recommended)**
```typescript
const backendApp = new app.ContainerApp(`ca-backend-${nameSuffix}`, {
  // ... other config
  configuration: {
    secrets: [
      {
        name: 'tagger-azure-vision-key',
        keyVaultUrl: pulumi.interpolate`https://kv-duumbi-persistent.vault.azure.net/secrets/TAGGER-AZURE-VISION-KEY`,
        identity: managedIdentity.id, // Managed Identity with Key Vault access
      },
    ],
  },
  template: {
    containers: [
      {
        name: 'backend',
        env: [
          {
            name: 'TAGGER_AZURE_VISION_ENDPOINT',
            secretRef: 'tagger-azure-vision-endpoint',
          },
          {
            name: 'TAGGER_AZURE_VISION_KEY',
            secretRef: 'tagger-azure-vision-key',
          },
        ],
      },
    ],
  },
});
```

**Option B: Doppler CLI in Container**
```dockerfile
# In Dockerfile
FROM python:3.11-slim

# Install Doppler CLI
RUN apt-get update && apt-get install -y curl && \
    curl -Ls https://cli.doppler.com/install.sh | sh

# Use Doppler to inject secrets
ENTRYPOINT ["doppler", "run", "--"]
CMD ["uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "8000"]
```

## Workflow

### Development
```bash
# Local development with Doppler
doppler run -- uvicorn src.main:app --reload
```

### Staging/Production Deployment

1. **Update secrets in Doppler** (single source of truth)
2. **Doppler auto-syncs to Azure Key Vault**
3. **Container Apps pull from Key Vault** (via managed identity)
4. **No manual secret management needed**

## Security Best Practices

1. **Never commit secrets** to version control
2. **Use Doppler for all environments** (dev, staging, production)
3. **Enable Doppler audit logs** for secret access tracking
4. **Rotate secrets regularly** (every 90 days)
5. **Use Managed Identity** for Azure Key Vault access
6. **Restrict Key Vault access** with RBAC policies

## Troubleshooting

### Issue: Doppler sync not working
**Solution:** Check service principal permissions on Key Vault

### Issue: Container Apps can't access Key Vault
**Solution:** Verify Managed Identity has "Key Vault Secrets User" role

### Issue: Secrets not updating in Container Apps
**Solution:** Restart Container App after Key Vault update

## References

- [Doppler Documentation](https://docs.doppler.com/)
- [Azure Key Vault Integration](https://docs.doppler.com/docs/azure-key-vault)
- [Container Apps Secrets](https://learn.microsoft.com/en-us/azure/container-apps/manage-secrets)

