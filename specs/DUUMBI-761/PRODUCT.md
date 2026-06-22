# DUUMBI-761: Authenticated Repo-To-Run Journey And Admin Provider Access

Spec for #761.

Related to #738, #750, #757, and #759.

This PR is specification-only and must leave #761 open. Do not use closing
references such as "closes", "fixes", or "resolves" for #761, #759, #757,
#750, or #738.

## Summary

Define the next bounded DUUMBI Loop product slice after:

- #738 delivered the provider-core/native CLI foundation in `hgahub/duumbi`.
- #750 delivered the first local/no-cost `duumbi-loop` web/API scaffold.
- #757 delivered the first production-integration `duumbi-loop` slice with
  AuthAdapter, Postgres persistence, Stripe test entitlement mirror,
  no-spend model routing, native no-provider preflight, and curated vault
  import boundary.
- #759 delivered the public `duumbi.dev` Loop entry route and Azure staging
  boundary.

This slice turns the staged Loop foundation into the first authenticated
repo-to-run journey. A user can enter DUUMBI Loop from `duumbi-web`, sign in,
register or select a GitHub repository as an optional adapter, create a
repository-linked task by annotation or instruction, and track the workflow in
the Loop UI. An administrator can monitor runs and configure provider access
that customers use indirectly through DUUMBI-owned model labels.

This slice does not complete the full DUUMBI Loop product.

## Product Outcome

An authenticated DUUMBI Loop user can start from the public `duumbi.dev` Loop
entry, reach the Loop app, connect repository context, create a task using a
clear annotation convention, and follow the run from intake through review
artifacts. An organization administrator can inspect the same work across the
organization and configure platform provider access without exposing raw
provider/model SKUs or provider credentials to customers.

## Source Context

Verified during spec drafting:

- #738, #750, #757, and #759 are closed as completed bounded slices.
- #761 is open and records the requested next product goal.
- `hgahub/duumbi-loop` currently exposes local/test login, organization
  dashboard, runs, providers, billing, native no-provider run creation,
  Postgres persistence, Stripe test entitlement mirror, no-spend model route
  decisions, curated vault import, `/health`, `/ready`, and
  `/ops/e2e-evidence`.
- `hgahub/duumbi-loop` does not yet expose the complete product journey for
  GitHub repository registration, annotation-based task intake, visible step
  tracking, or admin provider-access configuration.
- `hgahub/duumbi-web` already has the public Loop route from #759. This slice
  should add or adjust the navigational Loop menu/link and CTA behavior only as
  needed for the authenticated journey.
- `hgahub/duumbi-infra` already hosts the staging boundary from #759. This
  slice may need secret/config additions for provider-access placeholders, but
  it must not enable live provider/model spend by default.
- GitHub and GitLab remain optional adapters. The native `provider-duumbi` path
  remains primary.

## Scope

This product slice covers:

- `duumbi-web` Loop navigation into the authenticated Loop app,
- authenticated Loop app entry using the existing AuthAdapter boundary,
- organization-scoped GitHub repository registration as an optional adapter,
- repository selection for task creation,
- annotation or instruction based task intake for a selected repository,
- run tracking UI with step state, events, artifacts, and review results,
- admin run monitoring across the organization,
- admin provider-access configuration status and controls,
- provider route decision visibility for administrators,
- provider/model SKU hiding from customer-facing surfaces,
- audit evidence for repository, run, provider-access, and routing changes,
- test/staging-only E2E without live provider/model spend.

## Non-Goals

- No implementation code in this spec PR.
- No Ralph cycles in this spec PR.
- No Greptile review for this spec PR.
- No full DUUMBI Loop product launch.
- No production `auth.duumbi.dev` service implementation.
- No live Stripe products or live payment changes.
- No live provider/model spend by default.
- No requirement that GitHub or GitLab become prerequisites for DUUMBI Loop.
- No exposure of raw provider/model SKUs to customer-facing users.
- No storage of provider secrets in source control, PR bodies, issue comments,
  logs, evidence files, or browser-rendered customer pages.
- No broad worker queue execution beyond explicit, bounded E2E.
- No arbitrary vault import expansion.
- No production-grade GitHub App marketplace listing requirement in this slice.

## Users And Roles

- Visitor: reaches DUUMBI Loop from `duumbi-web`.
- Authenticated user: signs in and views the organization dashboard.
- Developer: registers or selects repositories and creates repository-linked
  tasks when permitted.
- Reviewer: views run state, artifacts, and review results.
- Organization admin: monitors organization runs, provider status, route
  decisions, and audit events.
- Platform admin: configures provider access and routing policy that customers
  consume through DUUMBI-owned model labels.

## Product Surfaces

### Public Loop Navigation

Owner: `hgahub/duumbi-web`.

The public DUUMBI site must expose a Loop menu item or equivalent top-level
navigation path. The path must make it clear that DUUMBI Loop is the app entry,
not only a marketing page. In staging, the CTA may target
`https://staging.loop.duumbi.dev` or a configured request-access path.

The page and navigation must not claim that the full DUUMBI Loop product is
complete. GitHub/GitLab must be presented as optional adapters, not as the only
way to use Loop.

### Authenticated Loop Entry

Owner: `hgahub/duumbi-loop`.

The Loop app must require authentication for organization dashboard, repository
registration, task creation, run tracking, and admin provider settings.
Staging may continue to use the local/test AuthAdapter while preserving the
future `auth.duumbi.dev` compatibility boundary.

### Repository Registration

Owner: `hgahub/duumbi-loop`.

A user with the right role can register a GitHub repository as an optional
adapter. The product must support:

- organization-scoped repository rows,
- provider connection status,
- repository URL or owner/name metadata,
- default branch or selected ref metadata,
- repository limit preflight from entitlement,
- disabled states for provider revoked, plan limit, or admin disabled,
- native workspace registration without Git provider credentials.

For this slice, a real GitHub credential may be simulated or test-scoped unless
the technical gates explicitly approve a GitHub App/OAuth path. The UI must not
require GitHub credentials to prove the native path still works.

### Annotation-Based Task Intake

Owner: `hgahub/duumbi-loop`.

The task creation surface must let a user create a run from a selected
repository using an annotation or repository-linked instruction. The accepted
annotation format is implementation-owned, but it must be explicit and
testable. Examples:

- `@duumbi-loop draft a spec for the selected issue`
- `@duumbi-loop review this change with Strict Review`
- a form field containing an annotation and selected repository/ref metadata

The annotation must resolve into a DUUMBI Loop run with:

- organization ID,
- actor user ID,
- repository registration ID,
- optional provider connection ID,
- annotation text,
- selected ref or workspace ref,
- workflow kind,
- DUUMBI-owned model label,
- estimated credits,
- initial step state,
- audit event.

### Run Tracking And Review

Owner: `hgahub/duumbi-loop`.

The user-facing UI must show:

- run state,
- current workflow step,
- queued/running/completed/blocked/failure states,
- credit estimate and final credit usage,
- model label,
- source/repository summary,
- artifacts,
- review target or review result when present,
- no-spend and no-live-provider evidence in staging.

The UI must be useful for repeated monitoring. Avoid a decorative-only view.

### Admin Monitoring

Owner: `hgahub/duumbi-loop`.

An organization admin can view:

- active and recent runs across the organization,
- run events and state transitions,
- repository registration status,
- provider connection status,
- model route decisions,
- credit debits and entitlement state,
- audit events for repository, task, provider-access, and routing changes.

### Admin Provider Access Configuration

Owner: `hgahub/duumbi-loop`, with secret/runtime support from
`hgahub/duumbi-infra` when hosted.

An admin surface must show provider access in a customer-safe way:

- DUUMBI-owned labels: Fast, Balanced, Deep Research, Strict Review,
  Private/BYOK,
- provider access status: not configured, test configured, enabled, disabled,
  or blocked,
- route mode: no-spend/mock, platform key, BYOK, or disabled,
- last validation timestamp,
- audit trail.

Customer-facing users must not see raw provider/model SKUs. Admin/audit
surfaces may store or show raw provider/model SKU metadata only when the user is
authorized and the value is needed for operations. Provider secrets must never
be displayed after entry.

## BDD Scenarios

### Scenario: Public navigation reaches Loop

Given a visitor opens `duumbi.dev`
When they use the Loop menu or Loop CTA
Then they can reach the configured DUUMBI Loop app entry
And the public copy does not claim that the full DUUMBI Loop product is
complete.

### Scenario: Authenticated app entry protects organization data

Given a visitor is not authenticated
When they open an organization dashboard, repository page, run page, or admin
provider settings page
Then the app requires authentication
And no organization data, repository metadata, provider status, route decision,
or artifact content is exposed.

### Scenario: User signs in and sees organization dashboard

Given a user completes the staging auth flow
When they open the Loop dashboard
Then they see their organization, current runs, repository status, credit
summary, and available DUUMBI-owned model labels.

### Scenario: GitHub repository registration is optional

Given a signed-in user has permission to manage repositories
When they open repository registration
Then GitHub is available as an optional adapter
And native workspace registration remains available without GitHub credentials.

### Scenario: Repository limit blocks registration before write

Given an organization has reached its repository entitlement limit
When a user submits another repository registration
Then the request fails closed before creating the repository row
And an audit event records the denied preflight without storing secrets.

### Scenario: Provider revoked repository is not selectable

Given a repository depends on a revoked provider connection
When a user opens the task creation form
Then the repository appears disabled or unavailable
And the UI explains that provider access must be repaired or native context
must be used.

### Scenario: User creates annotation task for registered repository

Given a signed-in developer selects a registered repository
And enters a valid `@duumbi-loop` annotation or repository-linked instruction
When they submit the task
Then the app creates a Loop run tied to that repository
And records the annotation, workflow kind, actor, selected ref, model label,
credit estimate, and audit event.

### Scenario: Invalid annotation is rejected

Given a signed-in developer enters an empty, malformed, or unsupported
annotation
When they submit the task
Then no run is created
And the UI shows a validation error without consuming credits.

### Scenario: Credit preflight blocks over-cap run

Given a task estimate exceeds the organization's max credits per run or
available credits
When a user submits the task
Then the request fails closed before queueing or writing a run
And the denial is visible in admin audit evidence.

### Scenario: User tracks workflow progress

Given a run exists for a repository-linked annotation
When the user opens the run detail page
Then they can see the run state, current step, run events, selected repository,
model label, estimated credits, artifacts, and review status.

### Scenario: Review artifact is visible when complete

Given a run produces a review result or GraphPatch review target
When the user opens the run detail page
Then the review summary and artifact references are visible
And the UI does not expose raw source dumps or secrets.

### Scenario: Admin sees organization run monitor

Given an organization admin opens the admin monitor
When runs are active or completed
Then the admin can see organization-wide run state, actor, repository, workflow
step, model label, route mode, credit usage, and failure/blocker summary.

### Scenario: Admin configures provider access status

Given a platform admin opens provider access settings
When they configure a provider in test or no-spend mode
Then the provider access status updates
And the secret material is write-only
And an audit event records who changed the configuration.

### Scenario: Customer cannot see raw provider SKUs

Given a customer-facing user opens dashboard, task creation, or run detail
When model choices or route decisions are shown
Then they see only DUUMBI-owned model labels
And raw provider/model SKUs are not visible.

### Scenario: Admin audit can inspect route decision metadata

Given an authorized admin opens route decision audit
When a run has a model route decision
Then the admin can see the DUUMBI-owned label, route mode, spend flag, and
audit metadata
And any raw provider/model SKU is redacted unless explicitly authorized.

### Scenario: Staging E2E uses no-spend routing

Given the staging environment is used for E2E
When a repository-linked annotation task is created and tracked
Then external LLM calls are 0
And live provider/model spend is 0
And live Stripe calls are 0
And Git provider credentials are either 0 or explicitly test-scoped.

### Scenario: GitHub adapter failure does not break native path

Given GitHub adapter credentials are missing, revoked, or disabled
When a user creates a native workspace task
Then the native `provider-duumbi` path still works
And the UI clearly separates native context from GitHub repository context.

### Scenario: Admin provider access does not start spend by default

Given an admin saves provider access configuration
When the environment is staging or no-spend mode
Then no live provider/model call is made
And spend remains blocked until explicit production approval.

### Scenario: Audit trail covers sensitive actions

Given repository registration, task creation, provider access changes, or route
policy changes occur
When an admin opens audit evidence
Then each action includes actor, organization, target, action, timestamp, and
correlation ID
And secrets are not included in audit payloads.

## Acceptance Gates

Stage 10 implementation must not start until:

- product and technical specs are approved,
- auth ownership remains within the `duumbi-loop` AuthAdapter boundary for this
  slice,
- GitHub credential ownership is decided as test-scoped/stubbed or explicitly
  approved for a real GitHub App/OAuth flow,
- provider secret storage is secret-backed and admin-only,
- billing/credit preflight semantics are mapped to tests,
- cross-repo write order is explicit,
- no-spend staging policy is testable,
- the implementation plan preserves non-closing references until final
  workflow closure.
