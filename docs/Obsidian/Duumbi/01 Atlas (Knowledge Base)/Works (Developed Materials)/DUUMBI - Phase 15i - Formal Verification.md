---
tags:
  - project/duumbi
  - milestone/phase-15
  - research/formal-verification
status: planned
github_milestone: ~
created: 2026-03-15
updated: 2026-03-15
---
# Phase 15i — Formal Verification (Research Phase) ⏳

> **Kill Criterion:** Given a function with pre/postconditions, `duumbi verify` generates verification conditions, feeds them to Z3, and either (a) proves correctness (returns "verified" + proof certificate) or (b) returns a concrete counterexample showing inputs that violate the postcondition. Demonstrated on at least 3 stdlib functions (abs, max, array sort invariant).
> **Status:** ⏳ Research — long-term vision, depends on Phase 9a specification foundation
> **Estimated duration:** Research phase (not time-boxed)

← Back: [[DUUMBI Roadmap Map]]

---

## Thesis

Dijkstra's program correctness vision (1976) has seen limited practical adoption because the gap between source code and formal specification is too wide. DUUMBI's semantic graph representation **closes this gap**: the program IS structured data, and the specifications are properties of the same data. There is no parsing, no AST construction, no intermediate representation loss — the graph nodes and their pre/postconditions exist at the same level of abstraction.

**The claim:** Formal verification on JSON-LD semantic graphs is structurally easier than on textual source code, because the verification condition generator operates directly on typed, structured graph nodes rather than on parsed AST representations.

This is a research hypothesis, not a proven result. Phase 15i is designed to test it.

---

## Why DUUMBI Graphs Are Uniquely Suited

### The Traditional Verification Pipeline (Textual Code)

```
Source code (.rs, .c, .java)
    │
    ▼ Parser (language-specific, thousands of rules)
AST (Abstract Syntax Tree)
    │
    ▼ CFG construction (control flow graph)
    │
    ▼ SSA transformation (static single assignment)
    │
    ▼ Type inference/checking
    │
    ▼ Verification Condition Generator
SMT formulas
    │
    ▼ Z3 / CVC5
Verified / Counterexample
```

Each step in this pipeline is a separate tool, a separate representation, and a potential source of soundness bugs. Tools like Dafny, F*, and SPARK/Ada spend enormous effort ensuring the pipeline is sound.

### The DUUMBI Verification Pipeline

```
JSON-LD graph (already structured, typed, ownership-annotated)
    │
    ▼ Verification Condition Generator (direct graph traversal)
SMT formulas
    │
    ▼ Z3 / CVC5
Verified / Counterexample
```

**Three steps eliminated:** parsing, CFG construction, SSA transformation. The JSON-LD graph is ALREADY:
- Structured (no parsing needed)
- Typed (types are explicit node properties, not inferred)
- In near-SSA form (each Op node produces a named output, consumed by downstream nodes)
- Ownership-annotated (memory safety is a graph property, not a compiler analysis result)
- Control-flow explicit (Blocks + Branch nodes = the CFG IS the graph)

### What This Means Concretely

To verify `abs(x: i64) -> i64` with postcondition `result >= 0`:

**Traditional (from Rust source):**
1. Parse `fn abs(x: i64) -> i64 { if x < 0 { -x } else { x } }`
2. Build AST with `if` expression, two branches
3. Convert to CFG with two basic blocks
4. Transform to SSA: `x_0 = param`, `cond = x_0 < 0`, branch on cond, `result_1 = -x_0`, `result_2 = x_0`, `result_3 = phi(result_1, result_2)`
5. Generate VCs: `(x_0 < 0 → -x_0 >= 0) ∧ (x_0 >= 0 → x_0 >= 0)`
6. Send to Z3

**DUUMBI (from JSON-LD graph):**
1. Read the graph: `Compare(x, 0, "lt")` → `Branch(cond, neg_block, pos_block)` → `Sub(0, x)` / `Return(x)`
2. The graph IS the CFG. The Op nodes ARE near-SSA.
3. Generate VCs directly: `(x < 0 → 0 - x >= 0) ∧ (x >= 0 → x >= 0)`
4. Send to Z3

Steps 1-4 of the traditional pipeline are **a single graph traversal** in DUUMBI.

---

## The Verification Condition Generator

### Architecture

```
duumbi:Function (with precondition + postcondition)
    │
    ▼
┌──────────────────────────────┐
│  VCGen: Graph → SMT Formulas  │
│                              │
│  For each Block:             │
│    Collect Op nodes          │
│    Build path conditions     │
│    At Return: assert post    │
│    At Branch: split paths    │
│    At Loop: check invariant  │
└──────────────────────────────┘
    │
    ▼
SMT-LIB v2 output (standard format)
    │
    ▼
┌──────────────────────────────┐
│  Z3 (or CVC5) SMT Solver     │
│                              │
│  Query: ¬(precondition ∧     │
│          path_conditions →   │
│          postcondition)      │
│                              │
│  UNSAT → postcondition holds │
│  SAT → counterexample found  │
└──────────────────────────────┘
    │
    ▼
duumbi:verificationStatus updated on the Function node
```

### VCGen Rules (Per Op Type)

| Op Type | SMT Translation | Notes |
|---|---|---|
| `Const(v)` | `(= result v)` | Literal value |
| `Add(a, b)` | `(= result (+ a b))` | Integer addition (bitvector in Z3 for overflow) |
| `Sub(a, b)` | `(= result (- a b))` | Integer subtraction |
| `Mul(a, b)` | `(= result (* a b))` | Integer multiplication |
| `Div(a, b)` | `(=> (not (= b 0)) (= result (div a b)))` | Division (guard against div-by-zero) |
| `Compare(a, b, op)` | `(= result (op a b))` | Comparison to boolean |
| `Branch(cond, T, F)` | Split into two paths: `cond=true` and `cond=false` | Fork verification |
| `Call(f, args)` | Substitute f's postcondition with args bound | Modular verification |
| `ArrayGet(arr, i)` | `(=> (and (>= i 0) (< i (length arr))) (= result (select arr i)))` | Bounds check |
| `ArrayPush(arr, v)` | `(= (length arr') (+ (length arr) 1))` | Length postcondition |
| `ReturnOk(v)` | `(= result (Ok v))` → assert postcondition | Success path |
| `ReturnErr(e)` | `(= result (Err e))` → assert error postcondition | Error path |

### Loop Verification (Hoare Logic)

For a loop with invariant `I` and variant `V`:

```
Proof obligations:
1. I holds before the loop (initialization)
2. If I holds at loop entry and loop condition is true, then I holds after loop body (preservation)
3. V decreases on each iteration and V >= 0 (termination)
4. If I holds and loop condition is false, then postcondition holds (postcondition)
```

In DUUMBI graph terms:
- The loop Block has `duumbi:loopInvariant` and `duumbi:loopVariant`
- The VCGen generates 4 SMT queries per loop
- All 4 must be UNSAT (proved) for the loop to be verified

### Ownership Verification (Novel)

The ownership model (Phase 9a) provides additional verification targets:

| Property | SMT Encoding | What It Proves |
|---|---|---|
| No use-after-free | `(=> (dropped x) (not (accessed x after_drop_point)))` | Memory safety |
| No double-free | `(= (drop_count x) 1)` | No resource leak / double free |
| Borrow exclusivity | `(not (and (shared_borrow x) (mut_borrow x)))` | Data race freedom |
| Lifetime validity | `(=> (borrow x scope) (alive x scope))` | No dangling references |

**These are ALREADY enforced by the schema validator (E020-E029).** But the validator uses graph-traversal heuristics. The formal verifier would provide mathematical proofs — useful for the highest certification levels (DO-178C Level A, ISO 26262 ASIL-D) where "the tool checked it" is insufficient and "there is a mathematical proof" is required.

---

## The `duumbi verify` Command

```bash
duumbi verify                        # verify all functions with specifications
duumbi verify --function math/abs    # verify specific function
duumbi verify --module math          # verify all functions in module
duumbi verify --timeout 30           # Z3 timeout per function (seconds)
duumbi verify --output proof.json    # export proof certificates
```

### Output

```
$ duumbi verify
Verifying math/abs...
  Precondition: true
  Postcondition: result >= 0 && (result == x || result == -x)
  Verification conditions: 2
  Z3 result: UNSAT (proved) ✅
  Time: 0.003s

Verifying sort/mergesort...
  Precondition: true
  Postcondition: is_sorted(result) && is_permutation(result, input)
  Loop invariant: forall i in 0..k: result[i] <= result[i+1]
  Verification conditions: 7
  Z3 result: UNSAT (proved) ✅
  Time: 1.2s

Verifying parser/parse_number...
  Precondition: true
  Postcondition: is_ok(result) => result.value >= 0
  Verification conditions: 3
  Z3 result: SAT (counterexample found) ❌
  Counterexample: input = "-42"  →  result = Ok(-42)  →  -42 >= 0 is FALSE
  Suggestion: postcondition should be "is_ok(result) => true" or
              function should handle negative numbers differently
  Time: 0.01s

Summary: 2/3 verified, 1 counterexample found
```

### Proof Certificates

For verified functions, the system exports a proof certificate:

```json
{
  "function": "duumbi:math/abs",
  "verified_at": "2026-06-15T10:30:00Z",
  "duumbi_version": "0.15.0",
  "z3_version": "4.13.0",
  "precondition": "true",
  "postcondition": "result >= 0 && (result == x || result == -x)",
  "verification_conditions": 2,
  "solver_result": "UNSAT",
  "proof_time_ms": 3,
  "graph_hash": "abc123...",
  "proof_hash": "def456..."
}
```

The `proof_hash` is stored on the function node as `duumbi:proofHash`. If the function's graph changes, the hash invalidates and re-verification is required.

---

## Integration with Existing Phases

| Phase | Integration Point |
|---|---|
| **Phase 9a** | Pre/postcondition fields in schema (foundation — already documented) |
| **Phase 9** | Benchmark runner optionally tracks specification generation quality |
| **Phase 10** | Knowledge graph stores verification results as `duumbi:VerificationResult` nodes |
| **Phase 12** | Verification Agent: a specialized dynamic agent that generates and checks specifications |
| **Phase 13** | Self-healing loop: if repair patch changes a verified function → re-verify automatically |
| **Phase 14** | Marketing: "The only AI development tool with formal verification" — compliance market differentiator |

### The Verification Agent (Phase 12 Extension)

A specialized agent template that can be dynamically spawned:

```jsonld
{
  "@type": "duumbi:AgentTemplate",
  "duumbi:name": "Verifier",
  "duumbi:role": "formal_verification",
  "duumbi:systemPrompt": "You are a formal verification specialist. Given a function and its implementation, generate preconditions, postconditions, and loop invariants that capture the function's intended behavior...",
  "duumbi:tools": ["graph.query", "graph.mutate", "verify.check"],
  "duumbi:specialization": "Generates and validates formal specifications for DUUMBI functions"
}
```

The Verification Agent:
1. Reads a function's implementation from the graph
2. Generates candidate specifications (pre/postcondition/invariants)
3. Calls `duumbi verify` on the function
4. If counterexample found → refines specifications and retries
5. If verified → stores proof certificate in knowledge graph

---

## Competitive Positioning

| Tool | Generates Code | Validates Code | Formally Verifies |
|---|---|---|---|
| Cursor / Copilot | ✅ (text) | ❌ (linting only) | ❌ |
| Claude Code | ✅ (text) | ✅ (runs tests) | ❌ |
| DUUMBI (current) | ✅ (graph) | ✅ (schema E001-E053) | ❌ |
| DUUMBI (Phase 15i) | ✅ (graph) | ✅ (schema E001-E053) | ✅ (Z3 proof) |
| Dafny | ❌ (manual) | ✅ | ✅ (Boogie/Z3) |
| SPARK/Ada | ❌ (manual) | ✅ | ✅ (GNATprove) |
| Lean 4 | ❌ (manual) | ✅ | ✅ (kernel) |

**DUUMBI Phase 15i would be the only tool that AI-generates code AND formally verifies it.** Dafny/SPARK/Lean require humans to write both code and specifications. DUUMBI's Verification Agent generates both — and the semantic graph makes verification structurally simpler.

---

## Research Questions (Open)

1. **Scalability:** Can Z3 handle verification conditions for functions with 100+ Op nodes? Or do we need abstraction/decomposition strategies?

2. **Specification quality from LLMs:** Can LLMs generate useful postconditions, or do they produce trivially true specifications (`postcondition: "true"`)? The Phase 9 benchmark data (with specification tracking) will provide empirical evidence.

3. **Loop invariant synthesis:** Generating correct loop invariants is the hardest part of formal verification. Can the two-phase generation strategy (Phase 9a) be extended to a third phase: logic → ownership → specifications?

4. **Heap verification:** The ownership model ensures no use-after-free. But can we verify functional correctness of heap-manipulating programs (e.g., "this sort function returns a sorted array")? This requires modeling the heap in Z3 — possible but expensive.

5. **Modular verification:** If function A calls function B, we verify A by substituting B's postcondition (not B's implementation). This is sound if B is verified. But what if B is an FFI function (UnsafeBlock)? The FFI boundary breaks the verification chain — how to handle this? (Likely answer: FFI functions have manually provided trusted specifications.)

6. **Formal model of Semantic Fixed Point:** Can the Semantic Fixed Point (valid schema ∧ passing tests ∧ fulfilled intent) be formalized as a fixed point in a lattice? If yes, does the verification add a fourth condition (valid schema ∧ formal proof ∧ passing tests ∧ fulfilled intent) that makes the lattice richer?

---

## Implementation Strategy (When the Time Comes)

### Dependencies (Rust Crates)

| Crate | Purpose |
|---|---|
| `z3` or `z3-sys` | Rust bindings to Z3 SMT solver |
| `smtlib` | SMT-LIB v2 format generation (alternative to direct Z3 API) |

### Estimated Scope

| Component | Effort | Complexity |
|---|---|---|
| Specification expression parser | 2-3 weeks | Medium (define grammar, write parser) |
| VCGen for basic Ops (arithmetic, comparison, branch) | 2-3 weeks | Medium |
| VCGen for loops (invariant checking) | 2-3 weeks | High (loop invariant verification is classically hard) |
| VCGen for Result/Option (error path verification) | 1-2 weeks | Medium |
| VCGen for ownership (memory safety proofs) | 2-4 weeks | High (heap modeling in Z3) |
| Z3 integration + proof certificate export | 1-2 weeks | Medium |
| Verification Agent (Phase 12 extension) | 2-3 weeks | Medium (prompt engineering for spec generation) |
| `duumbi verify` CLI command + Studio panel | 1 week | Low |
| **Total** | **13-21 weeks** | — |

This is a research effort — the timeline is uncertain. The kill criterion (3 stdlib functions verified) is designed to be achievable early, with the harder problems (loops, heap, scalability) as stretch goals.

---

## Expected Files

```
src/verify/
├── mod.rs           — duumbi verify command entry point
├── vcgen.rs         — verification condition generator (graph → SMT formulas)
├── smtlib.rs        — SMT-LIB v2 output format
├── z3_runner.rs     — Z3 process invocation + result parsing
├── spec_parser.rs   — specification expression parser
├── proof.rs         — proof certificate generation + storage
├── loops.rs         — loop invariant verification
├── ownership.rs     — ownership property verification (memory safety proofs)
└── counterexample.rs — counterexample formatting for human + LLM consumption
```

---

## Publications Potential

This Phase has at least three publishable research contributions:

1. **"Ownership and Lifetime Verification as Schema Constraints on Semantic Graphs"** — the Phase 9a ownership model as a novel type-safety enforcement mechanism, verified by Z3.

2. **"AI-Generated Programs with Formal Correctness Guarantees"** — the complete pipeline: intent → LLM → JSON-LD graph → specifications → Z3 proof → certified binary. No other system achieves this.

3. **"Verification Condition Generation from Semantic Graphs: Eliminating the AST-to-SSA Pipeline"** — comparative study showing that VCGen on DUUMBI graphs is simpler and more direct than traditional VCGen on parsed source code.

4. **"Lattice-Theoretic Formalization of the Semantic Fixed Point"** — if the fixed point can be formalized, this is a foundational theory paper.

---

## Related

- [[DUUMBI - Phase 9a - Type System Completion]] — pre/postcondition fields (foundation)
- [[DUUMBI - Phase 12 - Dynamic Agent System & MCP]] — Verification Agent template
- [[DUUMBI - Phase 13 - Self-Healing & Telemetry]] — re-verification after repair
- [[Semantic Fixed Point]] — the invariant that verification strengthens
- [[DUUMBI - PRD]] — long-term vision
