//! RAII guard that ensures `TraceSink::finish` is called on every terminal
//! path (success, error, cancel, panic).
//!
//! The runner's event loop tracks active trace IDs in a `HashMap` and
//! closes them on `TurnCompleted`. But other terminal paths (agent crash,
//! cancellation, timeout, panic) can skip the close, leaving traces
//! unflushed. The [`TraceFinishGuard`] ensures that any remaining active
//! traces are flushed when it is dropped.
//!
//! # Usage
//!
//! ```ignore
//! let guard = TraceFinishGuard::new(sink.clone(), role, model, format);
//! let trace_id = guard.begin();
//! // ... run agent ...
//! guard.close(trace_id); // normal close
//! // if dropped without close, Drop impl flushes all open traces
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use super::format::ToolFormat;
use super::trace::{FailureKind, ToolOutcome, ToolTrace, ToolTraceEvent, TraceId, TraceSink};
use crate::AgentRole;

/// RAII guard that tracks active trace IDs and ensures they are all
/// flushed to the sink on drop.
pub struct TraceFinishGuard {
    sink: Arc<dyn TraceSink>,
    active: Mutex<HashMap<TraceId, TraceState>>,
    role: AgentRole,
    model: String,
    format: ToolFormat,
}

struct TraceState {
    started_at_ms: i64,
    events: Vec<ToolTraceEvent>,
}

impl TraceFinishGuard {
    /// Create a new guard backed by the given sink.
    pub fn new(
        sink: Arc<dyn TraceSink>,
        role: AgentRole,
        model: String,
        format: ToolFormat,
    ) -> Self {
        Self {
            sink,
            active: Mutex::new(HashMap::new()),
            role,
            model,
            format,
        }
    }

    /// Begin tracking a trace ID. Returns the ID for correlation.
    pub fn begin(&self, trace_id: TraceId) {
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.active.lock().insert(
            trace_id,
            TraceState {
                started_at_ms: now_ms,
                events: Vec::new(),
            },
        );
    }

    /// Append an event to a tracked trace.
    pub fn append(&self, trace_id: TraceId, event: ToolTraceEvent) {
        self.sink.append(trace_id, event.clone());
        if let Some(state) = self.active.lock().get_mut(&trace_id) {
            state.events.push(event);
        }
    }

    /// Close a trace normally (removes it from the active set).
    /// The caller is responsible for having called `sink.finish()` or
    /// providing the terminal trace data.
    pub fn close(&self, trace_id: TraceId) {
        self.active.lock().remove(&trace_id);
    }

    /// Close a trace with a full `ToolTrace` (calls `sink.finish()`).
    pub fn finish(&self, trace: ToolTrace) {
        let id = trace.trace_id;
        self.sink.finish(trace);
        self.active.lock().remove(&id);
    }

    /// Number of currently active (unfinished) traces.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.lock().len()
    }

    /// Whether all traces have been properly closed.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.active.lock().is_empty()
    }

    /// Flush all remaining active traces as aborted. This is called by
    /// `Drop` but can also be called explicitly.
    pub fn flush_remaining(&self) {
        let mut active = self.active.lock();
        let entries: Vec<(TraceId, TraceState)> = active.drain().collect();
        drop(active); // Release lock before calling sink.

        let now_ms = chrono::Utc::now().timestamp_millis();
        for (trace_id, state) in entries {
            let trace = ToolTrace {
                trace_id,
                call_id: String::new(),
                role: self.role,
                model: self.model.clone(),
                format_used: self.format.clone(),
                started_at_ms: state.started_at_ms,
                ended_at_ms: now_ms,
                events: state.events,
                outcome: ToolOutcome::failure(FailureKind::Cancelled, 0, 0.0),
                enforcement_owner: None,
                policy_owner: None,
            };
            self.sink.finish(trace);
        }
    }
}

impl Drop for TraceFinishGuard {
    fn drop(&mut self) {
        self.flush_remaining();
    }
}

impl std::fmt::Debug for TraceFinishGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceFinishGuard")
            .field("active_count", &self.active_count())
            .field("role", &self.role)
            .field("model", &self.model)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use parking_lot::Mutex as ParkMutex;

    #[derive(Default, Clone)]
    struct TestSink {
        appends: Arc<ParkMutex<Vec<(TraceId, ToolTraceEvent)>>>,
        finishes: Arc<ParkMutex<Vec<ToolTrace>>>,
    }

    impl TraceSink for TestSink {
        fn append(&self, id: TraceId, e: ToolTraceEvent) {
            self.appends.lock().push((id, e));
        }
        fn finish(&self, t: ToolTrace) {
            self.finishes.lock().push(t);
        }
    }

    fn trace_id(b: u8) -> TraceId {
        TraceId::from_bytes([b; 16])
    }

    #[test]
    fn guard_flushes_on_drop() {
        let sink = TestSink::default();
        let finishes = sink.finishes.clone();

        {
            let guard = TraceFinishGuard::new(
                Arc::new(sink),
                AgentRole::Implementer,
                "test-model".into(),
                ToolFormat::OpenAiJson,
            );
            guard.begin(trace_id(0x01));
            guard.begin(trace_id(0x02));
            // Drop without closing.
        }

        assert_eq!(
            finishes.lock().len(),
            2,
            "both traces should be flushed on drop"
        );
    }

    #[test]
    fn closed_traces_not_flushed_on_drop() {
        let sink = TestSink::default();
        let finishes = sink.finishes.clone();

        {
            let guard = TraceFinishGuard::new(
                Arc::new(sink),
                AgentRole::Implementer,
                "test-model".into(),
                ToolFormat::OpenAiJson,
            );
            guard.begin(trace_id(0x01));
            guard.begin(trace_id(0x02));
            guard.close(trace_id(0x01));
            // 0x02 left open.
        }

        let finished = finishes.lock();
        assert_eq!(
            finished.len(),
            1,
            "only the unclosed trace should be flushed"
        );
        assert_eq!(finished[0].trace_id, trace_id(0x02));
    }

    #[test]
    fn finish_calls_sink_finish_and_removes() {
        let sink = TestSink::default();
        let finishes = sink.finishes.clone();

        let guard = TraceFinishGuard::new(
            Arc::new(sink),
            AgentRole::Implementer,
            "test-model".into(),
            ToolFormat::OpenAiJson,
        );
        guard.begin(trace_id(0x01));

        let trace = ToolTrace {
            trace_id: trace_id(0x01),
            call_id: "c1".into(),
            role: AgentRole::Implementer,
            model: "test".into(),
            format_used: ToolFormat::OpenAiJson,
            started_at_ms: 1000,
            ended_at_ms: 2000,
            events: vec![],
            outcome: ToolOutcome::success(10, 0.0),
            enforcement_owner: None,
            policy_owner: None,
        };
        guard.finish(trace);

        assert!(guard.is_clean());
        assert_eq!(finishes.lock().len(), 1);

        // Drop should not double-finish.
        drop(guard);
        assert_eq!(finishes.lock().len(), 1);
    }

    #[test]
    fn append_forwards_to_sink_and_tracks() {
        let sink = TestSink::default();
        let appends = sink.appends.clone();

        let guard = TraceFinishGuard::new(
            Arc::new(sink),
            AgentRole::Implementer,
            "test-model".into(),
            ToolFormat::OpenAiJson,
        );
        let id = trace_id(0x01);
        guard.begin(id);
        guard.append(id, ToolTraceEvent::StreamCoerced { at_ms: 100 });

        assert_eq!(appends.lock().len(), 1);
    }

    #[test]
    fn active_count_and_is_clean() {
        let guard = TraceFinishGuard::new(
            Arc::new(super::super::trace::NoopTraceSink),
            AgentRole::Implementer,
            "m".into(),
            ToolFormat::OpenAiJson,
        );
        assert!(guard.is_clean());
        assert_eq!(guard.active_count(), 0);

        guard.begin(trace_id(0x01));
        assert_eq!(guard.active_count(), 1);
        assert!(!guard.is_clean());

        guard.close(trace_id(0x01));
        assert!(guard.is_clean());
    }

    #[test]
    fn flush_remaining_explicit() {
        let sink = TestSink::default();
        let finishes = sink.finishes.clone();

        let guard = TraceFinishGuard::new(
            Arc::new(sink),
            AgentRole::Implementer,
            "test-model".into(),
            ToolFormat::OpenAiJson,
        );
        guard.begin(trace_id(0x01));
        guard.begin(trace_id(0x02));

        guard.flush_remaining();
        assert_eq!(finishes.lock().len(), 2);
        assert!(guard.is_clean());

        // Second call is idempotent.
        guard.flush_remaining();
        assert_eq!(finishes.lock().len(), 2);
    }
}
