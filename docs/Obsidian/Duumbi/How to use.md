---
tags:
  - tool/obsidian
  - doc/reference
status: final
created: 2026-02-05
updated: 2026-03-12
---
# How to Use This Vault

This Obsidian vault is the knowledge base for the DUUMBI project. It follows the **LYT (Linking Your Thinking)** methodology, adapted for AI-agent-supported development.

## Structure Overview

### `01 Atlas (Knowledge Base)/`

The heart of the vault. Three tiers:

- **`Dots (Atomic Ideas)/`** — One note, one concept. Each Dot captures a single idea, definition, or architectural decision. Keep these short (under 50 lines). Examples: [[Semantic Fixed Point]], [[JSON-LD Graph Representation]], [[Compilation Pipeline]].

- **`Maps (Overviews)/`** — Navigation hubs that connect Dots into coherent themes. Maps primarily contain links and brief context. Examples: [[DUUMBI Core Concepts Map]], [[DUUMBI Technical Architecture Map]], [[DUUMBI Roadmap Map]].

- **`Works (Developed Materials)/`** — Large, detailed documents built from multiple Dots and Maps. These are the authoritative specifications. Examples: [[DUUMBI - PRD]], [[DUUMBI - MVP Specification]], [[DUUMBI - Post-MVP Implementation Roadmap]].

### `02 Resources (Assets and Tools)/`

- **`Attachments (MediaFiles)/`** — Excalidraw diagrams, images, PDFs. Set as the default attachment folder in Obsidian settings.
- **`Sources (References)/`** — Notes from external sources (articles, papers, tools).
- **`Tag Pages (Tag Notes)/`** — Dedicated notes for major tags with Dataview queries.

### `03 Templates/`

Note templates for consistent structure (idea template, meeting template, etc.).

### `00 Inbox (ToProcess)/`

Quick capture. Regularly process into Dots or discard. Contains historical documents (Original PRD) that have been absorbed into the Atlas.

## For AI Agents

**If you are an AI agent working on DUUMBI, start here:**

1. **Orientation**: Read [[DUUMBI Core Concepts Map]] for the conceptual overview
2. **Terminology**: Read [[DUUMBI - Glossary]] for canonical definitions — use these terms exactly
3. **Current scope**: Read [[DUUMBI - MVP Specification]] for the build specification
4. **Active work**: Check [[DUUMBI - Post-MVP Implementation Roadmap]] for the current milestone
5. **Architecture**: Read [[DUUMBI Technical Architecture Map]] for technical decisions
6. **Roadmap**: Read [[DUUMBI Roadmap Map]] for timeline and business context

**Rules for graph mutations:**
- Every mutation must pass schema validation (`duumbi check`)
- The graph must reach [[Semantic Fixed Point]] before compilation
- Follow the JSON-LD schema at `.duumbi/schema/core.schema.json`
- Use 4-segment node IDs: `duumbi:<module>/<function>/<block>/<index>`

## Workflow

1. **Quick Capture** → `00 Inbox (ToProcess)/`
2. **Process** → create atomic Dots in `01 Atlas/Dots/`, link to Maps
3. **Connect** → use `[[wikilinks]]` extensively, check the graph view
4. **Develop** → when multiple Dots converge, create a Work in `01 Atlas/Works/`
5. **Review** → periodically update Maps, prune stale Dots

## Naming Conventions

- Dots: descriptive concept name (e.g., `Semantic Fixed Point.md`)
- Maps: `DUUMBI <Topic> Map.md` (e.g., `DUUMBI Core Concepts Map.md`)
- Works: `DUUMBI - <Document Name>.md` (e.g., `DUUMBI - PRD.md`)
- Language: **English** for all documentation
