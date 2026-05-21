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
    const payload = JSON.parse(params.get("payload"));

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

    // Trigger GitHub repository_dispatch
    const githubRepo = process.env.GITHUB_REPO || "hgahub/duumbi";
    const dispatchRes = await fetch(
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
        body: JSON.stringify({
          event_type: "stage-approval",
          client_payload: {
            stage: actionData.stage,
            issue_number: actionData.issue_number,
            decision: actionData.decision,
            rationale: actionData.rationale || `Approved by ${reviewer}`,
            pr_number: actionData.pr_number || 0,
            reviewer,
            slack_response_url: payload.response_url,
          },
        }),
      },
    );

    if (!dispatchRes.ok) {
      const errText = await dispatchRes.text();
      context.error("GitHub dispatch failed:", dispatchRes.status, errText);

      if (payload.response_url) {
        await fetch(payload.response_url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            replace_original: false,
            text: `⚠️ Approval workflow trigger failed (HTTP ${dispatchRes.status}). Please use the manual workflow dispatch as fallback.`,
          }),
        });
      }
      return { jsonBody: { text: "Dispatch failed." } };
    }

    // Acknowledge in Slack thread
    if (payload.response_url) {
      await fetch(payload.response_url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          replace_original: false,
          text: `⏳ Stage ${actionData.stage} *${actionData.decision}* triggered by <@${user.id}> — GitHub Actions workflow running…`,
        }),
      });
    }

    // Slack expects a 200 within 3 seconds for interactions
    return { status: 200, body: "" };
  },
});

// ── Helpers ──────────────────────────────────────────────────────

function verifySlackSignature(body, timestamp, signature, signingSecret) {
  if (!timestamp || !signature || !signingSecret) return false;

  // Reject requests older than 5 minutes
  const now = Math.floor(Date.now() / 1000);
  if (Math.abs(now - Number(timestamp)) > 300) return false;

  const baseString = `v0:${timestamp}:${body}`;
  const computed =
    "v0=" + crypto.createHmac("sha256", signingSecret).update(baseString).digest("hex");

  return crypto.timingSafeEqual(Buffer.from(computed), Buffer.from(signature));
}
