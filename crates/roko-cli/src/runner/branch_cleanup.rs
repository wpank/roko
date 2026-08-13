//! Post-plan branch cleanup — deletes roko-managed branches whose PRs have
//! been merged.
//!
//! Roko creates branches under three prefixes:
//!
//! - `roko/plan/<plan_id>`
//! - `roko/task/<plan_id>/<task_id>`
//! - `roko/attempt/<hash>`
//!
//! After a plan run completes, branches whose associated PRs are merged are
//! stale.  This module enumerates them, checks merge status via `gh pr view`,
//! and deletes both local and remote refs.

use std::path::Path;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Branch prefixes managed by roko.  Any local or remote branch matching one
/// of these prefixes is eligible for cleanup.
const ROKO_BRANCH_PREFIXES: &[&str] = &["roko/plan/", "roko/task/", "roko/attempt/"];

/// Returns `true` when `branch` starts with one of the roko-managed prefixes.
#[must_use]
pub fn is_roko_branch(branch: &str) -> bool {
    ROKO_BRANCH_PREFIXES
        .iter()
        .any(|prefix| branch.starts_with(prefix))
}

/// List all local branches matching roko prefixes.
///
/// Runs `git branch --list 'roko/*'` and filters to the known prefixes.
async fn list_roko_branches(workdir: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "--list", "roko/*"])
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("spawn git branch --list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git branch --list failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().trim_start_matches("* ").to_owned())
        .filter(|b| !b.is_empty() && is_roko_branch(b))
        .collect();

    Ok(branches)
}

/// Check whether a branch has a merged PR on the given remote.
///
/// Uses `gh pr view <branch> --json state` which returns the PR state.
/// Returns `true` only when the state is `"MERGED"`.
async fn branch_pr_is_merged(workdir: &Path, branch: &str) -> bool {
    let output = match Command::new("gh")
        .args(["pr", "view", branch, "--json", "state", "--jq", ".state"])
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(out) => out,
        Err(err) => {
            debug!(branch, error = %err, "gh pr view failed — skipping branch");
            return false;
        }
    };

    if !output.status.success() {
        // No PR exists for this branch, or gh is not installed/authenticated.
        debug!(
            branch,
            stderr = %String::from_utf8_lossy(&output.stderr),
            "no merged PR found for branch"
        );
        return false;
    }

    let state = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_uppercase();
    state == "MERGED"
}

/// Delete a local branch.
async fn delete_local_branch(workdir: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["branch", "-D", branch])
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("spawn git branch -D")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git branch -D {branch} failed: {stderr}");
    }
    Ok(())
}

/// Delete a remote branch.
async fn delete_remote_branch(workdir: &Path, remote: &str, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["push", remote, "--delete", branch])
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("spawn git push --delete")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Remote branch may already be deleted (e.g. GitHub auto-delete on merge).
        if stderr.contains("remote ref does not exist") {
            debug!(branch, remote, "remote branch already deleted");
            return Ok(());
        }
        anyhow::bail!("git push {remote} --delete {branch} failed: {stderr}");
    }
    Ok(())
}

/// Discover and delete roko-managed branches whose PRs have been merged.
///
/// Enumerates local branches under `roko/plan/`, `roko/task/`, and
/// `roko/attempt/`, checks each for a merged PR via `gh pr view`, and
/// deletes both the local and remote refs.
///
/// Returns the list of branch names that were cleaned up.
///
/// # Arguments
///
/// * `workdir` — Path to the git working directory.
/// * `remote` — Name of the git remote (usually `"origin"`).
pub async fn cleanup_merged_branches(workdir: &Path, remote: &str) -> Result<Vec<String>> {
    let branches = list_roko_branches(workdir).await?;

    if branches.is_empty() {
        debug!("no roko-managed branches found — nothing to clean up");
        return Ok(Vec::new());
    }

    info!(
        count = branches.len(),
        "checking roko branches for merged PRs"
    );

    let mut cleaned = Vec::new();

    for branch in &branches {
        if !branch_pr_is_merged(workdir, branch).await {
            debug!(branch, "branch PR not merged — skipping");
            continue;
        }

        info!(branch, "cleaning up merged branch");

        // Delete remote ref first (best-effort).
        if let Err(err) = delete_remote_branch(workdir, remote, branch).await {
            warn!(branch, remote, error = %err, "failed to delete remote branch");
        }

        // Delete local ref.
        match delete_local_branch(workdir, branch).await {
            Ok(()) => {
                info!(branch, "deleted local branch");
                cleaned.push(branch.clone());
            }
            Err(err) => {
                warn!(branch, error = %err, "failed to delete local branch");
            }
        }
    }

    if !cleaned.is_empty() {
        info!(count = cleaned.len(), branches = ?cleaned, "branch cleanup complete");
    }

    Ok(cleaned)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_roko_branch ────────────────────────────────────────────────────

    #[test]
    fn is_roko_branch_recognises_plan_branches() {
        assert!(is_roko_branch("roko/plan/my-plan"));
        assert!(is_roko_branch("roko/plan/01-alpha"));
    }

    #[test]
    fn is_roko_branch_recognises_task_branches() {
        assert!(is_roko_branch("roko/task/plan-1/task-a"));
        assert!(is_roko_branch("roko/task/p/t"));
    }

    #[test]
    fn is_roko_branch_recognises_attempt_branches() {
        assert!(is_roko_branch("roko/attempt/abc123"));
    }

    #[test]
    fn is_roko_branch_rejects_non_roko_branches() {
        assert!(!is_roko_branch("main"));
        assert!(!is_roko_branch("feature/roko-stuff"));
        assert!(!is_roko_branch("roko-legacy/plan/old"));
        assert!(!is_roko_branch(""));
    }

    // ── list_roko_branches ────────────────────────────────────────────────

    #[tokio::test]
    async fn list_roko_branches_in_real_repo() {
        // This test runs against the real repo; it should not fail even if
        // there are no roko/ branches.
        let workdir = std::env::current_dir().unwrap();
        let branches = list_roko_branches(&workdir).await;
        // We just verify it doesn't error — the actual list depends on the
        // repo state.
        assert!(
            branches.is_ok(),
            "listing branches should not fail: {branches:?}"
        );
    }

    // ── cleanup_merged_branches (integration-safe) ────────────────────────

    #[tokio::test]
    async fn cleanup_merged_branches_returns_empty_when_no_branches() {
        // Use a fresh temp git repo that has no roko/* branches.
        let dir = tempfile::TempDir::new().unwrap();
        let workdir = dir.path();

        // Initialize a bare git repo.
        let init = Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(workdir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();
        assert!(init.success());

        // Need at least one commit for branch listing to work.
        let _ = Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(workdir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();

        let result = cleanup_merged_branches(workdir, "origin").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_roko_branches_finds_matching_branches() {
        let dir = tempfile::TempDir::new().unwrap();
        let workdir = dir.path();

        // Init repo with a commit.
        let _ = Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(workdir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();
        let _ = Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(workdir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();

        // Create some roko branches.
        for branch in [
            "roko/plan/test-plan",
            "roko/task/plan/task-1",
            "roko/attempt/abc",
        ] {
            let _ = Command::new("git")
                .args(["branch", branch])
                .current_dir(workdir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .unwrap();
        }

        // Create a non-roko branch.
        let _ = Command::new("git")
            .args(["branch", "feature/unrelated"])
            .current_dir(workdir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();

        let branches = list_roko_branches(workdir).await.unwrap();
        assert_eq!(branches.len(), 3);
        assert!(branches.contains(&"roko/plan/test-plan".to_owned()));
        assert!(branches.contains(&"roko/task/plan/task-1".to_owned()));
        assert!(branches.contains(&"roko/attempt/abc".to_owned()));
        assert!(!branches.contains(&"feature/unrelated".to_owned()));
    }

    #[tokio::test]
    async fn delete_local_branch_removes_branch() {
        let dir = tempfile::TempDir::new().unwrap();
        let workdir = dir.path();

        // Init repo.
        let _ = Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(workdir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();
        let _ = Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(workdir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();

        // Create a branch.
        let _ = Command::new("git")
            .args(["branch", "roko/plan/delete-me"])
            .current_dir(workdir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();

        // Verify it exists.
        let before = list_roko_branches(workdir).await.unwrap();
        assert!(before.contains(&"roko/plan/delete-me".to_owned()));

        // Delete it.
        delete_local_branch(workdir, "roko/plan/delete-me")
            .await
            .unwrap();

        // Verify it's gone.
        let after = list_roko_branches(workdir).await.unwrap();
        assert!(!after.contains(&"roko/plan/delete-me".to_owned()));
    }
}
