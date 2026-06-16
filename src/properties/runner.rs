//! Property evidence runner surface for `duumbi check --properties`.
//!
//! This cycle wires contract discovery and evidence writing. Native invocation
//! is intentionally reported as unsupported until the execution harness is
//! implemented, so property runs do not create false pass/fail confidence.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};

use crate::contracts::{ContractClause, ContractSet, EffectClass};
use crate::graph::{FunctionInfo, SemanticGraph};

use super::evidence::{
    FunctionEvidence, FunctionEvidenceStatus, PropertyEvidence, PropertyEvidenceSettings,
    PropertyEvidenceSummary, UnsupportedEvidence,
};
use super::generator::{GeneratorSettings, generate_values};

const DEFAULT_MAX_ARRAY_LEN: usize = 8;
const DEFAULT_MAX_PRECONDITION_REJECTIONS: u32 = 256;
const NATIVE_HARNESS_MISSING: &str = "native_execution_harness_missing";

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

    let functions = discover_functions(graph, &options);
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
    options: &PropertyRunOptions,
) -> Vec<FunctionEvidence> {
    let mut functions: Vec<_> = graph
        .functions
        .iter()
        .filter(|function| !function.contracts.is_empty())
        .map(|function| unsupported_function(graph, function, options))
        .collect();
    functions.sort_by(|left, right| left.function_id.cmp(&right.function_id));
    functions
}

fn unsupported_function(
    graph: &SemanticGraph,
    function: &FunctionInfo,
    options: &PropertyRunOptions,
) -> FunctionEvidence {
    let unsupported = classify_unsupported(function, options);
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
) -> UnsupportedEvidence {
    match function.contracts.effect {
        EffectClass::Effectful => {
            return unsupported(
                "unsupported_effectful_function",
                "effectful functions need an approved effect model before property execution",
            );
        }
        EffectClass::ReadOnlyDeterministic | EffectClass::Pure | EffectClass::Unspecified => {}
    }

    if !function.contracts.invariants.is_empty() {
        return unsupported(
            "unsupported_invariant_execution_missing",
            "invariants are parsed and preserved but not executed by the v1 property runner",
        );
    }

    if function.contracts.postconditions.is_empty() {
        return unsupported(
            "unsupported_no_postconditions",
            "property execution needs at least one postcondition to check",
        );
    }

    let generator_settings = GeneratorSettings {
        seed: options.seed,
        cases: options.cases,
        max_array_len: options.max_array_len,
    };
    for param in &function.params {
        if let Err(err) = generate_values(&param.param_type, &generator_settings) {
            return unsupported(
                err.reason,
                format!(
                    "parameter '{}' of type {} cannot be generated in v1",
                    param.name, err.ty
                ),
            );
        }
    }

    unsupported(
        NATIVE_HARNESS_MISSING,
        "native generated-case execution is not wired yet; no cases were executed",
    )
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
            .filter(|function| function.status == FunctionEvidenceStatus::Passed)
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
    fn contract_functions_write_unsupported_harness_evidence() {
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
            NATIVE_HARNESS_MISSING
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
