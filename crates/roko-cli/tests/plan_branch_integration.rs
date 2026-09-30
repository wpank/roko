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

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use serde_json::Value;

/// The provider's reply once it has made its edit.
const PROVIDER_REPLY: &str = r#"printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"canary","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

/// Reads the task's title, the line after `# Task Request`, into `$title`.
const READ_TITLE: &str = r#"#!/bin/sh
set -eu
prompt="$(cat) $*"
title=$(printf '%s\n' "$prompt" | awk '/# Task Request$/ { getline; print; exit }')
"#;

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).expect("write the provider");
    let mut permissions = fs::metadata(path).expect("provider metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make the provider executable");
}

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
         command = {:?}\n\n[models.fake]\nprovider = \"fake\"\nslug = \"claude-sonnet-4-6\"\n",
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
    // Writes `<name>.txt` for the task titled `Write <name>.txt`. While the
    // `hang` file exists, the second task's provider waits after its edit.
    let provider = temp.path().join("provider.sh");
    write_executable(
        &provider,
        &format!(
            "{READ_TITLE}name=$(printf '%s' \"$title\" | sed -n 's/^Write \\([a-z]*\\)\\.txt$/\\1/p')
printf '%s\\n' \"$name\" > \"$name.txt\"
if [ \"$name\" = two ] && [ -e '{state}/hang' ]; then
  touch '{state}/two-started'
  i=0
  while [ -e '{state}/hang' ] && [ $i -lt 600 ]; do sleep 0.1; i=$((i + 1)); done
fi
{PROVIDER_REPLY}",
            state = state.display()
        ),
    );
    let mut tasks =
        String::from("[meta]\nplan = \"c3\"\nmax_parallel = 1\nskip_enrichment = true\n");
    let names = ["one", "two", "three"];
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
    let provider = temp.path().join("provider.sh");
    write_executable(
        &provider,
        &format!(
            r#"{READ_TITLE}case "$title" in
"Give a::double a factor")
  printf 'pub fn double(x: u32, factor: u32) -> u32 {{\n    x * factor\n}}\n\n#[cfg(test)]\nmod tests {{\n    #[test]\n    fn doubles() {{\n        assert_eq!(super::double(2, 2), 4);\n    }}\n}}\n' > a/src/lib.rs
  ;;
"Add b::quad")
  printf 'pub fn quad(x: u32) -> u32 {{\n    a::double(a::double(x))\n}}\n' > b/src/lib.rs
  ;;
esac
{PROVIDER_REPLY}"#
        ),
    );
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
