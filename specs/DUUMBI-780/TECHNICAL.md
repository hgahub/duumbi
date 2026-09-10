# DUUMBI-780: Bounded HTTP/SQLite/JSON Process Evidence — Technical Specification

Related to #780. [Product specification](PRODUCT.md).
Authority: [implementation decision](https://github.com/hgahub/duumbi/issues/780#issuecomment-5620459440).

## Execution and layering

`bench::runner` and `determinism::runner` reserve an ephemeral loopback port once
per invocation when process showcases are selected. The benchmark shares its
verifier behind an async mutex across providers so process attempts are serial;
ordinary i64 tasks retain provider concurrency. Replay is already serial.
`AttemptRequest::process_verifier` borrows that verifier; per-attempt evidence
is cleared/taken without changing its port or accumulating earlier attempts.

`intent::external_verifier::ExternalVerifier` owns the object-safe contract:
`prepare_spec`, boxed Send future `verify -> TestReport`, `repairable`, and
`kind`. The boxed future preserves dynamic dispatch without adding async-trait.
`intent::execute` depends only on this trait. The process implementation remains
in `bench::process`; `intent::attempt` reads its concrete retained evidence.

After initialization, the intent clone receives the run's port before saving,
hashing, and provider mutation. Declared dependencies already in config are
preserved. Missing declarations are materialized from the highest valid SemVer
under `.duumbi/cache/<scope>/<module>@<version>/graph`, with one execution-log
note per added or unavailable dependency. The complete materialization routine
runs on `spawn_blocking`, keeping filesystem I/O off the async worker. No
registry request is made.

`run_preflight_for_intent_with_external_verifier` replaces only `E_NO_TEST_CASES`
with Info `I_EXTERNAL_VERIFICATION` naming the verifier. All other preflight and
BDD rules remain intact. Plain CLI/REPL execution supplies no waiver.

External verification replaces i64 tests inside the normal execute/repair loop.
Program failures can trigger one normal repair pass and re-verification;
infrastructure failures cannot. The same verifier is used after repair, and
`first_pass_success`, `repair_attempted`, and `repair_success` retain their
ordinary meaning. Authoring failure before verification reports `not_run`.

## Process contract and lifecycle

The YAML states the stdin SQL, two fresh SQLite datasets, JSON response, and
single-request exit contract from PRODUCT.md. `ProcessVerifier::prepare_spec`
adds the selected port as an acceptance criterion. No generated graph is patched
by the harness. The flagship is an API reference, not a passing solution.

| Phase | Implementation and bound |
| --- | --- |
| Build | Local `duumbi build --output .duumbi/build/output` subprocess, 30 seconds; initialized workspace/vendor/cache dependencies only |
| Binding analysis | `src/bench/process/bindings.rs` resolves listener arguments through local function calls, parameter loads, and string concatenation; requires `127.0.0.1` and the run port |
| Port handoff | Release initial reservation; bind/release the same port before **every** launch, including second dataset, repair, and subsequent attempts; occupied port after a 250 ms async handoff window is Start infrastructure failure |
| Start | Fresh generated child; write one SQL line and close stdin; connect within 5 seconds, polling early child exit |
| Request | The readiness connection is the actual GET socket, avoiding consumption of a one-request server; 2-second write deadline |
| Response | 2-second read deadline; maximum 16 KiB including headers; validate HTTP framing, status 200, JSON and exact values/types |
| Exit | Require exit code 0 within 2 seconds after response |
| Cleanup | Kill process tree and reap child, separate 2-second reap deadline; pipe collection bounded to 1 second each |

Unix uses a dedicated process group; Windows uses an owned Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Windows creates children with
`CREATE_SUSPENDED`, attaches the job, and only then resumes the initial thread;
[Windows documents that suspended creation prevents execution](https://learn.microsoft.com/en-us/windows/win32/procthread/suspending-thread-execution).
A read-only, bounded IPv4/IPv6 TCP table snapshot distinguishes stale TIME_WAIT
from active listeners before a Windows reuse-enabled bind probe. Error, timeout, normal completion and
cancellation clean up the owned tree. Output pipes drain continuously and retain
at most 8 KiB each. Generated children inherit only PATH and SystemRoot; build
children additionally inherit compiler configuration and use workspace-local
temporary directories. Provider credentials are excluded. Evidence is redacted
and control characters removed before retention or repair prompts.

## Evidence and failure taxonomy

Benchmark `evidence.process[]` and replay `benchmark_evidence.process[]` are
ordered `Vec<ProcessEvidence>` entries for initial and repaired verification.
Each records contract `duumbi.http-sqlite-json.v1`, build command/output,
scenarios with SQL/port/route/status/response/expected values/assertion verdict,
process exit/deadline/kill/reap facts, and the first failure (`stage`, `kind`,
`detail`). No successful process pass is fabricated when authoring fails.
`process-evidence.json` persists even on success, within existing #779 caps;
parent reports retain structured evidence when artifact retention is exhausted.
Old reports without the optional process vector remain readable.

`classify_process_failure` overrides taxonomy only for terminal verifier failures:

| Stage | Failure kind | Root cause | Dominant error code |
| --- | --- | --- | --- |
| Any stage (including occupied Start port or toolchain Build failure) | infrastructure | provider_or_infrastructure; excluded from graph failures | `process_<stage>` |
| setup | program (defensive fallback) | provider_or_infrastructure | `process_setup` |
| assertion | program | product_logic_mismatch | `process_assertion` |
| build, start, request, response, exit | program | compiler_or_runtime | `process_<stage>` |

Process timeouts are not provider timeouts. Initial failure remains recorded if
repair succeeds. Replay signatures append `;process=<status>[:<stage>]` and
context/intent hashing includes the prepared port. Graph hashes can vary across
separate invocations; within a run the execution inputs remain identical.

## BDD-to-test mapping

| Product scenario | Automated evidence |
| --- | --- |
| 1, 2 | `benchmark_authors_builds_and_retains_sqlite_process_evidence` (real native build, SQLite, HTTP, cache and preflight log assertions) |
| 3 | `process_mismatch_enters_normal_repair_and_reverifies`, `unrepaired_json_mismatch_remains_a_product_logic_failure` |
| 4 | `authoring_failure_is_preserved_and_process_is_not_run` |
| 5 | process unit tests for occupied ports, missing toolchain, deadlines and cleanup |
| 6 | binding-analysis tests for public/unresolved and forwarded listener arguments |
| 7 | `determinism_replay_runs_the_same_process_contract`, two Pass attempts with equal intent/semantic/exact hashes and process signatures |
| 8 | `process_showcase_reaches_mutation_only_with_its_external_verifier` |
| Dependency preparation | `declared_dependencies_are_materialized_from_cache_once`, SemVer selection coverage |

Portable fake children use the Rust test binary; normal CI makes no LLM calls.
Run format, clippy, the focused library suite, native integration, and full tests:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib -- bench:: intent::attempt intent::preflight intent::execute determinism::
cargo test --test integration_duumbi780_process_evidence
cargo test --all
```

PR CI must compile and execute Ubuntu and Windows matrix jobs, including Job
Object tests; source inspection alone is not Windows execution evidence.

## Ralph cycle and manual E2E plan

Work in bounded cycles within the modules above, plus the specified tests/docs.
No runtime/stdlib or unrelated refactor is authorized. Local offline checks cost
USD 0 in external LLM use. A live cycle uses the user-requested `gpt-5.6-luna`
with `OPENAI_API_KEY`; expected external cost must stay at or below USD 1 per
cycle unless explicitly authorized otherwise. Stop for unavailable quota/model,
unresolvable in-scope blockers, scope change, or the resource gate.

Launch the built CLI in a throwaway initialized workspace before creating the PR:

In the throwaway workspace, configure the explicit test model (the provider
filter selects an existing configuration; it does not create one):

```toml
[[providers]]
provider = "openai"
model = "gpt-5.6-luna"
role = "primary"
api_key_env = "OPENAI_API_KEY"
```

```sh
duumbi benchmark --showcase scaled_http_sqlite_json --attempts 1 \
  --provider openai:gpt-5.6-luna \
  --output /tmp/duumbi-780-live.json
```

Commit a sanitized local evidence record under `docs/e2e/results/`. Distinguish
fixture success from live authoring success, report actual failure stage, and
never infer a live pass from a successful CLI exit or offline test.
