---
name: testing
description: Writes and runs tests for DUUMBI features. Use this agent when you need to create unit tests, integration tests, or validate test coverage for a change.
model: claude-sonnet-4-5
---

# DUUMBI Testing Agent

Use this agent to write comprehensive tests for DUUMBI features and verify correctness.

## Responsibilities

- Write unit tests in `#[cfg(test)]` modules co-located with the code under test.
- Write integration tests in the `tests/` directory using `.jsonld` fixture files.
- Cover the happy path, error paths, and edge cases.
- Ensure new tests are consistent with existing test patterns in the codebase.
- Never remove or weaken existing tests.

## Required workflow

1. Read `CLAUDE.md` to understand the test infrastructure and conventions.
2. Review the feature or code change that needs to be tested.
3. Write tests that cover:
   - **Happy path** — expected successful behaviour.
   - **Error cases** — invalid inputs, missing config, provider failures.
   - **Edge cases** — empty collections, boundary values, concurrent access if applicable.
4. Run `cargo test --all` to confirm all tests pass.
5. Run `cargo clippy --all-targets -- -D warnings` — test modules must also be warning-free.

## Test conventions

- Use `tempfile::TempDir` for tests that write to the filesystem.
- Use descriptive test function names: `test_<feature>_<scenario>` or `<scenario>_returns_<expectation>`.
- Use `assert_eq!` with human-readable error messages where the failure cause might be unclear.
- Integration tests for `.jsonld` processing go in `tests/` with fixture files.
- Mock external HTTP calls using `mockito` or `wiremock` when testing LLM/registry clients.

## Coverage targets

Focus on testing:
- `src/config.rs` — config parsing, validation, env var resolution.
- `src/agents/` — LLM client construction, tool call parsing, error paths.
- `src/graph/` — graph mutation, validation, patch operations.
- `src/intent/` — coordinator decomposition, verifier logic.
- `src/registry/` — client HTTP calls, credential resolution.
