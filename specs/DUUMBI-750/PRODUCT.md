# DUUMBI-750: DUUMBI Loop Web And Infrastructure Experience Slice

Spec for #750.

Related to #738. Builds on the merged provider-core/native CLI foundation in
#749.

This PR is specification-only and must leave #750 open. Do not use closing
references such as "closes", "fixes", or "resolves" for #750 or #738.

## Summary

Define the next DUUMBI Loop product slice: the web, account, dashboard,
repository, knowledge, configuration, intake, review, model-label, billing, and
infrastructure experience that sits above the DUUMBI-native provider foundation
delivered by #749.

#749 proved that DUUMBI Loop can run native intake/spec work from DUUMBI intents,
local workspace graph context, BDD artifacts, knowledge records, registry
metadata, snapshots, and GraphPatch review targets without GitHub or GitLab
credentials. This spec does not replace that foundation. It makes the next
product surface explicit so later implementation can connect a user-facing Loop
experience to the native provider contract without turning Git providers back
into prerequisites.

## Product Outcome

A user can visit a DUUMBI Loop web experience, sign in, create or enter an
organization, see their running Loop work, understand subscription and credit
usage, register optional Git providers and repositories, manage knowledge and
configuration, start an intake flow with research and knowledge-base context,
and review generated artifacts or graph changes.

The user sees DUUMBI-owned model labels such as `Fast`, `Balanced`,
`Deep Research`, and `Strict Review`. DUUMBI decides internally which provider
and model satisfy those labels for the workflow, cost policy, region policy, and
data policy in effect at run time.

## Current Source Context

Verified during spec drafting:

- #749 is merged into `hgahub/duumbi` and added `duumbi::loop_native` plus
  `duumbi loop intake-spec` and `duumbi loop review-patch`.
- The #749 implementation records local E2E evidence in
  `docs/e2e/results/duumbi-738-native-loop-20260619.md`.
- `duumbi::loop_native` already defines provider-neutral vocabulary for
  work items, context sources, artifact envelopes, run states, review targets,
  GraphPatch review targets, and DUUMBI-owned model labels.
- `duumbi-loop` is currently effectively empty except for Git metadata and
  `.gitignore`, so it should be treated as a scaffold target.
- `duumbi-web` is an Astro 6, Tailwind CSS 4, and MDX public site/docs project.
  Its visual tokens define a light parchment/ink/blue/rust system and a dark
  DUUMBI green accent system.
- `duumbi-infra` is a TypeScript Pulumi project using Azure Native, Azure AD,
  Random, DNS, Container Apps, storage, Log Analytics, Key Vault-style secret
  handling, and registry hosting.
- `duumbi-registry` is a Rust/Axum registry with SQLite-backed module metadata,
  package storage, web templates, local password auth, GitHub OAuth, JWT
  sessions, device-code flow, and embedded app tests via `build_app()`.
- `duumbi-vault` contains the Loop cloud planning references. The current
  direction says DUUMBI-native workflow is primary, GitHub/GitLab are adapters,
  Stripe is the default billing provider, Neon Postgres is the planned metadata
  and graph default for the cloud slice, and central DUUMBI SSO is desired.

## Product Principles

- DUUMBI-native first: the core flow is intent -> intake -> spec -> review,
  backed by DUUMBI graph, knowledge, BDD, registry, session, and artifact state.
- Git providers are optional adapters: GitHub and GitLab can enrich workflows,
  but users must not need Git provider credentials to understand or start the
  native DUUMBI Loop flow.
- The web app is operational, not decorative: dashboard pages prioritize dense,
  scannable state, clear remediation, and repeated use.
- Cost is visible before spend: credit estimates, subscription limits, and BYOK
  routing implications are shown before a run starts.
- Knowledge context is explicit: intake and review pages show which knowledge
  sources were used, what was excluded, and what needs approval.
- Provider/model routing is internal: user-facing model choices are DUUMBI
  product labels with policy-backed routing behind the scenes.
- Privacy and tenant isolation are product features: secrets, provider tokens,
  raw source retention, prompt retention, and knowledge exports must be
  understandable from the UI.

## Goals

- Specify the public Loop homepage using the duumbi.dev visual language.
- Specify user management, login, organization membership, roles, and account
  lifecycle needs for the Loop app.
- Specify the app dashboard showing running tasks, workflow state, questions,
  review state, and subscription usage.
- Specify optional Git provider registration and repository registration.
- Specify knowledge base, configuration, intake, and review UI behavior.
- Specify DUUMBI-owned model labels and policy-backed provider routing.
- Specify product acceptance with BDD scenarios that the technical spec maps to
  tests.

## Non-Goals

- No implementation code in this spec slice.
- No Ralph cycles in this spec slice.
- No Greptile review for this spec PR.
- No claim that the full DUUMBI Loop product, website, dashboard,
  infrastructure, billing, or cross-repo implementation is complete.
- No requirement that GitHub or GitLab credentials exist before a user can use
  the native DUUMBI Loop path.
- No full production launch decision for all cloud infrastructure.
- No final pricing calibration. This spec defines constraints and starting
  behavior; launch pricing must be validated with measured cost data.

## Visual Language

Public Loop pages should extend the existing `duumbi-web` token system:

- Light theme:
  - parchment: `#f5f2ea`
  - ink: `#1a1a18`
  - blue ink: `#1d4a8c`
  - rust: `#8c3d1d`
- Dark app theme:
  - background: `#0e1014`
  - surface: `#15181d`
  - DUUMBI green accent: `#86c06a`
  - DUUMBI green hover/accent: `#97cf7d`

Homepage and documentation pages may keep the editorial parchment/grid feel.
The authenticated Loop app should use the dark operational theme with compact
tables, status badges, side navigation, and graph-aware workflow visualizations.
Color must never be the only status indicator.

## Roles

- Owner: manages organization, billing, members, data export, deletion, provider
  connections, and model/data policy.
- Admin: manages providers, repositories, configuration, members except owner
  transfer, and workflow remediation.
- Developer: starts intake/spec work, answers questions, views artifacts, and
  creates or edits knowledge entries.
- Reviewer: runs and reviews review flows, answers review questions, and views
  artifacts.
- Billing admin: manages subscription, invoices, credit policy, and payment
  state without access to source artifacts by default.
- Viewer: reads dashboards, artifacts, and knowledge entries.
- Staff: DUUMBI internal support role with audited, read-only "view as org" and
  narrowly approved entitlement or suspension actions.

## Product Surfaces

### Loop Homepage

Route target: `https://loop.duumbi.dev`.

The homepage must present DUUMBI Loop as a DUUMBI-native workflow product, not
as only GitHub/GitLab automation.

Required sections:

- Hero: "DUUMBI Loop" as the first-viewport product signal and a direct
  description of the intent -> intake -> spec -> review workflow.
- Native workflow: shows DUUMBI intents, graph context, BDD artifacts,
  knowledge, and GraphPatch/snapshot review targets.
- Optional adapters: explains GitHub and GitLab as adapters for teams that want
  provider comments, PRs, and MRs.
- Knowledge and research: explains how intake uses organization knowledge,
  repository knowledge, graph summaries, and source evidence.
- Model labels: explains DUUMBI-owned labels without exposing provider SKU
  names as the product contract.
- Security and privacy: highlights tenant isolation, token storage, retention,
  no full repo dumps to LLM providers, and region/model policy.
- Pricing preview: explains seats, credits, BYOK behavior, usage visibility,
  and credit limits without promising final launch prices.
- CTA: sign in or request access to the Loop app.

### User Management And Login

Users must be able to:

- sign up and log in with email magic link,
- link supported identities after login,
- log out and revoke sessions,
- join an organization by invitation,
- create an organization when they do not belong to one,
- switch organizations,
- view and edit account profile data,
- export their account data,
- request account deletion.

The product direction prefers central DUUMBI SSO at `auth.duumbi.dev`. If the
first implementation slice cannot deliver that service, it must still define an
auth boundary that can migrate to central SSO without rewriting Loop domain
objects.

### Application Dashboard

Route target: `https://app.loop.duumbi.dev`.

The first authenticated screen must show operational state, not marketing.

Required dashboard content:

- running tasks by workflow type,
- status distribution by `queued`, `running`, `needs_input`, `needs_action`,
  `completed`, `failed`, `cancelled`, and `superseded`,
- "Waiting on you" section for open questions and permission/configuration
  remediation,
- recent intake, spec, review, and closure artifacts,
- subscription plan, included credits, purchased credits, credit burn,
  estimated next-run affordability, and hard blocks,
- enabled repository count and plan limit,
- knowledge candidates pending approval,
- provider health if optional adapters are connected,
- activation checklist for new organizations.

Dashboard state must distinguish:

- user input needed,
- admin or operator action needed,
- external provider permission problem,
- billing or credit block,
- platform failure,
- successful completion.

### Git Provider Registration

Git providers are optional adapters.

The UI must support:

- GitHub,
- GitHub Enterprise Server as a later or gated option,
- GitLab Cloud,
- GitLab Self-Hosted with manual webhook setup and SSRF protections.

Provider registration must show:

- connection status,
- last sync,
- connected organization/group count,
- enabled repository count,
- missing permission details,
- reconnect action,
- remove provider action with impact analysis,
- provider token/installation revocation handling.

The empty state must make clear that native DUUMBI Loop work can proceed without
connecting GitHub or GitLab.

### Repository Registration

Repositories are registered resources, not prerequisites for native Loop use.

The UI must support:

- enabling/disabling repositories from connected providers,
- manually registering a local/native DUUMBI workspace reference when no Git
  provider is connected,
- index status and graph snapshot status,
- last indexed time,
- language and graph summary,
- plan-limit checks,
- reindex action,
- per-repository retention override within plan limits,
- command allowlist for adapter-driven commands,
- `.duumbiignore` or equivalent source-exclusion status.

Repository states:

- disabled,
- queued,
- cloning,
- parsing,
- storing,
- enabled indexed,
- index failed,
- needs permissions,
- disabled by provider revoked,
- disabled by plan limit.

### Knowledge Base UI

The knowledge base must show durable organizational and repository knowledge
used by intake/spec/review flows.

Required behavior:

- browse entries by type, repository, status, source run, and update time,
- search entries,
- view provenance including run, issue/PR if any, graph snapshot, artifact, and
  source references,
- create manual entries,
- edit entries with audit trail,
- approve candidate entries,
- reject candidate entries with reason,
- deprecate entries without deleting history,
- export knowledge as part of organization export,
- show whether an intake/review used a knowledge entry.

Knowledge entry statuses:

- candidate,
- published,
- deprecated.

Knowledge entry types:

- decision,
- pattern,
- convention,
- API contract,
- gotcha,
- research note,
- review finding.

### Configuration UI

Configuration must be split between organization, account, repository, and
model/data policy.

Organization settings:

- general profile,
- members and invitations,
- billing,
- usage,
- model policy,
- region/data policy,
- retention,
- notifications,
- API tokens,
- audit log,
- data export,
- organization deletion.

Account settings:

- profile,
- linked identities,
- active sessions,
- personal tokens,
- notification preferences,
- account data export,
- account deletion.

Model/data policy:

- allowed DUUMBI model labels by workflow,
- blocked provider classes by region or data policy,
- BYOK enablement and validation,
- maximum estimated credits per run,
- maximum context size,
- prompt/source retention policy,
- fallback behavior when a provider/model is unavailable,
- "estimated" marker when BYOK pricing cannot be verified.

### Intake Flow

The intake flow must support DUUMBI-native and adapter-triggered starts.

Native start options:

- from dashboard: create intake from a typed intent,
- from dashboard: create intake from a selected workspace/repository,
- from DUUMBI CLI/artifact reference produced by provider-duumbi,
- from a saved question or knowledge item.

Adapter start options:

- GitHub or GitLab issue command when a provider is connected,
- future Slack or other thin-surface command as an adapter.

The intake page must show:

- executive summary,
- problem statement,
- user impact,
- business context,
- likely affected areas,
- graph and repository evidence,
- knowledge entries used,
- research notes,
- clarifying questions,
- confidence,
- business value,
- effort and time estimate,
- risk assessment,
- next actions,
- sources.

If clarification is needed, the run enters `needs_input` and creates question
topics. A spec run must not proceed as clean until required questions are
resolved or explicitly waived with audit evidence.

### Review Flow

The review flow must support:

- GraphPatch review targets,
- graph snapshot diffs,
- generated artifact diffs,
- GitHub/GitLab PR or MR diffs when adapters are connected.

The review page must show:

- review target type,
- source context,
- findings grouped by severity,
- spec acceptance criterion mapping,
- BDD coverage and missing tests,
- affected graph nodes or files,
- inline provider-comment status when an adapter exists,
- full dashboard findings even if provider inline-comment limits apply,
- final recommendation,
- confidence and residual risk.

Review severities:

- blocking,
- warning,
- nit,
- info.

Default behavior is comment mode even for blocking findings. Strict mode can be
enabled by policy or per repository when the provider supports blocking review.

### DUUMBI-Owned Model Labels

Users choose labels. DUUMBI resolves providers/models internally.

Initial product labels:

- Fast: lower latency, lower cost, bounded context.
- Balanced: default quality/cost path for normal intake/spec.
- Deep Research: higher context budget, more source triangulation, stricter
  evidence requirements.
- Strict Review: conservative review path for correctness, security, and spec
  compliance.
- Private/BYOK: route through organization-owned keys where configured.

The UI may show estimated credits and policy attributes, but it must not expose
raw provider/model IDs as stable primary choices. Raw provider details may be
available in audit/debug views for admins when needed.

## Billing, Subscription, And Cloud-Cost Constraints

The product must enforce spend controls before a workflow starts.

Required user-facing billing behavior:

- current plan,
- seat count,
- enabled repository limit,
- included credits,
- purchased credits,
- credit ledger,
- estimated credits before run start,
- final credits after run completion,
- BYOK orchestration fee marker,
- credit 80 percent warning,
- hard block at exhausted credits unless auto-recharge is enabled,
- billing inactive or dunning block,
- plan-limit remediation.

Required internal constraints:

- API and worker paths read entitlements from materialized entitlement state,
  not from live Stripe calls during request handling.
- Platform-key routing cannot select a model without a curated price.
- BYOK routing can proceed with conservative estimated credits if price is
  unknown and policy allows it.
- Cloud workers must have per-run and per-organization cost caps.
- Hosted E2E must use test-mode billing and explicit cloud budgets.

## Security And Privacy Requirements

- Tenant isolation applies to every organization-scoped API, artifact, graph,
  knowledge entry, run, provider connection, and billing record.
- Provider tokens, OAuth secrets, LLM API keys, webhook secrets, session secrets,
  and encryption keys must never be stored in plaintext in application tables or
  frontend environment variables.
- API tokens are stored hashed; raw values are shown once.
- Sessions use secure, HttpOnly cookies with rotation and revocation.
- GitLab Self-Hosted URLs require SSRF defense before connection tests.
- Raw source retention defaults must be visible and configurable within plan
  limits.
- Full repository dumps must not be sent to LLM providers.
- LLM context must be bounded, source-referenced, and policy-filtered.
- Analytics events must not contain source code, prompts, secrets, raw provider
  tokens, payment details, or personal contact data beyond pseudonymous IDs.
- Staff access must be audited and visibly marked.
- Data export and deletion flows must include account and organization scope.

## Acceptance Criteria

- Product spec is written in English and includes BDD scenarios.
- The scope explicitly includes homepage, login/user management, dashboard,
  optional Git provider registration, repository registration, knowledge base,
  configuration, intake, review, model labels, billing, and cloud-cost controls.
- The spec treats #749 as the provider-core/native CLI foundation.
- The spec does not claim the full DUUMBI Loop product is complete.
- GitHub and GitLab remain optional adapters.
- DUUMBI-owned model labels are the user-facing contract.
- Cross-repo ownership assumptions are passed to the technical spec.
- Security, privacy, billing, and cloud-cost constraints are visible before
  implementation.
- No implementation code or Ralph cycles are created by this spec slice.

## BDD Scenarios

### Scenario: Homepage presents DUUMBI-native Loop

Given a visitor opens the Loop homepage
When the first viewport and workflow sections render
Then the page presents DUUMBI Loop as intent -> intake -> spec -> review
And the page explains DUUMBI-native graph, BDD, knowledge, and artifact context
And GitHub/GitLab are described as optional adapters.

### Scenario: Login creates an organization session

Given a new user opens the Loop app
When they complete email magic-link login
Then they can create an organization
And the app creates a session scoped to that user and organization
And the user lands on the dashboard.

### Scenario: Invited user joins an existing organization

Given an admin invites a developer by email
When the developer accepts the invitation after login
Then the developer joins the organization with the invited role
And no new organization wizard is required.

### Scenario: Dashboard shows active Loop work

Given an organization has running and blocked Loop runs
When a member opens the dashboard
Then they see running tasks, workflow state, questions, remediation needs, and
recent artifacts
And `needs_input` and `needs_action` are distinguishable.

### Scenario: Dashboard shows subscription usage before spend

Given an organization has a subscription with credit limits
When a user opens the dashboard or starts a run
Then they see included credits, purchased credits, estimated run credits, and
hard blocks before work starts.

### Scenario: Native workflow can start without Git provider credentials

Given an organization has no GitHub or GitLab connection
When a developer starts intake from a DUUMBI intent or local workspace reference
Then the product starts a DUUMBI-native Loop run
And the run does not require a Git provider token, issue, PR, MR, or merge
commit.

### Scenario: Git provider connection is optional

Given an organization has not connected a Git provider
When an admin opens Code Providers
Then the empty state offers GitHub and GitLab connection
And it also states that native DUUMBI Loop work can continue without them.

### Scenario: Provider revocation disables dependent repositories

Given an organization has repositories enabled through a Git provider
When the provider installation is revoked
Then the provider status becomes revoked
And dependent repositories become disabled by provider revoked
And existing artifacts remain readable.

### Scenario: Repository registration triggers indexing status

Given an admin enables a repository
When the repository registration is accepted
Then the repository enters queued, indexing, or indexed state
And graph snapshot state is visible on the repository row.

### Scenario: Plan limit blocks repository enablement

Given an organization has reached its enabled repository limit
When an admin attempts to enable another repository
Then the action is blocked before indexing starts
And the UI shows the plan limit and upgrade or disable-remediation options.

### Scenario: Knowledge candidate approval updates usable context

Given a closure or review run creates a candidate knowledge entry
When a developer approves the candidate
Then the entry becomes published
And future intake/review flows can cite it as a knowledge source.

### Scenario: Configuration controls model and data policy

Given an owner opens model and data policy settings
When they set allowed labels, region policy, retention, and maximum credits per
run
Then future runs enforce those settings before provider/model routing.

### Scenario: Intake uses research and knowledge context

Given a developer starts intake for a DUUMBI-native work item
When relevant knowledge and graph sources exist
Then the intake artifact shows research notes, knowledge sources, graph sources,
clarifying questions, and source references.

### Scenario: Intake blocks on unresolved required questions

Given an intake run generates required question topics
When a developer attempts to continue to spec without resolving or waiving them
Then the workflow remains `needs_input`
And the unresolved topics are shown in the dashboard and run detail.

### Scenario: Review supports GraphPatch target

Given a DUUMBI-native run has a GraphPatch review target
When a reviewer opens the review page
Then they see affected graph nodes, source context, findings, severity,
acceptance-criterion mapping, and final recommendation.

### Scenario: Review supports optional Git adapter diff

Given a GitHub or GitLab adapter is connected
When a review run targets a PR or MR diff
Then the dashboard shows the full findings list
And the provider shows inline comments only where adapter limits and permissions
allow.

### Scenario: DUUMBI model labels hide provider routing

Given a user selects the `Balanced` model label for spec generation
When the run starts
Then the UI records the DUUMBI label and estimated credits
And it does not require the user to choose a raw provider/model SKU.

### Scenario: Cloud-cost policy fails closed

Given a run estimate exceeds the organization's maximum credits per run
When the user attempts to start the run
Then the run is not queued
And the UI shows the estimate, configured cap, and remediation path.

### Scenario: Staff access is audited

Given a staff user opens an organization in support mode
When they view organization state
Then the UI shows a support banner
And an audit event records the staff access.

## Build-Gate Decisions Required Before Implementation

The following are not blockers to this spec, but they must be resolved or
explicitly stubbed before an implementation PR can claim a hosted web+infra
slice:

- Whether central `auth.duumbi.dev` is implemented in a separate repo for this
  slice or represented by a stable interface and local test stub.
- Whether the first deployable app surface lives in `duumbi-loop` with
  `duumbi-web` owning only public pages, or whether `duumbi-web` temporarily
  hosts Loop public pages too.
- Whether Neon Postgres is approved for first hosted metadata and graph storage
  in the build slice.
- Which Azure resources can be created in `duumbi-infra` for hosted E2E.
- Which Stripe test-mode products, entitlements, and credit values are accepted
  for the first billing shell.
- Which LLM providers are allowed for platform-key routing in staging and which
  BYOK providers are allowed in local E2E.
- Which vault knowledge sources may be imported into hosted environments.

