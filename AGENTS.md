# DUUMBI — AI-First Semantic Graph Compiler

## About
Next-generation JSON-LD semantic graph compiler in Rust. Cranelift backend
for native code generation. petgraph for graph IR, serde_json for JSON-LD
parsing. AI agent graph mutation (OpenAI/Anthropic APIs), intent-driven
development, registry distribution, DUUMBI Studio web platform.

## Agent interaction style
- If the user writes in Hungarian, start with a natural everyday American
  English translation before answering.
- If the user writes in English, first provide a corrected, natural native
  American English version when wording can be improved.
- Be direct and evidence-aware: separate facts, assumptions, interpretations,
  uncertainty, and speculation when architecture or implementation risk matters.
- Challenge weak reasoning respectfully and concretely; prefer practical,
  maintainable solutions over clever but fragile ones.

## Repository layout
- **`hgahub/duumbi`** (this repo) — Rust source code, compiler, CLI, tests, technical docs
- **`hgahub/duumbi-vault`** — Obsidian planning vault (PRD, phase specs, roadmap, architecture diagrams)
  - Lokálisan: `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/`
  - Obsidian MCP nincs használatban — a vault fájljai közvetlenül elérhetők: Read/Edit/Grep/Glob toolokkal

## Project structure
src/
  parser/      # JSON-LD parsing (serde_json, json-ld crate) → typed AST
  graph/       # Semantic graph IR using petgraph (StableGraph<Node, Edge>)
  compiler/    # Graph → Cranelift IR lowering (cranelift-codegen, cranelift-frontend)
                 # CodegenBackend trait — Cranelift types never leak outside src/compiler/
  agents/      # AI agent framework for graph mutation (async, reqwest)
               # Phase 12: analyzer, assembler, template, cost, merger, rollback
               # agent_knowledge — strategy/failure-pattern persistence
  mcp/         # MCP server + client (JSON-RPC, 10 tools, external server proxy)
               # Phase 12: graph_query, graph_mutate, graph_validate, graph_describe
  intent/      # Intent-Driven Development (Phase 5): spec, coordinator, verifier, execute
  registry/    # Registry client, credentials, module packaging (Phase 7)
  types.rs     # DuumbiType (I64, F64, Bool, Void, String, Array<T>, Struct, &T, &mut T), Op enum
  deps.rs      # Dependency resolution, lockfile, vendor layer
  hash.rs      # Semantic hashing (SHA-256, @id-independent)
  manifest.rs  # Module manifest (manifest.toml)
  config.rs    # Config v2: workspace, registries, dependencies, vendor
  mcp/         # MCP server implementation (rmcp crate)
  web/         # WASM visualizer + axum HTTP server
  cli/         # CLI entry point (clap) — commands, deps, publish, yank, registry, repl
runtime/       # C runtime (duumbi_runtime.c) — print, alloc, string/array/struct shims
tests/         # Integration tests with .jsonld fixtures
crates/        # duumbi-studio (Leptos SSR web platform)

## Build and test
cargo build                          # Debug build
cargo build --release                # Release
cargo test --all                     # All tests (~817 tests)
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

## CLI commands (Phase 7 + 12)
- `duumbi mcp` — start the MCP server (JSON-RPC over stdio, 10 tools)
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
- `duumbi provider list|add|remove|set` — manage LLM provider configurations

@docs/architecture.md
@docs/coding-conventions.md
