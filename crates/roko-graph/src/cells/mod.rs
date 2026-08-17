//! Concrete cell implementations for the graph engine.
//!
//! - [`AgentCell`]: wraps LLM agent dispatch (send prompt, get response).
//! - [`ComposeCell`]: runs prompt assembly using roko-compose templates.
//! - [`GraduationCell`]: promotes qualifying Bus Pulses to durable Signals.
//! - [`TaskExecutorCell`]: host-dispatched cell for plan-to-graph converted tasks.
//! - [`PassthroughCell`]: stub cell that passes input through (placeholder for testing).
//! - **Cognitive loop cells**: 7 typed cells for the cognitive execution cycle.

pub mod agent;
pub mod cognitive;
pub mod compose;
pub mod corrigibility;
pub mod graduation;
pub mod immune;
pub mod stubs;
pub mod task_executor;

pub use agent::{AgentCell, AgentCellConfig};
pub use cognitive::{
    ActCell, AssessCell, CognitiveComposeCell, PersistCell, ReactCell, SenseCell, VerifyCell,
};
pub use compose::{ComposeCell, ComposeCellConfig};
pub use corrigibility::{
    CorrigibilityGraphOutput, CorrigibilityPipelineGraph, VerifyDeferenceCell, VerifyImpactCell,
    VerifySwitchCell, VerifyTaskCell, VerifyTruthCell, corrigibility_pipeline_graph,
    register_corrigibility_cells,
};
pub use graduation::GraduationCell;
pub use immune::{
    ImmuneAssessmentCell, ImmuneCellState, ImmuneContainmentCell, ImmuneEscalationCell,
    ImmuneGraphOutput, ImmunePerceptionCell, ImmunePipelineGraph, ImmuneValidationCell,
    immune_pipeline_graph, register_immune_cells,
};
pub use stubs::PassthroughCell;
pub use task_executor::{TaskDispatcher, TaskExecutionSpec, TaskExecutorCell};
