const STAGE10_MARKER_PREFIX = "<!-- duumbi-ralph-cycle-approval-slack-notified:v1";
const STAGE11_MARKER_PREFIX = "<!-- duumbi-implementation-review-slack-notified:v1";

const REQUIRED_RESOURCE_FIELDS = [
  "Issue",
  "Product spec",
  "Technical spec",
  "Current State",
  "Remaining Requirements",
  "Proposed Cycle Goal",
  "Planned Changes",
  "Planned Checks",
  "Resource Estimate",
  "Approval Trigger",
  "Stop Condition",
];

const DECISION_LABELS = {
  approve: "Approve Cycle",
  "narrow-scope": "Narrow Scope",
  "reject-defer": "Reject / Defer",
};

const NEXT_STATE_BY_DECISION = {
  approve: "In Progress",
  "narrow-scope": "Cycle Authorization",
  "reject-defer": "Deferred",
};

const SLACK_SECTION_TEXT_LIMIT = 3000;
const SLACK_FIELD_TEXT_LIMIT = 2000;

function normalizeText(value) {
  return String(value ?? "")
    .replace(/\r\n?/g, "\n")
    .replace(/\u0000/g, "")
    .replace(/[\u0001-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, "")
    .trim();
}

function truncate(value, maxLength) {
  const text = normalizeText(value);
  if (maxLength <= 0) {
    return "";
  }
  if (text.length <= maxLength) {
    return text;
  }
  return `${text.slice(0, Math.max(0, maxLength - 1)).trimEnd()}…`;
}

function canonicalFieldName(name) {
  return normalizeText(name)
    .replace(/\s+/g, " ")
    .replace(/:$/, "")
    .trim()
    .toLowerCase();
}

function displayFieldName(name) {
  const canonical = canonicalFieldName(name);
  return REQUIRED_RESOURCE_FIELDS.find((field) => canonicalFieldName(field) === canonical) || name;
}

function getCommentBody(comment) {
  if (typeof comment === "string") {
    return comment;
  }
  return comment?.body ?? "";
}

function getCommentId(comment) {
  if (typeof comment === "string") {
    return null;
  }
  return comment?.id ?? comment?.comment_id ?? null;
}

function getCommentUrl(comment) {
  if (typeof comment === "string") {
    return null;
  }
  return comment?.html_url ?? comment?.url ?? null;
}

function parseBoldFields(lines) {
  const fields = new Map();
  let activeName = null;
  let activeValue = [];

  function flush() {
    if (!activeName) {
      return;
    }
    const name = displayFieldName(activeName);
    const value = normalizeText(activeValue.join("\n"));
    if (value) {
      fields.set(name, value);
    }
    activeName = null;
    activeValue = [];
  }

  for (const line of lines) {
    const boldMatch = line.match(/^\*\*([^:*]+):\*\*\s*(.*)$/);
    const headingMatch = line.match(/^#{2,4}\s+(.+?)\s*$/);
    if (boldMatch) {
      flush();
      activeName = boldMatch[1];
      activeValue = [boldMatch[2]];
      continue;
    }
    if (headingMatch) {
      flush();
      continue;
    }
    if (activeName) {
      activeValue.push(line);
    }
  }
  flush();
  return fields;
}

function parseHeadingSections(lines, cycleHeadingIndex) {
  const fields = new Map();
  let activeName = null;
  let activeValue = [];

  function flush() {
    if (!activeName) {
      return;
    }
    const name = displayFieldName(activeName);
    const value = normalizeText(activeValue.join("\n"));
    if (value) {
      fields.set(name, value);
    }
    activeName = null;
    activeValue = [];
  }

  for (let index = cycleHeadingIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    const headingMatch = line.match(/^#{2,4}\s+(.+?)\s*$/);
    if (headingMatch) {
      flush();
      activeName = headingMatch[1];
      activeValue = [];
      continue;
    }
    if (activeName) {
      activeValue.push(line);
    }
  }
  flush();
  return fields;
}

export function sanitizePromptField(value) {
  return truncate(value, 4000);
}

export function sanitizeSlackField(value, maxLength = SLACK_FIELD_TEXT_LIMIT) {
  const escaped = normalizeText(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  return truncate(escaped, maxLength);
}

function buildSlackLabeledText(label, value, maxLength = SLACK_FIELD_TEXT_LIMIT) {
  const prefix = `*${label}*\n`;
  return `${prefix}${sanitizeSlackField(value, Math.max(0, maxLength - prefix.length))}`;
}

function buildSlackSectionText(parts, maxLength = SLACK_SECTION_TEXT_LIMIT) {
  const separator = "\n\n";
  const fixedLength =
    parts.reduce((length, part) => length + `*${part.label}*\n`.length, 0) +
    separator.length * Math.max(0, parts.length - 1);
  const valueBudget = Math.max(0, Math.floor((maxLength - fixedLength) / Math.max(1, parts.length)));
  const text = parts
    .map((part) => buildSlackLabeledText(part.label, part.value, `*${part.label}*\n`.length + valueBudget))
    .join(separator);
  return truncate(text, maxLength);
}

export function extractGithubUrls(text) {
  const matches = normalizeText(text).match(/https:\/\/github\.com\/[^\s)>"]+/g) ?? [];
  return [...new Set(matches.map((url) => url.replace(/[.,;:]+$/, "")))];
}

export function parseResourceApprovalRequest(comment) {
  const body = getCommentBody(comment);
  const text = normalizeText(body);
  const lines = text.split("\n");
  const headingIndex = lines.findIndex((line) =>
    /^## Ralph Cycle (\d+) Resource Approval Request\s*$/.test(line),
  );

  if (headingIndex === -1) {
    return {
      ok: false,
      found: false,
      cycleNumber: null,
      requestCommentId: getCommentId(comment),
      requestCommentUrl: getCommentUrl(comment),
      fields: {},
      missingFields: [...REQUIRED_RESOURCE_FIELDS],
      errors: ["Missing Ralph Cycle resource approval request heading."],
    };
  }

  const cycleNumber = Number(lines[headingIndex].match(/^## Ralph Cycle (\d+)/)[1]);
  const boldFields = parseBoldFields(lines.slice(headingIndex + 1));
  const sectionFields = parseHeadingSections(lines, headingIndex);
  const fields = {};

  for (const required of REQUIRED_RESOURCE_FIELDS) {
    const value =
      boldFields.get(required) ??
      sectionFields.get(required) ??
      boldFields.get(required.toLowerCase()) ??
      sectionFields.get(required.toLowerCase()) ??
      "";
    if (value) {
      fields[required] = sanitizePromptField(value);
    }
  }

  const missingFields = REQUIRED_RESOURCE_FIELDS.filter((field) => !fields[field]);

  return {
    ok: missingFields.length === 0,
    found: true,
    cycleNumber,
    requestCommentId: getCommentId(comment),
    requestCommentUrl: getCommentUrl(comment),
    fields,
    missingFields,
    errors: missingFields.map((field) => `Missing required field: ${field}`),
  };
}

export function hasStage10NotificationMarker(
  comments,
  requestCommentId,
  cycleNumber,
  markerPrefix = STAGE10_MARKER_PREFIX,
) {
  const requestNeedle = `request_comment_id=${requestCommentId}`;
  const cycleNeedle = `cycle=${cycleNumber}`;
  return comments.some((comment) => {
    const body = getCommentBody(comment);
    return (
      body.includes(markerPrefix) &&
      body.includes(requestNeedle) &&
      body.includes(cycleNeedle)
    );
  });
}

export function findLatestResourceApprovalRequest(
  comments,
  markerPrefix = STAGE10_MARKER_PREFIX,
) {
  const ordered = [...comments].reverse();
  for (const comment of ordered) {
    const parsed = parseResourceApprovalRequest(comment);
    if (!parsed.found || !parsed.ok) {
      continue;
    }
    if (
      parsed.requestCommentId &&
      hasStage10NotificationMarker(
        comments,
        parsed.requestCommentId,
        parsed.cycleNumber,
        markerPrefix,
      )
    ) {
      continue;
    }
    return parsed;
  }
  return null;
}

export function buildStage10ApprovalSlackMessage(input) {
  const fields = input.fields ?? {};
  const issueNumber = Number(input.issueNumber ?? input.issue_number);
  const cycleNumber = Number(input.cycleNumber ?? input.cycle_number);
  const requestCommentId = input.requestCommentId ?? input.request_comment_id ?? 0;
  const prNumber = Number(input.prNumber ?? input.pr_number ?? 0);
  const workflowUrl =
    input.workflowUrl ??
    "https://github.com/hgahub/duumbi/actions/workflows/stage-10-authorization.yml";
  const issueUrl = input.issueUrl ?? fields.Issue ?? `https://github.com/hgahub/duumbi/issues/${issueNumber}`;
  const requestUrl = input.requestCommentUrl ?? issueUrl;
  const title = `DUUMBI Stage 10 resource authorization needed for #${issueNumber}`;

  const actionValue = (decision) =>
    JSON.stringify({
      action_type: "stage_10_authorization",
      issue_number: issueNumber,
      cycle_number: cycleNumber,
      request_comment_id: requestCommentId,
      decision,
      pr_number: prNumber,
    });

  return {
    text: title,
    blocks: [
      {
        type: "section",
        text: {
          type: "mrkdwn",
          text: `*${sanitizeSlackField(title, SLACK_SECTION_TEXT_LIMIT - 20)}*\nCycle ${cycleNumber}`,
        },
      },
      {
        type: "section",
        fields: [
          { type: "mrkdwn", text: `*Issue*\n<${issueUrl}|#${issueNumber}>` },
          { type: "mrkdwn", text: `*Request*\n<${requestUrl}|comment ${requestCommentId}>` },
          {
            type: "mrkdwn",
            text: buildSlackLabeledText(
              "Product spec",
              fields["Product spec"] ?? input.productSpec ?? "unavailable",
            ),
          },
          {
            type: "mrkdwn",
            text: buildSlackLabeledText(
              "Technical spec",
              fields["Technical spec"] ?? input.technicalSpec ?? "unavailable",
            ),
          },
        ],
      },
      {
        type: "section",
        text: {
          type: "mrkdwn",
          text: buildSlackSectionText([
            { label: "Proposed cycle goal", value: fields["Proposed Cycle Goal"] },
            { label: "Resource estimate", value: fields["Resource Estimate"] },
            { label: "Planned checks", value: fields["Planned Checks"] },
            { label: "Approval trigger", value: fields["Approval Trigger"] },
            { label: "Stop condition", value: fields["Stop Condition"] },
          ]),
        },
      },
      {
        type: "actions",
        elements: [
          {
            type: "button",
            text: { type: "plain_text", text: "Approve Cycle" },
            style: "primary",
            value: actionValue("approve"),
          },
          {
            type: "button",
            text: { type: "plain_text", text: "Narrow Scope" },
            value: actionValue("narrow-scope"),
          },
          {
            type: "button",
            text: { type: "plain_text", text: "Reject / Defer" },
            style: "danger",
            value: actionValue("reject-defer"),
          },
        ],
      },
      {
        type: "context",
        elements: [
          {
            type: "mrkdwn",
            text: `Fallback: <${workflowUrl}|manual Stage 10 authorization workflow>. Slack authorizes at most this bounded cycle and never runs implementation.`,
          },
        ],
      },
    ],
  };
}

export function buildStage10DecisionComment(input) {
  const decision = input.decision;
  if (!Object.hasOwn(DECISION_LABELS, decision)) {
    throw new Error(`Unsupported Stage 10 decision: ${decision}`);
  }

  const nextState = input.nextState ?? NEXT_STATE_BY_DECISION[decision];
  const rationale =
    sanitizePromptField(input.rationale) ||
    (decision === "narrow-scope"
      ? "Narrow scope requested; reviewer details were not supplied in the button payload. Agent must request or use a narrower proposal before continuing."
      : `${DECISION_LABELS[decision]} by ${input.reviewer ?? "reviewer"}.`);
  const authorizedScope =
    decision === "approve"
      ? sanitizePromptField(input.authorizedScope ?? input.proposedCycleGoal ?? "The named bounded cycle in the request comment.")
      : decision === "reject-defer"
        ? "No cycle is authorized."
        : "The original cycle is not authorized; a revised narrower request is required.";

  return [
    "## Stage 10 Resource Authorization Decision",
    "",
    `**Decision:** ${DECISION_LABELS[decision]}`,
    `**Reviewer source:** ${sanitizePromptField(input.reviewer ?? "GitHub Actions")}`,
    `**Issue:** ${sanitizePromptField(input.issueUrl)}`,
    `**Cycle:** ${Number(input.cycleNumber ?? input.cycle_number)}`,
    `**Request comment:** ${sanitizePromptField(input.requestCommentUrl ?? "unavailable")}`,
    `**Rationale:** ${rationale}`,
    `**Authorized scope:** ${authorizedScope}`,
    `**Next state:** ${nextState}`,
  ].join("\n");
}

export function buildStage11Prompt(input) {
  return [
    "Run DUUMBI Stage 11 Review Artifact with duumbi-review-artifact.",
    "",
    `Target issue: ${sanitizePromptField(input.issueUrl)}`,
    `Implementation PR: ${sanitizePromptField(input.prUrl)}`,
    `Product spec artifact: ${sanitizePromptField(input.productSpec)}`,
    `Technical spec artifact: ${sanitizePromptField(input.technicalSpec)}`,
    "",
    "Goal: Verify CI, implementation evidence, BDD coverage, live E2E evidence, and",
    "Ralph Cycle evidence against the approved specs; produce a structured review",
    "artifact and recommendation for a human merge decision.",
    "",
    "Review policy: run Codex self-review as part of the artifact, inspect Copilot",
    "review state, treat CodeRabbit as advisory when present, and do not invoke",
    "Greptile unless the PR is a stable high-risk implementation change and the",
    "developer explicitly requested a manual deep review.",
    "",
    "Do not merge PRs, close issues, move work to Done, perform Stage 12 closure, or",
    "modify implementation code, specs, generated artifacts, or runtime assets.",
  ].join("\n");
}

export function buildStage11ReviewSlackMessage(input) {
  const issueNumber = Number(input.issueNumber ?? input.issue_number);
  const prNumber = Number(input.prNumber ?? input.pr_number);
  const prompt = input.prompt ?? buildStage11Prompt(input);
  return {
    text: `DUUMBI Stage 11 review handoff ready for #${issueNumber}`,
    blocks: [
      {
        type: "section",
        text: {
          type: "mrkdwn",
          text: `*DUUMBI Stage 11 review handoff ready*\nIssue <${input.issueUrl}|#${issueNumber}> · PR <${input.prUrl}|#${prNumber}>`,
        },
      },
      {
        type: "section",
        fields: [
          { type: "mrkdwn", text: buildSlackLabeledText("Product spec", input.productSpec) },
          { type: "mrkdwn", text: buildSlackLabeledText("Technical spec", input.technicalSpec) },
          { type: "mrkdwn", text: buildSlackLabeledText("Checks", input.checkSummary ?? "unavailable") },
          { type: "mrkdwn", text: buildSlackLabeledText("Evidence", input.evidenceSummary ?? "unavailable") },
        ],
      },
      {
        type: "section",
        text: {
          type: "mrkdwn",
          text: buildSlackLabeledText(
            "Ready-to-run Codex prompt",
            `\`\`\`${prompt}\`\`\``,
            SLACK_SECTION_TEXT_LIMIT,
          ),
        },
      },
    ],
  };
}

export function findStage6ProductSpecArtifact(comments) {
  return findSpecArtifact(comments, /## Stage 6 Product Spec Draft/i, /Product spec artifact:\s*(https:\/\/github\.com\/[^\s)]+)/i);
}

export function findStage8TechnicalSpecArtifact(comments) {
  return findSpecArtifact(comments, /## Stage 8 Technical Spec Draft/i, /Technical spec artifact:\s*(https:\/\/github\.com\/[^\s)]+)/i);
}

function findSpecArtifact(comments, headingPattern, artifactPattern) {
  for (const comment of [...comments].reverse()) {
    const body = getCommentBody(comment);
    if (!headingPattern.test(body)) {
      continue;
    }
    const match = body.match(artifactPattern);
    if (match) {
      return match[1].replace(/[.,;:]+$/, "");
    }
  }
  return null;
}

export function buildWorkflowMetrics(input) {
  return {
    schema_version: "duumbi.workflow_metrics.v1",
    generated_at: input.generatedAt ?? new Date().toISOString(),
    source: "github_actions",
    repository: input.repository ?? "hgahub/duumbi",
    workflow: {
      name: input.workflowName ?? null,
      file: input.workflowFile ?? null,
      run_id: input.runId ?? null,
      run_attempt: input.runAttempt ?? null,
      event_name: input.eventName ?? null,
      actor: input.actor ?? null,
      ref: input.ref ?? null,
      sha: input.sha ?? null,
      conclusion: input.conclusion ?? null,
      started_at: input.startedAt ?? null,
      completed_at: input.completedAt ?? null,
      duration_ms: input.durationMs ?? null,
    },
    correlation: {
      issue_number: input.issueNumber ?? null,
      pr_number: input.prNumber ?? null,
      stage: input.stage ?? null,
      decision: input.decision ?? null,
      project_status_target: input.projectStatusTarget ?? null,
    },
    counts: {
      issues_considered: input.issuesConsidered ?? 0,
      issues_queued: input.issuesQueued ?? 0,
      slack_notifications_attempted: input.slackNotificationsAttempted ?? 0,
      request_parse_failures: input.requestParseFailures ?? 0,
      artifact_links_found: input.artifactLinksFound ?? 0,
      artifact_links_missing: input.artifactLinksMissing ?? 0,
    },
    provider_usage: {
      available: false,
      reason: "no_provider_step",
    },
    privacy: {
      raw_slack_payloads: false,
      issue_or_comment_bodies: false,
      prompts_or_completions: false,
      secrets: false,
    },
  };
}

export { STAGE10_MARKER_PREFIX, STAGE11_MARKER_PREFIX, REQUIRED_RESOURCE_FIELDS };
