//! Native DUUMBI Loop provider-core vocabulary and provider-duumbi MVP.
//!
//! This module implements the first local provider-duumbi slice for DUUMBI-738.
//! It maps existing DUUMBI intent, graph, BDD, session, knowledge, and registry
//! workspace state into provider-neutral Loop objects, then writes deterministic
//! native intake/spec artifacts without requiring GitHub, GitLab, or an LLM
//! provider call.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::intent;
use crate::intent::bdd::{BddReadiness, BddReadinessReport, load_bdd_report};
use crate::intent::spec::IntentSpec;
use crate::knowledge::store::KnowledgeStore;
use crate::knowledge::types::KnowledgeNode;
use crate::patch::{GraphPatch, PatchOp};

/// Schema version for provider-neutral Loop artifacts.
pub const LOOP_ARTIFACT_SCHEMA_VERSION: &str = "duumbi.loop.artifact.v1";

/// Primary DUUMBI-owned model label for native Loop spec work.
pub const DEFAULT_SPEC_MODEL_LABEL: &str = "balanced";

/// Primary DUUMBI-owned model label for native Loop review work.
pub const DEFAULT_REVIEW_MODEL_LABEL: &str = "strict_review";

const MAX_GRAPH_SOURCE_DEPTH: usize = 16;

/// Provider kind for a Loop provider implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopProviderKind {
    /// Native DUUMBI provider backed by intents, session ledger, graph, and
    /// artifact files.
    Duumbi,
    /// GitHub adapter provider.
    Github,
    /// GitLab adapter provider.
    Gitlab,
}

impl std::fmt::Display for LoopProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duumbi => f.write_str("duumbi"),
            Self::Github => f.write_str("github"),
            Self::Gitlab => f.write_str("gitlab"),
        }
    }
}

/// Workflow type for a Loop run or artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowKind {
    /// Intake analysis.
    Intake,
    /// Specification generation.
    Spec,
    /// Change review.
    Review,
    /// Closure evidence.
    Closure,
}

/// State of a native Loop run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopRunState {
    /// Queued for later work.
    Queued,
    /// Currently running.
    Running,
    /// Waiting for user/product input.
    NeedsInput,
    /// Waiting for configuration or operator action.
    NeedsAction,
    /// Blocked before side effects.
    Blocked,
    /// Failed after starting.
    Failed,
    /// Cancelled by a user or operator.
    Cancelled,
    /// Completed successfully.
    Completed,
    /// Replaced by a newer run.
    Superseded,
}

impl std::fmt::Display for LoopRunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => f.write_str("queued"),
            Self::Running => f.write_str("running"),
            Self::NeedsInput => f.write_str("needs_input"),
            Self::NeedsAction => f.write_str("needs_action"),
            Self::Blocked => f.write_str("blocked"),
            Self::Failed => f.write_str("failed"),
            Self::Cancelled => f.write_str("cancelled"),
            Self::Completed => f.write_str("completed"),
            Self::Superseded => f.write_str("superseded"),
        }
    }
}

/// Kind of artifact carried in a Loop artifact envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Intake analysis artifact.
    Intake,
    /// Product specification artifact.
    ProductSpec,
    /// Technical specification artifact.
    TechnicalSpec,
    /// BDD/Gherkin artifact reference.
    Bdd,
    /// Review artifact.
    Review,
    /// Closure artifact.
    Closure,
    /// Run metadata artifact.
    Metadata,
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Intake => f.write_str("intake"),
            Self::ProductSpec => f.write_str("product_spec"),
            Self::TechnicalSpec => f.write_str("technical_spec"),
            Self::Bdd => f.write_str("bdd"),
            Self::Review => f.write_str("review"),
            Self::Closure => f.write_str("closure"),
            Self::Metadata => f.write_str("metadata"),
        }
    }
}

/// Source that produced or backs a provider-core work item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WorkItemSource {
    /// DUUMBI runtime intent stored under `.duumbi/intents`.
    DuumbiIntent {
        /// Intent slug.
        slug: String,
    },
    /// Session ledger entry.
    SessionLedger {
        /// Session identifier.
        session_id: String,
    },
    /// Explicit local request.
    LocalRequest {
        /// Local request description.
        description: String,
    },
    /// Optional external adapter reference.
    Adapter {
        /// Adapter provider kind.
        provider: LoopProviderKind,
        /// Non-secret external reference.
        external_ref: String,
    },
}

/// Provider-neutral work item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    /// Stable provider-core work item id.
    pub id: String,
    /// Provider kind that loaded the work item.
    pub provider_kind: LoopProviderKind,
    /// Provider-specific source mapped into provider-core.
    pub source: WorkItemSource,
    /// Human-readable title.
    pub title: String,
    /// Native status label.
    pub status: String,
    /// Stable artifact references already linked to the work item.
    pub artifact_refs: Vec<ArtifactRef>,
}

/// Context source category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSourceKind {
    /// JSON-LD graph file.
    Graph,
    /// BDD feature artifact.
    Bdd,
    /// Knowledge-base entry.
    Knowledge,
    /// Registry or dependency metadata.
    Registry,
    /// Snapshot evidence.
    Snapshot,
    /// Session ledger state.
    Session,
}

/// Bounded source reference used by native Loop context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSource {
    /// Source category.
    pub kind: ContextSourceKind,
    /// Workspace-relative path or stable source id.
    pub reference: String,
    /// Short non-secret summary.
    pub summary: String,
    /// Whether the summary is truncated.
    pub truncated: bool,
}

/// Context index produced by provider-duumbi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextIndex {
    /// Work item id this index belongs to.
    pub work_item_id: String,
    /// Graph source refs.
    pub graph_sources: Vec<ContextSource>,
    /// BDD source refs.
    pub bdd_sources: Vec<ContextSource>,
    /// Knowledge source refs.
    pub knowledge_sources: Vec<ContextSource>,
    /// Registry metadata refs.
    pub registry_sources: Vec<ContextSource>,
    /// Snapshot refs.
    pub snapshot_sources: Vec<ContextSource>,
    /// Session refs.
    pub session_sources: Vec<ContextSource>,
    /// Optional semantic graph hash when a graph exists and parses.
    pub graph_semantic_hash: Option<String>,
}

impl ContextIndex {
    /// Returns every source in deterministic display order.
    #[must_use]
    pub fn all_sources(&self) -> Vec<ContextSource> {
        let mut sources = Vec::new();
        sources.extend(self.graph_sources.clone());
        sources.extend(self.bdd_sources.clone());
        sources.extend(self.knowledge_sources.clone());
        sources.extend(self.registry_sources.clone());
        sources.extend(self.snapshot_sources.clone());
        sources.extend(self.session_sources.clone());
        sources
    }
}

/// Link to an artifact or source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Artifact kind.
    pub artifact_kind: ArtifactKind,
    /// Workspace-relative artifact path or stable external reference.
    pub path: String,
    /// Human-readable label.
    pub label: String,
}

/// Schema-versioned provider-neutral artifact envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    /// Artifact schema version.
    pub schema_version: String,
    /// Artifact kind.
    pub artifact_kind: ArtifactKind,
    /// Provider kind.
    pub provider_kind: LoopProviderKind,
    /// Work item id.
    pub work_item_id: String,
    /// Run id.
    pub run_id: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Bounded source refs.
    pub sources: Vec<ContextSource>,
    /// Structured artifact body.
    pub body: Value,
    /// Linked artifact refs.
    pub links: Vec<ArtifactRef>,
}

/// Review target kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewTargetKind {
    /// GraphPatch review target.
    GraphPatch,
    /// Graph snapshot diff review target.
    GraphSnapshotDiff,
    /// Generated artifact diff review target.
    GeneratedArtifactDiff,
}

/// Provider-neutral change set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ChangeSet {
    /// A proposed GraphPatch.
    GraphPatch {
        /// Number of GraphPatch operations.
        operation_count: usize,
        /// Affected graph node ids where known.
        affected_nodes: Vec<String>,
    },
    /// A graph snapshot diff.
    GraphSnapshotDiff {
        /// Before snapshot reference.
        before: String,
        /// After snapshot reference.
        after: String,
    },
    /// A generated artifact diff.
    GeneratedArtifactDiff {
        /// Before artifact reference.
        before: String,
        /// After artifact reference.
        after: String,
    },
}

/// Provider-neutral review target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewTarget {
    /// Review target kind.
    pub kind: ReviewTargetKind,
    /// Work item id.
    pub work_item_id: String,
    /// Change set under review.
    pub change_set: ChangeSet,
    /// Source refs used to build this target.
    pub sources: Vec<ContextSource>,
}

/// Native intake/spec run result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRunResult {
    /// Stable run id.
    pub run_id: String,
    /// Final run state.
    pub state: LoopRunState,
    /// Artifact refs written by the run.
    pub artifacts: Vec<ArtifactRef>,
    /// Blocking reasons when the run did not complete.
    pub blocking_reasons: Vec<String>,
    /// Non-secret context summary.
    pub context_summary: Vec<String>,
}

/// Native Loop provider implementation backed by a local DUUMBI workspace.
pub struct DuumbiProvider {
    workspace: PathBuf,
}

impl DuumbiProvider {
    /// Creates a provider-duumbi instance rooted at `workspace`.
    #[must_use]
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }

    /// Returns this provider's kind.
    #[must_use]
    pub const fn provider_kind(&self) -> LoopProviderKind {
        LoopProviderKind::Duumbi
    }

    /// Loads an existing DUUMBI intent as a provider-core work item.
    ///
    /// # Errors
    ///
    /// Returns an error when the intent cannot be loaded.
    pub fn load_work_item_from_intent(&self, slug: &str) -> Result<WorkItem, LoopNativeError> {
        let spec = intent::load_intent(&self.workspace, slug)?;
        let mut item = work_item_from_intent(slug, &spec);
        item.provider_kind = self.provider_kind();
        Ok(item)
    }

    /// Loads native DUUMBI context for a work item.
    ///
    /// # Errors
    ///
    /// Returns an error when referenced local artifacts cannot be read.
    pub fn load_context_index(&self, item: &WorkItem) -> Result<ContextIndex, LoopNativeError> {
        let slug = slug_from_work_item(item)?;
        let spec = intent::load_intent(&self.workspace, slug)?;
        let bdd_report = load_bdd_report(&spec, &self.workspace, slug);
        self.load_context_index_from_bdd(item, &bdd_report)
    }

    fn load_context_index_from_bdd(
        &self,
        item: &WorkItem,
        bdd_report: &BddReadinessReport,
    ) -> Result<ContextIndex, LoopNativeError> {
        let graph_sources = collect_graph_sources(&self.workspace)?;
        let graph_semantic_hash = graph_semantic_hash(&self.workspace, &graph_sources);
        let bdd_sources = bdd_report
            .feature_files
            .iter()
            .map(|feature| ContextSource {
                kind: ContextSourceKind::Bdd,
                reference: workspace_relative_or_display(&self.workspace, &feature.path),
                summary: format!(
                    "{} scenario(s){}",
                    feature.scenarios.len(),
                    feature
                        .feature
                        .as_ref()
                        .map(|title| format!(" for {title}"))
                        .unwrap_or_default()
                ),
                truncated: false,
            })
            .collect();
        let knowledge_sources = collect_knowledge_sources(&self.workspace);
        let registry_sources = collect_registry_sources(&self.workspace);
        let snapshot_sources = collect_snapshot_sources(&self.workspace)?;
        let session_sources = collect_session_sources(&self.workspace)?;

        Ok(ContextIndex {
            work_item_id: item.id.clone(),
            graph_sources,
            bdd_sources,
            knowledge_sources,
            registry_sources,
            snapshot_sources,
            session_sources,
            graph_semantic_hash,
        })
    }

    /// Runs the deterministic native intake+spec MVP for an intent.
    ///
    /// No external LLM provider or Git provider is constructed. Broken explicit
    /// BDD references block before artifacts are written.
    ///
    /// # Errors
    ///
    /// Returns an error when workspace I/O or serialization fails.
    pub fn run_intake_spec(&self, slug: &str) -> Result<NativeRunResult, LoopNativeError> {
        let spec = intent::load_intent(&self.workspace, slug)?;
        let work_item = work_item_from_intent(slug, &spec);
        let bdd_report = load_bdd_report(&spec, &self.workspace, slug);
        let blocking_reasons = if bdd_report.readiness == BddReadiness::Blocked {
            bdd_report
                .issues
                .iter()
                .map(|issue| format!("{}: {}", issue.code, issue.message))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if !blocking_reasons.is_empty() {
            return Ok(NativeRunResult {
                run_id: native_run_id(slug),
                state: LoopRunState::Blocked,
                artifacts: Vec::new(),
                blocking_reasons,
                context_summary: Vec::new(),
            });
        }

        let context = self.load_context_index_from_bdd(&work_item, &bdd_report)?;
        let run_id = native_run_id(slug);
        let created_at = Utc::now().to_rfc3339();
        let artifacts_dir = self
            .workspace
            .join(".duumbi")
            .join("loop")
            .join("runs")
            .join(&run_id)
            .join("artifacts");
        fs::create_dir_all(&artifacts_dir).map_err(|source| LoopNativeError::Io {
            path: artifacts_dir.display().to_string(),
            source,
        })?;

        let source_refs = context.all_sources();
        let intake_body = intake_body(&spec, &context);
        let product_body = product_spec_body(&spec, &bdd_report);
        let technical_body = technical_spec_body(&spec, &bdd_report, &context);

        let intake_ref = write_artifact_pair(
            &self.workspace,
            &artifacts_dir,
            ArtifactKind::Intake,
            "intake",
            ArtifactPairInput {
                work_item_id: &work_item.id,
                run_id: &run_id,
                created_at: &created_at,
                sources: source_refs.clone(),
                body: intake_body,
                markdown: render_intake_markdown(&spec, &context),
            },
        )?;
        let product_ref = write_artifact_pair(
            &self.workspace,
            &artifacts_dir,
            ArtifactKind::ProductSpec,
            "product_spec",
            ArtifactPairInput {
                work_item_id: &work_item.id,
                run_id: &run_id,
                created_at: &created_at,
                sources: source_refs.clone(),
                body: product_body.clone(),
                markdown: render_product_spec_markdown(&spec, &bdd_report),
            },
        )?;
        let technical_ref = write_artifact_pair(
            &self.workspace,
            &artifacts_dir,
            ArtifactKind::TechnicalSpec,
            "technical_spec",
            ArtifactPairInput {
                work_item_id: &work_item.id,
                run_id: &run_id,
                created_at: &created_at,
                sources: source_refs.clone(),
                body: technical_body.clone(),
                markdown: render_technical_spec_markdown(&spec, &bdd_report, &context),
            },
        )?;

        let metadata = ArtifactEnvelope {
            schema_version: LOOP_ARTIFACT_SCHEMA_VERSION.to_string(),
            artifact_kind: ArtifactKind::Metadata,
            provider_kind: LoopProviderKind::Duumbi,
            work_item_id: work_item.id.clone(),
            run_id: run_id.clone(),
            created_at,
            sources: source_refs,
            body: json!({
                "run_state": LoopRunState::Completed,
                "workflow": WorkflowKind::Spec,
                "duumbi_model_label": DEFAULT_SPEC_MODEL_LABEL,
                "estimated_credits": 0,
                "final_credits": 0,
                "resolved_provider": null,
                "resolved_model": null,
                "resource_policy": resource_policy_body(),
                "product_spec": product_body,
                "technical_spec": technical_body,
            }),
            links: vec![
                intake_ref.clone(),
                product_ref.clone(),
                technical_ref.clone(),
            ],
        };
        let metadata_path = artifacts_dir.join("metadata.json");
        write_json_file(&metadata_path, &metadata)?;
        let metadata_ref = artifact_ref(
            &self.workspace,
            &metadata_path,
            ArtifactKind::Metadata,
            "Native Loop metadata",
        )?;

        Ok(NativeRunResult {
            run_id,
            state: LoopRunState::Completed,
            artifacts: vec![intake_ref, product_ref, technical_ref, metadata_ref],
            blocking_reasons: Vec::new(),
            context_summary: context_summary(&context),
        })
    }

    /// Builds a review target for a proposed GraphPatch.
    ///
    /// # Errors
    ///
    /// Returns an error when the intent cannot be loaded.
    pub fn graph_patch_review_target(
        &self,
        slug: &str,
        patch: &GraphPatch,
    ) -> Result<ReviewTarget, LoopNativeError> {
        let item = self.load_work_item_from_intent(slug)?;
        let mut sources = collect_graph_sources(&self.workspace)?;
        sources.extend(collect_snapshot_sources(&self.workspace)?);
        let affected_nodes = affected_nodes(patch);
        Ok(ReviewTarget {
            kind: ReviewTargetKind::GraphPatch,
            work_item_id: item.id,
            change_set: ChangeSet::GraphPatch {
                operation_count: patch.ops.len(),
                affected_nodes,
            },
            sources,
        })
    }
}

/// Runs native intake+spec through provider-duumbi.
///
/// # Errors
///
/// Returns an error when workspace I/O or serialization fails.
pub fn run_native_intake_spec(
    workspace: &Path,
    slug: &str,
) -> Result<NativeRunResult, LoopNativeError> {
    DuumbiProvider::new(workspace).run_intake_spec(slug)
}

/// Builds a provider-duumbi GraphPatch review target.
///
/// # Errors
///
/// Returns an error when the intent cannot be loaded.
pub fn graph_patch_review_target(
    workspace: &Path,
    slug: &str,
    patch: &GraphPatch,
) -> Result<ReviewTarget, LoopNativeError> {
    DuumbiProvider::new(workspace).graph_patch_review_target(slug, patch)
}

/// Validates a workspace-relative artifact reference.
///
/// # Errors
///
/// Returns an error for empty, absolute, parent-directory, or prefix paths.
pub fn validate_relative_artifact_ref(reference: &str) -> Result<(), LoopNativeError> {
    let path = Path::new(reference);
    if reference.trim().is_empty() || path.is_absolute() {
        return Err(LoopNativeError::UnsafeArtifactRef {
            reference: reference.to_string(),
        });
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(LoopNativeError::UnsafeArtifactRef {
                    reference: reference.to_string(),
                });
            }
        }
    }
    Ok(())
}

/// Errors from native Loop provider operations.
#[derive(Debug, thiserror::Error)]
pub enum LoopNativeError {
    /// Intent subsystem error.
    #[error(transparent)]
    Intent(#[from] intent::IntentError),
    /// File I/O error.
    #[error("I/O error at '{path}': {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// JSON serialization error.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    /// Unsafe artifact reference.
    #[error("unsafe artifact reference: {reference}")]
    UnsafeArtifactRef {
        /// Rejected reference.
        reference: String,
    },
    /// Work item is not a native DUUMBI intent.
    #[error("work item is not a DUUMBI intent")]
    UnsupportedWorkItem,
}

struct ArtifactPairInput<'a> {
    work_item_id: &'a str,
    run_id: &'a str,
    created_at: &'a str,
    sources: Vec<ContextSource>,
    body: Value,
    markdown: String,
}

fn work_item_from_intent(slug: &str, spec: &IntentSpec) -> WorkItem {
    WorkItem {
        id: format!("duumbi:intent/{slug}"),
        provider_kind: LoopProviderKind::Duumbi,
        source: WorkItemSource::DuumbiIntent {
            slug: slug.to_string(),
        },
        title: truncate(&spec.intent, 96),
        status: spec.status.to_string(),
        artifact_refs: spec
            .bdd
            .feature_files
            .iter()
            .map(|path| ArtifactRef {
                artifact_kind: ArtifactKind::Bdd,
                path: path.clone(),
                label: "BDD feature".to_string(),
            })
            .collect(),
    }
}

fn slug_from_work_item(item: &WorkItem) -> Result<&str, LoopNativeError> {
    match &item.source {
        WorkItemSource::DuumbiIntent { slug } => Ok(slug),
        _ => Err(LoopNativeError::UnsupportedWorkItem),
    }
}

fn native_run_id(slug: &str) -> String {
    format!("duumbi-native-{}", sanitize_component(slug))
}

fn collect_graph_sources(workspace: &Path) -> Result<Vec<ContextSource>, LoopNativeError> {
    let graph_dir = workspace.join(".duumbi").join("graph");
    if !graph_dir.exists() {
        return Ok(Vec::new());
    }
    let mut sources = Vec::new();
    collect_jsonld_sources(workspace, &graph_dir, 0, &mut sources)?;
    sources.sort_by(|a, b| a.reference.cmp(&b.reference));
    Ok(sources)
}

fn collect_jsonld_sources(
    workspace: &Path,
    dir: &Path,
    depth: usize,
    sources: &mut Vec<ContextSource>,
) -> Result<(), LoopNativeError> {
    let entries = fs::read_dir(dir).map_err(|source| LoopNativeError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| LoopNativeError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| LoopNativeError::Io {
            path: path.display().to_string(),
            source,
        })?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if depth < MAX_GRAPH_SOURCE_DEPTH {
                collect_jsonld_sources(workspace, &path, depth + 1, sources)?;
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonld") {
            let metadata = fs::metadata(&path).map_err(|source| LoopNativeError::Io {
                path: path.display().to_string(),
                source,
            })?;
            sources.push(ContextSource {
                kind: ContextSourceKind::Graph,
                reference: workspace_relative_or_display(workspace, &path),
                summary: format!("JSON-LD graph module ({} bytes)", metadata.len()),
                truncated: false,
            });
        }
    }
    Ok(())
}

fn graph_semantic_hash(workspace: &Path, graph_sources: &[ContextSource]) -> Option<String> {
    if graph_sources.is_empty() {
        return None;
    }
    let mut modules = Vec::new();
    for source in graph_sources {
        let path = workspace.join(&source.reference);
        let content = fs::read_to_string(&path).ok()?;
        let value = serde_json::from_str::<Value>(&content).ok()?;
        modules.push(json!({
            "path": source.reference,
            "semantic_hash": crate::hash::semantic_hash_value(&value),
        }));
    }
    Some(crate::hash::semantic_hash_value(
        &json!({ "modules": modules }),
    ))
}

fn collect_knowledge_sources(workspace: &Path) -> Vec<ContextSource> {
    let store = KnowledgeStore::open_existing(workspace);
    let mut sources = store
        .load_all()
        .into_iter()
        .map(|node| ContextSource {
            kind: ContextSourceKind::Knowledge,
            reference: node.id().to_string(),
            summary: knowledge_summary(&node),
            truncated: false,
        })
        .collect::<Vec<_>>();
    sources.sort_by(|a, b| a.reference.cmp(&b.reference));
    sources
}

fn collect_registry_sources(workspace: &Path) -> Vec<ContextSource> {
    [
        ".duumbi/config.toml",
        ".duumbi/deps.lock",
        "manifest.toml",
        "duumbi.toml",
    ]
    .iter()
    .filter_map(|relative| {
        let path = workspace.join(relative);
        path.exists().then(|| ContextSource {
            kind: ContextSourceKind::Registry,
            reference: (*relative).to_string(),
            summary: "registry/dependency metadata".to_string(),
            truncated: false,
        })
    })
    .collect()
}

fn collect_snapshot_sources(workspace: &Path) -> Result<Vec<ContextSource>, LoopNativeError> {
    let history_dir = workspace.join(".duumbi").join("history");
    if !history_dir.exists() {
        return Ok(Vec::new());
    }
    let mut sources = Vec::new();
    let entries = fs::read_dir(&history_dir).map_err(|source| LoopNativeError::Io {
        path: history_dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| LoopNativeError::Io {
            path: history_dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonld") {
            sources.push(ContextSource {
                kind: ContextSourceKind::Snapshot,
                reference: workspace_relative_or_display(workspace, &path),
                summary: "graph snapshot".to_string(),
                truncated: false,
            });
        }
    }
    sources.sort_by(|a, b| a.reference.cmp(&b.reference));
    Ok(sources)
}

fn collect_session_sources(workspace: &Path) -> Result<Vec<ContextSource>, LoopNativeError> {
    let current = workspace
        .join(".duumbi")
        .join("session")
        .join("current.json");
    if !current.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&current).map_err(|source| LoopNativeError::Io {
        path: current.display().to_string(),
        source,
    })?;
    let summary = serde_json::from_str::<Value>(&content)
        .ok()
        .and_then(|value| {
            let session_id = value.get("session_id")?.as_str()?;
            let turns = value
                .get("turns")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            Some(format!("session {session_id} with {turns} turn(s)"))
        })
        .unwrap_or_else(|| "session ledger".to_string());
    Ok(vec![ContextSource {
        kind: ContextSourceKind::Session,
        reference: workspace_relative_or_display(workspace, &current),
        summary,
        truncated: false,
    }])
}

fn intake_body(spec: &IntentSpec, context: &ContextIndex) -> Value {
    json!({
        "summary": format!("Native DUUMBI Loop intake for: {}", spec.intent),
        "confidence": 0.78,
        "business_value": "Enables DUUMBI-native intake/spec without Git provider setup.",
        "effort": "bounded-local-mvp",
        "affected_areas": affected_areas(spec),
        "knowledge_citations": context.knowledge_sources,
        "graph_context": context.graph_sources,
        "registry_context": context.registry_sources,
        "questions": [],
        "recommended_next_action": "Proceed to native spec review.",
        "duumbi_model_label": DEFAULT_SPEC_MODEL_LABEL,
        "estimated_credits": 0,
        "resolved_provider": null,
        "resolved_model": null,
        "graph_semantic_hash": context.graph_semantic_hash,
    })
}

fn product_spec_body(
    spec: &IntentSpec,
    bdd_report: &crate::intent::bdd::BddReadinessReport,
) -> Value {
    json!({
        "summary": spec.intent,
        "goals": spec.acceptance_criteria,
        "non_goals": ["Git provider objects are not required for this native run."],
        "acceptance_criteria": spec.acceptance_criteria,
        "bdd_scenarios": bdd_report
            .feature_files
            .iter()
            .flat_map(|feature| feature.scenarios.iter().map(|scenario| scenario.name.clone()))
            .collect::<Vec<_>>(),
        "model_contract": {
            "user_facing_label": DEFAULT_SPEC_MODEL_LABEL,
            "resolved_provider": Value::Null,
            "resolved_model": Value::Null,
        },
    })
}

fn technical_spec_body(
    spec: &IntentSpec,
    bdd_report: &crate::intent::bdd::BddReadinessReport,
    context: &ContextIndex,
) -> Value {
    json!({
        "architecture_impact": "provider-duumbi maps IntentSpec, graph, BDD, knowledge, registry metadata, session, and snapshots into provider-core.",
        "affected_modules": affected_areas(spec),
        "bdd_to_test_mapping": bdd_report.coverage.iter().map(|coverage| {
            json!({
                "scenario": coverage.scenario,
                "classification": coverage.classification.label(),
                "verifier_tests": coverage.verifier_tests,
            })
        }).collect::<Vec<_>>(),
        "live_e2e_plan": [
            "Load a local DUUMBI intent.",
            "Load graph, BDD, knowledge, registry, snapshot, and session context.",
            "Write native intake/spec artifacts.",
            "Review a GraphPatch target without Git provider state."
        ],
        "resource_policy": resource_policy_body(),
        "context_counts": {
            "graph": context.graph_sources.len(),
            "bdd": context.bdd_sources.len(),
            "knowledge": context.knowledge_sources.len(),
            "registry": context.registry_sources.len(),
            "snapshot": context.snapshot_sources.len(),
            "session": context.session_sources.len(),
        },
    })
}

fn resource_policy_body() -> Value {
    json!({
        "external_llm_calls": 0,
        "expected_external_cost_usd": 0,
        "git_provider_required": false,
        "spec_model_label": DEFAULT_SPEC_MODEL_LABEL,
        "review_model_label": DEFAULT_REVIEW_MODEL_LABEL
    })
}

fn affected_areas(spec: &IntentSpec) -> Vec<Value> {
    spec.modules
        .create
        .iter()
        .map(|module| {
            json!({
                "module": module,
                "operation": "create",
                "reason": "listed in IntentSpec.modules.create"
            })
        })
        .chain(spec.modules.modify.iter().map(|module| {
            json!({
                "module": module,
                "operation": "modify",
                "reason": "listed in IntentSpec.modules.modify"
            })
        }))
        .collect()
}

fn context_summary(context: &ContextIndex) -> Vec<String> {
    vec![
        format!("graph_sources={}", context.graph_sources.len()),
        format!("bdd_sources={}", context.bdd_sources.len()),
        format!("knowledge_sources={}", context.knowledge_sources.len()),
        format!("registry_sources={}", context.registry_sources.len()),
        format!("snapshot_sources={}", context.snapshot_sources.len()),
        format!("session_sources={}", context.session_sources.len()),
    ]
}

fn render_intake_markdown(spec: &IntentSpec, context: &ContextIndex) -> String {
    let mut lines = vec![
        format!("# Native DUUMBI Loop Intake: {}", spec.intent),
        String::new(),
        "## Summary".to_string(),
        format!("Native provider-duumbi intake for `{}`.", spec.intent),
        String::new(),
        "## Sources".to_string(),
    ];
    for source in context.all_sources() {
        lines.push(format!(
            "- {:?}: {} - {}",
            source.kind, source.reference, source.summary
        ));
    }
    lines.extend([
        String::new(),
        "## Model And Cost".to_string(),
        format!("- DUUMBI model label: `{DEFAULT_SPEC_MODEL_LABEL}`"),
        "- External LLM calls: 0".to_string(),
        "- Estimated credits: 0".to_string(),
    ]);
    lines.join("\n") + "\n"
}

fn render_product_spec_markdown(
    spec: &IntentSpec,
    bdd_report: &crate::intent::bdd::BddReadinessReport,
) -> String {
    let mut lines = vec![
        format!("# Native DUUMBI Loop Product Spec: {}", spec.intent),
        String::new(),
        "## Acceptance Criteria".to_string(),
    ];
    if spec.acceptance_criteria.is_empty() {
        lines.push("- No acceptance criteria were recorded on the source intent.".to_string());
    } else {
        lines.extend(
            spec.acceptance_criteria
                .iter()
                .map(|criterion| format!("- {criterion}")),
        );
    }
    lines.extend([String::new(), "## BDD Scenarios".to_string()]);
    for feature in &bdd_report.feature_files {
        for scenario in &feature.scenarios {
            lines.push(format!("- {}", scenario.name));
        }
    }
    lines.join("\n") + "\n"
}

fn render_technical_spec_markdown(
    spec: &IntentSpec,
    bdd_report: &crate::intent::bdd::BddReadinessReport,
    context: &ContextIndex,
) -> String {
    let mut lines = vec![
        format!("# Native DUUMBI Loop Technical Spec: {}", spec.intent),
        String::new(),
        "## Architecture Impact".to_string(),
        "provider-duumbi maps native DUUMBI workspace state into provider-core.".to_string(),
        String::new(),
        "## BDD-To-Test Mapping".to_string(),
    ];
    for coverage in &bdd_report.coverage {
        let tests = if coverage.verifier_tests.is_empty() {
            "broader evidence required".to_string()
        } else {
            coverage.verifier_tests.join(", ")
        };
        lines.push(format!(
            "- {}: {} ({tests})",
            coverage.scenario,
            coverage.classification.label()
        ));
    }
    lines.extend([
        String::new(),
        "## Live E2E Plan".to_string(),
        "- Run `duumbi loop intake-spec <intent> --json` in a local workspace.".to_string(),
        "- Confirm artifacts are written under `.duumbi/loop/runs/`.".to_string(),
        "- Confirm no Git provider or external LLM provider is required.".to_string(),
        String::new(),
        "## Context Counts".to_string(),
        format!("- Graph: {}", context.graph_sources.len()),
        format!("- BDD: {}", context.bdd_sources.len()),
        format!("- Knowledge: {}", context.knowledge_sources.len()),
        format!("- Registry: {}", context.registry_sources.len()),
    ]);
    lines.join("\n") + "\n"
}

fn write_artifact_pair(
    workspace: &Path,
    artifacts_dir: &Path,
    kind: ArtifactKind,
    stem: &str,
    input: ArtifactPairInput<'_>,
) -> Result<ArtifactRef, LoopNativeError> {
    let json_path = artifacts_dir.join(format!("{stem}.json"));
    let markdown_path = artifacts_dir.join(format!("{stem}.md"));
    let markdown_ref = artifact_ref(workspace, &markdown_path, kind, &format!("{kind} markdown"))?;
    let envelope = ArtifactEnvelope {
        schema_version: LOOP_ARTIFACT_SCHEMA_VERSION.to_string(),
        artifact_kind: kind,
        provider_kind: LoopProviderKind::Duumbi,
        work_item_id: input.work_item_id.to_string(),
        run_id: input.run_id.to_string(),
        created_at: input.created_at.to_string(),
        sources: input.sources,
        body: input.body,
        links: vec![markdown_ref.clone()],
    };
    write_json_file(&json_path, &envelope)?;
    write_text_file(&markdown_path, &input.markdown)?;
    artifact_ref(workspace, &json_path, kind, &format!("{kind} artifact"))
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), LoopNativeError> {
    let json = serde_json::to_string_pretty(value)?;
    write_text_file(path, &(json + "\n"))
}

fn write_text_file(path: &Path, contents: &str) -> Result<(), LoopNativeError> {
    fs::write(path, contents).map_err(|source| LoopNativeError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn artifact_ref(
    workspace: &Path,
    path: &Path,
    artifact_kind: ArtifactKind,
    label: &str,
) -> Result<ArtifactRef, LoopNativeError> {
    let path = workspace_relative_or_display(workspace, path);
    validate_relative_artifact_ref(&path)?;
    Ok(ArtifactRef {
        artifact_kind,
        path,
        label: label.to_string(),
    })
}

fn affected_nodes(patch: &GraphPatch) -> Vec<String> {
    let mut nodes = BTreeSet::new();
    for op in &patch.ops {
        match op {
            PatchOp::AddFunction { function } => push_json_id(&mut nodes, function),
            PatchOp::AddBlock { function_id, block } => {
                nodes.insert(function_id.clone());
                push_json_id(&mut nodes, block);
            }
            PatchOp::AddOp { block_id, op } => {
                nodes.insert(block_id.clone());
                push_json_id(&mut nodes, op);
            }
            PatchOp::ModifyOp { node_id, .. }
            | PatchOp::RemoveNode { node_id }
            | PatchOp::SetEdge { node_id, .. } => {
                nodes.insert(node_id.clone());
            }
            PatchOp::ReplaceBlock { block_id, .. } => {
                nodes.insert(block_id.clone());
            }
        }
        if let PatchOp::SetEdge { target_id, .. } = op {
            nodes.insert(target_id.clone());
        }
    }
    nodes.into_iter().collect()
}

fn push_json_id(nodes: &mut BTreeSet<String>, value: &Value) {
    if let Some(id) = value.get("@id").and_then(Value::as_str) {
        nodes.insert(id.to_string());
    }
}

fn knowledge_summary(node: &KnowledgeNode) -> String {
    let status = if knowledge_tags(node).contains(&"candidate") {
        "candidate"
    } else {
        "accepted"
    };
    match node {
        KnowledgeNode::Success(record) => {
            format!("{status} success: {}", truncate(&record.request, 80))
        }
        KnowledgeNode::Failure(record) => {
            format!("{status} failure: {}", truncate(&record.error_summary, 80))
        }
        KnowledgeNode::Decision(record) => {
            format!("{status} decision: {}", truncate(&record.decision, 80))
        }
        KnowledgeNode::Pattern(record) => {
            format!("{status} pattern: {}", truncate(&record.name, 80))
        }
    }
}

fn knowledge_tags(node: &KnowledgeNode) -> Vec<&str> {
    match node {
        KnowledgeNode::Decision(record) => record.tags.iter().map(String::as_str).collect(),
        KnowledgeNode::Pattern(record) => record.tags.iter().map(String::as_str).collect(),
        KnowledgeNode::Success(_) | KnowledgeNode::Failure(_) => Vec::new(),
    }
}

fn workspace_relative_or_display(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .map(|relative| {
            relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_else(|_| path.display().to_string())
}

fn sanitize_component(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed.to_string()
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let prefix = value
        .chars()
        .take(max.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::spec::{IntentBdd, IntentModules, IntentStatus};

    fn sample_spec() -> IntentSpec {
        IntentSpec {
            intent: "Build native loop intake".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["Native intake works without Git".to_string()],
            modules: IntentModules {
                create: vec!["loop/native".to_string()],
                modify: vec![],
            },
            test_cases: vec![],
            dependencies: vec![],
            bdd: IntentBdd {
                feature_files: vec!["features/native.feature".to_string()],
            },
            context: None,
            created_at: Some("2026-06-19T00:00:00Z".to_string()),
            execution: None,
        }
    }

    #[test]
    fn work_item_uses_provider_core_names() {
        let item = work_item_from_intent("native-loop", &sample_spec());
        let json = serde_json::to_string(&item).expect("work item serializes");

        assert_eq!(item.id, "duumbi:intent/native-loop");
        assert!(!json.contains("PullRequest"));
        assert!(!json.contains("MergeCommit"));
    }

    #[test]
    fn artifact_ref_rejects_path_escape() {
        assert!(validate_relative_artifact_ref(".duumbi/loop/artifact.json").is_ok());
        assert!(validate_relative_artifact_ref("../secret").is_err());
        assert!(validate_relative_artifact_ref("/tmp/secret").is_err());
    }

    #[test]
    fn graph_patch_affected_nodes_are_stable_and_sorted() {
        let patch = GraphPatch {
            ops: vec![
                PatchOp::SetEdge {
                    node_id: "duumbi:main/a".to_string(),
                    field: "duumbi:next".to_string(),
                    target_id: "duumbi:main/c".to_string(),
                },
                PatchOp::ModifyOp {
                    node_id: "duumbi:main/b".to_string(),
                    field: "duumbi:value".to_string(),
                    value: json!(1),
                },
            ],
        };

        assert_eq!(
            affected_nodes(&patch),
            vec![
                "duumbi:main/a".to_string(),
                "duumbi:main/b".to_string(),
                "duumbi:main/c".to_string(),
            ]
        );
    }
}
