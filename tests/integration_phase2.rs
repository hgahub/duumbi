//! Phase 2 AI benchmark integration tests.
//!
//! Validates the 20-command benchmark fixture: each test case applies a
//! pre-crafted `GraphPatch` (simulating a correct LLM response) to the
//! skeleton module and verifies the result passes the full
//! parse → build → validate pipeline.
//!
//! These tests are deterministic (no live API calls) and serve as the
//! Phase 2 kill criterion baseline: all 20 must pass for a ≥ 70% AI accuracy
//! target to be meaningful.
//!
//! # Scoring
//!
//! In production, `duumbi add` replaces the mock patches below with real
//! LLM responses. A separate manual scoring pass records pass/fail per
//! command; accuracy = passed / 20.

use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// The minimal skeleton module produced by `duumbi init`.
///
/// Represents `add(3, 5)` → prints 8 → returns 8.
fn skeleton_module() -> Value {
    json!({
        "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:main/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:main/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [
                    {
                        "@type": "duumbi:Const",
                        "@id": "duumbi:main/main/entry/0",
                        "duumbi:value": 3,
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Const",
                        "@id": "duumbi:main/main/entry/1",
                        "duumbi:value": 5,
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Add",
                        "@id": "duumbi:main/main/entry/2",
                        "duumbi:left": { "@id": "duumbi:main/main/entry/0" },
                        "duumbi:right": { "@id": "duumbi:main/main/entry/1" },
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Print",
                        "@id": "duumbi:main/main/entry/3",
                        "duumbi:operand": { "@id": "duumbi:main/main/entry/2" }
                    },
                    {
                        "@type": "duumbi:Return",
                        "@id": "duumbi:main/main/entry/4",
                        "duumbi:operand": { "@id": "duumbi:main/main/entry/2" }
                    }
                ]
            }]
        }]
    })
}

/// Apply a `GraphPatch` and validate the result through the full pipeline.
///
/// Returns `Ok(patched_value)` or `Err(description)`.
fn apply_and_validate(
    source: &Value,
    patch_ops: Vec<duumbi::patch::PatchOp>,
) -> Result<Value, String> {
    use duumbi::patch::{GraphPatch, apply_patch};

    let patch = GraphPatch { ops: patch_ops };
    let patched = apply_patch(source, &patch).map_err(|e| e.to_string())?;

    let json_str = serde_json::to_string(&patched).map_err(|e| e.to_string())?;
    duumbi::parser::parse_jsonld(&json_str).map_err(|e| e.to_string())?;

    Ok(patched)
}

// ---------------------------------------------------------------------------
// Benchmark test cases
// ---------------------------------------------------------------------------

/// CMD-01: Change the constant value 3 → 7
#[test]
fn bench_cmd01_modify_const_value() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::ModifyOp {
            node_id: "duumbi:main/main/entry/0".to_string(),
            field: "duumbi:value".to_string(),
            value: json!(7),
        }],
    );
    assert!(result.is_ok(), "CMD-01 failed: {:?}", result.err());
    let patched = result.unwrap();
    assert_eq!(
        patched["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][0]["duumbi:value"],
        7
    );
}

/// CMD-02: Change the second constant from 5 → 10
#[test]
fn bench_cmd02_modify_second_const() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::ModifyOp {
            node_id: "duumbi:main/main/entry/1".to_string(),
            field: "duumbi:value".to_string(),
            value: json!(10),
        }],
    );
    assert!(result.is_ok(), "CMD-02 failed: {:?}", result.err());
}

/// CMD-03: Change Add → Sub (subtract instead of add)
#[test]
fn bench_cmd03_change_add_to_sub() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::ModifyOp {
            node_id: "duumbi:main/main/entry/2".to_string(),
            field: "@type".to_string(),
            value: json!("duumbi:Sub"),
        }],
    );
    assert!(result.is_ok(), "CMD-03 failed: {:?}", result.err());
}

/// CMD-04: Change Add → Mul
#[test]
fn bench_cmd04_change_add_to_mul() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::ModifyOp {
            node_id: "duumbi:main/main/entry/2".to_string(),
            field: "@type".to_string(),
            value: json!("duumbi:Mul"),
        }],
    );
    assert!(result.is_ok(), "CMD-04 failed: {:?}", result.err());
}

/// CMD-05: Add a new Const op to the entry block
#[test]
fn bench_cmd05_add_const_op() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::AddOp {
            block_id: "duumbi:main/main/entry".to_string(),
            op: json!({
                "@type": "duumbi:Const",
                "@id": "duumbi:main/main/entry/5",
                "duumbi:value": 42,
                "duumbi:resultType": "i64"
            }),
        }],
    );
    assert!(result.is_ok(), "CMD-05 failed: {:?}", result.err());
}

/// CMD-06: Add a helper function (no params, returns i64)
#[test]
fn bench_cmd06_add_helper_function() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::AddFunction {
            function: json!({
                "@type": "duumbi:Function",
                "@id": "duumbi:main/helper",
                "duumbi:name": "helper",
                "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:main/helper/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [{
                        "@type": "duumbi:Const",
                        "@id": "duumbi:main/helper/entry/0",
                        "duumbi:value": 0,
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Return",
                        "@id": "duumbi:main/helper/entry/1",
                        "duumbi:operand": { "@id": "duumbi:main/helper/entry/0" }
                    }]
                }]
            }),
        }],
    );
    assert!(result.is_ok(), "CMD-06 failed: {:?}", result.err());
}

/// CMD-07: Add a second block to the main function
#[test]
fn bench_cmd07_add_block_to_function() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::AddBlock {
            function_id: "duumbi:main/main".to_string(),
            block: json!({
                "@type": "duumbi:Block",
                "@id": "duumbi:main/main/exit",
                "duumbi:label": "exit",
                "duumbi:ops": [{
                    "@type": "duumbi:Const",
                    "@id": "duumbi:main/main/exit/0",
                    "duumbi:value": 0,
                    "duumbi:resultType": "i64"
                },
                {
                    "@type": "duumbi:Return",
                    "@id": "duumbi:main/main/exit/1",
                    "duumbi:operand": { "@id": "duumbi:main/main/exit/0" }
                }]
            }),
        }],
    );
    assert!(result.is_ok(), "CMD-07 failed: {:?}", result.err());
}

/// CMD-08: Set the left operand of Add to reference the second Const
#[test]
fn bench_cmd08_set_edge_operand() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::SetEdge {
            node_id: "duumbi:main/main/entry/2".to_string(),
            field: "duumbi:left".to_string(),
            target_id: "duumbi:main/main/entry/1".to_string(),
        }],
    );
    assert!(result.is_ok(), "CMD-08 failed: {:?}", result.err());
}

/// CMD-09: Set the right operand of Add to reference the first Const
#[test]
fn bench_cmd09_set_edge_right_operand() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::SetEdge {
            node_id: "duumbi:main/main/entry/2".to_string(),
            field: "duumbi:right".to_string(),
            target_id: "duumbi:main/main/entry/0".to_string(),
        }],
    );
    assert!(result.is_ok(), "CMD-09 failed: {:?}", result.err());
}

/// CMD-10: Rename the main function
#[test]
fn bench_cmd10_rename_function() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::ModifyOp {
            node_id: "duumbi:main/main".to_string(),
            field: "duumbi:name".to_string(),
            value: json!("program"),
        }],
    );
    assert!(result.is_ok(), "CMD-10 failed: {:?}", result.err());
}

/// CMD-11: Change result type of the Add op
#[test]
fn bench_cmd11_change_result_type() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    // Change Add resultType — still valid i64 arithmetic
    let result = apply_and_validate(
        &source,
        vec![PatchOp::ModifyOp {
            node_id: "duumbi:main/main/entry/2".to_string(),
            field: "duumbi:resultType".to_string(),
            value: json!("i64"),
        }],
    );
    assert!(result.is_ok(), "CMD-11 failed: {:?}", result.err());
}

/// CMD-12: Add a Div op referencing existing consts
#[test]
fn bench_cmd12_add_div_op() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::AddOp {
            block_id: "duumbi:main/main/entry".to_string(),
            op: json!({
                "@type": "duumbi:Div",
                "@id": "duumbi:main/main/entry/5",
                "duumbi:left": { "@id": "duumbi:main/main/entry/0" },
                "duumbi:right": { "@id": "duumbi:main/main/entry/1" },
                "duumbi:resultType": "i64"
            }),
        }],
    );
    assert!(result.is_ok(), "CMD-12 failed: {:?}", result.err());
}

/// CMD-13: Change return type of main function to void
#[test]
fn bench_cmd13_change_return_type() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::ModifyOp {
            node_id: "duumbi:main/main".to_string(),
            field: "duumbi:returnType".to_string(),
            value: json!("void"),
        }],
    );
    assert!(result.is_ok(), "CMD-13 failed: {:?}", result.err());
}

/// CMD-14: Add a Sub op between two consts
#[test]
fn bench_cmd14_add_sub_op() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::AddOp {
            block_id: "duumbi:main/main/entry".to_string(),
            op: json!({
                "@type": "duumbi:Sub",
                "@id": "duumbi:main/main/entry/5",
                "duumbi:left": { "@id": "duumbi:main/main/entry/1" },
                "duumbi:right": { "@id": "duumbi:main/main/entry/0" },
                "duumbi:resultType": "i64"
            }),
        }],
    );
    assert!(result.is_ok(), "CMD-14 failed: {:?}", result.err());
}

/// CMD-15: Add a Mul op between two consts
#[test]
fn bench_cmd15_add_mul_op() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::AddOp {
            block_id: "duumbi:main/main/entry".to_string(),
            op: json!({
                "@type": "duumbi:Mul",
                "@id": "duumbi:main/main/entry/5",
                "duumbi:left": { "@id": "duumbi:main/main/entry/0" },
                "duumbi:right": { "@id": "duumbi:main/main/entry/1" },
                "duumbi:resultType": "i64"
            }),
        }],
    );
    assert!(result.is_ok(), "CMD-15 failed: {:?}", result.err());
}

/// CMD-16: Modify a constant to a large value
#[test]
fn bench_cmd16_large_constant() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::ModifyOp {
            node_id: "duumbi:main/main/entry/0".to_string(),
            field: "duumbi:value".to_string(),
            value: json!(1_000_000_i64),
        }],
    );
    assert!(result.is_ok(), "CMD-16 failed: {:?}", result.err());
}

/// CMD-17: Add a second function and a no-op const in it
#[test]
fn bench_cmd17_add_second_function_with_const() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![PatchOp::AddFunction {
            function: json!({
                "@type": "duumbi:Function",
                "@id": "duumbi:main/compute",
                "duumbi:name": "compute",
                "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:main/compute/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:main/compute/entry/0",
                            "duumbi:value": 100,
                            "duumbi:resultType": "i64"
                        },
                        {
                            "@type": "duumbi:Return",
                            "@id": "duumbi:main/compute/entry/1",
                            "duumbi:operand": { "@id": "duumbi:main/compute/entry/0" }
                        }
                    ]
                }]
            }),
        }],
    );
    assert!(result.is_ok(), "CMD-17 failed: {:?}", result.err());
}

/// CMD-18: Multi-op patch: add a new const AND update the Add's left operand
#[test]
fn bench_cmd18_multi_op_add_const_and_set_edge() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![
            PatchOp::AddOp {
                block_id: "duumbi:main/main/entry".to_string(),
                op: json!({
                    "@type": "duumbi:Const",
                    "@id": "duumbi:main/main/entry/5",
                    "duumbi:value": 99,
                    "duumbi:resultType": "i64"
                }),
            },
            PatchOp::SetEdge {
                node_id: "duumbi:main/main/entry/2".to_string(),
                field: "duumbi:left".to_string(),
                target_id: "duumbi:main/main/entry/5".to_string(),
            },
        ],
    );
    assert!(result.is_ok(), "CMD-18 failed: {:?}", result.err());
}

/// CMD-19: Multi-op patch: modify both const values
#[test]
fn bench_cmd19_modify_both_consts() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![
            PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/0".to_string(),
                field: "duumbi:value".to_string(),
                value: json!(20),
            },
            PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/1".to_string(),
                field: "duumbi:value".to_string(),
                value: json!(22),
            },
        ],
    );
    assert!(result.is_ok(), "CMD-19 failed: {:?}", result.err());
    let patched = result.unwrap();
    assert_eq!(
        patched["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][0]["duumbi:value"],
        20
    );
    assert_eq!(
        patched["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][1]["duumbi:value"],
        22
    );
}

/// CMD-20: Multi-op: add function + add block + add op (fully nested)
#[test]
fn bench_cmd20_add_function_block_op_sequence() {
    use duumbi::patch::PatchOp;
    let source = skeleton_module();
    let result = apply_and_validate(
        &source,
        vec![
            // Step 1: add a new function (minimal skeleton, will be extended)
            PatchOp::AddFunction {
                function: json!({
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/util",
                    "duumbi:name": "util",
                    "duumbi:returnType": "i64",
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:main/util/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [{
                            "@type": "duumbi:Const",
                            "@id": "duumbi:main/util/entry/0",
                            "duumbi:value": 1,
                            "duumbi:resultType": "i64"
                        },
                        {
                            "@type": "duumbi:Return",
                            "@id": "duumbi:main/util/entry/1",
                            "duumbi:operand": { "@id": "duumbi:main/util/entry/0" }
                        }]
                    }]
                }),
            },
            // Step 2: add a second block to the new function
            PatchOp::AddBlock {
                function_id: "duumbi:main/util".to_string(),
                block: json!({
                    "@type": "duumbi:Block",
                    "@id": "duumbi:main/util/alt",
                    "duumbi:label": "alt",
                    "duumbi:ops": [{
                        "@type": "duumbi:Const",
                        "@id": "duumbi:main/util/alt/0",
                        "duumbi:value": 0,
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Return",
                        "@id": "duumbi:main/util/alt/1",
                        "duumbi:operand": { "@id": "duumbi:main/util/alt/0" }
                    }]
                }),
            },
            // Step 3: add an op to the second block
            PatchOp::AddOp {
                block_id: "duumbi:main/util/alt".to_string(),
                op: json!({
                    "@type": "duumbi:Const",
                    "@id": "duumbi:main/util/alt/2",
                    "duumbi:value": 999,
                    "duumbi:resultType": "i64"
                }),
            },
        ],
    );
    assert!(result.is_ok(), "CMD-20 failed: {:?}", result.err());
}

// ---------------------------------------------------------------------------
// Benchmark scoring helper
// ---------------------------------------------------------------------------

/// Runs all 20 benchmark cases programmatically and prints an accuracy report.
///
/// This is not a `#[test]` — call it explicitly with `cargo test -- --nocapture bench_score`
/// to see the human-readable report.
#[test]
fn bench_score_report() {
    use duumbi::patch::PatchOp;

    struct Case {
        id: &'static str,
        description: &'static str,
        ops: Vec<PatchOp>,
    }

    let source = skeleton_module();

    let cases: Vec<Case> = vec![
        Case {
            id: "CMD-01",
            description: "Change constant 3 → 7",
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/0".into(),
                field: "duumbi:value".into(),
                value: json!(7),
            }],
        },
        Case {
            id: "CMD-02",
            description: "Change constant 5 → 10",
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/1".into(),
                field: "duumbi:value".into(),
                value: json!(10),
            }],
        },
        Case {
            id: "CMD-03",
            description: "Change Add → Sub",
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/2".into(),
                field: "@type".into(),
                value: json!("duumbi:Sub"),
            }],
        },
        Case {
            id: "CMD-04",
            description: "Change Add → Mul",
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/2".into(),
                field: "@type".into(),
                value: json!("duumbi:Mul"),
            }],
        },
        Case {
            id: "CMD-05",
            description: "Add a new Const op",
            ops: vec![PatchOp::AddOp {
                block_id: "duumbi:main/main/entry".into(),
                op: json!({
                    "@type": "duumbi:Const",
                    "@id": "duumbi:main/main/entry/5",
                    "duumbi:value": 42,
                    "duumbi:resultType": "i64"
                }),
            }],
        },
        Case {
            id: "CMD-06",
            description: "Add a helper function",
            ops: vec![PatchOp::AddFunction {
                function: json!({
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/helper",
                    "duumbi:name": "helper",
                    "duumbi:returnType": "i64",
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:main/helper/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            { "@type": "duumbi:Const", "@id": "duumbi:main/helper/entry/0",
                              "duumbi:value": 0, "duumbi:resultType": "i64" },
                            { "@type": "duumbi:Return", "@id": "duumbi:main/helper/entry/1",
                              "duumbi:operand": { "@id": "duumbi:main/helper/entry/0" } }
                        ]
                    }]
                }),
            }],
        },
        Case {
            id: "CMD-07",
            description: "Add a second block",
            ops: vec![PatchOp::AddBlock {
                function_id: "duumbi:main/main".into(),
                block: json!({
                    "@type": "duumbi:Block",
                    "@id": "duumbi:main/main/exit",
                    "duumbi:label": "exit",
                    "duumbi:ops": [
                        { "@type": "duumbi:Const", "@id": "duumbi:main/main/exit/0",
                          "duumbi:value": 0, "duumbi:resultType": "i64" },
                        { "@type": "duumbi:Return", "@id": "duumbi:main/main/exit/1",
                          "duumbi:operand": { "@id": "duumbi:main/main/exit/0" } }
                    ]
                }),
            }],
        },
        Case {
            id: "CMD-08",
            description: "Set left operand of Add",
            ops: vec![PatchOp::SetEdge {
                node_id: "duumbi:main/main/entry/2".into(),
                field: "duumbi:left".into(),
                target_id: "duumbi:main/main/entry/1".into(),
            }],
        },
        Case {
            id: "CMD-09",
            description: "Set right operand of Add",
            ops: vec![PatchOp::SetEdge {
                node_id: "duumbi:main/main/entry/2".into(),
                field: "duumbi:right".into(),
                target_id: "duumbi:main/main/entry/0".into(),
            }],
        },
        Case {
            id: "CMD-10",
            description: "Rename main function",
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main".into(),
                field: "duumbi:name".into(),
                value: json!("program"),
            }],
        },
        Case {
            id: "CMD-11",
            description: "Re-set result type of Add (no-op, already i64)",
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/2".into(),
                field: "duumbi:resultType".into(),
                value: json!("i64"),
            }],
        },
        Case {
            id: "CMD-12",
            description: "Add a Div op",
            ops: vec![PatchOp::AddOp {
                block_id: "duumbi:main/main/entry".into(),
                op: json!({
                    "@type": "duumbi:Div",
                    "@id": "duumbi:main/main/entry/5",
                    "duumbi:left": { "@id": "duumbi:main/main/entry/0" },
                    "duumbi:right": { "@id": "duumbi:main/main/entry/1" },
                    "duumbi:resultType": "i64"
                }),
            }],
        },
        Case {
            id: "CMD-13",
            description: "Change return type to void",
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main".into(),
                field: "duumbi:returnType".into(),
                value: json!("void"),
            }],
        },
        Case {
            id: "CMD-14",
            description: "Add a Sub op",
            ops: vec![PatchOp::AddOp {
                block_id: "duumbi:main/main/entry".into(),
                op: json!({
                    "@type": "duumbi:Sub",
                    "@id": "duumbi:main/main/entry/5",
                    "duumbi:left": { "@id": "duumbi:main/main/entry/1" },
                    "duumbi:right": { "@id": "duumbi:main/main/entry/0" },
                    "duumbi:resultType": "i64"
                }),
            }],
        },
        Case {
            id: "CMD-15",
            description: "Add a Mul op",
            ops: vec![PatchOp::AddOp {
                block_id: "duumbi:main/main/entry".into(),
                op: json!({
                    "@type": "duumbi:Mul",
                    "@id": "duumbi:main/main/entry/5",
                    "duumbi:left": { "@id": "duumbi:main/main/entry/0" },
                    "duumbi:right": { "@id": "duumbi:main/main/entry/1" },
                    "duumbi:resultType": "i64"
                }),
            }],
        },
        Case {
            id: "CMD-16",
            description: "Set constant to large value",
            ops: vec![PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/0".into(),
                field: "duumbi:value".into(),
                value: json!(1_000_000_i64),
            }],
        },
        Case {
            id: "CMD-17",
            description: "Add second function with const",
            ops: vec![PatchOp::AddFunction {
                function: json!({
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/compute",
                    "duumbi:name": "compute",
                    "duumbi:returnType": "i64",
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:main/compute/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            { "@type": "duumbi:Const", "@id": "duumbi:main/compute/entry/0",
                              "duumbi:value": 100, "duumbi:resultType": "i64" },
                            { "@type": "duumbi:Return", "@id": "duumbi:main/compute/entry/1",
                              "duumbi:operand": { "@id": "duumbi:main/compute/entry/0" } }
                        ]
                    }]
                }),
            }],
        },
        Case {
            id: "CMD-18",
            description: "Multi-op: add const + set edge",
            ops: vec![
                PatchOp::AddOp {
                    block_id: "duumbi:main/main/entry".into(),
                    op: json!({
                        "@type": "duumbi:Const",
                        "@id": "duumbi:main/main/entry/5",
                        "duumbi:value": 99,
                        "duumbi:resultType": "i64"
                    }),
                },
                PatchOp::SetEdge {
                    node_id: "duumbi:main/main/entry/2".into(),
                    field: "duumbi:left".into(),
                    target_id: "duumbi:main/main/entry/5".into(),
                },
            ],
        },
        Case {
            id: "CMD-19",
            description: "Multi-op: modify both consts",
            ops: vec![
                PatchOp::ModifyOp {
                    node_id: "duumbi:main/main/entry/0".into(),
                    field: "duumbi:value".into(),
                    value: json!(20),
                },
                PatchOp::ModifyOp {
                    node_id: "duumbi:main/main/entry/1".into(),
                    field: "duumbi:value".into(),
                    value: json!(22),
                },
            ],
        },
        Case {
            id: "CMD-20",
            description: "Multi-op: add function + block + op sequence",
            ops: vec![
                PatchOp::AddFunction {
                    function: json!({
                        "@type": "duumbi:Function",
                        "@id": "duumbi:main/util",
                        "duumbi:name": "util",
                        "duumbi:returnType": "i64",
                        "duumbi:blocks": [{
                            "@type": "duumbi:Block",
                            "@id": "duumbi:main/util/entry",
                            "duumbi:label": "entry",
                            "duumbi:ops": [
                                { "@type": "duumbi:Const", "@id": "duumbi:main/util/entry/0",
                                  "duumbi:value": 1, "duumbi:resultType": "i64" },
                                { "@type": "duumbi:Return", "@id": "duumbi:main/util/entry/1",
                                  "duumbi:operand": { "@id": "duumbi:main/util/entry/0" } }
                            ]
                        }]
                    }),
                },
                PatchOp::AddBlock {
                    function_id: "duumbi:main/util".into(),
                    block: json!({
                        "@type": "duumbi:Block",
                        "@id": "duumbi:main/util/alt",
                        "duumbi:label": "alt",
                        "duumbi:ops": [
                            { "@type": "duumbi:Const", "@id": "duumbi:main/util/alt/0",
                              "duumbi:value": 0, "duumbi:resultType": "i64" },
                            { "@type": "duumbi:Return", "@id": "duumbi:main/util/alt/1",
                              "duumbi:operand": { "@id": "duumbi:main/util/alt/0" } }
                        ]
                    }),
                },
                PatchOp::AddOp {
                    block_id: "duumbi:main/util/alt".into(),
                    op: json!({
                        "@type": "duumbi:Const",
                        "@id": "duumbi:main/util/alt/2",
                        "duumbi:value": 999,
                        "duumbi:resultType": "i64"
                    }),
                },
            ],
        },
    ];

    let total = cases.len();
    let mut passed = 0usize;
    let mut report = Vec::new();

    for case in &cases {
        let res = apply_and_validate(&source, case.ops.clone());
        let ok = res.is_ok();
        if ok {
            passed += 1;
        }
        let status = if ok { "PASS" } else { "FAIL" };
        let detail = res.err().unwrap_or_default();
        report.push(format!(
            "  [{status}] {id}: {desc}{err}",
            id = case.id,
            desc = case.description,
            err = if detail.is_empty() {
                String::new()
            } else {
                format!(" — {detail}")
            }
        ));
    }

    let accuracy = (passed as f64 / total as f64) * 100.0;
    println!("\n=== Phase 2 Benchmark (mock patches) ===");
    for line in &report {
        println!("{line}");
    }
    println!("\nScore: {passed}/{total} ({accuracy:.0}%) — Phase 2 kill criterion: ≥70% (14/20)");

    assert!(
        passed == total,
        "Mock benchmark must be 20/20 — {passed}/{total} passed. Fix the mock patches."
    );
}
