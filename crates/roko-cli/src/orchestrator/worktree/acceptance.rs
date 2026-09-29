//! Accepting an attempt onto its plan branch (gap-3b5361).
//!
//! [`WorktreeManager::accept_attempt`] commits what an attempt's checkout
//! holds on the attempt's own branch, then folds that commit into the plan
//! branch ([`format_branch_name`]) with plumbing only: `merge-tree`,
//! `commit-tree` and a compare-and-swap `update-ref`. No other checkout's
//! branch, index or files ever change.

use std::path::Path;

use super::git_ops::{ISOLATION_DIRS, ensure_git_success};
use super::{
    AcceptedWorktree, AttemptAcceptance, OperationLifecycle, WorktreeError, WorktreeHandle,
    WorktreeManager, format_branch_name,
};
use crate::runner::merge::{MergeTree, merge_tree_result};

/// Identity of the commits acceptance writes: the harness, not the operator,
/// whose identity the managed git commands cannot read (they ignore the
/// global config).
const ACCEPT_IDENTITY: [&str; 4] = ["-c", "user.name=roko", "-c", "user.email=roko@localhost"];

/// Trailer naming the run a commit on a plan branch was accepted in.
const RUN_TRAILER: &str = "Roko-Run: ";

impl WorktreeManager {
    /// Body of [`WorktreeManager::accept_attempt`], run while holding the
    /// manager's operation and the repository's mutation lock.
    pub(super) async fn accept_locked(
        &self,
        plan_id: &str,
        task_id: &str,
        handle: WorktreeHandle,
        acceptance: &AttemptAcceptance,
        lifecycle: &OperationLifecycle,
    ) -> Result<AcceptedWorktree, WorktreeError> {
        self.validate_git_policy(false).await?;
        let attempt_commit = self
            .commit_attempt(&handle, plan_id, task_id, acceptance, lifecycle)
            .await?;
        let commit_oid = self
            .fold_into_plan(plan_id, task_id, &attempt_commit, acceptance, lifecycle)
            .await?;
        let accepted = AcceptedWorktree {
            handle,
            attempt_commit,
            commit_oid,
        };
        self.accepted
            .lock()
            .insert(plan_id.to_string(), accepted.clone());
        Ok(accepted)
    }

    /// Commit everything the attempt's checkout holds on its own branch and
    /// return that commit. A checkout with nothing to commit still gets a
    /// commit, so the acceptance is on record. Leaves out the config copies
    /// roko itself put in the checkout.
    async fn commit_attempt(
        &self,
        handle: &WorktreeHandle,
        plan_id: &str,
        task_id: &str,
        acceptance: &AttemptAcceptance,
        lifecycle: &OperationLifecycle,
    ) -> Result<String, WorktreeError> {
        let checkout = handle.path.as_path();
        let attempt_ref = format!("refs/heads/{}", handle.branch);
        // An agent may have switched the checkout to another branch.
        let head = self
            .git_probe_stdout_at(checkout, &["symbolic-ref", "--quiet", "HEAD"])
            .await
            .unwrap_or_default();
        if head != attempt_ref {
            return Err(WorktreeError::GitFailed {
                stderr: format!(
                    "attempt checkout `{}` is on `{head}`, not on its branch `{}`",
                    handle.id, handle.branch
                ),
            });
        }
        let parent = self
            .git_probe_stdout_at(checkout, &["rev-parse", "--verify", "HEAD^{commit}"])
            .await?;

        let mut excludes = Vec::new();
        for dir in ISOLATION_DIRS {
            let tracked = self
                .git_probe_output_at(checkout, &["cat-file", "-e", &format!("HEAD:{dir}")])
                .await?
                .status
                .success();
            if !tracked {
                excludes.push(format!(":(exclude){dir}"));
            }
        }
        let mut add = vec!["add", "--all", "--", "."];
        add.extend(excludes.iter().map(String::as_str));
        ensure_git_success(self.mutation_at(checkout, &add, lifecycle).await?)?;
        let tree = self
            .mutation_stdout(checkout, &["write-tree"], lifecycle)
            .await?;

        let subject = if acceptance.title.trim().is_empty() {
            format!("roko: {plan_id}/{task_id}")
        } else {
            format!("roko: {plan_id}/{task_id}: {}", acceptance.title.trim())
        };
        let message = with_trailers(&subject, plan_id, task_id, acceptance);
        let mut commit_tree = ACCEPT_IDENTITY.to_vec();
        commit_tree.extend([
            "commit-tree",
            "--no-gpg-sign",
            tree.as_str(),
            "-p",
            parent.as_str(),
            "-m",
            message.as_str(),
        ]);
        let commit = self
            .mutation_stdout(checkout, &commit_tree, lifecycle)
            .await?;
        ensure_git_success(
            self.mutation_at(
                checkout,
                &[
                    "update-ref",
                    "-m",
                    "roko: commit accepted attempt",
                    &attempt_ref,
                    &commit,
                    &parent,
                ],
                lifecycle,
            )
            .await?,
        )?;
        Ok(commit)
    }

    /// Fold `attempt_commit` into the plan branch and return its new tip.
    async fn fold_into_plan(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt_commit: &str,
        acceptance: &AttemptAcceptance,
        lifecycle: &OperationLifecycle,
    ) -> Result<String, WorktreeError> {
        let repo = self.config.repo_root.as_path();
        let branch = format_branch_name(plan_id);
        let plan_ref = format!("refs/heads/{branch}");
        let Some(tip) = self.git_ref_oid(&plan_ref, false).await? else {
            self.move_plan_branch(&plan_ref, attempt_commit, None, lifecycle)
                .await?;
            return Ok(attempt_commit.to_string());
        };
        if let Some(checkout) = self.checked_out_at(&plan_ref).await? {
            return Err(WorktreeError::GitFailed {
                stderr: format!(
                    "`{branch}` is checked out at {checkout}, so roko left it alone (moving it \
                     would put that checkout out of step with its HEAD)"
                ),
            });
        }

        // The first acceptance of the plan in this process continues the
        // branch only when it belongs to the same run (a resumed run), unless
        // the attempt already builds on its tip, which then loses nothing.
        let continuing = self.accepted.lock().contains_key(plan_id);
        let same_run = continuing
            || self.commit_run(&tip).await?.as_deref() == Some(acceptance.run_id.as_str());
        let new_tip = if self.is_ancestor(&tip, attempt_commit).await? {
            attempt_commit.to_string()
        } else if !same_run {
            let archive = format!("refs/roko/plan-archive/{plan_id}/{tip}");
            let zero = "0".repeat(tip.len());
            let archived = self
                .mutation_at(
                    repo,
                    &[
                        "update-ref",
                        "-m",
                        "roko: keep another run's plan branch",
                        &archive,
                        &tip,
                        &zero,
                    ],
                    lifecycle,
                )
                .await?;
            let kept = archived.status.success()
                || self.git_ref_oid(&archive, false).await?.as_deref() == Some(tip.as_str());
            if !kept {
                ensure_git_success(archived)?;
            }
            tracing::info!(
                plan_id,
                branch = %branch,
                archive = %archive,
                "plan branch holds another run's work; kept it and started the branch afresh"
            );
            attempt_commit.to_string()
        } else if self.is_ancestor(attempt_commit, &tip).await? {
            return Ok(tip);
        } else {
            let merge = self
                .mutation_at(
                    repo,
                    &[
                        "merge-tree",
                        "--write-tree",
                        "--name-only",
                        "--no-messages",
                        "-z",
                        "--end-of-options",
                        &tip,
                        attempt_commit,
                    ],
                    lifecycle,
                )
                .await?;
            let tree = match merge_tree_result(&merge) {
                Ok(MergeTree::Clean { tree }) => tree,
                Ok(MergeTree::Conflicted { paths }) => {
                    return Err(WorktreeError::Conflict {
                        branch,
                        paths: paths.join(", "),
                    });
                }
                Err(stderr) => {
                    return Err(WorktreeError::GitFailed {
                        stderr: format!("git merge-tree failed (needs git 2.38+): {stderr}"),
                    });
                }
            };
            let subject = format!("roko: fold {plan_id}/{task_id} into {branch}");
            let message = with_trailers(&subject, plan_id, task_id, acceptance);
            let mut commit_tree = ACCEPT_IDENTITY.to_vec();
            commit_tree.extend([
                "commit-tree",
                "--no-gpg-sign",
                tree.as_str(),
                "-p",
                tip.as_str(),
                "-p",
                attempt_commit,
                "-m",
                message.as_str(),
            ]);
            self.mutation_stdout(repo, &commit_tree, lifecycle).await?
        };
        self.move_plan_branch(&plan_ref, &new_tip, Some(&tip), lifecycle)
            .await?;
        Ok(new_tip)
    }

    /// Move `plan_ref` from `old` (`None`: it must not exist) to `new`.
    async fn move_plan_branch(
        &self,
        plan_ref: &str,
        new: &str,
        old: Option<&str>,
        lifecycle: &OperationLifecycle,
    ) -> Result<(), WorktreeError> {
        let zero = "0".repeat(new.len());
        let old = old.unwrap_or(&zero);
        let moved = self
            .mutation_at(
                &self.config.repo_root,
                &[
                    "update-ref",
                    "-m",
                    "roko: accept attempt",
                    plan_ref,
                    new,
                    old,
                ],
                lifecycle,
            )
            .await?;
        if moved.status.success() {
            return Ok(());
        }
        Err(WorktreeError::GitFailed {
            stderr: format!(
                "`{plan_ref}` moved while roko accepted an attempt, so it was left alone: {}",
                String::from_utf8_lossy(&moved.stderr).trim()
            ),
        })
    }

    /// The path of the worktree that has `branch_ref` checked out, if any.
    async fn checked_out_at(&self, branch_ref: &str) -> Result<Option<String>, WorktreeError> {
        let listed = self
            .git_probe_output_at(
                &self.config.repo_root,
                &["for-each-ref", "--format=%(worktreepath)", branch_ref],
            )
            .await?;
        if !listed.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&listed.stderr).trim().to_string(),
            });
        }
        Ok(String::from_utf8_lossy(&listed.stdout)
            .lines()
            .map(str::trim)
            .find(|path| !path.is_empty())
            .map(ToOwned::to_owned))
    }

    /// The run named by `commit`'s last `Roko-Run` trailer.
    async fn commit_run(&self, commit: &str) -> Result<Option<String>, WorktreeError> {
        let body = self
            .git_probe_stdout_at(&self.config.repo_root, &["cat-file", "commit", commit])
            .await?;
        Ok(body
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix(RUN_TRAILER))
            .map(|run| run.trim().to_string()))
    }

    /// Whether `ancestor` is `descendant` or one of its ancestors.
    async fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, WorktreeError> {
        let output = self
            .git_probe_output_at(
                &self.config.repo_root,
                &["merge-base", "--is-ancestor", ancestor, descendant],
            )
            .await?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            }),
        }
    }

    async fn mutation_at(
        &self,
        current_dir: &Path,
        args: &[&str],
        lifecycle: &OperationLifecycle,
    ) -> Result<std::process::Output, WorktreeError> {
        Ok(self
            .git_mutation_output_at(current_dir, args, lifecycle, true)
            .await?)
    }

    /// Trimmed stdout of a successful mutation.
    async fn mutation_stdout(
        &self,
        current_dir: &Path,
        args: &[&str],
        lifecycle: &OperationLifecycle,
    ) -> Result<String, WorktreeError> {
        let output = self.mutation_at(current_dir, args, lifecycle).await?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// `subject`, then the trailers that tie a commit to its attempt.
fn with_trailers(
    subject: &str,
    plan_id: &str,
    task_id: &str,
    acceptance: &AttemptAcceptance,
) -> String {
    let AttemptAcceptance {
        run_id,
        attempt_key,
        verdict,
        ..
    } = acceptance;
    format!(
        "{subject}\n\n\
         Roko-Plan: {plan_id}\n\
         Roko-Task: {task_id}\n\
         Roko-Attempt: {attempt_key}\n\
         {RUN_TRAILER}{run_id}\n\
         Roko-Verdict: {verdict}\n"
    )
}
