# DUUMBI-763 Product Spec: GitHub Adapter And Async Worker Slice

Related to #763

Workflow note: this is a spec-only artifact. Any PR containing this document must
use non-closing issue references and must leave #763 open for Stage 10
implementation.

## Summary

DUUMBI Loop already has the native provider-core foundation, local and
production-integration scaffolding, the public duumbi.dev Loop entry, and an
authenticated repository-to-run journey with test/stub provider access. This
slice turns the GitHub repository adapter from a test/stub boundary into a real,
optional GitHub App integration and introduces controlled asynchronous worker
execution for queued Loop tasks.

This is not full DUUMBI Loop completion. The slice focuses on a bounded GitHub
adapter and worker path that proves repository events can enter DUUMBI Loop,
move through a controlled queue, execute with no live model/provider spend, and
surface trustworthy run/review state to users and admins.

## Product Outcome

Users can connect a GitHub App installation, register allowed repositories,
submit or receive annotation-driven tasks, and watch those tasks progress from
queued to running to review/completed/failed without exposing raw provider/model
SKUs or making GitHub a prerequisite for native DUUMBI workflows.

Admins can verify provider access health, webhook intake, queue backlog, worker
state, retries, blocked runs, and audit evidence. Operators can keep hosted
staging no-spend and low-cost while enabling worker execution only for explicit
E2E windows.

## Current Context

- #738 delivered the provider-core/native CLI foundation.
- #750 delivered the first local/no-cost `duumbi-loop` web+infra slice.
- #757 delivered the first production-integration `duumbi-loop` slice with
  AuthAdapter hardening, Postgres persistence, Stripe test entitlement mirroring,
  no-spend model routing, native no-provider preflight, curated vault import
  boundary, and local Postgres evidence.
- #759 delivered the public duumbi.dev Loop entry route and Azure staging
  boundary.
- #761 delivered authenticated repository/task/admin surfaces, optional provider
  adapter boundaries, annotation task intake, run/event/artifact tracking, admin
  monitoring, provider access configuration, and Postgres persistence for
  task/run/admin objects.

## Stage 5 Acceptance Check

The slice may proceed to Stage 7 and Stage 9 drafting because the product goal
is bounded and the remaining ownership decisions are resolved in this spec:

- GitHub is implemented as an optional GitHub App adapter, not as a prerequisite
  for native DUUMBI runs.
- `duumbi-loop` owns the GitHub adapter, webhook intake, token exchange,
  repository sync, queue, worker, and admin monitoring behavior for this slice.
- GitHub OAuth is not selected for repository installation in this slice. It may
  remain a future user-identity enhancement, but installation access must use a
  GitHub App.
- Worker execution uses a Postgres-backed queue first. No new external queue
  service is required for this slice.
- Hosted staging may use a non-production GitHub App only when app secrets are
  explicitly configured and an E2E window is approved.
- CI and local E2E must use deterministic fake GitHub fixtures and must not need
  live GitHub credentials.
- No live provider/model spend, live Stripe products, production auth, or Ralph
  cycles are allowed.

## Users

- Organization member: starts native and GitHub-backed Loop tasks and reviews
  progress for repositories they can access.
- Organization admin: connects GitHub, approves repository registration, manages
  provider access, and monitors queue and worker health.
- DUUMBI operator: configures staging secrets, reviews webhook/worker evidence,
  and keeps no-spend and cloud-cost controls intact.

## In Scope

- GitHub App ownership decision and repository installation flow.
- GitHub repository registration, metadata sync, and permission validation.
- GitHub webhook intake for repository events and `@duumbi-loop` annotation
  tasks.
- Secure token/secret storage policy and rotation boundary.
- Postgres-backed asynchronous worker queue for bounded task execution.
- Worker state transitions from queued to running to review/completed/failed.
- Retry, cancel, timeout, and failure evidence.
- User-facing workflow tracking updates.
- Admin monitoring for provider access, webhook intake, queue backlog, worker
  health, blocked runs, audit records, and failed jobs.
- No-spend DUUMBI model routing for local and hosted smoke.
- Billing entitlement and credit preflight before queue writes and before worker
  execution.
- Security and privacy controls for webhook payloads, repository metadata, and
  installation credentials.

## Out Of Scope

- Full DUUMBI Loop product completion.
- GitLab real adapter implementation beyond preserving interface boundaries.
- Production auth or central `auth.duumbi.dev`.
- Live Stripe products or live Stripe payment flows.
- Live provider/model spend or external LLM calls.
- Exposing raw provider/model SKUs to customers.
- GitHub Marketplace publication.
- Broad pull request authoring, merge automation, or repository write-back
  beyond explicitly approved annotation/review evidence.
- Hosted Azure apply or smoke unless required secrets, budget guardrails, and
  explicit approval are present.
- A new external managed queue service.
- Ralph cycles.

## Product Requirements

### GitHub App Connection

An organization admin can start a GitHub App installation flow from provider
settings. After installation, DUUMBI Loop records a provider connection with the
installation id, selected permissions summary, selected repository scope,
credential reference, and validation timestamp.

The UI must make the relationship clear:

- GitHub is optional.
- Native DUUMBI workspaces still work without GitHub.
- GitHub access can be revoked or disabled without deleting native runs.
- Repository access is limited to installed and explicitly registered
  repositories.

### Repository Registration And Sync

An admin can register repositories from the GitHub installation scope. DUUMBI
Loop stores provider owner, repository name, default branch, selected ref policy,
metadata sync timestamp, and repository status.

If the installation no longer grants access, the repository becomes disabled by
provider revocation and no new queued work may be created for it.

### Annotation Intake

DUUMBI Loop accepts GitHub webhook events for approved repositories and can turn
an annotation such as `@duumbi-loop ...` into a task request. The annotation
intake must preserve enough source context for traceability while avoiding raw
payload exposure in normal user/admin views.

Duplicate webhook delivery must be idempotent.

### Async Worker Execution

Task creation writes a durable queued run/job after auth, repository access,
billing entitlement, credit estimate, model label, and provider policy checks
pass. A worker claims queued work, moves it to running, executes the no-spend
DUUMBI-native path or approved GitHub-context path, records events/artifacts,
and finishes in review/completed/failed/cancelled/blocked.

Worker execution must be controllable:

- disabled by default in hosted staging,
- enabled only through explicit configuration for local or approved E2E,
- bounded by timeout, retry, cancellation, and concurrency rules,
- visible to admins.

### Tracking And Admin Monitoring

Users can see queued, running, review, completed, failed, cancelled, and blocked
states with timestamps and evidence. Admins can see queue backlog, worker lease
health, retry counts, blocked reasons, provider access status, and audit events.

### Model Labels

Users continue to see DUUMBI-owned labels only: Fast, Balanced, Deep Research,
Strict Review, and Private/BYOK. Provider/model SKUs remain hidden from customer
surfaces and may appear only in admin/audit records where explicitly allowed.

### Billing And Credits

Run queueing fails closed before durable queue writes when the organization is
over repository limit, over parallel run limit, out of credits, or above
max-credits-per-run. Worker execution rechecks entitlement before claiming work.
If a queued job is cancelled, blocked before execution, or fails before billable
work, credit reservation must be released or recorded as zero final usage.

### Hosted Staging

Hosted staging may prove the flow only with deterministic no-spend routing and a
non-production GitHub App. If GitHub App secrets are absent, hosted smoke must
skip live GitHub intake and record that the GitHub live adapter gate is blocked,
while local fake-GitHub E2E remains required.

## BDD Scenarios

### Scenario: Native DUUMBI remains available without GitHub

Given an organization has no GitHub provider connection
When a member starts a native DUUMBI Loop task
Then the task can be queued through the provider-duumbi path
And the UI does not require GitHub credentials.

### Scenario: Admin starts GitHub App installation

Given I am an organization admin
When I open provider settings and choose GitHub
Then DUUMBI Loop starts a GitHub App installation flow
And the page states that GitHub is optional
And the installation request uses repository-scoped permissions only
And the callback requires a one-time state value bound to my session, user, and
organization.

### Scenario: GitHub installation is stored without raw tokens

Given GitHub returns an installation for an organization
When DUUMBI Loop validates the installation
Then the provider connection stores installation id, scopes summary, status, and
credential reference
And raw tokens are not stored in normal database fields or logs.

### Scenario: Repository registration validates installation access

Given an organization has a valid GitHub installation
When an admin registers a repository from that installation
Then DUUMBI Loop stores repository owner, name, default branch, provider
connection, and enabled status
And a member can select the repository for a task.

### Scenario: Revoked GitHub access disables repository queueing

Given a registered GitHub repository loses installation access
When DUUMBI Loop syncs repository metadata
Then the repository status becomes disabled by provider revocation
And new GitHub-backed task creation fails before queueing
And native DUUMBI tasks remain available.

### Scenario: Webhook signature is required

Given a GitHub webhook request has no valid signature
When DUUMBI Loop receives the request
Then the request is rejected
And no task request, run, or worker job is written.

### Scenario: Duplicate webhook delivery is idempotent

Given GitHub delivers the same event id twice
When DUUMBI Loop processes both deliveries
Then only one accepted task request and worker job are created
And the duplicate delivery is recorded as idempotently ignored.

### Scenario: Annotation creates a queued task

Given a valid webhook from an enabled repository contains `@duumbi-loop`
When DUUMBI Loop parses the annotation
Then it creates a task request and queued run/job
And the run shows repository, selected ref, annotation text, model label, and
queued timestamp.

### Scenario: Entitlement blocks queue writes

Given an organization is out of credits or over max credits per run
When a member submits a GitHub annotation task
Then DUUMBI Loop rejects the request before writing a queued job
And the user sees a billing or entitlement explanation.

### Scenario: Parallel run limit blocks excess queued work

Given an organization is already at its parallel run limit
When another GitHub-backed task is submitted
Then DUUMBI Loop rejects the new task before creating a task request, run, or
worker job
And the existing running tasks continue.

### Scenario: Worker claims queued work

Given a queued worker job exists
And worker execution is enabled for the environment
When the worker claims the job
Then the run moves from queued to running
And an audit event records worker id, lease timestamp, and attempt number.

### Scenario: Worker completion creates review evidence

Given a worker is running a GitHub-backed annotation task with no-spend routing
When the execution finishes successfully
Then the run moves to review or completed
And review artifacts, source summary, final credits, and no-spend evidence are
visible in run detail.

### Scenario: Worker failure records actionable evidence

Given a worker job fails
When the failure is retryable
Then DUUMBI Loop records the failure event, increments attempt count, and
reschedules within retry limits
And admins can inspect the failure reason without seeing secrets.

### Scenario: Worker timeout fails safely

Given a worker job exceeds the configured timeout
When the timeout is detected
Then the job is cancelled or failed according to retry policy
And the run shows timeout evidence
And no worker lease remains stuck indefinitely.

### Scenario: User cancellation stops pending work

Given a user has a queued or running task
When the user cancels the task
Then queued work is cancelled before worker claim where possible
And running work receives a cancellation signal
And the final run state is cancelled with audit evidence.

### Scenario: Admin monitors backlog and blocked runs

Given worker jobs are queued, running, failed, or blocked
When an admin opens the monitor
Then the admin sees backlog count, oldest queued age, active worker leases,
retry counts, blocked reasons, and provider access status.

### Scenario: Hosted smoke skips live GitHub when secrets are absent

Given hosted staging has no GitHub App private key or webhook secret configured
When hosted smoke runs
Then live GitHub intake is skipped with explicit evidence
And no live provider/model spend, live Stripe calls, or GitHub production
credentials are used.

### Scenario: Hosted smoke can use a staging GitHub App when approved

Given a non-production GitHub App is configured for staging
And an operator explicitly approves an E2E window
When a webhook from an allowlisted test repository is delivered
Then DUUMBI Loop accepts it, queues no-spend work, runs the worker within the
approved window, and records evidence
And the worker is disabled again after the E2E window.

## UX Notes

- GitHub should appear as an optional connected provider, not as the default
  DUUMBI Loop path.
- Provider settings must distinguish installation status, repository status, and
  provider access config.
- Run detail must emphasize workflow state and evidence, not implementation
  internals.
- Admin monitoring can be utilitarian and dense. It should prioritize backlog,
  stuck leases, retry spikes, revoked provider access, and blocked billing.

## Acceptance Criteria

- The product spec clearly states that this is not full DUUMBI Loop completion.
- GitHub App is selected as the real optional GitHub adapter path for this
  slice.
- Native provider-duumbi remains available without GitHub.
- BDD scenarios cover installation, repository sync, webhook security,
  annotation intake, queueing, worker execution, retry/cancel/timeout, billing
  preflight, admin monitoring, hosted no-spend smoke, and secret absence.
- Non-goals explicitly exclude GitLab real adapter, production auth, live
  Stripe, live provider/model spend, Marketplace publication, and Ralph cycles.
- Stage 10 implementation may begin only if the technical spec maps all BDD
  scenarios to tests and preserves no-spend/cloud-cost controls.

## Codex Self-Review

- Stage 7 product gate: Pass.
- Scope is bounded to GitHub optional adapter plus controlled async worker.
- Customer-facing behavior, admin behavior, non-goals, and acceptance criteria
  are explicit.
- BDD scenarios cover happy paths, denial paths, security, billing, retry,
  cancellation, timeout, hosted smoke, and no-secret operation.
- No implementation code is included.
- No closing issue reference is used.
