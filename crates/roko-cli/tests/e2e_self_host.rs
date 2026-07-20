//! SH06-T05: Deterministic e2e self-host smoke test.
//!
//! Proves the full state-machine pipeline without external dependencies:
//! fresh execution, gate failure + retry, terminal snapshot, dashboard
//! events, persistence round-trip, and resume equivalence.
//! No network, no child processes, no LLM calls.

#![allow(clippy::unwrap_used, clippy::too_many_lines)]

use std::collections::HashMap;
use std::fs;

use roko_cli::runner::persist::{
    PersistPaths, RUN_STATE_SCHEMA_VERSION, ReplanLedgerSnapshot, RunStateSnapshot,
    TaskDefFingerprint, load_run_state, save_run_state,
};
use roko_cli::runner::projection::{Projection, RawRuntimeEvent};
use roko_cli::runner::resume::prepare_resume;
use roko_cli::runner::state::RunState;
use roko_cli::runner::task_dag::{DagConfig, TaskDag};
use roko_cli::runner::types::*;
use roko_cli::task_parser::TaskDef;
use tempfile::tempdir;

const RUN: &str = "sh-smoke";

fn td(id: &str, deps: &[&str]) -> TaskDef {
    TaskDef {
        id: id.into(),
        title: format!("Task {id}"),
        description: None,
        role: Some("impl".into()),
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
fn ar(p: &str, t: &str, n: u32) -> TaskAttemptRef {
    TaskAttemptRef::new(p, t, n)
}
fn gc(p: &str, t: &str, ok: bool, n: u32) -> GateCompletion {
    GateCompletion {
        effect: None,
        attempt: Some(ar(p, t, n)),
        kind: GateCompletionKind::Gate,
        plan_id: p.into(),
        task_id: t.into(),
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
fn fresh(total: usize) -> RunState {
    let mut s = RunState::new(total);
    s.apply_runner_event(&RunnerEvent::run_started(
        RUN,
        vec!["p1".into()],
        total,
        false,
        None,
    ));
    s.apply_runner_event(&RunnerEvent::plan_started(RUN, "p1"));
    s
}
/// Apply event to both state and projection.
fn emit(s: &mut RunState, p: &Projection, ev: RunnerEvent) {
    s.apply_runner_event(&ev);
    let _ = p.publish(RawRuntimeEvent::Runner(ev));
}
fn snap(
    s: &RunState,
    fps: Vec<TaskDefFingerprint>,
    done: HashMap<String, Vec<String>>,
    fail: HashMap<String, Vec<String>>,
) -> RunStateSnapshot {
    RunStateSnapshot {
        schema_version: RUN_STATE_SCHEMA_VERSION,
        run_id: RUN.into(),
        started_at_ms: 0,
        timestamp_ms: 100,
        tasks_total: s.tasks_total,
        tasks_completed: s.tasks_completed,
        tasks_failed: s.tasks_failed,
        total_tokens_in: s.total_tokens_in,
        total_tokens_out: s.total_tokens_out,
        total_cost_usd: s.total_cost_usd,
        total_agent_calls: s.total_agent_calls,
        plan_costs: s.plan_costs.clone(),
        completed_tasks: done,
        failed_tasks: fail,
        lifecycle: Some(s.lifecycle.clone()),
        snapshot_fail_streak: 0,
        fingerprints: fps,
        replan_ledger: ReplanLedgerSnapshot::default(),
        revised_tasks: Vec::new(),
        cascade_router_json: None,
    }
}

// ─── 1: Full pipeline — execute, gate-fail/retry, dashboard, persist, resume ─

#[test]
fn self_host_smoke_full_pipeline() {
    let (ta, tb, tc) = (td("A", &[]), td("B", &[]), td("C", &["A", "B"]));
    let tasks: Vec<&TaskDef> = vec![&ta, &tb, &tc];
    let mut st = fresh(3);
    let mut dag = TaskDag::new(DagConfig::default());
    let proj = Projection::new(RUN);
    let _rx = proj.subscribe();

    // Phase 1: dispatch A and B
    dag.mark_running("p1", "A");
    dag.mark_running("p1", "B");
    let (a1, b1) = (ar("p1", "A", 1), ar("p1", "B", 1));
    emit(
        &mut st,
        &proj,
        RunnerEvent::task_attempt_started(RUN, a1.clone(), "A"),
    );
    emit(
        &mut st,
        &proj,
        RunnerEvent::task_attempt_started(RUN, b1.clone(), "B"),
    );

    // A passes
    emit(
        &mut st,
        &proj,
        RunnerEvent::task_attempt_completed(
            RUN,
            a1.clone(),
            TaskAttemptOutcome::Passed,
            None,
            1000,
            "sa",
            "m",
        ),
    );
    dag.mark_complete("p1", "A");
    assert_eq!(
        st.lifecycle.tasks["p1:A"].status,
        TaskLifecycleStatus::Passed
    );

    // B gate-fails, retries, then passes
    emit(
        &mut st,
        &proj,
        RunnerEvent::gate_dispatch_started(RUN, b1.clone(), GateCompletionKind::Gate, 1),
    );
    emit(
        &mut st,
        &proj,
        RunnerEvent::gate_completed(RUN, b1.clone(), &gc("p1", "B", false, 1)),
    );
    assert_eq!(
        st.lifecycle.task_attempts[&b1.key()].status,
        TaskAttemptStatus::GateFailed
    );

    let rd = RetryDecision::for_failure(RunnerFailureKind::Transient, 1, 2, "gate");
    assert!(rd.should_retry());
    emit(
        &mut st,
        &proj,
        RunnerEvent::retry_decision(RUN, b1.clone(), rd),
    );
    assert_eq!(
        st.lifecycle.tasks["p1:B"].status,
        TaskLifecycleStatus::Retrying
    );

    let b2 = ar("p1", "B", 2);
    emit(
        &mut st,
        &proj,
        RunnerEvent::task_attempt_started(RUN, b2.clone(), "B"),
    );
    assert_eq!(
        st.lifecycle.task_attempts[&b1.key()].status,
        TaskAttemptStatus::Superseded
    );
    emit(
        &mut st,
        &proj,
        RunnerEvent::task_attempt_completed(
            RUN,
            b2.clone(),
            TaskAttemptOutcome::Passed,
            None,
            2000,
            "sb",
            "m",
        ),
    );
    dag.mark_complete("p1", "B");
    assert_eq!(
        st.lifecycle.tasks["p1:B"].status,
        TaskLifecycleStatus::Passed
    );

    // C unblocks and passes
    let ready = dag.ready_tasks("p1", &tasks, &["A".into(), "B".into()], &[]);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, "C");
    dag.mark_running("p1", "C");
    let c1 = ar("p1", "C", 1);
    emit(
        &mut st,
        &proj,
        RunnerEvent::task_attempt_started(RUN, c1.clone(), "C"),
    );
    emit(
        &mut st,
        &proj,
        RunnerEvent::task_attempt_completed(
            RUN,
            c1.clone(),
            TaskAttemptOutcome::Passed,
            None,
            3000,
            "sc",
            "m",
        ),
    );
    dag.mark_complete("p1", "C");

    // Terminal events
    let plan = dag.plan("p1").unwrap();
    emit(
        &mut st,
        &proj,
        RunnerEvent::plan_completed(
            RUN,
            "p1",
            PlanOutcome::Succeeded,
            None,
            0.15,
            plan.completed.len(),
            plan.failed.len(),
        ),
    );
    emit(
        &mut st,
        &proj,
        RunnerEvent::run_completed(
            RUN,
            RunOutcome::Succeeded,
            RunTotals {
                total_tasks: 3,
                tasks_completed: 3,
                tasks_failed: 0,
                tasks_blocked: 0,
                tasks_skipped: 0,
                tasks_active: 0,
                tasks_pending: 0,
                total_agent_calls: 4,
                total_cost_usd: 0.15,
                duration_ms: 5000,
            },
            vec![],
        ),
    );

    // Verify terminal state
    assert_eq!(st.lifecycle.status, RunnerRunStatus::Completed);
    for key in ["p1:A", "p1:B", "p1:C"] {
        let t = st.lifecycle.tasks.get(key).unwrap();
        assert_eq!(t.status, TaskLifecycleStatus::Passed, "{key}");
        assert!(t.completed_at_ms.is_some(), "{key} completed_at_ms");
    }
    for (key, att) in &st.lifecycle.task_attempts {
        assert!(
            att.status == TaskAttemptStatus::Passed || att.status == TaskAttemptStatus::Superseded,
            "stale attempt {key}: {:?}",
            att.status,
        );
    }

    // DAG progress
    let done: Vec<String> = plan.completed.iter().cloned().collect();
    let summary = dag.progress_summary("p1", &tasks, &done, &plan.failed, &[], &[]);
    assert_eq!(
        (
            summary.terminal,
            summary.ready,
            summary.active,
            summary.blocked
        ),
        (3, 0, 0, 0)
    );

    // Dashboard events
    let dash = proj.dashboard_snapshot();
    assert!(!dash.events.is_empty());
    let types: Vec<&str> = dash.events.iter().map(|e| e.event_type.as_str()).collect();
    for expected in [
        "task.attempt.started",
        "task.attempt.completed",
        "gate.completed",
        "retry.decision",
        "run.completed",
    ] {
        assert!(types.contains(&expected), "missing {expected}");
    }
    for ev in &dash.events {
        assert_eq!(ev.run_id, RUN);
    }
    assert_eq!(proj.counters().coerced, 0);

    // Persistence round-trip
    let dir = tempdir().unwrap();
    let paths = PersistPaths::from_workdir(dir.path()).unwrap();
    let fps: Vec<_> = [&ta, &tb, &tc]
        .iter()
        .map(|t| TaskDefFingerprint::from_task(t, "p1"))
        .collect();
    let done_map = HashMap::from([("p1".into(), vec!["A".into(), "B".into(), "C".into()])]);
    save_run_state(
        &paths,
        &snap(&st, fps.clone(), done_map.clone(), HashMap::new()),
    )
    .unwrap();

    let ld = load_run_state(&paths).unwrap().unwrap();
    assert_eq!(
        (ld.schema_version, ld.run_id.as_str(), ld.tasks_total),
        (RUN_STATE_SCHEMA_VERSION, RUN, 3)
    );
    assert_eq!(ld.completed_tasks, done_map);
    let lc = ld.lifecycle.unwrap();
    assert_eq!(lc.status, RunnerRunStatus::Completed);
    for (key, task) in &st.lifecycle.tasks {
        assert_eq!(lc.tasks[key].status, task.status, "round-trip {key}");
    }

    // Resume
    let mut plans = HashMap::new();
    plans.insert("p1".into(), vec![ta.clone(), tb.clone(), tc.clone()]);
    let rpt = prepare_resume(&paths, &plans, &fps).unwrap();
    assert!(rpt.resumed);
    assert_eq!(rpt.prior_run_id.as_deref(), Some(RUN));
    assert_eq!(rpt.validated_tasks, 3);
    assert!(rpt.drifted_tasks.is_empty());

    // No orphan attempts
    for key in st.lifecycle.task_attempts.keys() {
        let pt = &key[..key.rfind(':').unwrap()];
        assert!(st.lifecycle.tasks.contains_key(pt), "orphan {key}");
    }
}

// ─── 2: Resume from mid-run snapshot ────────────────────────────────────

#[test]
fn resume_from_mid_run_replays_remaining() {
    let (ta, tb, tc) = (td("A", &[]), td("B", &["A"]), td("C", &["B"]));
    let dir = tempdir().unwrap();
    let paths = PersistPaths::from_workdir(dir.path()).unwrap();
    let fps: Vec<_> = [&ta, &tb, &tc]
        .iter()
        .map(|t| TaskDefFingerprint::from_task(t, "p1"))
        .collect();

    let mid = RunStateSnapshot {
        schema_version: RUN_STATE_SCHEMA_VERSION,
        run_id: RUN.into(),
        started_at_ms: 0,
        timestamp_ms: 50,
        tasks_total: 3,
        tasks_completed: 1,
        tasks_failed: 0,
        total_tokens_in: 200,
        total_tokens_out: 100,
        total_cost_usd: 0.05,
        total_agent_calls: 1,
        plan_costs: HashMap::new(),
        completed_tasks: HashMap::from([("p1".into(), vec!["A".into()])]),
        failed_tasks: HashMap::new(),
        lifecycle: None,
        snapshot_fail_streak: 0,
        fingerprints: fps.clone(),
        replan_ledger: ReplanLedgerSnapshot::default(),
        revised_tasks: Vec::new(),
        cascade_router_json: None,
    };
    save_run_state(&paths, &mid).unwrap();

    // Crash: partial trailing line in events.jsonl
    let valid = "{\"type\":\"task.started\",\"task_id\":\"A\"}\n";
    fs::write(
        &paths.events_jsonl,
        format!("{valid}{{\"type\":\"task.compl"),
    )
    .unwrap();

    let mut plans = HashMap::new();
    plans.insert("p1".into(), vec![ta.clone(), tb.clone(), tc.clone()]);
    let rpt = prepare_resume(&paths, &plans, &fps).unwrap();
    assert!(rpt.resumed);
    assert_eq!(rpt.validated_tasks, 3);
    assert!(rpt.drifted_tasks.is_empty());
    assert_eq!(fs::read_to_string(&paths.events_jsonl).unwrap(), valid);

    let ld = load_run_state(&paths).unwrap().unwrap();
    assert_eq!(ld.tasks_completed, 1);
    assert_eq!(ld.completed_tasks["p1"], vec!["A".to_string()]);

    // DAG: B ready, C blocked on B
    let mut dag = TaskDag::new(DagConfig::default());
    let refs: Vec<&TaskDef> = vec![&ta, &tb, &tc];
    let r1 = dag.ready_tasks("p1", &refs, &["A".into()], &[]);
    assert_eq!((r1.len(), r1[0].id.as_str()), (1, "B"));
    dag.mark_running("p1", "B");
    dag.mark_complete("p1", "B");
    let r2 = dag.ready_tasks("p1", &refs, &["A".into(), "B".into()], &[]);
    assert_eq!((r2.len(), r2[0].id.as_str()), (1, "C"));
}

// ─── 3: Failed task blocks dependents, terminal counts correct ──────────

#[test]
fn failed_task_blocks_dependent_terminal_counts() {
    let (ta, tb, tc) = (td("A", &[]), td("B", &["A"]), td("C", &[]));
    let tasks: Vec<&TaskDef> = vec![&ta, &tb, &tc];
    let mut st = fresh(3);
    let mut dag = TaskDag::new(DagConfig::default());

    dag.mark_running("p1", "A");
    dag.mark_running("p1", "C");
    let (a1, c1) = (ar("p1", "A", 1), ar("p1", "C", 1));
    st.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, a1.clone(), "A"));
    st.apply_runner_event(&RunnerEvent::task_attempt_started(RUN, c1.clone(), "C"));

    // A fails permanently -> B skipped
    st.apply_runner_event(&RunnerEvent::task_attempt_completed(
        RUN,
        a1,
        TaskAttemptOutcome::Failed,
        Some(RunnerFailureKind::Permanent),
        500,
        "s",
        "c",
    ));
    assert_eq!(
        dag.mark_failed_blocking_downstream("p1", "A", &tasks),
        vec!["B".to_string()]
    );

    // C passes
    st.apply_runner_event(&RunnerEvent::task_attempt_completed(
        RUN,
        c1,
        TaskAttemptOutcome::Passed,
        None,
        1000,
        "s",
        "c",
    ));
    dag.mark_complete("p1", "C");

    let plan = dag.plan("p1").unwrap();
    assert!(
        plan.failed.contains("A") && plan.skipped.contains_key("B") && plan.completed.contains("C")
    );
    let done: Vec<String> = plan.completed.iter().cloned().collect();
    let summary = dag.progress_summary("p1", &tasks, &done, &plan.failed, &[], &[]);
    assert_eq!(
        (
            summary.terminal,
            summary.ready,
            summary.active,
            summary.blocked
        ),
        (3, 0, 0, 0)
    );

    // Persistence round-trip captures both completed and failed
    let dir = tempdir().unwrap();
    let paths = PersistPaths::from_workdir(dir.path()).unwrap();
    let fps: Vec<_> = [&ta, &tb, &tc]
        .iter()
        .map(|t| TaskDefFingerprint::from_task(t, "p1"))
        .collect();
    let s = RunStateSnapshot {
        schema_version: RUN_STATE_SCHEMA_VERSION,
        run_id: RUN.into(),
        started_at_ms: 0,
        timestamp_ms: 50,
        tasks_total: 3,
        tasks_completed: 1,
        tasks_failed: 1,
        total_tokens_in: 0,
        total_tokens_out: 0,
        total_cost_usd: 0.0,
        total_agent_calls: 2,
        plan_costs: HashMap::new(),
        completed_tasks: HashMap::from([("p1".into(), vec!["C".into()])]),
        failed_tasks: HashMap::from([("p1".into(), vec!["A".into()])]),
        lifecycle: Some(st.lifecycle.clone()),
        snapshot_fail_streak: 0,
        fingerprints: fps,
        replan_ledger: ReplanLedgerSnapshot::default(),
        revised_tasks: Vec::new(),
        cascade_router_json: None,
    };
    save_run_state(&paths, &s).unwrap();
    let ld = load_run_state(&paths).unwrap().unwrap();
    assert_eq!((ld.tasks_completed, ld.tasks_failed), (1, 1));
    assert!(ld.failed_tasks["p1"].contains(&"A".into()));
    assert!(ld.completed_tasks["p1"].contains(&"C".into()));
}
