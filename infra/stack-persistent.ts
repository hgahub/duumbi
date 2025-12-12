import * as authorization from '@pulumi/azure-native/authorization';
import * as keyvault from '@pulumi/azure-native/keyvault';
import * as operationalinsights from '@pulumi/azure-native/operationalinsights';
import * as resources from '@pulumi/azure-native/resources';
import * as pulumi from '@pulumi/pulumi';
import { getTags } from './lib/tags';

const location = 'West Europe';
// Use 'Platform' as the environment tag for persistent/shared infrastructure
const tags = getTags({ environment: 'Platform' });

// Get current client configuration for Tenant ID
const clientConfig = authorization.getClientConfigOutput();

// Create an Azure Resource Group
export const resourceGroup = new resources.ResourceGroup('rg-duumbi-persistent', {
  resourceGroupName: 'rg-duumbi-persistent',
  location,
  tags,
});

// Create Key Vault for storing secrets (synced from Doppler)
export const vault = new keyvault.Vault('kv-duumbi-persistent', {
  resourceGroupName: resourceGroup.name,
  vaultName: 'kv-duumbi-persistent',
  location: resourceGroup.location,
  properties: {
    sku: {
      family: 'A',
      name: 'standard',
    },
    tenantId: clientConfig.tenantId,
    accessPolicies: [
      // Access policies will be managed via external tools (Doppler) or additional Pulumi resources
      // Initial deployment starts with empty policies to avoid circular dependencies or permission issues
    ],
    enableSoftDelete: true,
    softDeleteRetentionInDays: 90,
    enableRbacAuthorization: false, // Using Access Policies as per deployment guide
  },
  tags,
});

// Create a Log Analytics Workspace
export const workspace = new operationalinsights.Workspace('law-duumbi-mvp', {
  resourceGroupName: resourceGroup.name,
  workspaceName: 'law-duumbi-mvp',
  location: resourceGroup.location,
  sku: {
    name: 'PerGB2018',
  },
  retentionInDays: 30,
  tags,
});

export const sharedKeys = operationalinsights.getSharedKeysOutput({
  resourceGroupName: resourceGroup.name,
  workspaceName: workspace.name,
});

// Export values for use in other stacks
export const resourceGroupNameOutput = resourceGroup.name;
export const workspaceIdOutput = workspace.customerId;
export const sharedKeyOutput = sharedKeys.apply((keys) => keys.primarySharedKey!);
export const vaultNameOutput = vault.name;
export const vaultUrlOutput = vault.properties.vaultUri;
