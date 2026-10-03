//! Historical and synthetic resolution streams for the replay (S06 §0, §7
//! A1).
//!
//! - [`StreamSpec`] is the replay's `--stream`: `historical:<learn dir>` or
//!   `synthetic:<kind>@<t>`.
//! - A historical stream folds the directory's `efficiency.jsonl` and
//!   `costs.jsonl` (8106). Every resolution is `pre_instrumentation`, and
//!   the baseline and bounds come from its first window ([`calibrate`]).
//! - A synthetic stream is seeded resolutions, in control (E1 0.80, $0.05
//!   and five minutes per resolution, lognormal noise) up to position `t`
//!   and stepped after it the way the kind shows in resolutions under θ₀
//!   ([`Regime::stepped`]). Each position draws from its own ChaCha8 stream
//!   of the seed ([`Draws::at`]), whatever θ or arm reads it: the common
//!   random numbers of the replay evaluator (8117).

use std::collections::BTreeMap;
use std::path::PathBuf;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use roko_core::config::harness_params::{HarnessLadders, HarnessParams};
use roko_core::config::RokoConfig;

use super::catalog::DisturbanceKind;
use super::detect::Baseline;
use super::ev::{DrivePolicy, EvBounds, LowerBound, UpperBound};
use super::policy::{AuditPolicy, DEFAULT_HOLDOUT, VerifyPolicy, ViabilityPolicy};
use super::resolution::{CostSourceMix, HistoricalFoldReport, TaskResolution, fold_historical_dir};
use crate::telemetry::{AttemptOutcome, CostSource};

/// A synthetic stream's in-control levels.
pub const IN_CONTROL: Baseline = Baseline {
    pass_rate: 0.80,
    usd_per_resolution: 0.05,
    wall_ms: 300_000.0,
};
/// The sd of the lognormal noise on a synthetic cost and wall time.
pub const LOG_SD: f64 = 0.25;
/// Milliseconds between synthetic resolutions.
pub const SPACING_MS: i64 = 60_000;
/// Mixed into the seed of the position streams, so that they never share a
/// stream with the controller's decisions.
pub const STREAM_SALT: u64 = 0x5354_5245_414d_0001;
/// Resolutions a historical stream is calibrated on.
pub const CALIBRATION_WINDOW: usize = 20;
/// z of a normal's 90th percentile.
const Z_P90: f64 = 1.281_552;

/// What a synthetic stream steps into: a canonical disturbance, with
/// `provider_fault` possibly slowing every provider (`provider_fault_all`,
/// its unregulable form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StepKind {
    /// The disturbance.
    pub kind: DisturbanceKind,
    /// Every provider is slowed.
    pub all_providers: bool,
}

impl StepKind {
    /// The name of `provider_fault` on every provider.
    pub const ALL_PROVIDERS: &'static str = "provider_fault_all";

    /// The kinds a synthetic stream knows: `disturb.py`'s six,
    /// `price_shock` and `provider_fault_all`.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let mut kinds: Vec<Self> = DisturbanceKind::DISTURB_PY
            .into_iter()
            .chain([DisturbanceKind::PriceShock])
            .map(Self::of)
            .collect();
        kinds.push(Self {
            kind: DisturbanceKind::ProviderFault,
            all_providers: true,
        });
        kinds
    }

    /// `kind` on one provider.
    #[must_use]
    pub const fn of(kind: DisturbanceKind) -> Self {
        Self {
            kind,
            all_providers: false,
        }
    }

    /// The kind called `name`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::all().into_iter().find(|step| step.name() == name)
    }

    /// Its name in a `--stream`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        if self.all_providers {
            Self::ALL_PROVIDERS
        } else {
            self.kind.name()
        }
    }
}

/// How the resolutions at one position behave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Regime {
    /// The chance a resolution passes.
    pub pass_rate: f64,
    /// Its cost, as a multiple of the in-control cost.
    pub cost_factor: f64,
    /// Its wall time, as a multiple of the in-control wall time.
    pub wall_factor: f64,
    /// Every resolution had a provider error on some attempt.
    pub provider_errors: bool,
    /// Failures end on the task budget.
    pub budget_failures: bool,
    /// Attempts per resolution.
    pub attempts: u32,
}

impl Regime {
    /// In control.
    pub const IN_CONTROL: Self = Self {
        pass_rate: 0.80,
        cost_factor: 1.0,
        wall_factor: 1.0,
        provider_errors: false,
        budget_failures: false,
        attempts: 1,
    };

    /// The regime once `step` has started, under θ₀: what S06 §4.9's
    /// "expected effect" column looks like in resolutions, with S06 C1's
    /// steps (E1 0.80 → 0.50, E4 ×2) and every provider slowed ×5. A
    /// budget cut fails 70% of its tasks on the budget, so its cost per
    /// success rises too; a flaky verify reruns the visible check only, so
    /// its retries cost nothing extra.
    #[must_use]
    pub const fn stepped(step: StepKind) -> Self {
        let base = Self::IN_CONTROL;
        match step.kind {
            DisturbanceKind::ProviderFault if step.all_providers => Self {
                wall_factor: 5.0,
                ..base
            },
            DisturbanceKind::ProviderFault => Self {
                pass_rate: 0.50,
                wall_factor: 2.0,
                provider_errors: true,
                ..base
            },
            DisturbanceKind::ModelSwap | DisturbanceKind::ConventionFlip => Self {
                pass_rate: 0.50,
                ..base
            },
            DisturbanceKind::HarderMix => Self {
                pass_rate: 0.50,
                cost_factor: 1.25,
                ..base
            },
            DisturbanceKind::BudgetCut => Self {
                pass_rate: 0.30,
                budget_failures: true,
                ..base
            },
            DisturbanceKind::FlakyVerify => Self {
                pass_rate: 0.50,
                attempts: 2,
                ..base
            },
            DisturbanceKind::PriceShock => Self {
                cost_factor: 2.0,
                ..base
            },
        }
    }
}

/// The draws of one position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draws {
    /// Uniform in [0, 1): the resolution passes when this is below its pass
    /// rate, so a higher pass rate passes a superset of positions.
    pub pass: f64,
    /// Standard normal noise on the log of the cost.
    pub cost: f64,
    /// Standard normal noise on the log of the wall time.
    pub wall: f64,
}

impl Draws {
    /// The draws at `position`: ChaCha8 stream `position` of `seed` mixed
    /// with [`STREAM_SALT`].
    #[must_use]
    pub fn at(seed: u64, position: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ STREAM_SALT);
        rng.set_stream(position);
        let pass = rng.r#gen();
        let cost = gaussian(&mut rng);
        let wall = gaussian(&mut rng);
        Self { pass, cost, wall }
    }
}

fn gaussian(rng: &mut ChaCha8Rng) -> f64 {
    let u1: f64 = rng.gen_range(f64::MIN_POSITIVE..1.0);
    let u2: f64 = rng.gen_range(0.0..1.0);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// The resolution at `position` (from 1) in `regime` with `draws`.
#[must_use]
pub fn resolve(position: u64, regime: &Regime, draws: &Draws) -> TaskResolution {
    let passed = draws.pass < regime.pass_rate;
    let usd = IN_CONTROL.usd_per_resolution * regime.cost_factor * (LOG_SD * draws.cost).exp();
    let wall_ms = IN_CONTROL.wall_ms * regime.wall_factor * (LOG_SD * draws.wall).exp();
    let final_verdict = if passed {
        AttemptOutcome::Passed
    } else if regime.budget_failures {
        AttemptOutcome::BudgetExhausted
    } else {
        AttemptOutcome::GateFailed
    };
    let mut cost_source_mix = CostSourceMix::default();
    for _ in 0..regime.attempts {
        cost_source_mix.add(CostSource::ProviderUsage);
    }
    TaskResolution {
        chain_key: format!("synthetic:plan:t{position}"),
        final_verdict,
        attempts: regime.attempts,
        api_equiv_usd: Some(usd),
        cost_source_mix,
        wall_ms: Some(wall_ms as u64),
        provider_errors: u32::from(regime.provider_errors),
        conductor_restarts: 0,
        params_digest: None,
        arm: None,
        audited: false,
        audit_false_green: None,
        pre_instrumentation: false,
        resolved_at: i64::try_from(position)
            .ok()
            .map(|position| position.saturating_mul(SPACING_MS)),
    }
}

/// A seeded synthetic stream that steps after position `onset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticStream {
    /// What it steps into.
    pub step: StepKind,
    /// The last in-control position.
    pub onset: u64,
    /// The seed of every position's draws.
    pub seed: u64,
}

impl SyntheticStream {
    /// The regime at `position` (from 1) under θ₀.
    #[must_use]
    pub const fn regime(&self, position: u64) -> Regime {
        if position > self.onset {
            Regime::stepped(self.step)
        } else {
            Regime::IN_CONTROL
        }
    }

    /// The resolution at `position` under θ₀.
    #[must_use]
    pub fn resolution(&self, position: u64) -> TaskResolution {
        resolve(position, &self.regime(position), &Draws::at(self.seed, position))
    }

    /// Positions 1 to `length` under θ₀.
    #[must_use]
    pub fn resolutions(&self, length: u64) -> Vec<TaskResolution> {
        (1..=length)
            .map(|position| self.resolution(position))
            .collect()
    }

    /// The policy a calibration of the in-control levels proposes.
    #[must_use]
    pub fn policy(&self) -> ViabilityPolicy {
        let wall_p90_ms = IN_CONTROL.wall_ms * (Z_P90 * LOG_SD).exp();
        calibrated_policy(&IN_CONTROL, wall_p90_ms)
    }
}

/// The S5 policy a calibration at `baseline` proposes (S06 §4.2).
///
/// E1 ≥ max(0.5, p* − 0.15), E2 ≤ min(2 × the in-control cost per verified
/// success, $0.50), E3 ≤ 0.10, and E4 ≤ 2 × the in-control p90,
/// `wall_p90_ms`. A live run uses bounds only once a person has written
/// them to `.roko/policy/viability.toml`.
#[must_use]
pub fn calibrated_policy(baseline: &Baseline, wall_p90_ms: f64) -> ViabilityPolicy {
    let lo = (baseline.pass_rate - 0.15).max(0.5);
    let usd = baseline.usd_per_resolution / baseline.pass_rate.max(f64::EPSILON);
    ViabilityPolicy {
        policy_version: 1,
        ev: EvBounds {
            pass_rate: LowerBound { lo, inner: None },
            usd_per_verified_success: UpperBound {
                hi: 2.0 * usd,
                inner: None,
                abs_cap: Some(0.50),
            },
            false_green: UpperBound {
                hi: 0.10,
                inner: None,
                abs_cap: None,
            },
            latency_p90_s: UpperBound {
                hi: 2.0 * wall_p90_ms / 1000.0,
                inner: None,
                abs_cap: None,
            },
        },
        drive: DrivePolicy::default(),
        tiers: BTreeMap::new(),
        verify: VerifyPolicy::default(),
        audit: AuditPolicy::default(),
        holdout: DEFAULT_HOLDOUT,
    }
}

/// A historical stream's baseline and policy, calibrated on its first
/// [`CALIBRATION_WINDOW`] resolutions: their pass rate, median cost and
/// median wall time, and their wall-time p90.
#[must_use]
pub fn calibrate(resolutions: &[TaskResolution]) -> (Baseline, ViabilityPolicy) {
    let window = &resolutions[..resolutions.len().min(CALIBRATION_WINDOW)];
    let passed = window
        .iter()
        .filter(|resolution| resolution.verified_success())
        .count();
    let mut costs: Vec<f64> = window
        .iter()
        .filter_map(|resolution| resolution.api_equiv_usd)
        .collect();
    let mut walls: Vec<f64> = window
        .iter()
        .filter_map(|resolution| resolution.wall_ms)
        .map(|ms| ms as f64)
        .collect();
    let baseline = Baseline {
        pass_rate: passed as f64 / window.len().max(1) as f64,
        usd_per_resolution: quantile(&mut costs, 0.5),
        wall_ms: quantile(&mut walls, 0.5),
    };
    let wall_p90_ms = quantile(&mut walls, 0.9);
    (baseline, calibrated_policy(&baseline, wall_p90_ms))
}

/// The nearest-rank `q` quantile of `values`; 0 when there are none.
fn quantile(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_unstable_by(f64::total_cmp);
    let rank = ((values.len() as f64 * q).ceil() as usize).clamp(1, values.len());
    values[rank - 1]
}

/// θ₀ and its ladders for a replay: the default config's, with two
/// placeholder providers so that B1's provider order can move.
#[must_use]
pub fn replay_theta0() -> (HarnessParams, HarnessLadders) {
    let mut config = RokoConfig::default();
    for name in ["primary", "secondary"] {
        config
            .providers
            .entry(name.to_string())
            .or_default();
    }
    (
        HarnessParams::baseline(&config),
        HarnessLadders::from_config(&config),
    )
}

/// The replay's `--stream`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamSpec {
    /// `historical:<learn dir>`: the logs written before S01's telemetry.
    Historical(PathBuf),
    /// `synthetic:<kind>@<t>`: in control up to position `t`, then stepped.
    Synthetic {
        /// What it steps into.
        step: StepKind,
        /// The last in-control position.
        onset: u64,
    },
}

/// A stream ready to replay.
#[derive(Debug, Clone)]
pub struct LoadedStream {
    /// Its resolutions, oldest first.
    pub resolutions: Vec<TaskResolution>,
    /// The detectors' in-control levels.
    pub baseline: Baseline,
    /// The bounds a calibration proposes.
    pub policy: ViabilityPolicy,
    /// Folded from logs written before S01's telemetry.
    pub pre_instrumentation: bool,
    /// What the historical fold read and left out.
    pub report: Option<HistoricalFoldReport>,
}

impl StreamSpec {
    /// Parse a `--stream` value.
    ///
    /// # Errors
    ///
    /// What is wrong with it.
    pub fn parse(text: &str) -> Result<Self, String> {
        if let Some(dir) = text.strip_prefix("historical:") {
            return Ok(Self::Historical(PathBuf::from(dir)));
        }
        let Some(rest) = text.strip_prefix("synthetic:") else {
            return Err(format!(
                "`{text}` is neither historical:<dir> nor synthetic:<kind>@<t>"
            ));
        };
        let Some((name, onset)) = rest.split_once('@') else {
            return Err(format!("`{text}` needs @<t>, the last in-control position"));
        };
        let step = StepKind::parse(name).ok_or_else(|| format!("unknown kind `{name}`"))?;
        let onset = onset
            .parse()
            .map_err(|error| format!("`{onset}` is not a position: {error}"))?;
        Ok(Self::Synthetic { step, onset })
    }

    /// The spec as `--stream` writes it.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Historical(dir) => format!("historical:{}", dir.display()),
            Self::Synthetic { step, onset } => format!("synthetic:{}@{onset}", step.name()),
        }
    }

    /// Load the stream: fold the historical directory, or draw `length`
    /// synthetic resolutions from `seed`.
    ///
    /// # Errors
    ///
    /// A historical log that exists but cannot be read.
    pub fn load(&self, seed: u64, length: u64) -> std::io::Result<LoadedStream> {
        match self {
            Self::Historical(dir) => {
                let (resolutions, report) = fold_historical_dir(dir)?;
                let (baseline, policy) = calibrate(&resolutions);
                Ok(LoadedStream {
                    resolutions,
                    baseline,
                    policy,
                    pre_instrumentation: true,
                    report: Some(report),
                })
            }
            Self::Synthetic { step, onset } => {
                let stream = SyntheticStream {
                    step: *step,
                    onset: *onset,
                    seed,
                };
                Ok(LoadedStream {
                    resolutions: stream.resolutions(length),
                    baseline: IN_CONTROL,
                    policy: stream.policy(),
                    pre_instrumentation: false,
                    report: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_streams_step_after_onset_and_share_draws() {
        // Every kind parses back from its name, and specs from their label.
        for step in StepKind::all() {
            assert_eq!(StepKind::parse(step.name()), Some(step));
        }
        let spec = StreamSpec::parse("synthetic:model_swap@20").expect("a synthetic spec");
        assert_eq!(spec.label(), "synthetic:model_swap@20");
        let historical = StreamSpec::parse("historical:.roko/learn").expect("a historical spec");
        assert_eq!(historical, StreamSpec::Historical(PathBuf::from(".roko/learn")));
        assert!(StreamSpec::parse("synthetic:model_swap").is_err());
        assert!(StreamSpec::parse("synthetic:meteor@3").is_err());
        assert!(StreamSpec::parse("live:x").is_err());

        // In control up to the onset, stepped after it; a position's draws
        // are the same whatever reads them.
        let stream = SyntheticStream {
            step: StepKind::of(DisturbanceKind::ModelSwap),
            onset: 20,
            seed: 7,
        };
        assert_eq!(stream.regime(20), Regime::IN_CONTROL);
        assert!((stream.regime(21).pass_rate - 0.50).abs() < 1e-12);
        assert_eq!(Draws::at(7, 21), Draws::at(7, 21));
        assert_ne!(Draws::at(7, 21), Draws::at(7, 22));
        assert_ne!(Draws::at(7, 21), Draws::at(8, 21));
        let resolutions = stream.resolutions(400);
        assert_eq!(resolutions, stream.resolutions(400));
        assert_eq!(resolutions[20].resolved_at, Some(21 * SPACING_MS));
        let rate = |slice: &[TaskResolution]| {
            let passed = slice.iter().filter(|r| r.verified_success()).count();
            passed as f64 / slice.len() as f64
        };
        let long = SyntheticStream {
            onset: 200,
            ..stream
        }
        .resolutions(400);
        let (before, after) = (rate(&long[..200]), rate(&long[200..]));
        assert!(before > 0.7 && before < 0.9, "{before}");
        assert!(after > 0.4 && after < 0.6, "{after}");

        // A higher pass rate passes a superset of positions (common random
        // numbers).
        let better = Regime {
            pass_rate: 0.75,
            ..Regime::stepped(stream.step)
        };
        for position in 21..=400 {
            let draws = Draws::at(7, position);
            let base = resolve(position, &stream.regime(position), &draws);
            let lifted = resolve(position, &better, &draws);
            assert!(!base.verified_success() || lifted.verified_success());
        }

        // The calibrated bounds sit around the in-control levels.
        let policy = stream.policy();
        assert!(policy.problems().is_empty(), "{:?}", policy.problems());
        assert!((policy.ev.pass_rate.lo - 0.65).abs() < 1e-12);
        assert!((policy.ev.usd_per_verified_success.outer() - 0.125).abs() < 1e-12);
        let (baseline, calibrated) = calibrate(&resolutions[..20]);
        assert!((baseline.usd_per_resolution - 0.05).abs() < 0.02);
        assert!(calibrated.ev.latency_p90_s.hi > 600.0);
    }
}
