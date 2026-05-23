# DUUMBI-610: Collect LLM Usage And Workflow Runtime Metrics

## Summary

DUUMBI should emit a conservative first metrics artifact for GitHub Actions
workflows that coordinate agent-adjacent stage handoffs and for the existing
provider-backed intent evaluation script.

The first product slice is evidence-oriented, local to GitHub run artifacts, and
metadata-only. It should help maintainers answer which workflow paths are slow,
which paths involve measurable provider usage, and where optimization work
should start before changing automation behavior.

For v1, the accepted surface is:

```text
selected workflow or eval run -> metrics JSON artifact -> GitHub summary table
```

This is a specification PR only. Related to #610; the execution issue must stay
open for Stage 7 review and later workflow stages.

## Problem

DUUMBI now uses GitHub Actions for stage approval, Slack handoffs, product spec
review requests, technical spec review requests, documentation review triggers,
CI, coverage, and releases. Some paths are deterministic workflow glue. Others
are agent-adjacent or provider-backed enough that runtime and usage evidence
matters for later workflow optimization.

The current source state provides scattered evidence but not a consistent
metrics contract:

- `.github/workflows/stage-approval.yml` records stage decisions, changes
  labels, updates Project V2 when configured, posts comments, writes workflow
  summaries, and may notify Slack.
- `.github/workflows/human-acceptance-request.yml`,
  `.github/workflows/spec-review-request.yml`, and
  `.github/workflows/technical-spec-review-request.yml` build Slack review
  notification payloads and have scheduled sweeps.
- `scripts/slack-approval-bridge/README.md` records that deterministic Slack
  button approval reduced one approval path from roughly three hours and 108
  credits to roughly 30 seconds and zero credits.
- `scripts/eval_intent.sh` runs provider-backed `duumbi intent create`
  evaluations and writes quality reports, but does not capture provider request,
  token, cost, or latency evidence.

Without a shared artifact shape, maintainers cannot reliably compare workflow
run time, handoff latency, failure rates, or available provider usage before
deciding which automation to optimize.

## Outcome

When this issue is implemented:

- Selected DUUMBI GitHub Actions workflows emit a machine-readable metrics JSON
  artifact for each run.
- The same runs provide a human-readable GitHub step summary that highlights
  workflow duration, job or phase duration, conclusion, event type, and relevant
  issue or PR correlation.
- `scripts/eval_intent.sh` includes provider/evaluation metrics in its existing
  JSON result report when values are available safely.
- Metrics distinguish deterministic workflow timing from optional provider usage
  evidence instead of implying exact LLM consumption where no trustworthy data
  exists.
- Metrics avoid secrets, raw prompts, raw completions, Slack payload bodies,
  private comments, and provider credentials.
- Missing or unavailable usage fields are explicit `null` or omitted optional
  values, not guessed estimates.
- Maintainers can inspect the latest run artifact and identify likely
  optimization candidates without setting up an external dashboard or database.
- Existing stage routing, labels, Project updates, Slack notifications, review
  prompts, and provider-backed evaluation behavior continue to work as before.

## Scope

### In Scope

- Define the v1 metrics schema for GitHub Actions workflow metrics.
- Emit metrics for these workflows:
  - `.github/workflows/stage-approval.yml`
  - `.github/workflows/human-acceptance-request.yml`
  - `.github/workflows/spec-review-request.yml`
  - `.github/workflows/technical-spec-review-request.yml`
- Include workflow-level fields:
  - schema version.
  - repository, workflow name, workflow file, run ID, run attempt, actor, event
    name, ref, SHA, conclusion, and timestamp.
  - issue number, PR number, DUUMBI stage, and Project status when available.
  - start time, end time, and duration in milliseconds when available.
- Include job or phase-level fields where practical:
  - phase name.
  - conclusion or status.
  - duration in milliseconds.
  - failure count or warning count when available without broad log scraping.
- Include optional provider usage fields only when already available from the
  step, script, or provider response:
  - provider.
  - model.
  - request count.
  - prompt/input tokens.
  - completion/output tokens.
  - total tokens.
  - estimated cost in USD.
  - provider latency in milliseconds.
  - provider failure count.
- Extend `scripts/eval_intent.sh` so its report can carry the v1 usage/timing
  fields for provider-backed evaluations when safe data is available.
- Publish metrics through GitHub Actions artifacts and GitHub step summaries.
- Use metadata-only collection. Do not capture prompts, completions, raw Slack
  payloads, secrets, credentials, issue comment bodies, or private payload
  contents.
- Add focused validation for schema generation and summary behavior where
  practical.

### Explicitly Out Of Scope

- Technical specification content or implementation instructions.
- Implementation code, generated artifacts, runtime assets, or Ralph cycles
  during Stage 6.
- External metrics databases, SaaS observability tools, dashboards, OpenTelemetry
  collectors, Prometheus, Grafana, BigQuery, or data warehouses.
- Enforcing budgets, blocking runs based on usage, or changing resource-gate
  policy.
- Rewriting workflow architecture, Slack approval bridge behavior, stage
  semantics, labels, or Project status transitions.
- Exact Codex internal reasoning usage. If mentioned later, it must remain an
  estimate or declared unavailable.
- Capturing raw LLM prompts, completions, tool inputs, provider API keys, Slack
  request bodies, GitHub tokens, or other sensitive data.
- Adding metrics to every GitHub Actions workflow in the repository in this
  first slice.
- Measuring approval turnaround across multiple independent workflow runs as a
  durable business metric. V1 may include correlation fields that make this
  possible later, but cross-run analytics belong to a later issue.
- Changing `duumbi provider` UX, model routing, provider credentials, or
  `/provider` behavior.

## Constraints And Assumptions

Facts:

- Issue #610 is open and accepted for specification.
- Issue #610 is labeled `accepted` and `needs-spec`.
- Issue #610 is in the `Spec Needed` Project status at Stage 6 intake.
- The Stage 5 decision comment on 2026-05-23 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- The source inbox note
  `Duumbi/00 Inbox (ToProcess)/2026-05-22 - GitHub Actions LLM Usage Metrics.md`
  asks for metrics in GitHub Actions about LLM call consumption and run times.
- The active PRD says DUUMBI should connect intent, executable behavior, runtime
  feedback, agent activity, and evidence.
- The active PRD also says agent runs should produce reviewable artifacts and
  evidence.
- The active runbook uses GitHub Issues and Project state as the execution
  source of truth and keeps Stage 6 limited to product specification.
- `stage-approval.yml` currently handles Stage 5, Stage 7, and Stage 9 approval
  routing.
- The review request workflows notify Slack for human acceptance, product spec
  review, and technical spec review states.
- `scripts/eval_intent.sh` already writes JSON evaluation reports under
  `docs/e2e/results/`.

Assumptions:

- The first useful workflow set is approval/review handoff workflows plus the
  provider-backed evaluation script. CI, coverage, docs review, and release
  workflows can be added later if the artifact shape proves useful.
- GitHub Actions artifacts and step summaries are sufficient for v1 because the
  goal is inspectable evidence, not long-term analytics.
- Provider token and cost fields will not be consistently available across
  providers. The schema must support optional usage fields without forcing
  provider-specific complexity into all workflows.
- Wall-clock duration is useful even when provider usage is unavailable, as long
  as the summary clearly separates deterministic run time from LLM/provider
  usage.

Constraints:

- Metrics must not change the outcome of existing workflows.
- Metrics collection failures must not hide the original workflow result.
- Secret-safe behavior is mandatory; v1 must collect metadata only.
- Missing optional usage data must be visibly unavailable rather than inferred.
- The artifact schema must be stable enough for review and future extension.
- GitHub-only access must be enough to inspect metrics for v1.
- This spec PR must not mark #610 complete; it is a Stage 6 review artifact
  only.

## Decisions

- **Decision:** Use a file-based product spec for #610.
  **Evidence:** The work spans GitHub Actions workflows, Slack handoff paths,
  provider-backed evaluation behavior, artifact contracts, and evidence policy.
  It is durable enough to require source-controlled review history.

- **Decision:** Start with approval/review handoff workflows and
  `scripts/eval_intent.sh`.
  **Evidence:** The issue body explicitly points to stage approval, review
  handoff workflows, Slack approval bridge context, and provider-backed
  evaluation behavior. This is a narrower and safer first slice than adding
  metrics to every repository workflow.

- **Decision:** Use GitHub Actions artifacts and step summaries for v1.
  **Evidence:** The issue asks for a conservative first metrics slice and
  explicitly defers external dashboards or persistent stores until the artifact
  shape is proven.

- **Decision:** Treat provider usage as optional structured data.
  **Evidence:** Provider APIs expose usage differently, and current DUUMBI
  workflow code does not expose a uniform token or cost interface.

- **Decision:** Keep metrics metadata-only.
  **Evidence:** The triage risk says broad logs and artifacts can expose
  sensitive payloads. DUUMBI's workflow evidence should be useful without
  storing secrets, prompts, completions, or private payload bodies.

- **Decision:** This spec PR must not close the execution issue.
  **Evidence:** Stage 6 creates a reviewable product spec. Stage 7, Stage 8,
  Stage 9, Stage 10, Stage 11, and Stage 12 still need to happen.

## Behavior

Default behavior:

- Existing workflows continue to perform their current primary actions.
- Metrics are emitted as additional evidence, not as a new gate.
- A workflow run with no provider-backed step still emits timing and correlation
  metrics.
- A workflow run with provider-backed usage data emits usage fields only when
  the step or script can provide them safely.

Metrics artifact behavior:

- Each selected workflow run produces one JSON metrics artifact.
- The artifact uses a stable schema version field.
- The artifact includes enough correlation for later investigation:
  repository, workflow name, run ID, run attempt, event name, actor, ref, SHA,
  issue number, PR number, DUUMBI stage, and Project status when those values
  exist.
- Durations are numeric milliseconds.
- Timestamps use ISO 8601 UTC strings.
- Optional provider fields are absent or `null` when unavailable.
- The artifact must not include raw prompts, completions, Slack request bodies,
  GitHub token values, provider credentials, or full issue/comment payloads.

Summary behavior:

- Each selected workflow run writes a GitHub step summary with a concise table
  or equivalent readable section.
- The summary identifies the metrics artifact name or path.
- The summary highlights workflow duration, result, issue or PR correlation,
  and provider usage availability.
- Missing provider usage is labeled as unavailable rather than zero unless the
  run can prove zero provider requests occurred.
- The summary is useful when viewed alone but does not duplicate sensitive data.

Failure and cancellation behavior:

- Metrics should still be emitted on failure or cancellation when GitHub Actions
  permits cleanup or post-run steps to execute.
- Metrics collection failure should be reported as a warning and should not
  convert an otherwise successful workflow into a failed workflow.
- If the original workflow fails, the metrics artifact should preserve that
  original conclusion when available.

Evaluation script behavior:

- `scripts/eval_intent.sh` keeps its existing quality report behavior.
- Evaluation reports may include provider name, timestamp, per-task duration,
  total duration, request count, token counts, estimated cost, and provider
  failure count when those values are available safely.
- Evaluation reports must not store raw prompts, completions, provider API keys,
  or generated private payloads.
- If usage cannot be measured for a provider, the report should show that usage
  is unavailable rather than inventing values.

Security and privacy behavior:

- Metrics must be safe for normal GitHub Actions artifact retention.
- The schema should prefer identifiers and counts over payload bodies.
- Error messages included in metrics must be bounded and scrubbed if there is a
  credible chance they could contain secrets or private payload contents.

## BDD Scenarios

Feature: DUUMBI workflow metrics

Rule: Selected workflows emit metadata-only timing evidence

Scenario: Stage approval emits workflow metrics after an approval decision
Given a Stage Approval workflow run processes a Stage 5, Stage 7, or Stage 9 decision
When the workflow reaches its summary phase
Then the run publishes a metrics JSON artifact
And the artifact includes workflow name, run ID, event name, issue number, stage, conclusion, and duration
And the artifact does not include Slack tokens, GitHub tokens, provider credentials, raw prompts, or raw completions
And the GitHub step summary links or names the metrics artifact

Scenario: Human acceptance request emits metrics for a scheduled sweep
Given the human acceptance request workflow runs from its scheduled trigger
When it finishes checking eligible issues
Then the metrics artifact includes the workflow name, run ID, event name, conclusion, checked issue count when available, and duration
And the summary shows whether Slack notification work happened
But the artifact does not include raw Slack message payload bodies

Scenario: Product spec review request emits metrics with artifact correlation
Given an issue has the `spec-review` label
When the product spec review request workflow builds its notification
Then the metrics artifact records the issue number, workflow name, run ID, conclusion, and duration
And the summary shows the product spec artifact was found or unavailable
And missing artifact data is visible instead of silently omitted

Scenario: Technical spec review request emits metrics with stage correlation
Given an issue is ready for technical spec review notification
When the technical spec review request workflow completes
Then the metrics artifact records the issue number, workflow name, run ID, conclusion, and duration
And the summary identifies the technical spec review path
And no technical spec content or issue comment body is copied into the metrics artifact

Rule: Provider usage is optional and explicit

Scenario: Evaluation report includes provider usage when available
Given `scripts/eval_intent.sh` runs against a configured provider that exposes usage data
When the evaluation report is written
Then the report includes provider, model when known, request count, token counts when known, estimated cost when known, and latency or duration
And the existing per-task scoring data remains present

Scenario: Evaluation report marks unavailable provider usage
Given `scripts/eval_intent.sh` runs against a provider or command path that does not expose token or cost data
When the evaluation report is written
Then usage fields are absent or explicitly unavailable
And the report does not guess token counts or cost
And the quality scoring report remains usable

Rule: Metrics do not change workflow outcomes

Scenario: Metrics collection warning does not mask a successful workflow
Given a selected workflow completes its primary action successfully
And metrics collection cannot upload an artifact because of an artifact service error
When the workflow exits
Then the primary workflow result remains successful
And the summary or logs report the metrics upload warning

Scenario: Failed workflow preserves original failure evidence
Given a selected workflow fails during its primary action
When the metrics cleanup or summary step can still run
Then the metrics artifact records the failed conclusion
And the workflow still exits according to the original failure
And the metrics step does not replace the original failure with a different error

Rule: Metrics remain secret-safe

Scenario: Sensitive payload data is excluded from artifacts
Given a workflow has access to Slack, GitHub, or provider secrets
When the metrics artifact is generated
Then the artifact contains only metadata, identifiers, counts, durations, and sanitized bounded errors
And it excludes secret values, request bodies, raw prompts, raw completions, and full private payloads

## Tasks

- Define the v1 metrics schema and artifact naming contract.
- Add workflow metrics emission to `stage-approval.yml`.
- Add workflow metrics emission to `human-acceptance-request.yml`.
- Add workflow metrics emission to `spec-review-request.yml`.
- Add workflow metrics emission to `technical-spec-review-request.yml`.
- Extend `scripts/eval_intent.sh` reports with timing and optional provider
  usage fields.
- Add schema validation or focused tests for representative metrics payloads.
- Add static or dry-run evidence that summaries and artifacts remain
  metadata-only.
- Document how maintainers should inspect metrics artifacts for v1, if the
  implementation introduces a new helper script or artifact convention that is
  not self-evident from the workflow summary.

Independent work:

- The schema and test fixture can be drafted before individual workflow changes.
- The three review request workflows can be updated in parallel once the schema
  exists because their notification behavior is similar.
- `scripts/eval_intent.sh` can be updated independently as long as it writes the
  same schema fields for shared usage/timing concepts.

## Checks

- Product spec review verifies the v1 workflow set, metrics fields, privacy
  boundary, and out-of-scope items.
- Technical spec review maps each BDD scenario to concrete workflow/script
  checks.
- Workflow YAML parses after implementation.
- Any helper scripts or schema fixtures pass local validation.
- Artifact JSON validates against the accepted v1 schema or equivalent fixture
  checks.
- Static checks confirm no obvious secret names, raw prompt fields, raw
  completion fields, Slack body fields, or token values are emitted into metrics
  artifacts.
- Local dry-run or simulation evidence covers:
  - successful Stage Approval metrics payload.
  - notification workflow metrics payload.
  - missing optional provider usage.
  - provider usage present when simulated.
  - failed or cancelled workflow conclusion when practical.
- `scripts/eval_intent.sh` retains existing scoring report fields.
- GitHub Actions step summaries include the metrics artifact name or path.
- Review evidence states whether live GitHub Actions smoke testing was run. If
  not run, the reason must be explicit because live runs mutate real workflow
  state or require a safe test issue.

## Open Questions

- None blocking for product specification.
- Later design decision: whether metrics should eventually be exported to a
  durable store after the artifact shape proves useful.
- Later design decision: whether approval turnaround should become a
  cross-workflow metric once correlation data exists.
- Later design decision: whether budget gates should consume these metrics or a
  separate provider accounting surface.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/610
- Stage 5 decision comment:
  https://github.com/hgahub/duumbi/issues/610#issuecomment-4524947132
- Source inbox note:
  `Duumbi/00 Inbox (ToProcess)/2026-05-22 - GitHub Actions LLM Usage Metrics.md`
- Active PRD:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active runbook:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Agentic development map:
  `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Service and research direction:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- Glossary:
  `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Stage approval workflow: `.github/workflows/stage-approval.yml`
- Human acceptance request workflow:
  `.github/workflows/human-acceptance-request.yml`
- Product spec review request workflow:
  `.github/workflows/spec-review-request.yml`
- Technical spec review request workflow:
  `.github/workflows/technical-spec-review-request.yml`
- Slack approval bridge docs: `scripts/slack-approval-bridge/README.md`
- Provider-backed evaluation script: `scripts/eval_intent.sh`
- Related precedent product spec: `specs/DUUMBI-593/PRODUCT.md`
