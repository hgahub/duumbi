import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  ENRICHMENT_MARKER_PREFIX,
  buildEnrichmentContext,
  buildObsidianTags,
  collectCandidatePaths,
  isEnrichmentCandidateText,
  runInboxEnrichment,
} from "./inbox-enrichment-dispatch.mjs";

function response(body, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: async () => JSON.stringify(body),
  };
}

function makeContext(inputs = {}) {
  return {
    workflow: "Inbox Enrichment Dispatch",
    runId: 12345,
    eventName: "workflow_dispatch",
    actor: "tester",
    ref: "refs/heads/main",
    sha: "abc123",
    serverUrl: "https://github.com",
    repo: { owner: "hgahub", repo: "duumbi" },
    payload: { inputs },
  };
}

function makeCore() {
  return {
    failed: null,
    warnings: [],
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

function makeWorkspace({ processed = false, secondRaw = false } = {}) {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "duumbi-inbox-enrichment-"));
  const vaultRoot = path.join(workspace, "duumbi-vault");
  const inboxRoot = path.join(vaultRoot, "Duumbi", "00 Inbox (ToProcess)");
  const processedRoot = path.join(vaultRoot, "Duumbi", "05 Archive", "Processed Inbox");
  const atlasRoot = path.join(vaultRoot, "Duumbi", "01 Atlas (Knowledge Base)", "Works (Developed Materials)");
  fs.mkdirSync(inboxRoot, { recursive: true });
  fs.mkdirSync(processedRoot, { recursive: true });
  fs.mkdirSync(atlasRoot, { recursive: true });

  const candidateText = processed
    ? [
        "---",
        "tags:",
        "  - duumbi/status/processed",
        "---",
        "# Already processed",
        `${ENRICHMENT_MARKER_PREFIX} status=processed -->`,
      ].join("\n")
    : "# Raw idea\n\nMake the provider setup easier for new users.\n";

  fs.writeFileSync(path.join(inboxRoot, "candidate.md"), candidateText);
  if (secondRaw) {
    fs.writeFileSync(path.join(inboxRoot, "second.md"), "# Second raw idea\n\nAdd another thing.\n");
  }
  fs.writeFileSync(path.join(processedRoot, "old.md"), "# Old processed note\n\nProvider setup background.\n");
  fs.writeFileSync(path.join(atlasRoot, "DUUMBI - PRD.md"), "# PRD\n\nDUUMBI is an AI-first semantic graph compiler.\n");
  fs.writeFileSync(path.join(atlasRoot, "DUUMBI - Glossary.md"), "# Glossary\n\nGraph IR: central semantic graph.\n");

  fs.mkdirSync(path.join(vaultRoot, "Duumbi", "01 Atlas (Knowledge Base)", "Maps (Overviews)"), { recursive: true });
  fs.writeFileSync(
    path.join(vaultRoot, "Duumbi", "01 Atlas (Knowledge Base)", "Maps (Overviews)", "DUUMBI Agentic Development Map.md"),
    "# Agentic Development Map\n\nInbox leads to triage.",
  );
  fs.writeFileSync(
    path.join(atlasRoot, "DUUMBI - Agentic Development Runbook.md"),
    "# Runbook\n\nStage 4 creates GitHub issues after enrichment.",
  );
  fs.writeFileSync(path.join(vaultRoot, "Duumbi", "How to use.md"), "# How to use\n\nUse Inbox for raw capture.");

  fs.mkdirSync(path.join(workspace, "docs"), { recursive: true });
  fs.mkdirSync(path.join(workspace, "src", "graph"), { recursive: true });
  fs.mkdirSync(path.join(workspace, "src", "compiler"), { recursive: true });
  fs.mkdirSync(path.join(workspace, "src", "agents"), { recursive: true });
  fs.mkdirSync(path.join(workspace, "src", "mcp"), { recursive: true });
  fs.mkdirSync(path.join(workspace, "crates", "duumbi-studio", "src"), { recursive: true });
  fs.writeFileSync(path.join(workspace, "AGENTS.md"), "# Agents\n\nKeep Query mode read-only.");
  fs.writeFileSync(path.join(workspace, "docs", "architecture.md"), "# Architecture\n\nGraph IR is central.");
  fs.writeFileSync(path.join(workspace, "docs", "coding-conventions.md"), "# Coding\n\nNo unwrap in library code.");
  fs.writeFileSync(path.join(workspace, "Cargo.toml"), "[package]\nname = \"duumbi\"\n");
  fs.writeFileSync(path.join(workspace, "src", "types.rs"), "pub enum DuumbiType { I64 }\n");
  fs.writeFileSync(path.join(workspace, "src", "graph", "mod.rs"), "pub mod graph {}\n");
  fs.writeFileSync(path.join(workspace, "src", "compiler", "mod.rs"), "pub mod compiler {}\n");
  fs.writeFileSync(path.join(workspace, "src", "agents", "mod.rs"), "pub mod agents {}\n");
  fs.writeFileSync(path.join(workspace, "src", "mcp", "mod.rs"), "pub mod mcp {}\n");
  fs.writeFileSync(path.join(workspace, "crates", "duumbi-studio", "src", "lib.rs"), "pub mod app {}\n");

  return { workspace, inboxRoot };
}

function makeFetch(decision = {}) {
  const calls = [];
  const fetchImpl = async (url, options = {}) => {
    const body = options.body ? JSON.parse(options.body) : {};
    calls.push({ url, method: options.method || "GET", body });

    if (url === "https://api.deepseek.com/chat/completions") {
      return response({
        model: "deepseek-v4-pro",
        choices: [{
          message: {
            content: JSON.stringify({
              title: "Simplify provider setup",
              interpreted_intent: "Prepare a task for making provider setup easier.",
              classification: "feature",
              business_value: "high",
              importance: "high",
              complexity: "medium",
              developer_summary: "Create a clear issue candidate for improving the provider setup flow.",
              uml_diagram_mermaid: [
                "flowchart TD",
                "  User[User] --> Setup[Provider setup]",
                "  Setup --> Ready[Configured provider]",
              ].join("\n"),
              clarifications_answered: ["The request concerns provider setup UX."],
              clarifications_open: ["Which provider should be the first manual test path?"],
              relevant_duumbi_context: ["docs/architecture.md explains the graph-first architecture."],
              related_github_context: "Not inspected; Stage 4 triage should verify GitHub state later.",
              initial_routing_recommendation: "GitHub issue",
              requested_follow_up: ["Create an issue during Stage 4 triage."],
              ai_agent_instructions: ["Create one GitHub issue; do not start implementation."],
              scope_in: ["Provider setup issue preparation."],
              scope_out: ["No source code changes during enrichment."],
              risks: ["The request may overlap with existing provider UX work."],
              facts: ["The raw Inbox note requests easier provider setup."],
              assumptions: ["The target user is a new DUUMBI user."],
              recommendations: ["Route to Stage 4 triage as a feature candidate."],
              status: "ready_for_triage",
              ...decision,
            }),
          },
        }],
        usage: { prompt_tokens: 1200, completion_tokens: 420, total_tokens: 1620 },
      });
    }

    if (url === "https://slack.com/api/chat.postMessage") {
      return response({ ok: true, ts: "123.456" });
    }

    throw new Error(`Unexpected fetch: ${url}`);
  };
  return { fetchImpl, calls };
}

function makeGit() {
  const calls = [];
  const git = (args, options = {}) => {
    calls.push({ args, cwd: options.cwd });
    if (args[0] === "status") return " M Duumbi/00 Inbox (ToProcess)/candidate.md\n";
    if (args[0] === "rev-parse") return "feedface1234567890\n";
    return "";
  };
  return { git, calls };
}

test("candidate detection ignores processed marker and processed tag", () => {
  assert.equal(isEnrichmentCandidateText("# Raw\n\nPlease build something."), true);
  assert.equal(isEnrichmentCandidateText(`${ENRICHMENT_MARKER_PREFIX} status=processed -->`), false);
  assert.equal(isEnrichmentCandidateText("---\ntags:\n  - duumbi/status/processed\n---\n# Done"), false);
});

test("collectCandidatePaths selects at most one unprocessed note", () => {
  const { workspace, inboxRoot } = makeWorkspace({ secondRaw: true });
  const result = collectCandidatePaths({
    vaultRoot: path.join(workspace, "duumbi-vault"),
    inboxRoot,
  });
  assert.deepEqual(result.candidatePaths, ["Duumbi/00 Inbox (ToProcess)/candidate.md"]);
});

test("buildObsidianTags emits Obsidian-compatible classification tags", () => {
  assert.deepEqual(buildObsidianTags({
    classification: "feature",
    business_value: "high",
    importance: "critical",
    complexity: "medium",
  }), [
    "duumbi/inbox/enriched",
    "duumbi/status/processed",
    "duumbi/classification/feature",
    "duumbi/value/high",
    "duumbi/importance/critical",
    "duumbi/complexity/medium",
  ]);
});

test("buildEnrichmentContext derives repository names from context", () => {
  const { workspace } = makeWorkspace();
  const contextPayload = buildEnrichmentContext({
    workspace,
    vaultRoot: path.join(workspace, "duumbi-vault"),
    candidatePath: "Duumbi/00 Inbox (ToProcess)/candidate.md",
    context: {
      repo: { owner: "example-owner", repo: "example-repo" },
    },
  });

  assert.equal(contextPayload.repository, "example-owner/example-repo");
  assert.equal(contextPayload.vault_repository, "example-owner/duumbi-vault");
});

test("runInboxEnrichment skips Slack and DeepSeek when all notes are processed", async () => {
  const { workspace } = makeWorkspace({ processed: true });
  const { fetchImpl, calls } = makeFetch();
  const { git, calls: gitCalls } = makeGit();
  const core = makeCore();

  const result = await runInboxEnrichment({
    env: {
      DEEPSEEK_API_KEY: "deepseek",
      GH_PROJECT_PAT: "pat",
      SLACK_BOT_TOKEN: "slack",
      SLACK_REVIEW_CHANNEL_ID: "C123",
      DUUMBI_METRICS_PATH: "metrics.json",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
    git,
  });

  assert.equal(result.changed, false);
  assert.equal(result.decision, "no_candidate_note");
  assert.equal(calls.length, 0);
  assert.equal(gitCalls.length, 0);
  assert.equal(core.failed, null);
  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.counts.slack_notifications_attempted, 0);
  assert.equal(metrics.provider_usage.reason, "no_candidate_note");
});

test("runInboxEnrichment fails before DeepSeek when vault write token is missing", async () => {
  const { workspace } = makeWorkspace();
  const { fetchImpl, calls } = makeFetch();
  const { git, calls: gitCalls } = makeGit();
  const core = makeCore();

  const result = await runInboxEnrichment({
    env: {
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
    git,
  });

  assert.equal(result.changed, false);
  assert.equal(result.decision, "enrichment_failed");
  assert.match(core.failed, /GH_PROJECT_PAT is required/);
  assert.equal(calls.some((call) => call.url === "https://api.deepseek.com/chat/completions"), false);
  assert.equal(gitCalls.length, 0);

  const original = fs.readFileSync(
    path.join(workspace, "duumbi-vault", "Duumbi", "00 Inbox (ToProcess)", "candidate.md"),
    "utf8",
  );
  assert.match(original, /# Raw idea/);
  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.correlation.decision, "enrichment_failed");
  assert.equal(metrics.counts.slack_notifications_attempted, 0);
  assert.equal(metrics.provider_usage.reason, "no_candidate_note");
});

test("runInboxEnrichment enriches one note, commits to vault, and posts Slack", async () => {
  const { workspace } = makeWorkspace({ secondRaw: true });
  const { fetchImpl, calls } = makeFetch();
  const { git, calls: gitCalls } = makeGit();
  const core = makeCore();

  const result = await runInboxEnrichment({
    env: {
      DEEPSEEK_API_KEY: "deepseek",
      DEEPSEEK_MODEL: "deepseek-v4-pro",
      GH_PROJECT_PAT: "pat",
      SLACK_BOT_TOKEN: "slack",
      SLACK_REVIEW_CHANNEL_ID: "C123",
      DUUMBI_METRICS_PATH: "metrics.json",
      GITHUB_RUN_ATTEMPT: "7",
    },
    context: makeContext(),
    core,
    summary: makeSummary(),
    fetchImpl,
    workspace,
    git,
  });

  assert.equal(result.changed, true);
  assert.equal(result.updatedPath, "Duumbi/00 Inbox (ToProcess)/candidate.md");
  assert.equal(result.commitSha, "feedface1234567890");
  assert.equal(calls.filter((call) => call.url === "https://api.deepseek.com/chat/completions").length, 1);
  assert.equal(calls.filter((call) => call.url === "https://slack.com/api/chat.postMessage").length, 1);
  assert.ok(gitCalls.some((call) => call.args[0] === "commit"));
  assert.ok(gitCalls.some((call) => call.args[0] === "push"));

  const updated = fs.readFileSync(
    path.join(workspace, "duumbi-vault", "Duumbi", "00 Inbox (ToProcess)", "candidate.md"),
    "utf8",
  );
  const untouched = fs.readFileSync(
    path.join(workspace, "duumbi-vault", "Duumbi", "00 Inbox (ToProcess)", "second.md"),
    "utf8",
  );
  assert.match(updated, /duumbi-inbox-enrichment:v1/);
  assert.match(updated, /duumbi\/status\/processed/);
  assert.match(updated, /## Developer summary/);
  assert.match(updated, /## UML overview/);
  assert.match(updated, /```mermaid/);
  assert.match(updated, /## AI agent instructions/);
  assert.match(untouched, /# Second raw idea/);

  const metrics = JSON.parse(fs.readFileSync(path.join(workspace, "metrics.json"), "utf8"));
  assert.equal(metrics.correlation.decision, "vault_note_enriched");
  assert.equal(metrics.workflow.run_attempt, 7);
  assert.equal(metrics.counts.slack_notifications_attempted, 1);
  assert.equal(metrics.provider_usage.provider, "deepseek");
});
