import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");

const readRepoFile = (path) => readFileSync(join(repoRoot, path), "utf8");

test("stage approval merges only reviewed spec PRs for Stage 7 and Stage 9 approvals", () => {
  const workflow = readRepoFile(".github/workflows/stage-approval.yml");

  assert.match(workflow, /contents:\s+write/);
  assert.match(workflow, /pulls\.merge/);
  assert.match(workflow, /specs\/DUUMBI-\$\{issueNumber\}\/PRODUCT\.md/);
  assert.match(workflow, /specs\/DUUMBI-\$\{issueNumber\}\/TECHNICAL\.md/);
  assert.match(workflow, /has no completed Copilot review/);
  assert.match(workflow, /successfulCopilotCheck/);
  assert.match(workflow, /has unresolved review threads/);
  assert.match(workflow, /must change only \$\{policy\.expectedPath\}/);
  assert.match(workflow, /Related to #\$\{issueNumber\}/);
  assert.match(workflow, /\.then\(\(runs\) => runs\.flat\(\)\.filter\(Boolean\)\)/);
});

test("product spec Slack review requests wait for review-clean PRs", () => {
  const workflow = readRepoFile(".github/workflows/spec-review-request.yml");

  assert.match(workflow, /PR is still draft/);
  assert.match(workflow, /Copilot review evidence has not completed/);
  assert.match(workflow, /successfulCopilotCheck/);
  assert.match(workflow, /unresolved review threads remain/);
  assert.match(workflow, /will be squash-merged on approval/);
  assert.match(workflow, /No product spec review notifications are ready to send/);
  assert.match(workflow, /\.then\(\(runs\) => runs\.flat\(\)\.filter\(Boolean\)\)/);
});

test("technical spec Slack review requests wait for review-clean PRs", () => {
  const workflow = readRepoFile(".github/workflows/technical-spec-review-request.yml");

  assert.match(workflow, /technical spec review requires a linked TECHNICAL\.md PR/);
  assert.match(workflow, /PR is still draft/);
  assert.match(workflow, /Copilot review evidence has not completed/);
  assert.match(workflow, /successfulCopilotCheck/);
  assert.match(workflow, /unresolved review threads remain/);
  assert.match(workflow, /will be squash-merged on approval/);
  assert.match(workflow, /No technical spec review notifications are ready to send/);
  assert.match(workflow, /\.then\(\(runs\) => runs\.flat\(\)\.filter\(Boolean\)\)/);
});
