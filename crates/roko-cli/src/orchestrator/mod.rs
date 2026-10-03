//! Orchestration types the Graph plan runner and roko's tools share.
//!
//! Plan discovery, task DAG, worktree management, executor state, replan
//! strategies, and crash-recovery snapshots. Plans reach their branches
//! through `graph_execution::delivery`, not a merge queue.

pub mod dag;
pub mod event_log;
pub mod executor;
pub mod plan_discovery;
pub mod replan;
pub mod scratch;
pub mod worktree;

pub use dag::{
    CpmAnalysis, DAG_EXECUTION_SNAPSHOT_SCHEMA_VERSION, DagConfig, DagError, DagExecutionSnapshot,
    DagMutation, DagMutationError, DagPartition, DagStats, DagTaskExecutionMetadata,
    DagTaskExecutionStatus, Durability, ExecutionWave, FusionConfig, IncrementalDag,
    UnifiedTaskDag, detect_cycle_nodes,
};
pub use event_log::{EventEntry, EventKind, EventLog, EventLogSnapshot, IntegrityError};
pub use executor::{
    CURRENT_SCHEMA_VERSION, DeltaSnapshot, EffectivePriorityTracker, ExecutorAction, ExecutorEvent,
    ExecutorSnapshot, GateResult, PersistedCircuitBreakerFailureRecord,
    PersistedCircuitBreakerState, PlanResourceInfo, PlanResumeDirective, PlanState,
    PlanStateMachine, PriorityCeiling, RecoveredPlanResume, RecoveredState, RecoveryEngine,
    RecoveryError, RecoveryResumePlan, RecoveryWarning, ResourceId, SnapshotConfig,
    SnapshotIntegrityError, SnapshotVerifier, SpeculativeExecution, TransitionError,
    WarningSeverity, current_schema_version,
};
pub use plan_discovery::{
    DiscoveryError, PlanFrontmatter, PlanInfo, ValidationError, discover_plans, parse_frontmatter,
    rank_plans, validate_frontmatter,
};
pub use replan::{
    FailureDisposition, PlanRevisionEvidence, PlanRevisionRequest, ReplanResult, ReplanStrategy,
};
pub use worktree::{
    WorktreeConfig, WorktreeError, WorktreeHandle, WorktreeHealth, WorktreeIsolationStatus,
    WorktreeManager, WorktreeOperationError, WorktreeSnapshot, format_branch_name,
    format_task_branch_name,
};
