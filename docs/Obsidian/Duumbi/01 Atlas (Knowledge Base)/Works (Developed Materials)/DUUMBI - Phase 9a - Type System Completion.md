---
tags:
  - project/duumbi
  - milestone/phase-9a
status: planned
github_milestone: ~
updated: 2026-03-14
---
# Phase 9a — Type System Completion ⏳

> **Kill Criterion:** (1) A String variable is created, concatenated, compared, and printed in a compiled binary. (2) A dynamic Array is created, elements appended, indexed, and iterated. (3) The schema validator rejects a use-after-move, a double-borrow-mut, and a dangling reference — all at graph validation time, before compilation. (4) A function returns `Result<i64, String>` — the caller handles both Ok and Err paths, and the validator rejects unhandled error cases. (5) All existing Phase 0–8 tests remain green.
> **Status:** ⏳ Planned — starts after Phase 8, before Phase 9
> **Estimated duration:** 10–14 weeks (solo developer)

← Back: [[DUUMBI Roadmap Map]]

---

## Summary

Introduce heap-allocated types (String, dynamic Array, Struct), a Rust-inspired linear ownership model with full lifetime analysis, error handling via Result type, and string operations into the DUUMBI type system.

This is the prerequisite for any non-trivial application — without strings, dynamic data structures, and error handling, the Phase 9 showcases (state machine, real-world apps) are impossible.

The ownership model is enforced entirely at the **schema validation layer** — not at runtime. If the graph passes validation, the compiled binary is memory-safe by construction. No garbage collector, no runtime overhead.

The error handling model follows Rust's `Result<T, E>` pattern — no exceptions, no try/catch. Errors are values in the graph, and the schema validator ensures every error path is handled.

**Scientific significance:** Implementing a full ownership/lifetime system AND algebraic error types as schema constraints on a semantic graph — rather than as compiler passes on an AST — has no published precedent.

---

## Architecture: Ownership in a Semantic Graph

### The Challenge

Traditional ownership systems (Rust's borrow checker) operate on ASTs with lexical scopes. DUUMBI has no AST and no lexical scopes — it has a directed graph of typed operations. The ownership rules must be expressed as **graph invariants** that the schema validator enforces.

### The Model

| Rust Concept | DUUMBI Graph Equivalent |
|---|---|
| Variable binding (`let x = ...`) | Node with `duumbi:owner` edge to a value node |
| Move (`let y = x`) | `duumbi:Move` Op: transfers `duumbi:owner` edge from source to destination |
| Immutable borrow (`&x`) | `duumbi:Borrow` Op: creates `duumbi:borrowedFrom` edge (shared, multiple allowed) |
| Mutable borrow (`&mut x`) | `duumbi:BorrowMut` Op: creates `duumbi:borrowedMutFrom` edge (exclusive) |
| Drop (scope exit) | `duumbi:Drop` Op: explicit deallocation node (inserted by system or user) |
| Lifetime (`'a`) | `duumbi:lifetime` property on borrow nodes — references a Block `@id` |
| Scope | Block node (`duumbi:Block`) — lifetime boundary |
| `Result<T, E>` | `duumbi:Result` type with `duumbi:okType` and `duumbi:errType` |
| `match` on Result | `duumbi:MatchResult` Op with `okBlock` and `errBlock` branches |
| `?` operator | `duumbi:Propagate` Op: early return on Err, unwrap on Ok |

### Validation Rules (Schema Validator Extensions)

New error codes E020–E035:

**Ownership (E020–E029):**

| Rule | Error Code | Description |
|---|---|---|
| Single owner | E020 | Every heap value has exactly one `duumbi:owner` edge at any point |
| Use-after-move | E021 | No node may reference a value after a `duumbi:Move` transferred ownership |
| Borrow exclusivity | E022 | Either N shared borrows OR 1 mutable borrow, never both |
| Borrow lifetime | E023 | A borrow's `duumbi:lifetime` must not outlive the owner's scope |
| Drop completeness | E024 | Every heap value must have `duumbi:Drop` reachable on all control flow paths |
| Double free | E025 | A value may not be dropped more than once |
| Dangling reference | E026 | No borrow may exist after the owner is dropped |
| Move out of borrow | E027 | Cannot move a value while any borrow exists |
| Lifetime parameter | E028 | Cross-function borrows must carry explicit `duumbi:lifetimeParam` |
| Return lifetime | E029 | Returned borrows must be tied to an input lifetime parameter |

**Error Handling (E030–E035):**

| Rule | Error Code | Description |
|---|---|---|
| Unhandled Result | E030 | A `Result` value must be consumed by `MatchResult` or `Propagate` — ignoring a Result is an error |
| Exhaustive match | E031 | `MatchResult` must have both `okBlock` and `errBlock` |
| Propagate context | E032 | `Propagate` (`?` equivalent) may only appear in functions that themselves return `Result` |
| Error type mismatch | E033 | `Propagate` requires the function's error type to match or be convertible from the inner Result's error type |
| Dead Ok path | E034 | If a function always returns Err on all paths, warn (likely logic error) |
| Result ownership | E035 | The inner value of Ok/Err follows ownership rules — unwrapping moves the value out |

### Ownership Flow Example

```jsonld
{
  "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
  "@graph": [
    {
      "@type": "duumbi:Alloc",
      "@id": "duumbi:main/main/entry/0",
      "duumbi:allocType": "String",
      "duumbi:initialValue": "hello",
      "duumbi:owner": "entry",
      "duumbi:resultType": "String"
    },
    {
      "@type": "duumbi:Borrow",
      "@id": "duumbi:main/main/entry/1",
      "duumbi:borrowedFrom": { "@id": "duumbi:main/main/entry/0" },
      "duumbi:mutable": false,
      "duumbi:lifetime": "entry",
      "duumbi:resultType": "&String"
    },
    {
      "@type": "duumbi:StringLength",
      "@id": "duumbi:main/main/entry/2",
      "duumbi:operand": { "@id": "duumbi:main/main/entry/1" },
      "duumbi:resultType": "i64"
    },
    {
      "@type": "duumbi:Drop",
      "@id": "duumbi:main/main/entry/3",
      "duumbi:target": { "@id": "duumbi:main/main/entry/0" }
    }
  ]
}
```

### Error Handling Example

```jsonld
{
  "@type": "duumbi:Function",
  "@id": "duumbi:parser/parse_number",
  "duumbi:params": [{"name": "input", "type": "&String"}],
  "duumbi:returnType": "Result<i64, String>",
  "duumbi:blocks": [
    {
      "@type": "duumbi:Block", "@id": "duumbi:parser/parse_number/entry",
      "duumbi:ops": [
        {
          "@type": "duumbi:Call",
          "@id": "duumbi:parser/parse_number/entry/0",
          "duumbi:function": "stdlib/try_parse_i64",
          "duumbi:args": [{"@id": "...input_borrow..."}],
          "duumbi:resultType": "Result<i64, String>"
        },
        {
          "@type": "duumbi:MatchResult",
          "@id": "duumbi:parser/parse_number/entry/1",
          "duumbi:operand": {"@id": "duumbi:parser/parse_number/entry/0"},
          "duumbi:okBlock": "duumbi:parser/parse_number/ok_path",
          "duumbi:errBlock": "duumbi:parser/parse_number/err_path",
          "duumbi:okBinding": "parsed_value",
          "duumbi:errBinding": "error_msg"
        }
      ]
    },
    {
      "@type": "duumbi:Block", "@id": "duumbi:parser/parse_number/ok_path",
      "duumbi:ops": [
        {"@type": "duumbi:ReturnOk", "duumbi:value": {"@id": "...parsed_value..."}}
      ]
    },
    {
      "@type": "duumbi:Block", "@id": "duumbi:parser/parse_number/err_path",
      "duumbi:ops": [
        {"@type": "duumbi:ReturnErr", "duumbi:value": {"@id": "...error_msg..."}}
      ]
    }
  ]
}
```

---

## Tasks

### M9a-OWN: Ownership & Lifetime System

#### Schema Extensions
- [ ] New node types: `duumbi:Alloc`, `duumbi:Move`, `duumbi:Borrow`, `duumbi:BorrowMut`, `duumbi:Drop`
- [ ] New properties: `duumbi:owner`, `duumbi:borrowedFrom`, `duumbi:borrowedMutFrom`, `duumbi:lifetime`, `duumbi:lifetimeParam`
- [ ] New reference types: `&T` (shared borrow), `&mut T` (mutable borrow)
- [ ] `core.schema.json` updated with all new types and validation rules
- [ ] Lifetime parameter syntax for cross-function borrows

#### Validator Implementation
- [ ] Ownership graph construction: build ownership/borrow edges from the semantic graph
- [ ] Control flow analysis: track ownership state across branches (both paths must drop)
- [ ] Use-after-move detection (E021): flag any reference to a moved value
- [ ] Borrow exclusivity check (E022): enforce shared XOR mutable rule
- [ ] Lifetime analysis (E023): borrows cannot outlive owner's Block scope
- [ ] Cross-function lifetime propagation (E028, E029): lifetime parameters on function signatures
- [ ] Drop completeness check (E024): all control flow paths lead to drop
- [ ] Double free detection (E025): no value dropped twice
- [ ] Dangling reference detection (E026): no borrow after drop
- [ ] Error messages: each E02x error includes the node `@id`, the conflicting operation, and a suggested fix

#### Automatic Drop Insertion
- [ ] System inserts `duumbi:Drop` nodes at Block scope exits for owned values
- [ ] Developer does not manually write Drop nodes (unless explicit early drop desired)
- [ ] Drop order: reverse allocation order (LIFO, matching Rust behavior)

#### Tests
- [ ] Unit tests for each E020–E029 error: valid graph passes, invalid graph caught
- [ ] Property-based testing: random valid ownership graphs → all pass validation
- [ ] Regression: all Phase 0–8 tests still green (ownership is additive, not breaking)

---

### M9a-ERROR: Error Handling via Result Type

#### Type Definitions
- [ ] `duumbi:Result` — algebraic type with two variants:
  - `duumbi:okType` — the success type (any type: i64, String, Struct, etc.)
  - `duumbi:errType` — the error type (typically String, but can be any type)
- [ ] `duumbi:Option` — algebraic type with two variants (None has no value):
  - `duumbi:someType` — the inner type
  - None variant carries no data
- [ ] Result and Option are **not heap-allocated** — they are tagged unions (discriminant + payload)
- [ ] Memory layout: `{ tag: i8 (0=Ok/Some, 1=Err/None), payload: max(sizeof(T), sizeof(E)) }`

#### Operations (New Op Nodes)
- [ ] `duumbi:ReturnOk` — wrap a value in Ok variant
  - Input: value of type T, Output: `Result<T, E>`
  - Moves the value into the Result (ownership transfer)
- [ ] `duumbi:ReturnErr` — wrap a value in Err variant
  - Input: value of type E, Output: `Result<T, E>`
  - Moves the error value into the Result
- [ ] `duumbi:MatchResult` — exhaustive pattern match on Result
  - Input: `Result<T, E>`
  - Properties: `okBlock` (Block @id), `errBlock` (Block @id), `okBinding` (name), `errBinding` (name)
  - The Ok block receives the unwrapped T value (owned), Err block receives E value (owned)
  - Both blocks MUST exist (E031 enforced by validator)
- [ ] `duumbi:Propagate` — equivalent to Rust's `?` operator
  - Input: `Result<T, E>`
  - If Ok: unwraps to T (continues execution)
  - If Err: immediately returns Err from the enclosing function
  - Validator enforces: enclosing function must return `Result<_, E>` (E032)
  - Validator enforces: error types must be compatible (E033)
- [ ] `duumbi:IsOk` / `duumbi:IsErr` — check Result variant without unwrapping
  - Input: `&Result<T, E>`, Output: `bool`
  - Does not consume the Result (takes borrow)
- [ ] `duumbi:UnwrapOr` — unwrap with default value
  - Input: `Result<T, E>`, default: `T`, Output: `T`
  - Moves out of Result on Ok, drops Err and returns default
- [ ] `duumbi:MapResult` — transform the Ok value
  - Input: `Result<T, E>`, mapper: Function `T -> U`, Output: `Result<U, E>`
- [ ] `duumbi:MatchOption` — exhaustive pattern match on Option
  - Input: `Option<T>`, Properties: `someBlock`, `noneBlock`, `someBinding`
- [ ] `duumbi:Unwrap` — panic on None/Err (development convenience, not recommended for production)
  - Input: `Result<T, E>` or `Option<T>`, Output: `T`
  - Runtime panic with error message if Err/None

#### Validator Rules (E030–E035)
- [ ] E030: Unhandled Result — a Result value MUST be consumed (MatchResult, Propagate, UnwrapOr, MapResult, or Unwrap). Ignoring = compile error.
- [ ] E031: MatchResult must have both okBlock and errBlock
- [ ] E032: Propagate only in functions returning Result
- [ ] E033: Error type compatibility check for Propagate
- [ ] E034: Warning if all paths return Err (dead Ok path)
- [ ] E035: Result ownership — unwrapping moves the inner value; the Result is consumed

#### Cranelift Lowering
- [ ] Result/Option as tagged union: `{ tag: i8, payload: [u8; max_size] }`
- [ ] `ReturnOk`/`ReturnErr` → set tag + copy payload
- [ ] `MatchResult` → branch on tag value (brif), jump to okBlock or errBlock
- [ ] `Propagate` → check tag, if Err → early return with Err, if Ok → extract payload
- [ ] `IsOk`/`IsErr` → load tag byte, compare with 0/1

#### C Runtime Shim
- [ ] `duumbi_panic(msg: *const StringHeader) -> !` — print error message and exit(1)
  - Used by `Unwrap` on Err/None

#### Tests
- [ ] Function returning Result<i64, String> — caller handles both paths correctly
- [ ] Propagate chains: function A calls function B (returns Result), uses `?`, returns Result
- [ ] Validator rejects: ignored Result (E030)
- [ ] Validator rejects: MatchResult with missing errBlock (E031)
- [ ] Validator rejects: Propagate in non-Result function (E032)
- [ ] Option<String>: MatchOption with Some and None paths

---

### M9a-STRING: String Type

#### Type Definition
- [ ] `duumbi:String` — heap-allocated, UTF-8, length-prefixed
- [ ] Memory layout: `{ length: i64, capacity: i64, data: *mut u8 }`
- [ ] Owned by default — follows ownership rules
- [ ] String literal: `duumbi:Alloc` with `allocType: "String"` and `initialValue`

#### Operations (New Op Nodes)
- [ ] `duumbi:StringConcat` — concatenate two strings → new owned String
  - Inputs: two `&String` (borrows), Output: owned `String`
- [ ] `duumbi:StringEquals` — byte-level equality comparison
  - Inputs: two `&String`, Output: `bool`
- [ ] `duumbi:StringCompare` — lexicographic ordering
  - Inputs: two `&String`, Output: `i64` (-1, 0, 1)
- [ ] `duumbi:StringLength` — return string length
  - Input: `&String`, Output: `i64`
- [ ] `duumbi:StringSlice` — extract substring (returns borrow)
  - Input: `&String`, start: `i64`, end: `i64`, Output: `&String`
- [ ] `duumbi:StringFromI64` — integer to string conversion
  - Input: `i64`, Output: owned `String`
- [ ] `duumbi:PrintString` — print string to stdout
  - Input: `&String`, Output: `void`
- [ ] `duumbi:StringContains` — check if substring exists
  - Inputs: `&String` (haystack), `&String` (needle), Output: `bool`
- [ ] `duumbi:StringFind` — find substring index
  - Inputs: `&String` (haystack), `&String` (needle), Output: `Option<i64>`

#### C Runtime Shim Extensions
- [ ] `duumbi_alloc(size: i64) -> *mut u8`
- [ ] `duumbi_dealloc(ptr: *mut u8)`
- [ ] `duumbi_string_new(data: *const u8, len: i64) -> *mut StringHeader`
- [ ] `duumbi_string_concat(a: *const StringHeader, b: *const StringHeader) -> *mut StringHeader`
- [ ] `duumbi_string_equals(a: *const StringHeader, b: *const StringHeader) -> i8`
- [ ] `duumbi_string_compare(a: *const StringHeader, b: *const StringHeader) -> i64`
- [ ] `duumbi_i64_to_string(val: i64) -> *mut StringHeader`
- [ ] `duumbi_print_string(s: *const StringHeader)`
- [ ] `duumbi_string_contains(haystack: *const StringHeader, needle: *const StringHeader) -> i8`
- [ ] `duumbi_string_find(haystack: *const StringHeader, needle: *const StringHeader) -> i64` (-1 if not found)

---

### M9a-ARRAY: Dynamic Array Type

#### Type Definition
- [ ] `duumbi:Array<T>` — heap-allocated, dynamically sized, homogeneous
- [ ] Memory layout: `{ length: i64, capacity: i64, element_size: i64, data: *mut T }`
- [ ] Owned by default — follows ownership rules
- [ ] Element type parametric: `duumbi:elementType` specifies `i64`, `f64`, `bool`, `String`, etc.

#### Operations (New Op Nodes)
- [ ] `duumbi:ArrayNew` — create empty array with initial capacity
- [ ] `duumbi:ArrayPush` — append element (may reallocate), takes `&mut Array<T>`, moves element
- [ ] `duumbi:ArrayGet` — read by index, returns `&T`, bounds-checked (panics if OOB)
- [ ] `duumbi:ArraySet` — write by index, takes `&mut Array<T>`, drops old, moves new
- [ ] `duumbi:ArrayLength` — return length
- [ ] `duumbi:ArraySlice` — subarray view (returns borrow)
- [ ] `duumbi:ArrayIter` — iteration: body Block receives `&T` element + `i64` index
- [ ] `duumbi:ArrayTryGet` — safe index access returning `Option<&T>` (no panic)

#### C Runtime Shim Extensions
- [ ] `duumbi_array_new(element_size: i64, capacity: i64) -> *mut ArrayHeader`
- [ ] `duumbi_array_push(arr: *mut ArrayHeader, element: *const u8) -> void`
- [ ] `duumbi_array_get(arr: *const ArrayHeader, index: i64) -> *const u8`
- [ ] `duumbi_array_set(arr: *mut ArrayHeader, index: i64, element: *const u8) -> void`
- [ ] `duumbi_array_length(arr: *const ArrayHeader) -> i64`

---

### M9a-STRUCT: Struct Type (Foundation)

- [ ] `duumbi:Struct` type definition: named fields with types
- [ ] `duumbi:StructNew` — allocate and initialize struct
- [ ] `duumbi:FieldGet` — read field (returns borrow if heap type)
- [ ] `duumbi:FieldSet` — write field (moves value in, drops old)
- [ ] Struct ownership: struct owns its fields; dropping struct drops all fields recursively
- [ ] Nested structs: recursive ownership
- [ ] Cranelift lowering: field offsets calculated at compile time

---

### M9a-CODEGEN: Cranelift Codegen Extensions

#### Heap Management
- [ ] `duumbi:Alloc` → call `duumbi_alloc`, store pointer
- [ ] `duumbi:Drop` → call `duumbi_dealloc`
- [ ] `duumbi:Move` → copy pointer (validator ensures no use-after-move, no runtime check)
- [ ] `duumbi:Borrow` / `duumbi:BorrowMut` → copy pointer (safety is compile-time)

#### String Operations Lowering
- [ ] All String ops → corresponding C runtime shim calls

#### Array Operations Lowering
- [ ] All Array ops → corresponding C runtime shim calls
- [ ] `ArrayIter` → Cranelift loop: index 0..length, call body block per element

#### Error Handling Lowering
- [ ] Result/Option as tagged union in stack memory
- [ ] `MatchResult` → brif on tag byte
- [ ] `Propagate` → check tag, early return if Err
- [ ] `Unwrap` → check tag, call `duumbi_panic` if Err/None

---

### M9a-TEST: Integration Tests

- [ ] String creation + concatenation + print → correct output
- [ ] String comparison (Equals, Compare) → correct results
- [ ] String contains + find → correct results (find returns Option)
- [ ] Array creation + push + get + iterate → correct output
- [ ] ArrayTryGet returns Option: Some for valid index, None for OOB
- [ ] Result: function returns Ok → caller gets value
- [ ] Result: function returns Err → caller handles error
- [ ] Propagate chain: A calls B calls C, error propagates to A
- [ ] Ownership: move string into function → original can't be used (E021)
- [ ] Borrow: pass &String to function → original still usable
- [ ] Lifetime: return &String from inner block → validator rejects (E023)
- [ ] Cross-function lifetime: returned &String tied to input → accepted (E028/E029)
- [ ] Unhandled Result → validator rejects (E030)
- [ ] MatchResult missing errBlock → validator rejects (E031)
- [ ] All Phase 0–8 tests remain green
- [ ] Benchmark: compile time increase <20% for existing programs

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Lifetime analysis too complex for graph | Medium | Critical | Start intra-function (E023), defer cross-function (E028/E029) if needed |
| Cranelift pointer handling edge cases | Medium | High | C runtime shims for complex ops; test on aarch64 + x86_64 |
| Schema validator performance regression | Low | Medium | Benchmark validation time; ownership checks O(n) |
| Breaking change to existing programs | Low | High | Ownership opt-in: programs without heap types skip ownership checks |
| LLM difficulty generating ownership-correct graphs | High | High | Phase 9 few-shot examples; autoresearch iteration on ownership patterns |
| Result type adds complexity to every function | Medium | Medium | Result is optional; functions can still return plain types |
| Tagged union layout correctness on different architectures | Low | High | Test on both aarch64 and x86_64; use fixed-size layout |

---

## Dependencies

```
Phase 1 (Type system: i64, f64, bool)  ──→ Phase 9a (adds String, Array, Struct, Result, ownership)
Phase 2 (AI Integration)               ──→ Phase 9a (LLM must learn ownership + error patterns)
Phase 4 (Schema validator)             ──→ Phase 9a (validator gains E020–E035 checks)
```

## Expected Files

```
src/types/
├── mod.rs          — type system entry point (extended)
├── ownership.rs    — ownership graph construction + validation
├── lifetime.rs     — lifetime analysis engine
├── string.rs       — String type definition + operations
├── array.rs        — Array type definition + operations
├── structs.rs      — Struct type definition + field operations
├── result.rs       — Result<T,E> and Option<T> types + operations
└── error_check.rs  — E030–E035 validation rules
src/codegen/
├── heap.rs         — Alloc, Drop, Move, Borrow codegen
├── string_ops.rs   — String operation lowering
├── array_ops.rs    — Array operation lowering
└── result_ops.rs   — Result/Option lowering (tagged union, MatchResult, Propagate)
src/runtime/
├── duumbi_runtime.c  — (extended: alloc, dealloc, string ops, array ops, panic)
└── duumbi_runtime.h  — header for new functions
.duumbi/schema/
└── core.schema.json  — (extended: new types, ownership, error handling, E020–E035)
```

## Monetization

Phase 9a does not unlock a paid tier. It completes the type system foundation required for all subsequent phases.

---

## Related

- [[DUUMBI - Phase 9 - Build Excellence & Multi-LLM]] — depends on this phase for String-based showcases
- [[DUUMBI - Phase 4 - Interactive CLI & Module System]] — Struct/Array were planned here
- [[DUUMBI - Phase 13 - Self-Healing & Telemetry]] — self-healing repair generates Result-returning code
- [[DUUMBI - PRD]] — Section 5.1 type system
- [[Compilation Pipeline]] — codegen extensions for heap types and error handling


---

## Addendum: Cranelift Abstraction Layer

> Added 2026-03-14. See [[DUUMBI - Cranelift Dependency Policy]] for full context.

Before implementing M9a-CODEGEN heap management, introduce a `CodegenBackend` trait that abstracts Cranelift-specific APIs. This:
1. Localizes Cranelift upgrade impact to a single module
2. Prepares the architecture for a future LLVM backend (Phase 15a)
3. Pins Cranelift version for the duration of Phase 9a (no mid-phase upgrades)

Tasks:
- [ ] Define `CodegenBackend` trait in `src/codegen/backend.rs`
- [ ] Refactor existing codegen to implement `CraneliftBackend`
- [ ] Pin Cranelift version with exact `=0.129.0` in Cargo.toml
- [ ] Add Cranelift canary CI job (weekly, non-blocking)
- [ ] All new M9a codegen (heap, string, array, result) goes through the trait, not direct Cranelift calls


---

## Addendum: Not In Scope — General Enum Type

> Added 2026-03-14.

Phase 9a introduces `Result<T, E>` and `Option<T>` as **specific tagged unions** with hardcoded variants (Ok/Err, Some/None). A general-purpose user-defined enum type (e.g., `Color { Red, Green, Blue }`, `HttpMethod { GET, POST, PUT }`) is **explicitly not in scope** for Phase 9a.

**Rationale:** Result and Option cover the two most critical use cases (error handling, nullable values). A general enum requires:
- User-defined variant declarations in the schema
- Pattern matching on arbitrary variant counts (not just 2)
- Variant payloads of heterogeneous types
- Exhaustiveness checking for N variants (not just Ok/Err)

This is a significant extension and is deferred to **Phase 15+ (Type System Extensions)**.

**Workaround for Phase 9a–14:** Use `i64` constants as enum discriminants with a naming convention:
```jsonld
{
  "@type": "duumbi:Const", "duumbi:value": 0, "duumbi:comment": "HttpMethod::GET"
}
```
This is not type-safe, but functional for early showcases. The Phase 15+ general enum will replace this pattern.


---

## Addendum: Concurrency Model — Explicit Single-Threaded

> Added 2026-03-14.

**The compiled binary is always single-threaded.** There is no `async`, no thread spawn, no concurrency primitive in the DUUMBI type system or Op set. This is a deliberate architectural decision, not an oversight.

### Rationale

1. **Ownership model simplicity.** Rust's borrow checker complexity doubles with concurrent access (`Send`, `Sync`, `Arc<Mutex<T>>`). DUUMBI's ownership model (Phase 9a) is already the hardest part of the roadmap. Adding concurrency would make it intractable for a solo developer in a reasonable timeframe.

2. **Determinism.** Single-threaded execution is deterministic — same input → same output → same trace. Multi-threaded execution introduces non-determinism that undermines the Semantic Fixed Point invariant.

3. **Self-healing compatibility.** Phase 13's trace-based back-mapping assumes a linear execution trace. Concurrent execution would require distributed tracing, which is a fundamentally harder problem.

### Where Concurrency DOES Exist

Concurrency exists in the **DUUMBI toolchain**, not in the generated program:

| Component | Concurrent? | Technology |
|---|---|---|
| Generated binary | **NO** — single-threaded | Cranelift AOT |
| DUUMBI CLI (intent execute) | Yes — multiple LLM calls | `tokio` async runtime |
| Phase 12 parallel Coder agents | Yes — concurrent LLM calls | `tokio` + semaphore |
| Phase 13 Ops Agent monitoring | Yes — separate process reads trace file | OS process |
| Studio WebSocket | Yes — async server | `axum` + `tokio` |

### Future: Concurrency in Generated Programs (Phase 15+)

If concurrency becomes a requirement, the path is:
- `duumbi:Spawn` Op → create a thread/task
- `duumbi:Channel` type → typed message passing (no shared memory)
- `duumbi:Mutex` / `duumbi:Arc` → shared state (requires `Send`/`Sync` traits in schema)
- This is Phase 15+ scope, after LLVM backend enables better threading support.

### Impact on Phase 13

The Phase 13 Ops Agent monitors the running binary by reading `.duumbi/telemetry/traces.jsonl` from a **separate process**. The binary writes to the file, the Ops Agent reads it. This is inter-process communication via filesystem — not intra-process concurrency. No `tokio` or threading needed in the generated binary.


---

## Addendum: `duumbi describe` Update for New Types

> Added 2026-03-14.

The pseudo-code generator (`duumbi describe`) must render the new Phase 9a types in human-readable form. Without this update, the describe output shows raw JSON-LD nodes (`Alloc`, `Borrow`, `MatchResult`) instead of intuitive pseudo-code.

### Required Mappings

| JSON-LD Pattern | Pseudo-Code Output |
|---|---|
| `Alloc { allocType: "String", initialValue: "hello" }` | `let x: String = "hello"` |
| `Borrow { borrowedFrom: x, mutable: false }` | `let ref_x: &String = &x` |
| `BorrowMut { borrowedMutFrom: x }` | `let ref_x: &mut String = &mut x` |
| `Move { source: x, dest: y }` | `let y: String = move x  // x is now invalid` |
| `Drop { target: x }` | `drop(x)` |
| `StringConcat { left: a, right: b }` | `let result: String = a + b` |
| `StringEquals { left: a, right: b }` | `a == b` |
| `StringCompare { left: a, right: b }` | `a.compare(b)  // → -1, 0, 1` |
| `ArrayNew { elementType: i64, capacity: 10 }` | `let arr: Array<i64> = Array::new(10)` |
| `ArrayPush { array: arr, element: v }` | `arr.push(v)` |
| `ArrayGet { array: arr, index: i }` | `arr[i]` |
| `ReturnOk { value: v }` | `return Ok(v)` |
| `ReturnErr { value: e }` | `return Err(e)` |
| `MatchResult { operand: r, okBlock: ..., errBlock: ... }` | `match r { Ok(v) => { ... }, Err(e) => { ... } }` |
| `Propagate { operand: r }` | `let v = r?` |

### Tasks
- [ ] Update `src/describe.rs` (or equivalent) with new type mappings
- [ ] Ownership annotations in describe output: show `move`, `&`, `&mut` explicitly
- [ ] Result/Option pattern match rendered as `match` block
- [ ] Integration tests: describe output for each Phase 9a showcase matches expected pseudo-code
- [ ] `duumbi describe --verbose`: include ownership/lifetime annotations; without flag, show simplified view


---

## Addendum: Multi-Language Projection (`duumbi describe --lang`)

> Added 2026-03-14.

The current `duumbi describe` outputs generic pseudo-code. Adding `--lang` flag renders the graph as **read-only projections** in familiar programming language syntax. This is NOT projectional editing (Phase 15b) — the user cannot edit the projected code. It is a read-only view that makes the graph comprehensible to developers who think in Rust/Python/Go.

### Usage

```bash
duumbi describe                     # Default pseudo-code (existing)
duumbi describe --lang rust         # Rust-like syntax with ownership annotations
duumbi describe --lang python       # Python-like syntax (simplified, no ownership shown)
duumbi describe --lang go           # Go-like syntax (error handling as if/err pattern)
duumbi describe --verbose           # Pseudo-code with full ownership/lifetime annotations
duumbi describe --lang rust --verbose  # Rust syntax with explicit lifetime parameters
```

### Language Projection Examples

Given a function that parses a number with error handling:

**Default (pseudo-code):**
```
fn parse_number(input: &String) -> Result<i64, String>:
    let result = try_parse_i64(input)?
    return Ok(result)
```

**Rust projection (`--lang rust`):**
```rust
fn parse_number(input: &String) -> Result<i64, String> {
    let result = try_parse_i64(input)?;
    Ok(result)
}
```

**Python projection (`--lang python`):**
```python
def parse_number(input: str) -> int:
    """Raises ValueError on parse failure"""
    result = try_parse_i64(input)  # may raise
    return result
```

**Go projection (`--lang go`):**
```go
func parseNumber(input string) (int64, error) {
    result, err := tryParseI64(input)
    if err != nil {
        return 0, err
    }
    return result, nil
}
```

### Implementation

- [ ] `src/describe/mod.rs` — existing pseudo-code generator (refactored to use `DescribeBackend` trait)
- [ ] `src/describe/backend.rs` — `DescribeBackend` trait: `render_function`, `render_block`, `render_op`
- [ ] `src/describe/pseudo.rs` — existing pseudo-code backend (default)
- [ ] `src/describe/rust_lang.rs` — Rust projection backend
- [ ] `src/describe/python_lang.rs` — Python projection backend
- [ ] `src/describe/go_lang.rs` — Go projection backend
- [ ] `--lang` flag added to `duumbi describe` CLI command (clap argument)
- [ ] `--verbose` flag: adds explicit ownership/lifetime annotations in any language mode

### Mapping Rules Per Language

| DUUMBI Concept | Rust | Python | Go |
|---|---|---|---|
| `&String` (shared borrow) | `&String` | `str` (implicit) | `string` (pass by value) |
| `&mut String` (mut borrow) | `&mut String` | `str` (Python has no mut) | `*string` (pointer) |
| `Move` | `let y = x; // x moved` | `y = x` (no move concept) | `y := x` |
| `Result<T, E>` | `Result<T, E>` | `T` + raises Exception | `(T, error)` |
| `Propagate (?)` | `?` | try/except (implicit) | `if err != nil { return }` |
| `Array<T>` | `Vec<T>` | `list[T]` | `[]T` |
| `Struct` | `struct Name { ... }` | `class Name: ...` | `type Name struct { ... }` |
| `Drop` | implicit (scope exit) | implicit (GC) | `defer close()` |
| `FileHandle` | `File` | `open()` context manager | `*os.File` + `defer f.Close()` |

### Why This Matters for Phase 9 Success

The `--lang` projections directly improve LLM success rates:
1. **Few-shot examples in Rust syntax** are more effective than JSON-LD examples — LLMs are trained on millions of Rust files, not on DUUMBI JSON-LD
2. **The system prompt can say:** "Generate JSON-LD that, when projected as Rust, would look like this: `fn sort(arr: &mut Vec<i64>) { ... }`"
3. **Human review** of AI-generated graphs becomes practical — instead of reading JSON-LD nodes, the developer reads Rust/Python/Go and confirms it matches intent


---

## Addendum: Two-Phase Generation Strategy

> Added 2026-03-14.

The single biggest risk to Phase 9's success rate is LLM difficulty generating ownership-correct graphs. Asking an LLM to simultaneously handle business logic AND ownership/lifetime rules in one pass is like asking a junior developer to write correct Rust on the first try — unlikely.

### The Problem

Current single-pass approach:
```
Intent → LLM → JSON-LD graph (logic + ownership + error handling)
                    ↓
              Validation (E001–E035)
                    ↓
              Pass or retry (entire graph)
```

If the LLM gets the logic right but the ownership wrong → retry regenerates everything, including the correct logic. Wasteful and fragile.

### The Solution: Two-Phase Generation

```
Phase 1: Logic Generation (no ownership)
    Intent → LLM → JSON-LD graph (plain types: i64, f64, bool, String-as-value)
                        ↓
                  Basic validation (E001–E019, no ownership checks)
                        ↓
                  Logic is correct? → proceed to Phase 2
                  Logic is wrong? → retry Phase 1 only

Phase 2: Ownership Annotation
    Logic graph → LLM → ownership-annotated graph (Alloc, Borrow, Move, Drop added)
                            ↓
                      Full validation (E001–E035, including ownership)
                            ↓
                      Ownership correct? → compile
                      Ownership wrong? → retry Phase 2 only (logic preserved)
```

### Benefits

1. **Separation of concerns:** The logic LLM call focuses ONLY on correctness. The ownership LLM call focuses ONLY on memory safety.
2. **Targeted retries:** If ownership fails, only the ownership pass retries. The correct logic is preserved.
3. **Better prompts:** Phase 1 prompt can be simpler ("generate a sort function"). Phase 2 prompt is specialized ("add ownership annotations to this existing graph").
4. **Higher success rate:** Each pass has a higher individual success rate because it solves a simpler problem.
5. **Cost efficiency:** Failed Phase 2 retries are cheaper than regenerating the entire graph.

### Implementation

- [ ] `TwoPhaseGenerator` in `src/orchestrator.rs` (or new `src/generation/two_phase.rs`)
- [ ] Phase 1 system prompt: focused on logic, types as values (no `&`, no `mut`, no `Drop`)
- [ ] Phase 1 validation: run E001–E019 only (skip ownership E020–E035)
- [ ] Phase 2 system prompt: "Given this logically correct graph, add ownership annotations"
  - Include ownership rules summary
  - Include the Rust projection (`--lang rust`) of the Phase 1 output as context
  - Include 3–5 few-shot examples of ownership annotation transforms
- [ ] Phase 2 validation: run full E001–E035
- [ ] Retry isolation: Phase 1 failures retry Phase 1; Phase 2 failures retry Phase 2
- [ ] Fallback: if Phase 2 fails after max retries → present the logic-correct but ownership-unsafe graph to the user with a warning ("logic is correct but ownership annotations need manual review")

### Config

```toml
# config.toml
[generation]
strategy = "two-phase"    # "single-pass" | "two-phase"
phase1-max-retries = 3    # logic generation retries
phase2-max-retries = 5    # ownership annotation retries (more tries — this is harder)
```

### Impact on Phase 9 Benchmarks

The benchmark runner (M9-ITER) should track Phase 1 vs Phase 2 success rates separately:
```json
{
  "showcase": "sorting",
  "phase1_success_rate": 0.98,   // logic almost always correct
  "phase2_success_rate": 0.92,   // ownership harder
  "overall_success_rate": 0.90   // combined
}
```

This decomposition will reveal whether the bottleneck is logic or ownership — critical data for prompt engineering optimization.


---

## Addendum: LLM-Friendly Validator Error Messages

> Added 2026-03-14.

The E020–E035 error messages are consumed by two audiences: humans (CLI output) and LLMs (retry loop context). Optimizing for LLM consumption directly increases Phase 9 success rates.

### Current Error Format (Phase 1–8, Human-Oriented)

```json
{"code": "E004", "message": "Orphan reference: node duumbi:main/entry/2 references non-existent duumbi:main/entry/99", "nodeId": "duumbi:main/entry/2"}
```

### Enhanced Error Format (Phase 9a+, Human + LLM Oriented)

```json
{
  "code": "E022",
  "severity": "error",
  "message": "Borrow exclusivity violation: value 'duumbi:sort/data/entry/0' has an active shared borrow at 'duumbi:sort/data/entry/2' while a mutable borrow is attempted at 'duumbi:sort/data/entry/5'",
  "nodeId": "duumbi:sort/data/entry/5",
  "conflicting_node": "duumbi:sort/data/entry/2",
  "rule": "A value may have either N shared borrows OR 1 mutable borrow, never both simultaneously",
  "suggested_fix": {
    "strategy": "end_shared_borrow_before_mut",
    "description": "Ensure the shared borrow at entry/2 is no longer used before creating the mutable borrow at entry/5. Either reorder operations or introduce a new Block scope.",
    "example_patch": {
      "action": "insert_block_boundary",
      "before_node": "duumbi:sort/data/entry/5",
      "rationale": "A new Block scope will end the shared borrow's lifetime before the mutable borrow begins"
    }
  },
  "context": {
    "owner": "duumbi:sort/data/entry/0",
    "owner_type": "Array<i64>",
    "active_borrows": [
      {"nodeId": "duumbi:sort/data/entry/2", "kind": "shared", "lifetime": "entry"}
    ],
    "attempted_borrow": {"nodeId": "duumbi:sort/data/entry/5", "kind": "mutable"}
  }
}
```

### What's New for LLM Consumption

| Field | Purpose | LLM Benefit |
|---|---|---|
| `rule` | The exact ownership/error rule in plain English | LLM can include the rule in its correction reasoning |
| `suggested_fix.strategy` | Named fix strategy (enum) | LLM can pattern-match on strategy name |
| `suggested_fix.description` | Human + LLM readable explanation | LLM uses this as a mini-prompt for the fix |
| `suggested_fix.example_patch` | Concrete fix action | LLM can directly implement this specific fix |
| `context` | Ownership state at error point | LLM sees what's owned, borrowed, and by whom |

### Fix Strategy Catalog

Every E020–E035 error has 1–3 named fix strategies:

| Error | Strategies |
|---|---|
| E021 (use-after-move) | `clone_before_move`, `reorder_operations`, `use_borrow_instead` |
| E022 (borrow exclusivity) | `end_shared_borrow_before_mut`, `clone_value`, `split_into_scopes` |
| E023 (borrow outlives owner) | `shorten_borrow_lifetime`, `extend_owner_scope`, `return_owned_instead` |
| E024 (drop not reached) | `add_drop_on_all_paths`, `restructure_branches` |
| E030 (unhandled Result) | `add_match_result`, `add_propagate`, `add_unwrap_or` |

### Implementation
- [ ] Extend `ValidationError` struct with `rule`, `suggested_fix`, `context` fields
- [ ] Implement fix strategy catalog: mapping from (error_code, graph_context) → named strategies
- [ ] JSON serialization of enhanced errors (backward compatible — old fields preserved)
- [ ] The retry loop prompt includes the full enhanced error JSON, not just the message string
- [ ] Few-shot examples in prompt: "When you see E022 with strategy `end_shared_borrow_before_mut`, do this: ..."


---

## Addendum: FFI — Foreign Function Interface (Unsafe)

> Added 2026-03-15.

The empty ecosystem problem has a pragmatic solution: if DUUMBI can call existing C/Rust libraries via FFI, the entire C ecosystem becomes available. This eliminates the need to reimplement everything in JSON-LD graph — instead, wrap existing proven libraries.

### The Model: Unsafe FFI (Rust-Style)

Like Rust's `unsafe` blocks, FFI calls in DUUMBI are explicitly marked as **unsafe**. The schema validator skips ownership checks on values crossing the FFI boundary. The developer takes responsibility for memory safety at the interface.

```jsonld
{
  "@type": "duumbi:ExternFunction",
  "@id": "duumbi:ffi/zlib/compress",
  "duumbi:name": "compress",
  "duumbi:externLib": "libz",
  "duumbi:externSymbol": "compress",
  "duumbi:params": [
    {"name": "dest", "type": "*mut u8", "duumbi:unsafe": true},
    {"name": "destLen", "type": "*mut i64", "duumbi:unsafe": true},
    {"name": "source", "type": "*const u8", "duumbi:unsafe": true},
    {"name": "sourceLen", "type": "i64"}
  ],
  "duumbi:returnType": "i64",
  "duumbi:unsafe": true
}
```

### Schema Extensions

#### New Node Types
- [ ] `duumbi:ExternFunction` — declares a function from an external C/Rust library
  - `duumbi:externLib`: library name (e.g., "libz", "libssl", "libsqlite3")
  - `duumbi:externSymbol`: the C symbol name to link against
  - `duumbi:params`: parameter list with types (may include raw pointers)
  - `duumbi:returnType`: return type (may be raw pointer)
  - `duumbi:unsafe`: must be `true` — explicit acknowledgment
- [ ] `duumbi:UnsafeBlock` — a Block where ownership validation is relaxed
  - All ops inside an UnsafeBlock skip E020–E029 checks
  - The block boundary restores normal validation
  - Analogous to Rust's `unsafe { ... }`
- [ ] `duumbi:RawPointer` — type for raw C pointers (`*const T`, `*mut T`)
  - Not subject to ownership tracking (no Alloc/Drop/Borrow)
  - Can only exist inside `UnsafeBlock`
  - Validator: E050 — `RawPointer` used outside `UnsafeBlock`

#### New Validator Rules

| Rule | Error Code | Description |
|---|---|---|
| Unsafe required | E050 | `RawPointer` type or `ExternFunction` call used outside an `UnsafeBlock` |
| Extern lib declared | E051 | `ExternFunction` references a library not listed in `config.toml [extern-libs]` |
| Symbol mismatch | E052 | `ExternFunction` param/return types inconsistent with declared signature |
| Unsafe boundary | E053 | An owned value passed into an `UnsafeBlock` must be explicitly moved or borrowed — no implicit crossing |

#### Config

```toml
# config.toml
[extern-libs]
libz = { link = "-lz" }                           # system library
libsqlite3 = { link = "-lsqlite3" }               # system library
mylib = { link = "-L/path/to -lmylib" }           # custom library
rustlib = { link = "/path/to/librust.a" }          # static Rust library
```

The `link` value is appended to the `cc` linker invocation. This is the same pattern already used for the C runtime shim.

### Cranelift Codegen

- [ ] `ExternFunction` → Cranelift: `declare_function` with `Linkage::Import` (external symbol)
- [ ] `UnsafeBlock` → Cranelift: no special codegen (safety is compile-time only)
- [ ] `RawPointer` → Cranelift: i64 (pointer-sized integer, same as current approach)
- [ ] Linker: `cc` invocation extended with `-l` flags from `config.toml [extern-libs]`

### FFI Wrapper Pattern (Recommended Usage)

While the FFI itself is unsafe, the recommended pattern is to write a **safe DUUMBI wrapper** around the extern function:

```
Unsafe layer (hidden):
    duumbi:ExternFunction "compress" — raw C call, unsafe

Safe wrapper (user-facing):
    duumbi:Function "stdlib/zlib/compress"
        — takes &Array<u8> (safe, borrowed)
        — inside UnsafeBlock: converts to raw pointer, calls extern
        — outside UnsafeBlock: wraps result in owned Array<u8>
        — returns Result<Array<u8>, String>
```

The user never touches unsafe code. The stdlib provides safe wrappers. This is identical to how Rust's standard library wraps libc calls.

### Impact on Ecosystem

FFI transforms the "empty ecosystem" problem:

| Without FFI | With FFI |
|---|---|
| Need to implement everything in JSON-LD | Wrap existing C libraries |
| No compression | `libz` via FFI → `stdlib/zlib.jsonld` |
| No regex | `libpcre` via FFI → `stdlib/regex.jsonld` |
| No crypto | `libssl` via FFI → `stdlib/crypto.jsonld` |
| No image processing | `libpng`/`libjpeg` via FFI → `stdlib/image.jsonld` |
| No serialization | `libyaml`, `libtoml` via FFI → `stdlib/yaml.jsonld`, `stdlib/toml.jsonld` |

The stdlib grows by wrapping, not reimplementing. Each wrapper is a thin DUUMBI module with an UnsafeBlock inside and a safe API outside.

### Implementation Tasks
- [ ] `duumbi:ExternFunction` schema type + JSON-LD validation
- [ ] `duumbi:UnsafeBlock` schema type + validator bypass logic
- [ ] `duumbi:RawPointer` type + E050 enforcement
- [ ] E050–E053 validator rules
- [ ] `[extern-libs]` config.toml section + linker flag injection
- [ ] Cranelift: `declare_function` with `Linkage::Import` for extern symbols
- [ ] `duumbi check --unsafe-report`: list all UnsafeBlocks and ExternFunctions in the project (audit)
- [ ] Example: `stdlib/zlib.jsonld` — compress/decompress wrapper around libz
- [ ] Documentation: "FFI Guide" in mdBook — how to wrap a C library

### Expected Files

```
src/types/
├── ffi.rs           — ExternFunction, UnsafeBlock, RawPointer types
└── unsafe_check.rs  — E050–E053 validator rules
src/codegen/
└── ffi.rs           — extern function declaration + linkage
stdlib/
├── zlib.jsonld      — example FFI wrapper (libz compress/decompress)
└── ... (future FFI wrappers)
```

### Risk Assessment

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| FFI misuse leads to memory corruption | Medium | High | `duumbi check --unsafe-report` audit tool; UnsafeBlock count as code quality metric |
| C library ABI mismatch (wrong types) | Medium | Critical | E052 type checking; require explicit type declarations |
| Cross-platform link failures (library not found) | Medium | Medium | `duumbi check --link-verify`: test linker flags before compilation |
| LLMs generating unnecessary UnsafeBlocks | Medium | Medium | Prompt engineering: "only use UnsafeBlock for FFI, never for regular code" |


---

## Addendum: Formal Verification Foundation — Pre/Postcondition Fields

> Added 2026-03-15.

This addendum lays the groundwork for formal verification (Phase 15) by adding optional specification fields to the JSON-LD schema. These fields carry zero implementation cost in Phase 9a — they are optional string properties that the schema validator stores but does not (yet) verify. However, they enable three things immediately:

1. **LLMs can generate specifications alongside code.** The intent pipeline can ask: "Also generate preconditions and postconditions for each function."
2. **Developers can write specifications.** The fields are visible in `duumbi describe --verbose` and in the Markdown view.
3. **Phase 15 verification builds on existing data.** When formal verification arrives, projects already have specifications in place.

### Schema Extensions

#### Function-Level Specifications

```jsonld
{
  "@type": "duumbi:Function",
  "@id": "duumbi:math/abs",
  "duumbi:name": "abs",
  "duumbi:params": [{"name": "x", "type": "i64"}],
  "duumbi:returnType": "i64",
  "duumbi:precondition": "true",
  "duumbi:postcondition": "result >= 0 && (result == x || result == -x)",
  "duumbi:verificationStatus": "unverified"
}
```

#### Loop Invariants

```jsonld
{
  "@type": "duumbi:Block",
  "@id": "duumbi:sort/merge/loop",
  "duumbi:isLoop": true,
  "duumbi:loopInvariant": "forall i in 0..k: arr[i] <= arr[i+1]",
  "duumbi:loopVariant": "n - k"
}
```

#### New Properties (All Optional)

| Property | Applies To | Type | Description |
|---|---|---|---|
| `duumbi:precondition` | Function | String (logical expression) | What must be true before the function executes |
| `duumbi:postcondition` | Function | String (logical expression) | What must be true after the function returns |
| `duumbi:loopInvariant` | Block (loop) | String (logical expression) | What remains true on every iteration |
| `duumbi:loopVariant` | Block (loop) | String (expression) | Decreasing quantity that proves termination |
| `duumbi:verificationStatus` | Function | Enum: `unverified` / `verified` / `failed` | Status of formal verification (set by Phase 15 verifier) |
| `duumbi:proofHash` | Function | String (SHA-256) | Hash of the last successful verification proof |
| `duumbi:assertionCount` | Function | i64 | Number of verification conditions generated (set by Phase 15) |

#### Expression Language for Specifications

Specifications are written in a simple expression language (not full first-order logic — that's Phase 15 territory):

```
// Supported in Phase 9a (stored as strings, not parsed):
result >= 0                          // comparison
x > 0 && y > 0                      // logical AND
x == 0 || y == 0                    // logical OR
forall i in 0..n: arr[i] >= 0       // universal quantifier (array)
exists i in 0..n: arr[i] == target  // existential quantifier
old(x) + 1 == result                // old() refers to precondition value
arr.length == old(arr.length) + 1   // after push, length increased by 1
```

Phase 9a does NOT parse or validate these expressions. They are stored as opaque strings. Phase 15 will define the formal grammar and implement the parser + verification condition generator.

#### `duumbi describe` Integration

The `--verbose` flag (already planned) shows specifications:

```
$ duumbi describe --verbose
fn abs(x: i64) -> i64
  requires: true
  ensures:  result >= 0 && (result == x || result == -x)
  status:   unverified
```

The `--lang rust` projection shows them as doc comments:

```rust
/// # Contract
/// - requires: true
/// - ensures: result >= 0 && (result == x || result == -x)
fn abs(x: i64) -> i64 {
    ...
}
```

### Implementation Tasks

- [ ] Add `duumbi:precondition`, `duumbi:postcondition`, `duumbi:loopInvariant`, `duumbi:loopVariant` as optional string fields in `core.schema.json`
- [ ] Add `duumbi:verificationStatus` enum field (default: `unverified`)
- [ ] Add `duumbi:proofHash`, `duumbi:assertionCount` optional fields
- [ ] Update `duumbi describe --verbose` to display pre/postconditions
- [ ] Update `--lang rust/python/go` projections to include specifications as comments
- [ ] Update intent YAML spec format: optional `specifications` section per function
- [ ] LLM prompt extension: "Also generate preconditions and postconditions" (optional, configurable)

### Impact on Phase 9 Benchmarks

The benchmark runner can optionally track specification generation:
```json
{
  "showcase": "sorting",
  "specs_generated": true,
  "preconditions": 3,
  "postconditions": 3,
  "loop_invariants": 1,
  "specs_quality": "human_review_needed"
}
```

This is informational only — specifications do not affect the ≥95% pass rate criterion. But tracking them provides data for Phase 15 feasibility assessment.
