# DUUMBI AI Code Review Policy

This policy defines which AI review services DUUMBI agents should use and when.
It exists to keep review evidence useful without wasting quota or triggering
unbounded review loops.

Copilot review was removed from this policy after the service subscription was
cancelled. Layered review now concentrates on the final implementation PR.

## Review Service Roles

| Service | Default role | Use when | Do not use when |
|---|---|---|---|
| Codex self-review | Mandatory agent-side review before a PR is marked ready, before a stage gate is approved, and before Stage 11 recommends merge readiness | The agent has task context, can inspect the diff, run checks, map evidence to specs, and classify blocking vs non-blocking findings | As the only evidence for implementation merge readiness |
| Codex review (`@chatgpt-codex-connector`) | Required automated reviewer on the final implementation PR | The implementation PR is complete and the developer or agent requests review before the human merge decision | Spec-only, docs-only, or intermediate PRs by default |
| Quick low-cost reviewers (MiniMax, DeepSeek Pro, Grok Build, Cursor BugBot) | Optional advisory review on non-final PRs | A fast, cheap second opinion on a spec, docs, config, or in-progress implementation PR is useful | As a required gate or as a substitute for Codex review on the final implementation PR |
| Greptile | Manual, quota-limited deep code review, **final implementation PR only** | The change touches source code, meets the high-risk criteria below, and the developer explicitly requests a deep detailed review at the end of the flow | Spec-only, docs-only, config-only, intermediate, small low-risk PRs, or every push/update |

`DUUMBI_REQUIRED_SPEC_REVIEWERS` is empty by default: spec-only PR gates rely on
Codex self-review plus checks. Do not add Greptile to that variable; Greptile is
a manual escalation path for the final implementation PR.

## Review-Clean Definition

A PR is review-clean only when all required items for its row are satisfied:

- checks are green, neutral, skipped, or explicitly not applicable
- Codex self-review found no blocking issue, or the blocking issue is fixed
- on the final implementation PR, the latest Codex
  (`@chatgpt-codex-connector`) review has no `CHANGES_REQUESTED`
- unresolved review threads are resolved after verifying the fix
- non-blocking findings are either fixed or explicitly accepted as remaining
  risk in the PR, issue, or Stage 11 artifact

Successful reviewer-request workflows do not count as review submissions.

## Stage And PR-Type Matrix

| Stage or PR type | Required review | Optional escalation | Notes |
|---|---|---|---|
| Stage 6+8 combined spec PR | Codex self-review plus checks | Quick low-cost reviewer (advisory) | Product and technical specs are drafted together; no external review wait between them. |
| Stage 7 / Stage 9 spec approval or AI gate | Codex Stage 7/9 review plus checks | Human review when AI gate is blocked | Approval must stay spec-only and non-closing, and fail closed on unresolved scope, security, migration, or verification questions. |
| Stage 10 implementation PR readiness | Codex self-review after implementation evidence is consolidated | Quick low-cost reviewer (advisory) | Stage 10 does not merge. It prepares evidence and routes to Stage 11. |
| Stage 11 implementation review (final PR) | Codex review via `@chatgpt-codex-connector`, green checks, resolved threads | Greptile when the PR meets high-risk criteria; signal the need on Slack and in the issue for the human decision | Human reviewer makes the final PR decision in GitHub. |
| Docs-only PR | Codex self-review plus applicable docs checks | None by default | Do not use Greptile. |
| Config-only or GitHub Actions-only PR | Codex self-review plus syntax/security-permission inspection and applicable checks | Quick low-cost reviewer for security-sensitive workflow/permission changes | Prefer static validation and focused human attention over general AI review churn. |

## Greptile Manual Escalation Criteria

Greptile runs only on the final implementation PR, after Codex review, when the
PR is stable enough that a deep review is likely to be final. A PR qualifies for
Greptile consideration when it touches source code and at least one of these is
true:

- Rust changes span more than five meaningful files or more than roughly 400
  non-generated lines
- compiler lowering, graph invariants, parser semantics, runtime C code,
  provider/auth/credentials, registry, MCP, or intent execution behavior changes
- async, concurrency, cancellation, rollback, repair, self-healing, or external
  provider behavior changes
- new dependencies, security-sensitive behavior, migrations, irreversible
  operations, or broad refactors are introduced
- Codex review finds a non-obvious blocking issue that would benefit from a
  second independent reviewer

When Greptile is used:

1. Run Codex self-review, the `@chatgpt-codex-connector` review, and relevant
   checks first.
2. Signal the Greptile need to the developer on Slack and in the issue; the
   developer triggers it manually.
3. Request Greptile once, with a focused comment such as
   `@greptileai review this PR for Rust correctness, graph invariants, and async safety`.
4. Address P0/P1 and genuinely blocking P2 findings.
5. Do not re-run Greptile after every fix. Use Codex self-review to verify fixes
   unless the developer explicitly authorizes one follow-up Greptile run.
6. Record in the PR or Stage 11 artifact whether Greptile was not used, used
   once, or re-run with explicit authorization.

## Blocking Policy

Blocking findings are correctness bugs, spec violations, security issues,
data loss, crashes, failing checks, unresolved required review decisions, or
missing evidence required by the approved specs.

Non-blocking findings include style, wording, minor maintainability, optional
test suggestions, and reviewer preferences that do not change behavior or risk.
Agents may fix non-blocking findings when cheap, but they must not create an
unbounded review loop to satisfy nits.

## Related Configuration

- `.github/workflows/spec-review-request.yml`,
  `.github/workflows/technical-spec-review-request.yml`, and
  `.github/workflows/stage-approval.yml` enforce review-clean spec gates;
  `DUUMBI_REQUIRED_SPEC_REVIEWERS` is empty by default.
- `.greptile/config.json` sets Greptile to manual-only with
  `skipReview: "AUTOMATIC"` and `triggerOnUpdates: false`.
