---
name: duumbi-spec-desktop
description: "Prepare and review DUUMBI Stage 6–9 specifications when the Owner manually starts a Codex Desktop task for an accepted issue. Produce a spec-only PR; never launch the retired Grok worker, merge, or start implementation."
---

# Manual Desktop specification

Start only when the Owner submits the handoff prompt in a Desktop task. Stage 5 acceptance
alone does not start a task. Read `docs/automation/manual-spec-handoff.md` for the handoff
and migration contract.

1. Read the target issue and latest structured Stage 5 Accept comment from the official
   workflow. Check acceptance is current and the issue is open. Follow its rationale and
   subsequent explicit Owner decisions over older intake suggestions. Do not request a new
   Accept merely because transport moved from the VM to Desktop.
2. Inspect repository instructions, current source, linked specifications and dated research.
   Treat old artifacts as evidence, not current approval. Resolve public technical questions
   through research. Distinguish verified, documented unsupported and unverified capabilities;
   implementability gaps must not become invented support. Ask genuine product decisions
   directly in this Desktop conversation. Do not send the Owner through VM continue/retry loops.
3. Prepare English `specs/DUUMBI-N/PRODUCT.md` with scope, acceptance criteria, BDD scenarios
   and a reasoned keep-together/split decision. Review it against the accepted requirements
   (Stage 7) and record findings and their resolution. Propose sub-issue boundaries before
   allocating execution work; follow the Owner's decision if decomposition changes scope.
4. Prepare `specs/DUUMBI-N/TECHNICAL.md` against that product contract, covering affected
   code, interfaces, compatibility, capability evidence, migrations, tests and dependencies.
   Perform a separate Stage 9 implementability review, record findings and resolve blockers.
   Request-only placeholders or unresolved required capabilities are not review approval.
5. Open or update a spec-only PR with non-closing issue references. Link it from the issue;
   report review evidence, validation and remaining decisions. Preserve the execution issue.
   Stop for Owner review/merge. Do not mark Ready for Build without the actual required gate
   evidence; do not merge or start Stage 10. Existing Stage 7/9 workflows may only be used
   under their explicit human review/merge contract, not as an automatic step of this skill.

Do not invoke `scripts/spec-automation/run.mjs`, `duumbi-spec-autopilot`, or
`duumbi-delivery-autopilot`. No Grok event, VM queue, scheduled job, decision reuse or
checkpoint deletion is required for this manual path. If an old worker still owns the
same issue, stop concurrent writing and have its routines disabled before proceeding.
