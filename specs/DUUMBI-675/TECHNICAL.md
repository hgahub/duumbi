# DUUMBI-675: Scheduled Provider Model Catalog Refresh With User Approval - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-675/PRODUCT.md` by adding a
safe v1 provider model catalog refresh path, updating DUUMBI's direct-provider
set, and keeping model choice internal to DUUMBI routing rather than a user
configuration chore.

The required implementation flow is:

```text
accepted direct-provider metadata -> deterministic public catalog + sha256 ->
bounded local hash check -> user review and approval -> validated atomic
user-level catalog cache -> embedded catalog fallback
```

This technical spec also includes the Stage 8 prompt addition: because the
provider count grows beyond the current `/provider` table size, the TUI provider
selection window must become scrollable when more than 8 providers are
available.

Related to #675. This is a technical-specification artifact only. The execution
issue must remain open for Stage 9 Technical Spec Review, Stage 10
implementation, Stage 11 review, and Stage 12 closure.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Oz implementation agents when routed from Slack or GitHub.
- Reviewer agents checking provider taxonomy, catalog validation, TUI behavior,
  CI coverage, and live E2E evidence.
- Docs agents coordinating DUUMBI docs-site changes for provider setup and
  catalog-refresh behavior.
- Stage 9 technical reviewers checking implementability and resource policy.

## Source Context

- GitHub issue: https://github.com/hgahub/duumbi/issues/675
- Product spec: `specs/DUUMBI-675/PRODUCT.md`
- Product spec PR: https://github.com/hgahub/duumbi/pull/679
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/675#issuecomment-4639355329
- Stage 6 product spec draft comment:
  https://github.com/hgahub/duumbi/issues/675#issuecomment-4641794447
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/675#issuecomment-4643749919
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`

Relevant source facts verified for Stage 8:

- `src/agents/model_catalog.rs`
  - Defines `ModelCatalogEntry`, `ModelSelectionContext`, `catalog()`,
    `select_model`, and `resolve_provider_config`.
  - Uses static `&'static str` model identifiers, which is insufficient for a
    refreshed user-level catalog without either an owned entry type or a
    catalog document abstraction.
  - Current embedded entries cover Anthropic, OpenAI, Grok, MiniMax, and
    OpenRouter.
  - Current routing scores are `quality`, `speed`, `cost_efficiency`,
    `reasoning`, and `coding`.
- `src/config.rs`
  - `ProviderKind` currently includes Anthropic, OpenAI, Grok, OpenRouter, and
    MiniMax.
  - `ProviderConfig` stores provider kind, role, optional legacy model override,
    API key env var, optional base URL, optional timeout, key storage, and
    optional auth-token env var.
  - Provider config intentionally stores env-var names and metadata, not secret
    values.
- `src/agents/factory.rs`
  - Constructs runtime providers from `ProviderKind`.
  - Current factory branches only cover Anthropic, OpenAI, Grok, OpenRouter,
    and MiniMax.
  - Existing OpenAI-compatible adapters are already used for Grok and MiniMax
    base-url behavior.
- `src/agents/model_access.rs`
  - Persists credential-scoped model access metadata under the user DUUMBI
    home, keyed by non-reversible credential fingerprint.
  - Tracks accessible, denied, auth-failed, and unknown model probe states.
  - This is useful input for routing but must remain separate from catalog
    update state and credentials.
- `src/cli/provider_startup.rs`
  - Discovers provider setup from known API key env vars at startup.
  - Current candidates include Anthropic, OpenAI, Grok via `XAI_API_KEY`,
    OpenRouter, and MiniMax.
  - Successful setup persists environment-backed user provider entries and
    records model-access probe metadata.
- `src/main.rs`
  - Runs provider auto-configuration during CLI startup and reloads effective
    config after successful setup.
  - Does not currently run a remote catalog hash check or adoption flow.
- `src/cli/app.rs`
  - Defines the TUI `/provider` wizard provider table through static
    `PROVIDER_KINDS`.
  - Renders every provider row in `render_provider_panel` without a provider
    list scroll window.
  - Tracks provider panel state as `selected`, `input_mode`, and `status_msg`.
  - Already has focused provider-panel tests for `Esc`, up/down bounds,
    configure, delete, test connection, paste, key entry, and status rendering.
- `src/cli/mode.rs`
  - Defines `PanelState::ProviderManager`.
  - It does not currently include a provider-list scroll offset.
- `src/cli/repl.rs`
  - Opens `/provider` with `selected: 0`, no provider list scroll state.
  - Query mode remains read-only by default; provider setup is a bounded
    configuration surface.
- `src/cli/provider.rs`
  - Implements non-TUI `duumbi provider list|add|remove|set`.
  - Current parser accepts Anthropic, OpenAI, Grok, OpenRouter, and MiniMax.
- `crates/duumbi-studio/src/lib.rs` and
  `crates/duumbi-studio/src/server_fns.rs`
  - Expose provider settings and provider connection tests for Studio.
  - Studio currently serializes provider kind via `Debug`/lowercase behavior and
    tests active provider connections through existing factory paths.
- `.github/workflows/copilot-review.yml`
  - Requests Copilot as the default automated reviewer on ready PRs.
  - Does not invoke Greptile.
- `.github/workflows/technical-spec-review-request.yml`
  - Requires a linked, open, non-draft TECHNICAL.md PR before Technical Spec
    Review notification is ready.
  - Treats actual reviewer submissions from required automated reviewers as
    review evidence.

Active workflow state verified:

- Issue #675 is open and labeled `accepted`, `product-spec-approved`, and
  `needs-tech-spec`.
- Product spec PR #679 was merged on 2026-06-07.
- The Stage 7 approval comment was added after the Stage Approval workflow
  failed to record the transition automatically.
- GitHub Project items were not exposed by `gh issue view`; label repair is the
  available verified state transition evidence.

Assumptions for implementation:

- `xai` should become the canonical provider key in new config, display, docs,
  catalog, and provider setup surfaces.
- Existing `grok` config should remain read-compatible where practical through
  an explicit legacy alias or migration path; it must not remain the canonical
  display/config key for new behavior.
- Existing `openrouter` configs should not be silently deleted. OpenRouter may
  remain legacy/runtime-compatible if needed, but it must not appear in the v1
  catalog, new provider setup list, or v1 docs.
- New provider client endpoints and compatibility details must be verified
  against provider documentation during Stage 10 before committing provider
  runtime behavior. If a direct provider cannot support DUUMBI's current
  `LlmProvider` contract safely, implementation must stop for a scope decision
  rather than shipping a nominal provider entry that cannot be tested.
- CI must not require live provider credentials. Live provider calls are manual
  smoke evidence only.

## Affected Areas

Expected source areas for Stage 10 implementation:

- Provider identity and config:
  - `src/config.rs`
  - `tests/integration_phase9b.rs`
  - `README.md`
  - config examples under `docs/` and `src/mcp/client/config.rs`
- Provider runtime construction:
  - `src/agents/factory.rs`
  - existing provider modules under `src/agents/`
  - new provider modules only when a direct-provider API adapter is verified
- Catalog schema, validation, storage, and routing:
  - `src/agents/model_catalog.rs`
  - new catalog schema/store/client module under `src/agents/` or a small
    submodule namespace such as `src/agents/model_catalog/`
  - unit tests colocated with catalog modules
- Provider setup and startup:
  - `src/cli/provider_startup.rs`
  - `src/cli/provider.rs`
  - `src/main.rs`
- REPL/TUI provider panel:
  - `src/cli/app.rs`
  - `src/cli/mode.rs`
  - `src/cli/repl.rs`
- Studio provider-management surface:
  - `crates/duumbi-studio/src/lib.rs`
  - `crates/duumbi-studio/src/server_fns.rs`
  - `crates/duumbi-studio/src/app.rs`
  - `crates/duumbi-studio/src/style/studio.css`
  - Studio layout or API tests under `crates/duumbi-studio/tests/`
- Publisher and validation automation:
  - a scheduled/manual GitHub Action for catalog generation
  - a deterministic generator script or Rust command
  - curated metadata files used by the generator
  - workflow summary/evidence artifact generation
- Documentation:
  - repository provider references such as `README.md` and `docs/testing/*`
  - `hgahub/duumbi-web/docs` for public docs-site provider setup,
    catalog-refresh behavior, and static catalog publication paths

Areas that must not change during Stage 8:

- `specs/DUUMBI-675/PRODUCT.md`
- implementation source files
- tests
- generated catalog artifacts
- runtime assets
- technical specs for other issues

Areas expected not to change during Stage 10:

- compiler/parser/graph/runtime behavior unrelated to providers or catalog
  metadata
- registry publishing semantics
- MCP graph mutation tools
- issue workflow approval semantics, except for any catalog-specific publisher
  workflow added by this issue

## Technical Approach

### Provider Taxonomy

Define one canonical provider metadata table and use it as the source of truth
for new provider setup surfaces, startup discovery, catalog validation, and docs
checks.

The v1 supported direct-provider table is:

| Display name | Canonical key | API key env var | V1 catalog | New setup |
| --- | --- | --- | --- | --- |
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` | yes | yes |
| OpenAI | `openai` | `OPENAI_API_KEY` | yes | yes |
| xAI | `xai` | `XAI_API_KEY` | yes | yes |
| MiniMax | `minimax` | `MINIMAX_API_KEY` | yes | yes |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` | yes | yes |
| Alibaba Cloud Model Studio (Qwen) | `qwen` | `DASHSCOPE_API_KEY` | yes | yes |
| Moonshot AI (Kimi) | `moonshot` | `MOONSHOT_API_KEY` | yes | yes |
| Zhipu AI (GLM) | `zhipu` | `ZHIPUAI_API_KEY` | yes | yes |
| Google Gemini | `gemini` | `GEMINI_API_KEY` | yes | yes |

Required compatibility behavior:

- `ProviderKind::Xai` or equivalent must serialize and display as `xai`.
- `grok` may deserialize or parse as a legacy alias for xAI if compatibility is
  retained.
- New examples, generated config, provider panel rows, startup setup messages,
  catalog entries, and docs must use `xai`.
- `OpenRouter` must be excluded from v1 catalog validation and new setup lists.
- Existing OpenRouter configs should either remain legacy-compatible or show a
  clear unsupported/migration state. They must not be silently removed or
  rewritten.

Implementation agents should avoid scattering provider lists across files.
Either expose a small provider metadata helper in config/provider code or keep a
single static table and derive:

- canonical key parsing
- display labels
- default API key env var
- provider setup list membership
- catalog v1 support membership
- legacy alias membership

### Catalog Schema And Hash Contract

Introduce a versioned v1 catalog document with owned, deserializable data rather
than only static `ModelCatalogEntry` values.

Required schema concepts:

- `schema_version`: exact v1 value, rejected when unsupported.
- `catalog_version` or `content_version`: semantic catalog version/hash input.
- `content_timestamp`: timestamp for the semantic catalog content, not an
  every-run generation timestamp.
- `generator_status_summary`: safe user/reviewer-facing summary of the
  generator result for the adopted semantic catalog, including whether all
  providers used fresh discovery or whether validated fallback metadata was
  included.
- `providers`: accepted direct providers with display name, canonical config
  key, API key env var, and provider-level notes.
- `provider_discovery_status`: per-provider status with discovery source,
  freshness/fallback state, and warning text when curated or
  previous-known-good metadata was used because live discovery was unavailable.
- `models`: provider/model entries with lifecycle state, routing scores,
  `reasoning`, `coding`, and optional safe metadata.
- provider/model provenance: semantic discovery status such as fresh discovery,
  curated fallback, previous-known-good fallback, or manually curated metadata.
  If fallback metadata is adopted, the catalog must carry a user-facing stale or
  fallback warning so CLI/Studio review surfaces can explain why the metadata is
  not freshly discovered.
- `change_summary`: concise user-facing summary for review surfaces.
- `source`: commit or curated metadata reference for the semantic content,
  including source and curation provenance sufficient for reviewer inspection.

Do not include per-run publisher evidence in the adopted catalog bytes. The
product spec requires that the semantic/adoption hash not change solely because
a scheduled workflow ran again. Therefore:

- `model-catalog.v1.json` must be deterministic for unchanged semantic inputs.
- `model-catalog.v1.sha256` must hash the exact deterministic public catalog
  bytes.
- Semantic provenance that affects user trust in the catalog, including whether
  a provider/model entry came from fresh discovery or accepted fallback metadata,
  belongs in `model-catalog.v1.json` and is part of the adoption hash.
- Per-run evidence such as workflow run ID, actual generation time, raw provider
  fetch attempts for that run, validation logs, and operational warnings that do
  not change adopted semantic provenance belongs in a separate workflow artifact
  or workflow summary, not in `model-catalog.v1.json`.

Validation must fail closed for:

- unsupported schema version
- missing required fields
- unsupported provider keys
- OpenRouter appearing in the v1 supported provider set
- `grok` appearing as a canonical provider key
- duplicate provider keys
- duplicate provider/model pairs
- empty model IDs
- invalid lifecycle values
- routing scores outside 0-100
- model entries pointing at unknown providers
- missing generator status summary
- missing per-provider discovery status
- fallback metadata without catalog-visible provenance or stale/fallback warning
- malformed or unexpected user-facing change summaries

### Embedded, Local, And Active Catalogs

Keep the embedded catalog as the deterministic fallback. Route model selection
through an active catalog resolver:

```text
valid user-level refreshed catalog -> embedded catalog
```

The refreshed catalog must live under user-level state:

- `~/.duumbi/model-catalog/current.json`
- `~/.duumbi/model-catalog/current.sha256`
- `~/.duumbi/model-catalog/state.json`

`state.json` should record only update-control state, such as:

- last check time
- installed hash
- last offered hash
- skipped hash or skipped hashes
- remind-later-until timestamp
- disabled flag
- check frequency
- last safe failure code/message without secrets

Atomic adoption requirements:

- write temp files in the same directory
- validate the downloaded bytes before writing active files
- write and read back the temp catalog when practical
- rename temp files into place only after validation succeeds
- never treat partial files as active
- fall back to the previous valid local catalog or embedded catalog after local
  write/read failures
- never mutate provider config or credentials as part of catalog adoption

### Catalog Client Check

Implement hash-first checking with bounded IO.

Default behavior:

- check at most once per day unless the user changes frequency
- fetch `model-catalog.v1.sha256` before downloading JSON
- if the hash matches the installed hash, update last-check state and continue
  quietly
- if the hash is skipped or still deferred, continue quietly
- if the hash is new and actionable, show a concise review surface
- on approval, download JSON, verify SHA-256, validate schema/provider/model
  fields, revalidate that the approved hash matches the reviewed catalog, then
  adopt atomically

Failure behavior:

- remote timeout, DNS/network failure, unavailable URL, hash mismatch, corrupted
  JSON, unsupported schema, invalid entries, and partial local writes must keep
  startup usable
- diagnostics must avoid secrets, raw provider payloads, and credential values
- ordinary commands must not become noisy when no user action is useful

Recommended user surfaces:

- interactive REPL/TUI startup: non-blocking notice only when a new actionable
  hash exists, with review available through `/provider`
- `/provider`: review, approve, skip, remind later, disable, and frequency
  controls
- non-TUI CLI: add explicit `duumbi provider catalog ...` subcommands or an
  equivalent provider-management path so users can review and control updates
  without Studio
- Studio provider-management: same review/adoption controls as CLI/TUI

### Provider Runtime Adapters

For each newly accepted provider, Stage 10 must decide whether the provider can
use an existing OpenAI-compatible adapter or needs a dedicated `LlmProvider`
implementation. Do not rely on provider names alone.

Rules:

- Verify current provider API docs before hardcoding endpoints or auth formats.
- Use existing `LlmProvider` trait and `factory` patterns.
- Keep provider-specific HTTP behavior inside provider modules, not scattered
  across CLI or catalog code.
- Add mock-HTTP tests for each new provider adapter.
- Keep CI credential-free.
- If a provider cannot support DUUMBI's required chat/tool contract safely in
  v1, stop for a scope decision before advertising it as ready in setup docs.

OpenRouter handling:

- It must not be emitted by v1 catalog generation.
- It must not appear in new setup/docs examples.
- If runtime compatibility remains, tests should label it as legacy behavior
  and avoid treating it as part of the accepted v1 direct-provider set.

### REPL/TUI Provider Panel Scroll

The current provider panel renders all provider rows. After the v1 provider set
expands to nine supported direct providers, the main provider list must render a
scrollable window when there are more than 8 provider rows.

Implementation direction:

- Extend `PanelState::ProviderManager` with a provider-list scroll offset or add
  equivalent state in `ReplApp`.
- Set the visible provider-row limit to 8 for the main provider selection list.
- On `Up` and `Down`, update the selected index and scroll offset so the
  selected provider remains visible.
- Preserve `Esc`, `Enter`, `D`, and `T` behavior.
- Preserve footer/status rows so `[Enter] Configure`, `[D] Delete key`, and
  `[T] Test connection` remain visible.
- Include a compact scroll affordance when not all providers are visible, such
  as `1-8 of 9`, `more below`, or a minimal up/down indicator.
- Reset or clamp the scroll offset when opening `/provider`, when provider list
  membership changes, or when returning from auth/key submodes.
- Keep auth/key input submodes non-scroll unless their own option count grows.
- Continue to let `Esc` close the active panel consistently.

The provider list should be derived from canonical provider metadata, not a
second hand-maintained table that can drift from catalog validation.

### Publisher And Publication

Add a scheduled and manually dispatchable publisher.

Required behavior:

- covers only accepted v1 direct providers
- excludes OpenRouter
- uses provider discovery where safe and available
- merges discovery with curated DUUMBI routing metadata
- produces deterministic JSON bytes for unchanged semantic inputs
- validates before publication
- computes the SHA-256 artifact
- records catalog-visible generator status summary and per-provider discovery
  status for fresh versus fallback-backed entries
- records run-specific workflow evidence separately from adopted catalog bytes
- publishes public files to:
  - `https://docs.duumbi.dev/model-catalog/v1/model-catalog.v1.json`
  - `https://docs.duumbi.dev/model-catalog/v1/model-catalog.v1.sha256`

Publication likely crosses repositories because `docs.duumbi.dev` is served
from `hgahub/duumbi-web`. If Stage 10 lacks a configured cross-repo publication
token, split the work into:

- DUUMBI repo generator/validator workflow evidence
- docs-site PR adding or updating static catalog files and docs

Closure must verify the public URLs, not just local generation.

### Studio Provider Management

Studio should expose the same catalog update state and controls as the CLI/TUI
provider-management path:

- current installed catalog hash/version
- remote changed hash notification
- approve, skip, remind later, disable, and frequency controls
- validation/failure messages without secrets
- keyboard-reachable controls
- cancel behavior that leaves the active catalog unchanged

Studio provider list APIs must use canonical provider keys. `Debug`-lowercase
serialization should be replaced or wrapped if it would emit legacy names or
provider variants that are not canonical.

### Documentation

Repository docs and public docs must align:

- accepted providers table with display name, environment variable, and config
  key
- `xai` as canonical
- `grok` only as a legacy alias/migration note if implemented
- OpenRouter removed from v1 examples and supported-provider docs
- catalog update behavior
- local storage paths
- hash-first checks and user controls
- offline/failure fallback
- privacy and credential boundaries

Public docs under `hgahub/duumbi-web/docs` are required product evidence even if
the implementation PR in this repo only prepares source behavior.

## Invariants

- No catalog update is adopted without explicit user approval.
- The embedded catalog remains usable when refreshed catalog state is absent or
  invalid.
- Catalog validation fails closed.
- The v1 catalog contains only accepted direct providers.
- OpenRouter is excluded from v1 catalog output, new setup UX, and v1 docs.
- `xai` is canonical; `grok` is compatibility-only if retained.
- Catalog update state is not credential storage.
- Catalog adoption never mutates provider credentials, env-var names, auth-token
  env vars, base URLs, roles, ordering, workspace config, user config, or
  explicit model overrides.
- Hash-first checks are bounded and low-noise.
- Per-run publisher evidence does not change the semantic adoption hash.
- CI tests remain credential-free.
- `/provider` stays keyboard-complete and bounded when provider count exceeds
  8.

## BDD-To-Test Mapping

| Product scenario | Required proof |
| --- | --- |
| V1 catalog contains only accepted direct providers | catalog validation unit test with the exact nine-provider set; provider metadata table test; docs/provider table check |
| OpenRouter is excluded from v1 | validation rejects OpenRouter in v1 catalog; generator fixture containing OpenRouter fails; docs grep/check proves no v1 OpenRouter examples |
| xAI is canonical while grok is compatibility-only | config deserialize/parse/display tests for `xai`; legacy `grok` alias test if retained; provider panel/docs examples use `xai` |
| Startup continues quietly when the installed hash is current | catalog client test with same hash: no JSON download, no user notification, provider config unchanged |
| Startup offers review when a newer hash is available | catalog client/UI state test for changed hash; TUI or Studio render test includes version/timestamp/summary and actions |
| Skipped catalog hash does not keep notifying the user | state test for skipped hash suppressing repeated notification until a new hash appears |
| Remind later defers notification | state test for remind-later timestamp before and after expiry |
| Approved catalog update is adopted atomically | store test writes valid catalog/hash/state through temp files and verifies future active catalog uses refreshed data |
| Hash mismatch fails closed | client/store test with mismatched JSON/hash keeps previous local or embedded catalog and records safe failure |
| Unsupported schema version fails closed | schema parser test rejects unsupported version and leaves startup usable |
| Canceling review leaves the active catalog unchanged | TUI/Studio state test cancels review and verifies active hash/catalog unchanged and focus returns |
| Stale approval is revalidated before adoption | test where reviewed hash changes before approval; adoption aborts unless the approved bytes/hash still match |
| Invalid provider/model fields fail closed | validation table tests for missing fields, invalid lifecycle, invalid scores, unsupported providers, empty model IDs |
| Partial local write does not corrupt the active catalog | temp-dir store test simulates failed write/rename and verifies previous valid catalog or embedded fallback remains active |
| Catalog adoption does not overwrite provider setup | config snapshot test before/after adoption for roles, order, env vars, auth token env vars, base URLs, key storage, model overrides |
| Legacy grok config remains explicit compatibility behavior | config load test for `grok` alias or unsupported migration state; no silent rewrite to user config without action |
| Scheduled publisher emits deterministic artifacts | generator fixture test proves same semantic input yields identical JSON bytes and SHA-256; run evidence changes do not affect catalog hash |
| Provider discovery outage uses accepted fallback metadata when valid | publisher fixture test with one discovery failure and curated/previous fallback still validates; public catalog includes generator status summary, per-provider discovery status, fallback provenance, and user-facing stale/fallback warning; run artifact includes operational warning evidence |
| Provider discovery outage blocks publication when no valid fallback exists | publisher fixture test fails generation/publication when required provider lacks discovery and fallback |
| Provider docs describe accepted setup details | docs tests or grep assertions verify all nine providers, env vars, and config keys; OpenRouter absent; `xai` canonical |
| Catalog update docs explain user controls and fallback | docs check for approval, skip, remind later, disable/frequency, local storage, validation, offline behavior, embedded fallback |
| Review controls are keyboard accessible | TUI/Studio tests for keyboard navigation, `Esc` cancel, focus return, and no adoption on cancel |

Additional Stage 8 prompt requirement:

| Technical scenario | Required proof |
| --- | --- |
| `/provider` list scrolls when provider count exceeds 8 | `src/cli/app.rs` focused state-machine/render tests with 9+ providers prove only 8 rows are visible, selected row scrolls into view, footer/status rows remain visible, `Esc` closes, and no provider row overlaps the prompt/status dock |

## Live E2E Plan

CI must use mocks and fixtures. Live provider calls are manual Stage 10 evidence
only and must use a temporary DUUMBI home/workspace.

Manual live path A: provider panel and one real direct provider

1. Build the debug binary.
2. Set `HOME` to a temporary directory and run DUUMBI in a temporary workspace.
3. Launch the REPL/TUI.
4. Open `/provider`.
5. Verify the provider list shows the accepted direct-provider set, not
   OpenRouter, and scrolls when the selected provider moves beyond row 8.
6. Configure one available direct provider through the panel.
7. Run `[T] Test connection`.
8. Verify success/failure output contains no secret material and does not write
   workspace graph/source files.
9. Press `Esc` and verify the panel closes and the prompt remains usable.

Manual live path B: catalog update against local test URLs

1. Serve a valid `model-catalog.v1.json` and `.sha256` from a local HTTP server
   or controlled test endpoint.
2. Configure DUUMBI's catalog URL override or test hook to use that endpoint.
3. Open the provider-management surface.
4. Approve the changed catalog.
5. Verify `~/.duumbi/model-catalog/current.json`, `current.sha256`, and
   `state.json` are written atomically under the temporary home.
6. Replace the catalog with a corrupt or hash-mismatched artifact.
7. Verify the previous valid catalog remains active.

Manual live path C: one provider-backed smoke

Use one configured provider key only when available. Recommended maximum:

- 1 provider
- 1 connection probe
- 1 minimal query or provider test request
- estimated cost under USD $0.05

If a provider requires more than one or two live calls to diagnose endpoint/auth
compatibility, stop and request human approval before continuing.

## Ralph Cycle Protocol

Ralph cycles for this issue should be narrow and evidence-oriented. Default
maximum without additional human approval: 3 autonomous cycles.

Cycle 1: Provider taxonomy and embedded catalog schema

- provider metadata source of truth
- `xai` canonical parsing/display
- legacy `grok` behavior
- OpenRouter v1 exclusion
- schema validation fixtures
- embedded catalog fallback shape

Cycle 2: Catalog publisher, storage, and client adoption

- deterministic generator inputs
- separate per-run evidence from adoption catalog bytes
- SHA-256 artifact
- user-level catalog store
- hash-first client checks
- approve/skip/remind/disable state
- atomic adoption and failure fallback

Cycle 3: User surfaces, docs, and live evidence

- `/provider` scroll behavior for more than 8 providers
- CLI/provider-management controls
- Studio provider-management controls
- docs updates including `hgahub/duumbi-web/docs`
- manual smoke evidence
- final CI/lint/test closure

Stop and request human approval before:

- adding a new external dependency for provider/catalog logic
- making more than 2 live provider calls in a cycle
- spending more than USD $0.05 on live provider checks
- changing provider behavior outside the approved direct-provider/catalog scope
- removing runtime compatibility for existing `grok` or `openrouter` configs
- requiring cross-repo credentials or deploy tokens not already available
- exceeding 3 Ralph cycles without a review-clean implementation PR

## Cycle Budget

- Maximum autonomous cycles: 3
- Maximum live provider calls without approval: 2 total
- Maximum live provider spend without approval: USD $0.05 total
- Maximum GitHub workflow reruns without approval: 2 per PR after an initial
  failure diagnosis
- No live provider calls in CI
- No Greptile invocation unless a developer explicitly requests manual deep
  review

## Task Breakdown

1. Provider metadata and config compatibility
   - Create or centralize canonical provider metadata.
   - Add accepted provider keys.
   - Make `xai` canonical.
   - Define and test `grok` compatibility or migration behavior.
   - Keep OpenRouter out of v1 support lists while preserving explicit legacy
     handling.

2. Runtime provider adapters
   - Verify each new provider's API contract.
   - Add factory branches and provider modules only when verified.
   - Add mock-HTTP tests.
   - Keep endpoint/auth assumptions documented and reviewable.

3. Catalog schema and validation
   - Define v1 catalog document types.
   - Add strict validation.
   - Convert embedded static entries into the same active catalog abstraction.
   - Preserve current routing score semantics.

4. Catalog store and client
   - Implement user-level storage.
   - Implement hash-first remote check.
   - Implement update-control state.
   - Implement atomic adoption and fallback.

5. Publisher
   - Add deterministic generator.
   - Add curated metadata fixtures.
   - Add scheduled/manual workflow.
   - Add separate workflow evidence artifact.
   - Publish to docs static path or prepare docs-site publication PR.

6. CLI and TUI
   - Add catalog review/control path.
   - Update provider setup to canonical provider metadata.
   - Implement `/provider` scroll window for more than 8 providers.
   - Add focused render/state-machine tests.

7. Studio
   - Expose catalog update state and controls.
   - Replace ad hoc provider key serialization where necessary.
   - Add keyboard/focus tests.

8. Docs
   - Update repo references.
   - Update public docs in `hgahub/duumbi-web/docs`.
   - Verify OpenRouter removal from v1 docs and `xai` canonical naming.

9. Evidence
   - Run formatting/lints/tests relevant to touched code.
   - Run manual TUI smoke.
   - Run local catalog update smoke.
   - Run one optional live provider smoke only within the budget above.

## Verification Plan

Minimum local verification for implementation PR:

- `cargo fmt --check`
- `cargo test --all`
- focused catalog schema/store/client tests
- focused provider config/provider metadata tests
- focused `src/cli/app.rs` provider panel scroll tests
- focused provider startup discovery tests
- focused Studio provider-management tests when Studio code changes
- docs grep/check for accepted providers, `xai`, and OpenRouter removal from v1
  docs/examples

Recommended additional verification:

- `cargo clippy --all-targets -- -D warnings`
- generator determinism test run twice with the same fixtures
- local HTTP catalog update smoke with valid, corrupt, and hash-mismatched
  artifacts
- manual `/provider` TUI smoke at desktop terminal size and a narrower terminal
  size
- public URL verification after catalog publication is wired

Manual smoke evidence should record:

- command(s) run
- temporary HOME/workspace path shape, without secrets
- provider selected, if any
- whether live provider call was made
- live call count and estimated cost
- result summary
- screenshots or terminal transcript snippets only when they contain no secrets

## Completion Criteria

Stage 10 implementation can be considered ready for review only when:

- all BDD scenarios have mapped proof
- `/provider` scroll behavior is tested for more than 8 providers
- OpenRouter is absent from v1 catalog output and new docs/setup lists
- `xai` is canonical in new behavior
- legacy `grok` and OpenRouter config behavior is explicit and tested
- catalog adoption cannot mutate provider config or credentials
- catalog-visible generator status summary, per-provider discovery status, and
  fallback provenance are present when accepted fallback metadata is adopted
- run-specific publisher evidence is separated from deterministic adoption
  catalog bytes
- user-level catalog update state is atomic and recoverable
- CI is green
- manual smoke evidence covers TUI and catalog update behavior
- public docs/static publication path is verified or explicitly blocked with a
  cross-repo follow-up before closure

## Failure And Escalation

Escalate before implementation proceeds if:

- a required direct provider cannot satisfy DUUMBI's `LlmProvider` contract
  without a product scope change
- provider docs contradict the accepted config key/env-var table
- docs-site publication requires credentials or repo permissions unavailable to
  the implementation agent
- schema/hash requirements conflict with the need to expose publisher evidence
- preserving legacy `grok` or OpenRouter configs proves unsafe or materially
  more complex than expected
- the TUI provider panel cannot fit eight rows plus footer/status controls at
  supported terminal sizes without a broader panel layout change

## Open Questions

No Stage 8-blocking questions.

Stage 10 agents must verify current provider API endpoint/auth details before
shipping runtime adapters for DeepSeek, Qwen/DashScope, Moonshot, Zhipu, and
Gemini. If any provider cannot be verified or tested safely, pause for a scope
decision rather than weakening the v1 direct-provider contract.
