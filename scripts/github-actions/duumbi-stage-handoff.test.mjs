import test from "node:test";
import assert from "node:assert/strict";

import {
  buildStage10ApprovalSlackMessage,
  buildStage10DecisionComment,
  buildStage11Prompt,
  buildStage11ReviewSlackMessage,
  buildWorkflowMetrics,
  extractGithubUrls,
  findLatestResourceApprovalRequest,
  findStage6ProductSpecArtifact,
  findStage8TechnicalSpecArtifact,
  hasStage10NotificationMarker,
  parseResourceApprovalRequest,
  sanitizePromptField,
  sanitizeSlackField,
} from "./duumbi-stage-handoff.mjs";

const validRequestBody = `## Ralph Cycle 2 Resource Approval Request

**Issue:** https://github.com/hgahub/duumbi/issues/595
**Product spec:** https://github.com/hgahub/duumbi/pull/616
**Technical spec:** https://github.com/hgahub/duumbi/pull/618

## Current State
Stage 10 is ready for implementation.

## Remaining Requirements
Helper, workflows, bridge routing, skill updates, and PR evidence remain.

## Proposed Cycle Goal
Add the pure Stage 10 and Stage 11 handoff helper and focused tests.

## Planned Changes
- scripts/github-actions/duumbi-stage-handoff.mjs
- scripts/github-actions/duumbi-stage-handoff.test.mjs

## Planned Checks
- node --test scripts/github-actions/duumbi-stage-handoff.test.mjs

## Resource Estimate
- external LLM calls: 0
- estimated external LLM cost: USD 0

## Approval Trigger
No trigger; this cycle is below thresholds.

## Stop Condition
Stop after tests pass or after a blocker appears.
`;

test("parseResourceApprovalRequest extracts a valid structured request", () => {
  const parsed = parseResourceApprovalRequest({
    id: 123456789,
    url: "https://github.com/hgahub/duumbi/issues/595#issuecomment-123456789",
    body: validRequestBody,
  });

  assert.equal(parsed.ok, true);
  assert.equal(parsed.found, true);
  assert.equal(parsed.cycleNumber, 2);
  assert.equal(parsed.requestCommentId, 123456789);
  assert.deepEqual(parsed.missingFields, []);
  assert.match(parsed.fields["Resource Estimate"], /USD 0/);
  assert.match(parsed.fields["Proposed Cycle Goal"], /handoff helper/);
});

test("parseResourceApprovalRequest reports required-field errors without guessing", () => {
  const malformed = validRequestBody.replace(/## Resource Estimate[\s\S]*?## Approval Trigger/, "## Approval Trigger");
  const parsed = parseResourceApprovalRequest({ id: 1, body: malformed });

  assert.equal(parsed.ok, false);
  assert.equal(parsed.found, true);
  assert.deepEqual(parsed.missingFields, ["Resource Estimate"]);
  assert.deepEqual(parsed.errors, ["Missing required field: Resource Estimate"]);
});

test("findLatestResourceApprovalRequest suppresses duplicates by marker but allows later cycles", () => {
  const cycle1 = { id: 11, body: validRequestBody.replace("Cycle 2", "Cycle 1") };
  const marker = {
    id: 12,
    body: "<!-- duumbi-ralph-cycle-approval-slack-notified:v1 issue=595 cycle=1 request_comment_id=11 -->",
  };
  const cycle2 = { id: 13, body: validRequestBody };

  assert.equal(hasStage10NotificationMarker([cycle1, marker], 11, 1), true);
  const latest = findLatestResourceApprovalRequest([cycle1, marker, cycle2]);
  assert.equal(latest.requestCommentId, 13);
  assert.equal(latest.cycleNumber, 2);
});

test("buildStage10ApprovalSlackMessage includes required fields and Stage 10 action values", () => {
  const parsed = parseResourceApprovalRequest({ id: 123456789, body: validRequestBody });
  const message = buildStage10ApprovalSlackMessage({
    issueNumber: 595,
    cycleNumber: parsed.cycleNumber,
    requestCommentId: parsed.requestCommentId,
    requestCommentUrl: "https://github.com/hgahub/duumbi/issues/595#issuecomment-123456789",
    fields: parsed.fields,
  });

  const rendered = JSON.stringify(message);
  assert.match(rendered, /DUUMBI Stage 10 resource authorization needed/);
  assert.match(rendered, /Resource estimate/);
  assert.match(rendered, /Approval trigger/);
  assert.match(rendered, /stage_10_authorization/);
  assert.match(rendered, /stage-10-authorization.yml/);

  const actionValues = message.blocks.find((block) => block.type === "actions").elements.map((element) => JSON.parse(element.value));
  assert.deepEqual(
    actionValues.map((value) => value.decision),
    ["approve", "narrow-scope", "reject-defer"],
  );
  assert.ok(actionValues.every((value) => value.issue_number === 595));
  assert.ok(actionValues.every((value) => value.cycle_number === 2));
});

test("buildStage10DecisionComment records one-cycle authorization semantics", () => {
  const comment = buildStage10DecisionComment({
    decision: "approve",
    reviewer: "Slack (hga)",
    issueUrl: "https://github.com/hgahub/duumbi/issues/595",
    cycleNumber: 2,
    requestCommentUrl: "https://github.com/hgahub/duumbi/issues/595#issuecomment-123456789",
    proposedCycleGoal: "Add the handoff helper.",
  });

  assert.match(comment, /## Stage 10 Resource Authorization Decision/);
  assert.match(comment, /\*\*Decision:\*\* Approve Cycle/);
  assert.match(comment, /\*\*Authorized scope:\*\* Add the handoff helper\./);
  assert.match(comment, /\*\*Next state:\*\* In Progress/);

  const narrow = buildStage10DecisionComment({
    decision: "narrow-scope",
    reviewer: "Slack (hga)",
    issueUrl: "https://github.com/hgahub/duumbi/issues/595",
    cycleNumber: 2,
  });
  assert.match(narrow, /revised narrower request is required/);
  assert.match(narrow, /Cycle Authorization/);

  const rejected = buildStage10DecisionComment({
    decision: "reject-defer",
    reviewer: "Slack (hga)",
    issueUrl: "https://github.com/hgahub/duumbi/issues/595",
    cycleNumber: 2,
  });
  assert.match(rejected, /No cycle is authorized/);
  assert.match(rejected, /Deferred/);
});

test("Stage 11 prompt and Slack message include issue, PR, specs, checks, and evidence", () => {
  const input = {
    issueNumber: 595,
    prNumber: 619,
    issueUrl: "https://github.com/hgahub/duumbi/issues/595",
    prUrl: "https://github.com/hgahub/duumbi/pull/619",
    productSpec: "https://github.com/hgahub/duumbi/pull/616",
    technicalSpec: "https://github.com/hgahub/duumbi/pull/618",
    checkSummary: "node tests passed",
    evidenceSummary: "Ralph Cycle 1 Evidence Report",
  };

  const prompt = buildStage11Prompt(input);
  assert.match(prompt, /duumbi-review-artifact/);
  assert.match(prompt, /Implementation PR: https:\/\/github\.com\/hgahub\/duumbi\/pull\/619/);
  assert.match(prompt, /Greptile unless the PR is a stable high-risk implementation change/);
  assert.match(prompt, /Do not merge PRs/);

  const message = buildStage11ReviewSlackMessage({ ...input, prompt });
  const rendered = JSON.stringify(message);
  assert.match(rendered, /Stage 11 review handoff ready/);
  assert.match(rendered, /node tests passed/);
  assert.match(rendered, /Ralph Cycle 1 Evidence Report/);
  assert.doesNotMatch(rendered, /"actions"/);
});

test("findStage6ProductSpecArtifact and findStage8TechnicalSpecArtifact prefer latest comments", () => {
  const comments = [
    { body: "## Stage 6 Product Spec Draft\n\nProduct spec artifact: https://github.com/hgahub/duumbi/pull/100" },
    { body: "## Stage 8 Technical Spec Draft\n\nTechnical spec artifact: https://github.com/hgahub/duumbi/pull/101" },
    { body: "## Stage 6 Product Spec Draft\n\nProduct spec artifact: https://github.com/hgahub/duumbi/pull/616" },
    { body: "## Stage 8 Technical Spec Draft\n\nTechnical spec artifact: https://github.com/hgahub/duumbi/pull/618" },
  ];

  assert.equal(findStage6ProductSpecArtifact(comments), "https://github.com/hgahub/duumbi/pull/616");
  assert.equal(findStage8TechnicalSpecArtifact(comments), "https://github.com/hgahub/duumbi/pull/618");
});

test("buildWorkflowMetrics keeps metadata-only workflow metrics shape", () => {
  const metrics = buildWorkflowMetrics({
    generatedAt: "2026-05-23T00:00:00.000Z",
    workflowName: "Stage 10 Authorization",
    workflowFile: ".github/workflows/stage-10-authorization.yml",
    runId: 123,
    issueNumber: 595,
    stage: "10",
    decision: "approve",
    slackNotificationsAttempted: 1,
  });

  assert.equal(metrics.schema_version, "duumbi.workflow_metrics.v1");
  assert.equal(metrics.provider_usage.available, false);
  assert.equal(metrics.provider_usage.reason, "no_provider_step");
  assert.equal(metrics.privacy.raw_slack_payloads, false);
  assert.equal(metrics.privacy.issue_or_comment_bodies, false);
  assert.equal(JSON.stringify(metrics).includes(["SLACK", "BOT_TOKEN"].join("_")), false);
});

test("sanitizers and URL extraction avoid unsafe Slack/control output", () => {
  assert.equal(sanitizePromptField(" hello\u0000\nworld "), "hello\nworld");
  assert.equal(sanitizeSlackField(" <hello> & goodbye "), "&lt;hello&gt; &amp; goodbye");
  assert.equal(sanitizeSlackField("&".repeat(1000)).length, 2000);
  assert.equal(sanitizeSlackField("&".repeat(1000), 120).length, 120);
  assert.deepEqual(extractGithubUrls("See https://github.com/hgahub/duumbi/pull/616."), [
    "https://github.com/hgahub/duumbi/pull/616",
  ]);
});

test("Slack messages stay within Block Kit text limits after escaping", () => {
  const parsed = parseResourceApprovalRequest({
    id: 123456789,
    body: validRequestBody
      .replace("Add the pure Stage 10 and Stage 11 handoff helper and focused tests.", "&".repeat(1000))
      .replace("- node --test scripts/github-actions/duumbi-stage-handoff.test.mjs", "<".repeat(1000)),
  });
  const stage10Message = buildStage10ApprovalSlackMessage({
    issueNumber: 595,
    cycleNumber: parsed.cycleNumber,
    requestCommentId: parsed.requestCommentId,
    fields: {
      ...parsed.fields,
      "Product spec": "&".repeat(1000),
      "Technical spec": "<".repeat(1000),
      "Resource Estimate": ">".repeat(1000),
      "Approval Trigger": "&<>".repeat(1000),
      "Stop Condition": "&".repeat(1000),
    },
  });

  const stage11Message = buildStage11ReviewSlackMessage({
    issueNumber: 595,
    prNumber: 619,
    issueUrl: "https://github.com/hgahub/duumbi/issues/595",
    prUrl: "https://github.com/hgahub/duumbi/pull/619",
    productSpec: "&".repeat(1000),
    technicalSpec: "<".repeat(1000),
    checkSummary: ">".repeat(1000),
    evidenceSummary: "&<>".repeat(1000),
    prompt: "&<>".repeat(2000),
  });

  for (const message of [stage10Message, stage11Message]) {
    for (const block of message.blocks) {
      if (block.type === "section" && block.text) {
        assert.ok(block.text.text.length <= 3000, block.text.text);
      }
      if (block.type === "section" && block.fields) {
        for (const field of block.fields) {
          assert.ok(field.text.length <= 2000, field.text);
        }
      }
    }
  }
});
