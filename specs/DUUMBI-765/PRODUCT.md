# DUUMBI-765 Product Spec: Hosted GitHub App Staging E2E And Controlled Worker Enablement

Related to #765

Workflow note: this is a spec-only artifact. Any PR containing this document must
use non-closing issue references and must leave #765 open for Stage 10
implementation.

## Summary

DUUMBI Loop now has the provider-core native foundation, local web and infra
scaffold, production-integration persistence, public duumbi.dev Loop entry,
authenticated repository and task surfaces, and the real optional GitHub App
adapter plus Postgres-backed async worker queue.

This slice proves that the GitHub App adapter and async worker path can operate
in hosted Azure staging under explicit operator control. It connects a
non-production GitHub App installation on `hgahub/duumbi-github-test` to
`staging.loop.duumbi.dev`, verifies signed webhook intake, writes durable
queue/run/event/artifact evidence to staging Postgres, enables the worker only
for an approved no-spend E2E window, and disables or scales it back to zero
after evidence is captured.

This is not full DUUMBI Loop completion. The slice is a bounded hosted staging
proof for GitHub App intake and controlled worker execution only.

## Product Outcome

DUUMBI operators can run one controlled hosted staging E2E that proves:

- the staging GitHub App can be installed on the approved test repository,
- `@duumbi-loop` annotations from GitHub reach DUUMBI Loop through a signed
  webhook,
- duplicate or invalid webhook deliveries do not create duplicate work,
- a queued task can move through worker-controlled states,
- no-spend model routing is enforced,
- the admin monitor exposes queue, worker, provider, audit, and failure
  evidence,
- the worker is disabled or scaled down after the E2E window.

Organization users still see GitHub as optional. Native provider-duumbi
workflows remain available without GitHub, and customer-facing model labels
remain DUUMBI-owned.

## Current Context

- #738 delivered the provider-core/native CLI foundation.
- #750 delivered the first local/no-cost `duumbi-loop` web+infra slice.
- #757 delivered the first production-integration `duumbi-loop` slice.
- #759 delivered the public duumbi.dev Loop entry route and Azure staging
  boundary.
- #761 delivered authenticated repository/task/admin surfaces and Postgres
  persistence.
- #763 delivered the real optional GitHub App adapter boundary and
  Postgres-backed async worker queue.

## Stage 5 Acceptance Check

The slice may proceed to Stage 7 and Stage 9 drafting because the core product
and architecture decisions are bounded:

- GitHub remains optional and is proven through a non-production GitHub App.
- The staging test repository is `hgahub/duumbi-github-test`.
- `duumbi-loop` owns app/API behavior, webhook handling, queue state, worker
  behavior, admin monitor evidence, and no-spend model routing.
- `duumbi-infra` owns Azure staging secret references, Container App
  environment variables, worker enablement controls, budget controls, and
  preview/apply evidence.
- `duumbi-web` changes are out of scope unless the public entry copy or CTA
  needs a small correction.
- Hosted worker execution is disabled by default and may run only during an
  explicitly approved E2E window.
- No production auth, live Stripe, live provider/model spend, GitLab adapter,
  or Ralph cycles are allowed.

## Users

- DUUMBI operator: configures the non-production GitHub App, staging secrets,
  worker E2E window, and teardown/disable procedure.
- Organization admin: verifies provider connection, repository registration,
  worker health, queue state, audit, and blocked/failed run evidence.
- Organization member: triggers a staged annotation task and reviews run state
  and artifacts.
- Reviewer/implementation agent: verifies the slice without treating hosted
  staging as production readiness.

## In Scope

- Non-production GitHub App staging configuration boundary.
- GitHub App installation for `hgahub/duumbi-github-test`.
- Staging webhook URL and HMAC signature verification.
- Key Vault and Container App secret-reference model for GitHub App settings.
- Allowlisted staging/test repository registration.
- Hosted `@duumbi-loop` annotation intake from GitHub webhook events.
- Staging Postgres persistence for queue, runs, events, artifacts, webhook
  deliveries, and provider state.
- Controlled worker enablement with max replicas 1, scale-to-zero where
  possible, and explicit E2E approval.
- No-spend DUUMBI model routing for the full hosted smoke.
- Admin monitor evidence for worker health, queue backlog, blocked/failed runs,
  provider access, audit, and webhook delivery state.
- Disable, scale-to-zero, or teardown evidence after E2E.

## Out Of Scope

- Full DUUMBI Loop product completion.
- Production auth or central `auth.duumbi.dev`.
- Live Stripe products, live Stripe webhooks, checkout, billing portal, or
  invoices.
- Live provider/model spend or external LLM calls.
- Customer exposure of raw provider/model SKUs.
- GitLab adapter implementation.
- GitHub Marketplace publication.
- Broad repository write-back, PR authoring, merge automation, or production
  repository automation.
- Production GitHub App credentials.
- Persistent hosted worker execution outside an approved E2E window.
- Creating a new production database service.
- Ralph cycles.

## Product Requirements

### Staging GitHub App Setup

Operators can configure a non-production GitHub App for staging. The app must be
owned by the DUUMBI/hgahub operator boundary, not by an individual developer's
personal workflow. The app must be installed only on approved test repositories
for this slice, starting with `hgahub/duumbi-github-test`.

The product must communicate that GitHub is optional. Native DUUMBI workflows
must not require a GitHub provider connection.

### Secret Boundary

GitHub App credentials and webhook secrets must be stored through Key Vault and
Container App secret references. Normal issue comments, PR bodies, Pulumi
outputs, logs, screenshots, and evidence files must not contain raw secrets,
private keys, installation tokens, database URLs, or webhook payload bodies.

### Hosted Webhook Intake

Staging accepts GitHub webhook deliveries only when the webhook secret is
configured and the request signature is valid. Unsupported events, unknown
installations, disabled providers, empty annotations, malformed annotations, and
duplicate deliveries must be recorded in safe evidence without retry storms or
duplicate queue writes.

### Allowlisted Test Repository

Only the approved staging repository may be used for the live hosted E2E:

```text
hgahub/duumbi-github-test
```

If any other repository is included in the GitHub App installation or webhook
path, the E2E must stop with findings unless the operator explicitly extends the
allowlist in a later issue.

### Controlled Worker Enablement

The hosted worker is disabled by default. To run the E2E, an operator must
explicitly enable the worker through the approved staging configuration and
record the E2E window. The worker must run with max replicas 1 and deterministic
no-spend routing. After E2E, the worker must be disabled, scaled to zero, or the
staging resources must be torn down according to the approved runbook.

### No-Spend Routing

All hosted smoke execution must use the no-spend DUUMBI model router. The UI and
run records may show DUUMBI-owned labels such as Fast, Balanced, Deep Research,
Strict Review, and Private/BYOK, but no customer surface may expose raw provider
or model SKUs.

### Staging Evidence

The E2E evidence must show:

- health/readiness status,
- configured environment mode,
- persistence backend,
- worker enabled/approved state before, during, and after the E2E window,
- webhook delivery status and idempotency behavior,
- queued/running/review/completed/failed or blocked state transitions,
- admin monitor queue and worker status,
- no-spend and no-live-service guardrails,
- teardown/disable state.

## BDD Scenarios

### Scenario: Native DUUMBI remains available without GitHub

Given a staging organization has no GitHub provider connection
When a user opens the Loop app
Then native provider-duumbi task creation remains available
And GitHub is presented as an optional adapter.

### Scenario: Operator configures a non-production GitHub App

Given the operator has created a non-production GitHub App for staging
When the app credentials are configured
Then the raw private key, client secret, webhook secret, and installation token
are stored only through approved secret references
And no raw secret appears in source control, logs, PR text, issue comments, or
evidence artifacts.

### Scenario: Test repository is allowlisted

Given the GitHub App is installed for staging
When the operator registers repositories for the E2E
Then `hgahub/duumbi-github-test` is accepted
And any unapproved repository is rejected or blocked before queueing.

### Scenario: Staging webhook requires a valid signature

Given GitHub sends a webhook to staging
When the signature is missing or invalid
Then DUUMBI Loop rejects the request
And no task request, run, artifact, or worker job is created.

### Scenario: Duplicate webhook delivery is idempotent in staging

Given GitHub sends the same delivery id twice
When staging receives both requests
Then only one accepted queue item is created
And the duplicate delivery is recorded as ignored.

### Scenario: Empty annotation does not retry forever

Given a signed GitHub comment contains only `@duumbi-loop`
When staging receives the webhook
Then DUUMBI Loop records an ignored delivery with empty-annotation evidence
And returns a successful non-queued response to prevent retry storms.

### Scenario: Annotation queues hosted staging work

Given the GitHub App is installed on `hgahub/duumbi-github-test`
And the repository is registered and enabled
And the organization has available test credits
When a signed GitHub comment contains `@duumbi-loop review this change`
Then DUUMBI Loop creates a task request, run, and queued worker job
And the run references the repository, annotation, model label, and webhook
delivery id.

### Scenario: Worker remains disabled by default

Given staging is deployed outside an E2E window
When the operator checks readiness and evidence
Then the worker reports disabled or scaled to zero
And worker execution outside explicit E2E is zero.

### Scenario: Worker runs only during an approved E2E window

Given a queued staging worker job exists
And the operator explicitly enables the worker for E2E
When the worker claims the job
Then the run moves from queued to running
And the worker uses the no-spend DUUMBI model router
And admin monitor shows the active or recently completed worker state.

### Scenario: Worker disable is verified after E2E

Given the hosted E2E has completed or failed with evidence
When the operator ends the E2E window
Then the worker is disabled, scaled to zero, or destroyed
And evidence records the final disabled/teardown state.

### Scenario: Billing and cloud cost gates fail closed

Given test billing or cloud budget guardrails are missing
When a hosted E2E run is requested
Then the system stops before worker execution
And records findings without live Stripe calls, live model spend, or uncontrolled
cloud cost.

### Scenario: Admin can inspect queue and provider evidence

Given a hosted GitHub annotation was processed
When an admin opens the worker/provider monitor
Then they can see webhook delivery status, provider access status, queue state,
attempt counts, blocked/failed reasons, and audit events.

## Acceptance Criteria

- The product and technical specs are merged in `hgahub/duumbi` as spec-only
  artifacts for #765.
- The spec PR uses only non-closing references such as `Related to #765`.
- The execution issue remains open after the spec PR.
- The Stage 10 prompt is included in the technical spec.
- The implementation path is bounded to staged GitHub App E2E and controlled
  worker enablement.
- The test repository is explicitly `hgahub/duumbi-github-test`.
- The full DUUMBI Loop product is explicitly not complete.

## Codex Self-Review

- Product scope is bounded to hosted staging E2E and controlled worker
  enablement.
- GitHub remains optional and GitLab remains out of scope.
- The spec does not require production auth, live Stripe, live model/provider
  spend, raw provider/model SKU exposure, or Ralph cycles.
- The spec records clear operator gates for secrets, worker enablement,
  cloud budget, and teardown.
- No unresolved product blocker remains for drafting. Missing live secrets are
  implementation prerequisites, not product-scope blockers.
