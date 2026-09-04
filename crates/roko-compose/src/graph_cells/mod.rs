//! Production compose and parallel enrichment Cells for the graph engine.
//!
//! This module replaces the template-substitution-only graph compose path
//! with the production nine-layer prompt pipeline expressed through
//! versioned Signals and parallel enrichment providers.
//!
//! # Architecture
//!
//! Seven enrichment provider Cells consume a [`signals::ComposeRequest`] and
//! produce enrichment payloads in parallel:
//!
//! | Cell ID | Module | Required? |
//! |---------|--------|-----------|
//! | `compose.knowledge@1` | [`knowledge`] | No |
//! | `compose.episodes@1` | [`episodes`] | No |
//! | `compose.playbook@1` | [`playbook`] | No |
//! | `compose.task_context@1` | [`task_context`] | **Yes** |
//! | `compose.modulation@1` | [`modulation`] | No |
//! | `compose.safety@1` | [`safety`] | **Yes** |
//! | `compose.experiment@1` | [`experiment`] | No |
//!
//! The [`aggregate`] Cell consumes one signal from every provider and
//! produces the final [`signals::ComposedPrompt`].
//!
//! # Layer constraints
//!
//! This module lives in `roko-compose` (layer 2) and must not import
//! layer-3 types. Each provider Cell accepts a layer-safe trait handle
//! (e.g., [`knowledge::KnowledgeProvider`]) that is injected from layer 3
//! via the registration manifest in `roko-execution`.

pub mod aggregate;
pub mod episodes;
pub mod experiment;
pub mod knowledge;
pub mod modulation;
pub mod playbook;
pub mod safety;
pub mod signals;
pub mod task_context;

// Re-export the core types for ergonomic use.
pub use aggregate::AggregateCell;
pub use episodes::{EpisodeProvider, EpisodesCell, NoopEpisodeProvider};
pub use experiment::{
    ExperimentCell, ExperimentProvider, ExperimentResult, NoopExperimentProvider,
};
pub use knowledge::{KnowledgeCell, KnowledgeProvider, NoopKnowledgeProvider};
pub use modulation::{ModulationCell, ModulationProvider, NoopModulationProvider};
pub use playbook::{NoopPlaybookProvider, PlaybookCell, PlaybookProvider};
pub use safety::{NoopSafetyContextProvider, SafetyCell, SafetyContextProvider};
pub use signals::{
    ComposeRequest, ComposeScope, ComposedPrompt, EpisodeSections, ExperimentAssignment,
    KnowledgeSections, ModulationSections, PlaybookSections, SafetySections, TaskContextSections,
    cell_ids,
};
pub use task_context::{NoopTaskContextProvider, TaskContextCell, TaskContextProvider};
