# Query Mode Specification

Status: proposed
Date: 2026-05-01

## Summary

`query` is DUUMBI's read-only conversational mode.

It lets the developer ask questions about the current workspace, graph, intent specs, build state, dependencies, sessions, and accumulated knowledge. It uses LLM assistance, but answers must be grounded in DUUMBI state and should expose the evidence used.

`query` is not a soft version of `agent`. It is a different contract:

- It may inspect.
- It may explain.
- It may recommend.
- It must not mutate project state.

## Goals

- Let users understand the semantic graph without reading raw JSON-LD.
- Support architecture and design discussion before changes are made.
- Make DUUMBI feel like a thinking partner, not only a mutation command runner.
- Reduce accidental writes from question-shaped prompts.
- Provide a safe bridge into `agent` or `intent` when the user chooses to act.

## Non-Goals

- Do not apply graph patches.
- Do not auto-build as a side effect.
- Do not create or execute intents.
- Do not call external write-capable tools.
- Do not hide uncertainty when the answer is inferred rather than directly present in state.

## User Stories

- As a developer, I can ask "What functions exist in this workspace?" and get a concise answer grouped by module.
- As a developer, I can ask "Why is this intent failing?" and get likely causes grounded in the intent YAML, verifier output, and graph structure.
- As a developer, I can ask "Where should a `power` function live?" and get an architectural recommendation with trade-offs.
- As a developer, I can ask "What will change if I modify this function signature?" and get callers, callees, and risk level.
- As a developer, I can ask "What did we do in the previous session?" and get a summary from session history.
- As a developer, I can ask "Can you do it?" and the system proposes a mode handoff instead of mutating silently.

## Mode Semantics

| Request shape | Preferred mode | Behavior |
|---|---|---|
| "What is...", "Why...", "Where...", "Explain...", "Compare..." | `query` | Answer with evidence and no writes |
| "Add...", "Fix...", "Change...", "Refactor this one thing..." | `agent` | Patch graph, snapshot, validate, build |
| "Build a feature/app/library...", "Plan...", "Implement with tests..." | `intent` | Create/review/execute intent spec |
| Ambiguous question with possible action | `query` | Explain options and offer explicit handoff |

## UX Contract

### CLI REPL

The mode strip should cycle through:

```text
query -> agent -> intent -> query
```

Recommended controls:

- `Shift+Tab`: cycle modes.
- `/mode query`, `/mode agent`, `/mode intent`: explicit switch.
- `/query <question>` or `/ask <question>`: one-shot query without permanently switching modes.
- `/agent <request>`: one-shot agent mutation from another mode.
- `/intent create <description>`: existing intent path remains.

Mode strip copy:

```text
Shift Tab switch mode    query
```

Prompt placeholders:

- `query`: `e.g. "what modules exist?" or /help`
- `agent`: `e.g. "add a clamp function to math/ops" or /help`
- `intent`: `e.g. "plan a calculator module" or /intent create`

### Studio

Studio chat should expose a compact segmented control:

- Query
- Agent
- Intent

The WebSocket request should include `mode`:

```json
{
  "type": "chat",
  "mode": "query",
  "module": "app/main",
  "c4_level": "component",
  "message": "What does this module do?"
}
```

Query responses should not trigger graph refresh. Agent mutations should keep the existing refresh behavior.

Recommended response frames:

```json
{ "type": "chunk", "text": "..." }
{ "type": "answer", "sources": [...], "confidence": "medium" }
{ "type": "handoff", "mode": "agent", "suggested_request": "..." }
```

## Query Engine Architecture

```text
User question
    |
    v
QueryEngine
    |
    +-- classify question intent
    +-- collect read-only context
    +-- fit context to token budget
    +-- call provider text API
    +-- attach sources and confidence
    v
QueryAnswer
```

Recommended module layout:

```text
src/
  interaction/
    mod.rs          # InteractionMode, InteractionRequest, InteractionOutcome
    router.rs       # shared routing hints and one-shot mode commands
  query/
    mod.rs
    engine.rs       # QueryEngine
    context.rs      # QueryContextAssembler
    sources.rs      # SourceRef, evidence formatting
    prompt.rs       # query system prompt
```

## Data Model

Sketch:

```rust
pub enum InteractionMode {
    Query,
    Agent,
    Intent,
}

pub struct QueryRequest {
    pub question: String,
    pub workspace_root: PathBuf,
    pub visible_module: Option<String>,
    pub c4_level: Option<C4Level>,
    pub session_turns: Vec<PersistentTurn>,
}

pub struct QueryAnswer {
    pub text: String,
    pub sources: Vec<SourceRef>,
    pub confidence: AnswerConfidence,
    pub suggested_handoff: Option<ModeHandoff>,
}

pub enum SourceRef {
    GraphModule { module: String, path: PathBuf },
    GraphNode { module: String, node_id: String },
    Intent { slug: String, path: PathBuf },
    KnowledgeNode { id: String, kind: String },
    SessionTurn { session_id: String, index: usize },
    CommandOutput { command: String },
}
```

## Context Sources

Minimum viable context:

- Workspace module map from `context::analyzer`.
- Function signatures and exports.
- Currently visible module or focused intent.
- `commands::describe_to_string(...)` for readable graph summaries.
- Active and archived intent metadata.
- Session turns from `SessionManager`.
- Knowledge nodes from `KnowledgeStore`.
- Validation diagnostics from non-mutating graph checks.

Later context:

- Dependency tree summaries.
- Registry search results when explicitly requested.
- Telemetry and benchmark summaries.
- Vector or embedding search over knowledge and docs.

## Provider API Requirement

The existing provider trait only supports tool-call graph mutation. Query mode needs plain text generation.

Add methods similar to:

```rust
fn answer<'a>(
    &'a self,
    system_prompt: &'a str,
    user_message: &'a str,
) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>>;

fn answer_streaming<'a>(
    &'a self,
    system_prompt: &'a str,
    user_message: &'a str,
    on_text: &'a (dyn Fn(&str) + Send + Sync),
) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>>;
```

Do not fake query mode through mutation tool calls. That would preserve the wrong system prompt, wrong failure mode, and wrong provider semantics.

## Answer Contract

Every query answer should follow this priority order:

1. Direct answer.
2. Evidence from DUUMBI state.
3. Assumptions or uncertainty.
4. Suggested next action, if useful.

For architecture-sensitive answers, explicitly separate:

- Facts: directly observed in graph, docs, config, session, or command output.
- Interpretation: reasoned conclusion from those facts.
- Risk: what could fail or become expensive.
- Next action: query, agent, or intent handoff.

## Safety Boundary

`query` mode must enforce read-only behavior in code, not only in prompt text.

Hard constraints:

- No writes to `.duumbi/graph`.
- No writes to `.duumbi/intents`.
- No snapshot creation.
- No auto-build output writes unless the user runs a slash command outside query.
- No `graph.mutate`.
- No registry publish/yank/install.
- No external MCP tool with write capability.

Allowed operations:

- Read graph files.
- Parse, validate, and describe graph in memory.
- Read config, manifests, lockfile, intents, knowledge, sessions.
- Run checks that do not persist project changes.

If an answer recommends a change, return a handoff proposal:

```text
This should be an agent task:
agent: Add power(base, exp) to calculator/ops and update main to demonstrate it.
```

The user must explicitly accept or paste/run the handoff.

## Interaction With Existing Modes

### Query to Agent

Use when a question resolves into a bounded mutation.

Example:

```text
User: Where should power(base, exp) live?
DUUMBI: It belongs in calculator/ops because add/sub/mul/div already live there.
Suggested agent request: Add power(base, exp) to calculator/ops and call it from main.
```

### Query to Intent

Use when a question resolves into a feature-sized goal.

Example:

```text
User: How should we add string utilities?
DUUMBI: This is intent-sized because it creates multiple functions and test cases.
Suggested intent: Create a string utility library with reverse, count_vowels, and is_palindrome, plus tests and a main demo.
```

### Agent to Query

When an `agent` prompt is question-shaped, suggest switching:

```text
This looks like a question. Run it in query mode? Use /query "..."
```

This should be a soft guard, not a blocker, because advanced users may intentionally ask an agent to inspect before mutating.

## Success Criteria

`query` mode is successful when:

- A user can ask five common project questions and receive grounded answers without graph changes.
- The same question in CLI and Studio uses the same engine.
- A query turn records in session history.
- Answer sources point to graph modules, intents, knowledge nodes, or session turns.
- Question-shaped prompts no longer accidentally mutate the graph.
- A query answer can cleanly hand off to `agent` or `intent`.
