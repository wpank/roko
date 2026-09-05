//! Transcript event stream types for tool audit.
//!
//! A transcript is a time-ordered sequence of [`TranscriptRecord`]s that
//! captures everything that happens during a provider run: assistant
//! output, tool lifecycle, subagent coordination, usage, and errors.
//!
//! Each record pairs a [`TranscriptEventMeta`] (correlation, timing,
//! identity) with a [`TranscriptEvent`] (the payload). Together they
//! form the canonical audit surface that downstream stores, UIs, and
//! telemetry consumers read.
//!
//! # Key types
//!
//! | Type | Purpose |
//! |---|---|
//! | [`TranscriptEvent`] | What happened (14 event kinds) |
//! | [`TranscriptEventMeta`] | When/where/who (correlation + timing) |
//! | [`TranscriptRecord`] | Meta + event combined |
//! | [`ToolLifecycleStatus`] | Terminal status of a tool call |

use serde::{Deserialize, Serialize};

use super::call::{ToolCall, ToolResult};
use super::def::ToolCategory;

// ─── ToolLifecycleStatus ─────────────────────────────────────────────────

/// Terminal status of a single tool invocation through its full lifecycle.
///
/// Starts at [`Pending`](Self::Pending) when the provider emits a tool call,
/// progresses through admission and execution, and settles at one of the
/// terminal variants. The status is write-once: once a tool reaches a
/// terminal state it never changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolLifecycleStatus {
    /// Call received but not yet admitted by policy.
    Pending,
    /// Passed admission (policy, permissions, schema) and queued for execution.
    Admitted,
    /// Handler is currently running.
    Executing,
    /// Handler returned a successful result.
    Succeeded,
    /// Handler returned an error result.
    Failed,
    /// Handler exceeded its timeout budget.
    TimedOut,
    /// Cancellation token fired before completion.
    Cancelled,
    /// Call was denied by policy (role, contract, allowlist).
    Denied,
    /// Handler panicked (caught via `catch_unwind`).
    Panicked,
    /// Interrupted by external signal (e.g. SIGINT, conductor abort).
    Interrupted,
}

impl ToolLifecycleStatus {
    /// Whether this status represents a terminal (settled) state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded
                | Self::Failed
                | Self::TimedOut
                | Self::Cancelled
                | Self::Denied
                | Self::Panicked
                | Self::Interrupted
        )
    }

    /// Stable string identifier for logs and metrics keys.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Admitted => "admitted",
            Self::Executing => "executing",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
            Self::Denied => "denied",
            Self::Panicked => "panicked",
            Self::Interrupted => "interrupted",
        }
    }
}

// ─── TranscriptEvent ─────────────────────────────────────────────────────

/// A single event in a provider run transcript.
///
/// Events are tagged with `#[serde(tag = "type", rename_all = "snake_case")]`
/// so each serialized record is self-describing and grep-able by type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TranscriptEvent {
    /// A provider run began.
    RunStarted {
        /// The system prompt hash (for dedup/diffing across runs).
        system_prompt_hash: Option<String>,
        /// Number of tools offered to the provider.
        tools_offered: u32,
    },

    /// An assistant text delta arrived (streaming).
    AssistantDelta {
        /// The text fragment.
        text: String,
    },

    /// A reasoning/thinking delta arrived (extended thinking).
    ReasoningDelta {
        /// The reasoning text fragment.
        text: String,
    },

    /// A tool invocation was received from the provider.
    ToolStarted {
        /// The parsed tool call.
        call: ToolCall,
        /// Initial lifecycle status (typically Pending or Admitted).
        status: ToolLifecycleStatus,
        /// Tool category if known at parse time.
        category: Option<ToolCategory>,
    },

    /// Incremental output from a running tool (streaming tool results).
    ToolOutputDelta {
        /// The tool call ID this delta belongs to.
        call_id: String,
        /// The output fragment.
        text: String,
    },

    /// A tool invocation completed (succeeded or failed).
    ToolFinished {
        /// The tool call ID.
        call_id: String,
        /// The tool result.
        result: ToolResult,
        /// Terminal lifecycle status.
        status: ToolLifecycleStatus,
        /// Execution wall-clock time in milliseconds.
        execution_ms: Option<u64>,
    },

    /// A snapshot of the current todo/task list state.
    TodoSnapshot {
        /// Serialized todo items (provider-specific format).
        items: serde_json::Value,
    },

    /// A subagent was spawned.
    SubagentStarted {
        /// Subagent identifier.
        subagent_id: String,
        /// The task or prompt given to the subagent.
        task: String,
    },

    /// Incremental update from a running subagent.
    SubagentUpdate {
        /// Subagent identifier.
        subagent_id: String,
        /// Update payload (progress, partial result, etc.).
        payload: serde_json::Value,
    },

    /// A subagent completed.
    SubagentFinished {
        /// Subagent identifier.
        subagent_id: String,
        /// Whether the subagent succeeded.
        success: bool,
        /// Summary of the subagent's result.
        summary: Option<String>,
    },

    /// Token usage report for a turn or run.
    Usage {
        /// Input/prompt tokens consumed.
        input_tokens: u64,
        /// Output/completion tokens generated.
        output_tokens: u64,
        /// Cache-read tokens (if applicable).
        cache_read_tokens: u64,
        /// Cache-creation tokens (if applicable).
        cache_creation_tokens: u64,
        /// Estimated USD cost for this usage block.
        cost_usd: Option<f64>,
    },

    /// The active provider or model changed mid-run (fallback, rotation).
    ProviderChanged {
        /// Previous provider name.
        from_provider: Option<String>,
        /// New provider name.
        to_provider: String,
        /// Previous model name.
        from_model: Option<String>,
        /// New model name.
        to_model: String,
        /// Reason for the change.
        reason: String,
    },

    /// A non-fatal warning during the run.
    Warning {
        /// Warning code or category.
        code: String,
        /// Human-readable warning message.
        message: String,
    },

    /// A fatal or significant error during the run.
    Error {
        /// Error code or category.
        code: String,
        /// Human-readable error message.
        message: String,
        /// Whether this error was recoverable.
        recoverable: bool,
    },

    /// The provider run finished.
    RunFinished {
        /// Whether the run completed successfully.
        success: bool,
        /// Total turns in the run.
        total_turns: u32,
        /// Total tool calls in the run.
        total_tool_calls: u32,
        /// Total wall-clock time in milliseconds.
        wall_ms: u64,
    },
}

// ─── TranscriptEventMeta ─────────────────────────────────────────────────

/// Correlation and timing metadata attached to every transcript event.
///
/// The `sequence` field is a monotonically increasing counter within a
/// run, ensuring total ordering even when wall-clock timestamps collide.
///
/// `task_id` and `attempt_id` enable end-to-end correlation with the
/// [`CorrelationEnvelope`](super::handler::CorrelationEnvelope) used by
/// execution records, without requiring a separate join step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptEventMeta {
    /// Run identifier (UUID or similar).
    pub run_id: String,
    /// Turn number within the run (0-indexed).
    pub turn_id: u32,
    /// Agent identifier that produced this event.
    pub agent_id: String,
    /// Monotonically increasing sequence number within the run.
    pub sequence: u64,
    /// Unix-millis timestamp when the event was recorded.
    pub timestamp_ms: i64,
    /// Provider name active at event time.
    pub provider: String,
    /// Model name active at event time.
    pub model: String,
    /// Optional parent event sequence (for nesting, e.g. subagent events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_event_id: Option<u64>,
    /// Task identifier from the plan (correlates with `CorrelationEnvelope::task_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Attempt identifier for this task (correlates with `CorrelationEnvelope::attempt_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
}

// ─── TranscriptRecord ────────────────────────────────────────────────────

/// A complete transcript record: metadata plus the event payload.
///
/// This is the unit of serialization written to JSONL transcript logs
/// and consumed by stores, UIs, and telemetry.
///
/// The `schema_version` field enables forward-compatible deserialization:
/// readers that encounter a newer version can still parse the fields they
/// understand and skip unknown ones (via `#[serde(deny_unknown_fields)]`
/// being absent). The current version is [`Self::CURRENT_SCHEMA_VERSION`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptRecord {
    /// Schema version for forward-compatible deserialization.
    ///
    /// Defaults to [`Self::CURRENT_SCHEMA_VERSION`] when absent in
    /// legacy JSONL (pre-versioned records omit this field).
    #[serde(default = "TranscriptRecord::default_schema_version")]
    pub schema_version: u32,
    /// Correlation and timing metadata.
    pub meta: TranscriptEventMeta,
    /// The event payload.
    pub event: TranscriptEvent,
}

impl TranscriptRecord {
    /// Current schema version for new records.
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    fn default_schema_version() -> u32 {
        Self::CURRENT_SCHEMA_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_meta() -> TranscriptEventMeta {
        TranscriptEventMeta {
            run_id: "run-001".into(),
            turn_id: 0,
            agent_id: "agent-alpha".into(),
            sequence: 1,
            timestamp_ms: 1_700_000_000_000,
            provider: "anthropic".into(),
            model: "claude-opus-4-6".into(),
            parent_event_id: None,
            task_id: None,
            attempt_id: None,
        }
    }

    #[test]
    fn lifecycle_status_terminal_classification() {
        assert!(!ToolLifecycleStatus::Pending.is_terminal());
        assert!(!ToolLifecycleStatus::Admitted.is_terminal());
        assert!(!ToolLifecycleStatus::Executing.is_terminal());
        assert!(ToolLifecycleStatus::Succeeded.is_terminal());
        assert!(ToolLifecycleStatus::Failed.is_terminal());
        assert!(ToolLifecycleStatus::TimedOut.is_terminal());
        assert!(ToolLifecycleStatus::Cancelled.is_terminal());
        assert!(ToolLifecycleStatus::Denied.is_terminal());
        assert!(ToolLifecycleStatus::Panicked.is_terminal());
        assert!(ToolLifecycleStatus::Interrupted.is_terminal());
    }

    #[test]
    fn lifecycle_status_as_str_stable() {
        assert_eq!(ToolLifecycleStatus::Pending.as_str(), "pending");
        assert_eq!(ToolLifecycleStatus::Succeeded.as_str(), "succeeded");
        assert_eq!(ToolLifecycleStatus::TimedOut.as_str(), "timed_out");
    }

    #[test]
    fn lifecycle_status_serde_roundtrip() {
        let statuses = [
            ToolLifecycleStatus::Pending,
            ToolLifecycleStatus::Admitted,
            ToolLifecycleStatus::Executing,
            ToolLifecycleStatus::Succeeded,
            ToolLifecycleStatus::Failed,
            ToolLifecycleStatus::TimedOut,
            ToolLifecycleStatus::Cancelled,
            ToolLifecycleStatus::Denied,
            ToolLifecycleStatus::Panicked,
            ToolLifecycleStatus::Interrupted,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let decoded: ToolLifecycleStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, s);
        }
    }

    #[test]
    fn transcript_event_run_started_roundtrip() {
        let event = TranscriptEvent::RunStarted {
            system_prompt_hash: Some("abc123".into()),
            tools_offered: 16,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"run_started\""));
        let decoded: TranscriptEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn transcript_event_tool_started_roundtrip() {
        let call = ToolCall::at(
            "call-1",
            "read_file",
            serde_json::json!({"path": "x.rs"}),
            1_700_000_000_000,
        );
        let event = TranscriptEvent::ToolStarted {
            call,
            status: ToolLifecycleStatus::Pending,
            category: Some(ToolCategory::Read),
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: TranscriptEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn transcript_event_tool_finished_roundtrip() {
        let event = TranscriptEvent::ToolFinished {
            call_id: "call-1".into(),
            result: ToolResult::text("file contents here"),
            status: ToolLifecycleStatus::Succeeded,
            execution_ms: Some(42),
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: TranscriptEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn transcript_event_usage_roundtrip() {
        let event = TranscriptEvent::Usage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 200,
            cache_creation_tokens: 50,
            cost_usd: Some(0.015),
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: TranscriptEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn transcript_event_subagent_lifecycle_roundtrip() {
        let events = [
            TranscriptEvent::SubagentStarted {
                subagent_id: "sub-1".into(),
                task: "research topic".into(),
            },
            TranscriptEvent::SubagentUpdate {
                subagent_id: "sub-1".into(),
                payload: serde_json::json!({"progress": 0.5}),
            },
            TranscriptEvent::SubagentFinished {
                subagent_id: "sub-1".into(),
                success: true,
                summary: Some("Found 3 results".into()),
            },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let decoded: TranscriptEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(&decoded, event);
        }
    }

    #[test]
    fn transcript_event_all_variants_roundtrip() {
        let events: Vec<TranscriptEvent> = vec![
            TranscriptEvent::RunStarted {
                system_prompt_hash: None,
                tools_offered: 0,
            },
            TranscriptEvent::AssistantDelta {
                text: "hello".into(),
            },
            TranscriptEvent::ReasoningDelta {
                text: "thinking...".into(),
            },
            TranscriptEvent::ToolOutputDelta {
                call_id: "c1".into(),
                text: "partial output".into(),
            },
            TranscriptEvent::TodoSnapshot {
                items: serde_json::json!([{"id": 1, "text": "do thing"}]),
            },
            TranscriptEvent::ProviderChanged {
                from_provider: Some("anthropic".into()),
                to_provider: "openai".into(),
                from_model: Some("claude-opus-4-6".into()),
                to_model: "gpt-4o".into(),
                reason: "rate limited".into(),
            },
            TranscriptEvent::Warning {
                code: "TOOL_SLOW".into(),
                message: "bash took 30s".into(),
            },
            TranscriptEvent::Error {
                code: "PROVIDER_ERROR".into(),
                message: "500 internal server error".into(),
                recoverable: true,
            },
            TranscriptEvent::RunFinished {
                success: true,
                total_turns: 5,
                total_tool_calls: 12,
                wall_ms: 45_000,
            },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let decoded: TranscriptEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(&decoded, event);
        }
    }

    #[test]
    fn transcript_meta_optional_parent() {
        let meta = sample_meta();
        let json = serde_json::to_string(&meta).unwrap();
        assert!(!json.contains("parent_event_id"));

        let mut meta_with_parent = sample_meta();
        meta_with_parent.parent_event_id = Some(42);
        let json = serde_json::to_string(&meta_with_parent).unwrap();
        assert!(json.contains("\"parent_event_id\":42"));
        let decoded: TranscriptEventMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, meta_with_parent);
    }

    #[test]
    fn transcript_record_roundtrip() {
        let record = TranscriptRecord {
            schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
            meta: sample_meta(),
            event: TranscriptEvent::AssistantDelta { text: "hi".into() },
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"schema_version\":1"));
        let decoded: TranscriptRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, record);
    }

    #[test]
    fn transcript_record_schema_version_defaults_on_missing() {
        // Legacy JSONL without schema_version should default to CURRENT.
        let json = r#"{"meta":{"run_id":"r","turn_id":0,"agent_id":"a","sequence":1,"timestamp_ms":0,"provider":"p","model":"m"},"event":{"type":"assistant_delta","text":"hi"}}"#;
        let record: TranscriptRecord = serde_json::from_str(json).unwrap();
        assert_eq!(
            record.schema_version,
            TranscriptRecord::CURRENT_SCHEMA_VERSION
        );
    }

    #[test]
    fn transcript_record_unknown_schema_version_preserved() {
        // Future schema versions deserialize without error.
        let json = r#"{"schema_version":99,"meta":{"run_id":"r","turn_id":0,"agent_id":"a","sequence":1,"timestamp_ms":0,"provider":"p","model":"m"},"event":{"type":"assistant_delta","text":"hi"}}"#;
        let record: TranscriptRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.schema_version, 99);
    }

    #[test]
    fn transcript_record_jsonl_multiline() {
        let records = vec![
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: TranscriptEventMeta {
                    sequence: 0,
                    ..sample_meta()
                },
                event: TranscriptEvent::RunStarted {
                    system_prompt_hash: None,
                    tools_offered: 16,
                },
            },
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: TranscriptEventMeta {
                    sequence: 1,
                    ..sample_meta()
                },
                event: TranscriptEvent::AssistantDelta {
                    text: "hello".into(),
                },
            },
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: TranscriptEventMeta {
                    sequence: 2,
                    ..sample_meta()
                },
                event: TranscriptEvent::RunFinished {
                    success: true,
                    total_turns: 1,
                    total_tool_calls: 0,
                    wall_ms: 1000,
                },
            },
        ];

        // Serialize as JSONL (one JSON object per line).
        let jsonl: String = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n");

        // Deserialize back line by line.
        let decoded: Vec<TranscriptRecord> = jsonl
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        assert_eq!(decoded, records);
    }

    #[test]
    fn transcript_records_ordered_by_sequence() {
        let mut records = vec![
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: TranscriptEventMeta {
                    sequence: 3,
                    ..sample_meta()
                },
                event: TranscriptEvent::RunFinished {
                    success: true,
                    total_turns: 1,
                    total_tool_calls: 0,
                    wall_ms: 100,
                },
            },
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: TranscriptEventMeta {
                    sequence: 1,
                    ..sample_meta()
                },
                event: TranscriptEvent::RunStarted {
                    system_prompt_hash: None,
                    tools_offered: 0,
                },
            },
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: TranscriptEventMeta {
                    sequence: 2,
                    ..sample_meta()
                },
                event: TranscriptEvent::AssistantDelta { text: "x".into() },
            },
        ];

        records.sort_by_key(|r| r.meta.sequence);

        let seqs: Vec<u64> = records.iter().map(|r| r.meta.sequence).collect();
        assert_eq!(seqs, vec![1, 2, 3]);
    }

    #[test]
    fn transcript_meta_correlation_fields_optional() {
        let meta = sample_meta();
        let json = serde_json::to_string(&meta).unwrap();
        // Optional fields should not appear when None.
        assert!(!json.contains("task_id"));
        assert!(!json.contains("attempt_id"));

        let mut meta_with_task = sample_meta();
        meta_with_task.task_id = Some("task-01".into());
        meta_with_task.attempt_id = Some("attempt-2".into());
        let json = serde_json::to_string(&meta_with_task).unwrap();
        assert!(json.contains("\"task_id\":\"task-01\""));
        assert!(json.contains("\"attempt_id\":\"attempt-2\""));
        let decoded: TranscriptEventMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, meta_with_task);
    }

    #[test]
    fn transcript_meta_legacy_without_correlation_defaults() {
        // Legacy meta without task_id/attempt_id deserializes with None.
        let json = r#"{"run_id":"r","turn_id":0,"agent_id":"a","sequence":1,"timestamp_ms":0,"provider":"p","model":"m"}"#;
        let meta: TranscriptEventMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.task_id, None);
        assert_eq!(meta.attempt_id, None);
    }

    // ─── Golden fixture covering all 14 event types ─────────────────────

    fn meta_at(seq: u64) -> TranscriptEventMeta {
        TranscriptEventMeta {
            run_id: "golden-run".into(),
            turn_id: 0,
            agent_id: "golden-agent".into(),
            sequence: seq,
            timestamp_ms: 1_700_000_000_000 + (seq as i64 * 100),
            provider: "anthropic".into(),
            model: "claude-opus-4-6".into(),
            parent_event_id: None,
            task_id: Some("task-golden".into()),
            attempt_id: Some("attempt-0".into()),
        }
    }

    fn golden_fixture() -> Vec<TranscriptRecord> {
        let make_call = |id: &str, name: &str| {
            ToolCall::at(
                id,
                name,
                serde_json::json!({"path": "src/main.rs"}),
                1_700_000_000_000,
            )
        };
        vec![
            // 1: RunStarted
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(1),
                event: TranscriptEvent::RunStarted {
                    system_prompt_hash: Some("sha256:abc123".into()),
                    tools_offered: 16,
                },
            },
            // 2: AssistantDelta
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(2),
                event: TranscriptEvent::AssistantDelta {
                    text: "Let me analyze the code.".into(),
                },
            },
            // 3: ReasoningDelta
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(3),
                event: TranscriptEvent::ReasoningDelta {
                    text: "I need to check the file structure first.".into(),
                },
            },
            // 4: ToolStarted (success path)
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(4),
                event: TranscriptEvent::ToolStarted {
                    call: make_call("call-ok", "read_file"),
                    status: ToolLifecycleStatus::Admitted,
                    category: Some(ToolCategory::Read),
                },
            },
            // 5: ToolOutputDelta
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(5),
                event: TranscriptEvent::ToolOutputDelta {
                    call_id: "call-ok".into(),
                    text: "fn main() {".into(),
                },
            },
            // 6: ToolFinished (success)
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(6),
                event: TranscriptEvent::ToolFinished {
                    call_id: "call-ok".into(),
                    result: ToolResult::text("fn main() { println!(\"hello\"); }"),
                    status: ToolLifecycleStatus::Succeeded,
                    execution_ms: Some(12),
                },
            },
            // 7: ToolStarted (failure path)
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(7),
                event: TranscriptEvent::ToolStarted {
                    call: make_call("call-fail", "bash"),
                    status: ToolLifecycleStatus::Admitted,
                    category: Some(ToolCategory::Exec),
                },
            },
            // 8: ToolFinished (failure)
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(8),
                event: TranscriptEvent::ToolFinished {
                    call_id: "call-fail".into(),
                    result: ToolResult::err(crate::tool::call::ToolError::Other(
                        "exit code 1".into(),
                    )),
                    status: ToolLifecycleStatus::Failed,
                    execution_ms: Some(500),
                },
            },
            // 9: ToolStarted (cancelled path)
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(9),
                event: TranscriptEvent::ToolStarted {
                    call: make_call("call-cancel", "bash"),
                    status: ToolLifecycleStatus::Admitted,
                    category: Some(ToolCategory::Exec),
                },
            },
            // 10: ToolFinished (cancelled)
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(10),
                event: TranscriptEvent::ToolFinished {
                    call_id: "call-cancel".into(),
                    result: ToolResult::err(crate::tool::call::ToolError::Cancelled),
                    status: ToolLifecycleStatus::Cancelled,
                    execution_ms: Some(3000),
                },
            },
            // 11: SubagentStarted
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(11),
                event: TranscriptEvent::SubagentStarted {
                    subagent_id: "sub-1".into(),
                    task: "research security implications".into(),
                },
            },
            // 12: SubagentUpdate
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(12),
                event: TranscriptEvent::SubagentUpdate {
                    subagent_id: "sub-1".into(),
                    payload: serde_json::json!({"progress": 0.75, "findings": 2}),
                },
            },
            // 13: SubagentFinished
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(13),
                event: TranscriptEvent::SubagentFinished {
                    subagent_id: "sub-1".into(),
                    success: true,
                    summary: Some("Found 2 security concerns".into()),
                },
            },
            // 14: TodoSnapshot
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(14),
                event: TranscriptEvent::TodoSnapshot {
                    items: serde_json::json!([
                        {"id": "1", "text": "Fix bug", "done": true},
                        {"id": "2", "text": "Add tests", "done": false}
                    ]),
                },
            },
            // 15: ProviderChanged
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(15),
                event: TranscriptEvent::ProviderChanged {
                    from_provider: Some("anthropic".into()),
                    to_provider: "openai".into(),
                    from_model: Some("claude-opus-4-6".into()),
                    to_model: "gpt-4o".into(),
                    reason: "rate limited".into(),
                },
            },
            // 16: Warning
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(16),
                event: TranscriptEvent::Warning {
                    code: "TOOL_SLOW".into(),
                    message: "bash execution took 3s".into(),
                },
            },
            // 17: Error
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(17),
                event: TranscriptEvent::Error {
                    code: "PROVIDER_5XX".into(),
                    message: "upstream returned 503".into(),
                    recoverable: true,
                },
            },
            // 18: Usage
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(18),
                event: TranscriptEvent::Usage {
                    input_tokens: 5000,
                    output_tokens: 2000,
                    cache_read_tokens: 1200,
                    cache_creation_tokens: 300,
                    cost_usd: Some(0.042),
                },
            },
            // 19: RunFinished
            TranscriptRecord {
                schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
                meta: meta_at(19),
                event: TranscriptEvent::RunFinished {
                    success: true,
                    total_turns: 3,
                    total_tool_calls: 3,
                    wall_ms: 45_000,
                },
            },
        ]
    }

    #[test]
    fn golden_fixture_covers_all_event_types() {
        let records = golden_fixture();
        assert_eq!(records.len(), 19, "golden fixture should have 19 records");

        // Verify every event type tag appears at least once.
        let types: std::collections::HashSet<String> = records
            .iter()
            .map(|r| {
                let json = serde_json::to_value(&r.event).unwrap();
                json.get("type").unwrap().as_str().unwrap().to_string()
            })
            .collect();

        let expected_types = [
            "run_started",
            "assistant_delta",
            "reasoning_delta",
            "tool_started",
            "tool_output_delta",
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
        for t in &expected_types {
            assert!(types.contains(*t), "missing event type: {t}");
        }
    }

    #[test]
    fn golden_fixture_jsonl_roundtrip_deterministic() {
        let records = golden_fixture();
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
    fn golden_fixture_replay_ordering_by_sequence() {
        let mut records = golden_fixture();
        // Shuffle the records.
        records.reverse();
        // Replay: sort by sequence.
        records.sort_by_key(|r| r.meta.sequence);
        let seqs: Vec<u64> = records.iter().map(|r| r.meta.sequence).collect();
        let expected: Vec<u64> = (1..=19).collect();
        assert_eq!(seqs, expected, "replay must restore sequence order");
    }

    #[test]
    fn golden_fixture_dedup_by_sequence() {
        let records = golden_fixture();
        // Duplicate every record.
        let mut doubled: Vec<TranscriptRecord> =
            records.iter().chain(records.iter()).cloned().collect();
        doubled.sort_by_key(|r| r.meta.sequence);
        // Dedup by sequence.
        let mut seen = std::collections::HashSet::new();
        let deduped: Vec<&TranscriptRecord> = doubled
            .iter()
            .filter(|r| seen.insert(r.meta.sequence))
            .collect();
        assert_eq!(deduped.len(), 19, "dedup must remove exact duplicates");
        // Verify ordering after dedup.
        let seqs: Vec<u64> = deduped.iter().map(|r| r.meta.sequence).collect();
        let expected: Vec<u64> = (1..=19).collect();
        assert_eq!(seqs, expected);
    }

    #[test]
    fn golden_fixture_call_id_keyed_pairing() {
        let records = golden_fixture();
        // Build call_id -> (started, finished) pairs.
        let mut started: std::collections::HashMap<String, &TranscriptRecord> =
            std::collections::HashMap::new();
        let mut finished: std::collections::HashMap<String, &TranscriptRecord> =
            std::collections::HashMap::new();

        for r in &records {
            match &r.event {
                TranscriptEvent::ToolStarted { call, .. } => {
                    started.insert(call.id.clone(), r);
                }
                TranscriptEvent::ToolFinished { call_id, .. } => {
                    finished.insert(call_id.clone(), r);
                }
                _ => {}
            }
        }

        // Every started call must have a finished counterpart.
        assert_eq!(started.len(), 3, "3 tool calls in golden fixture");
        assert_eq!(finished.len(), 3, "3 tool finishes in golden fixture");
        for call_id in started.keys() {
            assert!(
                finished.contains_key(call_id),
                "call {call_id} started but never finished"
            );
        }

        // Verify ordering: started.sequence < finished.sequence for each call.
        for (call_id, start_rec) in &started {
            let finish_rec = finished.get(call_id).unwrap();
            assert!(
                start_rec.meta.sequence < finish_rec.meta.sequence,
                "call {call_id}: started@{} must precede finished@{}",
                start_rec.meta.sequence,
                finish_rec.meta.sequence,
            );
        }

        // Verify terminal statuses match expectations.
        assert!(matches!(
            &finished["call-ok"].event,
            TranscriptEvent::ToolFinished {
                status: ToolLifecycleStatus::Succeeded,
                ..
            }
        ));
        assert!(matches!(
            &finished["call-fail"].event,
            TranscriptEvent::ToolFinished {
                status: ToolLifecycleStatus::Failed,
                ..
            }
        ));
        assert!(matches!(
            &finished["call-cancel"].event,
            TranscriptEvent::ToolFinished {
                status: ToolLifecycleStatus::Cancelled,
                ..
            }
        ));
    }

    #[test]
    fn golden_fixture_subagent_lifecycle_pairing() {
        let records = golden_fixture();
        let mut subagent_started = false;
        let mut subagent_updated = false;
        let mut subagent_finished = false;
        let mut lifecycle_order = Vec::new();

        for r in &records {
            match &r.event {
                TranscriptEvent::SubagentStarted { subagent_id, .. } if subagent_id == "sub-1" => {
                    subagent_started = true;
                    lifecycle_order.push(("started", r.meta.sequence));
                }
                TranscriptEvent::SubagentUpdate { subagent_id, .. } if subagent_id == "sub-1" => {
                    subagent_updated = true;
                    lifecycle_order.push(("update", r.meta.sequence));
                }
                TranscriptEvent::SubagentFinished { subagent_id, .. } if subagent_id == "sub-1" => {
                    subagent_finished = true;
                    lifecycle_order.push(("finished", r.meta.sequence));
                }
                _ => {}
            }
        }

        assert!(subagent_started, "sub-1 must be started");
        assert!(subagent_updated, "sub-1 must have updates");
        assert!(subagent_finished, "sub-1 must be finished");
        // Verify started < update < finished.
        assert!(lifecycle_order[0].1 < lifecycle_order[1].1);
        assert!(lifecycle_order[1].1 < lifecycle_order[2].1);
    }

    #[test]
    fn golden_fixture_correlation_fields_populated() {
        let records = golden_fixture();
        for r in &records {
            assert_eq!(
                r.meta.task_id.as_deref(),
                Some("task-golden"),
                "seq {}: task_id must be set",
                r.meta.sequence
            );
            assert_eq!(
                r.meta.attempt_id.as_deref(),
                Some("attempt-0"),
                "seq {}: attempt_id must be set",
                r.meta.sequence
            );
        }
    }

    #[test]
    fn golden_fixture_schema_version_consistent() {
        let records = golden_fixture();
        for r in &records {
            assert_eq!(
                r.schema_version,
                TranscriptRecord::CURRENT_SCHEMA_VERSION,
                "seq {}: schema_version must be current",
                r.meta.sequence
            );
        }
    }
}
