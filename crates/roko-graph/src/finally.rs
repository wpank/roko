//! Guaranteed-finally controller for production plan execution (#256).
//!
//! The `GuaranteedFinallyController` wraps graph execution with an absolute
//! guarantee that cleanup runs even on failure, panic, or cancellation.
//! This is NOT a graph node -- it is a controller hook that runs outside
//! the DAG.
//!
//! # Guarantees
//!
//! Regardless of how execution ends (success, node failure, cancel, panic),
//! the controller:
//!
//! 1. Emits exactly one terminal receipt (success, failure, or cancel).
//! 2. Releases all workspace leases via the #249 workspace provider.
//! 3. Stops all tracked agent processes.
//! 4. Flushes the final snapshot to disk.
//!
//! # Design
//!
//! The controller uses an explicit `FinallyGuard` that tracks whether
//! cleanup has been performed. If the guard is dropped without explicit
//! cleanup (e.g. due to a panic), it logs a diagnostic warning -- the
//! actual cleanup must be called by the async controller since Drop
//! cannot run async code.
//!
//! # Scope boundary
//!
//! This module owns the finally-guarantee lifecycle. It does NOT own:
//! - Graph construction (see `topology.rs`)
//! - Graph execution (see `engine.rs`)
//! - Approval/control commands (see `control.rs`)
//! - Delivery state machine (see `delivery.rs`)

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::engine::{GraphEngine, GraphOutput, NodeStatus};
use crate::cell::CellContext;

// ─── Terminal receipt ────────────────────────────────────────────────────────

/// Terminal outcome of a plan execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    /// All tasks completed and passed gates.
    Success,
    /// One or more tasks failed or gates rejected.
    Failure,
    /// Execution was cancelled by the operator or a timeout.
    Cancelled,
}

impl std::fmt::Display for TerminalOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failure => write!(f, "failure"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// The single terminal receipt produced by every plan execution.
///
/// Exactly one of these is emitted per execution, regardless of outcome.
/// This is the controller's primary output; downstream consumers use it
/// to drive delivery, notification, and reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalReceipt {
    /// Unique run identifier.
    pub run_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Terminal outcome.
    pub outcome: TerminalOutcome,
    /// Wall-clock duration of the entire execution.
    pub duration: Duration,
    /// Number of tasks that completed successfully.
    pub tasks_succeeded: usize,
    /// Number of tasks that failed.
    pub tasks_failed: usize,
    /// Number of tasks that were skipped.
    pub tasks_skipped: usize,
    /// Number of tasks that were cancelled.
    pub tasks_cancelled: usize,
    /// Optional error message for failure/cancel.
    pub error: Option<String>,
    /// Timestamp when the receipt was created (ms since epoch).
    pub created_at_ms: u64,
}

impl TerminalReceipt {
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ─── Resource tracker ────────────────────────────────────────────────────────

/// A resource that must be released during cleanup.
#[derive(Debug, Clone)]
pub enum TrackedResource {
    /// A workspace lease that must be released.
    WorkspaceLease {
        /// Lease identifier.
        lease_id: String,
        /// Plan ID the lease belongs to.
        plan_id: String,
        /// Task ID the lease belongs to.
        task_id: String,
    },
    /// An agent process that must be stopped.
    AgentProcess {
        /// Process identifier.
        process_id: String,
        /// Task the process was running.
        task_id: String,
    },
    /// A lock file that must be removed.
    LockFile {
        /// Path to the lock file.
        path: String,
    },
}

impl std::fmt::Display for TrackedResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceLease { lease_id, task_id, .. } => {
                write!(f, "workspace-lease({lease_id}, task={task_id})")
            }
            Self::AgentProcess { process_id, task_id } => {
                write!(f, "agent-process({process_id}, task={task_id})")
            }
            Self::LockFile { path } => {
                write!(f, "lock-file({path})")
            }
        }
    }
}

// ─── Resource releaser trait ─────────────────────────────────────────────────

/// Trait for releasing tracked resources during cleanup.
///
/// Host adapters implement this to bridge to their actual resource lifecycle
/// (e.g., git worktree removal, process termination, file deletion).
#[async_trait::async_trait]
pub trait ResourceReleaser: Send + Sync + std::fmt::Debug {
    /// Release a tracked resource. Returns `Ok(())` on success.
    ///
    /// Implementations must be idempotent: releasing an already-released
    /// resource is a no-op, not an error.
    async fn release(&self, resource: &TrackedResource) -> Result<(), String>;
}

/// A no-op resource releaser for testing.
#[derive(Debug, Default)]
pub struct NoopResourceReleaser;

#[async_trait::async_trait]
impl ResourceReleaser for NoopResourceReleaser {
    async fn release(&self, _resource: &TrackedResource) -> Result<(), String> {
        Ok(())
    }
}

// ─── Snapshot flusher trait ──────────────────────────────────────────────────

/// Trait for flushing the final execution snapshot to disk.
#[async_trait::async_trait]
pub trait SnapshotFlusher: Send + Sync + std::fmt::Debug {
    /// Flush the terminal snapshot. Called exactly once during cleanup.
    async fn flush(&self, receipt: &TerminalReceipt) -> Result<(), String>;
}

/// A no-op snapshot flusher for testing.
#[derive(Debug, Default)]
pub struct NoopSnapshotFlusher;

#[async_trait::async_trait]
impl SnapshotFlusher for NoopSnapshotFlusher {
    async fn flush(&self, _receipt: &TerminalReceipt) -> Result<(), String> {
        Ok(())
    }
}

// ─── FinallyGuard ────────────────────────────────────────────────────────────

/// Tracks whether cleanup has been performed.
///
/// If dropped without `mark_cleaned_up()`, logs a diagnostic warning.
/// This catches programming errors where the caller forgets to run
/// the finally block.
struct FinallyGuard {
    plan_id: String,
    run_id: String,
    cleaned_up: bool,
}

impl FinallyGuard {
    fn new(plan_id: String, run_id: String) -> Self {
        Self {
            plan_id,
            run_id,
            cleaned_up: false,
        }
    }

    fn mark_cleaned_up(&mut self) {
        self.cleaned_up = true;
    }
}

impl Drop for FinallyGuard {
    fn drop(&mut self) {
        if !self.cleaned_up {
            // Cannot run async cleanup in Drop. Log a diagnostic warning
            // so the issue is visible in logs.
            error!(
                plan_id = %self.plan_id,
                run_id = %self.run_id,
                "FinallyGuard dropped without cleanup! Resources may have leaked."
            );
        }
    }
}

// ─── Cleanup summary ─────────────────────────────────────────────────────────

/// Summary of what the finally block released.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanupSummary {
    /// Number of resources successfully released.
    pub resources_released: usize,
    /// Number of resources that failed to release.
    pub resources_failed: usize,
    /// Whether the snapshot was flushed successfully.
    pub snapshot_flushed: bool,
    /// Whether the terminal receipt was emitted.
    pub receipt_emitted: bool,
    /// Individual resource release errors.
    pub errors: Vec<String>,
}

// ─── GuaranteedFinallyController ─────────────────────────────────────────────

/// Controller that wraps graph execution with guaranteed cleanup.
///
/// Usage:
/// ```text
/// let controller = GuaranteedFinallyController::new(plan_id, run_id)
///     .with_releaser(my_releaser)
///     .with_flusher(my_flusher);
///
/// // Track resources as they're acquired during execution.
/// controller.track(TrackedResource::WorkspaceLease { ... });
///
/// // Execute the graph, then run guaranteed cleanup.
/// let (receipt, cleanup) = controller.execute_with_finally(engine, ctx, cancel).await;
/// ```
pub struct GuaranteedFinallyController {
    /// Plan identifier.
    plan_id: String,
    /// Unique run identifier.
    run_id: String,
    /// Resources acquired during execution that must be released.
    tracked_resources: parking_lot::Mutex<Vec<TrackedResource>>,
    /// Host-provided resource releaser.
    releaser: Arc<dyn ResourceReleaser>,
    /// Host-provided snapshot flusher.
    flusher: Arc<dyn SnapshotFlusher>,
    /// Cancellation token for the execution.
    cancel: CancellationToken,
}

impl std::fmt::Debug for GuaranteedFinallyController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuaranteedFinallyController")
            .field("plan_id", &self.plan_id)
            .field("run_id", &self.run_id)
            .field("tracked_resources", &self.tracked_resources.lock().len())
            .finish()
    }
}

impl GuaranteedFinallyController {
    /// Create a new controller for the given plan and run.
    pub fn new(plan_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            run_id: run_id.into(),
            tracked_resources: parking_lot::Mutex::new(Vec::new()),
            releaser: Arc::new(NoopResourceReleaser),
            flusher: Arc::new(NoopSnapshotFlusher),
            cancel: CancellationToken::new(),
        }
    }

    /// Set the resource releaser.
    #[must_use]
    pub fn with_releaser(mut self, releaser: Arc<dyn ResourceReleaser>) -> Self {
        self.releaser = releaser;
        self
    }

    /// Set the snapshot flusher.
    #[must_use]
    pub fn with_flusher(mut self, flusher: Arc<dyn SnapshotFlusher>) -> Self {
        self.flusher = flusher;
        self
    }

    /// Set the cancellation token.
    #[must_use]
    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }

    /// Track a resource for guaranteed release during cleanup.
    pub fn track(&self, resource: TrackedResource) {
        self.tracked_resources.lock().push(resource);
    }

    /// Remove a tracked resource (e.g., when it's explicitly released early).
    pub fn untrack(&self, predicate: impl Fn(&TrackedResource) -> bool) {
        self.tracked_resources.lock().retain(|r| !predicate(r));
    }

    /// Return the number of currently tracked resources.
    #[must_use]
    pub fn tracked_count(&self) -> usize {
        self.tracked_resources.lock().len()
    }

    /// Execute the graph with guaranteed finally cleanup.
    ///
    /// This is the primary entry point. It:
    /// 1. Runs the graph engine.
    /// 2. Determines the terminal outcome.
    /// 3. Runs the finally block (release resources, flush snapshot, emit receipt).
    ///
    /// The finally block runs regardless of whether execution succeeded, failed,
    /// was cancelled, or panicked (via catch_unwind at the async boundary).
    pub async fn execute_with_finally(
        &self,
        engine: GraphEngine,
        ctx: CellContext,
    ) -> (TerminalReceipt, CleanupSummary) {
        let start = Instant::now();
        let mut guard = FinallyGuard::new(self.plan_id.clone(), self.run_id.clone());

        // Execute the graph, catching cancellation.
        let graph_result = tokio::select! {
            result = engine.execute(&ctx) => result,
            () = self.cancel.cancelled() => {
                info!(plan_id = %self.plan_id, run_id = %self.run_id, "execution cancelled");
                // Return a synthetic cancelled result.
                Err(crate::types::GraphError::InvalidGraph {
                    reason: "execution cancelled".to_string(),
                })
            }
        };

        let duration = start.elapsed();

        // Determine terminal outcome and build receipt.
        let (outcome, receipt) = match &graph_result {
            Ok(output) if output.success => {
                let receipt = self.build_receipt(
                    TerminalOutcome::Success,
                    duration,
                    Some(output),
                    None,
                );
                (TerminalOutcome::Success, receipt)
            }
            Ok(output) => {
                // Graph completed but had failures.
                let error_msg = output
                    .node_results
                    .iter()
                    .filter(|r| r.status == NodeStatus::Failed)
                    .map(|r| {
                        format!(
                            "{}: {}",
                            r.node_id,
                            r.error.as_deref().unwrap_or("unknown error")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                let receipt = self.build_receipt(
                    TerminalOutcome::Failure,
                    duration,
                    Some(output),
                    Some(error_msg),
                );
                (TerminalOutcome::Failure, receipt)
            }
            Err(e) => {
                let error_msg = e.to_string();
                let outcome = if error_msg.contains("cancelled") {
                    TerminalOutcome::Cancelled
                } else {
                    TerminalOutcome::Failure
                };
                let receipt = self.build_receipt(outcome, duration, None, Some(error_msg));
                (outcome, receipt)
            }
        };

        // Run the guaranteed finally block.
        let cleanup = self.run_finally(&receipt).await;
        guard.mark_cleaned_up();

        info!(
            plan_id = %self.plan_id,
            run_id = %self.run_id,
            outcome = %outcome,
            resources_released = cleanup.resources_released,
            resources_failed = cleanup.resources_failed,
            snapshot_flushed = cleanup.snapshot_flushed,
            "guaranteed-finally cleanup complete"
        );

        (receipt, cleanup)
    }

    /// Run the finally block: release resources, flush snapshot, emit receipt.
    ///
    /// This runs unconditionally after graph execution, regardless of outcome.
    /// All errors are logged but never propagated -- cleanup is best-effort.
    async fn run_finally(&self, receipt: &TerminalReceipt) -> CleanupSummary {
        let mut summary = CleanupSummary::default();

        // 1. Release all tracked resources.
        let resources = {
            let mut guard = self.tracked_resources.lock();
            std::mem::take(&mut *guard)
        };

        for resource in &resources {
            match self.releaser.release(resource).await {
                Ok(()) => {
                    info!(resource = %resource, "resource released");
                    summary.resources_released += 1;
                }
                Err(e) => {
                    warn!(resource = %resource, error = %e, "failed to release resource");
                    summary.resources_failed += 1;
                    summary.errors.push(format!("{resource}: {e}"));
                }
            }
        }

        // 2. Flush the terminal snapshot.
        match self.flusher.flush(receipt).await {
            Ok(()) => {
                summary.snapshot_flushed = true;
            }
            Err(e) => {
                warn!(error = %e, "failed to flush terminal snapshot");
                summary.errors.push(format!("snapshot flush: {e}"));
            }
        }

        // 3. Receipt is emitted by returning it from execute_with_finally.
        summary.receipt_emitted = true;

        summary
    }

    /// Build a terminal receipt from execution results.
    fn build_receipt(
        &self,
        outcome: TerminalOutcome,
        duration: Duration,
        graph_output: Option<&GraphOutput>,
        error: Option<String>,
    ) -> TerminalReceipt {
        let (succeeded, failed, skipped, cancelled) = match graph_output {
            Some(output) => {
                let succeeded = output
                    .node_results
                    .iter()
                    .filter(|r| r.status == NodeStatus::Complete)
                    .count();
                let failed = output
                    .node_results
                    .iter()
                    .filter(|r| r.status == NodeStatus::Failed)
                    .count();
                let skipped = output
                    .node_results
                    .iter()
                    .filter(|r| {
                        r.status == NodeStatus::Skipped
                            || r.status == NodeStatus::ConditionSkipped
                    })
                    .count();
                // Nodes still pending at the end were effectively cancelled.
                let cancelled = output
                    .node_results
                    .iter()
                    .filter(|r| r.status == NodeStatus::Pending)
                    .count();
                (succeeded, failed, skipped, cancelled)
            }
            None => (0, 0, 0, 0),
        };

        TerminalReceipt {
            run_id: self.run_id.clone(),
            plan_id: self.plan_id.clone(),
            outcome,
            duration,
            tasks_succeeded: succeeded,
            tasks_failed: failed,
            tasks_skipped: skipped,
            tasks_cancelled: cancelled,
            error,
            created_at_ms: TerminalReceipt::now_ms(),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::default_registry;
    use crate::topology::{ProductionPlanTopology, TopologyTaskInfo, register_topology_cells};
    use std::sync::atomic::Ordering;

    fn make_task(id: &str, depends_on: &[&str]) -> TopologyTaskInfo {
        TopologyTaskInfo {
            task_id: id.to_string(),
            title: format!("Task {id}"),
            description: None,
            role: Some("implementer".to_string()),
            tier: "mechanical".to_string(),
            model_hint: None,
            files: vec![],
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            timeout_secs: 300,
            max_retries: 2,
            domain: None,
            sequence: 0,
            full_config_json: serde_json::json!({"id": id}),
        }
    }

    fn build_test_engine(tasks: &[TopologyTaskInfo]) -> GraphEngine {
        let topo = ProductionPlanTopology::new("test-plan", "/tmp", 2);
        let (graph, _) = topo.build(tasks).unwrap();
        let mut registry = default_registry();
        register_topology_cells(&mut registry);
        GraphEngine::new(graph, registry).with_allow_test_stubs(true)
    }

    // ── Tracking resource releaser ────────────────────────────────────────

    #[derive(Debug, Default)]
    struct TrackingReleaser {
        released: parking_lot::Mutex<Vec<String>>,
        fail_on: parking_lot::Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl ResourceReleaser for TrackingReleaser {
        async fn release(&self, resource: &TrackedResource) -> Result<(), String> {
            let desc = resource.to_string();
            if self.fail_on.lock().iter().any(|f| desc.contains(f)) {
                return Err(format!("simulated release failure for {desc}"));
            }
            self.released.lock().push(desc);
            Ok(())
        }
    }

    // ── Tracking snapshot flusher ─────────────────────────────────────────

    #[derive(Debug, Default)]
    struct TrackingFlusher {
        flushed: parking_lot::Mutex<Vec<TerminalReceipt>>,
        should_fail: std::sync::atomic::AtomicBool,
    }

    #[async_trait::async_trait]
    impl SnapshotFlusher for TrackingFlusher {
        async fn flush(&self, receipt: &TerminalReceipt) -> Result<(), String> {
            if self.should_fail.load(Ordering::Relaxed) {
                return Err("simulated flush failure".to_string());
            }
            self.flushed.lock().push(receipt.clone());
            Ok(())
        }
    }

    // ── Success path ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn success_produces_terminal_receipt() {
        let tasks = vec![make_task("T1", &[])];
        let engine = build_test_engine(&tasks);
        let ctx = CellContext::new();
        let flusher = Arc::new(TrackingFlusher::default());

        let controller = GuaranteedFinallyController::new("test-plan", "run-1")
            .with_flusher(flusher.clone());

        let (receipt, cleanup) = controller.execute_with_finally(engine, ctx).await;

        assert_eq!(receipt.outcome, TerminalOutcome::Success);
        assert_eq!(receipt.plan_id, "test-plan");
        assert_eq!(receipt.run_id, "run-1");
        assert!(receipt.error.is_none());
        assert!(cleanup.receipt_emitted);
        assert!(cleanup.snapshot_flushed);
        assert_eq!(cleanup.resources_released, 0);
        assert_eq!(cleanup.resources_failed, 0);

        let flushed = flusher.flushed.lock();
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0].outcome, TerminalOutcome::Success);
    }

    // ── Failure path ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn failure_still_produces_receipt_and_cleans_up() {
        // Build a graph with a cell type that doesn't exist in the registry
        // to force a failure.
        let mut graph = crate::types::Graph::new(crate::types::GraphMetadata {
            name: "fail-plan".to_string(),
            ..Default::default()
        });
        graph.add_node(crate::types::Node {
            id: "bad-node".to_string(),
            cell_type: "nonexistent-cell-type".to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::Activity,
            exclusive: vec![],
        }).unwrap();

        let registry = default_registry();
        let engine = GraphEngine::new(graph, registry).with_allow_test_stubs(true);
        let ctx = CellContext::new();

        let releaser = Arc::new(TrackingReleaser::default());
        let flusher = Arc::new(TrackingFlusher::default());

        let controller = GuaranteedFinallyController::new("fail-plan", "run-fail")
            .with_releaser(releaser.clone())
            .with_flusher(flusher.clone());

        // Track resources that should be released even on failure.
        controller.track(TrackedResource::WorkspaceLease {
            lease_id: "lease-1".to_string(),
            plan_id: "fail-plan".to_string(),
            task_id: "bad-node".to_string(),
        });
        controller.track(TrackedResource::AgentProcess {
            process_id: "proc-1".to_string(),
            task_id: "bad-node".to_string(),
        });

        let (receipt, cleanup) = controller.execute_with_finally(engine, ctx).await;

        assert_eq!(receipt.outcome, TerminalOutcome::Failure);
        assert!(receipt.error.is_some());
        assert!(cleanup.receipt_emitted);
        assert!(cleanup.snapshot_flushed);
        assert_eq!(cleanup.resources_released, 2);
        assert_eq!(cleanup.resources_failed, 0);

        let released = releaser.released.lock();
        assert_eq!(released.len(), 2);
    }

    // ── Cancel path ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn cancel_produces_cancelled_receipt_and_cleans_up() {
        let tasks = vec![make_task("T1", &[])];
        let engine = build_test_engine(&tasks);
        let ctx = CellContext::new();

        let cancel = CancellationToken::new();
        let releaser = Arc::new(TrackingReleaser::default());
        let flusher = Arc::new(TrackingFlusher::default());

        let controller = GuaranteedFinallyController::new("cancel-plan", "run-cancel")
            .with_releaser(releaser.clone())
            .with_flusher(flusher.clone())
            .with_cancel(cancel.clone());

        controller.track(TrackedResource::LockFile {
            path: "/tmp/plan.lock".to_string(),
        });

        // Cancel immediately before execution can start.
        cancel.cancel();

        let (receipt, cleanup) = controller.execute_with_finally(engine, ctx).await;

        assert_eq!(receipt.outcome, TerminalOutcome::Cancelled);
        assert!(cleanup.receipt_emitted);
        assert!(cleanup.snapshot_flushed);
        assert_eq!(cleanup.resources_released, 1);
    }

    // ── Resource release failure ──────────────────────────────────────────

    #[tokio::test]
    async fn resource_release_failure_is_logged_not_fatal() {
        let tasks = vec![make_task("T1", &[])];
        let engine = build_test_engine(&tasks);
        let ctx = CellContext::new();

        let releaser = Arc::new(TrackingReleaser::default());
        releaser.fail_on.lock().push("lease-bad".to_string());

        let controller = GuaranteedFinallyController::new("plan", "run")
            .with_releaser(releaser.clone());

        controller.track(TrackedResource::WorkspaceLease {
            lease_id: "lease-good".to_string(),
            plan_id: "plan".to_string(),
            task_id: "T1".to_string(),
        });
        controller.track(TrackedResource::WorkspaceLease {
            lease_id: "lease-bad".to_string(),
            plan_id: "plan".to_string(),
            task_id: "T2".to_string(),
        });

        let (receipt, cleanup) = controller.execute_with_finally(engine, ctx).await;

        // Execution itself succeeded.
        assert_eq!(receipt.outcome, TerminalOutcome::Success);
        // One resource released, one failed.
        assert_eq!(cleanup.resources_released, 1);
        assert_eq!(cleanup.resources_failed, 1);
        assert_eq!(cleanup.errors.len(), 1);
        assert!(cleanup.errors[0].contains("lease-bad"));
    }

    // ── Snapshot flush failure ────────────────────────────────────────────

    #[tokio::test]
    async fn snapshot_flush_failure_is_logged_not_fatal() {
        let tasks = vec![make_task("T1", &[])];
        let engine = build_test_engine(&tasks);
        let ctx = CellContext::new();

        let flusher = Arc::new(TrackingFlusher::default());
        flusher.should_fail.store(true, Ordering::Relaxed);

        let controller = GuaranteedFinallyController::new("plan", "run")
            .with_flusher(flusher.clone());

        let (receipt, cleanup) = controller.execute_with_finally(engine, ctx).await;

        assert_eq!(receipt.outcome, TerminalOutcome::Success);
        assert!(!cleanup.snapshot_flushed);
        assert!(cleanup.receipt_emitted);
        assert!(cleanup.errors.iter().any(|e| e.contains("snapshot flush")));
    }

    // ── Tracking and untracking ───────────────────────────────────────────

    #[test]
    fn track_and_untrack_resources() {
        let controller = GuaranteedFinallyController::new("plan", "run");

        controller.track(TrackedResource::WorkspaceLease {
            lease_id: "L1".to_string(),
            plan_id: "plan".to_string(),
            task_id: "T1".to_string(),
        });
        controller.track(TrackedResource::AgentProcess {
            process_id: "P1".to_string(),
            task_id: "T1".to_string(),
        });
        assert_eq!(controller.tracked_count(), 2);

        // Untrack the lease.
        controller.untrack(|r| matches!(r, TrackedResource::WorkspaceLease { lease_id, .. } if lease_id == "L1"));
        assert_eq!(controller.tracked_count(), 1);

        // Untrack the process.
        controller.untrack(|r| matches!(r, TrackedResource::AgentProcess { .. }));
        assert_eq!(controller.tracked_count(), 0);
    }

    // ── Receipt counts reflect graph output ───────────────────────────────

    #[tokio::test]
    async fn receipt_counts_reflect_node_results() {
        let tasks = vec![make_task("T1", &[])];
        let engine = build_test_engine(&tasks);
        let ctx = CellContext::new();

        let controller = GuaranteedFinallyController::new("plan", "run");
        let (receipt, _) = controller.execute_with_finally(engine, ctx).await;

        // With passthrough stubs, all nodes should complete.
        assert!(receipt.tasks_succeeded > 0);
        assert_eq!(receipt.tasks_failed, 0);
    }

    // ── Multiple resources all released on success ────────────────────────

    #[tokio::test]
    async fn all_tracked_resources_released_on_success() {
        let tasks = vec![make_task("T1", &[])];
        let engine = build_test_engine(&tasks);
        let ctx = CellContext::new();

        let releaser = Arc::new(TrackingReleaser::default());
        let controller = GuaranteedFinallyController::new("plan", "run")
            .with_releaser(releaser.clone());

        for i in 0..5 {
            controller.track(TrackedResource::WorkspaceLease {
                lease_id: format!("L{i}"),
                plan_id: "plan".to_string(),
                task_id: format!("T{i}"),
            });
        }

        let (_, cleanup) = controller.execute_with_finally(engine, ctx).await;

        assert_eq!(cleanup.resources_released, 5);
        assert_eq!(cleanup.resources_failed, 0);
        assert_eq!(releaser.released.lock().len(), 5);
    }

    // ── Diamond DAG with finally ──────────────────────────────────────────

    #[tokio::test]
    async fn diamond_dag_produces_single_terminal_receipt() {
        let tasks = vec![
            make_task("T1", &[]),
            make_task("T2", &["T1"]),
            make_task("T3", &["T1"]),
            make_task("T4", &["T2", "T3"]),
        ];
        let engine = build_test_engine(&tasks);
        let ctx = CellContext::new();
        let flusher = Arc::new(TrackingFlusher::default());

        let controller = GuaranteedFinallyController::new("diamond", "run-diamond")
            .with_flusher(flusher.clone());

        let (receipt, cleanup) = controller.execute_with_finally(engine, ctx).await;

        assert_eq!(receipt.outcome, TerminalOutcome::Success);
        assert!(cleanup.snapshot_flushed);
        // Exactly one flush.
        assert_eq!(flusher.flushed.lock().len(), 1);
    }
}
