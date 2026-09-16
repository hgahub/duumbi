# DUUMBI-817 — Provider Models and Reasoning Effort Support

Related to #817. Specification only; the execution issue stays open.

## Authority and status

This revision follows [Stage 5 Accept 5704193547](https://github.com/hgahub/duumbi/issues/817#issuecomment-5704193547)
of 2026-09-16 and the Owner's manual Desktop handoff. It supersedes the older
product requirements wherever they conflict. Approval of decision 5655972051
and its Stage 7 review is historical, not approval of this revision.

Status: product and technical contracts prepared for Owner review. Fresh
Desktop content reviews are recorded in [REVIEWS.md](REVIEWS.md); no workflow
approval, merge or Ready for Build transition is implied.

## Problem and outcome

The retained [Luna failure](../../docs/e2e/results/duumbi-780-completion-20260910/live-provider-error.json)
reports HTTP 400 on Chat Completions before graph generation. The desired
outcome is successful provider-native tool use with the requested supported
effort, without degrading reasoning or substituting another model.

Source baseline: `5e9fa23` (main after #822). The current OpenAI client uses Chat
Completions but does **not** serialize an explicit effort field. Anthropic uses
Messages without effort; Grok delegates to the shared OpenAI-compatible client.
The catalog's reasoning Boolean is a ranking hint, not an effort contract.
The reported provider error does not establish what exact request was sent.

Users are operators and maintainers running Agent/Intent authoring, Query,
provider probes and live benchmark validation. Provider setup remains through
`/provider`; model and effort selection remain internal.

## Scope

One joint delivery covers these ten models:

| Provider | Required models |
| --- | --- |
| OpenAI | GPT-5.6 Luna, Terra, Sol; GPT-6 Astra |
| Anthropic | Claude Sonnet 5, Opus 5, Haiku 4.5, Fable 5.1 |
| xAI | Grok 4.6; Grok Build 0.1 |

Include an authoritative model/API/tool/effort matrix, native request mapping,
response parsing, internal effort propagation, catalog integration, compatibility,
safe diagnostics and joint verification. All provider methods must preserve
their tool, text, callback and capture contracts.

Exclude new CLI/TUI effort controls, persisted default effort, new model-selection
UX, automatic effort policy, cross-provider effort equivalences, new graph
semantics, arbitrary tools, new provider onboarding, workflow changes, public
catalog deployment and unrelated #780 benchmark work. No implementation is
authorized by this specification task.

## Effort and capability contract

The internal caller supplies exactly one of:

1. **Omitted:** send no effort control. Also omit any thinking override introduced
   by this feature. Do not fill in `medium`, `high`, `none`, or a token budget.
   The provider's effective internal default is outside acceptance scope.
2. **Explicit none:** request disabled reasoning only through an officially
   verified native representation. This is distinct from omission, including
   for providers where omission currently happens to disable reasoning.
3. **Explicit named level:** preserve the documented value for that exact
   provider/model/API/tool combination. Do not clamp, drop, translate to an
   approximate budget, or choose a different model to make it work.

Existing callers default to omitted. Task complexity, performance history and
catalog ranking must not silently introduce an explicit effort. Internal
validation harnesses use the same request-options boundary as production.

Each combination has a dated evidence class:

| Class | Meaning | Dispatch behavior |
| --- | --- | --- |
| V | Officially verified support with URL, retrieval date and short excerpt | Dispatch with the reviewed native mapping |
| N | Officially documented non-support | Fail before send: `unsupported capability` |
| U | Unverified; official confirmation is missing or ambiguous | Fail before send: `unverified capability` |

Both failure classes are non-transient and distinct from credential access
failures. Neither permits retry through a different endpoint, reduced effort,
or another model. A provider model list or an HTTP 200 alone is not evidence
of the exact tool/effort capability. Missing documentation is not class N.

Every named model must have a V **omitted-effort native tool path** before
Stage 9 can pass. U explicit-effort combinations may remain U when they are
rejected before send. This permission does not excuse a missing omitted path.
Grok Build remains in the joint scope with effort omitted; its explicit-effort
cells remain U. [EVIDENCE.md](EVIDENCE.md) records the official documentation
chain establishing the omitted path, separately from future live evidence.

## Compatibility and call outcomes

For named models on their direct native provider endpoints, choose the verified
API surface even when effort is omitted. For previously supported models
outside this set, omitted effort preserves endpoint, request shape, headers,
authentication, model selection and fallback behavior. Unknown, non-retired
explicit model overrides remain usable through that legacy omitted path.
Legacy omission is a compatibility exception, not a V capability assertion.

Custom URLs and OpenRouter do not inherit direct-provider capabilities from
model names. An existing custom endpoint with omission retains its existing
transport. New explicit effort or an API migration needs an internal endpoint
descriptor naming the exact endpoint and reviewed protocol; no arbitrary URL
rewriting, origin change or credential redirection is permitted. Without that
descriptor, reject the new operation as unverified before sending it.

Once an explicit effort request resolves a model, retries must retain its
identity, endpoint profile and effort. Existing omitted-effort legacy fallback
is retained; it cannot be used as evidence that a particular named model works.
All named-model validation runs pin the requested model and disable substitution.

Preserve tool-to-`PatchOp` conversion, tool/text precedence, text fallback,
streaming callback ordering and captured-response availability. Query remains
tool-free and read-only. Partial, malformed or truncated tool output must never
be applied to the graph. Provider errors and capability failures must not appear
as successful empty mutations. Existing graph validation remains mandatory.

## Acceptance criteria

The sole delivery owner is unit `provider-reasoning-compatibility`.

| ID | Observable acceptance requirement |
| --- | --- |
| AC-817-01 | All ten models have exact IDs, selected API and tool/non-tool modes, V/N/U effort cells, mappings, prerequisites and dated official evidence. All ten omitted native tool paths are V before Stage 9 passes. |
| AC-817-02 | Omitted, explicit none and explicit level remain distinct through resolution, factory, wrappers, all five provider methods and retry. Existing callers supply omitted; no implicit level or invented budget appears. |
| AC-817-03 | Supported tools and effort use the native compatible surface, including OpenAI Responses. Requested model and effort are preserved; no downgrade or substitution rescues a failed request. |
| AC-817-04 | N and U fail before HTTP dispatch and graph mutation with different stable, non-transient errors identifying provider/model/API/tool mode/effort and safe evidence references. |
| AC-817-05 | Existing omitted-effort configs, unknown non-retired overrides, custom endpoints and unrelated compatible providers retain legacy behavior. Explicit effort on an unverified custom endpoint fails locally. |
| AC-817-06 | Tool, answer, callback and capture paths preserve their observable outcomes. Query sends no mutation tools. Partial/invalid output produces no applied patch. |
| AC-817-07 | Embedded/refreshed catalog handling, deterministic publisher inputs, diagnostics, CLI/REPL/TUI/Studio callers, tests and docs agree. Older catalogs do not invent explicit-effort support or rewrite credentials/configs. |
| AC-817-08 | Offline tests cover every matrix cell and call path, including zero-send failures, shared-client isolation, identity preservation and catalog compatibility. Live evidence covers all ten omitted tool paths and every V explicit value on its selected tool path, with tool-free and streaming coverage per adapter. |
| AC-817-09 | One joint release requires all named models and three providers. Missing access, skipped live cells or unresolved mandatory capabilities block completion; they are never converted into passing evidence. |

## BDD scenarios

| ID | Given / When / Then |
| --- | --- |
| BDD-817-01 | Given Luna and `high`, when mutation tools are requested, then send Responses with the same model and `reasoning.effort=high`, and parse returned calls into validated patches. |
| BDD-817-02 | Given each named model and omitted effort, when native tools are requested, then send no effort/thinking override through its V path; never require knowledge of the internal default. This includes Grok Build without an effort field. |
| BDD-817-03 | Given Astra and explicit none, when any call is attempted, then reject as documented unsupported with zero requests. |
| BDD-817-04 | Given Grok Build and explicit high, when a call is attempted while that cell is U, then return `unverified capability`, with no fallback and no request. |
| BDD-817-05 | Given Sonnet/Opus/Haiku and verified explicit none, when a Messages call is built, then use the native disabled-thinking representation; omission must not emit that field. |
| BDD-817-06 | Given Haiku and named high, when a call is attempted, then reject unsupported; never manufacture `budget_tokens`. Given Fable and none, reject unsupported. |
| BDD-817-07 | Given Fable and a V effort, when requesting mutation tools, then use automatic tool choice; never force a named tool or `any`. Text-only output remains text, not a patch. |
| BDD-817-08 | Given an old catalog/config and custom URL or unrelated provider, when effort is omitted, then preserve the existing request contract. An explicit effort without verified endpoint metadata fails as U. |
| BDD-817-09 | Given an explicit effort request and a timeout or rate limit, when retry policy applies, then retain model and effort; do not invoke another provider/model. Legacy omitted fallback is separately regression-tested. |
| BDD-817-10 | Given tool, text-only, mixed, malformed and truncated responses, when each applicable method parses them, then preserve callbacks/capture and apply only complete validated patches. |
| BDD-817-11 | Given Query with any allowed effort, when answering, then no mutation tools are sent and no graph/workspace write occurs. |
| BDD-817-12 | Given a refreshed catalog adds a model or changes reasoning rankings, when it is adopted, then it cannot create verified effort support or change credentials; legacy omission remains available as defined above. |
| BDD-817-13 | Given nine passing live model tests and absent Grok Build live evidence/access, when release completion is evaluated, then joint delivery remains blocked; this does not block specification review. |

## Decomposition decision

Keep #817 together. The matrix, preflight errors, internal options and shared
transport compatibility are one contract; separate provider releases would
violate Stage 5. Sequential implementation work packages can cover the common
boundary, OpenAI, Anthropic, xAI and joint verification after approval. They are
not independently shippable sub-issues. No new execution issue is allocated.

## Evidence standard and remaining validation

A model-specific capability declaration plus the provider-wide API contract
applicable to that capability can establish V; a separate code example or support
letter naming every model is not required. Such documentation composition must
be explicit and must not override any model-specific restriction. Absence of an
effort setting does not block omitted-effort calls. Absence of evidence for the
underlying mandatory tool capability still would.

This applies to Grok Build: its model page declares function calling, while xAI
specifies the common Responses function-tool request/response contract. The
reviewed omitted route uses both sources; the generic reasoning controls are
not extrapolated to that model. This corrects the earlier worker's unnecessarily
strict evidence threshold without changing Stage 5 scope.

Account entitlement, structured tool output and all requested V combinations
must still pass live implementation validation. No provider call or access test
has been performed in this specification task. No new product decision remains.
