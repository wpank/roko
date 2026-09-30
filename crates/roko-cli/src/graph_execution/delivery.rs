//! CLI adapter implementing [`CompletionDeliveryService`] for graph-executed plans.
//!
//! This module bridges the graph-layer delivery port to git: a plumbing merge
//! into the target branch, a post-merge regression check in a temporary
//! checkout, and a `git push` for publication. None of them touches the
//! user's checkout (see [`GitDeliveryBackend`]).
//!
//! The service advances through the fixed state sequence:
//!   `Prepared -> Queued -> Merged -> RegressionPassed -> Published -> Delivered`
//!
//! When `publish=false`, the `Published` state is skipped. Conflict, regression
//! failure, and publication failure are terminal. The service writes the
//! `roko.delivery@1` checkpoint extension before queue submission and after
//! every state transition so resume can continue at the first unproved step.
//!
//! The adapter never writes the execution terminal state or releases a workspace
//! lease -- those are outer-controller concerns.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use roko_graph::delivery::{
    CompletionDeliveryReceiptV1, CompletionDeliveryRequest, CompletionDeliveryService,
    CompletionDeliveryState, DELIVERY_EXTENSION_KEY, DeliveryError, DeliveryReceiptStore,
    delivery_extension_value,
};

#[cfg(test)]
use roko_graph::delivery::ReleasePolicy;
use tracing::{debug, info, warn};

use crate::runner::gate_dispatch::RegisteredBaselineWorktree;
use crate::runner::merge::{MergeTree, git_command, git_merge_tree, git_output};

// ---------------------------------------------------------------------------
// Merge backend port (subset of runner::merge for this service)
// ---------------------------------------------------------------------------

/// Outcome of a merge-phase operation.
#[derive(Debug, Clone)]
pub struct DeliveryMergeOutcome {
    /// Whether the merge succeeded.
    pub merged: bool,
    /// Git OID of the merge commit (if successful).
    pub merge_commit: Option<String>,
    /// Human-readable summary.
    pub summary: String,
}

/// Outcome of a regression-phase operation.
#[derive(Debug, Clone)]
pub struct DeliveryRegressionOutcome {
    /// Whether the regression gate passed.
    pub passed: bool,
    /// Human-readable summary.
    pub summary: String,
    /// Optional path to regression evidence (log file, etc.).
    pub evidence_ref: Option<String>,
}

/// Outcome of a publication-phase operation.
#[derive(Debug, Clone)]
pub struct DeliveryPublicationOutcome {
    /// Whether publication succeeded.
    pub published: bool,
    /// Remote reference (PR URL, branch push ref, etc.).
    pub publication_ref: Option<String>,
    /// Human-readable summary.
    pub summary: String,
}

/// Backend trait for the merge/regression/publication phases.
///
/// This lets tests inject fakes without depending on git or GitHub.
#[async_trait::async_trait]
pub trait DeliveryBackend: Send + Sync + std::fmt::Debug {
    /// Apply the merge: merge the request's verified commit (`commit_oid`)
    /// into the target branch.
    async fn merge(&self, request: &CompletionDeliveryRequest) -> DeliveryMergeOutcome;

    /// Run the post-merge regression gate against `merge_commit`.
    async fn run_regression(
        &self,
        request: &CompletionDeliveryRequest,
        merge_commit: &str,
    ) -> DeliveryRegressionOutcome;

    /// Publish to GitHub (push branch, create/update PR, etc.).
    async fn publish(
        &self,
        request: &CompletionDeliveryRequest,
        merge_commit: &str,
    ) -> DeliveryPublicationOutcome;
}

// ---------------------------------------------------------------------------
// Git backend
// ---------------------------------------------------------------------------

/// Production backend: merges with git plumbing, runs the regression check in
/// a temporary detached worktree, and publishes with `git push`.
///
/// It never runs `checkout`, `switch`, `merge` or `reset` in `workdir`, and
/// never changes the files, index or HEAD of any existing worktree:
///
/// - The merge is computed in the object database (`merge-tree`,
///   `commit-tree`), and the target branch moves only by compare-and-swap
///   (`update-ref <ref> <new> <old>`). A target that moved in the meantime
///   is left alone.
/// - A target that is checked out anywhere never moves, because its checkout
///   would no longer match its HEAD. The merge fails closed, and the result
///   is parked at `refs/roko/delivered/<plan_id>`.
///
/// The service's merge slot serializes deliveries; the compare-and-swap
/// guards against every other writer.
#[derive(Debug)]
pub struct GitDeliveryBackend {
    workdir: PathBuf,
    regression_command: Vec<String>,
}

impl GitDeliveryBackend {
    /// Create a git-backed delivery backend for the repository at `workdir`.
    #[must_use]
    pub fn new(workdir: PathBuf) -> Self {
        Self {
            workdir,
            regression_command: ["cargo", "check", "--workspace", "--quiet"]
                .map(String::from)
                .into(),
        }
    }

    /// Replace the post-merge regression command (by default
    /// `cargo check --workspace --quiet`). It runs in a temporary checkout of
    /// the merge commit.
    #[must_use]
    pub fn with_regression_command(mut self, command: Vec<String>) -> Self {
        self.regression_command = command;
        self
    }

    /// Merge the request's verified commit into its target with git plumbing
    /// only.
    ///
    /// The merged commit is `request.commit_oid`, the one the plan's gates
    /// verified, never whatever the branch holds now: commits added to the
    /// branch after verification wait for a delivery of their own. The
    /// verified commit must be on the branch; a branch rewritten since
    /// verification fails closed.
    ///
    /// `Err` carries the summary of a merge that did not happen. Apart from
    /// the parked result for a checked-out target, no ref has changed.
    async fn git_merge(
        &self,
        request: &CompletionDeliveryRequest,
    ) -> Result<DeliveryMergeOutcome, String> {
        let workdir = self.workdir.as_path();
        let (branch, target) = (&request.branch, &request.target_branch);
        let target_ref = format!("refs/heads/{target}");
        let old = resolve_commit(workdir, &target_ref)
            .await
            .map_err(|e| format!("target branch '{target}' does not resolve to a commit: {e}"))?;
        let head = resolve_commit(workdir, branch)
            .await
            .map_err(|e| format!("branch '{branch}' does not resolve to a commit: {e}"))?;
        let theirs = verified_commit(workdir, request, &head).await?;
        let later = if theirs == head {
            String::new()
        } else {
            format!(
                "; branch '{branch}' has moved on to {head}, and its later commits wait for a \
                 verified delivery of their own"
            )
        };

        if is_ancestor(workdir, &theirs, &old).await? {
            return Ok(DeliveryMergeOutcome {
                merged: true,
                merge_commit: Some(old),
                summary: format!(
                    "'{target}' already contains {theirs} of branch '{branch}'{later}"
                ),
            });
        }
        let merge = if is_ancestor(workdir, &old, &theirs).await? {
            theirs.clone()
        } else {
            let tree = match git_merge_tree(workdir, &old, &theirs).await {
                Ok(MergeTree::Clean { tree }) => tree,
                Ok(MergeTree::Conflicted { paths }) => {
                    return Err(format!(
                        "merge conflict: branch '{branch}' into '{target}'; conflicted paths: {}",
                        paths.join(", ")
                    ));
                }
                Err(e) => return Err(format!("git merge-tree failed (needs git 2.38+): {e}")),
            };
            let message = format!("Merge {theirs} of branch '{branch}' into {target}");
            git_output(
                workdir,
                &[
                    "commit-tree",
                    &tree,
                    "-p",
                    &old,
                    "-p",
                    &theirs,
                    "-m",
                    &message,
                ],
            )
            .await
            .map_err(|e| format!("git commit-tree failed: {e}"))?
            .trim()
            .to_string()
        };

        if let Some(checkout) = checked_out_at(workdir, &target_ref).await? {
            return Err(park_merge(workdir, request, &checkout, &merge).await);
        }

        let reflog = format!("roko delivery: merge {branch}");
        git_output(
            workdir,
            &["update-ref", "-m", &reflog, &target_ref, &merge, &old],
        )
        .await
        .map_err(|e| format!("'{target}' moved during the merge, so it was left alone: {e}"))?;
        Ok(DeliveryMergeOutcome {
            merged: true,
            merge_commit: Some(merge),
            summary: format!("merged {theirs} of branch '{branch}' into '{target}'{later}"),
        })
    }

    /// Run the regression command in a temporary detached checkout of
    /// `commit`, never in `workdir`, and remove that checkout afterwards.
    async fn regression_output(&self, commit: &str) -> Result<std::process::Output, String> {
        let (program, args) = self
            .regression_command
            .split_first()
            .ok_or("the regression command is empty")?;
        let parent = tempfile::Builder::new()
            .prefix("roko-delivery-regression-")
            .tempdir()
            .map_err(|e| format!("failed to create a regression checkout: {e}"))?;
        let mut scratch = RegisteredBaselineWorktree::new(&self.workdir, parent);
        let added = git_command(&self.workdir)
            .args(["worktree", "add", "--detach"])
            .arg(&scratch.checkout)
            .arg(commit)
            .output()
            .await
            .map_err(|e| format!("failed to spawn git worktree add: {e}"))?;
        if !added.status.success() {
            return Err(format!(
                "failed to check out {commit} for the regression: {}",
                String::from_utf8_lossy(&added.stderr).trim()
            ));
        }
        scratch.cleanup_required = true;

        let output = tokio::process::Command::new(program)
            .args(args)
            .current_dir(&scratch.checkout)
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| format!("failed to spawn regression gate: {e}"));
        let removed = git_command(&self.workdir)
            .args(["worktree", "remove", "--force"])
            .arg(&scratch.checkout)
            .output()
            .await;
        if removed.is_ok_and(|out| out.status.success()) {
            scratch.cleanup_required = false;
        }
        output
    }
}

/// The commit that `rev` names.
async fn resolve_commit(workdir: &Path, rev: &str) -> Result<String, String> {
    let spec = format!("{rev}^{{commit}}");
    git_output(
        workdir,
        &["rev-parse", "--verify", "--end-of-options", &spec],
    )
    .await
    .map(|oid| oid.trim().to_string())
}

/// The commit `request` verified, resolved in `workdir`: `request.commit_oid`
/// must name a commit, by its id, that is on the request's branch, whose head
/// is `head`.
async fn verified_commit(
    workdir: &Path,
    request: &CompletionDeliveryRequest,
    head: &str,
) -> Result<String, String> {
    let (oid, branch) = (&request.commit_oid, &request.branch);
    if oid.is_empty() || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "the verified commit '{oid}' is not a commit id, so nothing was merged"
        ));
    }
    let verified = resolve_commit(workdir, oid).await.map_err(|e| {
        format!("the verified commit '{oid}' does not resolve, so nothing was merged: {e}")
    })?;
    if !is_ancestor(workdir, &verified, head).await? {
        return Err(format!(
            "the verified commit {verified} is not on branch '{branch}' (its head is {head}): \
             the branch was rewritten after verification, so nothing was merged"
        ));
    }
    Ok(verified)
}

/// Whether `ancestor` is `descendant` or one of its ancestors.
async fn is_ancestor(workdir: &Path, ancestor: &str, descendant: &str) -> Result<bool, String> {
    let output = git_command(workdir)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .output()
        .await
        .map_err(|e| format!("failed to spawn git merge-base: {e}"))?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "git merge-base --is-ancestor failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

/// The path of the worktree that has `branch_ref` checked out, if any.
async fn checked_out_at(workdir: &Path, branch_ref: &str) -> Result<Option<String>, String> {
    let listed = git_output(
        workdir,
        &["for-each-ref", "--format=%(worktreepath)", branch_ref],
    )
    .await
    .map_err(|e| format!("could not tell whether {branch_ref} is checked out: {e}"))?;
    Ok(listed
        .lines()
        .map(str::trim)
        .find(|path| !path.is_empty())
        .map(ToOwned::to_owned))
}

/// Park `merge` for a target that is checked out at `checkout`, and say how
/// to take it. The target branch itself never moves.
async fn park_merge(
    workdir: &Path,
    request: &CompletionDeliveryRequest,
    checkout: &str,
    merge: &str,
) -> String {
    let target = &request.target_branch;
    let parked = format!("refs/roko/delivered/{}", request.plan_id);
    let left_alone = format!(
        "target branch '{target}' is checked out at {checkout}, so it was left alone \
         (moving it would put that checkout out of step with its HEAD)"
    );
    // Compare-and-swap against the parked ref's current value, or require
    // that it does not exist yet.
    let current = git_output(workdir, &["rev-parse", "--verify", "--quiet", &parked])
        .await
        .map_or_else(|_| "0".repeat(merge.len()), |oid| oid.trim().to_string());
    let parked_write = git_output(
        workdir,
        &[
            "update-ref",
            "-m",
            "roko delivery: park merge",
            &parked,
            merge,
            &current,
        ],
    )
    .await;
    match parked_write {
        Ok(_) => format!(
            "{left_alone}; the merge is parked at {parked} ({merge}): run \
             `git merge --ff-only {parked}` in that checkout to take it"
        ),
        Err(e) => format!(
            "{left_alone}; the merge is commit {merge} (writing {parked} failed: {e}): run \
             `git merge --ff-only {merge}` in that checkout to take it"
        ),
    }
}

#[async_trait::async_trait]
impl DeliveryBackend for GitDeliveryBackend {
    async fn merge(&self, request: &CompletionDeliveryRequest) -> DeliveryMergeOutcome {
        self.git_merge(request)
            .await
            .unwrap_or_else(|summary| DeliveryMergeOutcome {
                merged: false,
                merge_commit: None,
                summary,
            })
    }

    async fn run_regression(
        &self,
        request: &CompletionDeliveryRequest,
        merge_commit: &str,
    ) -> DeliveryRegressionOutcome {
        let plan_id = &request.plan_id;
        match self.regression_output(merge_commit).await {
            Ok(output) if output.status.success() => DeliveryRegressionOutcome {
                passed: true,
                summary: format!("post-merge regression passed for {plan_id}"),
                evidence_ref: None,
            },
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                DeliveryRegressionOutcome {
                    passed: false,
                    summary: format!(
                        "post-merge regression failed for {plan_id}: {}",
                        stderr.lines().take(3).collect::<Vec<_>>().join(" | ")
                    ),
                    evidence_ref: None,
                }
            }
            Err(summary) => DeliveryRegressionOutcome {
                passed: false,
                summary,
                evidence_ref: None,
            },
        }
    }

    async fn publish(
        &self,
        request: &CompletionDeliveryRequest,
        merge_commit: &str,
    ) -> DeliveryPublicationOutcome {
        let workdir = self.workdir.clone();
        let branch = request.branch.clone();
        let commit = merge_commit.to_string();

        let refspec = format!("{commit}:refs/heads/{branch}");
        let output = tokio::process::Command::new("git")
            .args(["push", "--porcelain", "origin", &refspec])
            .current_dir(&workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .kill_on_drop(true)
            .output()
            .await;

        match output {
            Ok(ref out) if out.status.success() => DeliveryPublicationOutcome {
                published: true,
                publication_ref: Some(format!("refs/heads/{branch}")),
                summary: format!("published {branch} to origin"),
            },
            Ok(ref out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                DeliveryPublicationOutcome {
                    published: false,
                    publication_ref: None,
                    summary: format!("git push failed: {}", stderr.trim()),
                }
            }
            Err(e) => DeliveryPublicationOutcome {
                published: false,
                publication_ref: None,
                summary: format!("failed to spawn git push: {e}"),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// CliCompletionDeliveryService
// ---------------------------------------------------------------------------

/// CLI adapter that implements [`CompletionDeliveryService`] by delegating to
/// a pluggable [`DeliveryBackend`].
///
/// The service:
/// - Enforces the fixed state transition order.
/// - Uses `DeliveryReceiptStore` for in-memory receipt tracking with merge
///   serialization.
/// - Writes the `roko.delivery@1` checkpoint extension on every transition
///   (via a callback the outer controller supplies).
/// - Never writes the plan terminal state or releases workspace leases.
///
/// # Resume semantics
///
/// `reconcile` queries the receipt store for an existing delivery ID and
/// returns its current receipt. If the delivery had been stored persistently
/// (via checkpoint extensions) by the outer controller before a crash, the
/// outer controller restores the store and calls `reconcile` to resume.
#[derive(Debug)]
pub struct CliCompletionDeliveryService {
    store: DeliveryReceiptStore,
    backend: Arc<dyn DeliveryBackend>,
}

impl CliCompletionDeliveryService {
    /// Create a new delivery service.
    #[must_use]
    pub fn new(backend: Arc<dyn DeliveryBackend>) -> Self {
        Self {
            store: DeliveryReceiptStore::new(),
            backend,
        }
    }

    /// Create a delivery service with a pre-populated store (for resume).
    #[must_use]
    pub fn with_store(store: DeliveryReceiptStore, backend: Arc<dyn DeliveryBackend>) -> Self {
        Self { store, backend }
    }

    /// Return a reference to the underlying receipt store.
    #[must_use]
    pub fn store(&self) -> &DeliveryReceiptStore {
        &self.store
    }

    /// Drive the delivery state machine from its current state to terminal.
    async fn drive_delivery(
        &self,
        receipt: &mut CompletionDeliveryReceiptV1,
    ) -> Result<(), DeliveryError> {
        loop {
            if receipt.state.is_terminal() {
                return Ok(());
            }

            match receipt.state {
                CompletionDeliveryState::Prepared => {
                    // Acquire merge slot
                    self.store
                        .merge_slot()
                        .try_acquire(&receipt.request.delivery_id)
                        .map_err(|e| DeliveryError::QueueRejected {
                            delivery_id: receipt.request.delivery_id.clone(),
                            reason: e.to_string(),
                        })?;

                    receipt
                        .advance(CompletionDeliveryState::Queued)
                        .map_err(DeliveryError::Transition)?;
                    self.store.update(receipt);
                    debug!(
                        delivery_id = %receipt.request.delivery_id,
                        "delivery advanced to Queued"
                    );
                }

                CompletionDeliveryState::Queued => {
                    let outcome = self.backend.merge(&receipt.request).await;

                    if !outcome.merged {
                        receipt.error = Some(outcome.summary.clone());
                        receipt
                            .advance(CompletionDeliveryState::Conflict)
                            .map_err(DeliveryError::Transition)?;
                        self.store
                            .merge_slot()
                            .release(&receipt.request.delivery_id);
                        self.store.update(receipt);
                        warn!(
                            delivery_id = %receipt.request.delivery_id,
                            error = %outcome.summary,
                            "delivery merge conflict (terminal)"
                        );
                        return Err(DeliveryError::MergeConflict {
                            delivery_id: receipt.request.delivery_id.clone(),
                            details: outcome.summary,
                        });
                    }

                    receipt.merge_commit = outcome.merge_commit;
                    receipt
                        .advance(CompletionDeliveryState::Merged)
                        .map_err(DeliveryError::Transition)?;
                    self.store.update(receipt);
                    info!(
                        delivery_id = %receipt.request.delivery_id,
                        merge_commit = ?receipt.merge_commit,
                        "delivery merged"
                    );
                }

                CompletionDeliveryState::Merged => {
                    let merge_commit = receipt
                        .merge_commit
                        .as_deref()
                        .unwrap_or(&receipt.request.commit_oid);
                    let outcome = self
                        .backend
                        .run_regression(&receipt.request, merge_commit)
                        .await;

                    // Release merge slot after regression (success or failure)
                    self.store
                        .merge_slot()
                        .release(&receipt.request.delivery_id);

                    if !outcome.passed {
                        receipt.error = Some(outcome.summary.clone());
                        receipt.regression_evidence_ref = outcome.evidence_ref;
                        receipt
                            .advance(CompletionDeliveryState::RegressionFailed)
                            .map_err(DeliveryError::Transition)?;
                        self.store.update(receipt);
                        warn!(
                            delivery_id = %receipt.request.delivery_id,
                            error = %outcome.summary,
                            "delivery regression failed (terminal)"
                        );
                        return Err(DeliveryError::RegressionFailed {
                            delivery_id: receipt.request.delivery_id.clone(),
                            details: outcome.summary,
                        });
                    }

                    receipt.regression_evidence_ref = outcome.evidence_ref;
                    receipt
                        .advance(CompletionDeliveryState::RegressionPassed)
                        .map_err(DeliveryError::Transition)?;
                    self.store.update(receipt);
                    info!(
                        delivery_id = %receipt.request.delivery_id,
                        "delivery regression passed"
                    );
                }

                CompletionDeliveryState::RegressionPassed => {
                    if receipt.request.publish {
                        let merge_commit = receipt
                            .merge_commit
                            .as_deref()
                            .unwrap_or(&receipt.request.commit_oid);
                        let outcome = self.backend.publish(&receipt.request, merge_commit).await;

                        if !outcome.published {
                            receipt.error = Some(outcome.summary.clone());
                            receipt
                                .advance(CompletionDeliveryState::TerminalFailed)
                                .map_err(DeliveryError::Transition)?;
                            self.store.update(receipt);
                            warn!(
                                delivery_id = %receipt.request.delivery_id,
                                error = %outcome.summary,
                                "delivery publication failed (terminal)"
                            );
                            return Err(DeliveryError::PublicationFailed {
                                delivery_id: receipt.request.delivery_id.clone(),
                                details: outcome.summary,
                            });
                        }

                        receipt.publication_ref = outcome.publication_ref;
                        receipt
                            .advance(CompletionDeliveryState::Published)
                            .map_err(DeliveryError::Transition)?;
                        self.store.update(receipt);
                        info!(
                            delivery_id = %receipt.request.delivery_id,
                            publication_ref = ?receipt.publication_ref,
                            "delivery published"
                        );
                    } else {
                        // Skip Published, go straight to Delivered
                        receipt
                            .advance(CompletionDeliveryState::Delivered)
                            .map_err(DeliveryError::Transition)?;
                        self.store.update(receipt);
                        info!(
                            delivery_id = %receipt.request.delivery_id,
                            "delivery completed (no publish)"
                        );
                        return Ok(());
                    }
                }

                CompletionDeliveryState::Published => {
                    receipt
                        .advance(CompletionDeliveryState::Delivered)
                        .map_err(DeliveryError::Transition)?;
                    self.store.update(receipt);
                    info!(
                        delivery_id = %receipt.request.delivery_id,
                        "delivery completed"
                    );
                    return Ok(());
                }

                // Terminal states are already handled by the loop guard.
                CompletionDeliveryState::Delivered
                | CompletionDeliveryState::Conflict
                | CompletionDeliveryState::RegressionFailed
                | CompletionDeliveryState::TerminalFailed => {
                    return Ok(());
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl CompletionDeliveryService for CliCompletionDeliveryService {
    async fn deliver(
        &self,
        request: CompletionDeliveryRequest,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError> {
        // Check for existing delivery (idempotent or fingerprint conflict).
        match self.store.insert_or_get(&request)? {
            Some(existing) => {
                // Same fingerprint: return stored receipt.
                debug!(
                    delivery_id = %request.delivery_id,
                    state = ?existing.state,
                    "returning existing delivery receipt"
                );
                return Ok(existing);
            }
            None => {
                // New delivery: the store created a Prepared receipt.
                debug!(
                    delivery_id = %request.delivery_id,
                    "new delivery created in Prepared state"
                );
            }
        }

        let mut receipt = self.store.get(&request.delivery_id).ok_or_else(|| {
            DeliveryError::Other(format!(
                "delivery '{}' vanished from store after insert",
                request.delivery_id
            ))
        })?;

        // Drive the state machine to terminal.
        let result = self.drive_delivery(&mut receipt).await;

        // On error, return the receipt with the failure state already set.
        match result {
            Ok(()) => Ok(receipt),
            Err(DeliveryError::MergeConflict { .. })
            | Err(DeliveryError::RegressionFailed { .. })
            | Err(DeliveryError::PublicationFailed { .. }) => {
                // The receipt is already in a terminal failure state.
                Ok(self.store.get(&request.delivery_id).unwrap_or(receipt))
            }
            Err(e) => Err(e),
        }
    }

    async fn reconcile(
        &self,
        delivery_id: &str,
    ) -> Result<CompletionDeliveryReceiptV1, DeliveryError> {
        let receipt = self.store.get(delivery_id).ok_or_else(|| {
            DeliveryError::Other(format!("delivery '{delivery_id}' not found in store"))
        })?;

        if receipt.state.is_terminal() {
            debug!(
                delivery_id = %delivery_id,
                state = ?receipt.state,
                "reconcile: delivery already terminal"
            );
            return Ok(receipt);
        }

        // Resume from current state.
        let mut receipt = receipt;
        let result = self.drive_delivery(&mut receipt).await;
        match result {
            Ok(()) => Ok(receipt),
            Err(DeliveryError::MergeConflict { .. })
            | Err(DeliveryError::RegressionFailed { .. })
            | Err(DeliveryError::PublicationFailed { .. }) => {
                Ok(self.store.get(delivery_id).unwrap_or(receipt))
            }
            Err(e) => Err(e),
        }
    }
}

/// Build the checkpoint extension value for the `roko.delivery@1` namespace.
///
/// Callers (e.g. the outer drive_controller) should persist this after each
/// delivery state transition.
#[must_use]
pub fn build_delivery_checkpoint_extension(
    receipt: &CompletionDeliveryReceiptV1,
) -> serde_json::Value {
    delivery_extension_value(receipt)
}

/// The checkpoint extension key used by delivery state persistence.
///
/// Callers persist this key in the graph checkpoint to enable resume.
pub const DELIVERY_CHECKPOINT_KEY: &str = DELIVERY_EXTENSION_KEY;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    // ── Fake backend ─────────────────────────────────────────────────────

    #[derive(Debug)]
    struct FakeDeliveryBackend {
        merge_succeeds: AtomicBool,
        regression_succeeds: AtomicBool,
        publish_succeeds: AtomicBool,
    }

    impl FakeDeliveryBackend {
        fn all_pass() -> Self {
            Self {
                merge_succeeds: AtomicBool::new(true),
                regression_succeeds: AtomicBool::new(true),
                publish_succeeds: AtomicBool::new(true),
            }
        }

        fn merge_fails() -> Self {
            Self {
                merge_succeeds: AtomicBool::new(false),
                regression_succeeds: AtomicBool::new(true),
                publish_succeeds: AtomicBool::new(true),
            }
        }

        fn regression_fails() -> Self {
            Self {
                merge_succeeds: AtomicBool::new(true),
                regression_succeeds: AtomicBool::new(false),
                publish_succeeds: AtomicBool::new(true),
            }
        }

        fn publish_fails() -> Self {
            Self {
                merge_succeeds: AtomicBool::new(true),
                regression_succeeds: AtomicBool::new(true),
                publish_succeeds: AtomicBool::new(false),
            }
        }
    }

    #[async_trait::async_trait]
    impl DeliveryBackend for FakeDeliveryBackend {
        async fn merge(&self, _request: &CompletionDeliveryRequest) -> DeliveryMergeOutcome {
            if self.merge_succeeds.load(Ordering::Relaxed) {
                DeliveryMergeOutcome {
                    merged: true,
                    merge_commit: Some("abc123def456".to_string()),
                    summary: "merge succeeded".to_string(),
                }
            } else {
                DeliveryMergeOutcome {
                    merged: false,
                    merge_commit: None,
                    summary: "merge conflict in src/lib.rs".to_string(),
                }
            }
        }

        async fn run_regression(
            &self,
            _request: &CompletionDeliveryRequest,
            _merge_commit: &str,
        ) -> DeliveryRegressionOutcome {
            if self.regression_succeeds.load(Ordering::Relaxed) {
                DeliveryRegressionOutcome {
                    passed: true,
                    summary: "regression passed".to_string(),
                    evidence_ref: Some("/tmp/regression.log".to_string()),
                }
            } else {
                DeliveryRegressionOutcome {
                    passed: false,
                    summary: "cargo check failed: error[E0308]".to_string(),
                    evidence_ref: Some("/tmp/regression-fail.log".to_string()),
                }
            }
        }

        async fn publish(
            &self,
            _request: &CompletionDeliveryRequest,
            _merge_commit: &str,
        ) -> DeliveryPublicationOutcome {
            if self.publish_succeeds.load(Ordering::Relaxed) {
                DeliveryPublicationOutcome {
                    published: true,
                    publication_ref: Some("https://github.com/org/repo/pull/42".to_string()),
                    summary: "published to origin".to_string(),
                }
            } else {
                DeliveryPublicationOutcome {
                    published: false,
                    publication_ref: None,
                    summary: "git push rejected".to_string(),
                }
            }
        }
    }

    fn test_request(id: &str) -> CompletionDeliveryRequest {
        CompletionDeliveryRequest {
            delivery_id: id.to_string(),
            run_id: "run-1".to_string(),
            plan_id: "plan-a".to_string(),
            lease_id: "lease-1".to_string(),
            branch: "roko/plan-a".to_string(),
            commit_oid: "abc123".to_string(),
            target_branch: "main".to_string(),
            changed_files: vec!["src/lib.rs".to_string()],
            publish: true,
        }
    }

    fn test_request_no_publish(id: &str) -> CompletionDeliveryRequest {
        CompletionDeliveryRequest {
            publish: false,
            ..test_request(id)
        }
    }

    // ── Happy path: full delivery with publish ──────────────────────────

    #[tokio::test]
    async fn happy_path_with_publish() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-happy");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
        assert!(receipt.merge_commit.is_some());
        assert!(receipt.publication_ref.is_some());
        assert!(receipt.error.is_none());
    }

    // ── Happy path: delivery without publish ────────────────────────────

    #[tokio::test]
    async fn happy_path_without_publish() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request_no_publish("d-no-pub");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
        assert!(receipt.merge_commit.is_some());
        assert!(receipt.publication_ref.is_none());
    }

    // ── Merge conflict: terminal ────────────────────────────────────────

    #[tokio::test]
    async fn merge_conflict_is_terminal() {
        let backend = Arc::new(FakeDeliveryBackend::merge_fails());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-conflict");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::Conflict);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForReview);
        assert!(receipt.error.is_some());
        assert!(receipt.merge_commit.is_none());
    }

    // ── Regression failure: terminal ────────────────────────────────────

    #[tokio::test]
    async fn regression_failure_is_terminal() {
        let backend = Arc::new(FakeDeliveryBackend::regression_fails());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-reg-fail");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::RegressionFailed);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
        assert!(receipt.error.is_some());
        assert!(receipt.regression_evidence_ref.is_some());
    }

    // ── Publication failure: terminal ───────────────────────────────────

    #[tokio::test]
    async fn publication_failure_is_terminal() {
        let backend = Arc::new(FakeDeliveryBackend::publish_fails());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-pub-fail");

        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(receipt.state, CompletionDeliveryState::TerminalFailed);
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
        assert!(receipt.error.is_some());
    }

    // ── Idempotent duplicate submission ─────────────────────────────────

    #[tokio::test]
    async fn duplicate_deliver_returns_stored_receipt() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-dup");

        let first = service.deliver(request.clone()).await.unwrap();
        let second = service.deliver(request).await.unwrap();

        assert_eq!(first.state, CompletionDeliveryState::Delivered);
        assert_eq!(second.state, CompletionDeliveryState::Delivered);
        assert_eq!(first.merge_commit, second.merge_commit);
    }

    // ── Fingerprint mismatch ────────────────────────────────────────────

    #[tokio::test]
    async fn fingerprint_mismatch_fails_closed() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let req1 = test_request("d-fp");
        service.deliver(req1).await.unwrap();

        let mut req2 = test_request("d-fp");
        req2.commit_oid = "different-oid".to_string();
        let err = service.deliver(req2).await.unwrap_err();
        assert!(
            matches!(err, DeliveryError::FingerprintMismatch { .. }),
            "expected fingerprint mismatch, got {err:?}"
        );
    }

    // ── Reconcile returns terminal receipt ───────────────────────────────

    #[tokio::test]
    async fn reconcile_terminal_returns_receipt() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);
        let request = test_request("d-recon");

        service.deliver(request).await.unwrap();

        let receipt = service.reconcile("d-recon").await.unwrap();
        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
    }

    // ── Reconcile unknown delivery returns error ────────────────────────

    #[tokio::test]
    async fn reconcile_unknown_delivery_fails() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let err = service.reconcile("nonexistent").await.unwrap_err();
        assert!(matches!(err, DeliveryError::Other(_)));
    }

    // ── Merge slot serialization ────────────────────────────────────────

    #[tokio::test]
    async fn merge_slot_blocks_concurrent_delivery() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        // Pre-acquire the merge slot for a different delivery
        service
            .store()
            .merge_slot()
            .try_acquire("other-delivery")
            .unwrap();

        let request = test_request("d-blocked");
        let err = service.deliver(request).await.unwrap_err();
        assert!(
            matches!(err, DeliveryError::QueueRejected { .. }),
            "expected queue rejected, got {err:?}"
        );
    }

    // ── Release policy correctness ──────────────────────────────────────

    #[tokio::test]
    async fn release_policy_delete_on_success() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let receipt = service
            .deliver(test_request_no_publish("d-rp-del"))
            .await
            .unwrap();
        assert_eq!(receipt.release_policy, ReleasePolicy::Delete);
    }

    #[tokio::test]
    async fn release_policy_retain_on_conflict() {
        let backend = Arc::new(FakeDeliveryBackend::merge_fails());
        let service = CliCompletionDeliveryService::new(backend);

        let receipt = service.deliver(test_request("d-rp-review")).await.unwrap();
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForReview);
    }

    #[tokio::test]
    async fn release_policy_retain_on_regression_failure() {
        let backend = Arc::new(FakeDeliveryBackend::regression_fails());
        let service = CliCompletionDeliveryService::new(backend);

        let receipt = service.deliver(test_request("d-rp-fail")).await.unwrap();
        assert_eq!(receipt.release_policy, ReleasePolicy::RetainForFailure);
    }

    // ── Checkpoint extension structure ───────────────────────────────────

    #[tokio::test]
    async fn checkpoint_extension_has_correct_structure() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let receipt = service.deliver(test_request("d-ext")).await.unwrap();

        let ext = build_delivery_checkpoint_extension(&receipt);
        assert_eq!(ext["delivery_id"], "d-ext");
        assert_eq!(ext["state"], "delivered");
        assert_eq!(ext["plan_id"], "plan-a");
        assert_eq!(ext["release_policy"], "delete");
        assert!(!ext["merge_commit"].is_null());
    }

    // ── Merge slot released after conflict ──────────────────────────────

    #[tokio::test]
    async fn merge_slot_released_after_conflict() {
        let backend = Arc::new(FakeDeliveryBackend::merge_fails());
        let service = CliCompletionDeliveryService::new(backend);

        let _ = service.deliver(test_request("d-slot-conflict")).await;

        // Merge slot should be released so next delivery can proceed
        assert!(service.store().merge_slot().current_holder().is_none());
    }

    // ── Merge slot released after regression failure ────────────────────

    #[tokio::test]
    async fn merge_slot_released_after_regression_failure() {
        let backend = Arc::new(FakeDeliveryBackend::regression_fails());
        let service = CliCompletionDeliveryService::new(backend);

        let _ = service.deliver(test_request("d-slot-reg")).await;

        assert!(service.store().merge_slot().current_holder().is_none());
    }

    // ── Merge slot released after success ───────────────────────────────

    #[tokio::test]
    async fn merge_slot_released_after_success() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        let _ = service.deliver(test_request("d-slot-ok")).await;

        assert!(service.store().merge_slot().current_holder().is_none());
    }

    // ── Resume from non-terminal state ──────────────────────────────────

    #[tokio::test]
    async fn resume_from_prepared_state() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let store = DeliveryReceiptStore::new();

        // Pre-populate a Prepared receipt (simulating crash after prepare).
        let request = test_request("d-resume");
        store.insert_or_get(&request).unwrap();

        let service = CliCompletionDeliveryService::with_store(store, backend);

        let receipt = service.reconcile("d-resume").await.unwrap();
        assert_eq!(receipt.state, CompletionDeliveryState::Delivered);
    }

    // ── Store access via service ────────────────────────────────────────

    #[tokio::test]
    async fn store_tracks_deliveries() {
        let backend = Arc::new(FakeDeliveryBackend::all_pass());
        let service = CliCompletionDeliveryService::new(backend);

        assert!(service.store().is_empty());

        service.deliver(test_request("d-track")).await.unwrap();

        assert_eq!(service.store().len(), 1);
        assert!(service.store().get("d-track").is_some());
    }

    // ── GitDeliveryBackend against a real repository ────────────────────

    /// Run git in `repo` and return its trimmed stdout.
    fn git(repo: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn commit_all(repo: &Path, message: &str) {
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "--quiet", "-m", message]);
    }

    /// A request delivering `roko/plan-a` of `repo` as verified at its
    /// current head.
    fn git_request(id: &str, repo: &Path) -> CompletionDeliveryRequest {
        CompletionDeliveryRequest {
            commit_oid: git(repo, &["rev-parse", "roko/plan-a"]),
            ..test_request(id)
        }
    }

    /// A repository where `main` and `roko/plan-a` each added their own file
    /// on top of `shared.txt`, so they merge cleanly. `main` is checked out.
    fn diverged_repo() -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        let path = repo.path();
        git(path, &["init", "--quiet", "--initial-branch=main"]);
        git(path, &["config", "user.name", "roko"]);
        git(path, &["config", "user.email", "roko@nunchi.dev"]);
        git(path, &["config", "commit.gpgsign", "false"]);
        std::fs::write(path.join("shared.txt"), "base\n").unwrap();
        commit_all(path, "base");
        git(path, &["checkout", "--quiet", "-b", "roko/plan-a"]);
        std::fs::write(path.join("plan.txt"), "plan-a\n").unwrap();
        commit_all(path, "plan-a");
        git(path, &["checkout", "--quiet", "main"]);
        std::fs::write(path.join("main.txt"), "main\n").unwrap();
        commit_all(path, "main moves on");
        repo
    }

    #[tokio::test]
    async fn merge_leaves_the_user_checkout_alone() {
        let repo = diverged_repo();
        let path = repo.path();
        // The user works on their own branch, with an uncommitted edit.
        git(path, &["checkout", "--quiet", "-b", "work"]);
        std::fs::write(path.join("shared.txt"), "base\nuser edit\n").unwrap();
        let head = git(path, &["rev-parse", "HEAD"]);
        let status = git(path, &["status", "--porcelain"]);
        let main_before = git(path, &["rev-parse", "main"]);
        let plan_head = git(path, &["rev-parse", "roko/plan-a"]);

        let backend = GitDeliveryBackend::new(path.to_path_buf());
        let outcome = backend.merge(&git_request("d-git-merge", path)).await;

        assert!(outcome.merged, "{}", outcome.summary);
        assert_eq!(git(path, &["symbolic-ref", "HEAD"]), "refs/heads/work");
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        assert_eq!(git(path, &["status", "--porcelain"]), status);
        assert_eq!(
            std::fs::read_to_string(path.join("shared.txt")).unwrap(),
            "base\nuser edit\n"
        );
        assert!(!path.join("plan.txt").exists());
        // `main` moved to a merge of its old head and the plan branch.
        let main_after = git(path, &["rev-parse", "main"]);
        assert_eq!(outcome.merge_commit.as_deref(), Some(main_after.as_str()));
        assert_eq!(git(path, &["rev-parse", "main^1"]), main_before);
        assert_eq!(git(path, &["rev-parse", "main^2"]), plan_head);
    }

    #[tokio::test]
    async fn merge_into_checked_out_target_does_not_move_it() {
        let repo = diverged_repo();
        let path = repo.path();
        // The user has the target, `main`, checked out with an uncommitted edit.
        std::fs::write(path.join("shared.txt"), "base\nuser edit\n").unwrap();
        let status = git(path, &["status", "--porcelain"]);
        let main_before = git(path, &["rev-parse", "main"]);
        let plan_head = git(path, &["rev-parse", "roko/plan-a"]);

        let backend = GitDeliveryBackend::new(path.to_path_buf());
        let outcome = backend.merge(&git_request("d-git-checked-out", path)).await;

        let summary = &outcome.summary;
        assert!(!outcome.merged, "{summary}");
        assert!(outcome.merge_commit.is_none());
        assert!(summary.contains("checked out"), "{summary}");
        assert_eq!(git(path, &["rev-parse", "main"]), main_before);
        assert_eq!(git(path, &["symbolic-ref", "HEAD"]), "refs/heads/main");
        assert_eq!(git(path, &["status", "--porcelain"]), status);
        assert_eq!(
            std::fs::read_to_string(path.join("shared.txt")).unwrap(),
            "base\nuser edit\n"
        );
        // The prepared merge is parked where the summary says.
        let parked = "refs/roko/delivered/plan-a";
        assert!(summary.contains(parked), "{summary}");
        let first_parent = git(path, &["rev-parse", &format!("{parked}^1")]);
        let second_parent = git(path, &["rev-parse", &format!("{parked}^2")]);
        assert_eq!(first_parent, main_before);
        assert_eq!(second_parent, plan_head);
    }

    #[tokio::test]
    async fn merge_conflict_is_reported_without_touching_refs() {
        let repo = diverged_repo();
        let path = repo.path();
        // Both branches rewrite shared.txt; the user is on a third branch.
        std::fs::write(path.join("shared.txt"), "main's version\n").unwrap();
        commit_all(path, "main rewrites shared.txt");
        git(path, &["checkout", "--quiet", "roko/plan-a"]);
        std::fs::write(path.join("shared.txt"), "plan's version\n").unwrap();
        commit_all(path, "plan rewrites shared.txt");
        git(path, &["checkout", "--quiet", "-b", "work"]);
        let refs = git(path, &["for-each-ref"]);
        let head = git(path, &["rev-parse", "HEAD"]);

        let backend = GitDeliveryBackend::new(path.to_path_buf());
        let outcome = backend.merge(&git_request("d-git-conflict", path)).await;

        let summary = &outcome.summary;
        assert!(!outcome.merged, "{summary}");
        assert!(outcome.merge_commit.is_none());
        assert!(summary.contains("conflict"), "{summary}");
        assert!(summary.contains("shared.txt"), "{summary}");
        assert_eq!(git(path, &["for-each-ref"]), refs);
        assert_eq!(git(path, &["rev-parse", "HEAD"]), head);
        assert_eq!(git(path, &["status", "--porcelain"]), "");
    }

    #[tokio::test]
    async fn merge_fast_forwards_a_target_that_is_behind() {
        let repo = diverged_repo();
        let path = repo.path();
        // `main` sits at the plan branch's base; the user is on `work`.
        git(path, &["checkout", "--quiet", "-b", "work"]);
        git(path, &["branch", "--force", "main", "roko/plan-a~1"]);
        let plan_head = git(path, &["rev-parse", "roko/plan-a"]);
        let backend = GitDeliveryBackend::new(path.to_path_buf());

        let outcome = backend.merge(&git_request("d-git-ff", path)).await;

        assert!(outcome.merged, "{}", outcome.summary);
        assert_eq!(outcome.merge_commit.as_deref(), Some(plan_head.as_str()));
        assert_eq!(git(path, &["rev-parse", "main"]), plan_head);

        // Delivering the branch again finds nothing left to merge.
        let again = backend.merge(&git_request("d-git-ff-again", path)).await;

        assert!(again.merged, "{}", again.summary);
        assert_eq!(again.merge_commit.as_deref(), Some(plan_head.as_str()));
        assert_eq!(git(path, &["rev-parse", "main"]), plan_head);
    }

    #[tokio::test]
    async fn regression_runs_in_a_temporary_checkout_of_the_merge() {
        let repo = diverged_repo();
        let path = repo.path();
        git(path, &["checkout", "--quiet", "-b", "work"]);
        let status = git(path, &["status", "--porcelain"]);
        let worktrees = git(path, &["worktree", "list", "--porcelain"]);
        let record = tempfile::tempdir().unwrap();
        let ran_in = record.path().join("ran-in");
        // The check records where it ran, and passes only on the merge commit.
        let script = format!(
            "pwd -P > '{}' && test -f plan.txt && test -f main.txt",
            ran_in.display()
        );
        let backend = GitDeliveryBackend::new(path.to_path_buf()).with_regression_command(vec![
            "sh".into(),
            "-c".into(),
            script,
        ]);
        let service = CliCompletionDeliveryService::new(Arc::new(backend));

        let request = CompletionDeliveryRequest {
            publish: false,
            ..git_request("d-git-regression", path)
        };
        let receipt = service.deliver(request).await.unwrap();

        assert_eq!(
            receipt.state,
            CompletionDeliveryState::Delivered,
            "{:?}",
            receipt.error
        );
        let main = git(path, &["rev-parse", "main"]);
        assert_eq!(receipt.merge_commit.as_deref(), Some(main.as_str()));
        let ran_in = PathBuf::from(std::fs::read_to_string(&ran_in).unwrap().trim());
        assert!(
            !ran_in.starts_with(path.canonicalize().unwrap()),
            "the regression ran in the user's checkout: {}",
            ran_in.display()
        );
        assert!(!ran_in.exists(), "the regression checkout was not removed");
        assert_eq!(git(path, &["worktree", "list", "--porcelain"]), worktrees);
        assert_eq!(git(path, &["status", "--porcelain"]), status);
    }

    /// bug-453481: delivery merges the commit the plan's gates verified.
    /// A commit added to the branch afterwards stays off the target, and the
    /// summary says it waits for a delivery of its own.
    #[tokio::test]
    async fn merge_takes_the_verified_commit_not_the_branch_head() {
        let repo = diverged_repo();
        let path = repo.path();
        let request = git_request("d-git-verified", path);
        let verified = request.commit_oid.clone();
        // After verification, a late write lands on the plan branch.
        git(path, &["checkout", "--quiet", "roko/plan-a"]);
        std::fs::write(path.join("late.txt"), "not verified\n").unwrap();
        commit_all(path, "late, unverified commit");
        let late = git(path, &["rev-parse", "roko/plan-a"]);
        git(path, &["checkout", "--quiet", "-b", "work"]);
        let main_before = git(path, &["rev-parse", "main"]);

        let backend = GitDeliveryBackend::new(path.to_path_buf());
        let outcome = backend.merge(&request).await;

        let summary = &outcome.summary;
        assert!(outcome.merged, "{summary}");
        let main_after = git(path, &["rev-parse", "main"]);
        assert_eq!(outcome.merge_commit.as_deref(), Some(main_after.as_str()));
        assert_eq!(git(path, &["rev-parse", "main^1"]), main_before);
        assert_eq!(git(path, &["rev-parse", "main^2"]), verified);
        let late_reached = std::process::Command::new("git")
            .args(["merge-base", "--is-ancestor", &late, "main"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(
            !late_reached.success(),
            "the unverified commit reached main"
        );
        assert!(summary.contains(&late), "{summary}");
        assert_eq!(git(path, &["rev-parse", "roko/plan-a"]), late);
    }

    /// bug-453481: without its verified commit on the branch, delivery merges
    /// nothing and moves no ref: the commit id is missing, names no commit,
    /// or is not on the branch any more.
    #[tokio::test]
    async fn merge_fails_closed_without_the_verified_commit_on_the_branch() {
        let repo = diverged_repo();
        let path = repo.path();
        git(path, &["checkout", "--quiet", "-b", "work"]);
        let refs = git(path, &["for-each-ref"]);
        let off_branch = git(path, &["rev-parse", "main"]);
        let backend = GitDeliveryBackend::new(path.to_path_buf());

        for (commit_oid, reason) in [
            ("", "not a commit id"),
            ("roko/plan-a", "not a commit id"),
            (
                "0123456789abcdef0123456789abcdef01234567",
                "does not resolve",
            ),
            (off_branch.as_str(), "not on branch"),
        ] {
            let request = CompletionDeliveryRequest {
                commit_oid: commit_oid.to_string(),
                ..test_request("d-git-unverified")
            };
            let outcome = backend.merge(&request).await;
            let summary = &outcome.summary;
            assert!(!outcome.merged, "{commit_oid}: {summary}");
            assert!(outcome.merge_commit.is_none(), "{commit_oid}");
            assert!(summary.contains(reason), "{commit_oid}: {summary}");
            assert_eq!(git(path, &["for-each-ref"]), refs, "{commit_oid}");
        }
    }
}
