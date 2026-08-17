//! Gateway error taxonomy.

use serde::{Deserialize, Serialize};

/// Provider failure classes relevant to fallback and key rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFailureKind {
    /// Provider rejected the request for quota/rate-limit reasons.
    RateLimited,
    /// Provider is temporarily unavailable.
    Unavailable,
    /// Provider did not respond before the deadline.
    Timeout,
    /// Non-retryable request or provider failure.
    Fatal,
}

impl ProviderFailureKind {
    /// Whether another model/provider should be attempted.
    #[must_use]
    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimited | Self::Unavailable | Self::Timeout)
    }
}

/// Failures surfaced by the inference gateway.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    /// No registered backend supports a routed model.
    #[error("no provider backend supports model {0}")]
    NoProvider(String),
    /// A provider attempt failed.
    #[error("provider {provider} failed ({kind:?}): {message}")]
    Provider {
        /// Provider identifier.
        provider: String,
        /// Retry/fallback class.
        kind: ProviderFailureKind,
        /// Sanitized failure message.
        message: String,
    },
    /// All eligible primary/fallback attempts failed.
    #[error("all provider attempts failed: {0}")]
    ProvidersExhausted(String),
    /// A request would exceed its caller budget.
    #[error("inference budget exceeded: remaining {remaining_microdollars} microdollars")]
    BudgetExceeded {
        /// Remaining budget at rejection.
        remaining_microdollars: u64,
    },
    /// Gateway request channel is unavailable.
    #[error("gateway request channel is closed")]
    ChannelClosed,
    /// The gateway loop has already been started.
    #[error("gateway loop already started")]
    AlreadyStarted,
    /// Cached bytes could not be decoded.
    #[error("invalid cached response: {0}")]
    CacheDecode(#[from] serde_json::Error),
    /// Durable telemetry write failed.
    #[error("gateway event log write failed: {0}")]
    EventLog(#[from] std::io::Error),
    /// Request was rejected by a bounded queue.
    #[error(transparent)]
    Backpressure(#[from] crate::backpressure::BackpressureError),
    /// Batch queue has reached its configured maximum.
    #[error("batch queue is full (capacity {capacity})")]
    BatchQueueFull {
        /// Configured queue capacity.
        capacity: usize,
    },
}

/// Gateway result alias.
pub type GatewayResult<T> = Result<T, GatewayError>;
