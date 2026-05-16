# DUUMBI-556: Expose Read-Only Query As First-Class Service Surface

## Summary

Make DUUMBI's read-only `query` workflow visible and coherent across the first
product surfaces users inspect: README onboarding, repo/public docs, CLI/REPL/TUI
guidance, and Studio chat copy.

This is a product-surface alignment pass over existing Query mode behavior. It
must not expand the query engine, provider architecture, graph analysis, or
answer-schema backend contract beyond what is already delivered. It should make
the current capability easier to discover, try, and trust, while clearly
separating delivered v1 metadata from future answer-schema hardening.

## Problem

DUUMBI's active product direction says users should ask read-only questions
before choosing write-capable Agent or Intent workflows. They should be able to
ask what exists, where behavior lives, why it behaves that way, what evidence is
available, what depends on it, and what risk a change carries before mutation
begins.

The source repository already contains Query mode implementation from PR #530:
`src/query/*`, CLI REPL Query handling, `/query` and `/ask` one-shot commands,
Studio chat mode tabs, source/confidence/model metadata, and read-only handoff
behavior. The product surfaces do not yet tell that story consistently. The
README still leads with compiler and mutation examples, the existing query mode
spec is still marked proposed, and public-facing docs/help do not present Query
as the safe first path.

The risk is product trust. If DUUMBI hides Query mode, evaluators will treat the
product as only a compiler or write-capable mutation tool. If DUUMBI overstates
Query mode, users will expect source-grounded evidence and risk analysis that v1
does not fully enforce yet.

## Outcome

When this is done:

- New users can discover Query mode from README onboarding before they encounter
  write-capable Agent or Intent examples.
- CLI/REPL/TUI guidance consistently describes Query as read-only, Agent as
  bounded graph mutation, and Intent as spec-driven feature work.
- Studio chat makes Query the obvious read-only default and explains answer
  metadata without implying graph mutation.
- Repo docs include a discoverable Query page or section with concrete examples
  for "what exists", "where behavior lives", and "what change risk exists".
- `docs/modes/query-mode-spec.md` no longer reads as purely speculative for v1
  behavior already delivered in PR #530.
- The docs and visible copy describe the current answer metadata users can
  expect: answer text, sources count or source references where exposed,
  confidence, model label, and suggested handoff when relevant.
- Future answer-schema ideas are preserved as future work instead of being
  silently implied as delivered behavior.
- Existing Query, Agent, Intent, provider setup, and graph mutation behavior
  remain unchanged except for copy, docs, and tests that verify those surfaces.

## Scope

### In Scope

- Add README onboarding that presents `duumbi` interactive Query mode as the
  safe read-only path before mutation examples.
- Document REPL one-shot commands `/query <question>` and `/ask <question>`
  where REPL slash-command examples are appropriate.
- Keep examples accurate to existing commands. Do not imply a top-level
  `duumbi query` shell subcommand unless implementation adds one through a
  separately accepted scope.
- Update CLI help, REPL/TUI copy, placeholders, slash-command descriptions, or
  help text where existing patterns support it.
- Update Studio-facing visible copy or docs so Query is framed as read-only and
  its metadata is understandable.
- Add or update repo docs for Query examples and Query/Agent/Intent boundaries.
- Reconcile `docs/modes/query-mode-spec.md` status so delivered v1 behavior and
  future schema work are distinct.
- Add focused tests or snapshot-style checks for changed help/copy surfaces
  where the repository already has local patterns.
- Preserve the explicit handoff rule: Query can recommend Agent or Intent, but
  write-capable work requires an explicit user action.
- Use repo docs as the minimum canonical public documentation source for this
  issue. If a docs.duumbi.dev source is present in the source repo, update it;
  if it is external or unavailable, record that limitation in review evidence.

### Explicitly Out Of Scope

- Rewriting Query engine architecture.
- Adding new graph analysis, dependency analysis, semantic search, telemetry, or
  formal evidence capabilities.
- Implementing the full standard answer schema as a backend/runtime contract.
- Adding a top-level shell command such as `duumbi query`.
- Changing provider setup, model routing, credentials, or default model UX.
- Changing Agent or Intent mutation semantics.
- Starting Phase 13 telemetry, Phase 16 Windows work, registry reuse
  recommendations, marketing launch content, or broader docs-site redesign.
- Creating technical specs, implementation code outside this issue's eventual
  implementation PR, or Ralph-cycle instructions during Stage 6.

## Constraints And Assumptions

Facts:

- Issue #556 is open, accepted, labeled `accepted` and `needs-spec`, and in
  Project status `Spec Needed`.
- Stage 5 accepted the issue on 2026-05-14 and routed it to `Spec Needed`.
- PR #530 merged on 2026-05-01 and implemented Query mode across CLI and Studio.
- `src/query/engine.rs` answers through the provider text API, assembles
  read-only context, returns sources, confidence, model label, and optional
  handoff.
- CLI REPL defaults to Query mode, supports `/mode query`, `/query`, `/ask`, and
  warns when a Query input looks mutation-like.
- Studio chat has Query, Agent, and Intent tabs, defaults chat mode to `query`,
  sends WebSocket chat frames with `mode`, and displays answer metadata.
- `README.md` currently introduces build/run/check/describe and AI mutation, but
  does not introduce Query as a first-class workflow.
- `docs/modes/query-mode-spec.md` still says `Status: proposed`.
- Active DUUMBI product notes state that Query should be the first service UX and
  should be visible in CLI, Studio, README, and public docs.

Assumptions:

- The near-term product value is discoverability and accurate framing, not
  deeper backend query capability.
- Users need to see Query before mutation so question-shaped prompts do not feel
  like write-capable requests.
- Source-grounding copy should be conservative because v1 metadata is useful but
  not yet the full standard answer schema described in roadmap notes.
- Repo docs are the reliable source available to this issue. External publication
  to docs.duumbi.dev may depend on a separate docs pipeline.

Constraints:

- Query must remain read-only by contract and by user-facing copy.
- The implementation must not create graph, intent, registry, provider, or
  workspace writes as a side effect of asking a question.
- Visible copy must not claim future capabilities such as telemetry-backed risk,
  formal evidence, semantic repository reuse, or Windows support.
- Documentation examples must distinguish REPL slash commands from shell
  commands.
- Existing provider setup remains the user-facing entry point through
  `/provider` or `duumbi provider`; `/model` remains compatibility-only.

## Decisions

- **Decision:** Use a file-based product spec for #556.
  **Evidence:** The work spans README, docs, CLI/TUI, Studio, and tests. It is
  user-visible and durable enough to need review history.

- **Decision:** v1 does not include the full standard answer schema as a backend
  contract.
  **Evidence:** Stage 5 explicitly called out schema scope risk. The accepted
  issue is about exposure, documentation, and UX contract over existing
  behavior. Full schema hardening should be a follow-up issue if still desired.

- **Decision:** v1 docs should describe current answer metadata and show example
  answer shape, not promise every future schema field.
  **Evidence:** Existing code exposes answer text, sources, confidence, model,
  and optional handoff. Roadmap notes also mention claim labels, node IDs,
  symbols, evidence, dependency impact, and risk, but those are not all enforced
  as a stable runtime schema today.

- **Decision:** README should show interactive Query mode and REPL one-shot
  `/query` or `/ask` examples, but should not claim a top-level shell
  `duumbi query` command.
  **Evidence:** CLI code supports Query through the REPL/TUI and slash commands;
  the command enum does not define a top-level `query` subcommand.

- **Decision:** Repo docs are required for this issue; docs.duumbi.dev updates
  are required only if the public-docs source is available in this repository or
  through the normal project documentation pipeline.
  **Evidence:** The Stage 4 source asks for public docs, but Stage 6 has verified
  only repo docs and source context locally. Blocking implementation on an
  unavailable external docs source would expand workflow risk without improving
  the product contract in this repo.

- **Decision:** `docs/modes/query-mode-spec.md` should be reconciled as a
  delivered/partial v1 contract plus future work, not deleted.
  **Evidence:** It remains useful architecture and behavior context, but the
  current `Status: proposed` line conflicts with PR #530 being merged.

- **Decision:** This spec PR must not close the execution issue.
  **Evidence:** Stage 6 creates a reviewable product spec. Stage 7, Stage 8, and
  later implementation still need to happen. PR text should use "Spec for #556"
  or "Related to #556" and avoid GitHub auto-close wording for #556.

## Behavior

### Defaults

- Query is presented as the default read-only path for understanding a DUUMBI
  workspace.
- Agent is presented as write-capable bounded graph mutation.
- Intent is presented as write-capable spec-driven feature work.
- Query examples appear before mutation examples in onboarding.
- Query answers are described as grounded in available DUUMBI workspace context,
  with metadata showing sources, confidence, model, and handoff where available.

### Inputs

- User questions in Query mode.
- Current workspace graph, intent, knowledge, and session context that Query
  already reads.
- Visible module and C4 level in Studio when available.
- README/docs/help copy and existing test patterns for changed surfaces.

### Outputs

- Updated onboarding and docs that make Query discoverable.
- Updated user-facing copy that makes mode boundaries consistent.
- Examples that show:
  - what exists in the workspace
  - where a behavior lives
  - what change risk or handoff path is visible
- Clear metadata explanation for sources, confidence, model, and suggested
  handoff.
- Review evidence showing what docs/copy surfaces changed and what checks passed.

### Visible States

- Query mode should be visibly read-only in CLI/TUI and Studio.
- Provider-unavailable states should remain honest: Query needs an available LLM
  provider and should direct users to provider setup without startup noise during
  unrelated flows.
- Mutation-like input in Query should produce a handoff suggestion, not mutate.
- Agent/Intent examples should remain present so DUUMBI does not look like only a
  chat interface.

### Error And Empty States

- If no provider is configured, Query documentation should point to provider
  setup rather than implying offline LLM answers.
- If a workspace has little graph context, docs should avoid promising high
  confidence answers.
- If source metadata is incomplete, Query may answer with lower confidence.
- If public docs publication is unavailable during implementation, the PR should
  document that repo docs were updated and identify the missing external publish
  step.

### Accessibility And Focus

- TUI changes must preserve existing keyboard-complete behavior.
- `Esc` behavior and active-panel closing semantics must not regress.
- Studio tab labels and titles should remain concise and understandable.
- Copy changes must not introduce clipped prompts or duplicate hints.

### Invariants

- Query mode never mutates graph files, intent files, registry state, provider
  configuration, or workspace artifacts as part of answering.
- Query-to-Agent or Query-to-Intent handoff is explicit.
- Docs do not claim future answer-schema fields as delivered runtime guarantees.
- Changed help/copy surfaces remain test-covered where the repo has feasible
  local patterns.

## Tasks

- README onboarding:
  - Add a Query-first quickstart step before build/mutation examples.
  - Show the difference between interactive `duumbi` usage and REPL slash
    commands.
  - Keep mutation examples after the read-only workflow.

- Repo and public docs:
  - Add or update a Query mode page/section with at least three concrete
    examples: what exists, where behavior lives, and what risk a change carries.
  - Document current answer metadata and explicit handoff behavior.
  - Reconcile `docs/modes/query-mode-spec.md` status and split delivered v1 from
    future answer-schema work.

- CLI/REPL/TUI copy:
  - Review `/help`, completion descriptions, placeholders, mode labels, and
    mutation-like Query warnings for consistent mode language.
  - Add focused tests for changed text where existing unit tests or snapshot
    helpers apply.

- Studio copy and docs:
  - Review chat tab titles, placeholders, welcome text, and answer metadata copy.
  - Ensure Query remains the default read-only visible state.
  - Add focused tests where existing Studio SSR or UX evidence tests apply.

- Review evidence:
  - Record changed surfaces and checks in the implementation PR.
  - Explicitly state whether docs.duumbi.dev source was updated, unavailable, or
    covered by repo docs publication.

The README/docs work can run independently from CLI/TUI and Studio copy review.
Tests should be adjusted near the changed surface to keep failures easy to
diagnose.

## Checks

- `cargo fmt --check`
- Focused tests for changed CLI/TUI copy, for example relevant `src/cli/app.rs`,
  `src/cli/repl.rs`, or `src/cli/completion.rs` tests.
- Focused Studio tests when Studio visible copy changes, for example
  `cargo test -p duumbi-studio --features ssr` or narrower matching tests.
- `cargo test --all` when the implementation touches shared CLI/Studio behavior
  or existing test coverage makes the cost reasonable.
- Manual smoke evidence:
  - README examples are accurate and do not imply a missing top-level command.
  - REPL `/query "what modules exist?"` remains read-only when a provider is
    configured.
  - Mutation-like text in Query produces a handoff suggestion instead of a write.
  - Studio Query tab remains default and does not refresh graph state after an
    answer.
- PR review evidence lists changed docs/help/copy surfaces and notes any
  unavailable public-docs publication step.

## Open Questions

None blocking for Stage 7 review.

Non-blocking implementation detail: if docs.duumbi.dev is generated from a
source outside this repository, which publish step should carry the repo docs
update to the public site? The implementation PR can record this as unavailable
if the source is not present.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/556
- Stage 5 decision comment:
  https://github.com/hgahub/duumbi/issues/556#issuecomment-4454254487
- Query implementation PR: https://github.com/hgahub/duumbi/pull/530
- README onboarding: `README.md`
- Query mode spec: `docs/modes/query-mode-spec.md`
- Operating modes docs: `docs/modes/README.md`
- Query engine: `src/query/engine.rs`
- Query context assembly: `src/query/context.rs`
- Query answer metadata: `src/query/sources.rs`
- CLI REPL Query handling: `src/cli/repl.rs`
- CLI REPL UI state and placeholders: `src/cli/app.rs`
- CLI slash completion: `src/cli/completion.rs`
- Studio chat markup: `crates/duumbi-studio/src/app.rs`
- Studio chat client: `crates/duumbi-studio/src/script/studio.js`
- Studio WebSocket Query handling: `crates/duumbi-studio/src/ws.rs`
- Active PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Service and Research Direction:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- Agentic Development Map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Development workflow:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Development Intake to Delivery Workflow.md`
