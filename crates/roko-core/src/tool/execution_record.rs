//! Enriched tool execution records for the audit trail.
//!
//! While [`TranscriptRecord`](super::transcript::TranscriptRecord) captures
//! the event stream as it happens, [`ToolExecutionEnvelope`] is the
//! settled, post-hoc record written once a tool call reaches a terminal
//! state. It combines correlation, identity, authority, timing, outcome,
//! and provenance into a single auditable document.
//!
//! [`ToolExecutionRecord`] is the lighter-weight inner record pairing
//! the raw call/result with timing data, suitable for embedding in the
//! envelope or using standalone.
//!
//! # Key types
//!
//! | Type | Purpose |
//! |---|---|
//! | [`ToolExecutionRecord`] | Call + result + timing |
//! | [`ToolExecutionEnvelope`] | Full audit envelope with 6 field groups |
//! | [`CorrelationEnvelope`] | Shared correlation IDs (run, task, attempt, turn, agent) |

use serde::{Deserialize, Serialize};

use super::call::{ToolCall, ToolResult};
use super::def::{ToolCategory, ToolSource};
use super::handler::CorrelationEnvelope;
use super::transcript::ToolLifecycleStatus;
use crate::AgentRole;

// ─── ToolExecutionRecord ─────────────────────────────────────────────────

/// A tool call paired with its result and timing data.
///
/// This is the compact inner record that can be embedded in the full
/// [`ToolExecutionEnvelope`] or used standalone when the full audit
/// context is not needed (e.g. in-memory caches, lightweight logs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    /// The inbound tool call from the provider.
    pub call: ToolCall,
    /// The outbound result (success or error).
    pub result: ToolResult,
    /// Unix-millis when admission checks completed.
    pub admitted_at_ms: i64,
    /// Unix-millis when handler execution started.
    pub started_at_ms: i64,
    /// Unix-millis when handler execution finished.
    pub finished_at_ms: i64,
    /// Time spent in the admission queue (ms).
    pub queue_ms: u64,
    /// Handler execution wall-clock time (ms).
    pub execution_ms: u64,
    /// Index of the provider that issued this call (for multi-provider runs).
    pub provider_index: usize,
}

// ─── ToolExecutionEnvelope ───────────────────────────────────────────────

/// Full audit envelope for a single tool execution.
///
/// Organized into six field groups:
///
/// 1. **Correlation** — IDs that locate this call in the execution hierarchy
/// 2. **Identity** — what tool was called and where it came from
/// 3. **Authority** — who was allowed to call it and why
/// 4. **Timing** — when each phase happened
/// 5. **Outcome** — what happened (status, result summary, artifacts)
/// 6. **Provenance** — which provider/model/server produced the call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolExecutionEnvelope {
    // ── Correlation ──────────────────────────────────────────────────
    /// Schema version for forward-compatible deserialization.
    pub schema_version: u32,
    /// Run identifier.
    pub run_id: String,
    /// Task identifier.
    pub task_id: String,
    /// Attempt identifier (retries increment).
    pub attempt_id: String,
    /// Turn identifier within the run.
    pub turn_id: String,
    /// Agent identifier.
    pub agent_id: String,
    /// Provider-assigned call ID (correlates with provider logs).
    pub provider_call_id: String,

    // ── Identity ─────────────────────────────────────────────────────
    /// Canonical snake_case tool name.
    pub canonical_tool_name: String,
    /// Where the tool definition came from.
    pub source: ToolSource,
    /// Tool category.
    pub category: ToolCategory,
    /// Index of the provider that issued this call.
    pub provider_index: usize,

    // ── Authority ────────────────────────────────────────────────────
    /// Role of the agent that invoked the tool.
    pub role: AgentRole,
    /// Effective capabilities granted for this call.
    pub effective_capabilities: Vec<String>,
    /// The policy rule that matched (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_rule: Option<String>,
    /// Hash of the agent contract in effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_hash: Option<String>,

    // ── Timing ───────────────────────────────────────────────────────
    /// Unix-millis when the call was received from the provider.
    pub received_at: i64,
    /// Unix-millis when admission checks completed.
    pub admitted_at: i64,
    /// Unix-millis when handler execution started.
    pub started_at: i64,
    /// Unix-millis when handler execution finished.
    pub finished_at: i64,
    /// Time in the admission queue (ms).
    pub queue_ms: u64,
    /// Handler execution wall-clock time (ms).
    pub execution_ms: u64,
    /// Source of the timeout deadline (e.g. "tool_def", "contract", "global").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_source: Option<String>,

    // ── Outcome ──────────────────────────────────────────────────────
    /// Terminal lifecycle status.
    pub status: ToolLifecycleStatus,
    /// Failure kind if status is not Succeeded (free-form string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_kind: Option<String>,
    /// Size of the result payload in bytes.
    pub result_bytes: u64,
    /// Number of artifacts produced.
    pub artifact_count: u32,
    /// Bitflags indicating which fields were truncated.
    #[serde(default)]
    pub truncation_flags: u32,
    /// Bitflags indicating which fields were redacted.
    #[serde(default)]
    pub redaction_flags: u32,

    // ── Provenance ───────────────────────────────────────────────────
    /// Provider name (e.g. "anthropic", "openai_compat").
    pub provider_name: String,
    /// Model name (e.g. "claude-opus-4-6").
    pub model_name: String,
    /// MCP server name if tool came from MCP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_server: Option<String>,
    /// Plugin ID if tool came from a plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
    /// Parent subagent ID if this call was made inside a subagent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_subagent_id: Option<String>,
}

impl ToolExecutionEnvelope {
    /// Current schema version.
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    /// Extract the correlation envelope from this record.
    #[must_use]
    pub fn correlation(&self) -> CorrelationEnvelope {
        CorrelationEnvelope {
            run_id: self.run_id.clone(),
            task_id: self.task_id.clone(),
            attempt_id: self.attempt_id.clone(),
            turn_id: self.turn_id.clone(),
            agent_id: self.agent_id.clone(),
        }
    }

    /// Total wall-clock time from receipt to finish (ms).
    #[must_use]
    pub fn total_ms(&self) -> u64 {
        if self.finished_at > self.received_at {
            (self.finished_at - self.received_at) as u64
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_correlation() -> CorrelationEnvelope {
        CorrelationEnvelope {
            run_id: "run-001".into(),
            task_id: "task-01".into(),
            attempt_id: "attempt-1".into(),
            turn_id: "turn-3".into(),
            agent_id: "agent-alpha".into(),
        }
    }

    fn sample_execution_record() -> ToolExecutionRecord {
        ToolExecutionRecord {
            call: ToolCall::at(
                "call-1",
                "read_file",
                serde_json::json!({"path": "src/main.rs"}),
                1_700_000_000_000,
            ),
            result: ToolResult::text("fn main() {}"),
            admitted_at_ms: 1_700_000_000_001,
            started_at_ms: 1_700_000_000_002,
            finished_at_ms: 1_700_000_000_050,
            queue_ms: 1,
            execution_ms: 48,
            provider_index: 0,
        }
    }

    fn sample_envelope() -> ToolExecutionEnvelope {
        ToolExecutionEnvelope {
            schema_version: ToolExecutionEnvelope::CURRENT_SCHEMA_VERSION,
            run_id: "run-001".into(),
            task_id: "task-01".into(),
            attempt_id: "attempt-1".into(),
            turn_id: "turn-3".into(),
            agent_id: "agent-alpha".into(),
            provider_call_id: "pc-abc".into(),
            canonical_tool_name: "read_file".into(),
            source: ToolSource::Builtin,
            category: ToolCategory::Read,
            provider_index: 0,
            role: AgentRole::Implementer,
            effective_capabilities: vec!["read".into()],
            matched_rule: None,
            contract_hash: None,
            received_at: 1_700_000_000_000,
            admitted_at: 1_700_000_000_001,
            started_at: 1_700_000_000_002,
            finished_at: 1_700_000_000_050,
            queue_ms: 1,
            execution_ms: 48,
            deadline_source: Some("tool_def".into()),
            status: ToolLifecycleStatus::Succeeded,
            failure_kind: None,
            result_bytes: 12,
            artifact_count: 0,
            truncation_flags: 0,
            redaction_flags: 0,
            provider_name: "anthropic".into(),
            model_name: "claude-opus-4-6".into(),
            mcp_server: None,
            plugin_id: None,
            parent_subagent_id: None,
        }
    }

    #[test]
    fn correlation_envelope_roundtrip() {
        let c = sample_correlation();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: CorrelationEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, c);
    }

    #[test]
    fn execution_record_roundtrip() {
        let r = sample_execution_record();
        let json = serde_json::to_string(&r).unwrap();
        let decoded: ToolExecutionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, r);
    }

    #[test]
    fn execution_envelope_roundtrip() {
        let e = sample_envelope();
        let json = serde_json::to_string(&e).unwrap();
        let decoded: ToolExecutionEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, e);
    }

    #[test]
    fn envelope_optional_fields_skip_when_none() {
        let e = sample_envelope();
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("matched_rule"));
        assert!(!json.contains("contract_hash"));
        assert!(!json.contains("mcp_server"));
        assert!(!json.contains("plugin_id"));
        assert!(!json.contains("parent_subagent_id"));
    }

    #[test]
    fn envelope_optional_fields_present_when_set() {
        let mut e = sample_envelope();
        e.matched_rule = Some("allow_read_tools".into());
        e.contract_hash = Some("sha256:abc123".into());
        e.mcp_server = Some("code-intel".into());
        e.plugin_id = Some("plugin-xyz".into());
        e.parent_subagent_id = Some("sub-1".into());

        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("matched_rule"));
        assert!(json.contains("contract_hash"));
        assert!(json.contains("mcp_server"));
        assert!(json.contains("plugin_id"));
        assert!(json.contains("parent_subagent_id"));

        let decoded: ToolExecutionEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, e);
    }

    #[test]
    fn envelope_correlation_extraction() {
        let e = sample_envelope();
        let c = e.correlation();
        assert_eq!(c.run_id, "run-001");
        assert_eq!(c.task_id, "task-01");
        assert_eq!(c.attempt_id, "attempt-1");
        assert_eq!(c.turn_id, "turn-3");
        assert_eq!(c.agent_id, "agent-alpha");
    }

    #[test]
    fn envelope_total_ms() {
        let e = sample_envelope();
        assert_eq!(e.total_ms(), 50);
    }

    #[test]
    fn envelope_total_ms_zero_on_invalid_ordering() {
        let mut e = sample_envelope();
        e.finished_at = e.received_at - 1;
        assert_eq!(e.total_ms(), 0);
    }

    #[test]
    fn envelope_schema_version_current() {
        let e = sample_envelope();
        assert_eq!(e.schema_version, 1);
        assert_eq!(
            e.schema_version,
            ToolExecutionEnvelope::CURRENT_SCHEMA_VERSION
        );
    }

    #[test]
    fn envelope_with_failure() {
        let mut e = sample_envelope();
        e.status = ToolLifecycleStatus::Failed;
        e.failure_kind = Some("permission_denied".into());
        e.result_bytes = 0;

        let json = serde_json::to_string(&e).unwrap();
        let decoded: ToolExecutionEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.status, ToolLifecycleStatus::Failed);
        assert_eq!(decoded.failure_kind.as_deref(), Some("permission_denied"));
    }

    #[test]
    fn envelope_with_mcp_source() {
        let mut e = sample_envelope();
        e.source = ToolSource::Mcp {
            server: "code-intel".into(),
        };
        e.category = ToolCategory::Mcp;
        e.mcp_server = Some("code-intel".into());

        let json = serde_json::to_string(&e).unwrap();
        let decoded: ToolExecutionEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(
            decoded.source,
            ToolSource::Mcp {
                server: "code-intel".into()
            }
        );
        assert_eq!(decoded.mcp_server.as_deref(), Some("code-intel"));
    }

    #[test]
    fn execution_record_timing_consistency() {
        let r = sample_execution_record();
        let computed_queue = r.started_at_ms - r.admitted_at_ms;
        let computed_exec = r.finished_at_ms - r.started_at_ms;
        assert_eq!(computed_queue, r.queue_ms as i64);
        assert_eq!(computed_exec, r.execution_ms as i64);
    }
}
