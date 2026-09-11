const test = require("node:test");
const assert = require("node:assert/strict");
const crypto = require("node:crypto");

const {
  actionTypeForAction,
  buildClientPayload,
  buildDispatchFailureText,
  buildDispatchSuccessText,
  eventTypeForAction,
  fallbackWorkflowName,
  handleSlackApproval,
  parentThreadTs,
  slackChannelId,
  verifySlackSignature,
} = require("./slackApproval.js");

const SIGNING_SECRET = "test-signing-secret";

function signBody(body, timestamp, secret = SIGNING_SECRET) {
  return "v0=" + crypto.createHmac("sha256", secret).update(`v0:${timestamp}:${body}`).digest("hex");
}

function mockRequest({ body, timestamp, signature }) {
  const headers = {
    "X-Slack-Request-Timestamp": timestamp,
    "X-Slack-Signature": signature,
  };
  return {
    text: async () => body,
    headers: {
      get: (name) => headers[name] ?? null,
    },
  };
}

function mockContext() {
  return { error() {} };
}

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

test("stage 9 approve payload still maps to stage-approval after project_status routing", () => {
  const actionData = {
    stage: "9",
    issue_number: 789,
    decision: "approve",
    pr_number: 791,
  };
  assert.equal(eventTypeForAction(actionData), "stage-approval");
  assert.notEqual(eventTypeForAction(actionData), "project-status");
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

test("project_status routes to project-status with thread metadata and without response_url", () => {
  const actionData = {
    action_type: "project_status",
    issue_number: 789,
    decision: "ready-for-build",
  };

  assert.equal(actionTypeForAction(actionData), "project_status");
  assert.equal(eventTypeForAction(actionData), "project-status");
  assert.equal(fallbackWorkflowName("project-status"), "project-status.yml");

  const slackPayload = {
    channel: { id: "C123" },
    message: { ts: "1234567890.000001", thread_ts: "1234567890.123456" },
    response_url: "https://hooks.slack.com/actions/T/B/secret",
  };

  const payload = buildClientPayload(
    actionData,
    "Slack (hga)",
    "Ready for build by Slack (hga)",
    slackPayload,
  );

  assert.deepEqual(payload, {
    action_type: "project_status",
    issue_number: 789,
    decision: "ready-for-build",
    rationale: "Ready for build by Slack (hga)",
    reviewer: "Slack (hga)",
    channel_id: "C123",
    thread_ts: "1234567890.123456",
  });
  assert.equal("slack_response_url" in payload, false);
  assert.equal("response_url" in payload, false);
  assert.equal("stage" in payload, false);
  assert.equal("pr_number" in payload, false);
});

test("project_status parent thread_ts falls back to the card ts when not already in a thread", () => {
  const slackPayload = {
    channel: { id: "C123" },
    message: { ts: "1234567890.000200" },
  };
  assert.equal(slackChannelId(slackPayload), "C123");
  assert.equal(parentThreadTs(slackPayload), "1234567890.000200");

  const payload = buildClientPayload(
    { action_type: "project_status", issue_number: 779, decision: "undo-done" },
    "Slack (hga)",
    "Undo done by Slack (hga)",
    slackPayload,
  );
  assert.equal(payload.thread_ts, "1234567890.000200");
  assert.equal(payload.decision, "undo-done");
});

test("project_status omits empty channel_id and thread_ts instead of sending empty strings", () => {
  const payload = buildClientPayload(
    { action_type: "project_status", issue_number: 789, decision: "ready-for-build" },
    "Slack (hga)",
    "Ready for build by Slack (hga)",
    { channel: { id: "" }, message: { ts: "   " } },
  );
  assert.equal("channel_id" in payload, false);
  assert.equal("thread_ts" in payload, false);
});

test("project_status success text names a Project Status update, not Stage 7/9 or implementation", () => {
  const text = buildDispatchSuccessText(
    "project-status",
    { decision: "ready-for-build" },
    { id: "U123" },
  );
  assert.match(text, /Project Status update/);
  assert.doesNotMatch(text, /Stage 7|Stage 9|implementation/i);
});

test("project_status dispatch failure Slack names project-status.yml and the Project UI, not Stage 7/9 Approve", () => {
  const text = buildDispatchFailureText("project-status", 500);
  assert.match(text, /project-status\.yml/);
  assert.match(text, /Project UI/);
  assert.match(text, /Project Status was not changed/);
  assert.doesNotMatch(text, /Approve/i);
  assert.doesNotMatch(text, /stage-approval/i);
});

test("invalid or missing signing inputs fail Slack signature verification", () => {
  assert.equal(verifySlackSignature("body", "123", "v0=abc", ""), false);
  assert.equal(verifySlackSignature("body", "not-a-number", "v0=abc", "secret"), false);
  assert.equal(verifySlackSignature("body", undefined, "v0=abc", "secret"), false);
  assert.equal(verifySlackSignature("body", "123", undefined, "secret"), false);
  assert.equal(verifySlackSignature("body", "123", "v0=abc", undefined), false);
});

test("stale or wrong HMAC signatures fail verification", () => {
  const now = 1_700_000_000;
  const body = "payload=%7B%7D";
  const freshTs = String(now);
  const staleTs = String(now - 301);
  const validSig = signBody(body, freshTs);

  assert.equal(verifySlackSignature(body, staleTs, signBody(body, staleTs), SIGNING_SECRET, now), false);
  assert.equal(verifySlackSignature(body, freshTs, "v0=" + "00".repeat(32), SIGNING_SECRET, now), false);
  assert.equal(verifySlackSignature(body, freshTs, validSig, SIGNING_SECRET, now), true);
});

test("invalid Slack signature returns 401 with zero outbound Slack or GitHub calls", async () => {
  const fetchCalls = [];
  const fetchImpl = async (...args) => {
    fetchCalls.push(args);
    return { ok: true, status: 204, text: async () => "" };
  };

  const now = 1_700_000_000;
  const slackPayload = {
    type: "block_actions",
    response_url: "https://hooks.slack.com/actions/T/B/untrusted",
    channel: { id: "C999" },
    message: { ts: "1.1" },
    user: { id: "U1", name: "attacker" },
    actions: [{
      value: JSON.stringify({
        action_type: "project_status",
        issue_number: 789,
        decision: "ready-for-build",
      }),
    }],
  };
  const body = `payload=${encodeURIComponent(JSON.stringify(slackPayload))}`;
  const cases = [
    { timestamp: undefined, signature: signBody(body, String(now)) },
    { timestamp: "not-a-number", signature: "v0=abc" },
    { timestamp: String(now - 301), signature: signBody(body, String(now - 301)) },
    { timestamp: String(now), signature: "v0=" + "ab".repeat(32) },
  ];

  for (const [index, headers] of cases.entries()) {
    fetchCalls.length = 0;
    const result = await handleSlackApproval(
      mockRequest({ body, timestamp: headers.timestamp, signature: headers.signature }),
      mockContext(),
      {
        fetch: fetchImpl,
        env: { SLACK_SIGNING_SECRET: SIGNING_SECRET, GITHUB_TOKEN: "gh", GITHUB_REPO: "hgahub/duumbi" },
        nowSeconds: now,
        awaitDispatch: true,
      },
    );
    assert.equal(result.status, 401, `case ${index} should be 401`);
    assert.equal(result.body, "Invalid signature");
    assert.equal(fetchCalls.length, 0, `case ${index} must not call fetch`);
  }

  fetchCalls.length = 0;
  const missingSecret = await handleSlackApproval(
    mockRequest({ body, timestamp: String(now), signature: signBody(body, String(now)) }),
    mockContext(),
    {
      fetch: fetchImpl,
      env: { GITHUB_TOKEN: "gh" },
      nowSeconds: now,
      awaitDispatch: true,
    },
  );
  assert.equal(missingSecret.status, 401);
  assert.equal(fetchCalls.length, 0);
});

test("valid project_status click dispatches project-status without forwarding response_url", async () => {
  const fetchCalls = [];
  const fetchImpl = async (url, options) => {
    fetchCalls.push({ url, options });
    return { ok: true, status: 204, text: async () => "" };
  };

  const now = 1_700_000_000;
  const slackPayload = {
    type: "block_actions",
    response_url: "https://hooks.slack.com/actions/T/B/ok",
    channel: { id: "C123" },
    message: { ts: "1234567890.000001", thread_ts: "1234567890.123456" },
    user: { id: "U123", name: "hga" },
    actions: [{
      value: JSON.stringify({
        action_type: "project_status",
        issue_number: 789,
        decision: "ready-for-build",
      }),
    }],
  };
  const body = `payload=${encodeURIComponent(JSON.stringify(slackPayload))}`;
  const timestamp = String(now);
  const result = await handleSlackApproval(
    mockRequest({ body, timestamp, signature: signBody(body, timestamp) }),
    mockContext(),
    {
      fetch: fetchImpl,
      env: { SLACK_SIGNING_SECRET: SIGNING_SECRET, GITHUB_TOKEN: "gh", GITHUB_REPO: "hgahub/duumbi" },
      nowSeconds: now,
      awaitDispatch: true,
    },
  );

  assert.equal(result.status, 200);
  const githubCall = fetchCalls.find((call) => String(call.url).includes("api.github.com/repos/hgahub/duumbi/dispatches"));
  assert.ok(githubCall, "expected GitHub repository_dispatch");
  const dispatched = JSON.parse(githubCall.options.body);
  assert.equal(dispatched.event_type, "project-status");
  assert.equal(dispatched.client_payload.action_type, "project_status");
  assert.equal(dispatched.client_payload.channel_id, "C123");
  assert.equal(dispatched.client_payload.thread_ts, "1234567890.123456");
  assert.equal("response_url" in dispatched.client_payload, false);
  assert.equal("slack_response_url" in dispatched.client_payload, false);

  const slackCall = fetchCalls.find((call) => call.url === slackPayload.response_url);
  assert.ok(slackCall, "valid-signature follow-up may use response_url inside the Function only");
  assert.match(JSON.parse(slackCall.options.body).text, /Project Status update/);
});

for (const type of ["message_action", "shortcut"]) {
  test(`retired ${type} intake cannot dispatch or send Slack callbacks`, async () => {
    const now = 1_700_000_000;
    const timestamp = String(now);
    const body = new URLSearchParams({ payload: JSON.stringify({
      type,
      callback_id: "duumbi-idea",
      response_url: "https://hooks.slack.com/actions/T/B/unused",
      channel: { id: "C123" },
      message: { ts: "123.456", text: "An idea" },
      user: { id: "U123" },
    }) }).toString();
    const calls = [];
    const result = await handleSlackApproval(
      mockRequest({ body, timestamp, signature: signBody(body, timestamp) }),
      mockContext(),
      {
        env: { SLACK_SIGNING_SECRET: SIGNING_SECRET, GITHUB_TOKEN: "test" },
        nowSeconds: now,
        awaitDispatch: true,
        fetch: async (...args) => { calls.push(args); return { ok: true }; },
      },
    );
    assert.deepEqual(result, { jsonBody: { text: "Unsupported interaction type." } });
    assert.deepEqual(calls, []);
  });
}
