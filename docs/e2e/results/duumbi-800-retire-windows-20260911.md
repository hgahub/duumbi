# DUUMBI-800 implementation evidence — 2026-09-11

Issue: [#800](https://github.com/hgahub/duumbi/issues/800).
Plan: [product](../../../specs/DUUMBI-800/PRODUCT.md) and
[technical](../../../specs/DUUMBI-800/TECHNICAL.md).
Branch: `codex/duumbi-800-retire-windows`, based on `main@52931c9`.

## Ralph Cycle 1 Evidence Report

**Cycle goal:** retire owned native Windows code, CI and support claims while
preserving Linux/macOS behavior. **Status:** completed implementation.

### Changes and source inventory

- CI now contains only the Ubuntu matrix member. Keeping that member preserves
  both the existing `check (ubuntu-latest)` context and the required aggregate
  `check`. Removed MSYS2, Windows compiler/libcurl setup and OS-only conditions.
  Coverage still runs on Ubuntu; the build script is included in scope filters.
- Removed the Windows Job Object/suspended-process/TCP-table module and all its
  integration. Unix process groups, cleanup, deadlines, cancellation and port
  probes remain. Removed Windows environment forwarding and executable suffix
  adapters from the verifier, workspace, telemetry and integration fixtures.
- Selected the existing POSIX branches of the C runtime, deleting Win32/MSVC
  networking, filesystem, timing, atomic and thread-local alternatives. Linker
  flags no longer include Winsock. Session saving directly replaces the old
  snapshot with Unix rename; its existing test now also checks a second save.
- Cargo regenerated the lockfile after removing the root `windows-sys` target
  dependency. Its only lockfile change is deleting that direct dependency edge;
  no package versions changed. Vendored SQLite has no diff.
- Current README, agent instructions, process-verifier documentation and
  generated release notes explain retirement and link/reference the archive.
  Release architectures are unchanged. README prerequisites reflect the locked
  Cranelift/Wasmtime Rust 1.95 floor and Ubuntu libcurl requirement.
- Native Windows targets receive an explicit build-script diagnostic pointing
  to the source archive. The guard only rejects target OS `windows`.

Residual references are intentional: upstream SQLite portability code,
transitive Windows packages in Cargo.lock, historical specifications/results,
the retirement policy/guard, generic path validation and portable PowerShell
completion generation. Rust slice `.windows()` calls are unrelated. In
particular, runtime rejection of backslashes, colons, traversal and absolute
paths remains intact. Prior specifications, including #378/#379/#380/#780,
describe their historical scope; #800 supersedes their Windows requirements.

### Preserved baseline

Read the remote tag before development and again after removal:

- Tag: [`windows-support-final-2026-09-10`](https://github.com/hgahub/duumbi/tree/windows-support-final-2026-09-10)
- Annotated tag object: `15eb936bc36a98f21b7bfea9059af53a7b856189`
- Peeled commit: `52931c91d9fc6f7a541e1c250e0837a9980dd46b`

The kickoff `origin/main` was the same commit, so no additional archive was
needed. The existing tag was neither moved nor republished; no release was made.
The active main ruleset requires `check` (GitHub Actions integration 15368),
which is retained with its original failure propagation.

## Ralph Cycle 2 Evidence Report

**Cycle goal:** validate the final source and prepare human review.
**Status:** local validation completed; remote CI/review tracked in the PR.

### Actual macOS application execution before PR creation

Host: macOS arm64, `rustc 1.95.0 (59807616e 2026-04-14)`.
Ran the built CLI with a fresh temporary HOME and no provider credentials:

```text
$ duumbi --version
duumbi 0.4.1-preview
$ duumbi init native-smoke
Project initialized at native-smoke/.duumbi
$ duumbi build
Build successful: ./.duumbi/build/output
$ duumbi run
exit=0
$ file .duumbi/build/output
Mach-O 64-bit executable arm64
```

Init, build and run all exited 0. Build and run were repeated successfully after
the final executable-path cleanup. Native HTTP/SQLite/JSON integration scenarios
exercise the richer service path with deterministic provider fixtures.

### Validation

- `cargo test --all -- --test-threads=4`: **3062 passed, 0 failed, 1 ignored**
  across 47 result groups including doctests. The ignored test is the existing
  explicitly gated production-registry smoke. Tests used the existing cache via
  `CARGO_TARGET_DIR`; none were skipped for this change.
- All five `integration_duumbi780_process_evidence` scenarios passed: authored
  SQLite service evidence, normal repair and re-verification, unrepaired JSON
  mismatch classification, determinism replay and authoring failure preservation.
  Unix cancellation/descendant cleanup, timeout/reap, delayed port release and
  occupied-port tests passed, as did TCP/HTTP/DB/JSON, file APIs, telemetry and
  the phase-1 workspace init/build/run test.
- `cargo fmt --check`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed on final Rust changes.
- `cargo audit`: passed, with five existing allowed warnings; this is not a
  claim that the dependency tree has no advisories.
- `pre-commit run --all-files`: passed, including YAML, formatting and secrets.
- Executed the compiled build script with target metadata: Windows rejected
  with the archival diagnostic; Linux, macOS and `unknown` were not rejected.
  This tests the diagnostic, not a Windows or WebAssembly build.
- Executed the actual aggregate shell body with prerequisite results:
  `success -> 0`; `failure`, `cancelled`, `skipped -> 1`.
- Exercised the scope regex: source/runtime/build-script/CI changes select Rust
  checks; README and product-spec-only changes select the documentation path.
- Final diff review: no blocking source finding; no owned Windows execution
  branches, suffix adapters or direct Windows dependency remain.

### Failures investigated

An initial full run hit the existing one-second stdin test deadline. The
unchanged test passed in isolation (0.42 seconds) and in subsequent full unit
test runs. No deadline or assertion was relaxed.

The earlier phase-1 workspace test assumed `target/debug/duumbi`, which failed
when reusing a separate `CARGO_TARGET_DIR`. Its executable-suffix cleanup now
uses Cargo's `CARGO_BIN_EXE_duumbi`; the build and run assertions are retained.

### Resources and review boundary

DUUMBI external LLM calls: **0**. External provider cost: **USD 0**. Codex internal
work is subscription reasoning; exact token/cost accounting is unavailable.
Compute consists of local compilation/tests and the implementation PR's CI.
No migrations, new runtime dependencies or external service changes.

Codex self-review covers the final source diff. The final PR carries Ubuntu CI
and automated Codex review results. Runtime/process changes qualify for optional
manual Greptile consideration under the repository policy; no Greptile request
or Slack notification was sent. The Stage 11 handoff path is
`.github/workflows/implementation-review-request.yml`. Finish in review;
merging and marking the issue Done are outside this implementation handoff.
