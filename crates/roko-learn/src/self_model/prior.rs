//! L0: the discounted hierarchical Beta prior with backoff and a lower confidence bound:
//! backlog task 6112.
//!
//! The strata are (model), (model, family) and (model, family, role), under a global stratum
//! (S04 §4.2). Each stratum's mean shrinks towards its parent's:
//!
//! ```text
//! p̂_s = (α0·p̂_parent + k_s)/(α0 + n_s)
//! LCB_s = BetaQ(δ; α0·p̂_parent + k_s, α0·(1 − p̂_parent) + n_s − k_s)
//! on each outcome: n ← λn + w; k ← λk + w·y        # w = 1/π for an audit-only label
//! ```
//!
//! An empty stratum therefore backs off to its parent. L0 gives the cold-start forecast and
//! L1's offset. The special functions behind `BetaQ` ([`ln_gamma`], [`reg_inc_beta`],
//! [`beta_quantile`]) and [`normal_quantile`] are public, for the cost model's Student-t
//! quantiles and L1's lower bound.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The prior's weight in pseudo-outcomes, α0 (S04 §4.10).
pub const ALPHA0: f64 = 4.0;
/// The forgetting factor, λ: an outcome's weight halves in about 34 updates.
pub const FORGETTING: f64 = 0.98;
/// The lower confidence bound's level, δ.
pub const LCB_DELTA: f64 = 0.20;
/// The global stratum's own prior mean.
pub const GLOBAL_PRIOR: f64 = 0.5;

/// A stratum's discounted counts: `n` outcomes, `k` of them passes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BetaCounts {
    /// The discounted weight of the outcomes seen.
    pub n: f64,
    /// The discounted weight of the passes among them.
    pub k: f64,
}

impl BetaCounts {
    /// Discount the counts by `forgetting`, then add an outcome `y` of weight `w`.
    pub fn observe(&mut self, y: bool, w: f64, forgetting: f64) {
        self.n = forgetting * self.n + w;
        self.k = forgetting * self.k + if y { w } else { 0.0 };
    }

    /// The stratum's mean, shrunk towards `parent` with weight `alpha0`.
    #[must_use]
    pub fn mean(&self, parent: f64, alpha0: f64) -> f64 {
        (alpha0 * parent + self.k) / (alpha0 + self.n)
    }

    /// The stratum's lower confidence bound at level `delta`, around `parent`.
    #[must_use]
    pub fn lcb(&self, parent: f64, alpha0: f64, delta: f64) -> f64 {
        let a = alpha0 * parent + self.k;
        let b = alpha0 * (1.0 - parent) + self.n - self.k;
        beta_quantile(delta, a, b)
    }
}

/// L0's forecast for one (model, family, role).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct L0Forecast {
    /// The deepest stratum's mean.
    pub p: f64,
    /// Its lower confidence bound.
    pub lcb: f64,
}

/// The discounted hierarchical Beta prior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HierarchicalBeta {
    /// The prior's weight, α0.
    pub alpha0: f64,
    /// The forgetting factor, λ.
    pub forgetting: f64,
    /// The lower bound's level, δ.
    pub delta: f64,
    /// The global stratum's prior mean.
    pub global_prior: f64,
    /// The global stratum.
    global: BetaCounts,
    /// The other strata, keyed `model`, `model|family` and `model|family|role`.
    strata: BTreeMap<String, BetaCounts>,
}

impl Default for HierarchicalBeta {
    fn default() -> Self {
        Self {
            alpha0: ALPHA0,
            forgetting: FORGETTING,
            delta: LCB_DELTA,
            global_prior: GLOBAL_PRIOR,
            global: BetaCounts::default(),
            strata: BTreeMap::new(),
        }
    }
}

impl HierarchicalBeta {
    /// An empty prior with the S04 defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The keys of (model), (model, family) and (model, family, role).
    fn keys(model: &str, family: &str, role: &str) -> [String; 3] {
        [
            model.to_string(),
            format!("{model}|{family}"),
            format!("{model}|{family}|{role}"),
        ]
    }

    /// A stratum's counts; empty when it has seen nothing.
    fn counts(&self, key: &str) -> BetaCounts {
        self.strata.get(key).copied().unwrap_or_default()
    }

    /// The forecast for `model` on a task of `family` in `role`: each stratum's mean shrunk
    /// towards its parent's, and the deepest stratum's lower bound.
    #[must_use]
    pub fn forecast(&self, model: &str, family: &str, role: &str) -> L0Forecast {
        let global = self.global.mean(self.global_prior, self.alpha0);
        let [model_key, family_key, role_key] = Self::keys(model, family, role);
        let model_p = self.counts(&model_key).mean(global, self.alpha0);
        let family_p = self.counts(&family_key).mean(model_p, self.alpha0);
        let role_counts = self.counts(&role_key);
        L0Forecast {
            p: role_counts.mean(family_p, self.alpha0),
            lcb: role_counts.lcb(family_p, self.alpha0, self.delta),
        }
    }

    /// Learn an outcome `y` of weight `w` (1/π for an audit-only label) for `model` on a task
    /// of `family` in `role`: every stratum it belongs to, the global one included.
    pub fn observe(&mut self, model: &str, family: &str, role: &str, y: bool, w: f64) {
        self.global.observe(y, w, self.forgetting);
        for key in Self::keys(model, family, role) {
            self.strata
                .entry(key)
                .or_default()
                .observe(y, w, self.forgetting);
        }
    }

    /// The discounted weight of outcomes the global stratum has seen.
    #[must_use]
    pub fn outcomes(&self) -> f64 {
        self.global.n
    }
}

/// The natural log of the gamma function (Lanczos, g = 7), accurate to about 1e-13.
pub fn ln_gamma(x: f64) -> f64 {
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_81,
        676.520_368_121_885,
        -1_259.139_216_722_4,
        771.323_428_777_653,
        -176.615_029_162_141,
        12.507_343_278_686_9,
        -0.138_571_095_265_72,
        9.984_369_578_019_57e-6,
        1.505_632_735_149_31e-7,
    ];
    if x < 0.5 {
        // The reflection formula.
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin().abs()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let t = x + 7.5;
    let series = COEFFICIENTS[1..]
        .iter()
        .zip(1_u32..)
        .fold(COEFFICIENTS[0], |sum, (coefficient, i)| {
            sum + coefficient / (x + f64::from(i))
        });
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + series.ln()
}

/// The continued fraction of the incomplete beta function (modified Lentz).
fn beta_continued_fraction(a: f64, b: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    const EPSILON: f64 = 1e-15;
    let floor = |value: f64| if value.abs() < TINY { TINY } else { value };
    let (sum, plus, minus) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 / floor(1.0 - sum * x / plus);
    let mut fraction = d;
    for m in 1..=300_u32 {
        let m = f64::from(m);
        let two_m = 2.0 * m;
        let even = m * (b - m) * x / ((minus + two_m) * (a + two_m));
        d = 1.0 / floor(1.0 + even * d);
        c = floor(1.0 + even / c);
        fraction *= d * c;
        let odd = -(a + m) * (sum + m) * x / ((a + two_m) * (plus + two_m));
        d = 1.0 / floor(1.0 + odd * d);
        c = floor(1.0 + odd / c);
        let step = d * c;
        fraction *= step;
        if (step - 1.0).abs() < EPSILON {
            break;
        }
    }
    fraction
}

/// The regularised incomplete beta function I_x(a, b): the Beta(a, b) distribution function
/// at `x`.
pub fn reg_inc_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let ln_front = ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (-x).ln_1p();
    let front = ln_front.exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_continued_fraction(a, b, x) / a
    } else {
        1.0 - front * beta_continued_fraction(b, a, 1.0 - x) / b
    }
}

/// The `p` quantile of Beta(a, b), by bisection on [`reg_inc_beta`].
pub fn beta_quantile(p: f64, a: f64, b: f64) -> f64 {
    let (a, b) = (a.max(1e-12), b.max(1e-12));
    let p = p.clamp(0.0, 1.0);
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..200 {
        let middle = low + 0.5 * (high - low);
        if reg_inc_beta(a, b, middle) < p {
            low = middle;
        } else {
            high = middle;
        }
    }
    low + 0.5 * (high - low)
}

/// The standard normal quantile Φ⁻¹(p) (Acklam's rational approximation, accurate to about
/// 1e-9).
pub fn normal_quantile(p: f64) -> f64 {
    const A: [f64; 6] = [
        -39.696_830_286_653_8,
        220.946_098_424_521,
        -275.928_510_446_969,
        138.357_751_867_269,
        -30.664_798_066_147_2,
        2.506_628_277_459_24,
    ];
    const B: [f64; 5] = [
        -54.476_098_798_224_1,
        161.585_836_858_041,
        -155.698_979_859_887,
        66.801_311_887_719_7,
        -13.280_681_552_885_7,
    ];
    const C: [f64; 6] = [
        -0.007_784_894_002_430_29,
        -0.322_396_458_041_137,
        -2.400_758_277_161_84,
        -2.549_732_539_343_73,
        4.374_664_141_464_97,
        2.938_163_982_698_78,
    ];
    const D: [f64; 4] = [
        0.007_784_695_709_041_46,
        0.322_467_129_070_04,
        2.445_134_137_143,
        3.754_408_661_907_42,
    ];
    const TAIL: f64 = 0.024_25;
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    let polynomial = |coefficients: &[f64], x: f64| {
        coefficients
            .iter()
            .fold(0.0, |sum, coefficient| sum * x + coefficient)
    };
    let tail = |q: f64| polynomial(&C, q) / (polynomial(&D, q) * q + 1.0);
    if p < TAIL {
        tail((-2.0 * p.ln()).sqrt())
    } else if p > 1.0 - TAIL {
        -tail((-2.0 * (-p).ln_1p()).sqrt())
    } else {
        let q = p - 0.5;
        let r = q * q;
        polynomial(&A, r) * q / (polynomial(&B, r) * r + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_backs_off_to_parent_when_stratum_is_empty() {
        let mut prior = HierarchicalBeta::new();
        for y in [true, true, true, false, true, true] {
            prior.observe("gpt-oss-120b", "focused", "implementer", y, 1.0);
        }
        let seen = prior.forecast("gpt-oss-120b", "focused", "implementer");
        // A family the model has not run backs off to the model's stratum.
        let new_family = prior.forecast("gpt-oss-120b", "architectural", "implementer");
        let model_only = {
            let global = prior.global.mean(GLOBAL_PRIOR, ALPHA0);
            prior.counts("gpt-oss-120b").mean(global, ALPHA0)
        };
        assert!((new_family.p - model_only).abs() < 1e-12, "{new_family:?}");
        // A model never seen backs off to the global stratum.
        let unseen = prior.forecast("glm-4.7", "focused", "implementer");
        let global = prior.global.mean(GLOBAL_PRIOR, ALPHA0);
        assert!((unseen.p - global).abs() < 1e-12, "{unseen:?}");
        // Five passes in six pull the deepest stratum above the global prior, and the lower
        // bound sits below the mean.
        assert!(seen.p > GLOBAL_PRIOR && seen.p > new_family.p.min(unseen.p));
        assert!(seen.lcb < seen.p && unseen.lcb < unseen.p);
        // An empty prior forecasts its global prior mean.
        let empty = HierarchicalBeta::new().forecast("m", "f", "r");
        assert!((empty.p - GLOBAL_PRIOR).abs() < 1e-12);
    }

    #[test]
    fn lcb_matches_reference_beta_quantiles() {
        // An empty stratum under a parent at 0.25 is Beta(1, 3): BetaQ(p) = 1 − (1 − p)^(1/3).
        let lcb = BetaCounts::default().lcb(0.25, ALPHA0, LCB_DELTA);
        assert!((lcb - (1.0 - 0.8_f64.powf(1.0 / 3.0))).abs() < 1e-6, "{lcb}");
        for (p, a, b, reference) in [
            (0.2, 2.0, 1.0, 0.2_f64.sqrt()),
            (0.9, 4.0, 1.0, 0.9_f64.powf(0.25)),
            (0.3, 1.0, 3.0, 1.0 - 0.7_f64.powf(1.0 / 3.0)),
            (0.05, 1.0, 7.5, 1.0 - 0.95_f64.powf(1.0 / 7.5)),
            (0.2, 0.5, 0.5, (std::f64::consts::PI * 0.1).sin().powi(2)),
        ] {
            let quantile = beta_quantile(p, a, b);
            assert!(
                (quantile - reference).abs() < 1e-6,
                "Beta({a}, {b}) at {p}: {quantile}"
            );
        }
        // I_x(2, 3) = 6x²(1 − x)² + 4x³(1 − x) + x⁴.
        for x in [0.1, 0.35, 0.6, 0.9] {
            let closed =
                6.0 * x * x * (1.0 - x) * (1.0 - x) + 4.0 * x.powi(3) * (1.0 - x) + x.powi(4);
            assert!((reg_inc_beta(2.0, 3.0, x) - closed).abs() < 1e-9, "{x}");
        }
        assert!((normal_quantile(0.8) - 0.841_621_233_572_914).abs() < 1e-8);
        assert!((normal_quantile(0.025) + 1.959_963_984_540_05).abs() < 1e-8);
        assert!((ln_gamma(5.0) - 24.0_f64.ln()).abs() < 1e-12);
    }

    #[test]
    fn forgetting_halves_an_old_outcome_in_about_34_updates() {
        let mut counts = BetaCounts::default();
        counts.observe(true, 1.0, FORGETTING);
        for _ in 0..34 {
            counts.observe(false, 1.0, FORGETTING);
        }
        // Only the first outcome passed, so k is its remaining weight: 0.98^34.
        assert!((counts.k - 0.5).abs() < 0.01, "{counts:?}");
    }

    #[test]
    fn updates_carry_their_weight() {
        let mut counts = BetaCounts::default();
        counts.observe(true, 2.5, FORGETTING);
        assert_eq!(counts, BetaCounts { n: 2.5, k: 2.5 });
        counts.observe(false, 0.5, FORGETTING);
        assert!((counts.n - (0.98 * 2.5 + 0.5)).abs() < 1e-12);
        assert!((counts.k - 0.98 * 2.5).abs() < 1e-12);
        // An audit label of weight 1/π = 4 moves the prior as much as four plain ones.
        let mut audited = HierarchicalBeta::new();
        audited.observe("m", "f", "r", false, 4.0);
        let mut plain = HierarchicalBeta::new();
        for _ in 0..4 {
            plain.observe("m", "f", "r", false, 1.0);
        }
        let (a, b) = (audited.forecast("m", "f", "r"), plain.forecast("m", "f", "r"));
        assert!(a.p < GLOBAL_PRIOR && b.p < GLOBAL_PRIOR);
        assert!((a.p - b.p).abs() < 0.03, "{a:?} against {b:?}");
    }
}
