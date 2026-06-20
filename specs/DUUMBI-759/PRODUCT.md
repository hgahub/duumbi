# DUUMBI-759: DUUMBI Loop Public Web And Azure Staging Slice

Spec for #759.

Related to #738, #750, and #757.

This PR is specification-only and must leave #759 open. Do not use closing
references such as "closes", "fixes", or "resolves" for #759, #757, #750, or
#738.

## Summary

Define the next bounded DUUMBI Loop slice after:

- #738 delivered the provider-core/native CLI foundation in `hgahub/duumbi`.
- #750 and `hgahub/duumbi-loop` PR #1 delivered the first local/no-cost
  Loop web/API scaffold.
- #757 and `hgahub/duumbi-loop` PR #2 delivered the first
  production-integration `duumbi-loop` slice with AuthAdapter, Postgres
  persistence, Stripe test entitlement mirror, no-spend model routing, native
  no-provider preflight, curated vault import boundary, and local Postgres
  evidence.

This slice connects the production-shaped Loop app foundation to the public
`duumbi.dev` experience and a constrained Azure staging boundary. It does not
complete the full DUUMBI Loop product.

## Product Outcome

A visitor can find DUUMBI Loop from a public `duumbi.dev` route, understand the
native intent -> intake -> spec -> review workflow, and follow a clear CTA into
the staging Loop app. A DUUMBI operator can deploy a low-cost staging boundary
that proves the public route, Loop app health/readiness, and operational
guardrails without enabling production auth, live Stripe, live provider/model
spend, real Git provider credentials, or worker execution outside explicit E2E.

## Source Context

Verified during spec drafting:

- `hgahub/duumbi` contains prior specs in `specs/DUUMBI-738`,
  `specs/DUUMBI-750`, and `specs/DUUMBI-757`.
- `hgahub/duumbi-loop` PR #2 is merged at
  `7e639f427742a17e29e9f3b384058cfe223b52a3`.
- `duumbi-loop` exposes `GET /health`, `GET /ready`, `GET /ops/e2e-evidence`,
  local login, dashboard, run creation, Stripe test webhook, model route
  decisions, and curated vault import endpoints.
- `duumbi-loop/docs/e2e/duumbi-757-local-postgres-e2e.md` records local
  evidence with external LLM calls = 0, Git provider credentials = 0, hosted
  cloud resources = 0, live Stripe calls = 0, live provider/model spend = 0,
  production auth dependencies = 0, and Ralph cycles = 0.
- `duumbi-web` is an Astro/Tailwind public site using the current duumbi.dev
  token system from `src/styles/global.css`.
- `duumbi-infra` is a TypeScript Pulumi/Azure Native repo with existing
  patterns for resource groups, DNS, Container Apps, Key Vault, Log Analytics,
  storage, alerts, tags, and a USD 20 project budget.
- Vault Loop planning docs remain reference material only. They are not runtime
  source of truth for this slice.

## Scope

This product slice covers:

- a public Loop entry route in `hgahub/duumbi-web`,
- public copy and visual design using the duumbi.dev visual language,
- a CTA into the Loop app or request-access path without implying general
  availability,
- Azure staging resource definitions in `hgahub/duumbi-infra`,
- a minimal `duumbi-loop` staging deploy contract only where the app must expose
  env vars, health/readiness, or no-spend smoke evidence,
- hosted smoke acceptance criteria for `staging.loop.duumbi.dev`,
- explicit budget, scale, teardown, secret, and no-spend guardrails.

## Non-Goals

- No implementation code in this spec PR.
- No Ralph cycles in this spec PR.
- No Greptile review for this spec PR.
- No full DUUMBI Loop launch.
- No hosted production auth or `auth.duumbi.dev` implementation.
- No live Stripe products or checkout.
- No live provider/model spend.
- No real GitHub/GitLab adapter requirement.
- No worker execution except an explicitly approved E2E queue smoke.
- No authenticated dashboard product polish beyond deploy-contract needs.
- No change to DUUMBI core unless implementation discovers a hard contract gap.
- No use of arbitrary vault content, personal notes, raw attachments, secrets,
  credentials, Inbox, or unreviewed drafts.

## Product Surfaces

### Public Loop Entry Route

Owner: `hgahub/duumbi-web`.

Route target: implementation may choose `/loop` on `duumbi.dev` first, with DNS
or redirect strategy for `loop.duumbi.dev` deferred to the infra PR if needed.

The page must:

- make "DUUMBI Loop" the first-viewport product signal,
- describe the DUUMBI-native workflow as intent -> intake -> spec -> review,
- show that GitHub and GitLab are optional adapters, not prerequisites,
- explain DUUMBI-owned model labels without exposing provider/model SKUs,
- show security/privacy guardrails at a public-page level,
- avoid claiming that full dashboard, billing, production auth, or hosted cloud
  workflow execution is complete,
- provide a CTA to "Request access" or "Open staging app" depending on the
  environment,
- reuse existing duumbi.dev visual tokens and page conventions.

The page should feel like a public product route, not a marketing-only
placeholder. It should be visually consistent with the current DUUMBI site and
must not introduce a separate brand system.

### Staging Loop App Entry

Owner: `hgahub/duumbi-loop`.

The staging app must expose:

- `GET /health` for liveness,
- `GET /ready` for readiness and backend/auth metadata,
- `GET /ops/e2e-evidence` for no-spend smoke evidence,
- a safe CTA landing path from the public page,
- deterministic no-spend/mock model routing,
- no requirement for GitHub/GitLab credentials.

The staging app may continue to use the local/test auth adapter for this slice.
Production auth remains out of scope.

### Azure Staging Boundary

Owner: `hgahub/duumbi-infra`.

Approved resource names:

- `rg-duumbi-loop-staging`
- `cae-duumbi-loop-staging`
- `ca-duumbi-loop-web-staging`
- `ca-duumbi-loop-worker-staging`
- `stduumbiloopstaging`
- `kv-duumbi-loop-staging`
- `log-duumbi-loop-staging`
- `staging.loop.duumbi.dev`

Budget policy:

- USD 20/month non-prod Loop budget.
- Alerts at 50/80/100 percent.
- Because budget alerts do not stop spend, the implementation must document and
  verify a 100 percent disable, teardown, or scale-to-zero runbook.

Scale policy:

- max replicas = 1 in staging,
- scale-to-zero where Azure supports it,
- worker disabled by default,
- worker enabled only for explicit E2E queue work and disabled again afterward.

## User-Facing Copy Requirements

The public route must be plain about maturity:

- "DUUMBI Loop is being staged" is acceptable.
- "Available now for all teams" is not acceptable.
- "GitHub/GitLab adapters are optional" must be clear.
- "No live provider/model spend in staging" must be true in evidence, not
  public marketing copy unless it helps explain privacy/cost behavior.

The model-label explanation must use DUUMBI-owned labels:

- Fast
- Balanced
- Deep Research
- Strict Review
- Private/BYOK

Raw provider/model SKUs must not be presented as user-facing choices.

## BDD Scenarios

### Scenario: Public Loop page is reachable and branded

Given a visitor opens the DUUMBI public Loop route
When the page loads
Then "DUUMBI Loop" is visible in the first viewport
And the page uses the duumbi.dev visual token system
And the page presents Loop as a DUUMBI-native workflow product.

### Scenario: Public copy does not overclaim completion

Given a visitor reads the public Loop page
When they review the CTA and product copy
Then the copy does not claim the full DUUMBI Loop product is complete
And the copy does not claim production auth, live billing, hosted workflow
execution, or Git adapters are generally available.

### Scenario: Native workflow is primary

Given a visitor reads the public Loop page
When the workflow is described
Then the page explains intent, intake, spec, review, artifacts, and knowledge
context
And GitHub/GitLab are described only as optional adapters.

### Scenario: DUUMBI-owned model labels are the public contract

Given a visitor reads model-related copy
When model choices are shown
Then the page shows DUUMBI-owned model labels
And raw provider/model SKUs are not exposed as product choices.

### Scenario: Public CTA reaches the staging boundary safely

Given a visitor clicks the Loop CTA in a staging environment
When the target opens
Then the target is either a request-access flow or the staging Loop app entry
And the target does not require GitHub/GitLab credentials
And the target does not start a live provider/model-spend workflow.

### Scenario: Staging app exposes health and evidence

Given the Azure staging app is deployed
When an operator checks `/health`, `/ready`, and `/ops/e2e-evidence`
Then health is successful
And readiness identifies the staging auth/persistence boundary
And evidence reports external LLM calls = 0, Git provider credentials = 0, live
Stripe calls = 0, live provider/model spend = 0, hosted cloud worker execution
outside E2E = 0, and Ralph cycles = 0.

### Scenario: Azure staging resources follow approved names

Given the infra PR is planned
When the operator previews or applies the staging stack
Then only approved Loop staging resource names are introduced
And resources are tagged consistently with existing DUUMBI Pulumi tags.

### Scenario: Budget controls are visible before hosted smoke

Given a hosted smoke is requested
When the infra plan is reviewed
Then the USD 20/month budget and 50/80/100 percent alerts exist
And the 100 percent disable/teardown policy is documented
And hosted smoke does not proceed without this evidence.

### Scenario: Worker remains disabled by default

Given no explicit E2E queue test is running
When staging resources are inspected
Then the Loop worker is disabled or scaled to zero
And max replicas do not exceed 1.

### Scenario: Hosted smoke has no live spend

Given hosted staging smoke runs
When evidence is collected
Then live provider/model spend is 0
And live Stripe calls are 0
And Git provider credentials are 0
And the model router is deterministic no-spend/mock.

## Acceptance Criteria

- Product spec is in English and includes BDD scenarios.
- Technical spec maps every BDD scenario to tests or evidence.
- Cross-repo ownership is explicit for `duumbi-web`, `duumbi-infra`, and
  `duumbi-loop`.
- Public route scope does not claim full product completion.
- Azure resources, budget cap, alerts, scale limits, worker default-disabled
  behavior, and teardown policy are explicit.
- No production auth, live Stripe, live model spend, or Git provider
  credentials are required.
- Stage 10 prompt is included in the technical spec.

## Stage 7 Product Gate Decision

Gate decision: pass.

Reasoning:

- Scope is bounded to public web entry, Azure staging, and minimal Loop deploy
  contract.
- BDD scenarios cover user-facing behavior, guardrails, and non-goals.
- The spec explicitly states that full DUUMBI Loop completion is out of scope.
- No unresolved product blocker remains for spec drafting.
