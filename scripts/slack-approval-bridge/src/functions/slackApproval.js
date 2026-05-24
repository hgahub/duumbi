const { app } = require("@azure/functions");
const crypto = require("node:crypto");

/**
 * Slack Approval Bridge — Azure Function
 *
 * Bridges Slack interactive button clicks → GitHub repository_dispatch
 * so that stage approvals execute deterministically via GitHub Actions
 * instead of spawning an LLM-based Oz agent.
 *
 * App Settings (configure in Azure Function App):
 *   SLACK_SIGNING_SECRET  — Slack app signing secret
 *   GITHUB_TOKEN          — GitHub PAT with repo scope
 *   GITHUB_REPO           — "owner/repo" (e.g. "hgahub/duumbi")
 */

app.http("slack-approval", {
  methods: ["POST"],
  authLevel: "anonymous",
  handler: async (request, context) => {
    const body = await request.text();
    const timestamp = request.headers.get("X-Slack-Request-Timestamp");
    const signature = request.headers.get("X-Slack-Signature");

    if (!verifySlackSignature(body, timestamp, signature, process.env.SLACK_SIGNING_SECRET)) {
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

    if (payload.type === "message_action" || payload.type === "shortcut") {
      const responseUrl = payload.response_url;
      const githubRepo = process.env.GITHUB_REPO || "hgahub/duumbi";
      const clientPayload = {
        surface: "Slack",
        callback_id: payload.callback_id || "",
        channel_id: payload.channel?.id || payload.channel_id || "",
        message_ts: payload.message?.ts || payload.message_ts || "",
        thread_ts: payload.message?.thread_ts || payload.message?.ts || "",
        user_id: payload.user?.id || "",
        user_name: payload.user?.username || payload.user?.name || "",
        text: payload.message?.text || "",
        slack_response_url: responseUrl,
      };
      dispatchGenericAsync(githubRepo, "slack-intake", clientPayload, responseUrl, "Slack intake", context);
      return { status: 200, body: "" };
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
    const githubRepo = process.env.GITHUB_REPO || "hgahub/duumbi";
    const clientPayload = {
      stage: actionData.stage,
      issue_number: actionData.issue_number,
      decision: actionData.decision,
      rationale: actionData.rationale || fallbackRationale,
      pr_number: actionData.pr_number || 0,
      cycle: actionData.cycle || 0,
      reviewer,
      slack_response_url: responseUrl,
    };
    const eventType = eventTypeForStage(actionData.stage);

    // Fire-and-forget: dispatch to GitHub + post Slack thread update
    dispatchAsync(githubRepo, eventType, clientPayload, responseUrl, actionData, user, context);

    // Return 200 immediately so Slack doesn't retry
    return { status: 200, body: "" };
  },
});

// ── Helpers ────────────────────────────────────────────────────────────

async function dispatchAsync(githubRepo, eventType, clientPayload, responseUrl, actionData, user, context) {
  try {
    const res = await fetch(
      `https://api.github.com/repos/${githubRepo}/dispatches`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
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
        await fetch(responseUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            replace_original: false,
            text: `⚠️ Approval workflow trigger failed (HTTP ${res.status}). Please use the manual workflow dispatch as fallback.`,
          }),
        });
      }
      return;
    }

    if (responseUrl) {
      await fetch(responseUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          replace_original: false,
          text: `⏳ Stage ${actionData.stage} *${actionData.decision}* triggered by <@${user.id}> — GitHub Actions workflow running…`,
        }),
      });
    }
  } catch (err) {
    context.error("dispatchAsync error:", err);
  }
}

async function dispatchGenericAsync(githubRepo, eventType, clientPayload, responseUrl, label, context) {
  try {
    const res = await fetch(
      `https://api.github.com/repos/${githubRepo}/dispatches`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
          "Content-Type": "application/json",
          "User-Agent": "duumbi-slack-approval-bridge/1.0",
        },
        body: JSON.stringify({ event_type: eventType, client_payload: clientPayload }),
      },
    );

    if (responseUrl) {
      await fetch(responseUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          replace_original: false,
          text: res.ok
            ? `⏳ ${label} triggered — GitHub Actions workflow running…`
            : `⚠️ ${label} workflow trigger failed (HTTP ${res.status}).`,
        }),
      });
    }
  } catch (err) {
    context.error("dispatchGenericAsync error:", err);
  }
}

function eventTypeForStage(stage) {
  const normalized = String(stage || "");
  if (normalized === "10") return "stage10-authorization";
  if (normalized === "11") return "stage11-merge-decision";
  return "stage-approval";
}

function verifySlackSignature(body, timestamp, signature, signingSecret) {
  if (!timestamp || !signature || !signingSecret) return false;

  // Reject non-numeric or stale timestamps (>5 minutes)
  const ts = Number(timestamp);
  if (!Number.isFinite(ts)) return false;
  const now = Math.floor(Date.now() / 1000);
  if (Math.abs(now - ts) > 300) return false;

  const baseString = `v0:${timestamp}:${body}`;
  const computed =
    "v0=" + crypto.createHmac("sha256", signingSecret).update(baseString).digest("hex");

  const a = Buffer.from(computed);
  const b = Buffer.from(signature);
  if (a.length !== b.length) return false;
  return crypto.timingSafeEqual(a, b);
}
