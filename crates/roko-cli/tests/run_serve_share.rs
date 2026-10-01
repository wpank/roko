#![cfg(unix)]

//! `roko run --share` on the Graph engine (gap-9980c6): `--share` starts the
//! control plane `--serve` starts, publishes the run to it, writes the run's
//! shared transcript, and `--effort` reaches the provider.
//!
//! No model runs: the agent is the shared scripted provider, and the run's
//! one task is verified by a declared gate rung, so no build runs either.

mod common;

use std::process::Output;

use assert_cmd::cargo::cargo_bin;
use common::scripted_provider::{Script, Turn};
use common::{ScriptedPlanWorkspace, pick_unused_port};
use serde_json::Value;

/// `roko.toml` addition: the rung every change must pass, which `roko run`
/// makes its task's verify step.
const CONFIG: &str = "gates.rungs = [{ name = \"main\", command = \"test -f src/main.rs\" }]\n";

/// `roko <args>` in the workspace's repository as its user, with the control
/// plane on `port`: provider keys, `ROKO_*` variables and the invoking
/// environment's log and config variables are removed.
fn roko(workspace: &ScriptedPlanWorkspace, port: u16, args: &[&str]) -> Output {
    let mut command = std::process::Command::new(cargo_bin("roko"));
    command
        .current_dir(&workspace.repo)
        .args(args)
        .env("HOME", &workspace.home)
        .env("PORT", port.to_string());
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
    // Warnings and errors on stderr, so a failure explains itself.
    command.env("ROKO_LOG", "warn");
    command.output().expect("run roko")
}

/// Whether a provider call's `argv` asks for `--effort <effort>`.
fn asks_for_effort(argv: &[String], effort: &str) -> bool {
    argv.windows(2)
        .any(|pair| pair[0] == "--effort" && pair[1] == effort)
}

#[test]
fn roko_run_share_serves_shares_and_keeps_its_effort() {
    let (workspace, provider) = ScriptedPlanWorkspace::with_provider(
        "unused",
        "[meta]\nplan = \"unused\"\n",
        &Script::new().otherwise(Turn::reply().append("src/main.rs", "// roko run\n")),
        CONFIG,
    );
    let repo = workspace.repo.display().to_string();

    let output = roko(
        &workspace,
        pick_unused_port(),
        &[
            "--model",
            "scripted",
            "--effort",
            "low",
            "run",
            "--share",
            "--workdir",
            &repo,
            "Append a comment to src/main.rs",
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    // The control plane started beside the run.
    assert!(stdout.contains("live agent output:"), "{stdout}\n{stderr}");
    let shared: Vec<_> = std::fs::read_dir(workspace.repo.join(".roko/shared"))
        .expect("the shared transcripts")
        .map(|entry| entry.expect("shared transcript").path())
        .collect();
    assert_eq!(shared.len(), 1, "{shared:?}");
    let bytes = std::fs::read(&shared[0]).expect("read the transcript");
    let transcript: Value = serde_json::from_slice(&bytes).expect("parse the transcript");
    assert_eq!(transcript["success"], true, "{transcript}");
    assert!(
        transcript["episode_id"]
            .as_str()
            .is_some_and(|run| run.starts_with("run-")),
        "{transcript}"
    );
    let argvs: Vec<_> = provider.calls().into_iter().map(|c| c.argv).collect();
    assert!(
        argvs.iter().any(|argv| asks_for_effort(argv, "low")),
        "no call asked for --effort low: {argvs:?}"
    );
}
