let app;
try {
  ({ app } = require("@azure/functions"));
} catch (error) {
  if (error.code !== "MODULE_NOT_FOUND") {
    throw error;
  }
  app = { http: () => {} };
}
const crypto = require("node:crypto");

/**
 * Slack Approval Bridge — Azure Function
 *
 * Bridges Slack interactive button clicks → GitHub repository_dispatch so that
 * approval decisions execute deterministically via GitHub Actions instead of
 * launching an agent directly.
 *
 * App Settings (configure in Azure Function App):
 *   SLACK_SIGNING_SECRET  — Slack app signing secret
 *   GITHUB_TOKEN          — GitHub PAT with repo scope
 *   GITHUB_REPO           — "owner/repo" (e.g. "hgahub/duumbi")
 */

app.http("slack-approval", {
  methods: ["POST"],
  authLevel: "anonymous",
  handler: (request, context) => handleSlackApproval(request, context),
});

// ── Helpers ────────────────────────────────────────────────────────────

function nonemptyString(value) {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function slackChannelId(payload) {
  return nonemptyString(payload?.channel?.id) || nonemptyString(payload?.channel_id);
}

function parentThreadTs(payload) {
  return nonemptyString(payload?.message?.thread_ts) || nonemptyString(payload?.message?.ts);
}

function slackThreadMetadata(slackPayload) {
  const metadata = {};
  const channelId = slackChannelId(slackPayload);
  const threadTs = parentThreadTs(slackPayload);
  if (channelId) metadata.channel_id = channelId;
  if (threadTs) metadata.thread_ts = threadTs;
  return metadata;
}

async function handleSlackApproval(request, context, deps = {}) {
  const fetchImpl = deps.fetch || globalThis.fetch;
  const env = deps.env || process.env;

  const body = await request.text();
  const timestamp = request.headers.get("X-Slack-Request-Timestamp");
  const signature = request.headers.get("X-Slack-Signature");

  if (!verifySlackSignature(body, timestamp, signature, env.SLACK_SIGNING_SECRET, deps.nowSeconds)) {
    return { status: 401, body: "Invalid signature" };
  }

  const params = new URLSearchParams(body);
  let payload;
  try {
    payload = JSON.parse(params.get("payload") || "{}");
  } catch {
    return { status: 400, jsonBody: { text: "Invalid Slack payload." } };
  }
  if (!payload || typeof payload !== "object") {
    return { status: 400, jsonBody: { text: "Invalid Slack payload." } };
  }

  if (payload.type !== "block_actions") {
    return { jsonBody: { text: "Unsupported interaction type." } };
  }

  const action = payload.actions?.[0];
  if (!action) {
    return { jsonBody: { text: "No action found." } };
  }

  let actionData;
  try {
    actionData = JSON.parse(action.value);
  } catch {
    return { jsonBody: { text: "Invalid action payload." } };
  }

  const user = payload.user;
  const reviewer = `Slack (${user.name || user.real_name || user.id})`;
  const decisionLabel = String(actionData.decision || "unknown").replace(/-/g, " ");
  const fallbackRationale = `${decisionLabel.charAt(0).toUpperCase() + decisionLabel.slice(1)} by ${reviewer}`;

  // Acknowledge Slack immediately (must respond within 3 seconds)
  // Then dispatch to GitHub asynchronously.
  const responseUrl = payload.response_url;
  const githubRepo = env.GITHUB_REPO || "hgahub/duumbi";
  const eventType = eventTypeForAction(actionData);
  const clientPayload = buildClientPayload(actionData, reviewer, fallbackRationale, payload);

  const work = dispatchAsync(
    githubRepo,
    eventType,
    clientPayload,
    responseUrl,
    actionData,
    user,
    context,
    { fetch: fetchImpl, env },
  );
  if (deps.awaitDispatch) await work;

  return { status: 200, body: "" };
}

function actionTypeForAction(actionData) {
  return actionData?.action_type || "stage_approval";
}

function eventTypeForAction(actionData) {
  const actionType = actionTypeForAction(actionData);
  if (actionType === "stage_10_authorization") return "stage-10-authorization";
  if (actionType === "project_status") return "project-status";
  return eventTypeForStage(actionData?.stage);
}

function normalizeStage10Decision(decision) {
  if (decision === "needs-clarification") return "narrow-scope";
  if (decision === "block") return "reject-defer";
  return decision;
}

function buildClientPayload(actionData, reviewer, fallbackRationale, slackPayload) {
  if (actionTypeForAction(actionData) === "project_status") {
    return {
      action_type: "project_status",
      issue_number: actionData.issue_number,
      decision: actionData.decision,
      rationale: actionData.rationale || fallbackRationale,
      reviewer,
      ...slackThreadMetadata(slackPayload),
    };
  }

  const payload = {
    stage: actionData.stage,
    issue_number: actionData.issue_number,
    decision: actionData.decision,
    rationale: actionData.rationale || fallbackRationale,
    pr_number: actionData.pr_number || 0,
    reviewer,
  };

  if (actionTypeForAction(actionData) === "stage_10_authorization" || String(actionData.stage || "") === "10") {
    payload.action_type = "stage_10_authorization";
    payload.decision = normalizeStage10Decision(actionData.decision);
    payload.cycle_number = actionData.cycle_number || actionData.cycle;
    payload.request_comment_id = actionData.request_comment_id;
  }

  return payload;
}

function fallbackWorkflowName(eventType) {
  if (eventType === "stage-10-authorization") return "stage-10-authorization.yml";
  if (eventType === "project-status") return "project-status.yml";
  return "stage-approval.yml";
}

function buildDispatchSuccessText(eventType, actionData, user) {
  if (eventType === "project-status") {
    return `⏳ Project Status update triggered by <@${user.id}> — GitHub Actions workflow running…`;
  }
  if (eventType === "stage-10-authorization") {
    return `⏳ Stage 10 cycle ${actionData.cycle_number || actionData.cycle || "?"} *${actionData.decision}* triggered by <@${user.id}> — GitHub Actions workflow running…`;
  }
  return `⏳ Stage ${actionData.stage} *${actionData.decision}* triggered by <@${user.id}> — GitHub Actions workflow running…`;
}

function buildDispatchFailureText(eventType, status) {
  if (eventType === "project-status") {
    return `⚠️ Project Status update failed (HTTP ${status}). Project Status was not changed. Run project-status.yml manually, or set Status in the GitHub Project UI.`;
  }
  return `⚠️ Approval workflow trigger failed (HTTP ${status}). Please use ${fallbackWorkflowName(eventType)} as the manual workflow dispatch fallback.`;
}

async function dispatchAsync(githubRepo, eventType, clientPayload, responseUrl, actionData, user, context, deps = {}) {
  const fetchImpl = deps.fetch || globalThis.fetch;
  const env = deps.env || process.env;
  try {
    const res = await fetchImpl(
      `https://api.github.com/repos/${githubRepo}/dispatches`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${env.GITHUB_TOKEN}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
          "Content-Type": "application/json",
          "User-Agent": "duumbi-slack-approval-bridge/1.0",
        },
        body: JSON.stringify({ event_type: eventType, client_payload: clientPayload }),
      },
    );

    if (!res.ok) {
      const errText = await res.text();
      context.error("GitHub dispatch failed:", res.status, errText);
      if (responseUrl) {
        await fetchImpl(responseUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            replace_original: false,
            text: buildDispatchFailureText(eventType, res.status),
          }),
        });
      }
      return;
    }

    if (responseUrl) {
      await fetchImpl(responseUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          replace_original: false,
          text: buildDispatchSuccessText(eventType, actionData, user),
        }),
      });
    }
  } catch (err) {
    context.error("dispatchAsync error:", err);
  }
}

function eventTypeForStage(stage) {
  const normalized = String(stage || "");
  if (normalized === "10") return "stage-10-authorization";
  return "stage-approval";
}

function verifySlackSignature(body, timestamp, signature, signingSecret, nowSeconds) {
  if (!timestamp || !signature || !signingSecret) return false;

  // Reject non-numeric or stale timestamps (>5 minutes)
  const ts = Number(timestamp);
  if (!Number.isFinite(ts)) return false;
  const now = Number.isFinite(nowSeconds) ? nowSeconds : Math.floor(Date.now() / 1000);
  if (Math.abs(now - ts) > 300) return false;

  const baseString = `v0:${timestamp}:${body}`;
  const computed =
    "v0=" + crypto.createHmac("sha256", signingSecret).update(baseString).digest("hex");

  const a = Buffer.from(computed);
  const b = Buffer.from(signature);
  if (a.length !== b.length) return false;
  return crypto.timingSafeEqual(a, b);
}

module.exports = {
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
};
