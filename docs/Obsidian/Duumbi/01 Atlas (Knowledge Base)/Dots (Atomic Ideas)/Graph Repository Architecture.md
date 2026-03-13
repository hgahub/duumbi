---
tags:
  - project/duumbi
  - concept/architecture
status: final
created: 2026-03-12
updated: 2026-03-12
related_maps:
  - "[[DUUMBI Core Concepts Map]]"
---
# Graph Repository Architecture

DUUMBI uses a **three-layer storage model** for semantic graph modules, inspired by npm/Cargo but operating at the graph level rather than the source code level.

## Three Layers (Resolution Order)

1. **Workspace** (`.duumbi/graph/`) — local development, highest priority. "Your code always wins."
2. **Vendor** (`.duumbi/vendor/`) — VCS-committed dependency snapshots for reproducible offline builds.
3. **Cache** (`.duumbi/cache/`) — registry-resolved modules downloaded on demand. Not in VCS.

If a module is not found in any layer → registry fetch. If registry fails → `E011 DependencyError`.

## Namespace Scoping

Format: `@<scope>/<module-name>`. Scopes: `@duumbi/` (official stdlib), `@<user>/` (personal), `@<org>/` (organization). Unscoped names are workspace-local only.

## Key Mechanisms

- **Semantic Hash**: deterministic fingerprint of a module's structure and values (excluding `@id`). Used for binary cache keying.
- **Manifest** (`manifest.toml`): module metadata — name, version, exports, license, min compiler version.
- **Lock file** (`deps.lock`): exact versions, sources, and integrity hashes for reproducible builds.

## Security

- Supply chain: lockfile integrity hash (SHA-256) + semantic hash
- Dependency confusion: workspace always wins over external modules
- Air-gapped: `duumbi deps vendor --all` + `duumbi build --offline`

## Related

- [[DUUMBI - Graph Repository Architecture]] — full architectural specification
- [[Open Source Monetization Model B]] — the registry is the primary monetization point
- [[Compilation Pipeline]] — how resolved modules feed into compilation
