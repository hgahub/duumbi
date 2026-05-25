import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  HUMAN_ACCEPTANCE_STATUS,
  TODO_STATUS,
  callDeepSeek,
  countOpenIssueItemsWithStatus,
  parseTriageDecision,
  runTriageQueueRefill,
  truncateText,
  validateTriageDecision,
} from "./triage-queue-refill.mjs";

function response(body, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: async () => JSON.stringify(body),
  };
}

function issueItem({ number, status, state = "OPEN", labels = [] }) {
  return {
    id: `item-${number}`,
    content: {
      id: `issue-node-${number}`,
      number,
      title: `Issue ${number}`,
      url: `https://github.com/hgahub/duumbi/issues/${number}`,
      state,
      body: `Body for issue ${number}`,
      labels: { nodes: labels.map((name) => ({ name })) },
    },
    fieldValues: {
      nodes: [
        {
          name: status,
          field: { name: "Status" },
        },
      ],
    },
  };
}

function projectWithItems(items) {
  return {
    id: "project-node",
    title: "Duumbi project",
    fields: {
      nodes: [
        {
          id: "status-field",
          name: "Status",
          options: [
            { id: "todo-option", name: TODO_STATUS },
            { id: "human-acceptance-option", name: HUMAN_ACCEPTANCE_STATUS },
          ],
        },
      ],
    },
    items: { nodes: items },
  };
}

function makeContext(inputs = {}) {
  return {
    workflow: "Triage Queue Refill",
    runId: 12345,
    eventName: "workflow_dispatch",
    actor: "tester",
    ref: "refs/heads/main",
    sha: "abc123",
    repo: { owner: "hgahub", repo: "duumbi" },
    payload: { inputs },
  };
}

function makeCore() {
  return {
    failed: null,
    infos: [],
    warnings: [],
    info(message) {
      this.infos.push(message);
    },
    warning(message) {
      this.warnings.push(message);
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

function makeWorkspace() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "duumbi-triage-refill-"));
  const inbox = path.join(workspace, "duumbi-vault", "Duumbi", "00 Inbox (ToProcess)");
  fs.mkdirSync(inbox, { recursive: true });
  fs.writeFileSync(path.join(inbox, "candidate.md"), "# Candidate\n\nShip a useful next thing.\n");
  return workspace;
}

function makeFetch({ project, decision, failAddToProject = false, failExistingLabel = false }) {
  const calls = [];
  const fetchImpl = async (url, options = {}) => {
    const body = options.body ? JSON.parse(options.body) : {};
    calls.push({ url, method: options.method || "GET", body });

    if (url === "https://api.github.com/graphql") {
      if (body.query.includes("discussions(first: 20")) {
        return response({
          data: {
            repository: {
              discussions: {
                nodes: [
                  {
                    title: "Idea discussion",
                    url: "https://github.com/hgahub/duumbi/discussions/1",
                    body: "A candidate idea.",
                    updatedAt: "2026-05-25T00:00:00Z",
                    category: { name: "Ideas" },
                  },
                ],
              },
            },
          },
        });
      }
      if (body.query.includes("projectV2(number")) {
        return response({ data: { organization: { projectV2: project }, user: null } });
      }
      if (body.query.includes("updateProjectV2ItemFieldValue")) {
        return response({ data: { updateProjectV2ItemFieldValue: { projectV2Item: { id: body.variables.itemId } } } });
      }
      if (body.query.includes("addProjectV2ItemById")) {
        return response({ data: { addProjectV2ItemById: { item: { id: "new-project-item" } } } });
      }
    }

    if (url === "https://api.deepseek.com/chat/completions") {
      return response({
        model: "deepseek-v4-pro",
        choices: [{ message: { content: JSON.stringify(decision) } }],
        usage: { prompt_tokens: 1000, completion_tokens: 200, total_tokens: 1200 },
      });
    }

    if (url.endsWith("/issues/10/comments")) {
      return response({ id: 1000, body: body.body });
    }
    if (url.endsWith("/issues/10/labels")) {
      if (failExistingLabel) {
        return response({ message: "Label does not exist" }, 404);
      }
      return response([{ name: "needs-human-review" }]);
    }
    if (url.endsWith("/repos/hgahub/duumbi/issues")) {
      return response({
        number: 99,
        node_id: "issue-node-99",
        html_url: "https://github.com/hgahub/duumbi/issues/99",
      });
    }
    if (url.endsWith("/issues/99/labels")) {
      return response([{ name: "needs-human-review" }]);
    }
    if (url.endsWith("/issues/99/comments")) {
      return response({ id: 1001, body: body.body });
    }
    if (url.endsWith("/issues/99")) {
      return response({ number: 99, state: body.state || "open" });
    }

    throw new Error(`Unexpected fetch: ${url} ${body.query || ""}`);
  };
  const originalFetch = fetchImpl;
  if (failAddToProject) {
    return {
      calls,
      fetchImpl: async (url, options = {}) => {
        const body = options.body ? JSON.parse(options.body) : {};
        if (url === "https://api.github.com/graphql" && body.query?.includes("addProjectV2ItemById")) {
          calls.push({ url, method: options.method || "GET", body });
          return response({ errors: [{ message: "Project permission denied" }] }, 200);
        }
        return originalFetch(url, options);
      },
    };
  }
  return { fetchImpl, calls };
}

test("counts only open Project V2 issues in Needs Human Acceptance", () => {
  const project = projectWithItems([
    issueItem({ number: 1, status: HUMAN_ACCEPTANCE_STATUS }),
    issueItem({ number: 2, status: HUMAN_ACCEPTANCE_STATUS, state: "CLOSED" }),
    issueItem({ number: 3, status: TODO_STATUS }),
    { id: "empty", content: null, fieldValues: { nodes: [] } },
  ]);

  assert.equal(countOpenIssueItemsWithStatus(project, HUMAN_ACCEPTANCE_STATUS), 1);
});

test("truncateText respects the requested maximum length", () => {
  assert.equal(truncateText("abcdef", 4), "a...");
  assert.equal(truncateText("abcdef", 3), "...");
  assert.equal(truncateText("abcdef", 2), "...");
  assert.equal(truncateText("abcdef", 0), "");
  assert.ok(truncateText("x".repeat(4001), 4000).length <= 4000);
});

test("callDeepSeek fails with a bounded timeout", async () => {
  const fetchImpl = (_url, options) => new Promise((_resolve, reject) => {
    options.signal.addEventListener("abort", () => {
      const error = new Error("The operation was aborted");
      error.name = "AbortError";
      reject(error);
    });
  });

  await assert.rejects(
    () => callDeepSeek({
      fetchImpl,
      apiKey: "deepseek-key",
      messages: [{ role: "user", content: "{}" }],
      timeoutMs: 1,
    }),
    /timed out after 1ms/,
  );
});

test("runTriageQueueRefill exits without model call when the queue is full", async () => {
  const project = projectWithItems([
    issueItem({ number: 1, status: HUMAN_ACCEPTANCE_STATUS }),
    issueItem({ number: 2, status: HUMAN_ACCEPTANCE_STATUS }),
    issueItem({ number: 3, status: HUMAN_ACCEPTANCE_STATUS }),
    issueItem({ number: 4, status: TODO_STATUS }),
  ]);
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "duumbi-triage-full-"));
  const { fetchImpl, calls } = makeFetch({ project, decision: { action: "no_action" } });
  const core = makeCore();

  const result = await runTriageQueueRefill({
    env: {
      GH_PROJECT_PAT: "pat",
      DUUMBI_PROJECT_NUMBER: "1",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.decision, "not_needed");
  assert.equal(core.failed, null);
  assert.equal(calls.some((call) => call.url.includes("deepseek.com")), false);
  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.counts.issues_queued, 0);
  assert.equal(metrics.provider_usage.reason, "human_acceptance_queue_full");
});

test("runTriageQueueRefill routes one eligible Todo issue to human acceptance", async () => {
  const project = projectWithItems([
    issueItem({ number: 1, status: HUMAN_ACCEPTANCE_STATUS }),
    issueItem({ number: 2, status: HUMAN_ACCEPTANCE_STATUS }),
    issueItem({ number: 10, status: TODO_STATUS }),
  ]);
  const decision = {
    action: "route_existing_issue",
    existing_issue_number: 10,
    source_links: ["duumbi-vault/Duumbi/00 Inbox (ToProcess)/candidate.md"],
    rationale: "Best next actionable candidate.",
  };
  const workspace = makeWorkspace();
  const { fetchImpl, calls } = makeFetch({ project, decision });
  const core = makeCore();

  const result = await runTriageQueueRefill({
    env: {
      GH_PROJECT_PAT: "pat",
      DEEPSEEK_API_KEY: "deepseek-key",
      DUUMBI_PROJECT_NUMBER: "1",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(core.failed, null);
  assert.equal(result.decision, "route_existing_issue");
  assert.equal(result.issueNumber, 10);
  assert.equal(calls.filter((call) => call.url === "https://api.deepseek.com/chat/completions").length, 1);

  const statusUpdates = calls.filter((call) => call.body.query?.includes("updateProjectV2ItemFieldValue"));
  assert.equal(statusUpdates.length, 1);
  assert.equal(statusUpdates[0].body.variables.itemId, "item-10");
  assert.equal(statusUpdates[0].body.variables.optionId, "human-acceptance-option");

  const labelCalls = calls.filter((call) => call.url.endsWith("/issues/10/labels"));
  assert.equal(labelCalls.length, 1);
  assert.deepEqual(labelCalls[0].body.labels, ["needs-human-review"]);
  assert.ok(calls.indexOf(labelCalls[0]) < calls.indexOf(statusUpdates[0]));

  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.counts.issues_queued, 1);
  assert.equal(metrics.counts.slack_notifications_attempted, 0);
  assert.equal(metrics.provider_usage.provider, "deepseek");
});

test("runTriageQueueRefill blocks before status update when existing issue label fails", async () => {
  const project = projectWithItems([
    issueItem({ number: 1, status: HUMAN_ACCEPTANCE_STATUS }),
    issueItem({ number: 10, status: TODO_STATUS }),
  ]);
  const decision = {
    action: "route_existing_issue",
    existing_issue_number: 10,
    source_links: ["duumbi-vault/Duumbi/00 Inbox (ToProcess)/candidate.md"],
    rationale: "Best next actionable candidate.",
  };
  const workspace = makeWorkspace();
  const { fetchImpl, calls } = makeFetch({ project, decision, failExistingLabel: true });
  const core = makeCore();

  const result = await runTriageQueueRefill({
    env: {
      GH_PROJECT_PAT: "pat",
      DEEPSEEK_API_KEY: "deepseek-key",
      DUUMBI_PROJECT_NUMBER: "1",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ok, false);
  assert.match(core.failed, /GitHub REST POST/);
  assert.equal(calls.some((call) => call.body.query?.includes("updateProjectV2ItemFieldValue")), false);
  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.counts.issues_considered, 2);
  assert.equal(metrics.provider_usage.request_count, null);
});

test("runTriageQueueRefill closes a created issue when Project insertion fails", async () => {
  const project = projectWithItems([
    issueItem({ number: 1, status: HUMAN_ACCEPTANCE_STATUS }),
    issueItem({ number: 2, status: HUMAN_ACCEPTANCE_STATUS }),
  ]);
  const decision = {
    action: "create_issue",
    source_links: ["duumbi-vault/Duumbi/00 Inbox (ToProcess)/candidate.md"],
    rationale: "No existing Todo issue represents the candidate.",
    issue: {
      title: "New triage candidate",
      body: "Create a new triage candidate.",
    },
  };
  const workspace = makeWorkspace();
  const { fetchImpl, calls } = makeFetch({ project, decision, failAddToProject: true });
  const core = makeCore();

  const result = await runTriageQueueRefill({
    env: {
      GH_PROJECT_PAT: "pat",
      DEEPSEEK_API_KEY: "deepseek-key",
      DUUMBI_PROJECT_NUMBER: "1",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
  });

  assert.equal(result.ok, false);
  assert.match(core.failed, /was closed after incomplete queue update/);
  assert.equal(calls.some((call) => call.url.endsWith("/issues/99/comments")), true);
  const closeCalls = calls.filter((call) => call.url.endsWith("/issues/99") && call.method === "PATCH");
  assert.equal(closeCalls.length, 1);
  assert.equal(closeCalls[0].body.state, "closed");
  assert.equal(calls.some((call) => call.url.endsWith("/issues/99/labels")), false);
});

test("triage decision validation rejects malformed or unsafe model output", () => {
  const contextPayload = {
    project: {
      eligible_todo_issues: [{ number: 7, item_id: "item-7" }],
    },
  };

  assert.throws(() => parseTriageDecision("not-json"), /not valid JSON/);
  assert.throws(
    () => validateTriageDecision({ action: "delete_issue" }, contextPayload),
    /not supported/,
  );
  assert.throws(
    () => validateTriageDecision({ action: "route_existing_issue", existing_issue_number: 7 }, contextPayload),
    /source_links/,
  );
  assert.throws(
    () => validateTriageDecision({
      action: "route_existing_issue",
      existing_issue_number: 99,
      source_links: ["https://github.com/hgahub/duumbi/issues/99"],
    }, contextPayload),
    /not an eligible Todo issue/,
  );
  assert.throws(
    () => validateTriageDecision({
      action: "create_issue",
      source_links: ["duumbi-vault/Duumbi/00 Inbox (ToProcess)/candidate.md"],
      issue: { body: "Missing title." },
    }, contextPayload),
    /issue.title/,
  );
  assert.throws(
    () => validateTriageDecision({
      action: "create_issue",
      source_links: ["duumbi-vault/Duumbi/00 Inbox (ToProcess)/candidate.md"],
      issue: { title: "Missing body" },
    }, contextPayload),
    /issue.body/,
  );
});
