//! Worktree manager — wraps `git worktree add/remove/list` so the
//! orchestrator can give each live plan its own isolated working
//! directory (parity §15).
//!
//! The manager is intentionally small: it shells out to the `git` binary
//! via [`tokio::process::Command`] and tracks active worktrees in an
//! in-memory map guarded by a [`parking_lot::Mutex`]. All git operations
//! are fallible and surface as [`WorktreeError`]. No `crate::*`
//! cross-imports, no hidden filesystem reads — everything the caller
//! needs is a parameter.
//!
//! The manager does **not** persist state to disk, reclaim budget, or
//! perform health checks — those live in higher-level subsystems that
//! compose over this module (see parity §15.3–§15.8).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;

/// Configuration handed to [`WorktreeManager::new`].
#[derive(Debug, Clone)]
pub struct WorktreeConfig {
    /// Absolute path to the main repository checkout. `git worktree`
    /// commands are executed with this as their working directory.
    pub repo_root: PathBuf,
    /// Branch used as the starting point when no explicit HEAD is
    /// provided to `git worktree add`. Mirrors Mori's behaviour of
    /// branching from `main` / the plan base branch.
    pub base_branch: String,
    /// Directory under which new worktrees are materialised. Each
    /// worktree lives at `<worktrees_root>/<id>`.
    pub worktrees_root: PathBuf,
}

/// A live worktree tracked by the manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeHandle {
    /// Stable caller-assigned identifier (typically the plan id).
    pub id: String,
    /// Absolute path to the worktree's checkout directory.
    pub path: PathBuf,
    /// Branch checked out in the worktree.
    pub branch: String,
    /// Unix epoch milliseconds at which the handle was created.
    pub created_at_ms: i64,
}

/// Errors returned by [`WorktreeManager`].
///
/// Every git invocation that exits non-zero surfaces its stderr
/// verbatim via [`WorktreeError::GitFailed`] so callers can log the
/// underlying failure without re-running the command.
#[derive(Debug, Error)]
pub enum WorktreeError {
    /// `git` returned a non-zero exit status. `stderr` is captured
    /// verbatim (lossily decoded if it was not valid UTF-8).
    #[error("git command failed: {stderr}")]
    GitFailed {
        /// Captured stderr from the failing git invocation.
        stderr: String,
    },
    /// Caller asked for a worktree that the manager doesn't track.
    #[error("worktree not found: {0}")]
    NotFound(String),
    /// Caller tried to create a worktree whose id is already live.
    #[error("worktree already exists: {0}")]
    AlreadyExists(String),
    /// Supplied identifier failed validation (empty, path separator, …).
    #[error("invalid worktree id: {0}")]
    InvalidId(String),
    /// Local filesystem error while preparing or removing a worktree.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Manages the lifecycle of per-plan git worktrees.
///
/// Clones of [`WorktreeManager`] share the same internal registry — the
/// handle is cheap to clone and safe to move across tasks.
#[derive(Clone)]
pub struct WorktreeManager {
    config: Arc<WorktreeConfig>,
    active: Arc<Mutex<HashMap<String, WorktreeHandle>>>,
}

impl WorktreeManager {
    /// Construct a new manager. The caller is responsible for making
    /// sure `config.repo_root` is a real git repository — validation is
    /// lazy: the first git command that runs inside the worktree root
    /// will surface any misconfiguration as [`WorktreeError::GitFailed`].
    #[must_use]
    pub fn new(config: WorktreeConfig) -> Self {
        Self {
            config: Arc::new(config),
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Compute the path the manager would use for `id`. This is a pure
    /// function of the config and does not touch the filesystem.
    #[must_use]
    pub fn path_for(&self, id: &str) -> PathBuf {
        self.config.worktrees_root.join(id)
    }

    /// Create a worktree for `id` checked out onto `branch`.
    ///
    /// `branch` is created from [`WorktreeConfig::base_branch`] if it
    /// does not already exist. Rejects duplicate ids and invalid id
    /// strings (empty, containing `/`, `\`, NUL, or leading `.`).
    pub async fn create(&self, id: &str, branch: &str) -> Result<WorktreeHandle, WorktreeError> {
        validate_id(id)?;

        // Reserve the slot up-front so racing callers conflict cleanly.
        {
            let guard = self.active.lock();
            if guard.contains_key(id) {
                return Err(WorktreeError::AlreadyExists(id.to_string()));
            }
        }

        let path = self.path_for(id);

        // Make sure `worktrees_root` exists before git tries to write.
        tokio::fs::create_dir_all(&self.config.worktrees_root).await?;

        let path_str = path.to_string_lossy().into_owned();

        // `git worktree add -B <branch> <path> <base>` creates or resets
        // the branch to match the base, then checks it out into a new
        // worktree. Using `-B` (vs `-b`) keeps the call idempotent at
        // the git level if a stray branch is lying around.
        let output = Command::new("git")
            .current_dir(&self.config.repo_root)
            .args([
                "worktree",
                "add",
                "-B",
                branch,
                &path_str,
                &self.config.base_branch,
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        let handle = WorktreeHandle {
            id: id.to_string(),
            path,
            branch: branch.to_string(),
            created_at_ms: chrono::Utc::now().timestamp_millis(),
        };

        let conflict = {
            let mut guard = self.active.lock();
            // Double-check in case a concurrent caller inserted first.
            if guard.contains_key(id) {
                true
            } else {
                guard.insert(id.to_string(), handle.clone());
                false
            }
        };

        if conflict {
            // Best-effort rollback of the git side; ignore failures
            // since the real winner owns the worktree now.
            let _ = self.git_remove(&handle.path).await;
            return Err(WorktreeError::AlreadyExists(id.to_string()));
        }

        Ok(handle)
    }

    /// Remove the worktree tracked under `id`. Errors if `id` isn't
    /// tracked. The underlying git directory is removed via
    /// `git worktree remove --force` so uncommitted files are cleaned up
    /// along with the metadata.
    pub async fn remove(&self, id: &str) -> Result<(), WorktreeError> {
        let handle = {
            let mut guard = self.active.lock();
            guard
                .remove(id)
                .ok_or_else(|| WorktreeError::NotFound(id.to_string()))?
        };

        if let Err(err) = self.git_remove(&handle.path).await {
            // Restore the registry entry so callers can retry.
            self.active.lock().insert(id.to_string(), handle);
            return Err(err);
        }

        Ok(())
    }

    /// Snapshot of every worktree currently tracked by the manager.
    /// Does **not** consult `git worktree list` — it reports the
    /// in-memory registry only.
    pub fn list(&self) -> Result<Vec<WorktreeHandle>, WorktreeError> {
        let mut out: Vec<WorktreeHandle> = {
            let guard = self.active.lock();
            guard.values().cloned().collect()
        };
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    async fn git_remove(&self, path: &Path) -> Result<(), WorktreeError> {
        let path_str = path.to_string_lossy().into_owned();
        let output = Command::new("git")
            .current_dir(&self.config.repo_root)
            .args(["worktree", "remove", "--force", &path_str])
            .output()
            .await?;

        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<(), WorktreeError> {
    if id.is_empty() {
        return Err(WorktreeError::InvalidId("id is empty".to_string()));
    }
    if id.starts_with('.') || id.starts_with('-') {
        return Err(WorktreeError::InvalidId(format!(
            "id `{id}` may not start with `.` or `-`"
        )));
    }
    if id.contains("..") {
        return Err(WorktreeError::InvalidId(format!(
            "id `{id}` may not contain `..`"
        )));
    }
    // Characters that would break either a filesystem path or a git ref:
    // `/`, `\`, NUL, whitespace, and the git-ref-forbidden set
    // (`~`, `^`, `:`, `?`, `*`, `[`, ASCII control). Also reject `@{`.
    for ch in id.chars() {
        let bad = matches!(
            ch,
            '/' | '\\' | '\0' | '~' | '^' | ':' | '?' | '*' | '[' | '@'
        ) || ch.is_whitespace()
            || ch.is_control();
        if bad {
            return Err(WorktreeError::InvalidId(format!(
                "id `{id}` contains forbidden character `{ch}`"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests below each spin up a throwaway git repo in a
    //! [`tempfile::TempDir`]. Every test that needs `git` first checks
    //! the binary via [`git_available`]; if git isn't on `$PATH`, the
    //! test returns early so `cargo test` still succeeds on machines
    //! without git (for instance minimal CI images).

    #![allow(clippy::unwrap_used)]

    use super::{validate_id, WorktreeConfig, WorktreeError, WorktreeManager};
    use std::path::Path;
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    fn git_available() -> bool {
        StdCommand::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn init_repo(dir: &Path) {
        // Keep the default branch name deterministic across git
        // versions and user configs.
        let status = StdCommand::new("git")
            .current_dir(dir)
            .args(["init", "-b", "main"])
            .status()
            .unwrap();
        assert!(status.success(), "git init failed");

        // Scoped identity so the test's commit is reproducible without
        // touching the user's global git config.
        for (k, v) in [
            ("user.email", "roko@example.test"),
            ("user.name", "Roko Test"),
            ("commit.gpgsign", "false"),
        ] {
            let ok = StdCommand::new("git")
                .current_dir(dir)
                .args(["config", k, v])
                .status()
                .unwrap()
                .success();
            assert!(ok, "git config {k} failed");
        }

        let ok = StdCommand::new("git")
            .current_dir(dir)
            .args(["commit", "--allow-empty", "-m", "init"])
            .status()
            .unwrap()
            .success();
        assert!(ok, "git commit --allow-empty failed");
    }

    fn make_manager() -> Option<(TempDir, WorktreeManager)> {
        if !git_available() {
            eprintln!("skipping: git not on PATH");
            return None;
        }
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        init_repo(&repo_root);
        let worktrees_root = tmp.path().join("worktrees");
        let mgr = WorktreeManager::new(WorktreeConfig {
            repo_root,
            base_branch: "main".to_string(),
            worktrees_root,
        });
        Some((tmp, mgr))
    }

    #[tokio::test]
    async fn create_worktree_materialises_directory() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let handle = mgr.create("01-alpha", "feature/alpha").await.unwrap();
        assert_eq!(handle.id, "01-alpha");
        assert_eq!(handle.branch, "feature/alpha");
        assert!(handle.path.exists(), "worktree dir should exist");
        assert!(handle.path.join(".git").exists(), ".git file expected");
        assert!(handle.created_at_ms > 0);
    }

    #[tokio::test]
    async fn list_returns_every_active_handle_sorted() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("02-bravo", "feature/bravo").await.unwrap();
        mgr.create("01-alpha", "feature/alpha").await.unwrap();
        let listed = mgr.list().unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "01-alpha");
        assert_eq!(listed[1].id, "02-bravo");
    }

    #[tokio::test]
    async fn remove_drops_handle_and_worktree() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let handle = mgr.create("03-charlie", "feature/charlie").await.unwrap();
        assert!(handle.path.exists());
        mgr.remove("03-charlie").await.unwrap();
        assert!(mgr.list().unwrap().is_empty());
        assert!(
            !handle.path.exists(),
            "git worktree remove --force should delete the dir"
        );
    }

    #[tokio::test]
    async fn create_remove_roundtrip_allows_reuse() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("04-delta", "feature/delta").await.unwrap();
        mgr.remove("04-delta").await.unwrap();
        // After removal we must be able to use the id again.
        let h2 = mgr.create("04-delta", "feature/delta").await.unwrap();
        assert_eq!(h2.id, "04-delta");
        assert_eq!(mgr.list().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn remove_nonexistent_is_not_found() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        let err = mgr.remove("nope").await.unwrap_err();
        assert!(matches!(err, WorktreeError::NotFound(ref id) if id == "nope"));
    }

    #[tokio::test]
    async fn duplicate_id_is_rejected() {
        let Some((_tmp, mgr)) = make_manager() else {
            return;
        };
        mgr.create("05-echo", "feature/echo").await.unwrap();
        let err = mgr.create("05-echo", "feature/echo-2").await.unwrap_err();
        assert!(matches!(err, WorktreeError::AlreadyExists(ref id) if id == "05-echo"));
        // Original handle is still there, in registry and on disk.
        assert_eq!(mgr.list().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn path_for_does_not_touch_disk() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorktreeManager::new(WorktreeConfig {
            repo_root: tmp.path().join("repo"),
            base_branch: "main".to_string(),
            worktrees_root: tmp.path().join("worktrees"),
        });
        let p = mgr.path_for("06-foxtrot");
        assert_eq!(p, tmp.path().join("worktrees").join("06-foxtrot"));
        assert!(!p.exists(), "path_for should be pure");
        // No git calls were made, so nothing was created.
        assert!(mgr.list().unwrap().is_empty());
    }

    #[tokio::test]
    async fn invalid_ids_are_rejected_before_git_runs() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorktreeManager::new(WorktreeConfig {
            repo_root: tmp.path().join("repo"),
            base_branch: "main".to_string(),
            worktrees_root: tmp.path().join("worktrees"),
        });
        // Neither repo nor worktrees_root exists — but we never reach
        // git because validate_id rejects these inputs first.
        for bad in [
            "",
            ".hidden",
            "-starts-with-dash",
            "has/slash",
            "has\\back",
            "white space",
            "has..dots",
            "has~tilde",
            "has^caret",
            "has:colon",
            "has?q",
            "has*star",
            "has[bracket",
            "has@at",
            "has\ttab",
        ] {
            let err = mgr.create(bad, "feature/x").await.unwrap_err();
            assert!(
                matches!(err, WorktreeError::InvalidId(_)),
                "expected InvalidId for `{bad}`, got {err:?}"
            );
        }
    }

    #[test]
    fn validate_id_accepts_reasonable_ids() {
        for good in [
            "01-alpha",
            "plan_42",
            "08a-some-thing",
            "nested.branch-name",
        ] {
            validate_id(good).unwrap();
        }
    }
}
