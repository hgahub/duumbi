import {
  inferProjectOwnerType,
  selectTargetProjectItem,
  statusNameFromItem,
} from "./project-status.mjs";

export const ENABLEMENT_VAR = "DUUMBI_PROJECT_STATUS_SLACK_BUTTONS";
export const READY_FOR_BUILD_MARKER_PREFIX = "<!-- duumbi-ready-for-build-slack-notified:v1";
export const CORRECTION_MARKER_PREFIX = "<!-- duumbi-project-status-correction-slack-notified:v1";
export const FALLBACK_WORKFLOW_NAME = "project-status.yml";

const SPEC_FILE_RE = /^specs\/DUUMBI-(\d+)\/(PRODUCT|TECHNICAL)\.md$/;

export function projectStatusButtonsEnabled(env = {}) {
  return String(env[ENABLEMENT_VAR] || "").trim().toLowerCase() === "true";
}

export function readyForBuildMarker(issueNumber) {
  return `<!-- duumbi-ready-for-build-slack-notified:v1 issue=${issueNumber} -->`;
}

export function correctionMarker({ issueNumber, kind, occurrence }) {
  return `<!-- duumbi-project-status-correction-slack-notified:v1 issue=${issueNumber};kind=${kind};occurrence=${occurrence} -->`;
}

export function commentsHaveMarker(comments, marker) {
  return (comments || []).some((comment) => String(comment.body || "").includes(marker));
}

export function reopenOccurrenceId({ delivery, updatedAt }) {
  return String(delivery || "").trim() || String(updatedAt || "").trim();
}

export function combinedSpecOccurrenceId(mergeSha) {
  return String(mergeSha || "").trim();
}

export function dispatchOccurrenceId(runId) {
  return String(runId || "").trim();
}

export function combinedSpecIssueNumberFromFiles(files) {
  const paths = (files || []).map((file) => file.filename || file).filter(Boolean);
  if (paths.length !== 2) return null;
  const parsed = paths.map((path) => path.match(SPEC_FILE_RE)).filter(Boolean);
  if (parsed.length !== 2) return null;
  const issues = new Set(parsed.map((match) => match[1]));
  const kinds = new Set(parsed.map((match) => match[2]));
  if (issues.size !== 1 || !kinds.has("PRODUCT") || !kinds.has("TECHNICAL")) return null;
  return Number([...issues][0]);
}

export function hasCombinedStage79AcceptComments(comments) {
  const bodies = (comments || []).map((comment) => comment.body || "");
  const approved = (heading) => bodies.some((body) => heading.test(body) && /\*\*Decision:\*\*\s*Approve/i.test(body));
  return approved(/Stage 7 Product Spec Review Decision/i)
    && approved(/Stage 9 Technical Spec Review Decision/i);
}

export function isCorrectionStatus(status, statusError) {
  return Boolean(statusError) || status === "Spec Needed" || status === "Done";
}

export function shouldIncludeUndoDone(status, statusError) {
  return Boolean(statusError) || status === "Done" || status == null;
}

export function projectStatusButtonValue(issueNumber, decision) {
  return JSON.stringify({
    action_type: "project_status",
    issue_number: issueNumber,
    decision,
  });
}

export function projectStatusWorkflowUrl(owner, repo) {
  return `https://github.com/${owner}/${repo}/actions/workflows/${FALLBACK_WORKFLOW_NAME}`;
}

function slackEscape(value) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

export function buildProjectStatusBlocks({ issue, owner, repo, includeUndoDone }) {
  const workflowUrl = projectStatusWorkflowUrl(owner, repo);
  const buttonValue = (decision) => projectStatusButtonValue(issue.number, decision);
  const elements = [
    {
      type: "button",
      text: { type: "plain_text", text: "Set Ready for Build", emoji: true },
      style: "primary",
      action_id: "project_status_ready_for_build",
      value: buttonValue("ready-for-build"),
      confirm: {
        title: { type: "plain_text", text: "Set Ready for Build" },
        text: {
          type: "mrkdwn",
          text: `Set Project Status for issue #${issue.number} to *Ready for Build*? This does not merge a spec PR.`,
        },
        confirm: { type: "plain_text", text: "Set Ready for Build" },
        deny: { type: "plain_text", text: "Cancel" },
      },
    },
  ];
  if (includeUndoDone) {
    elements.push({
      type: "button",
      text: { type: "plain_text", text: "Undo Done", emoji: true },
      action_id: "project_status_undo_done",
      value: buttonValue("undo-done"),
      confirm: {
        title: { type: "plain_text", text: "Undo Done" },
        text: {
          type: "mrkdwn",
          text: `Move open issue #${issue.number} off Done to *Ready for Build*? This will not reopen a closed issue and will not merge a spec PR.`,
        },
        confirm: { type: "plain_text", text: "Undo Done" },
        deny: { type: "plain_text", text: "Cancel" },
      },
    });
  }
  return [
    {
      type: "actions",
      block_id: `project_status_${issue.number}`,
      elements,
    },
    {
      type: "context",
      elements: [{
        type: "mrkdwn",
        text: `Buttons are handled by the Slack approval bridge. Fallback: run <${workflowUrl}|Project Status> workflow manually (decision=ready-for-build, issue=${issue.number}), or set Status in the GitHub Project UI.`,
      }],
    },
  ];
}

export function buildSlackPostPayload({ channel, text, blocksEnabled, blocks }) {
  const payload = { channel, text };
  if (blocksEnabled && blocks) payload.blocks = blocks;
  return payload;
}

export function planCorrectionPost({ comments, marker }) {
  if (commentsHaveMarker(comments, marker)) {
    return { post: false, reason: "duplicate_occurrence" };
  }
  return { post: true };
}

export function correctionKindForEvent({
  eventName,
  eventAction,
  mergedPullRequest,
  combinedSpecIssueNumber,
  dispatchIssueNumber,
  hasCombinedSpecs,
}) {
  if (eventName === "schedule") return null;
  if (eventName === "issues" && eventAction === "reopened") return "reopened";
  if (eventName === "pull_request" && mergedPullRequest && combinedSpecIssueNumber) return "combined-spec";
  if (eventName === "workflow_dispatch" && dispatchIssueNumber && hasCombinedSpecs) return "dispatch";
  return null;
}

export async function readDuumbiProjectStatus({
  fetchImpl,
  token,
  owner,
  repo,
  issueNumber,
  projectNumber,
  projectOwner,
}) {
  if (!token || !projectNumber) {
    return { status: null, error: "not_configured" };
  }
  const query = `
    query($owner:String!,$repo:String!,$number:Int!) {
      repository(owner:$owner,name:$repo) {
        issue(number:$number) {
          projectItems(first:20) {
            nodes {
              fieldValues(first:20) {
                nodes {
                  ... on ProjectV2ItemFieldSingleSelectValue {
                    name
                    field { ... on ProjectV2SingleSelectField { name } }
                  }
                }
              }
              project {
                number
                title
                owner {
                  ... on Organization { login }
                  ... on User { login }
                }
              }
            }
          }
        }
      }
    }`;
  const response = await fetchImpl("https://api.github.com/graphql", {
    method: "POST",
    headers: {
      Authorization: `bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query, variables: { owner, repo, number: Number(issueNumber) } }),
  });
  const body = await response.json();
  if (body.errors) {
    return { status: null, error: "query_failed", detail: body.errors.map((err) => err.message).join("; ") };
  }
  const nodes = body.data?.repository?.issue?.projectItems?.nodes || [];
  const item = selectTargetProjectItem(nodes, { projectNumber, projectOwner });
  if (!item) return { status: null, error: "item_missing" };
  return { status: statusNameFromItem(item), error: null };
}

function labelsFor(issue) {
  return (issue.labels || [])
    .map((label) => typeof label === "string" ? label : label.name)
    .filter(Boolean);
}

function extractGithubUrls(text) {
  return [...new Set((String(text || "").match(/https:\/\/github\.com\/[^\s)>"]+/g) || [])
    .map((url) => url.replace(/[.,;:]+$/, "")))];
}

function extractLabeledUrl(text, labelPattern) {
  const lines = String(text || "").split("\n");
  const line = lines.find((candidate) => {
    const match = candidate.match(/^\*\*([^:*]+):\*\*\s*(.+)$/);
    return match && labelPattern.test(match[1]);
  });
  return extractGithubUrls(line || "")[0] || "";
}

function extractNextPrompt(text, issue) {
  const match = String(text || "").match(/## Next Codex Prompt\s*```text\n([\s\S]*?)```/i);
  if (match?.[1]?.trim()) return match[1].trim();
  return [
    "Run DUUMBI Stage 10 Implementation Coordination with duumbi-implementation.",
    "",
    `Target issue: ${issue.html_url}`,
    "Mode: coordinate Stage 10 branch, PR, blocker, or evidence state",
    "",
    "Goal: Verify Ready for Build context, manage branch and PR readiness, consolidate Ralph-cycle evidence when relevant, choose the next routed action, and delegate implementation edits only through bounded Ralph cycles.",
    "",
    "Do not exceed the approved product or technical specs, skip resource gates, perform Stage 12 closure, merge PRs, or mark the issue done.",
  ].join("\n");
}

async function postSlack({ fetchImpl, token, payload }) {
  const response = await fetchImpl("https://slack.com/api/chat.postMessage", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json; charset=utf-8",
    },
    body: JSON.stringify(payload),
  });
  const body = await response.json();
  if (!response.ok || !body.ok) {
    throw new Error(`Slack chat.postMessage failed: ${body.error || response.status}`);
  }
  return body;
}

export async function runReadyForBuildHandoff(deps) {
  const {
    github,
    context,
    core,
    env = process.env,
    fetchImpl = globalThis.fetch,
    fs,
  } = deps;
  const { owner, repo } = context.repo;
  const eventName = context.eventName;
  const slackToken = env.SLACK_BOT_TOKEN;
  const slackChannel = env.SLACK_REVIEW_CHANNEL_ID;
  const warnings = [];
  const buttonsEnabled = projectStatusButtonsEnabled(env);
  const projectNumber = Number(env.DUUMBI_PROJECT_NUMBER || 0);
  const projectOwner = env.DUUMBI_PROJECT_OWNER || owner;
  const ownerType = inferProjectOwnerType(
    env.DUUMBI_PROJECT_OWNER_TYPE,
    context.payload?.repository?.owner?.type,
  );

  async function getIssue(issueNumber) {
    const { data } = await github.rest.issues.get({
      owner,
      repo,
      issue_number: Number(issueNumber),
    });
    return data;
  }

  async function loadComments(issueNumber) {
    return github.paginate(github.rest.issues.listComments, {
      owner,
      repo,
      issue_number: issueNumber,
      per_page: 100,
    });
  }

  async function listReadyProjectIssues() {
    const projectPat = env.GH_PROJECT_PAT;
    if (!projectPat || !projectNumber) {
      warnings.push("project_ready_scan_not_configured");
      return [];
    }
    const rootField = ownerType === "organization" ? "organization" : "user";
    const query = `
      query($owner:String!,$number:Int!,$cursor:String) {
        ${rootField}(login:$owner) {
          projectV2(number:$number) {
            items(first:100, after:$cursor) {
              nodes {
                content {
                  ... on Issue {
                    number
                    title
                    url
                    state
                    repository { nameWithOwner }
                    labels(first:30) { nodes { name } }
                  }
                }
                fieldValues(first:20) {
                  nodes {
                    ... on ProjectV2ItemFieldSingleSelectValue {
                      name
                      field { ... on ProjectV2SingleSelectField { name } }
                    }
                  }
                }
              }
              pageInfo { hasNextPage endCursor }
            }
          }
        }
      }`;
    const ready = [];
    let cursor = null;
    do {
      const response = await fetchImpl("https://api.github.com/graphql", {
        method: "POST",
        headers: {
          Authorization: `bearer ${projectPat}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ query, variables: { owner: projectOwner, number: projectNumber, cursor } }),
      });
      const body = await response.json();
      if (body.errors) {
        warnings.push("project_ready_scan_failed");
        core.warning(`Project Ready for Build scan failed: ${body.errors.map((err) => err.message).join("; ")}`);
        return ready;
      }
      const page = body.data?.[rootField]?.projectV2?.items;
      for (const projectItem of page?.nodes || []) {
        const issue = projectItem.content;
        if (!issue || issue.repository?.nameWithOwner !== `${owner}/${repo}` || issue.state !== "OPEN") continue;
        const status = (projectItem.fieldValues?.nodes || []).find((value) =>
          value?.field?.name === "Status" && value?.name === "Ready for Build",
        );
        if (!status) continue;
        ready.push({
          number: issue.number,
          title: issue.title,
          html_url: issue.url,
          labels: (issue.labels?.nodes || []).map((label) => ({ name: label.name })),
        });
      }
      cursor = page?.pageInfo?.hasNextPage ? page.pageInfo.endCursor : null;
    } while (cursor);
    return ready;
  }

  async function collectReadyCandidates() {
    const candidates = new Map();
    if (eventName === "issues" && context.payload.action !== "reopened") {
      const issue = context.payload.issue;
      if (context.payload.label?.name === "tech-spec-approved") candidates.set(issue.number, issue);
      return [...candidates.values()];
    }
    if (eventName === "issues" || eventName === "pull_request") {
      return [];
    }

    const inputIssue = String(context.payload.inputs?.issue_number || "").trim();
    if (eventName === "workflow_dispatch" && inputIssue) {
      candidates.set(Number(inputIssue), await getIssue(inputIssue));
      return [...candidates.values()];
    }

    const labeledIssues = await github.paginate(github.rest.issues.listForRepo, {
      owner,
      repo,
      state: "open",
      labels: "tech-spec-approved",
      per_page: 100,
    });
    for (const issue of labeledIssues.filter((item) => !item.pull_request)) {
      candidates.set(issue.number, issue);
    }
    for (const issue of await listReadyProjectIssues()) {
      candidates.set(issue.number, issue);
    }
    return [...candidates.values()];
  }

  async function collectCorrectionCandidate() {
    const kind = (() => {
      if (eventName === "schedule") return null;
      if (eventName === "issues" && context.payload.action === "reopened") return "reopened";
      if (eventName === "pull_request") return "combined-spec";
      if (eventName === "workflow_dispatch") return "dispatch";
      return null;
    })();
    if (!kind) return null;

    if (kind === "reopened") {
      const issue = context.payload.issue;
      if (!issue || issue.pull_request) return null;
      const occurrence = reopenOccurrenceId({
        delivery: env.GITHUB_DELIVERY || context.payload.delivery,
        updatedAt: issue.updated_at,
      });
      return { issue: await getIssue(issue.number), kind, occurrence };
    }

    if (kind === "combined-spec") {
      const pull = context.payload.pull_request;
      if (!pull?.merged) return null;
      const files = await github.paginate(github.rest.pulls.listFiles, {
        owner,
        repo,
        pull_number: pull.number,
        per_page: 100,
      });
      const issueNumber = combinedSpecIssueNumberFromFiles(files);
      if (!issueNumber) return null;
      const issue = await getIssue(issueNumber);
      if (issue.pull_request || issue.state !== "open") return null;
      return {
        issue,
        kind,
        occurrence: combinedSpecOccurrenceId(pull.merge_commit_sha),
      };
    }

    const inputIssue = String(context.payload.inputs?.issue_number || "").trim();
    if (!inputIssue) return null;
    const issue = await getIssue(inputIssue);
    if (issue.pull_request || issue.state !== "open") return null;
    const comments = await loadComments(issue.number);
    let hasSpecs = false;
    try {
      await github.rest.repos.getContent({
        owner,
        repo,
        path: `specs/DUUMBI-${issue.number}/PRODUCT.md`,
      });
      await github.rest.repos.getContent({
        owner,
        repo,
        path: `specs/DUUMBI-${issue.number}/TECHNICAL.md`,
      });
      hasSpecs = true;
    } catch (error) {
      if (error.status && error.status !== 404) throw error;
    }
    const hasCombinedSpecs = hasSpecs || hasCombinedStage79AcceptComments(comments);
    if (!hasCombinedSpecs) return null;
    return {
      issue,
      kind: "dispatch",
      occurrence: dispatchOccurrenceId(context.runId),
    };
  }

  async function notifyReady(issue) {
    const labels = labelsFor(issue);
    const statusResult = await readDuumbiProjectStatus({
      fetchImpl,
      token: env.GH_PROJECT_PAT,
      owner,
      repo,
      issueNumber: issue.number,
      projectNumber,
      projectOwner,
    });
    if (statusResult.error === "query_failed") {
      warnings.push(`project_status_query_failed_issue_${issue.number}`);
      core.warning(`Project status query failed for #${issue.number}: ${statusResult.detail || statusResult.error}`);
    }
    const projectStatus = statusResult.status;
    const isReady = labels.includes("tech-spec-approved") || projectStatus === "Ready for Build";
    if (!isReady) return { issue: issue.number, outcome: "not_ready" };

    const comments = await loadComments(issue.number);
    const marker = readyForBuildMarker(issue.number);
    if (commentsHaveMarker(comments, marker)) {
      return { issue: issue.number, outcome: "already_notified" };
    }

    if (!slackToken) throw new Error("SLACK_BOT_TOKEN is not configured.");
    if (!slackChannel) throw new Error("SLACK_REVIEW_CHANNEL_ID is not configured.");

    const stage9 = [...comments].reverse().find((comment) =>
      /Stage 9 (Technical Spec Review Decision|AI Gate Decision)/i.test(comment.body || ""),
    );
    const stage9Body = stage9?.body || "";
    const urls = extractGithubUrls(stage9Body);
    const technicalSpec =
      extractLabeledUrl(stage9Body, /^(Technical spec|Spec PR)$/i) ||
      urls.find((url) => /\/pull\/\d+/.test(url)) ||
      "";
    const productSpec = extractLabeledUrl(stage9Body, /^Product spec$/i) || "";
    const prompt = extractNextPrompt(stage9?.body || "", issue);
    const signal = [
      labels.includes("tech-spec-approved") ? "`tech-spec-approved` label" : null,
      projectStatus === "Ready for Build" ? "`Ready for Build` Project status" : null,
    ].filter(Boolean).join(" and ");

    const text = [
      "*DUUMBI Stage 10 handoff: Ready for Build*",
      `*Issue:* <${issue.html_url}|#${issue.number} ${slackEscape(issue.title)}>`,
      `*Signal:* ${signal || "Ready for Build"}`,
      technicalSpec ? `*Technical spec:* ${technicalSpec}` : null,
      productSpec ? `*Product spec:* ${productSpec}` : null,
      "",
      "*Next Codex prompt:*",
      "```",
      prompt,
      "```",
    ].filter((line) => line !== null).join("\n");

    const includeUndoDone = shouldIncludeUndoDone(projectStatus, statusResult.error);
    const payload = buildSlackPostPayload({
      channel: slackChannel,
      text,
      blocksEnabled: buttonsEnabled,
      blocks: [
        {
          type: "section",
          text: { type: "mrkdwn", text },
        },
        ...buildProjectStatusBlocks({ issue, owner, repo, includeUndoDone }),
      ],
    });
    const body = await postSlack({ fetchImpl, token: slackToken, payload });

    const markerBody = [
      marker,
      "Ready for Build Slack handoff posted.",
      "",
      `Slack timestamp: ${body.ts || "<unavailable>"}`,
      `Signal: ${signal || "Ready for Build"}`,
    ].join("\n");
    const { data: comment } = await github.rest.issues.createComment({
      owner,
      repo,
      issue_number: issue.number,
      body: markerBody,
    });
    return { issue: issue.number, outcome: "posted", comment_url: comment.html_url, buttons: Boolean(payload.blocks) };
  }

  async function notifyCorrection({ issue, kind, occurrence }) {
    if (issue.state !== "open" || issue.pull_request) {
      return { issue: issue.number, outcome: "skipped_closed_or_pr" };
    }
    const statusResult = await readDuumbiProjectStatus({
      fetchImpl,
      token: env.GH_PROJECT_PAT,
      owner,
      repo,
      issueNumber: issue.number,
      projectNumber,
      projectOwner,
    });
    if (!isCorrectionStatus(statusResult.status, statusResult.error)) {
      return { issue: issue.number, outcome: "status_not_stuck", status: statusResult.status };
    }

    const comments = await loadComments(issue.number);
    const marker = correctionMarker({ issueNumber: issue.number, kind, occurrence });
    const plan = planCorrectionPost({ comments, marker });
    if (!plan.post) return { issue: issue.number, outcome: plan.reason };

    if (!slackToken) throw new Error("SLACK_BOT_TOKEN is not configured.");
    if (!slackChannel) throw new Error("SLACK_REVIEW_CHANNEL_ID is not configured.");

    const includeUndoDone = shouldIncludeUndoDone(statusResult.status, statusResult.error);
    const text = [
      "*DUUMBI Project Status correction*",
      `*Issue:* <${issue.html_url}|#${issue.number} ${slackEscape(issue.title)}>`,
      `*Current status:* ${statusResult.status || "unknown"}`,
      `*Occurrence:* \`${kind}:${occurrence}\``,
      buttonsEnabled
        ? "Use the buttons to set DUUMBI Project Status to Ready for Build. This does not merge a spec PR."
        : "Project Status buttons are gated off until the Slack bridge Function routes `project_status`. Run the Project Status workflow manually or set Status in the GitHub Project UI.",
    ].join("\n");

    const payload = buildSlackPostPayload({
      channel: slackChannel,
      text,
      blocksEnabled: buttonsEnabled,
      blocks: [
        {
          type: "section",
          text: { type: "mrkdwn", text },
        },
        ...buildProjectStatusBlocks({ issue, owner, repo, includeUndoDone }),
      ],
    });
    const body = await postSlack({ fetchImpl, token: slackToken, payload });

    const markerBody = [
      marker,
      "Project Status correction/entry Slack posted.",
      "",
      `Slack timestamp: ${body.ts || "<unavailable>"}`,
      `Kind: ${kind}`,
      `Occurrence: ${occurrence}`,
    ].join("\n");
    const { data: comment } = await github.rest.issues.createComment({
      owner,
      repo,
      issue_number: issue.number,
      body: markerBody,
    });
    return {
      issue: issue.number,
      outcome: "posted_correction",
      comment_url: comment.html_url,
      buttons: Boolean(payload.blocks),
      marker,
    };
  }

  const results = [];
  for (const issue of await collectReadyCandidates()) {
    try {
      results.push(await notifyReady(issue));
    } catch (error) {
      warnings.push(`notify_failed_issue_${issue.number}`);
      core.warning(`Ready for Build handoff failed for #${issue.number}: ${error.message}`);
      results.push({ issue: issue.number, outcome: "failed", error: error.message });
    }
  }

  try {
    const correction = await collectCorrectionCandidate();
    if (correction) {
      results.push(await notifyCorrection(correction));
    }
  } catch (error) {
    warnings.push("correction_failed");
    core.warning(`Project Status correction/entry failed: ${error.message}`);
    results.push({ outcome: "failed", error: error.message });
  }

  const posted = results.filter((result) => result.outcome === "posted" || result.outcome === "posted_correction").length;
  const failed = results.filter((result) => result.outcome === "failed").length;
  const now = new Date().toISOString();
  if (fs && env.DUUMBI_METRICS_PATH) {
    fs.writeFileSync(env.DUUMBI_METRICS_PATH, `${JSON.stringify({
      schema_version: "duumbi.workflow_metrics.v1",
      generated_at: now,
      source: "github_actions",
      repository: `${owner}/${repo}`,
      workflow: {
        name: context.workflow,
        file: ".github/workflows/ready-for-build-handoff.yml",
        run_id: context.runId,
        run_attempt: Number(env.GITHUB_RUN_ATTEMPT || 1),
        event_name: eventName,
        actor: context.actor,
        ref: context.ref,
        sha: context.sha,
        conclusion: failed > 0 ? "failure" : "success",
        started_at: now,
        completed_at: now,
        duration_ms: null,
      },
      correlation: {
        issue_number: results.length === 1 ? results[0].issue : null,
        pr_number: null,
        stage: "10-handoff",
        decision: null,
        project_status: "Ready for Build",
      },
      counts: {
        issues_considered: results.length,
        issues_queued: posted,
        slack_notifications_attempted: posted + failed,
        artifact_links_found: null,
        artifact_links_missing: null,
      },
      provider_usage: {
        available: false,
        reason: "notification_only",
        provider: null,
        model: null,
        request_count: null,
        prompt_tokens: null,
        completion_tokens: null,
        total_tokens: null,
        estimated_cost_usd: null,
        latency_ms: null,
        failure_count: null,
      },
      privacy: {
        metadata_only: true,
        raw_prompts_included: false,
        raw_completions_included: false,
        raw_slack_payloads_included: false,
        secrets_included: false,
      },
      warnings,
      results,
    }, null, 2)}\n`);
  }

  await core.summary
    .addHeading("Ready for Build Handoff", 2)
    .addTable([
      [{ data: "Field", header: true }, { data: "Value", header: true }],
      ["Slack notifications posted", String(posted)],
      ["Failures", String(failed)],
      ["Buttons enabled", String(buttonsEnabled)],
      ["Results", JSON.stringify(results)],
    ])
    .write();

  if (failed > 0) {
    core.setFailed(`${failed} Ready for Build Slack handoff(s) failed.`);
  }
  return { results, buttonsEnabled, warnings };
}
