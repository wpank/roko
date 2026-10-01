#![cfg(unix)]

//! Canary C7 (W8 gate G7), the exit check of epic spec-edda86, through the
//! built `roko` binary with a scripted provider:
//!
//! - a silent agent is cancelled by the stall watchdog (spec-a0403b) and
//!   retried, long before its task's `timeout_secs`, and its process is gone;
//! - a run refuses to start while the workdir has less free disk than
//!   `[resources] min_free_disk_mb` (reg-7cf6f9), before any dispatch.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;

/// A `claude_cli` provider that reports one message and then goes silent:
/// it logs its pid, prints an assistant message, and becomes `sleep 300`
/// (same pid), longer than its task's `timeout_secs`.
const SILENT_PROVIDER: &str = r#"#!/bin/sh
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cat >/dev/null
printf '%s\n' "$$" >> "$dir/provider-pids"
printf '%s\n' '{"type":"assistant","message":{"id":"msg-1","content":[{"type":"text","text":"reading the task"}]}}'
exec sleep 300
"#;

/// One task on the silent provider. Without the watchdog its two attempts
/// would each run until the 120 s `timeout_secs`.
const SILENT_TASKS: &str = r#"[meta]
plan = "silent"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "T1 has a silent agent"
description = "Its agent reports one event and then says nothing."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "silent-model"
files = ["t1.txt"]
verify = [{ phase = "structural", command = "true" }]
timeout_secs = 120
max_retries = 1
"#;

/// The longest the silent run may take: its attempts stall after 2 s each,
/// far below the 2 × 120 s their timeout allows.
const SILENT_RUN_LIMIT: Duration = Duration::from_secs(60);

/// A workspace whose one model runs on [`SILENT_PROVIDER`], with `extra`
/// appended to its `roko.toml`, and the plan `tasks` under `plans/<plan>`.
fn write_workspace(workdir: &Path, extra: &str, plan: &str, tasks: &str) {
    let provider = workdir.join("silent-provider.sh");
    fs::write(&provider, SILENT_PROVIDER).expect("write provider script");
    fs::set_permissions(&provider, fs::Permissions::from_mode(0o755))
        .expect("make provider executable");
    fs::write(
        workdir.join("roko.toml"),
        format!(
            r#"
[agent]
default_model = "silent-model"
command = {provider:?}
bare_mode = false

[providers.silent-cli]
kind = "claude_cli"
command = {provider:?}

[models.silent-model]
provider = "silent-cli"
slug = "claude-sonnet-4-6"
context_window = 200000

[gates]
sibling_settle_secs = 0
{extra}"#,
            provider = provider.display().to_string(),
        ),
    )
    .expect("write roko.toml");
    let plan_dir = workdir.join("plans").join(plan);
    fs::create_dir_all(&plan_dir).expect("create plan directory");
    fs::write(plan_dir.join("tasks.toml"), tasks).expect("write tasks.toml");
}

/// `roko plan run` of `plans/<plan>` with `args`, through the built binary:
/// whether it succeeded, how long it took, and its output.
fn run_plan(workdir: &Path, plan: &str, args: &[&str]) -> (bool, Duration, String) {
    let started = Instant::now();
    let output = std::process::Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .args(["--json", "plan", "run"])
        .arg(format!("plans/{plan}"))
        .arg("--workdir")
        .arg(workdir)
        .args(args)
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
    (output.status.success(), started.elapsed(), log)
}

/// The pids the provider logged, one per call.
fn provider_pids(workdir: &Path) -> Vec<String> {
    fs::read_to_string(workdir.join("provider-pids"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// Whether process `pid` still exists.
fn alive(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The rows of the JSONL file at `path` (none when it does not exist).
fn jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[test]
fn supervision_canary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path();
    write_workspace(
        workdir,
        "\n[conductor]\ntask_stall_secs = 2\n",
        "silent",
        SILENT_TASKS,
    );
    let log_file = workdir.join("run-log.jsonl");
    let log_arg = log_file.display().to_string();

    let (succeeded, took, log) = run_plan(workdir, "silent", &["--log-file", &log_arg]);
    assert!(!succeeded, "the silent task fails, so the plan does: {log}");
    assert!(
        took < SILENT_RUN_LIMIT,
        "the watchdog must end the run long before 2 x 120 s; it took {took:?}: {log}"
    );

    // Both attempts ran, and each provider process is gone.
    let pids = provider_pids(workdir);
    assert_eq!(
        pids.len(),
        2,
        "the stalled attempt is retried once: {pids:?}\n{log}"
    );
    for pid in &pids {
        assert!(!alive(pid), "provider process {pid} outlived its attempt");
    }

    // The watchdog cancelled each attempt as stalled, and told the dashboard.
    let cancelled: Vec<Value> = jsonl(&log_file)
        .into_iter()
        .filter(|line| line["type"] == "dashboard.diagnosis")
        .map(|line| line["event"]["summary"].clone())
        .filter(|summary| summary["intervention_taken"] == "cancelled stalled attempt")
        .collect();
    for summary in &cancelled {
        assert_eq!(summary["subject"], "silent/T1", "{summary}");
    }
    let mut attempts: Vec<&str> = cancelled
        .iter()
        .filter_map(|summary| summary["id"].as_str())
        .collect();
    attempts.sort_unstable();
    attempts.dedup();
    assert_eq!(
        attempts.len(),
        2,
        "one cancellation per attempt: {cancelled:#?}\n{log}"
    );

    // The checkpoint fails the task, whose two attempts both settled failed.
    let roko = workdir.join(".roko");
    let checkpoint: Value = serde_json::from_str(
        &fs::read_to_string(roko.join("state/graph/silent/checkpoint.json"))
            .expect("the run wrote its Graph checkpoint"),
    )
    .expect("the checkpoint is JSON");
    assert_eq!(
        checkpoint["extensions"]["roko.task.outcome@1"]["value"]["failed"],
        serde_json::json!(["T1"]),
        "{checkpoint:#}"
    );
    let run_dirs: Vec<_> = fs::read_dir(roko.join("runs"))
        .expect("the run wrote .roko/runs")
        .map(|entry| entry.expect("run directory").path())
        .collect();
    assert_eq!(run_dirs.len(), 1, "{run_dirs:?}");
    let outcomes: Vec<String> = jsonl(&run_dirs[0].join("attempts.jsonl"))
        .into_iter()
        .filter(|line| line["schema_version"] == "roko.verdict/1")
        .map(|verdict| verdict["outcome"].as_str().unwrap_or("?").to_string())
        .collect();
    assert_eq!(outcomes.len(), 2, "{outcomes:?}");
    assert!(
        outcomes.iter().all(|outcome| outcome != "passed"),
        "{outcomes:?}"
    );
}

#[test]
fn supervision_canary_refuses_a_low_disk_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path();
    // No disk has a billion MB free.
    write_workspace(
        workdir,
        "\n[resources]\nmin_free_disk_mb = 1000000000\n",
        "silent",
        SILENT_TASKS,
    );

    let (succeeded, _, log) = run_plan(workdir, "silent", &[]);
    assert!(!succeeded, "the run must refuse to start: {log}");
    assert!(
        log.contains("MB available"),
        "the message names the free space: {log}"
    );
    assert!(
        log.contains("1000000000 MB required"),
        "the message names the threshold: {log}"
    );
    assert!(
        provider_pids(workdir).is_empty(),
        "nothing was dispatched: {log}"
    );
}
