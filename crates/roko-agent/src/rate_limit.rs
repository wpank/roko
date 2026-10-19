//! Per-provider request throttling for HTTP-backed agents.
//!
//! This limiter is keyed by provider ID so concurrent tasks can share a single
//! client-side rate budget for each upstream provider.
//!
//! ## Per-provider limits
//!
//! Construct with [`ProviderRateLimiter::new`] for a uniform default RPM, or
//! use [`ProviderRateLimiter::with_provider_limits`] to supply per-provider
//! RPM and TPM budgets read from [`roko_core::config::provider::ProviderConfig`].
//!
//! TPM tracking uses a sliding-window token counter (`TpmTracker`) independent
//! of governor (which does not natively support token-weighted quotas). Hard
//! TPM blocking is advisory: it delays the caller and emits a warning instead
//! of returning an error, because token budgets are frequently generous and a
//! hard block would stall unrelated providers sharing the same runtime.
//!
//! ## Health-aware acquisition
//!
//! When a [`ProviderHealthChecker`] is attached via
//! [`ProviderRateLimiter::with_health_registry`], the [`try_acquire`] method
//! consults the shared circuit-breaker state before waiting for an RPM slot.
//! Providers in `Open` circuit state are rejected immediately; providers in
//! `HalfOpen` state are allowed through as probe requests.
//!
//! [`try_acquire`]: ProviderRateLimiter::try_acquire
//!
//! ## Calls at once, learned from rate limits
//!
//! A provider that refuses a call with a rate limit (HTTP 429) while others
//! are in flight takes no more calls at once than those others.
//! [`ProviderRateLimiter::admit`] then holds back a call to it while that
//! many are in flight, so parallel tasks queue for the provider instead of
//! all calling it and being refused again (bug-4b6844). The limit holds for
//! [`LEARNED_LIMIT_HOLDS_FOR`] after the last refusal.

use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed, keyed::DefaultKeyedStateStore},
};

#[cfg(test)]
use std::time::Instant;

// ─── Health-aware circuit-breaker integration ──────────────────────────────

/// Circuit-breaker state for a provider, mirroring the three-state machine
/// in `roko_learn::provider_health::CircuitState`.
///
/// Defined here so `roko-agent` does not depend on `roko-learn` in production.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation -- requests proceed.
    Closed,
    /// Provider is unhealthy -- requests should be rejected immediately.
    Open,
    /// Cooldown expired -- one probe request is allowed.
    HalfOpen,
}

/// Dependency-safe trait for checking provider circuit-breaker state.
///
/// `roko_learn::provider_health::ProviderHealthRegistry` implements this trait
/// so the rate limiter can consult the shared circuit breaker without creating
/// a production `roko-agent` -> `roko-learn` dependency edge.
pub trait ProviderHealthChecker: Send + Sync {
    /// Return the current circuit state for `provider_id`.
    ///
    /// Unknown providers should return [`CircuitState::Closed`].
    fn circuit_state(&self, provider_id: &str) -> CircuitState;

    /// Record that a probe request (made during [`CircuitState::HalfOpen`])
    /// succeeded, transitioning the provider back to [`CircuitState::Closed`].
    fn record_probe_success(&self, provider_id: &str);

    /// Record that a probe request failed, resetting the provider to
    /// [`CircuitState::Open`] with a fresh cooldown.
    fn record_probe_failure(&self, provider_id: &str);
}

/// Outcome of a successful [`ProviderRateLimiter::try_acquire`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireOutcome {
    /// Provider is healthy; request may proceed normally.
    Healthy,
    /// Provider is in half-open (probing) state; caller should track the
    /// outcome and report back via [`ProviderHealthChecker::record_probe_success`]
    /// or [`ProviderHealthChecker::record_probe_failure`].
    Probing,
}

/// Error returned by [`ProviderRateLimiter::try_acquire`] when the provider's
/// circuit breaker is in [`CircuitState::Open`].
#[derive(Debug, Clone)]
pub struct RateLimitError {
    /// The provider whose circuit is open.
    pub provider_id: String,
}

impl fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "provider '{}' circuit is open -- rate limited",
            self.provider_id
        )
    }
}

impl std::error::Error for RateLimitError {}

/// Per-provider request and token limits loaded from `ProviderConfig.limits`.
#[derive(Clone, Debug)]
pub struct ProviderLimits {
    /// Maximum requests per minute.
    pub rpm: u32,
    /// Maximum tokens per minute (input + output). Zero means unlimited.
    pub tpm: u64,
}

/// Operational snapshot of one configured provider's rolling limits.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ProviderRateLimitSnapshot {
    /// Provider configuration key.
    pub provider_id: String,
    /// Requests observed in the current 60-second window.
    pub rpm_used: u64,
    /// Configured requests-per-minute limit.
    pub rpm_limit: u32,
    /// Tokens observed in the current 60-second window.
    pub tpm_used: u64,
    /// Configured tokens-per-minute limit; zero means unlimited.
    pub tpm_limit: u64,
}

impl From<roko_core::config::provider::ProviderLimits> for ProviderLimits {
    fn from(c: roko_core::config::provider::ProviderLimits) -> Self {
        Self {
            rpm: c.rpm,
            tpm: c.tpm,
        }
    }
}

/// Sliding-window token-per-minute tracker.
///
/// Uses a 60-second window divided into 6 buckets of 10 seconds each.
/// The current token count is the sum of all unexpired buckets.
#[derive(Debug)]
struct TpmTracker {
    /// Ring buffer of (bucket_start_secs, token_count) pairs.
    buckets: Mutex<Vec<(u64, u64)>>,
    /// Rolling token total (approximate, for quick advisory checks).
    rolling_total: AtomicU64,
}

impl TpmTracker {
    fn new() -> Self {
        Self {
            buckets: Mutex::new(Vec::with_capacity(6)),
            rolling_total: AtomicU64::new(0),
        }
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Record `tokens` consumed and return the new rolling total for this
    /// 60-second window.
    fn add(&self, tokens: u64) -> u64 {
        let now = Self::now_secs();
        let window_start = now.saturating_sub(60);
        let bucket_key = (now / 10) * 10; // align to 10-second buckets

        let mut buckets = self.buckets.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("tpm tracker buckets mutex poisoned; recovering");
            poisoned.into_inner()
        });
        // Evict buckets older than the 60-second window.
        buckets.retain(|(start, _)| *start >= window_start);
        // Add tokens to the current bucket.
        if let Some((_, count)) = buckets.iter_mut().find(|(s, _)| *s == bucket_key) {
            *count += tokens;
        } else {
            buckets.push((bucket_key, tokens));
        }
        let total: u64 = buckets.iter().map(|(_, c)| c).sum();
        self.rolling_total.store(total, Ordering::Relaxed);
        total
    }

    /// Return the approximate rolling TPM without updating state.
    fn current(&self) -> u64 {
        let now = Self::now_secs();
        let window_start = now.saturating_sub(60);
        let buckets = self.buckets.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("tpm tracker buckets mutex poisoned; recovering");
            poisoned.into_inner()
        });
        buckets
            .iter()
            .filter(|(s, _)| *s >= window_start)
            .map(|(_, c)| c)
            .sum()
    }
}

/// Per-provider RPM state stored alongside the TPM tracker.
struct ProviderState {
    /// Governor rate limiter using per-provider RPM.
    rpm_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    /// Configured RPM budget.
    rpm_limit: u32,
    /// Rolling request counter for operational reporting.
    rpm_tracker: Arc<TpmTracker>,
    /// TPM budget (0 = unlimited).
    tpm_limit: u64,
    /// Rolling token counter.
    tpm_tracker: Arc<TpmTracker>,
}

/// Async keyed rate limiter using per-provider RPM and TPM budgets.
///
/// Each provider keyed by its config ID gets its own RPM governor slot.
/// Providers without explicit config entries share the `default_rpm` slot.
///
/// When a [`ProviderHealthChecker`] is attached via [`with_health_registry`],
/// [`try_acquire`] checks the provider's circuit-breaker state before waiting
/// for an RPM slot. Providers in `Open` state are rejected immediately.
///
/// [`with_health_registry`]: ProviderRateLimiter::with_health_registry
/// [`try_acquire`]: ProviderRateLimiter::try_acquire
pub struct ProviderRateLimiter {
    /// Default RPM used when no per-provider config is available.
    default_rpm: NonZeroU32,
    /// Per-provider state (keyed RPM limiter + TPM tracker).
    providers: Mutex<HashMap<String, ProviderState>>,
    /// Shared fallback RPM limiter used for unknown providers.
    default_limiter: RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>,
    /// Optional shared circuit-breaker state consulted by [`try_acquire`].
    ///
    /// When set, providers whose circuit is [`CircuitState::Open`] are
    /// rejected immediately without consuming an RPM slot.
    ///
    /// [`try_acquire`]: ProviderRateLimiter::try_acquire
    shared_health: Option<Arc<dyn ProviderHealthChecker>>,
    /// What each endpoint's rate limits taught about the calls it takes at
    /// once ([`ProviderRateLimiter::admit`]).
    learned: Arc<Mutex<HashMap<String, LearnedConcurrency>>>,
}

impl std::fmt::Debug for ProviderRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRateLimiter")
            .field("default_rpm", &self.default_rpm)
            .field("has_health_checker", &self.shared_health.is_some())
            .finish()
    }
}

impl ProviderRateLimiter {
    /// Construct a keyed limiter with a uniform default RPM budget.
    ///
    /// All providers share the same RPM slot via the keyed default limiter.
    /// Use [`with_provider_limits`] to configure per-provider budgets.
    #[must_use]
    pub fn new(default_rpm: u32) -> Self {
        let default_rpm = NonZeroU32::new(default_rpm)
            .unwrap_or_else(|| NonZeroU32::new(60).expect("default RPM must be non-zero"));
        Self {
            default_rpm,
            providers: Mutex::new(HashMap::new()),
            default_limiter: RateLimiter::keyed(Quota::per_minute(default_rpm)),
            shared_health: None,
            learned: Arc::default(),
        }
    }

    /// Construct from a map of per-provider limits.
    ///
    /// Called at runtime construction so each concurrent agent shares one
    /// pool with the configured budgets.
    #[must_use]
    pub fn with_provider_limits(default_rpm: u32, limits: HashMap<String, ProviderLimits>) -> Self {
        let default_rpm = NonZeroU32::new(default_rpm)
            .unwrap_or_else(|| NonZeroU32::new(60).expect("default RPM must be non-zero"));

        let mut providers = HashMap::with_capacity(limits.len());
        for (provider_id, limit) in limits {
            let provider_rpm = NonZeroU32::new(limit.rpm).unwrap_or(default_rpm);
            let rpm_quota = Quota::per_minute(provider_rpm);
            let rpm_limiter = Arc::new(RateLimiter::direct(rpm_quota));
            providers.insert(
                provider_id,
                ProviderState {
                    rpm_limiter,
                    rpm_limit: provider_rpm.get(),
                    rpm_tracker: Arc::new(TpmTracker::new()),
                    tpm_limit: limit.tpm,
                    tpm_tracker: Arc::new(TpmTracker::new()),
                },
            );
        }

        Self {
            default_rpm,
            providers: Mutex::new(providers),
            default_limiter: RateLimiter::keyed(Quota::per_minute(default_rpm)),
            shared_health: None,
            learned: Arc::default(),
        }
    }

    /// Build from an iterator of `(provider_id, ProviderConfig)` pairs.
    ///
    /// Providers without a `limits` field receive the default RPM budget and
    /// an unlimited TPM budget, so they still appear in operational snapshots.
    ///
    /// Works with both `HashMap` and `IndexMap` via `.iter()`:
    ///
    /// ```rust,ignore
    /// let limiter = ProviderRateLimiter::from_provider_configs(
    ///     60,
    ///     config.effective_providers().iter(),
    /// );
    /// ```
    #[must_use]
    pub fn from_provider_configs<'a, I>(default_rpm: u32, configs: I) -> Self
    where
        I: Iterator<Item = (&'a String, &'a roko_core::config::provider::ProviderConfig)>,
    {
        let limits: HashMap<String, ProviderLimits> = configs
            .map(|(id, cfg)| {
                let limits = cfg.limits.as_ref().map_or(
                    ProviderLimits {
                        rpm: default_rpm,
                        tpm: 0,
                    },
                    |limits| ProviderLimits::from(limits.clone()),
                );
                (id.clone(), limits)
            })
            .collect();
        Self::with_provider_limits(default_rpm, limits)
    }

    /// Attach a shared provider health checker (circuit-breaker registry).
    ///
    /// When set, [`try_acquire`] consults the checker before waiting for an
    /// RPM slot. Providers in `Open` circuit state are rejected immediately;
    /// providers in `HalfOpen` state are allowed through as probe requests.
    ///
    /// The intended implementation is `ProviderHealthRegistry` from
    /// `roko-learn`, passed via `Arc<ProviderHealthRegistry>` at runtime
    /// construction.
    ///
    /// [`try_acquire`]: ProviderRateLimiter::try_acquire
    #[must_use]
    pub fn with_health_registry(mut self, registry: Arc<dyn ProviderHealthChecker>) -> Self {
        self.shared_health = Some(registry);
        self
    }

    /// Set the health checker on an existing limiter (non-builder variant).
    ///
    /// Useful when the limiter is already constructed and shared via `Arc`.
    pub fn set_health_registry(&mut self, registry: Arc<dyn ProviderHealthChecker>) {
        self.shared_health = Some(registry);
    }

    /// Return a reference to the attached health checker, if any.
    #[must_use]
    pub fn health_checker(&self) -> Option<&dyn ProviderHealthChecker> {
        self.shared_health.as_deref()
    }

    /// Wait until the next request for `provider_id` can proceed.
    ///
    /// If the provider has a configured RPM limit, acquires a slot from its
    /// dedicated governor. Otherwise falls back to the shared keyed limiter
    /// with the default RPM.
    pub async fn acquire(&self, provider_id: &str) {
        // Check for a dedicated per-provider governor first (no async inside lock).
        let dedicated = {
            let providers = self.providers.lock().unwrap_or_else(|poisoned| {
                tracing::warn!("rate limiter providers mutex poisoned; recovering");
                poisoned.into_inner()
            });
            providers.get(provider_id).map(|state| {
                (
                    Arc::clone(&state.rpm_limiter),
                    Arc::clone(&state.rpm_tracker),
                )
            })
        };

        if let Some((limiter, tracker)) = dedicated {
            limiter.until_ready().await;
            tracker.add(1);
        } else {
            self.default_limiter
                .until_key_ready(&provider_id.to_string())
                .await;
        }
    }

    /// Health-aware acquisition: check circuit-breaker state, then acquire an
    /// RPM slot.
    ///
    /// When a [`ProviderHealthChecker`] is attached:
    ///
    /// - **`Open`** circuit: returns [`RateLimitError`] immediately without
    ///   consuming an RPM slot.
    /// - **`HalfOpen`** circuit: acquires the RPM slot and returns
    ///   [`AcquireOutcome::Probing`] so the caller can track the probe outcome.
    /// - **`Closed`** circuit (or no health checker): acquires the RPM slot
    ///   and returns [`AcquireOutcome::Healthy`].
    ///
    /// # Errors
    ///
    /// Returns [`RateLimitError`] when the provider's circuit is open.
    pub async fn try_acquire(&self, provider_id: &str) -> Result<AcquireOutcome, RateLimitError> {
        // Check circuit-breaker state before consuming an RPM slot.
        let outcome = if let Some(ref checker) = self.shared_health {
            match checker.circuit_state(provider_id) {
                CircuitState::Open => {
                    tracing::warn!(
                        provider = provider_id,
                        "provider circuit is open -- rejecting request"
                    );
                    return Err(RateLimitError {
                        provider_id: provider_id.to_owned(),
                    });
                }
                CircuitState::HalfOpen => {
                    tracing::info!(
                        provider = provider_id,
                        "provider circuit is half-open -- allowing probe request"
                    );
                    AcquireOutcome::Probing
                }
                CircuitState::Closed => AcquireOutcome::Healthy,
            }
        } else {
            AcquireOutcome::Healthy
        };

        // RPM slot acquisition (same logic as `acquire`).
        self.acquire(provider_id).await;
        Ok(outcome)
    }

    /// Record token consumption and check advisory TPM limits.
    ///
    /// Returns the current rolling TPM after recording. When the TPM limit is
    /// set and would be exceeded, emits a warning and applies a brief back-off
    /// delay to avoid hammering the provider. This is advisory -- it does not
    /// return an error because TPM exhaustion is transient and often brief.
    pub async fn record_tokens(&self, provider_id: &str, tokens: u64) -> u64 {
        let (tracker, tpm_limit) = {
            let providers = self.providers.lock().unwrap_or_else(|poisoned| {
                tracing::warn!("rate limiter providers mutex poisoned; recovering");
                poisoned.into_inner()
            });
            if let Some(state) = providers.get(provider_id) {
                (Some(Arc::clone(&state.tpm_tracker)), state.tpm_limit)
            } else {
                (None, 0)
            }
        };

        let Some(tracker) = tracker else {
            return 0;
        };

        let current_tpm = tracker.add(tokens);

        if tpm_limit > 0 && current_tpm >= tpm_limit {
            let pct = (current_tpm * 100) / tpm_limit;
            tracing::warn!(
                provider = provider_id,
                current_tpm,
                tpm_limit,
                pct,
                "TPM budget approaching/exceeded -- applying brief back-off"
            );
            // Brief advisory delay: 1 second. A follow-up acquire() call will
            // apply the RPM governor's fuller wait.
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        current_tpm
    }

    /// Return the approximate current rolling TPM for a provider.
    #[must_use]
    pub fn current_tpm(&self, provider_id: &str) -> u64 {
        let providers = self.providers.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("rate limiter providers mutex poisoned; recovering");
            poisoned.into_inner()
        });
        providers
            .get(provider_id)
            .map(|s| s.tpm_tracker.current())
            .unwrap_or(0)
    }

    /// Return rolling RPM/TPM utilization for every configured provider.
    /// Providers without explicit limits report the runtime's default RPM
    /// budget and an unlimited (`0`) TPM budget.
    #[must_use]
    pub fn snapshot(&self) -> Vec<ProviderRateLimitSnapshot> {
        let providers = self.providers.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("rate limiter providers mutex poisoned; recovering");
            poisoned.into_inner()
        });
        let mut snapshots = providers
            .iter()
            .map(|(provider_id, state)| ProviderRateLimitSnapshot {
                provider_id: provider_id.clone(),
                rpm_used: state.rpm_tracker.current(),
                rpm_limit: state.rpm_limit,
                tpm_used: state.tpm_tracker.current(),
                tpm_limit: state.tpm_limit,
            })
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));
        snapshots
    }

    /// Admit one call to the provider at `endpoint`, or hold it back while
    /// the provider is at the calls at once it showed it takes
    /// ([`Self::note_rate_limited`]). The call counts as in flight until the
    /// returned [`AdmittedCall`] drops.
    ///
    /// # Errors
    ///
    /// Returns [`HeldCall`] when the endpoint is at its learned limit.
    pub fn admit(&self, endpoint: &str) -> Result<AdmittedCall, HeldCall> {
        self.admit_at(endpoint, std::time::Instant::now())
    }

    fn admit_at(&self, endpoint: &str, now: std::time::Instant) -> Result<AdmittedCall, HeldCall> {
        let mut learned = lock_learned(&self.learned);
        let state = learned.entry(endpoint.to_string()).or_default();
        if let Some((limit, at)) = state.limit {
            if now.saturating_duration_since(at) >= LEARNED_LIMIT_HOLDS_FOR {
                state.limit = None;
            } else if state.in_flight >= limit {
                return Err(HeldCall {
                    endpoint: endpoint.to_string(),
                    in_flight: state.in_flight,
                    limit,
                });
            }
        }
        state.in_flight += 1;
        Ok(AdmittedCall {
            learned: Arc::clone(&self.learned),
            endpoint: endpoint.to_string(),
        })
    }

    /// The provider at `endpoint` refused a call this limiter admitted with a
    /// rate limit (HTTP 429): it takes no more calls at once than the others
    /// in flight now, and at least one, until [`LEARNED_LIMIT_HOLDS_FOR`]
    /// passes without another refusal.
    pub fn note_rate_limited(&self, endpoint: &str) {
        self.note_rate_limited_at(endpoint, std::time::Instant::now());
    }

    fn note_rate_limited_at(&self, endpoint: &str, now: std::time::Instant) {
        let mut learned = lock_learned(&self.learned);
        let state = learned.entry(endpoint.to_string()).or_default();
        // The refused call itself is still in flight.
        let takes = state.in_flight.saturating_sub(1).max(1);
        let limit = state.limit.map_or(takes, |(limit, _)| limit.min(takes));
        state.limit = Some((limit, now));
    }

    #[cfg(test)]
    pub(crate) fn new_per_second(rps: u32) -> Self {
        // For test-only per-second limiting, use the default keyed path.
        let rps = NonZeroU32::new(rps).expect("test RPS must be non-zero");
        Self {
            default_rpm: NonZeroU32::new(rps.get().saturating_mul(60).max(1))
                .unwrap_or(NonZeroU32::new(60).unwrap()),
            providers: Mutex::new(HashMap::new()),
            default_limiter: RateLimiter::keyed(Quota::per_second(rps)),
            shared_health: None,
            learned: Arc::default(),
        }
    }
}

// ─── Calls at once, learned from rate limits (bug-4b6844) ─────────────────

/// How long the calls-at-once limit a provider showed by refusing a call
/// with a rate limit holds without another refusal. Calls then go out
/// unlimited again, and the next refusal teaches the limit afresh.
pub const LEARNED_LIMIT_HOLDS_FOR: std::time::Duration = std::time::Duration::from_secs(60);

/// What one endpoint's rate limits taught about the calls it takes at once.
#[derive(Debug, Default)]
struct LearnedConcurrency {
    /// Calls to it in flight, as [`ProviderRateLimiter::admit`] admitted them.
    in_flight: usize,
    /// The calls at once it takes, and when a refusal last showed it.
    limit: Option<(usize, std::time::Instant)>,
}

fn lock_learned(
    learned: &Mutex<HashMap<String, LearnedConcurrency>>,
) -> std::sync::MutexGuard<'_, HashMap<String, LearnedConcurrency>> {
    learned.lock().unwrap_or_else(|poisoned| {
        tracing::warn!("rate limiter learned-concurrency mutex poisoned; recovering");
        poisoned.into_inner()
    })
}

/// A call [`ProviderRateLimiter::admit`] let through: in flight until it
/// drops.
#[derive(Debug)]
pub struct AdmittedCall {
    learned: Arc<Mutex<HashMap<String, LearnedConcurrency>>>,
    endpoint: String,
}

impl Drop for AdmittedCall {
    fn drop(&mut self) {
        if let Some(state) = lock_learned(&self.learned).get_mut(&self.endpoint) {
            state.in_flight = state.in_flight.saturating_sub(1);
        }
    }
}

/// A call [`ProviderRateLimiter::admit`] held back: its provider is at the
/// calls at once it showed it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldCall {
    /// The provider's endpoint.
    pub endpoint: String,
    /// Its calls in flight.
    pub in_flight: usize,
    /// The calls at once it takes.
    pub limit: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bug-4b6844: a provider that refused one of three calls at once takes
    /// two. While two are in flight a third is held back, without calling it;
    /// once one ends, the next goes out; a minute after the last refusal,
    /// calls go out unlimited again.
    #[test]
    fn a_rate_limit_teaches_how_many_calls_at_once_a_provider_takes() {
        let limiter = ProviderRateLimiter::new(60);
        let endpoint = "https://api.z.ai/api/paas/v4/chat/completions";
        let t0 = std::time::Instant::now();
        let first = limiter.admit_at(endpoint, t0).expect("no limit yet");
        let second = limiter.admit_at(endpoint, t0).expect("no limit yet");
        let third = limiter.admit_at(endpoint, t0).expect("no limit yet");

        limiter.note_rate_limited_at(endpoint, t0);
        drop(third);
        let held = limiter
            .admit_at(endpoint, t0 + std::time::Duration::from_secs(1))
            .expect_err("two in flight is the limit");
        assert_eq!((held.in_flight, held.limit), (2, 2));
        // Another endpoint is not held.
        let other = limiter
            .admit_at("https://api.openai.com/v1/chat/completions", t0)
            .expect("another endpoint");

        drop(first);
        let next = limiter
            .admit_at(endpoint, t0 + std::time::Duration::from_secs(2))
            .expect("one call ended");
        assert!(
            limiter
                .admit_at(endpoint, t0 + std::time::Duration::from_secs(3))
                .is_err()
        );
        let later = t0 + LEARNED_LIMIT_HOLDS_FOR;
        let unlimited = limiter.admit_at(endpoint, later).expect("the limit lapsed");
        let again = limiter.admit_at(endpoint, later).expect("still unlimited");
        drop((second, other, next, unlimited, again));

        // A refusal with nothing else in flight still lets one call through.
        limiter.note_rate_limited_at(endpoint, later);
        let only = limiter.admit_at(endpoint, later).expect("one at a time");
        assert!(limiter.admit_at(endpoint, later).is_err());
        drop(only);
    }
    use tokio::join;

    #[tokio::test]
    async fn provider_rate_limiter_uses_default_rpm_when_zero() {
        let limiter = ProviderRateLimiter::new(0);
        let start = Instant::now();

        limiter.acquire("zai").await;
        limiter.acquire("zai").await;

        assert!(
            start.elapsed() < std::time::Duration::from_secs(2),
            "fallback limiter should not block early requests"
        );
    }

    #[tokio::test]
    async fn provider_rate_limiter_spreads_rapid_requests_for_same_provider() {
        let limiter = ProviderRateLimiter::new_per_second(5);
        let start = Instant::now();

        for _ in 0..10 {
            limiter.acquire("zai").await;
        }

        let elapsed = start.elapsed();
        assert!(
            elapsed >= std::time::Duration::from_millis(900),
            "expected throttling to spread 10 requests, got {elapsed:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(4),
            "expected test limiter to finish promptly, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn provider_rate_limiter_tracks_each_provider_independently() {
        let limiter = ProviderRateLimiter::new_per_second(1);
        let start = Instant::now();

        let ((), ()) = join!(limiter.acquire("zai"), limiter.acquire("openrouter"));

        assert!(
            start.elapsed() < std::time::Duration::from_millis(250),
            "different providers should not contend for the same budget"
        );
    }

    /// Two providers with independent per-second budgets do not contend.
    #[tokio::test]
    async fn per_provider_limits_give_independent_budgets() {
        let mut limits = HashMap::new();
        limits.insert("fast".to_string(), ProviderLimits { rpm: 600, tpm: 0 });
        limits.insert("slow".to_string(), ProviderLimits { rpm: 60, tpm: 0 });
        let limiter = ProviderRateLimiter::with_provider_limits(60, limits);
        let start = Instant::now();

        // Two concurrent acquires for different providers should not block each other.
        let ((), ()) = join!(limiter.acquire("fast"), limiter.acquire("slow"));

        assert!(
            start.elapsed() < std::time::Duration::from_millis(500),
            "independent providers should not contend: {:?}",
            start.elapsed()
        );
    }

    /// TPM tracker records tokens and returns the rolling total.
    #[test]
    fn tpm_tracker_accumulates_within_window() {
        let tracker = TpmTracker::new();
        let total = tracker.add(1000);
        assert_eq!(total, 1000);
        let total = tracker.add(500);
        assert_eq!(total, 1500);
    }

    /// `from_provider_configs` picks up per-provider limits.
    #[test]
    fn from_provider_configs_populates_per_provider_state() {
        use roko_core::agent::ProviderKind;
        use roko_core::config::provider::{ProviderConfig, ProviderLimits as CoreLimits};
        let mut configs = HashMap::new();
        configs.insert(
            "anthropic".to_string(),
            ProviderConfig {
                kind: ProviderKind::AnthropicApi,
                base_url: None,
                api_key_env: None,
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: Some(CoreLimits {
                    rpm: 50,
                    tpm: 40_000,
                    ..Default::default()
                }),
                require_confirmation: false,
                stream_usage: None,
                billing: None,
            },
        );
        let limiter = ProviderRateLimiter::from_provider_configs(60, configs.iter());
        // Provider state should exist for "anthropic".
        let providers = limiter.providers.lock().unwrap();
        assert!(
            providers.contains_key("anthropic"),
            "expected per-provider state for anthropic"
        );
        let state = providers.get("anthropic").unwrap();
        assert_eq!(state.tpm_limit, 40_000, "TPM limit should be 40000");
    }

    #[tokio::test]
    async fn snapshot_reports_default_and_explicit_provider_utilization() {
        use roko_core::agent::ProviderKind;
        use roko_core::config::provider::{ProviderConfig, ProviderLimits as CoreLimits};

        let mut configs = HashMap::new();
        configs.insert(
            "defaulted".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: None,
                api_key_env: None,
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
                billing: None,
            },
        );
        configs.insert(
            "limited".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                limits: Some(CoreLimits {
                    rpm: 500,
                    tpm: 50_000,
                    ..Default::default()
                }),
                base_url: None,
                api_key_env: None,
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                require_confirmation: false,
                stream_usage: None,
                billing: None,
            },
        );
        let limiter = ProviderRateLimiter::from_provider_configs(75, configs.iter());

        limiter.try_acquire("limited").await.expect("acquire");
        limiter.record_tokens("limited", 123).await;

        assert_eq!(
            limiter.snapshot(),
            vec![
                ProviderRateLimitSnapshot {
                    provider_id: "defaulted".to_string(),
                    rpm_used: 0,
                    rpm_limit: 75,
                    tpm_used: 0,
                    tpm_limit: 0,
                },
                ProviderRateLimitSnapshot {
                    provider_id: "limited".to_string(),
                    rpm_used: 1,
                    rpm_limit: 500,
                    tpm_used: 123,
                    tpm_limit: 50_000,
                },
            ]
        );
    }

    // ── Shared health integration tests ─────────────────────────────────

    /// Mock health checker that returns a fixed circuit state per provider.
    struct MockHealthChecker {
        states: Mutex<HashMap<String, CircuitState>>,
    }

    impl MockHealthChecker {
        fn new() -> Self {
            Self {
                states: Mutex::new(HashMap::new()),
            }
        }

        fn set_state(&self, provider: &str, state: CircuitState) {
            self.states
                .lock()
                .unwrap()
                .insert(provider.to_owned(), state);
        }
    }

    impl ProviderHealthChecker for MockHealthChecker {
        fn circuit_state(&self, provider_id: &str) -> CircuitState {
            self.states
                .lock()
                .unwrap()
                .get(provider_id)
                .copied()
                .unwrap_or(CircuitState::Closed)
        }

        fn record_probe_success(&self, provider_id: &str) {
            self.states
                .lock()
                .unwrap()
                .insert(provider_id.to_owned(), CircuitState::Closed);
        }

        fn record_probe_failure(&self, provider_id: &str) {
            self.states
                .lock()
                .unwrap()
                .insert(provider_id.to_owned(), CircuitState::Open);
        }
    }

    /// try_acquire returns Healthy when no health checker is set.
    #[tokio::test]
    async fn try_acquire_healthy_without_checker() {
        let limiter = ProviderRateLimiter::new(60);
        let outcome = limiter.try_acquire("anthropic").await.unwrap();
        assert_eq!(outcome, AcquireOutcome::Healthy);
    }

    /// try_acquire returns Healthy for a Closed circuit.
    #[tokio::test]
    async fn try_acquire_healthy_with_closed_circuit() {
        let checker = Arc::new(MockHealthChecker::new());
        checker.set_state("anthropic", CircuitState::Closed);
        let limiter = ProviderRateLimiter::new(60)
            .with_health_registry(checker as Arc<dyn ProviderHealthChecker>);
        let outcome = limiter.try_acquire("anthropic").await.unwrap();
        assert_eq!(outcome, AcquireOutcome::Healthy);
    }

    /// try_acquire returns RateLimitError for an Open circuit.
    #[tokio::test]
    async fn try_acquire_rejects_open_circuit() {
        let checker = Arc::new(MockHealthChecker::new());
        checker.set_state("anthropic", CircuitState::Open);
        let limiter = ProviderRateLimiter::new(60)
            .with_health_registry(checker as Arc<dyn ProviderHealthChecker>);
        let err = limiter.try_acquire("anthropic").await.unwrap_err();
        assert_eq!(err.provider_id, "anthropic");
        assert!(
            err.to_string().contains("circuit is open"),
            "error message should mention open circuit: {}",
            err
        );
    }

    /// try_acquire returns Probing for a HalfOpen circuit.
    #[tokio::test]
    async fn try_acquire_probes_halfopen_circuit() {
        let checker = Arc::new(MockHealthChecker::new());
        checker.set_state("anthropic", CircuitState::HalfOpen);
        let limiter = ProviderRateLimiter::new(60)
            .with_health_registry(checker as Arc<dyn ProviderHealthChecker>);
        let outcome = limiter.try_acquire("anthropic").await.unwrap();
        assert_eq!(outcome, AcquireOutcome::Probing);
    }

    /// Unknown providers default to Closed (healthy).
    #[tokio::test]
    async fn try_acquire_unknown_provider_defaults_healthy() {
        let checker = Arc::new(MockHealthChecker::new());
        // Do not set any state for "new-provider"
        let limiter = ProviderRateLimiter::new(60)
            .with_health_registry(checker as Arc<dyn ProviderHealthChecker>);
        let outcome = limiter.try_acquire("new-provider").await.unwrap();
        assert_eq!(outcome, AcquireOutcome::Healthy);
    }

    /// Open circuit for one provider does not affect another.
    #[tokio::test]
    async fn try_acquire_independent_per_provider() {
        let checker = Arc::new(MockHealthChecker::new());
        checker.set_state("anthropic", CircuitState::Open);
        checker.set_state("openrouter", CircuitState::Closed);
        let limiter = ProviderRateLimiter::new(60)
            .with_health_registry(checker as Arc<dyn ProviderHealthChecker>);

        // anthropic is blocked
        assert!(limiter.try_acquire("anthropic").await.is_err());
        // openrouter is fine
        let outcome = limiter.try_acquire("openrouter").await.unwrap();
        assert_eq!(outcome, AcquireOutcome::Healthy);
    }

    /// Probe success transitions checker state to Closed.
    #[tokio::test]
    async fn probe_success_transitions_to_closed() {
        let checker = Arc::new(MockHealthChecker::new());
        checker.set_state("anthropic", CircuitState::HalfOpen);
        let limiter = ProviderRateLimiter::new(60)
            .with_health_registry(Arc::clone(&checker) as Arc<dyn ProviderHealthChecker>);

        let outcome = limiter.try_acquire("anthropic").await.unwrap();
        assert_eq!(outcome, AcquireOutcome::Probing);

        // Simulate probe success
        checker.record_probe_success("anthropic");
        let outcome = limiter.try_acquire("anthropic").await.unwrap();
        assert_eq!(outcome, AcquireOutcome::Healthy);
    }

    /// Probe failure transitions checker state to Open.
    #[tokio::test]
    async fn probe_failure_transitions_to_open() {
        let checker = Arc::new(MockHealthChecker::new());
        checker.set_state("anthropic", CircuitState::HalfOpen);
        let limiter = ProviderRateLimiter::new(60)
            .with_health_registry(Arc::clone(&checker) as Arc<dyn ProviderHealthChecker>);

        let outcome = limiter.try_acquire("anthropic").await.unwrap();
        assert_eq!(outcome, AcquireOutcome::Probing);

        // Simulate probe failure
        checker.record_probe_failure("anthropic");
        assert!(limiter.try_acquire("anthropic").await.is_err());
    }

    /// set_health_registry works on an already-constructed limiter.
    #[tokio::test]
    async fn set_health_registry_after_construction() {
        let mut limiter = ProviderRateLimiter::new(60);
        assert!(limiter.health_checker().is_none());

        let checker = Arc::new(MockHealthChecker::new());
        checker.set_state("anthropic", CircuitState::Open);
        limiter.set_health_registry(checker as Arc<dyn ProviderHealthChecker>);
        assert!(limiter.health_checker().is_some());

        assert!(limiter.try_acquire("anthropic").await.is_err());
    }

    /// with_health_registry builder returns a limiter that shows the checker
    /// in debug output.
    #[test]
    fn debug_shows_health_checker_presence() {
        let without = ProviderRateLimiter::new(60);
        let debug_without = format!("{without:?}");
        assert!(
            debug_without.contains("has_health_checker: false"),
            "debug should show no health checker: {debug_without}"
        );

        let checker = Arc::new(MockHealthChecker::new());
        let with = ProviderRateLimiter::new(60)
            .with_health_registry(checker as Arc<dyn ProviderHealthChecker>);
        let debug_with = format!("{with:?}");
        assert!(
            debug_with.contains("has_health_checker: true"),
            "debug should show health checker: {debug_with}"
        );
    }

    /// Concurrent calls to the same provider share a single RPM budget.
    #[tokio::test]
    async fn pooled_limiter_gates_concurrent_calls_for_same_provider() {
        // 2 RPS = 120 RPM equivalent, but we use per_second for speed.
        let limiter = Arc::new(ProviderRateLimiter::new_per_second(2));
        let start = Instant::now();

        // Spawn 6 concurrent acquires -- with 2 RPS, they should take ~2s.
        let mut handles = Vec::new();
        for _ in 0..6 {
            let l = Arc::clone(&limiter);
            handles.push(tokio::spawn(async move {
                l.acquire("anthropic").await;
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        let elapsed = start.elapsed();
        assert!(
            elapsed >= std::time::Duration::from_millis(1_500),
            "6 requests at 2 RPS should take ~2s, got {elapsed:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(6),
            "should not take excessively long, got {elapsed:?}"
        );
    }
}
