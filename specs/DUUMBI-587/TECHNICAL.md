# DUUMBI-587: Validate Repair Patches And Produce Human-Reviewable Evidence - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-587/PRODUCT.md` by adding
a deterministic repair-patch validation guardrail and a bounded evidence report.

The implementation must prove this flow:

```text
mapped repair crash context + proposed GraphPatch + source graph ->
parse patch -> apply patch to a clone -> parse/build/validate patched graph ->
native rebuild -> relevant tests -> human-reviewable evidence
```

This issue validates a proposed repair candidate. It does not generate repair
patches, call an LLM provider, execute autonomous retries, silently write the
candidate into source, accept a repair, deploy a repair, hot-swap a binary, or
start a production self-healing workflow. Even when every local gate passes, the
repair remains pending human review and `accepted_for_application` remains
false.

This technical spec PR is related to #587 and must leave the execution issue
open for Stage 9 approval, implementation, review, and Stage 12 closure
evidence.

## Agent Audience

- Codex implementation agents running bounded Stage 10 Ralph cycles.
- Stage 10 tester and reviewer agents validating gate coverage and evidence.
- Rust telemetry agents working in `src/telemetry/mod.rs`.
- CLI agents adding or verifying a local repair-validation command surface.
- Graph, compiler, and workspace agents reusing parser, builder, validator, and
  rebuild APIs without broadening repair behavior.
- Reviewer agents checking that no product spec, implementation code outside the
  approved surface, generated artifact, runtime asset, or provider behavior is
  changed without approval.

## Source Context

- Product spec: `specs/DUUMBI-587/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/652
- Stage 7 product spec approval:
  https://github.com/hgahub/duumbi/issues/587#issuecomment-4609819934
- GitHub issue: https://github.com/hgahub/duumbi/issues/587
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/587#issuecomment-4606336397
- Stage 4 triage context:
  https://github.com/hgahub/duumbi/issues/587#issuecomment-4601062097
- Parent issue: https://github.com/hgahub/duumbi/issues/580
- Parent product spec: `specs/DUUMBI-580/PRODUCT.md`
- Parent technical spec: `specs/DUUMBI-580/TECHNICAL.md`
- Controlled crash proof spec: `specs/DUUMBI-586/TECHNICAL.md`
- Repair-agent input contract spec: `specs/DUUMBI-588/TECHNICAL.md`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant code verified for Stage 8:

- `src/telemetry/mod.rs`
  - Defines `RepairCrashContext`, `RepairValidationGate`,
    `RepairValidationGateEvidence`, `RepairValidationEvidence`,
    `required_repair_validation_gates()`, `parse_repair_graph_patch()`, and
    `repair_validation_evidence_from_graph_patch()`.
  - Current required gates are GraphPatch parse, atomic patch application,
    graph parse, graph validation, native rebuild, and relevant tests.
  - Current `RepairValidationEvidence` sets `requires_human_review = true` and
    `accepted_for_application = false`.
  - Current `GraphValidation` gate combines graph build and semantic validation
    in one enum variant. The approved product spec makes graph build failure a
    product-visible state, so Stage 10 must either add a distinct `GraphBuild`
    gate or add an explicit graph-build sub-result while preserving serialized
    evidence compatibility.
  - Unit tests already cover serializable repair crash context, human-review
    required evidence, all required gates, and invalid patch parsing.
- `src/patch.rs`
  - Defines `GraphPatch`, `PatchOp`, and `apply_patch()`.
  - `apply_patch()` clones the input JSON-LD value, applies operations to the
    clone, returns the patched clone on success, and leaves the original value
    unchanged on failure.
  - Unit tests already include `patch_is_all_or_nothing_on_error`.
- `src/mcp/tools/graph.rs`
  - `graph_mutate()` parses patch operations, applies the patch, runs
    parse/build/validate before writing, and writes only after validation.
  - `graph_validate()` runs parse/build/validate read-only.
  - #587 must reuse these validation semantics but must not write source.
- `src/cli/mod.rs` and `src/main.rs`
  - Current telemetry command surface includes `duumbi telemetry inspect`.
  - No repair-validation CLI command exists yet.
- `src/cli/commands.rs`
  - `duumbi build --trace` and single-file build behavior use existing
    parser/build/compiler/linker paths.
- `src/workspace.rs`
  - Provides workspace build/run helpers for workspace-level rebuild behavior.
- `tests/integration_telemetry.rs`
  - Contains controlled traced crash integration coverage and default untraced
    negative coverage.
  - Reuses `tests/fixtures/telemetry/option_none_unwrap.jsonld` and
    `tests/fixtures/telemetry/call_then_panic.jsonld`.
- `src/intent/execute.rs`
  - Contains verifier-failure repair precedent that can call a provider and
    write repaired graph files. That path is related precedent only and must not
    be reused as the #587 runtime-crash repair-validation implementation.

Relevant Obsidian context verified for Stage 8:

- `DUUMBI - PRD.md`
  - The first product promise is local developer/test runtime failure feedback,
    not automatic production self-repair.
  - Any repair must pass graph validation, rebuild, tests, and human review
    before acceptance.
- `Runtime Failure Feedback Loop.md`
  - Runtime failure feedback is local developer/test evidence first.
  - Repair acceptance remains behind graph validation, rebuild, relevant tests,
    and explicit human review.
- `DUUMBI - Service and Research Direction.md`
  - The local Phase 13 foundation is partial and includes trace/crash artifacts,
    back-mapping, repair crash context, and validation evidence contracts.
  - Production customer self-healing remains later research and requires
    consent, telemetry ingestion, account identity, release delivery, rollback,
    and operational safety.

Verified source facts:

- Existing graph mutation already has a parse/build/validate-before-write
  precedent.
- Existing telemetry repair validation evidence is intentionally not an
  acceptance record.
- Existing #586 and #588 technical specs keep Phase 13 local, deterministic,
  provider-independent, and artifact-backed.
- Default untraced runs must remain separate from traced telemetry and repair
  evidence.

Assumptions for implementation:

- The canonical v1 user-facing surface should be a local CLI command:
  `duumbi telemetry repair-validate`.
- The command should emit pretty JSON to stdout by default and write the same
  JSON only when `--output <path>` is explicitly supplied.
- Candidate validation should operate on an in-memory or temporary clone of the
  source graph. The original graph source must not be changed by validation,
  whether gates pass or fail.
- Relevant tests need an explicit or conservative local test plan. A candidate
  cannot be locally valid when the test gate is skipped or unavailable.
- Stage 10 may support both single-file and workspace graph sources when it can
  do so locally and safely. If workspace support requires a broader project
  indexing or mutation design, Stage 10 must stop and request approval instead
  of silently narrowing or expanding the product contract.

## Affected Areas

Expected implementation changes:

- Telemetry domain:
  - `src/telemetry/mod.rs`
    - Add a repair validation request/options type.
    - Add evidence fields for source artifact, patch summary, graph build,
      rebuild summary, test summary, local status, and human-review state.
    - Add or explicitly represent the graph build gate separately from graph
      validation.
    - Add a deterministic validation runner that consumes mapped repair context,
      patch JSON, graph source, optional rebuild/test configuration, and returns
      `RepairValidationEvidence`.
    - Keep existing human-review and acceptance defaults.
    - Keep existing patch parsing through `parse_repair_graph_patch()`.
    - Reuse `crate::patch::apply_patch()` for atomic application.
    - Reuse `crate::parser::parse_jsonld()`,
      `crate::graph::builder::build_graph()`, and
      `crate::graph::validator::validate()` for graph gates.
- CLI:
  - `src/cli/mod.rs`
    - Add `duumbi telemetry repair-validate` parser support.
    - Required inputs should include mapped context, patch, and source graph or
      workspace graph selection.
    - Add explicit `--output <path>` for writing evidence.
    - Add repeatable `--test <command>` or equivalent structured test-plan
      input when Stage 10 uses command-based test execution.
  - `src/main.rs`
    - Dispatch the repair-validation telemetry subcommand.
    - Print evidence JSON to stdout on success.
    - Surface failures through existing CLI error handling without provider
      setup or network behavior.
- Tests:
  - `src/telemetry/mod.rs` unit tests for every validation gate and evidence
    invariant.
  - `src/cli/mod.rs` parser tests for repair validation arguments.
  - `tests/integration_telemetry.rs` or a focused integration test file for the
    controlled live validation path.
  - Existing `src/patch.rs` tests may be referenced; add patch tests only if a
    #587 scenario is not already covered clearly enough.

Areas expected not to change:

- `specs/DUUMBI-587/PRODUCT.md`
- product specs for other issues
- generated telemetry artifacts
- `runtime/duumbi_runtime.c`
- `runtime/duumbi_runtime.h`
- compiler trace hook ABI
- provider/model configuration
- Repair-agent prompt generation
- intent verifier repair loop behavior
- Studio UI
- remote telemetry ingestion/export
- MCP mutation semantics, unless Stage 10 first proves a narrow helper reuse is
  necessary and remains read-only for #587

CI and local validation paths:

- `cargo fmt --check`
- `git diff --check`
- `cargo test repair_validation --lib`
- `cargo test repair_graph_patch --lib`
- `cargo test telemetry --lib`
- `cargo test telemetry_repair_validate_parses` or equivalent CLI parser test
- `cargo test --test integration_telemetry`
- A local CLI smoke path using `target/debug/duumbi telemetry repair-validate`

## Technical Approach

### 1. Add A Local Repair Validation Surface

Add a deterministic CLI command:

```text
duumbi telemetry repair-validate \
  --context <repair-context.json> \
  --patch <graph-patch.json> \
  --graph <source.jsonld> \
  [--workspace <workspace-root>] \
  [--module <relative-graph-path>] \
  [--test <command> ...] \
  [--output <repair-validation.json>]
```

Required behavior:

- `--context` points to a serialized `RepairCrashContext` or the approved #588
  equivalent mapped repair crash context.
- `--patch` points to proposed GraphPatch JSON. Accepted file shapes:
  - `{"ops":[...]}` for a complete `GraphPatch`.
  - `[...]` only if Stage 10 deliberately wraps it into `GraphPatch { ops }`
    and records that normalization in evidence.
- `--graph` points to the original JSON-LD source graph being validated.
- `--workspace` and `--module` are optional. If supplied, validation may build a
  temporary workspace clone and replace only the selected module path in the
  temporary copy.
- `--test` is repeatable. If no tests are supplied, the implementation must use
  the conservative default test plan below or mark the relevant-tests gate as
  failed/unavailable. It must not report local validation success without a
  passing test gate.
- `--output` writes the bounded evidence JSON to the requested path. Without
  `--output`, evidence goes only to stdout.
- The command must not call a provider, create a repair proposal, apply a patch
  to the original graph source, or mark a repair accepted.

Rejected command alternatives:

- Do not add `duumbi heal`, `duumbi repair --auto`, production crash ingestion,
  remote telemetry, hot-swap, deployment, or auto-apply flags in #587.
- Do not overload `duumbi telemetry inspect`; it remains a human-readable
  crash-evidence inspection command.
- Do not reuse `duumbi add` or `intent execute` repair behavior because those
  paths can call providers and write graph files.

### 2. Define Request And Evidence Shapes

Add a request/options type similar to:

```rust
pub struct RepairValidationRequest {
    pub crash_context: RepairCrashContext,
    pub patch_json: serde_json::Value,
    pub source: RepairValidationSource,
    pub rebuild: RepairRebuildPlan,
    pub tests: RepairTestPlan,
    pub output_path: Option<PathBuf>,
}

pub enum RepairValidationSource {
    SingleGraphFile(PathBuf),
    WorkspaceGraph {
        workspace_root: PathBuf,
        module_path: PathBuf,
    },
}

pub enum RepairTestPlan {
    Commands(Vec<RepairTestCommand>),
    ConservativeDefault,
}
```

The exact Rust names may change if Stage 10 finds a better local fit, but the
serialized evidence must remain reviewable and stable.

Extend or wrap `RepairValidationEvidence` so it includes:

- `schema_version`
- `crash_context`
- `proposed_patch`
- `source_artifact`
- `patch_summary`
- `gates`
- `rebuild_summary`
- `test_summary`
- `local_validation_passed`
- `requires_human_review`
- `accepted_for_application`
- `human_review_state`

Canonical values:

- `human_review_state = "pending"` when local gates pass.
- `human_review_state = "not_ready"` when any local gate fails.
- `requires_human_review = true` for every evidence report.
- `accepted_for_application = false` for every evidence report produced by
  #587.

If Stage 10 adds a new gate enum variant, prefer:

```rust
pub enum RepairValidationGate {
    GraphPatchParse,
    AtomicPatchApplication,
    GraphParse,
    GraphBuild,
    GraphValidation,
    NativeRebuild,
    RelevantTests,
}
```

If adding `GraphBuild` would break existing serialized artifacts, Stage 10 may
instead keep `GraphValidation` as the compatibility enum and add a distinct
`subgate: "graph_build"` evidence field. In either case, the evidence JSON must
show graph build failure separately from semantic graph validation failure.

### 3. Run Gates In A Fixed Order

Validation must short-circuit after the first blocking failed gate, but the
evidence report must still include passed earlier gates and the failed gate.

Gate order:

1. Input preflight:
   - context file exists and parses.
   - patch file exists and parses as JSON.
   - source graph/workspace artifact exists and is readable.
   - raw stderr/log text alone is rejected.
2. GraphPatch parse:
   - call `parse_repair_graph_patch()` or equivalent canonical
     `serde_json::from_value::<GraphPatch>()`.
3. Atomic patch application:
   - parse the original source as `serde_json::Value`.
   - call `crate::patch::apply_patch(&source, &patch)`.
   - never write the original source.
   - record operation count and failed operation summary when available.
4. Graph parse:
   - serialize the patched JSON-LD value and call
     `crate::parser::parse_jsonld()`.
5. Graph build:
   - call `crate::graph::builder::build_graph()`.
   - collect graph construction diagnostics separately from semantic validation
     diagnostics.
6. Graph validation:
   - call `crate::graph::validator::validate()`.
   - fail on any blocking diagnostic.
7. Native rebuild:
   - write the patched graph only into a temporary file or temporary workspace.
   - single-file mode uses the equivalent of:
     `duumbi build <candidate.jsonld> -o <tmp>/repair-candidate`.
   - workspace mode uses a temporary workspace clone and existing
     `workspace::build_workspace_with_options()` or equivalent CLI behavior.
   - record command/API summary, exit status, and bounded stderr/stdout.
8. Relevant tests:
   - run the explicit test commands or conservative default test plan.
   - record each command, exit status, and bounded output summary.
9. Human-review boundary:
   - if every local gate passed, set local validation passed and human review
     pending.
   - never set accepted for application.

### 4. Keep Source Preservation Observable

The implementation must prove source preservation in both success and failure
paths:

- Read original source bytes before validation.
- Apply patch only to a cloned JSON value.
- For CLI tests, compare original source bytes after failed validation.
- If validation uses a temporary workspace, write only inside the temp
  workspace.
- Evidence should identify the original source path and the temporary candidate
  path or state that the candidate existed in memory only.

### 5. Select Rebuild And Test Plans Conservatively

Single-file rebuild:

- Use existing single-file build behavior against the temporary patched graph.
- Output binary goes under a temp directory.
- Do not reuse or overwrite the user-supplied output path.

Workspace rebuild:

- Use a temp workspace clone or temp graph directory.
- Replace only the selected graph module in the temp copy.
- Build through existing workspace build helpers.
- If the workspace cannot be cloned or selected safely, fail the native rebuild
  gate with reviewable evidence instead of mutating the original workspace.

Relevant tests:

- Explicit `--test` commands are preferred for implementation PR evidence.
- Command output must be bounded. Store full logs only when an explicit artifact
  path is configured and it stays out of committed generated files.
- If no explicit tests are supplied, use this conservative default when the
  repo root is available:
  - `cargo test telemetry --lib`
  - `cargo test --test integration_telemetry`
- If the changed graph or repair context is broader than telemetry fixtures, the
  implementation agent must either supply explicit relevant tests or run
  `cargo test --all` before claiming local validation success.
- A skipped, missing, or unavailable test plan fails the relevant-tests gate.

### 6. Produce Bounded Human-Review Evidence

The evidence report must be bounded enough for GitHub review and CI artifacts.

Required output fields:

- crash context source and trace correlation.
- source graph/workspace artifact path.
- proposed patch JSON or normalized patch summary.
- patch operation count.
- one result per required gate.
- parse/build/validation diagnostic summaries.
- rebuild command/API summary and bounded output.
- test plan rationale and per-command results.
- local validation status.
- human-review requirement.
- repair acceptance state.

Output constraints:

- Do not include unbounded raw logs by default.
- Do not include provider prompts, provider responses, credentials, environment
  secrets, runtime values, heap snapshots, stack snapshots, or production crash
  payloads.
- Do not write generated evidence unless `--output` is supplied.
- Do not mark generated evidence as an accepted repair.

### 7. Keep Query Mode Read-Only

#587 must not change Query mode into a mutation or acceptance path.

Implementation options:

- Preferred v1: keep Query mode unchanged and expose validation evidence as
  deterministic JSON plus a review summary helper.
- If Stage 10 adds a read-only evidence summary used by Query mode, it must
  parse existing evidence and describe it without patching, writing, accepting,
  or calling provider-driven mutation behavior.

Required proof:

- Diff review shows no Query-mode mutation, source-write, or repair-acceptance
  path was added.
- Existing Query mode read-only tests remain green.
- If Query evidence summary code is added, add a focused test showing it reads a
  validation report and does not mutate graph source.

## Invariants

- Product spec `specs/DUUMBI-587/PRODUCT.md` remains unchanged.
- Validation requires mapped repair crash context, a proposed patch, and source
  graph/workspace evidence.
- Raw logs alone never validate a repair.
- Patch-shaped output is not trusted until it parses as `GraphPatch`.
- Patch application is atomic and validates on a clone or temporary copy.
- Graph parse, graph build, graph validation, native rebuild, and relevant tests
  are distinct product-visible gates.
- Failed gates remain reviewable.
- Local validation success does not mean repair acceptance.
- `requires_human_review` remains true.
- `accepted_for_application` remains false.
- Default untraced behavior remains unchanged.
- No provider, network, Slack, GitHub, Studio, external collector, or production
  telemetry service is required for validation.
- No repair prompt generation, autonomous retry loop, automatic apply, merge,
  deploy, hot-swap, or production self-healing behavior is added.
- Generated evidence artifacts are not committed.

## BDD-To-Test Mapping

| Product BDD Scenario | Required Technical Evidence |
| --- | --- |
| Controlled repair candidate enters validation | Unit test builds a `RepairValidationRequest` from mapped `RepairCrashContext`, valid `GraphPatch`, and graph source; asserts gates run in the documented order. CLI integration uses a controlled context/patch/source fixture and observes evidence linking the crash context. |
| Raw crash logs are not enough to validate a repair | CLI/parser or request validation test calls repair validation without `--context` and asserts a missing mapped-context error. Review evidence confirms no API accepts raw stderr/log text as context. |
| Missing patch candidate blocks validation | CLI/parser or request validation test omits `--patch` and asserts no gate is marked passed and no source graph is changed. |
| Unavailable source artifact blocks validation | Unit or CLI test points `--graph` to a missing/unreadable path and asserts source artifact unavailable, patch application not attempted, and local validation false. |
| Malformed patch fails before application | Unit test passes invalid GraphPatch JSON to `parse_repair_graph_patch()` through the validation runner; asserts GraphPatch parse gate fails, atomic patch gate is absent or not attempted, and evidence records parse diagnostics. |
| Patch application failure preserves the original graph | Unit test uses a GraphPatch referencing a missing node; asserts atomic patch gate fails and the original source value/file bytes are unchanged. Existing `src/patch.rs::patch_is_all_or_nothing_on_error` may be referenced but #587 needs validation-evidence coverage too. |
| Patch-shaped output is not automatically accepted | Unit test passes GraphPatch-shaped output before downstream gates; asserts `accepted_for_application == false`, `requires_human_review == true`, and missing downstream gates prevent local validation. |
| Graph parse failure blocks local success | Unit test applies a patch that leaves JSON serializable but invalid for `parser::parse_jsonld()`; asserts graph parse gate fails, later gates are not attempted, and diagnostics are included. |
| Graph build failure blocks local success | Unit test applies a patch that parses to AST but fails `builder::build_graph()`; asserts graph build gate or subgate fails distinctly from semantic graph validation. |
| Graph validation failure blocks local success | Unit test applies a patch that builds graph IR but produces validator diagnostics; asserts graph validation gate fails and diagnostics are included. |
| Native rebuild failure blocks local success | Unit test uses a rebuild runner trait seam or temp graph fixture that fails compile/link; asserts native rebuild gate fails with command/API summary. Add an integration variant if a stable fixture can trigger this without broad compiler changes. |
| Relevant test failure blocks local success | Unit test uses a test-runner trait seam or explicit failing command in a temp workspace; asserts relevant tests gate fails and failed command summary is included. |
| All local gates pass but human review is still required | Unit/integration test with a valid candidate and passing rebuild/test runners asserts every required local gate passes, `local_validation_passed == true`, `requires_human_review == true`, `accepted_for_application == false`, and `human_review_state == "pending"`. |
| Human reviewer requests revision after local validation | Unit test for evidence or linked review-state helper records `revision_requested` or equivalent external review state without setting `accepted_for_application`. If Stage 10 does not implement reviewer-state persistence, review evidence must show local validation stays pending and human review is outside #587. |
| Evidence report links crash, patch, validation, rebuild, and tests | Serialization test asserts evidence includes crash context, proposed patch/summary, source artifact, every gate result, rebuild summary, test summary, local status, and human-review requirement. |
| Failed validation remains reviewable | Unit test triggers a failed middle gate and asserts earlier passed gates and failed gate output remain visible while local validation stays false. |
| Production self-healing is requested from the validation guardrail | Review evidence confirms no production ingestion, auto-deploy, hot-swap, `duumbi heal`, or auto-accept command/flag exists. If Stage 10 adds CLI help text, assert it does not advertise production self-healing. |
| Query mode asks about repair validation evidence | Review evidence confirms Query mode remains read-only and cannot mutate/accept repairs. If a read-only evidence summary helper is added, unit test it against sample evidence and assert original graph bytes remain unchanged. |

## Live E2E Plan

#587 does not require a live provider-backed E2E path. Repair validation consumes
a proposed patch; it does not generate one. Live E2E therefore uses local
CLI/runtime evidence and a static patch fixture with zero external LLM calls.

Canonical interface:

- CLI: `duumbi telemetry repair-validate`.

Required credentials and environment:

- No provider credentials.
- No network.
- `DUUMBI_TELEMETRY_DIR` set to a temporary directory for the traced fixture.

Expected external LLM calls:

- `0`.

Estimated external LLM cost:

- `USD 0`.

Canonical Unix-like smoke path:

```sh
cargo build
tmp="$(mktemp -d)"
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" \
  target/debug/duumbi build --trace \
  tests/fixtures/telemetry/option_none_unwrap.jsonld \
  -o "$tmp/panic-fixture"
set +e
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" "$tmp/panic-fixture"
status="$?"
set -e
test "$status" -ne 0
target/debug/duumbi telemetry repair-context \
  --telemetry-dir "$tmp/telemetry" \
  --graph tests/fixtures/telemetry/option_none_unwrap.jsonld \
  --output "$tmp/repair-context.json"
cat > "$tmp/repair-patch.json" <<'JSON'
{
  "ops": [
    {
      "kind": "modify_op",
      "node_id": "duumbi:telemetry/main/entry/0",
      "field": "duumbi:value",
      "value": 1
    }
  ]
}
JSON
target/debug/duumbi telemetry repair-validate \
  --context "$tmp/repair-context.json" \
  --patch "$tmp/repair-patch.json" \
  --graph tests/fixtures/telemetry/option_none_unwrap.jsonld \
  --test "cargo test telemetry --lib" \
  --test "cargo test --test integration_telemetry" \
  --output "$tmp/repair-validation.json"
```

Pass criteria:

- traced fixture exits nonzero before the candidate patch.
- crash context is assembled from mapped local artifacts.
- repair validation exits successfully only when all local gates pass.
- evidence JSON includes source artifact, proposed patch, every required gate,
  rebuild summary, test summary, `local_validation_passed`, human-review
  requirement, and `accepted_for_application: false`.
- original source fixture remains unchanged.
- no provider credentials or network calls are used.

Cross-platform canonical evidence:

- `cargo test --test integration_telemetry`
- focused repair-validation unit tests in `src/telemetry/mod.rs`

Failure criteria:

- validation succeeds from raw logs without mapped context.
- validation succeeds without a patch or source artifact.
- malformed GraphPatch reaches patch application.
- failed patch application changes original source.
- graph build failure is hidden as generic validation success.
- rebuild or tests are skipped while local validation is reported as passed.
- evidence marks the repair accepted.
- command calls a provider or writes generated evidence without `--output`.

If #588 is not implemented when #587 Stage 10 starts, the implementation may
construct a test `RepairCrashContext` fixture directly from existing telemetry
types for unit coverage, but the live E2E path must stop before claiming full
repair-context integration until the approved #588 command or equivalent mapped
context artifact exists.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. summarize current state and remaining unmet #587 requirements.
2. propose one bounded implementation goal.
3. list intended file areas and commands before editing.
4. estimate external LLM calls, external LLM cost, command budget, and risk.
5. check whether the resource gate requires human approval.
6. implement only the approved or resource-permitted goal.
7. run the agreed checks.
8. report evidence, failures, cost, and remaining gaps.
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

Allowed autonomous cycle goals:

- Cycle goal option A: evidence schema, explicit graph-build gate/subgate, and
  serialization/unit tests in `src/telemetry/mod.rs`.
- Cycle goal option B: validation runner for context, patch parse, atomic apply,
  graph parse/build/validate gates, and failure evidence tests.
- Cycle goal option C: temp-source rebuild/test runner seams plus native rebuild
  and relevant-test gate coverage.
- Cycle goal option D: `duumbi telemetry repair-validate` CLI parser/dispatch
  and controlled integration smoke coverage.
- Cycle goal option E: final regression pass, live E2E evidence, and review
  cleanup.

The implementation agent may combine small substeps only when the resource
policy stays below thresholds and the resulting diff remains reviewable.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Approved implementation pool:
  - `src/telemetry/mod.rs`
  - `src/cli/mod.rs`
  - `src/main.rs`
  - `tests/integration_telemetry.rs`
  - focused test fixtures under `tests/fixtures/telemetry/` only when necessary
- Max files or modules per cycle: at most 3 source/test files from the approved
  pool, excluding unchanged fixtures. Touching 4 or more files in one cycle, or
  touching any file outside the approved pool, needs coordinator approval unless
  the extra file is a small focused test fixture.
- Expected command budget per low-budget cycle: up to 6 focused commands.
- Expected external LLM calls per cycle: 0.
- Estimated external LLM cost per cycle: USD 0.
- Hard human-approval threshold: stop before planned external LLM usage exceeds
  USD 2 or before planned external LLM calls exceed 10.
- Human approval is required before:
  - expanding beyond #587 product scope.
  - generating repair patches from an LLM.
  - calling OpenAI, Anthropic, OpenRouter, Grok, Minimax, or another provider.
  - changing provider setup/model behavior.
  - changing runtime assets or trace hook ABI.
  - changing compiler lowering beyond evidence required for a proven rebuild
    gate bug.
  - changing MCP mutation semantics.
  - adding a risky dependency.
  - performing a migration.
  - writing generated artifacts into the repo.
  - accepting irreversible operations.
  - making a security/privacy decision.
  - making a product or architecture decision not already covered here.
  - adding production crash ingestion, auto-apply, deployment, hot-swap, or
    autonomous self-healing behavior.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 3 low-budget cycles before pausing with evidence and
  remaining work.
- Stop and ask for human guidance when:
  - #588 context artifacts are unavailable and live #587 E2E cannot be proven.
  - workspace validation cannot be implemented without broader workspace-copy or
    project-indexing design.
  - relevant test selection cannot be made conservative enough.
  - a validation report needs runtime values, heap snapshots, credentials, or
    production payloads.
  - source preservation conflicts with existing build APIs.
  - any focused check fails twice after targeted fixes.

## Task Breakdown

1. Verify the issue still has `product-spec-approved` and remains in Technical
   Spec Needed before Stage 10 starts.
2. Check whether #588 implementation has landed. If not, keep unit tests
   context-fixture based and stop live repair-context integration before
   claiming full E2E.
3. Add or adapt repair validation evidence schema:
   - expose graph build distinctly.
   - preserve human-review and not-accepted defaults.
   - add source/rebuild/test summaries.
4. Add validation runner:
   - preflight context, patch, and source.
   - parse GraphPatch.
   - apply to cloned source.
   - parse/build/validate patched graph.
   - write temporary candidate only for rebuild/test gates.
5. Add rebuild and test plan execution:
   - single-file temp rebuild first.
   - workspace temp rebuild only when safe.
   - explicit tests or conservative defaults.
6. Add CLI parser and dispatch for `duumbi telemetry repair-validate`.
7. Add unit and integration tests mapped to the BDD table.
8. Run focused verification and then broader checks required by touched files.
9. Record implementation PR evidence:
   - BDD scenario to test mapping.
   - local checks.
   - live E2E results or a clearly explained #588 blocker.
   - zero external LLM calls and USD 0 external provider cost.

## Verification Plan

Required local checks for the implementation PR:

```sh
cargo fmt --check
git diff --check
cargo test repair_validation --lib
cargo test repair_graph_patch --lib
cargo test telemetry --lib
cargo test --test integration_telemetry
```

Required CLI parser check:

```sh
cargo test telemetry_repair_validate_parses --lib
```

The exact test name may differ, but the implementation PR must include parser
coverage for `telemetry repair-validate`.

Conditional checks:

```sh
cargo test --all
```

Run `cargo test --all` before implementation PR review when the diff touches
shared compiler, workspace, CLI dispatch, graph validation, or broad telemetry
behavior.

Manual live E2E:

- Run the Live E2E Plan above after focused tests pass and after #588 context
  generation is available.
- Record temporary artifact paths only in PR evidence; do not commit generated
  telemetry or validation artifacts.

## Completion Criteria

The implementation is complete when:

- All approved #587 BDD scenarios have mapped test, review, or live E2E
  evidence.
- Validation requires mapped context, patch candidate, and source graph.
- Malformed GraphPatch fails before application.
- Failed patch application preserves the original graph.
- Graph parse, graph build, graph validation, native rebuild, and relevant
  tests are represented as distinct reviewable gates.
- Failed gates produce bounded evidence and local validation false.
- All local gates passing still leaves human review required.
- `accepted_for_application` remains false.
- Evidence links crash context, patch, source artifact, validation gates,
  rebuild output, and test output.
- Default untraced behavior remains unchanged.
- Query mode remains read-only.
- No provider calls, production ingestion, auto-apply, deploy, hot-swap, or
  autonomous repair loop exists.
- Required focused checks pass.
- Any broader checks required by touched files pass.
- The implementation PR references #587 with non-closing language and leaves
  the execution issue open.

## Failure And Escalation

Stop and request coordinator or human approval if:

- The approved #588 context contract is unavailable and #587 live integration
  cannot be proven with a mapped context artifact.
- Graph build cannot be separated from graph validation without a schema or
  compatibility decision.
- Workspace validation requires broad workspace-copy, dependency, or project
  indexing design not covered here.
- Native rebuild cannot run on a temporary candidate without mutating user
  source.
- Relevant tests cannot be selected conservatively.
- Implementation requires provider calls, generated repair patches, or
  autonomous retries.
- Implementation requires runtime asset changes, trace hook ABI changes, MCP
  mutation behavior changes, provider behavior changes, risky dependencies,
  migrations, generated committed artifacts, security/privacy decisions, or
  production self-healing behavior.

## Open Questions

None blocking. Treat the following as fixed Stage 10 constraints:

- Use `duumbi telemetry repair-validate` as the v1 local validation surface.
- Emit evidence to stdout by default; write only when `--output` is explicit.
- Keep external LLM calls at 0 and provider cost at USD 0.
- Keep human review required and repair acceptance false in every #587 evidence
  report.
- Support single-file source validation first; add workspace validation only
  when it can be done through a temporary copy without broadening the scope.
