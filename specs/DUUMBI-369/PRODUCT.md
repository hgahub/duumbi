# DUUMBI-369: Medium Article - Why Software Should Be a Graph, Not Text

## Summary

DUUMBI should have a published thought-leadership article titled "Why Software
Should Be a Graph, Not Text" or a substantially equivalent final title. The
article should explain DUUMBI's core thesis to a technical audience: program
logic can be stored as a typed semantic graph instead of text files, giving
humans and agents a more inspectable, traceable, and validation-friendly
software representation.

The article is a Phase 14 launch asset. It should be published on Medium, with a
publication such as Towards Data Science preferred when available, then
cross-posted to dev.to and Hashnode. It should link to `duumbi.dev`, DUUMBI's
GitHub repository, and relevant public docs. It should include at least three
diagrams or illustrations that make the thesis understandable without requiring
the reader to inspect source code.

This is a content product spec. It does not specify implementation work in the
compiler, Studio, registry, docs site, or website.

## Problem

DUUMBI's README and architecture docs make a strong technical claim: programs
are stored as JSON-LD semantic graphs, validated, compiled through Cranelift,
and mutated by AI agents under graph validation. That claim is central to the
launch strategy, but it is currently scattered across README copy, architecture
notes, PRD language, and Phase 14 planning.

The launch audience needs one coherent public article that explains:

- why text-centric programming creates brittle AI coding workflows
- what a typed semantic graph changes
- how DUUMBI represents, validates, mutates, compiles, and inspects program
  behavior
- what DUUMBI can already claim based on current source and docs
- what remains future-facing and must not be oversold

Without this article, the launch story depends on terse repository copy and
fragmented documentation. That weakens discoverability, reduces credibility, and
makes it easier for readers to misunderstand DUUMBI as just another AI coding
assistant rather than a compiler and development workflow built around
first-class semantic structure.

## Outcome

When this is done:

- A 2000-3000 word English article is published on Medium.
- The article is accessible to technical readers familiar with compilers, AI
  coding tools, or developer tooling, without assuming prior DUUMBI knowledge.
- The article covers problem statement, semantic graph thesis, DUUMBI
  implementation, current evidence or results, and launch-relevant implications.
- At least three diagrams or illustrations are included.
- The article links to `https://www.duumbi.dev/` or `https://duumbi.dev/`, the
  DUUMBI GitHub repository, and relevant public docs when available.
- The article is cross-posted to dev.to and Hashnode with canonical-link handling
  or an explicit note identifying the Medium post as the original when the
  platform supports it.
- The final issue evidence includes publication URLs, cross-post URLs, diagram
  count, word count, source references used, and a short review note showing that
  current/future claims were checked.
- The execution issue remains open for later workflow stages until Stage 12
  closure verifies final delivery evidence.

## Scope

### In Scope

- Draft the article thesis, outline, and final English article.
- Explain DUUMBI's text-to-graph argument using evidence from current README,
  architecture docs, active PRD/glossary context, and Phase 14 launch planning.
- Describe DUUMBI as an AI-first semantic graph compiler using JSON-LD, typed
  semantic graph IR, validation, graph mutation, Cranelift code generation,
  Query mode, Intent workflow, and evidence-oriented agent development.
- Separate current behavior from future-facing vision in the article.
- Include at least three diagrams or illustrations, such as:
  - text pipeline versus graph-centered pipeline
  - intent to semantic graph to validation to binary
  - agent mutation with validation and review evidence
  - semantic fixed point across intent, graph, behavior, and evidence
- Publish on Medium, preferring Towards Data Science or a similar publication
  when practical.
- Cross-post to dev.to and Hashnode after Medium publication.
- Add correct links to DUUMBI's website, GitHub repository, and relevant docs.
- Capture publication evidence in GitHub issue comments.
- Preserve a reviewable record of article scope, diagrams, links, and
  cross-posting.

### Explicitly Out Of Scope

- Creating or changing compiler, CLI, Studio, registry, website, or docs code.
- Creating a technical spec for compiler or website implementation.
- Starting DUUMBI Stage 8, Stage 10, or Ralph-cycle implementation work.
- Publishing unsupported claims about production self-healing, autonomous repair
  acceptance, fully reliable AI code generation, or customer telemetry.
- Marketing features that do not reliably work.
- Producing YouTube videos, Hacker News launch content, landing page changes, or
  comparison pages unless separately authorized by their own issues.
- Treating the malformed dependency reference to "Issue #2" as proof that
  showcase demos are complete.
- Replacing public docs or README copy with the article.

## Constraints And Assumptions

Facts:

- Issue #369 is open and accepted for specification.
- Issue #369 is labeled `accepted`, `needs-spec`, `phase-14`, and
  `module:marketing`.
- The Stage 5 decision comment dated 2026-06-01 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- The issue acceptance criteria require a Medium article, 2000-3000 words, at
  least three diagrams or illustrations, links to DUUMBI web/GitHub surfaces, and
  cross-posts to dev.to and Hashnode.
- The repository README describes DUUMBI as an AI-first semantic graph compiler
  where programs are stored as JSON-LD graphs, validated, compiled, and linked to
  native binaries through Cranelift.
- `docs/architecture.md` describes JSON-LD parsing, semantic graph construction,
  schema validation, Cranelift compilation, graph patching, Query mode, and
  Intent workflow boundaries.
- The active PRD says DUUMBI should reduce drift between intent,
  implementation, execution state, and durable knowledge by making intent and
  structure first-class artifacts.
- The active glossary defines Semantic Graph as a structured representation of
  program meaning using nodes, edges, identifiers, types, and metadata.
- Phase 14 planning names this exact Medium article as the core thesis article
  for content marketing and says marketing should not promote unreliable
  features.
- GitHub issue/PR number 2 currently resolves to a closed PR titled
  `feat: Implement Supabase Auth`, not a showcase-demos issue. The dependency
  text in #369 is therefore treated as a stale or malformed dependency reference
  unless a later source identifies the intended showcase-demos artifact.

Assumptions:

- A file-based product spec is appropriate because the article is a durable Phase
  14 launch artifact with external publication, cross-posting, diagrams, and
  current-versus-future claim risk.
- The article can proceed without blocking on the malformed `#2` dependency
  because Stage 5 acceptance says no remaining open questions and the issue has
  enough source context to specify desired content behavior.
- The final publication venue may require editorial review, so the implementation
  task should allow publication submission evidence when a preferred Medium
  publication has not yet accepted the article.
- `duumbi.dev`, docs, and README links should be verified at publication time,
  because public URLs and site availability are operationally time-sensitive.
- Diagrams can be custom images, Mermaid-exported visuals, screenshots, or
  editorial illustrations as long as they clarify the article's technical claims
  and are suitable for the target publishing platforms.

Constraints:

- The article must not imply that current DUUMBI behavior is stronger than the
  evidence in README, source docs, tests, and accepted specs.
- The article must distinguish current product behavior from roadmap or vision.
- The article must avoid confidential implementation details, secrets, private
  Slack content, and unpublished internal-only claims.
- Cross-posts should avoid duplicate-content confusion by using canonical URLs or
  visible original-source notes where supported.
- This spec PR must not close the execution issue. It is a Stage 6 review
  artifact only, and later stages still need to approve the spec, draft any
  required technical plan, execute publication work, review delivery, and perform
  closure.

## Decisions

- **Decision:** Use a file-based product spec for #369.
  **Evidence:** The work is public, user-visible, launch-critical, involves
  external publication and cross-posting, and needs durable review history for
  claim boundaries and evidence expectations.

- **Decision:** The article should frame DUUMBI as a semantic graph compiler, not
  as a generic AI coding assistant.
  **Evidence:** README and architecture docs identify the typed JSON-LD semantic
  graph and validation-before-compilation pipeline as the core differentiator.

- **Decision:** The article should include explicit current-versus-future
  boundaries.
  **Evidence:** The PRD and Phase 14 plan both emphasize evidence-oriented
  claims and warn against marketing unreliable or speculative features as if they
  already work.

- **Decision:** The malformed `#2` dependency is not a Stage 6 blocker.
  **Evidence:** Stage 5 records no remaining open questions. The issue itself
  has enough acceptance criteria and source context to specify the article. The
  dependency should be surfaced as a publication-time evidence check, not treated
  as completion evidence.

- **Decision:** Cross-posting is part of the deliverable, not an optional
  follow-up.
  **Evidence:** Issue #369 acceptance criteria explicitly require cross-posting
  to dev.to and Hashnode.

- **Decision:** This spec PR must leave the execution issue open.
  **Evidence:** Stage 6 drafts a product spec only. Stage 7 approval, downstream
  execution, review, and Stage 12 closure still need to happen.

## Behavior

### Defaults

- The primary article target is Medium.
- The preferred Medium publication is Towards Data Science or a similar
  technical publication, but direct Medium publication is acceptable if a
  publication submission is unavailable, rejected, or would materially delay the
  launch plan.
- The article target length is 2000-3000 words.
- The article should use plain technical English, avoiding unexplained DUUMBI
  internals and avoiding hype-heavy marketing language.
- The article should link to public DUUMBI surfaces rather than private vault
  notes.

### Inputs

- Issue #369 title, body, acceptance criteria, labels, milestone, and comments.
- Stage 4 triage comment for launch-critical routing context.
- Stage 5 human acceptance comment for gate evidence.
- DUUMBI README.
- `docs/architecture.md`.
- Active vault PRD, Glossary, Core Concepts Map, Agentic Development Map, Phase
  14 plan, JSON-LD Graph Representation, Semantic Fixed Point, Compilation
  Pipeline, and Public Docs as Product Interface notes.
- Current public links for website, docs, registry, and repository.
- Any available showcase demos or screenshots verified at execution time.

### Outputs

- Medium article URL.
- dev.to cross-post URL.
- Hashnode cross-post URL.
- At least three diagrams or illustrations embedded in the article and carried
  into cross-posts when each platform supports them.
- Publication evidence comment on issue #369, including:
  - Medium URL
  - cross-post URLs
  - word count
  - diagram count
  - links included
  - current-versus-future claim review note
  - any publication acceptance or submission caveats

### Article Structure

The final article should include these content sections, though final headings
may be editorially adjusted:

1. Why text is a weak substrate for AI-generated software.
2. What changes when program meaning is represented as a typed semantic graph.
3. How DUUMBI represents program logic with JSON-LD, stable identifiers, graph
   nodes, edges, types, and metadata.
4. How DUUMBI validates and compiles graph structure through parser, semantic
   graph IR, validator, Cranelift lowering, linker, and runtime output.
5. How AI agents fit into the workflow through graph patches, Query mode, Intent
   workflow, review evidence, and human verification.
6. What DUUMBI has proven so far, using current README, architecture, tests,
   quickstart, and demo evidence available at execution time.
7. What remains future-facing, including stronger autonomous repair, production
   telemetry, registry ecosystem maturity, and broader showcase coverage.
8. Why the graph-not-text thesis matters for reliable agentic software
   development.
9. Where readers can try DUUMBI, read docs, inspect source, and follow the
   project.

### Claim Boundaries

- The article may say syntax errors are structurally avoided when program
  structure is represented and validated as graph data, matching README language.
- The article may say graph validation, type checking, compilation, tests, and
  review evidence make AI-generated changes more inspectable and safer to
  evaluate.
- The article may say DUUMBI aims to reduce drift among intent, implementation,
  runtime behavior, and knowledge.
- The article must not say DUUMBI guarantees correct programs from arbitrary
  natural language.
- The article must not say autonomous repair is accepted without validation and
  human review.
- The article must not claim production customer telemetry or silent update
  behavior as current product capability.
- The article must not claim all planned Phase 14 demos, registry modules, or
  website assets are complete unless verified during execution.

### Diagrams And Illustrations

The article should include at least three visuals. Acceptable minimum set:

- Diagram 1: Text-centric compiler/AI workflow versus DUUMBI graph-centered
  workflow.
- Diagram 2: DUUMBI pipeline from JSON-LD graph through parser, semantic graph,
  validator, Cranelift, linker, and binary.
- Diagram 3: Intent/agent mutation loop showing human intent, read-only query,
  bounded mutation, validation, tests, review evidence, and human verification.

Recommended optional visuals:

- Semantic fixed point diagram connecting intent, graph, runtime behavior, tests,
  and knowledge.
- A small JSON-LD snippet paired with a simplified graph view.
- A screenshot or diagram from a verified showcase demo, if available.

### Publication And Cross-Posting

- Medium should be the canonical first publication unless editorial workflow
  makes another order necessary.
- If the article is accepted by a Medium publication, record the publication
  name and URL.
- If the article is published directly on Medium, record that as acceptable
  delivery evidence.
- Cross-posts to dev.to and Hashnode should either set the Medium canonical URL
  where supported or visibly state that the Medium article is the original.
- Cross-posts should preserve links and visuals as much as platform support
  allows.
- Final issue evidence should identify any material differences between the
  Medium article and cross-post versions.

### Error States And Recovery

- If Medium publication submission is pending beyond the planned launch window,
  direct Medium publication is acceptable with evidence explaining the fallback.
- If a platform rejects or transforms a diagram, replace it with a compatible
  image or attach a clear textual equivalent before considering the platform
  deliverable complete.
- If optional docs links are unavailable at publication time, use the GitHub
  repository link for supporting context and record the unavailable docs link as
  a caveat. The article still needs DUUMBI website and GitHub repository links
  for final completion.
- If showcase demos are not available or not reliable, the article should avoid
  demo-specific claims and use architecture or pipeline visuals instead.
- If cross-post canonical URL support is unavailable, include an explicit
  original-publication note in the cross-post body.

### Accessibility And Editorial Quality

- Diagrams should have descriptive alt text or adjacent explanatory text where
  the platform supports it.
- The article should avoid relying on images alone to communicate critical
  technical claims.
- Technical terms such as JSON-LD, semantic graph, Cranelift, Query mode, Intent
  workflow, and semantic fixed point should be introduced before use or linked to
  supporting docs.
- The article should avoid private project jargon unless it is defined in the
  article.

## BDD Scenarios

```gherkin
Feature: Publish the DUUMBI graph-not-text thesis article

  Rule: The article explains the approved thesis using current, evidence-backed
  DUUMBI context

    Scenario: Reader encounters the core thesis without prior DUUMBI knowledge
      Given a technical reader familiar with compilers or AI coding tools
      When they read the published Medium article
      Then they understand that DUUMBI represents program logic as a typed
      semantic graph rather than as text files
      And they understand why that representation can make agent-generated
      changes more inspectable and validation-friendly
      And they can identify public links for trying DUUMBI or inspecting the
      repository

    Scenario: The article explains current DUUMBI behavior without overclaiming
      Given the README, architecture docs, PRD, glossary, and Phase 14 plan
      When the article describes DUUMBI capabilities
      Then current behavior is tied to current source or documentation evidence
      And future-facing capabilities are labeled as future, roadmap, or vision
      And the article does not claim guaranteed arbitrary natural-language
      correctness or autonomous repair acceptance
      And the article does not claim production customer telemetry or silent
      update behavior as current capability
      And the article does not claim planned Phase 14 demos, registry modules, or
      website assets are complete unless verified

    Scenario: The article includes the required narrative coverage
      Given issue #369 requires problem statement, semantic graph thesis, DUUMBI
      implementation, and results
      When the final article is reviewed before publication
      Then it includes a problem statement about text-centric AI coding fragility
      And it explains the typed semantic graph thesis
      And it describes DUUMBI's JSON-LD, validation, graph mutation, and
      Cranelift compilation workflow
      And it includes current evidence, results, demos, or explicitly stated
      evidence caveats
      And technical terms are introduced before use or linked to supporting docs

  Rule: The article is publishable as a Phase 14 launch asset

    Scenario: The Medium article meets acceptance criteria
      Given the final article draft is ready for publication
      When it is published on Medium
      Then the published article has 2000-3000 words
      And it includes at least three diagrams or illustrations
      And it links to DUUMBI's website
      And it links to the DUUMBI GitHub repository
      And publication evidence is posted to issue #369

    Scenario: The article uses diagrams that clarify the thesis
      Given the article includes at least three visuals
      When a reviewer inspects the visuals
      Then one visual compares text-centric and graph-centered workflows
      And one visual explains the DUUMBI compilation pipeline
      And one visual explains the human-agent validation and review loop
      And each visual has alt text or adjacent explanatory text where supported

    Scenario: Cross-posts preserve canonical context
      Given the Medium article is published
      When the article is cross-posted to dev.to and Hashnode
      Then each cross-post links back to the Medium original or configures a
      canonical URL where the platform supports it
      And each cross-post preserves required DUUMBI links
      And any missing or transformed visual is replaced or recorded as a caveat

    Scenario: Publication venue fallback is handled explicitly
      Given a preferred Medium publication is unavailable or would delay launch
      When the execution agent publishes directly on Medium
      Then direct Medium publication satisfies the Medium requirement
      And the issue evidence records why the preferred publication path was not
      used

  Rule: Completion evidence is reviewable

    Scenario: The execution issue records delivery evidence
      Given the Medium article and cross-posts are live
      When the execution agent updates issue #369
      Then the comment includes Medium, dev.to, and Hashnode URLs
      And it includes word count, diagram count, links included, and publication
      caveats
      And it includes the source references used
      And it includes a current-versus-future claim review note
      And the execution issue remains open for later workflow closure

    Scenario: Stale showcase dependency does not create unsupported claims
      Given issue #369 references `#2` as showcase demos
      And GitHub number #2 currently resolves to an unrelated closed PR
      When the article discusses demos or examples
      Then demo-specific claims use only verified current demos or source
      evidence
      And missing showcase evidence is treated as a caveat rather than assumed
      completion
```

## Tasks

- Confirm current public URLs for DUUMBI website, docs, registry, and GitHub
  repository.
- Confirm whether any reliable showcase demos, screenshots, or example outputs
  are available for inclusion.
- Draft article outline against the required article structure.
- Draft the article in English at 2000-3000 words.
- Create or select at least three diagrams or illustrations with clear technical
  purpose.
- Review the draft for evidence-backed current claims, future-facing claim
  labels, link correctness, and launch tone.
- Publish on Medium; submit to the preferred Medium publication first when
  timing allows, but use direct Medium publication if needed.
- Cross-post to dev.to and Hashnode with canonical URL handling or visible
  original-source notes.
- Post delivery evidence to issue #369.
- Leave issue #369 open for later review and closure workflow.

Independent work:

- Diagram production can proceed in parallel with article drafting once the
  outline is stable.
- Link verification can proceed independently shortly before publication.
- Cross-posting can proceed after the Medium URL exists.

## Checks

- Product spec review confirms this file changes only
  `specs/DUUMBI-369/PRODUCT.md` in the Stage 6 PR.
- Stage 7 review confirms the spec expresses observable behavior, not technical
  implementation details.
- Article draft review confirms:
  - 2000-3000 words
  - at least three diagrams or illustrations
  - problem statement, semantic graph thesis, DUUMBI implementation, and current
    results/evidence coverage
  - links to DUUMBI website and GitHub repository
  - current-versus-future claim separation
  - no unsupported production self-healing, telemetry, or autonomous repair
    acceptance claims
- Publication evidence confirms:
  - Medium URL is live
  - preferred publication submission status or direct-publication fallback
    decision is recorded as a caveat when relevant
  - dev.to URL is live
  - Hashnode URL is live
  - cross-post canonical or original-source handling is recorded
  - diagrams render acceptably on each platform or caveats are recorded
- BDD scenario coverage:
  - Reader thesis comprehension is covered by article review.
  - Claim-boundary behavior is covered by source/evidence review.
  - Medium acceptance criteria are covered by publication evidence.
  - Cross-post behavior is covered by final URL evidence.
  - Stale dependency handling is covered by demo/evidence review.

## Open Questions

None blocking for Stage 6.

Non-blocking execution-time questions:

- Which Medium publication, if any, will accept the article within the launch
  timing constraints?
- Which showcase demos or screenshots are reliable enough to include without
  weakening Phase 14's "do not market unreliable features" rule?
- Should the final title be exactly "Why Software Should Be a Graph, Not Text",
  or should it be editorially adjusted while preserving the issue-approved
  thesis?

## Sources

- Issue #369: https://github.com/hgahub/duumbi/issues/369
- Stage 4 triage comment:
  https://github.com/hgahub/duumbi/issues/369#issuecomment-4542717002
- Stage 5 human acceptance comment:
  https://github.com/hgahub/duumbi/issues/369#issuecomment-4596574731
- DUUMBI README: `README.md`
- DUUMBI architecture reference: `docs/architecture.md`
- DUUMBI vault: active PRD note.
- DUUMBI vault: active Glossary note.
- DUUMBI vault: Core Concepts Map note.
- DUUMBI vault: Agentic Development Map note.
- DUUMBI vault: archived Phase 14 Marketing and Go-to-Market plan.
- DUUMBI vault: JSON-LD Graph Representation dot.
- DUUMBI vault: Semantic Fixed Point dot.
- DUUMBI vault: Compilation Pipeline dot.
- DUUMBI vault: Public Docs as Product Interface dot.
- GitHub number #2 dependency check:
  https://github.com/hgahub/duumbi/pull/2
