//! Model routing configuration.

use serde::{Deserialize, Serialize};

use crate::task::TaskTier;

// ---- [routing] -----------------------------------------------------------

/// Routing algorithm for model selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoutingAlgorithm {
    /// Contextual bandit using upper-confidence bounds.
    LinUcb,
    /// Discounted Thompson sampling for non-stationary routing.
    Thompson,
}

impl RoutingAlgorithm {
    /// Stable config label used in TOML.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::LinUcb => "linucb",
            Self::Thompson => "thompson",
        }
    }
}

impl Default for RoutingAlgorithm {
    fn default() -> Self {
        Self::LinUcb
    }
}

/// Reward weights used to scalarize quality, cost, and latency signals.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardWeights {
    /// Relative weight for quality / success.
    #[serde(default = "default_reward_weight_quality")]
    pub quality: f64,
    /// Relative weight for low cost.
    #[serde(default = "default_reward_weight_cost")]
    pub cost: f64,
    /// Relative weight for low latency.
    #[serde(default = "default_reward_weight_latency")]
    pub latency: f64,
    /// Relative weight for knowledge-informed routing bias.
    /// Falls back to `latency` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_bias: Option<f64>,
    /// Multiplier weight for provider pass-rate bias.
    ///
    /// When set, the confidence/UCB score for each model is multiplied by
    /// `lerp(1.0, provider_pass_rate, provider_pass_rate_weight)` so that
    /// providers with poor historical pass rates are softly down-weighted
    /// without being completely excluded. Defaults to 0.0 (disabled).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_pass_rate_weight: Option<f64>,
}

const fn default_reward_weight_quality() -> f64 {
    0.5
}

const fn default_reward_weight_cost() -> f64 {
    0.3
}

const fn default_reward_weight_latency() -> f64 {
    0.2
}

impl Default for RewardWeights {
    fn default() -> Self {
        Self {
            quality: default_reward_weight_quality(),
            cost: default_reward_weight_cost(),
            latency: default_reward_weight_latency(),
            knowledge_bias: None,
            provider_pass_rate_weight: None,
        }
    }
}

/// Per-tier reward-weight overrides for routing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutingRewardWeightsConfig {
    /// Default weights used when a tier has no explicit override.
    #[serde(flatten)]
    pub default: RewardWeights,
    /// Optional override for mechanical tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanical: Option<RewardWeights>,
    /// Optional override for focused tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused: Option<RewardWeights>,
    /// Optional override for integrative tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrative: Option<RewardWeights>,
    /// Optional override for architectural tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architectural: Option<RewardWeights>,
}

impl RoutingRewardWeightsConfig {
    /// Resolve the effective weights for a task tier, read by
    /// [`TaskTier::parse`]. An unknown tier gets the default weights.
    #[must_use]
    pub fn for_tier(&self, tier: &str) -> RewardWeights {
        let tier_weights = match TaskTier::parse(tier) {
            Some(TaskTier::Mechanical) => self.mechanical,
            Some(TaskTier::Focused) => self.focused,
            Some(TaskTier::Integrative) => self.integrative,
            Some(TaskTier::Architectural) => self.architectural,
            None => None,
        };
        tier_weights.unwrap_or(self.default)
    }
}

impl Default for RoutingRewardWeightsConfig {
    fn default() -> Self {
        Self {
            default: RewardWeights::default(),
            mechanical: None,
            focused: None,
            integrative: None,
            architectural: None,
        }
    }
}

/// Model routing configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingConfig {
    /// Routing mode (`"auto_override"`).
    #[serde(default = "default_routing_mode")]
    pub mode: String,
    /// Online learning algorithm used by the router.
    #[serde(default)]
    pub algorithm: RoutingAlgorithm,
    /// Discount factor for Thompson sampling in non-stationary environments.
    #[serde(default = "default_routing_discount_factor")]
    pub discount_factor: f64,
    /// Model for low-complexity tasks.
    #[serde(default = "default_fast_model")]
    pub fast_task_model: String,
    /// Model for standard-complexity tasks.
    #[serde(default = "default_standard_model")]
    pub standard_task_model: String,
    /// Model for high-complexity / retry tasks.
    #[serde(default = "default_complex_model")]
    pub complex_task_model: String,
    /// Reward scalarization weights with optional per-tier overrides.
    #[serde(default)]
    pub weights: RoutingRewardWeightsConfig,
    /// Context strategy (`"mcp_first"`, `"hybrid"`, `"inline_heavy"`).
    #[serde(default = "default_context_strategy")]
    pub context_strategy: String,
    /// Provider IDs to exclude from routing regardless of health status.
    ///
    /// Uses provider IDs as defined in `[providers.<id>]` sections (e.g.
    /// `"openai"`, `"anthropic"`). Models backed by a disabled provider are
    /// filtered out before the cascade router sees them.
    ///
    /// ```toml
    /// [routing]
    /// disabled_providers = ["openai"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_providers: Vec<String>,
    /// Ordered `[models.*]` keys (or slugs) a plan task fails over to when the
    /// provider behind its model is out of usage or its circuit is open —
    /// e.g. the Claude CLI printing "You've hit your session limit".
    ///
    /// Setup: the list is empty by default, so failover is opt-in. Name models
    /// here, then give their providers an API key. HTTP providers read the key
    /// only from the env var named by `[providers.<id>] api_key_env`; in the
    /// shipped roko.toml that is `MOONSHOT_API_KEY` (moonshot: `kimi-k2-5`),
    /// `ZAI_API_KEY` (zai: `glm51`), `OPENAI_API_KEY` (openai: `gpt-4o`,
    /// `gpt-4o-mini`), `GROQ_API_KEY` (groq), and `CEREBRAS_API_KEY`
    /// (cerebras); `PERPLEXITY_API_KEY` feeds `sonar`, which has no tools and
    /// is never chosen. The CLI loads `~/.roko/.env` and
    /// `<workdir>/.roko/.env` automatically at startup, so keys can live there
    /// (`MOONSHOT_API_KEY=…`) instead of the shell profile.
    ///
    /// Behaviour: the refusing provider is skipped until its reported reset
    /// ("resets 4pm", read in local time), else for
    /// [`Self::exhaustion_cooldown_secs`], and the first candidate whose
    /// provider is healthy, not disabled, supports tools, and has a key runs
    /// the task in the same attempt (logged at WARN; cost and episode records
    /// carry the model that actually ran). `agent.fallback_model` is tried
    /// last. A task's `model_hint` is a preference, not a pin; an explicit
    /// `--model` override never fails over. With no usable candidate the task
    /// fails once with an error naming the missing env vars and reset time.
    /// Models whose slug roko does not recognise get a 3-tool cap unless their
    /// profile sets `max_tools`.
    ///
    /// ```toml
    /// [routing]
    /// fallback_models = ["kimi-k2-5", "glm51", "gpt-4o"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback_models: Vec<String>,
    /// How long an out-of-usage provider is skipped when it did not say when
    /// its window resets (seconds). A parsed reset time ("resets 4pm") always
    /// wins over this.
    #[serde(default = "default_exhaustion_cooldown_secs")]
    pub exhaustion_cooldown_secs: u64,
}

fn default_routing_mode() -> String {
    "auto_override".into()
}

fn default_fast_model() -> String {
    "claude-haiku-4-5".into()
}

fn default_standard_model() -> String {
    "claude-sonnet-4-6".into()
}

fn default_complex_model() -> String {
    "claude-opus-4-6".into()
}

fn default_context_strategy() -> String {
    "mcp_first".into()
}

const fn default_routing_discount_factor() -> f64 {
    0.99
}

const fn default_exhaustion_cooldown_secs() -> u64 {
    30 * 60
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            mode: default_routing_mode(),
            algorithm: RoutingAlgorithm::default(),
            discount_factor: default_routing_discount_factor(),
            fast_task_model: default_fast_model(),
            standard_task_model: default_standard_model(),
            complex_task_model: default_complex_model(),
            weights: RoutingRewardWeightsConfig::default(),
            context_strategy: default_context_strategy(),
            disabled_providers: Vec::new(),
            fallback_models: Vec::new(),
            exhaustion_cooldown_secs: default_exhaustion_cooldown_secs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_models_and_exhaustion_cooldown_parse_with_defaults() {
        let routing: RoutingConfig = toml::from_str(
            r#"
            mode = "auto_override"
            fallback_models = ["kimi-k2-5", "glm51", "gpt-4o"]
            "#,
        )
        .expect("routing config");
        assert_eq!(routing.fallback_models, ["kimi-k2-5", "glm51", "gpt-4o"]);
        assert_eq!(routing.exhaustion_cooldown_secs, 1_800);

        let defaults = RoutingConfig::default();
        assert!(defaults.fallback_models.is_empty());
        assert_eq!(defaults.exhaustion_cooldown_secs, 1_800);
    }
}
