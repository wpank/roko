//! Runner — plan execution infrastructure.
//!
//! The legacy Runner-v2 event loop (`event_loop.rs` and its 18 helper modules)
//! has been deleted. The Graph engine is now the sole execution engine;
//! execute plans with [`crate::graph_execution::run_graph_plan`].
//!
//! This module retains the shared infrastructure that the Graph engine and
//! other subsystems depend on: plan loading, gate dispatch, TUI bridging,
//! state management, types, and various adapters.
//!
//! # Config resolution
//!
//! Callers should build [`RunConfig`] from the effective [`RokoConfig`] via
//! [`RunConfig::from_roko_config`] so that timeouts, gates, models, and budget
//! limits all derive from the project config.
//!
//! [`RokoConfig`]: roko_core::config::schema::RokoConfig

// ── Public modules ──────────────────────────────────────────────────────
pub mod agent_stream;
pub mod conductor_adapter;
pub mod extension_loader;
pub mod extension_registry;
pub mod gate_dispatch;
pub mod graph_tui_bridge;
pub mod impact_analysis;
pub mod merge;
pub mod output_sink;
pub mod persist;
pub mod plan_dag;
pub mod plan_loader;
pub mod preflight;
pub mod projection;
pub mod queue_manifest;
pub mod resume;
pub mod sse_stream;
pub mod state;
pub mod status_file;
pub mod structured_log;
pub mod task_dag;
pub mod tui_bridge;
pub mod types;

// ── Crate-internal modules ──────────────────────────────────────────────
// Gate infrastructure — used by gate_dispatch (shared with Graph engine).
pub(crate) mod cargo_command;
pub(crate) mod gate_adapter;
pub(crate) mod gate_input;
pub(crate) mod gate_oracles;
pub(crate) mod gate_report;
pub(crate) mod inline_output;
pub(crate) mod promise_tracker;

// Re-export primary entry points.
pub use plan_loader::{Plan, load_plan, load_plan_lenient, load_plans, scaffold_missing_crates};
pub use sse_stream::SseStreamClient;
pub use types::{PlanReport, RunConfig, RunReport};
