# DUUMBI-369: Medium Article - Why Software Should Be a Graph, Not Text - Technical Specification

## Implementation Objective

Implement the approved product spec in `specs/DUUMBI-369/PRODUCT.md` by
producing, publishing, cross-posting, and evidencing the Phase 14 article
"Why Software Should Be a Graph, Not Text" or a substantially equivalent final
title.

The implementation must prove this flow:

```text
approved thesis -> source-backed claim ledger -> article draft ->
3+ explanatory visuals -> editorial/claim review -> Medium publication ->
dev.to and Hashnode cross-posts -> issue evidence -> later Stage 12 closure
```

This technical spec is for content execution. It does not require compiler,
CLI, Studio, registry, website, runtime, generated artifact, or product-spec
changes.

## Agent Audience

- Codex implementation agents running bounded Ralph cycles.
- Codex App or Codex Cloud agents coordinating article drafting, evidence
  review, link checking, and publication handoff.
- Human operator responsible for external publishing account access, final
  editorial approval, and any irreversible publication action.
- Stage 10 coordinator agents that must keep the work inside the approved
  product and technical specs.
- Stage 11/12 reviewer or closure agents verifying final publication evidence.

## Source Context

- Product spec: `specs/DUUMBI-369/PRODUCT.md`
- GitHub issue: https://github.com/hgahub/duumbi/issues/369
- Stage 4 triage decision:
  https://github.com/hgahub/duumbi/issues/369#issuecomment-4542717002
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/369#issuecomment-4596574731
- Stage 6 product spec PR: https://github.com/hgahub/duumbi/pull/645
- Stage 7 product spec approval:
  https://github.com/hgahub/duumbi/issues/369#issuecomment-4596962187
- Stage 7 merge SHA:
  `15d44b42a45a318b9a31355a11c731dec1b1361e`
- Repo instructions: `AGENTS.md`
- Architecture reference: `docs/architecture.md`
- Coding conventions: `docs/coding-conventions.md`
- README: `README.md`

Relevant source facts verified for Stage 8:

- `README.md` describes DUUMBI as an AI-first semantic graph compiler where
  programs are stored as JSON-LD graphs rather than text files, then validated,
  compiled, and linked through Cranelift.
- `README.md` links public surfaces for website, docs, and registry. On
  2026-06-02, `https://www.duumbi.dev/` and `https://docs.duumbi.dev/` returned
  HTTP 200. `https://github.com/hgahub/duumbi` returned HTTP 200. A registry
  header probe did not complete and must not be treated as verified publication
  evidence without a later timed check.
- `docs/architecture.md` documents the JSON-LD parser, semantic graph,
  validator, Cranelift compiler, linker, runtime, AI mutation pipeline, Query
  mode, Intent workflow, error JSONL format, and roadmap boundaries.
- `src/parser/mod.rs` parses JSON-LD into typed AST structures and maps
  malformed input to structured parser errors.
- `src/graph/mod.rs` defines `SemanticGraph` over `petgraph::StableGraph`,
  graph node metadata, graph edges, and lookup maps by `NodeId`.
- `src/compiler/mod.rs` exposes the `CodegenBackend` boundary and keeps
  Cranelift-specific implementation inside `src/compiler/`.
- `src/agents/mod.rs` defines the LLM provider trait, graph mutation provider
  calls, and read-only `answer` surface used by Query mode.
- `src/intent/mod.rs` defines Intent-driven development as YAML intent specs
  that can be created, reviewed, executed, verified, and archived.
- `src/query/mod.rs` is explicitly read-only conversational Query mode.
- `src/bench/showcases.rs` embeds six showcase intent specs:
  calculator, fibonacci, sorting, state machine, multi-module, and string ops.
  These are evidence candidates, not automatic proof that public showcase demos
  are live.
- `src/telemetry/mod.rs` contains local telemetry and trace-map support, but the
  PRD and product spec require production telemetry, silent updates, and
  autonomous repair acceptance to remain future-facing unless separately
  proven and approved.

Relevant tests and verification sources:

- `tests/` contains phase integration tests for parser, graph, compiler,
  intent, registry, telemetry, Query, and Studio-related behavior.
- `docs/e2e/scoring.md` describes intent-create quality scoring.
- `docs/e2e/intents/`, `docs/e2e/corpus/`, and `docs/e2e/results/` provide
  E2E intent and benchmark evidence candidates if the article discusses current
  results.
- `src/bench/showcases.rs` has unit coverage that all embedded showcase specs
  parse, but Stage 10 must run or otherwise verify any showcase/demo claim
  before using it as public evidence.

Relevant Obsidian context:

- DUUMBI vault: active PRD note.
- DUUMBI vault: active Glossary note.
- DUUMBI vault: Core Concepts Map note.
- DUUMBI vault: Agentic Development Map note.
- DUUMBI vault: archived Phase 14 Marketing and Go-to-Market plan.
- DUUMBI vault: JSON-LD Graph Representation dot.
- DUUMBI vault: Semantic Fixed Point dot.
- DUUMBI vault: Compilation Pipeline dot.
- DUUMBI vault: Public Docs as Product Interface dot.

Verified product/workflow facts:

- Issue #369 is open.
- The issue currently has `accepted`, `product-spec-approved`, and
  `needs-tech-spec` labels, plus Phase 14 and marketing labels.
- Stage 7 approval recorded no blocking findings, no non-blocking findings, and
  no remaining open questions.
- The product spec is merged on `main` and requires Medium publication,
  2000-3000 words, 3+ diagrams or illustrations, links to DUUMBI website and
  GitHub repository, cross-posting to dev.to and Hashnode, and final delivery
  evidence on the issue.

Assumptions for implementation:

- Article drafting can use Codex or another approved LLM as a writing assistant,
  but the deliverable is judged by publication evidence and claim review, not by
  the model used to draft it.
- A durable source-repo article draft is not required unless a human explicitly
  asks for one. Scratch drafts, external editors, platform drafts, screenshots,
  and issue evidence are sufficient for this issue.
- Human access or approval is required for irreversible external publication
  actions on Medium, dev.to, and Hashnode unless credentials and publication
  authority have already been delegated in the active run.

## Affected Areas

Expected Stage 10 delivery surfaces:

- Medium article page.
- dev.to cross-post page.
- Hashnode cross-post page.
- GitHub issue #369 evidence comment.
- Temporary local scratch drafts or exported article/diagram artifacts used
  during implementation, if needed.
- Optional screenshots or exported images used as evidence, if the publishing
  surfaces make automated verification weak.

Source-repo areas used as evidence but not expected to change:

- `README.md`
- `docs/architecture.md`
- `docs/e2e/scoring.md`
- `docs/e2e/intents/`
- `docs/e2e/corpus/`
- `docs/e2e/results/`
- `src/parser/`
- `src/graph/`
- `src/compiler/`
- `src/agents/`
- `src/intent/`
- `src/query/`
- `src/bench/`
- `src/telemetry/`
- `tests/`
- `AGENTS.md`

Areas expected not to change:

- `specs/DUUMBI-369/PRODUCT.md`
- implementation code
- tests
- migrations
- generated outputs
- runtime assets
- website or docs source
- registry data
- product spec approval comments or labels except normal workflow routing
- technical spec approval comments or labels except Stage 9 workflow routing

If Stage 10 discovers that a durable repo artifact is required, such as a
committed markdown article draft or committed diagram source files, it must stop
and request human approval or open a separate issue. Do not smuggle those assets
into an implementation PR under this technical spec.

## Technical Approach

### 1. Build A Source-Backed Claim Ledger

Before drafting the article, collect a concise claim ledger with three classes:

- Current, source-backed DUUMBI facts.
- Future-facing roadmap or vision claims.
- Forbidden or unsupported claims.

Minimum current-fact sources:

- `README.md`
- `docs/architecture.md`
- `specs/DUUMBI-369/PRODUCT.md`
- the active PRD and Glossary context
- source files listed in Source Context when a claim refers to implementation
  structure

Minimum forbidden claim checks:

- no guarantee of correct programs from arbitrary natural language
- no autonomous repair acceptance without validation and human review
- no production customer telemetry as current behavior
- no silent update or hot-swap claims as current behavior
- no claim that Phase 14 demos, registry modules, website assets, or public
  examples are complete unless verified in the implementation run

### 2. Draft The Article Against The Product-Spec Structure

Use the approved article structure from the product spec as the default outline:

1. why text is a weak substrate for AI-generated software
2. what changes when program meaning is represented as a typed semantic graph
3. how DUUMBI represents program logic with JSON-LD, stable identifiers, graph
   nodes, edges, types, and metadata
4. how DUUMBI validates and compiles graph structure through parser, semantic
   graph IR, validator, Cranelift lowering, linker, and runtime output
5. how AI agents fit through graph patches, Query mode, Intent workflow,
   review evidence, and human verification
6. what DUUMBI has proven so far, using verified source, docs, tests, or demo
   evidence
7. what remains future-facing
8. why the graph-not-text thesis matters for reliable agentic software
   development
9. where readers can try DUUMBI, read docs, inspect source, and follow the
   project

Recommendations:

- Keep the final article in plain technical English.
- Use "DUUMBI stores program logic as a typed semantic graph" as the central
  thesis.
- Avoid unsupported comparative claims against specific tools unless a separate
  verified benchmark source supports them.
- Prefer evidence phrasing such as "the current architecture does X" or "the
  project is designed to Y" over unqualified reliability promises.

### 3. Produce Or Select Three Required Visuals

Minimum visuals:

- text-centric compiler/AI workflow versus DUUMBI graph-centered workflow
- DUUMBI pipeline from JSON-LD graph through parser, semantic graph, validator,
  Cranelift, linker, and binary
- human-agent validation and review loop covering human intent, Query mode,
  bounded mutation, validation, tests, review evidence, and human verification

Visual requirements:

- each visual must have alt text or adjacent explanatory text when the platform
  supports it
- visuals must not depend on private paths, private screenshots, secrets, or
  internal-only Slack/vault context
- if diagrams are generated by an image model or design tool, record the tool
  and prompt summary in implementation evidence when practical
- if a platform transforms or drops a visual, replace it or record a caveat
  before marking that platform complete

### 4. Review Draft Quality And Claims Before Publication

Run an explicit article readiness review before publishing:

- word count is 2000-3000 words
- required narrative sections are present
- all required DUUMBI links are present
- at least three visuals are present
- visuals have alt text or adjacent explanatory text
- claim ledger checks pass
- current/future boundaries are visible
- stale `#2` showcase dependency is not treated as proof
- private local paths, secrets, private Slack content, and private vault details
  are absent

Recommended local checks for a markdown draft:

```bash
wc -w <draft.md>
rg -ni "guarantee|guaranteed|production telemetry|silent update|autonomous repair|self-healing" <draft.md>
rg -n "https://www.duumbi.dev|https://duumbi.dev|https://github.com/hgahub/duumbi" <draft.md>
```

These commands are aids, not substitutes for human or reviewer judgment.

### 5. Publish And Cross-Post With Evidence

Publication order:

1. Medium first, preferably through a suitable publication when it does not
   materially delay launch.
2. Direct Medium publication is acceptable if publication submission is blocked,
   rejected, unavailable, or would delay launch beyond the planned window.
3. Cross-post to dev.to and Hashnode after the Medium URL exists.
4. Configure canonical URL support where available, or visibly state that the
   Medium article is the original.

Publication actions are external and may be irreversible. Require human
approval when account access, paid promotion, publication submission,
cross-posting under an official account, or final publish buttons are involved
and authority is not already delegated.

### 6. Record Final Delivery Evidence

Post a GitHub issue comment on #369 with:

- Medium URL
- Medium publication name or direct-publication fallback note
- dev.to URL
- Hashnode URL
- word count
- diagram count
- links included
- source references used
- claim-boundary review note
- cross-post canonical/original-source handling
- material platform differences or caveats
- explicit note that the issue remains open for later review and Stage 12
  closure

Use non-closing references only, such as `Related to #369`.

## Invariants

- The execution issue remains open until later review and Stage 12 closure.
- Do not use GitHub auto-closing keywords against #369 in PR titles, PR bodies,
  commits, comments, or generated artifacts.
- Do not modify implementation code, tests, generated artifacts, runtime assets,
  or `specs/DUUMBI-369/PRODUCT.md` under this technical spec.
- Do not publish unsupported current-product claims.
- Do not expose private local paths, secrets, tokens, private Slack content, or
  unpublished internal-only planning details in public article content.
- Do not treat the stale `#2` dependency as evidence that showcase demos are
  complete.
- Do not spend money, use paid promotion, or perform irreversible external
  publication actions without delegated authority or human approval.
- Do not start Stage 12 closure. Final publication evidence still needs later
  review before closure.

## BDD-To-Test Mapping

| Product BDD scenario | Verification type | Evidence required |
| --- | --- | --- |
| Reader encounters the core thesis without prior DUUMBI knowledge | Editorial/review evidence | Article review checklist confirms the first third of the article states the graph-not-text thesis, explains inspectability/validation value, and includes public try/read/source links. |
| The article explains current DUUMBI behavior without overclaiming | Source/evidence review plus text search | Claim ledger maps current claims to README, architecture docs, source files, tests, or verified live evidence. Draft search and human review confirm forbidden claims are absent or explicitly future-facing. |
| The article includes the required narrative coverage | Editorial checklist | Review checklist confirms problem statement, semantic graph thesis, DUUMBI implementation, current results/evidence or caveats, and introduced/linked technical terms. |
| The Medium article meets acceptance criteria | Live publication evidence | Medium URL is live; word count is 2000-3000; diagram count is at least three; website and GitHub links are present; issue evidence comment is posted. |
| The article uses diagrams that clarify the thesis | Visual review evidence | Reviewer records that one visual covers text versus graph workflows, one covers the DUUMBI pipeline, one covers the human-agent validation loop, and each has alt text or adjacent explanation where supported. |
| Cross-posts preserve canonical context | Live URL and content review | dev.to and Hashnode URLs are live; each links to or canonically references the Medium original; required DUUMBI links remain present; visual differences are replaced or recorded as caveats. |
| Publication venue fallback is handled explicitly | Publication decision evidence | If a preferred Medium publication is unavailable or would materially delay launch, the live Medium article still exists and the issue evidence records why direct Medium publication was used. |
| The execution issue records delivery evidence | GitHub issue comment | Issue #369 comment includes Medium/dev.to/Hashnode URLs, word count, diagram count, links included, source references used, claim review note, caveats, and open-issue workflow note. |
| Stale showcase dependency does not create unsupported claims | Source/evidence review | Implementation evidence records that #2 is unrelated to showcase demos; demo-specific claims are used only when current demos, screenshots, source evidence, or run logs are verified. |

Automation notes:

- Word count can be checked locally with `wc -w` before publication and manually
  after publication if the platform markup changes the count.
- Link availability can be checked with `curl -I -L --max-time 10 <url>` where
  the platform permits HEAD requests; otherwise use browser/manual evidence.
- Claim-boundary checks require review judgment. Text search is only a
  backstop.

## Live E2E Plan

Canonical interface:

- Delivery interface: Medium, dev.to, and Hashnode publishing surfaces.
- Evidence interface: GitHub issue #369 comments and final live URLs.
- DUUMBI evidence interface: source inspection plus optional CLI/test commands
  only when the article cites specific runnable behavior or current results.

Real provider/LLM path:

- No DUUMBI live provider call is required because this issue does not change or
  validate DUUMBI LLM behavior.
- Codex or another approved LLM may be used as a content drafting assistant.
  Codex internal reasoning usage is reported only as an estimate. External
  model/API/tool calls must be counted under the resource policy.

Required credentials or sessions:

- Medium account or publication access.
- dev.to account access.
- Hashnode account access.
- Optional image/design tool access if diagrams are generated outside the repo.
- GitHub issue write access for delivery evidence.

Expected external LLM call count and cost:

- Expected DUUMBI provider calls: 0.
- Expected external content-assistant calls: 0-6 if the implementation agent
  chooses to use an external LLM or image generator beyond Codex internal
  reasoning.
- Expected external LLM/image cost: less than USD 2. Human approval is required
  above USD 2 or above 10 external calls.

Suggested live path:

1. Create a scratch article outline and claim ledger.
2. Draft the article and three visuals in a non-committed scratch area or the
   publishing platform draft editor.
3. Run the readiness review and local checks where possible.
4. Verify current public links with timed requests, for example:

   ```bash
   curl -I -L --max-time 10 https://www.duumbi.dev/
   curl -I -L --max-time 10 https://docs.duumbi.dev/
   curl -I -L --max-time 10 https://github.com/hgahub/duumbi
   ```

5. Publish or submit to Medium after human approval when needed.
6. Cross-post to dev.to and Hashnode with canonical/original-source handling.
7. Verify final live URLs and required links.
8. Post issue evidence.

Artifacts:

- Article draft or exported copy when available.
- Visual files or platform-rendered visual evidence.
- Medium URL.
- dev.to URL.
- Hashnode URL.
- Link-check evidence.
- Issue #369 delivery evidence comment.

Pass/fail criteria:

- Pass when every product-spec BDD scenario has evidence and the issue comment
  records delivery.
- Fail when a required platform cannot publish, a required link is unavailable,
  the article is outside 2000-3000 words, fewer than three visuals are present,
  required claim boundaries fail, or account/publication authority is missing.

## Ralph Cycle Protocol

Each cycle must:

1. summarize the current state and remaining unmet requirements
2. propose one bounded implementation goal
3. list intended file areas, external surfaces, and commands
4. estimate resource use and risk
5. check whether the resource gate requires human approval
6. implement only the approved or resource-permitted goal
7. run the agreed checks
8. report evidence, failures, and remaining gaps
9. stop if requirements are met, a blocker appears, resource thresholds are
   exceeded, scope changes, or the autonomous batch cap is reached

## Cycle Budget

- Default cycle size: one bounded implementation goal per cycle.
- Max committed source files per cycle: 0 unless a human explicitly approves a
  durable repo artifact. Scratch files outside the repo are allowed for drafting
  and verification.
- Max external publication surfaces per cycle: 1 publish/cross-post action.
- Expected command budget: up to 10 local source/evidence inspection commands
  and up to 6 timed URL checks per cycle.
- Expected external LLM calls: 0-3 per low-budget content drafting or diagram
  cycle; 0 DUUMBI provider calls unless the article cites live DUUMBI LLM
  behavior.
- Expected external LLM cost: less than USD 2 per autonomous batch.
- Human approval required when planned external LLM usage exceeds USD 2,
  exceeds 10 calls, exceeds approved scope, adds risky dependencies or
  irreversible operations, introduces migrations or persistent data changes,
  creates security/privacy risk, reaches a blocker, uses paid promotion,
  publishes through official accounts without delegated authority, requires
  account credentials, or needs a product/architecture decision.
- External LLM usage counted: DUUMBI live provider calls and external
  model/agent CLI or image-generation calls. Codex internal reasoning usage is
  reported only as an estimate.
- Autonomous batch cap: 3 low-budget cycles.
- When to stop and ask for human guidance: missing publishing authority, missing
  account credentials, desired public claim lacks evidence, required platform is
  unavailable, diagram generation requires paid or sensitive tooling,
  publication timing requires a launch trade-off, or any requirement conflicts
  with claim boundaries.

## Task Breakdown

1. Confirm Stage 10 entry state: #369 is Ready for Build, technical spec is
   approved, and the execution issue remains open.
2. Build the source-backed claim ledger from the approved product spec,
   README, architecture docs, relevant source files, tests/results, and active
   PRD/glossary context.
3. Verify current public DUUMBI links that may appear in the article.
4. Decide whether any showcase demos or current results are reliable enough to
   cite; if not, use architecture, pipeline, and source-backed examples instead.
5. Draft the article outline against the approved article structure.
6. Draft the 2000-3000 word article.
7. Create or select the three required visuals and alt/adjacent explanatory
   text.
8. Run claim-boundary, word-count, link, visual, and editorial readiness checks.
9. Obtain human approval for external publication actions if not already
   delegated.
10. Publish on Medium or record direct-publication fallback when a preferred
    publication path would delay launch.
11. Cross-post to dev.to and Hashnode with canonical/original-source handling.
12. Verify final URLs, links, visuals, and caveats.
13. Post delivery evidence to issue #369.
14. Stop for review; keep #369 open and do not start Stage 12.

Independently executable slices:

- Claim ledger and source verification.
- Article outline and draft.
- Visual production.
- Link and publication-surface verification.
- Cross-post preparation after Medium URL exists.
- Final issue evidence.

## Verification Plan

Before publication:

- Confirm article draft word count is 2000-3000 words.
- Confirm all required narrative sections are present.
- Confirm at least three visuals are present and mapped to the required visual
  topics.
- Confirm each visual has alt text or adjacent explanation where platform
  support allows.
- Confirm the article links to DUUMBI website and GitHub repository.
- Confirm technical terms are introduced before use or linked.
- Confirm claim ledger marks current, future, and unsupported claims.
- Confirm no private local paths, secrets, unpublished Slack details, or
  unsupported internal claims appear in public content.

After publication:

- Verify Medium URL is live.
- Verify Medium publication name or direct-publication fallback caveat is
  recorded.
- Verify dev.to URL is live.
- Verify Hashnode URL is live.
- Verify cross-post canonical/original-source handling.
- Verify required DUUMBI links remain present in each published surface.
- Verify visuals render or caveats are recorded.
- Verify issue #369 delivery evidence comment includes every required field.

Code/test commands:

- No Rust implementation tests are required for article-only delivery unless the
  article cites a specific live behavior, benchmark, demo, or test result.
- If the article cites current tests or showcases, run the narrowest relevant
  command and record evidence. Examples:

  ```bash
  cargo test --test integration_phase1
  cargo test --test integration_phase5
  cargo test --all
  ```

- Use `cargo test --all` only when broad current-product claims depend on the
  whole repo state. Do not imply test coverage proves product claims beyond what
  the tests actually exercise.

## Completion Criteria

Stage 10 implementation for #369 is complete only when:

- A live Medium article URL exists. A fallback note may explain direct Medium
  publication instead of preferred-publication acceptance, but it cannot replace
  the live Medium URL requirement.
- Final article is 2000-3000 words.
- Article includes at least three diagrams or illustrations.
- Article links to DUUMBI website and GitHub repository.
- Article covers problem statement, semantic graph thesis, DUUMBI
  implementation, and current results/evidence or caveats.
- Current and future claims are separated.
- Unsupported production telemetry, silent update, autonomous repair acceptance,
  and arbitrary natural-language correctness claims are absent.
- dev.to and Hashnode cross-posts are live.
- Cross-post canonical/original-source handling is recorded.
- Issue #369 has a delivery evidence comment with URLs, word count, diagram
  count, source references used, links included, claim review note, and caveats.
- Issue #369 remains open for later review and Stage 12 closure.

## Failure And Escalation

- If account credentials or publication authority are missing, stop and ask the
  human operator for approval or credentials. Do not work around account access.
- If preferred Medium publication review would delay launch, direct Medium
  publication is allowed after recording the fallback rationale.
- If a required public URL is unavailable, use the strongest available fallback
  allowed by the product spec and record the caveat. The website and GitHub
  links remain required for final completion.
- If a showcase/demo claim cannot be verified, remove or soften the claim and
  use architecture/source evidence instead.
- If diagrams cannot render on a platform, replace them with compatible images
  or record a caveat and provide textual equivalents.
- If article scope expands to comparison pages, videos, website changes,
  registry changes, or demo implementation, stop and request separate product
  authorization.
- If external LLM/image cost or call count exceeds the budget, stop and request
  human approval before continuing.
- If a reviewer identifies unsupported claims after publication, correct the
  public posts when possible, record the correction evidence on #369, and keep
  the issue open for review.

## Open Questions

None blocking for Stage 8 or Stage 10 planning.

Accepted execution-time choices that do not block Stage 9 approval:

- Which Medium publication, if any, will accept the article within the launch
  timing constraints?
- Which current demos, screenshots, benchmark outputs, or test results are
  reliable enough to cite publicly during the implementation run?
- Should the public title remain exactly "Why Software Should Be a Graph, Not
  Text", or should it be editorially adjusted while preserving the approved
  thesis?

These choices are bounded by the product spec and this technical spec. If an
implementation run turns any of them into a scope, claim, publication,
security/privacy, cost, or architecture decision, the agent must stop and request
human guidance.
