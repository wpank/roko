//! Layer-3 registration manifest for production compose Cells.
//!
//! This is the **only** layer-3 constructor manifest for the compose graph
//! cells. It maps `RuntimeServices` handles into the layer-safe trait
//! constructors consumed by `roko-compose::graph_cells`.
//!
//! # Architecture
//!
//! `roko-compose` (layer 2) defines the Cell implementations and their
//! provider traits. This module (layer 3) provides the registration function
//! that the coordinator calls to wire everything together:
//!
//! ```text
//! roko-compose (L2)          roko-execution (L3)
//! ┌──────────────────┐      ┌───────────────────────────┐
//! │ KnowledgeCell<P>  │◄─────│ register_compose_cells()  │
//! │ EpisodesCell<P>   │      │  maps RuntimeServices     │
//! │ PlaybookCell<P>   │      │  into provider impls      │
//! │ TaskContextCell<P>│      └───────────────────────────┘
//! │ ModulationCell<P> │
//! │ SafetyCell<P>     │
//! │ ExperimentCell<P> │
//! │ AggregateCell     │
//! └──────────────────┘
//! ```

use roko_compose::graph_cells::{
    self, AggregateCell, EpisodesCell, ExperimentCell, KnowledgeCell, ModulationCell,
    PlaybookCell, SafetyCell, TaskContextCell, cell_ids,
};
use roko_graph::registry::{CellDescriptor, CellRegistry};

/// Register all production compose Cells into the given `CellRegistry`.
///
/// This is the single layer-3 constructor manifest. It creates cells with
/// their default (no-op) providers. The coordinator should replace these
/// with live service handles before graph execution starts.
///
/// # Cell IDs registered
///
/// - `compose.knowledge@1`
/// - `compose.episodes@1`
/// - `compose.playbook@1`
/// - `compose.task_context@1`
/// - `compose.modulation@1`
/// - `compose.safety@1`
/// - `compose.experiment@1`
/// - `compose.aggregate@1`
pub fn register_compose_cells(registry: &mut CellRegistry) {
    let version = (1, 0, 0);

    registry.register_with_descriptor(
        cell_ids::KNOWLEDGE,
        CellDescriptor::new(cell_ids::KNOWLEDGE, version, None, None),
        |_config| Box::new(KnowledgeCell::default()),
    );

    registry.register_with_descriptor(
        cell_ids::EPISODES,
        CellDescriptor::new(cell_ids::EPISODES, version, None, None),
        |_config| Box::new(EpisodesCell::default()),
    );

    registry.register_with_descriptor(
        cell_ids::PLAYBOOK,
        CellDescriptor::new(cell_ids::PLAYBOOK, version, None, None),
        |_config| Box::new(PlaybookCell::default()),
    );

    registry.register_with_descriptor(
        cell_ids::TASK_CONTEXT,
        CellDescriptor::new(cell_ids::TASK_CONTEXT, version, None, None),
        |_config| Box::new(TaskContextCell::default()),
    );

    registry.register_with_descriptor(
        cell_ids::MODULATION,
        CellDescriptor::new(cell_ids::MODULATION, version, None, None),
        |_config| Box::new(ModulationCell::default()),
    );

    registry.register_with_descriptor(
        cell_ids::SAFETY,
        CellDescriptor::new(cell_ids::SAFETY, version, None, None),
        |_config| Box::new(SafetyCell::default()),
    );

    registry.register_with_descriptor(
        cell_ids::EXPERIMENT,
        CellDescriptor::new(cell_ids::EXPERIMENT, version, None, None),
        |_config| Box::new(ExperimentCell::default()),
    );

    registry.register_with_descriptor(
        cell_ids::AGGREGATE,
        CellDescriptor::new(cell_ids::AGGREGATE, version, None, None),
        |_config| Box::new(AggregateCell::default()),
    );
}

/// Returns the count of compose cells registered by [`register_compose_cells`].
///
/// Useful for assertions in integration tests.
pub const COMPOSE_CELL_COUNT: usize = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_compose_cells_registers_all_eight() {
        let mut registry = CellRegistry::new();
        register_compose_cells(&mut registry);

        assert_eq!(registry.len(), COMPOSE_CELL_COUNT);

        for id in cell_ids::ALL_PROVIDERS {
            assert!(
                registry.contains(id),
                "provider cell '{id}' should be registered"
            );
        }
        assert!(
            registry.contains(cell_ids::AGGREGATE),
            "aggregate cell should be registered"
        );
    }

    #[test]
    fn descriptors_have_correct_version() {
        let mut registry = CellRegistry::new();
        register_compose_cells(&mut registry);

        for id in cell_ids::ALL_PROVIDERS
            .iter()
            .chain(std::iter::once(&cell_ids::AGGREGATE))
        {
            let desc = registry
                .descriptor(id)
                .unwrap_or_else(|| panic!("descriptor for '{id}' missing"));
            assert_eq!(desc.version, (1, 0, 0), "version mismatch for '{id}'");
            assert!(!desc.is_stub, "'{id}' should not be a stub");
        }
    }

    #[test]
    fn can_create_cells_from_registry() {
        let mut registry = CellRegistry::new();
        register_compose_cells(&mut registry);

        let empty_config = toml::Value::Table(toml::map::Map::new());
        for id in cell_ids::ALL_PROVIDERS
            .iter()
            .chain(std::iter::once(&cell_ids::AGGREGATE))
        {
            let cell = registry
                .create(id, empty_config.clone())
                .unwrap_or_else(|e| panic!("failed to create '{id}': {e}"));
            assert_eq!(cell.cell_id(), *id);
        }
    }
}
