#![cfg(unix)]

//! Learning wiring census (S01 §4.8, P0-11 and P0-12): the exit check of
//! epic spec-b7303f.
//!
//! - `graph_dispatcher_production_wiring_census` builds the dispatcher with
//!   the production feedback wiring (`build_graph_feedback_context`, the
//!   code `roko plan run` uses) and checks every learning component S01 §5.8
//!   lists: each is wired or named in [`EXPECTED_MISSING`]. A component that
//!   goes missing fails, and so does one on the list that is wired now.
//! - `loop_census_fixture_settles_one_record_per_attempt` runs the four-task
//!   loop-census plan through the real `roko` binary with a scripted provider
//!   and checks, from the files alone, that every attempt settles exactly
//!   once and that every learning row joins an attempt.

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::sync::Arc;

use assert_cmd::cargo::cargo_bin;
use roko_cli::graph_execution::plan_runner::build_graph_feedback_context;
use roko_cli::graph_task_dispatch::{GraphTaskDispatcher, WiringReport};
use roko_learn::cascade_router::CascadeRouter;
use serde_json::Value;

/// Every learning component of S01 §5.8, in census order.
const S01_COMPONENTS: &[&str] = &[
    "sink.episode",
    "sink.routing",
    "sink.knowledge_ingestion",
    "sink.playbook_outcome",
    "sink.error_pattern",
    "sink.section_effect",
    "store.attempt_log",
    "store.prompt_experiment",
    "store.holdout",
    "store.decision_writer",
    "store.exposure_writer",
    "store.record_access",
    "reader.gate_thresholds",
];

/// The components nothing on the Graph path provides yet: the S02 backlog.
/// It only shrinks. The census fails when one of them is wired, so take it
/// off the list then.
const EXPECTED_MISSING: &[&str] = &[
    "sink.error_pattern",
    "sink.section_effect",
    "store.decision_writer",
    "store.exposure_writer",
    "store.record_access",
];

/// The stand-in `claude_cli` provider: it ignores its prompt and reports a
/// finished free turn, so each task's verify step alone decides its outcome.
const PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"census","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":3,"output_tokens":2},"is_error":false}'
"#;

/// A workspace whose models all run on [`PROVIDER`]: `census-model` for the
/// verified tasks, and `census-unverified` for T3 alone, so T3's routing
/// statistics are its own. `census-model` is the cheaper one, so the helper
/// calls after a failed verify step (which take the cheapest model) run on
/// it too.
fn write_workspace(workdir: &Path) {
    let provider = workdir.join("fake-provider.sh");
    fs::write(&provider, PROVIDER).expect("write provider script");
    fs::set_permissions(&provider, fs::Permissions::from_mode(0o755))
        .expect("make provider executable");
    fs::write(
        workdir.join("roko.toml"),
        format!(
            r#"
[agent]
default_model = "census-model"
command = {provider:?}
bare_mode = false

[providers.census-cli]
kind = "claude_cli"
command = {provider:?}

[models.census-model]
provider = "census-cli"
slug = "claude-sonnet-4-6"
context_window = 200000
cost_input_per_m = 0.1
cost_output_per_m = 0.1

[models.census-unverified]
provider = "census-cli"
slug = "claude-opus-4-1"
context_window = 200000
cost_input_per_m = 50.0
cost_output_per_m = 50.0

[gates]
sibling_settle_secs = 0
"#,
            provider = provider.display().to_string()
        ),
    )
    .expect("write roko.toml");
    fs::write(workdir.join("README.md"), "# loop census\n").expect("write README");
}

/// The loop-census plan (S01 P0-12): T1 passes its verify step; T2 fails it
/// with one retry; T3 has no verify step; T4 pins a model no provider
/// serves.
const LOOP_CENSUS_TASKS: &str = r#"[meta]
plan = "loop-census"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Passing task"
description = "Its verify step passes."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "census-model"
files = ["t1.txt"]
verify = [{ phase = "structural", command = "true" }]
timeout_secs = 60
max_retries = 0

[[task]]
id = "T2"
title = "Failing task"
description = "Its verify step fails on both attempts."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "census-model"
files = ["t2.txt"]
verify = [{ phase = "structural", command = "false", fail_msg = "T2 fails" }]
timeout_secs = 60
max_retries = 1

[[task]]
id = "T3"
title = "Unverified task"
description = "It has no verify step."
role = "scribe"
status = "ready"
tier = "focused"
model_hint = "census-unverified"
files = ["t3.txt"]
timeout_secs = 60
max_retries = 0

[[task]]
id = "T4"
title = "Unconfigured model"
description = "It pins a model no provider serves."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "census-unconfigured-model"
files = ["t4.txt"]
verify = [{ phase = "structural", command = "true" }]
timeout_secs = 60
max_retries = 0
"#;

/// The dispatcher a Graph plan run builds, with the production feedback
/// wiring for `workdir` and `cascade_router`, as the census sees it.
async fn production_census(
    workdir: &Path,
    config: &roko_core::config::schema::RokoConfig,
    cascade_router: Option<&Arc<CascadeRouter>>,
) -> WiringReport {
    let feedback = build_graph_feedback_context(workdir, config, cascade_router);
    let config = Arc::new(config.clone());
    let factory = roko_cli::dispatch::SharedAgentFactory::new(
        Arc::clone(&config),
        None,
        cascade_router.cloned(),
        None,
    )
    .await;
    GraphTaskDispatcher::new(Arc::new(factory), config, workdir.to_path_buf())
        .with_feedback(feedback)
        .wiring_report()
}

/// Why the census fails: a component missing without being expected to, or
/// one expected to be missing that is wired now.
fn census_failures(report: &WiringReport) -> Vec<String> {
    let mut failures = Vec::new();
    for component in &report.components {
        match (component.wired, EXPECTED_MISSING.contains(&component.id)) {
            (false, false) => failures.push(format!(
                "{} is not wired: {}",
                component.id, component.detail
            )),
            (true, true) => failures.push(format!(
                "{} is wired now: take it off EXPECTED_MISSING",
                component.id
            )),
            _ => {}
        }
    }
    failures
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graph_dispatcher_production_wiring_census() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path();
    write_workspace(workdir);
    let config = roko_core::config::loader::load_config_validated(workdir)
        .expect("load the workspace config")
        .into_config();
    let run_config = roko_cli::runner::RunConfig::from_roko_config(
        workdir.to_path_buf(),
        workdir.join("plans"),
        config.clone(),
    );
    let cascade_router = run_config.cascade_router.clone();
    assert!(cascade_router.is_some(), "a plan run has a cascade router");

    let report = production_census(workdir, &config, cascade_router.as_ref()).await;
    for component in &report.components {
        let state = if component.wired { "wired" } else { "MISSING" };
        eprintln!("{:<26} {state:<8} {}", component.id, component.detail);
    }
    eprintln!("EXPECTED_MISSING: {}", EXPECTED_MISSING.join(", "));
    let ids: Vec<&str> = report.components.iter().map(|c| c.id).collect();
    assert_eq!(
        ids, S01_COMPONENTS,
        "the census names every S01 §5.8 component"
    );
    assert_eq!(census_failures(&report), Vec::<String>::new());
    assert_eq!(report.missing(), EXPECTED_MISSING);

    // A sink that drops off the facade fails the census: without a router,
    // the routing sink is never registered.
    let without_router = production_census(workdir, &config, None).await;
    assert!(!without_router.facade_sinks.contains(&"routing"));
    let failures = census_failures(&without_router);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(
        failures[0].starts_with("sink.routing is not wired"),
        "{failures:?}"
    );
}

/// Assert that `file` has rows and that each names, at the JSON pointer
/// `key_at`, one of the `settled` attempts.
fn assert_rows_join(rows: &[Value], key_at: &str, settled: &BTreeSet<String>, file: &str) {
    assert!(!rows.is_empty(), "{file} has no rows");
    for row in rows {
        let key = row.pointer(key_at).and_then(Value::as_str);
        assert!(
            key.is_some_and(|key| settled.contains(key)),
            "{file} row joins no attempt: {row}"
        );
    }
}

/// The rows of the JSONL file at `path` (none when it does not exist).
fn jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("a JSON row"))
        .collect()
}

#[test]
fn loop_census_fixture_settles_one_record_per_attempt() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path();
    write_workspace(workdir);
    let plan_dir = workdir.join("plans/loop-census");
    fs::create_dir_all(&plan_dir).expect("create plan directory");
    fs::write(plan_dir.join("tasks.toml"), LOOP_CENSUS_TASKS).expect("write tasks.toml");

    let output = std::process::Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .args(["--json", "plan", "run", "plans/loop-census", "--workdir"])
        .arg(workdir)
        .env("HOME", workdir)
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("ROKO_CONFIG")
        .output()
        .expect("run roko plan run");
    let log = format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "T2 fails, so the plan does: {log}"
    );

    let roko = workdir.join(".roko");
    let run_dirs: Vec<_> = fs::read_dir(roko.join("runs"))
        .expect("the run wrote .roko/runs")
        .map(|entry| entry.expect("run directory").path())
        .collect();
    assert_eq!(run_dirs.len(), 1, "{run_dirs:?}\n{log}");
    let lines = jsonl(&run_dirs[0].join("attempts.jsonl"));
    let schema = |line: &Value| {
        line["schema_version"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    };
    let opens: Vec<&Value> = lines
        .iter()
        .filter(|line| schema(line) == "roko.attempt_open/1")
        .collect();
    let verdicts: Vec<&Value> = lines
        .iter()
        .filter(|line| schema(line) == "roko.verdict/1")
        .collect();
    assert_eq!(opens.len(), 5, "T2 retries once: {lines:#?}");
    assert_eq!(verdicts.len(), 5, "every attempt settles: {lines:#?}");
    let key = |line: &Value| line["attempt_key"].as_str().unwrap_or_default().to_string();
    let opened: BTreeSet<String> = opens.iter().map(|line| key(line)).collect();
    let settled: BTreeSet<String> = verdicts.iter().map(|line| key(line)).collect();
    assert_eq!(opened, settled, "no attempt is abandoned or settled twice");
    assert_eq!(settled.len(), 5);

    for verdict in &verdicts {
        assert!(verdict["outcome"].is_string(), "{verdict}");
        assert!(
            verdict["executed"]["model_requested"].is_string(),
            "{verdict}"
        );
        assert!(verdict["cost"]["source"].is_string(), "{verdict}");
    }
    let of_task = |task: &str| -> Vec<&Value> {
        verdicts
            .iter()
            .copied()
            .filter(|verdict| verdict["task_id"] == task)
            .collect()
    };
    let t1 = of_task("T1");
    assert_eq!((t1.len(), t1[0]["outcome"].as_str()), (1, Some("passed")));
    assert_eq!(t1[0]["learning_label"], 1);
    let t2 = of_task("T2");
    let t2_attempts: Vec<u64> = t2.iter().filter_map(|v| v["attempt"].as_u64()).collect();
    assert_eq!(t2_attempts, [1, 2]);
    for verdict in &t2 {
        assert_eq!(verdict["outcome"], "gate_failed");
        assert_eq!(verdict["learning_label"], 0);
    }
    let t3 = of_task("T3");
    assert_eq!(
        (t3.len(), t3[0]["outcome"].as_str()),
        (1, Some("unverified"))
    );
    assert!(
        t3[0]["learning_label"].is_null(),
        "an unverified attempt has no label"
    );
    assert_eq!(of_task("T4").len(), 1);

    // Every learning row the run wrote names one of its settled attempts.
    let efficiency: Vec<Value> = jsonl(&roko.join("learn/efficiency.jsonl"))
        .into_iter()
        .filter(|row| row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA)
        .collect();
    assert_rows_join(
        &efficiency,
        "/attempt_key",
        &settled,
        "learn/efficiency.jsonl",
    );
    let costs = jsonl(&roko.join("learn/costs.jsonl"));
    assert_rows_join(&costs, "/attempt_key", &settled, "learn/costs.jsonl");
    let episodes = jsonl(&roko.join("episodes.jsonl"));
    assert_rows_join(&episodes, "/extra/attempt_key", &settled, "episodes.jsonl");

    // T3 carries no label, so the router learns nothing from it: its
    // model has no routing trials (S01 SC3), while T1 and T2 train theirs.
    let router: Value = serde_json::from_str(
        &fs::read_to_string(roko.join("learn/cascade-router.json"))
            .expect("the run saved the router"),
    )
    .expect("router state is JSON");
    let trials = |model: &str| {
        router["confidence_stats"][model]["trials"]
            .as_u64()
            .unwrap_or(0)
    };
    assert!(
        trials("claude-sonnet-4-6") > 0,
        "T1 and T2 train the router: {router:#}"
    );
    for model in ["census-unverified", "claude-opus-4-1"] {
        assert_eq!(
            trials(model),
            0,
            "T3's model {model} gained routing trials: {router:#}"
        );
    }
}
