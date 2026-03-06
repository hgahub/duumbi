# Copilot Review Instructions for DUUMBI

Review pull requests with emphasis on correctness, safety, and docs consistency.

## High-priority checks

- Rust library code must not use `.unwrap()`.
- Prefer `thiserror` for module/library errors and `anyhow` only at CLI/application boundaries.
- Propagate errors with `?` and add contextual messages at module boundaries.
- Verify no behavioral regressions in compile pipeline: parser -> graph -> validator -> lowering -> linker.
- Flag risky changes in graph integrity, type handling, or Cranelift lowering semantics.

## Architecture-aware checks

- Treat the semantic graph as source of truth; watch for changes that can desync parser/graph/compiler assumptions.
- For graph identifier usage, prefer strong typing/newtype-style handling over loose raw primitives where applicable.
- In async code, avoid blocking operations in async contexts.

## CI and security checks

- Ensure PR keeps CI green (`fmt`, `clippy -D warnings`, `build`, `test`, `cargo audit`).
- Highlight dependency updates with potential breaking changes or security impact.

## Documentation checks

When PR changes CLI behavior, JSON-LD schema/types/ops, module/dependency behavior, or architecture:

- Confirm docs are updated under `sites/docs/src/`.
- If docs are missing, request concrete doc updates and list target files.

## Review style

- Prioritize findings: bugs, risks, regressions, and missing tests first.
- Be specific: include impacted files and exact behavior concerns.
- Keep suggestions actionable and minimal.
