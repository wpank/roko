//! The fixed `harness_policy` holdout (S06 §4.8).
//!
//! A share of task chains, 10% by default, runs θ₀ throughout, during
//! episodes and disturbances too. It is the within-run counterfactual for
//! SASO and L-M1's default arm for M2.
//!
//! - [`HarnessHoldout::layer`] is the `harness_policy` layer for S01's
//!   [`assign`]: the unit is the chain, h is fixed (never S03's 20% → 5%
//!   schedule), and S03's all-learning-off draw ([`GLOBAL_OFF_RATE`]) sends
//!   its chains to θ₀ as well. It is a factor of its own, not a draw on the
//!   route decision (S03 §4.3 lists it for completeness).
//! - [`params_for`] is the θ a dispatch runs: θ₀ on the holdout and all-off
//!   arms, and on the learned arm the controller's θ in `on` mode (θ₀ in
//!   shadow, whose θ is only the would-be one).
//! - Holdout rows reach the controller only as its cost baseline: the EV
//!   window and the detectors leave them out (8107), and the evaluation
//!   keeps them.
//!
//! `HoldoutExperiment` (`DefaultHasher`, logging only) is not used.

use roko_core::config::harness_params::HarnessParams;
use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};

use super::controller::Controller;
use super::policy::ViabilityPolicy;
use crate::loop_audit::assign::{GLOBAL_OFF_RATE, takes_default};
use crate::telemetry::{Arm, Assignment, AssignmentUnit, AttemptKey, LayerSpec, assign};

/// The layer of M1's holdout draw, L-M1's in the loop registry.
pub const HARNESS_POLICY_LAYER: &str = "harness_policy";

/// The `harness_policy` holdout's rates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarnessHoldout {
    /// h: the share of chains, outside the all-off arm, that run θ₀.
    pub h: f64,
    /// g: the all-learning-off share, whose chains run θ₀ too.
    pub g: f64,
}

impl HarnessHoldout {
    /// A holdout of `h`, with S03's all-off rate.
    #[must_use]
    pub const fn new(h: f64) -> Self {
        Self {
            h,
            g: GLOBAL_OFF_RATE,
        }
    }

    /// The holdout of the S5 policy when there is one, which M1 only reads,
    /// and of `[homeostasis] holdout` otherwise.
    #[must_use]
    pub fn of(config: &HomeostasisConfig, policy: Option<&ViabilityPolicy>) -> Self {
        Self::new(policy.map_or(config.holdout, |policy| policy.holdout))
    }

    /// The layer for the epoch `epoch` of a run seeded `run_seed`.
    #[must_use]
    pub fn layer(&self, epoch: &str, run_seed: u64) -> LayerSpec {
        LayerSpec {
            run_seed,
            layer: HARNESS_POLICY_LAYER.to_string(),
            epoch: epoch.to_string(),
            unit: AssignmentUnit::Chain,
            h: self.h,
            g: self.g,
        }
    }

    /// The chain of `key`'s assignment: every attempt of a chain gets the
    /// same arm.
    #[must_use]
    pub fn assign(&self, epoch: &str, run_seed: u64, key: &AttemptKey) -> Assignment {
        assign(&self.layer(epoch, run_seed), key)
    }
}

/// The θ a dispatch on `arm` runs: θ₀ on the holdout and all-off arms; on
/// the learned arm, the controller's θ in `on` mode and θ₀ otherwise.
#[must_use]
pub fn params_for(arm: Arm, controller: &Controller) -> HarnessParams {
    if takes_default(arm) || controller.mode() != HomeostasisMode::On {
        return controller.state().theta0.clone();
    }
    controller.theta().clone()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use roko_core::config::harness_params::HarnessLadders;
    use roko_core::config::{ProviderConfig, RokoConfig};

    use super::*;
    use crate::homeostasis::controller::Phase;
    use crate::homeostasis::detect::Baseline;
    use crate::homeostasis::ev::{DrivePolicy, EvBounds};
    use crate::homeostasis::policy::{AuditPolicy, SafetyBox, VerifyPolicy};
    use crate::homeostasis::resolution::{CostSourceMix, TaskResolution};
    use crate::telemetry::{AttemptOutcome, CostSource};

    fn resolution(index: u64, passed: bool, arm: Option<Arm>) -> TaskResolution {
        let mut cost_source_mix = CostSourceMix::default();
        cost_source_mix.add(CostSource::ProviderUsage);
        TaskResolution {
            chain_key: format!("run:plan:t{index}"),
            final_verdict: if passed {
                AttemptOutcome::Passed
            } else {
                AttemptOutcome::GateFailed
            },
            attempts: 1,
            api_equiv_usd: Some(0.05),
            cost_source_mix,
            wall_ms: Some(300_000),
            provider_errors: 0,
            conductor_restarts: 0,
            params_digest: None,
            arm,
            audited: false,
            audit_false_green: None,
            pre_instrumentation: false,
            resolved_at: i64::try_from(index * 60_000).ok(),
        }
    }

    #[test]
    fn holdout_rows_always_theta0() {
        let mut config = RokoConfig::default();
        for name in ["alpha", "beta"] {
            config
                .providers
                .insert(name.to_string(), ProviderConfig::default());
        }
        let theta0 = HarnessParams::baseline(&config);
        let ladders = HarnessLadders::from_config(&config);
        let policy = ViabilityPolicy {
            policy_version: 1,
            ev: EvBounds::s06_example(),
            drive: DrivePolicy::default(),
            tiers: BTreeMap::new(),
            verify: VerifyPolicy::default(),
            audit: AuditPolicy::default(),
            holdout: 0.10,
        };
        let settings = HomeostasisConfig {
            mode: HomeostasisMode::On,
            random_step_prob: 0.0,
            ..HomeostasisConfig::default()
        };
        let baseline = Baseline {
            pass_rate: 0.80,
            usd_per_resolution: 0.05,
            wall_ms: 300_000.0,
        };
        let mut controller = Controller::new(
            &settings,
            policy.clone(),
            theta0.clone(),
            ladders.clone(),
            baseline,
            7,
        );

        // A breach opens an episode whose first move takes θ off θ₀.
        for index in 1..=24 {
            let passed = index <= 20 && index % 5 != 1;
            controller.on_resolution(&resolution(index, passed, None));
        }
        assert_eq!(controller.phase(), Phase::Search);
        assert_ne!(controller.theta(), &theta0);

        // Holdout rows only teach the cost baseline: they open nothing and
        // never enter the window.
        let seen = controller.state().resolutions;
        for arm in [Arm::Default, Arm::GlobalOff] {
            let events = controller.on_resolution(&resolution(25, false, Some(arm)));
            assert!(events.is_empty(), "{events:?}");
        }
        assert_eq!(controller.state().resolutions, seen);
        assert_eq!(controller.state().holdout_resolutions, 2);

        // Over 10⁴ chains mid-episode: h of the chains outside the all-off
        // arm hold out, g are all-off, and every one of them runs θ₀.
        let holdout = HarnessHoldout::of(&settings, Some(&policy));
        assert_eq!(holdout, HarnessHoldout::new(0.10));
        let safety = SafetyBox::new(theta0.clone(), ladders, &policy);
        let (mut held, mut all_off, mut learned) = (0_u32, 0_u32, 0_u32);
        for index in 0..10_000 {
            let key = AttemptKey::new("gr-holdout", "plan", format!("t{index}"), 1);
            let assignment = holdout.assign("2026-10-03", 42, &key);
            assert_eq!(assignment.layer, HARNESS_POLICY_LAYER);
            let theta = params_for(assignment.arm, &controller);
            assert!(safety.check_dispatch(Some(assignment.arm), &theta).passed());
            match assignment.arm {
                Arm::Default | Arm::GlobalOff => {
                    assert_eq!(theta, theta0, "{index}");
                    if assignment.arm == Arm::Default {
                        held += 1;
                    } else {
                        all_off += 1;
                    }
                }
                Arm::Learned | Arm::Explore => {
                    assert_eq!(&theta, controller.theta());
                    learned += 1;
                }
            }
            // A retry of the chain stays in its arm.
            let retry = AttemptKey::new("gr-holdout", "plan", format!("t{index}"), 2);
            assert_eq!(holdout.assign("2026-10-03", 42, &retry).arm, assignment.arm);
        }
        let share = f64::from(held) / f64::from(held + learned);
        assert!((share - 0.10).abs() <= 0.01, "holdout share {share}");
        let off = f64::from(all_off) / 10_000.0;
        assert!((off - GLOBAL_OFF_RATE).abs() <= 0.01, "all-off share {off}");

        // In shadow mode every arm dispatches θ₀.
        controller.set_mode(HomeostasisMode::Shadow);
        assert_eq!(params_for(Arm::Learned, &controller), theta0);
    }
}
