# DUUMBI-381: Tier 1 Stdlib Server And Registry Publishing

## Summary

Create the final Tier 1 ecosystem bootstrap slice for DUUMBI:

- Add `@duumbi/stdlib-server` as a bounded local HTTP server module for demos
  and smoke tests.
- Publish implementation-ready Tier 1 stdlib modules to `registry.duumbi.dev`
  only after each module has accepted API, manifest, graph, dry-run package,
  and verification evidence.
- Prove that published modules can be searched, installed, imported, built, and
  run from clean DUUMBI workspaces.

Related to #381. This is a Stage 6 product specification only. The execution
issue must remain open for Stage 7 review, Stage 8 technical specification,
implementation, implementation review, and Stage 12 completion handling.

The product boundary is intentionally staged: publish modules that are already
implementation-ready first, then publish file, JSON, TCP, HTTP, database, TLS
behavior, and server modules only after their own upstream gates are complete.

## Problem

DUUMBI already has the core registry and dependency machinery: module manifests,
reproducible tarball packaging, archive integrity, registry search, publish,
download, cache installation, and dependency resolution. Recent Tier 1 stdlib
work also added or extended source modules for file I/O, JSON, and TCP.

What remains missing is the final ecosystem bootstrap contract:

- which Tier 1 modules are publishable now;
- which modules must wait for their own product, technical, implementation, and
  review gates;
- what `@duumbi/stdlib-server` v1 supports;
- what must be true before a package is published as `1.0.0`;
- what proves a published package is usable by a clean workspace;
- what evidence #381 must hand to #382 for downstream release validation.

Without that contract, #381 can become too broad. It could mix server runtime
design, registry production publishing, unfinished HTTP/database work, and broad
end-to-end validation into one ambiguous build. That creates release-integrity
risk because publishing an unstable `1.0.0` module makes the API harder to
change later.

## Outcome

When this is done:

- DUUMBI has a recorded Tier 1 publishing matrix that separates ready,
  publishable modules from accepted-but-not-yet-ready modules.
- Already-ready core stdlib modules can be packaged, dry-run published, then
  published to `registry.duumbi.dev` when credentials and permissions are
  verified.
- Newly implemented Tier 1 modules are published only after their own accepted
  product and technical specs, implementation evidence, checks, and review gates
  are complete.
- `@duumbi/stdlib-server` v1 exists as a small local static-route HTTP server
  module with explicit `timeout_ms` and `max_requests` controls.
- Users can discover published stdlib modules with `duumbi search stdlib`.
- Users can install published stdlib modules with
  `duumbi deps add @duumbi/stdlib-name@1.0.0`.
- Clean workspaces can import, build, and run minimal programs that call at
  least one export from every published module.
- Production publish attempts fail visibly when registry credentials,
  permissions, or registry target configuration are missing.
- Default `duumbi init` dependencies remain focused on the core modules unless
  a later accepted product decision changes default workspace contents.
- #382 receives a concrete module matrix and evidence contract for broader
  ecosystem smoke tests.

## Scope

### In Scope

- Define the Tier 1 publish matrix for #381.
- Publish implementation-ready Tier 1 modules first:
  - `@duumbi/stdlib-math`
  - `@duumbi/stdlib-io`
  - `@duumbi/stdlib-lang`
  - `@duumbi/stdlib-string`
  - `@duumbi/stdlib-file`, when implementation evidence from #378 is present
  - `@duumbi/stdlib-json`, when implementation evidence from #379 is present
  - `@duumbi/stdlib-net`, when implementation evidence from #379 is present
- Treat `@duumbi/stdlib-http`, any separate `@duumbi/stdlib-tls` package, and
  `@duumbi/stdlib-db` as publishable only after #380 completes its own gates.
- Add `@duumbi/stdlib-server` v1 with:
  - `server_new(host: string, port: i64, timeout_ms: i64) -> result<http_server, string>`
  - `route_add_static(server: http_server, method: string, path: string, status: i64, headers: json, body: string) -> result<i64, string>`
  - `server_start(server: http_server, max_requests: i64, timeout_ms: i64) -> result<i64, string>`
  - `server_close(server: http_server) -> result<i64, string>`
- Require server v1 to bind to loopback by default in tests and examples.
- Require explicit timeouts and request limits so tests and demos cannot hang
  indefinitely.
- Support static routes first, including method, path, response status, headers,
  and body.
- Use `json` headers from #379 when available.
- Represent `http_server` as an opaque runtime-backed resource type at the
  DUUMBI API boundary.
- Define visible `result<_, string>` errors for server, routing, publish,
  search, install, and verification failures.
- Verify each selected module has a valid manifest with name, version,
  description, license, and complete export list.
- Verify each selected module has valid graph files and parse/build validation
  appropriate to the module.
- Run `duumbi publish --dry-run` for each selected module before production
  publishing.
- Publish selected modules to the accepted registry target only after
  credentials, permissions, and target URL are verified.
- Verify `duumbi search stdlib` finds each published module.
- Verify `duumbi deps add @duumbi/stdlib-name@1.0.0` installs each published
  module into a clean workspace.
- Verify at least one minimal clean-workspace import/build/run path per
  published module.
- Hand the final module matrix and evidence expectations to #382 for broader
  release validation.
- Keep the spec PR non-closing and limited to this product spec file.

### Explicitly Out Of Scope

- Technical specification creation during Stage 6.
- Implementation code, tests, manifests, runtime changes, registry server
  changes, or Ralph cycles during Stage 6.
- Publishing modules whose upstream implementation or review gates are not
  complete.
- Treating `@duumbi/stdlib-server` v1 as a production web framework.
- Dynamic DUUMBI request-handler callbacks, middleware chains, streaming
  responses, WebSockets, HTTP/2, TLS termination, reverse proxy behavior,
  daemon supervision, connection pooling, and async/event-loop abstractions.
- Internet-dependent tests in default CI.
- Changing the `duumbi-registry` server unless publish/install verification
  proves a missing server capability.
- Adding integration modules to default `duumbi init` dependencies unless a
  later accepted product decision explicitly changes default workspace policy.
- Closing #381 from a spec-only PR.
- Approving the product spec during Stage 6.
- Invoking Greptile unless the developer explicitly requests a manual deep
  review.

## Constraints And Assumptions

Facts:

- Issue #381 is open and has Stage 5 acceptance with `Decision: Accept`,
  `Remaining open questions: none`, and `Next state: Spec Needed`.
- The accepted #381 decisions say to publish implementation-ready modules first
  and not block ready core modules on #379, #380, or server completion.
- The accepted #381 decisions narrow server v1 to static local routes with
  explicit `timeout_ms` and `max_requests`.
- The accepted #381 decisions remove middleware from v1.
- Issue #378 has completed Stage 12 and merged implementation evidence for
  `@duumbi/stdlib-io` and `@duumbi/stdlib-file`.
- Issue #379 has completed Stage 12 and merged implementation evidence for
  `@duumbi/stdlib-json` and `@duumbi/stdlib-net`.
- Issue #380 is accepted and still needs its own Stage 6 product spec before
  HTTP, TLS, or database modules should be treated as publish-ready.
- Issue #382 is the downstream ecosystem smoke-test issue, but it had not yet
  reached Stage 5 acceptance in the inspected context.
- The current source tree contains `stdlib/math.jsonld`, `stdlib/io.jsonld`,
  `stdlib/lang.jsonld`, `stdlib/string.jsonld`, `stdlib/file.jsonld`,
  `stdlib/json.jsonld`, and `stdlib/net.jsonld`.
- The current source tree does not contain `@duumbi/stdlib-server` graph or
  API artifacts.
- `src/cli/init.rs` writes math, io, lang, string, JSON, and TCP modules into
  the local stdlib cache in the inspected source context. `stdlib-file` exists
  as a source graph file and manifest, but is not wired into `duumbi init`
  cache generation in the inspected context.
- Default `duumbi init` config dependencies still list only math, io, lang, and
  string.
- `src/registry/package.rs` creates reproducible module archives containing
  `manifest.toml`, `graph/*.jsonld`, and `CHECKSUM`.
- `src/cli/publish.rs` validates graphs, packages modules, computes archive
  integrity, supports dry run, verifies credentials, and uploads through the
  registry client.
- `src/registry/client.rs` supports search, version resolution, download,
  integrity storage, publish, and yanking.
- `src/cli/deps.rs` supports registry dependency add, version resolution,
  download to `.duumbi/cache`, and `config.toml` dependency updates.
- The active DUUMBI runbook says GitHub is the execution source of truth,
  product specs are Stage 6 artifacts, Copilot review evidence is required for
  file-based spec gates, and Greptile is manual-only.
- The Phase 14 roadmap says registry seeding matters because an empty registry
  is a major adoption barrier, but it also says DUUMBI must not market features
  that do not reliably work.

Assumptions:

- `registry.duumbi.dev` is the intended production registry target, but the
  implementation stage must verify credentials, permissions, target URL, and
  server compatibility before non-dry-run publishing.
- The first #381 implementation can publish a subset of the Tier 1 matrix if
  the remaining modules are explicitly marked deferred until their own gates
  are complete.
- `@duumbi/stdlib-tls` may be a separate package only if #380 product and
  technical review approve a separate module. If #380 keeps TLS as verified
  HTTPS behavior inside `@duumbi/stdlib-http`, #381 should not invent a
  standalone TLS package.
- Server v1 may depend on the first-class `json` type from #379 for headers.
- Server v1 can use loopback-only tests and examples for deterministic evidence.
- #382 can consume #381's matrix and publish/install evidence after #382 reaches
  its own acceptance and specification gates.

Constraints:

- A package must not be published as `1.0.0` until its API, manifest exports,
  package archive, checks, and review evidence are complete.
- Production publish operations require explicit registry credentials and
  permissions.
- Tests and smoke paths for server/network behavior must be timeout-bounded and
  local by default.
- Server resources must have documented cleanup behavior.
- Every expected failure path in the new server API must return
  `result<_, string>`.
- The spec-only PR must use non-closing issue references such as
  `Related to #381` or `Spec for #381`.

## Decisions

- **Decision:** Use a file-based product spec for #381 at
  `specs/DUUMBI-381/PRODUCT.md`.
  **Evidence:** The accepted work is user-visible, cross-module,
  release-sensitive, and useful as durable context for Stage 8 and later review.

- **Decision:** Publish ready Tier 1 modules first instead of blocking on every
  future integration module.
  **Evidence:** The accepted #381 decision explicitly says not to block ready
  core modules on #379, #380, or server completion.

- **Decision:** The immediate publishable set starts with modules whose
  implementation and review gates are complete.
  **Evidence:** #378 and #379 have Stage 12 completion evidence. #380 does not
  yet have a product spec or implementation evidence in the inspected context.

- **Decision:** `@duumbi/stdlib-server` v1 is static-route, local-demo oriented,
  and bounded by explicit timeouts and request limits.
  **Evidence:** The accepted #381 decision says server v1 is static local
  routes only with explicit `timeout_ms` and `max_requests`.

- **Decision:** Dynamic request handlers and middleware are deferred.
  **Evidence:** The accepted #381 decision removes middleware from v1, and the
  inspected source context does not show an accepted graph callback/event-loop
  model for dynamic request handlers.

- **Decision:** Production publish is gated by dry-run package evidence first.
  **Evidence:** The accepted #381 decision requires dry-run/local package
  evidence before production publish and says production publish requires
  verified credentials, permissions, and registry target.

- **Decision:** New integration modules remain explicit dependencies instead of
  default `duumbi init` dependencies for now.
  **Evidence:** The accepted #381 decision keeps integration modules as
  explicit `duumbi deps add` dependencies, and current `config.toml` default
  dependencies list only math, io, lang, and string.

- **Decision:** Ecosystem-ready requires production publish plus #382
  clean-workspace search/install/build/run evidence.
  **Evidence:** The accepted #381 decision names #382 as the downstream
  clean-workspace E2E gate.

## Behavior

### Tier 1 Publish Matrix

#381 owns the publish matrix and the publish handoff evidence.

The matrix has four states:

- `ready-to-publish`: upstream implementation and review gates are complete;
  module package, dry-run, and local clean-workspace verification can proceed.
- `publishable-after-verify`: upstream implementation is present, but #381 must
  still verify manifest, package, dry-run, production credentials, registry
  target, search, install, and build/run evidence.
- `deferred-upstream`: upstream issue or spec gate is not complete; do not
  publish as part of the current pass.
- `published`: production registry publish completed and clean-workspace
  verification evidence exists.

Initial matrix from inspected context:

| Module | Initial #381 state | Reason |
|---|---|---|
| `@duumbi/stdlib-math` | `publishable-after-verify` | Core source module exists and default init uses it. |
| `@duumbi/stdlib-io` | `publishable-after-verify` | #378 completed Stage 12 and default init uses it. |
| `@duumbi/stdlib-lang` | `publishable-after-verify` | Core source module exists and default init uses it. |
| `@duumbi/stdlib-string` | `publishable-after-verify` | Core source module exists and default init uses it. |
| `@duumbi/stdlib-file` | `publishable-after-verify` | #378 completed Stage 12 and source module exists. |
| `@duumbi/stdlib-json` | `publishable-after-verify` | #379 completed Stage 12 and source module exists. |
| `@duumbi/stdlib-net` | `publishable-after-verify` | #379 completed Stage 12 and source module exists. |
| `@duumbi/stdlib-http` | `deferred-upstream` | #380 is accepted but not yet specified or implemented in the inspected context. |
| `@duumbi/stdlib-tls` if separate | `deferred-upstream` | #380 must decide whether TLS is separate or HTTP behavior. |
| `@duumbi/stdlib-db` | `deferred-upstream` | #380 is accepted but not yet specified or implemented in the inspected context. |
| `@duumbi/stdlib-server` | `deferred-upstream` until implemented by #381 | This issue defines its v1 product contract. |

#381 may complete a publish pass for the publishable subset without claiming the
entire future Tier 1 universe is ecosystem-ready.

### Module Publish Readiness

Before a module is published to production:

- The module has an accepted API and implementation evidence.
- The module has graph files that parse and validate.
- The module has a `manifest.toml` with name, version `1.0.0`, description,
  license, and complete function exports.
- Repeated packaging produces the same archive bytes or the same expected
  reproducibility evidence when the technical spec chooses a testable
  equivalent.
- The archive contains:
  - `manifest.toml`
  - `graph/*.jsonld`
  - `CHECKSUM`
- `duumbi publish --dry-run` succeeds and prints package contents and archive
  integrity.
- Any production publish target is the accepted registry for this module.
- Credentials and permissions are verified without printing tokens.

Expected failure states are visible:

- Missing manifest fields fail before publish.
- Invalid SemVer fails before publish.
- Missing graph files fail before publish.
- Graph validation failures fail before publish.
- Missing registry configuration fails before publish.
- Missing credentials fail before production publish.
- Registry authentication, reachability, or server errors fail visibly.

### Production Publishing

Production publishing targets `registry.duumbi.dev` unless Stage 8 or later
review documents a safer accepted target for a specific environment.

The product contract is:

- Dry-run packaging evidence comes first.
- Non-dry-run publish requires explicit credentials and permissions.
- Tokens are never printed in command output, logs, issue comments, PR bodies,
  or test artifacts.
- Publish success records module name, version, registry target, archive
  integrity, and package contents.
- Publishing an already published version must fail or be handled according to
  accepted registry server behavior; #381 must not silently overwrite a
  released `1.0.0` package without an accepted registry policy.

### Search And Install Verification

For every module published by #381:

- `duumbi search stdlib` finds the module by name or result metadata.
- `duumbi deps add @duumbi/stdlib-name@1.0.0` resolves the module from the
  accepted registry.
- The module downloads into a clean workspace cache path under
  `.duumbi/cache`.
- The installed module has `manifest.toml`, `graph/*.jsonld`, and integrity
  metadata.
- `config.toml` records the installed module with the exact resolved version.
- A minimal graph program imports the module and calls at least one exported
  function.
- The minimal program builds and runs with expected output or exit code.

Search/install/build/run failures identify the failed module and stage:
`search`, `install`, `manifest`, `import`, `build`, `run`, or `integrity`.

### Default Workspace Dependency Policy

`duumbi init` should continue to create reliable starter workspaces.

The default dependencies remain:

- `@duumbi/stdlib-math`
- `@duumbi/stdlib-io`
- `@duumbi/stdlib-lang`
- `@duumbi/stdlib-string`

Additional integration modules remain explicit `duumbi deps add` dependencies
until a later accepted product decision changes default init behavior.

This policy prevents new workspaces from implicitly depending on network,
database, server, TLS, or broader integration behavior before those modules have
stable user expectations and release evidence.

### @duumbi/stdlib-server V1

`@duumbi/stdlib-server` v1 provides a bounded local HTTP server for demos and
smoke tests.

The v1 API is:

- `server_new(host: string, port: i64, timeout_ms: i64) -> result<http_server, string>`
- `route_add_static(server: http_server, method: string, path: string, status: i64, headers: json, body: string) -> result<i64, string>`
- `server_start(server: http_server, max_requests: i64, timeout_ms: i64) -> result<i64, string>`
- `server_close(server: http_server) -> result<i64, string>`

`server_new` behavior:

- Creates an opaque `http_server` resource on success.
- Accepts loopback host values for tests and examples.
- Uses explicit `timeout_ms` so bind/setup behavior cannot hang indefinitely.
- Returns `Err(string)` for invalid host, invalid port, timeout, bind failure,
  unsupported address, or resource allocation failure.

`route_add_static` behavior:

- Adds a static route to a server that has not started yet.
- Accepts method, path, status, JSON headers, and string body.
- Returns `Ok(0)` on success.
- Returns `Err(string)` for invalid method, invalid path, invalid status,
  invalid headers, duplicate route conflict if unsupported, closed server,
  already-started server when mutation is unsupported, or resource failure.

`server_start` behavior:

- Starts serving registered static routes.
- Processes at most `max_requests` requests.
- Uses explicit `timeout_ms` so tests cannot hang indefinitely.
- Returns `Ok(n)` where `n` is the number of requests handled when the server
  stops cleanly.
- Returns `Err(string)` for invalid limits, timeout, runtime failure, closed
  server, or missing route table if the implementation treats that as invalid.

`server_close` behavior:

- Releases the server resource.
- Returns `Ok(0)` when close succeeds.
- Repeated close is safe and returns either `Ok(0)` or a stable `Err(string)`
  according to the technical spec, but must not crash or double free.
- Later operations on a closed server return `Err(string)`.

HTTP behavior:

- Static route matching uses exact method and exact path in v1.
- Unknown routes return a deterministic not-found response or a documented
  `Err(string)`-observable outcome, as Stage 8 specifies.
- Response bodies are DUUMBI strings.
- Response headers use the `json` type from #379.
- Tests and examples use loopback and local clients.

### Deferred Server Behavior

The following are not v1 behavior:

- Dynamic DUUMBI request-handler callbacks.
- Middleware.
- Request body parsing.
- Streaming responses.
- Long-running production daemon behavior.
- TLS termination.
- WebSockets.
- HTTP/2.
- Reverse proxy behavior.
- Production concurrency guarantees.
- Background services that outlive the DUUMBI program without explicit product
  design.

If any of these are needed later, they require separate accepted issues because
they depend on callback/event-loop/resource-lifetime semantics that are not
part of this v1 contract.

### #382 Handoff

#381 must leave enough evidence for #382 to validate the ecosystem release
without guessing.

The handoff should include:

- final module matrix;
- module publish status;
- registry target;
- package integrity values;
- package contents;
- search evidence;
- install evidence;
- clean-workspace build/run evidence;
- known deferred modules and their upstream issues.

#382 owns broad release smoke coverage after it reaches the required workflow
state. #381 should not expand into the whole #382 test matrix beyond the
minimal per-module verification needed to prove published packages are usable.

## BDD Scenarios

Feature: Tier 1 stdlib publishing and server bootstrap

Rule: Modules are published only after readiness evidence exists

Scenario: Ready core module passes dry-run package verification
Given `@duumbi/stdlib-math` has graph files and a complete manifest
When the release agent runs the accepted dry-run publish command
Then the command succeeds without uploading
And the output lists `manifest.toml`, `graph/*.jsonld`, `CHECKSUM`, and archive integrity

Scenario: Module with incomplete upstream gates is deferred
Given `@duumbi/stdlib-http` belongs to issue #380
And issue #380 has not completed its own spec and implementation gates
When #381 builds the publish matrix
Then `@duumbi/stdlib-http` is marked `deferred-upstream`
And no production publish is attempted for that module

Scenario: Production publish is blocked without credentials
Given a selected module has valid dry-run package evidence
And the target registry requires authentication
And no credential is configured for that registry
When the release agent attempts non-dry-run publish
Then the publish fails before upload
And the error identifies missing authentication without printing secrets

Scenario: Published module is discoverable
Given `@duumbi/stdlib-string@1.0.0` has been published to the accepted registry
When a user runs `duumbi search stdlib`
Then the search results include `@duumbi/stdlib-string`
And the result identifies version `1.0.0` or the available version metadata

Scenario: Published module installs into a clean workspace
Given a clean DUUMBI workspace configured for the accepted registry
And `@duumbi/stdlib-json@1.0.0` has been published
When a user runs `duumbi deps add @duumbi/stdlib-json@1.0.0`
Then the module is downloaded into `.duumbi/cache`
And `config.toml` records the exact installed version
And the installed package includes its manifest, graph files, and integrity metadata

Scenario: Installed module can be imported, built, and run
Given a clean workspace has installed `@duumbi/stdlib-file@1.0.0`
And a minimal graph program imports the module and calls one exported function
When the user builds and runs the program
Then the build succeeds
And the run exits with the expected output or exit code

Rule: `@duumbi/stdlib-server` v1 is local, static, and bounded

Scenario: Static route server returns a configured response
Given a DUUMBI program creates a server on a loopback host
And it adds a static `GET` route for `/health` with status `200`
And it starts the server with `max_requests` set to `1`
When a local HTTP client requests `/health`
Then the client receives status `200`
And the configured body and headers are returned
And `server_start` returns `Ok(1)`

Scenario: Server start stops after the request limit
Given a server has a static route
And `server_start` is called with `max_requests` set to `1`
When one matching local request is handled
Then the server stops cleanly
And no long-running daemon remains

Scenario: Server start times out without hanging
Given a server is started with an explicit short `timeout_ms`
And no request arrives before the timeout
When the timeout is reached
Then `server_start` returns `Err(string)` or a documented timeout outcome
And the test process does not hang indefinitely

Scenario: Invalid route data is reported as an error
Given a DUUMBI program has an open `http_server`
When it calls `route_add_static` with an invalid method or path
Then the function returns `Err(string)`
And no partial route is exposed to later requests

Scenario: Closed server rejects later operations
Given a DUUMBI program creates an `http_server`
And it calls `server_close`
When the program calls `route_add_static` or `server_start` on that closed server
Then the operation returns `Err(string)`
And the runtime does not crash or double free the resource

Rule: Default init remains conservative

Scenario: New workspaces keep the core default dependency set
Given a user creates a new workspace with `duumbi init`
When they inspect `.duumbi/config.toml`
Then the default dependencies include math, io, lang, and string
And integration modules such as JSON, TCP, HTTP, DB, and server are not added
unless a later accepted product decision changes the policy

Rule: #381 leaves release validation evidence for #382

Scenario: Downstream smoke-test issue receives a concrete matrix
Given #381 has published one or more Tier 1 modules
When #381 records its completion evidence
Then the evidence lists every published module, deferred module, registry
target, package integrity, and minimal search/install/build/run result
And #382 can use that matrix as its release-validation input

## Tasks

- Draft and approve this product spec through Stage 7.
- Draft an agent-facing technical spec in Stage 8 after product approval.
- Build a publish matrix from source state, upstream issue state, and accepted
  module readiness evidence.
- Add or verify publishable module workspace artifacts for each selected module.
- Implement `@duumbi/stdlib-server` v1 only after the technical spec is
  approved.
- Run package validation and dry-run publish for every selected module.
- Verify credentials, permissions, and registry target before production
  publish.
- Publish selected ready modules to `registry.duumbi.dev`.
- Verify search, install, cache layout, manifest, integrity, import, build, and
  run behavior from clean workspaces.
- Record deferred modules and the upstream issue that owns each deferral.
- Update #381 with spec, publish, and verification evidence.
- Hand the final published-module matrix to #382.

Tasks that can run independently after Stage 8:

- Publish verification for already-ready core modules.
- Server v1 implementation and local bounded server tests.
- Production registry credential/permission preflight.
- Clean-workspace search/install/build/run smoke verification.
- #382 handoff evidence preparation.

## Checks

Product/spec checks:

- Stage 6 spec PR changes only `specs/DUUMBI-381/PRODUCT.md`.
- PR title, body, commit message, and spec use non-closing issue references.
- PR body includes a workflow note that the spec-only PR must leave issue #381
  open.
- Codex self-review finds no blocking product-spec issues.
- Copilot submits an actual non-dismissed review for the file-based spec PR.
- All required review threads are resolved before routing the issue to
  `Spec Review`.
- Greptile is not invoked unless the developer explicitly requests it.

Implementation checks for later stages:

- Manifest validation covers name, version, description, license, and exports.
- Graph validation succeeds for every selected module.
- Reproducible package checks cover archive contents and integrity.
- `duumbi publish --dry-run` succeeds for every selected module.
- Production publish verifies registry target, credentials, permissions, and
  token-safe logging.
- `duumbi search stdlib` finds every published module.
- `duumbi deps add @duumbi/stdlib-name@1.0.0` works in clean workspaces.
- Installed package verification checks manifest, graph files, cache layout,
  exports, and integrity metadata.
- Minimal import/build/run verification covers at least one export per
  published module.
- Server tests use loopback, explicit `timeout_ms`, and explicit
  `max_requests`.
- Server failure tests cover invalid route data, bind failure where practical,
  timeout, closed resource behavior, and cleanup.
- Network/server tests do not require public internet.
- #382 handoff evidence lists published and deferred modules clearly.

Expected artifacts:

- `specs/DUUMBI-381/PRODUCT.md`
- Stage 7 approval record before Stage 8
- later `specs/DUUMBI-381/TECHNICAL.md` only after Stage 7 approval
- implementation PR evidence only after Stage 9 approval
- module publish matrix
- package integrity list
- search/install/build/run verification evidence
- #382 handoff evidence

## Open Questions

No blocking Stage 6 questions remain. The following are non-blocking
implementation or release questions because the accepted product behavior above
defines safe defaults:

- At implementation time, are `registry.duumbi.dev` credentials and publish
  permissions available for the selected modules?
- Will #380 approve a separate `@duumbi/stdlib-tls` package, or keep TLS as
  verified HTTPS behavior inside `@duumbi/stdlib-http`?
- Should production publish happen in one release PR/pass or in module-grouped
  passes as each upstream module reaches readiness?
- Should #382 be advanced through acceptance/spec review before #381 records
  final ecosystem-ready evidence, or should #381 merely hand off a matrix until
  #382 reaches its own workflow gate?

## Sources

- GitHub issue #381:
  `https://github.com/hgahub/duumbi/issues/381`
- Stage 5 acceptance comment for #381:
  `https://github.com/hgahub/duumbi/issues/381#issuecomment-4634752224`
- Accepted v1 decision comment for #381:
  `https://github.com/hgahub/duumbi/issues/381#issuecomment-4634745862`
- Related completed issue #378:
  `https://github.com/hgahub/duumbi/issues/378`
- Related completed issue #379:
  `https://github.com/hgahub/duumbi/issues/379`
- Related accepted issue #380:
  `https://github.com/hgahub/duumbi/issues/380`
- Downstream smoke-test issue #382:
  `https://github.com/hgahub/duumbi/issues/382`
- Existing product spec:
  `specs/DUUMBI-378/PRODUCT.md`
- Existing product spec:
  `specs/DUUMBI-379/PRODUCT.md`
- Architecture reference:
  `docs/architecture.md`
- Registry packaging:
  `src/registry/package.rs`
- Registry client:
  `src/registry/client.rs`
- Publish CLI:
  `src/cli/publish.rs`
- Dependency CLI:
  `src/cli/deps.rs`
- Init/default stdlib wiring:
  `src/cli/init.rs`
- Stdlib graph files:
  `stdlib/math.jsonld`, `stdlib/io.jsonld`, `stdlib/lang.jsonld`,
  `stdlib/string.jsonld`, `stdlib/file.jsonld`, `stdlib/json.jsonld`,
  `stdlib/net.jsonld`
- Phase 14 roadmap:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 14 - Marketing & Go-to-Market.md`
- Module Package Lifecycle:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Module Package Lifecycle.md`
- DUUMBI Registry Architecture:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/DUUMBI Registry Architecture.md`
- DUUMBI Agentic Development Runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
