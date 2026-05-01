# Operating Modes Implementation Tasks

Status: proposed
Date: 2026-05-01

## Recommended Sequence

Implement `query` in four increments:

1. Shared mode foundation.
2. CLI query mode.
3. Studio query mode.
4. MCP, knowledge, and hardening.

This sequence keeps the blast radius controlled. The first usable result should be a read-only CLI query loop backed by a real provider text API and deterministic local context.

## P0 - Shared Foundation

### MODE-001 - Create Shared Interaction Mode Model

Goal: Move operating-mode semantics out of CLI-only state.

Files:

- Create `src/interaction/mod.rs`
- Create `src/interaction/router.rs`
- Update `src/lib.rs`
- Update `src/cli/mode.rs`

Work:

- Define `InteractionMode { Query, Agent, Intent }`.
- Keep `ReplMode` only if it is a UI alias, or replace it with `InteractionMode`.
- Add labels, parser, cycling order, and tests.
- Add request-shape helpers for question-like, mutation-like, and intent-like inputs.

Acceptance criteria:

- `query`, `agent`, and `intent` labels are stable.
- Shift+Tab order can be expressed as a pure function.
- CLI and Studio can import the same enum.
- Existing `agent` and `intent` behavior remains unchanged.

Tests:

- Unit tests for labels, parsing, and cycle order.
- Existing REPL mode tests updated from two modes to three.

### MODE-002 - Add Provider Text Answer API

Goal: Support read-only LLM answers without graph mutation tools.

Files:

- `src/agents/mod.rs`
- `src/agents/anthropic.rs`
- `src/agents/openai.rs`
- `src/agents/grok.rs`
- `src/agents/openrouter.rs`
- `src/agents/minimax.rs`
- `src/agents/fallback.rs`
- Provider tests under existing integration/unit test modules.

Work:

- Add `answer(...)` and `answer_streaming(...)` to `LlmProvider`.
- Implement plain chat/completion calls per provider.
- Extend fallback chain to retry transient provider failures for text answers.
- Keep mutation tool-call methods intact.

Acceptance criteria:

- Query mode does not call `call_with_tools`.
- Providers can stream text chunks for query answers.
- Provider credential tests can still use a minimal answer call or existing no-tool validation.

Tests:

- Mock provider returns plain text answer.
- Fallback chain tries next provider on transient text-answer failure.
- Auth/model-denied errors do not incorrectly fall through as successful answers.

### MODE-003 - Build Query Core Module

Goal: Add a reusable read-only query engine.

Files:

- Create `src/query/mod.rs`
- Create `src/query/engine.rs`
- Create `src/query/context.rs`
- Create `src/query/prompt.rs`
- Create `src/query/sources.rs`
- Update `src/lib.rs`

Work:

- Define `QueryRequest`, `QueryAnswer`, `SourceRef`, `AnswerConfidence`, and `ModeHandoff`.
- Build `QueryContextAssembler` from existing `context::analyzer`, `KnowledgeStore`, `SessionManager`, intent loaders, and `describe_to_string`.
- Fit query context within token budget.
- Generate answer prompt with explicit read-only and evidence rules.
- Return sources even when provider text has no citations.

Acceptance criteria:

- Query engine can answer from a workspace without mutating files.
- Answer includes sources and confidence.
- Missing provider produces an actionable configuration error.
- Missing workspace still supports general DUUMBI help from local docs/config where possible.

Tests:

- Temp workspace query returns module/function summary.
- File hashes for `.duumbi/graph` and `.duumbi/intents` remain unchanged after query.
- Query with focused intent includes intent source.
- Query context budget trimming preserves direct target module.

## P1 - CLI Query Mode

### MODE-004 - Extend REPL Mode Cycling and Rendering

Goal: Make `query` visible and easy to select in the terminal UI.

Files:

- `src/cli/app.rs`
- `src/cli/mode.rs`
- `src/cli/repl.rs`

Work:

- Cycle `query -> agent -> intent -> query`.
- Update mode strip label and placeholder copy.
- Add tests for three-mode cycling.
- Preserve focused intent display in the mode strip.

Acceptance criteria:

- New REPL sessions start in the chosen default mode.
- Recommendation: default to `query` for safety in new or unfamiliar workspaces, but preserve `agent` if backward compatibility is considered more important.
- Mode strip always shows the active mode.

Tests:

- BackTab cycles through all three modes.
- Placeholder changes per mode.
- Status/mode rendering remains stable at narrow terminal widths.

### MODE-005 - Route Query Input in REPL

Goal: Natural-language input in `query` mode calls the query engine.

Files:

- `src/cli/repl.rs`
- `src/cli/completion.rs`

Work:

- Add `ReplMode::Query` or shared `InteractionMode::Query` branch in `process_input`.
- Implement `handle_query_input(...)`.
- Stream text chunks into the conversation pane.
- Record query turns in session history with task type `Query`.
- Add `/query <question>` and `/ask <question>` slash commands.
- Add `/mode <query|agent|intent>`.

Acceptance criteria:

- Query answers appear as assistant output.
- Query does not create snapshots.
- Query does not auto-build.
- `/query` works from any current mode.
- `/history` distinguishes query turns from mutation turns.

Tests:

- Unit tests for slash command completion entries.
- Integration test with mock provider proves no graph write.
- Session persistence includes query turn.

### MODE-006 - Add Query-to-Action Handoff

Goal: Let users move from understanding to action without losing context.

Files:

- `src/query/engine.rs`
- `src/cli/repl.rs`
- `src/cli/app.rs`

Work:

- Render `suggested_handoff` in query answers.
- Support one-shot commands:
  - `/agent <request>`
  - `/intent create <description>`
- Optional later feature: `/accept` applies the latest suggested handoff.

Acceptance criteria:

- Query can recommend an agent or intent request.
- No handoff runs automatically.
- Suggested handoff text is copyable from conversation blocks.

Tests:

- Query response with handoff does not mutate graph.
- `/agent <request>` from query mode routes to existing mutation path.

## P2 - Studio Query Mode

### MODE-007 - Extend Studio Chat Protocol With Mode

Goal: Make Studio chat mode-aware.

Files:

- `crates/duumbi-studio/src/ws.rs`
- `crates/duumbi-studio/src/script/studio.js`
- `crates/duumbi-studio/src/app.rs`
- `crates/duumbi-studio/src/state.rs`

Work:

- Add `mode` field to `ChatRequest`.
- Default missing mode to `agent` for protocol compatibility, or `query` for safety if UX is updated at the same time.
- Add response frame for query answers with sources and confidence.
- Ensure query mode sends no `refresh: true` frame.

Acceptance criteria:

- Studio can send `query`, `agent`, and `intent` modes.
- Query responses stream in the chat panel.
- Agent mutation behavior remains unchanged.
- Graph refresh happens only for mutation success.

Tests:

- WebSocket unit/integration test for `query` frame.
- Studio client handles `answer`, `handoff`, `error`, and existing `result` frames.

### MODE-008 - Add Studio Chat Mode Selector

Goal: Make mode boundaries visible in the main Studio workflow.

Files:

- `crates/duumbi-studio/src/app.rs`
- `crates/duumbi-studio/src/style/studio.css`
- `crates/duumbi-studio/src/script/studio.js`

Work:

- Add a compact segmented control in the chat panel: Query, Agent, Intent.
- Update placeholder copy by mode.
- Add non-intrusive safety copy through affordance, not explanatory paragraphs.
- Preserve selected mode across panel navigation during a browser session.

Acceptance criteria:

- User can switch modes without leaving Graph panel.
- Selected mode is visible near the chat input.
- Query mode never highlights changed graph nodes.
- Agent mode still highlights changed nodes after mutation.

Tests:

- JS state test or browser smoke test for mode selection.
- Manual Studio smoke: query "What modules exist?" then agent "Add..." and verify only agent refreshes graph.

### MODE-009 - Intent Handoff in Studio

Goal: Let query answers suggest intent creation for larger work.

Files:

- `crates/duumbi-studio/src/ws.rs`
- `crates/duumbi-studio/src/script/studio.js`
- `crates/duumbi-studio/src/server_fns.rs`

Work:

- Render query handoff actions in chat.
- "Create intent from suggestion" opens the existing create intent flow prefilled with suggested text.
- Do not execute automatically.

Acceptance criteria:

- Query can recommend intent creation.
- User reviews the generated intent before saving/executing.
- Existing create intent server function remains the only path that persists intent YAML.

Tests:

- Client-side handoff action populates create-intent textarea.
- No intent file exists until user confirms create.

## P3 - MCP and Knowledge Integration

### MODE-010 - Add Conversational Query MCP Tool

Goal: Expose query mode to external MCP clients.

Files:

- `src/mcp/tools/query.rs`
- `src/mcp/tools/mod.rs`
- `src/mcp/server.rs`

Work:

- Add `query.ask` tool.
- Inputs: question, optional module, optional c4_level, optional include_sources.
- Output: answer text, sources, confidence, suggested handoff.
- Enforce read-only behavior.

Acceptance criteria:

- External clients can ask DUUMBI questions without graph mutation.
- `query.ask` appears alongside existing graph tools.
- Tool docs clearly distinguish `query.ask` from `graph.query`.

Tests:

- MCP tool schema test.
- Tool invocation with temp workspace returns answer and leaves graph unchanged.

### MODE-011 - Add Query Knowledge Capture

Goal: Let useful query answers become durable knowledge without automatic noise.

Files:

- `src/query/engine.rs`
- `src/knowledge/types.rs`
- `src/knowledge/store.rs`
- CLI and Studio action surfaces.

Work:

- Add optional "save this as knowledge" flow for user-approved answers.
- Store as `DecisionRecord` or `PatternRecord` depending on answer type.
- Never auto-save every query answer.

Acceptance criteria:

- User can save a query answer as a decision or pattern.
- Saved knowledge appears in `/knowledge list`.
- Future query context can retrieve the saved node.

Tests:

- Save query answer roundtrip through `KnowledgeStore`.
- Query context includes matching saved knowledge by tag or keyword.

## P4 - Hardening and Product Quality

### MODE-012 - Add Mode Router Warnings

Goal: Reduce accidental misuse without blocking expert workflows.

Files:

- `src/interaction/router.rs`
- `src/cli/repl.rs`
- `crates/duumbi-studio/src/ws.rs`

Work:

- Detect question-shaped prompts in `agent` mode.
- Detect mutation-shaped prompts in `query` mode.
- Suggest mode switch or handoff.
- Allow explicit override.

Acceptance criteria:

- "What does this do?" in agent mode suggests query.
- "Add a function" in query mode returns a handoff instead of mutating.
- Slash commands still run exactly as requested.

Tests:

- Router classification tests.
- CLI route tests for ambiguous prompts.

### MODE-013 - Add Query Evaluation Fixtures

Goal: Keep answer quality testable.

Files:

- `tests/integration_query_mode.rs`
- `docs/testing/query-mode-walkthrough.md`

Work:

- Create temp workspaces with known graphs and intents.
- Ask deterministic questions using mock provider.
- Assert source selection, no writes, and handoff classification.
- Add manual walkthrough for live provider testing.

Acceptance criteria:

- Automated tests prove safety contract.
- Manual walkthrough covers CLI and Studio.
- Query mode is included in release readiness checklist.

Tests:

- `cargo test --all integration_query_mode`
- Manual Studio query smoke.

### MODE-014 - Update Architecture and User Docs

Goal: Make the three-mode model canonical.

Files:

- `docs/architecture.md`
- `docs/testing/phase15-walkthrough.md`
- Obsidian vault docs in a follow-up sync:
  - `DUUMBI - PRD.md`
  - `DUUMBI - CLI Interactive Surface Map.md`
  - `DUUMBI - Phase 15 - Studio Workflow Redesign.md`

Work:

- Document query/agent/intent responsibilities.
- Update REPL mode strip docs.
- Update Studio chat workflow docs.
- Clarify that provider selection remains internal.

Acceptance criteria:

- New contributors can answer: "Which mode should this prompt use?"
- CLI and Studio docs describe the same behavior.
- Existing architecture doc no longer implies all natural-language chat is mutation.

## Cross-Cutting Acceptance Criteria

- Query mode is read-only by enforced code path.
- Agent mode still supports direct graph mutation, snapshots, undo, and auto-build.
- Intent mode still supports spec creation, review, execution, and verification.
- CLI and Studio use the same shared mode definitions.
- Provider model selection remains internal; users choose provider setup, not low-level model defaults.
- Tests cover mode dispatch, provider text calls, query context, read-only guarantees, and Studio protocol frames.
