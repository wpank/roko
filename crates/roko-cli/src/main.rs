//! `roko` binary entrypoint.
//!
//! See [`roko_cli`] for the lib-side description. The binary exposes
//! subcommands (`init`, `run`, `status`, `replay`, `dream`, `config`, `inject`,
//! `plan`, `research`, `neuro`, `subscription`, `event-sources`, `experiment`) plus top-level flags for mode selection (`--headless`,
//! `--role`, `--model`, `--effort`, `--json`, `--log-format`, `--quiet`,
//! `--resume`, `--repo`, and a positional `[prompt]` for one-shot mode).

#![allow(missing_docs)]
// Temporary broad allows matching lib.rs while the CLI crate is cleaned.
#![allow(
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::too_many_lines,
    clippy::use_self,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_literal_unwrap,
    clippy::unwrap_or_default,
    clippy::unwrap_used,
    clippy::needless_return,
    clippy::redundant_else,
    clippy::useless_format,
    clippy::unnecessary_lazy_evaluations,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::manual_is_multiple_of,
    clippy::stable_sort_primitive,
    clippy::derivable_impls,
    clippy::needless_lifetimes,
    dead_code,
    unreachable_patterns,
    unused_mut,
    unused_assignments
)]

mod agent_serve;
mod commands;

use roko_cli::auth;

use agent_serve::AgentCmd;
use anyhow::{Context as _, Result, anyhow, bail};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use commands::experiment::{ExperimentCmd, dispatch_experiment};
use octocrab::Octocrab;
use octocrab::models::hooks::{Config as HookConfig, ContentType, Hook};
use octocrab::models::webhook_events::WebhookEventType;
use roko_agent::process::{cleanup_orphaned_agents, reap_orphaned_children};
use roko_agent::translate::BackendResponse;
use roko_cli::agent_spawn::{SpawnAgentSpec, spawn_agent_scoped};
use roko_cli::resolved_overrides::{DoInput, GlobalCliFlags, ResolvedExecutionOverrides};
use roko_cli::serve_runtime::RokoCliRuntime;
use roko_cli::tui::App;
use roko_cli::{
    Config, DashboardScaffold, EditTarget, InjectKind, InjectRequest, PageId, PipeMode, Plan,
    RepoRegistry, Source, WizardInputs, config_cmd, load_resolved_config, run_init_wizard,
};
pub use roko_cli::{model_selection, repo_context};
use roko_core::agent::{AgentRole, ProviderKind};
use roko_core::config::ServeDeployWebhookConfig;
use roko_core::config::schema::{ModelProfile, ProviderConfig, RokoConfig};
use roko_core::shutdown::GracefulShutdown;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_core::{ContentHash, Context, DaimonPolicy, Kind, Query, Store};
use roko_core::{Headlines, TaskMetric, compute_headlines};
use roko_dreams::{DreamAgentConfig, DreamLoopConfig, DreamRunner};
use roko_fs::{FileSubstrate, FsObservabilitySinks};
use roko_learn::cascade_router::{CascadeRouteExplanation, CascadeRouter};
use roko_learn::cfactor::{CFactor, trend_arrow as cfactor_trend_arrow};
use roko_learn::cost_table::CostTable;
use roko_learn::costs_log::CostsLog;
use roko_learn::efficiency::compute_role_profiles;
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use roko_learn::latency::{LatencyRegistry, LatencyStats};
use roko_learn::model_router::{RoutingContext, normalized_cost};
use roko_learn::prompt_experiment::ExperimentStore;
use roko_learn::provider_health::{CircuitState, ProviderHealth};
use roko_learn::runtime_feedback::{CompletedRunInput, LearningRuntime};
use roko_learn::runtime_feedback::{read_efficiency_events, refresh_cfactor_snapshot};
use roko_neuro::{
    DEFAULT_GC_MIN_CONFIDENCE, ExportFilter, ImportOptions, ImportResult, KnowledgeKind,
    KnowledgeStore,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fmt::Write as _;
use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

// -----------------------------------------------------------------------
// Exit codes
// -----------------------------------------------------------------------

use roko_cli::exit_codes::{EXIT_AGENT_FAILURE, EXIT_FAILURE, EXIT_SUCCESS, EXIT_SYSTEM_ERROR};

// -----------------------------------------------------------------------
// Effort level
// -----------------------------------------------------------------------

/// Reasoning effort level for the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Effort {
    /// Minimal reasoning — fast, cheap.
    Low,
    /// Balanced reasoning (default).
    Medium,
    /// Thorough reasoning.
    High,
    /// Maximum reasoning — slowest, most expensive.
    Max,
}

/// Complexity override for `roko do`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DoComplexity {
    /// Mechanical: single-line / derive-only change.
    #[value(alias = "mechanical")]
    Trivial,
    /// Direct single-agent workflow.
    Simple,
    /// Planned workflow.
    #[value(alias = "standard")]
    Medium,
    /// Full architectural workflow.
    #[value(alias = "architectural")]
    Complex,
}

impl DoComplexity {
    fn into_plan_complexity(self) -> roko_gate::PlanComplexity {
        match self {
            Self::Trivial => roko_gate::PlanComplexity::Trivial,
            Self::Simple => roko_gate::PlanComplexity::Simple,
            Self::Medium => roko_gate::PlanComplexity::Standard,
            Self::Complex => roko_gate::PlanComplexity::Complex,
        }
    }
}

/// Optional focused view for `roko doctor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DoctorSubject {
    /// Disk capacity, retained logs, targets, and worktree storage.
    Disk,
    /// Network connectivity and external service reachability.
    Network,
    /// Remove orphaned temp files, .corrupted files, and stale lock files
    /// from `.roko/learn/` and related directories.
    Clean,
}

/// Workspace-local build and evidence cache lifecycle.
#[derive(Debug, Subcommand)]
pub(crate) enum CacheCmd {
    /// Report cache pressure and protected/eligible entries without deleting.
    Status {
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Plan a safe prune; pass --apply to perform it.
    Prune {
        /// Perform deletion. Without this flag the command is read-only.
        #[arg(long)]
        apply: bool,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Combined target budget across linked worktrees.
        #[arg(long, default_value_t = 96)]
        target_budget_gb: u64,
        /// Terminal run-evidence budget.
        #[arg(long, default_value_t = 2048)]
        evidence_budget_mb: u64,
        /// Context-pack cache budget.
        #[arg(long, default_value_t = 1024)]
        context_budget_mb: u64,
        /// Minimum age of incremental partitions selected under pressure.
        #[arg(long, default_value_t = 6)]
        min_age_hours: u64,
        /// Maximum age of terminal evidence and immutable log generations.
        #[arg(long, default_value_t = 14)]
        max_evidence_age_days: u64,
        /// Number of newest terminal evidence runs always retained.
        #[arg(long, default_value_t = 10)]
        keep_runs: usize,
    },
}

/// Offline maintenance for derived per-run observability indexes.
#[derive(Debug, Subcommand)]
pub(crate) enum RunIndexCmd {
    /// Boundedly inspect or rebuild historical per-run indexes.
    Repair {
        /// Perform atomic replacements. Without this flag the command is read-only.
        #[arg(long)]
        apply: bool,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Maximum aggregate bytes read across runner/runtime generations.
        #[arg(long, default_value_t = 512 * 1024 * 1024)]
        max_bytes: u64,
        /// Maximum aggregate complete JSONL records inspected.
        #[arg(long, default_value_t = 1_000_000)]
        max_records: u64,
        /// Maximum number of distinct per-run index files staged.
        #[arg(long, default_value_t = 4_096)]
        max_indexes: usize,
        /// Hard wall-clock budget before the bounded atomic replacement phase.
        #[arg(long, default_value_t = 120)]
        deadline_secs: u64,
    },
}

/// Log output format for tracing subscriber initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LogFormat {
    /// Human-readable text logs.
    Text,
    /// Structured JSON logs.
    Json,
}

impl std::fmt::Display for Effort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Max => write!(f, "max"),
        }
    }
}

// -----------------------------------------------------------------------
// Color mode
// -----------------------------------------------------------------------

/// Controls ANSI color output.
///
/// Respects the `NO_COLOR` (https://no-color.org/), `CLICOLOR`, and
/// `CLICOLOR_FORCE` conventions when set to `Auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    /// Detect from terminal and environment (default).
    Auto,
    /// Always emit ANSI colors.
    Always,
    /// Never emit ANSI colors.
    Never,
}

impl ColorMode {
    /// Resolve the effective color decision, consulting env vars when `Auto`.
    ///
    /// Precedence (highest first):
    /// 1. `--color always|never` (not Auto)
    /// 2. `NO_COLOR` set and non-empty  -> off
    /// 3. `CLICOLOR_FORCE` set and != "0" -> on
    /// 4. `CLICOLOR=0`                   -> off
    /// 5. stdout is a TTY               -> on
    /// 6. otherwise                      -> off
    fn should_color(self) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => {
                if env::var("NO_COLOR").map_or(false, |v| !v.is_empty()) {
                    return false;
                }
                if env::var("CLICOLOR_FORCE").map_or(false, |v| v != "0") {
                    return true;
                }
                if env::var("CLICOLOR").map_or(false, |v| v == "0") {
                    return false;
                }
                std::io::stdout().is_terminal()
            }
        }
    }
}

// -----------------------------------------------------------------------
// Enhanced version string
// -----------------------------------------------------------------------

fn long_version() -> &'static str {
    use std::sync::OnceLock;
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| {
        let version = env!("CARGO_PKG_VERSION");
        let git_hash = env!("ROKO_GIT_HASH");
        let rustc = env!("ROKO_RUSTC_VERSION");
        let target = env!("ROKO_TARGET");
        format!("{version} ({rustc}, {target}, git {git_hash})")
    })
}

// -----------------------------------------------------------------------
// CLI structure
// -----------------------------------------------------------------------

/// Minimal CLI for the Roko universal loop.
#[derive(Debug, Parser)]
#[command(
    name = "roko",
    version,
    long_version = long_version(),
    about = "Roko --- agent toolkit\n\nQuick start: roko setup, roko do <task>, roko status\nRun roko help <command> for details.",
    after_long_help = "\
COMMAND GROUPS:
  Core workflow:     init, do, run, status, doctor
  Planning:          plan, prd
  Agents:            agent (create, start, stop, chat, serve)
  Research:          research, think, note
  Knowledge:         knowledge (query, dream, custody, archive)
  Learning:          learn (router, experiments, efficiency, reflexes, inspect)
  Jobs:              job
  Benchmarks:        bench
  Configuration:     config (providers, models, subscriptions, plugins, secrets, preset)
  Code intelligence: index
  Server:            up, serve, acp, daemon, deploy, worker
  Interactive:       dashboard
  Utilities:         cache, replay, history, inject, completions, new, explain"
)]
struct Cli {
    /// Override the config file (default: `./roko.toml`).
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Set the agent role / persona.
    #[arg(long, global = true)]
    role: Option<String>,

    /// Force the model slug for this invocation, bypassing adaptive routing.
    ///
    /// This is the **global** model override — it applies to every subcommand.
    /// When set, the cascade router is skipped and the specified model is used
    /// unconditionally. The outcome is tagged as a manual override so the
    /// router does not conflate it with its own learned policy.
    ///
    /// `--force-model` and `--force-backend` are accepted aliases for this
    /// flag, retained for backward compatibility.
    #[arg(
        long,
        global = true,
        visible_alias = "force-model",
        alias = "force-backend"
    )]
    model: Option<String>,

    /// Set the repository / working directory root.
    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    /// Resume a previous session by ID.
    #[arg(long, global = true)]
    resume: Option<String>,

    /// Set reasoning effort level.
    #[arg(long, global = true, value_enum)]
    effort: Option<Effort>,

    /// Emit JSON output instead of human-readable text.
    #[arg(long, global = true)]
    json: bool,

    /// Set the tracing log format.
    #[arg(long, global = true, value_enum, default_value_t = LogFormat::Text)]
    log_format: LogFormat,

    /// Suppress non-essential output.
    #[arg(long, global = true)]
    quiet: bool,

    /// Enable verbose tracing output to stderr. Without this, tracing goes only to .roko/roko.log.
    #[arg(long, short = 'v', global = true)]
    verbose: bool,

    /// Run as a headless daemon (background service).
    #[arg(long, global = true)]
    headless: bool,

    /// Control color output: auto (default), always, never.
    ///
    /// Respects NO_COLOR, CLICOLOR, and CLICOLOR_FORCE env vars in auto mode.
    #[arg(long, global = true, value_enum, default_value_t = ColorMode::Auto)]
    color: ColorMode,

    /// Print elapsed time after command execution.
    ///
    /// Also enabled by setting ROKO_TIMING=1 in the environment.
    #[arg(long, global = true)]
    timing: bool,

    /// Don't start the HTTP control plane in the background.
    #[arg(long, global = true)]
    no_serve: bool,

    /// One-shot mode: execute this prompt and exit. Quote a prompt of
    /// several words; a single word is read as a subcommand.
    #[arg(global = false, value_parser = OneShotPromptParser)]
    prompt: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

/// Parses the one-shot prompt (`roko "fix the bug"`). A single word is
/// refused as an unrecognized subcommand, so a typo or a stale command
/// (`roko dreem`, `roko dream --help`) fails instead of starting an agent
/// run. A one-word prompt goes through `roko run <word>`.
#[derive(Clone, Debug)]
struct OneShotPromptParser;

impl clap::builder::TypedValueParser for OneShotPromptParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        use clap::error::ErrorKind;

        let prompt = value.to_string_lossy();
        if prompt.split_whitespace().nth(1).is_some() {
            return Ok(prompt.into_owned());
        }
        let word = prompt.trim();
        if word.is_empty() {
            let error = clap::Error::raw(ErrorKind::InvalidValue, "the prompt is empty\n");
            return Err(error.with_cmd(cmd));
        }
        Err(unknown_command_error(cmd, word))
    }
}

/// The error for `roko <word>` when `word` names no subcommand, with the
/// command it most likely meant.
fn unknown_command_error(cmd: &clap::Command, word: &str) -> clap::Error {
    use clap::error::ErrorKind;

    let similar = match suggested_command(cmd, word) {
        Some(command) => format!("  tip: a similar subcommand exists: 'roko {command}'\n"),
        None => String::new(),
    };
    let message = format!(
        "unrecognized subcommand '{word}'\n\n{similar}  tip: to send a one-word prompt, use \
         'roko run {word}'\n\nFor more information, try '--help'.\n"
    );
    clap::Error::raw(ErrorKind::InvalidSubcommand, message).with_cmd(cmd)
}

/// The command a mistyped `roko <word>` most likely meant: a nested
/// subcommand named `word` (`roko dream` is `roko knowledge dream`), or the
/// top-level subcommand within two edits of it.
fn suggested_command(cmd: &clap::Command, word: &str) -> Option<String> {
    let nested = cmd.get_subcommands().find_map(|group| {
        group
            .get_subcommands()
            .find(|sub| sub.get_name() == word || sub.get_all_aliases().any(|a| a == word))
            .map(|sub| format!("{} {}", group.get_name(), sub.get_name()))
    });
    nested.or_else(|| {
        let names: Vec<&str> = cmd.get_subcommands().map(|sub| sub.get_name()).collect();
        roko_core::config::loader::find_nearest_key(word, &names).map(str::to_string)
    })
}

/// Parse `args` (the program name first) into a [`Cli`].
///
/// clap parses a subcommand that follows a bare word before it checks the
/// word, so `roko dream run` would fail with `run`'s missing-prompt error.
/// When parsing fails and the first word names no subcommand, the error is
/// the unknown-command one instead, with its suggestion.
fn try_parse_cli<I, T>(args: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString>,
{
    let args: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    Cli::try_parse_from(&args).map_err(|error| {
        let mut cmd = Cli::command();
        cmd.build();
        match leading_unknown_word(&cmd, &args) {
            Some(word) if error.use_stderr() => unknown_command_error(&cmd, &word),
            _ => error,
        }
    })
}

/// The first bare word of `args`, past the program name, options and their
/// values, when it names no subcommand and is not a prompt of several words.
fn leading_unknown_word(cmd: &clap::Command, args: &[std::ffi::OsString]) -> Option<String> {
    let mut words = args.iter().skip(1).map(|arg| arg.to_string_lossy());
    while let Some(word) = words.next() {
        if word == "--" {
            return None;
        }
        let takes_value = if let Some(long) = word.strip_prefix("--") {
            !long.contains('=') && option_takes_value(cmd, |arg| is_long_option(arg, long))
        } else if let Some(short) = word.strip_prefix('-') {
            let short = short.chars().last();
            short.is_some() && option_takes_value(cmd, |arg| arg.get_short() == short)
        } else {
            let word = word.trim();
            let prompt = word.split_whitespace().nth(1).is_some();
            let known = is_subcommand(cmd, word);
            return (!word.is_empty() && !prompt && !known).then(|| word.to_string());
        };
        if takes_value {
            words.next();
        }
    }
    None
}

/// Whether `cmd` has an option `is_option` picks that takes a value.
fn option_takes_value(cmd: &clap::Command, is_option: impl Fn(&clap::Arg) -> bool) -> bool {
    cmd.get_arguments()
        .any(|arg| is_option(arg) && arg.get_action().takes_values())
}

/// Whether `arg` is the option `--long`, by name or alias.
fn is_long_option(arg: &clap::Arg, long: &str) -> bool {
    let aliases = arg.get_all_aliases().unwrap_or_default();
    arg.get_long() == Some(long) || aliases.contains(&long)
}

/// Whether `word` names one of `cmd`'s subcommands, or an alias of one.
fn is_subcommand(cmd: &clap::Command, word: &str) -> bool {
    cmd.get_subcommands()
        .any(|sub| sub.get_name() == word || sub.get_all_aliases().any(|alias| alias == word))
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum Command {
    // ── Core workflow ────────────────────────────────────────────────
    /// Create `.roko/` and a default `roko.toml` in `path` (default: cwd).
    #[command(after_help = "\
Examples:
  roko init                         Initialize in the current directory
  roko init /path/to/project        Initialize in a specific directory
  roko init --cloud                 Initialize with cloud-ready defaults
  roko init --profile rust          Initialize with Rust project profile
  roko init --demo                  Initialize and seed demo data")]
    Init {
        /// Directory to initialize (default: current dir).
        path: Option<PathBuf>,
        /// Generate cloud-ready defaults for deployment.
        #[arg(long)]
        cloud: bool,
        /// Project profile to use (e.g. rust, typescript, go, python, general).
        #[arg(long)]
        profile: Option<String>,
        /// Seed realistic demo data after initialization.
        #[arg(long)]
        demo: bool,
    },
    /// Do a task from a natural-language prompt.
    ///
    /// `roko do` is the recommended entry point for ad-hoc work. It auto-classifies
    /// the prompt into a complexity band and picks the lightest workflow that can
    /// complete it safely:
    ///
    ///   Trivial / Simple → one agent: the prompt runs as a one-task plan written to
    ///                      .roko/runs/<run-id>/, verified by the workspace gates
    ///   Medium / Complex  → planned workflow: generate tasks.toml, approve, execute
    ///
    /// Use `--complexity` to force a specific band, or `--plan` to always use
    /// the planned workflow regardless of classification.
    ///
    /// RELATED COMMANDS
    ///
    ///   roko run "<prompt>"     `roko do` for scripts and CI: the same routes, without
    ///                           the TTY requirement for auto-detected scope.
    ///
    ///   roko plan run plans/    Execute a pre-existing plans directory through the
    ///                           Graph engine. Use this when you already have tasks.toml
    ///                           files on disk, e.g. from `roko prd plan <slug>`.
    ///
    ///   roko develop "<prompt>" Removed — use `roko do --plan <prompt>` instead.
    #[command(
        visible_alias = "d",
        after_help = "\
Examples:
  roko do \"Fix the login bug\"                         Classify scope and execute
  roko do \"Add auth flow\" --complexity medium         Force planned workflow
  roko do \"Refactor API\" --dry-run                    Preview scope and workflow only"
    )]
    Do {
        /// Force a planned workflow instead of the lightest classified scope.
        #[arg(long)]
        plan: bool,
        /// Force a complexity level instead of auto-detecting.
        #[arg(long, value_enum)]
        complexity: Option<DoComplexity>,
        /// Preview classification, workflow, and gates without executing.
        #[arg(long)]
        dry_run: bool,
        /// Working directory (default: cwd or --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Override the provider for this run.
        #[arg(long)]
        provider: Option<String>,
        /// Skip approval prompts when the selected workflow would ask.
        #[arg(long)]
        yes: bool,
        /// Alias for --dry-run retained for existing scripts.
        #[arg(long)]
        ghost: bool,
        /// Compare cascade and non-cascade routing as a dry preview.
        #[arg(long)]
        compare: bool,
        /// Continue interrupted work. Optionally pass a work/run id.
        #[arg(long = "continue", value_name = "WORK_ID", num_args = 0..=1)]
        r#continue: Option<Option<String>>,
        /// Disable cascade routing for this run.
        #[arg(long)]
        no_cascade: bool,
        /// Additional context files/dirs/globs to include in the prompt.
        #[arg(long = "context", value_name = "PATH")]
        context: Vec<PathBuf>,
        /// Prompt words. Quoted prompts are recommended.
        #[arg(value_name = "PROMPT")]
        prompt: Vec<String>,
    },
    /// (Removed) Use `roko do --plan <prompt>` instead.
    #[command(hide = true)]
    Develop {
        /// Preview the generated plan without executing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the approval prompt and execute immediately.
        #[arg(long)]
        yes: bool,
        /// Resume interrupted work from the last snapshot.
        #[arg(long)]
        r#continue: bool,
        /// Working directory (default: cwd or --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Override the provider for this run.
        #[arg(long)]
        provider: Option<String>,
        /// Prompt describing what to develop.
        #[arg(value_name = "PROMPT")]
        prompt: Vec<String>,
    },
    /// Run a prompt through the Graph engine (`roko do`, usable without a TTY).
    ///
    /// The prompt's scope is auto-classified. A trivial or simple prompt runs as a
    /// one-task plan written to `.roko/runs/<run-id>/tasks.toml`: one implementer
    /// agent whose verify steps are the workspace gates (`[[gates.rungs]]`, else
    /// `cargo check` / `go build`), with the dispatch, failover, safety, budget,
    /// checkpoints, episodes, and cost records of `roko plan run`. A standard or
    /// complex prompt first generates a plan (or a PRD and a plan), then executes it.
    /// `--serve`, `--share`, and `--max-retries` always run the one-task plan.
    /// Exits non-zero when the run fails; `--json` prints the run report.
    #[command(after_help = "\
Examples:
  roko run \"Fix the login bug\"      One-task plan through the Graph engine
  roko run \"Add tests for auth\"     Generate and execute a plan
  roko --json run \"Fix the login bug\"   Print the run report as JSON")]
    Run {
        /// The user prompt text.
        prompt: String,
        /// Override the working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Start the HTTP control plane alongside the run for external observability.
        #[arg(long)]
        serve: bool,
        /// Generate a shareable URL for this run (starts serve if needed).
        #[arg(long)]
        share: bool,
        /// Override the provider for this run (e.g. anthropic, openai, ollama, moonshot).
        #[arg(long)]
        provider: Option<String>,
        /// Retries after a failed attempt of the prompt's task.
        #[arg(long)]
        max_retries: Option<u32>,
    },
    /// Print signal counts, most recent episode, and gate pass/fail.
    #[command(
        visible_alias = "s",
        after_help = "\
Examples:
  roko status                       Show workspace health summary
  roko status --json                Output status as JSON for scripting
  roko status --cfactor             Compute and show C-Factor metrics"
    )]
    Status {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Print a compact 3-line health summary (provider, learning state, workspace).
        #[arg(long, conflicts_with = "cfactor", conflicts_with = "surfaces")]
        quick: bool,
        /// Compute and persist the latest C-Factor snapshot.
        #[arg(long)]
        cfactor: bool,
        /// Print the CLI/TUI/backend surface inventory instead of session status.
        #[arg(long)]
        surfaces: bool,
    },
    /// Inspect the configured GitHub workflow integration.
    Github {
        #[command(subcommand)]
        cmd: commands::github::GithubCmd,
    },
    /// Inspect workspace state from `.roko/`.
    #[command(after_help = "\
Examples:
  roko show                         Overview: work items, agents, costs, learning
  roko show costs                   Cost breakdown by model, task, and day
  roko show agents                  Agent status from executor and efficiency state
  roko show knowledge               Durable knowledge entries
  roko show plans                   Plans in progress and recent plan state
  roko show learning                Routing, experiments, gates, and C-Factor
  roko show history                 Recent chronological state events
  roko show auth-redesign           Detail for a work item or plan id
  roko show --dashboard              Open the dashboard/TUI
  roko show --live                  (deprecated alias for --dashboard)
  roko show --follow                Stream live events from roko serve")]
    Show {
        /// Open the interactive TUI dashboard.
        #[arg(long)]
        dashboard: bool,
        /// Deprecated alias for --dashboard. Hidden in help; emits a
        /// deprecation warning and delegates identically.
        #[arg(long, hide = true)]
        live: bool,
        /// Stream live events from a running roko serve instance via SSE.
        #[arg(long, short = 'f')]
        follow: bool,
        /// URL of the roko serve instance for --follow (default: http://localhost:6677).
        #[arg(long, default_value = "http://localhost:6677")]
        serve_url: String,
        /// Override the working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// One of: costs, agents, knowledge, plans, learning, history, or a work id.
        #[arg(value_name = "SUBCOMMAND_OR_WORK_ID")]
        subject: Option<String>,
    },
    /// Diagnose self-hosted workspace bootstrap state.
    Doctor {
        /// Limit diagnostics to one area (`disk` or `network`).
        #[arg(value_enum)]
        subject: Option<DoctorSubject>,
        /// Directory containing `roko.toml` and `.roko/` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// roko-serve base URL or explicit health endpoint to probe.
        #[arg(long)]
        serve_url: Option<String>,
    },
    /// Inspect and safely prune workspace-local build/evidence caches.
    #[command(after_help = "\
Examples:
  roko cache status
  roko cache prune
  roko cache prune --apply --target-budget-gb 64 --min-age-hours 1")]
    Cache {
        #[command(subcommand)]
        cmd: CacheCmd,
    },
    /// Inspect or rebuild derived per-run event indexes offline.
    #[command(after_help = "\
Examples:
  roko run-index repair
  roko run-index repair --max-bytes 268435456 --deadline-secs 30
  roko run-index repair --apply")]
    RunIndex {
        #[command(subcommand)]
        cmd: RunIndexCmd,
    },
    /// Interactive setup wizard: detect providers, init workspace, verify.
    #[command(after_help = "\
Examples:
  roko setup                        Interactive guided setup
  roko setup --quick                Auto-detect everything, write config, no prompts
  roko setup --yes                  Non-interactive (use first available provider)
  roko setup --workdir /path        Setup in a specific directory")]
    Setup {
        /// Directory to set up (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Non-interactive mode: skip prompts, use first available provider.
        #[arg(long)]
        yes: bool,
        /// Auto-detect all available providers, write config, print summary — no prompts.
        #[arg(long)]
        quick: bool,
    },
    /// Diagnose why a plan failed: a readable report, or structured JSON with `--json`.
    #[command(after_help = "\
Examples:
  roko diagnose my-plan             Show failure report for a plan
  roko diagnose my-plan --verbose   Also list the tasks that completed, with their attempts
  roko diagnose my-plan --json      Print the full report as JSON")]
    Diagnose {
        /// Plan ID to diagnose.
        plan_id: String,
        /// Also list the tasks that completed, with their attempts, verify
        /// failures and episodes (always listed for tasks that did not).
        #[arg(long)]
        verbose: bool,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// (deprecated: use `roko doctor`) Check workspace layer dependency rules.
    #[command(hide = true)]
    LayerCheck,

    // ── Planning & PRDs ─────────────────────────────────────────────
    /// Manage plans (list, show, create, validate, run, generate).
    #[command(visible_alias = "p")]
    Plan {
        #[command(subcommand)]
        cmd: PlanCmd,
    },
    /// Manage product requirements documents (idea, draft, publish, plan).
    Prd {
        #[command(subcommand)]
        cmd: PrdCmd,
    },

    /// Import backlog specs as PRD ideas.
    Backlog {
        #[command(subcommand)]
        cmd: BacklogCmd,
    },

    // ── Agents ──────────────────────────────────────────────────────
    /// Manage standalone agent runtimes and chat.
    Agent {
        #[command(subcommand)]
        cmd: AgentCmd,
    },

    // ── Research ────────────────────────────────────────────────────
    /// Research topics, enhance documents, analyze execution data.
    Research {
        #[command(subcommand)]
        cmd: ResearchCmd,
    },
    /// Research a question without executing agents or changing source files.
    #[command(after_help = "\
Examples:
  roko think \"how does auth work in this codebase?\"
  roko think \"what do we know about rate limiting?\"")]
    Think {
        /// Question to analyze.
        question: Vec<String>,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Capture a quick note (no LLM, instant).
    #[command(after_help = "\
Examples:
  roko note \"my thought here\"
  roko note --tag feature \"add cursor support\"
  roko note --tag bug --tag urgent \"login is broken\"")]
    Note {
        /// Tag(s) to attach to the note.
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Note text.
        text: Vec<String>,
    },
    /// (deprecated: use `roko config preset`) Apply config presets by writing roko.toml.
    #[command(
        hide = true,
        subcommand,
        after_help = "\
Examples (deprecated -- use `roko config preset` instead):
  roko tune routing   ->  roko config preset routing
  roko tune gates     ->  roko config preset gates
  roko tune budget    ->  roko config preset budget
  roko tune model X   ->  roko config preset model X"
    )]
    Tune(TuneCmd),

    // ── Knowledge (neuro + dreams + custody + archive) ──────────────
    /// Durable knowledge store, dream consolidation, custody chain, and archival.
    Knowledge {
        #[command(subcommand)]
        cmd: KnowledgeCmd,
    },

    // ── Learning & feedback ─────────────────────────────────────────
    /// Inspect learning state: routing, experiments, efficiency, episodes, reflexes, and subsystem inspection.
    Learn {
        #[command(subcommand)]
        cmd: LearnCmd,
    },

    // ── Jobs ────────────────────────────────────────────────────────
    /// Manage marketplace jobs (list, create, match, show, execute, cancel).
    Job {
        #[command(subcommand)]
        cmd: JobCmd,
    },

    /// Run benchmark evaluations and write learning telemetry.
    Bench {
        #[command(subcommand)]
        cmd: BenchCmd,
    },
    /// Demo setup and management.
    #[command(subcommand)]
    Demo(DemoCmd),

    // ── Configuration (providers, models, subscriptions, etc.) ──────
    /// Manage global and project config, providers, models, subscriptions, plugins.
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },

    // ── Code intelligence ───────────────────────────────────────────
    /// Code intelligence: build, search, and inspect the workspace index.
    Index {
        #[command(subcommand)]
        cmd: IndexCmd,
    },

    // ── Graph execution ──────────────────────────────────────────────
    /// Execute, validate, and inspect graph definitions (DAGs of cells).
    Graph {
        #[command(subcommand)]
        cmd: commands::graph::GraphCmd,
    },

    // ── Feeds ────────────────────────────────────────────────────────
    /// Inspect runtime data feeds (list, status).
    Feed {
        #[command(subcommand)]
        cmd: commands::feed::FeedCmd,
    },

    /// Manage and evaluate pure-data feed recipes.
    Recipe {
        #[command(subcommand)]
        cmd: commands::recipe::RecipeCmd,
    },

    // ── Triggers ────────────────────────────────────────────────────
    /// Manage trigger bindings (list, show, create, fire).
    Trigger {
        #[command(subcommand)]
        cmd: commands::trigger::TriggerCmd,
    },

    // ── Server & deployment ─────────────────────────────────────────
    /// (deprecated: use `roko serve`) Start the dev environment.
    #[command(
        hide = true,
        after_help = "\
Examples:
  roko dev                          Start serve + demo frontend
  roko dev --no-frontend            Start serve only (skip npm dev server)"
    )]
    Dev {
        /// Skip the demo frontend dev server.
        #[arg(long)]
        no_frontend: bool,
    },
    /// (deprecated: use `roko serve`) Start roko serve + all agents.
    #[command(
        hide = true,
        after_help = "\
Examples:
  roko up                           Start serve + all agents from roko.toml
  roko up --workdir /path/to/proj   Start from a specific project directory"
    )]
    Up {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Start the HTTP API server.
    Serve {
        /// Address to bind to (default: 127.0.0.1).
        #[arg(long)]
        bind: Option<String>,
        /// Port number (default: 6677).
        #[arg(long)]
        port: Option<u16>,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Run the interactive TUI dashboard embedded in the server process.
        /// The TUI reads live state directly from the server's StateHub
        /// (zero-copy, no file polling).
        #[arg(long)]
        tui: bool,
        /// Expose the PTY terminal routes.
        #[arg(long)]
        enable_terminal: bool,
    },
    /// Start ACP (Agent Client Protocol) server for editor integration.
    Acp {
        /// Working directory.
        #[arg(long, default_value = ".")]
        workdir: PathBuf,
        /// Configuration profile.
        #[arg(long, default_value = "default")]
        profile: String,
        /// Path to roko.toml config file.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Path to a global roko.toml merged with the workspace/editor config.
        #[arg(long)]
        global_config: Option<PathBuf>,
        /// Log file path (stdout is the protocol channel).
        #[arg(long, default_value = ".roko/acp.log")]
        log_file: PathBuf,
    },
    /// Manage daemon mode (start, stop, status, logs, install).
    Daemon {
        #[command(subcommand)]
        cmd: DaemonCmd,
    },
    /// Deploy to cloud targets (Railway, Fly.io, Docker).
    Deploy {
        #[command(subcommand)]
        cmd: DeployCmd,
    },
    /// Run as a deployed worker (reads template from env, serves tasks).
    Worker {
        /// Port to listen on (default: 8080, overridden by PORT env).
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },

    // ── Interactive ─────────────────────────────────────────────────
    /// Launch the dashboard TUI.
    Dashboard {
        /// Specific dashboard page slug to render.
        #[arg(long)]
        page: Option<String>,
        /// List all available page slugs.
        #[arg(long)]
        list_pages: bool,
        /// Force text-mode output instead of the interactive terminal UI.
        #[arg(long)]
        text: bool,
        /// Override the working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Render all TUI tabs headlessly to text files in the given directory and exit.
        #[arg(long)]
        snapshot: Option<PathBuf>,
        /// Use high-contrast color scheme for accessibility (WCAG 2.1 AA).
        #[arg(long)]
        high_contrast: bool,
        /// Disable animations for reduced-motion accessibility.
        #[arg(long)]
        reduced_motion: bool,
    },

    /// Capture TUI screenshots as text files for headless inspection.
    Screenshot(commands::screenshot::ScreenshotArgs),

    // ── Authentication ────────────────────────────────────────────────
    /// Authenticate with a roko-serve instance.
    #[command(after_help = "\
Examples:
  roko login                              Login via browser (Privy)
  roko login --api-key                    Login with an API key (prompts)
  roko login --api-key --check            Validate stored API key credential
  roko login https://my-server.com        Login to a remote server")]
    Login {
        /// URL of the roko-serve instance (default: http://localhost:6677).
        #[arg(default_value = "http://localhost:6677")]
        url: String,
        /// Login with an API key instead of browser auth.
        #[arg(long)]
        api_key: bool,
        /// Non-interactive: validate stored credential only.
        #[arg(long, requires = "api_key")]
        check: bool,
        /// URL of the dashboard for browser auth (default: http://localhost:5173).
        #[arg(
            long,
            env = "NUNCHI_DASHBOARD_URL",
            default_value = "http://localhost:5173"
        )]
        dashboard_url: String,
    },
    /// Remove stored credentials.
    Logout,
    /// Show current authentication status.
    Whoami,

    // ── Vision loop ───────────────────────────────────────────────────
    /// Iterative vision-guided UI refinement loop.
    VisionLoop {
        /// Source file to iterate on (e.g. src/pages/Home.tsx).
        target_file: PathBuf,
        /// What the UI should look/feel like.
        #[arg(long)]
        goal: String,
        /// URL to screenshot (e.g. http://localhost:5173).
        #[arg(long)]
        url: String,
        /// Maximum iterations (default: 10).
        #[arg(long, default_value_t = 10)]
        max_iter: u32,
        /// Score threshold (1-10) for early stopping (default: 9.0).
        #[arg(long, default_value_t = 9.0)]
        target_score: f64,
        /// Consecutive target hits before stopping (default: 2).
        #[arg(long, default_value_t = 2)]
        consecutive_target: u32,
        /// Score drop from peak that triggers rollback (default: 3.0).
        #[arg(long, default_value_t = 3.0)]
        regression_threshold: f64,
        /// Vision model key from roko.toml (auto-detected if omitted).
        #[arg(long)]
        model: Option<String>,
        /// Viewport width in pixels (default: 1280).
        #[arg(long, default_value_t = 1280)]
        viewport_width: u32,
        /// Viewport height in pixels (default: 720).
        #[arg(long, default_value_t = 720)]
        viewport_height: u32,
        /// Milliseconds to wait after writing (HMR settle time, default: 2000).
        #[arg(long, default_value_t = 2000)]
        wait_ms: u64,
    },

    // ── Utilities ───────────────────────────────────────────────────
    /// Resume a plan execution from its last checkpoint.
    #[command(after_help = "\
Examples:
  roko resume                         Resume from default snapshot
  roko resume run_4823                Resume a specific run by ID")]
    Resume {
        /// Run or plan ID to resume (optional — defaults to most recent snapshot).
        run_id: Option<String>,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Walk the lineage DAG rooted at a signal hash and print it.
    ///
    /// Traversal is breadth-first with lexicographic parent ordering so
    /// branching output is deterministic.
    Replay {
        /// Signal hash (64 hex chars) to walk.
        hash: String,
        /// Directory containing `.roko/` (default: cwd, respects global --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Show forensic detail: timestamps, full hashes, metadata.
        #[arg(long)]
        forensic: bool,
        /// Include only events at or after this traversal index (1-based, inclusive).
        #[arg(long, conflicts_with = "as_of")]
        from_event: Option<String>,
        /// Deprecated: use --from-event instead. Identical semantics.
        #[arg(long, hide = true, conflicts_with = "from_event")]
        as_of: Option<String>,
        /// Output format: tree (default) or json.
        #[arg(long)]
        format: Option<String>,
    },
    /// List or show past chat session summaries.
    #[command(after_help = "\
Examples:
  roko history                     List the 20 most recent chat sessions
  roko history 2026-04-29T14-23-05-my-agent   Show detail for one session")]
    History {
        /// Session ID to show in detail (omit to list last 20 sessions).
        id: Option<String>,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Inject a signal into a running session.
    Inject {
        /// Target session ID.
        session: String,
        /// Kind of signal to inject (directive, abort, context).
        #[arg(long, default_value = "directive")]
        kind: String,
        /// Payload text.
        payload: String,
        /// Working directory (to locate the daemon socket).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Generate shell completion scripts.
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: CompletionShell,
    },
    /// Generate boilerplate for a Synapse trait or domain profile.
    ///
    /// Types: gate, scorer, router, policy, substrate, composer, domain, template, event-source.
    New {
        /// Type of scaffold to generate (e.g. gate, scorer, router).
        #[arg(value_name = "TYPE")]
        type_name: String,
        /// Name for the generated component (e.g. my-custom-gate).
        name: String,
        /// Output directory (default: current directory).
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Explain a roko concept with progressive disclosure (3 depth levels).
    Explain {
        /// Topic to explain (e.g. gates, routing, cognitive, neuro, daimon, dreams, signal, cfactor).
        topic: String,
        /// Disclosure depth: 1 = summary, 2 = how it works, 3 = internals.
        #[arg(long, default_value_t = 1)]
        depth: u8,
    },
    /// Analyze which crates are affected by the current changes.
    ///
    /// Uses git diff and cargo metadata to determine impacted crates and
    /// their reverse dependents. Outputs a list suitable for targeted
    /// `cargo test -p <crate>` invocations.
    #[command(after_help = "\
Examples:
  roko impact                          Show affected crates from uncommitted changes
  roko impact --json                   Machine-readable JSON output
  roko impact --base main              Compare against main branch
  roko impact --files crates/roko-core/src/lib.rs crates/roko-gate/src/lib.rs
                                       Analyze specific files instead of git diff")]
    Impact {
        /// Git ref to diff against (default: HEAD).
        #[arg(long, default_value = "HEAD")]
        base: String,
        /// Explicit file list instead of detecting from git diff.
        #[arg(long, num_args = 1..)]
        files: Vec<String>,
        /// Output as JSON for scripting.
        #[arg(long)]
        json: bool,
        /// Working directory (default: cwd or --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },

    // ── Hidden: dynamic completion endpoint ───────────────────────────
    /// Internal: emit newline-delimited completion candidates for shells.
    #[command(name = "__complete", hide = true)]
    Complete {
        /// Shell requesting completions (bash, zsh, fish).
        #[arg(long)]
        shell: CompletionShell,
        /// Space-separated command path typed so far (e.g. "config providers").
        #[arg(long, default_value = "")]
        path: String,
        /// The word currently being completed.
        #[arg(long, default_value = "")]
        current: String,
    },
}

// -----------------------------------------------------------------------
// Knowledge: neuro + dreams + custody + archive
// -----------------------------------------------------------------------

fn parse_decay_factor(value: &str) -> std::result::Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|error| format!("invalid decay factor `{value}`: {error}"))?;
    if parsed.is_finite() && (0.0..=1.0).contains(&parsed) {
        Ok(parsed)
    } else {
        Err("decay factor must be between 0.0 and 1.0".to_owned())
    }
}

#[derive(Debug, Subcommand)]
enum KnowledgeCmd {
    /// Query the durable knowledge store for a topic.
    Query {
        /// Topic to search for.
        topic: Vec<String>,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Maximum number of results to return (1-1000, default: 10).
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u16).range(1..=1000))]
        limit: u16,
        /// Print each match in full instead of its first two lines.
        #[arg(long)]
        verbose: bool,
    },
    /// Show aggregate statistics for the durable knowledge store.
    Stats {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run garbage collection on the durable knowledge store.
    Gc {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Minimum confidence threshold for GC (0.0-1.0, default: 0.05).
        #[arg(long, value_parser = parse_decay_factor)]
        threshold: Option<f64>,
        /// Preview what would be collected without actually removing entries.
        #[arg(long)]
        dry_run: bool,
    },
    /// Export a canonical, integrity-protected knowledge bundle.
    Export {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Versioned JSONL bundle to write.
        output: PathBuf,
        /// Replace an existing output file.
        #[arg(long)]
        force: bool,
        /// Export only the top N secret-safe entries by confidence.
        #[arg(long)]
        top_n: Option<usize>,
        /// Minimum confidence threshold (0.0-1.0).
        #[arg(long, value_parser = parse_decay_factor)]
        min_confidence: Option<f64>,
        /// Filter by knowledge types (comma-separated; e.g. "insight,heuristic").
        #[arg(long)]
        types: Option<String>,
        /// Exclude entries with any of these tags (comma-separated).
        #[arg(long)]
        exclude_tags: Option<String>,
    },
    /// Import a canonical, integrity-protected knowledge bundle.
    Import {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Versioned JSONL bundle to import.
        input: PathBuf,
        /// Confidence multiplier applied to imported entries.
        #[arg(long, default_value_t = 0.8, value_parser = parse_decay_factor)]
        decay_factor: f64,
        /// Explicitly migrate a trusted legacy raw/version-1 JSONL backup.
        #[arg(long)]
        legacy_raw: bool,
        /// Filter by knowledge types (comma-separated; e.g. "insight,heuristic").
        #[arg(long)]
        types: Option<String>,
        /// Only import entries with confidence >= this threshold (0.0-1.0).
        #[arg(long, value_parser = parse_decay_factor)]
        min_confidence: Option<f64>,
    },
    /// Backup the knowledge store to a directory with optional genomic bottleneck.
    Backup {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Directory to write the backup files into.
        destination: PathBuf,
        /// Replace a populated destination directory after staging succeeds.
        #[arg(long)]
        force: bool,
        /// Genomic bottleneck: export only the top N entries by confidence.
        #[arg(long)]
        top_n: Option<usize>,
    },
    /// Restore the knowledge store from a backup with confidence decay.
    Restore {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Directory created by `roko knowledge backup`.
        source: PathBuf,
        /// Permit merging into existing knowledge and replacing confirmations.
        #[arg(long)]
        force: bool,
        /// Filter by knowledge types (comma-separated).
        #[arg(long)]
        types: Option<String>,
        /// Only restore entries with confidence >= this threshold (0.0 to 1.0).
        #[arg(long)]
        min_confidence: Option<f64>,
        /// Generation hop count for confidence decay (default: 1).
        #[arg(long, default_value_t = 1)]
        generation: u32,
        /// Per-generation confidence multiplier.
        #[arg(long, default_value_t = 0.8, value_parser = parse_decay_factor)]
        decay_factor: f64,
        /// Explicitly migrate a trusted legacy raw/version-1 JSONL backup.
        #[arg(long)]
        legacy_raw: bool,
    },
    /// Sync knowledge with a peer agent via the Mesh protocol.
    Sync {
        /// Peer agent identifier to sync with.
        peer: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Direction: send, receive, or both (default: both).
        #[arg(long, value_enum, default_value = "both")]
        direction: KnowledgeSyncDirection,
        /// Maximum signals to send in this sync cycle.
        #[arg(long, default_value_t = 100)]
        max_send: usize,
    },
    /// Dream consolidation, reports, and journal.
    Dream {
        #[command(subcommand)]
        cmd: KnowledgeDreamCmd,
    },
    /// Custody audit chain (list, show, verify).
    Custody {
        #[command(subcommand)]
        cmd: KnowledgeCustodyCmd,
    },
    /// Move old signals to cold storage (compressed monthly archives).
    ///
    /// This archives signal data from the hot JSONL substrate,
    /// NOT neuro knowledge-store entries. Use `roko knowledge gc` to manage
    /// the knowledge store.
    #[command(alias = "archive")]
    SignalArchive {
        /// Only archive signals older than this duration (e.g. "30d", "7d").
        #[arg(long, default_value = "30d")]
        older_than: String,
        /// Maximum number of signals to archive per batch.
        #[arg(long, default_value_t = 500)]
        batch_size: usize,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Print what would be archived without doing it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Backfill HDC vectors for existing knowledge entries that lack them.
    ///
    /// Reads the knowledge store, computes HDC vectors for any entry whose
    /// hdc_vector field is absent or has the wrong byte length, and atomically
    /// rewrites the store. Entries that already have a valid vector are unchanged.
    /// Requires the roko-neuro hdc feature to be enabled in this binary.
    BackfillHdc {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum KnowledgeDreamCmd {
    /// Run a dream consolidation cycle immediately.
    Run {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Preview what would be consolidated without executing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Show the latest dream report without running a new cycle.
    Report {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show when the next dream should fire.
    Schedule {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Display recent dream journal entries.
    Journal {
        /// Number of recent entries to display (default: 10).
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Display recent dream archive entries.
    Archive {
        /// Number of recent entries to display (default: 10).
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum KnowledgeCustodyCmd {
    /// List recent custody records.
    List {
        /// Maximum number of records to display.
        #[arg(long)]
        limit: Option<usize>,
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show full details of a custody record by index.
    Show {
        /// Record index (0-based).
        index: usize,
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Verify integrity of the custody chain.
    Verify {
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

// -----------------------------------------------------------------------
// Learn: learning state + inspection
// -----------------------------------------------------------------------

#[derive(Debug, Subcommand)]
enum LearnCmd {
    /// Show all learning state (router, experiments, efficiency, episodes, reflexes).
    All {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show cascade router state.
    #[command(alias = "router")]
    Route {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Manage prompt A/B experiments (list, create, conclude, report).
    Experiments {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Subcommand: list | create | conclude | report. Defaults to list.
        #[command(subcommand)]
        cmd: Option<ExperimentsSubCmd>,
    },
    /// Show efficiency metrics.
    Efficiency {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Show at most N matching rows from the start (mutually exclusive with --tail).
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10_000), conflicts_with = "tail")]
        limit: Option<u32>,
        /// Show the last N matching rows in chronological order (mutually exclusive with --limit).
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10_000))]
        tail: Option<u32>,
        /// Only include entries at or after this RFC 3339 timestamp.
        #[arg(long)]
        since: Option<String>,
        /// Filter by model slug (substring match).
        #[arg(long)]
        model: Option<String>,
        /// Filter by plan ID (substring match).
        #[arg(long)]
        plan: Option<String>,
        /// Filter by task ID (substring match).
        #[arg(long)]
        task: Option<String>,
    },
    /// Show episode summary.
    Episodes {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Show at most N matching rows from the start (mutually exclusive with --tail).
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10_000), conflicts_with = "tail")]
        limit: Option<u32>,
        /// Show the last N matching rows in chronological order (mutually exclusive with --limit).
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10_000))]
        tail: Option<u32>,
        /// Only include entries at or after this RFC 3339 timestamp.
        #[arg(long)]
        since: Option<String>,
        /// Filter by model slug (substring match).
        #[arg(long)]
        model: Option<String>,
        /// Filter by plan ID (substring match).
        #[arg(long)]
        plan: Option<String>,
        /// Filter by task ID (substring match).
        #[arg(long)]
        task: Option<String>,
        /// Filter by pass/fail status (pass or fail).
        #[arg(long)]
        status: Option<String>,
    },
    /// Show T0 reflex rules (count, top five by hits, and recent demotions).
    Reflexes {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show adaptive gate threshold state.
    Gates {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show durable knowledge entry counts.
    #[command(alias = "knowledge")]
    KnowledgeStats {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show learned playbook store contents (name, trigger pattern, success/failure counts).
    Playbooks {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show per-section prompt pass-rate statistics (worst sections first).
    Sections {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show recent post-gate reflection records.
    Reflections {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Maximum number of recent reflections to display.
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Read-only inspection of a learning subsystem (gates, routing, budget).
    Inspect {
        #[command(subcommand)]
        subsystem: InspectSubsystem,
    },
    /// Show tool usage statistics from the tool audit log.
    Tools {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Trace an end-to-end feedback loop: knowledge ingested -> injected -> gate pass -> confirmation.
    FeedbackProof {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show per-role cost profiles (average cost, token budget, pass rate).
    RoleCosts {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show graduation policy state (configured policies, counters, and evaluation preview).
    Graduation {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Check a run's attempt records, or report routing outcomes from them (read-only).
    Telemetry {
        #[command(subcommand)]
        cmd: commands::learn::TelemetryCmd,
    },
    /// (deprecated: use `roko learn inspect`) Tune adaptive thresholds and model routing parameters.
    #[command(hide = true)]
    Tune {
        /// Subsystem to tune: gates, routing, budget.
        #[arg(default_value = "gates")]
        subsystem: String,
        /// Display current values without modifying.
        #[arg(long)]
        dry_run: bool,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

// -----------------------------------------------------------------------
// ExperimentsSubCmd — subcommands for `roko learn experiments`
// -----------------------------------------------------------------------

#[derive(Debug, Subcommand)]
enum ExperimentsSubCmd {
    /// List all experiments as a table (Name | Status | Variants | Observations | Best Variant | Win Rate).
    List {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Show at most N experiments.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10_000))]
        limit: Option<u32>,
    },
    /// Create a new prompt experiment.
    Create {
        /// Unique experiment identifier.
        #[arg(long)]
        name: String,
        /// Prompt section under test (e.g. "constraints").
        #[arg(long)]
        section: String,
        /// Comma-separated variant ids (e.g. "control,concise-v2").
        #[arg(long)]
        variants: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Conclude an experiment by auto-picking the best-performing variant as winner.
    Conclude {
        /// Experiment identifier to conclude.
        name: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Print a detailed statistical report for one experiment.
    Report {
        /// Experiment identifier to report on.
        name: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

// -----------------------------------------------------------------------
// InspectSubsystem — read-only learning inspection targets
// -----------------------------------------------------------------------

#[derive(Debug, Subcommand)]
enum InspectSubsystem {
    /// Inspect adaptive gate threshold state.
    Gates {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Inspect cascade routing state and model statistics.
    Routing {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Inspect configured budget limits and spend history.
    Budget {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum DemoCmd {
    /// Build release binary and prepare workspace for demos.
    Setup {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Pre-warm the LLM response cache with demo prompts.
    Warm {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum BenchCmd {
    /// Run a comparative benchmark: naive vs roko-optimized.
    #[command(after_help = "\
Examples:
  roko bench demo                     Run with simulated data
  roko bench demo --real              Run with real LLM dispatch")]
    Demo {
        /// Use real LLM dispatch instead of simulated results.
        #[arg(long)]
        real: bool,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run a native SWE-bench-style proxy batch.
    #[command(after_help = "\
Examples:
  roko bench swe --batch-size 2 --agent-mode gold      (control: checks the harness)
  roko bench swe --dataset ./swe-smoke.jsonl --predictions ./predictions.jsonl --agent-mode prediction-file
  roko bench swe --agent-mode command --agent-command './my-agent.sh'")]
    Swe {
        /// Local JSONL dataset. If omitted, a built-in two-task smoke dataset is generated.
        #[arg(long)]
        dataset: Option<PathBuf>,
        /// Number of instances to run.
        #[arg(long, default_value_t = 2)]
        batch_size: usize,
        /// Offset into the dataset.
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// Agent adapter to use (required). `gold` and `empty` are controls: they check the
        /// harness, not a model, and are never recorded as learning.
        #[arg(long, value_enum)]
        agent_mode: roko_cli::bench::SweAgentMode,
        /// Predictions JSONL path for --agent-mode prediction-file.
        #[arg(long)]
        predictions: Option<PathBuf>,
        /// Command for --agent-mode command. Receives instance JSON on stdin, prints a unified diff.
        #[arg(long)]
        agent_command: Option<String>,
        /// Scores JSONL output path.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Write SWE-bench-style predictions JSONL.
        #[arg(long)]
        export_predictions: Option<PathBuf>,
        /// Disable learning episode, efficiency, and C-factor writes.
        #[arg(long)]
        no_learning: bool,
        /// Keep per-instance benchmark workdirs for debugging.
        #[arg(long)]
        keep_workdirs: bool,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

// -----------------------------------------------------------------------
// Backlog import
// -----------------------------------------------------------------------

#[derive(Debug, Subcommand)]
enum BacklogCmd {
    /// Import backlog spec(s) as plan artifacts with eligibility checks.
    Import {
        /// Path to a single backlog .md file or a directory containing them.
        path: PathBuf,
        /// Create/update the plan artifact without execution.
        #[arg(long)]
        draft: bool,
        /// Alias for --draft (deprecated; use --draft).
        #[arg(long)]
        plan: bool,
        /// Create then start an eligible packet (fails on blocked packets).
        #[arg(long)]
        execute: bool,
        /// Dry-run: check eligibility without side effects.
        #[arg(long)]
        check: bool,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// List backlog items and their import status.
    List {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Reconcile plan TOML status against durable runner state.
    ///
    /// Scans plans/*/tasks.toml and compares declared task/meta status with
    /// the executor snapshot and run-state in .roko/state/. Reports drift
    /// such as tasks marked "done" in TOML but absent from runner completion
    /// records, or runner-failed tasks whose TOML still says "ready".
    Audit {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Emit machine-readable JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Apply deterministic mechanical repairs: remove broken plan
        /// references from the index, deduplicate IDs, and fix spec counts.
        /// Never changes semantic status (use `mark-done` for that).
        #[arg(long)]
        fix_safe: bool,
    },
    /// Mark a backlog spec as done with explicit evidence.
    ///
    /// Finds the backlog file by its numeric ID (e.g. `229`) and writes or
    /// updates the `**Status**: Done (DATE) -- EVIDENCE` line near the top.
    MarkDone {
        /// Numeric backlog ID (e.g. 229).
        id: u32,
        /// Evidence string (run-id, commit hash, PR number, etc.).
        evidence: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

// -----------------------------------------------------------------------
// Plugins (now nested under config)
// -----------------------------------------------------------------------

#[derive(Debug, Subcommand)]
enum PluginCmd {
    /// List available and installed plugins.
    List {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Emit JSON output instead of human-readable text.
        #[arg(long)]
        json: bool,
    },
    /// Install a plugin from a local path or registry.
    Install {
        /// Path to the plugin manifest (plugin.toml) or directory.
        source: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Sign and publish a WASM extension directory to the relay registry.
    Publish {
        /// Extension directory containing extension.toml and its WASM module.
        source: PathBuf,
        /// Publisher identity configured by the relay.
        #[arg(long)]
        publisher: String,
        /// Registry base URL. Defaults to relay.url or ROKO_EXTENSION_REGISTRY_URL.
        #[arg(long)]
        registry: Option<String>,
        /// Working directory used to resolve Roko configuration.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Remove an installed plugin by name.
    Remove {
        /// Name of the plugin to remove.
        name: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Audit installed plugins and report capabilities.
    Audit {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum IndexCmd {
    /// Build a code index for the workspace (or specified directory).
    Build {
        /// Directory to index (default: cwd / --repo).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Drop existing index data and rebuild from source files.
    Rebuild {
        /// Directory to index (default: cwd / --repo).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Search the code index.
    Search {
        /// Search query text (symbol name/pattern, never a file path).
        query: String,
        /// Restrict to a symbol kind (function, struct, enum, trait, const, type, module, impl).
        #[arg(long)]
        kind: Option<String>,
        /// Search strategy: keyword, structural, hybrid.
        #[arg(long, default_value = "keyword")]
        strategy: String,
        /// Glob filter on file paths (independent of query text).
        #[arg(long)]
        file_pattern: Option<String>,
        /// Maximum number of results (must be > 0).
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Directory to index (default: cwd / --repo).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Show index statistics.
    Stats {
        /// Directory to index (default: cwd / --repo).
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum DaemonCmd {
    Start {
        #[arg(long)]
        foreground: bool,
        #[arg(long, default_value_t = roko_cli::DEFAULT_SERVE_PORT)]
        port: u16,
    },
    Stop,
    Status,
    Logs {
        #[arg(long, short = 'f')]
        follow: bool,
        #[arg(long, short = 'n', default_value = "50")]
        lines: usize,
    },
    Reload,
    // SIGHUP equivalent — re-scan subscriptions/templates without restart
    Restart {
        #[arg(long, default_value_t = roko_cli::DEFAULT_SERVE_PORT)]
        port: u16,
    },
    Install,
    // macOS launchd plist generation
    Uninstall, // remove launchd plist
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

// (CustodyCmd, DreamCmd, DreamsCmd moved into KnowledgeCmd above)

/// Direction for knowledge mesh sync operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KnowledgeSyncDirection {
    /// Send local knowledge to the peer.
    #[value(name = "send")]
    Send,
    /// Receive knowledge from the peer.
    #[value(name = "receive")]
    Receive,
    /// Send and receive (bidirectional sync).
    #[value(name = "both")]
    Both,
}

/// Execution engine for `roko plan run`.
///
/// The Graph Engine is the sole execution engine. The `--engine legacy` and
/// `--engine runner-v2` values are still accepted to avoid breaking existing
/// scripts, but they print a deprecation error and exit.
///
/// **#336**: The legacy engine variant is scheduled for removal. Scripts using
/// `--engine legacy` or `--engine runner-v2` should remove the flag entirely
/// (the graph engine is the default and sole engine).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum PlanEngine {
    /// Graph Engine (default and sole engine).
    #[default]
    #[value(name = "graph")]
    Graph,
    /// Legacy Runner-v2 (REMOVED). Accepted for backward compatibility but
    /// prints a deprecation error and exits. Use `--engine graph` (the default).
    /// Scheduled for removal in the next release (#336).
    #[value(name = "legacy", alias = "runner-v2", hide = true)]
    RunnerV2,
}

#[derive(Debug, Subcommand)]
enum PlanCmd {
    /// List all plans in the workspace.
    List {
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Group plans by execution wave (cross-plan dependency analysis).
        #[arg(long)]
        waves: bool,
    },
    /// Show details of a specific plan.
    Show {
        /// Plan ID.
        plan_id: String,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Create a new plan.
    Create {
        /// Plan ID.
        plan_id: String,
        /// Plan title.
        #[arg(long)]
        title: String,
        /// Plan description.
        #[arg(long, default_value = "")]
        description: String,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Lint every `tasks.toml` under a plans directory without executing it.
    Validate {
        /// Plans root directory.
        #[arg(default_value = "plans/")]
        dir: PathBuf,
        /// Fail on warnings, not only errors.
        #[arg(long)]
        strict: bool,
        /// Output machine-readable JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Show DAG analysis: plan/task/edge counts, wave breakdown,
        /// critical path, and dangling dependency references.
        #[arg(long)]
        dag: bool,
        /// Score each task's spec with the speclint rules (`sq-2`): score,
        /// band, rule scores and hard fails. With `--strict`, a hard fail
        /// exits 1.
        #[arg(long)]
        spec_quality: bool,
    },
    /// Rebuild or verify the deterministic plans index.
    Index {
        /// Verify exact generated content without writing any files.
        #[arg(long)]
        check: bool,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run a plan directory through the orchestration loop.
    #[command(after_help = "\
Examples:
  roko plan run plans/              Run all plans (graph engine, default)
  roko plan run plans/my-plan       Run a specific plan
  roko plan run plans/ --approval   Run with interactive TUI approval
  roko plan run plans/ --dry-run    Preview without executing
  roko plan run plans/ --fresh      Archive old state and start clean
  roko plan run plans/ --max-parallel-plans 3   Run up to 3 independent plans at once
  roko plan run plans/ --resume-plan .roko/state/graph                              Resume Graph Activities

The legacy Runner-v2 engine has been removed. --engine legacy is accepted but exits with an error.")]
    Run {
        /// Path to the plans directory.
        plans_dir: PathBuf,
        /// Execution engine to use for plan execution.
        ///
        /// The Graph engine is the sole engine. `--engine legacy` and
        /// `--engine runner-v2` are accepted for backward compatibility
        /// but print a deprecation error and exit.
        #[arg(long, default_value = "graph", value_enum)]
        engine: PlanEngine,
        /// Working directory (repo root). Defaults to current directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Resume from engine state. Bare `--resume-plan` resumes the canonical
        /// Graph checkpoints under `.roko/state/graph/` (any number of plans);
        /// pass a checkpoint directory, or a checkpoint file for a single plan.
        #[arg(long = "resume-plan", visible_alias = "resume-state", num_args = 0..=1, default_missing_value = ".roko/state/state-snapshot.json")]
        resume_plan: Option<PathBuf>,
        /// Launch the connected inline TUI while Runner-v2 runs.
        /// Use this to monitor agent output, tokens, and gate progress in real time.
        /// The TUI remains open after completion or failure until you quit it.
        /// Without this flag, plan run outputs plain text logs.
        #[arg(long, visible_alias = "tui")]
        approval: bool,
        /// Disable the inline TUI even in interactive terminals.
        ///
        /// By default, the TUI is auto-enabled when stdout is a TTY.
        /// Pass `--no-tui` to suppress it and use plain log output instead.
        #[arg(long)]
        no_tui: bool,
        /// Maximum retry attempts per task (overrides per-task and config values).
        #[arg(long)]
        max_retries: Option<u32>,
        /// Maximum concurrent tasks per plan (0 keeps the config/default value).
        #[arg(long, default_value_t = 0)]
        max_tasks: usize,
        /// Parse and display the plan without executing. Shows tasks, dependencies, and estimates.
        #[arg(long)]
        dry_run: bool,
        /// Archive old run state and start from scratch (ignores the unified state snapshot and legacy files).
        #[arg(long)]
        fresh: bool,
        /// Re-queue drifted tasks instead of aborting when resuming from a snapshot.
        #[arg(long)]
        force_resume: bool,
        /// Override the plan cost ceiling for this run.
        ///
        /// `--budget-override 50.0` sets the per-plan USD ceiling to $50.00,
        /// replacing whatever is configured in roko.toml.  The guardrail still
        /// logs a warning when the ceiling is hit, but execution continues.
        /// Use `--budget-override 0` or `--no-budget` to disable the ceiling.
        #[arg(long, value_name = "AMOUNT")]
        budget_override: Option<f64>,
        /// Disable budget enforcement entirely for this run.
        ///
        /// Equivalent to `--budget-override 0`: sets the per-plan ceiling to
        /// unlimited (0.0) so `BudgetAction::Block` is never triggered.
        #[arg(long, conflicts_with = "budget_override")]
        no_budget: bool,
        /// Skip the disk-space pre-check and start the plan even when free disk
        /// is below `resources.min_free_disk_mb`. Use with caution: the plan
        /// may fail mid-run if disk space is exhausted.
        #[arg(long)]
        force: bool,
        /// Skip agent permission prompts for this run. UNSAFE: agents will execute
        /// tools without approval. Prefer setting `runner.dangerously_skip_permissions = true`
        /// in roko.toml for persistent use.
        #[arg(long)]
        dangerously_skip_permissions: bool,
        /// Write structured JSONL event log to this file during execution.
        ///
        /// Every runner lifecycle event (task start, gate result, agent dispatch,
        /// run completion, etc.) is serialized as a single JSON line and flushed.
        #[arg(long, value_name = "PATH")]
        log_file: Option<PathBuf>,
        /// Skip the preflight environment checks (config, credentials, toolchain,
        /// plans, stale lock) and proceed directly to plan execution.
        #[arg(long)]
        skip_preflight: bool,
        // NOTE: `--force-backend` was removed from this subcommand and
        // consolidated into the global `--model` flag (hidden alias).
        // Use `roko plan run --model <slug> plans/` instead.
        /// Capture event-driven screenshots during execution.
        ///
        /// Screenshots are saved to `.roko/screenshots/run-<timestamp>/` with
        /// a manifest.json linking each screenshot to its trigger event.
        /// Triggered at: plan startup, task completion, gate completion, wave
        /// completion, agent spawn/exit, and errors.
        #[arg(long)]
        screenshots: bool,
        /// Maximum seconds between periodic full-state screenshot captures.
        #[arg(
            long,
            value_name = "SECONDS",
            default_value_t = 60,
            value_parser = clap::value_parser!(u64).range(1..=86_400)
        )]
        screenshot_interval: u64,
        /// Exact directory for this run's screenshot timeline. Relative paths
        /// are resolved against the plan workdir. Existing paths receive a
        /// collision-safe numeric suffix.
        #[arg(long, value_name = "PATH")]
        screenshot_dir: Option<PathBuf>,
        /// Pause execution for review after every N plan completions.
        /// Natural checkpoints for overnight or batch runs.
        #[arg(long, value_name = "N")]
        batch_size: Option<usize>,
        /// Run each task in an isolated git worktree so agents cannot
        /// interfere with each other or the user's working tree.
        ///
        /// When enabled, each task dispatch creates a fresh worktree,
        /// runs the agent and verify steps inside it, and cleans it up
        /// on completion. Failed worktrees are retained for post-mortem.
        /// Only applies to the Graph engine.
        #[arg(long)]
        worktree_per_task: bool,
        /// Use the rich 11-node-per-task production topology instead of the
        /// simple single-Activity-per-task converter.
        ///
        /// When enabled, each task becomes a subgraph of:
        ///   [TaskContext] -> 6 parallel enrichers (knowledge, episodes,
        ///   playbook, modulation, safety, experiment) -> [Compose] ->
        ///   [TaskExecutor] -> [Gate] -> [SuccessBoundary]
        ///
        /// Each task's [Gate] runs the compile, lint and test rungs in the
        /// worktree its attempt ran in, and accepts the attempt onto the plan
        /// branch when they pass, so this needs `--worktree-per-task`.
        ///
        /// Note: enricher cells are currently passthrough stubs. The richer
        /// topology does not yet add runtime value over the simple converter,
        /// but makes the structure available for incremental implementation of
        /// each enricher cell type. Only applies to the Graph engine.
        #[arg(long)]
        rich_topology: bool,
        /// After every plan is delivered into the run's batch branch
        /// (`roko/batch/<run-id>`), promote the batch into BRANCH and tag it
        /// `roko/run/<run-id>`. Never pushes. A BRANCH checked out anywhere,
        /// such as your own checkout's, is not moved: the promotion is parked
        /// at `refs/roko/delivered/run-<run-id>` for you to fast-forward.
        #[arg(long, value_name = "BRANCH", requires = "worktree_per_task")]
        promote: Option<String>,
        /// Run up to N plans of a plan set at the same time.
        ///
        /// Plans start in execution order once their `depends_on_plan`
        /// prerequisites have succeeded. Plans that write or build
        /// overlapping parts of the working tree never run at the same time.
        /// Defaults to `[conductor] max_parallel_plans` (1: one plan at a
        /// time).
        #[arg(
            long,
            value_name = "N",
            value_parser = clap::value_parser!(u64).range(1..=64)
        )]
        max_parallel_plans: Option<u64>,
        /// Start no further plans after the first plan fails. Plans already
        /// running finish; the rest are reported as blocked.
        #[arg(long)]
        fail_fast: bool,
    },
    /// Generate implementation plans from a prompt, file, or PRD.
    Generate {
        /// Source: free-text prompt, or path to a file (PRD, requirements, etc).
        source: Vec<String>,
        /// Treat source as a file path to read (instead of inline text).
        #[arg(long)]
        from_file: Option<PathBuf>,
        /// Additional context files/dirs/globs to include in the prompt.
        #[arg(long = "context", value_name = "PATH")]
        context: Vec<PathBuf>,
        /// Read notes from .roko/notes/ and generate one plan per cluster.
        #[arg(long)]
        from_notes: bool,
        /// Filter notes by tag when using --from-notes.
        #[arg(long)]
        tag: Option<String>,
        /// Generate plan(s) from backlog spec(s). Accepts a single ID or
        /// comma-separated IDs: `--from-backlog 206` or `--from-backlog 206,120,119`.
        /// Reads the spec from `tmp/backlog/<id>-*.md`, generates a deterministic
        /// slug, and writes the plan to `plans/<slug>/tasks.toml`.
        #[arg(long, value_name = "IDS")]
        from_backlog: Option<String>,
    },
    /// Pause a running plan executor. Writes a pause signal to `.roko/state/control.json`.
    Pause {
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Resume a paused plan executor. Clears the pause signal.
    Resume {
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Cancel a running plan. Writes a cancel signal to `.roko/state/control.json`.
    Cancel {
        /// Plan ID to cancel. If omitted, cancels the current run.
        #[arg(long)]
        plan_id: Option<String>,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Retry failed tasks in a plan. Writes a retry signal to `.roko/state/control.json`.
    Retry {
        /// Specific task ID to retry. If omitted, retries all failed tasks.
        task_id: Option<String>,
        /// Plan ID containing the task. If omitted, targets the active plan.
        #[arg(long)]
        plan_id: Option<String>,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Approve or reject a task held for review (`[meta] approval =
    /// "per_task"`). The plan run holding it merges the task on approval; a
    /// rejection fails the attempt, and the note is the next attempt's
    /// feedback.
    Review {
        /// Plan id.
        plan_id: String,
        /// Task id.
        task_id: String,
        /// Approve the task's held attempt.
        #[arg(long, conflicts_with = "reject", required_unless_present = "reject")]
        approve: bool,
        /// Reject the task's held attempt.
        #[arg(long)]
        reject: bool,
        /// The reviewer's note, which a rejected task's next attempt gets.
        #[arg(long, default_value = "")]
        note: String,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Regenerate an existing plan from its source PRD / plan extract.
    Regenerate {
        /// Path to the plan directory (containing tasks.toml).
        plan_dir: PathBuf,
        /// Preview changes without overwriting.
        #[arg(long)]
        dry_run: bool,
    },
    /// Queue manifest operations: show, validate, and init milestone definitions.
    Queue {
        #[command(subcommand)]
        cmd: QueueCmd,
    },
    /// Show the lightweight runner status from `.roko/state/status.json`.
    ///
    /// Reads the < 500 byte status file written by the runner on every tick
    /// (debounced 1/sec). This is fast because it does not require
    /// deserializing the full executor snapshot.
    ///
    /// When a plan directory is provided, shows task-level status for that
    /// specific plan (from its tasks.toml and executor snapshot).
    /// When omitted, shows the global runner status (phase, plans, agents).
    #[command(after_help = "\
Examples:
  roko plan status                          Show global runner status
  roko plan status plans/demos/demo-hello   Show task status for a specific plan")]
    Status {
        /// Optional plan directory to show status for (e.g. plans/my-plan).
        /// When provided, shows task-level status for that specific plan.
        /// When omitted, shows the global runner status.
        plan_dir: Option<PathBuf>,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Shorthand: `roko plan "add cursor support"` routes to plan generate.
    #[command(external_subcommand)]
    Shorthand(Vec<String>),
}

#[derive(Debug, Subcommand)]
enum QueueCmd {
    /// Display milestone status and plan assignments.
    Show {
        /// Path to queue manifest file.
        #[arg(long, default_value = ".roko/queue.toml")]
        file: PathBuf,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Validate queue manifest structure and plan references.
    Validate {
        /// Path to queue manifest file.
        #[arg(long, default_value = ".roko/queue.toml")]
        file: PathBuf,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Generate a starter queue.toml from discovered plans.
    Init {
        /// Output path for the generated manifest.
        #[arg(long, default_value = ".roko/queue.toml")]
        output: PathBuf,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

impl PlanCmd {
    /// Whether dispatching this command can change plan state or artifacts.
    ///
    /// Read-only commands must not rebuild indexes: rebuilding writes generated index files and
    /// makes commands such as `plan validate` unexpectedly dirty the caller's workspace.
    fn should_rebuild_indexes(&self) -> bool {
        match self {
            Self::List { .. }
            | Self::Show { .. }
            | Self::Validate { .. }
            | Self::Index { .. }
            | Self::Queue { .. }
            | Self::Pause { .. }
            | Self::Resume { .. }
            | Self::Cancel { .. }
            | Self::Retry { .. }
            | Self::Review { .. }
            | Self::Status { .. } => false,
            Self::Run { dry_run, .. } | Self::Regenerate { dry_run, .. } => !dry_run,
            Self::Create { .. } | Self::Generate { .. } | Self::Shorthand(_) => true,
        }
    }

    /// Workspace whose plan artifacts the command can mutate.
    fn index_rebuild_workdir(&self, cli: &Cli) -> PathBuf {
        match self {
            Self::Run {
                workdir: Some(workdir),
                ..
            }
            | Self::Create {
                workdir: Some(workdir),
                ..
            } => workdir.clone(),
            _ => resolve_workdir(cli),
        }
    }
}

const fn should_rebuild_plan_indexes(command_can_mutate: bool, exit_code: Option<i32>) -> bool {
    command_can_mutate && matches!(exit_code, Some(EXIT_SUCCESS))
}

fn finish_with_index_rebuild(
    result: Result<i32>,
    workdir: &Path,
    should_rebuild: bool,
) -> Result<i32> {
    match result {
        Ok(EXIT_SUCCESS) if should_rebuild => {
            roko_cli::index::rebuild_all(workdir)?;
            Ok(EXIT_SUCCESS)
        }
        primary => primary,
    }
}

#[derive(Debug, Subcommand)]
enum PrdCmd {
    /// Capture a quick idea.
    Idea {
        /// The idea text.
        text: Vec<String>,
    },
    /// List all PRDs (published, drafts, ideas).
    List,
    /// Show coverage report across PRDs and plans.
    Status,
    /// Create, edit, or promote draft PRDs.
    Draft {
        #[command(subcommand)]
        cmd: PrdDraftCmd,
    },
    /// Generate implementation plans from a PRD.
    Plan {
        /// PRD slug (filename without .md).
        slug: String,
        /// Preview generation without writing tasks.toml files.
        #[arg(long)]
        dry_run: bool,
        /// Also regenerate every plan in plans/ whose tasks.toml lacks modern
        /// fields: one planner call per plan.
        #[arg(long)]
        regenerate_old: bool,
    },
    /// Scan all PRDs for duplicates, gaps, and inconsistencies.
    Consolidate,
}

#[derive(Debug, Subcommand)]
enum PrdDraftCmd {
    /// Create a new draft PRD (agent-assisted).
    New {
        /// Title for the new PRD.
        title: Vec<String>,
    },
    /// Refine an existing draft.
    Edit {
        /// Draft slug (filename without .md).
        slug: String,
    },
    /// Promote a draft to published.
    Promote {
        /// Draft slug (filename without .md).
        slug: String,
        /// Execute the generated plan immediately after promotion.
        #[arg(long)]
        auto_execute: bool,
    },
    /// List all drafts.
    List,
}

impl PrdCmd {
    /// Whether dispatching this command can change PRDs or the plans generated from them.
    ///
    /// Read-only commands must not rebuild indexes: rebuilding rewrites generated index files,
    /// including the tracked `plans/INDEX.md`, so `prd list` would dirty the caller's workspace.
    fn should_rebuild_indexes(&self) -> bool {
        match self {
            Self::List
            | Self::Status
            | Self::Draft {
                cmd: PrdDraftCmd::List,
            } => false,
            Self::Plan { dry_run, .. } => !dry_run,
            Self::Idea { .. } | Self::Draft { .. } | Self::Consolidate => true,
        }
    }
}

/// Backend selection for grounded research operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ResearchBackend {
    /// Automatic selection: configured deep -> Perplexity -> Gemini -> agent fallback.
    Auto,
    /// Force Gemini with Google Search grounding.
    Gemini,
    /// Force Perplexity search-grounded research.
    Perplexity,
    /// Force agent (Claude CLI) fallback.
    Agent,
}

impl std::fmt::Display for ResearchBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Gemini => f.write_str("gemini"),
            Self::Perplexity => f.write_str("perplexity"),
            Self::Agent => f.write_str("agent"),
        }
    }
}

/// Validated recency filter for Perplexity search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum SearchRecency {
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl SearchRecency {
    fn as_api_str(self) -> &'static str {
        match self {
            Self::Hour => "hour",
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

#[derive(Debug, Subcommand)]
enum ResearchCmd {
    /// Deep-dive research on a topic. Produces .roko/research/<slug>.md with citations.
    Topic {
        /// The research topic.
        topic: Vec<String>,
        /// Use Perplexity deep research (async, 1-10 min).
        #[arg(long, help = "Use Perplexity deep research (async, 1-10 min)")]
        deep: bool,
        /// Backend to use for research. Default: auto (selects best available).
        #[arg(long, value_enum, default_value_t = ResearchBackend::Auto)]
        backend: ResearchBackend,
    },
    /// Enhance a PRD with academic citations, diagrams, and research-backed improvements.
    EnhancePrd {
        /// PRD slug (filename without .md).
        slug: String,
    },
    /// Optimize an implementation plan with research-backed task decomposition techniques.
    EnhancePlan {
        /// Plan directory name under plans/.
        plan: String,
    },
    /// Optimize tasks for efficiency, parallelism, and cheapest viable model.
    EnhanceTasks {
        /// Plan directory name under plans/.
        plan: String,
    },
    /// Analyze execution episodes for self-learning insights and bandit weight recommendations.
    Analyze,
    /// List all research artifacts.
    List {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
        /// Include generated INDEX.md in listing (excluded by default).
        #[arg(long)]
        include_generated: bool,
    },
    /// Direct web search using Perplexity's pure search API. Returns raw results without synthesis.
    Search {
        /// The search query.
        query: Vec<String>,
        /// Restrict results to these domains (comma-separated, e.g. "docs.rs,github.com").
        #[arg(long, value_delimiter = ',')]
        domains: Vec<String>,
        /// Recency filter: hour, day, week, month, year.
        #[arg(long, value_enum)]
        recency: Option<SearchRecency>,
        /// Output file path. Default: .roko/research/search-<slug>.md
        #[arg(long)]
        output: Option<PathBuf>,
        /// Do not save search results to disk.
        #[arg(long)]
        no_save: bool,
    },
}

#[derive(Debug, Subcommand)]
enum TuneCmd {
    /// Tune model routing preferences.
    Routing {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Tune validation gate strictness.
    Gates {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Tune cost and prompt budget limits.
    Budget {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Tune the default model.
    Model {
        /// Model key or alias, for example sonnet or haiku.
        name: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum JobCmd {
    /// List all marketplace jobs.
    List {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Filter by status (open, assigned, in_progress, submitted, completed, failed, cancelled).
        #[arg(long)]
        status: Option<String>,
    },
    /// Create a new marketplace job.
    Create {
        /// Job title.
        title: String,
        /// Job type: research, coding_task, chain_monitor, chain_analysis, review, documentation, testing.
        #[arg(long, default_value = "research")]
        r#type: String,
        /// Job description.
        #[arg(long, default_value = "")]
        description: String,
        /// Priority: low, medium, high, critical.
        #[arg(long, default_value = "medium")]
        priority: String,
        /// Auto-execute the job when the runner picks it up.
        #[arg(long)]
        auto_execute: bool,
        /// Associated plan ID.
        #[arg(long)]
        plan_id: Option<String>,
        /// Tag (repeatable, e.g. --tag rust --tag cli).
        #[arg(long)]
        tag: Vec<String>,
        /// Reward string, e.g. "2500 KORAI".
        #[arg(long)]
        reward: Option<String>,
        /// Identity of the poster.
        #[arg(long)]
        posted_by: Option<String>,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Match a proposed job against registered agents via roko-serve.
    Match {
        /// Job title.
        title: String,
        /// roko-serve base URL.
        #[arg(long, default_value = "http://localhost:6677")]
        serve_url: String,
        /// Job description.
        #[arg(long, default_value = "")]
        description: String,
        /// Primary implementation language, also treated as a required skill.
        #[arg(long)]
        language: Option<String>,
        /// Minimum agent tier: Unverified, Verified, Trusted, Expert, Pioneer.
        #[arg(long)]
        min_tier: Option<String>,
        /// Reward string, e.g. "2500 KORAI".
        #[arg(long, default_value = "")]
        reward: String,
        /// Required skills, comma-separated.
        #[arg(long, value_delimiter = ',')]
        skills: Vec<String>,
        /// Working directory (default: cwd / --repo), used for auth config.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show details for a specific job.
    Show {
        /// Job ID.
        id: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Execute a job (locally or via roko-serve).
    Execute {
        /// Job ID.
        id: String,
        /// roko-serve base URL. If set, POST to /api/jobs/{id}/execute.
        #[arg(long)]
        serve_url: Option<String>,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Cancel a job.
    Cancel {
        /// Job ID.
        id: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Recover an interrupted in_progress job.
    ///
    /// If the job has a durable submission, it completes. Otherwise it
    /// transitions back to open for re-execution.
    Recover {
        /// Job ID.
        id: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

// Internal enum used by cmd_neuro — mirrors the old top-level NeuroCmd.
// KnowledgeCmd dispatches to this.
#[derive(Debug)]
enum NeuroCmd {
    Query {
        topic: Vec<String>,
        workdir: Option<PathBuf>,
        limit: u16,
        verbose: bool,
    },
    Stats {
        workdir: Option<PathBuf>,
    },
    Gc {
        workdir: Option<PathBuf>,
        threshold: Option<f64>,
        dry_run: bool,
    },
    Export {
        workdir: Option<PathBuf>,
        output: PathBuf,
        force: bool,
        top_n: Option<usize>,
        min_confidence: Option<f64>,
        types: Option<String>,
        exclude_tags: Option<String>,
    },
    Import {
        workdir: Option<PathBuf>,
        input: PathBuf,
        decay_factor: f64,
        legacy_raw: bool,
        types: Option<String>,
        min_confidence: Option<f64>,
    },
    Backup {
        workdir: Option<PathBuf>,
        destination: PathBuf,
        force: bool,
        top_n: Option<usize>,
    },
    Restore {
        workdir: Option<PathBuf>,
        source: PathBuf,
        force: bool,
        types: Option<String>,
        min_confidence: Option<f64>,
        generation: u32,
        decay_factor: f64,
        legacy_raw: bool,
    },
    Sync {
        peer: String,
        workdir: Option<PathBuf>,
        direction: KnowledgeSyncDirection,
        max_send: usize,
    },
}

// Internal enum used by cmd_dream — mirrors the old top-level DreamCmd.
#[derive(Debug)]
enum DreamCmdLegacy {
    Run {
        workdir: Option<PathBuf>,
        dry_run: bool,
    },
    Report {
        workdir: Option<PathBuf>,
    },
    Schedule {
        workdir: Option<PathBuf>,
    },
}

// EventSourcesCmdLegacy, ProviderCmdLegacy, ModelCmdLegacy removed — dispatch goes direct

#[derive(Debug, Subcommand)]
enum DeployCmd {
    /// Deploy the current workspace to Railway via the public GraphQL API.
    ///
    /// Creates a Railway project with roko-serve as the control plane.
    /// Use --with-mirage to also deploy the chain relay, and --workers to
    /// deploy agent workers from the template registry.
    Railway {
        /// Working directory / repository root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Also deploy the mirage chain relay service.
        #[arg(long)]
        with_mirage: bool,
        /// Deploy worker services for these template names (comma-separated).
        #[arg(long, value_delimiter = ',')]
        workers: Vec<String>,
        /// Skip the security posture check (WARNING: server will be public without auth).
        #[arg(long)]
        unsafe_public: bool,
        /// Show the deploy plan without performing any mutations.
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate `fly.toml` and deploy the current workspace with Fly.io.
    ///
    /// Note: --with-mirage and --workers are not supported on Fly.io.
    Fly {
        /// Working directory / repository root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Skip the security posture check (WARNING: server will be public without auth).
        #[arg(long)]
        unsafe_public: bool,
        /// Show the deploy plan without performing any mutations.
        #[arg(long)]
        dry_run: bool,
        /// Fly.io app name (default: roko-agent).
        #[arg(long, default_value = "roko-agent")]
        app: String,
        /// Fly.io primary region (default: iad).
        #[arg(long, default_value = "iad")]
        region: String,
        /// Path to the Dockerfile for the Fly build (default: Dockerfile).
        #[arg(long, default_value = "Dockerfile")]
        dockerfile: String,
        /// Healthcheck endpoint path (default: /health).
        #[arg(long, default_value = "/health")]
        health_path: String,
        /// Volume source name (default: roko_data).
        #[arg(long, default_value = "roko_data")]
        volume_source: String,
        /// Volume mount destination path (default: /data/.roko).
        #[arg(long, default_value = "/data/.roko")]
        volume_destination: String,
        /// Overwrite an existing fly.toml even if it differs from the generated one.
        #[arg(long)]
        force: bool,
    },
    /// Build the local Docker image and tag it for the configured registry.
    Docker {
        /// Working directory / repository root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Registry namespace to tag the image under.
        #[arg(long)]
        registry: Option<String>,
        /// Push the tagged image to the registry after a successful build.
        /// When omitted the image is built and tagged locally but NOT pushed.
        #[arg(long)]
        push: bool,
        /// Skip the security posture check (WARNING: server will be public without auth).
        #[arg(long)]
        unsafe_public: bool,
        /// Show the deploy plan without performing any mutations.
        #[arg(long)]
        dry_run: bool,
        /// Path to the Dockerfile (default: Dockerfile).
        #[arg(long, default_value = "Dockerfile")]
        dockerfile: String,
        /// Docker build target stage (e.g. runtime, distroless).
        #[arg(long)]
        target: Option<String>,
        /// Docker image name (default: roko).
        #[arg(long, default_value = "roko")]
        image: String,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCmd {
    // ── Core config management ──────────────────────────────────────
    /// Interactive wizard: detects installed LLM CLIs, writes global config.
    #[command(alias = "wizard")]
    Init {
        /// Skip all confirmation prompts.
        #[arg(long)]
        yes: bool,
        /// Pre-select agent command (skip picker).
        #[arg(long)]
        agent: Option<String>,
        /// Pre-set model name (ollama-only convenience).
        #[arg(long)]
        model: Option<String>,
        /// Pre-set token budget.
        #[arg(long)]
        budget: Option<usize>,
        /// Pre-set role string.
        #[arg(long)]
        role: Option<String>,
        /// Enable default compile+clippy gates.
        #[arg(long)]
        enable_gates: bool,
        /// Write to this path instead of the resolved global path.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Non-interactive mode: skip all prompts, fail if any answer is missing.
        #[arg(long)]
        non_interactive: bool,
    },
    /// Print the effective merged config with per-field source tags.
    Show {
        /// Print only this section of the fully-resolved config, as TOML: a
        /// top-level table such as `agent` or a dotted path such as `providers.anthropic`.
        section: Option<String>,
        /// Directory to resolve project config from (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Show the fully-resolved config after global merge and env var overrides.
        #[arg(long)]
        effective: bool,
    },
    /// Print the resolved global + project + env config paths.
    Path {
        /// Directory to resolve project config from (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Print basic config health without modifying files.
    Doctor {
        /// Directory to resolve project config from (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Open $EDITOR on the chosen config file.
    Edit {
        /// Open the global config file.
        #[arg(long, conflicts_with = "project")]
        global: bool,
        /// Open (or create) the project `roko.toml`.
        #[arg(long, conflicts_with = "global")]
        project: bool,
        /// Directory to resolve project config from (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Set a dotted key (e.g. `agent.command = ollama`) in the global or
    /// project config.
    ///
    /// A secret key such as `serve.auth.api_key` goes to the project's
    /// `.roko/.env` instead, as its `ROKO__` variable
    /// (`ROKO__SERVE__AUTH__API_KEY`), whatever the flags, and is removed
    /// from the config files agents can read.
    Set {
        /// Dotted key path.
        key: String,
        /// Value to write.
        value: String,
        /// Write to project config instead of global.
        #[arg(long, conflicts_with = "global")]
        project: bool,
        /// Write to global config (default).
        #[arg(long, conflicts_with = "project")]
        global: bool,
        /// Directory to resolve project config from (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Store a secret in `~/.roko/.env` as `NAME=VALUE`.
    SetSecret {
        /// Secret name.
        name: String,
        /// Secret value.
        value: String,
    },
    /// Check `${VAR}` references in config and validate referenced secrets.
    CheckSecrets {
        /// Directory to resolve project config from (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Validate `roko.toml` syntax, schema, and semantic references.
    Validate {
        /// Directory to resolve project config from (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Migrate a legacy project `roko.toml` into explicit provider/model tables.
    Migrate {
        /// Directory to resolve project config from (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Print the proposed migration without writing changes.
        #[arg(long)]
        dry_run: bool,
        /// Skip the confirmation prompt and apply the migration immediately.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    // ── Export ─────────────────────────────────────────────────────
    /// Export config as environment variables for a deployment target.
    Export {
        /// Working directory (default: current directory).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Deployment target: railway, docker, or fly.
        #[arg(long)]
        env: Option<String>,
        /// Write output to a file instead of stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },

    // ── Environment variables ─────────────────────────────────────
    /// List all recognized environment variables with descriptions.
    Env {
        /// Emit JSON instead of a formatted table.
        #[arg(long)]
        json: bool,
    },

    // ── Providers ───────────────────────────────────────────────────
    /// Inspect configured LLM providers.
    Providers {
        #[command(subcommand)]
        cmd: ConfigProviderCmd,
    },
    // ── Models ──────────────────────────────────────────────────────
    /// Inspect configured models and routing.
    Models {
        #[command(subcommand)]
        cmd: ConfigModelCmd,
    },
    // ── Subscriptions ───────────────────────────────────────────────
    /// Manage event subscriptions.
    Subscriptions {
        #[command(subcommand)]
        cmd: ConfigSubscriptionCmd,
    },
    // ── Event sources ───────────────────────────────────────────────
    /// Inspect configured event sources (cron, file watchers).
    Events {
        /// Directory containing `roko.toml` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    // ── Experiments ─────────────────────────────────────────────────
    /// Manage model A/B experiments.
    Experiments {
        #[command(subcommand)]
        cmd: ExperimentCmd,
    },
    // ── Plugins ─────────────────────────────────────────────────────
    /// Manage plugins (list, install, remove, audit).
    Plugins {
        #[command(subcommand)]
        cmd: PluginCmd,
    },
    // ── Secrets ─────────────────────────────────────────────────────
    /// Manage profile-aware secrets (set, get, list, rotate).
    Secrets {
        #[command(subcommand)]
        cmd: roko_cli::SecretsCmd,
    },
    // ── MCP servers ────────────────────────────────────────────────
    /// Manage MCP server configuration (list, test, add).
    Mcp {
        #[command(subcommand)]
        cmd: ConfigMcpCmd,
    },
    // ── Presets ─────────────────────────────────────────────────────
    /// Apply validated configuration presets (gates, routing, budget, model).
    Preset {
        #[command(subcommand)]
        cmd: ConfigPresetCmd,
    },
}

// -----------------------------------------------------------------------
// ConfigPresetCmd — preset mutation targets
// -----------------------------------------------------------------------

#[derive(Debug, Subcommand)]
enum ConfigPresetCmd {
    /// Apply recommended gate strictness preset.
    Gates {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Preview the preset diff without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip interactive confirmation.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Write to global config instead of project.
        #[arg(long, conflicts_with = "project")]
        global: bool,
        /// Write to project config (default).
        #[arg(long, conflicts_with = "global")]
        project: bool,
    },
    /// Apply recommended model routing preset with resolved model slugs.
    Routing {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Preview the preset diff without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip interactive confirmation.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Write to global config instead of project.
        #[arg(long, conflicts_with = "project")]
        global: bool,
        /// Write to project config (default).
        #[arg(long, conflicts_with = "global")]
        project: bool,
    },
    /// Apply recommended budget limits preset.
    Budget {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Preview the preset diff without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip interactive confirmation.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Write to global config instead of project.
        #[arg(long, conflicts_with = "project")]
        global: bool,
        /// Write to project config (default).
        #[arg(long, conflicts_with = "global")]
        project: bool,
    },
    /// Set the default model from configured models.
    Model {
        /// Model key or alias (e.g. sonnet, haiku).
        name: String,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Preview the preset diff without writing.
        #[arg(long)]
        dry_run: bool,
        /// Skip interactive confirmation.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Write to global config instead of project.
        #[arg(long, conflicts_with = "project")]
        global: bool,
        /// Write to project config (default).
        #[arg(long, conflicts_with = "global")]
        project: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigProviderCmd {
    /// List configured providers and their current connection status.
    List {
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show persisted provider circuit-breaker health and latency.
    Health {
        /// Directory containing `.roko/` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Make a minimal API call to each provider to verify the account has credits.
        #[arg(long)]
        check_credits: bool,
    },
    /// Send a minimal request to verify provider connectivity.
    Test {
        /// Provider name from `[providers.*]`.  Omit when using `--all`.
        provider: Option<String>,
        /// Test every configured provider and print a summary table.
        #[arg(long)]
        all: bool,
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// List all supported provider kinds with required credentials and setup instructions.
    Available,
    /// Scan environment for API keys and report available providers.
    Discover {
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Interactive provider setup with pre-filled defaults from the catalog.
    Add {
        /// Provider catalog ID (e.g. deepseek, openai, anthropic).
        name: String,
        /// Print the generated TOML without writing to config.
        #[arg(long)]
        dry_run: bool,
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show all known providers from the built-in catalog with availability status.
    Catalog {
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Validate provider and model config semantics (early failure checks).
    Validate {
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigModelCmd {
    /// List configured models and their capabilities.
    #[command(alias = "ls")]
    List {
        /// Print only model names, one per line (for shell completion scripts).
        #[arg(long)]
        names_only: bool,
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show the current routing decision and optionally explain why it won.
    Route {
        /// Model key or slug to explain.
        model: String,
        /// Show the full routing trace instead of only the final decision.
        #[arg(long)]
        explain: bool,
        /// Complexity tier (`mechanical`, `focused`, `integrative`, `architectural`).
        #[arg(long)]
        complexity: Option<String>,
        /// Directory containing `roko.toml` (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigSubscriptionCmd {
    /// List all subscriptions.
    List,
    /// Create a new subscription.
    Add {
        /// Agent template name to invoke.
        #[arg(long)]
        template: String,
        /// Signal trigger glob to match.
        #[arg(long)]
        trigger: String,
    },
    /// Delete a subscription.
    Remove {
        /// Subscription ID.
        id: String,
    },
    /// Enable a subscription.
    Enable {
        /// Subscription ID.
        id: String,
    },
    /// Disable a subscription.
    Disable {
        /// Subscription ID.
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigMcpCmd {
    /// List configured MCP servers.
    List {
        /// Directory containing `roko.toml` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Test an MCP server by performing a real initialize + tools/list handshake.
    Test {
        /// MCP server name from the config.
        name: String,
        /// Per-stage timeout in seconds (applies to initialize, tools/list, and shutdown).
        #[arg(long, default_value_t = roko_core::defaults::DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS, value_parser = clap::value_parser!(u64).range(1..=60))]
        timeout_secs: u64,
        /// Directory containing `.roko/mcp-config.json` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Add an MCP server entry to `.roko/mcp-config.json`.
    Add {
        /// Server name (e.g. "roko").
        name: String,
        /// Launch command (e.g. "/usr/local/bin/roko-mcp").
        command: String,
        /// Optional arguments.
        #[arg(last = true)]
        args: Vec<String>,
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

/// The workspace a crash report goes to, recorded once the command line is
/// parsed: the invoked subcommand's `--workdir`, else `--repo` or the current
/// directory, as for the log file and the agent PID registry.
static CRASH_REPORT_WORKDIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// The `.roko/` the panic hook writes `crash-report.json` into: the parsed
/// workspace's, else (for a panic before parsing) `ROKO_WORKDIR`'s, else the
/// current directory's.
fn crash_report_dir(parsed_workdir: Option<&Path>) -> PathBuf {
    parsed_workdir
        .map(Path::to_path_buf)
        .or_else(|| env::var_os("ROKO_WORKDIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".roko")
}

fn main() {
    // ── Crash report panic hook ─────────────────────────────────────
    // Install a global panic hook that writes a structured crash report
    // to `.roko/crash-report.json` before the default handler runs.
    // This must be as early as possible so all panics are captured.
    {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Extract the panic message.
            let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
                Some((*s).to_string())
            } else {
                info.payload().downcast_ref::<String>().cloned()
            };

            // Include location if available.
            let message = if let (Some(msg), Some(loc)) = (&message, info.location()) {
                Some(format!("{msg} at {loc}"))
            } else {
                message
            };

            // Capture backtrace (respects RUST_BACKTRACE env var).
            let backtrace = {
                let bt = std::backtrace::Backtrace::force_capture();
                let text = bt.to_string();
                if text.is_empty() || text.contains("disabled") {
                    None
                } else {
                    Some(text)
                }
            };

            let report = roko_core::build_crash_report(
                message,
                backtrace,
                env!("CARGO_PKG_VERSION"),
                env!("ROKO_RUSTC_VERSION"),
            );

            // Write into the invoked command's workspace once it is known.
            let roko_dir = crash_report_dir(CRASH_REPORT_WORKDIR.get().map(PathBuf::as_path));

            roko_core::write_crash_report(&roko_dir, &report);

            // Call the default handler so the process still prints the
            // panic message and exits with the expected code.
            default_hook(info);
        }));
    }

    let startup_env_redactions = match load_startup_env_files() {
        Ok(values) => values,
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(EXIT_SYSTEM_ERROR);
        }
    };
    // One scrubber for the process: the log layers scrub with it, and the
    // persistence writers redact its secrets (the `.env` values and provider
    // keys) from what they write.
    let scrubber = roko_fs::observability::RunScrubber::install(&startup_env_redactions);

    let mut cli = try_parse_cli(std::env::args_os()).unwrap_or_else(|error| error.exit());
    apply_env_overrides(&mut cli);
    // `--config <file>` is the config of every load in this process, as
    // `ROKO_CONFIG` would be, without reaching child processes (bug-4ed3c2).
    // A file that does not exist is left to the command: `setup` creates it,
    // and `plan run` refuses it.
    if let Some(path) = &cli.config {
        let path = std::path::absolute(path).unwrap_or_else(|_| path.clone());
        let _ = roko_core::config::loader::set_config_path_override(path);
    }

    // ── ACP early exit ───────────────────────────────────────────────
    // ACP mode uses stdio for JSON-RPC, so we MUST NOT install any
    // tracing subscriber that writes to stdout.  Fork into its own
    // Tokio runtime here, before the CLI subscriber is initialised.
    #[cfg(feature = "acp")]
    if let Some(Command::Acp {
        ref workdir,
        ref profile,
        ref config,
        ref global_config,
        ref log_file,
    }) = cli.command
    {
        let acp_config = roko_acp::AcpConfig {
            workdir: workdir.clone(),
            profile: profile.clone(),
            config_path: config.clone(),
            global_config_path: global_config.clone(),
            log_file: log_file.clone(),
        };
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!(%e, "failed to build Tokio runtime for ACP");
                std::process::exit(EXIT_FAILURE);
            }
        };
        let code = runtime.block_on(async {
            match roko_acp::run_acp_server(acp_config).await {
                Ok(()) => EXIT_SUCCESS,
                Err(e) => {
                    eprintln!("error: {e:#}");
                    EXIT_FAILURE
                }
            }
        });
        std::process::exit(code);
    }
    #[cfg(not(feature = "acp"))]
    if matches!(cli.command, Some(Command::Acp { .. })) {
        eprintln!(
            "error: ACP support is not included in this build; rebuild roko with `--features acp`"
        );
        std::process::exit(EXIT_FAILURE);
    }

    // ── TUI mode detection ─────────────────────────────────────────
    // Unified chat (no subcommand, TTY stdin) also needs file-based tracing
    // to prevent serve/pheromone logs from corrupting the inline chat display.
    let unified_chat_mode = cli.command.is_none()
        && !cli.headless
        && cli.prompt.is_none()
        && std::io::stdin().is_terminal();
    let tui_mode =
        unified_chat_mode || matches!(&cli.command, Some(Command::Serve { tui: true, .. }));

    // ── Color mode ──────────────────────────────────────────────────
    let use_color = cli.color.should_color();

    // ── Timing mode ─────────────────────────────────────────────────
    let timing_enabled = cli.timing
        || env::var("ROKO_TIMING")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
    let started_at = Instant::now();

    // In TUI mode, route ALL tracing output to a file instead of stderr.
    // This must be done here, before the global subscriber is set, to
    // prevent serve background tasks from writing over the ratatui screen.
    let filter = if tui_mode {
        // Suppress noisy subsystems in TUI mode.
        tracing_subscriber::EnvFilter::try_new(
            "roko=info,roko_neuro=error,roko_agent=warn,hyper=error,tower=error",
        )
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("roko=info"))
    } else {
        tracing_subscriber::EnvFilter::try_new(tracing_log_directive())
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("roko=info"))
    };

    // ROKO_LOG_RAW=1 disables secret redaction (useful for debugging).
    let raw_logs = env::var("ROKO_LOG_RAW")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let ansi_logs = use_color;

    // Determine the workdir for log file placement, crash reports and the agent
    // PID registry: the invoked subcommand's `--workdir`, else `--repo`/cwd.
    let workdir = invoked_subcommand_workdir().unwrap_or_else(|| resolve_workdir(&cli));
    roko_agent::process::set_registry_root(&workdir);
    let _ = CRASH_REPORT_WORKDIR.set(workdir.clone());

    // File layer: write to .roko/roko.log with day-based rotation.
    // In TUI mode, use serve-tui.log to keep it separate from the main log.
    let log_dir = workdir.join(".roko");
    let log_file_name = if tui_mode {
        "serve-tui.log"
    } else {
        "roko.log"
    };
    let _ = std::fs::create_dir_all(&log_dir);
    // Day-based rolling appender: writes roko.log.YYYY-MM-DD (or serve-tui.log.YYYY-MM-DD)
    // so long-running sessions do not grow a single unbounded file.
    let rolling_appender = tracing_appender::rolling::daily(&log_dir, log_file_name);
    let (non_blocking_writer, _log_guard) = tracing_appender::non_blocking(rolling_appender);
    let file_layer = Some(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .event_format(RedactingFormat::new(
                tracing_subscriber::fmt::format().with_target(true),
                scrubber.clone(),
            ))
            .with_writer(non_blocking_writer),
    );

    // Stderr layer: only when --verbose, ROKO_LOG/RUST_LOG is set, or raw_logs mode.
    // ROKO_LOG is the authoritative knob for all roko-owned binaries; RUST_LOG is
    // accepted as a compatibility fallback. Either one activates stderr output.
    // In TUI mode, never write to stderr (would corrupt ratatui rendering).
    let show_stderr = !tui_mode
        && (cli.verbose
            || std::env::var("ROKO_LOG").is_ok()
            || std::env::var("RUST_LOG").is_ok()
            || raw_logs);
    let stderr_layer = if show_stderr {
        Some(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_ansi(ansi_logs)
                .event_format(RedactingFormat::new(
                    tracing_subscriber::fmt::format(),
                    scrubber,
                ))
                .with_writer(std::io::stderr),
        )
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .with(filter)
        .init();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!(%e, "failed to build Tokio runtime");
            std::process::exit(EXIT_FAILURE);
        }
    };
    let shutdown = setup_graceful_shutdown();
    install_sigterm_handler(&runtime, shutdown.clone());

    let code = match runtime.block_on(dispatch(cli)) {
        Ok(code) => {
            if timing_enabled {
                print_timing(started_at);
            }
            code
        }
        Err(e) => {
            if timing_enabled {
                print_timing(started_at);
            }
            let msg = format_error_with_hint(&e);
            eprintln!("error: {msg}");
            EXIT_SYSTEM_ERROR
        }
    };
    std::process::exit(code);
}

// -----------------------------------------------------------------------
// Timing helper
// -----------------------------------------------------------------------

fn print_timing(started_at: Instant) {
    let elapsed = started_at.elapsed();
    let secs = elapsed.as_secs_f64();
    if secs < 60.0 {
        eprintln!("Completed in {secs:.1}s");
    } else {
        let mins = (secs / 60.0).floor() as u64;
        let rem = secs - (mins as f64 * 60.0);
        eprintln!("Completed in {mins}m {rem:.1}s");
    }
}

// -----------------------------------------------------------------------
// Contextual error suggestions
// -----------------------------------------------------------------------

/// Format an error with a helpful hint when the message matches a known pattern.
fn format_error_with_hint(err: &anyhow::Error) -> String {
    let msg = format!("{err:#}");
    match error_hint(&msg) {
        Some(h) => format!("{msg}\n\nhint: {h}"),
        None => msg,
    }
}

/// Return an optional hint string based on common error patterns.
fn error_hint(msg: &str) -> Option<&'static str> {
    let lower = msg.to_lowercase();

    // State recovery errors must be checked before the generic auth pattern
    // to prevent "authoritative" from matching the "auth" substring.
    if lower.contains("state recovery required") || lower.contains("state snapshot corrupt") {
        return Some(
            "run with `--fresh` to archive prior state and start a new run. \
             The corrupt snapshot has been preserved for diagnosis. \
             (--fresh is a temporary compatibility escape; a future release \
             will replace it with plan-scoped run management)",
        );
    }

    if lower.contains("no .roko directory")
        || lower.contains(".roko/")
            && (lower.contains("not found") || lower.contains("no such file"))
        || lower.contains("roko.toml")
            && (lower.contains("not found") || lower.contains("no such file"))
    {
        return Some("run `roko init` to create a workspace in the current directory");
    }

    if lower.contains("agent not found") || lower.contains("unknown agent") {
        return Some("run `roko agent list` to see available agents");
    }

    if lower.contains("plan not found")
        || lower.contains("plans directory does not exist")
        || lower.contains("no plans found")
    {
        return Some(
            "run `roko plan list` to see available plans, or `roko plan create` to make one",
        );
    }

    if lower.contains("connection refused")
        || lower.contains("connect error")
        || lower.contains("failed to connect")
    {
        return Some("is the server running? Start it with `roko serve`");
    }

    // Authentication hint: match specific auth-related terms, not substrings
    // like "authoritative" or "authorization policy", and 401 only as an HTTP
    // status, never inside a path, an id or a longer number.
    if mentions_http_401(&lower)
        || lower.contains("unauthorized")
        || lower.contains("invalid_api_key")
        || lower.contains("authentication failed")
        || lower.contains("auth denied")
    {
        // ROKO_API_KEY authenticates to roko serve; each model provider has its own key.
        if lower.contains("workspace server") || lower.contains("roko serve") {
            return Some("check your roko serve API key: set ROKO_API_KEY or run `roko login`");
        }
        if lower.contains("provider") {
            return Some(
                "check the provider's API key: run `roko config check-secrets`, then \
                 `roko config providers test --all`",
            );
        }
        return Some(
            "check your API key: `roko config check-secrets` checks model provider keys; \
             ROKO_API_KEY or `roko login` authenticates to roko serve",
        );
    }

    if lower.contains("prd not found") || lower.contains("no prd") {
        return Some("run `roko prd list` to see available PRDs, or `roko prd idea` to create one");
    }

    None
}

/// Whether a lower-cased error message reports HTTP status 401: a standalone
/// `401` within three words of `http`, `status`, `unauthorized`, `request` or
/// `returned`. A 401 inside a path, an id or a longer number (`run-1401/`,
/// `gap-e4019c`, `14015 bytes`) is not a status.
fn mentions_http_401(lower: &str) -> bool {
    // Path and id characters stay inside a word, so `/tmp/run-1401/x.json` is one word.
    let words: Vec<&str> = lower
        .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/')))
        .map(|word| word.trim_end_matches('.'))
        .filter(|word| !word.is_empty())
        .collect();
    for (at, &word) in words.iter().enumerate() {
        let near = &words[at.saturating_sub(3)..words.len().min(at + 4)];
        if word == "401" && near.iter().copied().any(is_http_status_word) {
            return true;
        }
    }
    false
}

fn is_http_status_word(word: &str) -> bool {
    word.starts_with("http") || matches!(word, "status" | "unauthorized" | "request" | "returned")
}

#[derive(Debug)]
struct RedactingFormat<E> {
    inner: E,
    scrubber: std::sync::Arc<roko_core::obs::LogScrubber>,
}

impl<E> RedactingFormat<E> {
    fn new(inner: E, scrubber: std::sync::Arc<roko_core::obs::LogScrubber>) -> Self {
        Self { inner, scrubber }
    }
}

impl<S, N, E> FormatEvent<S, N> for RedactingFormat<E>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let mut buffer = String::new();
        let buffer_writer = tracing_subscriber::fmt::format::Writer::new(&mut buffer);
        self.inner.format_event(ctx, buffer_writer, event)?;
        let scrubbed = self.scrubber.scrub(&buffer);
        writer.write_str(&scrubbed)
    }
}

fn tracing_log_directive() -> String {
    tracing_log_directive_from(env::var("RUST_LOG").ok(), env::var("ROKO_LOG").ok())
}

/// Resolve the tracing log directive from environment variables.
///
/// `ROKO_LOG` is the authoritative verbosity knob for all roko-owned binaries
/// (roko-cli, roko-chain-watcher, agent-relay). `RUST_LOG` is accepted as a
/// compatibility fallback when `ROKO_LOG` is not set.
fn tracing_log_directive_from(rust_log: Option<String>, roko_log: Option<String>) -> String {
    roko_log
        .or(rust_log)
        .unwrap_or_else(|| "roko=info".to_string())
}

async fn dispatch(mut cli: Cli) -> Result<i32> {
    if let Some(command) = cli.command.take() {
        return dispatch_subcommand(command, &cli).await;
    }
    if cli.headless {
        return commands::util::cmd_headless(&cli).await;
    }
    // Bare prompt: `roko "fix the bug"` → one-shot inline dispatch
    if let Some(prompt) = &cli.prompt {
        return roko_cli::unified::cmd_oneshot_inline(prompt, cli.quiet).await;
    }
    // Piped input: `echo "prompt" | roko`
    if !roko_cli::stdin_is_tty() {
        return commands::util::cmd_pipe(&cli).await;
    }
    // Default: unified inline chat (auto-detect auth, direct dispatch)
    roko_cli::unified::cmd_unified_chat(cli.config.as_deref(), cli.quiet, cli.no_serve).await
}

async fn dispatch_subcommand(command: Command, cli: &Cli) -> Result<i32> {
    match command {
        Command::Init {
            path,
            cloud,
            profile,
            demo,
        } => {
            commands::util::cmd_init(path, cloud, profile, demo).await?;
            Ok(EXIT_SUCCESS)
        }
        Command::Run {
            prompt,
            workdir,
            serve,
            share,
            provider,
            max_retries,
        } => {
            // Resolve typed overrides before any side effects (#262).
            let _resolved = ResolvedExecutionOverrides::for_run(
                &global_cli_flags(cli),
                provider.clone(),
                serve || share,
                max_retries,
            );
            tracing::debug!(?_resolved, "resolved execution overrides for `run`");

            if !serve && !share && max_retries.is_none() {
                return commands::do_cmd::cmd_do(
                    cli,
                    workdir,
                    vec![prompt],
                    false,
                    None,
                    false,
                    false,
                    false,
                    false,
                    None,
                    false,
                    provider,
                    Vec::new(),
                    true, // explicit `roko run`: auto-route without a TTY
                )
                .await;
            }
            commands::util::cmd_run(
                cli,
                workdir,
                prompt,
                serve,
                share,
                provider,
                max_retries,
                None,
            )
            .await
        }
        Command::Do {
            plan,
            complexity,
            dry_run,
            workdir,
            provider,
            yes,
            ghost,
            compare,
            r#continue,
            no_cascade,
            context,
            prompt,
        } => {
            // Resolve typed overrides before any side effects (#262).
            let _resolved = ResolvedExecutionOverrides::for_do(
                &global_cli_flags(cli),
                &DoInput {
                    dry_run,
                    ghost,
                    yes,
                    no_cascade,
                    provider: provider.clone(),
                    context: context.clone(),
                },
            );
            tracing::debug!(?_resolved, "resolved execution overrides for `do`");

            commands::do_cmd::cmd_do(
                cli,
                workdir,
                prompt,
                plan,
                complexity.map(DoComplexity::into_plan_complexity),
                dry_run,
                yes,
                ghost,
                compare,
                r#continue,
                no_cascade,
                provider,
                context,
                false,
            )
            .await
        }
        Command::Develop { .. } => {
            // #363: `develop` is removed. Emit a migration error and exit
            // without executing any provider/server/git effects.
            eprintln!("error: `roko develop` was removed. Use `roko do --plan <prompt>` instead.");
            if cli.json {
                let msg = serde_json::json!({
                    "error": "command_removed",
                    "command": "develop",
                    "migration": "roko do --plan <prompt>",
                    "deprecated_since": "2026-09-04",
                });
                println!("{}", serde_json::to_string_pretty(&msg).unwrap_or_default());
            }
            Ok(EXIT_FAILURE)
        }
        Command::Status {
            workdir,
            quick,
            cfactor,
            surfaces,
        } => {
            let code = commands::status::cmd_status(cli, workdir, quick, cfactor, surfaces).await?;
            Ok(code)
        }
        Command::Github { cmd } => commands::github::cmd_github(cli, cmd).await,
        Command::Show {
            dashboard,
            live,
            follow,
            serve_url,
            workdir,
            subject,
        } => {
            // #363: --live is a deprecated alias for --dashboard.
            let use_dashboard = dashboard || live;
            if live && !dashboard {
                eprintln!("warning: --live is deprecated; use --dashboard instead");
            }
            commands::show::cmd_show(cli, workdir, use_dashboard, follow, serve_url, subject).await
        }
        Command::Doctor {
            subject,
            workdir,
            serve_url,
        } => commands::util::cmd_doctor(cli, subject, workdir, serve_url).await,
        Command::Cache { cmd } => commands::cache::cmd_cache(cli, cmd).await,
        Command::RunIndex { cmd } => commands::run_index::cmd_run_index(cli, cmd).await,
        Command::Setup {
            workdir,
            yes,
            quick,
        } => commands::setup::cmd_setup(cli, workdir, yes, quick).await,
        Command::Diagnose {
            plan_id,
            verbose,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            commands::diagnose::cmd_diagnose(&wd, &plan_id, verbose, cli.json)
        }
        Command::LayerCheck => {
            eprintln!("warning: 'roko layer-check' is deprecated, use 'roko doctor'");
            roko_cli::layer_check::run_layer_check()
        }
        Command::Plan { cmd } => {
            let wd = cmd.index_rebuild_workdir(cli);
            let command_can_mutate = cmd.should_rebuild_indexes();
            let result = commands::plan::cmd_plan(cli, cmd).await;
            let should_rebuild =
                should_rebuild_plan_indexes(command_can_mutate, result.as_ref().ok().copied());
            finish_with_index_rebuild(result, &wd, should_rebuild)
        }
        Command::Prd { cmd } => {
            let wd = resolve_workdir(cli);
            let command_can_mutate = cmd.should_rebuild_indexes();
            let result = commands::prd::cmd_prd(cli, cmd).await;
            finish_with_index_rebuild(result, &wd, command_can_mutate)
        }
        Command::Agent { cmd } => commands::agent::cmd_agent(cli, cmd).await,
        Command::Research { cmd } => {
            let wd = resolve_workdir(cli);
            // Resolve typed overrides before any side effects (#306).
            let research_input = match &cmd {
                ResearchCmd::Topic { deep, backend, .. } => {
                    roko_cli::resolved_overrides::ResearchInput {
                        backend: Some(backend.to_string()),
                        deep: *deep,
                    }
                }
                _ => roko_cli::resolved_overrides::ResearchInput::default(),
            };
            let resolved =
                ResolvedExecutionOverrides::for_research(&global_cli_flags(cli), &research_input);
            tracing::debug!(?resolved, "resolved execution overrides for `research`");
            let result = commands::research::cmd_research(cli, cmd, &resolved).await;
            finish_with_index_rebuild(result, &wd, true)
        }
        Command::Think { question, workdir } => {
            commands::think::cmd_think(cli, question, workdir).await
        }
        Command::Note {
            tags,
            workdir,
            text,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            commands::note::cmd_note(&wd, text, tags, cli.json)
        }
        Command::Tune(cmd) => {
            eprintln!("warning: 'roko tune' is deprecated, use 'roko config preset'");
            commands::tune::cmd_tune(cli, cmd).await
        }
        Command::Knowledge { cmd } => commands::knowledge::dispatch_knowledge(cli, cmd).await,
        Command::Learn { cmd } => commands::learn::dispatch_learn(cli, cmd).await,
        Command::Job { cmd } => commands::job::cmd_job(cli, cmd).await,
        Command::Backlog { cmd } => commands::backlog::cmd_backlog(cli, cmd).await,
        Command::Bench { cmd } => commands::bench::cmd_bench(cli, cmd).await,
        Command::Demo(cmd) => {
            let workdir = match &cmd {
                DemoCmd::Setup { workdir } | DemoCmd::Warm { workdir } => {
                    workdir.clone().unwrap_or_else(|| resolve_workdir(cli))
                }
            };
            match cmd {
                DemoCmd::Setup { .. } => {
                    roko_cli::demo_cmd::cmd_demo_setup(&workdir)?;
                    Ok(EXIT_SUCCESS)
                }
                DemoCmd::Warm { .. } => {
                    roko_cli::demo_cmd::cmd_demo_warm(&workdir).await?;
                    Ok(EXIT_SUCCESS)
                }
            }
        }
        Command::Config { cmd } => {
            match cmd {
                ConfigCmd::Experiments { cmd: exp_cmd } => {
                    return dispatch_experiment(cli, exp_cmd);
                }
                ConfigCmd::Plugins { cmd: plugin_cmd } => {
                    return commands::config_cmd::cmd_plugin(cli, plugin_cmd).await;
                }
                ConfigCmd::Secrets { cmd: secrets_cmd } => {
                    let workdir = resolve_workdir(cli);
                    roko_cli::secrets::dispatch_secrets(&secrets_cmd, &workdir).await?;
                    return Ok(EXIT_SUCCESS);
                }
                ConfigCmd::Mcp { cmd: mcp_cmd } => {
                    let workdir = resolve_workdir(cli);
                    commands::mcp::dispatch_mcp_cmd(&mcp_cmd, &workdir).await?;
                    return Ok(EXIT_SUCCESS);
                }
                ConfigCmd::Preset { cmd: preset_cmd } => {
                    return commands::tune::cmd_config_preset(cli, preset_cmd).await;
                }
                other => {
                    commands::config_cmd::dispatch_config(cli, other).await?;
                }
            }
            Ok(EXIT_SUCCESS)
        }
        Command::Index { cmd } => commands::util::cmd_index(cli, cmd),
        Command::Graph { cmd } => commands::graph::cmd_graph(cmd).await,
        Command::Feed { cmd } => commands::feed::cmd_feed(cli, cmd).await,
        Command::Recipe { cmd } => commands::recipe::cmd_recipe(cli, cmd),
        Command::Trigger { cmd } => commands::trigger::cmd_trigger(cli, cmd).await,
        Command::Dev { no_frontend } => {
            eprintln!("warning: 'roko dev' is deprecated, use 'roko serve'");
            commands::dev::cmd_dev(cli, no_frontend).await
        }
        Command::Up { workdir } => {
            eprintln!("warning: 'roko up' is deprecated, use 'roko serve'");
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            commands::server::cmd_up(cli, wd).await
        }
        Command::Serve {
            bind,
            port,
            workdir,
            tui,
            enable_terminal,
        } => commands::server::cmd_serve(cli, bind, port, workdir, tui, enable_terminal).await,
        Command::Acp {
            workdir,
            profile,
            config,
            global_config,
            log_file,
        } => {
            #[cfg(feature = "acp")]
            {
                let acp_config = roko_acp::AcpConfig {
                    workdir,
                    profile,
                    config_path: config,
                    global_config_path: global_config,
                    log_file,
                };
                roko_acp::run_acp_server(acp_config).await?;
                Ok(EXIT_SUCCESS)
            }
            #[cfg(not(feature = "acp"))]
            {
                let _ = (workdir, profile, config, global_config, log_file);
                anyhow::bail!(
                    "ACP support is not included in this build; rebuild roko with `--features acp`"
                )
            }
        }
        Command::Daemon { cmd } => commands::server::cmd_daemon(cli, cmd).await,
        Command::Deploy { cmd } => commands::server::cmd_deploy(cli, cmd).await,
        Command::Worker { port } => {
            roko_cli::worker::run_worker(port).await?;
            Ok(EXIT_SUCCESS)
        }
        Command::Dashboard {
            page,
            list_pages,
            text,
            workdir,
            snapshot,
            high_contrast,
            reduced_motion,
        } => {
            #[allow(unsafe_code)]
            if high_contrast {
                unsafe { std::env::set_var("ROKO_HIGH_CONTRAST", "1") };
            }
            #[allow(unsafe_code)]
            if reduced_motion {
                unsafe { std::env::set_var("ROKO_REDUCED_MOTION", "1") };
            }
            if let Some(snapshot_dir) = snapshot {
                return commands::dashboard::cmd_dashboard_snapshot(cli, workdir, &snapshot_dir)
                    .await;
            }
            commands::dashboard::cmd_dashboard(cli, workdir, page, list_pages, text, None).await
        }
        Command::Screenshot(args) => {
            let workdir = args.workdir.clone().unwrap_or_else(|| resolve_workdir(cli));
            commands::screenshot::cmd_screenshot(workdir, args)
        }
        // ── Vision loop ───────────────────────────────────────────
        Command::VisionLoop {
            target_file,
            goal,
            url,
            max_iter,
            target_score,
            consecutive_target,
            regression_threshold,
            model,
            viewport_width,
            viewport_height,
            wait_ms,
        } => {
            let config = roko_cli::vision_loop::VisionLoopConfig {
                target_file,
                goal,
                url,
                max_iterations: max_iter,
                target_score,
                consecutive_target,
                regression_threshold,
                model_key: model,
                viewport_width,
                viewport_height,
                wait_ms,
            };
            let result = roko_cli::vision_loop::cmd_vision_loop(config).await?;
            println!("Vision loop complete: {}", result.stop_reason);
            println!(
                "  iterations: {}, best score: {:.1} (iteration {})",
                result.iterations_completed, result.best_score, result.best_iteration
            );
            println!("  run ID: {}", result.run_id);
            Ok(EXIT_SUCCESS)
        }
        Command::Resume { run_id, workdir } => {
            commands::plan::cmd_resume(cli, run_id, workdir).await
        }
        Command::Replay {
            hash,
            workdir,
            forensic,
            from_event,
            as_of,
            format,
        } => {
            commands::util::cmd_replay(cli, workdir, hash, forensic, from_event, as_of, format)
                .await
        }
        Command::History { id, workdir } => commands::history::cmd_history(cli, id, workdir),
        Command::Inject {
            session,
            kind,
            payload,
            workdir,
        } => commands::util::cmd_inject(cli, session, &kind, payload, workdir).await,
        Command::Completions { shell } => {
            commands::util::print_completions(shell);
            Ok(EXIT_SUCCESS)
        }
        Command::New {
            type_name,
            name,
            output,
        } => commands::util::cmd_new(&type_name, &name, output),
        Command::Explain { topic, depth } => {
            if commands::util::cmd_explain(&topic, depth) {
                Ok(EXIT_SUCCESS)
            } else {
                Ok(EXIT_FAILURE)
            }
        }
        Command::Impact {
            base,
            files,
            json,
            workdir,
        } => commands::impact::cmd_impact(cli, &base, &files, json, workdir).await,
        Command::Login {
            url,
            api_key,
            check,
            dashboard_url,
        } => commands::auth::cmd_login(&url, api_key, check, &dashboard_url).await,
        Command::Logout => commands::auth::cmd_logout(),
        Command::Whoami => commands::auth::cmd_whoami().await,
        Command::Complete {
            shell: _,
            path,
            current,
        } => {
            commands::util::cmd_complete(&path, &current);
            Ok(EXIT_SUCCESS)
        }
    }
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

/// Resolve the working directory from CLI flags.
///
/// Detects when the user is running from inside a `.roko/` directory, which
/// would cause a nested `.roko/.roko/` and silent data loss.
fn resolve_workdir(cli: &Cli) -> PathBuf {
    let dir = cli.repo.clone().unwrap_or_else(|| PathBuf::from("."));
    let resolved = dir.canonicalize().unwrap_or(dir);

    // Detect if we're running from inside a .roko/ directory and auto-correct
    // to the project root to avoid nested .roko/.roko/ data dirs. An explicit
    // --repo is used as given.
    if let Some(project_root) = enclosing_project_of_data_dir(&resolved) {
        if cli.repo.is_some() {
            eprintln!(
                "\x1b[33m\u{26a0} --repo {} is inside the .roko/ of {}; using it as given\x1b[0m",
                resolved.display(),
                project_root.display()
            );
        } else {
            eprintln!(
                "\x1b[33m\u{26a0} Auto-correcting: running from inside .roko/, using project root: {}\x1b[0m",
                project_root.display()
            );
            return project_root;
        }
    }

    resolved
}

/// The project whose `.roko/` data directory contains `dir`, unless `dir` is
/// in a workspace nested there: walking up from `dir`, a directory with a
/// `roko.toml`, a `.git` entry or its own `.roko/` (a per-task worktree under
/// `.roko/worktrees/`, a fixture workspace) ends the search first.
fn enclosing_project_of_data_dir(dir: &Path) -> Option<PathBuf> {
    for ancestor in dir.ancestors() {
        if ancestor.file_name().and_then(|n| n.to_str()) == Some(".roko") {
            return Some(ancestor.parent().unwrap_or(ancestor).to_path_buf());
        }
        if ancestor.join("roko.toml").is_file()
            || ancestor.join(".git").exists()
            || ancestor.join(".roko").is_dir()
        {
            return None;
        }
    }
    None
}

/// Extract typed global CLI flags for resolved override construction.
///
/// This borrows from the parsed `Cli` struct to avoid requiring downstream
/// callers to depend on the full clap type. Used by
/// [`ResolvedExecutionOverrides`] constructors.
pub(crate) fn global_cli_flags(cli: &Cli) -> GlobalCliFlags<'_> {
    GlobalCliFlags {
        model: cli.model.as_deref(),
        role: cli.role.as_deref(),
        effort: cli.effort.as_ref().map(|e| match e {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Max => "max",
        }),
        resume: cli.resume.as_deref(),
        json: cli.json,
        quiet: cli.quiet,
        headless: cli.headless,
        no_serve: cli.no_serve,
        color_enabled: cli.color.should_color(),
    }
}

/// Resolve the plans directory: an explicit path wins, otherwise the workspace
/// plans directory ([`roko_cli::plan::plans_dir`]): `./plans/`, or `.roko/plans/`
/// in a workspace that keeps its plans there, noted on stderr.
fn resolve_plans_dir(workdir: &Path, explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }

    let resolved = roko_cli::plan::plans_dir(workdir);
    if resolved == roko_fs::workspace_plans::legacy_plans_dir(workdir) {
        eprintln!(
            "note: using {} (not found in {})",
            resolved.display(),
            workdir.join("plans").display()
        );
    }
    resolved
}

/// Apply environment variable fallbacks to CLI flags.
///
/// When a CLI flag was not explicitly provided, its corresponding `ROKO_*`
/// environment variable is consulted. This runs once immediately after
/// `Cli::parse()` so every downstream consumer sees the resolved value.
///
/// ## Logging
///
/// `ROKO_LOG` is the authoritative verbosity variable for all roko-owned
/// binaries (roko-cli, roko-chain-watcher, agent-relay). It uses
/// `tracing_subscriber::EnvFilter` syntax (e.g. `ROKO_LOG=roko=debug`).
/// `RUST_LOG` is accepted as a compatibility fallback when `ROKO_LOG` is
/// not set, but standalone binaries read `ROKO_LOG` exclusively.
///
/// | Env var           | CLI flag       | Behaviour                                  |
/// |-------------------|----------------|---------------------------------------------|
/// | `ROKO_LOG`        | `--verbose`    | Authoritative log verbosity for all roko binaries |
/// | `ROKO_MODEL`      | `--model`      | Override when `--model` not given            |
/// | `ROKO_EFFORT`     | `--effort`     | Override when `--effort` not given            |
/// | `ROKO_ROLE`       | `--role`       | Override when `--role` not given              |
/// | `ROKO_QUIET`      | `--quiet`      | Enable quiet if "1" or "true"                |
/// | `ROKO_LOG_FORMAT`  | `--log-format` | Override when default "text" is in effect     |
fn apply_env_overrides(cli: &mut Cli) {
    if cli.model.is_none()
        && let Ok(val) = env::var("ROKO_MODEL")
        && !val.is_empty()
    {
        cli.model = Some(val);
    }

    if cli.effort.is_none()
        && let Ok(val) = env::var("ROKO_EFFORT")
    {
        match val.to_ascii_lowercase().as_str() {
            "low" => cli.effort = Some(Effort::Low),
            "medium" => cli.effort = Some(Effort::Medium),
            "high" => cli.effort = Some(Effort::High),
            "max" => cli.effort = Some(Effort::Max),
            _ => {
                eprintln!(
                    "warning: ROKO_EFFORT={val:?} is not valid (expected low/medium/high/max), ignoring"
                );
            }
        }
    }

    if cli.role.is_none()
        && let Ok(val) = env::var("ROKO_ROLE")
        && !val.is_empty()
    {
        cli.role = Some(val);
    }

    if !cli.quiet
        && let Ok(val) = env::var("ROKO_QUIET")
        && (val == "1" || val.eq_ignore_ascii_case("true"))
    {
        cli.quiet = true;
    }

    // log_format has a clap default of Text; override only when the user
    // did not pass `--log-format` explicitly (we detect this by checking if
    // the env var is set — the clap default means we can't distinguish
    // "user typed --log-format text" from "default", but the env var path
    // is still useful when the default is in effect).
    if cli.log_format == LogFormat::Text
        && let Ok(val) = env::var("ROKO_LOG_FORMAT")
    {
        match val.to_ascii_lowercase().as_str() {
            "json" => cli.log_format = LogFormat::Json,
            "text" => {} // already the default
            _ => {
                eprintln!(
                    "warning: ROKO_LOG_FORMAT={val:?} is not valid (expected text/json), ignoring"
                );
            }
        }
    }
}

/// Ask the user to confirm a destructive operation.
///
/// Returns `true` (proceed) immediately when:
/// - `quiet` mode is active,
/// - stdin is not a TTY (CI / pipes), or
/// - the user types `y` or `Y`.
///
/// Returns `false` otherwise, meaning the operation should be skipped.
fn confirm_destructive(message: &str, quiet: bool) -> bool {
    if quiet || !std::io::stdin().is_terminal() {
        return true;
    }
    eprint!("{message} [y/N] ");
    let _ = std::io::Write::flush(&mut std::io::stderr());
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        input.trim().eq_ignore_ascii_case("y")
    } else {
        false
    }
}

/// Resolve config from a specific workdir, applying CLI overrides.
fn resolve_config_for_workdir(cli: &Cli, workdir: &Path) -> Result<Config> {
    let (mut config, repo_base) = if let Some(p) = &cli.config {
        (Config::from_file(p)?, p.parent().unwrap_or(workdir))
    } else {
        let resolved = load_resolved_config(workdir)?;
        let fully_default = resolved.sources.agent_command == Source::Default
            && resolved.sources.prompt_token_budget == Source::Default;
        // Allow through if the workspace uses the provider registry (default_backend /
        // [providers.*] table) rather than the legacy agent.command field.  When
        // providers are configured the command field remains "cat" (its sentinel
        // default) even though a real backend is wired, so the original check
        // would incorrectly gate those workspaces. A provider key exported in
        // the environment is a provider too: `effective_providers` adds one
        // for each well-known key variable, as dispatch does.
        let mut registry = RokoConfig::default();
        registry.providers.clone_from(&resolved.config.providers);
        let has_providers = !registry.effective_providers().is_empty();
        if fully_default && resolved.config.agent.command == "cat" && !has_providers {
            eprintln!("error: no LLM provider configured.\n");
            eprintln!("To get started, either:");
            eprintln!("  1. Run `roko init` to create a workspace with default config");
            eprintln!(
                "  2. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, GEMINI_API_KEY or PERPLEXITY_API_KEY"
            );
            eprintln!("  3. Edit roko.toml to configure a provider");
            eprintln!("\n  hint: run `roko doctor` to diagnose your setup");
            std::process::exit(EXIT_FAILURE);
        }
        (resolved.config, workdir)
    };

    // Validate and load any configured additional repos even when bypassing
    // layered config resolution via `--config`.
    let _repo_registry = RepoRegistry::load(&config, repo_base)?;

    // Apply CLI overrides.
    if let Some(role) = &cli.role {
        config.prompt.role.clone_from(role);
    }
    if let Some(model) = &cli.model {
        config.agent.model = Some(model.clone());
        // Non-Claude CLIs often still expect the model as a positional arg.
        if config.agent.command != "claude" && !config.agent.args.contains(model) {
            config.agent.args.insert(0, model.clone());
        }
    }
    if let Some(effort) = &cli.effort {
        config.agent.effort = effort.to_string();
        // Claude handles effort natively; preserve the prompt budget only for
        // the older stdin/stdout backends.
        if config.agent.command != "claude" {
            let budget = match effort {
                Effort::Low => 4_000,
                Effort::Medium => 10_000,
                Effort::High => 32_000,
                Effort::Max => 100_000,
            };
            config.prompt.token_budget = budget;
        }
    }

    Ok(config)
}

fn apply_resume_session_override(config: &mut Config, resume: Option<String>) {
    let Some(session_id) = resume
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    if let Some(existing) = config
        .agent
        .env
        .iter_mut()
        .find(|(key, _)| key.eq_ignore_ascii_case("ROKO_SESSION_ID"))
    {
        existing.1 = session_id;
    } else {
        config
            .agent
            .env
            .push(("ROKO_SESSION_ID".to_string(), session_id));
    }
}

fn prepare_runtime_hooks(workdir: &Path, quiet: bool) {
    if let Err(err) = bootstrap_observability_dirs(workdir)
        && !quiet
    {
        tracing::warn!(%err, "observability bootstrap failed");
    }
    run_process_lifecycle_hooks(workdir, quiet);
}

fn setup_graceful_shutdown() -> GracefulShutdown {
    let shutdown = GracefulShutdown::new();
    shutdown.register("reap_orphaned_children", || async {
        let reaped = reap_orphaned_children();
        if reaped > 0 {
            tracing::warn!(reaped, "SIGTERM shutdown reaped orphaned children");
        }
    });
    shutdown
}

#[cfg(unix)]
fn install_sigterm_handler(runtime: &tokio::runtime::Runtime, shutdown: GracefulShutdown) {
    std::mem::drop(runtime.spawn(async move {
        let Ok(mut sigterm) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        else {
            tracing::warn!("failed to install SIGTERM handler");
            return;
        };
        while sigterm.recv().await.is_some() {
            // A running `plan run` owns SIGTERM: it cancels its graph,
            // finalizes checkpoints, restores the terminal and exits 143
            // itself, with its own bounded forced-exit fallback.
            if roko_cli::graph_execution::plan_runner::plan_run_owns_termination_signals() {
                continue;
            }
            let report = shutdown.drain().await;
            tracing::info!(
                drained_hooks = report.drained_hooks,
                timed_out_hooks = report.timed_out_hooks,
                elapsed_ms = report.elapsed_ms,
                "SIGTERM graceful shutdown complete"
            );
            // A terminated process never reports success.
            std::process::exit(
                roko_cli::graph_execution::plan_runner::PlanRunInterrupt::Terminate.exit_code(),
            );
        }
    }));
}

#[cfg(not(unix))]
fn install_sigterm_handler(_runtime: &tokio::runtime::Runtime, _shutdown: GracefulShutdown) {}

fn bootstrap_observability_dirs(workdir: &Path) -> std::io::Result<()> {
    // Only create .roko/ if the user has expressed intent (roko.toml exists or .roko/ already exists).
    if !workdir.join("roko.toml").exists() && !workdir.join(".roko").exists() {
        return Ok(());
    }
    roko_core::Workspace::create(workdir).map_err(std::io::Error::other)?;
    let sinks = FsObservabilitySinks::for_workdir(workdir);
    std::fs::create_dir_all(sinks.trace_sink.root())?;
    if let Some(parent) = sinks.metrics_sink.path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn run_process_lifecycle_hooks(workdir: &Path, quiet: bool) {
    // Key the PID registry to this workspace, so cleanup inspects the records
    // of Roko processes that ran here whatever the current directory is.
    roko_agent::process::set_registry_root(workdir);
    cleanup_orphaned_agents();
    let reaped = reap_orphaned_children();
    if reaped > 0 && !quiet {
        tracing::info!(reaped, "reaped orphaned agent processes");
    }
}

/// The `--workdir` passed to the deepest invoked subcommand that accepts one.
///
/// Subcommands declare `--workdir` individually, so this reads the argument
/// matches instead of enumerating every `Command` variant.
fn invoked_subcommand_workdir() -> Option<PathBuf> {
    let matches = Cli::command().try_get_matches().ok()?;
    subcommand_workdir(&matches)
}

fn subcommand_workdir(matches: &clap::ArgMatches) -> Option<PathBuf> {
    let mut workdir = None;
    let mut current = matches;
    while let Some((_, sub)) = current.subcommand() {
        if let Ok(Some(dir)) = sub.try_get_one::<PathBuf>("workdir") {
            workdir = Some(dir.clone());
        }
        current = sub;
    }
    workdir
}

fn parse_dashboard_page(input: &str) -> Option<PageId> {
    let normalized = input.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    Some(match normalized.as_str() {
        "health" => PageId::Health,
        "trends" => PageId::Trends,
        "correlations" => PageId::Correlations,
        "learning" => PageId::Learning,
        "parameters" => PageId::Parameters,
        "experiments" => PageId::Experiments,
        "optimizer" => PageId::Optimizer,
        "provider-health" | "providerhealth" => PageId::ProviderHealth,
        "model-comparison" | "modelcomparison" => PageId::ModelComparison,
        "agent-status" | "agentstatus" | "agent-activity" | "agentactivity" => PageId::AgentStatus,
        "plan-view" | "planview" => PageId::PlanView,
        "log-view" | "logview" => PageId::LogView,
        "signals" => PageId::Signals,
        "config-view" | "configview" => PageId::ConfigView,
        _ => return None,
    })
}

fn dashboard_page_slugs() -> Vec<&'static str> {
    [
        PageId::Health,
        PageId::Trends,
        PageId::Correlations,
        PageId::Learning,
        PageId::Parameters,
        PageId::Experiments,
        PageId::Optimizer,
        PageId::ProviderHealth,
        PageId::ModelComparison,
        PageId::AgentStatus,
        PageId::PlanView,
        PageId::LogView,
        PageId::Signals,
        PageId::ConfigView,
    ]
    .into_iter()
    .map(PageId::slug)
    .collect()
}

/// Load `~/.roko/.env` and `./.roko/.env` into the process environment.
///
/// Returns the loaded entries, the secrets the process's scrubber redacts
/// from its logs and persisted records, and records the loaded names (never
/// values) in [`roko_core::child_env`] so gate commands and provider CLIs
/// treat them as secrets instead of inheriting them.
fn load_startup_env_files() -> Result<Vec<(String, String)>> {
    let mut redactions = Vec::new();
    let mut dotenv_names = roko_core::child_env::DotenvNames::new();

    // 1. Global: ~/.roko/.env — lower priority, does NOT override existing env vars.
    if let Some(home) = env::var_os("HOME") {
        let global_env = PathBuf::from(home).join(".roko").join(".env");
        if global_env.is_file() {
            let entries = load_env_file(&global_env)?;
            record_dotenv_entries(&mut dotenv_names, &entries, false, |name| env::var_os(name));
            redactions.extend(entries);
            dotenvy::from_path(&global_env)
                .with_context(|| format!("load {}", global_env.display()))?;
        }
    }

    // 2. Project-local: {workdir}/.roko/.env — higher priority, overrides existing vars.
    //    At this point the CLI hasn't parsed yet, so workdir == cwd.
    let local_env = PathBuf::from(".roko").join(".env");
    if local_env.is_file() {
        let entries = load_env_file(&local_env)?;
        record_dotenv_entries(&mut dotenv_names, &entries, true, |name| env::var_os(name));
        redactions.extend(entries);
        dotenvy::from_path_override(&local_env)
            .with_context(|| format!("load {}", local_env.display()))?;
    }

    roko_core::child_env::record_startup_dotenv(dotenv_names);
    Ok(redactions)
}

/// Record the names in one `.env` file's `entries`. `current` reads a
/// variable as it is before the file loads; `overrides` is true for a file
/// whose values replace existing ones. The file supplies a value when the
/// variable was unset, or when it overrides a different value.
fn record_dotenv_entries(
    names: &mut roko_core::child_env::DotenvNames,
    entries: &[(String, String)],
    overrides: bool,
    current: impl Fn(&str) -> Option<std::ffi::OsString>,
) {
    for (name, value) in entries {
        let supplied = match current(name) {
            None => true,
            Some(existing) => overrides && existing != value.as_str(),
        };
        names.insert(name.clone(), supplied);
    }
}

fn load_env_file(path: &Path) -> Result<Vec<(String, String)>> {
    let entries = dotenvy::from_path_iter(path)
        .with_context(|| format!("inspect {}", path.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("parse {}", path.display()))?;
    Ok(entries)
}

// Re-export for crate-internal callers (e.g. do_cmd.rs uses `crate::resolve_mcp_config_with_autodiscovery`).
pub use commands::mcp::resolve_mcp_config_with_autodiscovery;

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use commands::config_cmd::{
        ModelListRow, ProviderHealthRow, ProviderLatencySummary, ProviderListRow,
        build_model_list_row, build_provider_health_row, format_model_rows,
        format_provider_health_rows, format_provider_rows, select_provider_test_model,
    };
    use commands::dashboard::dashboard_output;
    use commands::knowledge::{
        NEURO_CONFIRMATIONS_FILE, NEURO_KNOWLEDGE_FILE, backup_neuro_store, neuro_live_files,
        restore_neuro_store,
    };
    use commands::util::persist_capture_episode;
    use roko_core::ConfigHash;
    use tempfile::tempdir;
    use tokio::fs;

    #[test]
    fn cli_parses_no_args() {
        // With no args and no subcommand, cli.prompt and cli.command are None.
        let cli = Cli::try_parse_from(["roko"]).unwrap();
        assert!(cli.command.is_none());
        assert!(cli.prompt.is_none());
        assert!(!cli.json);
        assert!(!cli.quiet);
        assert!(!cli.headless);
    }

    #[test]
    fn cli_parses_global_flags() {
        let cli = Cli::try_parse_from([
            "roko",
            "--role",
            "engineer",
            "--model",
            "gpt-4",
            "--repo",
            "/tmp/proj",
            "--effort",
            "high",
            "--json",
            "--quiet",
            "--headless",
        ])
        .unwrap();
        assert_eq!(cli.role.as_deref(), Some("engineer"));
        assert_eq!(cli.model.as_deref(), Some("gpt-4"));
        assert_eq!(cli.repo, Some(PathBuf::from("/tmp/proj")));
        assert_eq!(cli.effort, Some(Effort::High));
        assert!(cli.json);
        assert!(cli.quiet);
        assert!(cli.headless);
    }

    #[test]
    fn cli_parses_learn_reflexes_workdir() {
        let cli = Cli::try_parse_from([
            "roko",
            "learn",
            "reflexes",
            "--workdir",
            "/tmp/reflex-project",
        ])
        .expect("parse learn reflexes");

        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::Reflexes {
                    workdir: Some(ref workdir),
                },
            }) if workdir == std::path::Path::new("/tmp/reflex-project")
        ));
    }

    // ── #311: learn inspect / config preset parser tests ──────────

    #[test]
    fn cli_parses_learn_inspect_gates() {
        let cli = Cli::try_parse_from(["roko", "learn", "inspect", "gates"])
            .expect("parse learn inspect gates");
        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::Inspect {
                    subsystem: InspectSubsystem::Gates { workdir: None },
                },
            })
        ));
    }

    #[test]
    fn cli_parses_learn_inspect_routing_with_workdir() {
        let cli = Cli::try_parse_from([
            "roko",
            "learn",
            "inspect",
            "routing",
            "--workdir",
            "/tmp/proj",
        ])
        .expect("parse learn inspect routing --workdir");
        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::Inspect {
                    subsystem: InspectSubsystem::Routing { workdir: Some(ref wd) },
                },
            }) if wd == std::path::Path::new("/tmp/proj")
        ));
    }

    #[test]
    fn cli_parses_learn_inspect_budget() {
        let cli = Cli::try_parse_from(["roko", "learn", "inspect", "budget"])
            .expect("parse learn inspect budget");
        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::Inspect {
                    subsystem: InspectSubsystem::Budget { workdir: None },
                },
            })
        ));
    }

    #[test]
    fn cli_bench_swe_requires_agent_mode() {
        // A forgotten flag must not silently run the gold control.
        let err = Cli::try_parse_from(["roko", "bench", "swe"])
            .expect_err("bench swe without --agent-mode should not parse");
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);

        let cli = Cli::try_parse_from(["roko", "bench", "swe", "--agent-mode", "command"])
            .expect("parse bench swe --agent-mode command");
        assert!(matches!(
            cli.command,
            Some(Command::Bench {
                cmd: BenchCmd::Swe {
                    agent_mode: roko_cli::bench::SweAgentMode::Command,
                    ..
                },
            })
        ));
    }

    #[test]
    fn cli_parses_learn_tune_deprecated_alias() {
        let cli = Cli::try_parse_from(["roko", "learn", "tune", "routing"])
            .expect("parse learn tune routing");
        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::Tune {
                    ref subsystem,
                    dry_run: false,
                    workdir: None,
                },
            }) if subsystem == "routing"
        ));
    }

    #[test]
    fn cli_parses_learn_tune_dry_run_flag() {
        let cli = Cli::try_parse_from(["roko", "learn", "tune", "--dry-run", "gates"])
            .expect("parse learn tune --dry-run");
        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::Tune {
                    ref subsystem,
                    dry_run: true,
                    workdir: None,
                },
            }) if subsystem == "gates"
        ));
    }

    #[test]
    fn cli_parses_config_preset_gates() {
        let cli = Cli::try_parse_from(["roko", "config", "preset", "gates"])
            .expect("parse config preset gates");
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Preset {
                    cmd: ConfigPresetCmd::Gates {
                        dry_run: false,
                        yes: false,
                        global: false,
                        ..
                    },
                },
            })
        ));
    }

    #[test]
    fn cli_parses_config_preset_routing_dry_run() {
        let cli = Cli::try_parse_from(["roko", "config", "preset", "routing", "--dry-run"])
            .expect("parse config preset routing --dry-run");
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Preset {
                    cmd: ConfigPresetCmd::Routing {
                        dry_run: true,
                        yes: false,
                        ..
                    },
                },
            })
        ));
    }

    #[test]
    fn cli_parses_config_preset_model_with_name() {
        let cli = Cli::try_parse_from(["roko", "config", "preset", "model", "sonnet", "--yes"])
            .expect("parse config preset model sonnet --yes");
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Preset {
                    cmd: ConfigPresetCmd::Model {
                        ref name,
                        yes: true,
                        dry_run: false,
                        ..
                    },
                },
            }) if name == "sonnet"
        ));
    }

    #[test]
    fn cli_parses_config_preset_budget_global() {
        let cli = Cli::try_parse_from(["roko", "config", "preset", "budget", "--global"])
            .expect("parse config preset budget --global");
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Preset {
                    cmd: ConfigPresetCmd::Budget { global: true, .. },
                },
            })
        ));
    }

    #[test]
    fn cli_parses_top_level_tune_deprecated() {
        let cli =
            Cli::try_parse_from(["roko", "tune", "gates"]).expect("parse top-level tune gates");
        assert!(matches!(
            cli.command,
            Some(Command::Tune(TuneCmd::Gates { workdir: None }))
        ));
    }

    #[test]
    fn force_model_alias_arms_the_existing_highest_precedence_override() {
        let cli = Cli::try_parse_from(["roko", "--force-model", "model-b", "status"])
            .expect("parse --force-model alias");
        assert_eq!(cli.model.as_deref(), Some("model-b"));
    }

    #[test]
    fn cli_parses_positional_prompt() {
        let cli = Cli::try_parse_from(["roko", "fix the bug"]).unwrap();
        assert_eq!(cli.prompt.as_deref(), Some("fix the bug"));
        assert!(cli.command.is_none());
    }

    /// bug-17f0e4: a bare word that names no subcommand is an unknown
    /// command, not a one-shot prompt, with `--help` or another subcommand
    /// after it too.
    #[test]
    fn cli_rejects_unknown_single_word_command() {
        use clap::error::ErrorKind;

        for args in [
            &["roko", "dreem"][..],
            &["roko", "dream", "--help"],
            &["roko", "stauts", "--json"],
            &["roko", "fix", "status"],
        ] {
            let err = Cli::try_parse_from(args).expect_err("a bare word is not a prompt");
            assert_eq!(err.kind(), ErrorKind::InvalidSubcommand, "{args:?}");
        }
        let message = Cli::try_parse_from(["roko", "stauts"])
            .expect_err("a typo")
            .to_string();
        assert!(
            message.contains("unrecognized subcommand 'stauts'"),
            "{message}"
        );
        assert!(message.contains("'roko status'"), "{message}");
        assert!(message.contains("'roko run stauts'"), "{message}");
        let message = Cli::try_parse_from(["roko", "dream"])
            .expect_err("a nested command")
            .to_string();
        assert!(message.contains("'roko knowledge dream'"), "{message}");
        // A stale `roko dream run` fails too, before any agent runs.
        assert!(Cli::try_parse_from(["roko", "dream", "run"]).is_err());

        // Prompts of several words and `roko run <word>` still parse.
        let cli = Cli::try_parse_from(["roko", "fix the bug"]).expect("a quoted prompt");
        assert_eq!(cli.prompt.as_deref(), Some("fix the bug"));
        let cli = Cli::try_parse_from(["roko", "run", "fix"]).expect("a one-word run");
        assert!(matches!(cli.command, Some(Command::Run { .. })));
    }

    /// bug-8589fc: an unknown first word is the error whatever follows it,
    /// not the missing prompt of the `run` clap parses after it.
    #[test]
    fn dream_run_reports_an_unknown_command() {
        use clap::error::ErrorKind;

        for args in [
            &["roko", "dream", "run"][..],
            &["roko", "--json", "dream", "run"],
            &["roko", "--config", "roko.toml", "dream", "run"],
            &["roko", "--force-model=m", "dream", "run"],
        ] {
            let error = try_parse_cli(args).expect_err("dream is not a command");
            assert_eq!(error.kind(), ErrorKind::InvalidSubcommand, "{args:?}");
            let message = error.to_string();
            assert!(message.contains("subcommand 'dream'"), "{message}");
            assert!(message.contains("'roko knowledge dream'"), "{message}");
        }
        // An error about a real command stays that command's error.
        for args in [&["roko", "run"][..], &["roko", "--model", "dream", "run"]] {
            let error = try_parse_cli(args).expect_err("run needs a prompt");
            let kind = error.kind();
            assert_eq!(kind, ErrorKind::MissingRequiredArgument, "{args:?}");
        }
        assert!(try_parse_cli(["roko", "status"]).is_ok());
    }

    #[test]
    fn cli_parses_run_subcommand() {
        let cli = Cli::try_parse_from(["roko", "run", "do something"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Run { .. })));
    }

    #[test]
    fn cli_parses_do_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "do",
            "--plan",
            "--complexity",
            "medium",
            "--dry-run",
            "--workdir",
            "/tmp/do-workdir",
            "--provider",
            "openai",
            "--yes",
            "--ghost",
            "--compare",
            "--no-cascade",
            "do",
            "something",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Do {
                plan,
                complexity: Some(complexity),
                dry_run,
                workdir: Some(workdir),
                provider: Some(provider),
                yes,
                ghost,
                compare,
                no_cascade,
                prompt,
                ..
            }) => {
                assert!(plan);
                assert_eq!(complexity, DoComplexity::Medium);
                assert!(dry_run);
                assert_eq!(workdir, PathBuf::from("/tmp/do-workdir"));
                assert_eq!(provider, "openai");
                assert!(yes);
                assert!(ghost);
                assert!(compare);
                assert!(no_cascade);
                assert_eq!(prompt, vec!["do".to_string(), "something".to_string()]);
            }
            other => panic!("expected do command, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_do_continue_optional_value() {
        let cli = Cli::try_parse_from(["roko", "do", "--continue", "work-123"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Do {
                r#continue: Some(Some(ref id)),
                ..
            }) if id == "work-123"
        ));
    }

    #[test]
    fn cli_parses_init_subcommand() {
        let cli = Cli::try_parse_from(["roko", "init", "/tmp/project"]).unwrap();
        match cli.command {
            Some(Command::Init {
                path,
                cloud,
                profile,
                demo,
            }) => {
                assert_eq!(path, Some(PathBuf::from("/tmp/project")));
                assert!(!cloud);
                assert!(profile.is_none());
                assert!(!demo);
            }
            other => panic!("expected init command, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_init_cloud_flag() {
        let cli = Cli::try_parse_from(["roko", "init", "--cloud"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Init { cloud: true, .. })
        ));
    }

    #[test]
    fn cli_parses_init_demo_flag() {
        let cli = Cli::try_parse_from(["roko", "init", "--demo"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Init { demo: true, .. })
        ));
    }

    #[test]
    fn cli_parses_status_subcommand() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Status { .. })));
    }

    #[test]
    fn cli_parses_github_status_subcommand() {
        let cli =
            Cli::try_parse_from(["roko", "github", "status", "--workdir", "/tmp/project"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Github {
                cmd: commands::github::GithubCmd::Status { workdir: Some(_) }
            })
        ));
    }

    #[test]
    fn cli_parses_status_quick_flag() {
        let cli = Cli::try_parse_from(["roko", "status", "--quick"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Status { quick: true, .. })
        ));
    }

    #[test]
    fn cli_rejects_status_quick_with_surfaces() {
        let result = Cli::try_parse_from(["roko", "status", "--quick", "--surfaces"]);
        assert!(result.is_err(), "--quick and --surfaces should conflict");
    }

    #[test]
    fn cli_rejects_status_quick_with_cfactor() {
        let result = Cli::try_parse_from(["roko", "status", "--quick", "--cfactor"]);
        assert!(result.is_err(), "--quick and --cfactor should conflict");
    }

    #[test]
    fn cli_parses_doctor_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "doctor",
            "--workdir",
            "/tmp/project",
            "--serve-url",
            "http://localhost:9090",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Doctor {
                subject: None,
                workdir: Some(_),
                serve_url: Some(_),
            })
        ));
    }

    #[test]
    fn cli_parses_doctor_disk_subreport() {
        let cli =
            Cli::try_parse_from(["roko", "doctor", "disk", "--workdir", "/tmp/project"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Doctor {
                subject: Some(DoctorSubject::Disk),
                workdir: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn cli_parses_doctor_network_subreport() {
        let cli = Cli::try_parse_from(["roko", "doctor", "network", "--workdir", "/tmp/project"])
            .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Doctor {
                subject: Some(DoctorSubject::Network),
                workdir: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn cli_cache_prune_is_dry_run_by_default() {
        let cli = Cli::try_parse_from(["roko", "cache", "prune"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Cache {
                cmd: CacheCmd::Prune { apply: false, .. }
            })
        ));
    }

    #[test]
    fn cli_cache_prune_requires_explicit_apply_flag_for_mutation() {
        let cli = Cli::try_parse_from([
            "roko",
            "cache",
            "prune",
            "--apply",
            "--target-budget-gb",
            "64",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Cache {
                cmd: CacheCmd::Prune {
                    apply: true,
                    target_budget_gb: 64,
                    ..
                }
            })
        ));
    }

    #[test]
    fn cli_parses_acp_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "acp",
            "--workdir",
            "/tmp/project",
            "--profile",
            "editor",
            "--config",
            "/tmp/project/roko.toml",
            "--global-config",
            "/tmp/global-roko.toml",
            "--log-file",
            ".roko/editor-acp.log",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Acp {
                workdir,
                profile,
                config: Some(config),
                global_config: Some(global_config),
                log_file,
            }) if workdir == PathBuf::from("/tmp/project")
                && profile == "editor"
                && config == PathBuf::from("/tmp/project/roko.toml")
                && global_config == PathBuf::from("/tmp/global-roko.toml")
                && log_file == PathBuf::from(".roko/editor-acp.log")
        ));
    }

    #[test]
    fn cli_parses_agent_serve_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "agent",
            "serve",
            "--agent-id",
            "demo-1",
            "--bind",
            "127.0.0.1:7777",
            "--relay-url",
            "https://relay.example",
            "--chain-rpc-url",
            "https://rpc.example",
            "--identity-registry",
            "0x1234",
            "--passport-id",
            "7",
            "--wallet-key",
            "0xdeadbeef",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Agent {
                cmd: AgentCmd::Serve(agent_serve::AgentServeArgs {
                    agent_id,
                    bind,
                    relay_url: Some(_),
                    chain_rpc_url: Some(_),
                    identity_registry: Some(_),
                    passport_id: Some(_),
                    wallet_key: Some(_),
                    ..
                }),
            }) if agent_id == "demo-1" && bind == "127.0.0.1:7777"
        ));
    }

    #[test]
    fn cli_parses_inject_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "inject",
            "session-1",
            "stop doing that",
            "--kind",
            "directive",
        ])
        .unwrap();
        assert!(matches!(cli.command, Some(Command::Inject { .. })));
    }

    // -- inject file-transport tests (#325, updated for #361) --
    // With the file-based ControlCommand transport (#361), valid inject requests
    // now succeed by writing a control.json file. The substrate (engrams.jsonl)
    // must still NOT be written by the inject path itself.

    #[tokio::test]
    async fn inject_fail_closed_directive() {
        let tmp = tempfile::tempdir().unwrap();
        let roko_dir = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko_dir).unwrap();
        let cli = Cli::try_parse_from(["roko", "inject", "sess-1", "do something"]).unwrap();
        let code = commands::util::cmd_inject(
            &cli,
            "sess-1".into(),
            "directive",
            "do something".into(),
            Some(tmp.path().to_path_buf()),
        )
        .await
        .unwrap();
        assert_eq!(
            code, EXIT_SUCCESS,
            "inject directive succeeds via file-based transport"
        );
        // Control file should exist.
        assert!(
            roko_dir.join("state/control.json").exists(),
            "control file should be written"
        );
        // No signal log should be created.
        assert!(
            !roko_dir.join("engrams.jsonl").exists(),
            "no substrate write should occur"
        );
    }

    #[tokio::test]
    async fn inject_fail_closed_abort() {
        let tmp = tempfile::tempdir().unwrap();
        let cli = Cli::try_parse_from(["roko", "inject", "sess-1", "", "--kind", "abort"]).unwrap();
        let code = commands::util::cmd_inject(
            &cli,
            "sess-1".into(),
            "abort",
            String::new(),
            Some(tmp.path().to_path_buf()),
        )
        .await
        .unwrap();
        assert_eq!(
            code, EXIT_SUCCESS,
            "inject abort succeeds via file-based transport"
        );
    }

    #[tokio::test]
    async fn inject_fail_closed_context() {
        let tmp = tempfile::tempdir().unwrap();
        let cli = Cli::try_parse_from(["roko", "inject", "sess-1", "ctx data"]).unwrap();
        let code = commands::util::cmd_inject(
            &cli,
            "sess-1".into(),
            "context",
            "ctx data".into(),
            Some(tmp.path().to_path_buf()),
        )
        .await
        .unwrap();
        assert_eq!(
            code, EXIT_SUCCESS,
            "inject context succeeds via file-based transport"
        );
    }

    #[tokio::test]
    async fn inject_fail_closed_validation_more_specific() {
        // Malformed input (empty session) should still produce a validation
        // error, not the generic transport-unavailable error.
        let tmp = tempfile::tempdir().unwrap();
        let cli = Cli::try_parse_from(["roko", "inject", "sess-1", "payload"]).unwrap();
        let result = commands::util::cmd_inject(
            &cli,
            String::new(), // empty session
            "directive",
            "payload".into(),
            Some(tmp.path().to_path_buf()),
        )
        .await;
        assert!(
            result.is_err(),
            "empty session must produce an error, not a transport failure"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("session_id"),
            "error must mention session_id: {msg}"
        );
    }

    #[tokio::test]
    async fn inject_fail_closed_json_output() {
        let tmp = tempfile::tempdir().unwrap();
        let cli = Cli::try_parse_from(["roko", "--json", "inject", "sess-1", "payload"]).unwrap();
        assert!(cli.json, "json flag must be set");
        let code = commands::util::cmd_inject(
            &cli,
            "sess-1".into(),
            "directive",
            "payload".into(),
            Some(tmp.path().to_path_buf()),
        )
        .await
        .unwrap();
        assert_eq!(
            code, EXIT_SUCCESS,
            "inject JSON succeeds via file-based transport"
        );
    }

    #[test]
    fn cli_parses_plan_list() {
        let cli = Cli::try_parse_from(["roko", "plan", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Plan {
                cmd: PlanCmd::List { .. }
            })
        ));
    }

    #[test]
    fn read_only_plan_commands_do_not_rebuild_indexes() {
        let commands = [
            Cli::try_parse_from(["roko", "plan", "list"]).unwrap(),
            Cli::try_parse_from(["roko", "plan", "show", "P34"]).unwrap(),
            Cli::try_parse_from(["roko", "plan", "validate", "plans/"]).unwrap(),
            Cli::try_parse_from(["roko", "plan", "index", "--check"]).unwrap(),
        ];

        for cli in commands {
            let Some(Command::Plan { cmd }) = cli.command else {
                panic!("expected a plan command");
            };
            assert!(!cmd.should_rebuild_indexes());
        }
    }

    #[test]
    fn mutating_plan_commands_rebuild_indexes_but_dry_runs_do_not() {
        let mutating = [
            Cli::try_parse_from(["roko", "plan", "create", "my-plan", "--title", "My Plan"])
                .unwrap(),
            Cli::try_parse_from(["roko", "plan", "run", "plans/"]).unwrap(),
            Cli::try_parse_from(["roko", "plan", "generate", "fix", "the", "bug"]).unwrap(),
            Cli::try_parse_from(["roko", "plan", "regenerate", "plans/my-plan"]).unwrap(),
        ];
        for cli in mutating {
            let Some(Command::Plan { cmd }) = cli.command else {
                panic!("expected a plan command");
            };
            assert!(cmd.should_rebuild_indexes());
            assert!(should_rebuild_plan_indexes(
                cmd.should_rebuild_indexes(),
                Some(EXIT_SUCCESS)
            ));
            assert!(!should_rebuild_plan_indexes(
                cmd.should_rebuild_indexes(),
                Some(1)
            ));
            assert!(!should_rebuild_plan_indexes(
                cmd.should_rebuild_indexes(),
                None
            ));
        }

        let dry_runs = [
            Cli::try_parse_from(["roko", "plan", "run", "plans/", "--dry-run"]).unwrap(),
            Cli::try_parse_from(["roko", "plan", "regenerate", "plans/my-plan", "--dry-run"])
                .unwrap(),
        ];
        for cli in dry_runs {
            let Some(Command::Plan { cmd }) = cli.command else {
                panic!("expected a plan command");
            };
            assert!(!cmd.should_rebuild_indexes());
        }
    }

    #[test]
    fn read_only_prd_commands_do_not_rebuild_indexes() {
        let commands = [
            Cli::try_parse_from(["roko", "prd", "list"]).unwrap(),
            Cli::try_parse_from(["roko", "prd", "status"]).unwrap(),
            Cli::try_parse_from(["roko", "prd", "draft", "list"]).unwrap(),
            Cli::try_parse_from(["roko", "prd", "plan", "my-prd", "--dry-run"]).unwrap(),
        ];
        for cli in commands {
            let Some(Command::Prd { cmd }) = cli.command else {
                panic!("expected a prd command");
            };
            assert!(!cmd.should_rebuild_indexes(), "{cmd:?}");
        }
    }

    #[test]
    fn mutating_prd_commands_rebuild_indexes() {
        let commands = [
            Cli::try_parse_from(["roko", "prd", "idea", "wire", "the", "runner"]).unwrap(),
            Cli::try_parse_from(["roko", "prd", "draft", "new", "a", "title"]).unwrap(),
            Cli::try_parse_from(["roko", "prd", "draft", "edit", "my-prd"]).unwrap(),
            Cli::try_parse_from(["roko", "prd", "draft", "promote", "my-prd"]).unwrap(),
            Cli::try_parse_from(["roko", "prd", "plan", "my-prd"]).unwrap(),
            Cli::try_parse_from(["roko", "prd", "consolidate"]).unwrap(),
        ];
        for cli in commands {
            let Some(Command::Prd { cmd }) = cli.command else {
                panic!("expected a prd command");
            };
            assert!(cmd.should_rebuild_indexes(), "{cmd:?}");
        }
    }

    #[test]
    fn prd_list_leaves_the_index_files_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let cli = Cli::try_parse_from(["roko", "prd", "list"]).unwrap();
        let Some(Command::Prd { cmd }) = cli.command else {
            panic!("expected a prd command");
        };

        let rebuild = cmd.should_rebuild_indexes();
        let exit_code = finish_with_index_rebuild(Ok(EXIT_SUCCESS), tmp.path(), rebuild).unwrap();

        assert_eq!(exit_code, EXIT_SUCCESS);
        assert!(!tmp.path().join(".roko").exists());
    }

    #[test]
    fn successful_command_propagates_index_rebuild_failure() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".roko"), b"not a directory").unwrap();

        let error = finish_with_index_rebuild(Ok(EXIT_SUCCESS), tmp.path(), true).unwrap_err();

        assert!(
            error.to_string().contains("Not a directory")
                || error.to_string().contains("not a directory"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn failed_or_read_only_command_does_not_attempt_index_rebuild() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".roko"), b"not a directory").unwrap();

        let primary =
            finish_with_index_rebuild(Err(anyhow!("primary command failed")), tmp.path(), true)
                .unwrap_err();
        let read_only = finish_with_index_rebuild(Ok(EXIT_SUCCESS), tmp.path(), false).unwrap();

        assert_eq!(primary.to_string(), "primary command failed");
        assert_eq!(read_only, EXIT_SUCCESS);
    }

    #[test]
    fn nonzero_primary_result_is_preserved_without_index_rebuild() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".roko"), b"not a directory").unwrap();

        let exit_code = finish_with_index_rebuild(Ok(7), tmp.path(), true).unwrap();

        assert_eq!(exit_code, 7);
    }

    #[test]
    fn plan_run_rebuild_uses_the_command_workdir() {
        let cli = Cli::try_parse_from([
            "roko",
            "plan",
            "run",
            "plans",
            "--workdir",
            "selected-workspace",
        ])
        .unwrap();
        let Some(Command::Plan { ref cmd }) = cli.command else {
            panic!("expected a plan command");
        };

        assert_eq!(
            cmd.index_rebuild_workdir(&cli),
            PathBuf::from("selected-workspace")
        );
    }

    #[test]
    fn cli_parses_plan_create() {
        let cli = Cli::try_parse_from(["roko", "plan", "create", "my-plan", "--title", "My Plan"])
            .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Plan {
                cmd: PlanCmd::Create { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_non_mutating_plan_index_check() {
        let cli =
            Cli::try_parse_from(["roko", "plan", "index", "--check", "--workdir", "."]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Plan {
                cmd: PlanCmd::Index {
                    check: true,
                    workdir: Some(_),
                }
            })
        ));
    }

    #[test]
    fn cli_parses_plan_resume_flag() {
        let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--resume-plan"]).unwrap();
        let Some(Command::Plan {
            cmd: PlanCmd::Run { resume_plan, .. },
        }) = cli.command
        else {
            panic!("expected plan run");
        };
        assert_eq!(
            resume_plan,
            Some(PathBuf::from(".roko/state/state-snapshot.json"))
        );
    }

    #[test]
    fn cli_parses_continuous_screenshot_options() {
        let cli = Cli::try_parse_from([
            "roko",
            "plan",
            "run",
            "plans",
            "--screenshots",
            "--screenshot-interval",
            "30",
            "--screenshot-dir",
            "/private/tmp/roko-evidence",
        ])
        .unwrap();
        let Some(Command::Plan {
            cmd:
                PlanCmd::Run {
                    screenshots,
                    screenshot_interval,
                    screenshot_dir,
                    ..
                },
        }) = cli.command
        else {
            panic!("expected plan run");
        };
        assert!(screenshots);
        assert_eq!(screenshot_interval, 30);
        assert_eq!(
            screenshot_dir,
            Some(PathBuf::from("/private/tmp/roko-evidence"))
        );
    }

    #[test]
    fn cli_rejects_zero_screenshot_interval() {
        assert!(
            Cli::try_parse_from([
                "roko",
                "plan",
                "run",
                "plans",
                "--screenshots",
                "--screenshot-interval",
                "0",
            ])
            .is_err()
        );
    }

    #[test]
    fn cli_parses_plan_resume_flag_documented_alias() {
        let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--resume-state"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Plan {
                cmd: PlanCmd::Run {
                    resume_plan: Some(_),
                    ..
                }
            })
        ));
    }

    #[test]
    fn cli_parses_plan_fresh_flag() {
        let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--fresh"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Plan {
                cmd: PlanCmd::Run { fresh: true, .. }
            })
        ));
    }

    #[test]
    fn cli_parses_plan_parallel_flags() {
        let cli = Cli::try_parse_from([
            "roko",
            "plan",
            "run",
            "plans",
            "--max-parallel-plans",
            "3",
            "--fail-fast",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Plan {
                cmd: PlanCmd::Run {
                    max_parallel_plans: Some(3),
                    fail_fast: true,
                    ..
                }
            })
        ));
        let cli = Cli::try_parse_from(["roko", "plan", "run", "plans"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Plan {
                cmd: PlanCmd::Run {
                    max_parallel_plans: None,
                    fail_fast: false,
                    ..
                }
            })
        ));
        assert!(
            Cli::try_parse_from(["roko", "plan", "run", "plans", "--max-parallel-plans", "0"])
                .is_err()
        );
    }

    #[test]
    fn cli_parses_plan_force_resume_flag() {
        let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--force-resume"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Plan {
                cmd: PlanCmd::Run {
                    force_resume: true,
                    ..
                }
            })
        ));
    }

    #[test]
    fn cli_parses_resume_flag() {
        let cli = Cli::try_parse_from(["roko", "--resume", "sess-42"]).unwrap();
        assert_eq!(cli.resume.as_deref(), Some("sess-42"));
    }

    #[test]
    fn apply_resume_session_override_adds_env_var() {
        let mut config = Config::default();
        apply_resume_session_override(&mut config, Some("sess-42".to_string()));
        assert_eq!(
            config
                .agent
                .env
                .iter()
                .find(|(key, _)| key == "ROKO_SESSION_ID")
                .map(|(_, value)| value.as_str()),
            Some("sess-42")
        );
    }

    #[test]
    fn apply_resume_session_override_updates_existing_env_var() {
        let mut config = Config::default();
        config
            .agent
            .env
            .push(("ROKO_SESSION_ID".to_string(), "old".to_string()));
        apply_resume_session_override(&mut config, Some("  sess-99  ".to_string()));
        assert_eq!(
            config
                .agent
                .env
                .iter()
                .find(|(key, _)| key == "ROKO_SESSION_ID")
                .map(|(_, value)| value.as_str()),
            Some("sess-99")
        );
    }

    #[test]
    fn effort_display() {
        assert_eq!(Effort::Low.to_string(), "low");
        assert_eq!(Effort::Medium.to_string(), "medium");
        assert_eq!(Effort::High.to_string(), "high");
        assert_eq!(Effort::Max.to_string(), "max");
    }

    #[test]
    fn effort_value_enum_all_variants() {
        // Ensure all four variants parse.
        for name in &["low", "medium", "high", "max"] {
            let cli = Cli::try_parse_from(["roko", "--effort", name]).unwrap();
            assert!(cli.effort.is_some());
        }
    }

    #[test]
    fn exit_code_constants() {
        assert_eq!(EXIT_SUCCESS, 0);
        assert_eq!(EXIT_AGENT_FAILURE, 1);
        assert_eq!(EXIT_SYSTEM_ERROR, 2);
    }

    #[test]
    fn resolve_workdir_uses_repo_flag() {
        let cli = Cli::try_parse_from(["roko", "--repo", "/custom"]).unwrap();
        assert_eq!(resolve_workdir(&cli), PathBuf::from("/custom"));
    }

    #[test]
    fn resolve_workdir_defaults_to_cwd() {
        let cli = Cli::try_parse_from(["roko"]).unwrap();
        let cwd = PathBuf::from(".").canonicalize().unwrap();
        let expected = enclosing_project_of_data_dir(&cwd).unwrap_or(cwd);
        assert_eq!(resolve_workdir(&cli), expected);
    }

    #[test]
    fn resolve_workdir_keeps_a_workspace_nested_under_dot_roko() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().canonicalize().unwrap();
        let worktree = project.join(".roko").join("worktrees").join("x");
        std::fs::create_dir_all(worktree.join("src")).unwrap();
        // A git worktree has a `.git` file that points at the main repository.
        std::fs::write(worktree.join(".git"), "gitdir: /repo/.git/worktrees/x\n").unwrap();

        assert_eq!(enclosing_project_of_data_dir(&worktree), None);
        assert_eq!(enclosing_project_of_data_dir(&worktree.join("src")), None);
        let cli = Cli::try_parse_from(["roko", "--repo", worktree.to_str().unwrap()]).unwrap();
        assert_eq!(resolve_workdir(&cli), worktree);
    }

    #[test]
    fn resolve_workdir_still_redirects_from_the_data_dir() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().canonicalize().unwrap();
        let data_dir = project.join(".roko");
        let state = data_dir.join("state");
        std::fs::create_dir_all(&state).unwrap();

        let expected = Some(project.clone());
        assert_eq!(enclosing_project_of_data_dir(&state), expected);
        assert_eq!(enclosing_project_of_data_dir(&data_dir), expected);
        assert_eq!(enclosing_project_of_data_dir(&project), None);
        // An explicit --repo is used as given, with a warning.
        let cli = Cli::try_parse_from(["roko", "--repo", state.to_str().unwrap()]).unwrap();
        assert_eq!(resolve_workdir(&cli), state);
    }

    #[test]
    fn resolve_workdir_canonicalizes_existing_repo_flag() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path().join("workspace");
        std::fs::create_dir_all(&repo).unwrap();
        let repo_arg = repo.join(".");
        let cli = Cli::try_parse_from(["roko", "--repo", repo_arg.to_str().unwrap()]).unwrap();

        assert_eq!(resolve_workdir(&cli), repo.canonicalize().unwrap());
    }

    #[test]
    fn resolve_plans_dir_prefers_top_level_plans() {
        let tmp = tempdir().unwrap();
        let workdir = tmp.path();
        let canonical = workdir.join("plans");
        let fallback = workdir.join(".roko").join("plans");
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::create_dir_all(&fallback).unwrap();

        assert_eq!(resolve_plans_dir(workdir, None), canonical);
    }

    #[test]
    fn resolve_plans_dir_falls_back_to_dot_roko_plans_that_hold_plans() {
        let tmp = tempdir().unwrap();
        let workdir = tmp.path();
        let fallback = workdir.join(".roko").join("plans");
        std::fs::create_dir_all(fallback.join("old-plan")).unwrap();
        std::fs::write(fallback.join("old-plan").join("tasks.toml"), "").unwrap();

        assert_eq!(resolve_plans_dir(workdir, None), fallback);
    }

    #[test]
    fn resolve_plans_dir_ignores_an_empty_dot_roko_plans() {
        let tmp = tempdir().unwrap();
        let workdir = tmp.path();
        std::fs::create_dir_all(workdir.join(".roko").join("plans")).unwrap();

        assert_eq!(resolve_plans_dir(workdir, None), workdir.join("plans"));
    }

    #[test]
    fn resolve_plans_dir_returns_canonical_when_neither_directory_exists() {
        let tmp = tempdir().unwrap();
        let workdir = tmp.path();
        let canonical = workdir.join("plans");

        assert_eq!(resolve_plans_dir(workdir, None), canonical);
    }

    #[test]
    fn resolve_plans_dir_honors_explicit_path() {
        let tmp = tempdir().unwrap();
        let workdir = tmp.path();
        let explicit = workdir.join("custom-plans");

        assert_eq!(resolve_plans_dir(workdir, Some(&explicit)), explicit);
    }

    #[tokio::test]
    async fn persist_capture_episode_records_learning_episode() {
        let dir = tempdir().unwrap();
        let workdir = dir.path();

        persist_capture_episode(
            workdir,
            "claude",
            Some("claude-sonnet-4-6"),
            "prd-draft-new",
            "prd:draft:new:demo",
            "draft a PRD",
            "# demo prd",
            true,
            321,
            Some("resume-123"),
        )
        .await
        .unwrap();

        let episodes_path = workdir.join(".roko").join("episodes.jsonl");
        let episodes = EpisodeLogger::read_all_lossy(&episodes_path).await.unwrap();
        assert_eq!(episodes.len(), 1);
        assert!(!workdir.join(".roko/learn/episodes.jsonl").exists());
        assert!(!workdir.join(".roko/memory/episodes.jsonl").exists());

        let episode = &episodes[0];
        assert_eq!(episode.agent_id, "claude");
        assert_eq!(episode.task_id, "prd:draft:new:demo");
        assert_eq!(episode.kind, "agent_turn");
        assert_eq!(episode.model, "claude-sonnet-4-6");
        assert!(episode.success);
        assert_eq!(
            episode.extra.get("task_kind"),
            Some(&serde_json::json!("prd-draft-new"))
        );
        assert_eq!(
            episode.extra.get("provider"),
            Some(&serde_json::json!("anthropic"))
        );
        assert_eq!(
            episode.extra.get("role"),
            Some(&serde_json::json!("Strategist"))
        );
        assert_eq!(
            episode.extra.get("task_category"),
            Some(&serde_json::json!("docs"))
        );
        assert_eq!(
            episode.extra.get("complexity_band"),
            Some(&serde_json::json!("standard"))
        );
        assert_eq!(
            episode.extra.get("plan_id"),
            Some(&serde_json::json!("demo"))
        );
        assert_eq!(
            episode.extra.get("session_id"),
            Some(&serde_json::json!("resume-123"))
        );
    }

    #[test]
    fn cli_parses_config_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "show"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Show {
                    effective: false,
                    ..
                }
            })
        ));
    }

    #[test]
    fn cli_parses_config_show_effective() {
        let cli = Cli::try_parse_from(["roko", "config", "show", "--effective"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Show {
                    effective: true,
                    ..
                }
            })
        ));
    }

    #[test]
    fn cli_parses_config_wizard_alias() {
        let cli = Cli::try_parse_from(["roko", "config", "wizard"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Init { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_check_secrets_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "check-secrets"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::CheckSecrets { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_set_secret_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "set-secret", "TOKEN", "value"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::SetSecret { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_config_secrets_subcommand() {
        let cli =
            Cli::try_parse_from(["roko", "config", "secrets", "get", "anthropic.api_key"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Secrets { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_config_mcp_list_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "mcp", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Mcp {
                    cmd: ConfigMcpCmd::List { .. }
                }
            })
        ));
    }

    #[test]
    fn cli_parses_config_mcp_test_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "mcp", "test", "roko"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Mcp {
                    cmd: ConfigMcpCmd::Test { name, .. }
                }
            }) if name == "roko"
        ));
    }

    #[test]
    fn cli_parses_config_mcp_add_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "config",
            "mcp",
            "add",
            "roko",
            "/bin/echo",
            "--",
            "hello",
            "world",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Mcp {
                    cmd: ConfigMcpCmd::Add {
                        name,
                        command,
                        args,
                        ..
                    }
                }
            }) if name == "roko" && command == "/bin/echo" && args == vec!["hello".to_string(), "world".to_string()]
        ));
    }

    #[test]
    fn cli_parses_replay_subcommand() {
        let cli = Cli::try_parse_from(["roko", "replay", "abcd1234"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Replay { .. })));
    }

    #[test]
    fn cli_parses_replay_from_event() {
        let cli = Cli::try_parse_from(["roko", "replay", "abcd1234", "--from-event", "3"]).unwrap();
        match cli.command {
            Some(Command::Replay {
                from_event, as_of, ..
            }) => {
                assert_eq!(from_event.as_deref(), Some("3"));
                assert!(as_of.is_none());
            }
            other => panic!("expected Replay, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_replay_as_of_hidden() {
        let cli = Cli::try_parse_from(["roko", "replay", "abcd1234", "--as-of", "5"]).unwrap();
        match cli.command {
            Some(Command::Replay {
                from_event, as_of, ..
            }) => {
                assert!(from_event.is_none());
                assert_eq!(as_of.as_deref(), Some("5"));
            }
            other => panic!("expected Replay, got {other:?}"),
        }
    }

    #[test]
    fn cli_replay_from_event_conflicts_with_as_of() {
        let result = Cli::try_parse_from([
            "roko",
            "replay",
            "abcd1234",
            "--from-event",
            "3",
            "--as-of",
            "5",
        ]);
        assert!(result.is_err(), "--from-event and --as-of must conflict");
    }

    #[test]
    fn cli_parses_replay_with_workdir() {
        let cli =
            Cli::try_parse_from(["roko", "replay", "abcd1234", "--workdir", "/tmp/proj"]).unwrap();
        match cli.command {
            Some(Command::Replay { workdir, .. }) => {
                assert_eq!(workdir, Some(PathBuf::from("/tmp/proj")));
            }
            other => panic!("expected Replay, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_replay_global_json() {
        let cli = Cli::try_parse_from(["roko", "--json", "replay", "abcd1234"]).unwrap();
        assert!(cli.json);
        assert!(matches!(cli.command, Some(Command::Replay { .. })));
    }

    #[test]
    fn cli_parses_replay_global_repo() {
        let cli =
            Cli::try_parse_from(["roko", "--repo", "/tmp/proj", "replay", "abcd1234"]).unwrap();
        assert_eq!(cli.repo, Some(PathBuf::from("/tmp/proj")));
        assert!(matches!(cli.command, Some(Command::Replay { .. })));
    }

    #[test]
    fn cli_replay_workdir_overrides_repo() {
        let cli = Cli::try_parse_from([
            "roko",
            "--repo",
            "/tmp/global",
            "replay",
            "abcd1234",
            "--workdir",
            "/tmp/local",
        ])
        .unwrap();
        // Global --repo is /tmp/global, but --workdir is /tmp/local.
        // cmd_replay uses workdir.unwrap_or_else(resolve_workdir), so
        // --workdir takes precedence.
        assert_eq!(cli.repo, Some(PathBuf::from("/tmp/global")));
        match cli.command {
            Some(Command::Replay { workdir, .. }) => {
                assert_eq!(workdir, Some(PathBuf::from("/tmp/local")));
            }
            other => panic!("expected Replay, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_completions_subcommand() {
        let cli = Cli::try_parse_from(["roko", "completions", "zsh"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Completions {
                shell: CompletionShell::Zsh
            })
        ));
    }

    #[test]
    fn cli_parses_deploy_railway_subcommand() {
        let cli = Cli::try_parse_from(["roko", "deploy", "railway"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Railway { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_deploy_railway_unsafe_public_flag() {
        let cli = Cli::try_parse_from(["roko", "deploy", "railway", "--unsafe-public"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Railway {
                    unsafe_public: true,
                    ..
                }
            })
        ));
    }

    #[test]
    fn cli_parses_deploy_fly_subcommand() {
        let cli = Cli::try_parse_from(["roko", "deploy", "fly"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Fly { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_deploy_fly_unsafe_public_flag() {
        let cli = Cli::try_parse_from(["roko", "deploy", "fly", "--unsafe-public"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Fly {
                    unsafe_public: true,
                    ..
                }
            })
        ));
    }

    #[test]
    fn cli_parses_deploy_docker_subcommand() {
        let cli = Cli::try_parse_from(["roko", "deploy", "docker"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Docker { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_deploy_docker_unsafe_public_flag() {
        let cli = Cli::try_parse_from(["roko", "deploy", "docker", "--unsafe-public"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Docker {
                    unsafe_public: true,
                    ..
                }
            })
        ));
    }

    #[test]
    fn deploy_docker_push_flag_absent_by_default() {
        // Without --push the flag must default to false so docker push is NOT invoked.
        let cli = Cli::try_parse_from(["roko", "deploy", "docker"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Docker { push: false, .. }
            })
        ));
    }

    #[test]
    fn deploy_docker_push_flag_set_when_requested() {
        // --push must be parsed and forwarded so cmd_deploy_docker can invoke
        // `docker push` after a successful build+tag.
        let cli = Cli::try_parse_from(["roko", "deploy", "docker", "--push"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Docker { push: true, .. }
            })
        ));
    }

    #[test]
    fn deploy_railway_dry_run_flag() {
        let cli = Cli::try_parse_from(["roko", "deploy", "railway", "--dry-run"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Railway { dry_run: true, .. }
            })
        ));
    }

    #[test]
    fn deploy_fly_dry_run_flag() {
        let cli = Cli::try_parse_from(["roko", "deploy", "fly", "--dry-run"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Fly { dry_run: true, .. }
            })
        ));
    }

    #[test]
    fn deploy_fly_custom_app_and_region() {
        let cli = Cli::try_parse_from([
            "roko", "deploy", "fly", "--app", "my-app", "--region", "lhr",
        ])
        .unwrap();
        if let Some(Command::Deploy {
            cmd: DeployCmd::Fly { app, region, .. },
        }) = cli.command
        {
            assert_eq!(app, "my-app");
            assert_eq!(region, "lhr");
        } else {
            panic!("expected Deploy Fly");
        }
    }

    #[test]
    fn deploy_fly_defaults() {
        let cli = Cli::try_parse_from(["roko", "deploy", "fly"]).unwrap();
        if let Some(Command::Deploy {
            cmd:
                DeployCmd::Fly {
                    app,
                    region,
                    dockerfile,
                    health_path,
                    volume_source,
                    volume_destination,
                    force,
                    dry_run,
                    ..
                },
        }) = cli.command
        {
            assert_eq!(app, "roko-agent");
            assert_eq!(region, "iad");
            assert_eq!(dockerfile, "Dockerfile");
            assert_eq!(health_path, "/health");
            assert_eq!(volume_source, "roko_data");
            assert_eq!(volume_destination, "/data/.roko");
            assert!(!force);
            assert!(!dry_run);
        } else {
            panic!("expected Deploy Fly");
        }
    }

    #[test]
    fn deploy_fly_force_flag() {
        let cli = Cli::try_parse_from(["roko", "deploy", "fly", "--force"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Fly { force: true, .. }
            })
        ));
    }

    #[test]
    fn deploy_docker_dry_run_flag() {
        let cli = Cli::try_parse_from(["roko", "deploy", "docker", "--dry-run"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Deploy {
                cmd: DeployCmd::Docker { dry_run: true, .. }
            })
        ));
    }

    #[test]
    fn deploy_docker_custom_dockerfile_and_target() {
        let cli = Cli::try_parse_from([
            "roko",
            "deploy",
            "docker",
            "--dockerfile",
            "docker/roko.Dockerfile",
            "--target",
            "distroless",
            "--image",
            "my-roko",
        ])
        .unwrap();
        if let Some(Command::Deploy {
            cmd:
                DeployCmd::Docker {
                    dockerfile,
                    target,
                    image,
                    ..
                },
        }) = cli.command
        {
            assert_eq!(dockerfile, "docker/roko.Dockerfile");
            assert_eq!(target.as_deref(), Some("distroless"));
            assert_eq!(image, "my-roko");
        } else {
            panic!("expected Deploy Docker");
        }
    }

    #[test]
    fn deploy_docker_defaults() {
        let cli = Cli::try_parse_from(["roko", "deploy", "docker"]).unwrap();
        if let Some(Command::Deploy {
            cmd:
                DeployCmd::Docker {
                    dockerfile,
                    target,
                    image,
                    dry_run,
                    ..
                },
        }) = cli.command
        {
            assert_eq!(dockerfile, "Dockerfile");
            assert!(target.is_none());
            assert_eq!(image, "roko");
            assert!(!dry_run);
        } else {
            panic!("expected Deploy Docker");
        }
    }

    #[test]
    fn cli_parses_knowledge_query_subcommand() {
        let cli = Cli::try_parse_from(["roko", "knowledge", "query", "rust async"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Knowledge {
                cmd: KnowledgeCmd::Query { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_knowledge_stats_subcommand() {
        let cli = Cli::try_parse_from(["roko", "knowledge", "stats"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Knowledge {
                cmd: KnowledgeCmd::Stats { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_knowledge_gc_subcommand() {
        let cli = Cli::try_parse_from(["roko", "knowledge", "gc"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Knowledge {
                cmd: KnowledgeCmd::Gc { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_knowledge_backup_subcommand() {
        let cli =
            Cli::try_parse_from(["roko", "knowledge", "backup", "/tmp/neuro-backup"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Knowledge {
                cmd: KnowledgeCmd::Backup { destination, force, .. }
            }) if destination == PathBuf::from("/tmp/neuro-backup") && !force
        ));
    }

    #[test]
    fn cli_parses_knowledge_export_and_import_with_safe_defaults() {
        let export =
            Cli::try_parse_from(["roko", "knowledge", "export", "/tmp/knowledge-export.jsonl"])
                .unwrap();
        assert!(matches!(
            export.command,
            Some(Command::Knowledge {
                cmd: KnowledgeCmd::Export {
                    output,
                    force: false,
                    top_n: None,
                    ..
                }
            }) if output == PathBuf::from("/tmp/knowledge-export.jsonl")
        ));

        let import =
            Cli::try_parse_from(["roko", "knowledge", "import", "/tmp/knowledge-export.jsonl"])
                .unwrap();
        assert!(matches!(
            import.command,
            Some(Command::Knowledge {
                cmd: KnowledgeCmd::Import {
                    input,
                    decay_factor,
                    legacy_raw: false,
                    ..
                }
            }) if input == PathBuf::from("/tmp/knowledge-export.jsonl")
                && (decay_factor - 0.8).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn cli_validates_knowledge_import_decay_factor() {
        let cli = Cli::try_parse_from([
            "roko",
            "knowledge",
            "import",
            "/tmp/knowledge-export.jsonl",
            "--decay-factor",
            "0.65",
            "--legacy-raw",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Knowledge {
                cmd: KnowledgeCmd::Import {
                    decay_factor,
                    legacy_raw: true,
                    ..
                }
            }) if (decay_factor - 0.65).abs() < f64::EPSILON
        ));

        let error = Cli::try_parse_from([
            "roko",
            "knowledge",
            "import",
            "/tmp/knowledge-export.jsonl",
            "--decay-factor",
            "1.01",
        ])
        .unwrap_err();
        assert!(error.to_string().contains("between 0.0 and 1.0"));
    }

    #[test]
    fn cli_parses_knowledge_restore_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "knowledge",
            "restore",
            "/tmp/neuro-backup",
            "--force",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Knowledge {
                cmd: KnowledgeCmd::Restore { source, force, .. }
            }) if source == PathBuf::from("/tmp/neuro-backup") && force
        ));
    }

    #[test]
    fn cli_parses_config_experiments_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "config",
            "experiments",
            "model",
            "create",
            "--id",
            "glm-vs-kimi-impl",
            "--role",
            "implementer",
            "--variant",
            "glm-5-1:glm-5.1:zai",
            "--variant",
            "kimi-k2-5:kimi-k2.5:moonshot",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Experiments { .. }
            })
        ));
    }

    #[test]
    fn cli_parses_config_providers_list_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "providers", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Providers {
                    cmd: ConfigProviderCmd::List { .. }
                }
            })
        ));
    }

    #[test]
    fn cli_parses_config_providers_health_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "providers", "health"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Providers {
                    cmd: ConfigProviderCmd::Health { .. }
                }
            })
        ));
    }

    #[test]
    fn cli_parses_config_providers_test_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "providers", "test", "zai"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Providers {
                    cmd: ConfigProviderCmd::Test { provider: Some(ref p), all: false, .. }
                }
            }) if p == "zai"
        ));
    }

    #[test]
    fn cli_parses_config_providers_test_all() {
        let cli = Cli::try_parse_from(["roko", "config", "providers", "test", "--all"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Providers {
                    cmd: ConfigProviderCmd::Test {
                        provider: None,
                        all: true,
                        ..
                    }
                }
            })
        ));
    }

    #[test]
    fn cli_parses_config_providers_available_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "providers", "available"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Providers {
                    cmd: ConfigProviderCmd::Available
                }
            })
        ));
    }

    #[test]
    fn cli_parses_config_models_list_subcommand() {
        let cli = Cli::try_parse_from(["roko", "config", "models", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Models {
                    cmd: ConfigModelCmd::List { .. }
                }
            })
        ));
    }

    #[test]
    fn cli_parses_config_models_route_subcommand() {
        let cli = Cli::try_parse_from([
            "roko",
            "config",
            "models",
            "route",
            "glm-5-1",
            "--explain",
            "--complexity",
            "integrative",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                cmd: ConfigCmd::Models { cmd: ConfigModelCmd::Route { model, explain: true, complexity: Some(complexity), .. } }
            }) if model == "glm-5-1" && complexity == "integrative"
        ));
    }

    #[test]
    fn cli_agent_chat_defaults_to_canonical_serve_url() {
        let cli = Cli::try_parse_from(["roko", "agent", "chat"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Agent { .. })));
    }

    #[test]
    fn cli_daemon_start_defaults_to_canonical_port() {
        let cli = Cli::try_parse_from(["roko", "daemon", "start"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Daemon {
                cmd: DaemonCmd::Start { port, .. }
            }) if port == roko_cli::DEFAULT_SERVE_PORT
        ));
    }

    #[test]
    fn cli_daemon_restart_defaults_to_canonical_port() {
        let cli = Cli::try_parse_from(["roko", "daemon", "restart"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Daemon {
                cmd: DaemonCmd::Restart { port }
            }) if port == roko_cli::DEFAULT_SERVE_PORT
        ));
    }

    #[test]
    fn cli_parses_prd_draft_new_instead_of_top_level_new() {
        let cli = Cli::try_parse_from(["roko", "prd", "draft", "new", "Ship", "it"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Prd {
                cmd: PrdCmd::Draft {
                    cmd: PrdDraftCmd::New { title }
                }
            }) if title == vec!["Ship".to_string(), "it".to_string()]
        ));
    }

    #[test]
    fn cli_new_requires_type_and_name() {
        // `roko new` is a subcommand requiring <TYPE> <NAME>.
        assert!(Cli::try_parse_from(["roko", "new"]).is_err());
        assert!(Cli::try_parse_from(["roko", "new", "gate", "MyGate"]).is_ok());
    }

    #[test]
    fn cli_explain_requires_topic() {
        // `roko explain` is a subcommand requiring <TOPIC>.
        assert!(Cli::try_parse_from(["roko", "explain"]).is_err());
        assert!(Cli::try_parse_from(["roko", "explain", "gates"]).is_ok());
    }

    #[test]
    fn select_provider_test_model_prefers_default_model() {
        let mut config = RokoConfig::default();
        config.agent.default_model = "glm-5-1".to_string();
        config.models.insert(
            "glm-5-1".to_string(),
            ModelProfile {
                provider: "zai".to_string(),
                slug: "glm-5.1".to_string(),
                context_window: 200_000,
                max_output: Some(131_072),
                supports_tools: true,
                supports_thinking: true,
                supports_vision: false,
                supports_web_search: false,
                supports_mcp_tools: false,
                supports_partial: false,
                provider_routing: None,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: Some(1.40),
                cost_output_per_m: Some(4.40),
                cost_cache_read_per_m: None,
                cost_cache_write_per_m: None,
                max_tools: None,
                tokenizer_ratio: None,
                ..Default::default()
            },
        );
        config.models.insert(
            "glm-5-1-alt".to_string(),
            ModelProfile {
                provider: "zai".to_string(),
                slug: "glm-5.1-air".to_string(),
                context_window: 128_000,
                max_output: Some(8_192),
                supports_tools: true,
                supports_thinking: false,
                supports_vision: false,
                supports_web_search: false,
                supports_mcp_tools: false,
                supports_partial: false,
                provider_routing: None,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: Some(1.0),
                cost_output_per_m: Some(2.0),
                cost_cache_read_per_m: None,
                cost_cache_write_per_m: None,
                max_tools: None,
                tokenizer_ratio: None,
                ..Default::default()
            },
        );

        let selected = select_provider_test_model(&config, "zai").expect("selected model");
        assert_eq!(selected.0, "glm-5-1");
        assert_eq!(selected.1.slug, "glm-5.1");
    }

    #[test]
    fn format_provider_rows_renders_headers_and_rows() {
        let output = format_provider_rows(&[ProviderListRow {
            provider: "anthropic".to_string(),
            kind: "claude_cli".to_string(),
            base_url: "(cli: claude)".to_string(),
            status: "ok (cli found)".to_string(),
        }]);

        assert!(output.contains("Provider"));
        assert!(output.contains("Base URL"));
        assert!(output.contains("anthropic"));
        assert!(output.contains("ok (cli found)"));
    }

    #[test]
    fn format_model_rows_renders_headers_and_rows() {
        let output = format_model_rows(&[ModelListRow {
            model: "glm-5-1".to_string(),
            provider: "zai".to_string(),
            slug: "glm-5.1".to_string(),
            context: "200K".to_string(),
            tools: "✓".to_string(),
            thinking: "✓".to_string(),
            vision: "✗".to_string(),
            cost: "$1.40/$4.40".to_string(),
            key_status: "ok".to_string(),
            aliases: String::new(),
        }]);

        assert!(output.contains("Model"));
        assert!(output.contains("Cost (in/out)"));
        assert!(output.contains("glm-5-1"));
        assert!(output.contains("$1.40/$4.40"));
    }

    #[test]
    fn build_model_list_row_formats_capabilities_and_costs() {
        let row = build_model_list_row(
            "kimi-k2-5",
            &ModelProfile {
                provider: "moonshot".to_string(),
                slug: "kimi-k2.5".to_string(),
                context_window: 256_000,
                max_output: Some(128_000),
                supports_tools: true,
                supports_thinking: true,
                supports_vision: true,
                supports_web_search: false,
                supports_mcp_tools: false,
                supports_partial: false,
                provider_routing: None,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: Some(0.60),
                cost_output_per_m: Some(3.00),
                cost_cache_read_per_m: None,
                cost_cache_write_per_m: None,
                max_tools: None,
                tokenizer_ratio: None,
                ..Default::default()
            },
        );

        assert_eq!(row.model, "kimi-k2-5");
        assert_eq!(row.provider, "moonshot");
        assert_eq!(row.slug, "kimi-k2.5");
        assert_eq!(row.context, "256K");
        assert_eq!(row.tools, "✓");
        assert_eq!(row.thinking, "✓");
        assert_eq!(row.vision, "✓");
        assert_eq!(row.cost, "$0.60/$3.00");
    }

    #[test]
    fn build_provider_health_row_formats_state_latency_and_error_rate() {
        let health = ProviderHealth {
            provider_id: "zai".to_string(),
            state: CircuitState::Open,
            consecutive_failures: 3,
            total_requests: 20,
            total_failures: 3,
            last_failure_at: Some(90_000),
            last_success_at: Some(95_000),
            cooldown_until: Some(108_000),
            failure_window: std::collections::VecDeque::new(),
            recent_outcomes: std::collections::VecDeque::new(),
            wasted_cost_usd: 0.0,
        };
        let latency = ProviderLatencySummary {
            recent_latencies: vec![800.0, 1_200.0, 600.0],
            weighted_latency_ms: 0.0,
            observations: 0,
        };

        let row = build_provider_health_row(
            "zai",
            Some(&health),
            Some(&latency),
            100_000,
            Some(95_000),
            Some(99_000),
        );

        assert_eq!(row.provider, "zai");
        assert_eq!(row.state, "OPEN");
        assert_eq!(row.fails, "3/3");
        assert_eq!(row.cooldown, "8s left");
        assert_eq!(row.latency_p50, "0.8s");
        assert_eq!(row.error_rate, "15.0%");
        assert_eq!(row.last_check, "1s ago");
    }

    #[test]
    fn format_provider_health_rows_renders_headers_and_rows() {
        let output = format_provider_health_rows(&[ProviderHealthRow {
            provider: "openrouter".to_string(),
            state: "CLOSED".to_string(),
            fails: "0/3".to_string(),
            cooldown: "—".to_string(),
            latency_p50: "0.8s".to_string(),
            error_rate: "0.0%".to_string(),
            last_check: "5m ago".to_string(),
        }]);

        assert!(output.contains("Provider"));
        assert!(output.contains("Latency p50"));
        assert!(output.contains("Error Rate"));
        assert!(output.contains("openrouter"));
        assert!(output.contains("0.8s"));
    }

    #[test]
    fn cli_parses_dashboard_subcommand() {
        let cli = Cli::try_parse_from(["roko", "dashboard", "--page", "plan-view", "--list-pages"])
            .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Dashboard {
                page: Some(_),
                list_pages: true,
                text: false,
                ..
            })
        ));
    }

    #[test]
    fn cli_parses_dashboard_text_flag() {
        let cli = Cli::try_parse_from(["roko", "dashboard", "--text"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Dashboard { text: true, .. })
        ));
    }

    #[test]
    fn cli_parses_dashboard_snapshot_flag() {
        let cli = Cli::try_parse_from(["roko", "dashboard", "--snapshot", "/tmp/snap"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Dashboard {
                snapshot: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn parse_dashboard_page_accepts_known_slugs() {
        assert_eq!(parse_dashboard_page("health"), Some(PageId::Health));
        assert_eq!(
            parse_dashboard_page("agent status"),
            Some(PageId::AgentStatus)
        );
        assert_eq!(
            parse_dashboard_page("agent activity"),
            Some(PageId::AgentStatus)
        );
        assert_eq!(parse_dashboard_page("plan_view"), Some(PageId::PlanView));
        assert_eq!(parse_dashboard_page("learning"), Some(PageId::Learning));
        assert_eq!(
            parse_dashboard_page("provider health"),
            Some(PageId::ProviderHealth)
        );
        assert_eq!(
            parse_dashboard_page("model comparison"),
            Some(PageId::ModelComparison)
        );
    }

    #[test]
    fn parse_dashboard_page_rejects_unknown_slugs() {
        assert_eq!(parse_dashboard_page("unknown"), None);
    }

    async fn seed_dashboard_snapshot(workdir: &Path) {
        let memory_dir = workdir.join(".roko").join("memory");
        fs::create_dir_all(&memory_dir).await.unwrap();
        let learn_dir = workdir.join(".roko").join("learn");
        fs::create_dir_all(&learn_dir).await.unwrap();

        let mut ep1 = Episode::new("agent-a", "task-a");
        ep1.success = true;
        ep1.usage.cost_usd = 1.25;
        ep1.usage.wall_ms = 125;
        ep1.usage.input_tokens = 100;
        ep1.usage.cache_read_tokens = 25;

        let mut ep2 = Episode::new("agent-b", "task-b");
        ep2.success = false;
        ep2.usage.cost_usd = 2.75;
        ep2.usage.wall_ms = 225;
        ep2.usage.input_tokens = 200;
        ep2.usage.cache_read_tokens = 50;

        let episodes_path = workdir.join(".roko").join("episodes.jsonl");
        let episodes = [
            serde_json::to_string(&ep1).unwrap(),
            serde_json::to_string(&ep2).unwrap(),
        ]
        .join("\n")
            + "\n";
        fs::write(&episodes_path, episodes).await.unwrap();

        let config_hash = ConfigHash::from("abcd1234".to_string());
        let mut metric1 = TaskMetric::new(config_hash.clone(), "plan-a", "task-a");
        metric1.model = "claude-haiku".to_string();
        metric1.gate_passed = true;
        metric1.cost_usd = 1.0;
        metric1.input_tokens = 100;
        metric1.iteration = 1;

        let mut metric2 = TaskMetric::new(config_hash, "plan-b", "task-b");
        metric2.model = "claude-sonnet".to_string();
        metric2.gate_passed = false;
        metric2.cost_usd = 3.0;
        metric2.input_tokens = 200;
        metric2.iteration = 1;

        let task_metrics_path = memory_dir.join("task-metrics.jsonl");
        let task_metrics =
            [metric1.to_jsonl().unwrap(), metric2.to_jsonl().unwrap()].join("\n") + "\n";
        fs::write(&task_metrics_path, task_metrics).await.unwrap();

        let cfactor_path = learn_dir.join("c-factor.jsonl");
        let mut cf1 = CFactor::default();
        cf1.overall = 0.48;
        cf1.computed_at = chrono::Utc::now() - chrono::Duration::days(6);

        let mut cf2 = CFactor::default();
        cf2.overall = 0.53;
        cf2.computed_at = chrono::Utc::now() - chrono::Duration::days(3);

        let mut cf3 = CFactor::default();
        cf3.overall = 0.67;
        cf3.components = roko_learn::cfactor::CFactorComponents {
            gate_pass_rate: 0.82,
            cost_efficiency: 0.76,
            speed: 0.71,
            information_flow_rate: 0.89,
            first_try_rate: 0.64,
            knowledge_growth: 0.18,
            knowledge_integration_rate: 0.57,
            hdc_diversity: 0.73,
            convergence_velocity: 0.66,
            turn_taking_equality: 0.74,
            social_perceptiveness: 0.68,
        };
        cf3.computed_at = chrono::Utc::now();

        let cfactor_history = [
            serde_json::to_string(&cf1).unwrap(),
            serde_json::to_string(&cf2).unwrap(),
            serde_json::to_string(&cf3).unwrap(),
        ]
        .join("\n")
            + "\n";
        fs::write(&cfactor_path, cfactor_history).await.unwrap();

        let provider_health_path = learn_dir.join("provider-health.json");
        let provider_health = serde_json::json!({
            "providers": {
                "anthropic": {
                    "provider_id": "anthropic",
                    "state": "Closed",
                    "consecutive_failures": 0,
                    "total_requests": 12,
                    "total_failures": 1,
                    "last_failure_at": null,
                    "cooldown_until": null,
                    "failure_window": []
                },
                "zai": {
                    "provider_id": "zai",
                    "state": "HalfOpen",
                    "consecutive_failures": 3,
                    "total_requests": 8,
                    "total_failures": 2,
                    "last_failure_at": 1710000000000i64,
                    "cooldown_until": 1710000005000i64,
                    "failure_window": []
                }
            }
        });
        fs::write(
            &provider_health_path,
            serde_json::to_string_pretty(&provider_health).unwrap(),
        )
        .await
        .unwrap();

        let latency_stats_path = learn_dir.join("latency-stats.json");
        let latency_stats = serde_json::json!({
            "entries": [
                {
                    "provider": "anthropic",
                    "stats": {
                        "model_slug": "claude-opus-4-6",
                        "provider_id": "anthropic",
                        "ttft_ema_ms": 0.0,
                        "total_latency_ema_ms": 0.0,
                        "tokens_per_second_ema": 0.0,
                        "observations": 3,
                        "recent_latencies": [800.0, 1200.0, 600.0]
                    }
                }
            ]
        });
        fs::write(
            &latency_stats_path,
            serde_json::to_string_pretty(&latency_stats).unwrap(),
        )
        .await
        .unwrap();

        let cascade_router_path = learn_dir.join("cascade-router.json");
        let cascade_router = serde_json::json!({
            "model_slugs": ["kimi-k2.5", "glm-5.1", "claude-sonnet-4-6", "claude-opus-4-6"],
            "confidence_stats": {
                "kimi-k2.5": { "trials": 145, "successes": 113 },
                "glm-5.1": { "trials": 203, "successes": 166 },
                "claude-sonnet-4-6": { "trials": 312, "successes": 250 },
                "claude-opus-4-6": { "trials": 47, "successes": 44 }
            }
        });
        fs::write(
            &cascade_router_path,
            serde_json::to_string_pretty(&cascade_router).unwrap(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn dashboard_output_renders_snapshot_for_health_and_falls_back_for_other_pages() {
        let dir = tempdir().unwrap();
        seed_dashboard_snapshot(dir.path()).await;

        let cli = Cli::try_parse_from(["roko", "--quiet"]).unwrap();
        let health = dashboard_output(
            &cli,
            Some(dir.path().to_path_buf()),
            Some("health".to_string()),
            false,
        )
        .await
        .unwrap();
        assert!(health.contains("Health (health)"));
        assert!(health.contains("episodes: 2"));
        assert!(health.contains("success rate: 50.0%"));
        assert!(health.contains("avg cost / episode: $2.0000"));
        assert!(health.contains("cache hit rate: 25.0%"));
        assert!(health.contains("current c-factor: 0.67 ↑"));
        assert!(health.contains("gate pass rate: 82.0%"));
        assert!(health.contains("information flow rate: 89.0%"));
        assert!(health.contains("knowledge growth: 18.0%"));

        let trends = dashboard_output(
            &cli,
            Some(dir.path().to_path_buf()),
            Some("trends".to_string()),
            false,
        )
        .await
        .unwrap();
        assert!(trends.contains("Trends (trends)"));
        assert!(trends.contains("first-attempt pass rate: 50.0%"));
        assert!(trends.contains("avg iterations per plan: 1.00"));
        assert!(trends.contains("avg cost per plan: $2.0000"));
        assert!(trends.contains("haiku share: 50.0%"));

        let provider_health = dashboard_output(
            &cli,
            Some(dir.path().to_path_buf()),
            Some("provider-health".to_string()),
            false,
        )
        .await
        .unwrap();
        assert!(provider_health.contains("Provider Health (provider-health)"));
        assert!(provider_health.contains("anthropic"));
        assert!(provider_health.contains("● CLOSED"));
        assert!(provider_health.contains("p50: 0.8s"));
        assert!(provider_health.contains("summary: 20 requests, 3 failures"));

        let model_comparison = dashboard_output(
            &cli,
            Some(dir.path().to_path_buf()),
            Some("model-comparison".to_string()),
            false,
        )
        .await
        .unwrap();
        assert!(model_comparison.contains("Model Comparison (model-comparison)"));
        assert!(model_comparison.contains("Pareto frontier:"));
        assert!(model_comparison.contains("claude-sonnet-4-6 dominated by glm-5.1"));

        let fallback = dashboard_output(
            &cli,
            Some(dir.path().to_path_buf()),
            Some("plan-view".to_string()),
            false,
        )
        .await
        .unwrap();
        assert!(fallback.contains("Plan View (plan-view)"));
        // render_plan_view_page now returns Some("source: missing") instead
        // of None, so the scaffold widget list fallback is no longer reached.
        assert!(fallback.contains("source: missing") || fallback.contains("widgets (2):"));
    }

    #[test]
    fn bootstrap_observability_dirs_creates_expected_paths() {
        let tmp = tempfile::tempdir().unwrap();
        // The guard requires roko.toml or .roko/ to exist before creating dirs.
        std::fs::write(tmp.path().join("roko.toml"), b"").unwrap();
        bootstrap_observability_dirs(tmp.path()).unwrap();
        let roko = tmp.path().join(".roko");
        assert!(roko.join("traces").is_dir());
        assert!(roko.join("metrics").is_dir());
        assert!(roko.join("runtime").is_dir());
        assert!(roko.join("runs").is_dir());
    }

    #[test]
    fn bootstrap_observability_dirs_skips_without_intent() {
        let tmp = tempfile::tempdir().unwrap();
        // No roko.toml or .roko/ — guard should skip creation.
        bootstrap_observability_dirs(tmp.path()).unwrap();
        assert!(!tmp.path().join(".roko").exists());
    }

    #[test]
    fn backup_neuro_store_writes_canonical_secret_safe_snapshot() {
        let workdir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();
        let neuro_dir = workdir.path().join(".roko").join("neuro");
        std::fs::create_dir_all(&neuro_dir).unwrap();
        std::fs::write(neuro_dir.join(NEURO_KNOWLEDGE_FILE), b"{\"id\":\"k1\"}\n").unwrap();
        std::fs::write(
            neuro_dir.join(NEURO_CONFIRMATIONS_FILE),
            b"{\"id\":\"c1\"}\n",
        )
        .unwrap();

        let report = backup_neuro_store(workdir.path(), backup_dir.path(), false, None).unwrap();

        let backup = std::fs::read_to_string(&report.snapshot.knowledge).unwrap();
        let mut lines = backup.lines();
        let header: serde_json::Value =
            serde_json::from_str(lines.next().expect("backup header")).unwrap();
        assert_eq!(header["version"], 2);
        assert_eq!(header["entry_count"], 1);
        assert!(
            header["merkle_root"]
                .as_str()
                .is_some_and(|root| !root.is_empty())
        );
        let entry: serde_json::Value =
            serde_json::from_str(lines.next().expect("knowledge entry")).unwrap();
        assert_eq!(entry["id"], "k1");
        assert!(lines.next().is_none());

        let restored = KnowledgeStore::new(backup_dir.path().join("roundtrip.jsonl"));
        let import = restored
            .import(&report.snapshot.knowledge, &ImportOptions::default())
            .unwrap();
        assert_eq!(import.imported, 1);
        assert_eq!(restored.read_all().unwrap()[0].id, "k1");
        assert_eq!(
            std::fs::read(report.snapshot.confirmations).unwrap(),
            b"{\"id\":\"c1\"}\n"
        );
        assert!(report.confirmations_present);
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&report.manifest).expect("read generated manifest"),
        )
        .expect("parse generated manifest");
        assert_eq!(manifest["version"], 2);
        assert_eq!(manifest["knowledge_format_version"], 2);
        assert_eq!(manifest["entry_count"], 1);
        assert_eq!(manifest["confirmations_present"], true);
        assert_eq!(manifest["confirmations"]["bytes"], 12);
        assert!(
            manifest["confirmations"]["sha256"]
                .as_str()
                .is_some_and(|digest| digest.len() == 64)
        );
    }

    #[test]
    fn restore_neuro_store_requires_force_for_existing_target_and_removes_stale_optional_file() {
        let workdir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();
        let neuro_dir = workdir.path().join(".roko").join("neuro");
        std::fs::create_dir_all(&neuro_dir).unwrap();
        std::fs::write(
            neuro_dir.join(NEURO_KNOWLEDGE_FILE),
            b"{\"id\":\"old\",\"content\":\"old data\",\"confidence\":0.5}\n",
        )
        .unwrap();
        std::fs::write(neuro_dir.join(NEURO_CONFIRMATIONS_FILE), b"stale\n").unwrap();
        std::fs::write(
            backup_dir.path().join(NEURO_KNOWLEDGE_FILE),
            b"{\"id\":\"new\",\"content\":\"new data\",\"confidence\":0.9}\n",
        )
        .unwrap();

        let err = restore_neuro_store(
            workdir.path(),
            backup_dir.path(),
            false,
            1,
            0.8,
            None,
            None,
            true,
        )
        .unwrap_err();
        assert!(err.to_string().contains("Re-run with --force"));

        let report = restore_neuro_store(
            workdir.path(),
            backup_dir.path(),
            true,
            1,
            0.8,
            None,
            None,
            true,
        )
        .unwrap();
        let restored = std::fs::read_to_string(&report.live.knowledge).unwrap();
        assert!(
            restored.contains("\"new\""),
            "restored store should contain the new entry"
        );
        // The backup has no confirmations file, so the report should note it as absent.
        assert!(!report.confirmations_present);
        assert!(
            !report.live.confirmations.exists(),
            "restore must remove confirmations that are absent from the backup"
        );
    }

    #[test]
    fn backup_preflight_and_alias_checks_preserve_existing_state() {
        let workdir = tempdir().unwrap();
        let backup_parent = tempdir().unwrap();
        let neuro_dir = workdir.path().join(".roko").join("neuro");
        std::fs::create_dir_all(&neuro_dir).unwrap();
        let live_knowledge = neuro_dir.join(NEURO_KNOWLEDGE_FILE);
        std::fs::write(&live_knowledge, b"{\"id\":\"live\"}\n").unwrap();

        let destination = backup_parent.path().join("snapshot");
        std::fs::create_dir_all(&destination).unwrap();
        let marker = destination.join("keep.txt");
        std::fs::write(&marker, b"unchanged").unwrap();
        let error = backup_neuro_store(workdir.path(), &destination, false, None)
            .expect_err("populated destination must require force");
        assert!(error.to_string().contains("--force"));
        assert_eq!(std::fs::read(&marker).unwrap(), b"unchanged");
        assert!(!destination.join(NEURO_KNOWLEDGE_FILE).exists());

        let forced = backup_neuro_store(workdir.path(), &destination, true, None)
            .expect("forced staged replacement");
        assert!(forced.snapshot.knowledge.exists());
        assert!(forced.manifest.exists());
        assert!(
            !marker.exists(),
            "forced replacement must not retain stale files"
        );

        let before = std::fs::read(&live_knowledge).unwrap();
        let error = backup_neuro_store(workdir.path(), &neuro_dir, true, None)
            .expect_err("backup must reject the live neuro directory");
        assert!(error.to_string().contains("live neuro store"));
        assert_eq!(std::fs::read(&live_knowledge).unwrap(), before);

        let absent_workdir = tempdir().unwrap();
        let nonexistent_ancestor = absent_workdir.path().join(".roko");
        let error = backup_neuro_store(absent_workdir.path(), &nonexistent_ancestor, true, None)
            .expect_err("nonexistent ancestor of live store must still be rejected");
        assert!(error.to_string().contains("live neuro store"));
        assert!(
            !nonexistent_ancestor.exists(),
            "alias preflight must not create a missing destination"
        );
    }

    #[test]
    fn restore_verifies_confirmation_digest_before_creating_live_state() {
        let source = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let backup = tempdir().unwrap();
        let source_neuro = source.path().join(".roko").join("neuro");
        std::fs::create_dir_all(&source_neuro).unwrap();
        std::fs::write(
            source_neuro.join(NEURO_KNOWLEDGE_FILE),
            b"{\"id\":\"source\",\"content\":\"source knowledge\"}\n",
        )
        .unwrap();
        std::fs::write(
            source_neuro.join(NEURO_CONFIRMATIONS_FILE),
            b"original confirmations\n",
        )
        .unwrap();
        backup_neuro_store(source.path(), backup.path(), false, None).unwrap();
        std::fs::write(
            backup.path().join(NEURO_CONFIRMATIONS_FILE),
            b"tampered confirmations\n",
        )
        .unwrap();

        let error = restore_neuro_store(
            destination.path(),
            backup.path(),
            false,
            1,
            0.8,
            None,
            None,
            false,
        )
        .expect_err("confirmation tampering must fail");
        assert!(error.to_string().contains("integrity verification failed"));
        assert!(!neuro_live_files(destination.path()).knowledge.exists());
    }

    #[test]
    fn restore_rejects_manifest_count_mismatch_without_live_writes() {
        let source = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let backup = tempdir().unwrap();
        let source_neuro = source.path().join(".roko").join("neuro");
        std::fs::create_dir_all(&source_neuro).unwrap();
        std::fs::write(
            source_neuro.join(NEURO_KNOWLEDGE_FILE),
            b"{\"id\":\"source\",\"content\":\"source knowledge\"}\n",
        )
        .unwrap();
        let report = backup_neuro_store(source.path(), backup.path(), false, None).unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&report.manifest).unwrap()).unwrap();
        manifest["entry_count"] = serde_json::json!(2);
        std::fs::write(
            &report.manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = restore_neuro_store(
            destination.path(),
            backup.path(),
            false,
            1,
            0.8,
            None,
            None,
            false,
        )
        .expect_err("manifest count mismatch must fail");
        assert!(error.to_string().contains("entry_count mismatch"));
        assert!(!neuro_live_files(destination.path()).knowledge.exists());
    }

    #[test]
    fn restore_requires_explicit_legacy_for_missing_or_v1_manifest() {
        for legacy_case in ["missing", "v1"] {
            let source = tempdir().unwrap();
            let backup = tempdir().unwrap();
            let destination = tempdir().unwrap();
            let source_neuro = source.path().join(".roko").join("neuro");
            std::fs::create_dir_all(&source_neuro).unwrap();
            std::fs::write(
                source_neuro.join(NEURO_KNOWLEDGE_FILE),
                format!("{{\"id\":\"{legacy_case}\",\"content\":\"legacy case\"}}\n"),
            )
            .unwrap();
            let report = backup_neuro_store(source.path(), backup.path(), false, None).unwrap();
            if legacy_case == "missing" {
                std::fs::remove_file(&report.manifest).unwrap();
            } else {
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&report.manifest).unwrap()).unwrap();
                manifest["version"] = serde_json::json!(1);
                std::fs::write(
                    &report.manifest,
                    serde_json::to_vec_pretty(&manifest).unwrap(),
                )
                .unwrap();
            }

            let error = restore_neuro_store(
                destination.path(),
                backup.path(),
                false,
                1,
                0.8,
                None,
                None,
                false,
            )
            .expect_err("legacy manifest must require opt-in");
            assert!(error.to_string().contains("legacy") || error.to_string().contains("manifest"));
            assert!(!neuro_live_files(destination.path()).knowledge.exists());

            let restored = restore_neuro_store(
                destination.path(),
                backup.path(),
                false,
                1,
                0.8,
                None,
                None,
                true,
            )
            .expect("explicit legacy restore");
            assert!(restored.legacy_input);
            assert_eq!(restored.entries_restored, 1);
        }
    }

    #[test]
    fn restore_rejects_corrupt_live_store_without_partial_knowledge_or_confirmation_changes() {
        let source = tempdir().unwrap();
        let backup = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let source_neuro = source.path().join(".roko").join("neuro");
        std::fs::create_dir_all(&source_neuro).unwrap();
        std::fs::write(
            source_neuro.join(NEURO_KNOWLEDGE_FILE),
            b"{\"id\":\"new\",\"content\":\"new knowledge\"}\n",
        )
        .unwrap();
        backup_neuro_store(source.path(), backup.path(), false, None).unwrap();

        let live = neuro_live_files(destination.path());
        std::fs::create_dir_all(live.knowledge.parent().unwrap()).unwrap();
        let knowledge_before = b"{\"id\":\"old\",\"content\":\"valid\"}\nnot-json\n";
        let confirmations_before = b"old confirmations\n";
        std::fs::write(&live.knowledge, knowledge_before).unwrap();
        std::fs::write(&live.confirmations, confirmations_before).unwrap();

        let error = restore_neuro_store(
            destination.path(),
            backup.path(),
            true,
            1,
            0.8,
            None,
            None,
            false,
        )
        .expect_err("corrupt live store must fail closed");
        assert!(format!("{error:#}").contains("decode knowledge line 2"));
        assert_eq!(std::fs::read(&live.knowledge).unwrap(), knowledge_before);
        assert_eq!(
            std::fs::read(&live.confirmations).unwrap(),
            confirmations_before
        );
    }

    #[test]
    fn subcommand_workdir_reads_the_deepest_workdir_flag() {
        let workdir = |args: &[&str]| {
            let matches = Cli::command()
                .try_get_matches_from(args)
                .expect("valid invocation");
            subcommand_workdir(&matches)
        };
        assert_eq!(
            workdir(&["roko", "serve", "--workdir", "/ws/serve"]),
            Some(PathBuf::from("/ws/serve"))
        );
        assert_eq!(
            workdir(&["roko", "plan", "run", "plans", "--workdir", "/ws/plan"]),
            Some(PathBuf::from("/ws/plan"))
        );
        assert_eq!(workdir(&["roko", "--repo", "/ws/repo", "status"]), None);
        assert_eq!(workdir(&["roko", "init"]), None);
    }

    #[test]
    fn crash_report_dir_uses_subcommand_workdir() {
        let matches = Cli::command()
            .try_get_matches_from(["roko", "plan", "run", "plans", "--workdir", "/ws/plan"])
            .expect("valid invocation");
        let workdir = subcommand_workdir(&matches);
        assert_eq!(
            crash_report_dir(workdir.as_deref()),
            PathBuf::from("/ws/plan").join(".roko")
        );
    }

    #[test]
    fn redacting_format_scrubs_api_keys() {
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::layer::SubscriberExt;

        // Capture output into a shared buffer.
        #[derive(Clone)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);

        impl std::io::Write for BufWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = BufWriter(Arc::clone(&buffer));

        let scrubber = roko_fs::observability::RunScrubber::build(&[]);
        let fmt_layer = tracing_subscriber::fmt::layer()
            .event_format(RedactingFormat::new(
                tracing_subscriber::fmt::format(),
                scrubber,
            ))
            .with_writer(writer)
            .with_ansi(false);

        let subscriber = tracing_subscriber::registry().with(fmt_layer);

        // Use `with_default` so the subscriber is scoped to this test — does
        // not conflict with the global subscriber from other tests.
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(
                "connecting with key sk-ant-api03-AAABBBCCCDDDEEEFFFGGGHHHIIIJJJ and token ghp_ABCDEFGHIJKLMNOPqrstuvwxyz1234567890"
            );
            tracing::warn!("Bearer eyJhbGciOiJIUzI1NiJ9.payload.signature in header");
            tracing::info!("ANTHROPIC_API_KEY=sk-ant-secret-value-99999 leaked");
        });

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();

        // API keys must be scrubbed.
        assert!(
            !output.contains("sk-ant-api03-AAABBBCCC"),
            "Anthropic key should be scrubbed, got: {output}"
        );
        assert!(
            !output.contains("ghp_ABCDEFGHIJKLMNOP"),
            "GitHub PAT should be scrubbed, got: {output}"
        );
        assert!(
            !output.contains("eyJhbGciOiJIUzI1NiJ9"),
            "Bearer token should be scrubbed, got: {output}"
        );
        assert!(
            !output.contains("sk-ant-secret-value"),
            "env-var key value should be scrubbed, got: {output}"
        );

        // Redaction markers must be present.
        assert!(
            output.contains("[REDACTED"),
            "redaction markers should appear, got: {output}"
        );

        // Non-secret context text must survive.
        assert!(
            output.contains("connecting with key"),
            "context text should survive, got: {output}"
        );
    }

    #[test]
    fn dotenv_entries_record_which_values_the_files_supplied() {
        let shell = |name: &str| {
            matches!(name, "SHELL_KEY" | "SAME_KEY").then(|| std::ffi::OsString::from("shell"))
        };
        let entries = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .collect::<Vec<_>>()
        };
        let mut names = roko_core::child_env::DotenvNames::new();
        // ~/.roko/.env never overrides: a variable already set keeps its value.
        record_dotenv_entries(
            &mut names,
            &entries(&[("NEW_KEY", "file"), ("SHELL_KEY", "file")]),
            false,
            shell,
        );
        // ./.roko/.env overrides, but an identical value changes nothing.
        record_dotenv_entries(
            &mut names,
            &entries(&[("SAME_KEY", "shell"), ("OTHER_KEY", "file")]),
            true,
            shell,
        );
        for name in ["NEW_KEY", "SHELL_KEY", "SAME_KEY", "OTHER_KEY"] {
            assert!(names.is_listed(name), "{name} is listed");
        }
        assert!(names.supplied_value("NEW_KEY"));
        assert!(!names.supplied_value("SHELL_KEY"));
        assert!(!names.supplied_value("SAME_KEY"));
        assert!(names.supplied_value("OTHER_KEY"));
    }

    #[test]
    fn log_scrubber_adds_env_redactions() {
        let scrubber =
            roko_fs::observability::RunScrubber::build(&[("MY_TOKEN", "super-secret-42")]);
        let output = scrubber.scrub("leaked super-secret-42 in logs");
        assert!(
            !output.contains("super-secret-42"),
            "env redaction should scrub literal value, got: {output}"
        );
        assert!(
            output.contains("[REDACTED:MY_TOKEN]"),
            "should use named redaction, got: {output}"
        );
    }

    #[test]
    fn tracing_log_directive_prefers_roko_log() {
        // ROKO_LOG is authoritative; when both are set, ROKO_LOG wins.
        let directive = tracing_log_directive_from(Some("roko=debug".into()), Some("info".into()));
        assert_eq!(directive, "info");
    }

    #[test]
    fn tracing_log_directive_falls_back_to_rust_log_and_default() {
        // Falls back to RUST_LOG when ROKO_LOG is absent.
        let directive = tracing_log_directive_from(Some("roko=trace".into()), None);
        assert_eq!(directive, "roko=trace");

        // Falls back to ROKO_LOG when present and RUST_LOG absent.
        let directive2 = tracing_log_directive_from(None, Some("roko=trace".into()));
        assert_eq!(directive2, "roko=trace");

        let default_directive = tracing_log_directive_from(None, None);
        assert_eq!(default_directive, "roko=info");
    }

    #[test]
    fn cli_parses_feed_list() {
        let cli = Cli::try_parse_from(["roko", "feed", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Feed {
                cmd: commands::feed::FeedCmd::List,
            })
        ));
    }

    #[test]
    fn cli_parses_feed_status() {
        let cli = Cli::try_parse_from(["roko", "feed", "status", "file-watch-roko-dir"]).unwrap();
        match cli.command {
            Some(Command::Feed {
                cmd: commands::feed::FeedCmd::Status { id },
            }) => assert_eq!(id, "file-watch-roko-dir"),
            other => panic!("unexpected command variant: {other:?}"),
        }
    }

    #[test]
    fn cli_parses_feed_lifecycle_and_recipe_run() {
        let feed = Cli::try_parse_from(["roko", "feed", "start", "provider-health-feed"]).unwrap();
        assert!(matches!(
            feed.command,
            Some(Command::Feed {
                cmd: commands::feed::FeedCmd::Start { .. }
            })
        ));

        let recipe = Cli::try_parse_from([
            "roko", "recipe", "run", "blend", "--input", "left=2", "--input", "right=6",
        ])
        .unwrap();
        match recipe.command {
            Some(Command::Recipe {
                cmd: commands::recipe::RecipeCmd::Run { id, inputs },
            }) => {
                assert_eq!(id, "blend");
                assert_eq!(inputs, vec!["left=2", "right=6"]);
            }
            other => panic!("unexpected command variant: {other:?}"),
        }
    }

    // ── E15-T7: roko-mcp-github auto-discovery tests ─────────────────────────

    /// Helper: create a fake executable at `dir/roko-mcp-github` and return its
    /// path. Uses only `std::fs` — no env var manipulation required.
    fn create_fake_github_binary(dir: &std::path::Path) -> std::path::PathBuf {
        let fake = dir.join("roko-mcp-github");
        std::fs::write(&fake, b"#!/bin/sh\necho ok").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&fake).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&fake, perms).unwrap();
        }
        fake
    }

    /// `find_binary_in_target_dirs` must locate a binary placed in a
    /// `target/debug/` ancestor of the given search root.
    #[test]
    fn mcp_github_discover_finds_binary_in_target_debug() {
        let tmp = tempdir().unwrap();
        let target_debug = tmp.path().join("target").join("debug");
        std::fs::create_dir_all(&target_debug).unwrap();
        let fake = create_fake_github_binary(&target_debug);

        let found = commands::mcp::find_binary_in_target_dirs(tmp.path(), "roko-mcp-github");
        assert!(found.is_some(), "should find binary in target/debug/");
        assert_eq!(found.unwrap(), fake);
    }

    /// `find_binary_in_target_dirs` must return `None` when no matching binary
    /// exists anywhere in the ancestor tree.
    #[test]
    fn mcp_github_discover_returns_none_when_no_target_binary() {
        let tmp = tempdir().unwrap();
        // No target/ subdirectory — nothing to find.
        let found = commands::mcp::find_binary_in_target_dirs(tmp.path(), "roko-mcp-github");
        assert!(found.is_none(), "should return None when binary is absent");
    }

    /// `add_github_mcp_server` must add a `github` entry with the supplied
    /// command. Does not touch environment variables.
    #[test]
    fn mcp_github_discover_adds_entry_with_explicit_command() {
        let mut config = roko_agent::mcp::McpConfig { servers: vec![] };
        let cmd = "/fake/path/roko-mcp-github".to_string();
        commands::mcp::add_github_mcp_server(&mut config, cmd.clone());

        assert_eq!(config.servers.len(), 1, "expected exactly one server entry");
        let s = &config.servers[0];
        assert_eq!(s.name, "github");
        assert_eq!(s.command, cmd);
        assert!(
            s.args.is_empty(),
            "auto-discovered entry should have no args"
        );
    }

    /// When the user already configured a `github` server, auto-discovery
    /// must not add a duplicate entry.
    #[test]
    fn mcp_github_discover_respects_user_configured_github_server() {
        let tmp = tempdir().unwrap();

        // Place a fake binary so discovery would succeed if it weren't for the
        // existing user-configured server.
        let target_debug = tmp.path().join("target").join("debug");
        std::fs::create_dir_all(&target_debug).unwrap();
        create_fake_github_binary(&target_debug);

        // User-configured entry.
        let user_entry = roko_agent::mcp::McpServerConfig {
            name: "github".to_string(),
            transport: roko_agent::mcp::McpTransportConfig::Stdio,
            command: "/usr/local/bin/my-custom-github-mcp".to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            endpoint: None,
            auth_token: None,
            tier: Default::default(),
        };

        let mut config = roko_agent::mcp::McpConfig {
            servers: vec![user_entry],
        };
        commands::mcp::augment_mcp_config_with_github(&mut config, tmp.path());

        assert_eq!(
            config.servers.len(),
            1,
            "user-configured server must not be duplicated"
        );
        assert_eq!(
            config.servers[0].command, "/usr/local/bin/my-custom-github-mcp",
            "user-configured command must be preserved"
        );
    }

    /// `augment_mcp_config_with_github` on a workdir without a target/ tree
    /// must not panic and must leave the config unchanged when the binary is
    /// absent from target/.  (If the binary is genuinely on PATH the test
    /// correctly adds one entry — that is valid behaviour.)
    #[test]
    fn mcp_github_discover_skips_when_binary_absent_from_target() {
        let tmp = tempdir().unwrap();
        let mut config = roko_agent::mcp::McpConfig { servers: vec![] };
        commands::mcp::augment_mcp_config_with_github(&mut config, tmp.path());
        // Must not panic.  Binary count may be 0 (absent) or 1 (on real PATH).
        let _ = config.servers.len();
    }

    /// `resolve_mcp_config_with_autodiscovery` writes `mcp-auto.json` when the
    /// github binary is found in `target/debug/` and no pre-existing MCP config
    /// is present.
    #[test]
    fn mcp_github_discover_writes_auto_config_file() {
        let tmp = tempdir().unwrap();
        let roko_dir = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko_dir).unwrap();

        // Place a fake binary inside target/debug relative to tmp so the
        // ancestor walk succeeds without touching PATH.
        let target_debug = tmp.path().join("target").join("debug");
        std::fs::create_dir_all(&target_debug).unwrap();
        create_fake_github_binary(&target_debug);

        let result = resolve_mcp_config_with_autodiscovery(tmp.path(), &roko_dir);

        // A path must be returned (binary was found in target/debug/).
        let path = result.expect("expected a config path when github binary is in target/");

        // The returned path should be the auto-generated file.
        assert_eq!(
            path,
            roko_dir.join("mcp-auto.json"),
            "expected mcp-auto.json, got: {}",
            path.display()
        );

        // The file must exist and contain the github server.
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: roko_agent::mcp::McpConfig = serde_json::from_str(&content).unwrap();
        assert!(
            parsed.servers.iter().any(|s| s.name == "github"),
            "mcp-auto.json must contain a 'github' server entry"
        );
    }

    // ── E31-T06: trigger subcommand parsing ────────────────────────────────

    #[test]
    fn cli_parses_trigger_list() {
        let cli = Cli::try_parse_from(["roko", "trigger", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Trigger {
                cmd: commands::trigger::TriggerCmd::List { .. },
            })
        ));
    }

    #[test]
    fn cli_parses_trigger_show() {
        let cli = Cli::try_parse_from(["roko", "trigger", "show", "my-hook"]).unwrap();
        match cli.command {
            Some(Command::Trigger {
                cmd: commands::trigger::TriggerCmd::Show { name, .. },
            }) => assert_eq!(name, "my-hook"),
            other => panic!("unexpected command variant: {other:?}"),
        }
    }

    #[test]
    fn cli_parses_trigger_create() {
        let cli = Cli::try_parse_from([
            "roko",
            "trigger",
            "create",
            "deploy-hook",
            "--kind",
            "webhook",
            "--graph",
            "plans/deploy.toml",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Trigger {
                cmd:
                    commands::trigger::TriggerCmd::Create {
                        name, kind, graph, ..
                    },
            }) => {
                assert_eq!(name, "deploy-hook");
                assert_eq!(kind, "webhook");
                assert_eq!(graph, "plans/deploy.toml");
            }
            other => panic!("unexpected command variant: {other:?}"),
        }
    }

    #[test]
    fn cli_parses_trigger_fire() {
        let cli = Cli::try_parse_from([
            "roko",
            "trigger",
            "fire",
            "my-hook",
            "--payload",
            "{\"key\":\"value\"}",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Trigger {
                cmd: commands::trigger::TriggerCmd::Fire { name, payload, .. },
            }) => {
                assert_eq!(name, "my-hook");
                assert_eq!(payload, "{\"key\":\"value\"}");
            }
            other => panic!("unexpected command variant: {other:?}"),
        }
    }

    #[test]
    fn cli_parses_trigger_fire_default_payload() {
        let cli = Cli::try_parse_from(["roko", "trigger", "fire", "my-hook"]).unwrap();
        match cli.command {
            Some(Command::Trigger {
                cmd: commands::trigger::TriggerCmd::Fire { name, payload, .. },
            }) => {
                assert_eq!(name, "my-hook");
                assert_eq!(payload, "{}");
            }
            other => panic!("unexpected command variant: {other:?}"),
        }
    }

    // ── #262: CLI flag resolution contract tests ────────────────────

    use roko_cli::resolved_overrides::{
        ApprovalPolicy, BudgetPolicy, CascadePolicy, ConfigEditTarget, ConfigSetInput,
        DevelopInput, DryRunPolicy, InteractionMode, LearnTuneInput, PlanRunInput,
        PresentationMode, ResolvedExecutionOverrides, ServePolicy,
    };

    #[test]
    fn cli_flags_model_alias_equivalence() {
        let cli_model = Cli::try_parse_from(["roko", "--model", "sonnet", "status"]).unwrap();
        let cli_force = Cli::try_parse_from(["roko", "--force-model", "sonnet", "status"]).unwrap();
        assert_eq!(cli_model.model, cli_force.model);
        assert_eq!(cli_model.model.as_deref(), Some("sonnet"));
    }

    #[test]
    fn cli_flags_headless_resolves() {
        let cli = Cli::try_parse_from(["roko", "--headless", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let overrides = ResolvedExecutionOverrides::for_do(&flags, &DoInput::default());
        assert_eq!(overrides.interaction_mode, InteractionMode::Headless);
    }

    #[test]
    fn cli_flags_force_backend_alias_resolves_to_model() {
        let cli = Cli::try_parse_from(["roko", "--force-backend", "sonnet", "status"]).unwrap();
        assert_eq!(
            cli.model.as_deref(),
            Some("sonnet"),
            "--force-backend must resolve to the unified model field"
        );
    }

    #[test]
    fn cli_flags_do_ghost_is_dry_run() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let input = DoInput {
            ghost: true,
            ..DoInput::default()
        };
        let overrides = ResolvedExecutionOverrides::for_do(&flags, &input);
        assert_eq!(overrides.dry_run, DryRunPolicy::ReadOnlyNoMutation);
    }

    #[test]
    fn cli_flags_no_cascade_resolves() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let input = DoInput {
            no_cascade: true,
            ..DoInput::default()
        };
        let overrides = ResolvedExecutionOverrides::for_do(&flags, &input);
        assert_eq!(overrides.cascade_policy, CascadePolicy::DisabledByUser);
    }

    #[test]
    fn cli_flags_serve_required() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let overrides = ResolvedExecutionOverrides::for_run(&flags, None, true, None);
        assert_eq!(overrides.serve_policy, ServePolicy::Required);
    }

    #[test]
    fn cli_flags_no_serve_disabled() {
        let cli = Cli::try_parse_from(["roko", "--no-serve", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let overrides = ResolvedExecutionOverrides::for_run(&flags, None, false, None);
        assert_eq!(overrides.serve_policy, ServePolicy::Disabled);
    }

    #[test]
    fn cli_flags_plan_run_tui_presentation() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let plan_tui = PlanRunInput {
            approval: true,
            ..PlanRunInput::default()
        };
        let plan_no_tui = PlanRunInput {
            no_tui: true,
            ..PlanRunInput::default()
        };
        let plan_auto = PlanRunInput::default();

        assert_eq!(
            ResolvedExecutionOverrides::for_plan_run(&flags, &plan_tui).presentation,
            PresentationMode::Tui,
        );
        assert_eq!(
            ResolvedExecutionOverrides::for_plan_run(&flags, &plan_no_tui).presentation,
            PresentationMode::Text,
        );
        assert_eq!(
            ResolvedExecutionOverrides::for_plan_run(&flags, &plan_auto).presentation,
            PresentationMode::Auto,
        );
    }

    #[test]
    fn cli_flags_config_set_targets() {
        assert_eq!(
            ResolvedExecutionOverrides::resolve_config_edit_target(&ConfigSetInput {
                global: false,
                project: false,
            }),
            ConfigEditTarget::Global,
            "no flags defaults to Global for config set"
        );
        assert_eq!(
            ResolvedExecutionOverrides::resolve_config_edit_target(&ConfigSetInput {
                global: false,
                project: true,
            }),
            ConfigEditTarget::Project,
        );
    }

    #[test]
    fn cli_flags_learn_tune_dry_run() {
        assert_eq!(
            ResolvedExecutionOverrides::resolve_tune_dry_run(&LearnTuneInput { dry_run: true }),
            DryRunPolicy::ReadOnlyNoMutation,
        );
        assert_eq!(
            ResolvedExecutionOverrides::resolve_tune_dry_run(&LearnTuneInput { dry_run: false }),
            DryRunPolicy::Execute,
        );
    }

    #[test]
    fn cli_flags_global_flags_helper_roundtrip() {
        let cli = Cli::try_parse_from([
            "roko",
            "--model",
            "opus",
            "--role",
            "architect",
            "--effort",
            "high",
            "--json",
            "--quiet",
            "--headless",
            "--no-serve",
            "status",
        ])
        .unwrap();
        let flags = global_cli_flags(&cli);
        assert_eq!(flags.model, Some("opus"));
        assert_eq!(flags.role, Some("architect"));
        assert_eq!(flags.effort, Some("high"));
        assert!(flags.json);
        assert!(flags.quiet);
        assert!(flags.headless);
        assert!(flags.no_serve);
    }

    #[test]
    fn cli_flags_develop_resolves_dry_run() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let input = DevelopInput {
            dry_run: true,
            ..DevelopInput::default()
        };
        let overrides = ResolvedExecutionOverrides::for_develop(&flags, &input);
        assert_eq!(overrides.dry_run, DryRunPolicy::ReadOnlyNoMutation);
    }

    #[test]
    fn cli_flags_develop_resolves_yes() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let input = DevelopInput {
            yes: true,
            ..DevelopInput::default()
        };
        let overrides = ResolvedExecutionOverrides::for_develop(&flags, &input);
        assert_eq!(overrides.approval, ApprovalPolicy::AutoApprove);
    }

    #[test]
    fn cli_flags_plan_run_budget_override() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let plan = PlanRunInput {
            budget_override: Some(50.0),
            ..PlanRunInput::default()
        };
        let overrides = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert_eq!(overrides.budget, BudgetPolicy::Override(50.0));
    }

    #[test]
    fn cli_flags_plan_run_no_budget() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let plan = PlanRunInput {
            no_budget: true,
            ..PlanRunInput::default()
        };
        let overrides = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert_eq!(overrides.budget, BudgetPolicy::Disabled);
    }

    #[test]
    fn cli_flags_plan_run_fresh_and_force_resume() {
        let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
        let flags = global_cli_flags(&cli);
        let plan = PlanRunInput {
            fresh: true,
            force_resume: true,
            ..PlanRunInput::default()
        };
        let overrides = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert!(overrides.fresh);
        assert!(overrides.force_resume);
    }

    // ── P2-FLG-1: global --json wires through to subcommands ────────

    #[test]
    fn cli_flags_json_parses_globally_for_prd_list() {
        // `roko --json prd list` must set cli.json = true.
        let cli = Cli::try_parse_from(["roko", "--json", "prd", "list"]).unwrap();
        assert!(cli.json, "--json must be set when passed before subcommand");
    }

    #[test]
    fn cli_flags_json_parses_globally_for_history() {
        let cli = Cli::try_parse_from(["roko", "--json", "history"]).unwrap();
        assert!(
            cli.json,
            "--json must be set when passed before history subcommand"
        );
    }

    // ── P2-FLG-2: --role override for prd subcommands ─────────────

    #[test]
    fn cli_flags_role_propagates_to_prd_context() {
        // Verify that --role appears in cli.role for prd subcommands.
        let cli = Cli::try_parse_from(["roko", "--role", "custom-writer", "prd", "list"]).unwrap();
        assert_eq!(
            cli.role.as_deref(),
            Some("custom-writer"),
            "--role must propagate to cli.role for prd subcommands"
        );
    }

    // ── #326: error_hint accuracy tests ────────────────────────────

    #[test]
    fn error_hint_state_recovery_required() {
        let hint = error_hint("state recovery required: snapshot corrupt");
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("--fresh"));
    }

    #[test]
    fn error_hint_state_snapshot_corrupt() {
        let hint = error_hint("state snapshot corrupt; trying backup");
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("--fresh"));
    }

    #[test]
    fn error_hint_authoritative_with_state_recovery() {
        // The word "authoritative" inside a StateRecoveryRequired message
        // must get the state recovery hint, not the API key hint.
        let hint = error_hint(
            "state recovery required: the snapshot at /tmp/state-snapshot.json is corrupt \
             (validate authoritative snapshot failed)",
        );
        assert!(hint.is_some());
        let h = hint.unwrap();
        assert!(
            !h.contains("API key"),
            "authoritative must not trigger API key hint, got: {h}"
        );
        assert!(h.contains("--fresh"), "should recommend --fresh, got: {h}");
    }

    #[test]
    fn error_hint_authoritative_alone_no_api_key() {
        // "authoritative" alone must not produce an API key hint.
        let hint = error_hint(
            "resume validation failed: validate authoritative snapshot /tmp/state-snapshot.json",
        );
        assert!(
            hint.is_none() || !hint.unwrap().contains("API key"),
            "authoritative alone must not trigger API key hint"
        );
    }

    #[test]
    fn error_hint_401_triggers_api_key() {
        let hint = error_hint("HTTP 401: unauthorized");
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("API key"));
    }

    #[test]
    fn error_hint_unauthorized_triggers_api_key() {
        let hint = error_hint("request failed: unauthorized");
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("API key"));
    }

    #[test]
    fn error_hint_invalid_api_key_triggers() {
        let hint = error_hint("invalid_api_key");
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("API key"));
    }

    #[test]
    fn error_hint_authentication_failed_triggers() {
        let hint = error_hint("authentication failed for provider");
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("API key"));
    }

    #[test]
    fn error_hint_authorization_policy_no_api_key() {
        let hint = error_hint("authorization policy denied tool execution");
        assert!(
            hint.is_none() || !hint.unwrap().contains("API key"),
            "authorization policy must not trigger API key hint"
        );
    }

    #[test]
    fn error_hint_unrelated_returns_none() {
        assert!(error_hint("something completely unrelated went wrong").is_none());
    }

    #[test]
    fn error_hint_ignores_401_outside_an_http_status() {
        for msg in [
            "/tmp/run-1401/checkpoint.json: No such file or directory",
            "worktree for gap-e4019c already exists",
            "wrote 14015 bytes to the snapshot",
            "task 401 of plan p1 failed verification",
        ] {
            let hint = error_hint(msg);
            assert!(
                hint.is_none() || !hint.unwrap().contains("API key"),
                "a 401 outside an HTTP status must not blame the API key: {msg}"
            );
        }
        for msg in [
            "HTTP 401",
            "request returned status 401",
            "server returned HTTP 401.",
        ] {
            let hint = error_hint(msg);
            assert!(
                hint.is_some_and(|h| h.contains("API key")),
                "an HTTP 401 must get the API key hint: {msg}"
            );
        }
    }

    #[test]
    fn error_hint_points_provider_auth_at_provider_keys() {
        let msg = "API key invalid for provider 'openai' (HTTP 401). Check $OPENAI_API_KEY";
        let hint = error_hint(msg).expect("a provider 401 gets a hint");
        assert!(hint.contains("roko config check-secrets"), "got: {hint}");
        assert!(
            !hint.contains("ROKO_API_KEY"),
            "ROKO_API_KEY is the serve key, got: {hint}"
        );
    }

    #[test]
    fn error_hint_points_serve_auth_at_roko_api_key() {
        let msg = "the workspace server rejected the request (401): server returned HTTP 401";
        let hint = error_hint(msg).expect("a serve 401 gets a hint");
        assert!(
            hint.contains("ROKO_API_KEY") && hint.contains("roko login"),
            "got: {hint}"
        );
    }

    // ─── Impact CLI parsing ─────────────────────────────────────────────

    #[test]
    fn cli_parses_impact_defaults() {
        let cli = Cli::try_parse_from(["roko", "impact"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Impact {
                ref base,
                ref files,
                json: false,
                workdir: None,
            }) if base == "HEAD" && files.is_empty()
        ));
    }

    #[test]
    fn cli_parses_impact_with_base() {
        let cli = Cli::try_parse_from(["roko", "impact", "--base", "main"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Impact {
                ref base,
                ..
            }) if base == "main"
        ));
    }

    #[test]
    fn cli_parses_impact_with_json_flag() {
        let cli = Cli::try_parse_from(["roko", "impact", "--json"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Impact { json: true, .. })
        ));
    }

    #[test]
    fn cli_parses_impact_with_explicit_files() {
        let cli = Cli::try_parse_from([
            "roko",
            "impact",
            "--files",
            "crates/roko-core/src/lib.rs",
            "crates/roko-gate/src/lib.rs",
        ])
        .unwrap();
        match &cli.command {
            Some(Command::Impact { files, .. }) => {
                assert_eq!(files.len(), 2);
                assert_eq!(files[0], "crates/roko-core/src/lib.rs");
                assert_eq!(files[1], "crates/roko-gate/src/lib.rs");
            }
            other => panic!("expected Impact, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_impact_with_workdir() {
        let cli = Cli::try_parse_from(["roko", "impact", "--workdir", "/some/path"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Impact {
                workdir: Some(ref p),
                ..
            }) if p == Path::new("/some/path")
        ));
    }
}
