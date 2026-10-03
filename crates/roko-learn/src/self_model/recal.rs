//! Cross-fitted isotonic recalibration of forecasts: backlog task 6115.
//!
//! An online logistic model can rank attempts well and still be miscalibrated. S04 §4.2
//! recalibrates with isotonic regression (pool-adjacent-violators), refitted every 25 outcomes
//! on the last 200. The fit is cross-fitted over two folds, outcomes alternating between them:
//! a forecast is mapped by the map fitted on the fold its own outcome will not join, so no
//! outcome recalibrates its own forecast. Each fold's map must also earn its place: it is used
//! only while it scores a lower Brier than the raw forecasts on the other fold's outcomes, so a
//! forecaster that is already calibrated is not made noisier. Below 30 outcomes the map is the
//! identity.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::metrics::mean_square;

/// Outcomes between refits.
pub const REFIT_EVERY: usize = 25;
/// The outcomes a refit uses: the most recent ones.
pub const WINDOW: usize = 200;
/// Below this many outcomes, forecasts pass through unchanged.
pub const MIN_OUTCOMES: usize = 30;

/// A monotone map of [0, 1] onto itself: the pool-adjacent-violators fit's blocks, between
/// whose (mean forecast, mean outcome) knots it interpolates linearly.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IsotonicMap {
    /// The blocks' (mean forecast, mean outcome), both increasing.
    knots: Vec<(f64, f64)>,
}

impl IsotonicMap {
    /// The isotonic fit of `(forecast, outcome, weight)` points.
    #[must_use]
    pub fn fit(points: &[(f64, bool, f64)]) -> Self {
        let mut sorted: Vec<&(f64, bool, f64)> =
            points.iter().filter(|point| point.2 > 0.0).collect();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Each block: (Σ w·p, Σ w·y, Σ w).
        let mut blocks: Vec<(f64, f64, f64)> = Vec::new();
        for &&(p, y, w) in &sorted {
            blocks.push((w * p, if y { w } else { 0.0 }, w));
            while blocks.len() > 1 {
                let last = blocks[blocks.len() - 1];
                let before = blocks[blocks.len() - 2];
                if before.1 / before.2 <= last.1 / last.2 {
                    break;
                }
                blocks.pop();
                let merged = blocks.last_mut().expect("two blocks were there");
                merged.0 += last.0;
                merged.1 += last.1;
                merged.2 += last.2;
            }
        }
        Self {
            knots: blocks
                .into_iter()
                .map(|(sum_p, sum_y, weight)| (sum_p / weight, (sum_y / weight).clamp(0.0, 1.0)))
                .collect(),
        }
    }

    /// The recalibrated forecast for `p`; `p` itself when the map has no knots.
    #[must_use]
    pub fn apply(&self, p: f64) -> f64 {
        let (Some(first), Some(last)) = (self.knots.first(), self.knots.last()) else {
            return p;
        };
        if p <= first.0 {
            return first.1;
        }
        if p >= last.0 {
            return last.1;
        }
        for pair in self.knots.windows(2) {
            let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
            if x0 <= p && p <= x1 {
                if x1 == x0 {
                    return y1;
                }
                return y0 + (y1 - y0) * (p - x0) / (x1 - x0);
            }
        }
        p
    }

    /// Whether the map never decreases.
    #[must_use]
    pub fn is_monotone(&self) -> bool {
        self.knots
            .windows(2)
            .all(|pair| pair[0].0 <= pair[1].0 && pair[0].1 <= pair[1].1)
    }
}

/// The cross-fitted, refitted recalibrator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recalibrator {
    /// The most recent outcomes: (raw forecast, outcome, weight, fold).
    window: VecDeque<(f64, bool, f64, usize)>,
    /// Outcomes seen in all.
    pub outcomes: usize,
    /// Each fold's map, once fitted.
    maps: [Option<IsotonicMap>; 2],
    /// Whether each fold's map beat the raw forecasts on the other fold.
    accepted: [bool; 2],
}

impl Default for Recalibrator {
    fn default() -> Self {
        Self {
            window: VecDeque::with_capacity(WINDOW + 1),
            outcomes: 0,
            maps: [None, None],
            accepted: [false, false],
        }
    }
}

impl Recalibrator {
    /// The recalibrated forecast for a raw forecast `p`, made before its outcome is known. The
    /// outcome will join fold `outcomes % 2`, so the other fold's map applies, when accepted.
    #[must_use]
    pub fn apply(&self, p: f64) -> f64 {
        if self.outcomes < MIN_OUTCOMES {
            return p;
        }
        let other = 1 - self.outcomes % 2;
        match &self.maps[other] {
            Some(map) if self.accepted[other] => map.apply(p),
            _ => p,
        }
    }

    /// Learn the outcome `y` of weight `w` of a raw forecast `p`, refitting on schedule.
    pub fn observe(&mut self, p: f64, y: bool, w: f64) {
        self.window.push_back((p, y, w, self.outcomes % 2));
        self.outcomes += 1;
        if self.window.len() > WINDOW {
            self.window.pop_front();
        }
        if self.outcomes >= MIN_OUTCOMES && self.outcomes % REFIT_EVERY == 0 {
            self.refit();
        }
    }

    /// Fit each fold's map and keep it only if it beats the raw forecasts on the other fold.
    fn refit(&mut self) {
        for fold in 0..2 {
            let points = |wanted: bool| -> Vec<(f64, bool, f64)> {
                self.window
                    .iter()
                    .filter(|point| (point.3 == fold) == wanted)
                    .map(|&(p, y, w, _)| (p, y, w))
                    .collect()
            };
            let (fit, held_out) = (points(true), points(false));
            let map = IsotonicMap::fit(&fit);
            let outcome = |y: bool| if y { 1.0 } else { 0.0 };
            let raw = mean_square(held_out.iter().map(|&(p, y, w)| (p - outcome(y), w)));
            let mapped = mean_square(
                held_out
                    .iter()
                    .map(|&(p, y, w)| (map.apply(p) - outcome(y), w)),
            );
            self.accepted[fold] =
                matches!((mapped, raw), (Some(mapped), Some(raw)) if mapped < raw);
            self.maps[fold] = Some(map);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::TestRng;
    use crate::self_model::features::FeatureVector;
    use crate::self_model::logit::{OnlineLogit, sigmoid};
    use crate::self_model::metrics::{ECE_BINS, Scored, ece};

    #[test]
    fn prequential_ece_below_003_after_isotonic() {
        let mut rng = TestRng::new(3);
        let mut model = OnlineLogit::default();
        let mut recal = Recalibrator::default();
        let mut scored = Vec::with_capacity(6_000);
        for _ in 0..6_000 {
            let (x0, x1, x2) = (rng.normal(), rng.normal(), rng.normal());
            let y = rng.bernoulli(sigmoid(x0 - 0.5 * x1 + 0.25 * x2 - 0.5));
            let features =
                FeatureVector::from_pairs([("bias", 1.0), ("x0", x0), ("x1", x1), ("x2", x2)]);
            let p = sigmoid(model.score(&features, 0.0));
            // Scored before the outcome is known: prequential.
            scored.push(Scored::new(recal.apply(p), y));
            model.update(&features, 0.0, y, 1.0);
            recal.observe(p, y, 1.0);
        }
        let ece = ece(&scored, ECE_BINS).expect("ece");
        assert!(ece < 0.03, "{ece}");
    }

    #[test]
    fn the_fitted_map_is_monotone() {
        let mut rng = TestRng::new(19);
        let points: Vec<(f64, bool, f64)> = (0..300)
            .map(|_| {
                let p = rng.uniform();
                (p, rng.bernoulli(p * p), 1.0)
            })
            .collect();
        let map = IsotonicMap::fit(&points);
        assert!(map.is_monotone(), "{map:?}");
        let mapped: Vec<f64> = (0..=20).map(|i| map.apply(f64::from(i) / 20.0)).collect();
        assert!(
            mapped.windows(2).all(|pair| pair[0] <= pair[1]),
            "{mapped:?}"
        );
        assert!(mapped.iter().all(|p| (0.0..=1.0).contains(p)));
        // Violators pool: an outcome order against the forecasts' becomes one flat block.
        let pooled = IsotonicMap::fit(&[(0.2, true, 1.0), (0.8, false, 1.0)]);
        assert_eq!(pooled.apply(0.1), 0.5);
        assert_eq!(pooled.apply(0.9), 0.5);
    }

    #[test]
    fn below_30_outcomes_forecasts_pass_through() {
        let mut recal = Recalibrator::default();
        for i in 0..(MIN_OUTCOMES - 1) {
            recal.observe(0.9, i % 3 == 0, 1.0);
            assert_eq!(recal.apply(0.9), 0.9);
            assert_eq!(recal.apply(0.123), 0.123);
        }
        // An overconfident forecaster is pulled towards its outcomes once the map earns it.
        for i in 0..200 {
            recal.observe(0.9, i % 3 == 0, 1.0);
        }
        assert!(recal.apply(0.9) < 0.6, "{}", recal.apply(0.9));
    }
}
