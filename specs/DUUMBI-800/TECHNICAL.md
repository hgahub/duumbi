# DUUMBI-800 — Implementation plan

Product contract: [PRODUCT.md](PRODUCT.md). Issue: #800.

## Boundaries

- Preserve the verified remote archival tag at the unchanged `origin/main`
  baseline `52931c9`; use branch `codex/duumbi-800-retire-windows`.
- Keep the Ubuntu member and `check` aggregate in `.github/workflows/ci.yml`;
  remove Windows jobs, setup, conditions and obsolete comments. Include the
  new build script in Rust-relevant scope detection and coverage path filters.
  Keep coverage execution and release architecture matrices unchanged; update
  the generated release notes with the support policy and archival link.
- Remove `src/bench/process/windows.rs` and its integration. Keep Unix process
  group ownership, deadlines, reaping, cancellation and non-invasive port
  checks. Generated services no longer need Windows environment inheritance.
- Select the existing Unix branches of `runtime/duumbi_runtime.c`, removing
  Win32/MSVC adapters and obsolete stat fallbacks. Preserve Linux/macOS socket,
  filesystem, telemetry and HTTP/SQLite/JSON behavior. Do not edit SQLite.
- Remove the direct Windows dependency through Cargo, Windows linker flags,
  executable suffixes, tests and timeout allowances. Remove the Windows
  delete-before-rename workaround in session saving so Unix replacement is
  atomic. Keep generic hostile-path validation and PowerShell completions
  (PowerShell can run on supported hosts).
- Add a root-package build script rejecting a Windows target with a clear
  diagnostic; it must not reject Studio's separate WebAssembly build.
- Update README, agent platform guidance and current process-verifier docs;
  mark prior process evidence as historical without rewriting its results.

## BDD-to-check mapping

| Product scenario | Evidence |
| --- | --- |
| 1: preserved baseline | Remote annotated tag and peeled commit read before edits and after completion |
| 2: Ubuntu CI and required aggregate | Workflow inspection, YAML/action validation, PR Ubuntu CI and aggregate result; unchanged aggregate failure propagation |
| 3: native behavior | `cargo test --all -- --test-threads=4`, including five #780 native E2E tests, process lifecycle/port tests, TCP/HTTP/DB/JSON/filesystem and session tests; macOS CLI init/build/run smoke before PR |
| 4: unsupported target and owned-code removal | Execute compiled build script with Windows target metadata and assert clear rejection; source/dependency inventory; inspect lockfile diff and unchanged vendored code |

## Resource and completion policy

One bounded implementation cycle followed by an evidence/review cycle; continue
fixes within the same issue as necessary. No external LLM calls or provider
credentials are needed (expected external cost USD 0). Existing deterministic
fixtures provide the native E2E path. Local compilation/tests and Ubuntu CI
are the planned compute cost; allow roughly 30–60 minutes depending on caches.
No new runtime dependencies, migrations or release publication.

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, the full
test command above, `cargo audit` where available and
`pre-commit run --all-files` before committing. Four test threads reduce
contention in existing short process deadlines without skipping tests.
Review the final diff, open a PR only after local application testing, and
record actual results and remaining limitations in the evidence report.
Finish in review with required CI passing; do not merge or mark Done.
