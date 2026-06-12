# DUUMBI-382: Tier 1 Stdlib Ecosystem Smoke Tests

## Summary

Create ecosystem smoke tests and release evidence that prove accepted Tier 1
DUUMBI standard-library modules work after registry-style distribution, not
only when loaded from the developer source tree.

The smoke contract covers the user-visible path:

- discover stdlib modules with `duumbi search stdlib`;
- install a selected module with `duumbi deps add @duumbi/stdlib-name@1.0.0`;
- verify package metadata, manifest, graph files, cache layout, and integrity;
- import the installed module from a clean workspace;
- build and run a minimal program that calls at least one export from that
  module;
- record release evidence that identifies each module, version, registry
  target, package integrity, stage result, and any deferred reason.

Related to #382. This is a Stage 6 product specification only. The execution
issue must remain open for Stage 7 product review, Stage 8 technical
specification, implementation, implementation review, and Stage 12 completion
handling.

#382 is downstream of the Tier 1 stdlib work in #378, #379, #380, and #381. It
must validate the accepted published-module matrix instead of guessing from a
stale wishlist or blindly testing every file under `stdlib/`.

## Problem

DUUMBI now has source-level Tier 1 stdlib modules, registry client behavior,
package archives, cache installation, and several issue-specific integration
tests. Those are necessary, but they do not fully prove the ecosystem promise
that a user can discover an accepted stdlib package, install it into a clean
workspace, import it, build it, and run it.

The current risk is matrix drift:

- issue #382 was originally written when #379, #380, and #381 were still in
  progress;
- the current source tree now contains file, JSON, TCP, HTTP, DB, and server
  modules;
- #380 and #381 have Stage 12 evidence, but #381's source-backed publish matrix
  still records HTTP, optional TLS, and DB rows as deferred in the inspected
  source context;
- source files existing under `stdlib/` are not the same thing as accepted
  registry publication or clean-workspace install evidence;
- a hard-coded smoke matrix would either miss newly accepted modules or fail on
  modules that are intentionally deferred.

Without a durable #382 spec, the release-validation tests can become too loose
to protect users or too strict to run reliably in CI.

## Outcome

When this is done:

- DUUMBI has deterministic default CI smoke tests for the accepted Tier 1
  stdlib module matrix using an embedded/local registry fixture.
- The smoke matrix is derived from accepted module-readiness evidence, not from
  a stale hard-coded future list.
- Matrix drift is visible: if upstream Stage 12/source evidence and the
  publish matrix disagree, the smoke workflow reports a specific mismatch
  rather than silently skipping or inventing a module.
- Each module selected for the embedded smoke matrix is packaged or seeded into
  an isolated registry fixture, discovered by search, installed into a clean
  workspace, imported, built, and run.
- Every smoke failure identifies the module and stage: matrix, package, search,
  install, manifest, cache, integrity, import, build, run, or evidence.
- Default CI does not call `registry.duumbi.dev`, does not require public
  internet, and does not require real credentials.
- A separate guarded production smoke path exists for `registry.duumbi.dev`.
  It runs only with explicit credentials and an explicit environment flag, and
  never by default on untrusted pull requests.
- Release evidence includes a module matrix table, test log, package integrity
  list, and deferred-module list.
- The smoke tests preserve the default workspace dependency policy: `duumbi
  init` remains core-only unless a later accepted product decision changes it.

## Scope

### In Scope

- Add a durable product contract for Tier 1 stdlib ecosystem smoke tests.
- Build the smoke matrix from accepted evidence:
  - issue #378 for `@duumbi/stdlib-io` and `@duumbi/stdlib-file`;
  - issue #379 for `@duumbi/stdlib-json` and `@duumbi/stdlib-net`;
  - issue #380 for `@duumbi/stdlib-http` and `@duumbi/stdlib-db`;
  - issue #381 for `@duumbi/stdlib-server` and Tier 1 publish handoff;
  - current source-backed matrix evidence in `src/registry/publish_matrix.rs`;
  - current source graph and manifest/cache evidence.
- Treat raw `@duumbi/stdlib-tls` as out of the required v1 module matrix unless
  a later accepted product and technical spec creates a separate public TLS
  module. In #380 v1, TLS is verified HTTPS behavior inside
  `@duumbi/stdlib-http`, not a raw public TLS socket module.
- Require an explicit matrix reconciliation step before running module smokes.
  If a module has accepted upstream Stage 12/source evidence but the publish
  matrix still marks it deferred, the workflow must report a matrix-staleness
  failure or update the accepted matrix in the implementation scope before
  claiming release validation success.
- Use embedded/local registry infrastructure for default CI:
  - isolated registry server or local registry fixture;
  - in-memory or temp storage;
  - local test token only when publish requires authentication;
  - random local port;
  - no production registry calls.
- For each selected module:
  - create a clean `TempDir` DUUMBI workspace;
  - configure the workspace registry to the fixture;
  - publish or seed the package into the fixture;
  - run or exercise the equivalent of `duumbi search stdlib`;
  - run or exercise the equivalent of
    `duumbi deps add @duumbi/stdlib-name@1.0.0`;
  - verify cache entry, `manifest.toml`, graph files, exports, and integrity
    metadata;
  - import the installed module from the clean workspace;
  - build a minimal program that calls at least one accepted export;
  - run the compiled program and assert expected output or exit code.
- Add module-specific smoke depth appropriate to the accepted module behavior:
  - `@duumbi/stdlib-math`: deterministic numeric function such as `abs`,
    `max`, or `clamp`;
  - `@duumbi/stdlib-io`: print wrapper or result-returning line output with
    deterministic stdout;
  - `@duumbi/stdlib-lang`: accepted helper such as `assert_true`;
  - `@duumbi/stdlib-string`: deterministic string helper such as `length`,
    `contains`, `find`, `trim`, `to_upper`, or `replace`;
  - `@duumbi/stdlib-file`: temp workspace file path only, with workspace
    confinement preserved;
  - `@duumbi/stdlib-json`: parse, field access or array access, and stringify;
  - `@duumbi/stdlib-net`: loopback TCP fixture with explicit timeouts;
  - `@duumbi/stdlib-http`: loopback HTTP/HTTPS fixture with bounded timeouts,
    if the reconciled matrix marks it publishable or published;
  - `@duumbi/stdlib-db`: temporary SQLite file or `:memory:` database, if the
    reconciled matrix marks it publishable or published;
  - `@duumbi/stdlib-server`: bounded loopback static route with explicit
    `max_requests` and timeout.
- Add a guarded production `registry.duumbi.dev` smoke command, workflow, or
  documented test path that:
  - requires explicit credentials;
  - requires an explicit environment flag;
  - is skipped by default in CI;
  - is disabled on untrusted pull requests;
  - searches, installs, verifies metadata/integrity, builds, and runs against
    the actual production registry only when authorized.
- Produce release evidence that can be attached to the issue or PR:
  - module matrix table;
  - per-module stage results;
  - package integrity list;
  - registry target;
  - deferred module reasons;
  - production smoke skipped/run status.
- Keep the Stage 6 PR non-closing and limited to this product spec file.

### Explicitly Out Of Scope

- Technical specification creation during Stage 6.
- Implementation code, tests, manifests, runtime changes, registry server
  changes, or Ralph cycles during Stage 6.
- Publishing modules to `registry.duumbi.dev`.
- Changing production registry server architecture.
- Adding broad public docs, marketing claims, landing-page content, or launch
  copy.
- Adding unfinished modules to the smoke matrix before their upstream product,
  technical, implementation, and review gates are complete.
- Treating source files alone as proof that a module is published or
  release-ready.
- Running public internet-dependent smoke tests in default CI.
- Reading or printing real registry tokens in logs, issue comments, PR bodies,
  or artifacts.
- Adding integration modules to default `duumbi init` dependencies unless a
  later accepted product decision changes that policy.
- Testing raw TLS socket APIs in v1. #380 explicitly keeps raw TLS out of the
  public v1 module surface.
- Closing #382 from a spec-only PR.
- Approving the product spec during Stage 6.
- Invoking Greptile unless the developer explicitly requests a manual deep
  review.

## Constraints And Assumptions

Facts:

- Issue #382 is open, labeled `accepted` and `needs-spec`, and has Stage 5
  acceptance with `Decision: Accept`, `Remaining open questions: none`, and
  `Next state: Spec Needed` on 2026-06-05.
- The accepted #382 decision says default CI should use an embedded
  `duumbi-registry` test server and should not call `registry.duumbi.dev`.
- The accepted #382 decision says the smoke matrix should come from the
  accepted #381 publish list.
- The accepted #382 decision says future module handling should remain pending
  until upstream implementation and publishing gates complete.
- The accepted #382 decision says minimum smoke depth is search, install,
  import, build, and run with expected output or exit code.
- The accepted #382 decision says production registry verification is opt-in
  and credential-gated.
- Issue #378 is closed with approved product and technical specs for console
  and workspace file I/O.
- Issue #379 is closed with approved product and technical specs for JSON and
  TCP.
- Issue #380 is closed with Stage 12 evidence for HTTP/HTTPS behavior through
  `@duumbi/stdlib-http` and local SQLite behavior through
  `@duumbi/stdlib-db`; raw TLS socket APIs remain out of v1.
- Issue #381 is closed with Stage 12 evidence for `@duumbi/stdlib-server` and
  embedded-registry clean-workspace server verification. Production publish
  remained intentionally gated on human approval, credentials, permissions, and
  registry target.
- The current source tree contains `stdlib/math.jsonld`, `stdlib/io.jsonld`,
  `stdlib/lang.jsonld`, `stdlib/string.jsonld`, `stdlib/file.jsonld`,
  `stdlib/json.jsonld`, `stdlib/net.jsonld`, `stdlib/http.jsonld`,
  `stdlib/db.jsonld`, and `stdlib/server.jsonld`.
- The current source tree contains source manifests for file and server modules
  and cache-generation manifest logic for default and opt-in stdlib modules.
- `src/registry/publish_matrix.rs` records a source-backed Tier 1 matrix, but
  in the inspected source context still marks `@duumbi/stdlib-http`, optional
  `@duumbi/stdlib-tls`, and `@duumbi/stdlib-db` as deferred to #380.
- `tests/kill_criterion_phase7.rs` already demonstrates an embedded registry
  server with in-memory SQLite, random local port, publish, download, and clean
  workspace install verification.
- `tests/integration_duumbi_381_cycle3.rs` already demonstrates an embedded
  registry smoke path for `@duumbi/stdlib-server`.
- `docs/coding-conventions.md` says registry integration tests should use
  isolated `TempDir` workspaces and avoid live registry calls in CI.
- `docs/architecture.md` documents `.duumbi/cache/@scope/name@version/graph`,
  `manifest.toml`, `CHECKSUM`, lockfile integrity, scope-level registry
  routing, and credential storage.
- The active PRD says DUUMBI should be evidence-oriented, human-verifiable, and
  able to answer what proves behavior works.
- The Phase 14 roadmap says DUUMBI must not market a feature that does not
  reliably work, and identifies pre-seeded stdlib modules as an adoption
  requirement.
- DUUMBI workflow guidance requires Codex self-review and actual Copilot review
  evidence for file-based spec PR gates. Greptile is manual-only by default.

Assumptions:

- `registry.duumbi.dev` is the intended production target for public stdlib
  packages, but #382 must not require that service for default CI.
- Production publish evidence may be absent when #382 implementation begins.
  In that case, #382 can still add deterministic embedded-registry smoke tests
  and record production smoke as gated/skipped until production publish is
  authorized and complete.
- The module matrix may need one small source-backed reconciliation update if
  accepted #380/#381 closure evidence has advanced beyond the currently
  recorded matrix.
- The implementation stage can reuse existing registry clients, package code,
  `duumbi init`, dependency-add code, and workspace build/run helpers instead
  of inventing a separate test-only distribution path.
- Per-module smoke fixtures should choose the smallest representative accepted
  export that proves installed-module import/build/run behavior. Deep module
  behavior remains owned by each upstream module's own tests.

Constraints:

- Stage 6 must not create a technical spec or implementation code.
- The spec-only PR must use non-closing issue references such as `Related to
  #382` or `Spec for #382`.
- The execution issue must remain open through later DUUMBI workflow stages.
- Default CI must be deterministic, local, isolated, and independent of public
  registry uptime.
- Network-related smokes must use loopback fixtures, explicit timeouts, and
  bounded request counts.
- Database smokes must use temporary SQLite files or `:memory:` databases and
  clean up after themselves.
- Filesystem smokes must stay inside the temp workspace boundary.
- Production smoke must fail closed when credentials, explicit opt-in flag,
  registry target, or trust context is missing.
- Tokens and credentials must never be printed or persisted in test output.
- Failure output must identify module and stage without requiring the reviewer
  to inspect raw logs.

## Decisions

- **Decision:** Use a file-based product spec at
  `specs/DUUMBI-382/PRODUCT.md`.
  **Evidence:** #382 is cross-module, release-sensitive, CI-facing, and useful
  durable context for Stage 8 and Stage 10.

- **Decision:** Default CI uses embedded/local registry smokes, not the
  production registry.
  **Evidence:** The accepted #382 decision explicitly selects embedded
  `duumbi-registry` for CI and forbids default production calls.

- **Decision:** The smoke matrix is evidence-driven and reconciled before
  running module tests.
  **Evidence:** #382 was written before several upstream issues reached Stage
  12, while current source and issue evidence have advanced. A static matrix
  would drift.

- **Decision:** Source files alone are insufficient for release validation.
  **Evidence:** The product outcome is registry discovery, install, import,
  build, and run from a clean workspace; parsing `stdlib/*.jsonld` does not
  prove that path.

- **Decision:** Production registry smoke is a separate guarded path.
  **Evidence:** Production registry calls depend on credentials, permissions,
  network availability, and live service state. The accepted #382 decision says
  the production smoke must be opt-in/manual or explicitly guarded.

- **Decision:** Raw TLS is not a required v1 smoke module.
  **Evidence:** #380 Stage 12 records that TLS is verified HTTPS behavior
  through `@duumbi/stdlib-http`, and raw TLS socket APIs remain out of v1.

- **Decision:** #382 should report matrix staleness as a first-class failure
  mode.
  **Evidence:** Current inspected source has HTTP/DB modules and #380 closure
  evidence, while the publish matrix still marks HTTP/DB as deferred. Release
  validation must not silently skip accepted modules because of stale metadata.

## Behavior

### Matrix Reconciliation

Before module smokes run, #382 builds a candidate matrix from accepted evidence.

The matrix row for each module includes:

- module name;
- expected version;
- source graph path, when available;
- source manifest or generated manifest source;
- upstream issue;
- upstream state evidence;
- publish matrix state;
- selected smoke state:
  - `required`;
  - `deferred`;
  - `production-gated`;
  - `matrix-mismatch`;
- deferral or mismatch reason.

A module is `required` for embedded CI smoke when:

- the upstream issue has accepted implementation/closure evidence or is a core
  pre-existing module;
- source graph and manifest/export metadata are available;
- the module is in the accepted publish matrix as publishable or published, or
  the implementation updates the matrix as part of the approved #382 scope;
- the module has a safe local fixture for import/build/run verification.

A module is `deferred` when:

- its upstream issue/spec/implementation/review gate is not complete;
- no accepted public module exists;
- it is explicitly not a v1 public module, such as raw `@duumbi/stdlib-tls` in
  the current accepted #380 scope.

A module is `matrix-mismatch` when accepted upstream evidence and the publish
matrix disagree. For example, if #380 is closed with Stage 12 HTTP/DB evidence
but the source-backed matrix still says HTTP/DB are deferred, the smoke
workflow must fail with a targeted message or update the matrix before
declaring #382 successful.

### Embedded Registry Smoke

Default CI smoke uses an isolated embedded/local registry.

For each required module:

- Create or package a module archive with canonical package contents.
- Publish or seed it into the fixture registry.
- Start from a fresh `TempDir` workspace.
- Configure the workspace to use the fixture registry.
- Search for stdlib modules.
- Add the exact module dependency.
- Verify config, cache, manifest, graph files, exports, and integrity.
- Build a minimal importer program.
- Run the program and assert a deterministic result.

The smoke may use direct library helpers or CLI commands, but the evidence must
map to the user-visible command path. If a direct helper replaces a CLI call,
the technical spec must justify why the helper exercises the same behavior.

### Module-Specific Smoke Depth

Each required module gets one minimal behavior smoke at installed-module grain.

The smoke should prove distribution and linking, not re-test every upstream
module edge case.

Expected examples:

| Module | Minimum smoke |
|---|---|
| `@duumbi/stdlib-math` | Import and call a deterministic numeric export such as `abs`, `max`, or `clamp`; assert output or exit code. |
| `@duumbi/stdlib-io` | Import and call a print or line-output export; assert deterministic stdout and exit code. |
| `@duumbi/stdlib-lang` | Import and call `assert_true` or another accepted helper; assert success. |
| `@duumbi/stdlib-string` | Import and call a deterministic string helper; assert output or exit code. |
| `@duumbi/stdlib-file` | Use only temp workspace files; verify workspace-confined read/write or existence behavior. |
| `@duumbi/stdlib-json` | Parse JSON, access a field or array element, stringify or print a deterministic value. |
| `@duumbi/stdlib-net` | Use loopback TCP with explicit timeouts; prove connect/listen/read/write or a safe minimal accepted flow. |
| `@duumbi/stdlib-http` | If required by the reconciled matrix, use loopback HTTP or local HTTPS fixture; prove response status/body/header access with explicit timeout. |
| `@duumbi/stdlib-db` | If required by the reconciled matrix, use SQLite `:memory:` or a temp workspace file; execute/query/read/free. |
| `@duumbi/stdlib-server` | Use loopback static route with explicit timeout and `max_requests`; verify response and process exit. |

### Isolation And Safety

Every smoke run is isolated:

- fresh workspace;
- temp registry state;
- temp cache;
- temp database or in-memory database;
- loopback network only;
- random local ports;
- explicit timeout;
- bounded request count;
- no shared global graph state;
- no production credentials;
- no public internet dependency.

Failure messages must include enough context for triage:

```text
module=@duumbi/stdlib-json stage=import status=failed detail=<short reason>
```

The exact format may be JSON, table output, or test assertion text, but it must
be easy to identify the module and stage.

### Production Registry Smoke

The production smoke path validates actual `registry.duumbi.dev` packages only
when explicitly authorized.

It must require:

- an explicit environment flag or manual command option;
- configured registry credentials when the operation requires credentials;
- a trusted repository/event context;
- no untrusted pull request execution;
- token-safe logging.

Production smoke should:

- run `duumbi search stdlib` against production;
- install published modules into clean workspaces;
- verify package metadata and integrity;
- build and run representative module importers;
- record module/version/integrity evidence.

If production packages have not been published yet, the production smoke reports
`production-gated` with the missing approval/credential/publish evidence. That
is not a default CI failure unless the workflow was explicitly invoked as a
production release gate.

### Release Evidence

#382 completion evidence must include:

- final smoke matrix;
- required modules and deferred modules;
- matrix mismatch status, if any;
- registry target for embedded and production smoke;
- per-module package integrity;
- per-module stage status;
- production smoke run/skipped/gated status;
- local and CI checks;
- known follow-up risks.

This evidence should be linked from the GitHub issue and implementation PR so
Stage 11 and Stage 12 can verify that the release-validation contract was met.

## BDD Scenarios

Feature: Tier 1 stdlib ecosystem smoke validation

Rule: The smoke matrix is evidence-driven

Scenario: Accepted module is included in the embedded smoke matrix
Given `@duumbi/stdlib-json` has accepted upstream Stage 12 evidence
And its source graph and manifest/export metadata are available
And the accepted publish matrix marks it publishable or published
When #382 builds the embedded smoke matrix
Then `@duumbi/stdlib-json` is marked `required`
And the smoke plan includes package, search, install, manifest, import, build,
and run stages for that module

Scenario: Deferred module is skipped with a reason
Given raw `@duumbi/stdlib-tls` is not an accepted public v1 module
When #382 builds the embedded smoke matrix
Then raw `@duumbi/stdlib-tls` is marked `deferred`
And the reason states that #380 v1 keeps TLS as verified HTTPS behavior inside
`@duumbi/stdlib-http`

Scenario: Stale publish matrix row fails clearly
Given issue #380 has Stage 12 evidence for `@duumbi/stdlib-http`
And the source tree contains `stdlib/http.jsonld`
And the source-backed publish matrix still marks `@duumbi/stdlib-http` as
`deferred-upstream`
When #382 reconciles module readiness
Then the workflow reports `matrix-mismatch` for `@duumbi/stdlib-http`
And it does not claim release-validation success until the matrix is corrected
or the deferral is explicitly justified by accepted evidence

Rule: Default CI uses an embedded registry

Scenario: A required module is discovered and installed from a fixture registry
Given an embedded registry fixture contains `@duumbi/stdlib-string@1.0.0`
And a fresh DUUMBI workspace is configured to use that fixture registry
When the smoke runs the search and dependency-add path
Then `duumbi search stdlib` finds `@duumbi/stdlib-string`
And `duumbi deps add @duumbi/stdlib-string@1.0.0` installs it into the
workspace cache
And the workspace config records the exact installed version

Scenario: Installed package metadata is verified
Given `@duumbi/stdlib-file@1.0.0` was installed into a clean workspace
When the smoke verifies the cache entry
Then the cache contains `manifest.toml`
And the cache contains one or more `graph/*.jsonld` files
And integrity metadata exists
And the manifest export list includes the accepted file exports

Scenario: Installed module can be imported, built, and run
Given a clean workspace installed `@duumbi/stdlib-math@1.0.0`
And a minimal program imports the module and calls `abs`
When the smoke builds and runs the workspace
Then the build succeeds
And the program exits with the expected output or exit code

Rule: Module smokes remain local and bounded

Scenario: Network module uses loopback and explicit timeouts
Given `@duumbi/stdlib-net` is required by the smoke matrix
When the module smoke runs
Then it uses a loopback TCP fixture
And every blocking operation has an explicit timeout
And the test cannot hang indefinitely

Scenario: HTTP module does not require public internet
Given `@duumbi/stdlib-http` is required by the reconciled matrix
When the module smoke runs in default CI
Then it uses a local loopback HTTP or HTTPS fixture
And it does not call a public URL
And it verifies status, body, or headers through the installed module

Scenario: DB module uses temporary local storage
Given `@duumbi/stdlib-db` is required by the reconciled matrix
When the module smoke runs
Then it uses SQLite `:memory:` or a temp workspace file
And it executes, queries, reads, and frees resources
And no shared database file remains after the test

Scenario: Server module exits after one request
Given `@duumbi/stdlib-server` is required by the smoke matrix
And the test program registers a static `GET /health` route
When a loopback client requests `/health`
Then the client receives the configured response
And `server_start` returns after `max_requests` is reached
And no long-running server process remains

Rule: Production registry smoke is guarded

Scenario: Production smoke is skipped by default
Given default CI runs for a pull request
When the production registry smoke job is evaluated
Then it does not call `registry.duumbi.dev`
And it reports that production smoke is skipped or gated

Scenario: Production smoke fails closed without explicit opt-in
Given production registry credentials exist
But the explicit production-smoke flag is absent
When the production smoke command is invoked
Then the command exits without searching or installing from production
And the output says the explicit opt-in flag is required

Scenario: Authorized production smoke verifies published modules
Given `registry.duumbi.dev` contains accepted published stdlib modules
And credentials and the explicit production-smoke flag are present
When the production smoke runs
Then it searches for stdlib modules in production
And it installs each selected published module into a clean workspace
And it verifies package metadata, integrity, import, build, and run behavior
And it records module/version/integrity evidence without printing tokens

Rule: Release evidence is reviewable

Scenario: Failure identifies module and stage
Given the smoke cannot import `@duumbi/stdlib-json` after installation
When the test fails
Then the failure output identifies module `@duumbi/stdlib-json`
And the failure output identifies stage `import`
And the failure output includes a concise reason

Scenario: Completion evidence lists required and deferred modules
Given all embedded smoke tests pass
When #382 records release-validation evidence
Then the evidence includes a module matrix table
And it lists every required module result
And it lists every deferred or production-gated module with its reason
And it includes package integrity values for tested packages

## Tasks

- Draft and approve this product spec through Stage 7.
- Draft an agent-facing technical spec in Stage 8 after product approval.
- Reconcile the Tier 1 module matrix using upstream issue evidence, source
  evidence, and `src/registry/publish_matrix.rs`.
- Add or update matrix evidence so accepted modules are not silently skipped.
- Build embedded/local registry smoke infrastructure from existing registry
  test patterns.
- Add per-module smoke fixtures for every required module.
- Verify package contents, manifests, graph files, exports, cache layout, and
  integrity.
- Exercise search, dependency add, import, build, and run paths from clean
  workspaces.
- Add guarded production registry smoke behavior.
- Record release evidence in the implementation PR and linked issue.

Tasks that can run independently after Stage 8:

- Matrix reconciliation and evidence formatting.
- Embedded registry fixture setup.
- Core modules smoke fixtures.
- Network/server modules smoke fixtures.
- HTTP/DB smoke fixtures after matrix reconciliation marks them required.
- Production smoke guard and skip/gated evidence.

## Checks

Product/spec checks:

- Stage 6 spec PR changes only `specs/DUUMBI-382/PRODUCT.md`.
- PR title, body, commit message, and spec use non-closing issue references.
- PR body includes a workflow note that the spec-only PR must leave issue #382
  open.
- Codex self-review finds no blocking product-spec issues.
- Copilot submits an actual non-dismissed review for the file-based spec PR.
- All required review threads are resolved before routing the issue to
  `Spec Review`.
- Greptile is not invoked unless the developer explicitly requests it.

Implementation checks for later stages:

- Matrix reconciliation detects accepted/deferred/mismatch state for every
  Tier 1 candidate.
- Embedded registry smokes use isolated temp state and no production registry.
- Every required module passes search, install, manifest/cache/integrity,
  import, build, and run stages.
- Module-specific smoke fixtures are deterministic and local.
- Network/server tests use loopback, explicit timeouts, and bounded request
  counts.
- DB tests use SQLite `:memory:` or temp workspace files.
- File tests stay inside the temp workspace boundary.
- Production smoke is skipped/gated by default and cannot run on untrusted pull
  requests.
- Production smoke requires explicit opt-in and token-safe logging.
- Failure messages identify module and stage.
- Release evidence includes module matrix, stage results, integrity values,
  registry target, production smoke status, and deferred/mismatch reasons.
- Default `duumbi init` dependency policy remains unchanged unless a later
  accepted spec changes it.

Expected artifacts:

- `specs/DUUMBI-382/PRODUCT.md`
- later `specs/DUUMBI-382/TECHNICAL.md` only after Stage 7 approval
- implementation PR evidence only after Stage 9 approval
- module matrix evidence
- embedded registry smoke test results
- production smoke skipped/gated/run evidence
- package integrity list

## Open Questions

No blocking Stage 6 questions remain. The following are non-blocking
implementation or release questions because the accepted product behavior above
defines safe defaults:

- Will #382 update `src/registry/publish_matrix.rs` directly when accepted
  upstream evidence has advanced, or will it generate a separate smoke matrix
  from issue/source evidence?
- Which exact minimal export should each module smoke use when multiple
  equivalent deterministic exports exist?
- Are production `registry.duumbi.dev` credentials and publish permissions
  available when the production smoke path is manually invoked?
- Should production smoke report be attached as a PR artifact, an issue comment,
  or both after authorized release validation?

## Sources

- GitHub issue #382:
  `https://github.com/hgahub/duumbi/issues/382`
- Stage 5 acceptance comment for #382:
  `https://github.com/hgahub/duumbi/issues/382#issuecomment-4635079074`
- Accepted v1 decision comment for #382:
  `https://github.com/hgahub/duumbi/issues/382#issuecomment-4635074640`
- Related completed issue #378:
  `https://github.com/hgahub/duumbi/issues/378`
- Related completed issue #379:
  `https://github.com/hgahub/duumbi/issues/379`
- Related completed issue #380:
  `https://github.com/hgahub/duumbi/issues/380`
- Related completed issue #381:
  `https://github.com/hgahub/duumbi/issues/381`
- Existing product specs:
  `specs/DUUMBI-378/PRODUCT.md`, `specs/DUUMBI-379/PRODUCT.md`,
  `specs/DUUMBI-380/PRODUCT.md`, `specs/DUUMBI-381/PRODUCT.md`
- Existing technical specs:
  `specs/DUUMBI-378/TECHNICAL.md`, `specs/DUUMBI-379/TECHNICAL.md`,
  `specs/DUUMBI-380/TECHNICAL.md`, `specs/DUUMBI-381/TECHNICAL.md`
- Publish matrix:
  `src/registry/publish_matrix.rs`
- Embedded registry test pattern:
  `tests/kill_criterion_phase7.rs`
- Existing server embedded-registry smoke evidence:
  `tests/integration_duumbi_381_cycle3.rs`
- Registry packaging:
  `src/registry/package.rs`
- Registry client:
  `src/registry/client.rs`
- Dependency CLI:
  `src/cli/deps.rs`
- Init/default stdlib wiring:
  `src/cli/init.rs`
- Architecture reference:
  `docs/architecture.md`
- Coding conventions:
  `docs/coding-conventions.md`
- Stdlib graph files:
  `stdlib/math.jsonld`, `stdlib/io.jsonld`, `stdlib/lang.jsonld`,
  `stdlib/string.jsonld`, `stdlib/file.jsonld`, `stdlib/json.jsonld`,
  `stdlib/net.jsonld`, `stdlib/http.jsonld`, `stdlib/db.jsonld`,
  `stdlib/server.jsonld`
- DUUMBI vault note: `DUUMBI - PRD`
- DUUMBI vault note: `DUUMBI - Glossary`
- DUUMBI vault note: `DUUMBI Agentic Development Map`
- DUUMBI vault note: `DUUMBI - Agentic Development Runbook`
- DUUMBI vault note: `DUUMBI Registry Architecture`
- DUUMBI vault note: `Module Package Lifecycle`
- DUUMBI vault note: `Registry Authentication Model`
- DUUMBI vault note: `AI Code Review Service Policy`
- DUUMBI vault note: `DUUMBI - Phase 14 - Marketing & Go-to-Market`
