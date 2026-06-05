# DUUMBI-380: @duumbi/stdlib-http, @duumbi/stdlib-tls, And @duumbi/stdlib-db

## Summary

Add the next Tier 1 integration standard-library layer so DUUMBI graph programs
can call HTTP APIs, use verified HTTPS by default, and persist small local data
sets through SQLite without host-side glue code.

The accepted v1 boundary is:

- `@duumbi/stdlib-http` exposes timeout-bounded HTTP client functions for GET,
  POST, PUT, DELETE, response status, response body, response headers, and
  response cleanup.
- TLS is included as verified HTTPS behavior for HTTP requests. Certificate
  verification is on by default. Raw TLS socket APIs are not exposed in v1.
- `@duumbi/stdlib-db` exposes minimal local SQLite functions for opening a
  database, executing parameterized statements, querying rows, reading string
  values, and explicit resource cleanup.
- HTTP headers use the first-class `json` type from #379.
- Database query results use an opaque `db_rows` resource with `db_rows_len`,
  `db_row_get`, and cleanup semantics.
- The accepted end-to-end demo shape is API fetch -> JSON parse -> DUUMBI
  transform -> SQLite write/query, using local or loopback fixtures in CI.

Related to #380. This is a Stage 6 product specification only. The execution
issue must remain open for Stage 7 review, Stage 8 technical specification,
implementation, implementation review, and Stage 12 closure.

## Problem

DUUMBI now has practical local stdlib foundations and, on `main`, first-class
JSON and TCP resource support. That makes structured data and socket-level
communication possible, but it still leaves common integration demos awkward:
users cannot make a plain HTTP request, cannot rely on default HTTPS
certificate verification, and cannot persist local state in a SQLite database
from DUUMBI graph programs.

Without this work:

- API-fetch examples need host scripts or hand-written TCP protocol handling.
- HTTPS safety remains implicit or unavailable instead of being a product
  contract.
- SQLite examples need host-side database code even though local persistence is
  a natural next step after file, JSON, and TCP modules.
- Phase 14 ecosystem demos stay too toy-like for developers evaluating whether
  DUUMBI can build realistic integration workflows.
- Later Tier 1 publishing in #381 has no reviewable contract for HTTP, TLS, and
  DB module behavior.

The product requirement is not a full networking, security, or database
platform. It is a small, auditable, testable integration layer that is useful
for demos and early users while preserving DUUMBI's graph-first runtime model
and review gates.

## Outcome

When this is done:

- A user can add `@duumbi/stdlib-http` and call `http_get`, `http_post`,
  `http_put`, or `http_delete` with explicit timeout values.
- Successful HTTP transport returns an opaque `http_response` resource that
  lets the graph inspect status code, body text, and headers.
- Non-2xx HTTP status codes are visible through `http_status` and do not turn a
  transport-successful response into an `Err(string)`.
- DNS, connection, timeout, TLS, malformed URL, request-header, allocation, and
  response-body representation failures return visible `Err(string)` values.
- HTTPS requests verify certificates by default.
- Certificate verification failures return visible `Err(string)` values and
  are not silently ignored.
- No safe/default v1 API exposes certificate verification bypass.
- No v1 API exposes raw `tls_socket`, `tls_connect`, `tls_wrap`, `tls_read`, or
  `tls_write` behavior.
- A user can add `@duumbi/stdlib-db` and open a local SQLite database, execute
  parameterized SQL statements, query rows, read row values as strings, and
  close/free resources explicitly.
- SQLite statement errors, parameter errors, path-policy failures, query errors,
  closed-resource use, and nonrepresentable row values return visible
  `Err(string)` values.
- HTTP, HTTPS, and DB tests are local, loopback, temporary, and timeout-bounded;
  default CI does not depend on public internet services.
- Module manifests and documentation describe the accepted v1 functions,
  resource ownership, failure behavior, and intentionally deferred features.
- `duumbi init` does not add these modules to default new workspaces unless #381
  or a later accepted product spec changes the Tier 1 distribution policy.

## Scope

### In Scope

- Add a durable product contract for `@duumbi/stdlib-http`.
- Define the accepted v1 HTTP API:
  - `http_get(url: string, headers: json, timeout_ms: i64) -> result<http_response, string>`
  - `http_post(url: string, headers: json, body: string, timeout_ms: i64) -> result<http_response, string>`
  - `http_put(url: string, headers: json, body: string, timeout_ms: i64) -> result<http_response, string>`
  - `http_delete(url: string, headers: json, timeout_ms: i64) -> result<http_response, string>`
  - `http_status(response: http_response) -> result<i64, string>`
  - `http_body(response: http_response) -> result<string, string>`
  - `http_headers(response: http_response) -> result<json, string>`
  - `http_response_free(response: http_response) -> result<i64, string>`
- Define `http_response` as a first-class opaque runtime-backed resource type.
  Users must not see or manipulate raw pointers, descriptors, or integer handles
  as the HTTP response API.
- Require explicit `timeout_ms` parameters for all outbound HTTP calls.
- Use `json` request headers from #379 as a JSON object whose keys are header
  names and whose values are strings.
- Return response headers as a JSON object whose values are strings. Header name
  case and duplicate-header handling must be deterministic and documented by
  the implementation.
- Treat HTTP response bodies as DUUMBI strings in v1. Bodies that cannot be
  safely represented as DUUMBI strings return `Err(string)`.
- Define the accepted v1 TLS behavior:
  - HTTPS requests through `@duumbi/stdlib-http` verify certificates by default.
  - TLS/certificate failures return visible `Err(string)` values.
  - TLS support may be packaged as an internal dependency or separate module
    artifact, but v1 has no raw public TLS socket API.
- Define the accepted v1 SQLite API:
  - `db_open(path: string) -> result<db_connection, string>`
  - `db_execute(conn: db_connection, sql: string, params: array<string>) -> result<i64, string>`
  - `db_query(conn: db_connection, sql: string, params: array<string>) -> result<db_rows, string>`
  - `db_rows_len(rows: db_rows) -> result<i64, string>`
  - `db_row_get(rows: db_rows, row_index: i64, column: string) -> result<string, string>`
  - `db_close(conn: db_connection) -> result<i64, string>`
  - `db_rows_free(rows: db_rows) -> result<i64, string>`
- Define `db_connection` and `db_rows` as first-class opaque runtime-backed
  resource types. Users must not see or manipulate raw SQLite pointers or
  integer handles as the DB API.
- Require `db_execute` and `db_query` to use prepared statements for parameter
  binding rather than string interpolation by the stdlib.
- Accept string-only row extraction in v1. Typed extraction, nullable helpers,
  and JSON row projection are deferred.
- Require local-only database behavior. File-backed database paths are
  workspace-confined using the #378 path policy unless Stage 7 product review
  changes that decision. The special SQLite in-memory database name `:memory:`
  is accepted as a local non-file database.
- Add source-level module artifacts, manifests/export lists, and documentation
  in the repository path accepted by the later technical spec.
- Update parser, type, lowering, runtime, cache/module, and linker surfaces only
  after Stage 7 and Stage 8 approval, and only as required to satisfy this
  product contract.
- Document the cross-platform dependency strategy for the chosen HTTP/TLS/SQLite
  implementation, including any vendored or system library requirements.
- Verify behavior with local fixtures: loopback HTTP, local HTTPS/TLS evidence,
  and temporary SQLite databases.
- Keep the spec PR non-closing and limited to this product spec file.

### Explicitly Out Of Scope

- Technical specification creation during Stage 6.
- Implementation code, source tests, runtime changes, manifests, or Ralph cycles
  during Stage 6.
- Raw TLS socket APIs in v1, including `tls_connect`, `tls_wrap`, `tls_read`,
  `tls_write`, and `tls_close`.
- Insecure certificate bypass in safe/default APIs.
- HTTP redirects as automatic client behavior in v1. Redirect responses are
  returned as ordinary HTTP responses with status and headers visible.
- Browser-grade HTTP behavior such as cookies, caches, proxies, multipart forms,
  streaming request/response bodies, authentication frameworks, HTTP/2 tuning,
  WebSockets, and compression controls.
- URL parser APIs separate from HTTP request functions.
- Binary response/body APIs or arbitrary byte buffers.
- Remote database protocols such as Postgres, MySQL, SQL Server, Redis, or cloud
  database clients.
- SQLite connection pools, async database APIs, migrations, schema management
  helpers, ORM behavior, transaction helper APIs beyond ordinary SQL statements,
  prepared statement handles, blob APIs, and backup/replication behavior.
- SQL generation, SQL linting, SQL safety analysis, or query-builder semantics.
- Internet-dependent tests, public API endpoints in CI, production database
  services, or tests requiring credentials.
- Adding these modules to default `duumbi init` workspaces before #381 or a
  later accepted product spec changes distribution policy.
- Publishing these modules to `registry.duumbi.dev`; #381 owns Tier 1
  publication.
- E2E coverage for all Tier 1 stdlib modules; #382 owns broad installed-module
  smoke tests.

## Constraints And Assumptions

Facts:

- Issue #380 is open and titled `feat(stdlib): @duumbi/stdlib-http +
  @duumbi/stdlib-tls + @duumbi/stdlib-db`.
- Issue #380 has labels `phase-14`, `module:ecosystem`, `accepted`, and
  `needs-spec`.
- Stage 4 routed #380 to `Needs Human Acceptance` as a concrete ecosystem
  feature with clear dependencies.
- Stage 5 accepted #380 on 2026-06-05 from Slack reviewer source `hga`, with no
  remaining open questions and next state `Spec Needed`.
- The Stage 5 pre-build decision comment narrows v1 to HTTP client + verified
  HTTPS behavior + minimal local SQLite support.
- The Stage 5 pre-build decision comment explicitly excludes raw TLS socket APIs
  from v1 unless a later concrete accepted demo requires them.
- The Stage 5 pre-build decision comment selects first-class `json` headers from
  #379.
- The Stage 5 pre-build decision comment selects opaque `db_rows` for DB query
  results.
- The Stage 5 pre-build decision comment keeps these modules registry-add-only
  until #381 publishes Tier 1 stdlib cleanly.
- The Stage 5 pre-build decision comment selects the minimum accepted demo:
  API fetch -> JSON parse -> DUUMBI transform -> SQLite write/query, using
  local/loopback CI fixtures.
- The current source repo has `stdlib/file.jsonld`, `stdlib/json.jsonld`, and
  `stdlib/net.jsonld` on `main`, in addition to the existing math, io, lang, and
  string modules.
- The current `DuumbiType` includes `json`, `tcp_socket`, and `tcp_listener`.
  It does not include `http_response`, `db_connection`, `db_rows`, or
  `tls_socket`.
- The current parser recognizes type strings for `json`, `tcp_socket`, and
  `tcp_listener`, but not the #380 resource types.
- The current compiler lowering treats heap/resource values and result/option
  values as pointer-sized opaque runtime values at the Cranelift boundary.
- The current runtime has JSON and TCP shims. It does not have HTTP, TLS, or
  SQLite shims.
- The current linker has `-lm` on Unix/macOS and adds `-lws2_32` on Windows for
  TCP. It does not link curl, TLS, SQLite, or another HTTP/TLS/DB library.
- `Cargo.toml` uses Rust `reqwest` for host-side CLI/registry/agent behavior,
  but that does not automatically expose HTTP to compiled DUUMBI graph programs.
- Related #379 defines JSON and TCP foundations that #380 should build on.
- Related #378 defines workspace-confined file/path behavior that local SQLite
  file paths should align with.
- Related #381 owns Tier 1 stdlib publishing and default distribution policy.
- Related #382 owns clean-workspace installed-module smoke coverage.
- The active PRD says useful demos must be reliable before Phase 14 marketing,
  and that intent, behavior, evidence, and review gates must stay connected.
- DUUMBI review policy requires Codex self-review plus Copilot review evidence
  for file-based Stage 6 product spec PRs. Greptile is manual-only and is not
  part of the default spec gate.

Assumptions:

- This spec may rely on #379 JSON/TCP behavior as current source context because
  those modules are present on `main`.
- Implementation can introduce new graph-level resource types when the approved
  technical spec shows the parser, type, lowering, runtime, and ownership
  changes needed.
- HTTP v1 can treat URLs as strings validated by the accepted client/runtime
  implementation rather than adding a separate URL type.
- Header values as strings are sufficient for v1 request and response headers,
  even though later APIs may need richer multi-value header support.
- HTTP response bodies as DUUMBI strings are sufficient for v1 demos. Binary
  payloads can be represented later by a byte-buffer feature when accepted.
- `db_execute` returning changed-row count, with `0` for DDL or statements where
  SQLite reports no changed rows, is sufficient for v1.
- `db_row_get` returning strings is sufficient for v1. Stage 8 may define exact
  SQLite-to-string conversion details as long as behavior is deterministic and
  documented.
- A local HTTPS verification test can be implemented without exposing an
  insecure public bypass API, for example through a test-only trust fixture or
  other technical-spec-approved harness.
- Reproducible vendored/static dependencies are preferred where practical, but
  Stage 8 may choose system libraries when it records platform requirements and
  packaging/linking consequences.

Constraints:

- Stage 6 must not create a technical spec or implementation code.
- The spec-only PR must use non-closing issue references such as `Related to
  #380` or `Spec for #380`.
- The execution issue must remain open through later DUUMBI workflow stages.
- HTTP, TLS, and DB recoverable failures must use `result<_, string>`.
- No accepted #380 API may expose raw user-visible integer handles for HTTP
  responses, database connections, row sets, or TLS internals.
- All outbound HTTP calls must require explicit timeout parameters.
- HTTP request execution must not happen during import, workspace initialization,
  graph parsing, type checking, compilation, or manifest inspection.
- HTTPS certificate verification must be the safe default.
- Local DB file behavior must not silently grant arbitrary filesystem access.
- Tests must not rely on public internet availability, third-party services,
  credentials, production databases, or shared local state.
- Runtime errors must not log credentials, authorization headers, request bodies,
  response bodies, SQL parameters, SQL result values, or database contents unless
  a later accepted spec defines safe diagnostic behavior.

## Decisions

- **Decision:** Use a file-based product spec for #380.
  **Evidence:** The accepted work is user-visible, cross-module, runtime-facing,
  security-sensitive, and useful as durable context for Stage 8 and later
  implementation review.

- **Decision:** V1 includes HTTP client behavior, verified HTTPS behavior, and
  minimal local SQLite behavior.
  **Evidence:** The Stage 5 pre-build decision comment explicitly accepted this
  boundary.

- **Decision:** Raw TLS socket APIs are deferred from v1.
  **Evidence:** The Stage 5 pre-build decision says not to expose raw TLS socket
  APIs unless a concrete accepted demo requires them.

- **Decision:** `@duumbi/stdlib-http` uses first-class `json` values for request
  and response headers.
  **Evidence:** The Stage 5 pre-build decision selected the #379 `json` type for
  headers.

- **Decision:** `http_response` is a first-class opaque resource type.
  **Evidence:** The issue requires response status/body/headers with cleanup
  semantics where needed, and existing #379 TCP/JSON resource conventions reject
  user-visible raw handles.

- **Decision:** `http_status` returns `result<i64, string>` rather than a bare
  `i64`.
  **Evidence:** Released or otherwise invalid response resources must fail
  visibly. A result-returning status accessor preserves that lifecycle contract
  without requiring a panic or sentinel status value.

- **Decision:** HTTP non-2xx statuses are response states, not transport errors.
  **Evidence:** The user outcome includes inspecting status codes. Treating
  non-2xx as `Ok(http_response)` preserves status/body/header inspection.

- **Decision:** Redirect responses are returned as ordinary responses in v1.
  **Evidence:** Redirect policy is part of broader browser-grade behavior, and
  v1 should keep request behavior auditable instead of hiding a follow chain.

- **Decision:** HTTP bodies are text-only DUUMBI strings in v1.
  **Evidence:** Existing DUUMBI graph APIs expose `string` but no accepted byte
  buffer type. Nonrepresentable payloads should be visible errors, not corrupted
  strings.

- **Decision:** `@duumbi/stdlib-db` uses opaque `db_connection` and `db_rows`
  resources.
  **Evidence:** The Stage 5 pre-build decision selects `db_rows`, and current
  DUUMBI resource conventions use opaque graph-level resource types for handles.

- **Decision:** DB parameters are `array<string>` and must be bound through
  prepared statements.
  **Evidence:** The issue explicitly asks for parameterized execution/query.
  String-only parameters keep v1 small while preventing stdlib-level SQL
  interpolation.

- **Decision:** `db_row_get` returns string values only in v1.
  **Evidence:** The issue accepts string-only row access if typed extraction is
  deferred and documented.

- **Decision:** SQL `NULL` values are not silently converted to empty strings.
  **Evidence:** Empty string and NULL are different database states. V1 has no
  nullable row helper, so `db_row_get` should return `Err(string)` for NULL
  values unless Stage 7 product review chooses a named nullable API.

- **Decision:** File-backed SQLite paths are workspace-confined; `:memory:` is
  accepted as a local in-memory database special case.
  **Evidence:** #378 already defines DUUMBI's workspace-confined path policy,
  and allowing arbitrary file paths would create a broader filesystem access
  surface than the accepted local persistence outcome needs.

- **Decision:** These modules remain opt-in and registry-add-only until #381 or
  a later accepted spec changes distribution policy.
  **Evidence:** The Stage 5 pre-build decision explicitly keeps them out of
  default `duumbi init` workspaces.

- **Decision:** This spec PR must leave the execution issue open.
  **Evidence:** Stage 6 produces a review candidate. Stage 7 approval, Stage 8
  technical specification, implementation, review, merge, and closure still need
  to happen.

## Behavior

### Module Availability

`@duumbi/stdlib-http` and `@duumbi/stdlib-db` are opt-in modules.

TLS v1 is exposed to users as verified HTTPS behavior through
`@duumbi/stdlib-http`. If implementation needs a separate
`@duumbi/stdlib-tls` package or internal artifact, that package must not expose
raw TLS socket APIs in v1.

New workspaces created by `duumbi init` continue to include the current default
stdlib dependencies unless #381 or a later accepted spec changes that policy.

Importing, parsing, validating, compiling, packaging, or inspecting these
modules does not create HTTP requests, TLS handshakes, or database connections.

### HTTP Request Inputs

HTTP functions accept:

- `url` as a DUUMBI `string`.
- `headers` as a DUUMBI `json` object.
- `body` as a DUUMBI `string` for POST and PUT.
- `timeout_ms` as a positive `i64` number of milliseconds.

Header JSON must be an object. Keys are header names. Values are strings.
Non-string header values return `Err(string)`.

Header names are case-insensitive for HTTP semantics. The implementation must
document the canonical form it uses in returned header JSON.

Unsupported or malformed URL strings return `Err(string)`.

Unsupported schemes return `Err(string)`. V1 supports `http://` and
`https://`.

### HTTP Request Outputs

On successful HTTP transport, request functions return `Ok(http_response)`.

`http_status(response)` returns `Ok(i64)` with the response status code.

`http_headers(response)` returns `Ok(json)` for the deterministic header JSON
object.

`http_body(response)` returns `Ok(string)` when the response body can be safely
represented as a DUUMBI string.

`http_response_free(response)` returns `Ok(0)` when the response resource is
released successfully.

HTTP status codes such as 301, 302, 400, 404, and 500 remain inspectable
responses when the transport and protocol exchange completed.

### HTTP Error States And Recovery

Expected HTTP failures return `Err(string)`:

- unsupported URL scheme;
- malformed URL;
- invalid header JSON shape;
- non-string header value;
- invalid `timeout_ms`, including `timeout_ms <= 0`;
- DNS failure;
- connection failure;
- request timeout;
- TLS/certificate failure for HTTPS;
- response protocol failure;
- response body cannot be represented as a DUUMBI string;
- allocation/resource failure where recoverable;
- use of an already released response resource.

The error string should be concise, plain English, and specific enough for
tests and agents to identify the error class without depending on full
operating-system wording.

A user graph may retry a failed HTTP request by calling the request function
again. There is no hidden automatic retry in v1.

### HTTPS And TLS Behavior

HTTPS requests made through `@duumbi/stdlib-http` verify certificates by
default.

If certificate verification fails, the HTTP request returns `Err(string)`.

Safe/default APIs do not bypass certificate verification.

V1 does not expose raw TLS sockets, certificate store mutation APIs, client
certificate APIs, or custom trust configuration as public graph functions.

The implementation and tests must prove the default verification behavior
without requiring public internet services in default CI.

### Database Inputs

`db_open(path)` accepts either:

- `:memory:` for a local in-memory SQLite database; or
- a workspace-confined relative path for a file-backed SQLite database.

File-backed DB paths follow the #378 workspace path policy:

- reject absolute paths;
- reject path traversal that escapes the workspace;
- reject home expansion, environment-variable expansion, URL-like paths, and
  platform drive prefixes as v1 unsupported behavior;
- validate the operation path at the DB open boundary, not only through helper
  functions.

`db_execute` and `db_query` accept:

- an open `db_connection`;
- a SQL string;
- `array<string>` parameters bound in order.

V1 does not expose named parameters, typed parameters, blob parameters,
statement handles, or transaction helper APIs.

### Database Outputs

`db_open` returns `Ok(db_connection)` on success.

`db_execute` returns `Ok(i64)` with the number of rows changed by the statement
when SQLite reports that value. Statements such as table creation may return
`Ok(0)`.

`db_query` returns `Ok(db_rows)` on success.

`db_rows_len(rows)` returns `Ok(i64)` with the row count.

`db_row_get(rows, row_index, column)` returns `Ok(string)` when the row and
column exist and the SQLite value can be represented as a DUUMBI string.

`db_rows_free(rows)` returns `Ok(0)` when the row set is released successfully.

`db_close(conn)` returns `Ok(0)` when the connection is closed successfully.

### Database Error States And Recovery

Expected DB failures return `Err(string)`:

- invalid database path;
- workspace path-policy violation;
- missing or invalid parent directory for a file-backed database;
- permission failure;
- SQLite open failure;
- SQL syntax or execution failure;
- parameter count mismatch;
- unsupported parameter or row value representation;
- row index below zero;
- row index out of range;
- missing column name;
- SQL NULL returned through `db_row_get`;
- use of closed connection;
- use of released row set;
- close/free failure where recoverable.

The DB API does not hide SQL errors as empty row sets or success codes.

A user graph may retry DB operations when the connection or row resource state
still makes the retry valid. There is no hidden automatic retry in v1.

### Empty States

HTTP:

- An empty request body is valid for POST and PUT.
- Empty request headers use an empty JSON object.
- Empty response headers return an empty JSON object when the runtime genuinely
  has no headers to report.
- An empty response body returns `Ok("")`.

DB:

- `db_open(":memory:")` succeeds when SQLite can create an in-memory database.
- `db_execute` with no parameters uses an empty `array<string>`.
- `db_query` that returns no rows yields `Ok(db_rows)`, and `db_rows_len`
  returns `Ok(0)`.
- `db_row_get` on an empty row set returns `Err(string)`.
- Empty string parameter values are valid SQL text parameter values.
- Empty SQL strings return `Err(string)`.

### Cancellation, Offline, And Timeout Behavior

There is no separate cancellation API in v1.

HTTP timeout is the bounded cancellation mechanism for outbound HTTP operations.

Offline, DNS-unavailable, refused, unreachable, and permission-denied network
states return `Err(string)`.

SQLite operations do not have a public timeout parameter in v1. The technical
spec may add bounded internal busy-timeout behavior if needed, but it must not
change the public API without product review.

Tests must use conservative timeouts and local fixtures so default CI cannot
hang indefinitely or depend on external service uptime.

### Race Conditions And Invariants

HTTP response resources must remain opaque at the graph boundary.

`db_connection` and `db_rows` resources must remain opaque at the graph
boundary.

Repeated response free, row free, or DB close behavior must be safe. It may
return `Err(string)`, but it must not double-free or crash.

Operations on released or closed resources must return `Err(string)` when
recoverable.

No public API introduced by #380 may imply that server, WebSocket, browser, or
remote database behavior is complete.

Loopback tests must account for local server startup ordering with deterministic
synchronization or bounded retry inside the test harness.

SQLite file tests must use isolated temporary workspaces or in-memory databases.

No #380 test may depend on public internet, production registry services, shared
database files, credentials, or ambient machine-specific state.

### Accessibility And User-Facing Text

No new graphical UI behavior is required by this issue.

If examples or docs include CLI, REPL, or Studio output, HTTP/TLS/DB errors
should be readable as plain text and should not rely on color-only meaning.

Error strings should avoid leaking secrets, authorization headers, payloads, SQL
parameters, SQL result values, file contents, or database contents.

## BDD Scenarios

Feature: HTTP client functions return inspectable responses

  Scenario: GET returns status, headers, and body from a loopback service
    Given a loopback HTTP service responds to `/status` with status `200`
    And the response includes header `content-type: application/json`
    And the response body is `{"ok":true}`
    And a DUUMBI program imports `@duumbi/stdlib-http`
    When the program calls `http_get` with the loopback URL, empty JSON headers,
      and a positive timeout
    And the program calls `http_status`, `http_headers`, and `http_body`
    Then `http_get` returns `Ok(http_response)`
    And `http_status` returns `Ok(200)`
    And `http_headers` returns `Ok(json)` containing the content type
    And `http_body` returns `Ok("{\"ok\":true}")`

  Scenario: Non-2xx responses remain inspectable
    Given a loopback HTTP service responds with status `404`
    And the response body is `not found`
    When the program calls `http_get`
    And then calls `http_status` and `http_body`
    Then `http_get` returns `Ok(http_response)`
    And `http_status` returns `Ok(404)`
    And `http_body` returns `Ok("not found")`

  Scenario: POST sends headers and body
    Given a loopback HTTP service records request headers and body
    And a DUUMBI program has header JSON `{"content-type":"application/json"}`
    And the body string is `{"name":"duumbi"}`
    When the program calls `http_post` with a positive timeout
    Then the result is `Ok(http_response)`
    And the service receives the content-type header
    And the service receives the exact request body

  Scenario: Invalid header JSON is a recoverable error
    Given a DUUMBI program imports `@duumbi/stdlib-http`
    And the program has a header value that is not a JSON object
    When the program calls `http_get`
    Then the result is `Err(string)`
    And the error text identifies the header shape problem

  Scenario: Request timeout returns an error instead of hanging
    Given a loopback HTTP service delays longer than the request timeout
    When the program calls `http_get` with a short positive timeout
    Then the result is `Err(string)`
    And the call returns within the accepted timeout tolerance

  Scenario: Redirects are returned as ordinary responses
    Given a loopback HTTP service responds with status `302`
    And the response includes a `location` header
    When the program calls `http_get`
    Then the result is `Ok(http_response)`
    And `http_status` returns `Ok(302)`
    And `http_headers` exposes the location header
    And the client does not automatically issue a second request

  Scenario: Response resources can be released safely
    Given a DUUMBI program receives `Ok(http_response)`
    When the program calls `http_response_free`
    And then calls `http_status` on the same response
    Then `http_response_free` returns `Ok(0)`
    And the later `http_status` returns `Err(string)`
    And the program does not crash

Feature: HTTPS verifies certificates by default

  Scenario: Verified HTTPS succeeds under the accepted local test harness
    Given a local HTTPS test service is configured with a certificate trusted by
      the accepted test harness
    And a DUUMBI program imports `@duumbi/stdlib-http`
    When the program calls `http_get` with an `https://` URL and a positive
      timeout
    Then the result is `Ok(http_response)`
    And the response status and body are inspectable

  Scenario: Untrusted certificates fail visibly
    Given a local HTTPS test service presents an untrusted certificate
    When the program calls `http_get` with an `https://` URL and a positive
      timeout
    Then the result is `Err(string)`
    And the error text identifies a TLS or certificate verification failure
    And no safe/default API bypasses the verification failure

  Scenario: Raw TLS socket APIs are not exported in v1
    Given the implementation provides #380 stdlib manifests
    When the manifest exports are inspected
    Then `@duumbi/stdlib-http` exposes the accepted HTTP functions
    And no v1 manifest exports `tls_connect`, `tls_wrap`, `tls_read`,
      `tls_write`, or `tls_close`

Feature: SQLite local persistence works through opaque resources

  Scenario: Open an in-memory database, insert a row, and query it
    Given a DUUMBI program imports `@duumbi/stdlib-db`
    When the program calls `db_open(":memory:")`
    And calls `db_execute` to create a table
    And calls `db_execute` with parameter array `["Ada"]` to insert a row
    And calls `db_query` to read the row
    And calls `db_rows_len`
    And calls `db_row_get` for column `name`
    Then `db_open` returns `Ok(db_connection)`
    And the create statement returns `Ok(0)`
    And the insert statement returns an `Ok(i64)` changed-row count
    And `db_query` returns `Ok(db_rows)`
    And `db_rows_len` returns `Ok(1)`
    And `db_row_get` returns `Ok("Ada")`

  Scenario: File-backed databases stay inside the workspace
    Given a DUUMBI workspace root
    And a DUUMBI program imports `@duumbi/stdlib-db`
    When the program calls `db_open("data/demo.sqlite")`
    Then the result is `Ok(db_connection)`
    When the program calls `db_open("../outside.sqlite")`
    Then the result is `Err(string)`
    And the error text identifies a workspace path-policy failure

  Scenario: SQL parameters are bound instead of interpolated
    Given a DUUMBI program imports `@duumbi/stdlib-db`
    And the program has parameter value `Ada'); DROP TABLE users; --`
    When the program calls `db_execute` with SQL `insert into users(name) values (?)`
    Then the value is stored as a string parameter
    And the database does not execute the parameter text as SQL

  Scenario: Querying an empty result set is successful but row access fails
    Given a query returns no rows
    When the program calls `db_query`
    And then calls `db_rows_len`
    Then `db_query` returns `Ok(db_rows)`
    And `db_rows_len` returns `Ok(0)`
    When the program calls `db_row_get` with row index `0`
    Then the result is `Err(string)`

  Scenario: Missing columns are recoverable errors
    Given a row set contains column `name`
    When the program calls `db_row_get` with column `missing`
    Then the result is `Err(string)`
    And the program can branch on that result without a runtime panic

  Scenario: SQL NULL is not confused with empty string
    Given a row contains SQL NULL in column `note`
    When the program calls `db_row_get` for column `note`
    Then the result is `Err(string)`
    And the error text identifies that the value is not representable as a v1
      string result

  Scenario: Closed database connections reject later operations safely
    Given an open `db_connection`
    When the program calls `db_close`
    And then calls `db_query` on the same connection
    Then `db_close` returns `Ok(0)`
    And the later `db_query` returns `Err(string)`
    And the program does not crash

  Scenario: Row resources can be released safely
    Given a successful `db_query` returns `Ok(db_rows)`
    When the program calls `db_rows_free`
    And then calls `db_rows_len` on the same rows resource
    Then `db_rows_free` returns `Ok(0)`
    And the later `db_rows_len` returns `Err(string)`
    And the program does not crash

Feature: HTTP, JSON, transform, and SQLite compose into one local demo

  Scenario: Fetch local JSON and persist a transformed value
    Given a loopback HTTP service returns body `{"name":"duumbi","stars":3}`
    And a DUUMBI program imports `@duumbi/stdlib-http`
    And the program imports `@duumbi/stdlib-json`
    And the program imports `@duumbi/stdlib-db`
    When the program calls `http_get`
    And parses the response body with `parse`
    And reads the `name` field with `get_field`
    And stringifies the field
    And inserts the transformed value into SQLite
    And queries the inserted value
    Then every fallible operation returns `Ok(_)`
    And the queried row contains the expected transformed value
    And the test uses only loopback HTTP and a temporary or in-memory SQLite
      database

Feature: Distribution remains opt-in until Tier 1 publication

  Scenario: New workspaces keep the current default stdlib set
    Given a user runs `duumbi init`
    When the workspace config is generated
    Then the default dependencies still include the current default stdlib
      modules
    And the default dependencies do not include `@duumbi/stdlib-http`
    And the default dependencies do not include `@duumbi/stdlib-db`

  Scenario: Tests do not require external services
    Given CI runs the #380 test suite
    When HTTP, HTTPS, and SQLite behavior is verified
    Then HTTP tests use loopback services
    And HTTPS tests use local certificate fixtures or an equivalent local test
      harness
    And SQLite tests use temporary workspace files or in-memory databases
    And no test requires public internet, third-party credentials, production
      services, or external service uptime

## Tasks

Implementation should be broken down after Stage 7 and Stage 8 approval. The
following product task groups are independent enough to plan separately:

1. HTTP API and type contract:
   Define `http_response`, accepted type strings, request/response result
   shapes, timeout validation, header JSON shape, status/body/header access, and
   resource cleanup behavior.

2. HTTP runtime behavior:
   Implement GET, POST, PUT, DELETE, status access, body access, header access,
   response cleanup, deterministic error construction, non-2xx handling, and no
   automatic redirects.

3. TLS behavior for HTTPS:
   Implement verified HTTPS as the default path, certificate-error reporting,
   local test harness support, and no public raw TLS socket exports.

4. HTTP module packaging:
   Add the stdlib graph module, manifest export list, cache/module plumbing, and
   module-facing documentation for the accepted v1 HTTP functions.

5. DB API and type contract:
   Define `db_connection` and `db_rows`, accepted type strings, path behavior,
   prepared parameter binding, string-only row extraction, NULL behavior, row
   count behavior, and cleanup behavior.

6. DB runtime behavior:
   Implement open, execute, query, rows length, row get, close, rows free,
   SQLite error conversion, workspace path handling, and safe closed-resource
   behavior.

7. DB module packaging:
   Add the stdlib graph module, manifest export list, cache/module plumbing, and
   module-facing documentation for the accepted v1 DB functions.

8. Cross-platform dependency and linker strategy:
   Record the chosen HTTP/TLS/SQLite implementation strategy, vendored vs
   system requirements, linker flags, packaging implications, and supported
   target behavior.

9. Verification:
   Add focused unit, integration, and local E2E smoke tests that map back to the
   BDD scenarios without external internet or shared database dependencies.

10. Review evidence:
   Record accepted API surfaces, security boundaries, dependency strategy, BDD
   coverage, local fixture evidence, lifecycle evidence, and distribution
   behavior in the implementation PR.

11. Tier 1 sequencing:
   Leave registry publication and broad installed-module smoke coverage to #381
   and #382 unless a later approved spec changes sequencing.

## Checks

Stage 6 spec checks:

- Product spec exists at `specs/DUUMBI-380/PRODUCT.md`.
- Spec content is English.
- Spec follows the DUUMBI product spec structure.
- Spec includes BDD scenarios for HTTP, HTTPS/TLS, SQLite, composition,
  distribution, and local-test-boundary behavior.
- Spec PR changes only `specs/DUUMBI-380/PRODUCT.md`.
- Spec PR title, body, commit message, and spec text use non-closing references
  such as `Related to #380` or `Spec for #380`.
- Spec PR body includes a workflow note that the PR is specification-only and
  must leave the execution issue open.
- Codex self-review finds no blocking product/spec issue.
- Required automated review evidence exists as an actual non-dismissed Copilot
  review submission.
- Checks are green, neutral, skipped, or explicitly not applicable.
- Review threads are resolved, including outdated unresolved threads after
  later changes.
- Greptile is not invoked unless the developer explicitly requests a manual
  deep review.

Implementation checks after later approval:

- Parser/type tests cover `http_response`, `db_connection`, and `db_rows` type
  strings or the accepted equivalent representation.
- Graph validation tests reject invalid use of #380 result/resource values where
  applicable.
- Runtime C compile/link checks pass on supported targets.
- Cross-platform dependency/linking behavior is documented and tested where
  feasible.
- `@duumbi/stdlib-http` graph and manifest export exactly the accepted v1 HTTP
  functions.
- TLS behavior is documented as verified HTTPS through HTTP v1, with no raw
  public TLS socket exports.
- `@duumbi/stdlib-db` graph and manifest export exactly the accepted v1 DB
  functions.
- HTTP tests cover GET, POST, PUT, DELETE, status access, body access, header
  access, non-2xx responses, redirect response behavior, invalid header JSON,
  invalid timeout, timeout, malformed URL, unsupported scheme, response cleanup,
  closed-resource behavior, and nonrepresentable body behavior.
- HTTPS tests cover successful verified HTTPS under the accepted local harness
  and untrusted certificate failure.
- DB tests cover in-memory open, workspace-confined file open, path escape
  rejection, create table, insert, execute row count, query, rows length, row
  get, parameter binding, missing column, invalid row index, SQL NULL behavior,
  close, rows free, closed-resource behavior, and SQL error handling.
- Composition tests cover local API fetch -> JSON parse -> DUUMBI transform ->
  SQLite write/query.
- Tests prove outbound HTTP operations cannot hang indefinitely.
- Tests prove no default CI path uses public internet, third-party endpoints,
  credentials, production services, or shared database state.
- Distribution tests prove default `duumbi init` dependencies remain unchanged
  unless #381 or a later accepted spec changes the policy.
- Documentation or module descriptions explain HTTP/TLS/DB usage, error
  behavior, timeout requirements, resource cleanup, security boundaries,
  dependency requirements, and out-of-scope features.
- `cargo fmt --check` passes when implementation code exists.
- `cargo clippy --all-targets -- -D warnings` passes when implementation code
  exists.
- `cargo test --all` passes when implementation code exists.

BDD coverage expectations:

- HTTP request/response scenarios are covered by local loopback integration
  tests.
- HTTP timeout scenarios are covered by bounded tests with conservative
  tolerances.
- HTTPS scenarios are covered by local certificate fixtures, an accepted local
  harness, or equivalent deterministic evidence that does not depend on public
  internet.
- DB scenarios are covered by temporary workspace files and/or in-memory SQLite
  integration tests.
- Resource lifecycle scenarios are covered by explicit cleanup tests and
  repeated close/free behavior tests.
- Composition scenario is covered by a local E2E or integration smoke test after
  #379 JSON support is available in the implementation branch.
- Default-init and manifest scenarios are covered by init/cache/module tests.
- External-service exclusion is covered by test design and PR evidence.

## Open Questions

None blocking for product specification.

Stage 8 must still decide technical details that do not change the accepted
product behavior, including:

- exact HTTP/TLS implementation dependency;
- vendored/static versus system library strategy for HTTP/TLS/SQLite;
- exact linker flags and packaging behavior on macOS, Linux, and Windows;
- exact internal layout and ownership implementation for `http_response`,
  `db_connection`, and `db_rows`;
- exact response-header canonicalization and duplicate-header handling;
- exact response-body text decoding policy;
- exact SQLite value-to-string conversion policy for numbers and booleans;
- exact SQLite busy-timeout or lock-handling behavior;
- exact local HTTPS verification test harness.

If any of those technical decisions would change the public API, error contract,
security boundary, default-init policy, local-test requirement, or accepted
scope above, Stage 8 must route the change back for product review instead of
silently broadening #380.

## Sources

- GitHub issue #380: `https://github.com/hgahub/duumbi/issues/380`
- Stage 4 triage refill comment for #380:
  `https://github.com/hgahub/duumbi/issues/380#issuecomment-4631822507`
- V1 decisions comment for #380:
  `https://github.com/hgahub/duumbi/issues/380#issuecomment-4634627905`
- Stage 5 human acceptance comment for #380:
  `https://github.com/hgahub/duumbi/issues/380#issuecomment-4634633057`
- Related issue #378: `https://github.com/hgahub/duumbi/issues/378`
- Related issue #379: `https://github.com/hgahub/duumbi/issues/379`
- Related issue #381: `https://github.com/hgahub/duumbi/issues/381`
- Related issue #382: `https://github.com/hgahub/duumbi/issues/382`
- `specs/DUUMBI-378/PRODUCT.md`
- `specs/DUUMBI-379/PRODUCT.md`
- `docs/architecture.md`
- `docs/coding-conventions.md`
- `docs/automation/code-review-policy.md`
- `docs/automation/agentic-development-orchestration.md`
- `.github/workflows/copilot-review.yml`
- `.github/workflows/spec-review-request.yml`
- `Cargo.toml`
- `stdlib/file.jsonld`
- `stdlib/json.jsonld`
- `stdlib/net.jsonld`
- `src/types.rs`
- `src/parser/mod.rs`
- `src/compiler/lowering.rs`
- `src/compiler/linker.rs`
- `runtime/duumbi_runtime.c`
- `runtime/duumbi_runtime.h`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Active Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Active Agentic Development Runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Spec-First Agentic Development dot:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Spec-First Agentic Development.md`
- Phase 14 roadmap context:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 14 - Marketing & Go-to-Market.md`
