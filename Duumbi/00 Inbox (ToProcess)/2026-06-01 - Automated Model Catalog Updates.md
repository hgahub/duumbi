# 2026-06-01 - Automated Model Catalog Updates

## Source
- Surface: Slack
- Link: https://hgabor.slack.com/archives/C08SK7E6R7T/p1780344708914099
- Submitted by: Slack user `U4SMDRG9M`

## Raw input
The user asked to capture a Hungarian-language idea as DUUMBI Stage 1 intake.

Natural English translation:

> We have currently hard-coded which providers and which provider models DUUMBI uses. At regular intervals, an Action should run that collects each configured provider's newest models, plus their statistics, metrics, and benchmarks. It should place this under the DUUMBI website in a file and also create a hash file containing that file's hash. When the user starts DUUMBI, once per day it should check whether the hash value changed. If it changed, DUUMBI should notify the user that an update is available and let them download it if they want. If the user approves, DUUMBI should download it and update the usable models, locally stored model statistics, and availability information in settings.

## Interpreted intent
DUUMBI should move from a fully release-embedded, static LLM model catalog toward an opt-in, remotely updated model metadata feed. A scheduled GitHub Action or equivalent automation would publish fresh provider/model metadata and a hash under the DUUMBI website, while the CLI/TUI/Studio startup flow would check the hash at most once per day and offer a user-approved update path for local model availability and statistics.

## Classification
feature proposal; architecture decision; product decision

## Clarifications
### Answered
- The request is intended as Stage 1 intake and should be captured for later triage.
- The source idea explicitly requires user approval before downloading and applying updated model metadata.
- The proposed refresh source is a periodically generated file hosted under the DUUMBI website, with a companion hash file.

### Open
- Which providers should be included in the automated collection first: only configured providers, all DUUMBI-supported providers, or both with filtering at client update time?
- What exact metrics are trusted enough to publish: provider-listed context/pricing/capability metadata, DUUMBI-measured performance, external benchmarks, or a normalized blend?
- Should updated metadata affect automatic routing immediately after user approval, or only inform availability/statistics until a later release changes routing policy?
- How should DUUMBI handle provider API terms, benchmark licensing, and stale or conflicting model metadata?

## Relevant DUUMBI context
- `AGENTS.md` says provider setup should collect credentials and verify access, while model choice should stay internal to the model catalog, routing inputs, or performance knowledge.
- `src/agents/model_catalog.rs` currently defines a versioned internal LLM model catalog with static model metadata and deterministic routing inputs.
- `src/config.rs` documents the legacy `model` field and says newer configs resolve concrete model selection from DUUMBI's versioned internal model catalog.
- `sites/docs/src/getting-started/workspace.md` documents the workspace `.duumbi/config.toml` as containing LLM provider/model/API key information.
- `docs/architecture.md` describes `.duumbi/config.toml` as the workspace configuration surface.
- Duplicate checks searched the repository for the Slack thread URL and similar model catalog/update/hash wording; no existing captured note for this Slack thread was found in the available local source tree.
- The configured Obsidian vault path from `AGENTS.md` was not present in this container, so this Stage 1 note was captured under the repository-local `Duumbi/00 Inbox (ToProcess)/` path.

## Initial routing recommendation
GitHub Discussion idea first, then likely GitHub issue after triage if accepted. The idea affects provider/model UX, update trust, website-hosted artifacts, CLI/TUI startup checks, local configuration/state, and possibly Studio/docs, so triage should decide whether to split it into product and technical follow-up work.

## Requested follow-up
- Preserve the Hungarian source idea by translating it into English for DUUMBI triage.
- Route the captured idea through Stage 4 triage; do not implement it during Stage 1.

## Notes
- Facts:
  - The current available source includes a static `src/agents/model_catalog.rs` with embedded model IDs and scoring fields.
  - Existing guidance says `/provider` should remain the user-facing provider setup entry point and `/model` should stay compatibility-only.
  - The user explicitly requested a once-daily startup hash check and user-approved download/update flow.
- Assumptions:
  - "Action" means a scheduled GitHub Action or similar CI automation owned by the DUUMBI project.
  - "DUUMBI website" likely means a static, versioned public artifact location under the docs/site deployment rather than an arbitrary runtime API.
  - "Local settings" may include workspace `.duumbi/config.toml`, user-level DUUMBI state, or a separate cache; triage should decide the correct persistence boundary.
- Recommendations:
  - Treat this as a product/architecture proposal rather than a simple code task because it introduces remote update trust, artifact signing/hash semantics, startup UX, provider metadata normalization, and routing-policy implications.
  - Prefer an opt-in update flow that does not change credentials or mutate user provider choices, and keep model choice internal to DUUMBI routing as current guidance requires.
  - Consider stronger integrity controls than a plain companion hash if the metadata file is fetched remotely, such as signed metadata or release-pinned trust roots.
