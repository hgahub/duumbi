# DUUMBI — AI-First Semantic Graph Compiler

## About
Next-generation JSON-LD semantic graph compiler in Rust. Cranelift backend
for native code generation. petgraph for graph IR, serde_json for JSON-LD
parsing. AI agent graph mutation (OpenAI/Anthropic APIs), intent-driven
development, registry distribution, DUUMBI Studio web platform.

## Project structure
src/
  parser/      # JSON-LD parsing (serde_json, json-ld crate) → typed AST
  graph/       # Semantic graph IR using petgraph (StableGraph<Node, Edge>)
  compiler/    # Graph → Cranelift IR lowering (cranelift-codegen, cranelift-frontend)
  agents/      # AI agent framework for graph mutation (async, reqwest)
  intent/      # Intent-Driven Development (Phase 5): spec, coordinator, verifier, execute
  registry/    # Registry client, credentials, module packaging (Phase 7)
  deps.rs      # Dependency resolution, lockfile, vendor layer
  hash.rs      # Semantic hashing (SHA-256, @id-independent)
  manifest.rs  # Module manifest (manifest.toml)
  config.rs    # Config v2: workspace, registries, dependencies, vendor
  mcp/         # MCP server implementation (rmcp crate)
  web/         # WASM visualizer + axum HTTP server
  cli/         # CLI entry point (clap) — commands, deps, publish, yank, registry, repl
tests/         # Integration tests with .jsonld fixtures
crates/        # duumbi-studio (Leptos SSR web platform)

## Build and test
cargo build                          # Debug build
cargo build --release                # Release
cargo test --all                     # All tests (~790 tests)
cargo clippy --all-targets -- -D warnings  # Zero-warning lint policy
cargo fmt --check                    # Format check

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
- Registry client: reqwest with retry, credentials from ~/.duumbi/credentials.toml

## Architecture notes
- Graph IR is the central data structure — all transformations are graph→graph
- Cranelift compilation: Graph → Cranelift IR (one function per subgraph)
- AI agents receive read-only graph snapshots, propose mutation plans
- MCP server exposes graph query/mutation as tools
- Intent system (Phase 5): YAML spec → Coordinator tasks → LLM mutations → Verifier
- Registry (Phase 7): publish/download modules as .tar.gz, SemVer resolution,
  lockfile v1 with integrity hashes, vendor layer for offline builds
- Dependency resolution: workspace → vendor → cache → registry (E011 if not found)

## CLI commands (Phase 7)
- `duumbi search <query>` — search modules in configured registries
- `duumbi publish [--registry R] [--dry-run] [-y]` — package and upload module
- `duumbi yank <@scope/name@version> [--registry R] [-y]` — mark version as yanked
- `duumbi deps install [--frozen]` — resolve and download all deps, update lockfile
- `duumbi deps add <@scope/name[@ver]> [--registry R]` — add registry dependency
- `duumbi deps update [name]` — update to latest compatible versions
- `duumbi deps vendor [--all] [--include "pattern"]` — copy deps to vendor/
- `duumbi deps audit` — verify lockfile integrity hashes
- `duumbi deps tree [--depth N]` — display dependency tree
- `duumbi registry add|list|remove|default|login|logout` — manage registries
- `duumbi upgrade` — migrate Phase 4-5 workspace to Phase 7 format

@docs/architecture.md
@docs/coding-conventions.md