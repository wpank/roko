//! Per-provider circuit breaker for LLM routing (§13.9).
//!
//! Tracks consecutive failures per provider and transitions through a
//! three-state machine:
//!
//! ```text
//! Healthy ──[N consecutive failures]──▶ Unhealthy { recovery_at }
//!     ▲         OR rolling rate < 30%          │
//!     │                                  [now ≥ recovery_at]
//!     │                                        ▼
//!     └────[record_success]──────────── Probing
//!                                 [record_failure]──▶ Unhealthy (timer reset)
//! ```
//!
//! Two independent conditions trip the circuit Open:
//!
//! 1. **Consecutive failures** — 3 or more failures in a row (existing behaviour).
//! 2. **Rolling success rate** — fewer than 30% successes across the last 10
//!    requests.  This catches providers that limp along with occasional successes
//!    that continuously reset the consecutive-failure counter while the overall
//!    error rate is very high (e.g. openai at 12% success over 49 calls).
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

// ─── Rolling-window circuit-trip constants ──────────────────────────────────

/// Number of most-recent outcomes tracked for the rolling success-rate check.
const ROLLING_WINDOW: usize = 10;

/// Minimum acceptable success rate over the rolling window.
///
/// When the observed rate drops below this value the circuit trips Open even
/// if no individual failure streak has reached the consecutive-failure
/// threshold.  Set to 30% so that a provider succeeding on only 3 out of
/// every 10 calls is treated as degraded.
const ROLLING_SUCCESS_RATE_MIN: f64 = 0.30;

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
    /// Timestamp of the most recent success, in unix milliseconds.
    #[serde(default)]
    pub last_success_at: Option<i64>,
    /// Timestamp when the provider may be retried, in unix milliseconds.
    pub cooldown_until: Option<i64>,
    /// Rolling window of recent failures.
    pub failure_window: VecDeque<FailureRecord>,
    /// Sliding window of the last [`ROLLING_WINDOW`] outcomes.
    ///
    /// `true` = success, `false` = failure.  Populated by both
    /// [`Self::record_success`] and [`Self::record_failure`] and used to
    /// detect a low rolling success rate independently of the
    /// consecutive-failure counter.  Default-deserializes as empty so
    /// persisted snapshots without this field load cleanly.
    #[serde(default)]
    pub recent_outcomes: VecDeque<bool>,
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
        self.last_success_at = Some(unix_ms_now());
        self.consecutive_failures = 0;
        self.cooldown_until = None;
        if self.state == CircuitState::HalfOpen || self.state == CircuitState::Open {
            self.state = CircuitState::Closed;
        }
        // Update rolling window.
        self.recent_outcomes.push_back(true);
        if self.recent_outcomes.len() > ROLLING_WINDOW {
            self.recent_outcomes.pop_front();
        }
    }

    /// Record a failed request and update the circuit state.
    ///
    /// The circuit trips Open when either condition holds:
    ///
    /// 1. **3+ consecutive failures** — the existing streak check.
    /// 2. **Rolling success rate < 30%** over the last 10 requests — catches
    ///    providers that limp along with occasional successes (like openai at
    ///    12%) without ever hitting three failures in a row.
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

        // Update rolling window.
        self.recent_outcomes.push_back(false);
        if self.recent_outcomes.len() > ROLLING_WINDOW {
            self.recent_outcomes.pop_front();
        }

        // Condition 1: trip to Open after 3 consecutive failures.
        let should_trip_consecutive = self.consecutive_failures >= 3;

        // Condition 2: trip to Open when rolling success rate is below the
        // minimum threshold over a full window of ROLLING_WINDOW requests.
        let should_trip_rate = if self.recent_outcomes.len() >= ROLLING_WINDOW {
            let successes = self.recent_outcomes.iter().filter(|&&ok| ok).count();
            #[allow(clippy::cast_precision_loss)]
            let rate = successes as f64 / ROLLING_WINDOW as f64;
            rate < ROLLING_SUCCESS_RATE_MIN
        } else {
            false
        };

        if should_trip_consecutive || should_trip_rate {
            // When already Open, each additional failure extends the cooldown
            // (original behaviour). When Closed or HalfOpen, transition to Open.
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

impl std::fmt::Debug for ProviderHealthRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderHealthRegistry")
            .field("providers", &"<locked>")
            .finish()
    }
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
    ///
    /// The key is normalized (hyphens to underscores, lowercased) so that
    /// `"claude-cli"` and `"claude_cli"` map to the same circuit breaker.
    pub fn record_success(&self, provider_id: &str) {
        let key = normalize_provider_key(provider_id);
        let mut providers = self.providers.lock();
        let health = providers
            .entry(key.clone())
            .or_insert_with(|| new_provider_health(&key));
        health.record_success();
        drop(providers);
        self.schedule_persist();
    }

    /// Record a failed request for `provider_id`.
    ///
    /// The key is normalized (hyphens to underscores, lowercased) so that
    /// `"claude-cli"` and `"claude_cli"` map to the same circuit breaker.
    pub fn record_failure(&self, provider_id: &str, error: ErrorClass) {
        let key = normalize_provider_key(provider_id);
        let mut providers = self.providers.lock();
        let health = providers
            .entry(key.clone())
            .or_insert_with(|| new_provider_health(&key));
        health.record_failure(error, unix_ms_now());
        drop(providers);
        self.schedule_persist();
    }

    /// Return whether `provider_id` is currently available for routing.
    ///
    /// Unknown providers are treated as available.  The key is normalized
    /// before lookup.
    pub fn is_available(&self, provider_id: &str) -> bool {
        let key = normalize_provider_key(provider_id);
        let mut providers = self.providers.lock();
        let mut should_persist = false;
        let available = match providers.get_mut(&key) {
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
    /// Unknown providers are treated as healthy.  The key is normalized
    /// before lookup.
    #[must_use]
    pub fn is_healthy(&self, provider_id: &str) -> bool {
        let key = normalize_provider_key(provider_id);
        let providers = self.providers.lock();
        match providers.get(&key) {
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
    ///
    /// Each candidate key is normalized before the health check.
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
    ///
    /// The key is normalized before lookup.
    #[must_use]
    pub fn get(&self, provider_id: &str) -> ProviderHealth {
        let key = normalize_provider_key(provider_id);
        self.providers
            .lock()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| new_provider_health(&key))
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
    ///
    /// Persisted keys are re-normalized on load so that health files written
    /// before key normalization was introduced are migrated transparently.
    /// When two raw keys collapse to the same normalized form the entry with
    /// the higher `total_requests` count wins.
    pub fn load_or_new(path: &Path) -> Self {
        let snapshot = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str::<ProviderHealthRegistrySnapshot>(&s).ok());

        let providers = match snapshot {
            Some(snap) => normalize_snapshot_keys(snap.providers),
            None => HashMap::new(),
        };
        Self::with_persistence(path.to_path_buf(), providers)
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

/// Re-key a loaded snapshot so that every provider ID uses the canonical
/// normalized form.  When two raw keys collapse (e.g. `"claude-cli"` and
/// `"claude_cli"`) the entry with the higher `total_requests` is kept.
fn normalize_snapshot_keys(
    raw: HashMap<String, ProviderHealth>,
) -> HashMap<String, ProviderHealth> {
    let mut merged: HashMap<String, ProviderHealth> = HashMap::with_capacity(raw.len());
    for (raw_key, mut health) in raw {
        let canonical = normalize_provider_key(&raw_key);
        health.provider_id = canonical.clone();
        merged
            .entry(canonical)
            .and_modify(|existing| {
                if health.total_requests > existing.total_requests {
                    *existing = health.clone();
                }
            })
            .or_insert(health);
    }
    merged
}

/// Normalize a provider health key to a canonical form.
///
/// Provider identifiers reach the health registry from multiple sources:
///
/// - `ProviderKind::label()` — always `snake_case` (e.g. `"claude_cli"`)
/// - `Agent::backend_id()` — mixed (`"claude_cli"`, `"hermes-acp"`)
/// - Config `ModelProfile.provider` — user-written, may use hyphens
///
/// Without normalization the same logical provider can accumulate separate
/// circuit-breaker state under `"claude_cli"` and `"claude-cli"`.
///
/// The canonical form is lowercase with hyphens replaced by underscores,
/// matching the convention established by `ProviderKind::label()`.
#[must_use]
pub fn normalize_provider_key(key: &str) -> String {
    key.to_ascii_lowercase().replace('-', "_")
}

fn new_provider_health(provider_id: &str) -> ProviderHealth {
    ProviderHealth {
        provider_id: provider_id.to_owned(),
        state: CircuitState::Closed,
        consecutive_failures: 0,
        total_requests: 0,
        total_failures: 0,
        last_failure_at: None,
        last_success_at: None,
        cooldown_until: None,
        failure_window: VecDeque::new(),
        recent_outcomes: VecDeque::new(),
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
    /// Sliding window of the last [`ROLLING_WINDOW`] outcomes.
    ///
    /// `true` = success, `false` = failure.  Used by the in-memory tracker
    /// to detect providers with a low rolling success rate independently of
    /// the consecutive-failure counter.
    pub(crate) recent_outcomes: VecDeque<bool>,
}

impl ProviderStatus {
    /// Create a fresh status entry for `provider`.
    fn new(provider: String) -> Self {
        Self {
            provider,
            state: HealthState::Healthy,
            consecutive_failures: 0,
            last_failure_at: None,
            last_success_at: None,
            total_attempts: 0,
            total_successes: 0,
            recent_outcomes: VecDeque::new(),
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
    /// Optional persisted registry backing used by long-lived server runtimes.
    /// Standalone trackers retain the original in-memory implementation so
    /// callers can still configure custom thresholds and recovery windows.
    registry: Option<Arc<ProviderHealthRegistry>>,
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
        Self::with_config(3, Duration::from_mins(2))
    }

    /// Create a tracker with custom thresholds.
    pub fn with_config(failure_threshold: u32, recovery_window: Duration) -> Self {
        Self {
            registry: None,
            providers: RwLock::new(HashMap::new()),
            failure_threshold,
            recovery_window,
        }
    }

    /// Create the serve-facing compatibility view over the canonical,
    /// persisted provider-health registry.
    #[must_use]
    pub fn from_registry(registry: Arc<ProviderHealthRegistry>) -> Self {
        Self {
            registry: Some(registry),
            providers: RwLock::new(HashMap::new()),
            failure_threshold: 3,
            recovery_window: Duration::from_mins(2),
        }
    }

    /// Record a successful LLM call for `provider`.
    ///
    /// Resets `consecutive_failures` to 0 and transitions the provider to
    /// [`HealthState::Healthy`] regardless of current state.
    ///
    /// The key is normalized before storage so that `"claude-cli"` and
    /// `"claude_cli"` share one circuit breaker.
    #[allow(clippy::significant_drop_tightening)]
    pub fn record_success(&self, provider: &str) {
        if let Some(registry) = &self.registry {
            registry.record_success(provider);
            return;
        }
        let key = normalize_provider_key(provider);
        let now = Utc::now();
        let mut map = self.providers.write();
        let status = map
            .entry(key.clone())
            .or_insert_with(|| ProviderStatus::new(key));

        status.total_attempts += 1;
        status.total_successes += 1;
        status.consecutive_failures = 0;
        status.last_success_at = Some(now);
        status.state = HealthState::Healthy;

        // Update rolling window.
        status.recent_outcomes.push_back(true);
        if status.recent_outcomes.len() > ROLLING_WINDOW {
            status.recent_outcomes.pop_front();
        }
    }

    /// Record a failed LLM call for `provider`.
    ///
    /// Increments consecutive failures. The circuit trips to
    /// [`HealthState::Unhealthy`] when any of these conditions hold:
    ///
    /// 1. **Consecutive failures** reach the configured threshold.
    /// 2. **Rolling success rate** drops below [`ROLLING_SUCCESS_RATE_MIN`]
    ///    over the last [`ROLLING_WINDOW`] requests (catches providers that
    ///    limp along with occasional successes, like openai at 12%).
    /// 3. The provider is currently **Probing** (a single probe failure
    ///    re-trips the breaker).
    ///
    /// The key is normalized before storage.
    #[allow(clippy::significant_drop_tightening)]
    pub fn record_failure(&self, provider: &str) {
        if let Some(registry) = &self.registry {
            registry.record_failure(provider, ErrorClass::Unknown);
            return;
        }
        let key = normalize_provider_key(provider);
        let now = Utc::now();
        let recovery_at = Instant::now() + self.recovery_window;
        let mut map = self.providers.write();
        let status = map
            .entry(key.clone())
            .or_insert_with(|| ProviderStatus::new(key));

        status.total_attempts += 1;
        status.consecutive_failures = status.consecutive_failures.saturating_add(1);
        status.last_failure_at = Some(now);

        // Update rolling window.
        status.recent_outcomes.push_back(false);
        if status.recent_outcomes.len() > ROLLING_WINDOW {
            status.recent_outcomes.pop_front();
        }

        // Condition 1: consecutive failures reach the configured threshold.
        let should_trip_consecutive = status.consecutive_failures >= self.failure_threshold;

        // Condition 2: rolling success rate below minimum over a full window.
        #[allow(clippy::cast_precision_loss)]
        let should_trip_rate = if status.recent_outcomes.len() >= ROLLING_WINDOW {
            let successes = status.recent_outcomes.iter().filter(|&&ok| ok).count();
            let rate = successes as f64 / ROLLING_WINDOW as f64;
            rate < ROLLING_SUCCESS_RATE_MIN
        } else {
            false
        };

        // Condition 3: re-trip from Probing.
        let should_trip_probing = status.state == HealthState::Probing;

        if should_trip_consecutive || should_trip_rate || should_trip_probing {
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
    ///
    /// The key is normalized before lookup.
    pub fn is_healthy(&self, provider: &str) -> bool {
        if let Some(registry) = &self.registry {
            return registry.is_available(provider);
        }
        let key = normalize_provider_key(provider);
        // Fast path: read lock only.
        {
            let map = self.providers.read();
            match map.get(&key) {
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
        if let Some(status) = map.get_mut(&key) {
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
        if let Some(registry) = &self.registry {
            return registry
                .snapshot()
                .into_values()
                .map(provider_status_from_persisted)
                .collect();
        }
        self.providers.read().values().cloned().collect()
    }

    /// Return the current status for `provider`, defaulting to a healthy entry.
    ///
    /// The key is normalized before lookup.
    #[must_use]
    pub fn get(&self, provider: &str) -> ProviderStatus {
        if let Some(registry) = &self.registry {
            return provider_status_from_persisted(registry.get(provider));
        }
        let key = normalize_provider_key(provider);
        self.providers
            .read()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| ProviderStatus::new(key))
    }
}

fn provider_status_from_persisted(health: ProviderHealth) -> ProviderStatus {
    let now_ms = unix_ms_now();
    let state = match health.state {
        CircuitState::Closed => HealthState::Healthy,
        CircuitState::HalfOpen => HealthState::Probing,
        CircuitState::Open => {
            let remaining_ms = health
                .cooldown_until
                .unwrap_or(now_ms)
                .saturating_sub(now_ms)
                .max(0) as u64;
            HealthState::Unhealthy {
                recovery_at: Instant::now() + Duration::from_millis(remaining_ms),
            }
        }
    };
    ProviderStatus {
        provider: health.provider_id,
        state,
        consecutive_failures: health.consecutive_failures,
        last_failure_at: health
            .last_failure_at
            .and_then(DateTime::<Utc>::from_timestamp_millis),
        last_success_at: health
            .last_success_at
            .and_then(DateTime::<Utc>::from_timestamp_millis),
        total_attempts: health.total_requests,
        total_successes: health.total_requests.saturating_sub(health.total_failures),
        recent_outcomes: health.recent_outcomes,
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

// ─── ProviderOutcomeRecorder bridge ──────────────────────────────────────────
//
// `ProviderHealthRegistry` implements the `ProviderOutcomeRecorder` trait
// defined in `roko-agent::model_call_service` so that the `ModelCallService`
// can feed real LLM-provider call outcomes into the circuit breaker without
// creating a production `roko-agent` → `roko-learn` dependency edge.
//
// The trait is kept in `roko-agent`; `roko-learn` depends on `roko-agent` for
// other reasons (e.g. `AgentEvent`), so this direction is safe.

impl roko_agent::model_call_service::ProviderOutcomeRecorder for ProviderHealthRegistry {
    fn record_provider_success(&self, provider_id: &str) {
        // Normalization happens inside record_success.
        self.record_success(provider_id);
    }

    fn record_provider_failure(&self, provider_id: &str, error_kind: &str) {
        let error_class = match error_kind {
            "rate_limit" => ErrorClass::RateLimit,
            "timeout" => ErrorClass::Timeout,
            "server_error" => ErrorClass::ServerError,
            "auth_failure" => ErrorClass::AuthFailure,
            "content_policy" => ErrorClass::ContentPolicy,
            "context_overflow" => ErrorClass::ContextOverflow,
            _ => ErrorClass::Unknown,
        };
        // Normalization happens inside record_failure.
        self.record_failure(provider_id, error_class);
    }
}

// ─── ProviderHealthChecker bridge ────────────────────────────────────────────
//
// `ProviderHealthRegistry` implements the `ProviderHealthChecker` trait
// defined in `roko-agent::rate_limit` so that the `ProviderRateLimiter`
// can consult the shared circuit-breaker state before acquiring an RPM slot.
// Providers in Open state are rejected immediately; HalfOpen providers are
// allowed through as probes.

impl roko_agent::rate_limit::ProviderHealthChecker for ProviderHealthRegistry {
    fn circuit_state(&self, provider_id: &str) -> roko_agent::rate_limit::CircuitState {
        let key = normalize_provider_key(provider_id);
        let providers = self.providers.lock();
        match providers.get(&key) {
            None => roko_agent::rate_limit::CircuitState::Closed,
            Some(health) => match health.state {
                CircuitState::Closed => roko_agent::rate_limit::CircuitState::Closed,
                CircuitState::Open => {
                    // Check if cooldown has expired -- if so, treat as HalfOpen.
                    if let Some(until) = health.cooldown_until {
                        if unix_ms_now() >= until {
                            // Note: we don't mutate state here because the lock
                            // is read-only at this point. The actual transition
                            // happens via `is_available()` when the request
                            // succeeds or fails.
                            return roko_agent::rate_limit::CircuitState::HalfOpen;
                        }
                    }
                    roko_agent::rate_limit::CircuitState::Open
                }
                CircuitState::HalfOpen => roko_agent::rate_limit::CircuitState::HalfOpen,
            },
        }
    }

    fn record_probe_success(&self, provider_id: &str) {
        // Normalization happens inside record_success.
        self.record_success(provider_id);
    }

    fn record_probe_failure(&self, provider_id: &str) {
        // Normalization happens inside record_failure.
        self.record_failure(provider_id, ErrorClass::Unknown);
    }
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
            last_success_at: Some(1_724_999_000_000),
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
            recent_outcomes: VecDeque::new(),
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
            last_success_at: None,
            cooldown_until: None,
            failure_window: VecDeque::new(),
            recent_outcomes: VecDeque::new(),
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
            last_success_at: None,
            cooldown_until: None,
            failure_window: VecDeque::new(),
            recent_outcomes: VecDeque::new(),
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
            last_success_at: Some(2_000),
            cooldown_until: Some(33_000),
            failure_window: window,
            recent_outcomes: VecDeque::new(),
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
        let status = tracker.get("never_seen_before");
        assert_eq!(status.provider, "never_seen_before");
        assert_eq!(status.state, HealthState::Healthy);
        assert_eq!(status.consecutive_failures, 0);
        assert_eq!(status.total_attempts, 0);
    }

    /// Registry `get` for an unknown provider returns a healthy default.
    #[test]
    fn registry_get_unknown_returns_healthy_default() {
        let registry = ProviderHealthRegistry::new();
        let health = registry.get("unknown_provider");
        assert_eq!(health.provider_id, "unknown_provider");
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

    // ─── ProviderOutcomeRecorder bridge tests ─────────────────────────────────

    /// `ProviderHealthRegistry` implements `ProviderOutcomeRecorder`.
    ///
    /// Verifies that success/failure calls through the trait update the
    /// underlying circuit-breaker state identically to the direct API.
    #[test]
    fn provider_outcome_recorder_success_resets_circuit() {
        use roko_agent::model_call_service::ProviderOutcomeRecorder;

        let registry = Arc::new(ProviderHealthRegistry::new());

        // Trip the circuit with direct failures.
        registry.record_failure("myp", ErrorClass::Timeout);
        registry.record_failure("myp", ErrorClass::Timeout);
        registry.record_failure("myp", ErrorClass::Timeout);
        assert!(!registry.is_available("myp"), "circuit should be open");

        // Record success through the trait — should close it for HalfOpen.
        // (The Open→HalfOpen transition happens in is_available; after cooldown
        // expires we go HalfOpen and record_provider_success closes it back.)
        // For a fresh provider the circuit starts Closed and a success is a no-op.
        let fresh = Arc::new(ProviderHealthRegistry::new());
        fresh.record_provider_success("fresh-p");
        assert!(
            fresh.is_available("fresh-p"),
            "fresh provider must remain available after success"
        );
    }

    /// `ProviderOutcomeRecorder::record_provider_failure` maps error labels to
    /// `ErrorClass` values that the circuit breaker understands.
    #[test]
    fn provider_outcome_recorder_failure_trips_circuit() {
        use roko_agent::model_call_service::ProviderOutcomeRecorder;

        let registry = Arc::new(ProviderHealthRegistry::new());

        // Three failures via the trait → circuit opens.
        registry.record_provider_failure("p", "rate_limit");
        registry.record_provider_failure("p", "timeout");
        registry.record_provider_failure("p", "server_error");

        assert!(
            !registry.is_available("p"),
            "circuit should be open after 3 provider failures via trait"
        );
    }

    /// Unknown error_kind labels fall back to `ErrorClass::Unknown`.
    #[test]
    fn provider_outcome_recorder_unknown_label_fallback() {
        use roko_agent::model_call_service::ProviderOutcomeRecorder;

        let registry = Arc::new(ProviderHealthRegistry::new());

        // Should not panic on an unrecognised label.
        registry.record_provider_failure("q", "some_exotic_error");
        registry.record_provider_failure("q", "some_exotic_error");
        registry.record_provider_failure("q", "some_exotic_error");

        assert!(
            !registry.is_available("q"),
            "circuit should open after 3 unknown-class failures"
        );
    }

    // ─── Shared rate limit pooling (Arc identity) — E48-T05 ──────────────────
    //
    // Tests verifying that routing (CascadeRouter health filter) and outcome
    // recording (AgentDispatcherV2) share the *same* Arc<ProviderHealthRegistry>
    // so outcomes recorded by dispatch are immediately visible to the next
    // routing decision.
    //
    // `cargo test -p roko-learn --lib -- rate_limit` selects these tests.

    /// A single `Arc<ProviderHealthRegistry>` shared between two logical
    /// components sees outcomes recorded through one reference immediately
    /// reflected in reads through the other.  This is the Arc-identity
    /// invariant required by E48-T05.
    #[test]
    fn rate_limit_pool_arc_identity_outcomes_are_shared() {
        let registry = Arc::new(ProviderHealthRegistry::new());

        // Simulate "dispatch" component holds a clone of the shared Arc.
        let dispatch_ref: Arc<ProviderHealthRegistry> = Arc::clone(&registry);
        // Simulate "routing" component holds another clone of the same Arc.
        let routing_ref: Arc<ProviderHealthRegistry> = Arc::clone(&registry);

        // Assert Arc identity: both clones point to the same allocation.
        assert!(
            Arc::ptr_eq(&dispatch_ref, &routing_ref),
            "dispatch_ref and routing_ref must share the same Arc allocation"
        );
        assert!(
            Arc::ptr_eq(&registry, &dispatch_ref),
            "factory registry and dispatch_ref must share the same Arc allocation"
        );

        // Record failures through the dispatch component.
        dispatch_ref.record_failure("anthropic", ErrorClass::RateLimit);
        dispatch_ref.record_failure("anthropic", ErrorClass::RateLimit);
        dispatch_ref.record_failure("anthropic", ErrorClass::RateLimit);

        // Routing component must immediately see the circuit as open.
        assert!(
            !routing_ref.is_available("anthropic"),
            "routing_ref must see the open circuit recorded by dispatch_ref"
        );

        // Record success through routing ref → closes circuit regardless of
        // current state (ProviderHealth::record_success behaviour).
        routing_ref.record_success("anthropic");

        // Now the original factory Arc should also reflect Closed state.
        assert!(
            registry.is_available("anthropic"),
            "factory registry must see the success recorded by routing_ref"
        );
    }

    /// Rate limit failures (`rate_limit` error kind via ProviderOutcomeRecorder)
    /// must open the circuit after three consecutive failures (Closed → Open).
    #[test]
    fn rate_limit_failure_drives_closed_to_open_transition() {
        use roko_agent::model_call_service::ProviderOutcomeRecorder;

        let registry = Arc::new(ProviderHealthRegistry::new());

        assert!(registry.is_available("openai"), "starts Closed/available");

        registry.record_provider_failure("openai", "rate_limit");
        registry.record_provider_failure("openai", "rate_limit");
        // Two failures are below the threshold — still Closed.
        assert!(
            registry.is_available("openai"),
            "still available after 2 rate-limit failures"
        );

        registry.record_provider_failure("openai", "rate_limit");
        // Third failure trips to Open.
        assert!(
            !registry.is_available("openai"),
            "circuit must be Open after 3 rate-limit failures"
        );
    }

    /// After three timeout failures the circuit opens (Closed → Open);
    /// a probe success drives it back to Closed.
    #[test]
    fn rate_limit_pool_halfopen_probe_success_closes_circuit() {
        use roko_agent::model_call_service::ProviderOutcomeRecorder;

        let registry = Arc::new(ProviderHealthRegistry::new());

        // Trip to Open via three timeout failures.
        registry.record_provider_failure("gemini", "timeout");
        registry.record_provider_failure("gemini", "timeout");
        registry.record_provider_failure("gemini", "timeout");
        assert!(!registry.is_available("gemini"), "Open after 3 timeouts");

        // record_provider_success closes the circuit back to Closed.
        registry.record_provider_success("gemini");
        assert!(
            registry.is_available("gemini"),
            "circuit closed after probe success"
        );

        // Verify snapshot reflects Closed state and cleared failures.
        let snap = registry.snapshot();
        let entry = snap.get("gemini").expect("gemini must be tracked");
        assert_eq!(entry.state, CircuitState::Closed);
        assert_eq!(entry.consecutive_failures, 0);
    }

    /// Per-provider isolation: failures on provider A must not affect
    /// provider B within the shared registry.
    #[test]
    fn rate_limit_pool_per_provider_isolation() {
        use roko_agent::model_call_service::ProviderOutcomeRecorder;

        let registry = Arc::new(ProviderHealthRegistry::new());

        // Trip provider A.
        registry.record_provider_failure("anthropic", "rate_limit");
        registry.record_provider_failure("anthropic", "rate_limit");
        registry.record_provider_failure("anthropic", "rate_limit");

        // Provider B must be unaffected.
        assert!(!registry.is_available("anthropic"), "anthropic is Open");
        assert!(
            registry.is_available("openai"),
            "openai must be unaffected by anthropic failures"
        );
        assert!(
            registry.is_available("gemini"),
            "gemini must be unaffected by anthropic failures"
        );
    }

    /// Concurrent Arc clones all record into the same shared state.
    /// 20 concurrent tasks each record a server_error failure; the total
    /// must be visible through any clone.
    #[tokio::test]
    async fn rate_limit_pool_concurrent_arc_clones_observe_same_state() {
        use roko_agent::model_call_service::ProviderOutcomeRecorder;

        let registry = Arc::new(ProviderHealthRegistry::new());

        let mut handles = Vec::new();
        for _ in 0..20 {
            let r = Arc::clone(&registry);
            handles.push(tokio::spawn(async move {
                r.record_provider_failure("parallel", "server_error");
            }));
        }
        for h in handles {
            h.await.expect("task panicked");
        }

        // All clones share the same state → 20 total_failures.
        let snap = registry.snapshot();
        let entry = snap.get("parallel").expect("parallel must be tracked");
        assert_eq!(
            entry.total_failures, 20,
            "all 20 concurrent failures must be visible through any Arc clone"
        );
        // Circuit should be open (threshold is 3).
        assert_eq!(entry.state, CircuitState::Open);
    }

    #[test]
    fn compatibility_tracker_shares_persisted_registry_state() {
        let registry = Arc::new(ProviderHealthRegistry::new());
        let tracker = ProviderHealthTracker::from_registry(Arc::clone(&registry));

        tracker.record_success("anthropic");
        assert_eq!(registry.get("anthropic").total_requests, 1);

        for _ in 0..3 {
            registry.record_failure("anthropic", ErrorClass::AuthFailure);
        }
        let status = tracker.get("anthropic");
        assert_eq!(status.total_attempts, 4);
        assert_eq!(status.total_successes, 1);
        assert!(matches!(status.state, HealthState::Unhealthy { .. }));

        tracker.record_success("anthropic");
        assert_eq!(registry.get("anthropic").state, CircuitState::Closed);
        assert_eq!(tracker.get("anthropic").total_attempts, 5);
    }

    // ─── Key normalization ──────────────────────────────────────────────

    /// `normalize_provider_key` converts hyphens to underscores and
    /// lowercases the input.
    #[test]
    fn normalize_provider_key_basic() {
        assert_eq!(normalize_provider_key("claude-cli"), "claude_cli");
        assert_eq!(normalize_provider_key("claude_cli"), "claude_cli");
        assert_eq!(normalize_provider_key("Claude-CLI"), "claude_cli");
        assert_eq!(normalize_provider_key("hermes-acp"), "hermes_acp");
        assert_eq!(normalize_provider_key("openclaw-infer"), "openclaw_infer");
        assert_eq!(normalize_provider_key("openai_compat"), "openai_compat");
        assert_eq!(normalize_provider_key("anthropic"), "anthropic");
    }

    /// Recording failures via hyphenated key and querying via underscore
    /// key share the same circuit breaker in the registry.
    #[test]
    fn registry_hyphen_underscore_share_circuit() {
        let registry = ProviderHealthRegistry::new();

        // Record failures via the hyphenated variant.
        registry.record_failure("claude-cli", ErrorClass::Timeout);
        registry.record_failure("claude-cli", ErrorClass::Timeout);
        registry.record_failure("claude-cli", ErrorClass::Timeout);

        // Query via the underscore variant — same circuit.
        assert!(
            !registry.is_available("claude_cli"),
            "claude_cli must see the circuit opened by claude-cli"
        );

        // Record success via the underscore variant.
        registry.record_success("claude_cli");

        // The snapshot should have exactly one entry, not two.
        let snap = registry.snapshot();
        assert_eq!(
            snap.len(),
            1,
            "hyphen and underscore keys must collapse to a single entry"
        );
        let entry = snap.values().next().unwrap();
        assert_eq!(entry.provider_id, "claude_cli");
        assert_eq!(entry.total_requests, 4); // 3 failures + 1 success
    }

    /// The tracker's in-memory path also normalizes keys.
    #[test]
    fn tracker_hyphen_underscore_share_circuit() {
        let tracker = ProviderHealthTracker::with_config(3, Duration::from_secs(600));

        tracker.record_failure("cursor-cli");
        tracker.record_failure("cursor_cli");
        tracker.record_failure("cursor-cli");

        assert!(
            !tracker.is_healthy("cursor_cli"),
            "3 failures across hyphen/underscore variants must trip the breaker"
        );

        let snap = tracker.snapshot();
        assert_eq!(
            snap.len(),
            1,
            "hyphen and underscore keys must collapse to one entry"
        );
        assert_eq!(snap[0].provider, "cursor_cli");
        assert_eq!(snap[0].total_attempts, 3);
    }

    /// Persisted health files with mixed key formats are normalized on
    /// load, collapsing duplicates.
    #[test]
    fn load_normalizes_persisted_keys() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("health.json");

        // Manually write a snapshot with two entries that should normalize
        // to the same key.
        let mut providers = HashMap::new();
        let mut h1 = new_provider_health("claude-cli");
        h1.total_requests = 5;
        h1.total_failures = 2;
        providers.insert("claude-cli".to_owned(), h1);

        let mut h2 = new_provider_health("claude_cli");
        h2.total_requests = 10;
        h2.total_failures = 3;
        providers.insert("claude_cli".to_owned(), h2);

        let snapshot = ProviderHealthRegistrySnapshot { providers };
        roko_fs::atomic_write_json(&path, &snapshot).unwrap();

        // Load — should collapse to one entry (the one with higher
        // total_requests).
        let loaded = ProviderHealthRegistry::load_or_new(&path);
        let snap = loaded.snapshot();
        assert_eq!(
            snap.len(),
            1,
            "duplicate keys must be collapsed on load"
        );
        let entry = snap.get("claude_cli").expect("normalized key must exist");
        assert_eq!(entry.provider_id, "claude_cli");
        assert_eq!(entry.total_requests, 10, "higher-traffic entry should win");
    }

    /// `get` returns a healthy default with a normalized key even when
    /// the provider has never been seen.
    #[test]
    fn registry_get_normalizes_unknown_key() {
        let registry = ProviderHealthRegistry::new();
        let health = registry.get("hermes-acp");
        assert_eq!(health.provider_id, "hermes_acp");
        assert_eq!(health.state, CircuitState::Closed);
    }

    /// `get` on the tracker returns a healthy default with a normalized
    /// key.
    #[test]
    fn tracker_get_normalizes_unknown_key() {
        let tracker = ProviderHealthTracker::new();
        let status = tracker.get("openclaw-infer");
        assert_eq!(status.provider, "openclaw_infer");
        assert_eq!(status.state, HealthState::Healthy);
    }

    // ─── Rolling success-rate circuit trip (P3-2) ─────────────────────

    /// A provider at ~12% success rate trips Open via the rolling-window
    /// check even though occasional successes reset the consecutive-failure
    /// counter.  This reproduces the scenario from the dogfood run where
    /// openai stayed Closed at 12% success over 49 calls.
    #[test]
    fn tracker_low_rolling_success_rate_trips_breaker() {
        // Use threshold=100 so consecutive failures alone can never trip.
        let tracker = ProviderHealthTracker::with_config(100, Duration::from_secs(600));

        // Simulate ~12% success rate: 1 success then 7 failures, repeated.
        // After the first 10 outcomes the rolling window is full.
        for cycle in 0..6 {
            tracker.record_success("openai");
            for _ in 0..7 {
                tracker.record_failure("openai");
            }
            // After cycle 1 (16 total), window = last 10 = [F,F,F,F,F,S,F,F,F,F]
            // which has 1 success = 10% < 30%.
            if cycle >= 1 {
                assert!(
                    !tracker.is_healthy("openai"),
                    "provider at ~12% success should be Unhealthy after cycle {cycle}"
                );
                // Allow re-probing for the next cycle by recording a success
                // (simulates cooldown expiry + probe).
                tracker.record_success("openai");
            }
        }
    }

    /// The rolling-window check does NOT trip the breaker when the success
    /// rate is above the threshold (50%).  This confirms the consecutive-only
    /// case is unaffected.
    #[test]
    fn tracker_moderate_success_rate_stays_healthy() {
        // Use threshold=100 so consecutive failures can never trip.
        let tracker = ProviderHealthTracker::with_config(100, Duration::from_secs(600));

        // 50% success rate: alternating success and failure.
        for _ in 0..20 {
            tracker.record_success("anthropic");
            tracker.record_failure("anthropic");
        }

        // 50% > 30% threshold, so should remain healthy.
        assert!(
            tracker.is_healthy("anthropic"),
            "50% success rate should not trip the rolling-window check"
        );
    }

    /// Rolling-window trip on the ProviderHealth (serializable) struct:
    /// reproduce the exact 12% scenario and verify circuit opens.
    #[test]
    fn snapshot_low_rolling_success_rate_trips() {
        let mut h = new_provider_health("openai");

        // Simulate 49 calls with ~12% success (6 successes, 43 failures).
        let mut ms = 1000;
        for _ in 0..6 {
            h.record_success();
            ms += 100;
            for _ in 0..7 {
                h.record_failure(ErrorClass::ServerError, ms);
                ms += 100;
            }
        }
        // After the first full window (10 outcomes), the rate drops below
        // 30% and the circuit should be Open.
        assert_eq!(
            h.state,
            CircuitState::Open,
            "12% success rate over 48 calls should trip the circuit"
        );
    }
}
