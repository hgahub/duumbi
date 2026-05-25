import fsModule from "node:fs";
import pathModule from "node:path";

import {
  DEFAULT_DEEPSEEK_MODEL,
  callDeepSeek,
  estimateDeepSeekCostUsd,
  normalizeText,
  truncateText,
} from "./triage-queue-refill.mjs";

export const WORKFLOW_FILE = ".github/workflows/clarification-routing.yml";
export const HUMAN_ACCEPTANCE_LABEL = "needs-human-review";
export const CLARIFICATION_PREFIX_PATTERN = /^@Clarification\b/i;
export const MARKER_PREFIX = "<!-- duumbi-clarification-routing:v2";

function nowIso() {
  return new Date().toISOString();
}

function labelsForIssue(issue) {
  return (issue?.labels || [])
    .map((label) => (typeof label === "string" ? label : label?.name))
    .filter(Boolean);
}

function stripJsonFence(value) {
  const text = normalizeText(value);
  const fenceMatch = text.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/i);
  return fenceMatch ? fenceMatch[1].trim() : text;
}

function normalizeStringArray(value, maxItemLength = 1000) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => truncateText(item, maxItemLength)).filter(Boolean);
}

function markdownList(items, fallback = "- none") {
  const values = normalizeStringArray(items);
  return values.length ? values.map((item) => `- ${item}`).join("\n") : fallback;
}

async function readJsonResponse(response) {
  const text = await response.text();
  try {
    return { text, json: text ? JSON.parse(text) : {} };
  } catch (error) {
    throw new Error(`Response was not JSON: ${truncateText(error.message, 160)}; body=${truncateText(text, 240)}`);
  }
}

export function parseClarificationDecision(content) {
  const text = stripJsonFence(content);
  try {
    const parsed = JSON.parse(text);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("Decision must be a JSON object.");
    }
    return parsed;
  } catch (firstError) {
    const start = text.indexOf("{");
    const end = text.lastIndexOf("}");
    if (start >= 0 && end > start) {
      try {
        const parsed = JSON.parse(text.slice(start, end + 1));
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) return parsed;
      } catch {
        // Fall through to the original parse error.
      }
    }
    throw new Error(`DeepSeek clarification decision was not valid JSON: ${firstError.message}`);
  }
}

export function validateClarificationDecision(decision) {
  const recommendation = normalizeText(decision?.recommendation);
  const validRecommendations = new Set(["ready_for_human_acceptance", "needs_more_clarification"]);
  if (!validRecommendations.has(recommendation)) {
    throw new Error(`DeepSeek clarification recommendation is not supported: ${recommendation || "<missing>"}`);
  }

  const refinedSummary = truncateText(decision.refined_summary, 3000);
  if (!refinedSummary) {
    throw new Error("DeepSeek clarification decision is missing refined_summary.");
  }

  const sourceCommentUrls = normalizeStringArray(decision.source_comment_urls, 500);
  if (sourceCommentUrls.length === 0) {
    throw new Error("DeepSeek clarification decision is missing source_comment_urls.");
  }

  return {
    recommendation,
    refined_summary: refinedSummary,
    clarified_requirements: normalizeStringArray(decision.clarified_requirements),
    acceptance_criteria: normalizeStringArray(decision.acceptance_criteria),
    remaining_questions: normalizeStringArray(decision.remaining_questions),
    rationale: truncateText(decision.rationale || "", 2000),
    source_comment_urls: sourceCommentUrls,
  };
}

export function buildClarificationMessages(contextPayload) {
  const schema = {
    recommendation: "ready_for_human_acceptance | needs_more_clarification",
    refined_summary: "short issue refinement based on the clarification comment",
    clarified_requirements: ["specific requirement inferred from the issue and clarification"],
    acceptance_criteria: ["observable acceptance criterion suitable for human acceptance"],
    remaining_questions: ["question that still blocks acceptance, empty when ready"],
    rationale: "short factual rationale",
    source_comment_urls: ["source GitHub comment URL used for the refinement"],
  };

  return [
    {
      role: "system",
      content: [
        "You are the DUUMBI Stage 5 clarification evaluator.",
        "Return exactly one valid JSON object and no markdown.",
        "Use the issue and explicit @Clarification comment to refine the task for human acceptance.",
        "Do not approve the issue, create specs, create PRs, modify source code, or start implementation.",
        "If material ambiguity remains, use needs_more_clarification and list the blocking questions.",
        "If the issue is clear enough for human acceptance, use ready_for_human_acceptance.",
        `Expected JSON schema example: ${JSON.stringify(schema)}`,
      ].join("\n"),
    },
    {
      role: "user",
      content: JSON.stringify(contextPayload, null, 2),
    },
  ];
}

export function buildClarificationComment(decision, { marker, sourceCommentUrl }) {
  return [
    marker,
    "## Clarification Synthesis",
    "",
    `**Recommendation:** ${decision.recommendation === "ready_for_human_acceptance" ? "Ready for human acceptance" : "Needs more clarification"}`,
    `**Source comment:** ${sourceCommentUrl}`,
    "",
    "### Refined Summary",
    "",
    decision.refined_summary,
    "",
    "### Clarified Requirements",
    "",
    markdownList(decision.clarified_requirements),
    "",
    "### Acceptance Criteria",
    "",
    markdownList(decision.acceptance_criteria),
    "",
    "### Remaining Questions",
    "",
    markdownList(decision.remaining_questions),
    "",
    "### Rationale",
    "",
    decision.rationale || "DeepSeek evaluated the explicit clarification comment against the current issue context.",
  ].join("\n");
}

export function buildSlackText({ issue, issueNumber, issueUrl, sourceCommentUrl, decision }) {
  return [
    "*DUUMBI Clarification Routing*",
    `*Issue:* <${issueUrl}|#${issueNumber} ${issue.title}>`,
    `*Comment:* ${sourceCommentUrl}`,
    `*Recommendation:* ${decision.recommendation === "ready_for_human_acceptance" ? "Ready for human acceptance" : "Needs more clarification"}`,
    "",
    `*Refined summary:* ${truncateText(decision.refined_summary, 900)}`,
    "",
    "*Remaining questions:*",
    markdownList(decision.remaining_questions),
  ].join("\n");
}

export function createGithubApi({ fetchImpl = fetch, token, owner, repo }) {
  const authHeaders = {
    Authorization: `bearer ${token}`,
    "Content-Type": "application/json",
    "User-Agent": "duumbi-clarification-routing",
  };

  const rest = async (method, urlPath, body = null) => {
    const response = await fetchImpl(`https://api.github.com${urlPath}`, {
      method,
      headers: authHeaders,
      body: body ? JSON.stringify(body) : undefined,
    });
    const { text, json } = await readJsonResponse(response);
    if (!response.ok) {
      throw new Error(`GitHub REST ${method} ${urlPath} failed: ${response.status} ${truncateText(text, 240)}`);
    }
    return json;
  };

  return {
    owner,
    repo,
    async getIssue(issueNumber) {
      return rest("GET", `/repos/${owner}/${repo}/issues/${issueNumber}`);
    },
    async listIssueComments(issueNumber) {
      return rest("GET", `/repos/${owner}/${repo}/issues/${issueNumber}/comments?per_page=100`);
    },
    async createIssueComment(issueNumber, body) {
      return rest("POST", `/repos/${owner}/${repo}/issues/${issueNumber}/comments`, { body });
    },
  };
}

function providerUsageNotCalled(reason) {
  return {
    available: false,
    reason,
    provider: null,
    model: null,
    request_count: null,
    prompt_tokens: null,
    completion_tokens: null,
    total_tokens: null,
    estimated_cost_usd: null,
    latency_ms: null,
    failure_count: null,
  };
}

function providerUsageFromDeepSeek(model, usage, latencyMs) {
  return {
    available: true,
    reason: "deepseek_api_call",
    provider: "deepseek",
    model,
    request_count: 1,
    prompt_tokens: Number.isFinite(Number(usage.prompt_tokens)) ? Number(usage.prompt_tokens) : null,
    completion_tokens: Number.isFinite(Number(usage.completion_tokens)) ? Number(usage.completion_tokens) : null,
    total_tokens: Number.isFinite(Number(usage.total_tokens)) ? Number(usage.total_tokens) : null,
    estimated_cost_usd: estimateDeepSeekCostUsd(model, usage),
    latency_ms: Number.isFinite(latencyMs) ? latencyMs : null,
    failure_count: 0,
  };
}

export function buildWorkflowMetrics({
  context,
  conclusion,
  decision = null,
  counts,
  providerUsage,
  warnings = [],
  issueNumber = null,
  generatedAt = nowIso(),
}) {
  return {
    schema_version: "duumbi.workflow_metrics.v1",
    generated_at: generatedAt,
    source: "github_actions",
    repository: `${context.repo.owner}/${context.repo.repo}`,
    workflow: {
      name: context.workflow,
      file: WORKFLOW_FILE,
      run_id: context.runId,
      run_attempt: Number(process.env.GITHUB_RUN_ATTEMPT || 1),
      event_name: context.eventName,
      actor: context.actor,
      ref: context.ref,
      sha: context.sha,
      conclusion,
      started_at: generatedAt,
      completed_at: generatedAt,
      duration_ms: null,
    },
    correlation: {
      issue_number: issueNumber,
      pr_number: null,
      stage: "clarification-routing",
      decision,
      project_status: null,
    },
    counts,
    provider_usage: providerUsage,
    privacy: {
      metadata_only: true,
      raw_prompts_included: false,
      raw_completions_included: false,
      raw_slack_payloads_included: false,
      secrets_included: false,
    },
    warnings,
  };
}

function buildIgnoredResult(reason) {
  return {
    ok: true,
    ignored: true,
    reason,
    decision: "ignored",
    issueNumber: null,
    model: null,
  };
}

async function postSlack({ fetchImpl, env, text, warnings }) {
  const token = env.SLACK_BOT_TOKEN;
  const channel = env.DUUMBI_AGENT_DISPATCH_CHANNEL_ID || env.SLACK_REVIEW_CHANNEL_ID;
  if (!token || !channel) return "not_configured";

  const response = await fetchImpl("https://slack.com/api/chat.postMessage", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json; charset=utf-8",
    },
    body: JSON.stringify({ channel, text }),
  });
  const { json } = await readJsonResponse(response);
  if (response.ok && json.ok) return "posted";

  warnings.push(`slack_notification_failed:${json.error || response.status}`);
  return "failed";
}

async function writeSummary(summary, result) {
  if (!summary) return;
  const table = [
    [{ data: "Field", header: true }, { data: "Value", header: true }],
    ["Issue", result.issueNumber ? `#${result.issueNumber}` : "none"],
    ["Decision", result.decision || "none"],
    ["Ignored", String(Boolean(result.ignored))],
    ["Reason", result.reason || "none"],
    ["Slack dispatch", result.slackNotification || "not_sent"],
    ["Model", result.model || "not_called"],
  ];
  await summary
    .addHeading("DUUMBI Clarification Routing", 2)
    .addTable(table)
    .write();
}

function buildContextFromGitHubEnv(env = process.env) {
  const eventPath = env.GITHUB_EVENT_PATH;
  const payload = eventPath ? JSON.parse(fsModule.readFileSync(eventPath, "utf8")) : {};
  const [owner, repo] = String(env.GITHUB_REPOSITORY || "/").split("/");
  return {
    workflow: env.GITHUB_WORKFLOW || "Clarification Routing",
    runId: Number(env.GITHUB_RUN_ID || 0),
    eventName: env.GITHUB_EVENT_NAME || "",
    actor: env.GITHUB_ACTOR || "unknown",
    ref: env.GITHUB_REF || "",
    sha: env.GITHUB_SHA || "",
    repo: { owner, repo },
    payload,
  };
}

function summaryFromEnv(env = process.env, fs = fsModule) {
  if (!env.GITHUB_STEP_SUMMARY) return null;
  return {
    rows: [],
    addHeading(text, level) {
      fs.appendFileSync(env.GITHUB_STEP_SUMMARY, `${"#".repeat(level)} ${text}\n\n`);
      return this;
    },
    addTable(rows) {
      const [header, ...bodyRows] = rows;
      fs.appendFileSync(env.GITHUB_STEP_SUMMARY, `| ${header.map((cell) => cell.data || cell).join(" | ")} |\n`);
      fs.appendFileSync(env.GITHUB_STEP_SUMMARY, `| ${header.map(() => "---").join(" | ")} |\n`);
      for (const row of bodyRows) {
        fs.appendFileSync(env.GITHUB_STEP_SUMMARY, `| ${row.join(" | ")} |\n`);
      }
      fs.appendFileSync(env.GITHUB_STEP_SUMMARY, "\n");
      return this;
    },
    async write() {
      return undefined;
    },
  };
}

export async function runClarificationRouting({
  env = process.env,
  context,
  core,
  summary,
  fetchImpl = fetch,
  fs = fsModule,
  path = pathModule,
  workspace = process.cwd(),
} = {}) {
  const ctx = context || buildContextFromGitHubEnv(env);
  const warnings = [];
  const metricsPath = env.DUUMBI_METRICS_PATH || "duumbi-workflow-metrics.json";
  const writeMetrics = (metrics) => {
    fs.writeFileSync(path.resolve(workspace, metricsPath), `${JSON.stringify(metrics, null, 2)}\n`);
  };

  let result = {
    ok: false,
    ignored: false,
    reason: null,
    decision: null,
    issueNumber: null,
    model: null,
    slackNotification: "not_sent",
  };

  const finishIgnored = async (reason, issueNumber = null) => {
    result = { ...result, ...buildIgnoredResult(reason), issueNumber };
    writeMetrics(buildWorkflowMetrics({
      context: ctx,
      conclusion: "success",
      decision: "ignored",
      issueNumber,
      counts: {
        issues_considered: issueNumber ? 1 : 0,
        issues_queued: 0,
        slack_notifications_attempted: 0,
        artifact_links_found: null,
        artifact_links_missing: null,
      },
      providerUsage: providerUsageNotCalled(reason),
      warnings,
    }));
    await writeSummary(summary, result);
    core?.info?.(`Clarification routing ignored: ${reason}.`);
    return result;
  };

  try {
    const isCommentEvent = ctx.eventName === "issue_comment";
    const issuePayload = isCommentEvent ? ctx.payload.issue : null;
    const manualInputs = ctx.payload.inputs || {};
    const issueNumber = isCommentEvent ? Number(issuePayload?.number || 0) : Number(manualInputs.issue_number || 0);
    const sourceCommentUrl = isCommentEvent
      ? String(ctx.payload.comment?.html_url || "")
      : String(manualInputs.comment_url || "manual workflow dispatch");
    const commentBody = isCommentEvent ? String(ctx.payload.comment?.body || "") : String(manualInputs.comment_body || "@Clarification");

    if (isCommentEvent && issuePayload?.pull_request) {
      return finishIgnored("pull_request_comment", issueNumber || null);
    }
    if (!Number.isInteger(issueNumber) || issueNumber < 1) {
      throw new Error(`Invalid issue number: ${issueNumber || "<missing>"}`);
    }
    result = { ...result, issueNumber };
    if (!CLARIFICATION_PREFIX_PATTERN.test(commentBody.trimStart())) {
      return finishIgnored("missing_clarification_prefix", issueNumber);
    }

    const token = env.GITHUB_TOKEN;
    if (!token) {
      throw new Error("GITHUB_TOKEN is not configured; failing closed before clarification routing.");
    }

    const api = createGithubApi({
      fetchImpl,
      token,
      owner: ctx.repo.owner,
      repo: ctx.repo.repo,
    });
    const issue = await api.getIssue(issueNumber);
    const labels = labelsForIssue(issue);
    if (!labels.includes(HUMAN_ACCEPTANCE_LABEL)) {
      return finishIgnored("missing_needs_human_review_label", issueNumber);
    }

    const marker = `${MARKER_PREFIX} issue=${issueNumber} comment=${sourceCommentUrl} -->`;
    const comments = await api.listIssueComments(issueNumber);
    if (comments.some((comment) => String(comment.body || "").includes(marker))) {
      return finishIgnored("duplicate_clarification_marker", issueNumber);
    }

    const deepSeekApiKey = env.DEEPSEEK_API_KEY;
    if (!deepSeekApiKey) {
      throw new Error("DEEPSEEK_API_KEY is not configured; failing closed before clarification synthesis.");
    }

    const issueUrl = issue.html_url || `https://github.com/${ctx.repo.owner}/${ctx.repo.repo}/issues/${issueNumber}`;
    const contextPayload = {
      generated_at: nowIso(),
      repository: `${ctx.repo.owner}/${ctx.repo.repo}`,
      stage: "clarification-routing",
      target_issue: {
        number: issueNumber,
        title: truncateText(issue.title, 240),
        url: issueUrl,
        labels,
        body: truncateText(issue.body || "", 6000),
      },
      source_clarification_comment: {
        url: sourceCommentUrl,
        body: truncateText(commentBody, 4000),
      },
      recent_comments: comments.slice(-12).map((comment) => ({
        url: comment.html_url,
        author: comment.user?.login || null,
        body: truncateText(comment.body || "", 2000),
        created_at: comment.created_at || null,
      })),
      policy: {
        required_issue_label: HUMAN_ACCEPTANCE_LABEL,
        trigger_prefix: "@Clarification",
        forbidden: [
          "Do not approve the issue.",
          "Do not change labels.",
          "Do not update Project V2.",
          "Do not create product specs.",
          "Do not create technical specs.",
          "Do not create PRs.",
          "Do not modify source code.",
          "Do not start implementation.",
        ],
      },
    };

    const messages = buildClarificationMessages(contextPayload);
    const deepSeekResponse = await callDeepSeek({
      fetchImpl,
      apiKey: deepSeekApiKey,
      model: env.DEEPSEEK_MODEL || DEFAULT_DEEPSEEK_MODEL,
      messages,
    });
    const decision = validateClarificationDecision(parseClarificationDecision(deepSeekResponse.content));

    await api.createIssueComment(issueNumber, buildClarificationComment(decision, {
      marker,
      sourceCommentUrl,
    }));

    const slackNotification = await postSlack({
      fetchImpl,
      env,
      text: buildSlackText({ issue, issueNumber, issueUrl, sourceCommentUrl, decision }),
      warnings,
    });

    result = {
      ok: true,
      ignored: false,
      reason: null,
      decision: decision.recommendation,
      issueNumber,
      model: deepSeekResponse.model,
      slackNotification,
    };

    writeMetrics(buildWorkflowMetrics({
      context: ctx,
      conclusion: "success",
      decision: decision.recommendation,
      issueNumber,
      counts: {
        issues_considered: 1,
        issues_queued: 0,
        slack_notifications_attempted: slackNotification === "posted" ? 1 : 0,
        artifact_links_found: decision.source_comment_urls.length,
        artifact_links_missing: decision.source_comment_urls.length ? 0 : 1,
      },
      providerUsage: providerUsageFromDeepSeek(deepSeekResponse.model, deepSeekResponse.usage, deepSeekResponse.latencyMs),
      warnings,
    }));
    await writeSummary(summary, result);
    core?.info?.(`Clarification routing decision: ${decision.recommendation} for issue #${issueNumber}.`);
    return result;
  } catch (error) {
    const message = truncateText(error.message || error, 500);
    warnings.push(message);
    writeMetrics(buildWorkflowMetrics({
      context: ctx,
      conclusion: "blocked",
      decision: "blocked",
      issueNumber: result.issueNumber,
      counts: {
        issues_considered: result.issueNumber ? 1 : null,
        issues_queued: 0,
        slack_notifications_attempted: 0,
        artifact_links_found: null,
        artifact_links_missing: null,
      },
      providerUsage: providerUsageNotCalled("clarification_routing_blocked"),
      warnings,
    }));
    await writeSummary(summary, { ...result, decision: "blocked", reason: message });
    core?.setFailed?.(message);
    return { ...result, ok: false, error: message };
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const core = {
    info(message) {
      console.log(message);
    },
    setFailed(message) {
      console.error(message);
      process.exitCode = 1;
    },
  };
  await runClarificationRouting({
    env: process.env,
    context: buildContextFromGitHubEnv(process.env),
    core,
    summary: summaryFromEnv(process.env),
  });
}
