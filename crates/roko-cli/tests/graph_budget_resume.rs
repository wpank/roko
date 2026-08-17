#![cfg(unix)]

//! End-to-end coverage for resume-durable Graph provider-cost accounting.

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).expect("write provider script");
    let mut permissions = fs::metadata(path).expect("provider metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make provider executable");
}

fn setup_workspace(workdir: &Path) {
    let provider = workdir.join("fake-provider.sh");
    write_executable(
        &provider,
        r#"#!/bin/sh
set -eu
cat >/dev/null
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
printf 'call\n' >> "$script_dir/provider-calls"
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"resume-cost-output"}}'
printf '%s\n' '{"type":"result","session_id":"resume-cost","model":"claude-sonnet-4-6","total_cost_usd":0.001,"usage":{"input_tokens":10,"output_tokens":5},"is_error":false}'
"#,
    );
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

[budget]
max_plan_usd = 0.01
max_turn_usd = 0.01
"#,
            provider = provider.display().to_string()
        ),
    )
    .expect("write roko config");

    let plan_dir = workdir.join("plans/resume-cost");
    fs::create_dir_all(&plan_dir).expect("create plan directory");
    fs::write(
        plan_dir.join("plan.md"),
        "# Plan: resume-cost\n\nVerify cost survives Graph resume.\n",
    )
    .expect("write plan markdown");
    fs::write(
        plan_dir.join("tasks.toml"),
        r#"
[meta]
plan = "resume-cost"
iteration = 1
total = 1
done = 0
status = "ready"
max_parallel = 1
estimated_total_minutes = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Record one provider cost"
description = "Return a deterministic provider result."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "graph-model"
files = ["fixture-output.txt"]
allowed_tools = []
denied_tools = []
mcp_servers = []
depends_on = []
depends_on_plan = []
acceptance = []
verify = [{ phase = "structural", command = "true", fail_msg = "fixture verification failed" }]
timeout_secs = 10
max_retries = 0
"#,
    )
    .expect("write plan tasks");
}

fn run_graph(workdir: &Path, resume: bool) -> Value {
    let mut command = Command::new(cargo_bin("roko"));
    command
        .current_dir(workdir)
        .arg("--json")
        .args(["plan", "run", "plans/resume-cost", "--engine", "graph"])
        .arg("--workdir")
        .arg(workdir);
    if resume {
        command.arg("--resume-plan").arg(".roko/state/graph");
    }
    let assert = command.assert().success();
    serde_json::from_slice(&assert.get_output().stdout).expect("parse Graph JSON output")
}

#[test]
fn cli_graph_resume_restores_actual_provider_cost_without_reinvocation() {
    let temp = tempfile::tempdir().expect("tempdir");
    setup_workspace(temp.path());

    let first = run_graph(temp.path(), false);
    assert_eq!(first["total_cost_usd"].as_f64(), Some(0.001));
    assert_eq!(
        fs::read_to_string(temp.path().join("provider-calls"))
            .expect("first provider call")
            .lines()
            .count(),
        1
    );

    let resumed = run_graph(temp.path(), true);
    assert_eq!(resumed["total_cost_usd"].as_f64(), Some(0.001));
    assert_eq!(
        resumed["plan_budgets"][0]["spent_usd"].as_f64(),
        Some(0.001)
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("provider-calls"))
            .expect("provider call log after resume")
            .lines()
            .count(),
        1,
        "replayed Graph task must not invoke the provider again"
    );

    fs::write(
        temp.path().join(".roko/state/graph/resume-cost/costs.json"),
        b"corrupt",
    )
    .expect("corrupt cost ledger");
    Command::new(cargo_bin("roko"))
        .current_dir(temp.path())
        .arg("--json")
        .args([
            "plan",
            "run",
            "plans/resume-cost",
            "--engine",
            "graph",
            "--resume-plan",
            ".roko/state/graph",
        ])
        .arg("--workdir")
        .arg(temp.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("parse Graph cost ledger"));
}
