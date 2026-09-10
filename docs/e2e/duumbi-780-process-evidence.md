# DUUMBI-780 bounded HTTP/SQLite/JSON verification

`scaled_http_sqlite_json` now invokes the normal intent mutation and repair path
in both `benchmark` and `determinism replay`. Its empty i64 test list is replaced
by one applicable process check. Other preflight and BDD errors still block
execution; an authoring failure retains its original classification.

## Service contract

The existing showcase YAML describes a one-request service. Before authoring,
the harness reserves a TCP port on `127.0.0.1` and adds it to the intent's
acceptance criteria. The graph is never rewritten by the verifier. The harness
releases the reservation immediately before launching the compiled program.
There is a short bind handoff window because the current runtime cannot inherit
a listener; contention produces a visible startup failure rather than a pass.

Each process reads one SQL INSERT statement from stdin, creates a fresh
in-memory SQLite database with `facts(name text not null)`, executes the INSERT,
and queries `SELECT name FROM facts ORDER BY rowid`. The same binary is launched
with these independent, deterministic datasets:

| Dataset | Inserted names, in row order | Expected count | Expected first_fact |
| --- | --- | --- | --- |
| one_row | Ada Lovelace | 1 | Ada Lovelace |
| two_rows | Grace Hopper; Katherine Johnson | 2 | Grace Hopper |

The verifier requests `GET /facts`, requires HTTP 200, parses JSON, and compares
all five public fields by value and type: `service = "scaled-http-sqlite-json"`,
`route = "/facts"`, `storage = "sqlite-memory"`, integer `count`, and string
`first_fact`. Additional JSON fields are allowed. A constant response cannot
pass both datasets. These are observable SQL-driven behavior checks, not a
formal proof of internal data provenance against an adversarial implementation.

A readiness connection becomes the actual HTTP request connection, so probing
does not consume the service's one-request allowance. Before launch, a bounded
static check resolves listener host/port constants through local function calls,
parameter loads, and string concatenation. It requires `127.0.0.1` and the
selected port. Public or unresolved computed bindings fail without launching
that program. This check allows arbitrary module layouts and graph IDs, but
conservatively rejects binding arguments it cannot resolve. The harness is not
an OS security sandbox for arbitrary hostile programs.

## Bounds and cleanup

- Build: 30 seconds, local `duumbi build`, workspace/vendor/cache resolution;
  no registry download or provider call in the verifier.
- Startup: 5 seconds, including connection attempts.
- Request write and response read: 2 seconds each.
- Response: at most 16 KiB including headers; Content-Length, when present,
  must match. The current stdlib's unencoded HTTP response contract is used.
- Exit: 2 seconds after the response; a successful service must exit with 0.
- Cleanup: kill and reap the child, with a separate 2-second reap deadline.
  Pipe collection has a 1-second bound per stream.
- Stdout/stderr: continuously drained to avoid pipe deadlocks, retaining at
  most 8 KiB per stream. Secret-bearing lines and configured credential values
  are redacted; control characters and a truncated final log line are removed.

Unix children run in dedicated process groups. Windows children belong to an
owned Job Object with kill-on-close semantics. Completion, errors, deadlines,
and cancellation terminate the owned process tree. Generated services inherit
only PATH and SystemRoot; build children additionally receive the required
compiler settings, with temporary build files confined to the attempt.

## Evidence and classification

Both report surfaces contain the same `BenchmarkEvidence.process` records.
`process-evidence.json` is retained even for passing runs without
`--keep-workspaces`, subject to the existing per-attempt and per-run retention
caps. The parent report keeps structured evidence if retention is exhausted.
Historical reports without the optional process field remain readable.

Records contain the build command, exit/reap/deadline facts, redacted output,
SQL inputs, selected port, requested route, response status/body, expected
values, assertions, and the first failed stage. Initial and repaired passes
are retained in order. Failure stages are `setup`, `build`, `start`, `request`,
`response`, `assertion`, and `exit`.

Program failures enter normal graph repair. Infrastructure failures such as a
missing compiler executable do not request graph repair. A process deadline
is a program failure, not a provider timeout. Final JSON mismatches are product
logic failures; build/process failures are compiler/runtime failures. A failure
before verification is reported as process `not_run`, without replacing the
original authoring error with `evidence_required`.

The injected port is an execution input and appears in the recorded intent and
process evidence. The port is reserved once per benchmark/replay run and shared across sequential
process attempts, including providers. Each launch first checks port availability;
a conflict is a Start infrastructure failure. Intent and graph hashes now remain
comparable within a run; separate invocations can still select different ports.
Replay behavior signatures append `;process=<status>[:<stage>]`.

## Offline checks and local evidence

```sh
cargo test --lib bench::process::tests
cargo test --test integration_duumbi780_process_evidence
```

Integration tests inject deterministic provider patches derived from the
flagship example. They exercise actual intent mutation, validation, native
compilation, SQLite execution, loopback HTTP, repair, and artifact retention.
The replay test exercises the same contract through the replay runner. Normal
CI makes no live LLM calls. Portable fake children use the Rust test executable,
without a shell or Python dependency. Unix additionally checks descendant
cleanup after cancellation.

To retain a manual native E2E run:

```sh
DUUMBI_780_EVIDENCE_DIR=docs/e2e/results/duumbi-780-process-20260910 \
  cargo test --test integration_duumbi780_process_evidence -- --nocapture
```

See [the original local evidence record](results/duumbi-780-process-20260910.md)
and [the completion evidence](results/duumbi-780-completion-20260910.md), including
two-attempt replay hashes and the requested live model attempt.
The scripted provider verifies the execution machinery; it does not measure a
live model's ability to author this service.
