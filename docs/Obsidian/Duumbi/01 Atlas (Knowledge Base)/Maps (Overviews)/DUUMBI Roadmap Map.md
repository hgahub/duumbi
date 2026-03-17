---
tags:
  - project/duumbi
  - map/roadmap
status: active
created: 2026-03-12
updated: 2026-03-17
---
# DUUMBI — Development Roadmap (Hub)

> Master overview. Every milestone links to its own note. Status reflects GitHub issues, milestones, and actual completion.

---

## Status Summary

| Phase | Title | GitHub | Status |
|-------|-------|--------|--------|
| 0 | Proof of Concept | 9/9 ✓ | ✅ Done |
| 1 | Usable CLI | 11/11 ✓ | ✅ Done |
| 2 | AI Integration | 13/13 ✓ | ✅ Done |
| 3 | Web Visualizer | 7/7 ✓ | ✅ Done |
| 4 | Interactive CLI & Module System | 16/16 ✓ | ✅ Done |
| 5 | Intent-Driven Development | 22/22 ✓ | ✅ Done |
| 6 | DUUMBI Studio | 47/47 ✓ | ✅ Done |
| 7 | Registry & Distribution | 35/37 ✓ | 🔄 In Progress (2 infra) |
| 8 | Registry Auth & User Management | 22/22 ✓ | ✅ Done |
| 9a | Type System Completion | 37/37 ✓ | ✅ Done |
| 9 | Build Excellence & Multi-LLM | – | ⏳ Planned |
| 10 | Intelligent Context & Knowledge Graph | – | ⏳ Planned |
| 11 | CLI UX & Developer Experience | – | ⏳ Planned |
| 12 | Dynamic Agent System & MCP | – | ⏳ Planned |
| 13 | Self-Healing & Telemetry | – | ⏳ Planned |
| 14 | Marketing & Go-to-Market | – | ⏳ Planned (continuous) |

**Current delivery focus:** Phase 9 — Build Excellence & Multi-LLM (Phase 9a fully complete, 37/37 ✓)

---

## Dependency Graph

```
Phase 0-8 (DONE)
    │
    ▼
Phase 9a: Type System Completion (String, Array, Struct, Ownership/Lifetimes)
    │
    ▼
Phase 9: Build Excellence & Multi-LLM
    │
    ├──→ Phase 10: Intelligent Context & Knowledge Graph
    │       │
    │       ├──→ Phase 11: CLI UX & Developer Experience
    │       │
    │       └──→ Phase 12: Dynamic Agent System & MCP
    │               │
    │               └──→ Phase 13: Self-Healing & Telemetry
    │
    └──→ Phase 14: Marketing & Go-to-Market (continuous, starts after Phase 9 stable)
              ↑ strengthens as Phase 10, 11, 12 complete
```

---

## Milestone Notes

### MVP (Phase 0–3) ✅

- [[DUUMBI - Phase 0 - Proof of Concept]] — JSON-LD → Cranelift → native binary ✅
- [[DUUMBI - Phase 1 - Usable CLI]] — CLI commands, fibonacci, f64/bool ✅
- [[DUUMBI - Phase 2 - AI Integration]] — `duumbi add`, undo, 20/20 benchmark ✅
- [[DUUMBI - Phase 3 - Web Visualizer]] — Cytoscape.js + axum + WebSocket ✅

### Post-MVP Foundation (Phase 4–8) ✅

- [[DUUMBI - Phase 4 - Interactive CLI & Module System]] — REPL, module system, stdlib ✅
- [[DUUMBI - Phase 5 - Intent-Driven Development]] — Intent spec, Coordinator, Verifier ✅
- [[DUUMBI - Phase 6 - DUUMBI Studio]] — Leptos SSR, C4 drill-down, chat UI ✅
- [[DUUMBI - Phase 7 - Registry & Distribution]] — publish, install, lockfile v1 🔄
- [[DUUMBI - Phase 8 - Registry Auth & User Management]] — OAuth, tokens, device code flow ✅

### Type System & Build (Phase 9a–9) 🔄

- [[DUUMBI - Phase 9a - Type System Completion]] — String, Array, Struct, ownership/lifetimes — all delivered ✅
- [[DUUMBI - Phase 9 - Build Excellence & Multi-LLM]] — Stdlib, multi-provider, autoresearch benchmarks ⏳

### Intelligence & UX (Phase 10–11) ⏳

- [[DUUMBI - Phase 10 - Intelligent Context & Knowledge Graph]] — Automatic context, codebase awareness, knowledge graph ⏳
- [[DUUMBI - Phase 11 - CLI UX & Developer Experience]] — Colors, menus, session management, Copilot CLI-level UX ⏳

### Agent Platform (Phase 12–13) ⏳

- [[DUUMBI - Phase 12 - Dynamic Agent System & MCP]] — MCP server, dynamic agents, knowledge persistence ⏳
- [[DUUMBI - Phase 13 - Self-Healing & Telemetry]] — OpenTelemetry, repair loop, autonomous improvement ⏳

### Go-to-Market (Phase 14) ⏳

- [[DUUMBI - Phase 14 - Marketing & Go-to-Market]] — duumbi.dev, content, community ⏳

---

## Kill Criterion Summary

| Phase | Kill Criterion | Result |
|-------|----------------|--------|
| 0 | `add(3,5)` → binary prints `8` | ✅ |
| 1 | External dev installs + runs in < 10 min | ✅ |
| 2 | > 70% correct on 20-command benchmark | ✅ 20/20 |
| 3 | 3/3 devs confirm faster than raw JSON-LD | ✅ |
| 4 | `abs(-7) = 7` via init → 2-module → binary | ✅ |
| 5 | `double(21)=42` via intent pipeline | ✅ |
| 6 | 3/3 devs faster navigation web vs CLI | ✅ |
| 7 | Deterministic hash + publish+install + offline build | 🔄 |
| 8 | GitHub OAuth → token → device code flow login | ✅ |
| 9a | String + Array + ownership validation rejects use-after-move | ✅ |
| 9 | 5 showcases × 2+ LLM providers = flawless | ⏳ |
| 10 | Existing graph recognition + modular add + auto-context | ⏳ |
| 11 | 3/3 new users succeed via `/` menu in 10 min + `/resume` works | ⏳ |
| 12 | External MCP client → graph mutation + dynamic agent team | ⏳ |
| 13 | Runtime error → nodeId → valid patch → tests green | ⏳ |
| 14 | duumbi.dev live + 3 demos + article + 50 stars | ⏳ |

---

## Business Milestones

| Timeframe | Event | Revenue Model |
|-----------|-------|---------------|
| Phase 0–8 | Community building | Donation (GitHub Sponsors) |
| Phase 9a–9 | Type system + build stabilization | Free tier strengthened |
| Phase 10–11 | PRO value proposition | PRO tier ($19/month) |
| Phase 12 | Multi-agent platform | TEAM tier ($49/month/seat) |
| Phase 13 | Self-healing | TEAM tier enhanced |
| 18+ months | Enterprise interest | Enterprise (custom pricing) |

---

## Estimated Timeline (Solo Developer)

| Phase | Title | Estimate | Prerequisite |
|-------|-------|----------|-------------|
| 9a | Type System Completion | 8–12 wks | Phase 8 done |
| 9 | Build Excellence & Multi-LLM | 8–10 wks | Phase 9a |
| 10 | Intelligent Context & Knowledge Graph | 8–10 wks | Phase 9 |
| 11 | CLI UX & Developer Experience | 6–8 wks | Phase 10 (M10-SESSION) |
| 12 | Dynamic Agent System & MCP | 10–12 wks | Phase 9 + 10 |
| 13 | Self-Healing & Telemetry | 6–8 wks | Phase 12 |
| 14 | Marketing & Go-to-Market | Continuous | Phase 9 stable |

**Total estimated: ~46–60 weeks for Phase 9a–13 (sequential).**

---

## Future Vision (Phase 15+)

| # | Direction | Description |
|---|-----------|-------------|
| 15a | **LLVM Backend** | Inkwell crate → LLVM IR → more targets (WASM, RISC-V) |
| 15b | **Projectional Editing** | Graph ↔ pseudo-code ↔ diagram view switching |
| 15c | **Diagram-Driven Development** | Draw C4 diagrams → automatic graph generation |
| 15d | **DUUMBI Playground** | Browser-based WASM compiler + visualizer |
| 15e | **IDE Plugin** | VS Code extension: schema autocomplete, live graph preview |
| 15f | **Code Import** | Python/JS/Rust → JSON-LD graph conversion |

---

## Related Documents

- [[DUUMBI - PRD]] — Long-term vision
- [[DUUMBI - MVP Specification]] — Phase 0–3 specification
- [[DUUMBI - Post-MVP Roadmap]] — Business plan, monetization
- [[DUUMBI - Graph Repository Architecture]] — Phase 7 architecture
- [[DUUMBI - Architecture Diagram]] — Technical architecture
- [[DUUMBI - Tools and Components]] — Tech stack
- [[DUUMBI - Glossary]] — Terminology


> [!important] **Timeline update (2026-03-14):** Phase 9a estimate increased to 10–14 weeks (from 8–12) due to addition of Result/Option error handling system. Total sequential estimate: **~48–62 weeks for Phase 9a–13.**


> [!important] **Kill Criterion update (2026-03-14):** Phase 9 kill criterion refined: "flawlessly" replaced with "≥95% success rate (19/20 runs)" per showcase per provider. Phase 14 GitHub star target increased from 50 to 200 (within 30 days of HN launch).


> [!note] **Phase 15+ addition (2026-03-14):** Added "15g: General Enum Type" — user-defined tagged unions with arbitrary variants, pattern matching, exhaustiveness checking. Deferred from Phase 9a where only Result/Option (2-variant) tagged unions exist.


> [!important] **Scope update (2026-03-14):** Phase 9 estimate increased from 8–10 weeks to **10–14 weeks** due to addition of File I/O stdlib, JSON parse/serialize, TCP socket, and HTTP client. Phase 9a also adds: multi-language projection (`--lang rust/python/go`), two-phase generation strategy, LLM-friendly validator error messages, and automatic prompt evolution system. **Revised total sequential estimate: ~52–70 weeks for Phase 9a–13.**


> [!important] **Major scope update (2026-03-15):** Three critical additions to close the production-readiness gap:
> 1. **Phase 9a: FFI (Foreign Function Interface)** — unsafe ExternFunction + UnsafeBlock + RawPointer. The entire C library ecosystem becomes accessible. E050–E053 validator rules.
> 2. **Phase 9: TLS (LibreSSL/OpenSSL) + SQLite (vendored) + Event Loop (poll-based multi-connection server).** The showcase #4 is now a production-viable SMTP server with TLS, database, and concurrent connections.
> 3. **Phase 14: Ecosystem Seed Strategy** — 10 Tier 1 stdlib modules pre-installed, 10 Tier 2 within 3 months. FFI makes wrapping C libraries a 1–3 day task instead of weeks.
>
> **Revised time estimates:** Phase 9a: 10–14 wks (unchanged). Phase 9: 12–16 wks (was 10–14). **Total sequential: ~54–74 weeks for Phase 9a–13.**
>
> **Why this matters:** These three changes transform DUUMBI from "demo-grade calculator" to "production-viable tool with TLS, database, multi-connection server, and access to the C ecosystem via FFI."


> [!success] **Execution update (2026-03-17):** Phase 8 is fully complete (`#194`–`#215`, 22/22 issues closed). Phase 9a has started in GitHub: milestone **Phase 9a-1: Heap Types & Runtime** closed all 18 tracked issues (`#227`–`#244`, including integration kill criterion `#243`), and the next active item is **#240 Heap memory management: Drop insertion at block scope exits** in milestone **Phase 9a-2: Ownership & Lifetimes**.


> [!success] **Execution update (2026-03-17 — Phase 9a complete):** Milestone **Phase 9a-2: Ownership & Lifetimes** is fully closed — all 19 issues (`#240`, `#250`–`#267`) merged in PR [#268](https://github.com/hgahub/duumbi/pull/268). Delivered: `Op::Alloc/Move/Borrow/Drop`, `DuumbiType::Ref(&T)/RefMut(&mut T)`, E020–E029 ownership validator, automatic LIFO drop insertion, 890 tests (62 new), 6 JSON-LD fixtures. **Phase 9a is now 100% complete (37/37 issues across both sub-milestones).** Kill criterion satisfied: validator rejects use-after-move (E021), double-borrow-mut (E022), and dangling reference (E026) at graph validation time. Next active phase: **Phase 9 — Build Excellence & Multi-LLM.**


> [!note] **Phase 15 update (2026-03-15):** Phase 15a (LLVM Backend) expanded to include **WASM compilation target** (`duumbi build --target wasm32`). WASM enables: browser-based DUUMBI playground (onboarding), Figma plugin generation (with MCP client from Phase 12), Cloudflare Workers / edge computing deployment. Requires WASI-compatible runtime shim (replaces libc-based C shim). Phase 12 also gained **MCP Client capability** — agents can call external MCP servers (Figma, GitHub, Browser, Database) alongside internal DUUMBI tools.
>
> Updated Phase 15 table:
> | # | Direction | Description |
> |---|-----------|-------------|
> | 15a | **LLVM Backend + WASM Target** | Inkwell → LLVM IR → x86/aarch64/WASM. WASI runtime shim. Browser playground. Figma plugin target. |
> | 15b | **Projectional Editing** | Graph ↔ code view (bidirectional editing, not read-only `--lang`) |
> | 15c | **Diagram-Driven Development** | Draw C4 diagrams → automatic graph generation |
> | 15d | **DUUMBI Playground** | Browser-based WASM compiler + visualizer (depends on 15a WASM) |
> | 15e | **IDE Plugin** | VS Code extension: schema autocomplete, live graph preview |
> | 15f | **Code Import** | Python/JS/Rust → JSON-LD graph conversion |
> | 15g | **General Enum Type** | User-defined tagged unions, N-variant pattern matching |
> | 15h | **Concurrency** | Thread spawn, channels, Mutex/Arc in generated programs |


> [!important] **Formal Verification addition (2026-03-15):**
> - **Phase 9a:** Added optional `duumbi:precondition`, `duumbi:postcondition`, `duumbi:loopInvariant`, `duumbi:loopVariant`, `duumbi:verificationStatus` fields to the JSON-LD schema. Zero implementation cost — stored but not verified yet. LLMs can generate them, developers can write them.
> - **Phase 15i (NEW): Formal Verification** — VCGen from JSON-LD graph → Z3 SMT solver → proof certificate or counterexample. `duumbi verify` command. Verification Agent (Phase 12 extension). The thesis: formal verification on semantic graphs eliminates the parsing/AST/SSA pipeline that makes traditional formal verification impractical. Three stdlib functions (abs, max, sort invariant) as kill criterion.
> - **Scientific significance:** DUUMBI would be the **only tool that AI-generates code AND formally verifies it.** Dafny/SPARK/Lean require manual code + manual specifications. DUUMBI generates both. Four publishable research contributions identified.
>
> Updated Phase 15 table:
> | # | Direction | Description |
> |---|-----------|-------------|
> | 15a | **LLVM Backend + WASM Target** | Inkwell → LLVM IR → x86/aarch64/WASM. WASI runtime shim. |
> | 15b | **Projectional Editing** | Graph ↔ code (bidirectional editing) |
> | 15c | **Diagram-Driven Development** | Draw C4 diagrams → automatic graph generation |
> | 15d | **DUUMBI Playground** | Browser WASM compiler (depends on 15a) |
> | 15e | **IDE Plugin** | VS Code: schema autocomplete, live graph preview |
> | 15f | **Code Import** | Python/JS/Rust → JSON-LD conversion |
> | 15g | **General Enum Type** | User-defined tagged unions, N-variant matching |
> | 15h | **Concurrency** | Thread spawn, channels, Mutex/Arc |
> | 15i | **Formal Verification** | VCGen → Z3 → proof certificates. AI-generated specs. Verification Agent. |
