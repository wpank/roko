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
//! - `loop_census_routed_task_logs_fallback_decision` runs the same plan and
//!   checks its route decisions: one per attempt, and routed T4's labelled a
//!   fallback, since a guard replaced the cascade router's pick. Every
//!   attempt's prompt items are in the run's exposure log too.

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use assert_cmd::cargo::cargo_bin;
use roko_cli::graph_execution::plan_runner::build_graph_feedback_context;
use roko_cli::graph_task_dispatch::{GraphTaskDispatcher, WiringReport};
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::model_call_feedback::ModelCallJournal;
use roko_learn::routing_log::RoutingDecisionLog;
use roko_learn::telemetry::DecisionSource;
use roko_learn::telemetry::report::{RunRecords, route_report};
use serde_json::Value;

/// Every learning component of S01 §5.8, in census order. The legacy
/// holdout split is not one: it gated nothing, and S03's registry lists it
/// (L-holdout) as retired (4101).
const S01_COMPONENTS: &[&str] = &[
    "sink.episode",
    "sink.routing",
    "sink.knowledge_ingestion",
    "sink.playbook_outcome",
    "sink.error_pattern",
    "sink.section_effect",
    "store.attempt_log",
    "store.prompt_experiment",
    "store.decision_writer",
    "store.exposure_writer",
    "store.record_access",
    "reader.gate_thresholds",
];

/// The components nothing on the Graph path provides yet: the S02 backlog.
/// It only shrinks. The census fails when one of them is wired, so take it
/// off the list then.
const EXPECTED_MISSING: &[&str] = &["sink.section_effect"];

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
/// it too. `census-disabled` runs on `census-off`, which `[routing]
/// disabled_providers` lists: the cascade router may pick it, but no task
/// runs on it. The ladder is off, so a task without a model hint is the
/// cascade router's to route.
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

[providers.census-off]
kind = "claude_cli"
command = {provider:?}

[models.census-disabled]
provider = "census-off"
slug = "census-disabled-model"
context_window = 200000
cost_input_per_m = 50.0
cost_output_per_m = 50.0

[routing]
disabled_providers = ["census-off"]

[routing.ladder]
enabled = false

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
/// with one retry; T3 has no verify step; T4 has no model hint, so the
/// cascade router routes it. T4 runs first, before any attempt of the run
/// writes knowledge that could weigh into its pick.
const LOOP_CENSUS_TASKS: &str = r#"[meta]
plan = "loop-census"
max_parallel = 1
skip_enrichment = true
# T3 ends unverified on purpose; without this, plan run refuses the plan (PLAN_037).
allow_unverified = true

[[task]]
id = "T1"
title = "Passing task"
description = "Its verify step passes."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "census-model"
files = ["t1.txt"]
depends_on = ["T4"]
verify = [{ phase = "structural", command = "test -d ." }]
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
depends_on = ["T4"]
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
depends_on = ["T4"]
timeout_secs = 60
max_retries = 0

[[task]]
id = "T4"
title = "Routed task"
description = "It has no model hint: the cascade router routes it."
role = "implementer"
status = "ready"
tier = "focused"
files = ["t4.txt"]
verify = [{ phase = "structural", command = "test -d ." }]
timeout_secs = 60
max_retries = 0
"#;

/// The dispatcher a Graph plan run builds, with the production feedback
/// wiring for `workdir` and `cascade_router` (journaled, as a run journals
/// it), as the census sees it.
async fn production_census(
    workdir: &Path,
    config: &roko_core::config::schema::RokoConfig,
    cascade_router: Option<&Arc<CascadeRouter>>,
) -> WiringReport {
    let shared_config = Arc::new(config.clone());
    let factory = roko_cli::dispatch::SharedAgentFactory::new(
        Arc::clone(&shared_config),
        None,
        cascade_router.cloned(),
        None,
    )
    .await;
    let journal = cascade_router.map(|_| {
        Arc::new(ModelCallJournal::for_learn_dir(
            &workdir.join(".roko/learn"),
        ))
    });
    let feedback = build_graph_feedback_context(
        workdir,
        config,
        cascade_router,
        journal.as_ref(),
        factory.error_pattern_store(),
    );
    GraphTaskDispatcher::new(Arc::new(factory), shared_config, workdir.to_path_buf())
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

/// The cascade router's starting state for the loop-census run: its static
/// stage picks `census-disabled-model` for implementers, a model on a
/// disabled provider, so a guard must replace routed T4's pick. The model
/// list is the workspace's, as a plan run loads it (sorted slugs).
const SEEDED_ROUTER: &str = r#"{
    "model_slugs": ["census-disabled-model", "claude-opus-4-1", "claude-sonnet-4-6"],
    "role_table": {"implementer": "census-disabled-model"},
    "confidence_stats": {}
}"#;

/// Run the loop-census plan through the real `roko` binary in a fresh
/// workspace whose cascade router starts from [`SEEDED_ROUTER`]. Returns the
/// workspace, the run's directory under `.roko/runs`, and the run's output.
fn run_loop_census() -> (tempfile::TempDir, PathBuf, String) {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path();
    write_workspace(workdir);
    let learn = workdir.join(".roko/learn");
    fs::create_dir_all(&learn).expect("create .roko/learn");
    fs::write(learn.join("cascade-router.json"), SEEDED_ROUTER).expect("seed the router");
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

    let mut run_dirs: Vec<PathBuf> = fs::read_dir(workdir.join(".roko/runs"))
        .expect("the run wrote .roko/runs")
        .map(|entry| entry.expect("run directory").path())
        .collect();
    assert_eq!(run_dirs.len(), 1, "{run_dirs:?}\n{log}");
    let run_dir = run_dirs.remove(0);
    (temp, run_dir, log)
}

#[test]
fn loop_census_fixture_settles_one_record_per_attempt() {
    let (temp, run_dir, _log) = run_loop_census();
    let roko = temp.path().join(".roko");
    let lines = jsonl(&run_dir.join("attempts.jsonl"));
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
    // model has no routing trials (S01 SC3). The router learns only from
    // its own picks (decision 4111): T1 and T2 pin their model with a hint,
    // and a guard replaced routed T4's pick, so they train it no more.
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
    for model in ["claude-sonnet-4-6", "census-unverified", "claude-opus-4-1"] {
        assert_eq!(
            trials(model),
            0,
            "{model} gained routing trials: {router:#}"
        );
    }
}

/// S01 §7.1 and §7.4: every attempt of the loop-census run leaves one route
/// decision. T1–T3 pin their models; T4 is routed, and since the cascade
/// router's pick runs on a disabled provider, a guard falls back to the
/// default and the row says so, which `route-report` counts as an honest
/// fallback rather than a masked route.
#[test]
fn loop_census_routed_task_logs_fallback_decision() {
    let (_temp, run_dir, log) = run_loop_census();
    let run = RunRecords::load(&run_dir).expect("load the run");
    assert!(run.invalid.is_empty(), "{:?}", run.invalid);
    assert_eq!(
        run.decisions.len(),
        5,
        "one route decision per attempt\n{log}"
    );
    // Every attempt logs what its prompt retrieved (S01 P0-9). The fixture's
    // workspace has no knowledge store, so its rows are prompt sections.
    let exposed: BTreeSet<&str> = run
        .exposures
        .iter()
        .map(|line| line.record.identity.attempt_key.as_str())
        .collect();
    assert_eq!(exposed.len(), 5, "{log}");
    let decisions_of = |task: &str| -> Vec<&RoutingDecisionLog> {
        run.decisions
            .iter()
            .map(|line| &line.record)
            .filter(|row| row.task_id == task)
            .collect()
    };
    for row in run.decisions.iter().map(|line| &line.record) {
        let total: f64 = row.candidates.iter().filter_map(|c| c.p).sum();
        assert!((total - 1.0).abs() < 1e-9, "{row:?}");
    }
    for task in ["T1", "T2", "T3"] {
        for row in decisions_of(task) {
            assert_eq!(row.source, Some(DecisionSource::TaskHint), "{row:?}");
        }
    }
    let t4 = decisions_of("T4");
    assert_eq!(t4.len(), 1, "{t4:?}");
    let t4 = t4[0];
    assert_eq!(t4.source, Some(DecisionSource::Fallback), "{t4:?}");
    assert_eq!(t4.fallback_reason.as_deref(), Some("provider_disabled"));
    assert_eq!(
        t4.proposals.learned.as_deref(),
        Some("census-disabled-model")
    );
    assert_eq!(t4.selected_model, roko_core::defaults::MODEL_FOCUSED);
    let pick = t4
        .candidates
        .iter()
        .find(|candidate| candidate.model == "census-disabled-model")
        .expect("the router's pick is a candidate");
    assert!(!pick.eligible, "{pick:?}");
    assert_eq!(pick.ineligible_reason.as_deref(), Some("provider_disabled"));

    let report = route_report(std::slice::from_ref(&run), None);
    let row = |source: &str| {
        report
            .rows
            .iter()
            .find(|row| row.source == source)
            .unwrap_or_else(|| panic!("no {source} row: {report:?}"))
    };
    let fallback = row("fallback");
    assert_eq!(fallback.attempts.len(), 1);
    assert_eq!(
        fallback.masked,
        Some(Vec::new()),
        "a fallback is not masked"
    );
    let pinned = row("task_hint");
    assert_eq!(pinned.attempts.len(), 4);
    assert_eq!(pinned.unlabeled.len(), 1, "T3 has no label");
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.masked.as_ref().is_none_or(Vec::is_empty)),
        "{report:?}"
    );
}
