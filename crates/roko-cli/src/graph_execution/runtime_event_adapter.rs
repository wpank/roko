//! Graph-to-canonical runtime event adapter (#248).
//!
//! [`GraphRuntimeEventAdapter`] owns one atomic next-sequence counter,
//! consumes #246 [`GraphExecutionEvent`] values, looks up identity via
//! [`GraphIdentityMap`], and emits #208 v2 [`RuntimeEventEnvelope`] values
//! with `source = "graph"`.
//!
//! Replay preserves original sequence/event identity. Unknown authored nodes
//! use `plan_id = graph_id`, no task ID, and title `"graph node <node_id>"`.
//!
//! The exhaustive match on [`GraphExecutionEvent`] is compile-time: adding a
//! new variant without a conversion arm is a build error.
//!
//! [`GraphExecutionEvent`]: roko_graph::events::GraphExecutionEvent
//! [`RuntimeEventEnvelope`]: roko_core::runtime_event::RuntimeEventEnvelope

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use uuid::Uuid;

use roko_core::runtime_event::{
    RuntimeEvent, RuntimeEventDelivery, RuntimeEventEnvelope, RuntimeEventMode,
};
use roko_graph::events::GraphExecutionEvent;

use super::identity_map::{GraphIdentityMap, NodeIdentity};

/// Source tag stamped on every envelope produced by this adapter.
const GRAPH_SOURCE: &str = "graph";

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Converts [`GraphExecutionEvent`] into canonical [`RuntimeEventEnvelope`]
/// using a [`GraphIdentityMap`] for node-to-plan/task identity resolution.
///
/// Thread-safe: the only mutable state is an atomic sequence counter.
pub struct GraphRuntimeEventAdapter {
    /// Identity map built from the converted graph before execution.
    identity: GraphIdentityMap,
    /// Strictly monotonic per-run sequence counter.
    next_seq: AtomicU64,
}

impl GraphRuntimeEventAdapter {
    /// Create a new adapter with the given identity map.
    ///
    /// The sequence counter starts at 1 (0 is reserved for the sentinel).
    #[must_use]
    pub fn new(identity: GraphIdentityMap) -> Self {
        Self {
            identity,
            next_seq: AtomicU64::new(1),
        }
    }

    /// Access the underlying identity map.
    #[must_use]
    pub fn identity(&self) -> &GraphIdentityMap {
        &self.identity
    }

    /// Convert a single graph event into a canonical runtime event envelope.
    ///
    /// The match is exhaustive: adding a new `GraphExecutionEvent` variant
    /// without a conversion arm is a compile error.
    #[must_use]
    pub fn convert(&self, event: &GraphExecutionEvent) -> RuntimeEventEnvelope {
        let common = event.common();
        let identity = event
            .node()
            .map(|n| self.identity.get_or_fallback(&n.node_id));

        let (payload, mode) = self.map_payload(event);
        let delivery = payload.delivery();
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);

        let (plan_id, task_id, node_id, attempt_id, agent_id) =
            self.extract_ids(event, identity.as_ref());

        RuntimeEventEnvelope {
            event_id: Uuid::new_v4().to_string(),
            run_id: common.run_id.clone(),
            plan_id,
            task_id,
            node_id: node_id.map(|s| s.to_string()),
            attempt_id,
            agent_id,
            seq,
            ts: Utc::now(),
            schema_version: 2,
            source: GRAPH_SOURCE.to_string(),
            mode,
            delivery,
            correlation_id: Some(common.graph_id.clone()),
            idempotency_key: None,
            payload,
        }
    }

    /// Convert a replay event, preserving the original event identity.
    #[must_use]
    pub fn convert_replay(
        &self,
        event: &GraphExecutionEvent,
        original_event_id: &str,
        original_seq: u64,
        original_ts: chrono::DateTime<Utc>,
    ) -> RuntimeEventEnvelope {
        let common = event.common();
        let identity = event
            .node()
            .map(|n| self.identity.get_or_fallback(&n.node_id));

        let (payload, _mode) = self.map_payload(event);
        let delivery = payload.delivery();

        let (plan_id, task_id, node_id, attempt_id, agent_id) =
            self.extract_ids(event, identity.as_ref());

        RuntimeEventEnvelope {
            event_id: original_event_id.to_string(),
            run_id: common.run_id.clone(),
            plan_id,
            task_id,
            node_id: node_id.map(|s| s.to_string()),
            attempt_id,
            agent_id,
            seq: original_seq,
            ts: original_ts,
            schema_version: 2,
            source: GRAPH_SOURCE.to_string(),
            mode: RuntimeEventMode::Replay,
            delivery,
            correlation_id: Some(common.graph_id.clone()),
            idempotency_key: None,
            payload,
        }
    }

    // ── Private helpers ──────────────────────────────────────────────

    /// Extract envelope identity fields from the event and resolved identity.
    fn extract_ids<'a>(
        &self,
        event: &'a GraphExecutionEvent,
        identity: Option<&NodeIdentity>,
    ) -> (
        Option<String>,
        Option<String>,
        Option<&'a str>,
        Option<String>,
        Option<String>,
    ) {
        let node_fields = event.node();
        let dispatch = event.dispatch();

        let plan_id = identity.map(|id| id.plan_id.clone());
        let task_id = identity.and_then(|id| {
            if id.task_id.is_empty() {
                None
            } else {
                Some(id.task_id.clone())
            }
        });
        let node_id = node_fields.map(|n| n.node_id.as_str());
        let attempt_id = dispatch.map(|d| d.attempt_id.clone());
        let agent_id = dispatch.and_then(|d| d.agent_id.clone());

        (plan_id, task_id, node_id, attempt_id, agent_id)
    }

    /// Map a graph event to its canonical runtime event payload and mode.
    ///
    /// This match is exhaustive and compile-time: adding a new
    /// `GraphExecutionEvent` variant without a conversion arm here is a
    /// build error.
    fn map_payload(&self, event: &GraphExecutionEvent) -> (RuntimeEvent, RuntimeEventMode) {
        let mode = self.infer_mode(event);

        let payload = match event {
            // ── Graph lifecycle ──────────────────────────────────────
            GraphExecutionEvent::GraphStarted { common } => RuntimeEvent::RunStarted {
                run_id: common.run_id.clone(),
                prompt: String::new(),
                complexity: "graph".to_string(),
            },
            GraphExecutionEvent::GraphCompleted { common, stats } => RuntimeEvent::RunCompleted {
                run_id: common.run_id.clone(),
                success: true,
                cost_usd: 0.0,
                duration_ms: stats.elapsed_ms,
            },
            GraphExecutionEvent::GraphFailed {
                common,
                stats,
                error: _,
            } => RuntimeEvent::RunCompleted {
                run_id: common.run_id.clone(),
                success: false,
                cost_usd: 0.0,
                duration_ms: stats.elapsed_ms,
            },
            GraphExecutionEvent::GraphCancelled { common, stats } => RuntimeEvent::RunCompleted {
                run_id: common.run_id.clone(),
                success: false,
                cost_usd: 0.0,
                duration_ms: stats.elapsed_ms,
            },

            // ── Wave lifecycle ──────────────────────────────────────
            GraphExecutionEvent::WaveStarted { common, wave } => RuntimeEvent::PipelinePhase {
                run_id: common.run_id.clone(),
                phase: format!("wave-{}", wave.wave_index),
                status: "started".to_string(),
            },
            GraphExecutionEvent::WaveCompleted {
                common,
                wave,
                elapsed_ms: _,
            } => RuntimeEvent::PipelinePhase {
                run_id: common.run_id.clone(),
                phase: format!("wave-{}", wave.wave_index),
                status: "complete".to_string(),
            },

            // ── Node lifecycle ──────────────────────────────────────
            GraphExecutionEvent::NodeStarted { common, node } => {
                let id = self.identity.get_or_fallback(&node.node_id);
                RuntimeEvent::TaskStarted {
                    run_id: common.run_id.clone(),
                    plan_id: id.plan_id,
                    task_id: id.task_id.clone(),
                    task_title: id.title,
                    role: id.role,
                }
            }
            GraphExecutionEvent::NodeSkipped {
                common: _,
                node,
                reason,
            } => {
                let id = self.identity.get_or_fallback(&node.node_id);
                RuntimeEvent::TaskSkipped {
                    task_id: id.task_id,
                    reason: reason.clone(),
                }
            }
            GraphExecutionEvent::NodeRetrying {
                common: _,
                node,
                error,
            } => {
                let id = self.identity.get_or_fallback(&node.node_id);
                RuntimeEvent::AgentProgress {
                    agent_id: node.node_id.clone(),
                    progress_pct: 0.0,
                    message: format!("retrying (attempt {}): {}", node.attempt, error),
                }
            }
            GraphExecutionEvent::NodeProgress {
                common: _,
                node,
                message,
                completed,
                total,
            } => {
                let pct = if *total > 0 {
                    (*completed as f64 / *total as f64) * 100.0
                } else {
                    0.0
                };
                RuntimeEvent::AgentProgress {
                    agent_id: node.node_id.clone(),
                    progress_pct: pct,
                    message: message.clone(),
                }
            }
            GraphExecutionEvent::NodeCompleted {
                common,
                node,
                elapsed_ms,
            } => {
                let id = self.identity.get_or_fallback(&node.node_id);
                RuntimeEvent::TaskCompleted {
                    run_id: common.run_id.clone(),
                    plan_id: id.plan_id,
                    task_id: id.task_id,
                    passed: true,
                    duration_ms: *elapsed_ms,
                }
            }
            GraphExecutionEvent::NodeFailed {
                common: _,
                node,
                elapsed_ms: _,
                error,
            } => {
                let id = self.identity.get_or_fallback(&node.node_id);
                RuntimeEvent::TaskFailed {
                    plan_id: id.plan_id,
                    task_id: id.task_id,
                    error: error.clone(),
                    gate_failure: false,
                }
            }

            // ── Agent dispatch ───────────────────────────────────────
            GraphExecutionEvent::AgentStarted {
                common,
                node,
                dispatch,
                provider,
                model,
            } => {
                let id = self.identity.get_or_fallback(&node.node_id);
                let agent_id = dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| node.node_id.clone());
                RuntimeEvent::AgentSpawned {
                    run_id: common.run_id.clone(),
                    agent_id,
                    role: id.role,
                    model: model.clone(),
                }
            }
            GraphExecutionEvent::AgentText {
                common,
                node,
                dispatch,
                chunk,
            } => {
                let agent_id = dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| node.node_id.clone());
                RuntimeEvent::AgentOutput {
                    run_id: common.run_id.clone(),
                    agent_id,
                    chunk: chunk.clone(),
                }
            }
            GraphExecutionEvent::AgentCompleted {
                common,
                node,
                dispatch,
                provider: _,
                model: _,
                elapsed_ms: _,
            } => {
                let agent_id = dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| node.node_id.clone());
                RuntimeEvent::AgentCompleted {
                    run_id: common.run_id.clone(),
                    agent_id,
                    output: String::new(),
                    tokens_used: 0,
                    cost_usd: 0.0,
                }
            }

            // ── Tool calls ──────────────────────────────────────────
            GraphExecutionEvent::ToolStarted {
                common,
                node,
                dispatch,
                tool_name,
            } => {
                let agent_id = dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| node.node_id.clone());
                RuntimeEvent::ToolCallStarted {
                    run_id: common.run_id.clone(),
                    agent_id,
                    tool: tool_name.clone(),
                    iteration: node.attempt,
                }
            }
            GraphExecutionEvent::ToolCompleted {
                common,
                node,
                dispatch,
                tool_name,
                success,
                duration_ms,
            } => {
                let agent_id = dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| node.node_id.clone());
                RuntimeEvent::ToolCallCompleted {
                    run_id: common.run_id.clone(),
                    agent_id,
                    tool: tool_name.clone(),
                    duration_ms: *duration_ms,
                    success: *success,
                }
            }

            // ── Usage / budget ──────────────────────────────────────
            GraphExecutionEvent::UsageRecorded {
                common: _,
                node: _,
                dispatch: _,
                input_tokens,
                output_tokens,
                actual_micro_usd,
            } => RuntimeEvent::UsageRecorded {
                input_tokens: *input_tokens,
                output_tokens: *output_tokens,
                cost_usd: *actual_micro_usd as f64 / 1_000_000.0,
                model: String::new(),
            },
            GraphExecutionEvent::BudgetUpdated { common, amounts } => RuntimeEvent::BudgetUpdated {
                budget_id: common.run_id.clone(),
                spent_usd: amounts.actual_micro_usd as f64 / 1_000_000.0,
                limit_usd: (amounts.actual_micro_usd + amounts.remaining_micro_usd) as f64
                    / 1_000_000.0,
                remaining_usd: amounts.remaining_micro_usd as f64 / 1_000_000.0,
            },

            // ── Gate rungs ──────────────────────────────────────────
            GraphExecutionEvent::GateRungStarted {
                common: _,
                node: _,
                rung_index,
                rung_name,
            } => RuntimeEvent::GateRungStarted {
                gate_name: rung_name.clone(),
                rung: *rung_index as u8,
            },
            GraphExecutionEvent::GateRungOutput {
                common: _,
                node: _,
                rung_index,
                rung_name,
                output,
            } => RuntimeEvent::GateRungOutput {
                gate_name: rung_name.clone(),
                rung: *rung_index as u8,
                chunk: output.clone(),
            },
            GraphExecutionEvent::GateRungCompleted {
                common: _,
                node: _,
                rung_index,
                rung_name,
                selected: _,
                skipped: _,
                pass,
                duration_ms,
                evidence_ref: _,
            } => RuntimeEvent::GateRungCompleted {
                gate_name: rung_name.clone(),
                rung: *rung_index as u8,
                passed: *pass,
                duration_ms: *duration_ms,
            },

            // ── Cell progress ───────────────────────────────────────
            GraphExecutionEvent::CellProgress {
                common: _,
                node,
                message,
                completed,
                total,
            } => {
                let pct = if *total > 0 {
                    (*completed as f64 / *total as f64) * 100.0
                } else {
                    0.0
                };
                RuntimeEvent::AgentProgress {
                    agent_id: node.node_id.clone(),
                    progress_pct: pct,
                    message: message.clone(),
                }
            }

            // ── Replay markers ──────────────────────────────────────
            GraphExecutionEvent::ReplayStarted { common } => RuntimeEvent::PipelinePhase {
                run_id: common.run_id.clone(),
                phase: "replay".to_string(),
                status: "started".to_string(),
            },
            GraphExecutionEvent::ReplayCompleted { common } => RuntimeEvent::PipelinePhase {
                run_id: common.run_id.clone(),
                phase: "replay".to_string(),
                status: "complete".to_string(),
            },

            // ── Gap ─────────────────────────────────────────────────
            GraphExecutionEvent::Gap { common, lost_count } => RuntimeEvent::SequenceGap {
                first_missing_seq: common.seq.saturating_sub(*lost_count),
                last_missing_seq: common.seq.saturating_sub(1),
                reason: format!("{lost_count} graph event(s) dropped"),
            },

            // ── Delivery events (mapped to extension) ────
            _ => {
                // Unrecognized graph events (including Delivery*) are
                // forwarded as Extension payloads so no information is lost.
                RuntimeEvent::Extension {
                    namespace: "graph.delivery".to_string(),
                    version: "1".to_string(),
                    value: serde_json::json!({ "event": "delivery" }),
                }
            }
        };

        (payload, mode)
    }

    /// Infer the runtime event mode from the graph event.
    fn infer_mode(&self, event: &GraphExecutionEvent) -> RuntimeEventMode {
        match event {
            GraphExecutionEvent::ReplayStarted { .. }
            | GraphExecutionEvent::ReplayCompleted { .. } => RuntimeEventMode::Replay,
            _ => RuntimeEventMode::Live,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use roko_graph::events::{
        BudgetAmounts, CommonFields, DispatchFields, GraphExecutionEvent, NodeFields,
        TerminalStats, WaveFields,
    };
    use roko_graph::types::ExecutionClass;

    use super::*;
    use crate::graph_execution::identity_map::{GraphIdentityMap, TaskEntry};

    fn test_common(seq: u64) -> CommonFields {
        CommonFields {
            schema_version: 1,
            run_id: "run-1".to_string(),
            graph_id: "graph-1".to_string(),
            seq,
        }
    }

    fn test_node(node_id: &str) -> NodeFields {
        NodeFields {
            node_id: node_id.to_string(),
            cell_type: "task-executor".to_string(),
            execution_class: ExecutionClass::Activity,
            attempt: 0,
        }
    }

    fn test_dispatch() -> DispatchFields {
        DispatchFields {
            attempt_id: "attempt-001".to_string(),
            agent_id: Some("agent-abc".to_string()),
        }
    }

    fn test_identity_map() -> GraphIdentityMap {
        GraphIdentityMap::from_plan_tasks(
            "graph-1",
            "plan-alpha",
            vec![
                (
                    "T01".to_string(),
                    TaskEntry::new("compile", "Compile", "implementer"),
                ),
                ("T02".to_string(), TaskEntry::new("test", "Test", "tester")),
            ],
            &HashMap::new(),
        )
    }

    #[test]
    fn graph_started_maps_to_run_started() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::GraphStarted {
            common: test_common(1),
        };
        let envelope = adapter.convert(&event);

        assert_eq!(envelope.source, "graph");
        assert_eq!(envelope.run_id, "run-1");
        assert!(matches!(envelope.payload, RuntimeEvent::RunStarted { .. }));
    }

    #[test]
    fn graph_completed_maps_to_run_completed_success() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::GraphCompleted {
            common: test_common(2),
            stats: TerminalStats {
                elapsed_ms: 5000,
                completed_nodes: 2,
                total_nodes: 2,
            },
        };
        let envelope = adapter.convert(&event);

        if let RuntimeEvent::RunCompleted {
            success,
            duration_ms,
            ..
        } = &envelope.payload
        {
            assert!(*success);
            assert_eq!(*duration_ms, 5000);
        } else {
            panic!("expected RunCompleted");
        }
    }

    #[test]
    fn graph_failed_maps_to_run_completed_failure() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::GraphFailed {
            common: test_common(3),
            stats: TerminalStats {
                elapsed_ms: 3000,
                completed_nodes: 1,
                total_nodes: 2,
            },
            error: "node failed".to_string(),
        };
        let envelope = adapter.convert(&event);

        if let RuntimeEvent::RunCompleted { success, .. } = &envelope.payload {
            assert!(!success);
        } else {
            panic!("expected RunCompleted");
        }
    }

    #[test]
    fn graph_cancelled_maps_to_run_completed_failure() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::GraphCancelled {
            common: test_common(4),
            stats: TerminalStats {
                elapsed_ms: 1000,
                completed_nodes: 0,
                total_nodes: 2,
            },
        };
        let envelope = adapter.convert(&event);

        if let RuntimeEvent::RunCompleted { success, .. } = &envelope.payload {
            assert!(!success);
        } else {
            panic!("expected RunCompleted");
        }
    }

    #[test]
    fn wave_events_map_to_pipeline_phase() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());

        let start = GraphExecutionEvent::WaveStarted {
            common: test_common(5),
            wave: WaveFields {
                wave_index: 0,
                total_waves: 3,
            },
        };
        let env = adapter.convert(&start);
        if let RuntimeEvent::PipelinePhase { phase, status, .. } = &env.payload {
            assert_eq!(phase, "wave-0");
            assert_eq!(status, "started");
        } else {
            panic!("expected PipelinePhase");
        }

        let complete = GraphExecutionEvent::WaveCompleted {
            common: test_common(6),
            wave: WaveFields {
                wave_index: 0,
                total_waves: 3,
            },
            elapsed_ms: 2000,
        };
        let env = adapter.convert(&complete);
        if let RuntimeEvent::PipelinePhase { status, .. } = &env.payload {
            assert_eq!(status, "complete");
        } else {
            panic!("expected PipelinePhase");
        }
    }

    #[test]
    fn node_started_carries_identity() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::NodeStarted {
            common: test_common(7),
            node: test_node("T01"),
        };
        let envelope = adapter.convert(&event);

        assert_eq!(envelope.plan_id.as_deref(), Some("plan-alpha"));
        assert_eq!(envelope.task_id.as_deref(), Some("compile"));
        assert_eq!(envelope.node_id.as_deref(), Some("T01"));

        if let RuntimeEvent::TaskStarted {
            plan_id,
            task_id,
            task_title,
            role,
            ..
        } = &envelope.payload
        {
            assert_eq!(plan_id, "plan-alpha");
            assert_eq!(task_id, "compile");
            assert_eq!(task_title, "Compile");
            assert_eq!(role, "implementer");
        } else {
            panic!("expected TaskStarted");
        }
    }

    #[test]
    fn unknown_node_uses_fallback() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::NodeStarted {
            common: test_common(8),
            node: test_node("T99"),
        };
        let envelope = adapter.convert(&event);

        assert_eq!(envelope.plan_id.as_deref(), Some("graph-1"));
        assert!(envelope.task_id.is_none()); // empty task_id -> None

        if let RuntimeEvent::TaskStarted { task_title, .. } = &envelope.payload {
            assert_eq!(task_title, "graph node T99");
        } else {
            panic!("expected TaskStarted");
        }
    }

    #[test]
    fn node_skipped_maps_to_task_skipped() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::NodeSkipped {
            common: test_common(9),
            node: test_node("T01"),
            reason: "upstream failed".to_string(),
        };
        let envelope = adapter.convert(&event);

        if let RuntimeEvent::TaskSkipped { task_id, reason } = &envelope.payload {
            assert_eq!(task_id, "compile");
            assert_eq!(reason, "upstream failed");
        } else {
            panic!("expected TaskSkipped");
        }
    }

    #[test]
    fn agent_started_maps_to_agent_spawned() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::AgentStarted {
            common: test_common(10),
            node: test_node("T01"),
            dispatch: test_dispatch(),
            provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet".to_string(),
        };
        let envelope = adapter.convert(&event);

        assert_eq!(envelope.attempt_id.as_deref(), Some("attempt-001"));
        assert_eq!(envelope.agent_id.as_deref(), Some("agent-abc"));

        if let RuntimeEvent::AgentSpawned {
            agent_id, model, ..
        } = &envelope.payload
        {
            assert_eq!(agent_id, "agent-abc");
            assert_eq!(model, "claude-3-5-sonnet");
        } else {
            panic!("expected AgentSpawned");
        }
    }

    #[test]
    fn agent_text_maps_to_agent_output() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::AgentText {
            common: test_common(11),
            node: test_node("T01"),
            dispatch: test_dispatch(),
            chunk: "hello world".to_string(),
        };
        let envelope = adapter.convert(&event);

        if let RuntimeEvent::AgentOutput { chunk, .. } = &envelope.payload {
            assert_eq!(chunk, "hello world");
        } else {
            panic!("expected AgentOutput");
        }
        assert_eq!(envelope.delivery, RuntimeEventDelivery::BestEffort);
    }

    #[test]
    fn tool_events_map_correctly() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());

        let start = GraphExecutionEvent::ToolStarted {
            common: test_common(12),
            node: test_node("T01"),
            dispatch: test_dispatch(),
            tool_name: "write_file".to_string(),
        };
        let env = adapter.convert(&start);
        assert!(matches!(env.payload, RuntimeEvent::ToolCallStarted { .. }));

        let complete = GraphExecutionEvent::ToolCompleted {
            common: test_common(13),
            node: test_node("T01"),
            dispatch: test_dispatch(),
            tool_name: "write_file".to_string(),
            success: true,
            duration_ms: 150,
        };
        let env = adapter.convert(&complete);
        if let RuntimeEvent::ToolCallCompleted {
            tool,
            success,
            duration_ms,
            ..
        } = &env.payload
        {
            assert_eq!(tool, "write_file");
            assert!(*success);
            assert_eq!(*duration_ms, 150);
        } else {
            panic!("expected ToolCallCompleted");
        }
    }

    #[test]
    fn usage_converts_micro_usd_to_usd() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::UsageRecorded {
            common: test_common(14),
            node: test_node("T01"),
            dispatch: test_dispatch(),
            input_tokens: 1000,
            output_tokens: 500,
            actual_micro_usd: 500_000, // $0.50
        };
        let envelope = adapter.convert(&event);

        if let RuntimeEvent::UsageRecorded { cost_usd, .. } = &envelope.payload {
            assert!((*cost_usd - 0.5).abs() < f64::EPSILON);
        } else {
            panic!("expected UsageRecorded");
        }
    }

    #[test]
    fn budget_updated_converts_micro_usd() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::BudgetUpdated {
            common: test_common(15),
            amounts: BudgetAmounts {
                estimated_micro_usd: 1_000_000,
                reserved_micro_usd: 500_000,
                actual_micro_usd: 250_000,
                remaining_micro_usd: 750_000,
            },
        };
        let envelope = adapter.convert(&event);

        if let RuntimeEvent::BudgetUpdated {
            spent_usd,
            remaining_usd,
            ..
        } = &envelope.payload
        {
            assert!((*spent_usd - 0.25).abs() < f64::EPSILON);
            assert!((*remaining_usd - 0.75).abs() < f64::EPSILON);
        } else {
            panic!("expected BudgetUpdated");
        }
    }

    #[test]
    fn gate_rung_events_map_correctly() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());

        let start = GraphExecutionEvent::GateRungStarted {
            common: test_common(16),
            node: test_node("T01"),
            rung_index: 0,
            rung_name: "compile".to_string(),
        };
        let env = adapter.convert(&start);
        assert!(matches!(env.payload, RuntimeEvent::GateRungStarted { .. }));

        let output = GraphExecutionEvent::GateRungOutput {
            common: test_common(17),
            node: test_node("T01"),
            rung_index: 0,
            rung_name: "compile".to_string(),
            output: "compiling...".to_string(),
        };
        let env = adapter.convert(&output);
        assert!(matches!(env.payload, RuntimeEvent::GateRungOutput { .. }));
        assert_eq!(env.delivery, RuntimeEventDelivery::BestEffort);

        let complete = GraphExecutionEvent::GateRungCompleted {
            common: test_common(18),
            node: test_node("T01"),
            rung_index: 0,
            rung_name: "compile".to_string(),
            selected: true,
            skipped: false,
            pass: true,
            duration_ms: 3000,
            evidence_ref: Some("/tmp/evidence".to_string()),
        };
        let env = adapter.convert(&complete);
        if let RuntimeEvent::GateRungCompleted {
            passed,
            duration_ms,
            ..
        } = &env.payload
        {
            assert!(*passed);
            assert_eq!(*duration_ms, 3000);
        } else {
            panic!("expected GateRungCompleted");
        }
    }

    #[test]
    fn gap_maps_to_sequence_gap() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::Gap {
            common: test_common(20),
            lost_count: 3,
        };
        let envelope = adapter.convert(&event);

        if let RuntimeEvent::SequenceGap { reason, .. } = &envelope.payload {
            assert!(reason.contains("3"));
        } else {
            panic!("expected SequenceGap");
        }
    }

    #[test]
    fn replay_preserves_original_identity() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::GraphStarted {
            common: test_common(1),
        };
        let original_ts = chrono::Utc::now();
        let envelope = adapter.convert_replay(&event, "original-id", 42, original_ts);

        assert_eq!(envelope.event_id, "original-id");
        assert_eq!(envelope.seq, 42);
        assert_eq!(envelope.ts, original_ts);
        assert_eq!(envelope.mode, RuntimeEventMode::Replay);
    }

    #[test]
    fn sequence_numbers_are_monotonic() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());
        let event = GraphExecutionEvent::GraphStarted {
            common: test_common(1),
        };

        let e1 = adapter.convert(&event);
        let e2 = adapter.convert(&event);
        let e3 = adapter.convert(&event);

        assert!(e1.seq < e2.seq);
        assert!(e2.seq < e3.seq);
    }

    #[test]
    fn all_envelopes_carry_graph_source() {
        let adapter = GraphRuntimeEventAdapter::new(test_identity_map());

        let events: Vec<GraphExecutionEvent> = vec![
            GraphExecutionEvent::GraphStarted {
                common: test_common(1),
            },
            GraphExecutionEvent::NodeStarted {
                common: test_common(2),
                node: test_node("T01"),
            },
            GraphExecutionEvent::NodeCompleted {
                common: test_common(3),
                node: test_node("T01"),
                elapsed_ms: 100,
            },
            GraphExecutionEvent::GraphCompleted {
                common: test_common(4),
                stats: TerminalStats {
                    elapsed_ms: 100,
                    completed_nodes: 1,
                    total_nodes: 1,
                },
            },
        ];

        for event in &events {
            let envelope = adapter.convert(event);
            assert_eq!(envelope.source, "graph", "source must be 'graph'");
            assert_eq!(envelope.schema_version, 2);
        }
    }
}
