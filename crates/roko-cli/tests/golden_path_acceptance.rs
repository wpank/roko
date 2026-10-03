#![cfg(unix)]

//! The golden path's acceptance test (gap-f30b8e; backlog 3115): a seed
//! repository and a spec-first plan of nine tasks over it, each with a
//! planner-written `[task.accept]` test, in `tests/fixtures/golden_path/`
//! (`seed/` and `plan/`).
//!
//! - `golden_path_fixture_is_red_on_the_seed`: the seed passes the plan's
//!   whole-plan check, and every task's acceptance test fails on it, so a
//!   pass credits only the task's work.
//!
//! The TypeScript checks need Node 22.6 or later, which runs TypeScript by
//! stripping its types; without it they are skipped, and the test says so.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use roko_cli::task_accept::{AcceptStore, pin_task};
use roko_cli::task_parser::TasksFile;

/// A part of the fixture: `seed` or `plan`.
fn fixture(part: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden_path")
        .join(part)
}

/// Copy the directory `from` to `to`.
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create a directory");
    for entry in fs::read_dir(from).expect("read a fixture directory") {
        let entry = entry.expect("a directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy a fixture file");
        }
    }
}

/// Whether `node` runs TypeScript by stripping its types (Node 22.6 or
/// later).
fn node_runs_typescript() -> bool {
    Command::new("node")
        .args(["--experimental-strip-types", "-e", "const n: number = 1;"])
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Whether `command` runs Node.
fn needs_node(command: &str) -> bool {
    command.contains("node ")
}

/// Run `command` with `sh -c` in `dir`, with cargo building into `target`:
/// whether it passed, and what it printed.
fn sh(dir: &Path, target: &Path, command: &str) -> (bool, String) {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("run sh");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), printed)
}

/// backlog 3115: the seed is green, and every task's checks are red on it.
/// The seed passes the plan's `[meta] verify` (fmt, clippy, the Rust, Python
/// and TypeScript suites). Each task's pinned acceptance test, run as `plan
/// run` runs it, and any verify step of its own then fail on the seed.
#[test]
fn golden_path_fixture_is_red_on_the_seed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let seed = temp.path().join("seed");
    copy_dir(&fixture("seed"), &seed);
    let target = temp.path().join("target");
    let typescript = node_runs_typescript();
    if !typescript {
        eprintln!("node 22.6 or later is not on PATH: the TypeScript checks are skipped");
    }
    let plan_dir = fixture("plan");
    let plan = TasksFile::parse(&plan_dir.join("tasks.toml")).expect("the fixture plan parses");
    let count = plan.tasks.len();
    assert!((8..=10).contains(&count), "{count} tasks");

    for step in &plan.meta.verify {
        if needs_node(&step.command) && !typescript {
            continue;
        }
        let (passed, printed) = sh(&seed, &target, &step.command);
        assert!(passed, "the seed fails `{}`:\n{printed}", step.command);
    }

    // The acceptance tests are pinned outside any home directory.
    let store = AcceptStore::at(temp.path().join("accept"));
    let plan_id = plan.meta.plan.clone();
    for mut task in plan.tasks {
        let id = task.id.clone();
        assert!(task.has_accept_tests(), "{id} has no [task.accept] test");
        assert!(task.model_hint.is_none(), "{id} pins a model");
        pin_task(&store, &seed, &plan_id, &plan_dir, &mut task).expect("pin the accept tests");
        for step in &task.verify {
            if needs_node(&step.command) && !typescript {
                continue;
            }
            let (passed, printed) = sh(&seed, &target, &step.command);
            assert!(
                !passed,
                "{id}: a check passes on the seed, so a pass would credit nothing:\n{}\n{printed}",
                step.command
            );
        }
    }
}
