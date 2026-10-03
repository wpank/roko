//! S02.P1-14's arm set: the arm of every per-chain randomised loop for one
//! task chain, drawn once over the loop-audit layers ([`super::assign`]).
//!
//! At attempt open, Graph dispatch draws the chain's [`ArmSet`]: one
//! assignment on each per-chain layer (knowledge, playbooks, sections, the
//! prompt variant and the placebo) and the chain's all-learning-off draw on
//! the `global` layer. Every draw goes through S01's one assignment function
//! with the unit key of the chain, so a retry re-derives the same arms from
//! its key and the loops' layers stay independent. The per-attempt
//! `route.explore` draw stays in the router.
//!
//! Maximize mode (`[experiments] maximize`, `roko plan run --no-holdout`)
//! withholds nothing: g = h = 0, so every loop takes its learned arm at
//! propensity 1, and decisions are still logged. Forced arms
//! (`[experiments] force_arms`) pin layers to an arm, and estimates leave
//! those chains out.

use std::collections::BTreeMap;

use roko_core::config::experiments::ExperimentsConfig;
use serde::{Deserialize, Serialize};

use super::assign::{GLOBAL_OFF_RATE, LoopLayer, takes_default};
use super::spec::{AuditState, LoopId, LoopSpec, Registry};
use crate::telemetry::{Arm, Assignment, AssignmentUnit, AttemptKey, LayerSpec, assign};

/// The loops whose arm a chain draws at attempt open (S02.P1-14).
pub const ARM_SET_LOOPS: [&str; 5] = ["L-know", "L-play", "L-sec", "L-prompt-exp", "L-placebo"];
/// The placebo's layer (S03 §4.3).
pub const PLACEBO_LAYER: &str = "placebo";
/// The condition of a run whose layers all draw.
pub const NORMAL_CONDITION: &str = "normal";
/// The condition of a maximize-mode run, which withholds nothing.
pub const MAXIMIZE_CONDITION: &str = "maximize";
/// The condition of a run with forced arms, which estimates leave out.
pub const FORCED_CONDITION: &str = "forced";

/// How a run randomises its loops (decision 4115).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ArmMode {
    /// Every per-chain layer draws its arm.
    #[default]
    Normal,
    /// Nothing is withheld: every loop takes its learned arm at propensity
    /// 1, for must-succeed runs.
    Maximize,
    /// The named layers (by layer name or loop id) take the named arms at
    /// propensity 1; the others draw as usual.
    Forced(BTreeMap<String, Arm>),
}

impl ArmMode {
    /// The mode `[experiments]` sets: maximize, else the forced arms, else
    /// normal. A forced arm that names no arm (`learned`, `default` or
    /// `global_off`) is logged and left out.
    #[must_use]
    pub fn for_config(config: &ExperimentsConfig) -> Self {
        if config.maximize {
            return Self::Maximize;
        }
        let forced: BTreeMap<String, Arm> = config
            .force_arms
            .iter()
            .filter_map(|(layer, arm)| {
                let parsed = match arm.trim() {
                    "learned" => Some(Arm::Learned),
                    "default" => Some(Arm::Default),
                    "global_off" => Some(Arm::GlobalOff),
                    _ => None,
                };
                if parsed.is_none() {
                    tracing::warn!(
                        %layer,
                        %arm,
                        "[experiments] force_arms names no arm (learned, default or global_off); \
                         ignored"
                    );
                }
                parsed.map(|arm| (layer.clone(), arm))
            })
            .collect();
        if forced.is_empty() {
            Self::Normal
        } else {
            Self::Forced(forced)
        }
    }

    /// The condition an arm set in this mode records.
    #[must_use]
    pub const fn condition_id(&self) -> &'static str {
        match self {
            Self::Normal => NORMAL_CONDITION,
            Self::Maximize => MAXIMIZE_CONDITION,
            Self::Forced(_) => FORCED_CONDITION,
        }
    }
}

/// The run-level inputs of an arm set's draws.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmDraws {
    /// The run's assignment seed (S01 `experiment.seed`).
    pub run_seed: u64,
    /// The epoch the layer keys are derived for: the UTC day, or a bench
    /// run's id.
    pub epoch: String,
    /// g, the all-learning-off rate.
    pub g: f64,
    /// Each loop's audit state; a loop not named is on probation.
    pub states: BTreeMap<LoopId, AuditState>,
    /// The holdout schedule, and a campaign's fixed rate.
    pub layers: LoopLayer,
}

impl ArmDraws {
    /// The draws of `epoch` with `run_seed`, S03's schedule and g, and every
    /// loop on probation.
    #[must_use]
    pub fn new(run_seed: u64, epoch: impl Into<String>) -> Self {
        Self {
            run_seed,
            epoch: epoch.into(),
            g: GLOBAL_OFF_RATE,
            states: BTreeMap::new(),
            layers: LoopLayer::default(),
        }
    }
}

/// One chain's arms: an assignment on each per-chain layer, by layer name,
/// and the chain's all-off draw under `global`. It serializes as the
/// `assignment` object S01's decision rows embed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmSet {
    /// The chain the arms belong to ([`AttemptKey::chain_key`]).
    pub chain_key: String,
    /// Each layer's assignment, by layer name.
    pub arms: BTreeMap<String, Assignment>,
    /// `normal`, `maximize` or `forced`; estimates leave out forced chains.
    pub condition_id: String,
}

impl ArmSet {
    /// Draw the arms of `key`'s chain on every per-chain layer of `loops`,
    /// the loop registry, in `mode`. Pure: the same chain, draws and mode
    /// always give the same set, so a retry inherits its chain's arms.
    #[must_use]
    pub fn assign(key: &AttemptKey, loops: &Registry, mode: &ArmMode, draws: &ArmDraws) -> Self {
        let maximize = *mode == ArmMode::Maximize;
        let layers = LoopLayer {
            maximize,
            ..draws.layers
        };
        let g = if maximize { 0.0 } else { draws.g };
        let global = LayerSpec {
            run_seed: draws.run_seed,
            layer: crate::telemetry::assign::GLOBAL_LAYER.to_string(),
            epoch: draws.epoch.clone(),
            unit: AssignmentUnit::Chain,
            h: 0.0,
            g,
        };
        let mut arms = BTreeMap::new();
        arms.insert(global.layer.clone(), assign(&global, key));
        for spec in ARM_SET_LOOPS.iter().filter_map(|id| loops.get(id)) {
            let state = draws.states.get(&spec.id).copied().unwrap_or_default();
            let layer = layers.layer_spec(spec, state, &draws.epoch, draws.run_seed, g);
            arms.insert(layer.layer.clone(), assign(&layer, key));
        }
        if let ArmMode::Forced(forced) = mode {
            force(&mut arms, forced, loops);
        }
        Self {
            chain_key: key.chain_key(),
            arms,
            condition_id: mode.condition_id().to_string(),
        }
    }

    /// The assignment on `layer`, if the set has that layer.
    #[must_use]
    pub fn get(&self, layer: &str) -> Option<&Assignment> {
        self.arms.get(layer)
    }

    /// Whether the loop on `layer` runs its default policy π⁰ in this chain:
    /// its default arm or the all-off arm. A layer the set lacks runs its
    /// learned policy.
    #[must_use]
    pub fn takes_default(&self, layer: &str) -> bool {
        self.get(layer)
            .is_some_and(|assignment| takes_default(assignment.arm))
    }

    /// The placebo's assignment (S03 §4.3).
    #[must_use]
    pub fn placebo(&self) -> Option<&Assignment> {
        self.get(PLACEBO_LAYER)
    }
}

/// Pin the layers `forced` names, by layer name or the id of a loop of
/// `loops`, to their arms at propensity 1.
fn force(
    arms: &mut BTreeMap<String, Assignment>,
    forced: &BTreeMap<String, Arm>,
    loops: &Registry,
) {
    let specs: Vec<&LoopSpec> = ARM_SET_LOOPS
        .iter()
        .filter_map(|id| loops.get(id))
        .collect();
    for (layer, assignment) in arms.iter_mut() {
        let by_loop = specs
            .iter()
            .filter(|spec| spec.assignment_layer().as_str() == layer.as_str())
            .find_map(|spec| forced.get(spec.id.as_str()));
        if let Some(&arm) = forced.get(layer).or(by_loop) {
            assignment.arm = arm;
            assignment.propensity = 1.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use roko_core::config::experiments::ExperimentsConfig;

    use super::{
        ArmDraws, ArmMode, ArmSet, FORCED_CONDITION, MAXIMIZE_CONDITION, NORMAL_CONDITION,
    };
    use crate::loop_audit::spec::Registry;
    use crate::telemetry::{Arm, AttemptKey};

    fn chain(index: usize) -> AttemptKey {
        AttemptKey::new("gr-arms", "plan", format!("t{index}"), 1)
    }

    /// S02.P1-14: a chain draws its arms once, so every call and every retry
    /// gets the same set; chains draw independently, and the all-off arm is
    /// the chain's on every layer; maximize mode leaves every loop on its
    /// learned arm at propensity 1; forced arms pin a layer; the set
    /// serializes as the decision rows' assignment.
    #[test]
    fn arm_set_draws_once_per_chain_and_maximize_draws_nothing() {
        const CHAINS: usize = 4_000;
        let registry = Registry::embedded().expect("embedded loop registry");
        let draws = ArmDraws::new(7, "2026-10-03");
        let normal = |key: &AttemptKey| ArmSet::assign(key, &registry, &ArmMode::Normal, &draws);

        let first = normal(&chain(0));
        assert_eq!(first.condition_id, NORMAL_CONDITION);
        let layers: Vec<&str> = first.arms.keys().map(String::as_str).collect();
        let expected = [
            "global",
            "knowledge",
            "placebo",
            "playbooks",
            "prompt_variant",
            "sections",
        ];
        assert_eq!(layers, expected);
        assert_eq!(normal(&chain(0)), first, "the same chain, the same arms");
        let retry = AttemptKey::new("gr-arms", "plan", "t0", 3);
        assert_eq!(normal(&retry), first, "a retry inherits its chain's arms");

        // Chains draw independently: outside the all-off arm, knowledge is
        // withheld at its probation h whatever the placebo drew.
        let (mut placebo_default, mut placebo_learned) = ([0_u32; 2], [0_u32; 2]);
        for index in 0..CHAINS {
            let set = normal(&chain(index));
            let off = set.get("global").expect("the all-off draw").arm == Arm::GlobalOff;
            for assignment in set.arms.values() {
                assert_eq!(assignment.arm == Arm::GlobalOff, off, "{set:?}");
            }
            if off {
                continue;
            }
            let withheld = usize::from(set.takes_default("knowledge"));
            match set.placebo().expect("the placebo").arm {
                Arm::Default => placebo_default[withheld] += 1,
                _ => placebo_learned[withheld] += 1,
            }
        }
        let rate = |counts: [u32; 2]| f64::from(counts[1]) / f64::from(counts[0] + counts[1]);
        let (with_default, with_learned) = (rate(placebo_default), rate(placebo_learned));
        assert!((0.15..0.25).contains(&with_default), "{with_default}");
        let gap = (with_default - with_learned).abs();
        assert!(gap < 0.05, "{with_default} vs {with_learned}");

        // Maximize mode withholds nothing.
        for index in 0..200 {
            let set = ArmSet::assign(&chain(index), &registry, &ArmMode::Maximize, &draws);
            assert_eq!(set.condition_id, MAXIMIZE_CONDITION);
            for assignment in set.arms.values() {
                assert_eq!(assignment.arm, Arm::Learned, "{set:?}");
                assert!((assignment.propensity - 1.0).abs() < 1e-12, "{set:?}");
            }
        }

        // A forced layer takes its arm at propensity 1; the others draw.
        let forced = ArmMode::Forced(BTreeMap::from([("L-know".to_string(), Arm::Default)]));
        let set = ArmSet::assign(&chain(0), &registry, &forced, &draws);
        assert_eq!(set.condition_id, FORCED_CONDITION);
        let knowledge = set.get("knowledge").expect("the knowledge arm");
        assert_eq!((knowledge.arm, knowledge.propensity), (Arm::Default, 1.0));
        assert_eq!(set.get("sections"), first.get("sections"));

        // `[experiments]` sets the mode; a forced arm that names no arm is
        // left out.
        let mut config = ExperimentsConfig::default();
        assert_eq!(ArmMode::for_config(&config), ArmMode::Normal);
        config
            .force_arms
            .insert("L-know".to_string(), "default".to_string());
        config
            .force_arms
            .insert("sections".to_string(), "sometimes".to_string());
        assert_eq!(ArmMode::for_config(&config), forced);
        config.maximize = true;
        assert_eq!(ArmMode::for_config(&config), ArmMode::Maximize);

        let row = serde_json::to_value(&first).expect("serialize the arm set");
        assert_eq!(row["arms"]["placebo"]["layer"], "placebo");
        let back: ArmSet = serde_json::from_value(row).expect("parse the arm set");
        assert_eq!(back, first);
    }
}
