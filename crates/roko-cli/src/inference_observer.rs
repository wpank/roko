//! CLI inference observer that publishes canonical RuntimeEvents.

use roko_agent::InferenceObserver;
use roko_core::RuntimeEvent;

/// Emits inference lifecycle events onto the shared runtime event bus.
#[derive(Debug, Default)]
pub struct RuntimeEventInferenceObserver;

impl RuntimeEventInferenceObserver {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl InferenceObserver for RuntimeEventInferenceObserver {
    fn on_runtime_event_with_cursor(&self, event: &RuntimeEvent, cursor: Option<u64>) {
        roko_runtime::event_bus::emit_runtime_event_with_cursor(event.clone(), cursor);
    }

    fn on_start(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        auto_routed: bool,
    ) {
        self.on_start_with_cursor(run_id, request_id, model, agent_id, auto_routed, None);
    }

    fn on_start_with_cursor(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        auto_routed: bool,
        cursor: Option<u64>,
    ) {
        roko_runtime::event_bus::emit_runtime_event_with_cursor(
            RuntimeEvent::InferenceStarted {
                run_id: run_id.to_string(),
                request_id: request_id.to_string(),
                model: model.to_string(),
                agent_id: agent_id.to_string(),
                auto_routed,
            },
            cursor,
        );
    }

    fn on_complete(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration_ms: u64,
    ) {
        self.on_complete_with_cursor(
            run_id,
            request_id,
            model,
            agent_id,
            input_tokens,
            output_tokens,
            cost_usd,
            duration_ms,
            None,
        );
    }

    fn on_complete_with_cursor(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration_ms: u64,
        cursor: Option<u64>,
    ) {
        roko_runtime::event_bus::emit_runtime_event_with_cursor(
            RuntimeEvent::InferenceCompleted {
                run_id: run_id.to_string(),
                request_id: request_id.to_string(),
                model: model.to_string(),
                agent_id: agent_id.to_string(),
                input_tokens,
                output_tokens,
                cost_usd,
                duration_ms,
            },
            cursor,
        );
    }

    fn on_error(&self, run_id: &str, request_id: &str, model: &str, agent_id: &str, error: &str) {
        self.on_error_with_cursor(run_id, request_id, model, agent_id, error, None);
    }

    fn on_error_with_cursor(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        error: &str,
        cursor: Option<u64>,
    ) {
        roko_runtime::event_bus::emit_runtime_event_with_cursor(
            RuntimeEvent::InferenceFailed {
                run_id: run_id.to_string(),
                request_id: request_id.to_string(),
                model: model.to_string(),
                agent_id: agent_id.to_string(),
                error: error.to_string(),
            },
            cursor,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_aware_observer_preserves_durable_cursor() {
        let bus = roko_runtime::event_bus::runtime_event_bus::<RuntimeEvent>();
        let start = bus.total_emitted();
        let observer = RuntimeEventInferenceObserver::new();

        observer.on_start_with_cursor(
            "observer-cursor-test",
            "request-cursor-test",
            "test-model",
            "agent-test",
            false,
            Some(128),
        );

        let event = bus
            .replay_from(start)
            .into_iter()
            .find(|event| {
                matches!(
                    &event.payload,
                    RuntimeEvent::InferenceStarted { request_id, .. }
                        if request_id == "request-cursor-test"
                )
            })
            .expect("cursor-aware inference event");
        assert_eq!(event.cursor, Some(128));

        observer.on_runtime_event_with_cursor(
            &RuntimeEvent::AgentTrace {
                run_id: "observer-cursor-test".to_string(),
                agent_id: "agent-test".to_string(),
                turn: 1,
                tool_calls: Vec::new(),
                reasoning: None,
                usage: roko_core::foundation::TokenUsage::default(),
            },
            Some(256),
        );
        let trace = bus
            .replay_from(start)
            .into_iter()
            .find(|event| {
                matches!(
                    &event.payload,
                    RuntimeEvent::AgentTrace { agent_id, .. } if agent_id == "agent-test"
                )
            })
            .expect("cursor-aware trace event");
        assert_eq!(trace.cursor, Some(256));
    }
}
