# Grok Bot setup — DUUMBI

Save the separate `grok-intake-skill.md` recipe as a private Grok Bot skill.
The recipe is portable; put the following instance settings in the selected
Bot's profile/memory, not in the recipe:

- Vault repository: `hgahub/duumbi-vault`, branch `main`.
- Vault checkout: `/workspace/duumbi-vault` (suggested).
- Tooling repository: `hgahub/duumbi`, updated to the intake-lifecycle release.
- Tooling checkout: `/workspace/duumbi` (suggested).
- Inbox: `Duumbi/00 Inbox (ToProcess)/`.
- Default owner: `hgahub`, unless a particular submitter is known.
- Required tools: Node.js 22+, Git, authenticated repository access.

## User steps

1. Open the intended Bot (or create `DUUMBI Intake`). Connect GitHub with access
   to read the source repository and read/write the private vault. Use the
   secure login/connection interface for authentication, not chat text.
2. Ask the Bot to prepare/update those two checkouts under `/workspace`, verify
   Node/Git availability, and verify vault access without creating an idea note.
3. Attach or paste `grok-intake-skill.md` and say: “Save this as a private skill
   named duumbi-grok-intake. Saving must not run it or create a routine.”
4. Enable the saved skill for the intended Bot under Settings → Plugins → Yours
   if required by your app version. Invoke it from the slash/autocomplete menu;
   use the actual saved pill/link returned by Grok (`sand-workflow:...`) rather
   than inventing a slug.
5. After the DUUMBI PR is merged and vault metadata migrated, submit a real idea
   using the saved skill. Verify the returned GitHub note link and commit; the
   note must contain `source: grok` and `intake_status: captured`.

The Grok cloud computer is separate from the user's Mac. Saving a recipe does
not grant GitHub access or synchronize a local vault. Skills and underlying
computer access are shared account capabilities; enabling a skill per Bot is
not file/credential isolation. No recurring routine is needed for intake.

Sources checked 2026-09-12:
- https://docs.x.ai/grok-bot/skills-routines-and-automations
- https://docs.x.ai/grok-bot/computer-and-apps

The user's Grok Bot also confirms skill creation from name/description/body and
invocation through the saved skill pill. The installed skill's actual link is
returned by Grok during creation.
