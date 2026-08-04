//! Graph engine errors.
//!
//! The canonical error enum lives in [`crate::types::GraphError`].
//! This module re-exports it and provides a convenience `Result` alias.
//!
//! [`CellError`] is a sibling enum for per-Cell execution errors, convertible
//! to [`GraphError::NodeFailed`] for graph-level reporting.

pub use crate::types::GraphError;

/// Convenience Result type for graph operations.
pub type Result<T> = std::result::Result<T, GraphError>;

// ─── Cell-level error taxonomy ────────────────────────────────────────────────

/// Recovery strategy to apply after a [`CellError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorRecovery {
    /// Retry execution with exponential backoff.
    Retry {
        /// Maximum number of retry attempts.
        max_attempts: u32,
        /// Base backoff in milliseconds (doubles each attempt).
        backoff_ms: u64,
    },
    /// Skip this cell and allow dependents to continue if possible.
    Skip {
        /// Human-readable explanation of why the cell was skipped.
        reason: String,
    },
    /// Escalate to the supervisor for manual intervention.
    Escalate {
        /// Target supervisor or policy name to escalate to.
        to: String,
    },
    /// Abort the entire graph execution immediately.
    Abort {
        /// Human-readable explanation of why execution was aborted.
        reason: String,
    },
}

/// Per-Cell execution error taxonomy.
///
/// These are the typed failure modes a [`crate::cell::Cell`] can report. The
/// graph engine uses [`CellError::suggested_recovery`] to select a default
/// recovery strategy, which callers can override.
///
/// Convert to [`GraphError::NodeFailed`] for graph-level reporting via
/// [`CellError::into_node_failed`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CellError {
    /// Transient failure — retry with backoff.
    #[error("transient cell error: {message}")]
    Transient {
        /// Human-readable description of the failure.
        message: String,
        /// Suggested initial backoff before the first retry (milliseconds).
        retry_after_ms: u64,
    },

    /// Permanent failure — cannot be retried.
    #[error("permanent cell error: {message}")]
    Permanent {
        /// Human-readable description of the failure.
        message: String,
    },

    /// A required resource is exhausted; wait before retrying.
    #[error("resource exhausted ({resource}): {message}")]
    ResourceExhausted {
        /// Name of the exhausted resource (e.g. "token_budget", "api_quota").
        resource: String,
        /// Human-readable description.
        message: String,
        /// Suggested wait duration in milliseconds before retrying.
        wait_ms: u64,
    },

    /// The cell did not finish within its allocated time.
    #[error("cell timeout after {elapsed_ms}ms: {message}")]
    Timeout {
        /// Human-readable description.
        message: String,
        /// How long the cell ran before timing out (milliseconds).
        elapsed_ms: u64,
    },

    /// A policy check rejected the cell's execution.
    #[error("policy violation ({policy}): {message}")]
    PolicyViolation {
        /// Name of the violated policy (e.g. "safety", "role_auth").
        policy: String,
        /// Human-readable description.
        message: String,
    },

    /// The cell consumed more budget than was available.
    #[error("budget exceeded (limit={limit}, consumed={consumed}): {message}")]
    BudgetExceeded {
        /// Budget limit that was breached (in microdollars or tokens).
        limit: u64,
        /// How much was actually consumed.
        consumed: u64,
        /// Human-readable description.
        message: String,
    },
}

impl CellError {
    /// Default [`ErrorRecovery`] strategy for this error variant.
    ///
    /// The graph engine uses this as a starting point; callers may override it
    /// based on domain-specific knowledge.
    #[must_use]
    pub fn suggested_recovery(&self) -> ErrorRecovery {
        match self {
            Self::Transient { retry_after_ms, .. } => ErrorRecovery::Retry {
                max_attempts: 3,
                backoff_ms: *retry_after_ms,
            },
            Self::Permanent { message } => ErrorRecovery::Abort {
                reason: message.clone(),
            },
            Self::ResourceExhausted { wait_ms, .. } => ErrorRecovery::Retry {
                max_attempts: 5,
                backoff_ms: *wait_ms,
            },
            Self::Timeout { .. } => ErrorRecovery::Retry {
                max_attempts: 2,
                backoff_ms: 1_000,
            },
            Self::PolicyViolation { policy, .. } => {
                ErrorRecovery::Escalate { to: policy.clone() }
            }
            Self::BudgetExceeded { message, .. } => ErrorRecovery::Skip {
                reason: message.clone(),
            },
        }
    }

    /// Convert this [`CellError`] into a [`GraphError::NodeFailed`] for
    /// graph-level error reporting.
    #[must_use]
    pub fn into_node_failed(self, node_id: impl Into<String>) -> GraphError {
        GraphError::NodeFailed {
            node_id: node_id.into(),
            reason: self.to_string(),
        }
    }
}
