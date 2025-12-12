import * as app from '@pulumi/azure-native/app';
import * as authorization from '@pulumi/azure-native/authorization';
import * as keyvault from '@pulumi/azure-native/keyvault';
import * as managedidentity from '@pulumi/azure-native/managedidentity';
import * as resources from '@pulumi/azure-native/resources';
import * as pulumi from '@pulumi/pulumi';
import { getTags } from './lib/tags';
import { createCognitiveServices } from './modules/cognitive-services';

const stackName = pulumi.getStack();
const nameSuffix = stackName; // e.g., 'staging', 'production'
const isProduction = stackName === 'production';

// Define tags based on environment
const tags = getTags({
  environment: isProduction ? 'Production' : 'Staging',
});

const location = 'West Europe';

// Read values from persistent stack
const persistentStackRef = new pulumi.StackReference(
  `${pulumi.getOrganization()}/${pulumi.getProject()}/persistent`
);

const workspaceCustomerId = persistentStackRef.getOutput('workspaceIdOutput');
const workspaceSharedKey = persistentStackRef.getOutput('sharedKeyOutput');
const vaultName = persistentStackRef.getOutput('vaultNameOutput');
const vaultUrl = persistentStackRef.getOutput('vaultUrlOutput');

// Create Resource Group for Workload (Environment-specific)
const resourceGroup = new resources.ResourceGroup(`rg-duumbi-${nameSuffix}`, {
  resourceGroupName: `rg-duumbi-${nameSuffix}`,
  location,
  tags,
});

// Create Managed Identity for Container App
const managedIdentity = new managedidentity.UserAssignedIdentity(
  `id-backend-${nameSuffix}`,
  {
    resourceGroupName: resourceGroup.name,
    resourceName: `id-backend-${nameSuffix}`,
    location,
    tags,
  }
);

// Create Azure AI Vision (Computer Vision) for Tagger module
const cognitiveServices = createCognitiveServices({
  resourceGroupName: resourceGroup.name,
  location,
  nameSuffix,
  tags,
  sku: isProduction ? 'S1' : 'F0', // Free tier for staging, Standard for production
});

// Store Azure AI Vision key in Key Vault (requires persistent stack reference)
const visionKeySecret = new keyvault.Secret(
  `secret-tagger-vision-key-${nameSuffix}`,
  {
    resourceGroupName: persistentStackRef.getOutput('resourceGroupNameOutput'),
    vaultName: vaultName,
    secretName: `TAGGER-AZURE-VISION-KEY-${nameSuffix.toUpperCase()}`,
    properties: {
      value: cognitiveServices.visionKey,
    },
  }
);

// Grant Managed Identity access to Key Vault secrets
const keyVaultAccessPolicy = new keyvault.AccessPolicy(
  `kv-access-backend-${nameSuffix}`,
  {
    resourceGroupName: persistentStackRef.getOutput('resourceGroupNameOutput'),
    vaultName: vaultName,
    policy: {
      tenantId: managedIdentity.tenantId,
      objectId: managedIdentity.principalId,
      permissions: {
        secrets: ['get', 'list'],
      },
    },
  }
);

// Create a Container Apps Environment
const environment = new app.ManagedEnvironment(`cae-duumbi-${nameSuffix}`, {
  resourceGroupName: resourceGroup.name,
  environmentName: `cae-duumbi-${nameSuffix}`,
  location,
  appLogsConfiguration: {
    destination: 'log-analytics',
    logAnalyticsConfiguration: {
      customerId: workspaceCustomerId,
      sharedKey: workspaceSharedKey,
    },
  },
  tags,
});

// Create Container App (Backend Monolith)
const backendApp = new app.ContainerApp(
  `ca-backend-${nameSuffix}`,
  {
    resourceGroupName: resourceGroup.name,
    containerAppName: `ca-backend-${nameSuffix}`,
    managedEnvironmentId: environment.id,
    identity: {
      type: 'UserAssigned',
      userAssignedIdentities: [managedIdentity.id],
    },
    configuration: {
      ingress: {
        external: true,
        targetPort: 8000, // FastAPI default port
      },
      secrets: [
        {
          name: 'tagger-vision-key',
          value: cognitiveServices.visionKey,
        },
      ],
    },
    template: {
      containers: [
        {
          name: 'backend',
          image: 'mcr.microsoft.com/azuredocs/containerapps-helloworld:latest', // Placeholder until CI/CD is set up
          resources: {
            cpu: 0.5,
            memory: '1.0Gi',
          },
          env: [
            {
              name: 'PORT',
              value: '8000',
            },
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
    tags,
  },
  { dependsOn: [keyVaultAccessPolicy, visionKeySecret] }
);

// Exports
export const resourceGroupNameOutput = resourceGroup.name;
export const managedIdentityId = managedIdentity.id;
export const managedIdentityPrincipalId = managedIdentity.principalId;
export const backendUrl = backendApp.configuration.apply(
  (config) => config?.ingress?.fqdn
);

// Azure AI Vision outputs (for Doppler configuration)
export const visionAccountName = cognitiveServices.visionAccountName;
export const visionEndpoint = cognitiveServices.visionEndpoint;
export const visionKey = pulumi.secret(cognitiveServices.visionKey); // Encrypted in Pulumi state
export const visionKeySecretName = visionKeySecret.name;
