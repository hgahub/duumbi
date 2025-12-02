/**
 * Standard tags for all Azure resources in Duumbi
 */
export interface ResourceTags {
  [key: string]: string;
}

export interface TagOptions {
  environment: 'Production' | 'Staging' | 'Development' | 'Platform';
  owner?: string;
}

export function getTags(options: TagOptions): ResourceTags {
  return {
    Environment: options.environment,
    CostCenter: 'Duumbi',
    Owner: options.owner || 'Duumbi Team',
    ManagedBy: 'Pulumi',
    Project: 'Duumbi',
  };
}
