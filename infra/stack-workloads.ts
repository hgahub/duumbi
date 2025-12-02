import * as app from '@pulumi/azure-native/app';
import * as resources from '@pulumi/azure-native/resources';
import * as pulumi from '@pulumi/pulumi';
import { getTags } from './lib/tags';

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

// Create Resource Group for Workload (Environment-specific)
const resourceGroup = new resources.ResourceGroup(`rg-duumbi-${nameSuffix}`, {
  resourceGroupName: `rg-duumbi-${nameSuffix}`,
  location,
  tags,
});

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
const backendApp = new app.ContainerApp(`ca-backend-${nameSuffix}`, {
  resourceGroupName: resourceGroup.name,
  containerAppName: `ca-backend-${nameSuffix}`,
  managedEnvironmentId: environment.id,
  configuration: {
    ingress: {
      external: true,
      targetPort: 8000, // FastAPI default port
    },
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
        ],
      },
    ],
  },
  tags,
});

export const resourceGroupNameOutput = resourceGroup.name;
export const backendUrl = backendApp.configuration.apply(
  (config) => config?.ingress?.fqdn
);
