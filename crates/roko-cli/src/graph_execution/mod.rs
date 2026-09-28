//! Graph execution host adapters.
//!
//! This module bridges graph-layer resource ports (defined in `roko-graph`) to
//! CLI-layer implementations backed by the existing `WorktreeManager`,
//! `MergeQueue`, and `GitHubWorkflow`.
//!
//! # Submodules
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`control_adapter`] | CLI/TUI command transport to graph-layer control service |
//! | [`delivery`] | Post-execution merge, regression, and publication pipeline |
//! | [`feedback`] | 12-row completion feedback settlement sinks |
//! | [`identity_map`] | Graph node-to-plan/task identity resolution |
//! | [`runtime_event_adapter`] | Graph event to canonical runtime event conversion |
//! | [`view_state`] | TUI/HTTP graph status projection |
//! | [`workflow_caller`] | Canary comparison and legacy facade marker |
//! | [`workspaces`] | Worktree-backed execution workspace provider |

pub mod control_adapter;
pub mod delivery;
pub mod feedback;
pub mod identity_map;
pub mod plan_runner;
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
pub use feedback::{CompletionSinkResult, build_settler};
pub use identity_map::{GraphIdentityMap, NodeIdentity};
pub use plan_runner::{GraphPlanRunParams, run_graph_plan};
pub use runtime_event_adapter::GraphRuntimeEventAdapter;
pub use view_state::{
    GraphNodeRow, GraphNodeStatus, GraphStatusSummary, GraphViewState, GraphViewStateProjector,
    HotGraphStatus, SharedGraphViewState, populate_dependencies,
};
pub use workflow_caller::{
    CanaryAuthoritative, CanaryComparisonReport, LEGACY_WORKFLOW_ENGINE_FROZEN,
    run_canary_comparison,
};
pub use workspaces::WorktreeExecutionWorkspaceProvider;
