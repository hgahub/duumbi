# DUUMBI — Coding Conventions

> Companion to CLAUDE.md. Claude Code reads this when writing or reviewing
> Rust code. Rules are non-negotiable unless explicitly noted otherwise.

---

## Error handling

**Library code** (`src/parser/`, `src/graph/`, `src/compiler/`):
- Return `Result<T, DuumbiError>` with module-specific error types via `thiserror`
- Propagate with `?`; add `.context("what failed")` at module boundaries
- `.unwrap()` is **forbidden** — CI clippy will reject it
- `.expect("msg")` is allowed only for true invariants that cannot fail at runtime;
  the message must state the invariant, not just the operation

```rust
// ✅ correct
fn load_graph(path: &Path) -> Result<SemanticGraph, ParseError> {
    let text = fs::read_to_string(path).context("reading .jsonld file")?;
    parse_jsonld(&text)
}

// ❌ forbidden
let text = fs::read_to_string(path).unwrap();
```

**Application / CLI boundary** (`src/cli/`, `main.rs`):
- Use `anyhow::Result` and `anyhow::Context`
- Convert library errors to `anyhow` at the CLI layer only

**Error reporting to user:**
- Structured JSONL → stdout (machine-readable, see `docs/architecture.md`)
- Human-readable summary → stderr
- Never mix the two streams

---

## Type safety

Newtypes are mandatory for all graph identifiers — never pass raw integers:

```rust
// ✅ correct
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);   // wraps the @id string

// ❌ forbidden
fn get_node(id: String) -> Node { ... }  // too loose
fn get_node(idx: u32) -> Node { ... }    // no semantic meaning
```

Required newtypes: `NodeId`, `EdgeId`, `BlockLabel`, `FunctionName`, `ModuleName`.

---

## Graph code (petgraph)

- Use `StableGraph<N, E>` when node/edge indices must survive removals
- Use `DiGraph<N, E>` only for read-only or append-only graphs
- Never store raw `NodeIndex` across mutations of a `Graph` (use `StableGraph`)
- Traversal: prefer `petgraph::visit::{Bfs, Dfs, Topo}` over manual iteration
- Cycle detection: use `petgraph::algo::is_cyclic_directed` before compilation

```rust
// ✅ preferred traversal
use petgraph::visit::Topo;
let mut topo = Topo::new(&graph);
while let Some(node) = topo.next(&graph) { ... }
```

---

## Cranelift patterns

- Always use `FunctionBuilder` via `FunctionBuilderContext` — never call
  `InstBuilder` methods directly outside a builder scope
- Declare all SSA values before use; Cranelift is strict about value dominance
- One Cranelift function per `duumbi:Function` node — no inlining at this layer
- After emitting IR, call `builder.finalize()` before `module.define_function()`

```rust
// ✅ correct Cranelift structure
let mut ctx = codegen::Context::new();
let mut fn_ctx = FunctionBuilderContext::new();
{
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);
    let entry = builder.create_block();
    builder.switch_to_block(entry);
    builder.seal_block(entry);
    // ... emit instructions ...
    builder.finalize();
}
module.define_function(func_id, &mut ctx)?;
```

---

## Naming conventions

| Item | Convention | Example |
|------|-----------|---------|
| Functions, methods | `snake_case` | `build_graph`, `emit_const` |
| Types, traits, enums | `PascalCase` | `SemanticGraph`, `CompileError` |
| Constants, statics | `SCREAMING_SNAKE` | `MAX_BLOCK_DEPTH` |
| Modules | `snake_case` | `mod graph_builder` |
| JSON-LD Op types | `duumbi:PascalCase` | `duumbi:Const`, `duumbi:Add` |

---

## Documentation

- All `pub` items require a doc comment (`///`)
- Doc comments describe *what* and *why*, not *how*
- Use `#[must_use]` on all functions returning `Result` or meaningful values
- Module-level `//! doc` comments required for every module in `src/`

```rust
/// Validates the semantic graph against the core schema.
///
/// Returns a list of all validation errors found. An empty vec means valid.
/// Does not short-circuit on first error — collects all errors.
#[must_use]
pub fn validate(graph: &SemanticGraph) -> Vec<ValidationError> { ... }
```

---

## Async code

- Runtime: `tokio` only — no `async-std`, no `smol`
- No blocking calls inside `async fn`: use `tokio::fs`, `tokio::process`
- CPU-bound work (compilation): `tokio::task::spawn_blocking`
- Agent HTTP calls: `reqwest` with `tokio` feature

---

## Testing

- Unit tests: in the same file as the code (`#[cfg(test)] mod tests`)
- Integration tests: in `tests/` with real `.jsonld` fixtures
- No mocking libraries — use trait objects for LLM provider seams
- Every error code (E001–E009) must have at least one test that triggers it
- AI agent tests use hardcoded mock LLM responses — no live API calls in CI

---

## What Claude Code should never do in this codebase

- Add `.unwrap()` anywhere in library code
- Use `DiGraph` where node indices may be invalidated by removals
- Call Cranelift `InstBuilder` methods outside a `FunctionBuilder` scope
- Write `println!` for debug output — use `tracing::debug!` / `tracing::info!`
- Store raw `String` where a newtype (`NodeId`, `FunctionName`) should be used
- Add `#[allow(clippy::...)]` without a comment explaining why
