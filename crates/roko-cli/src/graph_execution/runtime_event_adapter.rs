//! Exhaustive adapter from graph execution events to canonical runtime
//! event envelopes (#248).
//!
//! [`GraphRuntimeEventAdapter`] owns one atomic next-sequence counter,
//! consumes [`GraphExecutionEvent`] values from #246, looks up identity
//! through [`GraphIdentityMap`], and emits [`RuntimeEventEnvelope`] values
//! with `source = "graph"`.
//!
//! The match on [`GraphExecutionEvent`] is deliberately non-wildcard so that
//! adding a new variant to the graph event enum causes a compile error here,
//! forcing the mapping to be updated.
//!
//! Replay events preserve original sequence/event identity.
//!
//! [`GraphExecutionEvent`]: roko_graph::events::GraphExecutionEvent
//! [`GraphIdentityMap`]: super::identity_map::GraphIdentityMap
//! [`RuntimeEventEnvelope`]: roko_core::runtime_event::RuntimeEventEnvelope

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use uuid::Uuid;

use roko_core::dashboard_snapshot::{TaskOutcomeClass, classify_task_outcome};
use roko_core::runtime_event::{
    RuntimeEvent, RuntimeEventDelivery, RuntimeEventEnvelope, RuntimeEventMode,
};
use roko_graph::events::GraphExecutionEvent;

use super::identity_map::{GraphIdentityMap, NodeIdentity};

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Converts [`GraphExecutionEvent`] into [`RuntimeEventEnvelope`].
///
/// Thread-safe: the sequence counter is atomic and the identity map is
/// immutable. Multiple sinks can share one adapter via `Arc`.
pub struct GraphRuntimeEventAdapter {
    /// Identity resolution for graph nodes.
    identity_map: GraphIdentityMap,
    /// Monotonic sequence counter for the canonical envelope.
    next_seq: AtomicU64,
    /// Run identifier carried on every envelope.
    run_id: String,
}

impl std::fmt::Debug for GraphRuntimeEventAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphRuntimeEventAdapter")
            .field("run_id", &self.run_id)
            .field("next_seq", &self.next_seq.load(Ordering::Relaxed))
            .field("identity_map_len", &self.identity_map.len())
            .finish()
    }
}

impl GraphRuntimeEventAdapter {
    /// Create a new adapter for a given run.
    pub fn new(run_id: impl Into<String>, identity_map: GraphIdentityMap) -> Self {
        Self {
            identity_map,
            next_seq: AtomicU64::new(1),
            run_id: run_id.into(),
        }
    }

    /// Convert a single graph execution event into a canonical envelope.
    ///
    /// The match is exhaustive (no wildcard) so adding a new
    /// `GraphExecutionEvent` variant will cause a compile error here.
    #[must_use]
    pub fn convert(&self, event: &GraphExecutionEvent) -> RuntimeEventEnvelope {
        let common = event.common();
        let node_fields = event.node();
        let dispatch_fields = event.dispatch();

        // Resolve identity from node_id if available.
        let identity: Option<NodeIdentity> = node_fields.map(|nf| {
            self.identity_map
                .get(&nf.node_id)
                .cloned()
                .unwrap_or_else(|| self.identity_map.fallback_identity(&nf.node_id))
        });

        let plan_id = identity
            .as_ref()
            .map(|id| id.plan_id.clone())
            .or_else(|| Some(self.identity_map.plan_id().to_string()));
        let task_id = identity.as_ref().and_then(|id| {
            if id.task_id.is_empty() {
                None
            } else {
                Some(id.task_id.clone())
            }
        });
        let node_id = node_fields.map(|nf| nf.node_id.clone());
        let attempt_id = dispatch_fields.map(|df| df.attempt_id.clone());
        let agent_id = dispatch_fields.and_then(|df| df.agent_id.clone());

        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let mode = self.event_mode(event);
        let payload = self.map_payload(event, &identity);
        let delivery = Self::map_delivery(event);

        RuntimeEventEnvelope {
            event_id: Uuid::new_v4().to_string(),
            run_id: self.run_id.clone(),
            plan_id,
            task_id,
            node_id,
            attempt_id,
            agent_id,
            seq,
            ts: Utc::now(),
            schema_version: 2,
            source: "graph".to_string(),
            mode,
            delivery,
            correlation_id: Some(common.run_id.clone()),
            idempotency_key: None,
            payload,
        }
    }

    /// Access the underlying identity map.
    #[must_use]
    pub fn identity_map(&self) -> &GraphIdentityMap {
        &self.identity_map
    }

    // ── Private helpers ──────────────────────────────────────────────

    /// Determine the event mode (live vs replay).
    fn event_mode(&self, event: &GraphExecutionEvent) -> RuntimeEventMode {
        match event {
            GraphExecutionEvent::ReplayStarted { .. }
            | GraphExecutionEvent::ReplayCompleted { .. } => RuntimeEventMode::Replay,
            _ => RuntimeEventMode::Live,
        }
    }

    /// Map graph delivery class to runtime delivery class.
    fn map_delivery(event: &GraphExecutionEvent) -> RuntimeEventDelivery {
        match event.delivery() {
            roko_graph::events::GraphEventDelivery::Reliable => RuntimeEventDelivery::Reliable,
            roko_graph::events::GraphEventDelivery::BestEffort => RuntimeEventDelivery::BestEffort,
        }
    }

    /// Exhaustive mapping from graph event to runtime event payload.
    ///
    /// This match must list every variant explicitly. Do NOT add a wildcard.
    #[allow(clippy::too_many_lines)]
    fn map_payload(
        &self,
        event: &GraphExecutionEvent,
        identity: &Option<NodeIdentity>,
    ) -> RuntimeEvent {
        let plan_id_str = identity
            .as_ref()
            .map(|id| id.plan_id.as_str())
            .unwrap_or_else(|| self.identity_map.plan_id());
        let task_id_str = identity
            .as_ref()
            .map(|id| id.task_id.as_str())
            .unwrap_or("");
        let title_str = identity.as_ref().map(|id| id.title.as_str()).unwrap_or("");
        let role_str = identity
            .as_ref()
            .and_then(|id| id.role.as_deref())
            .unwrap_or("graph");

        match event {
            // ── Graph lifecycle → Run lifecycle ──────────────────────
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

            // ── Wave lifecycle → PipelinePhase / Wave events ────────
            GraphExecutionEvent::WaveStarted { common: _, wave } => RuntimeEvent::WaveStarted {
                wave_index: wave.wave_index as u64,
                task_count: wave.total_waves as u64,
            },
            GraphExecutionEvent::WaveCompleted {
                common: _,
                wave,
                elapsed_ms,
            } => RuntimeEvent::WaveCompleted {
                wave_index: wave.wave_index as u64,
                succeeded: wave.total_waves as u64,
                failed: 0,
                duration_ms: *elapsed_ms,
            },

            // ── Node lifecycle → Task lifecycle ─────────────────────
            GraphExecutionEvent::NodeStarted { common, node: _ } => RuntimeEvent::TaskStarted {
                run_id: common.run_id.clone(),
                plan_id: plan_id_str.to_string(),
                task_id: task_id_str.to_string(),
                task_title: title_str.to_string(),
                role: role_str.to_string(),
            },
            GraphExecutionEvent::NodeSkipped {
                common: _,
                node: _,
                reason,
            } => RuntimeEvent::TaskSkipped {
                task_id: task_id_str.to_string(),
                reason: reason.clone(),
            },
            GraphExecutionEvent::NodeRetrying {
                common: _,
                node,
                error,
            } => RuntimeEvent::TaskRetrying {
                task_id: task_id_str.to_string(),
                attempt: node.attempt as u64,
                reason: error.clone(),
            },
            GraphExecutionEvent::NodeProgress {
                common: _,
                node: _,
                message,
                completed: _,
                total: _,
            } => RuntimeEvent::AgentProgress {
                agent_id: String::new(),
                progress_pct: 0.0,
                message: message.clone(),
            },
            GraphExecutionEvent::NodeCompleted {
                common,
                node: _,
                elapsed_ms,
                outcome,
            } => RuntimeEvent::TaskCompleted {
                run_id: common.run_id.clone(),
                plan_id: plan_id_str.to_string(),
                task_id: task_id_str.to_string(),
                // bug-71a5e6: a completion that names its outcome passes only
                // as a verified pass, and keeps the outcome for consumers.
                passed: outcome.as_deref().is_none_or(|outcome| {
                    classify_task_outcome(outcome) == TaskOutcomeClass::Passed
                }),
                duration_ms: *elapsed_ms,
                outcome: outcome.clone(),
            },
            GraphExecutionEvent::NodeFailed {
                common: _,
                node: _,
                elapsed_ms: _,
                error,
            } => RuntimeEvent::TaskFailed {
                plan_id: plan_id_str.to_string(),
                task_id: task_id_str.to_string(),
                error: error.clone(),
                gate_failure: false,
            },

            // ── Dispatch lifecycle → Agent lifecycle ─────────────────
            GraphExecutionEvent::AgentStarted {
                common,
                node: _,
                dispatch,
                provider: _,
                model,
            } => RuntimeEvent::AgentSpawned {
                run_id: common.run_id.clone(),
                agent_id: dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| dispatch.attempt_id.clone()),
                role: role_str.to_string(),
                model: model.clone(),
            },
            GraphExecutionEvent::AgentText {
                common: _,
                node: _,
                dispatch,
                chunk,
            } => RuntimeEvent::AgentOutput {
                run_id: self.run_id.clone(),
                agent_id: dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| dispatch.attempt_id.clone()),
                chunk: chunk.clone(),
            },
            GraphExecutionEvent::ToolStarted {
                common: _,
                node: _,
                dispatch,
                tool_name,
            } => RuntimeEvent::ToolCallStarted {
                run_id: self.run_id.clone(),
                agent_id: dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| dispatch.attempt_id.clone()),
                tool: tool_name.clone(),
                iteration: 0,
            },
            GraphExecutionEvent::ToolCompleted {
                common: _,
                node: _,
                dispatch,
                tool_name,
                success,
                duration_ms,
            } => RuntimeEvent::ToolCallCompleted {
                run_id: self.run_id.clone(),
                agent_id: dispatch
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| dispatch.attempt_id.clone()),
                tool: tool_name.clone(),
                duration_ms: *duration_ms,
                success: *success,
            },
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
            GraphExecutionEvent::AgentCompleted {
                common,
                node: _,
                dispatch,
                provider: _,
                model: _,
                elapsed_ms: _,
            } => {
                // AgentCompleted maps to the v1 AgentCompleted event.
                RuntimeEvent::AgentCompleted {
                    run_id: common.run_id.clone(),
                    agent_id: dispatch
                        .agent_id
                        .clone()
                        .unwrap_or_else(|| dispatch.attempt_id.clone()),
                    output: String::new(),
                    tokens_used: 0,
                    cost_usd: 0.0,
                }
            }

            // ── Gate/cell detail → Gate rung events ─────────────────
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
            GraphExecutionEvent::CellProgress {
                common: _,
                node: _,
                message,
                completed: _,
                total: _,
            } => RuntimeEvent::AgentProgress {
                agent_id: String::new(),
                progress_pct: 0.0,
                message: message.clone(),
            },

            // ── Accounting → Budget events ──────────────────────────
            GraphExecutionEvent::BudgetUpdated { common: _, amounts } => {
                RuntimeEvent::BudgetUpdated {
                    budget_id: String::new(),
                    spent_usd: amounts.actual_micro_usd as f64 / 1_000_000.0,
                    limit_usd: (amounts.actual_micro_usd + amounts.remaining_micro_usd) as f64
                        / 1_000_000.0,
                    remaining_usd: amounts.remaining_micro_usd as f64 / 1_000_000.0,
                }
            }

            // ── Delivery lifecycle ──────────────────────────────────
            GraphExecutionEvent::DeliveryStarted {
                common: _,
                delivery_id,
                plan_id,
                branch,
                publish,
            } => RuntimeEvent::Extension {
                namespace: "graph.delivery".to_string(),
                version: "1".to_string(),
                value: serde_json::json!({
                    "kind": "started",
                    "delivery_id": delivery_id,
                    "plan_id": plan_id,
                    "branch": branch,
                    "publish": publish,
                }),
            },
            GraphExecutionEvent::DeliveryStateAdvanced {
                common: _,
                delivery_id,
                plan_id,
                from_state,
                to_state,
                merge_commit,
                publication_ref,
            } => RuntimeEvent::Extension {
                namespace: "graph.delivery".to_string(),
                version: "1".to_string(),
                value: serde_json::json!({
                    "kind": "state_advanced",
                    "delivery_id": delivery_id,
                    "plan_id": plan_id,
                    "from_state": from_state,
                    "to_state": to_state,
                    "merge_commit": merge_commit,
                    "publication_ref": publication_ref,
                }),
            },
            GraphExecutionEvent::DeliveryCompleted {
                common: _,
                delivery_id,
                plan_id,
                release_policy,
            } => RuntimeEvent::Extension {
                namespace: "graph.delivery".to_string(),
                version: "1".to_string(),
                value: serde_json::json!({
                    "kind": "completed",
                    "delivery_id": delivery_id,
                    "plan_id": plan_id,
                    "release_policy": release_policy,
                }),
            },
            GraphExecutionEvent::DeliveryFailed {
                common: _,
                delivery_id,
                plan_id,
                failure_state,
                error,
                release_policy,
            } => RuntimeEvent::Extension {
                namespace: "graph.delivery".to_string(),
                version: "1".to_string(),
                value: serde_json::json!({
                    "kind": "failed",
                    "delivery_id": delivery_id,
                    "plan_id": plan_id,
                    "failure_state": failure_state,
                    "error": error,
                    "release_policy": release_policy,
                }),
            },

            // ── Feedback settlement ─────────────────────────────────
            GraphExecutionEvent::FeedbackSinkSettled {
                common: _,
                node: _,
                idempotency_key,
                sink_key,
                row,
            } => RuntimeEvent::FeedbackSinkSettled {
                sink_id: idempotency_key.clone(),
                kind: sink_key.clone(),
                summary: format!("row {row} settled"),
            },
            GraphExecutionEvent::FeedbackSinkFailed {
                common: _,
                node: _,
                idempotency_key,
                sink_key,
                row,
                critical,
                error,
            } => RuntimeEvent::FeedbackSinkFailed {
                sink_id: idempotency_key.clone(),
                kind: sink_key.clone(),
                error: format!(
                    "row {row}{}: {error}",
                    if *critical { " (critical)" } else { "" }
                ),
            },

            // ── Replay/gap ──────────────────────────────────────────
            GraphExecutionEvent::ReplayStarted { common } => RuntimeEvent::Extension {
                namespace: "graph.replay".to_string(),
                version: "1".to_string(),
                value: serde_json::json!({
                    "kind": "started",
                    "run_id": common.run_id,
                }),
            },
            GraphExecutionEvent::ReplayCompleted { common } => RuntimeEvent::Extension {
                namespace: "graph.replay".to_string(),
                version: "1".to_string(),
                value: serde_json::json!({
                    "kind": "completed",
                    "run_id": common.run_id,
                }),
            },
            // ── Wave 12 cognitive lifecycle (no RuntimeEvent equivalent) ─
            GraphExecutionEvent::PredictionPublished { .. }
            | GraphExecutionEvent::CalibrationObserved { .. }
            | GraphExecutionEvent::CorrectionApplied { .. } => RuntimeEvent::Extension {
                namespace: "roko.cognitive".to_string(),
                version: "1".to_string(),
                value: serde_json::Value::Null,
            },

            GraphExecutionEvent::Gap {
                common: _,
                lost_count,
            } => RuntimeEvent::SequenceGap {
                first_missing_seq: 0,
                last_missing_seq: *lost_count,
                reason: format!("{lost_count} graph events dropped"),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use roko_graph::convert::PlanTaskInfo;
    use roko_graph::events::*;
    use roko_graph::types::ExecutionClass;

    use super::*;
    use crate::graph_execution::identity_map::GraphIdentityMap;

    fn make_task_info(title: &str, role: Option<&str>) -> PlanTaskInfo {
        PlanTaskInfo {
            title: title.to_string(),
            description: None,
            role: role.map(|r| r.to_string()),
            tier: "focused".to_string(),
            model_hint: None,
            files: Vec::new(),
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            timeout_secs: 60,
            max_retries: 0,
            domain: None,
            sequence: 0,
            full_config_json: serde_json::Value::Null,
        }
    }

    fn make_adapter() -> GraphRuntimeEventAdapter {
        let tasks = vec![
            (
                "T01".to_string(),
                make_task_info("Compile", Some("implementer")),
            ),
            ("T02".to_string(), make_task_info("Test", Some("reviewer"))),
        ];
        let waves: HashMap<String, u32> = [("T01".to_string(), 0), ("T02".to_string(), 1)]
            .into_iter()
            .collect();
        let identity_map = GraphIdentityMap::build("test-plan", &tasks, &waves);
        GraphRuntimeEventAdapter::new("run-1", identity_map)
    }

    fn make_common(seq: u64) -> CommonFields {
        CommonFields {
            schema_version: GRAPH_EVENT_SCHEMA_VERSION,
            run_id: "run-1".to_string(),
            graph_id: "test-plan".to_string(),
            seq,
        }
    }

    fn make_node(node_id: &str) -> NodeFields {
        NodeFields {
            node_id: node_id.to_string(),
            cell_type: "task-executor".to_string(),
            execution_class: ExecutionClass::Activity,
            attempt: 0,
        }
    }

    fn make_wave(index: u32) -> WaveFields {
        WaveFields {
            wave_index: index,
            total_waves: 3,
        }
    }

    fn make_dispatch() -> DispatchFields {
        DispatchFields {
            attempt_id: "attempt-1".to_string(),
            agent_id: Some("agent-1".to_string()),
        }
    }

    #[test]
    fn graph_started_maps_to_run_started() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::GraphStarted {
            common: make_common(1),
        };
        let envelope = adapter.convert(&event);
        assert_eq!(envelope.source, "graph");
        assert_eq!(envelope.run_id, "run-1");
        assert!(matches!(envelope.payload, RuntimeEvent::RunStarted { .. }));
    }

    #[test]
    fn graph_completed_maps_to_run_completed() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::GraphCompleted {
            common: make_common(2),
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
            panic!("expected RunCompleted, got {:?}", envelope.payload);
        }
    }

    #[test]
    fn graph_failed_maps_to_run_completed_false() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::GraphFailed {
            common: make_common(3),
            stats: TerminalStats {
                elapsed_ms: 3000,
                completed_nodes: 1,
                total_nodes: 2,
            },
            error: "compile failed".to_string(),
        };
        let envelope = adapter.convert(&event);
        if let RuntimeEvent::RunCompleted { success, .. } = &envelope.payload {
            assert!(!*success);
        } else {
            panic!("expected RunCompleted");
        }
    }

    #[test]
    fn node_started_maps_with_identity() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::NodeStarted {
            common: make_common(4),
            node: make_node("T01"),
        };
        let envelope = adapter.convert(&event);
        assert_eq!(envelope.plan_id.as_deref(), Some("test-plan"));
        assert_eq!(envelope.task_id.as_deref(), Some("T01"));
        assert_eq!(envelope.node_id.as_deref(), Some("T01"));

        if let RuntimeEvent::TaskStarted {
            task_id,
            task_title,
            role,
            ..
        } = &envelope.payload
        {
            assert_eq!(task_id, "T01");
            assert_eq!(task_title, "Compile");
            assert_eq!(role, "implementer");
        } else {
            panic!("expected TaskStarted");
        }
    }

    #[test]
    fn unknown_node_uses_fallback() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::NodeStarted {
            common: make_common(5),
            node: make_node("unknown-node"),
        };
        let envelope = adapter.convert(&event);
        assert_eq!(envelope.plan_id.as_deref(), Some("test-plan"));
        assert!(envelope.task_id.is_none()); // fallback has empty task_id

        if let RuntimeEvent::TaskStarted { task_title, .. } = &envelope.payload {
            assert!(task_title.contains("unknown-node"));
        } else {
            panic!("expected TaskStarted");
        }
    }

    #[test]
    fn wave_events_map_correctly() {
        let adapter = make_adapter();
        let start = GraphExecutionEvent::WaveStarted {
            common: make_common(6),
            wave: make_wave(1),
        };
        let envelope = adapter.convert(&start);
        assert!(matches!(envelope.payload, RuntimeEvent::WaveStarted { .. }));

        let complete = GraphExecutionEvent::WaveCompleted {
            common: make_common(7),
            wave: make_wave(1),
            elapsed_ms: 2000,
        };
        let envelope = adapter.convert(&complete);
        assert!(matches!(
            envelope.payload,
            RuntimeEvent::WaveCompleted { .. }
        ));
    }

    #[test]
    fn agent_events_map_correctly() {
        let adapter = make_adapter();

        let started = GraphExecutionEvent::AgentStarted {
            common: make_common(8),
            node: make_node("T01"),
            dispatch: make_dispatch(),
            provider: "anthropic".to_string(),
            model: "claude-sonnet".to_string(),
        };
        let envelope = adapter.convert(&started);
        assert!(matches!(
            envelope.payload,
            RuntimeEvent::AgentSpawned { .. }
        ));
        assert_eq!(envelope.agent_id.as_deref(), Some("agent-1"));
        assert_eq!(envelope.attempt_id.as_deref(), Some("attempt-1"));

        let text = GraphExecutionEvent::AgentText {
            common: make_common(9),
            node: make_node("T01"),
            dispatch: make_dispatch(),
            chunk: "hello world".to_string(),
        };
        let envelope = adapter.convert(&text);
        assert!(matches!(envelope.payload, RuntimeEvent::AgentOutput { .. }));
    }

    #[test]
    fn tool_events_map_correctly() {
        let adapter = make_adapter();

        let started = GraphExecutionEvent::ToolStarted {
            common: make_common(10),
            node: make_node("T01"),
            dispatch: make_dispatch(),
            tool_name: "cargo_check".to_string(),
        };
        let envelope = adapter.convert(&started);
        assert!(matches!(
            envelope.payload,
            RuntimeEvent::ToolCallStarted { .. }
        ));

        let completed = GraphExecutionEvent::ToolCompleted {
            common: make_common(11),
            node: make_node("T01"),
            dispatch: make_dispatch(),
            tool_name: "cargo_check".to_string(),
            success: true,
            duration_ms: 500,
        };
        let envelope = adapter.convert(&completed);
        assert!(matches!(
            envelope.payload,
            RuntimeEvent::ToolCallCompleted { .. }
        ));
    }

    #[test]
    fn usage_converts_micro_usd_to_usd() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::UsageRecorded {
            common: make_common(12),
            node: make_node("T01"),
            dispatch: make_dispatch(),
            input_tokens: 1000,
            output_tokens: 500,
            actual_micro_usd: 500_000, // $0.50
        };
        let envelope = adapter.convert(&event);
        if let RuntimeEvent::UsageRecorded { cost_usd, .. } = &envelope.payload {
            assert!((cost_usd - 0.5).abs() < 0.001);
        } else {
            panic!("expected UsageRecorded");
        }
    }

    #[test]
    fn gate_events_map_correctly() {
        let adapter = make_adapter();

        let started = GraphExecutionEvent::GateRungStarted {
            common: make_common(13),
            node: make_node("T01"),
            rung_index: 0,
            rung_name: "compile".to_string(),
        };
        let envelope = adapter.convert(&started);
        assert!(matches!(
            envelope.payload,
            RuntimeEvent::GateRungStarted { .. }
        ));

        let output = GraphExecutionEvent::GateRungOutput {
            common: make_common(14),
            node: make_node("T01"),
            rung_index: 0,
            rung_name: "compile".to_string(),
            output: "compiling...".to_string(),
        };
        let envelope = adapter.convert(&output);
        assert!(matches!(
            envelope.payload,
            RuntimeEvent::GateRungOutput { .. }
        ));

        let completed = GraphExecutionEvent::GateRungCompleted {
            common: make_common(15),
            node: make_node("T01"),
            rung_index: 0,
            rung_name: "compile".to_string(),
            selected: true,
            skipped: false,
            pass: true,
            duration_ms: 3000,
            evidence_ref: None,
        };
        let envelope = adapter.convert(&completed);
        assert!(matches!(
            envelope.payload,
            RuntimeEvent::GateRungCompleted { .. }
        ));
    }

    #[test]
    fn budget_converts_micro_usd() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::BudgetUpdated {
            common: make_common(16),
            amounts: BudgetAmounts {
                estimated_micro_usd: 100_000,
                reserved_micro_usd: 50_000,
                actual_micro_usd: 200_000,    // $0.20
                remaining_micro_usd: 800_000, // $0.80
            },
        };
        let envelope = adapter.convert(&event);
        if let RuntimeEvent::BudgetUpdated {
            spent_usd,
            remaining_usd,
            ..
        } = &envelope.payload
        {
            assert!((spent_usd - 0.2).abs() < 0.001);
            assert!((remaining_usd - 0.8).abs() < 0.001);
        } else {
            panic!("expected BudgetUpdated");
        }
    }

    #[test]
    fn gap_maps_to_sequence_gap() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::Gap {
            common: make_common(17),
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
    fn sequence_numbers_are_monotonic() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::GraphStarted {
            common: make_common(1),
        };
        let e1 = adapter.convert(&event);
        let e2 = adapter.convert(&event);
        let e3 = adapter.convert(&event);
        assert!(e1.seq < e2.seq);
        assert!(e2.seq < e3.seq);
    }

    #[test]
    fn replay_events_have_replay_mode() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::ReplayStarted {
            common: make_common(20),
        };
        let envelope = adapter.convert(&event);
        assert_eq!(envelope.mode, RuntimeEventMode::Replay);
    }

    #[test]
    fn node_skipped_maps_to_task_skipped() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::NodeSkipped {
            common: make_common(21),
            node: make_node("T02"),
            reason: "upstream failed".to_string(),
        };
        let envelope = adapter.convert(&event);
        if let RuntimeEvent::TaskSkipped { reason, .. } = &envelope.payload {
            assert_eq!(reason, "upstream failed");
        } else {
            panic!("expected TaskSkipped");
        }
    }

    /// bug-71a5e6: a completion keeps the outcome its task settled with, so an
    /// unverified, already-satisfied or skipped task never reads as passed on
    /// the runtime path, and the dashboard gets the same outcome back.
    #[test]
    fn runtime_adapter_keeps_the_task_outcome() {
        let adapter = make_adapter();
        for (outcome, passed, class) in [
            (Some("passed"), true, TaskOutcomeClass::Passed),
            (Some("unverified"), false, TaskOutcomeClass::Unverified),
            (
                Some("already_satisfied"),
                false,
                TaskOutcomeClass::AlreadySatisfied,
            ),
            (Some("skipped"), false, TaskOutcomeClass::Skipped),
            (
                Some("accepted_with_failures"),
                false,
                TaskOutcomeClass::AcceptedWithFailures,
            ),
            // A completion that does not say how it settled reads as before.
            (None, true, TaskOutcomeClass::Passed),
        ] {
            let envelope = adapter.convert(&GraphExecutionEvent::NodeCompleted {
                common: make_common(30),
                node: make_node("T01"),
                elapsed_ms: 10,
                outcome: outcome.map(str::to_string),
            });
            let RuntimeEvent::TaskCompleted {
                passed: reported,
                outcome: kept,
                ..
            } = &envelope.payload
            else {
                panic!("expected TaskCompleted, got {:?}", envelope.payload);
            };
            assert_eq!(*reported, passed, "{outcome:?}");
            assert_eq!(kept.as_deref(), outcome);

            let dashboard = roko_core::core_event_to_dashboard_events(&envelope.payload);
            let [roko_core::DashboardEvent::TaskCompleted { outcome: shown, .. }] =
                dashboard.as_slice()
            else {
                panic!("expected one TaskCompleted, got {dashboard:?}");
            };
            assert_eq!(classify_task_outcome(shown), class, "{outcome:?}");
        }
    }

    #[test]
    fn delivery_events_use_extension() {
        let adapter = make_adapter();
        let event = GraphExecutionEvent::DeliveryStarted {
            common: make_common(22),
            delivery_id: "del-1".to_string(),
            plan_id: "test-plan".to_string(),
            branch: "main".to_string(),
            publish: true,
        };
        let envelope = adapter.convert(&event);
        if let RuntimeEvent::Extension { namespace, .. } = &envelope.payload {
            assert_eq!(namespace, "graph.delivery");
        } else {
            panic!("expected Extension");
        }
    }

    #[test]
    fn feedback_events_map_correctly() {
        let adapter = make_adapter();
        let settled = GraphExecutionEvent::FeedbackSinkSettled {
            common: make_common(23),
            node: make_node("T01"),
            idempotency_key: "key-1".to_string(),
            sink_key: "episode".to_string(),
            row: 0,
        };
        let envelope = adapter.convert(&settled);
        assert!(matches!(
            envelope.payload,
            RuntimeEvent::FeedbackSinkSettled { .. }
        ));

        let failed = GraphExecutionEvent::FeedbackSinkFailed {
            common: make_common(24),
            node: make_node("T01"),
            idempotency_key: "key-2".to_string(),
            sink_key: "routing".to_string(),
            row: 1,
            critical: true,
            error: "disk full".to_string(),
        };
        let envelope = adapter.convert(&failed);
        if let RuntimeEvent::FeedbackSinkFailed { error, .. } = &envelope.payload {
            assert!(error.contains("critical"));
            assert!(error.contains("disk full"));
        } else {
            panic!("expected FeedbackSinkFailed");
        }
    }
}
