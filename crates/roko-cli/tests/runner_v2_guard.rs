//! CI guard: ensure no production call sites use the legacy `PlanRunner`
//! (backlog #131).
//!
//! Both `prd.rs::run_generated_plans()` and `serve_runtime.rs` have been
//! migrated to call `crate::runner::run` (runner-v2). This test statically
//! verifies that no new call sites for the legacy `PlanRunner::from_plans_dir`
//! pattern appear in the CLI crate source.
//!
//! The test scans Rust source files for the pattern `PlanRunner::from_plans_dir`
//! and fails if any call site is found (the definition itself is excluded).

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
/// runner-v2 safety, learning, and gate wiring. All production paths must
/// use `crate::runner::run` (runner-v2) instead.
#[test]
fn no_legacy_plan_runner_call_sites() {
    let hits = scan_for_pattern(&crate_src_dir(), "PlanRunner::from_plans_dir");
    if !hits.is_empty() {
        let mut msg = String::from(
            "ERROR: legacy PlanRunner::from_plans_dir call site(s) detected.\n\
             All production paths must use runner-v2 (`crate::runner::run`).\n\n",
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

/// Verify that `prd.rs` uses `crate::runner::run` for plan execution.
#[test]
fn prd_uses_runner_v2() {
    let prd_path = crate_src_dir().join("prd.rs");
    assert!(prd_path.exists(), "prd.rs should exist");
    let content = fs::read_to_string(&prd_path).expect("read prd.rs");
    assert!(
        content.contains("crate::runner::run"),
        "prd.rs::run_generated_plans should call crate::runner::run (runner-v2)"
    );
}

/// Verify that `serve_runtime.rs` uses `crate::runner::run` for plan execution.
#[test]
fn serve_runtime_uses_runner_v2() {
    let serve_path = crate_src_dir().join("serve_runtime.rs");
    assert!(serve_path.exists(), "serve_runtime.rs should exist");
    let content = fs::read_to_string(&serve_path).expect("read serve_runtime.rs");
    assert!(
        content.contains("crate::runner::run"),
        "serve_runtime.rs should call crate::runner::run (runner-v2)"
    );
}
