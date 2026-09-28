# DUUMBI-817 — Target catalog and retirement contract

Related to #817. Owner revision: **2026-09-16**, from the complete Desktop
retirement/introduction list and the subsequent canonical-effort-only decision.
Audit: [Owner scope and effort decision](https://github.com/hgahub/duumbi/issues/817#issuecomment-5705161911).
These explicit decisions supersede the original ten-model/three-provider scope;
the remaining Stage 5 omission, evidence and no-substitution conditions remain.
This is DUUMBI product retirement, not a claim that each provider discontinued
these models. The earlier partial repeated lists are superseded.

## Complete supported target

Exactly **17 curated models across 8 active providers**: nine retained from the
initial scope, seven additions, and the existing DeepSeek V4 Pro. Every row
supports omitted effort on its documented native tool path. `none` below is an
explicit off switch, separate from omission. Evidence and limitations are in
[EVIDENCE.md](EVIDENCE.md); no live API success is asserted.

| Provider | Exact API model ID | Change | Allowed explicit controls |
| --- | --- | --- | --- |
| OpenAI | `gpt-5.6-luna` | Retained from initial scope | none, low, medium, high, xhigh, max |
| OpenAI | `gpt-5.6-terra` | Retained from initial scope | none, low, medium, high, xhigh, max |
| OpenAI | `gpt-5.6-sol` | Retained from initial scope | none, low, medium, high, xhigh, max |
| OpenAI | `gpt-6-astra` | Retained from initial scope | low, medium, high, xhigh, max |
| Anthropic | `claude-sonnet-5` | Retained from initial scope | none, low, medium, high, xhigh, max |
| Anthropic | `claude-opus-5` | Retained from initial scope | none, low, medium, high, xhigh, max |
| Anthropic | `claude-fable-5-1` | Retained from initial scope | low, medium, high, xhigh, max |
| xAI | `grok-4.6` | Retained from initial scope | low, medium, high, xhigh |
| xAI | `grok-build-0.1` | Retained from initial scope | No verified explicit control; omission supported |
| MiniMax | `MiniMax-M3` | Introduced by revision | none; named levels unverified |
| DeepSeek | `deepseek-v4-pro` | Existing model retained | none, high, max; low unverified as an independent level |
| DeepSeek | `deepseek-flash` | Introduced: DeepSeek V4.1 Flash | none, low, high, max |
| Alibaba / Qwen | `qwen3.8-max` | Introduced by revision | none, low, medium, xhigh |
| Alibaba / Qwen | `qwen3.8-flash` | Introduced by revision | none, low, medium, xhigh |
| Moonshot | `kimi-k3` | Introduced by revision | low, high, max |
| Zhipu / Z.ai | `glm-5.3` | Introduced by revision | low, high, max |
| Zhipu / Z.ai | `glm-5.3-flash` | Introduced by revision | low, high, max |

Only independent native effort levels are admitted. Documented aliases such as
Qwen high/max → xhigh or DeepSeek Flash medium/xhigh → high are deliberately
rejected, not forwarded or remapped. A typed `NonCanonicalEffort` failure
explains this product policy; it must not claim the upstream API rejects them.
Missing effort support alone never blocks a model's omitted-effort path.

## Retirements

Remove these **22 currently embedded model entries** from active selection:

| Provider | Retired IDs |
| --- | --- |
| OpenAI | `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.5` |
| Anthropic | `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5` |
| xAI | `grok-4.20-reasoning`, `grok-4.20-non-reasoning`, `grok-4-1-fast-reasoning`, `grok-4-1-fast-non-reasoning` |
| MiniMax | `MiniMax-M2.7`, `MiniMax-M2.7-highspeed`, `MiniMax-M2.5`, `MiniMax-M2.5-highspeed` |
| DeepSeek | `deepseek-v4-flash` |
| Alibaba / Qwen | `qwen3.5-plus`, `qwen3.5-flash` |
| Moonshot | `kimi-k2.6`, `moonshot-v1-128k` |
| Zhipu / Z.ai | `glm-5.1`, `glm-4.7-flash` |
| Google | `gemini-3.5-flash` |

Also retire the **OpenRouter provider**, including `openrouter/auto` and every
model configured through that provider. No OpenRouter-backed provider setup,
probe, automatic route, explicit call or fallback remains available.

Retirement is model-family aware for officially equivalent versioned IDs:
`claude-haiku-4-5-20251001` is retired with `claude-haiku-4-5`; other officially
identified snapshots/aliases of a listed retired model cannot bypass its tombstone.
Use a finite reviewed alias table, not broad version-prefix bans. Keep the
existing `grok-code-fast-1` retirement. DeepSeek's legacy Flash alias being
accepted/redirected upstream does not override DUUMBI retirement.

Google has no remaining curated model: Gemini 3.5 Flash is removed from active
catalog/setup offers and automatic routes. This is not a blanket retirement of
all possible Google models: a non-retired explicit legacy Gemini/custom model
keeps omitted-effort compatibility. No replacement Google model is introduced.

## Migration and observable behavior

- Tombstones take precedence over embedded, cached, refreshed and provider-discovered
  catalogs, accessible-model records, explicit overrides and default/fallback routes.
  Apply them before capability lookup and legacy compatibility exceptions.
- A retired explicit model returns `RetiredModel`; OpenRouter returns
  `RetiredProvider`, before HTTP or graph mutation. Do not silently choose a
  replacement, even with omitted effort. A remediation message points to
  `/provider`; it does not ask users to manage a new default model.
- Persisted provider/config and V1 catalog formats remain readable. Retain
  credentials and old config values until the user edits them. Old entries may
  be displayed as retired when opening provider management; they are never active.
  Loading a mixed old/new config must not prevent unrelated commands or a valid
  provider from working. Do not add unrelated startup warnings.
- Automatic selection with no explicit model filters retired candidates before
  ranking. It may select a valid target model using existing policy; that is not
  a rewrite of an explicit retired choice. Retired-only configuration has a
  clear local no-eligible-provider outcome and no network call.
- CLI, REPL/TUI, Studio, examples, docs, curated publisher inputs and tests share
  this same target/retirement policy. `/provider` remains the entry point;
  `/model` remains compatibility-only. OpenRouter enum/config parsing can stay
  as a migration tombstone, but its runtime dispatch/onboarding is removed.
- Public catalog deployment is outside this spec task. Implementation must remain
  correct with old published catalogs; rollout must not depend on deleting them.

## Verification obligations

Exercise every listed retirement and the pinned Haiku alias through selection,
explicit factory/direct-client calls, probe, stale/refresh adoption and fallback.
Assert zero sends and no substitute for explicit retirement. Verify mixed configs,
retired-only configs, retained credentials, non-retired legacy custom models,
17 active curated rows/8 providers, deterministic publication and all UI surfaces.
