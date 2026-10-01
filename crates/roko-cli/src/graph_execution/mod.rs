//! Graph execution host adapters.
//!
//! This module bridges graph-layer resource ports (defined in `roko-graph`) to
//! CLI-layer implementations: `WorktreeManager` for workspaces, git plumbing
//! in `GitDeliveryBackend` for delivery merges, and `git push` for
//! publication.
//!
//! # Submodules
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`agent_slots`] | Run-wide cap on concurrently executing tasks (`max_agents`) |
//! | [`batch`] | The run's batch branch, and delivery of finished plans into it |
//! | [`control_adapter`] | CLI/TUI command transport to graph-layer control service |
//! | [`delivery`] | Post-execution merge, regression, and publication pipeline |
//! | [`disk_admission`] | Free-disk check at run start, and disk headroom per attempt |
//! | [`event_log`] | `--log-file` JSONL recorder and StateHub event taps |
//! | [`fast_lane`] | FAST lane (`./dev.sh fast`) run deadline |
//! | [`identity_map`] | Graph node-to-plan/task identity resolution |
//! | [`plan_runner`] | Runs a selected plan set through the Graph engine |
//! | [`plan_set`] | Plan-set order, footprints, and which plans may run at once |
//! | [`plan_verify`] | The whole-plan gate: `[meta] verify` on a plan's integrated result |
//! | [`run_manifest`] | Each checkpoint run's `manifest.json`: harness, config hash, invocations |
//! | [`runtime_event_adapter`] | Graph event to canonical runtime event conversion |
//! | [`view_state`] | TUI/HTTP graph status projection |
//! | [`workflow_caller`] | Legacy workflow facade marker |
//! | [`workspaces`] | Worktree-backed execution workspace provider |

pub mod agent_slots;
pub mod batch;
pub mod control_adapter;
pub mod delivery;
pub mod disk_admission;
pub mod event_log;
pub mod fast_lane;
pub mod identity_map;
pub mod plan_runner;
pub mod plan_set;
pub mod plan_verify;
pub mod run_manifest;
pub mod runtime_event_adapter;
pub mod view_state;
pub mod workflow_caller;
pub mod workspaces;

// Re-export primary public types for convenient access.
pub use control_adapter::{GraphCommandEffect, GraphExecutionControlAdapter};
pub use delivery::{
    CliCompletionDeliveryService, DELIVERY_CHECKPOINT_KEY, DeliveryBackend, DeliveryMergeOutcome,
    DeliveryPublicationOutcome, DeliveryRegressionOutcome, GitDeliveryBackend,
};
pub use event_log::{EventTap, RunEventLog, Tapped};
pub use identity_map::{GraphIdentityMap, NodeIdentity};
pub use plan_runner::{GraphPlanRunParams, compute_plan_run_order, run_graph_plan};
pub use plan_set::{PlanSetOrder, plan_set_order};
pub use runtime_event_adapter::GraphRuntimeEventAdapter;
pub use view_state::{
    GraphNodeRow, GraphNodeStatus, GraphStatusSummary, GraphViewState, GraphViewStateProjector,
    HotGraphStatus, SharedGraphViewState, populate_dependencies,
};
pub use workflow_caller::LEGACY_WORKFLOW_ENGINE_FROZEN;
pub use workspaces::WorktreeExecutionWorkspaceProvider;
