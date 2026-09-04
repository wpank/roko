//! Serve-layer bridge for canonical runtime event projection (#248).
//!
//! [`RuntimeEventBridge`] wraps the shared
//! [`RuntimeEventDashboardProjector`] from `roko-runtime` and forwards the
//! same canonical [`RuntimeEventEnvelope`] to SSE/HTTP adapters. It does
//! **not** maintain another variant match -- projection is delegated entirely
//! to the underlying projector.
//!
//! # Usage
//!
//! ```ignore
//! let bridge = RuntimeEventBridge::new(projector, sse_adapter);
//! // For each incoming canonical envelope:
//! bridge.forward(&envelope);
//! ```
//!
//! [`RuntimeEventDashboardProjector`]: roko_runtime::runtime_event_dashboard::RuntimeEventDashboardProjector
//! [`RuntimeEventEnvelope`]: roko_core::runtime_event::RuntimeEventEnvelope

use std::sync::Arc;

use roko_core::runtime_event::RuntimeEventEnvelope;
use roko_runtime::runtime_event_dashboard::{ProjectionResult, RuntimeEventDashboardProjector};
use tracing::{debug, trace, warn};

use crate::adapters::SseAdapter;

// ---------------------------------------------------------------------------
// Bridge
// ---------------------------------------------------------------------------

/// Serve-layer bridge that projects canonical runtime events into the
/// dashboard and SSE channel.
///
/// The bridge owns an `Arc<RuntimeEventDashboardProjector>` and an
/// `Arc<SseAdapter>`. It delegates projection to the projector and
/// forwards the resulting `DashboardEvent` values to SSE subscribers
/// alongside the raw envelope summary.
pub struct RuntimeEventBridge {
    /// Shared canonical projector.
    projector: Arc<RuntimeEventDashboardProjector>,
    /// SSE adapter for broadcasting events to connected clients.
    sse: Arc<SseAdapter>,
}

impl RuntimeEventBridge {
    /// Create a new bridge.
    #[must_use]
    pub fn new(projector: Arc<RuntimeEventDashboardProjector>, sse: Arc<SseAdapter>) -> Self {
        Self { projector, sse }
    }

    /// Forward a canonical envelope through the projection pipeline.
    ///
    /// 1. The projector maps the envelope to `DashboardEvent` values
    ///    (with dedup and terminal conflict rejection).
    /// 2. Each `DashboardEvent` is sent to the SSE adapter as a summary.
    /// 3. The raw envelope is also forwarded to SSE for clients that
    ///    consume the full schema.
    ///
    /// Returns the projection result for the caller to inspect.
    pub fn forward(&self, envelope: &RuntimeEventEnvelope) -> ProjectionResult {
        let result = self.projector.project(envelope);

        match &result {
            ProjectionResult::Projected(dashboard_events) => {
                // Forward envelope summary to SSE.
                self.emit_sse(envelope);

                trace!(
                    event_id = %envelope.event_id,
                    dashboard_events = dashboard_events.len(),
                    "projected runtime event to dashboard"
                );
            }
            ProjectionResult::Duplicate => {
                debug!(
                    event_id = %envelope.event_id,
                    "skipped duplicate runtime event"
                );
            }
            ProjectionResult::ConflictingTerminal => {
                warn!(
                    event_id = %envelope.event_id,
                    run_id = %envelope.run_id,
                    "rejected conflicting terminal event"
                );
            }
            ProjectionResult::NoProjection => {
                trace!(
                    event_id = %envelope.event_id,
                    kind = %envelope.payload.kind(),
                    "no dashboard projection for event"
                );
            }
        }

        result
    }

    /// Forward a batch of envelopes. Returns the count of successfully
    /// projected envelopes.
    pub fn forward_batch(
        &self,
        envelopes: impl IntoIterator<Item = RuntimeEventEnvelope>,
    ) -> usize {
        let mut projected = 0;
        for envelope in envelopes {
            if matches!(self.forward(&envelope), ProjectionResult::Projected(_)) {
                projected += 1;
            }
        }
        projected
    }

    /// Access the underlying projector.
    #[must_use]
    pub fn projector(&self) -> &RuntimeEventDashboardProjector {
        &self.projector
    }

    /// Access the underlying SSE adapter.
    #[must_use]
    pub fn sse(&self) -> &SseAdapter {
        &self.sse
    }

    /// Emit the envelope through the existing SSE adapter pipeline.
    ///
    /// Uses `consume_envelope` which translates the `RuntimeEvent` payload
    /// through the adapter's existing `to_sse_event` path, preserving the
    /// same SSE schema that standalone producers use.
    fn emit_sse(&self, envelope: &RuntimeEventEnvelope) {
        self.sse.consume_envelope(envelope);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use roko_core::runtime_event::{RuntimeEvent, RuntimeEventEnvelope, RuntimeEventMode};

    use super::*;

    fn test_envelope(event_id: &str, payload: RuntimeEvent) -> RuntimeEventEnvelope {
        RuntimeEventEnvelope {
            event_id: event_id.to_string(),
            run_id: "run-1".to_string(),
            plan_id: Some("plan-1".to_string()),
            task_id: Some("task-1".to_string()),
            node_id: None,
            attempt_id: None,
            agent_id: None,
            seq: 1,
            ts: Utc::now(),
            schema_version: 2,
            source: "graph".to_string(),
            mode: RuntimeEventMode::Live,
            delivery: payload.delivery(),
            correlation_id: None,
            idempotency_key: None,
            payload,
        }
    }

    fn make_bridge() -> RuntimeEventBridge {
        let projector = Arc::new(RuntimeEventDashboardProjector::new());
        let sse = Arc::new(SseAdapter::new(64));
        RuntimeEventBridge::new(projector, sse)
    }

    #[test]
    fn forward_projects_and_emits_sse() {
        let bridge = make_bridge();
        let mut rx = bridge.sse().subscribe();

        let envelope = test_envelope(
            "fwd-1",
            RuntimeEvent::RunStarted {
                run_id: "run-1".to_string(),
                prompt: String::new(),
                complexity: "graph".to_string(),
            },
        );

        let result = bridge.forward(&envelope);
        assert!(matches!(result, ProjectionResult::Projected(_)));

        // SSE should have received the event.
        let sse_event = rx.try_recv().expect("should have received SSE event");
        assert_eq!(sse_event.kind, "run_started");
        assert_eq!(sse_event.run_id, "run-1");
    }

    #[test]
    fn duplicate_does_not_emit_sse() {
        let bridge = make_bridge();
        let mut rx = bridge.sse().subscribe();

        let envelope = test_envelope(
            "dup-sse-1",
            RuntimeEvent::RunStarted {
                run_id: "run-1".to_string(),
                prompt: String::new(),
                complexity: "graph".to_string(),
            },
        );

        bridge.forward(&envelope);
        let _ = rx.try_recv(); // consume first

        let result = bridge.forward(&envelope);
        assert!(matches!(result, ProjectionResult::Duplicate));
        assert!(rx.try_recv().is_err(), "duplicate should not emit SSE");
    }

    #[test]
    fn forward_batch_counts_projected() {
        let bridge = make_bridge();
        let envelopes = vec![
            test_envelope(
                "batch-1",
                RuntimeEvent::RunStarted {
                    run_id: "run-1".to_string(),
                    prompt: String::new(),
                    complexity: "graph".to_string(),
                },
            ),
            test_envelope(
                "batch-1", // duplicate
                RuntimeEvent::RunStarted {
                    run_id: "run-1".to_string(),
                    prompt: String::new(),
                    complexity: "graph".to_string(),
                },
            ),
            test_envelope(
                "batch-2",
                RuntimeEvent::TaskStarted {
                    run_id: "run-1".to_string(),
                    plan_id: "plan-1".to_string(),
                    task_id: "compile".to_string(),
                    task_title: "Compile".to_string(),
                    role: "impl".to_string(),
                },
            ),
        ];

        let projected = bridge.forward_batch(envelopes);
        assert_eq!(projected, 2, "should project 2 of 3 (one duplicate)");
    }

    #[test]
    fn conflicting_terminal_does_not_emit_sse() {
        let bridge = make_bridge();
        let mut rx = bridge.sse().subscribe();

        let first = test_envelope(
            "ct-1",
            RuntimeEvent::RunCompleted {
                run_id: "run-1".to_string(),
                success: true,
                cost_usd: 0.0,
                duration_ms: 1000,
            },
        );
        let second = test_envelope(
            "ct-2",
            RuntimeEvent::RunCompleted {
                run_id: "run-1".to_string(),
                success: false,
                cost_usd: 0.0,
                duration_ms: 2000,
            },
        );

        bridge.forward(&first);
        let _ = rx.try_recv(); // consume first

        let result = bridge.forward(&second);
        assert!(matches!(result, ProjectionResult::ConflictingTerminal));
        assert!(
            rx.try_recv().is_err(),
            "conflicting terminal should not emit SSE"
        );
    }
}
