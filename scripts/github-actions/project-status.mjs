export const TARGET_STATUS = "Ready for Build";
export const ALLOWED_DECISIONS = Object.freeze(["ready-for-build", "undo-done"]);
export const ACTION_TYPE = "project_status";
export const WORKFLOW_FILE = ".github/workflows/project-status.yml";
export const FALLBACK_WORKFLOW_NAME = "project-status.yml";

function nonemptyString(value) {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

export function inferProjectOwnerType(configured, repositoryOwnerType) {
  const normalized = String(configured || "").trim().toLowerCase();
  if (normalized === "organization" || normalized === "user") return normalized;
  return repositoryOwnerType === "Organization" ? "organization" : "user";
}

export function decisionLabel(decision) {
  return decision === "undo-done" ? "Undo Done" : "Set Ready for Build";
}

export function resolveProjectStatusInputs({ eventName, payload, actor }) {
  const raw = eventName === "repository_dispatch"
    ? { ...(payload?.client_payload || {}) }
    : {
      issue_number: payload?.inputs?.issue_number,
      decision: payload?.inputs?.decision,
      rationale: payload?.inputs?.rationale,
      reviewer: payload?.inputs?.reviewer,
      action_type: ACTION_TYPE,
    };

  delete raw.slack_response_url;
  delete raw.response_url;

  const reviewer = nonemptyString(raw.reviewer) || nonemptyString(actor) || "unknown";
  return {
    actionType: nonemptyString(raw.action_type),
    issueNumber: Number(raw.issue_number),
    decision: nonemptyString(raw.decision),
    rationale: nonemptyString(raw.rationale) || `${decisionLabel(raw.decision)} by ${reviewer}`,
    reviewer,
    channelId: nonemptyString(raw.channel_id),
    threadTs: nonemptyString(raw.thread_ts),
  };
}

export function resolveSlackDestination({ channelId, threadTs, reviewChannelId }) {
  const channel = nonemptyString(channelId);
  const thread = nonemptyString(threadTs);
  if (channel && thread) {
    return { mode: "in-thread", channel, thread_ts: thread };
  }
  const fallback = nonemptyString(reviewChannelId);
  if (!fallback) {
    return { mode: "unavailable", channel: null, thread_ts: undefined };
  }
  return { mode: "channel-only", channel: fallback, thread_ts: undefined };
}

export function workflowDispatchUrl(owner, repo) {
  return `https://github.com/${owner}/${repo}/actions/workflows/${FALLBACK_WORKFLOW_NAME}`;
}

export function buildFailureSlackText({ issueNumber, owner, repo, reason }) {
  const fallback = workflowDispatchUrl(owner, repo);
  return [
    `⚠️ Project Status was *not* changed for issue #${issueNumber}.`,
    reason,
    `Fallback: run <${fallback}|Project Status> workflow manually (decision=ready-for-build, issue=${issueNumber}), or set Status in the GitHub Project UI.`,
  ].filter(Boolean).join("\n");
}

export function buildClosedIssueSlackText({ issueNumber, owner, repo }) {
  const fallback = workflowDispatchUrl(owner, repo);
  return [
    `⚠️ Issue #${issueNumber} is *closed*. Project Status was not changed and the issue was not reopened.`,
    "Reopen the GitHub issue first, then use the Project Status correction/entry message or the GitHub Project UI.",
    `Fallback: run <${fallback}|Project Status> workflow manually after the issue is open.`,
  ].join("\n");
}

export function buildSuccessSlackText({ issueNumber, decision, previousStatus, idempotent }) {
  const previous = previousStatus || "unknown";
  if (idempotent) {
    return `✅ DUUMBI Project Status for issue #${issueNumber} was already *Ready for Build* (${decisionLabel(decision)}).`;
  }
  return `✅ DUUMBI Project Status for issue #${issueNumber} is now *Ready for Build* (${decisionLabel(decision)}; previous: ${previous}).`;
}

export function buildStatusComment({
  decision,
  reviewer,
  previousStatus,
  projectNumber,
  slackInThread,
  idempotent,
}) {
  const lines = [
    "## Project Status Update",
    `**Decision:** ${decisionLabel(decision)}`,
    `**Reviewer source:** ${reviewer}`,
    `**Previous status:** ${previousStatus || "unknown"}`,
    "**Project:** Ready for Build",
    `**Target project:** DUUMBI_PROJECT_NUMBER=${projectNumber}`,
    "**Spec PR merged:** no",
  ];
  if (idempotent) {
    lines.push("**Idempotent:** Status was already Ready for Build");
  }
  if (!slackInThread) {
    lines.push("**Slack reply:** not in-thread (channel-only; no channel_id/thread_ts on this dispatch)");
  }
  return lines.join("\n");
}

export function selectTargetProjectItem(nodes, { projectNumber, projectOwner }) {
  const number = Number(projectNumber);
  const owner = String(projectOwner || "").toLowerCase();
  if (!Number.isInteger(number) || number < 1 || !owner) return null;
  return (nodes || []).find((item) => {
    const n = Number(item?.project?.number);
    const login = String(item?.project?.owner?.login || "").toLowerCase();
    return n === number && login === owner;
  }) || null;
}

export function statusNameFromItem(item) {
  for (const value of item?.fieldValues?.nodes || []) {
    if (value?.field?.name === "Status" && value?.name) return value.name;
  }
  return null;
}

export function createGraphqlClient(fetchImpl, token) {
  return async function gql(query, variables) {
    const response = await fetchImpl("https://api.github.com/graphql", {
      method: "POST",
      headers: {
        Authorization: `bearer ${token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ query, variables }),
    });
    const body = await response.json();
    if (body.errors) throw new Error(body.errors.map((err) => err.message).join("; "));
    return body.data;
  };
}

export async function updateDuumbiReadyForBuild({
  gql,
  owner,
  repo,
  issueNumber,
  projectNumber,
  projectOwner,
  projectOwnerType,
}) {
  if (!projectNumber) {
    throw new Error("DUUMBI_PROJECT_NUMBER is not configured; Status was not changed.");
  }

  const rootField = projectOwnerType === "organization" ? "organization" : "user";
  const projectData = await gql(`
    query($login:String!,$number:Int!) {
      ${rootField}(login:$login) {
        projectV2(number:$number) {
          id
          number
          title
          fields(first: 30) {
            nodes {
              ... on ProjectV2SingleSelectField { id name options { id name } }
            }
          }
        }
      }
    }`, { login: projectOwner, number: Number(projectNumber) });

  const project = projectData?.[rootField]?.projectV2;
  if (!project) {
    throw new Error(`Configured DUUMBI Project ${projectOwner}/#${projectNumber} was not found.`);
  }

  const statusField = (project.fields?.nodes || []).find((field) => field?.name === "Status");
  const option = statusField?.options?.find((entry) => entry.name === TARGET_STATUS);
  if (!statusField || !option) {
    throw new Error(`Status option "${TARGET_STATUS}" was not found on DUUMBI Project ${projectOwner}/#${projectNumber}.`);
  }

  const itemsData = await gql(`
    query($owner:String!,$repo:String!,$number:Int!) {
      repository(owner:$owner,name:$repo) {
        issue(number:$number) {
          projectItems(first: 20) {
            nodes {
              id
              project {
                id
                number
                title
                owner {
                  ... on Organization { login }
                  ... on User { login }
                }
              }
              fieldValues(first: 20) {
                nodes {
                  ... on ProjectV2ItemFieldSingleSelectValue {
                    name
                    field { ... on ProjectV2SingleSelectField { name } }
                  }
                }
              }
            }
          }
        }
      }
    }`, { owner, repo, number: Number(issueNumber) });

  const nodes = itemsData?.repository?.issue?.projectItems?.nodes || [];
  const item = selectTargetProjectItem(nodes, { projectNumber, projectOwner });
  if (!item) {
    throw new Error(`Issue #${issueNumber} is not on the configured DUUMBI Project ${projectOwner}/#${projectNumber}.`);
  }
  if (item.project.id !== project.id) {
    throw new Error("Target Project item did not match the configured DUUMBI Project id.");
  }

  const previousStatus = statusNameFromItem(item);
  if (previousStatus === TARGET_STATUS) {
    return {
      mutated: false,
      idempotent: true,
      previousStatus,
      projectId: project.id,
      itemId: item.id,
    };
  }

  await gql(`
    mutation($pid: ID!, $iid: ID!, $fid: ID!, $oid: String!) {
      updateProjectV2ItemFieldValue(input: {
        projectId: $pid, itemId: $iid, fieldId: $fid,
        value: { singleSelectOptionId: $oid }
      }) { projectV2Item { id } }
    }`, {
    pid: project.id,
    iid: item.id,
    fid: statusField.id,
    oid: option.id,
  });

  return {
    mutated: true,
    idempotent: false,
    previousStatus,
    projectId: project.id,
    itemId: item.id,
  };
}

export async function postSlackMessage({ fetchImpl, token, destination, text }) {
  if (!token || destination.mode === "unavailable" || !destination.channel) {
    return { ok: false, skipped: true };
  }
  const body = { channel: destination.channel, text };
  if (destination.thread_ts) body.thread_ts = destination.thread_ts;
  const response = await fetchImpl("https://slack.com/api/chat.postMessage", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json; charset=utf-8",
    },
    body: JSON.stringify(body),
  });
  const payload = await response.json();
  if (!response.ok || !payload.ok) {
    throw new Error(`Slack chat.postMessage failed: ${payload.error || response.status}`);
  }
  return { ok: true, ts: payload.ts, mode: destination.mode };
}

export function buildProjectStatusMetrics({
  context,
  issueNumber,
  decision,
  conclusion,
  warnings,
}) {
  const now = new Date().toISOString();
  return {
    schema_version: "duumbi.workflow_metrics.v1",
    generated_at: now,
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
      started_at: now,
      completed_at: now,
      duration_ms: null,
    },
    correlation: {
      issue_number: issueNumber || null,
      pr_number: null,
      stage: "project-status",
      decision: decision || null,
      project_status: TARGET_STATUS,
    },
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

export async function runProjectStatusJob(deps) {
  const {
    github,
    context,
    core,
    env = process.env,
    fetchImpl = globalThis.fetch,
    fs,
  } = deps;
  const { owner, repo } = context.repo;
  const warnings = [];
  const recorded = {
    mutations: [],
    slackPosts: [],
    comments: [],
    outcome: "failed",
  };

  const inputs = resolveProjectStatusInputs({
    eventName: context.eventName,
    payload: context.payload,
    actor: context.actor,
  });
  const destination = resolveSlackDestination({
    channelId: inputs.channelId,
    threadTs: inputs.threadTs,
    reviewChannelId: env.SLACK_REVIEW_CHANNEL_ID,
  });
  const slackToken = env.SLACK_BOT_TOKEN;
  const projectNumber = Number(env.DUUMBI_PROJECT_NUMBER || 0);
  const projectOwner = env.DUUMBI_PROJECT_OWNER || owner;
  const projectOwnerType = inferProjectOwnerType(
    env.DUUMBI_PROJECT_OWNER_TYPE,
    context.payload?.repository?.owner?.type,
  );

  const notify = async (text) => {
    const post = await postSlackMessage({ fetchImpl, token: slackToken, destination, text });
    recorded.slackPosts.push({ ...post, text, destination });
    return post;
  };

  const fail = async (message, slackText) => {
    recorded.outcome = "failed";
    recorded.failureMessage = message;
    if (slackText) {
      try {
        await notify(slackText);
      } catch (error) {
        warnings.push(`slack_notify_failed:${error.message}`);
        core.warning(`Slack notify failed: ${error.message}`);
      }
    }
    core.setFailed(message);
    return recorded;
  };

  try {
    recorded.inputs = inputs;
    recorded.destination = destination;

    if (inputs.actionType && inputs.actionType !== ACTION_TYPE) {
      return fail(
        `Unsupported action_type: ${inputs.actionType}`,
        buildFailureSlackText({
          issueNumber: inputs.issueNumber || "?",
          owner,
          repo,
          reason: `Unsupported action_type \`${inputs.actionType}\`.`,
        }),
      );
    }
    if (!Number.isInteger(inputs.issueNumber) || inputs.issueNumber < 1) {
      return fail(`Invalid issue_number: ${inputs.issueNumber}`);
    }
    if (!ALLOWED_DECISIONS.includes(inputs.decision)) {
      return fail(
        `Unsupported decision: ${inputs.decision}`,
        buildFailureSlackText({
          issueNumber: inputs.issueNumber,
          owner,
          repo,
          reason: `Unsupported decision \`${inputs.decision}\`.`,
        }),
      );
    }

    const { data: issue } = await github.rest.issues.get({
      owner,
      repo,
      issue_number: inputs.issueNumber,
    });
    if (issue.state !== "open") {
      return fail(
        `Issue #${inputs.issueNumber} is closed. Reopen it in GitHub first; Slack will not reopen it.`,
        buildClosedIssueSlackText({ issueNumber: inputs.issueNumber, owner, repo }),
      );
    }

    if (!env.GH_PROJECT_PAT || !projectNumber) {
      const reason = !env.GH_PROJECT_PAT
        ? "GH_PROJECT_PAT is not configured."
        : "DUUMBI_PROJECT_NUMBER is not configured.";
      const commentBody = [
        "## Project Status Update",
        `**Decision:** ${decisionLabel(inputs.decision)}`,
        `**Reviewer source:** ${inputs.reviewer}`,
        "**Project:** not changed",
        `**Target project:** DUUMBI_PROJECT_NUMBER=${env.DUUMBI_PROJECT_NUMBER || "unset"}`,
        "**Spec PR merged:** no",
        `**Error:** ${reason}`,
      ].join("\n");
      const { data: comment } = await github.rest.issues.createComment({
        owner,
        repo,
        issue_number: inputs.issueNumber,
        body: commentBody,
      });
      recorded.comments.push(comment);
      return fail(
        `${reason} Status was not changed.`,
        buildFailureSlackText({ issueNumber: inputs.issueNumber, owner, repo, reason }),
      );
    }

    const gql = createGraphqlClient(fetchImpl, env.GH_PROJECT_PAT);
    const wrappedGql = async (query, variables) => {
      if (String(query).includes("updateProjectV2ItemFieldValue")) {
        recorded.mutations.push({ query, variables });
      }
      return gql(query, variables);
    };

    let update;
    try {
      update = await updateDuumbiReadyForBuild({
        gql: wrappedGql,
        owner,
        repo,
        issueNumber: inputs.issueNumber,
        projectNumber,
        projectOwner,
        projectOwnerType,
      });
    } catch (error) {
      const commentBody = [
        "## Project Status Update",
        `**Decision:** ${decisionLabel(inputs.decision)}`,
        `**Reviewer source:** ${inputs.reviewer}`,
        "**Project:** not changed",
        `**Target project:** DUUMBI_PROJECT_NUMBER=${projectNumber}`,
        "**Spec PR merged:** no",
        `**Error:** ${error.message}`,
      ].join("\n");
      const { data: comment } = await github.rest.issues.createComment({
        owner,
        repo,
        issue_number: inputs.issueNumber,
        body: commentBody,
      });
      recorded.comments.push(comment);
      return fail(
        error.message,
        buildFailureSlackText({ issueNumber: inputs.issueNumber, owner, repo, reason: error.message }),
      );
    }

    const commentBody = buildStatusComment({
      decision: inputs.decision,
      reviewer: inputs.reviewer,
      previousStatus: update.previousStatus,
      projectNumber,
      slackInThread: destination.mode === "in-thread",
      idempotent: update.idempotent,
    });
    const { data: comment } = await github.rest.issues.createComment({
      owner,
      repo,
      issue_number: inputs.issueNumber,
      body: commentBody,
    });
    recorded.comments.push(comment);

    await notify(buildSuccessSlackText({
      issueNumber: inputs.issueNumber,
      decision: inputs.decision,
      previousStatus: update.previousStatus,
      idempotent: update.idempotent,
    }));

    recorded.outcome = "success";
    recorded.update = update;
    recorded.inputs = inputs;
    recorded.destination = destination;
    core.info(`Project Status ${inputs.decision} for #${inputs.issueNumber}: ${TARGET_STATUS}`);
    return recorded;
  } finally {
    if (fs && env.DUUMBI_METRICS_PATH) {
      const metrics = buildProjectStatusMetrics({
        context,
        issueNumber: inputs.issueNumber,
        decision: inputs.decision,
        conclusion: recorded.outcome === "success" ? "success" : "failure",
        warnings,
      });
      fs.writeFileSync(env.DUUMBI_METRICS_PATH, `${JSON.stringify(metrics, null, 2)}\n`);
    }
  }
}
