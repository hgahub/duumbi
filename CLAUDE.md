# DUUMBI — AI-First Semantic Graph Compiler

## About
Next-generation JSON-LD semantic graph compiler in Rust. Cranelift backend
for native code generation. petgraph for graph IR, serde_json for JSON-LD
parsing. Planned: AI agent graph mutation (OpenAI/Anthropic APIs), MCP
server, WASM+axum web visualizer.

## Project structure
src/
  parser/      # JSON-LD parsing (serde_json, json-ld crate) → typed AST
  graph/       # Semantic graph IR using petgraph (DiGraph<Node, Edge>)
  compiler/    # Graph → Cranelift IR lowering (cranelift-codegen, cranelift-frontend)
  agents/      # AI agent framework for graph mutation (async, reqwest)
  mcp/         # MCP server implementation (rmcp crate)
  web/         # WASM visualizer + axum HTTP server
  cli/         # CLI entry point (clap)
tests/         # Integration tests with .jsonld fixtures

## Build and test
cargo build                          # Debug build
cargo build --release                # Release
cargo test --all                     # All tests
cargo clippy --all-targets -- -D warnings  # Zero-warning lint policy
cargo fmt --check                    # Format check
wasm-pack build crates/web-viz       # Build WASM visualizer

## Code standards
- Use `thiserror` per module, `anyhow` at application boundaries only
- NEVER `.unwrap()` in library code; `.expect("invariant: ...")` for true invariants
- Propagate errors with `?`; provide `.context()` messages at module boundaries
- Newtypes for NodeId, EdgeId, GraphId — never raw u32/usize
- All public items need doc comments; use `#[must_use]` on Result-returning fns
- snake_case functions, PascalCase types, SCREAMING_SNAKE constants
- Prefer `petgraph::stable_graph::StableGraph` when indices must survive mutation
- Cranelift: use `FunctionBuilder` patterns, never raw `InstBuilder` calls
- Async code: tokio runtime, no blocking in async contexts

## Architecture notes
- Graph IR is the central data structure — all transformations are graph→graph
- Cranelift compilation: Graph → Cranelift IR (one function per subgraph)
- AI agents receive read-only graph snapshots, propose mutation plans
- MCP server exposes graph query/mutation as tools

@docs/architecture.md
@docs/coding-conventions.md