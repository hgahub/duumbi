# 2026-05-22 - GitHub Actions LLM Usage Metrics

## Source
- Source: Codex
- Surface: Codex
- Conversation context: Slack thread in `C08SK7E6R7T` (`p1779433792156479`), user asked to use `duumbi-codex-intake` skill and capture an idea.
- Submitted by: `<@U4SMDRG9M>`

## Raw input
Collect metrics in GitHub Actions about LLM call consumption and run times so later workflow executions can be optimized.

## Interpreted intent
The user proposes adding observability for CI usage of LLM calls (for example token/credit consumption, request counts, and per-step or per-job timing) inside GitHub Actions. The goal is to make future optimization decisions based on measured cost and latency rather than assumptions.

## Classification
feature proposal

## Clarifications
### Answered
- Capture requested explicitly via `duumbi-codex-intake` skill usage request.

### Open
- Which LLM usage dimensions should be mandatory first (tokens, cost estimate, request count, latency percentile, failures)?
- Should metrics remain in GitHub artifacts/logs only, or be exported to an external dashboard/store?
- What optimization target should triage prioritize first: cost reduction, faster runtime, or reliability?

## Relevant DUUMBI context
- `AGENTS.md` (repository guidance and stage/UX constraints for DUUMBI workflows).
- Skill instructions: `.agents/skills/duumbi-codex-intake/SKILL.md`.

## Related GitHub context
Not inspected; triage should verify GitHub state later.

## Initial routing recommendation
GitHub issue

## Requested follow-up
- Capture this as an idea for later triage and potential implementation planning.

## Notes
- Facts:
  - User requested Stage 2 capture with the `duumbi-codex-intake` skill.
  - User idea targets GitHub Actions telemetry for LLM usage and runtime.
- Assumptions:
  - The proposal applies to DUUMBI automation flows that invoke LLM-backed steps.
  - Existing CI does not yet provide sufficient cost/runtime observability.
- Recommendations:
  - Stage 4 triage should convert this into a scoped GitHub issue with explicit metric definitions and acceptance criteria.
