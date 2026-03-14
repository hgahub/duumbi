---
tags:
  - project/duumbi
  - milestone/phase-12
status: planned
github_milestone: ~
updated: 2026-03-14
---
# Phase 12 — Dynamic Agent System & MCP ⏳

> **Kill Criterion:** (1) An external MCP client (e.g., Claude Desktop) successfully calls `graph.query` and `graph.mutate` tools, modifying the graph. (2) A task triggers dynamic agent assembly — the system creates the right agent team for the job without hardcoded agent types. (3) Agent knowledge persists in JSON-LD and improves over subsequent runs. (4) Two Coder agents execute in parallel on different modules, and their patches merge without conflict. (5) Total LLM cost for a 5-task intent stays within configurable budget.
> **Status:** ⏳ Planned — starts after Phase 10
> **Estimated duration:** 10–12 weeks (solo developer)

← Back: [[DUUMBI Roadmap Map]]

---

## Summary

This phase transforms DUUMBI from a tool with a single AI integration point into a **dynamic multi-agent platform**. Instead of hardcoding 6 fixed agents, the system dynamically assembles agent teams based on the task at hand.

Each agent's knowledge — system prompts, successful patterns, failure modes, preferred strategies — is stored as a JSON-LD knowledge graph. The user sees Markdown; the system sees structured, queryable data. This eliminates the need for users to manually maintain skills, hooks, rules files, or any other context artifacts that current AI-assisted development tools require.

**This is DUUMBI's answer to CLAUDE.md, .cursorrules, and Copilot instructions — but automated and self-improving.**

---

## Tasks

### M12-MCP: MCP Server Implementation
- [ ] `rmcp` crate integration — DUUMBI CLI as MCP server
- [ ] Core MCP tools:
  - [ ] `graph.query` — graph traversal (node search, neighbors, paths)
  - [ ] `graph.mutate` — graph modification (patch application)
  - [ ] `graph.validate` — schema validation execution
  - [ ] `graph.describe` — pseudo-code generation
- [ ] Build MCP tools:
  - [ ] `build.compile` — trigger compilation
  - [ ] `build.run` — execute compiled binary
- [ ] Registry MCP tools:
  - [ ] `deps.search` — registry search
  - [ ] `deps.install` — dependency installation
- [ ] Intent MCP tools:
  - [ ] `intent.create` — create intent from natural language
  - [ ] `intent.execute` — execute intent pipeline
- [ ] Backward compatibility: existing direct API calls continue to work
- [ ] MCP transport: stdio (CLI) + SSE (Studio integration)
- [ ] Studio MCP integration: the axum server (port 8421) acts as both web server AND MCP SSE endpoint. Single process, two roles — no second server needed. SSE endpoint at `/mcp/sse`, web UI at all other routes.

---

### M12-TASK-ANALYSIS: Task Analysis Engine (Rule-Based)

The engine that examines an incoming task and determines which agents to spawn. **Fully rule-based, deterministic, no LLM call needed for the analysis itself.**

#### Task Dimensions

Every incoming task is scored on 4 dimensions:

| Dimension | Scoring Method | Values |
|---|---|---|
| **Complexity** | Node count estimate from intent spec | `simple` (1 function), `moderate` (2-5 functions), `complex` (6+ functions or multi-module) |
| **Type** | Pattern match on intent spec fields + CLI context | `create`, `modify`, `test`, `refactor`, `fix` |
| **Scope** | Module count from intent spec `modules.create` + `modules.modify` | `single-module`, `multi-module`, `cross-project` |
| **Risk** | Heuristic: does it touch main? Modify exports? Change types? | `low`, `medium`, `high` |

#### Scoring Rules (Deterministic)

```
Input: IntentSpec or CLI command context
Output: TaskProfile { complexity, type, scope, risk }

Rules:
  IF modules.create.len() == 0 AND modules.modify.len() == 1:
    scope = single-module
  ELIF modules.create.len() + modules.modify.len() > 1:
    scope = multi-module

  IF test_cases.len() > 0 AND modules.create.len() == 0:
    type = test
  ELIF intent contains "refactor" or "restructure" or "split":
    type = refactor
  ELIF error_context is present (retry after E0xx):
    type = fix
  ELIF modules.create.len() > 0:
    type = create
  ELSE:
    type = modify

  IF total estimated functions <= 1:
    complexity = simple
  ELIF total estimated functions <= 5:
    complexity = moderate
  ELSE:
    complexity = complex

  IF modules.modify contains "app/main":
    risk += 1
  IF any modified module has external dependents (backlink check):
    risk += 1
  IF task changes function signatures (return types, params):
    risk += 1
  risk = low (0), medium (1), high (2+)
```

#### Agent Team Assembly Rules

The TaskProfile maps to an agent team via deterministic rules:

| TaskProfile | Agent Team | Execution Strategy |
|---|---|---|
| `simple` + `create` + `single-module` + `low` | 1× Coder | Sequential |
| `simple` + `modify` + `single-module` + `low` | 1× Coder | Sequential |
| `simple` + `test` + `*` + `*` | 1× Tester | Sequential |
| `moderate` + `create` + `single-module` + `*` | 1× Planner → 1× Coder → 1× Tester | Pipeline |
| `moderate` + `create` + `multi-module` + `*` | 1× Planner → N× Coder (parallel) → 1× Tester | Parallel |
| `moderate` + `modify` + `*` + `medium/high` | 1× Planner → 1× Coder → 1× Reviewer → 1× Tester | Pipeline |
| `complex` + `*` + `multi-module` + `*` | 1× Planner → N× Coder (parallel) → 1× Reviewer → 1× Tester | Parallel |
| `*` + `refactor` + `*` + `*` | 1× Planner → 1× Coder → 1× Reviewer → 1× Tester | Pipeline (always review) |
| `*` + `fix` + `*` + `*` | 1× Coder (with error context) | Sequential (fast path) |

**N× Coder:** one Coder agent per module in `modules.create`. Each gets its own module as target.

**Fallback:** If task analysis fails or produces contradictory signals → fall back to single Coder agent (current `duumbi add` behavior). Log the failure to knowledge graph for future analysis.

#### Implementation
- [ ] `TaskAnalyzer` struct: `analyze(intent_spec: &IntentSpec, graph: &StableGraph) -> TaskProfile`
- [ ] `TeamAssembler` struct: `assemble(profile: &TaskProfile, templates: &[AgentTemplate]) -> AgentTeam`
- [ ] Task dimension scoring: 4 rule-based classifiers
- [ ] Agent team mapping: lookup table from TaskProfile to team composition
- [ ] Fallback logic: graceful degradation to single-agent mode
- [ ] Logging: every analysis decision logged to knowledge graph (for future tuning)

---

### M12-MERGE: Concurrent Graph Merge Algorithm

When multiple Coder agents work in parallel, each produces a `GraphPatch`. These patches must be merged into the main graph. The merge algorithm is **deterministic and conflict-aware**.

#### Architecture

```
Coder Agent 1 ──→ GraphPatch A (module: smtp/server)
Coder Agent 2 ──→ GraphPatch B (module: pop3/server)
                         │
                         ▼
                ┌────────────────────┐
                │   Merge Engine      │
                │                    │
                │ 1. Conflict check  │
                │ 2. Dependency check│
                │ 3. Apply or reject │
                └────────────────────┘
                         │
                         ▼
                   Merged Graph
```

#### Merge Strategy: Module-Scoped Last-Writer-Wins with Conflict Detection

**Core principle:** Parallel agents work on **different modules**. The Team Assembly rules guarantee this — each Coder gets a different module from `modules.create`. Therefore, the merge is module-scoped, not node-scoped.

**Merge rules:**

1. **No overlap (common case):** Patch A touches module X, Patch B touches module Y → both apply cleanly. Order doesn't matter (commutative at module scope).

2. **Shared dependency:** Both patches add `duumbi:imports` to the same external module → both imports are merged (set union of import lists). No conflict.

3. **Main.jsonld conflict:** Both patches modify `app/main` (e.g., adding function calls) → **sequential merge**:
   - Sort patches by module name (deterministic order)
   - Apply first patch to main
   - Re-validate
   - Apply second patch to main
   - Re-validate
   - If either validation fails → reject that patch, flag for retry

4. **Node ID collision:** Two patches create nodes with the same `@id` → **reject both**, flag for Planner re-decomposition. This should not happen if the Planner assigns distinct module namespaces.

5. **Cross-module reference:** Patch A creates a function, Patch B calls that function → **dependency ordering**:
   - Detect cross-references between patches
   - Apply the "provider" patch first (the one that creates the function)
   - Apply the "consumer" patch second
   - If circular dependency → reject, flag for Planner re-decomposition

#### Implementation
- [ ] `MergeEngine` struct: `merge(patches: Vec<GraphPatch>, graph: &mut StableGraph) -> MergeResult`
- [ ] `MergeResult`: `{ applied: Vec<PatchId>, rejected: Vec<(PatchId, ConflictReason)> }`
- [ ] Conflict detection: module scope overlap check, node ID collision check, cross-reference analysis
- [ ] Dependency ordering: topological sort of patches based on cross-references
- [ ] Main.jsonld sequential merge with re-validation between patches
- [ ] All merge decisions logged to knowledge graph

---

### M12-COST: LLM Cost Control

Dynamic agent spawning can lead to unbounded LLM costs. This section defines hard limits and monitoring.

#### Budget Architecture

```
config.toml
├── [permissions]
│   ├── max-tokens = 8192        # per single LLM call (from Phase 10)
│   └── max-retries = 5          # per single task (from Phase 10)
├── [cost]
│   ├── budget-per-intent = 50000    # max total tokens for one intent execution
│   ├── budget-per-session = 200000  # max total tokens per CLI session
│   ├── max-parallel-agents = 3      # max concurrent LLM calls
│   ├── circuit-breaker-failures = 5 # consecutive failures → stop spawning
│   └── alert-threshold-pct = 80     # warn at 80% of budget
```

#### Cost Tracking (Per Intent Execution)

```
Intent: "Build SMTP server"
├── Planner:    1,200 tokens  (1 call)
├── Coder A:    3,400 tokens  (2 calls: generate + 1 retry)
├── Coder B:    2,800 tokens  (1 call)
├── Reviewer:   1,500 tokens  (1 call)
├── Tester:     2,100 tokens  (1 call)
├── TOTAL:     11,000 tokens  ← tracked against budget-per-intent
└── BUDGET:    50,000 tokens  ← remaining: 39,000
```

#### Circuit Breaker

If N consecutive agent executions fail (all retries exhausted), the orchestrator stops spawning new agents:

```
State: CLOSED (normal) → OPEN (broken) → HALF-OPEN (testing)

CLOSED: agents spawn normally
  → N consecutive failures → transition to OPEN

OPEN: no new agents spawned, existing agents complete
  → User manually resets (/permissions set circuit-breaker reset)
  → OR: after configurable cooldown (default 5 min) → HALF-OPEN

HALF-OPEN: one agent allowed to execute
  → Success → back to CLOSED
  → Failure → back to OPEN
```

#### Rate Limiting

- Maximum `max-parallel-agents` concurrent LLM calls (default 3)
- Agent spawn queue: if limit reached, new agents wait in FIFO queue
- Timeout: if an agent waits >60 seconds in queue → cancel and log

#### Implementation
- [ ] `CostTracker` struct: per-intent and per-session token accounting
- [ ] Budget enforcement: before every LLM call, check remaining budget → reject if exceeded
- [ ] `CircuitBreaker` struct: state machine (Closed/Open/HalfOpen) with configurable thresholds
- [ ] Rate limiter: `tokio::sync::Semaphore` with `max-parallel-agents` permits
- [ ] Agent spawn queue: `tokio::sync::mpsc` bounded channel
- [ ] Alert at threshold: when 80% of budget consumed → CLI warning + Studio notification
- [ ] Budget exceeded: graceful stop → report what was completed, what was skipped
- [ ] `/stats` integration: cost data flows to Phase 10 UsageStats model

---

### M12-DYNAGENT: Dynamic Agent Assembly
- [ ] Agent template system: agent definitions stored as JSON-LD nodes
  - `duumbi:AgentTemplate` type with fields: name, role, systemPrompt, tools, specialization, tokenBudget
  - Templates are graph nodes — queryable, linkable, versionable
  - **Seed templates:** 5 built-in templates (Planner, Coder, Reviewer, Tester, Repair) installed by `duumbi init`
- [ ] Agent instantiation: template + task context (from M10-CONTEXT) + relevant knowledge → agent instance
- [ ] Agent lifecycle: spawn → execute → report → learn → terminate
- [ ] No hardcoded agent types: new specializations = new JSON-LD template nodes
- [ ] User-editable agent definitions: Markdown view via Phase 10 M10-MARKDOWN layer

### M12-AGENTKNOW: Agent Knowledge Persistence
- [ ] Per-agent knowledge graph: each template accumulates knowledge over time
  - Successful strategies: `duumbi:Strategy` nodes — "when task looks like X, approach Y works"
  - Failure patterns: `duumbi:FailurePattern` nodes — "approach Z fails for recursion tasks"
  - Prompt refinements: evolved system prompts based on outcomes
- [ ] Knowledge stored as JSON-LD: `duumbi:AgentKnowledge`, `duumbi:Strategy`, `duumbi:FailurePattern`
- [ ] Markdown view: users can read and edit agent knowledge
- [ ] Knowledge transfer: on instantiation, inject relevant knowledge into agent context (uses M10-CONTEXT algorithm)
- [ ] Knowledge pruning strategy:
  - **Score-based:** each Strategy/FailurePattern has `successCount` and `failCount` fields
  - Pruning threshold: if `failCount / (successCount + failCount) > 0.7` AND `totalAttempts > 10` → mark as `deprecated`
  - Deprecated knowledge is never auto-injected, but remains queryable (audit trail)
  - User can override: manually un-deprecate in Markdown view
  - **No automatic deletion.** Only deprecation. Deletion is a manual user action.

### M12-ORCHESTR: Orchestration Engine
- [ ] Dynamic Coordinator: analyzes task (M12-TASK-ANALYSIS) → assembles team (M12-DYNAGENT) → manages execution
- [ ] Execution strategies:
  - **Sequential:** simple tasks, one agent after another
  - **Pipeline:** Planner → Coder → Reviewer → Tester, each feeds into next
  - **Parallel:** multiple Coders on different modules, then merge (M12-MERGE)
- [ ] Review pipeline: every patch passes through Reviewer (if risk >= medium)
- [ ] Progress reporting: real-time status per agent in CLI and Studio
- [ ] Cost enforcement: M12-COST budget checks before every agent spawn
- [ ] Rollback: if any agent in a team fails after max retries:
  - The **entire team's patches** are rolled back (snapshot restore from before intent execution)
  - Partial results are NOT applied — atomic team execution
  - Failed attempt logged to knowledge graph for learning
  - User notified with summary of what failed and why

---

## Architecture

```
User Intent
    │
    ▼
┌──────────────────────────────┐
│  M12-TASK-ANALYSIS            │
│  Task Analysis Engine         │
│  (rule-based, deterministic)  │
│                              │
│  Input: IntentSpec + Graph   │
│  Output: TaskProfile         │
│  {complexity, type, scope,   │
│   risk}                      │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│  M12-DYNAGENT                 │
│  Team Assembler               │
│                              │
│  TaskProfile → Agent Team    │
│  Templates from knowledge    │
│  graph (JSON-LD nodes)       │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────┐
│  Dynamic Agent Team                           │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐       │
│  │ Planner  │ │ Coder×N  │ │ Tester  │       │
│  │ +context │ │ +context │ │ +context│       │
│  │ +knowledge│ │ +knowledge│ │+knowledge│     │
│  └──────────┘ └──────────┘ └─────────┘       │
│                                              │
│  M12-COST: budget tracked per agent          │
│  M12-COST: circuit breaker on failures       │
│  M12-COST: rate limiter (max N parallel)     │
└──────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│  MCP Tool Calls               │
│  graph.mutate, build,        │
│  validate, etc.              │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│  M12-MERGE                    │
│  Concurrent Graph Merge       │
│  (module-scoped, det.)       │
│                              │
│  Conflict → reject + retry   │
│  No conflict → apply + test  │
└──────────────────────────────┘
    │
    ▼
  Knowledge Update
  (successes/failures → agent knowledge graphs)
  (cost data → Phase 10 UsageStats)
```

---

## Design Principles

1. **No manual context management.** Users never write .cursorrules, CLAUDE.md, or hook files.
2. **Agents are data, not code.** Templates are JSON-LD nodes. New agent type = new graph node.
3. **Knowledge compounds.** Every execution improves the knowledge base.
4. **Transparency.** All decisions visible in Markdown. No black box.
5. **Graceful degradation.** Analysis or assembly failure → fallback to single-agent mode.
6. **Deterministic analysis.** Task Analysis Engine is rule-based. Same input = same team.
7. **Cost-bounded.** LLM spending has hard limits. Circuit breaker prevents runaway costs.
8. **Atomic team execution.** If the team fails, all patches roll back. No partial state.

---

## Dependencies

```
Phase 9 (Build Excellence)   ──→ Phase 12 (stable multi-LLM required)
Phase 9a (Type System)       ──→ Phase 12 (agents generate ownership-correct code)
Phase 10 (Knowledge Graph)   ──→ Phase 12 (agents USE the knowledge graph + M10-CONTEXT)
Phase 5 (Intent)             ──→ Phase 12 (intent pipeline is the execution framework)
```

## Expected Files

```
src/mcp/
├── mod.rs          — MCP server core (rmcp)
├── sse.rs          — SSE transport for Studio integration
├── tools/
│   ├── graph.rs    — graph.query, graph.mutate, graph.validate, graph.describe
│   ├── build.rs    — build.compile, build.run
│   ├── deps.rs     — deps.search, deps.install
│   └── intent.rs   — intent.create, intent.execute
src/agents/
├── mod.rs          — agent system entry point
├── template.rs     — AgentTemplate JSON-LD type + seed templates
├── knowledge.rs    — agent knowledge persistence + pruning
├── analyzer.rs     — M12-TASK-ANALYSIS: TaskAnalyzer + scoring rules
├── assembler.rs    — TeamAssembler: TaskProfile → AgentTeam
├── orchestrator.rs — execution coordination + strategy selection
├── merger.rs       — M12-MERGE: concurrent graph patch merging
├── cost.rs         — M12-COST: CostTracker + CircuitBreaker + rate limiter
├── rollback.rs     — atomic team rollback on failure
└── markdown.rs     — Markdown view/edit for agent knowledge
```

## Monetization

Phase 12 unlocks the **DUUMBI TEAM tier** ($49/month/seat):
- Dynamic multi-agent orchestration
- Agent knowledge persistence and self-improvement
- Team admin panel + audit log
- SLA: 99.9% registry uptime

---

## Related

- [[DUUMBI - Phase 10 - Intelligent Context & Knowledge Graph]] — knowledge graph + context assembly
- [[DUUMBI - Phase 9 - Build Excellence & Multi-LLM]] — multi-LLM prerequisite
- [[DUUMBI - Phase 9a - Type System Completion]] — agents generate ownership-correct code
- [[DUUMBI - Phase 13 - Self-Healing & Telemetry]] — self-healing uses dynamic agents
- [[DUUMBI - Cranelift Dependency Policy]] — codegen abstraction layer
- [[DUUMBI - PRD]] — Section 7 "Autonomous AI Agents (Vision)"


---

## Addendum: Agent Template Versioning & Audit Trail

> Added 2026-03-14.

When an agent template's system prompt evolves (due to knowledge pruning, manual edits, or strategy refinements), the generated graphs remain valid (schema validator decides, not the prompt). However, for audit trail and regression analysis, every AI-generated node must record which template version produced it.

### Implementation

- [ ] Every `duumbi:AgentTemplate` node carries `duumbi:templateVersion` (SemVer string, auto-incremented on system prompt change)
- [ ] Every AI-generated graph node carries:
  - `duumbi:generatedByTemplate`: reference to the template `@id`
  - `duumbi:generatedByTemplateVersion`: the template version at generation time
  - `duumbi:generatedAt`: ISO timestamp
- [ ] These fields are metadata — they don't affect compilation or validation, but are queryable in the knowledge graph
- [ ] `duumbi describe --audit` mode: shows which agent template produced each function/module
- [ ] Regression detection: if a template version change causes benchmark success rate to drop → alert in `/stats ai` output
- [ ] Template version history: stored in `.duumbi/knowledge/agent-templates/{name}/versions/` as sequential JSON-LD snapshots


---

## Addendum: MCP Client Capability — Agents Calling External Tools

> Added 2026-03-15.

Phase 12 defines DUUMBI as an MCP **server** (exposing tools to external clients). This addendum adds MCP **client** capability: DUUMBI agents can call tools on external MCP servers. This enables integration with the broader MCP ecosystem — Figma, GitHub, browsers, databases, and any future MCP-compatible service.

### Architecture

```
DUUMBI Agent (e.g., Planner)
    │
    ├── MCP Server tools (internal): graph.query, graph.mutate, build.compile
    │
    └── MCP Client calls (external):
        ├── Figma MCP → read design components, extract styles, get layout data
        ├── GitHub MCP → read issues, create PRs, check CI status
        ├── Browser MCP → screenshot, read page content, test deployed app
        ├── Database MCP → query schema, inspect data, verify migrations
        └── Custom MCP → any user-configured MCP server
```

**Key principle:** The DUUMBI agent decides which external tools to call based on the task context. The Task Analysis Engine (M12-TASK-ANALYSIS) determines if external tools are needed and which MCP servers to connect to.

### Configuration

```toml
# config.toml
[mcp-clients]
figma = { url = "https://figma.mcp.example.com/sse", description = "Figma design data" }
github = { url = "https://github.mcp.example.com/sse", description = "GitHub repository" }
browser = { url = "http://localhost:3001/sse", description = "Browser automation" }
```

### New Op in Task Analysis

The Task Analysis Engine (M12-TASK-ANALYSIS) gains a new dimension:

| Dimension | Values | Example |
|---|---|---|
| **External tools needed** | `none`, `figma`, `github`, `browser`, `custom` | Intent: "Create a Figma plugin that..." → `figma` |

If external tools are detected, the assembled agent team includes MCP client connections to the relevant servers. The agents can then call external tools alongside internal DUUMBI tools.

### Implementation

- [ ] `MpcClientManager` struct: manages connections to configured external MCP servers
- [ ] Client discovery: on agent startup, connect to configured MCP servers, retrieve available tool lists
- [ ] Tool injection: external tools are available to agents alongside internal DUUMBI tools
- [ ] Agent prompt context: include available external tool descriptions in agent system prompts
- [ ] Permission control: `[mcp-clients]` in config.toml — user explicitly configures which external servers are trusted
- [ ] Cost tracking: external MCP tool calls counted in M12-COST budget (as "external tool" category)
- [ ] Error handling: external MCP server unavailable → graceful degradation (agent proceeds without external tools)

### Use Cases Enabled

| Integration | What DUUMBI Agents Can Do | Value |
|---|---|---|
| **Figma MCP** | Read component tree, extract design tokens, generate UI logic based on design | Design → code bridge (not full frontend, but data extraction + plugin generation) |
| **GitHub MCP** | Read issues for context, create PRs with generated code, check CI results | Automated PR workflow for DUUMBI-generated modules |
| **Browser MCP** | Test deployed applications, screenshot for validation, scrape documentation | Automated acceptance testing |
| **Database MCP** | Read schema for DbQuery generation, verify data after operations | Schema-aware database code generation |
| **Slack/Discord MCP** | Post build results, notify on self-healing events | Team communication integration |

### Relationship to WASM + Figma

The original question about Figma + WASM integration is answered by combining two features:

1. **MCP Client (this addendum):** DUUMBI agents read Figma design data via Figma MCP → use it as context for code generation
2. **WASM target (Phase 15a):** The generated code compiles to WASM → runs as Figma plugin or in browser

Together: **Figma MCP provides the "what" (design data), DUUMBI generates the "how" (logic as JSON-LD graph), WASM compilation provides the "where" (runs in Figma/browser).** This is not full frontend development, but it enables design-tooling automation that no other semantic compiler offers.

### Expected Files

```
src/mcp/
├── client/
│   ├── mod.rs        — MpcClientManager
│   ├── discovery.rs  — connect to external servers, list available tools
│   ├── proxy.rs      — route agent tool calls to external MCP servers
│   └── config.rs     — [mcp-clients] config parsing
```
