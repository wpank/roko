//! CI guard: production code executes plans through the Graph engine.
//!
//! Runner-v2 and its `runner::run` entry point are gone; plan execution goes
//! through `graph_execution::run_graph_plan`. These tests scan the CLI crate
//! source so that neither the legacy `PlanRunner::from_plans_dir` pattern
//! (backlog #131) nor a `runner::run(` call comes back, and pin the Graph
//! engine at every caller that used to reach the removed stub.

use std::fs;
use std::path::{Path, PathBuf};

fn crate_src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn scan_for_pattern(dir: &Path, pattern: &str) -> Vec<(PathBuf, usize, String)> {
    let mut hits = Vec::new();
    scan_dir(dir, pattern, &mut hits);
    hits
}

fn scan_dir(dir: &Path, pattern: &str, hits: &mut Vec<(PathBuf, usize, String)>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip target/ and hidden directories.
            let dominated = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.starts_with('.') || name == "target");
            if !dominated {
                scan_dir(&path, pattern, hits);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for (line_no, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    hits.push((path.clone(), line_no + 1, line.to_string()));
                }
            }
        }
    }
}

/// Ensure `PlanRunner::from_plans_dir` is not called anywhere in the CLI crate.
///
/// The legacy `PlanRunner` has an unbounded `Vec` memory leak and bypasses
/// the Graph engine's safety, learning, and gate wiring. All production paths
/// must use the Graph engine (`cmd_plan_run_engine` in `commands/plan.rs`).
#[test]
fn no_legacy_plan_runner_call_sites() {
    let hits = scan_for_pattern(&crate_src_dir(), "PlanRunner::from_plans_dir");
    if !hits.is_empty() {
        let mut msg = String::from(
            "ERROR: legacy PlanRunner::from_plans_dir call site(s) detected.\n\
             All production paths must use the Graph engine (`cmd_plan_run_engine`).\n\n",
        );
        for (path, line, content) in &hits {
            msg.push_str(&format!(
                "  {}:{}: {}\n",
                path.display(),
                line,
                content.trim()
            ));
        }
        panic!("{msg}");
    }
}

/// Ensure nothing in the CLI crate calls `runner::run(`.
///
/// The Runner-v2 entry point was deleted; while it existed as a stub it made
/// `roko develop`, `roko do`, PRD auto-execution, and the cloud worker fail
/// on every run. Execute plans with `graph_execution::run_graph_plan`.
#[test]
fn no_runner_run_call_sites() {
    let hits = scan_for_pattern(&crate_src_dir(), "runner::run(");
    if !hits.is_empty() {
        let mut msg = String::from(
            "ERROR: `runner::run(` call site(s) detected; Runner-v2 was removed.\n\
             Execute plans with `graph_execution::run_graph_plan`.\n\n",
        );
        for (path, line, content) in &hits {
            msg.push_str(&format!(
                "  {}:{}: {}\n",
                path.display(),
                line,
                content.trim()
            ));
        }
        panic!("{msg}");
    }
}

/// Verify that every production plan-execution caller uses the Graph engine.
///
/// Each of these used to terminate in the deprecated `runner::run` stub,
/// which always returned an error.
#[test]
fn plan_execution_callers_use_graph_engine() {
    for caller in [
        "serve_runtime.rs",   // POST /api/plans/{id}/execute
        "prd.rs",             // PRD auto_plan + auto-execute
        "worker/cloud.rs",    // deployed cloud code-implementer worker
        "commands/do_cmd.rs", // `roko develop` and `roko do --plan`
    ] {
        let path = crate_src_dir().join(caller);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        assert!(
            content.contains("run_graph_plan"),
            "{caller} should execute plans via graph_execution::run_graph_plan"
        );
    }
}
