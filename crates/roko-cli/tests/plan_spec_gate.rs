#![cfg(unix)]
//! The plan-load spec gate (3231): before its first task starts, a Graph run
//! records one `spec.quality` and one `spec.gate` line per task in its run
//! directory; with `[spec_quality] mode = "off"` it records none; and a
//! verify step that can never fail refuses the plan before any dispatch.

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::ScriptedPlanWorkspace;
use common::scripted_provider::{Script, Turn};

const PLAN: &str = "spec-gate";

/// Two tasks; T2's verify step is `VERIFY_T2`.
const TASKS: &str = r#"[meta]
plan = "spec-gate"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Write one.txt"
description = "Write one.txt holding the line 1."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["one.txt"]
timeout_secs = 120
max_retries = 0
verify = [{ phase = "structural", command = "test -f one.txt" }]

[[task]]
id = "T2"
title = "Write two.txt"
description = "Write two.txt holding the line 2."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["two.txt"]
depends_on = ["T1"]
timeout_secs = 120
max_retries = 0
verify = [{ phase = "structural", command = "VERIFY_T2" }]
"#;

fn log(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Every JSON line of every run's `name` file under `repo`.
fn run_lines(repo: &Path, name: &str) -> Vec<serde_json::Value> {
    let runs = fs::read_dir(repo.join(".roko/runs")).into_iter().flatten();
    runs.flatten()
        .filter_map(|run| fs::read_to_string(run.path().join(name)).ok())
        .flat_map(|text| {
            let lines: Vec<serde_json::Value> = text
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();
            lines
        })
        .collect()
}

/// When the first attempt of any run started, in Unix ms.
fn first_attempt_started(repo: &Path) -> Option<i64> {
    run_lines(repo, "attempts.jsonl")
        .iter()
        .filter_map(|line| line["attempt_started_at"].as_i64())
        .min()
}

/// The task ids of the records of kind `ev`, in file order.
fn task_ids(records: &[serde_json::Value], ev: &str) -> Vec<String> {
    records
        .iter()
        .filter(|record| record["ev"] == ev)
        .map(|record| record["task_id"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// 3231: a run records one spec.quality and one spec.gate line per task
/// before its first attempt; `mode = "off"` records none; under enforce, a
/// `|| true` verify step is refused before any attempt.
#[test]
fn plan_run_records_spec_quality_and_gate_per_task() {
    let script = Script::new()
        .task("T1", [Turn::reply().write("one.txt", "1\n")])
        .task("T2", [Turn::reply().write("two.txt", "2\n")]);
    let tasks = TASKS.replace("VERIFY_T2", "test -f two.txt");

    // No holdout draw (3232), so every decision here is the gate's own.
    let advise_config = "spec_quality.holdout_frac = 0.0\n";
    let (advise, _provider) =
        ScriptedPlanWorkspace::with_provider(PLAN, &tasks, &script, advise_config);
    let run = advise.run_plan(PLAN, &[]);
    assert!(run.status.success(), "{}", log(&run));
    let records = run_lines(&advise.repo, "spec.jsonl");
    assert_eq!(task_ids(&records, "spec.quality"), ["T1", "T2"], "{records:?}");
    assert_eq!(task_ids(&records, "spec.gate"), ["T1", "T2"], "{records:?}");
    for gate in records.iter().filter(|record| record["ev"] == "spec.gate") {
        assert_eq!(gate["mode"], "advise", "{gate}");
        assert_eq!(gate["holdout"], false, "{gate}");
        assert_ne!(gate["decision"], "block", "{gate}");
    }
    let first = first_attempt_started(&advise.repo).expect("the plan's tasks ran");
    for record in &records {
        let at = record["recorded_at_ms"].as_i64().expect("recorded_at_ms");
        assert!(at <= first, "{record} after the first attempt at {first}");
    }

    let off_config = "spec_quality.mode = \"off\"\n";
    let (off, _provider) = ScriptedPlanWorkspace::with_provider(PLAN, &tasks, &script, off_config);
    let run = off.run_plan(PLAN, &[]);
    assert!(run.status.success(), "{}", log(&run));
    assert!(run_lines(&off.repo, "spec.jsonl").is_empty());

    let vacuous = TASKS.replace("VERIFY_T2", "test -f two.txt || true");
    let enforce_config = "spec_quality.mode = \"enforce\"\n";
    let (enforce, _provider) =
        ScriptedPlanWorkspace::with_provider(PLAN, &vacuous, &script, enforce_config);
    let run = enforce.run_plan(PLAN, &[]);
    assert!(!run.status.success(), "{}", log(&run));
    assert!(log(&run).contains("HF2"), "{}", log(&run));
    assert_eq!(first_attempt_started(&enforce.repo), None, "{}", log(&run));
}
