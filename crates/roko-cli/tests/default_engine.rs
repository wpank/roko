//! Regression coverage for the bare `plan run` engine default.

mod common;

use std::fs;

use common::{SAMPLE_PLAN_ID, run_roko_isolated, setup_sample_plan_workspace};
use tempfile::tempdir;

/// A bare `roko plan run` executes its plan on the Graph engine and leaves
/// what readers of the workspace need (bug-230de6): the run's dashboard
/// events in `.roko/events.jsonl` and in its per-run index, its episodes and
/// its Graph checkpoint.
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
    let lines: Vec<serde_json::Value> = events
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let task_event = |line: &serde_json::Value| {
        matches!(
            line["type"].as_str(),
            Some("task_started" | "task_completed")
        ) && line["plan_id"] == SAMPLE_PLAN_ID
            && line["task_id"] == "T1"
    };
    assert!(
        lines.iter().any(task_event),
        "bare default plan run recorded no event for its task; events: {events}"
    );
    assert!(
        lines.iter().any(|line| line["type"] == "gate_result"),
        "bare default plan run recorded no gate result; events: {events}"
    );

    // Every line names the run, and the run's index holds the same lines.
    let run_id = lines
        .iter()
        .find_map(|line| line["run_id"].as_str())
        .unwrap_or_else(|| panic!("no event names its run; events: {events}"));
    assert!(
        lines.iter().all(|line| line["run_id"] == run_id),
        "{events}"
    );
    let index_path = roko_fs::run_index::run_index_path(&events_path, run_id)
        .unwrap_or_else(|err| panic!("index path for {run_id}: {err}"));
    let index = fs::read_to_string(&index_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", index_path.display()));
    assert_eq!(index.lines().count(), events.lines().count(), "{index}");

    let episodes_path = workdir.join(".roko/episodes.jsonl");
    let episodes = fs::read_to_string(&episodes_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", episodes_path.display()));
    assert!(
        !episodes.trim().is_empty(),
        "bare default plan run did not persist an episode"
    );

    let checkpoint_path = workdir
        .join(".roko/state/graph")
        .join(SAMPLE_PLAN_ID)
        .join("checkpoint.json");
    assert!(
        checkpoint_path.is_file(),
        "bare default plan run wrote no Graph checkpoint at {}",
        checkpoint_path.display()
    );
}
