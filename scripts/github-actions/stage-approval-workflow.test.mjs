import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");

const readRepoFile = (path) => readFileSync(join(repoRoot, path), "utf8");
const reviewerBotSuffixReplacement = /replace\(\s*\/\\\[bot\\\]\$\/\s*,\s*(["'])\1\s*\)/;

test("stage approval merges only reviewed spec PRs for Stage 7 and Stage 9 approvals", () => {
  const workflow = readRepoFile(".github/workflows/stage-approval.yml");

  assert.match(workflow, /pull_request:\s*\n\s+types:\s+\[closed\]/);
  assert.match(workflow, /github\.event\.pull_request\.merged == true/);
  assert.match(workflow, /Spec-only PR #\$\{prNumber\} was merged by/);
  assert.ok(workflow.includes("files[0].filename.match(/^specs\\/DUUMBI-(\\d+)\\/PRODUCT\\.md$/)"));
  assert.ok(workflow.includes("files[0].filename.match(/^specs\\/DUUMBI-(\\d+)\\/TECHNICAL\\.md$/)"));
  assert.match(workflow, /contents:\s+write/);
  assert.match(workflow, /pulls\.merge/);
  assert.match(workflow, /specs\/DUUMBI-\$\{issueNumber\}\/PRODUCT\.md/);
  assert.match(workflow, /specs\/DUUMBI-\$\{issueNumber\}\/TECHNICAL\.md/);
  assert.match(workflow, /required automated review evidence from/);
  assert.match(workflow, /requiredAutomatedReviewers/);
  assert.match(workflow, /Greptile is manual-only/);
  assert.match(workflow, /out of DUUMBI_REQUIRED_SPEC_REVIEWERS/);
  assert.match(workflow, /normalizeReviewerLogin/);
  assert.match(workflow, reviewerBotSuffixReplacement);
  assert.match(workflow, /has unresolved review threads/);
  assert.match(workflow, /must change only \$\{policy\.expectedPath\}/);
  assert.match(workflow, /Related to #\$\{issueNumber\}/);
  assert.match(workflow, /checkRuns\.push\(\.\.\.pageRuns\)/);
});

test("project-status workflow is merge-free and isolated from Stage 7/9 spec PR validation", () => {
  const workflow = readRepoFile(".github/workflows/project-status.yml");
  const stageApproval = readRepoFile(".github/workflows/stage-approval.yml");

  assert.match(workflow, /repository_dispatch:\s*\n\s+types:\s*\[project-status\]/);
  assert.match(workflow, /workflow_dispatch:/);
  assert.match(workflow, /DUUMBI_PROJECT_NUMBER/);
  assert.match(workflow, /ready-for-build/);
  assert.match(workflow, /undo-done/);
  assert.equal(workflow.includes("pulls.merge"), false);
  assert.equal(workflow.includes("validateAndMergeSpecPr"), false);
  assert.equal(/issues\.update|state:\s*["']open["']/.test(workflow), false);
  assert.match(workflow, /contents:\s+read/);
  assert.match(workflow, /issues:\s+write/);
  assert.doesNotMatch(workflow, /pull-requests:\s+write/);

  assert.ok(stageApproval.includes("files[0].filename.match(/^specs\\/DUUMBI-(\\d+)\\/PRODUCT\\.md$/)"));
  assert.ok(stageApproval.includes("files[0].filename.match(/^specs\\/DUUMBI-(\\d+)\\/TECHNICAL\\.md$/)"));
  assert.match(stageApproval, /must change only \$\{policy\.expectedPath\}/);
  assert.match(stageApproval, /pulls\.merge/);
});

test("ready-for-build handoff gates Project Status buttons and listens for correction events", () => {
  const workflow = readRepoFile(".github/workflows/ready-for-build-handoff.yml");
  assert.match(workflow, /types:\s*\[labeled, reopened\]/);
  assert.match(workflow, /pull_request:\s*\n\s+types:\s*\[closed\]/);
  assert.match(workflow, /DUUMBI_PROJECT_STATUS_SLACK_BUTTONS/);
  assert.match(workflow, /project-status-handoff\.mjs/);
  assert.equal(workflow.includes("pulls.merge"), false);
  assert.equal(workflow.includes("validateAndMergeSpecPr"), false);
});

test("product spec Slack review requests wait for review-clean PRs", () => {
  const workflow = readRepoFile(".github/workflows/spec-review-request.yml");

  assert.match(workflow, /PR is still draft/);
  assert.match(workflow, /required automated review evidence missing from/);
  assert.match(workflow, /requiredAutomatedReviewers/);
  assert.match(workflow, /Greptile is manual-only/);
  assert.match(workflow, /out of DUUMBI_REQUIRED_SPEC_REVIEWERS/);
  assert.match(workflow, /normalizeReviewerLogin/);
  assert.match(workflow, reviewerBotSuffixReplacement);
  assert.match(workflow, /unresolved review threads remain/);
  assert.match(workflow, /will be squash-merged on approval/);
  assert.match(workflow, /No product spec review notifications are ready to send/);
  assert.match(workflow, /\.then\(\(runs\) => runs\.flat\(\)\.filter\(Boolean\)\)/);
  assert.match(workflow, /GH_PROJECT_PAT: \$\{\{ secrets\.GH_PROJECT_PAT \}\}/);
  assert.match(workflow, /updateProjectStatus\(issue\.number, "Spec Review"\)/);
  assert.match(workflow, /Project status updates/);
});

test("technical spec Slack review requests wait for review-clean PRs", () => {
  const workflow = readRepoFile(".github/workflows/technical-spec-review-request.yml");

  assert.match(workflow, /technical spec review requires a linked TECHNICAL\.md PR/);
  assert.match(workflow, /PR is still draft/);
  assert.match(workflow, /required automated review evidence missing from/);
  assert.match(workflow, /requiredAutomatedReviewers/);
  assert.match(workflow, /Greptile is manual-only/);
  assert.match(workflow, /out of DUUMBI_REQUIRED_SPEC_REVIEWERS/);
  assert.match(workflow, /normalizeReviewerLogin/);
  assert.match(workflow, reviewerBotSuffixReplacement);
  assert.match(workflow, /unresolved review threads remain/);
  assert.match(workflow, /will be squash-merged on approval/);
  assert.match(workflow, /No technical spec review notifications are ready to send/);
  assert.match(workflow, /\.then\(\(runs\) => runs\.flat\(\)\.filter\(Boolean\)\)/);
});
