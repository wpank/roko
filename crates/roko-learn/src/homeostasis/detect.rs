//! Change detectors (S06 §4.4) over roko-gate's SPC charts, which this
//! module reuses as they are.
//!
//! - E1: a lower CUSUM on the pass indicator (1 for a verified success),
//!   with target p* and drift k = (p* − L₁)/2.
//! - E2 and E4: upper CUSUMs on the log-ratio of a resolution's cost, or wall
//!   time, to its in-control median, with k = 0.35 (half of ln 2: a doubling
//!   is the shift they look for). A resolution without a reported cost or a
//!   wall time adds no observation.
//! - E3: the posterior rule of [`super::ev`], P(fg > U₃) > 0.9.
//!
//! A detector alarms when its CUSUM crosses H in the breach direction, and
//! roko-gate's `CusumDetector` then restarts that sum. A breach is confirmed
//! by `confirm_k` (2) consecutive alarms, each within `confirm_gap`
//! observations of the previous one, so one unlucky run does not open an
//! episode. Every detector restarts after an applied change
//! ([`DetectorBank::reset_all`]). An `EwmaControlChart` per EV gives the
//! demo's warn level and never confirms anything.
//!
//! H is tuned by simulation (S06 C1): with an in-control pass rate of 0.80,
//! L₁ = 0.65 and log-ratio noise of sd 0.5, E1 H = 1.1 and E2/E4 H = 1.2 give
//! a joint ARL₀ of about 135 resolutions and a median confirmed delay of 8
//! resolutions on each of the steps E1 0.80 → 0.50, E2 ×2 and E4 ×2. S06's
//! starting values (H = 2 for E1, 4 for E2 and E4) are single-alarm
//! thresholds: two such alarms take more than 20 resolutions. A calibration
//! with noisier costs or latencies must raise H and rerun
//! `cusum_arl0_at_least_100_in_bounds`.

use roko_gate::spc::{ControlStatus, CusumDetector, CusumShift, EwmaControlChart};
use serde::{Deserialize, Serialize};

use super::ev::{Estimate, Ev, EvBounds, FALSE_GREEN_BREACH_PROBABILITY};
use super::resolution::TaskResolution;

/// The smallest drift allowance E1's CUSUM takes, for a baseline at or
/// below its bound.
const MIN_PASS_RATE_DRIFT: f64 = 0.01;

/// The in-control levels the detectors watch for shifts from, as a
/// calibration run measures them (S06 §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    /// p*: the verified pass rate.
    pub pass_rate: f64,
    /// c*: the median API-equivalent cost of one resolution.
    pub usd_per_resolution: f64,
    /// w*: the median wall time of one resolution, in ms.
    pub wall_ms: f64,
}

/// The detectors' tuning.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DetectorTuning {
    /// E1's decision interval H.
    pub pass_rate_h: f64,
    /// E2's and E4's drift allowance k on the log-ratio.
    pub log_ratio_k: f64,
    /// E2's and E4's decision interval H.
    pub log_ratio_h: f64,
    /// Consecutive alarms that confirm a breach (`[homeostasis] confirm_k`).
    pub confirm_k: u32,
    /// The most observations from one alarm to the next that still count
    /// as consecutive.
    pub confirm_gap: u32,
    /// The EWMA warn charts' smoothing λ.
    pub ewma_lambda: f64,
    /// The EWMA warn charts' control-limit multiplier L.
    pub ewma_limit: f64,
    /// The log-ratio noise the E2 and E4 warn charts assume (sd).
    pub log_ratio_sd: f64,
}

impl Default for DetectorTuning {
    fn default() -> Self {
        Self {
            pass_rate_h: 1.1,
            log_ratio_k: 0.35,
            log_ratio_h: 1.2,
            confirm_k: 2,
            confirm_gap: 4,
            ewma_lambda: 0.2,
            ewma_limit: 3.0,
            log_ratio_sd: 0.5,
        }
    }
}

/// What the detectors read from one resolution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// The resolution was a verified success.
    pub passed: bool,
    /// Its API-equivalent cost; `None` when unknown or not reported usage,
    /// so E2's detector skips it.
    pub usd: Option<f64>,
    /// Its wall time in ms; `None` when unknown.
    pub wall_ms: Option<u64>,
}

impl Observation {
    /// The observation of `resolution`.
    #[must_use]
    pub fn of(resolution: &TaskResolution) -> Self {
        let reported = resolution.cost_source_mix.all_reported();
        Self {
            passed: resolution.verified_success(),
            usd: resolution.api_equiv_usd.filter(|_| reported),
            wall_ms: resolution.wall_ms,
        }
    }
}

/// What the bank saw on one resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectorReport {
    /// EVs whose detector alarmed in the breach direction, confirmed or not.
    pub alarms: Vec<Ev>,
    /// EVs whose breach this resolution confirmed: each may open an episode.
    pub confirmed: Vec<Ev>,
    /// EVs whose EWMA chart is past its warning limit in the breach
    /// direction: the demo's warn level, which confirms nothing.
    pub warnings: Vec<Ev>,
}

impl DetectorReport {
    fn record(&mut self, ev: Ev, outcome: WatchOutcome) {
        if outcome.alarm {
            self.alarms.push(ev);
        }
        if outcome.confirmed {
            self.confirmed.push(ev);
        }
        if outcome.warning {
            self.warnings.push(ev);
        }
    }
}

/// The detectors of E1 to E4 (S06 §4.4).
#[derive(Debug, Clone)]
pub struct DetectorBank {
    baseline: Baseline,
    pass_rate: CusumWatch,
    usd_per_verified_success: CusumWatch,
    latency_p90_s: CusumWatch,
    false_green: Confirmation,
}

impl DetectorBank {
    /// Detectors for shifts from `baseline`, with E1's drift set by its
    /// bound in `bounds`.
    #[must_use]
    pub fn new(baseline: Baseline, bounds: &EvBounds, tuning: &DetectorTuning) -> Self {
        let pass_k = ((baseline.pass_rate - bounds.pass_rate.lo) / 2.0).max(MIN_PASS_RATE_DRIFT);
        let bernoulli_sd = (baseline.pass_rate * (1.0 - baseline.pass_rate)).sqrt();
        let confirmation = Confirmation::new(tuning.confirm_k, tuning.confirm_gap);
        let log_ratio_watch = || CusumWatch {
            watch: CusumShift::Upward,
            target: 0.0,
            cusum: CusumDetector::new(0.0, tuning.log_ratio_h, tuning.log_ratio_k),
            warn: EwmaControlChart::new(
                0.0,
                tuning.log_ratio_sd,
                tuning.ewma_lambda,
                tuning.ewma_limit,
            ),
            confirmation,
        };
        Self {
            baseline,
            pass_rate: CusumWatch {
                watch: CusumShift::Downward,
                target: baseline.pass_rate,
                cusum: CusumDetector::new(baseline.pass_rate, tuning.pass_rate_h, pass_k),
                warn: EwmaControlChart::new(
                    baseline.pass_rate,
                    bernoulli_sd,
                    tuning.ewma_lambda,
                    tuning.ewma_limit,
                ),
                confirmation,
            },
            usd_per_verified_success: log_ratio_watch(),
            latency_p90_s: log_ratio_watch(),
            false_green: Confirmation::new(tuning.confirm_k, 1),
        }
    }

    /// Feed one adaptive-arm resolution and, when audits exist, the
    /// window's E3 estimate. Holdout rows never reach the detectors.
    pub fn observe(
        &mut self,
        observation: &Observation,
        false_green: Option<&Estimate>,
    ) -> DetectorReport {
        let mut report = DetectorReport::default();
        let passed = if observation.passed { 1.0 } else { 0.0 };
        report.record(Ev::PassRate, self.pass_rate.observe(passed));
        if let Some(x) = log_ratio(observation.usd, self.baseline.usd_per_resolution) {
            let outcome = self.usd_per_verified_success.observe(x);
            report.record(Ev::UsdPerVerifiedSuccess, outcome);
        }
        let wall = observation.wall_ms.map(|ms| ms as f64);
        if let Some(x) = log_ratio(wall, self.baseline.wall_ms) {
            report.record(Ev::LatencyP90S, self.latency_p90_s.observe(x));
        }
        if let Some(estimate) = false_green {
            let alarm = estimate
                .breach_probability
                .is_some_and(|probability| probability > FALSE_GREEN_BREACH_PROBABILITY);
            let outcome = WatchOutcome {
                alarm,
                confirmed: self.false_green.observe(alarm),
                warning: false,
            };
            report.record(Ev::FalseGreen, outcome);
        }
        report
    }

    /// Restart every detector, as after every applied change (S06 §4.4).
    pub fn reset_all(&mut self) {
        self.pass_rate.reset();
        self.usd_per_verified_success.reset();
        self.latency_p90_s.reset();
        self.false_green.reset();
    }
}

/// `ln(value / star)`, when both are positive.
fn log_ratio(value: Option<f64>, star: f64) -> Option<f64> {
    let value = value?;
    if value > 0.0 && star > 0.0 {
        Some((value / star).ln())
    } else {
        None
    }
}

/// One observation's result at one detector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct WatchOutcome {
    alarm: bool,
    confirmed: bool,
    warning: bool,
}

/// One EV's CUSUM in its breach direction, with its confirmation and its
/// EWMA warn chart.
#[derive(Debug, Clone)]
struct CusumWatch {
    watch: CusumShift,
    target: f64,
    cusum: CusumDetector,
    warn: EwmaControlChart,
    confirmation: Confirmation,
}

impl CusumWatch {
    fn observe(&mut self, x: f64) -> WatchOutcome {
        let alarm = self.cusum.update(x) == Some(self.watch);
        let drifted = match self.watch {
            CusumShift::Downward => self.warn.update(x) != ControlStatus::InControl
                && self.warn.current() < self.target,
            CusumShift::Upward => self.warn.update(x) != ControlStatus::InControl
                && self.warn.current() > self.target,
        };
        WatchOutcome {
            alarm,
            confirmed: self.confirmation.observe(alarm),
            warning: drifted,
        }
    }

    fn reset(&mut self) {
        self.cusum.reset();
        self.warn.reset();
        self.confirmation.reset();
    }
}

/// `needed` consecutive alarms, each within `gap` observations of the one
/// before, confirm a breach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Confirmation {
    needed: u32,
    gap: u32,
    tick: u64,
    streak: u32,
    last_alarm: Option<u64>,
}

impl Confirmation {
    const fn new(needed: u32, gap: u32) -> Self {
        Self {
            needed,
            gap,
            tick: 0,
            streak: 0,
            last_alarm: None,
        }
    }

    /// Count one observation, which alarmed or not. Returns whether it
    /// confirms a breach; a confirmation starts a fresh streak.
    fn observe(&mut self, alarm: bool) -> bool {
        self.tick += 1;
        if !alarm {
            return false;
        }
        let consecutive = self
            .last_alarm
            .is_some_and(|last| self.tick - last <= u64::from(self.gap));
        self.streak = if consecutive { self.streak + 1 } else { 1 };
        self.last_alarm = Some(self.tick);
        if self.streak < self.needed.max(1) {
            return false;
        }
        self.streak = 0;
        self.last_alarm = None;
        true
    }

    fn reset(&mut self) {
        *self = Self::new(self.needed, self.gap);
    }
}

#[cfg(test)]
mod tests {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;
    use crate::homeostasis::ev::estimate_false_green;

    /// In control: 80% pass, $0.05 and five minutes per resolution.
    const BASELINE: Baseline = Baseline {
        pass_rate: 0.80,
        usd_per_resolution: 0.05,
        wall_ms: 300_000.0,
    };
    /// The sd of the log-ratio noise on costs and wall times.
    const LOG_SD: f64 = 0.5;

    /// The S06 example bounds with E1's default L₁ = max(0.5, p* − 0.15).
    fn bounds() -> EvBounds {
        let mut bounds = EvBounds::s06_example();
        bounds.pass_rate.lo = 0.65;
        bounds.pass_rate.inner = Some(0.70);
        bounds
    }

    fn gaussian(rng: &mut ChaCha8Rng) -> f64 {
        let u1: f64 = rng.gen_range(f64::MIN_POSITIVE..1.0);
        let u2: f64 = rng.gen_range(0.0..1.0);
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    /// One resolution passing with probability `pass_rate`, its cost and
    /// wall time lognormal around the baseline times `cost` and `wall`.
    fn draw(rng: &mut ChaCha8Rng, pass_rate: f64, cost: f64, wall: f64) -> Observation {
        let usd = BASELINE.usd_per_resolution * cost * (LOG_SD * gaussian(rng)).exp();
        let wall_ms = BASELINE.wall_ms * wall * (LOG_SD * gaussian(rng)).exp();
        Observation {
            passed: rng.gen_bool(pass_rate),
            usd: Some(usd),
            wall_ms: Some(wall_ms as u64),
        }
    }

    #[test]
    fn cusum_arl0_at_least_100_in_bounds() {
        // 400 in-bounds streams, each run until its first confirmed breach
        // of any EV: the mean run length is the episode ARL₀.
        let tuning = DetectorTuning::default();
        let mut rng = ChaCha8Rng::seed_from_u64(2026);
        let streams = 400_u32;
        let cap = 2_000_u32;
        let mut total = 0_u64;
        let mut alarms = 0_u64;
        for _ in 0..streams {
            let mut bank = DetectorBank::new(BASELINE, &bounds(), &tuning);
            let mut run_length = cap;
            for t in 1..=cap {
                let report = bank.observe(&draw(&mut rng, 0.80, 1.0, 1.0), None);
                alarms += report.alarms.len() as u64;
                if !report.confirmed.is_empty() {
                    run_length = t;
                    break;
                }
            }
            total += u64::from(run_length);
        }
        let arl0 = total as f64 / f64::from(streams);
        assert!(arl0 >= 100.0, "ARL0 {arl0} below 100 resolutions");
        // Confirmation is what buys the ARL₀: single alarms come far more
        // often than one per 100 resolutions.
        assert!(alarms > u64::from(streams), "{alarms} alarms");
    }

    #[test]
    fn step_detected_within_10() {
        // Twenty in-control resolutions, then E1 0.80 → 0.50, E2 ×2 or
        // E4 ×2: the median delay to the stepped EV's confirmed breach is
        // at most ten resolutions. A false episode before the step restarts
        // the detectors, as an applied change would.
        let tuning = DetectorTuning::default();
        let onset = 20_usize;
        let scenarios = [
            (Ev::PassRate, 0.50, 1.0, 1.0),
            (Ev::UsdPerVerifiedSuccess, 0.80, 2.0, 1.0),
            (Ev::LatencyP90S, 0.80, 1.0, 2.0),
        ];
        let mut rng = ChaCha8Rng::seed_from_u64(20);
        for (ev, pass_rate, cost, wall) in scenarios {
            let mut delays = Vec::with_capacity(400);
            for _ in 0..400 {
                let mut bank = DetectorBank::new(BASELINE, &bounds(), &tuning);
                let mut delay = 300;
                for t in 0..onset + 300 {
                    let observation = if t < onset {
                        draw(&mut rng, 0.80, 1.0, 1.0)
                    } else {
                        draw(&mut rng, pass_rate, cost, wall)
                    };
                    let report = bank.observe(&observation, None);
                    if t < onset {
                        if !report.confirmed.is_empty() {
                            bank.reset_all();
                        }
                    } else if report.confirmed.contains(&ev) {
                        delay = t - onset + 1;
                        break;
                    }
                }
                delays.push(delay);
            }
            delays.sort_unstable();
            let median = delays[delays.len() / 2];
            assert!(median <= 10, "{ev:?}: median confirmed delay {median}");
        }
    }

    #[test]
    fn confirmation_needs_consecutive_alarms_and_skips_unreported_costs() {
        let mut confirmation = Confirmation::new(2, 4);
        assert!(!confirmation.observe(true));
        assert!(!confirmation.observe(false));
        assert!(!confirmation.observe(false));
        assert!(confirmation.observe(true), "second alarm three ticks later");
        assert!(!confirmation.observe(true), "a confirmation starts afresh");
        for _ in 0..5 {
            assert!(!confirmation.observe(false));
        }
        assert!(!confirmation.observe(true), "six ticks apart: not consecutive");
        assert!(confirmation.observe(true));
        assert!(!confirmation.observe(true));
        confirmation.reset();
        assert!(!confirmation.observe(true), "reset clears the streak");

        // A run of failures confirms an E1 breach; a reset restarts it.
        let mut bank = DetectorBank::new(BASELINE, &bounds(), &DetectorTuning::default());
        let failed = Observation {
            passed: false,
            usd: None,
            wall_ms: None,
        };
        let confirmed_at = (1..=10)
            .find(|_| !bank.observe(&failed, None).confirmed.is_empty())
            .expect("failures confirm a breach");
        assert!(confirmed_at <= 4, "confirmed at {confirmed_at}");
        bank.reset_all();
        assert!(bank.observe(&failed, None).confirmed.is_empty());

        // Costs that are not reported usage never reach E2's detector.
        let mut resolution_bank =
            DetectorBank::new(BASELINE, &bounds(), &DetectorTuning::default());
        let mut expensive = crate::homeostasis::resolution::TaskResolution {
            chain_key: "run:plan:task".to_string(),
            final_verdict: crate::telemetry::AttemptOutcome::Passed,
            attempts: 1,
            api_equiv_usd: Some(50.0),
            cost_source_mix: crate::homeostasis::resolution::CostSourceMix::default(),
            wall_ms: None,
            provider_errors: 0,
            conductor_restarts: 0,
            params_digest: None,
            arm: None,
            audited: false,
            audit_false_green: None,
            pre_instrumentation: false,
            resolved_at: None,
        };
        expensive
            .cost_source_mix
            .add(crate::telemetry::CostSource::Estimated);
        assert_eq!(Observation::of(&expensive).usd, None);
        for _ in 0..20 {
            let report = resolution_bank.observe(&Observation::of(&expensive), None);
            assert!(!report.alarms.contains(&Ev::UsdPerVerifiedSuccess));
        }

        // E3 confirms on two consecutive evaluations above 0.9.
        let audits: Vec<bool> = (0..30).map(|index| index < 7).collect();
        let breach = estimate_false_green(&audits, 0.10);
        let passed = Observation {
            passed: true,
            usd: None,
            wall_ms: None,
        };
        let mut bank = DetectorBank::new(BASELINE, &bounds(), &DetectorTuning::default());
        assert!(bank.observe(&passed, Some(&breach)).confirmed.is_empty());
        assert_eq!(
            bank.observe(&passed, Some(&breach)).confirmed,
            [Ev::FalseGreen]
        );
    }
}
