# DUUMBI-583: Define Traced Build Mode And Telemetry Configuration

## Summary

Define the first narrow Phase 13 build and configuration surface for local
runtime telemetry.

The product contract is an opt-in traced build mode, conservative local
telemetry configuration, and enough visible behavior to prove that traced
builds can be enabled without changing default production-style builds. This
spec is the first child execution slice of #580. It prepares later work for
function/block trace events, local crash artifacts, and back-mapping evidence,
but does not itself require remote observability, Studio UI, repair execution,
or hot-swap behavior.

For v1, the accepted product surface is:

```text
default build remains uninstrumented
explicit traced build selection -> local-only telemetry settings -> testable traced binary
```

## Problem

Phase 13 needs runtime telemetry to connect execution failures back to DUUMBI
graph context. The active product direction is deliberately local-first: a
developer should be able to enable tracing, reproduce a failure, inspect local
evidence, and later map that evidence back to graph function/block context.

The current source state does not yet expose the first switch needed for that
path:

- `duumbi build` supports input, output, and offline mode, but no trace mode.
- `duumbi run` executes the current workspace binary and has no trace-specific
  behavior.
- `.duumbi/config.toml` has workspace, provider, registry, dependency, vendor,
  logging, and related settings, but no telemetry section.
- `runtime/duumbi_runtime.c` prints panic messages to stderr and exits, but does
  not record telemetry or crash artifacts.
- `src/compiler/lowering.rs` already traverses graph functions, blocks, and
  nodes during lowering, but has no product-level instruction to emit trace
  instrumentation.

Without a narrow build/config contract, later telemetry work risks expanding
into remote OpenTelemetry export, production instrumentation, dashboards, or
repair-agent execution before the basic opt-in boundary is reviewable and
testable.

## Outcome

When this issue is implemented:

- A developer can explicitly request a traced build.
- A normal `duumbi build` remains uninstrumented and produces the same
  production-style behavior as before.
- Traced mode is local-only by default and can be tested without network access,
  provider credentials, external collectors, Studio, or remote services.
- Telemetry configuration has conservative defaults for enabled/off behavior,
  sampling mode, sample rate, and local artifact paths.
- Configuration makes it clear when telemetry is disabled, when it is available
  for traced builds, and where local artifacts will be written by later slices.
- Single-file and workspace build paths either share the same traced-build
  contract or explicitly reject unsupported combinations with actionable errors.
- Later child issues can add function/block trace events and crash artifacts
  against this accepted surface without redesigning the user-facing flag and
  configuration semantics.
- The execution issue remains open for later Stage 7, Stage 8, implementation,
  review, and closure workflow; this product spec PR is specification-only.

## Scope

### In Scope

- Define an opt-in traced build surface, with `duumbi build --trace` as the
  canonical v1 command.
- Preserve uninstrumented default behavior for `duumbi build`.
- Define local telemetry configuration fields:
  - telemetry enabled/off behavior.
  - sampling mode.
  - sample rate.
  - local artifact directory or file path defaults.
- Require traced behavior to be local-only for this issue.
- Require traced build behavior to be testable without external collectors or
  network services.
- Require clear user-facing behavior for unsupported trace/config combinations.
- Require review evidence that default build output and default runtime behavior
  are not unintentionally instrumented.
- Define enough product boundary for later #584 function/block trace event work
  and #585 local artifact work.
- Include CLI help and documentation expectations for the traced build surface
  if the implementation adds the command flag.

### Explicitly Out Of Scope

- Technical specification content or implementation instructions.
- Implementation code, source changes outside this product spec, or Ralph
  cycles during Stage 6.
- Function/block trace event emission; that belongs to #584.
- Crash dumps, trace-to-graph mapping artifacts, or crash artifact inspection;
  those belong to #585 and #586.
- OpenTelemetry remote export, OTLP endpoints, Jaeger, Grafana Tempo, or other
  external collectors.
- Studio telemetry UI, graph overlays, dashboards, alerting, or monitoring.
- Repair-agent execution, repair context generation, patch validation, or
  automatic repair acceptance.
- Hot-swap, running binary replacement, production customer crash ingestion,
  account-based telemetry, upload, retention, consent, or privacy policy work.
- Making traced mode the default for release or production-style builds.
- Requiring `duumbi run --trace` in v1. A run shortcut may be proposed later,
  but the accepted first surface is traced build selection.

## Constraints And Assumptions

Facts:

- Issue #583 is open and accepted for specification.
- Issue #583 is labeled `accepted` and `needs-spec`.
- Issue #583 is in the `Spec Needed` Project status at Stage 6 intake.
- The Stage 5 decision comment on 2026-05-22 records `Decision: Accept`,
  `Next state: Spec Needed`, and no remaining open questions.
- #583 is a child issue of #580.
- The approved #580 product and technical specs sequence #583 before #584,
  #585, #586, #588, and #587.
- The active PRD defines the first runtime feedback promise as local
  developer/test feedback, not production customer telemetry or autonomous
  repair.
- The Phase 13 roadmap note proposes a default uninstrumented `duumbi build`
  and an instrumented `duumbi build --trace`.
- `src/cli/mod.rs` currently exposes `duumbi build` with input, `--output`, and
  `--offline`, but no `--trace` flag.
- `src/main.rs` dispatches `duumbi build` through shared build helpers and
  `duumbi run` through the workspace binary runner.
- `src/cli/commands.rs` and `src/workspace.rs` provide separate single-file and
  workspace build paths.
- `src/config.rs` has no telemetry config section today.
- `runtime/duumbi_runtime.c` has `duumbi_panic()` and runtime support functions,
  but no trace hooks or crash artifact writes.
- `docs/architecture.md` already lists `.duumbi/telemetry/traces.jsonl` as the
  intended location for runtime trace mapping data.

Assumptions:

- The safest v1 surface is `duumbi build --trace`; it is explicit, easy to
  document, and avoids making `duumbi run` rebuild implicitly.
- `duumbi run --trace` is useful but not required for the first accepted slice
  because a traced binary can be produced through `build --trace` and then run
  normally.
- Telemetry configuration should be accepted even when telemetry is disabled, so
  users can prepare config without changing default build behavior.
- The product contract should define local artifact locations now, even if later
  issues create the artifacts.
- Sampling should be part of the configuration contract now, but actual event
  volume and performance tuning belong to later trace-event work.
- Local artifact paths use `.duumbi/telemetry/` by default, with
  `DUUMBI_TELEMETRY_DIR` as the explicit override for tests and one-off runs.

Constraints:

- Default builds must not emit trace calls, initialize telemetry runtime state,
  write telemetry artifacts, or require telemetry config.
- Traced builds must be opt-in and visibly local-only in help text, docs, and
  review evidence.
- Telemetry must not require OpenAI, Anthropic, registry, Studio, Slack, GitHub,
  OTLP, or any other networked service.
- Invalid telemetry config must fail early with actionable error messages before
  a traced build silently falls back to surprising behavior.
- Sampling defaults must avoid broad overhead. A disabled or conservative sample
  rate is acceptable until later trace-event behavior is implemented.
- Local artifact path defaults must stay inside the workspace unless the user
  explicitly overrides them.
- This spec PR must not mark #583 complete; it is a Stage 6 review artifact
  only.

## Decisions

- **Decision:** Use a file-based product spec for #583.
  **Evidence:** The issue spans CLI UX, configuration semantics, compiler
  lowering boundaries, runtime behavior, test expectations, and Phase 13
  sequencing. It is durable enough to require source-controlled review history.

- **Decision:** Treat `duumbi build --trace` as the canonical v1 opt-in traced
  build command.
  **Evidence:** The issue and Phase 13 note both name `duumbi build --trace`.
  The current `run` command executes an already built workspace binary, so
  putting the first switch on build keeps rebuild behavior explicit.

- **Decision:** Keep default `duumbi build` uninstrumented.
  **Evidence:** #583 acceptance criteria require default builds to remain
  uninstrumented, and the #580 parent spec excludes always-on production
  instrumentation from the first Phase 13 slice.

- **Decision:** Define telemetry config as local-only for this issue.
  **Evidence:** The accepted #580 product slice requires local developer/test
  telemetry without network services or external collectors.

- **Decision:** Include sampling mode and sample rate in the first config
  surface, but do not require full performance tuning in this issue.
  **Evidence:** The issue explicitly asks for sampling mode and sample rate.
  Later #584 trace-event work is the right place to prove exact event emission
  and overhead behavior.

- **Decision:** Do not require `duumbi run --trace` in v1.
  **Evidence:** The accepted issue asks whether `run` should accept trace flags
  as an open question. A traced build followed by a normal run is enough to
  establish the first build/config contract without adding implicit rebuild
  semantics.

- **Decision:** This spec PR must be specification-only and must not close the
  execution issue.
  **Evidence:** Stage 6 creates a reviewable product spec. Product approval,
  technical specification, implementation, implementation review, and closure
  evidence remain later workflow stages.

## Behavior

### Defaults

- `duumbi build` remains uninstrumented unless the user explicitly selects
  traced behavior.
- Telemetry is local-only by default.
- No telemetry artifacts are written by default uninstrumented builds.
- Missing telemetry configuration does not prevent default builds.
- Missing telemetry configuration for traced builds uses documented conservative
  defaults.
- The default artifact root is `.duumbi/telemetry/`.
- `DUUMBI_TELEMETRY_DIR` overrides the configured artifact directory for tests
  and one-off runs.
- Telemetry config is read for traced builds. Default uninstrumented builds do
  not validate telemetry config as part of the build path.

### Inputs

- CLI command:
  - `duumbi build --trace`
  - existing `duumbi build` arguments such as optional input, `--output`, and
    `--offline`.
- Optional workspace telemetry configuration.
- A DUUMBI graph input or workspace build context.
- Local filesystem access for workspace-local artifact paths.

### Outputs

- Default build:
  - native binary output as before.
  - no trace instrumentation contract.
  - no telemetry artifact writes.
- Traced build:
  - native binary output that is marked or built as trace-capable for later
    function/block trace behavior.
  - user-facing success/failure behavior consistent with existing build output.
  - no dependency on external collectors or services.
- Traced-build config validation:
  - valid telemetry settings are accepted.
  - invalid booleans, unsupported sampling modes, invalid sample rates, or unsafe
    artifact paths fail with actionable diagnostics.

### Configuration Contract

The product-level telemetry configuration must cover these concepts, regardless
of exact TOML names chosen by the technical spec:

- `enabled` or equivalent:
  - default: false.
  - never enables instrumentation by itself.
  - `duumbi build --trace` is the only v1 opt-in for instrumentation.
  - within traced builds, controls whether runtime telemetry emission and local
    artifact writes are enabled when the trace-capable binary runs.
  - if `enabled = true` is configured but `--trace` is omitted, the build remains
    uninstrumented and should not fail because of telemetry config alone.
  - if `duumbi build --trace` is used with `enabled = false`, the build may
    produce a trace-capable binary, but runtime telemetry emission and local
    artifact writes stay disabled.
- `sampling mode`:
  - supports a deterministic mode suitable for tests.
  - supports a conservative default suitable for local development.
- `sample rate`:
  - accepts an explicit bounded value.
  - rejects values outside the documented range.
- `artifact-dir` or equivalent artifact path:
  - defaults to `.duumbi/telemetry/`.
  - can be inspected by tests.
  - does not imply remote upload.
  - `DUUMBI_TELEMETRY_DIR` overrides `artifact-dir` for tests and one-off runs.
  - runtime telemetry file paths are resolved from `DUUMBI_TELEMETRY_DIR` when
    set, otherwise from `artifact-dir`, whose default is `.duumbi/telemetry/`.
- `capture-values`:
  - exists in the parent telemetry configuration surface.
  - value and argument capture are out of scope for this issue.
  - must stay false in the first implementation.

### Error States

- Unknown trace flags or unsupported command combinations fail through normal CLI
  argument validation.
- Invalid telemetry config fails before traced-build compilation succeeds.
- Default `duumbi build` without `--trace` does not fail solely because a
  telemetry config section exists or contains telemetry-specific errors.
- A traced build requested outside a supported build context must fail with a
  message that explains what is supported.
- An unsafe telemetry artifact path is rejected with an actionable error; it is
  not silently normalized.
- Offline mode remains about dependency resolution and must not be reinterpreted
  as telemetry network control; traced telemetry is local-only regardless of
  `--offline`.
- If later implementation cannot preserve traced behavior for both single-file
  and workspace builds in the same slice, unsupported paths must fail explicitly
  rather than silently producing uninstrumented output.

### Invariants

- Default build behavior remains stable and uninstrumented.
- Traced behavior is opt-in.
- Local telemetry configuration does not create remote export behavior.
- Trace-capable builds can be exercised in CI without provider credentials,
  network access, external services, or Studio.
- The build/config surface remains narrow enough for #584 and #585 to add
  trace events and artifacts without changing the user-facing command contract.

## BDD Scenarios

Feature: Traced build mode and telemetry configuration

Rule: Default builds remain uninstrumented

Scenario: Build without trace uses the existing default behavior

Given a valid DUUMBI graph or workspace
When the developer runs `duumbi build` without `--trace`
Then the build succeeds or fails according to the existing build rules
And no telemetry configuration is required
And no trace artifacts are written
And the output is not treated as trace-capable

Scenario: Missing telemetry config does not break normal builds

Given a valid DUUMBI workspace with no telemetry config section
When the developer runs `duumbi build`
Then the build does not fail because telemetry config is absent
And no telemetry artifact directory is required

Rule: Traced builds are explicit and local

Scenario: Developer requests a traced build

Given a valid DUUMBI graph or workspace
When the developer runs `duumbi build --trace`
Then DUUMBI builds a trace-capable binary or reports a traced-build-specific
error
And the command does not require an OTLP collector, provider credentials,
Studio, or network access
And the command keeps telemetry behavior local to the workspace

Scenario: Traced build preserves existing build options

Given a valid DUUMBI graph or workspace
When the developer runs `duumbi build --trace --output <path>`
Then DUUMBI writes the binary to the requested output path when the build
succeeds
And the traced build behavior is selected explicitly

Scenario: Offline traced build does not require telemetry services

Given a valid DUUMBI workspace whose dependencies are available from workspace
or vendor layers
When the developer runs `duumbi build --trace --offline`
Then dependency resolution follows the existing offline rules
And telemetry does not attempt remote export
And no external telemetry service is required

Rule: Telemetry configuration is conservative and validated

Scenario: Traced build uses conservative defaults

Given a valid DUUMBI workspace with no telemetry config section
When the developer runs `duumbi build --trace`
Then DUUMBI uses documented conservative local defaults
And those defaults do not enable remote telemetry export
And telemetry emission defaults to disabled unless local config enables it
And artifact paths resolve to `.duumbi/telemetry/` unless
`DUUMBI_TELEMETRY_DIR` overrides them

Scenario: Telemetry config alone does not instrument default builds

Given a valid DUUMBI workspace with telemetry config enabled
When the developer runs `duumbi build` without `--trace`
Then DUUMBI produces a default uninstrumented build
And the telemetry config does not imply trace instrumentation
And telemetry config validation does not block the default build path

Scenario: Disabled telemetry config gates runtime emission in traced builds

Given a valid DUUMBI workspace with telemetry config disabled
When the developer runs `duumbi build --trace`
Then DUUMBI may produce a trace-capable binary
But runtime telemetry emission and local artifact writes remain disabled

Scenario: Invalid sample rate is rejected

Given a DUUMBI workspace with telemetry config
And the sample rate is outside the documented allowed range
When the developer runs `duumbi build --trace`
Then DUUMBI fails before producing a successful traced build
And the error message identifies the invalid sample rate
And the error message states the valid range or expected format

Scenario: Unsupported sampling mode is rejected

Given a DUUMBI workspace with telemetry config
And the sampling mode is not supported
When the developer runs `duumbi build --trace`
Then DUUMBI fails before producing a successful traced build
And the error message identifies the unsupported sampling mode

Scenario: Unsafe artifact path is rejected

Given a DUUMBI workspace with telemetry config
And the configured artifact path is not acceptable for local telemetry output
When the developer runs `duumbi build --trace`
Then DUUMBI rejects the path with an actionable error
And it does not silently upload telemetry elsewhere

Rule: Unsupported trace surfaces are explicit

Scenario: Run trace shortcut is not available in v1

Given a DUUMBI workspace with a previously built binary
When the developer asks for traced behavior through a command not accepted by
the implementation
Then DUUMBI reports that traced build selection is supported through
`duumbi build --trace`
And it does not silently rebuild or run an uninstrumented binary as if tracing
were active

Scenario: Unsupported build context fails clearly

Given a DUUMBI build context that the implementation does not yet support for
traced builds
When the developer runs `duumbi build --trace`
Then DUUMBI fails with a message that names the unsupported context
And the message gives the supported traced-build path
And the command does not produce an unmarked uninstrumented binary while
claiming traced mode succeeded

## Tasks

- Define the CLI product surface for traced builds, centered on
  `duumbi build --trace`.
- Define the telemetry config fields, defaults, validation behavior, and local
  artifact path expectations.
- Ensure single-file and workspace build behavior is either consistently
  trace-capable or explicitly scoped with clear unsupported-context messages.
- Preserve default build behavior and prove that telemetry is not enabled by
  default.
- Add or update user-facing help/docs for the traced build surface and local
  config behavior.
- Prepare the accepted surface for later #584 trace-event emission and #585
  local artifact creation without implementing those later slices here.

## Checks

- Product review verifies this spec against #583, the Stage 5 decision, the #580
  parent spec, the active PRD, and Phase 13 notes.
- CLI help or command parsing evidence shows `duumbi build --trace` exists after
  implementation.
- Focused tests prove `duumbi build` without `--trace` does not require
  telemetry config and does not emit telemetry artifacts.
- Focused tests prove `duumbi build --trace` accepts valid local config and
  rejects invalid sample rates, unsupported sampling modes, and unacceptable
  artifact paths.
- Focused tests prove traced build behavior works without network access,
  provider credentials, external collectors, or Studio.
- Review evidence compares default build behavior before and after the change.
- Regression tests cover both single-file and workspace build paths, or record a
  reviewed explicit limitation for unsupported traced contexts.
- CI includes normal formatting and Rust test checks expected for the touched
  modules at implementation time.
- Manual smoke evidence includes at least:
  - default build path.
  - traced build path.
  - invalid telemetry config path.
  - local-only behavior with no external collector.

## Open Questions

- Should `duumbi run --trace` become a later convenience command after the
  traced build contract is stable?
- Should workspace initialization eventually write an explicit commented
  telemetry example, or should traced builds continue to rely on internal
  defaults unless users add the section themselves?
- What exact sampling modes should the technical spec accept for the first
  implementation: disabled, deterministic, probabilistic, always, or a smaller
  subset?

None of these questions blocks Stage 7 review because the product boundary is
clear: default builds stay uninstrumented, traced builds are opt-in, config is
local and conservative, and remote export is out of scope.

## Sources

- Related to #583:
  https://github.com/hgahub/duumbi/issues/583
- Parent product spec:
  `specs/DUUMBI-580/PRODUCT.md`
- Parent technical sequencing context:
  `specs/DUUMBI-580/TECHNICAL.md`
- Stage 4 triage comment:
  https://github.com/hgahub/duumbi/issues/583#issuecomment-4522470830
- Stage 5 acceptance decision:
  https://github.com/hgahub/duumbi/issues/583#issuecomment-4522522582
- Active PRD:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - PRD.md`
- Glossary:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Works (Developed Materials)/DUUMBI - Glossary.md`
- Agentic development map:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/01 Atlas (Knowledge Base)/Maps (Overviews)/DUUMBI Agentic Development Map.md`
- Phase 13 roadmap note:
  `/Users/heizergabor/space/hgahub/duumbi-vault/Duumbi/05 Archive/Execution and Roadmap Docs/DUUMBI - Phase 13 - Self-Healing & Telemetry.md`
- Architecture reference:
  `docs/architecture.md`
- CLI command surface:
  `src/cli/mod.rs`
  `src/main.rs`
  `src/cli/commands.rs`
  `src/workspace.rs`
- Config source:
  `src/config.rs`
- Compiler lowering source:
  `src/compiler/lowering.rs`
- Runtime source:
  `runtime/duumbi_runtime.c`
