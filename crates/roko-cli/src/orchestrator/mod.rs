//! Orchestration types owned by the live CLI runner.
//!
//! Plan discovery, task DAG, worktree management, parallel execution,
//! merge queue, replan strategies, and crash-recovery snapshots.

pub mod dag;
pub mod event_log;
pub mod executor;
pub mod merge_queue;
pub mod plan_discovery;
pub mod replan;
pub mod runtime_snapshot;
pub(crate) mod safety;
pub mod worktree;

pub use dag::{
    CpmAnalysis, DAG_EXECUTION_SNAPSHOT_SCHEMA_VERSION, DagConfig, DagError, DagExecutionSnapshot,
    DagMutation, DagMutationError, DagPartition, DagStats, DagTaskExecutionMetadata,
    DagTaskExecutionStatus, Durability, ExecutionWave, FusionConfig, IncrementalDag,
    UnifiedTaskDag, detect_cycle_nodes,
};
pub use event_log::{EventEntry, EventKind, EventLog, EventLogSnapshot, IntegrityError};
pub use executor::{
    CURRENT_SCHEMA_VERSION, DeltaSnapshot, EffectivePriorityTracker, ExecutorAction,
    ExecutorConfig, ExecutorEvent, ExecutorSnapshot, GateResult, ParallelExecutor,
    PersistedCircuitBreakerFailureRecord, PersistedCircuitBreakerState, PlanResourceInfo,
    PlanResumeDirective, PlanState, PlanStateMachine, PriorityCeiling, RecoveredPlanResume,
    RecoveredState, RecoveryEngine, RecoveryError, RecoveryResumePlan, RecoveryWarning,
    ResourceBudget, ResourceId, ResourceUsage, SnapshotConfig, SnapshotIntegrityError,
    SnapshotVerifier, SpeculativeExecution, TransitionError, WarningSeverity,
    current_schema_version,
};
pub use merge_queue::{
    DEFAULT_MAX_MERGE_RETRIES, MergeConflict, MergeQueue, MergeQueueEntrySnapshot,
    MergeQueueMetrics, MergeQueueSnapshot, MergeRequest, MergeReservation, MergeStatus,
    ReservedMerge,
};
pub use plan_discovery::{
    DiscoveryError, PlanFrontmatter, PlanInfo, ValidationError, discover_plans, parse_frontmatter,
    rank_plans, validate_frontmatter,
};
pub use replan::{
    FailureDisposition, PlanRevisionEvidence, PlanRevisionRequest, ReplanResult, ReplanStrategy,
};
pub use runtime_snapshot::{
    ORCHESTRATOR_SNAPSHOT_SCHEMA_VERSION, OrchestratorSnapshot,
    orchestrator_snapshot_schema_version,
};
pub use worktree::{
    WorktreeConfig, WorktreeError, WorktreeHandle, WorktreeHealth, WorktreeIsolationStatus,
    WorktreeManager, WorktreeSnapshot, format_branch_name, format_task_branch_name,
};
