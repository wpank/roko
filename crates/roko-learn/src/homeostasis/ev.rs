//! The essential variables (S06 §4.2), estimated over a window of task
//! resolutions, each with a Schmitt band, and the drive D that is 0 inside
//! the viability region.
//!
//! - E1 `pass_rate`: passes over resolutions, with a Wilson 90% interval.
//!   It breaches below `lo` and recovers at `inner` (`lo + 0.05`); σ = 0.10.
//! - E2 `usd_per_verified_success`: Σ cost / (N_passed · (1 − f̂g)), with a
//!   bootstrap 90% interval. It breaches above `min(hi, abs_cap)` and
//!   recovers at `inner` (0.9 of the bound); σ is half the bound.
//! - E3 `false_green`: the Beta(1, 9) posterior over the last 30 audits. It
//!   breaches when P(fg > `hi`) > 0.9 and recovers when the posterior mean
//!   reaches `inner` (0.9 · `hi`); σ = 0.05.
//! - E4 `latency_p90_s`: the nearest-rank p90 of wall seconds, with a
//!   bootstrap 90% interval. It breaches above `hi` and recovers at `inner`
//!   (0.9 · `hi`); σ is half of `hi`.
//!
//! E2 is unmeasurable, never 0, while any resolution in the window has a
//! cost that is unknown or whose source is `unknown`, `estimated` or `mock`.
//! CLI subscription runs are priced API-equivalent (D1), so they count. E3
//! is unmeasured until M4 supplies audits (S05.12). An EV without a value
//! adds nothing to D, is listed as unmeasured, and never changes its band.
//!
//! D = (Σ wᵢ gᵢⁿ)^(1/m), gᵢ = distance outside the inner band / σᵢ
//! (`keramati2014homeostatic`), with n = 3 and m = 2 so the worst EV
//! dominates, and the weights of the S5 policy (D9: 1, 1, 2, 0.5).

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use super::resolution::TaskResolution;

/// z of a two-sided 90% interval.
pub const Z90: f64 = 1.644_854;
/// Resamples behind each bootstrap interval.
pub const BOOTSTRAP_RESAMPLES: usize = 400;
/// The bootstrap's seed, so one window always gives one interval.
pub const BOOTSTRAP_SEED: u64 = 0x5e06_e2e4;
/// Audits E3 looks back over.
pub const FALSE_GREEN_AUDITS: usize = 30;
/// E3's prior Beta(1, 9): a 10% false-green rate, worth ten audits.
pub const FALSE_GREEN_PRIOR: (u32, u32) = (1, 9);
/// E3 breaches when the posterior puts more than this on fg > U₃.
pub const FALSE_GREEN_BREACH_PROBABILITY: f64 = 0.9;
/// E1's σ in the drive.
pub const PASS_RATE_SIGMA: f64 = 0.10;
/// E3's σ in the drive.
pub const FALSE_GREEN_SIGMA: f64 = 0.05;
/// E2's and E4's σ in the drive, as a share of their bound.
pub const UPPER_SIGMA_SHARE: f64 = 0.5;
/// E1 recovers this far above its bound when the policy names no inner band.
pub const LOWER_INNER_MARGIN: f64 = 0.05;
/// An upper EV recovers at this share of its bound when the policy names no
/// inner band.
pub const UPPER_INNER_SHARE: f64 = 0.9;

/// An essential variable (S06 §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ev {
    /// E1: verified pass rate.
    PassRate,
    /// E2: API-equivalent dollars per verified success (P1).
    UsdPerVerifiedSuccess,
    /// E3: share of audited passes that fail stronger checks.
    FalseGreen,
    /// E4: p90 wall time per resolution, in seconds.
    LatencyP90S,
}

/// The side of its band an EV breaches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Breaches below its bound (E1).
    Lower,
    /// Breaches above its bound (E2, E3, E4).
    Upper,
}

impl Ev {
    /// Every EV, E1 to E4.
    pub const ALL: [Self; 4] = [
        Self::PassRate,
        Self::UsdPerVerifiedSuccess,
        Self::FalseGreen,
        Self::LatencyP90S,
    ];

    /// The EV's name in the S5 policy and in controller records.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PassRate => "pass_rate",
            Self::UsdPerVerifiedSuccess => "usd_per_verified_success",
            Self::FalseGreen => "false_green",
            Self::LatencyP90S => "latency_p90_s",
        }
    }

    /// The side the EV breaches on.
    #[must_use]
    pub const fn side(self) -> Side {
        match self {
            Self::PassRate => Side::Lower,
            Self::UsdPerVerifiedSuccess | Self::FalseGreen | Self::LatencyP90S => Side::Upper,
        }
    }
}

// ── Bounds and weights (S5) ───────────────────────────────────────────

/// A lower bound with its recovery band (E1), as the S5 policy writes it:
/// `{ lo = 0.70, inner = 0.75 }`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LowerBound {
    /// The EV breaches below this.
    pub lo: f64,
    /// The EV recovers at or above this; `lo + 0.05` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner: Option<f64>,
}

impl LowerBound {
    /// The recovery threshold.
    #[must_use]
    pub fn inner(&self) -> f64 {
        self.inner.unwrap_or(self.lo + LOWER_INNER_MARGIN)
    }
}

/// An upper bound with its recovery band (E2, E3, E4), as the S5 policy
/// writes it: `{ hi = 0.12, inner = 0.108, abs_cap = 0.50 }`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpperBound {
    /// The EV breaches above this, or above `abs_cap` when that is lower.
    pub hi: f64,
    /// The EV recovers at or below this; 0.9 of the bound when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner: Option<f64>,
    /// An absolute cap on the bound: E2's U₂ is `min(hi, abs_cap)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abs_cap: Option<f64>,
}

impl UpperBound {
    /// The breach threshold: `hi`, capped by `abs_cap`.
    #[must_use]
    pub fn outer(&self) -> f64 {
        self.abs_cap.map_or(self.hi, |cap| self.hi.min(cap))
    }

    /// The recovery threshold.
    #[must_use]
    pub fn inner(&self) -> f64 {
        self.inner.unwrap_or(UPPER_INNER_SHARE * self.outer())
    }
}

/// The four bounds, the S5 policy's `ev` table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvBounds {
    /// E1.
    pub pass_rate: LowerBound,
    /// E2.
    pub usd_per_verified_success: UpperBound,
    /// E3.
    pub false_green: UpperBound,
    /// E4, in seconds.
    pub latency_p90_s: UpperBound,
}

impl EvBounds {
    /// S06 §5's example policy: E1 ≥ 0.70, E2 ≤ $0.12 (cap $0.50), E3 ≤
    /// 0.10, E4 ≤ 900 s, with its inner bands.
    #[must_use]
    pub const fn s06_example() -> Self {
        Self {
            pass_rate: LowerBound {
                lo: 0.70,
                inner: Some(0.75),
            },
            usd_per_verified_success: UpperBound {
                hi: 0.12,
                inner: Some(0.108),
                abs_cap: Some(0.50),
            },
            false_green: UpperBound {
                hi: 0.10,
                inner: None,
                abs_cap: None,
            },
            latency_p90_s: UpperBound {
                hi: 900.0,
                inner: Some(810.0),
                abs_cap: None,
            },
        }
    }

    /// The breach threshold of `ev`.
    #[must_use]
    pub fn outer(&self, ev: Ev) -> f64 {
        match ev {
            Ev::PassRate => self.pass_rate.lo,
            Ev::UsdPerVerifiedSuccess => self.usd_per_verified_success.outer(),
            Ev::FalseGreen => self.false_green.outer(),
            Ev::LatencyP90S => self.latency_p90_s.outer(),
        }
    }

    /// The recovery threshold of `ev`.
    #[must_use]
    pub fn inner(&self, ev: Ev) -> f64 {
        match ev {
            Ev::PassRate => self.pass_rate.inner(),
            Ev::UsdPerVerifiedSuccess => self.usd_per_verified_success.inner(),
            Ev::FalseGreen => self.false_green.inner(),
            Ev::LatencyP90S => self.latency_p90_s.inner(),
        }
    }

    /// σ of `ev` in the drive: 0.10 for E1, 0.05 for E3, and half the bound
    /// for E2 and E4.
    #[must_use]
    pub fn sigma(&self, ev: Ev) -> f64 {
        match ev {
            Ev::PassRate => PASS_RATE_SIGMA,
            Ev::FalseGreen => FALSE_GREEN_SIGMA,
            Ev::UsdPerVerifiedSuccess | Ev::LatencyP90S => UPPER_SIGMA_SHARE * self.outer(ev),
        }
    }
}

/// The drive's weight of each EV, the S5 policy's `drive.weights`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DriveWeights {
    /// E1's weight.
    pub pass_rate: f64,
    /// E2's weight.
    pub usd_per_verified_success: f64,
    /// E3's weight.
    pub false_green: f64,
    /// E4's weight.
    pub latency_p90_s: f64,
}

impl Default for DriveWeights {
    /// D9 (decision 8102): 1, 1, 2, 0.5, so a false green weighs double.
    fn default() -> Self {
        Self {
            pass_rate: 1.0,
            usd_per_verified_success: 1.0,
            false_green: 2.0,
            latency_p90_s: 0.5,
        }
    }
}

impl DriveWeights {
    /// The weight of `ev`.
    #[must_use]
    pub const fn get(&self, ev: Ev) -> f64 {
        match ev {
            Ev::PassRate => self.pass_rate,
            Ev::UsdPerVerifiedSuccess => self.usd_per_verified_success,
            Ev::FalseGreen => self.false_green,
            Ev::LatencyP90S => self.latency_p90_s,
        }
    }
}

/// The drive's shape, the S5 policy's `drive` table: D = (Σ wᵢ gᵢⁿ)^(1/m).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DrivePolicy {
    /// The exponent of each gᵢ (3).
    pub n: u32,
    /// The root of the sum (2).
    pub m: u32,
    /// The weights.
    pub weights: DriveWeights,
}

impl Default for DrivePolicy {
    fn default() -> Self {
        Self {
            n: 3,
            m: 2,
            weights: DriveWeights::default(),
        }
    }
}

// ── Estimates ─────────────────────────────────────────────────────────

/// Why an EV has no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvGap {
    /// No resolution in the window, or none with a wall time.
    NoData,
    /// E2: a resolution's cost is unknown, or its source is `unknown`,
    /// `estimated` or `mock`. E2 is then unmeasurable, never 0.
    UnreportedCost,
    /// E2: no verified success in the window to divide by. E1 sees it.
    NoSuccess,
    /// E3: no audit yet; M4 supplies them (S05.12).
    NoAudits,
}

/// One EV's estimate over the window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Estimate {
    /// The EV.
    pub ev: Ev,
    /// The point estimate; `None` when `gap` says why there is none.
    pub value: Option<f64>,
    /// Its 90% interval.
    pub interval: Option<(f64, f64)>,
    /// Observations behind it: resolutions, or audits for E3.
    pub n: usize,
    /// Why `value` is `None`.
    pub gap: Option<EvGap>,
    /// E3 only: the posterior probability that the false-green rate is above
    /// its bound.
    pub breach_probability: Option<f64>,
}

impl Estimate {
    /// An estimate with a value.
    #[must_use]
    pub const fn of(ev: Ev, value: f64, interval: Option<(f64, f64)>, n: usize) -> Self {
        Self {
            ev,
            value: Some(value),
            interval,
            n,
            gap: None,
            breach_probability: None,
        }
    }

    /// An estimate without a value, and why.
    #[must_use]
    pub const fn missing(ev: Ev, n: usize, gap: EvGap) -> Self {
        Self {
            ev,
            value: None,
            interval: None,
            n,
            gap: Some(gap),
            breach_probability: None,
        }
    }

    /// Whether the estimate has a value.
    #[must_use]
    pub const fn is_measured(&self) -> bool {
        self.value.is_some()
    }
}

/// The four estimates of one window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EvEstimates {
    /// E1.
    pub pass_rate: Estimate,
    /// E2.
    pub usd_per_verified_success: Estimate,
    /// E3.
    pub false_green: Estimate,
    /// E4.
    pub latency_p90_s: Estimate,
}

impl EvEstimates {
    /// The estimate of `ev`.
    #[must_use]
    pub const fn get(&self, ev: Ev) -> &Estimate {
        match ev {
            Ev::PassRate => &self.pass_rate,
            Ev::UsdPerVerifiedSuccess => &self.usd_per_verified_success,
            Ev::FalseGreen => &self.false_green,
            Ev::LatencyP90S => &self.latency_p90_s,
        }
    }
}

/// The estimators' inputs: the last `window` resolutions of the adaptive
/// arm, and the last [`FALSE_GREEN_AUDITS`] audit findings among them.
/// Holdout rows run θ₀ and never steer M1, so they are left out.
#[derive(Debug, Clone, Default)]
pub struct EvWindow {
    window: usize,
    resolutions: Vec<TaskResolution>,
    audits: Vec<bool>,
}

impl EvWindow {
    /// An empty window of `window` resolutions (at least one).
    #[must_use]
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            ..Self::default()
        }
    }

    /// Take `resolution` in, dropping the oldest beyond the window. Returns
    /// `false`, and takes nothing, for a holdout row.
    pub fn push(&mut self, resolution: &TaskResolution) -> bool {
        if resolution.holdout() {
            return false;
        }
        if resolution.audited
            && let Some(false_green) = resolution.audit_false_green
        {
            push_bounded(&mut self.audits, false_green, FALSE_GREEN_AUDITS);
        }
        push_bounded(&mut self.resolutions, resolution.clone(), self.window);
        true
    }

    /// The resolutions in the window, oldest first.
    #[must_use]
    pub fn resolutions(&self) -> &[TaskResolution] {
        &self.resolutions
    }

    /// The audit findings in view, oldest first: `true` is a false green.
    #[must_use]
    pub fn audits(&self) -> &[bool] {
        &self.audits
    }

    /// Whether the window holds `window` resolutions.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.resolutions.len() >= self.window
    }

    /// Every EV's estimate, with E3 judged against `bounds`. E2's
    /// false-green correction f̂g is E3's posterior mean while audits exist
    /// (S05's per-stratum estimate replaces it once M4 publishes one), and 0
    /// before.
    #[must_use]
    pub fn estimate(&self, bounds: &EvBounds) -> EvEstimates {
        let false_green = estimate_false_green(&self.audits, bounds.false_green.outer());
        EvEstimates {
            pass_rate: estimate_pass_rate(&self.resolutions),
            usd_per_verified_success: estimate_usd_per_verified_success(
                &self.resolutions,
                false_green.value,
            ),
            false_green,
            latency_p90_s: estimate_latency_p90_s(&self.resolutions),
        }
    }
}

fn push_bounded<T>(items: &mut Vec<T>, item: T, capacity: usize) {
    items.push(item);
    if items.len() > capacity {
        items.remove(0);
    }
}

/// E1: verified successes over resolutions, with its Wilson 90% interval.
#[must_use]
pub fn estimate_pass_rate(window: &[TaskResolution]) -> Estimate {
    let n = window.len();
    if n == 0 {
        return Estimate::missing(Ev::PassRate, 0, EvGap::NoData);
    }
    let passed = window
        .iter()
        .filter(|resolution| resolution.verified_success())
        .count();
    let rate = passed as f64 / n as f64;
    Estimate::of(Ev::PassRate, rate, Some(wilson_interval(passed, n)), n)
}

/// The Wilson score interval at 90% of `successes` in `trials` (at least
/// one).
#[must_use]
pub fn wilson_interval(successes: usize, trials: usize) -> (f64, f64) {
    let n = trials.max(1) as f64;
    let p = successes as f64 / n;
    let z2 = Z90 * Z90;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half = Z90 * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denominator;
    ((center - half).max(0.0), (center + half).min(1.0))
}

/// E2: every resolution's API-equivalent cost, failed ones included, over
/// the verified successes corrected for false greens, with a bootstrap 90%
/// interval. `false_green` is f̂g; `None` counts as 0.
#[must_use]
pub fn estimate_usd_per_verified_success(
    window: &[TaskResolution],
    false_green: Option<f64>,
) -> Estimate {
    let ev = Ev::UsdPerVerifiedSuccess;
    let n = window.len();
    if n == 0 {
        return Estimate::missing(ev, 0, EvGap::NoData);
    }
    let unreported = window.iter().any(|resolution| {
        resolution.api_equiv_usd.is_none() || !resolution.cost_source_mix.all_reported()
    });
    if unreported {
        return Estimate::missing(ev, n, EvGap::UnreportedCost);
    }
    let correction = 1.0 - false_green.unwrap_or(0.0).clamp(0.0, 1.0);
    let Some(ratio) = cost_ratio(window.iter(), correction) else {
        return Estimate::missing(ev, n, EvGap::NoSuccess);
    };
    let interval = bootstrap_interval(window, |sample| {
        cost_ratio(sample.iter().copied(), correction)
    });
    Estimate::of(ev, ratio, interval, n)
}

fn cost_ratio<'a, I>(resolutions: I, correction: f64) -> Option<f64>
where
    I: IntoIterator<Item = &'a TaskResolution>,
{
    let mut cost = 0.0;
    let mut passed = 0_usize;
    for resolution in resolutions {
        cost += resolution.api_equiv_usd.unwrap_or(0.0);
        passed += usize::from(resolution.verified_success());
    }
    let successes = passed as f64 * correction;
    (successes > 0.0).then_some(cost / successes)
}

/// E3: the Beta(1, 9) posterior over the last [`FALSE_GREEN_AUDITS`] audit
/// findings (`true` is a false green). The value is the posterior mean, the
/// interval its 90% credible interval, and `breach_probability` the
/// posterior probability that the rate is above `bound`.
#[must_use]
pub fn estimate_false_green(audits: &[bool], bound: f64) -> Estimate {
    let audits = &audits[audits.len().saturating_sub(FALSE_GREEN_AUDITS)..];
    let n = audits.len();
    if n == 0 {
        return Estimate::missing(Ev::FalseGreen, 0, EvGap::NoAudits);
    }
    let found = audits.iter().filter(|&&false_green| false_green).count();
    let (prior_a, prior_b) = FALSE_GREEN_PRIOR;
    let a = prior_a + u32::try_from(found).unwrap_or(u32::MAX);
    let b = prior_b + u32::try_from(n - found).unwrap_or(u32::MAX);
    let mean = f64::from(a) / (f64::from(a) + f64::from(b));
    let interval = (beta_quantile(a, b, 0.05), beta_quantile(a, b, 0.95));
    let mut estimate = Estimate::of(Ev::FalseGreen, mean, Some(interval), n);
    estimate.breach_probability = Some(1.0 - beta_cdf(a, b, bound));
    estimate
}

/// The CDF at `x` of Beta(`a`, `b`) with whole `a`, `b` ≥ 1, through the
/// binomial identity I_x(a, b) = P(Binomial(a + b − 1, x) ≥ a).
#[must_use]
pub fn beta_cdf(a: u32, b: u32, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let a = a.max(1);
    let trials = a + b.max(1) - 1;
    let odds = x / (1.0 - x);
    let mut pmf = (1.0 - x).powi(i32::try_from(trials).unwrap_or(i32::MAX));
    let mut below = 0.0;
    for j in 0..a {
        below += pmf;
        pmf *= f64::from(trials - j) / f64::from(j + 1) * odds;
    }
    (1.0 - below).clamp(0.0, 1.0)
}

/// The `q` quantile of Beta(`a`, `b`), by bisection on [`beta_cdf`].
#[must_use]
pub fn beta_quantile(a: u32, b: u32, q: f64) -> f64 {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if beta_cdf(a, b, mid) < q {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    f64::midpoint(lo, hi)
}

/// E4: the nearest-rank p90 of the window's wall times, in seconds, with a
/// bootstrap 90% interval. Resolutions without a wall time are left out.
#[must_use]
pub fn estimate_latency_p90_s(window: &[TaskResolution]) -> Estimate {
    let seconds: Vec<f64> = window
        .iter()
        .filter_map(|resolution| resolution.wall_ms)
        .map(|ms| ms as f64 / 1000.0)
        .collect();
    if seconds.is_empty() {
        return Estimate::missing(Ev::LatencyP90S, 0, EvGap::NoData);
    }
    let interval = bootstrap_interval(&seconds, |sample| {
        Some(p90(sample.iter().map(|&&seconds| seconds).collect()))
    });
    Estimate::of(Ev::LatencyP90S, p90(seconds.clone()), interval, seconds.len())
}

/// The nearest-rank p90: the ⌈0.9·n⌉-th smallest value.
fn p90(mut values: Vec<f64>) -> f64 {
    values.sort_unstable_by(f64::total_cmp);
    nearest_rank(&values, 9, 10)
}

/// The ⌈n · numerator / denominator⌉-th smallest of `sorted` (at least the
/// first). `sorted` is not empty.
fn nearest_rank(sorted: &[f64], numerator: usize, denominator: usize) -> f64 {
    let rank = (sorted.len() * numerator).div_ceil(denominator).max(1);
    sorted[rank.min(sorted.len()) - 1]
}

/// The percentile 90% interval of `statistic` over seeded resamples of
/// `rows`; `None` when fewer than half the resamples have a value.
fn bootstrap_interval<T>(
    rows: &[T],
    statistic: impl Fn(&[&T]) -> Option<f64>,
) -> Option<(f64, f64)> {
    if rows.is_empty() {
        return None;
    }
    let mut rng = ChaCha8Rng::seed_from_u64(BOOTSTRAP_SEED);
    let mut sample: Vec<&T> = Vec::with_capacity(rows.len());
    let mut values = Vec::with_capacity(BOOTSTRAP_RESAMPLES);
    for _ in 0..BOOTSTRAP_RESAMPLES {
        sample.clear();
        sample.extend((0..rows.len()).map(|_| &rows[rng.gen_range(0..rows.len())]));
        values.extend(statistic(&sample));
    }
    if values.len() * 2 < BOOTSTRAP_RESAMPLES {
        return None;
    }
    values.sort_unstable_by(f64::total_cmp);
    Some((nearest_rank(&values, 1, 20), nearest_rank(&values, 19, 20)))
}

// ── Schmitt bands and the drive ───────────────────────────────────────

/// A band's change of state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BandTransition {
    /// The EV crossed its outer bound.
    Breach,
    /// The EV came back inside its inner band.
    Recover,
}

/// One EV's Schmitt trigger: it breaches at the outer bound and recovers
/// only inside the inner band, so an EV that hovers at its bound does not
/// flap. E3 breaches on its posterior probability instead of its mean.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SchmittBand {
    /// The EV.
    pub ev: Ev,
    /// The breach threshold.
    pub outer: f64,
    /// The recovery threshold.
    pub inner: f64,
    /// Whether the EV is breached.
    pub breached: bool,
}

impl SchmittBand {
    /// The band of `ev` under `bounds`, not breached.
    #[must_use]
    pub fn new(ev: Ev, bounds: &EvBounds) -> Self {
        Self {
            ev,
            outer: bounds.outer(ev),
            inner: bounds.inner(ev),
            breached: false,
        }
    }

    /// Whether `estimate` is beyond the outer bound.
    #[must_use]
    pub fn beyond_outer(&self, estimate: &Estimate) -> bool {
        if self.ev == Ev::FalseGreen {
            return estimate
                .breach_probability
                .is_some_and(|probability| probability > FALSE_GREEN_BREACH_PROBABILITY);
        }
        estimate.value.is_some_and(|value| match self.ev.side() {
            Side::Lower => value < self.outer,
            Side::Upper => value > self.outer,
        })
    }

    /// Whether `estimate` is inside the inner band.
    #[must_use]
    pub fn within_inner(&self, estimate: &Estimate) -> bool {
        estimate.value.is_some_and(|value| match self.ev.side() {
            Side::Lower => value >= self.inner,
            Side::Upper => value <= self.inner,
        })
    }

    /// Feed one estimate and return the transition it causes. An estimate
    /// without a value changes nothing.
    pub fn update(&mut self, estimate: &Estimate) -> Option<BandTransition> {
        if !self.breached && self.beyond_outer(estimate) {
            self.breached = true;
            return Some(BandTransition::Breach);
        }
        if self.breached && self.within_inner(estimate) {
            self.breached = false;
            return Some(BandTransition::Recover);
        }
        None
    }
}

/// The four bands.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EvBands {
    /// E1.
    pub pass_rate: SchmittBand,
    /// E2.
    pub usd_per_verified_success: SchmittBand,
    /// E3.
    pub false_green: SchmittBand,
    /// E4.
    pub latency_p90_s: SchmittBand,
}

impl EvBands {
    /// The bands of `bounds`, none breached.
    #[must_use]
    pub fn new(bounds: &EvBounds) -> Self {
        Self {
            pass_rate: SchmittBand::new(Ev::PassRate, bounds),
            usd_per_verified_success: SchmittBand::new(Ev::UsdPerVerifiedSuccess, bounds),
            false_green: SchmittBand::new(Ev::FalseGreen, bounds),
            latency_p90_s: SchmittBand::new(Ev::LatencyP90S, bounds),
        }
    }

    /// The band of `ev`.
    #[must_use]
    pub const fn get(&self, ev: Ev) -> &SchmittBand {
        match ev {
            Ev::PassRate => &self.pass_rate,
            Ev::UsdPerVerifiedSuccess => &self.usd_per_verified_success,
            Ev::FalseGreen => &self.false_green,
            Ev::LatencyP90S => &self.latency_p90_s,
        }
    }

    fn get_mut(&mut self, ev: Ev) -> &mut SchmittBand {
        match ev {
            Ev::PassRate => &mut self.pass_rate,
            Ev::UsdPerVerifiedSuccess => &mut self.usd_per_verified_success,
            Ev::FalseGreen => &mut self.false_green,
            Ev::LatencyP90S => &mut self.latency_p90_s,
        }
    }

    /// Feed one window's estimates; returns the transitions, E1 first.
    pub fn update(&mut self, estimates: &EvEstimates) -> Vec<(Ev, BandTransition)> {
        Ev::ALL
            .into_iter()
            .filter_map(|ev| {
                self.get_mut(ev)
                    .update(estimates.get(ev))
                    .map(|transition| (ev, transition))
            })
            .collect()
    }

    /// The breached EVs, E1 first.
    #[must_use]
    pub fn breached(&self) -> Vec<Ev> {
        Ev::ALL
            .into_iter()
            .filter(|&ev| self.get(ev).breached)
            .collect()
    }
}

/// The drive of one window and its parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drive {
    /// D = (Σ wᵢ gᵢⁿ)^(1/m): 0 inside the viability region.
    pub value: f64,
    /// gᵢ of each EV, E1 first: its distance outside the inner band over σᵢ.
    pub excess: Vec<(Ev, f64)>,
    /// The EVs without a value: each adds nothing to D.
    pub unmeasured: Vec<Ev>,
}

/// The drive of `estimates` under `bounds` and `policy`
/// (`keramati2014homeostatic`).
#[must_use]
pub fn drive(estimates: &EvEstimates, bounds: &EvBounds, policy: &DrivePolicy) -> Drive {
    let exponent = i32::try_from(policy.n).unwrap_or(i32::MAX);
    let mut sum = 0.0;
    let mut excess = Vec::with_capacity(Ev::ALL.len());
    let mut unmeasured = Vec::new();
    for ev in Ev::ALL {
        let estimate = estimates.get(ev);
        if !estimate.is_measured() {
            unmeasured.push(ev);
        }
        let distance = estimate.value.map_or(0.0, |value| match ev.side() {
            Side::Lower => (bounds.inner(ev) - value).max(0.0),
            Side::Upper => (value - bounds.inner(ev)).max(0.0),
        });
        let g = if distance > 0.0 {
            distance / bounds.sigma(ev).max(f64::EPSILON)
        } else {
            0.0
        };
        sum += policy.weights.get(ev) * g.powi(exponent);
        excess.push((ev, g));
    }
    let value = if sum > 0.0 {
        sum.powf(1.0 / f64::from(policy.m.max(1)))
    } else {
        0.0
    };
    Drive {
        value,
        excess,
        unmeasured,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::resolution::CostSourceMix;
    use crate::telemetry::{Arm, AttemptOutcome, CostSource};

    /// A resolution priced from reported provider usage.
    fn resolution(passed: bool, usd: f64, wall_ms: u64) -> TaskResolution {
        let mut cost_source_mix = CostSourceMix::default();
        cost_source_mix.add(CostSource::ProviderUsage);
        TaskResolution {
            chain_key: format!("run:plan:{usd}:{wall_ms}"),
            final_verdict: if passed {
                AttemptOutcome::Passed
            } else {
                AttemptOutcome::GateFailed
            },
            attempts: 1,
            api_equiv_usd: Some(usd),
            cost_source_mix,
            wall_ms: Some(wall_ms),
            provider_errors: 0,
            conductor_restarts: 0,
            params_digest: None,
            arm: None,
            audited: false,
            audit_false_green: None,
            pre_instrumentation: false,
            resolved_at: None,
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
        (actual - expected).abs() <= tolerance
    }

    #[test]
    fn e2_unmeasurable_on_unknown_cost() {
        let window: Vec<TaskResolution> = (0..5)
            .map(|index| resolution(index % 2 == 0, 0.10, 60_000))
            .collect();
        let measured = estimate_usd_per_verified_success(&window, None);
        let value = measured.value.expect("reported costs give a value");
        assert!(close(value, 0.5 / 3.0, 1e-12), "{value}");

        // One unknown, estimated or mock row makes E2 unmeasurable, never 0.
        for source in [CostSource::Unknown, CostSource::Estimated, CostSource::Mock] {
            let mut tainted = window.clone();
            let mut mix = CostSourceMix::default();
            mix.add(source);
            tainted[2].cost_source_mix = mix;
            let estimate = estimate_usd_per_verified_success(&tainted, None);
            assert_eq!(estimate.value, None, "{source:?}");
            assert_eq!(estimate.gap, Some(EvGap::UnreportedCost), "{source:?}");
        }
        let mut unpriced = window.clone();
        unpriced[4].api_equiv_usd = None;
        let estimate = estimate_usd_per_verified_success(&unpriced, None);
        assert_eq!(estimate.gap, Some(EvGap::UnreportedCost));

        // CLI subscription usage is priced API-equivalent, so it counts.
        let mut subscription = window.clone();
        for row in &mut subscription {
            let mut mix = CostSourceMix::default();
            mix.add(CostSource::CliUsage);
            row.cost_source_mix = mix;
        }
        let estimate = estimate_usd_per_verified_success(&subscription, None);
        assert_eq!(estimate.value, measured.value);

        // No success: no ratio, and E1 is the EV that sees it.
        let failed: Vec<TaskResolution> = (0..5).map(|_| resolution(false, 0.1, 1)).collect();
        assert_eq!(
            estimate_usd_per_verified_success(&failed, None).gap,
            Some(EvGap::NoSuccess)
        );

        // In the drive the unmeasurable E2 adds nothing and is flagged.
        let mut window_state = EvWindow::new(20);
        for row in &unpriced {
            assert!(window_state.push(row));
        }
        let bounds = EvBounds::s06_example();
        let estimates = window_state.estimate(&bounds);
        assert_eq!(estimates.usd_per_verified_success.value, None);
        let shaped = drive(&estimates, &bounds, &DrivePolicy::default());
        assert!(shaped.unmeasured.contains(&Ev::UsdPerVerifiedSuccess));
        assert!(shaped.unmeasured.contains(&Ev::FalseGreen), "no audits yet");
        let mut bands = EvBands::new(&bounds);
        assert!(
            !bands
                .update(&estimates)
                .iter()
                .any(|&(ev, _)| ev == Ev::UsdPerVerifiedSuccess)
        );
    }

    #[test]
    fn drive_and_bands_match_hand_computed_fixture() {
        // 20 resolutions: 14 passes, $0.07 each, wall times 53 s to 1060 s;
        // ten passes audited, three of them false greens.
        let failures = [3, 7, 11, 15, 18, 19];
        let audited = [0, 1, 2, 4, 5, 6, 8, 9, 10, 12];
        let false_greens = [0, 4, 8];
        let mut window = EvWindow::new(20);
        for index in 0..20_u64 {
            let position = usize::try_from(index).expect("small index");
            let mut row = resolution(!failures.contains(&position), 0.07, (index + 1) * 53_000);
            if audited.contains(&position) {
                row.audited = true;
                row.audit_false_green = Some(false_greens.contains(&position));
            }
            assert!(window.push(&row));
        }
        // A holdout row never enters the window.
        let mut holdout = resolution(false, 9.0, 9_999_000);
        holdout.arm = Some(Arm::Default);
        assert!(!window.push(&holdout));
        assert!(window.is_full());
        assert_eq!(window.audits().len(), 10);

        let bounds = EvBounds::s06_example();
        let estimates = window.estimate(&bounds);

        // E1 = 14/20 with its Wilson 90% interval.
        let e1 = estimates.pass_rate;
        assert!(close(e1.value.expect("E1"), 0.7, 1e-12));
        let (lo, hi) = e1.interval.expect("Wilson interval");
        assert!(close(lo, 0.516_196_238_218_447, 1e-9), "{lo}");
        assert!(close(hi, 0.836_140_607_793_578, 1e-9), "{hi}");

        // E3: Beta(4, 16): mean 0.2, P(fg > 0.10) = P(Bin(19, 0.1) ≤ 3).
        let e3 = estimates.false_green;
        assert!(close(e3.value.expect("E3"), 0.2, 1e-12));
        let probability = e3.breach_probability.expect("E3 breach probability");
        assert!(close(probability, 0.885_002_442_195_639, 1e-9), "{probability}");
        let (lo, hi) = e3.interval.expect("credible interval");
        assert!(close(lo, 0.075_293_816_568_876, 1e-6), "{lo}");
        assert!(close(hi, 0.359_425_649_640_373, 1e-6), "{hi}");

        // E2 = $1.40 / (14 · (1 − 0.2)) = $0.125.
        let e2 = estimates.usd_per_verified_success;
        let value = e2.value.expect("E2");
        assert!(close(value, 0.125, 1e-9), "{value}");
        let (lo, hi) = e2.interval.expect("bootstrap interval");
        assert!(lo <= value && value <= hi, "{lo} {value} {hi}");

        // E4: the 18th smallest of 53 s .. 1060 s.
        let e4 = estimates.latency_p90_s;
        assert!(close(e4.value.expect("E4"), 954.0, 1e-9));
        let (lo, hi) = e4.interval.expect("bootstrap interval");
        assert!(lo <= 954.0 && 954.0 <= hi, "{lo} {hi}");

        // E2 and E4 cross their bounds; E1 sits between bound and band; E3
        // is above its bound but not yet with 90% posterior probability.
        let mut bands = EvBands::new(&bounds);
        assert_eq!(
            bands.update(&estimates),
            [
                (Ev::UsdPerVerifiedSuccess, BandTransition::Breach),
                (Ev::LatencyP90S, BandTransition::Breach)
            ]
        );
        assert_eq!(
            bands.breached(),
            [Ev::UsdPerVerifiedSuccess, Ev::LatencyP90S]
        );

        // D = (1·0.5³ + 1·0.2833³ + 2·2.2³ + 0.5·0.32³)^½.
        let shaped = drive(&estimates, &bounds, &DrivePolicy::default());
        assert!(close(shaped.value, 4.632_507_892_100_172, 1e-9), "{}", shaped.value);
        let excess: Vec<f64> = shaped.excess.iter().map(|&(_, g)| g).collect();
        for (actual, expected) in excess.iter().zip([0.5, 0.017 / 0.06, 2.2, 0.32]) {
            assert!(close(*actual, expected, 1e-9), "{actual} vs {expected}");
        }
        assert!(shaped.unmeasured.is_empty());

        // Inside every inner band, D is 0.
        let mut calm = EvWindow::new(20);
        for _ in 0..20 {
            calm.push(&resolution(true, 0.05, 100_000));
        }
        let calm_estimates = calm.estimate(&bounds);
        assert_eq!(drive(&calm_estimates, &bounds, &DrivePolicy::default()).value, 0.0);

        // Hysteresis: E1 breaches below 0.70, stays breached between the
        // bound and the band, and recovers at 0.75; no value changes nothing.
        let mut band = SchmittBand::new(Ev::PassRate, &bounds);
        let at = |value| Estimate::of(Ev::PassRate, value, None, 20);
        assert_eq!(band.update(&at(0.72)), None);
        assert_eq!(band.update(&at(0.65)), Some(BandTransition::Breach));
        assert_eq!(band.update(&at(0.72)), None);
        assert!(band.breached);
        assert_eq!(
            band.update(&Estimate::missing(Ev::PassRate, 0, EvGap::NoData)),
            None
        );
        assert_eq!(band.update(&at(0.76)), Some(BandTransition::Recover));

        // E3 breaches on 7 false greens in 30 audits (P = 0.963), holds at
        // 5 in 30 (mean 0.15 above the 0.09 band) and recovers at 2 in 30.
        let mut band = SchmittBand::new(Ev::FalseGreen, &bounds);
        let audits = |found: usize| -> Vec<bool> { (0..30).map(|index| index < found).collect() };
        let e3_of = |found| estimate_false_green(&audits(found), bounds.false_green.outer());
        assert!(close(e3_of(7).breach_probability.expect("P"), 0.963_379_384_778_421, 1e-9));
        assert_eq!(band.update(&e3_of(7)), Some(BandTransition::Breach));
        assert_eq!(band.update(&e3_of(5)), None);
        assert_eq!(band.update(&e3_of(2)), Some(BandTransition::Recover));
        assert_eq!(
            estimate_false_green(&[], 0.1).gap,
            Some(EvGap::NoAudits)
        );
    }
}
