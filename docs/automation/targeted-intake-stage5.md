# Developer-selected intake → Stage 5

Use **Stage 4 - Select Intake for Stage 5** (`intake-stage5.yml`) to select one enriched
Inbox idea by its exact `intake_id`. It bypasses the scheduled queue's minimum-size rule,
not Human Acceptance. No model call, paid API, spec generation or implementation starts.
The Stage 3b enriched note is preserved in the issue for the human's decision.

## GitHub

Actions → Stage 4 - Select Intake for Stage 5 → Run workflow → enter `intake_id`.

```sh
gh workflow run intake-stage5.yml --repo hgahub/duumbi -f intake_id=YOUR_INTAKE_ID
```

Find the ID in the note's YAML frontmatter, not its filename. The workflow searches the
entire Inbox; selection is not limited to the scheduled sweep's first 200 files/eight notes.
Only exactly one matching `ready_for_triage` note may be queued. Missing IDs, duplicate IDs,
malformed frontmatter or other statuses fail without creating an issue. An already archived
`triaged` note returns its existing issue link and does not reroute downstream work.

The controller reuses an issue with the same intake marker or exact source-note path. It
can route a Todo issue or repair its own incomplete Stage 5 insertion, but cannot roll back
accepted, closed, clarification or other downstream work. Identity deduplication is exact;
this command does not run another semantic deduplication model or reprioritize other notes.

After Project Status = Needs Human Acceptance is set, the `needs-human-review` label starts
the existing Stage 5 notification flow. Only then is the note marked `triaged`, linked to
its issue, archived and pushed to the vault. The queue may already be full: developer
selection still proceeds. The human still explicitly accepts/rejects/clarifies in Stage 5.

## Slack activation after merging the implementation PR

1. Wait for the existing **Slack Bridge Deployment** workflow to publish the updated Azure
   Function. It is triggered by the bridge code merge into main.
2. Open the existing DUUMBI app at [Slack Apps](https://api.slack.com/apps).
3. **Slash Commands → Create New Command**:
   - Command: `/duumbi-triage`
   - Request URL: `https://func-duumbi-slack-bridge.azurewebsites.net/api/slack-approval`
   - Short description: `Select an Inbox idea for Human Acceptance`
   - Usage hint: `<intake_id>`
4. Save. Verify the app has the `commands` scope and reinstall/approve changed permissions
   if Slack requests it. Keep the existing `chat:write` permission for the result message.
5. Invite the app to the channel where developers invoke the command, including private
   channels when needed. Run `/duumbi-triage YOUR_INTAKE_ID` in that channel.

Slack sends commands to their configured Request URL and requires acknowledgement within
three seconds. The bridge verifies the existing Slack signature and makes one bounded
GitHub dispatch request. See [Slack slash command documentation](https://docs.slack.dev/interactivity/implementing-slash-commands/)
and the [commands scope](https://docs.slack.dev/reference/scopes/commands/).

The immediate reply is private and confirms dispatch only. The workflow later posts an
additional private result to the invoking user/channel with the issue or failure and an
Actions run link. The normal Stage 5 approval card follows the existing review-channel flow.
If private-message delivery fails, the authoritative result remains in the Actions summary.
The original `response_url` is not stored in GitHub dispatch payloads.

The command configuration is versioned in
[`slack-intake-command.fragment.json`](../../scripts/slack-approval-bridge/slack-intake-command.fragment.json).
It is a merge fragment, not a replacement app manifest: preserve existing commands, scopes
and interactivity settings when incorporating it into the Slack app.

No new Azure resource or secret is required. Pulumi continues to own the existing bridge
settings (`SLACK_SIGNING_SECRET`, `GITHUB_TOKEN`, `GITHUB_REPO`, `SLACK_BOT_TOKEN`); this change
only updates function code and documents the Slack app's slash-command configuration. The
Action uses the existing GH_PROJECT_PAT, DUUMBI_PROJECT_* and SLACK_BOT_TOKEN settings.

## Retry and pilot

Both this Action and scheduled triage share the `duumbi-vault-intake` concurrency group.
Check Actions for cancelled pending runs if several requests arrive together; GitHub
concurrency is not a durable FIFO queue. Retry a cancelled selection with the same ID.

A created issue carries its intake marker in the initial POST, so a lost create response or
failed project/label operation can be retried using the same issue. The note stays in Inbox
until the queue update succeeds. After a failed vault push, rerun with the same ID from a
fresh checkout: it reuses the existing Stage 5 issue and completes archiving. Scheduled
triage skips note paths already linked to a non-Todo Project issue, avoiding duplicate work
while that targeted recovery is pending. Never delete the linked issue just to retry.

Pilot: select one known ready note, verify one Stage 5 issue/card and the archived note,
then repeat the same ID and verify no second issue/card or downstream status rollback.
Also check a captured note: it must fail without side effects. No live pilot is performed
by the tests; tests use fake GitHub/Slack clients and a temporary vault.
