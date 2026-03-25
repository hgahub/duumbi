//! Cost control utilities for the dynamic agent system.
//!
//! Provides [`CostTracker`] for token budget enforcement, [`CircuitBreaker`]
//! for agent spawn protection, and [`AgentRateLimiter`] for concurrency control.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use thiserror::Error;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::config::CostSection;

/// Errors originating from cost control enforcement.
#[derive(Debug, Error)]
pub enum CostError {
    /// Token budget was exceeded.
    #[error("Token budget exceeded: used {used} of {limit} tokens")]
    BudgetExceeded {
        /// Tokens consumed so far.
        used: usize,
        /// Configured budget limit.
        limit: usize,
    },

    /// Circuit breaker is open due to too many consecutive failures.
    #[error("Circuit breaker open: {consecutive_failures} consecutive failures")]
    CircuitOpen {
        /// Number of consecutive failures that triggered the open state.
        consecutive_failures: u32,
    },

    /// Could not acquire an agent spawn slot within the timeout.
    #[error("Agent spawn timed out after {timeout_secs}s waiting for available slot")]
    SpawnTimeout {
        /// Seconds waited before timing out.
        timeout_secs: u64,
    },
}

// ---------------------------------------------------------------------------
// CostTracker
// ---------------------------------------------------------------------------

/// Tracks token usage against configured per-intent and per-session budgets.
///
/// Thread-safe: uses `AtomicUsize` counters internally. Multiple agent tasks
/// can call [`record_usage`](CostTracker::record_usage) concurrently.
pub struct CostTracker {
    config: CostSection,
    intent_tokens: AtomicUsize,
    session_tokens: AtomicUsize,
}

impl CostTracker {
    /// Creates a new tracker with zeroed counters and the given budget config.
    #[must_use]
    pub fn new(config: CostSection) -> Self {
        Self {
            config,
            intent_tokens: AtomicUsize::new(0),
            session_tokens: AtomicUsize::new(0),
        }
    }

    /// Checks whether either per-intent or per-session budget has been exceeded.
    ///
    /// Returns `Err(CostError::BudgetExceeded)` at the first exceeded limit.
    #[must_use = "budget check result must be handled"]
    pub fn check_budget(&self) -> Result<(), CostError> {
        let intent = self.intent_tokens.load(Ordering::Relaxed);
        if intent >= self.config.budget_per_intent {
            return Err(CostError::BudgetExceeded {
                used: intent,
                limit: self.config.budget_per_intent,
            });
        }

        let session = self.session_tokens.load(Ordering::Relaxed);
        if session >= self.config.budget_per_session {
            return Err(CostError::BudgetExceeded {
                used: session,
                limit: self.config.budget_per_session,
            });
        }

        Ok(())
    }

    /// Records that `tokens` were consumed, adding to both counters.
    pub fn record_usage(&self, tokens: usize) {
        self.intent_tokens.fetch_add(tokens, Ordering::Relaxed);
        self.session_tokens.fetch_add(tokens, Ordering::Relaxed);
    }

    /// Returns the number of tokens consumed in the current intent execution.
    #[must_use]
    pub fn intent_usage(&self) -> usize {
        self.intent_tokens.load(Ordering::Relaxed)
    }

    /// Returns the total number of tokens consumed in the current CLI session.
    #[must_use]
    pub fn session_usage(&self) -> usize {
        self.session_tokens.load(Ordering::Relaxed)
    }

    /// Resets the per-intent token counter to zero.
    ///
    /// Call this between intent executions so each intent starts with a fresh
    /// budget. The session counter is never reset.
    pub fn reset_intent(&self) {
        self.intent_tokens.store(0, Ordering::Relaxed);
    }

    /// Returns `true` when intent token usage has reached or exceeded the
    /// configured alert threshold percentage.
    #[must_use]
    pub fn is_alert_threshold(&self) -> bool {
        let used = self.intent_tokens.load(Ordering::Relaxed);
        let threshold =
            self.config.budget_per_intent * usize::from(self.config.alert_threshold_pct) / 100;
        used >= threshold
    }
}

// ---------------------------------------------------------------------------
// CircuitBreaker
// ---------------------------------------------------------------------------

/// Current state of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — agent spawning is allowed.
    Closed,
    /// Too many failures — agent spawning is blocked.
    Open,
    /// Testing recovery — the next spawn is allowed as a probe.
    HalfOpen,
}

/// Circuit breaker state machine for agent spawning.
///
/// Transitions:
/// - `Closed` + N consecutive failures → `Open`
/// - `Open` → [`allow_spawn`](CircuitBreaker::allow_spawn) returns `false`
/// - After [`reset`](CircuitBreaker::reset): moves to `HalfOpen`
/// - `HalfOpen` + [`record_success`](CircuitBreaker::record_success) → `Closed`
/// - `HalfOpen` + [`record_failure`](CircuitBreaker::record_failure) → `Open`
pub struct CircuitBreaker {
    state: CircuitState,
    consecutive_failures: u32,
    threshold: u32,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker in the `Closed` state.
    ///
    /// `threshold` is the number of consecutive failures that will open the circuit.
    #[must_use]
    pub fn new(threshold: u32) -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            threshold,
        }
    }

    /// Records a successful agent call.
    ///
    /// - `Closed` or `HalfOpen` → resets the failure counter and moves to `Closed`.
    /// - `Open` → resets the failure counter but leaves the state as `Open`
    ///   (call [`reset`](CircuitBreaker::reset) to re-allow spawning).
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        if self.state != CircuitState::Open {
            self.state = CircuitState::Closed;
        }
    }

    /// Records a failed agent call.
    ///
    /// - Increments the consecutive failure counter.
    /// - When the counter reaches the threshold, moves to `Open`.
    /// - `HalfOpen` failure immediately re-opens the circuit.
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        if self.state == CircuitState::HalfOpen || self.consecutive_failures >= self.threshold {
            self.state = CircuitState::Open;
        }
    }

    /// Returns `true` if a new agent spawn should be allowed.
    ///
    /// - `Closed` or `HalfOpen`: `true`
    /// - `Open`: `false`
    #[must_use]
    pub fn allow_spawn(&self) -> bool {
        self.state != CircuitState::Open
    }

    /// Returns the current circuit state.
    #[must_use]
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Transitions an `Open` circuit to `HalfOpen`, allowing one test spawn.
    ///
    /// If the circuit is already `Closed` or `HalfOpen`, this is a no-op.
    pub fn reset(&mut self) {
        if self.state == CircuitState::Open {
            self.state = CircuitState::HalfOpen;
        }
    }
}

// ---------------------------------------------------------------------------
// AgentRateLimiter
// ---------------------------------------------------------------------------

/// Rate limiter that caps the number of concurrently running LLM agent calls.
///
/// Uses a [`tokio::sync::Semaphore`] internally. Callers acquire a permit
/// before spawning an agent; the permit is released when the agent completes
/// (or is dropped).
pub struct AgentRateLimiter {
    semaphore: Arc<Semaphore>,
    max_permits: usize,
}

impl AgentRateLimiter {
    /// Creates a new rate limiter allowing at most `max_parallel` concurrent agents.
    #[must_use]
    pub fn new(max_parallel: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_parallel)),
            max_permits: max_parallel,
        }
    }

    /// Acquires a spawn permit, waiting up to 60 seconds.
    ///
    /// Returns `Err(CostError::SpawnTimeout)` if no permit becomes available
    /// within the timeout window.
    #[must_use = "the returned permit must be held for the duration of the agent call"]
    pub async fn acquire(&self) -> Result<OwnedSemaphorePermit, CostError> {
        tokio::time::timeout(
            std::time::Duration::from_secs(60),
            Arc::clone(&self.semaphore).acquire_owned(),
        )
        .await
        .map_err(|_| CostError::SpawnTimeout { timeout_secs: 60 })?
        .map_err(|_| {
            // Semaphore::acquire_owned only errors when the semaphore is closed,
            // which never happens in our usage (we own the Arc).
            CostError::SpawnTimeout { timeout_secs: 60 }
        })
    }

    /// Returns the number of permits currently available (not held by any agent).
    #[must_use]
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Returns the maximum number of concurrent agents configured.
    #[must_use]
    pub fn max_permits(&self) -> usize {
        self.max_permits
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CostSection;

    fn default_cost_section() -> CostSection {
        CostSection {
            budget_per_intent: 50_000,
            budget_per_session: 200_000,
            max_parallel_agents: 3,
            circuit_breaker_failures: 5,
            alert_threshold_pct: 80,
        }
    }

    // -----------------------------------------------------------------------
    // CostSection serde tests
    // -----------------------------------------------------------------------

    #[test]
    fn cost_section_default_values() {
        let section = CostSection::default();
        assert_eq!(section.budget_per_intent, 50_000);
        assert_eq!(section.budget_per_session, 200_000);
        assert_eq!(section.max_parallel_agents, 3);
        assert_eq!(section.circuit_breaker_failures, 5);
        assert_eq!(section.alert_threshold_pct, 80);
    }

    #[test]
    fn cost_section_custom_values_parse() {
        let toml = r#"
budget-per-intent = 10000
budget-per-session = 40000
max-parallel-agents = 2
circuit-breaker-failures = 3
alert-threshold-pct = 70
"#;
        let section: CostSection = toml::from_str(toml).expect("custom cost section must parse");
        assert_eq!(section.budget_per_intent, 10_000);
        assert_eq!(section.budget_per_session, 40_000);
        assert_eq!(section.max_parallel_agents, 2);
        assert_eq!(section.circuit_breaker_failures, 3);
        assert_eq!(section.alert_threshold_pct, 70);
    }

    #[test]
    fn cost_section_backward_compat_empty() {
        // An empty [cost] table must parse without errors using all defaults.
        let toml = "";
        let section: CostSection =
            toml::from_str(toml).expect("empty cost section must parse with defaults");
        assert_eq!(section.budget_per_intent, 50_000);
    }

    #[test]
    fn duumbi_config_without_cost_section_parses() {
        use crate::config::DuumbiConfig;
        let toml = r#"
[llm]
provider = "anthropic"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"
"#;
        let cfg: DuumbiConfig = toml::from_str(toml).expect("config without [cost] must parse");
        assert!(cfg.cost.is_none());
    }

    #[test]
    fn duumbi_config_with_cost_section_parses() {
        use crate::config::DuumbiConfig;
        let toml = r#"
[cost]
budget-per-intent = 25000
max-parallel-agents = 5
"#;
        let cfg: DuumbiConfig = toml::from_str(toml).expect("config with [cost] must parse");
        let cost = cfg.cost.expect("cost section must be present");
        assert_eq!(cost.budget_per_intent, 25_000);
        assert_eq!(cost.max_parallel_agents, 5);
        // Unset fields must use their defaults
        assert_eq!(cost.budget_per_session, 200_000);
        assert_eq!(cost.alert_threshold_pct, 80);
    }

    // -----------------------------------------------------------------------
    // CostTracker tests
    // -----------------------------------------------------------------------

    #[test]
    fn cost_tracker_check_budget_passes_when_under_limit() {
        let tracker = CostTracker::new(default_cost_section());
        tracker.record_usage(10_000);
        assert!(tracker.check_budget().is_ok());
    }

    #[test]
    fn cost_tracker_check_budget_fails_when_intent_exceeded() {
        let tracker = CostTracker::new(default_cost_section());
        tracker.record_usage(50_000);
        let err = tracker
            .check_budget()
            .expect_err("must error when budget exceeded");
        assert!(matches!(
            err,
            CostError::BudgetExceeded {
                used: 50_000,
                limit: 50_000
            }
        ));
    }

    #[test]
    fn cost_tracker_check_budget_fails_when_session_exceeded() {
        let mut cfg = default_cost_section();
        cfg.budget_per_session = 5_000;
        cfg.budget_per_intent = 100_000; // high intent budget so session triggers first
        let tracker = CostTracker::new(cfg);
        tracker.record_usage(5_000);
        let err = tracker
            .check_budget()
            .expect_err("must error when session budget exceeded");
        assert!(matches!(
            err,
            CostError::BudgetExceeded {
                used: 5_000,
                limit: 5_000
            }
        ));
    }

    #[test]
    fn cost_tracker_alert_threshold_not_reached_initially() {
        let tracker = CostTracker::new(default_cost_section());
        assert!(!tracker.is_alert_threshold());
    }

    #[test]
    fn cost_tracker_alert_threshold_triggered_at_80_pct() {
        let tracker = CostTracker::new(default_cost_section());
        // 80% of 50_000 = 40_000
        tracker.record_usage(40_000);
        assert!(tracker.is_alert_threshold());
    }

    #[test]
    fn cost_tracker_alert_threshold_not_triggered_below_80_pct() {
        let tracker = CostTracker::new(default_cost_section());
        // 79% of 50_000 = 39_500
        tracker.record_usage(39_500);
        assert!(!tracker.is_alert_threshold());
    }

    #[test]
    fn cost_tracker_reset_intent_clears_intent_counter() {
        let tracker = CostTracker::new(default_cost_section());
        tracker.record_usage(30_000);
        assert_eq!(tracker.intent_usage(), 30_000);
        tracker.reset_intent();
        assert_eq!(tracker.intent_usage(), 0);
        // Session counter must remain unchanged
        assert_eq!(tracker.session_usage(), 30_000);
    }

    #[test]
    fn cost_tracker_reset_intent_allows_budget_check_to_pass_again() {
        let tracker = CostTracker::new(default_cost_section());
        tracker.record_usage(50_000);
        assert!(tracker.check_budget().is_err());
        tracker.reset_intent();
        assert!(tracker.check_budget().is_ok());
    }

    // -----------------------------------------------------------------------
    // CircuitBreaker tests
    // -----------------------------------------------------------------------

    #[test]
    fn circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new(3);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_spawn());
    }

    #[test]
    fn circuit_breaker_closed_to_open_transition() {
        let mut cb = CircuitBreaker::new(3);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn circuit_breaker_open_blocks_spawn() {
        let mut cb = CircuitBreaker::new(2);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_spawn());
    }

    #[test]
    fn circuit_breaker_success_resets_failure_count() {
        let mut cb = CircuitBreaker::new(3);
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        // Failure count should be reset; one more failure should not open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn circuit_breaker_reset_moves_open_to_half_open() {
        let mut cb = CircuitBreaker::new(2);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.allow_spawn());
    }

    #[test]
    fn circuit_breaker_half_open_success_closes_circuit() {
        let mut cb = CircuitBreaker::new(2);
        cb.record_failure();
        cb.record_failure();
        cb.reset();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_spawn());
    }

    #[test]
    fn circuit_breaker_half_open_failure_reopens_circuit() {
        let mut cb = CircuitBreaker::new(2);
        cb.record_failure();
        cb.record_failure();
        cb.reset();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_spawn());
    }

    #[test]
    fn circuit_breaker_reset_noop_when_already_closed() {
        let mut cb = CircuitBreaker::new(3);
        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    // -----------------------------------------------------------------------
    // AgentRateLimiter tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn rate_limiter_acquire_succeeds_when_permits_available() {
        let limiter = AgentRateLimiter::new(2);
        let permit = limiter.acquire().await;
        assert!(permit.is_ok());
        assert_eq!(limiter.available_permits(), 1);
    }

    #[tokio::test]
    async fn rate_limiter_permits_exhausted_causes_timeout() {
        let limiter = AgentRateLimiter::new(1);
        // Hold the only permit
        let _permit = limiter.acquire().await.expect("first acquire must succeed");
        assert_eq!(limiter.available_permits(), 0);

        // Use a custom semaphore with zero permits to avoid the 60s wait
        let zero_limiter = AgentRateLimiter {
            semaphore: Arc::new(Semaphore::new(0)),
            max_permits: 1,
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            zero_limiter.acquire(),
        )
        .await;
        // Either the outer timeout fires or our internal 60s timeout — both are Err
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[tokio::test]
    async fn rate_limiter_permit_released_on_drop() {
        let limiter = AgentRateLimiter::new(1);
        {
            let _permit = limiter.acquire().await.expect("acquire must succeed");
            assert_eq!(limiter.available_permits(), 0);
        } // permit dropped here
        assert_eq!(limiter.available_permits(), 1);
    }

    #[tokio::test]
    async fn rate_limiter_max_permits_reported_correctly() {
        let limiter = AgentRateLimiter::new(5);
        assert_eq!(limiter.max_permits(), 5);
    }
}
