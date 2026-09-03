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
pub mod convert;
pub mod engine;
pub mod error;
pub mod fingerprint;
pub mod hot;
pub mod loader;
pub mod registry;
pub mod replay;
pub mod topo;
pub mod types;

// Re-export primary types at crate root for convenience.
pub use cell::{Cell, CellContext, CellVersion};
pub use engine::{
    FlowHandle, FlowStatus, GraphEngine, GraphOutput, GraphSnapshot, MergeEnqueuer, MergeRequest,
    NodeResult, NodeStatus, SerializableNodeStatus, SerializableSignal, ValidatedGraph,
    default_registry,
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
