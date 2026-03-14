---
tags:
  - project/duumbi
  - milestone/phase-13
status: planned
github_milestone: ~
updated: 2026-03-14
---
# Phase 13 — Self-Healing & Telemetry ⏳

> **Kill Criterion:** The system (1) detects a runtime error, (2) identifies the exact JSON-LD nodeId via traceId back-mapping, (3) generates a valid corrective patch, (4) tests pass after applying the patch, (5) the diff is human-reviewable. Additionally: trace sampling keeps instrumented binary within 2× of uninstrumented performance.
> **Status:** ⏳ Planned — starts after Phase 12
> **Estimated duration:** 6–8 weeks (solo developer)

← Back: [[DUUMBI Roadmap Map]]

---

## Summary

Close the feedback loop between runtime behavior and graph structure. When a compiled binary fails at runtime, the system traces back to the exact JSON-LD node that caused the failure, dynamically spawns a Repair agent (via Phase 12's dynamic agent system), generates a corrective graph patch, validates it, and presents it for review or auto-applies it.

The self-healing loop incorporates an autoresearch-inspired autonomous improvement cycle: when a fix attempt fails, the system tries alternative strategies automatically, learning from each attempt.

**Hot-swap (replacing running binary modules without restart) is explicitly deferred to Phase 15+.** The Phase 13 repair cycle produces a new binary that requires restart. Rationale: hot-swap requires a module-level dynamic linking architecture that doesn't exist yet. The current single-binary compilation model (Cranelift → .o → cc → executable) has no module boundary at the binary level.

---

## Tasks

### M13-TEL: Telemetry v2 (OpenTelemetry)

#### Instrumentation Strategy

The core challenge: instrumenting every Op node creates unacceptable overhead. The solution is a **two-mode compilation** system:

| Mode | Flag | Instrumentation Level | Performance Impact | Use Case |
|---|---|---|---|---|
| **Release** | `duumbi build` (default) | None — no trace code emitted | 0% overhead | Production deployment |
| **Instrumented** | `duumbi build --trace` | Sampled spans at function + block boundaries | Target: <2× overhead | Debugging, self-healing |

**Key design decision:** Instrumentation is NOT per-Op-node. It is per-Function and per-Block. Rationale:
- Per-Op instrumentation (every Add, Compare, Load, Store) creates 10-100× overhead — unacceptable
- Per-Function + per-Block gives sufficient granularity: the back-mapping identifies which Block in which Function failed
- Within a Block, the error message + Op index narrows down to the exact node without needing a span per Op

#### Trace Sampling

Even in `--trace` mode, not every function call is traced:

```toml
# config.toml
[telemetry]
enabled = true
mode = "sampled"              # "all" | "sampled" | "off"
sample-rate = 0.1             # 10% of function calls get full spans
always-trace-functions = []   # list of function @ids that are always traced (100%)
export-format = "otlp-json"   # "otlp-json" | "otlp-grpc" | "jsonl"
export-endpoint = ""          # empty = local file only; URL = remote collector
```

Sampling implementation:
- [ ] At function entry: generate random u64, compare against threshold (sample-rate × u64::MAX)
- [ ] If sampled: emit span start, set thread-local flag for Block spans within this function
- [ ] If not sampled: skip all spans for this call (near-zero overhead — one comparison)
- [ ] `always-trace-functions`: override sampling for specific functions (always 100%)
- [ ] Cranelift codegen: the `--trace` flag adds conditional span emission at function prologue/epilogue and block entry/exit

#### Compile-Time Instrumentation (Cranelift Codegen)

- [ ] `--trace` flag: `duumbi build --trace` → `CraneliftBackend` emits additional trace code
- [ ] Function prologue: `call duumbi_trace_function_enter(function_id: i64, sample_decision: i8)`
- [ ] Function epilogue: `call duumbi_trace_function_exit(function_id: i64)`
- [ ] Block entry: `call duumbi_trace_block_enter(block_id: i64)` (only if function was sampled)
- [ ] Block exit: `call duumbi_trace_block_exit(block_id: i64)`
- [ ] C runtime shim additions for trace functions (see below)

#### Runtime Value Capture (Without DWARF Debug Info)

The review identified that "variable values at failure point" requires debug info. Instead of DWARF (complex, Cranelift support limited), DUUMBI uses **explicit value logging at strategic points:**

- [ ] **Panic handler instrumentation:** When `duumbi_panic` is called (from Result::Unwrap, ArrayGet OOB, etc.), the panic handler captures:
  - The function_id and block_id from thread-local trace state
  - The panic message (String)
  - The arguments that were passed to the failing function (captured at function entry if tracing is active)
- [ ] **Function argument snapshot:** In `--trace` mode, function entry logs argument values to a ring buffer:
  - `duumbi_trace_capture_arg(arg_index: i64, arg_type: i64, arg_value: i64)` — for i64/f64/bool
  - `duumbi_trace_capture_arg_string(arg_index: i64, string_ptr: *const StringHeader)` — for String
  - Ring buffer: last 64 function calls (fixed size, circular overwrite)
  - On panic: dump ring buffer to `.duumbi/telemetry/crash_dump.jsonl`
- [ ] **No DWARF dependency.** All debug information comes from the trace instrumentation, not from binary debug symbols. This is a deliberate architectural choice: DUUMBI's debug info is graph-level (nodeId), not source-level (line numbers).

#### Trace Storage & Export

- [ ] Local storage: `.duumbi/telemetry/traces.jsonl` (append-only, per-run)
- [ ] Crash dump: `.duumbi/telemetry/crash_dump.jsonl` (ring buffer snapshot on panic)
- [ ] OTLP JSON export: POST to configurable endpoint (for external collectors like Jaeger, Grafana Tempo)
- [ ] Trace → nodeId mapping table: compiled into the binary as a static array (function_id → `@id` string)

#### C Runtime Shim Extensions

```c
// Thread-local trace state
__thread int64_t  _duumbi_current_function_id;
__thread int64_t  _duumbi_current_block_id;
__thread int8_t   _duumbi_trace_active;  // 1 if this call was sampled

// Trace ring buffer (last 64 function entries)
typedef struct {
    int64_t function_id;
    int64_t arg_values[8];   // max 8 args, i64 representation
    int64_t timestamp_ns;
} TraceEntry;
TraceEntry _duumbi_trace_ring[64];
int _duumbi_trace_ring_pos = 0;

void duumbi_trace_function_enter(int64_t func_id, int8_t sampled);
void duumbi_trace_function_exit(int64_t func_id);
void duumbi_trace_block_enter(int64_t block_id);
void duumbi_trace_block_exit(int64_t block_id);
void duumbi_trace_capture_arg(int64_t index, int64_t type_tag, int64_t value);
void duumbi_trace_capture_arg_string(int64_t index, const void* str_ptr);
void duumbi_trace_dump_crash(const char* panic_msg); // called from duumbi_panic
```

#### Studio Telemetry Panel
- [ ] Visual trace explorer: timeline view of function calls with block-level detail
- [ ] Graph overlay: highlight traced functions/blocks on the C4 navigator
- [ ] Crash dump viewer: show captured argument values at failure point
- [ ] Trace filtering: by function, by module, by time range

---

### M13-HEAL: Self-Healing Loop

#### Back-Mapping

The mapping from runtime trace to graph node:

```
Runtime crash
    │
    ▼
Panic handler fires
    │ captures: function_id, block_id, panic_msg, arg ring buffer
    │
    ▼
Crash dump written to .duumbi/telemetry/crash_dump.jsonl
    │
    ▼
Back-mapping table lookup:
    │ function_id (i64) → function @id (e.g., "duumbi:parser/parse_number")
    │ block_id (i64)    → block @id (e.g., "duumbi:parser/parse_number/entry")
    │
    ▼
Graph node identified → load from .duumbi/graph/
    │
    ▼
Repair Agent context assembled:
    - Faulty block: all Op nodes in the identified block
    - Function context: the full function containing the block
    - Error message: panic string
    - Argument values: from crash dump ring buffer
    - Historical fixes: knowledge graph query for same function/block/error pattern
```

#### Repair Agent Spawning
- [ ] Uses Phase 12 dynamic agent system: spawns Repair agent from template
- [ ] Repair Agent receives context assembled by the back-mapping pipeline
- [ ] Context uses M10-CONTEXT algorithm (FixError task type) for graph subset selection
- [ ] Repair Agent has access to MCP tools: `graph.mutate`, `graph.validate`, `build.compile`, `build.run`

#### Repair → Rebuild → Test Cycle
- [ ] Repair Agent generates corrective graph patch
- [ ] Schema validation (including ownership E020-E029 and error handling E030-E035)
- [ ] `build.compile` with `--trace` flag (to verify the fix doesn't introduce new trace anomalies)
- [ ] Run existing test cases from intent spec
- [ ] If all pass → present diff for human review (or auto-apply with `--auto` flag)
- [ ] If tests fail → feed back to autoresearch cycle (M13-AUTOITER)

---

### M13-ANOMALY: Anomaly Detection with Adaptive Baselines

The review identified: how does the system know that 100ms is "slow" if it's never seen the program run? Solution: **learning baseline from initial runs.**

#### Baseline Calibration

```
First run with --trace:
    │
    ▼
Collect baseline metrics:
    - Per-function: median execution time, p95 execution time, call count
    - Per-module: total execution time, error count
    - Global: total runtime, peak memory
    │
    ▼
Store baseline: .duumbi/telemetry/baseline.json
    │
    ▼
Subsequent runs compared against baseline
```

- [ ] **Calibration run:** `duumbi run --trace --calibrate` — runs the binary, collects metrics, saves as baseline
- [ ] **Automatic calibration:** if no baseline exists, first `--trace` run automatically becomes the baseline
- [ ] **Baseline update:** `duumbi run --trace --recalibrate` — replaces baseline with current run metrics
- [ ] **Baseline format:**
  ```json
  {
    "calibrated_at": "2026-05-01T12:00:00Z",
    "functions": {
      "duumbi:parser/parse_number": {
        "median_ns": 1200,
        "p95_ns": 3500,
        "call_count": 1000
      }
    },
    "global": {
      "total_runtime_ms": 450,
      "peak_memory_bytes": 8388608
    }
  }
  ```

#### Alert Thresholds (Relative to Baseline)

```toml
# config.toml
[telemetry.alerts]
panic-count = 1                      # any panic = alert (absolute)
latency-multiplier = 3.0             # function >3× slower than baseline p95 = alert
error-rate-threshold = 0.05          # >5% error rate = alert (absolute, for Result::Err counting)
memory-growth-multiplier = 2.0       # >2× baseline peak memory = alert
```

- [ ] Thresholds are **relative to baseline** (latency, memory) or **absolute** (panic count, error rate)
- [ ] No baseline → no latency/memory alerts (only panic and error rate work without calibration)
- [ ] Alert output: structured JSONL to `.duumbi/telemetry/alerts.jsonl` + CLI notification + Studio panel

---

### M13-AUTOITER: Autonomous Improvement Cycle (Autoresearch-Inspired)
- [ ] If first repair attempt fails → try alternative strategy automatically
- [ ] Strategy rotation: different prompt approaches, different agent templates
- [ ] Maximum N attempts (configurable, default 5)
- [ ] Each attempt logged to knowledge graph: what was tried, what happened
- [ ] Successful repairs fed back into agent knowledge (Phase 12 learning loop)
- [ ] Failure patterns accumulated: "this type of error is not auto-fixable" → flag for human
- [ ] Cost control: repair attempts count against Phase 12 M12-COST budget-per-session

### M13-OPS: Ops Agent
- [ ] Running application monitoring via telemetry stream (reads traces.jsonl in real-time)
- [ ] Alert thresholds from M13-ANOMALY baseline comparison
- [ ] Alert → Repair Agent pipeline: automatic triage and repair initiation
- [ ] Health dashboard in Studio: green/yellow/red status per module
- [ ] `duumbi monitor` CLI command: starts Ops Agent in foreground, tails trace output

---

## Hot-Swap Decision

> **Explicit deferral to Phase 15+.**

The Original PRD (Section 7.2) envisions hot-swap: replacing a running module without restarting the binary. This requires:
1. Module-level dynamic linking (the binary is currently monolithic)
2. Symbol relocation at runtime
3. State transfer between old and new module versions
4. Cranelift JIT mode instead of AOT compilation

None of these exist in the current architecture. Implementing hot-swap would require fundamental changes to the compilation pipeline. The Phase 13 repair cycle instead produces a **new binary that requires restart** — this is the pragmatic choice that delivers 90% of the self-healing value without the architectural risk.

Hot-swap becomes feasible if/when:
- Phase 15a (LLVM backend) provides better JIT support
- The module system evolves to produce separate .so/.dylib per module
- A runtime process manager is introduced (like systemd-style reload)

---

## Self-Healing Flow (Updated)

```
Compiled Binary (running with --trace)
    │
    ▼
Sampled Traces → .duumbi/telemetry/traces.jsonl
    │             (sample rate configurable, default 10%)
    ▼
Baseline Comparison (M13-ANOMALY)
    │ First run → calibrate baseline
    │ Subsequent runs → compare against baseline
    ▼
Anomaly Detection
    │ panic (any) → ALERT
    │ function >3× baseline p95 → ALERT
    │ error rate >5% → ALERT
    │ memory >2× baseline → ALERT
    ▼
On panic: Crash Dump
    │ function_id + block_id + panic_msg + arg ring buffer
    │ → .duumbi/telemetry/crash_dump.jsonl
    ▼
Back-mapping: function_id → @id → JSON-LD node
    ▼
Dynamic Repair Agent spawned (Phase 12 system)
    │
    ├── Context: faulty block + error + arg values + history
    ├── Strategy 1: direct fix based on error message
    ├── Strategy 2: pattern match from knowledge graph
    └── Strategy 3: simplified reconstruction
    │
    ▼
Repair Patch → Validate → Rebuild (--trace) → Test
    │
    ├── Pass → Diff for review (or --auto apply) → NEW binary (restart required)
    │           └── Feed success into knowledge graph
    └── Fail → Try next strategy (max N, within cost budget)
                └── Feed failure into knowledge graph
```

---

## Dependencies

```
Phase 12 (Dynamic Agents)   ──→ Phase 13 (Repair Agent uses dynamic agent system + cost control)
Phase 10 (Knowledge Graph)  ──→ Phase 13 (repair history, baseline, crash dumps in knowledge graph)
Phase 9a (Type System)      ──→ Phase 13 (Result type error tracking, panic handler)
Phase 9 (Build Excellence)  ──→ Phase 13 (reliable build pipeline required)
Phase 3 (Telemetry v1)      ──→ Phase 13 (v2 extends existing traceId system)
```

See also: [[DUUMBI - Cranelift Dependency Policy]] — trace instrumentation is deeply Cranelift-coupled; version frozen during Phase 13 implementation.

## Expected Files

```
src/telemetry/
├── mod.rs          — telemetry v2 core + --trace flag handling
├── otel.rs         — OpenTelemetry integration (OTLP JSON export)
├── mapping.rs      — function_id/block_id → @id back-mapping table
├── sampling.rs     — sample rate logic + always-trace overrides
├── baseline.rs     — M13-ANOMALY: calibration + baseline storage
├── anomaly.rs      — anomaly detection engine (baseline comparison)
├── crash_dump.rs   — ring buffer dump on panic
└── ops.rs          — Ops Agent (monitoring + alerting + duumbi monitor)
src/healing/
├── mod.rs          — self-healing loop orchestration
├── repair.rs       — repair strategy execution
├── autoiter.rs     — autonomous iteration cycle
└── review.rs       — human review gate + diff presentation
src/codegen/
├── trace.rs        — --trace flag: conditional span emission in Cranelift codegen
└── mapping_table.rs — compile-time generation of function_id → @id mapping
src/runtime/
├── duumbi_runtime.c — (extended: trace functions, ring buffer, crash dump)
└── duumbi_runtime.h — (extended: trace struct definitions)
.duumbi/telemetry/
├── traces.jsonl      — sampled trace output
├── crash_dump.jsonl  — panic ring buffer dumps
├── baseline.json     — calibration baseline
└── alerts.jsonl      — anomaly alert log
```

## Monetization

Phase 13 strengthens the TEAM tier ($49/month/seat) with self-healing as a premium feature. The telemetry dashboard (Studio panel) is available in PRO tier ($19/month). The `--trace` compilation flag is free tier (open source).

---

## Related

- [[DUUMBI - Phase 12 - Dynamic Agent System & MCP]] — agent infrastructure + cost control
- [[DUUMBI - Phase 10 - Intelligent Context & Knowledge Graph]] — knowledge persistence
- [[DUUMBI - Phase 9a - Type System Completion]] — Result type, panic handler
- [[DUUMBI - Phase 9 - Build Excellence & Multi-LLM]] — build reliability
- [[DUUMBI - Cranelift Dependency Policy]] — version freeze during Phase 13
- [[DUUMBI - PRD]] — Section 7.2 "Self-Healing (Vision)"


---

## Addendum: Self-Healing CI Testability

> Added 2026-03-14.

The kill criterion requires "runtime error → nodeId → valid patch → tests green." This must be testable in CI, deterministically and reproducibly.

### Strategy: Intentionally Faulty Showcase Graphs

```
tests/self-healing/
├── faulty_div_by_zero/
│   ├── intent.yaml        — showcase spec (calculator with missing div-by-zero check)
│   ├── graph/              — pre-built graph that compiles but panics at runtime
│   ├── expected_error.json — { "error_type": "panic", "function": "calculator/div", "block": "entry" }
│   └── expected_fix.json   — { "patch_type": "add_result_check", "target_block": "entry" }
├── faulty_oob_access/
│   ├── intent.yaml        — array access with invalid index
│   ├── graph/              — pre-built graph that panics on ArrayGet OOB
│   ├── expected_error.json
│   └── expected_fix.json
└── faulty_use_after_move/
    ├── intent.yaml        — ownership violation that passes a lenient validator version
    ├── graph/              — graph that crashes at runtime due to use-after-move
    ├── expected_error.json
    └── expected_fix.json
```

### CI Job: Self-Healing Integration Test

```yaml
# .github/workflows/self-healing-test.yml
name: Self-Healing Integration
on: [push, weekly schedule]
jobs:
  test-repair:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build DUUMBI
        run: cargo build --release
      - name: Run faulty binary
        run: |
          cd tests/self-healing/faulty_div_by_zero
          duumbi build --trace
          duumbi run || true  # expect crash
      - name: Verify crash dump
        run: |
          # Check .duumbi/telemetry/crash_dump.jsonl exists
          # Verify function_id matches expected_error.json
      - name: Run repair
        run: |
          duumbi heal --auto  # triggers Repair Agent
      - name: Verify fix
        run: |
          duumbi build
          duumbi run  # should succeed now
          # Verify test cases pass
```

### Tasks
- [ ] Create 3 intentionally faulty showcase graphs (div-by-zero, OOB, use-after-move)
- [ ] Each faulty graph: pre-built (not LLM-generated) to ensure deterministic test
- [ ] `duumbi heal` CLI command: triggers the self-healing pipeline on the most recent crash dump
- [ ] CI job: build → run (crash) → heal → run (success) → verify
- [ ] Deterministic repair: for the pre-built faulty graphs, the Repair Agent should produce the same fix every time (mock LLM responses for CI)
- [ ] Mock LLM: `DUUMBI_LLM_MOCK=true` env var → uses pre-recorded repair responses from `tests/self-healing/*/mock_response.json`
