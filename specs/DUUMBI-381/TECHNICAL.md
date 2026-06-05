# DUUMBI-381: Tier 1 Stdlib Server And Registry Publishing - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-381/PRODUCT.md` by
delivering the final Tier 1 ecosystem bootstrap slice:

- Build and record a source-backed Tier 1 publish matrix that separates ready
  modules from modules deferred to upstream issues.
- Make already-ready Tier 1 stdlib modules packageable, dry-run publishable,
  and verifiable through search, install, import, build, and run evidence.
- Add `@duumbi/stdlib-server` v1 as a bounded local static-route HTTP server
  module.
- Keep integration modules out of default `duumbi init` dependencies unless a
  later accepted product decision changes that policy.
- Hand a concrete module matrix and verification contract to #382.

Related to #381. This is a Stage 8 technical specification only. The execution
issue must remain open for Stage 9 technical review, Stage 10 implementation,
Stage 11 implementation review, and Stage 12 completion handling.

## Agent Audience

Use this spec for:

- Codex implementation agents coordinating Stage 10 Ralph cycles.
- Rust/compiler agents changing type, parser, graph validation, result-safety,
  lowering, and linker behavior.
- Runtime agents adding the C `http_server` resource and bounded HTTP handling.
- Registry and CLI agents tightening publish validation and package evidence.
- Test agents building local-only registry and loopback HTTP coverage.
- Stage 9 and Stage 11 reviewers checking implementation evidence against the
  accepted BDD behavior.

Do not use this spec to start implementation during Stage 8 or Stage 9.

## Source Context

- Product spec: `specs/DUUMBI-381/PRODUCT.md`.
- Product spec PR: `https://github.com/hgahub/duumbi/pull/669`.
- Product spec merge SHA:
  `6e3686c69e7dcde6dfa9d1179fd41964b9415477`.
- GitHub issue: `https://github.com/hgahub/duumbi/issues/381`.
- Stage 7 product spec approval:
  `https://github.com/hgahub/duumbi/issues/381#issuecomment-4635037851`.
- Repo instructions: `AGENTS.md`.
- Architecture reference: `docs/architecture.md`.
- Coding conventions: `docs/coding-conventions.md`.
- Related upstream product context:
  - #378 completed implementation evidence for file I/O behavior.
  - #379 completed implementation evidence for JSON and TCP behavior.
  - #380 has product-spec approval, but no technical-spec approval or
    implementation completion in the inspected context.
  - #382 is still a downstream release smoke-test handoff target, not a
    prerequisite for #381 implementation.

Relevant code verified for Stage 8:

- `src/types.rs`
  - Current source has first-class `DuumbiType::Json`,
    `DuumbiType::TcpSocket`, and `DuumbiType::TcpListener`.
  - Current source has JSON and TCP operations, but no `HttpServer` type and no
    server operations.
- `src/parser/mod.rs`
  - `parse_type_str()` recognizes `json`, `tcp_socket`, and `tcp_listener`.
  - `parse_op()` maps JSON and TCP `duumbi:*` operation strings.
  - `duumbi:args` parsing currently exists for calls only.
- `src/parser/ast.rs`
  - `OpAst` already contains `operand`, `left`, `right`, and `args`.
- `src/graph/builder.rs`
  - Builder code already materializes `op_ast.args` as indexed argument edges
    for any operation whose parser populates `args`.
- `src/graph/validator.rs`
  - Operation typing exists for runtime-backed operations and must be extended
    for server operations.
- `src/graph/result_safety.rs`
  - Result-producing operation tracking exists and must include server
    operations.
- `src/compiler/lowering.rs`
  - Runtime function imports, heap/resource type lowering, drop handling, and
    pointer-sized resource representation already support JSON and TCP.
- `runtime/duumbi_runtime.c`
  - JSON parsing/stringifying and TCP socket/listener resources exist.
  - TCP runtime code already has loopback-friendly socket behavior to extend or
    mirror for HTTP server v1.
- `runtime/duumbi_runtime.h`
  - JSON and TCP runtime declarations exist and must be extended for server
    declarations.
- `src/registry/package.rs`
  - Packaging already collects sorted `.duumbi/graph/*.jsonld` files and emits
    `manifest.toml`, `graph/*.jsonld`, and `CHECKSUM`.
- `src/cli/publish.rs`
  - Publish currently validates only `.duumbi/graph/main.jsonld` before
    packaging. That is too narrow for stdlib module packages because packaging
    itself supports multiple named graph files.
- `src/cli/init.rs`
  - Default dependencies remain math, io, lang, and string.
  - JSON and net modules are cached but not default dependencies.
- `tests/kill_criterion_phase7.rs`
  - Provides an embedded registry-server test pattern with in-memory database,
    loopback listener, token, and registry client.
- `tests/integration_duumbi379.rs`
  - Provides dynamic fixture and loopback TCP test patterns with timeouts.

Verified source facts:

- There is no current `http_server` type string.
- There are no current server built-in operations or runtime symbols.
- There is no production-concurrency or callback/event-loop model for dynamic
  request handlers.
- The existing runtime has no byte-buffer type; server request-body parsing is
  out of scope for v1.
- Default CI must not depend on public internet access or production registry
  credentials.

Assumptions for implementation:

- A single-threaded blocking C runtime loop with explicit timeout and request
  limit is sufficient for server v1.
- Server tests can use loopback clients and harness-selected free ports.
- Production publishing to `registry.duumbi.dev` is a gated release operation
  requiring credentials and human authorization, not a default CI behavior.
- Embedded-registry tests are the authoritative automated substitute for
  production registry access in CI.

## Affected Areas

Expected Stage 10 implementation changes:

- Core type model:
  - `src/types.rs`
- JSON-LD parser:
  - `src/parser/mod.rs`
  - optional parser helper for `duumbi:args` reuse by non-call operations
- Graph construction, validation, and result-safety:
  - `src/graph/builder.rs` only if existing indexed argument edges need server
    operation-specific adjustment
  - `src/graph/validator.rs`
  - `src/graph/result_safety.rs`
- Compiler lowering:
  - `src/compiler/lowering.rs`
  - `src/compiler/linker.rs` only if server socket linkage needs platform
    changes beyond the existing TCP runtime path
- C runtime:
  - `runtime/duumbi_runtime.c`
  - `runtime/duumbi_runtime.h`
- Stdlib graph modules and manifests:
  - new `stdlib/server.jsonld`
  - new `stdlib/server.manifest.toml` or the local manifest source used by the
    existing stdlib publication flow
- Workspace initialization and stdlib cache:
  - `src/cli/init.rs`, only to cache `@duumbi/stdlib-server` as an explicit
    installable module if implementation follows the JSON/net cache pattern
  - default dependencies must not include server
- Registry publish validation:
  - `src/cli/publish.rs`
  - focused publish/package tests
- Tests and fixtures:
  - parser/type tests for `http_server` and nested result forms
  - server lowering/runtime integration tests
  - embedded-registry publish/search/install tests
  - default init dependency-policy tests
  - publish-matrix evidence tests or a deterministic evidence helper

Areas that must not change during Stage 10 unless a later approved spec expands
scope:

- `specs/DUUMBI-381/PRODUCT.md`
- product specs for #380, #382, or unrelated issues
- implementation of HTTP client, TLS, database, WebSocket, middleware, dynamic
  callbacks, async event loops, or production daemon supervision
- default workspace dependency policy
- production registry server code, unless publish/install evidence proves a
  missing server-side compatibility issue and a human approves scope expansion
- provider/model setup flows
- REPL/TUI mutation behavior
- Studio UI behavior
- generated artifacts, committed binaries, credentials, or public-internet
  dependent tests

## Technical Approach

### Publish Matrix

Add a deterministic publish matrix step to the implementation evidence. The
matrix may be produced by tests, a release helper, or the implementation PR
body, but it must be source-backed and reviewable.

Initial matrix states:

| Module | Required #381 state before production publish |
|---|---|
| `@duumbi/stdlib-math` | `publishable-after-verify` |
| `@duumbi/stdlib-io` | `publishable-after-verify` |
| `@duumbi/stdlib-lang` | `publishable-after-verify` |
| `@duumbi/stdlib-string` | `publishable-after-verify` |
| `@duumbi/stdlib-file` | `publishable-after-verify` when source manifest, graph, dry-run, and import/build/run evidence pass |
| `@duumbi/stdlib-json` | `publishable-after-verify` when source manifest, graph, dry-run, and import/build/run evidence pass |
| `@duumbi/stdlib-net` | `publishable-after-verify` when source manifest, graph, dry-run, and import/build/run evidence pass |
| `@duumbi/stdlib-server` | `publishable-after-verify` only after #381 server implementation and review evidence pass |
| `@duumbi/stdlib-http` | `deferred-upstream` until #380 technical spec, implementation, and review gates complete |
| `@duumbi/stdlib-tls` if separate | `deferred-upstream` until #380 explicitly approves a separate module |
| `@duumbi/stdlib-db` | `deferred-upstream` until #380 technical spec, implementation, and review gates complete |

Matrix evidence must include:

- module name and version;
- source graph path or deferral reason;
- upstream issue/gate dependency when deferred;
- dry-run package status;
- production publish status or gated reason;
- registry target;
- archive integrity when packaging succeeds;
- search/install/import/build/run status when a module is published.

The implementation must not claim future Tier 1 ecosystem readiness for modules
that remain `deferred-upstream`.

### Publish Validation

Tighten `duumbi publish` validation so it matches package behavior:

- Discover every `.duumbi/graph/*.jsonld` file.
- Fail visibly if no `.jsonld` graph file exists.
- Validate every discovered graph file before packaging.
- Stop requiring `.duumbi/graph/main.jsonld` specifically for module publish.
- Preserve clear error context that names the failing graph file.
- Preserve current manifest validation, archive creation, integrity display,
  credential checks, and dry-run behavior.

This change keeps workspace packages compatible with stdlib modules whose graph
files are named after the module instead of `main.jsonld`.

### Embedded Registry Verification

Use the existing embedded-registry test pattern instead of production registry
access for automated verification:

- Start an in-memory `duumbi-registry` server on `127.0.0.1:0`.
- Create a test user and token through the registry database layer.
- Configure a clean DUUMBI workspace with the embedded registry URL.
- Publish a selected package through the same client/CLI path used by
  production publish.
- Search for the package.
- Install it into a separate clean workspace.
- Verify cache contents, manifest, graph files, and integrity metadata.
- Build and run a minimal program that imports one export from the installed
  module.

Production publishing remains outside default CI:

- The implementation PR must show dry-run and embedded-registry evidence.
- A non-dry-run publish to `registry.duumbi.dev` requires human approval,
  configured credentials, and a token-safe evidence log.
- Missing credentials must produce the existing `E014`-style failure before
  upload.

### Server Type And Parser Model

Add first-class DUUMBI type:

- `DuumbiType::HttpServer`

Parser type string:

- `http_server`
- nested result form such as `result<http_server,string>`

Type behavior:

- `fmt::Display` must print `http_server`.
- `is_heap_type()` must return true for `HttpServer`.
- `duumbi_type_to_cl()` must map `HttpServer` to pointer-sized `types::I64`.
- `type_size()` must treat `HttpServer` as pointer-sized.
- Drop/free handling must release server resources without double free.

Add built-in operations:

- `Op::ServerNew`
- `Op::RouteAddStatic`
- `Op::ServerStart`
- `Op::ServerClose`

JSON-LD operation names:

- `duumbi:ServerNew`
- `duumbi:RouteAddStatic`
- `duumbi:ServerStart`
- `duumbi:ServerClose`

Operation field mapping:

- `ServerNew`
  - `duumbi:operand`: host string node
  - `duumbi:left`: port i64 node
  - `duumbi:right`: setup timeout_ms i64 node
  - `duumbi:resultType`: `result<http_server,string>`
- `RouteAddStatic`
  - `duumbi:operand`: server node
  - `duumbi:args`: exactly five node refs in this order:
    method string, path string, status i64, headers json, body string
  - `duumbi:resultType`: `result<i64,string>`
- `ServerStart`
  - `duumbi:operand`: server node
  - `duumbi:left`: max_requests i64 node
  - `duumbi:right`: timeout_ms i64 node
  - `duumbi:resultType`: `result<i64,string>`
- `ServerClose`
  - `duumbi:operand`: server node
  - `duumbi:resultType`: `result<i64,string>`

Parser implementation must reuse or extract the existing `duumbi:args` parsing
logic so non-call operations can parse argument arrays without duplicating
fragile JSON field handling.

### Server Validation

Extend graph validation so the static types above are enforced:

- `ServerNew` requires `string`, `i64`, `i64` inputs and returns
  `result<http_server,string>`.
- `RouteAddStatic` requires `http_server`, `string`, `string`, `i64`, `json`,
  and `string` inputs and returns `result<i64,string>`.
- `ServerStart` requires `http_server`, `i64`, and `i64` inputs and returns
  `result<i64,string>`.
- `ServerClose` requires `http_server` input and returns `result<i64,string>`.
- Missing operands or wrong `duumbi:args` arity must be parse or validation
  errors, not lowering panics.
- All server operations that return `result<_,string>` must be included in
  result-safety analysis.

### Server Runtime

Add runtime symbols with stable C signatures equivalent to:

- `int64_t duumbi_server_new(void *host, int64_t port, int64_t timeout_ms)`
- `int64_t duumbi_route_add_static(void *server, void *method, void *path,
  int64_t status, void *headers, void *body)`
- `int64_t duumbi_server_start(void *server, int64_t max_requests,
  int64_t timeout_ms)`
- `int64_t duumbi_server_close(void *server)`
- `void duumbi_server_free(void *server)`

The `int64_t` return values are runtime result pointers, matching the existing
lowering convention for result-returning runtime calls.

Runtime resource model:

- Introduce an opaque `DuumbiHttpServer` structure.
- `server_new` binds and listens on the requested host/port during creation so
  bind failures surface before routes are registered.
- Only loopback hosts are required for tests and examples. Implementation may
  support more host strings, but tests must not depend on non-loopback network
  access.
- Store a finite in-memory static route table owned by the server resource.
- Route table mutation is allowed before `server_start`.
- Route mutation after `server_start` returns `Err(string)`.
- Closed resources reject later operations with `Err(string)`.
- `server_free` must be safe on null, already-closed, and partially-created
  resources.

Runtime HTTP behavior:

- Accept one TCP connection at a time.
- Read one HTTP/1.1 request line and bounded headers.
- Ignore request bodies in v1.
- Match exact method and exact path.
- Return the configured status, headers, and body for matched routes.
- Return a deterministic `404 Not Found` response with body `not found` for
  unmatched routes. This still counts as one handled request.
- Always include a correct `Content-Length` header for runtime-generated
  responses.
- Close the connection after each response.
- Do not implement persistent connections, chunked encoding, TLS, middleware,
  request callbacks, or streaming.

Timeout and request-limit behavior:

- `timeout_ms` must be positive for `server_new` and `server_start`.
- `max_requests` must be positive for `server_start`.
- If `server_start` handles `max_requests` requests, it returns `Ok(n)`.
- If `server_start` times out before any request is handled, it returns
  `Err(string)`.
- If `server_start` handles at least one request and then times out before
  `max_requests`, it may return `Ok(n)` where `n` is the handled count, but the
  implementation must document and test the chosen behavior.
- No server test may rely on an unbounded blocking call.

Route validation:

- Method must be non-empty ASCII token text. The implementation must accept at
  least `GET`.
- Path must start with `/`.
- Status must be in the inclusive HTTP range `100..=599`.
- Headers must be a JSON object whose keys and values can be serialized as HTTP
  header strings. Invalid header JSON returns `Err(string)`.
- Duplicate method/path route registration must return a stable `Err(string)`
  unless implementation explicitly replaces routes and tests that replacement.
  Prefer returning `Err(string)` for v1.

Close behavior:

- `server_close` releases the listening socket and route table.
- The first successful close returns `Ok(0)`.
- Repeated close returns `Ok(0)` and must not double free.
- Later route or start operations on a closed server return `Err(string)`.

### Server Stdlib Module

Add `@duumbi/stdlib-server` with version `1.0.0`.

The graph module must export:

- `server_new`
- `route_add_static`
- `server_start`
- `server_close`

The module manifest must include:

- name: `@duumbi/stdlib-server`
- version: `1.0.0`
- description
- license
- complete export list

The graph wrappers must expose the approved product signatures:

- `server_new(host: string, port: i64, timeout_ms: i64)
  -> result<http_server, string>`
- `route_add_static(server: http_server, method: string, path: string,
  status: i64, headers: json, body: string) -> result<i64, string>`
- `server_start(server: http_server, max_requests: i64, timeout_ms: i64)
  -> result<i64, string>`
- `server_close(server: http_server) -> result<i64, string>`

`@duumbi/stdlib-server` may be written into the init cache for explicit
dependency use if the implementation follows the JSON/net cache pattern. It
must not be added to default dependencies.

## Invariants

- Spec-only PRs must use non-closing issue references and must leave #381 open.
- Stage 10 implementation must not publish a module as `1.0.0` without accepted
  API, manifest, graph, package, dry-run, and verification evidence.
- Production publish must not print or persist credentials.
- Default CI must pass without `registry.duumbi.dev` access.
- All network tests must use loopback and explicit timeouts.
- Server resources must be opaque at the DUUMBI API boundary.
- Server failures must return `result<_,string>` instead of crashing or hanging.
- Graph validation failures must occur before lowering wherever input types or
  operation arity are invalid.
- Default `duumbi init` dependencies remain math, io, lang, and string.
- #380 modules remain deferred until #380 completes its own technical,
  implementation, and review gates.
- #382 receives handoff evidence but #381 must not expand into the full #382
  release smoke matrix.

## BDD-To-Test Mapping

| Product scenario | Required automated or reviewed evidence |
|---|---|
| Ready core module passes dry-run package verification | Test or release helper creates a publishable workspace for `@duumbi/stdlib-math`, runs the dry-run publish path, and asserts archive contents include `manifest.toml`, `graph/*.jsonld`, `CHECKSUM`, and integrity. |
| Module with incomplete upstream gates is deferred | Publish-matrix evidence marks `@duumbi/stdlib-http`, optional `@duumbi/stdlib-tls`, and `@duumbi/stdlib-db` as `deferred-upstream` with #380 as the dependency. Review evidence is acceptable because this depends on GitHub workflow state. |
| Production publish is blocked without credentials | CLI or integration test runs non-dry-run publish against a configured authenticated registry name with no credential and asserts an `E014`-style missing-authentication error before upload. |
| Published module is discoverable | Embedded-registry integration test publishes `@duumbi/stdlib-string@1.0.0`, runs search through the registry client or CLI path, and asserts the module/version metadata is returned. |
| Published module installs into a clean workspace | Embedded-registry integration test installs a published package with `duumbi deps add`, then asserts `.duumbi/cache`, `config.toml`, manifest, graph files, and integrity metadata. |
| Installed module can be imported, built, and run | Clean-workspace integration test imports at least one installed module export, builds the program, runs it, and checks output or exit code. Cover at least one core module and one runtime-backed module when available. |
| Static route server returns a configured response | Loopback integration test compiles/runs a DUUMBI fixture that creates a server, registers `GET /health`, starts with `max_requests = 1`, sends a local HTTP request, and asserts status, headers, body, and `Ok(1)`. |
| Server start stops after the request limit | Same loopback fixture or a focused second fixture asserts the process exits after one handled request and leaves no long-running child. |
| Server start times out without hanging | Loopback/no-client integration test starts the server with short timeout and asserts a documented timeout result and bounded process exit. |
| Invalid route data is reported as an error | Runtime or compiled-fixture test calls `route_add_static` with invalid method or path and asserts `Err(string)` with no later exposed route. |
| Closed server rejects later operations | Runtime or compiled-fixture test closes a server and then asserts `route_add_static` and `server_start` return `Err(string)` without crash or double free. |
| New workspaces keep the core default dependency set | Init/config test asserts default dependencies include only math, io, lang, and string; server, JSON, TCP, HTTP, DB, and TLS are absent unless later product approval changes policy. |
| Downstream smoke-test issue receives a concrete matrix | Stage 10 PR body or issue comment lists final module matrix, registry target, package integrity, publish status, minimal verification result, and #382 handoff. |

Minimum new automated test coverage:

- `http_server` type parse/display/heap behavior.
- Server operation parser coverage, including `RouteAddStatic` `duumbi:args`
  arity.
- Validation errors for wrong server operation types and missing operands.
- Result-safety tracking for server operations.
- Lowering import and runtime call coverage for server operations.
- Runtime loopback success, timeout, invalid route, and closed-resource cases.
- Publish validation over all graph files, including non-`main.jsonld` graph
  names.
- Embedded-registry publish/search/install/build/run coverage.
- Default init dependency-policy coverage.

## Live E2E Plan

Default live E2E for Stage 10 must use local resources only:

1. Build the CLI with `cargo build`.
2. Run focused tests for publish validation and embedded-registry behavior.
3. Run focused tests for server loopback behavior.
4. Create a temporary clean workspace with `duumbi init`.
5. Verify default dependencies remain math, io, lang, and string.
6. Configure the temporary workspace to use an embedded or local test registry.
7. Publish at least one ready stdlib module to that registry.
8. Run `duumbi search stdlib`.
9. Run `duumbi deps add @duumbi/stdlib-name@1.0.0`.
10. Build and run a minimal importer program from the clean workspace.
11. Run a compiled server fixture that returns `200` for `GET /health` and
    exits after `max_requests = 1`.

Expected external LLM calls: 0.

Expected public internet calls in default CI: 0.

Production publish E2E is gated:

- Target: `registry.duumbi.dev`, unless later approved otherwise.
- Required before running: human approval, configured credentials, confirmed
  target URL, and token-safe logging.
- Stop condition: if credentials or permissions are unavailable, record dry-run
  and embedded-registry evidence and request human approval instead of
  attempting workarounds.

## Ralph Cycle Resource Policy

Stage 10 agents must use bounded Ralph cycles. This spec does not authorize
Ralph cycles during Stage 8 or Stage 9.

Autonomous cycle budget:

- Maximum autonomous cycles before human review: 3.
- Maximum changed implementation/test files per cycle: 10, excluding this
  technical spec and PR metadata.
- Expected LLM/API cost per cycle: USD 0 for implementation verification.
- Expected external services per cycle: none in default CI.
- Expected network access per cycle: loopback only.

Cycle sequencing:

1. Publish validation and matrix evidence.
2. Server type/parser/validator/lowering/runtime implementation.
3. Embedded-registry, clean-workspace, and server E2E verification.

Agents must stop and request human approval before:

- non-dry-run production publishing;
- using or changing credentials;
- introducing public-internet-dependent tests;
- changing registry server behavior;
- changing default workspace dependency policy;
- implementing #380 HTTP/TLS/DB modules;
- implementing #382 broad release smoke coverage;
- adding dynamic server callbacks, middleware, async event loops, TLS,
  WebSockets, or daemon supervision;
- adding a new third-party C dependency or system package requirement;
- exceeding USD 2 estimated external API spend;
- exceeding the autonomous cycle count;
- requiring destructive git or filesystem operations.

## Task Breakdown

1. Reconfirm Stage 9 approval of this technical spec before implementation.
2. Add publish validation that checks every graph file instead of only
   `main.jsonld`.
3. Add tests for publish validation and missing credentials.
4. Add or normalize publishable stdlib module workspaces/manifests for the ready
   module set.
5. Add deterministic publish-matrix evidence and mark #380-owned modules
   deferred.
6. Add `HttpServer` type support.
7. Add parser support for server operations and reusable `duumbi:args` parsing.
8. Add graph validation and result-safety coverage for server operations.
9. Add lowering and runtime symbol declarations for server operations.
10. Implement the C `DuumbiHttpServer` resource and bounded HTTP handling.
11. Add `@duumbi/stdlib-server` graph and manifest.
12. Optionally cache `@duumbi/stdlib-server` in `duumbi init` without adding it
    to default dependencies.
13. Add loopback server integration tests.
14. Add embedded-registry publish/search/install/import/build/run tests.
15. Record production publish gate status, dry-run evidence, and #382 handoff
    evidence in the implementation PR and issue.

## Verification Plan

Required local checks for Stage 10 implementation:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`
- Focused publish validation test command selected by the implementation agent.
- Focused embedded-registry integration test command selected by the
  implementation agent.
- Focused server loopback integration test command selected by the
  implementation agent.

Manual or gated checks:

- Non-dry-run `duumbi publish` to `registry.duumbi.dev` only after human
  approval and credential verification.
- Production search/install/build/run evidence only after production publish
  succeeds.
- #382 handoff comment after final matrix evidence is available.

Evidence requirements:

- Test output or PR evidence for every BDD scenario.
- Archive integrity values for packaged modules.
- Registry target and publish status for each module.
- Deferral reason and upstream issue for each deferred module.
- Clean-workspace import/build/run result for every published module.
- Token-safe logs for any credential-dependent action.

## Completion Criteria

Stage 10 implementation is complete only when:

- Stage 9 has approved this technical spec.
- The implementation PR includes only changes allowed by this spec or
  separately approved scope.
- Ready modules have dry-run package evidence.
- Production publish either succeeds with approved credentials or is explicitly
  stopped at the human approval gate with complete dry-run and embedded-registry
  evidence.
- `@duumbi/stdlib-server` v1 implements the approved API.
- All server operations are parsed, validated, lowered, and runtime-backed.
- Server resources clean up safely and closed-resource behavior is tested.
- Search, install, import, build, and run verification exists for published
  modules through embedded-registry automation and production evidence when
  authorized.
- Default `duumbi init` dependencies remain unchanged.
- #380-owned modules are not published unless their gates complete.
- #382 receives the concrete matrix and evidence contract.
- Required CI and review gates are clean.

## Failure And Escalation

Escalate to a human instead of continuing autonomously when:

- Product or technical scope conflicts with #380 or #382.
- Production registry credentials, permissions, or target URL are missing.
- Production publish would overwrite or conflict with an existing `1.0.0`
  release.
- Registry server compatibility requires server-side changes.
- Server runtime implementation requires a new platform dependency that is not
  already available in CI.
- Loopback server tests are flaky after bounded retry/timeout adjustments.
- A reviewer identifies a blocking API, security, resource-lifetime, or release
  integrity concern.
- The implementation cannot prove a BDD scenario with automated evidence or an
  approved manual gate.

## Open Questions

No blocking open questions remain for Stage 10 implementation.

The following items are explicit gates, not unresolved requirements:

- Production publishing requires human approval and credentials at
  implementation time.
- #380 HTTP/TLS/DB modules remain deferred until #380 completes its own gates.
- #382 broad release validation consumes #381 evidence after #382 reaches the
  required workflow state.
