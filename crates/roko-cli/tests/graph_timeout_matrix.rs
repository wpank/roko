#![cfg(unix)]

//! The Graph engine's timeout and termination matrix (q-1faa0c).
//!
//! The dev audit's runtime fixes for timeouts and shutdown lived in
//! Runner-v2's event loop, which was deleted. Each case here proves one of
//! them on `roko plan run` end to end: the real binary, in a throwaway
//! workspace whose agent is a fake Claude CLI, so no model is called.
//!
//! - `timeout_keeps_usage`: a timed-out attempt keeps the tokens and the
//!   model its agent streamed, in `.roko/learn/costs.jsonl`.
//! - `terminal_projections_agree`: after a pass, a failed verify step, a
//!   timeout and SIGTERM mid-task, the exit code, the `--log-file`
//!   `run.completed` line, the checkpoint status and `roko plan status`
//!   agree, `run.completed` and the checkpoint name the same stop request,
//!   and no agent the run registered is left alive.
//! - `terminal_projections_agree_in_task_worktrees`: the timeout and
//!   SIGTERM cases again, with the task in its own git worktree, which is
//!   the default since gap-4ec59f. Every other case pins the shared working
//!   tree, as `ScriptedPlanWorkspace` does.
//! - `interrupt_settles_when_agent_ignores_sigterm`: an agent that ignores
//!   SIGTERM is killed, and the run exits 143 in bounded time.
//! - `timeout_retry_continues_from_partial_work`: a timed-out attempt's edits
//!   are not verified; the next attempt is told to continue from them, and
//!   they are still there.
//! - `fast_deadline_stops_the_run`: a FAST run stops at its deadline as it
//!   does on SIGTERM.
//! - `resume_after_timeout_is_idempotent`: resuming a plan whose task timed
//!   out runs that task again but neither re-runs nor re-records the task
//!   that had passed.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use common::ScriptedPlanWorkspace;
use common::scripted_provider::{Script, Turn};
use serde_json::Value;

/// The plan every case runs.
const PLAN: &str = "matrix";

/// `roko.toml` additions: no verify step waits for sibling tasks.
const CONFIG: &str = "gates.sibling_settle_secs = 0\n";

/// Longest a run may take before the test gives up on it.
const RUN_LIMIT: Duration = Duration::from_secs(180);

/// Longest a stopped run may take to exit: the run's drain
/// (`INTERRUPT_DRAIN_TIMEOUT`, 3 s) and its forced exit
/// (`FORCED_EXIT_GRACE`, 10 s), with room for a loaded machine.
const STOP_LIMIT: Duration = Duration::from_secs(25);

/// One task of the plan.
struct Task {
    id: &'static str,
    verify: &'static str,
    timeout_secs: u64,
    max_retries: u32,
    depends_on: &'static [&'static str],
}

impl Task {
    /// Task `id`, whose verify step runs `verify`, with an attempt timeout of
    /// `timeout_secs` and no retries.
    fn new(id: &'static str, verify: &'static str, timeout_secs: u64) -> Self {
        Self {
            id,
            verify,
            timeout_secs,
            max_retries: 0,
            depends_on: &[],
        }
    }

    /// The task, run once `ids` have finished.
    fn after(mut self, ids: &'static [&'static str]) -> Self {
        self.depends_on = ids;
        self
    }

    /// The task, with `max_retries` more attempts after a failed one.
    fn retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// `tasks.toml` of [`PLAN`], one implementer task naming `NOTES.md` per
/// [`Task`]. `meta.total` is set, as the FAST lane requires.
fn tasks_toml(tasks: &[Task]) -> String {
    let mut toml = format!(
        "[meta]\nplan = \"{PLAN}\"\ntotal = {}\nmax_parallel = 1\nskip_enrichment = true\n",
        tasks.len()
    );
    for task in tasks {
        toml.push_str(&format!(
            r#"
[[task]]
id = "{id}"
title = "Task {id}"
description = "Report a finished turn."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["NOTES.md"]
depends_on = {depends_on:?}
timeout_secs = {timeout_secs}
max_retries = {max_retries}
verify = [{{ phase = "structural", command = {verify:?} }}]
"#,
            id = task.id,
            depends_on = task.depends_on,
            timeout_secs = task.timeout_secs,
            max_retries = task.max_retries,
            verify = task.verify,
        ));
    }
    toml
}

/// An agent turn that changes the task's file and reports a finished turn.
fn edit() -> Turn {
    Turn::reply().append("NOTES.md", "attempt\n")
}

/// An agent turn that changes the task's file and then works on, silently,
/// for a minute.
fn hang() -> Turn {
    edit().silent_for(60.0)
}

/// `roko` with `args` in the workspace's repository, as its user: provider
/// keys, `ROKO_*` variables and the invoking environment's log and config
/// variables are removed, as `ScriptedPlanWorkspace::run_plan` does, and so
/// are the variables that would point roko's git at another repository.
fn roko(workspace: &ScriptedPlanWorkspace, args: &[&str]) -> Command {
    let mut command = Command::new(cargo_bin("roko"));
    command
        .current_dir(&workspace.repo)
        .args(args)
        .env("HOME", &workspace.home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in roko_core::child_env::PROVIDER_KEY_VARS {
        command.env_remove(name);
    }
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("ROKO_") {
            command.env_remove(name);
        }
    }
    for name in [
        "RUST_LOG",
        "XDG_CONFIG_HOME",
        "CLAUDECODE",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
    ] {
        command.env_remove(name);
    }
    // Warnings and errors on stderr, so a failure explains itself.
    command.env("ROKO_LOG", "warn");
    command
}

/// `roko plan run` of [`PLAN`], with `extra` appended. The run's events go
/// to `fixtures/events.jsonl`.
fn plan_run(workspace: &ScriptedPlanWorkspace, extra: &[&str]) -> Command {
    let mut command = roko(workspace, &["--json", "plan", "run"]);
    command
        .arg(format!("plans/{PLAN}"))
        .arg("--no-tui")
        .arg("--workdir")
        .arg(&workspace.repo)
        .arg("--log-file")
        .arg(events_path(workspace))
        .args(extra);
    command
}

fn events_path(workspace: &ScriptedPlanWorkspace) -> PathBuf {
    workspace.fixtures.join("events.jsonl")
}

/// A `roko` process the test started, with its output drained as it runs.
struct Run {
    child: Child,
    stdout: std::thread::JoinHandle<Vec<u8>>,
    stderr: std::thread::JoinHandle<Vec<u8>>,
}

/// How a `roko` process ended.
struct Ended {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl Run {
    fn start(mut command: Command) -> Self {
        let mut child = command.spawn().expect("start roko");
        let stdout = drain(child.stdout.take().expect("stdout pipe"));
        let stderr = drain(child.stderr.take().expect("stderr pipe"));
        Self {
            child,
            stdout,
            stderr,
        }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Wait for the process to exit, killing it after `limit`.
    fn wait(mut self, limit: Duration) -> Ended {
        let started = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("poll roko") {
                break status;
            }
            if started.elapsed() > limit {
                let _ = self.child.kill();
                let status = self.child.wait().expect("reap roko");
                let ended = Self::output(status, self.stdout, self.stderr);
                panic!("roko ran past {limit:?}\n{}", ended.context());
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        Self::output(status, self.stdout, self.stderr)
    }

    fn output(
        status: ExitStatus,
        stdout: std::thread::JoinHandle<Vec<u8>>,
        stderr: std::thread::JoinHandle<Vec<u8>>,
    ) -> Ended {
        let text = |reader: std::thread::JoinHandle<Vec<u8>>| {
            String::from_utf8_lossy(&reader.join().expect("output reader")).into_owned()
        };
        Ended {
            status,
            stdout: text(stdout),
            stderr: text(stderr),
        }
    }
}

impl Ended {
    /// The run's output, for failure messages: stdout, and the tail of
    /// stderr.
    fn context(&self) -> String {
        let tail = self
            .stderr
            .get(self.stderr.len().saturating_sub(4000)..)
            .unwrap_or(&self.stderr);
        format!(
            "exit: {}\nstdout:\n{}\nstderr (tail):\n{tail}",
            self.status, self.stdout
        )
    }
}

fn drain(mut pipe: impl std::io::Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// Run `command` to its end.
fn run(command: Command) -> Ended {
    Run::start(command).wait(RUN_LIMIT)
}

/// The JSON lines of `path`; none when it does not exist.
fn json_lines(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// The status in [`PLAN`]'s checkpoint.
fn checkpoint_status(workspace: &ScriptedPlanWorkspace) -> String {
    let path = workspace
        .repo
        .join(".roko/state/graph")
        .join(PLAN)
        .join("checkpoint.json");
    let checkpoint: Value = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
    )
    .expect("checkpoint json");
    checkpoint["status"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The stop [`PLAN`]'s checkpoint names in its `roko.run.stop@1` extension
/// (gap-fab2cc), such as `SIGTERM` or `deadline`; `None` when no stop
/// request ended its run.
fn checkpoint_stop(workspace: &ScriptedPlanWorkspace) -> Option<String> {
    roko_cli::graph_checkpoint::canonical_stop_cause(&workspace.repo, PLAN)
}

/// The status `roko plan status` reports for [`PLAN`].
fn plan_status(workspace: &ScriptedPlanWorkspace) -> String {
    let mut command = roko(workspace, &["--json", "plan", "status"]);
    command
        .arg(format!("plans/{PLAN}"))
        .arg("--workdir")
        .arg(&workspace.repo);
    let output = command.output().expect("run roko plan status");
    serde_json::from_slice::<Value>(&output.stdout).map_or_else(
        |error| format!("unreadable ({error})"),
        |status| status["status"].as_str().unwrap_or_default().to_string(),
    )
}

/// The `run.completed` line of the run's `--log-file`.
fn run_completed(workspace: &ScriptedPlanWorkspace) -> Value {
    json_lines(&events_path(workspace))
        .into_iter()
        .rfind(|line| line["type"] == "run.completed")
        .unwrap_or(Value::Null)
}

/// The rows of `.roko/learn/costs.jsonl` for task `id`.
fn cost_rows(workspace: &ScriptedPlanWorkspace, id: &str) -> Vec<Value> {
    json_lines(&workspace.repo.join(".roko/learn/costs.jsonl"))
        .into_iter()
        .filter(|row| row["task_id"] == id)
        .collect()
}

/// Whether process `pid` exists.
fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Whether process `pid` is gone, or goes within a few seconds.
fn gone(pid: u32) -> bool {
    let started = Instant::now();
    while alive(pid) {
        if started.elapsed() > Duration::from_secs(5) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    true
}

/// Every agent PID the run registered in the workspace's PID registry
/// (`.roko/runtime/agent-pids/`).
fn registered_agent_pids(workspace: &ScriptedPlanWorkspace) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir(workspace.repo.join(".roko/runtime/agent-pids")) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .flat_map(|record| {
            record["children"]
                .as_array()
                .map(|children| {
                    children
                        .iter()
                        .filter_map(|child| child["pid"].as_u64())
                        .filter_map(|pid| u32::try_from(pid).ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Send `signal` (such as `TERM`) to process `pid`.
fn signal(pid: u32, signal: &str) {
    let status = Command::new("kill")
        .args([&format!("-{signal}"), &pid.to_string()])
        .status()
        .expect("run kill");
    assert!(status.success(), "kill -{signal} {pid} failed");
}

/// Wait for `ready` to hold, for at most a minute.
fn wait_for(what: &str, ready: impl Fn() -> bool) {
    let started = Instant::now();
    while !ready() {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

// ── timeout_keeps_usage ─────────────────────────────────────────────────

/// A fake Claude CLI that streams one priced assistant message, then works
/// on silently, past any attempt timeout: it never reaches its `result`
/// event.
const STALLING_AGENT: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"system","subtype":"init","session_id":"matrix","model":"claude-sonnet-4-6"}'
printf '%s\n' '{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"text","text":"working"}],"usage":{"input_tokens":1000,"output_tokens":200}}}'
exec sleep 120
"#;

/// q-1faa0c, behaviour 1 (bug-690dc6): a provider that streams usage and
/// then times out keeps its tokens, model and cost.
#[test]
fn timeout_keeps_usage() {
    // A loaded machine may not deliver the message before a short timeout:
    // each run doubles the timeout until one is cut short after it arrived.
    for timeout_secs in [3, 6, 12] {
        let workspace = ScriptedPlanWorkspace::new(
            PLAN,
            &tasks_toml(&[Task::new("T1", "test -d .", timeout_secs)]),
            STALLING_AGENT,
            CONFIG,
        );
        let ended = run(plan_run(&workspace, &[]));
        assert_eq!(ended.status.code(), Some(1), "{}", ended.context());
        let rows = cost_rows(&workspace, "T1");
        let [row] = rows.as_slice() else {
            panic!(
                "expected one cost row for T1: {rows:#?}\n{}",
                ended.context()
            );
        };
        assert_eq!(row["success"], false, "{row}");
        if row["input_tokens"] == 0 {
            // The message did not arrive before the timeout.
            continue;
        }
        assert_eq!(row["outcome"], "timeout", "{row}");
        assert_eq!(row["input_tokens"], 1_000, "{row}");
        assert_eq!(row["output_tokens"], 200, "{row}");
        assert_eq!(row["model"], "claude-sonnet-4-6", "{row}");
        assert_eq!(row["cost_source"], "estimated", "{row}");
        assert!(
            row["cost_usd"].as_f64().is_some_and(|cost| cost > 0.0),
            "{row}"
        );
        return;
    }
    panic!("no run kept the usage its agent streamed before the timeout");
}

// ── terminal_projections_agree ──────────────────────────────────────────

/// What every projection of a finished run says.
#[derive(Debug, PartialEq, Eq)]
struct Projections {
    exit_code: Option<i32>,
    outcome: String,
    logged_exit_code: Option<i64>,
    /// The stop request `run.completed` names (`interrupted_by`).
    logged_stop: Option<String>,
    checkpoint: String,
    /// The stop request the checkpoint names (`roko.run.stop@1`).
    checkpoint_stop: Option<String>,
    status: String,
}

impl Projections {
    /// The run's exit code, its `run.completed` line, its checkpoint and
    /// `roko plan status`.
    fn of(workspace: &ScriptedPlanWorkspace, ended: &Ended) -> Self {
        let end = run_completed(workspace);
        Self {
            exit_code: ended.status.code(),
            outcome: end["outcome"].as_str().unwrap_or_default().to_string(),
            logged_exit_code: end["exit_code"].as_i64(),
            logged_stop: end["interrupted_by"].as_str().map(str::to_string),
            checkpoint: checkpoint_status(workspace),
            checkpoint_stop: checkpoint_stop(workspace),
            status: plan_status(workspace),
        }
    }

    /// A run that exited `exit_code`, which its `run.completed` line logs
    /// with `outcome`, whose checkpoint says `checkpoint`, and which `roko
    /// plan status` reports as `status`. No stop request ended it.
    fn expected(exit_code: i32, outcome: &str, checkpoint: &str, status: &str) -> Self {
        Self {
            exit_code: Some(exit_code),
            outcome: outcome.to_string(),
            logged_exit_code: Some(i64::from(exit_code)),
            logged_stop: None,
            checkpoint: checkpoint.to_string(),
            checkpoint_stop: None,
            status: status.to_string(),
        }
    }

    /// The same run, ended by the stop request `stop`, which its
    /// `run.completed` line and its checkpoint both name (gap-1d8a49).
    fn stopped_by(mut self, stop: &str) -> Self {
        self.logged_stop = Some(stop.to_string());
        self.checkpoint_stop = Some(stop.to_string());
        self
    }
}

/// One way a one-task run of [`PLAN`] ends, and what its projections say.
struct Ending {
    name: &'static str,
    task: Task,
    /// What the task's agent does.
    turn: Turn,
    /// SIGTERM goes to `roko` once the agent has started.
    terminate: bool,
    /// The task runs in its own git worktree (`--worktree-per-task`).
    task_worktrees: bool,
    expected: Projections,
}

impl Ending {
    /// A run in the shared working tree whose task `task`'s agent plays
    /// `turn`, and whose projections say `expected`.
    fn new(name: &'static str, task: Task, turn: Turn, expected: Projections) -> Self {
        Self {
            name,
            task,
            turn,
            terminate: false,
            task_worktrees: false,
            expected,
        }
    }

    /// The same run, sent SIGTERM once its agent has started.
    fn terminated(mut self) -> Self {
        self.terminate = true;
        self
    }

    /// The same run, with its task in its own git worktree, as `plan run`
    /// runs tasks by default since gap-4ec59f.
    fn in_task_worktrees(mut self) -> Self {
        self.task_worktrees = true;
        self
    }
}

/// A run whose task's attempt runs past its 3 s timeout.
fn timeout_ending() -> Ending {
    Ending::new(
        "timeout",
        Task::new("T1", "test -d .", 3),
        hang(),
        Projections::expected(1, "failed", "failed", "failed"),
    )
}

/// A run sent SIGTERM while its agent works.
fn sigterm_ending() -> Ending {
    Ending::new(
        "SIGTERM mid-task",
        Task::new("T1", "test -d .", 60),
        hang(),
        Projections::expected(143, "cancelled", "interrupted", "interrupted").stopped_by("SIGTERM"),
    )
    .terminated()
}

/// Run `ending`, and check that its projections say what it expects, that
/// its agent ran in the checkout it should, and that none of its agents is
/// left alive.
fn assert_ending(ending: Ending) {
    let Ending {
        name,
        task,
        turn,
        terminate,
        task_worktrees,
        expected,
    } = ending;
    let case = if task_worktrees {
        format!("{name}, task worktrees")
    } else {
        name.to_string()
    };
    let (workspace, provider) = ScriptedPlanWorkspace::with_provider(
        PLAN,
        &tasks_toml(&[task]),
        &Script::new().otherwise(turn),
        CONFIG,
    );
    let args: &[&str] = if task_worktrees {
        &["--worktree-per-task"]
    } else {
        &[]
    };
    let run = Run::start(plan_run(&workspace, args));
    let ended = if terminate {
        wait_for("the agent to start", || {
            provider.calls().iter().any(|call| call.pid.is_some())
        });
        signal(run.pid(), "TERM");
        run.wait(STOP_LIMIT)
    } else {
        run.wait(RUN_LIMIT)
    };

    assert_eq!(
        Projections::of(&workspace, &ended),
        expected,
        "{case}\n{}",
        ended.context()
    );
    let calls = provider.calls();
    assert!(
        !calls.is_empty(),
        "{case}: no agent ran\n{}",
        ended.context()
    );
    // A task worktree is a checkout under `.roko/worktrees/`; otherwise the
    // agent works in the repository itself.
    let worktrees = workspace.repo.join(".roko/worktrees");
    for call in &calls {
        assert_eq!(
            call.cwd.starts_with(&worktrees),
            task_worktrees,
            "{case}: the agent ran in {}",
            call.cwd.display()
        );
    }
    let agents: Vec<u32> = calls
        .iter()
        .filter_map(|call| call.pid)
        .chain(registered_agent_pids(&workspace))
        .collect();
    for pid in agents {
        assert!(gone(pid), "{case}: agent {pid} outlived the run");
    }
}

/// q-1faa0c, behaviour 2: whatever ends a run, its exit code, its
/// `run.completed` line, its checkpoint and `roko plan status` agree, the
/// stop request that ended it is named alike in `run.completed` and the
/// checkpoint, and none of its agents is left alive.
#[test]
fn terminal_projections_agree() {
    for ending in [
        Ending::new(
            "pass",
            Task::new("T1", "test -d .", 60),
            edit(),
            Projections::expected(0, "succeeded", "succeeded", "complete"),
        ),
        Ending::new(
            "failed verify step",
            Task::new("T1", "false", 60),
            edit(),
            Projections::expected(1, "failed", "failed", "failed"),
        ),
        timeout_ending(),
        sigterm_ending(),
    ] {
        assert_ending(ending);
    }
}

/// gap-1d8a49: a timeout and SIGTERM end a run whose task runs in its own
/// git worktree, the default since gap-4ec59f, as they end one in the
/// shared working tree.
#[test]
fn terminal_projections_agree_in_task_worktrees() {
    for ending in [timeout_ending(), sigterm_ending()] {
        assert_ending(ending.in_task_worktrees());
    }
}

// ── interrupt_settles_when_agent_ignores_sigterm ────────────────────────

/// A fake Claude CLI that ignores SIGTERM, records its PID beside itself in
/// `agent.pid`, and works on, as one process, for two minutes.
const STUBBORN_AGENT: &str = r#"#!/bin/sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
trap '' TERM
cat >/dev/null
printf '%s\n' "$$" > "$dir/agent.pid.tmp"
mv "$dir/agent.pid.tmp" "$dir/agent.pid"
exec sleep 120
"#;

/// q-1faa0c, behaviour 3: SIGTERM to a run whose agent ignores SIGTERM ends
/// in bounded time: the run exits 143, its checkpoint says `interrupted` and
/// names SIGTERM as the stop, and the agent is killed.
#[test]
fn interrupt_settles_when_agent_ignores_sigterm() {
    let workspace = ScriptedPlanWorkspace::new(
        PLAN,
        &tasks_toml(&[Task::new("T1", "test -d .", 120)]),
        STUBBORN_AGENT,
        CONFIG,
    );
    let agent_pid = workspace.fixtures.join("agent.pid");
    let run = Run::start(plan_run(&workspace, &[]));
    wait_for("the agent to start", || agent_pid.exists());
    let pid: u32 = std::fs::read_to_string(&agent_pid)
        .expect("agent pid")
        .trim()
        .parse()
        .expect("agent pid number");

    let signalled = Instant::now();
    signal(run.pid(), "TERM");
    let ended = run.wait(STOP_LIMIT);
    let took = signalled.elapsed();

    assert_eq!(ended.status.code(), Some(143), "{}", ended.context());
    assert!(took < STOP_LIMIT, "the run took {took:?} to stop");
    assert_eq!(
        checkpoint_status(&workspace),
        "interrupted",
        "{}",
        ended.context()
    );
    assert!(gone(pid), "the agent that ignored SIGTERM outlived the run");
    // The checkpoint names the stop, and so does `run.completed` unless the
    // run was forced out before it could write one (gap-1d8a49).
    let stop = checkpoint_stop(&workspace);
    assert_eq!(stop.as_deref(), Some("SIGTERM"), "{}", ended.context());
    let logged = run_completed(&workspace)["interrupted_by"]
        .as_str()
        .map(str::to_string);
    assert!(
        logged.is_none() || logged == stop,
        "run.completed names {logged:?}, the checkpoint {stop:?}"
    );
}

// ── timeout_retry_continues_from_partial_work ───────────────────────────

/// q-1faa0c, behaviour 4: a timed-out attempt's edits are not verified
/// ("timeout salvage" is dropped). The task's next attempt is told that the
/// attempt before it ran out of time and left its edits in the working
/// tree, and they are still there for it to continue from.
#[test]
fn timeout_retry_continues_from_partial_work() {
    // A loaded machine may cut the first attempt off before its edit: each
    // run doubles the timeout until the first attempt made its edit in time.
    for timeout_secs in [3, 6, 12] {
        let signals = tempfile::tempdir().expect("tempdir");
        let hold = signals.path().join("hold");
        let edited = signals.path().join("edited");
        std::fs::write(&hold, "").expect("write the hold file");
        let (workspace, provider) = ScriptedPlanWorkspace::with_provider(
            PLAN,
            &tasks_toml(&[Task::new("T1", "test -d .", timeout_secs).retries(1)]),
            // The first attempt edits, says so, and works on past its
            // timeout; the second finishes.
            &Script::new().task("T1", [edit().hold_while(&hold, &edited, 60), edit()]),
            CONFIG,
        );
        let ended = run(plan_run(&workspace, &[]));
        if !edited.exists() {
            // The first attempt ran out of time before its edit.
            continue;
        }

        assert_eq!(ended.status.code(), Some(0), "{}", ended.context());
        let calls = provider.calls_for("T1");
        let [first, second] = calls.as_slice() else {
            panic!(
                "expected two attempts at T1: {calls:#?}\n{}",
                ended.context()
            );
        };
        let resumes = |prompt: &str| prompt.contains("# Resuming a timed-out task");
        assert!(!resumes(&first.prompt), "{}", first.prompt);
        assert!(resumes(&second.prompt), "{}", second.prompt);
        assert_eq!(
            std::fs::read_to_string(workspace.repo.join("NOTES.md")).unwrap_or_default(),
            "attempt\nattempt\n",
            "the second attempt did not find the first one's edit\n{}",
            ended.context()
        );
        return;
    }
    panic!("no first attempt made its edit before its timeout");
}

// ── fast_deadline_stops_the_run ─────────────────────────────────────────

/// q-1faa0c, behaviour 5: a FAST run (`ROKO_FAST_MODE`) stops at its
/// deadline (`ROKO_FAST_PLAN_DEADLINE_SECS`) as SIGTERM stops a run: it
/// exits 143, its checkpoint says `interrupted`, its `run.completed` line
/// and its checkpoint both name the deadline, and its agent is gone. The
/// per-attempt FAST bounds (gap-4a6dcb) have tests of their own.
#[test]
fn fast_deadline_stops_the_run() {
    // A loaded machine may reach a short deadline before the agent starts:
    // each run doubles the deadline until one stops a running agent.
    for deadline_secs in ["3", "6", "12"] {
        let (workspace, provider) = ScriptedPlanWorkspace::with_provider(
            PLAN,
            &tasks_toml(&[Task::new("T1", "test -d .", 120)]),
            &Script::new().otherwise(hang()),
            CONFIG,
        );
        let mut command = plan_run(&workspace, &[]);
        command
            .env("ROKO_FAST_MODE", "1")
            .env("ROKO_FAST_PLAN_DEADLINE_SECS", deadline_secs);
        let ended = run(command);
        let agents: Vec<u32> = provider
            .calls()
            .iter()
            .filter_map(|call| call.pid)
            .collect();
        if agents.is_empty() {
            // The deadline came before the agent started.
            continue;
        }

        assert_eq!(
            Projections::of(&workspace, &ended),
            Projections::expected(143, "cancelled", "interrupted", "interrupted")
                .stopped_by("deadline"),
            "{}",
            ended.context()
        );
        for pid in agents {
            assert!(gone(pid), "agent {pid} outlived the run");
        }
        return;
    }
    panic!("no run's agent started before its FAST deadline");
}

// ── resume_after_timeout_is_idempotent ──────────────────────────────────

/// q-1faa0c: resuming a plan after a task timed out runs that task again,
/// but the task that had passed is replayed, not run or recorded again.
#[test]
fn resume_after_timeout_is_idempotent() {
    let (workspace, provider) = ScriptedPlanWorkspace::with_provider(
        PLAN,
        &tasks_toml(&[
            Task::new("T1", "test -d .", 60),
            Task::new("T2", "test -d .", 3).after(&["T1"]),
        ]),
        // T2's first attempt runs past its timeout; its next one finishes.
        &Script::new().task("T2", [hang(), edit()]).otherwise(edit()),
        CONFIG,
    );

    let first = run(plan_run(&workspace, &[]));
    assert_eq!(first.status.code(), Some(1), "{}", first.context());
    assert_eq!(
        checkpoint_status(&workspace),
        "failed",
        "{}",
        first.context()
    );
    assert_eq!(provider.calls_for("T1").len(), 1, "{}", first.context());
    assert_eq!(provider.calls_for("T2").len(), 1, "{}", first.context());
    let t1_rows = cost_rows(&workspace, "T1").len();
    assert_eq!(t1_rows, 1, "{}", first.context());

    let resumed = run(plan_run(
        &workspace,
        &["--resume-plan", ".roko/state/graph"],
    ));
    assert_eq!(resumed.status.code(), Some(0), "{}", resumed.context());
    assert_eq!(
        checkpoint_status(&workspace),
        "succeeded",
        "{}",
        resumed.context()
    );
    assert_eq!(
        provider.calls_for("T1").len(),
        1,
        "the passed task ran again\n{}",
        resumed.context()
    );
    assert_eq!(provider.calls_for("T2").len(), 2, "{}", resumed.context());
    assert_eq!(
        cost_rows(&workspace, "T1").len(),
        t1_rows,
        "the passed task was recorded again\n{}",
        resumed.context()
    );
    assert_eq!(
        cost_rows(&workspace, "T2").len(),
        2,
        "{}",
        resumed.context()
    );
}
