//! Task-level helpers (originally extracted from the legacy orchestrator).
//!
//! This module contains:
//! - Crate derivation from file paths (`crate_name_for_path`, `task_target_crates`)

use std::collections::HashSet;

// ── Crate helpers ─────────────────────────────────────────────────────

/// Derive a crate name from the task's modified files.
pub(crate) fn task_crate_name(task_def: Option<&crate::task_parser::TaskDef>) -> Option<String> {
    let mut seen = HashSet::new();
    task_def
        .into_iter()
        .flat_map(|task| task.files.iter())
        .filter_map(|file| crate_name_for_path(file))
        .find(|crate_name| seen.insert(crate_name.clone()))
}

/// Collect all distinct crate names from a task's modified files.
///
/// Used by gate dispatch to scope `cargo check` / `cargo clippy` to only
/// the crates a task touches, avoiding false failures from pre-existing
/// errors in unrelated crates.
pub(crate) fn task_target_crates(task_def: Option<&crate::task_parser::TaskDef>) -> Vec<String> {
    let mut seen = HashSet::new();
    task_def
        .into_iter()
        .flat_map(|task| task.files.iter())
        .filter_map(|file| crate_name_for_path(file))
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

/// Best-effort crate key derivation from a repository-relative file path.
pub(crate) fn crate_name_for_path(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let parts: Vec<&str> = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    match parts.as_slice() {
        [first, second, ..] if *first == "crates" || *first == "apps" => {
            Some((*second).to_string())
        }
        [first, second, ..] if matches!(*second, "src" | "tests" | "benches") => {
            Some((*first).to_string())
        }
        [first, ..] if matches!(*first, "src" | "tests" | "benches") => {
            Some("workspace".to_string())
        }
        [first, ..] if *first == "Cargo.toml" => Some("workspace".to_string()),
        _ => None,
    }
}
