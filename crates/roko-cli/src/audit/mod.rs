//! The audit worker's checks (S05 §4.3): they re-check a selected green
//! attempt away from the agent, in a worktree of the audit vault.
//!
//! - [`worktree`]: the audit worktree of a selection's result tree, with the
//!   visible tests the attempt changed restored from its base (7122);
//! - [`rerun`]: A2, the task's checks run three times in that worktree
//!   (7122);
//! - [`worker`]: each run's audit worker, which takes the run's selected
//!   units one at a time, runs phase A and phase B on them, and appends
//!   their `audit.result` (7123).
//!
//! Selection (DP1) is `graph_task_dispatch::audit_select`; the lottery, the
//! ledger, the hidden-suite store and the canary scanner are
//! `roko_gate::audit`.

pub mod rerun;
pub mod worker;
pub mod worktree;

use std::path::Path;

/// Git in `repo` with a fixed identity and no signing, so a commit made from
/// a tree does not depend on the operator's config, and no prompt.
pub(crate) fn git(repo: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .env("GIT_AUTHOR_NAME", "roko-audit")
        .env("GIT_AUTHOR_EMAIL", "audit@roko.invalid")
        .env("GIT_COMMITTER_NAME", "roko-audit")
        .env("GIT_COMMITTER_EMAIL", "audit@roko.invalid")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git {} failed in {}: {}",
            args.join(" "),
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
