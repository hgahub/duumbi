# DUUMBI Greptile Review Rules

Greptile is manual-only for this repository. It should be used as a scarce,
high-signal reviewer for complex implementation PRs, not as a default reviewer
for every PR or commit.

## Review Focus

- Prioritize correctness, security, data loss, crashes, Rust compile/runtime
  behavior, graph invariants, async/concurrency safety, provider/auth handling,
  registry integrity, MCP/intent boundaries, and missing tests.
- Treat style, naming, wording, documentation polish, and optional refactors as
  non-blocking unless they create concrete behavior or maintenance risk.
- Prefer one concise blocking finding over several speculative comments.
- Do not ask for broad refactors unless the current change is unsafe or violates
  an approved spec.

## DUUMBI Boundaries

- The semantic graph IR is the central source of truth.
- Cranelift types must remain contained inside `src/compiler/`.
- AI agents should receive read-only graph snapshots and propose explicit
  mutation plans.
- Query mode is read-only by default; mutation belongs in Agent or Intent mode.
- Stage 10 implementation must stay inside approved product and technical specs
  and Ralph-cycle resource gates.

## PR Type Expectations

- Specs, docs, and routine config PRs should not receive Greptile review unless
  a developer explicitly asks for it.
- Implementation PRs with complex Rust, security-sensitive behavior,
  provider/auth changes, async/concurrency behavior, compiler/runtime changes,
  or cross-module refactors are the main Greptile use case.
- If a finding is a nit, label it as non-blocking in the review text.
