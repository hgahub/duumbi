import * as pulumi from '@pulumi/pulumi';

const stack = pulumi.getStack();

if (stack === 'persistent') {
  const persistent = require('./stack-persistent');
  // Export for Pulumi stack outputs
  module.exports = persistent;
} else if (stack === 'platform') {
  const platform = require('./stack-platform');
  // Export for Pulumi stack outputs
  module.exports = platform;
} else if (stack === 'staging' || stack === 'production') {
  const workloads = require('./stack-workloads');
  // Export for Pulumi stack outputs
  module.exports = workloads;
} else {
  throw new Error(
    `Unknown stack: ${stack}. Use 'persistent', 'staging', or 'production' stack.`
  );
}
