# DUUMBI-379: @duumbi/stdlib-json And @duumbi/stdlib-net Modules - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-379/PRODUCT.md` by adding
two opt-in Tier 1 standard library modules:

- `@duumbi/stdlib-json` for first-class JSON parsing, stringifying, object field
  access, array length, and array item access.
- `@duumbi/stdlib-net` for first-class bounded TCP connect, listen, accept,
  read, write, socket close, and listener close operations.

The implementation must preserve the approved public contract:

- JSON values, sockets, and listeners are first-class DUUMBI heap/resource types,
  not user-visible raw integer handles.
- Recoverable JSON and TCP failures return `result<_, string>`.
- Every blocking TCP operation has an explicit positive `timeout_ms` argument.
- JSON/TCP modules are cached by `duumbi init` but are not added to default
  workspace dependencies.
- Tests use local strings, fixtures, and loopback networking only.

Technical spec for #379. This specification is non-closing and the execution
issue must remain open for Stage 9 review, Stage 10 implementation, Stage 11
review, and Stage 12 closure.

## Agent Audience

Use this spec for:

- Codex implementation agents coordinating Stage 10 Ralph cycles.
- Rust/compiler agents changing parser, type, graph validation, Cranelift
  lowering, linker, and workspace initialization surfaces.
- Runtime agents adding C JSON/TCP shims and lifecycle handling.
- Test agents building local-only JSON and TCP integration coverage.
- Stage 9 and Stage 11 reviewers checking that implementation evidence maps
  back to the approved product behavior.

Do not use this spec to start implementation during Stage 8 or Stage 9.

## Source Context

- Product spec: `specs/DUUMBI-379/PRODUCT.md`.
- Product spec PR: `https://github.com/hgahub/duumbi/pull/662`.
- Product spec merge SHA:
  `5c2f973f6fc0c3a49b9b5d9d9b480c63cf2ee0bd`.
- GitHub issue: `https://github.com/hgahub/duumbi/issues/379`.
- Stage 7 product spec approval:
  `https://github.com/hgahub/duumbi/issues/379#issuecomment-4631004765`.
- Stage 5 human acceptance:
  `https://github.com/hgahub/duumbi/issues/379#issuecomment-4629134558`.
- Repo instructions: `AGENTS.md`.
- Architecture reference: `docs/architecture.md`.
- Coding conventions: `docs/coding-conventions.md`.

Relevant code verified for Stage 8:

- `src/types.rs`
  - `DuumbiType` currently includes primitive types, `String`, `Array`,
    `Struct`, references, `Result`, and `Option`.
  - `DuumbiType::is_heap_type()` currently treats string, array, struct,
    result, and option values as heap/runtime-managed values.
  - `Op` has no JSON or TCP operations today.
- `src/parser/mod.rs`
  - `parse_type_str()` recognizes primitives, arrays, structs, results,
    options, and references.
  - `parse_op()` maps `duumbi:*` operation strings into `Op` variants.
- `src/parser/ast.rs`
  - `OpAst` already has `operand`, `left`, `right`, and `args` reference slots.
    New built-in JSON/TCP operations can use those fields without changing the
    JSON-LD AST shape.
- `src/compiler/lowering.rs`
  - `declare_all_runtime_fns()` declares all C runtime functions available to
    generated code.
  - `duumbi_type_to_cl()` maps heap/result/option types to pointer-sized
    Cranelift values.
  - `compile_function()` imports runtime functions once per generated function.
  - Existing `Drop` handling dispatches heap frees for strings, arrays,
    structs, results, and options.
  - `type_size()` maps heap-like values to pointer size.
- `src/compiler/linker.rs`
  - `compile_runtime()` currently compiles `runtime/duumbi_runtime.c` as one C
    source file.
  - Platform linker args currently include math linkage and warning stripping,
    but not Windows socket linkage.
- `runtime/duumbi_runtime.c`
  - Existing runtime support includes allocation, strings, arrays, structs,
    results, options, math, telemetry, and string helpers.
  - `DuumbiString` tracks length and stores a null terminator for C interop.
  - `DuumbiResult` stores a tag and `int64_t` payload. Result free currently
    frees the result container; payload lifecycle is handled by graph ownership
    and explicit drops.
- `runtime/duumbi_runtime.h`
  - Declares print, allocation, string, array, and struct functions.
  - It does not yet declare result, option, JSON, or TCP runtime functions.
- `src/cli/init.rs`
  - Embeds `stdlib/math.jsonld`, `stdlib/io.jsonld`, `stdlib/lang.jsonld`, and
    `stdlib/string.jsonld`.
  - Writes each embedded stdlib module and manifest into the M5 cache layout.
  - Default dependencies currently include only math, io, lang, and string.
- `stdlib/string.jsonld`
  - Provides the graph-module pattern for stdlib wrappers around built-in ops.
- `tests/integration_phase9a1.rs`
  - Shows fixture build/run patterns for runtime-backed features.
- `tests/integration_phase9a3.rs`
  - Shows direct parser, graph, lowering, result, and option test patterns.
- `tests/integration_phase7_local.rs`
  - Shows workspace cache and local dependency resolution test patterns.
- `.github/workflows/ci.yml`
  - Spec-only changes skip Rust checks.
  - Implementation changes to Rust, runtime, tests, Cargo, or CI run Rust
    formatting, clippy, build, tests, and aggregate checks.

Verified source facts:

- There is no current `json`, `tcp_socket`, or `tcp_listener` type string.
- There are no current JSON/TCP built-in ops or runtime symbols.
- There is no accepted byte-buffer type, so TCP v1 must use DUUMBI `string`
  payloads and reject nonrepresentable incoming bytes.
- `duumbi init` can cache modules without adding them to default dependencies.
- Existing runtime and lowering paths already use pointer-sized values for
  heap/resource-like runtime handles.

Assumptions for implementation:

- A blocking C runtime with explicit timeouts is sufficient for v1.
- JSON values can be backed by a small vendored C JSON implementation or an
  equivalent no-network build-time implementation, provided CI does not require
  system JSON packages or internet access.
- The preferred low-risk runtime build path is to keep `compile_runtime()`
  single-source by including a vendored JSON C implementation from
  `duumbi_runtime.c`, with license text preserved. If implementation instead
  compiles multiple C runtime sources, linker tests must cover that change.
- JSON number behavior may follow the selected JSON runtime representation,
  typically double-backed numeric storage, as long as parse/stringify behavior
  is deterministic and not advertised as exact decimal preservation.
- TCP tests can use loopback sockets and test-harness-selected free ports on all
  supported CI platforms.

## Affected Areas

Expected Stage 10 implementation changes:

- Core type model:
  - `src/types.rs`
  - parser/type tests
- JSON-LD parser:
  - `src/parser/mod.rs`
  - optional parser helpers for node reference extraction
- Graph validation and typing where operation/type validation is enforced:
  - `src/graph/*`
  - relevant validation tests
- Compiler lowering:
  - `src/compiler/lowering.rs`
  - optional small helper module under `src/compiler/` if it reduces repeated
    runtime-call boilerplate
- Runtime linking:
  - `src/compiler/linker.rs`
- C runtime:
  - `runtime/duumbi_runtime.c`
  - `runtime/duumbi_runtime.h`
  - optional vendored JSON source/header under `runtime/third_party/` or another
    clearly named runtime-owned path
- Stdlib graph modules:
  - new `stdlib/json.jsonld`
  - new `stdlib/net.jsonld`
- Workspace initialization and module cache:
  - `src/cli/init.rs`
- Tests and fixtures:
  - parser/type tests for new type strings and nested result forms
  - lowering/runtime declaration tests where existing patterns support them
  - workspace init/cache tests for JSON and net modules
  - JSON integration fixtures under `tests/fixtures/`
  - TCP loopback integration tests under `tests/`
- Documentation or module descriptions:
  - minimal module-facing text in manifests, docs, or examples as required by
    the implementation PR evidence

Areas that must not change during Stage 10 unless a later approved spec
explicitly expands scope:

- `specs/DUUMBI-379/PRODUCT.md`
- HTTP, TLS, database, UDP, server routing, JSONPath, schema validation,
  streaming JSON, or byte-buffer APIs
- default `duumbi init` dependencies
- provider/model setup flows
- REPL/TUI mutation behavior
- Studio UI behavior
- registry publication to `registry.duumbi.dev`
- broad Tier 1 installed-module smoke coverage owned by #382
- generated build output, committed runtime artifacts, external service
  credentials, or public-internet-dependent tests

## Technical Approach

### Type And Parser Model

Add first-class DUUMBI types:

- `DuumbiType::Json`
- `DuumbiType::TcpSocket`
- `DuumbiType::TcpListener`

Parser type strings:

- `json`
- `tcp_socket`
- `tcp_listener`
- nested result forms such as `result<json,string>`,
  `result<tcp_socket,string>`, and `result<tcp_listener,string>`

Type behavior:

- `fmt::Display` must round-trip the accepted type strings.
- `is_heap_type()` must return true for `json`, `tcp_socket`, and
  `tcp_listener`.
- `duumbi_type_to_cl()` must map these types to pointer-sized `types::I64`
  values.
- `type_size()` must treat these values as pointer-sized runtime handles.
- Existing result/option semantics remain unchanged.

Add operation variants for accepted built-ins:

- JSON:
  - `Op::JsonParse`
  - `Op::JsonStringify`
  - `Op::JsonGetField`
  - `Op::JsonArrayLen`
  - `Op::JsonArrayGet`
- TCP:
  - `Op::TcpConnect`
  - `Op::TcpListen`
  - `Op::TcpAccept`
  - `Op::TcpRead`
  - `Op::TcpWrite`
  - `Op::TcpClose`
  - `Op::TcpListenerClose`

JSON-LD operation names:

- `duumbi:JsonParse`
- `duumbi:JsonStringify`
- `duumbi:JsonGetField`
- `duumbi:JsonArrayLen`
- `duumbi:JsonArrayGet`
- `duumbi:TcpConnect`
- `duumbi:TcpListen`
- `duumbi:TcpAccept`
- `duumbi:TcpRead`
- `duumbi:TcpWrite`
- `duumbi:TcpClose`
- `duumbi:TcpListenerClose`

Reference field mapping:

| Operation | `operand` | `left` | `right` |
| --- | --- | --- | --- |
| `JsonParse` | input string | none | none |
| `JsonStringify` | json value | none | none |
| `JsonGetField` | json value | key string | none |
| `JsonArrayLen` | json value | none | none |
| `JsonArrayGet` | json value | index i64 | none |
| `TcpConnect` | host string | port i64 | timeout_ms i64 |
| `TcpListen` | host string | port i64 | timeout_ms i64 |
| `TcpAccept` | listener | timeout_ms i64 | none |
| `TcpRead` | socket | max_bytes i64 | timeout_ms i64 |
| `TcpWrite` | socket | data string | timeout_ms i64 |
| `TcpClose` | socket | none | none |
| `TcpListenerClose` | listener | none | none |

`Op::output_type()` must return each new built-in op's declared
`resultType`. The graph validator must continue rejecting invalid missing
references, unknown type strings, and mismatched result shapes where the current
validation layer has enough information to do so.

### Runtime Function Contract

Declare and lower calls to these runtime symbols:

| Runtime symbol | C signature shape | DUUMBI result |
| --- | --- | --- |
| `duumbi_json_parse` | `void *(void *input)` | `result<json,string>` |
| `duumbi_json_stringify` | `void *(void *value)` | `result<string,string>` |
| `duumbi_json_get_field` | `void *(void *value, void *key)` | `result<json,string>` |
| `duumbi_json_array_len` | `void *(void *value)` | `result<i64,string>` |
| `duumbi_json_array_get` | `void *(void *value, int64_t index)` | `result<json,string>` |
| `duumbi_json_free` | `void (void *value)` | cleanup only |
| `duumbi_tcp_connect` | `void *(void *host, int64_t port, int64_t timeout_ms)` | `result<tcp_socket,string>` |
| `duumbi_tcp_listen` | `void *(void *host, int64_t port, int64_t timeout_ms)` | `result<tcp_listener,string>` |
| `duumbi_tcp_accept` | `void *(void *listener, int64_t timeout_ms)` | `result<tcp_socket,string>` |
| `duumbi_tcp_read` | `void *(void *socket, int64_t max_bytes, int64_t timeout_ms)` | `result<string,string>` |
| `duumbi_tcp_write` | `void *(void *socket, void *data, int64_t timeout_ms)` | `result<i64,string>` |
| `duumbi_tcp_close` | `void *(void *socket)` | `result<i64,string>` |
| `duumbi_tcp_listener_close` | `void *(void *listener)` | `result<i64,string>` |
| `duumbi_tcp_socket_free` | `void (void *socket)` | cleanup only |
| `duumbi_tcp_listener_free` | `void (void *listener)` | cleanup only |

Runtime result helpers should be reused. Add small internal C helpers only when
they remove duplicated error construction, for example:

- create `Ok(i64)` without duplicating result allocation;
- create `Ok(pointer)` for JSON/socket/listener/string payloads;
- create `Err(string)` from a C string without exposing payload data.

The implementation must not make `DuumbiResult` recursively free unknown
payloads by default. Existing result payload ownership is type-directed at the
graph/lowering level; broad recursive freeing would risk double-freeing values
unless it is introduced through a separate, tested ownership change.

### JSON Runtime

Preferred implementation:

- Vendor a small permissively licensed C JSON implementation, such as cJSON,
  under a runtime-owned path and preserve its license.
- Keep build behavior offline and deterministic.
- Include the implementation from `duumbi_runtime.c` or update runtime
  compilation to handle multiple C source files with tests.

Required JSON behavior:

- `duumbi_json_parse` validates that input is a DUUMBI string and parses the
  full string.
- Valid objects, arrays, strings, numbers, booleans, and `null` are accepted.
- Invalid syntax returns `Err(string)` with parse-failure context.
- `duumbi_json_get_field` accepts only JSON objects and DUUMBI string keys.
- Missing fields and object kind mismatches return `Err(string)`.
- `duumbi_json_get_field` returns an owned JSON value, not a borrowed child that
  can outlive or alias a freed parent.
- `duumbi_json_array_len` accepts only arrays and returns `Ok(i64)`.
- `duumbi_json_array_get` accepts only arrays, rejects negative indexes and
  out-of-range indexes, and returns an owned JSON value.
- `duumbi_json_stringify` returns compact valid JSON. Formatting, whitespace,
  and object key order are not product guarantees.
- JSON number parse/stringify behavior must be deterministic and documented in
  implementation evidence if precision differs from source text.
- `duumbi_json_free` is null-safe and frees exactly one owned JSON value.

### TCP Runtime

Runtime representation:

- `tcp_socket` should be an opaque runtime struct containing the OS socket
  handle and closed state.
- `tcp_listener` should be an opaque runtime struct containing the OS listener
  handle and closed state.
- Free functions must be idempotent with respect to already-closed resources and
  must not double-close invalid OS handles.

Platform requirements:

- POSIX implementation may use `getaddrinfo`, `socket`, `connect`, `bind`,
  `listen`, `accept`, `recv`, `send`, `poll` or `select`, `fcntl`, and
  `close`.
- Windows implementation must use Winsock types and functions, initialize
  Winsock exactly once or lazily in a thread-safe-enough process-local path, and
  close sockets with `closesocket`.
- `src/compiler/linker.rs` must add `-lws2_32` for Windows runtime linking if
  TCP support uses Winsock symbols.
- All paths must convert OS errors into `Err(string)` without panicking.

Argument validation:

- `port` must be in `1..=65535`; `port <= 0` returns `Err(string)`.
- `timeout_ms` must be positive; `timeout_ms <= 0` returns `Err(string)`.
- `max_bytes` must be positive; `max_bytes <= 0` returns `Err(string)`.
- Host and data arguments must be valid DUUMBI strings.

Blocking behavior:

- `tcp_connect`, `tcp_listen`, `tcp_accept`, `tcp_read`, and `tcp_write` must
  respect explicit timeout values and return within a conservative test
  tolerance.
- Use nonblocking connect plus poll/select or platform socket timeouts; choose
  the simpler path that works reliably on macOS, Linux, and Windows CI.
- No accepted TCP operation may hang indefinitely in tests or normal use.

Read/write behavior:

- `tcp_read` allocates at most `max_bytes`.
- If the peer closes before any data is read, return `Err(string)` identifying
  EOF or peer close.
- If some bytes are read before peer close or timeout, return
  `Ok(partial_string)` when the bytes are valid under the DUUMBI string
  contract.
- If received bytes cannot be represented as a DUUMBI string, return
  `Err(string)` without truncation, replacement, or payload logging.
- `tcp_write` sends bytes from the DUUMBI string and returns the number of bytes
  accepted by the OS/runtime boundary. Partial writes are allowed and must be
  visible through the `Ok(i64)` byte count.
- `tcp_write(socket, "", timeout_ms)` returns `Ok(0)` when the socket is open.

Lifecycle behavior:

- `tcp_close` on an open socket closes it and returns `Ok(0)`.
- Later read/write/close operations on the same closed socket return
  `Err(string)` and do not crash.
- `tcp_listener_close` on an open listener closes it and returns `Ok(0)`.
- Later accept/close operations on the same closed listener return
  `Err(string)` and do not crash.
- Drop/free paths must close unclosed OS handles as cleanup without changing the
  visible result contract of explicit close functions.

### Stdlib Modules And Cache

Add `stdlib/json.jsonld` with exactly these exported wrapper functions:

- `parse(input: string) -> result<json,string>`
- `stringify(value: json) -> result<string,string>`
- `get_field(value: json, key: string) -> result<json,string>`
- `array_len(value: json) -> result<i64,string>`
- `array_get(value: json, index: i64) -> result<json,string>`

Add `stdlib/net.jsonld` with exactly these exported wrapper functions:

- `tcp_connect(host: string, port: i64, timeout_ms: i64) -> result<tcp_socket,string>`
- `tcp_listen(host: string, port: i64, timeout_ms: i64) -> result<tcp_listener,string>`
- `tcp_accept(listener: tcp_listener, timeout_ms: i64) -> result<tcp_socket,string>`
- `tcp_read(socket: tcp_socket, max_bytes: i64, timeout_ms: i64) -> result<string,string>`
- `tcp_write(socket: tcp_socket, data: string, timeout_ms: i64) -> result<i64,string>`
- `tcp_close(socket: tcp_socket) -> result<i64,string>`
- `tcp_listener_close(listener: tcp_listener) -> result<i64,string>`

Update `src/cli/init.rs` to:

- embed `stdlib/json.jsonld` and `stdlib/net.jsonld`;
- write `@duumbi/stdlib-json@1.0.0` and `@duumbi/stdlib-net@1.0.0` into the
  cache layout;
- write manifests whose export lists exactly match the accepted v1 functions;
- keep `DEFAULT_CONFIG_REST` dependencies unchanged.

Do not publish these modules to the registry in #379. Do not add them to default
dependencies in #379.

## Invariants

- Public APIs use `json`, `tcp_socket`, and `tcp_listener`, not raw user-visible
  `i64` handles.
- JSON/TCP functions return `result<_, string>` for recoverable failures.
- TCP operations that can block require explicit positive timeout values.
- Network activity happens only when user graph logic calls `@duumbi/stdlib-net`
  functions.
- New workspace default dependencies remain math, io, lang, and string only.
- Tests do not require public internet, DNS reliability, third-party services,
  credentials, or external service uptime.
- Runtime errors do not log socket payloads, credentials, tokens, or secrets.
- Incoming TCP bytes that are not safely representable as DUUMBI strings return
  errors instead of lossy strings.
- JSON child access returns owned JSON values or otherwise proves safe ownership;
  it must not return dangling borrowed children.
- Explicit close functions and implicit drops must not double-close or
  double-free.
- Cranelift types remain inside `src/compiler/`.
- New public Rust items include doc comments and follow `AGENTS.md` coding
  conventions.
- Stage 10 implementation must stay inside the approved product scope. Any HTTP,
  TLS, DB, server routing, UDP, JSONPath, schema-validation, byte-buffer, or
  default-init policy change requires separate approval.

## BDD-To-Test Mapping

| Product BDD scenario | Required implementation verification |
| --- | --- |
| Parse a valid JSON object and read a field | Integration fixture imports `@duumbi/stdlib-json`, parses `{"name":"duumbi","enabled":true}`, gets `name`, stringifies it, and prints/asserts JSON `"duumbi"`. Parser/type tests cover `result<json,string>`. |
| Invalid JSON returns a visible error | Fixture or unit test calls `parse` on `{"name":` and verifies `ResultIsOk` is false or `unwrap_err` prints/contains a JSON parse error. |
| Missing JSON object fields are recoverable errors | Fixture parses `{"name":"duumbi"}`, calls `get_field("missing")`, and verifies an Err branch without runtime panic. |
| JSON kind mismatches are recoverable errors | Fixture parses `[1,2,3]`, calls `get_field("name")`, and verifies an Err string mentioning object expectation. |
| JSON arrays can be traversed by length and index | Fixture parses `[10,20,30]`, verifies `array_len` returns `Ok(3)`, `array_get(1)` returns JSON, and stringify returns `20`. |
| Invalid JSON array indexes are recoverable errors | Unit or fixture tests cover `array_get(-1)` and `array_get(3)` on a three-item array returning Err. |
| Stringify produces parseable JSON for nested mixed values | Integration test parses `{"items":[1,null,true,"x"]}`, stringifies, reparses, and verifies the second parse succeeds and selected values remain semantically equivalent. |
| JSON resource lifecycle is safe under repeated use | Runtime/integration test loops parse/get/stringify/drop enough times to catch obvious lifecycle errors. When available, run under sanitizer-like local tooling; otherwise record repeated-run evidence in PR. |
| Connect to a loopback echo service | Rust integration test starts a local `TcpListener` on `127.0.0.1:0`, passes its port to a DUUMBI fixture, and verifies connect/write/read/close round trip for `ping`. |
| DUUMBI can create a loopback listener and accept a connection | Integration test selects a free loopback port, retries setup on bind collision, runs a DUUMBI fixture that calls `tcp_listen` and `tcp_accept`, and uses a local client thread to connect and exchange a small string. |
| Connect timeout returns an error instead of hanging | Unit or integration test uses a deterministic unreachable/refused local target with a short positive timeout and verifies Err plus elapsed time within tolerance. Avoid public DNS/internet targets. |
| Read timeout returns an error instead of hanging | Loopback test opens a socket with no incoming data, calls `tcp_read` with a short timeout, verifies Err and bounded elapsed time. |
| Closed sockets reject later operations safely | Fixture closes a socket, then attempts read/write/close and verifies Err without process crash. |
| Invalid TCP arguments are recoverable errors | Tests cover `port <= 0`, `timeout_ms <= 0`, `max_bytes <= 0`, and negative values for relevant operations. |
| Nonrepresentable TCP bytes are not silently corrupted | Loopback test sends invalid UTF-8 or other nonrepresentable bytes and verifies `tcp_read` returns Err without truncation or replacement. |
| New workspaces keep the current default stdlib set | `src/cli/init.rs` tests verify JSON/net cache entries exist while generated `config.toml` dependencies exclude `@duumbi/stdlib-json` and `@duumbi/stdlib-net`. |
| Manifests expose only the accepted v1 functions | Init/cache tests parse JSON/net manifests and assert exact export sets. Additional static test verifies no JSONPath, iterator, schema, HTTP, TLS, DB, server, or UDP exports. |
| Tests do not require external network services | PR evidence and test names show JSON tests use local strings/fixtures, TCP tests bind loopback/local services, and no test uses public internet, third-party credentials, or service uptime. |

## Live E2E Plan

Canonical interface: CLI build/run of JSON-LD graph programs, because #379 adds
runtime-backed stdlib behavior and no new interactive UI.

Provider/LLM path:

- No DUUMBI provider or external LLM call is required.
- Expected external LLM calls for implementation verification: 0.
- Estimated external LLM cost for implementation verification: USD 0.
- Codex reasoning during Ralph cycles is not a DUUMBI external LLM budget item.

Credentials and environment:

- No provider credentials.
- No public internet access.
- Local loopback networking must be available.
- Windows CI must have standard Winsock linkage through the compiler/linker
  environment.

Manual/local smoke commands for Stage 10:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`
- `cargo test --test <json_tcp_integration_test_name>` once the implementation
  introduces a focused integration test file.
- `cargo run -- build <json_fixture>.jsonld -o <temp_binary>` followed by
  running the produced binary and verifying expected JSON output.
- `cargo run -- build <tcp_fixture>.jsonld -o <temp_binary>` followed by a
  loopback harness or integration test that starts local TCP peers and verifies
  bounded read/write behavior.

Recommended focused E2E fixtures:

- JSON success fixture:
  - imports `@duumbi/stdlib-json`;
  - parses object/array JSON;
  - reads a field and an array item;
  - stringifies both;
  - prints deterministic output.
- JSON error fixture:
  - parses invalid JSON or uses wrong-kind access;
  - branches on `ResultIsOk`;
  - prints a deterministic error marker rather than panicking.
- TCP client echo fixture:
  - accepts host/port constants supplied by the test fixture or generated test
    file;
  - connects to a local harness echo server;
  - writes `ping`;
  - reads `ping`;
  - closes the socket.
- TCP listener fixture:
  - listens on a harness-selected free loopback port;
  - accepts one client;
  - reads/writes a small string;
  - closes socket and listener.

Pass criteria:

- Every BDD scenario has automated test coverage or explicit review evidence.
- TCP timeout tests are bounded and do not introduce flaky public-network
  assumptions.
- JSON and TCP runtime resources are dropped safely in success and error paths.
- Default init dependencies remain unchanged.
- Full implementation validation passes on local platform and CI, or any
  platform-specific failure is fixed before review approval.

Fail criteria:

- Any JSON/TCP recoverable error panics instead of returning `Err(string)`.
- Any accepted API exposes raw integer handles as the user-visible type.
- Any TCP test relies on public internet, public DNS, credentials, or external
  service uptime.
- Any blocking TCP operation can hang indefinitely.
- Default `duumbi init` dependencies include JSON or net in #379.
- Runtime/linker changes fail on a supported CI platform.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. Restate the current implementation state and the BDD scenarios still
   uncovered.
2. Propose one bounded implementation target, such as type/parser support,
   JSON runtime support, TCP runtime support, stdlib cache wiring, or focused
   tests.
3. List files expected to change and commands expected to run.
4. Estimate external calls, external cost, and risk before making changes.
5. Check the resource gate before any action that crosses the cycle budget.
6. Apply only scoped changes needed for the cycle target.
7. Run focused verification for that target.
8. Record evidence, remaining risks, and whether another Ralph cycle is needed.

Human approval is required before:

- estimated external LLM or service cost exceeds USD 2 in one cycle;
- more than 10 external network/service calls would be made in one cycle;
- any public-internet or credential-backed test is proposed;
- a new runtime dependency, vendored third-party C library, or license-bearing
  source is introduced without clear license evidence;
- implementation expands beyond JSON/TCP v1 product scope;
- default workspace dependencies or registry publication policy would change;
- a destructive or irreversible operation is needed;
- CI failure diagnosis would require substantially broad unrelated refactoring;
- architecture decisions conflict with `AGENTS.md`, product spec invariants, or
  existing compiler/runtime boundaries.

## Cycle Budget

Default autonomous batch:

- Up to 3 low-risk Ralph cycles may run before the agent must pause and report a
  consolidated evidence summary.
- Each autonomous cycle should keep expected external LLM/service cost at USD 0.
- Each autonomous cycle should keep external network/service calls at 0.
- Local loopback socket operations inside automated tests do not count as
  external service calls.

Suggested cycle sequence:

1. Type/parser/stdlib wrapper skeleton:
   - add new types and ops;
   - add `stdlib/json.jsonld` and `stdlib/net.jsonld`;
   - add init cache wiring without default dependency changes;
   - run parser/init tests.
2. JSON runtime/lowering:
   - add JSON runtime symbols and lowering;
   - add JSON tests and fixtures;
   - run focused JSON tests.
3. TCP runtime/lowering:
   - add TCP runtime symbols, linker support, and lowering;
   - add loopback tests and fixtures;
   - run focused TCP tests.
4. Full validation and cleanup:
   - run formatting, clippy, full tests, and local CLI smoke paths;
   - update implementation PR evidence.

If any cycle discovers that result payload ownership, runtime build structure,
cross-platform sockets, or JSON dependency licensing cannot be solved within
this scope, stop and request human approval instead of silently broadening the
work.

## Task Breakdown

1. Add type support.
   - Extend `DuumbiType`.
   - Add display, heap, parser, and pointer-size handling.
   - Add tests for simple and nested type strings.

2. Add operation support.
   - Extend `Op`.
   - Parse JSON/TCP operation names.
   - Define `output_type()` behavior for new ops.
   - Validate required references and result shapes where existing validation
     supports it.

3. Add runtime declarations and lowering.
   - Extend `RuntimeFuncs`.
   - Declare JSON/TCP runtime symbols.
   - Lower each new op with the reference field mapping in this spec.
   - Track heap/resource results consistently with existing ownership logic.
   - Extend `Drop` dispatch for `json`, `tcp_socket`, and `tcp_listener`.

4. Add JSON runtime.
   - Vendor or otherwise provide offline JSON parsing/stringifying support.
   - Implement parse, stringify, field access, array length, array get, and
     cleanup.
   - Return owned JSON values and `Err(string)` for recoverable failures.

5. Add TCP runtime.
   - Implement opaque socket/listener structs.
   - Implement connect, listen, accept, read, write, close, listener close, and
     cleanup.
   - Add cross-platform timeout and linker support.
   - Convert OS errors into concise `Err(string)` values.

6. Add stdlib modules.
   - Add `stdlib/json.jsonld` and `stdlib/net.jsonld`.
   - Export exactly the accepted v1 functions.
   - Keep graph wrappers consistent with existing stdlib module patterns.

7. Add init/cache wiring.
   - Embed JSON/net modules.
   - Write their cache modules and manifests.
   - Keep default dependencies unchanged.
   - Add exact-export and default-dependency tests.

8. Add JSON verification.
   - Cover success, invalid parse, missing field, wrong kind, array traversal,
     invalid indexes, nested mixed stringify/reparse, and lifecycle repetition.

9. Add TCP verification.
   - Cover local echo connect, DUUMBI listener/accept, connect timeout, read
     timeout, closed-resource operations, invalid arguments, nonrepresentable
     bytes, and no external-network dependency.

10. Run full validation and prepare review evidence.
    - Run required local commands.
    - Summarize BDD mapping, timeout evidence, lifecycle evidence, platform
      handling, and default-init behavior in the implementation PR.

## Verification Plan

Required local commands before implementation PR review:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`

Focused tests to add or update:

- Parser/type tests:
  - `json`
  - `tcp_socket`
  - `tcp_listener`
  - `result<json,string>`
  - `result<tcp_socket,string>`
  - `result<tcp_listener,string>`
- Operation parser/lowering tests:
  - all JSON op names;
  - all TCP op names;
  - missing required references;
  - invalid result types where validation can enforce them.
- Runtime JSON tests:
  - valid object parse;
  - invalid parse;
  - missing field;
  - wrong kind;
  - array length;
  - array item;
  - invalid negative and out-of-bounds indexes;
  - nested mixed stringify/reparse;
  - repeated lifecycle exercise.
- Runtime TCP tests:
  - connect/write/read/close against a local echo server;
  - listen/accept/close with a local client;
  - connect timeout or refused target with bounded elapsed time;
  - read timeout with bounded elapsed time;
  - operations after close;
  - invalid port, timeout, and max byte arguments;
  - nonrepresentable read bytes.
- Init/cache tests:
  - JSON and net cache module files exist;
  - JSON and net manifests parse;
  - manifest export lists are exact;
  - default config dependencies exclude JSON and net.

CI expectations:

- Docs/spec-only PR for this technical spec should use docs-only CI behavior.
- Implementation PR must run Rust-relevant CI.
- Windows CI must pass if TCP support introduces Winsock linkage.
- No implementation PR check may require public internet or external service
  uptime.

Review evidence expected in implementation PR:

- Files changed summary by affected area.
- BDD-to-test coverage table or checklist.
- JSON ownership/lifecycle notes.
- TCP timeout/lifecycle notes.
- Cross-platform socket/linker notes.
- Confirmation that default dependencies are unchanged.
- Local command outputs or CI links for required validation.

## Completion Criteria

Implementation is complete only when:

- `@duumbi/stdlib-json` exists as a cacheable stdlib module and exports exactly
  the accepted v1 JSON functions.
- `@duumbi/stdlib-net` exists as a cacheable stdlib module and exports exactly
  the accepted v1 TCP functions.
- `json`, `tcp_socket`, and `tcp_listener` are first-class DUUMBI types.
- New JSON/TCP built-in ops parse, validate, lower, and call runtime shims.
- Recoverable JSON/TCP failures return `Err(string)`.
- Blocking TCP operations use explicit positive timeouts.
- Runtime cleanup is safe for JSON values, sockets, and listeners.
- JSON tests cover all approved JSON BDD scenarios.
- TCP tests cover all approved TCP BDD scenarios using loopback only.
- Init/cache tests prove JSON/net modules are cached but not default
  dependencies.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test --all` pass locally or in CI before Stage 11 approval.
- Implementation PR evidence links back to this technical spec and the approved
  product spec.

## Failure And Escalation

Stop and request human approval if:

- A proposed solution would expose JSON/socket/listener as raw user-visible
  integer handles.
- Result payload ownership cannot be made safe without changing existing
  result/option ownership semantics.
- TCP cannot be made reliable across supported CI platforms without dropping a
  supported target or broadening runtime architecture.
- JSON dependency licensing is unclear or incompatible with repository policy.
- Tests would require public internet, DNS availability, credentials, or
  third-party service uptime.
- Implementation needs HTTP, TLS, DB, UDP, server routing, JSONPath, schema
  validation, streaming JSON, byte buffers, or registry publication.
- The implementation would add JSON/net to default dependencies.
- Local or CI validation reveals failures outside #379 that require unrelated
  refactoring to proceed.

Non-blocking implementation choices that do not require product re-approval:

- Exact vendored JSON library or equivalent offline implementation.
- Exact compact JSON string formatting.
- Exact internal C struct layout for JSON/socket/listener handles.
- Exact timeout implementation strategy, provided behavior remains bounded and
  cross-platform.
- Exact test file names and fixture organization.

## Open Questions

None blocking for implementation or Stage 9 review.

Implementation agents may choose the exact vendored JSON implementation,
internal numeric representation, socket timeout mechanics, and fixture names as
long as the choices preserve the approved product behavior and invariants in
this technical spec.

## Stage 9 Review Checklist

Stage 9 approval should verify:

- This PR changes only `specs/DUUMBI-379/TECHNICAL.md`.
- The technical spec stays within the approved product spec in
  `specs/DUUMBI-379/PRODUCT.md`.
- BDD-to-test mapping covers every product BDD scenario.
- Live E2E plan is local-only and names CLI build/run as the canonical surface.
- Ralph Cycle Protocol and Cycle Budget include the USD 2 / 10 external call
  human approval gate.
- No implementation code, tests, generated artifacts, runtime assets, or product
  spec files are changed during Stage 8/9.
- PR and issue references are non-closing and the execution issue remains open.
- Codex self-review and required automated review evidence are clean before the
  issue moves to Ready for Build.

## Sources

- `specs/DUUMBI-379/PRODUCT.md`
- `https://github.com/hgahub/duumbi/issues/379`
- `https://github.com/hgahub/duumbi/pull/662`
- `AGENTS.md`
- `docs/architecture.md`
- `docs/coding-conventions.md`
- `.github/workflows/ci.yml`
- `stdlib/string.jsonld`
- `src/types.rs`
- `src/parser/mod.rs`
- `src/parser/ast.rs`
- `src/compiler/lowering.rs`
- `src/compiler/linker.rs`
- `runtime/duumbi_runtime.c`
- `runtime/duumbi_runtime.h`
- `src/cli/init.rs`
- `tests/integration_phase9a1.rs`
- `tests/integration_phase9a3.rs`
- `tests/integration_phase7_local.rs`
