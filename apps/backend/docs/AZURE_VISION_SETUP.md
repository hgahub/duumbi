# Azure AI Vision Setup Guide

## Overview

The Tagger module uses Azure AI Vision API for advanced image analysis including:
- Object detection
- Tag generation
- Caption generation
- Scene understanding

## Prerequisites

- Azure subscription
- Azure AI Vision resource created

## Step 1: Create Azure AI Vision Resource

### Using Azure Portal:

1. Go to [Azure Portal](https://portal.azure.com)
2. Click "Create a resource"
3. Search for "Computer Vision" or "Azure AI Vision"
4. Click "Create"
5. Fill in the details:
   - **Subscription**: Your Azure subscription
   - **Resource group**: Create new or use existing
   - **Region**: Choose closest to your users (e.g., `westeurope`)
   - **Name**: e.g., `duumbi-vision-prod`
   - **Pricing tier**: 
     - **Free (F0)**: 20 calls/min, 5K calls/month (for testing)
     - **Standard (S1)**: 10 calls/sec, pay-as-you-go (for production)
6. Click "Review + Create"
7. Click "Create"

### Using Azure CLI:

```bash
# Login to Azure
az login

# Create resource group (if needed)
az group create --name duumbi-rg --location westeurope

# Create Computer Vision resource
az cognitiveservices account create \
  --name duumbi-vision-prod \
  --resource-group duumbi-rg \
  --kind ComputerVision \
  --sku F0 \
  --location westeurope \
  --yes
```

## Step 2: Get API Credentials

### Using Azure Portal:

1. Navigate to your Computer Vision resource
2. Go to "Keys and Endpoint" in the left menu
3. Copy:
   - **Endpoint**: e.g., `https://duumbi-vision-prod.cognitiveservices.azure.com/`
   - **Key 1** or **Key 2**: Your API key

### Using Azure CLI:

```bash
# Get endpoint
az cognitiveservices account show \
  --name duumbi-vision-prod \
  --resource-group duumbi-rg \
  --query properties.endpoint \
  --output tsv

# Get keys
az cognitiveservices account keys list \
  --name duumbi-vision-prod \
  --resource-group duumbi-rg
```

## Step 3: Configure Environment Variables

### Local Development (.env file):

Create or update `apps/backend/.env`:

```bash
# Azure AI Vision Configuration
TAGGER_AZURE_VISION_ENDPOINT=https://your-resource-name.cognitiveservices.azure.com/
TAGGER_AZURE_VISION_KEY=your-api-key-here
```

### Production (Azure Container Apps):

Set environment variables in your deployment:

```bash
# Using Azure CLI
az containerapp update \
  --name duumbi-backend \
  --resource-group duumbi-rg \
  --set-env-vars \
    TAGGER_AZURE_VISION_ENDPOINT=https://your-resource-name.cognitiveservices.azure.com/ \
    TAGGER_AZURE_VISION_KEY=secretref:azure-vision-key
```

## Step 4: Test the Integration

Run the test script:

```bash
cd apps/backend
uv run python -c "
from src.tagger.azure_client import AzureVisionService
import asyncio

async def test():
    service = AzureVisionService()
    result = await service.analyze_image_url(
        'https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg'
    )
    print('Tags:', [tag['name'] for tag in result['tags'][:5]])
    print('Caption:', result['caption']['text'])

asyncio.run(test())
"
```

Expected output:
```
Tags: ['grass', 'outdoor', 'nature', 'sky', 'field']
Caption: A grassy field with a blue sky
```

## Pricing Information

### Free Tier (F0):
- **Transactions**: 5,000 per month
- **Rate**: 20 calls per minute
- **Cost**: Free
- **Best for**: Development and testing

### Standard Tier (S1):
- **Transactions**: Pay per 1,000 transactions
- **Rate**: 10 calls per second
- **Cost**: ~$1.00 per 1,000 transactions
- **Best for**: Production

### Estimated Costs:

For a real estate platform with 1,000 listings/month, each with 10 images:
- **Total images**: 10,000/month
- **Cost**: ~$10/month (Standard tier)

## Security Best Practices

1. **Never commit API keys** to version control
2. **Use Azure Key Vault** for production secrets
3. **Rotate keys regularly** (every 90 days)
4. **Use Managed Identity** when possible (for Azure-hosted apps)
5. **Monitor usage** to detect anomalies

## Troubleshooting

### Error: "Access denied due to invalid subscription key"
- Check that `TAGGER_AZURE_VISION_KEY` is set correctly
- Verify the key hasn't been regenerated in Azure Portal

### Error: "Resource not found"
- Check that `TAGGER_AZURE_VISION_ENDPOINT` matches your resource endpoint
- Ensure the endpoint ends with a trailing slash

### Error: "Rate limit exceeded"
- You've hit the free tier limit (20 calls/min)
- Upgrade to Standard tier or wait for rate limit to reset

## Additional Resources

- [Azure AI Vision Documentation](https://learn.microsoft.com/en-us/azure/ai-services/computer-vision/)
- [Pricing Calculator](https://azure.microsoft.com/en-us/pricing/calculator/)
- [API Reference](https://learn.microsoft.com/en-us/rest/api/computervision/)

