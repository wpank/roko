//! Move priors seeded from the self-model (M3), and the feedforward pre-arm
//! (S06 §4.4, §4.5; Conant–Ashby, `conant1970every`).
//!
//! - [`M3Prior`] is the controller's [`MovePrior`] from M3: μ⁰ = −ΔD̂, the
//!   drive change a [`DrivePredictor`] predicts for the recent task mix
//!   under θ ⊕ m against θ, from P(pass), the median cost and the p90
//!   latency, with variance v_base / w_M3. The weight follows calibration
//!   ([`m3_weight`]): 1 with a rolling ECE ≤ 0.05 and no drift flag, 0.25
//!   with an ECE ≤ 0.10, and 0 otherwise, where the catalog-sign prior
//!   (±0.1) comes back exactly. A mis-calibrated model so degrades toward
//!   Ashby's random search, which still finds a viable setting when one is
//!   in the box. Priors rank the candidates the SafetyBox admitted; they
//!   never add one.
//! - [`M3Predictions`] hands the replay's A3-gated and A3-mis arms the same
//!   priors (8117).
//! - [`Prearm`] is the feedforward pre-arm, an ablation off by default
//!   (`[homeostasis] feedforward_prearm`): when M3's predicted drive for the
//!   next queued tasks rises by more than 0.2, the detectors' thresholds H
//!   are halved for 10 resolutions ([`Controller::set_detector_tuning`]).
//!
//! The self-model forecasts per routing arm (S04.T04, `self_model`); an
//! adapter that aggregates them over the queued tasks' features into a
//! [`DrivePredictor`] needs the dispatch-side task mix and the rung-to-model
//! map, which the plan-run sink has (8122).
//!
//! [`Controller::set_detector_tuning`]: super::controller::Controller::set_detector_tuning

use std::sync::Arc;

use roko_core::config::harness_params::HarnessParams;
use roko_core::config::homeostasis::HomeostasisConfig;
use serde::{Deserialize, Serialize};

use super::controller::{
    Candidate, MovePrior, MovePriorValue, PRIOR_VARIANCE, PriorSource, catalog_prior,
};
use super::detect::DetectorTuning;
use super::ev::{DrivePolicy, Estimate, Ev, EvBounds, EvEstimates, EvGap, drive};
use super::policy::ViabilityPolicy;
use super::replay::Predictions;

/// ECE at or below which M3's priors count in full (with no drift flag).
pub const ECE_FULL: f64 = 0.05;
/// ECE at or below which M3's priors count a quarter.
pub const ECE_PARTIAL: f64 = 0.10;
/// The predicted rise in drive that pre-arms the detectors.
pub const PREARM_RISE: f64 = 0.2;
/// Resolutions a pre-arm lasts.
pub const PREARM_RESOLUTIONS: u32 = 10;
/// Queued tasks the pre-arm's prediction covers.
pub const PREARM_QUEUE: usize = 10;

/// What M3 predicts for the recent task mix under a θ.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PredictedLevels {
    /// P(verified success) of a task.
    pub pass_rate: f64,
    /// The median cost of a task, USD.
    pub cost_p50_usd: f64,
    /// The 90th-percentile latency of a task, seconds.
    pub latency_p90_s: f64,
}

/// The self-model's calibration: S04's rolling ECE and drift flag.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    /// The rolling expected calibration error; `None` before there is one.
    pub ece: Option<f64>,
    /// S04 flagged drift.
    pub drift: bool,
}

/// M3's predictions for the recent task mix.
pub trait DrivePredictor: std::fmt::Debug + Send + Sync {
    /// The predicted levels under `theta`; `None` when there is no
    /// prediction.
    fn predict(&self, theta: &HarnessParams) -> Option<PredictedLevels>;

    /// The predictor's calibration now.
    fn calibration(&self) -> Calibration;
}

/// w_M3: 1 with an ECE at most [`ECE_FULL`] and no drift flag, 0.25 with
/// an ECE at most [`ECE_PARTIAL`], 0 otherwise (S06 §4.5).
#[must_use]
pub fn m3_weight(calibration: &Calibration) -> f64 {
    match calibration.ece {
        Some(ece) if ece <= ECE_FULL && !calibration.drift => 1.0,
        Some(ece) if ece <= ECE_PARTIAL => 0.25,
        _ => 0.0,
    }
}

/// The controller's move prior from M3.
#[derive(Debug, Clone)]
pub struct M3Prior {
    predictor: Arc<dyn DrivePredictor>,
    bounds: EvBounds,
    drive: DrivePolicy,
}

impl M3Prior {
    /// Priors from `predictor`, with drives under `policy`.
    #[must_use]
    pub fn new(predictor: Arc<dyn DrivePredictor>, policy: &ViabilityPolicy) -> Self {
        Self {
            predictor,
            bounds: policy.ev,
            drive: policy.drive,
        }
    }

    /// D̂: the drive of the levels predicted under `theta`, E3 unmeasured.
    #[must_use]
    pub fn predicted_drive(&self, theta: &HarnessParams) -> Option<f64> {
        let levels = self.predictor.predict(theta)?;
        let cost = if levels.pass_rate > 0.0 {
            let usd = levels.cost_p50_usd / levels.pass_rate;
            Estimate::of(Ev::UsdPerVerifiedSuccess, usd, None, 1)
        } else {
            Estimate::missing(Ev::UsdPerVerifiedSuccess, 1, EvGap::NoSuccess)
        };
        let estimates = EvEstimates {
            pass_rate: Estimate::of(Ev::PassRate, levels.pass_rate, None, 1),
            usd_per_verified_success: cost,
            false_green: Estimate::missing(Ev::FalseGreen, 0, EvGap::NoAudits),
            latency_p90_s: Estimate::of(Ev::LatencyP90S, levels.latency_p90_s, None, 1),
        };
        Some(drive(&estimates, &self.bounds, &self.drive).value)
    }
}

impl MovePrior for M3Prior {
    fn prior(
        &self,
        theta: &HarnessParams,
        candidate: &Candidate,
        breached: &[Ev],
    ) -> MovePriorValue {
        let weight = m3_weight(&self.predictor.calibration());
        let fallback = catalog_prior(candidate.entry.as_ref(), breached);
        if weight <= 0.0 {
            return fallback;
        }
        let now = self.predicted_drive(theta);
        let next = self.predicted_drive(&candidate.next);
        let (Some(now), Some(next)) = (now, next) else {
            return fallback;
        };
        MovePriorValue {
            mean: now - next,
            variance: PRIOR_VARIANCE / weight,
            source: PriorSource::M3,
        }
    }
}

/// M3's priors for the replay's A3-gated and A3-mis arms.
#[derive(Debug, Clone)]
pub struct M3Predictions {
    predictor: Arc<dyn DrivePredictor>,
    policy: ViabilityPolicy,
}

impl M3Predictions {
    /// The predictions of `predictor`, with drives under `policy`.
    #[must_use]
    pub fn new(predictor: Arc<dyn DrivePredictor>, policy: ViabilityPolicy) -> Self {
        Self { predictor, policy }
    }
}

impl Predictions for M3Predictions {
    fn prior(&self) -> Box<dyn MovePrior> {
        Box::new(M3Prior::new(Arc::clone(&self.predictor), &self.policy))
    }
}

/// The feedforward pre-arm (S06 §4.4): an ablation, off unless
/// `[homeostasis] feedforward_prearm` is set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prearm {
    enabled: bool,
    base: DetectorTuning,
    remaining: u32,
}

impl Prearm {
    /// The pre-arm `config` asks for, around the detectors' `base` tuning.
    #[must_use]
    pub const fn new(config: &HomeostasisConfig, base: DetectorTuning) -> Self {
        Self {
            enabled: config.feedforward_prearm,
            base,
            remaining: 0,
        }
    }

    /// Whether H is halved now.
    #[must_use]
    pub const fn armed(&self) -> bool {
        self.remaining > 0
    }

    /// `base` with both decision intervals H halved.
    #[must_use]
    pub fn halved(&self) -> DetectorTuning {
        DetectorTuning {
            pass_rate_h: self.base.pass_rate_h / 2.0,
            log_ratio_h: self.base.log_ratio_h / 2.0,
            ..self.base
        }
    }

    /// M3 predicts drive `predicted` for the next [`PREARM_QUEUE`] queued
    /// tasks against `current`: a rise above [`PREARM_RISE`] arms, or
    /// re-arms, for [`PREARM_RESOLUTIONS`] resolutions. Returns the tuning
    /// to switch the detectors to when the pre-arm starts.
    pub fn observe_prediction(&mut self, current: f64, predicted: f64) -> Option<DetectorTuning> {
        if !self.enabled || predicted - current <= PREARM_RISE {
            return None;
        }
        let starts = self.remaining == 0;
        self.remaining = PREARM_RESOLUTIONS;
        starts.then(|| self.halved())
    }

    /// Count one resolution. Returns the base tuning when the pre-arm ends.
    pub fn on_resolution(&mut self) -> Option<DetectorTuning> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        (self.remaining == 0).then_some(self.base)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use roko_core::config::harness_params::{HarnessLadders, Knob, Step};
    use roko_core::config::homeostasis::HomeostasisMode;
    use roko_core::config::{ProviderConfig, RokoConfig};

    use super::*;
    use crate::homeostasis::catalog::catalog_move;
    use crate::homeostasis::controller::{Controller, ControllerEvent, MoveKey};
    use crate::homeostasis::detect::Baseline;
    use crate::homeostasis::policy::{AuditPolicy, VerifyPolicy};
    use crate::homeostasis::resolution::{CostSourceMix, TaskResolution};
    use crate::telemetry::{AttemptOutcome, CostSource};

    /// A predictor whose pass rate rises 0.1 per added retry and with
    /// nothing else, as under a convention flip, where a stronger tier does
    /// not help.
    #[derive(Debug)]
    struct Stub {
        calibration: Calibration,
    }

    impl DrivePredictor for Stub {
        fn predict(&self, theta: &HarnessParams) -> Option<PredictedLevels> {
            Some(PredictedLevels {
                pass_rate: 0.5 + 0.1 * f64::from(theta.retry_delta),
                cost_p50_usd: 0.05,
                latency_p90_s: 300.0,
            })
        }

        fn calibration(&self) -> Calibration {
            self.calibration
        }
    }

    fn stub(ece: Option<f64>, drift: bool) -> Arc<dyn DrivePredictor> {
        Arc::new(Stub {
            calibration: Calibration { ece, drift },
        })
    }

    fn fixture() -> (RokoConfig, ViabilityPolicy) {
        let mut config = RokoConfig::default();
        for name in ["alpha", "beta"] {
            config
                .providers
                .insert(name.to_string(), ProviderConfig::default());
        }
        let policy = ViabilityPolicy {
            policy_version: 1,
            ev: EvBounds::s06_example(),
            drive: DrivePolicy::default(),
            tiers: BTreeMap::new(),
            verify: VerifyPolicy::default(),
            audit: AuditPolicy::default(),
            holdout: 0.10,
        };
        (config, policy)
    }

    fn resolution(index: u64, passed: bool) -> TaskResolution {
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
            arm: None,
            audited: false,
            audit_false_green: None,
            pre_instrumentation: false,
            resolved_at: i64::try_from(index * 60_000).ok(),
        }
    }

    #[test]
    fn m3_prior_weight_follows_ece() {
        // The three ECE bands give the three weights; drift drops a sharp
        // model to the middle band.
        let at = |ece, drift| m3_weight(&Calibration { ece, drift });
        assert_eq!(at(Some(0.03), false), 1.0);
        assert_eq!(at(Some(0.03), true), 0.25);
        assert_eq!(at(Some(0.08), false), 0.25);
        assert_eq!(at(Some(0.20), false), 0.0);
        assert_eq!(at(None, false), 0.0);

        let (config, policy) = fixture();
        let theta0 = HarnessParams::baseline(&config);
        let ladders = HarnessLadders::from_config(&config);
        let retry = theta0
            .step(Knob::RetryDelta, Step::Up, &ladders)
            .expect("one more retry");
        let candidate = Candidate {
            key: MoveKey {
                knob: Knob::RetryDelta,
                direction: Step::Up,
            },
            entry: catalog_move(Knob::RetryDelta.kind(), Step::Up),
            next: retry,
        };
        let breached = [Ev::PassRate];

        // Calibrated: μ⁰ is the predicted drive reduction, sqrt(2.5³) −
        // sqrt(1.5³), with v_base; a quarter weight quadruples the variance.
        let sharp = M3Prior::new(stub(Some(0.03), false), &policy);
        let prior = sharp.prior(&theta0, &candidate, &breached);
        let expected = 2.5_f64.powi(3).sqrt() - 1.5_f64.powi(3).sqrt();
        assert!((prior.mean - expected).abs() < 1e-9, "{prior:?}");
        assert_eq!(prior.variance, PRIOR_VARIANCE);
        assert_eq!(prior.source, PriorSource::M3);
        let loose = M3Prior::new(stub(Some(0.08), false), &policy);
        let prior = loose.prior(&theta0, &candidate, &breached);
        assert!((prior.mean - expected).abs() < 1e-9);
        assert_eq!(prior.variance, 4.0 * PRIOR_VARIANCE);

        // w = 0 reproduces the catalog-sign prior exactly.
        let blind = M3Prior::new(stub(Some(0.20), false), &policy);
        assert_eq!(
            blind.prior(&theta0, &candidate, &breached),
            catalog_prior(candidate.entry.as_ref(), &breached)
        );

        // With calibrated priors the controller tells a convention flip from
        // a model swap: the predictor sees no gain in a stronger tier, so the
        // first guided move is in convention flip's row (B2 retries), not
        // B1's floor. The replay hands A3-gated and A3-mis the same priors.
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
        let mut controller =
            Controller::new(&settings, policy.clone(), theta0, ladders, baseline, 7)
                .with_prior(Box::new(M3Prior::new(stub(Some(0.03), false), &policy)));
        let mut first = None;
        for index in 1..=24 {
            let passed = index <= 20 && index % 5 != 1;
            for event in controller.on_resolution(&resolution(index, passed)) {
                if let ControllerEvent::Change(change) = event {
                    first.get_or_insert(*change);
                }
            }
        }
        let first = first.expect("the breach makes a move");
        assert_eq!(first.knob, Knob::RetryDelta);
        assert_eq!(
            first.predicted.map(|prior| prior.source),
            Some(PriorSource::M3)
        );
        let predictions = M3Predictions::new(stub(Some(0.03), false), policy);
        let handed = predictions.prior();
        assert!((handed.prior(controller.theta(), &candidate, &breached).mean).is_finite());

        // The pre-arm is off by default; on, a rise above 0.2 halves H for
        // ten resolutions.
        let base = DetectorTuning::default();
        let mut off = Prearm::new(&HomeostasisConfig::default(), base);
        assert_eq!(off.observe_prediction(0.0, 1.0), None);
        let enabled = HomeostasisConfig {
            feedforward_prearm: true,
            ..HomeostasisConfig::default()
        };
        let mut on = Prearm::new(&enabled, base);
        assert_eq!(on.observe_prediction(0.5, 0.6), None, "a rise of 0.1");
        let halved = on.observe_prediction(0.5, 0.8).expect("a rise of 0.3 arms");
        assert!((halved.pass_rate_h - base.pass_rate_h / 2.0).abs() < 1e-12);
        assert!((halved.log_ratio_h - base.log_ratio_h / 2.0).abs() < 1e-12);
        assert!(on.armed());
        for _ in 1..PREARM_RESOLUTIONS {
            assert_eq!(on.on_resolution(), None);
        }
        assert_eq!(on.on_resolution(), Some(base));
        assert!(!on.armed());
    }
}
