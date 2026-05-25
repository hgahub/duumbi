import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  HUMAN_ACCEPTANCE_LABEL,
  MARKER_PREFIX,
  parseClarificationDecision,
  runClarificationRouting,
  validateClarificationDecision,
} from "./clarification-routing.mjs";

function response(body, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: async () => JSON.stringify(body),
  };
}

function makeContext({
  body = "@Clarification Please refine this issue.",
  issueNumber = 584,
  pullRequest = false,
} = {}) {
  return {
    workflow: "Clarification Routing",
    runId: 12345,
    eventName: "issue_comment",
    actor: "tester",
    ref: "refs/heads/main",
    sha: "abc123",
    repo: { owner: "hgahub", repo: "duumbi" },
    payload: {
      issue: {
        number: issueNumber,
        pull_request: pullRequest ? { url: "https://api.github.com/repos/hgahub/duumbi/pulls/584" } : undefined,
      },
      comment: {
        html_url: `https://github.com/hgahub/duumbi/issues/${issueNumber}#issuecomment-1`,
        body,
      },
    },
  };
}

function makeCore() {
  return {
    failed: null,
    infos: [],
    info(message) {
      this.infos.push(message);
    },
    setFailed(message) {
      this.failed = message;
    },
  };
}

function makeSummary() {
  return {
    rows: [],
    addHeading() {
      return this;
    },
    addTable(rows) {
      this.rows = rows;
      return this;
    },
    async write() {
      return undefined;
    },
  };
}

function makeWorkspace(prefix = "duumbi-clarification-routing-") {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function makeFetch({
  labels = [HUMAN_ACCEPTANCE_LABEL],
  comments = [],
  decision = {
    recommendation: "ready_for_human_acceptance",
    refined_summary: "Clarified issue summary.",
    clarified_requirements: ["Keep the route bounded to Stage 5."],
    acceptance_criteria: ["A synthesis comment is posted."],
    remaining_questions: [],
    rationale: "The clarification is sufficient.",
    source_comment_urls: ["https://github.com/hgahub/duumbi/issues/584#issuecomment-1"],
  },
  slackOk = true,
} = {}) {
  const calls = [];
  const fetchImpl = async (url, options = {}) => {
    const body = options.body ? JSON.parse(options.body) : null;
    calls.push({ url, method: options.method || "GET", body });

    if (url.endsWith("/repos/hgahub/duumbi/issues/584")) {
      return response({
        number: 584,
        title: "Clarify Stage 5 issue",
        body: "Original issue body.",
        html_url: "https://github.com/hgahub/duumbi/issues/584",
        labels: labels.map((name) => ({ name })),
      });
    }

    if (url.includes("/repos/hgahub/duumbi/issues/584/comments?per_page=100")) {
      const parsed = new URL(url);
      const page = Number(parsed.searchParams.get("page") || 1);
      const start = (page - 1) * 100;
      return response(comments.slice(start, start + 100));
    }

    if (url.endsWith("/repos/hgahub/duumbi/issues/584/comments")) {
      return response({ id: 1000, body: body.body });
    }

    if (url === "https://api.deepseek.com/chat/completions") {
      return response({
        model: "deepseek-v4-pro",
        choices: [{ message: { content: typeof decision === "string" ? decision : JSON.stringify(decision) } }],
        usage: { prompt_tokens: 900, completion_tokens: 180, total_tokens: 1080 },
      });
    }

    if (url === "https://slack.com/api/chat.postMessage") {
      return response(slackOk ? { ok: true, ts: "123.456" } : { ok: false, error: "channel_not_found" });
    }

    throw new Error(`Unexpected fetch: ${url}`);
  };

  return { fetchImpl, calls };
}

test("runClarificationRouting ignores @Copilot issue comments", async () => {
  const workspace = makeWorkspace("duumbi-clarification-copilot-");
  const { fetchImpl, calls } = makeFetch();
  const core = makeCore();

  const result = await runClarificationRouting({
    env: {
      GITHUB_TOKEN: "token",
      DEEPSEEK_API_KEY: "deepseek",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext({ body: "@Copilot Foglald össze röviden." }),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ignored, true);
  assert.equal(result.reason, "missing_clarification_prefix");
  assert.equal(calls.length, 0);
  assert.equal(core.failed, null);
  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.provider_usage.reason, "missing_clarification_prefix");
});

test("runClarificationRouting ignores @Codex issue comments", async () => {
  const workspace = makeWorkspace("duumbi-clarification-codex-");
  const { fetchImpl, calls } = makeFetch();
  const core = makeCore();

  const result = await runClarificationRouting({
    env: {
      GITHUB_TOKEN: "token",
      DEEPSEEK_API_KEY: "deepseek",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext({ body: "@Codex Is this scope right?" }),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ignored, true);
  assert.equal(result.reason, "missing_clarification_prefix");
  assert.equal(calls.length, 0);
  assert.equal(core.failed, null);
});

test("runClarificationRouting requires needs-human-review label", async () => {
  const workspace = makeWorkspace("duumbi-clarification-label-");
  const { fetchImpl, calls } = makeFetch({ labels: ["enhancement"] });
  const core = makeCore();

  const result = await runClarificationRouting({
    env: {
      GITHUB_TOKEN: "token",
      DEEPSEEK_API_KEY: "deepseek",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ignored, true);
  assert.equal(result.reason, "missing_needs_human_review_label");
  assert.equal(calls.some((call) => call.url === "https://api.deepseek.com/chat/completions"), false);
  assert.equal(core.failed, null);
});

test("runClarificationRouting ignores duplicate clarification markers", async () => {
  const workspace = makeWorkspace("duumbi-clarification-duplicate-");
  const marker = `${MARKER_PREFIX} issue=584 comment=https://github.com/hgahub/duumbi/issues/584#issuecomment-1 -->`;
  const { fetchImpl, calls } = makeFetch({
    comments: [{ html_url: "https://github.com/hgahub/duumbi/issues/584#issuecomment-2", body: `${marker}\nAlready handled.` }],
  });
  const core = makeCore();

  const result = await runClarificationRouting({
    env: {
      GITHUB_TOKEN: "token",
      DEEPSEEK_API_KEY: "deepseek",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ignored, true);
  assert.equal(result.reason, "duplicate_clarification_marker");
  assert.equal(calls.some((call) => call.url === "https://api.deepseek.com/chat/completions"), false);
  assert.equal(core.failed, null);
});

test("runClarificationRouting paginates comments before duplicate detection", async () => {
  const workspace = makeWorkspace("duumbi-clarification-pagination-");
  const marker = `${MARKER_PREFIX} issue=584 comment=https://github.com/hgahub/duumbi/issues/584#issuecomment-1 -->`;
  const comments = Array.from({ length: 120 }, (_value, index) => ({
    html_url: `https://github.com/hgahub/duumbi/issues/584#issuecomment-${index}`,
    body: index === 115 ? `${marker}\nAlready handled on a later page.` : `Comment ${index}`,
  }));
  const { fetchImpl, calls } = makeFetch({ comments });
  const core = makeCore();

  const result = await runClarificationRouting({
    env: {
      GITHUB_TOKEN: "token",
      DEEPSEEK_API_KEY: "deepseek",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ignored, true);
  assert.equal(result.reason, "duplicate_clarification_marker");
  assert.equal(
    calls.filter((call) => call.url.includes("/repos/hgahub/duumbi/issues/584/comments?per_page=100")).length,
    2,
  );
  assert.equal(calls.some((call) => call.url === "https://api.deepseek.com/chat/completions"), false);
  assert.equal(core.failed, null);
});

test("runClarificationRouting posts synthesis comment and Slack notification", async () => {
  const workspace = makeWorkspace("duumbi-clarification-success-");
  const { fetchImpl, calls } = makeFetch({
    comments: [
      {
        html_url: "https://github.com/hgahub/duumbi/issues/584#issuecomment-0",
        body: "Previous discussion.",
        user: { login: "dev" },
        created_at: "2026-05-25T00:00:00Z",
      },
    ],
  });
  const core = makeCore();

  const result = await runClarificationRouting({
    env: {
      GITHUB_TOKEN: "token",
      DEEPSEEK_API_KEY: "deepseek",
      SLACK_BOT_TOKEN: "slack",
      SLACK_REVIEW_CHANNEL_ID: "C123",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ok, true);
  assert.equal(result.ignored, false);
  assert.equal(result.decision, "ready_for_human_acceptance");
  assert.equal(result.slackNotification, "posted");
  assert.equal(core.failed, null);

  const deepSeekCalls = calls.filter((call) => call.url === "https://api.deepseek.com/chat/completions");
  assert.equal(deepSeekCalls.length, 1);
  assert.equal(deepSeekCalls[0].body.response_format.type, "json_object");

  const commentCalls = calls.filter((call) => call.url.endsWith("/repos/hgahub/duumbi/issues/584/comments") && call.method === "POST");
  assert.equal(commentCalls.length, 1);
  assert.match(commentCalls[0].body.body, /Clarification Synthesis/);
  assert.match(commentCalls[0].body.body, /duumbi-clarification-routing:v2/);

  const slackCalls = calls.filter((call) => call.url === "https://slack.com/api/chat.postMessage");
  assert.equal(slackCalls.length, 1);
  assert.equal(slackCalls[0].body.channel, "C123");

  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.counts.slack_notifications_attempted, 1);
  assert.equal(metrics.provider_usage.provider, "deepseek");
  assert.equal(metrics.privacy.raw_prompts_included, false);
});

test("runClarificationRouting fails closed on invalid DeepSeek JSON", async () => {
  const workspace = makeWorkspace("duumbi-clarification-invalid-json-");
  const { fetchImpl } = makeFetch({ decision: "not-json" });
  const core = makeCore();

  const result = await runClarificationRouting({
    env: {
      GITHUB_TOKEN: "token",
      DEEPSEEK_API_KEY: "deepseek",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ok, false);
  assert.match(core.failed, /not valid JSON/);
  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.workflow.conclusion, "blocked");
});

test("clarification decision validation rejects unsafe model output", () => {
  assert.throws(() => parseClarificationDecision("not-json"), /not valid JSON/);
  assert.throws(
    () => validateClarificationDecision({ recommendation: "approve" }),
    /not supported/,
  );
  assert.throws(
    () => validateClarificationDecision({
      recommendation: "ready_for_human_acceptance",
      source_comment_urls: ["https://github.com/hgahub/duumbi/issues/584#issuecomment-1"],
    }),
    /refined_summary/,
  );
  assert.throws(
    () => validateClarificationDecision({
      recommendation: "ready_for_human_acceptance",
      refined_summary: "Clear enough.",
    }),
    /source_comment_urls/,
  );
});
