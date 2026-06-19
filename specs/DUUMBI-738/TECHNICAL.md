# DUUMBI-738: DUUMBI Loop Native Workflow Adaptation - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-738/PRODUCT.md` by making
the DUUMBI-native workflow the primary Loop provider path.

The implementation must establish this contract:

```text
DUUMBI intent/session event -> provider-core WorkItem ->
native intake/spec artifacts -> GraphPatch or graph snapshot review target ->
approved patch/artifact closure event
```

GitHub and GitLab remain provider adapters. They must map into the same
provider-core objects instead of defining the core object model.

The first Stage 10 slice must prove DUUMBI-native intake+spec end-to-end
against a local/CLI workspace and graph-aware `duumbi-registry` context without
requiring GitHub or GitLab credentials. Broader cloud dashboard, Git adapter,
and closure automation work can follow only after the native provider contract
is working and testable.

Related to #738. This is a technical-specification artifact only. The execution
issue must remain open for Stage 10 implementation, Stage 11 review, and Stage
12 closure.

## Agent Audience

- Codex App implementation agents running bounded local Ralph cycles.
- Codex CLI or Cloud agents used for focused cross-repo implementation.
- Stage 10 coordinator agents sequencing work across `duumbi`, `duumbi-loop`,
  `duumbi-web`, `duumbi-infra`, `duumbi-registry`, and `duumbi-vault`.
- Reviewer agents validating provider-core boundaries, BDD-to-test coverage,
  security/privacy, resource policy, and live E2E evidence.
- Human maintainers reviewing product scope and cross-repo rollout risk.

Do not use this spec to start implementation before Stage 9 approval and Ready
for Build routing.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/738
- Product spec: `specs/DUUMBI-738/PRODUCT.md`
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/738#issuecomment-4752179082
- Loop reference docs:
  - `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/02 Resources (Assets and Tools)/Sources (References)/duumbi-loop-codex-task.md`
  - `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/02 Resources (Assets and Tools)/Sources (References)/duumbi-loop-codex-task-part2.md`
- Repo instructions:
  - `/Users/heizergabor/space/hgahub/duumbi/AGENTS.md`
  - `/Users/heizergabor/space/hgahub/duumbi-registry/AGENTS.md`
- Existing BDD precedent: `specs/DUUMBI-673/PRODUCT.md` and
  `specs/DUUMBI-673/TECHNICAL.md`

Verified issue facts:

- #738 is open.
- #738 has labels `accepted` and `needs-spec`.
- Stage 5 accepted the issue on 2026-06-19.
- Stage 5 records no remaining open questions.
- The issue requires `provider-duumbi` as the primary implementation of
  provider-core.
- The issue keeps GitHub/GitLab as optional adapters.
- The issue requires DUUMBI-native intake+spec MVP against a local/CLI
  workspace and graph-aware registry.

Verified `hgahub/duumbi` source facts:

- `src/intent/spec.rs`
  - `IntentSpec` stores intent text, lifecycle status, acceptance criteria,
    module hints, i64 verifier test cases, dependencies, BDD references,
    optional context, created timestamp, and execution metadata.
  - `IntentBdd` is defaulted and links `.feature` artifacts.
- `src/intent/bdd.rs`
  - Provides BDD readiness, linked feature parsing, coverage classification,
    bounded rendering, and prompt-context formatting.
- `src/intent/create.rs`
  - Creates intent YAML and BDD artifacts through shared create flows.
- `src/intent/execute.rs`
  - Runs provider-free preflight before mutation side effects.
  - Loads BDD context, marks intent in progress, snapshots graph state,
    decomposes tasks, calls provider mutation, writes graph files, and runs
    verifier tests.
- `src/workflow.rs`
  - Exposes shared `create_intent`, `execute_intent`, `graph_evidence`,
    `build_workspace`, and `run_workspace` functions used by CLI and Studio.
- `src/session/mod.rs`
  - Persists session state under `.duumbi/session/current.json` and
    `.duumbi/session/history`.
  - Tracks conversation turns and usage statistics.
- `src/patch.rs`
  - Defines `GraphPatch` and atomic `PatchOp` operations applied to JSON-LD.
- `src/snapshot.rs`
  - Provides graph snapshot support for rollback/evidence paths.
- `src/knowledge/store.rs`
  - Stores file-backed knowledge nodes under `.duumbi/knowledge`.
- `src/interaction/router.rs`
  - Classifies requests into Query, Agent, Intent, or Unknown request shapes.
- `src/agents/model_catalog.rs`
  - Contains provider/model catalog and routing precedent.

Verified `hgahub/duumbi-web` facts:

- The stack is Astro 6, Tailwind CSS 4, MDX, sitemap, and RSS.
- The public site defines brand tokens in `src/styles/global.css`.
- Light theme uses parchment/ink/blue/rust tokens.
- Dark theme uses DUUMBI green accent tokens around `#86c06a` and `#97cf7d`.
- Existing pages are marketing/docs oriented; there is no Loop app dashboard in
  the inspected source tree.

Verified `hgahub/duumbi-registry` facts:

- Rust/Axum registry server with library `build_app()` for embedded tests.
- SQLite database layer, filesystem package storage, web frontend, and REST API.
- Auth supports local username/password and GitHub OAuth.
- Device-code flow exists for CLI login in GitHub OAuth mode.

Verified `hgahub/duumbi-infra` facts:

- TypeScript Pulumi project using Azure Native, Azure AD, and Random providers.
- Existing stacks include persistent, platform, registry, and related Azure
  resources.

Verified `hgahub/duumbi-loop` fact:

- The local checkout is effectively empty aside from Git metadata and
  `.gitignore`. Stage 10 should treat it as a likely scaffold target, not an
  existing implementation base.

## Affected Repositories

Stage 10 will likely require multiple PRs. The Stage 10 coordinator must keep
each PR focused and must keep #738 open until closure.

### `hgahub/duumbi`

Expected areas:

- Native provider primitives that reuse existing intent, session, graph,
  BDD, snapshot, knowledge, and registry-client modules.
- Shared artifact schemas if the core DUUMBI crate should own canonical Rust
  types.
- CLI local MVP entry points if the first native intake/spec proof belongs in
  the existing CLI.
- Tests proving native provider operation without GitHub/GitLab credentials.

Candidate files/modules:

- `src/intent/spec.rs`
- `src/intent/create.rs`
- `src/intent/review.rs`
- `src/intent/execute.rs`
- `src/workflow.rs`
- `src/session/mod.rs`
- `src/patch.rs`
- `src/snapshot.rs`
- `src/knowledge/*`
- `src/registry/*`
- `src/interaction/*`
- new focused module such as `src/loop_native/*` only if the contract clearly
  belongs in the DUUMBI core crate

### `hgahub/duumbi-loop`

Expected areas:

- Provider-core contract.
- `provider-duumbi`.
- Optional adapter interfaces for GitHub and GitLab.
- Loop run/artifact domain model.
- Worker/service orchestration for intake, spec, review, and closure.
- API contracts consumed by future dashboard pages.

Because the repository is currently sparse, Stage 10 must start with a minimal
scaffold and tests rather than a broad service rewrite.

Recommended package shape if no existing structure appears:

```text
crates/
  provider-core/
  provider-duumbi/
  provider-github/        # adapter only; may be stubbed behind traits in v1
  provider-gitlab/        # adapter only; may be stubbed behind traits in v1
  loop-artifacts/
  loop-worker/
  loop-api/
```

If Stage 10 finds a stronger existing structure, use it and document the
deviation in implementation evidence.

### `hgahub/duumbi-web`

Expected areas:

- Loop homepage route or site section.
- Public pricing/docs links if Loop pages live in this repo.
- Shared visual tokens for Loop public pages.
- Later app-shell/dashboard work only if Stage 10 has explicit writable access
  and scope for app pages.

Do not replace the Astro site with a new framework unless a separate accepted
architecture decision approves it.

### `hgahub/duumbi-registry`

Expected areas:

- Graph-aware registry metadata endpoints or client contracts needed by native
  intake/spec.
- Auth/session behavior only if the native MVP needs registry-backed user or
  project identity.
- Embedded e2e tests using `build_app()` and in-memory SQLite where possible.

### `hgahub/duumbi-infra`

Expected areas:

- Later Azure resources for Loop service, workers, dashboards, queues,
  scheduler, secrets, and monitoring.
- Stage 10 MVP should not require cloud deployment unless the approved slice is
  explicitly expanded.

### `hgahub/duumbi-vault`

Expected areas:

- Reference docs only. Do not make vault updates part of the required Stage 10
  implementation unless maintainers explicitly request planning-doc updates.

## Architecture

### Provider-Core Contract

Provider-core should express workflow concepts without Git-specific names:

```rust
pub trait LoopProvider {
    fn provider_kind(&self) -> ProviderKind;
    fn load_work_item(&self, id: &WorkItemId) -> Result<WorkItem>;
    fn load_context_index(&self, item: &WorkItem) -> Result<ContextIndex>;
    fn write_artifact(&self, item: &WorkItem, artifact: ArtifactEnvelope) -> Result<ArtifactRef>;
    fn load_review_target(&self, item: &WorkItem) -> Result<ReviewTarget>;
    fn record_review(&self, item: &WorkItem, review: ReviewArtifact) -> Result<ReviewRef>;
    fn record_closure(&self, item: &WorkItem, closure: ClosureEvent) -> Result<ClosureRef>;
}
```

Exact Rust names are not mandated. The invariant is that provider-core objects
must not be named after Git-only concepts such as `Issue`, `PullRequest`, or
`MergeCommit`.

Recommended core objects:

- `WorkItem`
- `WorkItemSource`
- `ContextIndex`
- `ArtifactEnvelope`
- `ArtifactRef`
- `ReviewTarget`
- `ChangeSet`
- `ClosureEvent`
- `ProviderCapability`
- `ProviderHealth`

### `provider-duumbi`

`provider-duumbi` maps native DUUMBI state to provider-core:

- `WorkItem`
  - `IntentSpec`
  - session-ledger request
  - explicit CLI/local request
- `ContextIndex`
  - `.duumbi/graph/**/*.jsonld`
  - `snapshot` metadata
  - `knowledge` entries
  - BDD feature files
  - registry module metadata
  - bounded source excerpts when necessary
- `ReviewTarget`
  - `GraphPatch`
  - graph snapshot diff
  - generated artifact diff
- `ClosureEvent`
  - approved patch application
  - accepted artifact state
  - graph validation/build/run evidence

`provider-duumbi` must not require:

- Git remote
- GitHub token
- GitLab token
- issue number
- PR/MR URL
- merge commit

### GitHub/GitLab Adapters

GitHub and GitLab adapters translate their provider objects into provider-core:

- issue/MR/PR command -> `WorkItem`
- repository/code index -> `ContextIndex`
- spec branch/PR -> `ArtifactRef`
- PR/MR diff -> `ReviewTarget`
- merge commit -> `ClosureEvent`

Adapter-specific capabilities should be explicit:

- can write provider comment
- can create branch
- can create PR/MR
- can read inline diff
- can post inline review
- can detect merge/closed state

The shared artifact schema must stay the same across native and Git-backed
paths.

## Data And Artifact Contracts

### Run State

Recommended run states:

- `queued`
- `running`
- `needs_input`
- `needs_action`
- `blocked`
- `failed`
- `cancelled`
- `completed`
- `superseded`

Required run fields:

- run id
- workflow type: intake, spec, review, closure
- provider kind: duumbi, github, gitlab
- source reference
- status
- user/org/workspace
- DUUMBI model label
- resolved provider/model audit fields when available
- estimated credits
- final credits
- confidence
- started/completed timestamps
- artifact refs
- source refs
- audit events

### Artifact Envelope

Use a schema-versioned envelope for all provider paths:

```json
{
  "schema_version": "duumbi.loop.artifact.v1",
  "artifact_kind": "intake|product_spec|technical_spec|bdd|review|closure",
  "provider_kind": "duumbi|github|gitlab",
  "work_item_id": "...",
  "run_id": "...",
  "created_at": "...",
  "sources": [],
  "body": {},
  "links": []
}
```

Markdown artifacts are still acceptable for human review, but the worker should
also retain structured metadata so dashboard and tests do not scrape markdown.

### Intake Artifact

Required fields:

- summary
- confidence
- business value
- effort
- affected areas with source refs
- knowledge citations
- graph/registry context refs
- questions
- recommended next action
- model label and credit estimate

### Spec Artifact

Required fields:

- product summary
- goals and non-goals
- acceptance criteria
- BDD scenario refs
- technical plan
- BDD-to-test mapping
- live E2E plan
- implementation resource policy

### Review Artifact

Required fields:

- status
- review target type
- blocking findings
- warnings
- missing BDD/test coverage
- acceptance-criteria mapping
- source refs
- posted provider comments where applicable

### Closure Artifact

Required fields:

- final outcome
- approved change reference
- graph snapshot before/after
- validation/build/run evidence
- knowledge updates
- follow-up items

## User-Facing Model Routing

Do not make raw provider/model SKU selection the primary user-facing model
control.

Recommended user labels:

- `fast`
- `balanced`
- `deep_analysis`
- `strict_review`
- `low_cost`
- `byok_balanced`

Recommended routing input:

```json
{
  "task": "intake|spec|review|closure|embedding|graph_reasoning",
  "duumbi_model_label": "balanced",
  "capabilities": ["reasoning", "code", "json_schema", "long_context"],
  "region": "EU|USA|China|global",
  "max_credits": 3,
  "provider_policy": {
    "allowed_providers": [],
    "blocked_providers": [],
    "byok": false
  }
}
```

Required invariants:

- The dashboard shows DUUMBI label, credit estimate, confidence, and evidence
  quality.
- Admin/audit views may show resolved provider/model after the run.
- Raw credential values must never be stored in run artifacts.
- Missing pricing for platform-key models must block routing or require
  conservative BYOK estimation.

## UI And Page Requirements

Implementation can split UI delivery across multiple PRs, but the architecture
must reserve these pages and contracts:

| Page | Required Evidence For Stage 10 |
| --- | --- |
| Loop homepage | Route or page plan using existing duumbi.dev visual tokens; if implemented, Astro build evidence. |
| Login/account | Auth state contract, roles, org membership, account recovery/deletion requirements. |
| Dashboard | Run list, status, DUUMBI model label, duration, confidence, estimated/final credits, subscription usage. |
| Git providers | Optional GitHub/GitLab connection cards and permission health; native actions remain available without Git. |
| Repositories/workspaces | Register DUUMBI workspace, registry project, or Git repository; show index and entitlement state. |
| Knowledge base | Search/list/detail with provenance and accepted/candidate distinction. |
| Configuration | Org model policy, provider policy, retention, notifications, billing, members, audit. |
| Intake | Sourced artifact view, questions, knowledge citations, graph context, ready-for-spec state. |
| Review | Findings, AC/BDD mapping, GraphPatch/snapshot diff support, missing evidence. |

Use existing duumbi.dev tokens as the default visual source. If app pages need a
new component system, Stage 10 must justify the choice and preserve the current
brand palette.

## Implementation Sequence

Recommended Stage 10 order:

1. Confirm Stage 9 approval and Ready for Build state.
2. Establish the provider-core contract and schema-versioned artifact types.
3. Implement `provider-duumbi` against local `IntentSpec`, session, graph,
   snapshot, BDD, knowledge, and registry context.
4. Add provider-free local intake+spec MVP using deterministic fixtures or a
   bounded mock model path first.
5. Add graph-aware registry metadata loading for the native context index.
6. Add review target modeling for GraphPatch and graph snapshot diffs.
7. Add GitHub/GitLab adapter stubs or narrow adapters only after the native
   contract is stable.
8. Add dashboard/API read contracts for runs and artifacts.
9. Add Loop homepage/design updates only after core contracts are testable or
   in a separate follow-up PR if implementation scope becomes too large.
10. Run focused tests, live E2E, Codex self-review, and final implementation PR
    review. Greptile is reserved for the final implementation PR, not spec PRs.

## BDD-To-Test Mapping

| Product BDD Scenario | Verification Evidence |
| --- | --- |
| Run intake and spec without a Git provider | Integration test creates a temp DUUMBI workspace with an `IntentSpec`, no Git remote, no GitHub/GitLab credentials, runs native intake+spec through provider-duumbi or local CLI, and asserts intake/spec artifacts plus status `completed` or honest `needs_input`. |
| Map a DUUMBI intent into provider-core | Unit test maps an `IntentSpec` with BDD refs into `WorkItem`; asserts stable ID, title, author/source fallback, status, artifact refs, and no Git-specific type names in the core object API. |
| Review a graph patch instead of a PR diff | Unit/integration test supplies a `GraphPatch` and before/after graph snapshot; asserts `ReviewTarget.kind == graph_patch` or equivalent, affected nodes are rendered, and review findings can map to AC/BDD IDs. |
| Close a native workflow by approving a patch application | Integration test applies an approved GraphPatch in a temp workspace, runs graph validation/build evidence where applicable, and asserts `ClosureEvent` links patch id, before/after snapshot ids, and validation evidence without merge commit fields. |
| Keep GitHub as an optional adapter | Adapter unit test maps a synthetic GitHub issue/PR fixture into provider-core and asserts the artifact schema matches native artifacts; no native provider tests require GitHub credentials. |
| Surface running work and subscription usage | API/component test or snapshot fixture asserts run list rows include status, workflow type, source, DUUMBI model label, duration, confidence, estimated/final credits, and subscription usage fields. |
| Register a Git provider without making it mandatory | UI/API test asserts Git provider settings expose connect/health states while native intake/spec actions remain enabled for a workspace with no Git provider. |
| Register a repository or native workspace | Unit/API test registers native workspace, registry project, and Git repository fixtures; asserts project type, index status, graph snapshot status, and entitlement impact. |
| Use the knowledge base during intake | Integration test seeds accepted and candidate knowledge entries; native intake cites accepted entries and marks candidates distinctly in structured artifact metadata. |
| Configure DUUMBI-owned model labels | Unit/API test resolves a DUUMBI model label into internal routing policy and asserts user-facing artifacts store the label while audit metadata stores resolved provider/model without credential values. |
| Apply the duumbi.dev visual system | Frontend test or visual/snapshot review asserts Loop pages reference existing brand tokens and include non-color status labels/icons. If pages are deferred, Stage 10 must record a blocked/deferred evidence note instead of claiming UI completion. |
| Fail closed on missing native artifacts | Integration test references a missing graph snapshot or BDD artifact and asserts run status `blocked` or `needs_action`, no provider fallback, and no mutation side effects. |

Additional technical tests:

- Serialization roundtrip for schema-versioned artifact envelopes.
- Provider capability matrix for DUUMBI, GitHub, and GitLab.
- Path safety tests for artifact refs and registry/context refs.
- Credit estimate and quota gate tests for native runs.
- Audit metadata redaction tests for provider credentials and raw prompts.
- Bounded context tests proving full repo dumps are not sent to model routing.

## Live E2E Plan

Stage 10 must include a live E2E path or an honest blocked-evidence report.

Default local provider-free E2E:

1. Create a temp DUUMBI workspace.
2. Initialize `.duumbi/graph/main.jsonld`, `.duumbi/intents/<slug>.yaml`, and
   linked BDD feature file.
3. Seed `.duumbi/knowledge` with one accepted entry and one candidate entry.
4. Run native provider-duumbi intake.
5. Run native provider-duumbi spec.
6. Assert artifacts include:
   - source refs
   - knowledge citations
   - acceptance criteria
   - BDD mapping
   - model label
   - estimated credits
7. Run native review against a synthetic GraphPatch and snapshot diff.
8. Assert review findings map to AC/BDD IDs.
9. Approve/apply a safe patch or record closure as deferred if closure is out
   of the implemented slice.

Optional low-cost provider-backed E2E:

- May run only if credentials are already configured and expected cost stays
  inside the Ralph Cycle budget.
- Must use a low-cost DUUMBI model label and record resolved provider/model in
  audit metadata.
- Must not be required for CI.
- Must record skipped/blocked status when credentials or budget are missing.

Dashboard/manual E2E:

- If Stage 10 implements UI routes, use the app's existing test stack or
  Playwright/manual screenshot evidence to verify no blank pages, correct
  palette use, non-overlapping text, and visible status/subscription fields.
- If UI is not implemented in the first slice, do not claim UI acceptance.
  Record it as deferred with exact follow-up scope.

## Security, Privacy, And Reliability Requirements

- No raw provider credentials in artifacts, logs, run records, comments, or
  dashboard JSON.
- No full repository dump to LLM providers.
- Every context excerpt must have a source reference and bounded size.
- Artifact paths must reject `..`, absolute paths, and provider-controlled path
  separators.
- Native provider runs must be idempotent where practical; repeated commands
  should update or supersede runs rather than create ambiguous duplicates.
- `needs_action` is required for permission/configuration blockers.
- `needs_input` is required for unanswered product questions.
- Native workflow failures must not mutate graph files before preflight and
  artifact reference validation pass.
- GraphPatch review must validate patch applicability before approval.
- Billing/credit gates must run before costly provider calls.
- Model routing must respect org provider allow/block lists, BYOK settings,
  region policy, and max-cost settings.

## Observability

Stage 10 should emit or persist:

- run lifecycle events
- provider-core mapping events
- context source counts and truncation flags
- model label, estimated credits, final credits, and resolved provider/model
  audit fields
- artifact creation events
- review target loading events
- GraphPatch validation status
- quota and policy gate decisions
- blocked/needs_action reasons

Do not emit secrets, raw prompts containing sensitive source, or provider
response bodies by default.

## Cross-Repo Coordination Policy

- Use one implementation PR per repo unless a smaller cross-repo batch is
  clearly safer and reviewable.
- Each PR must reference #738 with non-closing language until final closure.
- The first PR should establish shared contracts and native MVP tests.
- Dashboard and infra PRs must not proceed ahead of provider-core/native MVP
  unless they are pure docs/design scaffolds.
- If Stage 10 cannot obtain writable access to a required repo, stop and record
  a blocker instead of implementing a misleading partial substitute.

## Ralph Cycle Resource Policy

Stage 10 may run Ralph cycles only after Stage 9 approval and Ready for Build
routing.

Each Ralph cycle must:

1. State the cycle objective and repo scope.
2. Reconfirm the issue is Ready for Build and #738 remains open.
3. Read changed files and relevant instructions before editing.
4. Make a bounded change.
5. Run focused checks for that change.
6. Record BDD scenario coverage, live E2E progress, and remaining risk.
7. Stop or ask for human direction if a gate below is hit.

Resource gates:

- External LLM/provider budget per cycle: maximum USD 2 expected cost.
- External LLM/provider call cap per cycle: maximum 10 calls.
- Default local deterministic tests must not use external LLM calls.
- Codex internal reasoning does not count as DUUMBI external provider usage.
- If a live E2E action is expected to exceed USD 2 or 10 calls, stop for human
  approval before running it.
- If Stage 10 needs new paid cloud resources, stop for human approval.
- If implementation requires a new production dependency in Rust, TypeScript,
  infra, or database migrations, record the rationale and review risk before
  proceeding.
- If a cycle touches authentication, authorization, billing, secrets, provider
  credentials, or tenant isolation, run security-focused review before the next
  cycle.
- If a cycle changes provider routing or model labels, include audit and
  fallback tests.
- If a cycle changes graph mutation or patch application, include rollback and
  no-side-effect tests.

Suggested cycle sequence:

- Cycle 1: provider-core vocabulary, artifact envelopes, and native provider
  fixtures.
- Cycle 2: provider-duumbi local WorkItem/ContextIndex mapping from intent,
  session, graph, BDD, knowledge, and registry metadata.
- Cycle 3: native intake+spec MVP with deterministic fixtures and artifact
  persistence.
- Cycle 4: GraphPatch/snapshot review target and missing-artifact failure
  behavior.
- Cycle 5: model-label routing metadata, credit/quota gates, and audit
  redaction.
- Cycle 6: optional GitHub/GitLab adapter mapping tests.
- Cycle 7: UI/API read contracts for dashboard and homepage slices if still in
  scope.

Autonomous batch cap:

- At most three low-risk Ralph cycles may run before the coordinator must pause
  and summarize progress, unless the user explicitly grants a longer batch.
- Any blocker in product scope, architecture ownership, security, billing,
  cloud cost, or cross-repo access stops the batch immediately.

## Stage 10 Implementation Prompt

Use this prompt after Stage 9 approval and Ready for Build routing:

```text
Run DUUMBI Stage 10 implementation for #738 using specs/DUUMBI-738/PRODUCT.md
and specs/DUUMBI-738/TECHNICAL.md.

Goal: implement the first DUUMBI-native Loop workflow slice. Establish
provider-core vocabulary, implement provider-duumbi as the primary path, prove
native intake+spec against a local/CLI DUUMBI workspace and graph-aware registry
context without GitHub/GitLab credentials, and add review-target support for
GraphPatch or graph snapshot diffs if it fits the approved slice.

Keep GitHub/GitLab as optional adapters. Do not make Git provider credentials,
issues, PRs, MRs, or merge commits prerequisites for the native MVP.

Follow the BDD-to-test mapping, live E2E plan, security/privacy requirements,
and Ralph Cycle resource policy in the technical spec. Use non-closing
references to #738 in all PRs until final workflow closure. Greptile is reserved
for the final implementation PR review and must not be used for spec PRs.

Stop with findings if product scope, architecture ownership, security, billing,
cloud cost, or cross-repo access creates a blocker.
```

## Stage 9 Approval Checklist

Stage 9 approval should verify:

- Stage 5 acceptance was confirmed.
- Product spec exists and contains BDD scenarios.
- Technical spec maps every BDD scenario to verification evidence.
- Technical spec names concrete repo/source context.
- No implementation code or Ralph cycles were created during spec drafting.
- Provider-core avoids Git-specific object names.
- `provider-duumbi` is primary and GitHub/GitLab are adapters.
- Native local/CLI intake+spec MVP can be verified without Git credentials.
- Live E2E plan is bounded and honest about optional provider-backed paths.
- Ralph Cycle policy includes cost, call, scope, security, billing, dependency,
  and blocker gates.
- Spec-only PR uses non-closing references and leaves #738 open.
- Codex self-review reports no blocking findings.

## Open Questions

None blocking for Stage 10 implementation.

Stage 10 may choose exact crate names, API route names, database table names,
component names, and DUUMBI model-label display copy as implementation details
when the choices preserve the product and technical contracts above.
