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
should simply be called without that field. The later Owner lifecycle revision replaces the original scope; see
[MODEL_LIFECYCLE.md](MODEL_LIFECYCLE.md).

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
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| openai / `gpt-5.6-luna` | Responses | V | V | V | V | V | V | V | O1, O4, O7 |
| openai / `gpt-5.6-terra` | Responses | V | V | V | V | V | V | V | O2, O4, O7 |
| openai / `gpt-5.6-sol` | Responses | V | V | V | V | V | V | V | O3, O4, O7 |
| openai / `gpt-6-astra` | Responses | V | N | V | V | V | V | V | O5, O6, O8 |
| anthropic / `claude-sonnet-5` | Messages | V | V | V | V | V | V | V | A1, A5, A6, A7 |
| anthropic / `claude-opus-5` | Messages | V | V | V | V | V | V | V | A2, A5, A6, A7 |
| anthropic / `claude-fable-5-1` | Messages | V | N | V | V | V | V | V | A4, A5, A6, A7 |
| xai / `grok-4.6` | Responses | V | N | V | V | V | V | N | X1, X3, X4, X5 |
| xai / `grok-build-0.1` | Responses | V | U | U | U | U | U | U | X2, X4, X5, X6 |
| minimax / `MiniMax-M3` | Chat Completions | V | V | U | U | U | U | U | M1, M2 |
| deepseek / `deepseek-v4-pro` | Chat Completions | V | V | U | V† | V | V† | V | D1, D2, D3, D4 |
| deepseek / `deepseek-flash` | Chat Completions | V | V | V | V† | V | V† | V | D1, D2, D3 |
| qwen / `qwen3.8-max` | Chat Completions | V | V | V | V | V† | V | V† | Q1, Q2, Q3 |
| qwen / `qwen3.8-flash` | Chat Completions | V | V | V | V | V† | V | V† | Q1, Q2, Q4 |
| moonshot / `kimi-k3` | Chat Completions | V | N | V | N | V | N | V | K1 |
| zhipu / `glm-5.3` | Chat Completions | V | N | V | N | V | N | V | Z1, Z3 |
| zhipu / `glm-5.3-flash` | Chat Completions | V | N | V | N | V | N | V | Z1, Z2, Z3 |

Haiku, including its pinned alias, is retired by the Owner and has no active
capability row. Do not rewrite Grok Build to a `grok-code-fast-*` alias.

**V†** is V evidence of API acceptance **as a documented alias only**, with
`canonical=false`: the Owner's policy rejects it as `NonCanonicalEffort` before
dispatch. It is not an allowed effort and is excluded from live accepted-cell
coverage. N means no supported native control for that request; it does not
promise the provider will reject rather than normalize an invalid value.
For DeepSeek Pro, low is U as an independent model-specific level: older
model-specific guidance lists high/max, while the current shared API adds low.
Use none/high/max; do not infer that Pro acquired Flash's distinct low level.

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
  `tool_use` content blocks. Use automatic choice for all three models. Never
  invent thinking budgets. Fable cannot disable thinking; Opus disabled
  thinking must not coexist with `xhigh`/`max`. Ordinary API access and adequate
  output allowance are required; account access is not proven by these docs.
- **xAI Responses (X1–X6):** Grok 4.6 named levels use `reasoning.effort`.
  Grok Build sends no reasoning controls; explicit values remain U. Flat
  function tools yield `function_call` output items; answers use message items.
  Use `store:false`. Native key/model access is required. Early-access status
  is recorded as an entitlement risk, not evidence that omission is unsupported.

- **MiniMax M3 (M1–M2):** Chat Completions with standard function tools;
  explicit none uses `thinking:{"type":"disabled"}`. Omission sends no thinking
  override. Named effort controls remain U. Set `reasoning_split:true` as an
  output-format control so reasoning does not enter visible answer content;
  it is not an effort setting or a means to disable thinking.
- **DeepSeek (D1–D4):** Chat Completions; none uses disabled `thinking`, named
  native levels use top-level `reasoning_effort`. Omit both when omitted.
  Omit `tool_choice` (older Pro guidance disallows it); ordinary function tools
  remain supported. Do not forward medium/xhigh aliases. Retain the Pro ID:
  the current pricing table and corrected changelog explicitly retain service
  despite an older retirement notice still visible on the quick-start page.
- **Qwen3.8 (Q1–Q4):** use native `reasoning_effort` low/medium/xhigh or explicit
  none as `enable_thinking:false`; omission includes neither. Do not send
  `thinking_budget` or pass high/max aliases. Both models declare function
  calling in Singapore. Regional/workspace endpoint and access must match the
  credential; no automatic region change is authorized.
- **Kimi K3 (K1):** thinking cannot be disabled. Top-level `reasoning_effort`
  supports low/high/max. Use `max_completion_tokens` for the bounded output
  allowance; omit fixed sampling/penalty fields. Native tools and answers use
  the same Chat API. Account entitlement is a separate live prerequisite.
- **GLM5.3 and Flash (Z1–Z3):** top-level `reasoning_effort` low/high/max;
  thinking cannot be disabled. Omission sends no thinking/effort override.
  Z2 explicitly makes Flash's text parameters consistent with GLM5.3 and lists
  native function calling. This is documented composition, not guessed parity.
  Use the direct general API, not a Coding Plan or third-party route.

JSON responses are the selected transport for OpenAI and xAI in this change;
the five Chat profiles also preserve buffered callback delivery. Anthropic
retains its existing SSE tool path. Streaming wire support is documented, but
new OpenAI/xAI SSE implementations are not a product requirement. Do not claim
incremental delivery where the implementation buffers a response.

## Official source register

Every source below was retrieved on **2026-09-16**. Excerpts are deliberately
short; surrounding claims are paraphrases. Model pages establish IDs/current
listing; API pages establish wire contracts. No scheduled retirement date is
assumed from a missing announcement. Refresh lifecycle/access evidence before
implementation release, particularly for newly introduced models and early-access Grok Build.

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
| M1 | [MiniMax OpenAI-compatible API](https://platform.minimax.io/docs/api-reference/text-openai-api) | `MiniMax-M3`; thinking disabled/adaptive; function tools |
| M2 | [MiniMax Chat reference](https://platform.minimax.io/docs/api-reference/text-chat-openai) | `reasoning_split`; separates reasoning from answer content |
| D1 | [DeepSeek model table](https://api-docs.deepseek.com/quick_start/pricing/) | `deepseek-flash`; V4.1 Flash; V4 Pro retained; tool calls for both |
| D2 | [DeepSeek thinking and tools](https://api-docs.deepseek.com/guides/thinking_mode/) | Native low/high/max and documented aliases; disabled thinking; tool protocol |
| D3 | [DeepSeek Chat API](https://api-docs.deepseek.com/api/create-chat-completion/) | Shared model enum and effort schema; current low/high/max controls |
| D4 | [DeepSeek Pro integration](https://api-docs.deepseek.com/quick_start/agent_integrations/oh_my_pi/) | Pro high/max; omit tool_choice; preserve assistant reasoning in future loops |
| D5 | [DeepSeek change log](https://api-docs.deepseek.com/updates/) | Corrected September 10 entry retains V4 Pro after September 14 |
| Q1 | [Qwen Chat API](https://www.alibabacloud.com/help/en/model-studio/qwen-api-via-openai-chat-completions) | Native low/medium/xhigh; separate provider aliases; regional base URLs |
| Q2 | [Qwen thinking](https://www.alibabacloud.com/help/en/model-studio/deep-thinking) | `enable_thinking`; no simultaneous effort and thinking_budget |
| Q3 | [Qwen3.8-Max](https://www.alibabacloud.com/help/en/model-studio/qwen3-8-max) | `qwen3.8-max`; Singapore function calling |
| Q4 | [Qwen3.8-Flash](https://www.alibabacloud.com/help/en/model-studio/qwen3-8-flash) | `qwen3.8-flash`; Singapore function calling |
| K1 | [Kimi K3 quick start](https://platform.kimi.ai/docs/guide/kimi-k3-quickstart) | `kimi-k3`; low/high/max; always-on thinking; native tools |
| Z1 | [GLM5.3](https://docs.z.ai/guides/llm/glm-5.3) | `glm-5.3`; low/high/max; thinking enabled only |
| Z2 | [GLM5.3 Flash](https://docs.z.ai/guides/vlm/glm-5.3-flash) | `glm-5.3-flash`; text parameters consistent with GLM5.3; function calling |
| Z3 | [Z.ai Chat API](https://docs.z.ai/api-reference/llm/chat-completion) | Native Chat request/response, function tools, reasoning_effort |

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

## Revision evidence judgments

The 17 omitted native tool rows are V by model declarations plus applicable
native API contracts. No API/model-specific restriction was ignored to reach
that result. Effort aliases are documented but not admitted; optional U cells
remain locally rejected, including MiniMax named effort, Grok Build explicit
controls and DeepSeek Pro low. None of those U cells blocks omitted operation.

The GLM Flash documentation moved under `/guides/vlm/`; the model card's old
`/guides/llm/` link failed. The live documentation index resolved the correct
page, so the failed URL is not evidence of missing API support.
DeepSeek pricing and corrected changelog outweigh the obsolete Pro redirection
paragraph; retain Pro and verify actual response model identity at release.
No invisible provider-internal routing is claimed proven by documentation.
