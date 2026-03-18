//! Benchmark report generation.
//!
//! Aggregates [`BenchmarkResult`] entries into a [`BenchmarkReport`] with
//! per-showcase and per-provider statistics, kill criterion evaluation,
//! and optional regression detection against a baseline.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core result types
// ---------------------------------------------------------------------------

/// Outcome of a single benchmark run (one showcase × one provider × one attempt).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Showcase name (e.g. `"calculator"`).
    pub showcase: String,
    /// Provider name (e.g. `"anthropic"`).
    pub provider: String,
    /// Attempt number (1-based).
    pub attempt: u32,
    /// Whether all tests passed.
    pub success: bool,
    /// Categorized failure reason (if `!success`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_category: Option<ErrorCategory>,
    /// Raw error message (if `!success`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Number of test cases that passed.
    pub tests_passed: usize,
    /// Total number of test cases.
    pub tests_total: usize,
    /// Wall-clock duration in seconds.
    pub duration_secs: f64,
}

/// Broad failure category for error breakdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// JSON Schema or YAML validation failure.
    SchemaError,
    /// Type mismatch (E001) or type-related validation.
    TypeError,
    /// Program compiles but produces wrong output.
    LogicError,
    /// Compilation or link failure / runtime crash.
    Crash,
    /// LLM provider returned an error (rate limit, auth, network).
    ProviderError,
    /// Mutation pipeline failed (patch rejected, retry exhausted).
    MutationFailed,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaError => write!(f, "schema_error"),
            Self::TypeError => write!(f, "type_error"),
            Self::LogicError => write!(f, "logic_error"),
            Self::Crash => write!(f, "crash"),
            Self::ProviderError => write!(f, "provider_error"),
            Self::MutationFailed => write!(f, "mutation_failed"),
        }
    }
}

/// Categorizes an error message into an [`ErrorCategory`].
#[must_use]
pub fn categorize_error(msg: &str) -> ErrorCategory {
    let lower = msg.to_lowercase();
    if lower.contains("schema") || lower.contains("e009") {
        ErrorCategory::SchemaError
    } else if lower.contains("type mismatch")
        || lower.contains("e001")
        || lower.contains("e002")
        || lower.contains("e003")
    {
        ErrorCategory::TypeError
    } else if lower.contains("rate limit")
        || lower.contains("401")
        || lower.contains("403")
        || lower.contains("api key")
        || lower.contains("provider")
        || lower.contains("timeout")
    {
        ErrorCategory::ProviderError
    } else if lower.contains("mutation")
        || lower.contains("patch")
        || lower.contains("retry")
        || lower.contains("tool_use")
        || lower.contains("function_call")
    {
        ErrorCategory::MutationFailed
    } else if lower.contains("link failed")
        || lower.contains("compile")
        || lower.contains("cranelift")
        || lower.contains("segfault")
        || lower.contains("signal")
    {
        ErrorCategory::Crash
    } else {
        // Default: if the run produced wrong output it's a logic error.
        ErrorCategory::LogicError
    }
}

// ---------------------------------------------------------------------------
// Aggregated report
// ---------------------------------------------------------------------------

/// Full benchmark report with aggregated statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// ISO-8601 start timestamp.
    pub started_at: String,
    /// ISO-8601 end timestamp.
    pub finished_at: String,
    /// Duumbi version string.
    pub duumbi_version: String,
    /// Number of attempts per (showcase, provider) pair.
    pub attempts_per_run: u32,
    /// Per-showcase aggregated stats.
    pub showcases: Vec<ShowcaseSummary>,
    /// Raw result entries.
    pub results: Vec<BenchmarkResult>,
    /// Whether the kill criterion is met.
    pub kill_criterion_met: bool,
}

/// Per-showcase summary across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowcaseSummary {
    /// Showcase name.
    pub name: String,
    /// Total attempts across all providers.
    pub total_attempts: u32,
    /// Total successful attempts.
    pub successes: u32,
    /// Success rate [0.0, 1.0].
    pub success_rate: f64,
    /// Per-provider breakdown.
    pub providers: Vec<ProviderStats>,
}

/// Per-provider stats within a showcase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    /// Provider name.
    pub name: String,
    /// Total attempts.
    pub attempts: u32,
    /// Successful attempts.
    pub successes: u32,
    /// Success rate [0.0, 1.0].
    pub success_rate: f64,
    /// Error category breakdown (category → count).
    pub error_categories: BTreeMap<String, u32>,
}

/// A detected regression compared to a baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    /// Showcase name.
    pub showcase: String,
    /// Provider name.
    pub provider: String,
    /// Previous success rate.
    pub baseline_rate: f64,
    /// Current success rate.
    pub current_rate: f64,
    /// Absolute drop.
    pub drop: f64,
}

impl BenchmarkReport {
    /// Aggregates raw results into a full report.
    #[must_use]
    pub fn from_results(
        results: Vec<BenchmarkResult>,
        attempts_per_run: u32,
        started_at: String,
        finished_at: String,
    ) -> Self {
        let showcases = aggregate_showcases(&results);
        let kill_criterion_met = check_kill_criterion(&showcases);

        Self {
            started_at,
            finished_at,
            duumbi_version: env!("CARGO_PKG_VERSION").to_string(),
            attempts_per_run,
            showcases,
            results,
            kill_criterion_met,
        }
    }

    /// Serializes the report to pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (should not happen with valid data).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Writes the report JSON to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    pub fn write_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = self.to_json().map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Prints a human-readable summary table to stderr.
    pub fn print_summary(&self) {
        eprintln!();
        eprintln!("╔══════════════════╦══════════════════╦══════════╦═══════════╗");
        eprintln!("║ Showcase         ║ Provider         ║ Success  ║ Rate      ║");
        eprintln!("╠══════════════════╬══════════════════╬══════════╬═══════════╣");

        for showcase in &self.showcases {
            for prov in &showcase.providers {
                eprintln!(
                    "║ {:<16} ║ {:<16} ║ {:>3}/{:<4} ║ {:>6.1}%   ║",
                    truncate(&showcase.name, 16),
                    truncate(&prov.name, 16),
                    prov.successes,
                    prov.attempts,
                    prov.success_rate * 100.0,
                );
            }
        }

        eprintln!("╚══════════════════╩══════════════════╩══════════╩═══════════╝");
        eprintln!();

        if self.kill_criterion_met {
            eprintln!("Kill criterion: PASSED (5/6 showcases × 2+ providers ≥ 95%)");
        } else {
            eprintln!("Kill criterion: NOT MET (need 5/6 showcases × 2+ providers ≥ 95%)");
        }
        eprintln!();
    }

    /// Prints error category breakdown to stderr.
    pub fn print_error_breakdown(&self) {
        let mut totals: BTreeMap<String, u32> = BTreeMap::new();
        for result in &self.results {
            if let Some(ref cat) = result.error_category {
                *totals.entry(cat.to_string()).or_insert(0) += 1;
            }
        }

        if totals.is_empty() {
            return;
        }

        eprintln!("Error breakdown:");
        for (cat, count) in &totals {
            eprintln!("  {cat}: {count}");
        }
        eprintln!();
    }
}

// ---------------------------------------------------------------------------
// Kill criterion
// ---------------------------------------------------------------------------

/// Checks whether the kill criterion is met.
///
/// Criterion: at least 5 out of 6 showcases must have ≥ 95% success rate
/// across at least 2 different providers.
#[must_use]
pub fn check_kill_criterion(showcases: &[ShowcaseSummary]) -> bool {
    let passing = showcases
        .iter()
        .filter(|s| {
            let qualifying_providers = s
                .providers
                .iter()
                .filter(|p| p.success_rate >= 0.95)
                .count();
            qualifying_providers >= 2
        })
        .count();

    passing >= 5
}

// ---------------------------------------------------------------------------
// Regression detection
// ---------------------------------------------------------------------------

/// Detects regressions by comparing current results against a baseline report.
///
/// A regression is flagged when the success rate for a (showcase, provider) pair
/// drops by more than `threshold` (e.g. 0.05 = 5 percentage points).
#[must_use]
pub fn detect_regressions(
    current: &BenchmarkReport,
    baseline: &BenchmarkReport,
    threshold: f64,
) -> Vec<Regression> {
    let mut baseline_map: BTreeMap<(String, String), f64> = BTreeMap::new();
    for showcase in &baseline.showcases {
        for prov in &showcase.providers {
            baseline_map.insert(
                (showcase.name.clone(), prov.name.clone()),
                prov.success_rate,
            );
        }
    }

    let mut regressions = Vec::new();
    for showcase in &current.showcases {
        for prov in &showcase.providers {
            let key = (showcase.name.clone(), prov.name.clone());
            if let Some(&base_rate) = baseline_map.get(&key) {
                let drop = base_rate - prov.success_rate;
                if drop > threshold {
                    regressions.push(Regression {
                        showcase: showcase.name.clone(),
                        provider: prov.name.clone(),
                        baseline_rate: base_rate,
                        current_rate: prov.success_rate,
                        drop,
                    });
                }
            }
        }
    }

    regressions
}

/// Prints regression warnings to stderr.
pub fn print_regressions(regressions: &[Regression]) {
    if regressions.is_empty() {
        return;
    }

    eprintln!("⚠ Regressions detected:");
    for r in regressions {
        eprintln!(
            "  {} / {}: {:.1}% → {:.1}% (dropped {:.1}%)",
            r.showcase,
            r.provider,
            r.baseline_rate * 100.0,
            r.current_rate * 100.0,
            r.drop * 100.0,
        );
    }
    eprintln!();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Aggregates raw results into per-showcase summaries.
fn aggregate_showcases(results: &[BenchmarkResult]) -> Vec<ShowcaseSummary> {
    // Group by showcase
    let mut by_showcase: BTreeMap<String, Vec<&BenchmarkResult>> = BTreeMap::new();
    for r in results {
        by_showcase.entry(r.showcase.clone()).or_default().push(r);
    }

    by_showcase
        .into_iter()
        .map(|(name, entries)| {
            let total_attempts = entries.len() as u32;
            let successes = entries.iter().filter(|r| r.success).count() as u32;
            let success_rate = if total_attempts > 0 {
                f64::from(successes) / f64::from(total_attempts)
            } else {
                0.0
            };

            // Group by provider
            let mut by_provider: BTreeMap<String, Vec<&&BenchmarkResult>> = BTreeMap::new();
            for r in &entries {
                by_provider.entry(r.provider.clone()).or_default().push(r);
            }

            let providers = by_provider
                .into_iter()
                .map(|(pname, pentries)| {
                    let attempts = pentries.len() as u32;
                    let psuccesses = pentries.iter().filter(|r| r.success).count() as u32;
                    let prate = if attempts > 0 {
                        f64::from(psuccesses) / f64::from(attempts)
                    } else {
                        0.0
                    };

                    let mut error_categories: BTreeMap<String, u32> = BTreeMap::new();
                    for r in &pentries {
                        if let Some(ref cat) = r.error_category {
                            *error_categories.entry(cat.to_string()).or_insert(0) += 1;
                        }
                    }

                    ProviderStats {
                        name: pname,
                        attempts,
                        successes: psuccesses,
                        success_rate: prate,
                        error_categories,
                    }
                })
                .collect();

            ShowcaseSummary {
                name,
                total_attempts,
                successes,
                success_rate,
                providers,
            }
        })
        .collect()
}

/// Truncates a string to `max_len`, appending `…` if needed.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

/// Loads a baseline report from a JSON file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_baseline(path: &Path) -> Result<BenchmarkReport, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read baseline '{}': {e}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|e| format!("failed to parse baseline '{}': {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(showcase: &str, provider: &str, success: bool) -> BenchmarkResult {
        BenchmarkResult {
            showcase: showcase.to_string(),
            provider: provider.to_string(),
            attempt: 1,
            success,
            error_category: if success {
                None
            } else {
                Some(ErrorCategory::LogicError)
            },
            error_message: if success {
                None
            } else {
                Some("wrong output".to_string())
            },
            tests_passed: if success { 4 } else { 2 },
            tests_total: 4,
            duration_secs: 5.0,
        }
    }

    #[test]
    fn kill_criterion_met_with_5_passing() {
        let showcases: Vec<String> = vec![
            "calculator",
            "fibonacci",
            "sorting",
            "state_machine",
            "multi_module",
            "string_ops",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let mut results = Vec::new();
        // 5 showcases with 100% success on 2 providers
        for sc in &showcases[..5] {
            for prov in &["anthropic", "openai"] {
                for attempt in 1..=20 {
                    let mut r = make_result(sc, prov, true);
                    r.attempt = attempt;
                    results.push(r);
                }
            }
        }
        // 6th showcase fails
        for prov in &["anthropic", "openai"] {
            for attempt in 1..=20 {
                let mut r = make_result(&showcases[5], prov, false);
                r.attempt = attempt;
                results.push(r);
            }
        }

        let report = BenchmarkReport::from_results(
            results,
            20,
            "2026-03-18T00:00:00Z".to_string(),
            "2026-03-18T01:00:00Z".to_string(),
        );

        assert!(report.kill_criterion_met);
    }

    #[test]
    fn kill_criterion_not_met_with_only_4_passing() {
        let showcases = ["calculator", "fibonacci", "sorting", "state_machine"];

        let mut results = Vec::new();
        for sc in &showcases {
            for prov in &["anthropic", "openai"] {
                let mut r = make_result(sc, prov, true);
                r.attempt = 1;
                results.push(r);
            }
        }
        // Two showcases fail
        for sc in &["multi_module", "string_ops"] {
            for prov in &["anthropic", "openai"] {
                let mut r = make_result(sc, prov, false);
                r.attempt = 1;
                results.push(r);
            }
        }

        let report = BenchmarkReport::from_results(
            results,
            1,
            "2026-03-18T00:00:00Z".to_string(),
            "2026-03-18T01:00:00Z".to_string(),
        );

        assert!(!report.kill_criterion_met);
    }

    #[test]
    fn categorize_error_patterns() {
        assert_eq!(
            categorize_error("E009 schema invalid"),
            ErrorCategory::SchemaError
        );
        assert_eq!(
            categorize_error("E001 Type mismatch: expected i64"),
            ErrorCategory::TypeError
        );
        assert_eq!(
            categorize_error("rate limit exceeded"),
            ErrorCategory::ProviderError
        );
        assert_eq!(
            categorize_error("mutation failed after 3 retries"),
            ErrorCategory::MutationFailed
        );
        assert_eq!(
            categorize_error("link failed: undefined symbol"),
            ErrorCategory::Crash
        );
        assert_eq!(
            categorize_error("expected 8 but got 7"),
            ErrorCategory::LogicError
        );
    }

    #[test]
    fn json_roundtrip() {
        let results = vec![make_result("calculator", "anthropic", true)];
        let report = BenchmarkReport::from_results(
            results,
            1,
            "2026-03-18T00:00:00Z".to_string(),
            "2026-03-18T00:01:00Z".to_string(),
        );

        let json = report.to_json().expect("serialization failed");
        let parsed: BenchmarkReport = serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(parsed.showcases.len(), 1);
        assert_eq!(parsed.results.len(), 1);
        assert!(parsed.results[0].success);
    }

    #[test]
    fn regression_detected() {
        let baseline_results = vec![
            make_result("calculator", "anthropic", true),
            make_result("calculator", "anthropic", true),
        ];
        let baseline = BenchmarkReport::from_results(
            baseline_results,
            2,
            "2026-03-17T00:00:00Z".to_string(),
            "2026-03-17T01:00:00Z".to_string(),
        );

        let current_results = vec![
            make_result("calculator", "anthropic", true),
            make_result("calculator", "anthropic", false),
        ];
        let current = BenchmarkReport::from_results(
            current_results,
            2,
            "2026-03-18T00:00:00Z".to_string(),
            "2026-03-18T01:00:00Z".to_string(),
        );

        let regressions = detect_regressions(&current, &baseline, 0.05);
        assert_eq!(regressions.len(), 1);
        assert_eq!(regressions[0].showcase, "calculator");
        assert!((regressions[0].baseline_rate - 1.0).abs() < f64::EPSILON);
        assert!((regressions[0].current_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn no_regression_within_threshold() {
        let baseline_results = vec![
            make_result("calculator", "anthropic", true),
            make_result("calculator", "anthropic", true),
            make_result("calculator", "anthropic", true),
            make_result("calculator", "anthropic", false),
        ];
        let baseline = BenchmarkReport::from_results(
            baseline_results,
            4,
            "2026-03-17T00:00:00Z".to_string(),
            "2026-03-17T01:00:00Z".to_string(),
        );

        // Same rate (75%)
        let current_results = vec![
            make_result("calculator", "anthropic", true),
            make_result("calculator", "anthropic", true),
            make_result("calculator", "anthropic", true),
            make_result("calculator", "anthropic", false),
        ];
        let current = BenchmarkReport::from_results(
            current_results,
            4,
            "2026-03-18T00:00:00Z".to_string(),
            "2026-03-18T01:00:00Z".to_string(),
        );

        let regressions = detect_regressions(&current, &baseline, 0.05);
        assert!(regressions.is_empty());
    }
}
