//! Unified event types shared across learning subsystems.
//!
//! These events are intentionally lightweight and provider-agnostic so runtime
//! components can publish one stream that downstream learning systems consume.

use crate::anomaly::Anomaly;
use crate::provider_health::ErrorClass;
use roko_agent::Usage;
use roko_agent::chat_types::FinishReason;
use tokio::sync::broadcast;

/// A chunk from a streaming response, used for learning feedback.
#[derive(Clone, Debug)]
pub struct StreamChunk {
    /// Delta text content in this chunk.
    pub delta: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
}

/// Canonical event payload emitted by the learning/runtime feedback pipeline.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum AgentEvent {
    TurnStarted {
        task_id: String,
        model: String,
        provider: String,
        timestamp_ms: i64,
        /// `true` when the operator forced a specific model via `--model`,
        /// `--force-model`, or `--force-backend`. Override dispatches are
        /// excluded from LinUCB bandit updates to prevent user overrides
        /// from corrupting learned routing weights (UX34 / P4-9).
        is_model_override: bool,
    },
    ToolCallExecuted {
        tool_name: String,
        duration_ms: u64,
        success: bool,
        result_tokens: u64,
    },
    TurnCompleted {
        turn: u32,
        usage: Usage,
        tool_call_count: usize,
        gate_passed: Option<bool>,
        finish_reason: FinishReason,
    },
    GateResult {
        gate_name: String,
        passed: bool,
        score: f32,
        duration_ms: u64,
        /// Task that triggered this gate run; used to correlate with the
        /// buffered efficiency event so `gate_passed` can be set correctly.
        task_id: String,
    },
    ProviderError {
        provider_id: String,
        error_class: ErrorClass,
        status: u16,
    },
    CostRecorded {
        model: String,
        provider: String,
        cost_usd: f64,
        tokens: u64,
    },
    AnomalyDetected {
        anomaly: Anomaly,
    },
    ExperimentAssigned {
        experiment_id: String,
        variant_id: String,
    },
    SessionEstablished {
        session_id: String,
        provider: String,
    },
    ModelSelected {
        model: String,
        stage: String,
        score: f64,
    },
    SomaticMarkerFired {
        task_id: String,
        valence: f64,
        intensity: f64,
        source_episode_count: usize,
    },
    StreamChunk {
        chunk: StreamChunk,
    },
    /// Safety layer denied a tool call before execution.
    ///
    /// The `denial_reason` must contain only the policy reason (e.g. "tool
    /// `write_file` is not allowed for role `reviewer`"); it must never
    /// include agent output or user data.
    SafetyDenial {
        /// Name of the tool that was denied.
        tool_name: String,
        /// Policy reason for the denial (sanitized).
        denial_reason: String,
        /// Task identifier at the time of denial.
        task_id: String,
        /// Epoch milliseconds at the time of denial, matching the convention
        /// used by [`AgentEvent::TurnStarted::timestamp_ms`].
        timestamp: i64,
    },
}

/// Pub/sub bus for broadcasting learning events to multiple subscribers.
pub struct EventBus {
    tx: broadcast::Sender<AgentEvent>,
}

impl EventBus {
    /// Create a new event bus with the given broadcast channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish an event to all live subscribers.
    ///
    /// If there are no subscribers, the event is dropped.
    pub fn publish(&self, event: AgentEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to future events from this bus.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.tx.subscribe()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentEvent, EventBus};

    #[tokio::test]
    async fn event_bus_broadcasts_to_multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut first = bus.subscribe();
        let mut second = bus.subscribe();

        bus.publish(AgentEvent::SessionEstablished {
            session_id: "session-123".into(),
            provider: "zai".into(),
        });

        let first_event = first
            .recv()
            .await
            .expect("first subscriber should receive event");
        let second_event = second
            .recv()
            .await
            .expect("second subscriber should receive event");

        assert!(matches!(
            first_event,
            AgentEvent::SessionEstablished {
                session_id,
                provider,
            } if session_id == "session-123" && provider == "zai"
        ));
        assert!(matches!(
            second_event,
            AgentEvent::SessionEstablished {
                session_id,
                provider,
            } if session_id == "session-123" && provider == "zai"
        ));
    }

    #[test]
    fn event_bus_publish_with_no_subscribers_does_not_error() {
        let bus = EventBus::new(16);
        bus.publish(AgentEvent::SessionEstablished {
            session_id: "session-123".into(),
            provider: "zai".into(),
        });
    }
}
