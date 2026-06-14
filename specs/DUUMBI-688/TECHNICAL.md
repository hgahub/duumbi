# DUUMBI-688: Flagship HTTP + SQLite + JSON Reference Program - Technical Specification

Related to #688. This is a specification-only artifact. The execution issue
must remain open for implementation, implementation review, and Stage 12
closure.

## Implementation Objective

Create one durable flagship example that demonstrates DUUMBI building and
running a local HTTP JSON API backed by SQLite. The implementation must be
small enough for a visitor to inspect, deterministic enough for CI, and honest
about current runtime boundaries.

The finished implementation must add:

- `examples/flagship-http-sqlite-json/` with reader-facing example sources and
  a README.
- Root-level and in-repo documentation links that identify this as the first
  runnable reference example.
- Focused integration coverage that builds the example, starts the compiled
  binary on loopback, sends one HTTP request, parses the JSON response, and
  verifies deterministic shutdown or cleanup.

Do not add implementation code during this Stage 8 spec PR.

## Agent Audience

This spec is for the Stage 10 implementation agent. It assumes the agent can
read Rust, JSON-LD graph fixtures, DUUMBI workspace configuration, integration
tests, and GitHub issue/PR history.

The agent should optimize for a narrow implementation that proves the accepted
product behavior without turning #688 into a runtime API design issue.

## Source Context

- Issue: https://github.com/hgahub/duumbi/issues/688
- Stage 5 acceptance:
  https://github.com/hgahub/duumbi/issues/688#issuecomment-4702900849
- Product spec: `specs/DUUMBI-688/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/712
- Product spec merge: `9e5d2f30829a9a8328e8abff3102ca77d6a67c16`
- Stage 6 draft comment:
  https://github.com/hgahub/duumbi/issues/688#issuecomment-4702927943
- Stage 7 AI gate comment:
  https://github.com/hgahub/duumbi/issues/688#issuecomment-4702932341
- Stage 7 decision comment:
  https://github.com/hgahub/duumbi/issues/688#issuecomment-4702932782
- Project architecture: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Workspace build helpers: `src/workspace.rs`
- CLI build/run/describe commands: `src/cli/mod.rs`, `src/cli/commands.rs`
- Workspace init behavior: `src/cli/init.rs`
- Workspace config loading: `src/config.rs`
- Core operation definitions: `src/types.rs`
- Stdlib modules: `stdlib/json.jsonld`, `stdlib/db.jsonld`,
  `stdlib/server.jsonld`, `stdlib/http.jsonld`
- Existing HTTP + JSON + SQLite composition evidence:
  `tests/integration_duumbi380_e2e.rs`
- Existing clean-workspace and bounded server smoke patterns:
  `tests/integration_duumbi382_ecosystem_smoke.rs`
- Repository ignore rule: `.gitignore` ignores `.duumbi/` globally.
- Vault context:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Flagship Examples and Showcase Programs.md`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active development map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`

## Affected Areas

Expected implementation files:

- `examples/flagship-http-sqlite-json/README.md`
- `examples/flagship-http-sqlite-json/graph/main.jsonld`
- `examples/flagship-http-sqlite-json/config.toml`
- `README.md`
- A durable in-repo docs page, such as `docs/examples.md`
- A focused integration test, such as
  `tests/integration_duumbi688_flagship_example.rs`

Potential implementation files, only if the selected example layout requires
them:

- `examples/flagship-http-sqlite-json/intent.md` or a similarly small companion
  artifact that helps explain the graph without introducing new behavior.
- `.gitignore`, if the implementation intentionally commits a `.duumbi/`
  example workspace and therefore needs narrowly scoped negation rules.
- CI workflow files, only if the new integration test is not already covered by
  existing `cargo test --all` checks.

Out of scope unless a blocking defect is discovered and a human approves scope
expansion:

- New DUUMBI public operations.
- New stdlib module exports.
- Runtime server/database/JSON API redesign.
- New external services, credentials, or production registry dependencies.
- Broad CLI, REPL, TUI, Studio, or registry behavior changes.

## Technical Approach

Use the existing workspace build/run path. The preferred checked-in example
layout is non-dot source files under
`examples/flagship-http-sqlite-json/`, because repository `.gitignore`
currently ignores `.duumbi/` globally.

The implementation should use this pattern:

1. Keep the reader-facing graph at
   `examples/flagship-http-sqlite-json/graph/main.jsonld`.
2. Keep reader-facing workspace configuration at
   `examples/flagship-http-sqlite-json/config.toml`.
3. In the integration test, materialize a temporary DUUMBI workspace by copying
   those files into `<temp>/.duumbi/config.toml` and
   `<temp>/.duumbi/graph/main.jsonld`, plus any local dependency sources needed
   for an offline build.
4. Build the temporary workspace with `duumbi::workspace::build_workspace` and
   `workspace_output_path`.
5. Start the compiled binary with `std::process::Command`, not
   `run_workspace_binary`, because the HTTP server test must drive the request
   while the process is alive.
6. Drive one loopback HTTP request with `TcpStream`, parse the response body
   with `serde_json`, assert stable fields, then wait for deterministic process
   exit with a timeout.

If the implementation chooses to commit a literal `.duumbi/` workspace inside
the example directory instead, it must add narrow `.gitignore` negation rules
for only that example and prove with `git status --short --ignored` or
equivalent review evidence that intended example files are tracked while build
outputs and runtime state remain ignored.

The graph should:

- Import only existing accepted stdlib modules:
  `@duumbi/stdlib-server`, `@duumbi/stdlib-db`, `@duumbi/stdlib-json`, and any
  minimal already-accepted helper modules needed for string or I/O behavior.
- Bind `server_new` to `127.0.0.1` by default.
- Use a documented default port or an argument-driven port if the current graph
  and runtime path can support it without new APIs. The integration test may
  patch or generate the graph from the checked-in template to avoid port
  collisions.
- Open SQLite with `:memory:` unless the implementation can keep a file-backed
  database inside the temporary workspace and remove or ignore generated state
  deterministically. The product spec permits either when the SQLite evidence
  remains visible.
- Create a table, insert deterministic sample data, query it, read at least one
  row via `db_rows_len` and `db_row_get`, and close/free DB resources.
- Construct a JSON response string that includes at least:
  - `service`: a stable example name such as
    `flagship-http-sqlite-json`;
  - one value read back from SQLite;
  - one count proving the query result size;
  - one status or route field that makes the response inspectable.
- Register the response with `route_add_static` and serve it through
  `server_start` with `max_requests = 1` and a short timeout.
- Close server resources after serving.

Do not add dynamic HTTP handlers. The accepted server module exposes bounded
static routes. For #688, building a static response from SQLite-derived values
before registering the route is sufficient and more faithful to the existing
API. If current string/JSON capabilities cannot construct the required body
without hidden host-side logic, stop and escalate with the smallest concrete
gap instead of inventing new runtime APIs.

## Invariants

- The example must build from a clean clone using normal DUUMBI build paths.
- The core behavior must live in DUUMBI graph/workspace artifacts, not in a Rust
  test helper that fabricates the response.
- The default server must bind only to loopback.
- Runtime behavior must be bounded by request count and timeout.
- The test must not require public internet, credentials, provider setup, or
  production registry access.
- The implementation must not commit generated binaries, logs, SQLite runtime
  output, `.duumbi/build`, or temporary state.
- The README transcript must match the implemented default behavior closely
  enough for a visitor to compare their local run.
- The example must not overstate production readiness.
- Spec-only and implementation PR text must use non-closing references such as
  `Related to #688` or `Implementation for #688`.

## BDD-To-Test Mapping

| Product scenario | Automated or review evidence |
| --- | --- |
| A visitor discovers the flagship example from repository docs | Root `README.md` links to `examples/flagship-http-sqlite-json/`; `docs/examples.md` or equivalent links to the same example; integration or review checklist asserts both references. |
| A clean clone can build the example without external services | `tests/integration_duumbi688_flagship_example.rs` materializes a temp workspace from the checked-in example and calls `build_workspace` successfully. |
| The example returns SQLite-backed JSON over HTTP | The integration test starts the compiled binary, sends `GET /facts` or the documented route over `127.0.0.1`, parses the HTTP body with `serde_json`, and asserts response fields derived from `db_query`, `db_rows_len`, and `db_row_get` output. |
| The graph remains inspectable | README names the graph/config files, stdlib imports, and `duumbi describe` command; review verifies that the checked-in graph is formatted and reader-facing. |
| The runtime is bounded | Integration test requires `max_requests = 1`, uses read/connect timeouts, and kills/fails the child process if it does not exit after the request. |
| CI catches example rot | The new integration test is part of the default test suite or a documented CI path; PR evidence includes the relevant GitHub check. |
| Repeated runs are predictable | Integration test runs from a fresh temp workspace, asserts no committed runtime state is required, and verifies cleanup or ignored state behavior. |
| Optional intent/BDD companion remains useful | If added, review verifies it is small, deterministic, and explanatory only; no automated behavior may depend on it. |

## Live E2E Plan

The live E2E path for the implementation PR must use the compiled DUUMBI binary
and the checked-in example artifacts.

Canonical local command shape:

```sh
cargo build
target/debug/duumbi build --offline
target/debug/duumbi describe
target/debug/duumbi run
```

The exact working directory and arguments must match the final example README.
If the checked-in layout is non-dot source files, the README may use a short
documented setup command to materialize a local `.duumbi/` workspace, but that
setup must be deterministic, local, and should not require extra tools beyond
standard shell/Rust commands already used by the repo.

Automated E2E test plan:

1. Create a `tempfile::TempDir`.
2. Copy example config and graph into `<temp>/.duumbi/`.
3. Seed local stdlib dependencies from checked-in `stdlib/` files or use the
   same init/cache pattern already proven by #382, without production registry
   access.
4. Build with `build_workspace(temp.path(), &workspace_output_path(temp.path()), offline)`.
5. Spawn the compiled binary with `current_dir(temp.path())`.
6. Poll/connect to the documented loopback route with a deadline.
7. Send one HTTP request and read the complete response.
8. Parse the body with `serde_json::from_str`.
9. Assert stable response fields and status.
10. Wait for the process to exit within a short timeout; kill and fail if it
    does not.
11. Assert stdout/stderr are either empty or exactly documented.
12. Assert no committed example file is a generated runtime artifact.

No real provider or LLM path is required for the E2E plan. Expected external
LLM cost is USD 0. Public network access is not required.

## Ralph Cycle Protocol

Use Ralph cycles only during Stage 10 implementation, not during this spec
stage. Each cycle should have one bounded goal and should leave reviewable
evidence in the implementation PR.

Recommended cycles:

- Cycle 1: Add example source layout and README skeleton; prove files are
  tracked despite `.duumbi/` ignore constraints.
- Cycle 2: Add graph/config content and local build materialization helper in
  the integration test.
- Cycle 3: Add bounded live HTTP E2E assertions and docs links.
- Cycle 4: Polish transcript, cleanup behavior, and final verification.

Stop early if a cycle reveals that existing stdlib/server/string/JSON/database
capabilities cannot construct a SQLite-derived static response without new
runtime API work.

## Cycle Budget

- External LLM approval threshold: request human approval before any single
  cycle or reviewer action expected to exceed USD 1 in external LLM cost.
- Autonomous iteration cap: none. Continue Ralph cycles until the scoped
  implementation goal is complete, a gate fails, or a blocker appears.
- Default cycle scope: one coherent implementation slice touching roughly
  one example area plus its focused test or docs evidence.
- Suggested per-cycle file budget: 1-6 files. Exceeding this is allowed when
  the work is mechanical and still inside #688 scope, but explain it in the PR.
- Suggested per-cycle command budget: targeted `cargo test` for the new test or
  affected module first; run broader verification before requesting review.
- Codex internal reasoning and local source inspection do not count as external
  LLM cost.

Human approval is required before:

- Adding or changing public DUUMBI operations or stdlib exports.
- Introducing a new external service, credential requirement, production
  registry dependency, or public network dependency.
- Changing CI behavior beyond adding the focused #688 smoke coverage.
- Making a product decision that contradicts `PRODUCT.md`.
- Continuing after a cleanly reproduced architectural blocker.

## Task Breakdown

1. Confirm current issue labels include product and technical approval state
   before starting implementation.
2. Create or update `examples/flagship-http-sqlite-json/` with a non-ignored
   source layout.
3. Add graph/config sources that import the accepted stdlib modules and build a
   SQLite-derived static JSON response.
4. Add README instructions for build, describe, run, request, response,
   cleanup, limitations, and troubleshooting.
5. Add root README and docs links.
6. Add `tests/integration_duumbi688_flagship_example.rs` using the live E2E
   plan above.
7. Run focused formatting/checks for changed Rust tests and Markdown.
8. Run targeted tests for the new E2E path.
9. Run broader verification required by the implementation PR.
10. Record implementation evidence in the PR and issue without using issue
    completion keywords.

## Verification Plan

Minimum local verification for implementation:

```sh
cargo fmt --check
cargo test --test integration_duumbi688_flagship_example
```

Recommended broader verification before implementation review:

```sh
cargo test --all
cargo clippy --all-targets -- -D warnings
```

If `cargo test --all` is too expensive for one Ralph cycle, run the focused
test before pushing and record the broader command as required before review.

The implementation PR must also include review evidence for:

- The exact README command transcript.
- The HTTP response body.
- Process timeout/shutdown behavior.
- Absence of committed runtime state.
- No public internet, credentials, provider setup, or production registry use.

## Completion Criteria

Stage 10 implementation is complete only when:

- The example directory exists and is linked from repository docs.
- The graph/config are tracked and buildable from a clean checkout.
- The live E2E test proves loopback HTTP + SQLite + JSON behavior from the
  compiled DUUMBI binary.
- The default runtime path exits or can be stopped deterministically.
- CI includes the #688 smoke coverage.
- The implementation PR review finds no blocking product, architecture,
  security, or test gaps.
- Issue #688 remains open until the final implementation PR is merged and Stage
  12 closure evidence is recorded.

## Failure And Escalation

Escalate and stop implementation if:

- Existing stdlib server behavior cannot serve a response string built from DB
  query output.
- The graph cannot construct valid JSON without host-side response fabrication
  or new runtime API work.
- The example requires public network, credentials, provider setup, or
  production registry access.
- The only viable checked-in workspace layout requires broad `.gitignore`
  exceptions.
- The live E2E test is flaky because of port binding, process lifetime, or
  unbounded runtime behavior.
- Any gate reviewer raises a blocking product or technical finding.

Escalation output should include:

- The exact command or test that failed.
- The smallest missing capability or broken invariant.
- Whether the issue needs product clarification, runtime API work, or a narrower
  example shape.

## Open Questions

No blocking open questions remain.

Non-blocking implementation choice: the implementation may use either
`:memory:` SQLite or workspace-confined file-backed SQLite. Prefer `:memory:`
unless file-backed storage adds meaningful reader evidence without creating
cleanup or tracking risk.
