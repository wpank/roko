#![cfg(unix)]
//! A plan run leaves the operator's checkout as it found it (1222), however
//! it is started: `roko plan run`, or `roko run plans/<dir>` (bug-26935e).
//! Its results go to the plan branch and `.roko/`; before, every successful
//! run also rebuilt `plans/INDEX.md` and the `.roko` indexes in the
//! checkout (`finish_with_index_rebuild` in `main.rs`, and two more call
//! sites in `run_cmd.rs`), which showed up as an untracked or changed file.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::ScriptedPlanWorkspace;
use common::scripted_provider::{Script, Turn};

const PLAN: &str = "clean-checkout";

/// One task, which writes `one.txt`. The whole-plan check passes, so no
/// cargo runs.
const TASKS: &str = r#"[meta]
plan = "clean-checkout"
max_parallel = 1
skip_enrichment = true

[[meta.verify]]
phase = "plan"
command = "true"

[[task]]
id = "T1"
title = "Write one.txt"
description = "Write one.txt holding the line 1."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["one.txt"]
depends_on = []
timeout_secs = 120
max_retries = 0
verify = [{ phase = "structural", command = "test -f one.txt" }]
"#;

fn log(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Run git in the workspace's repository, with its home, and return stdout
/// without its trailing newline.
fn git(workspace: &ScriptedPlanWorkspace, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(&workspace.repo)
        .env("HOME", &workspace.home)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string()
}

/// Build the fixture shared by both tests below: a plan with the default
/// per-task worktrees (gap-4ec59f, rather than the fixture's usual shared
/// working tree), committed, with no `plans/INDEX.md` yet. Returns the
/// workspace, the pre-run `HEAD`, and the index path.
fn setup() -> (ScriptedPlanWorkspace, String, PathBuf) {
    let script = Script::new().task("T1", [Turn::reply().write("one.txt", "1\n")]);
    let (workspace, _provider) = ScriptedPlanWorkspace::with_provider(PLAN, TASKS, &script, "");
    let config_path = workspace.repo.join("roko.toml");
    let config = fs::read_to_string(&config_path).expect("read roko.toml");
    assert!(
        config.contains("runner.worktree_per_task = false\n"),
        "{config}"
    );
    let config = config.replace("runner.worktree_per_task = false\n", "");
    fs::write(&config_path, config).expect("write roko.toml");
    workspace.git(&["commit", "--quiet", "--all", "-m", "task worktrees"]);
    let base = git(&workspace, &["rev-parse", "HEAD"]);
    let index = workspace.repo.join("plans").join("INDEX.md");
    assert!(!index.exists(), "the fixture has no plan index");
    (workspace, base, index)
}

/// After `run`: the task's commit is on the plan branch, the checkout keeps
/// its pre-run `HEAD` (`base`), `git status` lists nothing outside
/// `.roko/`, and `index` (`plans/INDEX.md`) is still absent.
fn assert_checkout_clean(
    workspace: &ScriptedPlanWorkspace,
    base: &str,
    index: &Path,
    run: &Output,
) {
    assert!(run.status.success(), "{}", log(run));

    // The task's work is on the plan branch.
    let accepted = git(
        workspace,
        &["rev-list", &format!("{base}..roko/plan/{PLAN}")],
    );
    assert_eq!(accepted.lines().count(), 1, "{}", log(run));
    // The checkout is as the run found it.
    assert_eq!(git(workspace, &["rev-parse", "HEAD"]), base);
    let status = git(
        workspace,
        &["status", "--porcelain", "--untracked-files=all"],
    );
    let changed: Vec<&str> = status
        .lines()
        .filter(|line| !line.get(3..).is_some_and(|path| path.starts_with(".roko/")))
        .collect();
    assert!(
        changed.is_empty(),
        "the run changed the checkout: {changed:?}\n{}",
        log(run)
    );
    assert!(
        !index.exists(),
        "the run wrote {}\n{}",
        index.display(),
        log(run)
    );
}

/// 1222: after a successful `roko plan run` with the default per-task
/// worktrees, the checkout is left clean.
#[test]
fn plan_run_leaves_operator_checkout_clean() {
    let (workspace, base, index) = setup();
    let run = workspace.run_plan(PLAN, &[]);
    assert_checkout_clean(&workspace, &base, &index, &run);
}

/// bug-26935e: `roko run plans/<dir>` leaves the checkout clean too. Before
/// the fix, it rebuilt the plan indexes after a successful run
/// (`run_cmd.rs::run_plan_dir`), unlike `roko plan run` (1222).
#[test]
fn run_plan_dir_leaves_operator_checkout_clean() {
    let (workspace, base, index) = setup();
    let run = workspace.run_args(&["run", &format!("plans/{PLAN}"), "--workdir"], &[]);
    assert_checkout_clean(&workspace, &base, &index, &run);
}
