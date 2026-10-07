//! Benefit estimators: IPW difference in means, CUPED and AIPW within declared bounds
//! (S03 §4.4–§4.5; backlog 5113).
//!
//! β = E[U | do(Z = L)] − E[U | do(Z = D)] on the outcome
//! `U_t = 1[verified pass] − (M_t/π_t)·fg_t`, where M_t is S05's independent
//! audit draw, π_t its logged selection probability floored at q, and fg_t
//! the false-green label: U ∈ [1 − 1/q, 1]. β_pass is the same contrast on
//! U^pass = 1[verified pass], β_fg = β_pass − β, and β_cost the saving
//! E[c | D] − E[c | L] on the normalized chain cost c = min(api_equiv_usd /
//! c_cap, 1), only where S01's `cost.source` is reported usage. A chain
//! whose verdict is `Unverified` or `ForcedAccept` scores 0 (no verified
//! success); per-arm Unverified shares are reported. Pre-instrumentation
//! rows (no typed verdict) are excluded.
//!
//! Per-loop β reads rows with `global_off = false` only, weighted by the
//! conditional default propensity p_t = (P_t(D) − g)/(1 − g) recovered from
//! the logged propensity, never from config; all-off rows feed only
//! β_global. Every increment ψ_t has a bound B_t fixed before the arm is
//! drawn (S03 §4.5's table), and each estimator runs an empirical-Bernstein
//! confidence sequence ([`super::cs::EmpiricalBernsteinCs`]) on ψ_t scaled
//! by its own B_t. CUPED's coefficient κ and the covariate mean are fit on
//! earlier rows only (predictable) and κ is clipped to ±κ_max; AIPW's
//! predictions are clipped to `[0, 1]`. Without a covariate or predictions,
//! the post-stratified estimator is AIPW with the stratum-and-arm running
//! means of earlier rows as its predictions.

use std::collections::HashMap;

use super::cs::EmpiricalBernsteinCs;
use crate::telemetry::records::{CostSource, GateVerdictTag};

/// S05's audit floor q = π_min: no selection probability counts below it.
pub const AUDIT_FLOOR: f64 = 0.05;

/// The clip κ_max of CUPED's coefficient.
pub const KAPPA_MAX: f64 = 2.0;

/// The chain cost, in API-equivalent USD, that normalizes to 1.
pub const COST_CAP_USD: f64 = 1.0;

/// The estimators' parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenefitConfig {
    /// Level of each confidence sequence (the caller's α/K).
    pub alpha: f64,
    /// The audit floor q.
    pub audit_floor: f64,
    /// CUPED's κ_max.
    pub kappa_max: f64,
    /// c_cap, the chain cost that normalizes to 1.
    pub cost_cap_usd: f64,
}

impl Default for BenefitConfig {
    fn default() -> Self {
        Self {
            alpha: 0.05,
            audit_floor: AUDIT_FLOOR,
            kappa_max: KAPPA_MAX,
            cost_cap_usd: COST_CAP_USD,
        }
    }
}

/// What is known of a chain's context before its arm is drawn (R.S03-5).
/// The caller builds it before the draw and never from the arm.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreAssignment {
    /// The one arm-blind covariate X_t ∈ `[0, 1]`: LOG1 mean VS from S09's
    /// prereg lock in bench campaigns, S04's forecast in production.
    pub covariate: Option<f64>,
    /// S04's per-arm predictions (m̂_L, m̂_D), clipped to `[0, 1]` on use.
    pub predictions: Option<(f64, f64)>,
    /// The stratum of the post-stratified estimator (task type, role,
    /// tier).
    pub stratum: Option<String>,
}

/// How a chain settled, as the outcome reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainOutcome {
    /// The chain carries this gate verdict tag (S01's typed verdict).
    Tagged(GateVerdictTag),
    /// The chain failed its verify steps.
    Failed,
    /// No typed verdict: a pre-instrumentation row, which β excludes.
    PreInstrumentation,
}

/// S05's audit draw on a chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuditDraw {
    /// M_t: the chain was selected for audit.
    pub selected: bool,
    /// π_t, the logged selection probability.
    pub pi: f64,
    /// fg_t: the audit found a false green.
    pub false_green: bool,
}

/// One settled chain on a loop's layer.
#[derive(Debug, Clone, PartialEq)]
pub struct BenefitRow {
    /// What was known before the arm was drawn.
    pub pre: PreAssignment,
    /// The chain drew the learned arm.
    pub learned_arm: bool,
    /// The chain drew the all-learning-off arm (G_t = 1).
    pub global_off: bool,
    /// The logged propensity of the default arm, P_t(D) = g + (1 − g)·h_t.
    pub logged_default: f64,
    /// The all-off rate g in force.
    pub global_rate: f64,
    /// How the chain settled.
    pub outcome: ChainOutcome,
    /// S05's audit draw, if the chain had one.
    pub audit: Option<AuditDraw>,
    /// The chain's API-equivalent cost and its source.
    pub cost: Option<(f64, CostSource)>,
}

impl BenefitRow {
    /// p_t = P(Z = D | G = 0) = (P_t(D) − g)/(1 − g), from the logged
    /// propensity.
    #[must_use]
    pub fn default_propensity(&self) -> f64 {
        (self.logged_default - self.global_rate) / (1.0 - self.global_rate)
    }

    /// U^pass_t: 1 for a verified pass, 0 otherwise; `None` for a
    /// pre-instrumentation row.
    #[must_use]
    pub fn pass_outcome(&self) -> Option<f64> {
        match self.outcome {
            ChainOutcome::Tagged(verdict) => Some(f64::from(u8::from(verified_pass(verdict)))),
            ChainOutcome::Failed => Some(0.0),
            ChainOutcome::PreInstrumentation => None,
        }
    }

    /// U_t = U^pass_t − (M_t/π_t)·fg_t, with π_t floored at `audit_floor`.
    #[must_use]
    pub fn outcome(&self, audit_floor: f64) -> Option<f64> {
        let pass = self.pass_outcome()?;
        let penalty = self.audit.map_or(0.0, |audit| {
            if audit.selected && audit.false_green {
                1.0 / audit.pi.max(audit_floor)
            } else {
                0.0
            }
        });
        Some(pass - penalty)
    }

    /// c_t = min(api_equiv_usd / `cap`, 1), only for reported usage.
    #[must_use]
    pub fn normalized_cost(&self, cap: f64) -> Option<f64> {
        let (usd, source) = self.cost?;
        source
            .is_reported_usage()
            .then(|| (usd / cap).clamp(0.0, 1.0))
    }
}

/// Whether every authored verify step passed: `Passed`, or a pass with
/// pre-existing failures, or an already-satisfied task.
fn verified_pass(verdict: GateVerdictTag) -> bool {
    matches!(
        verdict,
        GateVerdictTag::Passed
            | GateVerdictTag::PassedWithPreexistingFailures
            | GateVerdictTag::AlreadySatisfied
    )
}

/// One increment ψ_t and its declared bound B_t ≥ |ψ_t|.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Increment {
    /// ψ_t.
    pub value: f64,
    /// B_t.
    pub bound: f64,
}

/// The increments one row adds, per estimator; `None` where the estimator
/// does not apply to the row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Increments {
    /// IPW difference in means on U.
    pub dim: Increment,
    /// CUPED, with a covariate.
    pub cuped: Option<Increment>,
    /// AIPW, with predictions.
    pub aipw: Option<Increment>,
    /// The post-stratified estimator, with a stratum.
    pub post_stratified: Option<Increment>,
    /// IPW difference in means on U^pass.
    pub pass: Increment,
    /// The cost saving, with reported usage.
    pub cost: Option<Increment>,
}

/// An estimator's running estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// Increments taken.
    pub n: u64,
    /// Their mean, the point estimate.
    pub mean: f64,
    /// Their sample variance.
    pub variance: f64,
    /// The interval of the confidence sequence; `None` before the first.
    pub interval: Option<(f64, f64)>,
}

impl Estimate {
    /// The standard error of the mean, √(variance/n).
    #[must_use]
    pub fn standard_error(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            (self.variance / self.n as f64).sqrt()
        }
    }
}

/// One estimator's increments: their sums and confidence sequence.
#[derive(Debug, Clone)]
struct Track {
    cs: EmpiricalBernsteinCs,
    n: u64,
    sum: f64,
    squares: f64,
}

impl Track {
    fn new(alpha: f64) -> Self {
        Self {
            cs: EmpiricalBernsteinCs::new(alpha),
            n: 0,
            sum: 0.0,
            squares: 0.0,
        }
    }

    fn push(&mut self, increment: Increment) {
        self.n += 1;
        self.sum += increment.value;
        self.squares += increment.value * increment.value;
        self.cs.push(increment.value, increment.bound);
    }

    fn estimate(&self) -> Estimate {
        let n = self.n as f64;
        let mean = if self.n == 0 { 0.0 } else { self.sum / n };
        let variance = if self.n < 2 {
            0.0
        } else {
            ((self.squares - n * mean * mean) / (n - 1.0)).max(0.0)
        };
        Estimate {
            n: self.n,
            mean,
            variance,
            interval: self.cs.interval(),
        }
    }
}

/// Where [`BenefitEstimator::push`] counted a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Into the loop's estimators and β_global.
    Loop,
    /// An all-off row: into β_global only.
    Global,
    /// Excluded: no typed verdict, or no usable propensity.
    Excluded,
}

/// A loop's benefit estimators over its settled chains, in order.
#[derive(Debug, Clone)]
pub struct BenefitEstimator {
    config: BenefitConfig,
    dim: Track,
    cuped: Track,
    aipw: Track,
    post_stratified: Track,
    pass: Track,
    cost: Track,
    global: Track,
    /// CUPED's pooled, arm-blind sums over earlier rows: n, ΣX, ΣU, ΣXU, ΣX².
    covariate_sums: (f64, f64, f64, f64, f64),
    /// The post-stratified predictions: per stratum and arm, n and ΣU.
    strata: HashMap<(String, bool), (u64, f64)>,
    /// Per arm (learned, default): Unverified chains and all chains.
    unverified: [(u64, u64); 2],
}

impl BenefitEstimator {
    /// Estimators with `config`.
    #[must_use]
    pub fn new(config: BenefitConfig) -> Self {
        let track = || Track::new(config.alpha);
        Self {
            config,
            dim: track(),
            cuped: track(),
            aipw: track(),
            post_stratified: track(),
            pass: track(),
            cost: track(),
            global: track(),
            covariate_sums: (0.0, 0.0, 0.0, 0.0, 0.0),
            strata: HashMap::new(),
            unverified: [(0, 0); 2],
        }
    }

    /// CUPED's predictable κ and covariate mean from earlier rows; κ = 0
    /// until the covariate varies.
    fn cuped_fit(&self) -> (f64, f64) {
        let (n, x, u, xu, xx) = self.covariate_sums;
        if n < 2.0 {
            return (0.0, if n > 0.0 { x / n } else { 0.5 });
        }
        let variance = xx - x * x / n;
        let kappa = if variance > 1e-12 {
            (xu - x * u / n) / variance
        } else {
            0.0
        };
        let kappa_max = self.config.kappa_max;
        (kappa.clamp(-kappa_max, kappa_max), x / n)
    }

    /// The increments `row` adds to the loop's estimators, before any state
    /// update; `None` for a row the loop's estimators exclude (all-off,
    /// pre-instrumentation, or a propensity outside (0, 1)).
    #[must_use]
    pub fn increments(&self, row: &BenefitRow) -> Option<Increments> {
        let p = row.default_propensity();
        if row.global_off || !(p > 0.0 && p < 1.0) {
            return None;
        }
        let q = self.config.audit_floor;
        let u = row.outcome(q)?;
        let pass = row.pass_outcome()?;
        let p_min = p.min(1.0 - p);
        let weight = if row.learned_arm {
            1.0 / (1.0 - p)
        } else {
            -1.0 / p
        };
        let doubly_robust = |(learned, default): (f64, f64)| {
            let (learned, default) = (learned.clamp(0.0, 1.0), default.clamp(0.0, 1.0));
            let correction = if row.learned_arm {
                (u - learned) / (1.0 - p)
            } else {
                -(u - default) / p
            };
            Increment {
                value: learned - default + correction,
                bound: 1.0 + 1.0 / (q * p_min),
            }
        };
        let cuped = row.pre.covariate.map(|x| {
            let (kappa, mean) = self.cuped_fit();
            Increment {
                value: (u - kappa * (x.clamp(0.0, 1.0) - mean)) * weight,
                bound: (1.0 / q + self.config.kappa_max) / p_min,
            }
        });
        let post_stratified = row.pre.stratum.as_ref().map(|stratum| {
            let mean = |learned: bool| {
                let key = (stratum.clone(), learned);
                self.strata
                    .get(&key)
                    .map_or(0.0, |(n, sum)| sum / *n as f64)
            };
            doubly_robust((mean(true), mean(false)))
        });
        Some(Increments {
            dim: Increment {
                value: u * weight,
                bound: 1.0 / (q * p_min),
            },
            cuped,
            aipw: row.pre.predictions.map(doubly_robust),
            post_stratified,
            pass: Increment {
                value: pass * weight,
                bound: 1.0 / p_min,
            },
            cost: row
                .normalized_cost(self.config.cost_cap_usd)
                .map(|cost| Increment {
                    value: -cost * weight,
                    bound: 1.0 / p_min,
                }),
        })
    }

    /// Take one settled chain.
    pub fn push(&mut self, row: &BenefitRow) -> Admission {
        let q = self.config.audit_floor;
        let Some(u) = row.outcome(q) else {
            return Admission::Excluded;
        };
        let g = row.global_rate;
        if g > 0.0 && g < 1.0 {
            let weight = if row.global_off {
                -1.0 / g
            } else {
                1.0 / (1.0 - g)
            };
            self.global.push(Increment {
                value: u * weight,
                bound: 1.0 / (q * g.min(1.0 - g)),
            });
        }
        if row.global_off {
            return Admission::Global;
        }
        let Some(increments) = self.increments(row) else {
            return Admission::Excluded;
        };
        self.dim.push(increments.dim);
        self.pass.push(increments.pass);
        for (track, increment) in [
            (&mut self.cuped, increments.cuped),
            (&mut self.aipw, increments.aipw),
            (&mut self.post_stratified, increments.post_stratified),
            (&mut self.cost, increments.cost),
        ] {
            if let Some(increment) = increment {
                track.push(increment);
            }
        }
        if let Some(x) = row.pre.covariate {
            let x = x.clamp(0.0, 1.0);
            let sums = &mut self.covariate_sums;
            *sums = (
                sums.0 + 1.0,
                sums.1 + x,
                sums.2 + u,
                sums.3 + x * u,
                sums.4 + x * x,
            );
        }
        if let Some(stratum) = &row.pre.stratum {
            let entry = self
                .strata
                .entry((stratum.clone(), row.learned_arm))
                .or_insert((0, 0.0));
            *entry = (entry.0 + 1, entry.1 + u);
        }
        let arm = &mut self.unverified[usize::from(!row.learned_arm)];
        arm.1 += 1;
        let unverified = row.outcome == ChainOutcome::Tagged(GateVerdictTag::Unverified);
        arm.0 += u64::from(unverified);
        Admission::Loop
    }

    /// β by the IPW difference in means.
    #[must_use]
    pub fn dim(&self) -> Estimate {
        self.dim.estimate()
    }

    /// β by CUPED, over the rows with a covariate.
    #[must_use]
    pub fn cuped(&self) -> Estimate {
        self.cuped.estimate()
    }

    /// β by AIPW, over the rows with predictions.
    #[must_use]
    pub fn aipw(&self) -> Estimate {
        self.aipw.estimate()
    }

    /// β by the post-stratified estimator, over the rows with a stratum.
    #[must_use]
    pub fn post_stratified(&self) -> Estimate {
        self.post_stratified.estimate()
    }

    /// β_pass.
    #[must_use]
    pub fn pass(&self) -> Estimate {
        self.pass.estimate()
    }

    /// β_cost, the saving, over the rows with reported usage.
    #[must_use]
    pub fn cost(&self) -> Estimate {
        self.cost.estimate()
    }

    /// β_global, learning on against all-off, over every instrumented row.
    #[must_use]
    pub fn global(&self) -> Estimate {
        self.global.estimate()
    }

    /// β_fg = β_pass − β, by the difference in means.
    #[must_use]
    pub fn false_green(&self) -> f64 {
        self.pass.estimate().mean - self.dim.estimate().mean
    }

    /// The Unverified share of the learned and the default arm's chains.
    #[must_use]
    pub fn unverified_shares(&self) -> (f64, f64) {
        let share = |(unverified, all): (u64, u64)| {
            if all == 0 {
                0.0
            } else {
                unverified as f64 / all as f64
            }
        };
        (share(self.unverified[0]), share(self.unverified[1]))
    }
}

#[cfg(test)]
mod tests {
    use super::super::sim::SplitMix64;
    use super::*;

    /// A row at default propensity `p` (no all-off arm) with outcome
    /// `passed`.
    fn row(learned_arm: bool, p: f64, passed: bool) -> BenefitRow {
        BenefitRow {
            pre: PreAssignment::default(),
            learned_arm,
            global_off: false,
            logged_default: p,
            global_rate: 0.0,
            outcome: if passed {
                ChainOutcome::Tagged(GateVerdictTag::Passed)
            } else {
                ChainOutcome::Failed
            },
            audit: None,
            cost: None,
        }
    }

    /// A row of a two-arm layer at p = 0.2 with pass rates 0.7 (learned)
    /// and 0.6 (default), so β = 0.1.
    fn draw_row(rng: &mut SplitMix64) -> BenefitRow {
        let learned = rng.next_f64() >= 0.2;
        let rate = if learned { 0.7 } else { 0.6 };
        row(learned, 0.2, rng.next_f64() < rate)
    }

    /// S03 §4.5: AIPW is unbiased whatever its outcome model's quality. Over
    /// 10³ runs with a wrong model (m̂_L = 0.2, m̂_D = 0.9), the mean estimate
    /// is within 2 standard errors of the true β = 0.1.
    #[test]
    fn aipw_unbiased_with_misspecified_model() {
        const RUNS: u64 = 1_000;
        let mut estimates = Vec::new();
        for run in 0..RUNS {
            let mut rng = SplitMix64::new(run);
            let mut estimator = BenefitEstimator::new(BenefitConfig::default());
            for _ in 0..400 {
                let mut chain = draw_row(&mut rng);
                chain.pre.predictions = Some((0.2, 0.9));
                estimator.push(&chain);
            }
            estimates.push(estimator.aipw().mean);
        }
        let n = estimates.len() as f64;
        let mean = estimates.iter().sum::<f64>() / n;
        let variance = estimates.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let standard_error = (variance / n).sqrt();
        println!("AIPW over {RUNS} runs: mean {mean:.4} ± {standard_error:.4} (true 0.1)");
        assert!(
            (mean - 0.1).abs() <= 2.0 * standard_error,
            "{mean} ± {standard_error}"
        );
    }

    /// S03 §4.5: with a covariate that predicts the outcome, CUPED's
    /// increments vary less than the difference in means', so the variance
    /// reduction factor exceeds 1, and both estimate the same β.
    #[test]
    fn cuped_reduces_variance_on_synthetic() {
        let mut rng = SplitMix64::new(5113);
        let mut estimator = BenefitEstimator::new(BenefitConfig::default());
        for _ in 0..5_000 {
            let x = rng.next_f64();
            let learned = rng.next_f64() >= 0.2;
            let rate = 0.1 + 0.7 * x + if learned { 0.1 } else { 0.0 };
            let mut chain = row(learned, 0.2, rng.next_f64() < rate);
            chain.pre.covariate = Some(x);
            estimator.push(&chain);
        }
        let (dim, cuped) = (estimator.dim(), estimator.cuped());
        let factor = dim.variance / cuped.variance;
        println!("CUPED variance reduction factor {factor:.3}");
        assert!(factor > 1.0, "VRF {factor}");
        assert_eq!(dim.n, cuped.n);
        assert!(
            (cuped.mean - 0.1).abs() < 4.0 * cuped.standard_error(),
            "{cuped:?}"
        );
    }

    /// S03 §4.5's table: every increment stays within its declared bound at
    /// the worst case (q = 0.05, p_t = h_min = 0.02 under an all-off rate,
    /// audited false greens below the floor, extreme covariates and
    /// predictions); all-off rows feed only β_global; β_cost needs reported
    /// usage; pre-instrumentation rows are excluded.
    #[test]
    fn increments_within_declared_bounds() {
        let mut rng = SplitMix64::new(42);
        let mut estimator = BenefitEstimator::new(BenefitConfig::default());
        let g = 0.03;
        let logged_default = g + (1.0 - g) * 0.02;
        for i in 0..2_000_u32 {
            let learned = rng.next_f64() >= 0.02;
            let passed = rng.next_f64() < 0.7;
            let mut chain = row(learned, logged_default, passed);
            chain.global_rate = g;
            chain.audit = Some(AuditDraw {
                selected: rng.next_f64() < 0.5,
                pi: 0.01,
                false_green: passed && rng.next_f64() < 0.5,
            });
            chain.pre.covariate = Some(if i % 2 == 0 { 0.0 } else { 1.0 });
            chain.pre.predictions = Some((-0.5, 1.5));
            chain.pre.stratum = Some(format!("stratum-{}", i % 3));
            let source = if i % 4 == 0 {
                CostSource::Estimated
            } else {
                CostSource::CliUsage
            };
            chain.cost = Some((rng.next_f64() * 3.0, source));
            let increments = estimator.increments(&chain).expect("a loop row");
            let all = [
                Some(increments.dim),
                increments.cuped,
                increments.aipw,
                increments.post_stratified,
                Some(increments.pass),
                increments.cost,
            ];
            for increment in all.into_iter().flatten() {
                assert!(
                    increment.value.abs() <= increment.bound + 1e-9,
                    "{increment:?}"
                );
            }
            assert_eq!(increments.cost.is_some(), source.is_reported_usage());
            assert_eq!(estimator.push(&chain), Admission::Loop);
        }
        let (dim, cost) = (estimator.dim(), estimator.cost());
        assert_eq!(
            (dim.n, cost.n),
            (2_000, 1_500),
            "β_cost needs reported usage"
        );
        let worst = estimator.dim().interval.expect("an interval");
        assert!(worst.0 < worst.1);

        let mut all_off = row(false, logged_default, true);
        all_off.global_rate = g;
        all_off.global_off = true;
        let global_before = estimator.global().n;
        assert_eq!(estimator.push(&all_off), Admission::Global);
        assert_eq!(estimator.dim().n, 2_000, "an all-off row is not the loop's");
        assert_eq!(estimator.global().n, global_before + 1);

        let mut uninstrumented = row(true, logged_default, true);
        uninstrumented.outcome = ChainOutcome::PreInstrumentation;
        assert_eq!(estimator.push(&uninstrumented), Admission::Excluded);
        assert_eq!(estimator.global().n, global_before + 1);
    }
}
