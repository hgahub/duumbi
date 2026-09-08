//! Shared isolated-attempt executor for benchmark and determinism replay.
//!
//! Creates a `TempDir`, runs structured intent execute, copies bounded
//! allowlisted evidence out, then drops the temporary workspace.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agents::LlmProvider;
use crate::bench::report::ErrorCategory;
use crate::determinism::digest::{exact_graph_digest, safe_artifact_key, sha256_hex_bytes};
use crate::hash;
use crate::intent::execute::{
    ExecutionPhaseEvent, IntentExecutionOutcome, PhaseEventStatus, StructuredExecuteError,
    run_execute_structured,
};
use crate::intent::spec::{IntentSpec, IntentStatus};
use crate::intent::taxonomy::{
    ClassifyInput, RootCauseAttribution, RootCauseClass, classify, counts_as_graph_failure,
};
use crate::intent::{load_intent, save_intent};
use crate::knowledge::learning::redact_secret_text;

use super::execute::CapturedDiagnostic;

/// Per-run retained-evidence total (32 MiB).
pub const RUN_CAP_BYTES: u64 = 33_554_432;
/// Reserved stub/metadata pool.
pub const STUB_POOL_BYTES: u64 = 262_144;
/// Allowlisted content cap (`RUN_CAP_BYTES - STUB_POOL_BYTES`).
pub const CONTENT_CAP_BYTES: u64 = RUN_CAP_BYTES - STUB_POOL_BYTES;
/// Maximum bytes for one `truncation.json` stub.
pub const STUB_MAX_BYTES: u64 = 4_096;
/// Sanitized execute.log cap.
pub const EXECUTE_LOG_MAX_BYTES: usize = 256 * 1024;
/// Redacted model I/O file cap.
pub const MODEL_IO_MAX_BYTES: usize = 256 * 1024;
/// Default per-attempt file count.
pub const PER_ATTEMPT_FILES_DEFAULT: usize = 32;
/// Per-attempt file count with `--keep-workspaces`.
pub const PER_ATTEMPT_FILES_KEEP: usize = 64;
/// Default per-attempt content bytes (1 MiB).
pub const PER_ATTEMPT_CONTENT_DEFAULT: u64 = 1024 * 1024;
/// Per-attempt content bytes with `--keep-workspaces` (8 MiB).
pub const PER_ATTEMPT_CONTENT_KEEP: u64 = 8 * 1024 * 1024;

/// Default benchmark artifact root.
pub const DEFAULT_BENCHMARK_ARTIFACT_DIR: &str = ".duumbi/benchmark/attempts";

/// Persistence status independent of execution `root_cause`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePersistence {
    /// Allowlisted files were written.
    Complete,
    /// Some files were written (stub or truncated).
    Partial,
    /// Copy/retention I/O failed.
    Failed,
    /// Hashes/summaries live only in the parent JSON.
    JsonOnly,
}

/// Opt-in model I/O capture status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelIoStatus {
    /// Flag was off.
    NotRequested,
    /// Current-attempt payloads were captured and redacted.
    Captured,
    /// Some payloads were captured.
    Partial,
    /// Provider did not expose payloads.
    Unavailable,
}

/// Why attempt files were omitted from the run tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OmissionReason {
    /// Remaining content budget cannot hold the allowlist; stub written.
    ContentBudgetExhausted,
    /// Stub pool cannot hold another stub; no new run-tree files.
    StubPoolExhausted,
}

/// Hash evidence for one attempt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptHashes {
    /// Initial exact graph digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_graph_exact: Option<String>,
    /// Initial semantic graph hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_graph_semantic: Option<String>,
    /// Final exact graph digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_graph_exact: Option<String>,
    /// Final semantic graph hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_graph_semantic: Option<String>,
    /// Intent spec hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_spec: Option<String>,
    /// Sanitized transcript hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    /// Prompt hash honesty (`full`, `partial`, `unavailable`).
    pub prompt_hash_status: String,
}

/// Observed phase facts, including recovered events.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseEvidence {
    /// Ordered phase events.
    pub events: Vec<ExecutionPhaseEvent>,
}

/// Resolved model identity for an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ModelIdentityEvidence {
    /// Model identity was resolved.
    Available {
        /// Provider/model label.
        label: String,
    },
    /// Model identity was unavailable.
    Unavailable {
        /// Stable reason.
        reason: String,
    },
}

/// Request for one isolated attempt.
pub struct AttemptRequest<'a> {
    /// Unique run identifier (never reused across concurrent runs).
    pub run_id: &'a str,
    /// Task/showcase id.
    pub task_id: &'a str,
    /// Intent spec to execute.
    pub spec: &'a IntentSpec,
    /// Provider used for this attempt.
    pub provider: &'a dyn LlmProvider,
    /// Raw provider route used for path keys.
    pub provider_key: &'a str,
    /// One-based attempt number.
    pub attempt: u32,
    /// Artifact root (`--artifact-dir`).
    pub artifact_dir: &'a Path,
    /// Copy graph/intent snapshot.
    pub keep_workspace: bool,
    /// Capture current-attempt model I/O.
    pub capture_model_io: bool,
    /// Intent slug saved into the isolated workspace.
    pub slug: &'a str,
    /// When `false`, skip TempDir execute (JSON-only remaining rows).
    pub execute: bool,
    /// Process-evidence status for taxonomy (`unsupported`, `broader_evidence_required`).
    pub process_evidence_status: Option<&'a str>,
    /// Inject a copy failure after execute (tests).
    pub force_persist_failure: bool,
    /// Whether provider credentials were missing.
    pub credentials_missing: bool,
    /// Current-attempt captured model I/O payloads (already redacted).
    pub captured_model_io: Vec<CapturedModelIo>,
}

/// One captured, redacted model I/O file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedModelIo {
    /// File name under `model-io/`.
    pub name: String,
    /// Redacted body.
    pub body: String,
}

/// Evidence returned by [`run_isolated_attempt`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptEvidence {
    /// Structured execute outcome when execute ran.
    pub outcome: IntentExecutionOutcome,
    /// Resolved model identity.
    pub model_identity: ModelIdentityEvidence,
    /// Graph/intent/transcript hashes.
    pub hashes: AttemptHashes,
    /// Sanitized execution log lines.
    pub sanitized_log: Vec<String>,
    /// Relative artifact paths that remain after TempDir drop.
    pub artifact_paths: Vec<String>,
    /// Observed phase facts.
    pub phase_evidence: PhaseEvidence,
    /// Failure class; `None` on success.
    pub root_cause: Option<RootCauseClass>,
    /// Attribution; omitted on success.
    pub root_cause_attribution: Option<RootCauseAttribution>,
    /// Coarse category; JSON `null` on unknown failures.
    pub error_category: Option<ErrorCategory>,
    /// Whether the error_category key must be present (failed attempts).
    pub error_category_key_required: bool,
    /// Evidence persistence status.
    pub evidence_persistence: EvidencePersistence,
    /// Sanitized persistence error, when any.
    pub persistence_error: Option<String>,
    /// Model I/O capture status.
    pub model_io_status: ModelIoStatus,
    /// Omission reason when files were skipped.
    pub omission_reason: Option<OmissionReason>,
    /// Whether execute ran.
    pub executed: bool,
    /// Terminal intent status text.
    pub intent_status: String,
    /// Validator/verifier summary text.
    pub summaries_text: String,
    /// Wall-clock duration in seconds.
    pub duration_secs: f64,
    /// Whether this row counts as a graph failure.
    pub graph_failure: bool,
    /// Captured diagnostics copied from the outcome.
    pub diagnostics: Vec<CapturedDiagnostic>,
}

/// Per-run retention counters.
#[derive(Debug, Clone)]
pub struct RunRetentionState {
    /// Allowlisted content bytes written under the run tree.
    pub content_bytes: u64,
    /// Stub/metadata bytes written under the run tree.
    pub stub_bytes: u64,
    /// Stop creating artifact-producing attempts.
    pub stop_artifact_attempts: bool,
}

impl Default for RunRetentionState {
    fn default() -> Self {
        Self {
            content_bytes: 0,
            stub_bytes: 0,
            stop_artifact_attempts: false,
        }
    }
}

impl RunRetentionState {
    /// Remaining allowlisted content budget.
    #[must_use]
    pub fn remaining_content(&self) -> u64 {
        CONTENT_CAP_BYTES.saturating_sub(self.content_bytes)
    }

    /// Remaining stub pool.
    #[must_use]
    pub fn remaining_stub(&self) -> u64 {
        STUB_POOL_BYTES.saturating_sub(self.stub_bytes)
    }

    /// Measured invariant check against locked caps.
    #[must_use]
    pub fn within_caps(&self) -> bool {
        self.content_bytes + self.stub_bytes <= RUN_CAP_BYTES
            && self.content_bytes <= CONTENT_CAP_BYTES
            && self.stub_bytes <= STUB_POOL_BYTES
    }
}

/// Generates a unique run id for one invocation.
#[must_use]
pub fn generate_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("run-{nanos}-{}", std::process::id())
}

/// Runs one isolated attempt and copies bounded evidence out of the TempDir.
#[must_use = "attempt evidence should be recorded in the parent report"]
pub async fn run_isolated_attempt<F>(
    request: AttemptRequest<'_>,
    init_workspace: &F,
    retention: &Mutex<RunRetentionState>,
) -> AttemptEvidence
where
    F: Fn(&Path) -> Result<(), anyhow::Error>,
{
    let start = Instant::now();
    let model_identity = match request.provider.model_name() {
        Some(model) if !model.trim().is_empty() => ModelIdentityEvidence::Available {
            label: request.provider.model_label(),
        },
        _ => ModelIdentityEvidence::Unavailable {
            reason: "provider did not expose resolved model".to_string(),
        },
    };

    {
        let state = retention.lock().expect("invariant: retention mutex");
        if !request.execute || state.stop_artifact_attempts {
            return json_only_unexecuted(request, model_identity, start.elapsed().as_secs_f64());
        }
    }

    if request.process_evidence_status == Some("broader_evidence_required") {
        return process_evidence_row(request, model_identity, start.elapsed().as_secs_f64());
    }

    let tmp = match tempfile::TempDir::new() {
        Ok(tmp) => tmp,
        Err(error) => {
            return finalize_without_workspace(
                request,
                model_identity,
                format!("tempdir creation failed: {error}"),
                start.elapsed().as_secs_f64(),
            );
        }
    };
    let workspace = tmp.path();

    let mut log = Vec::new();
    let mut hashes = AttemptHashes {
        prompt_hash_status: "partial".to_string(),
        ..AttemptHashes::default()
    };
    let mut intent_status = IntentStatus::Pending.to_string();
    let persist_error: Option<String>;
    let outcome;

    let init_result = (|| {
        init_workspace(workspace).map_err(|error| format!("init failed: {error}"))?;
        let graph_dir = workspace.join(".duumbi/graph");
        hashes.initial_graph_exact = exact_graph_digest(&graph_dir).ok();
        hashes.initial_graph_semantic = hash::semantic_hash(&graph_dir).ok();
        let mut run_spec = request.spec.clone();
        run_spec.status = IntentStatus::Pending;
        save_intent(workspace, request.slug, &run_spec)
            .map_err(|error| format!("failed to save intent: {error}"))?;
        hashes.intent_spec = serde_yaml::to_string(&run_spec)
            .ok()
            .map(|yaml| sha256_hex_bytes(yaml.as_bytes()));
        Ok::<(), String>(())
    })();

    if let Err(error) = init_result {
        outcome = infra_outcome("init", &error);
        persist_error = Some(redact_secret_text(&error));
        let mut evidence = assemble_evidence(
            &request,
            outcome,
            model_identity,
            hashes,
            &log,
            intent_status,
            persist_error,
            start.elapsed().as_secs_f64(),
        );
        {
            let mut state = retention.lock().expect("invariant: retention mutex");
            persist_according_to_policy(
                &request,
                workspace,
                &mut state,
                &mut evidence,
                request.force_persist_failure,
            );
        }
        drop(tmp);
        return evidence;
    }

    match run_execute_structured(request.provider, workspace, request.slug, &mut log).await {
        Ok(executed) => {
            outcome = executed;
            persist_error = None;
        }
        Err(error) => {
            if let Some(structured) = error.downcast_ref::<StructuredExecuteError>() {
                outcome = structured.partial.clone();
                persist_error = Some(redact_secret_text(&structured.message));
            } else {
                let message = format!("{error:#}");
                outcome = infra_outcome("execute", &message);
                persist_error = Some(redact_secret_text(&message));
            }
        }
    }

    let graph_dir = workspace.join(".duumbi/graph");
    hashes.final_graph_exact = exact_graph_digest(&graph_dir).ok();
    hashes.final_graph_semantic = hash::semantic_hash(&graph_dir).ok();
    if let Ok(final_spec) = load_intent(workspace, request.slug)
        .or_else(|_| crate::bench::runner::load_archived_intent(workspace, request.slug))
    {
        intent_status = final_spec.status.to_string();
    }

    let sanitized_joined = redact_secret_text(&log.join("\n"));
    hashes.transcript = Some(sha256_hex_bytes(sanitized_joined.as_bytes()));

    let mut evidence = assemble_evidence(
        &request,
        outcome,
        model_identity,
        hashes,
        &log,
        intent_status,
        persist_error,
        start.elapsed().as_secs_f64(),
    );
    {
        let mut state = retention.lock().expect("invariant: retention mutex");
        persist_according_to_policy(
            &request,
            workspace,
            &mut state,
            &mut evidence,
            request.force_persist_failure,
        );
    }
    drop(tmp);
    evidence
}

fn json_only_unexecuted(
    request: AttemptRequest<'_>,
    model_identity: ModelIdentityEvidence,
    duration_secs: f64,
) -> AttemptEvidence {
    let mut outcome = blank_outcome("stub_pool_exhausted");
    outcome.terminal_status = "stub_pool_exhausted".to_string();
    AttemptEvidence {
        outcome,
        model_identity,
        hashes: AttemptHashes {
            prompt_hash_status: "unavailable".to_string(),
            ..AttemptHashes::default()
        },
        sanitized_log: Vec::new(),
        artifact_paths: Vec::new(),
        phase_evidence: PhaseEvidence::default(),
        root_cause: Some(RootCauseClass::Unknown),
        root_cause_attribution: Some(RootCauseAttribution::NoMatchingRule),
        error_category: None,
        error_category_key_required: true,
        evidence_persistence: EvidencePersistence::JsonOnly,
        persistence_error: None,
        model_io_status: if request.capture_model_io {
            ModelIoStatus::Unavailable
        } else {
            ModelIoStatus::NotRequested
        },
        omission_reason: Some(OmissionReason::StubPoolExhausted),
        executed: false,
        intent_status: "not_executed".to_string(),
        summaries_text: String::new(),
        duration_secs,
        graph_failure: false,
        diagnostics: Vec::new(),
    }
}

fn process_evidence_row(
    request: AttemptRequest<'_>,
    model_identity: ModelIdentityEvidence,
    duration_secs: f64,
) -> AttemptEvidence {
    let mut outcome = blank_outcome("process_evidence_required");
    outcome.success = false;
    outcome.tests_total = request.spec.test_cases.len();
    outcome.phase_events.push(ExecutionPhaseEvent {
        phase: "preflight".to_string(),
        status: PhaseEventStatus::Terminal,
        error_code: Some("broader_evidence_required".to_string()),
    });
    assemble_evidence(
        &request,
        outcome,
        model_identity,
        AttemptHashes {
            prompt_hash_status: "unavailable".to_string(),
            ..AttemptHashes::default()
        },
        &[],
        "pending".to_string(),
        None,
        duration_secs,
    )
}

fn finalize_without_workspace(
    request: AttemptRequest<'_>,
    model_identity: ModelIdentityEvidence,
    error: String,
    duration_secs: f64,
) -> AttemptEvidence {
    let outcome = infra_outcome("init", &error);
    assemble_evidence(
        &request,
        outcome,
        model_identity,
        AttemptHashes {
            prompt_hash_status: "unavailable".to_string(),
            ..AttemptHashes::default()
        },
        &[],
        "pending".to_string(),
        Some(redact_secret_text(&error)),
        duration_secs,
    )
}

fn blank_outcome(terminal_status: &str) -> IntentExecutionOutcome {
    IntentExecutionOutcome {
        success: false,
        first_pass_success: false,
        repair_attempted: false,
        repair_applied: false,
        repair_success: None,
        mutation_retry_count: None,
        repair_retry_count: None,
        retries_remaining: None,
        tests_passed: 0,
        tests_total: 0,
        terminal_status: terminal_status.to_string(),
        phase_events: Vec::new(),
        diagnostics: Vec::new(),
        dominant_error_code: None,
    }
}

fn infra_outcome(phase: &str, message: &str) -> IntentExecutionOutcome {
    let mut outcome = blank_outcome("infrastructure_error");
    outcome.phase_events.push(ExecutionPhaseEvent {
        phase: phase.to_string(),
        status: PhaseEventStatus::Terminal,
        error_code: Some("provider_or_infrastructure".to_string()),
    });
    outcome.diagnostics.push(CapturedDiagnostic {
        code: Some("provider_or_infrastructure".to_string()),
        message: redact_secret_text(message),
    });
    outcome.dominant_error_code = Some("provider_or_infrastructure".to_string());
    outcome
}

fn assemble_evidence(
    request: &AttemptRequest<'_>,
    outcome: IntentExecutionOutcome,
    model_identity: ModelIdentityEvidence,
    hashes: AttemptHashes,
    log: &[String],
    intent_status: String,
    persist_error: Option<String>,
    duration_secs: f64,
) -> AttemptEvidence {
    let sanitized_log: Vec<String> = redact_secret_text(&log.join("\n"))
        .lines()
        .map(str::to_string)
        .filter(|line| !line.is_empty())
        .collect();
    let graph_compiled =
        hashes.final_graph_exact.is_some() || hashes.final_graph_semantic.is_some();
    let classification = classify(&ClassifyInput {
        success: outcome.success,
        events: &outcome.phase_events,
        diagnostics: &outcome.diagnostics,
        tests_passed: outcome.tests_passed,
        tests_total: outcome.tests_total,
        terminal_status: &outcome.terminal_status,
        log_text: &sanitized_log.join("\n"),
        process_evidence_status: request.process_evidence_status,
        graph_compiled,
        credentials_missing: request.credentials_missing,
    });
    let summaries_text = format!(
        "intent_status={intent_status}\ntests={}/{}",
        outcome.tests_passed, outcome.tests_total
    );
    let model_io_status = if !request.capture_model_io {
        ModelIoStatus::NotRequested
    } else if request.captured_model_io.is_empty() {
        ModelIoStatus::Unavailable
    } else {
        ModelIoStatus::Captured
    };
    AttemptEvidence {
        phase_evidence: PhaseEvidence {
            events: outcome.phase_events.clone(),
        },
        diagnostics: outcome.diagnostics.clone(),
        graph_failure: counts_as_graph_failure(&classification, request.credentials_missing)
            && !outcome.success,
        error_category_key_required: !outcome.success,
        root_cause: classification.root_cause,
        root_cause_attribution: classification.attribution,
        error_category: classification.error_category,
        evidence_persistence: EvidencePersistence::JsonOnly,
        persistence_error: persist_error,
        model_io_status,
        omission_reason: None,
        executed: true,
        intent_status,
        summaries_text,
        duration_secs,
        outcome,
        model_identity,
        hashes,
        sanitized_log,
        artifact_paths: Vec::new(),
    }
}

fn persist_according_to_policy(
    request: &AttemptRequest<'_>,
    workspace: &Path,
    retention: &mut RunRetentionState,
    evidence: &mut AttemptEvidence,
    force_failure: bool,
) {
    if force_failure {
        if evidence.root_cause.is_none() && !evidence.outcome.success {
            // Keep any already established execution class.
        }
        evidence.evidence_persistence = EvidencePersistence::Failed;
        evidence.persistence_error = Some("forced persist failure".to_string());
        return;
    }

    if retention.stop_artifact_attempts {
        evidence.evidence_persistence = EvidencePersistence::JsonOnly;
        evidence.omission_reason = Some(OmissionReason::StubPoolExhausted);
        evidence.artifact_paths.clear();
        return;
    }

    let success = evidence.outcome.success;
    let write_failed_allowlist = !success;
    let write_snapshot = request.keep_workspace;
    let write_model_io = request.capture_model_io
        && !request.captured_model_io.is_empty()
        && evidence.model_io_status == ModelIoStatus::Captured;

    if success && !write_snapshot && !write_model_io {
        evidence.evidence_persistence = EvidencePersistence::JsonOnly;
        evidence.artifact_paths.clear();
        return;
    }

    let planned = planned_files(
        request,
        workspace,
        evidence,
        write_failed_allowlist,
        write_snapshot,
        write_model_io,
    );
    let content_len: u64 = planned.iter().map(|file| file.body.len() as u64).sum();
    let remaining_content = retention.remaining_content();
    let remaining_stub = retention.remaining_stub();

    if content_len > remaining_content {
        if remaining_stub == 0 {
            evidence.evidence_persistence = EvidencePersistence::JsonOnly;
            evidence.omission_reason = Some(OmissionReason::StubPoolExhausted);
            evidence.artifact_paths.clear();
            retention.stop_artifact_attempts = true;
            return;
        }
        match write_stub(request, retention, evidence, &planned) {
            Ok(path) => {
                evidence.evidence_persistence = EvidencePersistence::Partial;
                evidence.omission_reason = Some(OmissionReason::ContentBudgetExhausted);
                evidence.artifact_paths = vec![path];
            }
            Err(error) => {
                evidence.evidence_persistence = EvidencePersistence::Failed;
                evidence.persistence_error = Some(redact_secret_text(&error.to_string()));
            }
        }
        return;
    }

    match write_content_files(request, retention, &planned) {
        Ok(paths) => {
            evidence.artifact_paths = paths;
            evidence.evidence_persistence = if evidence.artifact_paths.is_empty() {
                EvidencePersistence::JsonOnly
            } else {
                EvidencePersistence::Complete
            };
        }
        Err(error) => {
            evidence.evidence_persistence = EvidencePersistence::Failed;
            evidence.persistence_error = Some(redact_secret_text(&error.to_string()));
        }
    }
}

struct PlannedFile {
    relative: String,
    body: Vec<u8>,
    is_stub: bool,
}

fn planned_files(
    request: &AttemptRequest<'_>,
    workspace: &Path,
    evidence: &AttemptEvidence,
    failed_allowlist: bool,
    snapshot: bool,
    model_io: bool,
) -> Vec<PlannedFile> {
    let mut files = Vec::new();
    if failed_allowlist {
        files.push(PlannedFile {
            relative: "phase_evidence.json".to_string(),
            body: serde_json::to_vec_pretty(&evidence.phase_evidence).unwrap_or_default(),
            is_stub: false,
        });
        files.push(PlannedFile {
            relative: "summaries.json".to_string(),
            body: serde_json::to_vec_pretty(&json!({
                "intent_status": evidence.intent_status,
                "tests_passed": evidence.outcome.tests_passed,
                "tests_total": evidence.outcome.tests_total,
                "validator_verifier": evidence.summaries_text,
            }))
            .unwrap_or_default(),
            is_stub: false,
        });
        files.push(PlannedFile {
            relative: "intent-status.txt".to_string(),
            body: format!("{}\n{}", evidence.intent_status, evidence.summaries_text).into_bytes(),
            is_stub: false,
        });
        files.push(PlannedFile {
            relative: "hashes.json".to_string(),
            body: serde_json::to_vec_pretty(&evidence.hashes).unwrap_or_default(),
            is_stub: false,
        });
        let mut log = evidence.sanitized_log.join("\n").into_bytes();
        if log.len() > EXECUTE_LOG_MAX_BYTES {
            log.truncate(EXECUTE_LOG_MAX_BYTES);
        }
        files.push(PlannedFile {
            relative: "execute.log".to_string(),
            body: log,
            is_stub: false,
        });
    }
    if snapshot {
        files.extend(snapshot_files(workspace));
    }
    if model_io {
        for payload in &request.captured_model_io {
            let mut body = redact_secret_text(&payload.body).into_bytes();
            if body.len() > MODEL_IO_MAX_BYTES {
                body.truncate(MODEL_IO_MAX_BYTES);
            }
            files.push(PlannedFile {
                relative: format!("model-io/{}", payload.name),
                body,
                is_stub: false,
            });
        }
    }
    apply_per_attempt_caps(files, request.keep_workspace)
}

fn apply_per_attempt_caps(mut files: Vec<PlannedFile>, keep_workspace: bool) -> Vec<PlannedFile> {
    let file_cap = if keep_workspace {
        PER_ATTEMPT_FILES_KEEP
    } else {
        PER_ATTEMPT_FILES_DEFAULT
    };
    let byte_cap = if keep_workspace {
        PER_ATTEMPT_CONTENT_KEEP
    } else {
        PER_ATTEMPT_CONTENT_DEFAULT
    };
    // Drop lowest priority first: model-io, snapshot, execute.log, then hashes/summaries.
    let priority = |name: &str| -> u8 {
        if name.starts_with("model-io/") {
            0
        } else if name.starts_with("workspace/") {
            1
        } else if name == "execute.log" {
            2
        } else {
            3
        }
    };
    files.sort_by(|a, b| priority(&b.relative).cmp(&priority(&a.relative)));
    let mut kept = Vec::new();
    let mut bytes = 0u64;
    for file in files {
        if kept.len() >= file_cap {
            break;
        }
        let next = bytes.saturating_add(file.body.len() as u64);
        if next > byte_cap {
            continue;
        }
        bytes = next;
        kept.push(file);
    }
    kept
}

fn snapshot_files(workspace: &Path) -> Vec<PlannedFile> {
    let mut files = Vec::new();
    let graph_dir = workspace.join(".duumbi/graph");
    let intents_dir = workspace.join(".duumbi/intents");
    collect_snapshot_dir(&graph_dir, workspace, &mut files);
    collect_snapshot_dir(&intents_dir, workspace, &mut files);
    files
}

fn collect_snapshot_dir(dir: &Path, workspace: &Path, files: &mut Vec<PlannedFile>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_symlink() {
            match skip_escaping_symlink(&path, workspace) {
                Ok(true) => continue,
                Ok(false) => {}
                Err(_) => continue,
            }
        }
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(
                name,
                "model-io" | "prompts" | "responses" | "target" | "cache"
            ) {
                continue;
            }
            collect_snapshot_dir(&path, workspace, files);
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".o") || name == "duumbi_runtime.o" {
            continue;
        }
        let Ok(relative) = path.strip_prefix(workspace) else {
            continue;
        };
        let Ok(body) = fs::read(&path) else {
            continue;
        };
        let sanitized = redact_secret_text(&String::from_utf8_lossy(&body));
        files.push(PlannedFile {
            relative: format!("workspace/{}", relative.display()),
            body: sanitized.into_bytes(),
            is_stub: false,
        });
    }
}

fn skip_escaping_symlink(path: &Path, workspace: &Path) -> io::Result<bool> {
    let target = fs::read_link(path)?;
    let resolved = if target.is_absolute() {
        target
    } else {
        path.parent().unwrap_or(path).join(target)
    };
    let canonical_workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let canonical_target = resolved.canonicalize().unwrap_or(resolved);
    Ok(!canonical_target.starts_with(&canonical_workspace))
}

fn attempt_dir(request: &AttemptRequest<'_>, unique: u32) -> PathBuf {
    let task_key = safe_artifact_key(request.task_id, "task");
    let provider_key = safe_artifact_key(request.provider_key, "provider");
    let attempt = if unique == 0 {
        request.attempt.to_string()
    } else {
        format!("{}-{unique}", request.attempt)
    };
    request
        .artifact_dir
        .join(request.run_id)
        .join(task_key)
        .join(provider_key)
        .join(attempt)
}

fn allocate_attempt_dir(request: &AttemptRequest<'_>) -> io::Result<PathBuf> {
    let mut unique = 0u32;
    loop {
        let dest = attempt_dir(request, unique);
        if dest.exists() {
            unique += 1;
            if unique > 32 {
                return Err(io::Error::other(
                    "attempt directory collision could not be resolved",
                ));
            }
            continue;
        }
        fs::create_dir_all(&dest)?;
        return Ok(dest);
    }
}

fn write_content_files(
    request: &AttemptRequest<'_>,
    retention: &mut RunRetentionState,
    planned: &[PlannedFile],
) -> io::Result<Vec<String>> {
    let dest = allocate_attempt_dir(request)?;
    let mut paths = Vec::new();
    for file in planned {
        let path = dest.join(&file.relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &file.body)?;
        retention.content_bytes = retention
            .content_bytes
            .saturating_add(file.body.len() as u64);
        if let Ok(relative) = path.strip_prefix(request.artifact_dir) {
            paths.push(relative.display().to_string());
        } else {
            paths.push(path.display().to_string());
        }
    }
    Ok(paths)
}

fn write_stub(
    request: &AttemptRequest<'_>,
    retention: &mut RunRetentionState,
    evidence: &AttemptEvidence,
    planned: &[PlannedFile],
) -> io::Result<String> {
    let omitted: Vec<String> = planned.iter().map(|file| file.relative.clone()).collect();
    let stub = json!({
        "omission_reason": "content_budget_exhausted",
        "omitted_files": omitted,
        "content_bytes": retention.content_bytes,
        "stub_bytes": retention.stub_bytes,
        "cap_bytes": RUN_CAP_BYTES,
        "stub_pool_bytes": STUB_POOL_BYTES,
        "intent_status": evidence.intent_status,
        "truncated": true,
        "original_bytes": planned.iter().map(|file| file.body.len() as u64).sum::<u64>(),
        "retained_bytes": 0u64,
    });
    let mut body = serde_json::to_vec_pretty(&stub).unwrap_or_default();
    if body.len() as u64 > STUB_MAX_BYTES {
        body.truncate(STUB_MAX_BYTES as usize);
    }
    if body.len() as u64 > retention.remaining_stub() {
        return Err(io::Error::other("stub does not fit in remaining stub pool"));
    }
    let dest = allocate_attempt_dir(request)?;
    let path = dest.join("truncation.json");
    let mut file = fs::File::create(&path)?;
    file.write_all(&body)?;
    retention.stub_bytes = retention.stub_bytes.saturating_add(body.len() as u64);
    Ok(path
        .strip_prefix(request.artifact_dir)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string()))
}

/// Walks a run artifact tree and returns `(content_bytes, stub_bytes)`.
#[must_use = "measured byte sums should be asserted"]
pub fn measure_run_tree(run_dir: &Path) -> io::Result<(u64, u64)> {
    let mut content = 0u64;
    let mut stub = 0u64;
    measure_tree(run_dir, &mut content, &mut stub)?;
    Ok((content, stub))
}

fn measure_tree(dir: &Path, content: &mut u64, stub: &mut u64) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            measure_tree(&path, content, stub)?;
            continue;
        }
        let len = entry.metadata()?.len();
        if path.file_name().and_then(|name| name.to_str()) == Some("truncation.json") {
            *stub = stub.saturating_add(len);
        } else {
            *content = content.saturating_add(len);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{AgentError, LlmProvider};
    use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus, TestCase};
    use crate::patch::PatchOp;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    struct CountingProvider {
        calls: AtomicUsize,
    }

    impl Default for CountingProvider {
        fn default() -> Self {
            Self {
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl LlmProvider for CountingProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn call_with_tools<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(AgentError::NoToolCalls) })
        }

        fn call_with_tools_streaming<'a>(
            &'a self,
            system_prompt: &'a str,
            user_message: &'a str,
            _on_text: &'a (dyn Fn(&str) + Send + Sync),
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
            self.call_with_tools(system_prompt, user_message)
        }
    }

    fn sample_spec() -> IntentSpec {
        IntentSpec {
            intent: "Build calculator".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["add(a, b) returns a + b".to_string()],
            modules: IntentModules {
                create: vec!["calculator/ops".to_string()],
                modify: vec!["app/main".to_string()],
            },
            test_cases: vec![TestCase {
                name: "addition".to_string(),
                function: "add".to_string(),
                args: vec![3, 5],
                expected_return: 8,
            }],
            dependencies: Vec::new(),
            bdd: Default::default(),
            context: None,
            created_at: None,
            execution: None,
        }
    }

    fn init_workspace(path: &Path) -> Result<(), anyhow::Error> {
        fs::create_dir_all(path.join(".duumbi/graph"))?;
        fs::write(
            path.join(".duumbi/graph/main.jsonld"),
            r#"{"@context":{"duumbi":"https://duumbi.dev/ns/core#"},"@type":"duumbi:Module","@id":"duumbi:main","duumbi:name":"main","duumbi:functions":[]}"#,
        )?;
        Ok(())
    }

    fn request<'a>(
        run_id: &'a str,
        spec: &'a IntentSpec,
        provider: &'a CountingProvider,
        artifact_dir: &'a Path,
    ) -> AttemptRequest<'a> {
        AttemptRequest {
            run_id,
            task_id: "scaled_math_pipeline",
            spec,
            provider,
            provider_key: "mock",
            attempt: 1,
            artifact_dir,
            keep_workspace: false,
            capture_model_io: false,
            slug: "benchmark-showcase",
            execute: true,
            process_evidence_status: None,
            force_persist_failure: false,
            credentials_missing: false,
            captured_model_io: Vec::new(),
        }
    }

    #[tokio::test]
    async fn failed_attempt_retains_allowlist_after_tempdir_drop() {
        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = sample_spec();
        let provider = CountingProvider::default();
        let retention = Mutex::new(RunRetentionState::default());
        let evidence = run_isolated_attempt(
            request("run-test", &spec, &provider, artifacts.path()),
            &init_workspace,
            &retention,
        )
        .await;

        assert!(!evidence.outcome.success);
        assert!(!evidence.outcome.repair_attempted);
        assert_eq!(evidence.evidence_persistence, EvidencePersistence::Complete);
        assert!(!evidence.artifact_paths.is_empty());
        for relative in &evidence.artifact_paths {
            let path = artifacts.path().join(relative);
            assert!(path.exists(), "missing retained file {}", path.display());
        }
        let joined = evidence.artifact_paths.join(" ");
        assert!(joined.contains("execute.log"));
        assert!(joined.contains("intent-status.txt"));
        assert!(joined.contains("summaries.json"));
        assert!(joined.contains("phase_evidence.json"));
        assert!(!joined.contains("workspace/"));
        assert!(!joined.contains("model-io"));
        assert!(retention.lock().expect("mutex").within_caps());
    }

    #[tokio::test]
    async fn persist_failure_keeps_execution_class() {
        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = sample_spec();
        let provider = CountingProvider::default();
        let retention = Mutex::new(RunRetentionState::default());
        let mut req = request("run-fail", &spec, &provider, artifacts.path());
        req.force_persist_failure = true;
        let evidence = run_isolated_attempt(req, &init_workspace, &retention).await;

        assert!(!evidence.outcome.success);
        assert_eq!(evidence.evidence_persistence, EvidencePersistence::Failed);
        assert!(evidence.root_cause.is_some());
        assert_eq!(
            evidence.root_cause,
            Some(RootCauseClass::TaskDecompositionOrMissingFunction)
        );
    }

    #[test]
    fn content_budget_exhaustion_writes_stub_only() {
        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = sample_spec();
        let provider = CountingProvider::default();
        let mut retention = RunRetentionState {
            content_bytes: CONTENT_CAP_BYTES,
            stub_bytes: 0,
            stop_artifact_attempts: false,
        };
        let mut evidence = AttemptEvidence {
            outcome: blank_outcome("verifier_failed"),
            model_identity: ModelIdentityEvidence::Unavailable {
                reason: "test".to_string(),
            },
            hashes: AttemptHashes {
                prompt_hash_status: "partial".to_string(),
                ..AttemptHashes::default()
            },
            sanitized_log: vec!["log".to_string()],
            artifact_paths: Vec::new(),
            phase_evidence: PhaseEvidence::default(),
            root_cause: Some(RootCauseClass::SchemaGraphValidation),
            root_cause_attribution: Some(RootCauseAttribution::MatchedRule),
            error_category: Some(ErrorCategory::SchemaError),
            error_category_key_required: true,
            evidence_persistence: EvidencePersistence::JsonOnly,
            persistence_error: None,
            model_io_status: ModelIoStatus::NotRequested,
            omission_reason: None,
            executed: true,
            intent_status: "failed".to_string(),
            summaries_text: "summary".to_string(),
            duration_secs: 0.1,
            graph_failure: true,
            diagnostics: Vec::new(),
        };
        persist_according_to_policy(
            &request("run-budget", &spec, &provider, artifacts.path()),
            artifacts.path(),
            &mut retention,
            &mut evidence,
            false,
        );
        assert_eq!(
            evidence.omission_reason,
            Some(OmissionReason::ContentBudgetExhausted)
        );
        assert_eq!(evidence.evidence_persistence, EvidencePersistence::Partial);
        assert_eq!(evidence.artifact_paths.len(), 1);
        assert!(
            evidence.artifact_paths[0].ends_with("truncation.json"),
            "{}",
            evidence.artifact_paths[0]
        );
        let run_dir = artifacts.path().join("run-budget");
        let (content, stub) = measure_run_tree(&run_dir).expect("measure");
        assert!(content + stub <= RUN_CAP_BYTES);
        assert!(content <= CONTENT_CAP_BYTES);
        assert!(stub <= STUB_POOL_BYTES);
        assert_eq!(content, 0);
        assert!(stub > 0);
        assert_eq!(
            evidence.root_cause,
            Some(RootCauseClass::SchemaGraphValidation)
        );
    }

    #[test]
    fn stub_pool_exhaustion_writes_no_new_files_and_stops() {
        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = sample_spec();
        let provider = CountingProvider::default();
        let mut retention = RunRetentionState {
            content_bytes: CONTENT_CAP_BYTES,
            stub_bytes: STUB_POOL_BYTES,
            stop_artifact_attempts: false,
        };
        let mut evidence = AttemptEvidence {
            outcome: blank_outcome("verifier_failed"),
            model_identity: ModelIdentityEvidence::Unavailable {
                reason: "test".to_string(),
            },
            hashes: AttemptHashes {
                prompt_hash_status: "partial".to_string(),
                ..AttemptHashes::default()
            },
            sanitized_log: vec!["log".to_string()],
            artifact_paths: Vec::new(),
            phase_evidence: PhaseEvidence::default(),
            root_cause: Some(RootCauseClass::Unknown),
            root_cause_attribution: Some(RootCauseAttribution::NoMatchingRule),
            error_category: None,
            error_category_key_required: true,
            evidence_persistence: EvidencePersistence::JsonOnly,
            persistence_error: None,
            model_io_status: ModelIoStatus::NotRequested,
            omission_reason: None,
            executed: true,
            intent_status: "failed".to_string(),
            summaries_text: "summary".to_string(),
            duration_secs: 0.1,
            graph_failure: true,
            diagnostics: Vec::new(),
        };
        persist_according_to_policy(
            &request("run-stub", &spec, &provider, artifacts.path()),
            artifacts.path(),
            &mut retention,
            &mut evidence,
            false,
        );
        assert_eq!(
            evidence.omission_reason,
            Some(OmissionReason::StubPoolExhausted)
        );
        assert_eq!(evidence.evidence_persistence, EvidencePersistence::JsonOnly);
        assert!(evidence.artifact_paths.is_empty());
        assert!(retention.stop_artifact_attempts);
        let run_dir = artifacts.path().join("run-stub");
        assert!(!run_dir.exists() || fs::read_dir(&run_dir).map(|d| d.count()).unwrap_or(0) == 0);
        let (content, stub) = measure_run_tree(&run_dir).expect("measure");
        assert_eq!(content, 0);
        assert_eq!(stub, 0);
        assert!(content + stub <= RUN_CAP_BYTES);
        assert!(stub <= STUB_POOL_BYTES);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_omitted_from_snapshot() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let outside = tmp.path().join("outside.txt");
        fs::write(&outside, "secret-outside").expect("outside");
        let workspace = tmp.path().join("ws");
        fs::create_dir_all(workspace.join(".duumbi/graph")).expect("graph");
        fs::write(
            workspace.join(".duumbi/graph/main.jsonld"),
            r#"{"@type":"duumbi:Module"}"#,
        )
        .expect("graph write");
        symlink(&outside, workspace.join(".duumbi/graph/escape.jsonld")).expect("symlink");
        let files = snapshot_files(&workspace);
        let joined: String = files
            .iter()
            .map(|file| String::from_utf8_lossy(&file.body).into_owned())
            .collect();
        assert!(!joined.contains("secret-outside"));
        assert!(outside.exists());
        assert_eq!(
            fs::read_to_string(&outside).expect("read"),
            "secret-outside"
        );
    }

    #[test]
    fn safe_artifact_key_rejects_traversal() {
        let key = safe_artifact_key("../etc/passwd", "task");
        assert!(!key.contains(".."));
        assert!(!key.contains('/'));
    }

    fn repair_request<'a>(
        run_id: &'a str,
        spec: &'a IntentSpec,
        provider: &'a crate::intent::test_support::ScriptedRepairProvider,
        artifact_dir: &'a Path,
    ) -> AttemptRequest<'a> {
        AttemptRequest {
            run_id,
            task_id: "repair_fixture",
            spec,
            provider,
            provider_key: "mock",
            attempt: 1,
            artifact_dir,
            keep_workspace: false,
            capture_model_io: false,
            slug: "benchmark-showcase",
            execute: true,
            process_evidence_status: None,
            force_persist_failure: false,
            credentials_missing: false,
            captured_model_io: Vec::new(),
        }
    }

    #[tokio::test]
    async fn execute_through_repair_no_patch_records_repair_attempted() {
        use crate::intent::test_support::{
            RepairScript, ScriptedRepairProvider, init_skeleton_workspace, repair_fixture_spec,
        };

        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = repair_fixture_spec();
        let provider = ScriptedRepairProvider::new(RepairScript::NoPatch);
        let retention = Mutex::new(RunRetentionState::default());
        let evidence = run_isolated_attempt(
            repair_request("run-nopatch", &spec, &provider, artifacts.path()),
            &init_skeleton_workspace,
            &retention,
        )
        .await;

        assert!(!evidence.outcome.success);
        assert!(evidence.outcome.repair_attempted);
        assert!(!evidence.outcome.repair_applied);
        assert_eq!(evidence.outcome.repair_success, Some(false));
        assert!(!evidence.outcome.first_pass_success);
        assert!(evidence.executed);
        assert!(!evidence.artifact_paths.is_empty());
    }

    #[tokio::test]
    async fn execute_through_repair_patched_fail_keeps_failure() {
        use crate::intent::test_support::{
            RepairScript, ScriptedRepairProvider, init_skeleton_workspace, repair_fixture_spec,
        };

        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = repair_fixture_spec();
        let provider = ScriptedRepairProvider::new(RepairScript::PatchedFail);
        let retention = Mutex::new(RunRetentionState::default());
        let evidence = run_isolated_attempt(
            repair_request("run-patched-fail", &spec, &provider, artifacts.path()),
            &init_skeleton_workspace,
            &retention,
        )
        .await;

        assert!(!evidence.outcome.success);
        assert!(evidence.outcome.repair_attempted);
        assert!(evidence.outcome.repair_applied);
        assert_eq!(evidence.outcome.repair_success, Some(false));
    }

    #[tokio::test]
    async fn execute_through_repair_success_passes_verifier() {
        use crate::intent::test_support::{
            RepairScript, ScriptedRepairProvider, init_skeleton_workspace, repair_fixture_spec,
        };

        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = repair_fixture_spec();
        let provider = ScriptedRepairProvider::new(RepairScript::RepairedSuccess);
        let retention = Mutex::new(RunRetentionState::default());
        let evidence = run_isolated_attempt(
            repair_request("run-repaired", &spec, &provider, artifacts.path()),
            &init_skeleton_workspace,
            &retention,
        )
        .await;

        assert!(
            evidence.outcome.success,
            "terminal={} tests={}/{} log={:?}",
            evidence.outcome.terminal_status,
            evidence.outcome.tests_passed,
            evidence.outcome.tests_total,
            evidence.sanitized_log
        );
        assert!(evidence.outcome.repair_attempted);
        assert!(evidence.outcome.repair_applied);
        assert_eq!(evidence.outcome.repair_success, Some(true));
        assert!(!evidence.outcome.first_pass_success);
    }

    #[tokio::test]
    async fn infra_err_after_repair_keeps_repair_attempted() {
        use crate::intent::execute::FAIL_IO_AFTER_REPAIR;
        use crate::intent::test_support::{
            RepairScript, ScriptedRepairProvider, init_skeleton_workspace, repair_fixture_spec,
        };
        use std::sync::atomic::Ordering;

        FAIL_IO_AFTER_REPAIR.store(true, Ordering::SeqCst);
        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = repair_fixture_spec();
        let provider = ScriptedRepairProvider::new(RepairScript::NoPatch);
        let retention = Mutex::new(RunRetentionState::default());
        let evidence = run_isolated_attempt(
            repair_request("run-infra-repair", &spec, &provider, artifacts.path()),
            &init_skeleton_workspace,
            &retention,
        )
        .await;
        FAIL_IO_AFTER_REPAIR.store(false, Ordering::SeqCst);

        assert!(!evidence.outcome.success);
        assert!(evidence.outcome.repair_attempted);
        assert!(!evidence.outcome.repair_applied);
        assert!(
            evidence
                .persistence_error
                .as_deref()
                .is_some_and(|msg| msg.contains("injected infrastructure error"))
                || evidence
                    .sanitized_log
                    .iter()
                    .any(|line| line.contains("Repair"))
        );
    }
}
