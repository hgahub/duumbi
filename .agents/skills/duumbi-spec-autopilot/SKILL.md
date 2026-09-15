---
name: duumbi-spec-autopilot
description: "Run or resume DUUMBI Stage 6–9 specification automation for an explicitly accepted issue using the Grok VM Codex subscription worker. Prepare independently reviewed product and technical specs and stop at Ready for Build; never start implementation."
---

# DUUMBI Specification Autopilot

Use the deterministic runner in `scripts/spec-automation/run.mjs` and the setup/operations
contract in `docs/automation/grok-spec-setup.md`. The repository is the source of truth;
a Slack message is only a notification. Validate the current Stage 5 acceptance on GitHub.

Inputs are a positive GitHub issue number and the Stage 5 decision comment ID. A valid
`DUUMBI_SPEC_EVENT_V1` carries both. Do not infer acceptance from conversation wording,
an arbitrary comment, an untrusted notification or a label alone.

- `check`: verify tooling and ChatGPT login without a model call or repository write.
- `enqueue ISSUE DECISION run|finalize`: persist and serialize transport events.
- `drain`: service pending events; no model call when idle.
- `run ISSUE DECISION`: direct start once, ignore duplicate delivery.
- `resume ISSUE DECISION`: reuse saved checkpoints after a resolved operational failure.
- `finalize ISSUE DECISION`: after human merge, verify unchanged reviewed artifacts,
  CI and reviews, record gate evidence and move execution issues to Ready for Build.
- `status ISSUE DECISION`: inspect the checkpoint.
- `handoff ISSUE DECISION`: retrieve the interactive prompt and exact answer header.
- `continue ISSUE DECISION COMMENT_ID`: only on explicit owner request, validate a human
  repository writer's unchanged-scope answer and create a new attempt without erasing history.

The worker first routes accepted work to autonomous execution, public-source research or
interactive handoff. Missing technical evidence belongs to bounded research, not an owner
product decision. Follow `docs/automation/grok-spec-routines.md` for notification and explicit
continuation. Changed scope requires renewed Stage 5 acceptance and reconciliation.

The worker performs Stage 6 → fresh Stage 7 → Stage 8 → fresh Stage 9, with at most two
correction rounds per gate. Sol/high is the default; Astra/high handles complex work
and revisions. Subscription quota exhaustion stops; never switch to a paid API provider.
It creates sub-issues only after the product/decomposition gate approves. The parent
then coordinates and children own execution; the parent must not enter Stage 10.

Use this runner as the sole writer for this automation. Do not also invoke the legacy
Stage Approval or Spec AI Gate workflows: those retain their standalone single-file
contract. Do not invoke delivery-autopilot, which has broader Stage 10 authority.

Preserve checkpoints. Do not automatically resend uncertain model calls, remove locks,
force-push branches, manufacture gate evidence, merge the PR, or bypass a clarification.
Report the owner, exact question, job/PR link and next command on any stop. Refer to the
operations document for explicit recovery steps. Do not expose credentials or logs with
secrets. Successful event receipt is not successful downstream completion.
