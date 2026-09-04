//! The `roko` binary's library surface.
//!
//! This crate wires Roko's primitives (Store, Compose, Agent, Verify,
//! React) into a one-shot CLI loop. It does **not** implement a plan runner
//! or DAG executor — it drives a single prompt through the universal loop
//! and writes the resulting signals to disk.
//!
//! See [`run_once`] for the core loop and [`Config`] for the `roko.toml`
//! schema.

#![allow(clippy::module_name_repetitions)]
#![allow(missing_docs)]
// Temporary broad allows for the large CLI crate (895 clippy findings).
// These should be tightened incrementally as individual modules are cleaned.
#![allow(
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::collapsible_match,
    clippy::too_many_lines,
    clippy::large_enum_variant,
    clippy::too_long_first_doc_paragraph,
    clippy::use_self,
    clippy::needless_borrow,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_literal_unwrap,
    clippy::unwrap_or_default,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_cast,
    clippy::unnecessary_to_owned,
    clippy::unnecessary_join,
    clippy::unnecessary_filter_map,
    clippy::unnecessary_trailing_comma,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_debug_formatting,
    clippy::needless_continue,
    clippy::needless_lifetimes,
    clippy::needless_collect,
    clippy::redundant_closure,
    clippy::obfuscated_if_else,
    clippy::field_reassign_with_default,
    clippy::derivable_impls,
    clippy::derive_partial_eq_without_eq,
    clippy::ref_option,
    clippy::duration_suboptimal_units,
    clippy::unreadable_literal,
    clippy::format_collect,
    clippy::manual_contains,
    clippy::manual_inspect,
    clippy::manual_range_patterns,
    clippy::manual_split_once,
    clippy::manual_strip,
    clippy::manual_pattern_char_comparison,
    clippy::manual_div_ceil,
    clippy::manual_clamp,
    clippy::manual_checked_ops,
    clippy::many_single_char_names,
    clippy::match_like_matches_macro,
    clippy::match_bool,
    clippy::map_entry,
    clippy::if_same_then_else,
    clippy::bind_instead_of_map,
    clippy::iter_cloned_collect,
    clippy::single_char_add_str,
    clippy::single_char_pattern,
    clippy::float_cmp,
    clippy::should_implement_trait,
    clippy::wrong_self_convention,
    clippy::vec_init_then_push,
    clippy::while_let_loop,
    clippy::comparison_chain,
    clippy::equatable_if_let,
    clippy::no_effect_underscore_binding,
    clippy::double_must_use,
    clippy::double_ended_iterator_last,
    clippy::let_underscore_future,
    clippy::missing_fields_in_debug,
    clippy::suspicious_operation_groupings,
    clippy::suspicious_open_options,
    clippy::doc_link_with_quotes,
    clippy::doc_overindented_list_items,
    clippy::collection_is_never_read,
    clippy::io_other_error,
    clippy::literal_string_with_formatting_args,
    clippy::unchecked_time_subtraction,
    clippy::approx_constant,
    clippy::erasing_op,
    clippy::stable_sort_primitive,
    clippy::implicit_hasher,
    clippy::imprecise_flops,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::manual_is_multiple_of,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_return,
    clippy::redundant_else,
    clippy::unnecessary_lazy_evaluations,
    clippy::useless_format,
    clippy::unwrap_used,
    dropping_references,
    private_interfaces,
    unreachable_patterns,
    unused_assignments,
    unused_imports,
    unused_mut,
    unused_variables,
    dead_code
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
pub mod auth;
pub mod auth_detect;
pub mod bench;
pub mod bench_demo;
pub mod bootstrap;
pub mod chain_handler;
pub mod chain_registry;
pub mod chat;
pub mod chat_history;
pub mod chat_inline;
pub mod chat_session;
pub mod clean;
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
pub mod model_selection;
pub mod note_cluster;
pub mod oneshot;
// The legacy 21K-line orchestrate.rs engine was deleted in E12-T07.
// The v2 event_loop.rs in runner/ is the sole execution engine.
pub mod cli_output;
pub mod orchestrator;
pub mod output_format;
pub mod pipe;
pub mod plan;
pub mod plan_generate;
pub mod plan_generator;
pub mod plan_policy;
pub mod prd;
pub mod prd_prompt;
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
pub mod scaffold;
pub mod scope_resolver;
pub mod secrets;
pub mod share;
pub mod snapshot_migrate;
pub mod snapshot_reconcile;
pub mod spinner;
pub mod status;
pub mod subscriptions;
pub mod surface_inventory;
pub mod task_helpers;
pub mod task_parser;
pub mod transcript;
pub mod tui;
pub mod unified;
pub mod vision_loop;
pub mod worker;
pub mod workspace_lock;
pub mod workspace_paths;

pub mod serve_runtime;

/// Server modules re-exported from the `roko-serve` crate.
pub use roko_serve as serve;

pub use config::{
    AgentConfig, Config, ConfigLayer, ConfigPaths, ConfigSources, DreamsConfig, GateConfig,
    PromptConfig, PromptFile, RepoEntry, RepoRegistry, ResolvedConfig, ServeAuthLayer, ServeLayer,
    Source, ToolsConfig, load_resolved_config,
};

pub use config_cmd::{EditTarget, WizardInputs, run_init_wizard};
pub use daemon::{DaemonConfig, DaemonMode, DaemonState, DaemonStatus};
pub use deployment::SigstoreVerifier;
pub use episode::EpisodePolicy;
pub use inject::{InjectKind, InjectRequest};
pub use layer_check::LayerViolation;
pub use oneshot::{OneshotMode, OneshotResult};
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
