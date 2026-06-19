//! Append-only replay ledger writer.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::evidence::LedgerEvent;

/// Errors produced while writing replay ledger events.
#[derive(Debug, Error)]
pub enum LedgerError {
    /// Ledger file open failed.
    #[error("failed to open replay ledger at {path}: {source}")]
    Open {
        /// Ledger path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Ledger event serialization failed.
    #[error("failed to serialize replay ledger event: {0}")]
    Serialize(#[source] serde_json::Error),
    /// Ledger event write failed.
    #[error("failed to write replay ledger event at {path}: {source}")]
    Write {
        /// Ledger path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Ledger event flush failed.
    #[error("failed to flush replay ledger at {path}: {source}")]
    Flush {
        /// Ledger path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Append-only JSONL writer for replay ledger events.
pub struct LedgerWriter {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl LedgerWriter {
    /// Opens a ledger writer in append-only mode.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError`] if the ledger file cannot be opened.
    #[must_use = "ledger writer must be used to record events"]
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .map_err(|source| LedgerError::Open {
                path: path.display().to_string(),
                source,
            })?;
        Ok(Self {
            path: path.to_path_buf(),
            writer: BufWriter::new(file),
        })
    }

    /// Appends one event as a JSON line and flushes it.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError`] if serialization, writing, or flushing fails.
    #[must_use = "ledger append result must be handled"]
    pub fn append(&mut self, event: &LedgerEvent) -> Result<(), LedgerError> {
        serde_json::to_writer(&mut self.writer, event).map_err(LedgerError::Serialize)?;
        self.writer
            .write_all(b"\n")
            .map_err(|source| LedgerError::Write {
                path: self.path.display().to_string(),
                source,
            })?;
        self.writer.flush().map_err(|source| LedgerError::Flush {
            path: self.path.display().to_string(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::determinism::evidence::{LedgerEventKind, REPLAY_LEDGER_SCHEMA_VERSION};

    #[test]
    fn ledger_writer_appends_json_lines() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("ledger.jsonl");
        let mut writer = LedgerWriter::open(&path).expect("ledger opens");

        writer
            .append(&LedgerEvent::new(
                "run-1",
                LedgerEventKind::RunStarted,
                1,
                "2026-06-17T00:00:00Z",
                serde_json::json!({"ok": true}),
            ))
            .expect("event writes");
        writer
            .append(&LedgerEvent::new(
                "run-1",
                LedgerEventKind::RunCompleted,
                2,
                "2026-06-17T00:00:01Z",
                serde_json::json!({"ok": true}),
            ))
            .expect("event writes");

        let contents = fs::read_to_string(path).expect("ledger reads");
        let lines: Vec<_> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(REPLAY_LEDGER_SCHEMA_VERSION));
        assert!(lines[1].contains("\"event\":\"run_completed\""));
    }
}
