//! Attempt cost and latency forecasts from a Normal-Inverse-Gamma regression on log scale:
//! backlog task 6114.
//!
//! Token use varies up to 30× between runs of the same task (`bai2026spend`), so cost and
//! latency are forecast as distributions (S04 §4.2). Each is a conjugate Normal-Inverse-Gamma
//! regression, on log(`api_equiv_usd`) and on log(latency in seconds), over the features
//! [1, arm one-hot, log(spec_tokens), family one-hot, n_declared_paths]. The quantiles come
//! from the Student-t predictive, whose quantile uses L0's incomplete beta. A row whose cost is
//! unknown, or whose cost source is `unknown`, is skipped and counted.
//!
//! An arm a regression has not observed has no forecast of its own: its coefficient is still
//! the vague prior, and the exp of the predictive's q90 reached 6e26 USD and seconds. Such an
//! arm gets a log-normal prior instead, flagged as one ([`CostModel::forecast`]; bug-09173e):
//! its cost around the price snapshot's price of a [`TYPICAL_ATTEMPT`] on its model, its
//! latency around what the observed attempts took.

use roko_core::pricing_snapshot::TokenCounts;
use serde::{Deserialize, Serialize};

use super::ArmKey;
use super::features::FeatureVector;
use super::prior::{beta_quantile, normal_quantile};
use crate::telemetry::CostSource;

/// A typical agent attempt's tokens, the basis of an unobserved arm's cost prior (bug-09173e):
/// about 30k input tokens, four fifths of them read from the prompt cache, and 3k output
/// tokens, the median attempt of the round-v6 live runs (36k input, 3.3k output), rounded. One
/// basis for every role, since only implementer attempts were measured.
pub const TYPICAL_ATTEMPT: TokenCounts = TokenCounts {
    input: 6_000,
    cache_read: 24_000,
    cache_write_5m: 0,
    cache_write_1h: 0,
    output: 3_000,
    reasoning: 0,
};

/// The log-scale spread σ of an unobserved arm's cost and latency prior: a log-normal whose
/// central 90% spans exp(2 · 1.645 · σ) ≈ 37×, the "up to 30×" by which token use varies
/// between runs of one task (`bai2026spend`). Its q90 is exp(1.2816 · σ) ≈ 4.1 times its
/// median.
pub const PRIOR_LOG_SD: f64 = 1.1;

/// An unobserved arm's median latency while no attempt's latency has been observed, in
/// seconds: a minute, above the 7 to 56 s the round-v6 live API attempts took, for slower
/// agents.
pub const PRIOR_LATENCY_S: f64 = 60.0;

/// The prior precision of every coefficient, κ (Λ0 = κI): a weak prior around 0.
pub const PRIOR_PRECISION: f64 = 0.01;
/// The inverse-gamma prior's shape, a0.
pub const PRIOR_SHAPE: f64 = 1.0;
/// The inverse-gamma prior's scale, b0.
pub const PRIOR_SCALE: f64 = 1.0;

/// The `p` quantile of Student's t with `dof` degrees of freedom, from the incomplete beta:
/// for p > 1/2, t = √(ν(1 − x)/x) with x = BetaQ(2(1 − p); ν/2, 1/2).
pub fn student_t_quantile(p: f64, dof: f64) -> f64 {
    if p < 0.5 {
        return -student_t_quantile(1.0 - p, dof);
    }
    if p == 0.5 {
        return 0.0;
    }
    let x = beta_quantile(2.0 * (1.0 - p), dof / 2.0, 0.5);
    (dof * (1.0 - x) / x).sqrt()
}

/// A Student-t predictive distribution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StudentT {
    /// The location.
    pub location: f64,
    /// The scale.
    pub scale: f64,
    /// The degrees of freedom.
    pub dof: f64,
}

impl StudentT {
    /// The `p` quantile.
    #[must_use]
    pub fn quantile(&self, p: f64) -> f64 {
        self.location + student_t_quantile(p, self.dof) * self.scale
    }
}

/// A conjugate Normal-Inverse-Gamma Bayesian linear regression over named features. A feature
/// gets a dimension when it is first seen; one never seen keeps its prior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NigRegression {
    /// Each dimension's feature name.
    names: Vec<String>,
    /// Σ w·x·xᵀ.
    xtx: Vec<Vec<f64>>,
    /// Σ w·x·y.
    xty: Vec<f64>,
    /// Σ w·y².
    yty: f64,
    /// Σ w: the weight of the rows learned.
    pub n: f64,
    /// κ.
    pub prior_precision: f64,
    /// a0.
    pub prior_shape: f64,
    /// b0.
    pub prior_scale: f64,
}

impl Default for NigRegression {
    fn default() -> Self {
        Self {
            names: Vec::new(),
            xtx: Vec::new(),
            xty: Vec::new(),
            yty: 0.0,
            n: 0.0,
            prior_precision: PRIOR_PRECISION,
            prior_shape: PRIOR_SHAPE,
            prior_scale: PRIOR_SCALE,
        }
    }
}

impl NigRegression {
    /// The dimension of feature `name`, added when new.
    fn dimension(&mut self, name: &str) -> usize {
        if let Some(index) = self.names.iter().position(|known| known == name) {
            return index;
        }
        self.names.push(name.to_string());
        for row in &mut self.xtx {
            row.push(0.0);
        }
        self.xtx.push(vec![0.0; self.names.len()]);
        self.xty.push(0.0);
        self.names.len() - 1
    }

    /// Learn target `y` of weight `w` for `x`.
    pub fn observe(&mut self, x: &FeatureVector, y: f64, w: f64) {
        let entries: Vec<(usize, f64)> = x
            .pairs()
            .map(|(name, value)| (self.dimension(name), value))
            .collect();
        for &(i, xi) in &entries {
            self.xty[i] += w * xi * y;
            for &(j, xj) in &entries {
                self.xtx[i][j] += w * xi * xj;
            }
        }
        self.yty += w * y * y;
        self.n += w;
    }

    /// Whether rows of positive weight have set feature `name` to a value other than 0, so its
    /// coefficient is more than its prior.
    fn observed(&self, name: &str) -> bool {
        self.names
            .iter()
            .position(|known| known == name)
            .is_some_and(|index| self.xtx[index][index] > 0.0)
    }

    /// The weighted mean of the targets learned, read from the constant feature `constant`
    /// that every row sets to 1 (Σ w·1·y / Σ w); `None` before any row.
    fn target_mean(&self, constant: &str) -> Option<f64> {
        let index = self.names.iter().position(|known| known == constant)?;
        (self.n > 0.0).then(|| self.xty[index] / self.n)
    }

    /// Λn = κI + Σ w·x·xᵀ.
    fn precision(&self) -> Vec<Vec<f64>> {
        let mut precision = self.xtx.clone();
        for (i, row) in precision.iter_mut().enumerate() {
            row[i] += self.prior_precision;
        }
        precision
    }

    /// The predictive distribution of the target for `x`.
    #[must_use]
    pub fn predictive(&self, x: &FeatureVector) -> StudentT {
        let precision = self.precision();
        let mean = solve(&precision, &self.xty).unwrap_or_else(|| vec![0.0; self.names.len()]);
        let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        let fit: f64 = mean
            .iter()
            .zip(&precision)
            .map(|(mi, row)| mi * dot(row, &mean))
            .sum();
        let shape = self.prior_shape + self.n / 2.0;
        let scale = (self.prior_scale + 0.5 * (self.yty - fit)).max(self.prior_scale * 1e-9);

        // xᵀΛn⁻¹x: a feature with no dimension keeps its prior precision κ.
        let mut dense = vec![0.0; self.names.len()];
        let mut unseen = 0.0;
        for (name, value) in x.pairs() {
            match self.names.iter().position(|known| known == name) {
                Some(index) => dense[index] = value,
                None => unseen += value * value / self.prior_precision,
            }
        }
        let location = dot(&dense, &mean);
        let spread = solve(&precision, &dense).map_or(0.0, |v| dot(&dense, &v));
        StudentT {
            location,
            scale: (scale / shape * (1.0 + spread + unseen)).sqrt(),
            dof: 2.0 * shape,
        }
    }
}

/// Solve `a`·v = `b` by Gauss-Jordan elimination with partial pivoting; `None` when `a` is
/// singular.
fn solve(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    let mut rows: Vec<Vec<f64>> = a
        .iter()
        .zip(b)
        .map(|(row, bi)| row.iter().copied().chain([*bi]).collect())
        .collect();
    for column in 0..n {
        let magnitude = |row: usize| rows[row][column].abs();
        let pivot = (column..n).max_by(|&i, &j| magnitude(i).total_cmp(&magnitude(j)))?;
        rows.swap(column, pivot);
        if rows[column][column].abs() < 1e-300 {
            return None;
        }
        for row in 0..n {
            if row != column {
                let factor = rows[row][column] / rows[column][column];
                for k in column..=n {
                    let step = factor * rows[column][k];
                    rows[row][k] -= step;
                }
            }
        }
    }
    Some((0..n).map(|i| rows[i][n] / rows[i][i]).collect())
}

/// An attempt's cost and latency forecast.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CostForecast {
    /// The median cost, in USD at the price snapshot.
    pub cost_q50: f64,
    /// The 90th-percentile cost.
    pub cost_q90: f64,
    /// The median latency, in seconds.
    pub lat_q50_s: f64,
    /// The 90th-percentile latency, in seconds.
    pub lat_q90_s: f64,
    /// The cost quantiles are the prior's: no cost of the arm has been observed (bug-09173e).
    #[serde(default)]
    pub cost_prior: bool,
    /// The latency quantiles are the prior's: no latency of the arm has been observed.
    #[serde(default)]
    pub latency_prior: bool,
}

/// The constant feature every row of the cost model sets.
const BIAS: &str = "bias";

/// The median and 90th percentile of an unobserved arm's log-normal prior around `median`,
/// spread [`PRIOR_LOG_SD`].
fn prior_quantiles(median: f64) -> (f64, f64) {
    let q90 = median * (normal_quantile(0.9) * PRIOR_LOG_SD).exp();
    (median, q90)
}

/// The cost and latency regressions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CostModel {
    /// The regression on log(`api_equiv_usd`).
    cost: NigRegression,
    /// The regression on log(latency in seconds).
    latency: NigRegression,
    /// Rows whose cost was unknown, or whose cost source was `unknown`: skipped.
    pub skipped_costs: usize,
}

impl CostModel {
    /// The feature that names `arm`.
    fn arm_feature(arm: &ArmKey) -> String {
        format!("arm:{arm}")
    }

    /// The features of an attempt of `arm` on a task of `family`: [1, arm one-hot,
    /// log(spec_tokens), family one-hot, n_declared_paths]. An unknown count is 0.
    #[must_use]
    pub fn features(
        arm: &ArmKey,
        family: &str,
        spec_tokens: Option<f64>,
        declared_paths: Option<f64>,
    ) -> FeatureVector {
        let log_tokens = spec_tokens.unwrap_or(0.0).max(0.0).ln_1p();
        FeatureVector::from_pairs([
            (BIAS.to_string(), 1.0),
            (Self::arm_feature(arm), 1.0),
            ("log_spec_tokens".to_string(), log_tokens),
            (format!("family:{family}"), 1.0),
            (
                "n_declared_paths".to_string(),
                declared_paths.unwrap_or(0.0),
            ),
        ])
    }

    /// Learn an attempt with features `x`, its cost `api_equiv_usd` from `cost_source`, its
    /// latency and weight `w`. An unknown cost is skipped and counted.
    pub fn observe(
        &mut self,
        x: &FeatureVector,
        api_equiv_usd: Option<f64>,
        cost_source: CostSource,
        latency_s: Option<f64>,
        w: f64,
    ) {
        match api_equiv_usd.filter(|cost| *cost > 0.0) {
            Some(cost) if cost_source != CostSource::Unknown => {
                self.cost.observe(x, cost.ln(), w);
            }
            _ => self.skipped_costs += 1,
        }
        if let Some(latency) = latency_s.filter(|latency| *latency > 0.0) {
            self.latency.observe(x, latency.ln(), w);
        }
    }

    /// The forecast for an attempt of `arm` with features `x`. A regression that has observed
    /// the arm forecasts from its Student-t predictive. One that has not cannot: the arm's
    /// coefficient is still its prior, N(0, σ²/κ), and with κ = 0.01 the exp of the
    /// predictive's q90 reached 6e26 USD and seconds. The arm's quantiles then come from a
    /// log-normal prior of spread [`PRIOR_LOG_SD`], flagged as the prior's (bug-09173e): cost
    /// around `prior_cost_usd`, the price snapshot's price of a [`TYPICAL_ATTEMPT`] on its model,
    /// and latency around the geometric mean of the latencies observed, else
    /// [`PRIOR_LATENCY_S`].
    #[must_use]
    pub fn forecast(&self, arm: &ArmKey, x: &FeatureVector, prior_cost_usd: f64) -> CostForecast {
        let name = Self::arm_feature(arm);
        let (cost_q50, cost_q90, cost_prior) = if self.cost.observed(&name) {
            let cost = self.cost.predictive(x);
            (cost.quantile(0.5).exp(), cost.quantile(0.9).exp(), false)
        } else {
            let (q50, q90) = prior_quantiles(prior_cost_usd);
            (q50, q90, true)
        };
        let (lat_q50_s, lat_q90_s, latency_prior) = if self.latency.observed(&name) {
            let latency = self.latency.predictive(x);
            (
                latency.quantile(0.5).exp(),
                latency.quantile(0.9).exp(),
                false,
            )
        } else {
            let observed = self.latency.target_mean(BIAS).map(f64::exp);
            let (q50, q90) = prior_quantiles(observed.unwrap_or(PRIOR_LATENCY_S));
            (q50, q90, true)
        };
        CostForecast {
            cost_q50,
            cost_q90,
            lat_q50_s,
            lat_q90_s,
            cost_prior,
            latency_prior,
        }
    }

    /// The weight of the costs learned.
    #[must_use]
    pub fn cost_rows(&self) -> f64 {
        self.cost.n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::TestRng;

    #[test]
    fn nig_quantiles_cover_lognormal_costs() {
        let means = [-4.0, -3.0, -2.0];
        let sigma = 0.8;
        let arm = |i: usize| FeatureVector::from_pairs([("bias", 1.0), (["a", "b", "c"][i], 1.0)]);
        let mut rng = TestRng::new(5);
        let mut model = NigRegression::default();
        for i in 0..3_000 {
            let y = means[i % 3] + sigma * rng.normal();
            model.observe(&arm(i % 3), y, 1.0);
        }
        let (mut below_median, mut below_q90) = (0, 0);
        for i in 0..3_000 {
            let predictive = model.predictive(&arm(i % 3));
            let cost = (means[i % 3] + sigma * rng.normal()).exp();
            if cost <= predictive.quantile(0.5).exp() {
                below_median += 1;
            }
            if cost <= predictive.quantile(0.9).exp() {
                below_q90 += 1;
            }
        }
        let median = f64::from(below_median) / 3_000.0;
        let q90 = f64::from(below_q90) / 3_000.0;
        assert!((median - 0.5).abs() <= 0.05, "{median}");
        assert!((q90 - 0.9).abs() <= 0.05, "{q90}");
        assert!((student_t_quantile(0.9, 10.0) - 1.372_183_6).abs() < 1e-6);
        assert!((student_t_quantile(0.1, 10.0) + 1.372_183_6).abs() < 1e-6);
    }

    /// With no data the regression's predictive is as wide as its prior is vague, so an arm no
    /// row has shown forecasts its bounded prior instead, flagged (bug-09173e). Once observed,
    /// the arm forecasts from the regression, which narrows with the data, while an arm still
    /// unobserved keeps its prior cost and takes the observed attempts' latency.
    #[test]
    fn an_unobserved_arm_forecasts_its_prior() {
        let arm = ArmKey::roko("cerebras", "gpt-oss-120b");
        let x = CostModel::features(&arm, "focused", None, None);
        let empty = CostModel::default();
        let vague = empty.cost.predictive(&x);
        let spread = (vague.quantile(0.9) - vague.quantile(0.5)).exp();
        assert!(spread > 100.0, "{vague:?}");
        let prior = empty.forecast(&arm, &x, 0.02);
        assert!(prior.cost_prior && prior.latency_prior, "{prior:?}");
        assert!((prior.cost_q50 - 0.02).abs() < 1e-12, "{prior:?}");
        let ratio = (normal_quantile(0.9) * PRIOR_LOG_SD).exp();
        assert!((ratio - 4.1).abs() < 0.05, "{ratio}");
        assert!(
            (prior.cost_q90 / prior.cost_q50 - ratio).abs() < 1e-9,
            "{prior:?}"
        );
        assert!(
            (prior.lat_q50_s - PRIOR_LATENCY_S).abs() < 1e-9,
            "{prior:?}"
        );

        let mut trained = CostModel::default();
        let mut rng = TestRng::new(3);
        for _ in 0..200 {
            let cost = (-4.0 + 0.5 * rng.normal()).exp();
            trained.observe(&x, Some(cost), CostSource::ProviderUsage, Some(90.0), 1.0);
        }
        let narrow = trained.forecast(&arm, &x, 0.02);
        assert!(!narrow.cost_prior && !narrow.latency_prior, "{narrow:?}");
        assert!(narrow.cost_q90 / narrow.cost_q50 < 3.0, "{narrow:?}");
        assert!(
            (narrow.cost_q50 - (-4.0_f64).exp()).abs() < 0.005,
            "{narrow:?}"
        );
        assert!((narrow.lat_q50_s - 90.0).abs() < 1.0, "{narrow:?}");

        let other = ArmKey::roko("zai", "glm-4.7");
        let y = CostModel::features(&other, "focused", None, None);
        let unseen = trained.forecast(&other, &y, 0.0128);
        assert!(unseen.cost_prior && unseen.latency_prior, "{unseen:?}");
        assert!((unseen.cost_q50 - 0.0128).abs() < 1e-12, "{unseen:?}");
        assert!((unseen.lat_q50_s - 90.0).abs() < 1e-6, "{unseen:?}");
    }

    #[test]
    fn unknown_costs_are_skipped() {
        let arm = ArmKey::roko("zai", "glm-4.7");
        let x = CostModel::features(&arm, "focused", Some(800.0), None);
        let mut model = CostModel::default();
        model.observe(&x, None, CostSource::ProviderUsage, None, 1.0);
        model.observe(&x, Some(0.02), CostSource::Unknown, None, 1.0);
        model.observe(&x, Some(0.02), CostSource::CliUsage, Some(30.0), 1.0);
        assert_eq!(model.skipped_costs, 2);
        assert_eq!(model.cost_rows(), 1.0);
    }
}
