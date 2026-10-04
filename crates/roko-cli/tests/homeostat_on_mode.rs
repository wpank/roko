#![cfg(unix)]

//! S06 §7 A4 (backlog 8129): M1 acts on a real run.
//!
//! `param_change_takes_effect_on_next_attempt` runs a [`TASKS`]-task plan
//! through the real `roko` binary, one task at a time, in a workspace with
//! `[homeostasis] mode = "on"`, an S5 policy and a scripted provider. Every
//! task's verify step passes before task [`DROP_AT`] and fails from there on,
//! a synthetic `model_swap` that breaches E1. Then, from the run's files
//! alone:
//! - the `harness_policy` rows split between the learned arm and θ₀'s arms;
//! - every row on a holdout or all-off arm runs θ₀;
//! - the controller applies a change, and a later attempt on the learned arm
//!   runs a θ whose digest differs from θ₀'s;
//! - the controller's code never names the run's ground truth (8119).

use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin;
use roko_learn::loop_audit::assign::takes_default;
use roko_learn::telemetry::Arm;
use roko_learn::telemetry::report::RunRecords;

/// The plan's tasks: `plan run` refuses more than 64.
const TASKS: usize = 64;

/// The first task whose verify step fails.
const DROP_AT: usize = 12;

/// The stand-in `claude_cli` provider: it ignores its prompt and reports a
/// finished free turn, so each task's verify step alone decides its outcome.
const PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"m1","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":3,"output_tokens":2},"is_error":false}'
"#;

/// The S5 policy: S06 §5's bounds, and a 25% holdout, so that every day's
/// draws give the run's chains both arms (one with none has odds below
/// 10⁻⁸).
const POLICY: &str = r#"
policy_version = 1
holdout = 0.25
ev.pass_rate = { lo = 0.70, inner = 0.75 }
ev.usd_per_verified_success = { hi = 0.12, inner = 0.108, abs_cap = 0.50 }
ev.false_green = { hi = 0.10 }
ev.latency_p90_s = { hi = 900, inner = 810 }
"#;

/// A workspace whose one model runs on [`PROVIDER`], with M1 on, the model
/// ladder off and [`POLICY`] as its S5 policy.
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
default_model = "m1-model"
command = {provider:?}
bare_mode = false

[providers.m1-cli]
kind = "claude_cli"
command = {provider:?}

[models.m1-model]
provider = "m1-cli"
slug = "claude-sonnet-4-6"
context_window = 200000
cost_input_per_m = 0.1
cost_output_per_m = 0.1
supports_tools = false

[routing.ladder]
enabled = false

[homeostasis]
mode = "on"

[gates]
sibling_settle_secs = 0

[spec_quality]
mode = "off"
red_on_base = false
"#,
            provider = provider.display().to_string()
        ),
    )
    .expect("write roko.toml");
    fs::write(workdir.join("README.md"), "# M1 on-mode run\n").expect("write README");
    let policy = workdir.join(".roko/policy");
    fs::create_dir_all(&policy).expect("create the policy directory");
    fs::write(policy.join("viability.toml"), POLICY).expect("write the S5 policy");
}

/// The plan: [`TASKS`] independent tasks, run one at a time, past failures,
/// each with one attempt whose verify step passes before [`DROP_AT`].
fn tasks_toml() -> String {
    let mut toml = String::from(
        "[meta]\nplan = \"m1-on\"\nmax_parallel = 1\nskip_enrichment = true\n\
         failure_policy = \"skip_failed\"\n",
    );
    for task in 0..TASKS {
        let check = if task < DROP_AT { "true" } else { "false" };
        let _ = write!(
            toml,
            r#"
[[task]]
id = "M{task}"
title = "Run step {task} of the M1 fixture"
description = "Its verify step decides it."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "m1-model"
files = ["m{task}.txt"]
verify = [{{ phase = "structural", command = "{check}" }}]
timeout_secs = 60
max_retries = 0
"#
        );
    }
    toml
}

/// The rows of the workspace's controller ledger.
fn controller_rows(workdir: &Path) -> Vec<serde_json::Value> {
    let ledger = workdir.join(".roko/learn/controller.jsonl");
    fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[test]
fn param_change_takes_effect_on_next_attempt() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path();
    write_workspace(workdir);
    let plan_dir = workdir.join("plans/m1-on");
    fs::create_dir_all(&plan_dir).expect("create plan directory");
    fs::write(plan_dir.join("tasks.toml"), tasks_toml()).expect("write tasks.toml");

    // The run fails, since its later tasks fail: only its files are read.
    let output = std::process::Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .args(["--json", "plan", "run", "plans/m1-on", "--workdir"])
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
    let run_dirs: Vec<PathBuf> = fs::read_dir(workdir.join(".roko/runs"))
        .unwrap_or_else(|error| panic!("the run wrote .roko/runs: {error}\n{log}"))
        .map(|entry| entry.expect("run directory").path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(run_dirs.len(), 1, "one run: {run_dirs:?}\n{log}");
    let run = RunRecords::load(&run_dirs[0]).expect("read the run's records");
    assert!(run.invalid.is_empty(), "{:?}", run.invalid);

    // One harness_policy row per attempt, split between the arms.
    let rows: Vec<_> = run
        .harness_decisions
        .iter()
        .map(|line| &line.record)
        .collect();
    assert_eq!(rows.len(), TASKS, "{log}");
    let theta0 = rows[0].default.params_digest();
    let held: Vec<_> = rows.iter().filter(|row| takes_default(row.arm)).collect();
    let learned = rows.iter().filter(|row| row.arm == Arm::Learned).count();
    assert!(!held.is_empty(), "no chain drew θ₀'s arms");
    assert!(
        learned >= TASKS / 4,
        "{learned} of {TASKS} chains on the learned arm"
    );
    // Every holdout and all-off row runs θ₀.
    for row in &held {
        assert_eq!(row.params_digest, theta0, "{:?}", row.identity.task_id);
    }

    // M1 applies a change, and a later learned attempt runs the new θ.
    let ledger = controller_rows(workdir);
    let applied = ledger
        .iter()
        .find(|row| row["kind"] == "param.change" && row["applied"] == true);
    assert!(applied.is_some(), "M1 applied no change: {ledger:?}\n{log}");
    let moved = rows
        .iter()
        .filter(|row| row.arm == Arm::Learned && row.params_digest != theta0)
        .count();
    assert!(moved > 0, "no attempt ran the changed θ");
    for row in rows.iter().filter(|row| row.params_digest != theta0) {
        assert!(row.policy_version > 0, "{row:?}");
    }

    // The controller never names the run's ground truth.
    let sources = [
        include_str!("../src/runtime_feedback/homeostasis.rs"),
        include_str!("../../roko-learn/src/homeostasis/controller.rs"),
    ];
    for source in sources {
        assert!(!source.contains("disturbances.jsonl"));
    }
}
