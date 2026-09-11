# DUUMBI-800 — Retire native Windows support

Issue: https://github.com/hgahub/duumbi/issues/800

## Decision and authorization

Native Windows maintenance and validation consume resources that the current
product does not justify. Focus native development on Linux and macOS. Windows
may be reconsidered when the product matures; no return date or WSL support is
promised. The user explicitly requested planning and implementation on
2026-09-11. This document records that request; it does not claim a separate
historical Stage 7 or Stage 9 approval.

## Acceptance scenarios

1. Before removal, a developer can check out the immutable annotated tag
   `windows-support-final-2026-09-10`, pointing to commit
   `52931c91d9fc6f7a541e1c250e0837a9980dd46b`. This is a source archive, not a
   new binary release. Never move or overwrite the tag.
2. A code PR runs the Ubuntu checks, coverage and dependency audit. No Windows
   runner or setup is scheduled. The required `check` aggregate still fails
   when its prerequisite fails and succeeds for valid documentation-only PRs.
3. On Linux and macOS, generated programs still compile and run, including
   HTTP/SQLite/JSON process verification, timeouts, cancellation and occupied
   ports. Existing assertions and path security boundaries remain enforced.
4. Native Windows is explicitly unsupported in current documentation and fails
   with a clear build diagnostic. DUUMBI-owned Windows execution paths and
   direct dependencies are removed. Upstream vendored code and transitive
   dependency metadata remain intact.

Historical evidence remains historical. Release architectures, provider/model
behavior and unrelated application features are outside this change.
