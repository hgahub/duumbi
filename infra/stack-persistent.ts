import * as operationalinsights from '@pulumi/azure-native/operationalinsights';
import * as resources from '@pulumi/azure-native/resources';
import * as pulumi from '@pulumi/pulumi';
import { getTags } from './lib/tags';

const location = 'West Europe';
// Use 'Platform' as the environment tag for persistent/shared infrastructure
const tags = getTags({ environment: 'Platform' });

// Create an Azure Resource Group
export const resourceGroup = new resources.ResourceGroup('rg-duumbi-persistent', {
  resourceGroupName: 'rg-duumbi-persistent',
  location,
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
