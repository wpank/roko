//! Regression coverage for the bare `plan run` engine default.

mod common;

use std::fs;

use common::{run_roko_isolated, setup_sample_plan_workspace};
use tempfile::tempdir;

#[test]
fn default_engine_does_real_work() {
    let temp = tempdir().expect("tempdir");
    let workdir = temp.path();
    setup_sample_plan_workspace(workdir);
    let tasks_path = workdir.join("plans/test-wire-xyz/tasks.toml");
    let tasks = fs::read_to_string(&tasks_path).expect("read tasks.toml");
    fs::write(
        &tasks_path,
        tasks.replace("max_retries = 1", "max_retries = 0"),
    )
    .expect("disable retries for regression fixture");

    // Intentionally omit `--engine`: this test protects the CLI default.
    let _run = run_roko_isolated(workdir, &["plan", "run", "plans"]);

    let events_path = workdir.join(".roko/events.jsonl");
    let events = fs::read_to_string(&events_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", events_path.display()));
    assert!(
        events.lines().any(|line| line.contains("task.attempt")),
        "bare default plan run did not execute a task; events: {events}"
    );

    let ledger_path = workdir.join(".roko/state/run-ledger.jsonl");
    let ledger = fs::read_to_string(&ledger_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", ledger_path.display()));
    assert!(
        ledger.lines().any(|line| line.contains("gate_outcome")),
        "bare default plan run did not run gates; ledger: {ledger}"
    );
}
