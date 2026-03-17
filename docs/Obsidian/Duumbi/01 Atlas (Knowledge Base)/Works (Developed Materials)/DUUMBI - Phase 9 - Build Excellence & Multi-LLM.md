---
tags:
  - project/duumbi
  - milestone/phase-9
status: in-progress
github_milestone: '"Phase 9a-3: Error Handling (#12); Phase 9A: Stdlib (#13);
  Phase 9B: Multi-LLM (#14); Phase 9C: Benchmark (#15)"'
updated: 2026-03-17
---
# Phase 9 — Build Excellence & Multi-LLM ⏳

> **Kill Criterion:** 5 showcase applications compile and run correctly in ≥95% of attempts (19/20 runs minimum) across at least 2 LLM providers (Anthropic + OpenAI, plus at least one of: Grok/OpenRouter). Benchmark runner produces a reproducible result matrix with CI integration.
> **Status:** ⏳ Planned — starts after Phase 9a
> **Estimated duration:** 8–10 weeks (solo developer)

← Back: [[DUUMBI Roadmap Map]]

---

## Summary

The build pipeline must become reliable, predictable, and multi-provider before any higher-level features (agents, marketing, UX) can succeed. This phase stabilizes the instruction set, expands stdlib, integrates multiple LLM providers with fallback chains, and establishes an autoresearch-inspired iterative benchmark system.

**Rationale:** Everything else builds on this. Marketing a product that fails on common tasks is counterproductive. Multi-agent orchestration on an unreliable build is chaos. This phase is the foundation.

---

## Error Budget Definition

"Flawless" is not a useful engineering criterion. The real question is: **what failure rate is acceptable?**

| Metric | Target | Measurement |
|---|---|---|
| **Per-showcase success rate** | ≥95% (19/20 runs) | Run each showcase 20 times per provider, count successes |
| **Per-provider success rate** | ≥90% (18/20 across all showcases) | Aggregate across all 5 showcases for one provider |
| **Cross-provider consistency** | ≥85% | Same showcase succeeds on ≥85% of providers tested |
| **Regression rate** | 0% | No previously passing showcase may start failing after code changes |

**Failure categories (tracked separately):**
- **Schema failure:** LLM generates invalid JSON-LD (caught by validator) — acceptable, triggers retry
- **Type error:** LLM generates wrong types (caught by ownership/type validator) — acceptable, triggers retry
- **Logic error:** Program compiles but produces wrong output — unacceptable at >5% rate
- **Crash:** Compiled binary panics — unacceptable at >2% rate

---

## Tasks

### M9-STDLIB: Standard Library Expansion

#### Math Operations: sqrt and pow Implementation Strategy

`sqrt` and `pow` do not have native Cranelift opcodes. Implementation via C runtime shims:

- [ ] `duumbi:Sqrt` Op → Cranelift: `call duumbi_sqrt(f64) -> f64`
  - C runtime shim: `double duumbi_sqrt(double x) { return sqrt(x); }` (links to libm)
  - Input type: `f64` only (no integer sqrt — use `f64` conversion first)
  - Validator: reject `Sqrt` on non-f64 types (new error E040)
- [ ] `duumbi:Pow` Op → Cranelift: `call duumbi_pow(f64, f64) -> f64`
  - C runtime shim: `double duumbi_pow(double base, double exp) { return pow(base, exp); }`
  - For integer power: `duumbi:PowI64` → iterative multiplication in graph (no libm dependency)
  - Validator: `Pow` requires f64 operands; `PowI64` requires i64 operands
- [ ] `duumbi:Modulo` Op → Cranelift: `srem` for i64 (signed remainder), `call fmod` for f64
- [ ] `duumbi:BitwiseAnd` → Cranelift: `band`
- [ ] `duumbi:BitwiseOr` → Cranelift: `bor`
- [ ] `duumbi:BitwiseXor` → Cranelift: `bxor`
- [ ] `duumbi:ShiftLeft` → Cranelift: `ishl`
- [ ] `duumbi:ShiftRight` → Cranelift: `sshr` (arithmetic, sign-extending)

#### Stdlib Modules
- [ ] `stdlib/lang.jsonld` — type casting (i64↔f64, i64→String via Phase 9a), assertions (`duumbi:Assert` → panic if false)
- [ ] `stdlib/math.jsonld` — expanded with `Sqrt`, `Pow`, `PowI64`, `Modulo`, `abs`, `max`, `min`
- [ ] `stdlib/io.jsonld` — expanded: stdin reading (`duumbi:ReadLine` → C shim `duumbi_read_line() -> String`), formatted output
- [ ] `stdlib/string.jsonld` — string utility functions built on Phase 9a String ops (trim, split, join — implemented as graph compositions, not new Op nodes)
- [ ] Each stdlib module: full schema compliance + unit tests + mdBook documentation
- [ ] Design reference: OpenJDK `java.lang`, `java.math`

### M9-LLM: Multi-LLM Provider Integration
- [ ] `LlmProvider` trait implementations:
  - [ ] Grok (xAI) — `https://api.x.ai/v1/chat/completions` (OpenAI-compatible API)
  - [ ] OpenRouter — meta-router, single API for Claude/GPT/Gemini/Llama/Mistral
- [ ] Provider fallback chain: if primary fails → automatically try next
- [ ] `[providers]` section in `config.toml`: priority order, API keys, model selection
- [ ] Provider-specific prompt tuning: each provider may need different system prompts for optimal JSON-LD + ownership-correct generation
- [ ] Integration tests: same intent × each provider → compare output quality
- [ ] Ownership prompt engineering: teach each provider the DUUMBI ownership model (Phase 9a) through system prompt + few-shot examples specific to that provider's strengths/weaknesses

### M9-ITER: Iterative Benchmark System (Autoresearch-Inspired)

Inspired by [Karpathy's autoresearch](https://github.com/karpathy/autoresearch) pattern: autonomous iteration cycles where AI modifies, builds, measures, and keeps or discards results automatically.

#### Showcase Specifications
- [ ] 5 showcase intent specs (YAML):
  1. **Calculator** — `add`, `sub`, `mul`, `div` with i64 and f64, error handling for div-by-zero via `Result<f64, String>`
  2. **Fibonacci** — recursive, branching, multi-function, tests fib(10)=55
  3. **Sorting** — mergesort on `Array<i64>`, ownership-correct (borrows for comparison, moves for swap)
  4. **State machine** — SMTP-like states with String commands, `Result` for invalid transitions
  5. **Multi-module** — 2+ modules with cross-module calls, String passing, shared types via imports

#### Benchmark Runner

- [ ] `duumbi benchmark` CLI command: runs all showcases × all configured providers
- [ ] **Output format:** structured JSON report

```json
{
  "run_id": "bench-2026-04-15-001",
  "timestamp": "2026-04-15T14:30:00Z",
  "duumbi_version": "0.9.0",
  "cranelift_version": "0.129.0",
  "results": [
    {
      "showcase": "calculator",
      "provider": "anthropic/claude-sonnet-4-20250514",
      "attempts": 20,
      "successes": 19,
      "failures": [
        {
          "attempt": 7,
          "failure_type": "schema_error",
          "error_code": "E022",
          "retry_count": 5,
          "final_error": "Borrow exclusivity violation on div result"
        }
      ],
      "avg_tokens_per_attempt": 3200,
      "avg_compile_time_ms": 450,
      "avg_total_time_ms": 8500
    }
  ],
  "summary": {
    "overall_success_rate": 0.96,
    "per_provider": {
      "anthropic": 0.97,
      "openai": 0.94,
      "grok": 0.91
    },
    "per_showcase": {
      "calculator": 1.00,
      "fibonacci": 0.98,
      "sorting": 0.95,
      "state_machine": 0.90,
      "multi_module": 0.92
    }
  }
}
```

- [ ] **CI integration:** `duumbi benchmark --ci` → exit code 0 if all targets met, exit code 1 if any below threshold
- [ ] **Regression detection:** compare current run against `.duumbi/benchmarks/baseline.json` → flag any showcase that dropped >5%
- [ ] **GitHub Actions job:** weekly benchmark run, results posted as PR comment or GitHub Pages dashboard
- [ ] Report storage: `.duumbi/benchmarks/results/{run_id}.json` (append-only history)

#### Autoresearch Cycle
- [ ] For each showcase × provider: generate → build → run → measure
- [ ] If fail → retry with different strategy (max 5 iterations):
  1. Original prompt + error message
  2. Simplified prompt (fewer constraints)
  3. Step-by-step decomposition (generate one function at a time)
  4. Different model via OpenRouter (if available)
  5. Minimal reproduction (strip to simplest failing case)
- [ ] Error categorization: classify errors into schema/type/logic/crash buckets
- [ ] Few-shot library: successful runs saved as examples, matched by showcase type + error pattern

### M9-INST: Instruction Set Stabilization
- [ ] Complete instruction set documentation: every Op → Cranelift mapping → type rules → edge cases
  - Include Phase 9a ops: Alloc, Drop, Move, Borrow, BorrowMut, all String/Array/Result ops
- [ ] Identify and fix missing edge cases (nested calls, recursive structs, deeply branched control flow)
- [ ] Regression test suite: any new Op must have ≥3 test cases before merge
- [ ] Cranelift version pin: see [[DUUMBI - Cranelift Dependency Policy]]

---

## Dependencies

```
Phase 9a (Type System)     ──→ Phase 9 (String, Array, Result, ownership required for showcases)
Phase 7 (Registry client)  ──→ Phase 9 (stdlib modules use registry patterns)
Phase 8 (Registry Auth)    ──→ Phase 9 (publish showcase modules to registry)
Phase 5 (Intent)           ──→ Phase 9 (showcase intents use intent pipeline)
```

## Expected Files

```
src/providers/
├── mod.rs        — LlmProvider trait + fallback chain
├── anthropic.rs  — (existing, refactored)
├── openai.rs     — (existing, refactored)
├── grok.rs       — xAI provider
└── openrouter.rs — OpenRouter provider
stdlib/
├── lang.jsonld   — type casting, assertions
├── math.jsonld   — (expanded: Sqrt, Pow, PowI64, Modulo, bitwise)
├── io.jsonld     — (expanded: ReadLine)
└── string.jsonld — string utilities (trim, split, join)
benchmarks/
├── showcase/     — 5 intent YAML specs
├── runner.rs     — benchmark runner + CI integration
├── report.rs     — JSON report generation + regression detection
└── results/      — per-run result JSONs
src/runtime/
├── duumbi_runtime.c — (extended: sqrt, pow, fmod, read_line)
└── duumbi_runtime.h — (extended)
```

## Monetization

Phase 9 does not directly unlock a paid tier. It strengthens the free tier so that paid features (Phase 12+) have a solid foundation.

---

## Related

- [[DUUMBI - Phase 9a - Type System Completion]] — prerequisite (String, Array, Result, ownership)
- [[DUUMBI - Phase 10 - Intelligent Context & Knowledge Graph]] — builds on stable build
- [[DUUMBI - Phase 11 - CLI UX & Developer Experience]] — builds on stable build
- [[DUUMBI - Phase 12 - Dynamic Agent System & MCP]] — requires stable multi-LLM
- [[DUUMBI - Phase 14 - Marketing & Go-to-Market]] — requires showcase demos
- [[DUUMBI - Cranelift Dependency Policy]] — version pinning strategy


> [!note] **Showcase update (2026-03-14):** Added 6th showcase "String manipulation" — a simpler String-only test that validates Phase 9a String/Option types without the complexity of a state machine. If showcase #4 (state machine) struggles with the ≥95% target, showcase #6 ensures at least one String showcase passes reliably. The kill criterion remains "5 of 6 showcases" at ≥95% — the 6th provides a safety margin, not a relaxation of the standard.

Showcase #6 to add to M9-ITER:
  6. **String manipulation** — concat, compare, length, find (returns Option), contains. Pure String operations, no state machine logic. Validates Phase 9a String + Option types in isolation.


---

## Addendum: File I/O & JSON Parse — Stdlib Extensions

> Added 2026-03-14.

Without file system access and structured data parsing, DUUMBI remains a "calculator-grade" tool. 80% of real applications need to read config files, write logs, or parse structured data. Adding File I/O and JSON to Phase 9 stdlib makes the showcases realistic and the tool practically useful.

### M9-FILE: File I/O Stdlib

New owned type: `duumbi:FileHandle` — represents an open file. Follows ownership rules (Drop closes the file).

#### New Op Nodes

| Op | Input | Output | C Shim | Notes |
|---|---|---|---|---|
| `duumbi:FileOpen` | path: `&String`, mode: `&String` | `Result<FileHandle, String>` | `duumbi_file_open(path, mode)` | mode: "r", "w", "a", "rw" |
| `duumbi:FileRead` | handle: `&FileHandle` | `Result<String, String>` | `duumbi_file_read(handle)` | Reads entire file into String |
| `duumbi:FileReadLine` | handle: `&mut FileHandle` | `Result<Option<String>, String>` | `duumbi_file_read_line(handle)` | None at EOF |
| `duumbi:FileWrite` | handle: `&mut FileHandle`, data: `&String` | `Result<i64, String>` | `duumbi_file_write(handle, data)` | Returns bytes written |
| `duumbi:FileClose` | handle: `FileHandle` (move) | `void` | `duumbi_file_close(handle)` | Consumes the handle (ownership) |
| `duumbi:FileExists` | path: `&String` | `bool` | `duumbi_file_exists(path)` | No handle needed |

**Ownership model integration:**
- `FileOpen` returns an owned `FileHandle` — the caller is responsible for closing it
- `FileClose` consumes (moves) the handle — use-after-close is E021 (use-after-move)
- Automatic Drop insertion: if a FileHandle goes out of scope without `FileClose`, the auto-Drop system closes it (RAII pattern)
- `FileRead` borrows the handle (shared) — multiple reads OK
- `FileWrite` borrows the handle mutably — exclusive access during write

#### C Runtime Shim

```c
typedef struct {
    FILE* fp;
    int64_t mode;  // 0=read, 1=write, 2=append, 3=read-write
} DuumbiFileHandle;

DuumbiFileHandle* duumbi_file_open(const StringHeader* path, const StringHeader* mode);
StringHeader*     duumbi_file_read(const DuumbiFileHandle* handle);
StringHeader*     duumbi_file_read_line(DuumbiFileHandle* handle);  // NULL at EOF
int64_t           duumbi_file_write(DuumbiFileHandle* handle, const StringHeader* data);
void              duumbi_file_close(DuumbiFileHandle* handle);
int8_t            duumbi_file_exists(const StringHeader* path);
```

#### Stdlib Module
- [ ] `stdlib/file.jsonld` — File I/O functions wrapping the Op nodes
- [ ] Unit tests: open → write → close → open → read → verify content
- [ ] Error handling: FileOpen returns `Result` (file not found, permission denied)
- [ ] mdBook documentation page

---

### M9-JSON: JSON Parse & Serialize

JSON handling is essential for config files, API responses, and data exchange. Implemented as graph-level operations that convert between JSON strings and DUUMBI Struct/Array types.

#### New Op Nodes

| Op | Input | Output | Notes |
|---|---|---|---|
| `duumbi:JsonParse` | json: `&String`, targetType: type spec | `Result<Struct, String>` | Parses JSON string into a Struct matching the target type definition |
| `duumbi:JsonParseArray` | json: `&String`, elementType: type spec | `Result<Array<T>, String>` | Parses JSON array into Array<T> |
| `duumbi:JsonSerialize` | value: `&Struct` or `&Array<T>` | `String` | Serializes Struct/Array to JSON string |
| `duumbi:JsonGet` | json: `&String`, key: `&String` | `Option<String>` | Extract a single value from JSON without full parse |

**Implementation strategy:** The JSON parser is a C runtime shim using a minimal embedded JSON parser (not linking to an external library — either hand-written or using a single-header lib like `cJSON`).

#### C Runtime Shim

```c
// Parses JSON string into a flat key-value structure
// The graph-level JsonParse Op maps JSON fields to Struct fields by name
StringHeader* duumbi_json_get(const StringHeader* json, const StringHeader* key);
StringHeader* duumbi_json_serialize_i64(int64_t value);
StringHeader* duumbi_json_serialize_string(const StringHeader* value);
// Full struct parse/serialize handled at graph level — the codegen maps fields
```

#### Stdlib Module
- [ ] `stdlib/json.jsonld` — JSON utility functions
- [ ] Tests: parse JSON config → Struct → access fields → serialize back → compare
- [ ] Error handling: malformed JSON returns `Err(String)` with parse error message

---

### M9-NET: Network I/O (TCP + Basic HTTP Client)

TCP sockets and a basic HTTP client. Single-threaded, blocking. This enables the state machine showcase (#4) to be a real server, and enables API consumption use cases.

#### New Op Nodes — TCP

| Op | Input | Output | C Shim | Notes |
|---|---|---|---|---|
| `duumbi:TcpListen` | addr: `&String`, port: `i64` | `Result<TcpListener, String>` | `duumbi_tcp_listen` | Bind + listen |
| `duumbi:TcpAccept` | listener: `&TcpListener` | `Result<TcpStream, String>` | `duumbi_tcp_accept` | Blocking accept |
| `duumbi:TcpConnect` | addr: `&String`, port: `i64` | `Result<TcpStream, String>` | `duumbi_tcp_connect` | Client connect |
| `duumbi:TcpRead` | stream: `&mut TcpStream` | `Result<String, String>` | `duumbi_tcp_read` | Read until newline or buffer full |
| `duumbi:TcpWrite` | stream: `&mut TcpStream`, data: `&String` | `Result<i64, String>` | `duumbi_tcp_write` | Write bytes |
| `duumbi:TcpClose` | stream: `TcpStream` (move) | `void` | `duumbi_tcp_close` | Consumes stream |

#### New Op Nodes — HTTP Client (Convenience Layer)

| Op | Input | Output | C Shim | Notes |
|---|---|---|---|---|
| `duumbi:HttpGet` | url: `&String` | `Result<String, String>` | `duumbi_http_get` | Simple GET, returns body |
| `duumbi:HttpPost` | url: `&String`, body: `&String` | `Result<String, String>` | `duumbi_http_post` | POST with body, returns response |

**Implementation:** HTTP client built on raw TCP sockets in the C shim — minimal HTTP/1.1 implementation (no TLS in Phase 9). TLS (HTTPS) deferred to Phase 15+ or via C shim linking to `libcurl`.

#### Ownership Model
- `TcpListener` and `TcpStream` are owned types — follow RAII (auto-close on Drop)
- `TcpAccept` borrows the listener (shared) — listener keeps listening
- `TcpRead`/`TcpWrite` borrow the stream mutably — exclusive access
- `TcpClose` consumes (moves) the stream

#### C Runtime Shim

```c
typedef struct { int fd; } DuumbiTcpListener;
typedef struct { int fd; } DuumbiTcpStream;

DuumbiTcpListener* duumbi_tcp_listen(const StringHeader* addr, int64_t port);
DuumbiTcpStream*   duumbi_tcp_accept(const DuumbiTcpListener* listener);
DuumbiTcpStream*   duumbi_tcp_connect(const StringHeader* addr, int64_t port);
StringHeader*      duumbi_tcp_read(DuumbiTcpStream* stream);
int64_t            duumbi_tcp_write(DuumbiTcpStream* stream, const StringHeader* data);
void               duumbi_tcp_close(DuumbiTcpStream* stream);
void               duumbi_tcp_listener_close(DuumbiTcpListener* listener);

// HTTP convenience (built on TCP)
StringHeader*      duumbi_http_get(const StringHeader* url);
StringHeader*      duumbi_http_post(const StringHeader* url, const StringHeader* body);
```

#### Stdlib Modules
- [ ] `stdlib/net.jsonld` — TCP socket functions
- [ ] `stdlib/http.jsonld` — HTTP client convenience functions (built on net)
- [ ] Tests: TCP echo server (listen → accept → read → write back → close)
- [ ] Tests: HTTP GET to a known endpoint → parse response

#### Scope Limitations (Explicit)
- **No TLS/HTTPS in Phase 9.** HTTP only. TLS requires linking to OpenSSL/rustls — deferred.
- **No async.** All socket operations are blocking. The binary is single-threaded.
- **No DNS resolution.** IP addresses only (or rely on OS `getaddrinfo` via C shim).

---

### Updated Showcase Specifications (with File/JSON/Network)

The 6 showcases are updated to use the new stdlib capabilities:

| # | Showcase | Uses File I/O | Uses JSON | Uses Network | Complexity |
|---|---|---|---|---|---|
| 1 | Calculator | No | No | No | Simple |
| 2 | Fibonacci | No | No | No | Simple |
| 3 | Sorting | No | No | No | Moderate (ownership) |
| 4 | State machine | Yes (config file) | Yes (JSON config) | Yes (TCP server) | High |
| 5 | Multi-module | Yes (log file) | No | No | Moderate |
| 6 | String manipulation | No | No | No | Simple (Phase 9a validation) |

Showcase #4 (state machine) now reads its initial state from a JSON config file and listens on a TCP port — making it a realistic SMTP-like server demo.

Showcase #5 (multi-module) writes a log file — demonstrating file I/O with ownership (FileHandle lifecycle).

---

### Updated Expected Files

```
stdlib/
├── lang.jsonld    — type casting, assertions
├── math.jsonld    — Sqrt, Pow, Modulo, bitwise
├── io.jsonld      — stdin/stdout
├── string.jsonld  — trim, split, join
├── file.jsonld    — File I/O (FileHandle, open, read, write, close)
├── json.jsonld    — JSON parse/serialize
├── net.jsonld     — TCP sockets
└── http.jsonld    — HTTP client (GET, POST)
src/runtime/
├── duumbi_runtime.c  — (extended: file ops, json ops, tcp ops, http ops)
└── duumbi_runtime.h  — (extended: FileHandle, TcpListener, TcpStream structs)
```

### Updated Time Estimate

Phase 9 estimate increases from 8–10 weeks to **10–14 weeks** due to File I/O (+2 wks), JSON (+1 wk), Network (+3 wks). Total roadmap impact: ~4–6 weeks additional.


---

## Addendum: Automatic Prompt Evolution (Autoresearch on Prompts)

> Added 2026-03-14.

The Phase 9 benchmark runner tests showcase × provider combinations. But the **prompt** is equally important as the showcase and the provider. Different prompt formulations yield different success rates. The benchmark system should also iterate on prompts.

### How It Works

```
For each showcase × provider:
    For each prompt variant (3–4 variants per showcase):
        Run 20 attempts
        Record success rate per variant
    
    Select best-performing variant → becomes default for this showcase × provider
    Archive all variants + results in .duumbi/benchmarks/prompts/
```

### Prompt Variant Types

| Variant | Description | When It's Better |
|---|---|---|
| **Direct** | "Generate a mergesort function for Array<i64>" | Simple tasks, strong models (Opus/GPT-4) |
| **Step-by-step** | "First generate the comparison function, then the merge function, then the sort function" | Complex tasks, weaker models |
| **Rust-anchored** | "Generate JSON-LD that, when displayed as Rust, would look like: `fn sort(arr: &mut Vec<i64>) { ... }`" | Models strong in Rust (Claude, GPT-4) |
| **Ownership-explicit** | "Generate a sort function. The array is borrowed mutably. Elements are compared via shared borrows. No allocations inside the loop." | When Phase 2 (ownership) of two-phase generation fails frequently |

### Implementation
- [ ] Prompt variant storage: `.duumbi/benchmarks/prompts/{showcase}/{variant_name}.txt`
- [ ] Benchmark runner extended: test all variants, record per-variant success rates
- [ ] Auto-selection: best variant per showcase × provider stored in `.duumbi/benchmarks/best_prompts.json`
- [ ] The orchestrator loads `best_prompts.json` when generating — uses the proven-best prompt for each task type
- [ ] Prompt evolution log: which variant won, by how much, on which date — enables tracking improvement over time
- [ ] New prompt variants can be added manually (user creates a `.txt` file) or by LLM suggestion (Phase 12 agent knowledge)

### Benchmark Runner JSON Report Extension

```json
{
  "showcase": "sorting",
  "provider": "anthropic",
  "prompt_variants": [
    {"name": "direct", "attempts": 20, "successes": 16, "rate": 0.80},
    {"name": "step_by_step", "attempts": 20, "successes": 18, "rate": 0.90},
    {"name": "rust_anchored", "attempts": 20, "successes": 19, "rate": 0.95},
    {"name": "ownership_explicit", "attempts": 20, "successes": 17, "rate": 0.85}
  ],
  "best_variant": "rust_anchored",
  "best_rate": 0.95
}
```

### Connection to `--lang` Projection

The "Rust-anchored" prompt variant uses `duumbi describe --lang rust` output as part of the prompt context. This creates a virtuous cycle:
1. `--lang rust` shows the developer what the graph looks like as Rust
2. The same Rust projection is used as LLM context ("make something that looks like this")
3. The LLM is better at generating graphs that look like Rust code it's been trained on
4. Success rates improve → the best prompt variant gets selected automatically


---

## Addendum: TLS, SQLite, and Event Loop — Production-Grade Stdlib

> Added 2026-03-15.

The basic TCP/HTTP stdlib (M9-NET) serves HTTP without encryption, handles one client at a time, and has no persistent storage. This makes it demo-grade. Three additions bring it to production-viable:

### M9-TLS: HTTPS via LibreSSL/OpenSSL

The generated binary remains single-threaded. TLS is handled entirely in the C runtime shim by linking to the system's TLS library.

#### New Op Nodes

| Op | Input | Output | Notes |
|---|---|---|---|
| `duumbi:TlsConnect` | addr: `&String`, port: `i64` | `Result<TlsStream, String>` | Client-side TLS connection |
| `duumbi:TlsRead` | stream: `&mut TlsStream` | `Result<String, String>` | Read from TLS stream |
| `duumbi:TlsWrite` | stream: `&mut TlsStream`, data: `&String` | `Result<i64, String>` | Write to TLS stream |
| `duumbi:TlsClose` | stream: `TlsStream` (move) | `void` | Close TLS connection (RAII) |
| `duumbi:HttpsGet` | url: `&String` | `Result<String, String>` | Convenience: HTTPS GET |
| `duumbi:HttpsPost` | url: `&String`, body: `&String` | `Result<String, String>` | Convenience: HTTPS POST |

**Server-side TLS** (STARTTLS pattern):
| Op | Input | Output | Notes |
|---|---|---|---|
| `duumbi:TlsAccept` | listener: `&TcpListener`, cert_path: `&String`, key_path: `&String` | `Result<TlsStream, String>` | Accept + TLS handshake |

#### C Runtime Implementation Strategy

Two options, chosen at compile time via `config.toml`:

```toml
[build]
tls-backend = "system"    # "system" (link to OS OpenSSL/LibreSSL) | "vendored" (bundle LibreSSL)
```

- **`system`**: Link to OS-provided `libssl` + `libcrypto`. Zero additional binary size. Requires OpenSSL/LibreSSL installed on target. The `cc` linker invocation adds `-lssl -lcrypto`.
- **`vendored`**: Bundle LibreSSL (small, embeddable, ISC license). Increases binary size ~1MB. Works in air-gapped environments.

```c
// C shim wraps OpenSSL/LibreSSL API
#include <openssl/ssl.h>
#include <openssl/err.h>

typedef struct {
    SSL* ssl;
    int fd;
} DuumbiTlsStream;

DuumbiTlsStream* duumbi_tls_connect(const StringHeader* addr, int64_t port);
StringHeader*     duumbi_tls_read(DuumbiTlsStream* stream);
int64_t           duumbi_tls_write(DuumbiTlsStream* stream, const StringHeader* data);
void              duumbi_tls_close(DuumbiTlsStream* stream);

// Convenience (built on TLS)
StringHeader*     duumbi_https_get(const StringHeader* url);
StringHeader*     duumbi_https_post(const StringHeader* url, const StringHeader* body);

// Server-side
DuumbiTlsStream* duumbi_tls_accept(int listener_fd, const StringHeader* cert_path, const StringHeader* key_path);
```

#### Stdlib
- [ ] `stdlib/tls.jsonld` — TLS stream operations
- [ ] `stdlib/https.jsonld` — HTTPS convenience (built on tls + http)
- [ ] Tests: HTTPS GET to a known endpoint (e.g., https://httpbin.org/get) → parse response

---

### M9-DB: SQLite Embedded Database

SQLite is a single C file (`sqlite3.c`, ~250KB) that compiles directly into the binary. No external server, no network, no configuration. Perfect for DUUMBI's single-threaded, self-contained model.

#### New Op Nodes

| Op | Input | Output | Notes |
|---|---|---|---|
| `duumbi:DbOpen` | path: `&String` | `Result<DbConnection, String>` | Open/create SQLite database file |
| `duumbi:DbExecute` | conn: `&DbConnection`, sql: `&String` | `Result<i64, String>` | Execute SQL (INSERT/UPDATE/DELETE), returns rows affected |
| `duumbi:DbQuery` | conn: `&DbConnection`, sql: `&String` | `Result<DbResult, String>` | Execute SELECT, returns result set |
| `duumbi:DbResultNext` | result: `&mut DbResult` | `Option<DbRow>` | Iterate rows (None when exhausted) |
| `duumbi:DbRowGetI64` | row: `&DbRow`, column: `i64` | `Result<i64, String>` | Read i64 column value |
| `duumbi:DbRowGetString` | row: `&DbRow`, column: `i64` | `Result<String, String>` | Read String column value |
| `duumbi:DbRowGetF64` | row: `&DbRow`, column: `i64` | `Result<f64, String>` | Read f64 column value |
| `duumbi:DbClose` | conn: `DbConnection` (move) | `void` | Close database (RAII) |

**Ownership model:**
- `DbConnection` is owned — Drop closes the connection
- `DbResult` borrows the connection (shared) — connection stays open during iteration
- `DbRow` borrows the result (shared) — result stays valid during row access
- SQL injection prevention: parameterized queries via `duumbi:DbExecuteParam` (future enhancement)

#### C Runtime

```c
#include "sqlite3.h"  // vendored, compiled into the binary

typedef struct { sqlite3* db; } DuumbiDbConnection;
typedef struct { sqlite3_stmt* stmt; } DuumbiDbResult;
typedef struct { sqlite3_stmt* stmt; int current_row; } DuumbiDbRow;

DuumbiDbConnection* duumbi_db_open(const StringHeader* path);
int64_t             duumbi_db_execute(const DuumbiDbConnection* conn, const StringHeader* sql);
DuumbiDbResult*     duumbi_db_query(const DuumbiDbConnection* conn, const StringHeader* sql);
DuumbiDbRow*        duumbi_db_result_next(DuumbiDbResult* result);  // NULL when done
int64_t             duumbi_db_row_get_i64(const DuumbiDbRow* row, int64_t column);
StringHeader*       duumbi_db_row_get_string(const DuumbiDbRow* row, int64_t column);
double              duumbi_db_row_get_f64(const DuumbiDbRow* row, int64_t column);
void                duumbi_db_close(DuumbiDbConnection* conn);
```

#### Stdlib
- [ ] `stdlib/db.jsonld` — SQLite operations
- [ ] Tests: open → create table → insert → query → verify rows → close
- [ ] sqlite3.c vendored in `src/runtime/vendor/sqlite3.c` (public domain, ~250KB)

---

### M9-EVENTLOOP: Connection Multiplexing (select/poll)

The generated binary is single-threaded. Without multiplexing, the TCP server handles one client at a time — unacceptable for any real server. The solution: a `select()`/`poll()` event loop hidden inside the C runtime shim.

**Key architectural principle:** The DUUMBI type system and graph remain single-threaded and synchronous. The event loop is invisible to the graph — it lives entirely in the C shim. The developer writes linear request-handling logic; the runtime multiplexes connections.

#### New Op Nodes

| Op | Input | Output | Notes |
|---|---|---|---|
| `duumbi:ServerStart` | addr: `&String`, port: `i64`, handler: Function @id | `Result<void, String>` | Start event loop, dispatch connections to handler |
| `duumbi:ServerStartTls` | addr, port, cert_path, key_path, handler | `Result<void, String>` | Same but with TLS |

The `handler` is a DUUMBI function with signature `fn(stream: &mut TcpStream) -> Result<void, String>`. The event loop calls this function for each client connection. From the handler's perspective, it is a normal synchronous function — read request, process, write response. The event loop handles the multiplexing transparently.

#### C Runtime

```c
// Event loop based on poll() (POSIX) or select() (fallback)
// Each accepted connection calls the DUUMBI-compiled handler function

typedef void (*DuumbiHandler)(DuumbiTcpStream* stream);

void duumbi_server_start(
    const StringHeader* addr,
    int64_t port,
    DuumbiHandler handler,     // function pointer to compiled DUUMBI handler
    int64_t max_connections    // max simultaneous connections (default 64)
);
```

**How it works internally:**
1. `duumbi_server_start` creates a listening socket
2. Enters a `poll()` loop
3. On new connection → `accept()` → add to poll set
4. On readable connection → call `handler(stream)` synchronously
5. Handler returns → response is written → connection closed or kept alive
6. Loop continues

**Limitation:** Each handler call is blocking — while handler executes for client A, client B waits. This is acceptable for request/response protocols (HTTP, SMTP) where handler execution is fast (<100ms). For long-running handlers, the server queues connections.

**Configuration:**
```toml
[server]
max-connections = 64          # poll() set size
handler-timeout-ms = 5000     # kill handler if it takes too long (prevents hung connections)
```

#### Stdlib
- [ ] `stdlib/server.jsonld` — ServerStart, ServerStartTls convenience wrappers
- [ ] Tests: start server → send 3 concurrent requests → all get responses
- [ ] Benchmark: measure requests/second for a simple echo handler

---

### Updated Showcase #4 (State Machine) — Now Production-Viable

With TLS + SQLite + event loop, the SMTP state machine showcase becomes realistic:

```
Showcase #4: SMTP-like Server
├── Reads config from JSON file (M9-FILE + M9-JSON)
├── Listens on TCP port with TLS (M9-TLS + M9-EVENTLOOP)
├── Handles multiple concurrent connections (poll-based)
├── State machine: GREETING → EHLO → MAIL FROM → RCPT TO → DATA → QUIT
├── Stores received messages in SQLite (M9-DB)
├── Returns proper SMTP response codes
└── Error handling via Result throughout
```

This is a program someone would actually use — not a toy.

### Updated Expected Files

```
stdlib/
├── ... (existing)
├── tls.jsonld     — TLS stream operations
├── https.jsonld   — HTTPS convenience (GET, POST)
├── db.jsonld      — SQLite operations
└── server.jsonld  — Event loop server (ServerStart, ServerStartTls)
src/runtime/
├── duumbi_runtime.c   — (extended: TLS, SQLite, event loop)
├── duumbi_runtime.h   — (extended)
└── vendor/
    └── sqlite3.c      — vendored SQLite (public domain)
```

### Updated Time Estimate

Phase 9 estimate increases from 10–14 weeks to **12–16 weeks**:
- TLS: +1–2 weeks (mostly C shim + linker flag configuration)
- SQLite: +1 week (sqlite3.c is drop-in)
- Event loop: +1–2 weeks (poll() integration + handler dispatch)


---

## Scope Decision (2026-03-17)

**Deferred to Phase 10+:** The following sections in this note are deferred and will NOT be implemented in Phase 9:

- **M9-FILE** (File I/O) → Phase 10+
- **M9-JSON** (JSON Parse & Serialize) → Phase 10+
- **M9-NET** (TCP + HTTP) → Phase 10+
- **M9-TLS** (HTTPS via LibreSSL/OpenSSL) → Phase 10+
- **M9-DB** (SQLite Embedded Database) → Phase 10+
- **M9-EVENTLOOP** (Connection Multiplexing) → Phase 10+

**Rationale:** These features add significant implementation scope (6+ weeks) without contributing to the Phase 9 kill criterion. The showcases can be implemented with in-memory data structures, String operations, and Result/Option error handling — no file/network I/O needed.

**Showcase #4 (State Machine) simplified:** In-memory state machine with String commands instead of TCP server + JSON config + SQLite storage.

**Revised Phase 9 structure:**
- **9a-3**: Error Handling (Result/Option types, Match op, E030–E035) — 14 issues, branch `phase9a-3/error-handling`
- **9A**: Stdlib & Instruction Set (Math ops, stdlib modules) — 16 issues, parallel with 9B
- **9B**: Multi-LLM Providers (LlmProvider trait, Grok/OpenRouter, fallback) — 12 issues, parallel with 9A
- **9C**: Benchmark & Showcases (6 showcases, benchmark runner, CI) — 14 issues, depends on 9a-3 + 9A + 9B

**GitHub issues:** #271–#284 (Phase 9a-3)
