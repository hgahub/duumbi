---
tags:
  - project/duumbi
  - concept/business
status: final
created: 2026-03-12
updated: 2026-03-12
related_maps:
  - "[[DUUMBI Core Concepts Map]]"
---
# Open Source Monetization (Model B)

DUUMBI follows the **Fully Open + Managed Service** monetization model. All source code — including the Intent Engine, multi-agent orchestration, and registry server — is open source under AGPLv3. Revenue comes from hosted services, not from restricting access to code.

## Core Principle

> The software is free. The operations are valuable.

## License Structure

| Component | License | Rationale |
|---|---|---|
| DUUMBI CLI (compiler, validator, all ops) | AGPLv3 | Fully open; AGPLv3 prevents cloud competitors from offering it as a service without contributing back |
| JSON-LD Core Schema | CC0 (Public Domain) | The schema is a standard — anyone should use it freely |
| Registry server code | AGPLv3 | Open source; self-hostable by anyone |
| Intent Engine code | AGPLv3 | Open source; included in the CLI |

## Revenue Sources

- **Hosted registry** (`registry.duumbi.dev`) — private modules, binary cache CDN
- **Managed telemetry dashboard** — trace visualization, node-level error mapping
- **Support tiers** — prioritized response, dedicated support engineering
- **Enterprise managed deployment** — on-premise setup, compliance assistance

## Pricing Tiers

- **Free (self-hosted)**: unlimited, full functionality, community support
- **Pro ($19/mo)**: hosted registry + CDN + private modules + telemetry dashboard
- **Team ($49/mo/seat)**: + SSO + audit log + SLA
- **Enterprise (custom)**: + on-premise deployment + dedicated support + compliance

## Phased Rollout

- Phase 0-4: donation only (GitHub Sponsors + OpenCollective)
- Phase 5 (Registry launch): Pro tier starts
- Phase 6-7: Team tier starts

## Related

- [[DUUMBI - Post-MVP Roadmap]] — full business plan and monetization timeline
- [[Graph Repository Architecture]] — the registry architecture that underpins Pro/Team tiers
