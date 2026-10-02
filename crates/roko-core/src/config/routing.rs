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

// ---- [routing.ladder] ----------------------------------------------------

/// One rung of the routing ladder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LadderRung {
    /// Name that `start`, rung hints and escalation refer to (`"cheap"`).
    pub name: String,
    /// `[models.*]` key or model slug the rung runs.
    pub model: String,
}

/// Start rung, by name, for each task tier.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LadderStart {
    /// Start rung of mechanical tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanical: Option<String>,
    /// Start rung of focused tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused: Option<String>,
    /// Start rung of integrative tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrative: Option<String>,
    /// Start rung of architectural tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architectural: Option<String>,
}

impl LadderStart {
    /// D11's start rungs: mechanical and focused tasks on `cheap`,
    /// integrative on `mid`, architectural on `top`.
    #[must_use]
    pub fn d11() -> Self {
        let name = |tier| Some(d11_start_rung(tier).to_string());
        Self {
            mechanical: name(TaskTier::Mechanical),
            focused: name(TaskTier::Focused),
            integrative: name(TaskTier::Integrative),
            architectural: name(TaskTier::Architectural),
        }
    }

    /// The start rung named for `tier`, if any.
    #[must_use]
    pub fn get(&self, tier: TaskTier) -> Option<&str> {
        match tier {
            TaskTier::Mechanical => self.mechanical.as_deref(),
            TaskTier::Focused => self.focused.as_deref(),
            TaskTier::Integrative => self.integrative.as_deref(),
            TaskTier::Architectural => self.architectural.as_deref(),
        }
    }
}

/// Start rung D11 gives a tier, by the default rung names.
const fn d11_start_rung(tier: TaskTier) -> &'static str {
    match tier {
        TaskTier::Mechanical | TaskTier::Focused => "cheap",
        TaskTier::Integrative => "mid",
        TaskTier::Architectural => "top",
    }
}

/// One role's own ladder: a `[[routing.ladder.roles]]` entry.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LadderRoleConfig {
    /// Task role the entry applies to (`"implementer"`, `"reviewer"`, ...).
    pub role: String,
    /// Replaces the ladder's rungs for this role. An empty list takes the
    /// role off the ladder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rungs: Option<Vec<LadderRung>>,
    /// Start rungs for this role. A tier left out uses the ladder's `start`.
    #[serde(default)]
    pub start: LadderStart,
}

/// `[routing.ladder]`: the model rung a plan task without a `model_hint`
/// starts on, by its role and tier, or by its `rung` hint.
///
/// On by default, with DECISIONS D11's executor cascade (gpt-oss-120b, then
/// GLM-4.7, then gpt-5.4-mini) and Sonnet as the last rung. A rung whose
/// model this workspace cannot dispatch (no `[models.*]` entry, no
/// credentials, a disabled provider, no tool use) is skipped; with no rung
/// left the ladder is off and the router picks as before. `--model` and a
/// task's `model_hint` still win, and the cascade router's own pick is only
/// logged. Planning models are `[authoring] planner_model`, not rungs.
///
/// ```toml
/// [routing.ladder]
/// rungs = [
///   { name = "cheap", model = "cerebras-gptoss" },
///   { name = "top", model = "claude-sonnet" },
/// ]
/// start = { mechanical = "cheap", focused = "cheap", integrative = "top", architectural = "top" }
///
/// [[routing.ladder.roles]]
/// role = "reviewer"
/// start = { mechanical = "top", focused = "top" }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LadderConfig {
    /// Route plan tasks by the ladder (decided 2026-09-29: on by default).
    #[serde(default = "super::agent::default_true")]
    pub enabled: bool,
    /// Rungs, cheapest first.
    #[serde(default = "default_ladder_rungs")]
    pub rungs: Vec<LadderRung>,
    /// Start rung for each tier. A tier left out uses D11's rung name; a
    /// name that matches no rung means the first rung.
    #[serde(default = "LadderStart::d11")]
    pub start: LadderStart,
    /// Per-role overrides of `rungs` or `start`. Always serialized, so the
    /// config loader's schema keeps `[[routing.ladder.roles]]`.
    #[serde(default)]
    pub roles: Vec<LadderRoleConfig>,
    /// Probe each rung run by roko's tool loop with one tool-use call at plan
    /// start, at most once a day, and skip a rung that cannot do agent work
    /// (decision 1119, backlog 1121). FAST and `--no-budget` runs never probe.
    #[serde(default = "super::agent::default_true")]
    pub probe: bool,
}

/// D11's cascade plus Sonnet, cheapest first.
fn default_ladder_rungs() -> Vec<LadderRung> {
    [
        ("cheap", "gpt-oss-120b"),
        ("mid", "glm-4.7"),
        ("strong", "gpt-5.4-mini"),
        ("top", crate::defaults::MODEL_FOCUSED),
    ]
    .into_iter()
    .map(|(name, model)| LadderRung {
        name: name.to_string(),
        model: model.to_string(),
    })
    .collect()
}

impl Default for LadderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rungs: default_ladder_rungs(),
            start: LadderStart::d11(),
            roles: Vec::new(),
            probe: true,
        }
    }
}

/// The ladder one task climbs, from [`LadderConfig::resolve`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedLadder<'a> {
    /// The task's rungs (its role's own, else the ladder's), cheapest first.
    pub rungs: &'a [LadderRung],
    /// Indices into `rungs` of the rungs that can run, ascending.
    pub usable: Vec<usize>,
    /// Index into `rungs` of the start rung, always one of `usable`.
    pub start: usize,
}

impl<'a> ResolvedLadder<'a> {
    /// The rung the task starts on.
    #[must_use]
    pub fn start_rung(&self) -> &'a LadderRung {
        &self.rungs[self.start]
    }
}

impl LadderConfig {
    /// The ladder a task of `tier` in `role` climbs, keeping the rungs whose
    /// model `runnable` accepts. `None` when the ladder is off or no rung is
    /// left for the task.
    ///
    /// The start rung is the task's `rung_hint` when that names one of its
    /// rungs, else the one named by the role's `start`, else the ladder's
    /// `start`, else D11; a start name that matches no rung means the first
    /// rung. When that rung cannot run, the task starts on the next runnable
    /// rung above it, else the nearest one below.
    #[must_use]
    pub fn resolve(
        &self,
        role: &str,
        tier: TaskTier,
        rung_hint: Option<&str>,
        runnable: impl Fn(&str) -> bool,
    ) -> Option<ResolvedLadder<'_>> {
        if !self.enabled {
            return None;
        }
        let entry = self.role_entry(role);
        let rungs = self.rungs_for(entry);
        let usable: Vec<usize> = rungs
            .iter()
            .enumerate()
            .filter(|(_, rung)| runnable(&rung.model))
            .map(|(index, _)| index)
            .collect();
        let named = rung_hint
            .and_then(|hint| rungs.iter().position(|rung| rung.name == hint))
            .unwrap_or_else(|| {
                let name = self.start_name(entry, tier);
                rungs.iter().position(|rung| rung.name == name).unwrap_or(0)
            });
        let start = usable
            .iter()
            .copied()
            .find(|&index| index >= named)
            .or_else(|| usable.last().copied())?;
        Some(ResolvedLadder {
            rungs,
            usable,
            start,
        })
    }

    /// Whether `name` names one of the rungs a task in `role` climbs (its
    /// role's own rungs, else the ladder's), as a task's `rung` hint must.
    #[must_use]
    pub fn has_rung(&self, role: &str, name: &str) -> bool {
        self.rungs_for(self.role_entry(role))
            .iter()
            .any(|rung| rung.name == name)
    }

    /// Every rung of the ladder and of its role overrides.
    pub fn all_rungs(&self) -> impl Iterator<Item = &LadderRung> {
        self.rungs.iter().chain(
            self.roles
                .iter()
                .filter_map(|entry| entry.rungs.as_deref())
                .flatten(),
        )
    }

    /// Mistakes worth a warning: a rung without a name or model, a rung name
    /// used twice, a role entry without a role, and a start name that
    /// matches no rung (those tasks start on the first rung).
    #[must_use]
    pub fn issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        let scopes = std::iter::once(("routing.ladder".to_string(), None)).chain(
            self.roles.iter().map(|entry| {
                (
                    format!("routing.ladder.roles `{}`", entry.role.trim()),
                    Some(entry),
                )
            }),
        );
        for (scope, entry) in scopes {
            if entry.is_some_and(|entry| entry.role.trim().is_empty()) {
                issues.push(format!("{scope}: the entry names no role"));
            }
            let rungs = self.rungs_for(entry);
            if entry.is_none_or(|entry| entry.rungs.is_some()) {
                let mut names = std::collections::HashSet::new();
                for rung in rungs {
                    if rung.name.trim().is_empty() || rung.model.trim().is_empty() {
                        issues.push(format!("{scope}: every rung needs a name and a model"));
                    } else if !names.insert(rung.name.as_str()) {
                        issues.push(format!("{scope}: rung `{}` is defined twice", rung.name));
                    }
                }
            }
            if rungs.is_empty() {
                continue;
            }
            for tier in TaskTier::ALL {
                let name = self.start_name(entry, tier);
                if !rungs.iter().any(|rung| rung.name == name) {
                    issues.push(format!(
                        "{scope}: no rung is named `{name}`, so {tier} tasks start on the first \
                         rung"
                    ));
                }
            }
        }
        issues
    }

    fn role_entry(&self, role: &str) -> Option<&LadderRoleConfig> {
        let role = role.trim();
        self.roles
            .iter()
            .find(|entry| entry.role.trim().eq_ignore_ascii_case(role))
    }

    fn rungs_for<'a>(&'a self, entry: Option<&'a LadderRoleConfig>) -> &'a [LadderRung] {
        entry
            .and_then(|entry| entry.rungs.as_deref())
            .unwrap_or(&self.rungs)
    }

    fn start_name<'a>(&'a self, entry: Option<&'a LadderRoleConfig>, tier: TaskTier) -> &'a str {
        entry
            .and_then(|entry| entry.start.get(tier))
            .or_else(|| self.start.get(tier))
            .unwrap_or_else(|| d11_start_rung(tier))
    }

    /// The rungs a task in `role` climbs, cheapest first: its role's own
    /// rungs, else the ladder's. Escalation (gap-460230) walks them upward.
    #[must_use]
    pub fn role_rungs(&self, role: &str) -> &[LadderRung] {
        self.rungs_for(self.role_entry(role))
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
    /// Model for express-mode tasks and the preferred model for one-shot
    /// helper calls (quality judge, error enrichment).
    ///
    /// Deprecated for choosing plan-task models: [`Self::ladder`] does that.
    #[serde(default = "default_fast_model")]
    pub fast_task_model: String,
    /// Deprecated: no routing decision reads it. [`Self::ladder`] picks
    /// plan-task models.
    #[serde(default = "default_standard_model")]
    pub standard_task_model: String,
    /// Deprecated: no routing decision reads it. [`Self::ladder`] picks
    /// plan-task models.
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
    /// `[routing.ladder]`: the model rung each plan task without a
    /// `model_hint` starts on, by role and tier. On by default.
    #[serde(default)]
    pub ladder: LadderConfig,
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
            ladder: LadderConfig::default(),
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

    fn start_model(ladder: &LadderConfig, role: &str, tier: TaskTier) -> Option<String> {
        ladder
            .resolve(role, tier, None, |_| true)
            .map(|resolved| resolved.start_rung().model.clone())
    }

    /// DECISIONS D11: the executor cascade is gpt-oss-120b, then GLM-4.7,
    /// then gpt-5.4-mini; PLAN.md adds Sonnet as the last rung. The ladder
    /// is on by default (decided 2026-09-29).
    #[test]
    fn default_ladder_follows_d11() {
        let ladder = LadderConfig::default();
        assert!(ladder.enabled);
        let rungs: Vec<(&str, &str)> = ladder
            .rungs
            .iter()
            .map(|rung| (rung.name.as_str(), rung.model.as_str()))
            .collect();
        assert_eq!(
            rungs,
            [
                ("cheap", "gpt-oss-120b"),
                ("mid", "glm-4.7"),
                ("strong", "gpt-5.4-mini"),
                ("top", "claude-sonnet-4-6"),
            ]
        );
        let start = |tier| start_model(&ladder, "implementer", tier);
        assert_eq!(start(TaskTier::Mechanical).as_deref(), Some("gpt-oss-120b"));
        assert_eq!(start(TaskTier::Focused).as_deref(), Some("gpt-oss-120b"));
        assert_eq!(start(TaskTier::Integrative).as_deref(), Some("glm-4.7"));
        assert_eq!(
            start(TaskTier::Architectural).as_deref(),
            Some("claude-sonnet-4-6")
        );
        assert!(ladder.issues().is_empty(), "{:?}", ladder.issues());

        // A config without [routing.ladder] gets the same ladder.
        let routing: RoutingConfig = toml::from_str("mode = \"auto_override\"").expect("routing");
        assert_eq!(routing.ladder, ladder);
        assert_eq!(RoutingConfig::default().ladder, ladder);
    }

    #[test]
    fn ladder_skips_rungs_that_cannot_run() {
        let ladder = LadderConfig::default();
        // Only Claude can run: every tier starts on the top rung, as the
        // router's Sonnet default did.
        for tier in TaskTier::ALL {
            let resolved = ladder
                .resolve("implementer", tier, None, |model| {
                    model == "claude-sonnet-4-6"
                })
                .expect("the top rung runs");
            assert_eq!(resolved.start, 3);
            assert_eq!(resolved.usable, [3]);
        }
        // A skipped start rung moves up to the next runnable rung, else down.
        let no_cheap = |model: &str| model != "gpt-oss-120b";
        let resolved = ladder
            .resolve("implementer", TaskTier::Mechanical, None, no_cheap)
            .expect("ladder");
        assert_eq!(resolved.start_rung().name, "mid");
        let only_cheap = |model: &str| model == "gpt-oss-120b";
        let resolved = ladder
            .resolve("implementer", TaskTier::Architectural, None, only_cheap)
            .expect("ladder");
        assert_eq!(resolved.start_rung().name, "cheap");
        // No runnable rung, or the ladder turned off: the router decides.
        assert!(
            ladder
                .resolve("implementer", TaskTier::Focused, None, |_| false)
                .is_none()
        );
        let off = LadderConfig {
            enabled: false,
            ..LadderConfig::default()
        };
        assert!(
            off.resolve("implementer", TaskTier::Focused, None, |_| true)
                .is_none()
        );
    }

    /// gap-dbf2a6: a task's `rung` hint replaces its tier's start rung, and a
    /// hinted rung that cannot run moves up like any start rung. A name that
    /// matches none of the task's rungs is ignored; plan validate rejects it.
    #[test]
    fn rung_hint_sets_the_start_rung() {
        fn hinted_start(
            ladder: &LadderConfig,
            role: &str,
            tier: TaskTier,
            rung: Option<&str>,
            runnable: impl Fn(&str) -> bool,
        ) -> Option<String> {
            ladder
                .resolve(role, tier, rung, runnable)
                .map(|resolved| resolved.start_rung().name.clone())
        }

        let ladder = LadderConfig::default();
        let start = |tier, rung| hinted_start(&ladder, "implementer", tier, rung, |_| true);
        assert_eq!(start(TaskTier::Mechanical, None).as_deref(), Some("cheap"));
        assert_eq!(
            start(TaskTier::Mechanical, Some("strong")).as_deref(),
            Some("strong")
        );
        // A hint may also start a task below its tier's start rung.
        assert_eq!(
            start(TaskTier::Architectural, Some("mid")).as_deref(),
            Some("mid")
        );
        assert_eq!(
            start(TaskTier::Mechanical, Some("stronk")).as_deref(),
            Some("cheap")
        );
        let no_strong = |model: &str| model != "gpt-5.4-mini";
        assert_eq!(
            hinted_start(
                &ladder,
                "implementer",
                TaskTier::Mechanical,
                Some("strong"),
                no_strong
            )
            .as_deref(),
            Some("top")
        );
        assert!(ladder.has_rung("implementer", "strong"));
        assert!(!ladder.has_rung("implementer", "stronk"));

        // A role with its own rungs takes hints by its own rung names.
        let mut with_scribe = ladder.clone();
        with_scribe.roles.push(LadderRoleConfig {
            role: "scribe".to_string(),
            rungs: Some(vec![
                LadderRung {
                    name: "docs".to_string(),
                    model: "gpt-4o-mini".to_string(),
                },
                LadderRung {
                    name: "careful".to_string(),
                    model: "claude-sonnet-4-6".to_string(),
                },
            ]),
            ..LadderRoleConfig::default()
        });
        assert_eq!(
            hinted_start(
                &with_scribe,
                "scribe",
                TaskTier::Mechanical,
                Some("careful"),
                |_| true
            )
            .as_deref(),
            Some("careful")
        );
        assert!(with_scribe.has_rung("scribe", "careful"));
        assert!(!with_scribe.has_rung("scribe", "strong"));
        assert!(with_scribe.has_rung("implementer", "strong"));
    }

    #[test]
    fn ladder_roles_override_start_and_rungs() {
        let routing: RoutingConfig = toml::from_str(
            r#"
            [ladder]
            rungs = [
              { name = "cheap", model = "cerebras-gptoss" },
              { name = "top", model = "claude-sonnet" },
            ]
            start = { integrative = "top" }

            [[ladder.roles]]
            role = "reviewer"
            start = { mechanical = "top" }

            [[ladder.roles]]
            role = "Scribe"
            rungs = [{ name = "docs", model = "gpt-4o-mini" }]

            [[ladder.roles]]
            role = "strategist"
            rungs = []
            "#,
        )
        .expect("ladder config");
        let ladder = &routing.ladder;
        assert!(ladder.enabled);

        let start = |role, tier| start_model(ladder, role, tier);
        assert_eq!(
            start("implementer", TaskTier::Mechanical).as_deref(),
            Some("cerebras-gptoss")
        );
        assert_eq!(
            start("implementer", TaskTier::Integrative).as_deref(),
            Some("claude-sonnet")
        );
        // A tier left out of `start` keeps D11's name.
        assert_eq!(
            start("implementer", TaskTier::Architectural).as_deref(),
            Some("claude-sonnet")
        );
        assert_eq!(
            start("reviewer", TaskTier::Mechanical).as_deref(),
            Some("claude-sonnet")
        );
        assert_eq!(
            start("reviewer", TaskTier::Integrative).as_deref(),
            Some("claude-sonnet"),
            "the role falls back to the ladder's start"
        );
        assert_eq!(
            start("scribe", TaskTier::Architectural).as_deref(),
            Some("gpt-4o-mini")
        );
        assert_eq!(start("strategist", TaskTier::Mechanical), None);
        assert_eq!(
            ladder
                .all_rungs()
                .map(|rung| rung.model.as_str())
                .collect::<Vec<_>>(),
            ["cerebras-gptoss", "claude-sonnet", "gpt-4o-mini"]
        );

        // The scribe's only rung is `docs`, so no start name matches it.
        let scribe = |name: &str, tier: &str| {
            format!(
                "routing.ladder.roles `Scribe`: no rung is named `{name}`, so {tier} tasks start \
                 on the first rung"
            )
        };
        assert_eq!(
            ladder.issues(),
            [
                scribe("cheap", "mechanical"),
                scribe("cheap", "focused"),
                scribe("top", "integrative"),
                scribe("top", "architectural"),
            ]
        );

        let typo =
            toml::from_str::<RoutingConfig>("[ladder]\nstart = { mechancial = \"cheap\" }\n");
        assert!(typo.is_err(), "an unknown tier key is rejected");
    }

    #[test]
    fn ladder_issues_name_duplicate_rungs_and_roleless_entries() {
        let rung = |name: &str, model: &str| LadderRung {
            name: name.to_string(),
            model: model.to_string(),
        };
        let ladder = LadderConfig {
            rungs: vec![
                rung("cheap", "a"),
                rung("cheap", "b"),
                rung("mid", ""),
                rung("top", "c"),
            ],
            roles: vec![LadderRoleConfig::default()],
            ..LadderConfig::default()
        };
        assert_eq!(
            ladder.issues(),
            [
                "routing.ladder: rung `cheap` is defined twice",
                "routing.ladder: every rung needs a name and a model",
                "routing.ladder.roles ``: the entry names no role",
            ]
        );
    }

    /// The config loader drops every key its schema tree lacks, so the
    /// ladder, role entries included, must survive a real load.
    #[test]
    fn ladder_survives_a_config_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("roko.toml");
        std::fs::write(
            &path,
            r#"
schema_version = 2
config_version = 2

[routing.ladder]
enabled = false
rungs = [{ name = "only", model = "claude-sonnet" }]
start = { architectural = "only" }

[[routing.ladder.roles]]
role = "reviewer"
start = { mechanical = "only" }
"#,
        )
        .expect("write roko.toml");

        let config = crate::config::loader::load_config_file(
            &path,
            &crate::config::loader::LoadOptions {
                merge_global: false,
                apply_env_overrides: false,
                apply_hierarchical_env: false,
                ..crate::config::loader::LoadOptions::default()
            },
        )
        .expect("load roko.toml");

        let ladder = &config.routing.ladder;
        assert!(!ladder.enabled);
        assert_eq!(ladder.rungs.len(), 1);
        assert_eq!(ladder.start.get(TaskTier::Architectural), Some("only"));
        assert_eq!(ladder.start.get(TaskTier::Mechanical), None);
        assert_eq!(ladder.roles.len(), 1);
        assert_eq!(ladder.roles[0].role, "reviewer");
        assert_eq!(
            ladder.roles[0].start.get(TaskTier::Mechanical),
            Some("only")
        );
    }
}
