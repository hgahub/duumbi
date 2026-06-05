# DUUMBI-379: @duumbi/stdlib-json And @duumbi/stdlib-net Modules

## Summary

Add two Tier 1 standard library modules that let DUUMBI graph programs work
with structured data and raw TCP connections without host-side glue code:

- `@duumbi/stdlib-json` for parsing JSON text, stringifying JSON values, reading
  object fields, and traversing arrays by length and index.
- `@duumbi/stdlib-net` for explicit-timeout TCP connect, listen, accept, read,
  write, and close operations.

The accepted v1 product boundary is JSON plus TCP only. JSON values and TCP
resources must be first-class DUUMBI heap/resource types at the graph API
boundary, not plain user-visible integer handles. Failures must use
`result<_, string>` so user graphs can branch on errors instead of panicking or
silently receiving invalid data.

Related to #379. This is a Stage 6 product specification only. The execution
issue must remain open for Stage 7 review, Stage 8 technical specification,
implementation, implementation review, and Stage 12 closure.

## Problem

The current repository standard library modules cover math, print-oriented I/O,
language helpers, and string helpers. That is enough for local computation demos,
but not enough for common integration work where data arrives as structured JSON
or leaves through a socket/service boundary.

Without this work:

- DUUMBI examples must hard-code structured inputs or use host scripts to parse
  JSON.
- Later HTTP, TLS, database, and server modules either duplicate low-level
  runtime behavior or wait for a reusable JSON/TCP foundation.
- Users cannot build credible local integration demos that parse payloads,
  choose values from the graph, and exchange data with loopback or remote TCP
  services.
- Network and JSON failures have no accepted stdlib-level error contract.

The product requirement is not a broad integration platform. It is the smallest
reviewable foundation that makes JSON and raw TCP useful, bounded, and testable
inside DUUMBI's graph-first runtime model.

## Outcome

When this is done:

- Users can add `@duumbi/stdlib-json` and parse a DUUMBI string into a
  first-class `json` value.
- Users can retrieve object fields, get array lengths, get array items, and
  stringify JSON values back into DUUMBI strings.
- JSON parse errors, missing fields, wrong JSON kinds, invalid indexes, and
  stringify failures return `Err(string)` with human-readable messages.
- Users can add `@duumbi/stdlib-net` and open TCP client or listener flows with
  explicit timeout values on every blocking operation.
- TCP connect, listen, accept, read, write, and close operations return
  `result<_, string>` and cannot hang tests indefinitely.
- Socket and listener lifecycle is explicit: resources can be closed, later
  operations on closed resources report errors, and repeated close/drop behavior
  is safe rather than crashing or double-freeing.
- New JSON and TCP stdlib modules have graph files, manifests/export lists, and
  source-level distribution plumbing consistent with the accepted Tier 1
  approach.
- New workspaces do not receive `@duumbi/stdlib-json` or `@duumbi/stdlib-net` as
  default dependencies unless #381 or a later accepted spec explicitly changes
  the Tier 1 default-init policy.
- Tests prove JSON behavior and loopback TCP behavior without relying on
  external internet services.

## Scope

### In Scope

- Add a durable product contract for `@duumbi/stdlib-json`.
- Add a durable product contract for `@duumbi/stdlib-net`.
- Define the accepted v1 JSON API:
  - `parse(input: string) -> result<json, string>`
  - `stringify(value: json) -> result<string, string>`
  - `get_field(value: json, key: string) -> result<json, string>`
  - `array_len(value: json) -> result<i64, string>`
  - `array_get(value: json, index: i64) -> result<json, string>`
- Define `json` as a first-class DUUMBI heap type represented internally by an
  opaque runtime value. Users must not see or manipulate raw pointer/integer
  handles as the JSON API.
- Preserve mixed JSON values including objects, arrays, strings, numbers,
  booleans, and `null` without forcing them into homogeneous `array<T>` or
  named `struct<T>` conversions in v1.
- Define JSON value ownership and cleanup expectations so repeated parse/access
  flows do not leak, double-free, or allow use-after-free behavior.
- Define the accepted v1 TCP API:
  - `tcp_connect(host: string, port: i64, timeout_ms: i64) -> result<tcp_socket, string>`
  - `tcp_listen(host: string, port: i64, timeout_ms: i64) -> result<tcp_listener, string>`
  - `tcp_accept(listener: tcp_listener, timeout_ms: i64) -> result<tcp_socket, string>`
  - `tcp_read(socket: tcp_socket, max_bytes: i64, timeout_ms: i64) -> result<string, string>`
  - `tcp_write(socket: tcp_socket, data: string, timeout_ms: i64) -> result<i64, string>`
  - `tcp_close(socket: tcp_socket) -> result<i64, string>`
  - `tcp_listener_close(listener: tcp_listener) -> result<i64, string>`
- Define `tcp_socket` and `tcp_listener` as first-class DUUMBI resource types
  represented internally by opaque runtime values. Users must not see or
  manipulate raw socket descriptors or integer handles as the API.
- Require explicit `timeout_ms` parameters for all TCP operations that can block.
- Require TCP behavior to be testable through loopback flows in CI or local
  integration tests. Loopback listener tests should use a free port selected by
  the test harness before the DUUMBI program runs, and port-collision failures
  should be handled as retryable test setup rather than a new stdlib API.
- Add source-level module artifacts and manifest export lists in the repository
  path accepted by the technical spec.
- Update parser, type, lowering, runtime, cache/module, and linker surfaces only
  as required by the later approved technical spec to satisfy this product
  contract.
- Document the module behavior and boundaries in user-facing or module-facing
  text that implementation reviewers can inspect.
- Keep the spec PR non-closing and limited to this product spec file.

### Explicitly Out Of Scope

- Technical specification creation during Stage 6.
- Implementation code, source tests, runtime changes, manifest changes, or Ralph
  cycles during Stage 6.
- HTTP methods, headers, status-code helpers, URL parsing, TLS, certificate
  verification, SQLite, database protocols, and server routing. Those belong to
  #380, #381, or later accepted work.
- UDP in v1.
- JSONPath, JQ-style querying, schema validation, JSON patch, streaming JSON,
  or a general JSON query language.
- A true JSON array iterator object in v1.
- Arbitrary conversion between JSON and DUUMBI `array<T>` or `struct<T>` values.
- Hidden global TCP timeout defaults.
- Async/event-loop APIs, connection pooling, production server concurrency,
  backpressure abstractions, TLS wrapping, or protocol-specific clients.
- Internet-dependent tests, tests requiring third-party services, or tests that
  require public DNS/network availability.
- Arbitrary binary protocol support beyond values that can be safely represented
  by the accepted DUUMBI string contract. If incoming bytes cannot be safely
  represented as a DUUMBI string, the operation must report `Err(string)` rather
  than corrupting or truncating data.
- Publishing these modules to `registry.duumbi.dev`; #381 owns Tier 1
  publication.
- Adding these modules to default `duumbi init` dependencies; #381 or a later
  accepted product decision owns any default-init policy change.
- E2E coverage for all Tier 1 stdlib modules; #382 owns broad ecosystem smoke
  tests.

## Constraints And Assumptions

Facts:

- Issue #379 is open and titled `feat(stdlib): @duumbi/stdlib-json +
  @duumbi/stdlib-net modules`.
- Issue #379 has labels `phase-14`, `module:ecosystem`, `accepted`, and
  `needs-spec`.
- Stage 5 accepted #379 on 2026-06-05 from Slack reviewer source `hga`, with no
  remaining open questions and next state `Spec Needed`.
- Stage 4 routed #379 to `Needs Human Acceptance` as an unblocked P1 ecosystem
  feature.
- Existing repo stdlib graph files are `stdlib/math.jsonld`, `stdlib/io.jsonld`,
  `stdlib/lang.jsonld`, and `stdlib/string.jsonld`.
- `duumbi init` currently embeds those four stdlib modules into the cache and
  lists `@duumbi/stdlib-math`, `@duumbi/stdlib-io`, `@duumbi/stdlib-lang`, and
  `@duumbi/stdlib-string` as default dependencies.
- `DuumbiType` currently includes primitive types plus `string`, `array<T>`,
  `struct<Name>`, references, `result<T,E>`, and `option<T>`. It does not yet
  include `json`, `tcp_socket`, or `tcp_listener`.
- The parser currently recognizes type strings for primitives, arrays, structs,
  results, options, and references.
- Compiler lowering currently treats existing heap/result/option values as
  pointer-sized opaque runtime values at the Cranelift boundary.
- `runtime/duumbi_runtime.c` currently provides print, string, array, struct,
  result, option, math, telemetry, and string-utility shims. It does not yet
  provide JSON or TCP runtime functions.
- `@duumbi/stdlib-io` currently wraps print functions and is already included in
  default init dependencies.
- Related issue #378 covers stdin and workspace-confined file I/O and is the
  natural source-side complement for JSON examples.
- Related issue #380 covers HTTP, TLS, and database modules and depends on
  shared JSON/TCP conventions from #379.
- Related issue #381 covers `@duumbi/stdlib-server` and publication of all Tier
  1 modules to the registry.
- Related issue #382 covers E2E smoke tests for installed Tier 1 modules.
- The active PRD says GitHub Issues, PRs, CI, and Project state hold execution
  state, while specs capture explicit behavior and evidence requirements before
  implementation.
- DUUMBI review policy requires Codex self-review plus Copilot review evidence
  for file-based Stage 6 product spec PRs. Greptile is manual-only and is not
  part of the default spec gate.

Assumptions:

- #379 may be specified independently of #378, but implementation and examples
  should align with #378 where stdin/file data becomes a natural JSON source.
- The implementation stage can introduce new graph-level resource types when the
  approved technical spec shows the parser, type, lowering, runtime, and
  ownership changes needed.
- TCP tests can use loopback servers/listeners and deterministic timeouts rather
  than external services.
- A blocking socket runtime with explicit timeouts is sufficient for v1.
- JSON object field order is not a product guarantee. Semantic JSON equivalence
  matters more than preserving input formatting or key order.
- JSON numbers are representable enough for parse/stringify and field traversal
  in v1, but exact numeric precision policy belongs in the technical spec as
  long as behavior is deterministic and documented.

Constraints:

- Stage 6 must not create a technical spec or implementation code.
- The spec-only PR must use non-closing issue references such as `Related to
  #379` or `Spec for #379`.
- The execution issue must remain open through later DUUMBI workflow stages.
- JSON/TCP APIs must return `result<_, string>` for recoverable failures.
- No accepted API may expose raw user-visible integer handles for JSON values,
  sockets, or listeners.
- TCP operations that can block must have explicit timeout parameters and must
  be testable without indefinite hangs.
- Raw TCP introduces network access from graph programs. The module must be
  opt-in, have visible call sites, and must not create implicit network activity
  during workspace initialization or unrelated compilation.
- Tests must not rely on public internet availability, third-party endpoints, or
  credentials.
- Runtime errors must not log credentials, tokens, or socket payloads unless a
  later accepted spec explicitly defines safe diagnostic behavior.

## Decisions

- **Decision:** Use a file-based product spec for #379.
  **Evidence:** The accepted work is cross-module, runtime-facing,
  user-visible, and foundational for #380, #381, and #382. It needs durable
  review history in the source repository.

- **Decision:** `@duumbi/stdlib-json` v1 exposes `parse`, `stringify`,
  `get_field`, `array_len`, and `array_get`.
  **Evidence:** These functions are explicitly listed in the accepted issue and
  provide useful JSON handling without creating a full query language.

- **Decision:** JSON values are first-class `json` heap values, not user-visible
  `i64` handles.
  **Evidence:** The accepted issue explicitly rejects plain user-visible
  integer handles because mixed arrays, `null`, nested objects, booleans,
  numbers, and strings should remain representable without fragile conversions.

- **Decision:** JSON failures use `result<_, string>`, not `option<_>`.
  **Evidence:** The accepted issue says parse errors, missing fields, type
  mismatches, invalid indexes, and stringify failures should carry
  human-readable error text.

- **Decision:** v1 JSON array traversal uses `array_len` and `array_get`, not an
  iterator object.
  **Evidence:** The accepted issue states that a true `array_iter` abstraction is
  out of scope for v1 because iterator state/lifetime semantics are not yet
  necessary.

- **Decision:** `@duumbi/stdlib-net` v1 exposes raw TCP client and listener
  primitives with explicit timeouts.
  **Evidence:** The accepted issue lists connect, listen, accept if needed,
  read, write, close, and listener lifecycle functions, and requires explicit
  `timeout_ms` values.

- **Decision:** v1 includes `tcp_listener` and `tcp_accept`.
  **Evidence:** Loopback/server smoke tests need a DUUMBI-owned way to prove
  both sides of the raw TCP lifecycle without external services, and #381's
  server module will need listener conventions later.

- **Decision:** TCP socket and listener values are first-class resource types,
  not user-visible descriptors or integer handles.
  **Evidence:** This matches the accepted JSON handle boundary and prevents
  users from accidentally treating OS descriptors or runtime pointers as stable
  DUUMBI data.

- **Decision:** TCP payloads are exposed as DUUMBI strings in v1, with visible
  errors for bytes that cannot be safely represented.
  **Evidence:** The accepted API uses `string` for `tcp_read` and `tcp_write`,
  and DUUMBI does not yet have an accepted byte-buffer type.

- **Decision:** No hidden global timeout default is accepted.
  **Evidence:** The accepted issue explicitly says timeout values must be
  function parameters so generated graphs remain auditable and tests cannot hang
  indefinitely.

- **Decision:** `@duumbi/stdlib-json` and `@duumbi/stdlib-net` remain
  registry-add-only until #381 unless a later accepted product spec changes
  default init behavior.
  **Evidence:** The accepted issue says not to add these modules to default
  `duumbi init` dependencies immediately.

- **Decision:** This spec PR must leave the execution issue open.
  **Evidence:** Stage 6 produces a review candidate. Stage 7 approval, Stage 8
  technical specification, implementation, review, merge, and closure still need
  to happen.

## Behavior

### Defaults

- `@duumbi/stdlib-json` and `@duumbi/stdlib-net` are opt-in modules.
- New workspaces created by `duumbi init` continue to include the current default
  stdlib dependencies unless #381 or a later accepted spec changes that policy.
- Importing or compiling a workspace does not create network traffic by itself.
- Network activity happens only when graph logic calls `@duumbi/stdlib-net`
  functions.
- TCP operations use caller-supplied `timeout_ms` values; there is no hidden
  default timeout.
- JSON object key order, whitespace, and formatting are not preserved by
  default. `stringify` must produce valid JSON that represents the same JSON
  value, subject to the documented numeric policy.
- All recoverable JSON/TCP errors are returned as `Err(string)`.

### Inputs

- JSON text as a DUUMBI `string`.
- JSON object field names as DUUMBI `string` keys.
- JSON array indexes as `i64`.
- TCP host as a DUUMBI `string`.
- TCP port as `i64`.
- TCP timeout values as `i64` milliseconds.
- TCP read limits as `i64` byte counts.
- TCP write payloads as DUUMBI `string` values.
- TCP socket and listener resources returned by prior successful calls.

### Outputs

- `json` values for successful parse, field access, and array item access.
- DUUMBI `string` values for successful JSON stringify and TCP read.
- `i64` values for JSON array length, TCP bytes written, and successful close
  status.
- `tcp_socket` values for successful connect and accept.
- `tcp_listener` values for successful listen.
- `Err(string)` values for all recoverable JSON, TCP, argument, timeout,
  resource-lifecycle, and representability errors.

### Visible States

- JSON parse succeeds and returns `Ok(json)`.
- JSON parse fails and returns `Err(string)` with parse context useful enough for
  a user or agent to understand the failure.
- A JSON value is an object, array, string, number, boolean, or `null`; functions
  that need a specific kind reject other kinds with `Err(string)`.
- A TCP socket is open, timed out, closed, or failed.
- A TCP listener is open, timed out while accepting, closed, or failed.
- A socket peer closes the connection before any data is available; `tcp_read`
  returns `Err(string)` that identifies peer close or EOF instead of returning an
  ambiguous empty success, hanging, or panicking.
- A socket peer closes the connection after some data has been read; `tcp_read`
  may return `Ok(partial_string)` for the bytes already received when they are
  safely representable, and a later read reports EOF/peer close as `Err(string)`.
- A write succeeds fully or partially and reports the number of bytes accepted by
  the runtime/OS boundary.

### Empty States

- Parsing an empty string returns `Err(string)`.
- Parsing valid empty JSON values such as `{}` and `[]` returns `Ok(json)`.
- `array_len([])` returns `Ok(0)`.
- `array_get([], 0)` returns `Err(string)`.
- `get_field({}, "missing")` returns `Err(string)`.
- `tcp_write(socket, "", timeout_ms)` returns `Ok(0)` if the socket is open and
  the empty write is accepted as a no-op.
- `tcp_read(socket, max_bytes, timeout_ms)` with no available data before the
  timeout returns `Err(string)`.

### Error States And Recovery

- Invalid JSON syntax returns `Err(string)`.
- `get_field` on a non-object returns `Err(string)`.
- Missing object fields return `Err(string)`.
- `array_len` or `array_get` on a non-array returns `Err(string)`.
- Negative array indexes return `Err(string)`.
- Out-of-bounds array indexes return `Err(string)`.
- Stringify failure returns `Err(string)`.
- JSON allocation/resource failures return `Err(string)` where recoverable; only
  unrecoverable process-level failures may use the existing panic path.
- Invalid host strings return `Err(string)`.
- Invalid port values outside the accepted TCP port range, including `port <= 0`,
  return `Err(string)`.
- Invalid `timeout_ms` values, including `timeout_ms <= 0`, return `Err(string)`.
- Invalid `max_bytes` values, including `max_bytes <= 0`, return `Err(string)`.
- DNS, connect, bind, listen, accept, read, write, and close failures return
  `Err(string)`.
- If incoming TCP bytes cannot be safely represented as a DUUMBI string, the
  read returns `Err(string)` rather than silent truncation or replacement.
- Operations on closed sockets or listeners return `Err(string)` and must not
  crash.
- Repeated close/drop behavior must be safe and visible through `result` or
  runtime evidence; it must not double-free.
- A user graph may retry a JSON parse/access operation by calling it again.
- A user graph may retry TCP operations when the resource state still makes the
  retry valid. There is no automatic hidden retry.

### Cancellation, Offline, And Timeout Behavior

- There is no separate cancellation API in v1.
- Timeout is the bounded cancellation mechanism for blocking TCP operations.
- Offline, DNS-unavailable, refused, unreachable, and permission-denied network
  states return `Err(string)`.
- Tests must use explicit short timeouts that keep CI bounded.
- The module must not create background tasks that outlive the graph program.

### Race Conditions And Invariants

- Loopback tests must account for listener-start and client-connect ordering
  with deterministic synchronization or bounded retry inside the test harness.
- Closing a socket or listener while another accepted operation is in progress
  must not create undefined behavior, double-free, or a process crash. The
  technical spec may constrain concurrent use if DUUMBI does not yet provide
  concurrency primitives, but the documented v1 behavior must stay safe.
- `json`, `tcp_socket`, and `tcp_listener` values must stay opaque at the user
  graph boundary.
- Recoverable failures must be inspectable through DUUMBI `result` values.
- Tests must prove that no accepted TCP operation can hang indefinitely.
- No public API introduced by #379 may imply that HTTP, TLS, DB, or server
  behavior is complete.

### Accessibility And User-Facing Text

- There is no new interactive UI in #379.
- Error strings should be concise, plain English, and specific enough to support
  CLI output, test assertions, and agent diagnosis.
- Module descriptions should explain what users can build and what remains out
  of scope.

## BDD Scenarios

Feature: JSON values are first-class DUUMBI data

  Scenario: Parse a valid JSON object and read a field
    Given a DUUMBI program imports `@duumbi/stdlib-json`
    And the program has the string `{"name":"duumbi","enabled":true}`
    When the program calls `parse`
    And then calls `get_field` with the key `name`
    Then `parse` returns `Ok(json)`
    And `get_field` returns `Ok(json)`
    And stringifying the returned field produces valid JSON for `"duumbi"`

  Scenario: Invalid JSON returns a visible error
    Given a DUUMBI program imports `@duumbi/stdlib-json`
    And the program has the string `{"name":`
    When the program calls `parse`
    Then the result is `Err(string)`
    And the error text explains that JSON parsing failed

  Scenario: Missing JSON object fields are recoverable errors
    Given a parsed JSON object `{"name":"duumbi"}`
    When the program calls `get_field` with the key `missing`
    Then the result is `Err(string)`
    And the program can branch on that result without a runtime panic

  Scenario: JSON kind mismatches are recoverable errors
    Given a parsed JSON array `[1,2,3]`
    When the program calls `get_field` with the key `name`
    Then the result is `Err(string)`
    And the error text identifies that an object was expected

  Scenario: JSON arrays can be traversed by length and index
    Given a parsed JSON array `[10,20,30]`
    When the program calls `array_len`
    And then calls `array_get` with index `1`
    Then `array_len` returns `Ok(3)`
    And `array_get` returns `Ok(json)`
    And stringifying the returned item produces valid JSON for `20`

  Scenario: Invalid JSON array indexes are recoverable errors
    Given a parsed JSON array `[10,20,30]`
    When the program calls `array_get` with index `-1`
    Then the result is `Err(string)`
    When the program calls `array_get` with index `3`
    Then the result is `Err(string)`

  Scenario: Stringify produces parseable JSON for nested mixed values
    Given a parsed JSON value `{"items":[1,null,true,"x"]}`
    When the program calls `stringify`
    Then the result is `Ok(string)`
    And parsing that output returns a semantically equivalent JSON value

  Scenario: JSON resource lifecycle is safe under repeated use
    Given a test program repeatedly parses, reads, stringifies, and drops JSON
      values
    When the program runs under the accepted runtime lifecycle checks
    Then the program completes without use-after-free, double-free, or
      unbounded resource growth evidence

Feature: Raw TCP operations are bounded and explicit

  Scenario: Connect to a loopback echo service
    Given a loopback TCP echo service is listening in the test environment
    And a DUUMBI program imports `@duumbi/stdlib-net`
    When the program calls `tcp_connect` with host `127.0.0.1`, the echo port,
      and a positive timeout
    And the program calls `tcp_write` with the string `ping`
    And the program calls `tcp_read` with `max_bytes` greater than or equal to 4
    Then the connect result is `Ok(tcp_socket)`
    And the write result is `Ok(4)`
    And the read result is `Ok("ping")`

  Scenario: DUUMBI can create a loopback listener and accept a connection
    Given a DUUMBI program imports `@duumbi/stdlib-net`
    And the test harness selected a free loopback port before the program runs
    When the program calls `tcp_listen` on the loopback host and selected port
    And a client connects before the accept timeout
    And the program calls `tcp_accept`
    Then `tcp_listen` returns `Ok(tcp_listener)`
    And `tcp_accept` returns `Ok(tcp_socket)`
    And both resources can be closed explicitly

  Scenario: Connect timeout returns an error instead of hanging
    Given a DUUMBI program imports `@duumbi/stdlib-net`
    When the program calls `tcp_connect` with an unreachable target and a short
      positive timeout
    Then the result is `Err(string)`
    And the call returns within the accepted timeout tolerance

  Scenario: Read timeout returns an error instead of hanging
    Given an open TCP socket with no available incoming data
    When the program calls `tcp_read` with a positive `max_bytes` and short
      positive timeout
    Then the result is `Err(string)`
    And the call returns within the accepted timeout tolerance

  Scenario: Closed sockets reject later operations safely
    Given an open TCP socket
    When the program calls `tcp_close`
    And then calls `tcp_read` on the same socket
    Then `tcp_close` returns `Ok(0)`
    And the later `tcp_read` returns `Err(string)`
    And the program does not crash

  Scenario: Invalid TCP arguments are recoverable errors
    Given a DUUMBI program imports `@duumbi/stdlib-net`
    When the program calls `tcp_connect` with port `0`
    Then the result is `Err(string)`
    When the program calls `tcp_read` with `max_bytes` equal to `0`
    Then the result is `Err(string)`
    When the program calls `tcp_read` with negative `max_bytes`
    Then the result is `Err(string)`
    When the program calls any blocking TCP operation with `timeout_ms` equal to
      `0`
    Then the result is `Err(string)`
    When the program calls any blocking TCP operation with negative `timeout_ms`
    Then the result is `Err(string)`

  Scenario: Nonrepresentable TCP bytes are not silently corrupted
    Given an open TCP socket receives bytes that the accepted DUUMBI string
      contract cannot represent safely
    When the program calls `tcp_read`
    Then the result is `Err(string)`
    And the runtime does not silently truncate, replace, or misreport the
      payload as valid text

Feature: Module distribution remains opt-in until Tier 1 publication

  Scenario: New workspaces keep the current default stdlib set
    Given a user runs `duumbi init`
    When the workspace config is generated
    Then the default dependencies still include the current default stdlib
      modules
    And the default dependencies do not include `@duumbi/stdlib-json`
    And the default dependencies do not include `@duumbi/stdlib-net`

  Scenario: Manifests expose only the accepted v1 functions
    Given the implementation provides `@duumbi/stdlib-json`
    Then its manifest exports `parse`, `stringify`, `get_field`, `array_len`,
      and `array_get`
    And it does not export JSONPath, iterator, or schema-validation functions
    Given the implementation provides `@duumbi/stdlib-net`
    Then its manifest exports the accepted TCP socket and listener lifecycle
      functions
    And it does not export HTTP, TLS, DB, server routing, or UDP functions

  Scenario: Tests do not require external network services
    Given CI runs the #379 test suite
    When JSON and TCP behavior is verified
    Then JSON tests use local strings and fixtures
    And TCP tests use loopback or local test harness services
    And no test requires public internet, third-party credentials, or an
      external service uptime dependency

## Tasks

Implementation should be broken down after Stage 7 and Stage 8 approval. The
following product task groups are independent enough to plan separately:

1. JSON API and type contract:
   Define the `json` graph type, accepted type strings, ownership behavior,
   result shapes, numeric policy, and error text expectations.

2. JSON runtime behavior:
   Implement parse, stringify, field access, array length, array get, allocation,
   cleanup, and recoverable error construction.

3. `@duumbi/stdlib-json` module packaging:
   Add the stdlib graph module, manifest export list, cache/module plumbing, and
   module-facing documentation.

4. TCP resource contract:
   Define `tcp_socket` and `tcp_listener` graph types, lifecycle behavior,
   timeout argument validation, resource cleanup, and closed-resource behavior.

5. TCP runtime behavior:
   Implement connect, listen, accept, read, write, close, listener close,
   timeout handling, OS error conversion, and safe teardown.

6. `@duumbi/stdlib-net` module packaging:
   Add the stdlib graph module, manifest export list, cache/module plumbing, and
   module-facing documentation.

7. Verification:
   Add focused unit, integration, and loopback E2E smoke tests that map back to
   the BDD scenarios without external network dependencies.

8. Review evidence:
   Record the accepted API, boundary decisions, BDD coverage, timeout evidence,
   lifecycle evidence, and default-init/distribution behavior in the
   implementation PR.

9. Tier 1 sequencing:
   Leave registry publication and broad installed-module smoke coverage to #381
   and #382 unless a later approved spec changes sequencing.

## Checks

Stage 6 spec checks:

- Product spec exists at `specs/DUUMBI-379/PRODUCT.md`.
- Spec content is English.
- Spec follows the DUUMBI product spec structure.
- Spec includes BDD scenarios for JSON, TCP, distribution, and test-boundary
  behavior.
- Spec PR changes only `specs/DUUMBI-379/PRODUCT.md`.
- Spec PR title, body, commit message, and spec text use non-closing references
  such as `Related to #379` or `Spec for #379`.
- Spec PR body includes a workflow note that the PR is specification-only and
  must leave the execution issue open.
- Codex self-review finds no blocking product/spec issue.
- Required automated review evidence exists as an actual non-dismissed Copilot
  review submission.
- Checks are green, neutral, skipped, or explicitly not applicable.
- Review threads are resolved, including outdated unresolved threads after
  fixes.
- Greptile is not invoked unless the developer explicitly requests a manual deep
  review.

Implementation checks after later approval:

- Parser/type tests cover `json`, `tcp_socket`, and `tcp_listener` type strings
  or the accepted equivalent representation.
- Graph validation tests reject invalid use of JSON/TCP result/resource values
  where applicable.
- Runtime C compile/link checks pass on supported targets.
- `@duumbi/stdlib-json` graph and manifest export exactly the accepted v1 JSON
  functions.
- `@duumbi/stdlib-net` graph and manifest export exactly the accepted v1 TCP
  functions.
- JSON tests cover valid object parse, invalid parse, missing field, wrong kind,
  array length, array get, invalid index, stringify, nested mixed values,
  `null`, booleans, numbers, strings, and lifecycle cleanup.
- TCP tests cover connect, listen, accept, read, write, close, listener close,
  timeout, invalid arguments, closed-resource behavior, nonrepresentable
  payload behavior, and no external network dependency.
- Tests prove TCP operations cannot hang indefinitely.
- Distribution tests prove default `duumbi init` dependencies remain unchanged
  unless #381 or a later accepted spec changes the policy.
- Documentation or module descriptions explain JSON/TCP usage, error behavior,
  timeout requirements, opt-in network access, and out-of-scope higher-level
  protocols.
- `cargo fmt --check` passes when implementation code exists.
- `cargo clippy --all-targets -- -D warnings` passes when implementation code
  exists.
- `cargo test --all` passes when implementation code exists.

BDD coverage expectations:

- JSON parse/access/stringify scenarios are covered by unit or integration
  tests.
- JSON lifecycle scenario is covered by runtime ownership tests, sanitizer-like
  evidence where available, or repeated-run resource tests accepted by the
  technical spec.
- TCP loopback scenarios are covered by local-only integration or E2E smoke
  tests.
- Timeout scenarios are covered by bounded tests with conservative tolerances.
- Default-init and manifest scenarios are covered by init/cache/module tests.
- External-network exclusion is covered by test design and PR evidence.

## Open Questions

None blocking for product specification.

Stage 8 must still decide technical details that do not change the accepted
product behavior, including:

- exact internal JSON parser dependency or vendored/no-extra-link approach;
- exact JSON numeric precision/storage policy;
- exact runtime layout and ownership implementation for `json`, `tcp_socket`,
  and `tcp_listener`;
- exact cross-platform socket support boundary and linker flags;
- exact test harness shape for deterministic loopback flows.

If any of those technical decisions would change the public API, error contract,
default-init policy, or accepted scope above, Stage 8 must route the change back
for product review instead of silently broadening #379.

## Sources

- GitHub issue #379: `https://github.com/hgahub/duumbi/issues/379`
- Stage 5 acceptance comment for #379:
  `https://github.com/hgahub/duumbi/issues/379#issuecomment-4629134558`
- Stage 4 triage refill comment for #379:
  `https://github.com/hgahub/duumbi/issues/379#issuecomment-4628571024`
- Related issue #378: `https://github.com/hgahub/duumbi/issues/378`
- Related issue #380: `https://github.com/hgahub/duumbi/issues/380`
- Related issue #381: `https://github.com/hgahub/duumbi/issues/381`
- Related issue #382: `https://github.com/hgahub/duumbi/issues/382`
- `docs/architecture.md`
- `docs/coding-conventions.md`
- `docs/automation/code-review-policy.md`
- `.github/workflows/copilot-review.yml`
- `.github/workflows/spec-review-request.yml`
- `stdlib/io.jsonld`
- `stdlib/math.jsonld`
- `stdlib/lang.jsonld`
- `stdlib/string.jsonld`
- `src/cli/init.rs`
- `src/types.rs`
- `src/parser/mod.rs`
- `src/compiler/lowering.rs`
- `runtime/duumbi_runtime.c`
- `runtime/duumbi_runtime.h`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Active Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Development Intake to Delivery Workflow Stage 6 section:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Development Intake to Delivery Workflow.md`
- JSON-LD Graph Representation dot:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/JSON-LD Graph Representation.md`
- GitHub Project as Execution Source of Truth dot:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/GitHub Project as Execution Source of Truth.md`
