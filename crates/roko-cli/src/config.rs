//! `roko.toml` schema — declarative config for the CLI's universal loop.
//!
//! The config picks an agent backend (any CLI that reads prompts on stdin),
//! sets a token budget for prompt composition, and lists the gates to run
//! on the agent's output.

use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig, ProviderRouting, RokoConfig};
use roko_core::config::{
    DEFAULT_TTFT_TIMEOUT_MS, ServeConfig, ServeDeployConfig, ServeDeployWebhookConfig,
};
use roko_core::defaults::{
    DEFAULT_CONNECT_TIMEOUT_MS, DEFAULT_PLAN_TIMEOUT_SECS, DEFAULT_REQUEST_TIMEOUT_MS,
};
use roko_daimon::StrategySpaceDefinition;
use roko_dreams::DreamSchedulePolicy;

/// The top-level `roko.toml` document.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Agent subprocess backend (the external CLI invoked via `ExecAgent`).
    pub agent: ExecAgentConfig,
    /// Automatically generate a plan when a PRD is promoted.
    #[serde(default)]
    pub auto_plan: bool,
    /// Automatic dream-cycle settings for daemon mode.
    #[serde(default)]
    pub dreams: DreamsConfig,
    /// Daimon affect-engine configuration.
    #[serde(default)]
    pub daimon: DaimonConfig,
    /// The agent role and chat prompt budget a command resolves at run time
    /// (`--role`, `--effort`). No roko.toml key sets them: `[prompt] role` and
    /// `token_budget` were removed, so a file's `[prompt]` is not read here.
    #[serde(skip)]
    pub prompt: PromptConfig,
    /// Per-repository configuration blocks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repos: Vec<RepoConfig>,
    /// Gates to run on the agent output, in declaration order.
    #[serde(default, rename = "gate")]
    pub gates: Vec<GateConfig>,
    /// Plan-level runner settings.
    #[serde(default)]
    pub runner: RunnerConfig,
    /// Durable runtime/control-plane settings.
    #[serde(default)]
    pub runtime: RuntimeControlConfig,
    /// Cost budget configuration.
    #[serde(default)]
    pub budget: BudgetConfig,
    /// Provider registry keyed by provider name.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub providers: IndexMap<String, ProviderConfig>,
    /// Model registry keyed by model name.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub models: IndexMap<String, ModelProfile>,
    /// Learning / feedback-loop settings.
    #[serde(default)]
    pub learning: LearningLayer,
    /// API serving options.
    #[serde(default)]
    pub serve: ServeConfig,
    /// Structured log output format for cloud deployments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_format: Option<String>,
    /// HTTP bind address for cloud deployments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    /// Persistent workspace directory for cloud deployments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: ExecAgentConfig::default(),
            auto_plan: false,
            dreams: DreamsConfig::default(),
            daimon: DaimonConfig::default(),
            prompt: PromptConfig::default(),
            repos: Vec::new(),
            // No legacy `[[gate]]` entries, as when a file leaves them out:
            // these gates are only counted, never run.
            gates: Vec::new(),
            runner: RunnerConfig::default(),
            runtime: RuntimeControlConfig::default(),
            budget: BudgetConfig::default(),
            providers: IndexMap::new(),
            models: IndexMap::new(),
            learning: LearningLayer::default(),
            serve: ServeConfig::default(),
            log_format: None,
            bind: None,
            data_dir: None,
        }
    }
}

impl Config {
    /// Read a TOML config from `path`.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        Self::parse_toml(&text).with_context(|| format!("parse config {}", path.display()))
    }

    /// Parse a TOML config from a string.
    pub fn parse_toml(text: &str) -> Result<Self> {
        let config: Self = parse_toml_with_env(text, "invalid roko.toml")?;
        config.dreams.validate().context("validate [dreams]")?;
        Ok(config)
    }

    /// Render this config back to a TOML string.
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("serialize roko.toml")
    }

    /// Render the default `roko.toml` template used by `roko init`.
    pub fn default_toml_template(cloud: bool) -> Result<String> {
        crate::init::render_init_template(cloud)
    }

    /// Build a CLI `Config` from a validated core `RokoConfig`.
    ///
    /// This is the primary conversion path used by `load_resolved_config()`.
    /// The core loader is the single source of truth for providers, models,
    /// agent defaults, and env overrides. CLI-only compatibility fields
    /// (`gates`, `runtime`, `prompt`) retain their existing defaults because
    /// those sections are not yet part of the core schema; once they are
    /// migrated, this function will map them directly.
    pub fn from_roko_config(core: &RokoConfig) -> Result<Self> {
        let core_agent = &core.agent;
        let agent = ExecAgentConfig {
            model: Some(core_agent.default_model.clone()),
            effort: core_agent.default_effort.clone(),
            bare_mode: core_agent.bare_mode,
            command: core_agent
                .command
                .clone()
                .unwrap_or_else(ExecAgentConfig::default_command),
            args: core_agent.args.clone().unwrap_or_default(),
            timeout_ms: core_agent
                .timeout_ms
                .unwrap_or(ExecAgentConfig::default_timeout()),
            env: core_agent.env.clone().unwrap_or_default(),
            env_passthrough: core_agent.env_passthrough.clone(),
            fallback_model: core_agent.fallback_model.clone(),
            clean_output: ExecAgentConfig::default_clean(),
            mcp_config: None,
            tier_models: core_agent.tier_models.clone(),
            escalation: EscalationConfig::default(),
        };

        let dreams: DreamsConfig = core.dreams.clone().into();
        dreams.validate().context("validate [dreams]")?;

        let daimon = DaimonConfig::from_core(&core.daimon)?;

        Ok(Self {
            agent,
            auto_plan: core.prd.auto_plan,
            dreams,
            daimon,
            prompt: PromptConfig::default(),
            repos: core.repos.clone(),
            gates: Vec::new(),
            runner: RunnerConfig {
                plan_timeout_secs: core.runner.plan_timeout_secs,
                dangerously_skip_permissions: core.runner.dangerously_skip_permissions,
            },
            runtime: RuntimeControlConfig::default(),
            budget: BudgetConfig::from_core(&core.budget),
            providers: core.providers.clone(),
            models: core.models.clone(),
            learning: LearningLayer::from_core_learning(&core.learning),
            serve: core.serve.clone(),
            log_format: None,
            bind: None,
            data_dir: None,
        })
    }
}

/// Subprocess dispatch configuration for the external agent CLI invoked via `ExecAgent`.
///
/// This is a CLI-layer concept: it describes how to invoke an external agent
/// binary (command, args, timeout, env). It is distinct from
/// `roko_core::config::AgentConfig`, which holds agent model/role/provider
/// settings that are part of the canonical `roko.toml` schema.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExecAgentConfig {
    /// Program name, e.g. `"cat"`, `"ollama"`, `"claude"`.
    #[serde(default = "ExecAgentConfig::default_command")]
    pub command: String,
    /// Extra args passed to the program.
    #[serde(default)]
    pub args: Vec<String>,
    /// Preferred model slug for Claude-style CLIs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Reasoning effort passed to Claude-style CLIs.
    #[serde(default = "ExecAgentConfig::default_effort")]
    pub effort: String,
    /// Whether Claude CLI replaces its built-in system prompt with Roko's
    /// canonical prompt instead of appending to it.
    #[serde(default = "ExecAgentConfig::default_bare_mode")]
    pub bare_mode: bool,
    /// Optional fallback model slug for Claude-style CLIs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,
    /// Timeout in milliseconds (default: `120_000`).
    #[serde(default = "ExecAgentConfig::default_timeout")]
    pub timeout_ms: u64,
    /// Env vars passed to the subprocess. Useful for `OLLAMA_NOPROGRESS=1`,
    /// API keys, `OLLAMA_HOST`, etc.
    #[serde(default)]
    pub env: Vec<(String, String)>,
    /// `[agent] env_passthrough`: variables a provider CLI keeps although
    /// roko loaded them from a `.env` file (an exact name or `PREFIX*`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_passthrough: Vec<String>,
    /// Whether to post-process the agent output — strip ANSI escapes and
    /// reasoning-model "thinking" traces. Default: `true` (so reasoning
    /// models like glm-4 / gemma-reasoning work out of the box).
    #[serde(default = "ExecAgentConfig::default_clean")]
    pub clean_output: bool,
    /// Optional path to an MCP config file (`.mcp.json`). When set, this
    /// is passed to Claude via `--mcp-config`. If unset, `ClaudeCliAgent`
    /// auto-discovers by walking up from the working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_config: Option<PathBuf>,
    /// Per-tier model mapping. Keys: `mechanical`, `focused`, `integrative`, `architectural`.
    #[serde(default)]
    pub tier_models: std::collections::HashMap<String, String>,
    /// Retry escalation configuration.
    #[serde(default)]
    pub escalation: EscalationConfig,
}

/// Backward-compatibility alias. Prefer `ExecAgentConfig`.
pub type AgentConfig = ExecAgentConfig;

/// Automatic dream-cycle settings for daemon mode.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DreamsConfig {
    /// Enable the automatic dream cycle.
    #[serde(default = "DreamsConfig::default_auto_dream")]
    pub auto_dream: bool,
    /// Idle duration threshold, in minutes, before a dream can run.
    #[serde(default = "DreamsConfig::default_idle_threshold_mins")]
    pub idle_threshold_mins: u64,
    /// Minimum number of new episodes required before dreaming.
    #[serde(default = "DreamsConfig::default_min_episodes_for_dream")]
    pub min_episodes_for_dream: usize,
    /// Optional seven-field cron expression used as a fallback cadence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_cron: Option<String>,
    /// Trigger as soon as this many unconsolidated episodes exist. Zero
    /// disables the episode-count trigger; the idle/cron minimum still uses
    /// `min_episodes_for_dream`.
    #[serde(default)]
    pub episode_count_trigger: usize,
    /// Idle-delay multiplier after a high-quality dream cycle.
    #[serde(default = "DreamsConfig::default_quality_gain")]
    pub quality_gain: f64,
    /// Idle-delay multiplier after a low-quality cycle.
    #[serde(default = "DreamsConfig::default_quality_penalty")]
    pub quality_penalty: f64,
}

impl DreamsConfig {
    const fn default_auto_dream() -> bool {
        true
    }

    const fn default_idle_threshold_mins() -> u64 {
        15
    }

    const fn default_min_episodes_for_dream() -> usize {
        5
    }

    const fn default_quality_gain() -> f64 {
        0.75
    }

    const fn default_quality_penalty() -> f64 {
        1.25
    }

    /// Convert the resolved CLI fields into the shared dream runtime policy.
    #[must_use]
    pub fn schedule_policy(&self) -> DreamSchedulePolicy {
        DreamSchedulePolicy {
            enabled: self.auto_dream,
            idle_threshold_mins: self.idle_threshold_mins,
            scheduled_cron: self.scheduled_cron.clone(),
            manual_enabled: true,
            quality_gain: self.quality_gain,
            quality_penalty: self.quality_penalty,
            episode_count_trigger: self.episode_count_trigger,
        }
    }

    /// Validate the resolved scheduling fields before they reach a runtime.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid cron expression or invalid adaptive
    /// quality multiplier.
    pub fn validate(&self) -> Result<()> {
        self.schedule_policy().validate()
    }
}

impl Default for DreamsConfig {
    fn default() -> Self {
        Self {
            auto_dream: Self::default_auto_dream(),
            idle_threshold_mins: Self::default_idle_threshold_mins(),
            min_episodes_for_dream: Self::default_min_episodes_for_dream(),
            scheduled_cron: None,
            episode_count_trigger: 0,
            quality_gain: Self::default_quality_gain(),
            quality_penalty: Self::default_quality_penalty(),
        }
    }
}

impl From<roko_core::config::execution::DreamScheduleConfig> for DreamsConfig {
    fn from(core: roko_core::config::execution::DreamScheduleConfig) -> Self {
        Self {
            auto_dream: core.auto_dream,
            idle_threshold_mins: core.idle_threshold_mins,
            min_episodes_for_dream: core.min_episodes_for_dream,
            scheduled_cron: core.scheduled_cron,
            episode_count_trigger: core.episode_count_trigger,
            quality_gain: core.quality_gain,
            quality_penalty: core.quality_penalty,
        }
    }
}

/// Daimon affect-engine configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DaimonConfig {
    /// Domain-specific strategy-space registration for somatic markers.
    #[serde(default)]
    pub strategy_space: StrategySpaceDefinition,
}

impl DaimonConfig {
    /// Convert from core schema `DaimonConfig` to this CLI adapter type.
    ///
    /// # Errors
    ///
    /// Returns an error if `strategy_space.dimensions` does not contain
    /// exactly 8 entries.
    pub fn from_core(core: &roko_core::config::execution::DaimonConfig) -> Result<Self> {
        let dims: [String; 8] =
            core.strategy_space
                .dimensions
                .clone()
                .try_into()
                .map_err(|values: Vec<String>| {
                    anyhow!(
                        "daimon.strategy_space.dimensions must contain exactly 8 entries, got {}",
                        values.len()
                    )
                })?;
        let def = StrategySpaceDefinition {
            domain: core.strategy_space.domain.clone(),
            dimensions: dims,
        };
        Ok(Self {
            strategy_space: def.validate()?,
        })
    }
}

impl Default for DaimonConfig {
    fn default() -> Self {
        Self {
            strategy_space: StrategySpaceDefinition::default(),
        }
    }
}

impl ExecAgentConfig {
    fn default_command() -> String {
        "cat".to_string()
    }

    const fn default_timeout() -> u64 {
        DEFAULT_REQUEST_TIMEOUT_MS
    }

    fn default_effort() -> String {
        "medium".to_string()
    }

    const fn default_bare_mode() -> bool {
        true
    }

    const fn default_clean() -> bool {
        true
    }
}

impl Default for ExecAgentConfig {
    fn default() -> Self {
        Self {
            command: Self::default_command(),
            args: Vec::new(),
            model: None,
            effort: Self::default_effort(),
            bare_mode: Self::default_bare_mode(),
            fallback_model: None,
            timeout_ms: Self::default_timeout(),
            env: Vec::new(),
            env_passthrough: Vec::new(),
            clean_output: Self::default_clean(),
            mcp_config: None,
            tier_models: std::collections::HashMap::new(),
            escalation: EscalationConfig::default(),
        }
    }
}

/// Retry escalation configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EscalationConfig {
    /// Maximum retries per task before failing.
    #[serde(default = "EscalationConfig::default_max_retries")]
    pub max_retries: u32,
    /// Whether to escalate to a higher-tier model on failure.
    #[serde(default = "EscalationConfig::default_escalate")]
    pub escalate_model: bool,
}

impl EscalationConfig {
    const fn default_max_retries() -> u32 {
        3
    }
    const fn default_escalate() -> bool {
        true
    }
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            max_retries: Self::default_max_retries(),
            escalate_model: Self::default_escalate(),
        }
    }
}

/// Cost budget configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BudgetConfig {
    /// Maximum USD spend per plan.
    #[serde(default = "BudgetConfig::default_max_plan")]
    pub max_plan_usd: f64,
    /// Maximum USD spend per agent turn.
    #[serde(default = "BudgetConfig::default_max_turn")]
    pub max_turn_usd: f64,
    /// Maximum USD spend per task.
    #[serde(default = "BudgetConfig::default_max_task")]
    pub max_task_usd: f64,
    /// Maximum USD spend per session (across all plans).
    #[serde(default = "BudgetConfig::default_max_session")]
    pub max_session_usd: f64,
    /// Warn at this percentage of budget consumed.
    #[serde(default = "BudgetConfig::default_warn_pct")]
    pub warn_at_percent: u32,
}

impl BudgetConfig {
    // The spend caps default to core's `[budget]` defaults (`0.0`, no cap),
    // so a key a file leaves out gets the same cap whether the file is the
    // workspace roko.toml or is passed with `roko --config`.
    fn default_max_plan() -> f64 {
        f64::from(roko_core::config::BudgetConfig::default().max_plan_usd)
    }
    fn default_max_task() -> f64 {
        f64::from(roko_core::config::BudgetConfig::default().max_task_usd)
    }
    fn default_max_turn() -> f64 {
        f64::from(roko_core::config::BudgetConfig::default().max_turn_usd)
    }
    /// `max_session_usd` is the v1 name of core's `max_plan_usd`.
    fn default_max_session() -> f64 {
        Self::default_max_plan()
    }
    const fn default_warn_pct() -> u32 {
        80
    }

    /// Take the spend caps from the core `[budget]` section.
    ///
    /// A cap of `0.0` means no cap in both. Settings the core section lacks
    /// keep this type's defaults.
    #[must_use]
    pub fn from_core(core: &roko_core::config::BudgetConfig) -> Self {
        Self {
            max_plan_usd: f64::from(core.max_plan_usd),
            max_turn_usd: f64::from(core.max_turn_usd),
            max_task_usd: f64::from(core.max_task_usd),
            ..Self::default()
        }
    }

    /// Return the USD spend at which the plan should start warning.
    #[must_use]
    pub fn warn_threshold_usd(&self) -> f64 {
        if self.max_plan_usd <= 0.0 {
            0.0
        } else {
            self.max_plan_usd * f64::from(self.warn_at_percent) / 100.0
        }
    }
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_plan_usd: Self::default_max_plan(),
            max_turn_usd: Self::default_max_turn(),
            max_task_usd: Self::default_max_task(),
            max_session_usd: Self::default_max_session(),
            warn_at_percent: Self::default_warn_pct(),
        }
    }
}

/// Durable runtime/control-plane configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeControlConfig {
    /// Path to the process-session ledger. Relative paths resolve from the workspace root.
    #[serde(default = "RuntimeControlConfig::default_process_session_ledger")]
    pub process_session_ledger: PathBuf,
    /// Maximum age for resumable process-session metadata before resume fails closed.
    #[serde(default = "RuntimeControlConfig::default_resume_max_staleness_secs")]
    pub resume_max_staleness_secs: u64,
}

impl RuntimeControlConfig {
    fn default_process_session_ledger() -> PathBuf {
        PathBuf::from(".roko/state/process-sessions.json")
    }

    const fn default_resume_max_staleness_secs() -> u64 {
        24 * 60 * 60
    }

    /// Resolve the configured ledger path against a workspace root.
    #[must_use]
    pub fn process_session_ledger_path(&self, workdir: &Path) -> PathBuf {
        if self.process_session_ledger.is_absolute() {
            self.process_session_ledger.clone()
        } else {
            workdir.join(&self.process_session_ledger)
        }
    }

    /// Return the resume staleness window in milliseconds.
    #[must_use]
    pub const fn resume_max_staleness_ms(&self) -> u64 {
        self.resume_max_staleness_secs.saturating_mul(1_000)
    }

    /// Validate runtime control-plane settings.
    pub fn validate(&self) -> Result<()> {
        if self.process_session_ledger.as_os_str().is_empty() {
            bail!("runtime.process_session_ledger must not be empty");
        }
        if self.resume_max_staleness_secs == 0 {
            bail!("runtime.resume_max_staleness_secs must be greater than zero");
        }
        Ok(())
    }
}

impl Default for RuntimeControlConfig {
    fn default() -> Self {
        Self {
            process_session_ledger: Self::default_process_session_ledger(),
            resume_max_staleness_secs: Self::default_resume_max_staleness_secs(),
        }
    }
}

/// Plan-level runner configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunnerConfig {
    /// Wall-clock timeout for the entire plan execution.
    #[serde(default = "RunnerConfig::default_plan_timeout_secs")]
    pub plan_timeout_secs: u64,
    /// When true, the agent subprocess is launched with
    /// `--dangerously-skip-permissions` so it can write files without
    /// interactive prompts.  Mirrors `runner.dangerously_skip_permissions` in
    /// `roko.toml`.  Default: `false`.
    #[serde(default)]
    pub dangerously_skip_permissions: bool,
}

impl RunnerConfig {
    const fn default_plan_timeout_secs() -> u64 {
        DEFAULT_PLAN_TIMEOUT_SECS
    }
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            plan_timeout_secs: Self::default_plan_timeout_secs(),
            dangerously_skip_permissions: false,
        }
    }
}

/// The prompt settings a command resolves at run time. No roko.toml key sets
/// them: `[prompt] role` and `token_budget` were removed (bug-d5051e).
#[derive(Clone, Debug)]
pub struct PromptConfig {
    /// Token budget for the chat system prompt (approximate — 4 bytes per
    /// token). `--effort` sets it for non-Claude backends.
    pub token_budget: usize,
    /// Agent role label (default `implementer`) that model selection uses.
    /// `--role` sets it.
    pub role: String,
}

impl PromptConfig {
    const fn default_budget() -> usize {
        10_000
    }

    fn default_role() -> String {
        "implementer".to_string()
    }
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            token_budget: Self::default_budget(),
            role: Self::default_role(),
        }
    }
}

/// Partial `DreamsConfig` — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DreamsLayer {
    /// Enable the automatic dream cycle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_dream: Option<bool>,
    /// Idle threshold in minutes before dreaming.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_threshold_mins: Option<u64>,
    /// Minimum number of new episodes required before dreaming.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_episodes_for_dream: Option<usize>,
    /// Optional fallback cron expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_cron: Option<String>,
    /// Optional unconsolidated-episode trigger threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode_count_trigger: Option<usize>,
    /// Optional high-quality idle-delay multiplier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_gain: Option<f64>,
    /// Optional low-quality idle-delay multiplier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_penalty: Option<f64>,
}

impl DreamsLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            auto_dream: overlay.auto_dream.or(self.auto_dream),
            idle_threshold_mins: overlay.idle_threshold_mins.or(self.idle_threshold_mins),
            min_episodes_for_dream: overlay
                .min_episodes_for_dream
                .or(self.min_episodes_for_dream),
            scheduled_cron: overlay.scheduled_cron.or(self.scheduled_cron),
            episode_count_trigger: overlay.episode_count_trigger.or(self.episode_count_trigger),
            quality_gain: overlay.quality_gain.or(self.quality_gain),
            quality_penalty: overlay.quality_penalty.or(self.quality_penalty),
        }
    }

    /// Resolve into a concrete [`DreamsConfig`] value.
    #[must_use]
    pub fn resolve(self) -> DreamsConfig {
        let defaults = DreamsConfig::default();
        DreamsConfig {
            auto_dream: self.auto_dream.unwrap_or(defaults.auto_dream),
            idle_threshold_mins: self
                .idle_threshold_mins
                .unwrap_or(defaults.idle_threshold_mins),
            min_episodes_for_dream: self
                .min_episodes_for_dream
                .unwrap_or(defaults.min_episodes_for_dream),
            scheduled_cron: self.scheduled_cron.or(defaults.scheduled_cron),
            episode_count_trigger: self
                .episode_count_trigger
                .unwrap_or(defaults.episode_count_trigger),
            quality_gain: self.quality_gain.unwrap_or(defaults.quality_gain),
            quality_penalty: self.quality_penalty.unwrap_or(defaults.quality_penalty),
        }
    }
}

/// Partial `DaimonConfig` — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DaimonLayer {
    /// Domain-specific strategy-space registration for somatic markers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_space: Option<StrategySpaceLayer>,
}

impl DaimonLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            strategy_space: match (self.strategy_space, overlay.strategy_space) {
                (Some(base), Some(overlay)) => Some(base.merge(overlay)),
                (None, Some(overlay)) => Some(overlay),
                (Some(base), None) => Some(base),
                (None, None) => None,
            },
        }
    }

    /// Resolve into a concrete [`DaimonConfig`] value.
    pub fn resolve(self) -> Result<DaimonConfig> {
        let defaults = DaimonConfig::default();
        Ok(DaimonConfig {
            strategy_space: match self.strategy_space {
                Some(strategy_space) => strategy_space.resolve()?,
                None => defaults.strategy_space,
            },
        })
    }
}

/// Partial strategy-space registration config.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct StrategySpaceLayer {
    /// Domain identifier for this strategy-space mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Human-readable labels for the fixed 8 dimensions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<String>>,
}

impl StrategySpaceLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            domain: overlay.domain.or(self.domain),
            dimensions: overlay.dimensions.or(self.dimensions),
        }
    }

    /// Resolve into a validated [`StrategySpaceDefinition`].
    pub fn resolve(self) -> Result<StrategySpaceDefinition> {
        let defaults = StrategySpaceDefinition::default();
        let domain = self.domain.unwrap_or(defaults.domain);
        let dimensions_vec = self
            .dimensions
            .unwrap_or_else(|| defaults.dimensions.into_iter().collect());
        let dimensions: [String; 8] =
            dimensions_vec.try_into().map_err(|values: Vec<String>| {
                anyhow!(
                    "daimon.strategy_space.dimensions must contain exactly 8 entries, got {}",
                    values.len()
                )
            })?;
        StrategySpaceDefinition { domain, dimensions }.validate()
    }
}

/// Per-repository configuration inside `roko.toml`.
///
/// Re-exported from `roko_core::config::execution::RepoConfig` for source
/// compatibility. The schema-only definition now lives in the core crate.
pub use roko_core::config::execution::RepoConfig;

/// Loaded runtime data for a configured repository.
#[derive(Clone, Debug)]
pub struct RepoEntry {
    /// Declarative repo config from `roko.toml`.
    pub config: RepoConfig,
    /// Canonical repository root.
    pub root: PathBuf,
    /// Optional repo-local `.roko/roko.toml` config.
    pub roko_config: Option<RokoConfig>,
    /// Path to the repo-local config when it exists.
    pub roko_config_path: Option<PathBuf>,
}

/// Runtime registry of configured repositories.
#[derive(Clone, Debug, Default)]
pub struct RepoRegistry {
    repos: Vec<RepoEntry>,
}

impl RepoRegistry {
    /// Load and validate all configured repos.
    pub fn load(config: &Config, workdir: &Path) -> Result<Self> {
        let mut repos = Vec::with_capacity(config.repos.len());
        let mut seen_names = std::collections::HashSet::new();

        for repo in &config.repos {
            if repo.name.trim().is_empty() {
                return Err(anyhow!("configured repo name must not be empty"));
            }
            if !seen_names.insert(repo.name.clone()) {
                return Err(anyhow!("duplicate configured repo name: {}", repo.name));
            }

            let root = Self::resolve_root(repo, workdir)?;
            let (roko_config, roko_config_path) = Self::load_repo_config(&root, &repo.name)?;

            repos.push(RepoEntry {
                config: repo.clone(),
                root,
                roko_config,
                roko_config_path,
            });
        }

        Ok(Self { repos })
    }

    /// Return all loaded repo entries.
    #[must_use]
    pub fn repos(&self) -> &[RepoEntry] {
        &self.repos
    }

    /// True when no repos are configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.repos.is_empty()
    }

    /// Find a repo by configured name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&RepoEntry> {
        self.repos.iter().find(|repo| repo.config.name == name)
    }

    /// Find the repo whose name matches a repository full-name from a webhook
    /// signal payload (e.g. `"owner/repo"`). Falls back to matching the bare
    /// repo name portion.
    #[must_use]
    pub fn find_by_full_name(&self, full_name: &str) -> Option<&RepoEntry> {
        // Exact name match first.
        if let Some(entry) = self.get(full_name) {
            return Some(entry);
        }
        // Match bare name (e.g. "my-repo" in "owner/my-repo").
        let bare = full_name.rsplit('/').next().unwrap_or(full_name);
        self.repos.iter().find(|entry| entry.config.name == bare)
    }

    fn resolve_root(repo: &RepoConfig, workdir: &Path) -> Result<PathBuf> {
        let configured = if repo.path.is_absolute() {
            repo.path.clone()
        } else {
            workdir.join(&repo.path)
        };
        let root = configured.canonicalize().with_context(|| {
            format!("resolve repo '{}' path {}", repo.name, configured.display())
        })?;
        if !root.is_dir() {
            return Err(anyhow!(
                "configured repo '{}' path is not a directory: {}",
                repo.name,
                root.display()
            ));
        }
        Ok(root)
    }

    fn load_repo_config(
        root: &Path,
        repo_name: &str,
    ) -> Result<(Option<RokoConfig>, Option<PathBuf>)> {
        let path = root.join(".roko").join("roko.toml");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((None, None)),
            Err(e) => {
                return Err(
                    anyhow::Error::new(e).context(format!("read repo config {}", path.display()))
                );
            }
        };
        let config = RokoConfig::from_toml(&text)
            .map_err(|err| anyhow!(err))
            .with_context(|| {
                format!(
                    "parse repo config {} for repo {}",
                    path.display(),
                    repo_name
                )
            })?;
        Ok((Some(config), Some(path)))
    }
}

/// One gate entry in `roko.toml`. Multiple gates run in declaration order.
///
/// The `kind` field selects the gate type. Each variant has its own fields
/// (currently only `Shell` is fully configurable; other kinds accept a
/// `build_system` tag).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GateConfig {
    /// Arbitrary shell command; passes on exit code 0.
    Shell {
        /// Program to invoke.
        program: String,
        /// Args to pass.
        #[serde(default)]
        args: Vec<String>,
        /// Timeout in milliseconds.
        #[serde(default = "default_gate_timeout")]
        timeout_ms: u64,
    },
    /// `cargo check` (or equivalent) run in the working dir.
    Compile {
        /// Build system (cargo, npm, go, python, forge, make).
        #[serde(default = "default_build_system")]
        build_system: String,
        /// Timeout in milliseconds.
        #[serde(default = "default_gate_timeout_long")]
        timeout_ms: u64,
    },
    /// `cargo clippy` (or equivalent lint command).
    Clippy {
        /// Build system.
        #[serde(default = "default_build_system")]
        build_system: String,
        /// Timeout in milliseconds.
        #[serde(default = "default_gate_timeout_long")]
        timeout_ms: u64,
    },
    /// `cargo test` (or equivalent test command).
    Test {
        /// Build system.
        #[serde(default = "default_build_system")]
        build_system: String,
        /// Timeout in milliseconds.
        #[serde(default = "default_gate_timeout_long")]
        timeout_ms: u64,
    },
}

impl GateConfig {
    /// A default `shell` gate that runs `true` (always passes). Useful as a
    /// placeholder in `roko init` output.
    #[must_use]
    pub fn default_shell_true() -> Self {
        Self::Shell {
            program: "true".into(),
            args: Vec::new(),
            timeout_ms: default_gate_timeout(),
        }
    }
}

const fn default_gate_timeout() -> u64 {
    60_000
}

const fn default_gate_timeout_long() -> u64 {
    600_000
}

fn default_build_system() -> String {
    "cargo".into()
}

// -----------------------------------------------------------------------
// Layered config: global (~/.config/roko/config.toml) + project (./roko.toml)
// -----------------------------------------------------------------------

/// Where each field in a [`ResolvedConfig`] came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Value came from the global config file.
    Global,
    /// Value came from the project-local `roko.toml`.
    Project,
    /// Value is the built-in default.
    Default,
    /// Value came from `ROKO_CONFIG` or a `ROKO__*` override.
    Env,
}

impl Source {
    /// Short tag printed by `roko config show`.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Global => "[global]",
            Self::Project => "[project]",
            Self::Default => "[default]",
            Self::Env => "[env]",
        }
    }
}

// NOTE: The legacy `ConfigLayer` struct has been eliminated. Config loading
// goes through `load_resolved_config()` which delegates to the core
// `roko_core::config::loader`. Config writing (`config set`, `config init`)
// operates directly on raw `toml::Value` trees via `set_toml_dotted_key()`.

/// Partial overrides for `[learning]`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LearningLayer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replan_on_gate_failure: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_playbook_refresh: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_lookahead_router: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookahead_threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_threshold_flush_interval: Option<u64>,
}

impl LearningLayer {
    /// Construct from the core `LearningConfig` (all fields are populated).
    #[must_use]
    pub fn from_core_learning(core: &roko_core::config::LearningConfig) -> Self {
        Self {
            replan_on_gate_failure: Some(core.replan_on_gate_failure),
            auto_playbook_refresh: Some(core.auto_playbook_refresh),
            use_lookahead_router: Some(core.use_lookahead_router),
            lookahead_threshold: Some(core.lookahead_threshold),
            gate_threshold_flush_interval: Some(core.gate_threshold_flush_interval),
        }
    }

    pub fn merge(self, overlay: Self) -> Self {
        Self {
            replan_on_gate_failure: overlay
                .replan_on_gate_failure
                .or(self.replan_on_gate_failure),
            auto_playbook_refresh: overlay.auto_playbook_refresh.or(self.auto_playbook_refresh),
            use_lookahead_router: overlay.use_lookahead_router.or(self.use_lookahead_router),
            lookahead_threshold: overlay.lookahead_threshold.or(self.lookahead_threshold),
            gate_threshold_flush_interval: overlay
                .gate_threshold_flush_interval
                .or(self.gate_threshold_flush_interval),
        }
    }
}

/// Partial provider config used for layered merges.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProviderLayer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ProviderKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft_timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    /// Per-provider rate limits (RPM + TPM).
    ///
    /// Populated from `[providers.<name>.limits]` in `roko.toml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<roko_core::config::provider::ProviderLimits>,
}

impl ProviderLayer {
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        // If the overlay changes `kind`, don't inherit kind-specific fields
        // (api_key_env, base_url, command, args) from the base — they belong
        // to a different provider type and would cause misrouting.
        let kind_changed = overlay.kind.is_some() && overlay.kind != self.kind;
        if kind_changed {
            Self {
                kind: overlay.kind,
                base_url: overlay.base_url,
                api_key_env: overlay.api_key_env,
                command: overlay.command,
                args: overlay.args,
                // Timeouts are kind-agnostic, safe to inherit
                timeout_ms: overlay.timeout_ms.or(self.timeout_ms),
                ttft_timeout_ms: overlay.ttft_timeout_ms.or(self.ttft_timeout_ms),
                connect_timeout_ms: overlay.connect_timeout_ms.or(self.connect_timeout_ms),
                extra_headers: overlay.extra_headers.or(self.extra_headers),
                max_concurrent: overlay.max_concurrent.or(self.max_concurrent),
                limits: overlay.limits.or(self.limits),
            }
        } else {
            Self {
                kind: overlay.kind.or(self.kind),
                base_url: overlay.base_url.or(self.base_url),
                api_key_env: overlay.api_key_env.or(self.api_key_env),
                command: overlay.command.or(self.command),
                args: overlay.args.or(self.args),
                timeout_ms: overlay.timeout_ms.or(self.timeout_ms),
                ttft_timeout_ms: overlay.ttft_timeout_ms.or(self.ttft_timeout_ms),
                connect_timeout_ms: overlay.connect_timeout_ms.or(self.connect_timeout_ms),
                extra_headers: overlay.extra_headers.or(self.extra_headers),
                max_concurrent: overlay.max_concurrent.or(self.max_concurrent),
                limits: overlay.limits.or(self.limits),
            }
        }
    }

    pub fn resolve(self) -> Result<ProviderConfig> {
        Ok(ProviderConfig {
            kind: self.kind.context("missing required field `kind`")?,
            base_url: self.base_url,
            api_key_env: self.api_key_env,
            command: self.command,
            args: self.args,
            timeout_ms: self.timeout_ms.or(Some(DEFAULT_REQUEST_TIMEOUT_MS)),
            ttft_timeout_ms: self.ttft_timeout_ms.or(Some(DEFAULT_TTFT_TIMEOUT_MS)),
            connect_timeout_ms: self.connect_timeout_ms.or(Some(DEFAULT_CONNECT_TIMEOUT_MS)),
            extra_headers: self.extra_headers,
            max_concurrent: self.max_concurrent,
            limits: self.limits,
            require_confirmation: false,
        })
    }
}

/// Partial OpenRouter routing overrides used for layered merges.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProviderRoutingLayer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<Vec<String>>,
}

impl ProviderRoutingLayer {
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            sort: overlay.sort.or(self.sort),
            order: overlay.order.or(self.order),
            allow_fallbacks: overlay.allow_fallbacks.or(self.allow_fallbacks),
            max_price: overlay.max_price.or(self.max_price),
            require_parameters: overlay.require_parameters.or(self.require_parameters),
        }
    }

    #[must_use]
    pub fn resolve(self) -> ProviderRouting {
        ProviderRouting {
            sort: self.sort,
            order: self.order,
            allow_fallbacks: self.allow_fallbacks,
            max_price: self.max_price,
            require_parameters: self.require_parameters,
        }
    }
}

/// Partial model profile used for layered merges.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ModelProfileLayer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_tools: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_thinking: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_vision: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_web_search: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_mcp_tools: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_partial: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_grounding: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_code_execution: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_caching: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_routing: Option<ProviderRoutingLayer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_input_per_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_output_per_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_input_per_m_high: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_output_per_m_high: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_cache_read_per_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_cache_write_per_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tools: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokenizer_ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_search: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_citations: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_async: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_embedding_model: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_request: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<roko_core::ModelTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_max_completion_tokens: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tool_iterations: Option<u32>,
}

impl ModelProfileLayer {
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            provider: overlay.provider.or(self.provider),
            slug: overlay.slug.or(self.slug),
            context_window: overlay.context_window.or(self.context_window),
            max_output: overlay.max_output.or(self.max_output),
            supports_tools: overlay.supports_tools.or(self.supports_tools),
            supports_thinking: overlay.supports_thinking.or(self.supports_thinking),
            supports_vision: overlay.supports_vision.or(self.supports_vision),
            supports_web_search: overlay.supports_web_search.or(self.supports_web_search),
            supports_mcp_tools: overlay.supports_mcp_tools.or(self.supports_mcp_tools),
            supports_partial: overlay.supports_partial.or(self.supports_partial),
            supports_grounding: overlay.supports_grounding.or(self.supports_grounding),
            supports_code_execution: overlay
                .supports_code_execution
                .or(self.supports_code_execution),
            supports_caching: overlay.supports_caching.or(self.supports_caching),
            provider_routing: match (self.provider_routing, overlay.provider_routing) {
                (Some(base), Some(overlay)) => Some(base.merge(overlay)),
                (None, Some(overlay)) => Some(overlay),
                (Some(base), None) => Some(base),
                (None, None) => None,
            },
            tool_format: overlay.tool_format.or(self.tool_format),
            cost_input_per_m: overlay.cost_input_per_m.or(self.cost_input_per_m),
            cost_output_per_m: overlay.cost_output_per_m.or(self.cost_output_per_m),
            cost_input_per_m_high: overlay.cost_input_per_m_high.or(self.cost_input_per_m_high),
            cost_output_per_m_high: overlay
                .cost_output_per_m_high
                .or(self.cost_output_per_m_high),
            cost_cache_read_per_m: overlay.cost_cache_read_per_m.or(self.cost_cache_read_per_m),
            cost_cache_write_per_m: overlay
                .cost_cache_write_per_m
                .or(self.cost_cache_write_per_m),
            thinking_level: overlay.thinking_level.or(self.thinking_level),
            max_tools: overlay.max_tools.or(self.max_tools),
            tokenizer_ratio: overlay.tokenizer_ratio.or(self.tokenizer_ratio),
            supports_search: overlay.supports_search.or(self.supports_search),
            supports_citations: overlay.supports_citations.or(self.supports_citations),
            supports_async: overlay.supports_async.or(self.supports_async),
            is_embedding_model: overlay.is_embedding_model.or(self.is_embedding_model),
            search_context_size: overlay.search_context_size.or(self.search_context_size),
            cost_per_request: overlay.cost_per_request.or(self.cost_per_request),
            tier: overlay.tier.or(self.tier),
            use_max_completion_tokens: overlay
                .use_max_completion_tokens
                .or(self.use_max_completion_tokens),
            max_tool_iterations: overlay.max_tool_iterations.or(self.max_tool_iterations),
        }
    }

    pub fn resolve(self) -> Result<ModelProfile> {
        Ok(ModelProfile {
            provider: self.provider.context("missing required field `provider`")?,
            slug: self.slug.context("missing required field `slug`")?,
            context_window: self.context_window.unwrap_or(128_000),
            max_output: self.max_output,
            supports_tools: self.supports_tools.unwrap_or(true),
            supports_thinking: self.supports_thinking.unwrap_or(false),
            supports_vision: self.supports_vision.unwrap_or(false),
            supports_web_search: self.supports_web_search.unwrap_or(false),
            supports_mcp_tools: self.supports_mcp_tools.unwrap_or(false),
            supports_partial: self.supports_partial.unwrap_or(false),
            supports_grounding: self.supports_grounding.unwrap_or(false),
            supports_code_execution: self.supports_code_execution.unwrap_or(false),
            supports_caching: self.supports_caching.unwrap_or(false),
            provider_routing: self.provider_routing.map(ProviderRoutingLayer::resolve),
            tool_format: self
                .tool_format
                .unwrap_or_else(|| "openai_json".to_string()),
            cost_input_per_m: self.cost_input_per_m,
            cost_output_per_m: self.cost_output_per_m,
            cost_input_per_m_high: self.cost_input_per_m_high,
            cost_output_per_m_high: self.cost_output_per_m_high,
            cost_cache_read_per_m: self.cost_cache_read_per_m,
            cost_cache_write_per_m: self.cost_cache_write_per_m,
            thinking_level: self.thinking_level,
            max_tools: self.max_tools,
            tokenizer_ratio: self.tokenizer_ratio,
            supports_search: self.supports_search.unwrap_or(false),
            supports_citations: self.supports_citations.unwrap_or(false),
            supports_async: self.supports_async.unwrap_or(false),
            is_embedding_model: self.is_embedding_model.unwrap_or(false),
            search_context_size: self.search_context_size,
            cost_per_request: self.cost_per_request,
            use_max_completion_tokens: self.use_max_completion_tokens.unwrap_or(false),
            max_tool_iterations: self.max_tool_iterations,
            tier: self.tier,
        })
    }
}

/// Parse `text`, a `roko.toml`, into `T` with `${VAR}` references expanded.
///
/// An unknown key inside a `[providers.*]` or `[models.*]` entry is dropped
/// with a warning, as the config loader drops it, rather than failing the
/// parse (`strip_unknown_entry_fields`). So is a key roko removed, such as
/// `[executor]` (`drop_removed_config_keys`).
fn parse_toml_with_env<T>(text: &str, context: &'static str) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut value: toml::Value = toml::from_str(text).context(context)?;
    let mut diagnostics = roko_core::config::loader::drop_removed_config_keys(&mut value);
    let stripped = roko_core::config::loader::strip_unknown_entry_fields(&mut value);
    diagnostics.extend(stripped);
    for diagnostic in diagnostics {
        tracing::warn!(
            config_key = %diagnostic.key,
            "config warning: {}",
            diagnostic.message
        );
    }
    interpolate_env_values(&mut value)?;
    value
        .try_into()
        .map_err(|err| anyhow!(err))
        .context(context)
}

/// Parse a user-supplied value string into a `toml::Value` appropriate for the
/// given dotted key, then set it in the TOML document.
///
/// This replaces the old typed `ConfigLayer` approach: instead of maintaining a
/// parallel Option-wrapped schema we operate directly on the raw TOML tree,
/// preserving sparse serialization (only set keys appear in the file).
pub(crate) fn set_toml_dotted_key(doc: &mut toml::Value, key: &str, value: &str) -> Result<()> {
    let parsed = parse_value_for_key(key, value)?;
    let segments: Vec<&str> = key.split('.').collect();
    if segments.is_empty() {
        bail!("empty key");
    }

    // Walk/create intermediate tables.
    let _table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow!("config root is not a table"))?;
    // Work in-place on `doc` (table validation above guarantees it is a table).
    let mut cursor: &mut toml::Value = doc;
    for segment in &segments[..segments.len() - 1] {
        let tbl = cursor
            .as_table_mut()
            .ok_or_else(|| anyhow!("expected table at key segment '{segment}'"))?;
        cursor = tbl
            .entry(*segment)
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    }
    let leaf_key = segments[segments.len() - 1];
    let tbl = cursor
        .as_table_mut()
        .ok_or_else(|| anyhow!("expected table for leaf key '{leaf_key}'"))?;
    tbl.insert(leaf_key.to_string(), parsed);
    Ok(())
}

/// Determine the expected TOML type for a known dotted key and parse `value`
/// accordingly. Returns an error for unknown keys, and for a key roko removed,
/// with the reason it went.
fn parse_value_for_key(key: &str, value: &str) -> Result<toml::Value> {
    if let Some(reason) = roko_core::config::loader::removed_config_key_reason(key) {
        bail!("cannot set {key}: {reason}");
    }
    let segments: Vec<&str> = key.split('.').collect();
    match segments.as_slice() {
        // Booleans
        ["auto_plan"]
        | ["agent", "bare_mode"]
        | ["agent", "clean_output"]
        | ["dreams", "auto_dream"]
        | ["serve", "auto_start"]
        | ["serve", "auth", "enabled"]
        | ["learning", "replan_on_gate_failure"]
        | ["learning", "auto_playbook_refresh"]
        | ["learning", "use_lookahead_router"] => {
            let b = value
                .parse::<bool>()
                .with_context(|| format!("parse {key} as bool"))?;
            Ok(toml::Value::Boolean(b))
        }
        // Integers (u64 / usize / u32 — all stored as TOML Integer)
        ["agent", "timeout_ms"]
        | ["dreams", "idle_threshold_mins"]
        | ["dreams", "min_episodes_for_dream"]
        | ["dreams", "episode_count_trigger"]
        | ["runner", "plan_timeout_secs"]
        | ["learning", "gate_threshold_flush_interval"] => {
            let n = value
                .parse::<i64>()
                .with_context(|| format!("parse {key} as integer"))?;
            Ok(toml::Value::Integer(n))
        }
        // Floats
        ["dreams", "quality_gain"]
        | ["dreams", "quality_penalty"]
        | ["learning", "lookahead_threshold"] => {
            let f = value
                .parse::<f64>()
                .with_context(|| format!("parse {key} as float"))?;
            Ok(toml::Value::Float(f))
        }
        // Plain strings
        ["agent", "command"]
        | ["agent", "model"]
        | ["agent", "default_model"]
        | ["agent", "effort"]
        | ["agent", "fallback_model"]
        | ["agent", "mcp_config"]
        | ["authoring", "planner_model"]
        | ["dreams", "scheduled_cron"]
        | ["serve", "auth", "api_key"]
        | ["serve", "deploy", "provider"]
        | ["daimon", "strategy_space", "domain"] => Ok(toml::Value::String(value.to_string())),
        // String arrays (accept JSON array or whitespace-separated)
        ["agent", "args"]
        | ["daimon", "strategy_space", "dimensions"]
        | ["serve", "deploy", "environment"] => {
            let items = parse_string_list(value, &format!("parse {key} as string list"))?;
            let arr = items.into_iter().map(toml::Value::String).collect();
            Ok(toml::Value::Array(arr))
        }
        // JSON-parsed complex values
        ["agent", "env"] | ["serve", "deploy", "webhooks"] => {
            let json_val: serde_json::Value =
                serde_json::from_str(value).with_context(|| format!("parse {key} as JSON"))?;
            json_to_toml(&json_val).with_context(|| format!("convert {key} JSON to TOML"))
        }
        ["providers", name, "extra_headers"] => {
            let json_val: serde_json::Value = serde_json::from_str(value)
                .with_context(|| format!("parse providers.{name}.extra_headers as JSON object"))?;
            json_to_toml(&json_val)
                .with_context(|| format!("convert providers.{name}.extra_headers to TOML"))
        }
        // Provider fields with dynamic name
        ["providers", _, "kind"] => {
            // Validate it's a known ProviderKind by attempting deserialisation.
            let _kind: ProviderKind =
                serde_json::from_value(serde_json::Value::String(value.to_string()))
                    .context("parse provider kind")?;
            Ok(toml::Value::String(value.to_string()))
        }
        ["providers", _, "base_url"]
        | ["providers", _, "api_key_env"]
        | ["providers", _, "command"] => Ok(toml::Value::String(value.to_string())),
        ["providers", _, "args"] => {
            let items = parse_string_list(value, "parse provider args as string list")?;
            let arr = items.into_iter().map(toml::Value::String).collect();
            Ok(toml::Value::Array(arr))
        }
        ["providers", _, "timeout_ms"]
        | ["providers", _, "ttft_timeout_ms"]
        | ["providers", _, "connect_timeout_ms"]
        | ["providers", _, "max_concurrent"] => {
            let n = value
                .parse::<i64>()
                .with_context(|| format!("parse {key} as integer"))?;
            Ok(toml::Value::Integer(n))
        }
        // Model fields with dynamic name
        ["models", _, "provider"]
        | ["models", _, "slug"]
        | ["models", _, "tool_format"]
        | ["models", _, "thinking_level"]
        | ["models", _, "search_context_size"] => Ok(toml::Value::String(value.to_string())),
        ["models", _, "context_window"]
        | ["models", _, "max_output"]
        | ["models", _, "max_tools"] => {
            let n = value
                .parse::<i64>()
                .with_context(|| format!("parse {key} as integer"))?;
            Ok(toml::Value::Integer(n))
        }
        ["models", _, "supports_tools"]
        | ["models", _, "supports_thinking"]
        | ["models", _, "supports_vision"]
        | ["models", _, "supports_web_search"]
        | ["models", _, "supports_mcp_tools"]
        | ["models", _, "supports_partial"]
        | ["models", _, "supports_grounding"]
        | ["models", _, "supports_code_execution"]
        | ["models", _, "supports_caching"]
        | ["models", _, "supports_search"]
        | ["models", _, "supports_citations"]
        | ["models", _, "supports_async"]
        | ["models", _, "is_embedding_model"]
        | ["models", _, "use_max_completion_tokens"] => {
            let b = value
                .parse::<bool>()
                .with_context(|| format!("parse {key} as bool"))?;
            Ok(toml::Value::Boolean(b))
        }
        ["models", _, "cost_input_per_m"]
        | ["models", _, "cost_output_per_m"]
        | ["models", _, "cost_input_per_m_high"]
        | ["models", _, "cost_output_per_m_high"]
        | ["models", _, "cost_cache_read_per_m"]
        | ["models", _, "cost_cache_write_per_m"]
        | ["models", _, "cost_per_request"]
        | ["models", _, "tokenizer_ratio"] => {
            let f = value
                .parse::<f64>()
                .with_context(|| format!("parse {key} as float"))?;
            Ok(toml::Value::Float(f))
        }
        // Model routing sub-keys
        ["models", _, "provider_routing", "sort"] => Ok(toml::Value::String(value.to_string())),
        ["models", _, "provider_routing", "order"]
        | ["models", _, "provider_routing", "require_parameters"] => {
            let items = parse_string_list(value, &format!("parse {key} as string list"))?;
            let arr = items.into_iter().map(toml::Value::String).collect();
            Ok(toml::Value::Array(arr))
        }
        ["models", _, "provider_routing", "allow_fallbacks"] => {
            let b = value
                .parse::<bool>()
                .with_context(|| format!("parse {key} as bool"))?;
            Ok(toml::Value::Boolean(b))
        }
        ["models", _, "provider_routing", "max_price"] => {
            let f = value
                .parse::<f64>()
                .with_context(|| format!("parse {key} as float"))?;
            Ok(toml::Value::Float(f))
        }
        _ => parse_value_from_schema(key, value),
    }
}

/// Parse `value` for a key the table above does not list, by the TOML type
/// the v2 schema gives the key (`budget.max_plan_usd` is a float). A key
/// that `roko config validate` would not accept is unknown.
fn parse_value_from_schema(key: &str, value: &str) -> Result<toml::Value> {
    let expected = roko_core::config::loader::schema_value_for_path(key)
        .ok_or_else(|| anyhow!("unknown key: {key}"))?;
    match expected {
        toml::Value::Boolean(_) => {
            let b = value
                .parse::<bool>()
                .with_context(|| format!("parse {key} as bool"))?;
            Ok(toml::Value::Boolean(b))
        }
        toml::Value::Integer(_) => {
            let n = value
                .parse::<i64>()
                .with_context(|| format!("parse {key} as integer"))?;
            Ok(toml::Value::Integer(n))
        }
        toml::Value::Float(_) => {
            let f = value
                .parse::<f64>()
                .with_context(|| format!("parse {key} as float"))?;
            Ok(toml::Value::Float(f))
        }
        toml::Value::String(_) => Ok(toml::Value::String(value.to_string())),
        toml::Value::Datetime(_) => {
            let datetime = value
                .parse::<toml::value::Datetime>()
                .with_context(|| format!("parse {key} as datetime"))?;
            Ok(toml::Value::Datetime(datetime))
        }
        // Arrays take a JSON array or whitespace-separated strings.
        toml::Value::Array(_) if !value.trim_start().starts_with('[') => {
            let items = value
                .split_whitespace()
                .map(|item| toml::Value::String(item.to_string()))
                .collect();
            Ok(toml::Value::Array(items))
        }
        toml::Value::Array(_) | toml::Value::Table(_) => {
            let json_val: serde_json::Value =
                serde_json::from_str(value).with_context(|| format!("parse {key} as JSON"))?;
            json_to_toml(&json_val).with_context(|| format!("convert {key} JSON to TOML"))
        }
    }
}

fn parse_string_list(value: &str, context: &str) -> Result<Vec<String>> {
    if value.trim_start().starts_with('[') {
        serde_json::from_str(value).context(context.to_string())
    } else {
        Ok(value.split_whitespace().map(String::from).collect())
    }
}

/// Convert a `serde_json::Value` into a `toml::Value`.
fn json_to_toml(json: &serde_json::Value) -> Result<toml::Value> {
    match json {
        serde_json::Value::Null => Ok(toml::Value::String(String::new())),
        serde_json::Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml::Value::Float(f))
            } else {
                bail!("unsupported JSON number: {n}")
            }
        }
        serde_json::Value::String(s) => Ok(toml::Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let items: Result<Vec<_>> = arr.iter().map(json_to_toml).collect();
            Ok(toml::Value::Array(items?))
        }
        serde_json::Value::Object(map) => {
            let mut table = toml::map::Map::new();
            for (k, v) in map {
                table.insert(k.clone(), json_to_toml(v)?);
            }
            Ok(toml::Value::Table(table))
        }
    }
}

/// Read a config file as a raw `toml::Value`, preserving only the keys that
/// were actually set in the file (no default inflation).
pub(crate) fn read_toml_file(path: &Path) -> Result<toml::Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("read config {}", path.display()))?;
    let value: toml::Value =
        toml::from_str(&text).with_context(|| format!("parse config {}", path.display()))?;
    Ok(value)
}

/// Write a `toml::Value` back to a file as pretty TOML.
pub(crate) fn write_toml_file(path: &Path, value: &toml::Value) -> Result<()> {
    let rendered = toml::to_string_pretty(value).context("serialize config")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(path, rendered).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn interpolate_env_values(value: &mut toml::Value) -> Result<()> {
    match value {
        toml::Value::String(s) => {
            *s = interpolate_env_string(s)?;
        }
        toml::Value::Array(items) => {
            for item in items {
                interpolate_env_values(item)?;
            }
        }
        toml::Value::Table(entries) => {
            for (_, item) in entries.iter_mut() {
                interpolate_env_values(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn interpolate_env_string(input: &str) -> Result<String> {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            output.push_str(&rest[start..]);
            return Ok(output);
        };
        let expr = &after[..end];

        // Split on `:-` to support ${VAR:-fallback} syntax.
        let (var_name, default_value) = if let Some(sep) = expr.find(":-") {
            (&expr[..sep], Some(&expr[sep + 2..]))
        } else {
            (expr, None)
        };

        if var_name.is_empty()
            || !var_name
                .chars()
                .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            output.push_str("${");
            rest = &rest[start + 2..];
            continue;
        }
        let value = match std::env::var(var_name) {
            Ok(v) => v,
            Err(_) => {
                if let Some(fallback) = default_value {
                    fallback.to_string()
                } else {
                    tracing::warn!(
                        %var_name,
                        "env var referenced but not set; using empty string"
                    );
                    String::new()
                }
            }
        };
        output.push_str(&value);
        rest = &after[end + 1..];
    }

    output.push_str(rest);
    Ok(output)
}

/// Partial `AgentConfig` — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AgentLayer {
    /// Program to invoke.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Extra args.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Preferred model slug.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "default_model"
    )]
    pub model: Option<String>,
    /// Claude effort level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// Claude bare-mode toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bare_mode: Option<bool>,
    /// Claude fallback model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,
    /// Subprocess timeout in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Env vars for the agent subprocess.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<(String, String)>>,
    /// Whether to strip ANSI + thinking traces from agent output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clean_output: Option<bool>,
    /// Optional explicit MCP config path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_config: Option<PathBuf>,
}

impl AgentLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            command: overlay.command.or(self.command),
            args: overlay.args.or(self.args),
            model: overlay.model.or(self.model),
            effort: overlay.effort.or(self.effort),
            bare_mode: overlay.bare_mode.or(self.bare_mode),
            fallback_model: overlay.fallback_model.or(self.fallback_model),
            timeout_ms: overlay.timeout_ms.or(self.timeout_ms),
            env: overlay.env.or(self.env),
            clean_output: overlay.clean_output.or(self.clean_output),
            mcp_config: overlay.mcp_config.or(self.mcp_config),
        }
    }
}

/// Partial `RuntimeControlConfig` — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RuntimeControlLayer {
    /// Path to the process-session ledger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_session_ledger: Option<PathBuf>,
    /// Maximum process-session metadata age before resume fails closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_max_staleness_secs: Option<u64>,
}

impl RuntimeControlLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            process_session_ledger: overlay
                .process_session_ledger
                .or(self.process_session_ledger),
            resume_max_staleness_secs: overlay
                .resume_max_staleness_secs
                .or(self.resume_max_staleness_secs),
        }
    }

    /// Resolve into a validated [`RuntimeControlConfig`].
    pub fn resolve(self) -> Result<RuntimeControlConfig> {
        let defaults = RuntimeControlConfig::default();
        let config = RuntimeControlConfig {
            process_session_ledger: self
                .process_session_ledger
                .unwrap_or(defaults.process_session_ledger),
            resume_max_staleness_secs: self
                .resume_max_staleness_secs
                .unwrap_or(defaults.resume_max_staleness_secs),
        };
        config.validate()?;
        Ok(config)
    }
}

/// Partial `RunnerConfig` — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RunnerLayer {
    /// Wall-clock timeout for the entire plan execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_timeout_secs: Option<u64>,
}

impl RunnerLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            plan_timeout_secs: overlay.plan_timeout_secs.or(self.plan_timeout_secs),
        }
    }

    /// Resolve into a concrete [`RunnerConfig`] value.
    #[must_use]
    pub fn resolve(self) -> RunnerConfig {
        let defaults = RunnerConfig::default();
        RunnerConfig {
            plan_timeout_secs: self.plan_timeout_secs.unwrap_or(defaults.plan_timeout_secs),
            dangerously_skip_permissions: defaults.dangerously_skip_permissions,
        }
    }
}

/// Partial `ServeConfig` — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServeLayer {
    /// Port override for `roko serve`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Shared transcript retention period in days.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_ttl_days: Option<u64>,
    /// Whether to expose the PTY terminal routes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_enabled: Option<bool>,
    /// Whether serve-side publish events trigger orchestration automatically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_orchestrate: Option<bool>,
    /// API auth settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<ServeAuthLayer>,
    /// Cloud deployment settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deploy: Option<ServeDeployLayer>,
    /// Whether `roko` with no subcommand should auto-start the HTTP server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_start: Option<bool>,
}

impl ServeLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            port: overlay.port.or(self.port),
            share_ttl_days: overlay.share_ttl_days.or(self.share_ttl_days),
            terminal_enabled: overlay.terminal_enabled.or(self.terminal_enabled),
            auto_orchestrate: overlay.auto_orchestrate.or(self.auto_orchestrate),
            auth: match (self.auth, overlay.auth) {
                (Some(base), Some(overlay)) => Some(base.merge(overlay)),
                (_, Some(overlay)) => Some(overlay),
                (base, None) => base,
            },
            deploy: match (self.deploy, overlay.deploy) {
                (Some(base), Some(overlay)) => Some(base.merge(overlay)),
                (_, Some(overlay)) => Some(overlay),
                (base, None) => base,
            },
            auto_start: overlay.auto_start.or(self.auto_start),
        }
    }
}

/// Partial API auth settings — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServeAuthLayer {
    /// Whether `/api/*` requires an `X-Api-Key` header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Shared API key expected in `X-Api-Key`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

impl ServeAuthLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            enabled: overlay.enabled.or(self.enabled),
            api_key: overlay.api_key.or(self.api_key),
        }
    }

    /// Resolve into a concrete [`ServeConfig::auth`] value.
    #[must_use]
    pub fn resolve(
        self,
        defaults: roko_core::config::ServeAuthConfig,
    ) -> roko_core::config::ServeAuthConfig {
        roko_core::config::ServeAuthConfig {
            enabled: self.enabled.unwrap_or(defaults.enabled),
            api_key: self.api_key.unwrap_or(defaults.api_key),
            api_keys: defaults.api_keys,
            privy_app_id: defaults.privy_app_id,
            jwks_providers: defaults.jwks_providers,
            privy_workspace_id: defaults.privy_workspace_id,
            privy_allowed_roles: defaults.privy_allowed_roles,
            enforcement_mode: defaults.enforcement_mode,
            invite_expiry_days: defaults.invite_expiry_days,
        }
    }
}

/// Partial cloud deployment settings — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServeDeployLayer {
    /// Deployment provider, e.g. `railway` or `fly`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Environment variables required for deploy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<String>>,
    /// Webhooks to register after deploy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhooks: Option<Vec<ServeDeployWebhookLayer>>,
}

impl ServeDeployLayer {
    /// Merge another layer on top — `overlay` wins.
    #[must_use]
    pub fn merge(self, overlay: Self) -> Self {
        Self {
            provider: overlay.provider.or(self.provider),
            environment: overlay.environment.or(self.environment),
            webhooks: overlay.webhooks.or(self.webhooks),
        }
    }

    /// Resolve into a concrete [`ServeConfig::deploy`] value.
    #[must_use]
    pub fn resolve(self, defaults: ServeDeployConfig) -> ServeDeployConfig {
        ServeDeployConfig {
            provider: self.provider.unwrap_or(defaults.provider),
            environment: self.environment.unwrap_or(defaults.environment),
            webhooks: match self.webhooks {
                Some(webhooks) => webhooks
                    .into_iter()
                    .map(ServeDeployWebhookLayer::resolve)
                    .collect(),
                None => defaults.webhooks,
            },
        }
    }
}

/// Partial webhook registration settings — every field optional.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServeDeployWebhookLayer {
    /// Webhook provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Repository owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Repository name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
}

impl ServeDeployWebhookLayer {
    /// Resolve into a concrete [`ServeDeployWebhookConfig`].
    #[must_use]
    pub fn resolve(self) -> ServeDeployWebhookConfig {
        ServeDeployWebhookConfig {
            provider: self.provider.unwrap_or_else(|| "github".to_string()),
            owner: self.owner.unwrap_or_default(),
            repo: self.repo.unwrap_or_default(),
        }
    }
}

/// Absolute paths to the global and project config files (whether they
/// exist or not).
#[derive(Clone, Debug)]
pub struct ConfigPaths {
    /// Global config path (always set — even if file missing).
    pub global: Option<PathBuf>,
    /// Project config path, if discovered. None means no `roko.toml` in
    /// `workdir` or any ancestor.
    pub project: Option<PathBuf>,
    /// Value of `ROKO_CONFIG` env var if set — overrides the merge.
    pub env_override: Option<PathBuf>,
}

/// Resolve the path to the global config file.
///
/// Canonical path: `~/.roko/config.toml`.
/// Backward compat: if canonical doesn't exist but the legacy
/// `~/.config/roko/config.toml` (or `$XDG_CONFIG_HOME/roko/config.toml`)
/// does, return the legacy path. If neither exists, return the canonical
/// path so that `init` writes to the right place.
#[must_use]
pub fn global_config_path() -> Option<PathBuf> {
    roko_core::config::loader::global_config_path()
}

/// Merge providers and models from the global config into `config`.
///
/// Any provider/model in the global file that is *not* already in `config`
/// gets inserted. This lets project `roko.toml` files override specific
/// entries while inheriting the rest from `~/.roko/config.toml`.
///
/// Returns an error if the global config file exists but cannot be read or
/// parsed. Callers should treat this as fatal before constructing providers.
pub fn merge_global_providers(
    config: &mut roko_core::config::schema::RokoConfig,
) -> Result<(), roko_core::config::LoadConfigError> {
    roko_core::config::loader::merge_global_into(config)
}

/// Walk up from `start` looking for `roko.toml`. Returns the first hit.
#[must_use]
pub fn discover_project_config(start: &Path) -> Option<PathBuf> {
    roko_core::config::loader::discover_project_config(start)
}

/// Compute the paths used to resolve config for `workdir`.
#[must_use]
pub fn resolve_paths(workdir: &Path) -> ConfigPaths {
    ConfigPaths {
        global: global_config_path(),
        project: discover_project_config(workdir),
        env_override: std::env::var_os("ROKO_CONFIG").map(PathBuf::from),
    }
}

/// Fully-loaded config with field-level provenance.
#[derive(Clone, Debug)]
pub struct ResolvedConfig {
    /// Merged, default-filled config.
    pub config: Config,
    /// Loaded runtime repo registry.
    pub repo_registry: RepoRegistry,
    /// Which source supplied each field.
    pub sources: ConfigSources,
    /// Paths consulted during resolution.
    pub paths: ConfigPaths,
}

/// Per-field provenance for [`ResolvedConfig`].
///
/// This is a compatibility facade that downstream rendering uses. It is
/// now populated from the core loader's provenance records rather than
/// from the core loader's provenance records.
#[derive(Clone, Debug)]
pub struct ConfigSources {
    /// Where `auto_plan` came from.
    pub auto_plan: Source,
    /// Where `agent.command` came from.
    pub agent_command: Source,
    /// Where `agent.args` came from.
    pub agent_args: Source,
    /// Where `agent.model` came from.
    pub agent_model: Source,
    /// Where `agent.effort` came from.
    pub agent_effort: Source,
    /// Where `agent.bare_mode` came from.
    pub agent_bare_mode: Source,
    /// Where `agent.fallback_model` came from.
    pub agent_fallback_model: Source,
    /// Where `agent.timeout_ms` came from.
    pub agent_timeout_ms: Source,
    /// Where `budget.prompt_token_budget` came from.
    pub prompt_token_budget: Source,
    /// Where `providers` came from.
    pub providers: Source,
    /// Where `models` came from.
    pub models: Source,
    /// Where `dreams.auto_dream` came from.
    pub dreams_auto_dream: Source,
    /// Where `dreams.idle_threshold_mins` came from.
    pub dreams_idle_threshold_mins: Source,
    /// Where `dreams.min_episodes_for_dream` came from.
    pub dreams_min_episodes_for_dream: Source,
    /// Where `dreams.scheduled_cron` came from.
    pub dreams_scheduled_cron: Source,
    /// Where `dreams.episode_count_trigger` came from.
    pub dreams_episode_count_trigger: Source,
    /// Where `dreams.quality_gain` came from.
    pub dreams_quality_gain: Source,
    /// Where `dreams.quality_penalty` came from.
    pub dreams_quality_penalty: Source,
    /// Where `gates` came from.
    pub gates: Source,
    /// Where `runner.plan_timeout_secs` came from.
    pub runner_plan_timeout_secs: Source,
}

impl ConfigSources {
    /// Build provenance from the core loader's provenance records.
    ///
    /// Translates `ConfigSource` from the core provenance system into the
    /// CLI `Source` enum by checking which keys have non-default provenance
    /// entries.
    pub fn from_core_provenance(validated: &roko_core::config::ValidatedConfig) -> Self {
        use roko_core::config::ConfigSource as CS;

        let global_path = global_config_path();

        let lookup = |key: &str| -> Source {
            // Check merge_context field provenance first (most specific).
            // Skip for CS::File since FieldProvenance has no path field and
            // we cannot distinguish global vs project config.
            if let Some(fp) = validated
                .merge_context
                .field_provenance
                .iter()
                .find(|fp| fp.key == key)
            {
                match &fp.value_source {
                    CS::Env => return Source::Env,
                    CS::CliOverride | CS::ApiOverride => return Source::Env,
                    CS::Migration | CS::Evolved | CS::Composed | CS::Default => {
                        return Source::Default;
                    }
                    // For File/LocalOverride: fall through to provenance entries
                    // which carry path info for global vs project distinction.
                    CS::File | CS::LocalOverride => {}
                }
            }
            // Fall back to provenance entries which have path information.
            for entry in &validated.provenance {
                if entry.key == key {
                    return match &entry.source {
                        CS::Env => Source::Env,
                        CS::File | CS::LocalOverride => {
                            // Distinguish global config from project config by
                            // checking if the provenance path matches the known
                            // global config file location.
                            if let Some(ref prov_path) = entry.path {
                                if global_path.as_deref() == Some(prov_path.as_path()) {
                                    return Source::Global;
                                }
                            }
                            Source::Project
                        }
                        CS::CliOverride | CS::ApiOverride => Source::Env,
                        CS::Migration | CS::Evolved | CS::Composed | CS::Default => Source::Default,
                    };
                }
            }
            // If merge_context had File but no provenance entry matched,
            // conservatively report Project.
            if validated
                .merge_context
                .field_provenance
                .iter()
                .any(|fp| fp.key == key && matches!(fp.value_source, CS::File | CS::LocalOverride))
            {
                return Source::Project;
            }
            Source::Default
        };

        Self {
            auto_plan: lookup("prd.auto_plan"),
            agent_command: lookup("agent.command"),
            agent_args: lookup("agent.args"),
            agent_model: lookup("agent.default_model"),
            agent_effort: lookup("agent.default_effort"),
            agent_bare_mode: lookup("agent.bare_mode"),
            agent_fallback_model: lookup("agent.fallback_model"),
            agent_timeout_ms: lookup("agent.timeout_ms"),
            prompt_token_budget: lookup("budget.prompt_token_budget"),
            providers: lookup("providers"),
            models: lookup("models"),
            dreams_auto_dream: lookup("dreams.auto_dream"),
            dreams_idle_threshold_mins: lookup("dreams.idle_threshold_mins"),
            dreams_min_episodes_for_dream: lookup("dreams.min_episodes_for_dream"),
            dreams_scheduled_cron: lookup("dreams.scheduled_cron"),
            dreams_episode_count_trigger: lookup("dreams.episode_count_trigger"),
            dreams_quality_gain: lookup("dreams.quality_gain"),
            dreams_quality_penalty: lookup("dreams.quality_penalty"),
            gates: lookup("gates"),
            runner_plan_timeout_secs: lookup("runner.plan_timeout_secs"),
        }
    }
}

/// Load config using the unified core loader and return a [`ResolvedConfig`].
///
/// This is the primary config loading entry point for CLI code. It delegates
/// entirely to `roko_core::config::loader::load_config_validated_with_options()`
/// which handles: ancestor walk, `ROKO_CONFIG` env, global merge, named env
/// overrides (`ROKO_MODEL` etc.), hierarchical `ROKO__*` overrides,
/// interpolation, and file secret resolution.
///
/// The validated core `RokoConfig` is then converted to the CLI `Config` via
/// [`Config::from_roko_config()`], and provenance is derived from the core
/// loader's provenance records.
///
/// Precedence (highest first): hierarchical `ROKO__*` env vars -> named
/// `ROKO_*` env vars -> `ROKO_CONFIG` env var -> project `roko.toml` ->
/// global `~/.roko/config.toml` -> defaults.
pub fn load_resolved_config(workdir: &Path) -> Result<ResolvedConfig> {
    let paths = resolve_paths(workdir);

    let core_validated = roko_core::config::loader::load_config_validated_with_options(
        workdir,
        &roko_core::config::loader::LoadOptions::default(),
    )
    .map_err(|e| anyhow!("core config loader: {e}"))?;

    // Surface core validation diagnostics as warnings.
    for diagnostic in &core_validated.diagnostics {
        tracing::warn!(
            key = %diagnostic.key,
            message = %diagnostic.message,
            "roko config validation diagnostic"
        );
    }

    let config = Config::from_roko_config(core_validated.config())?;
    let sources = ConfigSources::from_core_provenance(&core_validated);
    let repo_registry = RepoRegistry::load(&config, workdir)?;

    Ok(ResolvedConfig {
        config,
        repo_registry,
        sources,
        paths,
    })
}

// -----------------------------------------------------------------------
// LLM CLI detection (for the `roko config init` wizard)
// -----------------------------------------------------------------------

/// A locally-installed LLM CLI that the wizard can offer as an agent backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedCli {
    /// The command name (`ollama`, `mods`, `llm`, `claude`, `aichat`).
    pub command: String,
    /// Default args the wizard will suggest (e.g. `["run", "<model>"]` for
    /// ollama — filled in from the model picker).
    pub default_args: Vec<String>,
    /// Human-readable description for the wizard prompt.
    pub description: String,
}

/// Candidates the wizard asks about, in order of preference.
#[must_use]
pub fn candidate_clis() -> Vec<DetectedCli> {
    vec![
        DetectedCli {
            command: "claude".into(),
            default_args: vec![],
            description: "Claude CLI (anthropic)".into(),
        },
        DetectedCli {
            command: "ollama".into(),
            default_args: vec!["run".into()],
            description: "Ollama (local models)".into(),
        },
        DetectedCli {
            command: "mods".into(),
            default_args: vec![],
            description: "charmbracelet/mods".into(),
        },
        DetectedCli {
            command: "llm".into(),
            default_args: vec![],
            description: "simonw/llm".into(),
        },
        DetectedCli {
            command: "aichat".into(),
            default_args: vec![],
            description: "aichat CLI".into(),
        },
        DetectedCli {
            command: "cat".into(),
            default_args: vec![],
            description: "cat (echo; smoke tests only)".into(),
        },
    ]
}

/// Return the subset of [`candidate_clis`] actually on the user's `PATH`.
#[must_use]
pub fn detect_clis() -> Vec<DetectedCli> {
    candidate_clis()
        .into_iter()
        .filter(|c| command_on_path(&c.command))
        .collect()
}

/// Cheap `which` — scan `$PATH` for an executable named `cmd`.
#[must_use]
pub fn command_on_path(cmd: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if let Ok(meta) = std::fs::metadata(&candidate) {
            if meta.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if meta.permissions().mode() & 0o111 != 0 {
                        return true;
                    }
                }
                #[cfg(not(unix))]
                {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::{InitProvider, render_init_template_for, write_init_config};

    /// Derive the set of known top-level TOML keys from serializing a default
    /// `RokoConfig`.  This stays in sync automatically as fields are added.
    fn known_config_keys() -> std::collections::HashSet<String> {
        let default_toml = toml::to_string(&RokoConfig::default()).expect("serialize default");
        let value: toml::Value = toml::from_str(&default_toml).expect("parse default");
        value
            .as_table()
            .expect("default is a table")
            .keys()
            .cloned()
            .collect()
    }

    #[test]
    fn parses_minimal_config() {
        let toml = r#"
[agent]
"#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert_eq!(cfg.agent.command, "cat");
        assert_eq!(
            cfg.agent.timeout_ms,
            roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS
        );
        assert_eq!(cfg.prompt.token_budget, 10_000);
        assert_eq!(cfg.prompt.role, "implementer");
        assert_eq!(cfg.runner.plan_timeout_secs, DEFAULT_PLAN_TIMEOUT_SECS);
        assert!(cfg.repos.is_empty());
    }

    #[test]
    fn parses_full_config() {
        let toml = r#"
[agent]
command = "ollama"
args = ["run", "llama3"]
timeout_ms = 30000

[budget]
warn_at_percent = 90

[dreams]
auto_dream = false
idle_threshold_mins = 30
min_episodes_for_dream = 8
scheduled_cron = "0 0 */4 * * * *"
episode_count_trigger = 12
quality_gain = 0.8
quality_penalty = 1.4

[[repos]]
name = "roko"
path = "/Users/will/dev/nunchi/roko/roko"
branch = "main"
templates = ["pr-reviewer", "test-writer", "ci-fixer"]

[[repos.subscriptions]]
template = "code-implementer"
trigger = "github:issues:labeled:implement"

[[gate]]
kind = "shell"
program = "echo"
args = ["ok"]

[[gate]]
kind = "compile"
build_system = "cargo"
"#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert_eq!(cfg.agent.command, "ollama");
        assert_eq!(
            cfg.agent.args,
            vec!["run".to_string(), "llama3".to_string()]
        );
        assert_eq!(cfg.agent.timeout_ms, 30_000);
        assert_eq!(cfg.budget.warn_at_percent, 90);
        assert!(!cfg.dreams.auto_dream);
        assert_eq!(cfg.dreams.idle_threshold_mins, 30);
        assert_eq!(cfg.dreams.min_episodes_for_dream, 8);
        assert_eq!(
            cfg.dreams.scheduled_cron.as_deref(),
            Some("0 0 */4 * * * *")
        );
        assert_eq!(cfg.dreams.episode_count_trigger, 12);
        assert_eq!(cfg.dreams.quality_gain, 0.8);
        assert_eq!(cfg.dreams.quality_penalty, 1.4);
        assert_eq!(cfg.repos.len(), 1);
        assert_eq!(cfg.repos[0].name, "roko");
        assert_eq!(
            cfg.repos[0].templates,
            vec![
                "pr-reviewer".to_string(),
                "test-writer".to_string(),
                "ci-fixer".to_string()
            ]
        );
        assert_eq!(cfg.repos[0].subscriptions.len(), 1);
        assert_eq!(cfg.repos[0].subscriptions[0].template, "code-implementer");
        assert_eq!(
            cfg.repos[0].subscriptions[0].trigger,
            "github:issues:labeled:implement"
        );
        assert_eq!(cfg.gates.len(), 2);
    }

    #[test]
    fn parse_rejects_invalid_dream_schedule() {
        let err = Config::parse_toml(
            r#"
[agent]
command = "cat"

[dreams]
scheduled_cron = "not a cron expression"
"#,
        )
        .unwrap_err();

        assert!(format!("{err:#}").contains("invalid dream schedule cron expression"));
    }

    /// gap-666ab3, bug-d5051e: `[executor]` and the v1 `[tools]` and
    /// `[prompt]` keys were removed. An old file that has them still parses:
    /// they are dropped with a warning, and the run-time prompt settings keep
    /// their defaults.
    #[test]
    fn old_executor_section_still_parses() {
        let toml = r#"
[agent]
command = "cat"

[tools]
prefer_mcp = false
global_denied = ["bash"]
mcp_timeout_secs = 15

[prompt]
token_budget = 20000
role = "You are a senior Rust engineer."

[executor]
max_concurrent_plans = 8
max_concurrent_tasks = 12
use_worktrees = true
"#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert_eq!(cfg.agent.command, "cat");
        assert_eq!(cfg.prompt.token_budget, 10_000);
        assert_eq!(cfg.prompt.role, "implementer");
        assert_eq!(cfg.runner.plan_timeout_secs, DEFAULT_PLAN_TIMEOUT_SECS);
    }

    #[test]
    fn parses_serve_auth_section_from_toml() {
        let toml = r#"
[agent]
command = "cat"

[serve.auth]
enabled = true
api_key = "secret"
        "#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert!(!cfg.serve.terminal_enabled);
        assert!(cfg.serve.auth.enabled);
        assert_eq!(cfg.serve.auth.api_key, "secret");
    }

    #[test]
    fn parses_serve_auto_start_section_from_toml() {
        let toml = r#"
[agent]
command = "cat"

[serve]
auto_start = true
        "#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert!(cfg.serve.auto_start);
    }

    #[test]
    fn parses_serve_deploy_section_from_toml() {
        let toml = r#"
[agent]
command = "cat"

[serve.deploy]
provider = "fly"
environment = ["GITHUB_TOKEN", "SLACK_BOT_TOKEN"]

[[serve.deploy.webhooks]]
provider = "github"
owner = "nunchi"
repo = "roko"
"#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert_eq!(cfg.serve.deploy.provider, "fly");
        assert_eq!(
            cfg.serve.deploy.environment,
            vec!["GITHUB_TOKEN".to_string(), "SLACK_BOT_TOKEN".to_string()]
        );
        assert_eq!(cfg.serve.deploy.webhooks.len(), 1);
        assert_eq!(cfg.serve.deploy.webhooks[0].provider, "github");
        assert_eq!(cfg.serve.deploy.webhooks[0].owner, "nunchi");
        assert_eq!(cfg.serve.deploy.webhooks[0].repo, "roko");
    }

    #[test]
    fn interpolates_env_vars_in_string_values() {
        let path = std::env::var("PATH").expect("PATH must be set for tests");
        let toml = r#"
[agent]
command = "${PATH}"
args = ["--token=${PATH}"]
model = "prefix-${PATH}-suffix"
"#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert_eq!(cfg.agent.command, path);
        assert_eq!(cfg.agent.args, vec![format!("--token={path}")]);
        assert_eq!(cfg.agent.model, Some(format!("prefix-{path}-suffix")));
    }

    #[test]
    fn missing_env_var_resolves_to_empty_string() {
        let toml = r#"
[agent]
command = "prefix-${ROKO_TEST_MISSING_SECRET_9B1C}-suffix"
"#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert_eq!(cfg.agent.command, "prefix--suffix");
    }

    #[test]
    fn env_var_with_default_uses_fallback() {
        let toml = r#"
[agent]
command = "${ROKO_TEST_MISSING_ABC123:-fallback_cmd}"
"#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert_eq!(cfg.agent.command, "fallback_cmd");
    }

    #[test]
    fn env_var_with_empty_default() {
        let toml = r#"
[agent]
command = "x${ROKO_TEST_MISSING_DEF456:-}y"
"#;
        let cfg = Config::parse_toml(toml).unwrap();
        assert_eq!(cfg.agent.command, "xy");
    }

    #[test]
    fn budget_warn_threshold_defaults_to_eighty_percent() {
        let budget = BudgetConfig {
            max_plan_usd: 10.0,
            ..BudgetConfig::default()
        };
        assert_eq!(budget.warn_at_percent, 80);
        assert!((budget.warn_threshold_usd() - 8.0).abs() < f64::EPSILON);
    }

    /// bug-367f33: `roko --config <file>` parses the file into this legacy
    /// `Config`, whose own defaults ($10 plan, $1 task and turn) filled the
    /// keys a `[budget]` table left out, while the same file loaded as the
    /// workspace roko.toml got core's (0.0, no cap).
    #[test]
    fn config_flag_budget_defaults_match_core() {
        let options = roko_core::config::loader::LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };
        for text in [
            "[agent]\n\n[budget]\n",
            "[agent]\n\n[budget]\nmax_plan_usd = 5.0\n",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("roko.toml");
            std::fs::write(&path, text).unwrap();

            // What `roko --config <file>` loads.
            let flag = Config::from_file(&path).unwrap().budget;
            // What the workspace loader makes of the same file.
            let core = roko_core::config::loader::load_config_file(&path, &options).unwrap();
            let workspace = Config::from_roko_config(&core).unwrap().budget;

            assert_eq!(flag.max_plan_usd, workspace.max_plan_usd, "{text:?}");
            assert_eq!(flag.max_task_usd, workspace.max_task_usd, "{text:?}");
            assert_eq!(flag.max_turn_usd, workspace.max_turn_usd, "{text:?}");
            assert_eq!(flag.max_task_usd, 0.0, "a missing cap means no cap");
        }
    }

    /// bug-9bb0be: `--config <path>` treats a typo inside a provider or model
    /// entry as the workspace loader does: the key is dropped with a warning
    /// and the rest of the entry loads.
    #[test]
    fn config_from_file_treats_a_provider_typo_like_the_loader() {
        let text = r#"[agent]

[providers.local]
kind = "openai_compat"
base_ulr = "http://localhost:11434/v1"
api_key_env = "LOCAL_KEY"

[models.local-model]
provider = "local"
slug = "llama3"
contxt_window = 8192
"#;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("roko.toml");
        std::fs::write(&path, text).expect("write config");
        let options = roko_core::config::loader::LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };

        // What `roko --config <file>` loads, and what the workspace loader
        // makes of the same file.
        let flag = Config::from_file(&path).expect("--config drops the typos");
        let core = roko_core::config::loader::load_config_file(&path, &options)
            .expect("the loader drops them");
        let workspace = Config::from_roko_config(&core).expect("convert the loaded config");

        for config in [&flag, &workspace] {
            let provider = &config.providers["local"];
            assert_eq!(provider.api_key_env.as_deref(), Some("LOCAL_KEY"));
            assert_eq!(provider.base_url, None);
            assert_eq!(config.models["local-model"].slug, "llama3");
        }
    }

    /// L13: a config without `[[gate]]` entries has the default's legacy
    /// gates, which are none. The default used to hold a `shell true`
    /// placeholder that never ran but was counted by `roko do`.
    #[test]
    fn config_without_gate_entries_has_the_default_gates() {
        let partial = Config::parse_toml("[agent]\n").expect("parse config");
        assert_eq!(partial.gates.len(), Config::default().gates.len());
        assert!(partial.gates.is_empty());
    }

    #[test]
    fn default_config_roundtrips_through_toml() {
        let cfg = Config::default();
        let text = cfg.to_toml().unwrap();
        let parsed = Config::parse_toml(&text).unwrap();
        assert_eq!(parsed.agent.command, cfg.agent.command);
        assert_eq!(parsed.auto_plan, cfg.auto_plan);
        assert_eq!(parsed.dreams.auto_dream, cfg.dreams.auto_dream);
        assert_eq!(
            parsed.dreams.idle_threshold_mins,
            cfg.dreams.idle_threshold_mins
        );
        assert_eq!(
            parsed.dreams.min_episodes_for_dream,
            cfg.dreams.min_episodes_for_dream
        );
        assert_eq!(parsed.providers, cfg.providers);
        assert_eq!(parsed.models, cfg.models);
        assert_eq!(parsed.repos.len(), cfg.repos.len());
        assert_eq!(parsed.gates.len(), cfg.gates.len());
        assert_eq!(parsed.serve.terminal_enabled, cfg.serve.terminal_enabled);
        assert_eq!(parsed.serve.auto_start, cfg.serve.auto_start);
        assert_eq!(parsed.serve.auth.enabled, cfg.serve.auth.enabled);
        assert_eq!(parsed.serve.auth.api_key, cfg.serve.auth.api_key);
        assert_eq!(parsed.serve.deploy.provider, cfg.serve.deploy.provider);
        assert_eq!(
            parsed.serve.deploy.environment,
            cfg.serve.deploy.environment
        );
        assert_eq!(parsed.serve.deploy.webhooks, cfg.serve.deploy.webhooks);
        assert_eq!(
            parsed.runner.plan_timeout_secs,
            cfg.runner.plan_timeout_secs
        );
    }

    #[test]
    fn repo_registry_loads_repo_local_config() {
        use std::fs;

        let tmp = tempfile::tempdir().unwrap();
        let repo_root = tmp.path().join("repo-a");
        fs::create_dir_all(repo_root.join(".roko")).unwrap();
        fs::write(
            repo_root.join(".roko").join("roko.toml"),
            "schema_version = 2\n",
        )
        .unwrap();

        let mut cfg = Config::default();
        cfg.repos = vec![RepoConfig {
            name: "repo-a".to_string(),
            path: PathBuf::from("repo-a"),
            branch: "main".to_string(),
            templates: Vec::new(),
            subscriptions: Vec::new(),
        }];

        let registry = RepoRegistry::load(&cfg, tmp.path()).unwrap();
        assert_eq!(registry.repos().len(), 1);
        let repo = registry.get("repo-a").unwrap();
        assert!(repo.root.ends_with("repo-a"));
        assert!(repo.roko_config.is_some());
        assert!(
            repo.roko_config_path
                .as_ref()
                .is_some_and(|path| path.ends_with(".roko/roko.toml"))
        );
    }

    #[test]
    fn repo_registry_errors_when_repo_path_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = Config::default();
        cfg.repos = vec![RepoConfig {
            name: "missing".to_string(),
            path: PathBuf::from("missing"),
            branch: "main".to_string(),
            templates: Vec::new(),
            subscriptions: Vec::new(),
        }];

        let err = RepoRegistry::load(&cfg, tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("missing"));
        assert!(msg.contains("resolve repo 'missing' path"));
    }

    #[test]
    fn set_toml_dotted_key_sets_fields_correctly() {
        let mut doc = toml::Value::Table(toml::map::Map::new());
        set_toml_dotted_key(&mut doc, "providers.zai.kind", "openai_compat").unwrap();
        set_toml_dotted_key(
            &mut doc,
            "providers.zai.base_url",
            "https://api.z.ai/api/paas/v4",
        )
        .unwrap();
        set_toml_dotted_key(&mut doc, "models.glm51.provider", "zai").unwrap();
        set_toml_dotted_key(&mut doc, "models.glm51.slug", "glm-5.1").unwrap();
        set_toml_dotted_key(&mut doc, "models.glm51.supports_thinking", "true").unwrap();
        set_toml_dotted_key(&mut doc, "runner.plan_timeout_secs", "1800").unwrap();
        set_toml_dotted_key(&mut doc, "learning.gate_threshold_flush_interval", "7").unwrap();

        assert_eq!(
            doc["providers"]["zai"]["kind"].as_str().unwrap(),
            "openai_compat"
        );
        assert_eq!(
            doc["providers"]["zai"]["base_url"].as_str().unwrap(),
            "https://api.z.ai/api/paas/v4"
        );
        assert_eq!(doc["models"]["glm51"]["provider"].as_str().unwrap(), "zai");
        assert_eq!(doc["models"]["glm51"]["slug"].as_str().unwrap(), "glm-5.1");
        assert!(
            doc["models"]["glm51"]["supports_thinking"]
                .as_bool()
                .unwrap()
        );
        assert_eq!(
            doc["runner"]["plan_timeout_secs"].as_integer().unwrap(),
            1800
        );
        assert_eq!(
            doc["learning"]["gate_threshold_flush_interval"]
                .as_integer()
                .unwrap(),
            7
        );
    }

    #[test]
    fn set_toml_dotted_key_sets_the_planner_model() {
        let mut doc = toml::Value::Table(toml::map::Map::new());
        set_toml_dotted_key(&mut doc, "authoring.planner_model", "claude-opus-4-6").unwrap();

        assert_eq!(
            doc["authoring"]["planner_model"].as_str().unwrap(),
            "claude-opus-4-6"
        );
    }

    /// bug-9434c4: the opt-in learning flags are not in the hand-written key
    /// list, so `config set` types them from the schema tree.
    #[test]
    fn config_set_accepts_learning_opt_in_keys() {
        assert_eq!(
            parse_value_for_key("learning.t0_reflexes", "true").unwrap(),
            toml::Value::Boolean(true)
        );
        assert_eq!(
            parse_value_for_key("learning.dreams.trigger_on_acp_episodes", "true").unwrap(),
            toml::Value::Boolean(true)
        );

        let mut doc = toml::Value::Table(toml::map::Map::new());
        for (key, value) in [
            ("learning.t0_reflexes", "true"),
            ("learning.dreams.trigger_on_plan_complete", "false"),
            ("learning.dreams.trigger_on_acp_episodes", "true"),
            ("learning.dreams.acp_episode_threshold", "4"),
            ("learning.dreams.max_concurrent", "2"),
        ] {
            set_toml_dotted_key(&mut doc, key, value).unwrap();
        }
        let unknown = roko_core::config::loader::validate_known_config_paths(&doc);
        assert!(unknown.is_empty(), "unknown keys: {unknown:?}");
        let config: RokoConfig = doc.try_into().expect("the edited config loads");
        assert!(config.learning.t0_reflexes);
        let dreams = &config.learning.dreams;
        assert!(!dreams.trigger_on_plan_complete);
        assert!(dreams.trigger_on_acp_episodes);
        assert_eq!(dreams.acp_episode_threshold, 4);
        assert_eq!(dreams.max_concurrent, 2);
    }

    /// bug-d5051e: `config set` refuses a v1 key the schema dropped, says why,
    /// and writes nothing; the keys that replaced two of them are settable.
    #[test]
    fn config_set_rejects_v1_keys() {
        for key in [
            "tools.prefer_mcp",
            "tools.global_denied",
            "tools.mcp_timeout_secs",
            "prompt.token_budget",
            "prompt.role",
            "prompt.files",
            "executor.use_worktrees",
        ] {
            let mut doc = toml::Value::Table(toml::map::Map::new());
            let err = set_toml_dotted_key(&mut doc, key, "1").unwrap_err();
            assert!(err.to_string().contains("was removed"), "{key}: {err}");
            assert_eq!(doc, toml::Value::Table(toml::map::Map::new()), "{key}");
        }

        let mut doc = toml::Value::Table(toml::map::Map::new());
        set_toml_dotted_key(&mut doc, "tools.deny", "bash").unwrap();
        set_toml_dotted_key(&mut doc, "budget.prompt_token_budget", "4000").unwrap();
        assert_eq!(doc["tools"]["deny"].as_array().map(Vec::len), Some(1));
        let budget = doc["budget"]["prompt_token_budget"].as_integer();
        assert_eq!(budget, Some(4000));
    }

    #[test]
    fn global_path_ends_in_roko_config_toml() {
        if let Some(path) = global_config_path() {
            assert!(
                path.ends_with(".roko/config.toml") || path.ends_with("roko/config.toml"),
                "expected path ending in .roko/config.toml or roko/config.toml, got: {path:?}"
            );
        }
    }

    #[test]
    fn discover_project_config_walks_up() {
        use std::fs;
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();
        fs::write(tmp.path().join("roko.toml"), "").unwrap();
        let found = discover_project_config(&nested).unwrap();
        assert_eq!(
            found.canonicalize().unwrap(),
            tmp.path().join("roko.toml").canonicalize().unwrap()
        );
    }

    #[test]
    fn detect_clis_does_not_panic() {
        let _ = detect_clis();
    }

    #[test]
    fn default_toml_template_includes_required_env_section() {
        let rendered = render_init_template_for(false, InitProvider::ClaudeCli).unwrap();
        assert!(rendered.contains("# REQUIRED_ENV"));
        assert!(rendered.contains("GITHUB_TOKEN"));
        assert!(rendered.contains("GITHUB_WEBHOOK_SECRET"));
        assert!(rendered.contains("SLACK_BOT_TOKEN"));
        assert!(rendered.contains("SLACK_SIGNING_SECRET"));
        assert!(rendered.contains("ANTHROPIC_API_KEY"));
        assert!(rendered.contains("config_version = 2"));
        assert!(rendered.contains("schema_version = 2"));
        assert!(rendered.contains("default_backend = \"claude\""));
        assert!(rendered.contains("default_model = \"claude-sonnet-4-6\""));
        assert!(
            rendered.contains("[providers.claude_cli]")
                || rendered.contains("# [providers.claude_cli]")
        );
        assert!(rendered.contains("kind = \"claude_cli\""));
        assert!(rendered.contains("command = \"claude\""));
        assert!(rendered.contains("[models.claude-sonnet-4-6]"));
        assert!(rendered.contains("provider = \"claude_cli\""));
        assert!(rendered.contains("slug = \"claude-sonnet-4-6\""));
        assert!(rendered.contains("context_window = 200000"));
        assert!(rendered.contains("tool_format = \"anthropic_blocks\""));
        assert!(rendered.contains("max_tools = 32"));
        assert!(rendered.contains("[prd]"));
        assert!(rendered.contains("auto_plan = false"));
        assert!(rendered.contains("auto_start = false"));
        assert!(rendered.contains("[learning]"));
        assert!(rendered.contains("gate_threshold_flush_interval = 10"));
    }

    #[test]
    fn init_template_model_overrides_same_slug_from_global_config() {
        let rendered = render_init_template_for(false, InitProvider::ClaudeCli).unwrap();
        let mut project = RokoConfig::from_toml(&rendered).expect("parse init template");
        let rendered_value: toml::Value = toml::from_str(&rendered).expect("parse template TOML");
        let unknown_keys = rendered_value
            .as_table()
            .expect("init template is a TOML table")
            .keys()
            .filter(|key| !known_config_keys().contains(key.as_str()))
            .collect::<Vec<_>>();
        assert!(
            unknown_keys.is_empty(),
            "fresh init template contains unrecognized top-level sections: {unknown_keys:?}"
        );
        let global = RokoConfig::from_toml(
            r#"
[providers.claude_cli]
kind = "claude_cli"
command = "claude"

[models.claude-sonnet]
provider = "claude_cli"
slug = "claude-sonnet-4-6"
context_window = 100000

[agent]
default_model = "claude-sonnet"
"#,
        )
        .expect("parse global config fixture");

        roko_core::config::loader::merge_global_config_into(&mut project, global);
        roko_core::config::loader::normalize_and_validate_dispatch_models(&mut project)
            .expect("fresh init config must remain dispatchable after global merge");

        assert_eq!(project.agent.default_model, "claude-sonnet-4-6");
        assert!(project.models.contains_key("claude-sonnet-4-6"));
        assert!(
            !project.models.contains_key("claude-sonnet"),
            "global alias with the init template's slug should be shadowed"
        );
    }

    /// bug-e1327f: without `claude` on PATH, `roko init` used to leave the
    /// default model pointing at the commented-out `claude_cli` provider, so
    /// every command failed with config invariant 3.
    #[test]
    fn init_without_claude_cli_writes_a_loadable_config() {
        // With ANTHROPIC_API_KEY set the model uses the Anthropic API;
        // without it, the model block is commented out with its provider.
        for (provider, model_provider) in [
            (InitProvider::AnthropicApi, Some("anthropic")),
            (InitProvider::Unconfigured, None),
        ] {
            let dir = tempfile::tempdir().expect("tempdir");
            write_init_config(dir.path(), false, provider)
                .unwrap_or_else(|err| panic!("{provider:?}: init refused its template: {err:#}"));

            let loaded = roko_core::config::loader::load_config_file(
                &dir.path().join("roko.toml"),
                &roko_core::config::loader::LoadOptions {
                    merge_global: false,
                    apply_env_overrides: false,
                    apply_hierarchical_env: false,
                    strict_validation: false,
                },
            )
            .unwrap_or_else(|err| panic!("{provider:?}: the loader rejected it: {err}"));
            assert_eq!(
                loaded
                    .models
                    .get("claude-sonnet-4-6")
                    .map(|model| model.provider.as_str()),
                model_provider,
                "{provider:?}"
            );
            for model in loaded.models.values() {
                assert!(
                    loaded.providers.contains_key(&model.provider),
                    "{provider:?}: model provider '{}' is not configured",
                    model.provider
                );
            }
        }
    }

    #[test]
    fn cloud_default_toml_template_includes_cloud_settings() {
        let rendered = Config::default_toml_template(true).unwrap();
        assert!(rendered.contains("schema_version = 2"));
        assert!(rendered.contains(r#"bind = "0.0.0.0""#));
        assert!(rendered.contains(r#"provider = "railway""#));
        assert!(rendered.contains("[models.claude-sonnet-4-6]"));
        assert!(rendered.contains("GITHUB_WEBHOOK_SECRET"));
        assert!(rendered.contains("Auto-register webhooks after deploy"));
        assert!(rendered.contains("[[serve.deploy.webhooks]]"));
    }

    #[test]
    fn default_toml_template_enables_auth_by_default() {
        let rendered = Config::default_toml_template(false).unwrap();
        assert!(
            rendered.contains("[serve.auth]"),
            "[serve.auth] table missing"
        );
        // The v2 template uses RokoConfig::default() which has auth enabled
        // (secure-by-default). Local users opt out in their roko.toml.
        assert!(
            rendered.contains("enabled = true"),
            "expected the rendered template to set serve.auth.enabled = true (secure-by-default)"
        );
    }

    #[test]
    fn load_resolved_config_has_no_project_sources_without_project_config() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = load_resolved_config(dir.path()).unwrap();
        // Without a project-level roko.toml, paths.project must be None.
        assert_eq!(resolved.paths.project, None);
        // On developer machines with ~/.roko/config.toml, keys like providers
        // may resolve to Global (from the global file) or Default. The key
        // invariant is: no project config exists, so project must be None.
        // We do NOT assert about individual field sources here because the
        // core provenance system cannot always distinguish global-file from
        // project-file origins (both use CS::File).
    }

    #[test]
    fn load_resolved_config_reads_project_toml() {
        let dir = tempfile::tempdir().unwrap();
        let toml = r#"
[agent]
command = "claude"
model = "opus-4"
"#;
        std::fs::write(dir.path().join("roko.toml"), toml).unwrap();

        let resolved = load_resolved_config(dir.path()).unwrap();
        // The project TOML sets command="claude" and model="opus-4", but env
        // vars, shell aliases, or a global ~/.roko/config.toml may override
        // the final resolved values. The key invariant: the project file was
        // discovered and its values contributed to resolution.
        assert!(
            resolved.paths.project.is_some(),
            "project roko.toml must be discovered"
        );
    }

    #[test]
    fn load_resolved_config_repo_registry_loads_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("roko.toml"), "[agent]\ncommand = \"cat\"\n").unwrap();

        let resolved = load_resolved_config(dir.path()).unwrap();
        // No repos configured, registry should be empty.
        assert!(resolved.repo_registry.is_empty());
    }

    #[test]
    fn load_resolved_config_config_paths_populated() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("roko.toml"), "[agent]\ncommand = \"cat\"\n").unwrap();

        let resolved = load_resolved_config(dir.path()).unwrap();
        // Global path is set when HOME is available.
        if let Some(ref global) = resolved.paths.global {
            assert!(!global.as_os_str().is_empty());
        }
        // Project path should point to the roko.toml we wrote.
        assert!(resolved.paths.project.is_some());
    }

    /// Unknown top-level keys in roko.toml produce a warning via
    /// the core loader's `validate_known_config_paths` instead of being silently swallowed.
    #[test]
    fn config_unknown_keys_warn() {
        // TOML with both known and unknown top-level keys.
        let toml_text =
            "typo_key = 42\n\n[agent]\ncommand = \"cat\"\n\n[bogus_section]\nkey = \"value\"\n";

        // Verify known keys pass the filter.
        let known_only =
            "[agent]\ncommand = \"cat\"\n\n[providers.test]\nkind = \"openai_compat\"\n";
        let value: toml::Value = toml::from_str(known_only).unwrap();
        let table = value.as_table().unwrap();
        let unknown_known: Vec<&String> = table
            .keys()
            .filter(|k| !known_config_keys().contains(k.as_str()))
            .collect();
        assert!(
            unknown_known.is_empty(),
            "expected no unknown keys in known-only TOML, got: {unknown_known:?}"
        );

        // Verify unknown keys ARE detected.
        let value: toml::Value = toml::from_str(toml_text).unwrap();
        let table = value.as_table().unwrap();
        let unknown: Vec<&String> = table
            .keys()
            .filter(|k| !known_config_keys().contains(k.as_str()))
            .collect();
        assert_eq!(
            unknown.len(),
            2,
            "expected 2 unknown keys (bogus_section, typo_key), got: {unknown:?}"
        );
        assert!(unknown.iter().any(|k| k.as_str() == "bogus_section"));
        assert!(unknown.iter().any(|k| k.as_str() == "typo_key"));
    }

    /// Core-validated providers/models are consumed by load_resolved_config
    /// (not silently discarded).
    #[test]
    fn load_resolved_config_consumes_core_providers_and_models() {
        let dir = tempfile::tempdir().unwrap();
        let toml = "[agent]\ncommand = \"cat\"\n\n[providers.test_provider]\nkind = \"openai_compat\"\nbase_url = \"https://test.example.com\"\napi_key_env = \"TEST_KEY\"\n\n[models.test_model]\nprovider = \"test_provider\"\nslug = \"test-v1\"\ncontext_window = 64000\n";
        std::fs::write(dir.path().join("roko.toml"), toml).unwrap();

        let resolved = load_resolved_config(dir.path()).unwrap();
        assert!(
            resolved.config.providers.contains_key("test_provider"),
            "core-validated provider must be present in resolved config"
        );
        assert!(
            resolved.config.models.contains_key("test_model"),
            "core-validated model must be present in resolved config"
        );
        let provider = resolved.config.providers.get("test_provider").unwrap();
        assert_eq!(
            provider.base_url.as_deref(),
            Some("https://test.example.com")
        );
        let model = resolved.config.models.get("test_model").unwrap();
        assert_eq!(model.provider, "test_provider");
        assert_eq!(model.slug, "test-v1");
        assert_eq!(model.context_window, 64000);
    }
}
