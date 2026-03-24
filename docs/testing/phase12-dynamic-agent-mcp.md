# Phase 12: Dynamic Agent System & MCP — Test Protocol

**Version:** 1.0
**Date:** 2026-03-24
**Branch:** `phase12/dynamic-agent-mcp`
**Milestone:** #21 (45 issues, #430–#474)

---

## Prerequisites

- [ ] Rust toolchain installed (`rustup show` → stable)
- [ ] Project builds: `cargo build` (a repo gyokereben)
- [ ] Existing tests pass: `cargo test --all`
- [ ] No clippy warnings: `cargo clippy --all-targets -- -D warnings`

### Binary

```bash
export DUUMBI="$(pwd)/target/debug/duumbi"
```

---

## Test Summary

| Category | Automated | Manual | Total |
|----------|-----------|--------|-------|
| Task Analysis (B) | 38 | 0 | 38 |
| Cost Control (D) | 25 | 0 | 25 |
| Agent Templates (C) | 41 | 0 | 41 |
| Merge Engine (E) | 24 | 0 | 24 |
| MCP Server (A) | 28 | 3 | 31 |
| MCP Client (G) | 25 | 0 | 25 |
| Kill Criterion (F) | 14 | 0 | 14 |
| Error Codes | 4 | 0 | 4 |
| **Total** | **199** | **3** | **202** |

---

## T1 — Task Analysis Engine (`src/agents/analyzer.rs`)

**Automatikus tesztek:** `cargo test -p duumbi --lib agents::analyzer`

### T1.1 Complexity scoring

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 1 | `simple_zero_test_cases` | 0 test case → Simple | ✅ |
| 2 | `simple_one_test_case` | 1 test case → Simple | ✅ |
| 3 | `moderate_three_test_cases` | 3 test cases → Moderate | ✅ |
| 4 | `moderate_five_test_cases` | 5 test cases → Moderate | ✅ |
| 5 | `complex_six_test_cases` | 6 test cases → Complex | ✅ |

### T1.2 Scope scoring

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 6 | `zero_modules_is_single_scope` | 0 module → SingleModule | ✅ |
| 7 | `single_module_one_create` | 1 create → SingleModule | ✅ |
| 8 | `single_module_one_modify` | 1 modify → SingleModule | ✅ |
| 9 | `multi_module_two_creates` | 2 creates → MultiModule | ✅ |
| 10 | `multi_module_one_create_one_modify` | 1+1 → MultiModule | ✅ |

### T1.3 Task type scoring

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 11 | `task_type_fix_keyword_fix` | "fix" → Fix | ✅ |
| 12 | `task_type_fix_keyword_bug` | "bug" → Fix | ✅ |
| 13 | `task_type_fix_keyword_error` | "error" → Fix | ✅ |
| 14 | `task_type_refactor_keyword_refactor` | "refactor" → Refactor | ✅ |
| 15 | `task_type_refactor_keyword_rename` | "rename" → Refactor | ✅ |
| 16 | `task_type_test_keyword_test` | "test" → Test | ✅ |
| 17 | `task_type_test_keyword_verify` | "verify" → Test | ✅ |
| 18 | `task_type_create_via_modules` | modules.create → Create | ✅ |
| 19 | `task_type_modify_fallback` | default → Modify | ✅ |
| 20 | `task_type_fix_takes_priority_over_test` | "fix" > "test" | ✅ |
| 21 | `case_insensitive_keyword_matching` | "FIX" → Fix | ✅ |
| 22 | `reorganise_british_spelling_is_refactor` | "reorganise" → Refactor | ✅ |

### T1.4 Risk scoring

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 23 | `risk_low_create_only_no_main` | create only → Low | ✅ |
| 24 | `risk_medium_modifies_one_module` | 1 modify → Medium | ✅ |
| 25 | `risk_high_touches_main` | main in modify → High | ✅ |
| 26 | `risk_high_multi_module_with_modify` | multi + modify → High | ✅ |

### T1.5 Integrated profile (9 lookup rows)

| # | Teszt | Profil | Státusz |
|---|-------|--------|---------|
| 27 | `profile_simple_create_single_low` | 1fn+1create+single+low | ✅ |
| 28 | `profile_simple_modify_single_low` | 1fn+0create+single+low | ✅ |
| 29 | `profile_simple_test_any` | "test" keyword | ✅ |
| 30 | `profile_moderate_create_single` | 3fn+1create+single | ✅ |
| 31 | `profile_moderate_create_multi` | 3fn+2create+multi | ✅ |
| 32 | `profile_moderate_modify_medium_risk` | 3fn+modify+medium | ✅ |
| 33 | `profile_complex_multi` | 6fn+multi | ✅ |
| 34 | `profile_refactor_keyword` | "refactor" keyword | ✅ |
| 35 | `profile_fix_keyword` | "fix" keyword | ✅ |
| 36 | `empty_spec_returns_safe_default` | empty → single Coder | ✅ |
| 37 | `error_context_in_intent_triggers_fix` | error ctx → Fix | ✅ |

---

## T2 — Cost Control (`src/agents/cost.rs`)

**Automatikus tesztek:** `cargo test -p duumbi --lib agents::cost`

### T2.1 CostTracker

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 1 | `cost_tracker_check_budget_passes_when_under_limit` | OK under limit | ✅ |
| 2 | `cost_tracker_check_budget_fails_when_intent_exceeded` | Err on exceed | ✅ |
| 3 | `cost_tracker_check_budget_fails_when_session_exceeded` | Err on session | ✅ |
| 4 | `cost_tracker_reset_intent_clears_intent_counter` | Reset works | ✅ |
| 5 | `cost_tracker_reset_intent_allows_budget_check_to_pass_again` | Re-enables | ✅ |
| 6 | `cost_tracker_alert_threshold_triggered_at_80_pct` | Alert at 80% | ✅ |
| 7 | `cost_tracker_alert_threshold_not_triggered_below_80_pct` | No alert | ✅ |
| 8 | `cost_tracker_alert_threshold_not_reached_initially` | Clean start | ✅ |

### T2.2 CircuitBreaker

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 9 | `circuit_breaker_starts_closed` | Initial = Closed | ✅ |
| 10 | `circuit_breaker_closed_to_open_transition` | N fails → Open | ✅ |
| 11 | `circuit_breaker_open_blocks_spawn` | Open → false | ✅ |
| 12 | `circuit_breaker_reset_moves_open_to_half_open` | Reset → HalfOpen | ✅ |
| 13 | `circuit_breaker_half_open_success_closes_circuit` | Success → Closed | ✅ |
| 14 | `circuit_breaker_half_open_failure_reopens_circuit` | Fail → Open | ✅ |
| 15 | `circuit_breaker_success_resets_failure_count` | Success clears count | ✅ |
| 16 | `circuit_breaker_reset_noop_when_already_closed` | Idempotent | ✅ |

### T2.3 AgentRateLimiter

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 17 | `rate_limiter_acquire_succeeds_when_permits_available` | OK | ✅ |
| 18 | `rate_limiter_permits_exhausted_causes_timeout` | SpawnTimeout | ✅ |
| 19 | `rate_limiter_permit_released_on_drop` | Re-acquirable | ✅ |
| 20 | `rate_limiter_max_permits_reported_correctly` | Correct count | ✅ |

### T2.4 Config compatibility

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 21 | `cost_section_default_values` | Sensible defaults | ✅ |
| 22 | `cost_section_custom_values_parse` | Custom TOML works | ✅ |
| 23 | `cost_section_backward_compat_empty` | Empty section OK | ✅ |
| 24 | `duumbi_config_with_cost_section_parses` | Full config OK | ✅ |
| 25 | `duumbi_config_without_cost_section_parses` | Old config OK | ✅ |

---

## T3 — Agent Templates (`src/agents/template.rs`, `assembler.rs`, `agent_knowledge.rs`)

**Automatikus tesztek:** `cargo test -p duumbi --lib agents::template agents::assembler agents::agent_knowledge`

### T3.1 Templates

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 1 | `seed_templates_returns_five` | 5 seed templates | ✅ |
| 2 | `all_roles_covered` | Planner, Coder, Reviewer, Tester, Repair | ✅ |
| 3 | `coder_and_repair_have_mutation_tools` | 7 tools each | ✅ |
| 4 | `planner_reviewer_tester_have_no_tools` | 0 tools | ✅ |
| 5 | `serialization_roundtrip` | JSON ↔ struct | ✅ |
| 6 | `find_by_role_coder` | Returns Coder template | ✅ |
| 7 | `find_by_role_missing_returns_none` | None for unknown | ✅ |
| 8 | `load_falls_back_to_seeds_when_dir_absent` | Seed fallback | ✅ |
| 9 | `save_seeds_creates_files` | Disk persistence | ✅ |
| 10 | `load_disk_overrides_seed` | Disk takes precedence | ✅ |

### T3.2 Team Assembler (9-row lookup table)

| # | Teszt | Profil → Team | Státusz |
|---|-------|---------------|---------|
| 11 | `row1_simple_create_single_low` | → [Coder], Sequential | ✅ |
| 12 | `row2_simple_modify_single_low` | → [Coder], Sequential | ✅ |
| 13 | `row3_test_any_dimensions` | → [Tester], Sequential | ✅ |
| 14 | `row4_moderate_create_single` | → [Planner,Coder,Tester], Pipeline | ✅ |
| 15 | `row5_moderate_create_multi` | → [Planner,Coder,Tester], Parallel | ✅ |
| 16 | `row6_moderate_modify_medium_risk` | → [Planner,Coder,Reviewer,Tester], Pipeline | ✅ |
| 17 | `row6_moderate_modify_high_risk` | → same as above | ✅ |
| 18 | `row7_complex_multi` | → [Planner,Coder,Reviewer,Tester], Parallel | ✅ |
| 19 | `row8_refactor_any_dimensions` | → [Planner,Coder,Reviewer,Tester], Pipeline | ✅ |
| 20 | `row9_fix_fast_path` | → [Coder], Sequential | ✅ |
| 21 | `default_fallback_single_coder` | → [Coder], Sequential | ✅ |
| 22 | `sequential_teams_have_one_coder` | parallel_coders=1 | ✅ |

### T3.3 Agent Knowledge

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 23 | `strategy_new_defaults` | Zero counters, not deprecated | ✅ |
| 24 | `strategy_should_deprecate_high_fail_rate` | >70% @10+ → true | ✅ |
| 25 | `strategy_should_not_deprecate_low_fail_rate` | <70% → false | ✅ |
| 26 | `strategy_should_not_deprecate_below_min_attempts` | <10 attempts → false | ✅ |
| 27 | `strategy_should_deprecate_exactly_at_boundary` | 7/10 (boundary, no deprecate) → false | ✅ |
| 28 | `strategy_serialization_roundtrip` | JSON ↔ struct | ✅ |
| 29 | `failure_pattern_new_defaults` | Zero counters | ✅ |
| 30 | `failure_pattern_should_deprecate` | >70% @10+ → true | ✅ |
| 31 | `failure_pattern_should_not_deprecate_below_min` | <10 → false | ✅ |
| 32 | `failure_pattern_serialization_roundtrip` | JSON ↔ struct | ✅ |
| 33 | `save_and_load_strategy` | TempDir roundtrip | ✅ |
| 34 | `save_and_load_failure_pattern` | TempDir roundtrip | ✅ |
| 35 | `load_empty_when_dir_absent` | Empty vec | ✅ |
| 36 | `prune_deprecated_marks_high_fail_rate` | Sets deprecated=true | ✅ |
| 37 | `prune_deprecated_does_not_mark_low_fail_rate` | Leaves alone | ✅ |
| 38 | `prune_deprecated_does_not_re_mark_already_deprecated` | Idempotent | ✅ |
| 39 | `prune_deprecated_no_dir_returns_zero` | No crash | ✅ |
| 40 | `relevant_strategies_filters_by_template_and_not_deprecated` | Correct filter | ✅ |
| 41 | `relevant_strategies_empty_when_no_match` | Empty vec | ✅ |

---

## T4 — Merge Engine (`src/agents/merger.rs`, `rollback.rs`)

**Automatikus tesztek:** `cargo test -p duumbi --lib agents::merger agents::rollback`

### T4.1 Conflict detection

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 1 | `detect_conflicts_no_overlap_returns_empty` | No conflicts | ✅ |
| 2 | `detect_conflicts_same_module_returns_pair` | SameModule found | ✅ |
| 3 | `detect_conflicts_node_id_collision_returns_pair` | Collision found | ✅ |
| 4 | `detect_conflicts_three_independent_patches` | No conflicts | ✅ |

### T4.2 Node ID extraction

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 5 | `collect_node_ids_from_add_function` | Extracts @id | ✅ |
| 6 | `collect_node_ids_from_add_block` | Extracts @id | ✅ |
| 7 | `collect_node_ids_from_add_op` | Extracts @id | ✅ |
| 8 | `collect_node_ids_from_replace_block` | Extracts @ids | ✅ |
| 9 | `collect_node_ids_empty_ops` | Empty set | ✅ |
| 10 | `collect_node_ids_skips_modify_remove_setedge` | No false positives | ✅ |

### T4.3 Merge strategies

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 11 | `two_patches_different_modules_both_applied` | 2 applied, 0 rejected | ✅ |
| 12 | `same_module_patches_rejected_with_same_module_reason` | Both rejected | ✅ |
| 13 | `node_id_collision_both_patches_rejected` | Both rejected | ✅ |
| 14 | `single_patch_always_applied` | 1 applied | ✅ |
| 15 | `single_patch_empty_ops_applied` | Empty patch OK | ✅ |
| 16 | `empty_patches_list_returns_empty_result` | Empty result | ✅ |

### T4.4 Workspace snapshot rollback

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 17 | `capture_on_empty_directory_returns_empty_snapshot` | Empty map | ✅ |
| 18 | `capture_on_nonexistent_path_returns_empty` | No crash | ✅ |
| 19 | `capture_reads_jsonld_files` | Reads content | ✅ |
| 20 | `capture_ignores_non_jsonld_files` | Skips .txt etc. | ✅ |
| 21 | `capture_traverses_subdirectories` | Recursive | ✅ |
| 22 | `restore_roundtrip_preserves_file_contents` | Exact match | ✅ |
| 23 | `restore_overwrites_changed_files` | Overwrites | ✅ |
| 24 | `files_returns_captured_paths` | Accessor works | ✅ |

---

## T5 — MCP Server (`src/mcp/`)

**Automatikus tesztek:** `cargo test -p duumbi --lib mcp`

### T5.1 JSON-RPC server

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 1 | `handle_initialize_returns_server_info` | Server name+version | ✅ |
| 2 | `handle_tools_list_returns_all_tools` | 10 tools listed | ✅ |
| 3 | `all_tool_definitions_have_valid_input_schema` | Valid JSON Schema | ✅ |
| 4 | `notification_returns_none` | No response for notification | ✅ |
| 5 | `unknown_method_returns_method_not_found` | -32601 error | ✅ |
| 6 | `tools_call_missing_name_returns_error` | Error response | ✅ |
| 7 | `tools_call_missing_params_returns_error` | Error response | ✅ |
| 8 | `tools_call_unknown_tool_returns_method_not_found` | Error response | ✅ |

### T5.2 Graph tools

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 9 | `graph_query_by_type` | Filters by @type | ✅ |
| 10 | `graph_query_by_node_id` | Filters by @id | ✅ |
| 11 | `graph_validate_returns_result` | valid + diagnostics | ✅ |
| 12 | `graph_validate_invalid_json` | Error or valid=false | ✅ |
| 13 | `graph_mutate_missing_ops_field` | Error | ✅ |
| 14 | `graph_describe_returns_description_or_error` | description field | ✅ |
| 15 | `tools_call_graph_query_by_type` | E2E via server | ✅ |
| 16 | `tools_call_graph_validate_on_valid_graph` | E2E via server | ✅ |

### T5.3 Build/Deps/Intent tools

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 17 | `build_compile_returns_informative_error` | Stub message | ✅ |
| 18 | `build_run_returns_informative_error` | Stub message | ✅ |
| 19 | `deps_search_requires_query` | Error without query | ✅ |
| 20 | `deps_search_suggests_cli` | Points to `duumbi search` | ✅ |
| 21 | `deps_install_suggests_cli` | Points to `duumbi deps install` | ✅ |
| 22 | `deps_install_frozen_flag_in_message` | Mentions --frozen | ✅ |
| 23 | `intent_create_requires_description` | Error without desc | ✅ |
| 24 | `intent_create_suggests_cli` | Points to CLI | ✅ |
| 25 | `intent_execute_requires_name` | Error without name | ✅ |
| 26 | `intent_execute_suggests_cli` | Points to CLI | ✅ |

### T5.4 MCP Client

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 27 | `mcp_client_config_serde_roundtrip` | JSON ↔ struct | ✅ |
| 28 | `mcp_client_config_default_trusted_is_true` | Default=true | ✅ |
| 29 | `duumbi_config_parses_mcp_clients_section` | TOML parse | ✅ |
| 30 | `duumbi_config_without_mcp_clients_parses_fine` | Backward compat | ✅ |
| 31 | `new_with_empty_configs` | Empty manager OK | ✅ |
| 32 | `server_names_returns_configured_names` | Lists names | ✅ |
| 33 | `is_trusted_returns_true_when_server_is_trusted` | Trust check | ✅ |
| 34 | `is_trusted_returns_false_when_server_is_untrusted` | Untrust check | ✅ |
| 35 | `register_tools_then_all_tools_returns_them` | Discovery roundtrip | ✅ |
| 36 | `find_tool_finds_across_servers` | Cross-server lookup | ✅ |
| 37 | `tools_for_server_returns_only_that_servers_tools` | Scoped lookup | ✅ |

### T5.M — Manual MCP tesztek

| # | Teszt | Lépések | Státusz |
|---|-------|---------|---------|
| M1 | `duumbi mcp` elindul | `$DUUMBI mcp` → JSON-RPC initialize request stdin-re, válasz stdout-on | ☐ |
| M2 | `duumbi mcp --help` | Flag-ek: `--sse`, `--port` megjelennek | ☐ |
| M3 | Claude Desktop integráció | `duumbi mcp` konfigurálva Claude Desktop-ban → tools/list → 10 tool látható | ☐ |

---

## T6 — Kill Criterion Tests (`tests/integration_phase12.rs`)

**Automatikus tesztek:** `cargo test --test integration_phase12`

### KC-1: External MCP client graph tools

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 1 | `kill_criterion_1a_mcp_graph_query_by_type` | graph_query returns Function nodes | ✅ |
| 2 | `kill_criterion_1b_mcp_graph_query_by_node_id` | Exact node ID match | ✅ |
| 3 | `kill_criterion_1c_mcp_graph_validate` | valid + diagnostics fields | ✅ |
| 4 | `kill_criterion_1d_mcp_graph_mutate_missing_ops` | Error on missing ops | ✅ |
| 5 | `kill_criterion_1e_mcp_graph_mutate_empty_ops` | Success with 0 ops | ✅ |

### KC-2: Dynamic agent assembly

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 6 | `kill_criterion_2_dynamic_assembly_multi_module` | 2 creates+3 tests → Planner+Coder+Tester, Parallel, ≥2 coders | ✅ |

### KC-3: Agent knowledge persistence

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 7 | `kill_criterion_3_knowledge_persistence` | Strategy save → load roundtrip | ✅ |
| 8 | `kill_criterion_3b_multiple_strategies_persist` | Multiple templates, both survive | ✅ |

### KC-4: Parallel merge

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 9 | `kill_criterion_4a_parallel_merge_independent_modules` | 2 applied, 0 rejected | ✅ |
| 10 | `kill_criterion_4b_parallel_merge_same_module_conflict` | 0 applied, 2 rejected | ✅ |
| 11 | `kill_criterion_4c_single_patch_always_applied` | 1 applied | ✅ |

### KC-5: Budget control

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 12 | `kill_criterion_5a_budget_control_basic` | 5×500 < 10000 budget | ✅ |
| 13 | `kill_criterion_5b_budget_control_exceeded` | 10500 > 10000 → E040 | ✅ |
| 14 | `kill_criterion_5c_budget_reset_between_intents` | reset_intent() clears, session accumulates | ✅ |

---

## T7 — Error Codes (`src/errors.rs`)

**Automatikus tesztek:** `cargo test -p duumbi --lib errors`

| # | Teszt | Elvárt | Státusz |
|---|-------|--------|---------|
| 1 | `error_codes_are_unique` | All 41 codes unique | ✅ |
| 2 | `e040_e048_codes_serialize_correctly` | Format: 1 letter + 3 digits | ✅ |
| 3 | `e040_budget_exceeded_with_details` | JSONL with details | ✅ |
| 4 | `e042_merge_conflict_with_node` | JSONL with nodeId | ✅ |

---

## T8 — Full Regression

```bash
cd /Users/heizergabor/space/hgahub/duumbi-phase12

# Teljes build
cargo build

# Összes teszt
cargo test --all
# Elvárt: 1747 teszt zöld, 0 failed

# Clippy
cargo clippy --all-targets -- -D warnings
# Elvárt: 0 warning

# Format check
cargo fmt --check
# Elvárt: OK
```

| # | Ellenőrzés | Elvárt | Státusz |
|---|------------|--------|---------|
| 1 | `cargo build` | Sikerül, 0 warning | ✅ |
| 2 | `cargo test --all` | 1747 teszt zöld | ✅ |
| 3 | `cargo clippy --all-targets -- -D warnings` | 0 warning | ✅ |
| 4 | Korábbi fázisok tesztjei nem törtek el | integration_phase0–phase10 mind zöld | ✅ |

---

## New Error Codes Reference

| Code | Name | Description |
|------|------|-------------|
| E040 | BUDGET_EXCEEDED | LLM token budget exceeded |
| E041 | CIRCUIT_OPEN | Circuit breaker open |
| E042 | MERGE_CONFLICT | Irreconcilable merge conflicts |
| E043 | NODE_ID_COLLISION | Two patches create same @id |
| E044 | AGENT_TIMEOUT | Agent spawn queue timeout |
| E045 | TEMPLATE_NOT_FOUND | Agent template not found |
| E046 | MCP_TOOL_ERROR | MCP tool invocation failed |
| E047 | MCP_CLIENT_UNREACHABLE | External MCP server unreachable |
| E048 | MCP_CLIENT_TOOL_NOT_FOUND | External tool not found |

---

## New Config Sections

```toml
# Cost control for dynamic agent system
[cost]
budget-per-intent = 50000
budget-per-session = 200000
max-parallel-agents = 3
circuit-breaker-failures = 5
alert-threshold-pct = 80

# External MCP server connections
[mcp-clients]
figma = { url = "https://figma.mcp.example.com/sse", description = "Figma design data" }
github = { url = "https://github.mcp.example.com/sse", description = "GitHub repos" }
```

---

## New Files

```
src/agents/analyzer.rs          — Task analysis engine (4D scoring)
src/agents/assembler.rs         — Team assembler (9-row lookup)
src/agents/template.rs          — Agent templates + TemplateStore
src/agents/agent_knowledge.rs   — Strategy + FailurePattern persistence
src/agents/cost.rs              — CostTracker + CircuitBreaker + RateLimiter
src/agents/merger.rs            — MergeEngine + conflict detection
src/agents/rollback.rs          — WorkspaceSnapshot (capture/restore)
src/mcp/mod.rs                  — MCP module root
src/mcp/server.rs               — JSON-RPC 2.0 MCP server
src/mcp/tools/mod.rs            — Tool module exports
src/mcp/tools/graph.rs          — graph.query/mutate/validate/describe
src/mcp/tools/build.rs          — build.compile/run (stubs)
src/mcp/tools/deps.rs           — deps.search/install (stubs)
src/mcp/tools/intent.rs         — intent.create/execute (stubs)
src/mcp/client/mod.rs           — McpClientManager
src/mcp/client/config.rs        — McpClientConfig
tests/integration_phase12.rs    — 14 kill criterion tests
```
