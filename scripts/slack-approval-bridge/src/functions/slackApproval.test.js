const test = require("node:test");
const assert = require("node:assert/strict");

const {
  actionTypeForAction,
  buildClientPayload,
  buildDispatchSuccessText,
  eventTypeForAction,
  fallbackWorkflowName,
  verifySlackSignature,
} = require("./slackApproval.js");

test("existing stage approval actions still route to stage-approval", () => {
  const actionData = {
    stage: "9",
    issue_number: 595,
    decision: "approve",
    pr_number: 618,
  };

  assert.equal(actionTypeForAction(actionData), "stage_approval");
  assert.equal(eventTypeForAction(actionData), "stage-approval");
  assert.equal(fallbackWorkflowName(eventTypeForAction(actionData)), "stage-approval.yml");

  const payload = buildClientPayload(
    actionData,
    "Slack (hga)",
    "Approve by Slack (hga)",
  );
  assert.deepEqual(payload, {
    stage: "9",
    issue_number: 595,
    decision: "approve",
    rationale: "Approve by Slack (hga)",
    pr_number: 618,
    reviewer: "Slack (hga)",
  });
  assert.equal("slack_response_url" in payload, false);
});

test("stage 10 authorization actions route to dedicated repository dispatch", () => {
  const actionData = {
    action_type: "stage_10_authorization",
    issue_number: 595,
    cycle_number: 2,
    request_comment_id: 123456789,
    decision: "narrow-scope",
    pr_number: 0,
  };

  assert.equal(actionTypeForAction(actionData), "stage_10_authorization");
  assert.equal(eventTypeForAction(actionData), "stage-10-authorization");
  assert.equal(fallbackWorkflowName(eventTypeForAction(actionData)), "stage-10-authorization.yml");

  const payload = buildClientPayload(
    actionData,
    "Slack (hga)",
    "Narrow scope by Slack (hga)",
  );
  assert.equal(payload.action_type, "stage_10_authorization");
  assert.equal(payload.issue_number, 595);
  assert.equal(payload.cycle_number, 2);
  assert.equal(payload.request_comment_id, 123456789);
  assert.equal(payload.decision, "narrow-scope");
  assert.equal(payload.rationale, "Narrow scope by Slack (hga)");
  assert.equal("slack_response_url" in payload, false);
});

test("legacy stage 10 payloads route to the dedicated authorization workflow", () => {
  const actionData = {
    stage: "10",
    issue_number: 595,
    cycle: 4,
    decision: "needs-clarification",
  };

  assert.equal(eventTypeForAction(actionData), "stage-10-authorization");
  assert.equal(fallbackWorkflowName(eventTypeForAction(actionData)), "stage-10-authorization.yml");

  const payload = buildClientPayload(
    actionData,
    "Slack (hga)",
    "Needs clarification by Slack (hga)",
  );
  assert.equal(payload.action_type, "stage_10_authorization");
  assert.equal(payload.cycle_number, 4);
  assert.equal(payload.decision, "narrow-scope");
});

test("stage 11 payloads fail closed through stage approval fallback", () => {
  const actionData = {
    stage: "11",
    issue_number: 595,
    pr_number: 621,
    decision: "approve-merge",
  };

  assert.equal(eventTypeForAction(actionData), "stage-approval");
  assert.equal(fallbackWorkflowName(eventTypeForAction(actionData)), "stage-approval.yml");
});

test("stage 10 Slack follow-up names the cycle and does not imply implementation execution", () => {
  const text = buildDispatchSuccessText(
    "stage-10-authorization",
    { cycle_number: 3, decision: "approve" },
    { id: "U123" },
  );

  assert.match(text, /Stage 10 cycle 3/);
  assert.match(text, /GitHub Actions workflow running/);
  assert.doesNotMatch(text, /implementation running/i);
});

test("invalid or missing signing inputs fail Slack signature verification", () => {
  assert.equal(verifySlackSignature("body", "123", "v0=abc", ""), false);
  assert.equal(verifySlackSignature("body", "not-a-number", "v0=abc", "secret"), false);
});
