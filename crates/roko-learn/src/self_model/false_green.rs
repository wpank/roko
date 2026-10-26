//! The false-green head: P(VS = 0 | the gates pass) for a model, learned from S05's
//! VS-labelled passes weighted 1/π (S04 §4.2's `p_fg`; bug-7683aa).
//!
//! It is a Beta-binomial rate with a prior worth [`FALSE_GREEN_STRENGTH`] pseudo-passes at
//! [`FALSE_GREEN_PRIOR`]: a global rate, and one rate per model shrunk towards it, as L0's
//! strata are towards their parents:
//!
//! ```text
//! p_global = (N·p0 + k)/(N + n)
//! p_model  = (N·p_global + k_m)/(N + n_m)
//! on each VS-labelled pass of weight w: n += w; k += w·[VS = 0]
//! ```
//!
//! It replaces L1's online logistic head. That head's first update on an audit label of
//! weight 20 moved each of a task's fifty-odd features by about 9.7, so once the model was
//! warm one audited false green put `p_fg` at 1.0 for every arm and P(VS) at 0. Here the
//! heaviest single label, a false green at weight 20, moves the global rate from 0.05 to
//! (N·p0 + 20)/(N + 20) = 0.095, and a run of them converges to the rate the labels show.
//!
//! The counts are not discounted. VS labels are rare, at most one per 1/π passes, and each
//! stands for up to 20 of them; discounting at L0's λ would also cap the rate a run of false
//! greens can reach, at about 0.73 with weight-20 labels.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::logit::FALSE_GREEN_PRIOR;
use super::prior::BetaCounts;

/// The head's prior weight, N, in pseudo-passes: 1/(π_floor·p0) = 400, with π_floor = 0.05,
/// S05's floor on an attempt's audit probability (`roko_gate::audit::policy::EPS_FLOOR`), and
/// p0 = [`FALSE_GREEN_PRIOR`]. That is as many passes as audits at the floor need to label one
/// false green at the prior's rate. So the heaviest single label, a false green at weight
/// 1/π_floor = 20, about doubles the global rate instead of saturating it.
pub const FALSE_GREEN_STRENGTH: f64 = 400.0;

/// No discounting: see the module doc.
const NO_FORGETTING: f64 = 1.0;

/// The false-green head.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FalseGreenHead {
    /// The prior's weight, N, in pseudo-passes.
    pub strength: f64,
    /// The prior's rate, p0.
    pub prior: f64,
    /// Every VS-labelled pass: their weight `n`, and `k` of it false greens.
    global: BetaCounts,
    /// The same per model, by the slug its provider launched.
    models: BTreeMap<String, BetaCounts>,
}

impl Default for FalseGreenHead {
    fn default() -> Self {
        Self {
            strength: FALSE_GREEN_STRENGTH,
            prior: FALSE_GREEN_PRIOR,
            global: BetaCounts::default(),
            models: BTreeMap::new(),
        }
    }
}

impl FalseGreenHead {
    /// P(VS = 0 | the gates pass) for `model`.
    #[must_use]
    pub fn forecast(&self, model: &str) -> f64 {
        let global = self.global.mean(self.prior, self.strength);
        self.models
            .get(model)
            .copied()
            .unwrap_or_default()
            .mean(global, self.strength)
    }

    /// Learn one VS-labelled pass of `model` with weight `w` (1/π): a false green when its VS
    /// label is 0.
    pub fn observe(&mut self, model: &str, false_green: bool, w: f64) {
        self.global.observe(false_green, w, NO_FORGETTING);
        self.models
            .entry(model.to_string())
            .or_default()
            .observe(false_green, w, NO_FORGETTING);
    }

    /// The weight of the VS-labelled passes learned.
    #[must_use]
    pub fn labels(&self) -> f64 {
        self.global.n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bug-7683aa: the head sits at its prior until a VS label comes. One false green at the
    /// heaviest weight, 20, moves the global rate to (400·0.05 + 20)/420 and its model's to
    /// (400·that + 20)/420, while a model with no labels stays at the global rate. A run of
    /// false greens converges towards 1, and a run of verified passes back towards 0.
    #[test]
    fn the_head_moves_by_the_weight_against_its_prior() {
        let mut head = FalseGreenHead::default();
        assert!((head.forecast("mid") - FALSE_GREEN_PRIOR).abs() < 1e-12);
        head.observe("mid", true, 20.0);
        let global = 40.0 / 420.0;
        assert!((head.forecast("strong") - global).abs() < 1e-12);
        let mid = 400.0_f64.mul_add(global, 20.0) / 420.0;
        assert!((head.forecast("mid") - mid).abs() < 1e-12);
        assert!(head.forecast("mid") < 0.15, "{}", head.forecast("mid"));
        for _ in 0..200 {
            head.observe("mid", true, 20.0);
        }
        assert!(head.forecast("mid") > 0.95, "{}", head.forecast("mid"));
        for _ in 0..2_000 {
            head.observe("mid", false, 20.0);
        }
        assert!(head.forecast("mid") < 0.15, "{}", head.forecast("mid"));
        assert!((head.labels() - 2_201.0 * 20.0).abs() < 1e-6);
    }
}
