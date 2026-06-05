# DUUMBI-378: Tier 1 Console And Workspace File I/O

## Summary

Extend DUUMBI's Tier 1 standard library with practical console input/output and workspace-confined text file operations.

This product spec covers two user-facing modules:

- `@duumbi/stdlib-io`: keep the existing print wrappers compatible, and add line-oriented stdin/stdout helpers.
- `@duumbi/stdlib-file`: add text-file read/write, existence checks, deterministic directory listing, and a minimal path helper for paths that stay inside the DUUMBI workspace.

The goal is to let DUUMBI graph programs ask for input, print readable line output, read workspace data, write generated artifacts, and build credible file-driven examples without host-side glue code.

## Problem

DUUMBI already has a basic `@duumbi/stdlib-io` graph module with print wrappers for `i64`, `f64`, `bool`, and `string`. That is enough for simple demos, but it is not enough for practical command-line or file-processing workflows.

Without this work, DUUMBI users must hardcode values, depend on external host scripts, or wait for later JSON/network/database modules before they can show graph programs consuming local input and producing useful output. That weakens Phase 14 ecosystem examples and makes later Tier 1 modules harder to demonstrate cleanly.

The missing behavior is not broad operating-system access. The v1 product need is smaller: line input, explicit newline printing, bounded UTF-8 text reads, UTF-8 text writes, existence checks, sorted directory listing, and safe path handling inside the workspace boundary.

## Outcome

When this is done:

- Existing `@duumbi/stdlib-io` users keep the current `print_i64`, `print_f64`, `print_bool`, and `print_string` behavior.
- DUUMBI users can read one stdin line as a `result<string, string>` value.
- DUUMBI users can print a string plus a newline through an explicit `print_ln` API with `result<i64, string>` error behavior.
- DUUMBI users can read valid UTF-8 text files under an explicit byte limit from inside the DUUMBI workspace.
- DUUMBI users can write UTF-8 text files inside the workspace, overwriting existing files by default.
- DUUMBI users can test whether a workspace-confined path exists before reading or writing.
- DUUMBI users can list directory entries in deterministic sorted order.
- DUUMBI users can join simple path components without gaining a bypass around the workspace path policy.
- Errors are visible as `Err(string)` values instead of silent return codes or panics for expected I/O failures.
- Distribution remains clear: `@duumbi/stdlib-io` is updated in the default init/cache path; `@duumbi/stdlib-file` stays registry-add-only until #381 or product review explicitly changes default workspace dependencies.

## Scope

### In Scope

- Extend the existing `stdlib/io.jsonld` module for `@duumbi/stdlib-io`.
- Preserve the existing exported print wrappers:
  - `print_i64(x: i64) -> i64`
  - `print_f64(x: f64) -> i64`
  - `print_bool(x: bool) -> i64`
  - `print_string(s: string) -> i64`
- Add `read_line() -> result<string, string>`.
- Add `print_ln(value: string) -> result<i64, string>`.
- Add a new `@duumbi/stdlib-file` module with:
  - `read_file(path: string, max_bytes: i64) -> result<string, string>`
  - `write_file(path: string, contents: string) -> result<i64, string>`
  - `file_exists(path: string) -> result<bool, string>`
  - `list_dir(path: string) -> result<array<string>, string>`
  - `path_join(left: string, right: string) -> result<string, string>`
- Enforce workspace-confined file paths for every file operation.
- Treat v1 file content as text-only valid UTF-8.
- Require explicit `max_bytes` for reads.
- Sort directory entries deterministically.
- Update stdlib manifest/export metadata for affected modules.
- Update initialization/cache generation for the existing `@duumbi/stdlib-io` module.
- Provide tests and documentation for user-visible behavior and safety boundaries.

### Explicitly Out Of Scope

- Technical spec creation, implementation code, or Ralph-cycle work in this Stage 6 artifact.
- Binary file I/O, byte buffers, arbitrary byte arrays, or invalid UTF-8 passthrough.
- Unbounded full-file reads.
- Streaming reads, chunked writes, file watches, recursive directory walking, or glob patterns.
- File metadata beyond existence checks.
- Append mode, chmod/chown, symlink-specific APIs, or platform-specific path behavior beyond the v1 policy.
- Absolute paths, home-directory expansion, environment-variable expansion, drive-letter paths, or any path that escapes the workspace boundary.
- A `printf` clone, `print_fmt`, or a new format-string language.
- Default installation of `@duumbi/stdlib-file` into every new workspace unless #381 or product/spec review explicitly accepts that distribution change.
- JSON, TCP, HTTP, TLS, database, server, or all-Tier-1 publishing behavior owned by related issues.

## Constraints And Assumptions

Facts:

- Issue #378 is open and has Stage 5 acceptance with `Decision: Accept`, `Remaining open questions: none`, and `Next state: Spec Needed`.
- The current source repo has `stdlib/io.jsonld` exporting `print_i64`, `print_f64`, `print_bool`, and `print_string`.
- `duumbi init` embeds `stdlib/io.jsonld`, writes `@duumbi/stdlib-io` into `.duumbi/cache/@duumbi/stdlib-io@1.0.0/graph/io.jsonld`, and includes `@duumbi/stdlib-io` in default dependencies.
- Existing stdlib cache entries get `manifest.toml` files generated from `ModuleManifest::new`.
- Runtime support already includes strings, arrays, result values, and option values.
- Current runtime print functions print human-readable values with a newline.
- The active PRD and workflow guidance require spec-first behavior, explicit edge cases, verification evidence, and human-reviewable artifacts before implementation.
- The Phase 14 ecosystem plan names `@duumbi/stdlib-io` and `@duumbi/stdlib-file` as Tier 1 essentials, while issue #378 narrows their v1 scope.

Assumptions:

- `result<string, string>` and `result<i64, string>` are the product-facing contracts even if Stage 8 decides the exact graph/runtime representation details.
- The workspace boundary is the DUUMBI workspace root selected by the CLI/build context, not the process current directory when those differ.
- `path_join` is a convenience helper only; every actual file operation must validate the final path independently.
- For `read_line`, end-of-file before any bytes is an expected error state unless Stage 8 proves an existing DUUMBI convention that should represent EOF differently.
- Empty strings are valid text contents for `write_file`.
- Empty files are valid successful `read_file` outputs when `max_bytes` permits zero bytes.

Constraints:

- Backward compatibility is mandatory for existing `@duumbi/stdlib-io` print wrappers.
- File operations must not imply unrestricted filesystem access or security sandboxing beyond the implemented workspace boundary.
- New expected I/O failures must return `Err(string)` where the API says `result<_, string>`.
- Error strings should be human-readable and stable enough for tests to assert the error class without depending on full operating-system wording.
- Tests must not hang on stdin or file I/O.
- Directory listing must be deterministic across supported platforms.
- Path behavior must be defined for nonexistent write targets, where full canonicalization of the target file may not be possible before creation.

## Decisions

- **Decision:** This spec is file-based at `specs/DUUMBI-378/PRODUCT.md`.
  **Evidence:** The accepted scope is user-visible, cross-module, runtime-adjacent, safety-sensitive, and useful as durable context for Stage 8 and later review.

- **Decision:** Existing print wrappers stay compatible.
  **Evidence:** Issue #378 explicitly requires current `print_i64`, `print_f64`, `print_bool`, and `print_string` behavior to remain compatible, and current users receive `@duumbi/stdlib-io` by default through `duumbi init`.

- **Decision:** `print_fmt` is deferred from v1.
  **Evidence:** Issue #378 states that formatting needs a typed, non-variadic contract and should not become a fragile `printf` clone before DUUMBI has an explicit format-language and lowering design.

- **Decision:** v1 file APIs are text-only and valid UTF-8 only.
  **Evidence:** Issue #378 explicitly defers binary I/O and byte buffers, and requires invalid UTF-8 to return `Err(string)`.

- **Decision:** `read_file` requires `max_bytes`.
  **Evidence:** Issue #378 explicitly rejects unbounded full-file reads for v1.

- **Decision:** `write_file` overwrites by default.
  **Evidence:** Issue #378 records overwrite-by-default as the v1 decision and defers append. A separate fail-if-exists API is only allowed if product/spec review accepts it.

- **Decision:** `path_join` is included in v1 as the minimal accepted path helper.
  **Evidence:** Issue #378 lists `path_join` as the candidate v1 helper. Keeping it minimal gives users a practical way to compose workspace-relative paths while preserving the rule that actual file operations validate final paths independently.

- **Decision:** File paths are workspace-confined.
  **Evidence:** Issue #378 requires rejecting absolute paths and rejecting `..` escapes after path normalization/canonicalization, and warns that runtime behavior must not imply unrestricted filesystem access.

- **Decision:** `@duumbi/stdlib-file` remains registry-add-only until #381 unless product/spec review changes default init behavior.
  **Evidence:** Issue #378 records this distribution decision, while #381 owns publishing all Tier 1 modules to the registry.

- **Decision:** #379, #381, and #382 remain separate scopes.
  **Evidence:** #379 covers JSON and TCP, #381 covers server and Tier 1 publishing, and #382 covers all Tier 1 ecosystem smoke tests.

## Behavior

### Module Availability

`@duumbi/stdlib-io` remains an existing default dependency for new DUUMBI workspaces created by `duumbi init`.

`@duumbi/stdlib-file` is available through the accepted registry/add path for stdlib modules, but is not added to default new workspace dependencies in this issue unless product/spec review explicitly changes that decision.

### Existing Console Output

The existing `print_i64`, `print_f64`, `print_bool`, and `print_string` APIs remain exported with their current signatures and visible output behavior.

Existing programs that import and call those print wrappers should compile and run the same way after this work.

### New Console Input And Output

`read_line() -> result<string, string>` reads one line from stdin.

On success:

- The returned string contains valid UTF-8 text.
- A trailing `\n` is excluded from the returned string when present.
- For CRLF input, the returned string should exclude the line ending as user-entered line structure rather than expose a stray carriage return.
- An empty line before a newline is a successful `Ok("")`.

On expected failure:

- EOF before any line content returns `Err(string)`.
- Invalid UTF-8 returns `Err(string)`.
- Underlying stdin read failure returns `Err(string)`.

`read_line` may block while waiting for stdin. Tests and smoke paths must provide deterministic stdin.

`print_ln(value: string) -> result<i64, string>` prints the string plus exactly one trailing newline as the explicit line-printing API for strings.

On success:

- The visible output is the string contents followed by one appended newline.
- Existing newline characters inside `value` are printed as string contents; `print_ln` does not strip or normalize them.
- The return value is `Ok(0)`.

On expected failure:

- Output/write failure returns `Err(string)`.

`print_ln` does not replace or remove `print_string`; it gives users an explicit result-returning line-output API.

### Workspace Path Policy

Every `@duumbi/stdlib-file` operation validates its input path against the active DUUMBI workspace root before touching the filesystem.

The path policy is:

- Reject empty paths unless Stage 8 identifies an existing DUUMBI convention for representing the workspace root safely.
- Reject absolute paths.
- Reject `..` segments that would escape the workspace after normalization.
- Reject home expansion such as `~`, environment-variable expansion, URL-like paths, and platform drive prefixes as v1 unsupported behavior.
- Normalize simple `.` segments and redundant separators without allowing escape.
- Validate the final file-operation path independently even if it came from `path_join`.
- For read/existence/list operations, resolve enough real filesystem state to prevent workspace escape.
- For write operations to nonexistent files, validate the normalized target path and its parent directory against the workspace boundary before creation.
- If any existing path component resolves outside the workspace boundary, including through a symlink, reject the operation rather than follow it outside the workspace.

The product contract is a workspace boundary, not a claim that DUUMBI provides a full operating-system sandbox.

### File Reading

`read_file(path: string, max_bytes: i64) -> result<string, string>` reads a workspace-confined text file.

On success:

- The file is inside the active DUUMBI workspace boundary.
- The file content is valid UTF-8.
- The returned string contains the full file content when its byte length is less than or equal to `max_bytes`.

On expected failure:

- `max_bytes < 0` returns `Err(string)`.
- `max_bytes == 0` succeeds only for an empty file; non-empty files return `Err(string)` for the byte-limit violation.
- Missing files, directories passed as files, permission failures, invalid UTF-8, and byte-limit violations return `Err(string)`.
- Paths that violate the workspace policy return `Err(string)`.

`read_file` must not perform unbounded reads.

### File Writing

`write_file(path: string, contents: string) -> result<i64, string>` writes UTF-8 text to a workspace-confined file.

On success:

- The target path is inside the workspace boundary.
- Existing files are overwritten by default.
- Empty contents are valid.
- The return value is `Ok(0)`.

On expected failure:

- Paths that violate the workspace policy return `Err(string)`.
- Missing or invalid parent directories return `Err(string)`.
- Permission failures or filesystem write failures return `Err(string)`.

`write_file` does not append and does not provide fail-if-exists behavior in v1. If Stage 7 product review requires fail-if-exists behavior, it should be added as a separately named API such as `write_file_new`; otherwise it belongs to a follow-up.

### Existence Checks

`file_exists(path: string) -> result<bool, string>` checks a workspace-confined path.

On success:

- `Ok(true)` means the path exists inside the workspace.
- `Ok(false)` means the path does not exist inside the workspace and the operation can determine that without a path-policy or permission failure.

On expected failure:

- Paths that violate the workspace policy return `Err(string)`.
- Permission or path-resolution failures that prevent a trustworthy answer return `Err(string)` when distinguishable.

### Directory Listing

`list_dir(path: string) -> result<array<string>, string>` lists entries in a workspace-confined directory.

On success:

- The result contains entry names as strings, not absolute paths.
- The result is sorted deterministically.
- An empty directory returns an empty array.

On expected failure:

- Paths that violate the workspace policy return `Err(string)`.
- Missing paths, files passed as directories, permission failures, and directory-read failures return `Err(string)`.

Recursive listing, globbing, hidden-file filtering rules, metadata, and symlink-specific behavior are out of scope.

### Path Joining

`path_join(left: string, right: string) -> result<string, string>` joins two path components into a normalized relative path string.

On success:

- The output is a relative path string suitable for display or for passing to file operations.
- Redundant separators and simple `.` segments are normalized.

On expected failure:

- Absolute `right` paths are rejected.
- A join result that would escape the workspace when later validated is rejected.
- Unsupported expansion syntax such as `~` or environment variables returns `Err(string)`.

`path_join` does not grant authority. File operations must still validate the returned path.

### Error And Empty States

All new v1 APIs that return `result<_, string>` must use `Err(string)` for expected runtime failures. Panics are reserved for internal invariants, not ordinary user I/O failures.

Empty valid text is allowed where it is meaningful:

- Empty stdin line: `Ok("")`.
- Empty file content: `Ok("")` when within `max_bytes`.
- Empty write contents: `Ok(0)`.
- Empty directory listing: `Ok([])`.

### Cancellation, Offline, And Retry Behavior

There is no automatic retry contract for v1 I/O or file operations.

Cancellation behavior is owned by the process, CLI harness, or test runner. The APIs should not hide cancellation as a successful result.

Offline behavior is not applicable to local console/file APIs.

### Race Conditions And Invariants

File state can change between `file_exists` and a later `read_file` or `write_file`. Users must not rely on `file_exists` as an atomic permission or locking check.

Path validation must happen at the operation that touches the filesystem, not only when a helper constructs a string.

Directory listing order must not depend on operating-system enumeration order.

Existing print wrapper compatibility is an invariant.

Workspace escape rejection is an invariant.

### Accessibility And Focus Rules

No new graphical UI behavior is required by this issue.

If examples or docs include REPL/TUI use, stdin prompts and file-operation errors should be readable as text and should not rely on color-only meaning.

## BDD Scenarios

Feature: Console input and line output for DUUMBI graph programs

  Rule: Existing print wrappers remain compatible

    Scenario: Existing integer print wrapper still works
      Given a workspace that imports `@duumbi/stdlib-io`
      And an existing graph program calls `print_i64(42)`
      When the program is built and run after the stdlib update
      Then the program prints the same visible integer output as before
      And the program does not need to change its import or function call

    Scenario: A program reads one stdin line
      Given a graph program calls `read_line()`
      And stdin provides the line `duumbi\n`
      When the program runs
      Then `read_line` returns `Ok("duumbi")`

    Scenario: A program reads an empty stdin line
      Given a graph program calls `read_line()`
      And stdin provides `\n`
      When the program runs
      Then `read_line` returns `Ok("")`

    Scenario: Invalid stdin text returns an error
      Given a graph program calls `read_line()`
      And stdin provides bytes that are not valid UTF-8
      When the program runs
      Then `read_line` returns `Err(string)`
      And the process does not panic for the expected input error

    Scenario: A program prints a line through the result-returning API
      Given a graph program calls `print_ln("hello")`
      When the program runs
      Then stdout contains `hello` followed by one newline
      And `print_ln` returns `Ok(0)`

Feature: Workspace-confined text file operations

  Rule: File operations cannot escape the workspace

    Scenario: Absolute paths are rejected
      Given a DUUMBI workspace rooted at a temporary directory
      And a graph program calls `read_file("/etc/passwd", 1024)`
      When the program runs
      Then `read_file` returns `Err(string)`
      And no file outside the workspace is read

    Scenario: Parent-directory escapes are rejected
      Given a DUUMBI workspace rooted at a temporary directory
      And a graph program calls `write_file("../outside.txt", "data")`
      When the program runs
      Then `write_file` returns `Err(string)`
      And no file outside the workspace is written

  Rule: Reads are bounded and text-only

    Scenario: A program reads a valid text file within the byte limit
      Given a workspace file `input.txt` contains `hello`
      And a graph program calls `read_file("input.txt", 5)`
      When the program runs
      Then `read_file` returns `Ok("hello")`

    Scenario: A read over the explicit byte limit fails
      Given a workspace file `input.txt` contains `hello`
      And a graph program calls `read_file("input.txt", 4)`
      When the program runs
      Then `read_file` returns `Err(string)`
      And the error identifies the byte-limit class

    Scenario: Invalid UTF-8 file content fails
      Given a workspace file contains bytes that are not valid UTF-8
      And a graph program calls `read_file` for that file
      When the program runs
      Then `read_file` returns `Err(string)`
      And the process does not panic for the expected file content error

  Rule: Writes are text-only and overwrite by default

    Scenario: A program writes a new text file
      Given a DUUMBI workspace has no `out.txt`
      And a graph program calls `write_file("out.txt", "hello")`
      When the program runs
      Then `write_file` returns `Ok(0)`
      And the workspace file `out.txt` contains `hello`

    Scenario: A program overwrites an existing text file
      Given a workspace file `out.txt` contains `old`
      And a graph program calls `write_file("out.txt", "new")`
      When the program runs
      Then `write_file` returns `Ok(0)`
      And the workspace file `out.txt` contains `new`

  Rule: Directory and path helpers are deterministic and safe

    Scenario: A program checks whether a file exists
      Given a workspace file `input.txt` exists
      When a graph program calls `file_exists("input.txt")`
      Then `file_exists` returns `Ok(true)`

    Scenario: A program lists a directory deterministically
      Given a workspace directory `data` contains files `b.txt` and `a.txt`
      When a graph program calls `list_dir("data")`
      Then `list_dir` returns `Ok(["a.txt", "b.txt"])`

    Scenario: A joined path is still validated by file operations
      Given a graph program calls `path_join("data", "input.txt")`
      And passes the result to `read_file`
      When both calls run
      Then `path_join` returns `Ok("data/input.txt")` or the platform-neutral equivalent
      And `read_file` validates that final path against the workspace boundary before reading

## Tasks

- Extend the `@duumbi/stdlib-io` product surface with `read_line` and `print_ln` while preserving the existing print wrappers.
- Define and add the `@duumbi/stdlib-file` product surface with text read/write, existence checks, deterministic directory listing, and `path_join`.
- Update manifest/export metadata for both affected modules.
- Update `duumbi init` cache generation for the existing `@duumbi/stdlib-io` exports and description.
- Keep `@duumbi/stdlib-file` out of default new workspace dependencies unless Stage 7 product review changes the distribution decision.
- Add or update user-facing module descriptions and examples that show line input, newline output, bounded text reads, overwriting writes, existence checks, sorted listing, and workspace boundary failures.
- Add focused tests for the console APIs, file APIs, path policy, UTF-8 policy, byte-limit policy, overwrite behavior, deterministic listing, and existing print wrapper compatibility.
- Add E2E smoke coverage for stdin line input, explicit line printing, reading a temp workspace file, writing a temp workspace file, checking existence, and listing a temp workspace directory.

Independent work:

- Existing `@duumbi/stdlib-io` compatibility tests and manifest assertions.
- `@duumbi/stdlib-file` graph/module metadata and documentation.
- Path-policy and UTF-8 error tests.
- Directory sorting tests.

Sequential work:

- Runtime/compiler support for new primitives before graph-module smoke tests can pass.
- `duumbi init` cache metadata update after final accepted `@duumbi/stdlib-io` export list is known.
- Registry-add/install validation for `@duumbi/stdlib-file` after module packaging path is available.

## Checks

- `cargo fmt --check`
- `cargo test --all`
- `cargo clippy --all-targets -- -D warnings`
- Focused compatibility tests proving existing `print_i64`, `print_f64`, `print_bool`, and `print_string` behavior remains unchanged.
- Unit tests or integration tests for `read_line`:
  - success with a normal line
  - success with an empty line
  - CRLF line ending normalization
  - EOF before content
  - invalid UTF-8
  - deterministic supplied stdin so tests cannot hang
- Unit tests or integration tests for `print_ln`:
  - writes exactly one trailing newline
  - returns `Ok(0)` on success
  - represents output failure as `Err(string)` where practical
- Unit tests or integration tests for `read_file`:
  - valid UTF-8 within `max_bytes`
  - empty file
  - missing file
  - directory passed as file
  - invalid UTF-8
  - `max_bytes < 0`
  - `max_bytes == 0` with empty and non-empty files
  - file larger than `max_bytes`
  - workspace escape attempts
- Unit tests or integration tests for `write_file`:
  - new file
  - overwrite existing file
  - empty contents
  - missing parent directory
  - workspace escape attempts
  - permission/path failure where practical
- Unit tests or integration tests for `file_exists`:
  - existing file
  - missing file
  - directory path
  - workspace escape attempts
  - permission/path failure where practical
- Unit tests or integration tests for `list_dir`:
  - deterministic sorted names
  - empty directory
  - missing directory
  - file passed as directory
  - workspace escape attempts
- Unit tests or integration tests for `path_join`:
  - normal join
  - redundant separator normalization
  - `.` segment handling
  - absolute right-hand path rejection
  - escape attempt rejection
  - confirmation that file operations still validate joined results
- E2E smoke tests:
  - stdin line input
  - newline printing
  - reading a temp workspace file with `max_bytes`
  - writing a temp workspace file
  - checking existence
  - listing a temp workspace directory
- Documentation or inline module descriptions explaining:
  - what users can build with the new I/O and file capabilities
  - text-only UTF-8 behavior
  - explicit read-size limits
  - overwrite-by-default writes
  - workspace-confined path policy
  - why `print_fmt`, binary I/O, append, recursive traversal, and unrestricted paths are out of scope

Stage 7 review evidence should confirm:

- The spec remains product-level and does not create a technical spec.
- The spec contains BDD scenarios that map to observable behavior.
- The spec uses non-closing references to #378.
- The spec-only PR leaves the execution issue open for later workflow stages.

## Open Questions

None blocking for product specification.

Non-blocking items for Stage 7 or Stage 8:

- Should EOF before any stdin bytes be represented only as `Err(string)`, or should DUUMBI later introduce a separate EOF-aware API?
- Should `write_file_new(path, contents)` be included in v1 as fail-if-exists behavior, or deferred to a follow-up?
- Should `path_join("", "file.txt")` be accepted as `file.txt`, or should empty path components be rejected consistently?
- How much of a platform-native path separator should be exposed in `path_join` output versus normalized DUUMBI-relative `/` separators?
- Should `@duumbi/stdlib-file` become a default dependency before #381, despite the accepted issue's registry-add-only recommendation?

## Sources

- Related issue: https://github.com/hgahub/duumbi/issues/378
- Stage 4 triage refill comment: https://github.com/hgahub/duumbi/issues/378#issuecomment-4627230151
- Stage 5 Human Acceptance Decision: https://github.com/hgahub/duumbi/issues/378#issuecomment-4629265563
- Related JSON/TCP issue: https://github.com/hgahub/duumbi/issues/379
- Related Tier 1 publishing issue: https://github.com/hgahub/duumbi/issues/381
- Related Tier 1 smoke test issue: https://github.com/hgahub/duumbi/issues/382
- Existing stdlib I/O graph: `stdlib/io.jsonld`
- Workspace initialization source: `src/cli/init.rs`
- Runtime support source: `runtime/duumbi_runtime.c`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- DUUMBI PRD: `DUUMBI - PRD`
- DUUMBI Glossary: `DUUMBI - Glossary`
- DUUMBI Agentic Development Map: `DUUMBI Agentic Development Map`
- DUUMBI Agentic Development Runbook: `DUUMBI - Agentic Development Runbook`
- Spec-first guidance: `Spec-First Agentic Development`
- AI review policy: `AI Code Review Service Policy`
- Graph repository architecture: `Graph Repository Architecture`
- Archived Phase 14 Marketing and Go-to-Market plan: `DUUMBI - Phase 14 - Marketing & Go-to-Market`
