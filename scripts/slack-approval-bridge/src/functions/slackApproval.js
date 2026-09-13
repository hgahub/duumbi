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
const { decisionDetails } = require("../../lib/stage5Decision.js");

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
  if (params.has("command")) return handleIntakeCommand(params, context, { fetch: fetchImpl, env });
  let payload;
  try {
    payload = JSON.parse(params.get("payload") || "{}");
  } catch {
    return { status: 400, jsonBody: { text: "Invalid Slack payload." } };
  }
  if (!payload || typeof payload !== "object") {
    return { status: 400, jsonBody: { text: "Invalid Slack payload." } };
  }

  if (payload.type === "view_submission") {
    return submitStage5Modal(payload, context, { fetch: fetchImpl, env });
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

  if (String(actionData.stage) === "5" && actionTypeForAction(actionData) === "stage_approval" && ["reject", "needs-clarification"].includes(actionData.decision)) {
    return openStage5Modal(payload, actionData, context, { fetch: fetchImpl, env });
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

async function handleIntakeCommand(params, context, { fetch: fetchImpl, env }) {
  const reply = (text) => ({ status: 200, jsonBody: { response_type: "ephemeral", text } });
  if (params.get("command") !== "/duumbi-triage") return reply("Unknown command. Use /duumbi-triage <intake_id>.");
  const intakeId = (params.get("text") || "").trim();
  if (!/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(intakeId)) return reply("Usage: /duumbi-triage <intake_id> — provide exactly one intake ID.");
  const channel = params.get("channel_id"), user = params.get("user_id");
  if (!/^[CG][A-Z0-9]+$/.test(channel || "") || !/^[UW][A-Z0-9]+$/.test(user || "")) return reply("Use the command in a Slack channel or private group.");
  if (!env.GITHUB_TOKEN) return reply("GitHub dispatch is not configured. Run intake-stage5.yml manually.");
  const repo = env.GITHUB_REPO || "hgahub/duumbi";
  const workflow = `https://github.com/${repo}/actions/workflows/intake-stage5.yml`;
  try {
    // A bounded synchronous request avoids fire-and-forget work after Azure ends the invocation.
    const response = await fetchImpl(`https://api.github.com/repos/${repo}/dispatches`, {
      method: "POST", signal: AbortSignal.timeout(2000),
      headers: { Authorization: `Bearer ${env.GITHUB_TOKEN}`, Accept: "application/vnd.github+json", "Content-Type": "application/json", "X-GitHub-Api-Version": "2022-11-28" },
      body: JSON.stringify({ event_type: "intake-stage5", client_payload: { intake_id: intakeId, channel_id: channel, user_id: user } }),
    });
    if (!response.ok) return reply(`Dispatch failed (HTTP ${response.status}). ${workflow}`);
    return reply(`Intake ${intakeId} submitted for Stage 5 routing. This is not human acceptance. The Action will check readiness and report the result here. ${workflow}`);
  } catch (error) {
    context.error("Intake dispatch outcome unknown:", error.name);
    return reply(`Dispatch outcome is unknown; check the Action before retrying. Reusing the same intake_id will not intentionally create another issue. ${workflow}`);
  }
}

const STAGE5_MODAL = "duumbi_stage5_decision";

function buildStage5Modal(actionData) {
  const clarification = actionData.decision === "needs-clarification";
  const input = (id, label, multiline = false) => ({
    type: "input", block_id: id, label: { type: "plain_text", text: label },
    element: { type: "plain_text_input", action_id: "value", multiline, max_length: id === "clarification_owner" ? 40 : 2000 },
  });
  return {
    type: "modal", callback_id: STAGE5_MODAL,
    private_metadata: JSON.stringify({ stage: "5", issue_number: Number(actionData.issue_number), decision: actionData.decision }),
    title: { type: "plain_text", text: clarification ? "Needs Clarification" : "Reject Issue" },
    submit: { type: "plain_text", text: clarification ? "Request clarification" : "Reject issue" },
    close: { type: "plain_text", text: "Cancel" },
    blocks: [
      { type: "section", text: { type: "plain_text", text: `Issue #${actionData.issue_number}. ${clarification ? "Record the blocking question and who should answer it." : "Submitting this form closes the issue."}` } },
      input("rationale", "Rationale", true),
      ...(clarification ? [input("clarification_question", "Question to resolve", true), input("clarification_owner", "Owner (GitHub username)")] : []),
    ],
  };
}

async function openStage5Modal(payload, actionData, context, { fetch: fetchImpl, env }) {
  if (!env.SLACK_BOT_TOKEN || !payload.trigger_id || !Number.isSafeInteger(Number(actionData.issue_number)) || Number(actionData.issue_number) < 1) {
    return { status: 200, jsonBody: { response_type: "ephemeral", text: "Cannot open decision form. Check bridge SLACK_BOT_TOKEN configuration; no decision was submitted." } };
  }
  try {
    const response = await fetchImpl("https://slack.com/api/views.open", {
      method: "POST", headers: { Authorization: `Bearer ${env.SLACK_BOT_TOKEN}`, "Content-Type": "application/json" },
      body: JSON.stringify({ trigger_id: payload.trigger_id, view: buildStage5Modal(actionData) }),
      signal: AbortSignal.timeout(2000),
    });
    const result = await response.json();
    if (!response.ok || !result.ok) throw new Error("Modal open failed");
    return { status: 200, body: "" };
  } catch {
    context.error("Stage 5 modal could not be opened.");
    return { status: 200, jsonBody: { response_type: "ephemeral", text: "Decision form could not open. No decision was submitted. Please try again." } };
  }
}

async function submitStage5Modal(payload, context, { fetch: fetchImpl, env }) {
  if (payload.view?.callback_id !== STAGE5_MODAL) return { status: 400, body: "Unknown modal" };
  let action;
  try { action = JSON.parse(payload.view.private_metadata); } catch { return { status: 400, body: "Invalid metadata" }; }
  if (action?.stage !== "5" || !Number.isSafeInteger(action.issue_number) || action.issue_number < 1 || !["reject", "needs-clarification"].includes(action.decision)) return { status: 400, body: "Invalid decision" };
  const values = payload.view.state?.values || {};
  const { details, errors } = decisionDetails({ ...action,
    rationale: values.rationale?.value?.value,
    clarification_question: values.clarification_question?.value?.value,
    clarification_owner: values.clarification_owner?.value?.value,
  });
  const errorResponse = (errors) => ({ status: 200, jsonBody: { response_action: "errors", errors } });
  if (Object.keys(errors).length) return errorResponse(errors);
  if (!env.GITHUB_TOKEN) return errorResponse({ rationale: "Bridge GitHub token is not configured. No decision was submitted." });
  // Await GitHub acceptance before closing the modal; never fire-and-forget a decision.
  try {
    const response = await fetchImpl(`https://api.github.com/repos/${env.GITHUB_REPO || "hgahub/duumbi"}/dispatches`, {
      method: "POST",
      headers: { Authorization: `Bearer ${env.GITHUB_TOKEN}`, Accept: "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28", "Content-Type": "application/json" },
      body: JSON.stringify({ event_type: "stage-approval", client_payload: {
        ...action, ...details, reviewer: `Slack (${payload.user?.id || "unknown"})`,
        decision_id: payload.view.id,
      } }),
      signal: AbortSignal.timeout(2000),
    });
    if (response.status !== 204) return errorResponse({ rationale: "GitHub did not accept the decision. Your inputs are preserved; try again or use the manual workflow." });
    return { status: 200, jsonBody: { response_action: "update", view: {
      type: "modal", title: { type: "plain_text", text: "Decision submitted" }, close: { type: "plain_text", text: "Close" },
      blocks: [{ type: "section", text: { type: "plain_text", text: "GitHub Actions accepted the request. Wait for its decision comment and Slack result before treating the change as complete." } }],
    } } };
  } catch {
    context.error("Stage 5 dispatch outcome is unknown; retain the modal for recovery.");
    return errorResponse({ rationale: "Dispatch outcome is unknown. Check GitHub Actions before retrying; the same form keeps its decision ID." });
  }
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
  buildStage5Modal,
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
