//! Model routing configuration.

use std::collections::BTreeMap;

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
    /// More `[models.*]` keys or slugs the rung may run in place of `model`
    /// (gap-51d2ca, decision 4111 option B1). With `[routing]
    /// explore_epsilon` ε above 0, a try the ladder routes to the rung draws
    /// among the k of the rung's models that can run here: `model` with
    /// probability 1 − ε + ε/k, each other one with ε/k. The route row logs
    /// that propensity, and the cascade router learns from the outcome at
    /// its inverse-propensity weight. A rung whose `model` cannot run is
    /// skipped whatever its alternates, a rung with one model that can run
    /// never draws, and climbs and failover go by rung as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternates: Vec<String>,
}

impl LadderRung {
    /// A rung named `name` that runs `model` alone.
    #[must_use]
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            alternates: Vec::new(),
        }
    }

    /// The rung's models: `model`, then its alternates.
    pub fn models(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.model.as_str()).chain(self.alternates.iter().map(String::as_str))
    }
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
///
/// A rung may name `alternates`, models it draws among at `[routing]
/// explore_epsilon` so that the cascade router learns from plan runs
/// ([`LadderRung::alternates`], gap-51d2ca):
///
/// ```toml
/// [routing.ladder]
/// rungs = [
///   { name = "cheap", model = "cerebras-gptoss" },
///   { name = "mid", model = "glm-4-7", alternates = ["kimi-k2-5"] },
///   { name = "top", model = "claude-sonnet" },
/// ]
/// ```
///
/// Each try on a rung is bounded (gap-3e770b): a try that uses up its rung's
/// turns, spend or time is judged by the task's checks on the work it left,
/// and fails on that rung when they fail, so two such tries climb.
/// `max_turns`, `max_usd` and `max_secs` set a rung's allowance by name; a
/// rung `max_turns` leaves out gets [`LadderConfig::try_turns`]'s default,
/// and one `max_secs` leaves out [`LadderConfig::try_secs`]'s (bug-f0007b).
///
/// ```toml
/// [routing.ladder]
/// max_turns = { cheap = 12, mid = 30 }
/// max_usd = { cheap = 0.06 }
/// max_secs = { cheap = 300, mid = 900 }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Turns one try of a task may take on a rung, by rung name
    /// ([`Self::try_turns`]).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub max_turns: BTreeMap<String, u32>,
    /// What one try of a task may spend on a rung, in USD, by rung name
    /// ([`Self::try_usd`]). A rung it leaves out has no spend allowance of
    /// its own; the plan's budget bounds every try either way, and is asked
    /// first.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub max_usd: BTreeMap<String, f64>,
    /// How long one try of a task may run on a rung, in seconds, by rung
    /// name ([`Self::try_secs`], bug-f0007b). `0` leaves the rung's tries
    /// without a time bound.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub max_secs: BTreeMap<String, u64>,
    /// Seconds per model call above which a rung's tries are slow, which a
    /// run says once with a `rung_slow` event ([`Self::slow_call_secs`],
    /// gap-3217a0); unset is [`SLOW_CALL_SECS`], `0` turns it off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slow_call_secs: Option<u64>,
    /// Seconds one try waits in all for a slot at a provider the run's own
    /// calls fill, before failover may hand the try to a substitute
    /// ([`Self::rate_limit_wait_secs`], bug-62ae1e); unset is
    /// [`RATE_LIMIT_WAIT_SECS`], `0` never waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_wait_secs: Option<u64>,
}

/// Fewest turns a try gets by default on a rung it can still climb from
/// ([`LadderConfig::try_turns`]).
pub const MIN_TRY_TURNS: u32 = 12;

/// Fewest seconds a try gets by default on a rung it can still climb from
/// ([`LadderConfig::try_secs`]).
pub const MIN_TRY_SECS: u64 = 300;

/// Seconds per model call above which a rung's tries are slow by default
/// ([`LadderConfig::slow_call_secs`]): a minute, well above the 1 to 10 s a
/// call takes on the API rungs of round v6 and v7, and below the two minutes
/// glm-4.7's reasoning took per call on Undertow's phase 1.
pub const SLOW_CALL_SECS: u64 = 60;

/// Seconds one try waits in all by default for a slot at a provider the
/// run's own calls fill ([`LadderConfig::rate_limit_wait_secs`]): ten
/// minutes, several of the calls of up to two minutes glm-4.7 made on
/// Undertow's phase 1.
pub const RATE_LIMIT_WAIT_SECS: u64 = 600;

/// D11's cascade plus Sonnet, cheapest first.
fn default_ladder_rungs() -> Vec<LadderRung> {
    [
        ("cheap", "gpt-oss-120b"),
        ("mid", "glm-4.7"),
        ("strong", "gpt-5.4-mini"),
        ("top", crate::defaults::MODEL_FOCUSED),
    ]
    .into_iter()
    .map(|(name, model)| LadderRung::new(name, model))
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
            max_turns: BTreeMap::new(),
            max_usd: BTreeMap::new(),
            max_secs: BTreeMap::new(),
            slow_call_secs: None,
            rate_limit_wait_secs: None,
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
    /// Index into `rungs` of the start rung the task's hint or tier names,
    /// which may not run.
    pub requested: usize,
}

impl<'a> ResolvedLadder<'a> {
    /// The rung the task starts on.
    #[must_use]
    pub fn start_rung(&self) -> &'a LadderRung {
        &self.rungs[self.start]
    }

    /// Whether no rung at or above the requested start can run, so the task
    /// starts below it (gap-ff2c38).
    #[must_use]
    pub fn start_unreachable(&self) -> bool {
        self.usable
            .last()
            .is_some_and(|&highest| highest < self.requested)
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
    /// rung above it. When no rung above it can run either, as in a chat run
    /// that drops a CLI rung at the top, it starts on the runnable rung below
    /// the highest one, so it can still climb once (gap-ff2c38), or on the
    /// only one that runs.
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
        let requested = rung_hint
            .and_then(|hint| rungs.iter().position(|rung| rung.name == hint))
            .unwrap_or_else(|| {
                let name = self.start_name(entry, tier);
                rungs.iter().position(|rung| rung.name == name).unwrap_or(0)
            });
        let start = match usable.iter().copied().find(|&index| index >= requested) {
            Some(index) => index,
            None => {
                let highest = usable.len().checked_sub(1)?;
                usable[highest.saturating_sub(1)]
            }
        };
        Some(ResolvedLadder {
            rungs,
            usable,
            start,
            requested,
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
    /// used twice, a role entry without a role, a start name that matches no
    /// rung (those tasks start on the first rung), and a try allowance for no
    /// rung or of no turns or no money.
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
                    let mut models = std::collections::HashSet::from([rung.model.trim()]);
                    for alternate in rung.alternates.iter().map(|model| model.trim()) {
                        if alternate.is_empty() {
                            issues.push(format!(
                                "{scope}: rung `{}` has an alternate that names no model",
                                rung.name
                            ));
                        } else if !models.insert(alternate) {
                            issues.push(format!(
                                "{scope}: rung `{}` names `{alternate}` twice",
                                rung.name
                            ));
                        }
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
        let named = |name: &str| self.all_rungs().any(|rung| rung.name == name);
        for (name, turns) in &self.max_turns {
            if !named(name) {
                issues.push(format!(
                    "routing.ladder.max_turns: no rung is named `{name}`"
                ));
            } else if *turns == 0 {
                issues.push(format!(
                    "routing.ladder.max_turns: `{name}` = 0 gives a try one turn"
                ));
            }
        }
        for (name, usd) in &self.max_usd {
            if !named(name) {
                issues.push(format!("routing.ladder.max_usd: no rung is named `{name}`"));
            } else if !(usd.is_finite() && *usd > 0.0) {
                issues.push(format!(
                    "routing.ladder.max_usd: `{name}` = {usd} is not a positive amount, so its \
                     tries have no spend allowance of their own"
                ));
            }
        }
        for name in self.max_secs.keys() {
            if !named(name) {
                issues.push(format!(
                    "routing.ladder.max_secs: no rung is named `{name}`"
                ));
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

    /// Turns one try may take on rung `rung`, from which the task can still
    /// climb `climbs_left` rungs, when its tier gives an attempt `tier_turns`
    /// (gap-3e770b). The rung's `max_turns`, when set. Otherwise each rung
    /// below the highest one the task can reach gets half the turns of the
    /// rung above it, from `tier_turns` on that highest rung, and never fewer
    /// than [`MIN_TRY_TURNS`] or more than `tier_turns`: a focused task (60
    /// turns) that can climb two rungs gets 15 on its start rung, then 30.
    /// `None` on the highest rung, where the tier's cap stands and is raised
    /// after a turn-cap stop as before.
    #[must_use]
    pub fn try_turns(&self, rung: &str, climbs_left: u32, tier_turns: u32) -> Option<u32> {
        if let Some(&turns) = self.max_turns.get(rung) {
            return Some(turns.max(1));
        }
        (climbs_left > 0).then(|| {
            tier_turns
                .checked_shr(climbs_left)
                .unwrap_or(0)
                .max(MIN_TRY_TURNS)
                .min(tier_turns)
        })
    }

    /// How long, in seconds, one try may run on rung `rung`, from which the
    /// task can still climb `climbs_left` rungs, when its attempt may run
    /// `tier_secs` (its `timeout_secs`, else `[timeouts] agent_dispatch_secs`;
    /// bug-f0007b). The rung's `max_secs`, when set, `0` for no bound.
    /// Otherwise as [`Self::try_turns`] shares out turns: each rung below the
    /// highest one the task can reach gets half the time of the rung above
    /// it, from `tier_secs` on that highest rung, and never less than
    /// [`MIN_TRY_SECS`] or more than `tier_secs`: with the default 1800 s, a
    /// task that can climb two rungs gets 450 s on its start rung, then 900 s.
    /// `None` on the highest rung, where the attempt's timeout stands.
    #[must_use]
    pub fn try_secs(&self, rung: &str, climbs_left: u32, tier_secs: u64) -> Option<u64> {
        if let Some(&secs) = self.max_secs.get(rung) {
            return (secs > 0).then_some(secs);
        }
        (climbs_left > 0).then(|| {
            tier_secs
                .checked_shr(climbs_left)
                .unwrap_or(0)
                .max(MIN_TRY_SECS)
                .min(tier_secs)
        })
    }

    /// Seconds per model call above which a rung's tries are slow
    /// (gap-3217a0): `slow_call_secs`, else [`SLOW_CALL_SECS`]; `None` when
    /// it is `0`.
    #[must_use]
    pub fn slow_call_secs(&self) -> Option<u64> {
        match self.slow_call_secs {
            Some(0) => None,
            Some(secs) => Some(secs),
            None => Some(SLOW_CALL_SECS),
        }
    }

    /// How long, in seconds, one try waits in all for a slot at a provider
    /// the run's own calls fill, at the calls at once its rate limits showed
    /// it takes, before failover may hand the try to a substitute on another
    /// rung (bug-62ae1e): `rate_limit_wait_secs`, else
    /// [`RATE_LIMIT_WAIT_SECS`].
    #[must_use]
    pub fn rate_limit_wait_secs(&self) -> u64 {
        self.rate_limit_wait_secs.unwrap_or(RATE_LIMIT_WAIT_SECS)
    }

    /// What one try may spend on rung `rung`, in USD: its `max_usd`, when
    /// that is a positive amount (gap-3e770b).
    #[must_use]
    pub fn try_usd(&self, rung: &str) -> Option<f64> {
        self.max_usd
            .get(rung)
            .copied()
            .filter(|usd| usd.is_finite() && *usd > 0.0)
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
    /// ε of the exploration draw (S02.P1-3, decision 2203): with probability
    /// ε a route the cascade router decides runs an eligible model drawn
    /// uniformly instead of its argmax, so every eligible model's logged
    /// propensity is at least ε/k. A pin (`--model`, a model hint) is never
    /// explored, and a ladder rung only among its own models, when it names
    /// [`LadderRung::alternates`] (gap-51d2ca); a run with learning frozen
    /// draws on no rung. Read through [`Self::effective_explore_epsilon`],
    /// which caps it at [`MAX_EXPLORE_EPSILON`]; 0 turns exploration off.
    #[serde(default = "default_explore_epsilon")]
    pub explore_epsilon: f64,
}

/// Default `[routing] explore_epsilon` (decision 2203, D12).
pub const DEFAULT_EXPLORE_EPSILON: f64 = 0.05;

/// Hard cap of `[routing] explore_epsilon` (decision 2203, D12).
pub const MAX_EXPLORE_EPSILON: f64 = 0.10;

impl RoutingConfig {
    /// `explore_epsilon` within `[0, MAX_EXPLORE_EPSILON]`; NaN reads as 0.
    #[must_use]
    pub const fn effective_explore_epsilon(&self) -> f64 {
        if self.explore_epsilon.is_nan() {
            0.0
        } else {
            self.explore_epsilon.clamp(0.0, MAX_EXPLORE_EPSILON)
        }
    }
}

const fn default_explore_epsilon() -> f64 {
    DEFAULT_EXPLORE_EPSILON
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
            explore_epsilon: default_explore_epsilon(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decision 2203: exploration is on at ε = 0.05 by default, and a
    /// configured ε is capped at 0.10.
    #[test]
    fn explore_epsilon_defaults_and_caps() {
        let epsilon = RoutingConfig::default().effective_explore_epsilon();
        assert!((epsilon - DEFAULT_EXPLORE_EPSILON).abs() < f64::EPSILON);
        let parsed: RoutingConfig = toml::from_str("explore_epsilon = 0.5").expect("parse");
        let capped = parsed.effective_explore_epsilon();
        assert!((capped - MAX_EXPLORE_EPSILON).abs() < f64::EPSILON);
        let off: RoutingConfig = toml::from_str("explore_epsilon = 0.0").expect("parse");
        assert!(off.effective_explore_epsilon().abs() < f64::EPSILON);
    }

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

    /// gap-ff2c38: when no rung at or above a task's start rung can run, as
    /// in a chat run that drops the CLI rung at the top, the task starts one
    /// runnable rung below the highest, so it can still climb once. A start
    /// that can run is unchanged.
    #[test]
    fn an_unreachable_start_keeps_one_climb() {
        let ladder = LadderConfig::default();
        let name = |resolved: &ResolvedLadder<'_>| resolved.start_rung().name.clone();
        let highest = |resolved: &ResolvedLadder<'_>| {
            let index = *resolved.usable.last().expect("a runnable rung");
            resolved.rungs[index].name.clone()
        };
        let no_top = |model: &str| model != "claude-sonnet-4-6";
        let architectural = ladder
            .resolve("implementer", TaskTier::Architectural, None, no_top)
            .expect("ladder");
        assert_eq!(name(&architectural), "mid");
        assert_eq!(highest(&architectural), "strong");
        assert!(architectural.start_unreachable());
        // A hint that names a rung that cannot run falls back the same way.
        let hinted = ladder
            .resolve("implementer", TaskTier::Focused, Some("top"), no_top)
            .expect("ladder");
        assert_eq!(name(&hinted), "mid");
        assert!(hinted.start_unreachable());
        // With one runnable rung, the task starts there.
        let only_cheap = |model: &str| model == "gpt-oss-120b";
        let alone = ladder
            .resolve("implementer", TaskTier::Architectural, None, only_cheap)
            .expect("ladder");
        assert_eq!(name(&alone), "cheap");
        assert!(alone.start_unreachable());

        // A start that runs, or a rung above it, changes nothing.
        for tier in TaskTier::ALL {
            let every = ladder
                .resolve("implementer", tier, None, |_| true)
                .expect("ladder");
            assert!(!every.start_unreachable(), "{tier}");
            if tier != TaskTier::Architectural {
                let without_top = ladder
                    .resolve("implementer", tier, None, no_top)
                    .expect("ladder");
                assert_eq!(without_top.start, every.start, "{tier}");
                assert!(!without_top.start_unreachable(), "{tier}");
            }
        }
        let no_cheap = |model: &str| model != "gpt-oss-120b";
        let moved_up = ladder
            .resolve("implementer", TaskTier::Mechanical, None, no_cheap)
            .expect("ladder");
        assert_eq!(name(&moved_up), "mid");
        assert!(!moved_up.start_unreachable());
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
                LadderRung::new("docs", "gpt-4o-mini"),
                LadderRung::new("careful", "claude-sonnet-4-6"),
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
        let rung = |name: &str, model: &str| LadderRung::new(name, model);
        // gap-51d2ca: an alternate that names no model, or a model the rung
        // names already, is an issue too.
        let mut top = rung("top", "c");
        top.alternates = ["d", " ", "c", "d"].map(String::from).to_vec();
        let ladder = LadderConfig {
            rungs: vec![rung("cheap", "a"), rung("cheap", "b"), rung("mid", ""), top],
            roles: vec![LadderRoleConfig::default()],
            ..LadderConfig::default()
        };
        assert_eq!(
            ladder.issues(),
            [
                "routing.ladder: rung `cheap` is defined twice",
                "routing.ladder: every rung needs a name and a model",
                "routing.ladder: rung `top` has an alternate that names no model",
                "routing.ladder: rung `top` names `c` twice",
                "routing.ladder: rung `top` names `d` twice",
                "routing.ladder.roles ``: the entry names no role",
            ]
        );
    }

    /// The config loader drops every key its schema tree lacks, so the
    /// ladder, role entries included, must survive a real load. Validation
    /// knows a rung's `alternates` (gap-51d2ca).
    #[test]
    fn ladder_survives_a_config_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("roko.toml");
        let text = r#"
schema_version = 2
config_version = 2

[routing.ladder]
enabled = false
rungs = [{ name = "only", model = "claude-sonnet", alternates = ["kimi-k2-5"] }]
start = { architectural = "only" }
max_turns = { only = 9 }
max_usd = { only = 0.25 }
max_secs = { only = 600 }

[[routing.ladder.roles]]
role = "reviewer"
start = { mechanical = "only" }
"#;
        std::fs::write(&path, text).expect("write roko.toml");
        let value: toml::Value = toml::from_str(text).expect("parse roko.toml");
        let unknown = crate::config::loader::validate_known_config_paths(&value);
        assert!(unknown.is_empty(), "{unknown:?}");

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
        assert_eq!(ladder.rungs[0].alternates, ["kimi-k2-5"]);
        let models: Vec<&str> = ladder.rungs[0].models().collect();
        assert_eq!(models, ["claude-sonnet", "kimi-k2-5"]);
        assert_eq!(ladder.start.get(TaskTier::Architectural), Some("only"));
        assert_eq!(ladder.start.get(TaskTier::Mechanical), None);
        assert_eq!(ladder.roles.len(), 1);
        assert_eq!(ladder.roles[0].role, "reviewer");
        assert_eq!(
            ladder.roles[0].start.get(TaskTier::Mechanical),
            Some("only")
        );
        assert_eq!(ladder.try_turns("only", 0, 60), Some(9));
        assert_eq!(ladder.try_usd("only"), Some(0.25));
        assert_eq!(ladder.try_secs("only", 0, 1800), Some(600));
    }

    /// gap-3e770b: a try on a rung the task can still climb from gets half
    /// the turns of the rung above it, from the tier's cap on the highest
    /// rung it can reach, at least [`MIN_TRY_TURNS`] and at most the tier's
    /// cap; the highest rung keeps the tier's cap. `max_turns` and
    /// `max_usd` set a rung's allowance outright, and a key that names no
    /// rung, no turns or no money is an issue.
    #[test]
    fn a_try_s_turns_halve_below_the_highest_rung_it_can_reach() {
        let ladder = LadderConfig::default();
        assert_eq!(ladder.try_turns("cheap", 2, 60), Some(15));
        assert_eq!(ladder.try_turns("mid", 1, 60), Some(30));
        assert_eq!(ladder.try_turns("strong", 0, 60), None);
        // Never below the floor, never above the tier's cap.
        assert_eq!(ladder.try_turns("cheap", 2, 40), Some(MIN_TRY_TURNS));
        assert_eq!(ladder.try_turns("cheap", 3, 8), Some(8));
        assert_eq!(ladder.try_turns("cheap", 40, 60), Some(MIN_TRY_TURNS));
        assert_eq!(ladder.try_usd("cheap"), None);

        let bounded: LadderConfig = toml::from_str(
            r#"
            max_turns = { cheap = 6, top = 40, gone = 3 }
            max_usd = { cheap = 0.05, mid = 0.0, nowhere = 1.0 }
            max_secs = { nowhere = 60 }
            "#,
        )
        .expect("ladder");
        assert_eq!(bounded.try_turns("cheap", 2, 60), Some(6));
        assert_eq!(bounded.try_turns("top", 0, 120), Some(40));
        assert_eq!(bounded.try_turns("mid", 1, 60), Some(30));
        assert_eq!(bounded.try_usd("cheap"), Some(0.05));
        assert_eq!(bounded.try_usd("mid"), None);
        assert_eq!(
            bounded.issues(),
            [
                "routing.ladder.max_turns: no rung is named `gone`",
                "routing.ladder.max_usd: `mid` = 0 is not a positive amount, so its tries have \
                 no spend allowance of their own",
                "routing.ladder.max_usd: no rung is named `nowhere`",
                "routing.ladder.max_secs: no rung is named `nowhere`",
            ]
        );
    }

    /// bug-f0007b: a try on a rung the task can still climb from gets half
    /// the time of the rung above it, from the attempt's time on the highest
    /// rung it can reach, at least [`MIN_TRY_SECS`] and at most the
    /// attempt's time; the highest rung keeps the attempt's timeout.
    /// `max_secs` sets a rung's time outright, and `0` takes its bound away.
    #[test]
    fn a_try_s_time_halves_below_the_highest_rung_it_can_reach() {
        let ladder = LadderConfig::default();
        assert_eq!(ladder.try_secs("mid", 2, 1800), Some(450));
        assert_eq!(ladder.try_secs("strong", 1, 1800), Some(900));
        assert_eq!(ladder.try_secs("top", 0, 1800), None);
        // Never below the floor, never above the attempt's time.
        assert_eq!(ladder.try_secs("cheap", 2, 600), Some(MIN_TRY_SECS));
        assert_eq!(ladder.try_secs("cheap", 1, 120), Some(120));
        assert_eq!(ladder.try_secs("cheap", 70, 1800), Some(MIN_TRY_SECS));

        let bounded: LadderConfig = toml::from_str(
            r#"
            max_secs = { cheap = 60, mid = 0, top = 2400 }
            "#,
        )
        .expect("ladder");
        assert_eq!(bounded.try_secs("cheap", 2, 1800), Some(60));
        assert_eq!(bounded.try_secs("mid", 1, 1800), None);
        assert_eq!(bounded.try_secs("top", 0, 1800), Some(2400));
        assert_eq!(bounded.try_secs("strong", 1, 1800), Some(900));
        assert!(bounded.issues().is_empty(), "{:?}", bounded.issues());
    }
}
