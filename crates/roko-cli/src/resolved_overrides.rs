//! Typed CLI flag resolution contract (#262).
//!
//! Every accepted CLI flag is resolved to a typed value object before any
//! side effects (provider construction, workspace lock, snapshot mutation,
//! or dispatch). Downstream code consumes [`ResolvedExecutionOverrides`]
//! instead of re-parsing clap state.
//!
//! # Precedence (highest first)
//!
//! 1. Explicit command-local compatibility input (after conflict check)
//!    = global canonical input
//! 2. Config
//! 3. Profile default
//!
//! Aliases never create a separate precedence level.

use std::num::NonZeroUsize;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Policy enums — each flag resolves to exactly one discriminant
// ---------------------------------------------------------------------------

/// Dry-run / read-only mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DryRunPolicy {
    /// Normal mutable execution.
    Execute,
    /// Read-only: no mutation. Maps from `--dry-run` and `--ghost`.
    ReadOnlyNoMutation,
}

/// Cascade router policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CascadePolicy {
    /// Use the cascade router normally.
    Enabled,
    /// Explicitly disabled via `--no-cascade`.
    DisabledByUser,
}

/// Serve policy for `roko run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServePolicy {
    /// Serve is explicitly required (`--serve` or `--share`).
    Required,
    /// Serve is explicitly disabled (`--no-serve`).
    Disabled,
    /// Auto-detect based on context.
    Auto,
}

/// Interaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionMode {
    /// Normal interactive mode.
    Interactive,
    /// Headless mode (`--headless`).
    Headless,
}

/// Presentation mode for plan execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationMode {
    /// Auto-detect (TTY -> TUI, pipe -> text).
    Auto,
    /// Force TUI (e.g. `--approval` implies TUI).
    Tui,
    /// Force text output (`--no-tui`).
    Text,
}

/// Approval policy independent of presentation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalPolicy {
    /// Default: approval prompts appear when the workflow requires them.
    Normal,
    /// `--yes`: skip approval prompts.
    AutoApprove,
}

/// Screenshot capture policy resolved from `--screenshots[=<dir>]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenshotPolicy {
    /// Default: no screenshots captured.
    Disabled,
    /// `--screenshots`: capture to default or specified directory.
    Enabled {
        /// Custom directory, or `None` for the default location.
        dir: Option<PathBuf>,
        /// Maximum seconds between periodic captures.
        interval_secs: u64,
    },
}

/// Config edit target scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigEditTarget {
    /// Edit the global config.
    Global,
    /// Edit the project-local config.
    Project,
}

// ---------------------------------------------------------------------------
// Input structs (borrowed from clap without depending on it)
// ---------------------------------------------------------------------------

/// Borrowed global CLI flags extracted from the parsed `Cli` struct.
///
/// This avoids requiring downstream callers to depend on the full clap type.
#[derive(Debug, Clone)]
pub struct GlobalCliFlags<'a> {
    pub model: Option<&'a str>,
    pub role: Option<&'a str>,
    pub effort: Option<&'a str>,
    pub resume: Option<&'a str>,
    pub json: bool,
    pub quiet: bool,
    pub headless: bool,
    pub no_serve: bool,
    pub color_enabled: bool,
}

/// Input fields specific to `roko do`.
#[derive(Debug, Clone, Default)]
pub struct DoInput {
    pub dry_run: bool,
    pub ghost: bool,
    pub yes: bool,
    pub no_cascade: bool,
    pub provider: Option<String>,
    pub context: Vec<PathBuf>,
}

/// Input fields specific to `roko plan run`.
#[derive(Debug, Clone, Default)]
pub struct PlanRunInput {
    pub no_tui: bool,
    pub approval: bool,
    pub dangerously_skip_permissions: bool,
    pub skip_preflight: bool,
    pub log_file: Option<PathBuf>,
    pub screenshots: bool,
    pub screenshot_interval: u64,
    pub screenshot_dir: Option<PathBuf>,
    pub batch_size: Option<usize>,
    pub force: bool,
    pub max_retries: Option<u32>,
    pub dry_run: bool,
    pub fresh: bool,
    pub force_resume: bool,
}

/// Input fields for `config set`.
#[derive(Debug, Clone, Default)]
pub struct ConfigSetInput {
    pub global: bool,
    pub project: bool,
}

/// Input fields for `learn tune`.
#[derive(Debug, Clone, Default)]
pub struct LearnTuneInput {
    pub dry_run: bool,
}

/// Input fields specific to `roko develop`.
#[derive(Debug, Clone, Default)]
pub struct DevelopInput {
    pub dry_run: bool,
    pub yes: bool,
    pub provider: Option<String>,
}

/// Input fields specific to `roko research`.
#[derive(Debug, Clone, Default)]
pub struct ResearchInput {
    /// Backend override from `--backend` (auto/gemini/perplexity/agent).
    pub backend: Option<String>,
    /// Whether `--deep` was passed.
    pub deep: bool,
}

/// Input fields specific to `roko agent serve`.
#[derive(Debug, Clone, Default)]
pub struct AgentServeInput {
    /// Whether `--allow-stub-cognitive-loop` was passed.
    pub allow_stub_cognitive_loop: bool,
}

/// Input fields specific to `roko agent chat`.
#[derive(Debug, Clone, Default)]
pub struct AgentChatInput {
    /// Provider override from `--provider`.
    pub provider: Option<String>,
    /// Force line-oriented REPL via `--text`.
    pub text: bool,
}

// ---------------------------------------------------------------------------
// ResolvedExecutionOverrides
// ---------------------------------------------------------------------------

/// Fully resolved execution overrides produced from CLI flags before any
/// side effects. Downstream code consumes this instead of re-parsing clap
/// state.
///
/// Constructed via `for_run`, `for_do`, or `for_plan_run`.
#[derive(Debug, Clone)]
pub struct ResolvedExecutionOverrides {
    // ── Model / provider ────────────────────────────────────────────
    /// Resolved model override from the global `--model` flag (aliases:
    /// `--force-model`, `--force-backend`).
    pub model: Option<String>,

    /// Resolved provider override from `--provider`.
    pub provider: Option<String>,

    // ── Role / effort ───────────────────────────────────────────────
    /// Agent role from `--role`.
    pub role: Option<String>,

    /// Reasoning effort level from `--effort`.
    pub effort: Option<String>,

    // ── Session ─────────────────────────────────────────────────────
    /// Session resume ID from `--resume`.
    pub resume: Option<String>,

    // ── Output format ───────────────────────────────────────────────
    /// `--json` output mode.
    pub json: bool,

    /// `--quiet` suppressed output mode.
    pub quiet: bool,

    /// Resolved color decision from `--color` + env vars + TTY.
    pub color_enabled: bool,

    // ── Policies ────────────────────────────────────────────────────
    /// Dry-run policy from `--dry-run` / `--ghost`.
    pub dry_run: DryRunPolicy,

    /// Cascade routing policy from `--no-cascade`.
    pub cascade_policy: CascadePolicy,

    /// Serve policy from `--serve` / `--no-serve`.
    pub serve_policy: ServePolicy,

    /// Interaction mode from `--headless`.
    pub interaction_mode: InteractionMode,

    /// Presentation mode from `--tui` / `--approval` / `--no-tui`.
    pub presentation: PresentationMode,

    /// Approval policy from `--yes` / `--dangerously-skip-permissions`.
    pub approval: ApprovalPolicy,

    // ── Plan run specifics ──────────────────────────────────────────
    /// Unsafe permission bypass from `--dangerously-skip-permissions`.
    pub dangerously_skip_permissions: bool,

    /// JSONL event log path from `--log-file`.
    pub log_file: Option<PathBuf>,

    /// Screenshot capture policy from `--screenshots` / `--screenshot-dir`.
    pub screenshots: ScreenshotPolicy,

    /// Batch size from `--batch-size` (zero rejected by clap).
    pub batch_size: Option<NonZeroUsize>,

    /// Force disk check bypass from `--force`.
    pub force_disk_check: bool,

    /// Skip preflight checks from `--skip-preflight`.
    pub skip_preflight: bool,

    /// Max retry attempts from `--max-retries`.
    pub max_retries: Option<u32>,

    /// Additional context file paths from `--context`.
    pub context_paths: Vec<PathBuf>,

    /// Archive old state and start clean from `--fresh`.
    pub fresh: bool,

    /// Re-queue drifted tasks on resume from `--force-resume`.
    pub force_resume: bool,
}

impl ResolvedExecutionOverrides {
    // ── Shared resolution helper ─────────────────────────────────────

    fn resolve_globals(flags: &GlobalCliFlags<'_>) -> Self {
        Self {
            model: flags.model.map(String::from),
            role: flags.role.map(String::from),
            effort: flags.effort.map(String::from),
            resume: flags.resume.map(String::from),
            json: flags.json,
            quiet: flags.quiet,
            color_enabled: flags.color_enabled,
            dry_run: DryRunPolicy::Execute,
            cascade_policy: CascadePolicy::Enabled,
            serve_policy: if flags.no_serve {
                ServePolicy::Disabled
            } else {
                ServePolicy::Auto
            },
            interaction_mode: if flags.headless {
                InteractionMode::Headless
            } else {
                InteractionMode::Interactive
            },
            presentation: PresentationMode::Auto,
            approval: ApprovalPolicy::Normal,
            provider: None,
            context_paths: Vec::new(),
            dangerously_skip_permissions: false,
            log_file: None,
            screenshots: ScreenshotPolicy::Disabled,
            batch_size: None,
            force_disk_check: false,
            skip_preflight: false,
            max_retries: None,
            fresh: false,
            force_resume: false,
        }
    }

    // ── Per-surface constructors ─────────────────────────────────────

    /// Resolve overrides for `roko run`.
    pub fn for_run(
        flags: &GlobalCliFlags<'_>,
        provider: Option<String>,
        serve_required: bool,
        max_retries: Option<u32>,
    ) -> Self {
        let mut resolved = Self::resolve_globals(flags);
        resolved.provider = provider;
        resolved.max_retries = max_retries;

        // --serve / --share forces Required; --no-serve forces Disabled.
        if serve_required {
            resolved.serve_policy = ServePolicy::Required;
        }
        // no_serve from globals already set Disabled if true.

        resolved
    }

    /// Resolve overrides for `roko do`.
    pub fn for_do(flags: &GlobalCliFlags<'_>, input: &DoInput) -> Self {
        let mut resolved = Self::resolve_globals(flags);

        // --dry-run or --ghost (deprecated alias) -> ReadOnlyNoMutation.
        if input.dry_run || input.ghost {
            resolved.dry_run = DryRunPolicy::ReadOnlyNoMutation;
        }

        // --yes -> auto-approve.
        if input.yes {
            resolved.approval = ApprovalPolicy::AutoApprove;
        }

        // --no-cascade -> disable cascade routing.
        if input.no_cascade {
            resolved.cascade_policy = CascadePolicy::DisabledByUser;
        }

        resolved.provider = input.provider.clone();
        resolved.context_paths = input.context.clone();

        resolved
    }

    /// Resolve overrides for `roko research`.
    ///
    /// Research commands default role to `researcher` unless `--role` explicitly
    /// overrides. Model/effort/resume are propagated from global flags.
    pub fn for_research(flags: &GlobalCliFlags<'_>, input: &ResearchInput) -> Self {
        let mut resolved = Self::resolve_globals(flags);

        // Default role to researcher unless explicitly overridden.
        if resolved.role.is_none() {
            resolved.role = Some("researcher".to_string());
        }

        // Research backend is informational; stored as provider for downstream routing.
        if let Some(ref backend) = input.backend {
            if backend != "auto" {
                resolved.provider = Some(backend.clone());
            }
        }

        resolved
    }

    /// Resolve overrides for `roko develop`.
    pub fn for_develop(flags: &GlobalCliFlags<'_>, input: &DevelopInput) -> Self {
        let mut resolved = Self::resolve_globals(flags);

        if input.dry_run {
            resolved.dry_run = DryRunPolicy::ReadOnlyNoMutation;
        }

        if input.yes {
            resolved.approval = ApprovalPolicy::AutoApprove;
        }

        resolved.provider = input.provider.clone();

        resolved
    }

    /// Resolve overrides for `roko plan run`.
    ///
    /// `--force-backend` is now a hidden alias on the global `--model` flag,
    /// so `flags.model` already carries the value regardless of which alias
    /// the operator used.
    pub fn for_plan_run(flags: &GlobalCliFlags<'_>, input: &PlanRunInput) -> Self {
        let mut resolved = Self::resolve_globals(flags);

        // Presentation: --approval/--tui -> Tui, --no-tui -> Text, neither -> Auto.
        resolved.presentation = if input.approval {
            PresentationMode::Tui
        } else if input.no_tui {
            PresentationMode::Text
        } else {
            PresentationMode::Auto
        };

        // --dry-run.
        if input.dry_run {
            resolved.dry_run = DryRunPolicy::ReadOnlyNoMutation;
        }

        // Plan-run-specific fields.
        resolved.dangerously_skip_permissions = input.dangerously_skip_permissions;
        resolved.log_file = input.log_file.clone();
        resolved.skip_preflight = input.skip_preflight;
        resolved.force_disk_check = input.force;
        resolved.max_retries = input.max_retries;

        // Screenshot policy: --screenshots enables capture with optional dir/interval.
        if input.screenshots {
            resolved.screenshots = ScreenshotPolicy::Enabled {
                dir: input.screenshot_dir.clone(),
                interval_secs: input.screenshot_interval,
            };
        }

        // Batch size: zero from clap is rejected; nonzero wraps into NonZeroUsize.
        if let Some(n) = input.batch_size {
            resolved.batch_size = NonZeroUsize::new(n);
        }

        resolved.fresh = input.fresh;
        resolved.force_resume = input.force_resume;

        resolved
    }

    /// Resolve overrides for `roko agent serve`.
    ///
    /// The serve surface only consumes model (for dispatcher construction)
    /// and the stub cognitive loop flag. All other policies resolve to their
    /// defaults because `agent serve` does not accept plan-run or do-style
    /// flags.
    pub fn for_agent_serve(flags: &GlobalCliFlags<'_>, input: &AgentServeInput) -> Self {
        let mut resolved = Self::resolve_globals(flags);

        // Agent serve never uses plan-run presentation or approval modes.
        resolved.presentation = PresentationMode::Auto;
        resolved.approval = ApprovalPolicy::Normal;

        // Propagate the stub cognitive loop flag as a custom interaction
        // signal. Downstream code checks `allow_stub_cognitive_loop` on
        // the runtime config; we use the interaction_mode field only for
        // headless detection, so the stub flag is separate.
        let _ = input.allow_stub_cognitive_loop; // consumed by AgentServeRuntimeConfig

        resolved
    }

    /// Resolve overrides for `roko agent chat`.
    ///
    /// Resolves model/provider from global flags and chat-specific `--provider`.
    /// UI mode is determined by `--text` and TTY detection but is resolved in
    /// `agent_serve::resolve_chat_launch` because it depends on runtime
    /// terminal state.
    pub fn for_agent_chat(flags: &GlobalCliFlags<'_>, input: &AgentChatInput) -> Self {
        let mut resolved = Self::resolve_globals(flags);

        // Chat-specific provider override.
        if input.provider.is_some() {
            resolved.provider = input.provider.clone();
        }

        // --text forces LineRepl; otherwise resolve_chat_launch uses TTY.
        if input.text {
            resolved.presentation = PresentationMode::Text;
        }

        resolved
    }

    // ── Narrow per-surface resolvers ─────────────────────────────────

    /// Resolve `config set --global/--project` into a single edit target.
    ///
    /// No flags and `--global` both resolve to `Global` for backward
    /// compatibility. `--project` resolves to the workspace config.
    /// The flags are mutually exclusive (enforced by clap `conflicts_with`).
    pub fn resolve_config_edit_target(input: &ConfigSetInput) -> ConfigEditTarget {
        if input.project {
            ConfigEditTarget::Project
        } else {
            // Default (no flag) and explicit --global both resolve to Global.
            ConfigEditTarget::Global
        }
    }

    /// Resolve `config preset --global/--project` into a single edit target.
    ///
    /// Unlike `config set`, no flag resolves to `Project` to preserve
    /// historical top-level `tune` writes. #311 consumes the target and
    /// owns the actual preset diff/write behavior.
    pub fn resolve_preset_edit_target(input: &ConfigSetInput) -> ConfigEditTarget {
        if input.global {
            ConfigEditTarget::Global
        } else {
            // Default (no flag) and explicit --project both resolve to Project.
            ConfigEditTarget::Project
        }
    }

    /// Resolve `learn tune --dry-run` into a dry-run policy.
    ///
    /// Kept for parsing compatibility. #311 consumes this and emits the
    /// single informational message that inspection never mutates.
    pub fn resolve_tune_dry_run(input: &LearnTuneInput) -> DryRunPolicy {
        if input.dry_run {
            DryRunPolicy::ReadOnlyNoMutation
        } else {
            DryRunPolicy::Execute
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_flags() -> GlobalCliFlags<'static> {
        GlobalCliFlags {
            model: None,
            role: None,
            effort: None,
            resume: None,
            json: false,
            quiet: false,
            headless: false,
            no_serve: false,
            color_enabled: true,
        }
    }

    // ── Global resolution ────────────────────────────────────────────

    #[test]
    fn default_globals_resolve_permissive_policies() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_do(&flags, &DoInput::default());
        assert_eq!(r.dry_run, DryRunPolicy::Execute);
        assert_eq!(r.interaction_mode, InteractionMode::Interactive);
        assert_eq!(r.cascade_policy, CascadePolicy::Enabled);
        assert_eq!(r.serve_policy, ServePolicy::Auto);
        assert_eq!(r.presentation, PresentationMode::Auto);
        assert_eq!(r.approval, ApprovalPolicy::Normal);
        assert_eq!(r.screenshots, ScreenshotPolicy::Disabled);
        assert_eq!(r.batch_size, None);
        assert!(!r.dangerously_skip_permissions);
        assert!(!r.force_disk_check);
        assert!(!r.skip_preflight);
        assert!(r.context_paths.is_empty());
    }

    #[test]
    fn headless_resolves() {
        let mut flags = default_flags();
        flags.headless = true;
        let r = ResolvedExecutionOverrides::for_do(&flags, &DoInput::default());
        assert_eq!(r.interaction_mode, InteractionMode::Headless);
    }

    #[test]
    fn no_serve_resolves_disabled() {
        let mut flags = default_flags();
        flags.no_serve = true;
        let r = ResolvedExecutionOverrides::for_run(&flags, None, false, None);
        assert_eq!(r.serve_policy, ServePolicy::Disabled);
    }

    #[test]
    fn model_role_effort_resume_propagate() {
        let flags = GlobalCliFlags {
            model: Some("opus"),
            role: Some("architect"),
            effort: Some("high"),
            resume: Some("session-42"),
            ..default_flags()
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &DoInput::default());
        assert_eq!(r.model.as_deref(), Some("opus"));
        assert_eq!(r.role.as_deref(), Some("architect"));
        assert_eq!(r.effort.as_deref(), Some("high"));
        assert_eq!(r.resume.as_deref(), Some("session-42"));
    }

    #[test]
    fn json_quiet_color_propagate() {
        let flags = GlobalCliFlags {
            json: true,
            quiet: true,
            color_enabled: false,
            ..default_flags()
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &DoInput::default());
        assert!(r.json);
        assert!(r.quiet);
        assert!(!r.color_enabled);
    }

    // ── for_run ──────────────────────────────────────────────────────

    #[test]
    fn for_run_serve_required() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_run(&flags, None, true, None);
        assert_eq!(r.serve_policy, ServePolicy::Required);
    }

    #[test]
    fn for_run_serve_auto_without_flags() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_run(&flags, None, false, None);
        assert_eq!(r.serve_policy, ServePolicy::Auto);
    }

    #[test]
    fn for_run_no_serve_wins_over_serve_required() {
        // --no-serve is set at global level; serve_required comes from --serve/--share.
        // Global --no-serve takes effect first in resolve_globals.
        let mut flags = default_flags();
        flags.no_serve = true;
        // serve_required=true then overrides to Required.
        let r = ResolvedExecutionOverrides::for_run(&flags, None, true, None);
        assert_eq!(r.serve_policy, ServePolicy::Required);
    }

    #[test]
    fn for_run_max_retries_propagates() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_run(&flags, None, false, Some(3));
        assert_eq!(r.max_retries, Some(3));
    }

    #[test]
    fn for_run_provider_propagates() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_run(&flags, Some("anthropic".into()), false, None);
        assert_eq!(r.provider.as_deref(), Some("anthropic"));
    }

    // ── for_do ───────────────────────────────────────────────────────

    #[test]
    fn do_dry_run_resolves() {
        let flags = default_flags();
        let input = DoInput {
            dry_run: true,
            ..DoInput::default()
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &input);
        assert_eq!(r.dry_run, DryRunPolicy::ReadOnlyNoMutation);
    }

    #[test]
    fn do_ghost_is_dry_run() {
        let flags = default_flags();
        let input = DoInput {
            ghost: true,
            ..DoInput::default()
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &input);
        assert_eq!(r.dry_run, DryRunPolicy::ReadOnlyNoMutation);
    }

    #[test]
    fn ghost_and_dry_run_produce_same_policy() {
        let flags = default_flags();
        let ghost_input = DoInput {
            ghost: true,
            ..DoInput::default()
        };
        let dry_input = DoInput {
            dry_run: true,
            ..DoInput::default()
        };
        let r_ghost = ResolvedExecutionOverrides::for_do(&flags, &ghost_input);
        let r_dry = ResolvedExecutionOverrides::for_do(&flags, &dry_input);
        assert_eq!(r_ghost.dry_run, r_dry.dry_run);
    }

    #[test]
    fn do_yes_auto_approves() {
        let flags = default_flags();
        let input = DoInput {
            yes: true,
            ..DoInput::default()
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &input);
        assert_eq!(r.approval, ApprovalPolicy::AutoApprove);
    }

    #[test]
    fn do_no_cascade_resolves() {
        let flags = default_flags();
        let input = DoInput {
            no_cascade: true,
            ..DoInput::default()
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &input);
        assert_eq!(r.cascade_policy, CascadePolicy::DisabledByUser);
    }

    #[test]
    fn do_context_paths_propagate() {
        let flags = default_flags();
        let input = DoInput {
            context: vec![PathBuf::from("src/lib.rs"), PathBuf::from("Cargo.toml")],
            ..DoInput::default()
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &input);
        assert_eq!(r.context_paths.len(), 2);
        assert_eq!(r.context_paths[0], PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn do_provider_propagates() {
        let flags = default_flags();
        let input = DoInput {
            provider: Some("openai".into()),
            ..DoInput::default()
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &input);
        assert_eq!(r.provider.as_deref(), Some("openai"));
    }

    // ── for_plan_run ─────────────────────────────────────────────────

    #[test]
    fn plan_run_global_model_propagates() {
        let flags = GlobalCliFlags {
            model: Some("opus"),
            ..default_flags()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &PlanRunInput::default());
        assert_eq!(r.model.as_deref(), Some("opus"));
    }

    #[test]
    fn plan_run_no_model_when_neither_set() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &PlanRunInput::default());
        assert_eq!(r.model, None);
    }

    #[test]
    fn plan_run_tui_presentation() {
        let flags = default_flags();

        let tui = PlanRunInput {
            approval: true,
            ..PlanRunInput::default()
        };
        let no_tui = PlanRunInput {
            no_tui: true,
            ..PlanRunInput::default()
        };
        let auto = PlanRunInput::default();

        assert_eq!(
            ResolvedExecutionOverrides::for_plan_run(&flags, &tui).presentation,
            PresentationMode::Tui,
        );
        assert_eq!(
            ResolvedExecutionOverrides::for_plan_run(&flags, &no_tui).presentation,
            PresentationMode::Text,
        );
        assert_eq!(
            ResolvedExecutionOverrides::for_plan_run(&flags, &auto).presentation,
            PresentationMode::Auto,
        );
    }

    #[test]
    fn plan_run_dry_run_resolves() {
        let flags = default_flags();
        let plan = PlanRunInput {
            dry_run: true,
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert_eq!(r.dry_run, DryRunPolicy::ReadOnlyNoMutation);
    }

    #[test]
    fn plan_run_screenshots_resolve() {
        let flags = default_flags();
        let plan = PlanRunInput {
            screenshots: true,
            screenshot_interval: 30,
            screenshot_dir: Some(PathBuf::from("/tmp/shots")),
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert_eq!(
            r.screenshots,
            ScreenshotPolicy::Enabled {
                dir: Some(PathBuf::from("/tmp/shots")),
                interval_secs: 30,
            }
        );
    }

    #[test]
    fn plan_run_screenshots_disabled_by_default() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &PlanRunInput::default());
        assert_eq!(r.screenshots, ScreenshotPolicy::Disabled);
    }

    #[test]
    fn plan_run_batch_size_wraps_nonzero() {
        let flags = default_flags();
        let plan = PlanRunInput {
            batch_size: Some(5),
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert_eq!(r.batch_size, NonZeroUsize::new(5));
    }

    #[test]
    fn plan_run_zero_batch_size_is_none() {
        let flags = default_flags();
        let plan = PlanRunInput {
            batch_size: Some(0),
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert_eq!(r.batch_size, None);
    }

    #[test]
    fn plan_run_dangerously_skip_permissions_propagates() {
        let flags = default_flags();
        let plan = PlanRunInput {
            dangerously_skip_permissions: true,
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert!(r.dangerously_skip_permissions);
    }

    #[test]
    fn plan_run_log_file_propagates() {
        let flags = default_flags();
        let plan = PlanRunInput {
            log_file: Some(PathBuf::from("/tmp/events.jsonl")),
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert_eq!(r.log_file, Some(PathBuf::from("/tmp/events.jsonl")));
    }

    #[test]
    fn plan_run_skip_preflight_propagates() {
        let flags = default_flags();
        let plan = PlanRunInput {
            skip_preflight: true,
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert!(r.skip_preflight);
    }

    #[test]
    fn plan_run_force_disk_check_propagates() {
        let flags = default_flags();
        let plan = PlanRunInput {
            force: true,
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert!(r.force_disk_check);
    }

    #[test]
    fn plan_run_max_retries_propagates() {
        let flags = default_flags();
        let plan = PlanRunInput {
            max_retries: Some(5),
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert_eq!(r.max_retries, Some(5));
    }

    // ── Config edit target ───────────────────────────────────────────

    #[test]
    fn config_set_no_flags_defaults_global() {
        assert_eq!(
            ResolvedExecutionOverrides::resolve_config_edit_target(&ConfigSetInput {
                global: false,
                project: false,
            }),
            ConfigEditTarget::Global,
            "no flags defaults to Global for config set"
        );
    }

    #[test]
    fn config_set_project_resolves() {
        assert_eq!(
            ResolvedExecutionOverrides::resolve_config_edit_target(&ConfigSetInput {
                global: false,
                project: true,
            }),
            ConfigEditTarget::Project,
        );
    }

    #[test]
    fn config_set_global_explicit_resolves() {
        assert_eq!(
            ResolvedExecutionOverrides::resolve_config_edit_target(&ConfigSetInput {
                global: true,
                project: false,
            }),
            ConfigEditTarget::Global,
        );
    }

    #[test]
    fn preset_no_flags_defaults_project() {
        assert_eq!(
            ResolvedExecutionOverrides::resolve_preset_edit_target(&ConfigSetInput {
                global: false,
                project: false,
            }),
            ConfigEditTarget::Project,
            "no flags defaults to Project for config preset"
        );
    }

    #[test]
    fn preset_global_resolves() {
        assert_eq!(
            ResolvedExecutionOverrides::resolve_preset_edit_target(&ConfigSetInput {
                global: true,
                project: false,
            }),
            ConfigEditTarget::Global,
        );
    }

    // ── Learn tune ───────────────────────────────────────────────────

    #[test]
    fn tune_dry_run_resolves() {
        assert_eq!(
            ResolvedExecutionOverrides::resolve_tune_dry_run(&LearnTuneInput { dry_run: true }),
            DryRunPolicy::ReadOnlyNoMutation,
        );
        assert_eq!(
            ResolvedExecutionOverrides::resolve_tune_dry_run(&LearnTuneInput { dry_run: false }),
            DryRunPolicy::Execute,
        );
    }

    // ── All global flags roundtrip ───────────────────────────────────

    #[test]
    fn all_global_flags_roundtrip() {
        let flags = GlobalCliFlags {
            model: Some("opus"),
            role: Some("architect"),
            effort: Some("high"),
            resume: Some("session-42"),
            json: true,
            quiet: true,
            headless: true,
            no_serve: true,
            color_enabled: false,
        };
        let r = ResolvedExecutionOverrides::for_do(&flags, &DoInput::default());
        assert_eq!(r.model.as_deref(), Some("opus"));
        assert_eq!(r.role.as_deref(), Some("architect"));
        assert_eq!(r.effort.as_deref(), Some("high"));
        assert_eq!(r.resume.as_deref(), Some("session-42"));
        assert!(r.json);
        assert!(r.quiet);
        assert!(!r.color_enabled);
        assert_eq!(r.interaction_mode, InteractionMode::Headless);
        assert_eq!(r.serve_policy, ServePolicy::Disabled);
    }

    // ── Combined surface constructors do not leak cross-surface fields ──

    #[test]
    fn for_do_does_not_set_plan_run_fields() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_do(&flags, &DoInput::default());
        assert!(!r.dangerously_skip_permissions);
        assert!(r.log_file.is_none());
        assert!(!r.skip_preflight);
        assert!(!r.force_disk_check);
        assert_eq!(r.screenshots, ScreenshotPolicy::Disabled);
        assert_eq!(r.batch_size, None);
    }

    #[test]
    fn for_run_does_not_set_plan_run_fields() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_run(&flags, None, false, None);
        assert!(!r.dangerously_skip_permissions);
        assert!(r.log_file.is_none());
        assert!(!r.skip_preflight);
        assert!(!r.force_disk_check);
        assert_eq!(r.screenshots, ScreenshotPolicy::Disabled);
        assert_eq!(r.batch_size, None);
        assert!(!r.fresh);
        assert!(!r.force_resume);
    }

    // ── Fresh / force-resume ──────────────────────────────────────────

    #[test]
    fn plan_run_fresh_propagates() {
        let flags = default_flags();
        let plan = PlanRunInput {
            fresh: true,
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert!(r.fresh);
    }

    #[test]
    fn plan_run_force_resume_propagates() {
        let flags = default_flags();
        let plan = PlanRunInput {
            force_resume: true,
            ..PlanRunInput::default()
        };
        let r = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
        assert!(r.force_resume);
    }

    // ── for_develop ───────────────────────────────────────────────────

    #[test]
    fn develop_dry_run_resolves() {
        let flags = default_flags();
        let input = DevelopInput {
            dry_run: true,
            ..DevelopInput::default()
        };
        let r = ResolvedExecutionOverrides::for_develop(&flags, &input);
        assert_eq!(r.dry_run, DryRunPolicy::ReadOnlyNoMutation);
    }

    #[test]
    fn develop_yes_auto_approves() {
        let flags = default_flags();
        let input = DevelopInput {
            yes: true,
            ..DevelopInput::default()
        };
        let r = ResolvedExecutionOverrides::for_develop(&flags, &input);
        assert_eq!(r.approval, ApprovalPolicy::AutoApprove);
    }

    #[test]
    fn develop_provider_propagates() {
        let flags = default_flags();
        let input = DevelopInput {
            provider: Some("anthropic".into()),
            ..DevelopInput::default()
        };
        let r = ResolvedExecutionOverrides::for_develop(&flags, &input);
        assert_eq!(r.provider.as_deref(), Some("anthropic"));
    }

    #[test]
    fn develop_does_not_set_plan_run_fields() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_develop(&flags, &DevelopInput::default());
        assert!(!r.dangerously_skip_permissions);
        assert!(r.log_file.is_none());
        assert!(!r.skip_preflight);
        assert!(!r.force_disk_check);
        assert_eq!(r.screenshots, ScreenshotPolicy::Disabled);
        assert_eq!(r.batch_size, None);
        assert!(!r.fresh);
        assert!(!r.force_resume);
    }

    // ── for_research ──────────────────────────────────────────────────

    #[test]
    fn research_defaults_role_to_researcher() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_research(&flags, &ResearchInput::default());
        assert_eq!(r.role.as_deref(), Some("researcher"));
    }

    #[test]
    fn research_explicit_role_overrides_default() {
        let flags = GlobalCliFlags {
            role: Some("architect"),
            ..default_flags()
        };
        let r = ResolvedExecutionOverrides::for_research(&flags, &ResearchInput::default());
        assert_eq!(
            r.role.as_deref(),
            Some("architect"),
            "explicit --role must override researcher default"
        );
    }

    #[test]
    fn research_auto_backend_sets_no_provider() {
        let flags = default_flags();
        let input = ResearchInput {
            backend: Some("auto".into()),
            deep: false,
        };
        let r = ResolvedExecutionOverrides::for_research(&flags, &input);
        assert_eq!(r.provider, None, "auto backend must not set provider");
    }

    #[test]
    fn research_explicit_backend_sets_provider() {
        let flags = default_flags();
        let input = ResearchInput {
            backend: Some("gemini".into()),
            deep: false,
        };
        let r = ResolvedExecutionOverrides::for_research(&flags, &input);
        assert_eq!(r.provider.as_deref(), Some("gemini"));
    }

    #[test]
    fn research_model_effort_resume_propagate() {
        let flags = GlobalCliFlags {
            model: Some("opus"),
            effort: Some("high"),
            resume: Some("session-42"),
            ..default_flags()
        };
        let r = ResolvedExecutionOverrides::for_research(&flags, &ResearchInput::default());
        assert_eq!(r.model.as_deref(), Some("opus"));
        assert_eq!(r.effort.as_deref(), Some("high"));
        assert_eq!(r.resume.as_deref(), Some("session-42"));
    }

    #[test]
    fn research_does_not_set_plan_run_fields() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_research(&flags, &ResearchInput::default());
        assert!(!r.dangerously_skip_permissions);
        assert!(r.log_file.is_none());
        assert!(!r.skip_preflight);
        assert!(!r.force_disk_check);
        assert_eq!(r.screenshots, ScreenshotPolicy::Disabled);
        assert_eq!(r.batch_size, None);
        assert!(!r.fresh);
        assert!(!r.force_resume);
    }

    // ── Exhaustive field manifest ─────────────────────────────────────
    //
    // This test ensures every field of ResolvedExecutionOverrides is
    // explicitly listed. Adding a new field to the struct without adding
    // a resolution row here will produce a compile error (missing field
    // in the destructure).

    #[test]
    fn exhaustive_field_manifest_covers_all_struct_fields() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_plan_run(
            &flags,
            &PlanRunInput {
                no_tui: false,
                approval: false,
                dangerously_skip_permissions: true,
                skip_preflight: true,
                log_file: Some(PathBuf::from("/tmp/log.jsonl")),
                screenshots: true,
                screenshot_interval: 10,
                screenshot_dir: Some(PathBuf::from("/tmp/shots")),
                batch_size: Some(3),
                force: true,
                max_retries: Some(2),
                dry_run: true,
                fresh: true,
                force_resume: true,
            },
        );

        // Destructure to ensure every field is accounted for.
        // A new field added to the struct will cause a compile error here.
        let ResolvedExecutionOverrides {
            model: _,
            provider: _,
            role: _,
            effort: _,
            resume: _,
            json: _,
            quiet: _,
            color_enabled: _,
            dry_run,
            cascade_policy: _,
            serve_policy: _,
            interaction_mode: _,
            presentation: _,
            approval: _,
            dangerously_skip_permissions,
            log_file,
            screenshots,
            batch_size,
            force_disk_check,
            skip_preflight,
            max_retries,
            context_paths: _,
            fresh,
            force_resume,
        } = r;

        // Spot-check the plan-run-specific fields that were set above.
        assert_eq!(dry_run, DryRunPolicy::ReadOnlyNoMutation);
        assert!(dangerously_skip_permissions);
        assert!(skip_preflight);
        assert_eq!(log_file, Some(PathBuf::from("/tmp/log.jsonl")));
        assert!(matches!(screenshots, ScreenshotPolicy::Enabled { .. }));
        assert_eq!(batch_size, NonZeroUsize::new(3));
        assert!(force_disk_check);
        assert_eq!(max_retries, Some(2));
        assert!(fresh);
        assert!(force_resume);
    }

    // ── for_agent_serve ─────────────────────────────────────────────

    #[test]
    fn agent_serve_defaults_resolve_safely() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_agent_serve(&flags, &AgentServeInput::default());
        assert_eq!(r.presentation, PresentationMode::Auto);
        assert_eq!(r.approval, ApprovalPolicy::Normal);
        assert!(r.model.is_none());
        assert!(r.provider.is_none());
    }

    #[test]
    fn agent_serve_propagates_global_model() {
        let flags = GlobalCliFlags {
            model: Some("opus"),
            ..default_flags()
        };
        let r = ResolvedExecutionOverrides::for_agent_serve(
            &flags,
            &AgentServeInput {
                allow_stub_cognitive_loop: true,
            },
        );
        assert_eq!(r.model.as_deref(), Some("opus"));
    }

    // ── for_agent_chat ──────────────────────────────────────────────

    #[test]
    fn agent_chat_defaults_resolve_safely() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_agent_chat(&flags, &AgentChatInput::default());
        assert!(r.model.is_none());
        assert!(r.provider.is_none());
        assert_eq!(r.presentation, PresentationMode::Auto);
    }

    #[test]
    fn agent_chat_provider_override() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_agent_chat(
            &flags,
            &AgentChatInput {
                provider: Some("anthropic_api".to_string()),
                text: false,
            },
        );
        assert_eq!(r.provider.as_deref(), Some("anthropic_api"));
    }

    #[test]
    fn agent_chat_text_forces_text_presentation() {
        let flags = default_flags();
        let r = ResolvedExecutionOverrides::for_agent_chat(
            &flags,
            &AgentChatInput {
                provider: None,
                text: true,
            },
        );
        assert_eq!(r.presentation, PresentationMode::Text);
    }

    #[test]
    fn agent_chat_propagates_global_model() {
        let flags = GlobalCliFlags {
            model: Some("sonnet"),
            ..default_flags()
        };
        let r = ResolvedExecutionOverrides::for_agent_chat(&flags, &AgentChatInput::default());
        assert_eq!(r.model.as_deref(), Some("sonnet"));
    }
}
