#![cfg(unix)]

//! Canary C5 (assessment W8, gate G5): a Graph run whose agent tampers with
//! what checks it is stopped before its verify steps, one whose agent changes
//! nothing on a tree that does not already pass its checks is rejected, and
//! the plan's checkpoint and `roko plan status` agree that it failed. A
//! `--fresh` rerun of a finished task, whose work is already there, completes
//! as already satisfied (gap-9eb1e1).
//!
//! Each plan holds one task and runs on its own, in a git repository, with
//! the shared scripted provider playing each task's turn. Each verify step
//! checks for the helper its task adds, then leaves a marker outside the
//! repository, so the test can tell whether it passed.

mod common;

use assert_cmd::cargo::cargo_bin;
use common::scripted_provider::{Script, ScriptedProvider, Turn};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A throwaway workspace: the repository roko runs in, a home directory for
/// the pinned acceptance-test store, the verify markers, and the agent.
struct Canary {
    _temp: tempfile::TempDir,
    root: PathBuf,
    repo: PathBuf,
}

/// `tests/math.rs` as the repository holds it.
const MATH_TEST: &str =
    "use c5::add::add;\n\n#[test]\nfn adds() {\n    assert_eq!(add(2, 2), 4);\n}\n";

/// What the agent does on each task's attempts.
///
/// - T1 tampers: beside a real change to its own file, it edits the pinned
///   acceptance test's source and adds `#[ignore]` to an existing test.
/// - T2 also edits `README.md`, outside its files.
/// - T3 changes nothing.
/// - T4 makes an honest change, then changes nothing on a rerun.
fn script() -> Script {
    Script::new()
        .task(
            "T1",
            [Turn::reply()
                .append(
                    "src/add.rs",
                    "pub fn add3(a: u8, b: u8, c: u8) -> u8 {\n    add(add(a, b), c)\n}\n",
                )
                .append(
                    "plans/c5-tamper/accept/add_accept.sh",
                    "# the agent was here\necho \"# pass 1\"\n",
                )
                .write(
                    "tests/math.rs",
                    &MATH_TEST.replace("#[test]\n", "#[test]\n#[ignore]\n"),
                )],
        )
        .task(
            "T2",
            [Turn::reply()
                .append(
                    "src/sub.rs",
                    "pub fn sub3(a: u8, b: u8, c: u8) -> u8 {\n    sub(sub(a, b), c)\n}\n",
                )
                .append("README.md", "\nThe scope task also wrote this.\n")],
        )
        .task(
            "T4",
            [
                Turn::reply().append(
                    "src/mul.rs",
                    "pub fn mul3(a: u8, b: u8, c: u8) -> u8 {\n    mul(mul(a, b), c)\n}\n",
                ),
                Turn::reply(),
            ],
        )
}

/// The planner's acceptance test for `add`, pinned by `[task.accept]`.
const ADD_ACCEPT: &str = "#!/bin/sh\n# add(2, 2) is 4\ngrep -q 'a + b' src/add.rs || { echo 'not ok 1 add'; exit 1; }\necho 'ok 1 add'\necho '# pass 1'\n";

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, text).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=c5",
            "-c",
            "user.email=c5@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(repo)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// One single-task plan: task `id`, named `plan`, asked to add a helper to
/// `file`, whose verify step checks for it and then leaves
/// `<root>/markers/<id>.ran`.
fn plan(canary: &Canary, plan: &str, id: &str, file: &str, accept: bool) {
    let marker = canary.root.join("markers").join(format!("{id}.ran"));
    // The helper the task asks for: `add3` in `src/add.rs`, and so on.
    let helper = Path::new(file)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| format!("{stem}3"))
        .expect("a file name");
    let accept = if accept {
        "\n[task.accept]\nfiles = [{ src = \"accept/add_accept.sh\", dest = \"tests/accept/add_accept.sh\", runner = \"sh {dest}\", count = 1 }]\n"
    } else {
        ""
    };
    let dir = canary.repo.join("plans").join(plan);
    write(
        &dir.join("plan.md"),
        &format!("# Plan: {plan}\n\nCanary C5.\n"),
    );
    write(
        &dir.join("tasks.toml"),
        &format!(
            r#"[meta]
plan = "{plan}"
iteration = 1
total = 1
done = 0
status = "ready"
max_parallel = 1
estimated_total_minutes = 1
skip_enrichment = true

[[task]]
id = "{id}"
title = "Extend {file}"
description = "Add a three-argument helper to {file}."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "c5-model"
files = ["{file}"]
allowed_tools = []
denied_tools = []
mcp_servers = []
depends_on = []
depends_on_plan = []
acceptance = []
verify = [{{ phase = "structural", command = "grep -q {helper} {file} && touch {marker}", fail_msg = "the helper is missing" }}]
timeout_secs = 30
max_retries = 0
{accept}"#,
            marker = marker.display(),
        ),
    );
}

impl Canary {
    /// A repository holding a small crate, a README, a test file, and the
    /// four single-task plans, committed; `diff_scope` goes to `[gates]`.
    /// The tasks run in the shared working tree, as they did before per-task
    /// worktrees became the default (gap-4ec59f).
    fn new(diff_scope: &str) -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        // Canonical, so paths roko prints and paths the test builds agree.
        let root = temp.path().canonicalize().expect("canonical tempdir");
        let repo = root.join("repo");
        fs::create_dir_all(root.join("markers")).expect("markers dir");
        fs::create_dir_all(root.join("home")).expect("home dir");
        let agent = ScriptedProvider::install(&root.join("provider"), &script()).command();

        write(&repo.join(".gitignore"), ".roko/\n");
        write(
            &repo.join("Cargo.toml"),
            "[package]\nname = \"c5\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        write(
            &repo.join("src/lib.rs"),
            "pub mod add;\npub mod mul;\npub mod sub;\n",
        );
        write(
            &repo.join("src/add.rs"),
            "pub fn add(a: u8, b: u8) -> u8 {\n    a + b\n}\n",
        );
        write(
            &repo.join("src/sub.rs"),
            "pub fn sub(a: u8, b: u8) -> u8 {\n    a - b\n}\n",
        );
        write(
            &repo.join("src/mul.rs"),
            "pub fn mul(a: u8, b: u8) -> u8 {\n    a * b\n}\n",
        );
        write(&repo.join("tests/math.rs"), MATH_TEST);
        write(&repo.join("README.md"), "# c5\n");
        write(
            &repo.join("roko.toml"),
            &format!(
                r#"[agent]
default_model = "c5-model"
command = {agent:?}
bare_mode = false

[providers.c5-cli]
kind = "claude_cli"
command = {agent:?}

[models.c5-model]
provider = "c5-cli"
slug = "claude-sonnet-4-6"
context_window = 200000

[gates]
cargo_fix_enabled = false
diff_scope = "{diff_scope}"

[runner]
worktree_per_task = false
"#,
                agent = agent.display().to_string()
            ),
        );
        let canary = Self {
            _temp: temp,
            root,
            repo,
        };
        plan(&canary, "c5-tamper", "T1", "src/add.rs", true);
        write(
            &canary.repo.join("plans/c5-tamper/accept/add_accept.sh"),
            ADD_ACCEPT,
        );
        plan(&canary, "c5-scope", "T2", "src/sub.rs", false);
        plan(&canary, "c5-empty", "T3", "src/mul.rs", false);
        plan(&canary, "c5-honest", "T4", "src/mul.rs", false);
        git(&canary.repo, &["init", "--quiet"]);
        git(&canary.repo, &["add", "--all"]);
        git(&canary.repo, &["commit", "--quiet", "-m", "seed"]);
        canary
    }

    /// Roko in the repository, with the test's home and logs on stderr.
    fn roko(&self, args: &[&str]) -> Output {
        Command::new(cargo_bin("roko"))
            .current_dir(&self.repo)
            .args(args)
            .env("HOME", self.root.join("home"))
            .env("ROKO_LOG", "roko=warn")
            .env_remove("RUST_LOG")
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("XDG_CONFIG_HOME")
            .output()
            .expect("run roko")
    }

    /// Run `plan`; returns roko's output and stderr.
    fn run(&self, plan: &str) -> (Output, String) {
        self.run_with(plan, &[])
    }

    /// [`Self::run`] with `extra` arguments to `roko plan run`.
    fn run_with(&self, plan: &str, extra: &[&str]) -> (Output, String) {
        let repo = self.repo.display().to_string();
        let plan_dir = format!("plans/{plan}");
        let mut args = vec![
            "--json",
            "plan",
            "run",
            plan_dir.as_str(),
            "--engine",
            "graph",
            "--workdir",
            repo.as_str(),
        ];
        args.extend_from_slice(extra);
        let output = self.roko(&args);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        (output, stderr)
    }

    fn verify_ran(&self, id: &str) -> bool {
        self.root.join("markers").join(format!("{id}.ran")).exists()
    }

    /// The plan's Graph checkpoint.
    fn checkpoint(&self, plan: &str) -> Value {
        let path = self
            .repo
            .join(".roko/state/graph")
            .join(plan)
            .join("checkpoint.json");
        let bytes =
            fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        serde_json::from_slice(&bytes).expect("checkpoint JSON")
    }

    /// `roko plan status` of the plan, as JSON.
    fn plan_status(&self, plan: &str) -> Value {
        let repo = self.repo.display().to_string();
        let output = self.roko(&[
            "--json",
            "plan",
            "status",
            &format!("plans/{plan}"),
            "--workdir",
            &repo,
        ]);
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "plan status JSON: {error}\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        })
    }

    /// Assert that the checkpoint and `roko plan status` agree on how the
    /// plan ended: `failed` names the tasks the checkpoint records as failed.
    fn assert_outcome(&self, plan: &str, failed: &[&str]) {
        let checkpoint = self.checkpoint(plan);
        let status = self.plan_status(plan);
        let recorded: Vec<&str> =
            checkpoint["extensions"]["roko.task.outcome@1"]["value"]["failed"]
                .as_array()
                .map(|failed| failed.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
        assert_eq!(recorded, failed, "checkpoint: {checkpoint:#}");
        let (checkpoint_status, plan_status) = if failed.is_empty() {
            ("succeeded", "complete")
        } else {
            ("failed", "failed")
        };
        assert_eq!(
            checkpoint["status"], checkpoint_status,
            "checkpoint: {checkpoint:#}"
        );
        assert_eq!(status["status"], plan_status, "plan status: {status:#}");
    }
}

#[test]
fn c5_tampering_attempt_is_flagged() {
    let canary = Canary::new("record");

    // T1 edits the pinned acceptance test's source and adds `#[ignore]` to an
    // existing test, beside a real change to its own file.
    let (output, stderr) = canary.run("c5-tamper");
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(!canary.verify_ran("T1"), "T1's verify steps must not run");
    assert!(stderr.contains("pre_verify:tamper"), "stderr: {stderr}");
    assert!(
        stderr.contains("accept_edited `plans/c5-tamper/accept/add_accept.sh`"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("skip_added `tests/math.rs`"),
        "stderr: {stderr}"
    );
    canary.assert_outcome("c5-tamper", &["T1"]);

    // T2 edits README.md, outside its files: recorded, and its verify steps
    // still run under the default `diff_scope = "record"`.
    let (output, stderr) = canary.run("c5-scope");
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(canary.verify_ran("T2"));
    assert!(
        stderr.contains("outside_scope `README.md`"),
        "the scope finding is recorded; stderr: {stderr}"
    );
    assert!(!stderr.contains("pre_verify:scope"), "stderr: {stderr}");
    canary.assert_outcome("c5-scope", &[]);
    assert_eq!(
        canary.checkpoint("c5-scope")["extensions"]["roko.gate.verdict@1"]["value"]["verdicts"]["T2"],
        "passed"
    );

    // The same attempt fails under `diff_scope = "enforce"`.
    let enforced = Canary::new("enforce");
    let (output, stderr) = enforced.run("c5-scope");
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(!enforced.verify_ran("T2"), "T2's verify steps must not run");
    assert!(stderr.contains("pre_verify:scope"), "stderr: {stderr}");
    assert!(
        stderr.contains("outside_scope `README.md`"),
        "stderr: {stderr}"
    );
    enforced.assert_outcome("c5-scope", &["T2"]);
}

#[test]
fn c5_empty_diff_is_rejected_before_verify() {
    let canary = Canary::new("record");

    // T3's agent answers without changing anything. Its verify steps run on
    // the unchanged tree to see whether the task was already done, and fail.
    let (output, stderr) = canary.run("c5-empty");
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(!canary.verify_ran("T3"), "T3's verify steps must not pass");
    assert!(stderr.contains("pre_verify:no_changes"), "stderr: {stderr}");
    canary.assert_outcome("c5-empty", &["T3"]);

    // The same task with a real change passes: the screen is not a blanket
    // rejection.
    let (output, stderr) = canary.run("c5-honest");
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(canary.verify_ran("T4"));
    canary.assert_outcome("c5-honest", &[]);

    // gap-9eb1e1: a `--fresh` rerun finds the work already there. The agent
    // rightly changes nothing, and the task completes as already satisfied.
    fs::remove_file(canary.root.join("markers/T4.ran")).expect("reset T4's marker");
    let (output, stderr) = canary.run_with("c5-honest", &["--fresh"]);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(canary.verify_ran("T4"), "T4's verify steps passed");
    canary.assert_outcome("c5-honest", &[]);
    let checkpoint = canary.checkpoint("c5-honest");
    assert_eq!(
        checkpoint["extensions"]["roko.gate.verdict@1"]["value"]["verdicts"]["T4"],
        "already_satisfied"
    );
}
