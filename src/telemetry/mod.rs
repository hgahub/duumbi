//! Local telemetry configuration and artifact path helpers.
//!
//! Phase 13 telemetry is opt-in. The defaults here keep normal builds and runs
//! uninstrumented while giving traced runs a deterministic local artifact
//! location when later build and runtime cycles wire the feature through.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::graph::program::Program;
use crate::graph::{BlockInfo, FunctionInfo, SemanticGraph};

/// Environment variable overriding the local telemetry artifact directory.
pub const TELEMETRY_DIR_ENV: &str = "DUUMBI_TELEMETRY_DIR";

/// Trace map artifact file name.
pub const TRACE_MAP_FILE: &str = "trace_map.json";

/// Crash artifact file name.
pub const CRASH_DUMP_FILE: &str = "crash_dump.jsonl";

/// Schema version for trace-to-graph map artifacts.
pub const TRACE_MAP_SCHEMA_VERSION: &str = "duumbi.telemetry.trace_map.v1";

/// Schema version for crash dump artifacts.
pub const CRASH_SCHEMA_VERSION: &str = "duumbi.telemetry.crash.v1";

/// Schema version for repair validation evidence artifacts.
pub const REPAIR_VALIDATION_SCHEMA_VERSION: &str = "duumbi.telemetry.repair_validation.v1";

/// Schema version for repair context artifacts.
pub const REPAIR_CONTEXT_SCHEMA_VERSION: &str = "duumbi.telemetry.repair_context.v1";

const DEFAULT_ARTIFACT_DIR: &str = ".duumbi/telemetry";
const TRACE_ID_DOMAIN: &[u8] = b"duumbi-trace-v1\0";

/// Telemetry mode selected for one build invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TelemetryBuildMode {
    /// Compile without telemetry instrumentation.
    #[default]
    Off,
    /// Compile with local function/block trace instrumentation.
    Trace,
}

impl TelemetryBuildMode {
    /// Returns true when the build should emit local trace instrumentation.
    #[must_use]
    pub fn is_trace(self) -> bool {
        matches!(self, Self::Trace)
    }
}

/// Build options shared by CLI, workspace, and workflow build surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildOptions {
    /// Restrict dependency resolution to workspace and vendor layers only.
    pub offline: bool,
    /// Telemetry mode selected for this build invocation.
    pub telemetry: TelemetryBuildMode,
}

/// Telemetry artifact and trace-map errors.
#[derive(Debug, Error)]
pub enum TelemetryError {
    /// Two graph identities produced the same trace identifier.
    #[error(
        "trace ID collision for {trace_id}: {existing_kind} '{existing_graph_id}' and {new_kind} '{new_graph_id}'"
    )]
    TraceIdCollision {
        /// Colliding trace identifier.
        trace_id: u64,
        /// Existing entry kind.
        existing_kind: TraceMapKind,
        /// Existing entry graph identifier.
        existing_graph_id: String,
        /// New entry kind.
        new_kind: TraceMapKind,
        /// New entry graph identifier.
        new_graph_id: String,
    },
    /// Trace-map artifact serialization failed.
    #[error("failed to serialize trace map: {0}")]
    Serialize(#[source] serde_json::Error),
    /// Trace-map artifact filesystem write failed.
    #[error("{context}: {source}")]
    Io {
        /// Human-readable artifact write step.
        context: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Required telemetry evidence was not available.
    #[error("missing telemetry evidence: {0}")]
    MissingEvidence(String),
    /// A traced build could not derive required graph identity metadata.
    #[error(
        "missing {kind} graph identity for module '{module}', function '{function}'{block_context}"
    )]
    MissingTraceGraphIdentity {
        /// Missing trace-map entry kind.
        kind: TraceMapKind,
        /// Owning module name.
        module: String,
        /// Owning function name.
        function: String,
        /// Optional block label for block identity failures.
        block: Option<String>,
        /// Formatted optional block context for display.
        block_context: String,
    },
    /// A telemetry artifact could not be parsed.
    #[error("failed to parse telemetry artifact '{path}': {source}")]
    Parse {
        /// Artifact path.
        path: String,
        /// Underlying parse error.
        #[source]
        source: serde_json::Error,
    },
    /// Crash IDs did not map to graph context.
    #[error("trace evidence is unmapped: {0}")]
    Unmapped(String),
}

/// Trace-map entry kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceMapKind {
    /// Function trace identifier.
    Function,
    /// Basic block trace identifier.
    Block,
}

impl TraceMapKind {
    fn as_trace_kind(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Block => "block",
        }
    }
}

impl std::fmt::Display for TraceMapKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_trace_kind())
    }
}

/// One graph identity to trace identifier mapping.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TraceMapEntry {
    /// Stable runtime trace identifier.
    pub trace_id: u64,
    /// Mapped graph element kind.
    pub kind: TraceMapKind,
    /// Graph `@id` represented by this trace identifier.
    pub graph_id: String,
    /// Owning module name.
    pub module: String,
    /// Owning function name.
    pub function: String,
    /// Optional block label for block entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<String>,
}

/// Deterministic trace-to-graph mapping artifact.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TraceMap {
    /// Trace map schema version.
    pub schema_version: String,
    /// Optional future program hash or build identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program_hash: Option<String>,
    /// Function and block mapping entries.
    pub entries: Vec<TraceMapEntry>,
}

/// One crash evidence entry from `crash_dump.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CrashArtifact {
    /// Crash artifact schema version.
    pub schema_version: String,
    /// Event kind.
    pub event: String,
    /// Panic message.
    pub message: String,
    /// Current function trace ID at crash time.
    pub function_id: u64,
    /// Current block trace ID at crash time.
    pub block_id: u64,
    /// Whether tracing was active when the crash was written.
    pub trace_active: bool,
}

/// Human-readable mapped crash evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectReport {
    /// Crash message.
    pub message: String,
    /// Mapped function graph ID.
    pub function_graph_id: String,
    /// Mapped block graph ID.
    pub block_graph_id: String,
}

impl InspectReport {
    /// Formats the report for CLI output.
    #[must_use]
    pub fn to_cli_output(&self) -> String {
        format!(
            "Crash: {}\nFunction: {}\nBlock: {}\nExact node evidence: unavailable in v1",
            self.message, self.function_graph_id, self.block_graph_id
        )
    }
}

/// Runtime trace IDs correlated with mapped graph context.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TraceCorrelation {
    /// Current function trace ID at crash time.
    pub function_trace_id: u64,
    /// Current block trace ID at crash time.
    pub block_trace_id: u64,
}

/// Selected crash entry for repair-context assembly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CrashEntrySelection {
    /// Select the latest non-empty JSONL crash entry.
    #[default]
    Latest,
    /// Select a specific 1-based JSONL line from the crash artifact.
    LineNumber(usize),
}

impl CrashEntrySelection {
    fn evidence_value(self, selected_line: usize) -> String {
        match self {
            Self::Latest => "latest".to_string(),
            Self::LineNumber(_) => format!("line:{selected_line}"),
        }
    }
}

/// Local evidence provenance for one repair context.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepairContextEvidence {
    /// Canonical evidence source.
    pub source: String,
    /// Crash artifact path used to build the context.
    pub crash_path: String,
    /// Trace map artifact path used to build the context.
    pub map_path: String,
    /// Selected 1-based JSONL crash line.
    pub selected_crash_line: usize,
    /// Canonical selection value: `latest` or `line:<N>`.
    pub selection: String,
    /// Graph source paths used to build bounded graph context.
    pub graph_sources: Vec<String>,
}

/// Options for assembling a repair context from local telemetry artifacts.
#[derive(Debug, Clone)]
pub struct RepairContextOptions {
    /// Telemetry artifact directory.
    pub telemetry_dir: PathBuf,
    /// Explicit crash artifact path.
    pub crash_path: Option<PathBuf>,
    /// Explicit trace map artifact path.
    pub map_path: Option<PathBuf>,
    /// Graph source paths used for bounded context.
    pub graph_sources: Vec<PathBuf>,
    /// Crash entry selection mode.
    pub crash_entry: CrashEntrySelection,
}

impl RepairContextOptions {
    /// Creates repair-context assembly options for the supplied telemetry directory.
    #[must_use]
    pub fn new(telemetry_dir: impl Into<PathBuf>) -> Self {
        Self {
            telemetry_dir: telemetry_dir.into(),
            crash_path: None,
            map_path: None,
            graph_sources: Vec::new(),
            crash_entry: CrashEntrySelection::Latest,
        }
    }
}

/// Request for deterministic repair candidate validation.
#[derive(Debug, Clone)]
pub struct RepairValidationRequest {
    /// Mapped repair crash context assembled from local telemetry artifacts.
    pub crash_context: RepairCrashContext,
    /// Proposed repair patch JSON. This must deserialize as [`crate::patch::GraphPatch`].
    pub patch_json: serde_json::Value,
    /// Source artifact used as the original graph candidate input.
    pub source: RepairValidationSource,
}

impl RepairValidationRequest {
    /// Creates a single-graph repair validation request.
    #[must_use]
    pub fn single_graph(
        crash_context: RepairCrashContext,
        patch_json: serde_json::Value,
        graph_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            crash_context,
            patch_json,
            source: RepairValidationSource::SingleGraphFile(graph_path.into()),
        }
    }
}

/// Source artifact selected for repair validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairValidationSource {
    /// Validate a single JSON-LD graph source file.
    SingleGraphFile(PathBuf),
}

impl RepairValidationSource {
    fn graph_path(&self) -> &Path {
        match self {
            Self::SingleGraphFile(path) => path,
        }
    }
}

/// Result from an externalized repair validation execution step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairValidationStepOutcome {
    /// Whether the execution step passed.
    pub passed: bool,
    /// Bounded human-readable step summary.
    pub summary: String,
    /// Optional command or candidate artifact backing this step.
    pub command: Option<String>,
    /// Optional bounded output summary.
    pub output: Option<String>,
}

impl RepairValidationStepOutcome {
    /// Creates one repair validation execution step result.
    #[must_use]
    pub fn new(passed: bool, summary: impl Into<String>, output: Option<String>) -> Self {
        Self {
            passed,
            summary: summary.into(),
            command: None,
            output,
        }
    }

    /// Creates one repair validation execution step result with command evidence.
    #[must_use]
    pub fn with_command(
        passed: bool,
        summary: impl Into<String>,
        command: impl Into<String>,
        output: Option<String>,
    ) -> Self {
        Self {
            passed,
            summary: summary.into(),
            command: Some(command.into()),
            output,
        }
    }
}

/// Execution seam for native rebuild and relevant-test repair validation gates.
pub trait RepairValidationRunner {
    /// Rebuilds the temporary patched candidate graph.
    fn rebuild_candidate(&mut self, candidate_graph_json: &str) -> RepairValidationStepOutcome;

    /// Runs candidate-aware relevant tests after a successful rebuild.
    fn run_relevant_tests(
        &mut self,
        candidate_graph_json: &str,
        rebuild: &RepairValidationStepOutcome,
    ) -> RepairValidationStepOutcome;
}

/// Command-backed runner for native repair validation rebuild and test gates.
pub struct RepairValidationCommandRunner {
    temp_dir: tempfile::TempDir,
    original_graph_path: PathBuf,
    candidate_graph_path: PathBuf,
    candidate_binary_path: PathBuf,
    build_command: Vec<String>,
    test_commands: Vec<String>,
    max_output_bytes: usize,
}

impl RepairValidationCommandRunner {
    /// Creates a command runner for single-graph repair validation.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError`] if the temporary candidate directory cannot be created.
    pub fn single_graph(
        duumbi_exe: impl Into<PathBuf>,
        original_graph_path: impl Into<PathBuf>,
        test_commands: Vec<String>,
    ) -> Result<Self, TelemetryError> {
        let original_graph_path = original_graph_path.into();
        let build_command = vec![
            duumbi_exe.into().display().to_string(),
            "build".to_string(),
            "{candidate_graph}".to_string(),
            "-o".to_string(),
            "{candidate_binary}".to_string(),
        ];
        Self::with_commands(original_graph_path, build_command, test_commands)
    }

    /// Creates a command runner with explicit rebuild and test command templates.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError`] if the temporary candidate directory cannot be created.
    pub fn with_commands(
        original_graph_path: impl Into<PathBuf>,
        build_command: Vec<String>,
        test_commands: Vec<String>,
    ) -> Result<Self, TelemetryError> {
        let temp_dir = tempfile::TempDir::new().map_err(|source| TelemetryError::Io {
            context: "Failed to create repair validation temp directory".to_string(),
            source,
        })?;
        let candidate_graph_path = temp_dir.path().join("candidate.jsonld");
        let candidate_binary_path = temp_dir.path().join(candidate_binary_file_name());

        Ok(Self {
            temp_dir,
            original_graph_path: original_graph_path.into(),
            candidate_graph_path,
            candidate_binary_path,
            build_command,
            test_commands,
            max_output_bytes: 4096,
        })
    }

    fn substitute_placeholders(&self, value: &str) -> String {
        value
            .replace(
                "{candidate_graph}",
                &self.candidate_graph_path.display().to_string(),
            )
            .replace(
                "{candidate_binary}",
                &self.candidate_binary_path.display().to_string(),
            )
            .replace(
                "{candidate_workspace}",
                &self.temp_dir.path().display().to_string(),
            )
            .replace(
                "{original_graph}",
                &self.original_graph_path.display().to_string(),
            )
    }

    fn run_command_template(&self, template: &[String]) -> RepairValidationStepOutcome {
        if template.is_empty() {
            return RepairValidationStepOutcome::new(
                false,
                "command template is empty",
                Some("no command configured".to_string()),
            );
        }

        let command_parts = template
            .iter()
            .map(|part| self.substitute_placeholders(part))
            .collect::<Vec<_>>();
        let output = Command::new(&command_parts[0])
            .args(&command_parts[1..])
            .env("DUUMBI_REPAIR_CANDIDATE_GRAPH", &self.candidate_graph_path)
            .env(
                "DUUMBI_REPAIR_CANDIDATE_BINARY",
                &self.candidate_binary_path,
            )
            .env("DUUMBI_REPAIR_CANDIDATE_WORKSPACE", self.temp_dir.path())
            .env("DUUMBI_REPAIR_ORIGINAL_GRAPH", &self.original_graph_path)
            .output();

        match output {
            Ok(output) => {
                let summary = if output.status.success() {
                    "command exited successfully"
                } else {
                    "command exited unsuccessfully"
                };
                RepairValidationStepOutcome::with_command(
                    output.status.success(),
                    summary,
                    command_parts.join(" "),
                    Some(self.command_output_summary(&output)),
                )
            }
            Err(err) => RepairValidationStepOutcome::with_command(
                false,
                "command failed to start",
                command_parts.join(" "),
                Some(err.to_string()),
            ),
        }
    }

    fn command_output_summary(&self, output: &std::process::Output) -> String {
        let stdout = bounded_lossy(&output.stdout, self.max_output_bytes);
        let stderr = bounded_lossy(&output.stderr, self.max_output_bytes);
        format!(
            "status: {}; stdout: {}; stderr: {}",
            output.status, stdout, stderr
        )
    }
}

impl RepairValidationRunner for RepairValidationCommandRunner {
    fn rebuild_candidate(&mut self, candidate_graph_json: &str) -> RepairValidationStepOutcome {
        if let Err(err) = fs::write(&self.candidate_graph_path, candidate_graph_json) {
            return RepairValidationStepOutcome::new(
                false,
                "failed to write temporary candidate graph",
                Some(err.to_string()),
            );
        }

        self.run_command_template(&self.build_command)
    }

    fn run_relevant_tests(
        &mut self,
        _candidate_graph_json: &str,
        _rebuild: &RepairValidationStepOutcome,
    ) -> RepairValidationStepOutcome {
        let commands = if self.test_commands.is_empty() {
            vec!["{candidate_binary}".to_string()]
        } else {
            self.test_commands.clone()
        };

        for command in &commands {
            if !is_candidate_aware_test_command(command) {
                return RepairValidationStepOutcome::new(
                    false,
                    "relevant test command is not candidate-aware",
                    Some(command.clone()),
                );
            }
        }

        let mut command_summaries = Vec::new();
        let mut summaries = Vec::new();
        for command in commands {
            let Some(parts) = shlex::split(&command) else {
                return RepairValidationStepOutcome::new(
                    false,
                    "failed to parse relevant test command",
                    Some(command),
                );
            };
            let result = self.run_command_template(&parts);
            command_summaries.push(
                result
                    .command
                    .clone()
                    .unwrap_or_else(|| self.substitute_placeholders(&command)),
            );
            summaries.push(result.output.unwrap_or_else(|| result.summary.clone()));
            if !result.passed {
                return RepairValidationStepOutcome::with_command(
                    false,
                    "candidate-aware relevant test failed",
                    command_summaries.join("\n"),
                    Some(summaries.join("\n")),
                );
            }
        }

        RepairValidationStepOutcome::with_command(
            true,
            "candidate-aware relevant tests passed",
            command_summaries.join("\n"),
            Some(summaries.join("\n")),
        )
    }
}

/// Agent-facing crash context for proposing a graph repair.
///
/// This context is derived from mapped telemetry artifacts only. It does not
/// include provider output, patch application results, or repair acceptance.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RepairCrashContext {
    /// Repair context schema version.
    pub schema_version: String,
    /// Crash message captured by the traced runtime.
    pub crash_message: String,
    /// Mapped function graph ID.
    pub function_id: String,
    /// Mapped block graph ID.
    pub block_id: String,
    /// Exact graph node ID when available from telemetry evidence.
    pub exact_node_id: Option<String>,
    /// Runtime trace IDs used to establish the graph correlation.
    pub trace_ids: TraceCorrelation,
    /// Bounded mapped graph context available for repair review.
    pub graph_context: serde_json::Value,
    /// Local artifact provenance for this context.
    pub evidence: RepairContextEvidence,
    /// Validation checks expected before any proposed repair can be reviewed.
    pub validation_expectations: Vec<String>,
    /// Test checks expected before any proposed repair can be reviewed.
    pub test_expectations: Vec<String>,
    /// Repairs proposed from this context still require human review.
    pub human_review_required: bool,
}

/// A required gate in repair validation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairValidationGate {
    /// Proposed patch parses as a [`crate::patch::GraphPatch`].
    GraphPatchParse,
    /// Patch application is atomic and leaves the source unchanged on failure.
    AtomicPatchApplication,
    /// Patched JSON-LD parses into the Duumbi AST.
    GraphParse,
    /// Patched JSON-LD builds into the Duumbi graph IR.
    GraphBuild,
    /// Patched graph passes semantic validation.
    GraphValidation,
    /// Native rebuild succeeds after the candidate patch.
    NativeRebuild,
    /// Relevant targeted and regression tests pass.
    RelevantTests,
}

/// Source graph or workspace artifact considered during repair validation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepairValidationSourceArtifact {
    /// Source artifact kind, such as `graph_sources`.
    pub kind: String,
    /// Local graph or workspace paths linked to the repair validation context.
    pub paths: Vec<String>,
}

impl RepairValidationSourceArtifact {
    fn from_context(crash_context: &RepairCrashContext) -> Self {
        Self {
            kind: "graph_sources".to_string(),
            paths: crash_context.evidence.graph_sources.clone(),
        }
    }

    fn from_source(source: &RepairValidationSource) -> Self {
        Self {
            kind: "graph_sources".to_string(),
            paths: vec![source.graph_path().display().to_string()],
        }
    }
}

/// Bounded summary of the proposed repair patch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepairPatchSummary {
    /// Number of patch operations in the proposed `GraphPatch`.
    pub operation_count: usize,
}

impl RepairPatchSummary {
    fn from_patch_value(proposed_patch: &serde_json::Value) -> Self {
        let operation_count = proposed_patch
            .get("ops")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);

        Self { operation_count }
    }
}

/// Coarse status for rebuild and test evidence summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairValidationStepStatus {
    /// The step has not run yet.
    NotAttempted,
    /// The step passed.
    Passed,
    /// The step failed.
    Failed,
}

impl RepairValidationStepStatus {
    fn from_gate(gates: &[RepairValidationGateEvidence], gate: RepairValidationGate) -> Self {
        gates
            .iter()
            .find(|evidence| evidence.gate == gate)
            .map_or(Self::NotAttempted, |evidence| {
                if evidence.passed {
                    Self::Passed
                } else {
                    Self::Failed
                }
            })
    }
}

/// Bounded native rebuild evidence summary.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepairRebuildSummary {
    /// Native rebuild status derived from the rebuild gate evidence.
    pub status: RepairValidationStepStatus,
    /// Optional rebuild command or API summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Optional bounded rebuild output summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl RepairRebuildSummary {
    fn from_gates(gates: &[RepairValidationGateEvidence]) -> Self {
        let gate = gates
            .iter()
            .find(|evidence| evidence.gate == RepairValidationGate::NativeRebuild);

        Self {
            status: RepairValidationStepStatus::from_gate(
                gates,
                RepairValidationGate::NativeRebuild,
            ),
            command: gate.and_then(|evidence| evidence.command.clone()),
            output: gate.and_then(|evidence| evidence.output.clone()),
        }
    }
}

/// Bounded relevant-test evidence summary.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepairTestSummary {
    /// Relevant-test status derived from the test gate evidence.
    pub status: RepairValidationStepStatus,
    /// Candidate-aware test command summaries.
    pub commands: Vec<String>,
    /// Optional bounded test output summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl RepairTestSummary {
    fn from_gates(gates: &[RepairValidationGateEvidence]) -> Self {
        let gate = gates
            .iter()
            .find(|evidence| evidence.gate == RepairValidationGate::RelevantTests);

        Self {
            status: RepairValidationStepStatus::from_gate(
                gates,
                RepairValidationGate::RelevantTests,
            ),
            commands: gate
                .and_then(|evidence| evidence.command.as_deref())
                .map(split_evidence_lines)
                .unwrap_or_default(),
            output: gate.and_then(|evidence| evidence.output.clone()),
        }
    }
}

/// Human review state represented by repair validation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairHumanReviewState {
    /// Local validation has not produced a human-review-ready candidate.
    NotReady,
    /// Local validation passed and the candidate awaits human review.
    Pending,
    /// A human reviewer requested revision after local validation.
    RevisionRequested,
    /// A human reviewer rejected the candidate after local validation.
    Rejected,
}

/// Evidence captured for one repair validation gate.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepairValidationGateEvidence {
    /// Gate represented by this evidence item.
    pub gate: RepairValidationGate,
    /// Whether the gate passed.
    pub passed: bool,
    /// Human-readable summary of the gate result.
    pub summary: String,
    /// Optional command or candidate artifact backing the gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Optional bounded output or diagnostic summary backing the gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl RepairValidationGateEvidence {
    /// Creates one repair validation gate evidence item.
    #[must_use]
    pub fn new(
        gate: RepairValidationGate,
        passed: bool,
        summary: impl Into<String>,
        output: Option<String>,
    ) -> Self {
        Self {
            gate,
            passed,
            summary: summary.into(),
            command: None,
            output,
        }
    }

    /// Creates one repair validation gate evidence item with command evidence.
    #[must_use]
    pub fn with_command(
        gate: RepairValidationGate,
        passed: bool,
        summary: impl Into<String>,
        command: Option<String>,
        output: Option<String>,
    ) -> Self {
        Self {
            gate,
            passed,
            summary: summary.into(),
            command,
            output,
        }
    }
}

/// Human-reviewable repair validation evidence.
///
/// The evidence intentionally separates "all local gates passed" from "repair
/// accepted": callers must keep human review in the loop before any repair is
/// treated as complete.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RepairValidationEvidence {
    /// Repair validation schema version.
    pub schema_version: String,
    /// Original mapped crash context.
    pub crash_context: RepairCrashContext,
    /// Proposed graph patch serialized for review.
    pub proposed_patch: serde_json::Value,
    /// Source artifact considered during validation.
    pub source_artifact: RepairValidationSourceArtifact,
    /// Bounded proposed patch summary.
    pub patch_summary: RepairPatchSummary,
    /// Validation gate results.
    pub gates: Vec<RepairValidationGateEvidence>,
    /// Bounded native rebuild summary.
    pub rebuild_summary: RepairRebuildSummary,
    /// Bounded relevant-test summary.
    pub test_summary: RepairTestSummary,
    /// Whether all required local validation gates passed.
    pub local_validation_passed: bool,
    /// Repairs must remain human-reviewable and are not silently accepted.
    pub requires_human_review: bool,
    /// Repair acceptance state. This remains false in telemetry evidence.
    pub accepted_for_application: bool,
    /// Human review state. Local validation can only move this to `pending`.
    pub human_review_state: RepairHumanReviewState,
}

impl RepairValidationEvidence {
    /// Creates repair validation evidence without accepting the repair.
    #[must_use]
    pub fn new(
        crash_context: RepairCrashContext,
        proposed_patch: serde_json::Value,
        gates: Vec<RepairValidationGateEvidence>,
    ) -> Self {
        let source_artifact = RepairValidationSourceArtifact::from_context(&crash_context);
        Self::with_source_artifact(crash_context, proposed_patch, source_artifact, gates)
    }

    fn with_source_artifact(
        crash_context: RepairCrashContext,
        proposed_patch: serde_json::Value,
        source_artifact: RepairValidationSourceArtifact,
        gates: Vec<RepairValidationGateEvidence>,
    ) -> Self {
        let local_validation_passed =
            required_repair_validation_gates()
                .iter()
                .all(|required_gate| {
                    gates
                        .iter()
                        .any(|evidence| evidence.gate == *required_gate && evidence.passed)
                });
        let human_review_state = if local_validation_passed {
            RepairHumanReviewState::Pending
        } else {
            RepairHumanReviewState::NotReady
        };
        let patch_summary = RepairPatchSummary::from_patch_value(&proposed_patch);
        let rebuild_summary = RepairRebuildSummary::from_gates(&gates);
        let test_summary = RepairTestSummary::from_gates(&gates);

        Self {
            schema_version: REPAIR_VALIDATION_SCHEMA_VERSION.to_string(),
            crash_context,
            proposed_patch,
            source_artifact,
            patch_summary,
            gates,
            rebuild_summary,
            test_summary,
            local_validation_passed,
            requires_human_review: true,
            accepted_for_application: false,
            human_review_state,
        }
    }

    /// Returns true when every required repair validation gate has evidence.
    #[must_use]
    pub fn has_required_gates(&self) -> bool {
        required_repair_validation_gates()
            .iter()
            .all(|required_gate| {
                self.gates
                    .iter()
                    .any(|evidence| evidence.gate == *required_gate)
            })
    }
}

/// Returns the required validation gates for repair evidence.
#[must_use]
pub fn required_repair_validation_gates() -> Vec<RepairValidationGate> {
    vec![
        RepairValidationGate::GraphPatchParse,
        RepairValidationGate::AtomicPatchApplication,
        RepairValidationGate::GraphParse,
        RepairValidationGate::GraphBuild,
        RepairValidationGate::GraphValidation,
        RepairValidationGate::NativeRebuild,
        RepairValidationGate::RelevantTests,
    ]
}

impl TraceMap {
    /// Creates a trace map after sorting and collision validation.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError::TraceIdCollision`] if two different graph
    /// identities produce the same runtime trace identifier.
    pub fn new(mut entries: Vec<TraceMapEntry>) -> Result<Self, TelemetryError> {
        entries.sort_by(|left, right| {
            left.kind
                .as_trace_kind()
                .cmp(right.kind.as_trace_kind())
                .then_with(|| left.graph_id.cmp(&right.graph_id))
        });
        check_trace_id_collisions(&entries)?;

        Ok(Self {
            schema_version: TRACE_MAP_SCHEMA_VERSION.to_string(),
            program_hash: None,
            entries,
        })
    }

    /// Creates a trace map for one semantic graph.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError::MissingTraceGraphIdentity`] if this graph has
    /// functions or blocks without stable graph identities. Returns
    /// [`TelemetryError::TraceIdCollision`] if generated IDs collide inside
    /// this graph.
    pub fn from_graph(graph: &SemanticGraph) -> Result<Self, TelemetryError> {
        Self::new(entries_for_graph(graph)?)
    }

    /// Creates a combined trace map for all modules in a program.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError::MissingTraceGraphIdentity`] if any program
    /// module has functions or blocks without stable graph identities. Returns
    /// [`TelemetryError::TraceIdCollision`] if generated IDs collide inside the
    /// compiled program.
    pub fn from_program(program: &Program) -> Result<Self, TelemetryError> {
        let entries = program
            .modules
            .values()
            .map(entries_for_graph)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        Self::new(entries)
    }
}

/// Generates a stable signed-`int64_t` compatible trace ID.
#[must_use]
pub fn trace_id(kind: TraceMapKind, graph_id: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(TRACE_ID_DOMAIN);
    hasher.update(kind.as_trace_kind().as_bytes());
    hasher.update(b"\0");
    hasher.update(graph_id.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes) & 0x7fff_ffff_ffff_ffff
}

/// Writes a trace map artifact to the supplied telemetry directory.
///
/// # Errors
///
/// Returns [`TelemetryError`] if the directory cannot be created, the map
/// cannot be serialized, or the artifact cannot be written.
pub fn write_trace_map(map: &TraceMap, telemetry_dir: &Path) -> Result<PathBuf, TelemetryError> {
    fs::create_dir_all(telemetry_dir).map_err(|source| TelemetryError::Io {
        context: format!(
            "Failed to create telemetry directory '{}'",
            telemetry_dir.display()
        ),
        source,
    })?;

    let bytes = serde_json::to_vec_pretty(map).map_err(TelemetryError::Serialize)?;
    let path = telemetry_dir.join(TRACE_MAP_FILE);
    fs::write(&path, bytes).map_err(|source| TelemetryError::Io {
        context: format!("Failed to write trace map '{}'", path.display()),
        source,
    })?;
    Ok(path)
}

/// Inspects crash evidence and maps it to function/block graph context.
///
/// # Errors
///
/// Returns [`TelemetryError`] when crash/map evidence is missing, malformed, or
/// cannot map the crash trace IDs to graph entries.
pub fn inspect_crash_artifacts(
    telemetry_dir: &Path,
    crash_path: Option<&Path>,
    map_path: Option<&Path>,
) -> Result<InspectReport, TelemetryError> {
    let mapped = mapped_crash_evidence(
        telemetry_dir,
        crash_path,
        map_path,
        CrashEntrySelection::Latest,
    )?;

    Ok(InspectReport {
        message: mapped.crash.message,
        function_graph_id: mapped.function.graph_id,
        block_graph_id: mapped.block.graph_id,
    })
}

/// Builds an agent-facing repair context from mapped crash evidence.
///
/// # Errors
///
/// Always returns [`TelemetryError::MissingEvidence`] because successful repair
/// context assembly requires explicit graph source paths. Use
/// [`repair_crash_context`] with graph-source-aware options instead.
#[deprecated(
    since = "0.3.3",
    note = "use repair_crash_context with RepairContextOptions instead"
)]
pub fn repair_crash_context_from_artifacts(
    _telemetry_dir: &Path,
    _crash_path: Option<&Path>,
    _map_path: Option<&Path>,
) -> Result<RepairCrashContext, TelemetryError> {
    Err(TelemetryError::MissingEvidence(
        "repair context requires at least one graph source; use graph-source-aware repair context options"
            .to_string(),
    ))
}

/// Builds an agent-facing repair context from mapped crash evidence and graph sources.
///
/// # Errors
///
/// Returns [`TelemetryError`] when crash/map/graph evidence is missing,
/// malformed, unmapped, stale, or ambiguous.
pub fn repair_crash_context(
    options: &RepairContextOptions,
) -> Result<RepairCrashContext, TelemetryError> {
    let mapped = mapped_crash_evidence(
        &options.telemetry_dir,
        options.crash_path.as_deref(),
        options.map_path.as_deref(),
        options.crash_entry,
    )?;
    let graph_context =
        repair_graph_context_from_sources(&mapped.function, &mapped.block, &options.graph_sources)?;
    let graph_sources = options
        .graph_sources
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    Ok(RepairCrashContext {
        schema_version: REPAIR_CONTEXT_SCHEMA_VERSION.to_string(),
        crash_message: mapped.crash.message,
        function_id: mapped.function.graph_id.clone(),
        block_id: mapped.block.graph_id.clone(),
        exact_node_id: None,
        trace_ids: TraceCorrelation {
            function_trace_id: mapped.crash.function_id,
            block_trace_id: mapped.crash.block_id,
        },
        graph_context,
        evidence: RepairContextEvidence {
            source: "local_telemetry_artifacts".to_string(),
            crash_path: mapped.crash_path.display().to_string(),
            map_path: mapped.map_path.display().to_string(),
            selected_crash_line: mapped.selected_crash_line,
            selection: mapped.selection.evidence_value(mapped.selected_crash_line),
            graph_sources,
        },
        validation_expectations: default_validation_expectations(),
        test_expectations: default_test_expectations(),
        human_review_required: true,
    })
}

/// Parses a proposed repair patch through the canonical [`crate::patch::GraphPatch`] contract.
///
/// # Errors
///
/// Returns [`TelemetryError`] when the proposed patch does not deserialize as a
/// graph patch.
pub fn parse_repair_graph_patch(
    patch: &serde_json::Value,
) -> Result<crate::patch::GraphPatch, TelemetryError> {
    serde_json::from_value(patch.clone()).map_err(|source| TelemetryError::Parse {
        path: "<repair graph patch>".to_string(),
        source,
    })
}

/// Creates human-reviewable validation evidence for a parsed repair patch.
///
/// # Errors
///
/// Returns [`TelemetryError`] when the patch cannot be serialized for evidence.
pub fn repair_validation_evidence_from_graph_patch(
    crash_context: RepairCrashContext,
    patch: &crate::patch::GraphPatch,
    gates: Vec<RepairValidationGateEvidence>,
) -> Result<RepairValidationEvidence, TelemetryError> {
    let proposed_patch = serde_json::to_value(patch).map_err(TelemetryError::Serialize)?;
    Ok(RepairValidationEvidence::new(
        crash_context,
        proposed_patch,
        gates,
    ))
}

/// Validates a proposed repair candidate through the local graph gates.
///
/// The original graph source is read but never written. Patch application uses
/// [`crate::patch::apply_patch()`], which clones the JSON-LD value before
/// applying operations.
///
/// # Errors
///
/// Returns [`TelemetryError`] when the source graph cannot be read or parsed as
/// JSON before the graph-gate evidence can be constructed.
pub fn validate_repair_candidate(
    request: RepairValidationRequest,
) -> Result<RepairValidationEvidence, TelemetryError> {
    validate_repair_candidate_inner(request, None)
}

/// Returns true when a repair context includes the graph source selected for validation.
#[must_use]
pub fn repair_context_includes_graph_source(
    crash_context: &RepairCrashContext,
    graph_path: &Path,
) -> bool {
    crash_context
        .evidence
        .graph_sources
        .iter()
        .any(|source| graph_sources_match(source, graph_path))
}

/// Validates a proposed repair candidate and executes rebuild/test gates through a runner.
///
/// The runner receives the temporary patched graph JSON and must remain
/// candidate-aware. It must not write the original source graph.
///
/// # Errors
///
/// Returns [`TelemetryError`] when the source graph cannot be read or parsed as
/// JSON before the graph-gate evidence can be constructed.
pub fn validate_repair_candidate_with_runner(
    request: RepairValidationRequest,
    runner: &mut dyn RepairValidationRunner,
) -> Result<RepairValidationEvidence, TelemetryError> {
    validate_repair_candidate_inner(request, Some(runner))
}

fn validate_repair_candidate_inner(
    request: RepairValidationRequest,
    runner: Option<&mut dyn RepairValidationRunner>,
) -> Result<RepairValidationEvidence, TelemetryError> {
    let source_artifact = RepairValidationSourceArtifact::from_source(&request.source);
    let mut gates = Vec::new();

    let patch = match parse_repair_graph_patch(&request.patch_json) {
        Ok(patch) => {
            gates.push(RepairValidationGateEvidence::new(
                RepairValidationGate::GraphPatchParse,
                true,
                "proposed patch parsed as GraphPatch",
                None,
            ));
            patch
        }
        Err(err) => {
            gates.push(RepairValidationGateEvidence::new(
                RepairValidationGate::GraphPatchParse,
                false,
                "proposed patch did not parse as GraphPatch",
                Some(err.to_string()),
            ));
            return Ok(RepairValidationEvidence::with_source_artifact(
                request.crash_context,
                request.patch_json,
                source_artifact,
                gates,
            ));
        }
    };

    let source_path = request.source.graph_path();
    let source_text = fs::read_to_string(source_path).map_err(|source| TelemetryError::Io {
        context: format!(
            "Failed to read repair validation source '{}'",
            source_path.display()
        ),
        source,
    })?;
    let source_value =
        serde_json::from_str::<serde_json::Value>(&source_text).map_err(|source| {
            TelemetryError::Parse {
                path: source_path.display().to_string(),
                source,
            }
        })?;

    let patched_value = match crate::patch::apply_patch(&source_value, &patch) {
        Ok(patched_value) => {
            gates.push(RepairValidationGateEvidence::new(
                RepairValidationGate::AtomicPatchApplication,
                true,
                format!(
                    "applied {} patch operation(s) to a cloned source",
                    patch.ops.len()
                ),
                None,
            ));
            patched_value
        }
        Err(err) => {
            gates.push(RepairValidationGateEvidence::new(
                RepairValidationGate::AtomicPatchApplication,
                false,
                "patch application failed before graph parsing",
                Some(err.to_string()),
            ));
            return Ok(RepairValidationEvidence::with_source_artifact(
                request.crash_context,
                request.patch_json,
                source_artifact.clone(),
                gates,
            ));
        }
    };

    let patched_text = serde_json::to_string(&patched_value).map_err(TelemetryError::Serialize)?;
    let module_ast = match crate::parser::parse_jsonld(&patched_text) {
        Ok(module_ast) => {
            gates.push(RepairValidationGateEvidence::new(
                RepairValidationGate::GraphParse,
                true,
                "patched JSON-LD parsed into the Duumbi AST",
                None,
            ));
            module_ast
        }
        Err(err) => {
            gates.push(RepairValidationGateEvidence::new(
                RepairValidationGate::GraphParse,
                false,
                "patched JSON-LD failed graph parsing",
                Some(err.to_string()),
            ));
            return Ok(RepairValidationEvidence::with_source_artifact(
                request.crash_context,
                request.patch_json,
                source_artifact.clone(),
                gates,
            ));
        }
    };

    let semantic_graph = match crate::graph::builder::build_graph(&module_ast) {
        Ok(semantic_graph) => {
            gates.push(RepairValidationGateEvidence::new(
                RepairValidationGate::GraphBuild,
                true,
                "patched AST built into graph IR",
                None,
            ));
            semantic_graph
        }
        Err(errors) => {
            gates.push(RepairValidationGateEvidence::new(
                RepairValidationGate::GraphBuild,
                false,
                "patched AST failed graph build",
                Some(join_graph_errors(&errors)),
            ));
            return Ok(RepairValidationEvidence::with_source_artifact(
                request.crash_context,
                request.patch_json,
                source_artifact.clone(),
                gates,
            ));
        }
    };

    let diagnostics = crate::graph::validator::validate(&semantic_graph);
    if diagnostics.is_empty() {
        gates.push(RepairValidationGateEvidence::new(
            RepairValidationGate::GraphValidation,
            true,
            "patched graph passed semantic validation",
            None,
        ));
    } else {
        gates.push(RepairValidationGateEvidence::new(
            RepairValidationGate::GraphValidation,
            false,
            "patched graph failed semantic validation",
            Some(join_diagnostics(&diagnostics)),
        ));
        return Ok(RepairValidationEvidence::with_source_artifact(
            request.crash_context,
            request.patch_json,
            source_artifact.clone(),
            gates,
        ));
    }

    if let Some(runner) = runner {
        let rebuild = runner.rebuild_candidate(&patched_text);
        gates.push(RepairValidationGateEvidence::with_command(
            RepairValidationGate::NativeRebuild,
            rebuild.passed,
            rebuild.summary.clone(),
            rebuild.command.clone(),
            rebuild.output.clone(),
        ));
        if !rebuild.passed {
            return Ok(RepairValidationEvidence::with_source_artifact(
                request.crash_context,
                request.patch_json,
                source_artifact.clone(),
                gates,
            ));
        }

        let tests = runner.run_relevant_tests(&patched_text, &rebuild);
        gates.push(RepairValidationGateEvidence::with_command(
            RepairValidationGate::RelevantTests,
            tests.passed,
            tests.summary,
            tests.command,
            tests.output,
        ));
    }

    Ok(RepairValidationEvidence::with_source_artifact(
        request.crash_context,
        request.patch_json,
        source_artifact,
        gates,
    ))
}

fn graph_sources_match(context_source: &str, graph_path: &Path) -> bool {
    let context_path = Path::new(context_source);
    if context_path == graph_path {
        return true;
    }

    match (context_path.canonicalize(), graph_path.canonicalize()) {
        (Ok(context_source), Ok(graph_path)) => context_source == graph_path,
        _ => false,
    }
}

fn join_graph_errors(errors: &[crate::graph::GraphError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn join_diagnostics(diagnostics: &[crate::errors::Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn candidate_binary_file_name() -> String {
    if cfg!(windows) {
        "repair-candidate.exe".to_string()
    } else {
        "repair-candidate".to_string()
    }
}

fn is_candidate_aware_test_command(command: &str) -> bool {
    command.contains("{candidate_graph}")
        || command.contains("{candidate_binary}")
        || command.contains("{candidate_workspace}")
}

fn split_evidence_lines(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn bounded_lossy(bytes: &[u8], max_bytes: usize) -> String {
    let len = bytes.len().min(max_bytes);
    let mut text = String::from_utf8_lossy(&bytes[..len]).replace('\n', "\\n");
    if bytes.len() > max_bytes {
        text.push_str("...[truncated]");
    }
    text
}

#[derive(Debug, Clone)]
struct MappedCrashEvidence {
    crash: CrashArtifact,
    crash_path: PathBuf,
    map_path: PathBuf,
    selected_crash_line: usize,
    selection: CrashEntrySelection,
    function: TraceMapEntry,
    block: TraceMapEntry,
}

fn mapped_crash_evidence(
    telemetry_dir: &Path,
    crash_path: Option<&Path>,
    map_path: Option<&Path>,
    selection: CrashEntrySelection,
) -> Result<MappedCrashEvidence, TelemetryError> {
    let crash_path = crash_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| telemetry_dir.join(CRASH_DUMP_FILE));
    let map_path = map_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| telemetry_dir.join(TRACE_MAP_FILE));
    let selected_crash = read_selected_crash(&crash_path, selection)?;
    let trace_map = read_trace_map(&map_path)?;

    if !selected_crash.artifact.trace_active {
        return Err(TelemetryError::MissingEvidence(
            "crash artifact was not written from an active traced run".to_string(),
        ));
    }

    let function = trace_map
        .entries
        .iter()
        .find(|entry| {
            entry.kind == TraceMapKind::Function
                && entry.trace_id == selected_crash.artifact.function_id
        })
        .ok_or_else(|| {
            TelemetryError::Unmapped(format!(
                "function trace ID {} was not found in '{}'",
                selected_crash.artifact.function_id,
                map_path.display()
            ))
        })?;
    let block = trace_map
        .entries
        .iter()
        .find(|entry| {
            entry.kind == TraceMapKind::Block && entry.trace_id == selected_crash.artifact.block_id
        })
        .ok_or_else(|| {
            TelemetryError::Unmapped(format!(
                "block trace ID {} was not found in '{}'",
                selected_crash.artifact.block_id,
                map_path.display()
            ))
        })?;

    Ok(MappedCrashEvidence {
        crash: selected_crash.artifact,
        crash_path,
        map_path,
        selected_crash_line: selected_crash.line_number,
        selection,
        function: function.clone(),
        block: block.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectedCrash {
    artifact: CrashArtifact,
    line_number: usize,
}

fn read_selected_crash(
    path: &Path,
    selection: CrashEntrySelection,
) -> Result<SelectedCrash, TelemetryError> {
    let content = fs::read_to_string(path).map_err(|source| TelemetryError::Io {
        context: format!("Failed to read crash artifact '{}'", path.display()),
        source,
    })?;

    let (line_number, line) = match selection {
        CrashEntrySelection::Latest => {
            let latest = content
                .lines()
                .enumerate()
                .filter(|(_, line)| !line.trim().is_empty())
                .last()
                .map(|(index, line)| (index + 1, line));
            latest.ok_or_else(|| {
                TelemetryError::MissingEvidence(format!(
                    "crash artifact '{}' did not contain any JSONL entries",
                    path.display()
                ))
            })?
        }
        CrashEntrySelection::LineNumber(line_number) if line_number > 0 => {
            let line = content.lines().nth(line_number - 1).ok_or_else(|| {
                TelemetryError::MissingEvidence(format!(
                    "crash artifact '{}' did not contain line {}",
                    path.display(),
                    line_number
                ))
            })?;
            if line.trim().is_empty() {
                return Err(TelemetryError::MissingEvidence(format!(
                    "crash artifact '{}' line {} is empty",
                    path.display(),
                    line_number
                )));
            }
            (line_number, line)
        }
        CrashEntrySelection::LineNumber(_) => {
            return Err(TelemetryError::MissingEvidence(
                "crash entry line numbers are 1-based".to_string(),
            ));
        }
    };

    let artifact = serde_json::from_str(line).map_err(|source| TelemetryError::Parse {
        path: format!("{}:{line_number}", path.display()),
        source,
    })?;
    Ok(SelectedCrash {
        artifact,
        line_number,
    })
}

fn repair_graph_context_from_sources(
    function: &TraceMapEntry,
    block: &TraceMapEntry,
    graph_sources: &[PathBuf],
) -> Result<serde_json::Value, TelemetryError> {
    if graph_sources.is_empty() {
        return Err(TelemetryError::MissingEvidence(
            "repair context requires at least one graph source".to_string(),
        ));
    }

    let mut function_matches = Vec::new();
    let mut block_matches = Vec::new();
    for graph_source in graph_sources {
        let source = read_graph_source(graph_source)?;
        let functions = graph_functions(&source.value);
        for candidate_function in &functions {
            for candidate_block in graph_blocks(candidate_function) {
                if graph_id(candidate_block) == Some(block.graph_id.as_str()) {
                    block_matches.push((
                        source.path.clone(),
                        graph_id(candidate_function)
                            .unwrap_or("<missing function @id>")
                            .to_string(),
                        candidate_block.clone(),
                    ));
                }
            }
        }

        let matches: Vec<_> = functions
            .iter()
            .copied()
            .filter(|candidate| graph_id(candidate) == Some(function.graph_id.as_str()))
            .collect();
        if matches.len() > 1 {
            return Err(TelemetryError::Unmapped(format!(
                "ambiguous graph source: function '{}' appears more than once in '{}'",
                function.graph_id,
                graph_source.display()
            )));
        }
        if let Some(candidate) = matches.into_iter().next() {
            function_matches.push((source.path, candidate.clone()));
        }
    }

    if block_matches.len() > 1 {
        return Err(TelemetryError::Unmapped(format!(
            "ambiguous graph source: block '{}' appears in more than one supplied graph context",
            block.graph_id
        )));
    }

    let (source_path, function_value) = match function_matches.len() {
        0 => {
            return Err(TelemetryError::Unmapped(format!(
                "stale or missing function graph context: '{}' was not found in supplied graph sources",
                function.graph_id
            )));
        }
        1 => function_matches
            .into_iter()
            .next()
            .expect("invariant: one function match exists"),
        _ => {
            return Err(TelemetryError::Unmapped(format!(
                "ambiguous graph source: function '{}' appears in more than one supplied graph file",
                function.graph_id
            )));
        }
    };

    let blocks = graph_blocks(&function_value);
    let block_matches: Vec<_> = blocks
        .into_iter()
        .filter(|candidate| graph_id(candidate) == Some(block.graph_id.as_str()))
        .collect();
    let block_value = match block_matches.len() {
        0 => {
            return Err(TelemetryError::Unmapped(format!(
                "stale or missing block graph context: '{}' was not found in mapped function '{}'",
                block.graph_id, function.graph_id
            )));
        }
        1 => block_matches
            .into_iter()
            .next()
            .expect("invariant: one block match exists")
            .clone(),
        _ => {
            return Err(TelemetryError::Unmapped(format!(
                "ambiguous graph source: block '{}' appears more than once in mapped function '{}'",
                block.graph_id, function.graph_id
            )));
        }
    };

    let mut function_shell = serde_json::Map::new();
    copy_json_field(&function_value, &mut function_shell, "@id");
    copy_json_field(&function_value, &mut function_shell, "@type");
    copy_json_field(&function_value, &mut function_shell, "duumbi:name");
    copy_json_field(&function_value, &mut function_shell, "duumbi:returnType");
    function_shell.insert(
        "duumbi:blocks".to_string(),
        serde_json::Value::Array(vec![block_value.clone()]),
    );

    Ok(serde_json::json!({
        "source": "mapped_graph_source",
        "source_path": source_path.display().to_string(),
        "function": {
            "trace_id": function.trace_id,
            "graph_id": function.graph_id,
            "module": function.module,
            "function": function.function
        },
        "block": {
            "trace_id": block.trace_id,
            "graph_id": block.graph_id,
            "module": block.module,
            "function": block.function,
            "block": block.block
        },
        "function_context": serde_json::Value::Object(function_shell),
        "selected_block": block_value,
        "exact_node_evidence": null,
        "context_limit": "containing_function_and_selected_block"
    }))
}

#[derive(Debug)]
struct GraphSource {
    path: PathBuf,
    value: serde_json::Value,
}

fn read_graph_source(path: &Path) -> Result<GraphSource, TelemetryError> {
    let content = fs::read_to_string(path).map_err(|source| TelemetryError::Io {
        context: format!("Failed to read graph source '{}'", path.display()),
        source,
    })?;
    let value = serde_json::from_str(&content).map_err(|source| TelemetryError::Parse {
        path: path.display().to_string(),
        source,
    })?;
    Ok(GraphSource {
        path: path.to_path_buf(),
        value,
    })
}

fn graph_functions(value: &serde_json::Value) -> Vec<&serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => {
            let mut functions = Vec::new();
            if object.get("@type").and_then(serde_json::Value::as_str) == Some("duumbi:Function") {
                functions.push(value);
            }
            for child in object.values() {
                functions.extend(graph_functions(child));
            }
            functions
        }
        serde_json::Value::Array(items) => items.iter().flat_map(graph_functions).collect(),
        _ => Vec::new(),
    }
}

fn graph_blocks(function: &serde_json::Value) -> Vec<&serde_json::Value> {
    function
        .get("duumbi:blocks")
        .and_then(serde_json::Value::as_array)
        .map(|blocks| blocks.iter().collect())
        .unwrap_or_default()
}

fn graph_id(value: &serde_json::Value) -> Option<&str> {
    value.get("@id").and_then(serde_json::Value::as_str)
}

fn copy_json_field(
    source: &serde_json::Value,
    target: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = source.get(field) {
        target.insert(field.to_string(), value.clone());
    }
}

fn default_validation_expectations() -> Vec<String> {
    [
        "proposed patch parses as GraphPatch",
        "patch application is atomic",
        "patched graph parses and builds",
        "graph validation passes",
        "native rebuild passes",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn default_test_expectations() -> Vec<String> {
    [
        "controlled crash remains reproducible before the patch",
        "targeted regression passes after the candidate patch",
        "default untraced build behavior remains unchanged",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn read_trace_map(path: &Path) -> Result<TraceMap, TelemetryError> {
    let content = fs::read_to_string(path).map_err(|source| TelemetryError::Io {
        context: format!("Failed to read trace map '{}'", path.display()),
        source,
    })?;
    serde_json::from_str(&content).map_err(|source| TelemetryError::Parse {
        path: path.display().to_string(),
        source,
    })
}

impl BuildOptions {
    /// Creates build options for the supplied offline and telemetry settings.
    #[must_use]
    pub fn new(offline: bool, telemetry: TelemetryBuildMode) -> Self {
        Self { offline, telemetry }
    }

    /// Creates default build options with offline mode optionally enabled.
    #[must_use]
    pub fn offline(offline: bool) -> Self {
        Self {
            offline,
            ..Self::default()
        }
    }
}

/// Optional local telemetry settings from the `[telemetry]` config section.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct TelemetrySection {
    /// Whether telemetry is enabled by config.
    ///
    /// Build and run surfaces still require an explicit traced mode before
    /// emitting telemetry. This field only records the user's local preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Sampling mode name for traced telemetry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling_mode: Option<String>,

    /// Sampling rate in the inclusive range `0.0..=1.0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<f64>,

    /// Local directory for telemetry artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_dir: Option<PathBuf>,

    /// Whether traced runs may capture argument or value snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_values: Option<bool>,
}

/// Supported telemetry sampling modes after validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetrySamplingMode {
    /// Stable sampling decisions suitable for tests and reproducible runs.
    Deterministic,
    /// Probability-based sampling using `sample-rate`.
    Probabilistic,
}

impl TelemetrySamplingMode {
    fn parse(value: &str) -> Result<Self, TelemetryValidationError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "deterministic" => Ok(Self::Deterministic),
            "probabilistic" => Ok(Self::Probabilistic),
            other => Err(TelemetryValidationError::UnsupportedSamplingMode {
                value: other.to_string(),
            }),
        }
    }
}

/// Effective telemetry config after traced-build validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTelemetryConfig {
    /// Enables runtime telemetry emission for trace-capable binaries.
    pub enabled: bool,
    /// Validated sampling mode.
    pub sampling_mode: TelemetrySamplingMode,
    /// Validated sample rate.
    pub sample_rate: f64,
    /// Resolved local artifact directory.
    pub artifact_dir: PathBuf,
    /// Whether the artifact dir came from `DUUMBI_TELEMETRY_DIR`.
    pub artifact_dir_overridden: bool,
    /// Whether argument or value snapshots may be captured.
    pub capture_values: bool,
}

/// Telemetry config validation failure.
#[derive(Debug, Error, PartialEq)]
pub enum TelemetryValidationError {
    /// Unsupported sampling mode.
    #[error(
        "telemetry sampling-mode '{value}' is unsupported; expected 'deterministic' or 'probabilistic'"
    )]
    UnsupportedSamplingMode {
        /// Invalid sampling mode value.
        value: String,
    },
    /// Invalid sample rate.
    #[error("telemetry sample-rate must be between 0.0 and 1.0 inclusive; got {value}")]
    InvalidSampleRate {
        /// Invalid sample rate value.
        value: f64,
    },
    /// Invalid artifact directory.
    #[error("telemetry artifact-dir is invalid: {reason}")]
    InvalidArtifactDir {
        /// Human-readable reason.
        reason: String,
    },
    /// Value capture is not supported yet.
    #[error("telemetry capture-values is not supported yet and must remain false")]
    CaptureValuesUnsupported,
}

impl TelemetrySection {
    /// Returns whether telemetry is enabled after defaults.
    #[must_use]
    pub fn effective_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    /// Returns whether argument or value snapshots may be captured after defaults.
    #[must_use]
    pub fn effective_capture_values(&self) -> bool {
        self.capture_values.unwrap_or(false)
    }

    /// Returns the configured sampling mode before validation.
    #[must_use]
    pub fn configured_sampling_mode(&self) -> &str {
        self.sampling_mode.as_deref().unwrap_or("deterministic")
    }

    /// Returns the configured sampling rate before validation.
    #[must_use]
    pub fn configured_sample_rate(&self) -> f64 {
        self.sample_rate.unwrap_or(0.0)
    }

    /// Returns the configured artifact directory before env overrides.
    #[must_use]
    pub fn configured_artifact_dir(&self) -> &Path {
        self.artifact_dir
            .as_deref()
            .unwrap_or_else(|| Path::new(DEFAULT_ARTIFACT_DIR))
    }

    /// Resolves the effective local artifact directory.
    ///
    /// `DUUMBI_TELEMETRY_DIR` wins when set to a non-empty value. Relative paths
    /// are interpreted relative to `workspace_root`.
    #[must_use]
    pub fn effective_artifact_dir(&self, workspace_root: &Path) -> PathBuf {
        self.effective_artifact_dir_with_env(workspace_root, std::env::var_os(TELEMETRY_DIR_ENV))
    }

    fn effective_artifact_dir_with_env(
        &self,
        workspace_root: &Path,
        env_override: Option<OsString>,
    ) -> PathBuf {
        let path = env_override
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.configured_artifact_dir().to_path_buf());

        if path.is_absolute() {
            path
        } else {
            workspace_root.join(path)
        }
    }

    /// Resolves and validates telemetry config for a traced build.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryValidationError`] when local telemetry config is
    /// invalid for traced build behavior.
    #[must_use = "telemetry validation errors should be handled"]
    pub fn resolve_for_trace(
        &self,
        workspace_root: &Path,
    ) -> Result<ResolvedTelemetryConfig, TelemetryValidationError> {
        self.resolve_for_trace_with_env(workspace_root, std::env::var_os(TELEMETRY_DIR_ENV))
    }

    fn resolve_for_trace_with_env(
        &self,
        workspace_root: &Path,
        env_override: Option<OsString>,
    ) -> Result<ResolvedTelemetryConfig, TelemetryValidationError> {
        if self.effective_capture_values() {
            return Err(TelemetryValidationError::CaptureValuesUnsupported);
        }

        let sample_rate = self.configured_sample_rate();
        if !sample_rate.is_finite() || !(0.0..=1.0).contains(&sample_rate) {
            return Err(TelemetryValidationError::InvalidSampleRate { value: sample_rate });
        }

        let sampling_mode = TelemetrySamplingMode::parse(self.configured_sampling_mode())?;
        let (artifact_dir, artifact_dir_overridden) =
            match env_override.filter(|value| !value.is_empty()) {
                Some(value) => (PathBuf::from(value), true),
                None => (self.configured_artifact_dir().to_path_buf(), false),
            };
        let artifact_dir = resolve_artifact_dir(workspace_root, &artifact_dir)?;

        Ok(ResolvedTelemetryConfig {
            enabled: self.effective_enabled(),
            sampling_mode,
            sample_rate,
            artifact_dir,
            artifact_dir_overridden,
            capture_values: self.effective_capture_values(),
        })
    }
}

fn resolve_artifact_dir(
    workspace_root: &Path,
    configured: &Path,
) -> Result<PathBuf, TelemetryValidationError> {
    if configured.as_os_str().is_empty() {
        return Err(TelemetryValidationError::InvalidArtifactDir {
            reason: "path is empty".to_string(),
        });
    }

    if configured
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(TelemetryValidationError::InvalidArtifactDir {
            reason: "parent directory traversal is not allowed".to_string(),
        });
    }

    if configured.is_absolute() {
        return Ok(configured.to_path_buf());
    }

    if configured
        .components()
        .any(|component| matches!(component, Component::Prefix(_)))
    {
        return Err(TelemetryValidationError::InvalidArtifactDir {
            reason: "relative path prefixes are not allowed".to_string(),
        });
    }

    Ok(workspace_root.join(configured))
}

fn entries_for_graph(graph: &SemanticGraph) -> Result<Vec<TraceMapEntry>, TelemetryError> {
    let mut entries = Vec::new();

    for function in &graph.functions {
        let function_graph_id = function_trace_graph_id(graph, function)?;
        entries.push(TraceMapEntry {
            trace_id: trace_id(TraceMapKind::Function, &function_graph_id),
            kind: TraceMapKind::Function,
            graph_id: function_graph_id,
            module: graph.module_name.0.clone(),
            function: function.name.0.clone(),
            block: None,
        });

        for block in &function.blocks {
            let block_graph_id = block_trace_graph_id(graph, function, block)?;
            entries.push(TraceMapEntry {
                trace_id: trace_id(TraceMapKind::Block, &block_graph_id),
                kind: TraceMapKind::Block,
                graph_id: block_graph_id,
                module: graph.module_name.0.clone(),
                function: function.name.0.clone(),
                block: Some(block.label.0.clone()),
            });
        }
    }

    Ok(entries)
}

/// Returns the graph identity used for a function trace entry.
///
/// # Errors
///
/// Returns [`TelemetryError::MissingTraceGraphIdentity`] when no block node can
/// be mapped back to a stable function graph identity.
pub fn function_trace_graph_id(
    graph: &SemanticGraph,
    function: &FunctionInfo,
) -> Result<String, TelemetryError> {
    function
        .blocks
        .iter()
        .find_map(|block| graph_id_from_first_node(graph, block, 2))
        .ok_or_else(|| missing_trace_graph_identity(graph, function, None, TraceMapKind::Function))
}

/// Returns the graph identity used for a block trace entry.
///
/// # Errors
///
/// Returns [`TelemetryError::MissingTraceGraphIdentity`] when the block cannot
/// be mapped back to a stable graph block identity.
pub fn block_trace_graph_id(
    graph: &SemanticGraph,
    function: &FunctionInfo,
    block: &BlockInfo,
) -> Result<String, TelemetryError> {
    graph_id_from_first_node(graph, block, 1).ok_or_else(|| {
        missing_trace_graph_identity(graph, function, Some(block), TraceMapKind::Block)
    })
}

fn missing_trace_graph_identity(
    graph: &SemanticGraph,
    function: &FunctionInfo,
    block: Option<&BlockInfo>,
    kind: TraceMapKind,
) -> TelemetryError {
    let block = block.map(|block| block.label.0.clone());
    let block_context = block
        .as_ref()
        .map_or_else(String::new, |label| format!(", block '{label}'"));
    TelemetryError::MissingTraceGraphIdentity {
        kind,
        module: graph.module_name.0.clone(),
        function: function.name.0.clone(),
        block,
        block_context,
    }
}

fn graph_id_from_first_node(
    graph: &SemanticGraph,
    block: &BlockInfo,
    parent_segments: usize,
) -> Option<String> {
    let node_id = block
        .nodes
        .first()
        .and_then(|node_index| graph.graph.node_weight(*node_index))
        .map(|node| node.id.0.as_str())?;

    parent_graph_id(node_id, parent_segments)
}

fn parent_graph_id(node_id: &str, parent_segments: usize) -> Option<String> {
    let mut end = node_id.len();
    for _ in 0..parent_segments {
        end = node_id[..end].rfind('/')?;
    }
    Some(node_id[..end].to_string())
}

fn check_trace_id_collisions(entries: &[TraceMapEntry]) -> Result<(), TelemetryError> {
    let mut seen: HashMap<u64, (TraceMapKind, &str)> = HashMap::new();
    for entry in entries {
        if let Some((existing_kind, existing_graph_id)) =
            seen.insert(entry.trace_id, (entry.kind, &entry.graph_id))
            && (existing_kind != entry.kind || existing_graph_id != entry.graph_id)
        {
            return Err(TelemetryError::TraceIdCollision {
                trace_id: entry.trace_id,
                existing_kind,
                existing_graph_id: existing_graph_id.to_string(),
                new_kind: entry.kind,
                new_graph_id: entry.graph_id.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn telemetry_defaults_are_local_and_off() {
        let section = TelemetrySection::default();

        assert!(!section.effective_enabled());
        assert!(!section.effective_capture_values());
        assert_eq!(section.configured_sampling_mode(), "deterministic");
        assert_eq!(section.configured_sample_rate(), 0.0);
        assert_eq!(
            section.configured_artifact_dir(),
            Path::new(DEFAULT_ARTIFACT_DIR)
        );
    }

    #[test]
    fn build_options_default_to_uninstrumented() {
        let options = BuildOptions::default();

        assert!(!options.offline);
        assert_eq!(options.telemetry, TelemetryBuildMode::Off);
        assert!(!options.telemetry.is_trace());
    }

    #[test]
    fn build_options_can_select_trace_mode() {
        let options = BuildOptions::new(true, TelemetryBuildMode::Trace);

        assert!(options.offline);
        assert!(options.telemetry.is_trace());
    }

    #[test]
    fn trace_ids_are_deterministic_and_int64_compatible() {
        let first = trace_id(TraceMapKind::Function, "duumbi:t/main");
        let second = trace_id(TraceMapKind::Function, "duumbi:t/main");
        let different_kind = trace_id(TraceMapKind::Block, "duumbi:t/main");

        assert_eq!(first, second);
        assert_ne!(first, different_kind);
        assert_eq!(first & 0x8000_0000_0000_0000, 0);
    }

    #[test]
    fn trace_map_entries_sort_by_kind_then_graph_id() {
        let map = TraceMap::new(vec![
            TraceMapEntry {
                trace_id: 3,
                kind: TraceMapKind::Function,
                graph_id: "duumbi:t/z".to_string(),
                module: "t".to_string(),
                function: "z".to_string(),
                block: None,
            },
            TraceMapEntry {
                trace_id: 1,
                kind: TraceMapKind::Block,
                graph_id: "duumbi:t/a/entry".to_string(),
                module: "t".to_string(),
                function: "a".to_string(),
                block: Some("entry".to_string()),
            },
            TraceMapEntry {
                trace_id: 2,
                kind: TraceMapKind::Function,
                graph_id: "duumbi:t/a".to_string(),
                module: "t".to_string(),
                function: "a".to_string(),
                block: None,
            },
        ])
        .expect("trace map entries should not collide");

        let ordered: Vec<&str> = map
            .entries
            .iter()
            .map(|entry| entry.graph_id.as_str())
            .collect();
        assert_eq!(
            ordered,
            vec!["duumbi:t/a/entry", "duumbi:t/a", "duumbi:t/z"]
        );
    }

    #[test]
    fn trace_map_rejects_colliding_ids() {
        let result = TraceMap::new(vec![
            TraceMapEntry {
                trace_id: 42,
                kind: TraceMapKind::Function,
                graph_id: "duumbi:t/main".to_string(),
                module: "t".to_string(),
                function: "main".to_string(),
                block: None,
            },
            TraceMapEntry {
                trace_id: 42,
                kind: TraceMapKind::Block,
                graph_id: "duumbi:t/main/entry".to_string(),
                module: "t".to_string(),
                function: "main".to_string(),
                block: Some("entry".to_string()),
            },
        ]);

        assert!(matches!(
            result,
            Err(TelemetryError::TraceIdCollision { .. })
        ));
    }

    #[test]
    fn trace_map_from_graph_contains_function_and_block_ids() {
        let graph = test_graph();
        let map = TraceMap::from_graph(&graph).expect("trace map should be generated");

        assert!(map.entries.iter().any(|entry| {
            entry.kind == TraceMapKind::Function
                && entry.graph_id == "duumbi:t/main"
                && entry.function == "main"
                && entry.block.is_none()
        }));
        assert!(map.entries.iter().any(|entry| {
            entry.kind == TraceMapKind::Block
                && entry.graph_id == "duumbi:t/main/entry"
                && entry.function == "main"
                && entry.block.as_deref() == Some("entry")
        }));
    }

    #[test]
    fn trace_map_from_graph_rejects_missing_function_identity() {
        let mut graph = test_graph();
        for (index, node) in graph.graph.node_weights_mut().enumerate() {
            node.id = crate::types::NodeId(format!("node-{index}"));
        }

        let err = TraceMap::from_graph(&graph).expect_err("trace map should require graph IDs");

        assert!(matches!(
            err,
            TelemetryError::MissingTraceGraphIdentity {
                kind: TraceMapKind::Function,
                ..
            }
        ));
        assert!(err.to_string().contains("missing function graph identity"));
    }

    #[test]
    fn trace_map_from_graph_rejects_missing_block_identity() {
        let mut graph = test_graph();
        graph.functions[0].blocks.push(crate::graph::BlockInfo {
            label: crate::types::BlockLabel("empty".to_string()),
            nodes: vec![],
        });

        let err = TraceMap::from_graph(&graph).expect_err("trace map should require block IDs");

        assert!(matches!(
            err,
            TelemetryError::MissingTraceGraphIdentity {
                kind: TraceMapKind::Block,
                block: Some(_),
                ..
            }
        ));
        assert!(err.to_string().contains("block 'empty'"));
    }

    #[test]
    fn write_trace_map_creates_artifact() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");

        let path = write_trace_map(&map, dir.path()).expect("trace map should be written");
        let content = std::fs::read_to_string(path).expect("trace map should be readable");

        assert!(content.contains(TRACE_MAP_SCHEMA_VERSION));
        assert!(content.contains("duumbi:t/main/entry"));
    }

    #[test]
    fn inspect_crash_artifacts_maps_function_and_block() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        write_test_crash(&dir, &map, "called Option::unwrap() on a None value");

        let report = inspect_crash_artifacts(dir.path(), None, None)
            .expect("crash artifacts should map to graph context");

        assert_eq!(report.function_graph_id, "duumbi:t/main");
        assert_eq!(report.block_graph_id, "duumbi:t/main/entry");
        assert!(
            report
                .to_cli_output()
                .contains("Exact node evidence: unavailable in v1")
        );
    }

    #[test]
    fn repair_crash_context_is_serializable_mapped_evidence() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let (function_trace_id, block_trace_id) =
            write_test_crash(&dir, &map, "called Option::unwrap() on a None value");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source.clone());

        let context = repair_crash_context(&options)
            .expect("repair context should be built from mapped evidence");

        assert_eq!(context.schema_version, REPAIR_CONTEXT_SCHEMA_VERSION);
        assert_eq!(
            context.crash_message,
            "called Option::unwrap() on a None value"
        );
        assert_eq!(context.function_id, "duumbi:t/main");
        assert_eq!(context.block_id, "duumbi:t/main/entry");
        assert_eq!(context.exact_node_id, None);
        assert_eq!(context.trace_ids.function_trace_id, function_trace_id);
        assert_eq!(context.trace_ids.block_trace_id, block_trace_id);
        assert!(context.human_review_required);
        assert_eq!(context.evidence.source, "local_telemetry_artifacts");
        assert_eq!(context.evidence.selected_crash_line, 1);
        assert_eq!(context.evidence.selection, "latest");
        assert_eq!(
            context.evidence.graph_sources,
            vec![graph_source.display().to_string()]
        );
        assert_eq!(
            context.graph_context["source"],
            serde_json::json!("mapped_graph_source")
        );
        assert_eq!(
            context.graph_context["block"]["graph_id"],
            serde_json::json!("duumbi:t/main/entry")
        );
        assert_eq!(
            context.graph_context["context_limit"],
            serde_json::json!("containing_function_and_selected_block")
        );
        assert_eq!(
            context.graph_context["exact_node_evidence"],
            serde_json::Value::Null
        );
        assert_eq!(
            context.graph_context["function_context"]["duumbi:blocks"]
                .as_array()
                .expect("bounded function context should include blocks")
                .len(),
            1
        );
        assert!(
            context
                .validation_expectations
                .iter()
                .any(|expectation| expectation == "proposed patch parses as GraphPatch")
        );
        assert!(
            context
                .test_expectations
                .iter()
                .any(|expectation| expectation
                    == "default untraced build behavior remains unchanged")
        );

        let serialized = serde_json::to_string(&context).expect("repair context should serialize");
        assert!(!serialized.contains("argument"));
        assert!(!serialized.contains("heap"));
        assert!(!serialized.contains("stack"));
        assert!(!serialized.contains("snapshot"));
        let roundtrip: RepairCrashContext =
            serde_json::from_str(&serialized).expect("repair context should deserialize");
        assert_eq!(roundtrip, context);
    }

    #[test]
    #[allow(deprecated)]
    fn repair_crash_context_legacy_helper_requires_graph_sources() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");

        let err = repair_crash_context_from_artifacts(dir.path(), None, None)
            .expect_err("legacy helper should not emit trace-map-only context");

        assert!(matches!(err, TelemetryError::MissingEvidence(_)));
        assert!(err.to_string().contains("graph source"));
    }

    #[test]
    fn repair_crash_context_selects_latest_non_empty_crash_entry() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        write_test_crash_entries(&dir, &map, &["first crash", "second crash"]);
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let context = repair_crash_context(&options).expect("latest crash should map");

        assert_eq!(context.crash_message, "second crash");
        assert_eq!(context.evidence.selected_crash_line, 3);
        assert_eq!(context.evidence.selection, "latest");
    }

    #[test]
    fn repair_crash_context_selects_explicit_crash_line() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        write_test_crash_entries(&dir, &map, &["first crash", "second crash"]);
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);
        options.crash_entry = CrashEntrySelection::LineNumber(1);

        let context = repair_crash_context(&options).expect("explicit crash line should map");

        assert_eq!(context.crash_message, "first crash");
        assert_eq!(context.evidence.selected_crash_line, 1);
        assert_eq!(context.evidence.selection, "line:1");
    }

    #[test]
    fn repair_crash_context_rejects_missing_graph_source() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        write_test_crash(&dir, &map, "missing graph source");
        let options = RepairContextOptions::new(dir.path());

        let err = repair_crash_context(&options).expect_err("graph source should be required");

        assert!(matches!(err, TelemetryError::MissingEvidence(_)));
        assert!(err.to_string().contains("graph source"));
    }

    #[test]
    fn repair_crash_context_rejects_missing_crash_artifact() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("missing crash artifact should fail");

        assert!(
            matches!(err, TelemetryError::Io { context, .. } if context.contains("Failed to read crash artifact"))
        );
    }

    #[test]
    fn repair_crash_context_rejects_malformed_crash_json() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        std::fs::write(dir.path().join(CRASH_DUMP_FILE), "{not json}\n")
            .expect("malformed crash artifact should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("malformed crash JSON should fail");

        assert!(
            matches!(err, TelemetryError::Parse { path, .. } if path.ends_with("crash_dump.jsonl:1"))
        );
    }

    #[test]
    fn repair_crash_context_rejects_missing_trace_map() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_test_crash(&dir, &map, "missing trace map");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("missing trace map should fail");

        assert!(
            matches!(err, TelemetryError::Io { context, .. } if context.contains("Failed to read trace map"))
        );
    }

    #[test]
    fn repair_crash_context_rejects_malformed_trace_map() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_test_crash(&dir, &map, "malformed trace map");
        std::fs::write(dir.path().join(TRACE_MAP_FILE), "{not json}")
            .expect("malformed trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("malformed trace map should fail");

        assert!(
            matches!(err, TelemetryError::Parse { path, .. } if path.ends_with(TRACE_MAP_FILE))
        );
    }

    #[test]
    fn repair_crash_context_rejects_untraced_crash_artifact() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let (function_trace_id, block_trace_id) = test_trace_ids(&map);
        write_test_crash_with_ids(
            &dir,
            "untraced crash",
            function_trace_id,
            block_trace_id,
            false,
        );
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("untraced crash should fail");

        assert!(
            matches!(err, TelemetryError::MissingEvidence(message) if message.contains("active traced run"))
        );
    }

    #[test]
    fn repair_crash_context_rejects_unmapped_function_trace_id() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let (_, block_trace_id) = test_trace_ids(&map);
        write_test_crash_with_ids(&dir, "unmapped function", u64::MAX, block_trace_id, true);
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("unmapped function ID should fail");

        assert!(
            matches!(err, TelemetryError::Unmapped(message) if message.contains("function trace ID"))
        );
    }

    #[test]
    fn repair_crash_context_rejects_unmapped_block_trace_id() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let (function_trace_id, _) = test_trace_ids(&map);
        write_test_crash_with_ids(&dir, "unmapped block", function_trace_id, u64::MAX, true);
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("unmapped block ID should fail");

        assert!(
            matches!(err, TelemetryError::Unmapped(message) if message.contains("block trace ID"))
        );
    }

    #[test]
    fn repair_crash_context_rejects_stale_graph_ids() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        write_test_crash(&dir, &map, "stale graph");
        let graph_source = write_test_graph_source(
            &dir,
            r#"{
                "@type": "duumbi:Module",
                "@id": "duumbi:other",
                "duumbi:functions": []
            }"#,
            "stale.jsonld",
        );
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("stale graph IDs should fail");

        assert!(matches!(err, TelemetryError::Unmapped(_)));
        assert!(err.to_string().contains("stale or missing function"));
    }

    #[test]
    fn repair_crash_context_rejects_duplicate_graph_ids_in_one_source() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        write_test_crash(&dir, &map, "duplicate graph");
        let mut graph_source: serde_json::Value =
            serde_json::from_str(test_graph_source()).expect("graph source should parse");
        let functions = graph_source["duumbi:functions"]
            .as_array_mut()
            .expect("test source should include function array");
        functions.push(functions[0].clone());
        let graph_source = write_test_graph_source(
            &dir,
            &serde_json::to_string_pretty(&graph_source).expect("graph should serialize"),
            "duplicate.jsonld",
        );
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("duplicate graph IDs should fail");

        assert!(matches!(err, TelemetryError::Unmapped(_)));
        assert!(err.to_string().contains("ambiguous graph source"));
    }

    #[test]
    fn repair_crash_context_rejects_duplicate_block_ids_in_one_function() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        write_test_crash(&dir, &map, "duplicate block");
        let mut graph_source: serde_json::Value =
            serde_json::from_str(test_graph_source()).expect("graph source should parse");
        let functions = graph_source["duumbi:functions"]
            .as_array_mut()
            .expect("test source should include function array");
        let blocks = functions[0]["duumbi:blocks"]
            .as_array_mut()
            .expect("test source should include block array");
        blocks.push(blocks[0].clone());
        let graph_source = write_test_graph_source(
            &dir,
            &serde_json::to_string_pretty(&graph_source).expect("graph should serialize"),
            "duplicate-block.jsonld",
        );
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);

        let err = repair_crash_context(&options).expect_err("duplicate block IDs should fail");

        assert!(matches!(err, TelemetryError::Unmapped(_)));
        assert!(err.to_string().contains("ambiguous graph source"));
    }

    #[test]
    fn repair_crash_context_rejects_duplicate_block_ids_across_sources() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        write_test_crash(&dir, &map, "duplicate block across sources");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let duplicate_block_source = write_test_graph_source(
            &dir,
            r#"{
                "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
                "@type": "duumbi:Module",
                "@id": "duumbi:duplicate",
                "duumbi:functions": [{
                    "@type": "duumbi:Function",
                    "@id": "duumbi:duplicate/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "void",
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:t/main/entry",
                        "duumbi:name": "entry",
                        "duumbi:operations": []
                    }]
                }]
            }"#,
            "duplicate-block-source.jsonld",
        );
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);
        options.graph_sources.push(duplicate_block_source);

        let err =
            repair_crash_context(&options).expect_err("duplicate block IDs across sources fail");

        assert!(matches!(err, TelemetryError::Unmapped(_)));
        assert!(err.to_string().contains("ambiguous graph source"));
    }

    #[test]
    fn repair_validation_evidence_keeps_human_review_required() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        write_test_crash(&dir, &map, "candidate repair");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);
        let context = repair_crash_context(&options).expect("repair context should be built");
        let patch = crate::patch::GraphPatch {
            ops: vec![crate::patch::PatchOp::ModifyOp {
                node_id: "duumbi:t/main/entry/0".to_string(),
                field: "duumbi:value".to_string(),
                value: serde_json::json!(1),
            }],
        };
        let gates = required_repair_validation_gates()
            .into_iter()
            .map(|gate| RepairValidationGateEvidence::new(gate, true, "passed", None))
            .collect();

        let evidence = repair_validation_evidence_from_graph_patch(context, &patch, gates)
            .expect("repair validation evidence should serialize the patch");

        assert!(evidence.has_required_gates());
        assert!(evidence.local_validation_passed);
        assert!(evidence.requires_human_review);
        assert!(!evidence.accepted_for_application);
        assert_eq!(evidence.human_review_state, RepairHumanReviewState::Pending);
        parse_repair_graph_patch(&evidence.proposed_patch)
            .expect("evidence patch should parse through GraphPatch");
    }

    #[test]
    fn repair_validation_evidence_requires_all_gates_to_pass() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        write_test_crash(&dir, &map, "candidate repair");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);
        let context = repair_crash_context(&options).expect("repair context should be built");
        let patch = serde_json::json!({ "ops": [] });
        let gates = vec![RepairValidationGateEvidence::new(
            RepairValidationGate::GraphPatchParse,
            true,
            "parsed",
            None,
        )];

        let evidence = RepairValidationEvidence::new(context, patch, gates);

        assert!(!evidence.has_required_gates());
        assert!(!evidence.local_validation_passed);
        assert!(evidence.requires_human_review);
        assert!(!evidence.accepted_for_application);
        assert_eq!(
            evidence.human_review_state,
            RepairHumanReviewState::NotReady
        );
    }

    #[test]
    fn repair_validation_required_gates_expose_graph_build_separately() {
        let gates = required_repair_validation_gates();

        assert_eq!(
            gates,
            vec![
                RepairValidationGate::GraphPatchParse,
                RepairValidationGate::AtomicPatchApplication,
                RepairValidationGate::GraphParse,
                RepairValidationGate::GraphBuild,
                RepairValidationGate::GraphValidation,
                RepairValidationGate::NativeRebuild,
                RepairValidationGate::RelevantTests,
            ]
        );
    }

    #[test]
    fn repair_validation_evidence_serializes_reviewable_schema_fields() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        write_test_crash(&dir, &map, "candidate repair");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source.clone());
        let context = repair_crash_context(&options).expect("repair context should be built");
        let patch = serde_json::json!({
            "ops": [{
                "kind": "modify_op",
                "node_id": "duumbi:t/main/entry/0",
                "field": "duumbi:value",
                "value": 1
            }]
        });
        let gates = required_repair_validation_gates()
            .into_iter()
            .map(|gate| match gate {
                RepairValidationGate::NativeRebuild => RepairValidationGateEvidence::with_command(
                    gate,
                    true,
                    "passed",
                    Some("duumbi build candidate".to_string()),
                    Some("build output".to_string()),
                ),
                RepairValidationGate::RelevantTests => RepairValidationGateEvidence::with_command(
                    gate,
                    true,
                    "passed",
                    Some("{candidate_binary}".to_string()),
                    Some("test output".to_string()),
                ),
                _ => RepairValidationGateEvidence::new(gate, true, "passed", None),
            })
            .collect();

        let evidence = RepairValidationEvidence::new(context, patch, gates);
        let serialized =
            serde_json::to_value(&evidence).expect("repair validation evidence should serialize");

        assert_eq!(
            serialized["schema_version"],
            REPAIR_VALIDATION_SCHEMA_VERSION
        );
        assert_eq!(serialized["source_artifact"]["kind"], "graph_sources");
        assert_eq!(
            serialized["source_artifact"]["paths"][0],
            graph_source.display().to_string()
        );
        assert_eq!(serialized["patch_summary"]["operation_count"], 1);
        assert_eq!(serialized["rebuild_summary"]["status"], "passed");
        assert_eq!(
            serialized["rebuild_summary"]["command"],
            "duumbi build candidate"
        );
        assert_eq!(serialized["rebuild_summary"]["output"], "build output");
        assert_eq!(serialized["test_summary"]["status"], "passed");
        assert_eq!(
            serialized["test_summary"]["commands"][0],
            "{candidate_binary}"
        );
        assert_eq!(serialized["test_summary"]["output"], "test output");
        assert_eq!(serialized["human_review_state"], "pending");
        assert_eq!(serialized["requires_human_review"], true);
        assert_eq!(serialized["accepted_for_application"], false);
    }

    #[test]
    fn repair_validation_graph_build_failure_is_reviewable_not_ready() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        write_test_crash(&dir, &map, "candidate repair");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source);
        let context = repair_crash_context(&options).expect("repair context should be built");
        let patch = serde_json::json!({ "ops": [] });
        let gates = vec![
            RepairValidationGateEvidence::new(
                RepairValidationGate::GraphPatchParse,
                true,
                "parsed",
                None,
            ),
            RepairValidationGateEvidence::new(
                RepairValidationGate::AtomicPatchApplication,
                true,
                "applied to clone",
                None,
            ),
            RepairValidationGateEvidence::new(
                RepairValidationGate::GraphParse,
                true,
                "parsed",
                None,
            ),
            RepairValidationGateEvidence::new(
                RepairValidationGate::GraphBuild,
                false,
                "builder rejected patched graph",
                Some("missing entry block".to_string()),
            ),
        ];

        let evidence = RepairValidationEvidence::new(context, patch, gates);

        assert!(!evidence.has_required_gates());
        assert!(!evidence.local_validation_passed);
        assert_eq!(
            evidence.human_review_state,
            RepairHumanReviewState::NotReady
        );
        assert_eq!(
            evidence
                .gates
                .iter()
                .find(|gate| gate.gate == RepairValidationGate::GraphBuild)
                .expect("graph build gate should be present")
                .summary,
            "builder rejected patched graph"
        );
    }

    #[test]
    fn validate_repair_candidate_reports_malformed_patch_before_application() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let request = RepairValidationRequest::single_graph(
            context,
            serde_json::json!({ "ops": [{ "kind": "unsupported" }] }),
            graph_source,
        );

        let evidence =
            validate_repair_candidate(request).expect("malformed patch should return evidence");

        assert!(!evidence.local_validation_passed);
        assert_eq!(evidence.gates.len(), 1);
        assert_eq!(
            evidence.gates[0].gate,
            RepairValidationGate::GraphPatchParse
        );
        assert!(!evidence.gates[0].passed);
        assert!(
            !evidence
                .gates
                .iter()
                .any(|gate| gate.gate == RepairValidationGate::AtomicPatchApplication)
        );
    }

    #[test]
    fn validate_repair_candidate_reports_selected_source_artifact() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, context_graph_source) = test_repair_context_and_source(&dir);
        let selected_graph_source =
            write_test_graph_source(&dir, test_graph_source(), "selected.jsonld");
        let request = RepairValidationRequest::single_graph(
            context,
            serde_json::json!({ "ops": [{ "kind": "unsupported" }] }),
            selected_graph_source.clone(),
        );

        let evidence =
            validate_repair_candidate(request).expect("malformed patch should return evidence");

        assert_eq!(
            evidence.source_artifact.paths,
            vec![selected_graph_source.display().to_string()]
        );
        assert_ne!(
            evidence.source_artifact.paths,
            vec![context_graph_source.display().to_string()]
        );
    }

    #[test]
    fn repair_context_graph_source_match_accepts_same_or_canonical_path() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let canonical_graph_source =
            std::fs::canonicalize(&graph_source).expect("graph source should canonicalize");

        assert!(repair_context_includes_graph_source(
            &context,
            &graph_source
        ));
        assert!(repair_context_includes_graph_source(
            &context,
            &canonical_graph_source
        ));
        assert!(!repair_context_includes_graph_source(
            &context,
            &dir.path().join("other.jsonld")
        ));
    }

    #[test]
    fn repair_test_summary_splits_multiple_commands() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, _) = test_repair_context_and_source(&dir);
        let gates = vec![RepairValidationGateEvidence::with_command(
            RepairValidationGate::RelevantTests,
            true,
            "candidate-aware tests passed",
            Some("first command\n\n second command ".to_string()),
            Some("test output".to_string()),
        )];

        let evidence =
            RepairValidationEvidence::new(context, serde_json::json!({ "ops": [] }), gates);

        assert_eq!(
            evidence.test_summary.commands,
            vec!["first command".to_string(), "second command".to_string()]
        );
    }

    #[test]
    fn validate_repair_candidate_preserves_source_on_patch_application_failure() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let original_bytes = std::fs::read(&graph_source).expect("source should be readable");
        let request = RepairValidationRequest::single_graph(
            context,
            serde_json::json!({
                "ops": [{
                    "kind": "modify_op",
                    "node_id": "duumbi:t/main/entry/missing",
                    "field": "duumbi:value",
                    "value": 1
                }]
            }),
            graph_source.clone(),
        );

        let evidence =
            validate_repair_candidate(request).expect("patch failure should return evidence");
        let after_bytes = std::fs::read(&graph_source).expect("source should remain readable");

        assert_eq!(after_bytes, original_bytes);
        assert!(!evidence.local_validation_passed);
        assert_eq!(
            evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .gate,
            RepairValidationGate::AtomicPatchApplication
        );
        assert!(
            !evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .passed
        );
    }

    #[test]
    fn validate_repair_candidate_reports_graph_build_failure_distinctly() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let request = RepairValidationRequest::single_graph(
            context,
            serde_json::json!({
                "ops": [{
                    "kind": "remove_node",
                    "node_id": "duumbi:t/main"
                }]
            }),
            graph_source,
        );

        let evidence =
            validate_repair_candidate(request).expect("graph build failure should return evidence");

        assert!(!evidence.local_validation_passed);
        assert_eq!(
            evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .gate,
            RepairValidationGate::GraphBuild
        );
        assert!(
            !evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .passed
        );
    }

    #[test]
    fn validate_repair_candidate_reports_graph_validation_failure() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let request = RepairValidationRequest::single_graph(
            context,
            serde_json::json!({
                "ops": [{
                    "kind": "add_op",
                    "block_id": "duumbi:t/main/entry",
                    "op": {
                        "@type": "duumbi:Const",
                        "@id": "duumbi:t/main/entry/2",
                        "duumbi:value": 2,
                        "duumbi:resultType": "i64"
                    }
                }]
            }),
            graph_source,
        );

        let evidence = validate_repair_candidate(request)
            .expect("graph validation failure should return evidence");

        assert!(!evidence.local_validation_passed);
        assert_eq!(
            evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .gate,
            RepairValidationGate::GraphValidation
        );
        assert!(
            !evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .passed
        );
    }

    #[test]
    fn validate_repair_candidate_passes_parse_build_validate_but_awaits_later_gates() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let request = RepairValidationRequest::single_graph(
            context,
            serde_json::json!({
                "ops": [{
                    "kind": "modify_op",
                    "node_id": "duumbi:t/main/entry/0",
                    "field": "duumbi:value",
                    "value": 1
                }]
            }),
            graph_source,
        );

        let evidence =
            validate_repair_candidate(request).expect("valid early gates should return evidence");

        assert!(!evidence.local_validation_passed);
        assert!(!evidence.has_required_gates());
        assert!(
            evidence
                .gates
                .iter()
                .any(|gate| gate.gate == RepairValidationGate::GraphValidation && gate.passed)
        );
        assert!(
            !evidence
                .gates
                .iter()
                .any(|gate| gate.gate == RepairValidationGate::NativeRebuild)
        );
        assert_eq!(
            evidence.human_review_state,
            RepairHumanReviewState::NotReady
        );
    }

    #[test]
    fn validate_repair_candidate_runner_reports_native_rebuild_failure() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let request =
            RepairValidationRequest::single_graph(context, valid_modify_patch(), graph_source);
        let mut runner = TestRepairValidationRunner {
            rebuild_passes: false,
            tests_pass: true,
        };

        let evidence = validate_repair_candidate_with_runner(request, &mut runner)
            .expect("rebuild failure should return evidence");

        assert!(!evidence.local_validation_passed);
        assert_eq!(
            evidence.rebuild_summary.status,
            RepairValidationStepStatus::Failed
        );
        assert_eq!(
            evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .gate,
            RepairValidationGate::NativeRebuild
        );
        assert!(
            !evidence
                .gates
                .iter()
                .any(|gate| gate.gate == RepairValidationGate::RelevantTests)
        );
    }

    #[test]
    fn validate_repair_candidate_runner_does_not_rebuild_after_validation_failure() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let request = RepairValidationRequest::single_graph(
            context,
            serde_json::json!({
                "ops": [{
                    "kind": "add_op",
                    "block_id": "duumbi:t/main/entry",
                    "op": {
                        "@type": "duumbi:Const",
                        "@id": "duumbi:t/main/entry/2",
                        "duumbi:value": 2,
                        "duumbi:resultType": "i64"
                    }
                }]
            }),
            graph_source,
        );
        let mut runner = TestRepairValidationRunner {
            rebuild_passes: true,
            tests_pass: true,
        };

        let evidence = validate_repair_candidate_with_runner(request, &mut runner)
            .expect("graph validation failure should return evidence");

        assert_eq!(
            evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .gate,
            RepairValidationGate::GraphValidation
        );
        assert!(
            !evidence
                .gates
                .iter()
                .any(|gate| gate.gate == RepairValidationGate::NativeRebuild)
        );
    }

    #[test]
    fn validate_repair_candidate_runner_reports_relevant_test_failure() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let request =
            RepairValidationRequest::single_graph(context, valid_modify_patch(), graph_source);
        let mut runner = TestRepairValidationRunner {
            rebuild_passes: true,
            tests_pass: false,
        };

        let evidence = validate_repair_candidate_with_runner(request, &mut runner)
            .expect("test failure should return evidence");

        assert!(!evidence.local_validation_passed);
        assert_eq!(
            evidence.rebuild_summary.status,
            RepairValidationStepStatus::Passed
        );
        assert_eq!(
            evidence.test_summary.status,
            RepairValidationStepStatus::Failed
        );
        assert_eq!(
            evidence
                .gates
                .last()
                .expect("failed gate should be present")
                .gate,
            RepairValidationGate::RelevantTests
        );
    }

    #[test]
    fn validate_repair_candidate_runner_allows_local_pass_but_not_acceptance() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let (context, graph_source) = test_repair_context_and_source(&dir);
        let request =
            RepairValidationRequest::single_graph(context, valid_modify_patch(), graph_source);
        let mut runner = TestRepairValidationRunner {
            rebuild_passes: true,
            tests_pass: true,
        };

        let evidence = validate_repair_candidate_with_runner(request, &mut runner)
            .expect("all local gates should return evidence");

        assert!(evidence.has_required_gates());
        assert!(evidence.local_validation_passed);
        assert_eq!(
            evidence.rebuild_summary.status,
            RepairValidationStepStatus::Passed
        );
        assert_eq!(
            evidence.test_summary.status,
            RepairValidationStepStatus::Passed
        );
        assert_eq!(evidence.test_summary.commands, vec!["{candidate_binary}"]);
        assert!(evidence.requires_human_review);
        assert!(!evidence.accepted_for_application);
        assert_eq!(evidence.human_review_state, RepairHumanReviewState::Pending);
    }

    #[test]
    fn command_runner_rejects_generic_relevant_test_command() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut runner = RepairValidationCommandRunner::with_commands(
            graph_source,
            vec![test_success_command(), "{candidate_graph}".to_string()],
            vec![test_success_command()],
        )
        .expect("command runner should be created");
        let rebuild = runner.rebuild_candidate(test_graph_source());
        assert!(rebuild.passed);

        let tests = runner.run_relevant_tests(test_graph_source(), &rebuild);

        assert!(!tests.passed);
        assert_eq!(
            tests.summary,
            "relevant test command is not candidate-aware"
        );
    }

    #[test]
    fn command_runner_runs_candidate_aware_test_command() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let graph_source = write_test_graph_source(&dir, test_graph_source(), "graph.jsonld");
        let mut runner = RepairValidationCommandRunner::with_commands(
            graph_source,
            vec![test_success_command(), "{candidate_graph}".to_string()],
            vec![test_success_command_template()],
        )
        .expect("command runner should be created");

        let rebuild = runner.rebuild_candidate(test_graph_source());
        let tests = runner.run_relevant_tests(test_graph_source(), &rebuild);

        assert!(rebuild.passed);
        assert!(tests.passed);
        assert!(
            tests
                .command
                .expect("test summary should include candidate-aware command")
                .contains("candidate.jsonld")
        );
    }

    #[test]
    fn repair_graph_patch_parse_rejects_invalid_patch() {
        let invalid = serde_json::json!({
            "ops": [{ "kind": "unsupported" }]
        });

        let result = parse_repair_graph_patch(&invalid);

        assert!(matches!(
            result,
            Err(TelemetryError::Parse { path, .. }) if path == "<repair graph patch>"
        ));
    }

    #[test]
    fn inspect_crash_artifacts_rejects_missing_map() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let crash = serde_json::json!({
            "schema_version": CRASH_SCHEMA_VERSION,
            "event": "panic",
            "message": "failure",
            "function_id": 1,
            "block_id": 2,
            "trace_active": true
        });
        std::fs::write(dir.path().join(CRASH_DUMP_FILE), format!("{crash}\n"))
            .expect("crash artifact should be written");

        let result = inspect_crash_artifacts(dir.path(), None, None);

        assert!(matches!(result, Err(TelemetryError::Io { .. })));
    }

    #[test]
    fn inspect_crash_artifacts_rejects_unmapped_crash_ids() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");

        let crash = serde_json::json!({
            "schema_version": CRASH_SCHEMA_VERSION,
            "event": "panic",
            "message": "failure",
            "function_id": u64::MAX,
            "block_id": u64::MAX,
            "trace_active": true
        });
        std::fs::write(dir.path().join(CRASH_DUMP_FILE), format!("{crash}\n"))
            .expect("crash artifact should be written");

        let result = inspect_crash_artifacts(dir.path(), None, None);

        assert!(
            matches!(result, Err(TelemetryError::Unmapped(message)) if message.contains("function trace ID"))
        );
    }

    #[test]
    fn telemetry_artifact_dir_resolves_relative_to_workspace() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let section = TelemetrySection {
            artifact_dir: Some(PathBuf::from("custom/telemetry")),
            ..TelemetrySection::default()
        };

        assert_eq!(
            section.effective_artifact_dir(workspace.path()),
            workspace.path().join("custom/telemetry")
        );
    }

    #[test]
    fn telemetry_env_override_wins() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let override_dir = workspace.path().join("env-telemetry");

        let section = TelemetrySection {
            artifact_dir: Some(PathBuf::from("config-telemetry")),
            ..TelemetrySection::default()
        };

        assert_eq!(
            section.effective_artifact_dir_with_env(
                workspace.path(),
                Some(override_dir.clone().into_os_string())
            ),
            override_dir
        );
    }

    #[test]
    fn telemetry_empty_env_override_falls_back_to_config() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let section = TelemetrySection {
            artifact_dir: Some(PathBuf::from("config-telemetry")),
            ..TelemetrySection::default()
        };

        let resolved = section
            .resolve_for_trace_with_env(workspace.path(), Some(OsString::new()))
            .expect("empty env override should be treated as unset");

        assert_eq!(
            resolved.artifact_dir,
            workspace.path().join("config-telemetry")
        );
        assert!(!resolved.artifact_dir_overridden);
    }

    #[test]
    fn telemetry_absolute_env_override_is_accepted() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let override_dir = workspace.path().join("absolute-telemetry");

        let resolved = TelemetrySection::default()
            .resolve_for_trace_with_env(
                workspace.path(),
                Some(override_dir.as_os_str().to_os_string()),
            )
            .expect("absolute env override should be accepted");

        assert_eq!(resolved.artifact_dir, override_dir);
        assert!(resolved.artifact_dir_overridden);
    }

    #[cfg(windows)]
    #[test]
    fn telemetry_trace_validation_rejects_windows_path_prefixes() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let section = TelemetrySection {
            artifact_dir: Some(PathBuf::from(r"C:telemetry")),
            ..TelemetrySection::default()
        };

        let err = section
            .resolve_for_trace(workspace.path())
            .expect_err("Windows path prefixes must fail");

        assert!(matches!(
            err,
            TelemetryValidationError::InvalidArtifactDir { .. }
        ));
    }

    #[test]
    fn telemetry_trace_validation_accepts_conservative_defaults() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");

        let resolved = TelemetrySection::default()
            .resolve_for_trace(workspace.path())
            .expect("default telemetry config should be valid");

        assert!(!resolved.enabled);
        assert_eq!(resolved.sampling_mode, TelemetrySamplingMode::Deterministic);
        assert_eq!(resolved.sample_rate, 0.0);
        assert_eq!(
            resolved.artifact_dir,
            workspace.path().join(DEFAULT_ARTIFACT_DIR)
        );
        assert!(!resolved.capture_values);
    }

    #[test]
    fn telemetry_trace_validation_rejects_invalid_sample_rate() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let section = TelemetrySection {
            sample_rate: Some(2.0),
            ..TelemetrySection::default()
        };

        let err = section
            .resolve_for_trace(workspace.path())
            .expect_err("invalid sample-rate must fail");

        assert_eq!(
            err,
            TelemetryValidationError::InvalidSampleRate { value: 2.0 }
        );
    }

    #[test]
    fn telemetry_trace_validation_rejects_unsupported_sampling_mode() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let section = TelemetrySection {
            sampling_mode: Some("always".to_string()),
            ..TelemetrySection::default()
        };

        let err = section
            .resolve_for_trace(workspace.path())
            .expect_err("unsupported sampling mode must fail");

        assert_eq!(
            err,
            TelemetryValidationError::UnsupportedSamplingMode {
                value: "always".to_string()
            }
        );
    }

    #[test]
    fn telemetry_trace_validation_rejects_parent_artifact_dir() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let section = TelemetrySection {
            artifact_dir: Some(PathBuf::from("../outside")),
            ..TelemetrySection::default()
        };

        let err = section
            .resolve_for_trace(workspace.path())
            .expect_err("parent traversal must fail");

        assert!(matches!(
            err,
            TelemetryValidationError::InvalidArtifactDir { .. }
        ));
    }

    #[test]
    fn telemetry_trace_validation_rejects_capture_values() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let section = TelemetrySection {
            capture_values: Some(true),
            ..TelemetrySection::default()
        };

        let err = section
            .resolve_for_trace(workspace.path())
            .expect_err("capture-values must fail");

        assert_eq!(err, TelemetryValidationError::CaptureValuesUnsupported);
    }

    fn test_graph() -> crate::graph::SemanticGraph {
        let ast = crate::parser::parse_jsonld(test_graph_source()).expect("fixture should parse");
        crate::graph::builder::build_graph(&ast).expect("fixture should build")
    }

    fn test_repair_context_and_source(dir: &TempDir) -> (RepairCrashContext, PathBuf) {
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        let graph_source = write_test_graph_source(dir, test_graph_source(), "graph.jsonld");
        write_test_crash(dir, &map, "candidate repair");
        let mut options = RepairContextOptions::new(dir.path());
        options.graph_sources.push(graph_source.clone());
        let context = repair_crash_context(&options).expect("repair context should be built");
        (context, graph_source)
    }

    fn valid_modify_patch() -> serde_json::Value {
        serde_json::json!({
            "ops": [{
                "kind": "modify_op",
                "node_id": "duumbi:t/main/entry/0",
                "field": "duumbi:value",
                "value": 1
            }]
        })
    }

    fn test_success_command() -> String {
        std::env::current_exe()
            .expect("test executable path should be available")
            .display()
            .to_string()
    }

    fn test_success_command_template() -> String {
        let command = test_success_command();
        let quoted = shlex::try_quote(&command).expect("test executable path should be quotable");
        format!("{quoted} {{candidate_graph}}")
    }

    struct TestRepairValidationRunner {
        rebuild_passes: bool,
        tests_pass: bool,
    }

    impl RepairValidationRunner for TestRepairValidationRunner {
        fn rebuild_candidate(&mut self, candidate_graph_json: &str) -> RepairValidationStepOutcome {
            assert!(candidate_graph_json.contains("duumbi:t/main/entry/0"));
            RepairValidationStepOutcome::with_command(
                self.rebuild_passes,
                if self.rebuild_passes {
                    "candidate rebuilt"
                } else {
                    "candidate rebuild failed"
                },
                "duumbi build {candidate_graph}",
                Some("rebuild output".to_string()),
            )
        }

        fn run_relevant_tests(
            &mut self,
            candidate_graph_json: &str,
            rebuild: &RepairValidationStepOutcome,
        ) -> RepairValidationStepOutcome {
            assert!(candidate_graph_json.contains("duumbi:t/main/entry/0"));
            assert!(rebuild.passed);
            RepairValidationStepOutcome::with_command(
                self.tests_pass,
                if self.tests_pass {
                    "candidate-aware tests passed"
                } else {
                    "candidate-aware tests failed"
                },
                "{candidate_binary}",
                Some("test output".to_string()),
            )
        }
    }

    fn test_graph_source() -> &'static str {
        r#"{
            "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
            "@type": "duumbi:Module",
            "@id": "duumbi:t",
            "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:t/main",
                "duumbi:name": "main",
                "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:t/main/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:t/main/entry/0",
                            "duumbi:value": 0,
                            "duumbi:resultType": "i64"
                        },
                        {
                            "@type": "duumbi:Return",
                            "@id": "duumbi:t/main/entry/1",
                            "duumbi:operand": {"@id": "duumbi:t/main/entry/0"}
                        }
                    ]
                }]
            }]
        }"#
    }

    fn write_test_graph_source(dir: &TempDir, source: &str, file_name: &str) -> PathBuf {
        let path = dir.path().join(file_name);
        std::fs::write(&path, source).expect("graph source should be written");
        path
    }

    fn write_test_crash(dir: &TempDir, map: &TraceMap, message: &str) -> (u64, u64) {
        write_test_crash_entries(dir, map, &[message])
    }

    fn write_test_crash_entries(dir: &TempDir, map: &TraceMap, messages: &[&str]) -> (u64, u64) {
        let (function_trace_id, block_trace_id) = test_trace_ids(map);
        let mut content = String::new();
        for (index, message) in messages.iter().enumerate() {
            if index > 0 {
                content.push('\n');
            }
            let crash = test_crash_json(message, function_trace_id, block_trace_id, true);
            content.push_str(&crash.to_string());
            content.push('\n');
        }
        std::fs::write(dir.path().join(CRASH_DUMP_FILE), content)
            .expect("crash artifact should be written");
        (function_trace_id, block_trace_id)
    }

    fn write_test_crash_with_ids(
        dir: &TempDir,
        message: &str,
        function_trace_id: u64,
        block_trace_id: u64,
        trace_active: bool,
    ) {
        let crash = test_crash_json(message, function_trace_id, block_trace_id, trace_active);
        std::fs::write(dir.path().join(CRASH_DUMP_FILE), format!("{crash}\n"))
            .expect("crash artifact should be written");
    }

    fn test_trace_ids(map: &TraceMap) -> (u64, u64) {
        let function = map
            .entries
            .iter()
            .find(|entry| entry.kind == TraceMapKind::Function)
            .expect("function entry should exist");
        let block = map
            .entries
            .iter()
            .find(|entry| entry.kind == TraceMapKind::Block)
            .expect("block entry should exist");
        (function.trace_id, block.trace_id)
    }

    fn test_crash_json(
        message: &str,
        function_trace_id: u64,
        block_trace_id: u64,
        trace_active: bool,
    ) -> serde_json::Value {
        serde_json::json!({
            "schema_version": CRASH_SCHEMA_VERSION,
            "event": "panic",
            "message": message,
            "function_id": function_trace_id,
            "block_id": block_trace_id,
            "trace_active": trace_active
        })
    }
}
