# DUUMBI-817 — Technical specification

Related to #817. Implements the contract in [PRODUCT.md](PRODUCT.md) only after
the required human gates. This PR contains no implementation. The evidence
matrix is [EVIDENCE.md](EVIDENCE.md); fresh review findings are in
[REVIEWS.md](REVIEWS.md). No Grok worker, merge or Stage 10 is started here.

## 1. Verified source baseline

Inspected main revision: `5e9fa23`.

| Existing location | Observed behavior / required integration |
| --- | --- |
| `src/agents/openai.rs` | Chat Completions, no effort field; tool, captured-tool, text-fallback and answer builders repeat request construction. Callback methods buffer JSON responses. Keep this legacy adapter for compatible providers. |
| `src/agents/anthropic.rs` | Messages; fixed `max_tokens=4096`; JSON and SSE tool paths; buffered answer callback; API-key or bearer authentication. Add one shared request policy across paths. |
| `src/agents/grok.rs` | Wrapper around OpenAiClient. Route named native xAI models through a separate Responses profile while preserving legacy construction. |
| `src/agents/mod.rs` | Object-safe LlmProvider exposes five methods; AgentError::is_transient controls fallback. Add capability and incomplete-output errors. |
| `src/agents/factory.rs` | Resolves model, credentials and shared compatible clients. Add options-aware construction without requiring callers to store model/effort preferences. |
| `src/agents/fallback.rs` | Falls through on transient errors and records last successful model label. Explicit-effort and pinned validation calls must not substitute models. |
| `src/agents/model_catalog.rs` | Static and refreshed ranking catalog; reasoning/coding Booleans; ModelSelectionContext; ResolvedProviderConfig assembly. No effort capability metadata. |
| `src/agents/model_catalog_publisher.rs` | Deterministic V1 metadata publisher; keep provider capability authority separate from discovery/ranking hints. |
| `src/config.rs` | ProviderConfig/ResolvedProviderConfig have no effort. Preserve persisted schema and defaults. |
| `src/tools.rs` | Existing schemas and tool-call-to-PatchOp converters. Adapt the envelope, not graph semantics. |
| `src/agents/orchestrator.rs`, `src/intent/`, `src/bench/runner.rs` | Tool use, validation/repair, bounded execution and model-I/O capture. Preserve production call path for validation. |
| `src/cli/`, `crates/duumbi-studio/src/ws.rs` | Provider setup and Query/Agent contexts. Audit default omission and error presentation, not new controls. |
| `docs/provider-catalog.md`, `tests/fixtures/model_catalog/` | Document capability authority, regenerate curated inputs only where necessary, retain old schema compatibility. |

The retained #780 error is reproduction evidence, not proof this baseline sends
explicit effort. No implementation should start by assuming an existing effort
configuration or by adding an arbitrary config field to fix it.

## 2. Internal options and resolution

Add `src/agents/provider_capabilities.rs` and `src/agents/responses.rs`.
The former owns the release-reviewed matrix and pure request-plan resolution;
the latter owns common Responses encoding/decoding with separate OpenAI/xAI
profiles. Public items require docs and Result-returning functions follow the
repository's `#[must_use]` convention.

Proposed Rust interface contract (design, not implementation):

```rust
enum ReasoningEffort { None, Low, Medium, High, Xhigh, Max }
struct ProviderRequestOptions {
    effort: Option<ReasoningEffort>, // None = omitted; Some(None) = explicit none
    max_output_tokens: Option<std::num::NonZeroU32>,
    pin_model: bool,
}
enum CallMode { Tools, Answer }
enum ApiSurface { ChatCompletions, Responses, Messages }
enum CapabilityClass { Verified, Unsupported, Unverified }
struct CapabilityEvidence { source_id: &'static str, retrieved_on: &'static str }
```

Use qualified `ReasoningEffort::None` in code/tests to avoid confusing it with
`Option::None`. Default options are omitted effort, no new output cap, and no
pin beyond existing selection. There is no persisted options field, environment
effort override or CLI/TUI control in this issue.

Introduce options-aware factory variants of the existing context/global-access
and explicit-key constructors. Existing constructors delegate with defaults.
Options are immutable for the constructed provider operation; workflows that
need another effort construct another provider through the same boundary.
Keep the existing object-safe LlmProvider signatures and five-method contract.
Direct native client constructors also default to omitted and use the same
resolver; they must not bypass preflight just because the factory was skipped.

Each call derives a `ResolvedCallPlan` from exact provider, resolved model,
endpoint profile, options and actual CallMode. It contains selected surface,
native effort encoding, tool-choice policy, response transport, output cap and
evidence references. Resolve before constructing/sending HTTP. Actual method
mode is authoritative: a wrongly defaulted `requires_tools` ranking hint must
not permit an invalid tool request.

Resolution order:

1. Resolve the model with existing selection/access policy. Explicit model
   overrides remain exact. Do not reinterpret catalog reasoning as effort.
2. Classify endpoint identity and locate the exact reviewed capability row.
   Never infer Responses support from a model prefix or arbitrary base URL.
3. Apply legacy omitted compatibility first for custom/compatible endpoints and
   models outside the named set. This branch emits no new effort control.
4. For named direct-native rows, select the documented surface. Check omitted
   support or the exact explicit cell. V yields a plan; N/U return typed errors.
5. For explicit effort with no reviewed row, fail U. Do not rescore/select
   another model based on the failure. Validate output-limit constraints.
6. Execute one plan; preserve it across the existing bounded repair/retry path.

Unknown effort strings fail input parsing as `invalid effort`, before matrix
lookup. The enum is a vocabulary, not a claim that all models accept every value.

## 3. Catalog authority and compatibility

Extend the existing catalog with a **release-owned capability lookup** keyed by
canonical provider and exact model ID. Do not change the published V1 JSON
schema in this issue. Ranking/lifecycle data and protocol capability data have
different provenance: remotely refreshed quality/reasoning/coding fields cannot
grant an effort value, request field or endpoint. Reviewed capability entries
include CallMode, each effort state, API surface, mapping, source IDs and dates.

Add all ten IDs to embedded/curated model data. Use the existing Haiku alias as
an explicitly documented equivalent capability key and allow its pinned ID.
Never map Grok Build to DUUMBI's retired `grok-code-fast-1`. Keep the existing
retirement rule intact. Ranking scores are maintained as routing preferences,
not evidence that one provider's effort is equivalent to another's.

Old V1 documents deserialize unchanged. Capability resolution joins their model
IDs against this release's reviewed lookup. A known reviewed model can use its
reviewed capability; absent capability evidence remains U for explicit effort.
A downloaded new model or a true `reasoning` Boolean cannot create support.
Unknown legacy models with omitted effort keep their existing transport.
Invalid catalogs retain the known-good/embedded fallback; preserve hashes,
adoption consent, credentials, explicit overrides and provider ordering.

Update publisher curated fixtures and documentation consistently; keep provider
discovery status truthful. No claim of fresh discovery is created by a static
matrix. Retain deterministic catalog bytes when semantic input is unchanged.
No new remote catalog endpoint, schedule or docs-site deployment is required.

## 4. Endpoint and authentication boundary

Native profiles bind provider identity to the existing official origins:
OpenAI `https://api.openai.com/v1/responses`, Anthropic
`https://api.anthropic.com/v1/messages`, xAI
`https://api.x.ai/v1/responses`. Authentication retains existing secret resolution
and provider-specific headers. API model support does not establish OAuth or
subscription entitlement; preserve legacy auth handling and report access errors.

For an explicit custom adapter, add an internal `EndpointCapabilityDescriptor`
containing exact endpoint URL(s), provider/protocol profile, allowed model/mode
rows and evidence references. It is supplied by a trusted embedding caller or
test harness, not model output, prompts or downloaded catalog data. The options
factory accepts it separately from credential/config fields. Only the selected
declared endpoint receives the credential; do not auto-append paths or silently
move between origins. Mock HTTP endpoints are test-only; production declared
endpoints require HTTPS. Disable redirects on the new authenticated transport.

Existing custom URLs with omitted effort use the old adapter unchanged. Without
a descriptor, explicit effort fails U. Declaring a descriptor selects an
implemented protocol; it does not invent native effort support, nor prove that
a proxy or the user's credential can access a model. Such access is separately
validated. Error messages redact query strings, credentials and header values.

## 5. Request and response adapters

### OpenAI Responses

Select Responses for all four named direct OpenAI models, tools and answers,
including omitted effort. Leave unrelated OpenAiClient consumers on the legacy
Chat Completions implementation.

- Map existing system/user prompts into `input` messages. Flatten each function
  tool from the existing wrapper to `type`, `name`, `description`, `parameters`.
  Set `strict:false` to retain existing optional-field schema semantics and
  `tool_choice:"auto"`. Do not require a tool merely to avoid handling text.
- For explicit V values serialize `reasoning:{"effort":value}`; omission removes
  the entire reasoning control. Do not send Chat's `reasoning_effort` field.
- Answers omit mutation tool definitions. Set `store:false`; use independent
  requests, not provider-stored conversations or `previous_response_id`.
- Parse typed `output` items: completed `function_call` name/JSON arguments
  become existing tool-call structs and then PatchOps; assistant
  `message`/`output_text` becomes visible text. Ignore reasoning content for
  patch/text parsing. Check response status/error/refusal before accepting ops.
- JSON buffering preserves existing callback behavior. Do not claim SSE.
  Preserve function ordering and tool-over-text precedence. Use no automatic
  switch back to Chat Completions after HTTP 400.

### Anthropic Messages

Use the same `ResolvedCallPlan` in JSON tool, SSE tool, capture and answer
builders. Keep Messages headers/auth and existing tool schema. Exact native
mapping is in EVIDENCE.md: omission sends no effort/thinking override; supported
explicit none sends disabled thinking alone; named effort sends
`output_config.effort` without an invented budget or extra thinking override.
Automatic choice applies to all four models; Fable must never receive forced
`any` or named-tool choice. No new beta header or per-message effort feature.

Parse structured `tool_use` only; do not turn textual tool-looking content into
PatchOps. Preserve text callbacks without exposing thinking/signature blocks as
answers. In SSE, assemble blocks by index and accept ops only after a complete
message terminal event and acceptable stop reason. EOF, errors, malformed JSON
or max-token truncation cannot produce a partial successful patch.

The current 4096 output cap remains the default when callers supply none.
The new internal output allowance can provide more room within the caller's
existing cost/time budgets; it is independent of effort. Never lower effort to
fit the allowance. Exhaustion returns a distinct incomplete-output error with
no patch; live validation must record the allowance used. Do not silently adopt
the provider documentation's large suggested budgets for existing users.

Current calls send fresh prompts and do not round-trip provider conversations.
Do not add a tool-result conversation loop in this issue. If a later change
introduces one, thinking/redacted blocks must be preserved according to the
provider contract; this specification is not permission to strip/reconstruct them.

### xAI Responses

Use the common Responses codec with a separate xAI request profile and official
xAI origin. Grok 4.6 sends only its verified named effort values. Grok Build
omission sends **no reasoning/effort control**; every explicit cell remains U.
Never borrow Grok 4.6 effort support. This implements documented ordinary
function calling using the evidence composition in EVIDENCE.md.

Use xAI's documented flat function-tool schema, automatic choice, `store:false`,
JSON output items and independent requests. Do not blindly inherit OpenAI-only
optional fields, strictness flags or beta behavior. Provider-specific fixtures
must validate the emitted key set. Existing buffered callbacks remain buffered;
no new xAI SSE work is required. Preserve `xai` provider labels and Grok wrapper
behavior for legacy models. No server-side search/code tools are enabled.

### Outcomes shared by all adapters

| Method | Required result |
| --- | --- |
| `call_with_tools` | Complete structured calls → ordered PatchOps; no calls retains the method's existing empty result contract; invalid output → error. |
| `call_with_tools_streaming` | Preserve each adapter's existing callback/NoToolCalls behavior, documented in baseline fixtures. Tool/text precedence stays unchanged; partial ops never escape. |
| `call_with_tools_captured` | Same validated ops plus actual received JSON body and existing capture status, through the same request plan. |
| `answer` | Text only, no mutation tools; no text/refusal/incomplete output is not success. |
| `answer_streaming` | Existing buffered or incremental callback semantics plus final text; no reasoning text or patch execution. |

Fixtures must pin the legacy per-method differences before refactoring; do not
normalize all empty/text responses into one outcome accidentally. Captures
remain opt-in under existing storage/redaction policy; never include secrets or
raw headers in capability diagnostics. Graph validation and atomic application
stay in the existing orchestrator after complete response parsing.

## 6. Errors, retries and observability

Add typed `UnsupportedCapability` and `UnverifiedCapability` AgentError variants
with provider/model/surface/mode/effort/evidence IDs. Stable display strings
contain `unsupported capability` and `unverified capability`, respectively.
Both return false from `is_transient`; local rejection generates no HTTP request,
provider fallback or repair attempt. Add incomplete-output classification
separate from NoToolCalls; incomplete data is never applied.

For options with explicit effort or `pin_model=true`, provider-chain construction
retains only the selected operation identity for dispatch; transient retries
may target that same model/endpoint within existing bounds. Do not construct a
fallback that silently drops options. Omitted legacy operations retain their
existing transient fallback policy. Named-model acceptance tests use pinning,
so a fallback cannot mask a failed matrix row. Repair retries preserve the same
options even when prompts change. Preserve HTTP 401/403/429/5xx classification.

Safe diagnostics record requested effort (`omitted` is a value, not an inferred
default), selected API, model, capability class and evidence revision. Error
formatters in CLI/REPL/TUI/Studio expose these distinctions without demanding a
default-model setup. No startup provider warnings are added for unrelated flows.

## 7. Validation and acceptance mapping

Offline matrix generation covers 10 models × 7 effort states × 2 call modes =
140 cells, plus invalid values, alias cases, unknown models and custom endpoints.
V rows assert exact native request keys; N/U assert the correct error and zero
mock-server requests. Do not call providers in default CI.

| Test group | Evidence required | Product coverage |
| --- | --- | --- |
| Pure resolver/matrix | Every cell, endpoint identity, source completeness, Haiku alias and retired Grok alias; no missing row treated as V/N | AC-01–04 |
| Request contracts | Omitted keys absent, explicit none distinct, exact levels, OpenAI/xAI isolation, Fable auto choice, no invented budgets | AC-02–05 |
| Response fixtures | Single/multiple/mixed calls, answer fallback, callback order, capture parity, malformed arguments, incomplete/refusal/error bodies, truncated SSE | AC-06 |
| Factory and chain | All five methods, default wrappers, global-access/probe constructors, zero fallback on capability error, pinned transient/repair behavior | AC-02–06 |
| Catalog migration | Old V1 load/adopt/hash behavior; unknown metadata cannot grant capabilities; invalid update rollback; deterministic publishing | AC-05, AC-07 |
| Workflow regression | Agent/Intent/bench use production options boundary; Query read-only; provider/no-provider paths; compatible-provider custom URLs/headers/auth preserved | AC-05–08 |
| Joint live matrix | Exact model/tool path, omitted plus every V explicit value; N/U remain local tests; no substitution, skipped/denied rows recorded as incomplete | AC-08–09 |

Use wiremock or the repository's existing HTTP-mock approach with injectable
endpoints; never mutate global production origins to test. Integration tests
must verify the received request and structured output-to-patch behavior,
not just HTTP 200 or request construction.

Implementation checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test --all`; Linux and macOS only. Add focused CLI/TUI state tests and a
manual provider/no-provider smoke when those interactions change. No such
runtime checks are asserted to have run in this spec-only PR.

Live tests are opt-in after implementation and resource authorization. They
must use the production options/factory/codec path and record date, commit,
model, endpoint, effort state, tool mode, transport, timeout/output allowance,
redacted outcome and resulting validated graph evidence. Cover all ten omitted
tool rows and each V explicit value on its selected tool path; additionally
cover tool-free answer, callback and capture integration for each adapter,
including Anthropic SSE. Test Grok Build directly with omitted effort. Do not
require a provider-internal effective effort value, force tools on Fable,
spend without the applicable budget gate, or call any N/U explicit cell live.

Missing credentials/access and output-budget exhaustion remain reported failures
or incomplete coverage. They do not remove models from joint acceptance. No
live calls were made while preparing this specification.

## 8. Delivery sequence, dependencies and rollback

After Owner review/merge and the actual Ready for Build gate, implement bounded
work packages in one delivery unit: (1) matrix/resolver/options and legacy
fixtures, (2) OpenAI Responses, (3) Anthropic controls, (4) xAI Responses and
catalog integration, (5) full regression plus joint live evidence. Each package
must remain reviewable; none independently fulfills #817. New sub-issues or
parallel execution are not created by this document.

Reuse reqwest, serde/serde_json, futures and existing error/test libraries.
No new provider SDK, database or public config migration is required. Verify
provider documentation/access again before release; new contradictory evidence
requires updating this matrix and its reviews, not runtime guesses.

Rollback means reverting the implementation as one feature and retaining
legacy persisted config/catalog readability. It must not silently dispatch an
explicit-effort call through the old adapter. Runtime HTTP failure is not a
rollback trigger that permits effort downgrade or model substitution.
