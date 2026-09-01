//! Reusable widgets for the dashboard TUI.

// Shared palette — must come before widget modules that depend on it.
pub mod rosedust;

pub mod braille;
pub mod cost_by_model;
pub mod diff_panel;
pub mod error_digest;
pub mod gate_output;
pub mod header_bar;
pub mod parallel_pool;
pub mod phase_compact;
pub mod plan_tree;
pub mod status_bar;
pub mod sys_metrics;
pub mod task_progress;
pub mod token_sparkline;
pub mod wave_progress;
