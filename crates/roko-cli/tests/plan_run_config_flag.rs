#![cfg(unix)]

//! bug-4ed3c2: `roko --config <file> plan run` runs under `<file>`, not the
//! workspace's `roko.toml`, and a `--config` file that does not exist fails
//! the run before any agent starts.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use assert_cmd::cargo::cargo_bin;

/// A provider that writes `chosen.txt`, holding `NAME`, in its working
/// directory and reports success.
const PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' 'NAME' > chosen.txt
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"config","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

const TASKS: &str = r#"[meta]
plan = "config"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Write chosen.txt"
description = "Write chosen.txt."
role = "implementer"
status = "ready"
tier = "focused"
files = ["chosen.txt"]
max_retries = 0

[[task.verify]]
phase = "structural"
command = "test -f chosen.txt"
"#;

/// A config whose only model runs the provider at `dir/<name>.sh`, which
/// writes `name` into `chosen.txt`.
fn config_running(dir: &Path, name: &str) -> String {
    let provider = dir.join(format!("{name}.sh"));
    fs::write(&provider, PROVIDER.replace("NAME", name)).expect("write the provider");
    let mut permissions = fs::metadata(&provider).expect("provider").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&provider, permissions).expect("make the provider executable");
    format!(
        "[agent]\ndefault_model = \"fake\"\n\n[providers.fake]\nkind = \"claude_cli\"\n\
         command = {:?}\n\n[models.fake]\nprovider = \"fake\"\nslug = \"claude-sonnet-4-6\"\n",
        provider.display().to_string()
    )
}

/// `roko --config <config> plan run plans/config` in `workdir`.
fn plan_run_with_config(workdir: &Path, config: &Path) -> Output {
    Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .arg("--config")
        .arg(config)
        .args(["plan", "run", "plans/config", "--no-tui", "--workdir"])
        .arg(workdir)
        .env_remove("ROKO_CONFIG")
        .output()
        .expect("run roko")
}

fn log(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The workspace's `roko.toml` runs a provider that writes `workspace`, and
/// `other.toml`, outside the workspace, one that writes `other`. With
/// `--config other.toml` the run uses the second; with a `--config` file
/// that does not exist it fails before any agent runs.
#[test]
fn plan_run_uses_the_config_flag() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path().join("workspace");
    fs::create_dir_all(workdir.join("plans/config")).expect("plan dir");
    fs::write(workdir.join("plans/config/tasks.toml"), TASKS).expect("tasks");
    fs::write(
        workdir.join("roko.toml"),
        config_running(temp.path(), "workspace"),
    )
    .expect("workspace config");
    let other = temp.path().join("other.toml");
    fs::write(&other, config_running(temp.path(), "other")).expect("other config");

    let missing = plan_run_with_config(&workdir, &temp.path().join("missing.toml"));
    assert!(!missing.status.success(), "{}", log(&missing));
    assert!(
        log(&missing).contains("missing.toml: no such file"),
        "{}",
        log(&missing)
    );
    assert!(!workdir.join("chosen.txt").exists(), "{}", log(&missing));

    let run = plan_run_with_config(&workdir, &other);
    assert!(run.status.success(), "{}", log(&run));
    assert_eq!(
        fs::read_to_string(workdir.join("chosen.txt")).expect("the agent ran"),
        "other\n",
        "{}",
        log(&run)
    );
}
