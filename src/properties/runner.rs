//! Property evidence runner surface for `duumbi check --properties`.
//!
//! This runner wires contract discovery, evidence writing, and the first native
//! generated-case execution path for pure non-main `i64` functions.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde_json::json;

use crate::contracts::{ContractClause, ContractSet, EffectClass};
use crate::graph::{FunctionInfo, SemanticGraph};
use crate::telemetry::BuildOptions;
use crate::types::DuumbiType;
use crate::workspace::{self, BinaryRunOutput};

use super::evidence::{
    FailureEvidence, FunctionEvidence, FunctionEvidenceStatus, PropertyEvidence,
    PropertyEvidenceSettings, PropertyEvidenceSummary, UnsupportedEvidence,
};
use super::generator::{GeneratorSettings, generate_values};
use super::predicate::{PredicateContext, eval_predicate};
use super::shrink::shrink_candidates;
use super::value::PropertyValue;

const DEFAULT_MAX_ARRAY_LEN: usize = 8;
const DEFAULT_MAX_PRECONDITION_REJECTIONS: u32 = 256;

/// Options for one property run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRunOptions {
    /// Global deterministic seed.
    pub seed: u64,
    /// Requested case count.
    pub cases: u32,
    /// Maximum generated collection length.
    pub max_array_len: usize,
    /// Maximum rejected candidates per case before a function is unsupported.
    pub max_precondition_rejections: u32,
    /// Optional explicit evidence output path.
    pub output_path: Option<PathBuf>,
}

impl Default for PropertyRunOptions {
    fn default() -> Self {
        Self {
            seed: 0,
            cases: 64,
            max_array_len: DEFAULT_MAX_ARRAY_LEN,
            max_precondition_rejections: DEFAULT_MAX_PRECONDITION_REJECTIONS,
            output_path: None,
        }
    }
}

/// Result of a property evidence run.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRunReport {
    /// Evidence artifact path.
    pub evidence_path: PathBuf,
    /// Evidence document written to disk.
    pub evidence: PropertyEvidence,
}

/// Discovers contract-bearing functions and writes property evidence.
///
/// Until native invocation support lands, every discovered function is reported
/// as unsupported rather than pass/fail.
pub fn run_properties(
    graph: &SemanticGraph,
    graph_input: &Path,
    options: PropertyRunOptions,
) -> Result<PropertyRunReport> {
    let started_at = now_rfc3339();
    let settings = PropertyEvidenceSettings {
        seed: options.seed,
        cases: options.cases,
        max_array_len: options.max_array_len,
        max_precondition_rejections: options.max_precondition_rejections,
    };
    let mut evidence = PropertyEvidence::new(
        command_for(graph_input, &options),
        graph_input.display().to_string(),
        started_at,
        String::new(),
        settings,
    );

    let functions = discover_functions(graph, graph_input, &options);
    evidence.summary = summarize(&functions);
    evidence.functions = functions;
    evidence.finished_at = now_rfc3339();

    let evidence_path = options
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(graph_input));
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create property evidence directory '{}'",
                parent.display()
            )
        })?;
    }
    fs::write(&evidence_path, evidence.to_json_string()?).with_context(|| {
        format!(
            "Failed to write property evidence artifact '{}'",
            evidence_path.display()
        )
    })?;

    Ok(PropertyRunReport {
        evidence_path,
        evidence,
    })
}

fn discover_functions(
    graph: &SemanticGraph,
    graph_input: &Path,
    options: &PropertyRunOptions,
) -> Vec<FunctionEvidence> {
    let mut functions: Vec<_> = graph
        .functions
        .iter()
        .filter(|function| !function.contracts.is_empty())
        .map(|function| run_function(graph, graph_input, function, options))
        .collect();
    functions.sort_by(|left, right| left.function_id.cmp(&right.function_id));
    functions
}

fn run_function(
    graph: &SemanticGraph,
    graph_input: &Path,
    function: &FunctionInfo,
    options: &PropertyRunOptions,
) -> FunctionEvidence {
    if let Some(unsupported) = classify_unsupported(function, options) {
        return unsupported_function(graph, function, unsupported);
    }

    match execute_i64_function(graph, graph_input, function, options) {
        Ok(evidence) => evidence,
        Err(unsupported) => unsupported_function(graph, function, unsupported),
    }
}

fn unsupported_function(
    graph: &SemanticGraph,
    function: &FunctionInfo,
    unsupported: UnsupportedEvidence,
) -> FunctionEvidence {
    FunctionEvidence {
        function_id: function_id(graph, function),
        function_name: function.name.0.clone(),
        effect: effect_label(&function.contracts.effect).to_string(),
        contract_ids: contract_ids(&function.contracts),
        status: FunctionEvidenceStatus::Unsupported,
        cases_generated: 0,
        cases_executed: 0,
        cases_rejected: 0,
        postconditions_checked: 0,
        unsupported: Some(unsupported),
        failure: None,
    }
}

fn classify_unsupported(
    function: &FunctionInfo,
    options: &PropertyRunOptions,
) -> Option<UnsupportedEvidence> {
    match function.contracts.effect {
        EffectClass::Effectful => {
            return Some(unsupported(
                "unsupported_effectful_function",
                "effectful functions need an approved effect model before property execution",
            ));
        }
        EffectClass::ReadOnlyDeterministic | EffectClass::Pure | EffectClass::Unspecified => {}
    }

    if function.return_type != DuumbiType::I64 {
        return Some(unsupported(
            "unsupported_return_type",
            format!(
                "native property execution currently supports i64 returns, found {}",
                function.return_type
            ),
        ));
    }

    if !function.contracts.invariants.is_empty() {
        return Some(unsupported(
            "unsupported_invariant_execution_missing",
            "invariants are parsed and preserved but not executed by the v1 property runner",
        ));
    }

    if function.contracts.postconditions.is_empty() {
        return Some(unsupported(
            "unsupported_no_postconditions",
            "property execution needs at least one postcondition to check",
        ));
    }

    let generator_settings = GeneratorSettings {
        seed: options.seed,
        cases: options.cases,
        max_array_len: options.max_array_len,
    };
    for param in &function.params {
        if let Err(err) = generate_values(&param.param_type, &generator_settings) {
            return Some(unsupported(
                err.reason,
                format!(
                    "parameter '{}' of type {} cannot be generated in v1",
                    param.name, err.ty
                ),
            ));
        }
        if param.param_type != DuumbiType::I64 {
            return Some(unsupported(
                "unsupported_parameter_type",
                format!(
                    "native property execution currently supports i64 parameters, found {} for '{}'",
                    param.param_type, param.name
                ),
            ));
        }
    }

    if function.name.0 == "main" {
        return Some(unsupported(
            "unsupported_main_function_invocation",
            "property execution currently wraps non-main functions to avoid replacing the target function",
        ));
    }

    None
}

fn execute_i64_function(
    graph: &SemanticGraph,
    graph_input: &Path,
    function: &FunctionInfo,
    options: &PropertyRunOptions,
) -> std::result::Result<FunctionEvidence, UnsupportedEvidence> {
    let generator_settings = GeneratorSettings {
        seed: options.seed,
        cases: options.cases,
        max_array_len: options.max_array_len,
    };
    let param_values = function
        .params
        .iter()
        .map(|param| generate_values(&param.param_type, &generator_settings))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|err| {
            unsupported(
                err.reason,
                format!("type {} cannot be generated in v1", err.ty),
            )
        })?;

    let mut evidence = FunctionEvidence {
        function_id: function_id(graph, function),
        function_name: function.name.0.clone(),
        effect: effect_label(&function.contracts.effect).to_string(),
        contract_ids: contract_ids(&function.contracts),
        status: FunctionEvidenceStatus::Passed,
        cases_generated: options.cases,
        cases_executed: 0,
        cases_rejected: 0,
        postconditions_checked: 0,
        unsupported: None,
        failure: None,
    };

    for case_index in 0..options.cases {
        let inputs = case_inputs(&param_values, case_index);
        let precondition = evaluate_preconditions(function, &inputs).map_err(|detail| {
            unsupported(
                "precondition_evaluation_error",
                format!("failed to evaluate preconditions: {detail}"),
            )
        })?;
        if !precondition {
            evidence.cases_rejected += 1;
            if evidence.cases_rejected > options.max_precondition_rejections {
                return Err(unsupported(
                    "precondition_rejection_budget_exhausted",
                    "preconditions rejected more generated inputs than the configured budget",
                ));
            }
            continue;
        }

        let result = run_native_i64_case(graph, graph_input, function, &inputs)?;
        evidence.cases_executed += 1;

        for clause in &function.contracts.postconditions {
            evidence.postconditions_checked += 1;
            let passed =
                evaluate_postcondition(function, clause, &inputs, result).map_err(|detail| {
                    unsupported(
                        "postcondition_evaluation_error",
                        format!("failed to evaluate postcondition: {detail}"),
                    )
                })?;
            if !passed {
                let (shrunk_counterexample, shrink_status) =
                    shrink_failure(graph, graph_input, function, clause, &inputs);
                let shrunk_counterexample =
                    (shrink_status == "shrunk").then_some(shrunk_counterexample);
                evidence.status = FunctionEvidenceStatus::Failed;
                evidence.failure = Some(FailureEvidence {
                    seed: options.seed,
                    case_index,
                    contract_id: clause.id.clone().or_else(|| clause.label.clone()),
                    actual: format!("result={result}"),
                    counterexample: inputs.clone(),
                    shrunk_counterexample,
                    shrink_status,
                });
                return Ok(evidence);
            }
        }
    }

    if evidence.cases_executed == 0 {
        return Err(unsupported(
            "no_precondition_satisfying_cases",
            "no generated inputs satisfied the function preconditions",
        ));
    }

    Ok(evidence)
}

fn case_inputs(param_values: &[Vec<PropertyValue>], case_index: u32) -> Vec<PropertyValue> {
    param_values
        .iter()
        .map(|values| values[case_index as usize % values.len()].clone())
        .collect()
}

fn evaluate_preconditions(
    function: &FunctionInfo,
    inputs: &[PropertyValue],
) -> std::result::Result<bool, String> {
    let context = bind_inputs(function, inputs);
    for clause in &function.contracts.preconditions {
        if !eval_predicate(&clause.expr, &context).map_err(|err| err.detail)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn evaluate_postcondition(
    function: &FunctionInfo,
    clause: &ContractClause,
    inputs: &[PropertyValue],
    result: i64,
) -> std::result::Result<bool, String> {
    let context = bind_inputs(function, inputs).with_result(PropertyValue::I64(result));
    eval_predicate(&clause.expr, &context).map_err(|err| err.detail)
}

fn shrink_failure(
    graph: &SemanticGraph,
    graph_input: &Path,
    function: &FunctionInfo,
    failed_clause: &ContractClause,
    original: &[PropertyValue],
) -> (Vec<PropertyValue>, String) {
    let mut current = original.to_vec();
    let mut shrunk = false;

    loop {
        let mut improved = false;
        'candidate: for idx in 0..current.len() {
            for candidate in shrink_candidates(&current[idx]) {
                if candidate == current[idx] {
                    continue;
                }
                let mut trial = current.clone();
                trial[idx] = candidate;
                let Ok(true) = evaluate_preconditions(function, &trial) else {
                    continue;
                };
                let Ok(result) = run_native_i64_case(graph, graph_input, function, &trial) else {
                    continue;
                };
                if evaluate_postcondition(function, failed_clause, &trial, result) == Ok(false) {
                    current = trial;
                    improved = true;
                    shrunk = true;
                    break 'candidate;
                }
            }
        }
        if !improved {
            break;
        }
    }

    if shrunk {
        (current, "shrunk".to_string())
    } else {
        (current, "minimal".to_string())
    }
}

fn bind_inputs(function: &FunctionInfo, inputs: &[PropertyValue]) -> PredicateContext {
    function
        .params
        .iter()
        .zip(inputs)
        .fold(PredicateContext::default(), |context, (param, value)| {
            context.with_binding(param.name.clone(), value.clone())
        })
}

fn run_native_i64_case(
    graph: &SemanticGraph,
    graph_input: &Path,
    function: &FunctionInfo,
    inputs: &[PropertyValue],
) -> std::result::Result<i64, UnsupportedEvidence> {
    let workspace = tempfile::Builder::new()
        .prefix("duumbi_property_")
        .tempdir()
        .map_err(|source| {
            unsupported(
                "native_workspace_create_failed",
                format!("failed to create temp property workspace: {source}"),
            )
        })?;
    run_native_i64_case_inner(workspace.path(), graph, graph_input, function, inputs)
}

fn run_native_i64_case_inner(
    workspace_root: &Path,
    graph: &SemanticGraph,
    graph_input: &Path,
    function: &FunctionInfo,
    inputs: &[PropertyValue],
) -> std::result::Result<i64, UnsupportedEvidence> {
    let graph_dir = workspace_root.join(".duumbi").join("graph");
    fs::create_dir_all(&graph_dir).map_err(|source| {
        unsupported(
            "native_workspace_create_failed",
            format!("failed to create '{}': {source}", graph_dir.display()),
        )
    })?;

    let wrapped = wrapped_source_module(graph, graph_input, function, inputs)?;
    let wrapped_source = serde_json::to_string_pretty(&wrapped).map_err(|source| {
        unsupported(
            "native_wrapper_serialize_failed",
            format!("failed to serialize wrapper module: {source}"),
        )
    })?;
    fs::write(graph_dir.join("main.jsonld"), wrapped_source).map_err(|source| {
        unsupported(
            "native_wrapper_write_failed",
            format!("failed to write wrapper module: {source}"),
        )
    })?;

    let output_path = workspace::workspace_output_path(workspace_root);
    workspace::build_workspace_with_options(
        workspace_root,
        &output_path,
        BuildOptions::offline(true),
    )
    .map_err(|source| {
        unsupported(
            "native_build_failed",
            format!("failed to build generated property wrapper: {source}"),
        )
    })?;
    let output = workspace::run_workspace_binary(workspace_root, &[]).map_err(|source| {
        unsupported(
            "native_run_failed",
            format!("failed to run generated property wrapper: {source}"),
        )
    })?;
    parse_i64_stdout(&output)
}

fn wrapped_source_module(
    graph: &SemanticGraph,
    graph_input: &Path,
    function: &FunctionInfo,
    inputs: &[PropertyValue],
) -> std::result::Result<serde_json::Value, UnsupportedEvidence> {
    let source = fs::read_to_string(graph_input).map_err(|source| {
        unsupported(
            "native_source_read_failed",
            format!(
                "failed to read source graph '{}': {source}",
                graph_input.display()
            ),
        )
    })?;
    let mut module: serde_json::Value = serde_json::from_str(&source).map_err(|source| {
        unsupported(
            "native_source_parse_failed",
            format!(
                "failed to parse source graph '{}': {source}",
                graph_input.display()
            ),
        )
    })?;
    let wrapper = wrapper_main_function(graph, function, inputs)?;
    let functions = module
        .get_mut("duumbi:functions")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| {
            unsupported(
                "native_wrapper_shape_invalid",
                "source module has no duumbi:functions array",
            )
        })?;
    if let Some(idx) = functions.iter().position(|item| {
        item.get("duumbi:name").and_then(serde_json::Value::as_str) == Some("main")
    }) {
        functions[idx] = wrapper;
    } else {
        functions.push(wrapper);
    }
    Ok(module)
}

fn wrapper_main_function(
    graph: &SemanticGraph,
    function: &FunctionInfo,
    inputs: &[PropertyValue],
) -> std::result::Result<serde_json::Value, UnsupportedEvidence> {
    let id_module = graph.module_name.0.trim_start_matches("duumbi:");
    let mut ops = Vec::new();
    let mut arg_ids = Vec::new();
    for (idx, input) in inputs.iter().enumerate() {
        let PropertyValue::I64(value) = input else {
            return Err(unsupported(
                "unsupported_runtime_value",
                format!("native i64 harness cannot execute {}", input.type_label()),
            ));
        };
        let id = format!("duumbi:{id_module}/main/entry/arg{idx}");
        ops.push(json!({
            "@type": "duumbi:Const",
            "@id": id,
            "duumbi:value": value,
            "duumbi:resultType": "i64",
        }));
        arg_ids.push(json!({ "@id": id }));
    }
    let call_id = format!("duumbi:{id_module}/main/entry/call");
    ops.push(json!({
        "@type": "duumbi:Call",
        "@id": call_id,
        "duumbi:function": function.name.0,
        "duumbi:args": arg_ids,
        "duumbi:resultType": function.return_type.to_string(),
    }));
    let print_id = format!("duumbi:{id_module}/main/entry/print");
    ops.push(json!({
        "@type": "duumbi:Print",
        "@id": print_id,
        "duumbi:operand": { "@id": call_id },
    }));
    let return_id = format!("duumbi:{id_module}/main/entry/return");
    ops.push(json!({
        "@type": "duumbi:Return",
        "@id": return_id,
        "duumbi:operand": { "@id": call_id },
    }));
    Ok(json!({
        "@type": "duumbi:Function",
        "@id": format!("duumbi:{id_module}/main"),
        "duumbi:name": "main",
        "duumbi:returnType": function.return_type.to_string(),
        "duumbi:blocks": [{
            "@type": "duumbi:Block",
            "@id": format!("duumbi:{id_module}/main/entry"),
            "duumbi:label": "entry",
            "duumbi:ops": ops,
        }],
    }))
}

fn parse_i64_stdout(output: &BinaryRunOutput) -> std::result::Result<i64, UnsupportedEvidence> {
    let last_line = output.stdout.lines().last().unwrap_or("").trim();
    last_line.parse::<i64>().map_err(|source| {
        unsupported(
            "native_output_parse_failed",
            format!(
                "failed to parse property wrapper stdout '{last_line}' as i64: {source}; stderr: {}",
                output.stderr.trim()
            ),
        )
    })
}

fn unsupported(reason: impl Into<String>, detail: impl Into<String>) -> UnsupportedEvidence {
    UnsupportedEvidence {
        reason: reason.into(),
        detail: detail.into(),
    }
}

fn summarize(functions: &[FunctionEvidence]) -> PropertyEvidenceSummary {
    PropertyEvidenceSummary {
        functions_discovered: functions.len() as u32,
        functions_checked: functions
            .iter()
            .filter(|function| {
                matches!(
                    function.status,
                    FunctionEvidenceStatus::Passed | FunctionEvidenceStatus::Failed
                )
            })
            .count() as u32,
        functions_unsupported: functions
            .iter()
            .filter(|function| function.status == FunctionEvidenceStatus::Unsupported)
            .count() as u32,
        properties_failed: functions
            .iter()
            .filter(|function| function.status == FunctionEvidenceStatus::Failed)
            .count() as u32,
    }
}

fn contract_ids(contracts: &ContractSet) -> Vec<String> {
    let mut ids = Vec::new();
    append_clause_ids(&mut ids, "precondition", &contracts.preconditions);
    append_clause_ids(&mut ids, "postcondition", &contracts.postconditions);
    append_clause_ids(&mut ids, "invariant", &contracts.invariants);
    ids
}

fn append_clause_ids(ids: &mut Vec<String>, kind: &str, clauses: &[ContractClause]) {
    for (idx, clause) in clauses.iter().enumerate() {
        if let Some(id) = &clause.id {
            ids.push(id.clone());
        } else if let Some(label) = &clause.label {
            ids.push(label.clone());
        } else {
            ids.push(format!("{kind}-{idx}"));
        }
    }
}

fn function_id(graph: &SemanticGraph, function: &FunctionInfo) -> String {
    let module = graph.module_name.0.trim_start_matches("duumbi:");
    format!("duumbi:{module}/{}", function.name.0)
}

fn effect_label(effect: &EffectClass) -> &'static str {
    match effect {
        EffectClass::Unspecified => "unspecified",
        EffectClass::Pure => "pure",
        EffectClass::ReadOnlyDeterministic => "read_only_deterministic",
        EffectClass::Effectful => "effectful",
    }
}

fn command_for(graph_input: &Path, options: &PropertyRunOptions) -> String {
    let mut command = format!(
        "duumbi check {} --properties --seed {} --cases {}",
        graph_input.display(),
        options.seed,
        options.cases
    );
    if let Some(output) = &options.output_path {
        command.push_str(&format!(" --property-output {}", output.display()));
    }
    command
}

fn default_output_path(graph_input: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let stem = graph_input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("graph");
    std::env::temp_dir().join(format!("duumbi-property-{stem}-{timestamp}.json"))
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::graph::builder::build_graph;
    use crate::parser::parse_jsonld;

    fn graph_from_contracts(contracts: &str, params: &str) -> SemanticGraph {
        let source = format!(
            r#"{{
                "@context": {{"duumbi": "https://duumbi.dev/schema#"}},
                "@type": "duumbi:Module",
                "@id": "duumbi:t",
                "duumbi:name": "t",
                "duumbi:functions": [{{
                    "@type": "duumbi:Function",
                    "@id": "duumbi:t/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "i64",
                    "duumbi:params": [{params}],
                    "duumbi:contracts": {contracts},
                    "duumbi:blocks": [{{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:t/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": [
                            {{
                                "@type": "duumbi:Const",
                                "@id": "duumbi:t/main/entry/c0",
                                "duumbi:value": 0,
                                "duumbi:resultType": "i64"
                            }},
                            {{
                                "@type": "duumbi:Return",
                                "@id": "duumbi:t/main/entry/ret",
                                "duumbi:operand": {{"@id": "duumbi:t/main/entry/c0"}}
                            }}
                        ]
                    }}]
                }}]
            }}"#
        );
        let module = parse_jsonld(&source).expect("contract graph should parse");
        build_graph(&module).expect("contract graph should build")
    }

    #[test]
    fn main_contract_functions_write_unsupported_invocation_evidence() {
        let graph = graph_from_contracts(
            r#"{
                "duumbi:effect": "pure",
                "duumbi:postconditions": [{
                    "duumbi:id": "result-nonnegative",
                    "duumbi:expr": {
                        "duumbi:op": ">=",
                        "duumbi:left": {"duumbi:var": "result"},
                        "duumbi:right": {"duumbi:const": 0}
                    }
                }]
            }"#,
            r#"{"duumbi:name": "n", "duumbi:paramType": "i64"}"#,
        );
        let output = tempfile::NamedTempFile::new()
            .expect("temp file")
            .path()
            .to_path_buf();
        let report = run_properties(
            &graph,
            Path::new("tests/fixtures/properties/passing_abs.jsonld"),
            PropertyRunOptions {
                seed: 717,
                cases: 32,
                output_path: Some(output.clone()),
                ..Default::default()
            },
        )
        .expect("property evidence should write");

        assert_eq!(report.evidence_path, output);
        assert_eq!(report.evidence.summary.functions_discovered, 1);
        assert_eq!(report.evidence.summary.functions_unsupported, 1);
        let function = &report.evidence.functions[0];
        assert_eq!(function.function_id, "duumbi:t/main");
        assert_eq!(function.contract_ids, vec!["result-nonnegative"]);
        assert_eq!(
            function.unsupported.as_ref().expect("unsupported").reason,
            "unsupported_main_function_invocation"
        );
        let written = fs::read_to_string(&report.evidence_path).expect("evidence written");
        assert!(written.contains("\"schema_version\":\"duumbi.property_evidence.v1\""));
    }

    #[test]
    fn resource_parameters_report_generator_reason_before_harness() {
        let graph = graph_from_contracts(
            r#"{
                "duumbi:effect": "pure",
                "duumbi:postconditions": [{
                    "duumbi:id": "ok",
                    "duumbi:expr": {"duumbi:const": true}
                }]
            }"#,
            r#"{"duumbi:name": "db", "duumbi:paramType": "db_connection"}"#,
        );
        let output = tempfile::NamedTempFile::new()
            .expect("temp file")
            .path()
            .to_path_buf();
        let report = run_properties(
            &graph,
            Path::new("resource.jsonld"),
            PropertyRunOptions {
                output_path: Some(output),
                ..Default::default()
            },
        )
        .expect("property evidence should write");

        assert_eq!(
            report.evidence.functions[0]
                .unsupported
                .as_ref()
                .expect("unsupported")
                .reason,
            "unsupported_resource_db_connection"
        );
    }
}
