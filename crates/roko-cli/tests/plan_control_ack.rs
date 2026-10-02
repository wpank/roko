#![cfg(unix)]

//! `roko plan pause`, `resume`, `cancel` and `retry` report the plan run's
//! answer (1209). They reach the run over the socket `roko inject` uses, and
//! exit non-zero when no run is listening or the run refuses. Before, they
//! wrote `.roko/state/control.json` and exited 0 whether or not a run ever
//! read it.

mod common;

use std::fs;
use std::path::Path;
use std::process::{Child, Command, Output};
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use common::ScriptedPlanWorkspace;
use common::scripted_provider::{Script, Turn};

const PLAN: &str = "control-ack";

/// One task, whose verify step passes.
const TASKS: &str = r#"[meta]
plan = "control-ack"
max_parallel = 1
skip_enrichment = true

[[meta.verify]]
phase = "plan"
command = "true"

[[task]]
id = "T1"
title = "Write one.txt"
description = "Append a line to one.txt."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["one.txt"]
timeout_secs = 120
max_retries = 0
verify = [{ phase = "structural", command = "test -f one.txt" }]
"#;

/// `roko` in `dir` with `home` as its home, the invoking environment's
/// provider keys and `ROKO_*`, log and config variables removed.
fn roko(dir: &Path, home: &Path) -> Command {
    let mut command = Command::new(cargo_bin("roko"));
    command.current_dir(dir).env("HOME", home);
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
    command
}

/// `roko plan <args> --workdir <dir>`.
fn plan_control(dir: &Path, home: &Path, args: &[&str]) -> Output {
    roko(dir, home)
        .arg("plan")
        .args(args)
        .arg("--workdir")
        .arg(dir)
        .output()
        .expect("run roko plan")
}

/// What a command printed, for assertions and failure messages.
fn printed(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn plan_pause_without_a_running_plan_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");

    let output = plan_control(dir.path(), home.path(), &["pause"]);

    assert!(
        !output.status.success(),
        "roko plan pause succeeded with no run to take it: {}",
        printed(&output)
    );
    let text = printed(&output);
    assert!(
        text.contains("no plan run is listening"),
        "the error does not say why: {text}"
    );
    assert!(
        text.contains(&dir.path().display().to_string()),
        "the error does not name the workdir: {text}"
    );
    assert!(
        !dir.path().join(".roko/state/control.json").exists(),
        "a command was left for a run that may never read it"
    );
}

/// Kill the run if the test ends before it does.
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if let Ok(None) = self.0.try_wait() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

#[test]
fn a_running_plan_answers_plan_control_commands() {
    // While `hang` exists, T1's agent waits after its edit.
    let signals = tempfile::tempdir().expect("tempdir");
    let hang = signals.path().join("hang");
    let started = signals.path().join("t1-started");
    let script = Script::new().task(
        "T1",
        [Turn::reply()
            .append("one.txt", "one\n")
            .hold_while(&hang, &started, 120)],
    );
    let (workspace, _provider) = ScriptedPlanWorkspace::with_provider(PLAN, TASKS, &script, "");
    let (repo, home) = (&workspace.repo, &workspace.home);
    fs::write(&hang, "").expect("hold T1's agent");
    let log = workspace.root.join("plan-run.log");
    let file = fs::File::create(&log).expect("create the run log");
    let mut run = KillOnDrop(
        roko(repo, home)
            .arg("--json")
            .args(["plan", "run", &format!("plans/{PLAN}"), "--workdir"])
            .arg(repo)
            .stdout(file.try_clone().expect("share the run log"))
            .stderr(file)
            .spawn()
            .expect("start roko plan run"),
    );
    let run_log = || fs::read_to_string(&log).unwrap_or_default();
    let waiting = Instant::now();
    while !started.exists() {
        assert!(
            run.0.try_wait().expect("poll roko").is_none(),
            "the run ended before T1 started\n{}",
            run_log()
        );
        assert!(
            waiting.elapsed() < Duration::from_secs(120),
            "T1 did not start\n{}",
            run_log()
        );
        std::thread::sleep(Duration::from_millis(100));
    }

    // Each command prints the run's answer.
    let paused = plan_control(repo, home, &["pause"]);
    assert!(paused.status.success(), "pause: {}", printed(&paused));
    assert!(
        printed(&paused).contains("no new task starts until resume"),
        "pause did not print the run's answer: {}",
        printed(&paused)
    );
    let retried = plan_control(repo, home, &["retry", "--plan-id", PLAN]);
    assert!(
        !retried.status.success(),
        "the run cannot rerun a running plan, yet retry succeeded: {}",
        printed(&retried)
    );
    assert!(
        printed(&retried).contains("is running"),
        "retry did not print the run's refusal: {}",
        printed(&retried)
    );
    let resumed = plan_control(repo, home, &["resume"]);
    assert!(resumed.status.success(), "resume: {}", printed(&resumed));
    assert!(
        printed(&resumed).contains("resumed"),
        "resume did not print the run's answer: {}",
        printed(&resumed)
    );

    // The run goes on to the end.
    fs::remove_file(&hang).expect("let T1's agent finish");
    let waiting = Instant::now();
    let status = loop {
        if let Some(status) = run.0.try_wait().expect("poll roko") {
            break status;
        }
        assert!(
            waiting.elapsed() < Duration::from_secs(180),
            "the run did not end\n{}",
            run_log()
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(
        status.success(),
        "the plan failed ({status})\n{}",
        run_log()
    );
}
