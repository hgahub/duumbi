# DUUMBI-817 — Desktop specification reviews

Related to #817. Review date: **2026-09-16**. Baseline: `5e9fa23`.
Owner revision: [5705161911](https://github.com/hgahub/duumbi/issues/817#issuecomment-5705161911).
Base acceptance: [5704193547](https://github.com/hgahub/duumbi/issues/817#issuecomment-5704193547).

The 2026-09-16 Owner lifecycle revision and canonical-effort-only answer in
[MODEL_LIFECYCLE.md](MODEL_LIFECYCLE.md) supersede the old scope. Both review
passes below were repeated against the revised five-document artifact.

These are fresh, separate Stage 7 product-content and Stage 9 implementability
review passes by the Desktop authoring agent. They are **self-reviews**, not an
independent reviewer, human approval or a dispatched workflow gate. Historical
worker verdicts were not reused. Owner review/merge remains required; the issue
is not marked Ready for Build.

## Desktop decisions and operational boundary

- The Owner confirmed all three retired Grok Spec routines are disabled and no
  #817 worker is active. No routine or worker was invoked by this task.
- The Owner challenged the proposed blocker for models without effort controls:
  ordinary calls should omit effort. This is consistent with the latest Accept.
- Review corrected the evidence threshold: official model capability plus an
  applicable common API contract may establish support. A separate example or
  support letter for every model is not required. The later Owner revision explicitly retires Haiku and expands the catalog.
- Old attempts, branch tips, evidence pack and recovery files were preserved.
- No automatic Stage 7/9 workflow was dispatched: the existing gate workflows
  can advance state/merge, which this task explicitly forbids.

## Stage 7 — product content

Reviewed PRODUCT.md against each clause of renewed acceptance, current source,
prior product artifact and Owner clarification.

| Finding | Resolution |
| --- | --- |
| S7-01: Older product required effective omitted defaults. | Removed that obligation; omission is tested by field absence, including Astra. |
| S7-02: Unknown explicit effort must not be called unsupported. | Added distinct V/N/U classes, error outcomes, zero-send BDD and acceptance criteria. |
| S7-03: Earlier drafting treated missing Grok Build effort/example as a spec blocker. | Separated baseline tool support from optional effort controls; EVIDENCE.md records the documentation composition. Explicit Grok Build remains U. |
| S7-04: Provider-by-provider release would conflict with joint delivery. | One unit owns all 12 criteria and 17 models/eight providers under the later Owner revision; no new sub-issues. |
| S7-05: Global allowlisting could break old custom endpoints/unknown overrides. | Legacy omitted path is explicit; new effort still requires verification. |
| S7-06: Live access was conflated with specification feasibility. | Live coverage blocks implementation release. Documentation establishes the spec contract; no live result is claimed here. |
| S7-07: Original scope conflicts with the Owner's complete retirement list. | 17-model target, seven introductions, 22 old model retirements plus OpenRouter removal; DeepSeek Pro retained; no unrequested Google replacement. |
| S7-08: API aliases are not independent native levels. | Owner explicitly chose native levels only; add canonical-admission policy, distinct NonCanonicalEffort error, AC-12 and BDD-16. |
| S7-09: Retirement could erase credentials or break mixed configurations. | Keep parsing/storage, expose retired state only in relevant management flows, forbid silent replacement and maintain usable valid providers. |

**Content assessment:** no unresolved blocking product finding. All accepted
invariants appear in AC-817-01–12 and BDD-817-01–18. No new product decision is
required. This is a recommendation for Owner review, not formal Stage 7 approval.

## Stage 9 — implementability

Separately reviewed TECHNICAL.md against the product contract, actual adapter,
factory, catalog and fallback code, and freshly retrieved official sources.

| Finding | Resolution / implementation verification |
| --- | --- |
| S9-01: The reported failure was being read as proof the current client sends effort. | Baseline explicitly says no effort field exists; retained failure is historical evidence. |
| S9-02: A global OpenAI client migration would break other compatible providers. | Separate Responses codec/profiles, exact native routing and legacy isolation tests. |
| S9-03: Flattening tools without controlling strictness can change optional arguments. | OpenAI sets `strict:false`; xAI uses its own allowed key set. Tool schema semantics stay unchanged. |
| S9-04: Method-specific builders can lose effort on capture, callback or retry. | Immutable options and one preflight plan for all five methods; factory/wrapper/repair tests. |
| S9-05: Capability rejection and transient fallback could substitute a model. | N/U are non-transient; explicit/pinned operations retain identity even on transient errors. Legacy omitted fallback remains separately tested. |
| S9-06: Catalog reasoning Booleans/discovery cannot establish wire support. | Release-reviewed lookup joins existing catalog IDs; no V1 schema migration or remotely invented capabilities. |
| S9-07: New Responses use can introduce provider-side response storage. | Explicit `store:false`, independent requests, no stored continuation IDs. |
| S9-08: Claude reasoning/truncation or incomplete SSE might yield partial mutations. | Parse only complete structured tool calls; terminal/status checks and incomplete-output errors; keep graph validation. |
| S9-09: Retired Haiku and active Fable/Opus must not share controls. | Haiku capability rows removed and pinned alias retired; separate active-model none/effort mappings; automatic Fable tool choice. |
| S9-10: Grok Build evidence was rejected solely for lack of a model-named example. | Resolved at specification level using X2+X4+X5+X6. This composition is explicit, not a fabricated live result. Grok 4.6 effort is not extrapolated. |
| S9-11: Runtime output allowance could silently change reasoning or cost. | Independent bounded internal output option; retain old defaults; never reduce effort to fit; report truncation. |
| S9-12: Required validation must use the actual production path and all 17 models. | 238-cell offline matrix plus opt-in joint live tool/answer/callback/capture coverage; no passing result for skipped/denied rows. |
| S9-13: Current retired explicit-model resolver silently falls back. | Fallible typed resolution; retirement precedes compatibility/selection; factory, direct clients, probes, fallback and catalog refresh all enforce tombstones. |
| S9-14: One shared Chat payload cannot express the five added providers safely. | Native profiles define thinking/effort keys, MiniMax reasoning separation, DeepSeek tool_choice omission, Kimi output-field/fixed-parameter rules and GLM always-on constraints. |
| S9-15: Qwen endpoint assumptions could redirect region-bound credentials. | Document Singapore workspace endpoint, exact trusted descriptor and existing base_url integration; do not migrate legacy origins automatically. |
| S9-16: GLM Flash's old model-card documentation link failed. | Official documentation index resolves /guides/vlm/glm-5.3-flash; it explicitly declares model ID, function calling and GLM5.3 text-parameter parity. Omitted tool path is V. |
| S9-17: DeepSeek Pro retirement/default guidance conflicts across pages. | Current pricing/corrected change log retains Pro; model-specific high/max retained. Distinct low stays U because generic schema and older Pro guidance conflict. This optional U does not block omission or spec review. |
| S9-18: Documented alias acceptance might be mislabeled as unsupported. | V† retains V evidence with canonical=false; reject via policy, exclude from admitted live cells, never claim the provider rejects the alias. |
| S9-19: Old catalogs could resurrect OpenRouter/retired models. | Same release-owned tombstones apply after decoding and before routing/publishing; preserve V1 readability and test stale/refresh/discovery paths. |

**Implementability assessment:** no unresolved blocking specification finding
after these corrections. Required interfaces, mappings, affected code, backward
compatibility, dependencies and verification are specified. Documentation
composition for Grok Build is the explicit evidence judgment for Owner review.
Runtime correctness, access, live latency/output budgets and joint release
evidence remain implementation work, not an already passed gate.

## Validation of this specification PR

- Only Markdown under `specs/DUUMBI-817/` is changed.
- Cross-document requirements, matrix dimensions and local links are checked.
- Repository pre-commit checks and Git whitespace validation are run before commit;
  exact outcomes are reported in the PR description.
- Rust runtime tests and live provider calls are not applicable to this docs-only
  change and were not run. No credentials were read for provider calls.
- No merge, implementation, issue closure or Ready for Build transition occurs.

Future content changes require revisiting the corresponding review findings;
this record must not be treated as approval of arbitrary later revisions.
