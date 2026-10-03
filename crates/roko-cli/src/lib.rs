//! The `roko` binary's library surface.
//!
//! This crate wires Roko's primitives (Store, Compose, Agent, Verify,
//! React) into a one-shot CLI loop. It does **not** implement a plan runner
//! or DAG executor — it drives a single prompt through the universal loop
//! and writes the resulting signals to disk.
//!
//! See [`run_once`] for the core loop and [`Config`] for the `roko.toml`
//! schema.

// Per-item #[allow] is preferred over blanket suppression.
// These lints are suppressed crate-wide because they are pervasive and stylistic
// across ~200 modules and ~80K lines of CLI/runner/TUI code.
#![allow(
    // dead_code, TEMPORARILY DISABLED FOR AUDIT
    missing_docs,
    // --- clippy pedantic / nursery / style lints that are pervasive ---
    clippy::approx_constant,
    clippy::bind_instead_of_map,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::collection_is_never_read,
    clippy::comparison_chain,
    clippy::derive_partial_eq_without_eq,
    clippy::derivable_impls,
    clippy::doc_link_with_quotes,
    clippy::doc_overindented_list_items,
    clippy::double_ended_iterator_last,
    clippy::double_must_use,
    clippy::duration_suboptimal_units,
    clippy::equatable_if_let,
    clippy::erasing_op,
    clippy::field_reassign_with_default,
    clippy::float_cmp,
    clippy::format_collect,
    clippy::if_same_then_else,
    clippy::implicit_hasher,
    clippy::imprecise_flops,
    clippy::io_other_error,
    clippy::iter_cloned_collect,
    clippy::let_underscore_future,
    clippy::literal_string_with_formatting_args,
    clippy::manual_checked_ops,
    clippy::manual_clamp,
    clippy::manual_contains,
    clippy::manual_div_ceil,
    clippy::manual_inspect,
    clippy::manual_is_multiple_of,
    clippy::manual_pattern_char_comparison,
    clippy::manual_range_patterns,
    clippy::manual_split_once,
    clippy::manual_strip,
    clippy::many_single_char_names,
    clippy::map_entry,
    clippy::match_bool,
    clippy::match_like_matches_macro,
    clippy::missing_fields_in_debug,
    clippy::module_name_repetitions,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_continue,
    clippy::needless_lifetimes,
    clippy::needless_return,
    clippy::no_effect_underscore_binding,
    clippy::obfuscated_if_else,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::ref_option,
    clippy::should_implement_trait,
    clippy::single_char_add_str,
    clippy::single_char_pattern,
    clippy::stable_sort_primitive,
    clippy::struct_field_names,
    clippy::suspicious_open_options,
    clippy::suspicious_operation_groupings,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines,
    clippy::unchecked_time_subtraction,
    clippy::unnecessary_cast,
    clippy::unnecessary_debug_formatting,
    clippy::unnecessary_filter_map,
    clippy::unnecessary_join,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_literal_unwrap,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_to_owned,
    clippy::unnecessary_trailing_comma,
    clippy::unnecessary_unwrap,
    clippy::unreadable_literal,
    clippy::unwrap_or_default,
    clippy::unwrap_used,
    clippy::use_self,
    clippy::useless_format,
    clippy::vec_init_then_push,
    clippy::while_let_loop,
    clippy::wrong_self_convention,
)]

extern crate self as roko_cli;

/// Canonical default port for the shipping `roko-serve` control plane.
///
/// Re-exported from [`roko_core::defaults::DEFAULT_SERVE_PORT`].
pub const DEFAULT_SERVE_PORT: u16 = roko_core::defaults::DEFAULT_SERVE_PORT;
/// Canonical default base URL for CLI and TUI calls into `roko-serve`.
pub const DEFAULT_SERVE_URL: &str = "http://localhost:6677";

// StateHub now lives in roko-runtime (moved from the path-include hack in
// roko-serve by Task 104). This re-export keeps `crate::state_hub::*`
// working for CLI modules that haven't migrated their imports yet.
pub mod state_hub {
    pub use roko_runtime::state_hub::*;
}

pub mod agent_config;
pub mod agent_episode;
pub mod agent_exec;
pub mod agent_spawn;
pub mod audit;
pub mod auth;
pub mod auth_detect;
pub(crate) mod background_writes;
pub mod bench;
pub mod bench_demo;
pub mod bootstrap;
#[cfg(feature = "chain")]
pub mod chain_handler;
#[cfg(feature = "chain")]
pub mod chain_registry;
pub mod chat;
pub mod chat_history;
pub mod chat_inline;
pub mod chat_session;
pub mod clean;
/// Subcommand implementations that live in the library; the `roko` binary's
/// own `commands` module re-exports them.
pub mod commands {
    pub mod diagnose;
    pub mod learn_loops;
}
pub mod config;
pub mod config_cmd;
pub mod config_helpers;
pub mod context_loader;
pub mod credentials;
pub mod custody;
pub mod daemon;
pub mod demo_cmd;
pub mod demo_seed;
pub mod deployment;
pub mod dispatch;
pub mod dispatch_v2;
pub mod doctor;
pub mod dry_run;
pub mod effects_apply;
pub mod episode;
pub mod event_sources;
pub mod execution_control;
pub mod exit_codes;
pub mod explain;
pub mod github_ops;
pub mod github_ops_impl;
pub mod graph_checkpoint;
#[path = "commands/graph.rs"]
pub(crate) mod graph_command;
pub mod graph_entry_cells;
pub mod graph_execution;
pub mod graph_task_dispatch;
pub mod hints;
pub mod index;
pub mod inference_observer;
#[path = "commands/init.rs"]
pub mod init;
pub mod inject;
pub mod inline;
pub(crate) mod knowledge_helpers;
#[path = "../../../scripts/layer_check.rs"]
pub mod layer_check;
pub mod learning_helpers;
pub mod loop_canary;
pub mod model_selection;
pub mod note_cluster;
// oneshot.rs was removed in #363 (zero callers after develop deprecation).
// The legacy 21K-line orchestrate.rs engine was deleted in E12-T07.
// The Runner-v2 event_loop.rs was deleted; the Graph engine is the sole execution engine.
pub mod cli_output;
pub mod cli_reporter;
pub mod orchestrator;
pub mod output_format;
pub mod pipe;
pub mod plan;
pub mod plan_authoring;
pub mod plan_brief;
pub mod plan_generate;
pub mod plan_generator;
pub mod plan_policy;
pub mod plan_validate;
pub mod projection;
pub mod prompting;
pub mod repl;
pub mod replay;
pub mod repo_context;
pub mod research;
pub mod resolved_overrides;
pub mod run;
pub mod run_inline;
pub mod runner;
pub mod runtime_feedback;
pub mod safety_provenance;
pub mod scaffold;
pub mod scope_resolver;
pub mod secrets;
pub mod share;
pub mod snapshot_migrate;
pub mod snapshot_reconcile;
pub mod spec_gate;
pub mod spec_red_on_base;
pub mod spinner;
pub mod status;
pub mod subscriptions;
pub mod surface_inventory;
pub mod task_accept;
pub mod task_helpers;
pub mod task_parser;
pub mod transcript;
pub mod tui;
pub mod unified;
pub mod vision_loop;
pub mod worker;
pub mod workspace_lock;
pub mod workspace_paths;

pub mod serve_client;
pub mod serve_runtime;
pub mod state_hub_ipc;

/// Server modules re-exported from the `roko-serve` crate.
pub use roko_serve as serve;

pub use config::{
    AgentConfig, Config, ConfigPaths, ConfigSources, DreamsConfig, ExecAgentConfig, GateConfig,
    PromptConfig, RepoEntry, RepoRegistry, ResolvedConfig, Source, load_resolved_config,
};

pub use config_cmd::{EditTarget, WizardInputs, run_init_wizard};
pub use daemon::{DaemonConfig, DaemonMode, DaemonState, DaemonStatus};
pub use deployment::SigstoreVerifier;
pub use episode::EpisodePolicy;
pub use inject::{InjectKind, InjectRequest};
pub use layer_check::LayerViolation;
// oneshot re-exports removed in #363 (module deleted).
// orchestrate re-exports removed in E12-T07 (module deleted).
pub use pipe::{PipeInput, PipeMode, stdin_is_tty};
pub use plan::{Plan, PlanSummary, PlanTask};
pub use repl::{ReplCommand, ReplMode, WorkspaceContext};
pub use run::{RunReport, RunUsage, run_once};
pub use secrets::SecretsCmd;
pub use status::{SessionStatus, StatusDiagnostic, collect_session_status};
pub use tui::{
    DashboardData, DashboardScaffold, DashboardSummary, PageId, PageScaffold, Theme, WidgetScaffold,
};
