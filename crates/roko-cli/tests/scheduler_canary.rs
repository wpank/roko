#![cfg(unix)]

//! Canary C6 (assessment W8, gate G6): one Graph plan, run through the built
//! `roko` binary, gets every scheduler fix at once.
//!
//! - A task that does not depend on a failed task still runs, even when it
//!   becomes ready after the failure, and the failed task's dependants are
//!   skipped.
//! - Tasks whose `files` overlap never run at the same time; tasks whose
//!   `files` are disjoint do.
//!
//! The plan omits `max_parallel`, so it runs as wide as its DAG allows. The
//! shared scripted provider plays each task's turn: a second of work (two for
//! T4), logged as `start <id>` and `end <id>`, and the marker each task's
//! verify step checks, never for T1.
//!
//! `scheduler_canary` runs the plan with `--no-budget`;
//! `scheduler_canary_under_a_plan_budget` runs it under a plan budget with
//! no `max_turn_usd`, the configuration a fresh workspace has (backlog 3103).

mod common;

use assert_cmd::cargo::cargo_bin;
use common::scripted_provider::{Script, ScriptedProvider, Turn};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

/// Each task's turn: T1 leaves no marker, T4 works for two seconds, and each
/// costs $0.01, so a plan budget settles its spend.
fn script() -> Script {
    ["T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8"]
        .into_iter()
        .fold(Script::new(), |script, id| {
            let work = Turn::reply()
                .silent_for(if id == "T4" { 2.0 } else { 1.0 })
                .usage(10, 5, 0.01);
            let turn = if id == "T1" {
                work
            } else {
                work.write(&format!("{id}.done"), "")
            };
            script.task(id, [turn])
        })
}

/// One `[[task]]` block. Its verify step passes once the fake agent has left
/// `<id>.done`.
fn task(id: &str, depends_on: &[&str], files: &[&str]) -> String {
    let list = |items: &[&str]| {
        items
            .iter()
            .map(|item| format!("{item:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        r#"
[[task]]
id = "{id}"
title = "Scheduler canary {id}"
description = "Scheduler canary task {id}."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "graph-model"
files = [{files}]
allowed_tools = []
denied_tools = []
mcp_servers = []
depends_on = [{depends_on}]
depends_on_plan = []
acceptance = []
verify = [{{ phase = "structural", command = "test -f {id}.done", fail_msg = "{id} left no marker" }}]
timeout_secs = 60
max_retries = 0
"#,
        files = list(files),
        depends_on = list(depends_on)
    )
}

/// `provider`, a `roko.toml` in `workdir` that routes every task to it and
/// ends with `extra_config`, and the plan:
///
/// - T1 fails its verify step; T2 depends on T1.
/// - T3 depends on T4, a slower root, so T3 becomes ready after T1 failed.
/// - T5 and T6 are roots that write the same file; T7 and T8 are roots that
///   write different files.
fn setup_workspace(workdir: &Path, provider: &ScriptedProvider, extra_config: &str) {
    let provider = provider.command();
    fs::write(
        workdir.join("roko.toml"),
        format!(
            r#"
[agent]
default_model = "graph-model"
command = {provider:?}
bare_mode = false

[providers.graph-cli]
kind = "claude_cli"
command = {provider:?}

[models.graph-model]
provider = "graph-cli"
slug = "claude-sonnet-4-6"
context_window = 200000

[gates]
sibling_settle_secs = 0
{extra_config}"#,
            provider = provider.display().to_string()
        ),
    )
    .expect("write roko config");

    let plan_dir = workdir.join("plans/canary");
    fs::create_dir_all(&plan_dir).expect("create plan directory");
    fs::write(plan_dir.join("plan.md"), "# Plan: canary\n\nCanary C6.\n").expect("plan.md");
    let tasks = [
        task("T1", &[], &["t1.txt"]),
        task("T2", &["T1"], &["t2.txt"]),
        task("T3", &["T4"], &["t3.txt"]),
        task("T4", &[], &["t4.txt"]),
        task("T5", &[], &["shared.txt"]),
        task("T6", &[], &["shared.txt"]),
        task("T7", &[], &["t7.txt"]),
        task("T8", &[], &["t8.txt"]),
    ]
    .concat();
    fs::write(
        plan_dir.join("tasks.toml"),
        format!(
            "[meta]\nplan = \"canary\"\niteration = 1\ntotal = 8\ndone = 0\nstatus = \"ready\"\n\
             estimated_total_minutes = 1\nskip_enrichment = true\n{tasks}"
        ),
    )
    .expect("write plan tasks");
}

/// `roko <args> --workdir <workdir>` through the built binary, as a user
/// whose home is `home` beside `workdir`: provider keys, `ROKO_*` variables
/// and the invoking environment's log and config variables are removed.
fn roko(workdir: &Path, args: &[&str]) -> Output {
    let home = workdir.with_file_name("home");
    fs::create_dir_all(&home).expect("create the home directory");
    let mut command = Command::new(cargo_bin("roko"));
    command
        .current_dir(workdir)
        .args(args)
        .arg("--workdir")
        .arg(workdir)
        .env("HOME", &home);
    for name in roko_core::child_env::PROVIDER_KEY_VARS {
        command.env_remove(name);
    }
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("ROKO_") {
            command.env_remove(name);
        }
    }
    for name in ["RUST_LOG", "XDG_CONFIG_HOME", "CLAUDECODE"] {
        command.env_remove(name);
    }
    command.output().expect("run roko")
}

fn json(output: &Output, what: &str) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{what} JSON: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn scheduler_canary() {
    run_canary("", &["--no-budget"]);
}

/// Backlog 3103: the same plan and turns under `[budget] max_plan_usd` with
/// no `max_turn_usd`. Each provider call reserves its share of the budget
/// (3102), so the scheduler's checks hold as they do with budgets off, and
/// the plan's spend settles with nothing left reserved.
#[test]
fn scheduler_canary_under_a_plan_budget() {
    let summary = run_canary("\n[budget]\nmax_plan_usd = 5.0\n", &[]);
    let budget = &summary["plan_budgets"][0];
    assert_eq!(budget["ceiling_usd"], 5.0, "{budget:#}");
    assert_eq!(budget["reserved_usd"], 0.0, "{budget:#}");
    assert!(budget["spent_usd"].as_f64() > Some(0.0), "{budget:#}");
}

/// Run the canary plan with `args` in a workspace whose `roko.toml` ends
/// with `extra_config`, check every scheduler fix, and return the run's JSON
/// summary.
fn run_canary(extra_config: &str, args: &[&str]) -> Value {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = &temp.path().join("work");
    fs::create_dir_all(workdir).expect("create the workdir");
    let provider = ScriptedProvider::install(&temp.path().join("provider"), &script());
    setup_workspace(workdir, &provider, extra_config);

    let started = Instant::now();
    let run_args = [&["--json", "plan", "run", "plans/canary"][..], args].concat();
    let run = roko(workdir, &run_args);
    assert!(
        started.elapsed() < Duration::from_secs(60),
        "the canary takes under a minute"
    );
    assert_eq!(
        run.status.code(),
        Some(1),
        "T1 fails the plan\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let events = provider.events();
    let events: Vec<&str> = events.iter().map(String::as_str).collect();
    let at = |event: &str| {
        events
            .iter()
            .position(|seen| *seen == event)
            .unwrap_or_else(|| panic!("`{event}` never happened: {events:?}"))
    };
    let ambiguous: Vec<Vec<String>> = provider
        .calls()
        .into_iter()
        .filter(|call| call.task_ids.len() > 1)
        .map(|call| call.task_ids)
        .collect();
    assert!(
        ambiguous.is_empty(),
        "calls naming several tasks: {ambiguous:?}"
    );

    // T2 depends on the failed T1 and never starts. T3 does not, becomes
    // ready only after T1 ended, and still runs.
    assert!(!events.contains(&"start T2"), "{events:?}");
    assert!(at("end T1") < at("start T3"), "{events:?}");
    // T5 and T6 write the same file: one runs after the other.
    assert!(
        at("end T5") < at("start T6") || at("end T6") < at("start T5"),
        "T5 and T6 ran together: {events:?}"
    );
    // T7 and T8 write different files: they run together.
    assert!(
        at("start T7") < at("end T8") && at("start T8") < at("end T7"),
        "T7 and T8 ran one after the other: {events:?}"
    );
    // The plan failed only once every other task had settled.
    for id in ["T3", "T4", "T5", "T6", "T7", "T8"] {
        assert!(workdir.join(format!("{id}.done")).exists(), "{id} passed");
    }

    let checkpoint = fs::read(workdir.join(".roko/state/graph/canary/checkpoint.json"))
        .expect("read the Graph checkpoint");
    let checkpoint: Value = serde_json::from_slice(&checkpoint).expect("checkpoint JSON");
    assert_eq!(checkpoint["status"], "failed", "{checkpoint:#}");
    let outcome = &checkpoint["extensions"]["roko.task.outcome@1"]["value"];
    assert_eq!(outcome["failed"], serde_json::json!(["T1"]), "{outcome:#}");
    assert_eq!(
        outcome["blocked_by"],
        serde_json::json!({ "T2": "T1" }),
        "{outcome:#}"
    );
    assert!(
        outcome["not_started"]
            .as_object()
            .is_none_or(serde_json::Map::is_empty),
        "{outcome:#}"
    );

    let status = json(
        &roko(workdir, &["--json", "plan", "status", "plans/canary"]),
        "plan status",
    );
    assert_eq!(status["status"], "failed", "plan status: {status:#}");
    json(&run, "plan run")
}
