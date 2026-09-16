---
name: duumbi-spec-autopilot
description: "Retired Grok VM specification entry point. Redirect explicit legacy requests to the manually started duumbi-spec-desktop workflow; never enqueue or resume the old worker."
---

# Retired specification worker

The Grok VM Stage 6–9 worker was retired on 2026-09-16. Do not run, enqueue, drain,
continue or finalize VM jobs. Stage 5 now records a prompt on GitHub and sends it to Slack;
the Owner starts specification manually in Codex Desktop with `duumbi-spec-desktop`.

Preserve historical checkpoints and source evidence. Do not delete a branch or restart
an accepted issue to migrate transport. See `docs/automation/manual-spec-handoff.md`.
