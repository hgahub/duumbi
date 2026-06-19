# DUUMBI-738: DUUMBI Loop Native Workflow Adaptation

Related to #738.

This PR is specification-only and must leave the execution issue open. Do not
use closing references such as "closes" or "fixes" for #738.

## Summary

Make the DUUMBI Loop workflow native to DUUMBI instead of Git-first.

The accepted product outcome is:

```text
DUUMBI intent -> intake -> spec -> review -> closure
```

That loop must run from DUUMBI intents, session-ledger events, graph snapshots,
and GraphPatch approval without requiring a GitHub issue, GitLab issue, PR, MR,
or merge commit. GitHub and GitLab remain supported as provider adapters for
teams that want issue and PR workflows, but they are no longer the primary
object model.

The first buildable product slice is a DUUMBI-native intake+spec flow against a
local or CLI workspace and the graph-aware `duumbi-registry`. The slice must
also define the web product shape for Loop: public homepage, login/account
management, dashboard, Git provider registration, repository registration,
knowledge base, configuration, intake, and review.

## Problem

The current Loop plan is useful for GitHub/GitLab teams, but it treats Git
objects as the canonical workflow objects:

- Git issue as the work request
- PR diff as the reviewable change
- merge commit as closure evidence
- tree-sitter/code-file indexing as the primary code knowledge path

That is the wrong center of gravity for DUUMBI. DUUMBI already has a stronger
native model:

- Query, Intent, and Agent modes
- runtime `IntentSpec` artifacts with acceptance criteria, BDD links, verifier
  tests, and execution metadata
- session persistence under `.duumbi/session`
- graph snapshots and GraphPatch mutation plans
- JSON-LD graph indexing and knowledge artifacts
- registry and workspace concepts that do not require Git

If Loop remains Git-first, DUUMBI-native users are forced to create external
Git provider objects just to run the product's own intent workflow. That adds
friction, leaks implementation detail into the product model, weakens local/CLI
flows, and makes the future app dashboard harder to reason about.

## Outcome

When this is implemented:

- A user can start a Loop intake/spec workflow from a DUUMBI intent or session
  event without connecting GitHub or GitLab.
- DUUMBI-native workflow objects are first-class:
  - work request: DUUMBI intent record
  - intake artifact: sourced analysis over the DUUMBI workspace, graph, registry
    metadata, knowledge base, and user context
  - spec artifact: product and technical specifications plus BDD mapping
  - review target: GraphPatch, graph snapshot diff, or generated artifact diff
  - closure event: approved patch application or accepted generated artifact
    state
- GitHub and GitLab are optional provider adapters that map their objects into
  the same Loop provider-core contract.
- The native flow uses the JSON-LD graph index for DUUMBI projects. It does not
  require a tree-sitter source-code index for DUUMBI-native projects.
- Artifact schemas align with DUUMBI Query, Intent, Agent, and BDD artifacts
  delivered by #673.
- The first MVP proves DUUMBI-native intake+spec end-to-end against a local/CLI
  workspace and the graph-aware `duumbi-registry`.
- The public and application web surfaces communicate the native DUUMBI flow,
  not only GitHub/GitLab automation.
- Users see DUUMBI-owned model labels. DUUMBI decides internally which provider
  and model satisfy the task, cost, data-residency, and quality policy.
- The execution issue remains open after this spec PR for Stage 10
  implementation and later workflow gates.

## Scope

### In Scope

- Define `provider-duumbi` as the primary provider implementation behind a
  shared provider-core contract.
- Preserve GitHub/GitLab as optional adapters that translate provider events
  into the same Loop workflow object model.
- Map current Git-first objects into native DUUMBI equivalents:
  - issue -> intent record
  - PR diff -> GraphPatch, graph snapshot diff, or generated artifact diff
  - merge commit -> approved patch application or accepted artifact state
  - issue comments -> intent/session events and dashboard commands
- Define native Loop artifact schemas for intake, spec, review, and closure.
- Use DUUMBI JSON-LD graph data, graph snapshots, registry metadata, and
  knowledge records as first-class context sources.
- Define the first local/CLI MVP for intake+spec without Git.
- Define product behavior for the requested pages:
  - Loop homepage
  - user management and login
  - application dashboard with running tasks, task status, and subscription
    usage
  - Git provider registration
  - repository registration
  - knowledge base
  - configuration
  - intake with research support using the knowledge base
  - review
- Apply the duumbi.dev visual system:
  - existing parchment/ink/blue-rust light tokens for public pages
  - existing dark app theme with DUUMBI green accent, including the current
    dark-theme accent family around `#86c06a` and `#97cf7d`
  - dense admin dashboard layout for operational pages
- Define how users see DUUMBI-owned model choices while provider/model routing
  remains internal.
- Define BDD scenarios that Stage 8 and Stage 10 can map to tests.

### Explicitly Out Of Scope

- Implementation code, tests, generated runtime artifacts, or Ralph cycles
  during Stage 6, Stage 7, Stage 8, or Stage 9.
- Removing GitHub or GitLab support.
- Full cloud production deployment in the first MVP.
- A full implementation of all homepage/dashboard pages in this spec PR.
- Full closure automation for every generated artifact type in the first MVP.
- Replacing DUUMBI's existing verifier, BDD, preflight, graph validation,
  provider routing, or registry client behavior.
- Exposing raw provider names or model IDs as the primary user-facing model
  selection contract.
- Sending a full repository dump to any LLM provider.
- Greptile review on this spec PR. Greptile is reserved for the final
  implementation PR.

## Constraints And Assumptions

Facts:

- Issue #738 is open.
- Issue #738 is labeled `accepted` and `needs-spec`.
- The Stage 5 Human Acceptance Decision comment dated 2026-06-19 records
  `Decision: Accept`, `Remaining open questions: none`, and `Next state: Spec
  Needed`.
- The issue asks for DUUMBI-native intake, spec, review, and closure triggered
  by DUUMBI intents and session-ledger events with no Git dependency.
- The issue explicitly keeps GitHub/GitLab as optional provider adapters.
- The issue requires `provider-duumbi` as the primary implementation of
  provider-core.
- The issue asks to replace the tree-sitter pipeline with the JSON-LD graph
  index for DUUMBI-native projects.
- The issue asks to align artifacts with DUUMBI Query, Intent, Agent, and BDD
  artifacts.
- The user clarified that the work spans `duumbi`, `duumbi-loop`,
  `duumbi-vault`, `duumbi-web`, `duumbi-infra`, and `duumbi-registry`.
- The user clarified that the duumbi.dev color system must be applied.
- The user clarified that users see DUUMBI-owned model labels while DUUMBI
  controls the actual provider/model routing internally.

Assumptions:

- `duumbi` remains the source of truth for compiler, graph, intent, session,
  BDD, registry-client, and local workflow primitives.
- `duumbi-loop` owns the cloud/service orchestration layer even though the
  current local checkout is effectively an empty coordination repository.
- `duumbi-web` owns public marketing/docs surfaces and should influence the
  Loop homepage visual language.
- `duumbi-registry` can be extended or adapted as the graph-aware registry
  surface for the MVP.
- `duumbi-infra` owns Azure deployment and operational resources for the future
  cloud service.
- For the first MVP, "no Git dependency" means a local workspace can run
  native intake+spec without Git remotes, provider comments, PRs, or merge
  commits. A workspace may still physically exist inside a Git checkout.

Constraints:

- Native Loop context selection must be bounded and sourced. Full repo dumps
  are not acceptable LLM context.
- The first MVP must work without GitHub/GitLab credentials.
- GitHub/GitLab adapter behavior must not fork the artifact model.
- Broken native artifact references should fail closed with actionable status
  rather than silently falling back to Git provider behavior.
- Dashboard state must distinguish user input needed, platform action needed,
  failed, cancelled, completed, and running.
- Color cannot be the only status indicator.
- Subscription and credit usage must be visible before starting costly runs.
- BYOK and platform-key routing must share the same user-facing DUUMBI model
  label contract.

## Decisions

- **Decision:** Make `provider-duumbi` the default provider path.
  **Evidence:** DUUMBI's native workflow already has intent, graph, session,
  BDD, and registry primitives. Making Git the primary provider forces external
  infrastructure into local DUUMBI workflows.

- **Decision:** Keep provider-core object names generic.
  **Evidence:** A provider-core contract that says `WorkItem`, `ChangeSet`,
  `ReviewTarget`, and `ClosureEvent` can be implemented by DUUMBI, GitHub, and
  GitLab. A contract that says `Issue`, `PullRequest`, and `MergeCommit` bakes
  Git into the abstraction.

- **Decision:** Use JSON-LD graph and snapshot data for DUUMBI-native projects.
  **Evidence:** DUUMBI is graph-centered. The active compiler and agent paths
  already operate on JSON-LD graphs and GraphPatch mutation plans.

- **Decision:** Treat BDD scenarios as a first-class artifact contract.
  **Evidence:** #673 delivered BDD/Gherkin companion artifact support for
  runtime intents, and #738 explicitly asks to align with BDD artifacts.

- **Decision:** User-facing model names are DUUMBI products, not provider SKUs.
  **Evidence:** Users should choose quality, speed, cost, privacy, and workflow
  intent. DUUMBI should route internally to provider/model combinations based on
  policy, availability, region, and task fit.

- **Decision:** The first MVP is local/CLI native intake+spec, not the whole
  cloud dashboard.
  **Evidence:** The issue names local/CLI workspace plus graph-aware registry as
  the MVP. The dashboard pages must be designed as product scope, but a local
  proof is the smallest reliable end-to-end slice.

## Product Surfaces

### Loop Homepage

The homepage at `loop.duumbi.dev` should explain the DUUMBI-native loop:

```text
Intent -> Intake -> Spec -> Review -> Closure
```

Required sections:

- hero with the DUUMBI Loop name and a direct product promise
- DUUMBI-native workflow, not only GitHub PR automation
- code graph knowledge base
- intake/spec/review/closure flow
- security and privacy
- pricing preview with credits and BYOK
- call to action into the dashboard

Visual direction:

- use the existing duumbi.dev brand system
- public pages may reuse the parchment/ink editorial feel
- Loop app CTAs and native workflow graphics should use the DUUMBI green accent
  used by the current dark theme
- avoid a generic SaaS gradient look

### User Management And Login

Required behavior:

- users can sign up, log in, log out, and recover access
- organizations support owner, admin, developer, reviewer, viewer, and billing
  roles
- invitation and identity-linking flows handle conflicts explicitly
- account deletion and org export/delete paths are discoverable from settings
- authentication state gates all app routes

### Application Dashboard

Required behavior:

- users can see their running and recent tasks
- each task shows workflow type, status, source, provider/model label,
  confidence, duration, estimated/final credit cost, and timestamp
- running tasks update live or near-live
- tasks waiting on the user are elevated above passive history
- subscription usage shows included credits, purchased credits, spent credits,
  pending estimated credits, reset date, and limit state
- quota blocks are visible before a costly run starts

### Git Provider Registration

Required behavior:

- users can connect GitHub and GitLab as optional adapters
- provider registration is not required for native DUUMBI local/CLI runs
- provider cards show connection status, permission health, last event, and
  repair actions
- branch/PR permission failures produce `needs_action`, not silent failure

### Repository Registration

Required behavior:

- users can register a DUUMBI-native workspace, a Git-backed repository, or a
  registry-backed project
- enabled repositories/workspaces show index status, last graph snapshot,
  knowledge coverage, and plan entitlement usage
- repo limits are enforced by plan before new indexing starts

### Knowledge Base

Required behavior:

- knowledge entries are searchable by run, issue/intent, source, tag, and
  provenance
- entries distinguish accepted knowledge from candidates
- intake can cite knowledge base entries and explain how each source affected
  scope, risk, or recommendation
- users can reject stale or unsafe knowledge

### Configuration

Required behavior:

- org settings include model policy, allowed providers, data residency,
  retention, integrations, notifications, billing, members, and audit export
- users see DUUMBI model labels such as fast, balanced, deep analysis, and
  strict review
- provider/model IDs remain internal routing details except in audit/debug views
  for admins
- BYOK settings verify credentials without requiring users to maintain default
  raw model IDs

### Intake

Required behavior:

- native intake can start from a DUUMBI intent or session-ledger event
- intake loads bounded context from the intent, graph, registry metadata,
  knowledge base, similar examples, and answered questions
- intake returns business value, effort, affected areas, risks, sources,
  questions, and a recommendation
- unresolved critical questions put the run into `needs_input`
- once questions are answered, the run becomes ready for spec

### Review

Required behavior:

- native review can target a GraphPatch, graph snapshot diff, generated
  artifact diff, or Git PR/MR diff through an adapter
- review maps findings to product acceptance criteria and BDD scenarios
- review distinguishes blocking findings, warnings, nits, missing tests, and
  broader-evidence gaps
- review comments and dashboard findings must reference the same underlying
  artifact IDs

## Native Workflow Model

Recommended provider-core vocabulary:

| Provider-Core Object | DUUMBI Provider Mapping | GitHub/GitLab Adapter Mapping |
| --- | --- | --- |
| `WorkItem` | intent record or session-ledger request | issue, discussion, or MR/PR command context |
| `ContextIndex` | JSON-LD graph index, registry metadata, knowledge store | code index plus optional graph/index artifacts |
| `IntakeArtifact` | native markdown/JSON artifact linked to intent/run | issue-linked artifact committed or attached by adapter |
| `SpecArtifact` | product/technical specs plus BDD links | spec files in branch/PR |
| `ChangeSet` | GraphPatch, graph snapshot diff, generated artifact diff | PR/MR diff |
| `ReviewTarget` | patch/diff/artifact bundle | PR/MR review diff |
| `ClosureEvent` | approved patch application or accepted artifact state | merge commit or closed PR/MR |

## User-Facing Model Contract

Users should not be asked to choose `provider:model` SKUs as the main product
workflow. The dashboard should expose DUUMBI-owned model labels:

- Fast
- Balanced
- Deep Analysis
- Strict Review
- Low Cost
- BYOK Balanced

Each label maps internally to a routing policy:

- task type: intake, spec, review, closure, embedding, graph reasoning
- capability needs: reasoning, code, JSON schema, long context, tool use,
  vision, embeddings
- region/data policy: EU, USA, China, or global fallback
- budget ceiling and credit estimate
- provider allowlist/blocklist
- fallback chain

Admin/audit views may show the resolved provider/model after the run for
debugging, compliance, and cost review. The default user workflow should show
the DUUMBI label, credit estimate, confidence, and evidence quality.

## BDD Scenarios

```gherkin
Feature: DUUMBI-native Loop workflow

  Scenario: Run intake and spec without a Git provider
    Given a DUUMBI workspace has an accepted intent record
    And no GitHub or GitLab provider is connected
    When the user starts a native Loop intake and spec run
    Then DUUMBI creates sourced intake and spec artifacts
    And the run does not require an issue, pull request, merge request, or merge commit

  Scenario: Map a DUUMBI intent into provider-core
    Given a DUUMBI intent has acceptance criteria and BDD artifacts
    When provider-duumbi loads the intent as a work item
    Then the work item exposes stable ID, title, author, status, sources, and artifact links
    And the provider-core contract does not use Git-specific object names

  Scenario: Review a graph patch instead of a PR diff
    Given a DUUMBI Agent proposes a GraphPatch for an intent
    When the native Loop review runs
    Then the review target is the GraphPatch and graph snapshot diff
    And each blocking finding references an acceptance criterion or BDD scenario

  Scenario: Close a native workflow by approving a patch application
    Given a native review has no blocking findings
    And an authorized user approves the proposed GraphPatch
    When DUUMBI applies the patch and records graph validation evidence
    Then the closure event links the approved patch, graph snapshot, validation evidence, and final artifact state
    And no merge commit is required

  Scenario: Keep GitHub as an optional adapter
    Given an organization has connected GitHub
    When a user starts Loop from a GitHub issue
    Then the GitHub adapter maps the issue into the same provider-core work item contract
    And the produced artifacts follow the same schema as the DUUMBI-native path

  Scenario: Surface running work and subscription usage
    Given a user has active intake, spec, and review runs
    When the user opens the application dashboard
    Then each run shows status, workflow type, source, DUUMBI model label, duration, confidence, and credit estimate
    And the subscription panel shows used, pending, remaining, and reset credits

  Scenario: Register a Git provider without making it mandatory
    Given an organization has no Git provider connected
    When an admin opens Git provider settings
    Then the app offers GitHub and GitLab connection flows
    And native DUUMBI workflow actions remain available without connecting either provider

  Scenario: Register a repository or native workspace
    Given an admin wants Loop to analyze a project
    When the admin registers a DUUMBI-native workspace, registry project, or Git repository
    Then the app records the project type, index status, graph snapshot status, and plan entitlement impact

  Scenario: Use the knowledge base during intake
    Given the knowledge base has accepted entries and candidates
    When native intake analyzes a new intent
    Then the intake artifact cites relevant accepted entries
    And candidate entries are clearly marked before they influence recommendations

  Scenario: Configure DUUMBI-owned model labels
    Given an org admin configures model policy
    When the admin chooses a default for spec runs
    Then the UI shows DUUMBI-owned labels such as Balanced or Deep Analysis
    And the audit view can show the resolved provider and model after a run

  Scenario: Apply the duumbi.dev visual system
    Given a user opens Loop public or app pages
    When the UI renders homepage, login, dashboard, providers, repositories, knowledge, configuration, intake, and review pages
    Then the pages use the existing duumbi.dev palette and DUUMBI green accent
    And status is communicated with text, icons, and shape, not color alone

  Scenario: Fail closed on missing native artifacts
    Given a native spec or review run references a graph snapshot or BDD artifact
    And the referenced artifact is missing or unreadable
    When the run starts
    Then the run enters a blocked or needs_action state with a repair message
    And DUUMBI does not silently switch to a Git provider fallback
```

## Acceptance Criteria

- `provider-duumbi` is the primary provider path in the design.
- GitHub and GitLab remain optional adapters, not prerequisites.
- Native intake+spec can run against a local/CLI DUUMBI workspace without Git
  provider credentials.
- Provider-core vocabulary avoids Git-specific object names.
- DUUMBI intent records, session events, graph snapshots, GraphPatch diffs, BDD
  artifacts, registry metadata, and knowledge entries are valid context sources.
- The spec defines every requested page and its minimum behavior.
- The dashboard exposes running tasks, task status, and subscription usage.
- The model UX uses DUUMBI-owned labels while provider/model routing remains
  internal.
- The visual direction applies the duumbi.dev palette and DUUMBI green accent.
- Review can target GraphPatch and graph snapshot diffs.
- Closure can be represented by approved patch application without a merge
  commit.
- BDD scenarios cover native provider operation, adapter compatibility, UI,
  model policy, knowledge use, and failure handling.
- Stage 10 implementation must keep this execution issue open until later
  workflow closure.

## Risks And Trade-Offs

- Provider-core may become too abstract if it hides important provider
  differences. Mitigation: keep provider-specific capabilities explicit but
  keep workflow artifacts common.
- The empty or sparse `duumbi-loop` repository increases Stage 10 scaffolding
  risk. Mitigation: implement the first MVP with narrow modules and avoid a
  full platform rewrite.
- Native GraphPatch review can be less familiar than PR review. Mitigation:
  render snapshot diffs, affected symbols, BDD mapping, and rollback evidence in
  human-readable form.
- DUUMBI-owned model labels improve UX but can obscure cost/debug details.
  Mitigation: expose resolved provider/model in audit/admin views after the run.
- Cross-repo implementation creates coordination risk. Mitigation: sequence the
  first MVP around local CLI/provider-core contracts before broad dashboard and
  infra work.

## Rollout Notes

1. Deliver provider-core and provider-duumbi MVP for local/CLI intake+spec.
2. Add graph-aware registry context integration.
3. Add review target support for GraphPatch and graph snapshot diffs.
4. Add dashboard read views for native runs and artifacts.
5. Add GitHub/GitLab adapters against the same contract.
6. Expand closure automation after native review evidence is reliable.

## Open Questions

None blocking for Stage 8 or Stage 10 planning.

Stage 10 may choose exact route names, component names, storage table names, and
DUUMBI model-label copy as long as the choices preserve the accepted product
contract above.
