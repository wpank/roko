//! `HarnessParams` (θ): the knob surface of M1, the ultrastable controller
//! (S06 §4.3).
//!
//! Each field is one knob, and each knob takes one notch of an ordered
//! ladder. What is not a field cannot be changed by M1: permissions, tool
//! allowlists, the sandbox, roles, credentials, models, budget ceilings,
//! authored verify steps and the conductor's thresholds have no field here.
//! B1 names `[routing.ladder]` rungs and refers to the configured providers
//! only by their position in `[providers]`; [`HarnessParams::validate`]
//! admits only the ladder's own rung names and a permutation of the
//! configured, enabled providers, so M1 can reorder providers but never add
//! or re-enable one.
//!
//! - [`HarnessParams::baseline`] builds θ₀ from the config. B1 follows
//!   decision 8101: `tier_floor` raises the ladder's start rung and
//!   `tier_cap` stops its climb, so θ₀ is `[routing.ladder] start` and the
//!   top rung, and changes nothing.
//! - [`HarnessParams::params_digest`] is `b3:` over the RFC 8785 canonical
//!   JSON of θ (D32), so it does not depend on field order, map order or the
//!   process that computed it.
//! - [`HarnessParamsHandle`] is the shared θ that decision points read;
//!   [`HarnessParamsHandle::swap`] replaces it and bumps its
//!   `policy_version` in one atomic step.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;

use super::fingerprint::canonical_json;
use super::schema::RokoConfig;
use crate::task::TaskTier;

// ── Notch ladders ─────────────────────────────────────────────────────

/// B2 `retry_delta`: retries added to an unauthored task's retry budget.
pub const RETRY_DELTA_NOTCHES: [i32; 4] = [-1, 0, 1, 2];
/// B2 `turn_cap_mult`: multiplier of the tier's turn cap.
pub const TURN_CAP_MULT_NOTCHES: [f64; 3] = [0.75, 1.0, 1.5];
/// B3 `extra_rungs`: verify-depth floor requests, from none (V0) to V4.
pub const EXTRA_RUNGS_NOTCHES: [VerifyDepth; 5] = [
    VerifyDepth::V0,
    VerifyDepth::V1,
    VerifyDepth::V2,
    VerifyDepth::V3,
    VerifyDepth::V4,
];
/// B4 `error_patterns_k`: error patterns the prompt shows.
pub const ERROR_PATTERNS_K_NOTCHES: [u32; 3] = [0, 5, 10];
/// B4 `knowledge_section`: off, then on.
pub const KNOWLEDGE_SECTION_NOTCHES: [bool; 2] = [false, true];
/// B5 `max_parallel`: agent slots. [`HarnessLadders`] keeps the ones below
/// `conductor.max_agents` and adds that value as the top notch.
pub const MAX_PARALLEL_NOTCHES: [u32; 3] = [1, 2, 4];
/// B6 `promise_min`: the promise below which a verify run is abandoned.
/// `0.0` is off, since no promise is below it.
pub const PROMISE_MIN_NOTCHES: [f64; 4] = [0.0, 0.1, 0.2, 0.3];
/// B6 `promise_consecutive`: low-promise readings in a row before a verify
/// run is abandoned.
pub const PROMISE_CONSECUTIVE_NOTCHES: [u32; 2] = [3, 2];
/// B7 `audit_boost`: multiple of S5's audit rate.
pub const AUDIT_BOOST_NOTCHES: [u32; 3] = [1, 2, 4];
/// B8 `task_budget_scale`: share of the S5 per-task budget ceiling.
pub const TASK_BUDGET_SCALE_NOTCHES: [f64; 3] = [1.0, 0.75, 0.5];

/// A verify-depth floor request on S05 §4.6's scale (B3). S05's ladder is
/// the single writer of verify depth and applies the higher of its own level
/// and this floor, so a floor can add checks but never remove one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VerifyDepth {
    /// No floor: the authored verify steps only.
    #[default]
    V0,
    /// Adds clippy and a tamper diff.
    V1,
    /// Adds a clean full-test re-run.
    V2,
    /// Adds hidden tests.
    V3,
    /// Adds mutation testing and an LLM review.
    V4,
}

impl VerifyDepth {
    /// `V0` to `V4`, as records write it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::V0 => "V0",
            Self::V1 => "V1",
            Self::V2 => "V2",
            Self::V3 => "V3",
            Self::V4 => "V4",
        }
    }
}

// ── Knobs ─────────────────────────────────────────────────────────────

/// A knob block (S06 §4.3). M1 changes one block per move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Block {
    /// Model tier and provider order.
    B1,
    /// Retry budget and turn cap.
    B2,
    /// Verify-depth floor requests (add-only).
    B3,
    /// Optional prompt context.
    B4,
    /// Agent slots.
    B5,
    /// Promise thresholds for abandoning verify runs.
    B6,
    /// Audit boost (add-only).
    B7,
    /// Task budget scale (decrease-only).
    B8,
}

impl Block {
    /// Every block, in order.
    pub const ALL: [Self; 8] = [
        Self::B1,
        Self::B2,
        Self::B3,
        Self::B4,
        Self::B5,
        Self::B6,
        Self::B7,
        Self::B8,
    ];

    /// `B1` to `B8`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::B1 => "B1",
            Self::B2 => "B2",
            Self::B3 => "B3",
            Self::B4 => "B4",
            Self::B5 => "B5",
            Self::B6 => "B6",
            Self::B7 => "B7",
            Self::B8 => "B8",
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A knob family: one field of [`HarnessParams`]. The B1 rungs hold one
/// [`Knob`] per tier and `provider_order` one per provider; every other
/// family is a single knob.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnobKind {
    /// B1 `tier_floor`.
    TierFloor,
    /// B1 `tier_cap`.
    TierCap,
    /// B1 `provider_order`.
    ProviderOrder,
    /// B2 `retry_delta`.
    RetryDelta,
    /// B2 `turn_cap_mult`.
    TurnCapMult,
    /// B3 `extra_rungs`.
    ExtraRungs,
    /// B4 `error_patterns_k`.
    ErrorPatternsK,
    /// B4 `knowledge_section`.
    KnowledgeSection,
    /// B5 `max_parallel`.
    MaxParallel,
    /// B6 `promise_min`.
    PromiseMin,
    /// B6 `promise_consecutive`.
    PromiseConsecutive,
    /// B7 `audit_boost`.
    AuditBoost,
    /// B8 `task_budget_scale`.
    TaskBudgetScale,
}

impl KnobKind {
    /// Every family, in block order.
    pub const ALL: [Self; 13] = [
        Self::TierFloor,
        Self::TierCap,
        Self::ProviderOrder,
        Self::RetryDelta,
        Self::TurnCapMult,
        Self::ExtraRungs,
        Self::ErrorPatternsK,
        Self::KnowledgeSection,
        Self::MaxParallel,
        Self::PromiseMin,
        Self::PromiseConsecutive,
        Self::AuditBoost,
        Self::TaskBudgetScale,
    ];

    /// The [`HarnessParams`] field of this family.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::TierFloor => "tier_floor",
            Self::TierCap => "tier_cap",
            Self::ProviderOrder => "provider_order",
            Self::RetryDelta => "retry_delta",
            Self::TurnCapMult => "turn_cap_mult",
            Self::ExtraRungs => "extra_rungs",
            Self::ErrorPatternsK => "error_patterns_k",
            Self::KnowledgeSection => "knowledge_section",
            Self::MaxParallel => "max_parallel",
            Self::PromiseMin => "promise_min",
            Self::PromiseConsecutive => "promise_consecutive",
            Self::AuditBoost => "audit_boost",
            Self::TaskBudgetScale => "task_budget_scale",
        }
    }

    /// The block this family belongs to.
    #[must_use]
    pub const fn block(self) -> Block {
        match self {
            Self::TierFloor | Self::TierCap | Self::ProviderOrder => Block::B1,
            Self::RetryDelta | Self::TurnCapMult => Block::B2,
            Self::ExtraRungs => Block::B3,
            Self::ErrorPatternsK | Self::KnowledgeSection => Block::B4,
            Self::MaxParallel => Block::B5,
            Self::PromiseMin | Self::PromiseConsecutive => Block::B6,
            Self::AuditBoost => Block::B7,
            Self::TaskBudgetScale => Block::B8,
        }
    }

    fn from_field(field: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.field() == field)
    }
}

/// One knob of [`HarnessParams`]. Records name it `field` or `field.key`:
/// `retry_delta`, `tier_floor.focused`, `provider_order.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Knob {
    /// B1: the rung the tier's tasks start on.
    TierFloor(TaskTier),
    /// B1: the highest rung the tier's tasks may climb to.
    TierCap(TaskTier),
    /// B1: the place in the failover order of the provider at this index of
    /// [`HarnessLadders::providers`]. One notch is one adjacent swap.
    ProviderRank(usize),
    /// B2 `retry_delta`.
    RetryDelta,
    /// B2 `turn_cap_mult`.
    TurnCapMult,
    /// B3 `extra_rungs`.
    ExtraRungs,
    /// B4 `error_patterns_k`.
    ErrorPatternsK,
    /// B4 `knowledge_section`.
    KnowledgeSection,
    /// B5 `max_parallel`.
    MaxParallel,
    /// B6 `promise_min`.
    PromiseMin,
    /// B6 `promise_consecutive`.
    PromiseConsecutive,
    /// B7 `audit_boost`.
    AuditBoost,
    /// B8 `task_budget_scale`.
    TaskBudgetScale,
}

impl Knob {
    /// The knobs that are a whole field: all but the B1 rungs and ranks.
    pub const SCALARS: [Self; 10] = [
        Self::RetryDelta,
        Self::TurnCapMult,
        Self::ExtraRungs,
        Self::ErrorPatternsK,
        Self::KnowledgeSection,
        Self::MaxParallel,
        Self::PromiseMin,
        Self::PromiseConsecutive,
        Self::AuditBoost,
        Self::TaskBudgetScale,
    ];

    /// The family of this knob.
    #[must_use]
    pub const fn kind(self) -> KnobKind {
        match self {
            Self::TierFloor(_) => KnobKind::TierFloor,
            Self::TierCap(_) => KnobKind::TierCap,
            Self::ProviderRank(_) => KnobKind::ProviderOrder,
            Self::RetryDelta => KnobKind::RetryDelta,
            Self::TurnCapMult => KnobKind::TurnCapMult,
            Self::ExtraRungs => KnobKind::ExtraRungs,
            Self::ErrorPatternsK => KnobKind::ErrorPatternsK,
            Self::KnowledgeSection => KnobKind::KnowledgeSection,
            Self::MaxParallel => KnobKind::MaxParallel,
            Self::PromiseMin => KnobKind::PromiseMin,
            Self::PromiseConsecutive => KnobKind::PromiseConsecutive,
            Self::AuditBoost => KnobKind::AuditBoost,
            Self::TaskBudgetScale => KnobKind::TaskBudgetScale,
        }
    }

    /// The block this knob belongs to.
    #[must_use]
    pub const fn block(self) -> Block {
        self.kind().block()
    }
}

impl fmt::Display for Knob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let field = self.kind().field();
        match self {
            Self::TierFloor(tier) | Self::TierCap(tier) => write!(f, "{field}.{tier}"),
            Self::ProviderRank(index) => write!(f, "{field}.{index}"),
            _ => f.write_str(field),
        }
    }
}

impl FromStr for Knob {
    type Err = HarnessParamsError;

    /// Parse a knob name. A tier key may be any label `TaskTier::parse`
    /// accepts; [`Knob`]'s `Display` writes the canonical one.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let unknown = || HarnessParamsError::UnknownKnob {
            knob: name.to_string(),
        };
        let (field, key) = match name.split_once('.') {
            Some((field, key)) => (field, Some(key)),
            None => (name, None),
        };
        let kind = KnobKind::from_field(field).ok_or_else(unknown)?;
        match (kind, key) {
            (KnobKind::TierFloor, Some(key)) => TaskTier::parse(key)
                .map(Self::TierFloor)
                .ok_or_else(unknown),
            (KnobKind::TierCap, Some(key)) => TaskTier::parse(key)
                .map(Self::TierCap)
                .ok_or_else(unknown),
            (KnobKind::ProviderOrder, Some(key)) => key
                .parse()
                .map(Self::ProviderRank)
                .map_err(|_| unknown()),
            (_, Some(_))
            | (KnobKind::TierFloor | KnobKind::TierCap | KnobKind::ProviderOrder, None) => {
                Err(unknown())
            }
            (kind, None) => Self::SCALARS
                .into_iter()
                .find(|knob| knob.kind() == kind)
                .ok_or_else(unknown),
        }
    }
}

impl Serialize for Knob {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Knob {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        name.parse().map_err(serde::de::Error::custom)
    }
}

/// One notch along a knob's ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// To the next notch: a stronger rung, more retries, a deeper
    /// verification floor, a smaller task budget. For a provider rank, one
    /// place later in the failover order.
    Up,
    /// To the previous notch.
    Down,
}

/// Why a θ or a move on it is not admissible.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HarnessParamsError {
    /// No such knob: an unknown name, a tier θ holds no rung for, or a
    /// provider index outside the ladders.
    #[error("unknown knob `{knob}`")]
    UnknownKnob {
        /// The knob's name.
        knob: String,
    },
    /// A knob's value is not one of its ladder's notches.
    #[error("{knob} = {value} is not a notch of its ladder {ladder}")]
    OffLadder {
        /// The knob's name.
        knob: String,
        /// Its value, as JSON.
        value: String,
        /// Its ladder, as a JSON array.
        ladder: String,
    },
    /// A notch index past the end of the knob's ladder.
    #[error("{knob} has {count} notches, so it has no notch {notch}")]
    NoSuchNotch {
        /// The knob's name.
        knob: String,
        /// The notch asked for.
        notch: usize,
        /// How many notches the ladder has.
        count: usize,
    },
    /// A step past either end of the knob's ladder.
    #[error("{knob} cannot step {step:?}: it is at the end of its ladder")]
    EndOfLadder {
        /// The knob's name.
        knob: String,
        /// The step asked for.
        step: Step,
    },
    /// `provider_order` is not a permutation of the configured providers.
    #[error("provider_order {order:?} does not permute the {providers} configured providers")]
    NotAPermutation {
        /// The order θ holds.
        order: Vec<usize>,
        /// How many providers the ladders hold.
        providers: usize,
    },
    /// A tier's cap is below its floor.
    #[error("tier_cap.{tier} ({cap}) is below tier_floor.{tier} ({floor})")]
    CapBelowFloor {
        /// The tier.
        tier: TaskTier,
        /// Its floor rung.
        floor: String,
        /// Its cap rung.
        cap: String,
    },
}

// ── Ladders from the config ───────────────────────────────────────────

/// The notch ladders that come from the config (B1 and B5). The others are
/// the `*_NOTCHES` constants.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HarnessLadders {
    /// `[routing.ladder] rungs` by name, cheapest first: the ladder of
    /// `tier_floor` and `tier_cap`. Empty while the ladder is off.
    pub rungs: Vec<String>,
    /// The configured providers that `agent.disabled_providers` does not
    /// name, in `[providers]` order: the set `provider_order` permutes.
    pub providers: Vec<String>,
    /// B5's notches: those of [`MAX_PARALLEL_NOTCHES`] below
    /// `conductor.max_agents`, then `conductor.max_agents` itself.
    pub max_parallel: Vec<u32>,
}

impl HarnessLadders {
    /// The ladders `config` defines.
    #[must_use]
    pub fn from_config(config: &RokoConfig) -> Self {
        let ladder = &config.routing.ladder;
        let rungs = if ladder.enabled {
            ladder.rungs.iter().map(|rung| rung.name.clone()).collect()
        } else {
            Vec::new()
        };
        let disabled = &config.agent.disabled_providers;
        let providers = config
            .providers
            .keys()
            .filter(|name| !disabled.contains(name))
            .cloned()
            .collect();
        let cap = u32::try_from(config.conductor.max_agents)
            .unwrap_or(u32::MAX)
            .max(1);
        let mut max_parallel: Vec<u32> = MAX_PARALLEL_NOTCHES
            .into_iter()
            .filter(|&slots| slots < cap)
            .collect();
        max_parallel.push(cap);
        Self {
            rungs,
            providers,
            max_parallel,
        }
    }

    /// `b3:` digest of these ladders, so a persisted θ can be checked
    /// against the ladders it was valid for.
    #[must_use]
    pub fn digest(&self) -> String {
        digest_of(self)
    }
}

// ── θ ─────────────────────────────────────────────────────────────────

/// θ: one notch of every knob (S06 §4.3). Every field is required except
/// the B1 maps and `provider_order`, which are empty when the config has no
/// ladder or no provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessParams {
    /// B2: retries added to an unauthored task's retry budget, clamped at
    /// dispatch to the adaptive bounds. An authored `max_retries` never
    /// changes (8101). θ₀: 0.
    pub retry_delta: i32,
    /// B2: multiplier of the tier's turn cap. θ₀: 1.0.
    pub turn_cap_mult: f64,
    /// B3: the verify-depth floor M1 requests from S05's ladder. θ₀: V0.
    pub extra_rungs: VerifyDepth,
    /// B4: error patterns the prompt shows. θ₀: 5, today's constant.
    pub error_patterns_k: u32,
    /// B4: whether the prompt carries the knowledge section. θ₀: on, as
    /// dispatch does today.
    pub knowledge_section: bool,
    /// B5: agent slots, at most `conductor.max_agents`. θ₀: that value.
    pub max_parallel: u32,
    /// B6: the promise below which a verify run is abandoned (`0.0` is
    /// off). θ₀: 0.2, `PromiseTracker`'s default.
    pub promise_min: f64,
    /// B6: low-promise readings in a row before a verify run is abandoned.
    /// θ₀: 2.
    pub promise_consecutive: u32,
    /// B7: multiple of S5's audit rate (add-only). θ₀: 1.
    pub audit_boost: u32,
    /// B8: share of the S5 per-task budget ceiling (decrease-only). θ₀: 1.0.
    pub task_budget_scale: f64,
    /// B1: the failover order, as indices into
    /// [`HarnessLadders::providers`]. θ₀: the config's order.
    #[serde(default)]
    pub provider_order: Vec<usize>,
    /// B1: the rung each tier's tasks start on. θ₀: `[routing.ladder]
    /// start`.
    #[serde(default)]
    pub tier_floor: BTreeMap<TaskTier, String>,
    /// B1: the highest rung each tier's tasks may climb to. θ₀: the top
    /// rung.
    #[serde(default)]
    pub tier_cap: BTreeMap<TaskTier, String>,
}

impl HarnessParams {
    /// θ₀ of `config`: the parameters dispatch uses when M1 is off, in
    /// shadow mode and on the holdout arm.
    #[must_use]
    pub fn baseline(config: &RokoConfig) -> Self {
        let ladders = HarnessLadders::from_config(config);
        let mut tier_floor = BTreeMap::new();
        let mut tier_cap = BTreeMap::new();
        if let Some(top) = ladders.rungs.last() {
            for tier in TaskTier::ALL {
                // The role-less ladder's own start rung, with every rung
                // counted as runnable: θ₀ moves nothing (8101).
                let resolved = config.routing.ladder.resolve("", tier, None, |_| true);
                if let Some(resolved) = resolved {
                    tier_floor.insert(tier, resolved.start_rung().name.clone());
                    tier_cap.insert(tier, top.clone());
                }
            }
        }
        Self {
            retry_delta: 0,
            turn_cap_mult: 1.0,
            extra_rungs: VerifyDepth::V0,
            error_patterns_k: 5,
            knowledge_section: true,
            max_parallel: ladders.max_parallel.last().copied().unwrap_or(1),
            promise_min: 0.2,
            promise_consecutive: 2,
            audit_boost: 1,
            task_budget_scale: 1.0,
            provider_order: (0..ladders.providers.len()).collect(),
            tier_floor,
            tier_cap,
        }
    }

    /// Every knob θ holds: the B1 rungs of its tiers, one rank per
    /// provider, then [`Knob::SCALARS`].
    #[must_use]
    pub fn knobs(&self) -> Vec<Knob> {
        let floors = self.tier_floor.keys().map(|&tier| Knob::TierFloor(tier));
        let caps = self.tier_cap.keys().map(|&tier| Knob::TierCap(tier));
        let ranks = (0..self.provider_order.len()).map(Knob::ProviderRank);
        floors.chain(caps).chain(ranks).chain(Knob::SCALARS).collect()
    }

    /// The value of `knob` as records write it: a rung name, a provider's
    /// place in the failover order (0 is first), a number, a bool or a
    /// `V0`..`V4` label. `None` when θ does not hold the knob.
    #[must_use]
    pub fn value(&self, knob: Knob) -> Option<Value> {
        let value = match knob {
            Knob::TierFloor(tier) => Value::from(self.tier_floor.get(&tier)?.as_str()),
            Knob::TierCap(tier) => Value::from(self.tier_cap.get(&tier)?.as_str()),
            Knob::ProviderRank(index) => {
                Value::from(self.provider_order.iter().position(|&p| p == index)?)
            }
            Knob::RetryDelta => Value::from(self.retry_delta),
            Knob::TurnCapMult => Value::from(self.turn_cap_mult),
            Knob::ExtraRungs => Value::from(self.extra_rungs.label()),
            Knob::ErrorPatternsK => Value::from(self.error_patterns_k),
            Knob::KnowledgeSection => Value::from(self.knowledge_section),
            Knob::MaxParallel => Value::from(self.max_parallel),
            Knob::PromiseMin => Value::from(self.promise_min),
            Knob::PromiseConsecutive => Value::from(self.promise_consecutive),
            Knob::AuditBoost => Value::from(self.audit_boost),
            Knob::TaskBudgetScale => Value::from(self.task_budget_scale),
        };
        Some(value)
    }

    /// The ordered notches of `knob`, in the form [`Self::value`] writes.
    #[must_use]
    pub fn ladder(knob: Knob, ladders: &HarnessLadders) -> Vec<Value> {
        match knob {
            Knob::TierFloor(_) | Knob::TierCap(_) => ladders
                .rungs
                .iter()
                .map(|name| Value::from(name.as_str()))
                .collect(),
            Knob::ProviderRank(_) => (0..ladders.providers.len()).map(Value::from).collect(),
            Knob::RetryDelta => values(&RETRY_DELTA_NOTCHES),
            Knob::TurnCapMult => values(&TURN_CAP_MULT_NOTCHES),
            Knob::ExtraRungs => EXTRA_RUNGS_NOTCHES
                .iter()
                .map(|depth| Value::from(depth.label()))
                .collect(),
            Knob::ErrorPatternsK => values(&ERROR_PATTERNS_K_NOTCHES),
            Knob::KnowledgeSection => values(&KNOWLEDGE_SECTION_NOTCHES),
            Knob::MaxParallel => values(&ladders.max_parallel),
            Knob::PromiseMin => values(&PROMISE_MIN_NOTCHES),
            Knob::PromiseConsecutive => values(&PROMISE_CONSECUTIVE_NOTCHES),
            Knob::AuditBoost => values(&AUDIT_BOOST_NOTCHES),
            Knob::TaskBudgetScale => values(&TASK_BUDGET_SCALE_NOTCHES),
        }
    }

    /// How many notches `knob`'s ladder has.
    #[must_use]
    pub fn notch_count(knob: Knob, ladders: &HarnessLadders) -> usize {
        Self::ladder(knob, ladders).len()
    }

    /// The index of `knob`'s value on its ladder.
    ///
    /// # Errors
    ///
    /// [`HarnessParamsError::UnknownKnob`] when θ or the ladders lack the
    /// knob, and [`HarnessParamsError::OffLadder`] when its value is not a
    /// notch.
    pub fn notch(&self, knob: Knob, ladders: &HarnessLadders) -> Result<usize, HarnessParamsError> {
        let unknown = || HarnessParamsError::UnknownKnob {
            knob: knob.to_string(),
        };
        if let Knob::ProviderRank(index) = knob
            && index >= ladders.providers.len()
        {
            return Err(unknown());
        }
        let value = self.value(knob).ok_or_else(unknown)?;
        let ladder = Self::ladder(knob, ladders);
        match ladder.iter().position(|notch| *notch == value) {
            Some(notch) => Ok(notch),
            None => Err(HarnessParamsError::OffLadder {
                knob: knob.to_string(),
                value: value.to_string(),
                ladder: Value::Array(ladder).to_string(),
            }),
        }
    }

    /// θ with `knob` set to the notch at index `notch` of its ladder. For a
    /// provider rank the provider moves to that place and the others keep
    /// their order. Only `knob` is checked; [`Self::validate`] checks the
    /// whole θ.
    ///
    /// # Errors
    ///
    /// [`HarnessParamsError::NoSuchNotch`] past the end of the ladder, and
    /// [`HarnessParamsError::UnknownKnob`] when θ does not hold the knob.
    pub fn with_notch(
        &self,
        knob: Knob,
        notch: usize,
        ladders: &HarnessLadders,
    ) -> Result<Self, HarnessParamsError> {
        let count = Self::notch_count(knob, ladders);
        if notch >= count {
            return Err(HarnessParamsError::NoSuchNotch {
                knob: knob.to_string(),
                notch,
                count,
            });
        }
        let mut next = self.clone();
        match knob {
            Knob::TierFloor(tier) => {
                set_rung(&mut next.tier_floor, knob, tier, &ladders.rungs[notch])?
            }
            Knob::TierCap(tier) => set_rung(&mut next.tier_cap, knob, tier, &ladders.rungs[notch])?,
            Knob::ProviderRank(index) => {
                move_provider(&mut next.provider_order, knob, index, notch)?
            }
            Knob::RetryDelta => next.retry_delta = RETRY_DELTA_NOTCHES[notch],
            Knob::TurnCapMult => next.turn_cap_mult = TURN_CAP_MULT_NOTCHES[notch],
            Knob::ExtraRungs => next.extra_rungs = EXTRA_RUNGS_NOTCHES[notch],
            Knob::ErrorPatternsK => next.error_patterns_k = ERROR_PATTERNS_K_NOTCHES[notch],
            Knob::KnowledgeSection => next.knowledge_section = KNOWLEDGE_SECTION_NOTCHES[notch],
            Knob::MaxParallel => next.max_parallel = ladders.max_parallel[notch],
            Knob::PromiseMin => next.promise_min = PROMISE_MIN_NOTCHES[notch],
            Knob::PromiseConsecutive => {
                next.promise_consecutive = PROMISE_CONSECUTIVE_NOTCHES[notch];
            }
            Knob::AuditBoost => next.audit_boost = AUDIT_BOOST_NOTCHES[notch],
            Knob::TaskBudgetScale => next.task_budget_scale = TASK_BUDGET_SCALE_NOTCHES[notch],
        }
        Ok(next)
    }

    /// θ with `knob` moved one notch.
    ///
    /// # Errors
    ///
    /// [`HarnessParamsError::EndOfLadder`] at either end of the ladder, and
    /// the errors of [`Self::notch`].
    pub fn step(
        &self,
        knob: Knob,
        step: Step,
        ladders: &HarnessLadders,
    ) -> Result<Self, HarnessParamsError> {
        let notch = self.notch(knob, ladders)?;
        let count = Self::notch_count(knob, ladders);
        let target = match step {
            Step::Up => Some(notch + 1).filter(|&target| target < count),
            Step::Down => notch.checked_sub(1),
        };
        let target = target.ok_or_else(|| HarnessParamsError::EndOfLadder {
            knob: knob.to_string(),
            step,
        })?;
        self.with_notch(knob, target, ladders)
    }

    /// Check that every knob sits on a notch of its ladder, that
    /// `provider_order` is a permutation of the configured providers, and
    /// that no tier's cap is below its floor.
    ///
    /// # Errors
    ///
    /// The first [`HarnessParamsError`] found.
    pub fn validate(&self, ladders: &HarnessLadders) -> Result<(), HarnessParamsError> {
        let mut order = self.provider_order.clone();
        order.sort_unstable();
        if !order.into_iter().eq(0..ladders.providers.len()) {
            return Err(HarnessParamsError::NotAPermutation {
                order: self.provider_order.clone(),
                providers: ladders.providers.len(),
            });
        }
        for knob in self.knobs() {
            self.notch(knob, ladders)?;
        }
        for (&tier, cap) in &self.tier_cap {
            let Some(floor) = self.tier_floor.get(&tier) else {
                continue;
            };
            let cap_notch = self.notch(Knob::TierCap(tier), ladders)?;
            if cap_notch < self.notch(Knob::TierFloor(tier), ladders)? {
                return Err(HarnessParamsError::CapBelowFloor {
                    tier,
                    floor: floor.clone(),
                    cap: cap.clone(),
                });
            }
        }
        Ok(())
    }

    /// The knobs whose values differ between `self` and `other`.
    #[must_use]
    pub fn changed_knobs(&self, other: &Self) -> Vec<Knob> {
        let knobs: BTreeSet<Knob> = self.knobs().into_iter().chain(other.knobs()).collect();
        knobs
            .into_iter()
            .filter(|&knob| self.value(knob) != other.value(knob))
            .collect()
    }

    /// The blocks of [`Self::changed_knobs`].
    #[must_use]
    pub fn changed_blocks(&self, other: &Self) -> BTreeSet<Block> {
        self.changed_knobs(other)
            .into_iter()
            .map(Knob::block)
            .collect()
    }

    /// `"b3:"` plus the BLAKE3 hex digest of θ's RFC 8785 canonical JSON
    /// (D32): equal θs have equal digests whatever their field order, and a
    /// restart computes the same digest.
    #[must_use]
    pub fn params_digest(&self) -> String {
        digest_of(self)
    }
}

fn values<T: Copy + Into<Value>>(notches: &[T]) -> Vec<Value> {
    notches.iter().map(|&notch| notch.into()).collect()
}

fn digest_of<T: Serialize>(value: &T) -> String {
    let json = serde_json::to_value(value).expect("plain data serializes to JSON");
    format!("b3:{}", blake3::hash(canonical_json(&json).as_bytes()).to_hex())
}

fn set_rung(
    rungs: &mut BTreeMap<TaskTier, String>,
    knob: Knob,
    tier: TaskTier,
    name: &str,
) -> Result<(), HarnessParamsError> {
    let slot = rungs
        .get_mut(&tier)
        .ok_or_else(|| HarnessParamsError::UnknownKnob {
            knob: knob.to_string(),
        })?;
    *slot = name.to_string();
    Ok(())
}

fn move_provider(
    order: &mut Vec<usize>,
    knob: Knob,
    index: usize,
    place: usize,
) -> Result<(), HarnessParamsError> {
    let unknown = || HarnessParamsError::UnknownKnob {
        knob: knob.to_string(),
    };
    let current = order.iter().position(|&p| p == index).ok_or_else(unknown)?;
    let provider = order.remove(current);
    if place > order.len() {
        order.insert(current, provider);
        return Err(unknown());
    }
    order.insert(place, provider);
    Ok(())
}

// ── The shared handle ─────────────────────────────────────────────────

/// θ together with the version its swap gave it.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionedParams {
    /// 0 for the handle's first θ; every [`HarnessParamsHandle::swap`] adds
    /// one. It counts θ's versions: the S5 viability policy file has its own
    /// `policy_version`, which M1 only reads.
    pub policy_version: u64,
    /// θ.
    pub params: HarnessParams,
    /// [`HarnessParams::params_digest`] of `params`, computed at the swap.
    pub params_digest: String,
    /// Why θ took this value, such as `theta0` or `homeostat:ep-0007/ch-0019`.
    pub reason: String,
}

/// The θ every decision point reads (S06 §4.1): an atomically swapped
/// [`VersionedParams`]. Clones share one θ, and a reader never sees a θ with
/// another θ's version or digest.
#[derive(Debug, Clone)]
pub struct HarnessParamsHandle {
    current: Arc<ArcSwap<VersionedParams>>,
}

impl HarnessParamsHandle {
    /// The reason of a handle's first θ.
    pub const INITIAL_REASON: &'static str = "theta0";

    /// A handle holding `params` at version 0.
    #[must_use]
    pub fn new(params: HarnessParams) -> Self {
        let params_digest = params.params_digest();
        let first = VersionedParams {
            policy_version: 0,
            params,
            params_digest,
            reason: Self::INITIAL_REASON.to_string(),
        };
        Self {
            current: Arc::new(ArcSwap::from_pointee(first)),
        }
    }

    /// The current θ with its version.
    #[must_use]
    pub fn load(&self) -> Arc<VersionedParams> {
        self.current.load_full()
    }

    /// The current θ's version.
    #[must_use]
    pub fn policy_version(&self) -> u64 {
        self.current.load().policy_version
    }

    /// Replace θ with `params` and return its new version, one more than
    /// the version it replaced. Concurrent swaps each get their own version.
    pub fn swap(&self, params: HarnessParams, reason: impl Into<String>) -> u64 {
        let params_digest = params.params_digest();
        let reason = reason.into();
        let previous = self.current.rcu(|current| VersionedParams {
            policy_version: current.policy_version + 1,
            params: params.clone(),
            params_digest: params_digest.clone(),
            reason: reason.clone(),
        });
        previous.policy_version + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::ProviderConfig;

    /// A θ off θ₀ on every block, with its fields in declaration order.
    const THETA_JSON: &str = r#"{
        "retry_delta": 1,
        "turn_cap_mult": 1.5,
        "extra_rungs": "V2",
        "error_patterns_k": 10,
        "knowledge_section": false,
        "max_parallel": 2,
        "promise_min": 0.1,
        "promise_consecutive": 3,
        "audit_boost": 2,
        "task_budget_scale": 0.75,
        "provider_order": [1, 0],
        "tier_floor": {"mechanical": "cheap", "focused": "mid"},
        "tier_cap": {"mechanical": "strong", "focused": "strong"}
    }"#;

    /// The same θ with its fields and map keys in reverse order.
    const THETA_JSON_REVERSED: &str = r#"{
        "tier_cap": {"focused": "strong", "mechanical": "strong"},
        "tier_floor": {"focused": "mid", "mechanical": "cheap"},
        "provider_order": [1, 0],
        "task_budget_scale": 0.75,
        "audit_boost": 2,
        "promise_consecutive": 3,
        "promise_min": 0.1,
        "max_parallel": 2,
        "knowledge_section": false,
        "error_patterns_k": 10,
        "extra_rungs": "V2",
        "turn_cap_mult": 1.5,
        "retry_delta": 1
    }"#;

    /// `b3:` of the RFC 8785 form of [`THETA_JSON`], computed outside Rust,
    /// so a restart, a reordering or a new map type cannot move it.
    const THETA_DIGEST: &str =
        "b3:6c1ae34d840e8946493003318e9f2e48d2e05daf09df14866237a0554d3bb448";

    /// The default config plus three providers, the third disabled.
    fn config_with_providers() -> RokoConfig {
        let mut config = RokoConfig::default();
        for name in ["alpha", "beta", "gamma"] {
            config
                .providers
                .insert(name.to_string(), ProviderConfig::default());
        }
        config.agent.disabled_providers = vec!["gamma".to_string()];
        config
    }

    #[test]
    fn ladders_roundtrip_and_digest_stable() {
        let config = config_with_providers();
        let ladders = HarnessLadders::from_config(&config);
        assert_eq!(ladders.rungs, ["cheap", "mid", "strong", "top"]);
        assert_eq!(ladders.providers, ["alpha", "beta"]);
        assert_eq!(ladders.max_parallel, [1, 2, 4, 8]);
        let theta0 = HarnessParams::baseline(&config);
        theta0.validate(&ladders).expect("θ₀ sits on every ladder");

        // TOML and JSON round-trips are lossless, for θ and the ladders.
        let toml_text = toml::to_string(&theta0).expect("θ₀ to TOML");
        let from_toml: HarnessParams = toml::from_str(&toml_text).expect("θ₀ from TOML");
        assert_eq!(from_toml, theta0, "{toml_text}");
        let json_text = serde_json::to_string(&theta0).expect("θ₀ to JSON");
        let from_json: HarnessParams = serde_json::from_str(&json_text).expect("θ₀ from JSON");
        assert_eq!(from_json, theta0);
        let ladders_text = toml::to_string(&ladders).expect("ladders to TOML");
        let ladders_back: HarnessLadders =
            toml::from_str(&ladders_text).expect("ladders from TOML");
        assert_eq!(ladders_back, ladders);

        // Every notch of every knob is reachable, reads back as itself, and
        // survives both round-trips with its digest.
        for knob in theta0.knobs() {
            assert_eq!(knob.to_string().parse::<Knob>(), Ok(knob));
            for notch in 0..HarnessParams::notch_count(knob, &ladders) {
                let moved = theta0
                    .with_notch(knob, notch, &ladders)
                    .expect("a notch on the ladder");
                assert_eq!(moved.notch(knob, &ladders), Ok(notch), "{knob}");
                let text = toml::to_string(&moved).expect("θ to TOML");
                let back: HarnessParams = toml::from_str(&text).expect("θ from TOML");
                assert_eq!(back, moved, "{knob} notch {notch}");
                let back: HarnessParams =
                    serde_json::from_str(&serde_json::to_string(&moved).expect("θ to JSON"))
                        .expect("θ from JSON");
                assert_eq!(back.params_digest(), moved.params_digest());
            }
        }

        // The digest ignores field order and map order, and is the digest
        // computed outside this process.
        let forward: HarnessParams = serde_json::from_str(THETA_JSON).expect("parse θ");
        let backward: HarnessParams =
            serde_json::from_str(THETA_JSON_REVERSED).expect("parse reversed θ");
        assert_eq!(forward, backward);
        assert_eq!(forward.params_digest(), THETA_DIGEST);
        assert_eq!(backward.params_digest(), THETA_DIGEST);
        forward.validate(&ladders).expect("the fixture sits on every ladder");
        assert_ne!(theta0.params_digest(), THETA_DIGEST);

        // A swap bumps policy_version and carries the new θ's digest.
        let handle = HarnessParamsHandle::new(theta0.clone());
        assert_eq!(handle.policy_version(), 0);
        assert_eq!(handle.load().reason, HarnessParamsHandle::INITIAL_REASON);
        let raised = theta0
            .step(Knob::TierFloor(TaskTier::Focused), Step::Up, &ladders)
            .expect("cheap -> mid");
        assert_eq!(raised.tier_floor[&TaskTier::Focused], "mid");
        assert_eq!(handle.swap(raised.clone(), "homeostat:ep-0001/ch-0001"), 1);
        let current = handle.load();
        assert_eq!(current.policy_version, 1);
        assert_eq!(current.params, raised);
        assert_eq!(current.params_digest, raised.params_digest());
        assert_ne!(current.params_digest, theta0.params_digest());
        let shared = handle.clone();
        assert_eq!(shared.swap(theta0.clone(), "rollback"), 2);
        assert_eq!(handle.policy_version(), 2);
        assert_eq!(handle.load().params, theta0);

        // A notch outside its ladder is rejected.
        assert!(matches!(
            theta0.with_notch(Knob::TurnCapMult, 3, &ladders),
            Err(HarnessParamsError::NoSuchNotch { count: 3, .. })
        ));
        assert!(matches!(
            theta0.step(Knob::TierCap(TaskTier::Focused), Step::Up, &ladders),
            Err(HarnessParamsError::EndOfLadder { .. })
        ));
        assert!(matches!(
            theta0.step(Knob::RetryDelta, Step::Down, &ladders)
                .and_then(|theta| theta.step(Knob::RetryDelta, Step::Down, &ladders)),
            Err(HarnessParamsError::EndOfLadder { .. })
        ));
        let mut off = theta0.clone();
        off.turn_cap_mult = 2.0;
        assert!(matches!(
            off.validate(&ladders),
            Err(HarnessParamsError::OffLadder { .. })
        ));
        let mut off = theta0.clone();
        off.tier_floor
            .insert(TaskTier::Focused, "frontier".to_string());
        assert!(matches!(
            off.validate(&ladders),
            Err(HarnessParamsError::OffLadder { .. })
        ));
        let mut off = theta0.clone();
        off.max_parallel = 3;
        assert!(off.validate(&ladders).is_err());
        let mut off = theta0.clone();
        off.provider_order = vec![0, 1, 2];
        assert!(matches!(
            off.validate(&ladders),
            Err(HarnessParamsError::NotAPermutation { providers: 2, .. })
        ));
        let mut off = theta0.clone();
        off.tier_floor.insert(TaskTier::Focused, "strong".to_string());
        off.tier_cap.insert(TaskTier::Focused, "mid".to_string());
        assert!(matches!(
            off.validate(&ladders),
            Err(HarnessParamsError::CapBelowFloor { .. })
        ));
        assert!(serde_json::from_str::<HarnessParams>(
            &THETA_JSON.replace("\"retry_delta\": 1,", "\"retry_delta\": 1, \"allowed_tools\": [],")
        )
        .is_err());
    }

    #[test]
    fn baseline_follows_ladder_start_and_moves_change_one_block() {
        let config = config_with_providers();
        let ladders = HarnessLadders::from_config(&config);
        let theta0 = HarnessParams::baseline(&config);
        // D11's start rungs; every cap at the top rung.
        assert_eq!(theta0.tier_floor[&TaskTier::Mechanical], "cheap");
        assert_eq!(theta0.tier_floor[&TaskTier::Focused], "cheap");
        assert_eq!(theta0.tier_floor[&TaskTier::Integrative], "mid");
        assert_eq!(theta0.tier_floor[&TaskTier::Architectural], "top");
        assert!(theta0.tier_cap.values().all(|cap| cap == "top"));
        assert_eq!(theta0.provider_order, [0, 1]);
        assert_eq!(theta0.max_parallel, 8);
        assert_eq!(theta0.knobs().len(), 4 + 4 + 2 + Knob::SCALARS.len());

        // A provider rank moves by adjacent swaps.
        let demoted = theta0
            .step(Knob::ProviderRank(0), Step::Up, &ladders)
            .expect("alpha to second place");
        assert_eq!(demoted.provider_order, [1, 0]);
        assert_eq!(
            theta0.changed_knobs(&demoted),
            [Knob::ProviderRank(0), Knob::ProviderRank(1)]
        );
        assert_eq!(
            theta0.changed_blocks(&demoted).into_iter().collect::<Vec<_>>(),
            [Block::B1]
        );
        let budget = theta0
            .step(Knob::TaskBudgetScale, Step::Up, &ladders)
            .expect("1.0 -> 0.75");
        assert_eq!(budget.task_budget_scale, 0.75);
        assert_eq!(theta0.changed_knobs(&budget), [Knob::TaskBudgetScale]);
        assert_eq!(budget.value(Knob::TaskBudgetScale), Some(Value::from(0.75)));

        // Knob names round-trip through serde too.
        let names = serde_json::to_string(&[Knob::TierCap(TaskTier::Integrative), Knob::RetryDelta])
            .expect("knobs to JSON");
        assert_eq!(names, r#"["tier_cap.integrative","retry_delta"]"#);
        assert_eq!(
            "tier_floor.standard".parse::<Knob>(),
            Ok(Knob::TierFloor(TaskTier::Focused))
        );
        assert!("retry_delta.focused".parse::<Knob>().is_err());
        assert!("allowed_tools".parse::<Knob>().is_err());

        // With the ladder off there is no B1 rung knob.
        let mut off = RokoConfig::default();
        off.routing.ladder.enabled = false;
        let theta0 = HarnessParams::baseline(&off);
        assert!(theta0.tier_floor.is_empty() && theta0.tier_cap.is_empty());
        theta0
            .validate(&HarnessLadders::from_config(&off))
            .expect("θ₀ without a ladder");
    }
}
