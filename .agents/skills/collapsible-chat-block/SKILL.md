---
name: collapsible-chat-block
description: Use when adding or changing DUUMBI TUI chat output that should appear as a headered collapsible block, including command logs, model reasoning, build output, diagnostics, or other verbose terminal content.
---

# Collapsible Chat Block

Use this skill when DUUMBI TUI output must stay inside the chat surface while remaining easy to scan.

## Required Behavior

- Render as a normal conversation output block, not in the status dock or activity row.
- Show a header with a disclosure marker:
  - `▸ Header` when collapsed.
  - `▾ Header` when expanded.
- Toggle collapsed/expanded state on mouse click against the header row.
- Allow the caller to choose the body text color through `OutputStyle`.
- Keep separator rows around the block by relying on `conversation_visual_rows` block-boundary spacing.

## Implementation Pattern

- Use `OutputRenderMode::Collapsible { header, expanded, style }` for reusable blocks.
- Add content with `ReplApp::push_collapsible_output(header, text, style, expanded)`.
- Use `OutputStyle::Normal` for user-facing command logs that should render white.
- Use `OutputStyle::Thinking` only for model reasoning/thinking content.
- Do not use `run_with_terminal_restore` for output that should remain inside the TUI; capture or structure the result and push it into a collapsible block.

## Validation

- Add or update a focused render test for the header marker, body visibility, click toggle behavior, and separator row behavior.
- Run `cargo fmt --check`, the relevant `cli::app::tests`, and `cargo clippy --all-targets -- -D warnings`.
