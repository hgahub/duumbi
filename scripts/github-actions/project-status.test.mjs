import assert from "node:assert/strict";
import test from "node:test";
import {
  ALLOWED_DECISIONS,
  buildClosedIssueSlackText,
  buildFailureSlackText,
  buildStatusComment,
  buildSuccessSlackText,
  resolveProjectStatusInputs,
  resolveSlackDestination,
  runProjectStatusJob,
  selectTargetProjectItem,
  updateDuumbiReadyForBuild,
} from "./project-status.mjs";

const DUUMBI_PROJECT = {
  id: "PVT_duumbi",
  number: 12,
  title: "DUUMBI",
  owner: { login: "hgahub" },
  fields: {
    nodes: [{
      id: "FIELD_status",
      name: "Status",
      options: [
        { id: "OPT_ready", name: "Ready for Build" },
        { id: "OPT_done", name: "Done" },
        { id: "OPT_spec", name: "Spec Needed" },
      ],
    }],
  },
};

const OTHER_PROJECT = {
  id: "PVT_other",
  number: 99,
  title: "Other Board",
  owner: { login: "hgahub" },
};

function item(project, status, itemId) {
  return {
    id: itemId,
    project,
    fieldValues: {
      nodes: [{ field: { name: "Status" }, name: status }],
    },
  };
}

function mockGraphql({ project = DUUMBI_PROJECT, items, mutate }) {
  const mutations = [];
  const gql = async (query, variables) => {
    if (query.includes("updateProjectV2ItemFieldValue")) {
      mutations.push(variables);
      if (mutate) return mutate(variables);
      return { updateProjectV2ItemFieldValue: { projectV2Item: { id: variables.iid } } };
    }
    if (query.includes("projectV2(number:$number)")) {
      const rootField = query.includes("organization(") ? "organization" : "user";
      return { [rootField]: { projectV2: project } };
    }
    if (query.includes("projectItems")) {
      return {
        repository: {
          issue: { projectItems: { nodes: items } },
        },
      };
    }
    throw new Error(`unexpected query: ${query.slice(0, 80)}`);
  };
  gql.mutations = mutations;
  return gql;
}

function mockGithub({ issue, comments = [] }) {
  const created = [];
  return {
    rest: {
      issues: {
        get: async () => ({ data: issue }),
        createComment: async ({ body }) => {
          const comment = { html_url: "https://github.com/hgahub/duumbi/issues/789#issuecomment-1", body };
          created.push(comment);
          comments.push(comment);
          return { data: comment };
        },
      },
    },
    createdComments: created,
  };
}

function mockCore() {
  const failures = [];
  return {
    setFailed: (message) => { failures.push(message); },
    warning: () => {},
    info: () => {},
    failures,
  };
}

test("selectTargetProjectItem keeps only the configured DUUMBI Project", () => {
  const nodes = [
    item(OTHER_PROJECT, "Done", "ITEM_other"),
    item(DUUMBI_PROJECT, "Spec Needed", "ITEM_duumbi"),
  ];
  const selected = selectTargetProjectItem(nodes, { projectNumber: 12, projectOwner: "hgahub" });
  assert.equal(selected.id, "ITEM_duumbi");
  assert.equal(selectTargetProjectItem(nodes, { projectNumber: 12, projectOwner: "someone-else" }), null);
});

test("updateDuumbiReadyForBuild mutates only the DUUMBI item", async () => {
  const gql = mockGraphql({
    items: [
      item(OTHER_PROJECT, "Done", "ITEM_other"),
      item(DUUMBI_PROJECT, "Done", "ITEM_duumbi"),
    ],
  });
  const result = await updateDuumbiReadyForBuild({
    gql,
    owner: "hgahub",
    repo: "duumbi",
    issueNumber: 779,
    projectNumber: 12,
    projectOwner: "hgahub",
    projectOwnerType: "organization",
  });
  assert.equal(result.mutated, true);
  assert.equal(result.previousStatus, "Done");
  assert.equal(gql.mutations.length, 1);
  assert.equal(gql.mutations[0].pid, "PVT_duumbi");
  assert.equal(gql.mutations[0].iid, "ITEM_duumbi");
  assert.equal(gql.mutations[0].oid, "OPT_ready");
});

test("missing target Project or item fails without mutating other boards", async () => {
  const gql = mockGraphql({
    project: null,
    items: [item(OTHER_PROJECT, "Done", "ITEM_other")],
  });
  await assert.rejects(
    () => updateDuumbiReadyForBuild({
      gql,
      owner: "hgahub",
      repo: "duumbi",
      issueNumber: 779,
      projectNumber: 12,
      projectOwner: "hgahub",
      projectOwnerType: "user",
    }),
    /not found/i,
  );
  assert.equal(gql.mutations.length, 0);

  const gqlItems = mockGraphql({
    items: [item(OTHER_PROJECT, "Done", "ITEM_other")],
  });
  await assert.rejects(
    () => updateDuumbiReadyForBuild({
      gql: gqlItems,
      owner: "hgahub",
      repo: "duumbi",
      issueNumber: 779,
      projectNumber: 12,
      projectOwner: "hgahub",
      projectOwnerType: "user",
    }),
    /not on the configured DUUMBI Project/,
  );
  assert.equal(gqlItems.mutations.length, 0);
});

test("already Ready for Build is idempotent and does not fail", async () => {
  const gql = mockGraphql({
    items: [item(DUUMBI_PROJECT, "Ready for Build", "ITEM_duumbi")],
  });
  const result = await updateDuumbiReadyForBuild({
    gql,
    owner: "hgahub",
    repo: "duumbi",
    issueNumber: 789,
    projectNumber: 12,
    projectOwner: "hgahub",
    projectOwnerType: "user",
  });
  assert.equal(result.idempotent, true);
  assert.equal(result.mutated, false);
  assert.equal(gql.mutations.length, 0);
});

test("workflow_dispatch without thread metadata is channel-only and uses github.actor", () => {
  const inputs = resolveProjectStatusInputs({
    eventName: "workflow_dispatch",
    payload: { inputs: { issue_number: "789", decision: "ready-for-build" } },
    actor: "hgahub",
  });
  assert.equal(inputs.reviewer, "hgahub");
  assert.equal(inputs.channelId, undefined);
  assert.equal(inputs.threadTs, undefined);
  assert.equal(ALLOWED_DECISIONS.includes(inputs.decision), true);

  const destination = resolveSlackDestination({
    channelId: inputs.channelId,
    threadTs: inputs.threadTs,
    reviewChannelId: "C-REVIEW",
  });
  assert.equal(destination.mode, "channel-only");
  assert.equal(destination.channel, "C-REVIEW");
  assert.equal(destination.thread_ts, undefined);

  const comment = buildStatusComment({
    decision: "ready-for-build",
    reviewer: inputs.reviewer,
    previousStatus: "Done",
    projectNumber: 12,
    slackInThread: false,
    idempotent: false,
  });
  assert.match(comment, /not in-thread/);
  assert.match(comment, /Spec PR merged:\*\* no/);
  assert.doesNotMatch(comment, /response_url/);
});

test("repository_dispatch with channel_id and thread_ts is in-thread", () => {
  const inputs = resolveProjectStatusInputs({
    eventName: "repository_dispatch",
    payload: {
      client_payload: {
        action_type: "project_status",
        issue_number: 789,
        decision: "undo-done",
        reviewer: "Slack (hga)",
        channel_id: "C123",
        thread_ts: "123.456",
        response_url: "https://hooks.slack.com/actions/nope",
      },
    },
    actor: "github-actions[bot]",
  });
  assert.equal(inputs.reviewer, "Slack (hga)");
  assert.equal(inputs.channelId, "C123");
  assert.equal(inputs.threadTs, "123.456");
  assert.equal("response_url" in inputs, false);

  const destination = resolveSlackDestination({
    channelId: inputs.channelId,
    threadTs: inputs.threadTs,
    reviewChannelId: "C-REVIEW",
  });
  assert.equal(destination.mode, "in-thread");
  assert.equal(destination.thread_ts, "123.456");
});

test("closed issue fails closed with in-thread Slack and no GraphQL mutation", async () => {
  const fetchCalls = [];
  const fetchImpl = async (url, options) => {
    fetchCalls.push({ url, options });
    return { ok: true, json: async () => ({ ok: true, ts: "9.9" }) };
  };
  const github = mockGithub({ issue: { state: "closed", number: 779 } });
  const core = mockCore();
  const recorded = await runProjectStatusJob({
    github,
    core,
    fetchImpl,
    env: {
      GH_PROJECT_PAT: "pat",
      DUUMBI_PROJECT_NUMBER: "12",
      DUUMBI_PROJECT_OWNER: "hgahub",
      SLACK_BOT_TOKEN: "xoxb",
      SLACK_REVIEW_CHANNEL_ID: "C-REVIEW",
    },
    context: {
      eventName: "repository_dispatch",
      actor: "bot",
      repo: { owner: "hgahub", repo: "duumbi" },
      payload: {
        client_payload: {
          action_type: "project_status",
          issue_number: 779,
          decision: "undo-done",
          channel_id: "C123",
          thread_ts: "1.2",
        },
      },
      workflow: "Project Status",
      runId: 1,
      ref: "refs/heads/main",
      sha: "abc",
    },
  });

  assert.equal(recorded.outcome, "failed");
  assert.match(core.failures[0], /closed/);
  assert.equal(recorded.mutations.length, 0);
  assert.equal(fetchCalls.length, 1);
  assert.equal(fetchCalls[0].url, "https://slack.com/api/chat.postMessage");
  const slackBody = JSON.parse(fetchCalls[0].options.body);
  assert.equal(slackBody.channel, "C123");
  assert.equal(slackBody.thread_ts, "1.2");
  assert.match(slackBody.text, /closed/);
  assert.doesNotMatch(slackBody.text, /Approve/);
  assert.equal(fetchCalls.some((call) => String(call.url).includes("graphql")), false);
});

test("manual dispatch without thread metadata posts channel-only and records github.actor", async () => {
  const fetchCalls = [];
  const fetchImpl = async (url, options) => {
    fetchCalls.push({ url, options });
    if (String(url).includes("graphql")) {
      const body = JSON.parse(options.body);
      if (body.query.includes("projectV2(number:$number)")) {
        return { ok: true, json: async () => ({ data: { user: { projectV2: DUUMBI_PROJECT } } }) };
      }
      if (body.query.includes("projectItems")) {
        return {
          ok: true,
          json: async () => ({
            data: {
              repository: {
                issue: {
                  projectItems: {
                    nodes: [item(DUUMBI_PROJECT, "Spec Needed", "ITEM_duumbi")],
                  },
                },
              },
            },
          }),
        };
      }
      if (body.query.includes("updateProjectV2ItemFieldValue")) {
        return { ok: true, json: async () => ({ data: { updateProjectV2ItemFieldValue: { projectV2Item: { id: "ITEM_duumbi" } } } }) };
      }
    }
    if (String(url).includes("chat.postMessage")) {
      return { ok: true, json: async () => ({ ok: true, ts: "9.9" }) };
    }
    throw new Error(url);
  };

  const github = mockGithub({ issue: { state: "open", number: 789 } });
  const core = mockCore();
  const recorded = await runProjectStatusJob({
    github,
    core,
    fetchImpl,
    env: {
      GH_PROJECT_PAT: "pat",
      DUUMBI_PROJECT_NUMBER: "12",
      DUUMBI_PROJECT_OWNER: "hgahub",
      DUUMBI_PROJECT_OWNER_TYPE: "user",
      SLACK_BOT_TOKEN: "xoxb",
      SLACK_REVIEW_CHANNEL_ID: "C-REVIEW",
    },
    context: {
      eventName: "workflow_dispatch",
      actor: "hgahub",
      repo: { owner: "hgahub", repo: "duumbi" },
      payload: {
        inputs: { issue_number: "789", decision: "ready-for-build" },
        repository: { owner: { type: "User" } },
      },
      workflow: "Project Status",
      runId: 42,
      ref: "refs/heads/main",
      sha: "abc",
    },
  });

  assert.equal(recorded.outcome, "success");
  assert.equal(core.failures.length, 0);
  assert.equal(github.createdComments[0].body.includes("Slack ("), false);
  assert.match(github.createdComments[0].body, /Reviewer source:\*\* hgahub/);
  assert.match(github.createdComments[0].body, /not in-thread/);
  const slack = fetchCalls.find((call) => String(call.url).includes("chat.postMessage"));
  const slackBody = JSON.parse(slack.options.body);
  assert.equal(slackBody.channel, "C-REVIEW");
  assert.equal("thread_ts" in slackBody, false);
});

test("closed-issue and failure copy never instruct Stage 7/9 Approve", () => {
  const closed = buildClosedIssueSlackText({ issueNumber: 779, owner: "hgahub", repo: "duumbi" });
  const failed = buildFailureSlackText({
    issueNumber: 779,
    owner: "hgahub",
    repo: "duumbi",
    reason: "GraphQL update failed",
  });
  for (const text of [closed, failed, buildSuccessSlackText({ issueNumber: 1, decision: "undo-done", previousStatus: "Done" })]) {
    assert.doesNotMatch(text, /Approve/);
    assert.doesNotMatch(text, /stage-approval/i);
  }
  assert.match(failed, /project-status\.yml/);
  assert.match(failed, /Project UI/);
});
