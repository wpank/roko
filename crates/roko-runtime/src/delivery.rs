//! Outbound message delivery state machine (#415).
//!
//! [`DeliveryStateMachine`] tracks the lifecycle of a single outbound message
//! from initial queuing through final delivery or dead-lettering.
//!
//! # State transitions
//!
//! ```text
//!   Queued
//!     │
//!     ▼  attempt()
//!   Sending
//!     │
//!     ├──► Delivered   (success)
//!     │
//!     └──► Failed      (error, attempts < max_retries)
//!               │
//!               ▼  retry()
//!             Retrying
//!               │
//!               ▼  attempt()
//!             Sending  …
//!               │
//!               └──► DeadLettered  (error, attempts >= max_retries)
//! ```
//!
//! # Backoff
//!
//! Retries use capped exponential backoff:
//! - Attempt 1: 1 s
//! - Attempt 2: 2 s
//! - Attempt 3: 4 s
//! - Attempt 4: 8 s
//! - Attempt 5+: 30 s (cap)
//!
//! # Usage
//!
//! ```rust,no_run
//! use roko_runtime::delivery::{DeliveryConfig, DeliveryStateMachine};
//!
//! let mut sm = DeliveryStateMachine::new("msg-42", DeliveryConfig::default());
//! sm.attempt(); // Queued → Sending
//! sm.succeed(); // Sending → Delivered
//! assert!(sm.is_terminal());
//! ```

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Maximum backoff delay applied after any single failure.
pub const MAX_BACKOFF_SECS: u64 = 30;

/// Configuration for the delivery retry policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryConfig {
    /// Maximum number of delivery attempts (first attempt + this many retries).
    ///
    /// A value of `0` means deliver exactly once with no retries.
    /// Defaults to `3`.
    pub max_retries: u32,

    /// Base interval for the first retry in milliseconds.
    ///
    /// Subsequent retries double this value, capped at [`MAX_BACKOFF_SECS`] × 1000.
    /// Defaults to `1000` ms (1 second).
    pub base_retry_delay_ms: u64,
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_retry_delay_ms: 1_000,
        }
    }
}

impl DeliveryConfig {
    /// Compute the backoff duration for the given zero-based retry index.
    ///
    /// - index 0 → `base_retry_delay_ms`
    /// - index 1 → `base_retry_delay_ms × 2`
    /// - …
    /// - capped at [`MAX_BACKOFF_SECS`] × 1000 ms.
    #[must_use]
    pub fn backoff_for(&self, retry_index: u32) -> Duration {
        let max_ms = MAX_BACKOFF_SECS * 1_000;
        // 2^retry_index, capped to avoid overflow; if the shift overflows use max.
        let multiplier = 1u64.checked_shl(retry_index).unwrap_or(u64::MAX);
        let ms = self.base_retry_delay_ms.saturating_mul(multiplier).min(max_ms);
        Duration::from_millis(ms)
    }
}

// ---------------------------------------------------------------------------
// DeliveryState
// ---------------------------------------------------------------------------

/// Lifecycle state of a single outbound message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DeliveryState {
    /// Message is waiting to be sent for the first time.
    Queued,
    /// A delivery attempt is currently in-flight.
    Sending {
        /// Zero-based attempt index (0 = first attempt, 1 = first retry, …).
        attempt: u32,
    },
    /// The message was delivered successfully.
    Delivered {
        /// Zero-based attempt index on which delivery succeeded.
        attempt: u32,
    },
    /// The most recent attempt failed; waiting for the backoff delay before retrying.
    Failed {
        /// The error that caused the failure.
        error: String,
        /// Zero-based attempt index that failed.
        attempt: u32,
        /// Monotonic timestamp when the failure occurred (not serialized; reset on restore).
        #[serde(skip)]
        failed_at: Option<Instant>,
    },
    /// Actively waiting before the next retry attempt.
    Retrying {
        /// The error that caused the previous failure.
        error: String,
        /// Zero-based attempt index of the upcoming retry.
        next_attempt: u32,
        /// How long to wait before the next attempt.
        backoff: Duration,
        /// Monotonic timestamp when the retry delay started (not serialized; reset on restore).
        #[serde(skip)]
        retry_started_at: Option<Instant>,
    },
    /// All retries exhausted; message moved to dead-letter storage.
    DeadLettered {
        /// Final error message.
        error: String,
        /// Total number of attempts made.
        total_attempts: u32,
    },
}

impl DeliveryState {
    /// Returns `true` if this state requires no further action.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Delivered { .. } | Self::DeadLettered { .. })
    }

    /// Human-readable label for the current state.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Sending { .. } => "sending",
            Self::Delivered { .. } => "delivered",
            Self::Failed { .. } => "failed",
            Self::Retrying { .. } => "retrying",
            Self::DeadLettered { .. } => "dead_lettered",
        }
    }
}

// ---------------------------------------------------------------------------
// DeliveryStateMachine
// ---------------------------------------------------------------------------

/// Tracks the delivery lifecycle for a single outbound message.
///
/// State transitions are driven by the caller:
/// - [`attempt`](Self::attempt) — start a send attempt.
/// - [`succeed`](Self::succeed) — record delivery success.
/// - [`fail`](Self::fail) — record delivery failure and schedule a retry.
/// - [`retry`](Self::retry) — transition `Failed` → `Retrying`.
///
/// The machine does **not** drive async I/O itself; it is a pure state tracker
/// that the owning send loop queries for the next action to take.
#[derive(Debug, Clone)]
pub struct DeliveryStateMachine {
    /// Opaque message identifier for logging.
    pub message_id: String,
    /// Delivery retry policy.
    pub config: DeliveryConfig,
    /// Current state.
    pub state: DeliveryState,
}

impl DeliveryStateMachine {
    /// Create a new state machine in the `Queued` state.
    #[must_use]
    pub fn new(message_id: impl Into<String>, config: DeliveryConfig) -> Self {
        Self {
            message_id: message_id.into(),
            config,
            state: DeliveryState::Queued,
        }
    }

    /// Transition to `Sending`.
    ///
    /// Valid from: `Queued`, `Retrying`.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if called from an invalid state.  In release
    /// builds the call is a no-op for invalid states.
    pub fn attempt(&mut self) {
        let attempt = match &self.state {
            DeliveryState::Queued => 0,
            DeliveryState::Retrying { next_attempt, .. } => *next_attempt,
            other => {
                debug_assert!(
                    false,
                    "attempt() called from invalid state: {}",
                    other.label()
                );
                return;
            }
        };
        self.state = DeliveryState::Sending { attempt };
    }

    /// Record a successful delivery.
    ///
    /// Valid from: `Sending`.
    pub fn succeed(&mut self) {
        if let DeliveryState::Sending { attempt } = self.state {
            self.state = DeliveryState::Delivered { attempt };
        } else {
            debug_assert!(
                false,
                "succeed() called from invalid state: {}",
                self.state.label()
            );
        }
    }

    /// Record a failed delivery attempt.
    ///
    /// If `attempts_so_far < max_retries`, transitions to `Failed` and
    /// schedules a retry. If `attempts_so_far >= max_retries`, transitions
    /// directly to `DeadLettered`.
    ///
    /// Valid from: `Sending`.
    pub fn fail(&mut self, error: impl Into<String>) {
        let DeliveryState::Sending { attempt } = self.state else {
            debug_assert!(
                false,
                "fail() called from invalid state: {}",
                self.state.label()
            );
            return;
        };
        let error = error.into();
        if attempt >= self.config.max_retries {
            // No more retries allowed.
            self.state = DeliveryState::DeadLettered {
                error,
                total_attempts: attempt + 1,
            };
        } else {
            self.state = DeliveryState::Failed {
                error,
                attempt,
                failed_at: Some(Instant::now()),
            };
        }
    }

    /// Transition `Failed` → `Retrying`, computing the appropriate backoff.
    ///
    /// Valid from: `Failed`.
    pub fn retry(&mut self) {
        let DeliveryState::Failed { ref error, attempt, .. } = self.state else {
            debug_assert!(
                false,
                "retry() called from invalid state: {}",
                self.state.label()
            );
            return;
        };
        let next_attempt = attempt + 1;
        // retry_index is 0-based: first retry (attempt=0→next=1) uses index 0.
        let retry_index = attempt;
        let backoff = self.config.backoff_for(retry_index);
        let error = error.clone();
        self.state = DeliveryState::Retrying {
            error,
            next_attempt,
            backoff,
            retry_started_at: Some(Instant::now()),
        };
    }

    /// Returns `true` if the current state is terminal (delivered or dead-lettered).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Returns `true` if the message was delivered successfully.
    #[must_use]
    pub fn is_delivered(&self) -> bool {
        matches!(self.state, DeliveryState::Delivered { .. })
    }

    /// Returns `true` if the message has been dead-lettered.
    #[must_use]
    pub fn is_dead_lettered(&self) -> bool {
        matches!(self.state, DeliveryState::DeadLettered { .. })
    }

    /// If in `Retrying` state and the backoff duration has elapsed, return
    /// `true` (the caller should invoke [`attempt`](Self::attempt) next).
    #[must_use]
    pub fn backoff_elapsed(&self) -> bool {
        if let DeliveryState::Retrying {
            backoff,
            retry_started_at: Some(started),
            ..
        } = &self.state
        {
            started.elapsed() >= *backoff
        } else {
            false
        }
    }

    /// Return the next backoff duration, if the machine is in `Retrying` state.
    #[must_use]
    pub fn next_backoff(&self) -> Option<Duration> {
        if let DeliveryState::Retrying { backoff, .. } = &self.state {
            Some(*backoff)
        } else {
            None
        }
    }

    /// Return the current state label (for logging).
    #[must_use]
    pub fn state_label(&self) -> &'static str {
        self.state.label()
    }
}

// ---------------------------------------------------------------------------
// Serializable snapshot for persistence
// ---------------------------------------------------------------------------

/// A serializable snapshot of a [`DeliveryStateMachine`] (omits `Instant` fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverySnapshot {
    /// Opaque message identifier.
    pub message_id: String,
    /// Current state (without monotonic timestamps).
    pub state: DeliveryState,
    /// Retry configuration.
    pub config: DeliveryConfig,
}

impl From<&DeliveryStateMachine> for DeliverySnapshot {
    fn from(sm: &DeliveryStateMachine) -> Self {
        // Clone state; Instant fields are #[serde(skip)] so they're absent from serialization.
        let state = sm.state.clone();
        // Strip monotonic instants so the snapshot is serialization-safe.
        let state = match state {
            DeliveryState::Failed { error, attempt, .. } => {
                DeliveryState::Failed { error, attempt, failed_at: None }
            }
            DeliveryState::Retrying { error, next_attempt, backoff, .. } => {
                DeliveryState::Retrying {
                    error,
                    next_attempt,
                    backoff,
                    retry_started_at: None,
                }
            }
            other => other,
        };
        Self {
            message_id: sm.message_id.clone(),
            state,
            config: sm.config.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_sm(id: &str) -> DeliveryStateMachine {
        DeliveryStateMachine::new(id, DeliveryConfig::default())
    }

    #[test]
    fn happy_path_delivered() {
        let mut sm = default_sm("msg-1");
        assert_eq!(sm.state_label(), "queued");

        sm.attempt();
        assert_eq!(sm.state_label(), "sending");

        sm.succeed();
        assert_eq!(sm.state_label(), "delivered");
        assert!(sm.is_terminal());
        assert!(sm.is_delivered());
        assert!(!sm.is_dead_lettered());
    }

    #[test]
    fn retry_cycle_then_delivered() {
        let config = DeliveryConfig {
            max_retries: 2,
            base_retry_delay_ms: 10,
        };
        let mut sm = DeliveryStateMachine::new("msg-2", config);

        // First attempt fails.
        sm.attempt();
        sm.fail("network error");
        assert_eq!(sm.state_label(), "failed");

        // Schedule retry.
        sm.retry();
        assert_eq!(sm.state_label(), "retrying");
        let backoff = sm.next_backoff().unwrap();
        assert_eq!(backoff, Duration::from_millis(10));

        // Second attempt succeeds.
        sm.attempt();
        sm.succeed();
        assert!(sm.is_delivered());
    }

    #[test]
    fn all_retries_exhausted_dead_letters() {
        let config = DeliveryConfig {
            max_retries: 2,
            base_retry_delay_ms: 10,
        };
        let mut sm = DeliveryStateMachine::new("msg-3", config);

        for _ in 0..=2 {
            sm.attempt();
            sm.fail("timeout");
            if !sm.is_dead_lettered() {
                sm.retry();
            }
        }

        assert!(sm.is_dead_lettered());
        assert!(sm.is_terminal());
        if let DeliveryState::DeadLettered { total_attempts, .. } = &sm.state {
            assert_eq!(*total_attempts, 3);
        }
    }

    #[test]
    fn zero_retries_dead_letters_on_first_failure() {
        let config = DeliveryConfig {
            max_retries: 0,
            base_retry_delay_ms: 1_000,
        };
        let mut sm = DeliveryStateMachine::new("msg-4", config);
        sm.attempt();
        sm.fail("fatal");
        assert!(sm.is_dead_lettered());
    }

    #[test]
    fn backoff_doubles_up_to_cap() {
        let config = DeliveryConfig {
            max_retries: 10,
            base_retry_delay_ms: 1_000,
        };
        assert_eq!(config.backoff_for(0), Duration::from_secs(1));
        assert_eq!(config.backoff_for(1), Duration::from_secs(2));
        assert_eq!(config.backoff_for(2), Duration::from_secs(4));
        assert_eq!(config.backoff_for(3), Duration::from_secs(8));
        // Cap at 30 s.
        assert_eq!(config.backoff_for(5), Duration::from_secs(30));
        assert_eq!(config.backoff_for(10), Duration::from_secs(30));
    }

    #[test]
    fn snapshot_round_trips_serde() {
        let mut sm = default_sm("msg-snap");
        sm.attempt();
        sm.fail("err");
        sm.retry();

        let snap = DeliverySnapshot::from(&sm);
        let json = serde_json::to_string(&snap).unwrap();
        let restored: DeliverySnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.message_id, "msg-snap");
        assert_eq!(restored.state.label(), "retrying");
    }

    #[test]
    fn is_terminal_false_for_intermediate_states() {
        let mut sm = default_sm("msg-term");
        assert!(!sm.is_terminal()); // Queued
        sm.attempt();
        assert!(!sm.is_terminal()); // Sending
        sm.fail("err");
        assert!(!sm.is_terminal()); // Failed
        sm.retry();
        assert!(!sm.is_terminal()); // Retrying
    }

    #[test]
    fn backoff_elapsed_false_when_not_retrying() {
        let mut sm = default_sm("msg-elapsed");
        assert!(!sm.backoff_elapsed());
        sm.attempt();
        assert!(!sm.backoff_elapsed());
        sm.succeed();
        assert!(!sm.backoff_elapsed());
    }
}
