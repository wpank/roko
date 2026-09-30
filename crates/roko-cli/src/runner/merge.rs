//! Merge dispatch wrapper around [`MergeQueue`].
//!
//! The runner used to translate `ExecutorAction::MergeBranch` into an
//! immediate `ExecutorEvent::MergeSucceeded`, which silently produced
//! broken merges whenever multiple plans ran in parallel and touched
//! overlapping files. This module routes merge actions through
//! `MergeQueue` instead and runs a real post-merge regression gate so a
//! broken integration branch can be detected and surfaced as a merge
//! failure instead of a silent success.
//!
//! `PlanMerger` reuses the existing `MergeQueue` for queue / lock semantics
//! and runs an injected merge backend and post-merge regression gate. It has
//! no built-in ones: a merge without both fails closed. Roko merges plan
//! results with `graph_execution::delivery::GitDeliveryBackend`, which never
//! stages, commits or merges in a checkout (git plumbing only).
//!
//! The git helpers at the end of this module (`git_command`, `git_output`,
//! `git_merge_tree`) are shared with delivery and worktree acceptance.

use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::orchestrator::{MergeQueue, MergeRequest, MergeReservation, ReservedMerge};
use futures::FutureExt;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use super::gate_dispatch::RUNG_MERGE;
use super::types::{
    GateCompletion, GateCompletionKind, GateEffectRef, GateVerdictSummary, RunnerFailureKind,
    TaskAttemptRef,
};

// ─── PlanMerger ─────────────────────────────────────────────────────────

/// Outcome of a merge dispatch attempt.
#[derive(Debug, PartialEq, Eq)]
pub enum MergeDispatch {
    /// This plan already owns an active reservation; duplicate submit rejected.
    AlreadyActive { plan_id: String },
    /// The merge was claimed and submitted to the regression gate. The
    /// caller should expect a `GateCompletion` with the matching plan id
    /// to arrive on the gate channel.
    Reserved { launch: MergeLaunch },
    /// The plan was enqueued but is currently blocked by an in-progress
    /// merge holding one or more of its files.
    Blocked {
        plan_id: String,
        launch: Option<MergeLaunch>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct MergeLaunch {
    request: MergeRequest,
    generation: u64,
    reservation: MergeReservation,
}

impl MergeLaunch {
    pub fn plan_id(&self) -> &str {
        &self.request.plan_id
    }

    pub fn branch_name(&self) -> &str {
        &self.request.branch_name
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn effect(&self) -> GateEffectRef {
        GateEffectRef {
            attempt: TaskAttemptRef::new(
                self.plan_id(),
                format!("merge:{}", self.branch_name()),
                u32::try_from(self.generation).unwrap_or(u32::MAX),
            ),
            kind: GateCompletionKind::Merge,
            rung: RUNG_MERGE,
            generation: self.generation,
        }
    }
}

static NEXT_MERGE_LAUNCH: AtomicU64 = AtomicU64::new(1);

fn merge_launch(reserved: ReservedMerge) -> MergeLaunch {
    MergeLaunch {
        request: reserved.request,
        generation: NEXT_MERGE_LAUNCH.fetch_add(1, Ordering::Relaxed),
        reservation: reserved.reservation,
    }
}

pub struct MergeProducer {
    pub effect: GateEffectRef,
    pub handle: JoinHandle<()>,
    pub start: oneshot::Sender<()>,
    #[allow(dead_code)] // accessed in test code only
    pub(crate) resolution: MergeResolution,
}

#[derive(Clone)]
pub struct MergeResolution {
    plan_id: String,
    reservation: MergeReservation,
}

impl MergeResolution {
    #[allow(dead_code)] // production caller not yet connected
    pub(crate) fn fail(self, queue: &MergeQueue, reason: &str) -> bool {
        queue.mark_failed_exact(&self.plan_id, self.reservation, reason)
    }
}

/// Wrapper around [`MergeQueue`] that submits merge requests, runs a
/// post-merge regression gate, and emits a `GateCompletion` describing
/// the outcome.
#[derive(Debug, Clone)]
pub struct PlanMerger {
    queue: MergeQueue,
    config: PlanMergerConfig,
}

/// Configuration for [`PlanMerger`].
#[derive(Debug, Clone)]
pub struct PlanMergerConfig {
    /// Working directory used when running the regression gate.
    pub workdir: PathBuf,
    /// Wall-clock timeout for the regression gate.
    pub regression_timeout: Duration,
    /// Merge backend. There is no built-in one: without it, a merge fails
    /// closed.
    pub merge_backend: Option<Arc<dyn MergeBackend>>,
    /// Post-merge regression gate. There is no built-in one: without it, a
    /// merge fails closed.
    pub regression_gate: Option<Arc<dyn RegressionGate>>,
}

impl PlanMergerConfig {
    /// Construct a config rooted at `workdir`, with no merge backend and no
    /// regression gate: install both before preparing a merge.
    #[must_use]
    pub fn new(workdir: PathBuf, regression_timeout: Duration) -> Self {
        Self {
            workdir,
            regression_timeout,
            merge_backend: None,
            regression_gate: None,
        }
    }

    /// Install a custom merge backend.
    #[must_use]
    pub fn with_merge_backend(mut self, backend: Arc<dyn MergeBackend>) -> Self {
        self.merge_backend = Some(backend);
        self
    }

    /// Install a custom regression gate (used by tests and integrations
    /// that want to stub out cargo).
    #[must_use]
    pub fn with_regression_gate(mut self, gate: Arc<dyn RegressionGate>) -> Self {
        self.regression_gate = Some(gate);
        self
    }
}

// ─── Merge backend ─────────────────────────────────────────────────────

/// Outcome of applying a plan merge/finalization request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeBackendOutcome {
    pub passed: bool,
    pub summary: String,
    pub failure_kind: Option<RunnerFailureKind>,
    pub duration_ms: u64,
    /// G04: Conflicted file paths populated on merge conflict.
    /// Empty when the merge succeeded or failed for non-conflict reasons.
    pub conflicted_paths: Vec<String>,
}

impl MergeBackendOutcome {
    #[must_use]
    pub fn pass(summary: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            passed: true,
            summary: summary.into(),
            failure_kind: None,
            duration_ms,
            conflicted_paths: Vec::new(),
        }
    }

    #[must_use]
    pub fn fail(
        summary: impl Into<String>,
        failure_kind: RunnerFailureKind,
        duration_ms: u64,
    ) -> Self {
        Self {
            passed: false,
            summary: summary.into(),
            failure_kind: Some(failure_kind),
            duration_ms,
            conflicted_paths: Vec::new(),
        }
    }

    /// Construct a failure outcome that carries the conflicted file paths,
    /// enabling downstream conflict-aware replan (G04).
    #[must_use]
    pub fn fail_with_conflicts(
        summary: impl Into<String>,
        failure_kind: RunnerFailureKind,
        duration_ms: u64,
        conflicted_paths: Vec<String>,
    ) -> Self {
        Self {
            passed: false,
            summary: summary.into(),
            failure_kind: Some(failure_kind),
            duration_ms,
            conflicted_paths,
        }
    }
}

/// Pluggable backend for applying a reserved merge request.
#[async_trait::async_trait]
pub trait MergeBackend: Send + Sync + std::fmt::Debug {
    async fn merge(&self, request: &MergeRequest, config: &PlanMergerConfig)
    -> MergeBackendOutcome;
}

// ─── Regression gate ────────────────────────────────────────────────────

/// Outcome of a post-merge regression gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegressionOutcome {
    pub passed: bool,
    pub summary: String,
    pub failure_kind: Option<RunnerFailureKind>,
    pub duration_ms: u64,
}

impl RegressionOutcome {
    #[must_use]
    pub fn pass(summary: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            passed: true,
            summary: summary.into(),
            failure_kind: None,
            duration_ms,
        }
    }

    #[must_use]
    pub fn fail(
        summary: impl Into<String>,
        failure_kind: RunnerFailureKind,
        duration_ms: u64,
    ) -> Self {
        Self {
            passed: false,
            summary: summary.into(),
            failure_kind: Some(failure_kind),
            duration_ms,
        }
    }
}

/// Pluggable regression gate. Implementors run whatever workspace check
/// is appropriate (`cargo check`, custom verifier, etc.).
#[async_trait::async_trait]
pub trait RegressionGate: Send + Sync + std::fmt::Debug {
    async fn run(&self, request: &MergeRequest, config: &PlanMergerConfig) -> RegressionOutcome;
}

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

impl PlanMerger {
    /// Construct a new merger. The merger borrows the existing
    /// `MergeQueue` from the runtime so resume snapshots remain coherent.
    #[must_use]
    pub fn new(queue: MergeQueue, config: PlanMergerConfig) -> Self {
        Self { queue, config }
    }

    /// Submit a `MergeRequest` to the queue and (if the queue grants the
    /// slot immediately) spawn the regression gate. Returns:
    ///
    /// - `MergeDispatch::Reserved` when this plan got the merge slot. The
    ///   caller should expect a `GateCompletion` with `kind == Gate` for
    ///   the merge plan id to land on `gate_tx` shortly.
    /// - `MergeDispatch::Blocked` when the plan is queued but waiting for
    ///   another plan's file locks to release.
    pub fn submit(&self, request: MergeRequest) -> MergeDispatch {
        let plan_id = request.plan_id.clone();
        if !self.queue.enqueue(request) {
            return MergeDispatch::AlreadyActive { plan_id };
        }

        let Some(reserved) = self.queue.reserve_next_mergeable_exact() else {
            return MergeDispatch::Blocked {
                plan_id,
                launch: None,
            };
        };
        if reserved.request.plan_id != plan_id {
            return MergeDispatch::Blocked {
                plan_id,
                launch: Some(merge_launch(reserved)),
            };
        }

        MergeDispatch::Reserved {
            launch: merge_launch(reserved),
        }
    }

    /// Try to drain the queue further. Useful after a merge completes to
    /// kick off the next non-conflicting plan.
    pub fn drain_next(&self) -> Option<MergeLaunch> {
        self.queue.reserve_next_mergeable_exact().map(merge_launch)
    }

    pub fn drain_bound(&self) -> usize {
        self.queue
            .snapshot()
            .entries
            .iter()
            .map(|entry| {
                usize::try_from(
                    crate::orchestrator::DEFAULT_MAX_MERGE_RETRIES
                        .saturating_sub(entry.request.retry_count)
                        .max(1),
                )
                .unwrap_or(usize::MAX)
            })
            .sum()
    }

    pub fn fail_launch(&self, launch: MergeLaunch, reason: &str) -> bool {
        self.queue
            .mark_failed_exact(&launch.request.plan_id, launch.reservation, reason)
    }

    pub fn prepare(
        &self,
        launch: MergeLaunch,
        gate_tx: mpsc::Sender<GateCompletion>,
    ) -> MergeProducer {
        let effect = launch.effect();
        let completion_effect = effect.clone();
        let resolution = MergeResolution {
            plan_id: launch.request.plan_id.clone(),
            reservation: launch.reservation,
        };
        let request = launch.request;
        let config = self.config.clone();
        let backends = self
            .config
            .merge_backend
            .clone()
            .zip(self.config.regression_gate.clone());

        let (start, start_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            if start_rx.await.is_err() {
                return;
            }
            let worker = AssertUnwindSafe(async {
                // No built-in backend: a merger missing one fails closed.
                let Some((merge_backend, gate)) = backends else {
                    return RegressionOutcome::fail(
                        "no merge backend or regression gate is configured, so nothing was \
                         merged",
                        RunnerFailureKind::Structural,
                        0,
                    );
                };
                let merge_outcome = merge_backend.merge(&request, &config).await;
                if merge_outcome.passed {
                    let gate_outcome = gate.run(&request, &config).await;
                    RegressionOutcome {
                        passed: gate_outcome.passed,
                        summary: format!("{}; {}", merge_outcome.summary, gate_outcome.summary),
                        failure_kind: gate_outcome.failure_kind,
                        duration_ms: merge_outcome
                            .duration_ms
                            .saturating_add(gate_outcome.duration_ms),
                    }
                } else {
                    RegressionOutcome {
                        passed: false,
                        summary: merge_outcome.summary,
                        failure_kind: merge_outcome.failure_kind,
                        duration_ms: merge_outcome.duration_ms,
                    }
                }
            })
            .catch_unwind()
            .await;
            let (outcome, gate_name) = match worker {
                Ok(outcome) => (outcome, "post-merge-regression"),
                Err(_) => (
                    RegressionOutcome {
                        passed: false,
                        summary: "merge producer panicked".to_string(),
                        failure_kind: Some(RunnerFailureKind::Structural),
                        duration_ms: 0,
                    },
                    "merge-producer-exception",
                ),
            };
            let passed = outcome.passed;

            let summary = GateVerdictSummary {
                gate_name: gate_name.to_string(),
                passed,
                skipped: false,
                summary: outcome.summary.clone(),
                error_digest: None,
                failure_kind: outcome.failure_kind,
                rung_index: None, // merge sentinel: not a canonical rung
            };

            let completion = GateCompletion {
                effect: Some(completion_effect.clone()),
                kind: GateCompletionKind::Merge,
                attempt: Some(completion_effect.attempt.clone()),
                plan_id: request.plan_id.clone(),
                task_id: format!("merge:{}", request.branch_name),
                rung: RUNG_MERGE,
                passed,
                failure_kind: outcome.failure_kind,
                verdicts: vec![summary],
                output: outcome.summary,
                duration_ms: outcome.duration_ms,
                selected_rungs: Vec::new(), // sentinel: no canonical rungs for merge
            };

            // Channel may be closed if the runner shut down — log only.
            if gate_tx.send(completion).await.is_err() {
                tracing::warn!("merge regression completion dropped — gate channel closed");
            }
        });
        MergeProducer {
            effect,
            handle,
            start,
            resolution,
        }
    }

    pub fn fail_resolution(&self, resolution: MergeResolution, reason: &str) -> bool {
        self.queue
            .mark_failed_exact(&resolution.plan_id, resolution.reservation, reason)
    }

    pub fn resolve_completion(
        &self,
        resolution: MergeResolution,
        passed: bool,
        reason: &str,
    ) -> bool {
        if passed {
            self.queue
                .mark_complete_exact(&resolution.plan_id, resolution.reservation)
        } else {
            self.queue
                .mark_failed_exact(&resolution.plan_id, resolution.reservation, reason)
        }
    }

    pub fn terminal_fail(&self, plan_id: &str, reason: &str) {
        self.queue.mark_terminal_failed(plan_id, reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct StubGate {
        calls: Mutex<Vec<MergeRequest>>,
        outcome: Mutex<Option<RegressionOutcome>>,
    }

    #[derive(Debug)]
    struct StubMerge {
        outcome: MergeBackendOutcome,
    }

    #[derive(Debug)]
    struct PanickingMerge;

    #[async_trait::async_trait]
    impl MergeBackend for StubMerge {
        async fn merge(
            &self,
            _request: &MergeRequest,
            _config: &PlanMergerConfig,
        ) -> MergeBackendOutcome {
            self.outcome.clone()
        }
    }

    #[async_trait::async_trait]
    impl MergeBackend for PanickingMerge {
        async fn merge(
            &self,
            _request: &MergeRequest,
            _config: &PlanMergerConfig,
        ) -> MergeBackendOutcome {
            panic!("merge backend panic")
        }
    }

    impl StubGate {
        fn new(outcome: RegressionOutcome) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                outcome: Mutex::new(Some(outcome)),
            }
        }
    }

    #[async_trait::async_trait]
    impl RegressionGate for StubGate {
        async fn run(
            &self,
            request: &MergeRequest,
            _config: &PlanMergerConfig,
        ) -> RegressionOutcome {
            self.calls.lock().unwrap().push(request.clone());
            self.outcome
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| RegressionOutcome::pass("ok", 1))
        }
    }

    fn merger_with_gate(gate: Arc<dyn RegressionGate>) -> PlanMerger {
        let merge: Arc<dyn MergeBackend> = Arc::new(StubMerge {
            outcome: MergeBackendOutcome::pass("merge ok", 1),
        });
        let cfg = PlanMergerConfig::new(PathBuf::from("/tmp"), Duration::from_secs(5))
            .with_merge_backend(merge)
            .with_regression_gate(gate);
        PlanMerger::new(MergeQueue::new(), cfg)
    }

    /// bug-207f35: there is no built-in merge backend or regression gate
    /// (the old ones staged, committed and merged in the checkout they were
    /// given), so a merger missing one fails the merge closed.
    #[tokio::test]
    async fn merge_without_backends_fails_closed() {
        let config = PlanMergerConfig::new(PathBuf::from("/tmp"), Duration::from_secs(5));
        let merger = PlanMerger::new(MergeQueue::new(), config);
        let (tx, mut rx) = mpsc::channel(4);
        let request = MergeRequest::new("plan-a", "roko/plan-a", vec!["src/lib.rs".to_string()], 0);
        let MergeDispatch::Reserved { launch } = merger.submit(request) else {
            panic!("expected reservation");
        };

        let producer = merger.prepare(launch, tx);
        producer.start.send(()).unwrap();
        let completion = rx.recv().await.expect("completion");

        assert!(!completion.passed);
        assert_eq!(completion.failure_kind, Some(RunnerFailureKind::Structural));
        assert!(
            completion.output.contains("nothing was merged"),
            "{}",
            completion.output
        );
    }

    #[tokio::test]
    async fn submit_reserves_first_plan_and_runs_regression() {
        let gate: Arc<StubGate> = Arc::new(StubGate::new(RegressionOutcome::pass("ok", 10)));
        let merger = merger_with_gate(gate.clone());
        let (tx, mut rx) = mpsc::channel(4);
        let request = MergeRequest::new("plan-a", "roko/plan-a", vec!["src/lib.rs".to_string()], 0);

        let MergeDispatch::Reserved { launch } = merger.submit(request) else {
            panic!("expected reservation");
        };
        assert_eq!(launch.plan_id(), "plan-a");
        let producer = merger.prepare(launch, tx);
        tokio::task::yield_now().await;
        assert!(gate.calls.lock().unwrap().is_empty());
        producer.start.send(()).unwrap();

        let completion = rx.recv().await.expect("regression gate completion");
        producer.handle.await.unwrap();
        assert_eq!(merger.queue.metrics().merging, 1);
        assert!(merger.resolve_completion(
            producer.resolution,
            completion.passed,
            &completion.output,
        ));
        assert_eq!(merger.queue.metrics().merging, 0);
        assert_eq!(completion.plan_id, "plan-a");
        assert!(completion.passed);
        assert_eq!(gate.calls.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn post_merge_failure_marks_failed_and_emits_completion() {
        let gate: Arc<StubGate> = Arc::new(StubGate::new(RegressionOutcome::fail(
            "regression: trait bound failed",
            RunnerFailureKind::Permanent,
            42,
        )));
        let merger = merger_with_gate(gate.clone());
        let (tx, mut rx) = mpsc::channel(4);
        let request = MergeRequest::new("plan-a", "roko/plan-a", vec!["src/lib.rs".into()], 0);

        let MergeDispatch::Reserved { launch } = merger.submit(request) else {
            panic!("expected reservation");
        };
        let producer = merger.prepare(launch, tx);
        producer.start.send(()).unwrap();

        let completion = rx.recv().await.expect("expected gate completion");
        producer.handle.await.unwrap();
        assert!(merger.resolve_completion(
            producer.resolution,
            completion.passed,
            &completion.output,
        ));
        assert_eq!(completion.plan_id, "plan-a");
        assert!(!completion.passed);
        assert_eq!(completion.failure_kind, Some(RunnerFailureKind::Permanent));
        assert_eq!(completion.verdicts.len(), 1);
        assert!(completion.verdicts[0].summary.contains("regression"));
    }

    #[tokio::test]
    async fn second_conflicting_plan_blocks_until_first_clears() {
        let gate = Arc::new(StubGate::new(RegressionOutcome::pass("ok", 1)));
        let merger = merger_with_gate(gate.clone());
        let (tx1, mut rx1) = mpsc::channel(4);

        let req_a = MergeRequest::new("plan-a", "roko/plan-a", vec!["src/lib.rs".into()], 0);
        let req_b = MergeRequest::new("plan-b", "roko/plan-b", vec!["src/lib.rs".into()], 0);

        let MergeDispatch::Reserved { launch } = merger.submit(req_a) else {
            panic!("expected reservation");
        };
        let producer_a = merger.prepare(launch, tx1.clone());
        producer_a.start.send(()).unwrap();

        // Wait for plan-a's regression gate to finish before B is submitted —
        // otherwise we cannot guarantee plan-a holds the lock.
        let completion_a = rx1.recv().await.expect("plan-a completion");
        producer_a.handle.await.unwrap();
        assert!(merger.resolve_completion(
            producer_a.resolution,
            completion_a.passed,
            &completion_a.output,
        ));
        assert_eq!(completion_a.plan_id, "plan-a");

        // Now submit B. It should not be blocked because plan-a is already
        // released.
        let (tx2, mut rx2) = mpsc::channel(4);
        let MergeDispatch::Reserved { launch } = merger.submit(req_b) else {
            panic!("expected reservation");
        };
        let producer_b = merger.prepare(launch, tx2);
        producer_b.start.send(()).unwrap();
        let completion_b = rx2.recv().await.expect("plan-b completion");
        producer_b.handle.await.unwrap();
        assert!(merger.resolve_completion(
            producer_b.resolution,
            completion_b.passed,
            &completion_b.output,
        ));
        assert_eq!(completion_b.plan_id, "plan-b");
    }

    #[tokio::test]
    async fn submit_returns_blocked_when_lock_held() {
        // Manually craft a queue with plan-a already merging so plan-b is
        // blocked when it submits.
        let gate = Arc::new(StubGate::new(RegressionOutcome::pass("ok", 1)));
        let merge: Arc<dyn MergeBackend> = Arc::new(StubMerge {
            outcome: MergeBackendOutcome::pass("merge ok", 1),
        });
        let cfg = PlanMergerConfig::new(PathBuf::from("/tmp"), Duration::from_secs(5))
            .with_merge_backend(merge)
            .with_regression_gate(gate.clone());
        let queue = MergeQueue::new();
        assert!(queue.enqueue(MergeRequest::new(
            "plan-a",
            "roko/plan-a",
            vec!["src/lib.rs".into()],
            10,
        )));
        assert!(queue.mark_merging("plan-a"));
        let merger = PlanMerger::new(queue.clone(), cfg);

        let dispatch = merger.submit(MergeRequest::new(
            "plan-b",
            "roko/plan-b",
            vec!["src/lib.rs".into()],
            0,
        ));
        assert!(
            matches!(dispatch, MergeDispatch::Blocked { ref plan_id, launch: None } if plan_id == "plan-b")
        );
    }

    #[tokio::test]
    async fn submit_cannot_secretly_launch_a_different_reserved_plan() {
        let gate = Arc::new(StubGate::new(RegressionOutcome::pass("ok", 1)));
        let merge: Arc<dyn MergeBackend> = Arc::new(StubMerge {
            outcome: MergeBackendOutcome::pass("merge ok", 1),
        });
        let config = PlanMergerConfig::new(PathBuf::from("/tmp"), Duration::from_secs(5))
            .with_merge_backend(merge)
            .with_regression_gate(gate.clone());
        let queue = MergeQueue::new();
        assert!(queue.enqueue(MergeRequest::new(
            "plan-b",
            "roko/plan-b",
            vec!["b.rs".into()],
            100,
        )));
        let merger = PlanMerger::new(queue, config);

        let MergeDispatch::Blocked {
            plan_id,
            launch: Some(launch),
        } = merger.submit(MergeRequest::new(
            "plan-a",
            "roko/plan-a",
            vec!["a.rs".into()],
            0,
        ))
        else {
            panic!("higher-priority queued plan should be returned as a launch token");
        };
        assert_eq!(plan_id, "plan-a");
        assert_eq!(launch.plan_id(), "plan-b");
        tokio::task::yield_now().await;
        assert!(gate.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn duplicate_active_submit_is_explicitly_rejected() {
        let merger = merger_with_gate(Arc::new(StubGate::new(RegressionOutcome::pass("ok", 1))));
        let request = MergeRequest::new("plan-a", "roko/plan-a", vec!["a.rs".into()], 1);
        assert!(matches!(
            merger.submit(request.clone()),
            MergeDispatch::Reserved { .. }
        ));
        assert_eq!(
            merger.submit(request),
            MergeDispatch::AlreadyActive {
                plan_id: "plan-a".to_string()
            }
        );
    }

    #[tokio::test]
    async fn dropping_start_token_cannot_run_dormant_merge() {
        let gate = Arc::new(StubGate::new(RegressionOutcome::pass("ok", 1)));
        let merger = merger_with_gate(gate.clone());
        let MergeDispatch::Reserved { launch } = merger.submit(MergeRequest::new(
            "plan-a",
            "roko/plan-a",
            vec!["a.rs".into()],
            0,
        )) else {
            panic!("expected reservation");
        };
        let (tx, mut rx) = mpsc::channel(1);
        let producer = merger.prepare(launch, tx);
        drop(producer.start);
        producer
            .handle
            .await
            .expect("dormant producer exits cleanly");
        assert!(gate.calls.lock().unwrap().is_empty());
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty) | Err(mpsc::error::TryRecvError::Disconnected)
        ));
    }

    #[tokio::test]
    async fn prepared_merge_carries_exact_launch_effect_and_resolution() {
        let merger = merger_with_gate(Arc::new(StubGate::new(RegressionOutcome::pass("ok", 1))));
        let MergeDispatch::Reserved { launch } = merger.submit(MergeRequest::new(
            "plan-a",
            "roko/plan-a",
            vec!["a.rs".into()],
            0,
        )) else {
            panic!("expected reservation");
        };
        let expected = launch.effect();
        let (tx, _rx) = mpsc::channel(1);
        let producer = merger.prepare(launch, tx);

        assert_eq!(producer.effect, expected);
        assert_eq!(producer.effect.attempt.plan_id, "plan-a");
        assert_eq!(producer.effect.attempt.task_id, "merge:roko/plan-a");
        drop(producer.start);
        producer.handle.await.unwrap();
        assert!(merger.fail_resolution(producer.resolution, "setup rollback"));
    }

    #[tokio::test]
    async fn producer_panic_emits_exact_exceptional_completion() {
        let cfg = PlanMergerConfig::new(PathBuf::from("/tmp"), Duration::from_secs(5))
            .with_merge_backend(Arc::new(PanickingMerge))
            .with_regression_gate(Arc::new(StubGate::new(RegressionOutcome::pass("ok", 1))));
        let merger = PlanMerger::new(MergeQueue::new(), cfg);
        let MergeDispatch::Reserved { launch } = merger.submit(MergeRequest::new(
            "plan-a",
            "roko/plan-a",
            vec!["a.rs".into()],
            0,
        )) else {
            panic!("expected reservation");
        };
        let expected = launch.effect();
        let (tx, mut rx) = mpsc::channel(1);
        let producer = merger.prepare(launch, tx);
        producer.start.send(()).unwrap();

        let completion = rx.recv().await.unwrap();
        producer.handle.await.unwrap();
        assert_eq!(completion.effect.as_ref(), Some(&expected));
        assert_eq!(completion.attempt.as_ref(), Some(&expected.attempt));
        assert!(!completion.passed);
        assert_eq!(completion.verdicts[0].gate_name, "merge-producer-exception");
        assert!(merger.fail_resolution(producer.resolution, &completion.output));
    }

    #[tokio::test]
    async fn drain_returns_exact_descriptor_without_spawning() {
        let gate = Arc::new(StubGate::new(RegressionOutcome::pass("ok", 1)));
        let merger = merger_with_gate(gate.clone());
        assert!(merger.queue.enqueue(MergeRequest::new(
            "plan-a",
            "roko/plan-a",
            vec!["a.rs".into()],
            7,
        )));
        let launch = merger.drain_next().expect("queued merge descriptor");
        assert_eq!(launch.plan_id(), "plan-a");
        assert_eq!(launch.branch_name(), "roko/plan-a");
        assert!(launch.generation() > 0);
        tokio::task::yield_now().await;
        assert!(gate.calls.lock().unwrap().is_empty());
    }

    // ── G04 tests ──────────────────────────────────────────────────────

    #[test]
    fn merge_backend_outcome_fail_with_conflicts_carries_paths() {
        let outcome = MergeBackendOutcome::fail_with_conflicts(
            "merge failed",
            RunnerFailureKind::Structural,
            42,
            vec!["src/main.rs".into(), "lib.rs".into()],
        );
        assert!(!outcome.passed);
        assert_eq!(outcome.conflicted_paths.len(), 2);
        assert_eq!(outcome.conflicted_paths[0], "src/main.rs");
        assert_eq!(outcome.conflicted_paths[1], "lib.rs");
        assert_eq!(outcome.duration_ms, 42);
    }

    #[test]
    fn merge_backend_outcome_pass_has_empty_conflicts() {
        let outcome = MergeBackendOutcome::pass("ok", 1);
        assert!(outcome.passed);
        assert!(outcome.conflicted_paths.is_empty());
    }

    #[test]
    fn merge_backend_outcome_fail_has_empty_conflicts() {
        let outcome = MergeBackendOutcome::fail("fail", RunnerFailureKind::Structural, 1);
        assert!(!outcome.passed);
        assert!(outcome.conflicted_paths.is_empty());
    }
}
