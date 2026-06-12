# DUUMBI-382: Tier 1 Stdlib Ecosystem Smoke Tests - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-382/PRODUCT.md` by adding
release-validation smoke coverage for the Tier 1 standard-library ecosystem.
The implementation must prove that accepted stdlib modules can be discovered,
installed, imported, built, and run from clean DUUMBI workspaces through a
registry-style distribution path.

Related to #382. This is a Stage 8 technical specification only. The execution
issue must remain open for Stage 9 technical review, Stage 10 implementation,
Stage 11 implementation review, and Stage 12 completion handling.

The accepted outcome requires:

- a reconciled source-backed Tier 1 smoke matrix;
- deterministic default CI smokes against an embedded/local registry fixture;
- one minimal installed-module import/build/run smoke for each required
  module;
- guarded production `registry.duumbi.dev` smoke behavior that is opt-in,
  credential-gated, and disabled by default on untrusted pull requests;
- reviewable release evidence that names module, version, registry target,
  package integrity, stage result, and deferred or gated reason.

Do not publish to production, change registry server architecture, change
default `duumbi init` dependency policy, expose raw TLS socket APIs, or start
Ralph cycles during Stage 8 or Stage 9.

## Agent Audience

Use this spec for:

- Codex implementation agents coordinating Stage 10 Ralph cycles.
- Rust registry/CLI agents working on package, search, dependency-add, cache,
  manifest, integrity, and publish-matrix behavior.
- Test agents building embedded-registry, clean-workspace, local network, local
  database, and production-gate smoke coverage.
- CI/workflow agents adding an opt-in production smoke path without introducing
  live registry calls into default pull-request CI.
- Stage 9 and Stage 11 reviewers checking implementability and evidence.

Do not use this spec to begin implementation during Stage 8 or Stage 9.

## Source Context

Verified source facts:

- Product spec: `specs/DUUMBI-382/PRODUCT.md`.
- Product spec PR: `https://github.com/hgahub/duumbi/pull/691`.
- Product spec merge SHA:
  `566f237c3beb9235796cde816bd18e6e0bdabaf6`.
- GitHub issue: `https://github.com/hgahub/duumbi/issues/382`.
- Stage 5 acceptance:
  `https://github.com/hgahub/duumbi/issues/382#issuecomment-4635079074`.
- Stage 7 product-spec approval:
  `https://github.com/hgahub/duumbi/issues/382#issuecomment-4692076596`.
- Issue #382 is open with labels `accepted`, `product-spec-approved`, and
  `needs-tech-spec`.
- Related upstream issues #378, #379, #380, and #381 are closed with Stage 12
  evidence.
- #380 Stage 12 evidence delivered accepted v1 HTTP/HTTPS behavior through
  `@duumbi/stdlib-http` and local SQLite behavior through
  `@duumbi/stdlib-db`. Raw TLS socket APIs remain out of v1.
- #381 Stage 12 evidence delivered `@duumbi/stdlib-server`, embedded-registry
  clean-workspace server verification, and intentionally left production
  publish gated on human approval, credentials, permissions, and registry
  target.
- Current stdlib source files include:
  - `stdlib/math.jsonld`
  - `stdlib/io.jsonld`
  - `stdlib/lang.jsonld`
  - `stdlib/string.jsonld`
  - `stdlib/file.jsonld`
  - `stdlib/json.jsonld`
  - `stdlib/net.jsonld`
  - `stdlib/http.jsonld`
  - `stdlib/db.jsonld`
  - `stdlib/server.jsonld`
- Current source manifests exist for `stdlib/file.manifest.toml` and
  `stdlib/server.manifest.toml`; other stdlib manifests are generated from
  source metadata in `src/cli/init.rs`.
- `src/registry/publish_matrix.rs` currently marks math, io, lang, string,
  file, JSON, net, and server as `publishable-after-verify`, but still marks
  HTTP, optional TLS, and DB as `deferred-upstream`.
- `src/cli/init.rs` seeds math, io, lang, string, JSON, net, server, HTTP, and
  DB into the cache. Default `[dependencies]` remain math, io, lang, and
  string only.
- `src/cli/init.rs` currently does not seed `@duumbi/stdlib-file` by default.
  File can still be packaged from source because `stdlib/file.jsonld` and
  `stdlib/file.manifest.toml` exist.
- `src/registry/package.rs` packs `manifest.toml`, sorted graph files, and
  `CHECKSUM` from `.duumbi/graph/`.
- `src/registry/client.rs` provides `RegistryClient::search()` and
  `RegistryClient::download_module()`. Download writes cache entries and an
  `.integrity` file.
- `src/cli/deps.rs` implements registry-backed dependency installation and
  writes lockfile evidence after resolving/downloading dependencies.
- `tests/kill_criterion_phase7.rs` demonstrates an embedded
  `duumbi-registry` test server with in-memory SQLite, random loopback port,
  test token, publish, download, cache, and clean-workspace loading.
- `tests/integration_duumbi_381_cycle3.rs` demonstrates embedded-registry
  search/download, clean-workspace import/build/run, server loopback, timeout,
  and default dependency-policy assertions for `@duumbi/stdlib-server`.
- `tests/integration_duumbi380_http.rs`,
  `tests/integration_duumbi380_db.rs`, and
  `tests/integration_duumbi380_e2e.rs` demonstrate local HTTP/HTTPS, local
  SQLite, and HTTP + JSON + SQLite composition behavior for #380.
- `tests/integration_phase9a_stdlib.rs` contains stdlib export and
  cross-module compile/run patterns.
- `docs/coding-conventions.md` says registry integration tests use isolated
  `TempDir` workspaces and avoid live registry calls in CI.
- `docs/architecture.md` documents cache layout,
  `.duumbi/cache/@scope/name@version/graph/`, `manifest.toml`, `CHECKSUM`,
  lockfile integrity, scope-level registry routing, and credentials in
  `~/.duumbi/credentials.toml`.
- `.github/workflows/ci.yml` skips Rust checks for documentation-only PRs and
  runs full Rust checks when `src/`, `tests/`, `runtime/`, Cargo files, or CI
  workflows change.
- `.github/workflows/copilot-review.yml` requests Copilot as the default
  automated PR reviewer and must not invoke Greptile.
- Repo instructions are in `AGENTS.md`.

Relevant Obsidian context:

- `DUUMBI - Agentic Development Runbook`: GitHub is the execution source of
  truth; Stage 7 and Stage 9 AI gates are allowed only when Codex self-review,
  Copilot review evidence, checks, scope, and review threads are clean.
- `AI Code Review Service Policy`: Codex self-review is required, Copilot is
  default automated evidence, CodeRabbit is advisory when present, and Greptile
  is manual-only.
- `DUUMBI Registry Architecture`: registry behavior is tested with embedded
  Axum servers and in-memory SQLite; public metadata/downloads are
  unauthenticated, publish/yank require bearer tokens.
- `Module Package Lifecycle`: packages move through publish, metadata
  indexing, search, download, dependency resolution, and optional yanking.
- `Registry Authentication Model`: CLI credentials belong in the user
  credentials store, not the vault.
- `DUUMBI - Phase 14 - Marketing & Go-to-Market`: do not market features that
  do not reliably work; stdlib registry readiness is adoption-critical.

Assumptions:

- Version `1.0.0` is the expected stdlib smoke version for the Tier 1 modules
  already represented in current source/cache metadata.
- Embedded-registry smoke tests can publish or seed package archives from
  source graph files and generated/source manifest metadata without requiring
  production credentials.
- The implementation can reuse `duumbi-registry` test-server patterns already
  present in integration tests.
- A production smoke path may initially report `production-gated` when
  production packages, credentials, permissions, or explicit opt-in are absent.
  That is not a default CI failure.
- The minimal export chosen for each module may be the smallest deterministic
  accepted export that proves installed-module import/build/run behavior.

Implementation recommendations:

- Treat matrix reconciliation as the first Stage 10 slice. Update
  `src/registry/publish_matrix.rs` so accepted source-backed HTTP and DB rows
  no longer remain stale when #380 evidence is accepted, while raw TLS remains
  deferred.
- Prefer one new integration-test file for the default embedded smoke matrix,
  for example `tests/integration_duumbi382_ecosystem_smoke.rs`, with local
  helper structs inside the test file unless reuse pressure justifies a small
  `tests/support` module.
- Use actual CLI binaries for `duumbi init`, `duumbi search`, and
  `duumbi deps add` wherever practical. If a direct Rust helper replaces one
  CLI call for speed or observability, assert that it calls the same registry,
  package, config, cache, or dependency code path.
- Keep production smoke as an ignored/manual integration test or
  workflow-dispatch path guarded by explicit environment variables. Default CI
  must not contact `registry.duumbi.dev`.

## Affected Areas

Expected Stage 10 implementation changes:

- Publish matrix and matrix tests:
  - `src/registry/publish_matrix.rs`
  - focused `registry::publish_matrix::tests`
- Embedded registry and ecosystem smoke tests:
  - new `tests/integration_duumbi382_ecosystem_smoke.rs`
  - optional small test-support helper if the test file would otherwise become
    unreadable
- Existing fixture patterns to reuse:
  - `tests/kill_criterion_phase7.rs`
  - `tests/integration_duumbi_381_cycle3.rs`
  - `tests/integration_duumbi380_http.rs`
  - `tests/integration_duumbi380_db.rs`
  - `tests/integration_duumbi380_e2e.rs`
  - `tests/integration_phase9a_stdlib.rs`
- Registry/package/client code only if the smoke exposes a real contract gap:
  - `src/registry/package.rs`
  - `src/registry/client.rs`
  - `src/registry/types.rs`
- CLI dependency/search surfaces only if the smoke exposes a real user-visible
  path gap:
  - `src/cli/deps.rs`
  - `src/cli/mod.rs`
  - `src/cli/registry.rs`
  - search command handling in the CLI entry path
- Workspace init/cache behavior only if implementation needs a helper to
  source manifests consistently:
  - `src/cli/init.rs`
  - default dependency tests must continue proving integration modules are
    opt-in
- Production smoke guard:
  - optional new `tests/integration_duumbi382_production_smoke.rs`
  - optional `.github/workflows/production-registry-smoke.yml` with
    `workflow_dispatch` only
- Implementation PR evidence:
  - PR body and issue comments should include the final matrix, per-module
    stage table, integrity list, and production skipped/gated/run state.

Areas that must not change without explicit human approval:

- `specs/DUUMBI-382/PRODUCT.md` after Stage 7 approval.
- Any product spec or technical spec for unrelated issues.
- Runtime behavior or stdlib public APIs unless the smoke reveals a blocking
  defect that cannot be verified without a scoped implementation correction.
- `duumbi init` default dependency policy.
- Production registry server architecture or production publish operations.
- Real credentials, credential storage, or token logging.
- Public-internet dependent behavior in default CI.
- Raw public `@duumbi/stdlib-tls` APIs.
- Generated binaries, runtime assets, or checked-in test output artifacts.

## Technical Approach

### 1. Reconcile The Source-Backed Matrix

Add or update a deterministic matrix validation path before module smokes run.
The final embedded smoke matrix must be derived from accepted evidence rather
than only from `stdlib/` file discovery.

Required final matrix behavior:

| Module | Embedded smoke state | Basis |
|---|---|---|
| `@duumbi/stdlib-math` | required | Core module, source-backed and publishable. |
| `@duumbi/stdlib-io` | required | Core module, source-backed and publishable. |
| `@duumbi/stdlib-lang` | required | Core module, source-backed and publishable. |
| `@duumbi/stdlib-string` | required | Core module, source-backed and publishable. |
| `@duumbi/stdlib-file` | required | #378 closed with Stage 12 evidence and source manifest exists. |
| `@duumbi/stdlib-json` | required | #379 closed with Stage 12 evidence and generated manifest metadata exists. |
| `@duumbi/stdlib-net` | required | #379 closed with Stage 12 evidence and generated manifest metadata exists. |
| `@duumbi/stdlib-http` | required after matrix reconciliation | #380 closed with Stage 12 evidence and source/cache metadata exists. |
| `@duumbi/stdlib-db` | required after matrix reconciliation | #380 closed with Stage 12 evidence and source/cache metadata exists. |
| `@duumbi/stdlib-server` | required | #381 closed with Stage 12 evidence and source manifest exists. |
| raw `@duumbi/stdlib-tls` | deferred | #380 v1 keeps TLS as HTTPS behavior inside `@duumbi/stdlib-http`. |

If implementation starts while `src/registry/publish_matrix.rs` still marks
HTTP or DB as `deferred-upstream`, the first cycle must either:

- update those rows to `PublishableAfterVerify` with source graph paths and
  #380 evidence, then proceed; or
- fail matrix reconciliation with a visible `matrix-mismatch` result and stop
  before claiming #382 success.

Do not silently skip HTTP or DB after #380 Stage 12 evidence is verified.

### 2. Build A Reusable Embedded Registry Smoke Harness

Use the existing embedded `duumbi-registry` pattern:

- in-memory SQLite database;
- temporary archive storage;
- random `127.0.0.1:0` listener;
- local test token only for publish;
- public search/download client without credentials;
- isolated `TempDir` workspaces and caches.

For each required module, the harness should:

1. build a package workspace from source graph and manifest metadata;
2. pack the module with `src/registry/package.rs`;
3. publish or seed the archive into the embedded registry;
4. create a clean workspace with `duumbi init`;
5. configure the workspace to use the embedded registry;
6. exercise `duumbi search stdlib` or the same `RegistryClient::search()` path
   with a CLI-equivalence assertion;
7. exercise `duumbi deps add @duumbi/stdlib-name@1.0.0` or the same
   dependency-install path with a CLI-equivalence assertion;
8. verify `manifest.toml`, one or more `graph/*.jsonld` files, export list,
   cache layout, lockfile/config state, and `.integrity`;
9. write a minimal importer graph;
10. build the workspace;
11. run the binary and assert deterministic stdout or exit code;
12. record a stage result with module and stage names.

The harness should use one module-result struct rather than scattered asserts
when practical, so failures can report:

```text
module=@duumbi/stdlib-json stage=import status=failed detail=<short reason>
```

### 3. Module-Smoke Fixture Depth

Use the smallest accepted export that proves installed-module distribution and
linking. Do not retest every upstream edge case.

| Module | Preferred smoke fixture |
|---|---|
| `@duumbi/stdlib-math` | Import `abs` or `clamp`; print or return a deterministic numeric result. |
| `@duumbi/stdlib-io` | Import a print wrapper or `print_ln`; assert deterministic stdout and exit code. |
| `@duumbi/stdlib-lang` | Import `assert_true`; assert success and exit code `0`. |
| `@duumbi/stdlib-string` | Import `length`, `contains`, `trim`, or `replace`; assert deterministic stdout or exit code. |
| `@duumbi/stdlib-file` | Use a temp workspace file only; verify read/write or existence behavior stays inside the workspace. |
| `@duumbi/stdlib-json` | Parse JSON, access a field or array element, and stringify or print a deterministic value. |
| `@duumbi/stdlib-net` | Use a loopback TCP fixture with explicit connect/read/write timeouts. |
| `@duumbi/stdlib-http` | Use a local loopback HTTP fixture, or local HTTPS only if supported by stable existing helpers; assert status/body/header behavior and no public URL. |
| `@duumbi/stdlib-db` | Use SQLite `:memory:` or a temp workspace file; execute, query, read, and clean up. |
| `@duumbi/stdlib-server` | Reuse the bounded loopback `GET /health` pattern with `max_requests = 1` and explicit timeout. |

Network and server smokes must bound every blocking operation with timeouts and
bounded request counts. Database smokes must use only `:memory:` or temp
workspace paths. File smokes must not touch paths outside the test workspace.

### 4. Production Smoke Guard

Add a production smoke path that is manual or explicitly opt-in. Acceptable
forms are:

- an ignored integration test such as
  `tests/integration_duumbi382_production_smoke.rs`; or
- a `workflow_dispatch`-only GitHub Actions workflow that calls the same Rust
  test/CLI path; or
- both, if the workflow is a thin wrapper over the ignored test.

Required guard behavior:

- default CI and normal pull requests never call `registry.duumbi.dev`;
- untrusted pull-request contexts cannot run the production smoke;
- `DUUMBI_PRODUCTION_REGISTRY_SMOKE=1` or an equivalent explicit flag is
  required;
- production registry target must be explicitly `https://registry.duumbi.dev`
  or a reviewed equivalent production URL;
- credentials are loaded through the normal DUUMBI credentials mechanism when
  needed and are never printed;
- without the explicit flag, the production path exits before search/download
  and reports that opt-in is required;
- if production packages are not present, report `production-gated` with the
  missing package/permission/evidence reason.

Suggested manual command:

```bash
DUUMBI_PRODUCTION_REGISTRY_SMOKE=1 cargo test -p duumbi --test integration_duumbi382_production_smoke -- --ignored --nocapture
```

Expected external LLM calls for production smoke are still `0`; registry HTTP
calls are not LLM calls, but they must be explicit, credential-safe, and
outside default CI.

### 5. Evidence Output

Implementation evidence must include:

- final reconciled matrix table;
- required modules and deferred modules;
- any matrix mismatch result before correction;
- embedded registry target;
- production smoke skipped/gated/run state;
- module, version, and integrity values for every tested package;
- per-module stage status for matrix, package, search, install, manifest,
  cache, integrity, import, build, run, and evidence;
- local and GitHub check results;
- remaining risks, if any.

Evidence can live in the implementation PR body plus issue comments. Do not
commit generated logs unless a later approved workflow explicitly requires a
durable artifact file.

## Invariants

- #382 validates accepted published-module readiness; it does not implement
  missing stdlib feature behavior.
- Source files alone are not sufficient evidence. Search, install, import,
  build, and run must be proven from a clean workspace.
- Default CI uses an embedded/local registry and no production registry calls.
- Production smoke is explicit, trusted-context-only, and credential-safe.
- `duumbi init` default dependencies remain math, io, lang, and string unless
  a later accepted product decision changes that policy.
- Raw public TLS socket APIs remain out of v1.
- All workspaces, caches, registry storage, databases, graph state, and network
  ports used by tests are isolated.
- Loopback network tests use explicit timeouts and bounded request counts.
- SQLite tests use `:memory:` or temp workspace files and clean up after
  themselves.
- Failure output identifies the module and stage without requiring raw-log
  archaeology.
- Greptile remains manual-only and must not be invoked unless the developer
  explicitly requests a manual deep review.
- Stage 10 must stop for human approval when the Ralph resource gate triggers.

## BDD-To-Test Mapping

| Product BDD scenario | Required evidence |
|---|---|
| Accepted module is included in the embedded smoke matrix | Focused matrix test asserts an accepted module such as `@duumbi/stdlib-json` becomes `required` when upstream evidence, source graph, manifest/export metadata, and publish matrix state agree. |
| Deferred module is skipped with a reason | Focused matrix test asserts raw `@duumbi/stdlib-tls` is `deferred` with the #380 v1 reason: TLS is HTTPS behavior inside `@duumbi/stdlib-http`. |
| Stale publish matrix row fails clearly | Focused reconciliation test or first implementation cycle shows HTTP/DB stale rows produce `matrix-mismatch`; final implementation updates matrix or otherwise records accepted deferral evidence before claiming success. |
| A required module is discovered and installed from a fixture registry | Embedded smoke publishes/seeds `@duumbi/stdlib-string@1.0.0`, runs CLI or equivalent search/add path, and asserts cache/config/lock state. |
| Installed package metadata is verified | Embedded smoke verifies `manifest.toml`, `graph/*.jsonld`, export list, cache layout, and `.integrity` for a module such as `@duumbi/stdlib-file`. |
| Installed module can be imported, built, and run | Embedded smoke imports `@duumbi/stdlib-math`, builds a clean workspace, runs the binary, and asserts deterministic stdout or exit code. |
| Network module uses loopback and explicit timeouts | Net module smoke uses `127.0.0.1`, random port, explicit read/write/connect timeout, bounded join/wait, and no public network. |
| HTTP module does not require public internet | HTTP module smoke uses a local loopback HTTP or HTTPS fixture and asserts status/body/header behavior through the installed module. |
| DB module uses temporary local storage | DB module smoke uses SQLite `:memory:` or a temp workspace path, executes/query/reads/cleans up, and leaves no shared DB file. |
| Server module exits after one request | Server module smoke uses `max_requests = 1`, loopback `GET /health`, bounded wait, response assertions, and process exit verification. |
| Production smoke is skipped by default | Default CI workflow and/or ignored production test prove no production registry call occurs without explicit invocation. |
| Production smoke fails closed without explicit opt-in | Production smoke test invoked without the flag exits before search/install and asserts an opt-in-required message. |
| Authorized production smoke verifies published modules | Manual/ignored production path, when credentials and flag exist, searches production, installs selected modules into clean workspaces, verifies metadata/integrity/import/build/run, and records evidence without tokens. |
| Failure identifies module and stage | A negative harness assertion or failure-format unit test proves errors include `module=<name>` and `stage=<stage>`. |
| Completion evidence lists required and deferred modules | Implementation PR/issue evidence includes final matrix, required/deferred/gated lists, per-module stage table, and package integrity values. |

Recommended commands for implementation verification:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -p duumbi registry::publish_matrix::tests --lib
cargo test -p duumbi --test integration_duumbi382_ecosystem_smoke -- --nocapture
cargo test --all
```

Run the production smoke only through the guarded manual path:

```bash
DUUMBI_PRODUCTION_REGISTRY_SMOKE=1 cargo test -p duumbi --test integration_duumbi382_production_smoke -- --ignored --nocapture
```

## Live E2E Plan

Canonical interface: CLI. The ecosystem smoke must prove the user-visible CLI
path for clean workspaces. Use direct Rust helpers only when they exercise the
same package, registry client, dependency, cache, or workspace build/run code
and the test states the equivalence.

Real provider/LLM path: none. #382 is registry, CLI, compiler, runtime, and CI
validation work. It does not require OpenAI, Anthropic, local LLM providers,
agent graph mutation, Intent mode, or TUI/Studio provider behavior.

Required credentials:

- Default embedded smoke: none outside the test process. Embedded registry may
  create a local test token in memory.
- Production smoke: normal DUUMBI registry credentials only when explicitly
  invoked against `registry.duumbi.dev`. Tokens must not be printed.

Environment variables:

- Default embedded smoke: none.
- Production smoke:
  - `DUUMBI_PRODUCTION_REGISTRY_SMOKE=1` or equivalent explicit opt-in.
  - Optional production registry URL override only if reviewed and not used in
    default CI.

Expected external LLM calls: `0`.

Estimated external LLM cost: `USD 0`.

Default E2E commands:

```bash
cargo test -p duumbi --test integration_duumbi382_ecosystem_smoke -- --nocapture
cargo test --all
```

Production E2E command:

```bash
DUUMBI_PRODUCTION_REGISTRY_SMOKE=1 cargo test -p duumbi --test integration_duumbi382_production_smoke -- --ignored --nocapture
```

Pass/fail criteria:

- Pass when every required embedded module completes package, search, install,
  manifest/cache/integrity, import, build, and run stages with deterministic
  output or exit status.
- Pass when deferred/raw TLS and production-gated modules are recorded with
  explicit reasons.
- Fail when an accepted module is silently skipped, a required stage fails, a
  matrix mismatch remains unresolved, a default CI path calls production, a
  token is printed, or a network/server test can hang indefinitely.

TUI and Studio: no full E2E required. If Stage 10 touches TUI or Studio despite
this spec, stop for scope approval because #382 is CLI/test/CI scoped.

## Ralph Cycle Protocol

Each cycle must:

1. summarize current state and remaining unmet requirements;
2. propose one bounded implementation goal;
3. list intended file areas and commands;
4. estimate resource use and risk;
5. check whether the resource gate requires human approval;
6. implement only the approved or resource-permitted goal;
7. run the agreed checks;
8. report evidence, failures, and remaining gaps;
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle:
  - Matrix/reconciliation cycle: up to 3 source/test files.
  - Embedded smoke harness cycle: up to 5 files or 5 modules of smoke coverage.
  - Remaining module/production guard cycle: up to 5 files or the remaining
    required module fixtures plus one guarded production path.
- Expected command budget per cycle:
  - `cargo fmt --check`
  - `git diff --check`
  - focused tests for changed areas
  - `cargo clippy --all-targets -- -D warnings` when Rust source or broad tests
    changed
  - `cargo test --all` before review handoff
- Human approval required when planned external LLM usage exceeds USD 2,
  exceeds 10 calls, exceeds approved scope, adds risky dependencies, changes
  migrations, touches security-sensitive credential behavior, needs production
  publish access, changes registry server architecture, changes default
  dependency policy, changes public stdlib APIs, changes runtime behavior
  outside a smoke-uncovered defect, encounters a blocker, or needs a product or
  architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: 3 low-budget cycles.
- When to stop and ask for human guidance:
  - any gate above is triggered;
  - production credentials or publish permissions are needed;
  - a required module has no safe local fixture;
  - matrix evidence conflicts with accepted issue/spec closure evidence after
    source inspection;
  - implementation would require changing accepted product behavior;
  - tests require public internet in default CI;
  - CI or reviewer findings reveal a scope conflict.

## Task Breakdown

1. Matrix reconciliation and source-backed evidence.
   - Verify current issue/source evidence for #378, #379, #380, and #381.
   - Update `src/registry/publish_matrix.rs` so HTTP and DB match #380 Stage 12
     evidence, while raw TLS remains deferred.
   - Add tests for required/deferred/mismatch behavior and duplicate rows.

2. Embedded registry smoke harness.
   - Extract or replicate the embedded registry setup from existing tests.
   - Add package-workspace generation for source-graph plus manifest metadata.
   - Add common stage result reporting with module and stage names.

3. Core and source-manifest module smokes.
   - Cover math, io, lang, string, file, and server.
   - Verify package metadata, cache layout, `.integrity`, imports, build, and
     run.

4. Integration module smokes.
   - Cover JSON, net, HTTP, and DB using local-only fixtures.
   - Reuse #379/#380 fixture patterns and add explicit timeouts/cleanup.

5. Production smoke guard.
   - Add ignored/manual production smoke path or workflow-dispatch wrapper.
   - Prove default skip and missing-opt-in fail-closed behavior.
   - Document manual command and evidence expectations in the implementation PR
     body.

6. Evidence consolidation.
   - Update implementation PR body and issue comment with the final matrix,
     stage table, integrity list, checks, and production smoke state.
   - Run full verification and route to Stage 11 only after checks and required
     review evidence are clean.

## Verification Plan

Local checks before implementation PR review:

- `git diff --check`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test -p duumbi registry::publish_matrix::tests --lib`
- `cargo test -p duumbi --test integration_duumbi382_ecosystem_smoke -- --nocapture`
- `cargo test --all`

Manual/guarded checks:

- Production opt-in guard without flag, proving no production search/install
  occurs.
- Authorized production smoke only when credentials, registry target,
  production packages, and trusted context are explicitly available.

Review evidence:

- Implementation PR body includes BDD-to-test mapping and final matrix.
- Issue comment includes release-validation evidence or links to the PR
  evidence.
- CI checks are green.
- Codex self-review has no blocking findings.
- Required automated reviewer submissions exist and blocking feedback is
  addressed.
- No unresolved review threads remain.

## Completion Criteria

Before Stage 10 can route to Stage 11 review, all of the following must be
true:

- Matrix reconciliation covers every Tier 1 candidate and final evidence does
  not silently skip accepted modules.
- `@duumbi/stdlib-http` and `@duumbi/stdlib-db` are no longer stale-deferred if
  #380 Stage 12 evidence remains accepted in the inspected source context.
- Raw `@duumbi/stdlib-tls` is deferred with an explicit v1 reason.
- Default embedded smoke uses only local/embedded registry infrastructure.
- Every required module completes search, install, manifest/cache/integrity,
  import, build, and run from a clean workspace.
- Network, HTTP, DB, and server smokes are local, bounded, and deterministic.
- Default CI does not call `registry.duumbi.dev`.
- Production smoke is guarded and reports skipped/gated/run state.
- Failure output identifies module and stage.
- Release evidence includes matrix table, stage results, package integrity
  values, registry target, production smoke status, and deferred reasons.
- Default dependency policy remains unchanged.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, focused
  #382 tests, and `cargo test --all` pass unless a documented platform-specific
  CI limitation is approved by a human.
- No product, architecture, security, migration, verification, or cost question
  remains unresolved.

## Failure And Escalation

- If matrix evidence conflicts with accepted issue closure evidence, report
  `matrix-mismatch` with module and reason, then either correct the matrix
  inside #382 scope or stop for human guidance.
- If a module cannot be packaged from source graph and manifest metadata, fail
  at `stage=package` and report the missing metadata.
- If CLI search/add cannot be used directly, document the helper equivalence
  and prove the same underlying code path; otherwise stop for review.
- If a test wants public internet or production registry access in default CI,
  reject that path and keep the smoke local.
- If production credentials, registry permissions, or production publish are
  required, stop for human approval.
- If a smoke reveals an upstream module behavior defect, keep the correction as
  narrow as possible and stop for approval if it changes public API, runtime
  architecture, default dependencies, or accepted product scope.
- If resource use exceeds USD 2, 10 external LLM calls, the three-cycle batch
  cap, or the planned file/module cap, stop and request human guidance.
- If CI or automated review finds a blocking issue, address it within the
  approved file areas and rerun the relevant checks before re-requesting review.

## Open Questions

None. The remaining implementation choices are bounded by this spec:

- update the source-backed publish matrix first;
- use embedded registry smoke tests for default CI;
- keep production smoke manual/opt-in and credential-gated;
- keep default dependencies unchanged;
- stop for human approval if a resource, scope, product, architecture,
  security, migration, or production-access gate triggers.
