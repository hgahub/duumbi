# DUUMBI-684: Semantic Rewrite Engine as Formal Graph Transformation Substrate - Technical Specification

## Implementation Objective

Implement the approved V1 product behavior in `specs/DUUMBI-684/PRODUCT.md` by
adding a conservative Semantic Rewrite Engine for deterministic, validated,
evidence-bearing graph-to-graph rewrites over DUUMBI JSON-LD semantic graph
modules.

V1 must provide:

- a shared backend rewrite engine with first-class rule metadata, deterministic
  matching, preview plans, apply plans, safety classes, cost limits, and
  explanation evidence;
- CLI `duumbi rewrite` rule discovery, read-only preview, and explicit apply
  behavior;
- MCP rule discovery, read-only preview, and write-capable apply tools backed by
  the same engine;
- snapshot-backed writes that only happen after candidate parse, semantic graph
  build, and validation succeed;
- tests and manual evidence proving no mutation on list, preview, no-match,
  stale match, validation failure, cost-bound failure, unsupported safety class,
  or cancellation paths.

Related to #684. This is a technical-specification artifact only. The execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Reviewer agents checking scope, rewrite safety claims, no-mutation guarantees,
  CLI/MCP contract consistency, and test coverage.
- Tester agents creating fixture-backed graph rewrite evidence.
- Docs agents documenting the V1 rewrite contract and deferred e-graph/autonomy
  work.
- Stage 9 technical reviewers checking implementability and resource policy.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/684
- Product spec: `specs/DUUMBI-684/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/704
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/684#issuecomment-4699293580
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Query mode specification: `docs/modes/query-mode-spec.md`
- Source intake note:
  `Duumbi/05 Archive/Processed Inbox/2026-05-19 - Semantic Rewrite Engine Positioning.md`
- PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Future Development Roadmap Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Future Development Roadmap Map.md`

Relevant source facts verified for Stage 8:

- `src/patch.rs`
  - Defines `GraphPatch` and `PatchOp`.
  - `apply_patch()` clones a raw JSON-LD `serde_json::Value`, applies all ops to
    the clone, and returns either a fully patched value or an error.
  - Existing patch operations include `ModifyOp`, `SetEdge`, `RemoveNode`, and
    `ReplaceBlock`, which are sufficient primitives for a conservative V1
    rewrite substrate when wrapped with rule-level preconditions and evidence.
  - `apply_patch()` does not parse, build, validate, snapshot, or write; callers
    own those safety boundaries.
- `src/main.rs`
  - `duumbi add` already saves an undo snapshot through `snapshot::save_snapshot`
    before writing a confirmed graph mutation.
  - `Commands::Add` and `Commands::Undo` are dispatched from the top-level CLI
    match.
  - `Commands::Mcp` starts the MCP server.
- `src/snapshot.rs`
  - Saves undo snapshots under `.duumbi/history/{N:06}.jsonld`.
  - `restore_latest()` implements LIFO undo behavior.
  - `snapshot_count()` reports available undo depth.
- `src/cli/mod.rs`
  - Defines top-level `Commands` through `clap`.
  - There is no existing `Rewrite` command group.
- `src/cli/commands.rs`
  - Provides shared parse/check/build helpers.
  - `parse_and_validate()` runs parser, graph builder, and validator, emits
    diagnostics, and returns a `SemanticGraph` for valid single-file input.
- `src/parser/mod.rs`
  - Parses JSON-LD into a typed module AST and reports `E003`, `E009`, and
    unknown-op diagnostics.
- `src/graph/mod.rs`
  - Defines `SemanticGraph` over `petgraph::stable_graph::StableGraph`.
  - Maintains a `node_map` from `NodeId` to `NodeIndex`.
  - Defines `GraphNode`, `GraphEdge`, `FunctionInfo`, and `BlockInfo`.
- `src/graph/builder.rs`
  - Builds the semantic graph in two passes: node creation and edge resolution.
  - Collects duplicate-ID, orphan-reference, and no-entry errors.
  - Provides `build_graph()` for standalone modules and
    `build_graph_no_call_check()` for library module contexts.
- `src/graph/validator.rs`
  - Validates structure, terminator position, cycles, types, return types,
    branch conditions, SSA dominance, branch targets, ownership, and
    Result/Option safety where applicable.
- `src/types.rs`
  - Defines stable newtypes such as `NodeId`, `BlockLabel`, `FunctionName`, and
    `ModuleName`.
  - Defines the current `Op` enum, including integer arithmetic, calls,
    branches, strings, arrays, structs, ownership, Result/Option, filesystem,
    HTTP, and DB operations.
- `src/mcp/server.rs`
  - Registers MCP tools from `McpServer::list_tools()`.
  - Dispatches `tools/call` through `dispatch_tool_call()`.
  - Wraps successful tool results as pretty JSON inside MCP text content.
- `src/mcp/tools/mod.rs`
  - Declares existing tool modules `build`, `deps`, `graph`, and `intent`.
- `src/mcp/tools/graph.rs`
  - Provides synchronous workspace filesystem tool handlers for query, mutate,
    validate, and describe.
  - `graph_mutate()` currently writes without creating an undo snapshot, so the
    rewrite apply tool must not copy that omission.
- `tests/integration_phase2.rs`
  - Contains fixture-backed `GraphPatch` integration tests using deterministic
    mock patches and parse/build validation, not live LLM calls.
- `tests/integration_phase12.rs`
  - Contains MCP `tools/list` and `tools/call` integration coverage suitable
    for rewrite MCP discovery and behavior tests.
- `Cargo.toml`
  - Already includes `petgraph`, `serde`, `serde_json`, `anyhow`, `thiserror`,
    `tempfile`, and `comfy-table`.
  - V1 should not need a new dependency. In particular, V1 must not add `egg`
    or another e-graph/equivalence-saturation dependency.

Verified product and vault context:

- Issue #684 is open with `accepted` and `needs-spec` labels.
- The Stage 5 decision comment dated 2026-06-13 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- The source note explicitly frames the rewrite engine as semantic graph-level
  transformation, not text or AST refactoring.
- The source note proposes staged work: Phase 1 rule DSL and graph-to-graph
  transformations, Phase 2 bounded e-graph equality saturation, and Phase 3
  autonomous intent-driven rewrite exploration.
- The PRD and Agentic Development Map emphasize read-only questions and
  evidence before mutation.
- `docs/modes/query-mode-spec.md` defines Query as read-only and explicitly not
  a mutation path.

Assumptions for implementation:

- V1 should introduce a new `src/rewrite/` module exported from `src/lib.rs`.
- V1 should use the existing JSON-LD source, parser, graph builder, validator,
  `GraphPatch`, and snapshot contracts instead of inventing a parallel graph
  persistence path.
- CLI is the canonical user-facing interface for live E2E; MCP mirrors the same
  backend behavior for external agents.
- V1 rewrite rules are provider-free and deterministic. Live LLM rule
  selection, rule synthesis, e-graph exploration, and autonomous rewrite
  campaigns require later product specs.
- Built-in V1 rules should favor simple pure i64 local rewrites with
  easy-to-review preconditions over broader optimization claims.

## Affected Areas

Expected Stage 10 implementation areas:

- Shared rewrite backend:
  - `src/rewrite/mod.rs`
  - `src/rewrite/rule.rs`
  - `src/rewrite/catalog.rs`
  - `src/rewrite/engine.rs`
  - `src/rewrite/evidence.rs`
  - `src/rewrite/error.rs`
- Library export:
  - `src/lib.rs`
- CLI command shape and dispatch:
  - `src/cli/mod.rs`
  - `src/cli/rewrite.rs`
  - `src/main.rs`
  - `src/cli/commands.rs` only if a reusable graph-load/validate helper is
    needed outside the current private CLI functions.
- MCP tool registration and dispatch:
  - `src/mcp/server.rs`
  - `src/mcp/tools/mod.rs`
  - `src/mcp/tools/rewrite.rs`
- Existing mutation primitives, if a narrow additive helper is needed:
  - `src/patch.rs`
  - `src/snapshot.rs`
- Tests:
  - unit tests in new `src/rewrite/*` modules;
  - unit tests in `src/cli/rewrite.rs` for argument-independent formatting or
    confirmation helpers;
  - MCP server/tool tests in `src/mcp/server.rs` and `src/mcp/tools/rewrite.rs`;
  - integration tests such as `tests/integration_duumbi684_rewrite.rs`;
  - focused MCP integration additions in `tests/integration_phase12.rs` if that
    remains the canonical MCP integration location.
- Documentation:
  - `docs/rewrite-engine.md`
  - `docs/architecture.md` only for a short architecture-map update if Stage 10
    reviewers want the new module shown in the pipeline.
  - `README.md` only if the public command summary needs a minimal rewrite
    command mention.

Areas that must not change during Stage 8:

- implementation source files
- tests
- generated outputs
- runtime assets
- specs for other issues

Areas expected not to change during Stage 10:

- Cranelift lowering semantics
- parser support for new JSON-LD op types
- existing `Op` semantics
- registry resolution, lockfile, vendor, publish, and yank behavior
- provider setup, model catalog, model routing, benchmark, and live LLM
  behavior
- `duumbi add`, intent execution, and MCP `graph_mutate` behavior except for
  shared helper extraction that preserves their observable contracts
- Query mode read-only behavior
- Studio UI unless a thin smoke check is needed to prove no accidental rewrite
  UI exposure

## Technical Approach

### Backend Module

Create a `rewrite` module that owns the rule model, catalog, matching, preview,
apply, and evidence contracts. Keep CLI and MCP code thin.

Recommended module shape:

```text
src/rewrite/
  mod.rs        # public API and exports
  rule.rs       # rule metadata, SafetyClass, RewriteRule trait
  catalog.rs    # built-in rule registration and lookup
  engine.rs     # preview/apply orchestration and bounds
  evidence.rs   # preview/apply result DTOs
  error.rs      # RewriteError
```

The backend should expose APIs similar to:

```rust
pub struct RewriteEngine {
    catalog: RewriteCatalog,
    limits: RewriteLimits,
}

pub fn list_rules(&self) -> Vec<RuleSummary>;
pub fn preview_module(&self, source: &serde_json::Value, options: PreviewOptions)
    -> Result<RewritePreview, RewriteError>;
pub fn apply_to_module(&self, source: &serde_json::Value, options: ApplyOptions)
    -> Result<RewriteApplyPlan, RewriteError>;
```

Names may change during implementation, but the layering must remain:

1. CLI/MCP loads a workspace module source.
2. Backend parses and builds semantic graph context for matching.
3. Backend finds deterministic matches and creates preview evidence.
4. Backend creates candidate patch or candidate source in memory only.
5. Backend validates the candidate through parse -> build -> validate before
   declaring it apply-ready.
6. CLI/MCP save an undo snapshot and write only after the backend returns a
   valid apply result and the user or tool explicitly requested mutation.

Do not let CLI and MCP implement separate matching logic. Differences between
human-readable CLI text and MCP JSON belong at the adapter layer only.

### Rule Model

Each rule must have stable metadata:

- `id`: stable kebab-case string, for example `i64-add-zero-right`;
- `display_name`;
- `category`, such as `canonicalization`, `local-optimization`, or
  `structural-cleanup`;
- `safety_class`: `structural-only`, `local-semantics-preserving`, or
  `experimental`;
- `description`;
- `preconditions`;
- `effect_summary`;
- `default_cost`;
- `explanation_template`;
- `apply_capable`: false for experimental rules.

Use Rust enums for safety class and category. Serialize them to stable
snake-case or kebab-case strings for CLI JSON and MCP output. Avoid raw string
typing inside the engine except at serde/API boundaries.

The rule trait should separate matching from patch production:

- `find_matches(context, limits) -> Vec<RewriteMatch>`
- `build_patch(source, match) -> GraphPatch` or equivalent candidate transform
- `explain(match, candidate) -> RewriteExplanation`

The implementation may start with `GraphPatch` as the candidate representation.
If a rule needs a more structured internal transform, it must still convert to a
validated candidate JSON-LD value before snapshot/write.

### Built-In V1 Catalog

Implement a small conservative built-in catalog. The exact final rule IDs may
be adjusted during Stage 10, but V1 must include enough rules to prove:

- rule discovery;
- one or more matching previews;
- one successful apply;
- no-match behavior;
- validation failure behavior through a test-only rule or injectable test
  catalog;
- experimental preview-only rejection.

Recommended built-in apply-capable rules:

| Rule ID | Safety class | Preconditions | Effect |
| --- | --- | --- | --- |
| `i64-add-zero-right` | `local-semantics-preserving` | `duumbi:Add` result is `i64`; right operand is a `duumbi:Const` i64 value `0`; left operand result type is `i64`; replacement preserves downstream references. | Replace the add result with the left operand or rewrite the containing block so downstream users observe the left operand value. |
| `i64-add-zero-left` | `local-semantics-preserving` | Same as above with the zero constant on the left. | Replace the add result with the right operand or rewrite the containing block so downstream users observe the right operand value. |
| `i64-mul-one-right` | `local-semantics-preserving` | `duumbi:Mul` result is `i64`; right operand is a `duumbi:Const` i64 value `1`; left operand result type is `i64`; replacement preserves downstream references. | Replace the multiply result with the left operand or rewrite the containing block so downstream users observe the left operand value. |

Recommended preview-only experimental rule:

| Rule ID | Safety class | Preconditions | Effect |
| --- | --- | --- | --- |
| `experimental-fold-i64-const-add` | `experimental` | Both operands to `duumbi:Add` are i64 constants. | Preview a constant-folding candidate, but reject apply in V1. |

Implementation agents may choose a smaller or different conservative catalog
when justified by source constraints, but must keep at least one apply-capable
local i64 rule and one experimental preview-only rule. Any broader rule that
touches calls, branches, ownership, Result/Option, filesystem, HTTP, DB, or
heap/string semantics requires a Stage 10 finding and reviewer acceptance before
implementation.

### Matching And IDs

Matching must be deterministic for an unchanged graph.

Use a stable ordering based on:

1. module path or module name;
2. function order from `FunctionInfo`;
3. block order from `BlockInfo`;
4. op order inside the block;
5. rule ID as a final tiebreaker when needed.

Match IDs must be stable enough for preview/apply in the same graph state and
should remain stable across repeated previews of unchanged input. A safe shape
is:

```text
<rule-id>:<module-name>:<primary-node-id>:<ordinal>
```

If node IDs contain characters that are inconvenient for CLI/MCP clients, add a
stable encoded form while still returning the touched node IDs separately.

Apply must not trust the preview object supplied by a caller. It must rerun
matching against the current graph, find the requested match ID, rerun
preconditions, and reject missing or changed matches as stale.

### Bounds And Cost Model

Add a `RewriteLimits` type with safe defaults:

- `max_matches_per_preview`: 100
- `max_matches_per_apply`: 1 unless `--all` or equivalent is explicitly set
- `max_apply_all_matches`: 10
- `max_touched_nodes_per_match`: 25
- `max_patch_ops_per_match`: 25
- `max_explanation_nodes`: 20 before truncation

The product contract allows different exact values if Stage 10 documents why,
but all limits must be explicit, testable, and represented in JSON output.

Cost evidence should include at least:

- `matches_considered`
- `matches_returned`
- `matches_truncated`
- `touched_node_count`
- `patch_op_count`
- `estimated_cost_units`
- `limits`

Cost-bound failure must happen before snapshot/write.

### Validation And Write Sequence

For preview:

1. Load the JSON-LD source.
2. Parse JSON-LD.
3. Build `SemanticGraph`.
4. Run validation on the current graph.
5. Match rules only when the current graph is valid.
6. Build candidate patches or candidate source in memory when practical.
7. Validate candidate source when practical.
8. Return evidence and warnings.
9. Write nothing.

For apply:

1. Load the JSON-LD source.
2. Parse JSON-LD.
3. Build `SemanticGraph`.
4. Run validation on the current graph.
5. Rerun matching and preconditions.
6. Enforce safety class and cost bounds.
7. Apply patch to a clone or produce candidate source.
8. Serialize candidate JSON-LD.
9. Parse candidate JSON-LD.
10. Build candidate `SemanticGraph`.
11. Run validator on the candidate graph.
12. Save undo snapshot of the original source.
13. Write pretty JSON-LD only after snapshot succeeds.
14. Return structured apply evidence.

Snapshot failure must prevent the final graph write. Final write failure must
return an explicit error; if the implementation can safely detect a partial
write risk, it should use write-to-temp plus rename for atomicity.

### CLI Surface

Add a top-level command group:

```text
duumbi rewrite list [--json]
duumbi rewrite preview --module <name-or-path> --rule <rule-id> [--json] [--limit <n>]
duumbi rewrite apply --module <name-or-path> --rule <rule-id> --match <match-id> [--yes] [--json]
duumbi rewrite apply --module <name-or-path> --rule <rule-id> --all --max-matches <n> [--yes] [--json]
```

Default module resolution:

- If `--module` is omitted, default to workspace `.duumbi/graph/main.jsonld`.
- If `--module main` is supplied, resolve to `.duumbi/graph/main.jsonld`.
- If `--module <path>.jsonld` is supplied, use that explicit path.
- Reject paths outside the workspace unless existing CLI conventions already
  permit explicit external graph files for check/build. If external files are
  accepted, apply must still snapshot/write the exact source file and document
  where the snapshot is stored.

Human-readable output:

- must not rely on color;
- must show rule ID, safety class, match count, touched nodes, cost summary, and
  validation status;
- must bound long node lists with explicit truncation counts;
- must show `No matches` as a successful preview state;
- must show `No rewrite applied` for cancellation.

JSON output:

- must use the same backend DTOs as MCP output where possible;
- must include `status`, `rule`, `matches`, `cost`, `validation`, `warnings`,
  and `explanation` fields for preview;
- must include `status`, `rule`, `match_id`, `snapshot_path`, `validation`,
  `operation_summary`, `warnings`, and `explanation` fields for apply.

Interactive apply requires confirmation unless `--yes` is supplied. Declining
confirmation must not create a snapshot.

### MCP Surface

Add three MCP tools backed by the same engine:

| Tool | Capability |
| --- | --- |
| `rewrite_list_rules` | Read-only rule discovery. |
| `rewrite_preview` | Read-only preview for one rule and module. |
| `rewrite_apply` | Explicit write-capable apply for one match or bounded all-matches request. |

Tool descriptions must clearly state read-only or write-capable behavior.
Because the current MCP server does not expose tool annotations, the
descriptions and schemas are the required V1 safety signal.

Recommended inputs:

- `rewrite_list_rules`
  - no required fields;
  - optional `include_experimental: bool`.
- `rewrite_preview`
  - required `rule_id: string`;
  - optional `module: string`, default `main`;
  - optional `limit: integer`, bounded by `RewriteLimits`.
- `rewrite_apply`
  - required `rule_id: string`;
  - optional `module: string`, default `main`;
  - either `match_id: string` or `all: true`;
  - optional `max_matches: integer`, bounded by `RewriteLimits`;
  - no interactive confirmation because MCP apply is already the explicit
    write-capable tool call.

Reject unknown fields through JSON Schema `additionalProperties: false` and
handler-side validation. Return tool errors rather than panics for wrong types,
unknown rule IDs, missing match selectors, stale matches, unsupported safety
classes, cost-bound failures, validation failures, snapshot failures, and write
failures.

### Existing Mutation Compatibility

Do not silently route existing mutation flows through the rewrite engine.

Required compatibility:

- `duumbi add` continues to use provider-backed GraphPatch generation and its
  existing confirmation/snapshot/write behavior.
- `duumbi intent execute` continues to use its current mutation and verification
  flow.
- MCP `graph_mutate` continues to accept GraphPatch ops exactly as before.
- Query mode remains read-only.

Sharing helper code is allowed when it reduces duplication and preserves
observable behavior. For example, extracting a reusable "load JSON-LD,
parse/build/validate candidate" helper is acceptable. Replacing existing
behavior or changing existing error text without tests is not.

### Documentation

Add `docs/rewrite-engine.md` covering:

- V1 purpose and non-goals;
- rule metadata and safety classes;
- built-in rule catalog;
- CLI examples;
- MCP tool examples;
- preview/apply evidence fields;
- snapshot and validation guarantees;
- cost bounds and apply-all policy;
- stale match behavior;
- experimental preview-only rules;
- relationship to `GraphPatch`, `duumbi add`, Query mode, and future e-graph
  work.

The docs must explicitly say V1 does not provide:

- e-graph equality saturation;
- formal proof generation;
- SMT/VCGen proof-carrying rewrites;
- autonomous self-evolution;
- live LLM rule selection or rule synthesis.

## Invariants

- List and preview are read-only and never write graph files, snapshots, config,
  credentials, registry cache, intent files, or telemetry.
- Apply writes only after candidate parse, graph build, and validation succeed.
- Apply saves an undo snapshot of the original graph source before writing.
- Snapshot failure prevents graph write.
- Apply reruns matching and preconditions against the current graph; stale
  preview IDs are rejected.
- Experimental rules are preview-only in V1.
- Cost bounds are enforced before mutation.
- Rule matching order is deterministic for unchanged input.
- CLI and MCP use the same backend rule catalog, matching, validation, and
  evidence DTOs.
- Existing `GraphPatch`, `duumbi add`, intent execution, Query mode, registry,
  compiler, and Studio behavior remain unchanged unless explicitly routed
  through `duumbi rewrite` or rewrite MCP tools.
- V1 does not add e-graph, theorem-proving, SMT, provider, or rule-synthesis
  dependencies.
- All public Rust items added for this issue have doc comments, use `#[must_use]`
  where appropriate, avoid `.unwrap()` in library code, and follow repository
  error-handling conventions.
- CI tests are deterministic and use fixtures, temp directories, and mocks only.
  Live provider keys are not required.

## BDD-To-Test Mapping

| Product BDD scenario | Required verification evidence |
| --- | --- |
| List available rewrite rules | Unit test for catalog metadata and CLI integration test for `duumbi rewrite list --json`; asserts rule ID, category, safety class, precondition summary, effect summary, and unchanged `.duumbi/graph` and `.duumbi/history`. MCP `tools/list` test asserts `rewrite_list_rules` is discoverable and marked read-only in description. |
| Preview a matching rewrite without mutation | Core engine unit test and CLI integration test using an i64 fixture that matches an apply-capable rule; asserts at least one match, deterministic match ID, touched node IDs, cost estimate, validation status, explanation, and byte-identical graph/history directories after preview. |
| Preview a rule with no matches | Core unit test and CLI/MCP test using a valid graph that lacks the selected pattern; asserts successful zero-match response, no snapshot, and no graph write. |
| Apply one selected rewrite match | Integration test creates a temp workspace, previews a matching rule, applies the selected match with `--yes`, asserts one snapshot is created, candidate passes parser/builder/validator, graph JSON-LD changes as expected, and apply evidence includes rule ID, match ID, touched nodes, snapshot path, validation status, and explanation. |
| Reject a stale match before writing | Integration test previews a match, mutates the fixture or applies a different safe change to invalidate the match, then attempts apply with the old match ID; asserts stale or missing match diagnostic, no new snapshot, and graph bytes unchanged from the current pre-apply state. |
| Validation failure prevents apply | Unit test uses an injectable test-only rule catalog or test rule that intentionally produces an invalid candidate; asserts parse/build/validation diagnostics are returned, original graph remains unchanged, no success evidence is emitted, and no snapshot is created. |
| Cost bounds prevent unbounded rewrite | Unit/integration test uses a fixture with more matches than the default apply-all limit; calls apply-all without raising the bound; asserts rejection or documented truncation behavior, reported match count and required action, and no unbounded mutation. Preferred V1 behavior is rejection. |
| MCP preview uses the same engine as CLI preview | Integration test calls CLI preview JSON and MCP `rewrite_preview` against the same fixture; normalizes adapter-only fields and asserts rule ID, match IDs, safety class, touched nodes, validation status, cost shape, and explanation shape match. |
| MCP apply is explicitly write-capable | MCP integration test calls `rewrite_apply` for one valid match; asserts snapshot creation, graph write after validation, MCP result marks mutation success, and evidence includes snapshot and validation fields. Tool description test asserts write-capable wording is present. |
| Experimental rewrite cannot be applied | Unit and CLI/MCP tests register or use an experimental rule; preview succeeds or returns preview-only evidence, apply rejects with unsupported safety class or preview-only message, and no graph/snapshot changes occur. |
| Existing agent mutation still works independently | Regression test runs existing `GraphPatch` apply or `duumbi add` mock-path coverage unchanged; MCP `graph_mutate` tests continue to pass. Review evidence confirms no `duumbi add`, intent execution, or Query mode dispatch was silently routed through rewrite apply. |

Additional required coverage:

- Unit tests for rule metadata serialization and deserialization.
- Unit tests for deterministic match ordering and repeated-preview stability.
- Unit tests for unknown rule, missing module, malformed JSON-LD, invalid
  current graph, unknown match selector, unsupported safety class, snapshot
  failure, and final write failure where feasible.
- CLI parser tests for `rewrite list`, `rewrite preview`, `rewrite apply
  --match`, `rewrite apply --all`, `--json`, `--yes`, `--limit`, and
  `--max-matches`.
- MCP schema tests for required fields, rejected unknown fields, and invalid
  types.
- Documentation review proving V1 does not claim e-graph optimization, formal
  proof, or autonomous self-evolution.

## Live E2E Plan

Canonical interface: CLI `duumbi rewrite`, because V1 is a user-visible
workspace graph workflow.

External LLM/provider path:

- V1 rewrite engine is deterministic and provider-free.
- Required credentials: none.
- Expected external LLM calls for required E2E: 0.
- Estimated external LLM cost for required E2E: USD 0.
- No live LLM-backed E2E is required because this issue does not implement LLM
  behavior. If Stage 10 discovers that live LLM rule selection or rule synthesis
  is needed, stop for product/architecture approval.

Manual CLI E2E outline:

1. Build the debug binary:

   ```text
   cargo build
   ```

2. Create a disposable workspace:

   ```text
   tmpdir=$(mktemp -d)
   cd "$tmpdir"
   /path/to/target/debug/duumbi init rewrite-smoke
   cd rewrite-smoke
   ```

3. Edit or copy a small graph fixture so `.duumbi/graph/main.jsonld` contains an
   i64 expression matching one V1 apply-capable rewrite rule.

4. List rules:

   ```text
   /path/to/target/debug/duumbi rewrite list --json
   ```

5. Preview one rule:

   ```text
   /path/to/target/debug/duumbi rewrite preview --module main --rule i64-add-zero-right --json
   ```

6. Apply one selected match:

   ```text
   /path/to/target/debug/duumbi rewrite apply --module main --rule i64-add-zero-right --match <match-id> --yes --json
   ```

7. Verify the graph remains valid:

   ```text
   /path/to/target/debug/duumbi check
   ```

8. Build or run when the fixture has executable `main` behavior:

   ```text
   /path/to/target/debug/duumbi build
   /path/to/target/debug/duumbi run
   ```

CLI pass criteria:

- rule list includes the expected built-in V1 rules;
- preview returns match evidence and writes nothing;
- apply creates one undo snapshot before writing;
- apply result includes rule ID, match ID, touched nodes, validation status,
  snapshot path, cost, and explanation;
- `duumbi check` passes after apply;
- `duumbi undo` restores the pre-apply graph when included in the smoke path.

Manual MCP E2E outline:

1. Start MCP from the same disposable workspace:

   ```text
   /path/to/target/debug/duumbi mcp
   ```

2. Send JSON-RPC requests:

   ```json
   {"jsonrpc":"2.0","id":1,"method":"tools/list"}
   ```

   ```json
   {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"rewrite_preview","arguments":{"module":"main","rule_id":"i64-add-zero-right"}}}
   ```

   ```json
   {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"rewrite_apply","arguments":{"module":"main","rule_id":"i64-add-zero-right","match_id":"<match-id>"}}}
   ```

MCP pass criteria:

- rewrite tools are discoverable;
- list and preview tool descriptions are read-only;
- apply tool description is explicitly write-capable;
- preview evidence matches CLI JSON shape after adapter normalization;
- apply saves a snapshot, validates, writes, and reports mutation evidence.

Stage 10 PR evidence must include:

- commands run;
- fixture path or inline fixture description;
- external LLM call count, expected to be `0`;
- cost estimate, expected to be `USD 0`;
- CLI output snippets with secrets absent;
- MCP request/response transcript or summarized log.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop only if requirements are met, a blocker appears, the expected external
   LLM cost of the next cycle exceeds USD 1, scope changes, or a product or
   architecture decision is needed; iteration count is not a stop condition

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 5 implementation modules plus directly
  associated tests/docs.
- Expected command budget per cycle:
  - `cargo fmt --check`
  - focused `cargo test` for edited modules or integration tests
  - `cargo clippy --all-targets -- -D warnings` when public APIs, CLI surfaces,
    MCP surfaces, or shared modules change
- Full pre-review command budget:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --all`
- Human approval required only when the next cycle will use an external LLM with
  expected cost above USD 1, exceeds approved scope, adds risky dependencies or
  irreversible operations, touches migrations/security-sensitive behavior,
  changes existing mutation semantics, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is covered by the Codex
  App subscription and never triggers the gate.
- Issue-specific required live E2E budget: 0 external LLM calls and USD 0.
- Optional live provider work: not in V1 scope. If proposed, stop for approval.
- CI external network budget: zero live provider calls and zero provider cost.
- No autonomous batch cap: cycles continue until completion, blocker, gate
  breach, or scope change.
- When to stop and ask for human guidance:
  - an e-graph, theorem-proving, SMT, or rule-synthesis dependency becomes
    necessary;
  - a rule must touch calls, branches, ownership, Result/Option, filesystem,
    HTTP, DB, heap, string, or array semantics to satisfy V1;
  - existing `duumbi add`, intent execution, Query mode, or MCP `graph_mutate`
    behavior would need observable changes;
  - snapshot/write atomicity cannot be implemented safely with the current
    filesystem helpers;
  - CLI and MCP cannot share one backend evidence shape;
  - a product behavior conflict appears between preview evidence, cost bounds,
    and apply-all behavior.

## Task Breakdown

1. Backend skeleton and data contracts
   - Add `src/rewrite/` module and export it from `src/lib.rs`.
   - Define `RuleSummary`, `SafetyClass`, `RewriteLimits`, `RewriteMatch`,
     `RewritePreview`, `RewriteApplyPlan`, `RewriteResult`, and `RewriteError`.
   - Add serialization tests for public DTOs.

2. Rule catalog and deterministic matching
   - Add built-in rule registration.
   - Implement deterministic traversal over validated semantic graph context.
   - Implement at least one apply-capable local i64 rule and one experimental
     preview-only rule.
   - Add unit tests for metadata, lookup, no rules, unknown rule, no matches,
     matching cases, and deterministic ordering.

3. Candidate generation and validation
   - Convert selected matches into `GraphPatch` or candidate JSON-LD source.
   - Validate candidates through parse -> build -> validate.
   - Add test-only invalid candidate support for validation failure tests.
   - Add tests for candidate validation, patch-op counts, touched node counts,
     and error mapping.

4. Apply orchestration and snapshot boundary
   - Implement apply rerun of matching/preconditions against current graph.
   - Enforce safety class and cost bounds.
   - Save snapshot before final write.
   - Add tests for successful apply, stale match, cost failure, snapshot
     failure, final write failure where feasible, and undo compatibility.

5. CLI command group
   - Add `Rewrite` to `Commands`.
   - Add `RewriteSubcommand`.
   - Implement `src/cli/rewrite.rs` adapter for list, preview, and apply.
   - Add CLI parser tests and integration tests using temp workspaces.

6. MCP tool surface
   - Add `src/mcp/tools/rewrite.rs`.
   - Register `rewrite_list_rules`, `rewrite_preview`, and `rewrite_apply` in
     `McpServer::list_tools()`.
   - Dispatch the tools from `dispatch_tool_call()`.
   - Add schema, discovery, read-only wording, write-capable wording, and
     `tools/call` tests.

7. Compatibility and no-mutation regressions
   - Add regression evidence for `GraphPatch`, existing MCP graph tools,
     `duumbi add` mock paths where available, intent execution paths where
     feasible, and Query mode no-write assumptions.
   - Snapshot graph/history/config/intent paths before and after list/preview.

8. Documentation and architecture notes
   - Add `docs/rewrite-engine.md`.
   - Optionally add a short architecture reference update.
   - Ensure docs distinguish V1 from e-graphs, proofs, and autonomous rewrites.

9. Verification and manual smoke
   - Run focused tests while building.
   - Run full pre-review command budget.
   - Run manual CLI smoke.
   - Run manual MCP smoke.
   - Record evidence in the implementation PR.

## Verification Plan

Required local verification before implementation PR review:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`
- Focused rewrite backend unit tests.
- Focused CLI parser and integration tests for `duumbi rewrite`.
- Focused MCP server/tool tests for rewrite tools.
- Integration tests proving list/preview no-mutation behavior.
- Integration tests proving apply snapshot/write/validate behavior.
- Regression tests proving existing `GraphPatch`, `duumbi add` mock paths,
  intent execution paths, MCP graph tools, and Query mode assumptions remain
  intact where directly affected.
- Documentation review confirming V1 safety claims and non-goals.

Manual verification before implementation PR review:

- CLI smoke for list, preview, apply, check, optional build/run, and undo.
- MCP smoke for `tools/list`, `rewrite_preview`, and `rewrite_apply`.
- Manual evidence that required external LLM calls were zero.

## Completion Criteria

The implementation is complete only when:

- `duumbi rewrite list` works in human-readable and JSON modes.
- `duumbi rewrite preview` returns deterministic match evidence and writes
  nothing.
- `duumbi rewrite apply` applies one explicitly selected match only after
  validation and snapshot creation.
- Apply-all behavior is bounded and rejects or safely caps requests above the
  documented limit.
- Unknown rule, no match, stale match, validation failure, cost-bound failure,
  unsupported safety class, cancellation, snapshot failure, and write failure
  paths are explicit and non-mutating where mutation has not already been
  safely committed.
- MCP exposes `rewrite_list_rules`, `rewrite_preview`, and `rewrite_apply`.
- MCP preview/list behavior is read-only.
- MCP apply is explicitly write-capable and enforces the same validation,
  snapshot, safety class, and cost-bound rules as CLI apply.
- CLI JSON and MCP result shapes are materially the same after adapter-only
  wrapping differences.
- Built-in rules include at least one apply-capable conservative local i64 rule
  and one experimental preview-only rule.
- Existing `GraphPatch`, `duumbi add`, intent execution, Query mode, MCP graph
  tools, registry, compiler, and Studio behavior are not regressed.
- Docs explain V1 usage, evidence, safety classes, bounds, snapshots, and
  deferred e-graph/autonomous work.
- Required tests and checks pass, or any environmental failure is isolated and
  documented with focused passing evidence.
- Manual CLI and MCP smoke evidence is recorded in the implementation PR.

## Failure And Escalation

- If a planned built-in rule cannot preserve downstream references safely,
  reduce the rule catalog to a simpler conservative rule and record the
  rejected rule as non-blocking future work.
- If `GraphPatch` cannot express a safe candidate cleanly, use an internal
  candidate-source transform but keep the same validation and snapshot boundary.
- If candidate validation requires changing parser, graph builder, validator,
  or `Op` semantics, stop for Stage 9/product-scope review unless the change is
  strictly additive and already implied by the accepted spec.
- If CLI and MCP output shapes drift, consolidate in backend DTOs before adding
  more adapter formatting.
- If any preview/list path writes to graph, history, config, intent, registry,
  credential, or telemetry paths, treat it as a blocking bug.
- If a new dependency is proposed, reject it by default. Stop for approval if
  the dependency is needed for correctness rather than convenience.
- If live provider/LLM behavior appears necessary, stop for approval because V1
  is provider-free.
- If GitHub Actions or local full-suite checks fail, implementation agents must
  triage whether the failure is caused by this change, patch within scope, and
  rerun focused checks before claiming completion.

## Open Questions

None blocking for Stage 10 implementation.

Non-blocking follow-up candidates:

- Add Phase 2 e-graph equality saturation after the V1 rule and evidence
  substrate is accepted.
- Add proof-carrying rewrites or SMT/VCGen integration after a formal
  verification product decision.
- Let future LLM flows select existing rewrite rules instead of emitting lower
  level patches.
- Store community rewrite rules in the registry after local rule metadata and
  safety contracts stabilize.
