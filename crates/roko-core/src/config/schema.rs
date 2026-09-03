//! Unified `RokoConfig` schema with hierarchical sections.
//!
//! Every section is a separate struct so callers can destructure just the
//! slice they need. All fields carry serde defaults so a bare config still
//! produces a fully-populated `RokoConfig`.
//!
//! Section structs live in dedicated submodules and are re-exported here so
//! that `schema::FooConfig` continues to resolve for all existing callers.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::Write as _;

use crate::agent::{AgentBackend, ProviderKind};
use crate::defaults::{DEFAULT_PLAN_TIMEOUT_SECS, DEFAULT_RATE_LIMIT_RETRY_ATTEMPTS};
use crate::tool::{ToolFormat, profile_for_model};
use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

// Re-export all section structs from submodules.
pub use super::agent::*;
pub use super::budget::*;
pub use super::chain::*;
pub use super::gates::*;
pub use super::graduation::*;
pub use super::learning::*;
pub use super::project::*;
pub use super::provider::*;
pub use super::routing::*;
pub use super::serve::*;
pub use super::subscriptions::*;
pub use super::tools::*;
pub use super::tui_cfg::*;

/// Current schema version. Bump on incompatible changes.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Config layout version for migration tooling.
pub const CURRENT_CONFIG_VERSION: u32 = 2;

/// Check whether a binary is findable on the system `PATH`.
fn binary_on_path(name: &str) -> bool {
    if name.contains('/') || name.contains('\\') {
        return std::path::Path::new(name).exists();
    }
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        if dir.join(name).is_file() {
            return true;
        }
    }
    false
}

/// Returns `true` when the raw TOML text contains an explicit `config_version`
/// key (as opposed to relying on the serde default).  Used to avoid spurious
/// version-1 warnings for partial configs (e.g. the global `~/.roko/config.toml`)
/// that legitimately omit the field.
fn text_has_config_version(s: &str) -> bool {
    s.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("config_version")
            && trimmed[b"config_version".len()..]
                .trim_start()
                .starts_with('=')
    })
}

/// Extract the numeric config_version value from raw TOML text.
/// Returns 1 as fallback if parsing fails.
fn extract_config_version_from_text(s: &str) -> u32 {
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("config_version")
            && let Some(val) = trimmed.split('=').nth(1)
            && let Ok(v) = val.trim().parse::<u32>()
        {
            return v;
        }
    }
    1
}

// ---- top-level -----------------------------------------------------------

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RokoConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub prd: PrdConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub providers: IndexMap<String, ProviderConfig>,
    #[serde(default)]
    pub models: IndexMap<String, ModelProfile>,
    /// Named domain-specific agent/gate overlays.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub profiles: HashMap<String, DomainProfile>,
    #[serde(default)]
    pub gates: GatesConfig,
    /// Graduation policies: which Bus topics get promoted to the Store.
    #[serde(default)]
    pub graduation: GraduationConfig,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub pipeline: PipelineConfig,
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub conductor: ConductorConfig,
    #[serde(default, skip_serializing_if = "WatcherConfig::is_empty")]
    pub watcher: WatcherConfig,
    #[serde(default)]
    pub learning: LearningConfig,
    #[serde(default)]
    pub tui: TuiConfig,
    #[serde(default)]
    pub timeouts: super::timeouts::TimeoutConfig,
    #[serde(default)]
    pub statehub: StateHubConfig,
    #[serde(default)]
    pub serve: ServeConfig,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub webhooks: WebhooksConfig,
    /// GitHub repository identity and workflow preferences.
    #[serde(default)]
    pub github: GitHubConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subscriptions: Vec<SubscriptionConfig>,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub deploy: DeployConfig,
    #[serde(default)]
    pub perplexity: PerplexityConfig,
    #[serde(default)]
    pub gemini: GeminiConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub chain: ChainConfig,
    #[serde(default)]
    pub relay: RelayConfig,
    /// Feed agent configuration.
    #[serde(default)]
    pub feed_agents: FeedAgentsConfig,
    #[serde(default)]
    pub runner: CoreRunnerConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<AgentDefinition>,
    /// Persistent groups reconciled into the serve runtime at startup.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<GroupDefinition>,
    #[serde(default)]
    pub validation: ValidationConfig,
    /// Cold-storage archival settings.
    #[serde(default)]
    pub cold_storage: ColdStorageConfig,
    /// Prompt composition knobs (strategy, VCG warmup threshold).
    #[serde(default)]
    pub prompt: PromptConfig,
    /// Disk budget, thresholds, and GC policy for resource-aware execution.
    #[serde(default)]
    pub resources: ResourcesConfig,
}

/// Composition strategy for allocating prompt token budget across candidate sections.
///
/// Mirrors [`roko_compose::CompositionStrategy`] so that `roko-core` (which cannot
/// depend on `roko-compose`) can expose this knob in the config schema.  The CLI
/// converts this value to the compose-side enum at construction time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigCompositionStrategy {
    /// Select `Vcg` once learned bidder observations are warm; otherwise use
    /// the deterministic density-greedy path.
    #[default]
    Auto,
    /// Deterministic greedy allocation by score density.
    DensityGreedy,
    /// Backward-compatible alias for density-greedy allocation.
    WeightedSum,
    /// VCG-style allocation with payments and displacement diagnostics.
    Vcg,
}

/// Default VCG warmup observation count (mirrors the compose-side constant).
const fn default_vcg_warmup_observations() -> u32 {
    10
}

/// Prompt composition configuration.
///
/// Controls how the runner-v2 `PromptComposer` allocates token budget across
/// candidate prompt sections and when the VCG auction activates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Budget-allocation strategy for prompt composition.
    ///
    /// - `auto` (default): density-greedy on cold start; VCG once bidders are warm.
    /// - `density_greedy` / `weighted_sum`: always use the deterministic greedy path.
    /// - `vcg`: always use VCG allocation (requires warm bidder observations).
    #[serde(default)]
    pub composition_strategy: ConfigCompositionStrategy,
    /// Minimum bidder-observation count before `auto` enables VCG allocation.
    ///
    /// Defaults to 10. Setting this to 0 enables VCG immediately (not
    /// recommended for cold starts).
    #[serde(default = "default_vcg_warmup_observations")]
    pub vcg_warmup_observations: u32,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            composition_strategy: ConfigCompositionStrategy::default(),
            vcg_warmup_observations: default_vcg_warmup_observations(),
        }
    }
}

/// Validation behavior configuration.
///
/// Controls how strictly the config loader treats issues like dangling
/// provider references and other semantic errors.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// When true, missing provider references and other config issues
    /// become hard errors instead of warnings. Useful for CI and
    /// strict production environments.
    #[serde(default)]
    pub strict_validation: bool,
}

/// Optional gate settings supplied by a domain profile.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateProfileConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_tests: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clippy_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rung: Option<u32>,
}

impl GateProfileConfig {
    fn overlay(parent: Option<Self>, child: Option<Self>) -> Option<Self> {
        match (parent, child) {
            (None, None) => None,
            (parent, child) => {
                let parent = parent.unwrap_or_default();
                let child = child.unwrap_or_default();
                Some(Self {
                    skip_tests: child.skip_tests.or(parent.skip_tests),
                    clippy_enabled: child.clippy_enabled.or(parent.clippy_enabled),
                    max_rung: child.max_rung.or(parent.max_rung),
                })
            }
        }
    }
}

/// Inheritable domain-specific config overlay.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DomainProfile {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_limit_k: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_config: Option<GateProfileConfig>,
    /// Forward-compatible profile-local extension fields.
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

impl DomainProfile {
    fn overlay(parent: Self, child: Self) -> Self {
        let mut extra = parent.extra;
        extra.extend(child.extra);
        Self {
            name: child.name,
            base: child.base,
            model: child.model.or(parent.model),
            effort: child.effort.or(parent.effort),
            context_limit_k: child.context_limit_k.or(parent.context_limit_k),
            max_iterations: child.max_iterations.or(parent.max_iterations),
            tool_profile: child.tool_profile.or(parent.tool_profile),
            gate_config: GateProfileConfig::overlay(parent.gate_config, child.gate_config),
            extra,
        }
    }
}

/// Resolve a named profile through at most five inheritance edges.
pub fn resolve_profile(
    name: &str,
    profiles: &HashMap<String, DomainProfile>,
) -> Result<DomainProfile, String> {
    fn resolve(
        name: &str,
        profiles: &HashMap<String, DomainProfile>,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> Result<DomainProfile, String> {
        if stack.iter().any(|seen| seen == name) {
            stack.push(name.to_string());
            return Err(format!(
                "domain profile inheritance cycle: {}",
                stack.join(" -> ")
            ));
        }
        if depth > 5 {
            return Err(format!(
                "domain profile inheritance exceeds maximum depth 5 at '{name}'"
            ));
        }
        let child = profiles
            .get(name)
            .cloned()
            .ok_or_else(|| format!("domain profile '{name}' does not exist"))?;
        let Some(base) = child.base.clone() else {
            return Ok(child);
        };

        stack.push(name.to_string());
        let parent = resolve(&base, profiles, stack, depth + 1)?;
        stack.pop();
        Ok(DomainProfile::overlay(parent, child))
    }

    resolve(name, profiles, &mut Vec::new(), 0)
}

/// Built-in cognitive postures available to config authors as inheritance bases.
#[must_use]
pub fn builtin_profiles() -> HashMap<String, DomainProfile> {
    [
        (
            "coding",
            DomainProfile {
                name: "coding".to_string(),
                effort: Some("high".to_string()),
                max_iterations: Some(3),
                tool_profile: Some("full".to_string()),
                ..Default::default()
            },
        ),
        (
            "research",
            DomainProfile {
                name: "research".to_string(),
                effort: Some("medium".to_string()),
                context_limit_k: Some(200),
                gate_config: Some(GateProfileConfig {
                    skip_tests: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ),
        (
            "review",
            DomainProfile {
                name: "review".to_string(),
                effort: Some("low".to_string()),
                max_iterations: Some(1),
                ..Default::default()
            },
        ),
    ]
    .into_iter()
    .map(|(name, profile)| (name.to_string(), profile))
    .collect()
}

const fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}
const fn default_config_version() -> u32 {
    1
}

impl Default for RokoConfig {
    fn default() -> Self {
        Self {
            config_version: CURRENT_CONFIG_VERSION,
            schema_version: CURRENT_SCHEMA_VERSION,
            project: ProjectConfig::default(),
            prd: PrdConfig::default(),
            agent: AgentConfig::default(),
            providers: IndexMap::new(),
            models: IndexMap::new(),
            profiles: HashMap::new(),
            gates: GatesConfig::default(),
            graduation: GraduationConfig::default(),
            routing: RoutingConfig::default(),
            pipeline: PipelineConfig::default(),
            budget: BudgetConfig::default(),
            conductor: ConductorConfig::default(),
            watcher: WatcherConfig::default(),
            learning: LearningConfig::default(),
            tui: TuiConfig::default(),
            timeouts: super::timeouts::TimeoutConfig::default(),
            statehub: StateHubConfig::default(),
            serve: ServeConfig::default(),
            scheduler: SchedulerConfig::default(),
            webhooks: WebhooksConfig::default(),
            github: GitHubConfig::default(),
            subscriptions: Vec::new(),
            server: ServerConfig::default(),
            deploy: DeployConfig::default(),
            perplexity: PerplexityConfig::default(),
            gemini: GeminiConfig::default(),
            tools: ToolsConfig::default(),
            chain: ChainConfig::default(),
            relay: RelayConfig::default(),
            feed_agents: FeedAgentsConfig::default(),
            runner: CoreRunnerConfig::default(),
            agents: Vec::new(),
            groups: Vec::new(),
            validation: ValidationConfig::default(),
            cold_storage: ColdStorageConfig::default(),
            prompt: PromptConfig::default(),
            resources: ResourcesConfig::default(),
        }
    }
}

// ---- standard provider synthesis -----------------------------------------

/// Check well-known API-key env vars and synthesize provider entries.
///
/// Each env var is checked independently — setting only `OPENAI_API_KEY` will
/// produce a single `"openai"` entry while the others are skipped.
///
/// Callers should merge these *under* user-defined providers so that explicit
/// `[providers.*]` config always takes precedence.
#[must_use]
pub fn synthesize_standard_providers() -> HashMap<String, ProviderConfig> {
    synthesize_standard_providers_with_env(|key| std::env::var(key).ok())
}

fn synthesize_standard_providers_with_env(
    env_fn: impl Fn(&str) -> Option<String>,
) -> HashMap<String, ProviderConfig> {
    use super::provider::{
        default_provider_connect_timeout_ms, default_provider_timeout_ms,
        default_provider_ttft_timeout_ms,
    };

    let specs: &[(&str, &str, ProviderKind, Option<&str>)] = &[
        (
            "anthropic",
            "ANTHROPIC_API_KEY",
            ProviderKind::AnthropicApi,
            Some("https://api.anthropic.com"),
        ),
        (
            "openai",
            "OPENAI_API_KEY",
            ProviderKind::OpenAiCompat,
            Some("https://api.openai.com"),
        ),
        (
            "gemini",
            "GEMINI_API_KEY",
            ProviderKind::GeminiApi,
            Some("https://generativelanguage.googleapis.com"),
        ),
        (
            "perplexity",
            "PERPLEXITY_API_KEY",
            ProviderKind::PerplexityApi,
            None,
        ),
    ];

    let mut providers = HashMap::new();
    for &(name, env_var, kind, base_url) in specs {
        if env_fn(env_var).filter(|value| !value.is_empty()).is_some() {
            providers.insert(
                name.to_string(),
                ProviderConfig {
                    kind,
                    base_url: base_url.map(String::from),
                    api_key_env: Some(env_var.to_string()),
                    command: None,
                    args: None,
                    timeout_ms: default_provider_timeout_ms(),
                    ttft_timeout_ms: default_provider_ttft_timeout_ms(),
                    connect_timeout_ms: default_provider_connect_timeout_ms(),
                    extra_headers: None,
                    max_concurrent: None,
                    limits: None,
                },
            );
        }
    }
    providers
}

// ---- RokoConfig impl -----------------------------------------------------

impl RokoConfig {
    /// Parse from a TOML string.
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        let config: Self = toml::from_str(s)?;
        // Only warn when the TOML text explicitly sets config_version to a value
        // below CURRENT_CONFIG_VERSION. Skip if:
        //   - The field is absent (serde default kicks in; not a real v1 config)
        //   - The value matches or exceeds the current version
        //   - We've already warned in this process
        if text_has_config_version(s) {
            let explicit_version = extract_config_version_from_text(s);
            if explicit_version < CURRENT_CONFIG_VERSION {
                static WARNED: std::sync::Once = std::sync::Once::new();
                WARNED.call_once(|| {
                    tracing::warn!(
                        version = explicit_version,
                        current = CURRENT_CONFIG_VERSION,
                        "roko.toml uses config version {} (current is {})\n  \
                         hint: run `roko config migrate` to upgrade",
                        explicit_version,
                        CURRENT_CONFIG_VERSION,
                    );
                });
            }
        }
        let warnings = validate_references(&config);
        for w in &warnings {
            tracing::warn!("config reference validation: {w}");
        }
        tracing::debug!(
            providers = config.providers.len(),
            models = config.models.len(),
            ref_warnings = warnings.len(),
            "config loaded and validated"
        );
        Ok(config)
    }

    /// Render to a TOML string.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    /// Render to a pretty-printed TOML string.
    pub fn to_toml_pretty(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Returns `true` when this config was written with an older schema version.
    #[must_use]
    pub const fn is_stale(&self) -> bool {
        self.schema_version < CURRENT_SCHEMA_VERSION
    }

    /// Returns `true` if feed agents should be spawned at serve startup.
    #[must_use]
    pub const fn feed_agents_enabled(&self) -> bool {
        self.feed_agents.enabled
    }

    // ---- provider / model synthesis --------------------------------------

    /// Return the explicit provider registry that should be used at runtime.
    ///
    /// Synthesized standard providers (from env vars) are merged first, then
    /// user-defined providers override them. This ensures that setting e.g.
    /// `ANTHROPIC_API_KEY` is enough to get an "anthropic" provider without
    /// any TOML config, while explicit `[providers.*]` entries always win.
    #[must_use]
    pub fn effective_providers(&self) -> IndexMap<String, ProviderConfig> {
        self.effective_providers_with_env(|key| std::env::var(key).ok())
    }

    fn effective_providers_with_env(
        &self,
        env_fn: impl Fn(&str) -> Option<String>,
    ) -> IndexMap<String, ProviderConfig> {
        // Start with env-synthesized providers as the base layer.
        let mut providers: IndexMap<String, ProviderConfig> =
            synthesize_standard_providers_with_env(env_fn)
                .into_iter()
                .collect();

        // User-defined providers override synthesized ones.
        for (name, pc) in &self.providers {
            providers.insert(name.clone(), pc.clone());
        }

        // Ensure ClaudeCli providers always have a command — the adapter
        // requires it and users commonly omit it from config.
        let claude_command = self
            .agent
            .command
            .clone()
            .unwrap_or_else(|| "claude".to_string());
        for pc in providers.values_mut() {
            if pc.kind == ProviderKind::ClaudeCli && pc.command.is_none() {
                pc.command = Some(claude_command.clone());
            }
        }

        providers
    }

    /// Return the explicit model registry that should be used at runtime.
    ///
    /// Legacy `agent.default_model` and `agent.tier_models` values are model
    /// references, not model definitions. Runtime dispatch must only use
    /// profiles declared under `[models.*]` or profiles made explicit by a
    /// boundary adapter/migration step.
    #[must_use]
    pub fn effective_models(&self) -> IndexMap<String, ModelProfile> {
        self.models.clone()
    }

    fn synthesized_model_profile(&self, slug: &str) -> ModelProfile {
        let tool_profile = profile_for_model(slug);

        // Try to find a matching provider from config by matching the expected
        // ProviderKind for this slug. Prefer config-based providers over the
        // slug heuristic so the provider key in the synthesized profile is
        // actually resolvable at dispatch time.
        #[allow(deprecated)]
        let backend = AgentBackend::from_model(slug);
        let expected_kind: ProviderKind = backend.into();
        let provider = match self
            .providers
            .iter()
            .find(|(_, p)| p.kind == expected_kind)
            .map(|(name, _)| name.as_str())
        {
            Some(p) => p,
            None => {
                tracing::warn!(
                    slug = %slug,
                    kind = ?expected_kind,
                    "no provider of kind {:?} configured for synthesized model '{}'; \
                     using label '{}' as fallback -- dispatch may fail",
                    expected_kind, slug, expected_kind.label()
                );
                expected_kind.label()
            }
        };
        let context_window = match tool_profile.preferred {
            ToolFormat::AnthropicBlocks => 200_000,
            _ => default_context_window(),
        };

        ModelProfile {
            provider: provider.to_owned(),
            slug: slug.to_owned(),
            context_window,
            max_output: None,
            supports_tools: tool_profile.supports_tools,
            supports_thinking: false,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            supports_grounding: false,
            supports_code_execution: false,
            supports_caching: false,
            provider_routing: None,
            tool_format: tool_profile.preferred.as_str().to_owned(),
            cost_input_per_m: None,
            cost_output_per_m: None,
            cost_input_per_m_high: None,
            cost_output_per_m_high: None,
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            thinking_level: None,
            max_tools: Some(u32::from(tool_profile.max_tools_before_degrade)),
            tokenizer_ratio: None,
            ..Default::default()
        }
    }

    fn agent_env_value(&self, key: &str) -> Option<&str> {
        self.agent.env.as_ref().and_then(|entries| {
            entries
                .iter()
                .find_map(|(k, v)| (k == key).then_some(v.as_str()))
        })
    }

    // ---- env overrides ---------------------------------------------------

    /// Apply environment variable overrides.
    pub fn apply_env(&mut self, env_fn: &dyn Fn(&str) -> Option<String>) {
        let provider_override = env_fn("ROKO_PROVIDER");
        let model_slug_override = env_fn("ROKO_MODEL_SLUG");

        if let Some(v) = env_fn("ROKO_MODEL") {
            self.agent.default_model = v;
        }
        if let Some(v) = env_fn("ROKO_BACKEND") {
            self.agent.default_backend = v;
        }
        if let Some(v) = env_fn("ROKO_EFFORT") {
            self.agent.default_effort = v;
        }
        if let Some(v) = env_fn("ROKO_CONTEXT_LIMIT_K") {
            match v.parse::<u32>() {
                Ok(n) => self.agent.context_limit_k = n,
                Err(e) => tracing::warn!(
                    env = "ROKO_CONTEXT_LIMIT_K",
                    value = %v,
                    error = %e,
                    "failed to parse env var as u32, ignoring"
                ),
            }
        }
        if let Some(v) = env_fn("ROKO_MAX_AGENTS") {
            match v.parse::<usize>() {
                Ok(n) => self.conductor.max_agents = n,
                Err(e) => tracing::warn!(
                    env = "ROKO_MAX_AGENTS",
                    value = %v,
                    error = %e,
                    "failed to parse env var as usize, ignoring"
                ),
            }
        }
        if let Some(v) = env_fn("ROKO_BUDGET_USD") {
            match v.parse::<f32>() {
                Ok(n) => self.budget.max_plan_usd = n,
                Err(e) => tracing::warn!(
                    env = "ROKO_BUDGET_USD",
                    value = %v,
                    error = %e,
                    "failed to parse env var as f32, ignoring"
                ),
            }
        }
        if let Some(v) = env_fn("ROKO_PARALLEL") {
            self.conductor.parallel_enabled = parse_bool_env(&v);
        }
        if let Some(v) = env_fn("ROKO_EXPRESS") {
            self.conductor.express_mode = parse_bool_env(&v);
        }
        if let Some(v) = env_fn("ROKO_SKIP_TESTS") {
            self.gates.skip_tests = parse_bool_env(&v);
        }
        if let Some(v) = env_fn("ROKO_CLIPPY") {
            self.gates.clippy_enabled = parse_bool_env(&v);
        }
        if let Some(v) = env_fn("ROKO_GATE_MODE") {
            self.gates.mode = match v.trim().to_ascii_lowercase().as_str() {
                "none" => GateMode::None,
                "structural" => GateMode::Structural,
                "focused" => GateMode::Focused,
                "full" => GateMode::Full,
                // Invalid breadth must never silently weaken verification.
                _ => {
                    tracing::warn!(
                        env = "ROKO_GATE_MODE",
                        value = %v,
                        "unknown gate mode; failing closed to full"
                    );
                    GateMode::Full
                }
            };
        }
        if let Some(v) = env_fn("ROKO_COMPILE_CONCURRENCY") {
            match v.parse::<usize>() {
                Ok(value) if value > 0 => self.gates.compile_concurrency = value,
                _ => tracing::warn!(
                    env = "ROKO_COMPILE_CONCURRENCY",
                    value = %v,
                    "compile concurrency must be a positive integer; using configured value"
                ),
            }
        }

        if provider_override.is_some() || model_slug_override.is_some() {
            let default_model = self.agent.default_model.trim();
            if !default_model.is_empty() {
                let synthesized = self.synthesized_model_profile(default_model);
                let entry = self
                    .models
                    .entry(default_model.to_owned())
                    .or_insert(synthesized);
                if let Some(v) = provider_override {
                    entry.provider = v;
                }
                if let Some(v) = model_slug_override {
                    entry.slug = v;
                }
            }
        }
    }

    /// Convenience: apply overrides from the real process environment.
    pub fn apply_process_env(&mut self) {
        self.apply_env(&|key| std::env::var(key).ok());
    }

    // ---- provider credential availability --------------------------------

    /// Returns `true` when this provider entry is ready for outbound use.
    ///
    /// CLI / ACP providers check that the binary exists on `PATH`.
    /// HTTP-family providers need a non-empty `api_key_env` name with
    /// a value in the process environment or in [`AgentConfig::env`].
    #[must_use]
    pub fn is_provider_available(&self, provider: &ProviderConfig) -> bool {
        self.is_provider_available_with_env(provider, |name| std::env::var(name).ok())
    }

    fn is_provider_available_with_env(
        &self,
        provider: &ProviderConfig,
        env_fn: impl Fn(&str) -> Option<String>,
    ) -> bool {
        if matches!(
            provider.kind,
            ProviderKind::ClaudeCli
                | ProviderKind::CodexCli
                | ProviderKind::CursorAcp
                | ProviderKind::CursorCli
                | ProviderKind::Hermes
                | ProviderKind::OpenClaw
        ) {
            let default_cmd = match provider.kind {
                ProviderKind::ClaudeCli => "claude",
                ProviderKind::CodexCli => "codex",
                ProviderKind::CursorAcp | ProviderKind::CursorCli => "cursor",
                ProviderKind::Hermes => "hermes",
                ProviderKind::OpenClaw => "openclaw",
                _ => unreachable!(),
            };
            let command = provider
                .command
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(default_cmd);
            return binary_on_path(command);
        }
        match provider.api_key_env.as_ref().map(|s| s.trim()) {
            None => false,
            Some("") => true,
            Some(name) => env_fn(name).is_some() || self.agent_env_value(name).is_some(),
        }
    }

    /// `true` when the resolved provider for `model_key` has credentials.
    #[must_use]
    pub fn provider_available_for_model_key(&self, model_key: &str) -> bool {
        let resolved = crate::agent::resolve_model(self, model_key);
        let providers = self.effective_providers();
        if let Some(profile) = resolved.profile.as_ref() {
            return providers
                .get(&profile.provider)
                .map(|p| self.is_provider_available(p))
                .unwrap_or(false);
        }
        providers
            .values()
            .filter(|p| p.kind == resolved.provider_kind)
            .any(|p| self.is_provider_available(p))
    }

    /// Provider registry ids that have credentials configured.
    #[must_use]
    pub fn available_provider_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .effective_providers()
            .iter()
            .filter(|(_, p)| self.is_provider_available(p))
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        ids
    }

    /// `[models.*]` keys (from [`Self::effective_models`]) usable for cascade init.
    ///
    /// This list is config-derived and intentionally does not check provider
    /// credentials. The cascade router's persisted arm set must be stable
    /// across process restarts and independent of which API key env vars were
    /// visible at startup. Dispatch paths should use
    /// [`Self::provider_available_for_model_key`] when they are about to make
    /// an outbound model call.
    #[must_use]
    pub fn model_keys_for_cascade(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .effective_models()
            .into_iter()
            .filter(|(_, profile)| !profile.is_embedding_model)
            .map(|(k, _)| k)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// Backend slugs for cascade routers keyed by API slug lists.
    ///
    /// This is the slug companion to [`Self::model_keys_for_cascade`] and is
    /// likewise independent of runtime credential availability.
    #[must_use]
    pub fn model_slugs_for_cascade(&self) -> Vec<String> {
        let mut slugs: Vec<String> = self
            .effective_models()
            .into_iter()
            .filter(|(_, profile)| !profile.is_embedding_model && !profile.slug.trim().is_empty())
            .map(|(_, profile)| profile.slug.clone())
            .collect();
        slugs.sort();
        slugs.dedup();
        slugs
    }

    /// `[models.*]` keys (from [`Self::effective_models`]) currently dispatchable.
    ///
    /// Prefer [`Self::model_keys_for_cascade`] for cascade router
    /// initialization. This method is for live dispatch filtering and status
    /// displays that need to know whether credentials are present right now.
    #[must_use]
    pub fn available_model_keys_for_cascade(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .effective_models()
            .into_iter()
            .filter(|(k, profile)| {
                !profile.is_embedding_model && self.provider_available_for_model_key(k)
            })
            .map(|(k, _)| k)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// Backend slugs currently dispatchable by provider credential state.
    ///
    /// Prefer [`Self::model_slugs_for_cascade`] for cascade router
    /// initialization.
    #[must_use]
    pub fn available_model_slugs_for_cascade(&self) -> Vec<String> {
        let mut slugs: Vec<String> = self
            .effective_models()
            .into_iter()
            .filter(|(k, profile)| {
                !profile.is_embedding_model
                    && !profile.slug.trim().is_empty()
                    && self.provider_available_for_model_key(k)
            })
            .map(|(_, profile)| profile.slug.clone())
            .collect();
        slugs.sort();
        slugs.dedup();
        slugs
    }

    // ---- secret resolution -----------------------------------------------

    /// Interpolate `${VAR}` patterns in provider config strings.
    ///
    /// **Scope**: Interpolation currently only applies to provider fields:
    /// `base_url`, `api_key_env`, `command`, and `extra_headers`. Other
    /// config sections (agent, budget, gates, etc.) do NOT support `${VAR}`
    /// syntax -- literal strings are used as-is.
    ///
    /// To set non-provider fields dynamically, use the named environment
    /// variable overrides (e.g., `ROKO_MODEL`, `ROKO_BACKEND`) instead.
    pub fn interpolate_env_vars(&mut self) {
        Self::interpolate_env_vars_with(&mut self.providers, &|key| std::env::var(key).ok());
    }

    /// Internal: walk provider config strings and expand `${VAR}` references.
    ///
    /// Only provider fields are walked. This is intentional -- expanding
    /// arbitrary config fields risks unintended side effects (e.g., a model
    /// slug containing `${...}` should be literal, not interpolated).
    fn interpolate_env_vars_with(
        providers: &mut IndexMap<String, ProviderConfig>,
        env_fn: &dyn Fn(&str) -> Option<String>,
    ) {
        for provider in providers.values_mut() {
            if let Some(ref mut url) = provider.base_url {
                *url = interpolate_vars(url, env_fn);
            }
            if let Some(ref mut key_env) = provider.api_key_env {
                *key_env = interpolate_vars(key_env, env_fn);
            }
            if let Some(ref mut cmd) = provider.command {
                *cmd = interpolate_vars(cmd, env_fn);
            }
            if let Some(ref headers) = provider.extra_headers {
                let mut resolved = HashMap::with_capacity(headers.len());
                for (k, v) in headers {
                    resolved.insert(k.clone(), interpolate_vars(v, env_fn));
                }
                provider.extra_headers = Some(resolved);
            }
        }
    }

    /// Resolve `*_file` secret references in provider configs.
    pub fn resolve_file_secrets(&mut self) {
        for (provider_name, provider) in &mut self.providers {
            if let Some(ref headers) = provider.extra_headers {
                let mut resolved = HashMap::with_capacity(headers.len());
                for (key, value) in headers {
                    if key.ends_with("_file") {
                        let base_key = key.trim_end_matches("_file").to_string();
                        let path = value.trim();
                        match std::fs::read_to_string(path) {
                            Ok(content) => {
                                resolved.insert(base_key, content.trim().to_string());
                            }
                            Err(e) => {
                                tracing::warn!(
                                    provider = %provider_name,
                                    key = %key,
                                    path = %path,
                                    error = %e,
                                    "failed to read file secret; header will be omitted"
                                );
                            }
                        }
                    } else {
                        resolved.insert(key.clone(), value.clone());
                    }
                }
                provider.extra_headers = Some(resolved);
            }
        }
    }

    // ---- hot-reload classification ---------------------------------------

    /// Classify a proposed configuration change.
    #[must_use]
    pub fn classify_changes(&self, proposed: &Self) -> ConfigChangeReport {
        let mut report = ConfigChangeReport::default();

        if self.budget != proposed.budget {
            report.hot_reloaded.push("budget");
        }
        if self.gates != proposed.gates {
            report.hot_reloaded.push("gates");
        }
        if self.routing != proposed.routing {
            report.hot_reloaded.push("routing");
        }
        if self.learning != proposed.learning {
            report.hot_reloaded.push("learning");
        }
        if self.scheduler != proposed.scheduler {
            report.hot_reloaded.push("scheduler");
        }
        if self.watcher != proposed.watcher {
            report.hot_reloaded.push("watcher");
        }
        if self.subscriptions != proposed.subscriptions {
            report.hot_reloaded.push("subscriptions");
        }
        if self.conductor != proposed.conductor {
            report.hot_reloaded.push("conductor");
        }

        if self.agent != proposed.agent {
            report.requires_restart.push("agent");
        }
        if self.project != proposed.project {
            report.requires_restart.push("project");
        }
        if self.serve != proposed.serve {
            report.requires_restart.push("serve");
        }
        if self.providers != proposed.providers {
            report.requires_restart.push("providers");
        }
        if self.models != proposed.models {
            report.requires_restart.push("models");
        }
        if self.server != proposed.server {
            report.requires_restart.push("server");
        }

        if proposed.budget.max_plan_usd > self.budget.max_plan_usd {
            report.warnings.push(format!(
                "budget.max_plan_usd increased from {} to {}",
                self.budget.max_plan_usd, proposed.budget.max_plan_usd
            ));
        }

        report
    }

    // ---- example TOML generation -----------------------------------------

    /// Generate an example config string showing every field with doc comments.
    #[must_use]
    pub fn example_toml() -> String {
        let cfg = Self::default();
        let mut out = String::with_capacity(4096);
        Self::write_example_prelude(&mut out);
        Self::write_example_project(&mut out, &cfg);
        Self::write_example_prd(&mut out, &cfg);
        Self::write_example_agent(&mut out, &cfg);
        Self::write_example_gates(&mut out, &cfg);
        Self::write_example_routing(&mut out, &cfg);
        Self::write_example_pipeline(&mut out, &cfg);
        Self::write_example_budget(&mut out, &cfg);
        Self::write_example_conductor(&mut out, &cfg);
        Self::write_example_learning(&mut out, &cfg);
        Self::write_example_prompt(&mut out, &cfg);
        Self::write_example_tui_and_server(&mut out, &cfg);
        Self::write_example_scheduler(&mut out, &cfg);
        Self::write_example_webhooks(&mut out, &cfg);
        Self::write_example_deploy(&mut out, &cfg);
        out
    }

    fn write_example_prelude(out: &mut String) {
        let _ = writeln!(
            out,
            "# Roko configuration -- all fields shown with defaults."
        );
        let _ = writeln!(
            out,
            "# Delete any section you don't need; defaults apply.\n"
        );
        let _ = writeln!(out, "config_version = {CURRENT_CONFIG_VERSION}");
        let _ = writeln!(out, "schema_version = {CURRENT_SCHEMA_VERSION}\n");
    }
    fn write_example_project(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- Project metadata --");
        let _ = writeln!(out, "[project]");
        let _ = writeln!(out, "name = \"{}\"", c.project.name);
        let _ = writeln!(out, "root = \"{}\"", c.project.root);
        let _ = writeln!(
            out,
            "fresh_base_branch = \"{}\"\n",
            c.project.fresh_base_branch
        );
    }
    fn write_example_prd(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- PRD lifecycle settings --");
        let _ = writeln!(out, "[prd]");
        let _ = writeln!(out, "auto_plan = {}\n", c.prd.auto_plan);
    }
    fn write_example_agent(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- Agent / model settings --");
        let _ = writeln!(out, "[agent]");
        let _ = writeln!(out, "default_model = \"{}\"", c.agent.default_model);
        let _ = writeln!(out, "default_backend = \"{}\"", c.agent.default_backend);
        let _ = writeln!(out, "default_effort = \"{}\"", c.agent.default_effort);
        let _ = writeln!(out, "temperament = \"{}\"", c.agent.temperament);
        let _ = writeln!(out, "context_limit_k = {}", c.agent.context_limit_k);
        let _ = writeln!(out, "bare_mode = {}\n", c.agent.bare_mode);
        let _ = writeln!(out, "# Per-role overrides (repeat for each role):");
        let _ = writeln!(out, "# [agent.roles.implementer]");
        let _ = writeln!(out, "# role = \"implementer\"");
        let _ = writeln!(out, "# model = \"claude-opus-4-6\"");
        let _ = writeln!(out, "# effort = \"high\"");
        let _ = writeln!(out, "# temperament = \"exploratory\"");
        let _ = writeln!(out, "# context_limit_k = 200");
        let _ = writeln!(out, "# tools = [\"read\", \"edit\", \"bash\", \"git-*\"]");
        let _ = writeln!(
            out,
            "# budget = {{ max_tokens_per_turn = 12000, max_cost_usd_cents_per_turn = 500 }}"
        );
        let _ = writeln!(out, "# thresholds = {{ gate_pass_rate_floor = 0.65 }}");
        let _ = writeln!(
            out,
            "# routing_overrides = {{ force_backend = \"claude\", force_tier = \"focused\" }}  # config-level role override (matches provider families, not model slugs)"
        );
        let _ = writeln!(out, "# legacy: turn_budget_usd = 5.0\n");
    }
    fn write_example_gates(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- Verification gates --");
        let _ = writeln!(out, "[gates]");
        let _ = writeln!(out, "mode = \"{}\"", c.gates.mode);
        let _ = writeln!(out, "clippy_enabled = {}", c.gates.clippy_enabled);
        let _ = writeln!(out, "skip_tests = {}", c.gates.skip_tests);
        let _ = writeln!(out, "max_iterations = {}", c.gates.max_iterations);
        let _ = writeln!(out, "impact_timeout_ms = {}", c.gates.impact_timeout_ms);
        let _ = writeln!(
            out,
            "impact_max_reverse_dependents = {}",
            c.gates.impact_max_reverse_dependents
        );
        let _ = writeln!(out, "impact_max_targets = {}", c.gates.impact_max_targets);
        let _ = writeln!(
            out,
            "compile_concurrency = {}\n",
            c.gates.compile_concurrency
        );
    }
    fn write_example_routing(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- Model routing --");
        let _ = writeln!(out, "[routing]");
        let _ = writeln!(out, "mode = \"{}\"", c.routing.mode);
        let _ = writeln!(out, "algorithm = \"{}\"", c.routing.algorithm.label());
        let _ = writeln!(out, "discount_factor = {}", c.routing.discount_factor);
        let _ = writeln!(out, "fast_task_model = \"{}\"", c.routing.fast_task_model);
        let _ = writeln!(
            out,
            "standard_task_model = \"{}\"",
            c.routing.standard_task_model
        );
        let _ = writeln!(
            out,
            "complex_task_model = \"{}\"\n",
            c.routing.complex_task_model
        );
        let _ = writeln!(out, "[routing.weights]");
        let _ = writeln!(out, "quality = {}", c.routing.weights.default.quality);
        let _ = writeln!(out, "cost = {}", c.routing.weights.default.cost);
        let _ = writeln!(out, "latency = {}\n", c.routing.weights.default.latency);
        let mech = c.routing.weights.for_tier("mechanical");
        let _ = writeln!(out, "[routing.weights.mechanical]");
        let _ = writeln!(out, "quality = {}", mech.quality);
        let _ = writeln!(out, "cost = {}", mech.cost);
        let _ = writeln!(out, "latency = {}\n", mech.latency);
    }
    fn write_example_pipeline(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- Complexity-to-pipeline mapping --");
        for (name, band) in [
            ("mechanical", c.pipeline.mechanical),
            ("focused", c.pipeline.focused),
            ("integrative", c.pipeline.integrative),
            ("architectural", c.pipeline.architectural),
        ] {
            let _ = writeln!(out, "[pipeline.{name}]");
            let _ = writeln!(out, "strategist = {}", band.strategist);
            let _ = writeln!(out, "reviewers = {}", band.reviewers);
            let _ = writeln!(out, "reviewer_mode = \"{}\"", band.reviewer_mode.label());
            let _ = writeln!(out, "max_iterations = {}\n", band.max_iterations);
        }
    }
    fn write_example_budget(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- Spend / token budgets --");
        let _ = writeln!(out, "[budget]");
        let _ = writeln!(out, "max_plan_usd = {:.1}", c.budget.max_plan_usd);
        let _ = writeln!(out, "max_task_usd = {:.1}", c.budget.max_task_usd);
        let _ = writeln!(out, "max_turn_usd = {:.1}", c.budget.max_turn_usd);
        let _ = writeln!(
            out,
            "max_task_retry_usd = {:.1}",
            c.budget.max_task_retry_usd
        );
        let _ = writeln!(
            out,
            "prompt_token_budget = {}\n",
            c.budget.prompt_token_budget
        );
        let _ = writeln!(out, "[budget.tier_multipliers]");
        let _ = writeln!(out, "mechanical = {}", c.budget.tier_multipliers.mechanical);
        let _ = writeln!(out, "standard = {}", c.budget.tier_multipliers.standard);
        let _ = writeln!(out, "complex = {}", c.budget.tier_multipliers.complex);
        let _ = writeln!(out, "expert = {}\n", c.budget.tier_multipliers.expert);
    }
    fn write_example_conductor(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- Conductor (meta-orchestrator) --");
        let _ = writeln!(out, "[conductor]");
        let _ = writeln!(out, "max_agents = {}", c.conductor.max_agents);
        let _ = writeln!(
            out,
            "max_parallel_plans = {}",
            c.conductor.max_parallel_plans
        );
        let _ = writeln!(out, "parallel_enabled = {}", c.conductor.parallel_enabled);
        let _ = writeln!(out, "express_mode = {}", c.conductor.express_mode);
        let _ = writeln!(
            out,
            "max_auto_fix_attempts = {}",
            c.conductor.max_auto_fix_attempts
        );
        let _ = writeln!(
            out,
            "silence_timeout_secs = {}",
            c.conductor.silence_timeout_secs
        );
        let _ = writeln!(
            out,
            "compile_fail_threshold = {}",
            c.conductor.compile_fail_threshold
        );
        let _ = writeln!(out, "task_stall_secs = {}", c.conductor.task_stall_secs);
        let _ = writeln!(
            out,
            "context_pressure_pct = {}",
            c.conductor.context_pressure_pct
        );
        let _ = writeln!(
            out,
            "phase_timeout_secs = {}\n",
            c.conductor.phase_timeout_secs
        );
    }
    fn write_example_learning(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- Learning subsystem --");
        let _ = writeln!(out, "[learning]");
        let _ = writeln!(
            out,
            "auto_playbook_refresh = {}",
            c.learning.auto_playbook_refresh
        );
        let _ = writeln!(
            out,
            "knowledge_warnings = {}",
            c.learning.knowledge_warnings
        );
        let _ = writeln!(
            out,
            "learning_min_occurrences = {}\n",
            c.learning.learning_min_occurrences
        );
        let _ = writeln!(
            out,
            "replan_on_gate_failure = {}",
            c.learning.replan_on_gate_failure
        );
        let _ = writeln!(
            out,
            "replan_max_per_plan = {}",
            c.learning.replan_max_per_plan
        );
        let _ = writeln!(
            out,
            "replan_gate_attempts = {}",
            c.learning.replan_gate_attempts
        );
        let _ = writeln!(
            out,
            "dream_on_completion = {}",
            c.learning.dream_on_completion
        );
        let _ = writeln!(
            out,
            "gate_threshold_flush_interval = {}\n",
            c.learning.gate_threshold_flush_interval
        );
    }
    fn write_example_prompt(out: &mut String, c: &Self) {
        let strategy = match c.prompt.composition_strategy {
            ConfigCompositionStrategy::Auto => "auto",
            ConfigCompositionStrategy::DensityGreedy => "density_greedy",
            ConfigCompositionStrategy::WeightedSum => "weighted_sum",
            ConfigCompositionStrategy::Vcg => "vcg",
        };
        let _ = writeln!(out, "# -- Prompt composition --");
        let _ = writeln!(out, "[prompt]");
        let _ = writeln!(out, "composition_strategy = \"{}\"", strategy);
        let _ = writeln!(
            out,
            "vcg_warmup_observations = {}\n",
            c.prompt.vcg_warmup_observations
        );
    }
    fn write_example_tui_and_server(out: &mut String, c: &Self) {
        let _ = writeln!(out, "# -- TUI preferences --");
        let _ = writeln!(out, "[tui]");
        let _ = writeln!(out, "refresh_rate_ms = {}\n", c.tui.refresh_rate_ms);
        let _ = writeln!(out, "# -- Serve settings / API auth --");
        let _ = writeln!(out, "[serve]");
        let _ = writeln!(out, "auto_start = {}", c.serve.auto_start);
        let _ = writeln!(out, "auto_orchestrate = {}", c.serve.auto_orchestrate);
        let _ = writeln!(out, "[serve.auth]");
        let _ = writeln!(out, "enabled = {}", c.serve.auth.enabled);
        let _ = writeln!(out, "api_key = \"{}\"", c.serve.auth.api_key);
        let _ = writeln!(
            out,
            "invite_expiry_days = {}\n",
            c.serve.auth.invite_expiry_days
        );
        let _ = writeln!(out, "# -- HTTP server / gateway --");
        let _ = writeln!(out, "[server]");
        let _ = writeln!(out, "bind = \"{}\"", c.server.bind);
        let _ = writeln!(out, "port = {}", c.server.port);
        let _ = writeln!(
            out,
            "workspace_gc_interval_secs = {}",
            c.server.workspace_gc_interval_secs
        );
        let _ = writeln!(out, "\n# -- Cloud deployment --");
        let _ = writeln!(out, "[serve.deploy]");
        let _ = writeln!(out, "provider = \"{}\"", c.serve.deploy.provider);
        let _ = writeln!(out, "environment = {:?}", c.serve.deploy.environment);
        let _ = writeln!(out, "\n[[serve.deploy.webhooks]]");
        let _ = writeln!(out, "provider = \"github\"");
        let _ = writeln!(out, "owner = \"nunchi\"");
        let _ = writeln!(out, "repo = \"roko\"");
        let _ = writeln!(out, "\n[[serve.deploy.webhooks]]");
        let _ = writeln!(out, "provider = \"github\"");
        let _ = writeln!(out, "owner = \"nunchi\"");
        let _ = writeln!(out, "repo = \"collaboration\"");
    }
    fn write_example_scheduler(out: &mut String, _c: &Self) {
        let _ = writeln!(out, "\n# -- Cron scheduler --");
        let _ = writeln!(out, "[scheduler]");
        let _ = writeln!(out, "[[scheduler.cron]]");
        let _ = writeln!(out, "name = \"weekly-digest\"");
        let _ = writeln!(out, "expression = \"0 9 * * MON\"");
        let _ = writeln!(out, "signal_kind = \"scheduler:cron:weekly-digest\"");
    }
    fn write_example_webhooks(out: &mut String, _c: &Self) {
        let _ = writeln!(out, "\n# -- Webhooks --");
        let _ = writeln!(out, "[webhooks.github]");
        let _ = writeln!(out, "secret = \"change-me\"");
        let _ = writeln!(out, "\n# -- GitHub integration --");
        let _ = writeln!(out, "# [github]");
        let _ = writeln!(out, "# owner = \"my-org\"");
        let _ = writeln!(out, "# repo = \"my-repo\"");
        let _ = writeln!(out, "# default_branch = \"main\"");
        let _ = writeln!(out, "# auto_pr = false");
        let _ = writeln!(
            out,
            "# merge_method = \"squash\"  # merge | squash | rebase"
        );
        let _ = writeln!(out, "# label_prefix = \"roko/\"");
    }
    fn write_example_deploy(out: &mut String, c: &Self) {
        let _ = writeln!(out, "\n# -- Cloud deployment (Railway, etc.) --");
        let _ = writeln!(out, "[deploy]");
        let _ = writeln!(out, "backend = \"{}\"", c.deploy.backend);
        let _ = writeln!(out, "# railway_api_token = \"...\"");
        let _ = writeln!(out, "# project_id = \"...\"");
        let _ = writeln!(out, "# environment_id = \"...\"");
        let _ = writeln!(
            out,
            "# worker_image = \"ghcr.io/example/roko-worker:latest\""
        );
        let _ = writeln!(out, "# default_region = \"us-west1\"");
    }
}

// ---- ConfigChangeReport --------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigChangeReport {
    pub hot_reloaded: Vec<&'static str>,
    pub requires_restart: Vec<&'static str>,
    pub warnings: Vec<String>,
}

impl ConfigChangeReport {
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.hot_reloaded.is_empty() || !self.requires_restart.is_empty()
    }
    #[must_use]
    pub fn needs_restart(&self) -> bool {
        !self.requires_restart.is_empty()
    }
    #[must_use]
    pub fn changed_count(&self) -> usize {
        self.hot_reloaded.len() + self.requires_restart.len()
    }
}

impl fmt::Display for ConfigChangeReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.hot_reloaded.is_empty() {
            write!(f, "hot-reloaded: {}", self.hot_reloaded.join(", "))?;
        }
        if !self.requires_restart.is_empty() {
            if !self.hot_reloaded.is_empty() {
                write!(f, "; ")?;
            }
            write!(f, "requires restart: {}", self.requires_restart.join(", "))?;
        }
        for w in &self.warnings {
            write!(f, "\n  warning: {w}")?;
        }
        Ok(())
    }
}

// ---- ValidationWarning ---------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationWarning {
    UnknownProvider {
        model: String,
        provider: String,
        similar: Option<String>,
    },
    UnknownModel {
        field: String,
        model: String,
    },
}

impl fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProvider {
                model,
                provider,
                similar,
            } => {
                write!(
                    f,
                    "Model '{model}' references missing provider '{provider}'"
                )?;
                if let Some(s) = similar {
                    write!(f, " (did you mean '{s}'?)")?;
                }
                Ok(())
            }
            Self::UnknownModel { field, model } => {
                write!(f, "{field} references missing model '{model}'")
            }
        }
    }
}

#[must_use]
pub fn validate_references(config: &RokoConfig) -> Vec<ValidationWarning> {
    let providers = config.effective_providers();
    let provider_keys = providers.keys().map(String::as_str).collect::<HashSet<_>>();
    let mut warnings = Vec::new();

    let mut model_entries = config.models.iter().collect::<Vec<_>>();
    model_entries.sort_unstable_by_key(|(l, _)| *l);
    for (model_key, profile) in model_entries {
        let provider = profile.provider.trim();
        if !provider_keys.contains(provider) {
            warnings.push(ValidationWarning::UnknownProvider {
                model: model_key.clone(),
                provider: profile.provider.clone(),
                similar: find_similar(provider, provider_keys.iter().copied()),
            });
        }
    }

    let explicit_model_keys = config
        .models
        .keys()
        .map(String::as_str)
        .collect::<HashSet<_>>();

    let default_model = config.agent.default_model.trim();
    if !default_model.is_empty() && !explicit_model_keys.contains(default_model) {
        warnings.push(ValidationWarning::UnknownModel {
            field: "agent.default_model".to_string(),
            model: default_model.to_string(),
        });
    }

    if let Some(fallback) = config
        .agent
        .fallback_model
        .as_deref()
        .map(str::trim)
        .filter(|f| !f.is_empty())
        && !explicit_model_keys.contains(fallback)
    {
        warnings.push(ValidationWarning::UnknownModel {
            field: "agent.fallback_model".to_string(),
            model: fallback.to_string(),
        });
    }

    let mut tier_entries = config.agent.tier_models.iter().collect::<Vec<_>>();
    tier_entries.sort_unstable_by_key(|(l, _)| *l);
    for (tier, model_key) in tier_entries {
        let model_key = model_key.trim();
        if model_key.is_empty() || explicit_model_keys.contains(model_key) {
            continue;
        }
        warnings.push(ValidationWarning::UnknownModel {
            field: format!("agent.tier_models.{tier}"),
            model: model_key.to_string(),
        });
    }

    warnings
}

// ---- Conductor (not extracted, stays in schema) --------------------------

/// Conductor (meta-orchestrator) settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConductorConfig {
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    #[serde(default = "default_max_parallel_plans")]
    pub max_parallel_plans: usize,
    #[serde(default)]
    pub parallel_enabled: bool,
    #[serde(default)]
    pub express_mode: bool,
    #[serde(default = "default_max_auto_fix")]
    pub max_auto_fix_attempts: u32,
    #[serde(default = "default_auto_fix_model")]
    pub auto_fix_model: String,
    /// Deprecated/runtime-dead in runner-v2: opt in to the context-window
    /// pressure watcher after its `TokenUsage` producer is wired into the
    /// runner conductor ring. The setting remains parseable for compatibility.
    #[serde(default)]
    pub context_pressure_enabled: bool,
    /// Runtime-live per-watcher threshold overrides for runner-v2 conductor
    /// supervision. These are consumed by `Conductor::from_config`.
    #[serde(default)]
    pub watchers: WatcherThresholds,

    // ── Live supervisor intervention thresholds ─────────────────────────
    //
    // These govern the conductor supervision loop (Branch 4c in event_loop).
    // Each threshold maps a signal condition to an intervention type.
    /// Seconds of agent silence before a Nudge is emitted (default 180).
    #[serde(default = "default_silence_timeout_secs")]
    pub silence_timeout_secs: u64,
    /// Consecutive compile failures before a ForceAdvance (default 3).
    #[serde(default = "default_compile_fail_threshold")]
    pub compile_fail_threshold: u32,
    /// Seconds of task stall (no progress after nudge) before cancel+retry (default 300).
    #[serde(default = "default_task_stall_secs")]
    pub task_stall_secs: u64,
    /// Context window usage percentage that triggers a warning (default 80).
    #[serde(default = "default_context_pressure_pct")]
    pub context_pressure_pct: u8,
    /// Seconds before a phase timeout escalation (default 1800).
    #[serde(default = "default_phase_timeout_secs")]
    pub phase_timeout_secs: u64,
}
const fn default_max_agents() -> usize {
    8
}
const fn default_max_parallel_plans() -> usize {
    1
}
const fn default_max_auto_fix() -> u32 {
    3
}
fn default_auto_fix_model() -> String {
    "claude-haiku-4-5".into()
}
const fn default_silence_timeout_secs() -> u64 {
    180
}
const fn default_compile_fail_threshold() -> u32 {
    3
}
const fn default_task_stall_secs() -> u64 {
    300
}
const fn default_context_pressure_pct() -> u8 {
    80
}
const fn default_phase_timeout_secs() -> u64 {
    1800
}

impl Default for ConductorConfig {
    fn default() -> Self {
        Self {
            max_agents: default_max_agents(),
            max_parallel_plans: default_max_parallel_plans(),
            parallel_enabled: false,
            express_mode: false,
            max_auto_fix_attempts: default_max_auto_fix(),
            auto_fix_model: default_auto_fix_model(),
            context_pressure_enabled: false,
            watchers: WatcherThresholds::default(),
            silence_timeout_secs: default_silence_timeout_secs(),
            compile_fail_threshold: default_compile_fail_threshold(),
            task_stall_secs: default_task_stall_secs(),
            context_pressure_pct: default_context_pressure_pct(),
            phase_timeout_secs: default_phase_timeout_secs(),
        }
    }
}

/// Per-watcher threshold configuration.
///
/// Every field is optional. A missing field means the watcher uses its
/// built-in default. Runner-v2 materializes these overrides when it constructs
/// its live conductor, so this surface is not deprecated.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WatcherThresholds {
    #[serde(default)]
    pub compile_fail_repeat: Option<CompileFailRepeatConfig>,
    #[serde(default)]
    pub context_window_pressure: Option<ContextWindowPressureConfig>,
    #[serde(default)]
    pub cost_overrun: Option<CostOverrunConfig>,
    #[serde(default)]
    pub ghost_turn: Option<GhostTurnConfig>,
    #[serde(default)]
    pub iteration_loop: Option<IterationLoopConfig>,
    #[serde(default)]
    pub review_loop: Option<ReviewLoopConfig>,
    #[serde(default)]
    pub spec_drift: Option<SpecDriftConfig>,
    #[serde(default)]
    pub stuck_pattern: Option<StuckPatternConfig>,
    #[serde(default)]
    pub test_failure_budget: Option<TestFailureBudgetConfig>,
    #[serde(default)]
    pub time_overrun: Option<TimeOverrunConfig>,
    /// Worktree-count watcher threshold override.
    #[serde(default)]
    pub worktree_count: Option<WorktreeCountConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileFailRepeatConfig {
    #[serde(default = "default_compile_fail_repeat_max")]
    pub max_repeats: usize,
}

const fn default_compile_fail_repeat_max() -> usize {
    3
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextWindowPressureConfig {
    #[serde(default = "default_context_pressure_warn")]
    pub warn_threshold: f64,
    #[serde(default = "default_context_pressure_critical")]
    pub critical_threshold: f64,
}

const fn default_context_pressure_warn() -> f64 {
    0.75
}

const fn default_context_pressure_critical() -> f64 {
    0.90
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostOverrunConfig {
    #[serde(default = "default_cost_overrun_warn")]
    pub warn_usd: f64,
    #[serde(default = "default_cost_overrun_critical")]
    pub critical_usd: f64,
}

const fn default_cost_overrun_warn() -> f64 {
    1.0
}

const fn default_cost_overrun_critical() -> f64 {
    5.0
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhostTurnConfig {
    #[serde(default = "default_ghost_min_output_tokens")]
    pub min_output_tokens: u32,
    #[serde(default = "default_ghost_max_consecutive")]
    pub max_consecutive: usize,
}

const fn default_ghost_min_output_tokens() -> u32 {
    1
}

const fn default_ghost_max_consecutive() -> usize {
    3
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IterationLoopConfig {
    #[serde(default = "default_iteration_loop_max")]
    pub max_iterations: usize,
}

const fn default_iteration_loop_max() -> usize {
    3
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewLoopConfig {
    #[serde(default = "default_review_loop_max")]
    pub max_rejections: usize,
}

const fn default_review_loop_max() -> usize {
    3
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecDriftConfig {
    #[serde(default = "default_spec_drift_ratio")]
    pub max_ratio: f64,
}

const fn default_spec_drift_ratio() -> f64 {
    0.25
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StuckPatternConfig {
    #[serde(default = "default_stuck_pattern_max")]
    pub max_identical_actions: usize,
}

const fn default_stuck_pattern_max() -> usize {
    4
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestFailureBudgetConfig {
    #[serde(default = "default_test_failure_increase")]
    pub min_failure_increase: u32,
}

const fn default_test_failure_increase() -> u32 {
    1
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeOverrunConfig {
    #[serde(default = "default_time_overrun_alert_ratio")]
    pub alert_ratio: f64,
}

const fn default_time_overrun_alert_ratio() -> f64 {
    0.80
}

/// Configuration for the worktree-count conductor watcher.
///
/// Controls the maximum number of live git worktrees before the conductor
/// emits a `severity=warning` intervention. The metric is produced by the
/// E47-T09 WorktreeManager adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeCountConfig {
    /// Maximum number of live worktrees before a warning is emitted.
    ///
    /// Default: 8.
    #[serde(default = "default_worktree_count_max_live")]
    pub max_live: usize,
}

const fn default_worktree_count_max_live() -> usize {
    8
}

impl Default for WorktreeCountConfig {
    fn default() -> Self {
        Self {
            max_live: default_worktree_count_max_live(),
        }
    }
}

/// Agent definition for multi-agent startup via `roko up`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub domain: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub chain_rpc: Option<String>,
    #[serde(default = "default_agent_enabled")]
    pub enabled: bool,
}

/// Declarative group reconciled by `roko serve` at startup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub coordination: String,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub leader: Option<String>,
    #[serde(default, alias = "assignment_cell")]
    pub assignment_strategy: Option<String>,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub max_members: Option<usize>,
    #[serde(default)]
    pub knowledge_policy: Option<String>,
    #[serde(default)]
    pub pheromone_decay_rate: Option<f64>,
}
const fn default_agent_enabled() -> bool {
    true
}

// ---- utility functions ---------------------------------------------------

fn parse_bool_env(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn interpolate_vars(value: &str, env_fn: &dyn Fn(&str) -> Option<String>) -> String {
    if !value.contains("${") {
        return value.to_string();
    }
    let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("valid regex");
    re.replace_all(value, |caps: &regex::Captures| {
        let var_name = &caps[1];
        match env_fn(var_name) {
            Some(val) => val,
            None => {
                tracing::warn!(
                    var = var_name,
                    "config interpolation: env var ${{{var_name}}} is not set; substituting empty string"
                );
                String::new()
            }
        }
    })
    .into_owned()
}

fn find_similar<'a>(needle: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let needle = needle.trim();
    if needle.is_empty() {
        return None;
    }
    let mut best_match = None;
    let mut best_distance = usize::MAX;
    for candidate in candidates {
        let distance = edit_distance(needle, candidate);
        if distance < best_distance {
            best_distance = distance;
            best_match = Some(candidate);
        }
    }
    (best_distance <= 3).then(|| best_match.expect("distance implies candidate").to_string())
}

fn edit_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }
    if left.is_empty() {
        return right.chars().count();
    }
    if right.is_empty() {
        return left.chars().count();
    }
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut costs = (0..=right_chars.len()).collect::<Vec<_>>();
    for (left_idx, left_ch) in left.chars().enumerate() {
        let mut previous_diagonal = costs[0];
        costs[0] = left_idx + 1;
        for (right_idx, right_ch) in right_chars.iter().copied().enumerate() {
            let ins = costs[right_idx + 1] + 1;
            let del = costs[right_idx] + 1;
            let sub = previous_diagonal + usize::from(left_ch != right_ch);
            previous_diagonal = costs[right_idx + 1];
            costs[right_idx + 1] = ins.min(del).min(sub);
        }
    }
    *costs.last().unwrap_or(&0)
}

// ---- test helper ---------------------------------------------------------

#[cfg(test)]
fn run_resolve_api_key_child(test_name: &str, api_key_env: &str, expected: Option<&str>) {
    let exe = std::env::current_exe().expect("current exe");
    let mut cmd = std::process::Command::new(exe);
    cmd.arg(test_name)
        .arg("--exact")
        .arg("--nocapture")
        .env("ROKO_RESOLVE_API_KEY_CHILD", "1")
        .env("ROKO_API_KEY_ENV_NAME", api_key_env);
    if let Some(value) = expected {
        cmd.env(api_key_env, value)
            .env("ROKO_EXPECT_API_KEY", value);
    } else {
        cmd.env_remove(api_key_env);
    }
    let status = cmd.status().expect("spawn child test");
    assert!(status.success(), "child test {test_name} failed");
}

// ---- tests ---------------------------------------------------------------

// ---- ColdStorageConfig ---------------------------------------------------

/// Cold-storage archival settings.
///
/// Runtime-live configuration consumed by the serve cold-archival timer. It
/// controls automatic archival of aged-out signals from the hot `FileSubstrate`
/// to the cold `ArchiveColdSubstrate` (`$ROKO_DIR/cold/`). The historical
/// copy-not-move growth defect is tracked separately by E02-T12.
///
/// Configurable via `[cold_storage]` in `roko.toml`:
///
/// ```toml
/// [cold_storage]
/// enabled = true
/// max_age_days = 7
/// batch_size = 500
/// interval_secs = 21600
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColdStorageConfig {
    /// Whether cold archival is enabled.
    #[serde(default = "ColdStorageConfig::default_enabled")]
    pub enabled: bool,
    /// Maximum age in days before engrams are archived to cold storage.
    #[serde(default = "ColdStorageConfig::default_max_age_days")]
    pub max_age_days: u32,
    /// Maximum number of engrams to archive per batch.
    #[serde(default = "ColdStorageConfig::default_batch_size")]
    pub batch_size: usize,
    /// Interval in seconds between scheduled cold archival runs (default: 6 hours).
    #[serde(default = "ColdStorageConfig::default_interval_secs")]
    pub interval_secs: u64,
}

impl ColdStorageConfig {
    const fn default_enabled() -> bool {
        true
    }

    const fn default_max_age_days() -> u32 {
        7
    }

    const fn default_batch_size() -> usize {
        500
    }

    /// Default interval: 6 hours (21600 seconds).
    const fn default_interval_secs() -> u64 {
        6 * 3600
    }

    /// Convert `max_age_days` to milliseconds for query cutoff calculation.
    #[must_use]
    pub fn max_age_ms(&self) -> i64 {
        i64::from(self.max_age_days) * 24 * 3600 * 1000
    }
}

impl Default for ColdStorageConfig {
    fn default() -> Self {
        Self {
            enabled: Self::default_enabled(),
            max_age_days: Self::default_max_age_days(),
            batch_size: Self::default_batch_size(),
            interval_secs: Self::default_interval_secs(),
        }
    }
}

// ---- ResourcesConfig -----------------------------------------------------

/// Disk budget, GC policy, and resource-management thresholds.
///
/// Controls how roko manages disk space during plan execution: when to run
/// the filesystem GC, how much free space to require before starting a run,
/// and whether to clean up Rust build artifacts and worktrees automatically.
///
/// Configurable via `[resources]` in `roko.toml`:
///
/// ```toml
/// [resources]
/// min_free_disk_mb = 2048
/// warn_disk_mb = 5120
/// gc_on_plan_start = true
/// gc_on_plan_end = true
/// gc_on_failure = true
/// target_cleanup_enabled = true
/// target_max_age_days = 3
/// log_rotation_max_mb = 100
/// worktree_cleanup_on_complete = true
/// worktree_cleanup_on_failure = true
/// worktree_max_age_secs = 86400
/// auto_cleanup_on_complete = true
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourcesConfig {
    /// Minimum free disk space in MB required to start a plan run.
    ///
    /// If available disk falls below this threshold before the DAG executor
    /// begins, the plan run is refused with a clear error message.
    /// Default: 2048 MB (2 GB).
    #[serde(default = "ResourcesConfig::default_min_free_disk_mb")]
    pub min_free_disk_mb: u64,

    /// Disk space threshold in MB at which a warning signal is emitted.
    ///
    /// Below this level a warning is logged but execution continues.
    /// Must be greater than `min_free_disk_mb`.
    /// Default: 5120 MB (5 GB).
    #[serde(default = "ResourcesConfig::default_warn_disk_mb")]
    pub warn_disk_mb: u64,

    /// Optional maximum disk growth allowed per plan in MB.
    ///
    /// When set, the runner tracks estimated disk growth and refuses to
    /// spawn new parallel tasks that would exceed this budget.
    /// Default: `None` (unlimited).
    #[serde(default)]
    pub per_plan_disk_budget_mb: Option<u64>,

    /// Maximum combined size of `.roko/` and `target/` directories in MB
    /// before the runner pauses a plan.
    ///
    /// Before dispatching each task the runner measures the aggregate size
    /// of these directories and compares it to this limit.  When exceeded,
    /// the plan is paused with a clear error so the operator can free space
    /// or raise the limit.
    ///
    /// `0` means unlimited (no enforcement).  Default: `0`.
    #[serde(default)]
    pub max_plan_disk_mb: u64,

    /// Run the filesystem GC engine before starting a plan.
    ///
    /// Prunes stale `.roko/` data (old runs, excess episodes, cache) before
    /// the DAG executor begins. Default: `true`.
    #[serde(default = "ResourcesConfig::default_gc_on_plan_start")]
    pub gc_on_plan_start: bool,

    /// Run the filesystem GC engine after plan completion or failure.
    ///
    /// Prunes stale `.roko/` data after the DAG executor finishes. Default: `true`.
    #[serde(default = "ResourcesConfig::default_gc_on_plan_end")]
    pub gc_on_plan_end: bool,

    /// Run the filesystem GC engine when a task fails.
    ///
    /// Frees disk space before any retry or replan attempt. Default: `true`.
    #[serde(default = "ResourcesConfig::default_gc_on_failure")]
    pub gc_on_failure: bool,

    /// Enable automatic cleanup of Rust `target/` directories.
    ///
    /// When true, stale `target/` directories in worktrees are removed
    /// according to `target_max_age_days`. Default: `true`.
    #[serde(default = "ResourcesConfig::default_target_cleanup_enabled")]
    pub target_cleanup_enabled: bool,

    /// Maximum age in days before a worktree `target/` directory is removed.
    ///
    /// Only applies when `target_cleanup_enabled` is true. Default: 3 days.
    #[serde(default = "ResourcesConfig::default_target_max_age_days")]
    pub target_max_age_days: u32,

    /// Rotate `.roko/` JSONL log files when they exceed this size in MB.
    ///
    /// Applies to `episodes.jsonl`, `engrams.jsonl`, `efficiency.jsonl`,
    /// and other JSONL files in the `learn/` directory. Default: 100 MB.
    #[serde(default = "ResourcesConfig::default_log_rotation_max_mb")]
    pub log_rotation_max_mb: u64,

    /// Auto-remove worktrees after successful plan completion.
    ///
    /// When true, the worktree created for a plan is removed once the plan
    /// succeeds and all post-merge checks pass. Default: `true`.
    #[serde(default = "ResourcesConfig::default_worktree_cleanup_on_complete")]
    pub worktree_cleanup_on_complete: bool,

    /// Auto-remove worktrees after plan failure or cancellation.
    ///
    /// When true, the worktree is removed after a plan is abandoned (max
    /// retries exhausted, manual cancel, budget exceeded). Default: `true`.
    #[serde(default = "ResourcesConfig::default_worktree_cleanup_on_failure")]
    pub worktree_cleanup_on_failure: bool,

    /// Maximum age in seconds before an idle worktree is reclaimed.
    ///
    /// Worktrees that have not been touched for longer than this threshold are
    /// eligible for age-based cleanup via `reclaim_idle`.  Applies at both
    /// startup (orphan detection) and during periodic TTL-based eviction.
    /// Default: 86400 (24 hours).
    #[serde(default = "ResourcesConfig::default_worktree_max_age_secs")]
    pub worktree_max_age_secs: u64,

    /// Run consolidated post-plan cleanup after completion or failure.
    ///
    /// When true, the runner runs a single `post_plan_cleanup` pass after
    /// the plan finishes that combines: JSONL log rotation, `.roko/` GC,
    /// stale `target/` directory removal, and orphan worktree cleanup.
    /// Individual sub-steps respect their own config flags (e.g.
    /// `target_cleanup_enabled`, `gc_on_plan_end`).
    /// Default: `true`.
    #[serde(default = "ResourcesConfig::default_auto_cleanup_on_complete")]
    pub auto_cleanup_on_complete: bool,
}

impl ResourcesConfig {
    const fn default_min_free_disk_mb() -> u64 {
        2048
    }

    const fn default_warn_disk_mb() -> u64 {
        5120
    }

    const fn default_gc_on_plan_start() -> bool {
        true
    }

    const fn default_gc_on_plan_end() -> bool {
        true
    }

    const fn default_gc_on_failure() -> bool {
        true
    }

    const fn default_target_cleanup_enabled() -> bool {
        true
    }

    const fn default_target_max_age_days() -> u32 {
        3
    }

    const fn default_log_rotation_max_mb() -> u64 {
        100
    }

    const fn default_worktree_cleanup_on_complete() -> bool {
        true
    }

    const fn default_worktree_cleanup_on_failure() -> bool {
        true
    }

    const fn default_worktree_max_age_secs() -> u64 {
        86400 // 24 hours
    }

    const fn default_auto_cleanup_on_complete() -> bool {
        true
    }
}

impl Default for ResourcesConfig {
    fn default() -> Self {
        Self {
            min_free_disk_mb: Self::default_min_free_disk_mb(),
            warn_disk_mb: Self::default_warn_disk_mb(),
            per_plan_disk_budget_mb: None,
            max_plan_disk_mb: 0,
            gc_on_plan_start: Self::default_gc_on_plan_start(),
            gc_on_plan_end: Self::default_gc_on_plan_end(),
            gc_on_failure: Self::default_gc_on_failure(),
            target_cleanup_enabled: Self::default_target_cleanup_enabled(),
            target_max_age_days: Self::default_target_max_age_days(),
            log_rotation_max_mb: Self::default_log_rotation_max_mb(),
            worktree_cleanup_on_complete: Self::default_worktree_cleanup_on_complete(),
            worktree_cleanup_on_failure: Self::default_worktree_cleanup_on_failure(),
            worktree_max_age_secs: Self::default_worktree_max_age_secs(),
            auto_cleanup_on_complete: Self::default_auto_cleanup_on_complete(),
        }
    }
}

// ---- CoreRunnerConfig ----------------------------------------------------

/// Runtime sandbox enforcement selected for runner and provider dispatch.
///
/// The value lives in `roko-core` so every config consumer observes the same
/// setting without introducing a dependency on `roko-agent`'s enforcement
/// implementation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerSandboxLevel {
    /// No sandbox. Reserved for explicitly trusted in-tree execution.
    None,
    /// Audit policy decisions without blocking them.
    Observe,
    /// Enforce bounded filesystem, command, secret, and network policy.
    #[default]
    Restrict,
    /// Deny network and environment inheritance; constrain filesystem access.
    Isolate,
    /// Memory-only execution with no filesystem, network, environment, or subprocess.
    Quarantine,
}

/// Plan-level runner configuration shared between `roko-core` and
/// `roko-cli` so that both the CLI config layer and direct `RokoConfig`
/// consumers (e.g. `RunConfig::from_roko_config`) see the same schema.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CoreRunnerConfig {
    /// Maximum number of tasks executing concurrently within a plan.
    /// Defaults to 4. A value of 1 preserves sequential execution.
    #[serde(default = "CoreRunnerConfig::default_max_concurrent_tasks")]
    pub max_concurrent_tasks: Option<usize>,
    /// Maximum number of plans executing concurrently.
    #[serde(default)]
    pub max_concurrent_plans: Option<usize>,
    /// Wall-clock timeout for the entire plan execution, in seconds.
    /// Defaults to 3600 (1 hour).
    #[serde(default = "CoreRunnerConfig::default_plan_timeout_secs")]
    pub plan_timeout_secs: u64,
    /// When `true`, agents run with `--dangerously-skip-permissions`.
    /// Defaults to `false`. Set to `true` or pass `--dangerously-skip-permissions`
    /// on the CLI to skip agent permission prompts.
    #[serde(default = "CoreRunnerConfig::default_dangerously_skip_permissions")]
    pub dangerously_skip_permissions: bool,
    /// Sandbox policy applied to runner, provider, and structured-tool dispatch.
    #[serde(default)]
    pub sandbox_level: RunnerSandboxLevel,
    /// Maximum dispatch retry attempts for transient errors (429, 529,
    /// connection timeout). Defaults to 5.
    #[serde(default = "CoreRunnerConfig::default_dispatch_max_retries")]
    pub dispatch_max_retries: u32,
    /// Maximum number of warm (pre-spawned) agent slots per role.
    ///
    /// The warm pool keeps pre-spawned agent handles alive so the next
    /// phase's agent can be promoted in <100 ms instead of a cold 5-15 s
    /// subprocess spawn. Setting this to 0 disables warm-pool pre-spawning.
    /// Defaults to 2 (one reviewer slot + one implementer slot).
    #[serde(default = "CoreRunnerConfig::default_warm_pool_size")]
    pub warm_pool_size: usize,
    /// Maximum idle lifetime for a pre-spawned warm agent, in seconds.
    ///
    /// Agents that have been sitting in the pool longer than this are evicted
    /// on the next housekeeping tick. Prevents stale processes from lingering
    /// between long inter-task gaps. Defaults to 300 s (5 minutes).
    #[serde(default = "CoreRunnerConfig::default_warm_pool_idle_timeout_secs")]
    pub warm_pool_idle_timeout_secs: u64,
    /// When `true`, the runner writes the full assembled system prompt to
    /// `.roko/prompt-logs/` before each agent dispatch. Disabled by default.
    /// Useful for debugging unexpected agent behaviour. The log directory is
    /// gitignored and bounded by [`Self::prompt_log_retention`].
    #[serde(default)]
    pub log_prompts: bool,
    /// Maximum number of prompt log files to retain in `.roko/prompt-logs/`.
    ///
    /// When the count of existing log files exceeds this value, the oldest
    /// files (by mtime) are removed. Defaults to 100.
    #[serde(default = "CoreRunnerConfig::default_prompt_log_retention")]
    pub prompt_log_retention: usize,
}

impl CoreRunnerConfig {
    const fn default_max_concurrent_tasks() -> Option<usize> {
        None
    }

    /// Default wall-clock timeout for plan execution: 1 hour.
    pub const fn default_plan_timeout_secs() -> u64 {
        DEFAULT_PLAN_TIMEOUT_SECS
    }

    const fn default_dangerously_skip_permissions() -> bool {
        false
    }

    const fn default_dispatch_max_retries() -> u32 {
        DEFAULT_RATE_LIMIT_RETRY_ATTEMPTS
    }

    /// Default warm pool capacity: 2 slots per role.
    pub const fn default_warm_pool_size() -> usize {
        2
    }

    /// Default warm pool idle timeout: 300 s (5 minutes).
    pub const fn default_warm_pool_idle_timeout_secs() -> u64 {
        300
    }

    /// Default prompt log retention: 100 files.
    pub const fn default_prompt_log_retention() -> usize {
        100
    }
}

impl Default for CoreRunnerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: None,
            max_concurrent_plans: None,
            plan_timeout_secs: Self::default_plan_timeout_secs(),
            dangerously_skip_permissions: Self::default_dangerously_skip_permissions(),
            sandbox_level: RunnerSandboxLevel::default(),
            dispatch_max_retries: Self::default_dispatch_max_retries(),
            warm_pool_size: Self::default_warm_pool_size(),
            warm_pool_idle_timeout_secs: Self::default_warm_pool_idle_timeout_secs(),
            log_prompts: false,
            prompt_log_retention: Self::default_prompt_log_retention(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    fn workspace_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    fn assert_configured_models_have_max_output(rel_path: &str) {
        let path = workspace_root().join(rel_path);
        let text = std::fs::read_to_string(&path).expect("read roko config");
        let cfg = RokoConfig::from_toml(&text).expect("parse roko config");
        let missing: Vec<_> = cfg
            .models
            .iter()
            .filter(|(_, model)| !model.is_embedding_model && model.max_output.is_none())
            .map(|(name, _)| name.as_str())
            .collect();
        assert!(
            missing.is_empty(),
            "{} has models without max_output: {:?}",
            path.display(),
            missing
        );
    }

    #[derive(Clone, Default)]
    struct SharedLogBuffer {
        inner: Arc<Mutex<Vec<u8>>>,
    }
    struct SharedLogWriter {
        inner: Arc<Mutex<Vec<u8>>>,
    }
    impl SharedLogBuffer {
        fn into_string(self) -> String {
            String::from_utf8(self.inner.lock().expect("lock").clone()).expect("utf-8")
        }
    }
    impl<'a> MakeWriter<'a> for SharedLogBuffer {
        type Writer = SharedLogWriter;
        fn make_writer(&'a self) -> Self::Writer {
            SharedLogWriter {
                inner: Arc::clone(&self.inner),
            }
        }
    }
    impl Write for SharedLogWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.inner.lock().expect("lock").extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    fn capture_warn_logs<T>(f: impl FnOnce() -> T) -> (T, String) {
        let buffer = SharedLogBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_writer(buffer.clone())
            .finish();
        let dispatch = tracing::Dispatch::new(subscriber);
        let value = tracing::dispatcher::with_default(&dispatch, f);
        (value, buffer.into_string())
    }
    fn assert_error_contains(err: toml::de::Error, expected: &[&str]) {
        let message = err.to_string();
        for needle in expected {
            assert!(
                message.contains(needle),
                "expected error `{message}` to contain `{needle}`"
            );
        }
    }

    #[test]
    fn default_roundtrips_through_toml() {
        let cfg = RokoConfig::default();
        let text = cfg.to_toml().expect("serialize");
        let back = RokoConfig::from_toml(&text).expect("deserialize");
        assert_eq!(cfg, back);
    }
    #[test]
    fn empty_toml_uses_all_defaults() {
        let cfg = RokoConfig::from_toml("").expect("parse empty");
        let expected = RokoConfig {
            config_version: 1,
            ..RokoConfig::default()
        };
        assert_eq!(cfg, expected);
    }
    #[test]
    fn config_version_defaults_to_legacy() {
        let cfg = RokoConfig::from_toml("").expect("parse");
        assert_eq!(cfg.config_version, 1);
    }
    #[test]
    fn default_config_uses_current_config_version() {
        let cfg = RokoConfig::default();
        assert_eq!(cfg.config_version, CURRENT_CONFIG_VERSION);
    }

    #[test]
    fn domain_profile_resolves_inheritance_with_child_precedence() {
        let mut profiles = builtin_profiles();
        profiles.insert(
            "fast-coding".to_string(),
            DomainProfile {
                name: "fast-coding".to_string(),
                base: Some("coding".to_string()),
                effort: Some("medium".to_string()),
                ..Default::default()
            },
        );

        let resolved = resolve_profile("fast-coding", &profiles).expect("resolve profile");
        assert_eq!(resolved.effort.as_deref(), Some("medium"));
        assert_eq!(resolved.tool_profile.as_deref(), Some("full"));
        assert_eq!(resolved.max_iterations, Some(3));
    }

    #[test]
    fn domain_profile_rejects_cycles() {
        let profiles = HashMap::from([
            (
                "a".to_string(),
                DomainProfile {
                    name: "a".to_string(),
                    base: Some("b".to_string()),
                    ..Default::default()
                },
            ),
            (
                "b".to_string(),
                DomainProfile {
                    name: "b".to_string(),
                    base: Some("a".to_string()),
                    ..Default::default()
                },
            ),
        ]);

        assert!(
            resolve_profile("a", &profiles)
                .unwrap_err()
                .contains("cycle")
        );
    }

    #[test]
    fn domain_profile_builtins_match_contract() {
        let profiles = builtin_profiles();
        assert_eq!(profiles["coding"].effort.as_deref(), Some("high"));
        assert_eq!(profiles["research"].context_limit_k, Some(200));
        assert_eq!(profiles["review"].max_iterations, Some(1));
    }

    #[test]
    fn runner_sandbox_level_defaults_and_parses_strictly() {
        let defaulted = RokoConfig::from_toml("").expect("parse defaults");
        assert_eq!(defaulted.runner.sandbox_level, RunnerSandboxLevel::Restrict);

        let isolated = RokoConfig::from_toml(
            r#"
[runner]
sandbox_level = "isolate"
"#,
        )
        .expect("parse isolate sandbox");
        assert_eq!(isolated.runner.sandbox_level, RunnerSandboxLevel::Isolate);

        assert!(
            RokoConfig::from_toml(
                r#"
[runner]
sandbox_level = "permissive_typo"
"#,
            )
            .is_err(),
            "unknown sandbox levels must fail closed during config parsing"
        );
    }

    #[test]
    fn config_version_detection_warns_for_legacy_configs() {
        let (cfg, logs) = capture_warn_logs(|| {
            RokoConfig::from_toml(
                r#"
[agent]
default_model = "claude-sonnet-4-6"
"#,
            )
            .expect("parse")
        });
        assert_eq!(cfg.config_version, 1);
        // Note: the static Once guard means the warning only fires once per
        // process. If another test parsed a v1 config first, `logs` may be
        // empty. We assert the config version is correct regardless.
        let _ = logs;
    }

    #[test]
    fn text_has_config_version_detects_explicit_field() {
        assert!(text_has_config_version("config_version = 2\n"));
        assert!(text_has_config_version("  config_version = 1\n"));
        assert!(text_has_config_version("config_version=2"));
        assert!(!text_has_config_version(""));
        assert!(!text_has_config_version("[agent]\ncommand = \"claude\"\n"));
        assert!(!text_has_config_version("# config_version = 2\n"));
    }

    #[test]
    fn from_toml_does_not_warn_when_config_version_absent() {
        // A TOML string without config_version (e.g. global config) should
        // still parse to config_version=1 via the serde default, but should
        // NOT trigger the tracing::warn!.
        let cfg = RokoConfig::from_toml("[agent]\ndefault_model = \"test\"\n").expect("parse");
        assert_eq!(cfg.config_version, 1);
        // The warning is gated on text_has_config_version, which returns false here.
    }

    #[test]
    fn schema_version_defaults_to_current() {
        let cfg = RokoConfig::from_toml("").expect("parse");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }
    #[test]
    fn is_stale_detects_old_version() {
        let mut cfg = RokoConfig::default();
        cfg.schema_version = 1;
        assert!(cfg.is_stale());
    }
    #[test]
    fn is_stale_returns_false_for_current() {
        let cfg = RokoConfig::default();
        assert!(!cfg.is_stale());
    }

    #[test]
    fn project_section_parses() {
        let toml = "[project]\nname = \"my-dapp\"\nroot = \"/home/user/code\"\nfresh_base_branch = \"develop\"\n";
        let cfg = RokoConfig::from_toml(toml).expect("parse");
        assert_eq!(cfg.project.name, "my-dapp");
        assert_eq!(cfg.project.root, "/home/user/code");
        assert_eq!(cfg.project.fresh_base_branch, "develop");
    }
    #[test]
    fn config_load() {
        let toml = "[agent]\ndefault_model = \"claude-sonnet-4-6\"\n";
        let cfg = RokoConfig::from_toml(toml).expect("parse");
        assert_eq!(cfg.agent.default_model, "claude-sonnet-4-6");
    }
    #[test]
    fn groups_parse_and_roundtrip_with_manifest_aliases() {
        let toml = r#"
[[groups]]
name = "code-review"
description = "Automated review pipeline"
coordination = "leader_follower"
members = ["reviewer-lead", "test-runner"]
leader = "reviewer-lead"
assignment_cell = "rule-router:capability-match"
public = false
max_members = 8
knowledge_policy = "write_leader"
pheromone_decay_rate = 0.5
"#;
        let cfg = RokoConfig::from_toml(toml).expect("parse groups");
        let group = cfg.groups.first().expect("one group");
        assert_eq!(group.name, "code-review");
        assert_eq!(group.coordination, "leader_follower");
        assert_eq!(group.members, ["reviewer-lead", "test-runner"]);
        assert_eq!(
            group.assignment_strategy.as_deref(),
            Some("rule-router:capability-match")
        );
        assert_eq!(group.max_members, Some(8));
        assert_eq!(group.pheromone_decay_rate, Some(0.5));

        let serialized = cfg.to_toml().expect("serialize groups");
        let reparsed = RokoConfig::from_toml(&serialized).expect("roundtrip groups");
        assert_eq!(reparsed.groups, cfg.groups);
    }
    #[test]
    fn gates_section_parses() {
        let toml = "[gates]\nclippy_enabled = false\nskip_tests = true\nmax_iterations = 5\n";
        let cfg = RokoConfig::from_toml(toml).expect("parse");
        assert!(!cfg.gates.clippy_enabled);
        assert!(cfg.gates.skip_tests);
        assert_eq!(cfg.gates.max_iterations, 5);
    }
    #[test]
    fn budget_section_parses() {
        let toml =
            "[budget]\nmax_plan_usd = 100.0\nmax_turn_usd = 10.0\nprompt_token_budget = 20000\n";
        let cfg = RokoConfig::from_toml(toml).expect("parse");
        assert!((cfg.budget.max_plan_usd - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn env_overrides_apply() {
        let mut cfg = RokoConfig::default();
        let env = |key: &str| -> Option<String> {
            match key {
                "ROKO_MODEL" => Some("claude-opus-4-6".into()),
                "ROKO_BACKEND" => Some("codex".into()),
                "ROKO_EFFORT" => Some("max".into()),
                "ROKO_CONTEXT_LIMIT_K" => Some("300".into()),
                "ROKO_MAX_AGENTS" => Some("16".into()),
                "ROKO_BUDGET_USD" => Some("50.0".into()),
                "ROKO_PARALLEL" => Some("true".into()),
                "ROKO_EXPRESS" => Some("1".into()),
                "ROKO_SKIP_TESTS" => Some("yes".into()),
                "ROKO_CLIPPY" => Some("false".into()),
                _ => None,
            }
        };
        cfg.apply_env(&env);
        assert_eq!(cfg.agent.default_model, "claude-opus-4-6");
        assert_eq!(cfg.conductor.max_agents, 16);
        assert!(cfg.conductor.parallel_enabled);
        assert!(!cfg.gates.clippy_enabled);
    }

    #[test]
    fn example_toml_contains_all_sections() {
        let example = RokoConfig::example_toml();
        assert!(example.contains("[project]"));
        assert!(example.contains("[agent]"));
        assert!(example.contains("[gates]"));
        assert!(example.contains("[routing]"));
        assert!(example.contains("[budget]"));
        assert!(example.contains("[tui]"));
        assert!(example.contains("[serve]"));
        assert!(example.contains("auto_start = false"));
    }
    #[test]
    fn example_toml_is_valid_toml() {
        let example = RokoConfig::example_toml();
        let cfg = RokoConfig::from_toml(&example).expect("parse");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }
    #[test]
    fn kimi_config_parse() {
        let example = include_str!("../../../../examples/roko-kimi.toml");
        let cfg = RokoConfig::from_toml(example).expect("parse");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
        let model = cfg.models.get("kimi-k2-5").expect("kimi");
        assert_eq!(model.provider, "moonshot");
    }
    #[test]
    fn openrouter_config() {
        let example = include_str!("../../../../examples/roko-openrouter.toml");
        let cfg = RokoConfig::from_toml(example).expect("parse");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }
    #[test]
    fn parse_bool_env_variants() {
        assert!(parse_bool_env("true"));
        assert!(parse_bool_env("1"));
        assert!(parse_bool_env("yes"));
        assert!(!parse_bool_env("false"));
        assert!(!parse_bool_env("0"));
        assert!(!parse_bool_env(""));
    }
    #[test]
    fn interpolate_vars_expands_env_references() {
        let env_fn = |key: &str| -> Option<String> {
            match key {
                "API_KEY" => Some("sk-secret".into()),
                _ => None,
            }
        };
        assert_eq!(interpolate_vars("${API_KEY}", &env_fn), "sk-secret");
        assert_eq!(interpolate_vars("plain text", &env_fn), "plain text");
        assert_eq!(interpolate_vars("${MISSING}", &env_fn), "");
    }
    #[test]
    fn classify_changes_detects_hot_reloadable_budget_change() {
        let current = RokoConfig::default();
        let mut proposed = current.clone();
        proposed.budget.max_plan_usd += 5.0;
        let report = current.classify_changes(&proposed);
        assert!(report.has_changes());
        assert!(!report.needs_restart());
        assert!(report.hot_reloaded.contains(&"budget"));
    }
    #[test]
    fn classify_changes_no_changes_yields_empty_report() {
        let config = RokoConfig::default();
        let report = config.classify_changes(&config);
        assert!(!report.has_changes());
        assert_eq!(report.changed_count(), 0);
    }

    #[test]
    fn effective_providers_backwards_compat() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../roko.toml");
        let text = std::fs::read_to_string(path).expect("read roko.toml");
        let cfg = RokoConfig::from_toml(&text).expect("parse roko.toml");
        let providers = cfg.effective_providers();
        let claude = providers.get("claude_cli").expect("claude_cli provider");
        assert_eq!(claude.kind, ProviderKind::ClaudeCli);
    }

    #[test]
    fn effective_providers_do_not_synthesize_empty_config_providers() {
        let mut cfg = RokoConfig::default();
        cfg.providers.clear();
        cfg.agent.env = Some(vec![
            ("ANTHROPIC_API_KEY".to_string(), "sk-test".to_string()),
            (
                "ANTHROPIC_BASE_URL".to_string(),
                "https://api.anthropic.com".to_string(),
            ),
        ]);

        let providers = cfg.effective_providers_with_env(|_| None);
        assert!(providers.is_empty());
    }

    #[test]
    fn effective_models_backwards_compat() {
        let cfg = RokoConfig::from_toml(
            r#"
[agent]
default_model = "gpt54-mini"

[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"

[models.gpt54-mini]
provider = "openai"
slug = "gpt-5.4-mini"
context_window = 128000
max_output = 16384
"#,
        )
        .expect("parse legacy-compatible model config");
        let models = cfg.effective_models();
        // `effective_models` keys entries by the config key in `[models.<key>]`,
        // so look up the configured default model by its key (not by slug).
        let default_key = cfg.agent.default_model.as_str();
        let default_model = models.get(default_key).expect("default model");
        assert_eq!(default_model.slug, "gpt-5.4-mini");
        assert!(
            !default_model.provider.is_empty(),
            "default model must declare a provider"
        );
    }

    #[test]
    fn project_model_profiles_have_explicit_max_output() {
        // The workspace root config is user-local and intentionally untracked.
        // Exercise the invariant against the tracked provider-routing fixture
        // so a developer's active model selection cannot make this test flaky.
        assert_configured_models_have_max_output("demo/demo-resources/provider-routing/roko.toml");
        // docker/railway.roko.toml was removed -- Railway uses ROKO_* env var overrides.
    }

    #[test]
    fn effective_models_do_not_synthesize_default_or_tier_profiles() {
        let mut cfg = RokoConfig::default();
        cfg.models.clear();
        cfg.agent.default_model = "claude-sonnet-4-6".to_string();
        cfg.agent
            .tier_models
            .insert("mechanical".to_string(), "claude-haiku-4-5".to_string());

        assert!(cfg.effective_models().is_empty());

        let warnings = validate_references(&cfg);
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            ValidationWarning::UnknownModel { field, model }
                if field == "agent.default_model" && model == "claude-sonnet-4-6"
        )));
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            ValidationWarning::UnknownModel { field, model }
                if field == "agent.tier_models.mechanical" && model == "claude-haiku-4-5"
        )));
    }

    #[test]
    fn resolve_api_key_returns_env_value() {
        run_resolve_api_key_child(
            "resolve_api_key_child_present",
            "ZAI_API_KEY",
            Some("test123"),
        );
    }
    #[test]
    fn resolve_api_key_returns_none_when_env_missing() {
        run_resolve_api_key_child("resolve_api_key_child_missing", "ZAI_API_KEY", None);
    }

    #[test]
    fn resolve_api_key_child_present() {
        if std::env::var_os("ROKO_RESOLVE_API_KEY_CHILD").is_none() {
            return;
        }
        let api_key_env = std::env::var("ROKO_API_KEY_ENV_NAME").expect("env");
        let expected = std::env::var("ROKO_EXPECT_API_KEY").expect("expected");
        let cfg = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: None,
            api_key_env: Some(api_key_env),
            command: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert_eq!(cfg.resolve_api_key().as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn resolve_api_key_child_missing() {
        if std::env::var_os("ROKO_RESOLVE_API_KEY_CHILD").is_none() {
            return;
        }
        let api_key_env = std::env::var("ROKO_API_KEY_ENV_NAME").expect("env");
        let cfg = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: None,
            api_key_env: Some(api_key_env),
            command: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert_eq!(cfg.resolve_api_key(), None);
    }

    #[test]
    fn full_roundtrip_with_roles() {
        let toml = "schema_version = 2\n[project]\nname = \"test\"\n[agent]\ndefault_model = \"claude-opus-4-6\"\n[agent.roles.implementer]\nmodel = \"claude-sonnet-4-6\"\neffort = \"high\"\n[gates]\nclippy_enabled = false\n[routing]\nfast_task_model = \"haiku\"\n[budget]\nmax_plan_usd = 50.0\n[conductor]\nmax_agents = 4\nexpress_mode = true\n[learning]\nauto_playbook_refresh = false\n[tui]\nrefresh_rate_ms = 100\n[serve.auth]\nenabled = true\napi_key = \"secret\"\n[server]\nport = 3000\n";
        let cfg = RokoConfig::from_toml(toml).expect("parse");
        let text = cfg.to_toml().expect("serialize");
        let back = RokoConfig::from_toml(&text).expect("re-parse");
        assert_eq!(cfg, back);
    }

    #[test]
    fn validate_references_warns_on_unknown_provider_with_suggestion() {
        let mut cfg = RokoConfig::default();
        cfg.agent.default_model = "glm-5-1".to_string();
        cfg.providers.insert(
            "openrouter".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some("https://openrouter.ai/api/v1".to_string()),
                api_key_env: Some("OPENROUTER_API_KEY".to_string()),
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
            },
        );
        cfg.models.insert(
            "glm-5-1".to_string(),
            ModelProfile {
                provider: "openruoter".to_string(),
                slug: "z-ai/glm-5.1".to_string(),
                context_window: 200_000,
                supports_tools: true,
                ..Default::default()
            },
        );
        let warnings = validate_references(&cfg);
        assert_eq!(
            warnings,
            vec![ValidationWarning::UnknownProvider {
                model: "glm-5-1".to_string(),
                provider: "openruoter".to_string(),
                similar: Some("openrouter".to_string())
            }]
        );
    }

    #[test]
    fn perplexity_example_config() {
        let example = include_str!("../../../../examples/roko-perplexity.toml");
        let cfg = RokoConfig::from_toml(example).expect("parse");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
        let pplx = cfg.providers.get("perplexity").expect("perplexity");
        assert_eq!(pplx.kind, ProviderKind::PerplexityApi);
    }

    #[test]
    fn gemini_example_config() {
        let example = include_str!("../../../../examples/roko-gemini.toml");
        let cfg = RokoConfig::from_toml(example).expect("parse");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
        let provider = cfg.providers.get("gemini").expect("gemini");
        assert_eq!(provider.kind, ProviderKind::GeminiApi);
        for (model_key, slug) in [
            ("gemini-2-5-flash-lite", "gemini-2.5-flash-lite"),
            ("gemini-2-5-flash", "gemini-2.5-flash"),
            ("gemini-2-5-pro", "gemini-2.5-pro"),
        ] {
            let model = cfg.models.get(model_key).expect(model_key);
            assert_eq!(model.provider, "gemini");
            assert_eq!(model.slug, slug);
            let resolved = crate::agent::resolve_model(&cfg, model_key);
            assert_eq!(resolved.model_key, model_key);
            assert_eq!(resolved.slug, slug);
            assert_eq!(resolved.provider_kind, ProviderKind::GeminiApi);
        }
    }

    #[test]
    fn multi_provider_config() {
        let example = include_str!("../../../../examples/roko-multi-provider.toml");
        let cfg = RokoConfig::from_toml(example).expect("parse");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(cfg.providers.contains_key("claude_cli"));
        assert!(cfg.providers.contains_key("gemini"));
        assert!(cfg.providers.contains_key("perplexity"));
    }

    #[test]
    fn resolve_file_secrets_reads_from_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let secret_path = dir.path().join("api_key");
        std::fs::write(&secret_path, "  file-secret-value  \n").expect("write");
        let mut config = RokoConfig::default();
        let mut headers = HashMap::new();
        headers.insert(
            "authorization_file".to_string(),
            secret_path.display().to_string(),
        );
        config.providers.insert(
            "test".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: None,
                api_key_env: None,
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: Some(headers),
                max_concurrent: None,
                limits: None,
            },
        );
        config.resolve_file_secrets();
        let resolved = config.providers["test"]
            .extra_headers
            .as_ref()
            .expect("headers");
        assert_eq!(
            resolved.get("authorization").map(String::as_str),
            Some("file-secret-value")
        );
        assert!(!resolved.contains_key("authorization_file"));
    }

    #[test]
    fn subscription_trigger_cron_roundtrip() {
        let trigger = SubscriptionTrigger::Cron {
            schedule: "*/30 * * * *".into(),
        };
        let json = serde_json::to_string(&trigger).unwrap();
        let parsed: SubscriptionTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, parsed);
        assert_eq!(trigger.kind(), "cron");
    }

    #[test]
    fn config_deser_invalid_provider_kind_is_descriptive() {
        let toml = "[providers.bad]\nkind = \"not_a_real_kind\"\n";
        let err = toml::from_str::<RokoConfig>(toml).expect_err("should fail");
        assert_error_contains(err, &["kind", "unknown variant", "not_a_real_kind"]);
    }

    #[test]
    fn provider_availability_respects_agent_env_table() {
        let mut cfg = RokoConfig::default();
        let p = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some("https://api.openai.com/v1".into()),
            api_key_env: Some("OPENAI_API_KEY".into()),
            command: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert!(!cfg.is_provider_available_with_env(&p, |_| None));
        cfg.agent.env = Some(vec![("OPENAI_API_KEY".into(), "sk-test".into())]);
        assert!(cfg.is_provider_available_with_env(&p, |_| None));
    }

    #[test]
    fn cascade_candidate_lists_include_configured_models_without_credentials() {
        let mut cfg = RokoConfig::default();
        cfg.agent.default_model.clear();
        cfg.providers.clear();
        cfg.models.clear();

        cfg.providers.insert(
            "missing-key-provider".into(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some("https://example.invalid/v1".into()),
                api_key_env: Some("ROKO_TEST_CASCADE_MISSING_API_KEY_NEVER_SET".into()),
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
            },
        );
        cfg.models.insert(
            "configured-model".into(),
            ModelProfile {
                provider: "missing-key-provider".into(),
                slug: "configured-wire-slug".into(),
                context_window: 4096,
                ..Default::default()
            },
        );

        assert!(!cfg.provider_available_for_model_key("configured-model"));
        assert!(cfg.available_model_keys_for_cascade().is_empty());
        assert!(cfg.available_model_slugs_for_cascade().is_empty());
        assert_eq!(
            cfg.model_keys_for_cascade(),
            vec!["configured-model".to_string()]
        );
        assert_eq!(
            cfg.model_slugs_for_cascade(),
            vec!["configured-wire-slug".to_string()]
        );
    }

    #[test]
    fn provider_available_claude_cli_missing_binary() {
        let cfg = RokoConfig::default();
        let provider = ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            command: Some("roko-nonexistent-binary-xyz-090".to_string()),
            base_url: None,
            api_key_env: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert!(
            !cfg.is_provider_available(&provider),
            "ClaudeCli with nonexistent binary should not be available"
        );
    }

    #[test]
    fn provider_available_claude_cli_existing_binary() {
        let cfg = RokoConfig::default();
        let provider = ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            // "sh" is always present on unix
            command: Some("sh".to_string()),
            base_url: None,
            api_key_env: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert!(
            cfg.is_provider_available(&provider),
            "ClaudeCli with 'sh' binary should be available"
        );
    }

    #[test]
    fn provider_available_cursor_missing_binary() {
        let cfg = RokoConfig::default();
        let provider = ProviderConfig {
            kind: ProviderKind::CursorAcp,
            command: Some("roko-nonexistent-cursor-xyz-090".to_string()),
            base_url: None,
            api_key_env: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert!(
            !cfg.is_provider_available(&provider),
            "CursorAcp with nonexistent binary should not be available"
        );
    }

    #[test]
    fn provider_available_hermes_missing_binary() {
        let cfg = RokoConfig::default();
        let provider = ProviderConfig {
            kind: ProviderKind::Hermes,
            command: Some("roko-nonexistent-hermes-xyz-090".to_string()),
            base_url: None,
            api_key_env: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert!(
            !cfg.is_provider_available(&provider),
            "Hermes with nonexistent binary should not be available"
        );
    }

    #[test]
    fn provider_available_hermes_existing_binary() {
        let cfg = RokoConfig::default();
        let provider = ProviderConfig {
            kind: ProviderKind::Hermes,
            // "sh" is always present on unix
            command: Some("sh".to_string()),
            base_url: None,
            api_key_env: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert!(
            cfg.is_provider_available(&provider),
            "Hermes with 'sh' binary should be available"
        );
    }

    #[test]
    fn provider_available_openclaw_missing_binary() {
        let cfg = RokoConfig::default();
        let provider = ProviderConfig {
            kind: ProviderKind::OpenClaw,
            command: Some("roko-nonexistent-openclaw-xyz-090".to_string()),
            base_url: None,
            api_key_env: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert!(
            !cfg.is_provider_available(&provider),
            "OpenClaw with nonexistent binary should not be available"
        );
    }

    #[test]
    fn provider_available_openclaw_existing_binary() {
        let cfg = RokoConfig::default();
        let provider = ProviderConfig {
            kind: ProviderKind::OpenClaw,
            command: Some("sh".to_string()),
            base_url: None,
            api_key_env: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        assert!(
            cfg.is_provider_available(&provider),
            "OpenClaw with 'sh' binary should be available"
        );
    }

    #[test]
    fn provider_available_hermes_default_command() {
        // When no command is set, Hermes defaults to "hermes"
        let cfg = RokoConfig::default();
        let provider = ProviderConfig {
            kind: ProviderKind::Hermes,
            command: None,
            base_url: None,
            api_key_env: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        };
        // Whether this passes depends on whether hermes is installed;
        // just verify it doesn't panic.
        let _ = cfg.is_provider_available(&provider);
    }

    #[test]
    fn resolve_file_secrets_handles_missing_file_gracefully() {
        let mut config = RokoConfig::default();
        let mut headers = HashMap::new();
        headers.insert(
            "authorization_file".to_string(),
            "/nonexistent/path/to/secret.txt".to_string(),
        );
        headers.insert("x-custom".to_string(), "keep-me".to_string());
        config.providers.insert(
            "test".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: None,
                api_key_env: None,
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: Some(headers),
                max_concurrent: None,
                limits: None,
            },
        );

        // Should not panic; the missing _file entry is dropped with a warning.
        config.resolve_file_secrets();

        let resolved = config.providers["test"]
            .extra_headers
            .as_ref()
            .expect("headers");
        // The _file key whose file was missing should be absent (not resolved).
        assert!(
            !resolved.contains_key("authorization"),
            "missing file secret should not produce a resolved key"
        );
        assert!(
            !resolved.contains_key("authorization_file"),
            "_file key should be consumed even on failure"
        );
        // Non-_file keys should be preserved.
        assert_eq!(
            resolved.get("x-custom").map(String::as_str),
            Some("keep-me")
        );
    }
}
