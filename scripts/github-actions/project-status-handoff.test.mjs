import assert from "node:assert/strict";
import test from "node:test";
import {
  buildProjectStatusBlocks,
  buildSlackPostPayload,
  combinedSpecIssueNumberFromFiles,
  commentsHaveMarker,
  correctionKindForEvent,
  correctionMarker,
  planCorrectionPost,
  projectStatusButtonValue,
  projectStatusButtonsEnabled,
  readDuumbiProjectStatus,
  readyForBuildMarker,
  reopenOccurrenceId,
  runReadyForBuildHandoff,
  shouldIncludeUndoDone,
} from "./project-status-handoff.mjs";

test("project status buttons default off and require the enablement var", () => {
  assert.equal(projectStatusButtonsEnabled({}), false);
  assert.equal(projectStatusButtonsEnabled({ DUUMBI_PROJECT_STATUS_SLACK_BUTTONS: "false" }), false);
  assert.equal(projectStatusButtonsEnabled({ DUUMBI_PROJECT_STATUS_SLACK_BUTTONS: "true" }), true);
});

test("gated Slack posts omit blocks until the enablement var is on", () => {
  const issue = { number: 789, title: "feat", html_url: "https://github.com/hgahub/duumbi/issues/789" };
  const blocks = buildProjectStatusBlocks({
    issue,
    owner: "hgahub",
    repo: "duumbi",
    includeUndoDone: true,
  });
  const values = JSON.stringify(blocks);
  assert.match(values, /action_type\\":\\"project_status/);
  assert.match(values, /project_status_ready_for_build/);
  assert.match(values, /project_status_undo_done/);
  assert.match(values, /project-status\.yml/);
  assert.doesNotMatch(values, /stage-approval\.yml/);
  assert.equal(projectStatusButtonValue(789, "ready-for-build").length < 2000, true);

  const gated = buildSlackPostPayload({
    channel: "C1",
    text: "hello",
    blocksEnabled: false,
    blocks,
  });
  assert.equal("blocks" in gated, false);

  const live = buildSlackPostPayload({
    channel: "C1",
    text: "hello",
    blocksEnabled: true,
    blocks,
  });
  assert.equal(live.blocks.length > 0, true);
});

test("Undo Done is included when Status is Done or the query failed", () => {
  assert.equal(shouldIncludeUndoDone("Done", null), true);
  assert.equal(shouldIncludeUndoDone("Spec Needed", null), false);
  assert.equal(shouldIncludeUndoDone(null, "query_failed"), true);
});

test("combined-spec files map to the issue number and reject single-file PRs", () => {
  assert.equal(combinedSpecIssueNumberFromFiles([
    { filename: "specs/DUUMBI-779/PRODUCT.md" },
    { filename: "specs/DUUMBI-779/TECHNICAL.md" },
  ]), 779);
  assert.equal(combinedSpecIssueNumberFromFiles([
    { filename: "specs/DUUMBI-779/PRODUCT.md" },
  ]), null);
  assert.equal(combinedSpecIssueNumberFromFiles([
    { filename: "specs/DUUMBI-779/PRODUCT.md" },
    { filename: "README.md" },
  ]), null);
});

test("hourly cron is not a correction producer event", () => {
  assert.equal(correctionKindForEvent({ eventName: "schedule" }), null);
  assert.equal(correctionKindForEvent({
    eventName: "issues",
    eventAction: "reopened",
  }), "reopened");
  assert.equal(correctionKindForEvent({
    eventName: "pull_request",
    mergedPullRequest: true,
    combinedSpecIssueNumber: 779,
  }), "combined-spec");
});

test("duplicate reopen occurrence posts once; a later reopen posts again", () => {
  const first = correctionMarker({ issueNumber: 779, kind: "reopened", occurrence: "111222333" });
  const second = correctionMarker({ issueNumber: 779, kind: "reopened", occurrence: "444555666" });
  const comments = [];
  assert.equal(planCorrectionPost({ comments, marker: first }).post, true);
  comments.push({ body: `${first}\nposted` });
  assert.equal(planCorrectionPost({ comments, marker: first }).post, false);
  assert.equal(planCorrectionPost({ comments, marker: second }).post, true);
  assert.equal(commentsHaveMarker(comments, readyForBuildMarker(779)), false);
});

test("Ready-for-Build v1 marker does not suppress a correction occurrence", () => {
  const comments = [{ body: `${readyForBuildMarker(779)}\nReady for Build Slack handoff posted.` }];
  const marker = correctionMarker({
    issueNumber: 779,
    kind: "reopened",
    occurrence: reopenOccurrenceId({ delivery: "111222333" }),
  });
  assert.equal(planCorrectionPost({ comments, marker }).post, true);
});

test("readDuumbiProjectStatus ignores unrelated Project boards", async () => {
  const fetchImpl = async () => ({
    json: async () => ({
      data: {
        repository: {
          issue: {
            projectItems: {
              nodes: [
                {
                  project: { number: 99, owner: { login: "hgahub" } },
                  fieldValues: { nodes: [{ field: { name: "Status" }, name: "Done" }] },
                },
                {
                  project: { number: 12, owner: { login: "hgahub" } },
                  fieldValues: { nodes: [{ field: { name: "Status" }, name: "Spec Needed" }] },
                },
              ],
            },
          },
        },
      },
    }),
  });
  const result = await readDuumbiProjectStatus({
    fetchImpl,
    token: "pat",
    owner: "hgahub",
    repo: "duumbi",
    issueNumber: 779,
    projectNumber: 12,
    projectOwner: "hgahub",
  });
  assert.equal(result.status, "Spec Needed");
  assert.equal(result.error, null);
});

function mockCore() {
  return {
    warning() {},
    info() {},
    setFailed() {},
    summary: {
      addHeading() { return this; },
      addTable() { return this; },
      write: async () => {},
    },
  };
}

test("same combined-spec merge occurrence is posted once", async () => {
  const slackPosts = [];
  const comments = [];
  const github = {
    paginate: async (fn, args) => fn(args),
    rest: {
      issues: {
        get: async () => ({
          data: {
            number: 779,
            title: "stuck combined spec",
            html_url: "https://github.com/hgahub/duumbi/issues/779",
            state: "open",
            labels: [],
          },
        }),
        listComments: async () => comments,
        createComment: async ({ body }) => {
          const comment = { body, html_url: "https://example/comment" };
          comments.push(comment);
          return { data: comment };
        },
      },
      pulls: {
        listFiles: async () => ([
          { filename: "specs/DUUMBI-779/PRODUCT.md" },
          { filename: "specs/DUUMBI-779/TECHNICAL.md" },
        ]),
      },
    },
  };
  const statusFetch = async (url, options) => {
    if (String(url).includes("graphql")) {
      return {
        json: async () => ({
          data: {
            repository: {
              issue: {
                projectItems: {
                  nodes: [{
                    project: { number: 12, owner: { login: "hgahub" } },
                    fieldValues: { nodes: [{ field: { name: "Status" }, name: "Spec Needed" }] },
                  }],
                },
              },
            },
          },
        }),
      };
    }
    if (String(url).includes("chat.postMessage")) {
      slackPosts.push(JSON.parse(options.body));
      return { ok: true, json: async () => ({ ok: true, ts: "1.1" }) };
    }
    throw new Error(url);
  };

  const context = {
    eventName: "pull_request",
    actor: "bot",
    repo: { owner: "hgahub", repo: "duumbi" },
    payload: {
      pull_request: { number: 787, merged: true, merge_commit_sha: "abc123" },
      repository: { owner: { type: "Organization" } },
    },
    workflow: "Ready For Build Handoff",
    runId: 9,
    ref: "refs/heads/main",
    sha: "def",
  };
  const env = {
    GH_PROJECT_PAT: "pat",
    DUUMBI_PROJECT_NUMBER: "12",
    DUUMBI_PROJECT_OWNER: "hgahub",
    SLACK_BOT_TOKEN: "xoxb",
    SLACK_REVIEW_CHANNEL_ID: "C-REVIEW",
    DUUMBI_PROJECT_STATUS_SLACK_BUTTONS: "true",
  };

  const first = await runReadyForBuildHandoff({
    github,
    context,
    core: mockCore(),
    env,
    fetchImpl: statusFetch,
  });
  const second = await runReadyForBuildHandoff({
    github,
    context,
    core: mockCore(),
    env,
    fetchImpl: statusFetch,
  });

  assert.equal(first.results.some((result) => result.outcome === "posted_correction"), true);
  assert.equal(second.results.some((result) => result.outcome === "duplicate_occurrence"), true);
  assert.equal(slackPosts.length, 1);
  assert.equal(slackPosts[0].blocks.some((block) => block.type === "actions"), true);
  assert.match(JSON.stringify(slackPosts[0].blocks), /project_status/);
  assert.equal(comments.some((comment) => comment.body.includes(readyForBuildMarker(779))), false);
  assert.equal(comments.some((comment) => comment.body.includes("kind=combined-spec;occurrence=abc123")), true);
});

test("reopen posts a new correction even when the Ready-for-Build v1 marker exists", async () => {
  const slackPosts = [];
  const comments = [{
    body: `${readyForBuildMarker(779)}\nReady for Build Slack handoff posted.`,
  }];
  const github = {
    paginate: async (fn, args) => fn(args),
    rest: {
      issues: {
        get: async () => ({
          data: {
            number: 779,
            title: "reopened after Done",
            html_url: "https://github.com/hgahub/duumbi/issues/779",
            state: "open",
            updated_at: "2026-09-08T10:00:00Z",
            labels: [{ name: "tech-spec-approved" }],
          },
        }),
        listComments: async () => comments,
        createComment: async ({ body }) => {
          const comment = { body, html_url: "https://example/comment2" };
          comments.push(comment);
          return { data: comment };
        },
      },
    },
  };
  const fetchImpl = async (url, options) => {
    if (String(url).includes("graphql")) {
      return {
        json: async () => ({
          data: {
            repository: {
              issue: {
                projectItems: {
                  nodes: [{
                    project: { number: 12, owner: { login: "hgahub" } },
                    fieldValues: { nodes: [{ field: { name: "Status" }, name: "Done" }] },
                  }],
                },
              },
            },
          },
        }),
      };
    }
    if (String(url).includes("chat.postMessage")) {
      slackPosts.push(JSON.parse(options.body));
      return { ok: true, json: async () => ({ ok: true, ts: "2.2" }) };
    }
    throw new Error(url);
  };

  const env = {
    GH_PROJECT_PAT: "pat",
    DUUMBI_PROJECT_NUMBER: "12",
    DUUMBI_PROJECT_OWNER: "hgahub",
    SLACK_BOT_TOKEN: "xoxb",
    SLACK_REVIEW_CHANNEL_ID: "C-REVIEW",
  };
  const context = {
    eventName: "issues",
    actor: "hgahub",
    repo: { owner: "hgahub", repo: "duumbi" },
    payload: {
      action: "reopened",
      delivery: "111222333",
      issue: {
        number: 779,
        updated_at: "2026-09-08T10:00:00Z",
      },
      repository: { owner: { type: "User" } },
    },
    workflow: "Ready For Build Handoff",
    runId: 11,
    ref: "refs/heads/main",
    sha: "abc",
  };

  const first = await runReadyForBuildHandoff({
    github, context, core: mockCore(), env, fetchImpl,
  });
  const second = await runReadyForBuildHandoff({
    github, context, core: mockCore(), env, fetchImpl,
  });
  const later = await runReadyForBuildHandoff({
    github,
    context: {
      ...context,
      payload: { ...context.payload, delivery: "444555666", issue: { number: 779, updated_at: "2026-09-08T11:00:00Z" } },
    },
    core: mockCore(),
    env,
    fetchImpl,
  });

  assert.equal(first.results.some((result) => result.outcome === "posted_correction"), true);
  assert.equal(second.results.some((result) => result.outcome === "duplicate_occurrence"), true);
  assert.equal(later.results.some((result) => result.outcome === "posted_correction"), true);
  assert.equal(slackPosts.length, 2);
  assert.equal("blocks" in slackPosts[0], false);
  assert.equal(first.buttonsEnabled, false);
});

test("hourly schedule does not scan Spec Needed issues for correction cards", async () => {
  let listed = false;
  const github = {
    paginate: async () => {
      listed = true;
      return [];
    },
    rest: {
      issues: {
        listForRepo: async () => [],
        listComments: async () => [],
      },
    },
  };
  const result = await runReadyForBuildHandoff({
    github,
    context: {
      eventName: "schedule",
      actor: "bot",
      repo: { owner: "hgahub", repo: "duumbi" },
      payload: { repository: { owner: { type: "User" } } },
      workflow: "Ready For Build Handoff",
      runId: 1,
      ref: "refs/heads/main",
      sha: "abc",
    },
    core: mockCore(),
    env: {
      GH_PROJECT_PAT: "",
      DUUMBI_PROJECT_NUMBER: "12",
      SLACK_BOT_TOKEN: "xoxb",
      SLACK_REVIEW_CHANNEL_ID: "C-REVIEW",
    },
    fetchImpl: async () => ({ json: async () => ({ data: {} }) }),
  });
  assert.equal(listed, true);
  assert.equal(result.results.some((entry) => String(entry.outcome || "").includes("correction")), false);
});
