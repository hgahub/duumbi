# DUUMBI-684: Semantic Rewrite Engine as Formal Graph Transformation Substrate

## Summary

DUUMBI should add a conservative V1 Semantic Rewrite Engine: a deterministic,
validated graph-to-graph transformation substrate for applying small,
explainable rewrites to DUUMBI JSON-LD semantic graphs.

The V1 product is not an e-graph optimizer and not autonomous self-evolution.
It is the safety and evidence layer that those later capabilities need:

- rewrite rules are first-class artifacts with IDs, descriptions,
  preconditions, safety contracts, cost bounds, and explanation text;
- preview is read-only and shows matched graph regions, proposed changes,
  validation expectations, and why the rewrite is considered safe;
- apply is explicit, bounded, snapshot-backed, and must pass the existing
  parse -> build -> validate pipeline before any graph file is written;
- CLI and MCP callers can use the same engine so humans and agents share one
  transformation contract;
- rule execution is deterministic and produces structured evidence for review.

Related to #684. This is a product-specification artifact only. The execution
issue must remain open for Stage 7 Product Spec Review, Stage 8 technical
specification, Stage 9 Technical Spec Review, Stage 10 implementation, Stage 11
review, and Stage 12 closure.

## Problem

DUUMBI already treats program logic as a validated semantic graph, but graph
mutation is still mostly patch-oriented:

- `GraphPatch` provides low-level atomic edits such as add, modify, remove,
  set edge, and replace block.
- `duumbi add` asks an LLM to emit those patch operations against raw JSON-LD
  context.
- validation catches malformed or type-invalid results, but the system does
  not yet have reusable semantic rewrite rules, rule-level preconditions,
  bounded search, stable explanations, or reviewable transformation evidence.

That leaves a gap between DUUMBI's graph-native thesis and its mutation model.
Patch operations are useful primitives, but they are not enough to express
compiler-like transformations such as canonicalization, identity simplification,
safe local optimization, or future equivalence exploration. Without a formal
rewrite substrate, agents must rediscover graph edits from freeform prompts, and
humans cannot inspect a reusable rule catalog before allowing mutation.

The accepted issue is intentionally architectural. The source note frames the
rewrite engine as a formal graph transformation system closer to compiler
optimization, theorem rewriting, and e-graph systems than to text or AST
refactoring. The product risk is overclaiming: V1 can provide deterministic,
validated, evidence-backed rewrites, but it must not claim theorem-level
semantic preservation, unbounded optimization, or autonomous self-evolution.

## Outcome

When this work is done:

- DUUMBI has a first-class Semantic Rewrite Engine in the core source repo.
- Users can list available rewrite rules from the CLI.
- Users can preview a rewrite against a workspace module without mutating the
  graph.
- Users can explicitly apply a matched rewrite with confirmation or `--yes`.
- Apply creates the same kind of undo snapshot expected from graph mutation
  workflows before writing.
- A rewrite result is written only after the patched graph parses, builds into
  the semantic graph IR, and passes validation.
- Failed preconditions, unsupported rules, missing matches, validation failures,
  and cost-bound failures produce explicit diagnostics and leave graph files
  unchanged.
- MCP clients can discover and call read-only preview/list rewrite tools and an
  explicit write-capable apply tool.
- Each preview/apply result includes bounded evidence: rule ID, match ID,
  touched node IDs, operation summary, cost estimate, validation status,
  warnings, and human-readable explanation.
- Existing `GraphPatch`, `duumbi add`, parser, validator, compiler, registry,
  intent, Query mode, and Studio behavior continue to work unless explicitly
  routed through the rewrite engine.
- E-graph equality saturation, proof-carrying rewrites, and autonomous
  rewrite exploration remain later stages behind separate accepted specs.

## Scope

### In Scope

- V1 rewrite rule model with stable rule IDs, descriptions, category, safety
  class, preconditions, effect summary, cost estimate, and explanation text.
- A deterministic rewrite engine that can find matches, produce a preview plan,
  and apply selected plans to cloned graph source.
- A small conservative built-in rule catalog sufficient to prove the substrate,
  focused on local, behavior-preserving i64/pure-op rewrites.
- Integration with the existing graph pipeline:
  - JSON-LD load;
  - parser;
  - `SemanticGraph` construction;
  - validator diagnostics;
  - `GraphPatch` or equivalent atomic patch application;
  - pretty JSON-LD write only after successful validation.
- CLI commands for rule discovery, read-only preview, and explicit apply.
- MCP tool discovery for the same list, preview, and apply behavior.
- Undo snapshot creation before successful apply writes.
- Structured JSON output mode for automation and human-readable output for CLI
  users.
- Unit and integration tests for rule matching, preview, apply, validation
  failure, cost bounds, no-match states, snapshots, CLI behavior, and MCP
  behavior.
- Documentation of the V1 rewrite contract, safety claims, and non-goals.

### Explicitly Out Of Scope

- E-graph equality saturation or an `egg`/equivalence-graph dependency.
- Formal proof generation, SMT/VCGen integration, or proof-carrying rewrites.
- Autonomous self-evolution, agent-selected rule exploration, or unattended
  rewrite campaigns.
- Broad compiler optimization passes or Cranelift IR optimization.
- Changing the graph data model, JSON-LD namespace, existing Op semantics, or
  existing validator error codes unless Stage 10 proves a narrow additive need.
- Replacing `GraphPatch`, `duumbi add`, or the existing agent mutation pipeline.
- Sending raw JSON-LD to LLMs as part of this issue.
- Studio UI changes beyond any automatic MCP/CLI behavior already exposed by
  shared backend code.
- Registry graph database evolution or semantic similarity/reuse ranking.
- New production telemetry, cloud sync, Slack workflows, or scheduled rewrite
  automation.

## Constraints And Assumptions

Facts:

- Issue #684 is open and labeled `accepted` and `needs-spec`.
- The Stage 5 Human Acceptance Decision comment dated 2026-06-13 records
  `Decision: Accept`, `Next state: Spec Needed`, and no remaining open
  questions.
- The issue originated from the processed vault note
  `Duumbi/05 Archive/Processed Inbox/2026-05-19 - Semantic Rewrite Engine Positioning.md`.
- The processed source note says the rewrite engine should operate at the
  semantic graph level, not as a text or AST refactor.
- The source note proposes staged development: Phase 1 core rule DSL and
  graph-to-graph transformations, Phase 2 bounded e-graph equality saturation,
  and Phase 3 autonomous intent-driven rewrite exploration.
- The current repo has `src/patch.rs` with `GraphPatch`, `PatchOp`, and an
  all-or-nothing `apply_patch` over cloned JSON-LD values.
- `PatchOp::ReplaceBlock` already exists for atomic block body rewrites.
- `src/graph/mod.rs` defines `SemanticGraph` around
  `petgraph::stable_graph::StableGraph`.
- `src/graph/validator.rs` validates structure, cycles, types, return types,
  branch conditions, SSA dominance, branch targets, ownership, and
  Result/Option safety where relevant.
- `src/mcp/tools/graph.rs` exposes graph query, mutate, validate, and describe
  MCP tools over the existing parse/build/validate pipeline.
- `src/hash.rs` computes deterministic graph hashes over canonicalized JSON-LD,
  but those hashes are structural identity evidence, not proof that two
  different structures are semantically equivalent.
- The PRD positions DUUMBI as graph-centered, evidence-oriented, queryable
  before mutation, and human-verifiable.
- Query mode is read-only and must remain read-only.
- The current GitHub token available during Stage 6 could not read Project V2
  fields because `read:project` scope was missing; labels and issue comments
  are the verified workflow state available to the drafting agent.

Assumptions:

- V1 should implement the Phase 1 core substrate only. E-graphs and autonomous
  exploration require separate specs because they materially change search
  space, dependencies, performance risk, and review expectations.
- V1 semantic preservation should be stated as a rule-specific safety contract
  backed by preconditions, validation, and tests, not as a universal theorem.
- Built-in V1 rules should be intentionally boring. Conservative local rewrites
  are more valuable than impressive rules that create semantic ambiguity.
- CLI is the canonical user interface for V1. MCP mirrors the same behavior for
  external agents.
- Existing `GraphPatch` remains the write primitive unless Stage 8 identifies a
  narrower internal representation that still uses the same validation and
  snapshot boundaries.
- A deterministic preview/apply engine has higher product value than adding
  live LLM involvement in V1.

## Decisions

- **Decision:** Use a file-based product spec.
  **Evidence:** The issue is architectural, cross-module, agent-facing, and
  likely to need durable implementation context and review history.

- **Decision:** Scope #684 V1 to the Phase 1 core rewrite substrate.
  **Evidence:** The accepted source note lists Phase 1 before e-graphs and
  autonomous exploration, and also names semantic explosion and unbounded search
  as primary risks.

- **Decision:** Expose CLI and MCP surfaces in V1.
  **Evidence:** The PRD requires queryable, human-verifiable graph behavior, and
  the current repo already exposes graph query/mutate/validate/describe through
  MCP for agent integration.

- **Decision:** Preview is read-only by default; apply is explicit and
  snapshot-backed.
  **Evidence:** Query mode and agentic workflow rules require read-only
  inspection before mutation, and existing mutation workflows rely on undo
  snapshots for safe local graph changes.

- **Decision:** V1 must reuse the parse -> build -> validate pipeline before
  writing.
  **Evidence:** The graph validator is DUUMBI's existing safety gate for
  structural and type correctness, and `GraphPatch` callers already rely on the
  same post-patch validation boundary.

- **Decision:** V1 must not add e-graph dependencies.
  **Evidence:** E-graph saturation is a larger optimization/search problem with
  different cost controls and proof/equivalence concerns. It belongs in a later
  accepted issue after the rule substrate exists.

- **Decision:** Do not route V1 rewrite apply through a live LLM.
  **Evidence:** The product outcome is deterministic, reviewable graph
  transformation. Live LLM rule suggestion can later select a rule, but the rule
  engine itself should be provider-free and testable in CI.

## Behavior

### Rule Discovery

Users can list available rewrite rules.

The rule list includes at least:

- rule ID;
- display name;
- category;
- safety class;
- whether the rule is read-only, previewable, and apply-capable;
- short description;
- precondition summary;
- effect summary;
- default cost bound.

If no rules are registered, the command/tool returns an explicit empty list and
does not error.

### Preview

Users can preview one rule against one workspace module.

Preview:

- loads the target JSON-LD graph source;
- builds semantic graph context for matching;
- evaluates rule preconditions deterministically;
- returns zero or more matches with stable match IDs for that invocation;
- shows touched node IDs and a concise explanation for each match;
- estimates rewrite cost before apply;
- prepares the candidate patch or candidate graph in memory;
- validates the candidate when practical without writing it;
- returns warnings if validation cannot be fully assessed until apply;
- leaves `.duumbi/graph`, `.duumbi/history`, config, credentials, registry
  cache, telemetry, and intent artifacts unchanged.

No-match preview is a successful read-only outcome. It must say that no matching
graph region was found for the selected rule and module.

### Apply

Users can apply one selected preview match or an explicitly bounded set of
matches.

Apply:

- requires a rule ID;
- requires a match selector unless the command explicitly opts into applying all
  matches within bounds;
- enforces rule cost bounds before writing;
- reruns preconditions against the current graph instead of trusting stale
  preview output;
- applies to a cloned graph source first;
- runs parse -> build -> validate on the candidate;
- saves an undo snapshot of the original source before writing;
- writes the changed JSON-LD only after validation succeeds;
- returns rule ID, match ID, touched node IDs, operation summary, validation
  status, snapshot path, warnings, and explanation.

Apply must leave graph files unchanged when:

- the rule ID is unknown;
- the match selector is unknown or stale;
- rule preconditions fail;
- cost bounds are exceeded;
- candidate patch application fails;
- parsing fails;
- graph construction fails;
- validation emits diagnostics;
- the snapshot cannot be written;
- the final graph write fails.

### Safety Claims

Each rule declares a safety class:

- `structural-only`: changes metadata or structure without changing executable
  behavior under declared preconditions;
- `local-semantics-preserving`: changes executable graph shape while preserving
  behavior under declared local preconditions;
- `experimental`: available for preview only unless a later accepted spec
  changes the policy.

V1 apply supports only non-experimental rules. If an experimental rule exists,
it may be listed and previewed, but apply must reject it with an explicit
message.

### Evidence And Explanations

Each preview/apply result includes a human-readable explanation.

Explanations must answer:

- what rule matched;
- which nodes were inspected and touched;
- what graph change is proposed or applied;
- why the rule's preconditions were satisfied;
- what validation was run;
- what remains outside the safety claim.

JSON output must provide the same evidence in machine-readable fields.

### CLI Surface

V1 exposes a `duumbi rewrite` command group.

The exact flag names may be finalized in the technical spec, but the user
behavior must cover:

- list rules;
- preview one rule against a module;
- apply one selected match explicitly;
- output human-readable text by default;
- output JSON for automation;
- require confirmation unless `--yes` is supplied;
- use bounded limits for "apply all" behavior.

### MCP Surface

V1 exposes MCP rewrite tools using the same engine:

- a read-only rule list tool;
- a read-only preview tool;
- an explicit apply tool that is clearly described as write-capable.

MCP apply must enforce the same validation, snapshot, and cost-bound rules as
CLI apply.

### Interaction With Existing Mutation

Existing `duumbi add`, intent execution, and MCP `graph_mutate` continue to work
as they do today.

V1 may share internal helper code with `GraphPatch`, but it must not silently
route existing agent mutation through rewrite apply. Any future LLM-selected
rewrite flow needs a separate product decision or a Stage 10 implementation
finding explicitly accepted by review.

### Empty, Error, And Cancellation States

Empty states are explicit and successful when no mutation was requested:

- no registered rules;
- no matches for a selected rule;
- previewable rule with no apply-capable safety class.

Error states are explicit and non-mutating:

- missing workspace graph;
- malformed JSON-LD;
- parse/build/validation diagnostics;
- unknown rule;
- stale match selector;
- cost bound exceeded;
- unsupported rule safety class;
- snapshot failure;
- write failure.

Interactive cancellation before confirmation leaves graph files and history
unchanged and reports that no rewrite was applied.

### Performance And Bounds

Rule matching and apply are bounded.

V1 behavior must include:

- default maximum matches inspected per rule;
- default maximum matches applied per command;
- default maximum touched nodes or patch operations per apply;
- clear error or truncation messages when limits are reached;
- deterministic ordering for matches so repeated previews are stable for an
  unchanged graph.

The implementation may expose limit flags, but it must keep safe defaults.

### Accessibility And Output

CLI output must be readable without color.

Human-readable output should use compact headings and tables/lists, with JSON
available for automation. Long node lists should be bounded with explicit
truncation counts.

## BDD Scenarios

Feature: Semantic rewrite engine

  Rule: Rule discovery and read-only preview

    Scenario: List available rewrite rules
      Given a DUUMBI workspace with the V1 rewrite engine installed
      When the user lists rewrite rules
      Then DUUMBI shows each registered rule with ID, category, safety class,
      precondition summary, and effect summary
      And no workspace graph file is modified

    Scenario: Preview a matching rewrite without mutation
      Given a workspace graph that contains an i64 expression matching a
      registered local-semantics-preserving rewrite rule
      When the user previews that rule against the module
      Then DUUMBI returns at least one match with a stable match ID, touched
      node IDs, cost estimate, proposed effect, validation status, and
      explanation
      And `.duumbi/graph` and `.duumbi/history` remain unchanged

    Scenario: Preview a rule with no matches
      Given a valid workspace graph that does not match the selected rewrite
      rule
      When the user previews that rule
      Then DUUMBI reports zero matches as a successful read-only result
      And it does not create a snapshot or write graph files

  Rule: Explicit apply

    Scenario: Apply one selected rewrite match
      Given a valid workspace graph
      And a previewed rewrite match that still satisfies its preconditions
      When the user applies that match with explicit confirmation
      Then DUUMBI saves an undo snapshot of the original graph
      And it applies the rewrite to a cloned candidate
      And the candidate passes parse, semantic graph build, and validation
      And DUUMBI writes the updated JSON-LD graph
      And the result reports the rule ID, match ID, touched nodes, snapshot
      path, validation status, and explanation

    Scenario: Reject a stale match before writing
      Given a user has a preview match ID from an earlier graph state
      And the current graph no longer satisfies the rule preconditions for that
      match
      When the user tries to apply the stale match ID
      Then DUUMBI rejects the apply request with an explicit stale or missing
      match diagnostic
      And no graph file or snapshot is written

    Scenario: Validation failure prevents apply
      Given a rewrite rule implementation produces an invalid candidate graph
      When the user applies a matching rewrite
      Then DUUMBI reports the parse, build, or validation diagnostics
      And the original graph remains unchanged
      And no success evidence is emitted

    Scenario: Cost bounds prevent unbounded rewrite
      Given a workspace graph with more matches than the default apply bound
      When the user requests applying all matches without raising the bound
      Then DUUMBI rejects or truncates according to the documented bound policy
      And it reports the match count and required user action
      And it does not perform an unbounded graph mutation

  Rule: Shared human and agent interface

    Scenario: MCP preview uses the same engine as CLI preview
      Given a valid workspace graph and a selected rewrite rule
      When an MCP client calls the read-only rewrite preview tool
      Then the response contains the same rule ID, match evidence, safety
      class, and explanation shape as the CLI JSON preview
      And the workspace graph remains unchanged

    Scenario: MCP apply is explicitly write-capable
      Given an MCP client calls the rewrite apply tool for a valid selected
      match
      When the rewrite passes cost bounds and validation
      Then DUUMBI writes the graph only after saving a snapshot
      And the MCP response marks the tool result as a successful mutation with
      snapshot and validation evidence

  Rule: V1 boundaries

    Scenario: Experimental rewrite cannot be applied
      Given a registered experimental rewrite rule
      When the user tries to apply it
      Then DUUMBI rejects the apply request with a message that experimental
      rules are preview-only in V1
      And no graph files are changed

    Scenario: Existing agent mutation still works independently
      Given a workspace that can be changed through the existing `duumbi add`
      GraphPatch flow
      When the rewrite engine is installed
      Then the existing agent mutation flow still uses its current confirmation,
      snapshot, patch, and validation behavior unless the user explicitly runs
      `duumbi rewrite`

## Tasks

- Define the V1 rule catalog and safety-class contract.
- Add the core rewrite engine and preview/apply data model.
- Implement conservative built-in rules that prove matching, preview, apply,
  validation, explanation, and cost bounds.
- Add CLI `duumbi rewrite` list, preview, and apply behavior.
- Add MCP rewrite list, preview, and apply tools.
- Wire apply through snapshot creation and the existing validation pipeline.
- Add JSON output shapes for automation.
- Add docs describing the V1 safety contract and deferred V2/V3 capabilities.
- Add tests and manual smoke evidence mapped to the BDD scenarios.

Independent slices:

- Rule model and built-in matching can be implemented before CLI/MCP.
- CLI preview can be implemented before apply.
- MCP preview can reuse the same JSON output after CLI preview exists.
- Apply can be tested through the core API before interactive CLI confirmation.
- Docs can be finalized after command names and output fields settle.

## Checks

Required verification:

- Unit tests for rule metadata, safety classes, preconditions, deterministic
  match ordering, no-match behavior, cost bounds, and explanation output.
- Unit tests for built-in conservative rules proving both matching and
  non-matching cases.
- Apply tests proving graph files are unchanged on stale match, failed
  precondition, cost-bound failure, candidate patch failure, parse failure,
  graph build failure, validation diagnostics, and snapshot failure.
- Integration tests proving successful apply creates a snapshot, writes the
  graph, and the result passes the existing parser, graph builder, and
  validator.
- CLI tests for `rewrite list`, `rewrite preview`, JSON output, confirmation
  cancellation, and `--yes` apply.
- MCP tests for rewrite tool discovery, read-only preview, and write-capable
  apply.
- Regression tests proving existing `GraphPatch`, `duumbi add` parsing paths,
  and MCP graph tools remain available.
- Documentation review proving V1 does not claim e-graph optimization, formal
  proof, or autonomous self-evolution.

Manual smoke expectations:

- Create or use a small initialized workspace with a graph that matches a V1
  rewrite rule.
- Run rule list and preview from the CLI.
- Run apply with `--yes`.
- Run `duumbi check`.
- Run `duumbi build` or `duumbi run` when the fixture has an executable `main`.
- Exercise MCP `tools/list`, rewrite preview, and rewrite apply against a
  temporary workspace.

BDD coverage expectations:

- Discovery and preview scenarios are covered by rule catalog, CLI preview, and
  MCP preview tests.
- Apply scenarios are covered by core apply and CLI apply integration tests.
- Validation failure and stale match scenarios are covered by negative apply
  tests.
- Cost-bound behavior is covered by a fixture with more matches than the apply
  limit.
- V1 boundary scenarios are covered by experimental-rule and existing-mutation
  regression tests.

Expected artifacts:

- `specs/DUUMBI-684/PRODUCT.md`
- `specs/DUUMBI-684/TECHNICAL.md`
- Stage 10 implementation PR with source, tests, docs, and manual smoke
  evidence

## Open Questions

None blocking for V1.

Deferred, non-blocking future decisions:

- Which e-graph library or custom equivalence representation should be used for
  a later Phase 2 optimizer.
- Whether future LLM flows should select rewrite rules directly instead of
  emitting lower-level patch operations.
- How proof-carrying rewrite evidence should connect to future VCGen or formal
  verification work.
- Whether graph-aware registry reuse should store community rewrite rules after
  the local V1 rule contract is stable.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/684
- Stage 5 acceptance:
  https://github.com/hgahub/duumbi/issues/684#issuecomment-4699293580
- Source note:
  `Duumbi/05 Archive/Processed Inbox/2026-05-19 - Semantic Rewrite Engine Positioning.md`
- PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Future Development Roadmap Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Future Development Roadmap Map.md`
- Token-Efficient Graph Representation for LLM I/O:
  `Duumbi/00 Inbox (ToProcess)/2026-06-12 - Token-Efficient Graph Representation for LLM IO.md`
- Determinism Program for AI Development:
  `Duumbi/00 Inbox (ToProcess)/2026-06-12 - Determinism Program for AI Development.md`
- Token Economics Benchmark:
  `Duumbi/00 Inbox (ToProcess)/2026-06-12 - Token Economics Benchmark.md`
- Repo architecture reference: `docs/architecture.md`
- Repo coding conventions: `docs/coding-conventions.md`
- Query mode specification: `docs/modes/query-mode-spec.md`
- Relevant source files:
  - `src/patch.rs`
  - `src/graph/mod.rs`
  - `src/graph/validator.rs`
  - `src/mcp/server.rs`
  - `src/mcp/tools/graph.rs`
  - `src/agents/orchestrator.rs`
  - `src/hash.rs`
