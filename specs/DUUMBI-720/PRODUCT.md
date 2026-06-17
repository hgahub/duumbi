# DUUMBI-720: Determinism Program For AI Development

## Summary

Related to #720.

Define the first buildable determinism-program slice for DUUMBI's AI
development loop. The slice makes determinism measurable for provider-backed
intent execution by replaying the same accepted intent or benchmark task under
locked context, recording append-only evidence, and reporting whether repeated
runs produce identical, semantically equivalent, or merely behaviorally
equivalent graph outcomes.

The product promise for this issue is:

```text
same intent + locked context + same provider route + same initial graph +
same registry state -> replay bundle with agreement metrics and divergence
evidence
```

This is a measurement and evidence foundation. It does not claim that LLM APIs
are inherently deterministic, and it does not require full rewrite-rule-based
mutation, a Studio dashboard, formal verification, or a public marketing
write-up before the first metric exists.

## Problem

DUUMBI's compiler, graph validation, semantic hashing, context assembly, BDD
artifacts, property evidence, telemetry evidence, and rewrite preview/apply
paths are increasingly deterministic. The AI layer remains probabilistic, and
the repository has no repeatable way to measure how stable intent-to-graph
outcomes are across identical attempts.

Current evidence is incomplete:

- `duumbi benchmark` can run provider-backed intent execution across core and
  scaled showcase suites, but the report is pass/fail oriented and does not
  compare generated graph outputs across attempts.
- `src/hash.rs` computes an `@id`-independent semantic hash, but benchmark and
  intent reports do not persist exact graph digests, semantic hashes, or
  equivalence classifications for replayed attempts.
- BDD artifacts and conservative BDD coverage classification exist in
  `src/intent/bdd.rs`, but replay reports do not yet use BDD or verifier
  evidence as an equivalence tier.
- Context assembly has deterministic tests, but replay evidence does not record
  prompt/context hashes, graph snapshot hashes, registry state hashes, or model
  route metadata.
- Rewrite preview/apply now has stable rule metadata, match IDs, validation
  evidence, and cost evidence, but it is not yet an end-to-end constrained LLM
  mutation path comparable to freeform provider mutations.
- Scaled smoke evidence from #689 is intentionally risk evidence: it can select
  multi-function, multi-module, and process-evidence tasks, but the latest
  committed scaled smoke report records 0/3 successes and provider usage data
  was unavailable.

Without a replay harness and evidence ledger, DUUMBI cannot honestly say whether
AI-driven graph mutation is stable, improving, or merely passing some verifier
cases by luck.

## Outcome

When this issue is implemented:

- A maintainer can run a CLI-first determinism replay for one or more existing
  benchmark or intent tasks with a configured provider route and an attempt
  count.
- Each replay run locks and records the effective provider route, resolved model
  identity when available, prompt hash, context hash, initial graph snapshot
  hash, registry/lockfile state hash, DUUMBI version, source commit, selected
  task, and attempt number.
- Each attempt writes or retains enough local evidence to compute:
  - exact graph-output agreement
  - semantic-hash agreement
  - verifier or BDD behavioral agreement
  - failure-category agreement
  - provider usage availability and unavailable reason
- The replay bundle includes an append-only event ledger plus a compact summary
  report suitable for `docs/e2e/results/` and release tracking.
- Divergences are classified instead of hidden. The report distinguishes exact
  mismatch, semantic mismatch, behavior mismatch, verifier failure, broader
  evidence required, provider failure, missing usage data, and unsupported
  comparison modes.
- Existing rewrite-rule evidence is surfaced as a comparison hook where it is
  applicable. The report may say `rewrite_strategy: not_applicable` or
  `not_yet_comparable`; it must not claim rewrite-rule improvement until the
  same task can be compared against a real constrained mutation strategy.
- The default `duumbi add`, `duumbi intent execute`, and `duumbi benchmark`
  behavior remains unchanged unless the user explicitly runs the determinism
  replay surface.
- No secrets, raw provider credentials, full prompts containing credentials, or
  personal data are written to replay evidence.
- The execution issue remains open through Stage 10 implementation and Stage 12
  closure; this spec PR is specification-only.

## Scope

### In Scope

- CLI-first replay harness for existing DUUMBI benchmark or persisted intent
  tasks.
- Attempt isolation so each replay attempt starts from the same initialized
  workspace and graph state.
- Locked-context metadata capture:
  provider route, model identity when available, prompt digest, BDD/context
  digest, initial graph digest, semantic graph hash, registry/lockfile digest,
  DUUMBI version, source commit, task id, suite, attempt, and timestamp.
- Append-only replay ledger under a deterministic local artifact directory.
- Summary report with per-task and aggregate determinism metrics.
- Equivalence ladder:
  exact graph output, semantic hash, verifier/BDD behavior, and accepted
  broader-evidence classification.
- Failure taxonomy aligned with existing benchmark categories and BDD evidence
  gaps.
- Integration with existing benchmark suites, especially the scaled smoke subset
  delivered by #689.
- Integration with existing BDD readiness and coverage evidence when replaying
  intents that have linked `.feature` files.
- Read-only comparison hooks for the existing rewrite rule catalog and
  rewrite-preview/apply evidence, without making rewrite-rule mutation a
  precondition for baseline replay metrics.
- Tests and documentation sufficient for the new CLI surface and artifact
  contract.

### Explicitly Out Of Scope

- Implementation code, tests, generated artifacts, or Ralph cycles during Stage
  6, Stage 7, Stage 8, or Stage 9.
- Changing default `duumbi add`, `duumbi intent execute`, `duumbi benchmark`,
  provider setup, Query mode, TUI startup, or Studio behavior.
- A Studio or web dashboard. A JSON/Markdown report is enough for this issue.
- Full constrained LLM mutation through typed rewrite rules.
- Proving that rewrite-rule mutation improves replay stability across the same
  corpus. This issue can prepare the comparison slot, but the claim requires a
  real comparable mutation strategy.
- Formal verification, SMT proof caching, VCGen, or certification export.
- Public marketing copy such as "How DUUMBI makes AI development
  deterministic."
- Remote telemetry ingestion, registry sync, cloud account history, or session
  sync.
- Normalizing all graph serialization across the whole repository. This issue
  may add canonicalization needed for replay evidence, but it must not rewrite
  unrelated graph files.
- Treating byte-identical output as the only success criterion.

## Constraints And Assumptions

Facts:

- Issue #720 is open, labeled `accepted` and `needs-spec`, and has a Stage 5
  Human Acceptance Decision comment dated 2026-06-17 with `Decision: Accept`,
  `Remaining open questions: none`, and `Next state: Spec Needed`.
- The issue source is the Obsidian note `2026-06-12 - Determinism Program for AI
  Development.md`.
- The active roadmap identifies determinism as an M1 goal: locked
  model/prompt/context replay, graph-diff stability metrics, and an evidence
  ledger.
- The active PRD frames DUUMBI as intent-first, queryable first,
  graph-centered, evidence-oriented, and human-verifiable.
- `src/hash.rs` provides deterministic semantic hashing for JSON-LD graph
  directories and intentionally excludes node `@id` values from semantic
  identity.
- `src/context/mod.rs` has deterministic context assembly tests.
- `src/intent/bdd.rs` supports linked BDD artifacts, BDD readiness, and
  conservative scenario coverage classification.
- `src/bench` already has provider-backed core and scaled showcase suites,
  result summaries, failure categories, first-pass fields, repair fields, and
  provider usage availability fields.
- The committed #689 scaled smoke report records 0/3 successful scaled tasks and
  usage data unavailable for all three rows.
- `src/rewrite` owns deterministic graph-to-graph rewrite preview/apply
  contracts with rule metadata, safety classes, match IDs, validation evidence,
  and cost evidence.

Assumptions:

- The first useful determinism metric is local and CLI-first because CI and
  release tracking can consume JSON/Markdown artifacts without introducing a UI
  dependency.
- Baseline replay should measure the current provider-backed mutation path
  before DUUMBI claims improvements from constrained rewrite-rule mutation.
- Semantic equivalence can initially mean equal semantic hash or equal accepted
  verifier/BDD evidence. Later formal equivalence can extend the ladder.
- Provider APIs may still vary even with stable prompts and model identifiers;
  the product should measure that variation rather than hide it.
- Missing provider token/cost usage is a reportable evidence gap, not a blocker
  for graph determinism metrics.

Constraints:

- Evidence must be deterministic enough for tests: stable schema versions,
  sorted records where practical, bounded rendered output, and no dependence on
  wall-clock order beyond recorded timestamps.
- Evidence must be safe: no raw credentials, no credential-bearing env var
  values, no full secret-bearing prompts, and no unbounded raw provider output.
- Replay attempts must not mutate the user's active workspace graph. Each
  attempt runs in an isolated temp or artifact workspace.
- Provider-backed replay must honor existing provider configuration and error
  paths; it must not add a new credential setup flow.
- The implementation must avoid broad architectural rewrites and keep Cranelift
  internals behind the compiler boundary.
- Spec-only PRs for this issue must use non-closing references and leave #720
  open for later workflow stages.

## Decisions

- **Decision:** Use a file-based product spec.
  **Evidence:** The issue is architectural, cross-module, durable, and
  implementation-facing. It spans CLI, benchmark runner, intent execution, BDD,
  hashing, artifact schemas, reports, tests, docs, and workflow evidence.

- **Decision:** The canonical first interface is CLI, not Studio.
  **Evidence:** Existing benchmark, intent, provider, and evidence workflows are
  CLI-first, and the roadmap says CLI is the automation/CI/agent surface.

- **Decision:** Baseline provider-backed replay is required before rewrite-rule
  improvement claims.
  **Evidence:** The issue goal is to make determinism measured and improvable.
  A comparison claim without a baseline would be unsupported.

- **Decision:** The first equivalence ladder is exact graph, semantic hash, and
  verifier/BDD behavior.
  **Evidence:** `src/hash.rs` already supports semantic hashing, BDD artifacts
  are delivered, and formal verification is a later roadmap track.

- **Decision:** Existing rewrite-rule evidence is included as a comparison hook,
  not as the primary implementation requirement.
  **Evidence:** `src/rewrite` has deterministic preview/apply evidence, but the
  accepted issue and related roadmap item also imply a future constrained LLM
  mutation path that does not exist yet.

- **Decision:** A local report replaces a dashboard for this issue.
  **Evidence:** The issue needs release-trackable metrics now; a UI dashboard is
  a separate product surface and would increase scope without improving the
  first measurement contract.

## Behavior

The initial product surface should be a CLI replay command or equivalent
CLI-first subcommand. Exact flag names belong in the technical spec, but the
observable workflow is:

1. The user chooses a benchmark suite/showcase or persisted intent, provider
   route, attempt count, and output path.
2. DUUMBI creates a replay run id and isolated attempt workspaces.
3. For every attempt, DUUMBI initializes or restores the same starting graph and
   captures locked-context metadata before mutation begins.
4. DUUMBI runs the current provider-backed intent execution path.
5. DUUMBI captures attempt outcome evidence, final graph digests, semantic hash,
   verifier results, BDD readiness/coverage where available, failure category,
   provider usage availability, and artifact paths.
6. DUUMBI appends events to the replay ledger and writes a summary report.
7. DUUMBI exits non-zero only for command-level failures, invalid input,
   corrupted evidence, unsafe output paths, or configured CI thresholds. A replay
   whose attempts diverge is a successful measurement unless the user requested
   CI gating.

Defaults and empty states:

- If no attempt count is provided, the command should choose a low-cost default
  suitable for local smoke evidence and display that value in the report.
- If no provider is configured, the command fails with the existing provider
  guidance rather than running a fake replay.
- If no task matches the requested suite/showcase/intent, the command fails
  with a clear input error and writes no partial summary.
- If BDD artifacts are absent, the report records BDD evidence as unavailable or
  warning rather than failing the replay.
- If process evidence is required but not automated, the report uses the
  existing broader-evidence classification.
- If provider usage data is unavailable, the report records the stable
  unavailable reason.
- If rewrite comparison is unavailable, the report records `not_applicable` or
  `not_yet_comparable` without treating it as failure.

Error and safety states:

- Unsafe output paths, path traversal, or evidence paths outside the selected
  artifact root fail before provider calls.
- A provider error is recorded for the affected attempt and included in the
  aggregate metric; it does not erase previous attempt evidence.
- Interrupted runs preserve already-written ledger entries and mark the run
  incomplete when possible.
- The command never writes raw credential values.
- The command never mutates the user's active `.duumbi/graph` directory.

Invariants:

- Same replay inputs should produce stable report ordering and schema shape.
- Attempt records are append-only within a run ledger.
- Exact graph agreement is stricter than semantic-hash agreement, which is
  stricter than verifier/BDD behavioral agreement.
- Passing verifier/BDD behavior cannot hide a semantic divergence; it can only
  classify the divergence as behaviorally equivalent.
- Release-level metrics must cite the replay bundle that produced them.

## BDD Scenarios

Feature: Determinism replay evidence for AI graph mutation

  Rule: Replays must lock and report their context

    Scenario: Run a low-cost replay for one benchmark showcase
      Given DUUMBI has at least one configured provider
      And the user selects one benchmark showcase and two attempts
      When the user runs the determinism replay command
      Then DUUMBI creates one replay bundle with two attempt records
      And each attempt records provider route, model identity when available,
      prompt hash, context hash, initial graph hash, registry state hash, DUUMBI
      version, source commit, task id, and attempt number
      And the summary report lists exact, semantic, and behavioral agreement
      metrics for the selected showcase

    Scenario: Preserve the user's active graph during replay
      Given the user runs a replay from an initialized DUUMBI workspace
      When DUUMBI executes each replay attempt
      Then every attempt uses an isolated workspace or restored graph snapshot
      And the user's active `.duumbi/graph` files are unchanged after the replay

  Rule: Divergences must be classified rather than hidden

    Scenario: Final graphs differ but semantic hashes match
      Given two replay attempts produce different serialized graph output
      And the attempts have the same semantic graph hash
      When DUUMBI writes the replay summary
      Then the exact agreement metric is below 100 percent
      And the semantic agreement metric records the attempts as equivalent
      And the report explains that the divergence is structural or identity-level

    Scenario: Semantic hashes differ but verifier or BDD evidence matches
      Given two replay attempts produce different semantic graph hashes
      And both attempts pass the same verifier tests or accepted BDD evidence
      When DUUMBI writes the replay summary
      Then the semantic agreement metric records a mismatch
      And the behavioral agreement metric records the attempts as equivalent
      And the report keeps the semantic divergence visible for later inspection

    Scenario: Provider usage is unavailable
      Given the provider response does not expose token or cost usage
      When DUUMBI records the attempt evidence
      Then the replay ledger marks provider usage as unavailable
      And the summary includes the stable unavailable reason
      And no cost claim is inferred from missing usage data

  Rule: Reports must support release tracking without overstating readiness

    Scenario: Scaled smoke replay includes broader-evidence tasks
      Given the selected scaled showcase requires process evidence beyond the
      current verifier
      When DUUMBI runs the replay
      Then the attempt is classified as broader evidence required unless process
      evidence is actually gathered
      And the summary does not count that row as a verified behavioral pass

    Scenario: Rewrite comparison is not yet comparable
      Given the replayed task uses provider-backed freeform mutation
      And no constrained rewrite-rule mutation strategy exists for the same task
      When DUUMBI writes comparison evidence
      Then the report records rewrite comparison as not yet comparable
      And the report does not claim rewrite-rule mutation improved determinism

    Scenario: CI gating is explicitly requested
      Given the user runs replay in CI mode with a required semantic agreement
      threshold
      When the replay summary falls below the threshold
      Then DUUMBI exits with a non-zero status
      And the replay bundle remains available for review

## Tasks

- Add a determinism/replay domain module with schema-versioned report and ledger
  types.
- Add a CLI replay surface that can target benchmark showcases first and, if
  practical, persisted intents second.
- Reuse benchmark showcase filtering, provider creation, temp workspace setup,
  intent save/execute, and benchmark failure taxonomy where possible.
- Add graph digest and semantic-hash capture before and after each attempt.
- Add prompt/context/BDD digest capture at the narrowest stable seam available
  without exposing raw secrets.
- Add append-only ledger writing and compact JSON summary writing.
- Add Markdown rendering or a report command suitable for committing release
  evidence under `docs/e2e/results/`.
- Add rewrite comparison metadata that can report unavailable, not applicable,
  or comparable evidence without invoking unsupported mutation modes.
- Add focused unit tests for schemas, metrics, digest ordering, path safety, and
  equivalence classification.
- Add integration tests for provider-free or mock-provider replay behavior and
  evidence output shape.
- Add one manual live E2E smoke path with a real provider and low attempt count
  after implementation.

## Checks

- Product spec BDD scenarios are mapped to technical checks in
  `specs/DUUMBI-720/TECHNICAL.md`.
- Unit tests cover:
  - exact, semantic, and behavioral agreement classification
  - provider usage unavailable handling
  - append-only ledger serialization
  - path safety for output/artifact roots
  - stable report ordering
  - rewrite comparison unavailable states
- Integration tests cover:
  - replay attempt isolation
  - summary JSON schema shape
  - benchmark showcase filtering
  - no mutation of the active workspace graph
  - non-zero CI exit behavior when a configured threshold fails
- Local checks:
  - `cargo fmt --check`
  - focused `cargo test` commands for new determinism modules and benchmark
    integration
  - `cargo clippy --all-targets -- -D warnings` before implementation PR review
- Live E2E expectation:
  - run a low-cost provider-backed replay against one smoke benchmark showcase
    with two attempts
  - retain the command, provider route, artifact path, metric summary, and any
    unavailable usage reasons
  - do not require the live replay to pass every generated task; the check is
    that measurement and evidence are correct
- Review evidence:
  - implementation PR shows a replay bundle or summarized artifact from the live
    E2E run
  - no Greptile is required for spec PRs; Greptile is reserved for the final
    implementation PR when applicable

## Open Questions

None blocking.

Accepted risks and follow-ups:

- The first local report is not a Studio dashboard.
- The first equivalence ladder does not include formal proofs.
- Rewrite-rule mutation improvement remains a future comparison claim until a
  comparable constrained mutation path exists.
- Provider usage and cost data may remain unavailable for some providers until
  provider adapters expose usage consistently.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/720
- Stage 5 acceptance:
  https://github.com/hgahub/duumbi/issues/720#issuecomment-4727795690
- Source Obsidian note:
  `Duumbi/05 Archive/Processed Inbox/2026-06-12 - Determinism Program for AI Development.md`
- Roadmap:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Future Development Roadmap Map.md`
- Agentic workflow:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Repo architecture: `docs/architecture.md`
- Repo coding conventions: `docs/coding-conventions.md`
- Semantic hashing: `src/hash.rs`
- Benchmark runner and reports: `src/bench/runner.rs`, `src/bench/report.rs`,
  `src/bench/showcases.rs`
- Intent execution and BDD artifacts: `src/intent/execute.rs`,
  `src/intent/bdd.rs`, `src/intent/spec.rs`
- Context assembly: `src/context/mod.rs`, `src/context/collector.rs`,
  `tests/integration_phase10_context.rs`
- Rewrite evidence: `src/rewrite/engine.rs`, `src/rewrite/evidence.rs`,
  `src/rewrite/rule.rs`, `tests/integration_duumbi684_rewrite.rs`
- Property evidence precedent: `src/properties/evidence.rs`
- Scaled smoke evidence:
  `docs/e2e/results/duumbi-689-scaled-smoke-20260616.md`,
  `docs/e2e/duumbi-689-known-limitations.md`
