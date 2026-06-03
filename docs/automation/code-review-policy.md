# DUUMBI AI Code Review Policy

This policy defines which AI review services DUUMBI agents should use and when.
It exists to keep review evidence useful without wasting quota or triggering
unbounded review loops.

## Review Service Roles

| Service | Default role | Use when | Do not use when |
|---|---|---|---|
| Codex self-review | Mandatory agent-side review before a PR is marked ready, before a stage gate is approved, and before Stage 11 recommends merge readiness | The agent has task context, can inspect the diff, run checks, map evidence to specs, and classify blocking vs non-blocking findings | As the only evidence for file-based spec approval or implementation merge readiness |
| Copilot review | Default automated PR review gate | Any source-controlled PR, including specs, technical specs, docs, config, and implementation | As proof that the agent ran tests or verified spec coverage |
| CodeRabbit | Optional advisory review when already configured and active on a PR | Implementation PRs where line-level maintainability, test coverage, or broad code-quality comments are useful | As a required DUUMBI gate unless branch protection explicitly requires it |
| Greptile | Manual, quota-limited deep code review | Stable implementation PRs with complex Rust, security-sensitive behavior, cross-module architecture changes, async/concurrency risk, compiler/runtime behavior, provider/auth changes, or broad refactors | Spec-only, docs-only, routine config-only, small low-risk PRs, or every push/update |

`DUUMBI_REQUIRED_SPEC_REVIEWERS` must remain the low-cost required automated
reviewer set for spec-only gates. In this repository the default is
`copilot-pull-request-reviewer`. Do not add Greptile to that variable; Greptile
is a manual escalation path.

## Review-Clean Definition

A PR is review-clean only when all required items for its row are satisfied:

- checks are green, neutral, skipped, or explicitly not applicable
- Codex self-review found no blocking issue, or the blocking issue is fixed
- the latest required automated reviewer submissions have no
  `CHANGES_REQUESTED`
- unresolved review threads are resolved after verifying the fix
- non-blocking findings are either fixed or explicitly accepted as remaining
  risk in the PR, issue, or Stage 11 artifact

Successful reviewer-request workflows do not count as review submissions.

## Stage And PR-Type Matrix

| Stage or PR type | Required review | Optional escalation | Notes |
|---|---|---|---|
| Stage 6 product spec PR | Codex product/spec self-review plus Copilot review evidence | Greptile only if a human explicitly requests product-risk review, which should be rare | Product specs are not code implementation reviews. Prefer Codex checklist and Copilot as default evidence. |
| Stage 7 product spec approval or AI gate | Codex Stage 7 review plus Copilot evidence on file-based specs | Human review when AI gate is blocked | Approval must stay spec-only and non-closing. |
| Stage 8 technical spec PR | Codex technical self-review plus Copilot review evidence | Greptile only for high-risk architecture or implementation-plan review after human request | Technical specs should prepare implementation, not trigger code-review quota by default. |
| Stage 9 technical spec approval or AI gate | Codex Stage 9 implementability review plus Copilot evidence | Human review when AI gate is blocked | Approval must fail closed on unresolved implementation, scope, security, migration, or verification questions. |
| Stage 10 implementation PR readiness | Codex self-review after implementation evidence is consolidated plus Copilot review | CodeRabbit advisory if active; Greptile for high-risk Rust/code criteria below | Stage 10 does not merge. It prepares evidence and routes to Stage 11. |
| Stage 11 implementation review | Codex review artifact, green checks, clean or handled Copilot review, and resolved threads | Greptile only when the PR meets high-risk criteria and the developer asks for it | Human reviewer makes the final PR decision in GitHub. |
| Docs-only PR | Codex self-review plus applicable docs checks; Copilot review may be present | None by default | Do not use Greptile. |
| Config-only or GitHub Actions-only PR | Codex self-review plus syntax/security-permission inspection and applicable checks; Copilot review may be present | Greptile only for security-sensitive workflow/permission changes after human request | Prefer static validation and focused human attention over general AI review churn. |

## Greptile Manual Escalation Criteria

Use Greptile only after the PR is stable enough that a deep review is likely to
be final. A PR qualifies for Greptile consideration when it touches source code
and at least one of these is true:

- Rust changes span more than five meaningful files or more than roughly 400
  non-generated lines
- compiler lowering, graph invariants, parser semantics, runtime C code,
  provider/auth/credentials, registry, MCP, or intent execution behavior changes
- async, concurrency, cancellation, rollback, repair, self-healing, or external
  provider behavior changes
- new dependencies, security-sensitive behavior, migrations, irreversible
  operations, or broad refactors are introduced
- Codex self-review or Copilot finds a non-obvious blocking issue that would
  benefit from a second independent reviewer

When Greptile is used:

1. Run Codex self-review and relevant checks first.
2. Request Greptile once, with a focused comment such as
   `@greptileai review this PR for Rust correctness, graph invariants, and async safety`.
3. Address P0/P1 and genuinely blocking P2 findings.
4. Do not re-run Greptile after every fix. Use Codex self-review to verify fixes
   unless the developer explicitly authorizes one follow-up Greptile run.
5. Record in the PR or Stage 11 artifact whether Greptile was not used, used
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

- `.github/workflows/copilot-review.yml` requests Copilot as the default PR
  reviewer.
- `.github/workflows/spec-review-request.yml`,
  `.github/workflows/technical-spec-review-request.yml`, and
  `.github/workflows/stage-approval.yml` enforce review-clean spec gates.
- `.greptile/config.json` sets Greptile to manual-only with
  `skipReview: "AUTOMATIC"` and `triggerOnUpdates: false`.
