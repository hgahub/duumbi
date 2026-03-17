# DUUMBI Obsidian documentation sync

Use this prompt when completed GitHub project, milestone, or issue work must be reflected in the Obsidian knowledge base.

## Primary targets

Start with the roadmap and the phase notes that are most likely to drift from GitHub execution status:

- `docs/Obsidian/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Roadmap Map.md`
- `docs/Obsidian/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Phase 8 - Registry Auth & User Management.md`
- `docs/Obsidian/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Phase 9a - Type System Completion.md`

Expand to other linked notes only when the GitHub project state shows they are also affected.

## Required workflow

1. Review the repository plus the relevant GitHub project, milestone, and issue status before editing anything.
2. Work on a dedicated branch and PR. Never write directly to `main`.
3. Keep edits surgical: update only the notes that are out of sync with actual GitHub delivery.
4. Preserve existing Obsidian conventions:
   - frontmatter keys such as `status`, `github_milestone`, `github_issues`, and `updated`
   - wikilinks such as `[[DUUMBI Roadmap Map]]`
   - English wording for durable project documentation
5. Refresh roadmap and phase-note status snapshots so they clearly separate completed GitHub delivery from still-open follow-up work.
6. If the completed work changes milestone sequencing or roadmap emphasis, update the roadmap map summary as well.
7. Validate the touched files for consistency before finishing.

## Expected output

When the sync is complete, provide:

- the branch/PR used for the documentation update
- the exact Obsidian notes that were updated
- the GitHub milestone/issues that were synchronized
- any remaining open blockers or follow-up notes

## Notifications

Prepare a concise Discord-ready status update instead of an email summary. The message should include:

- branch or PR link
- touched documentation files
- completed GitHub scope now reflected in Obsidian
- remaining open item(s), if any
