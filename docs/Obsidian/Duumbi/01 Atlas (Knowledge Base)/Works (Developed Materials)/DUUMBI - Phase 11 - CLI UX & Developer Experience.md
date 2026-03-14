---
tags:
  - project/duumbi
  - milestone/phase-11
status: planned
github_milestone: ~
updated: 2026-03-14
---
# Phase 11 — CLI UX & Developer Experience ⏳

> **Kill Criterion:** 3/3 new users successfully execute an intent using the `/` menu within 10 minutes without reading CLI documentation. A user can exit, return, and `/resume` a previous session with full context restored.
> **Status:** ⏳ Planned — starts after Phase 10
> **Estimated duration:** 6–8 weeks (solo developer)

← Back: [[DUUMBI Roadmap Map]]

---

## Summary

Transform the CLI from functional to delightful AND persistent. Two pillars:

1. **Visual UX** — Copilot CLI-level polish: colors, interactive menus, fuzzy-match discovery, contextual suggestions.
2. **Session Commands** — CLI commands that expose the session, task, stats, and permissions infrastructure built in Phase 10. The data layer lives in Phase 10; this phase builds the user-facing commands on top.

---

## Tasks

### M11-SESSION: Session Management Commands

These commands consume the `SessionManager`, `TaskManager`, `StatsCollector`, and `PermissionsManager` APIs built in Phase 10 (M10-SESSION).

#### `/resume` — Restore Previous Session
- [ ] Show: last active intent, pending tasks, conversation summary
- [ ] User confirms or selects which session to resume (if multiple archived)
- [ ] Conversation history reloaded into LLM context
- [ ] Status bar updated with restored session info

#### `/clear` — Reset Context (3 Levels)
- [ ] `/clear chat` — Clear conversation history only (LLM context reset)
- [ ] `/clear session` — Clear session state (back to fresh start, keep project data)
- [ ] `/clear all` — Full reset (session + conversation + cached LLM responses)
- [ ] Confirmation prompt before destructive clears

#### `/sessions` — Session History
- [ ] List archived sessions with timestamp, duration, summary
- [ ] Resume any archived session by selection

#### `/task` — Task Management
- [ ] `/task list` — Show active tasks (from intent pipeline + manual)
- [ ] `/task add "description"` — Create ad-hoc task (via Phase 10 Task API)
- [ ] `/task status [id]` — Show task progress, dependencies, blockers
- [ ] `/task done [id]` — Mark task complete
- [ ] Color-coded status: pending=yellow, active=blue, done=green, blocked=red

#### `/permissions` — Governance Controls
- [ ] `/permissions show` — Current permission state (formatted table)
- [ ] `/permissions set auto-build [on/off]`
- [ ] `/permissions set auto-apply [on/off]`
- [ ] `/permissions set providers [list]`
- [ ] `/permissions set max-tokens [n]`
- [ ] `/permissions set max-retries [n]`
- [ ] Changes persisted to `config.toml` via Phase 10 Permissions API

#### `/stats` — Usage Statistics Dashboard
- [ ] `/stats session` — Current session: duration, commands, AI calls
- [ ] `/stats tokens` — Per-provider token breakdown + cost estimate
- [ ] `/stats build` — Success/fail rate, average compile time
- [ ] `/stats ai` — Per-provider success rate, common error types
- [ ] `/stats history` — Historical trends: ASCII art graphs in terminal
- [ ] All data sourced from Phase 10 Stats API

---

### M11-COLOR: Color & Formatting
- [ ] Colored output: `owo-colors` or `colored` crate — errors red, success green, warnings yellow
- [ ] Progress indicators: spinner during build, progress bar during intent execute
- [ ] Structured output: tabular formatting via `comfy-table` crate
- [ ] Consistent color scheme: DUUMBI color palette (primary, accent, error, success, muted)
- [ ] Diff display: colored inline diffs for AI mutation previews (green=added, red=removed)

### M11-MENU: Interactive Menu System
- [ ] `/` triggers scrollable command dropdown (reedline completion integration)
- [ ] Typing narrows the list (fuzzy match on command names)
- [ ] Command grouping:
  - **Session**: `/resume`, `/clear`, `/sessions`
  - **Build**: `/build`, `/run`, `/check`, `/describe`
  - **Graph**: `/add`, `/undo`, `/viz`, `/deps`
  - **Intent**: `/intent`, `/task`
  - **Registry**: `/publish`, `/search`, `/install`
  - **System**: `/permissions`, `/stats`, `/help`, `/exit`
- [ ] Inline help: one-line description next to each command
- [ ] `Tab` completion for module names, function names, file paths
- [ ] Reference implementations to study: GitHub Copilot CLI, `fig` autocomplete, `atuin`

### M11-PROMPT: Contextual Prompts & Onboarding
- [ ] If workspace is empty → "Try: `/intent create` to start building"
- [ ] If common typo detected → "Did you mean: `/build`?"
- [ ] History-based autocomplete: suggest based on previous commands in session
- [ ] Status bar: current workspace, active model, last build status, session duration
- [ ] Error messages with actionable suggestions: "Build failed → Try `/check` for details"
- [ ] First-run wizard: guided setup (select provider, create first intent) for new users

---

## Complete Slash Command Reference

| Command | Description | Data Source |
|---------|-------------|------------|
| `/build` | Compile project | Phase 1 (existing) |
| `/run` | Execute binary | Phase 1 (existing) |
| `/check` | Validate graph | Phase 1 (existing) |
| `/describe` | Pseudo-code view | Phase 1 (existing) |
| `/undo` | Revert last mutation | Phase 2 (existing) |
| `/viz` | Open web visualizer | Phase 3 (existing) |
| `/deps` | Dependency management | Phase 4 (existing) |
| `/intent` | Intent pipeline | Phase 5 (existing) |
| `/resume` | Restore previous session | **NEW** — Phase 10 SessionManager |
| `/clear` | Reset context (chat/session/all) | **NEW** — Phase 10 SessionManager |
| `/sessions` | List session history | **NEW** — Phase 10 SessionManager |
| `/task` | Task management | **NEW** — Phase 10 TaskManager |
| `/permissions` | Governance controls | **NEW** — Phase 10 PermissionsManager |
| `/stats` | Usage statistics | **NEW** — Phase 10 StatsCollector |
| `/help` | Command reference | Phase 4 (enhanced) |
| `/exit` | Exit CLI | Phase 4 (existing) |
| `/publish` | Publish module | Phase 7 (existing) |
| `/search` | Registry search | Phase 7 (existing) |
| `/install` | Install dependency | Phase 7 (existing) |

---

## Dependencies

```
Phase 10 (Knowledge Graph)  ──→ Phase 11 (session/task/stats/permissions data layer)
Phase 9 (Build Excellence)  ──→ Phase 11 (no point polishing UX for broken functionality)
Phase 4 (REPL)              ──→ Phase 11 (reedline foundation already exists)
```

**Key architectural boundary:** Phase 10 owns the data (APIs, persistence, data models). Phase 11 owns the presentation (CLI commands, colors, menus, formatting). This separation allows the data layer to also serve Phase 12 (agents) and Phase 13 (self-healing) without CLI coupling.

## Expected Files

```
src/cli/
├── repl.rs        — (modified: colors, spinner, progress bar, session hooks)
├── commands.rs    — (modified: structured output, new slash commands)
├── theme.rs       — DUUMBI color palette definitions
├── completion.rs  — fuzzy-match command completion
├── help.rs        — inline help + contextual suggestions
├── session_cmd.rs — /resume, /clear, /sessions command handlers
├── task_cmd.rs    — /task command handler (calls Phase 10 Task API)
├── perm_cmd.rs    — /permissions command handler (calls Phase 10 Permissions API)
└── stats_cmd.rs   — /stats command handler (calls Phase 10 Stats API)
```

---

## Related

- [[DUUMBI - Phase 10 - Intelligent Context & Knowledge Graph]] — data layer (M10-SESSION)
- [[DUUMBI - Phase 9 - Build Excellence & Multi-LLM]] — prerequisite
- [[DUUMBI - Phase 14 - Marketing & Go-to-Market]] — polished CLI is essential for first impressions


---

## Addendum: Windows Compatibility

> Added 2026-03-14.

Phase 1 states: "Windows support is not planned for MVP." Phase 11 is the last UX phase before public launch (Phase 14 HN post). The developer tool market is ~40% Windows. An untested Windows experience after HN launch damages credibility.

### Strategy: WSL2-Tested, Not Native Windows

| Level | Support | Phase |
|---|---|---|
| **macOS (aarch64)** | Primary development target, fully tested | Phase 0+ |
| **Linux (x86_64)** | CI-tested, fully supported | Phase 0+ |
| **Windows via WSL2** | Tested, documented, best-effort support | **Phase 11** |
| **Native Windows (MSVC)** | Not supported, not planned before Phase 15+ | — |

### Phase 11 Tasks (Windows)
- [ ] WSL2 testing: `duumbi init` → `duumbi build` → `duumbi run` verified on Ubuntu 24.04 under WSL2
- [ ] WSL2 quickstart guide in docs.duumbi.dev: "Getting Started on Windows (WSL2)"
- [ ] Known limitations documented:
  - Native Windows (cmd.exe, PowerShell) is NOT supported — WSL2 required
  - Reedline colors/completion: verified working in Windows Terminal + WSL2
  - File watcher (`notify`): verified working on WSL2 filesystem (not Windows-mounted paths)
  - `cc` linker: WSL2 `apt install build-essential` required
- [ ] README badge: "Supported: macOS, Linux, Windows (WSL2)"
- [ ] CI job: GitHub Actions `windows-latest` with WSL2 Ubuntu step (non-blocking, report-only initially)


---

## Addendum: Studio Visual Updates for Phase 9a Types

> Added 2026-03-14.

The Phase 6 Studio renders the graph using the Phase 0–3 type system. After Phase 9a, the Studio must visualize new concepts: ownership edges, borrow relationships, lifetime scopes, Result flow, and String/Array types.

**Principle:** Most changes are automatic — the Studio renders graph nodes generically. But ownership relationships benefit from visual distinction.

### Automatic (No Code Change Needed)
- New node types (Alloc, Move, Borrow, Drop, StringConcat, ArrayNew, etc.) appear as nodes in the C4 Code view — the Studio already renders all `@type` values
- New properties (owner, borrowedFrom, lifetime) appear in the Inspector panel — already shows all properties

### Visual Enhancements (Phase 11 Scope)
- [ ] **Ownership edges:** Render `duumbi:owner` edges as solid bold lines (distinct from data flow edges)
- [ ] **Borrow edges:** Render `duumbi:borrowedFrom` / `duumbi:borrowedMutFrom` as dashed lines
  - Shared borrow: blue dashed
  - Mutable borrow: red dashed
- [ ] **Lifetime scopes:** Highlight the Block boundary that a borrow's `duumbi:lifetime` references
- [ ] **Result flow:** In the C4 Code view, show Ok path in green, Err path in red branching from MatchResult nodes
- [ ] **Drop nodes:** Render with a distinctive icon (⊗ or similar) to make resource cleanup visible
- [ ] **Inspector enhancements:** When selecting an owned value, show all active borrows and the expected drop location


---

## Addendum: `--lang` Projection in CLI and Studio

> Added 2026-03-14.

The `duumbi describe --lang` feature (defined in Phase 9a) needs CLI integration in Phase 11 and Studio integration.

### CLI Integration
- [ ] `--lang` flag on `duumbi describe`: `rust`, `python`, `go` (Phase 9a defines the backends)
- [ ] `/describe rust` slash command shortcut (equivalent to `duumbi describe --lang rust`)
- [ ] Default language configurable in `config.toml`:
  ```toml
  [describe]
  default-lang = "rust"    # "pseudo" | "rust" | "python" | "go"
  ```
- [ ] Studio Code View: when drilling down to C4 Code level, show the projected language instead of raw JSON-LD
- [ ] Studio language toggle: dropdown in the Inspector panel to switch between pseudo/rust/python/go views
- [ ] Copy-to-clipboard: the projected code is copyable from Studio (useful for documentation, code reviews)
