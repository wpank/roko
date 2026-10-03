//! The audit worker's checks (S05 §4.3): they re-check a selected green
//! attempt away from the agent, in a worktree of the audit vault.
//!
//! - [`worktree`]: the audit worktree of a selection's result tree, with the
//!   visible tests the attempt changed restored from its base (7122);
//! - [`rerun`]: A2, the task's checks run three times in that worktree
//!   (7122);
//! - [`worker`]: each run's audit worker, which takes the run's selected
//!   units one at a time, runs phase A and phase B on them, and appends
//!   their `audit.result` (7123);
//! - [`b1`]: B1, a hidden suite written by a model of another family,
//!   validated on the base tree and run on the result (7124);
//! - [`b2`]: B2, extreme mutants of the changed Rust functions (7125);
//! - [`b3`]: B3, a review by a model of another family, which only
//!   corroborates (7127);
//! - [`labels`]: DP5, each audited attempt's `vs.label` row, which teaches
//!   the run's self-model with weight 1/π (7134).
//!
//! Selection (DP1) is `graph_task_dispatch::audit_select`; the lottery, the
//! ledger, the hidden-suite store and the canary scanner are
//! `roko_gate::audit`.

pub mod b1;
pub mod b2;
pub mod b3;
pub mod labels;
pub mod rerun;
pub mod worker;
pub mod worktree;

use std::path::Path;

/// Git in `repo` with a fixed identity, no signing and no colour, so a
/// commit made from a tree and a diff's text do not depend on the
/// operator's config, and no prompt.
pub(crate) fn git(repo: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "commit.gpgsign=false", "-c", "color.ui=never"])
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
