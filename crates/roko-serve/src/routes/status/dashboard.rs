//! Dashboard, session status, operation status, and truth map endpoints.

use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::state::AppState;
use roko_runtime::process::{ProcessSessionLedger, default_process_session_ledger_path};

/// `GET /api/dashboard` — dashboard scaffold as JSON.
pub async fn dashboard(State(state): State<Arc<AppState>>) -> Json<Value> {
    let info = state.runtime.dashboard_scaffold(&state.workdir);
    Json(json!({ "rendered": info.rendered }))
}

/// `GET /api/status` — session status overview.
pub async fn session_status(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let ss = state.runtime.session_status(state.workdir.clone());
    let supervised_processes = state.supervisor.snapshots().await;
    let process_ledger_path = default_process_session_ledger_path(&state.workdir);
    let process_sessions = {
        let ledger = ProcessSessionLedger::load(&process_ledger_path).map_err(|err| {
            ApiError::internal(format!(
                "load process session ledger {}: {err}",
                process_ledger_path.display()
            ))
        })?;
        // load() returns a default (empty) ledger when the file does not exist,
        // so we always have a valid summary — no separate existence check needed.
        Some(ledger.state_summary(Some(24 * 60 * 60 * 1_000), unix_ms()))
    };
    let branch = git_branch(&state.workdir);
    Ok(Json(json!({
        "session_id": ss.session_id,
        "workdir": ss.workdir,
        "git_branch": branch,
        "daemon_running": ss.daemon_running,
        "signal_count": ss.signal_count,
        "episode_count": ss.episode_count,
        "last_episode_passed": ss.last_episode_passed,
        "supervised_processes": supervised_processes,
        "process_session_ledger": process_ledger_path,
        "process_sessions": process_sessions,
    })))
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis().min(u128::from(u64::MAX))).unwrap_or(u64::MAX)
        })
}

/// Read the current git branch from `<workdir>/.git/HEAD` without spawning a process.
///
/// Returns `Some(name)` when HEAD contains `ref: refs/heads/<name>`.
/// Returns `None` for:
/// - a detached HEAD (bare commit hash),
/// - a missing `.git` entry,
/// - a worktree whose `.git` is a file (linked worktrees created by `git worktree add`).
fn git_branch(workdir: &Path) -> Option<String> {
    let git_dir = workdir.join(".git");
    // Linked worktrees have `.git` as a file, not a directory.
    // In that case the actual gitdir is elsewhere and we can't reliably locate HEAD
    // without parsing the file, so we report null instead.
    if !git_dir.is_dir() {
        return None;
    }
    let content = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    // Symbolic ref format: "ref: refs/heads/<name>\n"
    content
        .trim()
        .strip_prefix("ref: refs/heads/")
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn git_branch_normal_head() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        assert_eq!(git_branch(dir.path()), Some("main".to_owned()));
    }

    #[test]
    fn git_branch_feature_branch() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/wip/portal-backend\n").unwrap();

        assert_eq!(
            git_branch(dir.path()),
            Some("wip/portal-backend".to_owned())
        );
    }

    #[test]
    fn git_branch_detached_head() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        // Detached HEAD: content is a bare commit hash.
        fs::write(
            git_dir.join("HEAD"),
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2\n",
        )
        .unwrap();

        assert_eq!(git_branch(dir.path()), None);
    }

    #[test]
    fn git_branch_no_repository() {
        let dir = tempfile::tempdir().unwrap();
        // No .git directory at all.
        assert_eq!(git_branch(dir.path()), None);
    }

    #[test]
    fn git_branch_git_is_file_linked_worktree() {
        let dir = tempfile::tempdir().unwrap();
        // In a linked worktree, .git is a file, not a directory.
        fs::write(
            dir.path().join(".git"),
            "gitdir: /some/other/path/.git/worktrees/wt\n",
        )
        .unwrap();

        assert_eq!(git_branch(dir.path()), None);
    }
}

/// `GET /api/operations/:id` — look up a background operation by ID.
///
/// Returns `{ "id", "kind", "status": "running"|"completed"|"failed",
/// "result"?, "error"? }` (section 6, P-2 of the portal contract).
/// `result` is parsed from the stored JSON string back into an object so
/// the portal can read its fields without a second parse step.
pub async fn operation_status(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    use crate::state::OperationStatus;

    let ops = state.operations.read().await;
    let handle = ops
        .get(&id)
        .ok_or_else(|| ApiError::not_found("operation not found"))?;

    let (status_str, result_val, error_val) = match &handle.status {
        OperationStatus::Running => ("running", Value::Null, Value::Null),
        OperationStatus::Completed { result } => {
            // Parse the stored JSON string back into an object when possible.
            let parsed = result
                .as_deref()
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .unwrap_or(Value::Null);
            ("completed", parsed, Value::Null)
        }
        OperationStatus::Failed { error } => ("failed", Value::Null, Value::String(error.clone())),
    };

    let mut response = json!({
        "id": id,
        "kind": handle.kind,
        "status": status_str,
    });
    if !result_val.is_null() {
        response["result"] = result_val;
    }
    if !error_val.is_null() {
        response["error"] = error_val;
    }
    drop(ops);
    Ok(Json(response))
}

/// `GET /api/truth_map` — return the entity truth-source registry.
pub async fn truth_map_handler() -> Json<Value> {
    Json(serde_json::to_value(crate::truth_map::truth_map()).unwrap_or_default())
}
