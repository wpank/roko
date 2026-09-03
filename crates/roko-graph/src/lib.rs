//! `roko-graph` -- Graph execution engine for the Roko toolkit.
//!
//! This crate provides the foundation for defining and executing directed acyclic
//! graphs (DAGs) of Cells. It includes:
//!
//! - **Cell** trait (universal computation unit for graph nodes)
//! - **Types** (`Graph`, `Node`, `Edge`, `NodeId`, `EdgeCondition`, `GraphMetadata`,
//!   `NodeOutput`, `NodeOutputStatus`, `GraphConfig`, `GraphPolicy`, `GraphMode`,
//!   `FailureStrategy`, `ExecutionClass`, `EdgeValidationError`)
//! - **Loader** (TOML parsing into `Graph` struct)
//! - **Registry** (`CellRegistry` for mapping cell type names to factory functions)
//! - **Topo** (topological sort, cycle detection, dependency resolution)
//! - **Error** (error types and `Result` alias)
//! - **Budget** (`BudgetTracker` and `BudgetEnforcer` for resource limit enforcement)
//! - **Condition** (conditional edge evaluation with `CompareOp` and `Condition`)
//! - **Cells** (built-in cell implementations: `AgentCell`, `ComposeCell`, `GraduationCell`)
//!
//! # Example
//!
//! ```rust
//! use roko_graph::{loader, topo};
//!
//! let toml_str = r#"
//! [graph]
//! name = "example"
//!
//! [[nodes]]
//! id = "step1"
//! cell_type = "noop"
//!
//! [[nodes]]
//! id = "step2"
//! cell_type = "noop"
//!
//! [[edges]]
//! from = "step1"
//! to = "step2"
//! "#;
//!
//! let graph = loader::load_from_str(toml_str).unwrap();
//! let order = topo::topological_order(&graph).unwrap();
//! assert_eq!(order, vec!["step1", "step2"]);
//! ```

pub mod budget;
pub mod cell;
pub mod cells;
pub mod condition;
pub mod control;
pub mod convert;
pub mod delivery;
pub mod engine;
pub mod error;
pub mod events;
pub mod fingerprint;
pub mod hot;
pub mod loader;
pub mod plan_mutation;
pub mod profile;
pub mod registry;
pub mod replay;
pub mod topo;
pub mod types;
pub mod workspace;

// Re-export primary types at crate root for convenience.
pub use cell::{Cell, CellContext, CellVersion};
pub use engine::{
    FlowHandle, FlowStatus, GRAPH_SNAPSHOT_SCHEMA_VERSION, GraphEngine, GraphOutput, GraphSnapshot,
    GraphSnapshotV2, MergeEnqueuer, MergeRequest, NodeResult, NodeStatus, SerializableNodeStatus,
    SerializableSignal, ValidatedGraph, default_registry, reconcile_running_status,
};
pub use registry::{CellDescriptor, CellFactory, CellRegistry};
pub use types::{
    Edge, EdgeCondition, EdgeValidationError, ExecutionClass, FailureStrategy, Graph, GraphConfig,
    GraphError, GraphMetadata, GraphMode, GraphNodeIdx, GraphPolicy, Node, NodeId, NodeOutput,
    NodeOutputStatus,
};

// Re-export from new modules.
pub use budget::{
    BudgetCheckpoint, BudgetEnforcer, BudgetLimits, BudgetTracker, NodeCost, NodeCostCheckpoint,
};
pub use condition::{CompareOp, Condition, evaluate};
pub use convert::{PlanTaskInfo, plan_to_graph, plan_to_graph_with_endpoints};
pub use error::Result as GraphResult;
pub use fingerprint::graph_execution_fingerprint;
pub use hot::{
    HotCheckpointError, HotCheckpointOptions, HotGraphCheckpointManifest, HotGraphFailure,
    HotGraphHandle, HotPolicy, LoopLevel, start_hot, start_hot_resumable,
    start_hot_resumable_with_budget, start_hot_with_budget,
};
pub use replay::{ActivityRecorder, ActivityReplayer, RecordEntry};
pub use roko_core::{LensConfig, LensRegistration, LensRegistry};
pub use workspace::{
    ExecutionWorkspaceProvider, WorkspaceAttemptId, WorkspaceError, WorkspaceLease,
    WorkspaceLeaseState, WorkspaceReconcileResult, WorkspaceReleasePolicy,
};

// Re-export graph execution event contract (#246).
pub use events::{
    BudgetAmounts, CommonFields, DispatchFields, EventSeqCounter, GraphEventDelivery,
    GraphEventDisposition, GraphEventError, GraphEventSink, GraphExecutionEvent, NodeFields,
    TerminalStats, WaveFields, GRAPH_EVENT_SCHEMA_VERSION,
};

// Re-export approval, control, and cancellation ports (#255).
pub use control::{
    ApprovalRequestV1, ApprovalResolution, ControlCommandKind, ControlEffect,
    ControlReceiptV1, ControlSnapshot, ExecutionControlService, FinalizationIntent,
    ReceiptStatus, build_approval_request, CONTROL_EXTENSION_NAME, CONTROL_EXTENSION_VERSION,
};

// Re-export graph-layer replan mutation adapter (#252).
pub use plan_mutation::{
    build_merge_with_rewiring, build_split_with_rewiring, completed_tasks_preserved,
    downstream_tasks, pending_siblings, upstream_tasks,
};

// Re-export completion delivery lifecycle (#254).
pub use delivery::{
    CompletionDeliveryReceiptV1, CompletionDeliveryRequest, CompletionDeliveryService,
    CompletionDeliveryState, DeliveryError, DeliveryReceiptStore, DeliveryTransitionError,
    MergeSlot, MergeSlotBlocked, ReleasePolicy, DELIVERY_EXTENSION_KEY, delivery_extension_value,
};

// Re-export authored graph production profile (#267).
pub use profile::{
    AuthoredGraphProfile, AuthoredGraphProfileBuilder, CapabilityDenial, CellCapabilityDenial,
    DenialReason, ProfileValidationError, RuntimeProfileKind, validate_cell_capabilities,
};
