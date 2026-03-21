# Contributing to DUUMBI

Thank you for your interest in contributing to DUUMBI! This guide will help you
get started.

## Prerequisites

- **Rust** (stable, latest) — [rustup.rs](https://rustup.rs)
- **C compiler** — `cc` on PATH (Xcode CLI tools on macOS, `build-essential` on Ubuntu)
- **Git** — for version control

## Getting started

```bash
# Clone the repo
git clone https://github.com/hgahub/duumbi.git
cd duumbi

# Build
cargo build

# Run all tests
cargo test --all

# Lint (zero-warning policy)
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt --check
```

## Development workflow

1. **Find an issue** — look for [`good first issue`](https://github.com/hgahub/duumbi/labels/good%20first%20issue) labels
2. **Comment on the issue** to signal you're working on it
3. **Create a feature branch** from `main`:
   ```bash
   git checkout -b fix/short-description main
   ```
4. **Make your changes** — follow the coding conventions below
5. **Run the full check suite** before pushing:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   ```
6. **Open a pull request** against `main`

## Coding conventions

Full details in [`docs/coding-conventions.md`](docs/coding-conventions.md). Key rules:

- **Error handling:** `thiserror` in library code, `anyhow` at CLI boundary only.
  Never `.unwrap()` in library code.
- **Type safety:** Use newtypes (`NodeId`, `FunctionName`) — never raw strings or integers for identifiers.
- **Graph code:** `StableGraph` when indices must survive mutations.
- **Cranelift:** Always use `FunctionBuilder` — never raw `InstBuilder`.
- **Naming:** `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` constants.
- **Documentation:** All `pub` items need `///` doc comments.
- **No `println!`** — use `tracing::debug!` / `tracing::info!`.

## Project structure

```
src/
  parser/      # JSON-LD parsing → typed AST
  graph/       # Semantic graph IR (petgraph)
  compiler/    # Cranelift code generation
  agents/      # AI agent framework (LLM providers)
  intent/      # Intent-Driven Development system
  registry/    # Registry client
  cli/         # CLI entry point (clap)
  mcp/         # MCP server
  web/         # Web visualizer
runtime/       # C runtime (duumbi_runtime.c)
tests/         # Integration tests
```

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
feat(compiler): add support for ConstF64 op
fix(parser): handle empty block arrays in JSON-LD
docs: update quickstart tutorial
test(intent): add verifier edge case tests
```

## Pull request guidelines

- Keep PRs focused — one logical change per PR
- Fill out the PR template completely
- Ensure all CI checks pass before requesting review
- Link the related issue(s) in the PR description

## Running specific test suites

```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# A specific test
cargo test test_name

# With output
cargo test test_name -- --nocapture
```

## Reporting bugs

Use the [bug report template](https://github.com/hgahub/duumbi/issues/new?template=bug_report.yml).
Include: DUUMBI version, OS, Rust version, steps to reproduce.

## Suggesting features

Use the [feature request template](https://github.com/hgahub/duumbi/issues/new?template=feature_request.yml).

## Security vulnerabilities

Please report security issues privately via
[GitHub Security Advisories](https://github.com/hgahub/duumbi/security/advisories/new).
Do **not** open a public issue for security vulnerabilities.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
By participating, you agree to uphold this code.

## License

By contributing, you agree that your contributions will be licensed under the
same license as the project (see [LICENSE](LICENSE)).
