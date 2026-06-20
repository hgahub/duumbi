# DUUMBI-757: DUUMBI Loop Production Integration Slice

Spec for #757.

Related to #738 and #750.

This PR is specification-only and must leave #757 open. Do not use closing
references such as "closes", "fixes", or "resolves" for #757, #750, or #738.

## Summary

Define the next bounded DUUMBI Loop production-integration slice after:

- #738 delivered the provider-core/native CLI foundation in `hgahub/duumbi`.
- #750 and `hgahub/duumbi-loop` PR #1 delivered the first local/no-cost
  `duumbi-loop` web and infrastructure slice.

This slice moves DUUMBI Loop from local proof to controlled staging integration.
It must prove that the user-facing Loop experience can run against production
shaped boundaries for auth, persistence, billing, deployment, vault import, and
model routing while preserving the native DUUMBI path.

This spec does not claim the full DUUMBI Loop product is complete. It defines a
buildable staging integration slice that later implementation can execute in
small cross-repo PRs.

## Product Outcome

A DUUMBI operator can deploy and test a staging Loop path where a user can:

- visit a public Loop entry point using the duumbi.dev visual language,
- sign in through the `duumbi-loop` `AuthAdapter` boundary,
- access a persisted organization dashboard backed by Postgres,
- see Stripe test-mode plan and credit entitlements,
- start or preflight a native DUUMBI Loop run without GitHub or GitLab
  credentials,
- see DUUMBI-owned model labels rather than raw provider/model SKUs,
- import only curated allowlisted vault knowledge,
- verify that Azure staging resources respect the approved budget and
  scale-to-zero policy.

The slice is successful when staging behaves like a production-shaped system
without requiring production auth, live Stripe products, live model spend, real
Git provider credentials, or full product completion.

## Source Context

Verified during spec drafting:

- `hgahub/duumbi` PR #749 is merged and provides the `duumbi::loop_native`
  provider-duumbi foundation, including native intake/spec and GraphPatch review
  target support.
- `hgahub/duumbi-loop` PR #1 is merged and implements the first local/no-cost
  Rust/Axum app/API scaffold with local auth, in-memory state, seeded
  organization data, Stripe-shaped webhook tests, native no-Git run creation,
  and GraphPatch review evidence.
- `duumbi-loop/docs/e2e/duumbi-750-local-e2e.md` records external LLM calls = 0,
  Git provider credentials = 0, hosted cloud resources = 0, live Stripe calls =
  0, production auth dependencies = 0, and Ralph cycles = 0 for the local slice.
- `duumbi-web/src/styles/global.css` defines the current duumbi.dev visual token
  system: parchment/ink/blue/rust for public pages and dark DUUMBI green accents
  for operational surfaces.
- `duumbi-infra/stack-persistent.ts` already contains Azure Native Pulumi
  patterns for resource groups, Key Vault, Log Analytics, and a project-level
  USD 20 monthly budget.
- `duumbi-registry/AGENTS.md` documents a Rust/Axum registry with SQLite,
  local password auth, GitHub OAuth, JWT sessions, and embedded app tests. This
  is useful precedent only; it is not the Loop auth or billing source of truth.

## Scope

This product slice covers a staging production-integration path for:

- public Loop route integration using duumbi.dev visual language,
- `duumbi-loop` AuthAdapter hardening and future `auth.duumbi.dev`
  compatibility,
- Postgres-backed Loop metadata for users, organizations, memberships,
  entitlements, credit ledger, runs, artifacts, knowledge imports, audit events,
  and model policy,
- Stripe test-mode products, webhooks, entitlement mirror, and credit preflight,
- Azure staging resource boundary with budget alerts and scale-to-zero behavior,
- hosted vault import allowlist and provenance,
- DUUMBI-owned model label policy with no-spend staging router,
- native provider-duumbi run path without GitHub/GitLab credentials,
- live E2E evidence for local Postgres and hosted staging smoke.

## Non-Goals

- No implementation code in this spec PR.
- No Ralph cycles in this spec PR.
- No Greptile review for this spec PR.
- No full production launch.
- No live Stripe products or live card/payment flows.
- No production `auth.duumbi.dev` service implementation.
- No live model provider spend.
- No real GitHub/GitLab adapter requirement.
- No full dashboard/product polish claim.
- No final pricing decision beyond test-mode entitlement and credit policy.
- No hosted import of arbitrary vault content.
- No cross-repo implementation before the technical spec gate is clean.

## Resolved Build-Gate Decisions

### Auth Ownership

`duumbi-loop` owns the `AuthAdapter` boundary for this slice.
`auth.duumbi.dev` is reserved as a future central SSO boundary, not a separate
required service yet.

The product requirement is replaceability: organization, membership, billing,
run, and audit semantics must not depend on the temporary auth implementation.

### Database Ownership

Postgres is the production data model target for Loop metadata. Neon Postgres is
approved for non-prod hosted smoke if credentials and budget are available.
Local development and CI may use a local Postgres test database.

The production-integration path must not extend the in-memory store or SQLite as
the hosted persistence path.

### Billing Ownership

Stripe test mode is the only billing provider in this slice.

Products:

- `duumbi_loop_starter_test`
- `duumbi_loop_team_test`

Credit unit: one DUUMBI internal billable usage unit. Test mode may treat 1
credit = USD 0.01 for reconciliation only.

Initial test plans:

| Plan | Seats | Repositories | Parallel runs | Monthly credits | Max credits per run |
| --- | ---: | ---: | ---: | ---: | ---: |
| Starter | 1 | 3 | 1 | 1000 | 25 |
| Team | 10 | 25 | 3 | 10000 | 100 |

Entitlement keys:

- `seat_limit`
- `repository_limit`
- `parallel_run_limit`
- `included_monthly_credits`
- `purchased_credits`
- `max_credits_per_run`
- `allowed_model_labels`
- `retention_days`
- `byok_allowed`
- `platform_keys_allowed`

### Azure Staging Boundary

Approved non-prod resources:

- `rg-duumbi-loop-staging`
- `cae-duumbi-loop-staging`
- `ca-duumbi-loop-web-staging`
- `ca-duumbi-loop-worker-staging`
- `stduumbiloopstaging`
- `kv-duumbi-loop-staging`
- `log-duumbi-loop-staging`
- `staging.loop.duumbi.dev`

Budget cap: USD 20/month for non-prod Loop, with alerts at 50/80/100 percent.

Scale-to-zero is required where possible. Staging max replicas is 1. The worker
must be disabled unless an explicit E2E queue test needs it. Hosted smoke must
not create live provider/model spend without separate approval.

### Cross-Repo PR Order

1. `hgahub/duumbi`: spec artifacts and core contract updates only if required.
2. `hgahub/duumbi-loop`: primary app/API/AuthAdapter/Postgres/billing
   mirror/worker implementation.
3. `hgahub/duumbi-web`: public Loop route using duumbi.dev visual language.
4. `hgahub/duumbi-infra`: Azure staging resources after app interfaces
   stabilize.
5. `hgahub/duumbi-registry`: read-only unless registry metadata API boundary is
   required.
6. `hgahub/duumbi-vault`: curated references/docs only, not runtime source of
   truth.

### Hosted Vault Import Policy

Only curated allowlisted vault references may be imported. The hosted path must
exclude Inbox, personal notes, secrets, credentials, raw attachments, and
unreviewed drafts.

Every imported item must carry provenance, source path, approval status, import
time, importer identity, and retention policy.

### Model Routing Policy

Users choose DUUMBI-owned labels:

- Fast
- Balanced
- Deep Research
- Strict Review
- Private/BYOK

DUUMBI decides provider/model routing behind those labels. Hosted smoke uses a
deterministic no-spend/mock router. Raw provider/model SKUs are admin/audit
metadata only, not the user-facing product contract.

`platform_keys_allowed` remains false until live-spend approval exists.
Private/BYOK may be represented in policy, but it is not required for hosted
smoke.

## Product Surfaces In This Slice

### Public Loop Entry Point

The public route should give users a clear first impression of DUUMBI Loop while
remaining bounded. It is not a full marketing launch.

Required content:

- DUUMBI Loop as the first-viewport product signal.
- Native workflow statement: intent -> intake -> spec -> review.
- Explicit "GitHub/GitLab are optional adapters" copy.
- DUUMBI-owned model labels.
- Security/privacy and staging/access expectation.
- Sign-in or request-access path to the staging app.

Visual requirement:

- Use the duumbi.dev token system from `duumbi-web`.
- Public pages may use the parchment/ink editorial theme.
- Authenticated app surfaces should keep the darker operational Loop style.

### Auth And Organization Entry

The user must be able to sign in through a test/staging `AuthAdapter` and land
in a persisted organization context.

Required behavior:

- session creation through `duumbi-loop`,
- secure, HttpOnly cookie behavior,
- organization lookup from Postgres,
- membership and role checks,
- account/session/token/export/delete boundaries retained from #750,
- adapter contract documented for future `auth.duumbi.dev` replacement.

### Dashboard And Run Preflight

The dashboard must prove production-shaped state, not visual completeness.

Required behavior:

- persisted run list and state summary,
- persisted entitlement summary,
- credit balance and max credits per run,
- provider-empty state that keeps native Loop available,
- native run preflight with no Git provider credentials,
- over-cap or exhausted-credit block before queue/write,
- no-spend model router evidence.

### Billing And Entitlement

Stripe test-mode webhook events must materialize entitlements in Postgres.

Required behavior:

- Starter and Team test plans are represented,
- webhook signature and idempotency behavior remain enforced,
- entitlements are read from the materialized mirror during request handling,
- credit ledger records grants, debits, and adjustments,
- run preflight uses credit balance, plan limits, billing state, and role
  permissions before queueing.

### Hosted Vault Knowledge Import

The product must support a controlled import path for curated vault references.

Required behavior:

- import only from an allowlist,
- exclude forbidden paths and unapproved content,
- show provenance for imported knowledge,
- make imported knowledge available to intake/review only after approval,
- audit import actions.

### Azure Staging And Cost Controls

The staging deployment must be testable and cheap.

Required behavior:

- approved resource names,
- budget and alerts,
- scale-to-zero where possible,
- max replicas 1,
- worker disabled unless needed for explicit E2E,
- no live model spend,
- evidence file proving resources, budget, and teardown/scale state.

## BDD Scenarios

### Scenario: Public Loop entry point uses duumbi.dev visual language

Given a visitor opens the staging Loop entry point
When the page renders
Then DUUMBI Loop is the first-viewport product signal
And the page uses the duumbi.dev visual token system
And the page describes the native intent -> intake -> spec -> review workflow.

### Scenario: Git providers remain optional on public copy

Given a visitor reads the public Loop page
When they reach adapter messaging
Then GitHub and GitLab are described as optional adapters
And the native DUUMBI path is presented as available without Git provider
credentials.

### Scenario: Staging login uses the AuthAdapter boundary

Given a staging user opens the Loop app
When they sign in through the configured test auth adapter
Then `duumbi-loop` creates a secure session
And the user lands in an organization context loaded from Postgres
And no central `auth.duumbi.dev` implementation is required for this slice.

### Scenario: Auth boundary can migrate to auth.duumbi.dev

Given future central SSO is introduced
When the auth adapter changes issuer behavior
Then organization, membership, billing, run, artifact, and audit ownership
remain unchanged.

### Scenario: Dashboard state persists across restart

Given an organization has seeded runs and entitlements
When the `duumbi-loop` process restarts
Then the dashboard still shows the organization, runs, memberships, and
entitlement summary from Postgres.

### Scenario: Stripe Starter webhook creates entitlements

Given Stripe sends a signed test-mode Starter subscription event
When the webhook is accepted
Then the entitlement mirror records `duumbi_loop_starter_test`
And the organization receives 1 seat, 3 repositories, 1 parallel run, 1000
monthly credits, and 25 max credits per run.

### Scenario: Stripe Team webhook creates entitlements

Given Stripe sends a signed test-mode Team subscription event
When the webhook is accepted
Then the entitlement mirror records `duumbi_loop_team_test`
And the organization receives 10 seats, 25 repositories, 3 parallel runs, 10000
monthly credits, and 100 max credits per run.

### Scenario: Stripe webhook is idempotent

Given a signed Stripe test event was already processed
When the same event is delivered again
Then the entitlement mirror is not double-applied
And the event is recorded as a replay or duplicate without changing credit
balance twice.

### Scenario: Run preflight blocks over max credits

Given an organization has Starter entitlement
When a user requests a run estimated above 25 credits
Then the request fails closed before queueing
And no run, worker job, credit debit, or external model call is created.

### Scenario: Native run can preflight without Git provider credentials

Given an organization has no GitHub or GitLab provider connection
When a developer creates a native DUUMBI Loop run
Then the system resolves entitlement, role, model label, and repository/workspace
state
And the preflight does not require Git provider credentials, issues, PRs, MRs,
or merge commits.

### Scenario: No-spend model routing hides provider SKUs

Given a user selects the Balanced DUUMBI model label
When staging preflight resolves model routing
Then the user-facing response shows Balanced
And raw provider/model SKU data is absent from the user UI
And admin/audit metadata records the deterministic no-spend router path.

### Scenario: Platform-key routing is disabled for live spend

Given `platform_keys_allowed` is false
When a staging run would require live provider/model spend
Then the run is blocked before execution
And the block reason explains that live spend is not approved for this slice.

### Scenario: Curated vault import accepts allowlisted content

Given an approved vault reference is on the hosted import allowlist
When an authorized user imports it
Then the knowledge entry is created with provenance, approval status, source
path, importer identity, and import timestamp.

### Scenario: Hosted vault import excludes unsafe content

Given a vault path points to Inbox, personal notes, secrets, credentials, raw
attachments, or unreviewed drafts
When a hosted import is requested
Then the import is rejected
And no knowledge entry is made usable for intake or review.

### Scenario: Azure staging route is reachable within budget policy

Given the staging resources are deployed
When an operator opens `staging.loop.duumbi.dev`
Then the public route and app health endpoint are reachable over HTTPS
And budget, alert, max-replica, and scale-to-zero evidence exists.

### Scenario: Worker is disabled outside explicit E2E

Given no explicit staging E2E queue test is running
When staging is idle
Then the worker is disabled or scaled to zero
And no background job creates cloud or model spend.

### Scenario: Hosted smoke runs with no live model spend

Given an operator starts the hosted smoke E2E
When a native no-Git fixture run is requested
Then the run uses the deterministic no-spend router
And evidence records external LLM calls = 0 and live provider/model cost = 0.

### Scenario: Cross-repo PR order is visible

Given implementation starts after this spec
When maintainers review the plan
Then the work is ordered through `duumbi`, `duumbi-loop`, `duumbi-web`,
`duumbi-infra`, `duumbi-registry`, and `duumbi-vault` according to this spec
And no repo takes ownership outside its boundary.

## Acceptance Criteria

- Product spec is in English and includes BDD scenarios.
- The spec explicitly states that DUUMBI Loop is not complete.
- The slice is bounded to staging production integration.
- Auth ownership, database ownership, billing values, Azure resources,
  cross-repo order, vault import policy, and model routing policy are recorded.
- GitHub and GitLab remain optional adapters.
- `provider-duumbi` remains the primary path.
- DUUMBI-owned model labels remain the user-facing contract.
- Security, privacy, billing, subscription, and cloud-cost constraints are
  visible before implementation.
- No implementation code, Ralph cycles, or Greptile review are created by this
  spec PR.
