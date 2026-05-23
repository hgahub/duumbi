//! Local telemetry configuration and artifact path helpers.
//!
//! Phase 13 telemetry is opt-in. The defaults here keep normal builds and runs
//! uninstrumented while giving traced runs a deterministic local artifact
//! location when later build and runtime cycles wire the feature through.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Agent-facing crash context for proposing a graph repair.
///
/// This context is derived from mapped telemetry artifacts only. It does not
/// include provider output, patch application results, or repair acceptance.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RepairCrashContext {
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
    /// Validation checks expected before any proposed repair can be reviewed.
    pub validation_expectations: Vec<String>,
    /// Test checks expected before any proposed repair can be reviewed.
    pub test_expectations: Vec<String>,
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
    /// Returns [`TelemetryError::TraceIdCollision`] if generated IDs collide
    /// inside this graph.
    pub fn from_graph(graph: &SemanticGraph) -> Result<Self, TelemetryError> {
        Self::new(entries_for_graph(graph))
    }

    /// Creates a combined trace map for all modules in a program.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError::TraceIdCollision`] if generated IDs collide
    /// inside the compiled program.
    pub fn from_program(program: &Program) -> Result<Self, TelemetryError> {
        let entries = program
            .modules
            .values()
            .flat_map(entries_for_graph)
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
    let mapped = mapped_crash_evidence(telemetry_dir, crash_path, map_path)?;

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
/// Returns [`TelemetryError`] when crash/map evidence is missing, malformed, or
/// cannot map the crash trace IDs to graph entries.
pub fn repair_crash_context_from_artifacts(
    telemetry_dir: &Path,
    crash_path: Option<&Path>,
    map_path: Option<&Path>,
) -> Result<RepairCrashContext, TelemetryError> {
    let mapped = mapped_crash_evidence(telemetry_dir, crash_path, map_path)?;

    Ok(RepairCrashContext {
        crash_message: mapped.crash.message,
        function_id: mapped.function.graph_id.clone(),
        block_id: mapped.block.graph_id.clone(),
        exact_node_id: None,
        trace_ids: TraceCorrelation {
            function_trace_id: mapped.crash.function_id,
            block_trace_id: mapped.crash.block_id,
        },
        graph_context: repair_graph_context(&mapped.function, &mapped.block),
        validation_expectations: default_validation_expectations(),
        test_expectations: default_test_expectations(),
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
    function: TraceMapEntry,
    block: TraceMapEntry,
}

fn mapped_crash_evidence(
    telemetry_dir: &Path,
    crash_path: Option<&Path>,
    map_path: Option<&Path>,
) -> Result<MappedCrashEvidence, TelemetryError> {
    let crash_path = crash_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| telemetry_dir.join(CRASH_DUMP_FILE));
    let map_path = map_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| telemetry_dir.join(TRACE_MAP_FILE));
    let crash = read_latest_crash(&crash_path)?;
    let trace_map = read_trace_map(&map_path)?;

    if !crash.trace_active {
        return Err(TelemetryError::MissingEvidence(
            "crash artifact was not written from an active traced run".to_string(),
        ));
    }

    let function = trace_map
        .entries
        .iter()
        .find(|entry| entry.kind == TraceMapKind::Function && entry.trace_id == crash.function_id)
        .ok_or_else(|| {
            TelemetryError::Unmapped(format!(
                "function trace ID {} was not found in '{}'",
                crash.function_id,
                map_path.display()
            ))
        })?;
    let block = trace_map
        .entries
        .iter()
        .find(|entry| entry.kind == TraceMapKind::Block && entry.trace_id == crash.block_id)
        .ok_or_else(|| {
            TelemetryError::Unmapped(format!(
                "block trace ID {} was not found in '{}'",
                crash.block_id,
                map_path.display()
            ))
        })?;

    Ok(MappedCrashEvidence {
        crash,
        function: function.clone(),
        block: block.clone(),
    })
}

fn repair_graph_context(function: &TraceMapEntry, block: &TraceMapEntry) -> serde_json::Value {
    serde_json::json!({
        "source": "mapped_trace_artifacts",
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
        "exact_node_evidence": null,
        "context_limit": "function_and_block_identity_only"
    })
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

fn read_latest_crash(path: &Path) -> Result<CrashArtifact, TelemetryError> {
    let content = fs::read_to_string(path).map_err(|source| TelemetryError::Io {
        context: format!("Failed to read crash artifact '{}'", path.display()),
        source,
    })?;
    let latest = content
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| {
            TelemetryError::MissingEvidence(format!(
                "crash artifact '{}' did not contain any JSONL entries",
                path.display()
            ))
        })?;

    serde_json::from_str(latest).map_err(|source| TelemetryError::Parse {
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct TelemetrySection {
    /// Whether telemetry is enabled by config.
    ///
    /// Build and run surfaces still require an explicit traced mode before
    /// emitting telemetry. This field only records the user's local preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Local directory for telemetry artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_dir: Option<PathBuf>,

    /// Whether traced runs may capture argument or value snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_values: Option<bool>,
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
        let path = std::env::var_os(TELEMETRY_DIR_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.configured_artifact_dir().to_path_buf());

        if path.is_absolute() {
            path
        } else {
            workspace_root.join(path)
        }
    }
}

fn entries_for_graph(graph: &SemanticGraph) -> Vec<TraceMapEntry> {
    let mut entries = Vec::new();

    for function in &graph.functions {
        let function_graph_id = function_trace_graph_id(graph, function);
        entries.push(TraceMapEntry {
            trace_id: trace_id(TraceMapKind::Function, &function_graph_id),
            kind: TraceMapKind::Function,
            graph_id: function_graph_id,
            module: graph.module_name.0.clone(),
            function: function.name.0.clone(),
            block: None,
        });

        for block in &function.blocks {
            let block_graph_id = block_trace_graph_id(graph, function, block);
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

    entries
}

/// Returns the graph identity used for a function trace entry.
#[must_use]
pub fn function_trace_graph_id(graph: &SemanticGraph, function: &FunctionInfo) -> String {
    function
        .blocks
        .iter()
        .find_map(|block| graph_id_from_first_node(graph, block, 2))
        .unwrap_or_else(|| format!("duumbi:{}/{}", graph.module_name.0, function.name.0))
}

/// Returns the graph identity used for a block trace entry.
#[must_use]
pub fn block_trace_graph_id(
    graph: &SemanticGraph,
    function: &FunctionInfo,
    block: &BlockInfo,
) -> String {
    graph_id_from_first_node(graph, block, 1).unwrap_or_else(|| {
        format!(
            "duumbi:{}/{}/{}",
            graph.module_name.0, function.name.0, block.label.0
        )
    })
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

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn telemetry_defaults_are_local_and_off() {
        let section = TelemetrySection::default();

        assert!(!section.effective_enabled());
        assert!(!section.effective_capture_values());
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
        let (function_trace_id, block_trace_id) =
            write_test_crash(&dir, &map, "called Option::unwrap() on a None value");

        let context = repair_crash_context_from_artifacts(dir.path(), None, None)
            .expect("repair context should be built from mapped evidence");

        assert_eq!(
            context.crash_message,
            "called Option::unwrap() on a None value"
        );
        assert_eq!(context.function_id, "duumbi:t/main");
        assert_eq!(context.block_id, "duumbi:t/main/entry");
        assert_eq!(context.exact_node_id, None);
        assert_eq!(context.trace_ids.function_trace_id, function_trace_id);
        assert_eq!(context.trace_ids.block_trace_id, block_trace_id);
        assert_eq!(
            context.graph_context["source"],
            serde_json::json!("mapped_trace_artifacts")
        );
        assert_eq!(
            context.graph_context["block"]["graph_id"],
            serde_json::json!("duumbi:t/main/entry")
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
        let roundtrip: RepairCrashContext =
            serde_json::from_str(&serialized).expect("repair context should deserialize");
        assert_eq!(roundtrip, context);
    }

    #[test]
    fn repair_validation_evidence_keeps_human_review_required() {
        let dir = TempDir::new().expect("invariant: temp dir creation must succeed");
        let map = TraceMap::from_graph(&test_graph()).expect("trace map should be generated");
        write_trace_map(&map, dir.path()).expect("trace map should be written");
        write_test_crash(&dir, &map, "candidate repair");
        let context = repair_crash_context_from_artifacts(dir.path(), None, None)
            .expect("repair context should be built");
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
        write_test_crash(&dir, &map, "candidate repair");
        let context = repair_crash_context_from_artifacts(dir.path(), None, None)
            .expect("repair context should be built");
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
        let _guard = ENV_LOCK
            .lock()
            .expect("invariant: test env lock must not be poisoned");
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let override_dir = workspace.path().join("env-telemetry");
        // SAFETY: this test serializes environment mutation with ENV_LOCK and
        // removes the variable before releasing the lock.
        unsafe {
            std::env::set_var(TELEMETRY_DIR_ENV, &override_dir);
        }

        let section = TelemetrySection {
            artifact_dir: Some(PathBuf::from("config-telemetry")),
            ..TelemetrySection::default()
        };

        assert_eq!(
            section.effective_artifact_dir(workspace.path()),
            override_dir
        );
        // SAFETY: see the set_var safety note above; the same lock is held.
        unsafe {
            std::env::remove_var(TELEMETRY_DIR_ENV);
        }
    }

    fn test_graph() -> crate::graph::SemanticGraph {
        let source = r#"{
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
        }"#;
        let ast = crate::parser::parse_jsonld(source).expect("fixture should parse");
        crate::graph::builder::build_graph(&ast).expect("fixture should build")
    }

    fn write_test_crash(dir: &TempDir, map: &TraceMap, message: &str) -> (u64, u64) {
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
        let crash = serde_json::json!({
            "schema_version": CRASH_SCHEMA_VERSION,
            "event": "panic",
            "message": message,
            "function_id": function.trace_id,
            "block_id": block.trace_id,
            "trace_active": true
        });
        std::fs::write(dir.path().join(CRASH_DUMP_FILE), format!("{crash}\n"))
            .expect("crash artifact should be written");
        (function.trace_id, block.trace_id)
    }
}
