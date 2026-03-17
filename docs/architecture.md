# DUUMBI — Architecture Reference

> Companion to CLAUDE.md. Claude Code reads this when working on structural,
> cross-component, or pipeline-level tasks.

## Core thesis

Program logic is stored as a **typed semantic graph** (JSON-LD), not as text
files. AI generates and mutates the graph. The toolchain validates every
mutation before compilation. Syntax errors are structurally impossible.

**Semantic Fixed Point:** a graph is compilable only when it passes schema
validation, type checking, and all tests pass.

---

## Component map

```
.jsonld files
    │
    ▼
JSON-LD Parser          (serde_json → typed AST)
    │
    ▼
Semantic Graph          (petgraph::StableGraph — single source of truth)
    │         │
    ▼         ▼
Schema        Cranelift Compiler    (graph nodes → Cranelift IR → .o)
Validator         │
    │             ▼
    │         Linker (cc)           ($CC env or `cc` fallback)
    │             │
    ▼             ▼
Error JSONL   Native Binary

Phase 2 (implemented):
  duumbi add "..."
      │
      ▼
  LlmClient (Anthropic / OpenAI)
      │   tool_use / function_calling API
      ▼
  Vec<PatchOp>  (6 variants: AddFunction, AddBlock, AddOp,
      │          ModifyOp, RemoveNode, SetEdge)
      ▼
  apply_patch()  →  JSON-LD Value  →  parse+build+validate  →  write to disk
      │                                                             │
      ▼                                                             ▼
  Retry (max 1)                                          .duumbi/history/ snapshot

Phase 3 additions:
  Telemetry Engine  →  traceId injection  →  Web Visualizer (WASM + axum)

Phase 5 (implemented):
  duumbi intent create "<description>"
      │
      ▼
  LlmClient  →  IntentSpec YAML  (.duumbi/intents/<slug>.yaml)
      │          (acceptance_criteria, modules, test_cases)
      ▼
  duumbi intent execute <slug>
      │
      ▼
  Coordinator::decompose()  →  Vec<Task>  (rule-based ordering)
      │   CreateModule tasks first, ModifyMain last
      ▼
  orchestrator::mutate_streaming()  ×  task count  (3-step retry)
      │
      ▼
  Verifier::run_tests()  →  TestReport
      │   generates wrapper main.jsonld, compiles, runs, checks exit code
      ▼
  archive_intent()  →  .duumbi/intents/history/<slug>.yaml
```

---

## Data formats

| Format | Role | Location |
|--------|------|----------|
| `.jsonld` | Source of truth for program logic | `.duumbi/graph/` |
| `core.schema.json` | JSON Schema for Op node validation | `.duumbi/schema/` |
| `.o` | Cranelift object file output | `.duumbi/build/` |
| `traces.jsonl` | Runtime traceId → nodeId mapping | `.duumbi/telemetry/` |
| `config.toml` | Workspace, LLM, registries, dependencies, vendor | `.duumbi/` |
| `deps.lock` | Lockfile v1 with integrity hashes | `.duumbi/` |
| `manifest.toml` | Module metadata (name, version, exports) | cache/vendor entries |
| `{N:06}.jsonld` | Undo history snapshots | `.duumbi/history/` |
| `<slug>.yaml` | Active intent specs | `.duumbi/intents/` |
| `<slug>.yaml` | Archived completed/failed intents | `.duumbi/intents/history/` |
| `credentials.toml` | Registry auth tokens (0600 perms) | `~/.duumbi/` |
| `.tar.gz` | Packaged module archive | publish pipeline |

**JSON-LD namespace:** `https://duumbi.dev/ns/core#` (prefix: `duumbi:`)

**nodeId format:** `duumbi:<module>/<function>/<block>/<index>`
Example: `duumbi:main/main/entry/2`

---

## Build pipeline

```
.jsonld files  →  parse  →  StableGraph  →  validate  →  Cranelift IR  →  output.o
                                                                              │
                                            cc output.o duumbi_runtime.o -o output
```

**Phase 0 kill criterion:** `add(3, 5)` → binary prints `8`, exits with code 8. ✓
**Phase 1 kill criterion:** External dev installs and runs fibonacci in < 10 min. ✓
**Phase 2 kill criterion:** > 70% correct on 20-command AI benchmark (mock: 20/20). ✓

**Full Op set:**

| duumbi: Op | Cranelift IR | Phase |
|------------|--------------|-------|
| `Const` (i64) | `iconst` | 0 |
| `ConstF64` (f64) | `f64const` | 1 |
| `ConstBool` (bool) | `iconst` (i8) | 1 |
| `Add` | `iadd` / `fadd` | 0 |
| `Sub` | `isub` / `fsub` | 0 |
| `Mul` | `imul` / `fmul` | 0 |
| `Div` | `sdiv` / `fdiv` | 0 |
| `Compare` | `icmp` / `fcmp` | 1 |
| `Branch` | `brif` | 1 |
| `Call` | `call` | 1 |
| `Load` | `use_var` | 1 |
| `Store` | `def_var` | 1 |
| `Print` | `call duumbi_print_*` | 0 |
| `Return` | `return` | 0 |
| `ConstString` (string) | `global_value` + `call duumbi_string_new` | 9a-1 |
| `PrintString` | `call duumbi_print_string` | 9a-1 |
| `StringConcat` | `call duumbi_string_concat` | 9a-1 |
| `StringEquals` | `call duumbi_string_equals` | 9a-1 |
| `StringCompare` | `call duumbi_string_compare` + `icmp` | 9a-1 |
| `StringLength` | `call duumbi_string_len` | 9a-1 |
| `StringSlice` | `call duumbi_string_slice` | 9a-1 |
| `StringContains` | `call duumbi_string_contains` | 9a-1 |
| `StringFind` | `call duumbi_string_find` | 9a-1 |
| `StringFromI64` | `call duumbi_string_from_i64` | 9a-1 |
| `ArrayNew` | `call duumbi_array_new` | 9a-1 |
| `ArrayPush` | `call duumbi_array_push` | 9a-1 |
| `ArrayGet` | `call duumbi_array_get` | 9a-1 |
| `ArraySet` | `call duumbi_array_set` | 9a-1 |
| `ArrayLength` | `call duumbi_array_len` | 9a-1 |
| `StructNew` | `call duumbi_struct_new` | 9a-1 |
| `FieldGet` | `call duumbi_struct_field_get` | 9a-1 |
| `FieldSet` | `call duumbi_struct_field_set` | 9a-1 |
| `Alloc` | `call duumbi_alloc` (type-specific) | 9a-2 |
| `Move` | pointer copy (no runtime cost) | 9a-2 |
| `Borrow` | pointer copy (no runtime cost) | 9a-2 |
| `BorrowMut` | pointer copy (no runtime cost) | 9a-2 |
| `Drop` | `call duumbi_*_free` (type-specific) | 9a-2 |
| `ResultOk` | `call duumbi_result_new_ok` | 9a-3 |
| `ResultErr` | `call duumbi_result_new_err` | 9a-3 |
| `ResultIsOk` | `call duumbi_result_is_ok` | 9a-3 |
| `ResultUnwrap` | `call duumbi_result_unwrap` | 9a-3 |
| `ResultUnwrapErr` | `call duumbi_result_unwrap_err` | 9a-3 |
| `OptionSome` | `call duumbi_option_new_some` | 9a-3 |
| `OptionNone` | `call duumbi_option_new_none` | 9a-3 |
| `OptionIsSome` | `call duumbi_option_is_some` | 9a-3 |
| `OptionUnwrap` | `call duumbi_option_unwrap` | 9a-3 |
| `Match` | `brif` on discriminant → ok/err blocks | 9a-3 |

**Types:** `i64`, `f64`, `bool`, `void`, `string`, `array<T>`, `struct<Name>`, `&T`, `&mut T`, `result<T,E>`, `option<T>`

---

## Linker strategy

1. Check `$CC` env var
2. Fall back to `cc` on PATH
3. Command: `cc output.o duumbi_runtime.o -o output -lc`

`duumbi_runtime.c` provides: `duumbi_print_i64(int64_t)`,
`duumbi_print_f64(double)` (Phase 1), `duumbi_print_bool(int8_t)` (Phase 1).

---

## AI mutation pipeline (Phase 2)

`duumbi add "<request>"` runs the following loop:

1. Load `.duumbi/config.toml` → `LlmClient` (Anthropic or OpenAI)
2. Read `.duumbi/graph/main.jsonld` as `serde_json::Value`
3. Send `SYSTEM_PROMPT + graph_json + user_request` to LLM with 6 tools
4. LLM responds with one or more tool calls → deserialized as `Vec<PatchOp>`
5. `apply_patch(source, patch)` — clone source, apply all ops (all-or-nothing)
6. `parse_jsonld → build_graph → validate` on the patched value
7. On failure: retry once with error message appended to user prompt
8. Show `describe_changes` diff summary, ask for confirmation (unless `--yes`)
9. `save_snapshot` to `.duumbi/history/{N:06}.jsonld`
10. Write patched graph to `.duumbi/graph/main.jsonld`

**GraphPatch operations** (`src/patch.rs`, serde tag: `"kind"`):

| Kind | Description |
|------|-------------|
| `add_function` | Append a complete function JSON-LD object |
| `add_block` | Append a block to a function by `function_id` |
| `add_op` | Append an op to a block by `block_id` |
| `modify_op` | Set a field on any node by `node_id` |
| `remove_node` | Remove a node (op/block/function) by `node_id` |
| `set_edge` | Set an `@id` reference field on a node |

**Undo:** `duumbi undo` restores the latest `.duumbi/history/*.jsonld` snapshot
and removes it (LIFO stack). `snapshot_count()` reports remaining undo depth.

---

## Error format (JSONL to stdout)

```json
{
  "level": "error",
  "code": "E001",
  "message": "Type mismatch: Add expects matching operand types",
  "nodeId": "duumbi:main/main/entry/2",
  "file": "graph/main.jsonld",
  "details": { "expected": "i64", "found": "f64", "field": "duumbi:left" }
}
```

Error codes: E001 type mismatch · E002 unknown Op · E003 missing field ·
E004 orphan reference · E005 duplicate @id · E006 no entry function ·
E007 cycle · E008 link failed · E009 schema invalid ·
E010 unresolved cross-module ref · E011 dependency not found ·
E012 module conflict · E013 registry unreachable · E014 auth failed ·
E015 integrity mismatch · E016 version not found ·
E020 single owner · E021 use-after-move · E022 borrow exclusivity ·
E023 lifetime exceeded · E024 drop incomplete · E025 double free ·
E026 dangling reference · E027 move while borrowed ·
E028 lifetime param missing · E029 return lifetime mismatch ·
E030 unhandled Result · E031 unhandled Option ·
E032 non-exhaustive match · E033 Result/Option type param mismatch ·
E034 unwrap without check · E035 wrong payload type

---

## Phase roadmap

| Phase | Goal | Kill criterion |
|-------|------|----------------|
| 0 | JSON-LD → native binary | `add(3,5)` prints `8` ✓ |
| 1 | Usable CLI | External dev installs + runs in < 10 min ✓ |
| 2 | AI graph mutation | > 70% correct on 20-cmd benchmark ✓ |
| 3 | Web visualizer | 3/3 devs confirm faster than raw JSON-LD ✓ |
| 4 | Interactive CLI + module system | `abs(-7) = 7` (init → 2-module → binary) ✓ |
| 5 | Intent-Driven Development | Verifier passes `double(21)=42` via intent pipeline ✓ |
| 6 | DUUMBI Studio | Leptos SSR web platform with graph visualization ✓ |
| 7 | Registry & Distribution | Module packaging, publish, install, lockfile v1 ✓ |
| 8 | Registry Auth | GitHub OAuth2, JWT sessions, device code flow ✓ |
| 9a-1 | Heap Types & Runtime | String concat+print, Array push+get, Struct field access ✓ |
| 9a-2 | Ownership & Lifetimes | Alloc/Move/Borrow/Drop ops, &T/&mut T, E020–E029 validator ✓ |
| 9a-3 | Error Handling | Result/Option types, Match op, E030–E035 validator |
| 9A | Stdlib & Instruction Set | Math ops (Sqrt, Pow, Modulo), stdlib modules |
| 9B | Multi-LLM Providers | LlmProvider trait, Grok/OpenRouter, fallback chain |
| 9C | Benchmark & Showcases | 6 showcases, benchmark runner, CI integration |

Phases beyond MVP (10–15): Knowledge base, CLI UX, Agent swarm, Self-healing, Marketing, Formal verification.

## Intent system (Phase 5)

**CLI commands:**
- `duumbi intent create "<description>"` — LLM generates structured YAML spec
- `duumbi intent review [name]` — list or show intent details; `--edit` opens $EDITOR
- `duumbi intent execute <name>` — decompose → mutate → verify → archive
- `duumbi intent status [name]` — show active intent status

**REPL slash commands:** `/intent`, `/intent create <desc>`, `/intent review [name]`,
`/intent execute <name>`, `/intent status [name]`

**Intent YAML layout (`.duumbi/intents/<slug>.yaml`):**
```yaml
intent: "Build a calculator"
version: 1
status: Pending
acceptance_criteria:
  - "add(a, b) returns a+b for i64"
modules:
  create: ["calculator/ops"]
  modify: ["app/main"]
test_cases:
  - name: basic_add
    function: add
    args: [3, 5]
    expected_return: 8
```

**Coordinator task order:** CreateModule → AddFunction (non-main) → ModifyMain

**Verifier strategy:** generates a temp `main.jsonld` that calls the target function,
compiles the full workspace, runs the binary, checks exit code against `expected_return`.

## Registry & Distribution (Phase 7)

**Architecture:** Registry server is a separate project (`duumbi-registry` repo),
infrastructure lives in `duumbi-infra`. This repo contains the **client-side** only:
packaging, publishing, downloading, authentication, and dependency resolution.

```
config.toml [registries]           ~/.duumbi/credentials.toml
         │                                   │
         ▼                                   ▼
  RegistryClient (reqwest + retry)  ←─── Bearer token
         │
    ┌────┴─────┬──────────┬──────────┐
    ▼          ▼          ▼          ▼
  search    publish     yank     download
    │          │          │          │
    │     pack_module()   │    .tar.gz → cache
    │     .tar.gz+hash    │          │
    ▼          ▼          ▼          ▼
  stdout   registry    registry   .duumbi/cache/@scope/name@ver/
           API POST    API DEL        │
                                      ▼
                               deps.lock (lockfile v1)
```

**config.toml v2 format:**
```toml
[workspace]
name = "myapp"
namespace = "myapp"
default-registry = "duumbi"

[registries]
duumbi = "https://registry.duumbi.dev"
company = "https://registry.acme.com"

[dependencies]
"@duumbi/stdlib-math" = "^1.0"
"@company/auth" = { version = "^3.0", registry = "company" }
"local-utils" = { path = "../shared/utils" }

[vendor]
strategy = "selective"
include = ["@company/*"]
```

**Dependency resolution pipeline (3-layer priority):**
1. **Workspace** — `.duumbi/graph/` (own source, highest priority)
2. **Vendor** — `.duumbi/vendor/@scope/name/graph/` (pinned, audited copies)
3. **Cache** — `.duumbi/cache/@scope/name@version/graph/` (downloaded)
4. Not found → E011

**Lockfile v1 (`deps.lock`):**
```toml
version = 1

[[dependencies]]
name = "@duumbi/stdlib-math"
version = "1.0.0"
source = "cache"
semantic_hash = "abc..."   # SHA-256 of canonicalized graph (id-independent)
integrity = "sha256-def..."  # SHA-256 of raw file bytes
resolved_path = ".duumbi/cache/@duumbi/stdlib-math@1.0.0/graph"
vendored = false
```

**Module package (.tar.gz) contents:**
- `manifest.toml` — name, version, description, exports.functions
- `graph/*.jsonld` — module graph files
- `CHECKSUM` — SHA-256 of each file

**Scope-level registry routing:**
`@scope/name` → registry named `scope` (e.g. `@company/auth` → `company` registry).
`@duumbi/*` always routes to `duumbi` registry. Falls back to `default-registry`.

**CLI commands:**
- `duumbi publish` — validate graph → pack .tar.gz → compute hash → upload
- `duumbi yank @scope/name@version` — mark version as yanked (still downloadable by lockfile)
- `duumbi deps install [--frozen]` — resolve + download → cache → lockfile
- `duumbi search <query>` — text search across configured registries

**Authentication:** Bearer tokens stored in `~/.duumbi/credentials.toml` (0600 perms).
`duumbi registry login <name>` prompts for token or accepts `--token` for CI.
