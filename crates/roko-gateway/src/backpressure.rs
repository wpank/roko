//! Three-level bounded backpressure with cancellation-safe RAII accounting.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// One provider's concurrency and bounded waiting-room configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderLimitConfig {
    /// Requests executing concurrently.
    pub concurrency: u32,
    /// Requests allowed to wait for an execution permit.
    pub queue_capacity: u32,
}

impl ProviderLimitConfig {
    /// Default queue size is twice the execution limit.
    #[must_use]
    pub const fn with_double_queue(concurrency: u32) -> Self {
        Self {
            concurrency,
            queue_capacity: concurrency.saturating_mul(2),
        }
    }
}

/// Backpressure limits. Tests and deployments may override every value.
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// Provider-specific concurrency and queue limits.
    pub providers: HashMap<String, ProviderLimitConfig>,
    /// Maximum queued + executing calls for one agent.
    pub per_agent: u32,
    /// Maximum queued + executing calls globally.
    pub global: u32,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        let providers = [
            ("anthropic", 50),
            ("openai", 50),
            ("gemini", 30),
            ("perplexity", 20),
            ("ollama", 4),
            ("openrouter", 50),
            ("other", 20),
        ]
        .into_iter()
        .map(|(name, concurrency)| {
            (
                name.to_string(),
                ProviderLimitConfig::with_double_queue(concurrency),
            )
        })
        .collect();
        Self {
            providers,
            per_agent: 8,
            global: 200,
        }
    }
}

/// Queue rejection class with HTTP-compatible retry guidance.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackpressureError {
    /// Provider execution plus waiting room is full.
    #[error("provider queue full: {provider}")]
    ProviderFull {
        /// Provider identifier.
        provider: String,
    },
    /// One agent already owns its maximum in-flight slots.
    #[error("agent queue full: {agent_id}")]
    AgentQueueFull {
        /// Agent identifier.
        agent_id: String,
    },
    /// Process-wide queue is full.
    #[error("gateway globally overloaded")]
    GlobalOverload,
}

impl BackpressureError {
    /// HTTP status code suitable for an API adapter.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        match self {
            Self::AgentQueueFull { .. } => 429,
            Self::ProviderFull { .. } | Self::GlobalOverload => 503,
        }
    }

    /// Suggested Retry-After duration in seconds.
    #[must_use]
    pub const fn retry_after_seconds(&self) -> u64 {
        match self {
            Self::AgentQueueFull { .. } => 2,
            Self::ProviderFull { .. } | Self::GlobalOverload => 5,
        }
    }
}

struct ProviderState {
    semaphore: Arc<Semaphore>,
    config: ProviderLimitConfig,
    outstanding: Arc<AtomicU32>,
    queued: Arc<AtomicU32>,
    active: Arc<AtomicU32>,
    rejected: AtomicU32,
}

impl ProviderState {
    fn new(config: ProviderLimitConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.concurrency as usize)),
            config,
            outstanding: Arc::new(AtomicU32::new(0)),
            queued: Arc::new(AtomicU32::new(0)),
            active: Arc::new(AtomicU32::new(0)),
            rejected: AtomicU32::new(0),
        }
    }
}

/// Per-provider queue snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ProviderBackpressureStats {
    /// Requests waiting for a provider execution slot.
    pub queue_depth: u32,
    /// Requests currently executing.
    pub active_requests: u32,
    /// Requests rejected because this provider was full.
    pub rejected_count: u32,
    /// Configured concurrency.
    pub concurrency_limit: u32,
    /// Configured waiting-room capacity.
    pub queue_capacity: u32,
}

/// Aggregate queue snapshot suitable for `/api/gateway/stats`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BackpressureStats {
    /// Provider-keyed queue snapshots.
    pub providers: HashMap<String, ProviderBackpressureStats>,
    /// Total queued + executing requests.
    pub global_in_flight: u32,
    /// Requests rejected by the global bound.
    pub global_rejected: u32,
}

/// Three-level queue/concurrency guard.
pub struct BackpressureGuard {
    providers: HashMap<String, Arc<ProviderState>>,
    agent_in_flight: DashMap<String, Arc<AtomicU32>>,
    per_agent_limit: u32,
    global_in_flight: Arc<AtomicU32>,
    global_limit: u32,
    global_rejected: AtomicU32,
}

impl Default for BackpressureGuard {
    fn default() -> Self {
        Self::new(BackpressureConfig::default())
    }
}

impl BackpressureGuard {
    /// Construct from explicit limits.
    #[must_use]
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            providers: config
                .providers
                .into_iter()
                .map(|(name, limit)| {
                    (
                        name.to_ascii_lowercase(),
                        Arc::new(ProviderState::new(limit)),
                    )
                })
                .collect(),
            agent_in_flight: DashMap::new(),
            per_agent_limit: config.per_agent.max(1),
            global_in_flight: Arc::new(AtomicU32::new(0)),
            global_limit: config.global.max(1),
            global_rejected: AtomicU32::new(0),
        }
    }

    /// Reserve global, agent, and provider capacity, then await provider concurrency.
    pub async fn acquire(
        &self,
        provider: &str,
        agent_id: &str,
    ) -> Result<BackpressurePermit, BackpressureError> {
        if !try_reserve(&self.global_in_flight, self.global_limit) {
            self.global_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(BackpressureError::GlobalOverload);
        }
        let agent = self
            .agent_in_flight
            .entry(agent_id.to_string())
            .or_insert_with(|| Arc::new(AtomicU32::new(0)))
            .clone();
        if !try_reserve(&agent, self.per_agent_limit) {
            release(&self.global_in_flight);
            return Err(BackpressureError::AgentQueueFull {
                agent_id: agent_id.to_string(),
            });
        }

        let provider_name = provider.to_ascii_lowercase();
        let state = self
            .providers
            .get(&provider_name)
            .or_else(|| self.providers.get("other"))
            .expect("default backpressure config always contains other")
            .clone();
        let max_outstanding = state
            .config
            .concurrency
            .saturating_add(state.config.queue_capacity);
        if !try_reserve(&state.outstanding, max_outstanding) {
            release(&agent);
            release(&self.global_in_flight);
            state.rejected.fetch_add(1, Ordering::Relaxed);
            return Err(BackpressureError::ProviderFull {
                provider: provider_name,
            });
        }

        let permit = match Arc::clone(&state.semaphore).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                state.queued.fetch_add(1, Ordering::Relaxed);
                let pending = PendingReservation {
                    provider_outstanding: Arc::clone(&state.outstanding),
                    queued: Arc::clone(&state.queued),
                    agent: Arc::clone(&agent),
                    global: Arc::clone(&self.global_in_flight),
                    armed: true,
                };
                match Arc::clone(&state.semaphore).acquire_owned().await {
                    Ok(permit) => {
                        pending.activate();
                        permit
                    }
                    Err(_) => {
                        state.rejected.fetch_add(1, Ordering::Relaxed);
                        return Err(BackpressureError::ProviderFull {
                            provider: provider_name,
                        });
                    }
                }
            }
        };
        state.active.fetch_add(1, Ordering::Relaxed);
        Ok(BackpressurePermit {
            _provider_permit: permit,
            provider_outstanding: Arc::clone(&state.outstanding),
            active: Arc::clone(&state.active),
            agent,
            global: Arc::clone(&self.global_in_flight),
        })
    }

    /// Read queue and rejection state without blocking request processing.
    #[must_use]
    pub fn stats(&self) -> BackpressureStats {
        BackpressureStats {
            providers: self
                .providers
                .iter()
                .map(|(name, state)| {
                    (
                        name.clone(),
                        ProviderBackpressureStats {
                            queue_depth: state.queued.load(Ordering::Relaxed),
                            active_requests: state.active.load(Ordering::Relaxed),
                            rejected_count: state.rejected.load(Ordering::Relaxed),
                            concurrency_limit: state.config.concurrency,
                            queue_capacity: state.config.queue_capacity,
                        },
                    )
                })
                .collect(),
            global_in_flight: self.global_in_flight.load(Ordering::Relaxed),
            global_rejected: self.global_rejected.load(Ordering::Relaxed),
        }
    }
}

/// RAII ownership of all three capacity levels.
#[derive(Debug)]
pub struct BackpressurePermit {
    _provider_permit: OwnedSemaphorePermit,
    provider_outstanding: Arc<AtomicU32>,
    active: Arc<AtomicU32>,
    agent: Arc<AtomicU32>,
    global: Arc<AtomicU32>,
}

impl Drop for BackpressurePermit {
    fn drop(&mut self) {
        release(&self.active);
        release(&self.provider_outstanding);
        release(&self.agent);
        release(&self.global);
    }
}

struct PendingReservation {
    provider_outstanding: Arc<AtomicU32>,
    queued: Arc<AtomicU32>,
    agent: Arc<AtomicU32>,
    global: Arc<AtomicU32>,
    armed: bool,
}

impl PendingReservation {
    fn activate(mut self) {
        release(&self.queued);
        self.armed = false;
    }
}

impl Drop for PendingReservation {
    fn drop(&mut self) {
        if self.armed {
            release(&self.provider_outstanding);
            release(&self.queued);
            release(&self.agent);
            release(&self.global);
        }
    }
}

fn try_reserve(counter: &AtomicU32, limit: u32) -> bool {
    counter
        .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |current| {
            (current < limit).then_some(current + 1)
        })
        .is_ok()
}

fn release(counter: &AtomicU32) {
    let _ = counter.fetch_update(Ordering::AcqRel, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(1))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backpressure_default_provider_limits_match_contract() {
        let guard = BackpressureGuard::default();
        let stats = guard.stats();
        assert_eq!(stats.providers["anthropic"].concurrency_limit, 50);
        assert_eq!(stats.providers["openai"].concurrency_limit, 50);
        assert_eq!(stats.providers["ollama"].concurrency_limit, 4);
        assert_eq!(stats.providers["anthropic"].queue_capacity, 100);
    }

    #[tokio::test]
    async fn backpressure_rejects_ninth_agent_request_and_releases_on_drop() {
        let guard = BackpressureGuard::default();
        let mut permits = Vec::new();
        for _ in 0..8 {
            permits.push(guard.acquire("anthropic", "agent").await.unwrap());
        }
        let error = guard.acquire("anthropic", "agent").await.unwrap_err();
        assert_eq!(error.status_code(), 429);
        assert_eq!(error.retry_after_seconds(), 2);
        permits.pop();
        assert!(guard.acquire("anthropic", "agent").await.is_ok());
    }

    #[tokio::test]
    async fn backpressure_global_overload_is_503() {
        let mut config = BackpressureConfig::default();
        config.global = 2;
        config.per_agent = 8;
        let guard = BackpressureGuard::new(config);
        let _first = guard.acquire("anthropic", "a").await.unwrap();
        let _second = guard.acquire("anthropic", "b").await.unwrap();
        let error = guard.acquire("anthropic", "c").await.unwrap_err();
        assert_eq!(error, BackpressureError::GlobalOverload);
        assert_eq!(error.status_code(), 503);
        assert_eq!(guard.stats().global_rejected, 1);
    }

    #[tokio::test]
    async fn backpressure_provider_waiting_room_is_bounded_and_cancellation_safe() {
        let mut config = BackpressureConfig::default();
        config.providers.insert(
            "tiny".into(),
            ProviderLimitConfig {
                concurrency: 1,
                queue_capacity: 1,
            },
        );
        config.per_agent = 10;
        config.global = 10;
        let guard = Arc::new(BackpressureGuard::new(config));
        let active = guard.acquire("tiny", "active").await.unwrap();

        let waiting_guard = Arc::clone(&guard);
        let waiting = tokio::spawn(async move { waiting_guard.acquire("tiny", "waiting").await });
        for _ in 0..20 {
            if guard.stats().providers["tiny"].queue_depth == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(guard.stats().providers["tiny"].queue_depth, 1);
        let error = guard.acquire("tiny", "rejected").await.unwrap_err();
        assert!(matches!(error, BackpressureError::ProviderFull { .. }));
        assert_eq!(error.status_code(), 503);

        waiting.abort();
        let _ = waiting.await;
        assert_eq!(guard.stats().providers["tiny"].queue_depth, 0);
        drop(active);
        assert!(guard.acquire("tiny", "after-cancel").await.is_ok());
    }
}
