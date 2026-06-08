# DUUMBI Provider Model Catalog

DUUMBI keeps model choice inside the model catalog and routing policy. Provider
credentials stay in provider configuration or credential storage, not in catalog
metadata.

## Accepted V1 Providers

| Provider | Environment Variable | Config Key |
| --- | --- | --- |
| Anthropic | `ANTHROPIC_API_KEY` | `anthropic` |
| OpenAI | `OPENAI_API_KEY` | `openai` |
| xAI | `XAI_API_KEY` | `xai` |
| MiniMax | `MINIMAX_API_KEY` | `minimax` |
| DeepSeek | `DEEPSEEK_API_KEY` | `deepseek` |
| Alibaba Cloud Model Studio (Qwen) | `DASHSCOPE_API_KEY` | `qwen` |
| Moonshot AI (Kimi) | `MOONSHOT_API_KEY` | `moonshot` |
| Zhipu AI (GLM) | `ZHIPUAI_API_KEY` | `zhipu` |
| Google Gemini | `GEMINI_API_KEY` | `gemini` |

`xai` is the canonical config key for xAI/Grok models. Existing `grok` configs
are compatibility-only legacy input and should not be used in new examples.
OpenRouter is excluded from the V1 direct-provider catalog because it is an
aggregator surface rather than a stable direct-provider target.

## Catalog Artifacts

The V1 publisher produces deterministic semantic artifacts:

- `model-catalog.v1.json`
- `model-catalog.v1.sha256`

The JSON artifact includes the schema version, content timestamp, source
provenance, accepted provider metadata, provider discovery status, model routing
metadata, and user-facing change summary. Run-specific workflow evidence stays
outside the JSON catalog so rerunning a scheduled workflow does not change the
adoption hash when semantic input is unchanged.

Accepted public URLs for closure verification:

- `https://docs.duumbi.dev/model-catalog/v1/model-catalog.v1.json`
- `https://docs.duumbi.dev/model-catalog/v1/model-catalog.v1.sha256`

This repository contains the generator and validation path. Public static
publication may require a separate docs-site update in `hgahub/duumbi-web`.

## Local Update State

Refreshed catalog state is user-level installation state:

- `~/.duumbi/model-catalog/current.json`
- `~/.duumbi/model-catalog/current.sha256`
- `~/.duumbi/model-catalog/state.json`

The client checks the remote hash first and stays quiet when the installed,
skipped, deferred, or disabled state means no user action is useful. Users can
approve, skip, remind later, or disable checks through provider-management
surfaces.

Approval revalidates the reviewed hash, downloads the catalog, verifies
SHA-256, validates schema/provider/model fields, and writes the catalog
atomically. Hash mismatch, unsupported schema, invalid provider/model entries,
download failures, offline startup, and partial local writes fail closed and
keep the previous local catalog or embedded catalog active.

Catalog adoption must not overwrite provider credentials, API key environment
variable names, auth-token environment variable names, base URLs, roles,
provider ordering, workspace config, user config, or explicit model overrides.
