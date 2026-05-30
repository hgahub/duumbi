import { execFileSync } from "node:child_process";
import fsModule from "node:fs";
import pathModule from "node:path";

import {
  DEFAULT_DEEPSEEK_MODEL,
  callDeepSeek,
  estimateDeepSeekCostUsd,
  normalizeText,
  truncateText,
} from "./triage-queue-refill.mjs";

export const WORKFLOW_FILE = ".github/workflows/inbox-enrichment-dispatch.yml";
export const ENRICHMENT_MARKER_PREFIX = "<!-- duumbi-inbox-enrichment:v1";
export const DEFAULT_SCAN_LIMIT = 200;

const REQUIRED_ENRICHMENT_SECTIONS = [
  "## Source",
  "## Raw input",
  "## Interpreted intent",
  "## Developer summary",
  "## UML overview",
  "## Classification",
  "## Clarifications",
  "### Answered",
  "### Open",
  "## Relevant DUUMBI context",
  "## Related GitHub context",
  "## Initial routing recommendation",
  "## Requested follow-up",
  "## AI agent instructions",
  "## Obsidian tags",
  "## Enrichment result",
];

const ACTIVE_VAULT_DOCS = [
  "Duumbi/How to use.md",
  "Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md",
  "Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md",
  "Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md",
  "Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md",
];

const SOURCE_CONTEXT_FILES = [
  "AGENTS.md",
  "docs/architecture.md",
  "docs/coding-conventions.md",
  "Cargo.toml",
  "src/types.rs",
  "src/graph/mod.rs",
  "src/compiler/mod.rs",
  "src/agents/mod.rs",
  "src/mcp/mod.rs",
  "crates/duumbi-studio/src/lib.rs",
];

const CLASSIFICATIONS = new Set([
  "idea",
  "bug",
  "feature",
  "research",
  "architecture",
  "execution",
  "knowledge",
  "skill",
  "unclear",
]);

const ESTIMATE_LEVELS = new Set(["low", "medium", "high", "critical"]);
const RESULT_STATUSES = new Set([
  "ready_for_triage",
  "duplicate_candidate",
  "needs_clarification",
  "no_action_candidate",
]);

function nowIso() {
  return new Date().toISOString();
}

function parsePositiveInteger(value, fallback) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function stripJsonFence(value) {
  const text = normalizeText(value);
  const fenceMatch = text.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/i);
  return fenceMatch ? fenceMatch[1].trim() : text;
}

function sanitizeTitle(value) {
  const title = truncateText(value, 140).replace(/\s+/g, " ").trim();
  return title || "Prepared DUUMBI Inbox Task";
}

function normalizeEnum(value, allowed, fallback) {
  const normalized = normalizeText(value).toLowerCase().replace(/\s+/g, "_");
  return allowed.has(normalized) ? normalized : fallback;
}

function normalizeStringArray(value, maxItems = 12, maxLength = 1200) {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => truncateText(item, maxLength))
    .filter(Boolean)
    .slice(0, maxItems);
}

function markdownList(items, fallback = "- none") {
  const values = normalizeStringArray(items);
  return values.length ? values.map((item) => `- ${item}`).join("\n") : fallback;
}

function markdownBlockquote(value) {
  const text = String(value ?? "").replace(/\r\n?/g, "\n").trimEnd();
  if (!text) return "> none";
  return text.split("\n").map((line) => `> ${line}`).join("\n");
}

function stripMarkdownFence(value) {
  return normalizeText(value)
    .replace(/^```(?:mermaid)?\s*/i, "")
    .replace(/\s*```$/i, "")
    .trim();
}

function defaultDiagram(title) {
  const safeTitle = title.replace(/[\[\]{}<>]/g, "").slice(0, 60) || "Prepared task";
  return [
    "flowchart TD",
    "  Raw[Raw inbox note] --> Enriched[Prepared task brief]",
    `  Enriched --> Goal[${safeTitle}]`,
    "  Goal --> Triage[Stage 4 triage]",
    "  Triage --> Issue[Future GitHub issue]",
  ].join("\n");
}

function tagName(value) {
  return normalizeText(value).toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "") || "unclear";
}

export function buildObsidianTags(decision) {
  return [
    "duumbi/inbox/enriched",
    "duumbi/status/processed",
    `duumbi/classification/${tagName(decision.classification)}`,
    `duumbi/value/${tagName(decision.business_value)}`,
    `duumbi/importance/${tagName(decision.importance)}`,
    `duumbi/complexity/${tagName(decision.complexity)}`,
  ];
}

export function hasProcessedMarker(text) {
  const normalized = String(text ?? "").toLowerCase();
  return normalized.includes("duumbi-inbox-enrichment:v1")
    || normalized.includes("duumbi/status/processed")
    || normalized.includes("#duumbi/status/processed");
}

export function isEnrichmentCandidateText(text) {
  const value = String(text ?? "");
  if (!normalizeText(value) || hasProcessedMarker(value)) return false;
  return REQUIRED_ENRICHMENT_SECTIONS.some((section) => !value.includes(section));
}

function toVaultRelative(path, vaultRoot, absolutePath) {
  return path.relative(vaultRoot, absolutePath).split(path.sep).join("/");
}

function normalizeTargetPath(path, vaultRoot, inboxRoot, targetPath) {
  const normalizedTarget = String(targetPath || "").trim().replace(/^duumbi-vault\//, "").replace(/^\/+/, "");
  const targetAbsolute = path.resolve(vaultRoot, normalizedTarget);
  const relativeToInbox = path.relative(inboxRoot, targetAbsolute);
  if (relativeToInbox.startsWith("..") || path.isAbsolute(relativeToInbox)) {
    throw new Error(`Target path is outside Duumbi/00 Inbox (ToProcess): ${targetPath}`);
  }
  return targetAbsolute;
}

export function collectCandidatePaths({
  fs = fsModule,
  path = pathModule,
  vaultRoot,
  inboxRoot,
  targetPath = "",
  scanLimit = DEFAULT_SCAN_LIMIT,
}) {
  if (!fs.existsSync(inboxRoot) || !fs.statSync(inboxRoot).isDirectory()) {
    throw new Error(`Missing Inbox path: ${inboxRoot}`);
  }

  let inspectedCount = 0;
  const candidatePaths = [];
  const effectiveScanLimit = parsePositiveInteger(scanLimit, DEFAULT_SCAN_LIMIT);

  const inspectFile = (absolutePath) => {
    inspectedCount += 1;
    const text = fs.readFileSync(absolutePath, "utf8");
    if (isEnrichmentCandidateText(text)) {
      candidatePaths.push(toVaultRelative(path, vaultRoot, absolutePath));
    }
  };

  if (targetPath) {
    const targetAbsolute = normalizeTargetPath(path, vaultRoot, inboxRoot, targetPath);
    if (!fs.existsSync(targetAbsolute) || !fs.statSync(targetAbsolute).isFile()) {
      throw new Error(`Target Inbox note not found: ${targetPath}`);
    }
    inspectFile(targetAbsolute);
    return { candidatePaths: candidatePaths.slice(0, 1), inspectedCount };
  }

  const visit = (currentDir) => {
    if (candidatePaths.length >= 1 || inspectedCount >= effectiveScanLimit) return;
    const entries = fs.readdirSync(currentDir, { withFileTypes: true })
      .sort((a, b) => a.name.localeCompare(b.name));
    for (const entry of entries) {
      if (candidatePaths.length >= 1 || inspectedCount >= effectiveScanLimit) return;
      const fullPath = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        visit(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".md")) {
        inspectFile(fullPath);
      }
    }
  };

  visit(inboxRoot);
  return { candidatePaths, inspectedCount };
}

function readRelativeFiles(fs, path, root, relativePaths, warnings, maxChars = 4000) {
  return relativePaths.flatMap((relativePath) => {
    const absolutePath = path.join(root, relativePath);
    if (!fs.existsSync(absolutePath) || !fs.statSync(absolutePath).isFile()) {
      warnings.push(`Context file is missing: ${relativePath}`);
      return [];
    }
    return [{
      path: relativePath,
      text: truncateText(fs.readFileSync(absolutePath, "utf8"), maxChars),
    }];
  });
}

function readMarkdownNotes(fs, path, root, baseRoot, limit, maxChars, excluded = new Set()) {
  if (!fs.existsSync(root) || !fs.statSync(root).isDirectory()) return [];
  const notes = [];
  const visit = (dir) => {
    if (notes.length >= limit) return;
    const entries = fs.readdirSync(dir, { withFileTypes: true })
      .sort((a, b) => a.name.localeCompare(b.name));
    for (const entry of entries) {
      if (notes.length >= limit) return;
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        visit(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".md")) {
        const relativePath = toVaultRelative(path, baseRoot, fullPath);
        if (!excluded.has(relativePath)) {
          notes.push({
            path: relativePath,
            text: truncateText(fs.readFileSync(fullPath, "utf8"), maxChars),
          });
        }
      }
    }
  };
  visit(root);
  return notes;
}

export function buildEnrichmentContext({
  fs = fsModule,
  path = pathModule,
  workspace,
  vaultRoot,
  candidatePath,
  warnings = [],
}) {
  const candidateAbsolute = path.join(vaultRoot, candidatePath);
  const candidateText = fs.readFileSync(candidateAbsolute, "utf8");
  const duumbiRoot = path.join(vaultRoot, "Duumbi");
  const inboxRoot = path.join(duumbiRoot, "00 Inbox (ToProcess)");
  const processedInboxRoot = path.join(duumbiRoot, "05 Archive", "Processed Inbox");
  const atlasRoot = path.join(duumbiRoot, "01 Atlas (Knowledge Base)");
  const excluded = new Set([candidatePath]);

  return {
    generated_at: nowIso(),
    repository: "hgahub/duumbi",
    vault_repository: "hgahub/duumbi-vault",
    stage: "inbox-enrichment",
    policy: {
      max_notes_to_update: 1,
      writes_allowed: ["same Inbox note in duumbi-vault/main"],
      forbidden: [
        "Do not create GitHub issues.",
        "Do not create pull requests.",
        "Do not move or archive Inbox notes.",
        "Do not modify source code.",
        "Do not start implementation.",
      ],
    },
    candidate_note: {
      path: candidatePath,
      text: truncateText(candidateText, 9000),
    },
    active_vault_docs: readRelativeFiles(fs, path, vaultRoot, ACTIVE_VAULT_DOCS, warnings, 4500),
    source_code_context: readRelativeFiles(fs, path, workspace, SOURCE_CONTEXT_FILES, warnings, 3500),
    duplicate_context: {
      active_inbox_notes: readMarkdownNotes(fs, path, inboxRoot, vaultRoot, 8, 2000, excluded),
      processed_inbox_notes: readMarkdownNotes(fs, path, processedInboxRoot, vaultRoot, 8, 2000),
      atlas_notes: readMarkdownNotes(fs, path, atlasRoot, vaultRoot, 8, 2200),
    },
  };
}

export function buildDeepSeekMessages(contextPayload) {
  const schema = {
    title: "short developer-readable title",
    interpreted_intent: "what the user is asking for",
    classification: "idea | bug | feature | research | architecture | execution | knowledge | skill | unclear",
    business_value: "low | medium | high | critical",
    importance: "low | medium | high | critical",
    complexity: "low | medium | high | critical",
    developer_summary: "clear implementation-oriented summary for humans",
    uml_diagram_mermaid: "Mermaid diagram body only; no markdown fences",
    clarifications_answered: ["facts already clear from the input/context"],
    clarifications_open: ["questions a human may need to answer before implementation"],
    relevant_duumbi_context: ["vault or source paths and why they matter"],
    related_github_context: "known related GitHub state, or say triage should verify later",
    initial_routing_recommendation: "GitHub issue | GitHub Discussion | Dot/Map/Work | skill update | no action | needs clarification | duplicate",
    requested_follow_up: ["explicit requested next actions"],
    ai_agent_instructions: ["instructions for later agents that will create a GitHub issue"],
    scope_in: ["candidate scope"],
    scope_out: ["non-goals"],
    risks: ["risks or trade-offs"],
    facts: ["factual observations"],
    assumptions: ["assumptions to verify"],
    recommendations: ["routing or implementation recommendations"],
    status: "ready_for_triage | duplicate_candidate | needs_clarification | no_action_candidate",
  };

  return [
    {
      role: "system",
      content: [
        "You are the DUUMBI Inbox Enrichment Agent.",
        "Return exactly one valid JSON object and no markdown outside JSON.",
        "Write durable note content in English.",
        "Prepare exactly one raw Inbox note for later Stage 4 triage.",
        "Use the DUUMBI vault and source-code context to make the task understandable to a human developer.",
        "Include a concise UML-style Mermaid diagram body. Prefer classDiagram or sequenceDiagram when appropriate; use flowchart TD only when a task-flow diagram is clearer.",
        "Add practical AI-agent instructions for a later agent that will create a GitHub issue.",
        "Clearly separate facts, assumptions, risks, and open questions.",
        "Do not invent completed work, GitHub issue numbers, approvals, or implementation details that are not present in the supplied context.",
        "Do not create GitHub issues, specs, PRs, source changes, Atlas notes, or implementation work.",
        `Expected JSON schema example: ${JSON.stringify(schema)}`,
      ].join("\n"),
    },
    {
      role: "user",
      content: JSON.stringify(contextPayload, null, 2),
    },
  ];
}

function buildJsonRepairMessages({ baseMessages, invalidContent, parseError }) {
  return [
    {
      role: "system",
      content: [
        "Repair one DUUMBI Inbox enrichment response.",
        "Return exactly one valid strict JSON object and no markdown outside JSON.",
        "Do not include comments, trailing commas, prose outside JSON, or JSON5 syntax.",
        `Previous parser error: ${truncateText(parseError?.message || parseError, 300)}`,
      ].join("\n"),
    },
    ...baseMessages,
    {
      role: "user",
      content: [
        "Repair this malformed enrichment response into one valid JSON object:",
        truncateText(invalidContent, 5000),
      ].join("\n\n"),
    },
  ];
}

export function parseEnrichmentDecision(content) {
  const text = stripJsonFence(content);
  try {
    const parsed = JSON.parse(text);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("Enrichment response must be a JSON object.");
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
        // Fall through to the original parser error.
      }
    }
    throw new Error(`DeepSeek enrichment response was not valid JSON: ${firstError.message}`);
  }
}

export function validateEnrichmentDecision(decision) {
  const title = sanitizeTitle(decision?.title);
  const classification = normalizeEnum(decision?.classification, CLASSIFICATIONS, "unclear");
  const businessValue = normalizeEnum(decision?.business_value, ESTIMATE_LEVELS, "medium");
  const importance = normalizeEnum(decision?.importance, ESTIMATE_LEVELS, "medium");
  const complexity = normalizeEnum(decision?.complexity, ESTIMATE_LEVELS, "medium");
  const status = normalizeEnum(decision?.status, RESULT_STATUSES, "ready_for_triage");
  const developerSummary = truncateText(decision?.developer_summary, 4000);
  const interpretedIntent = truncateText(decision?.interpreted_intent, 2500);

  if (!developerSummary) {
    throw new Error("DeepSeek enrichment response is missing developer_summary.");
  }
  if (!interpretedIntent) {
    throw new Error("DeepSeek enrichment response is missing interpreted_intent.");
  }

  const normalized = {
    title,
    interpreted_intent: interpretedIntent,
    classification,
    business_value: businessValue,
    importance,
    complexity,
    developer_summary: developerSummary,
    uml_diagram_mermaid: stripMarkdownFence(decision?.uml_diagram_mermaid) || defaultDiagram(title),
    clarifications_answered: normalizeStringArray(decision?.clarifications_answered),
    clarifications_open: normalizeStringArray(decision?.clarifications_open),
    relevant_duumbi_context: normalizeStringArray(decision?.relevant_duumbi_context),
    related_github_context: truncateText(decision?.related_github_context, 2500)
      || "Not inspected; Stage 4 triage should verify GitHub state later.",
    initial_routing_recommendation: truncateText(decision?.initial_routing_recommendation, 1000)
      || "GitHub issue",
    requested_follow_up: normalizeStringArray(decision?.requested_follow_up),
    ai_agent_instructions: normalizeStringArray(decision?.ai_agent_instructions, 16, 1400),
    scope_in: normalizeStringArray(decision?.scope_in),
    scope_out: normalizeStringArray(decision?.scope_out),
    risks: normalizeStringArray(decision?.risks),
    facts: normalizeStringArray(decision?.facts),
    assumptions: normalizeStringArray(decision?.assumptions),
    recommendations: normalizeStringArray(decision?.recommendations),
    status,
  };
  normalized.obsidian_tags = buildObsidianTags(normalized);
  return normalized;
}

export function buildEnrichedNote({ originalText, decision, candidatePath, generatedAt = nowIso() }) {
  const statusLabel = decision.status.replace(/_/g, " ");
  const hashtagLine = decision.obsidian_tags.map((tag) => `#${tag}`).join(" ");

  return [
    "---",
    "tags:",
    ...decision.obsidian_tags.map((tag) => `  - ${tag}`),
    "duumbi_inbox_enrichment: processed",
    `duumbi_inbox_enrichment_generated_at: ${generatedAt}`,
    "---",
    "",
    `# ${decision.title}`,
    "",
    `${ENRICHMENT_MARKER_PREFIX} status=processed generated_at=${generatedAt} -->`,
    "",
    "## Source",
    "- Surface: Manual Obsidian edit",
    `- Vault path: ${candidatePath}`,
    "- Submitted by: unknown unless explicit in the raw input",
    "",
    "## Raw input",
    markdownBlockquote(originalText),
    "",
    "## Interpreted intent",
    "",
    decision.interpreted_intent,
    "",
    "## Developer summary",
    "",
    decision.developer_summary,
    "",
    "## UML overview",
    "",
    "```mermaid",
    decision.uml_diagram_mermaid,
    "```",
    "",
    "## Classification",
    `- Type: ${decision.classification}`,
    `- Business value: ${decision.business_value}`,
    `- Importance: ${decision.importance}`,
    `- Complexity: ${decision.complexity}`,
    "",
    "## Clarifications",
    "### Answered",
    markdownList(decision.clarifications_answered),
    "",
    "### Open",
    markdownList(decision.clarifications_open),
    "",
    "## Relevant DUUMBI context",
    markdownList(decision.relevant_duumbi_context),
    "",
    "## Related GitHub context",
    "",
    decision.related_github_context,
    "",
    "## Initial routing recommendation",
    "",
    decision.initial_routing_recommendation,
    "",
    "## Requested follow-up",
    markdownList(decision.requested_follow_up),
    "",
    "## AI agent instructions",
    markdownList(decision.ai_agent_instructions),
    "",
    "## Scope candidate",
    "### In",
    markdownList(decision.scope_in),
    "",
    "### Out",
    markdownList(decision.scope_out),
    "",
    "## Risks and trade-offs",
    markdownList(decision.risks),
    "",
    "## Obsidian tags",
    "",
    hashtagLine,
    "",
    "## Enrichment result",
    `- Date: ${generatedAt}`,
    `- Status: ${statusLabel}`,
    "- Canonical duplicate: none verified",
    "- Facts:",
    markdownList(decision.facts, "  - none"),
    "- Assumptions:",
    markdownList(decision.assumptions, "  - none"),
    "- Recommendations:",
    markdownList(decision.recommendations, "  - none"),
    "",
  ].join("\n");
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

function providerUsageFromDeepSeek(model, usage, latencyMs, requestCount = 1) {
  return {
    available: true,
    reason: "deepseek_api_call",
    provider: "deepseek",
    model,
    request_count: requestCount,
    prompt_tokens: Number.isFinite(Number(usage.prompt_tokens)) ? Number(usage.prompt_tokens) : null,
    completion_tokens: Number.isFinite(Number(usage.completion_tokens)) ? Number(usage.completion_tokens) : null,
    total_tokens: Number.isFinite(Number(usage.total_tokens)) ? Number(usage.total_tokens) : null,
    estimated_cost_usd: estimateDeepSeekCostUsd(model, usage),
    latency_ms: Number.isFinite(latencyMs) ? latencyMs : null,
    failure_count: 0,
  };
}

function combineDeepSeekUsage(responses) {
  const usage = {
    prompt_tokens: 0,
    completion_tokens: 0,
    total_tokens: 0,
  };
  let latencyMs = 0;
  let hasPrompt = false;
  let hasCompletion = false;
  let hasTotal = false;

  for (const response of responses) {
    const current = response?.usage || {};
    const promptTokens = Number(current.prompt_tokens);
    const completionTokens = Number(current.completion_tokens);
    const totalTokens = Number(current.total_tokens);
    if (Number.isFinite(promptTokens)) {
      usage.prompt_tokens += promptTokens;
      hasPrompt = true;
    }
    if (Number.isFinite(completionTokens)) {
      usage.completion_tokens += completionTokens;
      hasCompletion = true;
    }
    if (Number.isFinite(totalTokens)) {
      usage.total_tokens += totalTokens;
      hasTotal = true;
    }
    const currentLatency = Number(response?.latencyMs);
    if (Number.isFinite(currentLatency)) latencyMs += currentLatency;
  }

  return {
    usage: {
      prompt_tokens: hasPrompt ? usage.prompt_tokens : null,
      completion_tokens: hasCompletion ? usage.completion_tokens : null,
      total_tokens: hasTotal ? usage.total_tokens : null,
    },
    latencyMs: latencyMs || null,
  };
}

async function getParsedEnrichmentDecision({ fetchImpl, apiKey, model, messages, core, warnings }) {
  const responses = [];
  const firstResponse = await callDeepSeek({
    fetchImpl,
    apiKey,
    model,
    messages,
    maxTokens: 5000,
  });
  responses.push(firstResponse);

  try {
    return {
      parsedDecision: parseEnrichmentDecision(firstResponse.content),
      responses,
    };
  } catch (error) {
    const warning = `DeepSeek enrichment response was malformed; retrying strict JSON repair once: ${truncateText(error.message, 240)}`;
    warnings.push(warning);
    core?.warning?.(warning);
    const repairResponse = await callDeepSeek({
      fetchImpl,
      apiKey,
      model,
      messages: buildJsonRepairMessages({
        baseMessages: messages,
        invalidContent: firstResponse.content,
        parseError: error,
      }),
      maxTokens: 5000,
    });
    responses.push(repairResponse);
    return {
      parsedDecision: parseEnrichmentDecision(repairResponse.content),
      responses,
    };
  }
}

function defaultGit(args, options) {
  return execFileSync("git", args, {
    cwd: options.cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

export function commitVaultChanges({
  vaultRoot,
  relativePaths,
  git = defaultGit,
  requireWriteToken = false,
  env = process.env,
}) {
  const paths = relativePaths.filter(Boolean);
  if (paths.length === 0) {
    return { committed: false, commitSha: null };
  }
  if (requireWriteToken && !normalizeText(env.GH_PROJECT_PAT)) {
    throw new Error("GH_PROJECT_PAT is required to commit directly to duumbi-vault/main.");
  }

  git(["config", "user.name", "duumbi-inbox-enrichment[bot]"], { cwd: vaultRoot });
  git(["config", "user.email", "duumbi-inbox-enrichment[bot]@users.noreply.github.com"], { cwd: vaultRoot });
  git(["add", "--", ...paths], { cwd: vaultRoot });
  const status = git(["status", "--porcelain", "--", ...paths], { cwd: vaultRoot });
  if (!normalizeText(status)) {
    return { committed: false, commitSha: null };
  }

  git(["commit", "-m", "chore: enrich DUUMBI inbox note"], { cwd: vaultRoot });
  const commitSha = normalizeText(git(["rev-parse", "HEAD"], { cwd: vaultRoot }));
  git(["push", "origin", "HEAD:main"], { cwd: vaultRoot });
  return { committed: true, commitSha };
}

async function postSlack({ fetchImpl, env, context, candidatePath, commitSha, workflowUrl, warnings }) {
  const token = env.SLACK_BOT_TOKEN;
  const channel = env.DUUMBI_AGENT_DISPATCH_CHANNEL_ID || env.SLACK_REVIEW_CHANNEL_ID;
  if (!token || !channel) {
    warnings.push("slack_notification_not_configured");
    return "not_configured";
  }

  try {
    const response = await fetchImpl("https://slack.com/api/chat.postMessage", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json; charset=utf-8",
      },
      body: JSON.stringify({
        channel,
        text: [
          "*DUUMBI Inbox Enrichment Complete*",
          `*Repository:* ${context.repo.owner}/${context.repo.repo}`,
          `*Vault note:* ${candidatePath}`,
          `*Vault commit:* ${commitSha || "committed"}`,
          `*Workflow run:* ${workflowUrl}`,
        ].join("\n"),
      }),
    });
    const text = await response.text();
    const body = text ? JSON.parse(text) : {};
    if (!response.ok || !body.ok) {
      warnings.push(`slack_notification_failed:${body.error || response.status}`);
      return "failed";
    }
    return "posted";
  } catch (error) {
    warnings.push(`slack_notification_failed:${truncateText(error.message || error, 160)}`);
    return "failed";
  }
}

function buildWorkflowMetrics({
  context,
  conclusion,
  decision,
  counts,
  providerUsage,
  warnings,
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
      issue_number: null,
      pr_number: null,
      stage: "inbox-enrichment",
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

function writeMetrics(fs, path, workspace, metricsPath, metrics) {
  const outputPath = path.resolve(workspace, metricsPath || "duumbi-workflow-metrics.json");
  fs.writeFileSync(outputPath, `${JSON.stringify(metrics, null, 2)}\n`);
}

async function writeSummary(summary, result) {
  if (!summary) return;
  const table = [
    [{ data: "Field", header: true }, { data: "Value", header: true }],
    ["Target", result.targetPath || "scheduled sweep"],
    ["Scan limit", String(result.scanLimit)],
    ["Inbox notes inspected", String(result.inspectedCount)],
    ["Candidate note", result.candidatePath || "none"],
    ["Updated note", result.updatedPath || "none"],
    ["Vault commit", result.commitSha || "none"],
    ["Slack dispatch", result.slackNotification || "not_needed"],
  ];
  await summary.addHeading("DUUMBI Inbox Enrichment", 2).addTable(table).write();
}

export async function runInboxEnrichment({
  env = process.env,
  context,
  core = null,
  summary = null,
  fetchImpl = fetch,
  fs = fsModule,
  path = pathModule,
  workspace = process.cwd(),
  git = defaultGit,
}) {
  const generatedAt = nowIso();
  const warnings = [];
  const inputs = context?.payload?.inputs || {};
  const targetPath = String(inputs.target_path || "").trim();
  const scanLimit = parsePositiveInteger(inputs.scan_limit || inputs.batch_size, DEFAULT_SCAN_LIMIT);
  const vaultRoot = path.resolve(workspace, "duumbi-vault");
  const inboxRoot = path.join(vaultRoot, "Duumbi", "00 Inbox (ToProcess)");
  const workflowUrl = `${context.serverUrl || "https://github.com"}/${context.repo.owner}/${context.repo.repo}/actions/runs/${context.runId}`;
  let providerUsage = providerUsageNotCalled("no_candidate_note");
  const result = {
    targetPath,
    scanLimit,
    inspectedCount: 0,
    candidatePath: null,
    updatedPath: null,
    commitSha: null,
    slackNotification: "not_needed",
  };

  try {
    const { candidatePaths, inspectedCount } = collectCandidatePaths({
      fs,
      path,
      vaultRoot,
      inboxRoot,
      targetPath,
      scanLimit,
    });
    result.inspectedCount = inspectedCount;

    if (candidatePaths.length === 0) {
      const metrics = buildWorkflowMetrics({
        context,
        conclusion: "success",
        decision: "no_candidate_note",
        counts: {
          issues_considered: inspectedCount,
          issues_queued: 0,
          slack_notifications_attempted: 0,
          artifact_links_found: 0,
          artifact_links_missing: null,
        },
        providerUsage,
        warnings: ["slack_notification_not_needed", ...warnings],
        generatedAt,
      });
      writeMetrics(fs, path, workspace, env.DUUMBI_METRICS_PATH, metrics);
      await writeSummary(summary, result);
      return { ...result, changed: false, decision: "no_candidate_note" };
    }

    const candidatePath = candidatePaths[0];
    result.candidatePath = candidatePath;
    const apiKey = env.DEEPSEEK_API_KEY;
    if (!apiKey) {
      throw new Error("DEEPSEEK_API_KEY is not configured; failing closed before Inbox enrichment.");
    }

    const contextPayload = buildEnrichmentContext({
      fs,
      path,
      workspace,
      vaultRoot,
      candidatePath,
      warnings,
    });
    const messages = buildDeepSeekMessages(contextPayload);
    const { parsedDecision, responses } = await getParsedEnrichmentDecision({
      fetchImpl,
      apiKey,
      model: env.DEEPSEEK_MODEL || DEFAULT_DEEPSEEK_MODEL,
      messages,
      core,
      warnings,
    });
    const lastResponse = responses.at(-1);
    const combinedUsage = combineDeepSeekUsage(responses);
    providerUsage = providerUsageFromDeepSeek(
      lastResponse.model,
      combinedUsage.usage,
      combinedUsage.latencyMs,
      responses.length,
    );
    const decision = validateEnrichmentDecision(parsedDecision);
    const candidateAbsolute = path.join(vaultRoot, candidatePath);
    const originalText = fs.readFileSync(candidateAbsolute, "utf8");
    const nextText = buildEnrichedNote({
      originalText,
      decision,
      candidatePath,
      generatedAt,
    });
    fs.writeFileSync(candidateAbsolute, nextText);
    result.updatedPath = candidatePath;

    const commitResult = commitVaultChanges({
      vaultRoot,
      relativePaths: [candidatePath],
      git,
      requireWriteToken: true,
      env,
    });
    result.commitSha = commitResult.commitSha;

    if (commitResult.committed) {
      result.slackNotification = await postSlack({
        fetchImpl,
        env,
        context,
        candidatePath,
        commitSha: commitResult.commitSha,
        workflowUrl,
        warnings,
      });
    }

    const metrics = buildWorkflowMetrics({
      context,
      conclusion: "success",
      decision: commitResult.committed ? "vault_note_enriched" : "no_vault_diff",
      counts: {
        issues_considered: result.inspectedCount,
        issues_queued: commitResult.committed ? 1 : 0,
        slack_notifications_attempted: commitResult.committed && result.slackNotification !== "not_configured" ? 1 : 0,
        artifact_links_found: commitResult.committed ? 1 : 0,
        artifact_links_missing: null,
      },
      providerUsage,
      warnings,
      generatedAt,
    });
    writeMetrics(fs, path, workspace, env.DUUMBI_METRICS_PATH, metrics);
    await writeSummary(summary, result);
    return { ...result, changed: commitResult.committed, decision: metrics.correlation.decision };
  } catch (error) {
    const message = error?.message || String(error);
    warnings.push(`inbox_enrichment_failed:${truncateText(message, 240)}`);
    const metrics = buildWorkflowMetrics({
      context,
      conclusion: "failure",
      decision: "enrichment_failed",
      counts: {
        issues_considered: result.inspectedCount,
        issues_queued: 0,
        slack_notifications_attempted: 0,
        artifact_links_found: 0,
        artifact_links_missing: null,
      },
      providerUsage,
      warnings,
      generatedAt,
    });
    writeMetrics(fs, path, workspace, env.DUUMBI_METRICS_PATH, metrics);
    await writeSummary(summary, result);
    core?.setFailed?.(message);
    return { ...result, changed: false, decision: "enrichment_failed", error: message };
  }
}
