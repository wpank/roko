//! Workflow graph cells and templates (#257).
//!
//! Expresses single-prompt workflows (mechanical, focused, integrative,
//! architectural) as named Graph templates without requiring graph cycles.
//!
//! # Modules
//!
//! - [`templates`] -- Named workflow template descriptors and alias table.
//!   Builds acyclic subgraphs per generation: Compose -> Implement -> Gate
//!   -> [Review] -> [Commit].
//!
//! - [`controller`] -- Outer workflow lifecycle controller. Manages
//!   generation creation on gate failure / review revise, enforces caps,
//!   computes idempotency keys, and produces durable phase receipts.
//!
//! - [`review_parser`] -- Pure, structured parser for reviewer output.
//!   Accepts exact JSON, fenced JSON, and legacy text format.
//!   Malformed output is `Unclear`, never implicit approval.
//!
//! - [`report`] -- Adapter mapping controller terminal state to
//!   `WorkflowRunReport` from `roko-runtime`.

pub mod controller;
pub mod report;
pub mod review_parser;
pub mod templates;

// Re-export primary types for convenience.
pub use controller::{
    ActivityScope, ControllerAction, PhaseInput, PhaseReceipt, WorkflowGraphController,
    WorkflowPhase, WorkflowTermination, idempotency_key,
};
pub use report::build_report;
pub use review_parser::{ReviewVerdict, parse_review};
pub use templates::{
    CANONICAL_NAMES, TEMPLATE_VERSION, TemplateResolutionError, WorkflowTemplateDescriptor,
    build_autofix_subgraph, build_generation_subgraph, cell_types, node_ids, resolve_template,
    resolve_template_name,
};
