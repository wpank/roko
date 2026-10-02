//! Statistical Process Control (SPC) extensions for adaptive gate thresholds.
//!
//! Three detectors beyond the base EMA:
//!
//! - **CUSUM (Cumulative Sum)** and the **EWMA Control Chart** live in
//!   roko-learn (`roko_learn::homeostasis::spc`), where M1's change detectors
//!   use them, and are re-exported here.
//!
//! - **BOCPD (Bayesian Online Change Point Detection)**: Detects abrupt regime
//!   changes (e.g., a model update causes sudden behavior shift). Maintains a
//!   run-length distribution and signals when posterior probability of a recent
//!   change point exceeds a threshold.
//!
//! Reference: docs/04-verification/06-adaptive-thresholds.md sections 11-15.

pub use roko_learn::homeostasis::spc::{
    ControlStatus, CusumDetector, CusumShift, EwmaControlChart,
};
use serde::{Deserialize, Serialize};

// ─── BOCPD (Bayesian Online Change Point Detection) ─────────────────────────

/// A detected change point from the BOCPD detector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangePoint {
    /// Observation index where the change was detected.
    pub observation_index: usize,
    /// Most probable run length at the time of detection.
    pub most_probable_run_length: usize,
    /// Posterior probability that a change occurred recently.
    pub change_probability: f64,
}

/// Bayesian Online Change Point Detection (BOCPD).
///
/// Maintains a run-length distribution updated at each observation.
/// When the posterior probability of a recent change point (run length
/// near zero) exceeds the threshold, a change is flagged.
///
/// Uses a Gaussian predictive model (conjugate normal-inverse-gamma)
/// for simplicity.
///
/// Reference: Adams & MacKay (2007), "Bayesian Online Changepoint Detection".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BocpdDetector {
    /// Hazard rate: prior probability of a change point at each step.
    /// Typically 1/expected_run_length (e.g. 1/100 = 0.01).
    hazard_rate: f64,
    /// Threshold on posterior change probability to flag a change.
    change_threshold: f64,
    /// Run-length distribution (unnormalized log-probabilities).
    run_length_probs: Vec<f64>,
    /// Sufficient statistics for each run length: (count, sum, sum_sq).
    sufficient_stats: Vec<(usize, f64, f64)>,
    /// Prior mean for the Gaussian model.
    prior_mean: f64,
    /// Prior variance for the Gaussian model.
    prior_var: f64,
    /// Total observations processed.
    observations: usize,
}

impl BocpdDetector {
    /// Create a new BOCPD detector.
    ///
    /// - `hazard_rate`: prior probability of change at each step (e.g. 0.01)
    /// - `change_threshold`: posterior probability to trigger alarm (e.g. 0.5)
    /// - `prior_mean`: expected observation mean
    /// - `prior_var`: expected observation variance
    #[must_use]
    pub fn new(hazard_rate: f64, change_threshold: f64, prior_mean: f64, prior_var: f64) -> Self {
        Self {
            hazard_rate: hazard_rate.clamp(0.001, 0.5),
            change_threshold: change_threshold.clamp(0.0, 1.0),
            // Start with run length 0 having probability 1.0.
            run_length_probs: vec![1.0],
            sufficient_stats: vec![(0, 0.0, 0.0)],
            prior_mean,
            prior_var: prior_var.max(0.001),
            observations: 0,
        }
    }

    /// Update with a new observation and return a change point if detected.
    pub fn update(&mut self, observation: f64) -> Option<ChangePoint> {
        self.observations += 1;

        let n = self.run_length_probs.len();

        // Step 1: Compute predictive probabilities for each run length.
        let mut predictive_probs = Vec::with_capacity(n);
        for (count, sum, sum_sq) in &self.sufficient_stats {
            let pred_prob = self.gaussian_predictive(*count, *sum, *sum_sq, observation);
            predictive_probs.push(pred_prob);
        }

        // Step 2: Compute growth probabilities (extend existing runs).
        let mut growth_probs = Vec::with_capacity(n);
        for (i, &prob) in self.run_length_probs.iter().enumerate() {
            growth_probs.push(prob * predictive_probs[i] * (1.0 - self.hazard_rate));
        }

        // Step 3: Compute change point probability (new run starts).
        let change_prob: f64 = self
            .run_length_probs
            .iter()
            .enumerate()
            .map(|(i, &prob)| prob * predictive_probs[i] * self.hazard_rate)
            .sum();

        // Step 4: Assemble new run-length distribution.
        let mut new_probs = Vec::with_capacity(n + 1);
        new_probs.push(change_prob);
        new_probs.extend_from_slice(&growth_probs);

        // Normalize.
        let total: f64 = new_probs.iter().sum();
        if total > f64::EPSILON {
            for p in &mut new_probs {
                *p /= total;
            }
        }

        // Step 5: Update sufficient statistics.
        let mut new_stats = Vec::with_capacity(n + 1);
        // New run (length 0) uses prior.
        new_stats.push((0, 0.0, 0.0));
        // Extend existing runs.
        for (count, sum, sum_sq) in &self.sufficient_stats {
            new_stats.push((
                count + 1,
                sum + observation,
                sum_sq + observation * observation,
            ));
        }

        self.run_length_probs = new_probs;
        self.sufficient_stats = new_stats;

        // Trim very small probabilities to bound memory.
        self.trim_low_probability(1e-8);

        // Step 6: Check if change probability exceeds threshold.
        let p_change = self.run_length_probs.first().copied().unwrap_or(0.0);
        if p_change > self.change_threshold {
            let most_probable = self.most_probable_run_length();
            return Some(ChangePoint {
                observation_index: self.observations,
                most_probable_run_length: most_probable,
                change_probability: p_change,
            });
        }

        None
    }

    /// Most probable run length from the current distribution.
    #[must_use]
    pub fn most_probable_run_length(&self) -> usize {
        self.run_length_probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map_or(0, |(i, _)| i)
    }

    /// Probability of a recent change point (run length = 0).
    #[must_use]
    pub fn change_probability(&self) -> f64 {
        self.run_length_probs.first().copied().unwrap_or(0.0)
    }

    /// Number of observations processed.
    #[must_use]
    pub fn observations(&self) -> usize {
        self.observations
    }

    /// Reset the detector.
    pub fn reset(&mut self) {
        self.run_length_probs = vec![1.0];
        self.sufficient_stats = vec![(0, 0.0, 0.0)];
        self.observations = 0;
    }

    /// Gaussian predictive probability using conjugate prior.
    fn gaussian_predictive(&self, count: usize, sum: f64, sum_sq: f64, observation: f64) -> f64 {
        let n = count as f64;
        let mean = if n > 0.0 {
            (self.prior_var * sum + self.prior_mean) / (n * self.prior_var + 1.0)
        } else {
            self.prior_mean
        };
        let var = if n > 0.0 {
            let sample_var = if n > 1.0 {
                (sum_sq - sum * sum / n) / (n - 1.0)
            } else {
                self.prior_var
            };
            sample_var / n + self.prior_var
        } else {
            self.prior_var
        };
        let var = var.max(0.001);

        // Gaussian PDF.
        let diff = observation - mean;
        (-0.5 * diff * diff / var).exp() / (2.0 * std::f64::consts::PI * var).sqrt()
    }

    /// Remove run lengths with negligible probability to bound memory.
    fn trim_low_probability(&mut self, min_prob: f64) {
        // Only trim from the tail (longest run lengths).
        while self.run_length_probs.len() > 2 {
            if let Some(&last) = self.run_length_probs.last() {
                if last < min_prob {
                    self.run_length_probs.pop();
                    self.sufficient_stats.pop();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
}

// ─── Composite SPC detector ────────────────────────────────────────────────

/// Aggregate SPC alert from any of the three detectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpcAlert {
    /// CUSUM detected a sustained shift.
    CusumShift(CusumShift),
    /// EWMA chart is out of control.
    EwmaOutOfControl {
        /// Current EWMA value.
        ewma_value: f64,
    },
    /// EWMA chart is in warning zone.
    EwmaWarning {
        /// Current EWMA value.
        ewma_value: f64,
    },
    /// BOCPD detected a change point.
    ChangePoint(ChangePoint),
}

/// Composite SPC detector that runs all three methods in parallel.
///
/// Any detector that fires produces an alert. Wire this into
/// `AdaptiveThresholds` to get richer anomaly detection beyond the
/// base EMA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpcDetector {
    /// CUSUM detector.
    pub cusum: CusumDetector,
    /// EWMA control chart.
    pub ewma_chart: EwmaControlChart,
    /// BOCPD detector.
    pub bocpd: BocpdDetector,
}

impl SpcDetector {
    /// Create a composite SPC detector with default parameters.
    ///
    /// - `target`: expected in-control value (e.g. 0.85 pass rate)
    /// - `sigma`: estimated standard deviation (e.g. 0.1)
    #[must_use]
    pub fn new(target: f64, sigma: f64) -> Self {
        Self {
            cusum: CusumDetector::new(target, 5.0, sigma / 2.0),
            ewma_chart: EwmaControlChart::new(target, sigma, 0.2, 3.0),
            bocpd: BocpdDetector::new(0.01, 0.5, target, sigma * sigma),
        }
    }

    /// Update all detectors with a new observation.
    ///
    /// Returns all alerts fired by any detector.
    pub fn update(&mut self, observation: f64) -> Vec<SpcAlert> {
        let mut alerts = Vec::new();

        if let Some(shift) = self.cusum.update(observation) {
            alerts.push(SpcAlert::CusumShift(shift));
        }

        match self.ewma_chart.update(observation) {
            ControlStatus::OutOfControl => {
                alerts.push(SpcAlert::EwmaOutOfControl {
                    ewma_value: self.ewma_chart.current(),
                });
            }
            ControlStatus::Warning => {
                alerts.push(SpcAlert::EwmaWarning {
                    ewma_value: self.ewma_chart.current(),
                });
            }
            ControlStatus::InControl => {}
        }

        if let Some(cp) = self.bocpd.update(observation) {
            alerts.push(SpcAlert::ChangePoint(cp));
        }

        alerts
    }

    /// Reset all detectors.
    pub fn reset(&mut self) {
        self.cusum.reset();
        self.ewma_chart.reset();
        self.bocpd.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── BOCPD tests ────────────────────────────────────────────────────

    #[test]
    fn bocpd_no_change_under_stable_process() {
        let mut bocpd = BocpdDetector::new(0.01, 0.5, 0.85, 0.01);
        let mut change_count = 0;
        for _ in 0..50 {
            if bocpd.update(0.85).is_some() {
                change_count += 1;
            }
        }
        // Under a perfectly stable process, ideally no changes.
        // (Allow at most 1 spurious detection from initial transient.)
        assert!(
            change_count <= 1,
            "got {change_count} changes under stable process"
        );
    }

    #[test]
    fn bocpd_detects_abrupt_change() {
        // Use a high hazard rate and low threshold for reliable detection.
        let mut bocpd = BocpdDetector::new(0.2, 0.15, 0.85, 0.01);
        // Stable regime.
        for _ in 0..30 {
            bocpd.update(0.85);
        }
        // Abrupt shift: completely different value.
        let mut max_change_prob = 0.0_f64;
        let mut detected = false;
        for _ in 0..50 {
            if let Some(_cp) = bocpd.update(0.1) {
                detected = true;
                break;
            }
            max_change_prob = max_change_prob.max(bocpd.change_probability());
        }
        assert!(
            detected,
            "BOCPD should detect abrupt change (max_change_prob={max_change_prob:.4})"
        );
    }

    #[test]
    fn bocpd_reset_clears_state() {
        let mut bocpd = BocpdDetector::new(0.01, 0.5, 0.85, 0.01);
        for _ in 0..10 {
            bocpd.update(0.5);
        }
        bocpd.reset();
        assert_eq!(bocpd.observations(), 0);
        assert_eq!(bocpd.run_length_probs.len(), 1);
    }

    // ─── Composite SPC tests ────────────────────────────────────────────

    #[test]
    fn spc_detector_no_alerts_under_normal() {
        let mut spc = SpcDetector::new(0.85, 0.1);
        for _ in 0..20 {
            let alerts = spc.update(0.85);
            assert!(alerts.is_empty(), "no alerts expected, got {alerts:?}");
        }
    }

    #[test]
    fn spc_detector_alerts_on_major_shift() {
        let mut spc = SpcDetector::new(0.85, 0.05);
        // Establish baseline.
        for _ in 0..20 {
            spc.update(0.85);
        }
        // Major shift.
        let mut any_alert = false;
        for _ in 0..50 {
            let alerts = spc.update(0.2);
            if !alerts.is_empty() {
                any_alert = true;
                break;
            }
        }
        assert!(any_alert, "SPC should detect major shift");
    }

    #[test]
    fn spc_detector_reset() {
        let mut spc = SpcDetector::new(0.85, 0.1);
        spc.update(0.1);
        spc.reset();
        assert_eq!(spc.cusum.observations(), 0);
        assert_eq!(spc.ewma_chart.observations(), 0);
        assert_eq!(spc.bocpd.observations(), 0);
    }
}
