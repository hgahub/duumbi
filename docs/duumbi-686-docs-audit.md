# DUUMBI-686 Docs Audit

## Summary

Issue #686 reconciles the legacy source-repo documentation surfaces with the
canonical public documentation site in `hgahub/duumbi-web/docs`.

The public source of truth is now `duumbi-web/docs/src/content/docs/`. The
source repo keeps internal architecture, automation, testing, provider catalog
source material, and implementation evidence. The old mdbook tree under
`sites/docs/` is removed after its useful reader-facing content is represented
in the canonical docs site.

Canonical docs implementation evidence:

- `hgahub/duumbi-web` main commit
  `13446d30ab7d23f2ba7cd368e6912e707289be67`
- `npm run build` from `/Users/heizergabor/space/hgahub/duumbi-web/docs`
  succeeded after the docs updates.

## Routing Rules

| Audience | Action |
| --- | --- |
| Public DUUMBI user | Rewrite or migrate to `duumbi-web/docs/src/content/docs/` |
| Contributor or implementation agent | Keep in `hgahub/duumbi/docs/` when current |
| Historical implementation evidence | Keep internal and label as historical when referenced |
| Generated or local cruft | Delete when present; keep `.gitignore` coverage |
| Obsolete mdbook content | Delete after canonical docs coverage exists |

## `sites/docs/src` Inventory

| Source path | Audience | Action | Destination | Rationale | Verification |
| --- | --- | --- | --- | --- | --- |
| `sites/docs/src/SUMMARY.md` | Public docs reader | Delete | `duumbi-web/docs/astro.config.mjs` sidebar | Starlight sidebar replaces mdbook summary. | Sidebar includes migrated provider, Query, support, registry, and reference pages. |
| `sites/docs/src/introduction.md` | Public docs reader | Rewrite | `duumbi-web/docs/src/content/docs/getting-started/introduction.md` | Core product framing remains useful but provider and Query wording needed refresh. | Introduction describes semantic graph pipeline, Query-first workflow, and current direct providers. |
| `sites/docs/src/getting-started/installation.md` | Public docs reader | Rewrite | `duumbi-web/docs/src/content/docs/getting-started/installation.md` | Old install claims must not overstate `cargo install duumbi` while #687 is open. | Installation page uses source build guidance and marks packaged install as pending. |
| `sites/docs/src/getting-started/quickstart.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/getting-started/quickstart.md` | Existing canonical quickstart already covers first build flow. | Quickstart remains linked from sidebar and install page. |
| `sites/docs/src/getting-started/workspace.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/getting-started/quickstart.md`, `duumbi-web/docs/src/content/docs/guides/module-system.md` | Workspace concepts are covered by quickstart and module docs; no one-to-one page needed. | Audit records coverage; no active mdbook link remains. |
| `sites/docs/src/cli/overview.md` | Public docs reader | Rewrite | `duumbi-web/docs/src/content/docs/reference/cli.md` | CLI reference must reflect current provider, Query, registry, and dependency surfaces. | CLI page includes core, provider, Query/REPL, intent, dependency, registry, and Studio commands. |
| `sites/docs/src/cli/init.md` | Public docs reader | Merge | `duumbi-web/docs/src/content/docs/reference/cli.md`, quickstart | Command-specific mdbook page is better represented in compact reference and quickstart. | CLI page lists `duumbi init`; quickstart shows first use. |
| `sites/docs/src/cli/build.md` | Public docs reader | Merge | `duumbi-web/docs/src/content/docs/reference/cli.md` | Command-specific mdbook page is stale duplication. | CLI page lists build behavior. |
| `sites/docs/src/cli/run.md` | Public docs reader | Merge | `duumbi-web/docs/src/content/docs/reference/cli.md` | Command-specific mdbook page is stale duplication. | CLI page lists run behavior. |
| `sites/docs/src/cli/check.md` | Public docs reader | Merge | `duumbi-web/docs/src/content/docs/reference/cli.md` | Command-specific mdbook page is stale duplication. | CLI page lists check behavior. |
| `sites/docs/src/cli/describe.md` | Public docs reader | Merge | `duumbi-web/docs/src/content/docs/reference/cli.md` | Command-specific mdbook page is stale duplication. | CLI page lists describe behavior. |
| `sites/docs/src/cli/add.md` | Public docs reader | Rewrite | `duumbi-web/docs/src/content/docs/getting-started/first-ai-mutation.md`, `reference/cli.md` | Write-capable Agent behavior needs clear contrast with read-only Query. | Query guide and CLI reference distinguish Query, Agent, and Intent. |
| `sites/docs/src/cli/undo.md` | Public docs reader | Merge | `duumbi-web/docs/src/content/docs/reference/cli.md` | Command-specific mdbook page is stale duplication. | CLI page lists undo behavior. |
| `sites/docs/src/cli/studio.md` | Public docs reader | Merge | `duumbi-web/docs/src/content/docs/reference/cli.md` | Studio launch command belongs in compact CLI reference for now. | CLI page lists `duumbi studio [--port N]`. |
| `sites/docs/src/jsonld/overview.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/reference/op-types.md`, `reference/type-system.md` | Current canonical reference splits op and type material. | Reference pages remain linked from sidebar. |
| `sites/docs/src/jsonld/namespace.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/getting-started/quickstart.md`, reference pages | Namespace details are visible in JSON-LD examples and reference material. | Quickstart example uses DUUMBI JSON-LD context. |
| `sites/docs/src/jsonld/ops.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/reference/op-types.md` | Canonical op-type page is the active reference. | Sidebar links Op Types. |
| `sites/docs/src/jsonld/types.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/reference/type-system.md` | Canonical type-system page is the active reference. | Sidebar links Type System. |
| `sites/docs/src/jsonld/schema.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/reference/error-codes.md`, op/type reference | Schema validation is user-visible through validation errors and reference pages. | CLI and error-code docs remain linked. |
| `sites/docs/src/jsonld/errors.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/reference/error-codes.md` | Canonical error-code page is the active reference. | Sidebar links Error Codes. |
| `sites/docs/src/modules/overview.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/guides/module-system.md` | Existing canonical module guide covers registry/module concepts. | Module System guide remains linked. |
| `sites/docs/src/modules/imports-exports.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/guides/module-system.md`, `getting-started/multi-module.md` | Multi-module material is already better placed in canonical guides. | Sidebar links both pages. |
| `sites/docs/src/modules/deps.md` | Public docs reader | Represent | `duumbi-web/docs/src/content/docs/guides/module-system.md`, registry pages | Dependency commands and registry behavior are covered in module/registry docs. | Module guide and registry group remain linked. |
| `sites/docs/src/modules/stdlib.md` | Public docs reader | Follow-up | `duumbi-web/docs` future stdlib page | Stable stdlib docs need current API evidence; not required for preview blocker. | Recorded as non-blocking follow-up. |
| `sites/docs/src/architecture/overview.md` | Public docs reader | Rewrite | `duumbi-web/docs/src/content/docs/getting-started/introduction.md` | High-level architecture is useful when rewritten for public readers. | Introduction includes compiler pipeline diagram. |
| `sites/docs/src/architecture/pipeline.md` | Public docs reader | Rewrite | `duumbi-web/docs/src/content/docs/getting-started/introduction.md` | Pipeline is core product orientation. | Introduction includes JSON-LD -> graph -> Cranelift pipeline. |
| `sites/docs/src/architecture/ai-mutation.md` | Public docs reader | Rewrite | `duumbi-web/docs/src/content/docs/guides/query-mode.md`, `first-ai-mutation.md` | AI mutation docs must distinguish Query from Agent/Intent. | Query guide documents read-only default and explicit write handoffs. |
| `sites/docs/src/contributing.md` | Contributor | Represent | `duumbi-web/docs/src/content/docs/community/support.md`, source repo `AGENTS.md` | Public contribution/support entry belongs in docs site; implementation rules stay internal. | Community page links issues, Discussions, Discord, and source repo. |

## `docs` Inventory

| Source path | Audience | Action | Destination | Rationale | Verification |
| --- | --- | --- | --- | --- | --- |
| `docs/architecture.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/architecture.md` | Source architecture notes are useful for implementation agents. | File remains in source repo. |
| `docs/coding-conventions.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/coding-conventions.md` | Source conventions govern implementation quality. | File remains in source repo. |
| `docs/provider-catalog.md` | Mixed public/internal | Represent publicly, keep internal source material | `duumbi-web/docs/src/content/docs/reference/provider-catalog.md` | #675 provider catalog content must be discoverable publicly, while source details remain useful internally. | Provider catalog page links public static JSON/SHA artifacts. |
| `docs/mcp-model-telemetry.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/mcp-model-telemetry.md` | Operational telemetry detail is not public getting-started material. | File remains in source repo. |
| `docs/rewrite-engine.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/rewrite-engine.md` | Rewrite-engine design is source architecture material. | File remains in source repo. |
| `docs/modes/README.md` | Contributor/agent | Keep internal, represent public overview | `duumbi-web/docs/src/content/docs/guides/query-mode.md` | Mode implementation roadmap is internal; public docs need user-facing Query/Agent/Intent framing. | Query guide added publicly. |
| `docs/modes/query-mode-spec.md` | Mixed public/internal | Represent publicly, keep internal contract | `duumbi-web/docs/src/content/docs/guides/query-mode.md` | Delivered v1 Query contract is user-facing; architecture details stay internal. | Query guide cites read-only behavior and handoffs. |
| `docs/modes/implementation-tasks.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/modes/implementation-tasks.md` | Development backlog is not public docs. | File remains in source repo. |
| `docs/modes/operating-modes-research.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/modes/operating-modes-research.md` | Research notes are not stable user documentation. | File remains in source repo. |
| `docs/automation/*.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/automation/` | Workflow automation docs govern agentic delivery, not public product usage. | Files remain in source repo. |
| `docs/testing/*.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/testing/` | Manual and phase walkthroughs are QA/development evidence. | Files remain in source repo. |
| `docs/e2e/scoring.md` | Contributor/agent | Keep internal | `hgahub/duumbi/docs/e2e/scoring.md` | Evaluation scoring is internal until #689 refreshes public eval claims. | File remains in source repo. |
| `docs/e2e/corpus/*` | Contributor/agent | Keep internal as grouped fixtures | `hgahub/duumbi/docs/e2e/corpus/` | Prompt/golden fixtures support deterministic testing and are not public guides. | Group covers all corpus `.txt`, `.gold.yaml`, and `.ref.py` files. |
| `docs/e2e/intents/*` | Contributor/agent | Keep internal as grouped generated intent fixtures | `hgahub/duumbi/docs/e2e/intents/` | Generated intent YAML supports tests and should not be presented as current public evidence. | Group covers all intent YAML files. |
| `docs/e2e/results/*20260331*` | Contributor/agent | Keep internal historical evidence | `hgahub/duumbi/docs/e2e/results/` | Results predate #689 scaled eval work and must not be marketed as current preview proof. | Group covers all current result JSON files. |
| `docs/.DS_Store` | None | Delete if present | None | Local macOS cruft is not documentation. | No file found in this worktree; `.gitignore` already covers `.DS_Store`. |
| `docs/.obsidian/` | None | Delete if present | None | Vault metadata does not belong in source repo docs. | No directory found in this worktree; `.gitignore` already covers `docs/.obsidian/`. |

## Deletion Preconditions

- Useful mdbook content is represented in canonical Starlight docs.
- Provider catalog, Query-first guidance, installation constraints, and
  community/support paths are present in `duumbi-web/docs`.
- Active source-repo references no longer tell contributors to update
  `sites/docs/src/`.
- Search checks are recorded in implementation evidence.

## Follow-Up Items

- Add a dedicated public stdlib reference when the stdlib API surface is stable
  enough for preview documentation.
- Revisit public eval/result language after #689 lands current scaled eval
  evidence.
- Update installation docs again after #687 provides a verified packaged
  install channel.
