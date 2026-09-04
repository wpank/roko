//! Canonical runtime-event-to-dashboard projector (#248).
//!
//! [`RuntimeEventDashboardProjector`] is the sole canonical mapping from
//! [`RuntimeEventEnvelope`] to [`DashboardEvent`]. All producers (runner,
//! graph, workflow engine, chat, ACP) emit envelopes; this projector
//! translates them into dashboard state mutations and publishes through
//! a [`StateHubSender`]-compatible callback.
//!
//! # Deduplication
//!
//! The projector deduplicates by `event_id` using a bounded 8,192-entry
//! LRU. Duplicate `event_id` values are silently dropped. A second
//! conflicting terminal for the same `run_id` or `(plan_id, task_id)` is
//! also rejected.
//!
//! # Thread safety
//!
//! The projector is `Send + Sync`. Internal state is protected by a
//! `parking_lot::Mutex`.
//!
//! [`RuntimeEventEnvelope`]: roko_core::runtime_event::RuntimeEventEnvelope
//! [`DashboardEvent`]: roko_core::dashboard_snapshot::DashboardEvent
//! [`StateHubSender`]: crate::state_hub::StateHubSender

use std::collections::{HashSet, VecDeque};

use roko_core::dashboard_snapshot::DashboardEvent;
use roko_core::runtime_event::{RuntimeEvent, RuntimeEventEnvelope};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Maximum event IDs retained in the dedup LRU.
const DEDUP_LRU_CAPACITY: usize = 8_192;

// ---------------------------------------------------------------------------
// Projector
// ---------------------------------------------------------------------------

/// Canonical `RuntimeEventEnvelope -> DashboardEvent` projector.
///
/// Thread-safe: all mutable state is behind a `parking_lot::Mutex`.
pub struct RuntimeEventDashboardProjector {
    inner: parking_lot::Mutex<Inner>,
}

/// Mutable interior of the projector.
struct Inner {
    /// Bounded LRU of recently seen event IDs for deduplication.
    seen_ids: VecDeque<String>,
    /// Fast lookup for the LRU.
    seen_set: HashSet<String>,
    /// Run IDs that have emitted a terminal event.
    terminal_runs: HashSet<String>,
    /// `(plan_id, task_id)` pairs that have emitted a terminal event.
    terminal_tasks: HashSet<(String, String)>,
}

impl Inner {
    fn new() -> Self {
        Self {
            seen_ids: VecDeque::with_capacity(DEDUP_LRU_CAPACITY),
            seen_set: HashSet::with_capacity(DEDUP_LRU_CAPACITY),
            terminal_runs: HashSet::new(),
            terminal_tasks: HashSet::new(),
        }
    }

    /// Returns `true` if this event_id has been seen before.
    fn is_duplicate(&mut self, event_id: &str) -> bool {
        if self.seen_set.contains(event_id) {
            return true;
        }
        // Evict oldest if at capacity.
        if self.seen_ids.len() >= DEDUP_LRU_CAPACITY
            && let Some(oldest) = self.seen_ids.pop_front()
        {
            self.seen_set.remove(&oldest);
        }
        self.seen_ids.push_back(event_id.to_string());
        self.seen_set.insert(event_id.to_string());
        false
    }

    /// Returns `true` if a terminal has already been recorded for this run.
    fn is_terminal_run(&self, run_id: &str) -> bool {
        self.terminal_runs.contains(run_id)
    }

    /// Mark a run as having received its terminal event.
    fn mark_terminal_run(&mut self, run_id: &str) {
        self.terminal_runs.insert(run_id.to_string());
    }

    /// Returns `true` if a terminal has already been recorded for this task.
    fn is_terminal_task(&self, plan_id: &str, task_id: &str) -> bool {
        self.terminal_tasks
            .contains(&(plan_id.to_string(), task_id.to_string()))
    }

    /// Mark a task as having received its terminal event.
    fn mark_terminal_task(&mut self, plan_id: &str, task_id: &str) {
        self.terminal_tasks
            .insert((plan_id.to_string(), task_id.to_string()));
    }
}

/// Result of projecting a single envelope.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectionResult {
    /// One or more `DashboardEvent` values were produced.
    Projected(Vec<DashboardEvent>),
    /// The event was a duplicate (already seen event_id).
    Duplicate,
    /// The event was a conflicting second terminal for its scope.
    ConflictingTerminal,
    /// The event did not map to any dashboard mutation.
    NoProjection,
}

impl RuntimeEventDashboardProjector {
    /// Create a new projector with empty dedup state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: parking_lot::Mutex::new(Inner::new()),
        }
    }

    /// Project a single envelope into zero or more `DashboardEvent` values.
    ///
    /// Returns `ProjectionResult::Duplicate` if the `event_id` has been seen.
    /// Returns `ProjectionResult::ConflictingTerminal` if a second terminal
    /// event arrives for the same run or task scope.
    pub fn project(&self, envelope: &RuntimeEventEnvelope) -> ProjectionResult {
        let mut inner = self.inner.lock();

        // 1. Dedup by event_id.
        if inner.is_duplicate(&envelope.event_id) {
            return ProjectionResult::Duplicate;
        }

        // 2. Map payload to dashboard events.
        let events = Self::map_to_dashboard(envelope);
        if events.is_empty() {
            return ProjectionResult::NoProjection;
        }

        // 3. Reject conflicting terminals.
        if Self::is_run_terminal(&envelope.payload) {
            if inner.is_terminal_run(&envelope.run_id) {
                return ProjectionResult::ConflictingTerminal;
            }
            inner.mark_terminal_run(&envelope.run_id);
        }
        if let Some((plan_id, task_id)) = Self::task_terminal_key(envelope) {
            if inner.is_terminal_task(&plan_id, &task_id) {
                return ProjectionResult::ConflictingTerminal;
            }
            inner.mark_terminal_task(&plan_id, &task_id);
        }

        ProjectionResult::Projected(events)
    }

    /// Reset all dedup and terminal state.
    pub fn reset(&self) {
        let mut inner = self.inner.lock();
        *inner = Inner::new();
    }

    // ── Private helpers ──────────────────────────────────────────────

    /// Whether this payload is a run-level terminal event.
    fn is_run_terminal(payload: &RuntimeEvent) -> bool {
        matches!(
            payload,
            RuntimeEvent::RunCompleted { .. } | RuntimeEvent::WorkflowCompleted { .. }
        )
    }

    /// If this envelope represents a task terminal, return (plan_id, task_id).
    fn task_terminal_key(envelope: &RuntimeEventEnvelope) -> Option<(String, String)> {
        match &envelope.payload {
            RuntimeEvent::TaskCompleted {
                plan_id, task_id, ..
            } => Some((plan_id.clone(), task_id.clone())),
            RuntimeEvent::TaskFailed {
                plan_id, task_id, ..
            } => Some((plan_id.clone(), task_id.clone())),
            RuntimeEvent::TaskSkipped { task_id, .. } => {
                let plan_id = envelope.plan_id.clone().unwrap_or_default();
                Some((plan_id, task_id.clone()))
            }
            _ => None,
        }
    }

    /// Convert a runtime event envelope into dashboard events.
    ///
    /// Returns an empty vec for events with no dashboard projection.
    #[allow(clippy::too_many_lines)]
    fn map_to_dashboard(envelope: &RuntimeEventEnvelope) -> Vec<DashboardEvent> {
        let plan_id_opt = envelope.plan_id.as_deref().unwrap_or("");
        let task_id_opt = envelope.task_id.as_deref().unwrap_or("");
        let ts_ms = envelope.ts.timestamp_millis() as u64;

        match &envelope.payload {
            // ── Run lifecycle ────────────────────────────────────────
            RuntimeEvent::RunStarted { .. } => {
                vec![DashboardEvent::PlanStarted {
                    plan_id: plan_id_opt.to_string(),
                    tasks_total: 0,
                }]
            }
            RuntimeEvent::RunCompleted {
                success,
                duration_ms,
                ..
            } => {
                let mut events = vec![];
                if !plan_id_opt.is_empty() {
                    events.push(DashboardEvent::PlanCompleted {
                        plan_id: plan_id_opt.to_string(),
                        success: *success,
                    });
                }
                events.push(DashboardEvent::RunCompleted {
                    outcome: if *success {
                        "success".to_string()
                    } else {
                        "failed".to_string()
                    },
                    duration_ms: *duration_ms,
                    cleanup_degraded: false,
                    surviving_agent_ids: vec![],
                    surviving_agent_pids: vec![],
                });
                events
            }

            // ── Workflow lifecycle ───────────────────────────────────
            RuntimeEvent::WorkflowCompleted { outcome, .. } => {
                let success = matches!(
                    outcome,
                    roko_core::runtime_event::WorkflowOutcome::Success { .. }
                );
                vec![DashboardEvent::RunCompleted {
                    outcome: if success {
                        "success".to_string()
                    } else {
                        "failed".to_string()
                    },
                    duration_ms: 0,
                    cleanup_degraded: false,
                    surviving_agent_ids: vec![],
                    surviving_agent_pids: vec![],
                }]
            }

            // ── Wave lifecycle ──────────────────────────────────────
            RuntimeEvent::PipelinePhase { phase, status, .. } => {
                vec![DashboardEvent::PhaseTransition {
                    plan_id: plan_id_opt.to_string(),
                    from: String::new(),
                    to: format!("{phase}:{status}"),
                }]
            }

            // ── Task lifecycle ──────────────────────────────────────
            RuntimeEvent::TaskStarted {
                plan_id,
                task_id,
                task_title,
                role: _,
                ..
            } => vec![DashboardEvent::TaskStarted {
                plan_id: plan_id.clone(),
                task_id: task_id.clone(),
                title: task_title.clone(),
                phase: "executing".to_string(),
            }],
            RuntimeEvent::TaskCompleted {
                plan_id,
                task_id,
                passed,
                ..
            } => vec![DashboardEvent::TaskCompleted {
                plan_id: plan_id.clone(),
                task_id: task_id.clone(),
                outcome: if *passed {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                },
            }],
            RuntimeEvent::TaskFailed {
                plan_id,
                task_id,
                error,
                ..
            } => {
                vec![
                    DashboardEvent::TaskCompleted {
                        plan_id: plan_id.clone(),
                        task_id: task_id.clone(),
                        outcome: "failed".to_string(),
                    },
                    DashboardEvent::Error {
                        message: format!("{plan_id}/{task_id}: {error}"),
                    },
                ]
            }
            RuntimeEvent::TaskSkipped { task_id, reason: _ } => {
                vec![DashboardEvent::TaskCompleted {
                    plan_id: plan_id_opt.to_string(),
                    task_id: task_id.clone(),
                    outcome: "skipped".to_string(),
                }]
            }
            RuntimeEvent::TaskRetrying {
                task_id, reason, ..
            } => vec![
                DashboardEvent::TaskPhaseChanged {
                    plan_id: plan_id_opt.to_string(),
                    task_id: task_id.clone(),
                    old_phase: "executing".to_string(),
                    new_phase: "retrying".to_string(),
                },
                DashboardEvent::EventLogEntry {
                    timestamp_ms: ts_ms,
                    event_type: "task_retrying".to_string(),
                    plan_id: plan_id_opt.to_string(),
                    task_id: task_id.clone(),
                    message: reason.clone(),
                },
            ],

            // ── Agent lifecycle ─────────────────────────────────────
            RuntimeEvent::AgentSpawned {
                agent_id,
                role,
                model,
                ..
            } => vec![DashboardEvent::AgentSpawned {
                agent_id: agent_id.clone(),
                plan_id: plan_id_opt.to_string(),
                task_id: task_id_opt.to_string(),
                attempt: 0,
                role: role.clone(),
                model: model.clone(),
                provider: String::new(),
            }],
            RuntimeEvent::AgentOutput {
                agent_id, chunk, ..
            } => vec![
                DashboardEvent::AgentOutput {
                    agent_id: agent_id.clone(),
                    plan_id: plan_id_opt.to_string(),
                    task_id: task_id_opt.to_string(),
                    attempt: 0,
                    content: chunk.clone(),
                },
                DashboardEvent::TaskOutputAppended {
                    task_id: task_id_opt.to_string(),
                    lines: vec![chunk.clone()],
                },
            ],
            RuntimeEvent::AgentProgress {
                agent_id: _,
                message,
                ..
            } => vec![
                DashboardEvent::TaskPhaseChanged {
                    plan_id: plan_id_opt.to_string(),
                    task_id: task_id_opt.to_string(),
                    old_phase: String::new(),
                    new_phase: message.clone(),
                },
                DashboardEvent::EventLogEntry {
                    timestamp_ms: ts_ms,
                    event_type: "agent_progress".to_string(),
                    plan_id: plan_id_opt.to_string(),
                    task_id: task_id_opt.to_string(),
                    message: message.clone(),
                },
            ],

            // ── Tool calls ──────────────────────────────────────────
            RuntimeEvent::ToolCallStarted { agent_id, tool, .. } => {
                vec![DashboardEvent::EventLogEntry {
                    timestamp_ms: ts_ms,
                    event_type: "tool_started".to_string(),
                    plan_id: plan_id_opt.to_string(),
                    task_id: task_id_opt.to_string(),
                    message: format!("{agent_id}: {tool}"),
                }]
            }
            RuntimeEvent::ToolCallCompleted {
                agent_id,
                tool,
                success,
                duration_ms,
                ..
            } => vec![DashboardEvent::EventLogEntry {
                timestamp_ms: ts_ms,
                event_type: "tool_completed".to_string(),
                plan_id: plan_id_opt.to_string(),
                task_id: task_id_opt.to_string(),
                message: format!(
                    "{agent_id}: {tool} {} ({duration_ms}ms)",
                    if *success { "ok" } else { "FAIL" }
                ),
            }],

            // ── Usage / budget ──────────────────────────────────────
            RuntimeEvent::UsageRecorded {
                input_tokens,
                output_tokens,
                cost_usd,
                ..
            } => {
                vec![
                    DashboardEvent::EfficiencyEvent {
                        plan_id: plan_id_opt.to_string(),
                        task_id: task_id_opt.to_string(),
                        metric: "input_tokens".to_string(),
                        value: *input_tokens as f64,
                    },
                    DashboardEvent::EfficiencyEvent {
                        plan_id: plan_id_opt.to_string(),
                        task_id: task_id_opt.to_string(),
                        metric: "output_tokens".to_string(),
                        value: *output_tokens as f64,
                    },
                    DashboardEvent::EfficiencyEvent {
                        plan_id: plan_id_opt.to_string(),
                        task_id: task_id_opt.to_string(),
                        metric: "cost_usd".to_string(),
                        value: *cost_usd,
                    },
                ]
            }
            RuntimeEvent::BudgetUpdated {
                spent_usd: _,
                remaining_usd,
                ..
            } => vec![DashboardEvent::EfficiencyEvent {
                plan_id: plan_id_opt.to_string(),
                task_id: task_id_opt.to_string(),
                metric: "budget_remaining_usd".to_string(),
                value: *remaining_usd,
            }],

            // ── Gate rungs ──────────────────────────────────────────
            RuntimeEvent::GateRungStarted {
                gate_name, rung: _, ..
            } => vec![DashboardEvent::GateRungStarted {
                plan_id: plan_id_opt.to_string(),
                task_id: task_id_opt.to_string(),
                rung_name: gate_name.clone(),
            }],
            RuntimeEvent::GateRungOutput {
                gate_name, chunk, ..
            } => vec![DashboardEvent::GateOutputLine {
                plan_id: plan_id_opt.to_string(),
                task_id: task_id_opt.to_string(),
                gate: gate_name.clone(),
                line: chunk.clone(),
            }],
            RuntimeEvent::GateRungCompleted {
                gate_name, passed, ..
            } => vec![DashboardEvent::GateResult {
                plan_id: plan_id_opt.to_string(),
                task_id: task_id_opt.to_string(),
                gate: gate_name.clone(),
                passed: *passed,
                output_text: None,
            }],

            // ── Sequence gap ────────────────────────────────────────
            RuntimeEvent::SequenceGap { reason, .. } => {
                vec![DashboardEvent::EventLogEntry {
                    timestamp_ms: ts_ms,
                    event_type: "sequence_gap".to_string(),
                    plan_id: plan_id_opt.to_string(),
                    task_id: task_id_opt.to_string(),
                    message: reason.clone(),
                }]
            }

            // ── Wave events ─────────────────────────────────────────
            RuntimeEvent::WaveStarted {
                wave_index,
                task_count,
            } => vec![DashboardEvent::PhaseTransition {
                plan_id: plan_id_opt.to_string(),
                from: String::new(),
                to: format!("wave-{wave_index} ({task_count} tasks)"),
            }],
            RuntimeEvent::WaveCompleted {
                wave_index,
                succeeded,
                failed,
                duration_ms,
            } => vec![DashboardEvent::PhaseTransition {
                plan_id: plan_id_opt.to_string(),
                from: format!("wave-{wave_index}"),
                to: format!(
                    "wave-{wave_index} complete ({succeeded}/{} in {duration_ms}ms)",
                    succeeded + failed
                ),
            }],

            // ── Error event ─────────────────────────────────────────
            RuntimeEvent::AgentFailed {
                agent_id, error, ..
            } => vec![DashboardEvent::Error {
                message: format!("agent {agent_id}: {error}"),
            }],

            // ── All other events: log entry only ────────────────────
            _ => vec![DashboardEvent::EventLogEntry {
                timestamp_ms: ts_ms,
                event_type: envelope.payload.kind().to_string(),
                plan_id: plan_id_opt.to_string(),
                task_id: task_id_opt.to_string(),
                message: format!("{}", envelope.payload),
            }],
        }
    }
}

impl Default for RuntimeEventDashboardProjector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use roko_core::runtime_event::{
        RuntimeEvent, RuntimeEventDelivery, RuntimeEventEnvelope, RuntimeEventMode,
    };

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

    #[test]
    fn run_started_projects_to_plan_started() {
        let projector = RuntimeEventDashboardProjector::new();
        let envelope = test_envelope(
            "e1",
            RuntimeEvent::RunStarted {
                run_id: "run-1".to_string(),
                prompt: String::new(),
                complexity: "graph".to_string(),
            },
        );
        let result = projector.project(&envelope);
        match result {
            ProjectionResult::Projected(events) => {
                assert_eq!(events.len(), 1);
                assert!(matches!(events[0], DashboardEvent::PlanStarted { .. }));
            }
            other => panic!("expected Projected, got {other:?}"),
        }
    }

    #[test]
    fn run_completed_projects_to_plan_and_run_completed() {
        let projector = RuntimeEventDashboardProjector::new();
        let envelope = test_envelope(
            "e2",
            RuntimeEvent::RunCompleted {
                run_id: "run-1".to_string(),
                success: true,
                cost_usd: 0.5,
                duration_ms: 5000,
            },
        );
        let result = projector.project(&envelope);
        match result {
            ProjectionResult::Projected(events) => {
                assert_eq!(events.len(), 2);
                assert!(matches!(events[0], DashboardEvent::PlanCompleted { .. }));
                assert!(matches!(events[1], DashboardEvent::RunCompleted { .. }));
            }
            other => panic!("expected Projected, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_event_id_rejected() {
        let projector = RuntimeEventDashboardProjector::new();
        let envelope = test_envelope(
            "dup-1",
            RuntimeEvent::RunStarted {
                run_id: "run-1".to_string(),
                prompt: String::new(),
                complexity: "graph".to_string(),
            },
        );

        assert!(matches!(
            projector.project(&envelope),
            ProjectionResult::Projected(_)
        ));
        assert!(matches!(
            projector.project(&envelope),
            ProjectionResult::Duplicate
        ));
    }

    #[test]
    fn conflicting_run_terminal_rejected() {
        let projector = RuntimeEventDashboardProjector::new();
        let first = test_envelope(
            "term-1",
            RuntimeEvent::RunCompleted {
                run_id: "run-1".to_string(),
                success: true,
                cost_usd: 0.0,
                duration_ms: 1000,
            },
        );
        let second = test_envelope(
            "term-2",
            RuntimeEvent::RunCompleted {
                run_id: "run-1".to_string(),
                success: false,
                cost_usd: 0.0,
                duration_ms: 2000,
            },
        );

        assert!(matches!(
            projector.project(&first),
            ProjectionResult::Projected(_)
        ));
        assert!(matches!(
            projector.project(&second),
            ProjectionResult::ConflictingTerminal
        ));
    }

    #[test]
    fn conflicting_task_terminal_rejected() {
        let projector = RuntimeEventDashboardProjector::new();
        let first = test_envelope(
            "task-term-1",
            RuntimeEvent::TaskCompleted {
                run_id: "run-1".to_string(),
                plan_id: "plan-1".to_string(),
                task_id: "task-1".to_string(),
                passed: true,
                duration_ms: 500,
            },
        );
        let second = test_envelope(
            "task-term-2",
            RuntimeEvent::TaskCompleted {
                run_id: "run-1".to_string(),
                plan_id: "plan-1".to_string(),
                task_id: "task-1".to_string(),
                passed: false,
                duration_ms: 600,
            },
        );

        assert!(matches!(
            projector.project(&first),
            ProjectionResult::Projected(_)
        ));
        assert!(matches!(
            projector.project(&second),
            ProjectionResult::ConflictingTerminal
        ));
    }

    #[test]
    fn different_tasks_can_both_terminate() {
        let projector = RuntimeEventDashboardProjector::new();
        let first = test_envelope(
            "diff-task-1",
            RuntimeEvent::TaskCompleted {
                run_id: "run-1".to_string(),
                plan_id: "plan-1".to_string(),
                task_id: "task-1".to_string(),
                passed: true,
                duration_ms: 500,
            },
        );
        let mut second = test_envelope(
            "diff-task-2",
            RuntimeEvent::TaskCompleted {
                run_id: "run-1".to_string(),
                plan_id: "plan-1".to_string(),
                task_id: "task-2".to_string(),
                passed: true,
                duration_ms: 500,
            },
        );
        second.task_id = Some("task-2".to_string());

        assert!(matches!(
            projector.project(&first),
            ProjectionResult::Projected(_)
        ));
        assert!(matches!(
            projector.project(&second),
            ProjectionResult::Projected(_)
        ));
    }

    #[test]
    fn task_started_projects_correctly() {
        let projector = RuntimeEventDashboardProjector::new();
        let envelope = test_envelope(
            "ts-1",
            RuntimeEvent::TaskStarted {
                run_id: "run-1".to_string(),
                plan_id: "plan-1".to_string(),
                task_id: "compile".to_string(),
                task_title: "Compile".to_string(),
                role: "implementer".to_string(),
            },
        );
        let result = projector.project(&envelope);
        match result {
            ProjectionResult::Projected(events) => {
                assert_eq!(events.len(), 1);
                if let DashboardEvent::TaskStarted { task_id, title, .. } = &events[0] {
                    assert_eq!(task_id, "compile");
                    assert_eq!(title, "Compile");
                } else {
                    panic!("expected TaskStarted");
                }
            }
            other => panic!("expected Projected, got {other:?}"),
        }
    }

    #[test]
    fn agent_output_produces_two_events() {
        let projector = RuntimeEventDashboardProjector::new();
        let envelope = test_envelope(
            "ao-1",
            RuntimeEvent::AgentOutput {
                run_id: "run-1".to_string(),
                agent_id: "agent-1".to_string(),
                chunk: "hello".to_string(),
            },
        );
        let result = projector.project(&envelope);
        match result {
            ProjectionResult::Projected(events) => {
                assert_eq!(events.len(), 2);
                assert!(matches!(events[0], DashboardEvent::AgentOutput { .. }));
                assert!(matches!(
                    events[1],
                    DashboardEvent::TaskOutputAppended { .. }
                ));
            }
            other => panic!("expected Projected, got {other:?}"),
        }
    }

    #[test]
    fn gate_rung_events_project() {
        let projector = RuntimeEventDashboardProjector::new();

        let start = test_envelope(
            "gr-1",
            RuntimeEvent::GateRungStarted {
                gate_name: "compile".to_string(),
                rung: 0,
            },
        );
        assert!(matches!(
            projector.project(&start),
            ProjectionResult::Projected(_)
        ));

        let output = test_envelope(
            "gr-2",
            RuntimeEvent::GateRungOutput {
                gate_name: "compile".to_string(),
                rung: 0,
                chunk: "compiling...".to_string(),
            },
        );
        assert!(matches!(
            projector.project(&output),
            ProjectionResult::Projected(_)
        ));

        let complete = test_envelope(
            "gr-3",
            RuntimeEvent::GateRungCompleted {
                gate_name: "compile".to_string(),
                rung: 0,
                passed: true,
                duration_ms: 3000,
            },
        );
        let result = projector.project(&complete);
        match result {
            ProjectionResult::Projected(events) => {
                if let DashboardEvent::GateResult { passed, .. } = &events[0] {
                    assert!(*passed);
                } else {
                    panic!("expected GateResult");
                }
            }
            other => panic!("expected Projected, got {other:?}"),
        }
    }

    #[test]
    fn reset_clears_dedup_state() {
        let projector = RuntimeEventDashboardProjector::new();
        let envelope = test_envelope(
            "reset-1",
            RuntimeEvent::RunStarted {
                run_id: "run-1".to_string(),
                prompt: String::new(),
                complexity: "graph".to_string(),
            },
        );

        assert!(matches!(
            projector.project(&envelope),
            ProjectionResult::Projected(_)
        ));
        assert!(matches!(
            projector.project(&envelope),
            ProjectionResult::Duplicate
        ));

        projector.reset();
        assert!(matches!(
            projector.project(&envelope),
            ProjectionResult::Projected(_)
        ));
    }

    #[test]
    fn lru_eviction_at_capacity() {
        let projector = RuntimeEventDashboardProjector::new();

        // Fill the LRU to capacity.
        for i in 0..DEDUP_LRU_CAPACITY {
            let envelope = test_envelope(
                &format!("lru-{i}"),
                RuntimeEvent::RunStarted {
                    run_id: format!("run-{i}"),
                    prompt: String::new(),
                    complexity: "test".to_string(),
                },
            );
            assert!(matches!(
                projector.project(&envelope),
                ProjectionResult::Projected(_)
            ));
        }

        // The oldest should still be in the LRU.
        let oldest = test_envelope(
            "lru-0",
            RuntimeEvent::RunStarted {
                run_id: "run-0".to_string(),
                prompt: String::new(),
                complexity: "test".to_string(),
            },
        );
        assert!(matches!(
            projector.project(&oldest),
            ProjectionResult::Duplicate
        ));

        // Add one more to trigger eviction.
        let overflow = test_envelope(
            "lru-overflow",
            RuntimeEvent::RunStarted {
                run_id: "run-overflow".to_string(),
                prompt: String::new(),
                complexity: "test".to_string(),
            },
        );
        assert!(matches!(
            projector.project(&overflow),
            ProjectionResult::Projected(_)
        ));

        // Now "lru-0" should have been evicted and no longer be a duplicate.
        let oldest_again = test_envelope(
            "lru-0",
            RuntimeEvent::RunStarted {
                run_id: "run-0-again".to_string(),
                prompt: String::new(),
                complexity: "test".to_string(),
            },
        );
        assert!(matches!(
            projector.project(&oldest_again),
            ProjectionResult::Projected(_)
        ));
    }

    #[test]
    fn usage_recorded_produces_efficiency_events() {
        let projector = RuntimeEventDashboardProjector::new();
        let envelope = test_envelope(
            "usage-1",
            RuntimeEvent::UsageRecorded {
                input_tokens: 1000,
                output_tokens: 500,
                cost_usd: 0.50,
                model: "test-model".to_string(),
            },
        );
        let result = projector.project(&envelope);
        match result {
            ProjectionResult::Projected(events) => {
                assert_eq!(events.len(), 3);
                // Should produce input_tokens, output_tokens, cost_usd efficiency events.
                for event in &events {
                    assert!(matches!(event, DashboardEvent::EfficiencyEvent { .. }));
                }
            }
            other => panic!("expected Projected, got {other:?}"),
        }
    }

    #[test]
    fn sequence_gap_produces_log_entry() {
        let projector = RuntimeEventDashboardProjector::new();
        let envelope = test_envelope(
            "gap-1",
            RuntimeEvent::SequenceGap {
                first_missing_seq: 5,
                last_missing_seq: 7,
                reason: "3 events dropped".to_string(),
            },
        );
        let result = projector.project(&envelope);
        match result {
            ProjectionResult::Projected(events) => {
                assert_eq!(events.len(), 1);
                if let DashboardEvent::EventLogEntry {
                    event_type,
                    message,
                    ..
                } = &events[0]
                {
                    assert_eq!(event_type, "sequence_gap");
                    assert!(message.contains("dropped"));
                } else {
                    panic!("expected EventLogEntry");
                }
            }
            other => panic!("expected Projected, got {other:?}"),
        }
    }
}
