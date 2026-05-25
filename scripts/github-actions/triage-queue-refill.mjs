import fsModule from "node:fs";
import pathModule from "node:path";

export const WORKFLOW_FILE = ".github/workflows/triage-queue-refill.yml";
export const HUMAN_ACCEPTANCE_STATUS = "Needs Human Acceptance";
export const TODO_STATUS = "Todo";
export const STATUS_FIELD_NAME = "Status";
export const HUMAN_ACCEPTANCE_LABEL = "needs-human-review";
export const DEFAULT_TARGET_HUMAN_ACCEPTANCE_MIN = 3;
export const DEFAULT_DEEPSEEK_MODEL = "deepseek-v4-pro";
export const MAX_ISSUES_CREATED_PER_RUN = 1;

const ACTIVE_VAULT_DOCS = [
  "Duumbi/How to use.md",
  "Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md",
  "Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md",
  "Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md",
  "Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md",
];

const DEEPSEEK_PRICING_USD_PER_1M = {
  "deepseek-v4-flash": {
    inputCacheHit: 0.0028,
    inputCacheMiss: 0.14,
    output: 0.28,
  },
  "deepseek-v4-pro": {
    inputCacheHit: 0.003625,
    inputCacheMiss: 0.435,
    output: 0.87,
  },
};

function nowIso() {
  return new Date().toISOString();
}

export function normalizeText(value) {
  return String(value ?? "")
    .replace(/\r\n?/g, "\n")
    .replace(/\u0000/g, "")
    .replace(/[\u0001-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, "")
    .trim();
}

export function truncateText(value, maxLength = 4000) {
  const text = normalizeText(value);
  if (maxLength <= 0) return "";
  if (text.length <= maxLength) return text;
  const ellipsis = "...";
  return `${text.slice(0, Math.max(0, maxLength - ellipsis.length)).trimEnd()}${ellipsis}`;
}

function parsePositiveInteger(value, fallback) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function labelsForIssue(issue) {
  return (issue?.labels?.nodes || issue?.labels || [])
    .map((label) => (typeof label === "string" ? label : label?.name))
    .filter(Boolean);
}

export function statusNameForProjectItem(item) {
  const values = item?.fieldValues?.nodes || [];
  const status = values.find((value) => value?.field?.name === STATUS_FIELD_NAME);
  return status?.name || null;
}

export function openIssueItemsWithStatus(project, statusName) {
  return (project?.items?.nodes || []).filter((item) => {
    const issue = item?.content;
    return issue?.state === "OPEN" && statusNameForProjectItem(item) === statusName;
  });
}

export function countOpenIssueItemsWithStatus(project, statusName = HUMAN_ACCEPTANCE_STATUS) {
  return openIssueItemsWithStatus(project, statusName).length;
}

export function summarizeIssueItem(item) {
  const issue = item.content || {};
  return {
    item_id: item.id,
    node_id: issue.id || null,
    number: Number(issue.number),
    title: truncateText(issue.title, 240),
    url: issue.url || null,
    state: issue.state || null,
    status: statusNameForProjectItem(item),
    labels: labelsForIssue(issue),
    body: truncateText(issue.body || "", 2500),
  };
}

export function findStatusField(project) {
  const fields = project?.fields?.nodes || [];
  return fields.find((field) => field?.name === STATUS_FIELD_NAME && Array.isArray(field.options)) || null;
}

export function findStatusOption(project, statusName) {
  const field = findStatusField(project);
  return field?.options?.find((option) => option?.name === statusName) || null;
}

export function validateProjectStatusConfig(project) {
  const statusField = findStatusField(project);
  if (!statusField) {
    throw new Error("Project V2 Status field is not readable; failing closed.");
  }
  const humanAcceptanceOption = findStatusOption(project, HUMAN_ACCEPTANCE_STATUS);
  if (!humanAcceptanceOption) {
    throw new Error(`Project V2 Status option "${HUMAN_ACCEPTANCE_STATUS}" is not readable; failing closed.`);
  }
  return { statusField, humanAcceptanceOption };
}

export function buildProjectSnapshot(project, targetMinimum = DEFAULT_TARGET_HUMAN_ACCEPTANCE_MIN) {
  const items = project?.items?.nodes || [];
  const humanAcceptanceIssues = openIssueItemsWithStatus(project, HUMAN_ACCEPTANCE_STATUS).map(summarizeIssueItem);
  const eligibleTodoIssues = openIssueItemsWithStatus(project, TODO_STATUS).map(summarizeIssueItem);
  const openProjectIssues = items
    .filter((item) => item?.content?.state === "OPEN")
    .map(summarizeIssueItem);

  return {
    project_title: project?.title || "",
    target_status: HUMAN_ACCEPTANCE_STATUS,
    target_minimum: targetMinimum,
    current_human_acceptance_count: humanAcceptanceIssues.length,
    eligible_todo_issues: eligibleTodoIssues,
    human_acceptance_issues: humanAcceptanceIssues,
    open_project_issues: openProjectIssues,
  };
}

function walkMarkdownFiles(fs, path, root, limit) {
  const results = [];
  const visit = (dir) => {
    if (results.length >= limit) return;
    const entries = fs
      .readdirSync(dir, { withFileTypes: true })
      .sort((a, b) => a.name.localeCompare(b.name));
    for (const entry of entries) {
      if (results.length >= limit) return;
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        visit(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".md")) {
        results.push(fullPath);
      }
    }
  };
  visit(root);
  return results;
}

function readMarkdownNotes(fs, path, root, baseRoot, limit, maxChars) {
  if (!fs.existsSync(root)) return [];
  return walkMarkdownFiles(fs, path, root, limit).map((absolutePath) => ({
    path: path.relative(baseRoot, absolutePath).split(path.sep).join("/"),
    text: truncateText(fs.readFileSync(absolutePath, "utf8"), maxChars),
  }));
}

function readVaultDocs(fs, path, vaultRoot, warnings) {
  return ACTIVE_VAULT_DOCS.flatMap((relativePath) => {
    const absolutePath = path.join(vaultRoot, relativePath);
    if (!fs.existsSync(absolutePath)) {
      warnings.push(`Vault context document is missing: ${relativePath}`);
      return [];
    }
    return [{
      path: relativePath,
      text: truncateText(fs.readFileSync(absolutePath, "utf8"), 5000),
    }];
  });
}

export async function collectTriageContext({
  fs = fsModule,
  path = pathModule,
  workspace = process.cwd(),
  vaultPath = "duumbi-vault",
  project,
  targetMinimum,
  api,
  warnings = [],
}) {
  const vaultRoot = path.resolve(workspace, vaultPath);
  const duumbiRoot = path.join(vaultRoot, "Duumbi");
  if (!fs.existsSync(vaultRoot) || !fs.statSync(vaultRoot).isDirectory()) {
    throw new Error(`DUUMBI vault checkout is unavailable at ${vaultRoot}; failing closed.`);
  }
  if (!fs.existsSync(duumbiRoot) || !fs.statSync(duumbiRoot).isDirectory()) {
    throw new Error(`DUUMBI vault root is unavailable at ${duumbiRoot}; failing closed.`);
  }

  let ideaDiscussions = [];
  try {
    ideaDiscussions = await api.listIdeaDiscussions();
  } catch (error) {
    warnings.push(`GitHub Ideas Discussions context unavailable: ${truncateText(error.message, 180)}`);
  }

  const inboxRoot = path.join(duumbiRoot, "00 Inbox (ToProcess)");
  const inboxNotes = readMarkdownNotes(fs, path, inboxRoot, vaultRoot, 8, 3500);
  const activeDocs = readVaultDocs(fs, path, vaultRoot, warnings);

  return {
    generated_at: nowIso(),
    repository: `${api.owner}/${api.repo}`,
    stage: "triage-queue-refill",
    policy: {
      max_new_human_acceptance_issues: MAX_ISSUES_CREATED_PER_RUN,
      target_status: HUMAN_ACCEPTANCE_STATUS,
      target_label: HUMAN_ACCEPTANCE_LABEL,
      route_existing_issue_requires_status: TODO_STATUS,
      forbidden: [
        "Do not create product specs.",
        "Do not create technical specs.",
        "Do not create PRs.",
        "Do not modify source code.",
        "Do not start implementation.",
        "Do not mark work accepted.",
      ],
    },
    project: buildProjectSnapshot(project, targetMinimum),
    inbox_notes: inboxNotes,
    idea_discussions: ideaDiscussions.slice(0, 10).map((discussion) => ({
      title: truncateText(discussion.title, 240),
      url: discussion.url,
      category: discussion.category?.name || null,
      updated_at: discussion.updatedAt || null,
      body: truncateText(discussion.body || "", 3000),
    })),
    active_vault_docs: activeDocs,
  };
}

export function buildDeepSeekMessages(contextPayload) {
  const schema = {
    action: "route_existing_issue | create_issue | needs_clarification | no_action",
    existing_issue_number: "integer, required only for route_existing_issue",
    issue: {
      title: "string, required only for create_issue",
      body: "string, required only for create_issue",
      user_outcome: "string",
      proposed_direction: "string",
      scope_in: ["string"],
      scope_out: ["string"],
      risks: ["string"],
      open_questions: ["string"],
    },
    source_links: ["non-empty source URLs or vault paths for route_existing_issue/create_issue"],
    rationale: "short factual rationale",
  };

  return [
    {
      role: "system",
      content: [
        "You are the DUUMBI Stage 4 triage refiller.",
        "Return exactly one valid json object and no markdown.",
        "Choose at most one next item.",
        "Prefer routing an eligible existing Todo issue when it already represents the work.",
        `Only route existing issues listed under project.eligible_todo_issues, and route them to ${HUMAN_ACCEPTANCE_STATUS}.`,
        "Create a new issue only when no existing eligible Todo issue represents the next actionable work.",
        "Use no_action when no actionable item remains. Use needs_clarification when the next candidate cannot be routed safely.",
        "Do not approve work, create specs, create PRs, change source code, or start implementation.",
        `Expected json schema example: ${JSON.stringify(schema)}`,
      ].join("\n"),
    },
    {
      role: "user",
      content: JSON.stringify(contextPayload, null, 2),
    },
  ];
}

function stripJsonFence(value) {
  const text = normalizeText(value);
  const fenceMatch = text.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/i);
  return fenceMatch ? fenceMatch[1].trim() : text;
}

export function parseTriageDecision(content) {
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
    throw new Error(`DeepSeek decision was not valid JSON: ${firstError.message}`);
  }
}

function normalizeStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => truncateText(item, 1000)).filter(Boolean);
}

function requireSourceLinks(decision) {
  const sourceLinks = normalizeStringArray(decision.source_links);
  if (sourceLinks.length === 0) {
    throw new Error("DeepSeek decision is missing required source_links.");
  }
  return sourceLinks;
}

export function validateTriageDecision(decision, contextPayload) {
  const action = normalizeText(decision?.action);
  const validActions = new Set(["route_existing_issue", "create_issue", "needs_clarification", "no_action"]);
  if (!validActions.has(action)) {
    throw new Error(`DeepSeek decision action is not supported: ${action || "<missing>"}`);
  }

  const normalized = {
    action,
    rationale: truncateText(decision.rationale || "", 2000),
    source_links: normalizeStringArray(decision.source_links),
  };

  if (action === "no_action" || action === "needs_clarification") {
    return normalized;
  }

  normalized.source_links = requireSourceLinks(decision);

  if (action === "route_existing_issue") {
    const issueNumber = Number(decision.existing_issue_number);
    if (!Number.isInteger(issueNumber) || issueNumber < 1) {
      throw new Error("route_existing_issue requires a positive existing_issue_number.");
    }
    const eligible = new Map((contextPayload?.project?.eligible_todo_issues || []).map((issue) => [Number(issue.number), issue]));
    if (!eligible.has(issueNumber)) {
      throw new Error(`Issue #${issueNumber} is not an eligible Todo issue for triage refill.`);
    }
    return {
      ...normalized,
      existing_issue_number: issueNumber,
      existing_issue: eligible.get(issueNumber),
    };
  }

  const issue = decision.issue || {};
  const title = truncateText(issue.title, 240);
  const body = truncateText(issue.body, 8000);
  if (!title) {
    throw new Error("create_issue decision is missing issue.title.");
  }
  if (!body) {
    throw new Error("create_issue decision is missing issue.body.");
  }

  return {
    ...normalized,
    issue: {
      title,
      body,
      user_outcome: truncateText(issue.user_outcome || "", 3000),
      proposed_direction: truncateText(issue.proposed_direction || "", 3000),
      scope_in: normalizeStringArray(issue.scope_in),
      scope_out: normalizeStringArray(issue.scope_out),
      risks: normalizeStringArray(issue.risks),
      open_questions: normalizeStringArray(issue.open_questions),
    },
  };
}

function markdownList(items, fallback = "- none") {
  const values = normalizeStringArray(items);
  return values.length ? values.map((item) => `- ${item}`).join("\n") : fallback;
}

export function buildCreatedIssueBody(decision) {
  const issue = decision.issue;
  return [
    "## Summary",
    "",
    issue.body,
    "",
    "## Source",
    "- Origin: Automated Stage 4 triage refill",
    "- Links:",
    markdownList(decision.source_links),
    "",
    "## User Outcome",
    "",
    issue.user_outcome || "Triage should determine whether this is worth accepting for specification.",
    "",
    "## Problem",
    "",
    issue.body,
    "",
    "## Proposed Direction",
    "",
    issue.proposed_direction || "Route this candidate to human acceptance before specification.",
    "",
    "## Knowledge Context",
    "- Relevant Obsidian notes:",
    markdownList(decision.source_links.filter((link) => !/^https?:\/\//i.test(link))),
    "- Related issues or discussions:",
    markdownList(decision.source_links.filter((link) => /^https?:\/\//i.test(link))),
    "- Existing code or docs:",
    "- not inspected by automated refill",
    "",
    "## Scope Candidate",
    "- In:",
    markdownList(issue.scope_in),
    "- Out:",
    markdownList(issue.scope_out),
    "",
    "## Risks And Trade-Offs",
    "",
    markdownList(issue.risks),
    "",
    "## Open Questions",
    "",
    markdownList(issue.open_questions),
    "",
    "## Triage Recommendation",
    "- accept for spec",
    "",
    "## Acceptance Gate",
    "- [ ] Human reviewed",
    "- [ ] Accepted for spec",
    "",
    "<!-- duumbi-triage-queue-refill:v1 -->",
  ].join("\n");
}

export function buildExistingIssueComment(decision) {
  return [
    "<!-- duumbi-triage-queue-refill:v1 -->",
    "## Stage 4 Automated Triage Refill",
    "",
    `**Decision:** Route existing issue to \`${HUMAN_ACCEPTANCE_STATUS}\`.`,
    `**Rationale:** ${decision.rationale || "Selected as the next bounded triage candidate."}`,
    "",
    "**Source links:**",
    markdownList(decision.source_links),
    "",
    "**Next stage:** Needs Human Acceptance",
  ].join("\n");
}

export function estimateDeepSeekCostUsd(model, usage = {}) {
  const pricing = DEEPSEEK_PRICING_USD_PER_1M[model] || DEEPSEEK_PRICING_USD_PER_1M[DEFAULT_DEEPSEEK_MODEL];
  const completionTokens = Number(usage.completion_tokens || 0);
  const cacheHitTokens = Number(usage.prompt_cache_hit_tokens || 0);
  const cacheMissTokens = Number(usage.prompt_cache_miss_tokens || 0);
  const promptTokens = Number(usage.prompt_tokens || 0);

  const inputCost = cacheHitTokens || cacheMissTokens
    ? (cacheHitTokens * pricing.inputCacheHit + cacheMissTokens * pricing.inputCacheMiss) / 1_000_000
    : (promptTokens * pricing.inputCacheMiss) / 1_000_000;
  const outputCost = (completionTokens * pricing.output) / 1_000_000;
  const cost = inputCost + outputCost;
  return Number.isFinite(cost) ? Number(cost.toFixed(8)) : null;
}

async function readJsonResponse(response) {
  const text = await response.text();
  try {
    return { text, json: text ? JSON.parse(text) : {} };
  } catch (error) {
    throw new Error(`Response was not JSON: ${truncateText(error.message, 160)}; body=${truncateText(text, 240)}`);
  }
}

export function createGithubApi({ fetchImpl = fetch, token, owner, repo }) {
  const authHeaders = {
    Authorization: `bearer ${token}`,
    "Content-Type": "application/json",
    "User-Agent": "duumbi-triage-queue-refill",
  };

  const graphql = async (query, variables = {}) => {
    const response = await fetchImpl("https://api.github.com/graphql", {
      method: "POST",
      headers: authHeaders,
      body: JSON.stringify({ query, variables }),
    });
    const { text, json } = await readJsonResponse(response);
    if (!response.ok) {
      throw new Error(`GitHub GraphQL failed: ${response.status} ${truncateText(text, 240)}`);
    }
    if (json.errors) {
      throw new Error(json.errors.map((error) => error.message).join("; "));
    }
    return json.data;
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
    graphql,
    rest,
    async loadProject(projectOwner, projectNumber) {
      let cursor = null;
      let title = "";
      let id = "";
      let fields = { nodes: [] };
      const nodes = [];
      for (;;) {
        const data = await graphql(`
          query($login: String!, $number: Int!, $cursor: String) {
            organization(login: $login) {
              projectV2(number: $number) {
                id
                title
                fields(first: 50) {
                  nodes {
                    ... on ProjectV2SingleSelectField { id name options { id name } }
                  }
                }
                items(first: 100, after: $cursor) {
                  pageInfo { hasNextPage endCursor }
                  nodes {
                    id
                    content {
                      ... on Issue {
                        id
                        number
                        title
                        url
                        state
                        body
                        labels(first: 20) { nodes { name } }
                      }
                    }
                    fieldValues(first: 50) {
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
            user(login: $login) {
              projectV2(number: $number) {
                id
                title
                fields(first: 50) {
                  nodes {
                    ... on ProjectV2SingleSelectField { id name options { id name } }
                  }
                }
                items(first: 100, after: $cursor) {
                  pageInfo { hasNextPage endCursor }
                  nodes {
                    id
                    content {
                      ... on Issue {
                        id
                        number
                        title
                        url
                        state
                        body
                        labels(first: 20) { nodes { name } }
                      }
                    }
                    fieldValues(first: 50) {
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
          }`, { login: projectOwner, number: projectNumber, cursor });
        const page = data.organization?.projectV2 || data.user?.projectV2;
        if (!page) return null;
        id = id || page.id;
        title = title || page.title;
        fields = page.fields || fields;
        nodes.push(...(page.items?.nodes || []));
        const pageInfo = page.items?.pageInfo;
        if (!pageInfo?.hasNextPage) break;
        cursor = pageInfo.endCursor;
      }
      return { id, title, fields, items: { nodes } };
    },
    async listIdeaDiscussions() {
      const data = await graphql(`
        query($owner: String!, $repo: String!) {
          repository(owner: $owner, name: $repo) {
            discussions(first: 20, orderBy: { field: UPDATED_AT, direction: DESC }) {
              nodes {
                title
                url
                body
                updatedAt
                category { name }
              }
            }
          }
        }`, { owner, repo });
      return (data.repository?.discussions?.nodes || []).filter((discussion) =>
        String(discussion.category?.name || "").toLowerCase() === "ideas",
      );
    },
    async updateProjectStatus(project, itemId, statusName) {
      const statusField = findStatusField(project);
      const option = findStatusOption(project, statusName);
      if (!statusField || !option) {
        throw new Error(`Project V2 Status option "${statusName}" is not readable; failing closed.`);
      }
      await graphql(`
        mutation($projectId: ID!, $itemId: ID!, $fieldId: ID!, $optionId: String!) {
          updateProjectV2ItemFieldValue(input: {
            projectId: $projectId,
            itemId: $itemId,
            fieldId: $fieldId,
            value: { singleSelectOptionId: $optionId }
          }) { projectV2Item { id } }
        }`, {
        projectId: project.id,
        itemId,
        fieldId: statusField.id,
        optionId: option.id,
      });
    },
    async addIssueToProject(projectId, issueNodeId) {
      const data = await graphql(`
        mutation($projectId: ID!, $contentId: ID!) {
          addProjectV2ItemById(input: { projectId: $projectId, contentId: $contentId }) {
            item { id }
          }
        }`, { projectId, contentId: issueNodeId });
      return data.addProjectV2ItemById?.item?.id || null;
    },
    async createIssue(title, body) {
      return rest("POST", `/repos/${owner}/${repo}/issues`, { title, body });
    },
    async createIssueComment(issueNumber, body) {
      return rest("POST", `/repos/${owner}/${repo}/issues/${issueNumber}/comments`, { body });
    },
    async addIssueLabel(issueNumber, label) {
      return rest("POST", `/repos/${owner}/${repo}/issues/${issueNumber}/labels`, { labels: [label] });
    },
    async updateIssue(issueNumber, fields) {
      return rest("PATCH", `/repos/${owner}/${repo}/issues/${issueNumber}`, fields);
    },
  };
}

export async function callDeepSeek({
  fetchImpl = fetch,
  apiKey,
  model = DEFAULT_DEEPSEEK_MODEL,
  messages,
  timeoutMs = 120_000,
}) {
  const started = Date.now();
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetchImpl("https://api.deepseek.com/chat/completions", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model,
        messages,
        response_format: { type: "json_object" },
        temperature: 0.2,
        max_tokens: 2500,
      }),
      signal: controller.signal,
    });
    const { text, json } = await readJsonResponse(response);
    if (!response.ok) {
      throw new Error(`DeepSeek API failed: ${response.status} ${truncateText(text, 240)}`);
    }
    const content = json.choices?.[0]?.message?.content;
    if (!normalizeText(content)) {
      throw new Error("DeepSeek returned empty decision content.");
    }
    return {
      model: json.model || model,
      content,
      usage: json.usage || {},
      latencyMs: Date.now() - started,
    };
  } catch (error) {
    if (error?.name === "AbortError") {
      throw new Error(`DeepSeek API timed out after ${timeoutMs}ms`);
    }
    throw error;
  } finally {
    clearTimeout(timeout);
  }
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
      stage: "triage-queue-refill",
      decision,
      project_status: HUMAN_ACCEPTANCE_STATUS,
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

async function applyTriageDecision({ api, project, decision }) {
  if (decision.action === "route_existing_issue") {
    await api.createIssueComment(decision.existing_issue_number, buildExistingIssueComment(decision));
    await api.addIssueLabel(decision.existing_issue_number, HUMAN_ACCEPTANCE_LABEL);
    await api.updateProjectStatus(project, decision.existing_issue.item_id, HUMAN_ACCEPTANCE_STATUS);
    return {
      issueNumber: decision.existing_issue_number,
      issueUrl: decision.existing_issue.url,
      action: decision.action,
    };
  }

  if (decision.action === "create_issue") {
    const createdIssue = await api.createIssue(decision.issue.title, buildCreatedIssueBody(decision));
    let itemId = null;
    try {
      itemId = await api.addIssueToProject(project.id, createdIssue.node_id);
      if (!itemId) {
        throw new Error("Created issue could not be added to Project V2.");
      }
      await api.addIssueLabel(createdIssue.number, HUMAN_ACCEPTANCE_LABEL);
      await api.updateProjectStatus(project, itemId, HUMAN_ACCEPTANCE_STATUS);
    } catch (error) {
      const rollbackBody = [
        "<!-- duumbi-triage-queue-refill-rollback:v1 -->",
        "Automated triage refill rolled this issue back because the Project V2 queue update did not complete.",
        "",
        `Failure: ${truncateText(error.message || error, 500)}`,
      ].join("\n");
      try {
        await api.createIssueComment(createdIssue.number, rollbackBody);
        await api.updateIssue(createdIssue.number, { state: "closed" });
      } catch (rollbackError) {
        throw new Error(`Created issue #${createdIssue.number} could not be fully queued, and rollback failed: ${truncateText(rollbackError.message || rollbackError, 300)}`);
      }
      throw new Error(`Created issue #${createdIssue.number} was closed after incomplete queue update: ${truncateText(error.message || error, 300)}`);
    }
    return {
      issueNumber: createdIssue.number,
      issueUrl: createdIssue.html_url,
      action: decision.action,
    };
  }

  return { issueNumber: null, issueUrl: null, action: decision.action };
}

async function writeSummary(summary, result) {
  if (!summary) return;
  const table = [
    [{ data: "Field", header: true }, { data: "Value", header: true }],
    ["Project", result.projectTitle || "unavailable"],
    ["Needs Human Acceptance count", String(result.humanAcceptanceCount ?? "unavailable")],
    ["Target minimum", String(result.targetMinimum ?? DEFAULT_TARGET_HUMAN_ACCEPTANCE_MIN)],
    ["Refill needed", String(result.refillNeeded)],
    ["Decision", result.decision || "none"],
    ["Issue", result.issueNumber ? `#${result.issueNumber}` : "none"],
    ["Direct Slack notification", "not_sent"],
    ["Model", result.model || "not_called"],
  ];
  await summary
    .addHeading("DUUMBI Triage Queue Refill", 2)
    .addTable(table)
    .write();
}

export async function runTriageQueueRefill({
  env = process.env,
  context,
  core,
  summary,
  fetchImpl = fetch,
  fs = fsModule,
  path = pathModule,
  workspace = process.cwd(),
} = {}) {
  const warnings = [];
  const metricsPath = env.DUUMBI_METRICS_PATH || "duumbi-workflow-metrics.json";
  const inputs = context?.payload?.inputs || {};
  const targetMinimum = parsePositiveInteger(inputs.target_human_acceptance_min, DEFAULT_TARGET_HUMAN_ACCEPTANCE_MIN);
  const projectPat = env.GH_PROJECT_PAT;
  const projectOwner = env.DUUMBI_PROJECT_OWNER || context.repo.owner;
  const projectNumber = Number(env.DUUMBI_PROJECT_NUMBER || 0);
  const deepSeekApiKey = env.DEEPSEEK_API_KEY;
  const deepSeekModel = env.DEEPSEEK_MODEL || DEFAULT_DEEPSEEK_MODEL;

  let result = {
    ok: false,
    projectTitle: null,
    humanAcceptanceCount: null,
    targetMinimum,
    refillNeeded: false,
    decision: null,
    issueNumber: null,
    model: null,
    inspectedIssueCount: null,
  };

  const writeMetrics = (metrics) => {
    fs.writeFileSync(path.resolve(workspace, metricsPath), `${JSON.stringify(metrics, null, 2)}\n`);
  };

  try {
    if (!projectPat || !projectNumber) {
      throw new Error("Project V2 configuration is unavailable. Set GH_PROJECT_PAT and repository variable DUUMBI_PROJECT_NUMBER.");
    }

    const api = createGithubApi({
      fetchImpl,
      token: projectPat,
      owner: context.repo.owner,
      repo: context.repo.repo,
    });
    const project = await api.loadProject(projectOwner, projectNumber);
    if (!project) {
      throw new Error(`Project V2 #${projectNumber} not found for ${projectOwner}.`);
    }
    validateProjectStatusConfig(project);

    const humanAcceptanceCount = countOpenIssueItemsWithStatus(project, HUMAN_ACCEPTANCE_STATUS);
    result = {
      ...result,
      ok: true,
      projectTitle: project.title,
      humanAcceptanceCount,
      inspectedIssueCount: project.items.nodes.length,
      refillNeeded: humanAcceptanceCount < targetMinimum,
    };

    if (!result.refillNeeded) {
      const metrics = buildWorkflowMetrics({
        context,
        conclusion: "success",
        decision: "not_needed",
        counts: {
          issues_considered: project.items.nodes.length,
          issues_queued: 0,
          slack_notifications_attempted: 0,
          artifact_links_found: null,
          artifact_links_missing: null,
        },
        providerUsage: providerUsageNotCalled("human_acceptance_queue_full"),
        warnings: ["slack_notification_not_needed", "model_call_not_needed"],
      });
      writeMetrics(metrics);
      await writeSummary(summary, { ...result, decision: "not_needed" });
      core?.info?.(`Needs Human Acceptance queue is full: ${humanAcceptanceCount}/${targetMinimum}.`);
      return { ...result, decision: "not_needed" };
    }

    if (!deepSeekApiKey) {
      throw new Error("DEEPSEEK_API_KEY is not configured; failing closed before triage refill.");
    }

    const triageContext = await collectTriageContext({
      fs,
      path,
      workspace,
      project,
      targetMinimum,
      api,
      warnings,
    });
    const messages = buildDeepSeekMessages(triageContext);
    const deepSeekResponse = await callDeepSeek({
      fetchImpl,
      apiKey: deepSeekApiKey,
      model: deepSeekModel,
      messages,
    });
    const parsedDecision = parseTriageDecision(deepSeekResponse.content);
    const decision = validateTriageDecision(parsedDecision, triageContext);
    const applied = await applyTriageDecision({ api, project, decision });
    const queued = applied.issueNumber ? 1 : 0;

    result = {
      ...result,
      ok: true,
      decision: decision.action,
      issueNumber: applied.issueNumber,
      issueUrl: applied.issueUrl,
      model: deepSeekResponse.model,
    };

    const metrics = buildWorkflowMetrics({
      context,
      conclusion: "success",
      decision: decision.action,
      issueNumber: applied.issueNumber,
      counts: {
        issues_considered: project.items.nodes.length,
        issues_queued: queued,
        slack_notifications_attempted: 0,
        artifact_links_found: decision.source_links?.length || null,
        artifact_links_missing: decision.source_links?.length ? 0 : null,
      },
      providerUsage: providerUsageFromDeepSeek(deepSeekResponse.model, deepSeekResponse.usage, deepSeekResponse.latencyMs),
      warnings,
    });
    writeMetrics(metrics);
    await writeSummary(summary, result);
    core?.info?.(`Triage refill decision: ${decision.action}${applied.issueNumber ? ` issue #${applied.issueNumber}` : ""}.`);
    return result;
  } catch (error) {
    const message = truncateText(error.message || error, 500);
    warnings.push(message);
    const metrics = buildWorkflowMetrics({
      context,
      conclusion: "blocked",
      decision: "blocked",
      issueNumber: result.issueNumber,
      counts: {
        issues_considered: result.inspectedIssueCount,
        issues_queued: 0,
        slack_notifications_attempted: 0,
        artifact_links_found: null,
        artifact_links_missing: null,
      },
      providerUsage: providerUsageNotCalled("triage_refill_blocked"),
      warnings,
    });
    writeMetrics(metrics);
    await writeSummary(summary, { ...result, decision: "blocked" });
    core?.setFailed?.(message);
    return { ...result, ok: false, error: message };
  }
}
