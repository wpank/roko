//! L1: the per-rung online Bayesian logistic forecaster: backlog task 6113.
//!
//! Each gate rung r has an online logistic regression with a diagonal Laplace approximation,
//! offset by L0's logit (S04 §4.2):
//!
//! ```text
//! z = w_r·x + offset
//! v = Σ x_j²/(h_j + λ2)
//! d: d = v(y − σ(z + d))            the implicit step, p' = σ(z + d)
//! w_j += (y − p')x_j/(h_j + λ2)
//! h_j += p'(1 − p')x_j²
//! p_lcb = σ((z − Φ⁻¹(1 − δ)√v)/√(1 + πv/8))
//! ```
//!
//! The step is implicit (bug-bce0ed): the gradient is taken where the step lands, not where it
//! starts. An explicit step moves the score by about v(y − p), and a task text of dozens of
//! words is dozens of fresh `bow:` features, each adding about 1 to v, so one outcome threw the
//! score tens of logits past its target: 57 of 66 forecasts in Undertow's model ended below
//! 1e-6 or above 1 − 1e-6, and saturated p(1 − p) then kept h from growing. The implicit step
//! moves the forecast toward the outcome and never past it.
//!
//! λ2 is also the prior precision, so a feature never seen has variance 1/λ2. `p_gate` is the
//! product of the rungs' forecasts, which handles fail-fast censoring: a rung after the first
//! failure did not run and learns nothing. Each rung's offset is the logit of L0's forecast to
//! the power 1/R, so the rungs multiply back to L0 before they learn. The false-green head,
//! which turns `p_gate` into the routing target p_vs = p_gate·(1 − p_fg), was a logistic head
//! here until bug-7683aa; it is [`super::false_green`]'s pseudo-count rate now.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::features::FeatureVector;
use super::prior::{LCB_DELTA, normal_quantile};

/// The diagonal ridge, λ2 (S04 §4.10).
pub const LAMBDA2: f64 = 1.0;

/// The false-green head's prior, P(VS = 0 | gates pass), until VS labels exist
/// ([`super::false_green`]).
pub const FALSE_GREEN_PRIOR: f64 = 0.05;

/// The logistic function, computed without overflow.
#[must_use]
pub fn sigmoid(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}

/// The log-odds of `p`, clamped away from 0 and 1.
#[must_use]
pub fn logit(p: f64) -> f64 {
    let p = p.clamp(1e-9, 1.0 - 1e-9);
    (p / (1.0 - p)).ln()
}

/// One online Bayesian logistic regression with a diagonal Laplace approximation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OnlineLogit {
    /// The weight of each feature.
    weights: BTreeMap<String, f64>,
    /// Each feature's accumulated curvature, h.
    curvature: BTreeMap<String, f64>,
    /// The total weight of the outcomes learned.
    pub updates: f64,
}

/// A logistic forecast.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LogitForecast {
    /// The forecast probability.
    pub p: f64,
    /// Its lower confidence bound.
    pub lcb: f64,
}

impl OnlineLogit {
    /// The log-odds of `x` above `offset`.
    #[must_use]
    pub fn score(&self, x: &FeatureVector, offset: f64) -> f64 {
        let dot: f64 = x
            .pairs()
            .map(|(name, value)| self.weight(name) * value)
            .sum();
        offset + dot
    }

    /// The variance of the score of `x`: Σ x_j²/(h_j + λ2).
    #[must_use]
    pub fn variance(&self, x: &FeatureVector) -> f64 {
        x.pairs()
            .map(|(name, value)| {
                let h = self.curvature.get(name).copied().unwrap_or(0.0);
                value.powi(2) / (h + LAMBDA2)
            })
            .sum()
    }

    /// The forecast for `x` above `offset`, with its lower bound at level `delta`.
    #[must_use]
    pub fn forecast(&self, x: &FeatureVector, offset: f64, delta: f64) -> LogitForecast {
        let z = self.score(x, offset);
        let v = self.variance(x);
        let shrink = (1.0 + std::f64::consts::PI * v / 8.0).sqrt();
        LogitForecast {
            p: sigmoid(z),
            lcb: sigmoid((z - normal_quantile(1.0 - delta) * v.sqrt()) / shrink),
        }
    }

    /// Learn outcome `y` of weight `w` for `x` above `offset`, by the implicit step: the score
    /// moves by `d` where `d = w·v·(y − σ(z + d))`.
    pub fn update(&mut self, x: &FeatureVector, offset: f64, y: bool, w: f64) {
        let z = self.score(x, offset);
        let target = if y { 1.0 } else { 0.0 };
        let reach = w * self.variance(x);
        // f(d) = d − reach·(target − σ(z + d)) rises in d, and its root lies between 0 and
        // ±reach, toward the outcome.
        let (mut low, mut high) = if y { (0.0, reach) } else { (-reach, 0.0) };
        for _ in 0..64 {
            let mid = 0.5 * (low + high);
            if mid - reach * (target - sigmoid(z + mid)) > 0.0 {
                high = mid;
            } else {
                low = mid;
            }
        }
        let landed = sigmoid(z + 0.5 * (low + high));
        for (name, value) in x.pairs() {
            let h = self.curvature.entry(name.to_string()).or_insert(0.0);
            let step = w * (target - landed) * value / (*h + LAMBDA2);
            *h += w * landed * (1.0 - landed) * value * value;
            *self.weights.entry(name.to_string()).or_insert(0.0) += step;
        }
        self.updates += w;
    }

    /// The weight of feature `name`; 0 when it was never seen.
    #[must_use]
    pub fn weight(&self, name: &str) -> f64 {
        self.weights.get(name).copied().unwrap_or(0.0)
    }
}

/// L1's forecast for one candidate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GateForecast {
    /// P(every rung passes): the product of the rungs' forecasts.
    pub p_gate: f64,
    /// The product of the rungs' lower bounds.
    pub p_gate_lcb: f64,
}

/// The per-rung forecasters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateForecaster {
    /// One model per gate rung, keyed by rung name.
    rungs: BTreeMap<String, OnlineLogit>,
    /// The lower bounds' level, δ.
    pub delta: f64,
}

impl Default for GateForecaster {
    fn default() -> Self {
        Self {
            rungs: BTreeMap::new(),
            delta: LCB_DELTA,
        }
    }
}

impl GateForecaster {
    /// Each rung's offset: the logit of L0's `p_l0` to the power 1/`rungs`.
    fn rung_offset(p_l0: f64, rungs: usize) -> f64 {
        let rungs = rungs.max(1) as f64;
        logit(p_l0.clamp(1e-9, 1.0).powf(1.0 / rungs))
    }

    /// The forecast for `x` through the gate `rungs`, in the order they run, with L0's
    /// forecast `p_l0`.
    #[must_use]
    pub fn forecast(&self, x: &FeatureVector, p_l0: f64, rungs: &[&str]) -> GateForecast {
        let offset = Self::rung_offset(p_l0, rungs.len());
        let empty = OnlineLogit::default();
        let (mut p_gate, mut p_gate_lcb) = (1.0, 1.0);
        if rungs.is_empty() {
            let forecast = empty.forecast(x, offset, self.delta);
            (p_gate, p_gate_lcb) = (forecast.p, forecast.lcb);
        }
        for rung in rungs {
            let model = self.rungs.get(*rung).unwrap_or(&empty);
            let forecast = model.forecast(x, offset, self.delta);
            p_gate *= forecast.p;
            p_gate_lcb *= forecast.lcb;
        }
        GateForecast { p_gate, p_gate_lcb }
    }

    /// Learn one attempt: each gate rung's outcome in the order the rungs run (`None` for a
    /// rung that did not run), with its weight `w`. Rungs after the first failure are censored
    /// and learn nothing.
    pub fn observe(
        &mut self,
        x: &FeatureVector,
        p_l0: f64,
        rungs: &[(&str, Option<bool>)],
        w: f64,
    ) {
        let offset = Self::rung_offset(p_l0, rungs.len());
        for (rung, outcome) in rungs {
            let Some(outcome) = *outcome else {
                break;
            };
            self.rungs
                .entry((*rung).to_string())
                .or_default()
                .update(x, offset, outcome, w);
            if !outcome {
                break;
            }
        }
    }

    /// The model of `rung`, when it has learned.
    #[must_use]
    pub fn rung(&self, rung: &str) -> Option<&OnlineLogit> {
        self.rungs.get(rung)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::TestRng;

    /// Draw `n` outcomes of a logistic model with weights `truth` on standard normal features,
    /// learning each as it arrives.
    fn learn_synthetic(n: usize, seed: u64, truth: &[f64]) -> OnlineLogit {
        let mut rng = TestRng::new(seed);
        let mut model = OnlineLogit::default();
        for _ in 0..n {
            let x: Vec<f64> = truth.iter().map(|_| rng.normal()).collect();
            let z: f64 = truth.iter().zip(&x).map(|(w, x)| w * x).sum();
            let y = rng.bernoulli(sigmoid(z));
            let features = FeatureVector::from_pairs(
                x.iter()
                    .enumerate()
                    .map(|(j, value)| (format!("x{j}"), *value)),
            );
            model.update(&features, 0.0, y, 1.0);
        }
        model
    }

    #[test]
    fn l1_logit_recovers_synthetic_weights() {
        let truth = [1.0, -0.5, 0.25];
        let model = learn_synthetic(20_000, 11, &truth);
        for (j, expected) in truth.iter().enumerate() {
            let learned = model.weight(&format!("x{j}"));
            assert!((learned - expected).abs() < 0.1, "x{j}: {learned}");
        }
        // The more it has seen, the tighter its bound.
        let x = FeatureVector::from_pairs([("x0", 1.0)]);
        let early = learn_synthetic(50, 11, &truth).forecast(&x, 0.0, LCB_DELTA);
        let late = model.forecast(&x, 0.0, LCB_DELTA);
        assert!(late.p - late.lcb < early.p - early.lcb, "{late:?}");
    }

    #[test]
    fn censored_rungs_are_not_updated() {
        let mut gate = GateForecaster::default();
        let x = FeatureVector::from_pairs([("bias", 1.0)]);
        let failed_build = [("build", Some(false)), ("test", Some(true))];
        let unfinished = [("build", Some(true)), ("test", None)];
        let passed = [("build", Some(true)), ("test", Some(true))];
        gate.observe(&x, 0.5, &failed_build, 1.0);
        gate.observe(&x, 0.5, &unfinished, 1.0);
        assert_eq!(gate.rung("build").map(|model| model.updates), Some(2.0));
        assert!(gate.rung("test").is_none(), "censored, then not run");
        gate.observe(&x, 0.5, &passed, 1.0);
        assert_eq!(gate.rung("test").map(|model| model.updates), Some(1.0));
        // Before any rung learns, the rungs multiply back to L0's forecast.
        let fresh = GateForecaster::default().forecast(&x, 0.3, &["build", "test"]);
        assert!((fresh.p_gate - 0.3).abs() < 1e-9, "{fresh:?}");
    }
}
