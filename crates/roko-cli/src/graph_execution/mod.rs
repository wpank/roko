//! Host adapters that bridge `roko-graph` execution ports to CLI-layer
//! services.
//!
//! Each submodule implements one async trait from `roko-graph` using the
//! existing CLI infrastructure (WorktreeManager, runner event bus, etc.)
//! without introducing a reverse dependency from `roko-graph` to `roko-cli`.

pub mod control_adapter;
pub mod delivery;
pub mod feedback;
pub mod identity_map;
pub mod runtime_event_adapter;
pub mod workflow_host;
pub mod workspaces;
