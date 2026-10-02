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
use commands::backlog::BacklogCmd;
use commands::bench::BenchCmd;
use commands::cache::CacheCmd;
use commands::config_cmd::ConfigCmd;
use commands::experiment::{ExperimentCmd, dispatch_experiment};
use commands::job::JobCmd;
use commands::knowledge::KnowledgeCmd;
use commands::learn::LearnCmd;
use commands::mcp::ConfigMcpCmd;
use commands::plan::PlanCmd;
use commands::research::{ResearchBackend, ResearchCmd, SearchRecency};
use commands::run_index::RunIndexCmd;
use commands::safety::SafetyCmd;
use commands::server::{DaemonCmd, DeployCmd};
use commands::tune::{ConfigPresetCmd, TuneCmd};
use commands::util::{CompletionShell, IndexCmd};
use octocrab::Octocrab;
use octocrab::models::hooks::{Config as HookConfig, ContentType, Hook};
use octocrab::models::webhook_events::WebhookEventType;
use roko_agent::process::{cleanup_orphaned_agents, reap_orphaned_children};
use roko_agent::translate::BackendResponse;
use roko_cli::agent_spawn::{SpawnAgentSpec, spawn_agent_scoped};
use roko_cli::resolved_overrides::{GlobalCliFlags, ResolvedExecutionOverrides, RunInput};
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

/// Size override for `roko run --complexity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RunComplexity {
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

impl RunComplexity {
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
    about = "Roko --- agent toolkit\n\nQuick start: roko setup, roko run \"<task>\", roko status\nRun roko help <command> for details.",
    after_long_help = "\
COMMAND GROUPS:
  Core workflow:     init, run, status, doctor
  Planning:          plan
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
    /// Run a prompt, or a plan directory, through the Graph engine.
    ///
    /// A prompt is sized first. A trivial or simple prompt runs as a one-task
    /// plan written to `.roko/runs/<run-id>/tasks.toml`: one implementer agent
    /// whose verify steps are the workspace gates (`[[gates.rungs]]`, else
    /// `cargo check` / `go build`), with the dispatch, failover, safety, budget,
    /// checkpoints, episodes, and cost records of `roko plan run`. A larger
    /// prompt first gets a plan written to `plans/<slug>/`, which then runs.
    ///
    /// `--plan` always writes the plan first; on a terminal roko shows it and
    /// asks before running it (`--yes` skips the question). `--plan --dry-run`
    /// writes the plan and stops, so you can review or edit it. A plan
    /// directory (`roko run plans/<slug>`) runs as `roko plan run` runs it.
    /// `--serve` and `--share` run the prompt as one task with the control
    /// plane. Exits non-zero when the run fails; `--json` prints the run report.
    #[command(after_help = "\
Examples:
  roko run \"Fix the login bug\"                   Size the prompt, then run it
  roko run --plan \"Add OAuth2 login\"             Write a plan, show it, run it
  roko run --plan --dry-run \"Add OAuth2 login\"   Write the plan and stop
  roko run --dry-run \"Fix the login bug\"         Show how the prompt would run
  roko run plans/add-oauth2-login                Run an existing plan
  roko --json run \"Fix the login bug\"            Print the run report as JSON")]
    Run {
        /// The prompt (quote it), or a plan directory such as `plans/<slug>`.
        #[arg(value_name = "PROMPT_OR_PLAN", required = true, num_args = 1..)]
        prompt: Vec<String>,
        /// Write a plan before running, whatever the prompt's size.
        #[arg(long)]
        plan: bool,
        /// Run nothing: show how the prompt would run, or with `--plan` write
        /// the plan and stop. With a plan directory, preview the plan.
        #[arg(long)]
        dry_run: bool,
        /// Run a `--plan` plan without asking first.
        #[arg(long, short = 'y')]
        yes: bool,
        /// Force the prompt's size instead of detecting it: trivial and simple
        /// run one task, standard and complex write a plan first.
        #[arg(long, value_enum)]
        complexity: Option<RunComplexity>,
        /// Additional context files, directories or globs for the planner.
        #[arg(long = "context", value_name = "PATH")]
        context: Vec<PathBuf>,
        /// Disable cascade routing for this run.
        #[arg(long)]
        no_cascade: bool,
        /// Override the working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Start the HTTP control plane alongside the run (one task).
        #[arg(long)]
        serve: bool,
        /// Generate a shareable URL for the run (one task; starts serve if needed).
        #[arg(long)]
        share: bool,
        /// Override the provider for this run (e.g. anthropic, openai, ollama, moonshot).
        #[arg(long)]
        provider: Option<String>,
        /// Retries after a failed attempt of a task.
        #[arg(long)]
        max_retries: Option<u32>,
        /// With a plan directory: archive old run state and start from scratch.
        #[arg(long)]
        fresh: bool,
        /// With a plan directory: resume from engine state, as `roko plan run
        /// --resume-plan` does (bare: the Graph checkpoints in `.roko/state/graph/`).
        #[arg(
            long = "resume-plan",
            value_name = "PATH",
            num_args = 0..=1,
            default_missing_value = ".roko/state/state-snapshot.json"
        )]
        resume_plan: Option<PathBuf>,
    },
    /// (Removed) Use `roko run "<prompt>"`, or `roko run --plan "<prompt>"`.
    ///
    /// The arguments still parse, so an old script gets the migration error
    /// instead of a usage error.
    #[command(hide = true, alias = "d")]
    Do {
        #[arg(long)]
        plan: bool,
        #[arg(long, value_enum)]
        complexity: Option<RunComplexity>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        workdir: Option<PathBuf>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        ghost: bool,
        #[arg(long)]
        compare: bool,
        #[arg(long = "continue", value_name = "WORK_ID", num_args = 0..=1)]
        r#continue: Option<Option<String>>,
        #[arg(long)]
        no_cascade: bool,
        #[arg(long = "context", value_name = "PATH")]
        context: Vec<PathBuf>,
        #[arg(value_name = "PROMPT")]
        prompt: Vec<String>,
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
  roko show agents --since all      Include agents with no activity in the last 7 days
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
        /// Count recent activity since WHEN: a span such as 24h or 7d, a YYYY-MM-DD date, an
        /// RFC 3339 time, or `all` (default: 7d).
        #[arg(long, value_name = "WHEN")]
        since: Option<String>,
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
        /// With `disk`: remove the leftover attempt checkouts of plans whose
        /// checkpoint succeeded, failed or was cancelled, once untouched for
        /// 7 days, unless they have changes. Their branches are kept.
        #[arg(long)]
        fix: bool,
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
    /// List immune isolation controls and release one, with an audit record.
    #[command(after_help = "\
Examples:
  roko safety controls
  roko safety controls --json
  roko safety release live-a/cli --reason \"blank answer, not tamper\"")]
    Safety {
        #[command(subcommand)]
        cmd: SafetyCmd,
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

    // ── Planning ────────────────────────────────────────────────────
    /// Manage plans (list, show, create, validate, run, generate).
    #[command(visible_alias = "p")]
    Plan {
        #[command(subcommand)]
        cmd: PlanCmd,
    },
    /// (Removed) Plans are the unit of work: `roko run --plan "<prompt>"`, or
    /// `roko plan generate "<prompt>"` and then `roko run plans/<slug>`.
    ///
    /// Any arguments still parse, so an old script gets the migration error
    /// instead of a usage error.
    #[command(hide = true)]
    Prd {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
        args: Vec<String>,
    },

    /// List and audit `tmp/backlog` items against the plan runs on record.
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
  roko resume run_4823                Resume a specific run by ID
  roko resume --max-tasks 2           Resume, running at most 2 tasks per plan at once")]
    Resume {
        /// Run or plan ID to resume (optional — defaults to most recent snapshot).
        run_id: Option<String>,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Maximum concurrent tasks per plan (0 keeps the config/default
        /// value). Any value resumes the run: it is not part of the
        /// checkpoint's identity.
        #[arg(long, default_value_t = 0)]
        max_tasks: usize,
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
            | Self::Budget { .. }
            | Self::Review { .. }
            | Self::Status { .. } => false,
            Self::Run { dry_run, .. } | Self::Regenerate { dry_run, .. } => !dry_run,
            Self::Create { .. }
            | Self::Generate { .. }
            | Self::Prepare { .. }
            | Self::Shorthand(_) => true,
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

/// Report that `roko <command>` was removed, with its replacement, and return
/// the failure exit code. With `--json`, stdout also gets the same facts as one
/// JSON object.
fn removed_command(cli: &Cli, command: &str, message: &str, migration: &str, since: &str) -> i32 {
    eprintln!("error: `roko {command}` was removed. {message}");
    if cli.json {
        let msg = serde_json::json!({
            "error": "command_removed",
            "command": command,
            "migration": migration,
            "deprecated_since": since,
        });
        println!("{}", serde_json::to_string_pretty(&msg).unwrap_or_default());
    }
    EXIT_FAILURE
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

// EventSourcesCmdLegacy, ProviderCmdLegacy, ModelCmdLegacy removed — dispatch goes direct

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
    if roko_agent::provider::error_classify::mentions_http_401(&lower)
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

    None
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
            plan,
            dry_run,
            yes,
            complexity,
            context,
            no_cascade,
            workdir,
            serve,
            share,
            provider,
            max_retries,
            fresh,
            resume_plan,
        } => {
            // Resolve typed overrides before any side effects (#262).
            let _resolved = ResolvedExecutionOverrides::for_run(
                &global_cli_flags(cli),
                &RunInput {
                    dry_run,
                    yes,
                    no_cascade,
                    provider: provider.clone(),
                    context: context.clone(),
                    serve_required: serve || share,
                    max_retries,
                },
            );
            tracing::debug!(?_resolved, "resolved execution overrides for `run`");

            commands::run_cmd::cmd_run(
                cli,
                commands::run_cmd::RunArgs {
                    input: prompt,
                    plan,
                    dry_run,
                    yes,
                    complexity: complexity.map(RunComplexity::into_plan_complexity),
                    context,
                    no_cascade,
                    workdir,
                    serve,
                    share,
                    provider,
                    max_retries,
                    fresh,
                    resume_plan,
                },
            )
            .await
        }
        // `do` and `prd` were folded into `roko run` (tmp/workflow-audit). They
        // still parse for one release and exit with the migration, without any
        // provider, server or git effect. (`develop`, an error since 2026-09-04,
        // is gone: clap reports it as an unknown subcommand.)
        Command::Do { .. } => Ok(removed_command(
            cli,
            "do",
            "Use `roko run \"<prompt>\"`, which sizes the prompt the same way; \
             for a plan first, `roko run --plan \"<prompt>\"`.",
            "roko run \"<prompt>\"",
            "2026-10-02",
        )),
        Command::Prd { .. } => Ok(removed_command(
            cli,
            "prd",
            "Plans are the unit of work: write one from a prompt with \
             `roko run --plan --dry-run \"<idea>\"` or `roko plan generate \"<idea>\"`, \
             then run it with `roko run plans/<slug>`.",
            "roko plan generate \"<idea>\"",
            "2026-10-02",
        )),
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
            since,
        } => {
            // #363: --live is a deprecated alias for --dashboard.
            let use_dashboard = dashboard || live;
            if live && !dashboard {
                eprintln!("warning: --live is deprecated; use --dashboard instead");
            }
            commands::show::cmd_show(
                cli,
                workdir,
                use_dashboard,
                follow,
                serve_url,
                subject,
                since,
            )
            .await
        }
        Command::Doctor {
            subject,
            workdir,
            serve_url,
            fix,
        } => commands::util::cmd_doctor(cli, subject, workdir, serve_url, fix).await,
        Command::Cache { cmd } => commands::cache::cmd_cache(cli, cmd).await,
        Command::RunIndex { cmd } => commands::run_index::cmd_run_index(cli, cmd).await,
        Command::Safety { cmd } => commands::safety::cmd_safety(cli, cmd).await,
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
        Command::Resume {
            run_id,
            workdir,
            max_tasks,
        } => commands::plan::cmd_resume(cli, run_id, workdir, max_tasks).await,
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
    FsObservabilitySinks::for_workdir(workdir).initialize()
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

// Re-export for crate-internal callers (e.g. `crate::resolve_mcp_config_with_autodiscovery`).
pub use commands::mcp::resolve_mcp_config_with_autodiscovery;

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
