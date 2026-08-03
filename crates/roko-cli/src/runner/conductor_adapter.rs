//! Adapter that maps runner-v2 events to conductor [`Engram`]s.
//!
//! The conductor watchers consume typed [`Engram`] streams (ghost-turn
//! signals, gate verdicts, cost metrics, plan phases). This module provides
//! pure mapping functions that convert [`RunnerEvent`] and [`AgentEvent`]
//! instances into the `Option<Engram>` that watchers expect, without
//! performing any IO or mutating runner state.
//!
//! Only events that at least one conductor watcher consumes are mapped;
//! everything else returns `None` to avoid ring buffer churn.

use roko_core::{Body, Engram, Kind};

use super::types::{AgentEvent, RunnerEvent};

// ─── Tag key constants ──────────────────────────────────────────────────
//
// Reuse exactly the tag keys that the conductor watchers already read.
// Do NOT invent new keys here.

/// Tag key used by conductor watchers to identify the plan.
const PLAN_ID_TAG: &str = "plan_id";
/// Tag key for task identification.
const TASK_TAG: &str = "task";
/// Tag key for the model name.
const MODEL_TAG: &str = "model";
/// Tag key for the provider name.
const PROVIDER_TAG: &str = "provider";
/// Tag key used on `Kind::Metric` signals for the metric name.
const METRIC_NAME_TAG: &str = "name";
/// Tag key used on `Kind::Metric` signals for the numeric value.
const METRIC_VALUE_TAG: &str = "value";
/// Tag key for severity on intervention signals.
const SEVERITY_TAG: &str = "severity";

/// Custom kind string the ghost-turn watcher listens for.
const GHOST_TURN_KIND: &str = "conductor.ghost_turn";

// ─── RunnerEvent -> Engram ──────────────────────────────────────────────

/// Map a [`RunnerEvent`] to an [`Engram`] the conductor can consume.
///
/// Returns `None` for events no watcher reads (the vast majority), keeping
/// ring buffer usage bounded.
#[must_use]
pub fn runner_event_to_engram(event: &RunnerEvent) -> Option<Engram> {
    match event {
        // Plan lifecycle -> PlanPhase engrams (used by extract_plan_id).
        RunnerEvent::PlanStarted { plan_id, .. } => Some(
            Engram::builder(Kind::PlanPhase)
                .body(Body::text("started"))
                .tag(PLAN_ID_TAG, plan_id.as_str())
                .tag("phase", "started")
                .build(),
        ),

        RunnerEvent::PlanCompleted {
            plan_id, cost_usd, ..
        } => Some(
            Engram::builder(Kind::PlanPhase)
                .body(Body::text("completed"))
                .tag(PLAN_ID_TAG, plan_id.as_str())
                .tag("phase", "completed")
                .tag("cost_usd", format!("{cost_usd:.4}"))
                .build(),
        ),

        // Gate verdicts -> GateVerdict engrams (consumed by
        // TestFailureBudgetWatcher and other gate-aware watchers).
        RunnerEvent::GateCompleted {
            attempt,
            kind,
            rung,
            passed,
            failure_kind,
            duration_ms,
            verdicts,
            ..
        } => {
            let test_count = verdicts.iter().fold((0u32, 0u32), |(p, f), v| {
                if v.skipped {
                    (p, f)
                } else if v.passed {
                    (p + 1, f)
                } else {
                    (p, f + 1)
                }
            });

            let body = Body::from_json(&serde_json::json!({
                "plan_id": attempt.plan_id,
                "task": attempt.task_id,
                "gate": format!("{kind:?}"),
                "rung": rung,
                "passed": passed,
                "failure_kind": failure_kind.map(|fk| format!("{fk:?}")),
                "duration_ms": duration_ms,
                "test_count": {
                    "passed": test_count.0,
                    "failed": test_count.1,
                },
            }))
            .ok()?;

            Some(
                Engram::builder(Kind::GateVerdict)
                    .body(body)
                    .tag(PLAN_ID_TAG, &attempt.plan_id)
                    .tag(TASK_TAG, &attempt.task_id)
                    .tag(SEVERITY_TAG, if *passed { "info" } else { "error" })
                    .build(),
            )
        }

        // Task attempt completed -> Metric cost signal for CostOverrunWatcher.
        RunnerEvent::TaskAttemptCompleted {
            attempt,
            model,
            provider,
            ..
        } => {
            // We emit nothing here directly. Cost is tracked via
            // AgentCompleted which has total_cost_usd.
            // However, we do emit an AgentOutput-kind engram for model/provider
            // tracking used by extract_provider.
            let mut builder = Engram::builder(Kind::AgentOutput)
                .body(Body::text(format!(
                    "task attempt completed: {}/{}",
                    attempt.plan_id, attempt.task_id
                )))
                .tag(PLAN_ID_TAG, &attempt.plan_id)
                .tag(TASK_TAG, &attempt.task_id);

            if !model.is_empty() {
                builder = builder.tag(MODEL_TAG, model.as_str());
            }
            if !provider.is_empty() {
                builder = builder.tag(PROVIDER_TAG, provider.as_str());
            }

            Some(builder.build())
        }

        // Agent completed with cost -> Metric engram for CostOverrunWatcher.
        RunnerEvent::AgentCompleted {
            attempt,
            total_cost_usd,
            ..
        } => {
            let cost = (*total_cost_usd)?;
            Some(
                Engram::builder(Kind::Metric)
                    .body(Body::text(format!("plan_cost={cost:.4}")))
                    .tag(METRIC_NAME_TAG, "plan_cost")
                    .tag(METRIC_VALUE_TAG, format!("{cost:.4}"))
                    .tag(PLAN_ID_TAG, &attempt.plan_id)
                    .tag(TASK_TAG, &attempt.task_id)
                    .build(),
            )
        }

        // All other events are not consumed by any conductor watcher.
        _ => None,
    }
}

// ─── AgentEvent -> Engram ───────────────────────────────────────────────

/// Convert an [`AgentEvent`] (turn-level) into a conductor-consumable
/// [`Engram`], if applicable.
///
/// The `plan_id`, `task_id`, `model`, and `provider` are supplied by the
/// caller from the dispatch context — the agent event itself does not
/// carry plan-level metadata.
///
/// Currently maps:
/// - `TurnCompleted` -> ghost-turn signal (the primary input for
///   `GhostTurnWatcher`)
/// - `Started` -> provider tracking signal
///
/// All other variants return `None`.
#[must_use]
pub fn agent_event_to_engram(
    event: &AgentEvent,
    plan_id: &str,
    task_id: &str,
    model: &str,
    provider: &str,
) -> Option<Engram> {
    match event {
        AgentEvent::TurnCompleted {
            total_cost_usd,
            is_error,
            ..
        } => {
            // Emit the exact schema the ghost-turn watcher deserializes.
            // We mark turns as ghost turns when there is an error or
            // when cost is zero (likely no meaningful output). The
            // actual ghost-turn classification is refined downstream;
            // we emit all turns and let the watcher filter.
            let cost_usd = total_cost_usd.unwrap_or(0.0);
            let body = Body::from_json(&serde_json::json!({
                "plan_id": plan_id,
                "task": task_id,
                "role": "Agent",
                "model": model,
                "cost_usd": cost_usd,
                "duration_ms": 0,
                "changed_files_before": [],
                "changed_files_after": [],
                "net_new_changes": 0,
                "output_meaningful": !is_error,
                "wasted_cost": *is_error,
            }))
            .ok()?;

            Some(
                Engram::builder(Kind::Custom(GHOST_TURN_KIND.into()))
                    .body(body)
                    .tag(PLAN_ID_TAG, plan_id)
                    .tag(TASK_TAG, task_id)
                    .tag(MODEL_TAG, model)
                    .tag(PROVIDER_TAG, provider)
                    .build(),
            )
        }

        AgentEvent::Started {
            provider: started_provider,
            model: started_model,
            ..
        } => Some(
            Engram::builder(Kind::AgentOutput)
                .body(Body::text("agent started"))
                .tag(PLAN_ID_TAG, plan_id)
                .tag(TASK_TAG, task_id)
                .tag(MODEL_TAG, started_model.as_str())
                .tag(PROVIDER_TAG, started_provider.as_str())
                .build(),
        ),

        // MessageDelta, ToolCall, ToolOutput, TokenUsage, SystemInit,
        // Error, Exited — no watcher consumes these.
        _ => None,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::types::{
        GateCompletionKind, GateVerdictSummary, RunnerFailureKind, TaskAttemptOutcome,
        TaskAttemptRef, TaskPhaseDurations,
    };
    use super::*;
    use roko_core::{Context, React};

    fn test_attempt() -> TaskAttemptRef {
        TaskAttemptRef {
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            attempt: 1,
        }
    }

    // ── RunnerEvent mapping tests ───────────────────────────────────────

    #[test]
    fn plan_started_maps_to_plan_phase() {
        let event = RunnerEvent::PlanStarted {
            timestamp: String::new(),
            timestamp_ms: 0,
            run_id: "run-1".into(),
            plan_id: "plan-1".into(),
        };
        let engram = runner_event_to_engram(&event).expect("should map");
        assert_eq!(engram.kind, Kind::PlanPhase);
        assert_eq!(engram.tag(PLAN_ID_TAG), Some("plan-1"));
    }

    #[test]
    fn gate_completed_pass_maps_to_gate_verdict() {
        let event = RunnerEvent::GateCompleted {
            timestamp: String::new(),
            timestamp_ms: 0,
            run_id: "run-1".into(),
            attempt: test_attempt(),
            kind: GateCompletionKind::Gate,
            rung: 2,
            passed: true,
            failure_kind: None,
            duration_ms: 500,
            output: String::new(),
            verdicts: vec![GateVerdictSummary {
                gate_name: "cargo-test".into(),
                passed: true,
                skipped: false,
                summary: "ok".into(),
                error_digest: None,
                failure_kind: None,
            }],
        };
        let engram = runner_event_to_engram(&event).expect("should map");
        assert_eq!(engram.kind, Kind::GateVerdict);
        assert_eq!(engram.tag(SEVERITY_TAG), Some("info"));
    }

    #[test]
    fn gate_completed_fail_maps_to_gate_verdict_error() {
        let event = RunnerEvent::GateCompleted {
            timestamp: String::new(),
            timestamp_ms: 0,
            run_id: "run-1".into(),
            attempt: test_attempt(),
            kind: GateCompletionKind::Gate,
            rung: 1,
            passed: false,
            failure_kind: Some(RunnerFailureKind::Transient),
            duration_ms: 1200,
            output: "compile error".into(),
            verdicts: vec![
                GateVerdictSummary {
                    gate_name: "cargo-check".into(),
                    passed: false,
                    skipped: false,
                    summary: "failed".into(),
                    error_digest: Some("error[E0308]".into()),
                    failure_kind: Some(RunnerFailureKind::Transient),
                },
                GateVerdictSummary {
                    gate_name: "cargo-test".into(),
                    passed: false,
                    skipped: false,
                    summary: "3 failures".into(),
                    error_digest: None,
                    failure_kind: Some(RunnerFailureKind::Transient),
                },
            ],
        };
        let engram = runner_event_to_engram(&event).expect("should map");
        assert_eq!(engram.kind, Kind::GateVerdict);
        assert_eq!(engram.tag(SEVERITY_TAG), Some("error"));
        // Verify test_count is populated for TestFailureBudgetWatcher
        let body: serde_json::Value = engram.body.as_json().unwrap();
        assert_eq!(body["test_count"]["failed"], 2);
        assert_eq!(body["test_count"]["passed"], 0);
    }

    #[test]
    fn agent_completed_with_cost_maps_to_metric() {
        let event = RunnerEvent::AgentCompleted {
            timestamp: String::new(),
            timestamp_ms: 0,
            run_id: "run-1".into(),
            attempt: test_attempt(),
            agent_id: "agent-1".into(),
            outcome: super::super::types::AgentDispatchOutcome::Completed,
            session_id: None,
            total_cost_usd: Some(1.50),
            turns: Some(3),
            exit_code: Some(0),
            message: None,
        };
        let engram = runner_event_to_engram(&event).expect("should map");
        assert_eq!(engram.kind, Kind::Metric);
        assert_eq!(engram.tag(METRIC_NAME_TAG), Some("plan_cost"));
    }

    #[test]
    fn agent_completed_without_cost_returns_none() {
        let event = RunnerEvent::AgentCompleted {
            timestamp: String::new(),
            timestamp_ms: 0,
            run_id: "run-1".into(),
            attempt: test_attempt(),
            agent_id: "agent-1".into(),
            outcome: super::super::types::AgentDispatchOutcome::Completed,
            session_id: None,
            total_cost_usd: None,
            turns: None,
            exit_code: None,
            message: None,
        };
        assert!(runner_event_to_engram(&event).is_none());
    }

    #[test]
    fn unmapped_events_return_none() {
        let event = RunnerEvent::RunStarted {
            timestamp: String::new(),
            timestamp_ms: 0,
            run_id: "run-1".into(),
            plan_ids: vec!["plan-1".into()],
            total_tasks: 5,
            resumed: false,
            resume_session: None,
        };
        assert!(runner_event_to_engram(&event).is_none());
    }

    // ── AgentEvent mapping tests ────────────────────────────────────────

    #[test]
    fn turn_completed_maps_to_ghost_turn() {
        let event = AgentEvent::TurnCompleted {
            session_id: Some("sess-1".into()),
            total_cost_usd: Some(0.50),
            num_turns: Some(1),
            is_error: false,
        };
        let engram =
            agent_event_to_engram(&event, "plan-1", "task-1", "claude-sonnet-4-6", "anthropic")
                .expect("should map");
        assert!(matches!(engram.kind, Kind::Custom(ref k) if k == GHOST_TURN_KIND));
        assert_eq!(engram.tag(PLAN_ID_TAG), Some("plan-1"));
        assert_eq!(engram.tag(MODEL_TAG), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn error_turn_maps_to_wasted_ghost_turn() {
        let event = AgentEvent::TurnCompleted {
            session_id: None,
            total_cost_usd: Some(0.25),
            num_turns: Some(1),
            is_error: true,
        };
        let engram = agent_event_to_engram(&event, "plan-1", "task-1", "model-x", "provider-y")
            .expect("should map");
        let body: serde_json::Value = engram.body.as_json().unwrap();
        assert_eq!(body["wasted_cost"], true);
        assert_eq!(body["output_meaningful"], false);
    }

    #[test]
    fn agent_started_maps_to_agent_output() {
        let event = AgentEvent::Started {
            agent_id: "agent-1".into(),
            provider: "anthropic".into(),
            model: "claude-sonnet-4-6".into(),
            pid: Some(12345),
        };
        let engram =
            agent_event_to_engram(&event, "plan-1", "task-1", "claude-sonnet-4-6", "anthropic")
                .expect("should map");
        assert_eq!(engram.kind, Kind::AgentOutput);
        assert_eq!(engram.tag(PROVIDER_TAG), Some("anthropic"));
    }

    #[test]
    fn message_delta_returns_none() {
        let event = AgentEvent::MessageDelta {
            text: "hello".into(),
        };
        assert!(agent_event_to_engram(&event, "plan-1", "task-1", "m", "p").is_none());
    }

    // ── Integration: ghost-turn stream drives non-Continue decision ─────

    #[test]
    fn ghost_turn_stream_drives_conductor_non_continue() {
        use roko_conductor::Conductor;

        // Build a stream of 4 error turns (ghost turns) — exceeds the
        // default MAX_GHOST_TURNS (3).
        let mut stream: Vec<Engram> = Vec::new();

        // First add a PlanPhase engram so extract_plan_id works.
        stream.push(
            Engram::builder(Kind::PlanPhase)
                .body(Body::text("implementing"))
                .tag(PLAN_ID_TAG, "plan-1")
                .tag("phase", "implementing")
                .build(),
        );

        for i in 0..4 {
            let event = AgentEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: Some(0.50 + i as f64 * 0.1),
                num_turns: Some(1),
                is_error: true,
            };
            let engram =
                agent_event_to_engram(&event, "plan-1", "task-1", "claude-sonnet-4-6", "anthropic")
                    .expect("error turn should map to ghost-turn engram");
            stream.push(engram);
        }

        let conductor = Conductor::default();
        let eval = conductor.evaluate_full(&stream, &Context::now());
        assert!(
            !eval.decision.is_continue(),
            "4 ghost turns should trigger a non-Continue decision, got: {:?}",
            eval.decision
        );
    }

    #[test]
    fn gate_fail_stream_drives_conductor_non_continue() {
        use roko_conductor::Conductor;

        // Build a stream with repeated gate failures on the same plan.
        let mut stream: Vec<Engram> = Vec::new();

        // PlanPhase for plan identity.
        stream.push(
            Engram::builder(Kind::PlanPhase)
                .body(Body::text("implementing"))
                .tag(PLAN_ID_TAG, "plan-1")
                .tag("phase", "implementing")
                .build(),
        );

        // Emit many failed gate verdict engrams — drives test-failure-budget
        // and iteration-loop watchers.
        for i in 0..6 {
            let event = RunnerEvent::GateCompleted {
                timestamp: String::new(),
                timestamp_ms: i * 1000,
                run_id: "run-1".into(),
                attempt: TaskAttemptRef {
                    plan_id: "plan-1".into(),
                    task_id: "task-1".into(),
                    attempt: i as u32 + 1,
                },
                kind: GateCompletionKind::Gate,
                rung: 1,
                passed: false,
                failure_kind: Some(RunnerFailureKind::Transient),
                duration_ms: 500,
                output: format!("failure {i}"),
                verdicts: vec![GateVerdictSummary {
                    gate_name: "cargo-test".into(),
                    passed: false,
                    skipped: false,
                    summary: format!("{} test failures", i + 1),
                    error_digest: None,
                    failure_kind: Some(RunnerFailureKind::Transient),
                }],
            };
            let engram = runner_event_to_engram(&event).expect("gate failure should map to engram");
            stream.push(engram);
        }

        let conductor = Conductor::default();
        let eval = conductor.evaluate_full(&stream, &Context::now());
        assert!(
            !eval.decision.is_continue(),
            "6 gate failures should trigger a non-Continue decision, got: {:?}",
            eval.decision
        );
    }

    #[test]
    fn task_attempt_completed_maps_with_model_provider() {
        let event = RunnerEvent::TaskAttemptCompleted {
            timestamp: String::new(),
            timestamp_ms: 0,
            run_id: "run-1".into(),
            attempt: test_attempt(),
            outcome: TaskAttemptOutcome::Passed,
            failure_kind: None,
            duration_ms: 3000,
            phase_durations: TaskPhaseDurations::default(),
            model: "claude-sonnet-4-6".into(),
            provider: "anthropic".into(),
        };
        let engram = runner_event_to_engram(&event).expect("should map");
        assert_eq!(engram.kind, Kind::AgentOutput);
        assert_eq!(engram.tag(MODEL_TAG), Some("claude-sonnet-4-6"));
        assert_eq!(engram.tag(PROVIDER_TAG), Some("anthropic"));
    }
}
