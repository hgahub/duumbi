# DUUMBI-738 Native Loop Stage 10 Evidence

Related to #738.

## Scope

This evidence covers the first DUUMBI-native Loop workflow slice:

- provider-core vocabulary in `duumbi::loop_native`
- provider-duumbi intake+spec path over local DUUMBI workspace context
- graph, BDD, knowledge, registry, session, and snapshot source indexing
- GraphPatch review target mapping without applying the patch
- CLI entrypoints under `duumbi loop`

GitHub and GitLab remain optional adapters. The native MVP does not require Git provider credentials, issues, pull requests, merge requests, or merge commits.

## Local E2E

Focused verification:

```text
cargo test loop_native
cargo test --test integration_duumbi738_loop_native
```

The integration fixture creates a local DUUMBI workspace with:

- `.duumbi/intents/native-loop.yaml`
- `.duumbi/intents/native-loop/features/native-loop.feature`
- `.duumbi/graph/main.jsonld`
- `.duumbi/deps.lock`
- `.duumbi/session/current.json`
- `.duumbi/knowledge/*`

The CLI E2E path runs:

```text
duumbi loop intake-spec native-loop --json
```

Expected native artifacts:

```text
.duumbi/loop/runs/duumbi-native-native-loop/artifacts/intake.json
.duumbi/loop/runs/duumbi-native-native-loop/artifacts/intake.md
.duumbi/loop/runs/duumbi-native-native-loop/artifacts/product_spec.md
.duumbi/loop/runs/duumbi-native-native-loop/artifacts/technical_spec.md
.duumbi/loop/runs/duumbi-native-native-loop/artifacts/metadata.json
```

## BDD-To-Test Mapping

| Product scenario | Verification |
| --- | --- |
| Native intake/spec runs without Git provider credentials | `native_intake_spec_writes_provider_free_artifacts_from_local_context`, `native_loop_cli_emits_json_result_without_git_provider_setup` |
| Graph-aware registry context is included | `native_intake_spec_writes_provider_free_artifacts_from_local_context` |
| BDD readiness blocks before side effects | `missing_explicit_bdd_reference_blocks_before_writing_artifacts` |
| GraphPatch can be reviewed as a native target | `graph_patch_review_target_maps_affected_nodes_without_applying_patch` |
| Git providers remain optional in native CLI | `loop_cli_help_keeps_git_providers_optional` |

## Ralph Cycle Resource Policy

- Ralph cycles created: 0
- External LLM calls by implementation path: 0
- Expected external LLM cost: USD 0
- Git provider API calls required by native workflow: 0
- GitHub/GitLab credentials required by native workflow: no
- Greptile used: no

## Deferred Work

- Loop homepage and full web dashboard surfaces remain outside this core DUUMBI-native slice.
- GitHub/GitLab adapters remain optional follow-on providers.
- Billing/subscription usage dashboards require product and infra ownership decisions before implementation.
- Full issue/project closure automation is intentionally not part of this implementation PR.
