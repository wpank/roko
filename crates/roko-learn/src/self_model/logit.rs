//! L1: the per-rung online Bayesian logistic forecaster and its false-green head: backlog
//! task 6113.
//!
//! Each gate rung r has an online logistic regression with a diagonal Laplace approximation,
//! offset by L0's logit (S04 §4.2):
//!
//! ```text
//! z = w_r·x + offset
//! h_j += p(1 − p)x_j²
//! w_j −= (p − y)x_j/(h_j + λ2)
//! v = Σ x_j²/(h_j + λ2)
//! p_lcb = σ((z − Φ⁻¹(1 − δ)√v)/√(1 + πv/8))
//! ```
//!
//! λ2 is also the prior precision, so a feature never seen has variance 1/λ2. `p_gate` is the
//! product of the rungs' forecasts, which handles fail-fast censoring: a rung after the first
//! failure did not run and learns nothing. Each rung's offset is the logit of L0's forecast to
//! the power 1/R, so the rungs multiply back to L0 before they learn. The false-green head
//! learns P(VS = 0 | gates pass) from VS-labelled passes only, weighted 1/π, and stays at its
//! prior until such labels exist. The routing target is p_vs = p_gate·(1 − p_fg).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::features::FeatureVector;
use super::prior::{LCB_DELTA, normal_quantile};

/// The diagonal ridge, λ2 (S04 §4.10).
pub const LAMBDA2: f64 = 1.0;

/// The false-green head's prior, P(VS = 0 | gates pass), until VS labels exist.
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

    /// Learn outcome `y` of weight `w` for `x` above `offset`.
    pub fn update(&mut self, x: &FeatureVector, offset: f64, y: bool, w: f64) {
        let p = sigmoid(self.score(x, offset));
        let error = p - if y { 1.0 } else { 0.0 };
        for (name, value) in x.pairs() {
            let h = self.curvature.entry(name.to_string()).or_insert(0.0);
            *h += w * p * (1.0 - p) * value * value;
            let step = w * error * value / (*h + LAMBDA2);
            *self.weights.entry(name.to_string()).or_insert(0.0) -= step;
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
    /// P(a pass is a false green).
    pub p_fg: f64,
    /// P(verified success) = `p_gate`·(1 − `p_fg`).
    pub p_vs: f64,
    /// `p_gate_lcb`·(1 − `p_fg`).
    pub p_vs_lcb: f64,
}

/// The per-rung forecasters and the false-green head.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateForecaster {
    /// One model per gate rung, keyed by rung name.
    rungs: BTreeMap<String, OnlineLogit>,
    /// The false-green head.
    false_green: OnlineLogit,
    /// The false-green prior.
    pub fg_prior: f64,
    /// The lower bounds' level, δ.
    pub delta: f64,
}

impl Default for GateForecaster {
    fn default() -> Self {
        Self {
            rungs: BTreeMap::new(),
            false_green: OnlineLogit::default(),
            fg_prior: FALSE_GREEN_PRIOR,
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
        let p_fg = sigmoid(self.false_green.score(x, logit(self.fg_prior)));
        GateForecast {
            p_gate,
            p_gate_lcb,
            p_fg,
            p_vs: p_gate * (1.0 - p_fg),
            p_vs_lcb: p_gate_lcb * (1.0 - p_fg),
        }
    }

    /// Learn one attempt: each gate rung's outcome in the order the rungs run (`None` for a
    /// rung that did not run), its VS label when audited (`y_vs`) and its weight `w`. Rungs
    /// after the first failure are censored and learn nothing; the false-green head learns
    /// only from a VS-labelled pass.
    pub fn observe(
        &mut self,
        x: &FeatureVector,
        p_l0: f64,
        rungs: &[(&str, Option<bool>)],
        y_vs: Option<bool>,
        w: f64,
    ) {
        let offset = Self::rung_offset(p_l0, rungs.len());
        let mut passed = !rungs.is_empty();
        for (rung, outcome) in rungs {
            let Some(outcome) = *outcome else {
                passed = false;
                break;
            };
            self.rungs
                .entry((*rung).to_string())
                .or_default()
                .update(x, offset, outcome, w);
            if !outcome {
                passed = false;
                break;
            }
        }
        if passed && let Some(verified) = y_vs {
            let offset = logit(self.fg_prior);
            self.false_green.update(x, offset, !verified, w);
        }
    }

    /// Learn the VS label of a pass whose rungs were learned before: the false-green head
    /// alone.
    pub fn observe_vs(&mut self, x: &FeatureVector, verified: bool, w: f64) {
        let offset = logit(self.fg_prior);
        self.false_green.update(x, offset, !verified, w);
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
        gate.observe(&x, 0.5, &failed_build, None, 1.0);
        gate.observe(&x, 0.5, &unfinished, None, 1.0);
        assert_eq!(gate.rung("build").map(|model| model.updates), Some(2.0));
        assert!(gate.rung("test").is_none(), "censored, then not run");
        gate.observe(&x, 0.5, &passed, None, 1.0);
        assert_eq!(gate.rung("test").map(|model| model.updates), Some(1.0));
        // Before any rung learns, the rungs multiply back to L0's forecast.
        let fresh = GateForecaster::default().forecast(&x, 0.3, &["build", "test"]);
        assert!((fresh.p_gate - 0.3).abs() < 1e-9, "{fresh:?}");
    }

    #[test]
    fn p_fg_stays_inert_without_vs_labels() {
        let mut gate = GateForecaster::default();
        let x = FeatureVector::from_pairs([("bias", 1.0), ("model:m", 1.0)]);
        for passed in [true, true, false, true] {
            gate.observe(&x, 0.6, &[("test", Some(passed))], None, 1.0);
        }
        let forecast = gate.forecast(&x, 0.6, &["test"]);
        let inert = (forecast.p_fg - FALSE_GREEN_PRIOR).abs() < 1e-12;
        assert!(inert, "{forecast:?}");
        let expected = forecast.p_gate * (1.0 - FALSE_GREEN_PRIOR);
        assert!((forecast.p_vs - expected).abs() < 1e-12);
        // A VS-labelled false green on a pass moves it; a failed gate's VS label does not.
        gate.observe(&x, 0.6, &[("test", Some(false))], Some(false), 1.0);
        let after_fail = gate.forecast(&x, 0.6, &["test"]);
        assert!((after_fail.p_fg - FALSE_GREEN_PRIOR).abs() < 1e-12);
        gate.observe(&x, 0.6, &[("test", Some(true))], Some(false), 4.0);
        let after_audit = gate.forecast(&x, 0.6, &["test"]);
        assert!(after_audit.p_fg > FALSE_GREEN_PRIOR, "{after_audit:?}");
        assert!(after_audit.p_vs_lcb <= after_audit.p_vs);
    }
}
