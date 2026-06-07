# DUUMBI-675: Scheduled Provider Model Catalog Refresh With User Approval

## Summary

Define the product behavior for keeping DUUMBI's provider model catalog current
without silently changing a user's provider setup or model-routing behavior.

DUUMBI currently embeds a static model catalog in the source release. That
catalog is useful because model choice stays internal to DUUMBI's routing policy,
but it can become stale faster than DUUMBI releases. This issue introduces a
versioned remote catalog update path for accepted direct providers, plus a
user-approved local adoption flow.

For v1, the accepted product surface is:

```text
curated direct-provider metadata -> published catalog artifact ->
bounded local hash check -> user review and approval ->
validated atomic local catalog cache -> embedded catalog fallback
```

This spec also fixes provider naming and scope for v1. The supported direct
providers are Anthropic, OpenAI, xAI, MiniMax, DeepSeek, Alibaba Cloud Model
Studio (Qwen), Moonshot AI (Kimi), Zhipu AI (GLM), and Google Gemini.
OpenRouter is not part of the v1 supported provider catalog because it is an
aggregator rather than a stable direct-provider surface.

The execution issue must remain open after this spec PR. This PR is
specification-only and is related to #675; it is not completion evidence for the
execution work.

## Problem

Provider model IDs, availability, lifecycle state, price signals, and routing
preferences change faster than DUUMBI releases. A static compiled catalog creates
several product risks:

- DUUMBI may route toward retired or deprecated models.
- Newly supported direct-provider models cannot be considered until a full
  release.
- Provider setup and documentation can drift from supported model metadata.
- Users may need to maintain explicit model overrides even though DUUMBI's UX
  should keep model choice mostly internal.
- A remote update path, if designed poorly, could silently alter routing,
  overwrite user choices, or weaken credential boundaries.

The current provider scope also needs cleanup. `grok` is a model/product label
under xAI and should not remain the canonical provider name. OpenRouter should
not be presented as a v1 supported provider because DUUMBI wants predictable
direct-provider routing.

The product need is a conservative update path that keeps metadata fresh while
preserving user control, credential safety, deterministic fallback behavior, and
auditable catalog integrity.

## Outcome

When this issue is implemented:

- DUUMBI has a documented versioned model catalog artifact for v1 provider and
  routing metadata.
- A scheduled and manually dispatchable publisher can produce deterministic
  `model-catalog.v1.json` and `model-catalog.v1.sha256` artifacts.
- The catalog is published at the accepted stable docs URLs:
  - `https://docs.duumbi.dev/model-catalog/v1/model-catalog.v1.json`
  - `https://docs.duumbi.dev/model-catalog/v1/model-catalog.v1.sha256`
- Local DUUMBI clients check for a changed remote catalog hash at most once per
  day by default.
- A changed hash produces a concise user-facing notification with catalog
  version, generation time, and change summary.
- Users can approve the update, skip that version, remind later, disable checks,
  or adjust the check frequency according to the implemented settings surface.
- Approved updates verify SHA-256, validate catalog schema, validate
  provider/model fields, and write local catalog files atomically.
- Failed checks, failed downloads, corrupted artifacts, unsupported schema
  versions, invalid provider/model entries, partial writes, offline startup, and
  timeouts keep the previous local catalog or embedded static catalog.
- The embedded catalog remains a deterministic fallback.
- Existing credentials, API key environment variable names, auth token
  environment variable names, custom base URLs, provider roles, provider order,
  explicit legacy model overrides, and workspace/user config are not silently
  changed by a catalog update.
- `xai` is the canonical config key and user-facing provider name for xAI/Grok
  models.
- Existing `grok` configs are handled only as a read-compatible legacy alias or
  explicit migration path, not as the canonical name in new UX or docs.
- OpenRouter is removed from the v1 supported provider catalog and v1 docs.
- Provider docs identify each accepted provider by display name, environment
  variable, and config key.
- Catalog discovery and routing-policy curation are treated as separate inputs:
  provider APIs can inform availability and direct model IDs, while DUUMBI
  curated metadata remains authoritative for routing scores and policy.
- The execution issue remains open for Stage 7 review, Stage 8 technical spec,
  Stage 9 review, implementation, Stage 11 review, and Stage 12 closure.

## Scope

### In Scope

- Define the v1 product contract for the remote provider model catalog.
- Define the accepted v1 provider set and canonical provider naming.
- Exclude OpenRouter from v1 provider catalog and docs.
- Define user-visible catalog refresh behavior, controls, states, and failure
  outcomes.
- Define catalog publication URLs under `docs.duumbi.dev`.
- Require a deterministic catalog artifact and SHA-256 hash artifact.
- Require generated timestamp, source workflow or commit evidence, schema
  version, provider identity, model lifecycle state, routing metadata, and
  user-facing changelog summary in the catalog artifact.
- Require provider discovery and DUUMBI routing scores to stay separate inputs.
- Require client-side hash-first checking, approval before adoption, schema and
  hash validation, provider/model validation, and atomic local cache updates.
- Require embedded static catalog fallback for remote failure, validation
  failure, unsupported version, offline startup, or absent local catalog.
- Require local update state to live separately from provider credentials and
  workspace configuration.
- Require CLI startup and Studio/provider-management notification surfaces.
- Require documentation updates for accepted providers and catalog refresh
  behavior.
- Require tests and manual checks that prove the BDD scenarios and fallback
  behavior without live provider credentials in CI.

### Explicitly Out Of Scope

- Technical specification content or implementation instructions during Stage 6.
- Implementation code, source changes outside this product spec, generated
  catalog files, workflow files, docs files, or tests during Stage 6.
- Ralph cycles or implementation coordination.
- Silent catalog adoption.
- Remote mutation of provider credentials, API key environment variables, auth
  token environment variables, custom base URLs, provider roles, provider order,
  workspace config, user config, or explicit model overrides.
- Making the remote catalog required for startup.
- Real-time model catalog updates.
- Automatic provider setup.
- Automatic provider switching without user-approved provider configuration.
- Mandatory live benchmarking in v1.
- Delegating DUUMBI quality, speed, cost-efficiency, routing capability, or
  provider-ordering policy to provider discovery responses alone.
- Keeping OpenRouter in the v1 provider catalog or v1 docs.
- Publishing the v1 public static catalog behind `registry.duumbi.dev` unless a
  separate registry API decision is accepted.
- Detached signing or a stronger authenticity scheme beyond HTTPS plus SHA-256
  in v1.
- MCP-based model telemetry analytics. That is a related follow-up idea, not
  part of this catalog refresh issue.
- Product spec approval, technical spec drafting, implementation, or closure.

## Constraints And Assumptions

Facts:

- Issue #675 is open.
- Issue #675 is labeled `accepted` and `needs-spec`.
- The Stage 5 decision comment on 2026-06-06 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- The issue body defines the accepted v1 provider set as Anthropic, OpenAI, xAI,
  MiniMax, DeepSeek, Alibaba Cloud Model Studio (Qwen), Moonshot AI (Kimi),
  Zhipu AI (GLM), and Google Gemini.
- The issue body explicitly excludes OpenRouter from v1.
- The issue body accepts weekly scheduled catalog generation plus manual
  dispatch and daily client hash checks by default.
- The issue body accepts `docs.duumbi.dev` static URLs for v1 catalog
  publication.
- The issue body accepts HTTPS plus SHA-256 as sufficient v1 integrity and
  transport protection.
- The issue body accepts user-level catalog storage under
  `~/.duumbi/model-catalog/`.
- The issue body identifies CLI startup and Studio/provider-management surfaces
  as notification surfaces.
- The source repository currently has a static catalog in
  `src/agents/model_catalog.rs`.
- The current static catalog includes Anthropic, OpenAI, Grok, MiniMax, and
  OpenRouter entries.
- `src/config.rs` currently defines provider kinds for Anthropic, OpenAI, Grok,
  OpenRouter, and MiniMax.
- `src/cli/provider_startup.rs` currently maps `XAI_API_KEY` to the `Grok`
  provider kind and includes `OPENROUTER_API_KEY`.
- Model access metadata is persisted separately from model performance telemetry.
- Model access metadata is user-level and keyed by a non-reversible credential
  fingerprint.
- Model performance telemetry is workspace-level and records provider/model
  usage and outcome metadata.
- Related closed issue #308 added provider config support.
- Related closed issue #484 added Studio provider management.
- Related closed issue #610 added workflow metrics context; it does not replace
  this catalog refresh issue.
- The active DUUMBI runbook requires Codex self-review, actual required
  automated reviewer evidence, clean checks, and resolved review threads for
  file-based spec gates.
- Copilot is the default required automated reviewer for file-based specs in
  this repository.
- Greptile is manual-only and must not be invoked for this spec unless a
  developer explicitly requests a manual deep review.

Assumptions:

- A user-level catalog cache is the correct scope because provider/model routing
  applies to the user's DUUMBI installation rather than one workspace.
- A remote catalog update can affect routing outcomes, so user approval is
  required before adoption.
- Hash-first checking is the right default because most startups should not
  download the full catalog.
- Daily client checks and weekly catalog generation balance freshness against
  notification noise.
- DUUMBI should preserve the current provider setup UX principle: provider
  credentials and provider setup remain user-controlled, while model choice
  stays mostly internal to catalog and routing policy.
- CI should not require live provider credentials to prove the catalog contract.
  Live provider checks can be optional manual evidence when safe.
- Provider APIs may be incomplete, rate-limited, inconsistent, or unavailable,
  so the publisher needs deterministic validation and previous-known-good or
  curated fallback behavior.
- Detached signing can be added later if the accepted threat model requires
  stronger publisher authenticity.

Constraints:

- The catalog update path must fail closed.
- The catalog update path must never block ordinary startup indefinitely.
- The catalog update path must not expose, copy, overwrite, or infer secrets.
- Catalog metadata must not include provider credentials, raw prompts, model
  completions, provider payloads, or user telemetry.
- Catalog update state must be separate from provider credential storage.
- Unsupported catalog schema versions must not be adopted.
- OpenRouter must not appear as a v1 supported provider in catalog output, new
  examples, provider setup guidance, or provider docs.
- New UX and docs must use `xai`, not `grok`, as the canonical provider key.
- Legacy `grok` support, if kept, must be explicit compatibility behavior.
- The embedded catalog must remain available when no refreshed catalog is
  adopted.
- The spec must not prescribe implementation internals that belong in Stage 8.

## Decisions

- **Decision:** Use a file-based product spec for #675.
  **Evidence:** The work is user-visible, cross-module, architectural, and
  durable. It spans provider configuration, model catalog metadata, scheduled
  publication, local state, CLI startup, Studio, docs, failure behavior, and
  test evidence.

- **Decision:** The v1 provider set is direct-provider only.
  **Evidence:** Stage 5 accepted the issue body listing Anthropic, OpenAI, xAI,
  MiniMax, DeepSeek, Alibaba Cloud Model Studio (Qwen), Moonshot AI (Kimi),
  Zhipu AI (GLM), and Google Gemini. The issue rationale excludes OpenRouter
  because backend provider/model selection behind an aggregator can be unstable.

- **Decision:** `xai` is canonical; `grok` is compatibility-only if retained.
  **Evidence:** The accepted issue body states that Grok is a model/product name
  under xAI and that new docs and config examples should use `xai`.

- **Decision:** Publish v1 catalog artifacts under `docs.duumbi.dev`.
  **Evidence:** The accepted issue body records infrastructure inspection showing
  `docs.duumbi.dev` is the existing Azure Static Web App publication surface,
  while `registry.duumbi.dev` is the module registry service.

- **Decision:** Use HTTPS plus SHA-256 for v1 integrity checks.
  **Evidence:** The Stage 5-accepted issue body explicitly accepts this for v1,
  with detached signing left as a possible follow-up if the threat model changes.

- **Decision:** Require user approval before local adoption.
  **Evidence:** The issue goal is not silent model hot-swapping. A catalog update
  can change routing decisions, so approval is the product trust boundary.

- **Decision:** Keep provider discovery and DUUMBI routing policy separate.
  **Evidence:** Provider APIs can report model IDs and availability, but DUUMBI
  owns model quality, speed, cost-efficiency, routing capabilities, and policy.

- **Decision:** Keep refreshed catalog state separate from credentials and
  workspace config.
  **Evidence:** The accepted issue body recommends `~/.duumbi/model-catalog/`
  and states that routing metadata is user-installation state, not credential
  storage or workspace config.

- **Decision:** Preserve the embedded catalog as fallback.
  **Evidence:** Offline startup, remote failures, invalid catalog data, and
  unsupported schema versions must not make DUUMBI unusable.

- **Decision:** Treat MCP telemetry analytics as a separate follow-up.
  **Evidence:** The 2026-06-06 Inbox note captures MCP model telemetry analytics
  as adjacent to #675 but separate because it concerns querying observed local
  model usage, not publishing or adopting catalog metadata.

## Behavior

### Provider Set And Naming

V1 supported direct providers:

| Provider | Environment Variable | Config Key |
|---|---|---|
| Anthropic | `ANTHROPIC_API_KEY` | `anthropic` |
| OpenAI | `OPENAI_API_KEY` | `openai` |
| xAI | `XAI_API_KEY` | `xai` |
| MiniMax | `MINIMAX_API_KEY` | `minimax` |
| DeepSeek | `DEEPSEEK_API_KEY` | `deepseek` |
| Alibaba Cloud Model Studio (Qwen) | `DASHSCOPE_API_KEY` | `qwen` |
| Moonshot AI (Kimi) | `MOONSHOT_API_KEY` | `moonshot` |
| Zhipu AI (GLM) | `ZHIPUAI_API_KEY` | `zhipu` |
| Google Gemini | `GEMINI_API_KEY` | `gemini` |

Provider behavior:

- New provider setup UX, config examples, docs, and catalog entries use the
  canonical config keys above.
- `xai` is the canonical provider key and user-facing provider name for xAI
  models.
- `grok` can remain only as a legacy alias or migration path if compatibility
  requires it.
- When legacy `grok` config is accepted, DUUMBI should explain or expose that
  `xai` is canonical for new configuration.
- OpenRouter is not included in v1 supported provider docs, new examples,
  startup auto-detection, or refreshed catalog entries.
- Removing OpenRouter from v1 support must not silently delete a user's existing
  config. If a compatibility warning or unsupported-provider state is needed,
  that behavior must be explicit and user-visible.

### Catalog Artifact

The v1 catalog artifact is named `model-catalog.v1.json`.

The matching hash artifact is named `model-catalog.v1.sha256`.

The catalog must include, at minimum:

- catalog schema version
- generated timestamp
- source commit, workflow run ID, or equivalent generation evidence
- generator status summary
- provider display name
- canonical provider config key
- API key environment variable name
- provider/model entries
- model ID
- lifecycle state: `active`, `deprecated`, or `retired`
- routing metadata compatible with DUUMBI's model-routing concepts, including
  quality, speed, cost-efficiency, reasoning capability, and coding capability
- source and curation provenance sufficient for reviewer inspection
- user-facing changelog summary
- provider discovery status, including warnings when a provider used curated or
  previous-known-good metadata because live discovery was unavailable

Catalog behavior:

- JSON output is deterministic for the same inputs.
- The SHA-256 hash is computed over the exact published JSON bytes.
- Downloading a changed catalog for review is not adoption. The catalog becomes
  active only after user approval and successful adoption validation.
- Provider-reported model discovery and DUUMBI-curated routing metadata are
  represented as separate inputs or provenance.
- A provider API outage does not automatically invalidate the whole catalog when
  previous-known-good or curated data is available and the resulting catalog is
  still valid.
- Publication fails rather than publishing incomplete unsupported data when a
  required provider has neither current discovery nor previous-known-good or
  curated metadata.
- Catalog entries never include secrets, credential fingerprints, user-specific
  telemetry, raw prompts, model completions, provider response payloads, or
  private local state.

### Scheduled Publisher

The v1 publisher behavior is:

- weekly scheduled generation by default
- manual dispatch for maintainers
- accepted direct-provider coverage only
- OpenRouter excluded
- deterministic JSON artifact generation
- schema validation before publication
- SHA-256 generation from the final JSON artifact
- publication to the accepted static docs paths
- workflow evidence for generation time, source commit or workflow run,
  provider fetch status, validation status, and warnings

The publisher should make provider metadata freshness visible without creating
notification churn for users.

### Stable Publication URL

Accepted v1 URLs:

- `https://docs.duumbi.dev/model-catalog/v1/model-catalog.v1.json`
- `https://docs.duumbi.dev/model-catalog/v1/model-catalog.v1.sha256`

Behavior:

- The local client fetches the hash first.
- The full catalog is downloaded only after a changed hash is detected and the
  implemented UX needs catalog details for review or adoption.
- A downloaded review candidate must not become active until the user approves
  it and all adoption validation succeeds.
- `registry.duumbi.dev` is not the v1 catalog publication target.

### Client-Side Check

The default client check runs at most once per day.

Required client behavior:

- Read local catalog update state before checking.
- Honor disabled checks and configured check frequency if those settings are
  implemented.
- Fetch only the remote hash before downloading the catalog.
- Use short, bounded timeouts so startup remains usable.
- If the hash is unchanged, continue startup quietly.
- If a skipped hash is still current, do not repeat the same notification until
  the user changes the decision or a later hash appears.
- If a remind-later decision is still active, defer notification until the
  remind-later time expires.
- If the hash changed and notification is allowed, show a concise update summary
  with version, generated timestamp, and user-facing change summary.
- Let the user approve, skip this version, remind later, disable checks, or
  adjust check frequency according to the implemented settings surface.
- Canceling or dismissing the review surface does not adopt a catalog and must
  leave the active catalog unchanged.
- On approval, download the catalog, verify SHA-256, validate schema, validate
  provider/model fields, and write the catalog atomically.
- If the user approves a stale hash after a newer hash appears, DUUMBI must
  revalidate the selected hash before adoption and must not accidentally adopt a
  different catalog than the user reviewed.
- If any step fails, keep using the previous local catalog or embedded fallback.
- The check must not write workspace graph files, provider credentials, provider
  config, or user model overrides.

### Local Storage

Refreshed catalog state is user-level installation state.

Recommended local storage shape:

```text
~/.duumbi/model-catalog/current.json
~/.duumbi/model-catalog/current.sha256
~/.duumbi/model-catalog/state.json
```

`state.json` tracks the update-control state needed for the user experience,
such as:

- last check timestamp
- last seen hash
- installed hash
- last offered hash
- skipped hash
- remind-later timestamp
- disabled checks or configured check frequency, if implemented

Storage behavior:

- Catalog storage is separate from `~/.duumbi/credentials.toml`.
- Catalog storage is separate from workspace `.duumbi/config.toml`.
- Atomic writes prevent partial catalog adoption.
- A corrupt local catalog causes fallback to the embedded catalog or previous
  valid local catalog, not a startup failure.
- Local catalog state does not include secrets.

### Notification And Controls

User-facing notification behavior:

- No notification appears when the hash is unchanged.
- No notification appears for ordinary startup if the check is disabled.
- A changed catalog notification is concise and action-oriented.
- The notification summarizes meaningful provider/model/routing changes rather
  than dumping raw JSON.
- Approval clearly states that local routing metadata may change after adoption.
- Skip applies to the specific hash.
- Remind later defers the same hash until the chosen or default reminder time.
- Failure to check or download is quiet or low-noise unless the user explicitly
  opened the provider/catalog surface.

Required surfaces:

- CLI startup notification or equivalent low-interruption command-line surface.
- Studio/provider-management surface for reviewing and applying catalog updates.

Accessibility and focus behavior:

- Catalog update controls are keyboard-reachable.
- `Esc` or an equivalent cancel action closes the active review surface without
  adoption.
- Focus returns to the invoking surface after cancel, skip, remind later, or
  approval.
- Button labels or control names must make the adoption consequence clear.

The exact UI layout belongs to Stage 8, but the product behavior must preserve
quiet startup and user control.

### Config Safety

Catalog adoption must not change:

- provider credentials
- API key environment variable names
- auth token environment variable names
- custom base URLs
- provider roles
- provider ordering
- user or workspace provider configuration
- explicit model overrides
- legacy provider aliases unless the user explicitly performs a migration

Catalog adoption may affect:

- the catalog entries DUUMBI considers for routing when no explicit model
  override prevents catalog selection
- provider/model availability metadata used by routing
- lifecycle warnings for deprecated or retired catalog models
- provider setup guidance and documentation

### Failure States

DUUMBI must remain usable when:

- the remote hash URL is unavailable
- the remote catalog URL is unavailable
- DNS, TLS, HTTP, timeout, or proxy failures occur
- the hash does not match the downloaded catalog
- the catalog schema version is unsupported
- required fields are missing
- a provider or model entry is invalid
- OpenRouter appears in the v1 catalog
- a required direct provider has no valid entry or accepted fallback metadata
- local write fails
- a local partial file is present from a previous interrupted write
- the catalog check exceeds its startup budget
- the user is offline
- overlapping check/adoption attempts occur

In these cases DUUMBI keeps using the previous valid local catalog or embedded
catalog and records only safe diagnostic information.

### Documentation

Documentation must:

- list the accepted v1 provider set with provider display name, environment
  variable, and config key
- use `xai` as canonical and explain any `grok` compatibility behavior
- remove OpenRouter from v1 examples and supported-provider docs
- explain the catalog update flow and user controls
- document the stable catalog URLs
- document local catalog storage paths
- document fallback behavior and offline startup behavior
- document that provider credentials and explicit config are not overwritten
- document privacy and security boundaries
- update existing provider references in the public docs areas identified by
  the issue body, including config, installation, and introduction references

### Invariants

- User approval is required before a changed remote catalog is adopted.
- Catalog update checks are bounded and low-noise.
- The embedded catalog remains available.
- Catalog validation fails closed.
- Catalog metadata is not credential storage.
- Provider setup remains user-controlled.
- Direct providers are explicit and stable.
- OpenRouter is excluded from v1.
- `xai` is canonical for xAI.
- Stage 6 produces only this product spec artifact.

## BDD Scenarios

Feature: Safe provider model catalog refresh

Rule: V1 provider scope is direct-provider only

Scenario: V1 catalog contains only accepted direct providers
Given DUUMBI has a v1 provider catalog
When the catalog is generated or validated
Then the provider set includes Anthropic, OpenAI, xAI, MiniMax, DeepSeek, Alibaba Cloud Model Studio (Qwen), Moonshot AI (Kimi), Zhipu AI (GLM), and Google Gemini
And each provider has a display name, canonical config key, and API key environment variable
And the catalog passes provider-set validation

Scenario: OpenRouter is excluded from v1
Given a generated v1 catalog candidate includes OpenRouter
When DUUMBI validates the catalog candidate
Then validation fails
And the candidate is not published or adopted
And the user-facing supported-provider docs do not list OpenRouter as a v1 provider

Scenario: xAI is canonical while grok is compatibility-only
Given a user reviews provider setup guidance
When xAI/Grok models are described
Then the provider display name is xAI
And the config key is `xai`
And `XAI_API_KEY` is the environment variable
And any `grok` support is labeled as legacy compatibility rather than the new canonical provider name

Rule: Remote checks are bounded and low-noise

Scenario: Startup continues quietly when the remote hash is unchanged
Given DUUMBI checked the model catalog hash within the allowed schedule
And the remote hash matches the last seen or installed hash
When DUUMBI starts
Then startup continues without a catalog notification
And no catalog JSON is downloaded
And provider configuration remains unchanged

Scenario: Startup offers review when a newer hash is available
Given DUUMBI is allowed to check the catalog
And the remote hash differs from the installed hash
And the hash is not currently skipped or deferred
When DUUMBI starts or the provider-management surface opens
Then the user sees a concise catalog update notification
And the notification includes catalog version, generated timestamp, and change summary
And the user can approve, skip this version, remind later, disable checks, or adjust check frequency according to the implemented surface

Scenario: Skipped catalog hash does not keep notifying the user
Given the user skipped catalog hash `H1`
And the remote hash is still `H1`
When DUUMBI starts again before a later hash appears
Then DUUMBI does not show the same update notification again
And the embedded or previously installed catalog remains active

Scenario: Remind later defers notification
Given the user chose remind later for catalog hash `H1`
And the remind-later timestamp has not expired
When DUUMBI starts
Then DUUMBI does not show the `H1` update notification
And no local catalog adoption occurs

Rule: Adoption is explicit and validated

Scenario: Approved catalog update is adopted atomically
Given the user approves a changed catalog hash
When DUUMBI downloads the catalog
And the downloaded bytes match the published SHA-256
And the catalog schema is supported
And every provider and model entry validates
Then DUUMBI writes `current.json`, `current.sha256`, and update state atomically
And future routing may use the refreshed catalog
And provider credentials and provider config are unchanged

Scenario: Hash mismatch fails closed
Given the user approves a catalog update
When the downloaded catalog bytes do not match the published SHA-256
Then DUUMBI rejects the downloaded catalog
And DUUMBI keeps using the previous local catalog or embedded fallback
And DUUMBI records safe diagnostic evidence without secrets

Scenario: Unsupported schema version fails closed
Given a downloaded catalog has an unsupported schema version
When DUUMBI validates the catalog
Then DUUMBI rejects the catalog
And startup remains usable
And the user is not asked to adopt that unsupported catalog as if it were valid

Scenario: Canceling review leaves the active catalog unchanged
Given DUUMBI shows a catalog update review surface
When the user cancels or dismisses the review
Then DUUMBI does not adopt the catalog
And the active catalog remains unchanged
And focus returns to the invoking surface

Scenario: Stale approval is revalidated before adoption
Given the user is reviewing catalog hash `H1`
And a newer remote hash `H2` appears before adoption completes
When the user approves the reviewed catalog
Then DUUMBI revalidates that the selected hash still matches the reviewed catalog
And DUUMBI does not adopt a different catalog than the user approved

Scenario: Invalid provider/model fields fail closed
Given a downloaded catalog has missing fields, invalid lifecycle values, invalid routing scores, or an unsupported provider
When DUUMBI validates the catalog
Then DUUMBI rejects the catalog
And the previous local catalog or embedded catalog remains active

Scenario: Partial local write does not corrupt the active catalog
Given DUUMBI is adopting an approved catalog update
When a local write fails before completion
Then DUUMBI does not treat partial files as the active catalog
And the previous valid local catalog or embedded catalog remains active
And the next startup can recover without manual file cleanup

Rule: User configuration and credentials are preserved

Scenario: Catalog adoption does not overwrite provider setup
Given the user has configured providers with roles, environment variables, custom base URLs, auth token variables, and explicit model overrides
When the user approves and adopts a refreshed catalog
Then DUUMBI does not change those provider configuration fields
And DUUMBI does not copy or store provider secrets in the catalog state
And explicit model overrides remain explicit user choices

Scenario: Legacy grok config remains explicit compatibility behavior
Given an existing user config uses `grok`
When DUUMBI loads provider configuration after xAI becomes canonical
Then DUUMBI either accepts `grok` as a documented legacy alias or presents a clear migration/unsupported state
And new docs and examples use `xai`
And no silent provider config rewrite occurs without explicit user action

Rule: Publisher output is deterministic and reviewable

Scenario: Scheduled publisher emits deterministic artifacts
Given the publisher has the same discovery and curated metadata inputs
When it generates `model-catalog.v1.json`
Then repeated generation produces the same JSON bytes
And the SHA-256 artifact matches those bytes
And workflow evidence records generation time, source commit or workflow run, provider fetch status, validation status, and warnings

Scenario: Provider discovery outage uses accepted fallback metadata when valid
Given one provider discovery call is unavailable
And previous-known-good or curated metadata exists for that provider
When the publisher builds the catalog
Then the catalog may still publish if validation passes
And provider discovery warnings are included in generation evidence
And the user-facing changelog does not claim fresh discovery for that provider

Scenario: Provider discovery outage blocks publication when no valid fallback exists
Given a required direct provider has neither current discovery data nor previous-known-good or curated metadata
When the publisher builds the catalog
Then publication fails
And no incomplete v1 catalog is published

Rule: Documentation and review evidence match the product behavior

Scenario: Provider docs describe accepted setup details
Given the docs are updated for v1 provider support
When a user reads provider setup documentation
Then each accepted provider lists Provider, Environment Variable, and config key
And OpenRouter is absent from v1 examples
And xAI uses the `xai` config key

Scenario: Catalog update docs explain user controls and fallback
Given the docs are updated for catalog refresh behavior
When a user reads the catalog update documentation
Then the docs explain hash-first checks, user approval, skip, remind later, disable or frequency controls if available, local storage, validation, offline behavior, and embedded fallback
And the docs state that credentials and explicit provider config are not overwritten

Scenario: Review controls are keyboard accessible
Given a catalog update review surface is shown in Studio or another interactive surface
When the user navigates with the keyboard
Then approval, skip, remind later, disable, and cancel controls are reachable
And `Esc` or an equivalent cancel action closes the review without adoption
And focus returns to the invoking surface

## Tasks

The implementation should be broken down into independently reviewable slices.
Stage 8 owns the technical design and exact file mapping, but product-level
work should cover:

1. Catalog schema and validation contract
   - Define the v1 catalog shape and validation behavior.
   - Include provider identity, model lifecycle, routing metadata, generation
     evidence, changelog summary, and provider discovery status.
   - Prove unsupported schema versions and invalid entries fail closed.

2. Provider set and naming update
   - Add the accepted direct-provider set to DUUMBI's provider model.
   - Make `xai` canonical.
   - Treat `grok` only as compatibility or migration behavior if retained.
   - Exclude OpenRouter from v1 support and new docs.

3. Catalog publisher
   - Generate deterministic JSON from accepted discovery and curated inputs.
   - Handle provider discovery partial failure with validated fallback behavior.
   - Compute the SHA-256 artifact.
   - Publish to the accepted docs static URLs.
   - Record safe workflow evidence.

4. Local catalog update state
   - Store user-level refreshed catalog files and update-control state under
     `~/.duumbi/model-catalog/`.
   - Keep this separate from credentials and workspace config.
   - Implement atomic writes and recovery from corrupt or partial local state.

5. Client check and adoption flow
   - Check the remote hash at most daily by default.
   - Respect skip, remind-later, disabled checks, and check frequency behavior.
   - Show concise update notifications only when action is useful.
   - Require approval before adoption.
   - Verify hash, schema, provider/model fields, and atomic write before use.

6. CLI and Studio user surfaces
   - Provide a quiet CLI startup surface for update availability.
   - Provide a Studio/provider-management surface for review and adoption.
   - Avoid startup noise for unrelated flows.

7. Documentation
   - Update provider docs for accepted providers.
   - Update config, installation, and introduction references.
   - Document catalog update behavior, local storage, controls, fallback,
     privacy, and security boundaries.

8. Verification and evidence
   - Map each BDD scenario to tests, CI checks, workflow dry runs, docs checks,
     or manual smoke evidence.
   - Keep live provider calls out of CI unless explicitly safe and mocked or
     credential-free.

## Checks

Required proof:

- Product spec review:
  - Codex self-review finds no blocking product/spec issue.
  - Copilot review evidence exists for the file-based spec PR.
  - Required automated reviews are actual reviewer submissions, not only
    reviewer-request workflow success.
  - Review threads are resolved after fixes.
  - Checks are green, neutral, skipped, or explicitly not applicable.

- Catalog schema and artifact checks:
  - Schema validation accepts a valid v1 catalog.
  - Schema validation rejects unsupported schema versions.
  - Schema validation rejects missing required fields.
  - Schema validation rejects invalid lifecycle values.
  - Schema validation rejects unsupported providers, including OpenRouter.
  - Deterministic generation test proves same inputs produce same JSON bytes.
  - SHA-256 generation test proves the hash matches the final JSON bytes.

- Provider set checks:
  - Accepted direct providers parse, display, serialize, and round-trip as
    intended.
  - `xai` is canonical in new config/docs paths.
  - `grok` legacy behavior, if retained, is tested and documented.
  - OpenRouter is absent from v1 catalog output and v1 docs.
  - Provider startup detection covers the accepted provider environment
    variables.

- Publisher checks:
  - Weekly/manual generation path validates before publication.
  - Provider discovery partial failure with valid fallback emits warnings and
    still validates.
  - Missing discovery and missing fallback for a required provider blocks
    publication.
  - Workflow evidence is metadata-only and does not include secrets, raw
    prompts, model completions, provider payloads, or user telemetry.

- Client check and state checks:
  - Daily throttle behavior.
  - Disabled-check behavior.
  - Changed hash behavior.
  - Unchanged hash behavior.
  - Skipped hash behavior.
  - Remind-later behavior.
  - Cancel/dismiss behavior.
  - Approval behavior.
  - Stale hash revalidation behavior.
  - Hash mismatch behavior.
  - Corrupted download behavior.
  - Unsupported schema behavior.
  - Invalid provider/model entry behavior.
  - Offline, timeout, and unavailable URL behavior.
  - Atomic update success.
  - Partial-write recovery.
  - Corrupt local catalog fallback.
  - Embedded catalog fallback.

- Config safety checks:
  - Catalog adoption does not mutate credentials.
  - Catalog adoption does not mutate API key environment variables.
  - Catalog adoption does not mutate auth token environment variables.
  - Catalog adoption does not mutate custom base URLs.
  - Catalog adoption does not mutate provider roles or ordering.
  - Catalog adoption does not mutate explicit model overrides.
  - Catalog adoption does not write workspace graph/source files.

- Documentation checks:
  - Provider docs list each accepted provider with Provider, Environment
    Variable, and config key.
  - Docs use `xai` as canonical.
  - Docs remove OpenRouter from v1 examples.
  - Docs describe update approval, local storage, fallback, controls, and
    privacy/security boundaries.
  - Config, installation, and introduction references are updated.

- Manual smoke expectations:
  - CLI startup remains quiet when no check is due or the hash is unchanged.
  - CLI startup remains usable when remote URLs are unavailable.
  - CLI or provider-management flow shows changed catalog summary and actions.
  - Studio/provider-management flow can review and apply a valid test catalog.
  - Studio/provider-management controls are keyboard reachable, cancelable, and
    return focus correctly.
  - A corrupt or invalid test catalog leaves routing on the previous local or
    embedded catalog.

## Open Questions

None blocking for Stage 6.

Follow-up candidates that should not block v1:

- Whether detached signing should be added after v1 if the accepted threat model
  requires publisher authenticity beyond HTTPS plus SHA-256.
- Whether benchmark-derived routing scores should become a separate validated
  input pipeline.
- Whether MCP-based local model telemetry analytics should expose catalog,
  model-access, and model-performance evidence through read-only tools or
  resources.
- Whether OpenRouter or other aggregators should ever be supported in a
  separate explicitly aggregator-aware product track.

## Sources

- GitHub issue: https://github.com/hgahub/duumbi/issues/675
- Stage 5 acceptance comment:
  https://github.com/hgahub/duumbi/issues/675#issuecomment-4639355329
- Related provider config issue: https://github.com/hgahub/duumbi/issues/308
- Related Studio provider management issue:
  https://github.com/hgahub/duumbi/issues/484
- Related workflow metrics issue: https://github.com/hgahub/duumbi/issues/610
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Active agentic development runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`
- Agentic development map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- AI review policy:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/AI Code Review Service Policy.md`
- Spec-first agentic development note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Spec-First Agentic Development.md`
- MCP model telemetry analytics follow-up note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/00 Inbox (ToProcess)/2026-06-06 - MCP Model Telemetry Analytics.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- Repository review policy: `docs/automation/code-review-policy.md`
- Orchestration reference: `docs/automation/agentic-development-orchestration.md`
- Current static model catalog: `src/agents/model_catalog.rs`
- Current provider config model: `src/config.rs`
- Current startup provider setup: `src/cli/provider_startup.rs`
- Current model access metadata store: `src/agents/model_access.rs`
- Current model performance telemetry store:
  `src/agents/model_performance.rs`
- Current provider tests: `tests/integration_phase9b.rs`
