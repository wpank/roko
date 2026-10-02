#![cfg(unix)]

//! `roko plan revise` (3228): a revision that validates is written and the
//! command prints its task-level diff; one that does not exits 1 and leaves
//! the plan as it was. The planner is a fake `claude_cli` script.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::{Command, Output};

use assert_cmd::cargo::cargo_bin;

const PLAN: &str = r#"[meta]
plan = "demo"

[[task]]
id = "T1"
title = "Add the greeting"
description = "Add greet.sh, which prints hello."
role = "implementer"
files = ["greet.sh"]
depends_on = []

[[task.verify]]
phase = "test"
command = "test -f greet.sh"

[[task]]
id = "T2"
title = "Add the config"
description = "Add config.ini with the retry limit."
role = "implementer"
files = ["config.ini"]
depends_on = []

[[task.verify]]
phase = "test"
command = "test -f config.ini"
"#;

/// A config whose only model is a fake planner in `dir` that answers every
/// call with `plan_toml` in a `toml` block.
fn config_answering(dir: &Path, plan_toml: &str) -> String {
    let reply = dir.join("reply.jsonl");
    let delta = serde_json::json!({
        "type": "content_block_delta",
        "delta": {"text": format!("```toml\n{plan_toml}```\n")},
    });
    let result = serde_json::json!({
        "type": "result",
        "session_id": "planner",
        "model": "claude-sonnet-4-6",
        "total_cost_usd": 0.0,
        "usage": {"input_tokens": 1, "output_tokens": 1},
    });
    fs::write(&reply, format!("{delta}\n{result}\n")).expect("write the reply");
    let planner = dir.join("planner.sh");
    let script = format!(
        "#!/bin/sh\nset -eu\ncat >/dev/null\ncat '{}'\n",
        reply.display()
    );
    fs::write(&planner, script).expect("write the planner");
    let mut permissions = fs::metadata(&planner).expect("planner").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&planner, permissions).expect("make the planner executable");
    format!(
        "[agent]\ndefault_model = \"fake\"\n\n[providers.fake]\nkind = \"claude_cli\"\n\
         command = {:?}\n\n[models.fake]\nprovider = \"fake\"\nslug = \"claude-sonnet-4-6\"\n",
        planner.display().to_string()
    )
}

/// `roko plan revise plans/demo --feedback <feedback>` in `workdir`.
fn revise(workdir: &Path, feedback: &str) -> Output {
    Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .args(["plan", "revise", "plans/demo", "--feedback", feedback])
        .env("HOME", workdir)
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

/// 3228: `plan revise` writes a valid revision and prints the changed task
/// and field; an invalid revision exits 1 and leaves the file unchanged.
#[test]
fn plan_revise_cli_prints_the_plan_diff() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path().join("workspace");
    fs::create_dir_all(workdir.join("plans/demo")).expect("plan dir");
    let tasks = workdir.join("plans/demo/tasks.toml");
    fs::write(&tasks, PLAN).expect("write the plan");

    let revised = PLAN.replace(
        "command = \"test -f greet.sh\"",
        "command = \"bash greet.sh | grep -qx hello\"",
    );
    assert_ne!(revised, PLAN);
    let config = config_answering(temp.path(), &revised);
    fs::write(workdir.join("roko.toml"), config).expect("write roko.toml");
    let run = revise(&workdir, "Check what greet.sh prints");
    assert!(run.status.success(), "{}", log(&run));
    let out = String::from_utf8_lossy(&run.stdout);
    assert!(
        out.contains("plan diff: 0 added, 0 removed, 1 changed"),
        "{}",
        log(&run)
    );
    assert!(out.contains("\n  ~ T1\n      verify: "), "{}", log(&run));
    assert!(!out.contains("~ T2"), "{}", log(&run));
    let written = fs::read_to_string(&tasks).expect("read the plan");
    assert!(
        written.contains("bash greet.sh | grep -qx hello"),
        "{written}"
    );

    // T2 without a verify step does not validate (PLAN_035).
    let invalid = revised.replace(
        "[[task.verify]]\nphase = \"test\"\ncommand = \"test -f config.ini\"\n",
        "",
    );
    assert_ne!(invalid, revised);
    let config = config_answering(temp.path(), &invalid);
    fs::write(workdir.join("roko.toml"), config).expect("write roko.toml");
    let rejected = revise(&workdir, "Drop T2's check");
    assert_eq!(rejected.status.code(), Some(1), "{}", log(&rejected));
    assert!(
        String::from_utf8_lossy(&rejected.stdout).contains("the revision was rejected"),
        "{}",
        log(&rejected)
    );
    assert_eq!(fs::read_to_string(&tasks).expect("read the plan"), written);
}
