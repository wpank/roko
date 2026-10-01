//! Batch integration of a plan run (spec-f830c4, #404).
//!
//! Under `--worktree-per-task` each passed attempt is accepted onto its plan's
//! branch `roko/plan/<plan>` (gap-3b5361). A run also gets one batch branch,
//! `roko/batch/<run-id>`, started at the operator's `HEAD`. Every plan's
//! attempts start from the batch, and each plan whose tasks all passed is
//! delivered into it: [`CliCompletionDeliveryService`] merges the plan
//! branch's verified tip by plumbing and runs the regression check on the
//! merge in the repository's regression checkout, never the operator's.
//! Deliveries run one at a time, since the
//! batch has one tip, so a plan sees every plan delivered before it started.
//!
//! Nothing here checks out, merges or commits in the operator's checkout. The
//! batch branch is never checked out, so it moves only by compare-and-swap.

use std::path::{Path, PathBuf};

use roko_graph::delivery::{
    CompletionDeliveryReceiptV1, CompletionDeliveryRequest, CompletionDeliveryService,
    CompletionDeliveryState, DeliveryError, DeliveryReceiptStore,
};

use super::delivery::{
    CliCompletionDeliveryService, DeliveryBackend, GitDeliveryBackend, is_ancestor,
};
use crate::orchestrator::worktree::format_branch_name;
use crate::runner::merge::git_output;

/// Namespace of the batch branches: `roko/batch/<run-id>`.
pub const BATCH_BRANCH_PREFIX: &str = "roko/batch/";

/// Namespace of the tags on promoted runs: `roko/run/<run-id>`.
pub const RUN_TAG_PREFIX: &str = "roko/run/";

/// Receipt extension recording what a delivered plan's attempt cleanup did
/// (gap-415c54): the checkouts it removed and the branches it kept.
pub const ATTEMPT_CLEANUP_EXTENSION: &str = "roko.attempt_cleanup";

/// What promoting a run's batch did (see [`BatchIntegration::promote`]).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Promotion {
    /// The branch promoted into.
    pub target: String,
    /// The commit the target now has, or would have when it did not move.
    pub commit: String,
    /// Whether the target moved. A target checked out anywhere never moves:
    /// the commit is parked at `refs/roko/delivered/run-<run-id>`, and the
    /// summary says how to take it.
    pub moved: bool,
    /// The annotated tag on `commit`: `roko/run/<run-id>`.
    pub tag: String,
    /// What happened, for the operator.
    pub summary: String,
}

/// The batch branch of one plan run, and the deliveries into it.
#[derive(Debug)]
pub struct BatchIntegration {
    repo: PathBuf,
    run_id: String,
    branch: String,
    /// Where the batch started: the operator's `HEAD` when the run created
    /// it, its tip when a resumed run continued it.
    base_oid: String,
    store: DeliveryReceiptStore,
    /// Deliveries take turns: each merges into the tip the last one left.
    queue: tokio::sync::Mutex<()>,
}

impl BatchIntegration {
    /// Open run `run_id`'s batch branch in `repo`: continue it when it exists
    /// (a resumed run), else create it at `HEAD`, which must not move it if
    /// someone else created it in the meantime.
    ///
    /// # Errors
    ///
    /// `repo` has no `HEAD` commit, or the branch could not be created.
    pub async fn open(repo: &Path, run_id: &str) -> Result<Self, String> {
        let branch = format!("{BATCH_BRANCH_PREFIX}{run_id}");
        let branch_ref = format!("refs/heads/{branch}");
        let base_oid = match commit_of(repo, &branch_ref).await {
            Some(tip) => tip,
            None => {
                let head = commit_of(repo, "HEAD")
                    .await
                    .ok_or_else(|| format!("{} has no HEAD commit", repo.display()))?;
                let absent = "0".repeat(head.len());
                git_output(
                    repo,
                    &[
                        "update-ref",
                        "-m",
                        "roko: start the run's batch",
                        &branch_ref,
                        &head,
                        &absent,
                    ],
                )
                .await
                .map_err(|e| format!("could not create {branch}: {e}"))?;
                head
            }
        };
        Ok(Self {
            repo: repo.to_path_buf(),
            run_id: run_id.to_string(),
            branch,
            base_oid,
            store: DeliveryReceiptStore::new(),
            queue: tokio::sync::Mutex::new(()),
        })
    }

    /// The repository the batch lives in.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// The batch branch, `roko/batch/<run-id>`.
    #[must_use]
    pub fn branch(&self) -> &str {
        &self.branch
    }

    /// The run the batch belongs to.
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// The command that takes the batch into the operator's checkout, which
    /// the run never changes (gap-4ec59f); see [`merge_command`].
    pub async fn merge_command(&self) -> Option<String> {
        merge_command(&self.repo, &self.branch).await
    }

    /// The deliveries' receipts, shared by every service that delivers into
    /// this batch.
    #[must_use]
    pub fn store(&self) -> &DeliveryReceiptStore {
        &self.store
    }

    /// [`Self::record`] of `receipt`, with what the delivered plan's attempt
    /// cleanup did, for the run summary (gap-415c54).
    #[must_use]
    pub fn summary_record(&self, receipt: &CompletionDeliveryReceiptV1) -> serde_json::Value {
        let mut record = self.record(receipt);
        if let Some(cleanup) = receipt.extensions.get(ATTEMPT_CLEANUP_EXTENSION) {
            record["attempt_cleanup"] = cleanup.clone();
        }
        record
    }

    /// The receipt of each delivery into the batch in this process, by plan
    /// id, for the run summary (gap-415c54).
    #[must_use]
    pub fn receipts(&self) -> Vec<CompletionDeliveryReceiptV1> {
        let mut receipts: Vec<_> = self
            .store
            .delivery_ids()
            .iter()
            .filter_map(|id| self.store.get(id))
            .collect();
        receipts.sort_by(|left, right| left.request.plan_id.cmp(&right.request.plan_id));
        receipts
    }

    /// The request that delivers plan `plan_id`'s branch at `verified`, the
    /// commit its accepted attempts left, into the batch.
    #[must_use]
    pub fn request(&self, plan_id: &str, verified: String) -> CompletionDeliveryRequest {
        CompletionDeliveryRequest {
            delivery_id: format!("{}:{plan_id}", self.run_id),
            run_id: self.run_id.clone(),
            plan_id: plan_id.to_string(),
            lease_id: String::new(),
            branch: format_branch_name(plan_id),
            commit_oid: verified,
            target_branch: self.branch.clone(),
            changed_files: Vec::new(),
            publish: false,
        }
    }

    /// Deliver `request` through `service`, which must use this batch's
    /// [`Self::store`], one delivery at a time.
    ///
    /// A `recorded` receipt of the same request, left by an earlier process of
    /// the run, is continued from its next step: a delivered plan is not
    /// merged again, and one that stopped after its merge only reruns its
    /// regression check. A recorded failure, or a receipt of another request,
    /// is delivered afresh. A merge whose regression check failed is taken
    /// back out of the batch.
    ///
    /// # Errors
    ///
    /// The service could not run the delivery at all; a merge conflict or a
    /// failed regression is a terminal receipt, not an error.
    pub async fn deliver(
        &self,
        service: &CliCompletionDeliveryService,
        request: CompletionDeliveryRequest,
        recorded: Option<CompletionDeliveryReceiptV1>,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError> {
        let _turn = self.queue.lock().await;
        let branch_ref = format!("refs/heads/{}", self.branch);
        let before = commit_of(&self.repo, &branch_ref).await;
        let resumable = recorded.filter(|receipt| {
            receipt.request.delivery_id == request.delivery_id
                && receipt.request_fingerprint == request.fingerprint()
                && !receipt.state.is_failed()
        });
        let mut receipt = match resumable {
            None => service.deliver(request).await?,
            Some(receipt) => {
                let delivery_id = receipt.request.delivery_id.clone();
                if self.store.get(&delivery_id).is_none() {
                    self.store.insert_or_get(&receipt.request)?;
                    self.store.update(&receipt);
                }
                service.reconcile(&delivery_id).await?
            }
        };
        // The batch holds only plans that passed their regression check: a
        // merge whose check failed is undone, so no later plan builds on it.
        if receipt.state == CompletionDeliveryState::RegressionFailed
            && let (Some(merge), Some(before)) = (receipt.merge_commit.clone(), before)
            && merge != before
        {
            let undone = git_output(
                &self.repo,
                &[
                    "update-ref",
                    "-m",
                    "roko: undo a delivery whose regression check failed",
                    &branch_ref,
                    &before,
                    &merge,
                ],
            )
            .await;
            let note = match undone {
                Ok(_) => format!("; {} was reset to {before}", self.branch),
                Err(e) => format!("; {} could not be reset to {before}: {e}", self.branch),
            };
            receipt.error = Some(receipt.error.unwrap_or_default() + &note);
            self.store.update(&receipt);
        }
        Ok(receipt)
    }

    /// Promote the batch into `target` once every plan is delivered: merge
    /// its tip with the same plumbing deliveries use (a fast-forward when the
    /// target has not moved), and tag the promoted commit `roko/run/<run-id>`.
    /// Never pushes. A target checked out anywhere is left alone and the
    /// commit parked (see [`Promotion::moved`]).
    ///
    /// # Errors
    ///
    /// The batch has no tip, its merge into `target` conflicts or fails, or
    /// the run's tag already names another commit. No ref moved.
    pub async fn promote(&self, target: &str) -> Result<Promotion, String> {
        let tip = commit_of(&self.repo, &format!("refs/heads/{}", self.branch))
            .await
            .ok_or_else(|| format!("{} does not resolve to a commit", self.branch))?;
        let target_ref = format!("refs/heads/{target}");
        let checked_out = git_output(
            &self.repo,
            &["for-each-ref", "--format=%(worktreepath)", &target_ref],
        )
        .await
        .map_err(|e| format!("could not tell whether {target} is checked out: {e}"))?
        .lines()
        .any(|path| !path.trim().is_empty());
        let park = format!("run-{}", self.run_id);
        let request = CompletionDeliveryRequest {
            delivery_id: format!("{}:promote", self.run_id),
            run_id: self.run_id.clone(),
            plan_id: park.clone(),
            lease_id: String::new(),
            branch: self.branch.clone(),
            commit_oid: tip,
            target_branch: target.to_string(),
            changed_files: Vec::new(),
            publish: false,
        };
        let outcome = GitDeliveryBackend::new(self.repo.clone())
            .merge(&request)
            .await;
        let (commit, moved) = match outcome.merge_commit {
            Some(commit) if outcome.merged => (commit, true),
            _ if checked_out => {
                let parked = commit_of(&self.repo, &format!("refs/roko/delivered/{park}"))
                    .await
                    .ok_or_else(|| outcome.summary.clone())?;
                (parked, false)
            }
            _ => return Err(outcome.summary),
        };
        let tag = format!("{RUN_TAG_PREFIX}{}", self.run_id);
        match commit_of(&self.repo, &format!("refs/tags/{tag}")).await {
            Some(tagged) if tagged == commit => {}
            Some(tagged) => {
                return Err(format!("tag {tag} already names another commit, {tagged}"));
            }
            None => {
                let message = format!(
                    "roko run {}: {} promoted into {target}",
                    self.run_id, self.branch
                );
                git_output(
                    &self.repo,
                    &[
                        "-c",
                        "user.name=roko",
                        "-c",
                        "user.email=roko@localhost",
                        "tag",
                        "--annotate",
                        "--no-sign",
                        "--message",
                        &message,
                        &tag,
                        &commit,
                    ],
                )
                .await
                .map_err(|e| format!("could not tag {commit} as {tag}: {e}"))?;
            }
        }
        Ok(Promotion {
            target: target.to_string(),
            commit,
            moved,
            tag,
            summary: outcome.summary,
        })
    }

    /// The batch record of a plan's delivery, kept in the plan's checkpoint
    /// under `roko.batch@1`.
    #[must_use]
    pub fn record(&self, receipt: &CompletionDeliveryReceiptV1) -> serde_json::Value {
        serde_json::json!({
            "run_id": self.run_id,
            "branch": self.branch,
            "base_oid": self.base_oid,
            "plan_id": receipt.request.plan_id,
            "plan_branch": receipt.request.branch,
            "verified_commit": receipt.request.commit_oid,
            "state": receipt.state,
            "merge_commit": receipt.merge_commit,
            "error": receipt.error,
        })
    }
}

/// The verified commit of plan `plan_id` in `repo`: the tip of its plan
/// branch, which only the acceptance of passed attempts moves (gap-3b5361).
/// `None` when no attempt of the plan was accepted.
pub async fn plan_branch_tip(repo: &Path, plan_id: &str) -> Option<String> {
    commit_of(repo, &format!("refs/heads/{}", format_branch_name(plan_id))).await
}

/// The run whose batch a resumed run continues: the batch an earlier process
/// recorded in the checkpoint of one of `plan_ids`, when its branch still
/// exists in `repo`.
pub async fn resumed_batch_run(repo: &Path, plan_ids: &[&str]) -> Option<String> {
    for plan_id in plan_ids {
        let Some(branch) = crate::graph_checkpoint::recorded_batch_branch(repo, plan_id) else {
            continue;
        };
        let Some(run_id) = branch.strip_prefix(BATCH_BRANCH_PREFIX) else {
            continue;
        };
        if commit_of(repo, &format!("refs/heads/{branch}"))
            .await
            .is_some()
        {
            return Some(run_id.to_string());
        }
    }
    None
}

/// The command that takes batch branch `branch` into the checkout at `repo`
/// (gap-4ec59f): `git merge --ff-only <branch>` while the checkout's `HEAD`
/// is behind the branch, as it is when nothing moved it since the run
/// started, and `git merge <branch>` once it has moved. `None` when `HEAD`
/// already has the branch's tip, or either does not resolve: there is
/// nothing to take.
pub async fn merge_command(repo: &Path, branch: &str) -> Option<String> {
    let tip = format!("refs/heads/{branch}");
    if is_ancestor(repo, &tip, "HEAD").await.ok()? {
        return None;
    }
    Some(if is_ancestor(repo, "HEAD", &tip).await.ok()? {
        format!("git merge --ff-only {branch}")
    } else {
        format!("git merge {branch}")
    })
}

/// A new run id for a batch: when it started, and a random suffix.
#[must_use]
pub fn new_batch_run_id() -> String {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    format!(
        "{}-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        &suffix[..8]
    )
}

/// Whether `repo` is a git checkout with a commit at `HEAD`, which a run's
/// batch branch and per-task worktrees start from (gap-4ec59f).
#[must_use]
pub fn has_head_commit(repo: &Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])
        .current_dir(repo)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The commit `rev` names in `repo`, if it names one.
async fn commit_of(repo: &Path, rev: &str) -> Option<String> {
    let spec = format!("{rev}^{{commit}}");
    git_output(repo, &["rev-parse", "--verify", "--quiet", &spec])
        .await
        .ok()
        .map(|oid| oid.trim().to_string())
        .filter(|oid| !oid.is_empty())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::graph_execution::delivery::{
        DeliveryBackend, DeliveryMergeOutcome, DeliveryPublicationOutcome,
        DeliveryRegressionOutcome, GitDeliveryBackend,
    };

    fn git(repo: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// gap-4ec59f: a workdir can isolate its tasks only as a git checkout
    /// with a commit for the batch and the worktrees to start from.
    #[test]
    fn has_head_commit_needs_a_checkout_with_a_commit() {
        let dir = tempfile::tempdir().expect("dir");
        assert!(!has_head_commit(dir.path()), "not a git checkout");
        git(dir.path(), &["init", "--quiet"]);
        assert!(!has_head_commit(dir.path()), "no commit yet");
        assert!(has_head_commit(repo_with_plan_branches().path()));
    }

    /// A repository on `main`, with plan branches `roko/plan/plan-a` and
    /// `roko/plan/plan-b` that each added a file of their own.
    fn repo_with_plan_branches() -> tempfile::TempDir {
        let repo = tempfile::tempdir().expect("repo");
        let path = repo.path();
        git(path, &["init", "--quiet", "--initial-branch=main"]);
        for (key, value) in [
            ("user.name", "Operator"),
            ("user.email", "operator@example.test"),
            ("commit.gpgsign", "false"),
        ] {
            git(path, &["config", key, value]);
        }
        // Roko's state, the regression checkout included, stays out of git.
        std::fs::write(path.join(".git/info/exclude"), ".roko/\n").expect("exclude");
        std::fs::write(path.join("shared.txt"), "base\n").expect("write");
        git(path, &["add", "-A"]);
        git(path, &["commit", "--quiet", "-m", "base"]);
        for plan in ["plan-a", "plan-b"] {
            git(
                path,
                &[
                    "checkout",
                    "--quiet",
                    "-b",
                    &format!("roko/plan/{plan}"),
                    "main",
                ],
            );
            std::fs::write(path.join(format!("{plan}.txt")), format!("{plan}\n")).expect("write");
            git(path, &["add", "-A"]);
            git(path, &["commit", "--quiet", "-m", plan]);
        }
        git(path, &["checkout", "--quiet", "main"]);
        repo
    }

    fn service(
        batch: &BatchIntegration,
        backend: GitDeliveryBackend,
    ) -> CliCompletionDeliveryService {
        CliCompletionDeliveryService::with_store(batch.store().clone(), Arc::new(backend))
    }

    /// spec-f830c4: finished plans are merged into the run's batch branch, the
    /// second by a real merge, and each merge's regression check runs in the
    /// repository's regression checkout, never the operator's. The
    /// operator's checkout, uncommitted edit and all, and the plan branches
    /// stay as they were.
    #[tokio::test]
    async fn batch_branch_merges_plan_in_temp_worktree() {
        let repo = repo_with_plan_branches();
        let path = repo.path();
        std::fs::write(path.join("shared.txt"), "base\noperator edit\n").expect("edit");
        let (head, status) = (
            git(path, &["rev-parse", "HEAD"]),
            git(path, &["status", "--porcelain"]),
        );
        let record = tempfile::tempdir().expect("record");
        let ran_in = record.path().join("ran-in");
        let batch = BatchIntegration::open(path, "run-1").await.expect("batch");
        assert_eq!(batch.branch(), "roko/batch/run-1");
        assert_eq!(git(path, &["rev-parse", "roko/batch/run-1"]), head);

        let mut receipts = Vec::new();
        for plan in ["plan-b", "plan-a"] {
            let backend = GitDeliveryBackend::new(path.to_path_buf()).with_regression_steps(vec![
                format!("pwd -P >> '{}'", ran_in.display()),
                format!("test -f {plan}.txt"),
            ]);
            let tip = plan_branch_tip(path, plan).await.expect("plan branch");
            let request = batch.request(plan, tip);
            let receipt = batch
                .deliver(&service(&batch, backend), request, None)
                .await
                .expect("delivery runs");
            assert_eq!(
                receipt.state,
                CompletionDeliveryState::Delivered,
                "{:?}",
                receipt.error
            );
            receipts.push(receipt);
        }

        // plan-b fast-forwarded the batch; plan-a was merged into it.
        let tip = git(path, &["rev-parse", "roko/batch/run-1"]);
        assert_eq!(receipts[1].merge_commit.as_deref(), Some(tip.as_str()));
        assert_eq!(
            git(path, &["rev-parse", "roko/batch/run-1^1"]),
            git(path, &["rev-parse", "roko/plan/plan-b"])
        );
        assert_eq!(
            git(path, &["rev-parse", "roko/batch/run-1^2"]),
            git(path, &["rev-parse", "roko/plan/plan-a"])
        );
        let files = git(path, &["ls-tree", "--name-only", "roko/batch/run-1"]);
        assert!(
            files.contains("plan-a.txt") && files.contains("plan-b.txt"),
            "{files}"
        );
        // Each regression ran in the regression checkout (bug-8cf581).
        let checkouts = std::fs::read_to_string(&ran_in).expect("ran");
        let regression_checkout = super::super::delivery::regression_checkout_path(path)
            .canonicalize()
            .expect("canonical");
        for checkout in checkouts.lines().map(PathBuf::from) {
            assert_eq!(checkout, regression_checkout);
        }
        // The operator's checkout never moved.
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        assert_eq!(git(path, &["symbolic-ref", "--short", "HEAD"]), "main");
        assert_eq!(git(path, &["status", "--porcelain"]), status);
        assert!(!path.join("plan-a.txt").exists());
        // The record names the batch and the delivered commit.
        let record = batch.record(&receipts[1]);
        assert_eq!(record["branch"], "roko/batch/run-1");
        assert_eq!(record["base_oid"], head.as_str());
        assert_eq!(record["state"], "delivered");
    }

    /// spec-f830c4: a run creates its batch at `HEAD`, and a resumed run
    /// continues the batch its checkpoints recorded, while that branch exists.
    #[tokio::test]
    async fn batch_branch_is_created_at_head_and_continued_on_resume() {
        let repo = repo_with_plan_branches();
        let path = repo.path();
        let head = git(path, &["rev-parse", "HEAD"]);
        let batch = BatchIntegration::open(path, "run-2").await.expect("batch");
        let tip = plan_branch_tip(path, "plan-a").await.expect("plan branch");
        let backend = GitDeliveryBackend::new(path.to_path_buf()).with_regression_steps(Vec::new());
        let receipt = batch
            .deliver(
                &service(&batch, backend),
                batch.request("plan-a", tip.clone()),
                None,
            )
            .await
            .expect("delivery runs");
        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(git(path, &["rev-parse", "roko/batch/run-2"]), tip);
        assert_ne!(tip, head);

        // Opening the run's batch again continues it where it is.
        let resumed = BatchIntegration::open(path, "run-2").await.expect("batch");
        assert_eq!(git(path, &["rev-parse", "roko/batch/run-2"]), tip);
        assert_eq!(resumed.record(&receipt)["base_oid"], tip.as_str());

        // A resumed run finds the batch its checkpoints recorded.
        for (plan, branch) in [
            ("plan-a", "roko/batch/run-2"),
            ("plan-b", "roko/batch/gone"),
        ] {
            let dir = path.join(".roko/state/graph").join(plan);
            std::fs::create_dir_all(&dir).expect("checkpoint dir");
            let manifest = serde_json::json!({
                "extensions": {"roko.batch@1": {"value": {"branch": branch}}}
            });
            std::fs::write(dir.join("checkpoint.json"), manifest.to_string()).expect("checkpoint");
        }
        assert_eq!(
            resumed_batch_run(path, &["plan-b", "plan-a"])
                .await
                .as_deref(),
            Some("run-2")
        );
        assert_eq!(resumed_batch_run(path, &["plan-b"]).await, None);
    }

    /// spec-f830c4: promoting a delivered batch moves a target that is not
    /// checked out and tags the promoted commit `roko/run/<run-id>`. A target
    /// checked out in the operator's checkout never moves: the promotion is
    /// parked, and still tagged.
    #[tokio::test]
    async fn promotion_moves_the_target_and_tags_the_run() {
        let repo = repo_with_plan_branches();
        let path = repo.path();
        let main = git(path, &["rev-parse", "main"]);
        git(path, &["branch", "release", "main"]);
        for (run, target) in [("run-p1", "release"), ("run-p2", "main")] {
            let batch = BatchIntegration::open(path, run).await.expect("batch");
            let tip = plan_branch_tip(path, "plan-a").await.expect("plan branch");
            let backend =
                GitDeliveryBackend::new(path.to_path_buf()).with_regression_steps(Vec::new());
            let receipt = batch
                .deliver(
                    &service(&batch, backend),
                    batch.request("plan-a", tip.clone()),
                    None,
                )
                .await
                .expect("delivery runs");
            assert_eq!(receipt.state, CompletionDeliveryState::Delivered);

            let promotion = batch.promote(target).await.expect("promotion");

            let tag = format!("roko/run/{run}");
            assert_eq!(promotion.tag, tag);
            assert_eq!(promotion.commit, tip);
            assert_eq!(git(path, &["rev-parse", &format!("{tag}^{{commit}}")]), tip);
            assert_eq!(
                git(path, &["cat-file", "-t", &tag]),
                "tag",
                "an annotated tag"
            );
            if target == "release" {
                assert!(promotion.moved, "{}", promotion.summary);
                assert_eq!(git(path, &["rev-parse", "release"]), tip);
            } else {
                assert!(!promotion.moved, "{}", promotion.summary);
                assert_eq!(git(path, &["rev-parse", "main"]), main);
                let parked = format!("refs/roko/delivered/run-{run}");
                assert_eq!(git(path, &["rev-parse", &parked]), tip);
            }
            // Promoting again changes nothing.
            assert_eq!(batch.promote(target).await.expect("again").commit, tip);
        }
    }

    /// gap-4ec59f: the end of a run names the command that takes its batch
    /// into the operator's checkout: a fast-forward while the checkout is
    /// still behind the batch, a merge once it moved, and nothing once the
    /// checkout has the batch's work or the branch is gone.
    #[tokio::test]
    async fn merge_command_fast_forwards_until_the_checkout_moves() {
        let repo = repo_with_plan_branches();
        let path = repo.path();
        let batch = BatchIntegration::open(path, "run-m").await.expect("batch");
        assert_eq!(batch.merge_command().await, None, "nothing delivered yet");

        let tip = plan_branch_tip(path, "plan-a").await.expect("plan branch");
        let backend = GitDeliveryBackend::new(path.to_path_buf()).with_regression_steps(Vec::new());
        let receipt = batch
            .deliver(
                &service(&batch, backend),
                batch.request("plan-a", tip),
                None,
            )
            .await
            .expect("delivery runs");
        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(
            batch.merge_command().await.as_deref(),
            Some("git merge --ff-only roko/batch/run-m")
        );

        // The operator committed since the run started.
        std::fs::write(path.join("operator.txt"), "operator\n").expect("write");
        git(path, &["add", "operator.txt"]);
        git(path, &["commit", "--quiet", "-m", "operator"]);
        assert_eq!(
            batch.merge_command().await.as_deref(),
            Some("git merge roko/batch/run-m")
        );

        git(path, &["merge", "--quiet", "--no-edit", "roko/batch/run-m"]);
        assert_eq!(batch.merge_command().await, None, "already taken");
        assert_eq!(merge_command(path, "roko/batch/gone").await, None);
    }

    /// Counts the merges and regression checks it is asked for; both pass.
    #[derive(Debug, Default)]
    struct CountingBackend {
        merges: AtomicUsize,
        regressions: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl DeliveryBackend for CountingBackend {
        async fn merge(&self, request: &CompletionDeliveryRequest) -> DeliveryMergeOutcome {
            self.merges.fetch_add(1, Ordering::SeqCst);
            DeliveryMergeOutcome {
                merged: true,
                merge_commit: Some(request.commit_oid.clone()),
                summary: "merged".to_string(),
            }
        }

        async fn run_regression(
            &self,
            _request: &CompletionDeliveryRequest,
            _merge_commit: &str,
        ) -> DeliveryRegressionOutcome {
            self.regressions.fetch_add(1, Ordering::SeqCst);
            DeliveryRegressionOutcome {
                passed: true,
                summary: "passed".to_string(),
                evidence_ref: None,
            }
        }

        async fn publish(
            &self,
            _request: &CompletionDeliveryRequest,
            _merge_commit: &str,
        ) -> DeliveryPublicationOutcome {
            DeliveryPublicationOutcome {
                published: false,
                publication_ref: None,
                summary: "not published".to_string(),
            }
        }
    }

    /// spec-f830c4: a resumed run continues a recorded delivery. One that
    /// stopped after its merge only reruns its regression check, a delivered
    /// one does nothing, and a failed one is delivered afresh.
    #[tokio::test]
    async fn a_resumed_delivery_continues_after_its_merge() {
        let repo = repo_with_plan_branches();
        let batch = BatchIntegration::open(repo.path(), "run-3")
            .await
            .expect("batch");
        let request = batch.request("plan-a", "a".repeat(40));
        let recorded = |states: &[CompletionDeliveryState]| {
            let mut receipt = CompletionDeliveryReceiptV1::prepared(request.clone());
            for state in states {
                receipt.advance(*state).expect("advance");
            }
            receipt.merge_commit = Some("b".repeat(40));
            receipt
        };
        use CompletionDeliveryState::{Conflict, Delivered, Merged, Queued, RegressionPassed};
        for (states, merges, regressions) in [
            (vec![Queued, Merged], 0, 1),
            (vec![Queued, Merged, RegressionPassed, Delivered], 0, 0),
            (vec![Queued, Conflict], 1, 1),
        ] {
            let batch = BatchIntegration::open(repo.path(), "run-3")
                .await
                .expect("batch");
            let backend = Arc::new(CountingBackend::default());
            let service = CliCompletionDeliveryService::with_store(
                batch.store().clone(),
                Arc::clone(&backend) as Arc<dyn DeliveryBackend>,
            );
            let receipt = batch
                .deliver(&service, request.clone(), Some(recorded(&states)))
                .await
                .expect("delivery runs");
            assert_eq!(receipt.state, Delivered, "{states:?}");
            assert_eq!(
                (
                    backend.merges.load(Ordering::SeqCst),
                    backend.regressions.load(Ordering::SeqCst)
                ),
                (merges, regressions),
                "{states:?}"
            );
        }
        drop(batch);
    }
}
