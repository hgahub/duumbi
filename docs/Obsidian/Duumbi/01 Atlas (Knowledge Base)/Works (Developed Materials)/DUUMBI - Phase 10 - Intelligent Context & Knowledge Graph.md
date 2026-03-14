---
tags:
  - project/duumbi
  - milestone/phase-10
status: planned
github_milestone: ~
updated: 2026-03-14
---
# Phase 10 — Intelligent Context & Knowledge Graph ⏳

> **Kill Criterion:** In a 3+ module project, `duumbi add "..."` (1) recognizes existing modules, (2) asks clarifying questions when ambiguous, (3) creates new modules when needed instead of dumping everything into main, (4) automatically wires in existing graph elements. Context assembly requires zero manual CLAUDE.md-style files. Session state persists across CLI restarts.
> **Status:** ⏳ Planned — starts after Phase 9
> **Estimated duration:** 8–10 weeks (solo developer)

← Back: [[DUUMBI Roadmap Map]]

---

## Summary

This phase builds DUUMBI's core differentiator: **the system manages its own context, not the user.** While tools like Claude Code require developers to maintain CLAUDE.md files, Cursor requires .cursorrules, and Copilot requires manual context management, DUUMBI automatically assembles the right context for every LLM call from its semantic graph.

The knowledge graph stores project decisions, patterns, successful operations, failures, session state, task data, and usage statistics as JSON-LD nodes — queryable, traversable, and automatically included in LLM prompts when relevant. Users see Markdown views they can read and edit; the system sees structured graph data.

This phase also builds the **session and workflow persistence infrastructure** that Phase 11 exposes through CLI commands.

**This is where the Original PRD's "Obsidian-like knowledge management" vision materializes — but graph-native, not file-based.**

---

## Tasks

### M10-KNOW: Knowledge Graph Foundation
- [ ] Project knowledge stored as JSON-LD nodes: decisions, patterns, errors, successes
- [ ] Markdown view/edit layer: users see `.md` files, system reads/writes JSON-LD graph (see M10-MARKDOWN for sync details)
- [ ] Automatic context assembly for LLM calls: deterministic, rule-based (see M10-CONTEXT for full algorithm)
- [ ] Success logging: `.duumbi/learning/successes.jsonl` — every successful operation recorded
- [ ] Automatic few-shot selection with token budget management (see M10-CONTEXT)
- [ ] Knowledge node types: `duumbi:Decision`, `duumbi:Pattern`, `duumbi:Error`, `duumbi:Success`
- [ ] Backlink traversal: every node knows what references it (bidirectional graph via petgraph)

---

### M10-CONTEXT: Context Assembly Algorithm (Deterministic, Rule-Based)

The core algorithm that replaces manual CLAUDE.md-style context files. Given a task, it selects the relevant graph subset for the LLM prompt. **Fully deterministic:** same task + same graph = same context every time.

#### Architecture

```
Incoming Task (from CLI or intent pipeline)
    │
    ▼
┌─────────────────────────────────┐
│  Step 1: Task Classification     │
│  Rule-based: what kind of task?  │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  Step 2: Traversal Strategy      │
│  Per task-type: which graph      │
│  traversal pattern to apply?     │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  Step 3: Node Collection         │
│  Execute traversal, collect      │
│  relevant nodes + properties     │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  Step 4: Token Budget Fitting    │
│  Rank by relevance, trim to      │
│  fit within token budget          │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  Step 5: Few-Shot Injection      │
│  Select matching past successes  │
│  within remaining token budget   │
└─────────────────────────────────┘
    │
    ▼
  Assembled Context → LLM Prompt
```

#### Step 1: Task Classification

Every incoming task is classified into one of these types (rule-based, no LLM needed):

| Task Type | Trigger | Example |
|---|---|---|
| `CreateModule` | Intent spec `modules.create` field | "Create smtp/server module" |
| `AddFunction` | `duumbi add` with function-creating intent | "Add a sort function" |
| `ModifyFunction` | `duumbi add` targeting existing function | "Make handle_ehlo return Result" |
| `ModifyMain` | Intent spec `modules.modify` contains `app/main` | "Wire new module into main" |
| `FixError` | Retry after validation failure (E0xx error) | "Fix E021 use-after-move" |
| `RefactorModule` | User intent mentions refactor/restructure | "Split this module into two" |
| `AddTest` | User intent mentions test/testing | "Add tests for the sort function" |

Classification rules are pattern-matched on: (a) intent spec fields, (b) CLI command context, (c) error codes from validator.

#### Step 2: Traversal Strategy (Per Task Type)

Each task type has a deterministic traversal strategy:

**CreateModule:**
```
1. ALL existing module names + their export lists (1 hop: module → exports)
2. ALL stdlib modules available (from stdlib/ directory)
3. Target module's intended neighbors (from intent spec `modules.create` context)
4. Schema excerpt: allowed Op types for the target C4 level
5. C4 structure summary (which C4 level is this module at?)
```

**AddFunction:**
```
1. Target module: ALL nodes (full module content)
2. Target module's imports: imported module export lists (1 hop outward)
3. Target module's existing functions: signatures only (not implementation)
4. If function name matches existing stdlib function → include that stdlib source
5. Schema: allowed Op types + type system summary
```

**ModifyFunction:**
```
1. Target function: ALL nodes (full function content including blocks + ops)
2. Target function's callers: which functions call this one (backlink traversal)
3. Target function's callees: which functions this one calls (forward traversal)
4. Target module: other functions in same module (signatures only)
5. If Result type involved: Result/Option schema excerpt
```

**ModifyMain:**
```
1. Main module: full content
2. ALL modules in project: name + export list
3. New module to wire in: full export list + function signatures
4. Dependency graph: current config.toml dependencies
```

**FixError:**
```
1. Error node: the specific node identified by error code + nodeId
2. Error context: 2-hop neighborhood around the error node
3. Error description: the E0xx error message + suggested fix
4. Similar past fixes: successes.jsonl filtered by same error code (max 3)
5. Schema excerpt: the rule that was violated
```

**RefactorModule:**
```
1. Target module: ALL nodes
2. ALL modules that import from target (backlink: who depends on this?)
3. ALL modules that target imports from (forward: what does this depend on?)
4. C4 structure: where does this module sit in the hierarchy?
```

**AddTest:**
```
1. Target function: full content
2. Target function's intent spec: acceptance criteria + test cases
3. Existing test patterns: from successes.jsonl filtered by test-related tasks
4. Verifier Agent test format: how tests are structured in DUUMBI
```

#### Step 3: Node Collection

The traversal strategy is executed on the petgraph `StableGraph`:
- Each step produces a set of node IDs
- Nodes are deduplicated (set union)
- Each node is serialized to its JSON-LD representation
- Properties included: `@id`, `@type`, `duumbi:name`, `duumbi:params`, `duumbi:returnType`, `duumbi:imports`, `duumbi:exports`
- For "signatures only" steps: only the function declaration is included, not blocks/ops

#### Step 4: Token Budget Fitting

The collected nodes are ranked and trimmed to fit the token budget:

```
Total token budget = permissions.max-tokens (from config.toml, default 8192)
Reserved for system prompt: 1500 tokens (fixed)
Reserved for few-shot examples: 1500 tokens (configurable)
Reserved for LLM response: 2000 tokens (minimum)
Available for context: budget - reserved = remaining
```

**Ranking heuristic (priority order):**
1. **Error context** (if FixError task): the broken node + error message (highest priority)
2. **Target node**: the direct subject of the task
3. **1-hop neighbors**: immediate dependencies and dependents
4. **Schema excerpts**: relevant type/op definitions
5. **2-hop neighbors**: indirect connections
6. **C4 summary**: structural overview (lowest priority, trimmed first)

If total exceeds budget: trim from bottom of priority list. Never trim the target node or error context.

**Token counting:** Use `tiktoken-rs` crate (already in Phase 4 dependencies) with cl100k_base encoding. Count tokens after JSON-LD serialization.

#### Step 5: Few-Shot Injection

From `.duumbi/learning/successes.jsonl`, select relevant past successes:

**Selection criteria (rule-based, scored):**
- Same task type: +3 points
- Same error code (for FixError): +5 points
- Same target module: +2 points
- Same Op types involved: +1 point per matching Op
- Recency: +1 point if within last 50 operations

**Budget management:**
- Maximum 3 few-shot examples per LLM call
- Each example trimmed to ≤500 tokens (only the patch delta, not the full graph)
- Total few-shot budget: 1500 tokens (configurable in config.toml)
- If no examples score >2 points: skip few-shot entirely (don't waste tokens on irrelevant examples)

---

### M10-MARKDOWN: Markdown ↔ JSON-LD Sync Layer

Users can view and edit knowledge graph content as Markdown files. The system maintains bidirectional sync.

#### Sync Architecture

```
.duumbi/knowledge/           ← JSON-LD (source of truth)
    │
    ├── decisions/
    │   └── use-result-for-errors.jsonld
    ├── patterns/
    │   └── module-naming-convention.jsonld
    └── successes/
        └── successes.jsonl

.duumbi/knowledge-md/        ← Markdown view (derived, auto-generated)
    │
    ├── decisions/
    │   └── use-result-for-errors.md
    ├── patterns/
    │   └── module-naming-convention.md
    └── README.md             ← auto-generated index
```

#### Sync Rules

1. **JSON-LD is the source of truth.** If conflict, JSON-LD wins.
2. **Auto-generation:** On every graph mutation, affected `.md` files are regenerated from JSON-LD.
3. **User edits:** When a user edits an `.md` file:
   - File watcher detects the change (same `notify-debouncer-mini` pattern as Phase 3)
   - System parses the Markdown and attempts to map changes back to JSON-LD properties
   - If mapping succeeds: JSON-LD is updated, `.md` is regenerated (round-trip validation)
   - If mapping fails: `.md` is reverted to last known good state, user is warned
4. **No concurrent edit:** The system holds a write lock during graph mutations. If the user edits `.md` during a mutation, the user edit is queued and applied after the mutation completes.
5. **Conflict resolution: last-writer-wins with notification.** If both system and user modified the same node property:
   - System mutation takes precedence (it's triggered by a validated operation)
   - User is notified: "Your edit to X was overwritten by a system operation. Your version is saved in `.duumbi/knowledge-md/.conflicts/`"
   - Conflict archive: `.duumbi/knowledge-md/.conflicts/{timestamp}-{filename}.md`

#### Markdown Format

Each knowledge node renders as a Markdown file with YAML frontmatter:

```markdown
---
id: "duumbi:knowledge/decisions/use-result-for-errors"
type: Decision
created: 2026-04-15
tags: [error-handling, type-system]
---
# Use Result<T,E> for Error Handling

## Decision
All functions that can fail should return `Result<T, E>` instead of panicking.

## Rationale
Matches Rust conventions. The schema validator (E030) enforces that Result values
are always handled, preventing ignored errors.

## Affected Modules
- parser/parse_number
- io/read_stdin
```

---

### M10-AWARE: Codebase Awareness
- [ ] Project Analyzer: full graph scan on startup → module map, dependency graph, available exports
- [ ] Context injection via M10-CONTEXT algorithm (not ad-hoc)
- [ ] Existing graph recognition: if `stdlib/math` exists and task needs `add` → wire it in, don't regenerate
- [ ] Clarifying questions: if intent is ambiguous (which module should this go in?) → interactive CLI question
- [ ] C4 level auto-assignment: every module node gets `duumbi:c4Level` (Context/Container/Component/Code)
- [ ] `duumbi describe --c4` → C4 diagram generation from the graph

### M10-MOD: Intelligent Modularization
- [ ] Task → module mapping: Coordinator distributes across multiple JSON-LD files, not one monolith
- [ ] Module boundary detection: explicit LLM instruction on when to create new modules
- [ ] Auto-wiring: new module automatically added to `config.toml` dependencies
- [ ] Main.jsonld update: new functions automatically called from main
- [ ] No-duplicate guarantee: if a function with matching signature exists in any reachable module → reuse it

### M10-PERF: Performance Metadata
- [ ] Benchmark data on module nodes: `duumbi:benchmarkMs`, `duumbi:memoryBytes`
- [ ] Intelligent selection: when multiple similar modules exist → prefer the faster one
- [ ] `duumbi bench <module>` command: isolated module benchmarking
- [ ] Performance metadata stored in the graph, not separate files

### M10-SESSION: Session & Workflow Persistence Infrastructure

This section provides the **data layer** for session management. Phase 11 builds the CLI commands on top.

#### Session State Persistence
- [ ] Session data model: `duumbi:Session` JSON-LD node type
  - Active conversation history (last N messages, configurable)
  - Current working context (active module, active intent, selected provider)
  - Pending operations (half-finished intents, queued tasks)
  - Session metadata (start time, total duration, provider usage counters)
- [ ] Session storage: `.duumbi/session/current.json` — serialized on every state change
- [ ] Session archive: `.duumbi/session/history/` — completed sessions
- [ ] Automatic save: session state written on every command completion
- [ ] Crash recovery: restorable from last checkpoint
- [ ] Session API: `SessionManager` with `save()`, `load()`, `archive()`, `list_archived()`

#### Task Data Model
- [ ] `duumbi:Task` JSON-LD node type in knowledge graph
  - Fields: id, description, status (pending/active/done/blocked), created_at, completed_at
  - Links: parent intent, blocking dependencies
  - Automatic creation from intent decomposition + manual creation via API
- [ ] Task query API: list by status, filter by intent, dependency resolution
- [ ] Relationship to Phase 5 intent tasks: Phase 5 Coordinator produces tasks → Phase 10 stores them as `duumbi:Task` nodes. Same entity, now persistent and queryable.

#### Usage Statistics Data Model
- [ ] `duumbi:UsageStats` JSON-LD node type
  - Per-session, per-provider, per-build metrics
  - Historical daily/weekly aggregates
- [ ] Stats collection: automatic instrumentation on every LLM call, build, and command
- [ ] Stats query API: by time range, by provider, by session
- [ ] Token cost estimation: configurable per-provider pricing in `config.toml`

#### Permissions Data Model
- [ ] `[permissions]` section in `config.toml` (auto-build, auto-apply, providers, max-tokens, max-retries)
- [ ] Permissions API: `PermissionsManager` with `get()`, `set()`, `validate()`
- [ ] Enforced at orchestrator level: every LLM call checks permissions

---

## Design Principles

1. **The graph IS the documentation.** No separate docs system.
2. **Atomic knowledge notes.** Small JSON-LD nodes composable into any document format.
3. **Dynamic context > static files.** Context assembled per-task, deterministically.
4. **Deterministic context selection.** Same task + same graph = same LLM prompt. No randomness, no embeddings, no external ML dependencies.
5. **Learning from experience.** Few-shot examples selected from past successes, within strict token budgets.
6. **JSON-LD is source of truth.** Markdown is a derived view. Conflicts resolved by last-writer-wins with notification.
7. **Data layer separated from UI.** APIs and data models here; CLI commands in Phase 11.

---

## Dependencies

```
Phase 9 (Build Excellence)  ──→ Phase 10 (stable build required)
Phase 5 (Intent)            ──→ Phase 10 (intent pipeline = framework for knowledge capture)
Phase 4 (Module System)     ──→ Phase 10 (module resolution = foundation for awareness)
Phase 4 (tiktoken-rs)       ──→ Phase 10 (token counting for budget management)
```

## Expected Files

```
src/knowledge/
├── mod.rs          — knowledge graph API
├── context.rs      — M10-CONTEXT: context assembly algorithm (5-step pipeline)
├── traversal.rs    — per-task-type graph traversal strategies
├── ranking.rs      — token budget fitting + priority ranking
├── learning.rs     — success logging + few-shot selection + scoring
├── analyzer.rs     — project analyzer (full graph scan)
└── markdown.rs     — M10-MARKDOWN: Markdown ↔ JSON-LD sync layer
src/session/
├── mod.rs          — SessionManager API
├── state.rs        — Session data model + serialization
├── archive.rs      — session history management
└── recovery.rs     — crash recovery logic
src/workflow/
├── tasks.rs        — duumbi:Task data model + query API
├── stats.rs        — duumbi:UsageStats collection + query API
└── permissions.rs  — PermissionsManager + config.toml integration
src/intent/
├── coordinator.rs  — (modified: intelligent modularization)
└── verifier.rs     — (modified: C4 awareness)
.duumbi/
├── knowledge/      — JSON-LD knowledge nodes (source of truth)
├── knowledge-md/   — Markdown views (derived, auto-generated)
│   └── .conflicts/ — user edit conflicts archive
├── learning/
│   └── successes.jsonl — successful operation log
└── session/
    ├── current.json — active session
    └── history/     — archived sessions
```

## Monetization

Phase 10 strengthens the PRO tier: intelligent context assembly and session persistence are clear differentiators.

---

## Related

- [[DUUMBI - Phase 9 - Build Excellence & Multi-LLM]] — prerequisite
- [[DUUMBI - Phase 11 - CLI UX & Developer Experience]] — builds CLI commands on this data layer
- [[DUUMBI - Phase 12 - Dynamic Agent System & MCP]] — agents use knowledge graph for context
- [[DUUMBI - Phase 13 - Self-Healing & Telemetry]] — self-healing feeds into knowledge graph
- [[DUUMBI - PRD]] — Section 6 "Knowledge Management (Vision)"


---

## Addendum: Schema Migration Strategy

> Added 2026-03-14.

The knowledge graph (Phase 10) and agent knowledge (Phase 12) accumulate data over months/years. The `core.schema.json` will evolve as new types and properties are added. Old knowledge nodes must remain readable after schema changes.

### Versioning

Every knowledge node carries a `duumbi:schemaVersion` field:

```jsonld
{
  "@type": "duumbi:Decision",
  "@id": "duumbi:knowledge/decisions/use-result",
  "duumbi:schemaVersion": "0.10.0",
  "duumbi:content": "Use Result<T,E> for error handling"
}
```

The version corresponds to the DUUMBI release that created the node (SemVer).

### Migration Rules

1. **Additive changes (new fields):** No migration needed. Old nodes missing new fields get default values when read. Example: Phase 12 adds `duumbi:generatedByTemplate` to graph nodes — old nodes from Phase 10 simply lack this field.

2. **Renamed fields:** Migration script maps old field name → new field name. Example: `duumbi:benchmarkMs` renamed to `duumbi:performanceMs` → migration rewrites all matching nodes.

3. **Removed fields:** Old field is ignored when read. No migration needed unless the removed field's data must be preserved elsewhere.

4. **Structural changes (rare):** If a node type's structure fundamentally changes (e.g., `duumbi:Task` gains nested sub-tasks), a migration script transforms old nodes to the new structure.

### Implementation

- [ ] `duumbi:schemaVersion` field on every knowledge node (M10-KNOW)
- [ ] `duumbi migrate` CLI command: scans all knowledge nodes, applies pending migrations
- [ ] Migration registry: `src/knowledge/migrations/` directory with sequential migration files
  - `v0_10_to_v0_11.rs` — field renames, structural transforms
  - Each migration is idempotent (safe to run multiple times)
- [ ] Automatic migration check on startup: if knowledge nodes have older schema version → prompt user to run `duumbi migrate`
- [ ] Backup before migration: snapshot of `.duumbi/knowledge/` before any migration

### When This Becomes Critical

- **Phase 10:** Low risk — first knowledge data, no legacy to migrate
- **Phase 11:** Low risk — session/stats data structure is simple
- **Phase 12:** **Medium risk** — agent knowledge accumulates, template system prompt evolves
- **Phase 13+:** **High risk** — months of telemetry baselines, repair history, strategy data

The migration system must be in place by Phase 12 start.
