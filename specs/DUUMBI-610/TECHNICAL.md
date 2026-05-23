# DUUMBI-610: Collect LLM Usage And Workflow Runtime Metrics - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-610/PRODUCT.md` by adding
the first conservative metrics slice for selected DUUMBI GitHub Actions
workflows and the provider-backed intent evaluation script.

The required product flow is:

```text
selected workflow or eval run -> metadata-only metrics JSON -> GitHub summary
```

This technical spec covers only v1 workflow and evaluation metrics. It does not
add dashboards, persistent metrics storage, budget enforcement, broad workflow
rewrites, or implementation beyond the approved scope.

Related to #610. This technical specification is a review artifact only; the
execution issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Oz implementation agents when work is routed from Slack or GitHub.
- Specialized workflow/CI agents changing GitHub Actions YAML and
  `actions/github-script` blocks.
- Tester/reviewer agents validating metadata artifacts, workflow summaries, and
  secret-safety boundaries.
- Stage 9 technical spec reviewers checking implementability and resource
  policy.

## Source Context

- Product spec: `specs/DUUMBI-610/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/612
- GitHub issue: https://github.com/hgahub/duumbi/issues/610
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/610#issuecomment-4525008230
- Stage 6 product spec draft comment:
  https://github.com/hgahub/duumbi/issues/610#issuecomment-4524960024
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/610#issuecomment-4524947132
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant code and workflow files verified for Stage 8:

- `.github/workflows/stage-approval.yml`
  - Handles Stage 5, Stage 7, and Stage 9 approval decisions from
    `workflow_dispatch` and `repository_dispatch`.
  - Posts decision comments, updates labels, updates Project V2 through
    `GH_PROJECT_PAT` when configured, posts Slack follow-up when configured, and
    writes a GitHub workflow summary.
  - Already builds next-stage Codex prompts for approval decisions.
  - Currently has no metrics artifact or artifact upload step.
- `.github/workflows/human-acceptance-request.yml`
  - Triggers on `needs-human-review` labeling, schedule, and manual dispatch.
  - The `notify` job validates requests, finds eligible issues, builds Slack
    messages, and posts Slack notifications.
  - The `mark-notified` job posts marker comments for notified issues.
  - Current outputs include `issue_numbers` and `should_notify`; no metrics
    output exists.
- `.github/workflows/spec-review-request.yml`
  - Triggers on `spec-review` labeling, schedule, and manual dispatch.
  - Finds the Stage 6 product spec artifact from issue comments and builds
    Slack review notifications.
  - Current outputs include `issue_numbers` and `should_notify`; no metrics
    output exists.
- `.github/workflows/technical-spec-review-request.yml`
  - Triggers on `technical-spec-review` labeling, schedule, and manual dispatch.
  - Finds Stage 8 technical spec artifacts from issue comments and builds Slack
    technical review notifications.
  - Current outputs include `issue_numbers` and `should_notify`; no metrics
    output exists.
- `.github/workflows/release.yml`
  - Uses `actions/upload-artifact@v7`; this establishes an existing artifact
    upload action version in the repository.
- `.github/workflows/coverage.yml`
  - Uses `actions/upload-artifact@v7`; this is another local precedent for the
    same upload action.
- `scripts/eval_intent.sh`
  - Runs `duumbi intent create` over `docs/e2e/corpus/*.txt`.
  - Supports `--provider` and `--filter`.
  - Writes JSON reports under `docs/e2e/results/`.
  - Tracks quality totals, pass/fail/error counts, and per-task score details.
  - Does not currently record per-task duration, total duration, provider usage
    availability, token counts, cost, or provider failure counts.
- `docs/e2e/scoring.md`
  - Defines the existing intent evaluation scoring dimensions and report
    meaning.
- `scripts/slack-approval-bridge/README.md`
  - Records the move from an LLM-based Oz approval path to deterministic Slack
    buttons and GitHub Actions.

Relevant Obsidian notes checked:

- `Duumbi/00 Inbox (ToProcess)/2026-05-22 - GitHub Actions LLM Usage Metrics.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- `Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Service and Research Direction.md`
- `Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`

Verified source facts:

- The approved product spec scopes v1 to Stage Approval, the three review
  request workflows, and `scripts/eval_intent.sh`.
- The selected workflows currently use inline `actions/github-script` logic and
  do not check out the repository.
- The selected review request workflows are multi-job workflows with `notify`
  and `mark-notified` jobs.
- `stage-approval.yml` is a single-job workflow.
- Existing selected workflows already sanitize Slack-bound text, but the Slack
  message JSON itself is sensitive enough that metrics must not store it.
- `scripts/eval_intent.sh` currently writes report JSON with provider,
  timestamp, totals, pass/fail/error counts, and result rows.
- Current `duumbi intent create` invocation in `scripts/eval_intent.sh` does not
  expose a verified token/cost usage contract in this inspected context.

Assumptions for implementation:

- The first implementation should avoid adding repository checkout to the
  selected workflows unless Stage 10 proves a local helper action is materially
  safer. Inline workflow metrics scripts are acceptable for v1 because the
  selected workflows already use inline `actions/github-script`.
- GitHub Actions job metadata from the Actions API is the least invasive source
  for workflow/job timing in v1.
- `GITHUB_TOKEN` with `actions: read` permission is sufficient for selected
  workflows to query their own workflow-run jobs.
- Metrics artifact upload failure is less important than preserving the primary
  workflow result.
- Provider request/token/cost fields for `scripts/eval_intent.sh` should remain
  unavailable until a verified DUUMBI/provider output exposes them safely.

## Affected Areas

Expected implementation changes:

- Workflow metrics:
  - `.github/workflows/stage-approval.yml`
  - `.github/workflows/human-acceptance-request.yml`
  - `.github/workflows/spec-review-request.yml`
  - `.github/workflows/technical-spec-review-request.yml`
- Workflow permissions:
  - Add `actions: read` where needed so metrics scripts can inspect run jobs.
  - Preserve existing `contents`, `issues`, and `pull-requests` permissions.
- Workflow outputs:
  - Add safe numeric/string outputs only when needed for metrics correlation,
    such as queued issue count, requested notification count, artifact found
    count, or artifact missing count.
  - Do not output Slack message bodies, full issue bodies, full comments, raw
    provider output, or secrets.
- Workflow artifacts:
  - Add metrics JSON generation to a final metrics step or final metrics job.
  - Upload one metrics artifact per selected workflow run with
    `actions/upload-artifact@v7`.
- Evaluation script:
  - `scripts/eval_intent.sh`
  - Preserve existing report fields and append metrics fields.
  - Do not commit generated files under `docs/e2e/results/`.
- Optional docs:
  - `docs/e2e/scoring.md` only if implementation changes the report contract in
    a way maintainers need to read.
  - A short workflow-metrics note only if artifact names or summary fields are
    not self-explanatory from the GitHub summary.
- Verification helpers:
  - Small local Node or shell simulation snippets are allowed during
    implementation evidence.
  - If a reusable helper file is added, keep it under a workflow-specific
    scripts path such as `scripts/github-actions/` and add focused validation.

Areas expected not to change:

- `specs/DUUMBI-610/PRODUCT.md`
- Rust compiler, parser, graph, registry, runtime, MCP, TUI, Studio, and provider
  configuration code.
- `scripts/slack-approval-bridge/` runtime behavior.
- Existing issue labels, Project status names, approval semantics, Slack button
  payload semantics, and next-stage prompt semantics.
- Generated evaluation reports already committed under `docs/e2e/results/`.
- External dashboards, metrics databases, or remote observability systems.

## Technical Approach

### Metrics schema

Implement a single v1 schema shape for workflow and eval metrics. The JSON
should be intentionally flat enough to review while still preserving typed
subsections.

Recommended top-level shape:

```json
{
  "schema_version": "duumbi.workflow_metrics.v1",
  "generated_at": "2026-05-23T00:00:00.000Z",
  "source": "github_actions",
  "repository": "hgahub/duumbi",
  "workflow": {
    "name": "Stage Approval",
    "file": ".github/workflows/stage-approval.yml",
    "run_id": 123,
    "run_attempt": 1,
    "event_name": "workflow_dispatch",
    "actor": "hgahub",
    "ref": "refs/heads/main",
    "sha": "abc123",
    "conclusion": "success",
    "started_at": "2026-05-23T00:00:00.000Z",
    "completed_at": "2026-05-23T00:00:30.000Z",
    "duration_ms": 30000
  },
  "correlation": {
    "issue_number": 610,
    "pr_number": null,
    "stage": "7",
    "decision": "approve",
    "project_status": null
  },
  "phases": [
    {
      "name": "execute",
      "conclusion": "success",
      "started_at": "2026-05-23T00:00:00.000Z",
      "completed_at": "2026-05-23T00:00:29.000Z",
      "duration_ms": 29000,
      "warning_count": null,
      "failure_count": 0
    }
  ],
  "counts": {
    "issues_considered": null,
    "issues_queued": null,
    "slack_notifications_attempted": null,
    "artifact_links_found": null,
    "artifact_links_missing": null
  },
  "provider_usage": {
    "available": false,
    "reason": "no_provider_step",
    "provider": null,
    "model": null,
    "request_count": null,
    "prompt_tokens": null,
    "completion_tokens": null,
    "total_tokens": null,
    "estimated_cost_usd": null,
    "latency_ms": null,
    "failure_count": null
  },
  "privacy": {
    "metadata_only": true,
    "raw_prompts_included": false,
    "raw_completions_included": false,
    "raw_slack_payloads_included": false,
    "secrets_included": false
  }
}
```

Schema rules:

- `schema_version` must be present and equal to `duumbi.workflow_metrics.v1` for
  selected GitHub Actions workflows.
- Eval reports may reuse the same nested `provider_usage` and duration fields
  while keeping the existing report's top-level scoring structure.
- Timestamps must be ISO 8601 UTC.
- Durations must be integers in milliseconds.
- Optional unavailable values must be `null` or explicitly marked unavailable.
- Provider usage `available: false` must not be interpreted as zero usage.
- Sensitive payload bodies must not appear anywhere in the artifact.
- Bounded sanitized error summaries are allowed only when they cannot carry
  secrets or raw payloads.

### Selected GitHub Actions workflows

For each selected workflow:

1. Add or preserve a final metrics path that runs with `if: always()`.
2. Query the current run's jobs through the GitHub Actions API.
3. Compute phase rows from job names, conclusions, `started_at`, and
   `completed_at`.
4. Compute workflow duration from the earliest known job start to the latest
   completed job time or current time when the metrics job is still running.
5. Add workflow-specific correlation from `github.event.inputs`,
   `github.event.client_payload`, and safe job outputs.
6. Write `duumbi-workflow-metrics.json`.
7. Write a concise GitHub step summary that names the artifact and reports
   duration, conclusion, issue/PR/stage correlation, queued issue count, and
   provider usage availability.
8. Upload the artifact with `actions/upload-artifact@v7`.
9. Ensure metrics generation and upload warnings do not mask the primary
   workflow result.

Recommended artifact name:

```text
duumbi-workflow-metrics-${{ github.workflow }}-${{ github.run_id }}-${{ github.run_attempt }}
```

If the workflow name creates awkward artifact names, normalize to lower-case
ASCII with spaces replaced by hyphens inside the script.

Implementation boundary:

- Do not add a local composite action unless the implementation also accepts the
  checkout cost and proves the checkout does not disrupt no-checkout workflows.
- Do not parse raw logs broadly for v1. Use structured job metadata and explicit
  safe outputs.
- Do not store Slack message JSON, issue bodies, comment bodies, provider
  outputs, or secret-like environment values.

### `stage-approval.yml`

Use a final metrics step in the existing `execute` job, or a final `metrics` job
if that proves clearer. The implementation must preserve the current Stage
Approval decision result.

Recommended correlation fields:

- `issue_number`
- `stage`
- `decision`
- `pr_number`
- `event_name`
- `repository_dispatch` versus `workflow_dispatch`
- product/technical spec artifact availability for Stage 7 or Stage 9 when the
  existing script has already derived it safely

Do not change:

- decision matrix semantics.
- label add/remove behavior.
- Project V2 status behavior.
- Slack result notification behavior.
- next-stage Codex prompt content except if a metrics summary link is added as a
  separate, non-disruptive line.

### Review request workflows

For `human-acceptance-request.yml`, `spec-review-request.yml`, and
`technical-spec-review-request.yml`, prefer a final `metrics` job with:

```yaml
needs: [notify, mark-notified]
if: always()
```

The metrics job should inspect `needs.notify.result`,
`needs.mark-notified.result`, and safe outputs from the `notify` job. It should
not depend on Slack success to emit metrics when GitHub permits the metrics job
to run.

Safe output additions:

- `issue_numbers`: already present.
- `should_notify`: already present.
- `issues_queued_count`: numeric count derived from `issuesToNotify.length`.
- `artifact_links_found`: only for spec review and technical spec review
  workflows, numeric count.
- `artifact_links_missing`: only for spec review and technical spec review
  workflows, numeric count.

Avoid:

- `slack_messages` in metrics artifacts.
- full artifact URLs if reviewers decide URLs are too payload-like; PR or issue
  numbers are enough for metrics. A yes/no or count is sufficient for v1.
- comments, issue bodies, Slack block JSON, and raw API response bodies.

### `scripts/eval_intent.sh`

Preserve the existing report contract and append metrics without changing the
meaning of scoring fields.

Recommended additions to each per-task row:

```json
{
  "task": "E01_calculator",
  "status": "pass",
  "score": 80,
  "duration_ms": 12000,
  "duumbi_command_attempted": true,
  "provider_usage": {
    "available": false,
    "reason": "duumbi_intent_create_usage_not_exposed"
  }
}
```

Recommended additions to the top-level report:

```json
{
  "provider": "minimax",
  "timestamp": "20260523_120000",
  "duration_ms": 120000,
  "usage_summary": {
    "available": false,
    "reason": "duumbi_intent_create_usage_not_exposed",
    "provider": "minimax",
    "model": null,
    "request_count": null,
    "prompt_tokens": null,
    "completion_tokens": null,
    "total_tokens": null,
    "estimated_cost_usd": null,
    "provider_failure_count": null
  }
}
```

Timing approach:

- Use a small shell helper for epoch milliseconds. If millisecond precision is
  not portable in local shells, seconds multiplied by 1000 is acceptable for
  v1, but the field must still be named `duration_ms`.
- Track total script duration and per-task duration around the `duumbi intent
  create` command plus scoring work.

Usage approach:

- Do not infer provider request counts from task count in the
  `provider_usage.request_count` field.
- If implementation wants to expose command attempts, use a separate field such
  as `duumbi_command_attempts`.
- Do not parse raw prompts or completions.
- If a later verified DUUMBI provider output exposes usage safely, map it into
  `provider_usage` and add focused tests in the same implementation PR.

### Rejected alternatives

- External metrics database or dashboard: rejected for v1 because the product
  spec requires GitHub artifacts and summaries first.
- Broad log scraping: rejected because it is fragile and increases privacy risk.
- Capturing raw Slack or provider payloads: rejected because v1 is metadata-only
  and secret-safe.
- Budget gates: rejected for this issue because enforcement belongs to a later
  decision after metrics shape and trust are proven.
- Exact Codex internal reasoning usage: rejected because the active runbook
  treats it as estimate-only.

## Invariants

- The execution issue stays open after this technical spec PR is merged or
  closed.
- Product spec content is not modified by Stage 10 implementation.
- Existing workflow primary behavior remains unchanged.
- Metrics collection cannot turn an otherwise successful primary workflow into
  a failed workflow.
- Metrics collection cannot hide the original workflow failure.
- Metrics artifacts are metadata-only.
- No secrets, provider credentials, Slack request bodies, Slack message JSON,
  raw prompts, raw completions, or full issue/comment bodies are written to
  metrics artifacts.
- Missing usage data is reported as unavailable, not guessed.
- GitHub Actions summaries name or link the metrics artifact.
- `scripts/eval_intent.sh` retains existing quality report fields and scoring
  behavior.
- Generated evaluation reports under `docs/e2e/results/` are not committed as
  part of implementation.
- No new external service dependency is introduced.

## BDD-To-Test Mapping

| Product BDD scenario | Evidence type | Required implementation evidence |
| --- | --- | --- |
| Stage approval emits workflow metrics after an approval decision | Workflow YAML validation, local `github-script` simulation, optional live workflow smoke | Parse `.github/workflows/stage-approval.yml`; simulate `workflow_dispatch` and `repository_dispatch` contexts; assert artifact JSON includes workflow name, run ID, event name, issue number, stage, conclusion, duration, privacy flags, and no banned payload fields. |
| Human acceptance request emits metrics for a scheduled sweep | Workflow YAML validation and local simulation | Parse `.github/workflows/human-acceptance-request.yml`; simulate scheduled `notify` outputs with zero and nonzero queued issues; assert metrics include workflow name, run ID, event name, conclusion, queued count, duration, and no Slack message body. |
| Product spec review request emits metrics with artifact correlation | Workflow YAML validation and local simulation | Parse `.github/workflows/spec-review-request.yml`; simulate found and missing Stage 6 artifact states; assert `artifact_links_found` or `artifact_links_missing` counts and summary text show availability without copying issue comment bodies. |
| Technical spec review request emits metrics with stage correlation | Workflow YAML validation and local simulation | Parse `.github/workflows/technical-spec-review-request.yml`; simulate found and missing Stage 8 artifact states; assert issue number, workflow name, run ID, conclusion, duration, and technical review path are represented without technical spec content. |
| Evaluation report includes provider usage when available | Script-level unit/simulation or controlled fixture | If implementation adds a verified usage input path, run a local simulation with synthetic safe usage data and assert provider, model, request count, tokens, cost, and latency map into the report while existing scoring fields remain. |
| Evaluation report marks unavailable provider usage | Local script run or shell simulation | Run `scripts/eval_intent.sh` in a no-provider or mocked path when practical, or simulate report assembly; assert `provider_usage.available` is false or unavailable, token/cost fields are not guessed, and scoring fields remain. |
| Metrics collection warning does not mask a successful workflow | Workflow script simulation and review evidence | Simulate artifact upload or metrics generation failure with `continue-on-error` or warning behavior; assert primary job results are preserved and warning text is visible. |
| Failed workflow preserves original failure evidence | Workflow script simulation and optional safe live workflow failure | Simulate a failed `notify` or `execute` job with final metrics job running under `if: always()`; assert metrics conclusion records failure and does not replace the original failure cause. |
| Sensitive payload data is excluded from artifacts | Static scan and JSON fixture inspection | Inspect generated JSON fixtures and workflow scripts for raw prompt/completion fields, Slack body fields, token values, `SLACK_BOT_TOKEN`, `GITHUB_TOKEN`, `GH_PROJECT_PAT`, and broad payload dumps; record static-scan evidence. |

Automation expectations:

- Use `ruby -e "require 'yaml'; YAML.load_file('<workflow>')"` or an
  equivalent parser for workflow syntax where `actionlint` is unavailable.
- Run `actionlint` on touched workflows when available.
- Use `jq` to validate generated metrics JSON fixtures.
- Use static `rg` checks for prohibited field names and secret-like values in
  generated metrics artifacts and changed workflow scripts.
- Live GitHub Actions smoke tests are optional and require a safe test issue or
  explicit human approval because selected workflows can mutate real issues,
  comments, labels, Slack notifications, and Project state.

## Live E2E Plan

Canonical interface: CLI, because the approved product spec touches the
provider-backed intent evaluation script rather than TUI or Studio behavior.

Live provider path:

- Command:
  `./scripts/eval_intent.sh --provider <configured-provider> --filter E`
- Provider setup:
  Use an existing configured DUUMBI provider through `.duumbi/config.toml` or
  existing provider environment variables. Do not add new provider UX.
- Required tools:
  `duumbi` binary or successful local `cargo build`, `jq`, and `yq`.
- Expected external LLM calls:
  Up to 10 DUUMBI provider-backed intent creation calls for the Easy corpus
  filter, based on the current corpus naming pattern and script loop.
- Estimated external LLM cost:
  Must be estimated by the implementation agent before the live run. The run may
  proceed autonomously only if the estimate is at or below USD 2 and at or below
  10 external calls.
- Artifacts:
  A JSON report under `docs/e2e/results/`. The report must not be committed
  unless a later human explicitly asks for generated result artifacts.
- Pass criteria:
  The report keeps existing scoring fields, includes total and per-task
  duration fields, marks provider usage unavailable unless verified usage is
  exposed, and stores no raw prompts, completions, or credentials.
- Fail criteria:
  The command cannot run because provider credentials are missing, external call
  estimate exceeds the resource gate, generated reports contain sensitive
  payloads, or scoring fields regress.

Workflow live smoke path:

- Preferred first implementation evidence is local simulation plus YAML parsing.
- A live workflow smoke may be run only against a safe test issue or with
  explicit human approval because Stage Approval and review request workflows can
  mutate issues, comments, labels, Slack messages, and Project state.
- If run, use a workflow path that minimizes mutation risk and records the exact
  issue, workflow run URL, artifact name, and cleanup expectation.

TUI and Studio:

- No TUI or Studio E2E is required for #610 because the approved behavior is
  GitHub Actions plus CLI evaluation script behavior.

## Ralph Cycle Protocol

Each Stage 10 Ralph cycle must:

1. summarize current issue state, branch, PR state, and unmet product/technical
   requirements.
2. propose one bounded implementation goal.
3. list intended files and commands before editing.
4. estimate external LLM calls, external LLM cost, command cost, and risk.
5. check whether the resource gate requires human approval.
6. implement only the approved or resource-permitted goal.
7. run the planned checks.
8. report evidence, failures, changed files, generated artifacts, and remaining
   gaps.
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, a product/architecture decision is needed, or the
   autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: up to three related workflow/script files, or
  one workflow plus one verification helper, unless a narrower cycle is more
  appropriate.
- Expected command budget per cycle:
  - `git diff --check`
  - YAML parse checks for touched workflows
  - focused Node/shell simulation for metrics payload generation
  - `jq` checks for generated metrics fixtures when used
  - `actionlint` for touched workflows when available
  - one focused `scripts/eval_intent.sh` smoke or simulation only when the
    cycle touches the eval script
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies or irreversible
  operations, mutates a real GitHub issue for live workflow smoke, posts to
  Slack intentionally, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: three low-budget cycles after Stage 9 approval.
- When to stop and ask for human guidance:
  - provider usage cannot be represented without guessing.
  - workflow metrics require a checkout-based local action and the added
    checkout changes workflow risk materially.
  - a live workflow smoke would mutate real issue state or Slack without
    explicit approval.
  - artifact upload failures cannot be made non-masking.
  - implementation would require changing product spec scope, provider UX,
    stage semantics, or external storage.

## Task Breakdown

1. Define the shared v1 metrics payload contract in implementation code or
   workflow-local helper functions.
2. Add metrics generation, summary output, and artifact upload to
   `.github/workflows/stage-approval.yml`.
3. Add safe count outputs and final metrics job to
   `.github/workflows/human-acceptance-request.yml`.
4. Add safe artifact availability counts and final metrics job to
   `.github/workflows/spec-review-request.yml`.
5. Add safe artifact availability counts and final metrics job to
   `.github/workflows/technical-spec-review-request.yml`.
6. Extend `scripts/eval_intent.sh` report assembly with total duration,
   per-task duration, and explicit provider usage availability.
7. Add or run focused local simulations for representative workflow metrics
   payloads.
8. Run YAML parsing, optional `actionlint`, JSON validation, static
   secret-safety scans, and eval-report checks.
9. Update minimal docs only if the artifact naming or report fields are not
   clear from summaries and existing docs.

Suggested cycle slicing:

- Cycle 1: shared schema decision plus `stage-approval.yml` metrics path and
  focused simulation.
- Cycle 2: review request workflow metrics paths and safe outputs.
- Cycle 3: `scripts/eval_intent.sh` report metrics plus final validation sweep.

## Verification Plan

Required static checks:

- `git diff --check`
- `git diff --cached --check` before commit
- YAML parse each changed workflow with Ruby, Python, or another local YAML
  parser.
- `actionlint` for changed workflows if available; if unavailable, record that
  explicitly.
- `rg` static scan over changed files and generated metrics fixtures for:
  - `raw_prompt`
  - `raw_completion`
  - `completion_text`
  - `prompt_text`
  - `SLACK_BOT_TOKEN`
  - `GITHUB_TOKEN`
  - `GH_PROJECT_PAT`
  - `SLACK_MESSAGES` in metrics artifact construction
  - broad `payload` dumps into metrics output

Required workflow metrics checks:

- Local simulation for Stage Approval metrics with:
  - workflow dispatch.
  - repository dispatch.
  - Stage 7 or Stage 9 artifact correlation.
  - successful and failed primary conclusion.
- Local simulation for review request metrics with:
  - scheduled run with zero queued issues.
  - scheduled run with nonzero queued issues.
  - missing spec artifact count.
  - found spec artifact count.
- JSON fixture validation with `jq`.
- Summary text inspection showing artifact name, duration, conclusion, and
  provider usage availability.
- Confirm `actions/upload-artifact@v7` steps are non-masking through
  `continue-on-error` or equivalent warning-only handling.

Required eval script checks:

- Run a no-live-call report assembly simulation where possible.
- If safe and below the resource gate, run one provider-backed eval path through
  `./scripts/eval_intent.sh --provider <configured-provider> --filter E`.
- Confirm the generated report keeps existing fields:
  - `provider`
  - `timestamp`
  - `total`
  - `passed`
  - `failed`
  - `errors`
  - `results`
- Confirm new fields exist:
  - top-level duration.
  - per-task duration.
  - provider usage availability or usage summary.
- Confirm generated reports are not staged unless explicitly approved.

PR review evidence:

- List touched workflow files and script files.
- Include command outputs or summaries for YAML parsing, JSON validation,
  static scans, and eval-report checks.
- State whether a live GitHub Actions smoke was run. If not, explain the risk
  and what local evidence covers instead.
- State expected external LLM calls and cost actually used.

## Completion Criteria

Implementation is complete when:

- `stage-approval.yml` emits a metadata-only metrics JSON artifact and GitHub
  summary for approval workflow runs.
- `human-acceptance-request.yml` emits a metadata-only metrics JSON artifact
  and GitHub summary for notification runs.
- `spec-review-request.yml` emits a metadata-only metrics JSON artifact and
  GitHub summary that includes product spec artifact availability.
- `technical-spec-review-request.yml` emits a metadata-only metrics JSON
  artifact and GitHub summary that identifies the technical spec review path.
- `scripts/eval_intent.sh` preserves existing scoring report behavior and adds
  timing plus optional provider usage availability.
- Missing provider usage is explicit and not guessed.
- Metrics collection and artifact upload cannot mask primary workflow success
  or failure.
- No sensitive payloads are included in generated metrics artifacts.
- All BDD scenarios have local simulation, static, live, or review evidence as
  mapped above.
- The implementation PR describes whether live GitHub Actions or live provider
  E2E was run and includes resource-use evidence.

## Failure And Escalation

If workflow YAML parsing fails:

- Stop the cycle after reporting the parse error and affected file.
- Do not proceed to live workflow runs.

If metrics collection can mask primary workflow results:

- Treat as a blocker.
- Revise the metrics step/job so failures are warning-only or stop for human
  guidance if that cannot be done safely.

If artifact upload is unavailable:

- Keep summary output as the minimum fallback.
- Report the upload failure as warning-only.
- Do not change the primary workflow conclusion because metrics upload failed.

If provider usage cannot be measured:

- Mark provider usage unavailable with a reason.
- Do not estimate tokens, request counts, or cost in the report fields that
  represent measured provider usage.

If live provider E2E would exceed the resource gate:

- Do not run it.
- Report estimated calls, estimated cost, provider, and reason.
- Ask for explicit human approval or use a no-live-call simulation.

If live GitHub workflow smoke would mutate real issue, Slack, or Project state:

- Do not run it without explicit approval.
- Prefer local simulation and static validation.

If implementation requires external storage, a new SaaS service, budget gates,
provider UX changes, or stage semantics changes:

- Stop and route back for product clarification because that exceeds the
  approved #610 scope.

## Open Questions

- None blocking for implementation.
- Non-blocking: Stage 10 may decide whether duplicated inline metrics helpers
  are acceptable for v1 or whether a small checked-out helper script is worth
  the added workflow checkout cost. Prefer inline helpers unless the duplication
  becomes materially harder to test or review.
