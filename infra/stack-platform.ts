import * as containerregistry from '@pulumi/azure-native/containerregistry';
import * as network from '@pulumi/azure-native/network';
import * as resources from '@pulumi/azure-native/resources';
import * as pulumi from '@pulumi/pulumi';
import { getTags } from './lib/tags';

const location = 'West Europe';
const tags = getTags({ environment: 'Platform' });

// Create Resource Group for Platform
export const resourceGroup = new resources.ResourceGroup('rg-duumbi-platform', {
  resourceGroupName: 'rg-duumbi-platform',
  location,
  tags,
});

// Create Azure Container Registry
export const registry = new containerregistry.Registry('acrduumbi', {
  resourceGroupName: resourceGroup.name,
  registryName: 'acrduumbi', // Must be globally unique
  location: resourceGroup.location,
  sku: {
    name: 'Basic', // Most cost-effective for MVP
  },
  adminUserEnabled: true, // For simple auth in MVP
  tags,
});

// --- DNS Zone & Records ---

export const dnsZone = new network.Zone('duumbi-zone', {
  resourceGroupName: resourceGroup.name,
  zoneName: 'duumbi.io',
  location: 'Global',
  tags,
});

// Root A Record (@) - Vercel
export const rootRecord = new network.RecordSet('root-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: '@',
  recordType: 'A',
  ttl: 300,
  aRecords: [
    { ipv4Address: '216.198.79.1' }, // Vercel project-specific IP
  ],
});

// WWW CNAME Record - Vercel
export const wwwRecord = new network.RecordSet('www-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: 'www',
  recordType: 'CNAME',
  ttl: 300,
  cnameRecord: { cname: 'd1d445165161870d.vercel-dns-017.com' },
});

// App CNAME Record (app.duumbi.io) - Vercel
export const appRecord = new network.RecordSet('app-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: 'app',
  recordType: 'CNAME',
  ttl: 300,
  cnameRecord: { cname: 'cname.vercel-dns.com' },
});

// Staging CNAME Record (staging.duumbi.io) - Vercel
export const stagingRecord = new network.RecordSet('staging-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: 'staging',
  recordType: 'CNAME',
  ttl: 300,
  cnameRecord: { cname: 'd1d445165161870d.vercel-dns-017.com' },
});

// Status CNAME Record (status.duumbi.io)
export const statusRecord = new network.RecordSet('status-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: 'status',
  recordType: 'CNAME',
  ttl: 300,
  cnameRecord: { cname: 'statuspage.betteruptime.com' },
});

// MX Records (PrivateEmail)
export const mxRecord = new network.RecordSet('mx-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: '@',
  recordType: 'MX',
  ttl: 1800,
  mxRecords: [
    { exchange: 'mx1.privateemail.com', preference: 10 },
    { exchange: 'mx2.privateemail.com', preference: 10 },
  ],
});

// TXT Record (@) - SPF
export const txtSpfRecord = new network.RecordSet('txt-spf-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: '@',
  recordType: 'TXT',
  ttl: 1800,
  txtRecords: [
    { value: ['v=spf1 include:spf.privateemail.com ~all'] },
    { value: ['4cv262m6mvm7718jlj7bq5tdr1mrb9lk'] },
  ],
});

// TXT Record (GitHub Challenge)
export const txtGithubRecord = new network.RecordSet('txt-github-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: '_github-challenge-duumbi-org',
  recordType: 'TXT',
  ttl: 1800,
  txtRecords: [{ value: ['afff4f34e4'] }],
});

// TXT Record (DKIM - default._domainkey)
export const txtDkimRecord = new network.RecordSet('txt-dkim-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: 'default._domainkey',
  recordType: 'TXT',
  ttl: 1800,
  txtRecords: [
    {
      value: [
        'v=DKIM1;k=rsa;p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAtYXo1d08UwcX0fTqQKNIKchipsSu82F6DbmgmwPcBkpia3uR664Ra5N6OAtt9lEIGMZprqUVEhgtZKiZd8A98GWFjdypELI6Pju8nq8gqdzowl8o0UlYrjWYMe+fheT6WOFPpAMTDMDt2jlSfocO00N3VJIhINcM93cJyGGYsY8DRmir7VymZyuYKCYbNFTHGu',
        '9f1JnHgRKVgppPp7T5il77Cpr//H7sLcpUIRxomug05to4w3cMODDX588veWf+dP4Aymo5K5zrAKJ9r2Ukyd2/YmzHhxtP2kEObZlDUs22SZAjsZsksGMWVNNJdnwH+ry5hdM36jhm0+53jxS4HwIDAQAB',
      ],
    },
  ],
});

// Local Dev Records (Optional, but preserved from backup)
export const maildevRecord = new network.RecordSet('maildev-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: 'maildev.local',
  recordType: 'A',
  ttl: 300,
  aRecords: [{ ipv4Address: '127.0.0.1' }],
});

export const whoamiRecord = new network.RecordSet('whoami-record', {
  resourceGroupName: resourceGroup.name,
  zoneName: dnsZone.name,
  relativeRecordSetName: 'whoami.local',
  recordType: 'A',
  ttl: 300,
  aRecords: [{ ipv4Address: '127.0.0.1' }],
});

// --- Exports ---

// Get DNS Zone name servers using output function
const zoneData = network.getZoneOutput({
  resourceGroupName: resourceGroup.name,
  zoneName: 'duumbi.io',
});

// Export as Pulumi Stack Outputs (not just TS exports)
export const resourceGroupNameOutput = pulumi.output(resourceGroup.name);
export const registryNameOutput = pulumi.output(registry.name);
export const registryLoginServerOutput = pulumi.output(registry.loginServer);
export const nameServersOutput = pulumi.output(zoneData.nameServers);
