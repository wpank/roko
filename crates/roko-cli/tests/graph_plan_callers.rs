//! `roko do --plan` and `plan run --log-file` drive the Graph engine end to
//! end.
//!
//! `roko do --plan` generates a plan with the strategist, then executes the
//! plans directory through `commands::do_cmd::run_plan_execution` (the path
//! the removed `roko develop` used). Until the Runner-v2 stub was removed,
//! that execution failed on every run. Every agent here is a mock script,
//! also shadowing `claude`/`codex`/`gemini` on `PATH`, so no model is called.
#![cfg(unix)]

mod common;

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;

/// Error the removed `runner::run` stub returned.
const RUNNER_V2_STUB_ERROR: &str = "legacy Runner-v2 event loop has been removed";

/// Run `roko` in `workdir`, isolated from the user's config and API keys,
/// and kill it after three minutes.
fn run_roko(workdir: &Path, args: &[&str]) -> Output {
    // Shadow every agent CLI with the mock so nothing can reach a real model.
    let shims = workdir.join("agent-shims");
    if !shims.is_dir() {
        std::fs::create_dir_all(&shims).expect("create agent shim dir");
        for name in ["claude", "codex", "gemini"] {
            std::os::unix::fs::symlink(workdir.join("mock-claude.sh"), shims.join(name))
                .expect("link agent shim");
        }
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(std::iter::once(shims).chain(std::env::split_paths(&path)))
        .expect("join PATH");
    let mut child = Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .args(args)
        .env("HOME", workdir)
        .env("PATH", path)
        .env("ROKO_LOG", "error")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("ROKO_FAST_MODE")
        .env_remove("ROKO_EVIDENCE_RUN_ID")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn roko");
    // Drain both pipes while waiting so a chatty run cannot block on a full pipe.
    let stdout = drain(child.stdout.take().expect("stdout pipe"));
    let stderr = drain(child.stderr.take().expect("stderr pipe"));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll roko") {
            break status;
        }
        if start.elapsed() > Duration::from_secs(180) {
            let _ = child.kill();
            break child.wait().expect("reap roko");
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    Output {
        status,
        stdout: stdout.join().expect("stdout reader"),
        stderr: stderr.join().expect("stderr reader"),
    }
}

fn drain(mut pipe: impl std::io::Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// The JSON run summary: the last stdout line that opens an object, through
/// the end of stdout.
fn json_report(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let start = stdout.rfind("\n{").map_or(0, |index| index + 1);
    serde_json::from_str(&stdout[start..]).unwrap_or_else(|err| {
        panic!(
            "parse JSON report: {err}\nstatus: {}\nstdout: {stdout}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_not_runner_v2_stub(output: &Output) {
    for stream in [&output.stdout, &output.stderr] {
        assert!(
            !String::from_utf8_lossy(stream).contains(RUNNER_V2_STUB_ERROR),
            "run reached the removed Runner-v2 stub:\n{}",
            String::from_utf8_lossy(stream)
        );
    }
}

#[test]
fn do_plan_executes_through_graph_engine() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let workdir = tmp.path();
    common::setup_sample_plan_workspace(workdir);

    // The mock strategist writes no plan, so the prepared sample plan in
    // `plans/` is what runs.
    let output = run_roko(
        workdir,
        &[
            "--json",
            "do",
            "--plan",
            "--complexity",
            "standard",
            "--yes",
            "run the prepared plan",
        ],
    );
    assert_not_runner_v2_stub(&output);
    let report = json_report(&output);

    assert_eq!(report["engine"], "graph", "report = {report:#}");
    assert_eq!(report["plan_count"], 1, "report = {report:#}");
    let agent_calls = report["total_agent_calls"].as_u64().unwrap_or(0);
    assert!(
        agent_calls > 0,
        "do --plan should have dispatched the mock agent; report = {report:#}"
    );
    // A finished task (verified or not) records an episode.
    let episodes = std::fs::read_to_string(workdir.join(".roko").join("episodes.jsonl"))
        .expect("episodes.jsonl after do --plan");
    assert!(!episodes.trim().is_empty(), "no episode recorded");
    let succeeded = report["succeeded"].as_bool().expect("succeeded flag");
    assert_eq!(output.status.success(), succeeded, "report = {report:#}");
}

#[test]
fn plan_run_log_file_records_one_bracketed_run() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let workdir = tmp.path();
    common::setup_sample_plan_workspace(workdir);

    let output = run_roko(
        workdir,
        &[
            "--json",
            "plan",
            "run",
            "plans",
            "--no-tui",
            "--log-file",
            "events.jsonl",
        ],
    );
    assert_not_runner_v2_stub(&output);
    let report = json_report(&output);

    let log = std::fs::read_to_string(workdir.join("events.jsonl")).expect("read --log-file");
    let lines: Vec<Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSONL line"))
        .collect();
    let types: Vec<&str> = lines
        .iter()
        .map(|line| line["type"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(types.first(), Some(&"run.started"), "types = {types:?}");
    assert_eq!(types.last(), Some(&"run.completed"), "types = {types:?}");
    assert_eq!(types.iter().filter(|t| **t == "run.started").count(), 1);
    assert_eq!(types.iter().filter(|t| **t == "run.completed").count(), 1);
    for expected in [
        "dashboard.plan_set_loaded",
        "dashboard.plan_started",
        "dashboard.agent_spawned",
        "dashboard.task_completed",
        "dashboard.plan_completed",
    ] {
        assert!(types.contains(&expected), "missing {expected}: {types:?}");
    }
    let run_id = &lines[0]["run_id"];
    assert!(run_id.is_string(), "run.started has no run_id");
    assert!(lines.iter().all(|line| &line["run_id"] == run_id));
    assert_eq!(
        lines[0]["plan_ids"],
        serde_json::json!([common::SAMPLE_PLAN_ID])
    );

    let end = lines.last().expect("terminal line");
    let succeeded = report["succeeded"].as_bool().expect("succeeded flag");
    let expected_outcome = if succeeded { "succeeded" } else { "failed" };
    assert_eq!(end["outcome"], expected_outcome, "end = {end:#}");
    assert_eq!(end["exit_code"], output.status.code().expect("exit code"));
    assert!(
        end["total_agent_calls"].as_u64().unwrap_or(0) > 0,
        "end = {end:#}"
    );
}
