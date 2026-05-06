//! File logging and command performance event support.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::{DuumbiConfig, LogLevel, LogMode, PerformanceLoggingSection};

const DEFAULT_GENERAL_LOG: &str = ".duumbi/logs/duumbi.log";
const DEFAULT_PERFORMANCE_LOG: &str = ".duumbi/logs/performance.jsonl";

/// CLI-provided logging overrides.
#[derive(Debug, Clone, Default)]
pub struct LoggingOverrides {
    /// General log level override.
    pub general_level: Option<LogLevel>,
    /// General log path override.
    pub general_path: Option<PathBuf>,
    /// General log write mode override.
    pub general_mode: Option<LogMode>,
    /// Performance log enabled override.
    pub performance_enabled: Option<bool>,
    /// Performance log path override.
    pub performance_path: Option<PathBuf>,
    /// Performance log write mode override.
    pub performance_mode: Option<LogMode>,
}

/// Fully resolved logging settings for a process invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLogging {
    /// General tracing log level, if enabled and writable.
    pub general_level: Option<LogLevel>,
    /// General tracing log path, if enabled and resolved.
    pub general_path: Option<PathBuf>,
    /// General tracing log write mode.
    pub general_mode: LogMode,
    /// Performance JSONL log path, if enabled and resolved.
    pub performance_path: Option<PathBuf>,
    /// Performance JSONL log write mode.
    pub performance_mode: LogMode,
}

/// Runtime logging handles.
#[derive(Debug, Clone)]
pub struct RuntimeLogging {
    performance: Option<PerformanceLogger>,
}

impl RuntimeLogging {
    /// Returns the active performance logger, if command timing is enabled.
    #[must_use]
    pub fn performance(&self) -> Option<&PerformanceLogger> {
        self.performance.as_ref()
    }
}

/// Append-only command performance event logger.
#[derive(Debug, Clone)]
pub struct PerformanceLogger {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl PerformanceLogger {
    /// Creates a performance logger at `path`.
    pub fn new(path: PathBuf, mode: LogMode) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if mode == LogMode::Rewrite {
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;
        }
        Ok(Self {
            path,
            lock: Arc::new(Mutex::new(())),
        })
    }

    /// Records a command start event and returns the start instant.
    #[must_use]
    pub fn record_start(&self, command: &str) -> Instant {
        let started = Instant::now();
        let _ = self.write_event(PerformanceEvent {
            timestamp: Utc::now(),
            command,
            phase: "command_start",
            elapsed_ms: 0,
            status: "started",
            error: None,
        });
        started
    }

    /// Records a successful command end event.
    pub fn record_success(&self, command: &str, started: Instant) {
        let _ = self.write_end(command, started.elapsed(), "success", None);
    }

    /// Records a failed command end event.
    pub fn record_error(&self, command: &str, started: Instant, error: &str) {
        let _ = self.write_end(command, started.elapsed(), "error", Some(error));
    }

    fn write_end(
        &self,
        command: &str,
        elapsed: Duration,
        status: &'static str,
        error: Option<&str>,
    ) -> io::Result<()> {
        self.write_event(PerformanceEvent {
            timestamp: Utc::now(),
            command,
            phase: "command_end",
            elapsed_ms: elapsed.as_millis() as u64,
            status,
            error: error.map(ToOwned::to_owned),
        })
    }

    fn write_event(&self, event: PerformanceEvent<'_>) -> io::Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("performance log lock poisoned"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(&event).map_err(io::Error::other)?;
        writeln!(file, "{line}")
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PerformanceEvent<'a> {
    timestamp: DateTime<Utc>,
    command: &'a str,
    phase: &'a str,
    elapsed_ms: u64,
    status: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Clone)]
struct SharedMakeWriter {
    file: Arc<Mutex<File>>,
}

struct SharedFileWriter {
    file: Arc<Mutex<File>>,
}

impl Write for SharedFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("log file lock poisoned"))?;
        file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("log file lock poisoned"))?;
        file.flush()
    }
}

impl<'a> MakeWriter<'a> for SharedMakeWriter {
    type Writer = SharedFileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedFileWriter {
            file: Arc::clone(&self.file),
        }
    }
}

/// Resolves logging settings from config plus CLI overrides.
#[must_use]
pub fn resolve_logging(
    workspace_root: &Path,
    config: &DuumbiConfig,
    overrides: &LoggingOverrides,
) -> ResolvedLogging {
    let logging = config.logging.clone().unwrap_or_default();
    let general = logging.general;
    let performance = logging.performance;

    let mut general_level = if general.enabled {
        Some(general.level)
    } else {
        None
    };
    if let Some(level) = overrides.general_level {
        general_level = (level != LogLevel::Off).then_some(level);
    }
    let general_path = overrides
        .general_path
        .clone()
        .or(general.path)
        .or_else(|| default_log_path(workspace_root, DEFAULT_GENERAL_LOG));
    let general_mode = overrides.general_mode.unwrap_or(general.mode);
    if general_level == Some(LogLevel::Off) {
        general_level = None;
    }

    let performance_enabled = resolve_performance_enabled(&performance, overrides);
    let performance_path = performance_enabled
        .then(|| {
            overrides
                .performance_path
                .clone()
                .or(performance.path)
                .or_else(|| default_log_path(workspace_root, DEFAULT_PERFORMANCE_LOG))
        })
        .flatten();
    let performance_mode = overrides.performance_mode.unwrap_or(performance.mode);

    ResolvedLogging {
        general_level: general_path.as_ref().and(general_level),
        general_path,
        general_mode,
        performance_path,
        performance_mode,
    }
}

fn resolve_performance_enabled(
    performance: &PerformanceLoggingSection,
    overrides: &LoggingOverrides,
) -> bool {
    overrides
        .performance_enabled
        .unwrap_or(performance.enabled || overrides.performance_path.is_some())
}

fn default_log_path(workspace_root: &Path, relative: &str) -> Option<PathBuf> {
    workspace_root
        .join(".duumbi")
        .exists()
        .then(|| workspace_root.join(relative))
}

/// Initializes general tracing and performance logging.
pub fn initialize(
    workspace_root: &Path,
    config: &DuumbiConfig,
    overrides: &LoggingOverrides,
) -> io::Result<RuntimeLogging> {
    let resolved = resolve_logging(workspace_root, config, overrides);
    if let (Some(level), Some(path)) = (resolved.general_level, resolved.general_path.as_ref()) {
        init_general_logging(path, resolved.general_mode, level)?;
    }
    let performance = resolved
        .performance_path
        .map(|path| PerformanceLogger::new(path, resolved.performance_mode))
        .transpose()?;
    Ok(RuntimeLogging { performance })
}

fn init_general_logging(path: &Path, mode: LogMode, level: LogLevel) -> io::Result<()> {
    let file = open_log_file(path, mode)?;
    let writer = SharedMakeWriter {
        file: Arc::new(Mutex::new(file)),
    };
    let filter = EnvFilter::new(level.to_string());
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(writer)
        .finish();
    let _ = subscriber.try_init();
    Ok(())
}

fn open_log_file(path: &Path, mode: LogMode) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.create(true).write(true);
    match mode {
        LogMode::Append => {
            options.append(true);
        }
        LogMode::Rewrite => {
            options.truncate(true);
        }
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GeneralLoggingSection, LoggingSection};
    use tempfile::TempDir;

    #[test]
    fn default_paths_require_workspace() {
        let tmp = TempDir::new().expect("invariant: temp dir");
        let resolved = resolve_logging(
            tmp.path(),
            &DuumbiConfig::default(),
            &LoggingOverrides::default(),
        );

        assert_eq!(resolved.general_level, None);
        assert_eq!(resolved.general_path, None);
        assert_eq!(resolved.performance_path, None);
    }

    #[test]
    fn default_general_log_uses_workspace_logs_dir() {
        let tmp = TempDir::new().expect("invariant: temp dir");
        fs::create_dir_all(tmp.path().join(".duumbi")).expect("mkdir");

        let resolved = resolve_logging(
            tmp.path(),
            &DuumbiConfig::default(),
            &LoggingOverrides::default(),
        );

        assert_eq!(resolved.general_level, Some(LogLevel::Error));
        assert_eq!(
            resolved.general_path,
            Some(tmp.path().join(DEFAULT_GENERAL_LOG))
        );
    }

    #[test]
    fn cli_level_off_disables_general_logging() {
        let tmp = TempDir::new().expect("invariant: temp dir");
        fs::create_dir_all(tmp.path().join(".duumbi")).expect("mkdir");
        let overrides = LoggingOverrides {
            general_level: Some(LogLevel::Off),
            ..LoggingOverrides::default()
        };

        let resolved = resolve_logging(tmp.path(), &DuumbiConfig::default(), &overrides);

        assert_eq!(resolved.general_level, None);
    }

    #[test]
    fn explicit_paths_do_not_require_workspace() {
        let tmp = TempDir::new().expect("invariant: temp dir");
        let path = tmp.path().join("custom.log");
        let overrides = LoggingOverrides {
            general_path: Some(path.clone()),
            ..LoggingOverrides::default()
        };

        let resolved = resolve_logging(tmp.path(), &DuumbiConfig::default(), &overrides);

        assert_eq!(resolved.general_level, Some(LogLevel::Error));
        assert_eq!(resolved.general_path, Some(path));
    }

    #[test]
    fn user_config_can_disable_general_logging() {
        let tmp = TempDir::new().expect("invariant: temp dir");
        fs::create_dir_all(tmp.path().join(".duumbi")).expect("mkdir");
        let config = DuumbiConfig {
            logging: Some(LoggingSection {
                general: GeneralLoggingSection {
                    enabled: false,
                    ..GeneralLoggingSection::default()
                },
                ..LoggingSection::default()
            }),
            ..DuumbiConfig::default()
        };

        let resolved = resolve_logging(tmp.path(), &config, &LoggingOverrides::default());

        assert_eq!(resolved.general_level, None);
    }

    #[test]
    fn performance_logger_rewrite_truncates_once_then_appends() {
        let tmp = TempDir::new().expect("invariant: temp dir");
        let path = tmp.path().join("performance.jsonl");
        fs::write(&path, "old\n").expect("write old log");
        let logger = PerformanceLogger::new(path.clone(), LogMode::Rewrite).expect("logger");

        let started = logger.record_start("build");
        logger.record_success("build", started);

        let log = fs::read_to_string(path).expect("read performance log");
        assert!(!log.contains("old"));
        assert_eq!(log.lines().count(), 2);
        assert!(log.contains("\"phase\":\"command_start\""));
        assert!(log.contains("\"status\":\"success\""));
    }
}
