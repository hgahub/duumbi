/**
 * Azure Cognitive Services module for AI Vision
 * 
 * This module creates Azure AI Vision (Computer Vision) resources
 * for the Tagger module's image analysis functionality.
 */

import * as cognitiveservices from '@pulumi/azure-native/cognitiveservices';
import * as pulumi from '@pulumi/pulumi';

export interface CognitiveServicesArgs {
  resourceGroupName: pulumi.Input<string>;
  location: pulumi.Input<string>;
  nameSuffix: string;
  tags?: { [key: string]: string };
  sku?: 'F0' | 'S1'; // F0 = Free tier, S1 = Standard tier
}

export interface CognitiveServicesOutputs {
  visionAccountName: pulumi.Output<string>;
  visionEndpoint: pulumi.Output<string>;
  visionKey: pulumi.Output<string>;
}

/**
 * Create Azure AI Vision (Computer Vision) account
 */
export function createCognitiveServices(
  args: CognitiveServicesArgs
): CognitiveServicesOutputs {
  const { resourceGroupName, location, nameSuffix, tags, sku = 'F0' } = args;

  // Create Computer Vision account
  const visionAccount = new cognitiveservices.Account(
    `cv-duumbi-${nameSuffix}`,
    {
      accountName: `cv-duumbi-${nameSuffix}`,
      resourceGroupName,
      location,
      kind: 'ComputerVision',
      sku: {
        name: sku,
      },
      properties: {
        customSubDomainName: `cv-duumbi-${nameSuffix}`,
        publicNetworkAccess: 'Enabled',
      },
      tags: {
        ...tags,
        service: 'tagger',
        purpose: 'image-analysis',
      },
    }
  );

  // Get the primary key
  const visionKeys = pulumi
    .all([resourceGroupName, visionAccount.name])
    .apply(([rgName, accountName]) =>
      cognitiveservices.listAccountKeys({
        resourceGroupName: rgName,
        accountName: accountName,
      })
    );

  const visionKey = visionKeys.apply((keys) => keys.key1!);

  // Construct endpoint URL
  const visionEndpoint = visionAccount.properties.endpoint;

  return {
    visionAccountName: visionAccount.name,
    visionEndpoint: visionEndpoint,
    visionKey: visionKey,
  };
}

