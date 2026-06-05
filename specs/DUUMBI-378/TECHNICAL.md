# DUUMBI-378: Tier 1 Console And Workspace File I/O - Technical Specification

## Implementation Objective

Implement the approved DUUMBI-378 product spec by extending Tier 1 standard library support for console input/output and workspace-confined UTF-8 text file operations.

This technical spec implements these approved outcomes:

- Preserve the existing `@duumbi/stdlib-io` print wrappers and their visible newline behavior.
- Add `read_line() -> result<string, string>` and `print_ln(value: string) -> result<i64, string>`.
- Add `@duumbi/stdlib-file` APIs for bounded UTF-8 reads, UTF-8 writes, existence checks, sorted directory listing, and simple path joining.
- Enforce a workspace-relative path policy before file operations touch the filesystem.
- Return expected I/O failures as `Err(string)` rather than panics or silent return codes.
- Keep `@duumbi/stdlib-file` out of default new workspace dependencies until #381 or a later product decision changes distribution.

Workflow note: the technical-spec PR for this issue is spec-only and must leave the execution issue open. Use non-closing references such as `Related to #378` or `Technical spec for #378`.

## Agent Audience

Primary implementation agents:

- Codex for source edits, focused tests, deterministic local E2E evidence, and review feedback handling.
- A Ralph-cycle implementation agent only after Stage 9 approval routes the issue to Ready for Build.

Review agents:

- Codex for Stage 9 technical spec review and implementation PR review evidence.
- Copilot as the default required automated reviewer for file-based spec and implementation PRs.
- Greptile only if the developer explicitly requests a manual deep review.

## Source Context

Verified facts:

- Product spec artifact: `specs/DUUMBI-378/PRODUCT.md`.
- Product spec PR: https://github.com/hgahub/duumbi/pull/661.
- Stage 7 product spec approval: https://github.com/hgahub/duumbi/issues/378#issuecomment-4630904683.
- GitHub issue: https://github.com/hgahub/duumbi/issues/378.
- Current `stdlib/io.jsonld` exports `print_i64`, `print_f64`, `print_bool`, and `print_string`.
- Current `src/cli/init.rs` embeds `stdlib/io.jsonld`, seeds `@duumbi/stdlib-io` into new workspace cache, and lists it in default dependencies.
- Current runtime print functions in `runtime/duumbi_runtime.c` append a newline and return `void`; the stdlib wrappers return `0` after calling them.
- `src/types.rs` already models `string`, `array<T>`, `result<T,E>`, and the existing result/option ops.
- `src/parser/mod.rs` parses `result<T,E>` and `array<T>` type strings and maps JSON-LD op fields onto reusable AST slots: `operand`, `left`, and `right`.
- `src/graph/builder.rs` already converts `operand`, `left`, and `right` AST references into semantic graph edges; no new edge label is needed for this issue.
- `src/graph/result_safety.rs` enforces handling for calls and ops that return `Result` or `Option`.
- `src/compiler/lowering.rs` lowers heap values, including strings, arrays, and results, as pointer-sized values and calls C runtime helpers for current heap/runtime ops.
- `runtime/duumbi_runtime.c` already has heap string, array, result, and option helpers, but no stdin, file, directory, or workspace-root helpers.
- `src/workspace.rs::run_workspace_binary` runs `.duumbi/build/output`, sets `current_dir(workspace_root)`, and captures stdout/stderr, but currently has no stdin helper and no explicit workspace-root environment contract.
- `main.rs` implements `duumbi run` by delegating workspace runs to `workspace::run_workspace_binary`.
- `.github/workflows/ci.yml` treats docs/spec-only PRs as documentation-only checks; implementation PRs must satisfy Rust checks.
- Repo instructions require no `.unwrap()` in library code, public docs on public items, focused tests for changed behavior, and zero-warning clippy.
- Active DUUMBI workflow guidance permits Stage 9 AI-gate approval only after Codex self-review, actual Copilot evidence, clean checks or docs-only inapplicability, no unresolved review threads, and no open scope/product/architecture/security/cost questions.

Assumptions:

- The active workspace root for file APIs is the DUUMBI workspace root selected by workspace build/run context, not arbitrary process current directory.
- Result payload representation should follow the current runtime convention: `Ok` and `Err` payloads are stored as `int64_t`, with heap pointers cast through that representation when needed.
- DUUMBI path strings use `/` as the stable, cross-platform relative separator. Native path conversion is an implementation detail inside the runtime.
- This issue does not need live LLM-provider validation because it does not change provider, agent, intent-generation, or model-routing behavior.

## Stage 8 Decisions

- EOF before any stdin bytes is `Err(string)` with a stable error class such as `stdin_eof`.
- `write_file_new` is not part of v1; fail-if-exists and append behavior are follow-ups.
- `path_join("", "file.txt")`, `path_join("dir", "")`, and any empty normalized path component are rejected with `Err(string)`.
- `@duumbi/stdlib-file` is not added to default `[dependencies]` by this issue.
- Single-file binaries or directly executed workspace binaries without an explicit workspace-root environment must return `Err(string)` from file APIs rather than silently falling back to process current directory.
- Backslash paths are unsupported in v1 DUUMBI path strings; use `/` separators and reject `\` to avoid platform-specific escape ambiguity.
- The implementation must leave all legacy print wrappers returning `i64` and must not convert them to `result<_, string>`.

## Affected Areas

Expected source changes:

- `stdlib/io.jsonld`
  - Preserve existing exports and wrapper bodies.
  - Add `read_line` and `print_ln` wrappers.
  - Export both new functions.

- `stdlib/file.jsonld`
  - Add a new graph module for `@duumbi/stdlib-file`.
  - Define and export `read_file`, `write_file`, `file_exists`, `list_dir`, and `path_join`.

- `stdlib/file.manifest.toml`
  - Add source manifest metadata for registry-add use and #381 publishing.
  - Include exports matching the new file module.
  - Do not make this a default workspace dependency.

- `src/types.rs`
  - Add op variants: `ReadLine`, `PrintLn`, `ReadFile`, `WriteFile`, `FileExists`, `ListDir`, and `PathJoin`.
  - Add `Display` strings for each new op.
  - Make `output_type` return the declared `result<_, string>` type for these result-returning ops, with validator checks enforcing the exact expected contract.

- `src/parser/mod.rs`
  - Parse new JSON-LD op types:
    - `duumbi:ReadLine`: no operands.
    - `duumbi:PrintLn`: `duumbi:operand`.
    - `duumbi:ReadFile`: `duumbi:path` mapped to `left`, `duumbi:maxBytes` mapped to `right`.
    - `duumbi:WriteFile`: `duumbi:path` mapped to `left`, `duumbi:contents` mapped to `right`.
    - `duumbi:FileExists`: `duumbi:path` mapped to `operand`.
    - `duumbi:ListDir`: `duumbi:path` mapped to `operand`.
    - `duumbi:PathJoin`: `duumbi:left` mapped to `left`, `duumbi:right` mapped to `right`.
  - Require `duumbi:resultType` on each result-returning op and report existing schema diagnostics when it is missing.

- `src/graph/validator.rs`
  - Validate operand and result contracts for the new ops.
  - Require `string` input for paths, contents, and `print_ln`.
  - Require `i64` input for `max_bytes`.
  - Require exact result types:
    - `ReadLine` -> `result<string,string>`
    - `PrintLn` -> `result<i64,string>`
    - `ReadFile` -> `result<string,string>`
    - `WriteFile` -> `result<i64,string>`
    - `FileExists` -> `result<bool,string>`
    - `ListDir` -> `result<array<string>,string>`
    - `PathJoin` -> `result<string,string>`

- `src/compiler/lowering.rs`
  - Declare C runtime functions:
    - `duumbi_read_line() -> i64`
    - `duumbi_print_ln(i64 value) -> i64`
    - `duumbi_file_read(i64 path, i64 max_bytes) -> i64`
    - `duumbi_file_write(i64 path, i64 contents) -> i64`
    - `duumbi_file_exists(i64 path) -> i64`
    - `duumbi_list_dir(i64 path) -> i64`
    - `duumbi_path_join(i64 left, i64 right) -> i64`
  - Lower each new op to the matching runtime call and store the returned result pointer/value.
  - Keep legacy `Print` and `PrintString` lowering unchanged.

- `runtime/duumbi_runtime.c`
  - Add runtime helpers for stdin, stdout-with-error-result, UTF-8 validation, workspace path validation, file operations, directory listing, and path joining.
  - Add platform-specific directory listing implementations under `#ifdef _WIN32` and POSIX branches.
  - Preserve existing runtime function signatures and visible print behavior.

- `src/workspace.rs`
  - Set `DUUMBI_WORKSPACE_ROOT` to the absolute workspace root when running workspace binaries.
  - Add a stdin-capable workspace run helper, such as `run_workspace_binary_with_stdin`, so deterministic tests can feed `read_line` without hanging.
  - Keep stdout/stderr capture behavior.

- `main.rs`
  - Ensure `duumbi run` workspace mode uses the updated workspace runner so file APIs see the workspace-root environment.
  - Do not make non-workspace direct binary execution an implicit workspace-root fallback.

- `src/cli/init.rs`
  - Update `@duumbi/stdlib-io` manifest/export metadata for the new IO functions.
  - Do not add `@duumbi/stdlib-file` to default dependencies.
  - Do not seed `@duumbi/stdlib-file` as a default dependency path in new workspaces.

- `src/mcp/tools/graph.rs` and graph describe surfaces
  - Add friendly names for new ops where the source uses explicit op formatting.
  - Keep the existing generic debug fallback for unrelated ops.

Expected test changes:

- Parser and validator unit tests for all new op shapes, operand types, and exact result types.
- Runtime/build integration tests for console IO and file IO behavior.
- Stdlib parse/export tests for updated `stdlib/io.jsonld` and new `stdlib/file.jsonld`.
- `duumbi init` tests proving `@duumbi/stdlib-io` exports are updated and `@duumbi/stdlib-file` is not a default dependency.
- Workspace run tests proving `DUUMBI_WORKSPACE_ROOT` is set in workspace mode.

Expected generated/local artifacts during validation:

- Temporary workspaces under the OS temp directory.
- Temporary UTF-8 and invalid-UTF-8 files inside those workspaces.
- Temporary compiled binaries under `.duumbi/build/`.
- No committed generated binaries, logs, runtime assets, or test output files.

CI/check paths:

- `cargo fmt --check`
- `cargo test --all`
- `cargo clippy --all-targets -- -D warnings`

## Technical Approach

### 1. Extend The Instruction Set Narrowly

Add dedicated op variants instead of modeling file APIs as generic `Call` nodes. The compiler already uses explicit ops for runtime-backed primitives such as strings, arrays, results, and printing. Keeping IO/file functionality in that pattern gives the parser, validator, lowering, and describe surfaces a clear contract.

The new ops are:

- `ReadLine`
- `PrintLn`
- `ReadFile`
- `WriteFile`
- `FileExists`
- `ListDir`
- `PathJoin`

All new user-visible APIs except legacy print wrappers return `Result`. The graph validator must reject missing or incorrect result types early so bad fixtures fail before Cranelift lowering.

### 2. Preserve Existing Print Compatibility

Do not change `Op::Print`, `Op::PrintString`, `duumbi_print_i64`, `duumbi_print_f64`, `duumbi_print_bool`, or `duumbi_print_string`.

Existing stdlib wrappers should keep this structure:

1. load the function parameter,
2. call the existing print op,
3. return `0`.

The new `print_ln` wrapper should call `duumbi:PrintLn` and return the runtime `Result` directly. This separates backwards-compatible print wrappers from the new result-returning API.

### 3. Use A Workspace-Root Environment Contract For Runtime File APIs

When DUUMBI runs a workspace binary through `duumbi run` or `workspace::run_workspace_binary`, the launcher must set:

```text
DUUMBI_WORKSPACE_ROOT=<absolute workspace root>
```

Runtime file APIs must read this environment variable, canonicalize it, and reject file operations if it is missing, empty, relative, or not a directory.

Reasons:

- Current compiled programs do not receive an implicit workspace-root parameter.
- Process current directory is not a reliable security boundary.
- This keeps file behavior tied to DUUMBI workspace execution without changing the compiled program ABI.

Directly running `.duumbi/build/output` outside `duumbi run` is allowed, but file APIs return `Err(string)` unless the caller sets `DUUMBI_WORKSPACE_ROOT` explicitly. Console-only APIs continue to work.

### 4. Enforce Deterministic DUUMBI-Relative Paths

Runtime path validation should operate in two phases.

Lexical validation:

- Reject empty strings.
- Reject paths containing NUL bytes.
- Reject absolute paths.
- Reject paths beginning with `~`, `$`, or URL-like prefixes.
- Reject drive-letter prefixes such as `C:`.
- Reject backslashes and UNC-style paths.
- Split only on `/`.
- Remove `.` and repeated separator segments.
- Reject any `..` segment.
- Reject a path that normalizes to empty.

Filesystem validation:

- Canonicalize the workspace root.
- Join the normalized DUUMBI-relative path to the root.
- For existing read/list/existence targets, canonicalize the target and require it to remain within the canonical root using path-component boundary checks.
- For missing existence targets, canonicalize the nearest existing parent under the root and return `Ok(false)` when the remaining lexical path is valid.
- For writes, canonicalize the parent directory and require it to remain within the canonical root. If the target exists, canonicalize the target before overwriting. If the target does not exist, validate the parent plus basename before creation.
- Reject symlink escapes by checking canonicalized existing components.

This is a workspace boundary, not a full operating-system sandbox. A filesystem race between validation and use is still possible and should be documented in code comments/tests as residual risk; do not claim stronger isolation.

### 5. Implement Runtime IO With Stable Error Classes

Runtime functions should return `Result` values using existing `duumbi_result_new_ok` and `duumbi_result_new_err` helpers.

Error strings should be human-readable and include stable class tokens so tests do not depend on platform-specific OS messages. Recommended classes:

- `stdin_eof`
- `stdin_invalid_utf8`
- `stdout_write_failed`
- `path_policy`
- `workspace_root_unavailable`
- `not_found`
- `not_file`
- `not_directory`
- `permission_denied`
- `byte_limit`
- `invalid_utf8`
- `io_error`

The implementation may include additional detail after the class token, but tests should assert the stable class rather than full text.

### 6. Validate UTF-8 Explicitly

Treat file and stdin contents as bytes at the runtime boundary and validate UTF-8 before creating a DUUMBI string.

Required behavior:

- `read_line` strips one trailing `\n`; if the remaining line ends with `\r` from CRLF, strip that `\r` too.
- `read_line` returns `Ok("")` for an empty line before a newline.
- `read_line` returns `Err(stdin_eof...)` for EOF before any bytes.
- `read_file` rejects invalid UTF-8 bytes.
- `write_file` validates the DUUMBI string contents before writing.
- Path strings must reject NUL bytes even if other DUUMBI strings can contain them.

### 7. Keep File Reads Bounded

`read_file(path, max_bytes)` must never perform an unbounded read.

Required behavior:

- `max_bytes < 0` returns `Err(byte_limit...)`.
- `max_bytes == 0` succeeds only for an empty file.
- A file larger than `max_bytes` returns `Err(byte_limit...)`.
- A directory path returns `Err(not_file...)`.
- Valid empty files return `Ok("")` when `max_bytes >= 0`.

The implementation may use file metadata before reading or stream up to `max_bytes + 1`; either approach must avoid unbounded allocation and still handle metadata races with an error result.

### 8. Make Directory Listing Deterministic

`list_dir(path)` returns `result<array<string>, string>`.

Required behavior:

- The path must refer to an existing directory inside the workspace.
- Exclude `.` and `..`.
- Return entry names only, not full paths.
- Sort names by bytewise/string order before returning.
- Store each name as a DUUMBI string pointer inside the existing runtime array representation.

The implementation must have POSIX and Windows branches because the CI matrix includes Windows for Rust-relevant PRs.

### 9. Define `path_join` As Pure Path Normalization

`path_join(left, right)` does not touch the filesystem and does not prove that the resulting path is safe for file access.

Required behavior:

- Reject empty `left` or `right`.
- Apply the same lexical DUUMBI-relative validation rules as file paths.
- Join with `/`.
- Normalize `.` and repeated separators.
- Reject any `..` segment.
- Return a DUUMBI-relative path with `/` separators in `Ok(string)`.

Every file API must independently validate the returned value again.

### 10. Keep `@duumbi/stdlib-file` Out Of Defaults

Implementation should add source and manifest metadata for `@duumbi/stdlib-file`, but must not add it to default new workspace dependencies.

Required checks:

- Existing new workspaces still include `@duumbi/stdlib-io`.
- Existing new workspaces do not include `@duumbi/stdlib-file`.
- `stdlib/file.jsonld` parses as a module and exports the approved functions.
- `stdlib/file.manifest.toml` parses and lists the same exports.

Issue #381 owns broader Tier 1 publishing and any default dependency policy change.

## BDD-To-Test Mapping

| Product scenario | Required test evidence |
|---|---|
| Existing print wrappers stay compatible | Integration test importing `@duumbi/stdlib-io` calls `print_i64`, `print_f64`, `print_bool`, and `print_string`; stdout remains exact and wrappers return `0`. |
| `read_line` reads a normal stdin line | Subprocess/workspace test feeds `hello\n`, handles `Result` with `ResultIsOk`/unwrap or `Match`, prints the value, and asserts `hello\n`. |
| `read_line` reads an empty stdin line | Subprocess test feeds `\n`, handles `Ok("")`, and asserts the program reaches the success branch without hanging. |
| Invalid UTF-8 stdin returns error | Subprocess test writes invalid bytes to piped stdin and asserts `Err` class contains `stdin_invalid_utf8`. |
| `print_ln("hello")` appends one newline | Runtime integration test calls the stdlib wrapper and asserts stdout is exactly `hello\n` and result is `Ok(0)`. |
| Absolute paths are rejected | File API integration test passes `/tmp/duumbi.txt` or platform equivalent and asserts `Err(path_policy...)`. |
| Parent-directory escapes are rejected | File API integration test passes `../outside.txt` and asserts `Err(path_policy...)`. |
| Valid file read within byte limit succeeds | Temp workspace test writes `data/input.txt`, program reads with sufficient `max_bytes`, and stdout matches content. |
| File read above byte limit fails | Temp workspace test reads a file with `max_bytes` below byte length and asserts `Err(byte_limit...)`. |
| Invalid UTF-8 file read fails | Temp workspace test writes invalid bytes and asserts `Err(invalid_utf8...)`. |
| Write new text file succeeds | Program calls `write_file("out.txt", "hello")`; host asserts `out.txt` exists inside workspace and contains `hello`. |
| Overwrite existing text file succeeds | Host pre-creates `out.txt`; program overwrites; host asserts old contents are gone. |
| File exists returns true/false | Program or runtime integration test checks an existing file and a missing valid path, asserting `Ok(true)` and `Ok(false)`. |
| Directory listing is sorted | Temp workspace creates unsorted files; program calls `list_dir`, checks length and indexed entries, and asserts deterministic order. |
| Joined paths are validated again | Program joins `data` and `input.txt`, then reads successfully; a separate test joins or passes invalid segments and asserts `Err(path_policy...)`. |
| Empty `path_join` components are rejected | Unit/runtime test calls `path_join("", "file.txt")` and `path_join("dir", "")`; both return `Err(path_policy...)`. |
| `@duumbi/stdlib-file` is not default | `duumbi init` test asserts default config dependencies exclude `@duumbi/stdlib-file`. |
| Workspace root is explicit | Workspace run test asserts file APIs succeed through `duumbi run`/workspace runner and fail with `workspace_root_unavailable` when no root is set. |

Test fixture guidance:

- Result-returning calls in graph fixtures must be handled in the same block or via `Match` so `src/graph/result_safety.rs` does not flag unhandled results.
- Prefer dedicated `tests/integration_phase378_io_file.rs` for new end-to-end behavior instead of expanding unrelated Phase 9a tests heavily.
- Keep invalid-UTF-8 file/stdin tests deterministic and non-hanging by using explicit child stdin pipes or a stdin-capable workspace runner.

## Live E2E Plan

This issue does not touch LLM behavior. Required live external LLM calls: `0`. Required provider credentials: none.

Deterministic live E2E after implementation:

1. Run `cargo build`.
2. Create a temporary DUUMBI workspace.
3. Run `target/debug/duumbi init` in that workspace.
4. Seed only the test workspace cache with `@duumbi/stdlib-file@1.0.0` from the committed `stdlib/file.jsonld` and `stdlib/file.manifest.toml` sources, then add `"@duumbi/stdlib-file" = "1.0.0"` to that workspace config. This simulates registry-add/cache resolution without changing default init behavior.
5. Add a workspace graph program that imports the updated IO module and the explicitly configured file module.
6. Create workspace files:
   - `data/input.txt` with valid UTF-8 text.
   - `data/list-b.txt` and `data/list-a.txt` for sorted listing.
7. Build the workspace with `target/debug/duumbi build`.
8. Run the workspace with `target/debug/duumbi run`, feeding stdin `hello\n`.
9. Assert stdout proves:
   - `read_line` returned `hello`.
   - `read_file` returned the file content.
   - `file_exists` returned true for an existing file and false for a missing path.
   - `list_dir` returned sorted entries.
   - `print_ln` appended exactly one newline per call.
10. Assert the host filesystem shows `write_file` created or overwrote the expected file inside the workspace.
11. Run one negative smoke path for workspace escape rejection. On platforms where symlink creation is permitted, include a symlink-to-outside case; otherwise record the platform skip and keep lexical escape tests mandatory.

The implementation PR must automate every deterministic E2E step that does not require elevated platform permissions. Symlink-escape coverage may be conditionally skipped only on platforms where creating the symlink is not permitted; lexical escape tests remain mandatory everywhere. Manual smoke evidence is useful for Stage 10/11, but automated tests are the acceptance baseline.

## Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Path traversal or symlink escape | Use lexical validation plus canonicalized filesystem boundary checks. Add absolute, `..`, symlink, missing-parent, and nonexistent-target tests. |
| Overclaiming sandbox strength | Document that this is a workspace boundary, not a full OS sandbox, and avoid claims of complete race-free isolation. |
| Windows directory/path behavior diverges | Use DUUMBI `/` path strings, reject backslashes/drive prefixes, and implement directory listing under `#ifdef _WIN32`. |
| Stdin tests hang | Use piped stdin with deterministic bytes or a stdin-capable workspace runner. Do not rely on interactive stdin in CI. |
| Result safety rejects new fixtures | Ensure every result-returning call is checked or matched before unwrap/return in test graphs. |
| Legacy stdout behavior regresses | Keep old runtime print functions unchanged and add exact stdout compatibility tests. |
| File module distribution becomes ambiguous | Add source/manifest metadata, but assert `@duumbi/stdlib-file` is absent from default dependencies. |
| Runtime C helper complexity grows too broad | Keep helpers local to UTF-8 text IO and path policy. Defer binary, streaming, append, recursive listing, and metadata APIs. |

## Rejected Alternatives

- Do not implement file APIs as unrestricted host filesystem calls. That violates the accepted workspace-confined scope.
- Do not fall back to process current directory when workspace root is unavailable. It makes file safety depend on how a binary was launched.
- Do not convert existing print wrappers to `result<_, string>`. That breaks current `@duumbi/stdlib-io` users.
- Do not add `write_file_new`, append mode, or fail-if-exists behavior in this issue.
- Do not add binary buffers or invalid-UTF-8 passthrough.
- Do not add recursive directory walking, globbing, file metadata, chmod/chown, watchers, sockets, HTTP, JSON parsing, or database behavior.
- Do not add live LLM/provider validation for this issue; deterministic compiler/runtime tests are the relevant evidence.

## Ralph Cycle Resource Policy

Stage 10 may run bounded Ralph cycles only after Stage 9 approval and Ready for Build routing.

Autonomous cycle budget:

- Maximum external LLM cost per cycle: USD 2.
- Maximum external LLM calls per cycle: 10.
- Expected external LLM calls for this implementation: 0 unless the developer explicitly asks for live agent/provider validation.
- No new third-party dependencies without human approval.
- No implementation outside the approved product and technical specs.

Suggested cycle split:

- Cycle 1: instruction-set, parser, validator, stdlib source, and manifest metadata.
- Cycle 2: runtime helpers, lowering, workspace-root runtime contract, and workspace runner stdin/root support.
- Cycle 3: integration tests, live deterministic E2E evidence, docs/comments required by public API changes, and review cleanup.

Human authorization is required before continuing if any of these occur:

- Estimated external LLM cost would exceed USD 2.
- Expected external LLM calls would exceed 10.
- Scope expands beyond DUUMBI-378.
- A risky dependency, migration, security-sensitive behavior, irreversible operation, or broad refactor becomes necessary.
- The implementation needs to change default `@duumbi/stdlib-file` distribution.
- The path policy requires a product or architecture decision not settled in this spec.
- Cross-platform runtime behavior cannot be made test-clean inside the approved scope.
- Checks fail in a way the agent cannot resolve without broadening scope.

## Approval Checklist For Stage 9

- Product spec #661 is merged and approved.
- This PR changes only `specs/DUUMBI-378/TECHNICAL.md`.
- The technical spec stays inside the accepted DUUMBI-378 product scope.
- BDD scenarios have explicit test mappings.
- Live E2E evidence path is defined and explains why no live LLM/provider run is required.
- Ralph Cycle resource policy includes USD 2, 10-call, scope, risky-dependency, migration, security-sensitive, blocker, and product/architecture gates.
- No unresolved scope, product, architecture, security, migration, cost, or verification questions remain.
- Codex self-review reports no blocking findings.
- Required automated reviewer evidence exists and has no blocking unresolved feedback.
- Checks are passing or documentation-only/spec-only inapplicable.
