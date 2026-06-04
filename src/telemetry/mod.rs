//! Local telemetry configuration and artifact path helpers.
//!
//! Phase 13 telemetry is opt-in. The defaults here keep normal builds and runs
//! uninstrumented while giving traced runs a deterministic local artifact
//! location when later build and runtime cycles wire the feature through.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

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
    /// Patched graph builds and passes graph validation.
    GraphValidation,
    /// Native rebuild succeeds after the candidate patch.
    NativeRebuild,
    /// Relevant targeted and regression tests pass.
    RelevantTests,
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
    /// Optional local artifact path or command summary backing the gate.
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
    /// Validation gate results.
    pub gates: Vec<RepairValidationGateEvidence>,
    /// Whether all required local validation gates passed.
    pub local_validation_passed: bool,
    /// Repairs must remain human-reviewable and are not silently accepted.
    pub requires_human_review: bool,
    /// Repair acceptance state. This remains false in telemetry evidence.
    pub accepted_for_application: bool,
}

impl RepairValidationEvidence {
    /// Creates repair validation evidence without accepting the repair.
    #[must_use]
    pub fn new(
        crash_context: RepairCrashContext,
        proposed_patch: serde_json::Value,
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

        Self {
            schema_version: REPAIR_VALIDATION_SCHEMA_VERSION.to_string(),
            crash_context,
            proposed_patch,
            gates,
            local_validation_passed,
            requires_human_review: true,
            accepted_for_application: false,
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
        let mut content = String::new();
        for (index, message) in messages.iter().enumerate() {
            if index > 0 {
                content.push('\n');
            }
            let crash = serde_json::json!({
                "schema_version": CRASH_SCHEMA_VERSION,
                "event": "panic",
                "message": message,
                "function_id": function.trace_id,
                "block_id": block.trace_id,
                "trace_active": true
            });
            content.push_str(&crash.to_string());
            content.push('\n');
        }
        std::fs::write(dir.path().join(CRASH_DUMP_FILE), content)
            .expect("crash artifact should be written");
        (function.trace_id, block.trace_id)
    }
}
