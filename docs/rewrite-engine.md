# Semantic Rewrite Engine

DUUMBI's V1 Semantic Rewrite Engine is a deterministic graph-to-graph rewrite
substrate for small, reviewable transformations over JSON-LD semantic graphs.
It is provider-free and uses the existing parse -> build -> validate pipeline.

V1 is intentionally conservative. It is not an optimizer framework, proof
system, or autonomous agent loop.

## Rule Model

Each rule has stable metadata:

- `id`: stable kebab-case rule ID.
- `displayName`: human-readable name.
- `category`: `canonicalization`, `local-optimization`, or
  `structural-cleanup`.
- `safetyClass`: `structural-only`, `local-semantics-preserving`, or
  `experimental`.
- `preconditions`: local facts that must hold before a match is valid.
- `effectSummary`: what the rewrite changes.
- `defaultCost`: deterministic per-match cost estimate.
- `explanationTemplate`: human-readable safety explanation.
- `applyCapable`: false for experimental rules.

The backend owns rule discovery, matching, preview evidence, apply planning,
cost limits, patch construction, and candidate validation. CLI and MCP adapters
only load modules, ask the backend for evidence, save snapshots, and write
validated candidates when apply is explicitly requested.

## Built-In V1 Rules

Apply-capable rules:

- `i64-add-zero-right`: rewrites downstream uses of `Add(x, 0)` to `x`.
- `i64-add-zero-left`: rewrites downstream uses of `Add(0, x)` to `x`.
- `i64-mul-one-right`: rewrites downstream uses of `Mul(x, 1)` to `x`.

Preview-only rule:

- `experimental-fold-i64-const-add`: detects `Add(Const(a), Const(b))` and
  reports the folded value, but apply rejects it in V1.

The identity rules redirect supported downstream JSON-LD references such as
`duumbi:left`, `duumbi:right`, `duumbi:operand`, and `duumbi:condition`. They do
not delete or reorder block operations in V1.

## CLI

List rules:

```sh
duumbi rewrite list
duumbi rewrite list --json
```

Preview a rule without mutation:

```sh
duumbi rewrite preview --rule i64-add-zero-right
duumbi rewrite preview --module main --rule i64-add-zero-right --limit 5 --json
```

Apply one match:

```sh
duumbi rewrite apply --rule i64-add-zero-right --match '<match-id>' --yes
```

Apply all matches within a bound:

```sh
duumbi rewrite apply --rule i64-add-zero-right --all --max-matches 3 --yes
```

If `--module` is omitted, DUUMBI uses `.duumbi/graph/main.jsonld`. A bare module
name such as `main` resolves to `.duumbi/graph/main.jsonld`; a `.jsonld` path is
used as provided.

Interactive apply asks for confirmation unless `--yes` is supplied. Declining
confirmation does not create a snapshot.

## MCP Tools

MCP exposes the same backend behavior:

- `rewrite_list_rules`: read-only rule discovery.
- `rewrite_preview`: read-only preview for one rule and module.
- `rewrite_apply`: write-capable apply for one match or bounded all-matches
  request.

MCP apply has no interactive confirmation because the tool call itself is the
explicit write request. It still reruns matching, validates the candidate, saves
an undo snapshot, and writes only after validation succeeds.

## Evidence

Preview evidence includes:

- `status`
- `rule`
- `matches`
- `cost`
- `warnings`

Each match includes:

- `matchId`
- `ruleId`
- `module`
- `primaryNodeId`
- `touchedNodeIds`
- `operationSummary`
- `explanation`
- `cost`
- `validation`

Apply evidence includes:

- selected match IDs
- touched node IDs
- validation status
- patch operation count
- cost evidence
- snapshot path at the CLI/MCP adapter layer

Match IDs use this shape:

```text
<rule-id>:<module-name>:<primary-node-id>:<ordinal>
```

Apply never trusts a preview object from the caller. It reruns matching against
the current graph and rejects missing or changed match IDs as stale.

## Safety And Bounds

Default bounds:

- max preview matches: 100
- max single apply matches: 1
- max apply-all matches: 10
- max touched nodes per match: 25
- max patch operations per match: 25
- max explanation nodes: 20

List and preview do not write graph files, snapshots, config, credentials,
registry cache, intent files, or telemetry.

Apply sequence:

1. Load JSON-LD source.
2. Parse source.
3. Build `SemanticGraph`.
4. Validate current graph.
5. Rerun matching and preconditions.
6. Enforce safety class and cost bounds.
7. Apply the rewrite patch to a clone in memory.
8. Parse, build, and validate the candidate graph.
9. Save an undo snapshot of the original source.
10. Write the pretty JSON-LD candidate.

Snapshot failure prevents graph writes. Candidate validation failure prevents
snapshots and writes.

## Relationship To Existing Mutation Paths

The rewrite engine does not replace `GraphPatch`, `duumbi add`, intent
execution, or MCP `graph_mutate`.

V1 rewrite apply uses `GraphPatch` internally as a safe candidate transform
primitive, then validates the candidate before adapters write it. Existing
provider-backed mutation and intent workflows keep their current behavior.
Query mode remains read-only.

## Non-Goals

V1 does not provide:

- e-graph equality saturation;
- an `egg` or equivalence-graph dependency;
- formal proof generation;
- SMT or VCGen proof-carrying rewrites;
- autonomous self-evolution;
- live LLM rule selection or rule synthesis;
- broad compiler optimization passes.
