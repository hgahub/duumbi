> **RETIRED — 2026-09-16.** Do not install or run the Grok Spec bot, skill or routines below.
> These instructions are historical recovery documentation only. Disable the three saved
> routines, preserve existing evidence, and use [manual Codex Desktop specification](manual-spec-handoff.md).
> Stage 5 no longer emits worker events. The Owner starts the Desktop task manually.

# Grok VM specification automation (Stage 6–9)

The Stage 5 acceptance workflow emits a `DUUMBI_SPEC_EVENT_V1` line in its existing
Slack summary. A Grok routine calls a deterministic Node worker on the persistent VM.
The worker rechecks the real GitHub acceptance, runs Codex CLI with ChatGPT login,
and opens one reviewed spec-only PR. **The human merges that PR.** A merge notification
then runs `finalize`; execution issues become Ready for Build. Stage 10 never starts.

This is a separate worker from the delivery-autopilot skill. Existing standalone
Stage 7/9 workflows retain their single-file contracts and must not also process these jobs.
The spec-automation label suppresses their Slack review request sweeps.

## One-time setup after the implementation PR is merged

Codex installation and ChatGPT login are already complete on the Grok VM. Do not copy
Codex credentials into GitHub Actions, Slack, repo files, or a skill. The bot needs Node 22+,
git, gh, and GitHub authentication for both repositories and the DUUMBI Project. Configure
GitHub through the bot's secure connections; verify its scopes without displaying tokens.
`gh auth setup-git` configures git to use that existing GitHub authentication.

### Node runtime (interactive and background)

The repository `.nvmrc` selects Node 22, and CI installs that runtime explicitly. The
worker supports Node 22 or newer; it reports the actual version and executable when
preflight fails. An installed Codex CLI does not prove the worker's `node` is recent enough.

On the VM, inspect `node --version` and `command -v node`. If nvm is already installed,
load its `nvm.sh` in the current shell, then run from `/workspace/duumbi`:

```sh
nvm install
nvm use
node --version
command -v node
command -v codex
DUUMBI_PROJECT_NUMBER=4 node scripts/spec-automation/run.mjs check
```

If nvm is absent, install Node 22 using the VM's existing runtime manager or the official
[nvm installation instructions](https://github.com/nvm-sh/nvm#installing-and-updating),
then run the commands above. Do not replace a shared system runtime blindly.

Record the resolved Node **bin directory** in the Spec bot's persistent routine PATH,
along with the existing Codex/gh/git directories. All three routines and their background
children need that PATH. `nvm use` in an interactive terminal alone does not configure a
scheduled shell. Run `check` once through the same noninteractive shell as the routine.
Changing Node can hide npm-global Codex from PATH; preserve its existing installation and
ChatGPT credentials. Do not launch or retry a model job as part of runtime verification.

The VM is shared by the account's bots. Use **one** specification worker/routine and
one state directory. This local lock is not a distributed queue. Do not deploy a second
worker on another host. Keep the durable state directory across bot/VM maintenance.

Run on the Grok VM (use an existing checkout if its location differs):

```sh
cd /workspace/duumbi
git status --short
git pull --ff-only origin main
gh auth setup-git
export DUUMBI_PROJECT_NUMBER=4
export DUUMBI_SPEC_STATE=/workspace/duumbi-spec-state
node scripts/spec-automation/run.mjs check
```

Save these non-secret environment values in the bot's persistent execution configuration.
Project 4 matches the existing DUUMBI_PROJECT_NUMBER repository variable (verified 2026-09-13).
The project number is the numeric part of the DUUMBI Project URL, not an issue number.
Do not set OPENAI_API_KEY or CODEX_API_KEY. `check` verifies ChatGPT auth and CLI flags,
GitHub repository access, the actual pagination command, Project visibility/write capability,
required Status options and (when reported) the classic-token project scope. It makes no model call and no GitHub write. Model availability is
only proven by a real run; do not silently substitute a different model if access fails.

Save [grok-spec-skill.md](grok-spec-skill.md) as a Grok skill. Bind one routine to the
existing Stage 5 acceptance notification channel **and the actual trusted Slack app sender**.
Do not trigger from every mention of “accept”, copied messages or arbitrary issue comments.
Configure a second event condition on that same worker for merged spec PR notifications.
Use `enqueue`: it persists each event before execution and keeps busy events queued.
Configure a lightweight five-minute Grok routine to run `drain`; it starts only pending
jobs, makes no model call when idle, and never retries failed model calls. This also
recovers a queued event arriving just as the previous drainer exits.

Copy-ready routine instructions:

> When the configured DUUMBI Stage 5 Slack app posts a line starting exactly with
> DUUMBI_SPEC_EVENT_V1, parse its JSON. Require version=1, repo=hgahub/duumbi and positive
> integer issue and decision values. Invoke `enqueue ISSUE DECISION run` through the saved DUUMBI specification skill.
> The worker persists and serializes events. Duplicate notifications must use `enqueue`,
> never `resume`. Do not run a model to reinterpret
> the event, change its scope, or choose a different provider. On worker failure notify the
> owner with job ID and error; do not automatically retry a model call. On awaiting_merge,
> show the PR link and wait for the human. For a merge notification of a branch named
> codex/spec-ISSUE-DECISION, invoke `enqueue ISSUE DECISION finalize`; the worker verifies the merge.
> Run `drain` every five minutes to service pending events after an invocation ends.
> Never merge a PR, start Stage 10, force-push, or clear a lock automatically.

A job can outlive a conversational tool call. Launch the worker in the VM's persistent
background-task facility, or use `nohup` with the configured environment and append-only
local logs; do not tie its lifetime to a short Slack routine timeout. For example, after
validating the two numeric event fields:

```sh
mkdir -p /workspace/duumbi-spec-state
nohup node /workspace/duumbi/scripts/spec-automation/run.mjs enqueue 123 456789 run >> /workspace/duumbi-spec-state/worker.log 2>&1 < /dev/null &
```

Use actual IDs. The same background pattern applies to `drain`. A PID/queued receipt is
not completion: the lightweight routine should inspect job/queue status and report only
new PRs, failures, clarification or completion, without another conversational model doing
specification work. Do not forward raw logs containing issue content to public channels.

The exact routine UI and event retention behavior must be verified in the user's Grok
surface. Installing a skill alone does not subscribe it to Slack/GitHub events. If merge
notifications are not available, run `finalize` manually after merging; no recurring Codex
model call is needed for polling. A webhook may carry the same JSON only if its authentication
and durable queue are provided by the bot platform; this implementation adds no public endpoint.

## Pilot

1. Select one small issue and accept it through Stage 5. The decision comment ID is the
   number following `issuecomment-` in the GitHub decision link.
2. Confirm exactly one job, branch and PR appear. Check the model/effort and stages in job.json.
3. Confirm Stage 7 ran before Stage 8 and review sessions are distinct CLI invocations.
4. Review the generated specs and green CI, then merge the spec PR yourself.
5. Verify `finalize` completes and the issue becomes Ready for Build; no implementation PR
   or Ralph cycle should start. A split parent is In Progress/coordination-only; its children
   are Ready for Build with separate spec paths.

```sh
node scripts/spec-automation/run.mjs run 123 456789
node scripts/spec-automation/run.mjs status 123 456789
node scripts/spec-automation/run.mjs finalize 123 456789
```

Numbers are examples. Do not process these literal issue/decision IDs.

## GitHub Project permissions and clarification stops

The worker reads and writes Project V2 status: `read:project` alone is insufficient.
For the gh CLI's stored OAuth login, on the VM run:

```sh
gh auth refresh --hostname github.com --scopes project
node scripts/spec-automation/run.mjs check
```

Complete GitHub's browser/device authorization as the intended account. Do not paste a
token into chat. If `GH_TOKEN`/`GITHUB_TOKEN` supplies the active credential, refreshing a
stored login does not upgrade that environment token: update its permissions through the
existing secure connection instead. Classic credentials need `project`; fine-grained
credentials need the applicable Projects read/write permission and access to Project 4.
Sources: [Project API authorization](https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-api-to-manage-projects)
and [gh auth refresh](https://cli.github.com/manual/gh_auth_refresh).

Preflight verifies permissions before any model call or remote branch claim, without
writing a test Project item. It checks `viewerCanUpdate`, Status options, and classic
OAuth scopes when the REST response reports them. Runtime mutations still fail closed
if credentials or project configuration change after this check.

A job may be `needs_clarification` while its queue is `attention`: the first is the
model's product-question result, while the second can reflect failure to write the
Needs Clarification Project status. Fixing credentials does not answer product questions.
Do not reset a completed clarification call or run `retry-call` to bypass the decision.
For unchanged scope, use the explicit continuation procedure below. Changed scope still
requires renewed Stage 5 acceptance and reconciliation.

## Recovery for job 817/5655972051 (gh 2.46 compatibility)

The worker now uses `gh api --paginate --jq tojson`; it does not require `--slurp`.
The official gh 2.46.0 macOS arm64 release was checksum-verified and tested read-only
against two comment pages and 22 check-run pages. The VM's Linux build still needs the
updated worker's `check` command; no VM CLI upgrade or login reset is required by this fix.

After the fix is merged, on the Grok VM:

```sh
cd /workspace/duumbi
git status --short
git pull --ff-only origin main
export DUUMBI_PROJECT_NUMBER=4
export DUUMBI_SPEC_STATE=/workspace/duumbi-spec-state
node scripts/spec-automation/run.mjs check
node scripts/spec-automation/run.mjs retry-event 817 5655972051
```

Run the last command in the persistent/background facility described above. It explicitly
requeues only the matching `attention` entry. If no job checkpoint exists yet, it starts
`run`; otherwise it resumes saved progress. On success the queue entry becomes `delivered`
and retains the previous error for audit. It never deletes state, steals locks, retries an
uncertain model call, merges a PR, or starts Stage 10. A model call still requires explicit
`retry-call` recovery if its outcome is uncertain. Do not make `retry-event` a recurring rule.

## What the worker owns

| Stage | Work | Gate / stop |
|---|---|---|
| 6 | English PRODUCT and decomposition, BDD acceptance IDs, units and ownership | Human product question → owner |
| 7 | Fresh-context product and decomposition review | Approve, revise, or clarification |
| 8 | Source-backed TECHNICAL per execution unit, tests, E2E and resource plan | Cannot broaden Stage 7 scope |
| 9 | Fresh-context implementability and coverage review | Approve, revise, or clarification |
| Publish | Fixed spec paths, hashes, one PR, review evidence | Await human merge and CI |
| Finalize | Same head, exact contents, no relevant source drift, green CI, resolved reviews | Ready for Build; no Stage 10 |

Sol (`gpt-5.6-sol`, high) is the default. High complexity escalates product review and
technical work to Astra (`gpt-6-astra`, high). Failed reviews get at most two correction
rounds with Astra. Normal success uses four Codex invocations; worst-case review revisions
use twelve. These consume shared subscription quota; they are not unlimited or free capacity.
There is no API fallback. Authentication, model availability, timeout, quota and malformed
output failures stop immediately with the uncertain call checkpoint intact.

The worker reads a fixed source SHA and a planning-vault snapshot. It strips API/secret
environment variables, ignores user provider configuration and project .codex configuration,
and invokes fresh ephemeral read-only sessions. This is not VM-level isolation from other
bots sharing the same account. Only deploy on a trusted VM with the existing account login.
Official headless CLI support does not by itself certify every third-party hosted automation
arrangement; no credentials are moved into public-repository CI by this design.

## Bounded decomposition

Stage 6 proposes, Stage 7 approves, Stage 8 validates technical boundaries, Stage 9 checks
coverage and acyclic dependencies. The controller rejects invalid keys, duplicates, cycles,
missing units and paths outside the fixed spec locations. After Stage 7, child issues are
created and linked through GitHub sub-issues, with an inherited acceptance reference. They
are not implementation-ready before Stage 9, merge and finalization. The parent owns
integration/coordinating the accepted outcome, not duplicate implementation.

The original human acceptance and child title/body hashes must remain unchanged. A changed
scope, closed issue or superseding human decision stops the worker. Unit dependency keys are
in DECOMPOSITION and the per-unit specs; AUTOMATION.json maps keys to actual child IDs.

## Recovery and audit

State: `$DUUMBI_SPEC_STATE/v1-ISSUE-DECISION/job.json`. Each successful model output is saved
before proceeding; schemas, outputs and JSONL usage events are beside it. Keep this directory
private. Do not upload raw logs or auth files to GitHub. AUTOMATION.json in the PR contains
artifact hashes, source/vault SHAs, gate rationale/model and decomposition IDs, not credentials.

- Queued events: queue/*.json records pending/processing/delivered/attention. `drain`
  processes only pending items. After an interrupted drainer, inspect processing records
  and use the explicit job recovery commands; never automatically requeue uncertain work.
- Duplicate event: `enqueue` preserves the original delivery; `run` reports the checkpoint and does not make another model call.
- Queued operational failure: after fixing the cause, use `retry-event ISSUE DECISION`
  (or append `finalize` for a failed merge event). This handles failures before job.json
  exists and updates queue delivery state. It refuses uncertain model calls.
- Direct GitHub/network failure: inspect the error and then `resume ISSUE DECISION`.
  Push recovery reuses the same commit and never force-pushes.
- Interrupted/failed model call: verify it has stopped and inspect quota/logs. With an explicit
  decision to retry, `retry-call ISSUE DECISION CALL_KEY` archives and unlocks only that uncertain call;
  then `resume`. Never automate this command. Successful calls cannot be unlocked.
- Stale lock: read worker.lock/owner.json and queue.lock/owner.json, confirm the recorded process and its Codex children
  have ended, then remove only the lock directory. Never delete it just because time elapsed.
- Incomplete snapshot or dirty publishing checkout: inspect it; preserve job.json and outputs.
  Repair that local checkout before resuming. Do not discard reviewed checkpoints.
- Ambiguous remote branch claim: reconcile the remote branch and local `claimed` checkpoint
  manually; do not steal another worker's claim or start another host.
- Clarification or exhausted reviews: use `handoff ISSUE DECISION` to retrieve the prompt.
  An authorized human posts the exact continuation header and unchanged-scope answer from
  that handoff, then explicitly invokes `continue ISSUE DECISION COMMENT_ID`. The worker
  verifies write permission, current acceptance, attempt and unchanged comment evidence.
  It archives the attempt, retains prior artifacts and restarts independent reviews.
  A changed product scope requires new Stage 5 acceptance and reconciliation of old
  branches/children; never delete checkpoints as a continuation mechanism.
- Finalization failure: repair the reported gate/access problem, then retry `finalize`.
  Successful comments and status writes are idempotent. Edited reviewed artifacts require
  a new review; the finalizer will not bless the changed head/content.

No live Grok execution is proven by local tests. The pilot above verifies event delivery,
actual subscription model access, project permissions, generation quality and end-to-end state.

## Autonomous routing and interactive continuation

Use the three copy-ready routines in [grok-spec-routines.md](grok-spec-routines.md).
Stage 0 is an internal routing assessment, not a replacement for Stage 5 acceptance.
Codex selects autonomous work, public-documentation research, or an owner handoff.
Technical unknowns trigger research; genuine product decisions trigger a concrete question.
Stages 6–9 may request research again. Maximum two research calls, sixteen model calls and
90 minutes of completed model-call time per attempt; each call has a 20-minute timeout.
A budget stop produces a handoff; an uncertain call remains an operational failure and is
never automatically retried. Explicit failed-call retries are archived separately.

Only the research session enables hosted live web search. Other sessions disable it.
The prompt requires public official sources, dated concise extracts, and distinctions
between supported, unsupported and unverified behavior. Source credibility is checked by
independent reviews, not proven by URL syntax validation. Research joins the model context
and the reviewed PR as RESEARCH.md. See the [Codex web search documentation](https://learn.chatgpt.com/docs/web-search).

Handoff is recorded as an idempotent GitHub comment and local handoff.md. Grok sends the
link and copyable prompt to the Owner; GitHub remains the durable source of truth. A failed
GitHub publication can be recovered with resume without another model call. Notification
transport is still the Grok routine: this change does not install a new Slack token/client.

An unchanged-scope continuation uses the same accepted issue/decision and remote branch,
with a new attempt number and input digest. Source/vault snapshots remain immutable;
new answer/evidence enters versioned context. All previous call files remain. Existing
approved product/decomposition is fixed if child allocation may have occurred; changed
product requires reconciliation. Both reviews run again. Editing/deleting a previously
used continuation comment blocks further execution and finalization.

Deploy after human PR merge: wait until the VM worker is idle; verify its checkout is
clean; `git pull --ff-only origin main`; run `node scripts/spec-automation/run.mjs check`.
Update the saved worker skill and the three routines from this repository. Existing
stopped jobs can use `handoff ISSUE DECISION`, then explicit continuation; no checkpoint
reset, automatic retry, new acceptance or model API key is needed for unchanged scope.
