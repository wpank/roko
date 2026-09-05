//! Source adapters that normalize events from different execution
//! sources into canonical [`TranscriptRecord`]s (Packet B — T004/T008/T009/T016/T024).
//!
//! Each adapter converts raw events from one source — runtime callbacks,
//! provider streams, CLI JSONL, ACP protocol messages, or remote relay
//! envelopes — into the same `TranscriptRecord` sequence. Downstream
//! stores, UIs, and telemetry consumers never branch on source type.
//!
//! # Adapter trait
//!
//! [`SourceAdapter`] defines the normalization contract. Every implementation
//! must produce the same `TranscriptRecord` shape for semantically equivalent
//! inputs, as verified by the conformance fixture in this module's tests.
//!
//! # Provided adapters
//!
//! | Adapter | Source |
//! |---|---|
//! | [`RuntimeAdapter`] | In-process runtime event callbacks |
//! | [`ProviderStreamAdapter`] | Provider SSE/streaming API events |
//! | [`CliJsonlAdapter`] | CLI subprocess JSONL output |
//! | [`AcpAdapter`] | ACP (Agent Client Protocol) messages |
//! | [`RemoteRelayAdapter`] | Remote relay envelope delivery |

use serde::{Deserialize, Serialize};

use super::call::{ToolCall, ToolError, ToolResult};
use super::def::ToolCategory;
use super::transcript::{
    ToolLifecycleStatus, TranscriptEvent, TranscriptEventMeta, TranscriptRecord,
};

// ─── Source kind tag ────────────────────────────────────────────────────

/// Identifies which execution source produced a transcript event.
///
/// Attached to records so downstream consumers can distinguish provenance
/// without parsing the event payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSourceKind {
    /// In-process runtime callbacks (runner event loop).
    Runtime,
    /// Direct provider API stream (Anthropic, OpenAI, Gemini, etc.).
    ProviderStream,
    /// CLI subprocess JSONL output (Claude CLI, Codex CLI).
    CliJsonl,
    /// ACP (Agent Client Protocol) messages.
    Acp,
    /// Remote relay envelope delivery.
    RemoteRelay,
}

impl EventSourceKind {
    /// Stable string tag for logs and metrics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::ProviderStream => "provider_stream",
            Self::CliJsonl => "cli_jsonl",
            Self::Acp => "acp",
            Self::RemoteRelay => "remote_relay",
        }
    }
}

// ─── Raw source event ───────────────────────────────────────────────────

/// A raw event from any source, before normalization.
///
/// Each variant carries the minimum fields the adapter needs to produce
/// a canonical `TranscriptRecord`. Provider-specific metadata is preserved
/// as opaque JSON for debugging but never leaks into the canonical model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum RawSourceEvent {
    /// Text delta from any source.
    TextDelta {
        text: String,
        /// Whether this is reasoning/thinking output vs. assistant output.
        is_reasoning: bool,
    },

    /// A tool call was received.
    ToolCallReceived {
        call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
        category: Option<ToolCategory>,
    },

    /// A tool call completed.
    ToolCallCompleted {
        call_id: String,
        /// Text content of the result (if success).
        result_text: Option<String>,
        /// Error message (if failure).
        error: Option<String>,
        /// Terminal status.
        status: ToolLifecycleStatus,
        /// Execution time in milliseconds.
        execution_ms: Option<u64>,
    },

    /// Token usage report.
    UsageReport {
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
        cost_usd: Option<f64>,
    },

    /// Provider or model changed mid-run.
    ProviderChanged {
        from_provider: Option<String>,
        to_provider: String,
        from_model: Option<String>,
        to_model: String,
        reason: String,
    },

    /// Run started.
    RunStarted {
        system_prompt_hash: Option<String>,
        tools_offered: u32,
    },

    /// Run finished.
    RunFinished {
        success: bool,
        total_turns: u32,
        total_tool_calls: u32,
        wall_ms: u64,
    },

    /// Warning.
    Warning { code: String, message: String },

    /// Error.
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },

    /// Subagent started.
    SubagentStarted { subagent_id: String, task: String },

    /// Subagent update.
    SubagentUpdate {
        subagent_id: String,
        payload: serde_json::Value,
    },

    /// Subagent finished.
    SubagentFinished {
        subagent_id: String,
        success: bool,
        summary: Option<String>,
    },

    /// Todo/task snapshot.
    TodoSnapshot { items: serde_json::Value },
}

// ─── Adapter context ────────────────────────────────────────────────────

/// Shared context for building transcript metadata.
///
/// Adapters increment `next_sequence` atomically; callers supply the
/// run/agent/provider identity.
#[derive(Debug, Clone)]
pub struct AdapterContext {
    pub run_id: String,
    pub agent_id: String,
    pub provider: String,
    pub model: String,
    pub task_id: Option<String>,
    pub attempt_id: Option<String>,
    next_sequence: u64,
    turn_id: u32,
}

impl AdapterContext {
    /// Create a new adapter context.
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        agent_id: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            agent_id: agent_id.into(),
            provider: provider.into(),
            model: model.into(),
            task_id: None,
            attempt_id: None,
            next_sequence: 0,
            turn_id: 0,
        }
    }

    /// Set task and attempt correlation IDs.
    pub fn with_correlation(
        mut self,
        task_id: impl Into<String>,
        attempt_id: impl Into<String>,
    ) -> Self {
        self.task_id = Some(task_id.into());
        self.attempt_id = Some(attempt_id.into());
        self
    }

    /// Update the active provider/model (for fallback tracking).
    pub fn set_provider(&mut self, provider: impl Into<String>, model: impl Into<String>) {
        self.provider = provider.into();
        self.model = model.into();
    }

    /// Advance to the next turn.
    pub fn advance_turn(&mut self) {
        self.turn_id += 1;
    }

    /// Build metadata for the next event.
    fn next_meta(&mut self, timestamp_ms: i64) -> TranscriptEventMeta {
        let seq = self.next_sequence;
        self.next_sequence += 1;
        TranscriptEventMeta {
            run_id: self.run_id.clone(),
            turn_id: self.turn_id,
            agent_id: self.agent_id.clone(),
            sequence: seq,
            timestamp_ms,
            provider: self.provider.clone(),
            model: self.model.clone(),
            parent_event_id: None,
            task_id: self.task_id.clone(),
            attempt_id: self.attempt_id.clone(),
        }
    }
}

// ─── SourceAdapter trait ────────────────────────────────────────────────

/// Normalize a raw source event into a canonical transcript record.
///
/// Every adapter must produce semantically equivalent `TranscriptRecord`s
/// for equivalent inputs, regardless of the source. The conformance tests
/// in this module verify this property.
pub trait SourceAdapter {
    /// Which source kind this adapter handles.
    fn source_kind(&self) -> EventSourceKind;

    /// Normalize a raw event into a canonical transcript record.
    fn normalize(
        &self,
        event: &RawSourceEvent,
        ctx: &mut AdapterContext,
        timestamp_ms: i64,
    ) -> TranscriptRecord;
}

// ─── Shared normalization logic ─────────────────────────────────────────

/// Convert a `RawSourceEvent` into a `TranscriptEvent`.
///
/// This is the shared kernel: all adapters produce the same event for the
/// same raw input. Individual adapters may pre-process their wire formats
/// into `RawSourceEvent` differently, but the final mapping is identical.
fn raw_to_transcript_event(raw: &RawSourceEvent) -> TranscriptEvent {
    match raw {
        RawSourceEvent::TextDelta {
            text,
            is_reasoning: true,
        } => TranscriptEvent::ReasoningDelta { text: text.clone() },

        RawSourceEvent::TextDelta {
            text,
            is_reasoning: false,
        } => TranscriptEvent::AssistantDelta { text: text.clone() },

        RawSourceEvent::ToolCallReceived {
            call_id,
            tool_name,
            arguments,
            category,
        } => TranscriptEvent::ToolStarted {
            call: ToolCall::at(call_id, tool_name, arguments.clone(), 0),
            status: ToolLifecycleStatus::Admitted,
            category: *category,
        },

        RawSourceEvent::ToolCallCompleted {
            call_id,
            result_text,
            error,
            status,
            execution_ms,
        } => {
            let result = if let Some(err) = error {
                ToolResult::err(ToolError::Other(err.clone()))
            } else {
                ToolResult::text(result_text.as_deref().unwrap_or(""))
            };
            TranscriptEvent::ToolFinished {
                call_id: call_id.clone(),
                result,
                status: *status,
                execution_ms: *execution_ms,
            }
        }

        RawSourceEvent::UsageReport {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            cost_usd,
        } => TranscriptEvent::Usage {
            input_tokens: *input_tokens,
            output_tokens: *output_tokens,
            cache_read_tokens: *cache_read_tokens,
            cache_creation_tokens: *cache_creation_tokens,
            cost_usd: *cost_usd,
        },

        RawSourceEvent::ProviderChanged {
            from_provider,
            to_provider,
            from_model,
            to_model,
            reason,
        } => TranscriptEvent::ProviderChanged {
            from_provider: from_provider.clone(),
            to_provider: to_provider.clone(),
            from_model: from_model.clone(),
            to_model: to_model.clone(),
            reason: reason.clone(),
        },

        RawSourceEvent::RunStarted {
            system_prompt_hash,
            tools_offered,
        } => TranscriptEvent::RunStarted {
            system_prompt_hash: system_prompt_hash.clone(),
            tools_offered: *tools_offered,
        },

        RawSourceEvent::RunFinished {
            success,
            total_turns,
            total_tool_calls,
            wall_ms,
        } => TranscriptEvent::RunFinished {
            success: *success,
            total_turns: *total_turns,
            total_tool_calls: *total_tool_calls,
            wall_ms: *wall_ms,
        },

        RawSourceEvent::Warning { code, message } => TranscriptEvent::Warning {
            code: code.clone(),
            message: message.clone(),
        },

        RawSourceEvent::Error {
            code,
            message,
            recoverable,
        } => TranscriptEvent::Error {
            code: code.clone(),
            message: message.clone(),
            recoverable: *recoverable,
        },

        RawSourceEvent::SubagentStarted { subagent_id, task } => TranscriptEvent::SubagentStarted {
            subagent_id: subagent_id.clone(),
            task: task.clone(),
        },

        RawSourceEvent::SubagentUpdate {
            subagent_id,
            payload,
        } => TranscriptEvent::SubagentUpdate {
            subagent_id: subagent_id.clone(),
            payload: payload.clone(),
        },

        RawSourceEvent::SubagentFinished {
            subagent_id,
            success,
            summary,
        } => TranscriptEvent::SubagentFinished {
            subagent_id: subagent_id.clone(),
            success: *success,
            summary: summary.clone(),
        },

        RawSourceEvent::TodoSnapshot { items } => TranscriptEvent::TodoSnapshot {
            items: items.clone(),
        },
    }
}

// ─── Concrete adapters ──────────────────────────────────────────────────

/// Runtime adapter: normalizes in-process runtime event callbacks.
#[derive(Debug, Default)]
pub struct RuntimeAdapter;

impl SourceAdapter for RuntimeAdapter {
    fn source_kind(&self) -> EventSourceKind {
        EventSourceKind::Runtime
    }

    fn normalize(
        &self,
        event: &RawSourceEvent,
        ctx: &mut AdapterContext,
        timestamp_ms: i64,
    ) -> TranscriptRecord {
        TranscriptRecord {
            schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
            meta: ctx.next_meta(timestamp_ms),
            event: raw_to_transcript_event(event),
        }
    }
}

/// Provider stream adapter: normalizes SSE/streaming API events.
#[derive(Debug, Default)]
pub struct ProviderStreamAdapter;

impl SourceAdapter for ProviderStreamAdapter {
    fn source_kind(&self) -> EventSourceKind {
        EventSourceKind::ProviderStream
    }

    fn normalize(
        &self,
        event: &RawSourceEvent,
        ctx: &mut AdapterContext,
        timestamp_ms: i64,
    ) -> TranscriptRecord {
        TranscriptRecord {
            schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
            meta: ctx.next_meta(timestamp_ms),
            event: raw_to_transcript_event(event),
        }
    }
}

/// CLI JSONL adapter: normalizes Claude/Codex CLI subprocess output.
#[derive(Debug, Default)]
pub struct CliJsonlAdapter;

impl SourceAdapter for CliJsonlAdapter {
    fn source_kind(&self) -> EventSourceKind {
        EventSourceKind::CliJsonl
    }

    fn normalize(
        &self,
        event: &RawSourceEvent,
        ctx: &mut AdapterContext,
        timestamp_ms: i64,
    ) -> TranscriptRecord {
        TranscriptRecord {
            schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
            meta: ctx.next_meta(timestamp_ms),
            event: raw_to_transcript_event(event),
        }
    }
}

/// ACP adapter: normalizes Agent Client Protocol messages.
#[derive(Debug, Default)]
pub struct AcpAdapter;

impl SourceAdapter for AcpAdapter {
    fn source_kind(&self) -> EventSourceKind {
        EventSourceKind::Acp
    }

    fn normalize(
        &self,
        event: &RawSourceEvent,
        ctx: &mut AdapterContext,
        timestamp_ms: i64,
    ) -> TranscriptRecord {
        TranscriptRecord {
            schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
            meta: ctx.next_meta(timestamp_ms),
            event: raw_to_transcript_event(event),
        }
    }
}

/// Remote relay adapter: normalizes relay envelope delivery.
#[derive(Debug, Default)]
pub struct RemoteRelayAdapter;

impl SourceAdapter for RemoteRelayAdapter {
    fn source_kind(&self) -> EventSourceKind {
        EventSourceKind::RemoteRelay
    }

    fn normalize(
        &self,
        event: &RawSourceEvent,
        ctx: &mut AdapterContext,
        timestamp_ms: i64,
    ) -> TranscriptRecord {
        TranscriptRecord {
            schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
            meta: ctx.next_meta(timestamp_ms),
            event: raw_to_transcript_event(event),
        }
    }
}

// ─── Conformance fixture ────────────────────────────────────────────────

/// Build the standard conformance fixture: a sequence of raw events that
/// exercises every event type. All adapters must produce semantically
/// identical `TranscriptRecord.event` payloads for these inputs.
#[cfg(test)]
pub fn conformance_fixture() -> Vec<RawSourceEvent> {
    vec![
        RawSourceEvent::RunStarted {
            system_prompt_hash: Some("sha256:conformance".into()),
            tools_offered: 8,
        },
        RawSourceEvent::TextDelta {
            text: "Analyzing the codebase.".into(),
            is_reasoning: false,
        },
        RawSourceEvent::TextDelta {
            text: "Need to check imports first.".into(),
            is_reasoning: true,
        },
        RawSourceEvent::ToolCallReceived {
            call_id: "tc-001".into(),
            tool_name: "read_file".into(),
            arguments: serde_json::json!({"path": "src/lib.rs"}),
            category: Some(ToolCategory::Read),
        },
        RawSourceEvent::ToolCallCompleted {
            call_id: "tc-001".into(),
            result_text: Some("pub mod lib;".into()),
            error: None,
            status: ToolLifecycleStatus::Succeeded,
            execution_ms: Some(15),
        },
        RawSourceEvent::ToolCallReceived {
            call_id: "tc-002".into(),
            tool_name: "bash".into(),
            arguments: serde_json::json!({"command": "cargo test"}),
            category: Some(ToolCategory::Exec),
        },
        RawSourceEvent::ToolCallCompleted {
            call_id: "tc-002".into(),
            result_text: None,
            error: Some("exit code 1".into()),
            status: ToolLifecycleStatus::Failed,
            execution_ms: Some(3000),
        },
        RawSourceEvent::ToolCallReceived {
            call_id: "tc-003".into(),
            tool_name: "bash".into(),
            arguments: serde_json::json!({"command": "sleep 999"}),
            category: Some(ToolCategory::Exec),
        },
        RawSourceEvent::ToolCallCompleted {
            call_id: "tc-003".into(),
            result_text: None,
            error: Some("cancelled".into()),
            status: ToolLifecycleStatus::Cancelled,
            execution_ms: Some(500),
        },
        RawSourceEvent::SubagentStarted {
            subagent_id: "sa-001".into(),
            task: "research security".into(),
        },
        RawSourceEvent::SubagentUpdate {
            subagent_id: "sa-001".into(),
            payload: serde_json::json!({"progress": 0.5}),
        },
        RawSourceEvent::SubagentFinished {
            subagent_id: "sa-001".into(),
            success: true,
            summary: Some("Found 1 issue".into()),
        },
        RawSourceEvent::TodoSnapshot {
            items: serde_json::json!([{"id": "1", "text": "Fix bug", "done": false}]),
        },
        RawSourceEvent::ProviderChanged {
            from_provider: Some("anthropic".into()),
            to_provider: "openai".into(),
            from_model: Some("claude-opus-4-6".into()),
            to_model: "gpt-4o".into(),
            reason: "rate limited".into(),
        },
        RawSourceEvent::Warning {
            code: "TOOL_SLOW".into(),
            message: "bash took 3s".into(),
        },
        RawSourceEvent::Error {
            code: "PROVIDER_5XX".into(),
            message: "503 service unavailable".into(),
            recoverable: true,
        },
        RawSourceEvent::UsageReport {
            input_tokens: 2000,
            output_tokens: 1000,
            cache_read_tokens: 500,
            cache_creation_tokens: 100,
            cost_usd: Some(0.025),
        },
        RawSourceEvent::RunFinished {
            success: true,
            total_turns: 2,
            total_tool_calls: 3,
            wall_ms: 30_000,
        },
    ]
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> AdapterContext {
        AdapterContext::new(
            "run-conformance",
            "agent-test",
            "anthropic",
            "claude-opus-4-6",
        )
        .with_correlation("task-1", "attempt-0")
    }

    /// Run the conformance fixture through an adapter and return the records.
    fn run_conformance(adapter: &dyn SourceAdapter) -> Vec<TranscriptRecord> {
        let fixture = conformance_fixture();
        let mut ctx = test_context();
        let base_ts = 1_700_000_000_000i64;
        fixture
            .iter()
            .enumerate()
            .map(|(i, event)| adapter.normalize(event, &mut ctx, base_ts + (i as i64 * 100)))
            .collect()
    }

    /// Extract the event type tag from a transcript event.
    fn event_type_tag(event: &TranscriptEvent) -> String {
        let json = serde_json::to_value(event).unwrap();
        json.get("type").unwrap().as_str().unwrap().to_string()
    }

    // ── Core conformance: all adapters produce identical event payloads ──

    #[test]
    fn all_adapters_produce_same_event_payloads() {
        let adapters: Vec<Box<dyn SourceAdapter>> = vec![
            Box::new(RuntimeAdapter),
            Box::new(ProviderStreamAdapter),
            Box::new(CliJsonlAdapter),
            Box::new(AcpAdapter),
            Box::new(RemoteRelayAdapter),
        ];

        // Collect events from the first adapter as the reference.
        let reference = run_conformance(&*adapters[0]);

        for adapter in &adapters[1..] {
            let records = run_conformance(&**adapter);
            assert_eq!(
                records.len(),
                reference.len(),
                "{:?} produced different record count",
                adapter.source_kind()
            );

            for (i, (ref_rec, test_rec)) in reference.iter().zip(records.iter()).enumerate() {
                assert_eq!(
                    ref_rec.event,
                    test_rec.event,
                    "adapter {:?} diverged at event {i}: {:?} vs {:?}",
                    adapter.source_kind(),
                    event_type_tag(&ref_rec.event),
                    event_type_tag(&test_rec.event),
                );
            }
        }
    }

    #[test]
    fn conformance_fixture_covers_all_event_types() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        let types: std::collections::HashSet<String> =
            records.iter().map(|r| event_type_tag(&r.event)).collect();

        let expected = [
            "run_started",
            "assistant_delta",
            "reasoning_delta",
            "tool_started",
            "tool_finished",
            "subagent_started",
            "subagent_update",
            "subagent_finished",
            "todo_snapshot",
            "provider_changed",
            "warning",
            "error",
            "usage",
            "run_finished",
        ];
        for t in &expected {
            assert!(types.contains(*t), "missing event type: {t}");
        }
    }

    #[test]
    fn conformance_fixture_has_tool_lifecycle_coverage() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        // Check we have success, failure, and cancellation tool paths.
        let mut has_success = false;
        let mut has_failure = false;
        let mut has_cancel = false;

        for r in &records {
            if let TranscriptEvent::ToolFinished { status, .. } = &r.event {
                match status {
                    ToolLifecycleStatus::Succeeded => has_success = true,
                    ToolLifecycleStatus::Failed => has_failure = true,
                    ToolLifecycleStatus::Cancelled => has_cancel = true,
                    _ => {}
                }
            }
        }

        assert!(has_success, "fixture must include successful tool call");
        assert!(has_failure, "fixture must include failed tool call");
        assert!(has_cancel, "fixture must include cancelled tool call");
    }

    #[test]
    fn conformance_fixture_tool_start_finish_pairing() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        let mut started: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut finished: std::collections::HashSet<String> = std::collections::HashSet::new();

        for r in &records {
            match &r.event {
                TranscriptEvent::ToolStarted { call, .. } => {
                    started.insert(call.id.clone());
                }
                TranscriptEvent::ToolFinished { call_id, .. } => {
                    finished.insert(call_id.clone());
                }
                _ => {}
            }
        }

        assert_eq!(started.len(), 3, "3 tool calls expected");
        assert_eq!(
            started, finished,
            "every started tool must have a matching finish"
        );
    }

    #[test]
    fn conformance_fixture_subagent_lifecycle() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        let mut started = false;
        let mut updated = false;
        let mut finished = false;

        for r in &records {
            match &r.event {
                TranscriptEvent::SubagentStarted {
                    subagent_id: id, ..
                } if id == "sa-001" => started = true,
                TranscriptEvent::SubagentUpdate {
                    subagent_id: id, ..
                } if id == "sa-001" => updated = true,
                TranscriptEvent::SubagentFinished {
                    subagent_id: id, ..
                } if id == "sa-001" => finished = true,
                _ => {}
            }
        }

        assert!(started && updated && finished, "full subagent lifecycle");
    }

    #[test]
    fn sequences_are_monotonic() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        let seqs: Vec<u64> = records.iter().map(|r| r.meta.sequence).collect();
        for pair in seqs.windows(2) {
            assert!(
                pair[0] < pair[1],
                "sequences must be strictly increasing: {} >= {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn timestamps_are_non_decreasing() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        let times: Vec<i64> = records.iter().map(|r| r.meta.timestamp_ms).collect();
        for pair in times.windows(2) {
            assert!(
                pair[0] <= pair[1],
                "timestamps must be non-decreasing: {} > {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn correlation_fields_propagate() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        for r in &records {
            assert_eq!(r.meta.run_id, "run-conformance");
            assert_eq!(r.meta.agent_id, "agent-test");
            assert_eq!(r.meta.task_id.as_deref(), Some("task-1"));
            assert_eq!(r.meta.attempt_id.as_deref(), Some("attempt-0"));
        }
    }

    #[test]
    fn schema_version_is_current() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        for r in &records {
            assert_eq!(r.schema_version, TranscriptRecord::CURRENT_SCHEMA_VERSION);
        }
    }

    #[test]
    fn source_kind_tags_are_distinct() {
        let adapters: Vec<Box<dyn SourceAdapter>> = vec![
            Box::new(RuntimeAdapter),
            Box::new(ProviderStreamAdapter),
            Box::new(CliJsonlAdapter),
            Box::new(AcpAdapter),
            Box::new(RemoteRelayAdapter),
        ];

        let kinds: std::collections::HashSet<_> =
            adapters.iter().map(|a| a.source_kind()).collect();
        assert_eq!(
            kinds.len(),
            5,
            "each adapter must have a unique source kind"
        );
    }

    #[test]
    fn event_source_kind_as_str_stable() {
        assert_eq!(EventSourceKind::Runtime.as_str(), "runtime");
        assert_eq!(EventSourceKind::ProviderStream.as_str(), "provider_stream");
        assert_eq!(EventSourceKind::CliJsonl.as_str(), "cli_jsonl");
        assert_eq!(EventSourceKind::Acp.as_str(), "acp");
        assert_eq!(EventSourceKind::RemoteRelay.as_str(), "remote_relay");
    }

    #[test]
    fn event_source_kind_serde_roundtrip() {
        let kinds = [
            EventSourceKind::Runtime,
            EventSourceKind::ProviderStream,
            EventSourceKind::CliJsonl,
            EventSourceKind::Acp,
            EventSourceKind::RemoteRelay,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let decoded: EventSourceKind = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, kind);
        }
    }

    #[test]
    fn raw_source_event_serde_roundtrip() {
        let fixture = conformance_fixture();
        for (i, event) in fixture.iter().enumerate() {
            let json = serde_json::to_string(event).unwrap();
            let decoded: RawSourceEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(&decoded, event, "raw event {i} failed serde roundtrip");
        }
    }

    #[test]
    fn conformance_records_are_jsonl_roundtrippable() {
        let adapter = RuntimeAdapter;
        let records = run_conformance(&adapter);

        let jsonl: String = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n");

        let decoded: Vec<TranscriptRecord> = jsonl
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        assert_eq!(decoded, records, "JSONL roundtrip must be deterministic");
    }

    #[test]
    fn adapter_context_set_provider_updates_meta() {
        let mut ctx = test_context();
        let adapter = RuntimeAdapter;
        let event = RawSourceEvent::TextDelta {
            text: "before".into(),
            is_reasoning: false,
        };

        let r1 = adapter.normalize(&event, &mut ctx, 1000);
        assert_eq!(r1.meta.provider, "anthropic");

        ctx.set_provider("openai", "gpt-4o");
        let r2 = adapter.normalize(&event, &mut ctx, 2000);
        assert_eq!(r2.meta.provider, "openai");
        assert_eq!(r2.meta.model, "gpt-4o");
    }

    #[test]
    fn adapter_context_advance_turn() {
        let mut ctx = test_context();
        let adapter = RuntimeAdapter;
        let event = RawSourceEvent::TextDelta {
            text: "t".into(),
            is_reasoning: false,
        };

        let r1 = adapter.normalize(&event, &mut ctx, 1000);
        assert_eq!(r1.meta.turn_id, 0);

        ctx.advance_turn();
        let r2 = adapter.normalize(&event, &mut ctx, 2000);
        assert_eq!(r2.meta.turn_id, 1);
    }
}
