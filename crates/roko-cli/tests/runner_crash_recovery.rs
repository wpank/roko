//! Deterministic crash-chain replay fixture for runner state transitions.
//!
//! Exercises the state management and event processing that SH01-SH05 fixed:
//! concurrent completions, gate failure + retry, blocked-dependent skipping,
//! retry exhaustion, timeout events, and terminal reconciliation.
//! All tests are unit-level (no I/O, no network, no external processes).

#![allow(clippy::unwrap_used, clippy::many_single_char_names)]

use std::collections::HashSet;

use roko_cli::runner::state::RunState;
use roko_cli::runner::task_dag::{DagConfig, SkippedReason, TaskDag};
use roko_cli::runner::types::*;
use roko_cli::task_parser::TaskDef;

// ─── Helpers ────────────────────────────────────────────────────────────

fn td(id: &str, deps: &[&str]) -> TaskDef {
    TaskDef {
        id: id.into(),
        title: id.into(),
        description: None,
        role: None,
        status: "ready".into(),
        tier: "focused".into(),
        frequency: None,
        model_hint: None,
        replan_strategy: None,
        max_loc: None,
        files: vec![],
        allowed_tools: None,
        denied_tools: None,
        mcp_servers: None,
        depends_on: deps.iter().map(|s| s.to_string()).collect(),
        depends_on_plan: vec![],
        split_into: None,
        context: None,
        verify: vec![],
        timeout_secs: 60,
        max_retries: 2,
        acceptance: vec![],
        acceptance_contract: None,
        domain: None,
        sequence: 0,
    }
}
fn aref(plan: &str, task: &str, n: u32) -> TaskAttemptRef {
    TaskAttemptRef::new(plan, task, n)
}
fn gc(plan: &str, task: &str, ok: bool) -> GateCompletion {
    GateCompletion {
        effect: None,
        attempt: Some(aref(plan, task, 1)),
        kind: GateCompletionKind::Gate,
        plan_id: plan.into(),
        task_id: task.into(),
        rung: 1,
        passed: ok,
        failure_kind: if ok {
            None
        } else {
            Some(RunnerFailureKind::Transient)
        },
        verdicts: Vec::new(),
        output: String::new(),
        duration_ms: 100,
    }
}
const RUN: &str = "crash-test";
/// Start a fresh RunState with run + plan events applied.
fn fresh(total: usize) -> RunState {
    let mut state = RunState::new(total);
    state.apply_runner_event(&RunnerEvent::run_started(
        RUN,
        vec!["p1".into()],
        total,
        false,
        None,
    ));
    state.apply_runner_event(&RunnerEvent::plan_started(RUN, "p1"));
    state
}

// ─── 1: Concurrent completions don't duplicate terminal events ──────────

#[test]
fn concurrent_completions_no_duplicate_terminal() {
    let mut state = fresh(2);
    let (ra, rb) = (aref("p1", "A", 1), aref("p1", "B", 1));
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, ra.clone(), "A"));
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, rb.clone(), "B"));

    // Both complete back-to-back
    for ref_ in [&ra, &rb] {
        state.apply_runner_event(&RunnerEvent::task_attempt_completed(
            RUN,
            ref_.clone(),
            TaskAttemptOutcome::Passed,
            None,
            1000,
            "s",
            "c",
        ));
    }
    // Duplicate of A (race condition) — must not corrupt state
    state.apply_runner_event(&RunnerEvent::task_attempt_completed(
        RUN,
        ra.clone(),
        TaskAttemptOutcome::Passed,
        None,
        1001,
        "s",
        "c",
    ));

    assert_eq!(
        state.lifecycle.tasks.get("p1:A").unwrap().status,
        TaskLifecycleStatus::Passed
    );
    assert_eq!(
        state.lifecycle.tasks.get("p1:B").unwrap().status,
        TaskLifecycleStatus::Passed
    );
    let count = state
        .lifecycle
        .tasks
        .values()
        .filter(|t| t.plan_id == "p1")
        .count();
    assert_eq!(count, 2, "exactly 2 task lifecycles, not more");
    assert!(
        state
            .lifecycle
            .task_attempts
            .get(&ra.key())
            .unwrap()
            .completed_at_ms
            .is_some()
    );
}

// ─── 2: Gate failure triggers retry; supersedes prior attempt ───────────

#[test]
fn gate_failure_retry_and_supersede() {
    let mut state = fresh(1);
    let a1 = aref("p1", "A", 1);
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, a1.clone(), "A"));
    state.apply_runner_event(&RunnerEvent::agent_dispatch_completed(
        RUN,
        a1.clone(),
        "ag",
        AgentDispatchOutcome::Spawned,
        Some("s".into()),
        Some(1),
        None,
    ));
    state.apply_runner_event(&RunnerEvent::gate_dispatch_started(
        RUN,
        a1.clone(),
        GateCompletionKind::Gate,
        1,
    ));
    state.apply_runner_event(&RunnerEvent::gate_completed(
        RUN,
        a1.clone(),
        &gc("p1", "A", false),
    ));

    let att = state.lifecycle.task_attempts.get(&a1.key()).unwrap();
    assert_eq!(att.status, TaskAttemptStatus::GateFailed);
    assert_eq!(att.failure_kind, Some(RunnerFailureKind::Transient));

    let rd = RetryDecision::for_failure(RunnerFailureKind::Transient, 1, 2, "retry");
    assert!(rd.should_retry());
    state.apply_runner_event(&RunnerEvent::retry_decision(RUN, a1.clone(), rd));
    assert_eq!(
        state.lifecycle.tasks.get("p1:A").unwrap().status,
        TaskLifecycleStatus::Retrying
    );

    // Attempt 2 starts — prior attempt must be superseded
    let a2 = aref("p1", "A", 2);
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, a2.clone(), "A"));
    assert_eq!(
        state.lifecycle.task_attempts.get(&a1.key()).unwrap().status,
        TaskAttemptStatus::Superseded,
    );

    // Attempt 2 passes
    state.apply_runner_event(&RunnerEvent::task_attempt_completed(
        RUN,
        a2.clone(),
        TaskAttemptOutcome::Passed,
        None,
        2000,
        "s",
        "c",
    ));
    assert_eq!(
        state.lifecycle.tasks.get("p1:A").unwrap().status,
        TaskLifecycleStatus::Passed
    );
}

// ─── 3: Blocked dependents get skipped on prerequisite failure ──────────

#[test]
fn blocked_dependent_skipped() {
    let (ta, tb, tc) = (td("A", &[]), td("B", &["A"]), td("C", &["B"]));
    let tasks: Vec<&TaskDef> = vec![&ta, &tb, &tc];
    let mut dag = TaskDag::new(DagConfig::default());
    dag.mark_running("p1", "A");

    let mut skipped = dag.mark_failed_blocking_downstream("p1", "A", &tasks);
    skipped.sort();
    assert_eq!(skipped, vec!["B".to_string(), "C".to_string()]);

    let plan = dag.plan("p1").unwrap();
    assert!(plan.failed.contains("A"));
    assert!(!plan.running.contains("A"));
    assert!(matches!(plan.skipped.get("B"),
        Some(SkippedReason::PrerequisiteFailed { prerequisite }) if prerequisite == "A"));
    assert!(matches!(plan.skipped.get("C"),
        Some(SkippedReason::PrerequisiteFailed { prerequisite }) if prerequisite == "B"));
    assert!(dag.ready_tasks("p1", &tasks, &[], &[]).is_empty());

    // Lifecycle: B and C never dispatched
    let mut state = fresh(3);
    let a1 = aref("p1", "A", 1);
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, a1.clone(), "A"));
    state.apply_runner_event(&RunnerEvent::task_attempt_completed(
        RUN,
        a1,
        TaskAttemptOutcome::Failed,
        Some(RunnerFailureKind::Permanent),
        500,
        "s",
        "c",
    ));
    assert!(
        !state
            .lifecycle
            .task_attempts
            .keys()
            .any(|k| k.contains('B') || k.contains('C'))
    );
}

// ─── 4: Retry exhaustion marks task as Exhausted ────────────────────────

#[test]
fn retry_exhaustion() {
    let mut state = fresh(1);
    let fail = gc("p1", "A", false);

    // 3 attempts, budget of 2 retries
    for num in 1..=3u32 {
        let ar = aref("p1", "A", num);
        state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, ar.clone(), "A"));
        state.apply_runner_event(&RunnerEvent::gate_dispatch_started(
            RUN,
            ar.clone(),
            GateCompletionKind::Gate,
            1,
        ));
        state.apply_runner_event(&RunnerEvent::gate_completed(RUN, ar.clone(), &fail));
        let rd = RetryDecision::for_failure(RunnerFailureKind::Transient, num, 2, "fail");
        state.apply_runner_event(&RunnerEvent::retry_decision(RUN, ar, rd));
    }

    let task = state.lifecycle.tasks.get("p1:A").unwrap();
    assert_eq!(task.status, TaskLifecycleStatus::Exhausted);
    assert!(task.completed_at_ms.is_some());
    assert_eq!(
        state
            .lifecycle
            .task_attempts
            .get(&aref("p1", "A", 3).key())
            .unwrap()
            .status,
        TaskAttemptStatus::Exhausted,
    );

    // Permanent failures are never retryable
    let perm = RetryDecision::for_failure(RunnerFailureKind::Permanent, 1, 5, "p");
    assert!(!perm.should_retry());
    assert_eq!(perm.action, RetryAction::NotRetryable);
}

// ─── 5: Timeout produces event and proper cleanup ───────────────────────

#[test]
fn timeout_event_and_cleanup() {
    let mut state = fresh(2);
    let a1 = aref("p1", "A", 1);
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, a1.clone(), "A"));
    state.apply_runner_event(&RunnerEvent::agent_dispatch_completed(
        RUN,
        a1.clone(),
        "ag",
        AgentDispatchOutcome::Spawned,
        Some("s".into()),
        Some(1),
        None,
    ));
    // Timeout path: AgentRunning -> Cancelling -> TimedOut
    state.apply_runner_event(&RunnerEvent::task_attempt_cancellation_requested(
        RUN,
        a1.clone(),
    ));
    state.apply_runner_event(&RunnerEvent::timeout_recorded(
        RUN,
        TimeoutEvent {
            kind: TimeoutKind::TaskAttempt,
            attempt: Some(a1.clone()),
            effect: None,
            owner_effect: None,
            limit_ms: 60_000,
            monotonic_elapsed_ms: 65_000,
            observed_at_ms: 1_700_000_065_000,
        },
    ));

    assert_eq!(
        state.lifecycle.task_attempts.get(&a1.key()).unwrap().status,
        TaskAttemptStatus::TimedOut,
    );
    let task = state.lifecycle.tasks.get("p1:A").unwrap();
    assert_eq!(task.status, TaskLifecycleStatus::TimedOut);
    assert!(task.status.is_terminal());

    // Global timeout (no attempt)
    state.apply_runner_event(&RunnerEvent::timeout_recorded(
        RUN,
        TimeoutEvent {
            kind: TimeoutKind::HardRun,
            attempt: None,
            effect: None,
            owner_effect: None,
            limit_ms: 3_600_000,
            monotonic_elapsed_ms: 3_700_000,
            observed_at_ms: 1_700_003_700_000,
        },
    ));
    let gt = state
        .lifecycle
        .global_timeout
        .as_ref()
        .expect("global timeout stored");
    assert_eq!(gt.kind, TimeoutKind::HardRun);
    assert_eq!(gt.limit_ms, 3_600_000);
}

// ─── 6: Terminal reconciliation — full crash-chain ──────────────────────

#[test]
fn terminal_reconciliation() {
    // DAG: A(pass), B(exhaust), C->A(pass), D->B(skip)
    let mut state = fresh(4);
    let mut dag = TaskDag::new(DagConfig::default());
    let (ta, tb, tc, td_) = (td("A", &[]), td("B", &[]), td("C", &["A"]), td("D", &["B"]));
    let tasks: Vec<&TaskDef> = vec![&ta, &tb, &tc, &td_];

    dag.mark_running("p1", "A");
    dag.mark_running("p1", "B");
    let (a1, b1) = (aref("p1", "A", 1), aref("p1", "B", 1));
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, a1.clone(), "A"));
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, b1.clone(), "B"));

    // A passes
    state.apply_runner_event(&RunnerEvent::task_attempt_completed(
        RUN,
        a1,
        TaskAttemptOutcome::Passed,
        None,
        1000,
        "s",
        "c",
    ));
    dag.mark_complete("p1", "A");

    // B gate-fails, retries once, then exhausted
    let bfail = gc("p1", "B", false);
    state.apply_runner_event(&RunnerEvent::gate_completed(RUN, b1.clone(), &bfail));
    state.apply_runner_event(&RunnerEvent::retry_decision(
        RUN,
        b1,
        RetryDecision::for_failure(RunnerFailureKind::Transient, 1, 1, "f"),
    ));
    let b2 = aref("p1", "B", 2);
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, b2.clone(), "B"));
    state.apply_runner_event(&RunnerEvent::gate_completed(RUN, b2.clone(), &bfail));
    state.apply_runner_event(&RunnerEvent::retry_decision(
        RUN,
        b2,
        RetryDecision::for_failure(RunnerFailureKind::Transient, 2, 1, "ex"),
    ));

    let skipped = dag.mark_failed_blocking_downstream("p1", "B", &tasks);
    assert_eq!(skipped, vec!["D".to_string()]);

    // C unblocked, runs, passes
    assert_eq!(dag.ready_tasks("p1", &tasks, &["A".into()], &[]).len(), 1);
    dag.mark_running("p1", "C");
    let c1 = aref("p1", "C", 1);
    state.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, c1.clone(), "C"));
    state.apply_runner_event(&RunnerEvent::task_attempt_completed(
        RUN,
        c1,
        TaskAttemptOutcome::Passed,
        None,
        1500,
        "s",
        "c",
    ));
    dag.mark_complete("p1", "C");

    // Final events
    let plan = dag.plan("p1").unwrap();
    state.apply_runner_event(&RunnerEvent::plan_completed(
        RUN,
        "p1",
        PlanOutcome::Failed,
        Some("B exhausted".into()),
        0.10,
        plan.completed.len(),
        plan.failed.len(),
    ));
    state.apply_runner_event(&RunnerEvent::run_completed(
        RUN,
        RunOutcome::Failed,
        RunTotals {
            total_tasks: 4,
            tasks_completed: 2,
            tasks_failed: 1,
            tasks_blocked: 0,
            tasks_skipped: 1,
            tasks_active: 0,
            tasks_pending: 0,
            total_agent_calls: 4,
            total_cost_usd: 0.10,
            duration_ms: 5000,
        },
        vec![],
    ));

    assert_eq!(state.lifecycle.status, RunnerRunStatus::Failed);
    assert_eq!(
        state.lifecycle.tasks.get("p1:A").unwrap().status,
        TaskLifecycleStatus::Passed
    );
    assert_eq!(
        state.lifecycle.tasks.get("p1:B").unwrap().status,
        TaskLifecycleStatus::Exhausted
    );
    assert_eq!(
        state.lifecycle.tasks.get("p1:C").unwrap().status,
        TaskLifecycleStatus::Passed
    );
    assert_eq!(
        state
            .lifecycle
            .tasks
            .get("p1:B")
            .unwrap()
            .latest_failure_kind,
        Some(RunnerFailureKind::Transient),
    );
    for key in ["p1:A", "p1:B", "p1:C"] {
        assert!(
            state
                .lifecycle
                .tasks
                .get(key)
                .unwrap()
                .completed_at_ms
                .is_some(),
            "{key} must have completed_at_ms"
        );
    }

    let failed_set: HashSet<String> = plan.failed.clone();
    let completed: Vec<String> = plan.completed.iter().cloned().collect();
    let summary = dag.progress_summary("p1", &tasks, &completed, &failed_set, &[], &[]);
    assert_eq!(
        (
            summary.terminal,
            summary.ready,
            summary.active,
            summary.blocked
        ),
        (4, 0, 0, 0)
    );
}

// ─── 7: Attempt status transition guards ────────────────────────────────

#[test]
fn transition_guards() {
    // Terminal states block all transitions
    for ts in [
        TaskAttemptStatus::Passed,
        TaskAttemptStatus::Exhausted,
        TaskAttemptStatus::Cancelled,
        TaskAttemptStatus::TimedOut,
        TaskAttemptStatus::Superseded,
    ] {
        assert!(!ts.can_transition_to(TaskAttemptStatus::Started));
        assert!(!ts.can_transition_to(TaskAttemptStatus::AgentRunning));
    }
    // Valid paths
    assert!(TaskAttemptStatus::Started.can_transition_to(TaskAttemptStatus::DispatchingAgent));
    assert!(TaskAttemptStatus::Gating.can_transition_to(TaskAttemptStatus::GateFailed));
    assert!(TaskAttemptStatus::GateFailed.can_transition_to(TaskAttemptStatus::Exhausted));
    assert!(TaskAttemptStatus::Retrying.can_transition_to(TaskAttemptStatus::Superseded));
    assert!(TaskAttemptStatus::Cancelling.can_transition_to(TaskAttemptStatus::TimedOut));
    // Self-transitions are idempotent
    assert!(TaskAttemptStatus::Gating.can_transition_to(TaskAttemptStatus::Gating));
}

// ─── 8: Failure classification drives retry policy ──────────────────────

#[test]
fn failure_classification() {
    assert_eq!(
        RunnerFailureKind::from_output("connection reset"),
        RunnerFailureKind::Transient
    );
    assert_eq!(
        RunnerFailureKind::from_output("API error: 529"),
        RunnerFailureKind::Transient
    );
    assert_eq!(
        RunnerFailureKind::from_output("rate limit 429"),
        RunnerFailureKind::Transient
    );
    assert_eq!(
        RunnerFailureKind::from_output("out of memory"),
        RunnerFailureKind::Resource
    );
    assert_eq!(
        RunnerFailureKind::from_output("verify script"),
        RunnerFailureKind::Structural
    );
    assert_eq!(
        RunnerFailureKind::from_output("bad code"),
        RunnerFailureKind::Permanent
    );
    assert_eq!(
        RunnerFailureKind::from_output(""),
        RunnerFailureKind::Unknown
    );
    assert!(RunnerFailureKind::Transient.is_retryable());
    assert!(RunnerFailureKind::Structural.is_retryable());
    assert!(!RunnerFailureKind::Resource.is_retryable());
    assert!(!RunnerFailureKind::Permanent.is_retryable());
}

// ─── 9: Diamond DAG skip propagation ────────────────────────────────────

#[test]
fn diamond_dag_skip() {
    let (ta, tb, tc, td_) = (
        td("A", &[]),
        td("B", &["A"]),
        td("C", &["A"]),
        td("D", &["B", "C"]),
    );
    let tasks: Vec<&TaskDef> = vec![&ta, &tb, &tc, &td_];
    let mut dag = TaskDag::new(DagConfig::default());
    dag.mark_running("p1", "A");

    let mut skipped = dag.mark_failed_blocking_downstream("p1", "A", &tasks);
    skipped.sort();
    assert_eq!(skipped, vec!["B", "C", "D"]);
    assert!(dag.plan("p1").unwrap().skipped.contains_key("D"));
    assert!(dag.ready_tasks("p1", &tasks, &[], &[]).is_empty());
    let failed_set = dag.plan("p1").unwrap().failed.clone();
    assert_eq!(
        dag.progress_summary("p1", &tasks, &[], &failed_set, &[], &[])
            .terminal,
        4
    );
}

// ─── 10: Event serialization round-trip ─────────────────────────────────

#[test]
fn event_round_trip() {
    let ar = aref("p1", "t", 1);
    let events: Vec<RunnerEvent> = vec![
        RunnerEvent::run_started(RUN, vec!["p1".into()], 3, false, None),
        RunnerEvent::plan_started(RUN, "p1"),
        RunnerEvent::task_attempt_started(RUN, ar.clone(), "t"),
        RunnerEvent::task_attempt_completed(
            RUN,
            ar.clone(),
            TaskAttemptOutcome::Passed,
            None,
            1000,
            "s",
            "c",
        ),
        RunnerEvent::timeout_recorded(
            RUN,
            TimeoutEvent {
                kind: TimeoutKind::TaskAttempt,
                attempt: Some(ar.clone()),
                effect: None,
                owner_effect: None,
                limit_ms: 60_000,
                monotonic_elapsed_ms: 65_000,
                observed_at_ms: 0,
            },
        ),
        RunnerEvent::retry_decision(
            RUN,
            ar.clone(),
            RetryDecision::for_failure(RunnerFailureKind::Transient, 1, 2, "gate"),
        ),
        RunnerEvent::plan_completed(RUN, "p1", PlanOutcome::Failed, None, 0.05, 1, 1),
        RunnerEvent::run_completed(
            RUN,
            RunOutcome::Failed,
            RunTotals {
                total_tasks: 3,
                tasks_completed: 1,
                tasks_failed: 1,
                tasks_blocked: 0,
                tasks_skipped: 1,
                tasks_active: 0,
                tasks_pending: 0,
                total_agent_calls: 2,
                total_cost_usd: 0.05,
                duration_ms: 3000,
            },
            vec![],
        ),
    ];
    for ev in &events {
        let json = serde_json::to_string(ev).unwrap();
        let rt: RunnerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev.event_type(), rt.event_type(), "round-trip: {json}");
    }
}
