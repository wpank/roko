//! Plan-executor state: per-plan state, the phase state machine,
//! crash-recovery snapshots, recovery, priority ceilings, resource budgets
//! and queue reordering.
//!
//! The parallel executor that drove these types from `[executor]` config was
//! removed (gap-666ab3): no plan run used it, and plans run on the Graph
//! engine. The persisted executor snapshot (`executor.json`) and the types it
//! holds remain.
//!
//! # Sub-modules
//!
//! - [`action`] — the `ExecutorAction` enum
//! - [`plan_state`] — per-plan mutable state
//! - [`snapshot`] — crash-recovery serialization
//! - [`state_machine`] — phase transition logic
//! - [`reorder`] — queue reordering strategies

use roko_core::AgentRole;
use serde::{Deserialize, Serialize};

pub mod action;
pub mod plan_state;
pub mod priority_ceiling;
pub mod recovery;
pub mod reorder;
pub mod resource_budget;
pub mod snapshot;
pub mod state_machine;

pub use action::ExecutorAction;
pub use plan_state::{GateResult, PlanResumeDirective, PlanState};
pub use priority_ceiling::{
    EffectivePriorityTracker, PlanResourceInfo, PriorityCeiling, ResourceId,
};
pub use recovery::{
    RecoveredPlanResume, RecoveredState, RecoveryEngine, RecoveryError, RecoveryResumePlan,
    RecoveryWarning, WarningSeverity,
};
pub use reorder::{priority_reorder, reorder_queue};
pub use resource_budget::{
    CostBudget, FullResourceBudget, RateLimitResource, ResourceCheck, ResourcePool,
    ResourceReservation, TaskResourceRequest, TokenBudget,
};
pub use snapshot::{
    CURRENT_SCHEMA_VERSION, DeltaSnapshot, ExecutorSnapshot, PersistedCircuitBreakerFailureRecord,
    PersistedCircuitBreakerState, SnapshotConfig, SnapshotIntegrityError, SnapshotVerifier,
    current_schema_version,
};
pub use state_machine::{ExecutorEvent, PlanStateMachine, TransitionError};

/// Live speculative execution tracking for dashboard and recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeculativeExecution {
    /// The plan that owns the task.
    pub plan_id: String,
    /// The task being shadowed.
    pub task: String,
    /// The original expectation in minutes.
    pub expected_minutes: u32,
    /// The elapsed runtime that triggered speculation.
    pub elapsed_minutes: u32,
    /// The backup role to spawn.
    pub backup_role: AgentRole,
    /// Projected spend for the backup branch.
    pub projected_cost_usd: f64,
    /// When the speculative branch was recorded.
    pub started_at_ms: u64,
}
