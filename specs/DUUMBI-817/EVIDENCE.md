# DUUMBI-817 — Capability evidence

Related to #817. Research retrieved on **2026-09-16**. Documentation evidence,
not live API validation. V means documented support, N documented non-support,
U unverified. Product semantics are in [PRODUCT.md](PRODUCT.md).

## Provenance and superseded conclusions

The [prior evidence pack](https://github.com/hgahub/duumbi-vault/blob/247068d4cb4806acfcdb90159f3d12e8d0bc2883/Duumbi/10%20Spec%20Evidence/817-provider-effort-2026-09-16-attempt2/EVIDENCE_PACK.md)
and its adjacent JSON/results were inspected, not merely the VM path. Pack ID:
`817-from-5655972051-attempt2`; original retrieval date: 2026-09-16. Prior
attempts and recovery artifacts remain untouched. The archive branch
`archive/spec-817-5655972051-attempt2-20260916` points to `9f345ff02cb94855a5e7b17e1433669d7aa9643c`;
the retired active branch `codex/spec-817-5704193547` was at
`706a2f4c53e9fb1c1aa6ff87edcc692b7a81acb9` when inspected. This PR uses a new
Desktop branch from `5e9fa23`; no worker checkpoint was resumed.

The [renewed acceptance](https://github.com/hgahub/duumbi/issues/817#issuecomment-5704193547)
removes the need to know effective omitted-effort defaults. Historical Astra
default uncertainty therefore does not block this revision. The Owner also
clarified in this Desktop conversation that models without effort controls
should simply be called without that field. No ten-model scope reduction follows.

The old pack marked Grok Build's API surface U because no example named that
model. The fresh review uses a consistent evidence standard: a model-specific
function-calling declaration plus applicable provider-wide protocol documentation
is sufficient. This is an explicit documentation-composition inference, not a
claim that a model-specific example or live test was found. It establishes the
ordinary omitted-effort path, not undocumented reasoning controls. The exact
same composition rule applies across providers.

## Authoritative selected-path matrix

Modes `T` and `A` mean client function tools and tool-free answers. Every row
below covers both modes on its selected native API; it does not assert support
on another endpoint, a proxy, Bedrock, Vertex, OpenRouter or a subscription plan.
Response transport modes and limitations follow the adapter contracts below.
Canonical provider key is `xai`; `grok` remains a legacy config alias.

| Provider / exact model ID | API | Omitted | Explicit none | low | medium | high | xhigh | max | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| openai / `gpt-5.6-luna` | Responses | V | V | V | V | V | V | V | O1, O4, O7 |
| openai / `gpt-5.6-terra` | Responses | V | V | V | V | V | V | V | O2, O4, O7 |
| openai / `gpt-5.6-sol` | Responses | V | V | V | V | V | V | V | O3, O4, O7 |
| openai / `gpt-6-astra` | Responses | V | N | V | V | V | V | V | O5, O6, O8 |
| anthropic / `claude-sonnet-5` | Messages | V | V | V | V | V | V | V | A1, A5, A6, A7 |
| anthropic / `claude-opus-5` | Messages | V | V | V | V | V | V | V | A2, A5, A6, A7 |
| anthropic / `claude-haiku-4-5-20251001` | Messages | V | V | N | N | N | N | N | A3, A5, A6, A7 |
| anthropic / `claude-fable-5-1` | Messages | V | N | V | V | V | V | V | A4, A5, A6, A7 |
| xai / `grok-4.6` | Responses | V | N | V | V | V | V | N | X1, X3, X4, X5 |
| xai / `grok-build-0.1` | Responses | V | U | U | U | U | U | U | X2, X4, X5, X6 |

The documented Haiku alias `claude-haiku-4-5` has the same capability entry;
preserve an explicitly selected alias on the wire. Do not rewrite Grok Build
to a `grok-code-fast-*` alias: DUUMBI deliberately retires `grok-code-fast-1`.
Other strings such as `minimal` or `ultra` are outside this six-value internal
vocabulary; reject them as invalid input, not a guessed provider equivalent.

## Native controls and protocol profiles

- **OpenAI Responses (O1–O8):** explicit levels, including supported `none`, use
  `reasoning.effort`. Omission leaves `reasoning` absent. Requests use `input`,
  flat function tools and automatic tool choice; outputs use typed `output`
  items. Astra tools require this endpoint. GPT-5.6 guidance recommends it for
  reasoning/tool workflows. Direct API key and model access are prerequisites.
- **Anthropic Messages (A1–A7):** named levels use `output_config.effort`.
  Supported explicit none uses only `thinking: {"type":"disabled"}`; it does
  not also set an effort. Omission sends neither override. Tool calls are
  `tool_use` content blocks. Use automatic choice for all four models. Never
  map Haiku levels into budgets. Fable cannot disable thinking; Opus disabled
  thinking must not coexist with `xhigh`/`max`. Ordinary API access and adequate
  output allowance are required; account access is not proven by these docs.
- **xAI Responses (X1–X6):** Grok 4.6 named levels use `reasoning.effort`.
  Grok Build sends no reasoning controls; explicit values remain U. Flat
  function tools yield `function_call` output items; answers use message items.
  Use `store:false`. Native key/model access is required. Early-access status
  is recorded as an entitlement risk, not evidence that omission is unsupported.

JSON responses are the selected transport for OpenAI and xAI in this change;
their existing callback methods can retain buffered text delivery. Anthropic
retains its existing SSE tool path. Streaming wire support is documented, but
new OpenAI/xAI SSE implementations are not a product requirement. Do not claim
incremental delivery where the implementation buffers a response.

## Official source register

Every source below was retrieved on **2026-09-16**. Excerpts are deliberately
short; surrounding claims are paraphrases. Model pages establish IDs/current
listing; API pages establish wire contracts. No scheduled retirement date is
assumed from a missing announcement. Refresh lifecycle/access evidence before
implementation release, particularly for Haiku and early-access Grok Build.

| ID | Official source | Short excerpt / evidence anchor |
| --- | --- | --- |
| O1 | [GPT-5.6 Luna](https://developers.openai.com/api/docs/models/gpt-5.6-luna) | “Reasoning.effort supports: none, low, medium (default), high, xhigh, and max.” |
| O2 | [GPT-5.6 Terra](https://developers.openai.com/api/docs/models/gpt-5.6-terra) | “Reasoning.effort supports: none, low, medium (default), high, xhigh, and max.” |
| O3 | [GPT-5.6 Sol](https://developers.openai.com/api/docs/models/gpt-5.6-sol) | “Reasoning.effort supports: none, low, medium (default), high, xhigh, and max.” |
| O4 | [GPT-5.6 guidance](https://developers.openai.com/api/docs/guides/latest-model?model=gpt-5.6) | “Use the Responses API for reasoning, tool-calling, and multi-turn workflows.” |
| O5 | [GPT-6 Astra](https://developers.openai.com/api/docs/models/gpt-6-astra) | “`reasoning.effort` supports `low`, `medium`, `high`, `xhigh`, and `max`.” |
| O6 | [Astra guidance](https://developers.openai.com/api/docs/guides/latest-model) | “GPT-6 Astra supports Chat Completions, but tool calling requires Responses.” |
| O7 | [Function calling](https://developers.openai.com/api/docs/guides/function-calling) | “explicitly set `strict: false`”; flat function schema and typed calls |
| O8 | [Create a response](https://developers.openai.com/api/reference/typescript/resources/beta/subresources/responses/methods/create) | Example combines `model: "gpt-6-astra"`, `tools`, `input`, `tool_choice: "auto"` and no effort field |
| A1 | [Sonnet 5](https://platform.claude.com/docs/en/models/sonnet-5/overview) | “adaptive thinking is on by default” |
| A2 | [Opus 5](https://platform.claude.com/docs/en/models/opus-5/overview) | “thinking can be disabled only at effort `high` or below” |
| A3 | [Haiku 4.5](https://platform.claude.com/docs/en/models/haiku-4-5/overview) | “Default effort” / “Not supported”; ID and pinned alias relation |
| A4 | [Fable 5.1](https://platform.claude.com/docs/en/models/fable-5-1/overview) | “forced tool use returns an error” |
| A5 | [Effort](https://platform.claude.com/docs/en/build-with-claude/effort) | “Set `output_config.effort` on the request.”; supported models/levels and tool-use interaction |
| A6 | [Thinking troubleshooting](https://platform.claude.com/docs/en/build-with-claude/thinking-troubleshooting) | “any value not listed as rejected is accepted”; per-model configuration table |
| A7 | [Tool use](https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview) | “With the default `tool_choice` of `{"type": "auto"}`”; Messages request/result protocol |
| X1 | [Grok 4.6](https://docs.x.ai/developers/models/grok-4.6) | “Function calling”; native model identity and capability |
| X2 | [Grok Build 0.1](https://docs.x.ai/developers/models/grok-build-0.1) | “grok-build-0.1”; “Function calling”; “Reasoning” |
| X3 | [Reasoning](https://docs.x.ai/developers/model-capabilities/text/reasoning) | “Controls reasoning depth (cannot be disabled)”; Grok 4.6 enum; no Grok Build enum |
| X4 | [Function calling](https://docs.x.ai/developers/tools/function-calling) | “Define custom tools that the model can invoke during a conversation.”; request example omits reasoning controls |
| X5 | [API comparison](https://docs.x.ai/developers/model-capabilities/text/comparison) | “The Responses API is the recommended way to interact with xAI models.” |
| X6 | [Responses reference](https://docs.x.ai/developers/rest-api-reference/inference/responses) | “A list of tools the model may call in JSON-schema.”; endpoint and typed response |
| X7 | [Release notes](https://docs.x.ai/developers/release-notes) | “Currently in early access.”; Grok Build entry |
| X8 | [Generate text](https://docs.x.ai/developers/model-capabilities/text/generate-text) | “you can set `store: false` on the request” |
| X9 | [Streaming](https://docs.x.ai/developers/model-capabilities/text/streaming) | Streaming transport reference; no new streaming implementation required |

The prior pack additionally used [Claude model overview](https://platform.claude.com/docs/en/models/overview)
(original retrieval 2026-09-16). Its historical retirement dates are not promoted
to current runtime policy here. No historical review result is reused as approval.

## Grok Build evidence assessment

1. X2 establishes the exact model and client function-calling capability.
2. X5 describes Responses as the common xAI model interface; X4 supplies the
   ordinary tool request with reasoning omitted; X6 defines its response shape.
3. No checked model-specific restriction contradicts that base capability.
   Applying this common protocol to X2's declared function calling is the
   documented-interface composition used for the V omitted row.
4. X3 scopes named reasoning controls to other named models. It supplies no
   Grok Build effort enum. All Grok Build explicit cells therefore remain U,
   including none; they do not inherit Grok 4.6 values or field support.
5. X7's early-access label and account-specific availability require a live
   access/tool test during implementation, just as other models require access.
   A denied/skipped live test blocks joint release, not specification drafting.

The earlier proposed blocker **B-817-01 is resolved at specification level** by
this corrected evidence assessment. This is not a live success claim. Reopen
the capability assessment if xAI supplies a model-specific contrary restriction.
