import { createHmac, randomUUID, timingSafeEqual } from "node:crypto";
import { createServer } from "node:http";

const TARGET_STATUS = "Needs Human Acceptance";
const DISPATCH_EVENT_TYPE = "duumbi-human-acceptance-needed";
const MAX_BODY_BYTES = 1024 * 1024;

export function verifySignature(rawBody, signatureHeader, secret) {
  if (!secret) {
    throw new Error("GITHUB_WEBHOOK_SECRET is required");
  }
  if (!signatureHeader?.startsWith("sha256=")) {
    return false;
  }

  const expected = `sha256=${createHmac("sha256", secret).update(rawBody).digest("hex")}`;
  const expectedBuffer = Buffer.from(expected);
  const actualBuffer = Buffer.from(signatureHeader);
  return expectedBuffer.length === actualBuffer.length
    && timingSafeEqual(expectedBuffer, actualBuffer);
}

export function extractStatusChange(payload) {
  const changes = payload?.changes;
  const fieldValue = changes?.field_value || changes?.fieldValue || changes?.field || {};
  const fieldName = String(
    fieldValue.field_name
      || fieldValue.fieldName
      || fieldValue.name
      || fieldValue.field?.name
      || "",
  );
  const from = fieldValue.from || fieldValue.previous || {};
  const to = fieldValue.to || fieldValue.current || fieldValue.new_value || fieldValue.newValue || {};
  const toName = String(
    to.name
      || to.option_name
      || to.optionName
      || to.value
      || fieldValue.to_name
      || fieldValue.toName
      || "",
  );

  return { fieldName, toName, from };
}

export function extractIssue(payload) {
  const item = payload?.projects_v2_item || {};
  const content = item.content || item.content_node || payload?.issue || {};
  const htmlUrl = content.html_url || content.url || item.content_url || "";
  const issueMatch = String(htmlUrl).match(/^https:\/\/github\.com\/([^/]+)\/([^/]+)\/issues\/(\d+)$/);
  const number = Number.parseInt(content.number || issueMatch?.[3] || "", 10);

  return {
    issueUrl: issueMatch ? issueMatch[0] : String(htmlUrl),
    issueNumber: Number.isInteger(number) ? number : null,
  };
}

export function buildDispatchPayload(payload, deliveryId = randomUUID()) {
  const action = payload?.action;
  if (!["edited", "updated"].includes(action)) {
    return null;
  }

  const { fieldName, toName } = extractStatusChange(payload);
  if (fieldName !== "Status" || toName !== TARGET_STATUS) {
    return null;
  }

  const { issueUrl, issueNumber } = extractIssue(payload);
  if (!issueUrl || !issueNumber) {
    throw new Error("projects_v2_item payload did not include a resolvable GitHub issue URL");
  }

  const item = payload.projects_v2_item || {};
  const project = item.project || payload.projects_v2 || {};

  return {
    issue_url: issueUrl,
    issue_number: issueNumber,
    project_item_id: item.node_id || item.id || "",
    project_url: project.html_url || project.url || "",
    status: TARGET_STATUS,
    triggered_by: payload.sender?.login || "unknown",
    correlation_id: deliveryId,
  };
}

async function dispatchRepositoryEvent(dispatchPayload, config) {
  const response = await fetch(
    `https://api.github.com/repos/${config.owner}/${config.repo}/dispatches`,
    {
      method: "POST",
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${config.token}`,
        "Content-Type": "application/json",
        "User-Agent": "duumbi-human-acceptance-bridge",
        "X-GitHub-Api-Version": "2022-11-28",
      },
      body: JSON.stringify({
        event_type: DISPATCH_EVENT_TYPE,
        client_payload: dispatchPayload,
      }),
    },
  );

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`repository_dispatch failed: ${response.status} ${body}`);
  }
}

function readRequestBody(request) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let totalBytes = 0;
    request.on("data", (chunk) => {
      totalBytes += chunk.length;
      if (totalBytes > MAX_BODY_BYTES) {
        reject(new Error("webhook payload exceeds 1 MiB"));
        request.destroy();
        return;
      }
      chunks.push(chunk);
    });
    request.on("error", reject);
    request.on("end", () => resolve(Buffer.concat(chunks)));
  });
}

function jsonResponse(response, statusCode, body) {
  response.writeHead(statusCode, { "Content-Type": "application/json" });
  response.end(`${JSON.stringify(body)}\n`);
}

export function createBridgeServer(config) {
  return createServer(async (request, response) => {
    try {
      if (request.method === "GET" && request.url === "/healthz") {
        jsonResponse(response, 200, { ok: true });
        return;
      }

      if (request.method !== "POST" || request.url !== "/webhook") {
        jsonResponse(response, 404, { error: "not_found" });
        return;
      }

      const event = request.headers["x-github-event"];
      const deliveryId = request.headers["x-github-delivery"] || randomUUID();
      if (event !== "projects_v2_item") {
        jsonResponse(response, 202, { ignored: true, reason: "event", event });
        return;
      }

      const rawBody = await readRequestBody(request);
      if (!verifySignature(rawBody, request.headers["x-hub-signature-256"], config.webhookSecret)) {
        jsonResponse(response, 401, { error: "invalid_signature" });
        return;
      }

      const payload = JSON.parse(rawBody.toString("utf8"));
      const dispatchPayload = buildDispatchPayload(payload, String(deliveryId));
      if (!dispatchPayload) {
        jsonResponse(response, 202, { ignored: true, reason: "status_filter" });
        return;
      }

      await dispatchRepositoryEvent(dispatchPayload, config);
      jsonResponse(response, 202, { dispatched: true, payload: dispatchPayload });
    } catch (error) {
      console.error(error);
      jsonResponse(response, 500, { error: String(error.message || error) });
    }
  });
}

function runSelfTest() {
  const payload = {
    action: "edited",
    changes: {
      field_value: {
        field_name: "Status",
        from: { name: "Todo" },
        to: { name: TARGET_STATUS },
      },
    },
    projects_v2_item: {
      node_id: "PVTI_test",
      content: {
        html_url: "https://github.com/hgahub/duumbi/issues/420",
        number: 420,
      },
      project: {
        html_url: "https://github.com/orgs/hgahub/projects/1",
      },
    },
    sender: { login: "octocat" },
  };

  const dispatchPayload = buildDispatchPayload(payload, "delivery-1");
  if (dispatchPayload?.issue_number !== 420) {
    throw new Error("self-test failed to extract issue number");
  }
  if (dispatchPayload?.status !== TARGET_STATUS) {
    throw new Error("self-test failed to extract target status");
  }
  console.log("self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else if (import.meta.url === `file://${process.argv[1]}`) {
  const config = {
    owner: process.env.GITHUB_OWNER || "hgahub",
    repo: process.env.GITHUB_REPO || "duumbi",
    token: process.env.GITHUB_DISPATCH_TOKEN,
    webhookSecret: process.env.GITHUB_WEBHOOK_SECRET,
  };

  if (!config.token) {
    throw new Error("GITHUB_DISPATCH_TOKEN is required");
  }

  const port = Number.parseInt(process.env.PORT || "3000", 10);
  createBridgeServer(config).listen(port, () => {
    console.log(`DUUMBI human acceptance bridge listening on :${port}`);
  });
}
