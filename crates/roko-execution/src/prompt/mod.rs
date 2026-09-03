//! Prompt assembly and caching bundle.
//!
//! This module consolidates prompt cache loading and the prompt assembly
//! interface that were previously owned by the CLI dispatch module.
//! The canonical construction path lives in
//! [`RuntimeServicesBuilder`](crate::builder::RuntimeServicesBuilder).

pub mod builder;
pub mod cache;
pub mod graph_registry;

pub use builder::PromptBuildHandle;
pub use cache::PromptCacheHandle;
pub use graph_registry::{COMPOSE_CELL_COUNT, register_compose_cells};
