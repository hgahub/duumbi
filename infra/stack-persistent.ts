import * as authorization from '@pulumi/azure-native/authorization';
import * as keyvault from '@pulumi/azure-native/keyvault';
import * as operationalinsights from '@pulumi/azure-native/operationalinsights';
import * as pulumi from '@pulumi/pulumi';
import * as resources from '@pulumi/azure-native/resources';
import { getTags } from './lib/tags';

const location = 'West Europe';
// Use 'Platform' as the environment tag for persistent/shared infrastructure
const tags = getTags({ environment: 'Platform' });

// Get current client configuration for Tenant ID
const clientConfig = authorization.getClientConfigOutput();

// Get Doppler Service Principal Object ID from Pulumi config
const config = new pulumi.Config();
const dopplerSpObjectId = config.require('dopplerSpObjectId');

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
      {
        tenantId: clientConfig.tenantId,
        objectId: dopplerSpObjectId,
        permissions: {
          secrets: ['get', 'list', 'set', 'delete'],
        },
      },
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
