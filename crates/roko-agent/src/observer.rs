//! Observer hooks for LLM inference calls.

use roko_core::RuntimeEvent;

/// Receives lifecycle notifications around a model inference request.
///
/// Implementations must be non-blocking. If they need async work, they should
/// enqueue internally and return immediately.
pub trait InferenceObserver: Send + Sync {
    /// Publish a non-inference runtime event after its durable write.
    ///
    /// The default keeps existing inference-only observers source compatible.
    fn on_runtime_event_with_cursor(&self, _event: &RuntimeEvent, _cursor: Option<u64>) {}

    /// Called immediately before a backend inference request starts.
    fn on_start(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        auto_routed: bool,
    );

    /// Cursor-aware variant used when the producer persisted before publish.
    fn on_start_with_cursor(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        auto_routed: bool,
        _cursor: Option<u64>,
    ) {
        self.on_start(run_id, request_id, model, agent_id, auto_routed);
    }

    /// Called after a backend inference request completes successfully.
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
    );

    /// Cursor-aware variant used when the producer persisted before publish.
    #[allow(clippy::too_many_arguments)]
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
        _cursor: Option<u64>,
    ) {
        self.on_complete(
            run_id,
            request_id,
            model,
            agent_id,
            input_tokens,
            output_tokens,
            cost_usd,
            duration_ms,
        );
    }

    /// Called after a backend inference request fails.
    fn on_error(&self, run_id: &str, request_id: &str, model: &str, agent_id: &str, error: &str);

    /// Cursor-aware variant used when the producer persisted before publish.
    fn on_error_with_cursor(
        &self,
        run_id: &str,
        request_id: &str,
        model: &str,
        agent_id: &str,
        error: &str,
        _cursor: Option<u64>,
    ) {
        self.on_error(run_id, request_id, model, agent_id, error);
    }
}

/// No-op observer for call sites that do not have an event pipeline.
#[derive(Debug, Default)]
pub struct NoopInferenceObserver;

impl InferenceObserver for NoopInferenceObserver {
    fn on_start(
        &self,
        _run_id: &str,
        _request_id: &str,
        _model: &str,
        _agent_id: &str,
        _auto_routed: bool,
    ) {
    }

    fn on_complete(
        &self,
        _run_id: &str,
        _request_id: &str,
        _model: &str,
        _agent_id: &str,
        _input_tokens: u64,
        _output_tokens: u64,
        _cost_usd: f64,
        _duration_ms: u64,
    ) {
    }

    fn on_error(
        &self,
        _run_id: &str,
        _request_id: &str,
        _model: &str,
        _agent_id: &str,
        _error: &str,
    ) {
    }
}
