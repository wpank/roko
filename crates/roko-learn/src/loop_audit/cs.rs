//! Confidence sequences with per-step bounds, and the SRM e-value (S03 §4.5; backlog 5110).
//!
//! Pure math, no I/O. Two anytime-valid confidence sequences (CS) for the
//! mean of bounded increments, read after every step without losing
//! coverage (`howard2021timeuniform`):
//!
//! - [`EmpiricalBernsteinCs`]: the predictable plug-in empirical-Bernstein CS
//!   (`waudbysmith2020estimating`, Theorem 2), in closed form.
//! - [`BettingCs`]: the hedged capital process with predictable plug-in bets
//!   (Theorem 3) over a grid of candidate means, the construction of S08's
//!   toolkit (`analysis/cs.py` over `audit/estimate.py::hedged_capital`), so
//!   that backlog 5111 can check one against the other.
//!
//! Every increment x_t carries its own bound B_t ≥ |x_t|, fixed before its
//! arm is drawn (S03 §4.5's table). Each step is rescaled by its own bound,
//! y_t = (x_t + B_t)/(2B_t) ∈ [0, 1], so a bet never exceeds 1/(2B_t) in the
//! increment's units. [`DifferenceCs`] runs a betting CS over the two-arm
//! IPW increments of [`ipw_difference`] at logged propensities, and
//! [`SrmEvalue`] is the sequential multinomial e-value on arm counts against
//! logged propensities (`lindon2020anytimevalid`). The Bonferroni split α/K
//! is the caller's.

/// Grid steps of a betting CS over its range, as in S08's toolkit.
pub const CS_GRID: usize = 1000;

/// The bet truncation c of Waudby-Smith and Ramdas: a capital factor never
/// drops below 1 − c.
pub const CS_C: f64 = 0.5;

/// Weight of the upward capital process; the downward one gets the rest.
pub const CS_HEDGE: f64 = 0.5;

/// The predictable plug-in bet λ̇_t = √(2·log(2/α) / (σ̂²_{t−1}·t·log(t + 1)))
/// of WSR 2024, where σ̂²_{t−1}·t = 1/4 + Σ_{i<t} (y_i − μ̂_i)² and
/// μ̂_i = (1/2 + Σ_{j≤i} y_j)/(i + 1). It reads past scaled values only,
/// never the mean under test, so every candidate mean shares it.
#[derive(Debug, Clone)]
struct PlugIn {
    log_term: f64,
    t: u64,
    total: f64,
    squares: f64,
}

impl PlugIn {
    fn new(alpha: f64) -> Self {
        Self {
            log_term: 2.0 * (2.0 / alpha).ln(),
            t: 1,
            total: 0.5,
            squares: 0.25,
        }
    }

    /// λ̇_t for the next value.
    fn bet(&self) -> f64 {
        (self.log_term / (self.squares * (self.t as f64 + 1.0).ln())).sqrt()
    }

    /// μ̂_{t−1}, the regularized mean of the values so far.
    fn mean(&self) -> f64 {
        self.total / self.t as f64
    }

    /// Take the next scaled value.
    fn observe(&mut self, y: f64) {
        self.total += y;
        self.squares += (y - self.total / (self.t + 1) as f64).powi(2);
        self.t += 1;
    }
}

/// The upward bet against mean `m`, truncated so the factor stays ≥ 1 − c.
fn up_bet(bet: f64, m: f64) -> f64 {
    if m == 0.0 { bet } else { bet.min(CS_C / m) }
}

/// The downward bet against mean `m`, truncated the same way.
fn down_bet(bet: f64, m: f64) -> f64 {
    if m == 1.0 { bet } else { bet.min(CS_C / (1.0 - m)) }
}

/// One step of the hedged capital against scaled mean `m`: update both
/// processes with scaled value `y` and return K_t^± = max(h·K^+, (1 − h)·K^−).
fn hedged_step(up: &mut f64, down: &mut f64, bet: f64, y: f64, m: f64) -> f64 {
    *up *= 1.0 + up_bet(bet, m) * (y - m);
    *down *= 1.0 - down_bet(bet, m) * (y - m);
    (CS_HEDGE * *up).max((1.0 - CS_HEDGE) * *down)
}

/// `x` with bound `bound` scaled to `[0, 1]`, and the step's support.
fn scaled(x: f64, bound: f64) -> (f64, f64, f64) {
    let (lo, hi) = (-bound, bound);
    (((x - lo) / (hi - lo)).clamp(0.0, 1.0), lo, hi)
}

/// The betting CS for the mean of bounded increments over a grid of
/// candidate means on `[lo, hi]` (WSR 2024, Theorem 3). A candidate leaves
/// for good once its hedged capital reaches 1/α; the interval is the hull of
/// the candidates still inside, widened by one grid step on each side and
/// clipped to the range, so it contains the exact running intersection.
#[derive(Debug, Clone)]
pub struct BettingCs {
    alpha: f64,
    lo: f64,
    hi: f64,
    grid: usize,
    plug_in: PlugIn,
    up: Vec<f64>,
    down: Vec<f64>,
    inside: Vec<bool>,
    steps: u64,
}

impl BettingCs {
    /// A CS at level `alpha` for a mean known to lie in `[lo, hi]`, on
    /// [`CS_GRID`] steps.
    #[must_use]
    pub fn new(alpha: f64, lo: f64, hi: f64) -> Self {
        Self::with_grid(alpha, lo, hi, CS_GRID)
    }

    /// [`Self::new`] on `grid` steps.
    #[must_use]
    pub fn with_grid(alpha: f64, lo: f64, hi: f64, grid: usize) -> Self {
        let points = grid.max(1) + 1;
        Self {
            alpha,
            lo,
            hi,
            grid: grid.max(1),
            plug_in: PlugIn::new(alpha),
            up: vec![1.0; points],
            down: vec![1.0; points],
            inside: vec![true; points],
            steps: 0,
        }
    }

    /// Take increment `x` with its bound `bound` ≥ |x|. A bound below the
    /// range is widened to it, which only weakens the bet; a value outside
    /// its bound is clamped to it.
    pub fn push(&mut self, x: f64, bound: f64) {
        let bound = bound.max(self.lo.abs()).max(self.hi.abs());
        let (y, step_lo, step_hi) = scaled(x, bound);
        // When the step's support is the range, a candidate's scaled mean
        // is its grid fraction exactly, as in S08's toolkit.
        let exact = step_lo == self.lo && step_hi == self.hi;
        let bet = self.plug_in.bet();
        let threshold = 1.0 / self.alpha;
        for j in 0..=self.grid {
            if !self.inside[j] {
                continue;
            }
            let fraction = j as f64 / self.grid as f64;
            let m = if exact {
                fraction
            } else {
                (self.lo + fraction * (self.hi - self.lo) - step_lo) / (step_hi - step_lo)
            };
            let capital = hedged_step(&mut self.up[j], &mut self.down[j], bet, y, m);
            if capital >= threshold {
                self.inside[j] = false;
            }
        }
        self.plug_in.observe(y);
        self.steps += 1;
    }

    /// The interval after the steps so far; `None` once no candidate is
    /// left.
    #[must_use]
    pub fn interval(&self) -> Option<(f64, f64)> {
        let first = self.inside.iter().position(|inside| *inside)?;
        let last = self.inside.iter().rposition(|inside| *inside)?;
        let low = first.saturating_sub(1) as f64 / self.grid as f64;
        let high = (last + 1).min(self.grid) as f64 / self.grid as f64;
        let span = self.hi - self.lo;
        Some((self.lo + low * span, self.lo + high * span))
    }

    /// The interval's width; `None` once no candidate is left.
    #[must_use]
    pub fn width(&self) -> Option<f64> {
        self.interval().map(|(low, high)| high - low)
    }

    /// Increments taken.
    #[must_use]
    pub const fn steps(&self) -> u64 {
        self.steps
    }
}

/// The predictable plug-in empirical-Bernstein CS (WSR 2020, Theorem 2):
/// the center Σ λ̃_i x_i / Σ λ̃_i and the radius
/// (log(2/α) + Σ v_i ψ_E(λ_i)) / Σ λ̃_i, where λ_i = min(λ̇_i, c) bets on the
/// scaled value, λ̃_i = λ_i/(2B_i) in the increment's units,
/// v_i = 4(y_i − μ̂_{i−1})² and ψ_E(λ) = (−log(1 − λ) − λ)/4.
#[derive(Debug, Clone)]
pub struct EmpiricalBernsteinCs {
    log_term: f64,
    plug_in: PlugIn,
    weights: f64,
    weighted: f64,
    penalty: f64,
}

impl EmpiricalBernsteinCs {
    /// A CS at level `alpha`.
    #[must_use]
    pub fn new(alpha: f64) -> Self {
        Self {
            log_term: (2.0 / alpha).ln(),
            plug_in: PlugIn::new(alpha),
            weights: 0.0,
            weighted: 0.0,
            penalty: 0.0,
        }
    }

    /// Take increment `x` with its bound `bound` ≥ |x|.
    pub fn push(&mut self, x: f64, bound: f64) {
        let (y, lo, hi) = scaled(x, bound);
        let lambda = self.plug_in.bet().min(CS_C);
        let deviation = y - self.plug_in.mean();
        let v = 4.0 * deviation * deviation;
        let psi = (-(1.0 - lambda).ln() - lambda) / 4.0;
        let weight = lambda / (hi - lo);
        self.weights += weight;
        self.weighted += weight * x.clamp(lo, hi);
        self.penalty += v * psi;
        self.plug_in.observe(y);
    }

    /// The interval after the steps so far; `None` before the first.
    #[must_use]
    pub fn interval(&self) -> Option<(f64, f64)> {
        if self.weights <= 0.0 {
            return None;
        }
        let center = self.weighted / self.weights;
        let radius = (self.log_term + self.penalty) / self.weights;
        Some((center - radius, center + radius))
    }

    /// The interval's width; `None` before the first step.
    #[must_use]
    pub fn width(&self) -> Option<f64> {
        self.interval().map(|(low, high)| high - low)
    }
}

/// The IPW increment of a two-arm difference at a logged propensity (S03
/// §4.5's `y_t·w_t`): `y·(1[learned]/(1 − p) − 1[default]/p)`, with `p` the
/// default arm's conditional propensity and |y| ≤ `y_bound`, and its bound
/// `y_bound / min(p, 1 − p)`, known before the arm is drawn.
#[must_use]
pub fn ipw_difference(y: f64, y_bound: f64, learned: bool, p_default: f64) -> (f64, f64) {
    let weight = if learned {
        1.0 / (1.0 - p_default)
    } else {
        -1.0 / p_default
    };
    (y * weight, y_bound / p_default.min(1.0 - p_default))
}

/// A betting CS on the learned-minus-default difference of an outcome with
/// |y| ≤ `y_bound`, from IPW increments at logged propensities (for 5118 and
/// 5119). The difference lies in `[−2·y_bound, 2·y_bound]`.
#[derive(Debug, Clone)]
pub struct DifferenceCs {
    cs: BettingCs,
    y_bound: f64,
}

impl DifferenceCs {
    /// A CS at level `alpha` for outcomes bounded by `y_bound`.
    #[must_use]
    pub fn new(alpha: f64, y_bound: f64) -> Self {
        Self {
            cs: BettingCs::new(alpha, -2.0 * y_bound, 2.0 * y_bound),
            y_bound,
        }
    }

    /// Take one unit's outcome `y`, its arm, and the default arm's logged
    /// conditional propensity `p_default`.
    pub fn push(&mut self, y: f64, learned: bool, p_default: f64) {
        let (increment, bound) = ipw_difference(y, self.y_bound, learned, p_default);
        self.cs.push(increment, bound);
    }

    /// The interval for the difference; `None` once no candidate is left.
    #[must_use]
    pub fn interval(&self) -> Option<(f64, f64)> {
        self.cs.interval()
    }
}

/// The sequential multinomial SRM e-value of one layer
/// (`lindon2020anytimevalid`): the Dirichlet-multinomial mixture's
/// predictive probability of each observed arm over its logged propensity,
/// multiplied over the units. Under the null that arms follow their logged
/// propensities it is a test martingale, so it reaches 1/α_srm with
/// probability at most α_srm however long it runs; a skewed split drives it
/// up.
#[derive(Debug, Clone)]
pub struct SrmEvalue {
    prior: f64,
    counts: Vec<u64>,
    units: u64,
    log_e: f64,
}

impl SrmEvalue {
    /// An e-value over `arms` arms with the uniform Dirichlet(1, …, 1)
    /// mixture.
    #[must_use]
    pub fn new(arms: usize) -> Self {
        Self {
            prior: 1.0,
            counts: vec![0; arms],
            units: 0,
            log_e: 0.0,
        }
    }

    /// Take one unit assigned to `arm`, whose logged propensities over the
    /// arms were `propensities`. An unknown arm or a zero propensity is
    /// ignored.
    pub fn push(&mut self, arm: usize, propensities: &[f64]) {
        let Some(&null) = propensities.get(arm) else {
            return;
        };
        if arm >= self.counts.len() || null <= 0.0 {
            return;
        }
        let arms = self.counts.len() as f64;
        let predictive = (self.prior + self.counts[arm] as f64)
            / (self.prior * arms + self.units as f64);
        self.log_e += predictive.ln() - null.ln();
        self.counts[arm] += 1;
        self.units += 1;
    }

    /// The e-value.
    #[must_use]
    pub fn e_value(&self) -> f64 {
        self.log_e.exp()
    }

    /// Whether the e-value has reached 1/`alpha`.
    #[must_use]
    pub fn rejects(&self, alpha: f64) -> bool {
        self.log_e >= (1.0 / alpha).ln()
    }
}

#[cfg(test)]
mod tests {
    use super::super::sim::SplitMix64;
    use super::*;

    /// A Bernoulli draw with probability `p`.
    fn bernoulli(rng: &mut SplitMix64, p: f64) -> bool {
        rng.next_f64() < p
    }

    /// Whether the betting CS covers `mean` at every step of `steps`: its
    /// hedged capital at the true mean stays below 1/α (no grid involved).
    fn betting_covers(steps: &[(f64, f64)], mean: f64, alpha: f64) -> bool {
        let mut plug_in = PlugIn::new(alpha);
        let (mut up, mut down) = (1.0, 1.0);
        for &(x, bound) in steps {
            let (y, lo, hi) = scaled(x, bound);
            let m = (mean - lo) / (hi - lo);
            if hedged_step(&mut up, &mut down, plug_in.bet(), y, m) >= 1.0 / alpha {
                return false;
            }
            plug_in.observe(y);
        }
        true
    }

    /// Whether the empirical-Bernstein CS covers `mean` at every step.
    fn bernstein_covers(steps: &[(f64, f64)], mean: f64, alpha: f64) -> bool {
        let mut cs = EmpiricalBernsteinCs::new(alpha);
        steps.iter().all(|&(x, bound)| {
            cs.push(x, bound);
            cs.interval().is_none_or(|(low, high)| low <= mean && mean <= high)
        })
    }

    /// One increment in `[0, 1]`: a success with probability 0.3.
    fn bernoulli_increment(rng: &mut SplitMix64) -> (f64, f64) {
        (f64::from(u8::from(bernoulli(rng, 0.3))), 1.0)
    }

    /// One heavy-tailed IPW increment at S03 §4.5's worst case: U = 1[pass]
    /// − (M/q)·fg with q = 0.05, the default arm at p = 0.02, both arms alike
    /// (β = 0), so |ψ| ≤ 1/(q·p) = 1000.
    fn worst_case_increment(rng: &mut SplitMix64) -> (f64, f64) {
        let (q, p) = (0.05, 0.02);
        let passed = bernoulli(rng, 0.7);
        let false_green = passed && bernoulli(rng, 0.1);
        let audited = bernoulli(rng, q);
        let u = f64::from(u8::from(passed))
            - f64::from(u8::from(audited && false_green)) / q;
        let learned = !bernoulli(rng, p);
        let (increment, _) = ipw_difference(u, 1.0 / q, learned, p);
        (increment, 1.0 / (q * p))
    }

    /// S03 T4: both CSs cover the mean at every step on at least 1 − α of
    /// 10⁴ paths, at α = 0.05: bounded increments with a light tail, and
    /// heavy-tailed IPW increments at q = 0.05, p_t = 0.02.
    #[test]
    fn cs_time_uniform_coverage() {
        const PATHS: u64 = 10_000;
        const STEPS: usize = 300;
        let alpha = 0.05;
        let scenarios: [(&str, f64, fn(&mut SplitMix64) -> (f64, f64)); 2] = [
            ("bernoulli(0.3)", 0.3, bernoulli_increment),
            ("heavy-tailed IPW", 0.0, worst_case_increment),
        ];
        for (name, mean, draw) in scenarios {
            let (mut betting, mut bernstein) = (0_u64, 0_u64);
            for path in 0..PATHS {
                let mut rng = SplitMix64::new(path);
                let steps: Vec<(f64, f64)> = (0..STEPS).map(|_| draw(&mut rng)).collect();
                betting += u64::from(betting_covers(&steps, mean, alpha));
                bernstein += u64::from(bernstein_covers(&steps, mean, alpha));
            }
            let betting = betting as f64 / PATHS as f64;
            let bernstein = bernstein as f64 / PATHS as f64;
            println!("{name}: betting coverage {betting:.4}, empirical Bernstein {bernstein:.4}");
            assert!(betting >= 1.0 - alpha, "{name}: betting {betting}");
            assert!(bernstein >= 1.0 - alpha, "{name}: empirical Bernstein {bernstein}");
        }
    }

    /// S03 §4.5: the SRM e-value stays below 1/α_srm on at least 95% of null
    /// paths (an 80/20 layer drawn at its logged propensities), and crosses
    /// it on a planted 55/45 split of a 50/50 layer.
    #[test]
    fn srm_evalue_controls_null_and_flags_imbalance() {
        let alpha = 0.05;
        let null_layer = [0.8, 0.2];
        let mut false_alarms = 0_u32;
        for path in 0..1_000 {
            let mut rng = SplitMix64::new(path);
            let mut srm = SrmEvalue::new(2);
            let mut alarmed = false;
            for _ in 0..2_000 {
                let arm = usize::from(!bernoulli(&mut rng, null_layer[0]));
                srm.push(arm, &null_layer);
                alarmed |= srm.rejects(alpha);
            }
            false_alarms += u32::from(alarmed);
        }
        println!("SRM null: {false_alarms} of 1000 paths crossed 1/α");
        assert!(false_alarms <= 50, "{false_alarms} false alarms");

        let fair = [0.5, 0.5];
        for path in 0..20 {
            let mut rng = SplitMix64::new(10_000 + path);
            let mut srm = SrmEvalue::new(2);
            for _ in 0..10_000 {
                let arm = usize::from(!bernoulli(&mut rng, 0.55));
                srm.push(arm, &fair);
            }
            assert!(srm.rejects(alpha), "path {path}: e = {}", srm.e_value());
        }
    }

    /// The IPW increment's bound holds on both arms, and the difference CS
    /// narrows around a planted difference.
    #[test]
    fn difference_cs_brackets_a_planted_effect() {
        for (y, learned, p) in [(1.0, true, 0.2), (1.0, false, 0.2), (-1.0, false, 0.02)] {
            let (increment, bound) = ipw_difference(y, 1.0, learned, p);
            assert!(increment.abs() <= bound, "{increment} beyond {bound}");
        }
        let mut cs = DifferenceCs::new(0.05, 1.0);
        let mut rng = SplitMix64::new(7);
        for _ in 0..5_000 {
            let learned = !bernoulli(&mut rng, 0.2);
            let rate = if learned { 0.7 } else { 0.4 };
            cs.push(f64::from(u8::from(bernoulli(&mut rng, rate))), learned, 0.2);
        }
        let (low, high) = cs.interval().expect("an interval");
        assert!(low <= 0.3 && 0.3 <= high, "[{low}, {high}]");
        assert!(low > 0.0, "the difference is detected: [{low}, {high}]");
    }

    /// Whether `got` matches the fixture's `want` (null for no interval) to
    /// a relative `tolerance`.
    fn close(got: Option<f64>, want: &serde_json::Value, tolerance: f64) -> bool {
        match (got, want.as_f64()) {
            (Some(got), Some(want)) => (got - want).abs() <= tolerance * want.abs().max(1e-12),
            (got, want) => got.is_none() && want.is_none(),
        }
    }

    /// The fixed-seed widths pinned for backlog 5111 (`widths.json`): the
    /// betting CS's are its grid steps exactly, the empirical-Bernstein
    /// CS's agree to 1e-9.
    #[test]
    fn cs_widths_reproduce_the_pinned_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/loop_audit_cs/widths.json");
        let text = std::fs::read_to_string(path).expect("the widths fixture");
        let fixture: serde_json::Value = serde_json::from_str(&text).expect("parse the fixture");
        let alpha = fixture["alpha"].as_f64().expect("alpha");
        let sequences = fixture["sequences"].as_array().expect("sequences");
        assert!(!sequences.is_empty());
        for sequence in sequences {
            let name = sequence["name"].as_str().unwrap_or("?");
            let bound = sequence["bound"].as_f64().expect("bound");
            let values = sequence["values"].as_array().expect("values");
            let mut betting = BettingCs::new(alpha, -bound, bound);
            let mut bernstein = EmpiricalBernsteinCs::new(alpha);
            for (step, value) in values.iter().enumerate() {
                let x = value.as_f64().expect("a value");
                betting.push(x, bound);
                bernstein.push(x, bound);
                let want = &sequence["betting_widths"][step];
                assert!(close(betting.width(), want, 1e-12), "{name} step {step}: betting");
                let want = &sequence["eb_widths"][step];
                assert!(close(bernstein.width(), want, 1e-9), "{name} step {step}: bernstein");
            }
        }
    }
}
