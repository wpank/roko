//! Canonical shared event taxonomy.
//!
//! This module defines [`CoreEvent`], a focused view of the runtime event
//! vocabulary that is **intended to flow across subsystem boundaries**.
//!
//! # Motivation
//!
//! `roko-core` already contains [`RuntimeEvent`] — the exhaustive, versioned
//! wire schema that every runtime producer (runner, graph, ACP, chat) emits
//! and every projector (TUI bridge, SSE adapter, StateHub, JSONL logger)
//! consumes.  However, that schema is large (50+ variants) and its internal
//! classification is embedded only in `delivery()` and `kind()` method
//! bodies.
//!
//! `CoreEvent` makes the taxonomy explicit:
//!
//! - **Lifecycle** — plan/run start/stop; task start/complete/fail/retry/skip.
//! - **Agent** — agent spawn/complete/fail and incremental output.
//! - **Gate** — rung start, output, and verdict (pass/fail).
//! - **Inference** — per-request inference tracking (start, TTFT, complete, fail).
//! - **Knowledge** — knowledge ingestion and consumption.
//! - **Budget** — budget updates and exhaustion.
//! - **Control** — pause/resume/cancel and approval flow.
//! - **Sequencing** — sequence-gap markers for best-effort loss detection.
//!
//! # Relationship to `RuntimeEvent`
//!
//! `CoreEvent` is a **type alias** for `RuntimeEvent`.  There is no runtime
//! cost and no second encoding.  Code that works with `RuntimeEvent` today
//! continues to compile unchanged.  New cross-subsystem wiring should use
//! `CoreEvent` at call sites where the intent is "emit an event that multiple
//! subsystems can consume," and use `RuntimeEvent` where the full schema is
//! needed (e.g. exhaustive `match` in projectors).
//!
//! ```rust,ignore
//! use roko_core::core_event::CoreEvent;
//! use roko_core::RuntimeEvent;
//!
//! // Both types refer to the same enum.
//! let ev: CoreEvent = RuntimeEvent::TaskStarted {
//!     run_id: "r1".into(),
//!     plan_id: "p1".into(),
//!     task_id: "t1".into(),
//!     task_title: "Build crate".into(),
//!     role: "implementer".into(),
//! };
//! ```
//!
//! # Taxonomy
//!
//! The table below lists the [`RuntimeEvent`] variants that belong to each
//! category and whether they cross subsystem boundaries (i.e. they are
//! consumed by more than one projector).
//!
//! ## Lifecycle
//!
//! | Variant                | Boundary |
//! |------------------------|----------|
//! | `WorkflowStarted`      | yes      |
//! | `WorkflowCompleted`    | yes      |
//! | `RunStarted`           | yes      |
//! | `RunCompleted`         | yes      |
//! | `TaskStarted`          | yes      |
//! | `TaskCompleted`        | yes      |
//! | `TaskFailed`           | yes      |
//! | `TaskRetrying`         | yes      |
//! | `TaskSkipped`          | yes      |
//! | `WaveStarted`          | yes      |
//! | `WaveCompleted`        | yes      |
//! | `PipelinePhase`        | yes      |
//! | `PhaseTransition`      | yes      |
//!
//! ## Agent
//!
//! | Variant           | Boundary |
//! |-------------------|----------|
//! | `AgentSpawned`    | yes      |
//! | `AgentCompleted`  | yes      |
//! | `AgentFailed`     | yes      |
//! | `AgentOutput`     | best-effort (TUI + SSE) |
//! | `AgentProgress`   | best-effort (TUI only)  |
//! | `AgentTrace`      | yes      |
//!
//! ## Gate
//!
//! | Variant            | Boundary          |
//! |--------------------|-------------------|
//! | `GateStarted`      | yes               |
//! | `GatePassed`       | yes               |
//! | `GateFailed`       | yes               |
//! | `GateRungStarted`  | yes               |
//! | `GateRungOutput`   | best-effort (TUI) |
//! | `GateRungCompleted`| yes               |
//!
//! ## Inference
//!
//! | Variant               | Boundary                |
//! |-----------------------|-------------------------|
//! | `InferenceStarted`    | yes                     |
//! | `InferenceCompleted`  | yes                     |
//! | `InferenceFailed`     | yes                     |
//! | `InferenceFirstToken` | best-effort (dashboard) |
//! | `ToolCallStarted`     | yes                     |
//! | `ToolCallCompleted`   | yes                     |
//! | `UsageRecorded`       | yes                     |
//!
//! ## Knowledge
//!
//! | Variant             | Boundary |
//! |---------------------|----------|
//! | `KnowledgeIngested` | yes      |
//! | `KnowledgeConsumed` | yes      |
//!
//! ## Budget
//!
//! | Variant        | Boundary |
//! |----------------|----------|
//! | `BudgetUpdated`| yes      |
//!
//! ## Control
//!
//! | Variant              | Boundary |
//! |----------------------|----------|
//! | `ApprovalRequested`  | yes      |
//! | `ApprovalResolved`   | yes      |
//! | `ControlApplied`     | yes      |
//!
//! ## Workspace / Merge / Publish
//!
//! | Variant              | Boundary |
//! |----------------------|----------|
//! | `WorkspaceAcquired`  | yes      |
//! | `WorkspaceReleased`  | yes      |
//! | `MergeQueued`        | yes      |
//! | `MergeCompleted`     | yes      |
//! | `PublishCompleted`   | yes      |
//!
//! ## Sequencing (reliability)
//!
//! | Variant          | Boundary |
//! |------------------|----------|
//! | `SequenceGap`    | yes — projectors must handle this |
//! | `StateCheckpointed` | yes   |
//! | `FeedbackRecorded`  | yes   |
//!
//! ## Extension
//!
//! | Variant       | Notes                                             |
//! |---------------|---------------------------------------------------|
//! | `Extension`   | Forward-compatible; namespace identifies producer |
//! | `Fallthrough` | Observability for unhandled routing paths         |
//!
//! # Producer / consumer map
//!
//! The following subsystems are the primary producers and consumers:
//!
//! | Subsystem          | Role                                 |
//! |--------------------|--------------------------------------|
//! | Graph engine       | producer (task/wave/gate/agent)      |
//! | Runner-v2 (legacy) | producer (task/gate/agent), emits `--engine legacy` only |
//! | ACP bridge         | producer (agent/inference)           |
//! | `roko-cli` chat    | producer (agent chunks)              |
//! | StateHub           | consumer + fan-out                   |
//! | TUI bridge         | consumer                             |
//! | SSE adapter        | consumer (HTTP push)                 |
//! | JSONL logger       | consumer (`.roko/state/`)            |
//! | Telemetry lenses   | consumer (sampling)                  |

use crate::dashboard_snapshot::DashboardEvent;
use crate::runtime_event::RuntimeEvent;

/// A type alias for [`RuntimeEvent`] that names the cross-subsystem event
/// vocabulary explicitly.
///
/// See the [module-level documentation](self) for the full taxonomy.
pub type CoreEvent = RuntimeEvent;

// Re-export the envelope so callers can use one import.
pub use crate::runtime_event::{
    RuntimeEventDelivery, RuntimeEventEnvelope, RuntimeEventMode, WorkflowOutcome,
};

/// Convert a [`CoreEvent`] (= [`RuntimeEvent`]) into zero or more
/// [`DashboardEvent`]s.
///
/// Not every `RuntimeEvent` variant maps to a `DashboardEvent`.  Variants
/// that have no direct dashboard representation return an empty `Vec`.
/// Variants that require two dashboard events (e.g. `WorkflowStarted` →
/// `PlanStarted` + `TaskStarted`) return multiple elements.
///
/// This is the canonical bridge between the `RuntimeEvent` producer side and
/// the `DashboardEvent` consumer side (TUI, SSE, StateHub).  The
/// `DashboardEventBridge` in `roko-serve` uses the same mapping for the
/// `WorkflowEngine` path; callers that have a `StateHubSender` should prefer
/// [`StateHubSender::publish_core_event`] which calls this internally.
///
/// # Example
///
/// ```rust,ignore
/// use roko_core::core_event::{CoreEvent, core_event_to_dashboard_events};
///
/// let ev = CoreEvent::TaskStarted {
///     run_id: "r1".into(),
///     plan_id: "p1".into(),
///     task_id: "t1".into(),
///     task_title: "Build crate".into(),
///     role: "implementer".into(),
/// };
/// let dash_events = core_event_to_dashboard_events(&ev);
/// assert_eq!(dash_events.len(), 1);
/// ```
pub fn core_event_to_dashboard_events(event: &CoreEvent) -> Vec<DashboardEvent> {
    match event {
        // ------------------------------------------------------------------
        // Workflow lifecycle → plan/task framing
        // ------------------------------------------------------------------
        RuntimeEvent::WorkflowStarted { run_id, prompt, .. } => {
            let plan_id = workflow_plan_id(run_id);
            let task_id = workflow_task_id(run_id);
            vec![
                DashboardEvent::PlanStarted {
                    plan_id: plan_id.clone(),
                    tasks_total: 0,
                },
                DashboardEvent::TaskStarted {
                    plan_id,
                    task_id,
                    title: prompt.clone(),
                    phase: "workflow".into(),
                },
            ]
        }
        RuntimeEvent::WorkflowCompleted { run_id, outcome } => {
            let plan_id = workflow_plan_id(run_id);
            let success = matches!(outcome, WorkflowOutcome::Success { .. });
            vec![
                DashboardEvent::TaskCompleted {
                    plan_id: plan_id.clone(),
                    task_id: workflow_task_id(run_id),
                    outcome: workflow_outcome_label(outcome),
                },
                DashboardEvent::PlanCompleted { plan_id, success },
            ]
        }
        RuntimeEvent::PhaseTransition { run_id, from, to } => {
            vec![DashboardEvent::PhaseTransition {
                plan_id: workflow_plan_id(run_id),
                from: from.clone(),
                to: to.clone(),
            }]
        }

        // ------------------------------------------------------------------
        // Task lifecycle
        // ------------------------------------------------------------------
        RuntimeEvent::TaskStarted {
            plan_id,
            task_id,
            task_title,
            role,
            ..
        } => vec![DashboardEvent::TaskStarted {
            plan_id: plan_id.clone(),
            task_id: task_id.clone(),
            title: task_title.clone(),
            phase: role.clone(),
        }],
        RuntimeEvent::TaskCompleted {
            plan_id,
            task_id,
            passed,
            outcome,
            ..
        } => vec![DashboardEvent::TaskCompleted {
            plan_id: plan_id.clone(),
            task_id: task_id.clone(),
            outcome: outcome
                .clone()
                .unwrap_or_else(|| if *passed { "passed" } else { "failed" }.into()),
        }],
        RuntimeEvent::TaskFailed {
            plan_id,
            task_id,
            error,
            ..
        } => vec![DashboardEvent::TaskCompleted {
            plan_id: plan_id.clone(),
            task_id: task_id.clone(),
            outcome: format!("failed: {error}"),
        }],

        // ------------------------------------------------------------------
        // Agent events
        // ------------------------------------------------------------------
        RuntimeEvent::AgentSpawned {
            run_id: _,
            agent_id,
            role,
            model,
        } => vec![DashboardEvent::AgentSpawned {
            agent_id: agent_id.clone(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
            role: role.clone(),
            model: model.clone(),
            provider: String::new(),
        }],
        RuntimeEvent::AgentOutput {
            agent_id, chunk, ..
        } => vec![DashboardEvent::AgentOutput {
            agent_id: agent_id.clone(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
            content: chunk.clone(),
        }],
        RuntimeEvent::AgentCompleted { agent_id, .. } => vec![DashboardEvent::AgentCompleted {
            agent_id: agent_id.clone(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
        }],
        RuntimeEvent::AgentFailed {
            agent_id, error, ..
        } => vec![DashboardEvent::Error {
            message: format!("agent {agent_id} failed: {error}"),
        }],

        // ------------------------------------------------------------------
        // Gate events
        // ------------------------------------------------------------------
        RuntimeEvent::GatePassed {
            run_id,
            gate_name,
            duration_ms,
        } => vec![DashboardEvent::GateResult {
            plan_id: workflow_plan_id(run_id),
            task_id: workflow_task_id(run_id),
            gate: gate_name.clone(),
            passed: true,
            output_text: Some(format!("passed in {duration_ms}ms")),
        }],
        RuntimeEvent::GateFailed {
            run_id,
            gate_name,
            output,
            duration_ms,
        } => vec![DashboardEvent::GateResult {
            plan_id: workflow_plan_id(run_id),
            task_id: workflow_task_id(run_id),
            gate: gate_name.clone(),
            passed: false,
            output_text: Some(format!("{output} ({duration_ms}ms)")),
        }],

        // ------------------------------------------------------------------
        // Budget events
        // ------------------------------------------------------------------
        RuntimeEvent::BudgetUpdated {
            budget_id,
            spent_usd,
            limit_usd,
            remaining_usd,
        } => vec![DashboardEvent::EventLogEntry {
            timestamp_ms: now_millis(),
            event_type: "budget_updated".into(),
            plan_id: budget_id.clone(),
            task_id: String::new(),
            message: format!(
                "budget {budget_id}: ${spent_usd:.4} / ${limit_usd:.4} (${remaining_usd:.4} remaining)"
            ),
        }],

        // ------------------------------------------------------------------
        // Variants without a direct DashboardEvent representation
        // ------------------------------------------------------------------
        RuntimeEvent::GateStarted { .. }
        | RuntimeEvent::GateRungStarted { .. }
        | RuntimeEvent::GateRungOutput { .. }
        | RuntimeEvent::GateRungCompleted { .. }
        | RuntimeEvent::FeedbackRecorded { .. }
        | RuntimeEvent::StateCheckpointed { .. }
        | RuntimeEvent::InferenceStarted { .. }
        | RuntimeEvent::InferenceCompleted { .. }
        | RuntimeEvent::InferenceFailed { .. }
        | RuntimeEvent::InferenceFirstToken { .. }
        | RuntimeEvent::ToolCallStarted { .. }
        | RuntimeEvent::ToolCallCompleted { .. }
        | RuntimeEvent::AgentTrace { .. }
        | RuntimeEvent::AgentProgress { .. }
        | RuntimeEvent::RunStarted { .. }
        | RuntimeEvent::RunCompleted { .. }
        | RuntimeEvent::KnowledgeIngested { .. }
        | RuntimeEvent::KnowledgeConsumed { .. }
        | RuntimeEvent::TaskRetrying { .. }
        | RuntimeEvent::TaskSkipped { .. }
        | RuntimeEvent::WaveStarted { .. }
        | RuntimeEvent::WaveCompleted { .. }
        | RuntimeEvent::PipelinePhase { .. }
        | RuntimeEvent::ApprovalRequested { .. }
        | RuntimeEvent::ApprovalResolved { .. }
        | RuntimeEvent::ControlApplied { .. }
        | RuntimeEvent::WorkspaceAcquired { .. }
        | RuntimeEvent::WorkspaceReleased { .. }
        | RuntimeEvent::MergeQueued { .. }
        | RuntimeEvent::MergeCompleted { .. }
        | RuntimeEvent::PublishCompleted { .. }
        | RuntimeEvent::SequenceGap { .. }
        | RuntimeEvent::UsageRecorded { .. }
        | RuntimeEvent::FeedbackSinkSettled { .. }
        | RuntimeEvent::FeedbackSinkFailed { .. }
        | RuntimeEvent::PredictionPublished { .. }
        | RuntimeEvent::ActualRecorded { .. }
        | RuntimeEvent::CorrectionApplied { .. }
        | RuntimeEvent::Extension { .. }
        | RuntimeEvent::Fallthrough { .. } => vec![],
    }
}

// ---------------------------------------------------------------------------
// Private helpers shared with the factory module
// ---------------------------------------------------------------------------

fn workflow_plan_id(run_id: &str) -> String {
    format!("wf-{}", run_id.chars().take(8).collect::<String>())
}

fn workflow_task_id(run_id: &str) -> String {
    format!("workflow-{}", run_id.chars().take(8).collect::<String>())
}

fn workflow_outcome_label(outcome: &WorkflowOutcome) -> String {
    match outcome {
        WorkflowOutcome::Success { commit_hash } => commit_hash
            .as_ref()
            .map_or_else(|| "success".to_string(), |hash| format!("success ({hash})")),
        WorkflowOutcome::Halted { reason } => format!("halted: {reason}"),
        WorkflowOutcome::Cancelled => "cancelled".to_string(),
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Helpers for constructing common [`CoreEvent`] / [`RuntimeEvent`] values.
///
/// These factory functions are thin wrappers over the enum variants; they exist
/// to make call sites more readable and to centralise the field set so that
/// schema changes only require updating one location.
pub mod factory {
    use super::CoreEvent;

    /// Construct a `TaskStarted` event.
    #[inline]
    pub fn task_started(
        run_id: impl Into<String>,
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        task_title: impl Into<String>,
        role: impl Into<String>,
    ) -> CoreEvent {
        CoreEvent::TaskStarted {
            run_id: run_id.into(),
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            task_title: task_title.into(),
            role: role.into(),
        }
    }

    /// Construct a `TaskCompleted` event.
    #[inline]
    pub fn task_completed(
        run_id: impl Into<String>,
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        passed: bool,
        duration_ms: u64,
    ) -> CoreEvent {
        CoreEvent::TaskCompleted {
            run_id: run_id.into(),
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            passed,
            duration_ms,
            outcome: None,
        }
    }

    /// Construct a `GatePassed` event.
    #[inline]
    pub fn gate_passed(
        run_id: impl Into<String>,
        gate_name: impl Into<String>,
        duration_ms: u64,
    ) -> CoreEvent {
        CoreEvent::GatePassed {
            run_id: run_id.into(),
            gate_name: gate_name.into(),
            duration_ms,
        }
    }

    /// Construct a `GateFailed` event.
    #[inline]
    pub fn gate_failed(
        run_id: impl Into<String>,
        gate_name: impl Into<String>,
        output: impl Into<String>,
        duration_ms: u64,
    ) -> CoreEvent {
        CoreEvent::GateFailed {
            run_id: run_id.into(),
            gate_name: gate_name.into(),
            output: output.into(),
            duration_ms,
        }
    }

    /// Construct an `AgentSpawned` event.
    #[inline]
    pub fn agent_spawned(
        run_id: impl Into<String>,
        agent_id: impl Into<String>,
        role: impl Into<String>,
        model: impl Into<String>,
    ) -> CoreEvent {
        CoreEvent::AgentSpawned {
            run_id: run_id.into(),
            agent_id: agent_id.into(),
            role: role.into(),
            model: model.into(),
        }
    }

    /// Construct an `AgentCompleted` event.
    #[inline]
    pub fn agent_completed(
        run_id: impl Into<String>,
        agent_id: impl Into<String>,
        output: impl Into<String>,
        tokens_used: u64,
        cost_usd: f64,
    ) -> CoreEvent {
        CoreEvent::AgentCompleted {
            run_id: run_id.into(),
            agent_id: agent_id.into(),
            output: output.into(),
            tokens_used,
            cost_usd,
        }
    }

    /// Construct a `BudgetUpdated` event.
    #[inline]
    pub fn budget_updated(
        budget_id: impl Into<String>,
        spent_usd: f64,
        limit_usd: f64,
    ) -> CoreEvent {
        let remaining_usd = (limit_usd - spent_usd).max(0.0);
        CoreEvent::BudgetUpdated {
            budget_id: budget_id.into(),
            spent_usd,
            limit_usd,
            remaining_usd,
        }
    }

    /// Construct a `SequenceGap` event.
    #[inline]
    pub fn sequence_gap(
        first_missing_seq: u64,
        last_missing_seq: u64,
        reason: impl Into<String>,
    ) -> CoreEvent {
        CoreEvent::SequenceGap {
            first_missing_seq,
            last_missing_seq,
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeEvent;

    #[test]
    fn core_event_is_runtime_event() {
        // CoreEvent must be the same type as RuntimeEvent — no conversion cost.
        let ev: CoreEvent = factory::task_started("r1", "p1", "t1", "Build", "implementer");
        assert!(matches!(ev, RuntimeEvent::TaskStarted { .. }));
    }

    #[test]
    fn factory_task_started_fields() {
        let ev = factory::task_started("run-1", "plan-1", "task-1", "Do work", "implementer");
        match ev {
            RuntimeEvent::TaskStarted {
                run_id,
                plan_id,
                task_id,
                task_title,
                role,
            } => {
                assert_eq!(run_id, "run-1");
                assert_eq!(plan_id, "plan-1");
                assert_eq!(task_id, "task-1");
                assert_eq!(task_title, "Do work");
                assert_eq!(role, "implementer");
            }
            _ => panic!("expected TaskStarted"),
        }
    }

    #[test]
    fn factory_task_completed_fields() {
        let ev = factory::task_completed("run-1", "plan-1", "task-1", true, 1234);
        match ev {
            RuntimeEvent::TaskCompleted {
                run_id,
                plan_id,
                task_id,
                passed,
                duration_ms,
                outcome,
            } => {
                assert_eq!(run_id, "run-1");
                assert_eq!(plan_id, "plan-1");
                assert_eq!(task_id, "task-1");
                assert!(passed);
                assert_eq!(duration_ms, 1234);
                assert_eq!(outcome, None);
            }
            _ => panic!("expected TaskCompleted"),
        }
    }

    #[test]
    fn factory_gate_events_fields() {
        let passed = factory::gate_passed("r", "compile", 500);
        assert!(matches!(
            passed,
            RuntimeEvent::GatePassed { gate_name, .. } if gate_name == "compile"
        ));

        let failed = factory::gate_failed("r", "test", "2 tests failed", 1500);
        assert!(matches!(
            failed,
            RuntimeEvent::GateFailed { gate_name, output, .. }
                if gate_name == "test" && output == "2 tests failed"
        ));
    }

    #[test]
    fn factory_agent_events_fields() {
        let spawned = factory::agent_spawned("r", "a1", "implementer", "claude-sonnet-4-6");
        assert!(matches!(
            spawned,
            RuntimeEvent::AgentSpawned { agent_id, role, .. }
                if agent_id == "a1" && role == "implementer"
        ));

        let completed = factory::agent_completed("r", "a1", "done", 100, 0.01);
        assert!(matches!(
            completed,
            RuntimeEvent::AgentCompleted {
                tokens_used: 100,
                ..
            }
        ));
    }

    #[test]
    fn factory_budget_updated_computes_remaining() {
        let ev = factory::budget_updated("b1", 3.0, 10.0);
        match ev {
            RuntimeEvent::BudgetUpdated {
                budget_id,
                spent_usd,
                limit_usd,
                remaining_usd,
            } => {
                assert_eq!(budget_id, "b1");
                assert!((spent_usd - 3.0).abs() < f64::EPSILON);
                assert!((limit_usd - 10.0).abs() < f64::EPSILON);
                assert!((remaining_usd - 7.0).abs() < f64::EPSILON);
            }
            _ => panic!("expected BudgetUpdated"),
        }
    }

    #[test]
    fn factory_budget_updated_clamps_negative_remaining() {
        let ev = factory::budget_updated("b2", 15.0, 10.0);
        match ev {
            RuntimeEvent::BudgetUpdated { remaining_usd, .. } => {
                assert_eq!(remaining_usd, 0.0, "remaining must not go negative");
            }
            _ => panic!("expected BudgetUpdated"),
        }
    }

    #[test]
    fn factory_sequence_gap_fields() {
        let ev = factory::sequence_gap(5, 8, "best-effort drops");
        match ev {
            RuntimeEvent::SequenceGap {
                first_missing_seq,
                last_missing_seq,
                reason,
            } => {
                assert_eq!(first_missing_seq, 5);
                assert_eq!(last_missing_seq, 8);
                assert_eq!(reason, "best-effort drops");
            }
            _ => panic!("expected SequenceGap"),
        }
    }

    // -----------------------------------------------------------------------
    // core_event_to_dashboard_events tests
    // -----------------------------------------------------------------------

    #[test]
    fn bridge_task_started_produces_one_dashboard_event() {
        let ev = factory::task_started("r1", "p1", "t1", "Build crate", "implementer");
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(
            out.len(),
            1,
            "TaskStarted should produce exactly one DashboardEvent"
        );
        match &out[0] {
            crate::dashboard_snapshot::DashboardEvent::TaskStarted {
                plan_id,
                task_id,
                title,
                phase,
            } => {
                assert_eq!(plan_id, "p1");
                assert_eq!(task_id, "t1");
                assert_eq!(title, "Build crate");
                assert_eq!(phase, "implementer");
            }
            other => panic!("expected DashboardEvent::TaskStarted, got {other:?}"),
        }
    }

    #[test]
    fn bridge_task_completed_passed() {
        let ev = factory::task_completed("r1", "p1", "t1", true, 500);
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(out.len(), 1);
        match &out[0] {
            crate::dashboard_snapshot::DashboardEvent::TaskCompleted { outcome, .. } => {
                assert_eq!(outcome, "passed");
            }
            other => panic!("expected DashboardEvent::TaskCompleted, got {other:?}"),
        }
    }

    #[test]
    fn bridge_task_completed_failed() {
        let ev = factory::task_completed("r1", "p1", "t1", false, 200);
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(out.len(), 1);
        match &out[0] {
            crate::dashboard_snapshot::DashboardEvent::TaskCompleted { outcome, .. } => {
                assert_eq!(outcome, "failed");
            }
            other => panic!("expected DashboardEvent::TaskCompleted, got {other:?}"),
        }
    }

    #[test]
    fn bridge_agent_spawned() {
        let ev = factory::agent_spawned("r1", "a1", "implementer", "claude-sonnet-4-6");
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(out.len(), 1);
        match &out[0] {
            crate::dashboard_snapshot::DashboardEvent::AgentSpawned {
                agent_id,
                role,
                model,
                ..
            } => {
                assert_eq!(agent_id, "a1");
                assert_eq!(role, "implementer");
                assert_eq!(model, "claude-sonnet-4-6");
            }
            other => panic!("expected DashboardEvent::AgentSpawned, got {other:?}"),
        }
    }

    #[test]
    fn bridge_agent_completed() {
        let ev = factory::agent_completed("r1", "a1", "done", 100, 0.01);
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(out.len(), 1);
        assert!(
            matches!(&out[0], crate::dashboard_snapshot::DashboardEvent::AgentCompleted { agent_id, .. } if agent_id == "a1")
        );
    }

    #[test]
    fn bridge_gate_passed() {
        let ev = factory::gate_passed("r1", "compile", 300);
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(out.len(), 1);
        match &out[0] {
            crate::dashboard_snapshot::DashboardEvent::GateResult { gate, passed, .. } => {
                assert_eq!(gate, "compile");
                assert!(*passed);
            }
            other => panic!("expected DashboardEvent::GateResult, got {other:?}"),
        }
    }

    #[test]
    fn bridge_gate_failed() {
        let ev = factory::gate_failed("r1", "test", "2 failures", 800);
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(out.len(), 1);
        match &out[0] {
            crate::dashboard_snapshot::DashboardEvent::GateResult { gate, passed, .. } => {
                assert_eq!(gate, "test");
                assert!(!*passed);
            }
            other => panic!("expected DashboardEvent::GateResult, got {other:?}"),
        }
    }

    #[test]
    fn bridge_workflow_started_produces_two_events() {
        let ev = RuntimeEvent::WorkflowStarted {
            run_id: "wf-001".into(),
            template: "express".into(),
            prompt: "fix the bug".into(),
        };
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(
            out.len(),
            2,
            "WorkflowStarted should produce PlanStarted + TaskStarted"
        );
        assert!(matches!(
            &out[0],
            crate::dashboard_snapshot::DashboardEvent::PlanStarted { .. }
        ));
        assert!(matches!(
            &out[1],
            crate::dashboard_snapshot::DashboardEvent::TaskStarted { .. }
        ));
    }

    #[test]
    fn bridge_workflow_completed_produces_two_events() {
        let ev = RuntimeEvent::WorkflowCompleted {
            run_id: "wf-001".into(),
            outcome: WorkflowOutcome::Success { commit_hash: None },
        };
        let out = core_event_to_dashboard_events(&ev);
        assert_eq!(
            out.len(),
            2,
            "WorkflowCompleted should produce TaskCompleted + PlanCompleted"
        );
        assert!(matches!(
            &out[0],
            crate::dashboard_snapshot::DashboardEvent::TaskCompleted { .. }
        ));
        assert!(matches!(
            &out[1],
            crate::dashboard_snapshot::DashboardEvent::PlanCompleted { success: true, .. }
        ));
    }

    #[test]
    fn bridge_no_op_variants_return_empty() {
        // Variants that have no dashboard representation must return empty vec.
        let unmapped: &[RuntimeEvent] = &[
            RuntimeEvent::FeedbackRecorded {
                run_id: "r".into(),
                kind: "gate".into(),
                summary: "ok".into(),
            },
            RuntimeEvent::StateCheckpointed {
                run_id: "r".into(),
                path: "/tmp/snap".into(),
            },
            RuntimeEvent::SequenceGap {
                first_missing_seq: 5,
                last_missing_seq: 7,
                reason: "test".into(),
            },
        ];
        for ev in unmapped {
            let out = core_event_to_dashboard_events(ev);
            assert!(
                out.is_empty(),
                "expected empty vec for {ev:?} but got {out:?}"
            );
        }
    }
}
