//! M1's change-detection charts: a two-sided CUSUM and an EWMA control chart.
//!
//! - **CUSUM (Cumulative Sum)**: detects sustained shifts in a process.
//!   Accumulates deviations from a target; when the cumulative sum exceeds
//!   threshold `h`, signals a shift. Good for catching gradual degradation.
//!
//! - **EWMA Control Chart**: exponentially weighted moving average with formal
//!   UCL/LCL (Upper/Lower Control Limits). More sensitive to small shifts than
//!   standard Shewhart charts.
//!
//! They moved here from roko-gate's `spc` module (Will's decision, 2026-10-02):
//! M1's detectors ([`super::detect`]) need them, and roko-learn (layer 2) may
//! not depend on roko-gate (layer 3). roko-gate re-exports both, so its
//! adaptive thresholds keep using the same code.
//!
//! Reference: docs/04-verification/06-adaptive-thresholds.md sections 11-15.

use serde::{Deserialize, Serialize};

// ─── CUSUM Detector ─────────────────────────────────────────────────────────

/// Cumulative Sum (CUSUM) detector for sustained shifts in a process.
///
/// Maintains two one-sided statistics (upper and lower) that accumulate
/// deviations from the target. When either exceeds `threshold_h`, a shift
/// is detected in the corresponding direction.
///
/// Parameters:
/// - `target`: the expected (in-control) value of the observation
/// - `threshold_h`: decision interval — larger values reduce false alarms
/// - `drift_k`: allowance (slack) parameter — typically half the shift to detect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CusumDetector {
    /// Target (in-control) value.
    pub target: f64,
    /// Decision threshold — alarm when cumsum exceeds this.
    pub threshold_h: f64,
    /// Drift allowance — half the smallest shift worth detecting.
    pub drift_k: f64,
    /// Upper one-sided cumulative sum (detects upward shifts).
    cumsum_upper: f64,
    /// Lower one-sided cumulative sum (detects downward shifts).
    cumsum_lower: f64,
    /// Number of observations processed.
    observations: usize,
}

/// Direction of a detected CUSUM shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CusumShift {
    /// The process shifted upward from the target.
    Upward,
    /// The process shifted downward from the target.
    Downward,
}

impl CusumDetector {
    /// Create a new CUSUM detector.
    ///
    /// - `target`: in-control mean (e.g. 0.85 pass rate)
    /// - `threshold_h`: alarm threshold (e.g. 5.0)
    /// - `drift_k`: slack parameter (e.g. 0.5 * minimum shift to detect)
    #[must_use]
    pub fn new(target: f64, threshold_h: f64, drift_k: f64) -> Self {
        Self {
            target,
            threshold_h: threshold_h.max(0.0),
            drift_k: drift_k.max(0.0),
            cumsum_upper: 0.0,
            cumsum_lower: 0.0,
            observations: 0,
        }
    }

    /// Update with a new observation and return whether a shift was detected.
    pub fn update(&mut self, observation: f64) -> Option<CusumShift> {
        self.observations += 1;

        // Upper CUSUM: detects upward shift.
        self.cumsum_upper = (self.cumsum_upper + observation - self.target - self.drift_k).max(0.0);
        // Lower CUSUM: detects downward shift.
        self.cumsum_lower = (self.cumsum_lower + self.target - observation - self.drift_k).max(0.0);

        if self.cumsum_upper > self.threshold_h {
            self.cumsum_upper = 0.0; // Reset after alarm.
            return Some(CusumShift::Upward);
        }
        if self.cumsum_lower > self.threshold_h {
            self.cumsum_lower = 0.0; // Reset after alarm.
            return Some(CusumShift::Downward);
        }

        None
    }

    /// Reset the detector state.
    pub fn reset(&mut self) {
        self.cumsum_upper = 0.0;
        self.cumsum_lower = 0.0;
        self.observations = 0;
    }

    /// Number of observations processed.
    #[must_use]
    pub fn observations(&self) -> usize {
        self.observations
    }

    /// Current upper cumulative sum.
    #[must_use]
    pub fn upper_sum(&self) -> f64 {
        self.cumsum_upper
    }

    /// Current lower cumulative sum.
    #[must_use]
    pub fn lower_sum(&self) -> f64 {
        self.cumsum_lower
    }
}

// ─── EWMA Control Chart ────────────────────────────────────────────────────

/// Control status of an EWMA chart observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlStatus {
    /// Within control limits — process is in control.
    InControl,
    /// Between 2-sigma and 3-sigma — potential issue developing.
    Warning,
    /// Beyond 3-sigma control limits — process is out of control.
    OutOfControl,
}

/// EWMA (Exponentially Weighted Moving Average) Control Chart.
///
/// Tracks a smoothed process mean with formal UCL/LCL at
/// `mean +/- L * sigma * sqrt(lambda / (2 - lambda))`.
///
/// More sensitive to small sustained shifts than Shewhart charts because
/// the exponential weighting carries memory of recent observations.
///
/// Parameters:
/// - `lambda`: smoothing factor in (0, 1]. Smaller = more smoothing.
/// - `sigma`: process standard deviation estimate.
/// - `control_limit_l`: multiplier for control limits (typically 3.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EwmaControlChart {
    /// Smoothing factor (0, 1].
    lambda: f64,
    /// Estimated process standard deviation.
    sigma: f64,
    /// Control limit multiplier (number of sigma).
    control_limit_l: f64,
    /// Current EWMA value.
    ewma: f64,
    /// Initial (target) mean.
    target: f64,
    /// Number of observations.
    observations: usize,
}

impl EwmaControlChart {
    /// Create a new EWMA control chart.
    ///
    /// - `target`: in-control mean
    /// - `sigma`: estimated process standard deviation
    /// - `lambda`: smoothing factor (0.0, 1.0], typical = 0.2
    /// - `control_limit_l`: sigma multiplier for limits, typical = 3.0
    #[must_use]
    pub fn new(target: f64, sigma: f64, lambda: f64, control_limit_l: f64) -> Self {
        let lambda = lambda.clamp(0.01, 1.0);
        Self {
            lambda,
            sigma: sigma.max(0.001),
            control_limit_l: control_limit_l.max(0.0),
            ewma: target,
            target,
            observations: 0,
        }
    }

    /// Update the chart with a new observation and return the control status.
    pub fn update(&mut self, observation: f64) -> ControlStatus {
        self.observations += 1;
        self.ewma = self.lambda * observation + (1.0 - self.lambda) * self.ewma;

        let limit_factor = self.sigma * (self.lambda / (2.0 - self.lambda)).sqrt();
        let ucl = self.target + self.control_limit_l * limit_factor;
        let lcl = self.target - self.control_limit_l * limit_factor;
        let warning_ucl = self.target + (self.control_limit_l * 2.0 / 3.0) * limit_factor;
        let warning_lcl = self.target - (self.control_limit_l * 2.0 / 3.0) * limit_factor;

        if self.ewma > ucl || self.ewma < lcl {
            ControlStatus::OutOfControl
        } else if self.ewma > warning_ucl || self.ewma < warning_lcl {
            ControlStatus::Warning
        } else {
            ControlStatus::InControl
        }
    }

    /// Current EWMA value.
    #[must_use]
    pub fn current(&self) -> f64 {
        self.ewma
    }

    /// Upper control limit.
    #[must_use]
    pub fn ucl(&self) -> f64 {
        let limit_factor = self.sigma * (self.lambda / (2.0 - self.lambda)).sqrt();
        self.target + self.control_limit_l * limit_factor
    }

    /// Lower control limit.
    #[must_use]
    pub fn lcl(&self) -> f64 {
        let limit_factor = self.sigma * (self.lambda / (2.0 - self.lambda)).sqrt();
        self.target - self.control_limit_l * limit_factor
    }

    /// Number of observations processed.
    #[must_use]
    pub fn observations(&self) -> usize {
        self.observations
    }

    /// Reset the chart to its initial state.
    pub fn reset(&mut self) {
        self.ewma = self.target;
        self.observations = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── CUSUM tests ────────────────────────────────────────────────────

    #[test]
    fn cusum_no_shift_under_normal_conditions() {
        let mut cusum = CusumDetector::new(0.85, 5.0, 0.05);
        // Feed in-control observations.
        for _ in 0..50 {
            assert!(cusum.update(0.85).is_none());
        }
    }

    #[test]
    fn cusum_detects_upward_shift() {
        let mut cusum = CusumDetector::new(0.5, 3.0, 0.1);
        let mut detected = false;
        // Sustained upward deviation.
        for _ in 0..100 {
            if cusum.update(0.9).is_some() {
                detected = true;
                break;
            }
        }
        assert!(detected, "CUSUM should detect upward shift");
    }

    #[test]
    fn cusum_detects_downward_shift() {
        let mut cusum = CusumDetector::new(0.85, 3.0, 0.05);
        let mut detected = false;
        // Sustained downward deviation.
        for _ in 0..100 {
            if let Some(CusumShift::Downward) = cusum.update(0.4) {
                detected = true;
                break;
            }
        }
        assert!(detected, "CUSUM should detect downward shift");
    }

    #[test]
    fn cusum_reset_clears_state() {
        let mut cusum = CusumDetector::new(0.85, 5.0, 0.05);
        cusum.update(0.1);
        cusum.update(0.1);
        assert!(cusum.lower_sum() > 0.0);
        cusum.reset();
        assert_eq!(cusum.upper_sum(), 0.0);
        assert_eq!(cusum.lower_sum(), 0.0);
        assert_eq!(cusum.observations(), 0);
    }

    // ─── EWMA Control Chart tests ───────────────────────────────────────

    #[test]
    fn ewma_in_control_under_normal_conditions() {
        let mut chart = EwmaControlChart::new(0.85, 0.1, 0.2, 3.0);
        for _ in 0..20 {
            let status = chart.update(0.85);
            assert_eq!(status, ControlStatus::InControl);
        }
    }

    #[test]
    fn ewma_out_of_control_on_large_deviation() {
        let mut chart = EwmaControlChart::new(0.85, 0.05, 0.2, 3.0);
        let mut out = false;
        for _ in 0..50 {
            if chart.update(0.2) == ControlStatus::OutOfControl {
                out = true;
                break;
            }
        }
        assert!(out, "EWMA should detect out-of-control");
    }

    #[test]
    fn ewma_control_limits_are_symmetric() {
        let chart = EwmaControlChart::new(0.5, 0.1, 0.2, 3.0);
        let ucl = chart.ucl();
        let lcl = chart.lcl();
        assert!((ucl - 0.5 - (0.5 - lcl)).abs() < 1e-10);
    }

    #[test]
    fn ewma_reset_restores_target() {
        let mut chart = EwmaControlChart::new(0.85, 0.1, 0.2, 3.0);
        chart.update(0.2);
        chart.update(0.2);
        chart.reset();
        assert!((chart.current() - 0.85).abs() < 1e-10);
        assert_eq!(chart.observations(), 0);
    }
}
