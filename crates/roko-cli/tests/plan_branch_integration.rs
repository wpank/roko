#![cfg(unix)]

//! Canaries C3 and C4 (gap-af00b1): real `roko plan run --worktree-per-task`
//! runs over a scripted provider.
//!
//! - C3: every task that passes leaves exactly one commit on the plan branch,
//!   touching only its own files, and the checkpoint names it. A run killed
//!   in the middle of a task and resumed leaves no edit unattributed, and the
//!   operator's checkout never changes.
//! - C4: two tasks that each pass their own verify but break the build
//!   together fail the plan on its `[meta] verify`, and `roko plan status`
//!   names the failed step.
//!
//! A plain `roko plan run`, with no flag and no `[runner]` setting, runs the
//! same way, since per-task worktrees are the default (gap-4ec59f).

mod common;

use std::fs;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use common::scripted_provider::{Script, ScriptedProvider, Turn};
use serde_json::Value;

/// Run git in `dir` and return its trimmed stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = StdCommand::new("git")
        .current_dir(dir)
        .args(args)
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
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Write `files` (path, contents) under `repo` with `provider` configured as
/// the only model, and commit them as the operator.
fn seed_repo(repo: &Path, provider: &Path, files: &[(&str, &str)]) {
    let config = format!(
        "[agent]\ndefault_model = \"fake\"\n\n[providers.fake]\nkind = \"claude_cli\"\n\
         command = {:?}\n\n[models.fake]\nprovider = \"fake\"\nslug = \"claude-sonnet-4-6\"\n\n\
         [spec_quality]\nred_on_base = false\n",
        provider.display().to_string()
    );
    for (path, contents) in [("roko.toml", config.as_str())]
        .into_iter()
        .chain(files.iter().map(|(path, contents)| (*path, *contents)))
    {
        let path = repo.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        fs::write(path, contents).expect("write fixture");
    }
    git(repo, &["init", "--quiet", "--initial-branch=main"]);
    for (key, value) in [
        ("user.name", "Operator"),
        ("user.email", "operator@example.test"),
        ("commit.gpgsign", "false"),
    ] {
        git(repo, &["config", key, value]);
    }
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "--quiet", "-m", "fixture"]);
}

/// The operator's checkout: its HEAD, its index and its status. The status
/// leaves out `plans/INDEX.md`, the plan index roko rewrites after every
/// successful command that can change plans (`finish_with_index_rebuild` in
/// `main.rs`), which is not the integration's doing.
fn operator_state(repo: &Path) -> [String; 3] {
    let status = git(repo, &["status", "--porcelain", "--untracked-files=all"])
        .lines()
        .filter(|line| !line.ends_with(" plans/INDEX.md"))
        .collect::<Vec<_>>()
        .join("\n");
    [
        git(repo, &["rev-parse", "HEAD"]),
        git(repo, &["ls-files", "--stage"]),
        status,
    ]
}

/// `roko --json plan run <plan> --worktree-per-task` in `repo`, building
/// into `target`.
fn plan_run(repo: &Path, plan: &str, target: &Path) -> StdCommand {
    let mut command = StdCommand::new(cargo_bin("roko"));
    command.arg("--json");
    command
        .current_dir(repo)
        .args(["plan", "run", plan, "--worktree-per-task", "--workdir"])
        .arg(repo)
        .env("CARGO_TARGET_DIR", target);
    command
}

fn wait_for(path: &Path, timeout: Duration) {
    let started = Instant::now();
    while !path.exists() {
        assert!(
            started.elapsed() < timeout,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// C3: three dependent tasks, each writing its own file. The run is killed
/// while the second task's provider is at work, then resumed.
#[test]
fn c3_each_passed_task_commits_once_on_the_plan_branch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (repo, state) = (temp.path().join("repo"), temp.path().join("state"));
    fs::create_dir_all(&repo).expect("repo dir");
    fs::create_dir_all(&state).expect("state dir");
    // Each task writes `<name>.txt`. While the `hang` file exists, the second
    // task's provider waits after its edit.
    let names = ["one", "two", "three"];
    let script = names
        .iter()
        .enumerate()
        .fold(Script::new(), |script, (index, name)| {
            let turn = Turn::reply().write(&format!("{name}.txt"), &format!("{name}\n"));
            let turn = if *name == "two" {
                turn.hold_while(&state.join("hang"), &state.join("two-started"), 60)
            } else {
                turn
            };
            script.task(&format!("T{}", index + 1), [turn])
        });
    let provider = ScriptedProvider::install(&temp.path().join("provider"), &script).command();
    let mut tasks =
        String::from("[meta]\nplan = \"c3\"\nmax_parallel = 1\nskip_enrichment = true\n");
    for (index, name) in names.iter().enumerate() {
        let depends_on = index
            .checked_sub(1)
            .map(|previous| format!("depends_on = [\"T{}\"]\n", previous + 1))
            .unwrap_or_default();
        tasks.push_str(&format!(
            "\n[[task]]\nid = \"T{id}\"\ntitle = \"Write {name}.txt\"\n\
             description = \"Write {name}.txt.\"\nrole = \"implementer\"\nstatus = \"ready\"\n\
             tier = \"focused\"\nfiles = [\"{name}.txt\"]\n{depends_on}\n\
             [[task.verify]]\nphase = \"structural\"\ncommand = \"test -f {name}.txt\"\n",
            id = index + 1
        ));
    }
    seed_repo(
        &repo,
        &provider,
        &[(".gitignore", ".roko/\n"), ("plans/c3/tasks.toml", &tasks)],
    );
    let base = git(&repo, &["rev-parse", "HEAD"]);
    let before = operator_state(&repo);
    let target = temp.path().join("target");

    // The first process dies while the second task's provider works.
    fs::write(state.join("hang"), "").expect("hang");
    let mut first = plan_run(&repo, "plans/c3", &target)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start roko");
    wait_for(&state.join("two-started"), Duration::from_secs(180));
    first.kill().expect("kill roko");
    first.wait().expect("reap roko");
    fs::remove_file(state.join("hang")).expect("release the provider");
    let branch_before_resume = git(&repo, &["rev-list", &format!("{base}..roko/plan/c3")]);
    assert_eq!(
        branch_before_resume.lines().count(),
        1,
        "only the first task was accepted before the kill"
    );

    // The resumed run finishes the plan.
    let resumed = plan_run(&repo, "plans/c3", &target)
        .args(["--resume-plan", ".roko/state/graph"])
        .output()
        .expect("resume roko");
    assert!(
        resumed.status.success(),
        "resume failed: {}\n{}",
        String::from_utf8_lossy(&resumed.stdout),
        String::from_utf8_lossy(&resumed.stderr)
    );

    // One commit per task on the plan branch, oldest first, each touching
    // only its task's file.
    let commits = git(
        &repo,
        &["rev-list", "--reverse", &format!("{base}..roko/plan/c3")],
    );
    let commits: Vec<&str> = commits.lines().collect();
    assert_eq!(commits.len(), names.len(), "{commits:?}");
    for (commit, name) in commits.iter().zip(names) {
        assert_eq!(
            git(&repo, &["show", "--name-only", "--format=", commit]),
            format!("{name}.txt"),
            "commit {commit}"
        );
    }
    // The second task's edit from before the kill is in its own commit.
    assert_eq!(
        git(&repo, &["show", &format!("{}:two.txt", commits[1])]),
        "two"
    );
    // The checkpoint names each accepted commit.
    let activities = fs::read_to_string(repo.join(".roko/state/graph/c3/activities.jsonl"))
        .expect("activity log");
    for commit in &commits {
        assert!(
            activities.contains(&format!("\"workspace.accepted_commit\":\"{commit}\"")),
            "the checkpoint does not name {commit}"
        );
    }
    // The operator's checkout never changed.
    assert_eq!(operator_state(&repo), before);
}

/// C4: task A changes `a::double`'s signature and passes `a`'s tests; task B
/// adds a caller in `b` with the old signature and passes its own,
/// file-scoped verify. Together `b` no longer builds, which the plan's
/// `[meta] verify` catches.
#[test]
fn c4_meta_verify_catches_tasks_that_break_together() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let script = Script::new()
        .task(
            "A",
            [Turn::reply().write(
                "a/src/lib.rs",
                "pub fn double(x: u32, factor: u32) -> u32 {\n    x * factor\n}\n\n\
                 #[cfg(test)]\nmod tests {\n    #[test]\n    fn doubles() {\n        \
                 assert_eq!(super::double(2, 2), 4);\n    }\n}\n",
            )],
        )
        .task(
            "B",
            [Turn::reply().write(
                "b/src/lib.rs",
                "pub fn quad(x: u32) -> u32 {\n    a::double(a::double(x))\n}\n",
            )],
        );
    let provider = ScriptedProvider::install(&temp.path().join("provider"), &script).command();
    let tasks = r#"[meta]
plan = "c4"
max_parallel = 2
skip_enrichment = true

[[meta.verify]]
phase = "build"
command = "cargo check --workspace --quiet"

[[task]]
id = "A"
title = "Give a::double a factor"
description = "Give a::double a factor."
role = "implementer"
status = "ready"
tier = "focused"
files = ["a/src/lib.rs"]

[[task.verify]]
phase = "test"
command = "cargo test -p a --quiet"

[[task]]
id = "B"
title = "Add b::quad"
description = "Add b::quad."
role = "implementer"
status = "ready"
tier = "focused"
files = ["b/src/lib.rs"]

[[task.verify]]
phase = "structural"
command = "grep -q 'pub fn quad' b/src/lib.rs"
"#;
    seed_repo(
        &repo,
        &provider,
        &[
            (".gitignore", ".roko/\ntarget/\nCargo.lock\n"),
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
            ),
            (
                "a/Cargo.toml",
                "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            ),
            (
                "a/src/lib.rs",
                "pub fn double(x: u32) -> u32 {\n    x * 2\n}\n",
            ),
            (
                "b/Cargo.toml",
                "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
                 [dependencies]\na = { path = \"../a\" }\n",
            ),
            ("b/src/lib.rs", "pub fn id(x: u32) -> u32 {\n    x\n}\n"),
            ("plans/c4/tasks.toml", tasks),
        ],
    );
    let base = git(&repo, &["rev-parse", "HEAD"]);
    let before = operator_state(&repo);
    let target = temp.path().join("target");

    // A plain-text run, whose summary names the plans that failed.
    let run = StdCommand::new(cargo_bin("roko"))
        .current_dir(&repo)
        .args(["plan", "run", "plans/c4", "--worktree-per-task", "--no-tui"])
        .arg("--workdir")
        .arg(&repo)
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .expect("run roko");
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    // Both tasks passed and were accepted onto the plan branch...
    assert!(
        git(&repo, &["show", "roko/plan/c4:a/src/lib.rs"]).contains("factor: u32"),
        "{log}"
    );
    assert!(
        git(&repo, &["show", "roko/plan/c4:b/src/lib.rs"]).contains("pub fn quad"),
        "{log}"
    );
    // ...but the plan did not succeed: the command exits 1 naming it, the
    // plan branch is kept, and nothing reached the batch.
    assert_eq!(run.status.code(), Some(1), "{log}");
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("Plans that did not succeed: c4"),
        "{log}"
    );
    let batch = git(
        &repo,
        &[
            "for-each-ref",
            "--format=%(objectname)",
            "refs/heads/roko/batch/",
        ],
    );
    assert_eq!(batch, base, "{log}");

    // The delivery ended `RegressionFailed`, and `roko plan status` names the
    // plan and the failed step.
    let status = Command::new(cargo_bin("roko"))
        .current_dir(&repo)
        .args(["--json", "plan", "status", "plans/c4", "--workdir"])
        .arg(&repo)
        .output()
        .expect("plan status");
    let status: Value = serde_json::from_slice(&status.stdout).expect("plan status JSON");
    let failure = status["plan_check_failure"].as_str().unwrap_or_default();
    for expected in [
        "ended regression_failed",
        "regression failed for c4",
        "cargo check --workspace --quiet",
    ] {
        assert!(failure.contains(expected), "{expected}: {status}");
    }
    assert_ne!(status["status"], "complete", "plan status: {status}");
    assert_eq!(operator_state(&repo), before);
}

/// gap-4ec59f: per-task worktrees are the default. A plain `roko plan run` in
/// a git checkout, with no flag and no `[runner]` setting, runs a dependent
/// task on its predecessor's work and delivers the plan into the run's batch
/// branch. The operator's checkout never changes, and the run ends with the
/// command that takes the work, which `roko plan status` repeats.
#[test]
fn a_plain_plan_run_isolates_its_tasks_by_default() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let script = Script::new()
        .task("T1", [Turn::reply().write("one.txt", "one\n")])
        .task("T2", [Turn::reply().write("two.txt", "two\n")]);
    let provider = ScriptedProvider::install(&temp.path().join("provider"), &script).command();
    let task = |id: &str, name: &str, extra: &str, check: &str| {
        format!(
            "\n[[task]]\nid = \"{id}\"\ntitle = \"Write {name}.txt\"\n\
             description = \"Write {name}.txt.\"\nrole = \"implementer\"\nstatus = \"ready\"\n\
             tier = \"focused\"\nfiles = [\"{name}.txt\"]\n{extra}\n\
             [[task.verify]]\nphase = \"structural\"\ncommand = \"{check}\"\n"
        )
    };
    // T2's check passes only on T1's work.
    let tasks = format!(
        "[meta]\nplan = \"isolated\"\nmax_parallel = 1\nskip_enrichment = true\n{}{}",
        task("T1", "one", "", "test -f one.txt"),
        task(
            "T2",
            "two",
            "depends_on = [\"T1\"]\n",
            "test -f one.txt && test -f two.txt",
        ),
    );
    seed_repo(
        &repo,
        &provider,
        &[
            (".gitignore", ".roko/\n"),
            ("plans/isolated/tasks.toml", &tasks),
        ],
    );
    let before = operator_state(&repo);

    let run = StdCommand::new(cargo_bin("roko"))
        .current_dir(&repo)
        .args(["plan", "run", "plans/isolated", "--no-tui", "--workdir"])
        .arg(&repo)
        .env("CARGO_TARGET_DIR", temp.path().join("target"))
        .env_remove("ROKO_CONFIG")
        .output()
        .expect("run roko");
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    let log = format!("{stdout}\n{}", String::from_utf8_lossy(&run.stderr));
    assert!(run.status.success(), "{log}");

    // The plan is on the run's one batch branch, and the checkout is as it was.
    let batch = git(
        &repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/roko/batch/",
        ],
    );
    assert_eq!(batch.lines().count(), 1, "one batch branch: {batch}\n{log}");
    let files = git(&repo, &["ls-tree", "--name-only", &batch]);
    assert!(
        files.contains("one.txt") && files.contains("two.txt"),
        "{files}\n{log}"
    );
    assert_eq!(operator_state(&repo), before, "{log}");
    assert!(!repo.join("one.txt").exists(), "{log}");

    // The run and `roko plan status` both say how to take the work.
    let command = format!("git merge --ff-only {batch}");
    assert!(stdout.contains(&command), "{log}");
    let status = Command::new(cargo_bin("roko"))
        .current_dir(&repo)
        .args(["--json", "plan", "status", "plans/isolated", "--workdir"])
        .arg(&repo)
        .env_remove("ROKO_CONFIG")
        .output()
        .expect("plan status");
    let status: Value = serde_json::from_slice(&status.stdout).expect("plan status JSON");
    assert_eq!(status["delivery"]["branch"], batch.as_str(), "{status}");
    assert_eq!(
        status["delivery"]["merge_command"],
        command.as_str(),
        "{status}"
    );

    // Taking it brings both tasks' work into the checkout.
    git(&repo, &["merge", "--ff-only", &batch]);
    for (file, text) in [("one.txt", "one\n"), ("two.txt", "two\n")] {
        assert_eq!(fs::read_to_string(repo.join(file)).expect("read"), text);
    }
}
