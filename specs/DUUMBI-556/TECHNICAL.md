# DUUMBI-556: Expose Read-Only Query As First-Class Service Surface - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-556/PRODUCT.md` by making
the existing read-only Query mode visible, coherent, and accurately described
across DUUMBI onboarding, repo docs, public docs source, CLI/REPL/TUI guidance,
and Studio chat copy.

This technical spec covers product-surface alignment only. It must not add a new
query engine, top-level `duumbi query` command, answer-schema backend contract,
provider behavior, graph analysis capability, telemetry, runtime asset, or
implementation of future roadmap items.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Oz implementation agents when routing work from Slack or GitHub.
- Stage 9 technical spec reviewers.
- Stage 10 testers/reviewers verifying that implementation evidence matches the
  approved product and technical specs.

## Source Context

- Product spec: `specs/DUUMBI-556/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/556
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/556#issuecomment-4466549835
- Stage 6 product spec PR: https://github.com/hgahub/duumbi/pull/562
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/556#issuecomment-4454254487
- Query implementation PR: https://github.com/hgahub/duumbi/pull/530
- Repo instructions: `AGENTS.md`

Relevant code and docs verified for Stage 8:

- `README.md`
- `docs/modes/README.md`
- `docs/modes/query-mode-spec.md`
- `docs/modes/implementation-tasks.md`
- `sites/docs/book.toml`
- `sites/docs/src/SUMMARY.md`
- `sites/docs/src/getting-started/quickstart.md`
- `sites/docs/src/cli/overview.md`
- `sites/docs/src/cli/studio.md`
- `src/cli/completion.rs`
- `src/cli/repl.rs`
- `src/cli/app.rs`
- `src/cli/mod.rs`
- `src/interaction/mod.rs`
- `src/interaction/router.rs`
- `src/query/engine.rs`
- `src/query/sources.rs`
- `crates/duumbi-studio/src/app.rs`
- `crates/duumbi-studio/src/script/studio.js`
- `crates/duumbi-studio/src/ws.rs`
- `src/cli/phase15_e2e.rs`

Relevant tests and checks verified for Stage 8:

- `src/cli/completion.rs` has slash-command metadata and tests.
- `src/cli/app.rs` has REPL/TUI rendering and placeholder tests.
- `src/cli/repl.rs` has Query answer formatting and pending-status tests.
- `crates/duumbi-studio/src/ws.rs` has answer-frame serialization tests.
- `src/cli/phase15_e2e.rs` has Studio UX evidence checks for Query as default
  and read-only.
- `sites/docs` is the mdBook source for `https://docs.duumbi.dev/`.

Relevant Obsidian notes:

- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Development Intake to Delivery Workflow.md`

Verified source facts:

- Query is already a shared `InteractionMode` default in `src/interaction/mod.rs`.
- CLI REPL defaults to Query mode and supports `/mode query`, `/query`, and
  `/ask`.
- CLI Query routes through `QueryEngine`, records session turns, displays answer
  metadata, and suggests handoff for mutation-like prompts.
- Studio chat defaults to `query`, sends `mode` in WebSocket chat frames, and
  displays source count, confidence, model, and suggested mode metadata.
- `src/cli/mod.rs` does not define a top-level `duumbi query` subcommand.
- `README.md`, `sites/docs/src/getting-started/quickstart.md`,
  `sites/docs/src/cli/overview.md`, and `sites/docs/src/cli/studio.md` still
  emphasize compiler or mutation workflows before read-only Query.
- `docs/modes/query-mode-spec.md` still says `Status: proposed` even though PR
  #530 merged Query mode implementation on 2026-05-01.

## Affected Areas

Expected implementation changes:

- README onboarding:
  - `README.md`
- Repo docs:
  - `docs/modes/README.md`
  - `docs/modes/query-mode-spec.md`
  - Optional new repo-doc page under `docs/modes/` if a separate Query examples
    page is cleaner than expanding existing files.
- Public docs source:
  - `sites/docs/src/getting-started/quickstart.md`
  - `sites/docs/src/cli/overview.md`
  - `sites/docs/src/cli/studio.md`
  - `sites/docs/src/SUMMARY.md`
  - Optional new page under `sites/docs/src/cli/` or
    `sites/docs/src/getting-started/` for Query examples.
- CLI/REPL/TUI copy surfaces:
  - `src/cli/completion.rs`
  - `src/cli/repl.rs`
  - `src/cli/app.rs`
- Studio copy surfaces:
  - `crates/duumbi-studio/src/app.rs`
  - `crates/duumbi-studio/src/script/studio.js`
- Focused tests for changed copy:
  - Existing tests in `src/cli/completion.rs`, `src/cli/repl.rs`,
    `src/cli/app.rs`, `crates/duumbi-studio/src/ws.rs`, or
    `src/cli/phase15_e2e.rs`, depending on touched copy.

Areas expected not to change:

- `src/query/*`, except only if implementation discovers an existing copy-only
  metadata label bug. Do not change Query engine behavior for this issue.
- `src/interaction/*`, except only if a copied label is inconsistent with the
  approved wording. Do not change mode semantics.
- `src/cli/mod.rs` command enum. Do not add a top-level `duumbi query`.
- Provider setup, model routing, credentials, and model catalog code.
- Graph, parser, compiler, registry, dependency, runtime, MCP mutation, and
  intent execution internals.
- Product spec files, including `specs/DUUMBI-556/PRODUCT.md`.
- Generated site output, `target/`, screenshots, runtime assets, or vendored
  generated artifacts.

CI and local validation paths:

- `cargo fmt --check`
- Focused CLI/TUI unit tests for touched files.
- Focused Studio SSR tests when Studio markup, WebSocket metadata, or UX evidence
  checks change.
- `cargo test --all` when implementation touches shared Rust code across CLI and
  Studio or when focused tests do not cover changed behavior.
- Manual smoke checks for README/docs accuracy, REPL Query read-only behavior,
  mutation-like Query handoff, and Studio Query default behavior.

## Technical Approach

### 1. Keep Existing Runtime Semantics

Treat PR #530 behavior as the runtime boundary. The implementation should expose
and document existing Query mode instead of extending it.

Allowed runtime-facing copy changes:

- Make Query read-only wording more explicit.
- Make Agent and Intent write-capable boundaries more explicit.
- Explain currently exposed answer metadata: answer text, sources count or
  source references where visible, confidence, model, and suggested handoff.
- Point users to provider setup when Query cannot answer because no LLM provider
  is configured.

Rejected alternatives:

- Adding `duumbi query` as a top-level shell command.
- Implementing full answer-schema fields such as claim labels, node IDs,
  dependency impact, or formal risk scoring as a backend contract.
- Changing Query context assembly or source confidence behavior.
- Hiding Agent/Intent mutation flows behind Query-first copy.

### 2. README Onboarding

Update `README.md` so Query appears before AI mutation. Keep the quickstart
accurate to existing commands:

- Show `duumbi` interactive usage as the entry point for Query.
- Show `/query "..."` and `/ask "..."` as REPL slash commands, not shell
  commands.
- Include at least one short example question that matches product-spec examples.
- Keep build/run/check/describe examples, but do not let them be the only first
  path.
- Keep AI mutation examples after Query-first onboarding and label them as
  write-capable.

### 3. Repo Docs

Update `docs/modes/query-mode-spec.md` from purely proposed status to a status
that separates delivered v1 behavior from future work. Suggested shape:

- `Status: delivered v1 / future work noted`
- Add a short "Delivered v1" section listing PR #530 behavior.
- Add a short "Future work" section for standard answer schema hardening and
  richer evidence/risk fields.
- Preserve read-only safety boundaries and mode semantics.

Update `docs/modes/README.md` or add a small Query examples page so repo docs
contain at least these examples:

- "What exists?" example.
- "Where does behavior live?" example.
- "What risk does this change carry?" example with conservative wording.

### 4. Public Docs Source

Update mdBook source under `sites/docs` because `sites/docs/book.toml` declares
`site-url = "https://docs.duumbi.dev/"`.

Expected changes:

- `sites/docs/src/getting-started/quickstart.md`: add a Query-first step before
  AI mutation.
- `sites/docs/src/cli/overview.md`: document interactive `duumbi` Query usage
  and REPL slash commands without inventing a top-level shell command.
- `sites/docs/src/cli/studio.md`: describe Studio Query as the read-only default
  and Agent as the write-capable mutation mode.
- `sites/docs/src/SUMMARY.md`: add any new Query docs page if one is created.

The implementation PR should state that the docs source was updated. It should
not publish the public site unless the release workflow explicitly does that.

### 5. CLI/REPL/TUI Copy

Review and adjust existing copy only where it improves consistency:

- `src/cli/completion.rs`: `/query`, `/ask`, `/agent`, and `/mode`
  descriptions.
- `src/cli/repl.rs`: `/mode` usage, `/query` usage, mutation-like Query warning,
  provider-unavailable Query message, and answer metadata label.
- `src/cli/app.rs`: Query/Agent/Intent placeholders and any empty-state guidance
  that should present Query-first discovery.

Do not add provider warnings on startup. Do not change `Esc` panel behavior,
mode switching behavior, or query execution flow.

### 6. Studio Copy

Review and adjust Studio chat copy without changing protocol semantics:

- `crates/duumbi-studio/src/app.rs`: chat welcome text, tab titles, and input
  placeholder if they still imply mutation-first behavior.
- `crates/duumbi-studio/src/script/studio.js`: per-mode placeholders, pending
  Query label, disconnected-state text, answer metadata label, and handoff copy.

Keep Query as the default `chatMode`. Keep Agent mutation refresh behavior
unchanged. Query answer frames must not trigger graph refresh.

### 7. Tests And Evidence

For each changed code copy surface, add or adjust focused tests near the local
pattern:

- `src/cli/completion.rs`: assert slash command descriptions for Query, Ask,
  Agent, and Mode when changed.
- `src/cli/app.rs`: assert Query default, prompt placeholder, and any changed
  empty-state copy.
- `src/cli/repl.rs`: assert Query answer metadata and mutation-like handoff copy
  when changed.
- `crates/duumbi-studio/src/ws.rs`: keep answer-frame metadata serialization
  stable if labels/fields change.
- `src/cli/phase15_e2e.rs`: adjust UX evidence tests if Studio Query tab titles
  or Agent tab titles change.

Documentation-only changes do not need Rust tests unless they are paired with
code copy changes. They still need review evidence showing examples are command
accurate and do not imply `duumbi query`.

## Invariants

- Query remains read-only by behavior and by visible copy.
- Query-to-Agent and Query-to-Intent handoffs remain explicit user actions.
- No implementation should add a top-level `duumbi query` command for this
  issue.
- No implementation should claim full standard answer schema fields as delivered
  runtime guarantees.
- Agent and Intent remain visible as write-capable paths; DUUMBI must not read
  as only a chat interface.
- Provider setup remains through `/provider` or `duumbi provider`; no default
  model maintenance UI should be introduced.
- TUI must remain quiet, bounded, keyboard-complete, and free of duplicate
  hints.
- `Esc` behavior must not regress.
- Studio Query must not refresh the graph after answers.
- Product specs and technical specs must not be edited during implementation
  cycles unless a later stage explicitly routes spec revision.
- Implementation PRs must include evidence for changed docs/help/copy surfaces.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas and commands
4. estimate resource use and risk
5. ask for explicit approval before starting
6. implement only the approved goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop if requirements are met or request approval for the next cycle

No Ralph cycle may start implementation edits without explicit approval. No
cycle may broaden scope into Query engine architecture, provider behavior, or
new CLI commands without returning to human guidance.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: 4 files for docs-only cycles, or 3 files when
  Rust/Studio code copy and tests are included.
- Expected command budget per cycle:
  - docs-only: `git diff --check` and targeted review of changed Markdown.
  - CLI/TUI code-copy cycle: `cargo fmt --check` plus focused `cargo test`
    filters for changed modules.
  - Studio code-copy cycle: `cargo fmt --check` plus focused Studio/UX tests, or
    `cargo test -p duumbi-studio --features ssr` when WebSocket or SSR code is
    touched.
  - final integration cycle: `cargo test --all` when code changes span both CLI
    and Studio or when focused tests are insufficient.
- Approval required before every cycle: yes.
- When to stop and ask for human guidance:
  - A change would require a new top-level command.
  - A change would alter Query engine behavior or answer schema serialization.
  - A docs claim depends on future telemetry, formal evidence, or semantic reuse.
  - Public docs publication requires credentials or external deployment.
  - Tests reveal existing behavior conflicts with the approved product spec.
  - More than two cycles are needed after docs/copy/test alignment is expected to
    be complete.

## Task Breakdown

1. README and command-accuracy pass.
   - Update `README.md` with Query-first onboarding before mutation examples.
   - Keep shell commands and REPL slash commands clearly separated.
   - Verify there is no `duumbi query` claim.

2. Repo docs pass.
   - Update `docs/modes/query-mode-spec.md` status and delivered-vs-future
     sections.
   - Update `docs/modes/README.md` or add a Query examples page.
   - Include examples for what exists, where behavior lives, and change risk.

3. Public docs source pass.
   - Update `sites/docs/src/getting-started/quickstart.md`.
   - Update `sites/docs/src/cli/overview.md`.
   - Update `sites/docs/src/cli/studio.md`.
   - Update `sites/docs/src/SUMMARY.md` if a new page is added.

4. CLI/REPL/TUI copy pass.
   - Review and update `src/cli/completion.rs`, `src/cli/repl.rs`, and
     `src/cli/app.rs` only where current copy conflicts with Query-first
     product framing.
   - Add focused tests near changed code.

5. Studio copy pass.
   - Review and update `crates/duumbi-studio/src/app.rs` and
     `crates/duumbi-studio/src/script/studio.js` only where current copy
     conflicts with Query-first framing.
   - Adjust focused UX evidence tests if changed strings are test-covered.

6. Final evidence pass.
   - Run agreed checks.
   - Confirm no forbidden files changed.
   - Confirm examples do not imply missing commands or future schema guarantees.
   - Record public-docs-source status and manual smoke evidence in the
     implementation PR.

## Verification Plan

Required static checks:

- `git diff --check`
- `cargo fmt --check` if any Rust code changes.

Focused automated checks, selected by changed files:

- `cargo test completion`
- `cargo test query_answer_formats_thinking_answer_and_model_metadata`
- `cargo test query_pending_status_animates_three_dots`
- `cargo test new_starts_in_query_mode`
- `cargo test conversation_render_wraps_long_query_output`
- `cargo test studio_ux_evidence`
- `cargo test -p duumbi-studio --features ssr answer_frame_serializes_model_metadata`

Broader automated checks:

- `cargo test -p duumbi-studio --features ssr` if Studio Rust or WebSocket code
  changes beyond string-only markup.
- `cargo test --all` if changes span shared CLI and Studio code or focused tests
  do not cover the modified behavior.

Manual checks:

- README quickstart is command-accurate and presents Query before mutation.
- `sites/docs` quickstart and CLI overview distinguish shell commands from REPL
  slash commands.
- `docs/modes/query-mode-spec.md` no longer reads as purely speculative for PR
  #530 delivered behavior.
- REPL Query examples remain read-only when run with a configured provider.
- Mutation-like input in Query produces a handoff suggestion rather than a write.
- Studio Query tab remains default and read-only; Agent remains the mutation
  path.
- Query answer metadata copy does not promise full future answer-schema fields.

Implementation PR evidence must include:

- Changed surfaces.
- Commands run and results.
- Manual smoke evidence or explicit reason a smoke path was not run.
- Confirmation that no product specs, technical specs, generated artifacts,
  runtime assets, or implementation-unrelated files were changed.

## Completion Criteria

The implementation is ready for PR review when:

- README introduces Query as the safe read-only path before write-capable
  mutation examples.
- Repo docs include discoverable Query examples for what exists, where behavior
  lives, and what risk a change carries.
- `sites/docs` source reflects Query-first public docs coverage.
- `docs/modes/query-mode-spec.md` distinguishes delivered v1 behavior from
  future answer-schema work.
- CLI/REPL/TUI copy consistently describes Query as read-only and Agent/Intent
  as write-capable.
- Studio visible copy consistently describes Query as read-only while preserving
  Agent mutation discoverability.
- Tests are added or updated for changed code-copy surfaces where local patterns
  exist.
- No new top-level `duumbi query` command exists.
- No Query engine, provider, graph, runtime, registry, or intent execution
  behavior is changed.
- Verification commands pass or any failures are explained with concrete
  follow-up.

## Failure And Escalation

- If tests fail because changed copy invalidates an existing assertion, update
  the assertion only when the new copy matches this technical spec and the
  product spec.
- If tests fail because behavior changed accidentally, revert or repair the
  behavior inside the approved cycle scope before reporting.
- If docs require claims that current Query metadata cannot support, downgrade
  the claim to delivered v1 behavior or stop for human guidance.
- If a required docs.duumbi.dev publication step is outside this repository,
  update `sites/docs` only and report the external publish step as follow-up
  evidence.
- If implementation needs a new command, new answer schema, new provider
  behavior, or Query engine change, stop and route back to human guidance because
  that exceeds the approved scope.
- If the cycle budget is exceeded, stop with evidence and ask for the next
  bounded approval rather than continuing.

## Open Questions

None blocking implementation.

Non-blocking implementation detail: the implementation PR should state whether
updating `sites/docs` is sufficient for docs.duumbi.dev publication or whether a
separate deployment step remains outside the repository.
