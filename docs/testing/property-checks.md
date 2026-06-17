# Contract-Based Property Checks

DUUMBI supports a v1 contract vocabulary for function-level property evidence.
The vocabulary is shared with future formal verification work, but
`duumbi check --properties` is randomized testing evidence only. A passing
property run is not a proof.

## Contract Shape

Contracts live on `duumbi:Function` nodes under `duumbi:contracts`.

```json
{
  "duumbi:contracts": {
    "duumbi:effect": "pure",
    "duumbi:preconditions": [],
    "duumbi:postconditions": [
      {
        "duumbi:id": "result-nonnegative",
        "duumbi:expr": {
          "duumbi:op": ">=",
          "duumbi:left": { "duumbi:var": "result" },
          "duumbi:right": { "duumbi:const": 0 }
        }
      }
    ],
    "duumbi:invariants": []
  }
}
```

Supported v1 expression nodes include variable references, constants,
comparisons, boolean combinators, basic numeric arithmetic, and bounded length
for supported local values. `result` is valid only in postconditions.

## CLI

Run ordinary validation plus property checks:

```sh
duumbi check tests/fixtures/properties/passing_identity.jsonld --properties --seed 717 --cases 3 --property-output /tmp/duumbi-property.json
```

Without `--properties`, `duumbi check` keeps its existing parse and graph
validation behavior. With `--properties`, validation runs first. Property
execution is skipped if validation fails.

## Evidence

Property evidence is compact JSON with schema version
`duumbi.property_evidence.v1`. It records:

- command, graph input, start and finish timestamps;
- seed, case count, collection bound, and precondition rejection budget;
- discovered, checked, unsupported, and failed function counts;
- function id, effect, contract ids, generated/executed/rejected case counts;
- failure seed, case index, actual result, counterexample, and shrink status.

Unsupported functions are reported explicitly. They are not counted as passed.

## Current V1 Execution Limit

The current native runner executes pure non-main functions with `i64`
parameters and an `i64` return value. It generates a temporary wrapper `main`,
builds through the normal DUUMBI native pipeline, runs the binary, and evaluates
postconditions locally.

Current unsupported cases include:

- contracts on `main`, because the wrapper must not replace the target
  function;
- non-`i64` native execution signatures;
- functions without an explicit `pure` or `read_only_deterministic` effect;
- effectful functions and runtime resource types;
- invariants, which are parsed and preserved for future verifier work but not
  executed as loop proofs.

`stdlib/math.jsonld` includes the first Tier 1 contract slice on `abs`:
`abs-result-nonnegative`. The runnable CLI fixtures under
`tests/fixtures/properties/` prove the first pass, fail, shrink, rejection, and
unsupported evidence paths.
