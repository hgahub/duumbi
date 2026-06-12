## Summary

Describe what changed and why.

## Change Type

- [ ] Feature
- [ ] Bug fix
- [ ] Refactor
- [ ] Documentation
- [ ] CI/Build

## Validation

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo nextest run --workspace`
- [ ] `cargo audit`

## AI Review Plan

- [ ] Codex self-review completed and blocking findings are fixed or listed
- [ ] Final implementation PR: Codex review requested from `@chatgpt-codex-connector`; otherwise optional quick low-cost review (MiniMax, DeepSeek Pro, Grok Build, Cursor BugBot) or explicitly not applicable
- [ ] Greptile not used, or manually requested on the final implementation PR because it meets the high-risk criteria in `docs/automation/code-review-policy.md` (need signaled on Slack and in the issue)
- [ ] Review threads are resolved after verifying fixes

## Docs Impact

- [ ] No docs update needed
- [ ] Updated docs under `sites/docs/src/`
- [ ] Docs follow-up issue created

## Notes for Reviewers

Add any context that helps with review (trade-offs, risks, follow-ups).
