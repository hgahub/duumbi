# DUUMBI-489: Phase 15 Final E2E Walkthrough Protocol

## Summary

Consolidate the Phase 15 end-to-end evidence into one operational walkthrough for proving DUUMBI across the CLI REPL and Studio workflow.

The final walkthrough should cover all three representative Phase 15 samples:

- Calculator from #486.
- String Utilities from #487.
- Math Library from #488.

The artifact should help a developer or reviewer run the protocol from a fresh local checkout, understand prerequisites, execute the sample paths, inspect evidence, classify failures, and decide whether Phase 15 has enough evidence to support the Studio workflow claim.

## Problem

Phase 15 is evidence-oriented: DUUMBI needs credible proof that intent-driven work can move through CLI and Studio surfaces in a way a human can rerun and inspect. The current walkthrough is useful but incomplete. It explicitly covers only Calculator and String Utilities, and it says not to expand into #488 or #489.

That leaves a developer-experience and review gap. Individual sample issues can pass while Phase 15 remains hard to close because the commands, prerequisites, timing expectations, provider setup, Studio checks, report fields, troubleshooting, and issue evidence are spread across issue threads, PRs, and partial documentation.

## Outcome

When this is done:

- `docs/testing/phase15-walkthrough.md` is the canonical Phase 15 all-samples walkthrough.
- A developer can follow the document to validate Calculator, String Utilities, and Math Library through CLI and Studio.
- The walkthrough states the prerequisites for Rust, the local binary, Studio, provider configuration, and evidence output paths.
- Each sample has copy-pasteable commands, expected graph/build/run evidence, timing targets, and pass/fail criteria.
- Studio validation steps cover the simplified `Intents`, `Graph`, and `Build` workflow for each sample.
- Query mode remains documented and verified as read-only; mutation remains Agent mode or explicit intent execution.
- Provider, graph, compiler, run-output, report, and Studio shared-backend failures are distinguishable.
- The document links the canonical issues and evidence for #486, #487, #488, and #489.
- Phase 15 reviewers can use the document as the operational protocol instead of reconstructing evidence from history.

## Scope

### In Scope

- Update `docs/testing/phase15-walkthrough.md` into the final all-samples Phase 15 protocol.
- Cover CLI REPL validation for Calculator, String Utilities, and Math Library.
- Cover Studio validation for Calculator, String Utilities, and Math Library.
- Include prerequisite setup for the Rust toolchain, C compiler, local DUUMBI binary, Studio build, and provider configuration.
- Include expected evidence for graph modules, generated functions, build output, run output, report fields, elapsed timing, UX checks, and Ralph Gate guidance.
- Include sample-specific pass criteria:
  - Calculator: calculator module, arithmetic operations, build/run success, Studio graph/build/run visibility.
  - String Utilities: `string/utils`, reverse, vowel count, palindrome behavior, build/run success, Studio graph/build/run visibility.
  - Math Library: canonical Math Library evidence defined by #488, build/run success, Studio graph/build/run visibility.
- Include troubleshooting guidance for missing credentials, provider authentication/rate-limit/network/timeout failures, graph evidence mismatches, compiler failures, run-output mismatches, report serialization failures, and Studio shared-backend failures.
- Preserve traceability to #486, #487, #488, #489, and relevant implementation PRs.
- Record whether the final accepted evidence uses one provider for all samples or documents the provider per sample.
- Make the main walkthrough operational, with historical logs linked rather than embedded as the primary path.

### Explicitly Out Of Scope

- Implementing or changing the Math Library harness or generated sample behavior for #488.
- Changing CLI, Studio, provider, graph, build, or run behavior except through later approved implementation work.
- Creating Phase 16 Windows work, Phase 13 telemetry or self-healing work, Phase 14 launch work, or stdlib ecosystem work.
- Creating public marketing copy or external claims that Phase 15 is complete.
- Product spec approval, technical spec creation, Ralph-cycle authorization, or implementation work as part of this Stage 6 artifact.

## Constraints And Assumptions

Facts:

- #489 is the accepted canonical issue for the final Phase 15 all-samples walkthrough.
- Stage 5 Human Acceptance exists for #489 and routes it to `Spec Needed`.
- #489 is in the Phase 15 milestone.
- #486 Calculator evidence is complete.
- #487 String Utilities evidence is complete and closed by PR #546.
- #488 Math Library is accepted but not yet complete at the time this product spec is drafted.
- The current walkthrough covers Calculator and String Utilities only and explicitly excludes #488 and #489.
- The current Phase 15 harness supports `calculator` and `string-utils`; `math-library` remains tied to #488.

Assumptions:

- Stage 6 product spec drafting may proceed now because the human explicitly requested it, but implementation of the final walkthrough remains gated on #488 evidence.
- #488 is a hard implementation prerequisite for #489. The final all-samples document should not be completed with speculative Math Library evidence.
- The final protocol may document the provider used for each accepted evidence run unless Stage 7 decides that one provider must be used across all three samples.
- Screenshot-ready Studio steps are sufficient unless Stage 7 explicitly requires committed screenshots.
- The existing `docs/testing/phase15-walkthrough.md` remains the target document unless Stage 8 identifies a stronger documentation structure.

Constraints:

- Do not claim Phase 15 completion unless the final walkthrough matches real accepted evidence.
- Do not expose raw provider secrets in documentation, report examples, logs, screenshots, or issue comments.
- Missing credentials must be explained as a setup or provider-evidence condition, not as a deterministic product failure.
- Provider failures must be distinguishable from graph, compiler, run-output, report, and Studio validation failures.
- Query mode must remain read-only in the protocol and in any described validation path.
- The walkthrough must remain copy-pasteable enough for a reviewer to run without knowing the issue history.

## Decisions

- **Decision:** #489 should use a file-based product spec.
  **Evidence:** The work is durable, cross-surface documentation that will guide technical specification, implementation, review, and later Phase 15 closure.

- **Decision:** #488 is a hard prerequisite for implementation, not a blocker for Stage 6 drafting.
  **Evidence:** Stage 5 recorded this question explicitly, and the user requested Stage 6 now. A product spec can define the dependency while preventing speculative implementation.

- **Decision:** The final walkthrough should be operational rather than historical.
  **Evidence:** #489's user outcome is that a developer can run one reliable protocol without reconstructing prerequisites, commands, timing expectations, provider setup, troubleshooting, or evidence from scattered history.

- **Decision:** The final walkthrough should preserve traceability to sample issues and PR evidence.
  **Evidence:** DUUMBI workflow relies on issue, spec, PR, and evidence links for human review and closure.

- **Decision:** Provider evidence should be explicit per accepted run unless Stage 7 requires a single-provider rule.
  **Evidence:** Existing Phase 15 evidence has used provider-parameterized execution and more than one configured provider path; requiring one provider across all samples is a review policy decision, not necessary for product specification.

## Behavior

### Defaults

- The canonical output document is `docs/testing/phase15-walkthrough.md`.
- The default protocol sequence is:
  1. Verify prerequisites.
  2. Build the local CLI and Studio artifacts needed for the walkthrough.
  3. Run Calculator evidence.
  4. Run String Utilities evidence.
  5. Run Math Library evidence after #488 is complete.
  6. Validate Studio behavior against generated workspaces.
  7. Review reports, timing, failure categories, and issue traceability.
- The document should prefer concrete commands and expected evidence over prose-only explanations.

### Inputs

- Local DUUMBI source checkout.
- Rust toolchain and C compiler.
- Built DUUMBI CLI binary.
- Built Studio artifact or documented Studio command, as required by the current implementation.
- Provider selection and credentials through the existing provider configuration mechanism.
- Output paths for Phase 15 JSON reports.
- Links to accepted evidence for #486, #487, and #488.

### Outputs

- Updated `docs/testing/phase15-walkthrough.md`.
- Copy-pasteable CLI commands for each sample.
- Studio validation steps for each sample.
- Expected module/function/build/run evidence for each sample.
- Expected report fields and example report output paths.
- Timing targets and interpretation guidance.
- Troubleshooting table or equivalent structured guidance.
- Traceability links to issues, specs, PRs, reports, and evidence comments.

### Success States

- A reviewer can follow the walkthrough from a fresh local checkout and know what to run first.
- All three sample sections identify the task id, provider command, output report path, expected generated module evidence, expected build/run evidence, Studio checks, and pass criteria.
- The document says how to classify common failure modes without reading source code.
- The document identifies #488 as the prerequisite source for Math Library evidence and does not invent evidence before #488 is complete.
- The document makes it clear whether the evidence was collected with one provider across all samples or records the provider per sample.
- The document links the canonical issues and relevant PR or evidence comments.

### Empty And Error States

- If provider credentials are missing, the walkthrough should tell the user how to identify the missing configuration without revealing secrets.
- If a provider request fails because of authentication, rate limits, network, malformed output, or timeout, the walkthrough should map that to the expected failure category.
- If graph evidence is missing or wrong, the walkthrough should distinguish missing module files from missing expected function semantics.
- If build succeeds but run output is wrong, the walkthrough should classify the problem as run-output or evidence mismatch rather than provider failure.
- If Studio cannot attach to the generated workspace, the walkthrough should distinguish shared-backend or server setup failure from CLI evidence failure.
- If Math Library evidence is not yet accepted, the walkthrough update should remain incomplete and implementation should stop rather than speculating.

### Timing And Performance

- The walkthrough should state expected elapsed-time targets:
  - Calculator target: under 10 minutes for a normal single-sample run.
  - String Utilities target: under 15 minutes for a normal single-sample run.
  - Math Library target: under 15 minutes for a normal single-sample run, unless #488 evidence justifies a different target.
- Timeout and retry behavior should be described in terms of evidence interpretation, not as a promise that every provider run will complete.
- The protocol should explain where elapsed time appears in the report and how a reviewer should compare it with the target.

### Studio Behavior

- Studio steps should validate the three visible workflow areas: `Intents`, `Graph`, and `Build`.
- For each sample, Studio validation should confirm graph visibility, build endpoint behavior, run endpoint behavior, and readable output.
- Query mode should be checked as read-only.
- Mutation should remain tied to Agent mode or explicit intent execution.
- Screenshot-ready instructions should be deterministic enough that a reviewer can capture the same states later.

### Documentation Quality

- Commands should be copy-pasteable and avoid hidden environment assumptions.
- Links should be specific enough to support Stage 7, Stage 8, Stage 11, and Stage 12 traceability.
- The main path should stay concise; historical logs and older evidence should be linked rather than pasted inline.
- The document should avoid stale statements that say #488 or #489 are out of scope once the final protocol work is complete.

## Tasks

- Review the accepted evidence and implementation state for #486 and #487.
- Wait for or verify accepted #488 Math Library evidence before finalizing the all-samples walkthrough.
- Update `docs/testing/phase15-walkthrough.md` from a two-sample walkthrough into the final Phase 15 protocol.
- Add or revise prerequisite setup instructions for CLI, Studio, provider credentials, and output directories.
- Add all-sample command sections with consistent report paths and expected task ids.
- Add sample-specific evidence expectations for Calculator, String Utilities, and Math Library.
- Add Studio validation sections for all three samples.
- Add timing targets and where to verify elapsed time.
- Add troubleshooting guidance for provider, graph, build, run, report, and Studio shared-backend failure classes.
- Add traceability links to issues, specs, PRs, evidence comments, and reports.
- Remove or update stale scope language that says #488 and #489 are excluded.
- Verify the documentation against actual accepted evidence rather than speculative desired behavior.

Independent work:

- Documentation structure and prerequisite cleanup can be prepared before #488 completes.
- Traceability link inventory can be prepared before #488 completes.
- Failure-class troubleshooting can be drafted from existing Phase 15 evidence before #488 completes.

Sequential work:

- Math Library command, expected evidence, timing, and Studio steps must wait for #488 accepted evidence.
- Final pass/fail language must wait until all three sample evidence paths are real.
- Phase 15 closure-facing language must wait until the completed walkthrough matches accepted evidence.

## Checks

Required documentation checks:

- Inspect the rendered Markdown locally or in GitHub preview for readable headings, code blocks, tables, and links.
- Verify every command block is copy-pasteable or clearly marked as an example.
- Verify every referenced issue, PR, spec, report, or evidence comment link resolves.
- Verify the document no longer incorrectly excludes #488 or #489 after this work is implemented.
- Verify no raw provider secret appears in the document.
- Verify the document states #488 as a prerequisite if #488 evidence is not yet merged when implementation starts.

Required evidence checks once #488 is complete:

- Re-run or verify Calculator evidence from the final walkthrough.
- Re-run or verify String Utilities evidence from the final walkthrough.
- Re-run or verify Math Library evidence from the final walkthrough.
- Verify Studio steps for all three samples against the generated or accepted workspaces.
- Verify timing targets are either met or explicitly explained by accepted evidence.

Recommended repository checks for a docs-only implementation:

```bash
git diff --check
```

If the implementation changes docs tooling, command wrappers, harness behavior, or source code, Stage 8 should require the corresponding repository tests in addition to the docs-only checks.

Pass criteria:

- A reviewer can run the final protocol without reading #486, #487, or #488 first.
- The final walkthrough is faithful to real accepted evidence.
- The final walkthrough identifies failure classes clearly enough to support follow-up issue creation.
- The final walkthrough preserves Query mode read-only expectations.
- The final walkthrough supports a later Phase 15 closure decision.

## Open Questions

None blocking for product specification.

Non-blocking items for Stage 7 or Stage 8:

- Should Stage 7 require one provider across all three samples, or accept provider-per-sample evidence when each provider is recorded?
- Should committed screenshots be required, or are deterministic screenshot-ready Studio steps enough?
- Should the Math Library elapsed-time target remain under 15 minutes, or should #488 evidence define a different target?
- Should the final walkthrough include a compact evidence matrix near the top, or keep evidence only within sample sections?

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/489
- Stage 5 Human Acceptance Decision: https://github.com/hgahub/duumbi/issues/489#issuecomment-4448729008
- Calculator sample: https://github.com/hgahub/duumbi/issues/486
- String Utilities sample: https://github.com/hgahub/duumbi/issues/487
- Math Library sample: https://github.com/hgahub/duumbi/issues/488
- String Utilities implementation PR: https://github.com/hgahub/duumbi/pull/546
- Phase 15 walkthrough: `docs/testing/phase15-walkthrough.md`
- Phase 15 harness: `src/cli/phase15_e2e.rs`
- DUUMBI PRD: `DUUMBI - PRD`
- DUUMBI Development Intake to Delivery Workflow: `DUUMBI - Development Intake to Delivery Workflow`
- DUUMBI Glossary: `DUUMBI - Glossary`
- DUUMBI Agentic Development Map: `DUUMBI Agentic Development Map`
- DUUMBI Service and Research Direction: `DUUMBI - Service and Research Direction`
