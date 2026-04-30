//! Persistent model-access metadata for provider credentials.
//!
//! Access is intentionally tracked separately from model performance. A model
//! can be high quality but unusable for the current key or subscription; the
//! router must know that before considering performance statistics.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{ProviderConfig, ProviderKind};

const MODEL_ACCESS_DIR: &str = ".duumbi/knowledge/model-access";
const MODEL_ACCESS_FILE: &str = "current.json";
const MODEL_ACCESS_EVENTS_FILE: &str = "events.jsonl";

/// Version of the probe contract written by this Duumbi release.
pub const MODEL_ACCESS_PROBE_VERSION: &str = "2026-04-provider-model-access-v1";

/// Accessibility state for a provider/model under the configured credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelAccessStatus {
    /// The model accepted a Duumbi-compatible request.
    Accessible,
    /// The provider reported that this key/subscription cannot use the model.
    Denied,
    /// The provider credential is invalid or not authorized.
    AuthFailed,
    /// The probe could not determine access due to transient or ambiguous failure.
    Unknown,
}

/// One model probe result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAccessProbeResult {
    /// Provider that served the probe.
    pub provider: String,
    /// Concrete model probed.
    pub model: String,
    /// Probe status.
    pub status: ModelAccessStatus,
    /// Compact provider-neutral reason code.
    pub reason_code: Option<String>,
    /// Sanitized provider error snippet.
    pub message: Option<String>,
    /// Probe timestamp.
    pub checked_at: DateTime<Utc>,
}

/// Append-only event for a model access probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAccessEvent {
    /// Non-reversible credential fingerprint.
    pub credential_fingerprint: String,
    /// Provider that served the probe.
    pub provider: String,
    /// Concrete model probed.
    pub model: String,
    /// Probe status.
    pub status: ModelAccessStatus,
    /// Compact provider-neutral reason code.
    pub reason_code: Option<String>,
    /// Sanitized provider error snippet.
    pub message: Option<String>,
    /// Probe timestamp.
    pub checked_at: DateTime<Utc>,
}

impl ModelAccessProbeResult {
    /// Creates a new probe result.
    #[must_use]
    pub fn new(
        provider: &ProviderKind,
        model: impl Into<String>,
        status: ModelAccessStatus,
        reason_code: Option<String>,
        message: Option<String>,
    ) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.into(),
            status,
            reason_code,
            message,
            checked_at: Utc::now(),
        }
    }
}

/// Stored model access record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAccessRecord {
    /// Non-reversible credential fingerprint.
    pub credential_fingerprint: String,
    /// Provider name.
    pub provider: String,
    /// Concrete model.
    pub model: String,
    /// Last observed access status.
    pub status: ModelAccessStatus,
    /// Last compact reason code.
    pub reason_code: Option<String>,
    /// Last sanitized provider message.
    pub message: Option<String>,
    /// Probe/catalog version.
    pub probe_version: String,
    /// Last time this model was checked.
    pub last_checked: DateTime<Utc>,
    /// Last time this model was confirmed accessible.
    pub last_success: Option<DateTime<Utc>>,
}

/// Complete access metadata file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAccessDb {
    /// Records keyed by `credential_fingerprint|provider|model`.
    pub records: HashMap<String, ModelAccessRecord>,
}

/// Summary of a provider credential probe across all catalog models.
#[derive(Debug, Clone)]
pub struct ProviderProbeReport {
    /// Provider that was probed.
    pub provider: ProviderKind,
    /// Per-model results.
    pub results: Vec<ModelAccessProbeResult>,
}

impl ProviderProbeReport {
    /// Number of models confirmed accessible.
    #[must_use]
    pub fn accessible_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.status == ModelAccessStatus::Accessible)
            .count()
    }

    /// Number of models that were probed.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.results.len()
    }

    /// Returns true if the credential itself failed authentication.
    #[must_use]
    pub fn has_auth_failure(&self) -> bool {
        self.results
            .iter()
            .any(|result| result.status == ModelAccessStatus::AuthFailed)
    }

    /// User-facing success summary.
    #[must_use]
    pub fn success_message(&self) -> String {
        format!(
            "{} registered. {} of {} Duumbi models are available.",
            self.provider,
            self.accessible_count(),
            self.total_count()
        )
    }
}

/// Reads and writes model access metadata.
pub struct ModelAccessStore;

impl ModelAccessStore {
    /// Records a full provider probe report.
    pub fn record_report(
        credential_fingerprint: &str,
        report: &ProviderProbeReport,
    ) -> std::io::Result<()> {
        Self::record_report_at(&duumbi_home(), credential_fingerprint, report)
    }

    /// Records a full provider probe report under an explicit home path.
    pub fn record_report_at(
        home: &Path,
        credential_fingerprint: &str,
        report: &ProviderProbeReport,
    ) -> std::io::Result<()> {
        let dir = model_access_dir_for_home(home);
        fs::create_dir_all(&dir)?;

        let mut events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(MODEL_ACCESS_EVENTS_FILE))?;

        let mut db = Self::load_db_from_home(home);
        for result in &report.results {
            let event = ModelAccessEvent {
                credential_fingerprint: credential_fingerprint.to_string(),
                provider: result.provider.clone(),
                model: result.model.clone(),
                status: result.status,
                reason_code: result.reason_code.clone(),
                message: result.message.clone(),
                checked_at: result.checked_at,
            };
            let line = serde_json::to_string(&event).map_err(std::io::Error::other)?;
            writeln!(events, "{line}")?;
            upsert_record(&mut db, credential_fingerprint, result);
        }

        let json = serde_json::to_string_pretty(&db).map_err(std::io::Error::other)?;
        fs::write(dir.join(MODEL_ACCESS_FILE), json)?;
        Ok(())
    }

    /// Loads model access metadata, returning an empty DB when absent.
    #[must_use]
    pub fn load_db() -> ModelAccessDb {
        Self::load_db_from_home(&duumbi_home())
    }

    /// Loads model access metadata from an explicit home path.
    #[must_use]
    pub fn load_db_from_home(home: &Path) -> ModelAccessDb {
        let path = model_access_dir_for_home(home).join(MODEL_ACCESS_FILE);
        let Ok(content) = fs::read_to_string(path) else {
            return ModelAccessDb::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    /// Returns models confirmed accessible for a provider.
    #[must_use]
    pub fn accessible_models(
        provider: &ProviderKind,
        credential_fingerprint: &str,
    ) -> HashSet<String> {
        Self::models_with_status(
            provider,
            credential_fingerprint,
            ModelAccessStatus::Accessible,
        )
    }

    /// Returns models denied for a provider.
    #[must_use]
    pub fn denied_models(provider: &ProviderKind, credential_fingerprint: &str) -> HashSet<String> {
        Self::models_with_status(provider, credential_fingerprint, ModelAccessStatus::Denied)
    }

    /// Returns models confirmed accessible for a provider from an explicit home path.
    #[must_use]
    pub fn accessible_models_from_home(
        home: &Path,
        provider: &ProviderKind,
        credential_fingerprint: &str,
    ) -> HashSet<String> {
        Self::models_with_status_from_home(
            home,
            provider,
            credential_fingerprint,
            ModelAccessStatus::Accessible,
        )
    }

    /// Returns models denied for a provider from an explicit home path.
    #[must_use]
    pub fn denied_models_from_home(
        home: &Path,
        provider: &ProviderKind,
        credential_fingerprint: &str,
    ) -> HashSet<String> {
        Self::models_with_status_from_home(
            home,
            provider,
            credential_fingerprint,
            ModelAccessStatus::Denied,
        )
    }

    /// Removes current access records for a provider credential fingerprint.
    ///
    /// Probe events remain append-only; `current.json` is the authoritative
    /// routing snapshot and is safe to prune when a credential is removed.
    pub fn remove_provider_for_fingerprint(
        provider: &ProviderKind,
        credential_fingerprint: &str,
    ) -> std::io::Result<usize> {
        Self::remove_provider_for_fingerprint_at(&duumbi_home(), provider, credential_fingerprint)
    }

    /// Removes current access records for a provider credential under an explicit home path.
    pub fn remove_provider_for_fingerprint_at(
        home: &Path,
        provider: &ProviderKind,
        credential_fingerprint: &str,
    ) -> std::io::Result<usize> {
        let provider_name = provider.to_string();
        Self::retain_records_at(home, |record| {
            !(record.provider == provider_name
                && record.credential_fingerprint == credential_fingerprint)
        })
    }

    /// Removes all current access records for a provider.
    pub fn remove_provider(provider: &ProviderKind) -> std::io::Result<usize> {
        Self::remove_provider_at(&duumbi_home(), provider)
    }

    /// Removes all current access records for a provider under an explicit home path.
    pub fn remove_provider_at(home: &Path, provider: &ProviderKind) -> std::io::Result<usize> {
        let provider_name = provider.to_string();
        Self::retain_records_at(home, |record| record.provider != provider_name)
    }

    fn models_with_status(
        provider: &ProviderKind,
        credential_fingerprint: &str,
        status: ModelAccessStatus,
    ) -> HashSet<String> {
        Self::models_with_status_from_home(&duumbi_home(), provider, credential_fingerprint, status)
    }

    fn models_with_status_from_home(
        home: &Path,
        provider: &ProviderKind,
        credential_fingerprint: &str,
        status: ModelAccessStatus,
    ) -> HashSet<String> {
        let provider_name = provider.to_string();
        Self::load_db_from_home(home)
            .records
            .values()
            .filter(|record| {
                record.provider == provider_name
                    && record.credential_fingerprint == credential_fingerprint
                    && record.status == status
            })
            .map(|record| record.model.clone())
            .collect()
    }

    fn retain_records_at(
        home: &Path,
        mut keep: impl FnMut(&ModelAccessRecord) -> bool,
    ) -> std::io::Result<usize> {
        let dir = model_access_dir_for_home(home);
        let path = dir.join(MODEL_ACCESS_FILE);
        let mut db = Self::load_db_from_home(home);
        let before = db.records.len();
        db.records.retain(|_, record| keep(record));
        let removed = before.saturating_sub(db.records.len());
        if removed == 0 {
            return Ok(0);
        }

        fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(&db).map_err(std::io::Error::other)?;
        fs::write(path, json)?;
        Ok(removed)
    }
}

/// Returns a non-reversible fingerprint for a provider credential.
#[must_use]
pub fn credential_fingerprint_for_secret(
    provider: &ProviderKind,
    credential: &str,
    is_subscription: bool,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"duumbi:model-access:v1");
    hasher.update(provider.to_string().as_bytes());
    hasher.update(if is_subscription {
        b"subscription".as_slice()
    } else {
        b"api-key".as_slice()
    });
    hasher.update(credential.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    format!("sha256:{hex}")
}

/// Returns the active credential fingerprint for a provider config, if the
/// configured credential is available in the environment.
#[must_use]
pub fn credential_fingerprint_from_env(config: &ProviderConfig) -> Option<String> {
    if let Some(token_env) = config.auth_token_env.as_deref()
        && let Ok(token) = std::env::var(token_env)
    {
        return Some(credential_fingerprint_for_secret(
            &config.provider,
            &token,
            true,
        ));
    }
    std::env::var(&config.api_key_env)
        .ok()
        .map(|key| credential_fingerprint_for_secret(&config.provider, &key, false))
}

/// Returns the global model-access directory for a given home path.
#[must_use]
pub fn model_access_dir_for_home(home: &Path) -> PathBuf {
    home.join(MODEL_ACCESS_DIR)
}

fn duumbi_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn upsert_record(
    db: &mut ModelAccessDb,
    credential_fingerprint: &str,
    result: &ModelAccessProbeResult,
) {
    let key = access_key(credential_fingerprint, &result.provider, &result.model);
    let last_success = if result.status == ModelAccessStatus::Accessible {
        Some(result.checked_at)
    } else {
        db.records.get(&key).and_then(|record| record.last_success)
    };

    db.records.insert(
        key,
        ModelAccessRecord {
            credential_fingerprint: credential_fingerprint.to_string(),
            provider: result.provider.clone(),
            model: result.model.clone(),
            status: result.status,
            reason_code: result.reason_code.clone(),
            message: result.message.clone(),
            probe_version: MODEL_ACCESS_PROBE_VERSION.to_string(),
            last_checked: result.checked_at,
            last_success,
        },
    );
}

fn access_key(credential_fingerprint: &str, provider: &str, model: &str) -> String {
    format!("{credential_fingerprint}|{provider}|{model}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn record_report_persists_events_and_accessible_models() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let report = ProviderProbeReport {
            provider: ProviderKind::MiniMax,
            results: vec![
                ModelAccessProbeResult::new(
                    &ProviderKind::MiniMax,
                    "MiniMax-M2.7",
                    ModelAccessStatus::Accessible,
                    None,
                    None,
                ),
                ModelAccessProbeResult::new(
                    &ProviderKind::MiniMax,
                    "MiniMax-M2.7-highspeed",
                    ModelAccessStatus::Denied,
                    Some("model_not_supported_by_plan".to_string()),
                    Some("plan does not support model".to_string()),
                ),
            ],
        };

        let fingerprint =
            credential_fingerprint_for_secret(&ProviderKind::MiniMax, "secret-key", false);
        ModelAccessStore::record_report_at(temp.path(), &fingerprint, &report)
            .expect("report must write");

        let accessible = ModelAccessStore::accessible_models_from_home(
            temp.path(),
            &ProviderKind::MiniMax,
            &fingerprint,
        );
        let denied = ModelAccessStore::denied_models_from_home(
            temp.path(),
            &ProviderKind::MiniMax,
            &fingerprint,
        );
        assert!(accessible.contains("MiniMax-M2.7"));
        assert!(denied.contains("MiniMax-M2.7-highspeed"));

        let access_dir = model_access_dir_for_home(temp.path());
        assert!(access_dir.ends_with(".duumbi/knowledge/model-access"));
        let events = fs::read_to_string(access_dir.join(MODEL_ACCESS_EVENTS_FILE))
            .expect("events must exist");
        assert_eq!(events.lines().count(), 2);
        assert!(!events.contains("secret-key"));
    }

    #[test]
    fn credential_fingerprint_change_separates_access_records() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let first = credential_fingerprint_for_secret(&ProviderKind::MiniMax, "first", false);
        let second = credential_fingerprint_for_secret(&ProviderKind::MiniMax, "second", false);
        assert_ne!(first, second);

        let report = ProviderProbeReport {
            provider: ProviderKind::MiniMax,
            results: vec![ModelAccessProbeResult::new(
                &ProviderKind::MiniMax,
                "MiniMax-M2.7",
                ModelAccessStatus::Accessible,
                None,
                None,
            )],
        };

        ModelAccessStore::record_report_at(temp.path(), &first, &report)
            .expect("first report must write");

        let first_access = ModelAccessStore::accessible_models_from_home(
            temp.path(),
            &ProviderKind::MiniMax,
            &first,
        );
        let second_access = ModelAccessStore::accessible_models_from_home(
            temp.path(),
            &ProviderKind::MiniMax,
            &second,
        );
        assert!(first_access.contains("MiniMax-M2.7"));
        assert!(second_access.is_empty());
    }

    #[test]
    fn remove_provider_for_fingerprint_prunes_current_snapshot_only() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let first = credential_fingerprint_for_secret(&ProviderKind::MiniMax, "first", false);
        let second = credential_fingerprint_for_secret(&ProviderKind::MiniMax, "second", false);
        let report = ProviderProbeReport {
            provider: ProviderKind::MiniMax,
            results: vec![ModelAccessProbeResult::new(
                &ProviderKind::MiniMax,
                "MiniMax-M2.7",
                ModelAccessStatus::Accessible,
                None,
                None,
            )],
        };

        ModelAccessStore::record_report_at(temp.path(), &first, &report)
            .expect("first report must write");
        ModelAccessStore::record_report_at(temp.path(), &second, &report)
            .expect("second report must write");
        let removed = ModelAccessStore::remove_provider_for_fingerprint_at(
            temp.path(),
            &ProviderKind::MiniMax,
            &first,
        )
        .expect("remove must write");

        assert_eq!(removed, 1);
        assert!(
            ModelAccessStore::accessible_models_from_home(
                temp.path(),
                &ProviderKind::MiniMax,
                &first
            )
            .is_empty()
        );
        assert!(
            ModelAccessStore::accessible_models_from_home(
                temp.path(),
                &ProviderKind::MiniMax,
                &second
            )
            .contains("MiniMax-M2.7")
        );
        let events = fs::read_to_string(
            model_access_dir_for_home(temp.path()).join(MODEL_ACCESS_EVENTS_FILE),
        )
        .expect("events must remain");
        assert_eq!(events.lines().count(), 2);
    }

    #[test]
    fn remove_provider_prunes_all_provider_current_records() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let minimax = credential_fingerprint_for_secret(&ProviderKind::MiniMax, "first", false);
        let openai = credential_fingerprint_for_secret(&ProviderKind::OpenAI, "first", false);
        ModelAccessStore::record_report_at(
            temp.path(),
            &minimax,
            &ProviderProbeReport {
                provider: ProviderKind::MiniMax,
                results: vec![ModelAccessProbeResult::new(
                    &ProviderKind::MiniMax,
                    "MiniMax-M2.7",
                    ModelAccessStatus::Accessible,
                    None,
                    None,
                )],
            },
        )
        .expect("minimax report must write");
        ModelAccessStore::record_report_at(
            temp.path(),
            &openai,
            &ProviderProbeReport {
                provider: ProviderKind::OpenAI,
                results: vec![ModelAccessProbeResult::new(
                    &ProviderKind::OpenAI,
                    "gpt-5.4-mini",
                    ModelAccessStatus::Accessible,
                    None,
                    None,
                )],
            },
        )
        .expect("openai report must write");

        let removed = ModelAccessStore::remove_provider_at(temp.path(), &ProviderKind::MiniMax)
            .expect("remove must write");

        assert_eq!(removed, 1);
        assert!(
            ModelAccessStore::accessible_models_from_home(
                temp.path(),
                &ProviderKind::MiniMax,
                &minimax
            )
            .is_empty()
        );
        assert!(
            ModelAccessStore::accessible_models_from_home(
                temp.path(),
                &ProviderKind::OpenAI,
                &openai
            )
            .contains("gpt-5.4-mini")
        );
    }
}
