# DUUMBI-780: Bounded HTTP/SQLite/JSON Process Evidence — Product Specification

Related to #780; follows #689 (scaled corpus) and #688 (stdlib reference).
Authority: [accepted implementation decision](https://github.com/hgahub/duumbi/issues/780#issuecomment-5620459440).

## Problem and user outcome

The scaled HTTP/SQLite/JSON row previously returned `evidence_required` before
provider mutation. It measured a verifier gap, not authoring capability.
Benchmark and determinism replay must execute the normal intent mutation and
repair path, build the generated service, and judge its observable behavior.

## Contract

The generated program must bind `127.0.0.1` on the port supplied in its intent,
read one SQL line from stdin, create a fresh in-memory SQLite `facts(name)`
table, execute that input, and serve one `GET /facts` before exiting with 0.
It must guard fallible operations, including `ReadLine` / `read_line`, with
`ResultIsOk` before `ResultUnwrap` as required by the authoring validator.

Two independent launches receive different INSERT statements. Both must return
HTTP 200 and a JSON object with these exact values and types:

| Field | One row | Two rows |
| --- | --- | --- |
| service | `scaled-http-sqlite-json` | `scaled-http-sqlite-json` |
| route | `/facts` | `/facts` |
| count | `1` (number) | `2` (number) |
| first_fact | `Ada Lovelace` | `Grace Hopper` |
| storage | `sqlite-memory` | `sqlite-memory` |

The second input inserts Grace Hopper followed by Katherine Johnson. A constant
response cannot pass both datasets. No graph rewriting substitutes for authoring.

## BDD scenarios

1. **Given** the process showcase, **when** benchmark or replay runs, **then**
   provider mutation occurs, followed by bounded build and behavioral checks.
2. **Given** a correct service, **when** both datasets are supplied to fresh
   launches, **then** exact JSON values pass, children exit and are reaped,
   and durable process evidence remains after the temporary workspace is removed.
3. **Given** an incorrect service value, **when** initial verification fails,
   **then** normal LLM repair is attempted and the repaired graph is verified
   again; initial failure and repaired success remain in ordered evidence.
4. **Given** failed authoring, **when** verification cannot be reached, **then**
   process status is `not_run` and the original root cause is preserved.
5. **Given** a taken port or missing toolchain, **when** verification runs,
   **then** infrastructure attribution excludes the failure from graph totals
   and avoids unhelpful graph repair.
6. **Given** a public or unresolved listener binding, **when** pre-launch analysis
   runs, **then** the generated service is never launched.
7. **Given** two replay attempts with the deterministic passing fixture,
   **when** the run completes, **then** intent and semantic graph hashes agree
   and both behavior signatures end in `;process=passed`.
8. **Given** process YAML without an external verifier, **when** ordinary intent
   execute runs, **then** `E_NO_TEST_CASES` still blocks; only a supplied verifier
   replaces that error with informational `I_EXTERNAL_VERIFICATION`.

## Risks and trade-offs (accepted)

The stdin-SQL contract adds input handling, Result guards, and dynamic row
counting beyond the flagship example. This makes authoring harder and can lower
pass rates for reasons beyond HTTP/SQLite/JSON composition. That additional
burden is accepted to reject constant-output solutions behaviorally.
`examples/flagship-http-sqlite-json` is a reference for the stdlib API only,
not a passing solution: it hard-codes its fact and port and lacks the write
path's E034 guards. Offline fixtures adapt it explicitly.

Two datasets are not a formal provenance proof against adversarial programs.
Binding analysis conservatively rejects values it cannot resolve. The verifier
is not an OS sandbox. Socket inheritance is unavailable, so bind/release before
launch has an unavoidable handoff race; a detected occupied port is infrastructure.
A fresh invocation may select a different port and therefore different hashes;
comparisons within one run use the same input port.

## Scope and completion

In scope: benchmark/replay, isolated workspace preparation, intent verifier
integration, process lifecycle and evidence, deterministic tests, manual local
E2E evidence, and supporting documentation. Out of scope: runtime/stdlib changes
to force success, arbitrary distributed/browser testing, external service access,
public ports, deployments, and the broader authoring-ceiling report.

Complete when tasks 1–8 of the decision are implemented, all required local
checks pass, and PR CI is green on Ubuntu and Windows. CI uses offline provider
fixtures only. Live authoring evidence is separate: claim generated-service
success only after a live row has `evidence.status=passed`.
