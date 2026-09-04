//! Graph execution host adapters.
//!
//! This module bridges graph-layer resource ports (defined in `roko-graph`) to
//! CLI-layer implementations backed by the existing `WorktreeManager`,
//! `MergeQueue`, and `GitHubWorkflow`.

pub mod control_adapter;
pub mod delivery;
pub mod feedback;
pub mod identity_map;
pub mod runtime_event_adapter;
pub mod view_state;
pub mod workflow_caller;
pub mod workspaces;
