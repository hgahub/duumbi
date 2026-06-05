# DUUMBI-585: Persist Crash Dumps And Trace-To-Graph Mapping Artifacts - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-585/PRODUCT.md` by turning
the local telemetry artifact contract into an agent-executable audit and, only
where needed, a narrow hardening plan.

The accepted product behavior is:

```text
explicit traced build/run -> local trace + crash + map artifacts ->
function/block graph evidence -> inspectable failure explanation
```

This issue is an artifact-contract reconciliation slice. Current `main` already
contains substantial Phase 13 telemetry behavior from related merged work:
traced build mode, telemetry config validation, function/block trace events,
trace-map generation, crash artifacts, telemetry inspection, repair context, and
repair validation evidence. Stage 10 agents must therefore audit current source
and tests before editing anything. If the accepted contract is already covered,
the implementation evidence should be evidence-focused. If a specific gap
remains, the implementation must make the smallest safe change that proves that
criterion.

This technical spec PR is related to #585 and must leave the execution issue
open for Stage 9 approval, Stage 10 implementation or evidence coordination,
Stage 11 review when applicable, and Stage 12 completion evidence.

## Agent Audience

- Codex implementation agents running bounded Stage 10 Ralph cycles.
- Codex CLI or Codex App agents auditing current telemetry evidence.
- Rust telemetry agents working in `src/telemetry/mod.rs`.
- CLI/workspace agents validating artifact directory behavior.
- Runtime/compiler agents only if the audit proves an artifact routing or trace
  emission bug that cannot be resolved through tests/evidence alone.
- Reviewer and tester agents checking BDD coverage, local E2E evidence, and
  no-provider/no-production-telemetry boundaries.

## Source Context

- Product spec: `specs/DUUMBI-585/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/660
- GitHub issue: https://github.com/hgahub/duumbi/issues/585
- Stage 7 product spec approval:
  https://github.com/hgahub/duumbi/issues/585#issuecomment-4630860404
- Stage 6 product spec draft:
  https://github.com/hgahub/duumbi/issues/585#issuecomment-4630822709
- Stage 5 human acceptance:
  https://github.com/hgahub/duumbi/issues/585#issuecomment-4629889364
- Stage 4 triage:
  https://github.com/hgahub/duumbi/issues/585#issuecomment-4550311052
- Parent issue: https://github.com/hgahub/duumbi/issues/580
- Parent product spec: `specs/DUUMBI-580/PRODUCT.md`
- Parent technical spec: `specs/DUUMBI-580/TECHNICAL.md`
- Traced build/config specs:
  - `specs/DUUMBI-583/PRODUCT.md`
  - `specs/DUUMBI-583/TECHNICAL.md`
- Function/block trace event specs:
  - `specs/DUUMBI-584/PRODUCT.md`
  - `specs/DUUMBI-584/TECHNICAL.md`
- Controlled failure evidence specs:
  - `specs/DUUMBI-586/PRODUCT.md`
  - `specs/DUUMBI-586/TECHNICAL.md`
- Repair context specs:
  - `specs/DUUMBI-588/PRODUCT.md`
  - `specs/DUUMBI-588/TECHNICAL.md`
- Repair validation specs:
  - `specs/DUUMBI-587/PRODUCT.md`
  - `specs/DUUMBI-587/TECHNICAL.md`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- AI review policy: `docs/automation/code-review-policy.md`
- Default automated review workflow: `.github/workflows/copilot-review.yml`
- Technical spec review workflow:
  `.github/workflows/technical-spec-review-request.yml`

Relevant verified source facts:

- `src/telemetry/mod.rs`
  - Defines `TELEMETRY_DIR_ENV`, `TRACE_MAP_FILE`, `CRASH_DUMP_FILE`,
    `TRACE_MAP_SCHEMA_VERSION`, and `CRASH_SCHEMA_VERSION`.
  - Defines `TelemetryBuildMode`, `BuildOptions`, `TelemetrySection`,
    `ResolvedTelemetryConfig`, `TraceMap`, `TraceMapEntry`, and
    `CrashArtifact`.
  - Generates stable trace IDs from graph identity and rejects trace ID
    collisions.
  - Writes `trace_map.json` through `write_trace_map()`.
  - Maps crash evidence through `inspect_crash_artifacts()` by joining crash
    `function_id` to a function trace-map entry and crash `block_id` to a block
    trace-map entry.
  - Fails closed for inactive traced crashes, missing map evidence, and unmapped
    crash IDs.
  - Validates telemetry config for traced builds, including artifact directory,
    sampling mode, sample rate, environment override, and unsupported value
    capture.
- `runtime/duumbi_runtime.c`
  - Defines runtime trace state and trace hook functions.
  - Writes `traces.jsonl` for function/block enter/exit events and panic
    correlation.
  - Writes `crash_dump.jsonl` for traced runtime panic evidence.
  - Honors `DUUMBI_TELEMETRY_DIR` and otherwise falls back to
    `.duumbi/telemetry`.
  - Preserves the original `duumbi panic:` stderr path.
- `runtime/duumbi_runtime.h`
  - Declares the trace hook ABI used by generated code.
- `src/compiler/lowering.rs`
  - Emits trace hook calls only when `TelemetryBuildMode::Trace` is selected.
  - Keeps default builds uninstrumented.
  - Computes emitted trace IDs from the same graph identities used by
    `trace_map.json`.
- `src/workspace.rs`
  - Builds workspace programs with explicit `BuildOptions`.
  - For traced workspace builds, resolves telemetry config and writes a combined
    `trace_map.json` to the resolved artifact directory.
- `src/cli/commands.rs`
  - Builds single-file programs with `duumbi build --trace`.
  - For traced single-file builds, writes `trace_map.json` to the telemetry
    artifact directory resolved from the current workspace/config context.
- `src/main.rs`
  - Exposes `duumbi telemetry inspect`, `repair-context`, and
    `repair-validate`.
  - `duumbi telemetry inspect` reads the default telemetry directory from config
    when `--telemetry-dir` is omitted.
  - `duumbi run` executes the existing workspace binary; it does not build with
    trace or embed telemetry config into a direct binary execution path.
- `tests/integration_telemetry.rs`
  - Builds and runs traced controlled failure fixtures with an isolated
    `DUUMBI_TELEMETRY_DIR`.
  - Asserts `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` exist.
  - Parses trace events and crash records and asserts they join to the trace
    map.
  - Runs `duumbi telemetry inspect` and asserts mapped function/block output
    plus exact-node-unavailable text.
  - Asserts default untraced failure does not produce telemetry artifacts and is
    not accepted as graph back-mapping evidence.
  - Covers repair context and repair validation flows that depend on mapped
    crash evidence.
- `tests/fixtures/telemetry/option_none_unwrap.jsonld`
  - Provides the deterministic controlled panic fixture.
- `tests/fixtures/telemetry/call_then_panic.jsonld`
  - Provides caller-context coverage after a helper call returns.

Relevant GitHub evidence:

- #580 is closed with merged implementation PR #604.
- #583 is closed with merged implementation PR #614.
- #584 is closed with merged implementation PR #633.
- #586 is closed with merged implementation PR #656.
- #588 is closed with merged implementation PR #657.
- #587 is closed with merged implementation PR #659.
- Product spec PR #660 is merged with Stage 7 approval and routed #585 to
  Technical Spec Needed.

Relevant Obsidian context:

- `DUUMBI - PRD.md`
  - Runtime failure feedback is local developer/test evidence first.
  - The current foundation maps controlled runtime failures to graph
    function/block context.
  - Repair validation evidence remains human-gated.
- `Runtime Failure Feedback Loop.md`
  - Traced behavior must be explicit and local by default.
  - Function/block graph back-mapping is the first reliable mapping level.
  - Crash evidence must preserve the original runtime failure signal.
  - No production crash ingestion, hot-swap, or autonomous repair acceptance is
    part of this slice.
- `DUUMBI - Service and Research Direction.md`
  - Source repo behavior and GitHub execution state are stronger evidence when
    archived prose and current source disagree.
  - The local Phase 13 foundation is a partial, local proof path rather than a
    production customer self-healing feature.
- `DUUMBI - Agentic Development Runbook.md`
  - GitHub is the execution source of truth.
  - Spec PRs may pass bounded AI gates when Codex self-review and Copilot
    evidence are clean.
  - Greptile remains manual-only and quota-limited.

Verified implementation interpretation:

- The primary Stage 10 job is not to reinvent telemetry artifacts. It is to
  prove the #585 contract against current source and patch only specific
  evidence gaps.
- The highest-risk verified gap is configured artifact-directory behavior:
  Rust build paths resolve config/env and can write `trace_map.json` to a custom
  artifact directory, while the C runtime can only honor
  `DUUMBI_TELEMETRY_DIR` or default `.duumbi/telemetry`.
- Directly executed binaries cannot read DUUMBI workspace config unless a new
  runtime configuration mechanism is designed. For v1, direct binary execution
  with a custom artifact directory should continue to require
  `DUUMBI_TELEMETRY_DIR` unless a later accepted product/architecture decision
  says otherwise.
- The CLI workspace runtime surface `duumbi run` may be the narrow place to
  keep configured workspace evidence together if Stage 10 proves the current
  source splits `trace_map.json` from runtime JSONL artifacts.

## Affected Areas

Expected Stage 10 audit and evidence areas:

- `src/telemetry/mod.rs`
- `runtime/duumbi_runtime.c`
- `runtime/duumbi_runtime.h`
- `src/compiler/lowering.rs`
- `src/workspace.rs`
- `src/cli/commands.rs`
- `src/main.rs`
- `src/cli/mod.rs`
- `tests/integration_telemetry.rs`
- `tests/fixtures/telemetry/option_none_unwrap.jsonld`
- `tests/fixtures/telemetry/call_then_panic.jsonld`
- Related specs under `specs/DUUMBI-580`, `specs/DUUMBI-583`,
  `specs/DUUMBI-584`, `specs/DUUMBI-586`, `specs/DUUMBI-588`, and
  `specs/DUUMBI-587`

Expected implementation areas only if a gap is verified:

- `tests/integration_telemetry.rs`
  - Add or harden artifact-contract integration coverage.
  - Preferred for malformed inspection evidence, configured artifact-directory
    E2E, and contract matrix assertions.
- `src/telemetry/mod.rs`
  - Add focused unit tests for `inspect_crash_artifacts()` malformed artifact
    behavior if current repair-context tests are not enough for #585.
  - Add narrowly scoped parser/path helpers only if test-only evidence cannot
    cover the product contract.
- `src/workspace.rs`, `src/main.rs`, or `src/cli/commands.rs`
  - Only if Stage 10 proves a product-blocking configured artifact-directory
    routing issue and the fix can stay inside CLI/workspace local execution
    semantics.
- `runtime/duumbi_runtime.c` and `runtime/duumbi_runtime.h`
  - Avoid changes by default. Edit only after a verified product-blocking
    runtime artifact writer defect and a resource-gate check, because this is
    compiler/runtime behavior.

Areas out of scope:

- Product specs, including `specs/DUUMBI-585/PRODUCT.md`.
- Technical specs for other issues.
- Generated telemetry artifacts, binaries, object files, crash dumps, trace
  output, or runtime assets.
- Provider/model configuration and external LLM calls.
- Remote telemetry export, OpenTelemetry, collectors, dashboards, Studio UI, or
  production crash ingestion.
- Repair patch generation, autonomous repair acceptance, hot-swap, deploy, or
  account-based self-healing.
- New artifact schema fields for runtime values, heap snapshots, stack
  snapshots, argument capture, retention, upload, rotation, or run IDs.

## Technical Approach

### 1. Build The Evidence Matrix Before Editing

Stage 10 must first prepare a scenario-by-scenario evidence matrix from current
`main`:

- product BDD scenario.
- current source owner.
- current test or CI evidence.
- gap classification: satisfied, needs test hardening, needs code hardening,
  out-of-scope architecture decision, or evidence-only disposition.
- intended command or review evidence.

Do not modify source until the matrix names a specific unmet criterion.

### 2. Prefer Existing Source Ownership

Facts:

- `src/telemetry/mod.rs` owns Rust schema constants, domain types, config
  validation, trace-map generation, crash inspection, repair context, and repair
  validation evidence.
- `runtime/duumbi_runtime.c` owns runtime JSONL writing for trace and crash
  artifacts.
- `src/compiler/lowering.rs` owns traced-mode hook emission.
- `src/workspace.rs` and `src/cli/commands.rs` own build-time trace-map writing.
- `src/main.rs` owns CLI telemetry inspection and workspace binary execution.

Implementation recommendation:

- Do not introduce parallel schema constants, duplicate runtime writers, or new
  artifact names.
- Keep artifact contract checks close to the existing telemetry tests.
- Reuse the controlled fixtures unless the audit proves they cannot cover a
  required scenario.

### 3. Prove The Existing Artifact Contract

The implementation or evidence artifact must prove these v1 contracts:

- `trace_map.json`
  - schema version is `duumbi.telemetry.trace_map.v1`.
  - entries are deterministic and sorted enough for repeatable review.
  - function entries map stable trace IDs to graph function `@id` values.
  - block entries map stable trace IDs to graph block `@id` values.
  - trace ID collisions fail.
- `traces.jsonl`
  - uses JSON Lines format.
  - traced runtime events include function/block enter/exit where applicable.
  - panic correlation records are local and append-only.
  - event `trace_id` values join same-kind trace-map entries.
- `crash_dump.jsonl`
  - uses JSON Lines format.
  - crash records include schema version, event kind, panic message, function
    trace ID, block trace ID, and `trace_active`.
  - latest crash selection or explicit line selection is deterministic where
    available.
  - crash function/block trace IDs join same-kind trace-map entries.
- inspection
  - reads selected crash/map artifacts.
  - reports mapped function/block graph IDs only after the join succeeds.
  - reports exact-node evidence as unavailable in v1.
  - fails closed for missing, malformed, untraced, or unmapped evidence.

### 4. Treat Configured Artifact Directory As The Main Audit Risk

The approved product spec contains two related scenarios:

- configured `artifact-dir` keeps all evidence together.
- if config-only routing splits build-time `trace_map.json` from runtime JSONL,
  the gap must be reported before the contract is claimed complete.

Stage 10 must run or add a workspace-level check for:

```text
[telemetry]
artifact-dir = "custom/telemetry"
```

Required evidence:

- `duumbi build --trace` writes `trace_map.json` under
  `custom/telemetry`.
- the runtime failure writes `traces.jsonl` and `crash_dump.jsonl` under the
  same accepted directory, or Stage 10 records the split as an explicit gap.
- `duumbi telemetry inspect` can join crash and map artifacts from that
  directory when the accepted CLI path is used.

Recommended implementation boundary if a code change is needed:

- Prefer a CLI/workspace-scoped behavior that keeps `duumbi run` aligned with
  the effective telemetry artifact directory when running from a workspace.
- Preserve user-set `DUUMBI_TELEMETRY_DIR` when present.
- Do not make default untraced builds depend on trace-only config validation.
- Do not encode workspace config into arbitrary direct binaries unless a human
  explicitly approves that architecture. Direct binary execution with a custom
  artifact directory should use `DUUMBI_TELEMETRY_DIR` in v1.

### 5. Keep Error Behavior Fail-Closed

Stage 10 must verify that all negative states fail without mapped graph context:

- missing `crash_dump.jsonl`.
- missing `trace_map.json`.
- malformed crash JSON.
- malformed trace-map JSON.
- `trace_active = false`.
- missing function trace ID in the map.
- missing block trace ID in the map.
- default untraced runtime failure.

If current tests cover a negative state through repair-context behavior but not
through `duumbi telemetry inspect`, Stage 10 must decide whether the product
spec requires inspect-specific coverage. If yes, add focused inspect tests.

### 6. Keep Privacy And Scope Boundaries

Stage 10 must not add:

- runtime value or argument capture.
- heap or stack snapshots.
- remote telemetry ingestion.
- upload/retention/rotation policies.
- exact-node evidence.
- provider calls.
- repair proposal generation or repair acceptance.

Existing tests that assert value capture is unsupported and serialized repair
evidence lacks runtime snapshots should stay green.

### 7. Evidence-Only Outcome Is Allowed When Fully Proven

If the audit proves all BDD scenarios are already satisfied by current source,
Stage 10 should not create duplicate code. It should post a Stage 10 evidence
record that maps each scenario to source, tests, CI, and related merged PRs.

If one or more gaps remain, Stage 10 should make the smallest permitted
hardening change and open the normal implementation PR for review.

## Invariants

- Default builds remain telemetry-off.
- Default untraced runtime failures preserve the original panic/stderr behavior
  and do not produce trace/crash/map evidence.
- Traced behavior remains explicit, local, and testable without network access,
  provider credentials, GitHub, Slack, Studio, or external collectors.
- `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` remain the v1
  artifact names.
- Source telemetry types and constants remain the canonical v1 schema owner.
- Runtime trace/crash writing remains append-only JSONL in v1.
- Inspection must not infer graph context from raw stderr, generic logs, or
  incomplete artifacts.
- Missing, malformed, untraced, or unmapped evidence fails closed.
- Runtime value/argument capture remains unsupported.
- Exact node evidence remains unavailable in v1.
- Original runtime panic behavior must not be hidden by telemetry artifact
  writing failures.
- Generated telemetry artifacts must not be committed.
- Direct binary execution with a non-default artifact directory requires
  `DUUMBI_TELEMETRY_DIR` unless a later approved architecture changes that
  contract.
- Greptile is not part of the default review gate and must not be invoked unless
  a developer explicitly requests manual deep review.

## BDD-To-Test Mapping

| Product BDD scenario | Current or required evidence | Commands / assertions |
| --- | --- | --- |
| Traced runtime failure produces the three local evidence artifacts | `tests/integration_telemetry.rs::traced_option_none_unwrap_writes_crash_evidence_and_inspects` already builds with `--trace`, runs with `DUUMBI_TELEMETRY_DIR`, asserts nonzero panic stderr, and checks all three files exist. Stage 10 must preserve this and cite CI. | `cargo test --test integration_telemetry traced_option_none_unwrap_writes_crash_evidence_and_inspects`; optional live CLI smoke with `target/debug/duumbi build --trace` and `telemetry inspect`. |
| Trace map entries preserve graph function and block identity | Unit tests in `src/telemetry/mod.rs` cover deterministic sorting, collision rejection, and function/block graph IDs. Integration tests parse `trace_map.json`. Stage 10 must verify schema version assertions are present or add focused coverage. | `cargo test trace_map --lib`; assertions for `TRACE_MAP_SCHEMA_VERSION`, function/block `TraceMapKind`, expected graph IDs, and collision failure. |
| Crash evidence joins to graph context | Integration helper `assert_latest_crash_joins_trace_map()` parses `crash_dump.jsonl` and joins crash IDs to same-kind trace-map entries; inspect output reports function/block and exact-node-unavailable. | `cargo test --test integration_telemetry traced_option_none_unwrap_writes_crash_evidence_and_inspects`; `cargo test --test integration_telemetry traced_call_then_panic_preserves_caller_context`. |
| Default untraced failure is not accepted as back-mapping evidence | `default_untraced_option_none_unwrap_is_not_back_mapping_proof` asserts no trace map, trace events, or crash dump and verifies inspect does not report graph context. | `cargo test --test integration_telemetry default_untraced_option_none_unwrap_is_not_back_mapping_proof`. |
| Missing evidence fails closed | Current source has missing map tests and untraced missing crash evidence. Stage 10 must verify missing crash and missing map behavior for `duumbi telemetry inspect`; add focused inspect tests if only repair-context tests cover the case. | `cargo test inspect_crash_artifacts --lib`; targeted unit test for missing `crash_dump.jsonl`; CLI inspect against an empty temp directory. |
| Malformed evidence fails closed | Current repair-context tests cover malformed crash/map JSON; current integration covers malformed config. Stage 10 must add or cite inspect-specific malformed crash/map tests if not already present. | Targeted unit tests around `inspect_crash_artifacts()` with malformed `crash_dump.jsonl` and malformed `trace_map.json`; expected `TelemetryError::Parse`. |
| Unmapped crash evidence fails closed | `inspect_crash_artifacts_rejects_unmapped_crash_ids` covers unmapped crash IDs; repair-context tests separately cover unmapped function/block IDs. Stage 10 must cite both or keep inspect-specific coverage. | `cargo test inspect_crash_artifacts_rejects_unmapped_crash_ids --lib`; assert `TelemetryError::Unmapped`. |
| Local artifact path override is honored for traced runs | Existing traced integration helper uses isolated `DUUMBI_TELEMETRY_DIR` for build and run. Stage 10 must preserve and cite it, plus assert source graph files are not mutated when relevant. | `cargo test --test integration_telemetry traced_option_none_unwrap_writes_crash_evidence_and_inspects`; compare fixture bytes before/after when adding new evidence. |
| Configured artifact directory keeps evidence together | Stage 10 must add or run a workspace-level E2E around `[telemetry] artifact-dir = "custom/telemetry"` with no env override. Expected pass if `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` are inspectable from the same directory. | New/focused integration test or manual smoke using workspace config, `duumbi build --trace`, workspace binary execution, and `duumbi telemetry inspect` without `--telemetry-dir`. |
| Config-only runtime artifact routing gap is reported | If current source splits `trace_map.json` into configured dir while runtime JSONL lands in default `.duumbi/telemetry`, Stage 10 must either apply a narrow CLI/workspace fix or record the gap as unmet evidence. | Evidence matrix plus failing/passing E2E. If fixed, add regression. If not fixed, post explicit blocker/gap before claiming the artifact contract complete. |
| Value capture remains absent in v1 | `telemetry_trace_validation_rejects_capture_values` covers config rejection; repair context/validation integration asserts no heap, stack, runtime value, or value snapshot strings. | `cargo test telemetry_trace_validation_rejects_capture_values --lib`; `cargo test --test integration_telemetry traced_option_none_unwrap_emits_repair_context`. |
| Source reconciliation avoids duplicate telemetry work | Stage 10 evidence must link existing source owners, related merged PRs, and test coverage before making changes. Any implementation PR must explain why each changed file is necessary. | Stage 10 evidence comment and PR description; diff inspection proves no duplicate schemas or redundant runtime writers. |

## Live E2E Plan

Canonical interface: CLI.

Provider path: none. The #585 artifact contract is local filesystem telemetry
and must not call OpenAI, Anthropic, GitHub, Slack, Studio, remote telemetry
collectors, or any external provider.

Required credentials or environment variables:

- no credentials.
- `DUUMBI_TELEMETRY_DIR` for direct binary local override tests.
- no `DUUMBI_TELEMETRY_DIR` for the configured `artifact-dir` workspace test.

Expected external LLM calls: 0.

Estimated external LLM cost: USD 0.

Recommended live traced-failure smoke:

```sh
cargo build
tmp="$(mktemp -d)"
target/debug/duumbi build --trace \
  tests/fixtures/telemetry/option_none_unwrap.jsonld \
  -o "$tmp/panic-fixture"
DUUMBI_TELEMETRY_DIR="$tmp/telemetry" "$tmp/panic-fixture" || true
test -f "$tmp/telemetry/trace_map.json"
test -f "$tmp/telemetry/traces.jsonl"
test -f "$tmp/telemetry/crash_dump.jsonl"
target/debug/duumbi telemetry inspect --telemetry-dir "$tmp/telemetry"
```

Pass criteria:

- build succeeds.
- run exits nonzero with `duumbi panic: called Option::unwrap() on a None value`
  visible on stderr.
- all three artifacts exist under the selected telemetry directory.
- inspection reports `Function: duumbi:telemetry/main`.
- inspection reports `Block: duumbi:telemetry/main/entry`.
- inspection reports exact node evidence as unavailable in v1.
- artifacts parse as JSON/JSONL and trace/crash IDs join the trace map.

Recommended configured artifact-directory smoke:

```sh
cargo build
repo="$(pwd)"
tmp="$(mktemp -d)"
workspace="$tmp/ws"
mkdir -p "$workspace/.duumbi/graph" "$workspace/.duumbi/build"
cp tests/fixtures/telemetry/option_none_unwrap.jsonld \
  "$workspace/.duumbi/graph/main.jsonld"
cat > "$workspace/.duumbi/config.toml" <<'TOML'
[workspace]
name = "telemetry-config-e2e"

[telemetry]
artifact-dir = "custom/telemetry"
TOML
(cd "$workspace" && "$repo/target/debug/duumbi" build --trace -o .duumbi/build/output)
(cd "$workspace" && "$repo/target/debug/duumbi" run || true)
(cd "$workspace" && "$repo/target/debug/duumbi" telemetry inspect)
```

Pass criteria:

- `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` are all under
  `$workspace/custom/telemetry`.
- `duumbi telemetry inspect` without `--telemetry-dir` reads the configured
  directory and reports mapped function/block context.
- default graph files under `.duumbi/graph` are not mutated.

Failure handling:

- If `trace_map.json` is under `custom/telemetry` but runtime JSONL is under
  `.duumbi/telemetry`, do not claim the configured artifact-directory BDD is
  satisfied.
- Either add a narrow CLI/workspace-scoped fix or report the gap and stop for a
  product/architecture decision if direct binary config support is requested.

Manual evidence allowed only when local platform limitations are pre-existing
and verified:

- If local native link tests fail with an environment/toolchain issue already
  reproduced on clean `main`, record the exact failure and require GitHub
  Ubuntu/Windows CI or another supported environment before review readiness.
- Do not use local platform failure as a reason to skip BDD evidence.

## Ralph Cycle Protocol

Each Stage 10 cycle must:

1. summarize the current state and remaining unmet #585 requirements.
2. propose one bounded implementation or evidence goal.
3. list intended file areas and commands before editing.
4. estimate resource use and implementation risk.
5. check whether the resource gate requires human approval.
6. implement only the approved or resource-permitted goal.
7. run the agreed checks.
8. report evidence, failures, review implications, and remaining gaps.
9. stop if requirements are met, a blocker appears, thresholds are exceeded,
   scope changes, or the autonomous batch cap is reached.

The first cycle must be an audit/evidence cycle. It may produce no code if the
contract is already proven. Later cycles may edit source only for named gaps.

## Cycle Budget

- Default cycle size: one bounded implementation or evidence goal per cycle.
- Max files or modules per cycle:
  - Audit-only cycle: no source edits.
  - Test-hardening cycle: up to 2 test/source-test files.
  - Narrow CLI/workspace hardening cycle: up to 3 files across
    `src/main.rs`, `src/workspace.rs`, `src/cli/commands.rs`,
    `src/cli/mod.rs`, and `tests/integration_telemetry.rs`.
  - Runtime/compiler cycle: stop for human approval before editing.
- Expected command budget per cycle:
  - `git diff --check`.
  - `cargo fmt --check` when Rust files changed.
  - one to three focused `cargo test` filters.
  - `cargo clippy --all-targets -- -D warnings` before implementation PR
    review readiness when Rust files changed.
  - `cargo test --all` before review readiness unless a verified platform
    blocker requires CI substitution.
- Human approval required when planned external LLM usage exceeds USD 2,
  exceeds 10 calls, exceeds approved scope, changes runtime/compiler behavior,
  adds risky dependencies, changes schemas/artifact names, introduces
  migrations, weakens security/privacy boundaries, performs irreversible
  operations, conflicts with related approved specs, or needs a product or
  architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Default external LLM budget for this issue: 0 calls / USD 0.
- Autonomous batch cap: three low-budget cycles after Stage 9 approval.
- When to stop and ask for human guidance:
  - configured artifact-directory behavior requires direct binary config
    support instead of CLI/env behavior.
  - current source contradicts the approved product spec.
  - a fix would require runtime C or compiler lowering changes.
  - tests expose a platform/toolchain blocker outside #585.
  - the implementation needs new artifact fields, remote telemetry, value
    capture, exact-node evidence, or repair behavior.

## Task Breakdown

1. Verify workflow context.
   - Confirm #585 is open with `product-spec-approved` and `needs-tech-spec` or
     later Stage 8/9 labels.
   - Confirm product spec PR #660 is merged and Stage 7 approval has no blocking
     findings.

2. Build the #585 evidence matrix.
   - Map all product BDD scenarios to current tests/source.
   - Identify satisfied, weak, and missing evidence.
   - Link related closed issues and merged PRs.

3. Run baseline checks.
   - `cargo test telemetry --lib`
   - `cargo test --test integration_telemetry`
   - focused CLI smoke if needed.
   - Record local platform blockers separately from product failures.

4. Harden inspect-specific fail-closed evidence if needed.
   - Add unit tests for malformed crash/map artifacts through
     `inspect_crash_artifacts()` if current coverage is repair-context-only.
   - Preserve existing `TelemetryError::Parse`, `MissingEvidence`, and
     `Unmapped` semantics.

5. Prove configured artifact-directory behavior.
   - Add or run a workspace-level E2E around `[telemetry] artifact-dir`.
   - If the artifacts split, choose the narrowest approved path:
     - fix CLI/workspace execution path if the change is local and low risk.
     - otherwise report the gap and request product/architecture guidance.

6. Confirm privacy and scope boundaries.
   - Validate `capture-values = true` rejection.
   - Confirm no runtime value, heap, stack, or value snapshot fields appear in
     telemetry-derived evidence.
   - Confirm no remote telemetry or provider behavior was added.

7. Prepare implementation or evidence PR.
   - If source changed, open implementation PR with non-closing issue references
     and full BDD evidence.
   - If no source changed and all behavior is proven, post Stage 10 evidence
     according to the workflow and ask the appropriate next-stage routing agent
     to proceed.

8. Before review readiness.
   - Run required local checks or document authorized CI substitution.
   - Run Codex self-review.
   - Wait for required automated reviewer evidence on any file-based PR.
   - Address blocking feedback and resolve all review threads after verifying
     fixes.

## Verification Plan

Required local checks for a test/code implementation PR:

- `git diff --check`
- `cargo fmt --check`
- `cargo test telemetry --lib`
- `cargo test --test integration_telemetry`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all` before review readiness, unless an environment blocker is
  verified against clean `main` and GitHub CI provides the supported-runner
  substitute.

Focused checks by area:

- Trace map schema and collision behavior:
  - `cargo test trace_map --lib`
- Inspection mapping and fail-closed behavior:
  - `cargo test inspect_crash_artifacts --lib`
- Artifact path config behavior:
  - focused integration test in `tests/integration_telemetry.rs`.
- CLI parser/inspection behavior:
  - `cargo test telemetry_inspect_parses`
  - live `target/debug/duumbi telemetry inspect` smoke.
- Privacy boundary:
  - `cargo test telemetry_trace_validation_rejects_capture_values --lib`
  - repair-context/validation serialization checks that assert no heap, stack,
    runtime value, or value snapshot fields.

Required GitHub evidence for any implementation PR:

- PR changes only approved Stage 10 files.
- CI/check rollup is green.
- Codex self-review has no blocking finding.
- Copilot review evidence is present or unavailable with explicit
  non-actionable infrastructure rationale.
- Greptile is not invoked unless the developer explicitly requests manual deep
  review.
- All review threads are resolved after verifying fixes.

Required evidence-only outcome when no code changes are needed:

- scenario-by-scenario source/test evidence matrix.
- links to #580, #583, #584, #586, #588, and #587 completion evidence.
- current `main` commit used for audit.
- commands/checks run and results.
- statement that no duplicate telemetry schema/writer/implementation was added.
- explicit handling of configured artifact-directory behavior.

## Completion Criteria

Before #585 implementation work can be recommended for review or completion:

- Every product BDD scenario has concrete source/test/manual evidence.
- `trace_map.json`, `traces.jsonl`, and `crash_dump.jsonl` are proven together
  for traced local runtime failure evidence.
- Trace-map entries preserve deterministic graph function/block identity.
- Crash evidence joins to same-kind function/block trace-map entries.
- `duumbi telemetry inspect` reports graph context only after a valid join.
- Default untraced failures are not accepted as back-mapping proof.
- Missing, malformed, untraced, and unmapped evidence fails closed.
- `DUUMBI_TELEMETRY_DIR` override behavior is proven.
- Configured `artifact-dir` behavior is either proven through accepted CLI
  execution or explicitly reported as a remaining product/architecture gap.
- Runtime value/argument capture remains absent and unsupported.
- Original panic/stderr behavior is preserved.
- Generated telemetry artifacts are not committed.
- No remote telemetry, provider calls, Studio UI, repair generation, repair
  acceptance, hot-swap, or production self-healing behavior is added.
- All checks and required automated reviews for any PR are clean.
- The execution issue remains open for later DUUMBI workflow stages.

## Failure And Escalation

- If baseline evidence already satisfies the contract, do not add duplicate
  code. Post evidence and route according to Stage 10/Stage 11 workflow.
- If tests fail due product behavior, fix only the smallest relevant area.
- If tests fail due local native-link/toolchain behavior, reproduce or check
  clean `main`, record exact failure, and use GitHub CI or a supported runner
  only when the blocker is demonstrably environmental.
- If configured artifact-directory behavior splits artifacts, do not claim
  completion. Either add the narrow approved fix or request human guidance.
- If a fix requires runtime C, compiler lowering, artifact schema changes, or
  direct-binary config embedding, stop for human approval before editing.
- If a reviewer requests remote telemetry, value capture, exact-node evidence,
  repair generation, or production self-healing, classify it as out of scope and
  ask for a separate product decision.
- If review feedback is blocking, patch only the approved technical area,
  rerun checks, push, wait for reviewers/checks, and resolve threads only after
  verifying the fix.
- If review feedback is non-blocking, either address it cheaply or document why
  it remains accepted risk.

## Open Questions

- None blocking for Stage 10 if the configured artifact-directory behavior is
  treated as an audit requirement with two acceptable outcomes: narrow
  CLI/workspace hardening or explicit gap reporting.
- Direct binary execution with a custom configured artifact directory is not
  assumed for v1. Direct binaries should use `DUUMBI_TELEMETRY_DIR` unless a
  later product/architecture decision approves embedding or loading workspace
  config at runtime.
- Exact node evidence and runtime value capture remain later product questions,
  not #585 implementation work.
