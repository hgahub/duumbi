# DUUMBI-593: Add Ready-To-Run Codex Prompts After Approval Gates - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-593/PRODUCT.md` by adding
ready-to-run Codex prompt output to the deterministic Stage Approval workflow
after successful human approvals.

Verified product-spec outcomes this technical spec implements:

- Stage 5 approval produces a Stage 6 Product Spec Draft prompt.
- Stage 7 approval produces a Stage 8 Technical Spec Draft prompt with the
  product spec artifact when available.
- Stage 9 approval produces a Stage 10 Implementation Coordination prompt with
  the product and technical spec artifacts when available.
- Generated prompts are visible from GitHub even when Slack is unavailable.
- Slack approval-result messages include the generated prompt when Slack output
  succeeds.
- Missing artifact URLs are explicit placeholders, not silently omitted or
  guessed.
- Stage 6 and Stage 8 prompt text preserves spec-only safety language.
- Non-approval decisions do not produce next-stage launch prompts.
- Existing approval routing, labels, Project updates, Slack button dispatch, and
  human gates remain unchanged.

This technical spec does not authorize implementation during Stage 8. Stage 10
implementation agents must request or operate under an approved Ralph-cycle
policy before changing workflow code, tests, docs, generated artifacts, runtime
assets, product specs, or implementation files.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles in the `duumbi`
  source repository.
- Stage 9 technical spec reviewers checking whether the proposed workflow change
  is bounded, testable, and aligned with the approved product spec.
- Stage 10 reviewers and testers verifying GitHub Actions, Slack summary, issue
  comments, and workflow-summary evidence.
- Oz agents only for optional cloud-side workflow smoke evidence if a human
  explicitly approves that execution path.

## Source Context

Verified facts:

- Product spec: `specs/DUUMBI-593/PRODUCT.md`.
- Product spec PR: https://github.com/hgahub/duumbi/pull/599.
- GitHub issue: https://github.com/hgahub/duumbi/issues/593.
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/593#issuecomment-4518823242.
- Stage 6 product spec draft comment:
  https://github.com/hgahub/duumbi/issues/593#issuecomment-4522101552.
- Stage 7 approval decision:
  https://github.com/hgahub/duumbi/issues/593#issuecomment-4522185209.
- Workflow gate verified on 2026-05-22: #593 is open, Project status is
  `Technical Spec Needed`, and labels include `product-spec-approved` and
  `needs-tech-spec`.
- Repo instructions: `AGENTS.md`.
- Active runbook:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Agentic Development Runbook.md`.
- Source note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Processed Inbox/DUUMBI Pipeline Automation Spec.md`.
- Active workflow map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`.

Relevant code and workflow files verified for Stage 8:

- `.github/workflows/stage-approval.yml`
- `.github/workflows/human-acceptance-request.yml`
- `.github/workflows/spec-review-request.yml`
- `.github/workflows/technical-spec-review-request.yml`
- `scripts/slack-approval-bridge/src/functions/slackApproval.js`
- `scripts/slack-approval-bridge/README.md`
- `scripts/slack-approval-bridge/package.json`

Relevant current behavior:

- `stage-approval.yml` handles Stage 5, Stage 7, and Stage 9 decisions through
  `workflow_dispatch` and `repository_dispatch`.
- The workflow already posts a structured decision comment to the execution
  issue.
- The workflow already mutates labels based on stage and decision.
- The workflow already attempts Project V2 status updates through
  `GH_PROJECT_PAT`.
- The workflow already posts a Slack approval-result summary through
  `INPUT_SLACK_RESPONSE_URL` and `SLACK_REVIEW_CHANNEL_ID` when configured.
- The workflow already writes a GitHub Actions summary table.
- The workflow already finds Stage 6 and Stage 8 artifact URLs from existing
  issue comments for Stage 7 and Stage 9.
- `spec-review-request.yml` and `technical-spec-review-request.yml` discover
  Stage 6 and Stage 8 artifact comments through marker text and GitHub URLs.
- `slackApproval.js` only translates Slack button clicks into
  `repository_dispatch` payloads for `stage-approval.yml`; it does not need to
  know the generated next-stage prompt text for this issue.
- `scripts/slack-approval-bridge/package.json` has no meaningful test harness;
  its `test` script prints "No tests configured" and exits successfully.

Assumptions and recommendations:

- Keep prompt generation inside `.github/workflows/stage-approval.yml` for this
  issue. Extract a reusable JavaScript file only if inline workflow-script logic
  becomes materially hard to review.
- Prefer one pure helper function for prompt construction and one small helper
  for appending output to Slack, issue comments, and the workflow summary.
- Use a fenced prompt block in Slack and GitHub output so placeholders and issue
  references are not interpreted as Slack links or GitHub task syntax.
- Use verified GitHub URLs extracted from comments or explicit placeholders.
  Do not infer artifact URLs from branch names or local paths.
- No live DUUMBI provider or external LLM path is involved in this work.

## Affected Areas

Expected source changes during Stage 10:

- `.github/workflows/stage-approval.yml`
  - Add prompt metadata for Stage 5, Stage 7, and Stage 9 approval paths.
  - Add prompt construction logic.
  - Include generated prompts in the decision comment.
  - Include generated prompts in Slack approval-result summaries.
  - Include generated prompts in the workflow summary.
  - Preserve existing label, Project, PR-comment, and Slack response behavior.

Optional source changes only if Stage 10 proves they are needed:

- `scripts/slack-approval-bridge/README.md`
  - Clarify fallback behavior only if prompt output materially changes the
    operator flow.
- `scripts/slack-approval-bridge/src/functions/slackApproval.js`
  - No expected change. Touch only if implementation discovers that the current
    payload contract prevents Stage Approval from emitting prompts correctly.

Expected tests and validation artifacts:

- Static workflow review of `.github/workflows/stage-approval.yml`.
- Local prompt-builder simulation evidence if the helper logic is extracted or
  copied into a temporary validation script.
- `git diff --check`.
- `actionlint` when available locally; otherwise record that it was unavailable.
- Optional controlled `workflow_dispatch` evidence on a safe test issue only
  after explicit human approval, because the workflow mutates labels, comments,
  and Project state.

Areas expected not to change:

- Product spec files, including `specs/DUUMBI-593/PRODUCT.md`.
- Technical spec approval labels or Stage 9 decision records.
- Rust source code, Rust tests, runtime assets, generated outputs, graph files,
  provider configuration, registry code, compiler code, parser code, Studio code,
  and DUUMBI runtime behavior.
- Slack bridge deployment infrastructure.
- New GitHub labels or Project fields.

## Technical Approach

### 1. Keep Stage Approval As The Prompt Source

Add prompt generation to the existing `github-script` body in
`.github/workflows/stage-approval.yml` after issue comments have been fetched and
artifact URLs have been resolved.

Recommended helper shape inside the script:

```javascript
const missingProductSpec = "<missing: product spec artifact>";
const missingTechnicalSpec = "<missing: technical spec artifact>";

const buildNextCodexPrompt = ({
  stage,
  decision,
  issueNumber,
  issueUrl,
  productSpecArtifact,
  techSpecArtifact,
}) => {
  if (decision !== "approve") return null;
  // Return an object with title, promptText, and summaryLabel.
};
```

The helper must return `null` for all non-approval decisions and unsupported
stage values. It must not change labels, Project status, PR comment behavior, or
Slack dispatch behavior.

### 2. Define Stage-Specific Prompt Templates

Stage 5 approval prompt:

- Stage: `Stage 6 Product Spec Draft`.
- Skill: `duumbi-spec-draft`.
- Target issue: the verified issue URL.
- Expected artifact: `specs/DUUMBI-<issue-number>/PRODUCT.md`, unless the skill
  determines that an issue-comment spec is sufficient.
- Goal: verify Stage 5 acceptance, inspect context, draft the English product
  spec with BDD scenarios, open a draft spec PR if file-based, link it from the
  issue, and move the issue to Spec Review.
- Boundary: no technical spec, implementation code, or Ralph cycles.
- Spec-only safety: use non-closing references such as `Related to #<issue>` or
  `Spec for #<issue>` and include a workflow note that the spec PR must leave the
  execution issue open.

Stage 7 approval prompt:

- Stage: `Stage 8 Technical Spec Draft`.
- Skill: `duumbi-tech-spec-draft`.
- Target issue: the verified issue URL.
- Product spec artifact: verified Stage 6 artifact URL or
  `<missing: product spec artifact>`.
- Expected artifact: `specs/DUUMBI-<issue-number>/TECHNICAL.md`.
- Goal: verify product spec approval, inspect repo instructions and affected
  source areas, draft the agent-facing technical spec with BDD-to-test mapping,
  live E2E plan, and Ralph Cycle resource policy, open a draft PR, link it from
  the issue, and move the issue to Technical Spec Review.
- Boundary: no implementation code, tests, generated artifacts, runtime assets,
  product spec changes, technical spec approval, or Ralph cycles.
- Spec-only safety: use non-closing references such as `Related to #<issue>` or
  `Technical spec for #<issue>` and include a workflow note that the technical
  spec PR must leave the execution issue open.

Stage 9 approval prompt:

- Stage: `Stage 10 Implementation Coordination`.
- Skill: `duumbi-implementation`.
- Target issue: the verified issue URL.
- Product spec artifact: verified Stage 6 artifact URL or
  `<missing: product spec artifact>`.
- Technical spec artifact: verified Stage 8 artifact URL or
  `<missing: technical spec artifact>`.
- Goal: verify Ready for Build context, manage branch and PR readiness,
  consolidate Ralph-cycle evidence when relevant, choose the next routed action,
  and delegate implementation edits only through bounded Ralph cycles.
- Boundary: do not exceed approved specs, skip resource gates, or perform Stage
  12 closure.

### 3. Preserve Existing Artifact Lookup

Keep the existing comment-based lookup as the source of artifact truth:

- Stage 7 uses the latest `Stage 6 Product Spec Draft` issue comment.
- Stage 9 uses the latest `Stage 8 Technical Spec Draft` issue comment and falls
  back to the Stage 6 comment for product spec context when needed.

Tighten behavior only where needed:

- Normalize empty artifact values to explicit placeholders before prompt
  construction.
- Preserve the current PR-number inference from artifact URLs.
- Do not infer artifact URLs from local paths, branch names, or PR titles.
- Do not fail an already-approved decision solely because a spec artifact comment
  is missing; expose the placeholder in all generated prompt surfaces.

### 4. Add Prompt To Durable GitHub Output

Append the generated prompt to the Stage Approval decision comment when
`buildNextCodexPrompt` returns a prompt.

Suggested decision-comment section:

````markdown
## Next Codex Prompt

```text
<prompt>
```
````

The decision comment is the preferred durable fallback because it already records
the approval event and is linked from PR comments for Stage 7 and Stage 9.

If the final implementation chooses a separate comment to keep decision comments
compact, the decision comment must link that separate prompt comment.

### 5. Add Prompt To Slack Result Output

Append the prompt to the existing `summary` text used for both Slack
`response_url` replies and review-channel posts.

Recommended shape:

````text
- Next: Stage 8 Technical Specification Preparation

Next Codex prompt:
```text
<prompt>
```
````

Do not add new Slack block interactivity for this issue. A plain copy-ready
prompt is enough and keeps the Slack approval bridge unchanged.

Guardrails:

- Keep Slack output best-effort; missing Slack secrets must not fail prompt
  generation.
- Use only verified URLs and explicit placeholders.
- Avoid including unsanitized user-provided multiline content inside the fenced
  prompt.
- If Slack message length becomes a practical problem during Stage 10 testing,
  keep the full prompt in GitHub and post a Slack pointer to the decision
  comment. Record that choice in implementation evidence.

### 6. Add Prompt To Workflow Summary

Add the prompt to `core.summary` after the existing decision table.

Recommended approach:

```javascript
if (nextPrompt) {
  core.summary
    .addHeading("Next Codex Prompt", 3)
    .addCodeBlock(nextPrompt.promptText, "text");
}
```

The summary is required because it gives a GitHub Actions fallback even when
Slack output is unavailable or the decision comment is missed.

### 7. Keep Technical Spec Review Discovery Compatible

The Stage 8 agent that creates a technical spec comment must still put the
technical spec PR URL first and the product spec artifact URL second. The current
`technical-spec-review-request.yml` parser treats the first URL in a Stage 8
comment as the technical spec artifact and the second URL as the product spec
artifact.

For this issue's Stage 10 implementation, do not change that parser unless the
Stage Approval prompt work directly breaks it. The expected implementation is
independent of review-request notification discovery.

## Invariants

- Stage Approval remains deterministic and does not call LLM providers.
- Generated prompts are launch assistance, not workflow state.
- The existing decision matrix remains authoritative for labels, Project status,
  and next state.
- Non-approval decisions do not generate next-stage prompts.
- Stage 5 approval still routes to `Spec Needed`.
- Stage 7 approval still routes to `Technical Spec Needed`.
- Stage 9 approval still routes to `Ready for Build`.
- Slack output remains best-effort and must not become the only prompt surface.
- GitHub issue comments and workflow summary must retain prompt access.
- Missing artifact URLs remain visible placeholders.
- Stage 6 and Stage 8 spec-only prompts must use non-closing issue references.
- The execution issue remains open until Stage 12 closure.
- Stage 10 implementation must not change product specs, technical specs, runtime
  assets, generated outputs, or unrelated source areas.

## BDD-To-Test Mapping

| Product BDD scenario | Evidence type | Required Stage 10 evidence |
|---|---|---|
| Stage 5 approval produces a Stage 6 product spec prompt | Workflow-script validation plus review evidence | Show that `buildNextCodexPrompt` with `stage="5"` and `decision="approve"` returns a prompt containing `duumbi-spec-draft`, the issue URL, Stage 6 goal text, BDD expectation, no technical-spec or implementation authorization, and spec-only safety language. |
| Stage 7 approval produces a Stage 8 technical spec prompt | Workflow-script validation plus artifact lookup review | Show that Stage 7 approval with a verified product spec URL returns a prompt containing `duumbi-tech-spec-draft`, the issue URL, the product spec artifact URL, Stage 8 goal text, no implementation/Ralph authorization, and spec-only safety language. |
| Stage 9 approval produces a Stage 10 implementation coordination prompt | Workflow-script validation plus artifact lookup review | Show that Stage 9 approval with product and technical spec URLs returns a prompt containing `duumbi-implementation`, the issue URL, both spec URLs, and Ralph Cycle resource-gate boundary text. |
| Stage 7 request changes does not produce a Stage 8 launch prompt | Workflow-script validation | Show that `decision="request-changes"` returns `null` or omits the Next Codex Prompt section while existing Stage 7 request-change routing remains unchanged. |
| Stage 5 needs clarification does not produce a Stage 6 launch prompt | Workflow-script validation | Show that `decision="needs-clarification"` returns `null` or omits the Next Codex Prompt section while existing clarification routing remains unchanged. |
| Product spec artifact is missing for Stage 7 approval | Workflow-script validation | Show that Stage 7 approval without a Stage 6 artifact uses `<missing: product spec artifact>` and does not invent a URL. |
| Technical spec artifact is missing for Stage 9 approval | Workflow-script validation | Show that Stage 9 approval without a Stage 8 artifact uses `<missing: technical spec artifact>` and does not invent a URL. |
| Slack is unavailable | Static workflow review plus optional dry-run evidence | Show that prompt construction and GitHub decision-comment/summary output happen outside Slack-only branches, and Slack missing-secret paths remain best-effort. |
| Workflow summary contains the prompt | Static workflow review or safe workflow run | Show that `core.summary` adds the Next Codex Prompt for approval decisions and includes placeholders when artifacts are missing. |
| Generated Stage 6 prompt avoids issue-closing language | Static content scan | Scan the generated Stage 6 template and PR text for common GitHub issue-closing keyword patterns before merge. |
| Generated Stage 8 prompt avoids issue-closing language | Static content scan | Scan the generated Stage 8 template and PR text for common GitHub issue-closing keyword patterns before merge. |

Recommended local validation command examples:

```sh
git diff --check
actionlint .github/workflows/stage-approval.yml
```

If `actionlint` is unavailable locally, record that explicitly and compensate
with careful static review plus GitHub Actions CI evidence.

Optional live workflow evidence:

- Use `workflow_dispatch` only on a safe test issue approved for mutation.
- Do not use #593 as a live test target after it has advanced beyond the stage
  being simulated.
- Capture the workflow run URL, decision comment URL, and final issue labels.
- Confirm any test issue is restored or clearly marked as test-only afterward.

## Live E2E Plan

Canonical interface: GitHub Actions `Stage Approval` workflow.

This issue does not touch DUUMBI LLM behavior. No live LLM-backed E2E path is
required or appropriate. Expected external LLM calls: 0. Estimated external LLM
cost: USD 0.

Required credentials or environment:

- GitHub token with permission to run workflow dispatches and read/write issues
  for any live GitHub Actions smoke test.
- `GH_PROJECT_PAT` only if the smoke test needs to verify Project V2 status
  updates.
- `SLACK_BOT_TOKEN`, `SLACK_REVIEW_CHANNEL_ID`, and Slack response URL only for
  optional Slack delivery evidence. The implementation must still pass GitHub
  fallback evidence without Slack.

Preferred no-mutation evidence:

- Static review of the final workflow diff.
- Local prompt-builder simulation if helper logic can be evaluated outside the
  workflow.
- GitHub Actions CI syntax/check evidence on the implementation PR.

Optional mutation-backed evidence after human approval:

```sh
gh workflow run stage-approval.yml \
  --repo hgahub/duumbi \
  -f stage=5 \
  -f issue_number=<safe-test-issue> \
  -f decision=approve \
  -f rationale="Safe prompt-generation smoke test"
```

Pass criteria:

- The approval decision still records normally.
- The decision comment includes `## Next Codex Prompt`.
- The workflow summary includes the same prompt.
- Slack output includes the prompt when Slack is configured, or the workflow logs
  show Slack was skipped or failed without losing GitHub prompt access.
- Labels and Project status match the existing decision matrix.
- No implementation PR, technical approval, Ralph cycle, or Stage 12 closure is
  started by the prompt.

Fail criteria:

- Any approval path omits the GitHub fallback prompt.
- Non-approval paths produce launch prompts.
- Missing artifacts are silently omitted or replaced with guessed URLs.
- Stage 6 or Stage 8 prompt text uses issue-closing reference syntax.
- Existing label, Project, PR-comment, or Slack bridge behavior regresses.

Artifacts:

- Implementation PR diff.
- Workflow run URL if live workflow evidence is used.
- Decision comment URL from the safe test issue if live workflow evidence is
  used.
- Screenshots are not required.

## Ralph Cycle Protocol

Each Stage 10 cycle must:

1. Summarize the current branch, base branch, issue state, and remaining unmet
   requirements.
2. Propose one bounded implementation goal.
3. List intended file areas and commands before editing.
4. Estimate external LLM usage, command cost, and risk.
5. Check whether the resource gate requires human approval.
6. Implement only the approved or resource-permitted goal.
7. Run the agreed checks.
8. Report evidence, failures, and remaining gaps.
9. Stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached.

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max files or modules per cycle: one primary workflow file plus at most one
  supporting documentation or bridge file if Stage 10 proves it is necessary.
- Expected command budget per cycle:
  - `git diff --check`
  - `actionlint .github/workflows/stage-approval.yml` when available
  - focused local prompt simulation when practical
  - no `cargo test` required unless Rust files are unexpectedly touched, which
    should normally be treated as scope expansion
- Expected external LLM calls: 0.
- Estimated external LLM cost: USD 0.
- Human approval required when planned external LLM usage exceeds USD 2, exceeds
  10 calls, exceeds approved scope, adds risky dependencies or irreversible
  operations, mutates live GitHub test issues, or needs a product/architecture
  decision.
- External LLM usage counted: DUUMBI live provider calls and external model or
  agent CLI calls. Codex internal reasoning usage is reported only as an
  estimate.
- Autonomous batch cap: two low-budget cycles, lower than the default because
  workflow mutation can affect live approval routing.
- When to stop and ask for human guidance:
  - the implementation cannot keep prompt generation inside
    `stage-approval.yml` without a larger script/module extraction
  - Slack message length limits require changing from full Slack prompt output
    to link-only Slack output
  - a live workflow smoke test is needed on a real issue
  - existing Stage Approval behavior must be changed beyond prompt output
  - any product spec open question becomes a blocking design choice

## Task Breakdown

1. Confirm #593 remains in a Stage 10-ready state after Stage 9 approval before
   implementation starts.
2. Inspect the latest `stage-approval.yml` and ensure no concurrent workflow
   changes conflict with this spec.
3. Add a pure prompt-construction helper inside the Stage Approval GitHub Script.
4. Add stage-specific prompt metadata for Stage 5, Stage 7, and Stage 9 approval
   paths.
5. Normalize missing product and technical spec artifacts to explicit
   placeholders.
6. Append the generated prompt to the decision comment for approval decisions.
7. Append the generated prompt to Slack approval-result summaries.
8. Append the generated prompt to the GitHub Actions workflow summary.
9. Verify non-approval decisions do not include prompts.
10. Verify existing label, Project, PR-comment, artifact lookup, and Slack
    bridge behavior are unchanged.
11. Run static checks and prompt-content scans.
12. Record evidence in the implementation PR.

Independently executable slices:

- Prompt helper and template construction.
- Decision-comment and workflow-summary output.
- Slack output wiring.
- Validation and evidence collection.

## Verification Plan

Required before Stage 10 PR review:

- Static diff review proves only approved files changed.
- `git diff --check` passes.
- `actionlint .github/workflows/stage-approval.yml` passes when available, or
  unavailability is recorded with fallback review evidence.
- Prompt templates are scanned for common GitHub issue-closing keyword patterns.
- Evidence demonstrates:
  - approval decisions generate prompts
  - non-approval decisions do not generate prompts
  - missing artifacts render explicit placeholders
  - GitHub decision comment contains or links the prompt
  - workflow summary contains the prompt
  - Slack output includes the prompt or points to durable GitHub prompt output if
    message length forces that narrower behavior

Suggested prompt-content scan:

```sh
tmp_patterns="$(mktemp)"
printf '%s\n' '<add issue-closing regex patterns here during Stage 10>' > "$tmp_patterns"
rg -n -f "$tmp_patterns" .github/workflows/stage-approval.yml
rm "$tmp_patterns"
```

The Stage 10 agent should replace the placeholder scan above with a concrete
safe pattern that checks the final template text without writing the forbidden
issue-closing phrases into this technical spec or the implementation PR body.

Optional live checks:

- A safe `workflow_dispatch` run on a test issue for Stage 5 approval.
- Stage 7 and Stage 9 prompt behavior may be validated by local simulation unless
  a safe test issue with matching Stage 6 and Stage 8 comments is available.

## Completion Criteria

The implementation is ready for PR review when:

- `stage-approval.yml` generates next-stage prompts for Stage 5, Stage 7, and
  Stage 9 approval decisions.
- The prompt is available from a durable GitHub surface.
- Slack approval-result output includes the prompt or a deliberate pointer to
  the durable GitHub prompt if full Slack output proves too large.
- Non-approval decisions produce no next-stage launch prompt.
- Missing product and technical spec artifacts are explicit placeholders.
- Stage 6 and Stage 8 prompts preserve spec-only safety language and avoid
  issue-closing reference syntax.
- Existing Stage Approval label and Project transitions are unchanged.
- Existing Slack approval bridge payloads remain compatible.
- Required checks and review evidence are attached to the implementation PR.
- No product specs, technical specs, implementation code outside the approved
  workflow/supporting-file scope, tests outside approved validation scope,
  generated artifacts, runtime assets, or Ralph-cycle outputs are modified during
  this Stage 8 PR.

## Failure And Escalation

If workflow syntax checks fail:

- Fix the workflow syntax within the same bounded cycle if the fix stays inside
  `.github/workflows/stage-approval.yml`.
- Stop and ask for guidance if fixing syntax requires moving logic into a new
  module or changing workflow architecture.

If Slack output is too long:

- Keep the full prompt in the decision comment and workflow summary.
- Change Slack output to include a concise pointer to the decision comment only
  if necessary.
- Record the trade-off in PR evidence because the product spec prefers Slack to
  carry the prompt when practical.

If artifact lookup is ambiguous:

- Prefer explicit missing-artifact placeholders over guessed URLs.
- Stop if the workflow needs a new durable artifact schema or issue-comment
  format to meet the product spec.

If a live workflow smoke test is needed:

- Ask for explicit human approval before mutating any real issue.
- Use a safe test issue, not #593 after it has advanced beyond the simulated
  stage.

If a product decision is needed:

- Stop and route back through GitHub comments rather than embedding a hidden
  decision in implementation.

## Open Questions

Non-blocking for implementation:

- Whether Slack should always include the full prompt or may fall back to a
  decision-comment pointer if message length becomes a practical problem.
- Whether a later issue should extract prompt templates into a separate reusable
  module once more stage-launch paths exist.
- Whether a future Codex product surface provides a safe prefilled local launch
  URL. This issue should stay copy-ready text only unless that capability is
  already supported and requires no new product design.
