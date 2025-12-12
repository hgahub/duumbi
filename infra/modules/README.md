# Pulumi Infrastructure Modules

Reusable Pulumi modules for Duumbi infrastructure.

## Modules

### cognitive-services.ts

Azure Cognitive Services module for AI Vision (Computer Vision).

**Purpose:** Create and manage Azure AI Vision resources for the Tagger module's image analysis functionality.

**Usage:**
```typescript
import { createCognitiveServices } from './modules/cognitive-services';

const cognitiveServices = createCognitiveServices({
  resourceGroupName: resourceGroup.name,
  location: 'westeurope',
  nameSuffix: 'staging',
  tags: { environment: 'staging' },
  sku: 'F0', // or 'S1' for production
});

// Outputs
export const visionEndpoint = cognitiveServices.visionEndpoint;
export const visionKey = pulumi.secret(cognitiveServices.visionKey);
```

**Parameters:**
- `resourceGroupName`: Azure resource group name
- `location`: Azure region (e.g., 'westeurope')
- `nameSuffix`: Environment suffix (e.g., 'staging', 'prod')
- `tags`: Optional resource tags
- `sku`: Pricing tier ('F0' = Free, 'S1' = Standard)

**Outputs:**
- `visionAccountName`: Computer Vision account name
- `visionEndpoint`: API endpoint URL
- `visionKey`: Primary API key (should be marked as secret)

**Resources Created:**
- Azure Cognitive Services Account (Computer Vision)
  - Name: `cv-duumbi-{nameSuffix}`
  - Kind: ComputerVision
  - Custom subdomain: `cv-duumbi-{nameSuffix}`
  - Public network access: Enabled

**Pricing:**
- **F0 (Free):** 5,000 transactions/month, 20 calls/min
- **S1 (Standard):** ~$1/1,000 transactions, 10 calls/sec

## Adding New Modules

1. Create new TypeScript file in `modules/`
2. Export interface for arguments
3. Export interface for outputs
4. Export creation function
5. Document in this README

**Example:**
```typescript
// modules/my-module.ts
export interface MyModuleArgs {
  resourceGroupName: pulumi.Input<string>;
  // ... other args
}

export interface MyModuleOutputs {
  resourceId: pulumi.Output<string>;
  // ... other outputs
}

export function createMyModule(args: MyModuleArgs): MyModuleOutputs {
  // Implementation
}
```

## Best Practices

1. **Use TypeScript interfaces** for type safety
2. **Mark secrets** with `pulumi.secret()`
3. **Add resource tags** for organization
4. **Use consistent naming** (`{service}-duumbi-{suffix}`)
5. **Document all parameters** and outputs
6. **Export outputs** for use in other stacks

## Testing

Test modules locally:
```bash
cd infra
pulumi preview
```

## References

- [Pulumi Azure Native](https://www.pulumi.com/registry/packages/azure-native/)
- [Azure Cognitive Services](https://learn.microsoft.com/en-us/azure/cognitive-services/)

