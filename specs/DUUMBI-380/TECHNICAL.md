# DUUMBI-380: HTTP/HTTPS and SQLite Stdlib Modules - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-380/PRODUCT.md` by adding
two opt-in Tier 1 integration standard-library modules:

- `@duumbi/stdlib-http` for timeout-bounded HTTP GET, POST, PUT, DELETE,
  response status/body/header access, and response cleanup.
- `@duumbi/stdlib-db` for local SQLite open, execute, query, row access, and
  resource cleanup.

TLS for v1 is an implementation-backed HTTPS behavior of the HTTP module:
certificate verification is enabled by default, untrusted certificates fail
visibly, and no raw TLS socket API is exposed.

The implementation must preserve the approved public contract:

- `http_response`, `db_connection`, and `db_rows` are first-class opaque
  DUUMBI resource types, not user-visible integer handles.
- Recoverable HTTP, TLS, and DB failures return `result<_, string>`.
- HTTP request operations require explicit positive `timeout_ms` arguments.
- HTTP headers are first-class `json` values from #379.
- SQLite file paths are workspace-confined and `:memory:` is the only accepted
  non-file database name.
- HTTP, HTTPS, and DB tests are local, loopback, temporary, and
  timeout-bounded.
- `@duumbi/stdlib-http` and `@duumbi/stdlib-db` are cached/available for
  explicit dependency use, but are not added to default `duumbi init`
  dependencies by this issue.

Technical spec for #380. This specification is non-closing and the execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

Use this spec for:

- Codex implementation agents coordinating Stage 10 Ralph cycles.
- Rust/compiler agents changing parser, type, graph validation, result-safety,
  Cranelift lowering, linker, workspace initialization, and module cache
  surfaces.
- Runtime agents adding C HTTP/TLS and SQLite shims, lifecycle handling, and
  dependency/linker integration.
- Test agents building local-only HTTP, HTTPS, SQLite, and composition
  integration coverage.
- Stage 9 and Stage 11 reviewers checking that implementation evidence maps
  back to the approved product behavior.

Do not use this spec to start implementation during Stage 8 or Stage 9.

## Source Context

Verified source facts:

- Product spec: `specs/DUUMBI-380/PRODUCT.md`.
- Product spec PR: `https://github.com/hgahub/duumbi/pull/670`.
- Product spec merge SHA:
  `5f16fa5ab6ed62b456b54c750579332b5736a498`.
- GitHub issue: `https://github.com/hgahub/duumbi/issues/380`.
- Stage 5 human acceptance:
  `https://github.com/hgahub/duumbi/issues/380#issuecomment-4634633057`.
- Stage 7 product spec approval:
  `https://github.com/hgahub/duumbi/issues/380#issuecomment-4635012538`.
- Current issue labels include `accepted`, `product-spec-approved`, and
  `needs-tech-spec`.
- Repo instructions: `AGENTS.md`.
- Architecture reference: `docs/architecture.md`.
- Coding conventions: `docs/coding-conventions.md`.
- Current `src/types.rs` includes `DuumbiType::Json`, `TcpSocket`, and
  `TcpListener`, but not `HttpResponse`, `DbConnection`, `DbRows`, or
  `TlsSocket`.
- Current `Op` includes DUUMBI-378 file/IO ops and DUUMBI-379 JSON/TCP ops, but
  no HTTP, TLS, or DB ops.
- Current `src/parser/mod.rs` recognizes `json`, `tcp_socket`, and
  `tcp_listener` type strings and parses JSON/TCP built-in ops, but not #380
  resource types or ops.
- Current `src/graph/builder.rs` already converts `OpAst::args` into ordered
  `GraphEdge::Arg(index)` edges; this can represent four-argument #380 ops
  without adding a new AST edge kind.
- Current `src/graph/validator.rs` validates exact result types for direct
  DUUMBI-378 runtime result producers, but not #379 or #380 direct result
  producers.
- Current `src/graph/result_safety.rs` treats DUUMBI-378 direct result
  producers as requiring handling, but not #379 JSON/TCP or #380 ops.
- Current `src/compiler/lowering.rs` declares and lowers DUUMBI-378 and
  DUUMBI-379 runtime functions. Heap/resource-like values are pointer-sized
  `I64` values at the Cranelift boundary.
- Current `src/compiler/linker.rs` compiles `runtime/duumbi_runtime.c` as one C
  source file and links `-lm`; Windows additionally links `-lws2_32` for TCP.
  It does not link curl, TLS, SQLite, or compile multiple runtime C sources.
- Current `runtime/duumbi_runtime.c` has string, array, struct, result, option,
  JSON, TCP, and workspace-confined file helpers. It does not have HTTP/TLS or
  SQLite helpers.
- Current `runtime/duumbi_runtime.h` declares JSON/TCP helpers but no #380
  runtime functions.
- Current `src/cli/init.rs` embeds and caches `stdlib/json.jsonld` and
  `stdlib/net.jsonld` as opt-in modules, while default dependencies remain
  math, io, lang, and string.
- Current `stdlib/` contains `file.jsonld`, `json.jsonld`, and `net.jsonld` but
  no `http.jsonld`, `db.jsonld`, or public TLS module.
- Current `tests/integration_duumbi379.rs` shows dynamic fixture generation,
  loopback TCP tests, bounded waits, and compiled-binary execution patterns.
- Current CI runs Rust checks on Ubuntu and Windows for Rust/runtime/test/Cargo
  changes, and documentation-only checks for spec-only PRs.

Relevant Obsidian context:

- Active PRD: intent, behavior, implementation evidence, and review gates must
  stay connected.
- Active glossary: technical specs are agent-facing implementation
  specifications; Ralph cycles are bounded implementation-and-evidence units.
- Active Agentic Development Map and Runbook: GitHub is the execution source of
  truth; Stage 9 can use a bounded AI gate only when Codex self-review,
  required automated review evidence, checks, scope, and review threads are
  clean.
- Phase 14 roadmap context: HTTP/TLS/DB are Tier 1 integration modules needed
  for realistic demos, while #381 owns publication/default-distribution policy.

Assumptions:

- The implementation may extend the existing explicit runtime-backed op pattern
  used by DUUMBI-378 and DUUMBI-379.
- A blocking C runtime implementation with explicit timeouts is acceptable for
  v1 because the product spec does not require async graph-level APIs.
- `libcurl` is the intended HTTP/HTTPS runtime dependency, using certificate
  verification by default and platform or bundle trust configuration.
- SQLite should be vendored through the SQLite amalgamation to avoid a system
  SQLite development dependency.
- Adding small Rust test-only dependencies for local HTTPS fixtures is
  acceptable when they are limited to tests, pass audit/license checks, and do
  not become graph runtime dependencies.

Implementation recommendations:

- Keep `@duumbi/stdlib-tls` out of the public v1 graph surface. If a packaging
  artifact is later needed for #381, it must not export raw TLS functions.
- Start Stage 10 with a dependency/linker feasibility slice before broad API
  implementation. If `libcurl` cannot be made deterministic on supported CI
  targets without broad platform work, stop for human product/architecture
  approval instead of silently replacing the strategy.

## Affected Areas

Expected Stage 10 source changes:

- Core type model:
  - `src/types.rs`
  - parser/type tests
- JSON-LD parser:
  - `src/parser/mod.rs`
  - parser tests for new type strings and op field mapping
- Graph validation and result handling:
  - `src/graph/validator.rs`
  - `src/graph/result_safety.rs`
  - focused validation/result-safety tests
- Compiler lowering:
  - `src/compiler/lowering.rs`
  - optional helper for ordered operand extraction when four-argument runtime
    calls use `GraphEdge::Arg(0)`
- Runtime linking:
  - `src/compiler/linker.rs`
  - optional runtime build helper module if the current single-source compile
    path becomes too complex
- C runtime:
  - `runtime/duumbi_runtime.c`
  - `runtime/duumbi_runtime.h`
  - vendored SQLite amalgamation under a clearly named runtime-owned path such
    as `runtime/third_party/sqlite/`
  - optional license/notice file required by vendored SQLite source
- Source stdlib graph modules:
  - new `stdlib/http.jsonld`
  - new `stdlib/db.jsonld`
  - source manifests if the implementation stores them beside stdlib modules
- Workspace initialization and module cache:
  - `src/cli/init.rs`
  - init/cache tests proving HTTP and DB are cacheable but not default
    dependencies
- Tests and fixtures:
  - parser/type tests for `http_response`, `db_connection`, and `db_rows`
  - validator tests for exact #380 operand and result contracts
  - result-safety tests for ignored #380 direct result producers
  - runtime/linker tests for libcurl and SQLite linkage
  - HTTP loopback integration tests under `tests/`
  - HTTPS local certificate integration tests under `tests/`
  - SQLite temporary workspace and `:memory:` integration tests under `tests/`
  - composition fixture or dynamic JSON-LD integration test for HTTP + JSON + DB
- Documentation or module descriptions:
  - manifest descriptions or docs/examples needed to describe accepted v1
    functions, error behavior, resource lifecycle, dependency requirements, and
    intentionally deferred features
- CI paths:
  - `.github/workflows/ci.yml` only if a supported-target dependency setup step
    is required and cannot be solved in the linker/runtime build code

Areas that must not change during Stage 10 unless a later approved spec expands
scope:

- `specs/DUUMBI-380/PRODUCT.md`
- `specs/DUUMBI-380/TECHNICAL.md` after Stage 9 approval
- raw TLS socket APIs, including any public `tls_socket` type or
  `tls_connect`, `tls_wrap`, `tls_read`, `tls_write`, or `tls_close` functions
- default `duumbi init` dependencies
- registry publication to `registry.duumbi.dev`
- broad installed-module smoke coverage owned by #382
- provider/model setup flows
- REPL/TUI mutation behavior
- Studio UI behavior
- generated build outputs, committed binaries, runtime logs, credentials,
  public-internet-dependent tests, or shared local databases

## Technical Approach

### 1. Extend Types And Operations Narrowly

Add first-class DUUMBI types:

- `DuumbiType::HttpResponse`
- `DuumbiType::DbConnection`
- `DuumbiType::DbRows`

Do not add `DuumbiType::TlsSocket` in #380.

Parser type strings:

- `http_response`
- `db_connection`
- `db_rows`
- nested result forms such as `result<http_response,string>`,
  `result<db_connection,string>`, and `result<db_rows,string>`

Type behavior:

- `fmt::Display` must round-trip the accepted type strings.
- `is_heap_type()` must return true for all three new resource types.
- `duumbi_type_to_cl()` must map all three to pointer-sized `types::I64`.
- `type_size()` must treat all three as pointer-sized runtime handles.
- Existing result/option semantics remain unchanged.

Add operation variants:

- HTTP:
  - `HttpGet`
  - `HttpPost`
  - `HttpPut`
  - `HttpDelete`
  - `HttpStatus`
  - `HttpBody`
  - `HttpHeaders`
  - `HttpResponseFree`
- DB:
  - `DbOpen`
  - `DbExecute`
  - `DbQuery`
  - `DbRowsLen`
  - `DbRowGet`
  - `DbClose`
  - `DbRowsFree`

Do not add raw TLS operation variants.

JSON-LD operation names:

- `duumbi:HttpGet`
- `duumbi:HttpPost`
- `duumbi:HttpPut`
- `duumbi:HttpDelete`
- `duumbi:HttpStatus`
- `duumbi:HttpBody`
- `duumbi:HttpHeaders`
- `duumbi:HttpResponseFree`
- `duumbi:DbOpen`
- `duumbi:DbExecute`
- `duumbi:DbQuery`
- `duumbi:DbRowsLen`
- `duumbi:DbRowGet`
- `duumbi:DbClose`
- `duumbi:DbRowsFree`

Reference field mapping:

| Operation | `operand` | `left` | `right` | `args` |
| --- | --- | --- | --- | --- |
| `HttpGet` | url string | headers json | timeout_ms i64 | none |
| `HttpDelete` | url string | headers json | timeout_ms i64 | none |
| `HttpPost` | url string | headers json | body string | timeout_ms as `args[0]` |
| `HttpPut` | url string | headers json | body string | timeout_ms as `args[0]` |
| `HttpStatus` | http_response | none | none | none |
| `HttpBody` | http_response | none | none | none |
| `HttpHeaders` | http_response | none | none | none |
| `HttpResponseFree` | http_response | none | none | none |
| `DbOpen` | path string | none | none | none |
| `DbExecute` | db_connection | sql string | params array<string> | none |
| `DbQuery` | db_connection | sql string | params array<string> | none |
| `DbRowsLen` | db_rows | none | none | none |
| `DbRowGet` | db_rows | row_index i64 | column string | none |
| `DbClose` | db_connection | none | none | none |
| `DbRowsFree` | db_rows | none | none | none |

The parser should require the fields above. Missing operand fields should use
existing missing-field/schema diagnostics.

### 2. Validate Operand And Result Contracts

Add validator checks for exact operand and result types:

- `HttpGet`, `HttpDelete`:
  - `url: string`
  - `headers: json`
  - `timeout_ms: i64`
  - return `result<http_response,string>`
- `HttpPost`, `HttpPut`:
  - `url: string`
  - `headers: json`
  - `body: string`
  - `timeout_ms: i64`
  - return `result<http_response,string>`
- `HttpStatus`: `http_response -> result<i64,string>`
- `HttpBody`: `http_response -> result<string,string>`
- `HttpHeaders`: `http_response -> result<json,string>`
- `HttpResponseFree`: `http_response -> result<i64,string>`
- `DbOpen`: `string -> result<db_connection,string>`
- `DbExecute`: `db_connection, string, array<string> -> result<i64,string>`
- `DbQuery`: `db_connection, string, array<string> -> result<db_rows,string>`
- `DbRowsLen`: `db_rows -> result<i64,string>`
- `DbRowGet`: `db_rows, i64, string -> result<string,string>`
- `DbClose`: `db_connection -> result<i64,string>`
- `DbRowsFree`: `db_rows -> result<i64,string>`

Update result-safety handling so every direct #380 result producer must be
handled by `ResultIsOk`, `Match`, `ResultUnwrap`, or equivalent existing
result-handling behavior in the same block.

Recommendation: also include DUUMBI-379 JSON/TCP direct result producers in
result-safety coverage while touching this code, because #380 composition
depends on JSON result handling. This is adjacent validation correctness, not a
public API expansion.

### 3. Lower To Runtime Calls

Declare runtime functions in `declare_all_runtime_fns()` and import them in
`compile_function()`.

Recommended runtime symbols:

HTTP public-result symbols:

- `duumbi_http_get(i64 url, i64 headers, i64 timeout_ms) -> i64`
- `duumbi_http_post(i64 url, i64 headers, i64 body, i64 timeout_ms) -> i64`
- `duumbi_http_put(i64 url, i64 headers, i64 body, i64 timeout_ms) -> i64`
- `duumbi_http_delete(i64 url, i64 headers, i64 timeout_ms) -> i64`
- `duumbi_http_status(i64 response) -> i64`
- `duumbi_http_body(i64 response) -> i64`
- `duumbi_http_headers(i64 response) -> i64`
- `duumbi_http_response_close(i64 response) -> i64`

DB public-result symbols:

- `duumbi_db_open(i64 path) -> i64`
- `duumbi_db_execute(i64 conn, i64 sql, i64 params) -> i64`
- `duumbi_db_query(i64 conn, i64 sql, i64 params) -> i64`
- `duumbi_db_rows_len(i64 rows) -> i64`
- `duumbi_db_row_get(i64 rows, i64 row_index, i64 column) -> i64`
- `duumbi_db_close(i64 conn) -> i64`
- `duumbi_db_rows_close(i64 rows) -> i64`

Automatic resource free symbols for `Drop`/scope-exit cleanup:

- `duumbi_http_response_free(i64 response) -> void`
- `duumbi_db_connection_free(i64 conn) -> void`
- `duumbi_db_rows_free(i64 rows) -> void`

Lowering must:

- pass ordered operands using existing `GraphEdge` sources;
- use ordered `Arg(0)` for POST/PUT `timeout_ms`;
- store returned result/resource values in `value_map`;
- add the new resource types to heap/resource auto-cleanup where the compiler
  tracks heap allocations;
- keep explicit public cleanup operations (`HttpResponseFree`, `DbClose`,
  `DbRowsFree`) separate from automatic resource wrapper free calls. The
  graph-level function names remain the approved `http_response_free` and
  `db_rows_free`, but runtime symbols should use `*_close` for
  result-returning public cleanup and reserve `*_free` for void Drop hooks.

### 4. HTTP/TLS Runtime Strategy

Use `libcurl` through the C runtime for HTTP and HTTPS.

Reasons:

- `libcurl` provides mature URL parsing, HTTP methods, timeout support,
  redirects-off behavior, response headers, response body buffering, and TLS
  certificate verification without exposing raw TLS graph APIs.
- It keeps #380 in the existing C runtime ABI instead of introducing a Rust
  staticlib bridge or a second async runtime boundary.
- The product spec permits system libraries when requirements and linker
  consequences are documented.

Runtime behavior:

- Support `http://` and `https://` schemes only.
- Disable automatic redirect following.
- Require `timeout_ms > 0`; invalid timeout returns `Err(string)`.
- Treat completed HTTP responses, including 3xx, 4xx, and 5xx, as
  `Ok(http_response)`.
- Treat malformed URL, unsupported scheme, invalid header JSON, non-string
  header values, DNS failure, connect failure, timeout, TLS/certificate
  failure, protocol failure, allocation failure, and invalid/released response
  access as `Err(string)`.
- Materialize the full response body and headers before returning
  `Ok(http_response)`; no v1 streaming.
- Bound materialization before allocation can grow unbounded. Initial internal
  caps:
  - response body: `8 MiB`;
  - aggregate response header bytes: `64 KiB`;
  - response header count: `256`.
  If a response exceeds these caps, the request returns `Err(string)` with
  `http_body_too_large` or `http_headers_too_large`. These are internal v1
  safety limits, not public API parameters.
- Accept UTF-8 response bodies only. Non-UTF-8 or otherwise
  nonrepresentable bodies return `Err(string)` from `http_body`.
- Response headers returned by `http_headers` must be a new JSON object. Header
  names are lowercased. Duplicate response header names are joined in arrival
  order using `, ` as the separator. Values must be UTF-8 strings; invalid
  header bytes return `Err(string)`.
- Request header JSON must be an object whose keys are non-empty HTTP token
  names and whose values are strings. Request header duplicates cannot be
  represented in JSON object form.
- Error strings should include stable class tokens such as `http_url`,
  `http_headers`, `http_timeout`, `http_tls`, `http_connect`,
  `http_response`, or `http_resource` so tests do not depend on full platform
  wording.
- Runtime errors must not log or include authorization headers, request bodies,
  response bodies, or other payload data.

TLS behavior:

- `CURLOPT_SSL_VERIFYPEER` and hostname verification must remain enabled for
  HTTPS requests.
- No graph API may disable verification.
- The local HTTPS success test may configure trust from the test harness by
  setting a process environment variable such as `CURL_CA_BUNDLE` or
  `SSL_CERT_FILE` for the compiled test binary. This is allowed because it is
  test-harness trust configuration, not a public DUUMBI graph API.
- The untrusted-certificate test must run without the test CA bundle and assert
  `Err(string)` with a TLS/certificate error class.

Dependency and linker requirements:

- Stage 10 Cycle 1 must prove compile/link behavior for `libcurl` and SQLite on
  the supported CI targets before large API implementation proceeds.
- Prefer deterministic discovery through `curl-config`, `pkg-config`, or
  documented platform flags. Do not silently skip HTTPS on a target.
- If `libcurl` headers/libraries are unavailable on a supported CI target and
  cannot be added through a narrow, auditable CI/setup change, stop for human
  approval. A replacement TLS stack, Rust staticlib bridge, or dropped target is
  a product/architecture decision.

Rejected alternatives:

- Do not implement HTTPS by hand over TCP in #380.
- Do not expose raw TLS sockets to satisfy HTTPS tests.
- Do not make public internet requests in default CI.
- Do not use Rust `reqwest` directly from generated graph programs unless a
  later architecture decision approves a Rust runtime bridge.

### 5. SQLite Runtime Strategy

Use the SQLite amalgamation vendored under a runtime-owned third-party path and
compile it as part of the runtime build.

Runtime behavior:

- `db_open(":memory:")` opens a local in-memory SQLite database.
- File-backed `db_open(path)` validates the path with the #378
  workspace-confined policy before calling SQLite.
- Absolute paths, traversal, home expansion, environment expansion,
  URL-like paths, platform drive prefixes, and missing/invalid parents return
  `Err(string)`.
- File-backed DB open must not create missing parent directories.
- `db_open` sets a bounded internal SQLite busy timeout, recommended
  `1000 ms`, because v1 has no public DB timeout parameter.
- `db_execute` uses `sqlite3_prepare_v2`, binds `array<string>` parameters by
  index with `SQLITE_TRANSIENT`, steps until completion, finalizes the
  statement, and returns SQLite's changed-row count. If rows are produced, return
  `Err(string)` telling the user to use `db_query`.
- `db_query` prepares and binds parameters, materializes the complete row set
  into a `db_rows` resource, finalizes the statement, and returns
  `Ok(db_rows)`.
- Bound row materialization before allocation can grow unbounded. Initial
  internal caps:
  - rows: `10000`;
  - aggregate copied cell bytes: `8 MiB`;
  - columns per result set: `256`.
  If a result exceeds these caps, `db_query` returns `Err(string)` with
  `db_rows_limit`. These are internal v1 safety limits, not public API
  parameters.
- `db_rows` stores copied column names and values so row resources are
  independent of the connection after query materialization.
- `db_row_get` accepts zero-based row indexes and exact column names.
- SQL NULL through `db_row_get` returns `Err(string)` with a stable
  nonrepresentable/null error class.
- SQLite integers, floats, and text may be returned through
  `sqlite3_column_text` conversion, copied as UTF-8 DUUMBI strings. BLOB or
  invalid UTF-8 values return `Err(string)`.
- `db_close` marks the connection closed and closes the SQLite handle, but keeps
  a small resource wrapper valid until automatic cleanup so later same-scope
  operations can return `Err(string)` instead of dereferencing freed memory.
- `db_rows_free` marks the row resource released and frees row contents, but
  keeps a small resource wrapper valid until automatic cleanup so later same
  scope row operations can return `Err(string)`.
- Automatic free functions must be idempotent and must not double-free after
  explicit public cleanup operations.
- Error strings should include stable class tokens such as `db_path`,
  `db_open`, `db_sql`, `db_params`, `db_row`, `db_null`, `db_resource`, or
  `db_busy`.
- DB errors must not log SQL parameters, row values, database contents, or file
  contents.

Rejected alternatives:

- Do not use a system SQLite dependency for default builds.
- Do not add database connection pools, migrations, ORM helpers, statement
  handles, transactions helpers, typed extraction, nullable helpers, blobs, or
  remote database clients in #380.

### 6. Stdlib Module Shape

Add `stdlib/http.jsonld` with exports exactly:

- `http_get`
- `http_post`
- `http_put`
- `http_delete`
- `http_status`
- `http_body`
- `http_headers`
- `http_response_free`

Add `stdlib/db.jsonld` with exports exactly:

- `db_open`
- `db_execute`
- `db_query`
- `db_rows_len`
- `db_row_get`
- `db_close`
- `db_rows_free`

Each function should be a thin graph wrapper around the matching built-in op,
following the style of `stdlib/json.jsonld`, `stdlib/net.jsonld`, and
`stdlib/file.jsonld`.

Update init/cache behavior:

- cache `@duumbi/stdlib-http` and `@duumbi/stdlib-db` in `.duumbi/cache`;
- include manifest/export metadata for both modules;
- do not add either module to default `[dependencies]`;
- do not add a public `@duumbi/stdlib-tls` graph module in #380.

### 7. Cross-Platform Runtime Build

Change runtime compile/linking as narrowly as possible:

- Continue to compile generated Cranelift object files and the DUUMBI runtime
  through the existing linker path.
- If SQLite is a separate C source, extend runtime compilation to compile
  multiple C sources into objects before linking.
- Keep platform-specific flags centralized in `src/compiler/linker.rs` or a
  small helper owned by the compiler/linker layer.
- Document the final platform requirements in implementation PR evidence.
- Add tests or build checks that fail with a clear `E008`-class message when a
  required C dependency cannot be found.

## Invariants

- No #380 public API exposes raw TLS sockets, TLS verification bypass, integer
  handles, raw pointers, curl handles, SQLite pointers, or raw file descriptors.
- HTTP requests do not execute during import, parsing, graph validation,
  compilation, package inspection, or workspace initialization.
- SQLite connections do not open during import, parsing, graph validation,
  compilation, package inspection, or workspace initialization.
- All HTTP request functions require positive explicit timeouts.
- HTTPS certificate verification is enabled by default.
- Redirects are returned as ordinary responses; the client does not
  automatically follow them.
- HTTP non-2xx statuses are inspectable response states, not transport errors.
- `http_response`, `db_connection`, and `db_rows` access after public cleanup
  returns `Err(string)` when recoverable and never crashes.
- DB file paths remain workspace-confined; `:memory:` is the only v1 exception.
- DB parameters are bound through prepared statements, not interpolated by the
  stdlib.
- SQL NULL is not silently converted to an empty string.
- Default new workspace dependencies remain unchanged by #380.
- Tests must not require public internet, third-party services, credentials,
  production databases, shared local state, or ambient machine-specific ports.
- Error messages must avoid secrets, request/response bodies, SQL parameters,
  SQL result values, and database contents.
- Greptile remains manual-only and is not part of the default Stage 8/9 gate.

## BDD-To-Test Mapping

| Product BDD scenario | Verification evidence |
| --- | --- |
| GET returns status, headers, and body from a loopback service | `tests/integration_duumbi380_http.rs` starts a loopback HTTP service, compiles a fixture using `HttpGet`, `HttpStatus`, `HttpHeaders`, and `HttpBody`, and asserts status `200`, lowercased `content-type`, and exact body text. |
| Non-2xx responses remain inspectable | HTTP integration test returns `404`; fixture proves `HttpGet` is `Ok`, `HttpStatus` is `Ok(404)`, and body remains readable. |
| POST sends headers and body | HTTP integration test records request headers/body; fixture uses `HttpPost`; test asserts the server received the content-type header and exact JSON body. |
| Invalid header JSON is a recoverable error | Validator/runtime test or integration fixture passes non-object JSON headers and asserts `ResultIsOk` prints false or unwraps an error containing `http_headers`. |
| Request timeout returns an error instead of hanging | Loopback service delays beyond timeout; test asserts the compiled binary exits within a bounded tolerance and reports `Err` with `http_timeout`. |
| Redirects are returned as ordinary responses | Loopback service returns `302` with `Location`; test asserts one request was received, status is `302`, headers expose `location`, and no second request occurs. |
| Response resources can be released safely | Integration or runtime test calls `http_response_free`, then `http_status`; asserts first returns `Ok(0)`, later access is `Err`, and process exits successfully. |
| Verified HTTPS succeeds under the accepted local test harness | HTTPS integration test starts a local TLS server with a test CA trusted through child-process environment, compiles/runs fixture against `https://127.0.0.1:<port>`, and asserts `Ok(http_response)`, status, and body. |
| Untrusted certificates fail visibly | Same HTTPS harness without the trust bundle; fixture asserts `HttpGet` returns `Err` with `http_tls` or certificate error class and no public bypass API is used. |
| Raw TLS socket APIs are not exported in v1 | Manifest/stdlib test parses `stdlib/http.jsonld`, `stdlib/db.jsonld`, and any generated manifests; asserts accepted exports exist and no export includes `tls_connect`, `tls_wrap`, `tls_read`, `tls_write`, or `tls_close`. |
| Open an in-memory database, insert a row, and query it | `tests/integration_duumbi380_db.rs` compiles a fixture using `DbOpen(":memory:")`, create/insert/query, `DbRowsLen`, and `DbRowGet`; asserts row count and `Ada`. |
| File-backed databases stay inside the workspace | Workspace integration test runs compiled fixture with `DUUMBI_WORKSPACE_ROOT`; `data/demo.sqlite` succeeds when `data/` exists and `../outside.sqlite` returns `Err` with `db_path`. |
| SQL parameters are bound instead of interpolated | DB integration test inserts `Ada'); DROP TABLE users; --` as a parameter, then queries table existence/value; asserts table still exists and stored value is literal text. |
| Querying an empty result set is successful but row access fails | DB fixture performs empty query; asserts `DbQuery` is `Ok`, `DbRowsLen` is `Ok(0)`, and `DbRowGet(0, ...)` is `Err`. |
| Missing columns are recoverable errors | DB fixture queries a row and calls `DbRowGet(..., "missing")`; asserts `Err` and successful process exit after branching. |
| SQL NULL is not confused with empty string | DB fixture inserts/selects NULL and calls `DbRowGet`; asserts `Err` with `db_null` or nonrepresentable error class. |
| Closed database connections reject later operations safely | DB fixture calls `DbClose`, then `DbQuery` on the same resource; asserts `Ok(0)`, then `Err`, and no crash. |
| Row resources can be released safely | DB fixture calls `DbRowsFree`, then `DbRowsLen`; asserts `Ok(0)`, then `Err`, and no crash. |
| Fetch local JSON and persist a transformed value | `tests/integration_duumbi380_e2e.rs` starts loopback HTTP returning JSON, compiles a fixture using HTTP + `@duumbi/stdlib-json` + DB, persists a transformed value into in-memory or temp SQLite, and asserts queried output. |
| New workspaces keep the current default stdlib set | `src/cli/init.rs` tests assert HTTP and DB modules are cached/manifested when accepted, but default `[dependencies]` exclude `@duumbi/stdlib-http` and `@duumbi/stdlib-db`. |
| Tests do not require external services | PR evidence plus tests show loopback HTTP, local HTTPS fixture, temp/in-memory SQLite, no public URLs, no credentials, and bounded timeouts. CI must pass without internet-dependent service calls. |

Additional required coverage:

- Parser/type tests for new type strings and nested `result<_,string>` forms.
- Parser tests for all #380 op field mappings, including POST/PUT
  `duumbi:args[0]` timeout.
- Validator tests for exact operand/result type errors.
- Result-safety tests proving ignored #380 direct result producers are rejected.
- HTTP overflow tests proving body/header materialization caps return
  recoverable `Err(string)` values instead of exhausting memory.
- DB overflow tests proving row/byte materialization caps return recoverable
  `Err(string)` values instead of exhausting memory.
- Linker/runtime tests proving missing dependency errors are clear and supported
  CI targets link successfully.

## Live E2E Plan

Canonical interface: DUUMBI CLI build/run of JSON-LD graph programs, because
#380 changes compiled graph runtime behavior rather than UI or provider
behavior.

Provider/LLM path:

- No DUUMBI provider or LLM call is required.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Codex internal reasoning usage should be reported qualitatively only; it is
  not part of the DUUMBI external LLM budget.

Credentials and environment:

- No API keys or public network credentials.
- `DUUMBI_WORKSPACE_ROOT` is required for file-backed DB tests and should be
  set by workspace-run helpers where applicable.
- Local HTTPS success tests may set `CURL_CA_BUNDLE` or `SSL_CERT_FILE` for the
  child process to trust the test CA.
- Tests choose free loopback ports dynamically and synchronize startup with
  bounded retries.

Commands and checks:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`
- Targeted during development:
  - `cargo test integration_duumbi380_http`
  - `cargo test integration_duumbi380_db`
  - `cargo test integration_duumbi380_e2e`
  - focused parser/validator/result-safety tests
- Manual smoke when implementation exists:
  - build a DUUMBI fixture that fetches loopback JSON, parses it, inserts into
    SQLite, queries it, and prints the expected transformed value;
  - run the compiled binary with only loopback services and temporary workspace
    state;
  - record command, exit status, stdout/stderr summary, and temp fixture path in
    the implementation PR evidence.

Pass/fail criteria:

- All fallible graph operations in the live E2E fixture return `Ok(_)`.
- The final query output contains the expected transformed value.
- HTTPS success and failure are both proven through local fixtures.
- No default test path reaches public internet or production services.
- CI is green on supported targets.

TUI/Studio parity:

- No full TUI or Studio E2E is required because #380 does not change UI
  behavior.
- If docs/examples expose these modules through CLI-visible help or module
  discovery, perform a thin CLI/module-cache smoke only.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. summarize current state and remaining unmet product/technical requirements;
2. propose one bounded implementation goal;
3. list intended file areas and commands before editing;
4. estimate external LLM calls, estimated cost, dependency risk, platform risk,
   and irreversible-operation risk;
5. check whether the resource gate requires human approval;
6. implement only the approved or resource-permitted goal;
7. run the agreed checks for that slice;
8. report evidence, failures, changed files, and remaining gaps;
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, risky dependency or migration work appears, a
   security/product/architecture decision is needed, or the autonomous batch cap
   is reached.

Recommended first cycles:

1. Dependency/linker feasibility and minimal runtime build proof for libcurl
   and vendored SQLite.
2. Type/parser/validator/result-safety contract for HTTP and DB.
3. Runtime HTTP/TLS response lifecycle and local HTTP/HTTPS tests.
4. Runtime SQLite lifecycle and DB tests.
5. Stdlib module/cache/docs, composition E2E, and full verification.

The implementation coordinator may split these further when risk is lower or
when checks expose a narrower failure.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle:
  - Dependency/linker proof: up to 4 files/modules.
  - Type/parser/validator/result-safety slice: up to 5 files/modules.
  - Runtime HTTP/TLS slice: up to 6 files/modules plus generated temporary test
    fixtures.
  - Runtime DB slice: up to 6 files/modules plus vendored SQLite source and
    license/notice file.
  - Stdlib/cache/composition slice: up to 6 files/modules.
- Expected command budget per cycle:
  - targeted `cargo test` for changed area;
  - `cargo fmt --check` when Rust files change;
  - `cargo clippy --all-targets -- -D warnings` and `cargo test --all` before
    marking an implementation PR ready for Stage 11.
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Human approval is required before continuing when planned or observed work:
  - exceeds USD 2 external LLM cost;
  - exceeds 10 external LLM calls;
  - exceeds approved product or technical scope;
  - introduces risky dependencies beyond `libcurl`, vendored SQLite
    amalgamation, and narrow test-only HTTPS fixture dependencies;
  - requires a migration, irreversible operation, release/distribution change,
    or registry publication;
  - changes security posture, certificate verification defaults, workspace path
    policy, or supported-target policy;
  - requires a product/architecture decision;
  - cannot make `libcurl`/SQLite link reliably on supported CI targets;
  - hits a blocker that prevents meaningful progress inside the approved plan.
- Autonomous batch cap: three consecutive low-budget cycles in one Stage 10 run.
- Stop and ask for human guidance when the dependency strategy, public API,
  verification harness, supported platform list, security boundary, or
  implementation scope would materially change.

## Task Breakdown

1. Prove dependency and linker feasibility:
   - choose exact `libcurl` discovery/link flags;
   - vendor SQLite amalgamation and license/notice if not already present;
   - extend runtime compilation for additional C source if needed;
   - prove supported CI targets can compile/link or stop for approval.
2. Add type and parser contract:
   - add new resource types;
   - parse type strings and display round-trips;
   - add HTTP/DB ops and field mappings;
   - add parser tests.
3. Add validator and result-safety rules:
   - exact operand/result checks for all #380 ops;
   - direct result handling checks for HTTP/DB and, if practical, #379 JSON/TCP;
   - focused negative and positive tests.
4. Add compiler lowering:
   - declare runtime symbols;
   - lower each #380 op;
   - dispatch automatic resource cleanup correctly;
   - add lowering/runtime declaration tests where existing patterns support it.
5. Implement HTTP/TLS runtime:
   - materialized response resource;
   - request header JSON validation;
   - lowercased deterministic response headers;
   - status/body/header accessors;
   - explicit release and automatic free;
   - timeout, redirect, non-2xx, TLS verification, and error class behavior.
6. Implement SQLite runtime:
   - connection and rows resources;
   - workspace path handling and `:memory:`;
   - prepared statement parameter binding;
   - execute/query materialization;
   - row access, NULL handling, cleanup, and closed/released resource errors.
7. Add stdlib modules and cache behavior:
   - `stdlib/http.jsonld`;
   - `stdlib/db.jsonld`;
   - manifests/export metadata;
   - `src/cli/init.rs` cache seeding without default dependency changes.
8. Add local integration and E2E tests:
   - HTTP loopback;
   - HTTPS trusted/untrusted local cert harness;
   - SQLite in-memory and workspace file tests;
   - composition HTTP + JSON + DB fixture;
   - default-init distribution tests.
9. Final verification and evidence:
   - run required full checks;
   - document dependency strategy, supported targets, local E2E evidence,
     BDD coverage, and remaining residual risk in the implementation PR.

## Verification Plan

Required automated verification:

- Parser/type unit tests for `http_response`, `db_connection`, `db_rows`, and
  nested result strings.
- Parser tests for all new op names and field mappings.
- Validator tests for each exact #380 result contract and representative
  operand type mismatches.
- Result-safety tests for ignored direct result producers and handled direct
  result producers.
- Runtime/linker tests for successful supported-target compilation and clear
  missing dependency failures.
- HTTP integration tests for GET, POST, PUT, DELETE, status, body, headers,
  non-2xx, redirect, invalid header JSON, invalid timeout, timeout, malformed
  URL, unsupported scheme, cleanup, released-resource access, and
  nonrepresentable body and body/header cap behavior where practical.
- HTTPS integration tests for trusted local harness success and untrusted local
  certificate failure.
- DB integration tests for `:memory:`, workspace file path success, path escape
  rejection, missing parent rejection, create/insert/query, changed-row count,
  parameter binding, empty results, missing column, invalid row index, SQL NULL,
  SQL error, row/byte cap overflow, close, rows free, and
  released/closed-resource access.
- Composition integration test for loopback API fetch -> JSON parse -> DUUMBI
  transform -> SQLite write/query.
- Init/cache tests proving `@duumbi/stdlib-http` and `@duumbi/stdlib-db` are
  not default dependencies.

Required commands before implementation PR review:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`

Required PR evidence:

- Changed-file summary showing no product spec or technical spec edits after
  Stage 9 approval.
- Dependency/linker summary naming exact HTTP/TLS/SQLite strategy.
- BDD coverage matrix or checklist linking tests to product scenarios.
- Live E2E command and result summary.
- Statement that tests use only loopback/local/temp resources.
- Statement that raw TLS exports are absent.
- Statement that default `duumbi init` dependencies remain unchanged.
- Codex self-review and required automated reviewer evidence for the
  implementation PR as required by Stage 11 policy.

## Completion Criteria

Implementation is complete only when:

- `@duumbi/stdlib-http` exports exactly the accepted HTTP v1 functions.
- `@duumbi/stdlib-db` exports exactly the accepted DB v1 functions.
- No public TLS socket type or raw TLS function is exposed.
- `http_response`, `db_connection`, and `db_rows` are first-class opaque
  resource types.
- All #380 public runtime functions return the accepted `result<_, string>`
  shapes.
- HTTP request functions validate positive timeouts.
- HTTP non-2xx and redirect responses are inspectable responses.
- HTTPS certificate verification is enabled by default and failure is visible.
- HTTP header canonicalization and duplicate handling are deterministic and
  documented.
- HTTP response body/header buffering is bounded and cap overflow returns
  `Err(string)`.
- SQLite paths are workspace-confined with `:memory:` accepted.
- SQLite parameters are bound through prepared statements.
- NULL and unsupported row values are visible errors.
- SQLite query row materialization is bounded and cap overflow returns
  `Err(string)`.
- Explicit cleanup and later access behavior are safe and tested.
- Default `duumbi init` dependencies do not include HTTP or DB.
- All product BDD scenarios have automated tests or explicitly named review
  evidence.
- Local live E2E passes without public internet or credentials.
- Required full checks pass on supported CI targets.
- Implementation PR evidence states residual dependency/platform risks.

## Failure And Escalation

Stop and request human guidance when:

- `libcurl` cannot be linked on a supported CI target through a narrow,
  documented build path.
- The implementation would need to expose a public certificate bypass, custom
  trust API, raw TLS socket API, or broader TLS configuration to pass tests.
- A replacement HTTP/TLS strategy is needed, such as OpenSSL/wolfSSL direct use
  or a Rust staticlib bridge.
- Vendored SQLite introduces license, audit, build, or platform concerns beyond
  the approved plan.
- Runtime/resource safety cannot be implemented without changing result,
  ownership, or Drop semantics beyond #380.
- The local HTTPS harness cannot prove both trusted and untrusted certificate
  behavior without public internet or insecure public APIs.
- DB path policy requirements conflict with existing #378 workspace path
  behavior.
- CI failures indicate platform support must be narrowed.
- Any external LLM usage would exceed USD 2 or 10 calls.
- Any migration, registry publication, default distribution change, or
  irreversible operation becomes necessary.

When tests fail inside scope:

- narrow to the smallest failing slice;
- add or adjust focused tests before broadening implementation;
- keep error strings stable by class token rather than full platform message;
- report failing command, failure class, likely cause, and next bounded cycle
  proposal.

When requirements conflict:

- preserve the approved product spec over implementation convenience;
- do not silently broaden public API or security boundaries;
- route back to Stage 8/9 or request human product/architecture approval as
  appropriate.

## Open Questions

None blocking for Stage 10 implementation under this technical spec.

Accepted implementation risks:

- `libcurl` platform availability must be proven before broad implementation.
  If that proof fails, the dependency strategy becomes a human
  product/architecture decision.
- The local HTTPS trust harness may use child-process environment trust
  configuration for tests. This must remain outside the public DUUMBI graph API.
- SQLite numeric-to-string formatting follows SQLite text conversion for v1 and
  should be tested only for deterministic representative values, not advertised
  as exact decimal preservation.
