//! Webhook delivery counters (#420).
//!
//! [`WebhookMetrics`] maintains three atomic counters — total sent, failed,
//! and retried — so the serve layer and CLI can report basic delivery health
//! without adding a full metrics dependency.
//!
//! Counters are monotonically increasing; they never decrement.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// WebhookMetrics
// ---------------------------------------------------------------------------

/// Atomic delivery counters for webhook events.
///
/// Cheaply cloneable (backed by `Arc`).  All updates are lock-free.
#[derive(Clone, Debug)]
pub struct WebhookMetrics {
    inner: Arc<MetricsInner>,
}

#[derive(Debug, Default)]
struct MetricsInner {
    sent: AtomicU64,
    failed: AtomicU64,
    retried: AtomicU64,
}

/// A snapshot of the current counter values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookMetricsSnapshot {
    /// Total number of webhook delivery attempts that succeeded.
    pub total_sent: u64,
    /// Total number of webhook delivery attempts that failed (including those
    /// that were subsequently retried).
    pub total_failed: u64,
    /// Total number of retry attempts made across all deliveries.
    pub total_retried: u64,
}

impl WebhookMetrics {
    /// Construct a fresh set of zeroed counters.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner::default()),
        }
    }

    /// Increment the "sent" counter by one.
    pub fn record_sent(&self) {
        self.inner.sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the "failed" counter by one.
    pub fn record_failed(&self) {
        self.inner.failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the "retried" counter by one.
    pub fn record_retry(&self) {
        self.inner.retried.fetch_add(1, Ordering::Relaxed);
    }

    /// Return a point-in-time snapshot of all counters.
    #[must_use]
    pub fn snapshot(&self) -> WebhookMetricsSnapshot {
        WebhookMetricsSnapshot {
            total_sent: self.inner.sent.load(Ordering::Relaxed),
            total_failed: self.inner.failed.load(Ordering::Relaxed),
            total_retried: self.inner.retried.load(Ordering::Relaxed),
        }
    }
}

impl Default for WebhookMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_start_at_zero() {
        let m = WebhookMetrics::new();
        let snap = m.snapshot();
        assert_eq!(snap.total_sent, 0);
        assert_eq!(snap.total_failed, 0);
        assert_eq!(snap.total_retried, 0);
    }

    #[test]
    fn record_sent_increments() {
        let m = WebhookMetrics::new();
        m.record_sent();
        m.record_sent();
        assert_eq!(m.snapshot().total_sent, 2);
    }

    #[test]
    fn record_failed_increments() {
        let m = WebhookMetrics::new();
        m.record_failed();
        assert_eq!(m.snapshot().total_failed, 1);
    }

    #[test]
    fn record_retry_increments() {
        let m = WebhookMetrics::new();
        m.record_retry();
        m.record_retry();
        m.record_retry();
        assert_eq!(m.snapshot().total_retried, 3);
    }

    #[test]
    fn clone_shares_state() {
        let m1 = WebhookMetrics::new();
        let m2 = m1.clone();
        m1.record_sent();
        assert_eq!(m2.snapshot().total_sent, 1);
    }

    #[test]
    fn snapshot_serializes_to_json() {
        let m = WebhookMetrics::new();
        m.record_sent();
        m.record_failed();
        m.record_retry();
        let snap = m.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"total_sent\":1"));
        assert!(json.contains("\"total_failed\":1"));
        assert!(json.contains("\"total_retried\":1"));
    }
}
