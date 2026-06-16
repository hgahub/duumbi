//! Schema-versioned property evidence records.

use serde::Serialize;

use super::value::PropertyValue;

/// Property evidence schema version.
pub const PROPERTY_EVIDENCE_SCHEMA_VERSION: &str = "duumbi.property_evidence.v1";

/// Complete property run evidence document.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PropertyEvidence {
    /// Evidence schema version.
    pub schema_version: &'static str,
    /// Command line that produced this evidence.
    pub command: String,
    /// Graph input path or identifier.
    pub graph_input: String,
    /// UTC start timestamp as RFC3339 text.
    pub started_at: String,
    /// UTC finish timestamp as RFC3339 text.
    pub finished_at: String,
    /// Runner settings.
    pub settings: PropertyEvidenceSettings,
    /// Run summary.
    pub summary: PropertyEvidenceSummary,
    /// Per-function records.
    pub functions: Vec<FunctionEvidence>,
}

impl PropertyEvidence {
    /// Creates an empty evidence document.
    #[must_use]
    pub fn new(
        command: impl Into<String>,
        graph_input: impl Into<String>,
        started_at: impl Into<String>,
        finished_at: impl Into<String>,
        settings: PropertyEvidenceSettings,
    ) -> Self {
        Self {
            schema_version: PROPERTY_EVIDENCE_SCHEMA_VERSION,
            command: command.into(),
            graph_input: graph_input.into(),
            started_at: started_at.into(),
            finished_at: finished_at.into(),
            settings,
            summary: PropertyEvidenceSummary::default(),
            functions: Vec::new(),
        }
    }

    /// Serializes this evidence document as compact JSON.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Settings recorded with property evidence.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PropertyEvidenceSettings {
    /// Global seed.
    pub seed: u64,
    /// Requested case count.
    pub cases: u32,
    /// Maximum generated collection length.
    pub max_array_len: usize,
    /// Maximum rejected candidates per case.
    pub max_precondition_rejections: u32,
}

/// Summary counts for one property run.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct PropertyEvidenceSummary {
    /// Functions with contracts discovered.
    pub functions_discovered: u32,
    /// Functions checked by property execution.
    pub functions_checked: u32,
    /// Functions skipped as unsupported.
    pub functions_unsupported: u32,
    /// Failed postconditions.
    pub properties_failed: u32,
}

/// Evidence for one function.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FunctionEvidence {
    /// Function id from JSON-LD.
    pub function_id: String,
    /// Function name.
    pub function_name: String,
    /// Effect class text.
    pub effect: String,
    /// Contract ids associated with this function.
    pub contract_ids: Vec<String>,
    /// Generation/execution status.
    pub status: FunctionEvidenceStatus,
    /// Number of cases generated.
    pub cases_generated: u32,
    /// Number of cases executed.
    pub cases_executed: u32,
    /// Number of candidate inputs rejected by preconditions.
    pub cases_rejected: u32,
    /// Number of postcondition checks run.
    pub postconditions_checked: u32,
    /// Unsupported evidence, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported: Option<UnsupportedEvidence>,
    /// Failure evidence, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<FailureEvidence>,
}

/// Function-level property status.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FunctionEvidenceStatus {
    /// Function passed all supported property cases.
    Passed,
    /// Function failed at least one property case.
    Failed,
    /// Function has contracts but cannot be checked in v1.
    Unsupported,
}

/// Unsupported reason evidence.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UnsupportedEvidence {
    /// Stable unsupported reason.
    pub reason: String,
    /// Human-readable detail.
    pub detail: String,
}

/// Failing property evidence.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FailureEvidence {
    /// Seed that reproduced the failure.
    pub seed: u64,
    /// Zero-based case index.
    pub case_index: u32,
    /// Failed contract id, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
    /// Actual result or failure detail.
    pub actual: String,
    /// Original counterexample.
    pub counterexample: Vec<PropertyValue>,
    /// Shrunk counterexample when shrink succeeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shrunk_counterexample: Option<Vec<PropertyValue>>,
    /// Shrink status such as `minimal`, `shrunk`, or `not_attempted`.
    pub shrink_status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_serializes_schema_version_and_summary() {
        let evidence = PropertyEvidence::new(
            "duumbi check --properties --seed 717",
            "tests/fixtures/properties/passing_abs.jsonld",
            "2026-06-16T00:00:00Z",
            "2026-06-16T00:00:01Z",
            PropertyEvidenceSettings {
                seed: 717,
                cases: 32,
                max_array_len: 8,
                max_precondition_rejections: 256,
            },
        );

        let json = evidence
            .to_json_string()
            .expect("evidence should serialize");
        assert!(json.contains("\"schema_version\":\"duumbi.property_evidence.v1\""));
        assert!(json.contains("\"functions_discovered\":0"));
        assert!(!json.contains("api_key"));
    }

    #[test]
    fn failure_evidence_serializes_counterexample() {
        let function = FunctionEvidence {
            function_id: "duumbi:t/main".to_string(),
            function_name: "main".to_string(),
            effect: "pure".to_string(),
            contract_ids: vec!["result-nonnegative".to_string()],
            status: FunctionEvidenceStatus::Failed,
            cases_generated: 1,
            cases_executed: 1,
            cases_rejected: 0,
            postconditions_checked: 1,
            unsupported: None,
            failure: Some(FailureEvidence {
                seed: 717,
                case_index: 0,
                contract_id: Some("result-nonnegative".to_string()),
                actual: "result=-1".to_string(),
                counterexample: vec![PropertyValue::I64(-1)],
                shrunk_counterexample: Some(vec![PropertyValue::I64(-1)]),
                shrink_status: "minimal".to_string(),
            }),
        };

        let json = serde_json::to_string(&function).expect("function evidence should serialize");
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"contract_id\":\"result-nonnegative\""));
        assert!(json.contains("\"shrink_status\":\"minimal\""));
    }
}
