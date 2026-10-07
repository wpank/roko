//! Prompt assembly bundle.
//!
//! This module holds the prompt assembly interface that was previously
//! owned by the CLI dispatch module. The prompt context cache has one owner,
//! roko-cli's `dispatch/prompt_cache.rs` (backlog 4203). The canonical
//! construction path lives in
//! [`RuntimeServicesBuilder`](crate::builder::RuntimeServicesBuilder).

pub mod builder;
pub mod graph_registry;

pub use builder::PromptBuildHandle;
pub use graph_registry::{COMPOSE_CELL_COUNT, register_compose_cells};
