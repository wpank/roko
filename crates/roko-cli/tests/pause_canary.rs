#![cfg(unix)]

//! Pause holds (G10, decision 1206): `roko plan pause` during a Graph run
//! lets the running attempt finish, and starts no task after it until
//! `roko plan resume`. Before, the run acknowledged the pause and showed it
//! as PAUSED, but nothing read the flag, and the next task started at once.
//!
//! The plan holds two tasks, T2 after T1, for the shared scripted provider.
//! T1's agent waits while a file exists, so the test can pause the run while
//! T1 is at work; each task's verify step leaves a marker outside the
//! repository, so the test knows when T1 settled.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output};
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use common::ScriptedPlanWorkspace;
use common::scripted_provider::{Script, Turn};

const PLAN: &str = "pause-canary";

/// Two tasks, T2 after T1. Each verify step leaves `<task>-verified` in the
/// fixtures; the whole-plan check passes, so no cargo runs.
const TASKS: &str = r#"[meta]
plan = "pause-canary"
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
depends_on = []
timeout_secs = 120
max_retries = 0
verify = [{ phase = "structural", command = "touch {fixtures}/T1-verified" }]

[[task]]
id = "T2"
title = "Write two.txt"
description = "Append a line to two.txt."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["two.txt"]
depends_on = ["T1"]
timeout_secs = 120
max_retries = 0
verify = [{ phase = "structural", command = "touch {fixtures}/T2-verified" }]
"#;

/// `roko` in `workspace`'s repository as its user, as
/// `ScriptedPlanWorkspace::run_plan` runs it: provider keys, `ROKO_*`
/// variables and the invoking environment's log and config variables are
/// removed.
fn roko(workspace: &ScriptedPlanWorkspace) -> Command {
    let mut command = Command::new(cargo_bin("roko"));
    command
        .current_dir(&workspace.repo)
        .env("HOME", &workspace.home);
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

/// `roko plan <action> --workdir <repo>`.
fn plan_control(workspace: &ScriptedPlanWorkspace, action: &str) -> Output {
    roko(workspace)
        .args(["plan", action, "--workdir"])
        .arg(&workspace.repo)
        .output()
        .unwrap_or_else(|error| panic!("run roko plan {action}: {error}"))
}

/// A `roko plan run` started in the background, its output in `log`. It is
/// killed if the test ends first.
struct BackgroundRun {
    child: Child,
    log: PathBuf,
}

impl BackgroundRun {
    fn start(workspace: &ScriptedPlanWorkspace, plan: &str) -> Self {
        let log = workspace.root.join("plan-run.log");
        let file = fs::File::create(&log).expect("create the run log");
        let child = roko(workspace)
            .arg("--json")
            .args(["plan", "run", &format!("plans/{plan}"), "--workdir"])
            .arg(&workspace.repo)
            .env("ROKO_LOG", "info")
            .stdout(file.try_clone().expect("share the run log"))
            .stderr(file)
            .spawn()
            .expect("start roko plan run");
        Self { child, log }
    }

    /// The last lines of the run's output, for failure messages.
    fn tail(&self) -> String {
        let text = fs::read_to_string(&self.log).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        lines[lines.len().saturating_sub(60)..].join("\n")
    }

    /// Whether the run is still going.
    fn is_running(&mut self) -> bool {
        self.child.try_wait().expect("poll roko").is_none()
    }

    /// Wait until `done` holds, failing when the run ends first or
    /// `timeout` passes; `what` names the wait.
    fn wait_until(&mut self, what: &str, timeout: Duration, done: impl Fn() -> bool) {
        let started = Instant::now();
        while !done() {
            assert!(
                self.is_running(),
                "the run ended before {what}\n{}",
                self.tail()
            );
            assert!(
                started.elapsed() < timeout,
                "timed out waiting until {what}\n{}",
                self.tail()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Wait for the run to end, at most `timeout`.
    fn wait(&mut self, timeout: Duration) -> ExitStatus {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("poll roko") {
                return status;
            }
            assert!(
                started.elapsed() < timeout,
                "the run did not end\n{}",
                self.tail()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for BackgroundRun {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// Whether `path` exists, asked when the closure runs.
fn exists(path: &Path) -> impl Fn() -> bool + '_ {
    move || path.exists()
}

/// Whether `path` is gone, asked when the closure runs.
fn gone(path: &Path) -> impl Fn() -> bool + '_ {
    move || !path.exists()
}

#[test]
fn pause_holds_next_task_until_resume() {
    // While `hang` exists, T1's agent waits after its edit.
    let signals = tempfile::tempdir().expect("tempdir");
    let hang = signals.path().join("hang");
    let t1_started = signals.path().join("t1-started");
    let script = Script::new()
        .task(
            "T1",
            [Turn::reply()
                .append("one.txt", "one\n")
                .hold_while(&hang, &t1_started, 120)],
        )
        .task("T2", [Turn::reply().append("two.txt", "two\n")]);
    let (workspace, provider) = ScriptedPlanWorkspace::with_provider(PLAN, TASKS, &script, "");
    let fixtures = workspace.fixtures.clone();
    fs::write(&hang, "").expect("hold T1's agent");
    let mut run = BackgroundRun::start(&workspace, PLAN);

    // T1's agent is at work: pause the run, and wait for the run to take
    // the command.
    run.wait_until(
        "T1's agent started",
        Duration::from_secs(120),
        exists(&t1_started),
    );
    let paused = plan_control(&workspace, "pause");
    assert!(
        paused.status.success(),
        "roko plan pause failed: {}\n{}",
        String::from_utf8_lossy(&paused.stderr),
        run.tail()
    );
    let control = workspace.repo.join(".roko/state/control.json");
    run.wait_until(
        "the run took the pause",
        Duration::from_secs(30),
        gone(&control),
    );

    // The running attempt finishes...
    fs::remove_file(&hang).expect("let T1's agent finish");
    run.wait_until(
        "T1's verify step ran",
        Duration::from_secs(120),
        exists(&fixtures.join("T1-verified")),
    );

    // ...and no task starts after it while the run is paused.
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        provider.calls_for("T2").is_empty(),
        "T2 started while the run was paused\n{}",
        run.tail()
    );
    assert!(
        run.is_running(),
        "the run ended while paused\n{}",
        run.tail()
    );

    // Resume: T2 runs, and the plan succeeds.
    let resumed = plan_control(&workspace, "resume");
    assert!(
        resumed.status.success(),
        "roko plan resume failed: {}\n{}",
        String::from_utf8_lossy(&resumed.stderr),
        run.tail()
    );
    let status = run.wait(Duration::from_secs(180));
    assert!(
        status.success(),
        "the plan did not succeed after resume ({status})\n{}",
        run.tail()
    );
    assert_eq!(
        provider.calls_for("T2").len(),
        1,
        "T2 ran once, after resume\n{}",
        run.tail()
    );
    assert!(
        fixtures.join("T2-verified").exists(),
        "T2's verify step did not run"
    );
}
