# DUUMBI-688: Flagship HTTP + SQLite + JSON Reference Program

## Summary

Add one durable flagship example under `examples/` that a visitor can build,
run, inspect, and use as the first "see DUUMBI work" reference program.

The accepted product shape is a small local JSON API backed by SQLite:

- the example builds as a normal DUUMBI workspace from a clean clone;
- running it starts a bounded loopback HTTP endpoint;
- the endpoint returns JSON derived from a SQLite-backed data set;
- the example README explains the graph structure, stdlib modules, expected
  commands, expected output, and cleanup behavior;
- root README and in-repo docs link to it as the flagship reference example;
- CI fails if the example stops building, serving, or returning the expected
  JSON.

Related to #688. This is a specification-only artifact. The execution issue
must remain open for technical specification, implementation, implementation
review, and Stage 12 closure.

## Problem

The current repository gives evaluators too little evidence that DUUMBI can
build realistic graph programs.

The accepted issue identifies these gaps:

- `examples/` is absent in the inspected worktree.
- Existing benchmark showcases are toy-scale arithmetic intents.
- The current source surface includes JSON, HTTP, SQLite, file, string,
  Result/Option, and resource-oriented stdlib/runtime support, but no visitor
  facing reference program that combines those capabilities.
- Without a flagship example, the claim that DUUMBI can build real software as
  a validated semantic graph is not supported by a runnable artifact.

The product need is a credible first example, not another internal fixture. A
visitor should be able to follow the README, run the example locally, see an
HTTP JSON response backed by SQLite state, and inspect the JSON-LD graph that
produced it.

## Outcome

When this is done:

- The repository contains `examples/flagship-http-sqlite-json/`.
- The example includes a DUUMBI workspace or equivalent buildable graph layout
  that uses accepted stdlib modules for HTTP serving, SQLite, and JSON.
- A visitor can run documented commands from a clean clone and observe a local
  HTTP JSON response whose values come from the example's SQLite data flow.
- The example is deterministic and local by default: loopback only, no public
  internet, no credentials, explicit timeout, explicit request limit, and no
  indefinite wait.
- The example README explains what the program does, which DUUMBI stdlib
  modules it uses, how to build/run/query it, what output to expect, how to
  inspect the graph, and how runtime state is cleaned up.
- The root README and a durable in-repo docs page link to the example as the
  "see it work" entry point.
- CI has a smoke path that builds the example, runs it, drives at least one
  loopback HTTP request, asserts the JSON response, and verifies that the
  example exits or can be stopped deterministically.
- The implementation records enough evidence in the PR and issue for later
  Stage 12 closure to verify the example did not merely compile accidentally.

## Scope

### In Scope

- Create a flagship example directory under
  `examples/flagship-http-sqlite-json/`.
- Add the example graph/workspace files needed for a clean local build.
- Demonstrate all of these capabilities in one coherent runnable path:
  - HTTP over loopback, preferably through `@duumbi/stdlib-server`;
  - SQLite through `@duumbi/stdlib-db`;
  - JSON parsing and/or JSON response construction through
    `@duumbi/stdlib-json` and existing string primitives where needed.
- Use a local SQLite database path confined to the example workspace, or
  `:memory:` if the technical spec shows that this gives better deterministic
  cleanup without weakening the visible SQLite evidence.
- Seed or derive at least one row of data through the graph, query it back from
  SQLite, and return it in a JSON HTTP response.
- Serve only loopback addresses by default.
- Bound runtime behavior with explicit timeout and request-count controls.
- Add an example README with:
  - purpose and expected user outcome;
  - dependency/stdlib module list;
  - build, run, query, and cleanup commands;
  - expected response body and relevant stdout/stderr;
  - graph file map and recommended `duumbi describe` inspection path;
  - limitations and non-goals.
- Add or update source-repo docs so the example is discoverable from the
  README and at least one in-repo docs page.
- Add focused CI or integration-test coverage that prevents the example from
  rotting.
- Add an optional intent or BDD companion artifact only if it stays small,
  deterministic, and useful as a reader-facing demonstration of the AI path.
- Keep the spec-only PR non-closing and limited to this product spec file.

### Explicitly Out Of Scope

- Implementation code, tests, docs edits, generated artifacts, runtime assets,
  or Ralph cycles during Stage 6 and Stage 8.
- New HTTP, SQLite, JSON, TCP, file I/O, Result/Option, server, or runtime
  public APIs for this issue.
- Changing accepted stdlib module contracts from #378, #379, #380, #381, or
  #382.
- Production-grade web framework behavior, dynamic request handlers,
  concurrency, authentication, TLS termination, WebSockets, HTTP/2, cookies,
  streaming bodies, database migrations, connection pools, or ORM behavior.
- Public internet-dependent examples or CI checks.
- Requiring credentials, registry production access, non-loopback services, or
  persistent background daemons.
- Shipping a committed SQLite database generated by a prior run unless the
  technical spec proves it is intentionally static data and not mutable output.
- Broad website or marketing work outside this repository. If public docs live
  in another repository, implementation should create a follow-up or leave a
  documented handoff rather than silently editing unavailable surfaces.
- Treating the example as proof that every Tier 1 stdlib module is registry
  published. #382 owns distribution smoke coverage.
- Marking #688 complete from a spec-only PR.
- Invoking Greptile on the spec PR. Greptile is reserved for the final
  implementation PR.

## Constraints And Assumptions

Facts:

- Issue #688 is open and titled `feat(examples): add flagship reference program
  (HTTP + SQLite + JSON)`.
- Issue #688 has labels `documentation`, `enhancement`, `module:ecosystem`,
  `accepted`, and `needs-spec` in the inspected GitHub state.
- Stage 5 accepted #688 on 2026-06-14 from Slack reviewer source `hga`, with
  no remaining open questions and next state `Spec Needed`.
- Stage 4 routed #688 to `Needs Human Acceptance` as a concrete preview
  blocker with clear acceptance criteria.
- The inspected worktree has no `examples/` directory.
- `README.md` currently documents install, quickstart, Query mode, AI mutation,
  build/test commands, and supported platforms, but does not link to a flagship
  example.
- `src/types.rs` contains JSON, TCP, HTTP server, HTTP client, and database
  operation variants, including `JsonParse`, `JsonStringify`, `JsonGetField`,
  `ServerNew`, `RouteAddStatic`, `ServerStart`, `HttpGet`, `HttpStatus`,
  `HttpBody`, `HttpHeaders`, `DbOpen`, `DbExecute`, `DbQuery`, `DbRowsLen`,
  and `DbRowGet`.
- `stdlib/json.jsonld`, `stdlib/http.jsonld`, `stdlib/db.jsonld`, and
  `stdlib/server.jsonld` exist in the inspected source tree.
- `src/cli/init.rs` seeds opt-in JSON, HTTP, DB, and server stdlib modules into
  local cache metadata while keeping default dependencies focused on core
  modules.
- `tests/integration_duumbi380_e2e.rs` already demonstrates an internal
  HTTP + JSON + SQLite composition test using a local loopback HTTP fixture,
  JSON parsing/stringifying, SQLite insert/query, and deterministic stdout.
- `tests/integration_duumbi382_ecosystem_smoke.rs` contains clean-workspace
  installed-module smoke patterns for Tier 1 stdlib modules.
- Prior specs #379, #380, #381, and #382 define the accepted JSON/TCP,
  HTTP/SQLite, server/publishing, and stdlib ecosystem smoke-test boundaries.
- The active PRD says DUUMBI should connect intent, graph structure, executable
  behavior, runtime feedback, and reviewable evidence.
- The active Agentic Development Map says product behavior should be clarified
  before implementation, technical plans should identify affected files and
  evidence, and GitHub Project/issues/PRs/CI are the execution source of truth.
- The vault inbox note `2026-06-12 - Flagship Examples and Showcase Programs`
  names #688 as the flagship program and asks for real examples that build
  offline from the released binary.

Assumptions:

- A local loopback JSON API is the most visitor-visible accepted shape for
  #688. The issue body permits either a small JSON API or a fetch-store-query
  tool; the vault note selects a small HTTP service backed by SQLite returning
  JSON.
- The implementation can build the JSON API using already accepted stdlib
  server, DB, JSON, string, file, and I/O primitives. If a narrow implementation
  gap appears, the implementation should stop for a scoped product or
  architecture decision instead of expanding the issue into new runtime API
  design.
- One flagship example is enough for #688. Additional examples belong to later
  work unless they are trivial supporting fixtures.
- A checked-in example workspace can be structured differently from `duumbi
  init` output if the README and CI prove that a visitor can still build and
  run it from a clean clone.
- The example may use a small shell/Rust test harness in CI to start the
  example and send the loopback request, but the user-facing flow should remain
  documented in normal terminal commands.

Constraints:

- The example must be deterministic, local, and timeout-bounded by default.
- The example must not require public internet, production registry access,
  credentials, or manual edits before first run.
- The example must not rely on hidden host-side business logic to satisfy the
  HTTP + SQLite + JSON claim. Host scripts may drive the smoke test, but the
  core example behavior must live in DUUMBI graph/workspace artifacts.
- The example must not write outside its workspace or leak runtime state into
  committed files.
- The example should prefer `result<_, string>`-style visible failures and
  explicit cleanup patterns already established by the stdlib specs.
- Documentation must avoid overstating production readiness. This is a
  reference example, not a production web service.
- Spec-only PRs must use non-closing issue references such as `Related to #688`
  or `Spec for #688`.

## Decisions

- **Decision:** Use a file-based product spec at
  `specs/DUUMBI-688/PRODUCT.md`.
  **Evidence:** The issue is non-trivial, durable, user-visible, cross-module,
  and implementation-facing.
- **Decision:** The flagship example will be a local JSON API backed by SQLite,
  not only a command-line fetch-store-query tool.
  **Evidence:** The accepted issue permits a JSON API, and the vault flagship
  examples note specifically identifies a small HTTP service backed by SQLite
  returning JSON as the #688 program.
- **Decision:** Default runtime behavior must be loopback-only, bounded by
  timeout and request count, and safe for CI.
  **Evidence:** Prior stdlib specs #379, #380, #381, and #382 consistently
  require local/loopback, temporary, timeout-bounded network and database
  evidence.
- **Decision:** #688 should not add new stdlib APIs unless implementation
  reveals a small blocking defect in already accepted behavior and a human
  approves scope expansion.
  **Evidence:** Existing source contains the needed capability family; #688 is
  an example/docs issue, not a runtime module issue.

## Behavior

The flagship example should behave as a small, inspectable local service:

- A visitor can run documented commands from the repository root or the example
  directory.
- The example builds through the same DUUMBI build path a user would exercise
  for ordinary graph programs.
- The example initializes or opens its SQLite data store inside the example
  workspace, inserts or prepares deterministic sample data, queries that data,
  constructs a JSON response, registers a loopback HTTP route, and serves the
  response.
- The default route should be simple and memorable, such as `GET /facts` or
  `GET /api/facts`.
- The JSON response should contain at least:
  - a stable example name or service name;
  - one value read from SQLite;
  - one count or status field proving a database query occurred;
  - a field that makes JSON structure visible beyond a plain string response.
- The server must bind only to loopback by default. If a port is configurable,
  the README must document the default and a collision-safe override.
- The server must use an explicit request limit and timeout. The default should
  allow CI and the README flow to finish without leaving a background process.
- If the port is unavailable, the example should fail visibly with a concise
  error and nonzero status, or use a documented deterministic fallback.
- If the SQLite path cannot be created, the JSON body cannot be constructed, or
  the HTTP server cannot start, the example should fail visibly and avoid
  partial hidden success.
- If no request arrives before the timeout, the example should exit or report
  timeout according to the documented behavior; it must not hang indefinitely.
- Runtime state should be created under the example workspace or a temporary
  directory and cleaned up or documented so repeated runs are predictable.
- The README should include a transcript that a visitor can compare to their
  local run. Minor path, port, or timestamp differences are acceptable only
  when documented.
- The README should identify the graph files and recommended inspection
  commands, including a `duumbi describe` path when supported.
- The root README and in-repo docs should make this the primary reference
  example link, not bury it behind internal test names.

## BDD Scenarios

Feature: Flagship HTTP + SQLite + JSON reference program

  Rule: The example is discoverable and buildable from a clean clone

    Scenario: Visitor finds the flagship example from the README
      Given a visitor opens the repository README
      When they look for a runnable DUUMBI example
      Then the README links to `examples/flagship-http-sqlite-json/`
      And the link describes it as the flagship HTTP + SQLite + JSON example

    Scenario: Visitor builds the example without external services
      Given a clean clone with DUUMBI build prerequisites installed
      And no public internet or registry credentials are available
      When the visitor follows the example README build command
      Then DUUMBI builds the example successfully
      And the build does not require production registry access
      And the build does not require public network access

  Rule: The example visibly combines HTTP, SQLite, and JSON

    Scenario: Example serves a SQLite-backed JSON response
      Given the flagship example has been built
      When the visitor runs the example using the documented default command
      And sends the documented loopback HTTP request before the timeout
      Then the response status is successful
      And the response body is valid JSON
      And the JSON includes at least one value read from SQLite
      And the JSON includes a count or status field derived from a SQLite query

    Scenario: The graph structure is inspectable
      Given the visitor opens the example directory
      When they inspect the graph files and run the documented describe command
      Then they can identify the HTTP route setup
      And they can identify the SQLite open/insert/query flow
      And they can identify the JSON response construction or parse/stringify
      And the README maps those areas to the relevant stdlib modules

    Scenario: Runtime behavior is bounded
      Given the example is started with its default settings
      When no HTTP request arrives before the configured timeout
      Then the process reports a timeout or exits according to the README
      And it does not wait indefinitely
      And it does not leave a persistent background daemon

  Rule: The example is stable enough to be release evidence

    Scenario: CI catches example breakage
      Given a pull request changes source, runtime, stdlib, example, or CI
      When CI runs the flagship example smoke path
      Then CI builds the example
      And CI starts the example with loopback-only settings
      And CI sends the documented request
      And CI validates the JSON response shape and key values
      And CI fails with a module/stage-specific error if any step fails

    Scenario: Repeated local runs remain predictable
      Given the visitor has already run the example once
      When they run the documented cleanup or repeat-run command
      Then stale SQLite state does not change the expected response
      And generated runtime artifacts do not appear as untracked committed
      source files

    Scenario: Optional intent artifact demonstrates the AI path
      Given the implementation includes an optional intent or BDD companion
      artifact
      When a visitor opens it
      Then it describes the flagship behavior in plain English
      And it does not require a live LLM call for the default CI smoke path

## Tasks

- Product and technical specification:
  - Draft and approve this product spec.
  - Draft and approve `specs/DUUMBI-688/TECHNICAL.md` before implementation.
- Example artifact:
  - Create `examples/flagship-http-sqlite-json/`.
  - Add the graph/workspace files needed to build and run the example.
  - Add a concise example README and expected transcript.
  - Add optional intent/BDD companion material only if it improves reader
    comprehension without increasing default run risk.
- Verification:
  - Add a focused smoke test or CI path that builds, runs, requests, validates,
    and cleans up the example.
  - Reuse existing local loopback, SQLite, and clean-workspace patterns where
    practical.
- Documentation:
  - Link the example from root `README.md`.
  - Add or update an in-repo docs page that lists runnable examples or points
    to the flagship example.
  - If external public docs are required but unavailable in this repo, record a
    follow-up rather than expanding #688 into cross-repository publishing work.
- Review evidence:
  - Include the final command evidence, response body, cleanup behavior, and
    docs links in the implementation PR.

Independent slices:

- The README/docs link work can be reviewed independently after the example
  path and name are stable.
- The smoke-test harness can be developed independently from the final example
  prose once the run/query contract is fixed.
- Optional intent material can be deferred without blocking #688 if the
  example itself and CI smoke evidence are complete.

## Checks

Product checks:

- `examples/flagship-http-sqlite-json/` exists.
- The example README explains purpose, modules, commands, expected output,
  graph inspection, limitations, and cleanup.
- Root README and in-repo docs link to the example.
- The default example path is local, deterministic, timeout-bounded, and
  credential-free.
- The example does not claim production web-service readiness.

BDD coverage:

- README discoverability maps to a docs/README assertion.
- Clean build maps to a local build command or integration test.
- SQLite-backed JSON response maps to a smoke test that drives one HTTP request
  and parses/asserts response JSON.
- Graph inspectability maps to README content and, where possible,
  `duumbi describe` output.
- Runtime bounds map to a timeout or max-request assertion.
- Repeatability maps to cleanup/re-run checks or use of isolated temp state.

Implementation verification expectations:

- Run `cargo fmt --check`.
- Run the focused example smoke test.
- Run the relevant existing tests that cover HTTP + JSON + SQLite composition,
  such as `cargo test --test integration_duumbi380_e2e`, unless renamed or
  superseded by the technical spec.
- Run any new integration test added for the example.
- Run broader `cargo test --all` or justify a narrower set if the implementation
  is docs/example-only and CI covers the remaining surface.
- Verify no implementation PR files include committed runtime databases,
  generated binaries, local logs, credentials, or temp artifacts.

Live E2E expectation:

- #688 itself does not require a live external LLM call.
- If implementation includes an optional intent-generation demonstration, live
  LLM verification must be optional, low-cost, and outside default CI.
- The default live E2E path is the local DUUMBI CLI build/run/query flow, not a
  public service or production registry call.

Review evidence:

- Codex self-review must find no blocking product, architecture, security,
  runtime, or verification gaps before Stage 7 approval.
- Greptile must not run on spec PRs.
- The implementation PR may use Greptile only if final implementation review
  policy explicitly calls for it.

## Open Questions

None blocking.

Non-blocking implementation choice:

- The technical spec should decide whether the SQLite store is file-backed
  under the example workspace or `:memory:`. The product requirement is visible
  SQLite-backed behavior plus deterministic cleanup; either storage mode is
  acceptable if the technical spec maps it to reliable evidence.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/688
- Stage 4 triage refill:
  https://github.com/hgahub/duumbi/issues/688#issuecomment-4699711458
- Stage 5 human acceptance:
  https://github.com/hgahub/duumbi/issues/688#issuecomment-4702900849
- Root README: `README.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- JSON/TCP product spec: `specs/DUUMBI-379/PRODUCT.md`
- HTTP/SQLite product spec: `specs/DUUMBI-380/PRODUCT.md`
- HTTP/SQLite technical spec: `specs/DUUMBI-380/TECHNICAL.md`
- Server/publishing product spec: `specs/DUUMBI-381/PRODUCT.md`
- Ecosystem smoke product spec: `specs/DUUMBI-382/PRODUCT.md`
- Ecosystem smoke technical spec: `specs/DUUMBI-382/TECHNICAL.md`
- Type/op source: `src/types.rs`
- Stdlib cache/init source: `src/cli/init.rs`
- JSON stdlib graph: `stdlib/json.jsonld`
- HTTP stdlib graph: `stdlib/http.jsonld`
- DB stdlib graph: `stdlib/db.jsonld`
- Server stdlib graph: `stdlib/server.jsonld`
- Existing HTTP + JSON + SQLite E2E fixture:
  `tests/integration_duumbi380_e2e.rs`
- Existing ecosystem smoke harness:
  `tests/integration_duumbi382_ecosystem_smoke.rs`
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active Glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic Development Map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Flagship examples vault note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-12 - Flagship Examples and Showcase Programs.md`
