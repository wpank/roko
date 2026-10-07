//! Git plumbing shared by delivery (`graph_execution::delivery`), batch
//! integration (`graph_execution::batch`) and worktree acceptance
//! (`orchestrator::worktree`): a `git` command that ignores the variables
//! pointing git at another repository, its output, and a merge computed in
//! the object database with `git merge-tree`, which never touches a
//! checkout, an index or a ref.
//!
//! This module used to hold `PlanMerger`, Runner v2's merge wrapper around
//! its merge queue. Plans deliver through `GitDeliveryBackend` instead:
//! `PlanMerger` was deleted with its last test callers (gap-3505fb), and the
//! queue itself with the orchestrator snapshot that recorded it (9205).

/// A `git` command run in `workdir`. It drops the variables that point git
/// at another repository, checkout or index, which git sets for hooks.
pub(crate) fn git_command(workdir: &std::path::Path) -> tokio::process::Command {
    let mut command = tokio::process::Command::new("git");
    command
        .current_dir(workdir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    command
}

pub(crate) async fn git_output(workdir: &std::path::Path, args: &[&str]) -> Result<String, String> {
    let output = git_command(workdir)
        .args(args)
        .output()
        .await
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Result of [`git_merge_tree`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MergeTree {
    /// The merge is clean; `tree` is the OID of the merged tree.
    Clean { tree: String },
    /// The merge conflicts in these paths.
    Conflicted { paths: Vec<String> },
}

/// Merge `theirs` into `ours` in the object database with
/// `git merge-tree --write-tree` (git 2.38+).
///
/// Only objects are written: no checkout, index or ref changes. Fails when git
/// cannot compute the merge at all, for example for an unknown revision or a
/// git without `--write-tree`.
pub(crate) async fn git_merge_tree(
    workdir: &std::path::Path,
    ours: &str,
    theirs: &str,
) -> Result<MergeTree, String> {
    let output = git_command(workdir)
        .args([
            "merge-tree",
            "--write-tree",
            "--name-only",
            "--no-messages",
            "-z",
            "--end-of-options",
            ours,
            theirs,
        ])
        .output()
        .await
        .map_err(|err| format!("failed to spawn git merge-tree: {err}"))?;
    merge_tree_result(&output)
}

/// Read the output of `git merge-tree --write-tree --name-only -z`: the merged
/// tree, the conflicted paths, or git's error when it could not merge at all.
pub(crate) fn merge_tree_result(output: &std::process::Output) -> Result<MergeTree, String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut fields = stdout.split('\0').filter(|field| !field.is_empty());
    let tree = fields
        .next()
        .filter(|tree| tree.bytes().all(|byte| byte.is_ascii_hexdigit()));
    match (output.status.code(), tree) {
        (Some(0), Some(tree)) => Ok(MergeTree::Clean {
            tree: tree.to_string(),
        }),
        // Exit 1 also means "not something we can merge"; only a conflict
        // prints the merged tree first.
        (Some(1), Some(_)) => Ok(MergeTree::Conflicted {
            paths: fields.map(ToOwned::to_owned).collect(),
        }),
        _ => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
    }
}
