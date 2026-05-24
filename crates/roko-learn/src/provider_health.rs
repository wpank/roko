//! Per-provider circuit breaker for LLM routing (§13.9).
//!
//! Tracks consecutive failures per provider and transitions through a
//! three-state machine:
//!
//! ```text
//! Healthy ──[N consecutive failures]──▶ Unhealthy { recovery_at }
//!     ▲                                        │
//!     │                                  [now ≥ recovery_at]
//!     │                                        ▼
//!     └────[record_success]──────────── Probing
//!                                 [record_failure]──▶ Unhealthy (timer reset)
//! ```
//!
//! # Thread safety
//!
//! All state is behind a [`parking_lot::RwLock`], making the tracker safe
//! for concurrent use from multiple tokio tasks.
//!
//! # `Instant` vs `SystemTime`
//!
//! Recovery timestamps use [`std::time::Instant`] so they are immune to
//! wall-clock adjustments. Because `Instant` is not serializable, the
//! tracker is an in-memory runtime component only. Persisted provider
//! snapshots use unix milliseconds and are handled by
//! [`ProviderHealthRegistry`].

use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ─── Serializable health snapshot types ────────────────────────────────────

/// Serialized circuit state for persisted provider-health snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Normal operation.
    Closed,
    /// Requests are blocked while the provider cools down.
    Open,
    /// One probe request is allowed after cooldown expires.
    HalfOpen,
}

/// Classified error category used to pick cooldown durations later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorClass {
    /// Provider returned a rate limit response.
    RateLimit,
    /// Provider returned an authentication or authorization failure.
    AuthFailure,
    /// Request timed out before completing.
    Timeout,
    /// Provider returned a 5xx or other transient server error.
    ServerError,
    /// Request was blocked by content policy.
    ContentPolicy,
    /// Context exceeded the provider's maximum window.
    ContextOverflow,
    /// Fallback classification when the exact class is unknown.
    Unknown,
}

/// Timestamped failure entry for the rolling failure window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureRecord {
    /// Failure timestamp in unix milliseconds.
    pub timestamp_ms: i64,
    /// Classified failure type.
    pub error_class: ErrorClass,
}

/// Serializable per-provider health snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Stable provider identifier.
    pub provider_id: String,
    /// Snapshot circuit state.
    pub state: CircuitState,
    /// Consecutive failures seen most recently.
    pub consecutive_failures: u32,
    /// Lifetime request count.
    pub total_requests: u64,
    /// Lifetime failure count.
    pub total_failures: u64,
    /// Timestamp of the most recent failure, in unix milliseconds.
    pub last_failure_at: Option<i64>,
    /// Timestamp when the provider may be retried, in unix milliseconds.
    pub cooldown_until: Option<i64>,
    /// Rolling window of recent failures.
    pub failure_window: VecDeque<FailureRecord>,
}

impl ProviderHealth {
    /// Record a successful request.
    ///
    /// A success from `HalfOpen` or `Open` closes the circuit. The `Open`
    /// case handles providers whose state was persisted as Open and whose
    /// cooldown expired before the process reloaded — without this, a
    /// success would clear `consecutive_failures` but leave the circuit
    /// permanently locked out.
    pub fn record_success(&mut self) {
        self.total_requests = self.total_requests.saturating_add(1);
        self.consecutive_failures = 0;
        self.cooldown_until = None;
        if self.state == CircuitState::HalfOpen || self.state == CircuitState::Open {
            self.state = CircuitState::Closed;
        }
    }

    /// Record a failed request and update the circuit state.
    pub fn record_failure(&mut self, error: ErrorClass, now_ms: i64) {
        self.total_requests = self.total_requests.saturating_add(1);
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.total_failures = self.total_failures.saturating_add(1);
        self.last_failure_at = Some(now_ms);
        self.failure_window.push_back(FailureRecord {
            timestamp_ms: now_ms,
            error_class: error,
        });
        if self.failure_window.len() > 20 {
            self.failure_window.pop_front();
        }

        // Trip to Open after 3 consecutive failures.
        if self.consecutive_failures >= 3 {
            self.state = CircuitState::Open;
            self.cooldown_until = Some(now_ms + self.cooldown_ms(error));
        }
    }

    /// Return whether the provider can receive a request at `now_ms`.
    ///
    /// When an open circuit's cooldown expires, the state advances to
    /// `HalfOpen` so the next request can act as a probe.
    pub fn is_available(&mut self, now_ms: i64) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(until) = self.cooldown_until {
                    if now_ms >= until {
                        self.state = CircuitState::HalfOpen;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Error-class-specific cooldown in milliseconds.
    fn cooldown_ms(&self, error: ErrorClass) -> i64 {
        match error {
            ErrorClass::RateLimit => 5_000,
            ErrorClass::Timeout => 10_000,
            ErrorClass::ServerError => 30_000,
            ErrorClass::AuthFailure => 300_000,
            _ => 5_000,
        }
    }
}

/// Persisted registry snapshot for loading and saving provider health.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ProviderHealthRegistrySnapshot {
    /// Per-provider health snapshots keyed by provider id.
    providers: HashMap<String, ProviderHealth>,
}

/// Thread-safe registry of provider health snapshots.
///
/// The registry stores [`ProviderHealth`] values keyed by provider id and
/// provides a disk-backed persistence layer for the runtime circuit breaker.
pub struct ProviderHealthRegistry {
    providers: Arc<Mutex<HashMap<String, ProviderHealth>>>,
    save_lock: Arc<Mutex<()>>,
    save_tx: Option<Sender<PersistCommand>>,
    save_worker: Option<JoinHandle<()>>,
}

const HEALTH_SAVE_DEBOUNCE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy)]
enum PersistCommand {
    Dirty,
    FlushAndStop,
}

impl ProviderHealthRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: Arc::new(Mutex::new(HashMap::new())),
            save_lock: Arc::new(Mutex::new(())),
            save_tx: None,
            save_worker: None,
        }
    }

    /// Record a successful request for `provider_id`.
    pub fn record_success(&self, provider_id: &str) {
        let mut providers = self.providers.lock();
        let health = providers
            .entry(provider_id.to_owned())
            .or_insert_with(|| new_provider_health(provider_id));
        health.record_success();
        drop(providers);
        self.schedule_persist();
    }

    /// Record a failed request for `provider_id`.
    pub fn record_failure(&self, provider_id: &str, error: ErrorClass) {
        let mut providers = self.providers.lock();
        let health = providers
            .entry(provider_id.to_owned())
            .or_insert_with(|| new_provider_health(provider_id));
        health.record_failure(error, unix_ms_now());
        drop(providers);
        self.schedule_persist();
    }

    /// Return whether `provider_id` is currently available for routing.
    ///
    /// Unknown providers are treated as available.
    pub fn is_available(&self, provider_id: &str) -> bool {
        let mut providers = self.providers.lock();
        let mut should_persist = false;
        let available = match providers.get_mut(provider_id) {
            Some(health) => {
                let previous_state = health.state;
                let available = health.is_available(unix_ms_now());
                should_persist = previous_state != health.state;
                available
            }
            None => true,
        };
        drop(providers);
        if should_persist {
            self.schedule_persist();
        }
        available
    }

    /// Return whether `provider_id` currently looks healthy without mutating
    /// the circuit state.
    ///
    /// Unknown providers are treated as healthy.
    #[must_use]
    pub fn is_healthy(&self, provider_id: &str) -> bool {
        let providers = self.providers.lock();
        match providers.get(provider_id) {
            None => true,
            Some(health) => match health.state {
                CircuitState::Closed | CircuitState::HalfOpen => true,
                CircuitState::Open => health
                    .cooldown_until
                    .is_some_and(|until| unix_ms_now() >= until),
            },
        }
    }

    /// Filter `candidates` to only providers that are currently available.
    pub fn available_providers(&self, candidates: &[String]) -> Vec<String> {
        candidates
            .iter()
            .filter(|provider_id| self.is_available(provider_id))
            .cloned()
            .collect()
    }

    /// Return a cloned snapshot of all tracked provider health records.
    #[must_use]
    pub fn snapshot(&self) -> HashMap<String, ProviderHealth> {
        self.providers.lock().clone()
    }

    /// Return the current snapshot for `provider_id`, defaulting to a
    /// healthy record when the provider has never been seen.
    #[must_use]
    pub fn get(&self, provider_id: &str) -> ProviderHealth {
        self.providers
            .lock()
            .get(provider_id)
            .cloned()
            .unwrap_or_else(|| new_provider_health(provider_id))
    }

    /// Persist the registry to `path` as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the registry snapshot cannot be serialized or if
    /// any filesystem step needed to write it fails.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let snapshot = ProviderHealthRegistrySnapshot {
            providers: self.providers.lock().clone(),
        };
        let _guard = self.save_lock.lock();
        save_snapshot(path, &snapshot)
    }

    /// Load the registry from `path`, or return a new empty registry.
    pub fn load_or_new(path: &Path) -> Self {
        let snapshot = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str::<ProviderHealthRegistrySnapshot>(&s).ok());

        match snapshot {
            Some(snapshot) => Self::with_persistence(path.to_path_buf(), snapshot.providers),
            None => Self::with_persistence(path.to_path_buf(), HashMap::new()),
        }
    }

    fn with_persistence(path: PathBuf, providers: HashMap<String, ProviderHealth>) -> Self {
        let providers = Arc::new(Mutex::new(providers));
        let save_lock = Arc::new(Mutex::new(()));
        let (save_tx, save_worker) =
            spawn_save_worker(path, Arc::clone(&providers), Arc::clone(&save_lock));
        Self {
            providers,
            save_lock,
            save_tx: Some(save_tx),
            save_worker: Some(save_worker),
        }
    }

    fn schedule_persist(&self) {
        if let Some(tx) = &self.save_tx {
            let _ = tx.send(PersistCommand::Dirty);
        }
    }
}

impl Default for ProviderHealthRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ProviderHealthRegistry {
    fn drop(&mut self) {
        if let Some(tx) = self.save_tx.take() {
            let _ = tx.send(PersistCommand::FlushAndStop);
        }
        if let Some(handle) = self.save_worker.take() {
            let _ = handle.join();
        }
    }
}

fn spawn_save_worker(
    path: PathBuf,
    providers: Arc<Mutex<HashMap<String, ProviderHealth>>>,
    save_lock: Arc<Mutex<()>>,
) -> (Sender<PersistCommand>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(PersistCommand::Dirty) => loop {
                    match rx.recv_timeout(HEALTH_SAVE_DEBOUNCE) {
                        Ok(PersistCommand::Dirty) => continue,
                        Ok(PersistCommand::FlushAndStop) => {
                            let snapshot = ProviderHealthRegistrySnapshot {
                                providers: providers.lock().clone(),
                            };
                            let _guard = save_lock.lock();
                            let _ = save_snapshot(&path, &snapshot);
                            return;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            let snapshot = ProviderHealthRegistrySnapshot {
                                providers: providers.lock().clone(),
                            };
                            let _guard = save_lock.lock();
                            let _ = save_snapshot(&path, &snapshot);
                            break;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            let snapshot = ProviderHealthRegistrySnapshot {
                                providers: providers.lock().clone(),
                            };
                            let _guard = save_lock.lock();
                            let _ = save_snapshot(&path, &snapshot);
                            return;
                        }
                    }
                },
                Ok(PersistCommand::FlushAndStop) => {
                    let snapshot = ProviderHealthRegistrySnapshot {
                        providers: providers.lock().clone(),
                    };
                    let _guard = save_lock.lock();
                    let _ = save_snapshot(&path, &snapshot);
                    return;
                }
                Err(_) => return,
            }
        }
    });
    (tx, handle)
}

fn save_snapshot(
    path: &Path,
    snapshot: &ProviderHealthRegistrySnapshot,
) -> Result<(), std::io::Error> {
    roko_fs::atomic_write_json(path, snapshot)
}

fn new_provider_health(provider_id: &str) -> ProviderHealth {
    ProviderHealth {
        provider_id: provider_id.to_owned(),
        state: CircuitState::Closed,
        consecutive_failures: 0,
        total_requests: 0,
        total_failures: 0,
        last_failure_at: None,
        cooldown_until: None,
        failure_window: VecDeque::new(),
    }
}

fn unix_ms_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

// ─── HealthState ─────────────────────────────────────────────────────────────

/// Circuit-breaker state for a single provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Provider is accepting requests normally.
    Healthy,
    /// Provider has tripped the failure threshold and is cooling down.
    /// `recovery_at` is the earliest instant a probe may be attempted.
    Unhealthy {
        /// Earliest instant at which the provider may be probed.
        recovery_at: Instant,
    },
    /// One probe request has been allowed; awaiting its outcome.
    Probing,
}

// ─── ProviderStatus ──────────────────────────────────────────────────────────

/// Snapshot of a single provider's health bookkeeping.
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    /// Provider identifier (e.g. `"openai"`, `"anthropic"`).
    pub provider: String,
    /// Current circuit-breaker state.
    pub state: HealthState,
    /// Number of failures since the last success.
    pub consecutive_failures: u32,
    /// When the most recent failure was recorded.
    pub last_failure_at: Option<DateTime<Utc>>,
    /// When the most recent success was recorded.
    pub last_success_at: Option<DateTime<Utc>>,
    /// Lifetime attempts routed through this provider.
    pub total_attempts: u64,
    /// Lifetime successful attempts.
    pub total_successes: u64,
}

impl ProviderStatus {
    /// Create a fresh status entry for `provider`.
    const fn new(provider: String) -> Self {
        Self {
            provider,
            state: HealthState::Healthy,
            consecutive_failures: 0,
            last_failure_at: None,
            last_success_at: None,
            total_attempts: 0,
            total_successes: 0,
        }
    }

    /// Return the observed failure rate across all lifetime attempts.
    #[must_use]
    pub fn error_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            return 0.0;
        }

        (self.total_attempts.saturating_sub(self.total_successes)) as f64
            / self.total_attempts as f64
    }
}

// ─── ProviderHealthTracker ───────────────────────────────────────────────────

/// Per-provider circuit breaker that gates bandit arm selection.
///
/// Use [`record_success`](Self::record_success) and
/// [`record_failure`](Self::record_failure) after each LLM call, then
/// call [`is_healthy`](Self::is_healthy) or
/// [`filter_arms`](Self::filter_arms) before selecting the next provider.
pub struct ProviderHealthTracker {
    /// Per-provider status, keyed by provider name.
    providers: RwLock<HashMap<String, ProviderStatus>>,
    /// Number of consecutive failures required to trip the breaker.
    failure_threshold: u32,
    /// Duration a provider stays in `Unhealthy` before a probe is allowed.
    recovery_window: Duration,
}

impl ProviderHealthTracker {
    /// Create a tracker with default thresholds (3 failures, 120 s recovery).
    pub fn new() -> Self {
        Self::with_config(3, Duration::from_secs(120))
    }

    /// Create a tracker with custom thresholds.
    pub fn with_config(failure_threshold: u32, recovery_window: Duration) -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            failure_threshold,
            recovery_window,
        }
    }

    /// Record a successful LLM call for `provider`.
    ///
    /// Resets `consecutive_failures` to 0 and transitions the provider to
    /// [`HealthState::Healthy`] regardless of current state.
    #[allow(clippy::significant_drop_tightening)]
    pub fn record_success(&self, provider: &str) {
        let now = Utc::now();
        let mut map = self.providers.write();
        let status = map
            .entry(provider.to_owned())
            .or_insert_with(|| ProviderStatus::new(provider.to_owned()));

        status.total_attempts += 1;
        status.total_successes += 1;
        status.consecutive_failures = 0;
        status.last_success_at = Some(now);
        status.state = HealthState::Healthy;
    }

    /// Record a failed LLM call for `provider`.
    ///
    /// Increments consecutive failures. When the counter reaches the
    /// configured threshold the provider transitions to
    /// [`HealthState::Unhealthy`].
    #[allow(clippy::significant_drop_tightening)]
    pub fn record_failure(&self, provider: &str) {
        let now = Utc::now();
        let recovery_at = Instant::now() + self.recovery_window;
        let mut map = self.providers.write();
        let status = map
            .entry(provider.to_owned())
            .or_insert_with(|| ProviderStatus::new(provider.to_owned()));

        status.total_attempts += 1;
        status.consecutive_failures = status.consecutive_failures.saturating_add(1);
        status.last_failure_at = Some(now);

        // Transition on threshold or re-trip from Probing.
        if status.consecutive_failures >= self.failure_threshold
            || status.state == HealthState::Probing
        {
            status.state = HealthState::Unhealthy { recovery_at };
        }
    }

    /// Returns `true` if the provider should receive traffic.
    ///
    /// - [`HealthState::Healthy`] → `true`
    /// - [`HealthState::Unhealthy`] with expired recovery window → transitions
    ///   to [`HealthState::Probing`] and returns `true` **once**.
    /// - [`HealthState::Probing`] (already transitioned) → `false`
    /// - [`HealthState::Unhealthy`] not yet expired → `false`
    /// - Unknown provider → `true` (lazily treated as healthy).
    pub fn is_healthy(&self, provider: &str) -> bool {
        // Fast path: read lock only.
        {
            let map = self.providers.read();
            match map.get(provider) {
                None => return true,
                Some(s) => match s.state {
                    HealthState::Healthy => return true,
                    HealthState::Probing => return false,
                    HealthState::Unhealthy { recovery_at } => {
                        if Instant::now() < recovery_at {
                            return false;
                        }
                        // Need to transition — fall through to write path.
                    }
                },
            }
        }

        // Slow path: upgrade to write lock and transition to Probing.
        let mut map = self.providers.write();
        if let Some(status) = map.get_mut(provider) {
            // Re-check after acquiring write lock (another thread may have
            // already transitioned).
            match status.state {
                HealthState::Unhealthy { recovery_at } if Instant::now() >= recovery_at => {
                    status.state = HealthState::Probing;
                    true
                }
                HealthState::Healthy => true,
                _ => false,
            }
        } else {
            // Inserted between our read and write — treat as healthy.
            true
        }
    }

    /// Filter a set of bandit arms, removing those whose provider is
    /// currently unhealthy.
    ///
    /// `provider_of` maps each arm identifier to its provider name.
    pub fn filter_arms<F>(&self, arms: &[String], provider_of: F) -> Vec<String>
    where
        F: Fn(&str) -> String,
    {
        arms.iter()
            .filter(|arm| self.is_healthy(&provider_of(arm)))
            .cloned()
            .collect()
    }

    /// Filter a set of bandit arms, keeping healthy arms when possible and
    /// otherwise returning the least unhealthy fallback arm.
    pub fn filter_arms_or_best<F>(&self, arms: &[String], provider_of: F) -> Vec<String>
    where
        F: Fn(&str) -> String,
    {
        let healthy = self.filter_arms(arms, &provider_of);
        if !healthy.is_empty() {
            return healthy;
        }

        self.least_unhealthy_arm(arms, provider_of)
            .into_iter()
            .collect()
    }

    /// Pick the least unhealthy arm from `arms`.
    pub fn least_unhealthy_arm<F>(&self, arms: &[String], provider_of: F) -> Option<String>
    where
        F: Fn(&str) -> String,
    {
        let now = Instant::now();
        arms.iter()
            .min_by(|left, right| {
                let left_status = self.get(&provider_of(left));
                let right_status = self.get(&provider_of(right));
                health_rank(&left_status, now).cmp(&health_rank(&right_status, now))
            })
            .cloned()
    }

    /// Return a snapshot of every tracked provider's status.
    pub fn snapshot(&self) -> Vec<ProviderStatus> {
        self.providers.read().values().cloned().collect()
    }

    /// Return the current status for `provider`, defaulting to a healthy entry.
    #[must_use]
    pub fn get(&self, provider: &str) -> ProviderStatus {
        self.providers
            .read()
            .get(provider)
            .cloned()
            .unwrap_or_else(|| ProviderStatus::new(provider.to_owned()))
    }
}

impl Default for ProviderHealthTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn health_rank(status: &ProviderStatus, now: Instant) -> (u8, u32, u128, u64) {
    let (state_rank, recovery_delay_ms) = match status.state {
        HealthState::Healthy => (0, 0),
        HealthState::Probing => (1, 0),
        HealthState::Unhealthy { recovery_at } => (
            2,
            recovery_at
                .checked_duration_since(now)
                .unwrap_or_default()
                .as_millis(),
        ),
    };

    (
        state_rank,
        status.consecutive_failures,
        recovery_delay_ms,
        status.total_attempts.saturating_sub(status.total_successes),
    )
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Unknown provider is implicitly healthy.
    #[test]
    fn unknown_provider_is_healthy() {
        let tracker = ProviderHealthTracker::new();
        assert!(tracker.is_healthy("never-seen"));
    }

    /// Three consecutive failures trip the breaker.
    #[test]
    fn three_failures_trips_breaker() {
        let tracker = ProviderHealthTracker::new();
        tracker.record_failure("p");
        tracker.record_failure("p");
        assert!(tracker.is_healthy("p"), "still healthy after 2 failures");

        tracker.record_failure("p");
        assert!(!tracker.is_healthy("p"), "unhealthy after 3 failures");
    }

    /// Two failures then a success resets the counter — stays healthy.
    #[test]
    fn success_resets_failure_counter() {
        let tracker = ProviderHealthTracker::new();
        tracker.record_failure("p");
        tracker.record_failure("p");
        tracker.record_success("p");
        tracker.record_failure("p");
        tracker.record_failure("p");
        assert!(
            tracker.is_healthy("p"),
            "counter was reset so 2 failures is still healthy"
        );
    }

    /// Unhealthy provider before recovery window expires → false.
    #[test]
    fn unhealthy_before_recovery() {
        let tracker = ProviderHealthTracker::with_config(1, Duration::from_secs(600));
        tracker.record_failure("p");
        assert!(!tracker.is_healthy("p"));
    }

    /// After recovery window the first `is_healthy` call returns true
    /// (transitions to Probing).
    #[test]
    fn unhealthy_expires_into_probing() {
        let tracker = ProviderHealthTracker::with_config(1, Duration::from_millis(0));
        tracker.record_failure("p");
        // recovery_at is effectively in the past immediately.
        assert!(
            tracker.is_healthy("p"),
            "first call after recovery → true (Probing)"
        );
    }

    /// While Probing, a second `is_healthy` call returns false.
    #[test]
    fn probing_only_allows_one_request() {
        let tracker = ProviderHealthTracker::with_config(1, Duration::from_millis(0));
        tracker.record_failure("p");
        assert!(tracker.is_healthy("p"), "first probe allowed");
        assert!(
            !tracker.is_healthy("p"),
            "second call while probing → false"
        );
    }

    /// Probing + success → Healthy, counter reset.
    #[test]
    fn probing_success_restores_healthy() {
        let tracker = ProviderHealthTracker::with_config(1, Duration::from_millis(0));
        tracker.record_failure("p");
        assert!(tracker.is_healthy("p")); // transitions to Probing
        tracker.record_success("p");

        // Now it should be Healthy again.
        assert!(tracker.is_healthy("p"));
        // And the counter is reset — one failure alone shouldn't trip it.
        // (Actually threshold is 1 here, so one failure *will* trip it — use 2)
        let snap: Vec<_> = tracker
            .snapshot()
            .into_iter()
            .filter(|s| s.provider == "p")
            .collect();
        assert_eq!(snap[0].consecutive_failures, 0);
    }

    /// Probing + failure → Unhealthy with a new recovery timer.
    #[test]
    fn probing_failure_retrips_breaker() {
        let tracker = ProviderHealthTracker::with_config(1, Duration::from_millis(0));
        tracker.record_failure("p"); // trip
        assert!(tracker.is_healthy("p")); // → Probing

        // Now set a long recovery so re-trip is observable.
        // We can't change config, so instead just check state after failure.
        tracker.record_failure("p");
        // The provider should be Unhealthy again. With 0 ms recovery it will
        // immediately allow probing, but the state transition happened.
        let snap: Vec<_> = tracker
            .snapshot()
            .into_iter()
            .filter(|s| s.provider == "p")
            .collect();
        assert!(
            matches!(snap[0].state, HealthState::Unhealthy { .. }),
            "should be Unhealthy after probe failure"
        );
    }

    /// `filter_arms` removes arms whose provider is unhealthy.
    #[test]
    fn filter_arms_drops_unhealthy() {
        let tracker = ProviderHealthTracker::with_config(1, Duration::from_secs(600));
        tracker.record_failure("bad");
        tracker.record_success("good");

        let arms = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        let result = tracker.filter_arms(&arms, |arm| {
            if arm == "b" {
                "bad".to_owned()
            } else {
                "good".to_owned()
            }
        });
        assert_eq!(result, vec!["a", "c"]);
    }

    /// `filter_arms` with empty input returns empty output.
    #[test]
    fn filter_arms_empty_input() {
        let tracker = ProviderHealthTracker::new();
        let result = tracker.filter_arms(&[], |arm| arm.to_owned());
        assert!(result.is_empty());
    }

    /// `snapshot` returns all tracked providers.
    #[test]
    fn snapshot_returns_all_providers() {
        let tracker = ProviderHealthTracker::new();
        tracker.record_success("alpha");
        tracker.record_failure("beta");
        tracker.record_success("gamma");

        let snap = tracker.snapshot();
        let mut names: Vec<_> = snap.iter().map(|s| s.provider.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }

    /// Concurrent access: 100 tasks each record a failure; final counter
    /// must equal 100.
    #[tokio::test]
    async fn concurrent_failures_are_consistent() {
        let tracker = Arc::new(ProviderHealthTracker::with_config(
            200,
            Duration::from_secs(600),
        ));
        let mut handles = Vec::new();

        for _ in 0..100 {
            let t = Arc::clone(&tracker);
            handles.push(tokio::spawn(async move {
                t.record_failure("contended");
            }));
        }

        for h in handles {
            h.await.expect("task panicked");
        }

        let snap: Vec<_> = tracker
            .snapshot()
            .into_iter()
            .filter(|s| s.provider == "contended")
            .collect();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].consecutive_failures, 100);
        assert_eq!(snap[0].total_attempts, 100);
    }

    /// Concurrent mixed operations: successes and failures interleaved.
    #[tokio::test]
    async fn concurrent_mixed_operations() {
        let tracker = Arc::new(ProviderHealthTracker::with_config(
            200,
            Duration::from_secs(600),
        ));
        let mut handles = Vec::new();

        for i in 0..100 {
            let t = Arc::clone(&tracker);
            handles.push(tokio::spawn(async move {
                if i % 2 == 0 {
                    t.record_success("mixed");
                } else {
                    t.record_failure("mixed");
                }
            }));
        }

        for h in handles {
            h.await.expect("task panicked");
        }

        let snap: Vec<_> = tracker
            .snapshot()
            .into_iter()
            .filter(|s| s.provider == "mixed")
            .collect();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].total_attempts, 100);
        assert_eq!(snap[0].total_successes, 50);
    }

    /// Serializable snapshot types round-trip through JSON.
    #[test]
    fn provider_health_types() {
        let health = ProviderHealth {
            provider_id: "anthropic".to_owned(),
            state: CircuitState::HalfOpen,
            consecutive_failures: 3,
            total_requests: 42,
            total_failures: 7,
            last_failure_at: Some(1_725_000_000_000),
            cooldown_until: Some(1_725_000_030_000),
            failure_window: VecDeque::from(vec![
                FailureRecord {
                    timestamp_ms: 1_725_000_000_000,
                    error_class: ErrorClass::RateLimit,
                },
                FailureRecord {
                    timestamp_ms: 1_725_000_010_000,
                    error_class: ErrorClass::Timeout,
                },
            ]),
        };

        let json = serde_json::to_string(&health).expect("serialize provider health");
        let decoded: ProviderHealth =
            serde_json::from_str(&json).expect("deserialize provider health");
        assert_eq!(decoded, health);

        let state_json = serde_json::to_string(&CircuitState::Open).expect("serialize state");
        let decoded_state: CircuitState =
            serde_json::from_str(&state_json).expect("deserialize state");
        assert_eq!(decoded_state, CircuitState::Open);
    }

    /// Three consecutive failures trip the circuit to Open, and cooldown
    /// expiry advances it to HalfOpen.
    #[test]
    fn provider_health_circuit_breaker_transitions() {
        let mut health = ProviderHealth {
            provider_id: "openai".to_owned(),
            state: CircuitState::Closed,
            consecutive_failures: 0,
            total_requests: 0,
            total_failures: 0,
            last_failure_at: None,
            cooldown_until: None,
            failure_window: VecDeque::new(),
        };

        health.record_failure(ErrorClass::Timeout, 1_000);
        health.record_failure(ErrorClass::Timeout, 2_000);
        assert_eq!(health.state, CircuitState::Closed);
        assert!(health.is_available(2_500));

        health.record_failure(ErrorClass::Timeout, 3_000);
        assert_eq!(health.state, CircuitState::Open);
        assert_eq!(health.cooldown_until, Some(13_000));
        assert!(!health.is_available(12_999));
        assert!(health.is_available(13_000));
        assert_eq!(health.state, CircuitState::HalfOpen);

        health.record_success();
        assert_eq!(health.state, CircuitState::Closed);
        assert_eq!(health.consecutive_failures, 0);
    }

    /// Error classes map to distinct cooldown durations.
    #[test]
    fn provider_health_circuit_breaker_cooldowns() {
        let mut health = ProviderHealth {
            provider_id: "anthropic".to_owned(),
            state: CircuitState::Closed,
            consecutive_failures: 0,
            total_requests: 0,
            total_failures: 0,
            last_failure_at: None,
            cooldown_until: None,
            failure_window: VecDeque::new(),
        };

        health.record_failure(ErrorClass::RateLimit, 10);
        health.record_failure(ErrorClass::RateLimit, 20);
        health.record_failure(ErrorClass::RateLimit, 30);
        assert_eq!(health.cooldown_until, Some(5_030));

        health.state = CircuitState::Closed;
        health.consecutive_failures = 0;
        health.cooldown_until = None;

        health.record_failure(ErrorClass::AuthFailure, 100);
        health.record_failure(ErrorClass::AuthFailure, 200);
        health.record_failure(ErrorClass::AuthFailure, 300);
        assert_eq!(health.cooldown_until, Some(300_300));
    }

    /// Registry stores per-provider state and filters unavailable providers.
    #[test]
    fn provider_health_registry_filters_unavailable_providers() {
        let registry = ProviderHealthRegistry::new();
        registry.record_success("good");
        registry.record_failure("bad", ErrorClass::Timeout);
        registry.record_failure("bad", ErrorClass::Timeout);
        registry.record_failure("bad", ErrorClass::Timeout);

        let candidates = vec!["good".to_owned(), "bad".to_owned(), "unknown".to_owned()];
        assert_eq!(
            registry.available_providers(&candidates),
            vec!["good".to_owned(), "unknown".to_owned()]
        );
    }

    /// Registry snapshots persist to disk and load back intact.
    #[test]
    fn provider_health_registry_roundtrip() {
        let tmp = TempDir::new().expect("create tempdir");
        let path = tmp.path().join("provider-health.json");

        let registry = ProviderHealthRegistry::new();
        registry.record_success("alpha");
        registry.record_failure("beta", ErrorClass::RateLimit);
        registry.record_failure("beta", ErrorClass::RateLimit);
        registry.record_failure("beta", ErrorClass::RateLimit);
        registry.save(&path).expect("save registry");

        let loaded = ProviderHealthRegistry::load_or_new(&path);
        assert!(loaded.is_available("alpha"));
        assert!(!loaded.is_available("beta"));

        let mut providers = loaded.providers.lock().keys().cloned().collect::<Vec<_>>();
        providers.sort();
        assert_eq!(providers, vec!["alpha".to_owned(), "beta".to_owned()]);
    }

    /// Persisted registry state survives a restart without a manual save.
    #[test]
    fn provider_health_health_persistence_round_trip() {
        let tmp = TempDir::new().expect("create tempdir");
        let path = tmp.path().join(".roko/learn/provider-health.json");

        {
            let registry = ProviderHealthRegistry::load_or_new(&path);
            registry.record_success("alpha");
            registry.record_failure("beta", ErrorClass::Timeout);
            registry.record_failure("beta", ErrorClass::Timeout);
            registry.record_failure("beta", ErrorClass::Timeout);

            let deadline = std::time::Instant::now() + Duration::from_millis(1_000);
            while !path.exists() && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(25));
            }
            assert!(path.exists(), "debounced autosave should create the file");

            let loaded = ProviderHealthRegistry::load_or_new(&path);
            assert!(loaded.is_available("alpha"));
            assert!(!loaded.is_available("beta"));

            let beta = loaded
                .providers
                .lock()
                .get("beta")
                .cloned()
                .expect("beta state");
            assert_eq!(beta.provider_id, "beta");
            assert_eq!(beta.total_failures, 3);
            assert_eq!(beta.state, CircuitState::Open);
        }
    }

    // ── Health snapshot recording ────────────────────────────────────────

    /// `record_success` increments total_requests and total_successes.
    #[test]
    fn snapshot_record_success_updates_counters() {
        let mut h = new_provider_health("test");
        h.record_success();
        assert_eq!(h.total_requests, 1);
        assert_eq!(h.total_failures, 0);
        assert_eq!(h.consecutive_failures, 0);
        assert_eq!(h.state, CircuitState::Closed);

        h.record_success();
        assert_eq!(h.total_requests, 2);
    }

    /// `record_failure` increments total_requests, total_failures, and
    /// consecutive_failures, and appends to the failure window.
    #[test]
    fn snapshot_record_failure_updates_counters() {
        let mut h = new_provider_health("test");
        h.record_failure(ErrorClass::Timeout, 1000);
        assert_eq!(h.total_requests, 1);
        assert_eq!(h.total_failures, 1);
        assert_eq!(h.consecutive_failures, 1);
        assert_eq!(h.last_failure_at, Some(1000));
        assert_eq!(h.failure_window.len(), 1);
        assert_eq!(h.failure_window[0].error_class, ErrorClass::Timeout);
        assert_eq!(h.failure_window[0].timestamp_ms, 1000);
    }

    /// The failure window is capped at 20 entries.
    #[test]
    fn snapshot_failure_window_caps_at_20() {
        let mut h = new_provider_health("test");
        for i in 0..25 {
            h.record_failure(ErrorClass::ServerError, i * 100);
        }
        assert_eq!(h.failure_window.len(), 20);
        // The oldest entries should have been evicted: first remaining
        // should be from i=5 (timestamp 500).
        assert_eq!(h.failure_window.front().unwrap().timestamp_ms, 500);
        assert_eq!(h.failure_window.back().unwrap().timestamp_ms, 2400);
    }

    /// `record_success` clears consecutive_failures and cooldown.
    #[test]
    fn snapshot_success_clears_failure_state() {
        let mut h = new_provider_health("test");
        h.record_failure(ErrorClass::Timeout, 100);
        h.record_failure(ErrorClass::Timeout, 200);
        assert_eq!(h.consecutive_failures, 2);
        h.record_success();
        assert_eq!(h.consecutive_failures, 0);
        assert_eq!(h.cooldown_until, None);
        assert_eq!(h.total_requests, 3);
        assert_eq!(h.total_failures, 2);
    }

    // ── Degradation detection thresholds ────────────────────────────────

    /// Exactly 2 failures do not trip the circuit (threshold is 3).
    #[test]
    fn two_failures_below_threshold() {
        let mut h = new_provider_health("test");
        h.record_failure(ErrorClass::RateLimit, 10);
        h.record_failure(ErrorClass::RateLimit, 20);
        assert_eq!(h.state, CircuitState::Closed);
        assert!(h.is_available(30));
    }

    /// Exactly 3 failures trip the circuit to Open.
    #[test]
    fn three_failures_trip_circuit() {
        let mut h = new_provider_health("test");
        h.record_failure(ErrorClass::RateLimit, 10);
        h.record_failure(ErrorClass::RateLimit, 20);
        h.record_failure(ErrorClass::RateLimit, 30);
        assert_eq!(h.state, CircuitState::Open);
        assert!(h.cooldown_until.is_some());
    }

    /// More than 3 consecutive failures keep the circuit Open and update
    /// the cooldown based on the most recent error class.
    #[test]
    fn additional_failures_extend_cooldown() {
        let mut h = new_provider_health("test");
        // First 3 with RateLimit (5s cooldown)
        h.record_failure(ErrorClass::RateLimit, 100);
        h.record_failure(ErrorClass::RateLimit, 200);
        h.record_failure(ErrorClass::RateLimit, 300);
        assert_eq!(h.cooldown_until, Some(5_300));

        // 4th failure with ServerError (30s cooldown) should extend
        h.record_failure(ErrorClass::ServerError, 400);
        assert_eq!(h.state, CircuitState::Open);
        assert_eq!(h.cooldown_until, Some(30_400));
        assert_eq!(h.consecutive_failures, 4);
    }

    /// Each error class produces a distinct cooldown duration.
    #[test]
    fn error_class_cooldown_values() {
        let h = new_provider_health("test");
        assert_eq!(h.cooldown_ms(ErrorClass::RateLimit), 5_000);
        assert_eq!(h.cooldown_ms(ErrorClass::Timeout), 10_000);
        assert_eq!(h.cooldown_ms(ErrorClass::ServerError), 30_000);
        assert_eq!(h.cooldown_ms(ErrorClass::AuthFailure), 300_000);
        assert_eq!(h.cooldown_ms(ErrorClass::ContentPolicy), 5_000);
        assert_eq!(h.cooldown_ms(ErrorClass::ContextOverflow), 5_000);
        assert_eq!(h.cooldown_ms(ErrorClass::Unknown), 5_000);
    }

    // ── Health status transitions ────────────────────────────────────────

    /// Full lifecycle: Closed -> Open -> HalfOpen -> Closed via success.
    #[test]
    fn full_transition_closed_open_halfopen_closed() {
        let mut h = new_provider_health("test");
        assert_eq!(h.state, CircuitState::Closed);

        // Trip to Open
        h.record_failure(ErrorClass::RateLimit, 100);
        h.record_failure(ErrorClass::RateLimit, 200);
        h.record_failure(ErrorClass::RateLimit, 300);
        assert_eq!(h.state, CircuitState::Open);
        // cooldown_until = 300 + 5000 = 5300

        // Before cooldown expires -> unavailable
        assert!(!h.is_available(5299));
        assert_eq!(h.state, CircuitState::Open);

        // After cooldown expires -> HalfOpen
        assert!(h.is_available(5300));
        assert_eq!(h.state, CircuitState::HalfOpen);

        // Success from HalfOpen -> Closed
        h.record_success();
        assert_eq!(h.state, CircuitState::Closed);
        assert_eq!(h.consecutive_failures, 0);
    }

    /// Full lifecycle: Closed -> Open -> HalfOpen -> Open via failure.
    #[test]
    fn transition_halfopen_failure_retrips() {
        let mut h = new_provider_health("test");

        // Trip to Open
        h.record_failure(ErrorClass::Timeout, 100);
        h.record_failure(ErrorClass::Timeout, 200);
        h.record_failure(ErrorClass::Timeout, 300);
        assert_eq!(h.state, CircuitState::Open);
        // cooldown_until = 300 + 10000 = 10300

        // Advance past cooldown -> HalfOpen
        assert!(h.is_available(10300));
        assert_eq!(h.state, CircuitState::HalfOpen);

        // Failure from HalfOpen should re-trip to Open
        h.record_failure(ErrorClass::Timeout, 10400);
        assert_eq!(h.state, CircuitState::Open);
        assert_eq!(h.cooldown_until, Some(10400 + 10_000));
    }

    /// Success from Open state (e.g. after reload) transitions to Closed.
    #[test]
    fn success_from_open_transitions_to_closed() {
        let mut h = new_provider_health("test");
        h.state = CircuitState::Open;
        h.consecutive_failures = 5;
        h.cooldown_until = Some(999_999);

        h.record_success();
        assert_eq!(h.state, CircuitState::Closed);
        assert_eq!(h.consecutive_failures, 0);
        assert_eq!(h.cooldown_until, None);
    }

    /// Success from Closed stays Closed.
    #[test]
    fn success_from_closed_stays_closed() {
        let mut h = new_provider_health("test");
        h.record_success();
        assert_eq!(h.state, CircuitState::Closed);
    }

    // ── Recovery detection ──────────────────────────────────────────────

    /// is_available returns false for Open circuit before cooldown.
    #[test]
    fn is_available_false_during_cooldown() {
        let mut h = new_provider_health("test");
        h.state = CircuitState::Open;
        h.cooldown_until = Some(10_000);
        assert!(!h.is_available(9_999));
    }

    /// is_available returns true and transitions to HalfOpen at
    /// exactly the cooldown boundary.
    #[test]
    fn is_available_transitions_at_cooldown_boundary() {
        let mut h = new_provider_health("test");
        h.state = CircuitState::Open;
        h.cooldown_until = Some(10_000);
        assert!(h.is_available(10_000));
        assert_eq!(h.state, CircuitState::HalfOpen);
    }

    /// is_available returns true for HalfOpen (probe allowed).
    #[test]
    fn is_available_true_for_halfopen() {
        let mut h = new_provider_health("test");
        h.state = CircuitState::HalfOpen;
        assert!(h.is_available(0));
    }

    /// Recovery cycle: trip -> wait -> probe succeeds -> healthy again.
    #[test]
    fn recovery_cycle_via_probe_success() {
        let mut h = new_provider_health("test");

        // Trip
        h.record_failure(ErrorClass::ServerError, 100);
        h.record_failure(ErrorClass::ServerError, 200);
        h.record_failure(ErrorClass::ServerError, 300);
        assert_eq!(h.state, CircuitState::Open);
        let cooldown = h.cooldown_until.unwrap(); // 300 + 30_000 = 30_300

        // Still blocked
        assert!(!h.is_available(cooldown - 1));

        // Probe allowed
        assert!(h.is_available(cooldown));
        assert_eq!(h.state, CircuitState::HalfOpen);

        // Probe succeeds
        h.record_success();
        assert_eq!(h.state, CircuitState::Closed);
        assert!(h.is_available(cooldown + 100));
    }

    /// Recovery cycle: trip -> wait -> probe fails -> re-tripped with
    /// new cooldown.
    #[test]
    fn recovery_cycle_probe_failure_retrips() {
        let mut h = new_provider_health("test");

        // Trip
        h.record_failure(ErrorClass::RateLimit, 100);
        h.record_failure(ErrorClass::RateLimit, 200);
        h.record_failure(ErrorClass::RateLimit, 300);
        let first_cooldown = h.cooldown_until.unwrap();

        // Probe
        assert!(h.is_available(first_cooldown));
        assert_eq!(h.state, CircuitState::HalfOpen);

        // Probe fails
        h.record_failure(ErrorClass::RateLimit, first_cooldown + 100);
        assert_eq!(h.state, CircuitState::Open);
        let second_cooldown = h.cooldown_until.unwrap();
        assert!(
            second_cooldown > first_cooldown,
            "new cooldown should be later"
        );
    }

    // ── Serialization / persistence roundtrip ───────────────────────────

    /// Full ProviderHealth struct serializes and deserializes faithfully.
    #[test]
    fn provider_health_serde_roundtrip_full() {
        let mut window = VecDeque::new();
        window.push_back(FailureRecord {
            timestamp_ms: 1_000,
            error_class: ErrorClass::RateLimit,
        });
        window.push_back(FailureRecord {
            timestamp_ms: 2_000,
            error_class: ErrorClass::ServerError,
        });
        window.push_back(FailureRecord {
            timestamp_ms: 3_000,
            error_class: ErrorClass::AuthFailure,
        });

        let health = ProviderHealth {
            provider_id: "test-provider".to_owned(),
            state: CircuitState::Open,
            consecutive_failures: 5,
            total_requests: 100,
            total_failures: 20,
            last_failure_at: Some(3_000),
            cooldown_until: Some(33_000),
            failure_window: window,
        };

        let json = serde_json::to_string_pretty(&health).unwrap();
        let decoded: ProviderHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, health);
        assert_eq!(decoded.failure_window.len(), 3);
        assert_eq!(
            decoded.failure_window[2].error_class,
            ErrorClass::AuthFailure
        );
    }

    /// All CircuitState variants roundtrip through JSON.
    #[test]
    fn circuit_state_serde_all_variants() {
        for state in [
            CircuitState::Closed,
            CircuitState::Open,
            CircuitState::HalfOpen,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let decoded: CircuitState = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, state, "roundtrip failed for {state:?}");
        }
    }

    /// All ErrorClass variants roundtrip through JSON.
    #[test]
    fn error_class_serde_all_variants() {
        let classes = [
            ErrorClass::RateLimit,
            ErrorClass::AuthFailure,
            ErrorClass::Timeout,
            ErrorClass::ServerError,
            ErrorClass::ContentPolicy,
            ErrorClass::ContextOverflow,
            ErrorClass::Unknown,
        ];
        for class in classes {
            let json = serde_json::to_string(&class).unwrap();
            let decoded: ErrorClass = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, class, "roundtrip failed for {class:?}");
        }
    }

    /// ProviderHealth with empty optional fields and empty window.
    #[test]
    fn provider_health_serde_minimal() {
        let health = new_provider_health("minimal");
        let json = serde_json::to_string(&health).unwrap();
        let decoded: ProviderHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, health);
        assert_eq!(decoded.last_failure_at, None);
        assert_eq!(decoded.cooldown_until, None);
        assert!(decoded.failure_window.is_empty());
    }

    /// Registry save/load roundtrip preserves multiple providers
    /// including their failure windows and circuit states.
    #[test]
    fn registry_persistence_preserves_failure_windows() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("health.json");

        let registry = ProviderHealthRegistry::new();
        // Build up some state
        registry.record_success("healthy");
        registry.record_failure("failing", ErrorClass::RateLimit);
        registry.record_failure("failing", ErrorClass::Timeout);
        registry.record_failure("failing", ErrorClass::ServerError);
        registry.save(&path).unwrap();

        let loaded = ProviderHealthRegistry::load_or_new(&path);
        let snap = loaded.snapshot();

        let healthy = snap.get("healthy").expect("healthy provider");
        assert_eq!(healthy.total_requests, 1);
        assert_eq!(healthy.state, CircuitState::Closed);

        let failing = snap.get("failing").expect("failing provider");
        assert_eq!(failing.total_failures, 3);
        assert_eq!(failing.consecutive_failures, 3);
        assert_eq!(failing.state, CircuitState::Open);
        assert_eq!(failing.failure_window.len(), 3);
        assert_eq!(failing.failure_window[0].error_class, ErrorClass::RateLimit);
        assert_eq!(failing.failure_window[1].error_class, ErrorClass::Timeout);
        assert_eq!(
            failing.failure_window[2].error_class,
            ErrorClass::ServerError
        );
    }

    /// Loading from a nonexistent path returns an empty registry.
    #[test]
    fn registry_load_nonexistent_returns_empty() {
        let registry = ProviderHealthRegistry::load_or_new(Path::new("/nonexistent/path.json"));
        assert!(registry.snapshot().is_empty());
    }

    /// Loading from a file with invalid JSON returns an empty registry.
    #[test]
    fn registry_load_invalid_json_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("corrupt.json");
        std::fs::write(&path, "not valid json{{{").unwrap();

        let registry = ProviderHealthRegistry::load_or_new(&path);
        assert!(registry.snapshot().is_empty());
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    /// First observation: a brand-new provider starts Closed with zero
    /// counters.
    #[test]
    fn first_observation_starts_healthy() {
        let h = new_provider_health("brand-new");
        assert_eq!(h.state, CircuitState::Closed);
        assert_eq!(h.consecutive_failures, 0);
        assert_eq!(h.total_requests, 0);
        assert_eq!(h.total_failures, 0);
        assert_eq!(h.last_failure_at, None);
        assert_eq!(h.cooldown_until, None);
        assert!(h.failure_window.is_empty());
    }

    /// Tracker `get` for an unknown provider returns a healthy default.
    #[test]
    fn tracker_get_unknown_returns_healthy_default() {
        let tracker = ProviderHealthTracker::new();
        let status = tracker.get("never-seen-before");
        assert_eq!(status.provider, "never-seen-before");
        assert_eq!(status.state, HealthState::Healthy);
        assert_eq!(status.consecutive_failures, 0);
        assert_eq!(status.total_attempts, 0);
    }

    /// Registry `get` for an unknown provider returns a healthy default.
    #[test]
    fn registry_get_unknown_returns_healthy_default() {
        let registry = ProviderHealthRegistry::new();
        let health = registry.get("unknown-provider");
        assert_eq!(health.provider_id, "unknown-provider");
        assert_eq!(health.state, CircuitState::Closed);
        assert_eq!(health.consecutive_failures, 0);
    }

    /// Rapid transitions: trip -> immediate recovery -> re-trip in quick
    /// succession.
    #[test]
    fn rapid_transitions_trip_recover_retrip() {
        let mut h = new_provider_health("rapid");

        // First trip
        h.record_failure(ErrorClass::RateLimit, 100);
        h.record_failure(ErrorClass::RateLimit, 101);
        h.record_failure(ErrorClass::RateLimit, 102);
        assert_eq!(h.state, CircuitState::Open);
        let cd1 = h.cooldown_until.unwrap();

        // Immediately recover
        assert!(h.is_available(cd1));
        assert_eq!(h.state, CircuitState::HalfOpen);
        h.record_success();
        assert_eq!(h.state, CircuitState::Closed);

        // Immediately re-trip
        h.record_failure(ErrorClass::Timeout, cd1 + 1);
        h.record_failure(ErrorClass::Timeout, cd1 + 2);
        h.record_failure(ErrorClass::Timeout, cd1 + 3);
        assert_eq!(h.state, CircuitState::Open);
        let cd2 = h.cooldown_until.unwrap();
        assert!(cd2 > cd1, "second cooldown should be after first");

        // Recover again
        assert!(h.is_available(cd2));
        assert_eq!(h.state, CircuitState::HalfOpen);
        h.record_success();
        assert_eq!(h.state, CircuitState::Closed);
    }

    /// A success immediately after a single failure keeps the provider
    /// Closed (no intermediate trip).
    #[test]
    fn interleaved_success_failure_no_trip() {
        let mut h = new_provider_health("interleaved");
        for _ in 0..10 {
            h.record_failure(ErrorClass::Timeout, 100);
            h.record_success();
        }
        assert_eq!(h.state, CircuitState::Closed);
        assert_eq!(h.consecutive_failures, 0);
        assert_eq!(h.total_requests, 20);
        assert_eq!(h.total_failures, 10);
    }

    /// Open circuit with no cooldown_until set: is_available returns
    /// false (guards against missing cooldown).
    #[test]
    fn open_without_cooldown_stays_unavailable() {
        let mut h = new_provider_health("test");
        h.state = CircuitState::Open;
        h.cooldown_until = None;
        assert!(!h.is_available(999_999_999));
    }

    /// ProviderStatus::error_rate is correct with zero and nonzero
    /// attempts.
    #[test]
    fn error_rate_calculation() {
        let status = ProviderStatus::new("test".to_owned());
        assert_eq!(status.error_rate(), 0.0);

        let tracker = ProviderHealthTracker::new();
        tracker.record_success("er");
        tracker.record_success("er");
        tracker.record_failure("er");
        let snap = tracker.get("er");
        // 3 attempts, 2 successes => error rate = 1/3
        let expected = 1.0 / 3.0;
        assert!((snap.error_rate() - expected).abs() < 1e-10);
    }

    /// Tracker with custom config: threshold=5 means 4 failures stay
    /// healthy, 5th trips.
    #[test]
    fn custom_threshold_respected() {
        let tracker = ProviderHealthTracker::with_config(5, Duration::from_secs(60));
        for _ in 0..4 {
            tracker.record_failure("p");
        }
        assert!(tracker.is_healthy("p"));
        tracker.record_failure("p");
        assert!(!tracker.is_healthy("p"));
    }

    /// `filter_arms_or_best` returns healthy arms when available, falls
    /// back to least unhealthy when all are down.
    #[test]
    fn filter_arms_or_best_fallback() {
        let tracker = ProviderHealthTracker::with_config(1, Duration::from_secs(600));

        // Make all providers unhealthy
        tracker.record_failure("p1");
        tracker.record_failure("p2");
        // p2 has more total failures -> worse
        tracker.record_failure("p2");

        let arms = vec!["a".to_owned(), "b".to_owned()];
        let result = tracker.filter_arms_or_best(&arms, |arm| {
            if arm == "a" {
                "p1".to_owned()
            } else {
                "p2".to_owned()
            }
        });
        // All unhealthy so fallback should return exactly one arm
        assert_eq!(result.len(), 1);
    }

    /// `filter_arms_or_best` returns all healthy arms when some are
    /// available.
    #[test]
    fn filter_arms_or_best_prefers_healthy() {
        let tracker = ProviderHealthTracker::with_config(1, Duration::from_secs(600));
        tracker.record_failure("bad");
        tracker.record_success("good");

        let arms = vec!["a".to_owned(), "b".to_owned()];
        let result = tracker.filter_arms_or_best(&arms, |arm| {
            if arm == "a" {
                "good".to_owned()
            } else {
                "bad".to_owned()
            }
        });
        assert_eq!(result, vec!["a"]);
    }

    /// Registry `is_healthy` (non-mutating) for unknown provider returns
    /// true.
    #[test]
    fn registry_is_healthy_unknown_true() {
        let registry = ProviderHealthRegistry::new();
        assert!(registry.is_healthy("ghost"));
    }

    /// Registry `is_healthy` for a Closed provider returns true.
    #[test]
    fn registry_is_healthy_closed_true() {
        let registry = ProviderHealthRegistry::new();
        registry.record_success("ok");
        assert!(registry.is_healthy("ok"));
    }

    /// Registry `is_healthy` for an Open provider before cooldown returns
    /// false.
    #[test]
    fn registry_is_healthy_open_false() {
        let registry = ProviderHealthRegistry::new();
        registry.record_failure("bad", ErrorClass::AuthFailure);
        registry.record_failure("bad", ErrorClass::AuthFailure);
        registry.record_failure("bad", ErrorClass::AuthFailure);
        // AuthFailure has 300s cooldown, so definitely still Open
        assert!(!registry.is_healthy("bad"));
    }

    /// Tracker Probing state: a failure during Probing re-trips even
    /// if consecutive_failures is below threshold.
    #[test]
    fn probing_failure_retrips_regardless_of_threshold() {
        let tracker = ProviderHealthTracker::with_config(5, Duration::from_millis(0));
        // Need 5 failures to trip
        for _ in 0..5 {
            tracker.record_failure("p");
        }
        // With 0ms recovery, first is_healthy call transitions to Probing
        assert!(tracker.is_healthy("p")); // -> Probing
        // One failure during Probing should re-trip
        tracker.record_failure("p");
        let snap = tracker.get("p");
        assert!(
            matches!(snap.state, HealthState::Unhealthy { .. }),
            "single failure during Probing should re-trip"
        );
    }

    /// Saturating arithmetic: consecutive_failures and total counters
    /// don't overflow.
    #[test]
    fn saturating_counters() {
        let mut h = new_provider_health("sat");
        h.consecutive_failures = u32::MAX;
        h.total_requests = u64::MAX;
        h.total_failures = u64::MAX;
        // Should not panic
        h.record_failure(ErrorClass::Unknown, 1);
        assert_eq!(h.consecutive_failures, u32::MAX);
        assert_eq!(h.total_requests, u64::MAX);
        assert_eq!(h.total_failures, u64::MAX);
    }

    /// Multiple providers are tracked independently in the tracker.
    #[test]
    fn independent_provider_tracking() {
        let tracker = ProviderHealthTracker::with_config(2, Duration::from_secs(600));
        tracker.record_failure("a");
        tracker.record_failure("a");
        tracker.record_failure("b");

        assert!(
            !tracker.is_healthy("a"),
            "a should be tripped after 2 failures"
        );
        assert!(
            tracker.is_healthy("b"),
            "b should still be healthy after 1 failure"
        );
        assert!(tracker.is_healthy("c"), "c (unknown) should be healthy");
    }

    /// Multiple providers are tracked independently in the registry.
    #[test]
    fn registry_independent_providers() {
        let registry = ProviderHealthRegistry::new();
        registry.record_failure("x", ErrorClass::Timeout);
        registry.record_failure("x", ErrorClass::Timeout);
        registry.record_failure("x", ErrorClass::Timeout);
        registry.record_success("y");

        assert!(!registry.is_available("x"));
        assert!(registry.is_available("y"));
    }
}
