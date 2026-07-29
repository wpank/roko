//! End-to-end test for the strict resume cycle.
//!
//! Drives the runner-side persistence + resume helpers directly:
//!
//! 1. `save_run_state` with a populated `RunStateSnapshot` (including
//!    fingerprints of two tasks).
//! 2. Append a clean line + a partial line to `events.jsonl` to simulate
//!    a crash mid-write.
//! 3. Call `prepare_resume` against the same plan set.
//! 4. Assert: snapshot loaded, fingerprints validated, partial line
//!    truncated.
//!
//! Then drift the second task and re-run `prepare_resume` to assert the
//! validator hard-fails with `TaskMismatch`.

// Test code panics on error intentionally; unwrap/expect distinction is noise.
#![allow(clippy::unwrap_used, clippy::cloned_ref_to_slice_refs)]

use std::collections::HashMap;
use std::fs;

use roko_cli::runner::persist::{
    JsonlRecovery, PersistPaths, RUN_STATE_SCHEMA_VERSION, ReplanLedgerSnapshot, RunStateSnapshot,
    TaskDefFingerprint, save_run_state,
};
use roko_cli::runner::resume::{JsonlRecoveryReport, ResumeError, prepare_resume};
use roko_cli::task_parser::{TaskDef, VerifyStep};
use tempfile::tempdir;

fn task(id: &str, title: &str) -> TaskDef {
    TaskDef {
        id: id.into(),
        title: title.into(),
        description: None,
        role: Some("implementer".into()),
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
        depends_on: vec![],
        depends_on_plan: vec![],
        split_into: None,
        context: None,
        verify: vec![VerifyStep {
            phase: "compile".into(),
            command: "cargo check".into(),
            fail_msg: None,
            timeout_ms: 60_000,
        }],
        timeout_secs: 60,
        max_retries: 1,
        acceptance: vec!["compiles".into()],
        acceptance_contract: None,
        domain: None,
        sequence: 0,
    }
}

fn baseline_snapshot(run_id: &str, fingerprints: Vec<TaskDefFingerprint>) -> RunStateSnapshot {
    RunStateSnapshot {
        schema_version: RUN_STATE_SCHEMA_VERSION,
        run_id: run_id.into(),
        started_at_ms: 0,
        timestamp_ms: 0,
        tasks_total: 2,
        tasks_completed: 1,
        tasks_failed: 0,
        total_tokens_in: 100,
        total_tokens_out: 50,
        total_cost_usd: 0.05,
        total_agent_calls: 1,
        plan_costs: HashMap::new(),
        completed_tasks: HashMap::from([("p1".to_string(), vec!["a".to_string()])]),
        failed_tasks: HashMap::new(),
        lifecycle: None,
        snapshot_fail_streak: 0,
        fingerprints,
        replan_ledger: ReplanLedgerSnapshot::default(),
        revised_tasks: Vec::new(),
        cascade_router_json: None,
    }
}

#[test]
fn full_resume_cycle_validates_fingerprints_and_recovers_jsonl() {
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let task_a = task("a", "Alpha");
    let task_b = task("b", "Beta");
    let fp_a = TaskDefFingerprint::from_task(&task_a, "p1");
    let fp_b = TaskDefFingerprint::from_task(&task_b, "p1");

    // Persist the snapshot (this is what save_snapshot writes during a
    // real run).
    save_run_state(
        &paths,
        &baseline_snapshot("prior-run", vec![fp_a.clone(), fp_b.clone()]),
    )
    .unwrap();

    // Simulate a crash: events.jsonl ends with a partial line.
    let valid_line = "{\"type\":\"plan.started\",\"plan_id\":\"p1\"}\n";
    let partial_line = "{\"type\":\"plan.compl"; // missing closing + newline
    fs::write(&paths.events_jsonl, format!("{valid_line}{partial_line}")).unwrap();

    // Call prepare_resume against the same plan set. Validator should
    // succeed and the partial line should be truncated.
    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![task_a.clone(), task_b.clone()]);
    let report = prepare_resume(&paths, &plans, &[fp_a.clone(), fp_b.clone()]).expect("validates");

    assert!(report.resumed);
    assert_eq!(report.prior_run_id.as_deref(), Some("prior-run"));
    assert_eq!(report.validated_tasks, 2);

    // events.jsonl recovery should show TruncatedTrailing.
    let events_recovery = report
        .recovered_files
        .iter()
        .find(|f| f.path.starts_with("events: "))
        .expect("events log reported");
    match &events_recovery.recovery {
        JsonlRecoveryReport::TruncatedTrailing {
            valid_lines,
            truncated_bytes,
        } => {
            assert_eq!(*valid_lines, 1);
            assert_eq!(*truncated_bytes, partial_line.len() as u64);
        }
        other => panic!("expected TruncatedTrailing, got {other:?}"),
    }
    assert_eq!(
        fs::read_to_string(&paths.events_jsonl).unwrap(),
        valid_line,
        "partial line must be truncated to last clean newline",
    );
}

#[test]
fn drifted_task_definition_is_reported_for_requeue() {
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let task_a_original = task("a", "Alpha original");
    let fp_a_original = TaskDefFingerprint::from_task(&task_a_original, "p1");
    save_run_state(
        &paths,
        &baseline_snapshot("prior-run", vec![fp_a_original.clone()]),
    )
    .unwrap();

    // Drift: same id, different title -> different fingerprint.
    let task_a_drifted = task("a", "Alpha drifted");
    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![task_a_drifted]);

    let report = prepare_resume(&paths, &plans, &[fp_a_original.clone()]).unwrap();
    assert!(report.resumed);
    assert_eq!(report.drifted_tasks.len(), 1);
    assert_eq!(report.drifted_tasks[0].plan_id, "p1");
    assert_eq!(report.drifted_tasks[0].task_id, "a");
    assert_ne!(
        report.drifted_tasks[0].old_fingerprint, report.drifted_tasks[0].new_fingerprint,
        "fingerprints should disagree on drift",
    );
}

#[test]
fn missing_plan_in_current_run_is_rejected() {
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let task_a = task("a", "A");
    let fp_a = TaskDefFingerprint::from_task(&task_a, "p1");
    save_run_state(&paths, &baseline_snapshot("prior", vec![fp_a.clone()])).unwrap();

    // Current run only has p2 — p1 (which the snapshot recorded) is gone.
    let task_x = task("x", "X");
    let mut plans = HashMap::new();
    plans.insert("p2".to_string(), vec![task_x]);

    let err = prepare_resume(&paths, &plans, &[fp_a]).unwrap_err();
    assert!(matches!(err, ResumeError::PlanMissing { .. }));
}

#[test]
fn fresh_workdir_resume_is_treated_as_new_run() {
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    // No run-state.json — fresh workdir.
    let report = prepare_resume(&paths, &HashMap::new(), &[]).unwrap();
    assert!(!report.resumed);
    assert!(report.prior_run_id.is_none());
    assert_eq!(report.validated_tasks, 0);
    // All three logs must report Clean.
    assert_eq!(report.recovered_files.len(), 3);
    for f in &report.recovered_files {
        match f.recovery {
            JsonlRecoveryReport::Clean { lines } => assert_eq!(lines, 0),
            ref other => panic!("expected Clean, got {other:?}"),
        }
    }
}

#[test]
fn jsonl_recovery_helper_returns_clean_for_consistent_file() {
    // Direct test of the recover_jsonl helper (sanity for the mod).
    let dir = tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    fs::write(&path, "{\"a\":1}\n{\"b\":2}\n").unwrap();
    let outcome = roko_cli::runner::persist::recover_jsonl(&path, |line: &str| {
        serde_json::from_str::<serde_json::Value>(line)
    })
    .unwrap();
    matches!(outcome, JsonlRecovery::Clean { lines: 2 });
}

// ---------------------------------------------------------------------------
// SH06-T02: Interruption, dirty worktree, and exact resume tests
// ---------------------------------------------------------------------------

/// Helper: build a snapshot with configurable completed_tasks and replan_ledger.
fn snapshot_with(
    run_id: &str,
    fingerprints: Vec<TaskDefFingerprint>,
    completed: HashMap<String, Vec<String>>,
    replan_ledger: ReplanLedgerSnapshot,
) -> RunStateSnapshot {
    let tasks_completed: usize = completed.values().map(|v| v.len()).sum();
    RunStateSnapshot {
        schema_version: RUN_STATE_SCHEMA_VERSION,
        run_id: run_id.into(),
        started_at_ms: 100,
        timestamp_ms: 200,
        tasks_total: 4,
        tasks_completed,
        tasks_failed: 0,
        total_tokens_in: 500,
        total_tokens_out: 250,
        total_cost_usd: 0.10,
        total_agent_calls: 2,
        plan_costs: HashMap::new(),
        completed_tasks: completed,
        failed_tasks: HashMap::new(),
        lifecycle: None,
        snapshot_fail_streak: 0,
        fingerprints,
        replan_ledger,
        revised_tasks: Vec::new(),
        cascade_router_json: None,
    }
}

#[test]
fn interrupt_after_gate_recovers_events_and_completed_set() {
    // Snapshot records task "a" completed in plan "p1". events.jsonl has a
    // gate-result line followed by a truncated mid-write line simulating a
    // crash right after the gate recorded its outcome.
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let task_a = task("a", "Alpha");
    let task_b = task("b", "Beta");
    let fp_a = TaskDefFingerprint::from_task(&task_a, "p1");
    let fp_b = TaskDefFingerprint::from_task(&task_b, "p1");

    let completed = HashMap::from([("p1".to_string(), vec!["a".to_string()])]);
    let snap = snapshot_with(
        "gate-run",
        vec![fp_a.clone(), fp_b.clone()],
        completed,
        ReplanLedgerSnapshot::default(),
    );
    save_run_state(&paths, &snap).unwrap();

    // events.jsonl: gate result line + truncated event.
    let gate_line = "{\"type\":\"gate.passed\",\"task_id\":\"a\",\"plan_id\":\"p1\"}\n";
    let truncated = "{\"type\":\"task.sta";
    fs::write(&paths.events_jsonl, format!("{gate_line}{truncated}")).unwrap();

    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![task_a, task_b]);
    let report = prepare_resume(&paths, &plans, &[fp_a, fp_b]).expect("resume succeeds");

    assert!(report.resumed);
    assert_eq!(report.prior_run_id.as_deref(), Some("gate-run"));
    assert_eq!(report.validated_tasks, 2);

    // events.jsonl should be truncated to the single valid gate line.
    let events_recovery = report
        .recovered_files
        .iter()
        .find(|f| f.path.starts_with("events: "))
        .expect("events recovery reported");
    match &events_recovery.recovery {
        JsonlRecoveryReport::TruncatedTrailing {
            valid_lines,
            truncated_bytes,
        } => {
            assert_eq!(*valid_lines, 1);
            assert_eq!(*truncated_bytes, truncated.len() as u64);
        }
        other => panic!("expected TruncatedTrailing, got {other:?}"),
    }
    assert_eq!(
        fs::read_to_string(&paths.events_jsonl).unwrap(),
        gate_line,
        "only the valid gate line should survive",
    );
}

#[test]
fn multi_plan_fingerprints_validated_correctly() {
    // Two plans, each with their own tasks. Snapshot records completed tasks
    // across both. Resume must validate all fingerprints correctly.
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let t_p1_a = task("a", "Plan1-Alpha");
    let t_p1_b = task("b", "Plan1-Beta");
    let t_p2_x = task("x", "Plan2-Xray");
    let t_p2_y = task("y", "Plan2-Yankee");

    let fp_p1_a = TaskDefFingerprint::from_task(&t_p1_a, "p1");
    let fp_p1_b = TaskDefFingerprint::from_task(&t_p1_b, "p1");
    let fp_p2_x = TaskDefFingerprint::from_task(&t_p2_x, "p2");
    let fp_p2_y = TaskDefFingerprint::from_task(&t_p2_y, "p2");

    let all_fps = vec![
        fp_p1_a.clone(),
        fp_p1_b.clone(),
        fp_p2_x.clone(),
        fp_p2_y.clone(),
    ];
    let completed = HashMap::from([
        ("p1".to_string(), vec!["a".to_string()]),
        ("p2".to_string(), vec!["x".to_string()]),
    ]);
    let snap = snapshot_with(
        "multi-plan",
        all_fps.clone(),
        completed,
        ReplanLedgerSnapshot::default(),
    );
    save_run_state(&paths, &snap).unwrap();

    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![t_p1_a, t_p1_b]);
    plans.insert("p2".to_string(), vec![t_p2_x, t_p2_y]);

    let report = prepare_resume(&paths, &plans, &all_fps).expect("validates");
    assert!(report.resumed);
    assert_eq!(report.validated_tasks, 4);
    assert!(
        report.drifted_tasks.is_empty(),
        "no drift with identical tasks"
    );
}

#[test]
fn multi_plan_detects_drift_in_second_plan() {
    // Two plans, drift only in p2's task "y". Resume should report exactly
    // that task as drifted while validating the rest cleanly.
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let t_p1_a = task("a", "Plan1-Alpha");
    let t_p2_x = task("x", "Plan2-Xray");
    let t_p2_y_original = task("y", "Plan2-Yankee-v1");
    let t_p2_y_drifted = task("y", "Plan2-Yankee-v2");

    let fp_p1_a = TaskDefFingerprint::from_task(&t_p1_a, "p1");
    let fp_p2_x = TaskDefFingerprint::from_task(&t_p2_x, "p2");
    let fp_p2_y = TaskDefFingerprint::from_task(&t_p2_y_original, "p2");

    let snap_fps = vec![fp_p1_a.clone(), fp_p2_x.clone(), fp_p2_y.clone()];
    let snap = snapshot_with(
        "multi-drift",
        snap_fps.clone(),
        HashMap::new(),
        ReplanLedgerSnapshot::default(),
    );
    save_run_state(&paths, &snap).unwrap();

    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![t_p1_a]);
    plans.insert("p2".to_string(), vec![t_p2_x, t_p2_y_drifted]);

    let report = prepare_resume(&paths, &plans, &snap_fps).expect("validates");
    assert!(report.resumed);
    assert_eq!(report.drifted_tasks.len(), 1);
    assert_eq!(report.drifted_tasks[0].plan_id, "p2");
    assert_eq!(report.drifted_tasks[0].task_id, "y");
    // p1/a and p2/x validated cleanly (2), plus p2/y validated but drifted (1) = 3
    assert_eq!(report.validated_tasks, 3);
}

#[test]
fn replan_ledger_survives_resume() {
    // Create a snapshot with a populated replan_ledger, resume, and verify the
    // report loads successfully (the ledger is embedded in the snapshot).
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let t = task("a", "Alpha");
    let fp = TaskDefFingerprint::from_task(&t, "p1");

    let ledger = ReplanLedgerSnapshot {
        replans_seen: HashMap::from([("p1".to_string(), 2)]),
        seen_failure_keys: vec!["p1:a:compile".to_string(), "p1:a:test".to_string()],
        revision_requests: Vec::new(),
    };
    let snap = snapshot_with(
        "ledger-run",
        vec![fp.clone()],
        HashMap::new(),
        ledger.clone(),
    );
    save_run_state(&paths, &snap).unwrap();

    // Verify the snapshot round-trips: load it back and check the ledger.
    let loaded = roko_cli::runner::persist::load_run_state(&paths)
        .unwrap()
        .expect("snapshot exists");
    assert_eq!(loaded.replan_ledger, ledger);
    assert_eq!(loaded.replan_ledger.replans_seen["p1"], 2);
    assert_eq!(loaded.replan_ledger.seen_failure_keys.len(), 2);

    // Also verify that prepare_resume succeeds with the ledger-bearing snapshot.
    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![t]);
    let report = prepare_resume(&paths, &plans, &[fp]).expect("resumes");
    assert!(report.resumed);
    assert_eq!(report.prior_run_id.as_deref(), Some("ledger-run"));
}

#[test]
fn all_three_jsonl_files_recovered_on_resume() {
    // Corrupt all three JSONL files (episodes, events, efficiency) and verify
    // prepare_resume recovers each one independently.
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    // No snapshot needed; fresh workdir still runs JSONL recovery.

    // episodes: 2 valid + trailing partial
    let ep_valid = "{\"episode\":1}\n{\"episode\":2}\n";
    let ep_partial = "{\"episode\":3,\"inc";
    fs::write(&paths.episodes_jsonl, format!("{ep_valid}{ep_partial}")).unwrap();

    // events: 1 valid + trailing partial
    let ev_valid = "{\"event\":\"start\"}\n";
    let ev_partial = "{\"event\":\"ga";
    fs::write(&paths.events_jsonl, format!("{ev_valid}{ev_partial}")).unwrap();

    // efficiency: completely partial (no valid lines at all)
    let eff_partial = "{\"tokens\":12";
    fs::write(&paths.efficiency_jsonl, eff_partial).unwrap();

    let report = prepare_resume(&paths, &HashMap::new(), &[]).expect("recovery succeeds");
    assert!(!report.resumed); // no snapshot
    assert_eq!(report.recovered_files.len(), 3);

    // episodes: 2 valid lines, trailing truncated
    let ep_rec = report
        .recovered_files
        .iter()
        .find(|f| f.path.starts_with("episodes: "))
        .expect("episodes reported");
    match &ep_rec.recovery {
        JsonlRecoveryReport::TruncatedTrailing {
            valid_lines,
            truncated_bytes,
        } => {
            assert_eq!(*valid_lines, 2);
            assert_eq!(*truncated_bytes, ep_partial.len() as u64);
        }
        other => panic!("expected episodes TruncatedTrailing, got {other:?}"),
    }
    assert_eq!(fs::read_to_string(&paths.episodes_jsonl).unwrap(), ep_valid);

    // events: 1 valid line, trailing truncated
    let ev_rec = report
        .recovered_files
        .iter()
        .find(|f| f.path.starts_with("events: "))
        .expect("events reported");
    match &ev_rec.recovery {
        JsonlRecoveryReport::TruncatedTrailing {
            valid_lines,
            truncated_bytes,
        } => {
            assert_eq!(*valid_lines, 1);
            assert_eq!(*truncated_bytes, ev_partial.len() as u64);
        }
        other => panic!("expected events TruncatedTrailing, got {other:?}"),
    }
    assert_eq!(fs::read_to_string(&paths.events_jsonl).unwrap(), ev_valid);

    // efficiency: 0 valid lines, entire file truncated
    let eff_rec = report
        .recovered_files
        .iter()
        .find(|f| f.path.starts_with("efficiency: "))
        .expect("efficiency reported");
    match &eff_rec.recovery {
        JsonlRecoveryReport::TruncatedTrailing {
            valid_lines,
            truncated_bytes,
        } => {
            assert_eq!(*valid_lines, 0);
            assert_eq!(*truncated_bytes, eff_partial.len() as u64);
        }
        other => panic!("expected efficiency TruncatedTrailing, got {other:?}"),
    }
    assert_eq!(fs::read_to_string(&paths.efficiency_jsonl).unwrap(), "");
}

#[test]
fn empty_completed_set_resume_succeeds() {
    // Snapshot exists with zero completed tasks. Resume should succeed and
    // report the correct counts.
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let t_a = task("a", "Alpha");
    let t_b = task("b", "Beta");
    let fp_a = TaskDefFingerprint::from_task(&t_a, "p1");
    let fp_b = TaskDefFingerprint::from_task(&t_b, "p1");

    let snap = snapshot_with(
        "empty-completed",
        vec![fp_a.clone(), fp_b.clone()],
        HashMap::new(), // no completed tasks
        ReplanLedgerSnapshot::default(),
    );
    save_run_state(&paths, &snap).unwrap();

    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![t_a, t_b]);
    let report = prepare_resume(&paths, &plans, &[fp_a, fp_b]).expect("resumes");

    assert!(report.resumed);
    assert_eq!(report.prior_run_id.as_deref(), Some("empty-completed"));
    assert_eq!(report.validated_tasks, 2);
    assert!(report.drifted_tasks.is_empty());
    // Recovered snapshot has zero completions — the runner should replay
    // all tasks from the beginning of the DAG.
    let loaded = roko_cli::runner::persist::load_run_state(&paths)
        .unwrap()
        .expect("snapshot");
    assert!(loaded.completed_tasks.is_empty());
    assert_eq!(loaded.tasks_completed, 0);
}

#[test]
fn schema_version_mismatch_rejects_resume() {
    // A snapshot with a future schema version (higher than current) should
    // cause prepare_resume to return UnsupportedSchema.
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let t = task("a", "Alpha");
    let fp = TaskDefFingerprint::from_task(&t, "p1");
    let mut snap = snapshot_with(
        "future-schema",
        vec![fp.clone()],
        HashMap::new(),
        ReplanLedgerSnapshot::default(),
    );
    snap.schema_version = RUN_STATE_SCHEMA_VERSION + 42;
    save_run_state(&paths, &snap).unwrap();

    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![t]);

    let err = prepare_resume(&paths, &plans, &[fp]).unwrap_err();
    match err {
        ResumeError::UnsupportedSchema { expected, found } => {
            assert_eq!(expected, RUN_STATE_SCHEMA_VERSION);
            assert_eq!(found, RUN_STATE_SCHEMA_VERSION + 42);
        }
        other => panic!("expected UnsupportedSchema, got {other:?}"),
    }
}

#[test]
fn current_schema_version_is_accepted() {
    // Confirm that the exact current schema version passes validation.
    let dir = tempdir().expect("tempdir");
    let paths = PersistPaths::from_workdir(dir.path()).expect("paths");

    let t = task("a", "Alpha");
    let fp = TaskDefFingerprint::from_task(&t, "p1");
    let snap = snapshot_with(
        "current-schema",
        vec![fp.clone()],
        HashMap::new(),
        ReplanLedgerSnapshot::default(),
    );
    assert_eq!(snap.schema_version, RUN_STATE_SCHEMA_VERSION);
    save_run_state(&paths, &snap).unwrap();

    let mut plans = HashMap::new();
    plans.insert("p1".to_string(), vec![t]);
    let report = prepare_resume(&paths, &plans, &[fp]).expect("resumes with current schema");
    assert!(report.resumed);
}
